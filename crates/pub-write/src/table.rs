//! Add and delete a table (spec 4E and 4E.1): a kind-16 table object, an empty index slot and
//! a kind-99 cells object in `Contents`; one rectangle per cell in the cell drawing plus one
//! frame shape in the page drawing (Escher); one Quill story for all cell texts with a `TCD `
//! chunk and an `MCLD` record of one frame per cell.
//!
//! Row and column insert/delete (spec 4E.1.5) will live here as well: the pieces below
//! (`grid`, `cell_records`, `cell_shape`, the registry helpers) are written so that they can
//! be reused for a table whose cells are not in row-major order.

use super::blank::{el_rec, Blk};
use super::group::{
    client_anchor_bytes, drawing_counters, group_counters, index_entry, install, items_of, put,
    rd, Body, Cts, EscherDoc, Rec,
};
use super::pages::EMPTY_SLOT;
use super::picture::{drop_overlap_refs, remove_objects};
use super::quill::{self, NewFrame};
use super::shape::{page_drawing, props};
use super::textbox::{append_record, ensure_font_table, ensure_registry};
use super::{
    install_quill, set_story_format, set_story_lists, CharFormat, CharSpan, LineRule, ListSpan,
    ParaAlign, ParaSpacing, ParaSpan, Result, WriteError, EMU_PER_POINT,
};
use pub_format::items::{decode_block, find_int};
use pub_format::Streams;
use std::collections::HashSet;

const KIND_PAGE: u64 = 67;
const KIND_TABLE: u64 = 16;
const KIND_CELLS: u64 = 99;
const KIND_STORY_INFO: u64 = 101;
const KIND_STORY_ROOT: u64 = 91;
/// Items 8 and 9 of every record of registry 284 (spec 4L.2).
const REGISTRY_CONSTANT: u32 = 0xCB18_967C;
/// Drawing id of the drawing that holds one rectangle per table cell (spids 3072 and up).
const CELL_DRAWING: u16 = 3;
/// FSP record headers: the cell rectangle (instance 1) and the table frame (instance 0xC9).
const FSP_CELL: u16 = 0x0012;
const FSP_FRAME: u16 = 0x0C92;
const FSP_FLAGS: u32 = 0x0A00;
/// Record header (version 10, instance 0x1A) of the client records.
const CLIENT_VI: u16 = 0x01AA;
/// Cell margins (0.04 in), the height of one Calibri 10 line, and cell item 14.
const MARGIN: u32 = 36_576;
const LINE_HEIGHT: u32 = 228_180;
const CELL_ITEM_14: u32 = 114_300;
/// Value of table item 4, which differs from session to session of Publisher (spec 4E.1.1).
const SESSION_VALUE: u16 = 0xA003;
/// Smallest column width (16.93843 pt) and row height (one text line) Publisher's
/// `AddTable` accepts in the cases we measured (spec 4E.1.1).
pub const MIN_COLUMN_EMU: u32 = 215_118;
pub const MIN_ROW_EMU: u32 = LINE_HEIGHT;
/// Most cells one table may have (the cell count is a 16-bit item).
pub const MAX_CELLS: u32 = 65_535;

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Table(m.into()))
}

/// A table to insert. Positions and sizes are EMU (12,700 per point), measured from the top
/// left corner of the page; the columns and the rows share the width and the height evenly.
#[derive(Debug, Clone)]
pub struct NewTable<'a> {
    pub left_emu: i64,
    pub top_emu: i64,
    pub width_emu: u32,
    pub height_emu: u32,
    pub rows: u32,
    pub columns: u32,
    /// Cell texts, row-major; fewer than `rows * columns` leaves the rest empty. `\n` (or
    /// `\r`) separates the paragraphs of a cell.
    pub cells: &'a [String],
}

impl<'a> NewTable<'a> {
    /// The same in points.
    pub fn in_points(
        left_pt: f64,
        top_pt: f64,
        width_pt: f64,
        height_pt: f64,
        rows: u32,
        columns: u32,
        cells: &'a [String],
    ) -> Self {
        let e = |v: f64| (v * EMU_PER_POINT).round();
        NewTable {
            left_emu: e(left_pt) as i64,
            top_emu: e(top_pt) as i64,
            width_emu: e(width_pt).max(0.0) as u32,
            height_emu: e(height_pt).max(0.0) as u32,
            rows,
            columns,
            cells,
        }
    }
}

/// `(running edge, size)` of each of `n` equal tracks across `total` EMU, rounded to whole EMU.
fn grid(total: u32, n: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::with_capacity(n as usize);
    let mut prev = 0u32;
    for i in 1..=u64::from(n) {
        let edge = ((u64::from(total) * i + u64::from(n) / 2) / u64::from(n)) as u32;
        out.push((edge, edge - prev));
        prev = edge;
    }
    out
}

fn grid_list(columns: &[(u32, u32)], rows: &[(u32, u32)]) -> Vec<Vec<u8>> {
    columns
        .iter()
        .chain(rows)
        .map(|&(edge, size)| el_rec(Blk::new().u32(1, edge).u32(2, size)))
        .collect()
}

/// One record of the cells object for the cell at (`row`, `column`) (spec 4E.1.2).
fn cell_record(row: u32, column: u32) -> Vec<u8> {
    let mut b = Blk::new();
    if row > 0 {
        b = b.u32(1, row).u32(2, row);
    }
    if column > 0 {
        b = b.u32(3, column).u32(4, column);
    }
    b = b.u32(9, LINE_HEIGHT);
    for id in 10..=13 {
        b = b.u32(id, MARGIN);
    }
    el_rec(b.u32(14, CELL_ITEM_14))
}

