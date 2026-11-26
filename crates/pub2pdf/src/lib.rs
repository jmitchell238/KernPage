//! Render a [`pub_format::Document`] to PDF.
//!
//! Text is drawn with real TrueType fonts when the run's font is found in the configured
//! font directories (see [`Options::font_dirs`]): the face is embedded as a subset
//! (CIDFontType2, Identity-H, with a ToUnicode map) and its real advance widths drive
//! wrapping and alignment. Otherwise the standard-14 fonts are used.
//!
//! Limitations: standard-14 fallback fonts use approximated advance widths and only
//! Latin-1 plus a few WinAnsi punctuation characters (others become `?`); fonts are
//! chosen by family name only (Helvetica, Times or Courier by name for the fallback;
//! no kerning, ligatures or complex-script shaping; a missing glyph shows the font's
//! `.notdef`); the font's embedding permission (OS/2 fsType) is honoured, CFF-flavoured
//! OpenType fonts are never embedded; text that overflows its box is neither clipped nor
//! shrunk; PNG and JPEG pictures are embedded stretched to their box, other formats and
//! undecodable images become a gray placeholder; tables are drawn with their real column widths,
//! row heights and cell margins (equal cells when the file has no grid), cell text as normal paragraphs; scheme colours are mapped by
//! the document's colour scheme (a fixed guess only when it has none); shapes are positioned by their anchor box only
//! (no rotation, flipping, dashes, arrowheads or gradients).

mod fonts;
mod meta;

use fonts::{EmbFont, FontDb};
use miniz_oxide::deflate::compress_to_vec_zlib;
use pdf_writer::types::{FontFlags, LineCapStyle, SystemInfo, UnicodeCmap};
use pdf_writer::{Content, Filter, Name, Pdf, Rect, Ref, Str};
use pub_format::{
    Alignment, Color, ColorScheme, Document, LineSpacing, ListInfo, ParaSpacing, Paragraph, PictureFormat,
    Shape, ShapeGeometry,
};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub use fonts::{FontDb as Fonts, FontFace};

const EMU_PER_POINT: f32 = 12_700.0; // 914,400 EMU per inch / 72 pt per inch
const LINE_SPACING: f32 = 1.2;
/// Baseline distance below the top of a line, as a fraction of the font size.
const ASCENT: f32 = 0.9;
/// Gap between a text box's edge and its text.
const INSET: f32 = 3.6;

/// Space between the stored text-frame margin and the text, in points. Measured (spec 4K,
/// `gen/wrap.pub`): a box at 72 pt with a 2.88 pt (0.04 in) margin has its lines start at
/// 75.6 pt, so the text sits 0.72 pt (0.01 in) further in than the margin says.
pub const TEXT_PAD_PT: f32 = 0.72;

/// Where text starts inside a shape: left, top, right, bottom in points; the box's own
/// text-frame margins (Escher 0x81..0x84, spec 4N.1) plus [`TEXT_PAD_PT`].
fn inset_pt(shape: &Shape) -> [f32; 4] {
    shape.inset_emu.map(|e| e as f32 / EMU_PER_POINT + TEXT_PAD_PT)
}
/// Table grid line width (UNCONFIRMED: the real border style is not decoded).
const TABLE_LINE_PT: f32 = 0.75;
/// Cap on table rows/columns drawn (corrupt-file guard).
const MAX_TABLE_DIM: u32 = 1000;
/// Circle approximation constant for Bezier ellipses.
const KAPPA: f32 = 0.552_284_8;
/// Environment variable listing font directories (path-separated).
pub const FONT_DIRS_ENV: &str = "KERNPAGE_FONT_DIRS";
/// Used when neither `--font-dir` nor the environment variable is given.
const DEFAULT_FONT_DIRS: [&str; 2] = ["C:\\Windows\\Fonts", "/mnt/c/Windows/Fonts"];

#[derive(Debug, Clone)]
pub struct Options {
    /// Draw each text box's outline in light gray (0.5 pt).
    pub outlines: bool,
    /// Directories searched for fonts. Empty = standard-14 fonts only. The default is
    /// `KERNPAGE_FONT_DIRS` if set, else the Windows font folders that exist.
    pub font_dirs: Vec<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            outlines: true,
            font_dirs: default_font_dirs(),
        }
    }
}

/// `KERNPAGE_FONT_DIRS` (path-separated) if set and non-empty, else the default Windows
/// font folders that exist.
pub fn default_font_dirs() -> Vec<PathBuf> {
    if let Some(v) = std::env::var_os(FONT_DIRS_ENV).filter(|v| !v.is_empty()) {
        return std::env::split_paths(&v)
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
    }
    DEFAULT_FONT_DIRS
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect()
}

/// Font scans are slow on network or WSL mounts, so each directory list is scanned once.
fn font_db(dirs: &[PathBuf]) -> Arc<FontDb> {
    static CACHE: OnceLock<Mutex<HashMap<Vec<PathBuf>, Arc<FontDb>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(map) = cache.lock() {
        if let Some(db) = map.get(dirs) {
            return db.clone();
        }
    }
    let db = Arc::new(FontDb::scan(dirs));
    if let Ok(mut map) = cache.lock() {
        map.insert(dirs.to_vec(), db.clone());
    }
    db
}

/// Read `input` and write the rendered PDF to `output`.
pub fn convert(input: &Path, output: &Path, opts: &Options) -> Result<(), String> {
    let doc = pub_format::read(input).map_err(|e| format!("{}: {e}", input.display()))?;
    std::fs::write(output, render(&doc, opts)).map_err(|e| format!("{}: {e}", output.display()))
}

fn color_rgb(color: Color, scheme: Option<&ColorScheme>) -> [u8; 3] {
    color.to_rgb(scheme)
}

fn unit(v: u8) -> f32 {
    f32::from(v) / 255.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Helvetica,
    Times,
    Courier,
}

impl Family {
    /// Map a Publisher font name to a standard family (serif names to Times, monospaced
    /// names to Courier, everything else to Helvetica).
    fn of(font: &str) -> Family {
        let f = font.to_ascii_lowercase();
        const MONO: [&str; 4] = ["courier", "consolas", "lucida console", "monaco"];
        const SERIF: [&str; 11] = [
            "times",
            "georgia",
            "cambria",
            "garamond",
            "palatino",
            "book antiqua",
            "bookman",
            "century",
            "constantia",
            "baskerville",
            "serif",
        ];
        if MONO.iter().any(|m| f.contains(m)) {
            Family::Courier
        } else if SERIF.iter().any(|s| f.contains(s)) && !f.contains("sans") {
            Family::Times
        } else {
            Family::Helvetica
        }
    }

    fn base_font(self, bold: bool, italic: bool) -> &'static str {
        match (self, bold, italic) {
            (Family::Helvetica, false, false) => "Helvetica",
            (Family::Helvetica, true, false) => "Helvetica-Bold",
            (Family::Helvetica, false, true) => "Helvetica-Oblique",
            (Family::Helvetica, true, true) => "Helvetica-BoldOblique",
            (Family::Times, false, false) => "Times-Roman",
            (Family::Times, true, false) => "Times-Bold",
            (Family::Times, false, true) => "Times-Italic",
            (Family::Times, true, true) => "Times-BoldItalic",
            (Family::Courier, false, false) => "Courier",
            (Family::Courier, true, false) => "Courier-Bold",
            (Family::Courier, false, true) => "Courier-Oblique",
            (Family::Courier, true, true) => "Courier-BoldOblique",
        }
    }
}

