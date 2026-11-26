//! Reader for Microsoft Publisher 2003-2010 `.pub` files, written from
//! `docs/spec/PUB-FORMAT-SPEC.md`.

mod color;
pub mod contents;
mod error;
mod escher;
use escher::unrotated_frame;
pub mod items;
pub mod list;
mod quill;

use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Seek};
use std::path::Path;

pub use color::{Color, ColorScheme};
pub use error::PubError;
pub use escher::{AnchorLocation, ChildInfo, Picture, PictureFormat, ShapeGeometry};
pub use items::IntSlot;
pub mod wrap;
pub use list::{
    bullet_display_char, is_symbol_font, line_starts, symbol_unicode, list_labels, list_labels_for, number_text, LineStarts, ListInfo,
    ListType, ParaIndents,
};
pub use quill::{Alignment, Field, Frame, LineSpacing, ParaSpacing, Paragraph, Script, TextRun};

const KIND_PAGE: u16 = 67;
const KIND_SECTIONS: u16 = 117;
const ITEM_SECTIONS: u16 = 47;
const KIND_FONT: u16 = 108;
const KIND_COLOR_SCHEME: u16 = 92;
const KIND_SETTINGS: u16 = 68;
const KIND_PAGE_SIZE: u16 = 76;
const LETTER_WIDTH_EMU: u32 = 7_772_400;
const LETTER_HEIGHT_EMU: u32 = 10_058_400;
const KIND_TABLE: u16 = 16;
const KIND_TABLE_CELLS: u16 = 99;
/// Table object items (spec 4E): rows, columns, total width, total height (EMU).
const ITEM_TABLE_ROWS: u16 = 102;
const ITEM_TABLE_COLUMNS: u16 = 103;
const ITEM_TABLE_WIDTH: u16 = 104;
const ITEM_TABLE_HEIGHT: u16 = 105;
/// Grid sizes (list: columns, then rows) and the object number of the kind-99 cell list.
const ITEM_TABLE_GRID: u16 = 109;
const ITEM_TABLE_CELLS: u16 = 107;
/// Default table cell margin, EMU (spec 4E).
const DEFAULT_CELL_MARGIN_EMU: u32 = 36_576;
const EMU_PER_PT: f32 = 12_700.0;
const ITEM_WIDTH: u16 = 170;
const ITEM_HEIGHT: u16 = 171;
/// Low byte = wrap type (spec 4K).
const ITEM_WRAP: u16 = 4;
const ITEM_WRAP2: u16 = 5;
/// Text-box story number and position in its chain of linked boxes (spec 4G).
const ITEM_CHAIN_STORY: u16 = 39;
const ITEM_CHAIN_POS: u16 = 40;
/// Page objects are found by walking the parent chain; this bounds a corrupt cycle.
/// Frame size tolerance when matching `MCLD` to `Contents`, EMU (1 pt).
const EMU_TOLERANCE: i64 = 12_700;
const MAX_PARENT_HOPS: usize = 32;
/// Most picture bytes (all shapes together, after decompression) one publication may hold.
const MAX_TOTAL_PICTURE_BYTES: usize = 512 << 20;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Document {
    pub pages: Vec<Page>,
    /// The whole Quill `TEXT` chunk.
    pub text: String,
    /// The `TEXT` chunk split into stories (Quill `STRS`).
    pub stories: Vec<String>,
    /// Quill `SYID`: the story number of each entry of `stories` (empty when unreadable).
    pub story_ids: Vec<u32>,
    pub fonts: Vec<String>,
    /// Page size in EMU (914,400 per inch).
    pub page_width_emu: u32,
    pub page_height_emu: u32,
    /// The margin guides of the master page (spec 4Q.6) as margins in EMU: left, right, top,
    /// bottom. `None` when the file has no readable set of four margin guides.
    pub margins_emu: Option<[u32; 4]>,
    /// The colour scheme (Contents kind 92, spec 4F); `None` when the file has none.
    pub color_scheme: Option<ColorScheme>,
    /// Chains whose `MCLD` group matched (frame count and sizes) and so got their column
    /// settings (spec 4I); diagnostics for the group-to-story mapping.
    pub columns_mapped: usize,
    /// Chains whose `MCLD` group did not match; their boxes use one column.
    pub columns_unmapped: usize,
    /// Shapes left out because they had neither a client anchor nor a usable child
    /// anchor (spec 4J).
    pub shapes_without_anchor: usize,
    /// The master pages (kind-67 objects with a name, spec 4T.2), in file order. A page's
    /// [`Page::master`] indexes this list.
    pub masters: Vec<MasterPage>,
    /// Page-number sections (spec 4T.4); empty = one Arabic section starting at 1.
    pub sections: Vec<Section>,
}

/// A master page (spec 4T): shapes drawn behind the shapes of every page that uses it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MasterPage {
    pub object_number: u32,
    /// The master's name (item 15), e.g. `Master Page A`.
    pub name: String,
    /// The short label (item 14), e.g. `A`.
    pub letter: String,
    pub shapes: Vec<Shape>,
}

/// How page numbers are written (`PbPageNumberFormat`, spec 4T.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumberFormat {
    #[default]
    Arabic,
    UpperRoman,
    LowerRoman,
    UpperLetter,
    LowerLetter,
}

impl NumberFormat {
    /// The stored value (item 3 of a section record). ~ Values 5 and above (ordinal, words,
    /// East Asian forms) are shown as Arabic for now.
    pub fn from_stored(v: u64) -> NumberFormat {
        match v {
            1 => NumberFormat::UpperRoman,
            2 => NumberFormat::LowerRoman,
            3 => NumberFormat::UpperLetter,
            4 => NumberFormat::LowerLetter,
            _ => NumberFormat::Arabic,
        }
    }

    /// `n` written in this format (0 is written as `0` in every format).
    pub fn format(self, n: u32) -> String {
        match self {
            NumberFormat::Arabic => n.to_string(),
            _ if n == 0 => "0".to_owned(),
            NumberFormat::UpperRoman => roman(n),
            NumberFormat::LowerRoman => roman(n).to_lowercase(),
            NumberFormat::UpperLetter => letters(n),
            NumberFormat::LowerLetter => letters(n).to_lowercase(),
        }
    }
}

fn roman(mut n: u32) -> String {
    const T: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut out = String::new();
    for (v, t) in T {
        while n >= v {
            out.push_str(t);
            n -= v;
        }
    }
    out
}

/// 1 -> A ... 26 -> Z, then ~ AA, BB, ... ZZ, AAA, ... (Word-style repetition; Publisher's
/// own letters beyond Z were not checked).
fn letters(n: u32) -> String {
    let c = char::from(b'A' + ((n - 1) % 26) as u8);
    std::iter::repeat_n(c, ((n - 1) / 26 + 1) as usize).collect()
}