/// The cell frame's layout cache in the `MCLD` record: items 0-3 (top, left, bottom, right),
/// a view-dependent value that Publisher recomputes (spec 4E.1.3). Fitted to the one session
/// we have values for (about 0.89 units per point).
fn layout_cache(x_pt: f64, y_pt: f64, w_pt: f64, h_pt: f64) -> [u32; 4] {
    let u = |v: f64| v.round().max(0.0) as u32;
    let top = 7_476.7 + 0.8935 * y_pt;
    let left = 7_557.7 + 0.8935 * x_pt;
    [
        u(top),
        u(left),
        u(top + 0.8935 * h_pt - 0.5),
        u(left + 0.8935 * w_pt - 0.5),
    ]
}

fn spid_of(drawing: &Rec) -> Result<u32> {
    match drawing.find(0xF008).map(|r| &r.body) {
        Some(Body::Atom(d)) => rd(d, 4),
        _ => fail("the drawing has no 0xF008"),
    }
}

/// The Escher rectangle of one cell (spec 4E.1.4): cell 0 carries only the table's object
/// number, cell k its position k in the story as well.
fn cell_shape(spid: u32, table: u32, index: u32) -> Rec {
    let mut fsp = spid.to_le_bytes().to_vec();
    fsp.extend_from_slice(&FSP_FLAGS.to_le_bytes());
    let mut client = vec![0, 0, 0, 0, 0x02, 0x68];
    client.extend_from_slice(&table.to_le_bytes());
    if index > 0 {
        client.extend_from_slice(&[0x03, 0x20]);
        client.extend_from_slice(&index.to_le_bytes());
    }
    let n = client.len() as u32;
    client[..4].copy_from_slice(&n.to_le_bytes());
    Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(vec![
            Rec::atom(FSP_CELL, 0xF00A, fsp),
            props(
                3,
                0xF00B,
                &[
                    (0x183, 0),
                    (0x1BF, 0x001F_001C),
                    (0x1FF, 0x0008_0000),
                    (0x285, 0),
                    (0x2CB, 0),
                    (0x2CC, 0),
                    (0x2CE, 0),
                    (0x2CF, 0),
                ],
            ),
            props(
                3,
                0xF122,
                &[
                    (0x1BF, 0x0060_0000),
                    (0x1FF, 0x0040_0040),
                    (0x295, 0),
                    (0x296, 0),
                    (0x297, 0),
                    (0x298, 0),
                    (0x299, 0),
                    (0x29A, 0),
                ],
            ),
            Rec::atom(CLIENT_VI, 0xF010, client),
        ]),
    }
}

/// The table's frame shape in the page drawing (spec 4E.1.4): a text-box-like container with
/// the table's frame as its anchor.
fn frame_shape(spid: u32, story: u32, table: u32, anchor: [i32; 4]) -> Rec {
    const REF0: u32 = 0x0800_0000;
    const REF7: u32 = 0x0800_0007;
    let mut fsp = spid.to_le_bytes().to_vec();
    fsp.extend_from_slice(&FSP_FLAGS.to_le_bytes());
    let mut secondary = vec![
        (0x80, story),
        (0x81, 0),
        (0x82, 0),
        (0x83, 0),
        (0x84, 0),
        (0x17F, 0x0030_0000),
        (0x1C0, REF0),
        (0x1C2, REF7),
        (0x1CB, 0x6338),
        (0x1FF, 0x0008_0000),
        (0x201, 0x0800_0004),
    ];
    secondary.extend((0x384..=0x387).map(|id| (id, 0x8EE0)));
    let mut tertiary = vec![(0x17F, 0x0380_0000), (0x1FF, 0x0040_0000)];
    for base in [0x540u16, 0x580, 0x5C0, 0x600] {
        tertiary.extend([(base, REF0), (base + 2, REF7), (base + 0xB, 0)]);
    }
    // 0x540, 0x580, 0x5C0, 0x600 have an 0xB entry, 0x640 has none (spec 4E.1.4).
    tertiary.extend([(0x640, REF0), (0x642, REF7), (0x6FF, 0x0002_0002)]);
    tertiary.sort_by_key(|e| e.0);
    let mut table_ref = vec![0x0a, 0, 0, 0, 0x01, 0x68];
    table_ref.extend_from_slice(&table.to_le_bytes());
    let mut story_ref = vec![0x0a, 0, 0, 0, 0x01, 0x20];
    story_ref.extend_from_slice(&story.to_le_bytes());
    Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(vec![
            Rec::atom(FSP_FRAME, 0xF00A, fsp),
            props(3, 0xF00B, &secondary),
            props(3, 0xF122, &tertiary),
            Rec::atom(CLIENT_VI, 0xF010, client_anchor_bytes(anchor)),
            Rec::atom(CLIENT_VI, 0xF011, table_ref),
            Rec::atom(CLIENT_VI, 0xF00D, story_ref),
        ]),
    }
}

/// Raise the drawing group's `maxspid` (first u32 of the 0xF006 header) to at least `min`.
fn raise_max_spid(head: &mut Rec, min: u32) -> Result<()> {
    let f6 = head
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF006)
        .ok_or_else(|| WriteError::Table("no 0xF006".into()))?;
    let d = f6.data_mut()?;
    let cur = rd(d, 0)?;
    put(d, 0, cur.max(min))
}

fn dgid_of(drawing: &Rec) -> u16 {
    drawing.find(0xF008).map_or(0, |r| r.vi >> 4)
}

/// The slot of the registry object of kind `kind` whose parent is the story root.
fn registry_slot(cts: &Cts, root: usize, kind: u64) -> Option<usize> {
    (0..cts.entries.len()).find(|&i| cts.kind(i) == Some(kind) && cts.parent(i).ok() == Some(root))
}

