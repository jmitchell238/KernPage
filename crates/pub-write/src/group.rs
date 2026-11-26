//! Group / Ungroup (spec 4J and 4J-b): the Escher group container and the `Contents`
//! kind-48 object, written the way Publisher 2010 writes them.

use super::{bounding_box, escher_header, Result, WriteError};
use cfb::CompoundFile;
use pub_format::items::{decode_block, find_int, locate_int};
use pub_format::Streams;
use std::collections::HashSet;
use std::io::{Cursor, Seek, SeekFrom, Write};

/// Page-centre-relative EMU -> the group coordinate space (spec 4J-b: 110,185,200 on both axes).
pub(crate) const ORIGIN: i64 = 110_185_200;
pub(crate) const KIND_GROUP: u64 = 48;
pub(crate) const KIND_FONT_TABLE: u64 = 108;
/// Index entry of a fresh group object: `{2: 48, 4: offset, 5: parent, 6: 258, 11: 22}`.
pub(crate) const GROUP_ENTRY: [u8; 30] = [
    0x00, 0x88, 0x1c, 0, 0, 0, 0x02, 0x18, 0x30, 0, 0x04, 0xb8, 0, 0, 0, 0, 0x05, 0x68, 0, 0, 0, 0,
    0x06, 0x10, 0x02, 0x01, 0x0b, 0x18, 0x16, 0,
];

/// A 30-byte index entry (spec 4.4, 6.2.5): kind, offset (set later), parent, the u16 of
/// item 6, optionally the flag item 8, and item 11.
pub(crate) fn index_entry(
    kind: u16,
    parent: u32,
    item6: u16,
    flag8: bool,
    item11: u16,
) -> Vec<u8> {
    let mut e = vec![0x00, 0x88, 0, 0, 0, 0];
    e.extend_from_slice(&[0x02, 0x18]);
    e.extend_from_slice(&kind.to_le_bytes());
    e.extend_from_slice(&[0x04, 0xb8, 0, 0, 0, 0, 0x05, 0x68]);
    e.extend_from_slice(&parent.to_le_bytes());
    e.extend_from_slice(&[0x06, 0x10]);
    e.extend_from_slice(&item6.to_le_bytes());
    if flag8 {
        e.extend_from_slice(&[0x08, 0x08]);
    }
    e.extend_from_slice(&[0x0b, 0x18]);
    e.extend_from_slice(&item11.to_le_bytes());
    let n = (e.len() - 2) as u32;
    e[2..6].copy_from_slice(&n.to_le_bytes());
    e
}

pub(crate) fn err<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Group(m.into()))
}

// ---------------------------------------------------------------- Escher tree

#[derive(Clone)]
pub(crate) struct Rec {
    pub(crate) vi: u16,
    pub(crate) ty: u16,
    pub(crate) body: Body,
}
#[derive(Clone)]
pub(crate) enum Body {
    Atom(Vec<u8>),
    Cont(Vec<Rec>),
}

impl Rec {
    pub(crate) fn atom(vi: u16, ty: u16, data: Vec<u8>) -> Rec {
        Rec {
            vi,
            ty,
            body: Body::Atom(data),
        }
    }
    pub(crate) fn kids(&self) -> &[Rec] {
        match &self.body {
            Body::Cont(k) => k,
            Body::Atom(_) => &[],
        }
    }
    pub(crate) fn kids_mut(&mut self) -> Result<&mut Vec<Rec>> {
        match &mut self.body {
            Body::Cont(k) => Ok(k),
            Body::Atom(_) => err("unexpected Escher layout"),
        }
    }
    pub(crate) fn data_mut(&mut self) -> Result<&mut Vec<u8>> {
        match &mut self.body {
            Body::Atom(d) => Ok(d),
            Body::Cont(_) => err("unexpected Escher layout"),
        }
    }
    pub(crate) fn find(&self, ty: u16) -> Option<&Rec> {
        self.kids().iter().find(|r| r.ty == ty)
    }
    /// `Contents` object of a shape container, or of a group container's own shape.
    pub(crate) fn object(&self) -> Option<u32> {
        match self.ty {
            0xF004 => {
                let Body::Atom(d) = &self.find(0xF011)?.body else {
                    return None;
                };
                find_int(&decode_block(d, 0).ok()?, 1).map(|v| v as u32)
            }
            0xF003 => self.kids().first()?.object(),
            _ => None,
        }
    }
}

pub(crate) fn parse_recs(d: &[u8], mut pos: usize, end: usize, depth: usize) -> Result<Vec<Rec>> {
    if depth > 32 {
        return err("Escher nesting too deep");
    }
    let mut out = Vec::new();
    while pos < end {
        let h =
            escher_header(d, pos).ok_or_else(|| WriteError::Group("truncated Escher".into()))?;
        let next = pos + 8 + h.len;
        if next > end {
            return err("Escher record extends past its container");
        }
        let vi = u16::from_le_bytes([d[pos], d[pos + 1]]);
        let body = if h.container {
            Body::Cont(parse_recs(d, pos + 8, next, depth + 1)?)
        } else {
            Body::Atom(d[pos + 8..next].to_vec())
        };
        out.push(Rec { vi, ty: h.ty, body });
        pos = next;
    }
    Ok(out)
}