/// One page-number section (spec 4T.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// First page of the section, 1-based (index into `Document::pages` + 1).
    pub start_page: u32,
    /// The number of the first page when the section does not continue the previous one.
    pub first_number: Option<u32>,
    pub format: NumberFormat,
    /// Items 4 and 5, both true when "show header and footer on the first page" is off. ?
    /// Which of the two means which is not known.
    pub hide_first_page_hf: bool,
}

/// The number page `page_index` (0-based) shows, as a plain number (spec 4T.4): within a
/// section, the section's first number (or, when it continues, the number the previous
/// section's numbering has reached) plus the page's index inside the section.
pub fn page_number_value(sections: &[Section], page_index: usize) -> u32 {
    let page = page_index as u32 + 1;
    let mut start_number = 1u32; // number shown by the first page of the current section
    let mut start_page = 1u32;
    for s in sections {
        if s.start_page > page {
            break;
        }
        let sp = s.start_page.max(1);
        start_number = s
            .first_number
            .unwrap_or_else(|| start_number.saturating_add(sp.saturating_sub(start_page)));
        start_page = sp;
    }
    start_number.saturating_add(page - start_page)
}

/// The text a page-number field shows on page `page_index` (0-based), written in the format
/// of the section the page is in.
pub fn page_number_in(sections: &[Section], page_index: usize) -> String {
    let page = page_index as u32 + 1;
    let format = sections
        .iter()
        .take_while(|s| s.start_page <= page)
        .last()
        .map_or(NumberFormat::Arabic, |s| s.format);
    format.format(page_number_value(sections, page_index))
}

/// [`page_number_in`] for a document.
pub fn page_number_text(doc: &Document, page_index: usize) -> String {
    page_number_in(&doc.sections, page_index)
}

impl Shape {
    /// A copy whose page-number field runs show `number` instead of `#` (spec 4T.3);
    /// `None` when the shape has no field.
    pub fn with_page_number(&self, number: &str) -> Option<Shape> {
        let has = |r: &TextRun| r.field.is_some();
        if !self.runs.iter().any(has) && !self.paragraphs.iter().flat_map(|p| &p.runs).any(has) {
            return None;
        }
        let swap = |r: &mut TextRun| {
            if r.field.is_some() {
                r.text = number.to_owned();
            }
        };
        let mut s = self.clone();
        s.runs.iter_mut().for_each(swap);
        s.paragraphs
            .iter_mut()
            .flat_map(|p| p.runs.iter_mut())
            .for_each(swap);
        s.text = Some(
            s.paragraphs
                .iter()
                .flat_map(|p| p.runs.iter().map(|r| r.text.as_str()))
                .collect(),
        );
        Some(s)
    }
}

/// How text of other boxes flows around a shape (spec 4K; `PbWrapType` numbering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WrapType {
    None,
    TopAndBottom,
    Square,
    Tight,
    Through,
    /// A value outside the five types (never written).
    Other(u8),
}

impl WrapType {
    /// From the low byte of `Contents` item 4 (present).
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => WrapType::None,
            1 => WrapType::TopAndBottom,
            2 => WrapType::Square,
            3 => WrapType::Tight,
            n => WrapType::Other(n),
        }
    }

    /// The wrap type from `Contents` items 4 and 5 (spec 4K): item 4 wins when present
    /// (high byte is noise); absent item 4 is top-and-bottom, or through when item 5 is 4.
    pub fn from_items(item4: Option<u16>, item5: Option<u16>) -> Self {
        match item4 {
            Some(v) => WrapType::from_byte(v as u8),
            None if item5.is_some_and(|v| v as u8 == 4) => WrapType::Through,
            None => WrapType::TopAndBottom,
        }
    }

    /// The `PbWrapType` number (0 none, 1 top and bottom, 2 square, 3 tight, 4 through).
    pub fn to_byte(self) -> u8 {
        match self {
            WrapType::None => 0,
            WrapType::TopAndBottom => 1,
            WrapType::Square => 2,
            WrapType::Tight => 3,
            WrapType::Through => 4,
            WrapType::Other(n) => n,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    pub object_number: u32,
    pub shapes: Vec<Shape>,
    /// Index into `Document::masters` of the page's master (item 13, spec 4T.2).
    pub master: Option<usize>,
    /// Item 7: the page does not show its master's shapes ("Ignore Master Page").
    pub ignore_master: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    pub object_number: u32,
    pub kind: u16,
    /// Frame in EMU, measured from the page centre. This is the **unrotated** frame: for
    /// rotated shapes the stored anchor's width and height are swapped back (spec 4H).
    pub left_emu: i32,
    pub top_emu: i32,
    pub right_emu: i32,
    pub bottom_emu: i32,
    pub width_emu: Option<u32>,
    pub height_emu: Option<u32>,
    /// Rotation in degrees clockwise, in [0, 360) (spec 4H); the frame above is unrotated.
    pub rotation_deg: f32,
    /// Object number of the group shape (a kind-48 object, spec 4J) this shape is a
    /// child of; `None` when it sits directly on the page. For children of a rotated
    /// group, `rotation_deg` includes the group's rotation.
    pub group: Option<u32>,
    /// Wrap type (spec 4K); `None` when the object has no item 4.
    pub wrap: WrapType,
    /// Wrap distances left, top, right, bottom in EMU (spec 4K); default 36,576.
    pub wrap_dist_emu: [u32; 4],
    /// Text-frame margins left, top, right, bottom in EMU (Escher 0x81..0x84, spec 4N.1);
    /// 36,576 (0.04 in) when the shape has none.
    pub inset_emu: [u32; 4],
    /// Text columns (spec 4I); 1 when unknown.
    pub columns: u32,
    /// Gap between columns in EMU (spec 4I).
    pub column_gap_emu: u32,
    pub story: Option<u32>,
    /// Story number from `Contents` item 39 (spec 4G); boxes of one chain share it.
    pub chain_story: Option<u32>,
    /// Position in the chain from `Contents` item 40 (absent = 0, spec 4G).
    pub chain_pos: u32,
    /// Text of the shape's story, if it has one.
    pub text: Option<String>,
    /// The text as formatted runs (adjacent runs are merged when all attributes match).
    pub runs: Vec<TextRun>,
    /// The same text split into paragraphs, each with its alignment.
    pub paragraphs: Vec<Paragraph>,
    /// The picture, for picture shapes whose data could be read.
    pub picture: Option<Picture>,
    /// Shape type from the Escher shape record (spec 4D).
    pub geometry: Option<ShapeGeometry>,
    /// Flipped horizontally / vertically (shape-record flags 0x40 / 0x80); a line runs
    /// bottom-left to top-right when exactly one is set.
    pub flip_h: bool,
    pub flip_v: bool,
    /// Fill colour; `None` when the shape is not filled.
    pub fill: Option<Color>,
    /// Outline colour and width in EMU; `None` when the line is off.
    pub line: Option<(Color, u32)>,
    /// The table, for kind-16 objects (spec 4E).
    pub table: Option<Table>,
}

/// Where a shape sits in a [`Document`]: page index and index within `Page::shapes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ShapeRef {
    pub page: usize,
    pub shape: usize,
}