/// Insert a new table with the given frame on page `page_index` (0-based). Returns the new
/// file and the table's `Contents` object number (`Shape::object_number`).
pub fn add_table(input: &[u8], page_index: usize, t: &NewTable) -> Result<(Vec<u8>, u32)> {
    let cells_total = u64::from(t.rows) * u64::from(t.columns);
    if t.rows == 0 || t.columns == 0 || cells_total > u64::from(MAX_CELLS) {
        return fail(format!(
            "a table needs 1 to {MAX_CELLS} cells, {} rows x {} columns given",
            t.rows, t.columns
        ));
    }
    let cell_count = cells_total as usize;
    if t.cells.len() > cell_count {
        return fail(format!(
            "{} cell texts for {cell_count} cells",
            t.cells.len()
        ));
    }
    let cols = grid(t.width_emu, t.columns);
    let rows = grid(t.height_emu, t.rows);
    if cols.iter().any(|c| c.1 < MIN_COLUMN_EMU) || rows.iter().any(|r| r.1 < MIN_ROW_EMU) {
        return fail(format!(
            "cells are too small (columns need {MIN_COLUMN_EMU} EMU, rows {MIN_ROW_EMU} EMU)"
        ));
    }
    let mut texts: Vec<String> = t.cells.to_vec();
    texts.resize(cell_count, String::new());
    add_table_tracks(input, page_index, t.left_emu, t.top_emu, &cols, &rows, &texts, false)
}