pub(crate) fn write_rec(r: &Rec, out: &mut Vec<u8>) {
    let at = out.len();
    out.extend_from_slice(&r.vi.to_le_bytes());
    out.extend_from_slice(&r.ty.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    match &r.body {
        Body::Atom(d) => out.extend_from_slice(d),
        Body::Cont(k) => k.iter().for_each(|c| write_rec(c, out)),
    }
    let len = (out.len() - at - 8) as u32;
    out[at + 4..at + 8].copy_from_slice(&len.to_le_bytes());
}

pub(crate) struct EscherDoc {
    pub(crate) head: Rec,
    pub(crate) drawings: Vec<([u8; 4], Rec)>,
}

impl EscherDoc {
    pub(crate) fn parse(d: &[u8]) -> Result<Self> {
        let h = escher_header(d, 0).ok_or_else(|| WriteError::Group("no drawing group".into()))?;
        let mut pos = 8 + h.len;
        let mut head = parse_recs(d, 0, pos, 0)?;
        if head.len() != 1 {
            return err("unexpected drawing group");
        }
        let mut drawings = Vec::new();
        while pos < d.len() {
            let prefix: [u8; 4] = d
                .get(pos..pos + 4)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| WriteError::Group("truncated drawing prefix".into()))?;
            pos += 4;
            let h = escher_header(d, pos)
                .ok_or_else(|| WriteError::Group("truncated drawing".into()))?;
            let end = pos + 8 + h.len;
            let mut r = parse_recs(d, pos, end.min(d.len()), 0)?;
            if end > d.len() || r.len() != 1 {
                return err("unexpected drawing layout");
            }
            drawings.push((prefix, r.remove(0)));
            pos = end;
        }
        let doc = EscherDoc {
            head: head.remove(0),
            drawings,
        };
        if doc.to_bytes() != d {
            return err("EscherStm does not re-serialise identically");
        }
        Ok(doc)
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_rec(&self.head, &mut out);
        for (p, r) in &self.drawings {
            out.extend_from_slice(p);
            write_rec(r, &mut out);
        }
        out
    }
}

pub(crate) fn rd(d: &[u8], at: usize) -> Result<u32> {
    d.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| WriteError::Group("short Escher record".into()))
}
pub(crate) fn put(d: &mut [u8], at: usize, v: u32) -> Result<()> {
    d.get_mut(at..at + 4)
        .map(|b| b.copy_from_slice(&v.to_le_bytes()))
        .ok_or_else(|| WriteError::Group("short Escher record".into()))
}

/// Path (child indexes) to the first record satisfying `pred`, depth first.
pub(crate) fn find_path(recs: &[Rec], pred: &dyn Fn(&Rec) -> bool) -> Option<Vec<usize>> {
    for (i, r) in recs.iter().enumerate() {
        if pred(r) {
            return Some(vec![i]);
        }
        if let Some(mut p) = find_path(r.kids(), pred) {
            p.insert(0, i);
            return Some(p);
        }
    }
    None
}

pub(crate) fn rec_at<'a>(root: &'a mut Rec, path: &[usize]) -> Result<&'a mut Rec> {
    let mut cur = root;
    for &i in path {
        cur = cur
            .kids_mut()?
            .get_mut(i)
            .ok_or_else(|| WriteError::Group("bad Escher path".into()))?;
    }
    Ok(cur)
}

/// The four ints of a 28-byte client anchor (0xF010: `1c000000` + items 1..4, 4-byte ints).
pub(crate) fn client_anchor(d: &[u8]) -> Result<[i32; 4]> {
    if d.len() != 28 {
        return err("unsupported client anchor layout");
    }
    let items = decode_block(d, 0)?;
    let mut v = [0i32; 4];
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = find_int(&items, i as u16 + 1)
            .ok_or_else(|| WriteError::Group("client anchor lacks a side".into()))?
            as u32 as i32;
    }
    Ok(v)
}

pub(crate) fn client_anchor_bytes(v: [i32; 4]) -> Vec<u8> {
    let mut d = vec![0x1c, 0, 0, 0];
    for (i, x) in v.iter().enumerate() {
        d.extend_from_slice(&[i as u8 + 1, 0x20]);
        d.extend_from_slice(&x.to_le_bytes());
    }
    d
}

pub(crate) fn anchor_atom(v: [i32; 4]) -> Rec {
    Rec::atom(0x018a, 0xF010, client_anchor_bytes(v))
}

pub(crate) fn ints16(v: [i64; 4]) -> Result<Vec<u8>> {
    let mut d = Vec::new();
    for x in v {
        let x = i32::try_from(x).map_err(|_| WriteError::OutOfRange(x))?;
        d.extend_from_slice(&x.to_le_bytes());
    }
    Ok(d)
}