impl Document {
    /// The regions text of shape `shape` on page `page` must avoid (spec 4K): every later
    /// (in front) shape that overlaps its frame, has a wrap type other than none and is
    /// not itself text. Polygons are in points from the page's top-left, expressed in
    /// the text shape's own unrotated frame.
    // TODO(pw-2wv.16): text boxes carry wrap type 3 by default (spec 4K); they are not
    // treated as wrappers until a sample shows what Publisher does with them.
    pub fn wrap_obstacles(&self, page: usize, shape: usize) -> Vec<wrap::Poly> {
        let Some(pg) = self.pages.get(page) else {
            return Vec::new();
        };
        let Some(me) = pg.shapes.get(shape) else {
            return Vec::new();
        };
        let (pw, ph) = (
            self.page_width_emu as f32 / EMU_PER_PT,
            self.page_height_emu as f32 / EMU_PER_PT,
        );
        let frame = |s: &Shape| {
            (
                pw / 2.0 + s.left_emu as f32 / EMU_PER_PT,
                ph / 2.0 + s.top_emu as f32 / EMU_PER_PT,
                pw / 2.0 + s.right_emu as f32 / EMU_PER_PT,
                ph / 2.0 + s.bottom_emu as f32 / EMU_PER_PT,
            )
        };
        let mine = frame(me);
        let centre = ((mine.0 + mine.2) / 2.0, (mine.1 + mine.3) / 2.0);
        pg.shapes
            .iter()
            .skip(shape + 1)
            .filter(|o| {
                o.wrap != WrapType::None
                    && o.story.is_none()
                    && o.chain_story.is_none()
                    && o.paragraphs.is_empty()
                    && o.table.is_none()
                    && o.object_number != me.object_number
            })
            .filter_map(|o| {
                let dist = o.wrap_dist_emu.map(|d| d as f32 / EMU_PER_PT);
                let poly = wrap::obstacle(frame(o), o.rotation_deg, o.wrap, dist, o.geometry)?;
                let poly = wrap::to_frame(&poly, centre, me.rotation_deg);
                // Keep only regions that reach the (unrotated) frame.
                let xs = || poly.pts.iter().map(|p| p.0);
                let ys = || poly.pts.iter().map(|p| p.1);
                let reaches = xs().fold(f32::MIN, f32::max) > mine.0
                    && xs().fold(f32::MAX, f32::min) < mine.2
                    && ys().fold(f32::MIN, f32::max) > mine.1
                    && ys().fold(f32::MAX, f32::min) < mine.3;
                reaches.then_some(poly)
            })
            .collect()
    }

    /// Group the text boxes by story number (`Contents` item 39) into chains, each ordered
    /// by chain position (item 40, absent = 0; ties keep document order). Chains are
    /// returned in the order their first box appears. A standalone box is a chain of one
    /// (spec 4G).
    pub fn chains(&self) -> Vec<Vec<ShapeRef>> {
        let mut groups: Vec<(u32, Vec<(u32, ShapeRef)>)> = Vec::new();
        for (pi, page) in self.pages.iter().enumerate() {
            for (si, shape) in page.shapes.iter().enumerate() {
                let Some(story) = shape.chain_story else {
                    continue;
                };
                let r = ShapeRef {
                    page: pi,
                    shape: si,
                };
                match groups.iter_mut().find(|(n, _)| *n == story) {
                    Some((_, v)) => v.push((shape.chain_pos, r)),
                    None => groups.push((story, vec![(shape.chain_pos, r)])),
                }
            }
        }
        groups
            .into_iter()
            .map(|(_, mut v)| {
                v.sort_by_key(|(pos, _)| *pos); // stable
                v.into_iter().map(|(_, r)| r).collect()
            })
            .collect()
    }
}

/// A table: its cells placed in row-major order by their row/column items (spec 4E.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub rows: u32,
    pub columns: u32,
    /// `rows * columns` cell texts (without the trailing `\r`), row-major. Paragraphs of a
    /// multi-paragraph cell are joined with `\r`.
    pub cells: Vec<String>,
    /// Column widths in EMU (item 109); empty when the grid is missing or inconsistent.
    pub column_widths_emu: Vec<u32>,
    /// Row heights in EMU (item 109); empty when the grid is missing or inconsistent.
    pub row_heights_emu: Vec<u32>,
    /// Per cell (row-major), its paragraphs with formatting. Parallel to `cells`.
    pub cell_paragraphs: Vec<Vec<Paragraph>>,
    /// Per cell margins and spans (kind-99 object); empty when it is missing or inconsistent.
    pub cell_props: Vec<CellProps>,
}

/// Margins and spans of one table cell (spec 4E). Margin order left, top, right, bottom
/// is assumed (items 10-13); every sample has the default on all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CellProps {
    pub first_row: u32,
    pub last_row: u32,
    pub first_column: u32,
    pub last_column: u32,
    pub margin_left_emu: u32,
    pub margin_top_emu: u32,
    pub margin_right_emu: u32,
    pub margin_bottom_emu: u32,
}

/// The three raw streams the reader needs, extracted from the compound file.
pub struct Streams {
    pub contents: Vec<u8>,
    pub escher: Vec<u8>,
    pub quill: Vec<u8>,
    /// `Escher/EscherDelayStm` (picture data); empty when the file has none.
    pub delay: Vec<u8>,
}

impl Streams {
    pub fn from_cfb(bytes: &[u8]) -> Result<Self, PubError> {
        let mut cf = cfb::CompoundFile::open(Cursor::new(bytes))?;
        Ok(Streams {
            contents: stream(&mut cf, "/Contents")?,
            escher: stream(&mut cf, "/Escher/EscherStm")?,
            quill: stream(&mut cf, "/Quill/QuillSub/CONTENTS")?,
            delay: optional_stream(&mut cf, "/Escher/EscherDelayStm")?,
        })
    }
}

