//! Master pages (spec 4T.8 to 4T.12): add, rename, delete, apply and ignore, and the margins
//! of one master. Shapes on a master are written by the ordinary shape writers, addressed with
//! a [`pub_format::master_surface`] index.
//!
//! Masters are numbered as the reader lists them (`pub_format::Document::masters`, the order of
//! their object numbers). Only single-page masters (item 16 = 3) are written; a file with a
//! two-page master is refused for every change that would have to know which half it touches.

use super::group::{install, items_of, rd, Cts};
use super::pages::{remove_object_tree, strip_escher};
use super::pagesetup::{
    edit_object, enc_block, flag, guide_list, guide_positions, int, put_item, utf16z,
};
use super::{Result, WriteError};
use pub_format::items::{decode_block, Item, Value};
use pub_format::Streams;

const KIND_PAGE: u64 = 67;
/// Item 45 of the document object: the number of master entries.
const ITEM_COUNT: u16 = 45;
/// Items 8 to 11 of the guides object of a master Publisher adds (spec 4T.8).
const NEW_GUIDE_GUTTER: i64 = 196_469;

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Master(m.into()))
}

struct Env {
    streams: Streams,
    cts: Cts,
    /// The master objects, in reader order.
    masters: Vec<u32>,
    /// The document pages' objects.
    pages: Vec<u32>,
    /// The document object (256), which lists masters and pages.
    root: usize,
}

fn load(input: &[u8]) -> Result<Env> {
    let doc = pub_format::read_bytes(input)?;
    let streams = Streams::from_cfb(input)?;
    let cts = Cts::parse(&streams.contents)?;
    let masters: Vec<u32> = doc.masters.iter().map(|m| m.object_number).collect();
    let pages: Vec<u32> = doc.pages.iter().map(|p| p.object_number).collect();
    let Some(&first) = masters.first().or(pages.first()) else {
        return fail("the file has no master page");
    };
    let root = cts.parent(first as usize)?;
    Ok(Env {
        streams,
        cts,
        masters,
        pages,
        root,
    })
}

fn master_obj(env: &Env, index: usize) -> Result<usize> {
    env.masters
        .get(index)
        .map(|&o| o as usize)
        .ok_or_else(|| WriteError::Master(format!("master page {} does not exist", index + 1)))
}

fn items(cts: &Cts, slot: usize) -> Result<Vec<Item>> {
    let off = cts.offset(slot)?;
    let len = rd(&cts.blocks, off)? as usize;
    let items = decode_block(&cts.blocks, off)?;
    if enc_block(&items)? != cts.blocks[off..off + len] {
        return fail(format!("object {slot} cannot be re-encoded exactly"));
    }
    Ok(items)
}

fn int_of(items: &[Item], id: u16) -> Option<u64> {
    items.iter().find(|i| i.id == id).and_then(Item::as_int)
}

/// A half of a two-page master is not written (spec 4T.10).
fn ensure_single(cts: &Cts, slot: usize) -> Result<()> {
    match int_of(&items(cts, slot)?, 16) {
        None | Some(3) => Ok(()),
        Some(_) => fail("two-page master pages are not supported"),
    }
}

fn string_item(id: u16, s: &str) -> Item {
    Item {
        id,
        class: 24,
        value: Value::Raw(utf16z(s)),
    }
}

fn ref_item(id: u16, v: usize) -> Item {
    Item {
        id,
        class: 14,
        value: Value::Int(v as u64),
    }
}

fn add_to_count(cts: &mut Cts, root: usize, delta: i64) -> Result<()> {
    edit_object(cts, root, |items| {
        if let Some(Value::Int(v)) = items
            .iter_mut()
            .find(|i| i.id == ITEM_COUNT)
            .map(|i| &mut i.value)
        {
            *v = (*v as i64 + delta).max(0) as u64;
        }
        Ok(())
    })
}

fn block_bytes(cts: &Cts, slot: usize) -> Result<Vec<u8>> {
    let off = cts.offset(slot)?;
    let len = rd(&cts.blocks, off)? as usize;
    cts.blocks
        .get(off..off + len)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| WriteError::Master("object block past the stream".into()))
}

