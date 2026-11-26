//! Generic block/item decoder (spec section 4.2).
//!
//! A block is `u32 length` (including itself) followed by items until the length is
//! used up. An item starts with a little-endian 16-bit header word: bits 0-10 are the item id
//! (0..=0x7FE) and bits 11-15 the value class; the payload depends only on the class. Items are
//! stored in ascending id order. See [`split_header`] and [`header`].

use crate::error::PubError;

/// Maximum nesting of blocks inside blocks; guards against stack exhaustion.
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// No payload (classes 0, 1, 15, 21, 22 and 28-31).
    Flag,
    /// Integer of 2, 4 or 8 bytes, little-endian.
    Int(u64),
    /// Fixed-size payload longer than 8 bytes (classes 6-10: 12, 16, 20, 24 or 32 bytes).
    Bytes(Vec<u8>),
    /// Nested block of items (classes 17, 19).
    Record(Vec<Item>),
    /// Nested block used as a list (classes 18, 20).
    List(Vec<Item>),
    /// Length-prefixed raw bytes (classes 16, 24-27); length prefix removed.
    Raw(Vec<u8>),
}

/// Split the two header bytes of an item into `(id, class)`:
/// `id = b0 | (b1 & 7) << 8`, `class = b1 >> 3`.
pub fn split_header(b0: u8, b1: u8) -> (u16, u8) {
    (u16::from(b0) | (u16::from(b1 & 7) << 8), b1 >> 3)
}

/// The two header bytes for an item with the given 11-bit `id` and value `class` (0..=31).
pub fn header(id: u16, class: u8) -> [u8; 2] {
    debug_assert!(id <= 0x7FF && class < 32);
    [id as u8, ((id >> 8) as u8 & 7) | (class << 3)]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// 11-bit item id.
    pub id: u16,
    /// Value class (header bits 11-15).
    pub class: u8,
    pub value: Value,
}

impl Item {
    pub fn as_int(&self) -> Option<u64> {
        match self.value {
            Value::Int(v) => Some(v),
            _ => None,
        }
    }
}

/// Find the first item with the given id.
pub fn find(items: &[Item], id: u16) -> Option<&Item> {
    items.iter().find(|i| i.id == id)
}

