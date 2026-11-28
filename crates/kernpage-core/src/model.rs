//! The editable document model. All positions are EMU (914,400 per inch) measured from
//! the page's top-left corner, y pointing down. Z-order is the order of `Page::shapes`
//! (last = topmost).

use crate::geometry::Rect;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use pub_format::{
    Field, LineSpacing, ListInfo, ListType, NumberFormat, ParaIndents, ParaSpacing, Section, WrapType,
};

pub const EMU_PER_INCH: i64 = 914_400;
pub const EMU_PER_POINT: i64 = 12_700;
/// Default margin guide distance: half an inch.
pub const DEFAULT_MARGIN_EMU: i64 = EMU_PER_INCH / 2;
pub const DEFAULT_FONT: &str = "Calibri";
pub const DEFAULT_SIZE_PT: f32 = 12.0;
/// The size of text with no size item in a `.pub` file (what Clear All Formatting returns to).
pub const FILE_DEFAULT_SIZE_PT: f32 = 10.0;

pub type ShapeId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Rgb = Rgb(0, 0, 0);
    pub const WHITE: Rgb = Rgb(255, 255, 255);

    fn default_line() -> Rgb {
        Rgb::BLACK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// Text without paragraph terminators (a `\n` is a line break inside the paragraph).
    pub text: String,
    pub font: String,
    pub size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Superscript / subscript (old AutoRecover snapshots have none).
    #[serde(default)]
    pub script: Script,
    pub color: Rgb,
    /// Set on the one-character run of a page-number field (spec 4T.3): the text stays `#`
    /// and is drawn as the number of the page being shown.
    #[serde(default)]
    pub field: Option<Field>,
}

/// Superscript / subscript of a run (Publisher's one-value character property, spec 4S).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Script {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

impl Script {
    /// Size of the drawn glyphs relative to the nominal size (spec 4S.3).
    pub const SIZE_FACTOR: f32 = 0.65;

    /// Baseline shift in units of the nominal size; positive = up.
    pub fn raise_factor(self) -> f32 {
        match self {
            Script::Normal => 0.0,
            Script::Superscript => 0.47,
            Script::Subscript => -0.145,
        }
    }

    /// Drawn size factor: 0.65 for super and subscript, 1 otherwise.
    pub fn size_factor(self) -> f32 {
        if self == Script::Normal { 1.0 } else { Self::SIZE_FACTOR }
    }
}

impl Run {
    pub fn plain(text: &str) -> Run {
        Run {
            text: text.to_owned(),
            font: DEFAULT_FONT.to_owned(),
            size_pt: DEFAULT_SIZE_PT,
            bold: false,
            italic: false,
            underline: false,
            script: Script::Normal,
            color: Rgb::BLACK,
            field: None,
        }
    }