/// The guide objects (kind 76, item 3) of every master page, for the page-setup writers.
/// Refuses a file with a two-page master.
pub(crate) fn guide_slots(cts: &Cts) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for slot in 0..cts.entries.len() {
        if cts.kind(slot) != Some(KIND_PAGE) {
            continue;
        }
        let Ok(it) = items(cts, slot) else { continue };
        // A master has item 3 (its guides); a page or helper page has not.
        let Some(g) = int_of(&it, 3) else { continue };
        if !matches!(int_of(&it, 16), None | Some(3)) {
            return Err(WriteError::PageSetup(
                "two-page (facing) master pages are not supported".into(),
            ));
        }
        out.push(g as usize);
    }
    if out.is_empty() {
        return Err(WriteError::PageSetup(
            "the file has no layout guides object".into(),
        ));
    }
    Ok(out)
}

/// Add a single-page master after the last one (spec 4T.8). Its body, helper objects and
/// guides are copies of master `template`'s (the guides keep that master's margins); `letter`
/// is item 14 and `name` item 15. The new master is the last of
/// [`pub_format::Document::masters`].
pub fn add_master(input: &[u8], template: usize, letter: char, name: &str) -> Result<Vec<u8>> {
    let mut env = load(input)?;
    let t = master_obj(&env, template)?;
    ensure_single(&env.cts, t)?;
    let cts = &mut env.cts;
    let tmpl = items(cts, t)?;
    let helper = |id: u16| {
        int_of(&tmpl, id)
            .map(|v| v as usize)
            .ok_or_else(|| WriteError::Master(format!("the template master has no item {id}")))
    };
    let (guides, k96, k119) = (helper(3)?, helper(9)?, helper(11)?);
    let (id, at) = cts.tail()?;
    let mut body: Vec<Item> = tmpl
        .iter()
        .filter(|i| !matches!(i.id, 1 | 2 | 4))
        .cloned()
        .collect();
    put_item(&mut body, ref_item(3, id + 3));
    put_item(&mut body, ref_item(9, id + 1));
    put_item(&mut body, ref_item(11, id + 2));
    put_item(&mut body, string_item(14, &letter.to_string()));
    put_item(&mut body, string_item(15, name));
    // The guides of a master Publisher adds carry its default gutter in items 8 to 11.
    let mut guide_items = items(cts, guides)?;
    for id in 8..=11 {
        put_item(&mut guide_items, int(id, NEW_GUIDE_GUTTER)?);
    }
    let blocks = [
        enc_block(&body)?,
        block_bytes(cts, k96)?,
        block_bytes(cts, k119)?,
        enc_block(&guide_items)?,
    ];
    let entries = vec![
        cts.entries[t].clone(),
        cts.entries[k96].clone(),
        cts.entries[k119].clone(),
        cts.entries[guides].clone(),
    ];
    let ins: Vec<u8> = blocks.iter().flatten().copied().collect();
    cts.blocks.splice(at..at, ins.iter().copied());
    cts.shift(at, true, ins.len() as isize)?;
    cts.insert_entries(id, entries)?;
    let mut off = at;
    for (k, b) in blocks.iter().enumerate() {
        cts.set_int(id + k, 4, off)?;
        if k > 0 {
            cts.set_int(id + k, 5, id)?;
        }
        off += b.len();
    }
    // Object 256: the master goes after the last master, the counts follow.
    let mut ids = cts.child_ids(env.root)?;
    let pos = ids
        .iter()
        .rposition(|x| env.masters.contains(x))
        .map_or(0, |p| p + 1);
    ids.insert(pos, id as u32);
    cts.set_child_ids(env.root, &ids)?;
    add_to_count(cts, env.root, 1)?;
    let contents = env.cts.finish()?;
    install(input, &contents, &env.streams.escher)
}

