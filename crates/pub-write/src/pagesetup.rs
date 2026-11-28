//! Page setup (spec 4Q): page size / orientation and margins.
//!
//! * [`set_page_size`] writes the "Current" form of the page-setup record (object 292) and
//!   updates every size field of `Contents` (spec 4Q.2); on-page shapes keep their Escher
//!   anchor (it is measured from the page centre), shapes that no longer touch the page move
//!   to the scratch area with page-corner anchors (4Q.4, 4Q.5, the behaviour of Publisher's
//!   user interface).
//! * [`set_margins`] rewrites the four layout guides of the master page (4Q.6).
//!
//! Items are edited either in place (fixed-size integers) or by decoding the whole object
//! block, changing the items and re-encoding it; the re-encoding must reproduce the old
//! bytes exactly before anything is changed, otherwise the object is refused.

use super::group::{
    client_anchor, client_anchor_bytes, install, int_at, items_of, put, rd, Body, Cts, EscherDoc,
};
use super::{Result, WriteError};
use pub_format::items::{decode_block, header, Item, Value};
use pub_format::Streams;
use std::collections::HashSet;

const KIND_PAGE: u64 = 67;
const KIND_DOCUMENT: u64 = 68;
const KIND_SETUP: u64 = 138;
const KIND_GROUP: u16 = 48;

/// 1,800 pt: the placeholder extent of a page object (spec 4Q.2).
const BIG: i64 = 22_860_000;
/// Item 17 of a kind-67 object is item 5 plus this (spec 4Q.2).
const BIG_ADD: i64 = 87_325_200;
/// 0.6 pt, the rounding step of the half-page formula.
const STEP: i64 = 7_620;
const MIN_EMU: u32 = 914_400;
/// Larger pages would make the scratch frame of spec 4Q.2 negative (untested, 4Q.8).
const MAX_EMU: u32 = 45_000_000;

const ESC_CONTAINER: u16 = 0xF004;
const ESC_GROUP: u16 = 0xF003;
const ESC_CLIENT_ANCHOR: u16 = 0xF010;

type AnchorPath = (usize, usize, usize, usize);

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::PageSetup(m.into()))
}

// ------------------------------------------------------------------ item encoding

pub(crate) fn enc_block(items: &[Item]) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    for it in items {
        body.extend_from_slice(&header(it.id, it.class));
        match &it.value {
            Value::Flag => {}
            Value::Int(x) => {
                let n = match it.class {
                    2 | 3 => 2,
                    4 | 11 | 13 | 14 | 23 => 4,
                    5 | 12 => 8,
                    c => return fail(format!("integer item {} has class {c}", it.id)),
                };
                if n < 8 && x >> (8 * n) != 0 {
                    return Err(WriteError::OutOfRange(*x as i64));
                }
                body.extend_from_slice(&x.to_le_bytes()[..n]);
            }
            Value::Bytes(b) => body.extend_from_slice(b),
            Value::Record(v) | Value::List(v) => body.extend_from_slice(&enc_block(v)?),
            Value::Raw(r) => {
                body.extend_from_slice(&(r.len() as u32 + 4).to_le_bytes());
                body.extend_from_slice(r);
            }
        }
    }
    let mut out = (body.len() as u32 + 4).to_le_bytes().to_vec();
    out.extend_from_slice(&body);
    Ok(out)
}

pub(crate) fn int(id: u16, v: i64) -> Result<Item> {
    let v = u32::try_from(v).map_err(|_| WriteError::OutOfRange(v))?;
    Ok(Item {
        id,
        class: 4,
        value: Value::Int(u64::from(v)),
    })
}

pub(crate) fn flag(id: u16) -> Item {
    Item {
        id,
        class: 1,
        value: Value::Flag,
    }
}

pub(crate) fn utf16z(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
    v.extend_from_slice(&[0, 0]);
    v
}

pub(crate) fn put_item(items: &mut Vec<Item>, new: Item) {
    match items.iter_mut().find(|i| i.id == new.id) {
        Some(old) => *old = new,
        None => {
            items.push(new);
            items.sort_by_key(|i| i.id);
        }
    }
}