    /// A copy of this run's formatting with different text.
    pub fn with_text(&self, text: &str) -> Run {
        Run {
            text: text.to_owned(),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub align: Align,
    pub spacing: ParaSpacing,
    /// The bullet or numbering the paragraph is in (spec 4R); the label is never in the text.
    #[serde(default)]
    pub list: Option<ListInfo>,
    /// Left, first-line and right indents in points.
    #[serde(default)]
    pub indents: ParaIndents,
}

impl Paragraph {
    pub fn plain(text: &str) -> Paragraph {
        Paragraph {
            runs: vec![Run::plain(text)],
            align: Align::Left,
            spacing: ParaSpacing::default(),
            list: None,
            indents: ParaIndents::default(),
        }
    }

    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    /// The formatting used when text is typed into this paragraph: its first run.
    pub fn base_run(&self) -> Run {
        self.runs.first().cloned().unwrap_or_else(|| Run::plain(""))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PictureFormat {
    Png,
    Jpeg,
    /// Windows Metafile clip art (decompressed bytes), drawn from its display list.
    Wmf,
    /// Enhanced Metafile clip art (decompressed bytes).
    Emf,
    /// Stored in a format we cannot display; drawn as a placeholder.
    Unknown,
}

/// The file an inserted picture came from, as Publisher records it with the picture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PictureFile {
    /// The file's name, with or without its folder.
    pub name: String,
    /// The file's last-write time as a Windows FILETIME, when known.
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeKind {
    TextBox {
        paragraphs: Vec<Paragraph>,
    },
    Picture {
        #[serde(with = "arc_bytes")]
        data: Arc<Vec<u8>>,
        format: PictureFormat,
        /// The file the picture was inserted from (saving records it); `None` for a picture
        /// read from a `.pub` file.
        file: Option<PictureFile>,
    },
    Rectangle,
    Ellipse,
    /// A straight line from the rectangle's top-left to its bottom-right.
    Line,
    /// A grid of cells, row-major. Column widths and row heights are in EMU; when empty
    /// (or not matching `cols` / `rows`) the cells are equal and fill the shape.
    /// A group (spec 4J): draws nothing itself; its `rect` is the union of its members,
    /// which are ordinary shapes of the page whose `group` points here.
    Group,
    Table {
        rows: u32,
        cols: u32,
        col_widths_emu: Vec<i64>,
        row_heights_emu: Vec<i64>,
        /// The heights the rows were set to, when some row has grown to fit its text (the
        /// grown height is in `row_heights_emu`; a row never shrinks below its entry here).
        /// Empty = no row has grown: `row_heights_emu` are the set heights.
        #[serde(default)]
        row_set_emu: Vec<i64>,
        cells: Vec<TableCell>,
    },
}

/// One table cell: its paragraphs and the margin between the cell edge and its text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub paragraphs: Vec<Paragraph>,
    /// Left, top, right, bottom margins, EMU (default 36,576).
    pub margins_emu: [i64; 4],
    /// The cell spans several rows or columns in the file (merged cells). The editor shows
    /// such a table as plain cells; row and column commands refuse it.
    #[serde(default)]
    pub merged: bool,
}

impl TableCell {
    pub const DEFAULT_MARGIN_EMU: i64 = 36_576;

    pub fn plain(text: &str) -> TableCell {
        TableCell {
            paragraphs: vec![Paragraph::plain(text)],
            margins_emu: [Self::DEFAULT_MARGIN_EMU; 4],
            merged: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Rgb,
    pub width_emu: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub rect: Rect,
    pub kind: ShapeKind,
    pub fill: Option<Rgb>,
    pub line: Option<Stroke>,
    /// Rotation about the rectangle's centre, degrees clockwise (spec 4H); `rect` is the
    /// unrotated frame.
    pub rotation_deg: f32,
    /// Line only: flipped horizontally / vertically within `rect`. The line runs top-left to
    /// bottom-right unless exactly one flag is set (then bottom-left to top-right); the
    /// flags also record which end the user started from.
    pub flip_h: bool,
    pub flip_v: bool,
    /// Text columns of a text box (1 = none); text fills column 1, then 2, ... (spec 4I).
    pub columns: u32,
    /// Gap between columns, EMU.
    pub column_gap_emu: i64,
    /// How text of other boxes flows around this shape (spec 4K).
    pub wrap: WrapType,
    /// Wrap distances left, top, right, bottom, EMU (default 36,576).
    pub wrap_dist_emu: [i64; 4],
    /// Text-frame margins left, top, right, bottom, EMU (Escher 0x81..0x84; default 0.04 in).
    /// Text of a text box starts this far inside the frame.
    pub inset_emu: [i64; 4],
    /// The group this shape belongs to, if any (clicking it selects the group).
    pub group: Option<ShapeId>,
}

impl Shape {
    /// A copy whose page-number field runs show `number` instead of `#` (spec 4T.3), or
    /// `None` when the shape has no such run. The model keeps `#`; only drawing swaps it.
    pub fn with_page_number(&self, number: &str) -> Option<Shape> {
        fn swap(paragraphs: &[Paragraph], number: &str) -> Option<Vec<Paragraph>> {
            if !paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.field.is_some()) {
                return None;
            }
            let mut out = paragraphs.to_vec();
            for r in out.iter_mut().flat_map(|p| p.runs.iter_mut()) {
                if r.field.is_some() {
                    r.text = number.to_owned();
                }
            }
            Some(out)
        }
        match &self.kind {
            ShapeKind::TextBox { paragraphs } => {
                let paragraphs = swap(paragraphs, number)?;
                let mut s = self.clone();
                s.kind = ShapeKind::TextBox { paragraphs };
                Some(s)
            }
            _ => None,
        }
    }

    pub const DEFAULT_WRAP_DIST_EMU: i64 = 36_576;
    /// Publisher 2021's default text-frame margin for a new text box: 0.04 in.
    pub const DEFAULT_INSET_EMU: i64 = 36_576;

    /// A shape with id 0 (the editor assigns a real id when it is added).
    pub fn new(rect: Rect, kind: ShapeKind) -> Shape {
        Shape {
            id: 0,
            rect,
            kind,
            fill: None,
            line: None,
            rotation_deg: 0.0,
            flip_h: false,
            flip_v: false,
            columns: 1,
            column_gap_emu: 0,
            wrap: WrapType::None,
            wrap_dist_emu: [Self::DEFAULT_WRAP_DIST_EMU; 4],
            inset_emu: [Self::DEFAULT_INSET_EMU; 4],
            group: None,
        }
    }

    pub fn text_box(rect: Rect, paragraphs: Vec<Paragraph>) -> Shape {
        Shape::new(rect, ShapeKind::TextBox { paragraphs })
    }

    /// Text-frame margins left, top, right, bottom in points.
    pub fn inset_pt(&self) -> [f64; 4] {
        self.inset_emu.map(|e| e as f64 / 12_700.0)
    }

    pub fn is_text_box(&self) -> bool {
        matches!(self.kind, ShapeKind::TextBox { .. })
    }
}

/// The four margin guides of a page, EMU from the page edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Margins {
    pub left: i64,
    pub right: i64,
    pub top: i64,
    pub bottom: i64,
}

impl Margins {
    pub const fn all(v: i64) -> Margins {
        Margins {
            left: v,
            right: v,
            top: v,
            bottom: v,
        }
    }

    /// Left, right, top, bottom (the order of the file's guide records).
    pub fn to_array(self) -> [i64; 4] {
        [self.left, self.right, self.top, self.bottom]
    }

    pub fn from_array(a: [i64; 4]) -> Margins {
        Margins {
            left: a[0],
            right: a[1],
            top: a[2],
            bottom: a[3],
        }
    }
}

/// A page's identity while the program runs: stable across inserts, moves and deletes, so
/// that saving can tell which page of the opened file each editor page is.
pub type PageId = u64;

/// A fresh page identity.
pub fn new_page_id() -> PageId {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1 << 32);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Identity (see [`PageId`]); not part of the page's content.
    pub id: PageId,
    pub width_emu: i64,
    pub height_emu: i64,
    /// The margin guides (Publisher keeps one set on the master page, the same for every
    /// page; the editor keeps the same values on every page).
    pub margins_emu: Margins,
    pub shapes: Vec<Shape>,
    /// Set when this page was made by Duplicate Page: the page it was copied from. The save
    /// planner uses it to write a real page duplicate (spec 4P.5) instead of a blank page
    /// plus new objects; not part of the page's content.
    pub duplicate_of: Option<PageId>,
    /// Index into `Document::masters` of the page's master (spec 4T.2).
    #[serde(default)]
    pub master: Option<usize>,
    /// The page does not show its master's shapes.
    #[serde(default)]
    pub ignore_master: bool,
}

/// A master page (spec 4T): shapes drawn behind the shapes of the pages that use it. Edited
/// in the Master Page view through the pseudo page indexes of [`master_surface`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPage {
    /// Identity (like [`PageId`] for pages): the save planner matches a master with the one
    /// of the opened file by it. Not part of the content, never saved in a `.kern` file
    /// (`0` after reading one; [`Document::repair_masters`] gives it a fresh one).
    #[serde(skip)]
    pub uid: u64,
    /// The one-character id Publisher shows (`'A'`, `'B'`, ...), assigned automatically when
    /// the master is added; `'\0'` (a `.kern` file from before ids existed) is repaired on
    /// load.
    #[serde(default)]
    pub id: char,
    /// The name, e.g. `Master Page A`.
    pub name: String,
    /// What the user typed as the description (at most [`MasterPage::MAX_DESCRIPTION`]
    /// characters). A `.pub` file has no place for it that we know of (research 2026-11-28).
    #[serde(default)]
    pub description: String,
    /// A two-page master (for facing pages): `left_shapes` is the left half and `shapes` the
    /// right half; page 1, 3, ... show the right half and page 2, 4, ... the left one.
    #[serde(default)]
    pub two_page: bool,
    /// The margin guides, used while the master is edited.
    #[serde(default = "MasterPage::default_margins")]
    pub margins_emu: Margins,
    /// The shapes of a single-page master, or of the right half of a two-page master.
    pub shapes: Vec<Shape>,
    /// The shapes of the left half of a two-page master (empty otherwise).
    #[serde(default)]
    pub left_shapes: Vec<Shape>,
}

impl PartialEq for MasterPage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.description == other.description
            && self.two_page == other.two_page
            && self.margins_emu == other.margins_emu
            && self.shapes == other.shapes
            && self.left_shapes == other.left_shapes
    }
}