/// Rename master `index`: its one-character id (item 14) and its name (item 15).
pub fn rename_master(input: &[u8], index: usize, letter: char, name: &str) -> Result<Vec<u8>> {
    let mut env = load(input)?;
    let m = master_obj(&env, index)?;
    edit_object(&mut env.cts, m, |items| {
        put_item(items, string_item(14, &letter.to_string()));
        put_item(items, string_item(15, name));
        Ok(())
    })?;
    let contents = env.cts.finish()?;
    install(input, &contents, &env.streams.escher)
}

/// Delete master `index` with its helper objects and shapes (spec 4T.9). Pages and helper
/// pages that used it move to the first remaining master. The last master cannot be deleted.
pub fn delete_master(input: &[u8], index: usize) -> Result<Vec<u8>> {
    let mut env = load(input)?;
    let m = master_obj(&env, index)?;
    if env.masters.len() < 2 {
        return fail("you cannot delete the default master page");
    }
    ensure_single(&env.cts, m)?;
    let replacement = env.masters[usize::from(index == 0)] as usize;
    let cts = &mut env.cts;
    for slot in 0..cts.entries.len() {
        if cts.kind(slot) != Some(KIND_PAGE) || slot == m {
            continue;
        }
        let off = cts.offset(slot)?;
        if let Some(&(_, _, p, e)) = items_of(&cts.blocks, off)?.iter().find(|i| i.0 == 13) {
            if e - p == 4 && rd(&cts.blocks, p)? as usize == m {
                cts.blocks[p..e].copy_from_slice(&(replacement as u32).to_le_bytes());
            }
        }
    }
    let gone = remove_object_tree(cts, env.root, m as u32)?;
    add_to_count(cts, env.root, -1)?;
    let escher = strip_escher(&env.streams.escher, &gone)?;
    let contents = env.cts.finish()?;
    install(input, &contents, &escher)
}

/// What a page shows of its masters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMaster {
    /// The document page (0-based).
    pub page: usize,
    /// The master the page uses (item 13); `None` leaves it as it is.
    pub master: Option<usize>,
    /// "Ignore Master Page" (item 7).
    pub ignore: bool,
}

/// Set the master and the ignore flag of pages (spec 4T.9, 4T.2).
pub fn set_page_masters(input: &[u8], changes: &[PageMaster]) -> Result<Vec<u8>> {
    let mut env = load(input)?;
    for c in changes {
        let page = *env.pages.get(c.page).ok_or(WriteError::NoSuchPage(c.page))? as usize;
        let target = c.master.map(|m| master_obj(&env, m)).transpose()?;
        edit_object(&mut env.cts, page, |items| {
            if let Some(m) = target {
                put_item(
                    items,
                    Item {
                        id: 13,
                        class: 13,
                        value: Value::Int(m as u64),
                    },
                );
            }
            if c.ignore {
                put_item(items, flag(7));
            } else {
                items.retain(|i| i.id != 7);
            }
            Ok(())
        })?;
    }
    let contents = env.cts.finish()?;
    install(input, &contents, &env.streams.escher)
}

/// Set the margins (left, right, top, bottom, EMU) of master `index` alone (spec 4T.12).
pub fn set_master_margins(input: &[u8], index: usize, margins: [u32; 4]) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let (w, h) = (
        i64::from(doc.page_width_emu),
        i64::from(doc.page_height_emu),
    );
    let mut env = load(input)?;
    let m = master_obj(&env, index)?;
    ensure_single(&env.cts, m)?;
    let guides = int_of(&items(&env.cts, m)?, 3)
        .ok_or_else(|| WriteError::Master("the master has no guides object".into()))?
        as usize;
    let [l, r, t, b] = margins.map(i64::from);
    if l + r >= w || t + b >= h {
        return fail("the margins leave no room on the page");
    }
    edit_object(&mut env.cts, guides, |items| {
        guide_positions(items)?;
        put_item(items, guide_list(l, w - r, t, h - b)?);
        Ok(())
    })?;
    let contents = env.cts.finish()?;
    install(input, &contents, &env.streams.escher)
}
