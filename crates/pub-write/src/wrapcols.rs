//! Change the text wrap type of an object (spec 4K) or the columns of a text box (spec 4I)
//! in place; the streams are rewritten because items and properties come and go.

use super::group::{find_path, install, items_of, put, rd, rec_at, Cts, EscherDoc, Rec};
use super::style::{remove_prop, set_prop};
use super::{mcld_item_slots, Result, WriteError};
use cfb::CompoundFile;
use pub_format::{Streams, WrapType};
use std::io::Cursor;

/// `MCLD` frame items: column gap and column count - 1 (they carry the 0x200 id bit).
const FRAME_GAP: u16 = 0x20A;
const FRAME_COLUMNS: u16 = 0x20B;
const MIRROR_BASE_EMU: u64 = 9_524;
const PROP_COLUMNS: u16 = 0x008C;
const PROP_SPACING: u16 = 0x008D;
const ITEM_WRAP: u16 = 4;
const ITEM_WRAP2: u16 = 5;
const MAX_COLUMNS: u32 = 64;

fn bad<T>(m: &str) -> Result<T> {
    Err(WriteError::Shape(m.into()))
}

/// A two-byte `Contents` item (class 2) with a small value.
fn small_item(id: u16, v: u16) -> Vec<u8> {
    let mut b = pub_format::items::header(id, 2).to_vec();
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// Replace (`new` = Some, in place), insert (kept in id order) or remove (`new` = None)
/// top-level item `id` of the object block at `off`, fixing the block length and the
/// offsets of later objects.
fn edit_item(cts: &mut Cts, off: usize, id: u16, new: Option<Vec<u8>>) -> Result<()> {
    let items = items_of(&cts.blocks, off)?;
    let old = items.iter().find(|i| i.0 == id).copied();
    let (at, remove) = match old {
        Some((_, h, _, e)) => (h, e - h),
        None => {
            let Some(at) = items.iter().find(|i| i.0 > id).map(|i| i.1) else {
                return bad("the object has no item after the wrap items");
            };
            (at, 0)
        }
    };
    let ins = new.unwrap_or_default();
    if remove == 0 && ins.is_empty() {
        return Ok(());
    }
    cts.splice(at, remove, &ins)?;
    let len = rd(&cts.blocks, off)? as isize + ins.len() as isize - remove as isize;
    put(&mut cts.blocks, off, len as u32)
}

/// Set the wrap type of shape `shape_index` on page `page_index` (spec 4K): 0/2/3 = item 4
/// (inserted when absent); top and bottom = no item 4; through = no item 4 and item 5 = 4.
/// The upper byte of item 4 is written as 0.
pub fn set_shape_wrap(
    input: &[u8],
    page_index: usize,
    shape_index: usize,
    wrap: WrapType,
) -> Result<Vec<u8>> {
    if let WrapType::Other(_) = wrap {
        return bad("unsupported wrap type");
    }
    let doc = pub_format::read_bytes(input)?;
    let shape = doc
        .pages
        .get(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?
        .shapes
        .get(shape_index)
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?;
    let streams = Streams::from_cfb(input)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let slot = shape.object_number as usize;
    let off = cts.offset(slot)?;
    let item5 = |c: &Cts| {
        items_of(&c.blocks, off).map(|v| {
            v.iter()
                .find(|i| i.0 == ITEM_WRAP2)
                .map(|i| c.blocks[i.2])
        })
    };
    match wrap {
        WrapType::None | WrapType::Square | WrapType::Tight => {
            edit_item(
                &mut cts,
                off,
                ITEM_WRAP,
                Some(small_item(ITEM_WRAP, u16::from(wrap.to_byte()))),
            )?;
            if item5(&cts)? == Some(4) {
                edit_item(&mut cts, off, ITEM_WRAP2, None)?;
            }
        }
        WrapType::TopAndBottom => {
            edit_item(&mut cts, off, ITEM_WRAP, None)?;
            if item5(&cts)? == Some(4) {
                edit_item(&mut cts, off, ITEM_WRAP2, None)?;
            }
        }
        WrapType::Through => {
            edit_item(&mut cts, off, ITEM_WRAP, None)?;
            edit_item(&mut cts, off, ITEM_WRAP2, Some(small_item(ITEM_WRAP2, 4)))?;
        }
        WrapType::Other(_) => unreachable!(),
    }
    install(input, &cts.finish()?, &streams.escher)
}

/// Set the column count and gap (EMU) of text box `shape_index` (spec 4I): Escher property
/// 0x8C (count, removed for one column) and 0x8D (2 x gap) of the shape's tertiary record,
/// plus the story's `MCLD` mirror items 0x20B (count - 1) and 0x20A (2 x gap + 9,524)
/// when the box maps to a frame.
pub fn set_text_columns(
    input: &[u8],
    page_index: usize,
    shape_index: usize,
    columns: u32,
    gap_emu: u32,
) -> Result<Vec<u8>> {
    if columns == 0 || columns > MAX_COLUMNS {
        return bad("column count out of range");
    }
    let doc = pub_format::read_bytes(input)?;
    let obj = doc
        .pages
        .get(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?
        .shapes
        .get(shape_index)
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?
        .object_number;
    let gap2 = gap_emu
        .checked_mul(2)
        .ok_or(WriteError::OutOfRange(i64::from(gap_emu) * 2))?;
    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let pred = |r: &Rec| r.ty == 0xF004 && r.object() == Some(obj);
    let (di, path) = esc
        .drawings
        .iter()
        .enumerate()
        .find_map(|(di, (_, d))| find_path(std::slice::from_ref(d), &pred).map(|p| (di, p)))
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?;
    let sp = rec_at(&mut esc.drawings[di].1, &path[1..])?;
    let tertiary = sp
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF122)
        .ok_or_else(|| WriteError::Shape("the text box has no column record".into()))?;
    if columns == 1 {
        remove_prop(tertiary, PROP_COLUMNS)?;
    } else {
        set_prop(tertiary, PROP_COLUMNS, columns)?;
    }
    set_prop(tertiary, PROP_SPACING, gap2)?;
    let mut out = install(input, &streams.contents, &esc.to_bytes())?;

    // The MCLD mirror (Publisher ignores it on load but rewrites it).
    if let Ok(Some([(gap_at, gap_len), (col_at, col_len)])) = mcld_item_slots(
        &doc,
        &streams,
        page_index,
        shape_index,
        [FRAME_GAP, FRAME_COLUMNS],
    ) {
        if gap_len == 4 && col_len == 2 {
            let mirror = u32::try_from(u64::from(gap2) + MIRROR_BASE_EMU)
                .map_err(|_| WriteError::OutOfRange(i64::from(gap2)))?;
            let mut cf = CompoundFile::open(Cursor::new(out))?;
            {
                use std::io::{Seek, SeekFrom, Write};
                let mut s = cf.open_stream("/Quill/QuillSub/CONTENTS")?;
                s.seek(SeekFrom::Start(gap_at as u64))?;
                s.write_all(&mirror.to_le_bytes())?;
                s.seek(SeekFrom::Start(col_at as u64))?;
                s.write_all(&((columns - 1) as u16).to_le_bytes())?;
            }
            cf.flush()?;
            out = cf.into_inner().into_inner();
        }
    }
    Ok(out)
}
