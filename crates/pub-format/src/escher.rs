//! `Escher/EscherStm` walker (spec 4A) and picture store (spec 4C), using the public
//! [MS-ODRAW] record header. Only the payloads of client anchor (0xF010), client data
//! (0xF011) and client textbox (0xF00D) records are Publisher item blocks; every other
//! record is skipped by its header length (the property table 0xF00B is read only for
//! the picture index).

use crate::color::Color;
use crate::error::PubError;
use crate::items::{self, Item};
use serde::Serialize;

const DGG_CONTAINER: u16 = 0xF000;
const DG_CONTAINER: u16 = 0xF002;
const CLIENT_ANCHOR: u16 = 0xF010;
const CLIENT_DATA: u16 = 0xF011;
const CLIENT_TEXTBOX: u16 = 0xF00D;
const OPT: u16 = 0xF00B;
const TERTIARY_OPT: u16 = 0xF122;
const SHAPE_RECORD: u16 = 0xF00A;
/// Shape-group container, shape container, group coordinate box and child anchor
/// ([MS-ODRAW] record types, spec 4J).
const SPGR_CONTAINER: u16 = 0xF003;
const SP_CONTAINER: u16 = 0xF004;
const FSPGR: u16 = 0xF009;
const CHILD_ANCHOR: u16 = 0xF00F;
const BSTORE_ENTRY: u16 = 0xF007;
const MAX_DEPTH: usize = 32;
/// Shape property: 1-based index into the picture store (spec 4C).
const PROP_PICTURE_INDEX: u16 = 0x0104;
/// Picture type values from [MS-ODRAW].
const BLIP_JPEG: u8 = 5;
const BLIP_PNG: u8 = 6;
const BLIP_EMF: u8 = 2;
const BLIP_WMF: u8 = 3;
/// [MS-ODRAW] OfficeArtMetafileHeader: 34 bytes; compression byte at 32, 0 = deflate, 0xFE = none.
const METAFILE_HEADER_LEN: usize = 34;
const METAFILE_COMPRESSION_AT: usize = 32;
const METAFILE_DEFLATE: u8 = 0;
const METAFILE_STORED: u8 = 0xFE;
/// Largest decompressed metafile accepted.
const MAX_METAFILE_BYTES: usize = 64 << 20;
/// Fill colour, line colour, line width (EMU), fill style booleans, line style booleans
/// (spec 4D).
const PROP_FILL_COLOR: u16 = 0x0181;
const PROP_FILL_BOOLS: u16 = 0x01BF;
const PROP_LINE_COLOR: u16 = 0x01C0;
const PROP_LINE_WIDTH: u16 = 0x01CB;
const PROP_LINE_BOOLS: u16 = 0x01FF;
/// `fFilled` in 0x01BF and `fLine` in 0x01FF.
const BOOL_FILLED: u32 = 0x10;
const BOOL_LINE: u32 = 0x08;
/// Default line width when property 0x01CB is absent: 0.75 pt (spec 4D).
const DEFAULT_LINE_WIDTH_EMU: u32 = 9525;
/// Shape type values (the shape record's instance field, spec 4D).
const SHAPE_RECTANGLE: u16 = 1;
const SHAPE_ELLIPSE: u16 = 3;
const SHAPE_LINE: u16 = 20;
/// Rotation: 16.16 fixed-point degrees, clockwise (spec 4H).
const PROP_ROTATION: u16 = 0x0004;
/// Wrap distances left, top, right, bottom (spec 4K).
const PROP_WRAP_DIST: [u16; 4] = [0x0384, 0x0385, 0x0386, 0x0387];
/// Default wrap distance, EMU (spec 4K).
pub const DEFAULT_WRAP_DIST_EMU: u32 = 36_576;
/// Text-frame margins left, top, right, bottom (spec 4N.1: "text insets", 0x81..0x84).
const PROP_INSET: [u16; 4] = [0x0081, 0x0082, 0x0083, 0x0084];
/// Margin used when a shape has no inset property: 0.04 in, what Publisher writes.
pub const DEFAULT_INSET_EMU: u32 = 36_576;
/// Blip record types are 0xF018 + picture type.
const BLIP_RECORD_FIRST: u16 = 0xF018;
const BLIP_RECORD_LAST: u16 = 0xF117;
/// Bytes of the picture-store entry payload we read (through the delay offset).
const BSE_MIN_LEN: usize = 32;

/// The geometry of a drawn shape (spec 4D); other [MS-ODRAW] shape types keep their value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ShapeGeometry {
    Rectangle,
    Ellipse,
    Line,
    Other(u16),
}