/// Insert a table whose columns and rows have the given `(running edge, size)` tracks and
/// whose cell texts (row-major, one per cell) are `texts`. `edited` sets table item 3, which
/// Publisher writes once a table is no longer the uniform grid it was created as (4E.1.1).
#[allow(clippy::too_many_arguments)]
fn add_table_tracks(
    input: &[u8],
    page_index: usize,
    left_emu: i64,
    top_emu: i64,
    cols: &[(u32, u32)],
    rows: &[(u32, u32)],
    texts: &[String],
    edited: bool,
) -> Result<(Vec<u8>, u32)> {
    let t = NewTable {
        left_emu,
        top_emu,
        width_emu: cols.last().map_or(0, |c| c.0),
        height_emu: rows.last().map_or(0, |r| r.0),
        rows: rows.len() as u32,
        columns: cols.len() as u32,
        cells: &[],
    };
    let cell_count = rows.len() * cols.len();
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .surface(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let (cx, cy) = (
        f64::from(doc.page_width_emu) / 2.0,
        f64::from(doc.page_height_emu) / 2.0,
    );
    let side = |v: f64| {
        let r = v.round();
        i32::try_from(r as i64).map_err(|_| WriteError::OutOfRange(r as i64))
    };
    let (l, tp) = (t.left_emu as f64, t.top_emu as f64);
    let anchor = [
        side(l - cx)?,
        side(tp - cy)?,
        side(l + f64::from(t.width_emu) - cx)?,
        side(tp + f64::from(t.height_emu) - cy)?,
    ];

    // ---- Quill: the story, its TCD chunk and one MCLD frame per cell.
    let mut frames = Vec::with_capacity(cell_count);
    for r in 0..t.rows as usize {
        for c in 0..t.columns as usize {
            let (x0, w) = (cols[c].0 - cols[c].1, cols[c].1);
            let (y0, h) = (rows[r].0 - rows[r].1, rows[r].1);
            let pt = |emu: f64| emu / EMU_PER_POINT;
            let (x, y) = (pt(l + f64::from(x0)), pt(tp + f64::from(y0)));
            let (wp, hp) = (pt(f64::from(w)), pt(f64::from(h)));
            frames.push(NewFrame {
                cache: Some(layout_cache(x, y, wp, hp)),
                left_pt: x,
                top_pt: y,
                width_pt: wp,
                height_pt: hp,
                width_emu: w,
                height_emu: h,
            });
        }
    }
    let streams = Streams::from_cfb(input)?;
    let (quill, story) = quill::add_table_story(&streams.quill, texts, &frames)?;

    // ---- Escher: one rectangle per cell, one frame in the page drawing.
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let page_objs: HashSet<u32> = page.shapes.iter().map(|s| s.object_number).collect();
    let page_di = page_drawing(&esc, &page_objs)?;
    let cell_di = esc
        .drawings
        .iter()
        .position(|(_, d)| dgid_of(d) == CELL_DRAWING)
        .ok_or_else(|| WriteError::Table("the file has no cell drawing (drawing id 3)".into()))?;
    if cell_di == page_di {
        return fail("the page drawing is the cell drawing");
    }

    let mut cts = Cts::parse(&streams.contents)?;
    ensure_font_table(&mut cts)?;
    let font = cts.tail()?.0;
    let (table_id, cells_id) = (font as u32, font as u32 + 2);
    let page_obj = page.object_number as usize;
    if cts.kind(page_obj) != Some(KIND_PAGE) {
        return fail("the page has no page object in Contents");
    }

    {
        let drawing = &mut esc.drawings[cell_di].1;
        let first = spid_of(drawing)? + 1;
        let last = first + cell_count as u32 - 1;
        let group = drawing
            .kids_mut()?
            .iter_mut()
            .find(|r| r.ty == 0xF003)
            .ok_or_else(|| WriteError::Table("the cell drawing has no group".into()))?;
        for k in 0..cell_count as u32 {
            group
                .kids_mut()?
                .push(cell_shape(first + k, table_id, k));
        }
        drawing_counters(drawing, cell_count as i32, Some(last))?;
        group_counters(&mut esc.head, CELL_DRAWING, cell_count as i32, cell_count as i32)?;
        raise_max_spid(&mut esc.head, last + 1)?;
    }
    {
        let drawing = &mut esc.drawings[page_di].1;
        let dgid = dgid_of(drawing);
        let spid = spid_of(drawing)? + 1;
        let group = drawing
            .kids_mut()?
            .iter_mut()
            .find(|r| r.ty == 0xF003)
            .ok_or_else(|| WriteError::Table("the drawing has no page group".into()))?;
        group
            .kids_mut()?
            .push(frame_shape(spid, story, table_id, anchor));
        drawing_counters(drawing, 1, Some(spid))?;
        group_counters(&mut esc.head, dgid, 1, 1)?;
    }

    // ---- Contents: the story registry (284 only), then the three slots in front of the font table.
    let root = cts
        .slot_of_kind(KIND_STORY_ROOT)
        .ok_or_else(|| WriteError::Table("the story root object (kind 91) is missing".into()))?;
    let info = registry_slot(&cts, root, KIND_STORY_INFO)
        .ok_or_else(|| WriteError::Table("the story registry (kind 101) is missing".into()))?;
    ensure_registry(&mut cts, info)?;
    append_record(&mut cts, info, &|last: &[u8], ordinal: usize| {
        let its = items_of(last, 2)?;
        let mut b = vec![0x01, 0x20];
        b.extend_from_slice(&story.to_le_bytes());
        b.extend_from_slice(&[0x03, 0x10, 0, 0]);
        b.extend_from_slice(&[0x07, 0x20]);
        b.extend_from_slice(&(ordinal as u32).to_le_bytes());
        for want in [8u16, 9] {
            match its.iter().find(|i| i.0 == want) {
                Some(&(_, h, _, e)) => b.extend_from_slice(&last[h..e]),
                None => {
                    b.extend_from_slice(&pub_format::items::header(want, 11));
                    b.extend_from_slice(&REGISTRY_CONSTANT.to_le_bytes());
                }
            }
        }
        Ok(b)
    })?;

    let mut tb = Blk::new().flag(2, true);
    if edited {
        tb = tb.flag(3, true);
    }
    let table_body = tb
        .c2(4, SESSION_VALUE)
        .u64(12, 0)
        .u64(13, 0)
        .u32(39, story)
        .flag(42, true)
        .u32(102, t.rows)
        .u32(103, t.columns)
        .u32(104, t.width_emu)
        .u32(105, t.height_emu)
        .own_ref(107, cells_id)
        .list(109, grid_list(cols, rows))
        .u32(112, 0xFFFF_FFFD)
        .u32(183, 0)
        .encode();
    let records: Vec<Vec<u8>> = (0..t.rows)
        .flat_map(|r| (0..t.columns).map(move |c| cell_record(r, c)))
        .collect();
    let cells_body = Blk::new()
        .u16(1, cell_count as u16)
        .list20(2, records)
        .encode();
    let (_, at) = cts.tail()?;
    let mut both = table_body.clone();
    both.extend_from_slice(&cells_body);
    cts.blocks.splice(at..at, both.iter().copied());
    cts.shift(at, true, both.len() as isize)?;
    cts.insert_entries(
        font,
        vec![
            index_entry(KIND_TABLE as u16, page_obj as u32, 258, false, 22),
            EMPTY_SLOT.to_vec(),
            index_entry(KIND_CELLS as u16, table_id, 258, true, 1),
        ],
    )?;
    cts.set_int(table_id as usize, 4, at)?;
    cts.set_int(cells_id as usize, 4, at + table_body.len())?;
    let mut kids = cts.child_ids(page_obj)?;
    kids.push(table_id);
    cts.set_child_ids(page_obj, &kids)?;

    let out = install(input, &cts.finish()?, &esc.to_bytes())?;
    Ok((install_quill(&out, &quill)?, table_id))
}

/// The ids in a registry record list: the record whose item 1 equals `story`, as
/// `(start, end)` of the `00 88 <block>` element.
fn registry_record(cts: &Cts, slot: usize, story: u32) -> Result<Option<(usize, usize)>> {
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let Some(&(_, _, lp, le)) = items.iter().find(|i| i.0 == 2) else {
        return Ok(None);
    };
    let mut pos = lp + 4;
    while pos < le {
        let len = rd(&cts.blocks, pos + 2)? as usize;
        if cts.blocks[pos + 1] >> 3 != 17 || pos + 2 + len > le {
            return fail("unexpected registry record");
        }
        let its = items_of(&cts.blocks, pos + 2)?;
        if its
            .iter()
            .find(|i| i.0 == 1)
            .map(|&(_, _, p, _)| rd(&cts.blocks, p))
            .transpose()?
            == Some(story)
        {
            return Ok(Some((pos, pos + 2 + len)));
        }
        pos += 2 + len;
    }
    Ok(None)
}

/// Remove the record of `story` from registry `slot`; an emptied registry goes back to its
/// pristine form (no items, no item 11 in its index entry) like Publisher's after a delete.
fn remove_registry_record(cts: &mut Cts, slot: usize, story: u32) -> Result<()> {
    let Some((start, end)) = registry_record(cts, slot, story)? else {
        return Ok(());
    };
    // The records behind it move up: item 7 is the record's ordinal (spec 4L.2); Publisher
    // refuses a file whose ordinals have a gap.
    let mut pos = end;
    let off0 = cts.offset(slot)?;
    let list_end = items_of(&cts.blocks, off0)?
        .iter()
        .find(|i| i.0 == 2)
        .map(|i| i.3)
        .ok_or_else(|| WriteError::Table("registry without list".into()))?;
    while pos < list_end {
        let len = rd(&cts.blocks, pos + 2)? as usize;
        if let Some(&(_, _, p, _)) = items_of(&cts.blocks, pos + 2)?.iter().find(|i| i.0 == 7) {
            let v = rd(&cts.blocks, p)?;
            put(&mut cts.blocks, p, v.saturating_sub(1))?;
        }
        pos += 2 + len;
    }
    let off = cts.offset(slot)?;
    let items = items_of(&cts.blocks, off)?;
    let find = |id: u16| items.iter().find(|i| i.0 == id).copied();
    let (Some(count), Some(list)) = (find(1), find(2)) else {
        return fail("registry without count or list");
    };
    let remaining = rd(&cts.blocks, count.2)?.saturating_sub(1);
    let block_len = rd(&cts.blocks, off)? as usize;
    if remaining == 0 {
        let (from, to) = (count.1, list.3);
        cts.splice(from, to - from, &[])?;
        put(&mut cts.blocks, off, (block_len - (to - from)) as u32)?;
        let e = &mut cts.entries[slot];
        if e.ends_with(&[0x0b, 0x18, 1, 0]) {
            e.truncate(e.len() - 4);
            let n = (e.len() - 2) as u32;
            e[2..6].copy_from_slice(&n.to_le_bytes());
        }
    } else {
        put(&mut cts.blocks, count.2, remaining)?;
        let list_len = rd(&cts.blocks, list.2)? as usize - (end - start);
        put(&mut cts.blocks, list.2, list_len as u32)?;
        cts.splice(start, end - start, &[])?;
        put(&mut cts.blocks, off, (block_len - (end - start)) as u32)?;
    }
    Ok(())
}

/// The object number a cell shape (any Escher shape container) points at: the 0xF010 client
/// record of a cell rectangle holds an item 2 = the table's object number (spec 4E.1.4).
fn cell_owner(shape: &Rec) -> Option<u32> {
    if shape.ty != 0xF004 {
        return None;
    }
    let Body::Atom(d) = &shape.find(0xF010)?.body else {
        return None;
    };
    if d.len() < 10 {
        return None;
    }
    find_int(&decode_block(d, 0).ok()?, 2).map(|v| v as u32)
}

/// Publisher removes the font table (kind 108) and the story root's reference to it (item 4)
/// together with the last story: a file that lost all its text is the blank's again
/// (`table-deleted` equals `table-empty`, spec 4E.1.5).
fn drop_font_table(cts: &mut Cts) -> Result<()> {
    let Some(font) = cts.font_slot_any() else {
        return Ok(());
    };
    let root = cts.parent(font)?;
    let off = cts.offset(root)?;
    let items = items_of(&cts.blocks, off)?;
    if let Some(&(_, h, p, e)) = items.iter().find(|i| i.0 == 4) {
        if e - p == 4 && rd(&cts.blocks, p)? as usize == font {
            let before = rd(&cts.blocks, off)? as usize;
            cts.splice(h, e - h, &[])?;
            put(&mut cts.blocks, off, (before - (e - h)) as u32)?;
        }
    }
    remove_objects(cts, &[font])
}

/// Delete table `object_id` (a `Shape::object_number`): its Contents objects, its cell
/// rectangles and frame, its registry record and its story. Publisher keeps the shape-id
/// and story-number high-water marks, and so do we (spec 4E.1.5).
pub fn delete_table(input: &[u8], object_id: u32) -> Result<Vec<u8>> {
    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let t = object_id as usize;
    if cts.kind(t) != Some(KIND_TABLE) {
        return fail(format!("object {object_id} is not a table"));
    }
    let page = cts.parent(t)?;
    if cts.kind(page) != Some(KIND_PAGE) {
        return fail("only a table directly on a page can be deleted");
    }
    let off = cts.offset(t)?;
    let items = items_of(&cts.blocks, off)?;
    let item = |id: u16| {
        items
            .iter()
            .find(|i| i.0 == id)
            .map(|&(_, _, p, _)| rd(&cts.blocks, p))
            .transpose()
    };
    let story = item(39)?.ok_or_else(|| WriteError::Table("the table has no story number".into()))?;
    let cells = item(107)?.map(|v| v as usize);

    // ---- Escher: cell rectangles and the frame go, the counters drop, the high marks stay.
    let mut removed_any = false;
    for (_, d) in esc.drawings.iter_mut() {
        let dgid = dgid_of(d);
        let Some(group) = d.kids_mut()?.iter_mut().find(|r| r.ty == 0xF003) else {
            continue;
        };
        let kids = group.kids_mut()?;
        let before = kids.len();
        kids.retain(|k| {
            k.ty != 0xF004 || (k.object() != Some(object_id) && cell_owner(k) != Some(object_id))
        });
        let n = (before - kids.len()) as i32;
        if n > 0 {
            removed_any = true;
            drawing_counters(d, -n, None)?;
            group_counters(&mut esc.head, dgid, -n, 0)?;
        }
    }
    if !removed_any {
        return fail("the table has no shapes in the drawings");
    }

    // ---- Contents.
    drop_overlap_refs(&mut cts, object_id)?;
    let mut ids = cts.child_ids(page)?;
    ids.retain(|&x| x != object_id);
    {
        let off = cts.offset(page)?;
        let items = items_of(&cts.blocks, off)?;
        let (a, b) = (
            items.iter().find(|i| i.0 == 1).copied(),
            items.iter().find(|i| i.0 == 2).copied(),
        );
        match (a, b) {
            // An empty list removes items 1 and 2 altogether (the page is then the blank's).
            (Some(a), Some(b)) if ids.is_empty() && a.3 == b.1 => {
                let before = rd(&cts.blocks, off)? as usize;
                cts.splice(a.1, b.3 - a.1, &[])?;
                put(&mut cts.blocks, off, (before - (b.3 - a.1)) as u32)?;
            }
            _ => cts.set_child_ids(page, &ids)?,
        }
    }
    if let Some(root) = cts.slot_of_kind(KIND_STORY_ROOT) {
        if let Some(info) = registry_slot(&cts, root, KIND_STORY_INFO) {
            remove_registry_record(&mut cts, info, story)?;
        }
    }
    let mut gone = vec![t];
    gone.extend(cells.filter(|&c| cts.kind(c) == Some(KIND_CELLS)));
    remove_objects(&mut cts, &gone)?;

    let (quill, stories_left) = quill::remove_story(&streams.quill, story)?;
    if stories_left == 0 {
        drop_font_table(&mut cts)?;
    }
    let out = install(input, &cts.finish()?, &esc.to_bytes())?;
    install_quill(&out, &quill)
}

// ------------------------------------------------------------- rows and columns (4E.1.7a)

/// A table read back for editing: track sizes (EMU) and the row-major cell texts.
struct Layout {
    page: usize,
    left_emu: i64,
    top_emu: i64,
    widths: Vec<u32>,
    heights: Vec<u32>,
    texts: Vec<String>,
    /// Per cell (row-major, parallel to `texts`), the paragraphs with their formatting as read
    /// from the file; carried through the rebuild (see [`restore_format`]).
    paras: Vec<Vec<pub_format::Paragraph>>,
}

/// The formatting of a new, empty cell that sits next to `src`: the first run of its first
/// paragraph (empty text) and that paragraph's alignment and spacing. Publisher's own rule is
/// not in the spec; copying the neighbour is what the cell it copies the size of looks like.
fn blank_like(src: &[pub_format::Paragraph]) -> Vec<pub_format::Paragraph> {
    let Some(p) = src.first() else {
        return Vec::new();
    };
    let Some(r) = p.runs.first() else {
        return Vec::new();
    };
    let mut run = r.clone();
    run.text = "\r".into();
    vec![pub_format::Paragraph {
        runs: vec![run],
        alignment: p.alignment,
        spacing: p.spacing,
        list: None,
        indents: Default::default(),
        continued: false,
    }]
}

impl Layout {
    fn rows(&self) -> usize {
        self.heights.len()
    }
    fn columns(&self) -> usize {
        self.widths.len()
    }
}

fn read_layout(input: &[u8], table_id: u32) -> Result<Layout> {
    let doc = pub_format::read_bytes(input)?;
    let found = doc
        .pages
        .iter()
        .enumerate()
        .map(|(pi, p)| (pi, &p.shapes))
        .chain(
            doc.masters
                .iter()
                .enumerate()
                .map(|(mi, m)| (pub_format::master_surface(mi), &m.shapes)),
        )
        .find_map(|(pi, shapes)| {
            shapes
                .iter()
                .find(|s| s.object_number == table_id && s.table.is_some())
                .map(|s| (pi, s))
        });
    let Some((page, shape)) = found else {
        return fail(format!("object {table_id} is not a table on a page"));
    };
    let t = shape.table.as_ref().expect("filtered on table");
    let (r, c) = (t.rows as usize, t.columns as usize);
    if r == 0
        || c == 0
        || t.column_widths_emu.len() != c
        || t.row_heights_emu.len() != r
        || t.cells.len() != r * c
    {
        return fail("the table's grid or cells could not be read");
    }
    if t.cell_props.iter().any(|p| p.first_row != p.last_row || p.first_column != p.last_column) {
        return fail("tables with merged cells cannot be edited");
    }
    Ok(Layout {
        page,
        left_emu: i64::from(shape.left_emu) + i64::from(doc.page_width_emu) / 2,
        top_emu: i64::from(shape.top_emu) + i64::from(doc.page_height_emu) / 2,
        widths: t.column_widths_emu.clone(),
        heights: t.row_heights_emu.clone(),
        texts: t.cells.clone(),
        paras: if t.cell_paragraphs.len() == r * c {
            t.cell_paragraphs.clone()
        } else {
            vec![Vec::new(); r * c]
        },
    })
}

fn utf16_len(p: &pub_format::Paragraph) -> usize {
    p.runs.iter().map(|r| r.text.encode_utf16().count()).sum()
}

/// What `set_story_format` can write of a run, as a comparable key. A scheme colour stays a
/// scheme index (it is not resolved, so it differs from the same literal colour).
fn run_key(r: &pub_format::TextRun) -> impl PartialEq {
    (
        r.bold,
        r.italic,
        r.underline,
        r.script,
        (r.size_pt * 100.0).round() as i64,
        r.font.clone(),
        r.color.filter(|c| *c != pub_format::Color::Rgb(0, 0, 0)),
    )
}

/// Put the carried cell formatting (`l.paras`) onto the rebuilt table `object`: every cell
/// paragraph of the new story gets the character runs (scheme colours kept as scheme
/// colours), alignment, spacing, list and indents of the source
/// paragraph at the same index of the same cell (the last one when the cell has more).
/// Surviving cells have the same text, so their runs map one to one; a paragraph whose length
/// changed takes the source's first run for all of it.
fn restore_format(bytes: Vec<u8>, object: u32, l: &Layout) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(&bytes)?;
    let shape = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .chain(doc.masters.iter().flat_map(|m| &m.shapes))
        .find(|s| s.object_number == object && s.table.is_some());
    let Some((shape, story)) = shape.and_then(|s| Some((s, s.story?))) else {
        return Ok(bytes);
    };
    let t = shape.table.as_ref().expect("filtered on table");
    let Some(story_index) = doc.story_numbers().iter().position(|&n| n == story) else {
        return Ok(bytes);
    };
    if t.cell_paragraphs.len() != l.paras.len() {
        return Ok(bytes);
    }
    let rgb = |r: &pub_format::TextRun| match r.color {
        Some(pub_format::Color::Rgb(r, g, b)) => Some([r, g, b]),
        _ => None,
    };
    let scheme_index = |r: &pub_format::TextRun| match r.color {
        Some(pub_format::Color::Scheme(i)) => Some(i),
        _ => None,
    };
    let char_format = |r: &pub_format::TextRun| CharFormat {
        bold: r.bold,
        italic: r.italic,
        underline: r.underline,
        script: r.script,
        size_pt: r.size_pt,
        font: r.font.clone(),
        color: rgb(r),
        scheme_color: scheme_index(r),
    };
    let mut chars = Vec::new();
    let mut paras = Vec::new();
    let mut lists = Vec::new();
    let mut differs = false;
    let mut lists_differ = false;
    let mut pos = 0usize;
    for (new_cell, old_cell) in t.cell_paragraphs.iter().zip(&l.paras) {
        for (j, np) in new_cell.iter().enumerate() {
            let len = utf16_len(np);
            let Some(sp) = old_cell.get(j).or(old_cell.last()) else {
                pos += len;
                continue;
            };
            let same_len = utf16_len(sp) == len;
            let new_runs = np.runs.iter().filter(|r| !r.text.is_empty());
            if np.alignment != sp.alignment
                || np.spacing != sp.spacing
                || !same_len
                || sp.runs.iter().filter(|r| !r.text.is_empty()).count() != new_runs.clone().count()
                || sp
                    .runs
                    .iter()
                    .filter(|r| !r.text.is_empty())
                    .zip(new_runs)
                    .any(|(a, b)| run_key(a) != run_key(b))
            {
                differs = true;
            }
            if same_len {
                let mut at = pos;
                for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
                    let n = r.text.encode_utf16().count();
                    chars.push(CharSpan { start: at, end: at + n, format: char_format(r) });
                    at += n;
                }
            } else if let Some(r) = sp.runs.first() {
                chars.push(CharSpan { start: pos, end: pos + len, format: char_format(r) });
            }
            if np.list != sp.list || np.indents != sp.indents {
                lists_differ = true;
            }
            lists.push(ListSpan {
                start: pos,
                end: pos + len,
                list: sp.list.clone(),
                indents: sp.indents,
            });
            paras.push(ParaSpan {
                start: pos,
                end: pos + len,
                align: match sp.alignment {
                    pub_format::Alignment::Left => ParaAlign::Left,
                    pub_format::Alignment::Center => ParaAlign::Center,
                    pub_format::Alignment::Right => ParaAlign::Right,
                    pub_format::Alignment::Justify => ParaAlign::Justify,
                },
                spacing: (sp.spacing != pub_format::ParaSpacing::default()).then(|| {
                    ParaSpacing {
                        space_before_pt: sp.spacing.space_before_pt,
                        space_after_pt: sp.spacing.space_after_pt,
                        line: match sp.spacing.line {
                            pub_format::LineSpacing::Multiple(m) => Some(LineRule::Multiple(m)),
                            pub_format::LineSpacing::Exactly(pt) => Some(LineRule::Exactly(pt)),
                            pub_format::LineSpacing::Unknown(_) => None,
                        },
                    }
                }),
            });
            pos += len;
        }
    }
    let bytes = if differs {
        set_story_format(&bytes, story_index, &chars, &paras)?
    } else {
        bytes
    };
    if lists_differ {
        set_story_lists(&bytes, story_index, &lists)
    } else {
        Ok(bytes)
    }
}