/// Find the first item with the given id and return its integer value.
pub fn find_int(items: &[Item], id: u16) -> Option<u64> {
    find(items, id).and_then(Item::as_int)
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, PubError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| PubError::format(offset, "offset overflow"))?;
    let b = data
        .get(offset..end)
        .ok_or_else(|| PubError::format(offset, "truncated: need 4 bytes"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decode the block that starts at `offset` in `data`.
pub fn decode_block(data: &[u8], offset: usize) -> Result<Vec<Item>, PubError> {
    decode_nested(data, offset, data.len(), 0).map(|(items, _)| items)
}

/// Decode a block at `offset` that must end at or before `limit`. Returns the items
/// and the block's total length.
fn decode_nested(
    data: &[u8],
    offset: usize,
    limit: usize,
    depth: usize,
) -> Result<(Vec<Item>, usize), PubError> {
    if depth > MAX_DEPTH {
        return Err(PubError::format(offset, "blocks nested too deeply"));
    }
    let total = read_u32(data, offset)? as usize;
    if total < 4 {
        return Err(PubError::format(offset, "block length smaller than 4"));
    }
    let end = offset
        .checked_add(total)
        .filter(|&e| e <= limit)
        .ok_or_else(|| PubError::format(offset, "block extends past its container"))?;
    let mut pos = offset + 4;
    let mut items = Vec::new();
    while pos < end {
        let hdr = data
            .get(pos..pos + 2)
            .filter(|_| pos + 2 <= end)
            .ok_or_else(|| PubError::format(pos, "misaligned block: item header past block end"))?;
        let (id, class) = split_header(hdr[0], hdr[1]);
        pos += 2;
        let value = match class {
            // Spec 4.2 size table: no payload.
            0 | 1 | 15 | 21 | 22 | 28..=31 => Value::Flag,
            2 | 3 => Value::Int(fixed_int(data, &mut pos, end, 2)?),
            4 | 11 | 13 | 14 | 23 => Value::Int(fixed_int(data, &mut pos, end, 4)?),
            5 | 12 => Value::Int(fixed_int(data, &mut pos, end, 8)?),
            6 => Value::Bytes(fixed_bytes(data, &mut pos, end, 12)?),
            7 => Value::Bytes(fixed_bytes(data, &mut pos, end, 16)?),
            8 => Value::Bytes(fixed_bytes(data, &mut pos, end, 20)?),
            9 => Value::Bytes(fixed_bytes(data, &mut pos, end, 24)?),
            10 => Value::Bytes(fixed_bytes(data, &mut pos, end, 32)?),
            class @ 17..=20 => {
                let (inner, len) = decode_nested(data, pos, end, depth + 1)?;
                pos += len;
                if matches!(class, 17 | 19) {
                    Value::Record(inner)
                } else {
                    Value::List(inner)
                }
            }
            // 16 and 24-27: length-prefixed raw bytes.
            _ => {
                let len = read_u32(data, pos)? as usize;
                let blob_end = pos
                    .checked_add(len)
                    .filter(|&e| len >= 4 && e <= end)
                    .ok_or_else(|| PubError::format(pos, "raw item length out of range"))?;
                let raw = data[pos + 4..blob_end].to_vec();
                pos = blob_end;
                Value::Raw(raw)
            }
        };
        items.push(Item { id, class, value });
    }
    Ok((items, total))
}

/// Where an integer item's payload sits in its stream: byte `offset` and `len` (2, 4 or 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntSlot {
    pub offset: usize,
    pub len: usize,
}

/// Find the integer item `id` among the **top-level** items of the block at `block_offset`
/// and return where its payload lives (spec 4.2 size table). Items nested in records or
/// lists are skipped by length, never entered. `Ok(None)` when absent or not an integer.
pub fn locate_int(data: &[u8], block_offset: usize, id: u16) -> Result<Option<IntSlot>, PubError> {
    let total = read_u32(data, block_offset)? as usize;
    let end = block_offset
        .checked_add(total)
        .filter(|&e| total >= 4 && e <= data.len())
        .ok_or_else(|| PubError::format(block_offset, "block extends past its container"))?;
    let mut pos = block_offset + 4;
    while pos < end {
        if pos + 2 > end {
            return Err(PubError::format(
                pos,
                "misaligned block: item header past block end",
            ));
        }
        let (item_id, class) = split_header(data[pos], data[pos + 1]);
        pos += 2;
        let (fixed, prefixed) = match class {
            0 | 1 | 15 | 21 | 22 | 28..=31 => (0, false),
            2 | 3 => (2, false),
            4 | 11 | 13 | 14 | 23 => (4, false),
            5 | 12 => (8, false),
            6 => (12, false),
            7 => (16, false),
            8 => (20, false),
            9 => (24, false),
            10 => (32, false),
            // Nested blocks (17-20) and raw bytes (16, 24-27) all start with a u32 length
            // that counts itself.
            _ => (0, true),
        };
        let skip = if prefixed {
            read_u32(data, pos)? as usize
        } else {
            fixed
        };
        let next = pos
            .checked_add(skip)
            .filter(|&n| n <= end && (!prefixed || skip >= 4))
            .ok_or_else(|| PubError::format(pos, "item payload past block end"))?;
        if item_id == id {
            return Ok(matches!(fixed, 2 | 4 | 8).then_some(IntSlot {
                offset: pos,
                len: fixed,
            }));
        }
        pos = next;
    }
    Ok(None)
}

fn fixed_bytes(data: &[u8], pos: &mut usize, end: usize, n: usize) -> Result<Vec<u8>, PubError> {
    let stop = pos
        .checked_add(n)
        .filter(|&s| s <= end)
        .ok_or_else(|| PubError::format(*pos, "misaligned block: payload past block end"))?;
    let bytes = data[*pos..stop].to_vec();
    *pos = stop;
    Ok(bytes)
}

fn fixed_int(data: &[u8], pos: &mut usize, end: usize, n: usize) -> Result<u64, PubError> {
    let bytes = fixed_bytes(data, pos, end, n)?;
    Ok(bytes
        .iter()
        .rev()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a block from already-encoded item bytes.
    fn block(body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 4) as u32).to_le_bytes().to_vec();
        v.extend_from_slice(body);
        v
    }

    fn one(body: &[u8]) -> Item {
        let items = decode_block(&block(body), 0).unwrap();
        assert_eq!(items.len(), 1);
        items.into_iter().next().unwrap()
    }

    #[test]
    fn locate_int_skips_nested_and_finds_top_level() {
        // item 1: record holding an id-5 int (must not be found), item 5: 4-byte int,
        // item 6: raw bytes (not an int).
        let inner = block(&[5, 0x20, 9, 9, 9, 9]);
        let mut body = vec![1, 0x88];
        body.extend_from_slice(&inner);
        body.extend_from_slice(&[5, 0x20, 1, 2, 3, 4]);
        body.extend_from_slice(&[6, 0xC0, 6, 0, 0, 0, 7, 7]);
        let b = block(&body);
        let slot = locate_int(&b, 0, 5).unwrap().unwrap();
        assert_eq!((slot.offset, slot.len), (4 + 2 + inner.len() + 2, 4));
        assert_eq!(&b[slot.offset..slot.offset + 4], &[1, 2, 3, 4]);
        assert_eq!(locate_int(&b, 0, 6).unwrap(), None);
        assert_eq!(locate_int(&b, 0, 99).unwrap(), None);
    }

    #[test]
    fn header_with_id_above_255() {
        // 0x2C 0x21: id = 0x2C | (1 << 8) = 0x12C, class = 0x21 >> 3 = 4 (4-byte int).
        let it = one(&[0x2C, 0x21, 1, 2, 3, 4]);
        assert_eq!((it.id, it.class), (0x12C, 4));
        assert_eq!(it.value, Value::Int(0x04030201));
        assert_eq!(split_header(0x2C, 0x21), (0x12C, 4));
        assert_eq!(header(0x12C, 4), [0x2C, 0x21]);
        assert_eq!(header(0x3C, 4), [0x3C, 0x20]);
        for id in [0u16, 1, 0xFF, 0x100, 0x7FE] {
            for class in [0u8, 1, 4, 17, 30, 31] {
                let h = header(id, class);
                assert_eq!(split_header(h[0], h[1]), (id, class));
            }
        }
    }

    #[test]
    fn lookup_does_not_match_id_plus_256() {
        // id 0x13C (bytes 3C 21) must not satisfy a lookup of 0x3C; id 0x3C follows it.
        let body = [0x3C, 0x21, 9, 9, 9, 9, 0x3C, 0x20, 1, 0, 0, 0];
        let b = block(&body);
        let slot = locate_int(&b, 0, 0x3C).unwrap().unwrap();
        assert_eq!(&b[slot.offset..slot.offset + 4], &[1, 0, 0, 0]);
        let slot = locate_int(&b, 0, 0x13C).unwrap().unwrap();
        assert_eq!(&b[slot.offset..slot.offset + 4], &[9, 9, 9, 9]);
        let only_high = block(&body[..6]);
        assert_eq!(locate_int(&only_high, 0, 0x3C).unwrap(), None);
        let items = decode_block(&only_high, 0).unwrap();
        assert_eq!(find_int(&items, 0x3C), None);
        assert_eq!(find_int(&items, 0x13C), Some(0x09090909));
    }

    #[test]
    fn flags_have_no_payload() {
        for ty in [0x00u8, 0x08, 0x0A, 0x78] {
            let it = one(&[7, ty]);
            assert_eq!((it.id, it.class, it.value),
                (7 | (u16::from(ty & 7) << 8), ty >> 3, Value::Flag)
            );
        }
    }

    #[test]
    fn small_ints() {
        for ty in [0x10u8, 0x12, 0x18] {
            assert_eq!(one(&[1, ty, 0x34, 0x12]).value, Value::Int(0x1234));
        }
    }

    #[test]
    fn four_byte_ints() {
        for ty in [0x20u8, 0x22, 0x58, 0x68, 0x70, 0xB8] {
            assert_eq!(
                one(&[1, ty, 0x78, 0x56, 0x34, 0x12]).value,
                Value::Int(0x12345678)
            );
        }
    }

    #[test]
    fn eight_byte_int() {
        let it = one(&[1, 0x28, 1, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(it.value, Value::Int(0x2_0000_0001));
    }

    #[test]
    fn wide_fixed_payloads_are_bytes() {
        let mut b = vec![1, 0x38];
        b.extend(0..16u8);
        assert_eq!(one(&b).value, Value::Bytes((0..16u8).collect()));
        let mut b = vec![1, 0x48];
        b.extend(0..24u8);
        assert_eq!(one(&b).value, Value::Bytes((0..24u8).collect()));
    }

    #[test]
    fn nested_record_and_list() {
        let inner = block(&[1, 0x10, 5, 0]);
        let mut rec = vec![9, 0x88];
        rec.extend(&inner);
        match one(&rec).value {
            Value::Record(v) => assert_eq!(v[0].value, Value::Int(5)),
            other => panic!("{other:?}"),
        }
        let list_body = [0, 0x78, 0, 0x10, 3, 0];
        let mut lst = vec![2, 0x90];
        lst.extend(block(&list_body));
        match one(&lst).value {
            Value::List(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0].value, Value::Flag);
                assert_eq!(v[1].value, Value::Int(3));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn nested_record_type_8a() {
        let mut rec = vec![9, 0x8A];
        rec.extend(block(&[1, 0x12, 5, 0]));
        match one(&rec).value {
            Value::Record(v) => assert_eq!(v[0].value, Value::Int(5)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn raw_blobs() {
        for ty in [0x80u8, 0xC0] {
            let mut b = vec![4, ty];
            b.extend(8u32.to_le_bytes());
            b.extend([b'A', 0, b'B', 0]);
            assert_eq!(one(&b).value, Value::Raw(vec![b'A', 0, b'B', 0]));
        }
    }

    #[test]
    fn flagged_variants_use_class_sizes() {
        for ty in [0x01u8, 0x02, 0x06, 0x07, 0x0A] {
            assert_eq!(one(&[1, ty]).value, Value::Flag);
        }
        for ty in [0x12u8, 0x1A] {
            let it = one(&[1, ty, 0x34, 0x12]);
            assert_eq!((it.class, it.value), (ty >> 3, Value::Int(0x1234)));
        }
        for ty in [0x22u8, 0x23, 0x5C] {
            let it = one(&[1, ty, 0x78, 0x56, 0x34, 0x12]);
            assert_eq!((it.class, it.value), (ty >> 3, Value::Int(0x12345678)));
        }
        let mut b = vec![4, 0x82];
        b.extend(6u32.to_le_bytes());
        b.extend([7, 8]);
        let it = one(&b);
        assert_eq!((it.class, it.value), (0x82 >> 3, Value::Raw(vec![7, 8])));
        let mut rec = vec![9, 0x8A];
        rec.extend(block(&[1, 0x10, 5, 0]));
        assert_eq!(one(&rec).class, 0x8A >> 3);
        let mut lst = vec![2, 0x90];
        lst.extend(block(&[0, 0xE0, 0, 0x78, 0, 0x10, 3, 0]));
        match one(&lst).value {
            Value::List(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!((v[0].class, &v[0].value), (0xE0 >> 3, &Value::Flag));
                assert_eq!(v[2].value, Value::Int(3));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_class_has_a_size() {
        let mut b = vec![1, 0x40];
        b.extend(0..20u8);
        assert_eq!(one(&b).value, Value::Bytes((0..20u8).collect()));
        let mut b = vec![1, 0x30];
        b.extend(0..12u8);
        assert_eq!(one(&b).value, Value::Bytes((0..12u8).collect()));
        let mut b = vec![1, 0x50];
        b.extend(0..32u8);
        assert_eq!(one(&b).value, Value::Bytes((0..32u8).collect()));
        assert_eq!(one(&[1, 0x60, 1, 0, 0, 0, 0, 0, 0, 0]).value, Value::Int(1));
        // 0x43 is class 8 as well: only 4 payload bytes is an error.
        assert!(decode_block(&block(&[1, 0x43, 0, 0, 0, 0]), 0).is_err());
    }

    #[test]
    fn empty_classes_read_no_payload() {
        for ty in [0xA8u8, 0xB0, 0xE8, 0xF0, 0xF8, 0xFF] {
            assert_eq!(one(&[3, ty]).value, Value::Flag);
        }
    }

    #[test]
    fn size_running_past_block_end_is_an_error_with_offset() {
        let data = block(&[1, 0x08, 2, 0x40, 0, 0]);
        match decode_block(&data, 0) {
            Err(PubError::Format { offset, .. }) => assert_eq!(offset, 8),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn misaligned_block_is_an_error() {
        // A 4-byte int whose payload runs past the end of the block.
        assert!(decode_block(&block(&[1, 0x20, 1, 2]), 0).is_err());
        // A lone trailing byte.
        assert!(decode_block(&block(&[1, 0x08, 9]), 0).is_err());
    }

    #[test]
    fn truncated_and_hostile_input_never_panics() {
        let mut good = vec![1, 0x88];
        good.extend(block(&[
            1, 0x20, 1, 2, 3, 4, 5, 0xC0, 8, 0, 0, 0, 1, 2, 3, 4,
        ]));
        let data = block(&good);
        assert!(decode_block(&data, 0).is_ok());
        for cut in 0..data.len() {
            let _ = decode_block(&data[..cut], 0);
        }
        for off in 0..data.len() + 3 {
            let _ = decode_block(&data, off);
        }
        assert!(decode_block(&[], 0).is_err());
        assert!(decode_block(&[0xFF; 8], 0).is_err());
        assert!(decode_block(&[3, 0, 0, 0], 0).is_err());
    }
}