impl ShapeGeometry {
    fn from_type(t: u16) -> Self {
        match t {
            SHAPE_RECTANGLE => ShapeGeometry::Rectangle,
            SHAPE_ELLIPSE => ShapeGeometry::Ellipse,
            SHAPE_LINE => ShapeGeometry::Line,
            other => ShapeGeometry::Other(other),
        }
    }
}

/// One picture-store entry (0xF007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlipEntry {
    pub blip_type: u8,
    pub size: u32,
    pub ref_count: u32,
    /// Offset of the picture record in `Escher/EscherDelayStm`.
    pub delay_offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PictureFormat {
    Png,
    Jpeg,
    /// Windows Metafile; `bytes` is the decompressed metafile.
    Wmf,
    /// Enhanced Metafile; `bytes` is the decompressed metafile.
    Emf,
    Other(u8),
}

/// An embedded picture: the original file bytes for PNG/JPEG, the decompressed metafile
/// (starting at its own header) for WMF/EMF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Picture {
    pub format: PictureFormat,
    /// Not serialized (would flood JSON output).
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

/// Everything read from `EscherStm`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Escher {
    pub shapes: Vec<EscherShape>,
    pub blips: Vec<BlipEntry>,
    /// Shapes with client data but neither a client anchor nor a usable child anchor;
    /// they are left out of `shapes` (spec 4J).
    pub anchorless: usize,
}

/// What the Escher stream says about one shape that has client data.
#[derive(Debug, Clone, PartialEq)]
pub struct EscherShape {
    /// Index of the drawing (in stream order) the shape was found in.
    pub drawing: usize,
    /// Anchor items 1..=4: left, top, right, bottom (0 where absent).
    pub anchor: [i32; 4],
    /// `Contents` object number from client data item 1.
    pub contents_object: u32,
    /// Story number from client textbox item 1.
    pub story: Option<u32>,
    /// 1-based picture store index from shape property 0x0104.
    pub picture_index: Option<u32>,
    /// Shape type from the shape record, if the shape has one.
    pub geometry: Option<ShapeGeometry>,
    /// Shape-record flags 0x40 (flip horizontally) and 0x80 (flip vertically).
    pub flip_h: bool,
    pub flip_v: bool,
    /// `None` when the shape is not filled (property 0x01BF).
    pub fill: Option<Color>,
    /// `None` when the line is off (property 0x01FF); width in EMU.
    pub line: Option<(Color, u32)>,
    /// Rotation in degrees clockwise, normalised to [0, 360) (spec 4H); 0 when absent.
    /// For a child of a rotated group this includes the group's rotation (spec 4J).
    pub rotation_deg: f32,
    /// `Contents` object number of the group shape this shape belongs to (spec 4J);
    /// `None` for shapes directly on the page.
    pub group: Option<u32>,
    /// Wrap distances left, top, right, bottom in EMU (spec 4K); default 36,576.
    pub wrap_dist_emu: [u32; 4],
    /// Text-frame margins left, top, right, bottom in EMU (properties 0x81..0x84);
    /// [`DEFAULT_INSET_EMU`] where absent.
    pub inset_emu: [u32; 4],
    /// Column count and gap in EMU from the tertiary option record (spec 4I: properties
    /// 0x8C, absent = 1, and 0x8D = 2 x gap); `None` when the shape has no such record.
    pub columns: Option<(u32, u32)>,
    /// Where a child of a group keeps its placement (spec 4J); `None` for client anchors.
    pub child: Option<ChildInfo>,
}

/// A grouped shape's child anchor (0xF00F) and the group it is mapped through (spec 4J).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChildInfo {
    /// Offset in `EscherStm` of the child anchor's 16 bytes (four little-endian i32:
    /// left, top, right, bottom in the group's coordinate box).
    pub offset: usize,
    /// The group's coordinate box (FSPGR).
    pub inner: [f64; 4],
    /// The group's unrotated rectangle on the page (client-anchor space).
    pub rect: [f64; 4],
    /// The group's total rotation in degrees clockwise.
    pub group_rotation: f32,
    /// The child's own rotation property in degrees.
    pub own_rotation: f32,
}

/// Property 0x0004 as degrees in [0, 360).
fn rotation(props: &[(u16, u32)]) -> f32 {
    let raw = props
        .iter()
        .find(|p| p.0 == PROP_ROTATION)
        .map_or(0, |p| p.1 as i32);
    (raw as f32 / 65536.0).rem_euclid(360.0)
}

struct Header {
    inst: u16,
    container: bool,
    ty: u16,
    len: usize,
}