fn tracks(sizes: &[u32]) -> Vec<(u32, u32)> {
    let mut edge = 0u32;
    sizes
        .iter()
        .map(|&s| {
            edge += s;
            (edge, s)
        })
        .collect()
}

/// Replace the table by its edited layout: delete it and add a clean one at the same place
/// (spec 4E.1.7a). Returns the new file and the new table's object number.
fn rebuild(input: &[u8], table_id: u32, l: &Layout) -> Result<(Vec<u8>, u32)> {
    if l.rows() as u64 * l.columns() as u64 > u64::from(MAX_CELLS) {
        return fail(format!("a table has at most {MAX_CELLS} cells"));
    }
    let without = delete_table(input, table_id)?;
    let (bytes, object) = add_table_tracks(
        &without,
        l.page,
        l.left_emu,
        l.top_emu,
        &tracks(&l.widths),
        &tracks(&l.heights),
        &l.texts,
        true,
    )?;
    Ok((restore_format(bytes, object, l)?, object))
}

/// True when `sizes` are the even split of their total that `AddTable` makes (so the table
/// has not been edited as far as Publisher's item 3 is concerned).
fn is_uniform(sizes: &[u32]) -> bool {
    let total: u32 = sizes.iter().sum();
    grid(total, sizes.len() as u32)
        .iter()
        .map(|g| g.1)
        .eq(sizes.iter().copied())
}

