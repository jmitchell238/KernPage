//! Add a new text box to a page (spec 4L): a cloned Escher text-box container, a new
//! `Contents` kind-1 object with its registry/overlap bookkeeping, and a new Quill story.

use super::blank::{el_rec, Blk};
use super::group::{
    anchor_atom, drawing_counters, group_counters, index_entry, install, items_of, put, rd, Body,
    Cts, EscherDoc, Rec, KIND_FONT_TABLE,
};
use super::shape::{page_drawing, props};
use super::quill::{self, NewFrame};
use super::{install_quill, Result, WriteError, EMU_PER_POINT, KIND_TEXT_BOX};
use pub_format::Streams;
use std::collections::HashSet;

const KIND_PAGE: u64 = 67;
const KIND_STORY_MAP: u64 = 97;
const KIND_STORY_INFO: u64 = 101;
const KIND_STORY_ROOT: u64 = 91;
/// Items 8 and 9 of every registry-284 record (spec 4L.2).
const REGISTRY_CONSTANT: u32 = 0xCB18_967C;
/// FSP record header of a text box (instance 0xCA, version 2) and its flags (spec 4L.1).
const FSP_TEXT_BOX: u16 = 0x0CA2;
const FSP_FLAGS: u32 = 0x0A00;

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::TextBox(m.into()))
}