pub(crate) fn drawing_counters(
    drawing: &mut Rec,
    cspids_delta: i32,
    new_lastspid: Option<u32>,
) -> Result<u32> {
    let f8 = drawing
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF008)
        .ok_or_else(|| WriteError::Group("drawing has no 0xF008".into()))?;
    let d = f8.data_mut()?;
    let cspids = rd(d, 0)?.wrapping_add_signed(cspids_delta);
    put(d, 0, cspids)?;
    let last = new_lastspid.unwrap_or(rd(d, 4)?);
    put(d, 4, last)?;
    Ok(last)
}

pub(crate) fn group_counters(
    head: &mut Rec,
    dgid: u16,
    csaved_delta: i32,
    fidcl_delta: i32,
) -> Result<()> {
    let f6 = head
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF006)
        .ok_or_else(|| WriteError::Group("no 0xF006".into()))?;
    let d = f6.data_mut()?;
    let saved = rd(d, 8)?.wrapping_add_signed(csaved_delta);
    put(d, 8, saved)?;
    if fidcl_delta != 0 {
        let pairs = rd(d, 4)?.saturating_sub(1) as usize;
        let mut done = false;
        for i in 0..pairs {
            let at = 16 + 8 * i;
            if rd(d, at)? == u32::from(dgid) {
                let c = rd(d, at + 4)?.wrapping_add_signed(fidcl_delta);
                put(d, at + 4, c)?;
                done = true;
            }
        }
        if !done {
            return err(format!(
                "drawing {dgid} has no FIDCL entry ({pairs} pairs, {:02x?})",
                &d[..d.len().min(40)]
            ));
        }
    }
    Ok(())
}

// ------------------------------------------------------------ Contents editing

pub(crate) fn skip_item(d: &[u8], pos: usize, end: usize) -> Result<usize> {
    let ty = *d
        .get(pos + 1)
        .ok_or_else(|| WriteError::Group("short block".into()))?;
    let (fixed, prefixed) = match ty >> 3 {
        0 | 1 | 15 | 21 | 22 | 28..=31 => (0, false),
        2 | 3 => (2, false),
        4 | 11 | 13 | 14 | 23 => (4, false),
        5 | 12 => (8, false),
        6 => (12, false),
        7 => (16, false),
        8 => (20, false),
        9 => (24, false),
        10 => (32, false),
        _ => (0, true),
    };
    let skip = if prefixed {
        rd(d, pos + 2)? as usize
    } else {
        fixed
    };
    let next = pos + 2 + skip;
    if next > end || (prefixed && skip < 4) {
        return err("malformed Contents item");
    }
    Ok(next)
}

/// Top-level items of the block at `off`: (id, header position, payload position, end).
pub(crate) fn items_of(d: &[u8], off: usize) -> Result<Vec<(u16, usize, usize, usize)>> {
    let end = off + rd(d, off)? as usize;
    if end > d.len() {
        return err("Contents block past the stream");
    }
    let mut pos = off + 4;
    let mut out = Vec::new();
    while pos < end {
        let next = skip_item(d, pos, end)?;
        let (id, _) = pub_format::items::split_header(d[pos], d[pos + 1]);
        out.push((id, pos, pos + 2, next));
        pos = next;
    }
    Ok(out)
}

pub(crate) fn int_at(d: &[u8], at: usize, len: usize) -> usize {
    d[at..at + len]
        .iter()
        .enumerate()
        .fold(0, |a, (i, &b)| a | (usize::from(b) << (8 * i)))
}

pub(crate) struct Cts {
    /// Header and object blocks (everything before the index).
    pub(crate) blocks: Vec<u8>,
    idx: Vec<u8>,
    list_payload: usize,
    list_end: usize,
    pub(crate) entries: Vec<Vec<u8>>,
}

