//! Add, move and delete document pages (spec 4P). A blank page is three new `Contents`
//! objects (page, kind 96, kind 119) plus an entry in object 256's page list; moving only
//! reorders that list; deleting removes the page's whole `Contents` subtree, its registry
//! records and its Escher containers. Escher and Quill are untouched by add and move.

use super::group::{
    drawing_counters, group_counters, install, items_of, put, rd, Body, Cts, EscherDoc,
    Rec,
};
use super::{Result, WriteError};
use pub_format::Streams;
use std::collections::HashSet;

const KIND_PAGE: u64 = 67;
const KIND_PAGE_96: u64 = 96;
const KIND_PAGE_119: u64 = 119;
const KIND_STORY_MAP: u64 = 97;
const KIND_STORY_INFO: u64 = 101;
const KIND_STORY_ROOT: u64 = 91;
/// Object 256 item 35: page serial counter (spec 4P.1).
const ITEM_SERIAL: u16 = 35;
pub(crate) const EMPTY_SLOT: [u8; 2] = [0x00, 0x78];

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Pages(m.into()))
}

struct Ctx {
    cts: Cts,
    page_objs: Vec<u32>,
    root: usize,
}

fn load(input: &[u8]) -> Result<(Streams, Ctx)> {
    let doc = pub_format::read_bytes(input)?;
    let streams = Streams::from_cfb(input)?;
    let cts = Cts::parse(&streams.contents)?;
    let page_objs: Vec<u32> = doc.pages.iter().map(|p| p.object_number).collect();
    let Some(&first) = page_objs.first() else {
        return fail("the document has no pages");
    };
    let root = cts.parent(first as usize)?;
    Ok((
        streams,
        Ctx {
            cts,
            page_objs,
            root,
        },
    ))
}

/// Write the document pages' ids into the positions the document pages already occupy in
/// the root list (master and helper pages keep their slots).
fn write_order(cx: &mut Ctx, new_order: &[u32]) -> Result<()> {
    let mut ids = cx.cts.child_ids(cx.root)?;
    let mut k = 0;
    for id in ids.iter_mut() {
        if cx.page_objs.contains(id) {
            *id = new_order[k];
            k += 1;
        }
    }
    cx.cts.set_child_ids(cx.root, &ids)
}

/// Index entry `{2: kind, 4: offset, 5: parent, 6: 258, 11: 1}` (the form of GROUP_ENTRY).
fn entry(kind: u16, parent: u32) -> Vec<u8> {
    let mut items = vec![0x02, 0x18];
    items.extend_from_slice(&kind.to_le_bytes());
    items.extend_from_slice(&[0x04, 0xb8, 0, 0, 0, 0, 0x05, 0x68]);
    items.extend_from_slice(&parent.to_le_bytes());
    items.extend_from_slice(&[0x06, 0x10, 0x02, 0x01, 0x0b, 0x18, 0x01, 0x00]);
    let mut e = vec![0x00, 0x88];
    e.extend_from_slice(&((4 + items.len()) as u32).to_le_bytes());
    e.extend_from_slice(&items);
    e
}

/// Add `delta` to the integer item `id` of the block of `slot`; returns the old value.
fn bump_item(cts: &mut Cts, slot: usize, id: u16, delta: i64) -> Result<u64> {
    let off = cts.offset(slot)?;
    let it = items_of(&cts.blocks, off)?;
    let &(_, _, p, e) = it
        .iter()
        .find(|i| i.0 == id)
        .ok_or_else(|| WriteError::Pages(format!("object {slot} lacks item {id}")))?;
    let n = e - p;
    if n != 4 && n != 8 {
        return fail(format!("item {id} of object {slot} is not an integer"));
    }
    let old = super::group::int_at(&cts.blocks, p, n) as u64;
    let new = (old as i64 + delta).max(0) as u64;
    cts.blocks[p..e].copy_from_slice(&new.to_le_bytes()[..n]);
    Ok(old)
}