/// A fresh master identity.
pub fn new_master_uid() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1 << 32);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl MasterPage {
    /// Longest description the Master Page dialogs accept.
    pub const MAX_DESCRIPTION: usize = 40;

    fn default_margins() -> Margins {
        Margins::all(DEFAULT_MARGIN_EMU)
    }

    /// The default name of the master with id `id`.
    pub fn default_name(id: char) -> String {
        format!("Master Page {id}")
    }

    /// An empty single-page master.
    pub fn new(id: char, margins: Margins) -> MasterPage {
        MasterPage {
            uid: new_master_uid(),
            id,
            name: Self::default_name(id),
            description: String::new(),
            two_page: false,
            margins_emu: margins,
            shapes: Vec::new(),
            left_shapes: Vec::new(),
        }
    }

    /// The shapes shown on a page: the left half for the left page of a two-page master.
    pub fn half(&self, left: bool) -> &[Shape] {
        if left && self.two_page {
            &self.left_shapes
        } else {
            &self.shapes
        }
    }

    /// Both halves, left first.
    pub fn all_shapes(&self) -> impl Iterator<Item = &Shape> {
        self.left_shapes.iter().chain(self.shapes.iter())
    }
}

/// The first id letter no master of `masters` uses: `A`..`Z`, then `a`..`z`, then `0`..`9`.
pub fn next_master_id(masters: &[MasterPage]) -> Option<char> {
    ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .find(|c| !masters.iter().any(|m| m.id == *c))
}