/// Add a text box holding `text` to page `page_index` (0-based). Position and size are in
/// points from the page's top-left corner. Paragraphs in `text` are separated by `\r` or
/// `\n`; the story always ends with a paragraph mark, so an empty string gives an empty box.
/// A file that has a text box uses its Escher container as the template; a blank one gets
/// the container, the font table, the story registries' items and the text engine's chunks
/// built from nothing, as Publisher creates them with its first text (spec 6.11).
pub fn add_text_box(
    input: &[u8],
    page_index: usize,
    left_pt: f64,
    top_pt: f64,
    width_pt: f64,
    height_pt: f64,
    text: &str,
) -> Result<Vec<u8>> {
    if ![left_pt, top_pt, width_pt, height_pt]
        .iter()
        .all(|v| v.is_finite())
        || width_pt <= 0.0
        || height_pt <= 0.0
    {
        return fail("the frame must be finite with a positive width and height");
    }
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .surface(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let emu = |v: f64| (v * EMU_PER_POINT).round();
    let (cx, cy) = (
        f64::from(doc.page_width_emu) / 2.0,
        f64::from(doc.page_height_emu) / 2.0,
    );
    let side = |v: f64| {
        let r = v.round();
        i32::try_from(r as i64).map_err(|_| WriteError::OutOfRange(r as i64))
    };
    let anchor = [
        side(emu(left_pt) - cx)?,
        side(emu(top_pt) - cy)?,
        side(emu(left_pt + width_pt) - cx)?,
        side(emu(top_pt + height_pt) - cy)?,
    ];
    let to_u32 = |v: f64| u32::try_from(v as i64).map_err(|_| WriteError::OutOfRange(v as i64));
    let (width_emu, height_emu) = (to_u32(emu(width_pt))?, to_u32(emu(height_pt))?);

    let streams = Streams::from_cfb(input)?;
    let (quill, story) = quill::add_story(
        &streams.quill,
        text,
        &NewFrame {
            cache: None,
            left_pt,
            top_pt,
            width_pt,
            height_pt,
            width_emu,
            height_emu,
        },
    )?;

    // ---- Escher: clone a text-box container, patch four fields, append.
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let page_objs: HashSet<u32> = page.shapes.iter().map(|s| s.object_number).collect();
    let is_box = |r: &Rec| {
        r.ty == 0xF004
            && r.object().is_some()
            && r.find(0xF00D).is_some()
            && r.find(0xF00A).is_some_and(|f| {
                f.vi == FSP_TEXT_BOX
                    && matches!(&f.body, Body::Atom(d) if rd(d, 4).ok() == Some(FSP_FLAGS))
            })
    };
    let mut found: Option<(usize, Rec, bool)> = None;
    for (di, (_, d)) in esc.drawings.iter().enumerate() {
        let Some(g) = d.kids().iter().find(|r| r.ty == 0xF003) else {
            continue;
        };
        for k in g.kids().iter().filter(|k| is_box(k)) {
            let on_page = k.object().is_some_and(|o| page_objs.contains(&o));
            if found.as_ref().is_none_or(|f| !f.2 && on_page) {
                found = Some((di, k.clone(), on_page));
            }
        }
    }
    let (di, mut container) = match found {
        Some((di, container, _)) => (di, container),
        None => (page_drawing(&esc, &page_objs)?, fresh_text_box_container()),
    };

    let mut cts = Cts::parse(&streams.contents)?;
    ensure_font_table(&mut cts)?;
    let font = cts.tail()?.0;
    let new_id = font as u32;
    let page_obj = page.object_number as usize;
    if cts.kind(page_obj) != Some(KIND_PAGE) {
        return fail("the page has no page object in Contents");
    }

    let drawing = &mut esc.drawings[di].1;
    let dgid = drawing.find(0xF008).map_or(0, |r| r.vi >> 4);
    let spid = match drawing.find(0xF008).map(|r| &r.body) {
        Some(Body::Atom(d)) => rd(d, 4)? + 1,
        _ => return fail("the drawing has no 0xF008"),
    };
    for k in container.kids_mut()? {
        match k.ty {
            0xF00A => put(k.data_mut()?, 0, spid)?,
            0xF00B => {
                let n = usize::from(k.vi >> 4);
                let d = k.data_mut()?;
                let at = (0..n)
                    .find(|i| {
                        d.get(6 * i..6 * i + 2)
                            .is_some_and(|b| b[0] == 0x80 && b[1] & 0x3F == 0)
                    })
                    .ok_or_else(|| WriteError::TextBox("template lacks property 0x80".into()))?;
                put(d, 6 * at + 2, story)?;
            }
            0xF010 => *k.data_mut()? = super::group::client_anchor_bytes(anchor),
            0xF011 => {
                let mut cd = vec![0x0a, 0, 0, 0, 0x01, 0x68];
                cd.extend_from_slice(&new_id.to_le_bytes());
                *k.data_mut()? = cd;
            }
            0xF00D => {
                let mut cd = vec![0x0a, 0, 0, 0, 0x01, 0x20];
                cd.extend_from_slice(&story.to_le_bytes());
                *k.data_mut()? = cd;
            }
            _ => {}
        }
    }
    drawing_counters(drawing, 1, Some(spid))?;
    let group = drawing
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF003)
        .ok_or_else(|| WriteError::TextBox("the drawing has no page group".into()))?;
    group.kids_mut()?.push(container);
    group_counters(&mut esc.head, dgid, 1, 1)?;

    // ---- Contents.
    // Older text boxes on this page that the new one overlaps (items 45/70/71).
    let [l, t, r, b] = anchor;
    for s in &page.shapes {
        if s.kind == KIND_TEXT_BOX
            && s.chain_story.is_some()
            && s.group.is_none()
            && s.left_emu < r
            && s.right_emu > l
            && s.top_emu < b
            && s.bottom_emu > t
        {
            add_overlap(&mut cts, s.object_number as usize, new_id)?;
        }
    }
    // Registries 283 (kind 97) and 284 (kind 101).
    let root = cts
        .slot_of_kind(KIND_STORY_ROOT)
        .ok_or_else(|| WriteError::TextBox("the story root object (kind 91) is missing".into()))?;
    let slot_of = |cts: &Cts, kind: u64| {
        (0..cts.entries.len())
            .find(|&i| cts.kind(i) == Some(kind) && cts.parent(i).ok() == Some(root))
    };
    let (Some(map), Some(info)) = (
        slot_of(&cts, KIND_STORY_MAP),
        slot_of(&cts, KIND_STORY_INFO),
    ) else {
        return fail("the story registries (kinds 97 and 101) are missing");
    };
    let rec_283 = |_: &[u8], _: usize| {
        let mut b = Vec::new();
        b.extend_from_slice(&[0x01, 0x20]);
        b.extend_from_slice(&story.to_le_bytes());
        b.extend_from_slice(&[0x03, 0x68]);
        b.extend_from_slice(&new_id.to_le_bytes());
        Ok(b)
    };
    ensure_registry(&mut cts, map)?;
    ensure_registry(&mut cts, info)?;
    append_record(&mut cts, map, &rec_283)?;
    let rec_284 = |last: &[u8], ordinal: usize| {
        let its = items_of(last, 2)?;
        let mut b = Vec::new();
        b.extend_from_slice(&[0x01, 0x20]);
        b.extend_from_slice(&story.to_le_bytes());
        b.extend_from_slice(&[0x07, 0x20]);
        b.extend_from_slice(&(ordinal as u32).to_le_bytes());
        for want in [8u16, 9] {
            match its.iter().find(|i| i.0 == want) {
                Some(&(_, h, _, e)) => b.extend_from_slice(&last[h..e]),
                // The first record of a file: the constant of every record (spec 4L.2).
                None => {
                    b.extend_from_slice(&pub_format::items::header(want, 11));
                    b.extend_from_slice(&REGISTRY_CONSTANT.to_le_bytes());
                }
            }
        }
        Ok(b)
    };
    append_record(&mut cts, info, &rec_284)?;

    // New object before the font table, its index entry, and the page's child list.
    let (_, at) = cts.tail()?;
    let mut body = vec![0u8; 4];
    body.extend_from_slice(&[0x02, 0x08, 0x04, 0x10, 0x03, 0x00]);
    body.extend_from_slice(&[0x0c, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&[0x0d, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    for (id, v) in [
        (0x27u8, story),
        (0xaa, width_emu),
        (0xab, height_emu),
        (0xb7, 0),
    ] {
        body.extend_from_slice(&[id, 0x20]);
        body.extend_from_slice(&v.to_le_bytes());
    }
    let n = body.len() as u32;
    body[..4].copy_from_slice(&n.to_le_bytes());
    cts.blocks.splice(at..at, body.iter().copied());
    cts.shift(at, true, body.len() as isize)?;
    cts.insert_entries(font, vec![index_entry(1, page_obj as u32, 0x0103, false, 22)])?;
    cts.set_int(font, 4, at)?;
    cts.set_int(font, 5, page_obj)?;
    let mut kids = cts.child_ids(page_obj)?;
    kids.push(new_id);
    cts.set_child_ids(page_obj, &kids)?;

    let out = install(input, &cts.finish()?, &esc.to_bytes())?;
    install_quill(&out, &quill)
}

type MakeRecord<'a> = dyn Fn(&[u8], usize) -> Result<Vec<u8>> + 'a;

/// Grow the length of the block at `off` by `delta`.
pub(crate) fn grow_block(cts: &mut Cts, off: usize, delta: usize) -> Result<()> {
    let len = rd(&cts.blocks, off)? as usize + delta;
    put(&mut cts.blocks, off, len as u32)
}

/// Append one record (`make(last record, new count)` gives its items) to the item-2 list of
/// registry object `slot` and add 1 to its item-1 count.
pub(crate) fn append_record(cts: &mut Cts, slot: usize, make: &MakeRecord) -> Result<()> {
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let find = |id: u16| items.iter().find(|i| i.0 == id).copied();
    let (_, _, cp, ce) =
        find(1).ok_or_else(|| WriteError::TextBox("registry has no count".into()))?;
    let (_, _, lp, le) =
        find(2).ok_or_else(|| WriteError::TextBox("registry has no list".into()))?;
    if ce - cp != 4 {
        return fail("unexpected registry count item");
    }
    // The last record of the list: `00 88 <block>`.
    let mut last: Option<&[u8]> = None;
    let mut pos = lp + 4;
    while pos < le {
        let len = rd(&cts.blocks, pos + 2)? as usize;
        if cts.blocks[pos + 1] >> 3 != 17 || pos + 2 + len > le {
            return fail("unexpected registry record");
        }
        last = Some(&cts.blocks[pos..pos + 2 + len]);
        pos += 2 + len;
    }
    // The first record of a registry has no predecessor to copy from.
    let count = rd(&cts.blocks, cp)? as usize + 1;
    let items_bytes = make(last.unwrap_or(&[0x00, 0x88, 4, 0, 0, 0]), count)?;
    let mut rec = vec![0x00, 0x88];
    rec.extend_from_slice(&((4 + items_bytes.len()) as u32).to_le_bytes());
    rec.extend_from_slice(&items_bytes);
    put(&mut cts.blocks, cp, count as u32)?;
    cts.blocks.splice(le..le, rec.iter().copied());
    cts.shift(le, true, rec.len() as isize)?;
    let ll = rd(&cts.blocks, lp)? as usize + rec.len();
    put(&mut cts.blocks, lp, ll as u32)?;
    grow_block(cts, off, rec.len())
}

/// Items 45 (flag), 70 (count) and 71 (list of newer overlapping object ids) on an older
/// text box (spec 4L.2).
pub(crate) fn add_overlap(cts: &mut Cts, obj: usize, new_id: u32) -> Result<()> {
    add_overlap_with(cts, obj, new_id, true)
}

/// As [`add_overlap`]; `flag45` is false for a new picture, which gives the older box items
/// 70/71 but not item 45 (spec 4C.1.5).
pub(crate) fn add_overlap_with(
    cts: &mut Cts,
    obj: usize,
    new_id: u32,
    flag45: bool,
) -> Result<()> {
    let off = cts.offset(obj)?;
    let items = items_of(&cts.blocks, off)?;
    let find = |id: u16| items.iter().find(|i| i.0 == id).copied();
    let entry = {
        let mut e = vec![0x00, 0x68];
        e.extend_from_slice(&new_id.to_le_bytes());
        e
    };
    if let (Some((_, _, cp, ce)), Some((_, _, lp, le))) = (find(70), find(71)) {
        if ce - cp != 4 {
            return fail("unexpected overlap count item");
        }
        let c = rd(&cts.blocks, cp)? + 1;
        put(&mut cts.blocks, cp, c)?;
        cts.blocks.splice(le..le, entry.iter().copied());
        cts.shift(le, true, entry.len() as isize)?;
        let ll = rd(&cts.blocks, lp)? as usize + entry.len();
        put(&mut cts.blocks, lp, ll as u32)?;
        return grow_block(cts, off, entry.len());
    }
    let mut ins = Vec::new();
    if flag45 && find(45).is_none() {
        ins.extend_from_slice(&pub_format::items::header(45, 1));
    }
    ins.extend_from_slice(&[
        pub_format::items::header(70, 4)[0],
        pub_format::items::header(70, 4)[1],
        1,
        0,
        0,
        0,
        pub_format::items::header(71, 20)[0],
        pub_format::items::header(71, 20)[1],
    ]);
    ins.extend_from_slice(&((4 + entry.len()) as u32).to_le_bytes());
    ins.extend_from_slice(&entry);
    let at = items
        .iter()
        .find(|i| i.0 > 71)
        .map(|i| i.1)
        .ok_or_else(|| WriteError::TextBox("text box body has no item after 71".into()))?;
    cts.blocks.splice(at..at, ins.iter().copied());
    cts.shift(at, false, ins.len() as isize)?;
    grow_block(cts, off, ins.len())
}

/// Insert item `id` (already encoded) into the body of object `slot`, in id order.
fn insert_item(cts: &mut Cts, slot: usize, id: u16, bytes: &[u8]) -> Result<()> {
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let at = items
        .iter()
        .find(|i| i.0 > id)
        .map_or(off + rd(&cts.blocks, off)? as usize, |i| i.1);
    cts.blocks.splice(at..at, bytes.iter().copied());
    cts.shift(at, true, bytes.len() as isize)?;
    grow_block(cts, off, bytes.len())
}

/// The font table (kind 108, spec 6.11) that Publisher creates with a file's first story: one
/// record for Calibri, the last object, referenced from the story root (kind 91, item 4).
/// Does nothing when the file has one.
pub(crate) fn ensure_font_table(cts: &mut Cts) -> Result<()> {
    if cts.font_slot_any().is_some() {
        return Ok(());
    }
    let root = cts
        .slot_of_kind(KIND_STORY_ROOT)
        .ok_or_else(|| WriteError::TextBox("the story root object (kind 91) is missing".into()))?;
    // Calibri's font signature (usb and csb words) and PANOSE number.
    let signature: Vec<u8> = [0xE400_7EFFu32, 0xC200_247B, 9, 0, 0x2000_01FF, 0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let record = Blk::new()
        .c2(3, 0)
        .string(4, "Calibri")
        .u32(6, 0x0010_00FF)
        .u32(7, 0x0005_BE03)
        .item(0x0B, 9, &signature)
        .raw(0x0D, &[2, 15, 5, 2, 2, 2, 4, 3, 2, 4])
        .u32(0x0E, 0x1400);
    let body = Blk::new().u32(1, 1).list20(2, vec![el_rec(record)]).encode();
    let (id, at) = cts.tail()?;
    cts.blocks.splice(at..at, body.iter().copied());
    cts.insert_entries(id, vec![index_entry(KIND_FONT_TABLE as u16, root as u32, 0x0102, true, 1)])?;
    cts.set_int(id, 4, at)?;
    let mut item = pub_format::items::header(4, 14).to_vec();
    item.extend_from_slice(&(id as u32).to_le_bytes());
    insert_item(cts, root, 4, &item)
}

/// Give a registry (kinds 97 and 101) that has never held a record its count (item 1) and
/// its empty record list (item 2), and its index entry the item 11 that Publisher adds.
pub(crate) fn ensure_registry(cts: &mut Cts, slot: usize) -> Result<()> {
    let off = cts.offset(slot)?;
    if items_of(&cts.blocks, off)?.is_empty() {
        let mut ins = pub_format::items::header(1, 4).to_vec();
        ins.extend_from_slice(&0u32.to_le_bytes());
        ins.extend_from_slice(&pub_format::items::header(2, 20));
        ins.extend_from_slice(&4u32.to_le_bytes());
        insert_item(cts, slot, 1, &ins)?;
    }
    if cts.int(slot, 11).is_none() {
        let e = &mut cts.entries[slot];
        e.extend_from_slice(&[0x0b, 0x18, 1, 0]);
        let n = (e.len() - 2) as u32;
        e[2..6].copy_from_slice(&n.to_le_bytes());
    }
    Ok(())
}

/// A text-box Escher container for a file without a text box to copy (spec 6.11). Spid,
/// story, anchor and object number are patched in by the caller.
fn fresh_text_box_container() -> Rec {
    const INSET: u32 = 36_576;
    const REF0: u32 = 0x0800_0000;
    const REF7: u32 = 0x0800_0007;
    const FLAGS: u32 = 0x0008_0000;
    let own = vec![
        (0x80, 0),
        (0x81, INSET),
        (0x82, INSET),
        (0x83, INSET),
        (0x84, INSET),
        (0x85, 1),
        (0xBF, 0x0002_0000),
        (0x181, REF7),
        (0x183, REF7),
        (0x1BF, 0x0010_0000),
        (0x1C0, REF0),
        (0x1C2, REF7),
        (0x1CB, 25_400),
        (0x1FF, FLAGS),
        (0x201, 0x0800_0004),
        (0x285, 0),
        (0x2CB, 0),
        (0x2CC, 0),
        (0x2CE, 0),
        (0x2CF, 0),
        (0x384, INSET),
        (0x385, INSET),
        (0x386, INSET),
        (0x387, INSET),
    ];
    let mut tertiary = vec![(0x8D, 73_152), (0x17F, 0x0040_0040), (0x1FF, 0x0040_0000)];
    tertiary.extend((0x295..=0x29A).map(|id| (id, 0)));
    for base in [0x540u16, 0x580, 0x5C0, 0x600, 0x640] {
        if base != 0x540 {
            tertiary.push((base - 1, FLAGS));
        }
        tertiary.extend([(base, REF0), (base + 2, REF7)]);
    }
    tertiary.push((0x6FF, 0x0002_0002));
    tertiary.sort_by_key(|e| e.0);
    let mut fsp = 0u32.to_le_bytes().to_vec();
    fsp.extend_from_slice(&FSP_FLAGS.to_le_bytes());
    Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(vec![
            Rec::atom(FSP_TEXT_BOX, 0xF00A, fsp),
            props(3, 0xF00B, &own),
            props(3, 0xF122, &tertiary),
            anchor_atom([0; 4]),
            Rec::atom(0x018a, 0xF011, vec![0; 10]),
            Rec::atom(0x018a, 0xF00D, vec![0; 10]),
        ]),
    }
}