fn header(data: &[u8], pos: usize) -> Result<Header, PubError> {
    let h = pos
        .checked_add(8)
        .and_then(|end| data.get(pos..end))
        .ok_or_else(|| PubError::format(pos, "truncated Escher record header"))?;
    Ok(Header {
        inst: u16::from_le_bytes([h[0], h[1]]) >> 4,
        container: h[0] & 0x0F == 0x0F,
        ty: u16::from_le_bytes([h[2], h[3]]),
        len: u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize,
    })
}

/// Walk the whole stream: one DggContainer, then repeated `[u32][DgContainer]`.
pub fn read(data: &[u8]) -> Result<Escher, PubError> {
    let first = header(data, 0)?;
    if first.ty != DGG_CONTAINER {
        return Err(PubError::format(
            0,
            "EscherStm does not start with a drawing group",
        ));
    }
    let mut pos = 8usize
        .checked_add(first.len)
        .filter(|&p| p <= data.len())
        .ok_or_else(|| PubError::format(0, "drawing group extends past the stream"))?;
    let mut blips = Vec::new();
    walk_dgg(data, 8, pos, 0, &mut blips)?;
    let mut shapes = Vec::new();
    let mut anchorless = 0;
    let mut drawing = 0;
    while pos < data.len() {
        pos += 4; // the u32 before each drawing (purpose unknown)
        if pos > data.len() {
            return Err(PubError::format(pos, "truncated drawing prefix"));
        }
        let h = header(data, pos)?;
        if h.ty != DG_CONTAINER || !h.container {
            return Err(PubError::format(
                pos,
                "expected a drawing container (0xF002)",
            ));
        }
        let end = (pos + 8)
            .checked_add(h.len)
            .filter(|&e| e <= data.len())
            .ok_or_else(|| PubError::format(pos, "drawing extends past the stream"))?;
        walk(
            data,
            pos + 8,
            end,
            drawing,
            0,
            Ctx::default(),
            &mut Sink {
                shapes: &mut shapes,
                anchorless: &mut anchorless,
            },
        )?;
        pos = end;
        drawing += 1;
    }
    Ok(Escher {
        shapes,
        blips,
        anchorless,
    })
}