impl Cts {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 0x2c {
            return err("short Contents");
        }
        let io = rd(data, 0x1A)? as usize;
        let idx = data
            .get(io..)
            .ok_or_else(|| WriteError::Group("bad index offset".into()))?
            .to_vec();
        if rd(&idx, 0)? as usize != idx.len() {
            return err("the object index is not the last block of Contents");
        }
        let it = items_of(&idx, 0)?;
        let &(_, _, pl, end) = it
            .iter()
            .find(|i| i.0 == 3)
            .ok_or_else(|| WriteError::Group("no object list".into()))?;
        let mut entries = Vec::new();
        let mut pos = pl + 4;
        while pos < end {
            let next = skip_item(&idx, pos, end)?;
            entries.push(idx[pos..next].to_vec());
            pos = next;
        }
        Ok(Cts {
            blocks: data[..io].to_vec(),
            idx,
            list_payload: pl,
            list_end: end,
            entries,
        })
    }

    pub(crate) fn int(&self, slot: usize, id: u16) -> Option<usize> {
        let e = self.entries.get(slot)?;
        let s = locate_int(e, 2, id).ok().flatten()?;
        Some(int_at(e, s.offset, s.len))
    }
    pub(crate) fn set_int(&mut self, slot: usize, id: u16, v: usize) -> Result<()> {
        let e = &mut self.entries[slot];
        let s = locate_int(e, 2, id)
            .ok()
            .flatten()
            .ok_or_else(|| WriteError::Group(format!("object {slot} lacks index item {id}")))?;
        if s.len < 8 && v >> (8 * s.len) != 0 {
            return err("value does not fit an index item");
        }
        for i in 0..s.len {
            e[s.offset + i] = (v >> (8 * i)) as u8;
        }
        Ok(())
    }
    pub(crate) fn kind(&self, slot: usize) -> Option<u64> {
        self.int(slot, 2).map(|v| v as u64)
    }
    pub(crate) fn offset(&self, slot: usize) -> Result<usize> {
        self.int(slot, 4)
            .ok_or_else(|| WriteError::Group(format!("object {slot} has no offset")))
    }
    pub(crate) fn parent(&self, slot: usize) -> Result<usize> {
        self.int(slot, 5)
            .ok_or_else(|| WriteError::Group(format!("object {slot} has no parent")))
    }
    /// The font-table object's slot, when the file has one. Publisher creates it together
    /// with the first story, so a blank file and a file with only shapes have none (spec
    /// 6.11). It is always the last slot; anything else is refused.
    /// The font table's slot wherever it is: Publisher leaves it in the middle once rows or
    /// columns of a table were inserted or deleted (spec 4E.1.5).
    pub(crate) fn font_slot_any(&self) -> Option<usize> {
        (0..self.entries.len())
            .rev()
            .find(|&i| self.kind(i) == Some(KIND_FONT_TABLE))
    }
    pub(crate) fn font_slot_opt(&self) -> Result<Option<usize>> {
        let Some(f) = (0..self.entries.len())
            .rev()
            .find(|&i| self.kind(i) == Some(KIND_FONT_TABLE))
        else {
            return Ok(None);
        };
        if f + 1 != self.entries.len() {
            return err("the font-table object is not the last slot");
        }
        Ok(Some(f))
    }
    /// The first slot of the first object that can still be added and the offset where its
    /// body goes: the font table's slot and body when there is one (new objects go in front
    /// of it), else the end of the object list and of the object blocks.
    pub(crate) fn tail(&self) -> Result<(usize, usize)> {
        match self.font_slot_any() {
            Some(f) if f + 1 == self.entries.len() => Ok((f, self.offset(f)?)),
            _ => Ok((self.entries.len(), self.blocks.len())),
        }
    }
    /// Put new index entries at slot `slot` (a value from [`Cts::tail`]); the font table, if
    /// there is one, moves up behind them and the one reference to it follows.
    pub(crate) fn insert_entries(&mut self, slot: usize, entries: Vec<Vec<u8>>) -> Result<()> {
        let n = entries.len();
        if self.kind(slot) == Some(KIND_FONT_TABLE) {
            let fp = self.parent(slot)?;
            self.entries.splice(slot..slot, entries);
            self.repoint(fp, slot, slot + n)
        } else if slot == self.entries.len() {
            self.entries.extend(entries);
            Ok(())
        } else {
            err("new objects must go in front of the font table or at the end")
        }
    }
    /// The first object of a kind.
    pub(crate) fn slot_of_kind(&self, kind: u64) -> Option<usize> {
        (0..self.entries.len()).find(|&i| self.kind(i) == Some(kind))
    }
    /// Shift the stored offset of every object block starting after `at` (or at it when
    /// `inclusive`) by `delta`.
    pub(crate) fn shift(&mut self, at: usize, inclusive: bool, delta: isize) -> Result<()> {
        for i in 0..self.entries.len() {
            if let Some(o) = self.int(i, 4) {
                if o > at || (inclusive && o == at) {
                    self.set_int(i, 4, o.wrapping_add_signed(delta))?;
                }
            }
        }
        Ok(())
    }
    /// Replace `blocks[at..at + remove]` with `ins`, keeping later offsets right.
    pub(crate) fn splice(&mut self, at: usize, remove: usize, ins: &[u8]) -> Result<()> {
        self.blocks.splice(at..at + remove, ins.iter().copied());
        self.shift(at, false, ins.len() as isize - remove as isize)
    }
    /// The one 4-byte int item of `slot`'s block holding `from`, set to `to`.
    pub(crate) fn repoint(&mut self, slot: usize, from: usize, to: usize) -> Result<()> {
        let off = self.offset(slot)?;
        let mut hit = None;
        for (id, _, p, e) in items_of(&self.blocks, off)? {
            if e - p == 4
                && matches!(self.blocks[p - 1] >> 3, 4 | 11 | 13 | 14 | 23)
                && int_at(&self.blocks, p, 4) == from
            {
                if hit.is_some() {
                    return err(format!(
                        "object {slot} refers to slot {from} twice (item {id})"
                    ));
                }
                hit = Some(p);
            }
        }
        let p = hit.ok_or_else(|| {
            WriteError::Group(format!("object {slot} does not refer to slot {from}"))
        })?;
        self.blocks[p..p + 4].copy_from_slice(&(to as u32).to_le_bytes());
        Ok(())
    }
    /// The page or group's child list; empty when the object has none (a blank page has
    /// neither item 1 nor item 2, spec 4P.1).
    pub(crate) fn child_ids(&self, slot: usize) -> Result<Vec<u32>> {
        let off = self.offset(slot)?;
        let it = items_of(&self.blocks, off)?;
        let Some(&(_, _, p, e)) = it.iter().find(|i| i.0 == 2) else {
            return Ok(Vec::new());
        };
        list_ids(&self.blocks, p, e)
    }
    pub(crate) fn set_child_ids(&mut self, slot: usize, ids: &[u32]) -> Result<()> {
        let off = self.offset(slot)?;
        let mut it = items_of(&self.blocks, off)?;
        if !it.iter().any(|i| i.0 == 2) {
            // Items 1 (count) and 2 (list) come before the first higher item.
            if it.iter().any(|i| i.0 == 1) {
                return err("page object has a child count but no child list");
            }
            let at = it
                .iter()
                .find(|i| i.0 > 2)
                .map_or(off + rd(&self.blocks, off)? as usize, |i| i.1);
            let mut ins = vec![0x01, 0x20, 0, 0, 0, 0, 0x02, 0xa0];
            ins.extend_from_slice(&list_bytes(&[]));
            self.splice(at, 0, &ins)?;
            let len = rd(&self.blocks, off)? as usize + ins.len();
            put(&mut self.blocks, off, len as u32)?;
            it = items_of(&self.blocks, off)?;
        }
        let find = |id: u16| it.iter().find(|i| i.0 == id).copied();
        let (_, _, lp, le) = find(2).ok_or_else(|| WriteError::Group("no child list".into()))?;
        let (_, _, cp, ce) = find(1).ok_or_else(|| WriteError::Group("no child count".into()))?;
        let old = list_ids(&self.blocks, lp, le)?.len();
        if ce - cp != 4 {
            return err("unexpected child count item");
        }
        let count = int_at(&self.blocks, cp, 4) as i64 + ids.len() as i64 - old as i64;
        self.blocks[cp..cp + 4].copy_from_slice(&(count.max(0) as u32).to_le_bytes());
        let new = list_bytes(ids);
        let delta = new.len() as isize - (le - lp) as isize;
        self.splice(lp, le - lp, &new)?;
        let len = (rd(&self.blocks, off)? as isize + delta) as u32;
        put(&mut self.blocks, off, len)
    }

    pub(crate) fn finish(self) -> Result<Vec<u8>> {
        let index_off = self.blocks.len();
        let mut list = vec![0u8; 4];
        self.entries.iter().for_each(|e| list.extend_from_slice(e));
        let n = list.len() as u32;
        list[..4].copy_from_slice(&n.to_le_bytes());
        let slots_delta = self.entries.len() as i64 - {
            let old = rd(&self.idx, self.list_payload)? as usize;
            let mut pos = self.list_payload + 4;
            let mut k = 0i64;
            while pos < self.list_payload + old {
                pos = skip_item(&self.idx, pos, self.list_payload + old)?;
                k += 1;
            }
            k
        };
        let mut idx = self.idx[..self.list_payload].to_vec();
        idx.extend_from_slice(&list);
        idx.extend_from_slice(&self.idx[self.list_end..]);
        // Items 1 and 2 of the index: number of slots and the last slot.
        for (id, _, p, e) in items_of(&self.idx, 0)? {
            if (id == 1 || id == 2) && e - p == 4 {
                let v = (int_at(&self.idx, p, 4) as i64 + slots_delta) as u32;
                idx[p..p + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        let total = idx.len() as u32;
        idx[..4].copy_from_slice(&total.to_le_bytes());
        let mut out = self.blocks;
        out.extend_from_slice(&idx);
        put(&mut out, 0x1A, index_off as u32)?;
        let len = out.len() as u32;
        put(&mut out, 8, len)?;
        Ok(out)
    }
}

pub(crate) fn list_ids(d: &[u8], payload: usize, end: usize) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    let mut pos = payload + 4;
    while pos < end {
        if end - pos < 6 || d[pos + 1] >> 3 != 14 {
            return err("unexpected child list entry");
        }
        out.push(rd(d, pos + 2)?);
        pos += 6;
    }
    Ok(out)
}

pub(crate) fn list_bytes(ids: &[u32]) -> Vec<u8> {
    let mut d = ((4 + 6 * ids.len()) as u32).to_le_bytes().to_vec();
    for id in ids {
        d.extend_from_slice(&[0x00, 0x70]);
        d.extend_from_slice(&id.to_le_bytes());
    }
    d
}

pub(crate) fn group_body(children: &[u32]) -> Vec<u8> {
    let mut b = vec![0u8; 4];
    b.extend_from_slice(&[0x02, 0x08, 0x04, 0x10, 0, 0]);
    b.extend_from_slice(&[0x0c, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x0d, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x14, 0x20]);
    b.extend_from_slice(&(children.len() as u32).to_le_bytes());
    b.extend_from_slice(&[0x15, 0xa0]);
    b.extend_from_slice(&list_bytes(children));
    b.extend_from_slice(&[0xb7, 0x20, 0, 0, 0, 0]);
    let n = b.len() as u32;
    b[..4].copy_from_slice(&n.to_le_bytes());
    b
}

// ------------------------------------------------------------------- public API

pub(crate) fn install(input: &[u8], contents: &[u8], escher: &[u8]) -> Result<Vec<u8>> {
    let mut cf = CompoundFile::open(Cursor::new(input.to_vec()))?;
    for (path, data) in [("/Contents", contents), ("/Escher/EscherStm", escher)] {
        let mut s = cf.open_stream(path)?;
        s.set_len(data.len() as u64)?;
        s.seek(SeekFrom::Start(0))?;
        s.write_all(data)?;
    }
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}

/// Group the given top-level shapes of a page (spec 4J-b A and B). The group container
/// takes the slot of the topmost selected shape; children keep their relative order.
/// Rotated or nested groups are not supported (an error is returned).
pub fn group_shapes(input: &[u8], page_index: usize, shape_indexes: &[usize]) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .pages
        .get(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let mut wanted = HashSet::new();
    let mut union: Option<[f64; 4]> = None;
    for &i in shape_indexes {
        let s = page.shapes.get(i).ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: i,
        })?;
        if !wanted.insert(s.object_number) {
            return err("a shape was selected twice");
        }
        if s.group.is_some() || page.shapes.iter().any(|t| t.group == Some(s.object_number)) {
            return err("nested groups are not supported yet");
        }
        let b = bounding_box(s);
        union = Some(union.map_or(b, |u| {
            [
                u[0].min(b[0]),
                u[1].min(b[1]),
                u[2].max(b[2]),
                u[3].max(b[3]),
            ]
        }));
    }
    let Some(union) = union.filter(|_| wanted.len() >= 2) else {
        return err("select at least two shapes");
    };
    let anchor = union.map(|v| v.round() as i64);

    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;

    // Locate the page's group container and the selected shape containers.
    let pred = |r: &Rec| {
        r.ty == 0xF003
            && r.kids()
                .iter()
                .any(|k| k.ty == 0xF004 && k.object().is_some_and(|o| wanted.contains(&o)))
    };
    let (di, path) = esc
        .drawings
        .iter()
        .enumerate()
        .find_map(|(di, (_, d))| find_path(std::slice::from_ref(d), &pred).map(|p| (di, p)))
        .ok_or_else(|| WriteError::Group("selected shapes not found in the drawing".into()))?;
    let dgid = esc.drawings[di].1.find(0xF008).map_or(0, |r| r.vi >> 4);
    let path = &path[1..];
    let ordered: Vec<(usize, u32)> = {
        let g = rec_at(&mut esc.drawings[di].1, path)?;
        g.kids()
            .iter()
            .enumerate()
            .filter(|(_, k)| k.ty == 0xF004)
            .filter_map(|(i, k)| k.object().filter(|o| wanted.contains(o)).map(|o| (i, o)))
            .collect()
    };
    if ordered.len() != wanted.len() {
        return err("the selected shapes are not all direct children of one page container");
    }
    let objs: Vec<u32> = ordered.iter().map(|x| x.1).collect();

    // Contents: group object takes the font table's slot.
    let (font, at) = cts.tail()?;
    let page_obj = cts.parent(objs[0] as usize)?;
    if objs
        .iter()
        .any(|&o| cts.parent(o as usize).ok() != Some(page_obj))
    {
        return err("the selected shapes have different parents");
    }
    let body = group_body(&objs);
    cts.blocks.splice(at..at, body.iter().copied());
    cts.shift(at, true, body.len() as isize)?;
    let mut entry = GROUP_ENTRY.to_vec();
    entry[10 + 2..10 + 6].copy_from_slice(&(at as u32).to_le_bytes());
    cts.insert_entries(font, vec![entry])?;
    let _ = KIND_GROUP;
    cts.set_int(font, 5, page_obj)?;
    for &o in &objs {
        cts.set_int(o as usize, 5, font)?;
    }
    let mut kids = cts.child_ids(page_obj)?;
    let last = kids
        .iter()
        .rposition(|k| objs.contains(k))
        .ok_or_else(|| WriteError::Group("the page does not list the shapes".into()))?;
    // Publisher puts the group at the *index* of the last grouped child in the old list
    // (spec 4J-b B shows this differs from the Escher z-order slot for some selections).
    kids.retain(|k| !objs.contains(k));
    kids.insert(last.min(kids.len()), font as u32);
    cts.set_child_ids(page_obj, &kids)?;

    // Escher.
    let drawing = &mut esc.drawings[di].1;
    let spid = {
        let f8 = drawing.find(0xF008).and_then(|r| match &r.body {
            Body::Atom(d) => rd(d, 4).ok(),
            Body::Cont(_) => None,
        });
        f8.ok_or_else(|| WriteError::Group("drawing has no 0xF008".into()))? + 1
    };
    drawing_counters(drawing, 1, Some(spid))?;
    group_counters(&mut esc.head, dgid, 1, 1)?;
    let drawing = &mut esc.drawings[di].1;
    let g = rec_at(drawing, path)?;
    let kids = g.kids_mut()?;
    let mut children = Vec::new();
    for (i, _) in ordered.iter().rev() {
        children.insert(0, kids.remove(*i));
    }
    for c in &mut children {
        to_child(c)?;
    }
    let fspgr = ints16(anchor.map(|v| v + ORIGIN))?;
    let mut shape = vec![Rec::atom(0x0001, 0xF009, fspgr)];
    let mut fsp = spid.to_le_bytes().to_vec();
    fsp.extend_from_slice(&0x201u32.to_le_bytes());
    shape.push(Rec::atom(0x0002, 0xF00A, fsp));
    shape.push(Rec::atom(
        0x0013,
        0xF122,
        vec![0xff, 0x06, 0x02, 0x00, 0x02, 0x00],
    ));
    let a = anchor.map(|v| v as i32);
    shape.push(anchor_atom(a));
    let mut cd = vec![0x0a, 0, 0, 0, 0x01, 0x68];
    cd.extend_from_slice(&(font as u32).to_le_bytes());
    shape.push(Rec::atom(0x018a, 0xF011, cd));
    let mut group = vec![Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(shape),
    }];
    group.append(&mut children);
    let slot = ordered[ordered.len() - 1].0 - (ordered.len() - 1);
    kids.insert(
        slot,
        Rec {
            vi: 0x000F,
            ty: 0xF003,
            body: Body::Cont(group),
        },
    );
    install(input, &cts.finish()?, &esc.to_bytes())
}

