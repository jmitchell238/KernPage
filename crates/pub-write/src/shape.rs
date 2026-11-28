//! Add a new rectangle, oval or line to a page (spec 4N): a freshly built Escher shape
//! container and a new `Contents` object with its index entry, page child-list entry and
//! the overlap bookkeeping on older text boxes. Quill and the story registries are untouched.

use super::group::{
    anchor_atom, drawing_counters, group_counters, install, items_of, rd, Body, Cts, EscherDoc,
    Rec, GROUP_ENTRY,
};
use super::textbox::{add_overlap, grow_block};
use super::{Result, WriteError, EMU_PER_POINT, KIND_TEXT_BOX};
use pub_format::Streams;
use std::collections::HashSet;

const KIND_PAGE: u64 = 67;
const KIND_LINE: u16 = 32;
const FSP_FLAGS: u32 = 0x0A00;
const FLIP_H: u32 = 0x40;
const FLIP_V: u32 = 0x80;
const INSET: u32 = 36_576;

/// What to add.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewShapeKind {
    Rectangle,
    Oval,
    /// A straight line inside the (normalised) box. Without flips it runs from the top-left
    /// to the bottom-right corner; `flip_v` makes it run from the bottom-left to the
    /// top-right, `flip_h` from the top-right to the bottom-left, both from bottom-right to
    /// top-left (spec 4N.1).
    Line {
        flip_h: bool,
        flip_v: bool,
    },
}

/// Fill and outline. `None` keeps Publisher's default (no fill; 0.75 pt scheme-colour line).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShapeStyle {
    pub fill: Option<[u8; 3]>,
    pub line: Option<[u8; 3]>,
    pub line_weight_pt: Option<f64>,
    /// No outline at all (the line-on bit of 0x01FF cleared).
    pub no_line: bool,
}

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Shape(m.into()))
}

fn rgb_value(c: [u8; 3]) -> u32 {
    u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16
}

pub(crate) fn props(vi_ver: u16, ty: u16, list: &[(u16, u32)]) -> Rec {
    let mut d = Vec::new();
    for (id, v) in list {
        d.extend_from_slice(&id.to_le_bytes());
        d.extend_from_slice(&v.to_le_bytes());
    }
    Rec::atom(((list.len() as u16) << 4) | vi_ver, ty, d)
}

/// Index of the drawing that holds a page's shapes: the one that has any of `page_objs`; for a
/// page without shapes the drawing of the document pages (drawing id 1, spec 6.4).
pub(crate) fn page_drawing(esc: &EscherDoc, page_objs: &HashSet<u32>) -> Result<usize> {
    let has = |d: &Rec| {
        d.kids().iter().filter(|r| r.ty == 0xF003).any(|g| {
            g.kids()
                .iter()
                .any(|k| k.object().is_some_and(|o| page_objs.contains(&o)))
        })
    };
    esc.drawings
        .iter()
        .position(|(_, d)| has(d))
        .or_else(|| {
            esc.drawings
                .iter()
                .position(|(_, d)| d.find(0xF008).is_some_and(|r| r.vi >> 4 == 1))
        })
        .ok_or_else(|| WriteError::Shape("the file has no drawing for the document pages".into()))
}