/// Walk the drawing-group container collecting picture-store entries (0xF007).
fn walk_dgg(
    data: &[u8],
    start: usize,
    end: usize,
    depth: usize,
    out: &mut Vec<BlipEntry>,
) -> Result<(), PubError> {
    if depth > MAX_DEPTH {
        return Err(PubError::format(
            start,
            "Escher containers nested too deeply",
        ));
    }
    let mut pos = start;
    while pos < end {
        let h = header(data, pos)?;
        let body = pos + 8;
        let next = body
            .checked_add(h.len)
            .filter(|&n| n <= end)
            .ok_or_else(|| PubError::format(pos, "Escher record extends past its container"))?;
        if h.container {
            walk_dgg(data, body, next, depth + 1, out)?;
        } else if h.ty == BSTORE_ENTRY && h.len >= BSE_MIN_LEN {
            // Payload: type, type (mac), 16-byte uid, u16 tag, u32 size, u32 refs, u32 delay offset.
            out.push(BlipEntry {
                blip_type: data[body],
                size: u32_at(data, body + 20).unwrap_or(0),
                ref_count: u32_at(data, body + 24).unwrap_or(0),
                delay_offset: u32_at(data, body + 28).unwrap_or(0),
            });
        }
        pos = next;
    }
    Ok(())
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    let s = b.get(o..o.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Property `want` of a property table (0xF00B): the record's instance field holds the
/// entry count; entries are `u16 id (low 14 bits) + flags`, `u32 value`.
fn property(data: &[u8], body: usize, len: usize, count: usize, want: u16) -> Option<u32> {
    (0..count).find_map(|i| {
        let at = body.checked_add(i.checked_mul(6)?)?;
        if i * 6 + 6 > len {
            return None;
        }
        let id = u16::from_le_bytes([*data.get(at)?, *data.get(at + 1)?]) & 0x3FFF;
        if id == want {
            u32_at(data, at + 2)
        } else {
            None
        }
    })
}

/// All simple properties of a property table as `(id, value)` pairs. Complex-property
/// data (which follows the table) is not read.
fn properties(data: &[u8], body: usize, len: usize, count: usize) -> Vec<(u16, u32)> {
    (0..count)
        .map_while(|i| {
            let off = i.checked_mul(6)?;
            if off.checked_add(6)? > len {
                return None;
            }
            let at = body.checked_add(off)?;
            let id = u16::from_le_bytes([*data.get(at)?, *data.get(at + 1)?]) & 0x3FFF;
            Some((id, u32_at(data, at + 2)?))
        })
        .collect()
}

/// Columns and gap (EMU) from the tertiary record's properties (spec 4I): 0x8C = count
/// (absent = 1), 0x8D = 2 x gap. `None` when neither is present.
fn tertiary_columns(props: &[(u16, u32)]) -> Option<(u32, u32)> {
    let get = |id| props.iter().find(|p| p.0 == id).map(|p| p.1);
    let (n, gap) = (get(0x8C), get(0x8D));
    if n.is_none() && gap.is_none() {
        return None;
    }
    Some((n.unwrap_or(1).clamp(1, 64), gap.unwrap_or(0) / 2))
}

/// Fill and line of a shape from its properties (spec 4D).
///
/// UNCONFIRMED: a missing boolean property means "off"; a filled shape with no 0x0181
/// uses scheme colour 7 and a lined shape with no 0x01C0 uses scheme colour 0 (the
/// published defaults are white fill and black line); the line width defaults to 0.75 pt.
fn look(props: &[(u16, u32)]) -> (Option<Color>, Option<(Color, u32)>) {
    let get = |id| props.iter().find(|p| p.0 == id).map(|p| p.1);
    let on = |id, bit| get(id).is_some_and(|v| v & bit != 0);
    let fill = on(PROP_FILL_BOOLS, BOOL_FILLED)
        .then(|| get(PROP_FILL_COLOR).map_or(Color::Scheme(7), Color::from_u32));
    let line = on(PROP_LINE_BOOLS, BOOL_LINE).then(|| {
        (
            get(PROP_LINE_COLOR).map_or(Color::Scheme(0), Color::from_u32),
            get(PROP_LINE_WIDTH).unwrap_or(DEFAULT_LINE_WIDTH_EMU),
        )
    });
    (fill, line)
}

/// Read the picture of a store entry from the delay stream (spec 4C): a blip record
/// (8-byte header, 16-byte uid, 1 tag byte, then the image bytes). Returns `None` when
/// the offset or length leaves the stream or the record is not a blip.
pub fn picture(delay: &[u8], entry: &BlipEntry) -> Option<Picture> {
    let at = entry.delay_offset as usize;
    let h = header(delay, at).ok()?;
    if !(BLIP_RECORD_FIRST..=BLIP_RECORD_LAST).contains(&h.ty) || h.container {
        return None;
    }
    let payload = delay.get(at + 8..(at + 8).checked_add(h.len)?)?;
    // [MS-ODRAW]: some blip kinds (instance values below) carry a second 16-byte uid.
    let inst = u16::from_le_bytes([delay[at], delay[at + 1]]) >> 4;
    let uids = if matches!(
        inst,
        0x46B | 0x6E1 | 0x6E3 | 0x3D5 | 0x217 | 0x543 | 0x7A9 | 0x6E7
    ) {
        2
    } else {
        1
    };
    if matches!(entry.blip_type, BLIP_WMF | BLIP_EMF) {
        let format = if entry.blip_type == BLIP_WMF {
            PictureFormat::Wmf
        } else {
            PictureFormat::Emf
        };
        return metafile_bytes(payload, 16 * uids).map(|bytes| Picture { format, bytes });
    }
    let bytes = payload.get(16 * uids + 1..)?.to_vec();
    let format = match entry.blip_type {
        BLIP_PNG => PictureFormat::Png,
        BLIP_JPEG => PictureFormat::Jpeg,
        other => PictureFormat::Other(other),
    };
    Some(Picture { format, bytes })
}

/// The metafile of an OfficeArtMetafileBlip payload: after the uid(s) comes a 34-byte
/// header (size, bounds, size in EMU, saved size, compression, filter), then the data,
/// deflate-compressed (zlib) unless the compression byte is 0xFE.
fn metafile_bytes(payload: &[u8], uid_len: usize) -> Option<Vec<u8>> {
    let head = payload.get(uid_len..uid_len.checked_add(METAFILE_HEADER_LEN)?)?;
    let saved = u32::from_le_bytes([head[28], head[29], head[30], head[31]]) as usize;
    let data = &payload[uid_len + METAFILE_HEADER_LEN..];
    let data = &data[..if saved == 0 {
        data.len()
    } else {
        saved.min(data.len())
    }];
    match head[METAFILE_COMPRESSION_AT] {
        METAFILE_DEFLATE => {
            miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, MAX_METAFILE_BYTES).ok()
        }
        METAFILE_STORED => Some(data.to_vec()),
        _ => None,
    }
}

/// A shape group's coordinate system (spec 4J), in the same space as client anchors.
#[derive(Debug, Clone, Copy)]
struct Group {
    /// The group's own coordinate box (FSPGR): left, top, right, bottom.
    inner: [f64; 4],
    /// The group's unrotated rectangle on the page.
    rect: [f64; 4],
    /// The group's total rotation in degrees clockwise (own plus enclosing groups').
    rotation: f32,
}

/// The innermost enclosing group while walking.
#[derive(Debug, Clone, Copy, Default)]
struct Ctx {
    group: Option<Group>,
    /// `Contents` object of the enclosing group shape.
    object: Option<u32>,
}

struct Sink<'a> {
    shapes: &'a mut Vec<EscherShape>,
    anchorless: &'a mut usize,
}