/// Which font program draws a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FontRef {
    /// A standard-14 font.
    Std(Family),
    /// An embedded face: index into `Resources::embedded`.
    Emb(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Style {
    font: FontRef,
    bold: bool,
    italic: bool,
    size: f32,
    /// Baseline shift in points (superscript up, subscript down; spec 4S.3).
    raise: f32,
    underline: bool,
    color: [u8; 3],
}

#[derive(Debug, Clone, Copy)]
struct Glyph {
    ch: char,
    style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageColor {
    Gray,
    Rgb,
    Cmyk,
}

/// A picture ready to be written as an image XObject.
struct ImageDef {
    id: Ref,
    width: i32,
    height: i32,
    color: ImageColor,
    /// `true` = `data` is a JPEG file (DCTDecode); `false` = zlib-compressed samples.
    jpeg: bool,
    data: Vec<u8>,
    /// Soft mask (alpha) image: its object and zlib-compressed 8-bit gray samples.
    smask: Option<(Ref, Vec<u8>)>,
}

/// Objects discovered while drawing: fonts and images, plus the object-number counter.
struct Resources {
    fonts: Vec<&'static str>,
    images: Vec<ImageDef>,
    next_ref: i32,
    font_dirs: Vec<PathBuf>,
    db: Option<Arc<FontDb>>,
    embedded: Vec<EmbFont>,
    /// Loaded faces by (path, face index).
    loaded: HashMap<(PathBuf, u32), Option<usize>>,
    /// Font choice by (lower-cased name, bold, italic).
    chosen: HashMap<(String, bool, bool), FontRef>,
    /// The document's colour scheme, for resolving scheme colours.
    scheme: Option<ColorScheme>,
}

impl Resources {
    fn new(first_ref: i32, font_dirs: Vec<PathBuf>) -> Resources {
        Resources {
            fonts: Vec::new(),
            images: Vec::new(),
            next_ref: first_ref,
            font_dirs,
            db: None,
            embedded: Vec::new(),
            loaded: HashMap::new(),
            chosen: HashMap::new(),
            scheme: None,
        }
    }

    fn alloc(&mut self) -> Ref {
        let r = Ref::new(self.next_ref);
        self.next_ref += 1;
        r
    }

    /// Index of a standard font in the resource list.
    fn std_index(&mut self, family: Family, bold: bool, italic: bool) -> usize {
        let name = family.base_font(bold, italic);
        match self.fonts.iter().position(|&f| f == name) {
            Some(i) => i,
            None => {
                self.fonts.push(name);
                self.fonts.len() - 1
            }
        }
    }

    /// Pick the font for a run: an embedded system face if one is found and may be
    /// embedded, else the standard-14 stand-in.
    fn resolve_font(&mut self, name: &str, bold: bool, italic: bool) -> FontRef {
        let key = (name.to_lowercase(), bold, italic);
        if let Some(f) = self.chosen.get(&key) {
            return *f;
        }
        let fallback = FontRef::Std(Family::of(name));
        let mut choice = fallback;
        if !self.font_dirs.is_empty() && !name.trim().is_empty() {
            let dirs = &self.font_dirs;
            let db = self.db.get_or_insert_with(|| font_db(dirs)).clone();
            if let Some(face) = db.find(name, bold, italic).filter(|f| f.embeddable) {
                let slot = (face.path.clone(), face.index);
                let ix = match self.loaded.get(&slot) {
                    Some(ix) => *ix,
                    None => {
                        let ix = EmbFont::load(face).map(|f| {
                            self.embedded.push(f);
                            self.embedded.len() - 1
                        });
                        self.loaded.insert(slot, ix);
                        ix
                    }
                };
                if let Some(ix) = ix {
                    choice = FontRef::Emb(ix);
                }
            }
        }
        self.chosen.insert(key, choice);
        choice
    }

    /// `(natural line height, ascent)` in points of a line set in `s`: from the font's
    /// metrics (ascent + descent + line gap) for an embedded font, else the stand-in
    /// proportions.
    fn metrics(&self, s: &Style) -> (f32, f32) {
        match s.font {
            FontRef::Emb(i) => {
                let f = &self.embedded[i];
                (f.line_height / 1000.0 * s.size, f.ascent / 1000.0 * s.size)
            }
            FontRef::Std(_) => (LINE_SPACING * s.size, ASCENT * s.size),
        }
    }

    /// Advance width of `ch` in points.
    fn char_width(&mut self, ch: char, s: &Style) -> f32 {
        match s.font {
            FontRef::Emb(i) => self.embedded[i].glyph(ch).advance_em * s.size,
            FontRef::Std(family) => std_char_width(ch, family, s),
        }
    }
}

/// Render every page of `doc` to PDF bytes (content streams are uncompressed).
pub fn render(doc: &Document, opts: &Options) -> Vec<u8> {
    let catalog_id = Ref::new(1);
    let tree_id = Ref::new(2);
    let (w_pt, h_pt) = (
        doc.page_width_emu as f32 / EMU_PER_POINT,
        doc.page_height_emu as f32 / EMU_PER_POINT,
    );
    // Two objects per page, numbered after the fixed ones; fonts and images follow.
    let page_ids: Vec<(Ref, Ref)> = (0..doc.pages.len() as i32)
        .map(|i| (Ref::new(3 + 2 * i), Ref::new(4 + 2 * i)))
        .collect();
    let mut res = Resources::new(3 + 2 * doc.pages.len() as i32, opts.font_dirs.clone());
    res.scheme = doc.color_scheme.clone();

    // Stories that run through linked text boxes are laid out frame by frame first.
    let (mut flowed, _) = flow_chains(doc, (w_pt, h_pt), &mut res);

    // Draw every page, remembering which images (indexes into `res.images`) it uses.
    let mut pages: Vec<(Vec<u8>, Vec<usize>)> = Vec::new();
    for (pi, page) in doc.pages.iter().enumerate() {
        let mut content = Content::new();
        let mut used_images = Vec::new();
        let number = pub_format::page_number_text(doc, pi);
        // The page's master is drawn behind its own shapes (spec 4T.6); a page-number field
        // shows the number of the page being drawn.
        let master = page
            .master
            .filter(|_| !page.ignore_master)
            .and_then(|m| doc.masters.get(m));
        for shape in master.into_iter().flat_map(|m| &m.shapes) {
            let shown = shape.with_page_number(&number);
            draw_shape(
                &mut content,
                shown.as_ref().unwrap_or(shape),
                (w_pt, h_pt),
                opts,
                &mut res,
                &mut used_images,
                None,
            );
        }
        for (si, shape) in page.shapes.iter().enumerate() {
            let shown = shape.with_page_number(&number);
            draw_shape(
                &mut content,
                shown.as_ref().unwrap_or(shape),
                (w_pt, h_pt),
                opts,
                &mut res,
                &mut used_images,
                flowed.remove(&(pi, si)),
            );
        }
        pages.push((content.finish().to_vec(), used_images));
    }

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id)
        .kids(page_ids.iter().map(|p| p.0))
        .count(page_ids.len() as i32);
    let font_count = res.fonts.len();
    let font_refs: Vec<Ref> = (0..font_count).map(|_| res.alloc()).collect();
    for (name, &id) in res.fonts.iter().zip(&font_refs) {
        pdf.type1_font(id)
            .base_font(Name(name.as_bytes()))
            .encoding_predefined(Name(b"WinAnsiEncoding"));
    }
    let emb_refs = write_embedded_fonts(&mut pdf, &mut res);
    for img in &res.images {
        if let Some((mask_id, mask)) = &img.smask {
            let mut x = pdf.image_xobject(*mask_id, mask);
            x.width(img.width)
                .height(img.height)
                .bits_per_component(8)
                .filter(Filter::FlateDecode);
            x.color_space().device_gray();
            drop(x);
        }
        let mut x = pdf.image_xobject(img.id, &img.data);
        x.width(img.width).height(img.height).bits_per_component(8);
        x.filter(if img.jpeg {
            Filter::DctDecode
        } else {
            Filter::FlateDecode
        });
        match img.color {
            ImageColor::Gray => x.color_space().device_gray(),
            ImageColor::Rgb => x.color_space().device_rgb(),
            ImageColor::Cmyk => {
                x.color_space().device_cmyk();
                // Adobe CMYK JPEGs store inverted values.
                x.decode([1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);
            }
        }
        if let Some((mask_id, _)) = &img.smask {
            x.s_mask(*mask_id);
        }
        drop(x);
    }
    for ((content, used_images), &(page_id, content_id)) in pages.iter().zip(&page_ids) {
        pdf.stream(content_id, content);
        let mut p = pdf.page(page_id);
        p.parent(tree_id)
            .media_box(Rect::new(0.0, 0.0, w_pt, h_pt))
            .contents(content_id);
        let mut resources = p.resources();
        let mut fonts = resources.fonts();
        for (i, &id) in font_refs.iter().enumerate() {
            fonts.pair(Name(format!("F{}", i + 1).as_bytes()), id);
        }
        for (i, &id) in emb_refs.iter().enumerate() {
            fonts.pair(Name(format!("E{}", i + 1).as_bytes()), id);
        }
        drop(fonts);
        if !used_images.is_empty() {
            let mut xo = resources.x_objects();
            for &i in used_images {
                xo.pair(Name(format!("Im{}", i + 1).as_bytes()), res.images[i].id);
            }
        }
    }
    pdf.finish()
}

/// Position in a story: (paragraph index, glyph offset inside that paragraph).
pub type StoryPos = (usize, usize);

/// Laid-out lines per (page, shape) index, one `Vec<Line>` per text column.
type FlowedLines = HashMap<(usize, usize), Vec<Vec<Line>>>;

/// Column geometry of a frame (spec 4I): `(count, column width, pitch)` in points, where
/// the pitch is the distance from one column's left edge to the next. The width is
/// `(frame width - insets - (n - 1) * gap) / n`.
fn column_geometry(columns: u32, gap_pt: f32, frame_width: f32, inset: [f32; 4]) -> (usize, f32, f32) {
    let n = columns.max(1) as usize;
    let inner = (frame_width - inset[0] - inset[2]).max(0.0);
    let width = ((inner - (n - 1) as f32 * gap_pt) / n as f32).max(0.0);
    (n, width, width + gap_pt)
}

/// Everything the text layout needs to know about one text frame: where it is, how big,
/// its columns and the regions text wraps around. Build it from a document with
/// [`FrameGeom::of_shape`] (the same numbers [`render`] uses) and hand a chain of them to
/// [`flow_story`] or [`FlowContext::flow`].
#[derive(Debug, Clone, PartialEq)]
pub struct FrameGeom {
    /// The frame's top-left corner in points from the page's top-left, unrotated.
    pub origin: (f32, f32),
    /// Width and height of the frame in points (insets included).
    pub size: (f32, f32),
    /// Number of text columns (at least 1).
    pub columns: u32,
    /// Space between columns in points.
    pub column_gap_pt: f32,
    /// Text-frame margins left, top, right, bottom in points.
    pub inset_pt: [f32; 4],
    /// Regions the text wraps around, in the frame's unrotated coordinates.
    pub obstacles: Vec<pub_format::wrap::Poly>,
}

impl FrameGeom {
    fn new(shape: &Shape, page: (f32, f32), obstacles: Vec<pub_format::wrap::Poly>) -> FrameGeom {
        let (l, b, r, t) = shape_rect(shape, page);
        FrameGeom {
            origin: (l, page.1 - t),
            size: (r - l, t - b),
            columns: shape.columns.max(1),
            column_gap_pt: shape.column_gap_emu as f32 / EMU_PER_POINT,
            inset_pt: inset_pt(shape),
            obstacles,
        }
    }

    /// The geometry of shape `shape` on page `page` of `doc`, with the wrap obstacles the
    /// PDF layout uses for it; `None` when there is no such shape.
    pub fn of_shape(doc: &Document, page: usize, shape: usize) -> Option<FrameGeom> {
        let s = doc.pages.get(page)?.shapes.get(shape)?;
        let size = (
            doc.page_width_emu as f32 / EMU_PER_POINT,
            doc.page_height_emu as f32 / EMU_PER_POINT,
        );
        Some(FrameGeom::new(s, size, doc.wrap_obstacles(page, shape)))
    }
}

/// Fill the columns of one frame, left to right, from story position `start`. Returns
/// the lines of each column and where the next frame resumes.
fn layout_columns(
    story: &[Paragraph],
    start: StoryPos,
    frame: &FrameGeom,
    res: &mut Resources,
) -> (Vec<Vec<Line>>, StoryPos) {
    let (n, width, pitch) = column_geometry(frame.columns, frame.column_gap_pt, frame.size.0, frame.inset_pt);
    let height = (frame.size.1 - frame.inset_pt[1] - frame.inset_pt[3]).max(0.0);
    let mut cursor = start;
    let mut cols = Vec::with_capacity(n);
    // `origin` is the frame's top-left in page points, unrotated (as are the obstacles).
    let (fl, ft) = frame.origin;
    for i in 0..n {
        let ctx = WrapCtx {
            left: fl + frame.inset_pt[0] + i as f32 * pitch,
            top: ft + frame.inset_pt[1],
            obstacles: &frame.obstacles,
            width,
            height,
        };
        let (lines, next) = layout(story, cursor, width, Some(height), Some(&ctx), res);
        cursor = next;
        cols.push(lines);
    }
    (cols, cursor)
}

/// Lay out every chain of two or more linked text boxes (spec 4G), and every multi-column
/// box (spec 4I): the story of the first box fills each frame line by line, inside its
/// insets and column by column, and continues in the next frame;
/// text left after the last frame is dropped. Returns the lines to draw per
/// (page, shape) and, per chain, the story position where each frame's text ends.
fn flow_chains(
    doc: &Document,
    page: (f32, f32),
    res: &mut Resources,
) -> (FlowedLines, Vec<Vec<StoryPos>>) {
    let mut lines = HashMap::new();
    let mut breaks = Vec::new();
    for chain in doc.chains() {
        let shape_of = |r: &pub_format::ShapeRef| &doc.pages[r.page].shapes[r.shape];
        let frames: Vec<FrameGeom> = chain
            .iter()
            .map(|r| FrameGeom::new(shape_of(r), page, doc.wrap_obstacles(r.page, r.shape)))
            .collect();
        // A lone box needs this only for its columns or text wrap.
        if chain.len() < 2
            && shape_of(&chain[0]).columns < 2
            && frames.iter().all(|f| f.obstacles.is_empty())
        {
            continue;
        }
        let Some(story) = chain
            .iter()
            .map(shape_of)
            .find(|s| !s.paragraphs.is_empty())
            .map(|s| &s.paragraphs)
        else {
            continue;
        };
        let mut cursor = (0, 0);
        let mut ends = Vec::new();
        for (r, geom) in chain.iter().zip(&frames) {
            let (frame, next) = layout_columns(story, cursor, geom, res);
            cursor = next;
            ends.push(next);
            lines.insert((r.page, r.shape), frame);
        }
        if chain.len() >= 2 {
            breaks.push(ends);
        }
    }
    (lines, breaks)
}

/// Re-usable state for [`FlowContext::flow`]: loaded fonts and measured font choices are
/// kept between calls, so flowing a story again after every edit costs only the layout.
pub struct FlowContext {
    res: Resources,
}

impl FlowContext {
    pub fn new(opts: &Options) -> FlowContext {
        FlowContext {
            res: Resources::new(1, opts.font_dirs.clone()),
        }
    }

    /// Where each of `frames` ends its text when `story` is laid out through them in
    /// order: story positions (paragraph index, glyph offset), the same line breaking and
    /// metrics as [`render`] and [`chain_breaks`]. A frame that ends exactly at a paragraph
    /// boundary reports (next paragraph, 0); the text after the last end overflows.
    pub fn flow(&mut self, frames: &[FrameGeom], story: &[Paragraph]) -> Vec<StoryPos> {
        let mut cursor = (0, 0);
        frames
            .iter()
            .map(|f| {
                cursor = layout_columns(story, cursor, f, &mut self.res).1;
                cursor
            })
            .collect()
    }
}

impl FlowContext {
    /// The height in points that `paras` take when laid out in a column `width_pt` wide
    /// with no height limit (the text of a table cell): the rows with their space before
    /// and after, without the space after the last paragraph. The last row counts at its
    /// natural height, without the extra of a line-spacing multiple (spec 4E.1, UNCONFIRMED
    /// beyond two samples: a one-line Calibri 10 cell is 12.2 pt, the four-line `grown`
    /// cell 3 x 14.53 + 12.2 = 55.78 pt, both at 1.19 lines).
    pub fn text_height(&mut self, paras: &[Paragraph], width_pt: f32) -> f32 {
        let lines = layout(paras, (0, 0), width_pt.max(0.0), None, None, &mut self.res).0;
        let rows: Vec<&Line> = lines.iter().filter(|l| !l.cont).collect();
        let mut total = 0.0f32;
        let mut tail = 0.0f32;
        for (i, line) in rows.iter().enumerate() {
            let mut pitch = line.pitch;
            if i + 1 == rows.len() {
                if let Some(LineSpacing::Multiple(k)) = paras.get(line.para).map(|p| p.spacing.line) {
                    if k > 1.0 {
                        pitch /= k;
                    }
                }
            }
            total += tail + line.lead + line.before + pitch;
            tail = line.after;
        }
        total
    }
}

/// One-shot [`FlowContext::flow`] (builds the font state each call; keep a `FlowContext`
/// when flowing repeatedly).
pub fn flow_story(frames: &[FrameGeom], story: &[Paragraph], opts: &Options) -> Vec<StoryPos> {
    FlowContext::new(opts).flow(frames, story)
}

/// Where each frame of every linked chain of `doc` ends its text, as story positions
/// (paragraph index, glyph offset in that paragraph); a frame that ends exactly at a
/// paragraph boundary reports (next paragraph, 0). One entry per chain of two or more
/// boxes, in chain order. Uses the same layout as [`render`].
pub fn chain_breaks(doc: &Document, opts: &Options) -> Vec<Vec<StoryPos>> {
    let mut res = Resources::new(1, opts.font_dirs.clone());
    res.scheme = doc.color_scheme.clone();
    let page = (
        doc.page_width_emu as f32 / EMU_PER_POINT,
        doc.page_height_emu as f32 / EMU_PER_POINT,
    );
    flow_chains(doc, page, &mut res).1
}

/// Top of each paragraph of shape `shape` on page `page` (the start of its space before),
/// in points below the top of the page, laid out as [`render`] draws it. A paragraph
/// that is empty of runs has no entry.
pub fn paragraph_tops(doc: &Document, page: usize, shape: usize, opts: &Options) -> Vec<f32> {
    let mut seen = None;
    single_box_lines(doc, page, shape, opts)
        .into_iter()
        .filter(|l| {
            let new = seen != Some(l.para) && !l.cont;
            if new {
                seen = Some(l.para);
            }
            new
        })
        .map(|l| l.start)
        .collect()
}

/// One laid-out row of a single text box: where it starts and the extent of its text.
#[derive(Debug, Clone, PartialEq)]
pub struct RowInfo {
    /// Top of the row (below its space before) in points below the top of the page.
    pub top: f32,
    /// Left edge of the first piece, points from the page's left edge.
    pub left: f32,
    /// Left edges and widths of the pieces the row's text sits in, in order.
    pub pieces: Vec<(f32, f32)>,
    /// Right end of the row's text (left-aligned text), points from the page's left edge.
    pub text_right: f32,
}

/// The rows of shape `shape` on page `page` (text wrap around objects included, spec 4K),
/// in order, as [`render`] lays them out; for a box with one column and no chain.
pub fn row_layout(doc: &Document, page: usize, shape: usize, opts: &Options) -> Vec<RowInfo> {
    let mut rows: Vec<RowInfo> = Vec::new();
    let Some(s) = doc.pages.get(page).and_then(|p| p.shapes.get(shape)) else {
        return rows;
    };
    let size = (
        doc.page_width_emu as f32 / EMU_PER_POINT,
        doc.page_height_emu as f32 / EMU_PER_POINT,
    );
    let (l, _, _, _) = shape_rect(s, size);
    for line in single_box_lines(doc, page, shape, opts) {
        let piece = (l + inset_pt(s)[0] + line.x_off, line.width.unwrap_or(line.col_width));
        let text_right = piece.0 + line.text_width;
        if line.cont {
            if let Some(r) = rows.last_mut() {
                r.pieces.push(piece);
                r.text_right = text_right;
                continue;
            }
        }
        rows.push(RowInfo {
            top: line.top,
            left: piece.0,
            pieces: vec![piece],
            text_right,
        });
    }
    rows
}

/// A [`Line`] with its position, for the layout queries above.
struct PlacedLine {
    para: usize,
    cont: bool,
    /// Start of the line's space before.
    start: f32,
    /// Top of the line proper.
    top: f32,
    x_off: f32,
    width: Option<f32>,
    col_width: f32,
    /// Width of the text without trailing spaces.
    text_width: f32,
}

fn single_box_lines(doc: &Document, page: usize, shape: usize, opts: &Options) -> Vec<PlacedLine> {
    let mut res = Resources::new(1, opts.font_dirs.clone());
    res.scheme = doc.color_scheme.clone();
    let size = (
        doc.page_width_emu as f32 / EMU_PER_POINT,
        doc.page_height_emu as f32 / EMU_PER_POINT,
    );
    let Some(s) = doc.pages.get(page).and_then(|p| p.shapes.get(shape)) else {
        return Vec::new();
    };
    let (l, _, r, t) = shape_rect(s, size);
    let inset = inset_pt(s);
    let width = (r - l - inset[0] - inset[2]).max(0.0);
    let obstacles = doc.wrap_obstacles(page, shape);
    let ctx = WrapCtx {
        left: l + inset[0],
        top: size.1 - t + inset[1],
        obstacles: &obstacles,
        width,
        height: f32::MAX,
    };
    let lines = layout(&s.paragraphs, (0, 0), width, None, Some(&ctx), &mut res).0;
    let mut cursor = size.1 - t + inset[1];
    let mut row_start = cursor;
    let mut placed = Vec::new();
    for line in &lines {
        if line.cont {
            cursor = row_start;
        } else {
            cursor += line.lead;
        }
        let start = cursor;
        cursor += line.before;
        if !line.cont {
            row_start = cursor;
        }
        placed.push(PlacedLine {
            para: line.para,
            cont: line.cont,
            start,
            top: cursor,
            x_off: line.x_off,
            width: line.width,
            col_width: width,
            text_width: {
                let end = line
                    .glyphs
                    .iter()
                    .rposition(|g| g.ch != ' ')
                    .map_or(0, |i| i + 1);
                line_width(&line.glyphs[..end], &mut res)
            },
        });
        cursor += line.pitch + line.after;
    }
    placed
}

/// Write every embedded font (Type0 + CIDFontType2 + descriptor + font file + ToUnicode)
/// and return the Type0 object of each.
fn write_embedded_fonts(pdf: &mut Pdf, res: &mut Resources) -> Vec<Ref> {
    let mut type0_refs = Vec::new();
    for ix in 0..res.embedded.len() {
        let [type0, cid, desc, file, to_unicode] = [(); 5].map(|()| res.alloc());
        let map_ref = res.alloc();
        let font = &res.embedded[ix];
        let prog = font.program();
        // Subset tag: six capitals derived from the index, as PDF requires.
        let tag: String = (0..6)
            .map(|k| char::from(b'A' + ((ix * 7 + k * 3) % 26) as u8))
            .collect();
        let base = format!("{tag}+{}", font.ps_name);
        let info = SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        };
        pdf.type0_font(type0)
            .base_font(Name(base.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid)
            .to_unicode(to_unicode);
        let mut c = pdf.cid_font(cid);
        c.subtype(pdf_writer::types::CidFontType::Type2)
            .base_font(Name(base.as_bytes()))
            .system_info(info)
            .font_descriptor(desc)
            .default_width(0.0);
        match &prog.cid_to_gid {
            Some(_) => c.cid_to_gid_map_stream(map_ref),
            None => c.cid_to_gid_map_predefined(Name(b"Identity")),
        };
        {
            let mut w = c.widths();
            for &(cid, width) in &prog.widths {
                w.consecutive(cid, [width]);
            }
        }
        drop(c);
        if let Some(map) = &prog.cid_to_gid {
            pdf.stream(map_ref, map);
        }
        let mut flags = FontFlags::NON_SYMBOLIC;
        if font.italic {
            flags |= FontFlags::ITALIC;
        }
        if font.fixed_pitch {
            flags |= FontFlags::FIXED_PITCH;
        }
        let [x0, y0, x1, y1] = font.bbox;
        pdf.font_descriptor(desc)
            .name(Name(base.as_bytes()))
            .flags(flags)
            .bbox(Rect::new(x0, y0, x1, y1))
            .italic_angle(font.italic_angle)
            .ascent(font.ascent)
            .descent(font.descent)
            .cap_height(font.cap_height)
            .stem_v(if font.bold { 140.0 } else { 80.0 })
            .font_file2(file);
        let compressed = compress_to_vec_zlib(&prog.bytes, 6);
        let mut f = pdf.stream(file, &compressed);
        f.filter(Filter::FlateDecode);
        f.pair(Name(b"Length1"), prog.bytes.len() as i32);
        drop(f);
        let mut cmap = UnicodeCmap::new(
            Name(b"Adobe-Identity-UCS"),
            SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"UCS"),
                supplement: 0,
            },
        );
        for &(cid, ch) in &prog.unicode {
            cmap.pair(cid, ch);
        }
        pdf.cmap(to_unicode, &cmap.finish());
        type0_refs.push(type0);
    }
    type0_refs
}