fn stream<F: Read + Seek>(cf: &mut cfb::CompoundFile<F>, path: &str) -> Result<Vec<u8>, PubError> {
    if !cf.is_stream(path) {
        return Err(PubError::MissingStream(path.trim_start_matches('/').into()));
    }
    let mut buf = Vec::new();
    cf.open_stream(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

fn optional_stream<F: Read + Seek>(
    cf: &mut cfb::CompoundFile<F>,
    path: &str,
) -> Result<Vec<u8>, PubError> {
    if cf.is_stream(path) {
        stream(cf, path)
    } else {
        Ok(Vec::new())
    }
}

pub fn read(path: &Path) -> Result<Document, PubError> {
    read_bytes(&std::fs::read(path)?)
}

pub fn read_bytes(bytes: &[u8]) -> Result<Document, PubError> {
    // Check the Contents stream first so older-generation files get the
    // "unsupported format generation" error even when other streams are missing.
    let mut cf = cfb::CompoundFile::open(Cursor::new(bytes))?;
    let contents = stream(&mut cf, "/Contents")?;
    contents::Contents::parse(&contents)?;
    let streams = Streams {
        contents,
        escher: stream(&mut cf, "/Escher/EscherStm")?,
        quill: stream(&mut cf, "/Quill/QuillSub/CONTENTS")?,
        delay: optional_stream(&mut cf, "/Escher/EscherDelayStm")?,
    };
    assemble(&streams)
}

/// Build a [`Document`] from already-extracted streams.
pub fn assemble(streams: &Streams) -> Result<Document, PubError> {
    let contents = contents::Contents::parse(&streams.contents)?;

    // Pages: kind-67 objects. Every sample also has a master page and four other kind-67
    // objects (blank.pub has six); the real pages are the ones whose item 6 (8 bytes) is
    // non-zero. Their ORDER is the order of the ids in the parent's (object 256) item 2
    // list (spec 4P.1). Item 6's low word is NOT the order (changed: it used to be sorted
    // by it, which breaks files with inserted or moved pages). Pages missing from the list
    // (not seen in practice) go last, by the old low-word rule.
    let mut pages: Vec<(usize, u64, Page, Option<u64>)> = Vec::new();
    let mut list_pos: Option<std::collections::HashMap<u32, usize>> = None;
    for entry in contents.objects.iter().filter(|e| e.kind == KIND_PAGE) {
        let items = contents.decode(entry)?;
        let seq = items::find_int(&items, 6).unwrap_or(0);
        if seq != 0 {
            if list_pos.is_none() {
                let mut m = std::collections::HashMap::new();
                if let Some(parent) = entry.parent.and_then(|p| contents.get(p)) {
                    let pitems = contents.decode(parent)?;
                    if let Some(items::Value::List(l)) =
                        items::find(&pitems, 2).map(|i| &i.value)
                    {
                        for (i, it) in l.iter().enumerate() {
                            if let Some(id) = it.as_int() {
                                m.entry(id as u32).or_insert(i);
                            }
                        }
                    }
                }
                list_pos = Some(m);
            }
            let pos = list_pos
                .as_ref()
                .and_then(|m| m.get(&entry.number))
                .copied()
                .unwrap_or(usize::MAX);
            pages.push((
                pos,
                seq & 0xFFFF_FFFF,
                Page {
                    object_number: entry.number,
                    shapes: Vec::new(),
                    master: None,
                    ignore_master: items::find(&items, 7)
                        .is_some_and(|i| i.value != items::Value::Int(0)),
                },
                items::find_int(&items, 13),
            ));
        }
    }
    pages.sort_by_key(|(pos, seq, _, _)| (*pos, *seq));
    let master_refs: Vec<Option<u64>> = pages.iter().map(|p| p.3).collect();
    let mut pages: Vec<Page> = pages.into_iter().map(|(_, _, p, _)| p).collect();

    // Master pages (spec 4T.2): kind-67 objects with no sequence number that have a name.
    let mut masters: Vec<MasterPage> = Vec::new();
    for entry in contents.objects.iter().filter(|e| e.kind == KIND_PAGE) {
        let items = contents.decode(entry)?;
        if items::find_int(&items, 6).unwrap_or(0) != 0 {
            continue;
        }
        let text = |id| match items::find(&items, id).map(|i| &i.value) {
            Some(items::Value::Raw(raw)) => quill::utf16le(raw).trim_end_matches('\0').to_owned(),
            _ => String::new(),
        };
        let name = text(15);
        if name.is_empty() {
            continue;
        }
        masters.push(MasterPage {
            object_number: entry.number,
            name,
            letter: text(14),
            shapes: Vec::new(),
        });
    }
    for (page, r) in pages.iter_mut().zip(&master_refs) {
        page.master = r.and_then(|n| masters.iter().position(|m| u64::from(m.object_number) == n));
    }

    // Shapes: geometry comes from Escher (anchor, story, Contents object number).
    // The page a shape is on is taken from the Contents parent chain (the shape
    // object's parent, or an ancestor of it, is the kind-67 page object), not from the
    // Escher drawing it sits in: the stream has 3 drawings regardless of page count, so
    // the drawing-to-page mapping is not known (spec 4A). Shapes whose chain reaches no
    // page (e.g. master-page shapes) are dropped.
    // UNCONFIRMED: parent-chain page attribution verified only on single-shape samples.
    let escher = escher::read(&streams.escher)?;
    let mut pic_cache: std::collections::HashMap<usize, Option<escher::Picture>> =
        std::collections::HashMap::new();
    let mut pic_bytes_given = 0usize;
    for es in &escher.shapes {
        let Some(entry) = contents.get(es.contents_object) else {
            continue;
        };
        let target = match page_of(&contents, entry.parent, &pages) {
            Some(ix) => Target::Page(ix),
            // A shape under a master page (spec 4T.1).
            None => match master_of(&contents, entry.parent, &masters) {
                Some(ix) => Target::Master(ix),
                None => continue,
            },
        };
        let items = contents.decode(entry)?;
        let dim = |id| items::find_int(&items, id).map(|v| v as u32);
        // Tables carry their total size in items 104/105 instead of 170/171 (spec 4E).
        let is_table = entry.kind == KIND_TABLE;
        let (w_item, h_item) = if is_table {
            (ITEM_TABLE_WIDTH, ITEM_TABLE_HEIGHT)
        } else {
            (ITEM_WIDTH, ITEM_HEIGHT)
        };
        let table_dims = if is_table {
            dim(ITEM_TABLE_ROWS).zip(dim(ITEM_TABLE_COLUMNS))
        } else {
            None
        };
        let [left_emu, top_emu, right_emu, bottom_emu] =
            unrotated_frame(es.anchor, es.rotation_deg);
        // Each blip is decoded once; every shape that shows it gets a copy, and the copies are
        // capped in total, so a few KB of file cannot ask for gigabytes (PW-SEC-006).
        let picture = es
            .picture_index
            .and_then(|i| (i as usize).checked_sub(1))
            .and_then(|ix| {
                let found = pic_cache
                    .entry(ix)
                    .or_insert_with(|| escher.blips.get(ix).and_then(|b| escher::picture(&streams.delay, b)))
                    .as_ref()?;
                pic_bytes_given = pic_bytes_given.checked_add(found.bytes.len())?;
                (pic_bytes_given <= MAX_TOTAL_PICTURE_BYTES).then(|| found.clone())
            });
        let new_shape = Shape {
            object_number: entry.number,
            kind: entry.kind,
            left_emu,
            top_emu,
            right_emu,
            bottom_emu,
            width_emu: dim(w_item),
            // Item 105 is stale after a row auto-grows (spec 4E.1.1): use the row grid.
            height_emu: table_dims
                .and_then(|(rows, columns)| grid_sizes(&items, rows, columns))
                .and_then(|g| g.1.iter().try_fold(0u32, |a, &h| a.checked_add(h)))
                .or_else(|| dim(h_item)),
            rotation_deg: es.rotation_deg,
            group: es.group,
            // Spec 4K: absent item 4 = top and bottom (item 5 = 4: through).
            wrap: WrapType::from_items(
                items::find_int(&items, ITEM_WRAP).map(|v| v as u16),
                items::find_int(&items, ITEM_WRAP2).map(|v| v as u16),
            ),
            wrap_dist_emu: es.wrap_dist_emu,
            inset_emu: es.inset_emu,
            columns: es.columns.map_or(1, |c| c.0),
            column_gap_emu: es.columns.map_or(0, |c| c.1),
            story: es.story,
            chain_story: dim(ITEM_CHAIN_STORY),
            chain_pos: dim(ITEM_CHAIN_POS).unwrap_or(0),
            text: None,
            runs: Vec::new(),
            paragraphs: Vec::new(),
            picture,
            geometry: es.geometry,
            flip_h: es.flip_h,
            flip_v: es.flip_v,
            fill: es.fill,
            line: es.line,
            // Cells are filled in once the story text is attached.
            table: table_dims.map(|(rows, columns)| Table {
                rows,
                columns,
                cells: Vec::new(),
                column_widths_emu: grid_sizes(&items, rows, columns)
                    .map(|g| g.0)
                    .unwrap_or_default(),
                row_heights_emu: grid_sizes(&items, rows, columns)
                    .map(|g| g.1)
                    .unwrap_or_default(),
                cell_paragraphs: Vec::new(),
                cell_props: cell_props(&contents, &items, rows, columns).unwrap_or_default(),
            }),
        };
        match target {
            Target::Page(ix) => pages[ix].shapes.push(new_shape),
            Target::Master(ix) => masters[ix].shapes.push(new_shape),
        }
    }

    let quill = quill::read(&streams.quill)?;
    let order = story_order(&pages, &masters, &quill.story_ids, quill.stories.len());
    attach_stories(pages.iter_mut().flat_map(|p| p.shapes.iter_mut()), &order, &quill.stories, &quill.cell_ends);
    attach_stories(masters.iter_mut().flat_map(|m| m.shapes.iter_mut()), &order, &quill.stories, &quill.cell_ends);
    let from_escher: std::collections::HashSet<u32> = escher
        .shapes
        .iter()
        .filter(|e| e.columns.is_some())
        .map(|e| e.contents_object)
        .collect();
    let column_report = attach_columns(&mut pages, &order, &quill.frames, &from_escher);

    let mut fonts = Vec::new();
    for entry in contents.objects.iter().filter(|e| e.kind == KIND_FONT) {
        collect_font_names(&contents.decode(entry)?, &mut fonts);
    }

    // UNCONFIRMED: fall back to US Letter when neither kind 68 nor 76 has a size.
    let (page_width_emu, page_height_emu) =
        page_size(&contents).unwrap_or((LETTER_WIDTH_EMU, LETTER_HEIGHT_EMU));

    let color_scheme = match contents
        .objects
        .iter()
        .find(|e| e.kind == KIND_COLOR_SCHEME)
    {
        Some(entry) => Some(ColorScheme::from_items(&contents.decode(entry)?)),
        None => None,
    };

    Ok(Document {
        pages,
        stories: quill
            .stories
            .iter()
            .map(|paras| story_text(paras))
            .collect(),
        story_ids: quill.story_ids,
        text: quill.text,
        fonts,
        page_width_emu,
        page_height_emu,
        margins_emu: margins(&contents, page_width_emu, page_height_emu),
        color_scheme,
        columns_mapped: column_report.0,
        columns_unmapped: column_report.1,
        shapes_without_anchor: escher.anchorless,
        masters,
        sections: sections(&contents)?,
    })
}

/// Assign `MCLD` column settings to text boxes (spec 4G, 4I). The k-th group belongs to
/// the k-th story; a chain takes its group only when the frame counts agree and every
/// frame's size matches the box's `Contents` size (when it has one) (within 1 pt, in either orientation
/// because rotated boxes may store them swapped). Otherwise its boxes keep one column.
/// Returns (chains mapped, chains left unmapped).
fn attach_columns(
    pages: &mut [Page],
    numbers: &[u32],
    groups: &[Vec<quill::Frame>],
    from_escher: &std::collections::HashSet<u32>,
) -> (usize, usize) {
    let doc = Document {
        pages: pages.to_vec(),
        text: String::new(),
        stories: Vec::new(),
        story_ids: Vec::new(),
        fonts: Vec::new(),
        page_width_emu: 0,
        page_height_emu: 0,
        margins_emu: None,
        color_scheme: None,
        columns_mapped: 0,
        columns_unmapped: 0,
        shapes_without_anchor: 0,
        masters: Vec::new(),
        sections: Vec::new(),
    };
    let (mut mapped, mut unmapped) = (0, 0);
    for chain in doc.chains() {
        let first = &doc.pages[chain[0].page].shapes[chain[0].shape];
        let group = first
            .story
            .and_then(|n| numbers.iter().position(|&x| x == n))
            .and_then(|rank| groups.get(rank));
        let close = |a: u32, b: u32| (i64::from(a) - i64::from(b)).abs() <= EMU_TOLERANCE;
        let fits = group.is_some_and(|g| {
            g.len() == chain.len()
                && g.iter().zip(&chain).all(|(f, r)| {
                    let s = &doc.pages[r.page].shapes[r.shape];
                    // A box with no stored size cannot be checked; the frame count alone
                    // has to agree for it.
                    let (Some(w), Some(h)) = (s.width_emu, s.height_emu) else {
                        return true;
                    };
                    (close(f.width_emu, w) && close(f.height_emu, h))
                        || (close(f.width_emu, h) && close(f.height_emu, w))
                })
        });
        if !fits {
            unmapped += 1;
            continue;
        }
        mapped += 1;
        for (f, r) in group.into_iter().flatten().zip(&chain) {
            let s = &mut pages[r.page].shapes[r.shape];
            // The Escher values (spec 4I) are authoritative; MCLD is only a mirror.
            if from_escher.contains(&s.object_number) {
                continue;
            }
            s.columns = f.columns;
            s.column_gap_emu = f.column_gap_emu;
        }
    }
    (mapped, unmapped)
}

/// The story numbers in story order: the k-th number belongs to story index k. This is
/// the Quill `SYID` list when it has one number per story and covers every number the
/// shapes use (stories of deleted shapes stay in the file as orphans, spec 4P.4);
/// otherwise the numbers the shapes use, sorted (spec 4G).
fn story_order(pages: &[Page], masters: &[MasterPage], story_ids: &[u32], story_count: usize) -> Vec<u32> {
    let mut numbers: Vec<u32> = pages
        .iter()
        .flat_map(|p| p.shapes.iter().filter_map(|s| s.story))
        .chain(masters.iter().flat_map(|m| m.shapes.iter().filter_map(|s| s.story)))
        .collect();
    numbers.sort_unstable();
    numbers.dedup();
    let mut sorted = story_ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if story_ids.len() == story_count
        && sorted.len() == story_ids.len()
        && numbers.iter().all(|n| story_ids.contains(n))
    {
        story_ids.to_vec()
    } else {
        numbers
    }
}

impl Document {
    /// Story numbers in story order (see the file's story mapping): `story_numbers()[k]`
    /// is the story number of `stories[k]`.
    pub fn story_numbers(&self) -> Vec<u32> {
        story_order(&self.pages, &self.masters, &self.story_ids, self.stories.len())
    }
}

/// Fill `Shape::text`/`runs` from the shape's story number.
///
/// UNCONFIRMED: story numbers are mapped by rank. The distinct story numbers used by all
/// shapes, sorted ascending, map to story indexes 0, 1, 2, ... Fits: hello/bold (2 -> 0),
/// two-boxes (2, 3 -> 0, 1) and Tika (1, 3 -> 0, 1; STRS has 2 stories). The spec's
/// "number - 2" rule would give Tika's story 1 the index -1. Shapes whose rank has no
/// story (more numbers than stories) get no text.
fn attach_stories<'a>(
    shapes: impl Iterator<Item = &'a mut Shape>,
    numbers: &[u32],
    stories: &[Vec<Paragraph>],
    cell_ends: &[Vec<u32>],
) {
    for shape in shapes {
        let Some(n) = shape.story else { continue };
        let Some(rank) = numbers.iter().position(|&x| x == n) else {
            continue;
        };
        let Some(paras) = stories.get(rank) else {
            continue;
        };
        shape.text = Some(story_text(paras));
        shape.runs = merged_runs(paras);
        shape.paragraphs = paras.clone();
        if let Some(t) = &mut shape.table {
            fill_table(t, paras, cell_ends.get(rank).map_or(&[][..], Vec::as_slice));
        }
    }
}

/// Column widths and row heights from item 109 (spec 4E, 4E.1.1): the first `columns`
/// records are the columns, then `rows` records. Item 1 of a record is the running edge
/// from the table's left/top and item 2 the size; after a row auto-grows only the running
/// edges are updated (item 2 stays stale, `table-2x2-grown`), so the sizes are the
/// differences of the edges; item 2 is used only when a track has no edge. `None` when
/// the list is short or a size is missing or zero.
fn grid_sizes(items: &[items::Item], rows: u32, columns: u32) -> Option<(Vec<u32>, Vec<u32>)> {
    let items::Value::List(list) = &items::find(items, ITEM_TABLE_GRID)?.value else {
        return None;
    };
    let (rows, columns) = (rows as usize, columns as usize);
    if rows == 0 || columns == 0 || list.len() < rows.checked_add(columns)? {
        return None;
    }
    let field = |rec: &items::Item, id| match &rec.value {
        items::Value::Record(f) => items::find_int(f, id).map(|v| v as u32),
        _ => None,
    };
    let track = |recs: &[items::Item]| -> Option<Vec<u32>> {
        let mut prev = 0u32;
        recs.iter()
            .map(|rec| {
                let size = match field(rec, 1) {
                    Some(edge) if edge > prev => edge - prev,
                    Some(_) => return None,
                    None => field(rec, 2)?,
                };
                prev = prev.checked_add(size)?;
                (size > 0).then_some(size)
            })
            .collect()
    };
    let cols = track(&list[..columns])?;
    let rws = track(&list[columns..columns + rows])?;
    Some((cols, rws))
}

/// Cell margins and spans from the kind-99 object named by item 107 (spec 4E). `None`
/// unless it lists exactly `rows * columns` cells with spans inside the grid.
fn cell_props(
    c: &contents::Contents,
    items: &[items::Item],
    rows: u32,
    columns: u32,
) -> Option<Vec<CellProps>> {
    let entry = c.get(items::find_int(items, ITEM_TABLE_CELLS)? as u32)?;
    if entry.kind != KIND_TABLE_CELLS {
        return None;
    }
    let decoded = c.decode(entry).ok()?;
    let items::Value::List(list) = &items::find(&decoded, 2)?.value else {
        return None;
    };
    if list.len() as u64 != u64::from(rows) * u64::from(columns) {
        return None;
    }
    list.iter()
        .map(|rec| {
            let items::Value::Record(f) = &rec.value else {
                return None;
            };
            let span = |id| items::find_int(f, id).unwrap_or(0) as u32;
            let margin = |id| items::find_int(f, id).map_or(DEFAULT_CELL_MARGIN_EMU, |v| v as u32);
            let p = CellProps {
                first_row: span(1),
                last_row: span(2),
                first_column: span(3),
                last_column: span(4),
                margin_left_emu: margin(10),
                margin_top_emu: margin(11),
                margin_right_emu: margin(12),
                margin_bottom_emu: margin(13),
            };
            (p.first_row <= p.last_row
                && p.last_row < rows
                && p.first_column <= p.last_column
                && p.last_column < columns)
                .then_some(p)
        })
        .collect()
}

/// Fill the table's cell text. The `TCD ` chunk is `cell_ends` (the one whose instance is
/// the story's ordinal, spec 4E.1.3); when it fits the story each cell keeps its
/// paragraphs, otherwise fall back to one paragraph per cell. The story holds the cells in
/// creation order, so with cell records (`t.cell_props`, in the same order) every cell is
/// moved to the slot `row * columns + column` its record names (spec 4E.1.5).
fn fill_table(t: &mut Table, paras: &[Paragraph], cell_ends: &[u32]) {
    let wanted = (t.rows as usize)
        .saturating_mul(t.columns as usize)
        .min(1 << 16);
    if let Some(mut groups) = split_cells(paras, cell_ends, wanted) {
        if let Some(slots) = cell_slots(&t.cell_props, t.rows, t.columns) {
            let mut placed: Vec<Vec<Paragraph>> = vec![Vec::new(); wanted];
            let mut props = t.cell_props.clone();
            for (k, &slot) in slots.iter().enumerate() {
                placed[slot] = std::mem::take(&mut groups[k]);
                props[slot] = t.cell_props[k];
            }
            groups = placed;
            t.cell_props = props;
        } else {
            t.cell_props.clear(); // no reliable positions: do not pair props with cells
        }
        t.cells = groups
            .iter()
            .map(|g| {
                let mut s: String = g
                    .iter()
                    .flat_map(|p| p.runs.iter().map(|r| r.text.as_str()))
                    .collect();
                if s.ends_with('\r') {
                    s.pop();
                }
                s
            })
            .collect();
        t.cell_paragraphs = groups;
        return;
    }
    t.cells = table_cells(paras, t.rows, t.columns);
    t.cell_paragraphs = (0..t.cells.len())
        .map(|i| paras.get(i).cloned().into_iter().collect())
        .collect();
    t.cell_props.clear();
}

/// The row-major slot of every cell record, or `None` unless the records are one per slot
/// (each record's first row/column is a distinct slot inside the grid).
fn cell_slots(props: &[CellProps], rows: u32, columns: u32) -> Option<Vec<usize>> {
    let total = (rows as usize).checked_mul(columns as usize)?;
    if props.len() != total {
        return None;
    }
    let mut seen = vec![false; total];
    props
        .iter()
        .map(|p| {
            let slot = (p.first_row as usize)
                .checked_mul(columns as usize)?
                .checked_add(p.first_column as usize)?;
            (slot < total && !std::mem::replace(&mut seen[slot], true)).then_some(slot)
        })
        .collect()
}

/// Split a story's paragraphs into cells at the `TCD ` end positions (spec 4E). `None`
/// when the count is not `wanted`, an end is not the position of a `\r`, ends do not
/// increase, or the last end is not the story length.
fn split_cells(paras: &[Paragraph], ends: &[u32], wanted: usize) -> Option<Vec<Vec<Paragraph>>> {
    if ends.len() != wanted || wanted == 0 {
        return None;
    }
    let text: Vec<u16> = paras
        .iter()
        .flat_map(|p| p.runs.iter().flat_map(|r| r.text.encode_utf16()))
        .collect();
    let last = *ends.last()? as usize;
    if last != text.len() {
        return None;
    }
    let mut prev: i64 = -1;
    for &e in &ends[..ends.len() - 1] {
        let e = e as usize;
        if (e as i64) <= prev || text.get(e) != Some(&0x0D) {
            return None;
        }
        prev = e as i64;
    }
    let mut out: Vec<Vec<Paragraph>> = vec![Vec::new(); wanted];
    let mut start = 0usize;
    for p in paras {
        let cell = ends[..ends.len() - 1].partition_point(|&e| (e as usize) < start);
        out[cell].push(p.clone());
        start += p
            .runs
            .iter()
            .map(|r| r.text.encode_utf16().count())
            .sum::<usize>();
    }
    out.iter().all(|c| !c.is_empty()).then_some(out)
}

/// Fallback cells of a table: the story text split at `\r`, row-major.
fn table_cells(paras: &[Paragraph], rows: u32, columns: u32) -> Vec<String> {
    let text = story_text(paras);
    let mut cells: Vec<String> = text.split('\r').map(str::to_owned).collect();
    if text.ends_with('\r') {
        cells.pop(); // the empty piece after the final terminator
    }
    let wanted = (rows as usize)
        .saturating_mul(columns as usize)
        .min(1 << 16);
    if cells.len() < wanted {
        cells.resize(wanted, String::new());
    }
    cells
}

fn story_text(paras: &[Paragraph]) -> String {
    paras
        .iter()
        .flat_map(|p| p.runs.iter().map(|r| r.text.as_str()))
        .collect()
}

/// All runs of a story in order, merging neighbours (even across paragraphs) whose
/// attributes all match.
fn merged_runs(paras: &[Paragraph]) -> Vec<TextRun> {
    let mut out: Vec<TextRun> = Vec::new();
    for run in paras.iter().flat_map(|p| &p.runs) {
        match out.last_mut() {
            Some(last)
                if last.bold == run.bold
                    && last.italic == run.italic
                    && last.size_pt == run.size_pt
                    && last.font == run.font
                    && last.underline == run.underline
                    && last.script == run.script
                    && last.color == run.color
                    && last.field.is_none()
                    && run.field.is_none() =>
            {
                last.text.push_str(&run.text);
            }
            _ => out.push(run.clone()),
        }
    }
    out
}

/// Page size (spec 4.5): kind 68, else kind 76, else (caller) Letter.
///
/// UNCONFIRMED: the spec says the width/height are the object's top-level items 1 and 2,
/// but in every sample (e.g. gen/legal.pub, kind-68 object 256) the top-level items 1 and 2
/// are a count and a list; the 7772400 x 12801600 pair is in the nested record at item 18
/// (kind 68) and item 3 (kind 76). We take the first record, searched depth-first, whose
/// items 1 and 2 are both 4-byte integers.
fn page_size(c: &contents::Contents) -> Option<(u32, u32)> {
    fn search(items: &[items::Item]) -> Option<(u32, u32)> {
        let pair = |id| items::find(items, id).filter(|i| i.class == 4)?.as_int();
        if let (Some(w), Some(h)) = (pair(1), pair(2)) {
            return Some((w as u32, h as u32));
        }
        items.iter().find_map(|it| match &it.value {
            items::Value::Record(v) | items::Value::List(v) => search(v),
            _ => None,
        })
    }
    let of_kind = |kind| {
        c.objects
            .iter()
            .filter(|e| e.kind == kind)
            .find_map(|e| search(&c.decode(e).ok()?))
    };
    of_kind(KIND_SETTINGS).or_else(|| of_kind(KIND_PAGE_SIZE))
}

/// The master page's four margin guides (spec 4Q.6): kind 76, item 2 = list of four records
/// {1: position from the top-left (absent = 0)} in the order left, right, top, bottom; the
/// right and bottom positions are `W - margin` and `H - margin`.
fn margins(c: &contents::Contents, w: u32, h: u32) -> Option<[u32; 4]> {
    c.objects.iter().filter(|e| e.kind == KIND_PAGE_SIZE).find_map(|e| {
        let items = c.decode(e).ok()?;
        let items::Value::List(els) = &items::find(&items, 2)?.value else {
            return None;
        };
        if els.len() != 4 {
            return None;
        }
        let mut pos = [0u32; 4];
        for (p, el) in pos.iter_mut().zip(els) {
            let items::Value::Record(v) = &el.value else {
                return None;
            };
            *p = items::find_int(v, 1).unwrap_or(0) as u32;
        }
        Some([pos[0], w.checked_sub(pos[1])?, pos[2], h.checked_sub(pos[3])?])
    })
}

enum Target {
    Page(usize),
    Master(usize),
}

fn master_of(c: &contents::Contents, mut parent: Option<u32>, masters: &[MasterPage]) -> Option<usize> {
    for _ in 0..MAX_PARENT_HOPS {
        let n = parent?;
        if let Some(ix) = masters.iter().position(|m| m.object_number == n) {
            return Some(ix);
        }
        parent = c.get(n)?.parent;
    }
    None
}

/// The page-number sections (spec 4T.4): the kind-117 object named by item 47 of the
/// settings object (kind 68). Empty when the file has none.
fn sections(c: &contents::Contents) -> Result<Vec<Section>, PubError> {
    let Some(settings) = c.objects.iter().find(|e| e.kind == KIND_SETTINGS) else {
        return Ok(Vec::new());
    };
    let Some(id) = items::find_int(&c.decode(settings)?, ITEM_SECTIONS) else {
        return Ok(Vec::new());
    };
    let Some(entry) = c.get(id as u32).filter(|e| e.kind == KIND_SECTIONS) else {
        return Ok(Vec::new());
    };
    let decoded = c.decode(entry)?;
    let Some(items::Value::List(list)) = items::find(&decoded, 2).map(|i| &i.value) else {
        return Ok(Vec::new());
    };
    Ok(list
        .iter()
        .filter_map(|rec| {
            let items::Value::Record(f) = &rec.value else {
                return None;
            };
            let flag = |id| items::find(f, id).is_some_and(|i| i.value != items::Value::Int(0));
            Some(Section {
                start_page: items::find_int(f, 1).unwrap_or(1).clamp(1, u64::from(u32::MAX)) as u32,
                first_number: items::find_int(f, 2).map(|v| v as u32),
                format: NumberFormat::from_stored(items::find_int(f, 3).unwrap_or(0)),
                hide_first_page_hf: flag(4) || flag(5),
            })
        })
        .collect())
}

fn page_of(c: &contents::Contents, mut parent: Option<u32>, pages: &[Page]) -> Option<usize> {
    for _ in 0..MAX_PARENT_HOPS {
        let n = parent?;
        if let Some(ix) = pages.iter().position(|p| p.object_number == n) {
            return Some(ix);
        }
        parent = c.get(n)?.parent;
    }
    None
}

/// A font object holds item 2, a list of records; item 4 of each is the UTF-16 name.
fn collect_font_names(items: &[items::Item], out: &mut Vec<String>) {
    if let Some(items::Item {
        value: items::Value::List(list),
        ..
    }) = items::find(items, 2)
    {
        for rec in list {
            if let items::Value::Record(fields) = &rec.value {
                if let Some(items::Value::Raw(raw)) = items::find(fields, 4).map(|i| &i.value) {
                    // Names are NUL-terminated.
                    let end = raw.len() - raw.len() % 2;
                    let end = (0..end)
                        .step_by(2)
                        .find(|&i| raw[i] == 0 && raw[i + 1] == 0);
                    out.push(quill::utf16le(&raw[..end.unwrap_or(raw.len())]));
                }
            }
        }
    }
}

/// Where the wrap type (`Contents` item 4, spec 4K) of the object lives; `None` when the
/// object has no such item (pictures).
pub fn wrap_slot(streams: &Streams, object_number: u32) -> Result<Option<IntSlot>, PubError> {
    let contents = contents::Contents::parse(&streams.contents)?;
    let Some(entry) = contents.get(object_number) else {
        return Ok(None);
    };
    items::locate_int(&streams.contents, entry.offset, ITEM_WRAP)
}

/// Byte positions, in the raw streams, of the integers that place one shape. Read-only
/// support for writers that patch a file in place without changing any length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeSlots {
    /// `Contents` object kind.
    pub kind: u16,
    pub rotation_deg: f32,
    /// Anchor items 1..=4 (left, top, right, bottom) in `Escher/EscherStm`.
    pub anchor: [Option<IntSlot>; 4],
    /// `Contents` items 170 / 171 (width, height).
    pub width: Option<IntSlot>,
    pub height: Option<IntSlot>,
    /// For a child of a group (spec 4J): its child anchor and the group mapping. The
    /// client anchor slots are then all `None`.
    pub child: Option<ChildInfo>,
    /// For a group shape (spec 4J): offset in `Escher/EscherStm` of its coordinate box
    /// (FSPGR: left, top, right, bottom as little-endian i32).
    pub group_box: Option<usize>,
}