/// Pseudo page indexes: a [`Command`](crate::command::Command) whose `page` is at least this
/// addresses a master page (see [`master_surface`]) instead of a document page, so every
/// shape command also edits master shapes.
pub const MASTER_SURFACE_BASE: usize = 1 << 30;

/// The page index that stands for master `master` (the right or only half; `left` for the
/// left half of a two-page master).
pub fn master_surface(master: usize, left: bool) -> usize {
    MASTER_SURFACE_BASE + master * 2 + usize::from(left)
}

/// `(master, left half)` for a pseudo page index, `None` for a real page index.
pub fn master_surface_parts(page: usize) -> Option<(usize, bool)> {
    let n = page.checked_sub(MASTER_SURFACE_BASE)?;
    Some((n / 2, n % 2 == 1))
}

impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        self.width_emu == other.width_emu
            && self.height_emu == other.height_emu
            && self.margins_emu == other.margins_emu
            && self.shapes == other.shapes
            && self.master == other.master
            && self.ignore_master == other.ignore_master
    }
}

impl Page {
    pub fn blank(width_emu: i64, height_emu: i64) -> Page {
        Page {
            id: new_page_id(),
            width_emu,
            height_emu,
            margins_emu: Margins::all(DEFAULT_MARGIN_EMU),
            shapes: Vec::new(),
            duplicate_of: None,
            master: None,
            ignore_master: false,
        }
    }