fn check_tracks(widths: &[u32], heights: &[u32], texts: &[String]) -> Result<()> {
    let cells = widths.len() as u64 * heights.len() as u64;
    if widths.is_empty() || heights.is_empty() || cells > u64::from(MAX_CELLS) {
        return fail(format!(
            "a table needs 1 to {MAX_CELLS} cells, {} rows x {} columns given",
            heights.len(),
            widths.len()
        ));
    }
    if texts.len() != cells as usize {
        return fail(format!("{} cell texts for {cells} cells", texts.len()));
    }
    if widths.iter().any(|&w| w < MIN_COLUMN_EMU) || heights.iter().any(|&h| h < MIN_ROW_EMU) {
        return fail(format!(
            "cells are too small (columns need {MIN_COLUMN_EMU} EMU, rows {MIN_ROW_EMU} EMU)"
        ));
    }
    Ok(())
}

/// Insert a table whose columns and rows have the given sizes (EMU, any mix) at `left_emu`,
/// `top_emu` from the page's top left corner; `texts` has one string per cell, row-major
/// (a newline separates the paragraphs of a cell). Table item 3 ("edited") is set when the sizes
/// are not the even split `AddTable` makes. Returns the file and the table's object number.
pub fn add_table_tracks_emu(
    input: &[u8],
    page_index: usize,
    left_emu: i64,
    top_emu: i64,
    widths: &[u32],
    heights: &[u32],
    texts: &[String],
) -> Result<(Vec<u8>, u32)> {
    check_tracks(widths, heights, texts)?;
    add_table_tracks(
        input,
        page_index,
        left_emu,
        top_emu,
        &tracks(widths),
        &tracks(heights),
        texts,
        !(is_uniform(widths) && is_uniform(heights)),
    )
}