fn set_pair(items: &mut [Item], id: u16, a: (u16, i64), b: (u16, i64)) -> Result<()> {
    let Some(Value::Record(v)) = items.iter_mut().find(|i| i.id == id).map(|i| &mut i.value)
    else {
        return fail(format!("item {id} is not a record"));
    };
    put_item(v, int(a.0, a.1)?);
    put_item(v, int(b.0, b.1)?);
    Ok(())
}

/// Decode object `slot`, let `f` change the items, write the object back. Refuses a block
/// that does not re-encode to itself.
pub(crate) fn edit_object(
    cts: &mut Cts,
    slot: usize,
    f: impl FnOnce(&mut Vec<Item>) -> Result<()>,
) -> Result<()> {
    let off = cts.offset(slot)?;
    let len = rd(&cts.blocks, off)? as usize;
    let mut items = decode_block(&cts.blocks, off)?;
    let old = cts.blocks[off..off + len].to_vec();
    if enc_block(&items)? != old {
        return fail(format!(
            "object {slot} cannot be re-encoded exactly; refusing to rewrite it"
        ));
    }
    f(&mut items)?;
    let new = enc_block(&items)?;
    if new != old {
        cts.splice(off, len, &new)?;
    }
    Ok(())
}

/// Set 4-byte integers inside the record item `item` of object `slot`, in place.
fn set_in_record(cts: &mut Cts, slot: usize, item: u16, vals: &[(u16, i64)]) -> Result<()> {
    let off = cts.offset(slot)?;
    let top = items_of(&cts.blocks, off)?;
    let Some(&(_, hp, p, _)) = top.iter().find(|i| i.0 == item) else {
        return fail(format!("object {slot} has no item {item}"));
    };
    if cts.blocks[hp + 1] >> 3 != 17 {
        return fail(format!("object {slot} item {item} is not a record"));
    }
    let inner = items_of(&cts.blocks, p)?;
    for &(sub, v) in vals {
        let v = u32::try_from(v).map_err(|_| WriteError::OutOfRange(v))?;
        let Some(&(_, ih, ip, ie)) = inner.iter().find(|i| i.0 == sub) else {
            return fail(format!("object {slot} item {item} has no item {sub}"));
        };
        if ie - ip != 4 || cts.blocks[ih + 1] >> 3 != 4 {
            return fail(format!("object {slot} item {item}.{sub} is not a u32"));
        }
        put(&mut cts.blocks, ip, v)?;
    }
    Ok(())
}

// ------------------------------------------------------------------- the guides

/// The four margin guides of spec 4Q.6 from absolute positions (EMU from the top-left).
pub(crate) fn guide_list(left: i64, right: i64, top: i64, bottom: i64) -> Result<Item> {
    let rec = |pos: i64, flags: [u16; 2]| -> Result<Item> {
        let mut v = Vec::new();
        if pos != 0 {
            v.push(int(1, pos)?);
        }
        v.extend(flags.map(flag));
        Ok(Item {
            id: 0,
            class: 17,
            value: Value::Record(v),
        })
    };
    Ok(Item {
        id: 2,
        class: 20,
        value: Value::List(vec![
            rec(left, [3, 4])?,
            rec(right, [2, 4])?,
            rec(top, [3, 4])?,
            rec(bottom, [2, 4])?,
        ]),
    })
}

/// The four guide positions of object 289's item list (a missing item 1 is position 0).
pub(crate) fn guide_positions(items: &[Item]) -> Result<[i64; 4]> {
    let Some(Value::List(els)) = items.iter().find(|i| i.id == 2).map(|i| &i.value) else {
        return fail("the layout guides object has no guide list");
    };
    if els.len() != 4 {
        return fail(format!(
            "the layout guides object holds {} guides, expected the four margin guides",
            els.len()
        ));
    }
    let mut out = [0i64; 4];
    for (o, e) in out.iter_mut().zip(els) {
        let Value::Record(v) = &e.value else {
            return fail("a margin guide is not a record");
        };
        *o = v
            .iter()
            .find(|i| i.id == 1)
            .and_then(Item::as_int)
            .unwrap_or(0) as i64;
    }
    Ok(out)
}