    pub fn shape(&self, id: ShapeId) -> Option<&Shape> {
        self.shapes.iter().find(|s| s.id == id)
    }

    /// The outermost group containing `id`, or `id` itself when it is not grouped.
    pub fn top_group(&self, id: ShapeId) -> ShapeId {
        let mut cur = id;
        for _ in 0..32 {
            match self.shape(cur).and_then(|s| s.group) {
                Some(g) if self.shape(g).is_some() => cur = g,
                _ => break,
            }
        }
        cur
    }

    /// Every shape below group `id` (members, members of member groups, ...).
    pub fn descendants(&self, id: ShapeId) -> Vec<ShapeId> {
        let mut out: Vec<ShapeId> = Vec::new();
        let mut todo = vec![id];
        while let Some(g) = todo.pop() {
            for s in self.shapes.iter().filter(|s| s.group == Some(g)) {
                if !out.contains(&s.id) && s.id != id {
                    out.push(s.id);
                    todo.push(s.id);
                }
            }
        }
        out
    }

    /// `ids` plus everything below the groups among them, without repeats.
    pub fn with_descendants(&self, ids: &[ShapeId]) -> Vec<ShapeId> {
        let mut out: Vec<ShapeId> = Vec::new();
        for &id in ids {
            for x in std::iter::once(id).chain(self.descendants(id)) {
                if !out.contains(&x) {
                    out.push(x);
                }
            }
        }
        out
    }
}

pub type StoryId = u64;

/// A story position: (paragraph index, glyph offset in that paragraph), as reported by
/// `pub2pdf::chain_breaks`. Glyphs are the characters of the runs without CR, LF and
/// control characters (a tab counts as one).
pub type StoryPos = (usize, usize);

/// The text of a chain of linked text boxes (spec 4G): one story that runs through its
/// frames. The story is the truth; the `paragraphs` of each frame's `TextBox` are a cache
/// of that frame's share (see the `story` module for slicing and splicing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub id: StoryId,
    /// The frames in chain order.
    pub frames: Vec<ShapeId>,
    /// The whole story, including the overflow after the last frame.
    pub paragraphs: Vec<Paragraph>,
    /// Where each frame's share ends, from the last layout (`ends[i]` is where frame `i`
    /// stops and frame `i + 1` starts; frame 0 starts at `(0, 0)`).
    pub ends: Vec<StoryPos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Shown in the title bar, e.g. "Publication1" or a file name.
    #[serde(default)]
    pub name: String,
    pub pages: Vec<Page>,
    /// Scheme colour 0 (the text/line colour) of the document: the colour of a new line or
    /// outline, as in Publisher.
    #[serde(default = "Rgb::default_line")]
    pub default_line: Rgb,
    /// Linked text box chains (empty for documents without any).
    #[serde(default)]
    pub stories: Vec<Story>,
    /// The master pages of an opened file (spec 4T); shown behind the pages, never edited.
    #[serde(default)]
    pub masters: Vec<MasterPage>,
    /// Page-number sections of an opened file (spec 4T.4).
    #[serde(default)]
    pub sections: Vec<Section>,
    #[serde(default)]
    next_id: ShapeId,
}

impl PartialEq for Document {
    /// The id counter is bookkeeping, not content.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.pages == other.pages
            && self.stories == other.stories
            && self.masters == other.masters
    }
}

impl Document {
    pub fn new(name: &str, pages: Vec<Page>) -> Document {
        let mut doc = Document {
            name: name.to_owned(),
            pages,
            default_line: Rgb::BLACK,
            stories: Vec::new(),
            masters: Vec::new(),
            sections: Vec::new(),
            next_id: 1,
        };
        // Give every shape that has no id a fresh one.
        let mut next = 1;
        for page in &mut doc.pages {
            for s in &mut page.shapes {
                if s.id == 0 {
                    s.id = next;
                }
                next = next.max(s.id) + 1;
            }
        }
        doc.next_id = next;
        doc
    }