/// Replace table `table_id` by one with the given sizes and texts at the given place on
/// the same page: the net effect of any series of row and column inserts and deletes, as
/// [`insert_table_rows`] and its siblings write them (delete the table, add a clean one,
/// spec 4E.1.7a). The caller puts the cell formatting back with `set_story_format`. The
/// new table is the last shape of its page. Returns the file and the new object number.
pub fn replace_table(
    input: &[u8],
    table_id: u32,
    left_emu: i64,
    top_emu: i64,
    widths: &[u32],
    heights: &[u32],
    texts: &[String],
) -> Result<(Vec<u8>, u32)> {
    check_tracks(widths, heights, texts)?;
    let page = read_layout(input, table_id)?.page;
    let without = delete_table(input, table_id)?;
    add_table_tracks(
        &without,
        page,
        left_emu,
        top_emu,
        &tracks(widths),
        &tracks(heights),
        texts,
        true,
    )
}

fn check_range(what: &str, first: u32, count: u32, have: usize, insert: bool) -> Result<(usize, usize)> {
    let (first, count) = (first as usize, count as usize);
    let max_first = if insert { have } else { have.saturating_sub(1) };
    if count == 0 || first > max_first || (!insert && first + count > have) {
        return fail(format!(
            "{what} {first} (+{count}) is outside the table's {have}"
        ));
    }
    Ok((first, count))
}