/// Page-space rectangle of a shape: (left, bottom, right, top), y pointing up.
fn shape_rect(shape: &Shape, (w_pt, h_pt): (f32, f32)) -> (f32, f32, f32, f32) {
    // Anchors are EMU from the page centre with y pointing down; PDF y points up.
    (
        w_pt / 2.0 + shape.left_emu as f32 / EMU_PER_POINT,
        h_pt / 2.0 - shape.bottom_emu as f32 / EMU_PER_POINT,
        w_pt / 2.0 + shape.right_emu as f32 / EMU_PER_POINT,
        h_pt / 2.0 - shape.top_emu as f32 / EMU_PER_POINT,
    )
}

/// Draw a shape; a rotated one (spec 4H) is drawn in its unrotated frame under a `cm`
/// that turns it clockwise about the frame's centre.
fn draw_shape(
    c: &mut Content,
    shape: &Shape,
    page: (f32, f32),
    opts: &Options,
    res: &mut Resources,
    used_images: &mut Vec<usize>,
    flowed: Option<Vec<Vec<Line>>>,
) {
    if shape.rotation_deg.rem_euclid(360.0) == 0.0 {
        return draw_shape_upright(c, shape, page, opts, res, used_images, flowed);
    }
    let (l, b, r, t) = shape_rect(shape, page);
    let (cx, cy) = ((l + r) / 2.0, (b + t) / 2.0);
    // PDF y points up, so a clockwise turn is a negative (counter-clockwise) angle.
    let (sin, cos) = (-shape.rotation_deg).to_radians().sin_cos();
    c.save_state().transform([
        cos,
        sin,
        -sin,
        cos,
        cx - cos * cx + sin * cy,
        cy - sin * cx - cos * cy,
    ]);
    draw_shape_upright(c, shape, page, opts, res, used_images, flowed);
    c.restore_state();
}