    /// Make the id counter safe after loading a document from a file: above every shape id.
    pub(crate) fn repair_next_id(&mut self) {
        let max = self
            .pages
            .iter()
            .flat_map(|p| p.shapes.iter())
            .chain(self.masters.iter().flat_map(|m| m.all_shapes()))
            .map(|s| s.id)
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max.saturating_add(1)).max(1);
    }

    /// A blank one-page document.
    pub fn blank(name: &str, width_emu: i64, height_emu: i64) -> Document {
        Document::new(name, vec![Page::blank(width_emu, height_emu)])
    }

    /// Give every page a fresh identity (page ids are only meaningful within one run; a
    /// document loaded from a recovery snapshot must not clash with pages made since).
    pub fn renew_page_ids(&mut self) {
        let old: Vec<PageId> = self.pages.iter().map(|p| p.id).collect();
        let new: Vec<PageId> = old.iter().map(|_| new_page_id()).collect();
        for (p, &id) in self.pages.iter_mut().zip(&new) {
            p.id = id;
            p.duplicate_of = p
                .duplicate_of
                .and_then(|d| old.iter().position(|&o| o == d).map(|i| new[i]));
        }
    }

    /// The text a page-number field shows on page `index` (0-based).
    pub fn page_number_text(&self, index: usize) -> String {
        pub_format::page_number_in(&self.sections, index)
    }

    /// The shapes of the master page that page `index` shows behind its own shapes (none
    /// when the page ignores its master). A two-page master shows its right half on pages
    /// 1, 3, ... and its left half on pages 2, 4, ...
    pub fn master_shapes(&self, index: usize) -> &[Shape] {
        let Some(p) = self.pages.get(index).filter(|p| !p.ignore_master) else {
            return &[];
        };
        p.master
            .and_then(|m| self.masters.get(m))
            .map_or(&[], |m| m.half(index % 2 == 1))
    }

    /// The first id letter no master uses: `A`..`Z`, then `a`..`z`, then `0`..`9`.
    pub fn next_master_id(&self) -> Option<char> {
        next_master_id(&self.masters)
    }

    /// Give masters without a usable id one, and an empty name its default (after reading a
    /// file).
    pub(crate) fn repair_masters(&mut self) {
        let mut used: Vec<char> = Vec::new();
        for i in 0..self.masters.len() {
            if self.masters[i].uid == 0 {
                self.masters[i].uid = new_master_uid();
            }
            let id = self.masters[i].id;
            if id == '\0' || id.is_control() || used.contains(&id) {
                let fresh = ('A'..='Z')
                    .chain('a'..='z')
                    .chain('0'..='9')
                    .find(|c| !used.contains(c) && !self.masters[i + 1..].iter().any(|m| m.id == *c))
                    .unwrap_or('?');
                self.masters[i].id = fresh;
            }
            used.push(self.masters[i].id);
            if self.masters[i].name.is_empty() {
                self.masters[i].name = MasterPage::default_name(self.masters[i].id);
            }
            let n = self.masters[i].description.chars().count();
            if n > MasterPage::MAX_DESCRIPTION {
                self.masters[i].description =
                    self.masters[i].description.chars().take(MasterPage::MAX_DESCRIPTION).collect();
            }
        }
        let count = self.masters.len();
        for p in &mut self.pages {
            if p.master.is_some_and(|m| m >= count) {
                p.master = None;
            }
        }
    }

    /// A page, or a master page (a pseudo index, see [`master_surface`]) as a page of the
    /// document's size and the master's margins. Masters are copied; pages are borrowed.
    pub fn surface(&self, index: usize) -> Option<std::borrow::Cow<'_, Page>> {
        let Some((mi, left)) = master_surface_parts(index) else {
            return self.pages.get(index).map(std::borrow::Cow::Borrowed);
        };
        let m = self.masters.get(mi)?;
        if left && !m.two_page {
            return None;
        }
        let first = self.pages.first()?;
        Some(std::borrow::Cow::Owned(Page {
            id: 0,
            width_emu: first.width_emu,
            height_emu: first.height_emu,
            margins_emu: m.margins_emu,
            shapes: m.half(left).to_vec(),
            duplicate_of: None,
            master: None,
            ignore_master: false,
        }))
    }

    /// Every shape list of the document: each page's, then each master's halves.
    pub fn shape_lists_mut(&mut self) -> impl Iterator<Item = &mut Vec<Shape>> {
        self.pages.iter_mut().map(|p| &mut p.shapes).chain(
            self.masters
                .iter_mut()
                .flat_map(|m| [&mut m.left_shapes, &mut m.shapes]),
        )
    }

    /// The shape list of a page or of a master half (pseudo index).
    pub fn shapes_mut(&mut self, index: usize) -> Option<&mut Vec<Shape>> {
        match master_surface_parts(index) {
            None => self.pages.get_mut(index).map(|p| &mut p.shapes),
            Some((mi, left)) => {
                let m = self.masters.get_mut(mi)?;
                Some(if left && m.two_page { &mut m.left_shapes } else { &mut m.shapes })
            }
        }
    }

    pub fn alloc_id(&mut self) -> ShapeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The story a text box is a frame of, if it is linked.
    pub fn story_of(&self, frame: ShapeId) -> Option<&Story> {
        self.stories.iter().find(|s| s.frames.contains(&frame))
    }

    pub fn page(&self, index: usize) -> Option<&Page> {
        self.pages.get(index)
    }

    /// A shape of page `page` (or of a master half, see [`master_surface`]).
    pub fn shape(&self, page: usize, id: ShapeId) -> Option<&Shape> {
        match master_surface_parts(page) {
            None => self.pages.get(page)?.shape(id),
            Some((mi, left)) => self.masters.get(mi)?.half(left).iter().find(|s| s.id == id),
        }
    }

    /// Every font family named by any text in the document, with the bold/italic
    /// combinations used (for loading fonts on screen).
    pub fn fonts_used(&self) -> Vec<(String, bool, bool)> {
        let mut out: Vec<(String, bool, bool)> = Vec::new();
        for s in self
            .pages
            .iter()
            .flat_map(|p| p.shapes.iter())
            .chain(self.masters.iter().flat_map(|m| m.all_shapes()))
        {
            let paras: Vec<&Paragraph> = match &s.kind {
                ShapeKind::TextBox { paragraphs } => paragraphs.iter().collect(),
                ShapeKind::Table { cells, .. } => {
                    cells.iter().flat_map(|c| c.paragraphs.iter()).collect()
                }
                _ => continue,
            };
            for r in paras.into_iter().flat_map(|p| p.runs.iter()) {
                let key = (r.font.clone(), r.bold, r.italic);
                if !out.contains(&key) {
                    out.push(key);
                }
            }
        }
        out
    }
}