/// Add a blank page after document page `index_after` (0-based); `None` puts it first.
/// The page copies its size from an existing page and uses master page A like it.
pub fn add_page(input: &[u8], index_after: Option<usize>) -> Result<Vec<u8>> {
    let (streams, mut cx) = load(input)?;
    if let Some(i) = index_after {
        if i >= cx.page_objs.len() {
            return Err(WriteError::NoSuchPage(i));
        }
    }
    let cts = &mut cx.cts;
    let font = cts.tail()?.0;
    // Template: the last document page; page-only items (1, 2, 8) are dropped.
    let tmpl = *cx.page_objs.last().expect("checked") as usize;
    let toff = cts.offset(tmpl)?;
    let tlen = rd(&cts.blocks, toff)? as usize;
    let new_id = font as u32;
    let serial = {
        let root = cx.root;
        bump_item(cts, root, ITEM_SERIAL, 1)?
    };
    let mut body = vec![0u8; 4];
    let mut have_6 = false;
    for (id, h, p, e) in items_of(&cts.blocks, toff)? {
        match id {
            1 | 2 | 8 => {}
            6 => {
                if e - p != 8 {
                    return fail("page item 6 is not 8 bytes");
                }
                body.extend_from_slice(&cts.blocks[h..p]);
                body.extend_from_slice(&((serial << 32) | 2).to_le_bytes());
                have_6 = true;
            }
            9 => {
                body.extend_from_slice(&cts.blocks[h..p]);
                body.extend_from_slice(&(new_id + 1).to_le_bytes());
            }
            11 => {
                body.extend_from_slice(&cts.blocks[h..p]);
                body.extend_from_slice(&(new_id + 2).to_le_bytes());
            }
            _ => body.extend_from_slice(&cts.blocks[h..e]),
        }
    }
    if !have_6 || toff + tlen > cts.blocks.len() {
        return fail("the template page has no item 6");
    }
    let n = body.len() as u32;
    body[..4].copy_from_slice(&n.to_le_bytes());
    let mut b96 = vec![0x0a, 0, 0, 0, 0x05, 0x20, 1, 0, 0, 0];
    // Publisher 2021 refuses a bare 20-byte kind 119 (tested, spec 4P.2): copy the
    // template page's 296-byte one (the web-form strings), as Publisher itself does.
    let t119 = (0..cts.entries.len())
        .find(|&i| cts.kind(i) == Some(KIND_PAGE_119) && cts.parent(i).ok() == Some(tmpl))
        .ok_or_else(|| WriteError::Pages("the template page has no kind 119 child".into()))?;
    let o119 = cts.offset(t119)?;
    let mut b119 = cts.blocks[o119..o119 + rd(&cts.blocks, o119)? as usize].to_vec();
    let l96 = b96.len();
    let (_, at) = cts.tail()?;
    let mut ins = body.clone();
    ins.append(&mut b96);
    ins.append(&mut b119);
    cts.blocks.splice(at..at, ins.iter().copied());
    cts.shift(at, true, ins.len() as isize)?;
    let root = cx.root as u32;
    cts.insert_entries(
        font,
        vec![
            entry(KIND_PAGE as u16, root),
            entry(KIND_PAGE_96 as u16, new_id),
            entry(KIND_PAGE_119 as u16, new_id),
        ],
    )?;
    cts.set_int(font, 4, at)?;
    cts.set_int(font + 1, 4, at + body.len())?;
    cts.set_int(font + 2, 4, at + body.len() + l96)?;
    // The page entry of a real page also has item 8 (flag); copy it from the template.
    cts.entries[font] = {
        let mut e = cts.entries[tmpl].clone();
        let off = at as u32;
        let s = pub_format::items::locate_int(&e, 2, 4)
            .ok()
            .flatten()
            .ok_or_else(|| WriteError::Pages("template index entry has no offset".into()))?;
        e[s.offset..s.offset + 4].copy_from_slice(&off.to_le_bytes());
        e
    };

    let mut ids = cts.child_ids(cx.root)?;
    let pos = match index_after {
        Some(i) => ids.iter().position(|&x| x == cx.page_objs[i]),
        None => ids.iter().position(|&x| x == cx.page_objs[0]),
    }
    .ok_or_else(|| WriteError::Pages("page not found in the page list".into()))?
        + usize::from(index_after.is_some());
    ids.insert(pos, new_id);
    cts.set_child_ids(cx.root, &ids)?;

    let contents = cx.cts.finish()?;
    install(input, &contents, &streams.escher)
}

/// Move document page `from` so that it becomes page `to` (both 0-based).
pub fn move_page(input: &[u8], from: usize, to: usize) -> Result<Vec<u8>> {
    let (streams, mut cx) = load(input)?;
    let n = cx.page_objs.len();
    for i in [from, to] {
        if i >= n {
            return Err(WriteError::NoSuchPage(i));
        }
    }
    if from == to {
        return Ok(input.to_vec());
    }
    let mut order = cx.page_objs.clone();
    let id = order.remove(from);
    order.insert(to, id);
    write_order(&mut cx, &order)?;
    let contents = cx.cts.finish()?;
    install(input, &contents, &streams.escher)
}