fn draw_shape_upright(
    c: &mut Content,
    shape: &Shape,
    page: (f32, f32),
    opts: &Options,
    res: &mut Resources,
    used_images: &mut Vec<usize>,
    flowed: Option<Vec<Vec<Line>>>,
) {
    let (left, bottom, right, top) = shape_rect(shape, page);
    let drawn = shape.fill.is_some() || shape.line.is_some();
    if drawn && shape.picture.is_none() {
        draw_geometry(c, shape, (left, bottom, right, top), res.scheme.as_ref());
    }
    // Drawn shapes without text get no debugging outline (it would sit on their edge).
    if opts.outlines && !(drawn && shape.paragraphs.is_empty() && shape.table.is_none()) {
        c.save_state()
            .set_stroke_gray(0.75)
            .set_line_width(0.5)
            .set_line_cap(LineCapStyle::ButtCap)
            .rect(left, bottom, right - left, top - bottom)
            .stroke()
            .restore_state();
    }
    if let Some(pic) = &shape.picture {
        if matches!(pic.format, PictureFormat::Wmf | PictureFormat::Emf) {
            match metafile::parse(&pic.bytes) {
                Ok(mf) => meta::draw(c, &mf, (left, bottom, right, top), res, used_images),
                Err(_) => placeholder(c, (left, bottom, right, top)),
            }
            return;
        }
        let image = match pic.format {
            PictureFormat::Png => decode_png(&pic.bytes, res),
            PictureFormat::Jpeg => decode_jpeg(&pic.bytes, res),
            PictureFormat::Wmf | PictureFormat::Emf | PictureFormat::Other(_) => None,
        };
        match image {
            Some(def) => {
                res.images.push(def);
                let ix = res.images.len() - 1;
                used_images.push(ix);
                c.save_state()
                    .transform([right - left, 0.0, 0.0, top - bottom, left, bottom])
                    .x_object(Name(format!("Im{}", ix + 1).as_bytes()))
                    .restore_state();
            }
            None => placeholder(c, (left, bottom, right, top)),
        }
        return;
    }
    if let Some(table) = &shape.table {
        draw_table(c, shape, table, (left, bottom, right, top), res);
        return;
    }
    let inset = inset_pt(shape);
    let width = (right - left - inset[0] - inset[2]).max(0.0);
    if let Some(columns) = flowed {
        let (_, col_width, pitch) = column_geometry(
            shape.columns,
            shape.column_gap_emu as f32 / EMU_PER_POINT,
            right - left,
            inset,
        );
        for (i, lines) in columns.iter().enumerate() {
            let x = left + inset[0] + i as f32 * pitch;
            draw_lines(c, lines, x, top - inset[1], col_width, res);
        }
        return;
    }
    if shape.paragraphs.is_empty() {
        return;
    }
    draw_text(c, &shape.paragraphs, left + inset[0], top - inset[1], width, res);
}

