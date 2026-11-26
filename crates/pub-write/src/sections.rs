//! Page-number sections (spec 4T.4): the kind-117 object named by item 47 of the settings
//! object (kind 68). Its item 1 is the record count and item 2 the list of records; item 1
//! of each record is the 1-based start page. Only the start pages and the number of
//! records are written here; every other byte of a record is kept.

use super::group::{int_at, items_of, put, rd, Cts};
use super::{Result, WriteError};
use pub_format::Streams;

const KIND_SETTINGS: u64 = 68;
const KIND_SECTIONS: u64 = 117;
const ITEM_SECTIONS: u16 = 47;

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Pages(m.into()))
}

/// Keep the section records `keep[k].0` (0-based, in the file's order) with the new start
/// page `keep[k].1`; every other record is dropped and the record count follows. The
/// caller passes the records in ascending order with the first start page 1.
pub fn set_sections(input: &[u8], keep: &[(usize, u32)]) -> Result<Vec<u8>> {
    if keep.is_empty() {
        return fail("a document keeps at least one section");
    }
    let streams = Streams::from_cfb(input)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let Some(settings) = cts.slot_of_kind(KIND_SETTINGS) else {
        return fail("the file has no settings object");
    };
    let soff = cts.offset(settings)?;
    let target = items_of(&cts.blocks, soff)?
        .into_iter()
        .find(|i| i.0 == ITEM_SECTIONS && i.3 - i.2 == 4)
        .map(|i| int_at(&cts.blocks, i.2, 4));
    let Some(slot) = target.filter(|&t| cts.kind(t) == Some(KIND_SECTIONS)) else {
        return fail("the file has no section object");
    };
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let find = |id| items.iter().find(|i| i.0 == id).copied();
    let (Some((_, _, cp, ce)), Some((_, _, lp, le))) = (find(1), find(2)) else {
        return fail("the section object lacks its count or list");
    };
    // The records of the list: (start, end) of each, header included.
    let mut recs: Vec<(usize, usize)> = Vec::new();
    let mut pos = lp + 4;
    while pos < le {
        let len = rd(&cts.blocks, pos + 2)? as usize;
        if len < 4 || pos + 2 + len > le {
            return fail("malformed section record");
        }
        recs.push((pos, pos + 2 + len));
        pos += 2 + len;
    }
    // New list.
    let mut list = vec![0u8; 4];
    for &(old, start) in keep {
        let Some(&(rs, re)) = recs.get(old) else {
            return fail("no such section record");
        };
        let mut rec = cts.blocks[rs..re].to_vec();
        let (_, _, p, e) = items_of(&rec, 2)?
            .into_iter()
            .find(|i| i.0 == 1)
            .ok_or_else(|| WriteError::Pages("a section record has no start page".into()))?;
        let width = e - p;
        if !matches!(width, 2 | 4 | 8) || (width < 8 && u64::from(start) >> (8 * width) != 0) {
            return fail("the section start page does not fit its item");
        }
        for k in 0..width {
            rec[p + k] = (start >> (8 * k)) as u8;
        }
        list.extend_from_slice(&rec);
    }
    let n = list.len() as u32;
    list[..4].copy_from_slice(&n.to_le_bytes());
    // The count item first (it sits before the list and keeps its size), then the list.
    let width = ce - cp;
    if !matches!(width, 2 | 4) {
        return fail("unexpected section count item");
    }
    let count = keep.len() as u32;
    for k in 0..width {
        cts.blocks[cp + k] = (count >> (8 * k)) as u8;
    }
    let delta = list.len() as isize - (le - lp) as isize;
    cts.splice(lp, le - lp, &list)?;
    let len = (rd(&cts.blocks, off)? as isize + delta) as u32;
    put(&mut cts.blocks, off, len)?;
    let contents = cts.finish()?;
    super::group::install(input, &contents, &streams.escher)
}