/// Insert `count` empty rows before row `before_row` (0-based; `rows` appends), like
/// `Rows.Add`: each copies the height of the row it goes before (the last row when
/// appending), the table grows by that, nothing else moves and the texts stay in their
/// cells. Returns the new file and the table's new object number.
pub fn insert_table_rows(
    input: &[u8],
    table_id: u32,
    before_row: u32,
    count: u32,
) -> Result<(Vec<u8>, u32)> {
    let mut l = read_layout(input, table_id)?;
    let (at, n) = check_range("row", before_row, count, l.rows(), true)?;
    let h = l.heights[at.min(l.rows() - 1)];
    let cols = l.columns();
    l.heights.splice(at..at, std::iter::repeat(h).take(n));
    l.texts.splice(at * cols..at * cols, std::iter::repeat(String::new()).take(n * cols));
    // New cells look like the cells of the row they were sized from (the one they go before).
    let src = at.min(l.rows() - n - 1);
    let blank: Vec<_> = (0..cols).map(|c| blank_like(&l.paras[src * cols + c])).collect();
    l.paras.splice(at * cols..at * cols, blank.iter().cloned().cycle().take(n * cols));
    rebuild(input, table_id, &l)
}

/// Delete `count` rows from `first_row` (0-based) with their text, like `Rows(n).Delete`.
/// Deleting every row is refused (Publisher raises `E_ACCESSDENIED`).
pub fn delete_table_rows(
    input: &[u8],
    table_id: u32,
    first_row: u32,
    count: u32,
) -> Result<(Vec<u8>, u32)> {
    let mut l = read_layout(input, table_id)?;
    let (at, n) = check_range("row", first_row, count, l.rows(), false)?;
    if n >= l.rows() {
        return fail("the last remaining row of a table cannot be deleted");
    }
    let cols = l.columns();
    l.heights.drain(at..at + n);
    l.texts.drain(at * cols..(at + n) * cols);
    l.paras.drain(at * cols..(at + n) * cols);
    rebuild(input, table_id, &l)
}

/// Insert `count` empty columns before column `before_column` (0-based; `columns` appends),
/// like `Columns.Add`; see [`insert_table_rows`].
pub fn insert_table_columns(
    input: &[u8],
    table_id: u32,
    before_column: u32,
    count: u32,
) -> Result<(Vec<u8>, u32)> {
    let mut l = read_layout(input, table_id)?;
    let (at, n) = check_range("column", before_column, count, l.columns(), true)?;
    let w = l.widths[at.min(l.columns() - 1)];
    let (rows, cols) = (l.rows(), l.columns());
    let mut texts = Vec::with_capacity(rows * (cols + n));
    let mut paras = Vec::with_capacity(rows * (cols + n));
    for r in 0..rows {
        let row = &l.texts[r * cols..(r + 1) * cols];
        texts.extend_from_slice(&row[..at]);
        texts.extend(std::iter::repeat(String::new()).take(n));
        texts.extend_from_slice(&row[at..]);
        let prow = &l.paras[r * cols..(r + 1) * cols];
        paras.extend_from_slice(&prow[..at]);
        let blank = blank_like(&prow[at.min(cols - 1)]);
        paras.extend(std::iter::repeat(blank).take(n));
        paras.extend_from_slice(&prow[at..]);
    }
    l.texts = texts;
    l.paras = paras;
    l.widths.splice(at..at, std::iter::repeat(w).take(n));
    rebuild(input, table_id, &l)
}

/// Delete `count` columns from `first_column` (0-based) with their text, like
/// `Columns(n).Delete`; the last remaining column is refused.
pub fn delete_table_columns(
    input: &[u8],
    table_id: u32,
    first_column: u32,
    count: u32,
) -> Result<(Vec<u8>, u32)> {
    let mut l = read_layout(input, table_id)?;
    let (at, n) = check_range("column", first_column, count, l.columns(), false)?;
    if n >= l.columns() {
        return fail("the last remaining column of a table cannot be deleted");
    }
    let (rows, cols) = (l.rows(), l.columns());
    let mut texts = Vec::with_capacity(rows * (cols - n));
    let mut paras = Vec::with_capacity(rows * (cols - n));
    for r in 0..rows {
        let row = &l.texts[r * cols..(r + 1) * cols];
        texts.extend_from_slice(&row[..at]);
        texts.extend_from_slice(&row[at + n..]);
        let prow = &l.paras[r * cols..(r + 1) * cols];
        paras.extend_from_slice(&prow[..at]);
        paras.extend_from_slice(&prow[at + n..]);
    }
    l.texts = texts;
    l.paras = paras;
    l.widths.drain(at..at + n);
    rebuild(input, table_id, &l)
}