/// How picture bytes are written to and read from JSON. By default (AutoRecover) they are a
/// hex string. While a `.kern` file is being written or read (see `native`), a picture is a
/// reference `assets/images/<sha256>.<ext>` and the bytes go to or come from a side table.
pub(crate) mod media {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Arc;

    pub(crate) enum Mode {
        Write(BTreeMap<String, Arc<Vec<u8>>>),
        Read(HashMap<String, Arc<Vec<u8>>>),
    }

    thread_local! {
        static MODE: RefCell<Option<Mode>> = const { RefCell::new(None) };
    }

    /// Installs a mode for the current thread; dropping it restores the default.
    pub(crate) struct Guard;

    impl Guard {
        pub(crate) fn install(mode: Mode) -> Guard {
            MODE.with(|m| *m.borrow_mut() = Some(mode));
            Guard
        }

        pub(crate) fn take_written(self) -> BTreeMap<String, Arc<Vec<u8>>> {
            match MODE.with(|m| m.borrow_mut().take()) {
                Some(Mode::Write(w)) => w,
                _ => BTreeMap::new(),
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            MODE.with(|m| *m.borrow_mut() = None);
        }
    }

    pub(super) fn reference_for(bytes: &Arc<Vec<u8>>) -> Option<String> {
        MODE.with(|m| match m.borrow_mut().as_mut() {
            Some(Mode::Write(w)) => {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(bytes.as_slice());
                let mut name = String::from("assets/images/");
                for b in digest {
                    name.push_str(&format!("{b:02x}"));
                }
                name.push('.');
                name.push_str(match crate::picture::sniff_format(bytes) {
                    super::PictureFormat::Png => "png",
                    super::PictureFormat::Jpeg => "jpg",
                    super::PictureFormat::Wmf => "wmf",
                    super::PictureFormat::Emf => "emf",
                    super::PictureFormat::Unknown => "bin",
                });
                w.entry(name.clone()).or_insert_with(|| bytes.clone());
                Some(name)
            }
            _ => None,
        })
    }