/// Turn a top-level shape container into a group child: fChild flag, child anchor.
fn to_child(c: &mut Rec) -> Result<()> {
    for r in c.kids_mut()? {
        match r.ty {
            0xF00A => {
                let d = r.data_mut()?;
                let f = rd(d, 4)? | 2;
                put(d, 4, f)?;
            }
            0xF010 => {
                let a = client_anchor(r.data_mut()?)?;
                *r = Rec::atom(0x0000, 0xF00F, ints16(a.map(|v| i64::from(v) + ORIGIN))?);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Ungroup the group at `group_index` of the page's shape list, as Publisher does
/// (spec 4J-b C), including property 0x388 = 1 on the children and the 0xF118 record.
pub fn ungroup(input: &[u8], page_index: usize, group_index: usize) -> Result<Vec<u8>> {
    ungroup_with(input, page_index, group_index, true)
}

/// Like [`ungroup`]; `publisher_extras = false` leaves out property 0x388 and record 0xF118.
pub fn ungroup_with(
    input: &[u8],
    page_index: usize,
    group_index: usize,
    publisher_extras: bool,
) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .pages
        .get(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let g = page
        .shapes
        .get(group_index)
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: group_index,
        })?;
    let members: Vec<_> = page
        .shapes
        .iter()
        .filter(|s| s.group == Some(g.object_number))
        .collect();
    if members.is_empty() {
        return err("the shape is not a group");
    }
    if g.group.is_some()
        || members
            .iter()
            .any(|m| page.shapes.iter().any(|t| t.group == Some(m.object_number)))
    {
        return err("nested groups are not supported yet");
    }
    if g.rotation_deg != 0.0 {
        return Err(WriteError::Rotated(g.rotation_deg));
    }
    let gobj = g.object_number;

    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;

    let slot = gobj as usize;
    if cts.kind(slot) != Some(KIND_GROUP) {
        return err("the group has no kind-48 object");
    }
    let parent = cts.parent(slot)?;
    let ids = {
        let off = cts.offset(slot)?;
        let it = items_of(&cts.blocks, off)?;
        let &(_, _, p, e) = it
            .iter()
            .find(|i| i.0 == 21)
            .ok_or_else(|| WriteError::Group("group has no child list".into()))?;
        list_ids(&cts.blocks, p, e)?
    };
    for &c in &ids {
        cts.set_int(c as usize, 5, parent)?;
    }
    let mut kids = cts.child_ids(parent)?;
    let pos = kids
        .iter()
        .position(|&k| k == gobj)
        .ok_or_else(|| WriteError::Group("the page does not list the group".into()))?;
    kids.splice(pos..=pos, ids.iter().copied());
    cts.set_child_ids(parent, &kids)?;
    let at = cts.offset(slot)?;
    let len = rd(&cts.blocks, at)? as usize;
    cts.blocks.drain(at..at + len);
    cts.shift(at, false, -(len as isize))?;
    let font = cts.font_slot_opt()?.unwrap_or(cts.entries.len());
    if slot + 1 == font {
        let fp = cts.font_slot_opt()?.map(|f| cts.parent(f)).transpose()?;
        cts.entries.remove(slot);
        if let Some(fp) = fp {
            cts.repoint(fp, font, slot)?;
        }
    } else {
        cts.entries[slot] = vec![0x00, 0x78];
    }

    // Escher.
    let pred = |r: &Rec| r.ty == 0xF003 && r.object() == Some(gobj);
    let (di, path) = esc
        .drawings
        .iter()
        .enumerate()
        .find_map(|(di, (_, d))| find_path(std::slice::from_ref(d), &pred).map(|p| (di, p)))
        .ok_or_else(|| WriteError::Group("group container not found".into()))?;
    let dgid = esc.drawings[di].1.find(0xF008).map_or(0, |r| r.vi >> 4);
    let path = &path[1..];
    let (last, parent_path) = path.split_last().expect("path holds the group itself");
    let drawing = &mut esc.drawings[di].1;
    let container = rec_at(drawing, parent_path)?;
    let Body::Cont(mut gk) = std::mem::replace(
        &mut container
            .kids_mut()?
            .get_mut(*last)
            .expect("path is valid")
            .body,
        Body::Cont(Vec::new()),
    ) else {
        return err("group container is not a container");
    };
    if gk.is_empty() {
        return err("empty group container");
    }
    let lead = gk.remove(0);
    let fspgr = lead
        .find(0xF009)
        .and_then(|r| match &r.body {
            Body::Atom(d) if d.len() == 16 => Some([0, 4, 8, 12].map(|o| rd(d, o).unwrap() as i32)),
            _ => None,
        })
        .ok_or_else(|| WriteError::Group("group has no coordinate box".into()))?;
    let ga = match lead.find(0xF010).map(|r| &r.body) {
        Some(Body::Atom(d)) => client_anchor(d)?,
        _ => return err("group has no client anchor"),
    };
    let (fw, fh) = (
        f64::from(fspgr[2]) - f64::from(fspgr[0]),
        f64::from(fspgr[3]) - f64::from(fspgr[1]),
    );
    if fw == 0.0 || fh == 0.0 {
        return err("group coordinate box is empty");
    }
    let (kx, ky) = (
        (f64::from(ga[2]) - f64::from(ga[0])) / fw,
        (f64::from(ga[3]) - f64::from(ga[1])) / fh,
    );
    let map = |v: i32, axis: usize, scale: f64| -> i64 {
        (f64::from(ga[axis]) + (f64::from(v) - f64::from(fspgr[axis])) * scale).round() as i64
    };
    for c in &mut gk {
        if c.ty != 0xF004 {
            return err("nested groups are not supported yet");
        }
        for r in c.kids_mut()? {
            match r.ty {
                0xF00A => {
                    let d = r.data_mut()?;
                    let f = rd(d, 4)? & !2;
                    put(d, 4, f)?;
                }
                0xF00F => {
                    let d = r.data_mut()?;
                    if d.len() != 16 {
                        return err("unexpected child anchor");
                    }
                    let v = [0, 4, 8, 12].map(|o| rd(d, o).unwrap() as i32);
                    let a = [
                        map(v[0], 0, kx),
                        map(v[1], 1, ky),
                        map(v[2], 0, kx),
                        map(v[3], 1, ky),
                    ];
                    *r = anchor_atom([
                        i32::try_from(a[0]).map_err(|_| WriteError::OutOfRange(a[0]))?,
                        i32::try_from(a[1]).map_err(|_| WriteError::OutOfRange(a[1]))?,
                        i32::try_from(a[2]).map_err(|_| WriteError::OutOfRange(a[2]))?,
                        i32::try_from(a[3]).map_err(|_| WriteError::OutOfRange(a[3]))?,
                    ]);
                }
                0xF00B if publisher_extras => add_property(r, 0x388, 1)?,
                _ => {}
            }
        }
    }
    let container = rec_at(drawing, parent_path)?;
    let k = container.kids_mut()?;
    k.splice(*last..=*last, gk);
    let drawing = &mut esc.drawings[di].1;
    drawing_counters(drawing, -1, None)?;
    if publisher_extras && drawing.find(0xF118).is_none() {
        let at = drawing
            .kids()
            .iter()
            .position(|r| r.ty == 0xF008)
            .map_or(0, |i| i + 1);
        drawing
            .kids_mut()?
            .insert(at, Rec::atom(0x0010, 0xF118, vec![1, 0, 0, 0]));
    }
    group_counters(&mut esc.head, dgid, -1, 0)?;
    install(input, &cts.finish()?, &esc.to_bytes())
}

/// Insert simple property `id` = `value` into an Escher property table (0xF00B), keeping
/// the id order; an existing property is left alone.
fn add_property(r: &mut Rec, id: u16, value: u32) -> Result<()> {
    let n = usize::from(r.vi >> 4);
    let d = r.data_mut()?;
    if d.len() < 6 * n {
        return err("short property table");
    }
    let mut at = 6 * n;
    for i in 0..n {
        let opid = u16::from_le_bytes([d[6 * i], d[6 * i + 1]]) & 0x3FFF;
        if opid == id {
            return Ok(());
        }
        if opid > id {
            at = 6 * i;
            break;
        }
    }
    let mut e = id.to_le_bytes().to_vec();
    e.extend_from_slice(&value.to_le_bytes());
    d.splice(at..at, e);
    r.vi = (r.vi & 0xF) | (((n + 1) as u16) << 4);
    Ok(())
}