/// Remove the records of a registry object (kind 97/101) for which `drop` is true. The
/// remaining records are not renumbered (spec 4P.4). Returns the removed records' blocks.
fn drop_records(
    cts: &mut Cts,
    slot: usize,
    drop: &dyn Fn(&[u8], usize) -> Result<bool>,
) -> Result<()> {
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let find = |id: u16| items.iter().find(|i| i.0 == id).copied();
    // A registry that never held a record has no items at all (spec 6.11).
    let (Some((_, _, cp, _)), Some((_, _, lp, le))) = (find(1), find(2)) else {
        return Ok(());
    };
    let mut ranges = Vec::new();
    let mut pos = lp + 4;
    while pos < le {
        let len = rd(&cts.blocks, pos + 2)? as usize;
        if cts.blocks[pos + 1] >> 3 != 17 || pos + 2 + len > le {
            return fail("unexpected registry record");
        }
        if drop(&cts.blocks, pos + 2)? {
            ranges.push((pos, 2 + len));
        }
        pos += 2 + len;
    }
    let removed: usize = ranges.iter().map(|r| r.1).sum();
    for &(at, len) in ranges.iter().rev() {
        cts.splice(at, len, &[])?;
    }
    let count = rd(&cts.blocks, cp)? as usize - ranges.len();
    put(&mut cts.blocks, cp, count as u32)?;
    let ll = rd(&cts.blocks, lp)? as usize - removed;
    put(&mut cts.blocks, lp, ll as u32)?;
    let bl = rd(&cts.blocks, off)? as usize - removed;
    put(&mut cts.blocks, off, bl as u32)
}

fn rec_int(blocks: &[u8], rec_block: usize, id: u16) -> Result<Option<u32>> {
    Ok(items_of(blocks, rec_block)?
        .into_iter()
        .find(|i| i.0 == id && i.3 - i.2 == 4)
        .map(|(_, _, p, _)| rd(blocks, p).unwrap_or(0)))
}

fn count_shapes(r: &Rec) -> u32 {
    u32::from(r.ty == 0xF004) + r.kids().iter().map(count_shapes).sum::<u32>()
}

pub(crate) fn picture_indexes(r: &Rec, out: &mut Vec<u32>) {
    if r.ty == 0xF00B {
        if let Body::Atom(d) = &r.body {
            for i in 0..usize::from(r.vi >> 4) {
                let at = 6 * i;
                if d.len() >= at + 6
                    && u16::from_le_bytes([d[at], d[at + 1]]) & 0x3FFF == 0x0104
                {
                    out.push(u32::from_le_bytes([d[at + 2], d[at + 3], d[at + 4], d[at + 5]]));
                }
            }
        }
    }
    r.kids().iter().for_each(|k| picture_indexes(k, out));
}

/// Remove every container whose Contents object is in `set`; returns the shapes removed.
fn strip(r: &mut Rec, set: &HashSet<u32>, pibs: &mut Vec<u32>) -> u32 {
    let Body::Cont(kids) = &mut r.body else {
        return 0;
    };
    let mut removed = 0;
    let mut i = 0;
    while i < kids.len() {
        let k = &kids[i];
        if matches!(k.ty, 0xF004 | 0xF003) && k.object().is_some_and(|o| set.contains(&o)) {
            removed += count_shapes(k);
            picture_indexes(k, pibs);
            kids.remove(i);
        } else {
            removed += strip(&mut kids[i], set, pibs);
            i += 1;
        }
    }
    removed
}

/// Lower the `cRef` (BSE payload offset 24) of the 1-based store entries in `pibs`.
fn lower_crefs(r: &mut Rec, seen: &mut u32, pibs: &[u32]) {
    adjust_crefs(r, seen, pibs, -1)
}

/// Add `sign` x (number of references in `pibs`) to the `cRef` of each store entry.
pub(crate) fn adjust_crefs(r: &mut Rec, seen: &mut u32, pibs: &[u32], sign: i64) {
    match &mut r.body {
        Body::Atom(d) => {
            if r.ty == 0xF007 && d.len() >= 36 {
                *seen += 1;
                let hits = pibs.iter().filter(|&&p| p == *seen).count() as u32;
                if hits > 0 {
                    let c = (i64::from(rd(d, 24).unwrap_or(0)) + sign * i64::from(hits)).max(0) as u32;
                    let _ = put(d, 24, c);
                }
            }
        }
        Body::Cont(k) => k.iter_mut().for_each(|c| adjust_crefs(c, seen, pibs, sign)),
    }
}