    /// `Some(Some(bytes))` found, `Some(None)` a reference with no such picture, `None` not a
    /// reference (or not reading a `.kern` file).
    pub(super) fn lookup(name: &str) -> Option<Option<Arc<Vec<u8>>>> {
        MODE.with(|m| match m.borrow().as_ref() {
            Some(Mode::Read(r)) if name.starts_with("assets/images/") => Some(r.get(name).cloned()),
            _ => None,
        })
    }
}

mod arc_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::Arc;

    pub fn serialize<S: Serializer>(v: &Arc<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        if let Some(name) = super::media::reference_for(v) {
            return s.serialize_str(&name);
        }
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(v.len() * 2);
        for &b in v.iter() {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
        s.serialize_str(&out)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<Vec<u8>>, D::Error> {
        let text = String::deserialize(d)?;
        match super::media::lookup(&text) {
            Some(Some(bytes)) => return Ok(bytes),
            Some(None) => return Err(serde::de::Error::custom("picture missing from the file")),
            None => {}
        }
        let bytes = text.as_bytes();
        if bytes.len() % 2 != 0 {
            return Err(serde::de::Error::custom("odd-length hex picture data"));
        }
        let nib = |c: u8| match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(serde::de::Error::custom("bad hex picture data")),
        };
        let mut out = Vec::with_capacity(bytes.len() / 2);
        for pair in bytes.chunks(2) {
            out.push(nib(pair[0])? << 4 | nib(pair[1])?);
        }
        Ok(Arc::new(out))
    }
}

/// The sections after a page was inserted at 0-based `index`: the new page belongs to the
/// section of the page before it, so every section that starts at or after `index` starts
/// one page later (spec 4T.4, ~).
pub fn sections_after_insert(sections: &[Section], index: usize) -> Vec<Section> {
    let at = index as u32 + 1; // 1-based position of the new page
    let mut out: Vec<Section> = sections.to_vec();
    for s in out.iter_mut().skip(1) {
        if s.start_page >= at {
            s.start_page += 1;
        }
    }
    out
}

/// The sections after the page at 0-based `index` was removed from a document of
/// `page_count` pages: later sections start one page earlier, a section whose only page
/// went is dropped, and the first section always starts at page 1 (spec 4T.4, ~).
pub fn sections_after_remove(sections: &[Section], index: usize, page_count: usize) -> Vec<Section> {
    let gone = index as u32 + 1; // 1-based page
    let mut out: Vec<Section> = Vec::new();
    for (i, s) in sections.iter().enumerate() {
        let end = sections
            .get(i + 1)
            .map_or(page_count as u32 + 1, |n| n.start_page); // first page after the section
        if s.start_page == gone && end == gone + 1 {
            continue; // its only page
        }
        let mut s = s.clone();
        if s.start_page > gone {
            s.start_page -= 1;
        }
        out.push(s);
    }
    if let Some(first) = out.first_mut() {
        first.start_page = 1;
    }
    out
}
