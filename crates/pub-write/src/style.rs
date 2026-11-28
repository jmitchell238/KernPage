//! Change the fill, outline, rotation or flip of an existing shape (spec 4D, 4H): the
//! properties are updated, inserted or removed in the shape's Escher property table
//! (0xF00B), which stays sorted with its count right. Nothing else in the file changes
//! except, when a rotation moves the shape across the quarter-turn boundaries, the stored
//! anchor (width and height swapped about the centre, spec 4H).

use super::group::{find_path, rec_at, Body, EscherDoc, Rec};
use super::{set_shape_frame, swapped_extent, Result, WriteError};
use pub_format::Streams;

const PROP_ROTATION: u16 = 0x0004;
const PROP_FILL_COLOR: u16 = 0x0181;
const PROP_FILL_BOOLS: u16 = 0x01BF;
const PROP_LINE_COLOR: u16 = 0x01C0;
const PROP_LINE_WIDTH: u16 = 0x01CB;
const PROP_LINE_BOOLS: u16 = 0x01FF;
/// `fFilled` / `fLine` and their "use" bits (the upper half of the boolean properties).
const FILLED: u32 = 0x10;
const FILLED_USE: u32 = 0x10_0000;
const LINE: u32 = 0x08;
const LINE_USE: u32 = 0x08_0000;
const FLIP_H: u32 = 0x40;
const FLIP_V: u32 = 0x80;

/// What to change; `None` leaves a property alone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShapeLook {
    /// `Some(None)` = no fill, `Some(Some(rgb))` = filled with this colour.
    pub fill: Option<Option<[u8; 3]>>,
    /// Turn the outline on or off.
    pub line_on: Option<bool>,
    pub line_color: Option<[u8; 3]>,
    pub line_weight_emu: Option<u32>,
    /// Clockwise degrees; 0 removes the property.
    pub rotation_deg: Option<f32>,
    pub flip_h: Option<bool>,
    pub flip_v: Option<bool>,
}

fn rgb_value(c: [u8; 3]) -> u32 {
    u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16
}

fn bad<T>(m: &str) -> Result<T> {
    Err(WriteError::Shape(m.into()))
}

/// Value of simple property `id` in a table.
pub(crate) fn get_prop(r: &Rec, id: u16) -> Option<u32> {
    let Body::Atom(d) = &r.body else { return None };
    (0..usize::from(r.vi >> 4)).find_map(|i| {
        let e = d.get(6 * i..6 * i + 6)?;
        (u16::from_le_bytes([e[0], e[1]]) & 0x3FFF == id)
            .then(|| u32::from_le_bytes([e[2], e[3], e[4], e[5]]))
    })
}

/// Set property `id` (replace or insert in id order).
pub(crate) fn set_prop(r: &mut Rec, id: u16, value: u32) -> Result<()> {
    let n = usize::from(r.vi >> 4);
    let d = r.data_mut()?;
    if d.len() < 6 * n {
        return bad("short property table");
    }
    let mut at = 6 * n;
    for i in 0..n {
        let opid = u16::from_le_bytes([d[6 * i], d[6 * i + 1]]) & 0x3FFF;
        if opid == id {
            if d[6 * i + 1] & 0x80 != 0 {
                return bad("the property is complex");
            }
            d[6 * i + 2..6 * i + 6].copy_from_slice(&value.to_le_bytes());
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

pub(crate) fn remove_prop(r: &mut Rec, id: u16) -> Result<()> {
    let n = usize::from(r.vi >> 4);
    let d = r.data_mut()?;
    if d.len() < 6 * n {
        return bad("short property table");
    }
    for i in 0..n {
        if u16::from_le_bytes([d[6 * i], d[6 * i + 1]]) & 0x3FFF == id {
            d.drain(6 * i..6 * i + 6);
            r.vi = (r.vi & 0xF) | (((n - 1) as u16) << 4);
            break;
        }
    }
    Ok(())
}

/// Apply `look` to shape `shape_index` of page `page_index`.
pub fn set_shape_style(
    input: &[u8],
    page_index: usize,
    shape_index: usize,
    look: &ShapeLook,
) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .surface(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let shape = page
        .shapes
        .get(shape_index)
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?;
    let obj = shape.object_number;
    let old_rotation = shape.rotation_deg;
    let old_frame = (
        shape.left_emu,
        shape.top_emu,
        shape.right_emu.wrapping_sub(shape.left_emu) as u32,
        shape.bottom_emu.wrapping_sub(shape.top_emu) as u32,
    );
    let text_dims = shape.width_emu.zip(shape.height_emu);
    let is_text_box = shape.kind == super::KIND_TEXT_BOX;

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
    let kids = sp.kids_mut()?;
    let table = kids
        .iter()
        .position(|r| r.ty == 0xF00B)
        .ok_or_else(|| WriteError::Shape("the shape has no property table".into()))?;

    if look.flip_h.is_some() || look.flip_v.is_some() {
        let fsp = kids
            .iter_mut()
            .find(|r| r.ty == 0xF00A)
            .ok_or_else(|| WriteError::Shape("the shape has no shape record".into()))?;
        let d = fsp.data_mut()?;
        if d.len() < 8 {
            return bad("short shape record");
        }
        let mut flags = u32::from_le_bytes([d[4], d[5], d[6], d[7]]);
        for (want, bit) in [(look.flip_h, FLIP_H), (look.flip_v, FLIP_V)] {
            match want {
                Some(true) => flags |= bit,
                Some(false) => flags &= !bit,
                None => {}
            }
        }
        d[4..8].copy_from_slice(&flags.to_le_bytes());
    }

    let t = &mut kids[table];
    if let Some(fill) = look.fill {
        let old = get_prop(t, PROP_FILL_BOOLS).unwrap_or(0);
        match fill {
            Some(c) => {
                set_prop(t, PROP_FILL_COLOR, rgb_value(c))?;
                set_prop(t, PROP_FILL_BOOLS, old | FILLED | FILLED_USE)?;
            }
            None => set_prop(t, PROP_FILL_BOOLS, (old | FILLED_USE) & !FILLED)?,
        }
    }
    if let Some(on) = look.line_on {
        let old = get_prop(t, PROP_LINE_BOOLS).unwrap_or(0);
        let v = if on {
            old | LINE | LINE_USE
        } else {
            (old | LINE_USE) & !LINE
        };
        set_prop(t, PROP_LINE_BOOLS, v)?;
    }
    if let Some(c) = look.line_color {
        set_prop(t, PROP_LINE_COLOR, rgb_value(c))?;
    }
    if let Some(w) = look.line_weight_emu {
        set_prop(t, PROP_LINE_WIDTH, w)?;
    }
    let mut new_rotation = old_rotation;
    if let Some(deg) = look.rotation_deg {
        let deg = deg.rem_euclid(360.0);
        new_rotation = deg;
        if deg == 0.0 {
            remove_prop(t, PROP_ROTATION)?;
        } else {
            set_prop(t, PROP_ROTATION, (f64::from(deg) * 65536.0).round() as i64 as u32)?;
        }
    }
    let out = super::group::install(input, &streams.contents, &esc.to_bytes())?;

    // The stored anchor follows the rotation's quarter-turn form (spec 4H).
    if swapped_extent(old_rotation) != swapped_extent(new_rotation) {
        let (w, h) = match (is_text_box, text_dims) {
            (true, Some(wh)) => wh,
            _ => (old_frame.2, old_frame.3),
        };
        return set_shape_frame(&out, page_index, shape_index, old_frame.0, old_frame.1, w, h);
    }
    Ok(out)
}
