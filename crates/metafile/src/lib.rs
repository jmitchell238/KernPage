//! Windows Metafile (WMF / EMF) parser producing a renderer-agnostic display list.
//!
//! Written only from the public specifications [MS-WMF] and [MS-EMF].
//!
//! # Output space
//!
//! All coordinates, sizes and line widths in the result are expressed in the **base
//! window space**: the logical coordinate space of the window that is in effect when
//! the first drawing record is played. Later changes of window/viewport origin or
//! extent, the map mode and (EMF) the world transform are folded into the emitted
//! coordinates, so a consumer needs no further mapping other than scaling
//! [`Metafile::bounds`] onto its target rectangle. For a placeable WMF this is the
//! space of the placeable bounding box; `y` grows downwards.
//!
//! Fixed-scale map modes other than `MM_TEXT` (`MM_LOMETRIC` and friends) are treated
//! as a 1:1 scale without the y-axis flip.
//!
//! [MS-WMF]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/
//! [MS-EMF]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-emf/

#![allow(clippy::chunks_exact_to_as_chunks)]

mod dib;
mod emf;
mod player;
mod wmf;

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.right - self.left
    }
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
}

/// A pen. `width` is in output units; 0 means a hairline (one device pixel).
/// Null pens are represented as `None` on the items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: LineStyle,
}

/// A solid fill. `approximated` is set when the real brush was hatched or a pattern
/// and this colour only approximates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub color: Color,
    pub approximated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    EvenOdd,
    NonZero,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSeg {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Baseline,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub face: String,
    /// Absolute height in output units.
    pub height: f64,
    /// True when the logical height was positive (cell height including internal
    /// leading); false when it was negative (character/em height) or zero.
    pub cell_height: bool,
    /// 100..=900 (400 normal, 700 bold); 0 when unspecified.
    pub weight: u16,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub charset: u8,
    /// Rotation in degrees counter-clockwise.
    pub escapement_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub text: String,
    /// Reference point, interpreted through `halign` / `valign`.
    pub pos: Point,
    pub font: Font,
    pub color: Color,
    pub halign: HAlign,
    pub valign: VAlign,
    /// Background colour when the background mode was opaque.
    pub background: Option<Color>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Polygon {
        points: Vec<Point>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
        rule: FillRule,
    },
    Polyline {
        points: Vec<Point>,
        stroke: Option<Stroke>,
    },
    PolyPolygon {
        polygons: Vec<Vec<Point>>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
        rule: FillRule,
    },
    /// Rectangle; `corner` is the (width, height) of the rounding ellipse if rounded.
    Rect {
        rect: Rect,
        corner: Option<(f64, f64)>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    Ellipse {
        rect: Rect,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    Path {
        segments: Vec<PathSeg>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
        rule: FillRule,
    },
    Text(Text),
    /// A decoded bitmap, top-down RGBA8 (`rgba.len() == width * height * 4`).
    Bitmap {
        dest: Rect,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    /// A bitmap that could not be decoded (compressed, palette-indexed usage, ...).
    BitmapPlaceholder {
        dest: Rect,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Wmf,
    Emf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metafile {
    pub format: Format,
    /// Extent of the picture in output space (see the crate docs).
    pub bounds: Rect,
    /// Logical units per inch from the placeable header (WMF only).
    pub inch: Option<u16>,
    pub items: Vec<Item>,
    /// Records parsed (including skipped ones).
    pub records: usize,
    /// Records that were unknown, unsupported for rendering, or malformed.
    pub skipped: usize,
    /// Record type -> count for skipped records (WMF function / EMF type number).
    pub skipped_types: BTreeMap<u32, usize>,
    /// Known records that have no effect on the display list (palettes, comments...).
    pub ignored: usize,
    /// Number of approximations made (hatched/pattern brushes, unsupported map modes).
    pub approximations: usize,
    /// True when the input ended (or became unparseable) before the end record.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetafileError {
    /// Fewer bytes than the minimum header.
    TooShort,
    /// Neither a WMF (optionally placeable) nor an EMF header.
    BadHeader(&'static str),
}

impl std::fmt::Display for MetafileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetafileError::TooShort => write!(f, "metafile data too short"),
            MetafileError::BadHeader(why) => write!(f, "bad metafile header: {why}"),
        }
    }
}

impl std::error::Error for MetafileError {}

/// Parse WMF (with or without the placeable header) or EMF bytes.
///
/// Truncated or damaged record streams yield a partial result with `truncated` set;
/// only an unusable header is an error. This function never panics.
pub fn parse(bytes: &[u8]) -> Result<Metafile, MetafileError> {
    if bytes.len() < 18 {
        return Err(MetafileError::TooShort);
    }
    let sig = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if sig == 1 && bytes.len() >= 44 && &bytes[40..44] == b" EMF" {
        emf::parse(bytes)
    } else {
        wmf::parse(bytes)
    }
}