/// Delete document page `index` and everything on it. Refuses to delete the only page.
/// Quill stories of the page's text boxes stay in the file as orphans (spec 4P.4: accepted
/// by Publisher 2021); a picture's blip store entry stays with its `cRef` lowered.
pub fn delete_page(input: &[u8], index: usize) -> Result<Vec<u8>> {
    let (streams, mut cx) = load(input)?;
    if index >= cx.page_objs.len() {
        return Err(WriteError::NoSuchPage(index));
    }
    if cx.page_objs.len() == 1 {
        return fail("the only page of a document cannot be deleted");
    }
    let page = cx.page_objs[index];
    let cts = &mut cx.cts;
    // The page and every descendant by index parent.
    let mut gone: HashSet<u32> = HashSet::from([page]);
    loop {
        let before = gone.len();
        for i in 0..cts.entries.len() {
            if cts.kind(i).is_some() {
                if let Ok(p) = cts.parent(i) {
                    if gone.contains(&(p as u32)) {
                        gone.insert(i as u32);
                    }
                }
            }
        }
        if gone.len() == before {
            break;
        }
    }
    let reg_parent = cts
        .slot_of_kind(KIND_STORY_ROOT)
        .ok_or_else(|| WriteError::Pages("the story root object (kind 91) is missing".into()))?;
    let slot_of = |cts: &Cts, kind: u64| {
        (0..cts.entries.len())
            .find(|&i| cts.kind(i) == Some(kind) && cts.parent(i).ok() == Some(reg_parent))
    };
    let (Some(map), Some(info)) = (
        slot_of(cts, KIND_STORY_MAP),
        slot_of(cts, KIND_STORY_INFO),
    ) else {
        return fail("the story registries (kinds 97 and 101) are missing");
    };
    // Registries: 283 records for deleted objects (item 3), 284 records of their stories.
    let mut stories: HashSet<u32> = HashSet::new();
    {
        let off = cts.offset(map)?;
        let items = items_of(&cts.blocks, off)?;
        let (lp, le) = items.iter().find(|i| i.0 == 2).map_or((0, 0), |i| (i.2, i.3));
        let mut pos = lp + 4;
        while pos < le {
            let len = rd(&cts.blocks, pos + 2)? as usize;
            if rec_int(&cts.blocks, pos + 2, 3)?.is_some_and(|o| gone.contains(&o)) {
                if let Some(s) = rec_int(&cts.blocks, pos + 2, 1)? {
                    stories.insert(s);
                }
            }
            pos += 2 + len;
        }
    }
    drop_records(cts, map, &|b, at| {
        Ok(rec_int(b, at, 3)?.is_some_and(|o| gone.contains(&o)))
    })?;
    drop_records(cts, info, &|b, at| {
        Ok(rec_int(b, at, 1)?.is_some_and(|s| stories.contains(&s)))
    })?;
    // Page list of object 256: item 2 (and its count item 1); item 35 stays.
    let mut ids = cts.child_ids(cx.root)?;
    ids.retain(|&x| x != page);
    cts.set_child_ids(cx.root, &ids)?;
    // Remove the objects' blocks, highest offset first, and empty their slots.
    let mut blocks = Vec::new();
    for &g in &gone {
        let off = cts.offset(g as usize)?;
        let len = rd(&cts.blocks, off)? as usize;
        if off + len > cts.blocks.len() {
            return fail("object block past the stream");
        }
        blocks.push((off, len));
    }
    for &g in &gone {
        cts.entries[g as usize] = EMPTY_SLOT.to_vec();
    }
    blocks.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    for (off, len) in blocks {
        cts.splice(off, len, &[])?;
    }
    // Close up empty slots directly below the font table, then trim trailing empties.
    if let Some(mut font) = cts.font_slot_opt()? {
        while font > 0 && cts.entries[font - 1] == EMPTY_SLOT {
            let fp = cts.parent(font)?;
            cts.entries.remove(font - 1);
            cts.repoint(fp, font, font - 1)?;
            font -= 1;
        }
    }
    while cts.entries.last().is_some_and(|e| *e == EMPTY_SLOT) {
        cts.entries.pop();
    }

    // Escher: remove the containers, fix the counters; lastspid and FIDCL stay.
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut pibs = Vec::new();
    let mut per: Vec<(u16, u32)> = Vec::new();
    for (_, d) in esc.drawings.iter_mut() {
        let n = strip(d, &gone, &mut pibs);
        if n > 0 {
            let dgid = d.find(0xF008).map_or(0, |r| r.vi >> 4);
            drawing_counters(d, -(n as i32), None)?;
            per.push((dgid, n));
        }
    }
    for (dgid, n) in per {
        group_counters(&mut esc.head, dgid, -(n as i32), 0)?;
    }
    if !pibs.is_empty() {
        lower_crefs(&mut esc.head, &mut 0, &pibs);
    }
    let contents = cx.cts.finish()?;
    install(input, &contents, &esc.to_bytes())
}