/// The direct records of one shape container that matter for placement.
#[derive(Default)]
struct Recs {
    anchor: Option<[i32; 4]>,
    child: Option<[i32; 4]>,
    child_at: usize,
    fspgr: Option<[i32; 4]>,
    object: Option<u32>,
    rotation: f32,
}

fn four(data: &[u8], at: usize) -> Option<[i32; 4]> {
    let v = |i: usize| u32_at(data, at + 4 * i).map(|x| x as i32);
    Some([v(0)?, v(1)?, v(2)?, v(3)?])
}

/// Same rule as [`unrotated_frame`] for fractional rectangles; it is its own inverse.
fn swap_frame(a: [f64; 4], rotation_deg: f32) -> [f64; 4] {
    if !is_swapped(rotation_deg) {
        return a;
    }
    let (cx, cy) = ((a[0] + a[2]) / 2.0, (a[1] + a[3]) / 2.0);
    let (hw, hh) = ((a[3] - a[1]) / 2.0, (a[2] - a[0]) / 2.0);
    [cx - hw, cy - hh, cx + hw, cy + hh]
}

fn is_swapped(rotation_deg: f32) -> bool {
    (45.0..135.0).contains(&rotation_deg) || (225.0..315.0).contains(&rotation_deg)
}

/// Turn a stored anchor into the unrotated frame (spec 4H): at rotations in [45, 135) and
/// [225, 315) the stored width and height are swapped around the centre.
pub(crate) fn unrotated_frame(a: [i32; 4], rotation_deg: f32) -> [i32; 4] {
    if !is_swapped(rotation_deg) {
        return a;
    }
    // Work in doubled coordinates so the centre stays integral.
    let (cx2, cy2) = (a[0] as i64 + a[2] as i64, a[1] as i64 + a[3] as i64);
    let (w, h) = (a[2] as i64 - a[0] as i64, a[3] as i64 - a[1] as i64);
    [
        ((cx2 - h) / 2) as i32,
        ((cy2 - w) / 2) as i32,
        ((cx2 + h) / 2) as i32,
        ((cy2 + w) / 2) as i32,
    ]
}

/// Where a shape sits: its unrotated frame and total rotation, or `None` when it has no
/// usable anchor. A child anchor (inside a group with a known coordinate box) is mapped
/// onto the group's rectangle with independent x and y scale (spec 4J) and then turned
/// about the group's centre by the group's rotation.
///
/// UNCONFIRMED: child anchors of rotated children use the same swapped-extent convention
/// as client anchors; group rotation is not sampled.
fn place(ctx: &Ctx, r: &Recs) -> Option<([f64; 4], f32)> {
    if let (Some(g), Some(c)) = (ctx.group, r.child) {
        let (iw, ih) = (g.inner[2] - g.inner[0], g.inner[3] - g.inner[1]);
        let (rw, rh) = (g.rect[2] - g.rect[0], g.rect[3] - g.rect[1]);
        if iw != 0.0 && ih != 0.0 {
            let (sx, sy) = (rw / iw, rh / ih);
            let mapped = [
                g.rect[0] + (f64::from(c[0]) - g.inner[0]) * sx,
                g.rect[1] + (f64::from(c[1]) - g.inner[1]) * sy,
                g.rect[0] + (f64::from(c[2]) - g.inner[0]) * sx,
                g.rect[1] + (f64::from(c[3]) - g.inner[1]) * sy,
            ];
            let mut f = swap_frame(mapped, r.rotation);
            if g.rotation != 0.0 {
                let (gx, gy) = ((g.rect[0] + g.rect[2]) / 2.0, (g.rect[1] + g.rect[3]) / 2.0);
                let (cx, cy) = ((f[0] + f[2]) / 2.0, (f[1] + f[3]) / 2.0);
                let (sin, cos) = f64::from(g.rotation).to_radians().sin_cos();
                let (dx, dy) = (cx - gx, cy - gy);
                let (nx, ny) = (gx + dx * cos - dy * sin, gy + dx * sin + dy * cos);
                f = [
                    f[0] + nx - cx,
                    f[1] + ny - cy,
                    f[2] + nx - cx,
                    f[3] + ny - cy,
                ];
            }
            return Some((f, (r.rotation + g.rotation).rem_euclid(360.0)));
        }
    }
    r.anchor
        .map(|a| (unrotated_frame(a, r.rotation).map(f64::from), r.rotation))
}