/// Add a rectangle, oval or line to page `page_index` (0-based). Position and size are in
/// points from the page's top-left corner. A line may have a zero width or height.
pub fn add_shape(
    input: &[u8],
    page_index: usize,
    kind: NewShapeKind,
    left_pt: f64,
    top_pt: f64,
    width_pt: f64,
    height_pt: f64,
    style: ShapeStyle,
) -> Result<Vec<u8>> {
    let is_line = matches!(kind, NewShapeKind::Line { .. });
    let finite = [left_pt, top_pt, width_pt, height_pt]
        .iter()
        .all(|v| v.is_finite());
    let sized = if is_line {
        width_pt >= 0.0 && height_pt >= 0.0
    } else {
        width_pt > 0.0 && height_pt > 0.0
    };
    if !finite || !sized {
        return fail("the frame must be finite with a positive size (a line may be flat)");
    }
    if style
        .line_weight_pt
        .is_some_and(|w| !w.is_finite() || w <= 0.0)
    {
        return fail("the line weight must be positive");
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
    let weight_emu = style.line_weight_pt.map(|w| to_u32(emu(w))).transpose()?;

    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let font = cts.tail()?.0;
    let new_id = font as u32;
    let page_obj = page.object_number as usize;
    if cts.kind(page_obj) != Some(KIND_PAGE) {
        return fail("the page has no page object in Contents");
    }

    // The drawing that holds this page's shapes.
    let page_objs: HashSet<u32> = page.shapes.iter().map(|s| s.object_number).collect();
    let di = page_drawing(&esc, &page_objs)?;
    let drawing = &mut esc.drawings[di].1;
    let dgid = drawing.find(0xF008).map_or(0, |r| r.vi >> 4);
    let spid = match drawing.find(0xF008).map(|r| &r.body) {
        Some(Body::Atom(d)) => rd(d, 4)? + 1,
        _ => return fail("the drawing has no 0xF008"),
    };

    // ---- Escher container, built from the spec 4N.1 tables.
    let (inst, flags) = match kind {
        NewShapeKind::Rectangle => (1u16, FSP_FLAGS),
        NewShapeKind::Oval => (3, FSP_FLAGS),
        NewShapeKind::Line { flip_h, flip_v } => (
            20,
            FSP_FLAGS | if flip_h { FLIP_H } else { 0 } | if flip_v { FLIP_V } else { 0 },
        ),
    };
    let mut fsp = spid.to_le_bytes().to_vec();
    fsp.extend_from_slice(&flags.to_le_bytes());
    let mut p: Vec<(u16, u32)> = vec![(0x81, INSET), (0x82, INSET), (0x83, INSET), (0x84, INSET)];
    if !is_line {
        p.push((0x181, style.fill.map_or(0x0800_0007, rgb_value)));
        p.push((0x183, 0x0800_0007));
        p.push((
            0x1BF,
            if style.fill.is_some() {
                0x0010_0010
            } else {
                0x0010_0000
            },
        ));
    } else {
        p.push((0x1BF, 0x0010_0000));
    }
    p.push((0x1C0, style.line.map_or(0x0800_0000, rgb_value)));
    p.push((0x1C2, 0x0800_0007));
    if let Some(w) = weight_emu {
        p.push((0x1CB, w));
    }
    p.extend([
        (0x1FF, if style.no_line { 0x0008_0000 } else { 0x0008_0008 }),
        (0x201, 0x0800_0004),
        (0x384, INSET),
        (0x385, INSET),
        (0x386, INSET),
        (0x387, INSET),
    ]);
    p.sort_by_key(|e| e.0);
    let mut t: Vec<(u16, u32)> = Vec::new();
    if !is_line {
        t.push((0x1FF, 0x0040_0040));
    }
    t.extend([
        (0x540, 0x0800_0000),
        (0x542, 0x0800_0007),
        (0x580, 0x0800_0000),
        (0x582, 0x0800_0007),
        (0x5C0, 0x0800_0000),
        (0x5C2, 0x0800_0007),
        (0x600, 0x0800_0000),
        (0x602, 0x0800_0007),
        (0x640, 0x0800_0000),
        (0x642, 0x0800_0007),
        (0x6FF, 0x0002_0002),
    ]);
    let mut cd = vec![0x0a, 0, 0, 0, 0x01, 0x68];
    cd.extend_from_slice(&new_id.to_le_bytes());
    let container = Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(vec![
            Rec::atom((inst << 4) | 2, 0xF00A, fsp),
            props(3, 0xF00B, &p),
            props(3, 0xF122, &t),
            anchor_atom(anchor),
            Rec::atom(0x018a, 0xF011, cd),
        ]),
    };
    drawing_counters(drawing, 1, Some(spid))?;
    let group = drawing
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF003)
        .ok_or_else(|| WriteError::Shape("the drawing has no page group".into()))?;
    group.kids_mut()?.push(container);
    group_counters(&mut esc.head, dgid, 1, 1)?;

    // ---- Contents: overlap bookkeeping on older text boxes (rectangle/oval 45/70/71, line 43).
    let [l, tp, r, b] = anchor;
    for s in &page.shapes {
        if s.kind == KIND_TEXT_BOX
            && s.chain_story.is_some()
            && s.group.is_none()
            && s.left_emu < r
            && s.right_emu > l
            && s.top_emu < b
            && s.bottom_emu > tp
        {
            if is_line {
                add_line_flag(&mut cts, s.object_number as usize)?;
            } else {
                add_overlap(&mut cts, s.object_number as usize, new_id)?;
            }
        }
    }

    // New object before the font table, its index entry, and the page's child list.
    let (_, at) = cts.tail()?;
    let mut body = vec![0u8; 4];
    body.extend_from_slice(&[0x02, 0x08, 0x04, 0x10]);
    body.extend_from_slice(&[if is_line { 0 } else { 3 }, 0x00]);
    body.extend_from_slice(&[0x0c, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&[0x0d, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    let tail: &[(u8, u32)] = if is_line {
        &[(0xb7, 0)]
    } else {
        &[(0x34, 0), (0xaa, width_emu), (0xab, height_emu), (0xb7, 0)]
    };
    for &(id, v) in tail {
        body.extend_from_slice(&[id, 0x20]);
        body.extend_from_slice(&v.to_le_bytes());
    }
    let n = body.len() as u32;
    body[..4].copy_from_slice(&n.to_le_bytes());
    cts.blocks.splice(at..at, body.iter().copied());
    cts.shift(at, true, body.len() as isize)?;
    let mut entry = GROUP_ENTRY.to_vec();
    entry[8] = if is_line { KIND_LINE as u8 } else { 1 };
    cts.insert_entries(font, vec![entry])?;
    cts.set_int(font, 4, at)?;
    cts.set_int(font, 5, page_obj)?;
    let mut kids = cts.child_ids(page_obj)?;
    kids.push(new_id);
    cts.set_child_ids(page_obj, &kids)?;

    install(input, &cts.finish()?, &esc.to_bytes())
}

/// Item 43 (flag) on an older text box a new line crosses (spec 4N.2).
fn add_line_flag(cts: &mut Cts, obj: usize) -> Result<()> {
    let off = cts.offset(obj)?;
    let items = items_of(&cts.blocks, off)?;
    if items.iter().any(|i| i.0 == 43) {
        return Ok(());
    }
    let at = items
        .iter()
        .find(|i| i.0 > 43)
        .map(|i| i.1)
        .ok_or_else(|| WriteError::Shape("text box body has no item after 43".into()))?;
    let ins = pub_format::items::header(43, 1);
    cts.blocks.splice(at..at, ins);
    cts.shift(at, false, ins.len() as isize)?;
    grow_block(cts, off, ins.len())
}