/// Grey box for a picture that cannot be drawn.
fn placeholder(c: &mut Content, (left, bottom, right, top): (f32, f32, f32, f32)) {
    c.save_state()
        .set_fill_gray(0.85)
        .set_stroke_gray(0.5)
        .set_line_width(0.5)
        .rect(left, bottom, right - left, top - bottom)
        .fill_nonzero_and_stroke()
        .restore_state();
}

/// Fill then stroke a rectangle, ellipse or line with the shape's colours and width.
fn draw_geometry(
    c: &mut Content,
    shape: &Shape,
    (l, b, r, t): (f32, f32, f32, f32),
    scheme: Option<&ColorScheme>,
) {
    let is_line = shape.geometry == Some(ShapeGeometry::Line);
    c.save_state();
    if let (Some(fill), false) = (shape.fill, is_line) {
        let [r, g, b] = color_rgb(fill, scheme);
        c.set_fill_rgb(unit(r), unit(g), unit(b));
    }
    if let Some((color, width)) = shape.line {
        let [r, g, b] = color_rgb(color, scheme);
        c.set_stroke_rgb(unit(r), unit(g), unit(b))
            .set_line_width(width as f32 / EMU_PER_POINT);
    }
    let fill = shape.fill.is_some() && !is_line;
    let stroke = shape.line.is_some();
    match shape.geometry {
        Some(ShapeGeometry::Line) => {
            // The anchor's top-left to bottom-right (a horizontal line has top = bottom);
            // exactly one flip makes it bottom-left to top-right.
            if shape.flip_h != shape.flip_v {
                c.move_to(l, b).line_to(r, t);
            } else {
                c.move_to(l, t).line_to(r, b);
            }
        }
        Some(ShapeGeometry::Ellipse) => {
            let (cx, cy, rx, ry) = ((l + r) / 2.0, (b + t) / 2.0, (r - l) / 2.0, (t - b) / 2.0);
            let (kx, ky) = (rx * KAPPA, ry * KAPPA);
            c.move_to(cx + rx, cy)
                .cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
                .cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
                .cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
                .cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
                .close_path();
        }
        // UNCONFIRMED: every other shape type is drawn as its bounding rectangle.
        _ => {
            c.rect(l, b, r - l, t - b);
        }
    }
    match (fill, stroke) {
        (true, true) => c.fill_nonzero_and_stroke(),
        (true, false) => c.fill_nonzero(),
        (false, true) => c.stroke(),
        (false, false) => c.end_path(),
    };
    c.restore_state();
}

/// Draw a table (spec 4E): real column widths and row heights when the file has them
/// (otherwise equal cells filling the frame), thin black grid lines, and each cell's
/// paragraphs laid out normally inside the cell margins. UNCONFIRMED: the real border style
/// is not decoded.
fn draw_table(
    c: &mut Content,
    shape: &Shape,
    table: &pub_format::Table,
    (l, b, r, t): (f32, f32, f32, f32),
    res: &mut Resources,
) {
    let (rows, cols) = (
        table.rows.clamp(1, MAX_TABLE_DIM) as usize,
        table.columns.clamp(1, MAX_TABLE_DIM) as usize,
    );
    // Cumulative edges from the table's left/top, in points.
    let edges = |sizes: &[u32], n: usize, total: f32| -> Vec<f32> {
        let mut e = vec![0.0f32];
        for i in 0..n {
            let step = if sizes.len() == n {
                sizes[i] as f32 / EMU_PER_POINT
            } else {
                total / n as f32
            };
            e.push(e[i] + step);
        }
        e
    };
    let xs = edges(&table.column_widths_emu, cols, r - l);
    let ys = edges(&table.row_heights_emu, rows, t - b);
    let cell_rect = |i: usize| {
        let (row, col) = (i / cols, i % cols);
        let (r0, r1, c0, c1) = match table.cell_props.get(i) {
            Some(p) => (
                p.first_row as usize,
                p.last_row as usize,
                p.first_column as usize,
                p.last_column as usize,
            ),
            None => (row, row, col, col),
        };
        (
            xs[c0.min(cols - 1)],
            ys[r0.min(rows - 1)],
            xs[c1.min(cols - 1) + 1],
            ys[r1.min(rows - 1) + 1],
        )
    };
    c.save_state()
        .set_stroke_gray(0.0)
        .set_line_width(TABLE_LINE_PT);
    for i in 0..rows * cols {
        let (x0, y0, x1, y1) = cell_rect(i);
        c.rect(l + x0, t - y1, x1 - x0, y1 - y0).stroke();
    }
    c.restore_state();
    for i in 0..rows * cols {
        let (x0, y0, x1, _) = cell_rect(i);
        let (ml, mt, mr) = match table.cell_props.get(i) {
            Some(p) => (
                p.margin_left_emu as f32 / EMU_PER_POINT,
                p.margin_top_emu as f32 / EMU_PER_POINT,
                p.margin_right_emu as f32 / EMU_PER_POINT,
            ),
            None => (INSET, INSET, INSET),
        };
        let fallback;
        let paras: &[Paragraph] = match table.cell_paragraphs.get(i) {
            Some(p) => p,
            None => {
                fallback = shape.paragraphs.get(i).cloned();
                fallback.as_slice()
            }
        };
        if !paras.is_empty() {
            draw_text(
                c,
                paras,
                l + x0 + ml,
                t - y0 - mt,
                (x1 - x0 - ml - mr).max(0.0),
                res,
            );
        }
    }
}

/// Lay out `paras` in a column of `max_width` whose top-left is (`x_left`, `y_top`).
fn draw_text(
    c: &mut Content,
    paras: &[Paragraph],
    x_left: f32,
    y_top: f32,
    max_width: f32,
    res: &mut Resources,
) {
    let lines = layout(paras, (0, 0), max_width, None, None, res).0;
    draw_lines(c, &lines, x_left, y_top, max_width, res);
}