/// Read the placement records among the direct children of a shape container.
fn scan_recs(data: &[u8], start: usize, end: usize) -> Result<Recs, PubError> {
    let mut r = Recs::default();
    let mut props: Vec<(u16, u32)> = Vec::new();
    let mut pos = start;
    while pos < end {
        let h = header(data, pos)?;
        let body = pos + 8;
        let next = body
            .checked_add(h.len)
            .filter(|&n| n <= end)
            .ok_or_else(|| PubError::format(pos, "Escher record extends past its container"))?;
        if !h.container {
            match h.ty {
                OPT => {
                    let count = usize::from(u16::from_le_bytes([data[pos], data[pos + 1]]) >> 4);
                    props = properties(data, body, h.len, count);
                }
                FSPGR => r.fspgr = four(data, body),
                CHILD_ANCHOR => {
                    r.child = four(data, body);
                    r.child_at = body;
                }
                CLIENT_ANCHOR | CLIENT_DATA => {
                    let block: Vec<Item> = items::decode_block(&data[..next], body)?;
                    if h.ty == CLIENT_ANCHOR {
                        let c = |id| items::find_int(&block, id).unwrap_or(0) as u32 as i32;
                        r.anchor = Some([c(1), c(2), c(3), c(4)]);
                    } else {
                        r.object = items::find_int(&block, 1).map(|v| v as u32);
                    }
                }
                _ => {}
            }
        }
        pos = next;
    }
    r.rotation = rotation(&props);
    Ok(r)
}

/// Walk a shape-group container (0xF003, spec 4J): its first shape container is the
/// group's own shape (coordinate box, anchor, client data); the rest are its children
/// and nested groups, which are walked with the group's coordinate system.
fn walk_group(
    data: &[u8],
    start: usize,
    end: usize,
    drawing: usize,
    depth: usize,
    ctx: Ctx,
    out: &mut Sink,
) -> Result<(), PubError> {
    if depth > MAX_DEPTH {
        return Err(PubError::format(
            start,
            "Escher containers nested too deeply",
        ));
    }
    let mut inner = ctx;
    let mut seen_own = false;
    let mut pos = start;
    while pos < end {
        let h = header(data, pos)?;
        let body = pos + 8;
        let next = body
            .checked_add(h.len)
            .filter(|&n| n <= end)
            .ok_or_else(|| PubError::format(pos, "Escher record extends past its container"))?;
        if h.container && h.ty == SPGR_CONTAINER {
            walk_group(data, body, next, drawing, depth + 1, inner, out)?;
        } else if h.container && h.ty == SP_CONTAINER && !seen_own {
            seen_own = true;
            let recs = scan_recs(data, body, next)?;
            inner = Ctx {
                object: recs.object.or(ctx.object),
                group: recs.fspgr.and_then(|f| {
                    let (rect, rotation) = place(&ctx, &recs)?;
                    Some(Group {
                        inner: f.map(f64::from),
                        rect,
                        rotation,
                    })
                }),
            };
            // The group's own shape belongs to the enclosing group.
            walk(data, body, next, drawing, depth + 1, ctx, out)?;
        } else if h.container {
            walk(data, body, next, drawing, depth + 1, inner, out)?;
        }
        pos = next;
    }
    Ok(())
}

