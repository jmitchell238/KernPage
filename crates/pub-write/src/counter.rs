//! The document save counter (spec 4M): item 60 of the document-header object 256.

use super::{patch, Result};
use cfb::CompoundFile;
use pub_format::{contents::Contents, items, Streams};
use std::io::Cursor;

const HEADER_OBJECT: u32 = 256;
const COUNTER_ITEM: u16 = 60;

/// Raise the document save counter by one (saturating at `u32::MAX`), in place: the item
/// is a fixed-size u32, so no stream changes length. Publisher raises it on the first
/// save of an editing session only; the caller decides when. A file without the item (or
/// without object 256) is returned unchanged; the item is never added.
pub fn bump_save_counter(input: &[u8]) -> Result<Vec<u8>> {
    let streams = Streams::from_cfb(input)?;
    let contents = Contents::parse(&streams.contents)?;
    let Some(entry) = contents.get(HEADER_OBJECT) else {
        return Ok(input.to_vec());
    };
    let Some(slot) = items::locate_int(&streams.contents, entry.offset, COUNTER_ITEM)?
        .filter(|s| s.len == 4)
    else {
        return Ok(input.to_vec());
    };
    let b = &streams.contents[slot.offset..slot.offset + 4];
    let next = u32::from_le_bytes([b[0], b[1], b[2], b[3]]).saturating_add(1);
    let mut cf = CompoundFile::open(Cursor::new(input.to_vec()))?;
    patch(&mut cf, "/Contents", &[(slot.offset, next)])?;
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}