/// Locate the placement integers of the shape with `Contents` object number
/// `object_number`. `Ok(None)` when the object or its Escher anchor does not exist.
pub fn shape_slots(streams: &Streams, object_number: u32) -> Result<Option<ShapeSlots>, PubError> {
    let contents = contents::Contents::parse(&streams.contents)?;
    let Some(entry) = contents.get(object_number) else {
        return Ok(None);
    };
    let Some(loc) = escher::locate_anchor(&streams.escher, object_number)? else {
        let esc = escher::read(&streams.escher)?;
        let Some(es) = esc
            .shapes
            .iter()
            .find(|s| s.contents_object == object_number && s.child.is_some())
        else {
            return Ok(None);
        };
        return Ok(Some(ShapeSlots {
            kind: entry.kind,
            rotation_deg: es.child.map_or(0.0, |c| c.own_rotation),
            anchor: [None; 4],
            width: items::locate_int(&streams.contents, entry.offset, ITEM_WIDTH)?,
            height: items::locate_int(&streams.contents, entry.offset, ITEM_HEIGHT)?,
            child: es.child,
            group_box: None,
        }));
    };
    let mut anchor = [None; 4];
    for (i, slot) in anchor.iter_mut().enumerate() {
        *slot = items::locate_int(&streams.escher, loc.block_offset, i as u16 + 1)?;
    }
    Ok(Some(ShapeSlots {
        kind: entry.kind,
        rotation_deg: loc.rotation_deg,
        anchor,
        width: items::locate_int(&streams.contents, entry.offset, ITEM_WIDTH)?,
        height: items::locate_int(&streams.contents, entry.offset, ITEM_HEIGHT)?,
        child: None,
        group_box: loc.fspgr_offset,
    }))
}