/// Walk the records in `start..end`. Leaf records of interest accumulate into the
/// innermost enclosing container; a container that saw client data emits a shape.
fn walk(
    data: &[u8],
    start: usize,
    end: usize,
    drawing: usize,
    depth: usize,
    ctx: Ctx,
    out: &mut Sink,
) -> Result<(), PubError> {
    if depth > MAX_DEPTH {
        return Err(PubError::format(
            start,
            "Escher containers nested too deeply",
        ));
    }
    let mut recs = Recs::default();
    let mut story: Option<u32> = None;
    let mut picture_index: Option<u32> = None;
    let mut geometry: Option<ShapeGeometry> = None;
    let mut shape_flags: u32 = 0;
    let mut props: Vec<(u16, u32)> = Vec::new();
    let mut tertiary: Vec<(u16, u32)> = Vec::new();
    let mut pos = start;
    while pos < end {
        let h = header(data, pos)?;
        let body = pos + 8;
        let next = body
            .checked_add(h.len)
            .filter(|&n| n <= end)
            .ok_or_else(|| PubError::format(pos, "Escher record extends past its container"))?;
        if h.ty == TERTIARY_OPT && !h.container {
            let count = usize::from(u16::from_le_bytes([data[pos], data[pos + 1]]) >> 4);
            tertiary = properties(data, body, h.len, count);
        } else if h.container && h.ty == SPGR_CONTAINER {
            walk_group(data, body, next, drawing, depth + 1, ctx, out)?;
        } else if h.container {
            walk(data, body, next, drawing, depth + 1, ctx, out)?;
        } else if h.ty == OPT {
            let count = usize::from(u16::from_le_bytes([data[pos], data[pos + 1]]) >> 4);
            picture_index = property(data, body, h.len, count, PROP_PICTURE_INDEX);
            props = properties(data, body, h.len, count);
        } else if h.ty == SHAPE_RECORD {
            geometry = Some(ShapeGeometry::from_type(h.inst));
            if h.len >= 8 {
                shape_flags = u32::from_le_bytes([
                    data[body + 4],
                    data[body + 5],
                    data[body + 6],
                    data[body + 7],
                ]);
            }
        } else if h.ty == CHILD_ANCHOR {
            recs.child = four(data, body);
            recs.child_at = body;
        } else if matches!(h.ty, CLIENT_ANCHOR | CLIENT_DATA | CLIENT_TEXTBOX) {
            let block: Vec<Item> = items::decode_block(&data[..next], body)?;
            let int = |id| items::find_int(&block, id);
            match h.ty {
                CLIENT_ANCHOR => {
                    let c = |id| int(id).unwrap_or(0) as u32 as i32;
                    recs.anchor = Some([c(1), c(2), c(3), c(4)]);
                }
                CLIENT_DATA => recs.object = int(1).map(|v| v as u32),
                _ => story = int(1).map(|v| v as u32),
            }
        }
        pos = next;
    }
    if let Some(contents_object) = recs.object {
        recs.rotation = rotation(&props);
        let Some((frame, rotation_deg)) = place(&ctx, &recs) else {
            *out.anchorless += 1;
            return Ok(());
        };
        let mapped = ctx.group.is_some() && recs.child.is_some();
        let anchor = if mapped {
            swap_frame(frame, rotation_deg).map(|v| v.round() as i32)
        } else {
            recs.anchor.unwrap_or([0; 4])
        };
        let (fill, line) = look(&props);
        out.shapes.push(EscherShape {
            drawing,
            anchor,
            contents_object,
            story,
            picture_index,
            geometry,
            flip_h: shape_flags & 0x40 != 0,
            flip_v: shape_flags & 0x80 != 0,
            fill,
            line,
            rotation_deg,
            group: ctx.object,
            child: match (ctx.group, recs.child) {
                (Some(g), Some(_)) if mapped => Some(ChildInfo {
                    offset: recs.child_at,
                    inner: g.inner,
                    rect: g.rect,
                    group_rotation: g.rotation,
                    own_rotation: recs.rotation,
                }),
                _ => None,
            },
            columns: tertiary_columns(&tertiary),
            inset_emu: PROP_INSET.map(|id| {
                props
                    .iter()
                    .find(|p| p.0 == id)
                    .map_or(DEFAULT_INSET_EMU, |p| p.1)
            }),
            wrap_dist_emu: PROP_WRAP_DIST.map(|id| {
                props
                    .iter()
                    .find(|p| p.0 == id)
                    .map_or(DEFAULT_WRAP_DIST_EMU, |p| p.1)
            }),
        });
    }
    Ok(())
}

/// Byte positions in `EscherStm` of one shape's client anchor payload, plus its rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchorLocation {
    /// Offset of the anchor record's payload (the Publisher item block, spec 4A).
    pub block_offset: usize,
    pub rotation_deg: f32,
    /// Offset of the group coordinate box (FSPGR, four little-endian i32) when the shape is
    /// a group (spec 4J).
    pub fspgr_offset: Option<usize>,
}

/// Locate the client anchor (0xF010) of the shape whose client data (0xF011) names
/// `contents_object`. Read-only; mirrors the reader's grouping (one shape per container).
pub fn locate_anchor(
    data: &[u8],
    contents_object: u32,
) -> Result<Option<AnchorLocation>, PubError> {
    let first = header(data, 0)?;
    let mut pos = 8usize
        .checked_add(first.len)
        .filter(|&p| p <= data.len())
        .ok_or_else(|| PubError::format(0, "drawing group extends past the stream"))?;
    while pos < data.len() {
        pos += 4;
        let h = header(data, pos)?;
        let end = (pos + 8)
            .checked_add(h.len)
            .filter(|&e| e <= data.len())
            .ok_or_else(|| PubError::format(pos, "drawing extends past the stream"))?;
        if let Some(found) = locate_in(data, pos + 8, end, 0, contents_object)? {
            return Ok(Some(found));
        }
        pos = end;
    }
    Ok(None)
}