/// Draw already laid-out lines in a column of `max_width` whose top-left is
/// (`x_left`, `y_top`).
fn draw_lines(
    c: &mut Content,
    lines: &[Line],
    x_left: f32,
    y_top: f32,
    max_width: f32,
    res: &mut Resources,
) {
    let mut cursor = y_top;
    let mut row_top = y_top;
    for line in lines {
        let x_left = x_left + line.x_off;
        let max_width = line.width.unwrap_or(max_width);
        if line.cont {
            cursor = row_top;
        } else {
            cursor -= line.lead + line.before;
            row_top = cursor;
        }
        let baseline = cursor - line.ascent;
        // Trailing spaces may hang past the edge and are never drawn.
        let end = line
            .glyphs
            .iter()
            .rposition(|g| g.ch != ' ')
            .map_or(0, |i| i + 1);
        let glyphs = &line.glyphs[..end];
        let width = line_width(glyphs, res);
        let justify = line.align == Alignment::Justify && !line.last;
        let spaces = glyphs.iter().filter(|g| g.ch == ' ').count();
        let extra = if justify && spaces > 0 {
            (max_width - width).max(0.0) / spaces as f32
        } else {
            0.0
        };
        let x0 = x_left
            + match line.align {
                Alignment::Left | Alignment::Justify => 0.0,
                Alignment::Center => (max_width - width) / 2.0,
                Alignment::Right => max_width - width,
            };
        if let Some(l) = &line.label {
            show(c, &l.text, &l.style, x_left + l.rel_x, baseline, res);
        }
        let mut x = x0;
        for (text, style) in segments(glyphs) {
            let start = x;
            if extra > 0.0 {
                // Justified: words are placed one by one so the gaps can grow (word
                // spacing would not apply to the two-byte codes of embedded fonts).
                for (n, word) in text.split(' ').enumerate() {
                    if n > 0 {
                        x += res.char_width(' ', &style) + extra;
                    }
                    if !word.is_empty() {
                        x += show(c, word, &style, x, baseline, res);
                    }
                }
            } else {
                x += show(c, &text, &style, x, baseline, res);
            }
            if style.underline {
                let y = baseline + style.raise - 0.145 * style.size;
                let [r, g, b] = style.color;
                c.save_state()
                    .set_stroke_rgb(unit(r), unit(g), unit(b))
                    .set_line_width((0.066 * style.size).max(0.3))
                    .move_to(start, y)
                    .line_to(x, y)
                    .stroke()
                    .restore_state();
            }
        }
        cursor -= line.pitch + line.after;
    }
}

/// Draw `text` at (`x`, `baseline`) and return its width.
fn show(
    c: &mut Content,
    text: &str,
    style: &Style,
    x: f32,
    baseline: f32,
    res: &mut Resources,
) -> f32 {
    let width: f32 = text.chars().map(|ch| res.char_width(ch, style)).sum();
    let (name, bytes) = match style.font {
        FontRef::Std(family) => {
            let ix = res.std_index(family, style.bold, style.italic);
            (format!("F{}", ix + 1), encode(text))
        }
        FontRef::Emb(ix) => {
            let font = &mut res.embedded[ix];
            let bytes = text
                .chars()
                .flat_map(|ch| font.glyph(ch).cid.to_be_bytes())
                .collect();
            (format!("E{}", ix + 1), bytes)
        }
    };
    let [r, g, b] = style.color;
    c.begin_text();
    if style.color == [0, 0, 0] {
        c.set_fill_gray(0.0);
    } else {
        c.set_fill_rgb(unit(r), unit(g), unit(b));
    }
    c.set_font(Name(name.as_bytes()), style.size)
        .next_line(x, baseline + style.raise)
        .show(Str(&bytes))
        .end_text();
    width
}

/// Approximate advance width in points for one character of a standard font.
fn std_char_width(ch: char, family: Family, s: &Style) -> f32 {
    let em = match family {
        Family::Courier => 0.6,
        Family::Helvetica | Family::Times => {
            let base = match ch {
                'i' | 'j' | 'l' | '.' | ',' | ':' | ';' | '\'' | '!' | '|' | ' ' | 'I' | 't'
                | 'f' => 0.28,
                'm' | 'w' | 'M' | 'W' => 0.8,
                _ if s.bold => 0.58,
                _ => 0.54,
            };
            if family == Family::Times {
                base * 0.9
            } else {
                base
            }
        }
    };
    em * s.size
}

fn line_width(line: &[Glyph], res: &mut Resources) -> f32 {
    line.iter().map(|g| res.char_width(g.ch, &g.style)).sum()
}

/// One wrapped line.
struct Line {
    glyphs: Vec<Glyph>,
    align: Alignment,
    /// Style of the paragraph mark (used for the height of an empty line).
    mark: Style,
    /// `true` for the last line of its paragraph (never justified).
    last: bool,
    /// Index of the paragraph the line belongs to.
    para: usize,
    /// Distance from this line's top to the next line's top, in points.
    pitch: f32,
    /// Baseline distance below the top of the line, in points.
    ascent: f32,
    /// Space before (first line of a paragraph only) and space after (last line only).
    before: f32,
    after: f32,
    /// Horizontal offset from the column's left edge and width of the piece of the row
    /// this line sits in; `None` width = the whole column (text wrap, spec 4K).
    x_off: f32,
    width: Option<f32>,
    /// `true` when this line continues the previous line's row (same baseline, next piece
    /// to the right) instead of starting a new row.
    cont: bool,
    /// Vertical room skipped above this row because no part of its band was free.
    lead: f32,
    /// The bullet or number drawn before the first line of a list paragraph.
    label: Option<LineLabel>,
}

/// A list label (spec 4R): its text and style, and its x position relative to the start
/// of the line's text (negative when it hangs to the left of the text).
struct LineLabel {
    text: String,
    style: Style,
    rel_x: f32,
}

/// Indents of one paragraph, in points from the column's left edge (spec 4R.1).
struct ParaGeom {
    /// Start of the first line's text, and of the lines after it.
    first_x: f32,
    rest_x: f32,
    /// Right indent.
    right: f32,
    label: Option<LineLabel>,
}

impl ParaGeom {
    /// The geometry of `para` whose label (if it is in a list) is `label`
    /// ([`pub_format::line_starts`]).
    fn of(para: &Paragraph, label: Option<&str>, mark: Style, res: &mut Resources) -> ParaGeom {
        let (Some(text), Some(info)) = (label, &para.list) else {
            let s = pub_format::line_starts(para.indents, None);
            return ParaGeom {
                first_x: s.first_x,
                rest_x: s.rest_x,
                right: s.right,
                label: None,
            };
        };
        let (text, style) = label_style(info, text, mark, res);
        let w: f32 = text.chars().map(|ch| res.char_width(ch, &style)).sum();
        let s = pub_format::line_starts(para.indents, Some(w));
        ParaGeom {
            first_x: s.first_x,
            rest_x: s.rest_x,
            right: s.right,
            label: Some(LineLabel {
                text,
                style,
                rel_x: s.label_x - s.first_x,
            }),
        }
    }

    /// Width of the first line and of the lines after it in a column of `max_width`
    /// (`max_width` itself without indents).
    fn widths(&self, max_width: f32) -> (f32, f32) {
        let w = |x: f32| {
            if x + self.right > 0.0 {
                (max_width - x - self.right).max(0.0)
            } else {
                max_width
            }
        };
        (w(self.first_x), w(self.rest_x))
    }
}

/// The text and style a list label is drawn in: a bullet in its own font and size (a
/// symbol-encoded font's code is mapped to Unicode and drawn in the text font), a number in
/// the paragraph's text style.
fn label_style(info: &ListInfo, text: &str, mark: Style, res: &mut Resources) -> (String, Style) {
    let mut style = Style {
        underline: false,
        raise: 0.0,
        ..mark
    };
    if !info.kind.is_bullet() {
        return (text.to_owned(), style);
    }
    style.bold = false;
    style.italic = false;
    if let Some(size) = info.bullet_size_pt.filter(|s| *s > 0.0) {
        style.size = size;
    }
    if !info.bullet_font.is_empty() && !pub_format::is_symbol_font(&info.bullet_font) {
        style.font = res.resolve_font(&info.bullet_font, false, false);
    }
    let mut ch = pub_format::bullet_display_char(info.bullet_char, &info.bullet_font);
    if matches!(style.font, FontRef::Std(_)) && encode(&ch.to_string()) == b"?" {
        ch = '\u{2022}';
    }
    (ch.to_string(), style)
}

/// Where the text of a column sits on the page and what it must avoid (spec 4K).
struct WrapCtx<'a> {
    /// Column left edge and top, points in the text frame's own (unrotated, y down) space.
    left: f32,
    top: f32,
    /// Regions to avoid, same space.
    obstacles: &'a [pub_format::wrap::Poly],
    /// Column width and height; the height bounds the search for a free band.
    width: f32,
    height: f32,
}

/// Line pitch and baseline distance for a line whose largest style is `style`.
fn pitch_ascent(style: &Style, sp: &ParaSpacing, res: &Resources) -> (f32, f32) {
    let (natural, ascent) = res.metrics(style);
    let pitch = match sp.line {
        LineSpacing::Multiple(k) => k * natural,
        LineSpacing::Exactly(pt) => pt,
        // Rule not decoded: use the style default.
        LineSpacing::Unknown(_) => 1.19 * natural,
    };
    let ascent = if natural > 0.0 {
        ascent * pitch / natural
    } else {
        0.0
    };
    (pitch, ascent)
}

/// The style that sets a line's height: the largest one among `glyphs`, else `mark`.
fn tallest(glyphs: &[Glyph], mark: Style) -> Style {
    glyphs
        .iter()
        .map(|g| g.style)
        .fold(None::<Style>, |best, s| match best {
            Some(b) if b.size >= s.size => Some(b),
            _ => Some(s),
        })
        .unwrap_or(mark)
}