/// Set the page margins (spec 4Q.6), in EMU. Only the layout-guides object of the master
/// page changes: left, right, top, bottom guide records at their absolute positions; a
/// margin of 0 writes no position item. Margins must fit the page.
pub fn set_margins(
    input: &[u8],
    left_emu: u32,
    right_emu: u32,
    top_emu: u32,
    bottom_emu: u32,
) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let (w, h) = (
        i64::from(doc.page_width_emu),
        i64::from(doc.page_height_emu),
    );
    let (l, r, t, b) = (
        i64::from(left_emu),
        i64::from(right_emu),
        i64::from(top_emu),
        i64::from(bottom_emu),
    );
    if l + r >= w || t + b >= h {
        return fail("the margins leave no room on the page");
    }
    let streams = Streams::from_cfb(input)?;
    let mut cts = Cts::parse(&streams.contents)?;
    for slot in super::masters::guide_slots(&cts)? {
        edit_object(&mut cts, slot, |items| {
            guide_positions(items)?;
            put_item(items, guide_list(l, w - r, t, h - b)?);
            Ok(())
        })?;
    }
    // A "Current"-form page-setup record mirrors the margins (items 6..9 of its first
    // item-18 element: top, left, bottom, right; seen in `page-a4-landscape-margins-2pages`).
    // The preset form does not (margin-only samples left it alone).
    if let Some(setup) = cts.slot_of_kind(KIND_SETUP) {
        edit_object(&mut cts, setup, |items| {
            let Some(Value::Record(rec)) =
                items.iter_mut().find(|i| i.id == 4).map(|i| &mut i.value)
            else {
                return Ok(());
            };
            if !rec.iter().any(|i| i.id == 5) {
                return Ok(());
            }
            let p18 = list_pos(rec, 18)?;
            if let Value::List(els) = &mut rec[p18].value {
                if let Value::Record(v) = &mut els[0].value {
                    for (k, x) in [t, l, b, r].iter().enumerate() {
                        put_item(v, int(6 + k as u16, *x)?);
                    }
                }
            }
            Ok(())
        })?;
    }
    install(input, &cts.finish()?, &streams.escher)
}

// ------------------------------------------------------------------- page size

fn ceil600_half(v: i64) -> i64 {
    // ceil(v / 2 / STEP) * STEP; v / 2 may be a half EMU.
    (v + 2 * STEP - 1) / (2 * STEP) * STEP
}

fn has_item(cts: &Cts, slot: usize, id: u16) -> bool {
    cts.offset(slot)
        .and_then(|o| items_of(&cts.blocks, o))
        .is_ok_and(|v| v.iter().any(|i| i.0 == id))
}

/// Integer item `id` (up to 8 bytes) of the object block, 0 when absent.
fn block_int(cts: &Cts, slot: usize, id: u16) -> usize {
    let Ok(off) = cts.offset(slot) else { return 0 };
    let Ok(top) = items_of(&cts.blocks, off) else { return 0 };
    top.iter()
        .find(|i| i.0 == id && i.3 - i.2 <= 8)
        .map_or(0, |i| int_at(&cts.blocks, i.2, i.3 - i.2))
}