fn locate_in(
    data: &[u8],
    start: usize,
    end: usize,
    depth: usize,
    want: u32,
) -> Result<Option<AnchorLocation>, PubError> {
    if depth > MAX_DEPTH {
        return Err(PubError::format(
            start,
            "Escher containers nested too deeply",
        ));
    }
    let mut anchor: Option<usize> = None;
    let mut fspgr_offset: Option<usize> = None;
    let mut object: Option<u32> = None;
    let mut props: Vec<(u16, u32)> = Vec::new();
    let mut pos = start;
    while pos < end {
        let h = header(data, pos)?;
        let body = pos + 8;
        let next = body
            .checked_add(h.len)
            .filter(|&n| n <= end)
            .ok_or_else(|| PubError::format(pos, "Escher record extends past its container"))?;
        if h.container {
            if let Some(found) = locate_in(data, body, next, depth + 1, want)? {
                return Ok(Some(found));
            }
        } else if h.ty == OPT {
            let count = usize::from(u16::from_le_bytes([data[pos], data[pos + 1]]) >> 4);
            props = properties(data, body, h.len, count);
        } else if h.ty == CLIENT_ANCHOR {
            anchor = Some(body);
        } else if h.ty == FSPGR && h.len >= 16 {
            fspgr_offset = Some(body);
        } else if h.ty == CLIENT_DATA {
            let block = items::decode_block(&data[..next], body)?;
            object = items::find_int(&block, 1).map(|v| v as u32);
        }
        pos = next;
    }
    Ok(match (object, anchor) {
        (Some(o), Some(block_offset)) if o == want => Some(AnchorLocation {
            block_offset,
            rotation_deg: rotation(&props),
            fspgr_offset,
        }),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(rotation: f32) -> Ctx {
        Ctx {
            group: Some(Group {
                inner: [0.0, 0.0, 100.0, 100.0],
                rect: [200.0, 100.0, 400.0, 200.0],
                rotation,
            }),
            object: Some(7),
        }
    }

    #[test]
    fn child_anchor_scales_independently_onto_the_group() {
        let recs = Recs {
            child: Some([0, 0, 50, 50]),
            ..Recs::default()
        };
        let (frame, rot) = place(&group(0.0), &recs).unwrap();
        assert_eq!(frame, [200.0, 100.0, 300.0, 150.0]);
        assert_eq!(rot, 0.0);
    }

    #[test]
    fn group_rotation_turns_children_about_the_group_centre() {
        let mut ctx = group(90.0);
        // A square group so the 90 degree turn keeps its extent.
        if let Some(g) = ctx.group.as_mut() {
            g.rect = [0.0, 0.0, 100.0, 100.0];
        }
        let recs = Recs {
            child: Some([0, 0, 50, 50]),
            ..Recs::default()
        };
        let (frame, rot) = place(&ctx, &recs).unwrap();
        // Centre (25, 25) turned clockwise (y down) about (50, 50) lands at (75, 25).
        assert!((frame[0] - 50.0).abs() < 1e-9 && (frame[1] - 0.0).abs() < 1e-9);
        assert!((frame[2] - 100.0).abs() < 1e-9 && (frame[3] - 50.0).abs() < 1e-9);
        assert_eq!(rot, 90.0);
    }

    #[test]
    fn shape_without_any_anchor_has_no_place() {
        assert!(place(&Ctx::default(), &Recs::default()).is_none());
        // A child anchor without a group coordinate box cannot be placed either.
        let recs = Recs {
            child: Some([0, 0, 1, 1]),
            ..Recs::default()
        };
        assert!(place(&Ctx::default(), &recs).is_none());
    }

    /// PW-SEC-006: a small deflate stream that expands past the metafile cap is refused instead
    /// of being inflated.
    #[test]
    fn deflate_bomb_metafile_is_refused() {
        let bomb = miniz_oxide::deflate::compress_to_vec_zlib(&vec![0u8; MAX_METAFILE_BYTES + 4096], 1);
        assert!(bomb.len() < 1_000_000, "the stream is small ({} bytes)", bomb.len());
        let mut payload = vec![0u8; 16]; // uid
        let mut head = [0u8; METAFILE_HEADER_LEN];
        head[METAFILE_COMPRESSION_AT] = METAFILE_DEFLATE;
        payload.extend_from_slice(&head);
        payload.extend_from_slice(&bomb);
        assert!(metafile_bytes(&payload, 16).is_none());
        // An honest small stream still works.
        let ok = miniz_oxide::deflate::compress_to_vec_zlib(&[7u8; 1000], 1);
        let mut good = vec![0u8; 16];
        good.extend_from_slice(&head);
        good.extend_from_slice(&ok);
        assert_eq!(metafile_bytes(&good, 16).map(|b| b.len()), Some(1000));
    }
}