impl Line {
    /// Set `pitch`, `ascent` and `after` from the paragraph's spacing (spec 3.6): a
    /// multiple of the font's natural line height, or an exact height. The baseline sits
    /// at the font's ascent scaled with the line (UNCONFIRMED: where Publisher puts the
    /// extra room of loose or exact spacing).
    fn set_vertical(&mut self, sp: &ParaSpacing, res: &Resources) {
        let style = tallest(&self.glyphs, self.mark);
        (self.pitch, self.ascent) = pitch_ascent(&style, sp, res);
        if self.last {
            self.after = sp.space_after_pt;
        }
    }
}

/// Word-wrap the paragraphs to `max_width`, starting at story position `start` (a line
/// start: paragraph index and glyph offset in it). With `max_height`, stop before the
/// first line that no longer fits; the returned position is where the next frame resumes
/// (paragraph count, 0 when everything was laid out).
fn layout(
    paras: &[Paragraph],
    start: StoryPos,
    max_width: f32,
    max_height: Option<f32>,
    wrap_ctx: Option<&WrapCtx>,
    res: &mut Resources,
) -> (Vec<Line>, StoryPos) {
    let mut out: Vec<Line> = Vec::new();
    let labels = pub_format::list_labels(paras);
    // Vertical position of the next line's space before, from the column top.
    let mut ypos = 0.0f32;
    let mut next = (paras.len(), 0);
    let mut used = 0.0f32;
    // Space after the previous paragraph's last line; it counts only once another line
    // follows it in the frame, so a frame's trailing space after never costs room
    // (spec 3.6, 4G).
    let mut pending_after = 0.0f32;
    'paras: for (pi, para) in paras.iter().enumerate().skip(start.0) {
        let mut glyphs: Vec<Glyph> = Vec::new();
        let mut mark = None;
        for run in &para.runs {
            let nominal = run.size_pt.max(1.0);
            let style = Style {
                font: res.resolve_font(&run.font, run.bold, run.italic),
                bold: run.bold,
                italic: run.italic,
                // Super/subscript: glyphs at 0.65 x the nominal size, baseline shifted by a
                // fraction of the nominal size (spec 4S.3).
                size: match run.script {
                    pub_format::Script::Normal => nominal,
                    _ => nominal * 0.65,
                },
                raise: match run.script {
                    pub_format::Script::Normal => 0.0,
                    pub_format::Script::Superscript => nominal * 0.47,
                    pub_format::Script::Subscript => -nominal * 0.145,
                },
                underline: run.underline,
                color: run
                    .color
                    .map_or([0, 0, 0], |c| color_rgb(c, res.scheme.as_ref())),
            };
            mark = Some(style);
            for ch in run.text.chars() {
                match ch {
                    '\r' | '\n' => {}
                    '\t' => glyphs.push(Glyph { ch: ' ', style }),
                    c if c.is_control() => {}
                    _ => glyphs.push(Glyph { ch, style }),
                }
            }
        }
        let Some(mark) = mark else { continue };
        let skip = if pi == start.0 { start.1 } else { 0 };
        let mut offset = skip.min(glyphs.len());
        let mut geom = ParaGeom::of(para, labels[pi].as_deref(), mark, res);
        let rows: Vec<Row> = match wrap_ctx.filter(|w| !w.obstacles.is_empty()) {
            Some(w) => wrap_rows(&glyphs[offset..], para, mark, skip, ypos, w, &geom, res).0,
            None => {
                let (w_first, w_rest) = geom.widths(max_width);
                let w_first = if skip == 0 { w_first } else { w_rest };
                wrap_indented(&glyphs[offset..], w_first, w_rest, res)
                    .into_iter()
                    .enumerate()
                    .map(|(i, glyphs)| {
                        let first = i == 0 && skip == 0;
                        Row {
                            glyphs,
                            x_off: if first { geom.first_x } else { geom.rest_x },
                            width: if first { w_first } else { w_rest },
                            cont: false,
                            lead: 0.0,
                        }
                    })
                    .collect()
            }
        };
        let n = rows.len();
        for (i, row) in rows.into_iter().enumerate() {
            let begin = (pi, offset);
            offset += row.glyphs.len();
            let mut line = Line {
                glyphs: row.glyphs,
                align: para.alignment,
                mark,
                last: i + 1 == n,
                para: pi,
                pitch: 0.0,
                ascent: 0.0,
                // spec: ? (a paragraph's space before at the very top of a frame is kept)
                before: if i == 0 && skip == 0 {
                    para.spacing.space_before_pt
                } else {
                    0.0
                },
                after: 0.0,
                x_off: row.x_off,
                width: (row.width != max_width).then_some(row.width),
                cont: row.cont,
                lead: row.lead,
                // The label belongs to the paragraph's first line (first piece of its row).
                label: if i == 0 && skip == 0 { geom.label.take() } else { None },
            };
            line.set_vertical(&para.spacing, res);
            if line.cont {
                // Same row as the previous piece: no extra room, no new space.
                line.before = 0.0;
            } else {
                ypos += line.lead + line.before + line.pitch + line.after;
            }
            if let Some(h) = max_height.filter(|_| !line.cont) {
                let need = pending_after + line.lead + line.before + line.pitch;
                if used + need > h + 0.01 {
                    next = begin;
                    break 'paras;
                }
                used += need;
                pending_after = 0.0;
                if line.last {
                    pending_after = line.after;
                }
            }
            next = if line.last { (pi + 1, 0) } else { (pi, offset) };
            out.push(line);
        }
    }
    // Pieces of one row share a baseline and a pitch.
    let mut i = 0;
    while i < out.len() {
        let mut j = i + 1;
        while j < out.len() && out[j].cont {
            j += 1;
        }
        if j > i + 1 {
            let pitch = out[i..j].iter().map(|l| l.pitch).fold(0.0, f32::max);
            let ascent = out[i..j].iter().map(|l| l.ascent).fold(0.0, f32::max);
            for l in &mut out[i..j] {
                l.pitch = pitch;
                l.ascent = ascent;
            }
        }
        i = j;
    }
    (out, next)
}

/// Word-wrap a paragraph whose first line is `first_width` wide and the others `max_width`
/// (indents, spec 4R).
fn wrap_indented(
    para: &[Glyph],
    first_width: f32,
    max_width: f32,
    res: &mut Resources,
) -> Vec<Vec<Glyph>> {
    let mut lines: Vec<Vec<Glyph>> = vec![Vec::new()];
    let mut width = 0.0f32;
    for word in para.split_inclusive(|g| g.ch == ' ') {
        let word_width = line_width(word, res);
        // Trailing space may hang past the edge; measure the word without it.
        let trimmed = word_width
            - word
                .last()
                .filter(|g| g.ch == ' ')
                .map_or(0.0, |g| res.char_width(g.ch, &g.style));
        let limit = if lines.len() == 1 { first_width } else { max_width };
        if width > 0.0 && width + trimmed > limit {
            lines.push(Vec::new());
            width = 0.0;
        }
        for g in word {
            let w = res.char_width(g.ch, &g.style);
            // A single word wider than the box is broken between characters.
            let limit = if lines.len() == 1 { first_width } else { max_width };
            if width > 0.0 && width + w > limit && g.ch != ' ' {
                lines.push(Vec::new());
                width = 0.0;
            }
            if let Some(line) = lines.last_mut() {
                line.push(*g);
            }
            width += w;
        }
    }
    lines
}

/// How many glyphs of `para` fit on one line of `max_width`: the same rule as [`wrap`],
/// for one line. With `split_words` false a first word that does not fit takes nothing
/// (the caller tries another piece); with it true the word is broken between characters.
fn take_line(para: &[Glyph], max_width: f32, split_words: bool, res: &mut Resources) -> usize {
    let mut width = 0.0f32;
    let mut taken = 0;
    for word in para.split_inclusive(|g| g.ch == ' ') {
        let word_width = line_width(word, res);
        let trimmed = word_width
            - word
                .last()
                .filter(|g| g.ch == ' ')
                .map_or(0.0, |g| res.char_width(g.ch, &g.style));
        if width > 0.0 && width + trimmed > max_width {
            return taken;
        }
        if width == 0.0 && trimmed > max_width && !split_words {
            return taken;
        }
        for g in word {
            let w = res.char_width(g.ch, &g.style);
            if width > 0.0 && width + w > max_width && g.ch != ' ' {
                return taken;
            }
            taken += 1;
            width += w;
        }
    }
    taken
}

/// One piece of a wrapped row.
struct Row {
    glyphs: Vec<Glyph>,
    x_off: f32,
    width: f32,
    cont: bool,
    lead: f32,
}