/// The scratch page: the first kind-67 object in the root list that is neither a document
/// page (item 6 non-zero) nor the master page (has item 3).
fn scratch_slot(cts: &Cts, doc_slot: usize) -> Result<Option<usize>> {
    for id in cts.child_ids(doc_slot)? {
        let id = id as usize;
        if cts.kind(id) == Some(KIND_PAGE)
            && !has_item(cts, id, 3)
            && block_int(cts, id, 6) == 0
        {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn set_children(cts: &mut Cts, slot: usize, ids: &[u32]) -> Result<()> {
    if !ids.is_empty() {
        return cts.set_child_ids(slot, ids);
    }
    // A page without shapes has neither item 1 nor item 2 (spec 4Q.4).
    let off = cts.offset(slot)?;
    let top = items_of(&cts.blocks, off)?;
    for id in [2u16, 1] {
        if let Some(&(_, hp, _, e)) = top.iter().find(|i| i.0 == id) {
            cts.splice(hp, e - hp, &[])?;
            let len = rd(&cts.blocks, off)? as usize - (e - hp);
            put(&mut cts.blocks, off, len as u32)?;
        }
    }
    Ok(())
}

/// Item 3 = true on a shape that was on the page (spec 4Q.4 rule 4).
fn add_page_flag(cts: &mut Cts, obj: usize) -> Result<()> {
    let off = cts.offset(obj)?;
    let top = items_of(&cts.blocks, off)?;
    if top.iter().any(|i| i.0 == 3) {
        return Ok(());
    }
    let at = top
        .iter()
        .find(|i| i.0 > 3)
        .map_or(off + rd(&cts.blocks, off)? as usize, |i| i.1);
    let ins = header(3, 1);
    cts.splice(at, 0, &ins)?;
    let len = rd(&cts.blocks, off)? + ins.len() as u32;
    put(&mut cts.blocks, off, len)
}

fn list_pos(rec: &[Item], id: u16) -> Result<usize> {
    match rec.iter().position(|i| i.id == id) {
        Some(p) if matches!(&rec[p].value, Value::List(l) if !l.is_empty()) => Ok(p),
        _ => fail(format!("the page-setup record has no list item {id}")),
    }
}

/// Rewrite object 292's item 4 as the "Current" form (spec 4Q.3, form C).
fn to_current_form(items: &mut [Item], w: i64, h: i64, m: [i64; 4]) -> Result<()> {
    let Some(Value::Record(rec)) = items.iter_mut().find(|i| i.id == 4).map(|i| &mut i.value)
    else {
        return fail("the page-setup record has no item 4");
    };
    let was_current = rec.iter().any(|i| i.id == 5);
    if !was_current {
        rec.retain(|i| !matches!(i.id, 3 | 4 | 6 | 7));
        put_item(rec, int(2, 161)?);
        put_item(
            rec,
            Item {
                id: 5,
                class: 24,
                value: Value::Raw(utf16z("Current")),
            },
        );
        put_item(rec, int(8, 2)?);
    }
    let p18 = list_pos(rec, 18)?;
    if let Value::List(l) = &mut rec[p18].value {
        match &mut l[0].value {
            Value::Record(v) => {
                put_item(v, int(3, w)?);
                put_item(v, int(4, h)?);
                // Items 6..9 hold the margins top, left, bottom, right (spec 4Q.3 note).
                for (k, x) in m.iter().enumerate() {
                    put_item(v, int(6 + k as u16, *x)?);
                }
            }
            _ => return fail("element of item 18 is not a record"),
        }
    }
    if !was_current {
        let p20 = list_pos(rec, 20)?;
        if let Value::List(l) = &mut rec[p20].value {
            if let Value::Record(v) = &mut l[0].value {
                put_item(v, flag(5));
            }
        }
    }
    Ok(())
}

/// Path to the client anchor atom of the shape container of Contents object `obj`.
fn anchor_path(esc: &EscherDoc, obj: u32) -> Option<AnchorPath> {
    for (di, (_, d)) in esc.drawings.iter().enumerate() {
        for (gi, g) in d.kids().iter().enumerate().filter(|(_, g)| g.ty == ESC_GROUP) {
            for (ci, c) in g.kids().iter().enumerate() {
                if c.ty == ESC_CONTAINER && c.object() == Some(obj) {
                    let ai = c.kids().iter().position(|a| a.ty == ESC_CLIENT_ANCHOR)?;
                    return Some((di, gi, ci, ai));
                }
            }
        }
    }
    None
}

fn anchor_mut(esc: &mut EscherDoc, p: AnchorPath) -> Option<&mut Vec<u8>> {
    let Body::Cont(gk) = &mut esc.drawings.get_mut(p.0)?.1.body else {
        return None;
    };
    let Body::Cont(ck) = &mut gk.get_mut(p.1)?.body else {
        return None;
    };
    let Body::Cont(ak) = &mut ck.get_mut(p.2)?.body else {
        return None;
    };
    match &mut ak.get_mut(p.3)?.body {
        Body::Atom(d) => Some(d),
        Body::Cont(_) => None,
    }
}

/// The scratch-area anchor of spec 4Q.5: page-corner values plus the edge bits. An axis is
/// measured from the far edge when the shape's far side lies beyond that edge (the rule
/// the samples follow; the spec's "left >= W" wording does not fit `Rect Far`).
fn scratch_anchor(rect: [i32; 4], w: i64, h: i64) -> Result<Vec<u8>> {
    let [l, t, r, b] = rect.map(i64::from);
    let (lp, rp) = (l + w / 2, r + w / 2);
    let (tp, bp) = (t + h / 2, b + h / 2);
    let bit0 = rp > w;
    let bit1 = bp > h;
    let (lp, rp) = if bit0 { (lp - w, rp - w) } else { (lp, rp) };
    let (tp, bp) = if bit1 { (tp - h, bp - h) } else { (tp, bp) };
    let cv = |x: i64| i32::try_from(x).map_err(|_| WriteError::OutOfRange(x));
    let mut d = client_anchor_bytes([cv(lp)?, cv(tp)?, cv(rp)?, cv(bp)?]);
    let bits = u16::from(bit0) | u16::from(bit1) << 1;
    if bits != 0 {
        d.extend_from_slice(&header(5, 3));
        d.extend_from_slice(&bits.to_le_bytes());
        let n = d.len() as u32;
        d[..4].copy_from_slice(&n.to_le_bytes());
    }
    Ok(d)
}

/// Change the page size (spec 4Q.2-4Q.5, "Current" form). `width_emu` and `height_emu` are
/// the new size (swap them for a change of orientation). On-page shapes keep their Escher
/// anchor; shapes that no longer touch the page move to the scratch area, the way
/// Publisher's user interface moves them (both axes converted); shapes that were on the
/// page gain item 3 = true. Margins are kept (the right and bottom guides follow the size).
///
/// Refused with [`WriteError::PageSetup`]: a two-page (facing) master, a shape that would
/// have to move to the scratch area but is a group, a table, a picture, rotated, or part of
/// a linked text-box chain, no scratch page, and sizes outside 1 in .. 49 in. A shape
/// exactly touching the page edge counts as on the page.
pub fn set_page_size(input: &[u8], width_emu: u32, height_emu: u32) -> Result<Vec<u8>> {
    if !(MIN_EMU..=MAX_EMU).contains(&width_emu) || !(MIN_EMU..=MAX_EMU).contains(&height_emu) {
        return fail(format!(
            "page size {width_emu} x {height_emu} EMU is outside 1 in .. 49 in"
        ));
    }
    let (w, h) = (i64::from(width_emu), i64::from(height_emu));
    let doc = pub_format::read_bytes(input)?;
    let (ow, oh) = (
        i64::from(doc.page_width_emu),
        i64::from(doc.page_height_emu),
    );
    let streams = Streams::from_cfb(input)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let doc_slot = cts
        .slot_of_kind(KIND_DOCUMENT)
        .ok_or_else(|| WriteError::PageSetup("the file has no document object".into()))?;
    let gslots = super::masters::guide_slots(&cts)?;
    let scratch = scratch_slot(&cts, doc_slot)?;

    // ---- which shapes stay, which move (from the reader's view of the old geometry)
    let in_chain: HashSet<(usize, usize)> = doc
        .chains()
        .iter()
        .filter(|c| c.len() > 1)
        .flatten()
        .map(|r| (r.page, r.shape))
        .collect();
    let mut flagged: Vec<usize> = Vec::new();
    let mut moves: Vec<(usize, usize, AnchorPath)> = Vec::new();
    for (pi, page) in doc.pages.iter().enumerate() {
        for (si, s) in page.shapes.iter().enumerate() {
            if s.group.is_some() {
                continue;
            }
            let [l, t, r, b] = [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu].map(i64::from);
            let on_page = 2 * r >= -w && 2 * l <= w && 2 * b >= -h && 2 * t <= h;
            let what = format!("shape {} (kind {})", s.object_number, s.kind);
            if s.rotation_deg.abs() > 0.001 && !on_page {
                return fail(format!("{what} is rotated and may leave the page"));
            }
            if s.kind == 1 && s.picture.is_none() && s.table.is_none() {
                flagged.push(s.object_number as usize);
            }
            if on_page {
                continue;
            }
            if s.kind == KIND_GROUP || s.table.is_some() || s.picture.is_some() {
                return fail(format!(
                    "{what} would move to the scratch area; groups, tables and pictures are not supported"
                ));
            }
            if in_chain.contains(&(pi, si)) {
                return fail(format!("{what} is linked to other text boxes"));
            }
            if scratch.is_none() {
                return fail(format!(
                    "{what} would move to the scratch area but the file has none"
                ));
            }
            let Some(path) = anchor_path(&esc, s.object_number) else {
                return fail(format!("{what} has no client anchor"));
            };
            moves.push((page.object_number as usize, s.object_number as usize, path));
        }
    }

    // ---- sizes
    set_in_record(&mut cts, doc_slot, 18, &[(1, w), (2, h)])?;
    let (a, b) = (ceil600_half(w), ceil600_half(h));
    for slot in 0..cts.entries.len() {
        if cts.kind(slot) != Some(KIND_PAGE) {
            continue;
        }
        let (w5, h5) = if Some(slot) == scratch {
            (BIG - a, BIG - b)
        } else {
            (BIG - a + w / 2, BIG - b + h / 2)
        };
        if has_item(&cts, slot, 5) {
            set_in_record(&mut cts, slot, 5, &[(1, w5), (2, h5)])?;
        }
        if has_item(&cts, slot, 17) {
            set_in_record(&mut cts, slot, 17, &[(1, w5 + BIG_ADD), (2, h5 + BIG_ADD)])?;
        }
    }
    let mut margins4 = [0i64; 4];
    for (k, &gslot) in gslots.iter().enumerate() {
        edit_object(&mut cts, gslot, |items| {
            let p = guide_positions(items)?;
            if k == 0 {
                margins4 = [p[2], p[0], oh - p[3], ow - p[1]];
            }
            let (rp, bp) = (w - (ow - p[1]), h - (oh - p[3]));
            if rp < p[0] || bp < p[2] {
                return fail("the margins do not fit the new page size");
            }
            put_item(items, guide_list(p[0], rp, p[2], bp)?);
            set_pair(items, 3, (1, w), (2, h))
        })?;
    }
    if let Some(setup) = cts.slot_of_kind(KIND_SETUP) {
        edit_object(&mut cts, setup, |items| to_current_form(items, w, h, margins4))?;
    }

    // ---- moves to the scratch area
    if let Some(scr) = scratch {
        let mut by_page: Vec<(usize, Vec<u32>)> = Vec::new();
        for (page, obj, path) in &moves {
            match by_page.iter_mut().find(|m| m.0 == *page) {
                Some(m) => m.1.push(*obj as u32),
                None => by_page.push((*page, vec![*obj as u32])),
            }
            let d = anchor_mut(&mut esc, *path)
                .ok_or_else(|| WriteError::PageSetup("bad Escher path".into()))?;
            let cur = client_anchor(d)?;
            *d = scratch_anchor(cur, w, h)?;
            cts.set_int(*obj, 5, scr)?;
        }
        let mut to_scratch: Vec<u32> = Vec::new();
        for (page, moved) in &by_page {
            let all = cts.child_ids(*page)?;
            to_scratch.extend(all.iter().copied().filter(|x| moved.contains(x)));
            let rest: Vec<u32> = all.into_iter().filter(|x| !moved.contains(x)).collect();
            set_children(&mut cts, *page, &rest)?;
        }
        if !to_scratch.is_empty() {
            let mut all = cts.child_ids(scr)?;
            all.extend(to_scratch);
            cts.set_child_ids(scr, &all)?;
        }
    }
    for obj in flagged {
        add_page_flag(&mut cts, obj)?;
    }
    install(input, &cts.finish()?, &esc.to_bytes())
}