/// Wrap one paragraph around obstacles (spec 4K): each row's band (its top to its top
/// plus its line height) is cut by the obstacles into free pieces; text fills the pieces
/// left to right on the same baseline, then the next row. Returns the rows and the
/// vertical position just below the paragraph. `y0` is where the paragraph's space
/// before starts, relative to the column top; `skip` > 0 means the paragraph continues
/// from an earlier frame (no space before).
fn wrap_rows(
    glyphs: &[Glyph],
    para: &Paragraph,
    mark: Style,
    skip: usize,
    y0: f32,
    ctx: &WrapCtx,
    geom: &ParaGeom,
    res: &mut Resources,
) -> (Vec<Row>, f32) {
    // spec: ? no rule for the narrowest usable piece; use about one average character.
    let min_piece = 0.5 * mark.size;
    let mut rows = Vec::new();
    let mut rest = glyphs;
    let mut y = y0;
    let mut first = true;
    loop {
        let before = if first && skip == 0 {
            para.spacing.space_before_pt
        } else {
            0.0
        };
        let probe = &rest[..rest.len().min(400)];
        let (band, _) = pitch_ascent(&tallest(probe, mark), &para.spacing, res);
        // This row's text area between the indents (the whole column without indents).
        let indent = if first && skip == 0 { geom.first_x } else { geom.rest_x };
        let (row_l, row_r) = if indent + geom.right < ctx.width {
            (ctx.left + indent, ctx.left + ctx.width - geom.right)
        } else {
            (ctx.left, ctx.left + ctx.width)
        };
        let mut lead = 0.0f32;
        let spans = loop {
            let top = ctx.top + y + before + lead;
            let s = pub_format::wrap::free_spans(
                ctx.obstacles,
                top,
                top + band,
                row_l,
                row_r,
                min_piece,
            );
            if !s.is_empty() {
                break s;
            }
            lead += band.max(1.0);
            if lead > ctx.height {
                // Nothing is free anywhere down the column: ignore the obstacles.
                lead = 0.0;
                break vec![(row_l, row_r)];
            }
        };
        let mut taken: Vec<(usize, (f32, f32))> = Vec::new();
        let mut at = 0;
        for &(a, b) in &spans {
            let n = take_line(&rest[at..], b - a, false, res);
            if n > 0 {
                taken.push((n, (a, b)));
                at += n;
            }
        }
        if taken.is_empty() && !rest.is_empty() {
            // Even the first word fits no piece: break it in the widest one.
            let widest = spans
                .iter()
                .copied()
                .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
                .unwrap_or((row_l, row_r));
            let n = take_line(rest, widest.1 - widest.0, true, res).max(1);
            taken.push((n, widest));
            at = n;
        }
        if taken.is_empty() {
            // An empty paragraph keeps one empty line.
            let (a, b) = spans[0];
            taken.push((0, (a, b)));
        }
        let (h, _) = pitch_ascent(&tallest(&rest[..at], mark), &para.spacing, res);
        let mut from = 0;
        for (i, &(n, (a, b))) in taken.iter().enumerate() {
            rows.push(Row {
                glyphs: rest[from..from + n].to_vec(),
                x_off: a - ctx.left,
                width: b - a,
                cont: i > 0,
                lead: if i == 0 { lead } else { 0.0 },
            });
            from += n;
        }
        rest = &rest[at..];
        y += before + lead + h;
        first = false;
        if rest.is_empty() {
            return (rows, y + para.spacing.space_after_pt);
        }
    }
}

/// Group a line into maximal same-style segments.
fn segments(line: &[Glyph]) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    for g in line {
        match out.last_mut() {
            Some((s, st)) if *st == g.style => s.push(g.ch),
            _ => out.push((g.ch.to_string(), g.style)),
        }
    }
    out
}

/// WinAnsi agrees with Latin-1 for the printable ranges; a few common punctuation
/// characters live in 0x80-0x9F; anything else is `?`.
fn encode(s: &str) -> Vec<u8> {
    s.chars()
        .map(|ch| match ch {
            '\u{20}'..='\u{7E}' | '\u{A0}'..='\u{FF}' => ch as u32 as u8,
            '\u{2026}' => 0x85,
            '\u{2018}' => 0x91,
            '\u{2019}' => 0x92,
            '\u{201C}' => 0x93,
            '\u{201D}' => 0x94,
            '\u{2022}' => 0x95,
            '\u{2013}' => 0x96,
            '\u{2014}' => 0x97,
            _ => b'?',
        })
        .collect()
}

/// Largest decoded PNG (bytes of 8-bit samples) the exporter will allocate for one picture.
const MAX_PNG_BYTES: usize = 256 << 20;

/// Decode a PNG to 8-bit gray or RGB samples (plus an alpha soft mask when present).
/// `None` on any decoding problem.
fn decode_png(bytes: &[u8], res: &mut Resources) -> Option<ImageDef> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    // The header is untrusted: refuse a picture that would need a huge buffer (PW-SEC-002).
    let need = reader.output_buffer_size()?;
    if need > MAX_PNG_BYTES {
        return None;
    }
    let mut buf = vec![0u8; need];
    let info = reader.next_frame(&mut buf).ok()?;
    let data = buf.get(..info.buffer_size())?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let (channels, color, alpha) = match info.color_type {
        png::ColorType::Grayscale => (1, ImageColor::Gray, false),
        png::ColorType::GrayscaleAlpha => (2, ImageColor::Gray, true),
        png::ColorType::Rgb => (3, ImageColor::Rgb, false),
        png::ColorType::Rgba => (4, ImageColor::Rgb, true),
        png::ColorType::Indexed => return None,
    };
    let color_channels = channels - usize::from(alpha);
    let mut samples = Vec::with_capacity(data.len());
    let mut mask = Vec::new();
    for px in data.chunks_exact(channels) {
        samples.extend_from_slice(&px[..color_channels]);
        if alpha {
            mask.push(px[color_channels]);
        }
    }
    let smask = if alpha {
        Some((res.alloc(), compress_to_vec_zlib(&mask, 6)))
    } else {
        None
    };
    Some(ImageDef {
        id: res.alloc(),
        width: i32::try_from(info.width).ok()?,
        height: i32::try_from(info.height).ok()?,
        color,
        jpeg: false,
        data: compress_to_vec_zlib(&samples, 6),
        smask,
    })
}

/// Pass a JPEG through as DCTDecode after reading its size and component count from the
/// first start-of-frame marker. `None` if no frame header is found.
fn decode_jpeg(bytes: &[u8], res: &mut Resources) -> Option<ImageDef> {
    if bytes.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = bytes[pos + 1];
        if marker == 0xFF || marker == 0x00 || (0xD0..=0xD8).contains(&marker) {
            pos += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = usize::from(u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]));
        // SOF0-SOF15 except DHT (C4), JPG (C8) and DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let f = bytes.get(pos + 4..pos + 4 + 6)?;
            let height = i32::from(u16::from_be_bytes([f[1], f[2]]));
            let width = i32::from(u16::from_be_bytes([f[3], f[4]]));
            let color = match f[5] {
                1 => ImageColor::Gray,
                3 => ImageColor::Rgb,
                4 => ImageColor::Cmyk,
                _ => return None,
            };
            if f[0] != 8 || width == 0 || height == 0 {
                return None;
            }
            return Some(ImageDef {
                id: res.alloc(),
                width,
                height,
                color,
                jpeg: true,
                data: bytes.to_vec(),
                smask: None,
            });
        }
        pos += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res() -> Resources {
        Resources::new(10, Vec::new())
    }

    #[test]
    fn jpeg_header_is_parsed_and_junk_is_rejected() {
        // SOI, APP0 (length 4), SOF0: 8 bits, 30 high, 40 wide, 3 components.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00,
            0x1E, 0x00, 0x28, 0x03,
        ];
        let img = decode_jpeg(&jpeg, &mut res()).unwrap();
        assert_eq!(
            (img.width, img.height, img.color),
            (40, 30, ImageColor::Rgb)
        );
        assert!(decode_jpeg(&[], &mut res()).is_none());
        assert!(decode_jpeg(&jpeg[..12], &mut res()).is_none());
        assert!(decode_jpeg(b"not a jpeg", &mut res()).is_none());
        assert!(decode_png(b"not a png", &mut res()).is_none());
    }

    /// PW-SEC-002: a PNG header claiming a gigantic size must be refused before any buffer is made.
    #[test]
    fn png_claiming_huge_dimensions_is_refused() {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for &b in data {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
                }
            }
            !crc
        }
        for (w, h) in [(60_000u32, 60_000u32), (1 << 30, 1 << 30), (30_000, 30_000)] {
            let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
            let mut chunk = b"IHDR".to_vec();
            chunk.extend_from_slice(&w.to_be_bytes());
            chunk.extend_from_slice(&h.to_be_bytes());
            chunk.extend_from_slice(&[8, 6, 0, 0, 0]);
            png.extend_from_slice(&13u32.to_be_bytes());
            png.extend_from_slice(&chunk);
            png.extend_from_slice(&crc32(&chunk).to_be_bytes());
            assert!(decode_png(&png, &mut res()).is_none(), "{w}x{h}");
        }
    }

    #[test]
    fn fonts_map_to_standard_families() {
        assert_eq!(Family::of("Times New Roman"), Family::Times);
        assert_eq!(Family::of("Georgia"), Family::Times);
        assert_eq!(Family::of("Courier New"), Family::Courier);
        assert_eq!(Family::of("Calibri"), Family::Helvetica);
        assert_eq!(Family::of("Arial"), Family::Helvetica);
        assert_eq!(Family::Times.base_font(true, true), "Times-BoldItalic");
    }
}
