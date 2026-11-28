//! "Edit in place" saving for `.pub` files.
//!
//! A document opened from a file keeps the file's original bytes plus a link from every editor shape
//! back to its source (page index, shape index, story). On save the editor model is
//! compared with the model as it was when the file was opened (the baseline); moves and
//! resizes become `set_shape_frame` calls and changed text becomes one `set_story_text_keep_formatting`
//! call per story. Anything else that changed is reported as an "unsupported change" so the
//! caller can warn instead of silently dropping data. A publication made in the editor
//! saves the same way against a generated blank ([`SaveSource::for_new`]).

use crate::model::*;
use pub_format as pf;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub enum SaveError {
    Write(pub_write::WriteError),
    Io(std::io::Error),
    /// The bytes produced by the writer could not be read back; nothing was written.
    Verify(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Write(e) => write!(f, "{e}"),
            SaveError::Io(e) => write!(f, "{e}"),
            SaveError::Verify(e) => write!(f, "the saved file failed its check ({e})"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<pub_write::WriteError> for SaveError {
    fn from(e: pub_write::WriteError) -> Self {
        SaveError::Write(e)
    }
}
impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

/// Where an editor shape came from in the source file.
#[derive(Debug, Clone, Copy)]
struct Link {
    page: usize,
    shape: usize,
    /// The shape is a child of a rotated group (spec 4J): moving it is not supported.
    group_rotated: bool,
}

/// The frames of one story (a standalone text box, or a linked chain in chain order).
#[derive(Debug, Clone)]
struct Unit {
    /// Index into the file's stories (as taken by `pub_write::set_story_text_keep_formatting`).
    story_index: usize,
    frames: Vec<ShapeId>,
    /// `mid_para[i]`: frame `i` starts in the middle of the paragraph frame `i - 1` ends in.
    mid_para: Vec<bool>,
    /// The overflow: story text after the last frame's break, exactly as read. It is in no
    /// frame, so it is appended to the story when saving (never dropped).
    tail: Vec<Paragraph>,
    /// The first tail paragraph continues the last frame's last paragraph.
    tail_mid: bool,
}

/// A moved or resized shape.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameChange {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub rect: crate::geometry::Rect,
}

/// A shape whose fill, outline, rotation or line flips changed.
#[derive(Debug, Clone, PartialEq)]
pub struct LookChange {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub look: pub_write::ShapeLook,
}

/// A shape whose text wrap type changed (spec 4K).
#[derive(Debug, Clone, PartialEq)]
pub struct WrapChange {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub wrap: pub_format::WrapType,
}

/// A text box whose column count or gap changed (spec 4I).
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnChange {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub columns: u32,
    pub gap_emu: i64,
}

/// A story whose text changed.
#[derive(Debug, Clone, PartialEq)]
pub struct StoryChange {
    pub story_index: usize,
    pub text: String,
    /// Text offsets (UTF-16 units) of the page-number fields the story has now (spec 4T.3).
    pub fields: Vec<usize>,
}

/// A table whose cell text changed: one string per cell, row-major, paragraphs joined by `\n`.
#[derive(Debug, Clone, PartialEq)]
pub struct TableChange {
    pub id: ShapeId,
    pub story_index: usize,
    pub cells: Vec<String>,
}

/// A story or table whose character or paragraph formatting changed: the formatting the
/// edited paragraphs must carry, at UTF-16 offsets of the story text after the text
/// changes of the same plan are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatChange {
    pub story_index: usize,
    /// Set when the story belongs to a table.
    pub table: Option<ShapeId>,
    pub chars: Vec<pub_write::CharSpan>,
    pub paras: Vec<pub_write::ParaSpan>,
    /// Bullets, numbering and indents of the edited paragraphs (spec 4R); written after
    /// `paras`. Empty for tables.
    pub lists: Vec<pub_write::ListSpan>,
}

/// A new stacking order for the shapes of one page.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderChange {
    pub page: usize,
    /// Shape ids, back-most first.
    pub ids: Vec<ShapeId>,
    /// The same shapes as indexes into the source file's page, back-most first.
    pub order: Vec<usize>,
}

/// A group created or dissolved since the file was opened (spec 4J-b).
#[derive(Debug, Clone, PartialEq)]
pub enum GroupChange {
    /// Group these members (ids, and their shape indexes in the source file's page).
    Group {
        page: usize,
        group: ShapeId,
        members: Vec<ShapeId>,
        shapes: Vec<usize>,
    },
    /// Ungroup the file's group shape at this index of the source page.
    Ungroup {
        page: usize,
        group: ShapeId,
        shape: usize,
        members: Vec<ShapeId>,
    },
}

/// A plain text box created since the file was opened (spec 4L). Frame in points from the
/// page's top-left corner; paragraphs of `text` are separated by `\r`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTextBox {
    pub id: ShapeId,
    pub page_index: usize,
    /// Creation sequence shared with [`NewShape`]: new objects are written in this order.
    pub seq: usize,
    pub left_pt: f64,
    pub top_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
    pub text: String,
    /// The formatted paragraphs, written after the plain text is added.
    pub paragraphs: Vec<Paragraph>,
    /// Text columns (1 = none) and the gap between them, EMU (spec 4I); written after the box is added.
    pub columns: u32,
    pub column_gap_emu: i64,
}

/// A plain rectangle, oval or line created since the file was opened (spec 4N). Frame in
/// points from the page's top-left corner; a line is described by its normalised box and
/// flip flags.
#[derive(Debug, Clone, PartialEq)]
pub struct NewShape {
    pub id: ShapeId,
    pub page_index: usize,
    /// Creation sequence shared with [`NewTextBox`].
    pub seq: usize,
    pub kind: pub_write::NewShapeKind,
    pub left_pt: f64,
    pub top_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
    pub style: pub_write::ShapeStyle,
}

/// A picture inserted since the file was opened (spec 4C.1). Frame in points from the
/// page's top-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPicture {
    pub id: ShapeId,
    pub page_index: usize,
    /// Creation sequence shared with [`NewTextBox`] and [`NewShape`].
    pub seq: usize,
    pub data: Arc<Vec<u8>>,
    pub format: pub_write::PictureKind,
    pub file_name: String,
    /// The source file's last-write time as a FILETIME, when known.
    pub modified: Option<u64>,
    pub left_pt: f64,
    pub top_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
}

/// A picture of the file that now shows another image (Change Picture, spec 4C.1.6). A
/// new frame is a separate [`FrameChange`].
#[derive(Debug, Clone, PartialEq)]
pub struct PictureChange {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    /// The picture's `Contents` object number in the source file.
    pub object: u32,
    pub data: Arc<Vec<u8>>,
    pub format: pub_write::PictureKind,
    pub file_name: String,
    pub modified: Option<u64>,
}

/// A picture of the file that was deleted (spec 4C.1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct PictureDeletion {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub object: u32,
}

/// A table created since the file was opened (spec 4E.1). Positions and track sizes in EMU
/// from the page's top-left corner; `texts` has one string per cell, row-major, paragraphs
/// joined by `\n`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTableObject {
    pub id: ShapeId,
    pub page_index: usize,
    /// Creation sequence shared with the other new objects.
    pub seq: usize,
    pub left_emu: i64,
    pub top_emu: i64,
    pub widths: Vec<u32>,
    pub heights: Vec<u32>,
    pub texts: Vec<String>,
    /// The formatted paragraphs of all cells in story order, written after the table is added.
    pub paragraphs: Vec<Paragraph>,
}

/// A table of the file whose rows or columns changed: it is replaced (deleted and added again
/// at the same place with the edited grid, spec 4E.1.7a), which is what the writer's row and
/// column inserts and deletes do. The result is the same for any series of such edits, so
/// the editor's final grid is written; cell formatting is written back afterwards.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRebuild {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    /// The table's `Contents` object number in the source file.
    pub object: u32,
    pub left_emu: i64,
    pub top_emu: i64,
    pub widths: Vec<u32>,
    pub heights: Vec<u32>,
    pub texts: Vec<String>,
    pub paragraphs: Vec<Paragraph>,
    /// Filled in by [`SaveSource::build`]: the new table's object number.
    pub new_object: Option<u32>,
    /// Filled in by [`SaveSource::build`]: the page's stacking order was put back (the new
    /// table is written on top of the page, then moved to where it was).
    pub order_restored: bool,
}

/// A table of the file that was deleted (spec 4E.1.5).
#[derive(Debug, Clone, PartialEq)]
pub struct TableDeletion {
    pub page: usize,
    pub shape: usize,
    pub id: ShapeId,
    pub object: u32,
}

/// One page operation, in the order it is applied. Indexes refer to the file as it is
/// when the step runs (see [`SavePlan::page_ops`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOp {
    /// Delete the page and everything on it.
    Delete { index: usize },
    /// Move a page so that it becomes page `to`.
    Move { from: usize, to: usize },
    /// Add a blank page after page `after`; `None` puts it first.
    Add { after: Option<usize> },
    /// Duplicate page `index` (spec 4P.5); the copy becomes page `index + 1`.
    Duplicate { index: usize },
}

/// Where each page of the opened file ends up after `ops`: `None` for a deleted page.
fn page_map(pages: usize, ops: &[PageOp]) -> Vec<Option<usize>> {
    // `Some(original)` for a page of the file, `None` for a page added by the plan.
    let mut list: Vec<Option<usize>> = (0..pages).map(Some).collect();
    for op in ops {
        match *op {
            PageOp::Delete { index } if index < list.len() => {
                list.remove(index);
            }
            PageOp::Move { from, to } if from < list.len() && to < list.len() => {
                let p = list.remove(from);
                list.insert(to, p);
            }
            PageOp::Add { after } => {
                let at = after.map_or(0, |a| (a + 1).min(list.len()));
                list.insert(at, None);
            }
            PageOp::Duplicate { index } if index < list.len() => {
                list.insert(index + 1, None);
            }
            _ => {}
        }
    }
    (0..pages)
        .map(|i| list.iter().position(|p| *p == Some(i)))
        .collect()
}

/// Page Setup changes (spec 4Q): a new page size (every page) and/or new margin guides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageSetupChange {
    pub size: Option<(i64, i64)>,
    pub margins: Option<Margins>,
}

/// What a save would do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SavePlan {
    /// Page deletions, moves and additions. Application order of a whole plan (so that the
    /// shape and story indexes the other steps use stay valid): 1. every per-page edit
    /// (groups, looks, wraps, columns, frames, stories, tables, formats, stacking order) on
    /// the ORIGINAL page and shape indexes; 2. page deletions, highest page first; 3. page
    /// moves, which bring the surviving pages into their final relative order; 4. page
    /// additions, in final page order, each `after` the page that precedes it in the
    /// final document (which is already in place); 5. new text boxes and shapes, on the
    /// FINAL page indexes (so they can sit on a page added in step 4). Stories of deleted
    /// text boxes stay in the file as unreferenced orphans (spec 4P.4). A page made by
    /// Duplicate Page whose content still equals its source's is a `Duplicate` step (in
    /// the same slot as an addition, followed by a `Move` when the copy is not right
    /// after its source); otherwise it is an addition plus new objects.
    pub page_ops: Vec<PageOp>,
    /// The editor pages written as page duplicates (copy, source), in the order of the
    /// `Duplicate` steps.
    pub duplicated: Vec<(PageId, PageId)>,
    /// Ungroups first, then groups.
    pub groups: Vec<GroupChange>,
    pub frames: Vec<FrameChange>,
    /// Fill, outline, rotation and flip changes; applied before the frames.
    pub looks: Vec<LookChange>,
    /// Wrap type changes, then column changes; applied after the looks.
    pub wraps: Vec<WrapChange>,
    pub columns: Vec<ColumnChange>,
    pub stories: Vec<StoryChange>,
    pub tables: Vec<TableChange>,
    pub formats: Vec<FormatChange>,
    pub orders: Vec<OrderChange>,
    /// New plain text boxes, in creation order; applied last.
    pub new_text_boxes: Vec<NewTextBox>,
    /// New rectangles, ovals and lines, in creation order; applied last, interleaved with
    /// the new text boxes by `seq`.
    pub new_shapes: Vec<NewShape>,
    /// New pictures, in creation order; applied last, interleaved with the other new
    /// objects by `seq`.
    pub new_pictures: Vec<NewPicture>,
    /// Pictures with another image; applied first, by object number.
    pub picture_changes: Vec<PictureChange>,
    /// Deleted pictures; applied after every step that uses the source file's shape
    /// indexes (and before the page operations).
    pub deleted_pictures: Vec<PictureDeletion>,
    /// Deleted tables, then tables with new rows or columns (replaced); applied right after
    /// the deleted pictures, by object number.
    pub deleted_tables: Vec<TableDeletion>,
    pub table_rebuilds: Vec<TableRebuild>,
    /// New tables, in creation order; applied last, interleaved with the other new objects.
    pub new_tables: Vec<NewTableObject>,
    /// A new page size and/or new margins. Applied after everything else: every other step
    /// is planned in the page's OLD size (the plan is made on the document shifted back by
    /// [`SaveSource::rebased`]), because the writer converts positions with the file's
    /// current page size, shapes keep their anchor from the page centre when the size
    /// changes (spec 4Q.4), and shapes that leave the page are moved to the scratch area by
    /// the writer, after which the source file's shape indexes no longer hold.
    pub page_setup: Option<PageSetupChange>,
    /// Page-number sections after page insertions, deletions and duplicates (spec 4T.4):
    /// `(record in the file, new start page)`, in order; records not listed are dropped.
    /// Empty = the sections are as in the file. Applied after the page operations.
    pub sections: Vec<(usize, u32)>,
    /// Human-readable descriptions of changes that cannot be saved yet.
    pub unsupported: Vec<String>,
}

impl SavePlan {
    pub fn is_empty(&self) -> bool {
        self.page_ops.is_empty()
            && self.frames.is_empty()
            && self.looks.is_empty()
            && self.wraps.is_empty()
            && self.columns.is_empty()
            && self.stories.is_empty()
            && self.tables.is_empty()
            && self.formats.is_empty()
            && self.orders.is_empty()
            && self.groups.is_empty()
            && self.new_text_boxes.is_empty()
            && self.new_shapes.is_empty()
            && self.new_pictures.is_empty()
            && self.picture_changes.is_empty()
            && self.deleted_pictures.is_empty()
            && self.deleted_tables.is_empty()
            && self.table_rebuilds.is_empty()
            && self.new_tables.is_empty()
            && self.page_setup.is_none()
            && self.sections.is_empty()
            && self.unsupported.is_empty()
    }
}

/// The original file and everything needed to patch it from the edited model.
#[derive(Debug, Clone)]
pub struct SaveSource {
    original: Arc<Vec<u8>>,
    baseline: Document,
    links: HashMap<ShapeId, Link>,
    units: Vec<Unit>,
    /// Table shapes and the index of the story holding their cell text.
    tables: HashMap<ShapeId, usize>,
    /// The document save counter was already raised for this opened file (spec 4M).
    counter_bumped: bool,
}

/// Why a copy of a text box with a page-number field cannot be saved: its field would need
/// its own entry in the file's field table, whose layout for several fields is not known.
const FIELD_COPY: &str = "page-number fields in duplicated or new text boxes";

/// Whether a text box holds a page-number field.
fn has_field(s: &Shape) -> bool {
    matches!(&s.kind, ShapeKind::TextBox { paragraphs }
        if paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.field.is_some()))
}

/// Map the editor's sections onto the records of the file's sections: the editor's list is
/// the file's with start pages changed and records dropped, so each editor section takes
/// the next file record that matches it in everything but the start page.
fn section_records(file: &[Section], now: &[Section]) -> Option<Vec<(usize, u32)>> {
    let mut at = 0;
    let mut out = Vec::new();
    for n in now {
        let k = (at..file.len()).find(|&k| {
            let f = &file[k];
            f.first_number == n.first_number
                && f.format == n.format
                && f.hide_first_page_hf == n.hide_first_page_hf
        })?;
        out.push((k, n.start_page));
        at = k + 1;
    }
    Some(out)
}

/// Text offsets (UTF-16 units, one unit per paragraph break) of the page-number field runs
/// of a story, in the text [`story_text_of`] makes.
fn field_offsets(paragraphs: &[Paragraph]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut at = 0;
    for (i, p) in paragraphs.iter().enumerate() {
        if i > 0 {
            at += 1;
        }
        for r in &p.runs {
            if r.field.is_some() {
                out.push(at);
            }
            at += r.text.encode_utf16().count();
        }
    }
    out
}

fn story_text_of(paragraphs: &[Paragraph]) -> String {
    paragraphs
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// One new object of a plan.
enum NewObject<'a> {
    TextBox(&'a NewTextBox),
    Shape(&'a NewShape),
    Picture(&'a NewPicture),
    Table(&'a NewTableObject),
}

/// The plan's new text boxes and shapes in creation order.
fn new_objects_in_order(plan: &SavePlan) -> Vec<NewObject<'_>> {
    let mut v: Vec<(usize, NewObject<'_>)> = plan
        .new_text_boxes
        .iter()
        .map(|b| (b.seq, NewObject::TextBox(b)))
        .chain(plan.new_shapes.iter().map(|n| (n.seq, NewObject::Shape(n))))
        .chain(
            plan.new_pictures
                .iter()
                .map(|n| (n.seq, NewObject::Picture(n))),
        )
        .chain(plan.new_tables.iter().map(|n| (n.seq, NewObject::Table(n))))
        .collect();
    v.sort_by_key(|e| e.0);
    v.into_iter().map(|e| e.1).collect()
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn para_align(a: Align) -> pub_write::ParaAlign {
    match a {
        Align::Left => pub_write::ParaAlign::Left,
        Align::Center => pub_write::ParaAlign::Center,
        Align::Right => pub_write::ParaAlign::Right,
        Align::Justify => pub_write::ParaAlign::Justify,
    }
}

/// For each paragraph of `after`: whether it differs from `before` (compared at the same
/// position from the start, or from the end, as formatted text).
fn changed_flags(before: &[Paragraph], after: &[Paragraph]) -> Vec<bool> {
    let (cb, ca) = (
        crate::text::canonical(before),
        crate::text::canonical(after),
    );
    (0..ca.len())
        .map(|i| {
            let from_end = ca.len() - 1 - i;
            let same_start = cb.get(i) == Some(&ca[i]);
            let same_end = from_end < cb.len() && cb[cb.len() - 1 - from_end] == ca[i];
            !(same_start || same_end)
        })
        .collect()
}

/// The formatting spans of the `changed` paragraphs of a story made of `paras` (each
/// followed by its break). A paragraph that is a single run also formats its break, as
/// formatting a whole paragraph in Publisher does.
fn format_spans(
    before: &[Paragraph],
    paras: &[Paragraph],
    changed: &[bool],
) -> (
    Vec<pub_write::CharSpan>,
    Vec<pub_write::ParaSpan>,
    Vec<pub_write::ListSpan>,
) {
    let mut chars = Vec::new();
    let mut aligns = Vec::new();
    let mut lists = Vec::new();
    let has_list_state =
        |p: &Paragraph| p.list.is_some() || p.indents != crate::model::ParaIndents::default();
    let mut pos = 0;
    for (i, (p, &ch)) in paras.iter().zip(changed).enumerate() {
        let runs = crate::text::normalize_runs(&p.runs);
        // Colours are written only for paragraphs whose set of colours changed, so text
        // coloured by the document's scheme keeps that link when only its font changes.
        let recolor = before.get(i).is_none_or(|b| color_set(b) != color_set(p));
        let color = |r: &crate::model::Run| recolor.then_some([r.color.0, r.color.1, r.color.2]);
        let len = utf16_len(&p.text());
        if ch {
            let mut at = pos;
            let uniform = runs.len() == 1;
            for r in runs.iter().filter(|r| !r.text.is_empty()) {
                let n = utf16_len(&r.text);
                chars.push(pub_write::CharSpan {
                    start: at,
                    end: at + n + usize::from(uniform),
                    format: pub_write::CharFormat {
                        bold: r.bold,
                        italic: r.italic,
                        underline: r.underline,
                        script: script_out(r.script),
                        size_pt: r.size_pt,
                        font: r.font.clone(),
                        color: color(r),
                        scheme_color: None,
                    },
                });
                at += n;
            }
            if len == 0 {
                // An empty paragraph: only its break carries the style.
                let r = &runs[0];
                chars.push(pub_write::CharSpan {
                    start: pos,
                    end: pos + 1,
                    format: pub_write::CharFormat {
                        bold: r.bold,
                        italic: r.italic,
                        underline: r.underline,
                        script: script_out(r.script),
                        size_pt: r.size_pt,
                        font: r.font.clone(),
                        color: color(r),
                        scheme_color: None,
                    },
                });
            }
            aligns.push(pub_write::ParaSpan {
                start: pos,
                end: pos + len + 1,
                align: para_align(p.align),
                spacing: Some(para_spacing(p.spacing)),
            });
            // List items are written for list paragraphs, paragraphs with indents, and
            // paragraphs that were in a list or had indents (the same position from the
            // start or from the end); other paragraphs keep whatever they have.
            let from_end = paras.len() - 1 - i;
            let was = before.get(i).is_some_and(has_list_state)
                || from_end < before.len() && has_list_state(&before[before.len() - 1 - from_end]);
            if has_list_state(p) || was {
                lists.push(pub_write::ListSpan {
                    start: pos,
                    end: pos + len + 1,
                    list: p.list.clone(),
                    indents: p.indents,
                });
            }
        }
        pos += len + 1;
    }
    (chars, aligns, lists)
}

/// The distinct colours of a paragraph's text.
fn color_set(p: &Paragraph) -> Vec<Rgb> {
    let mut out: Vec<Rgb> = p
        .runs
        .iter()
        .filter(|r| !r.text.is_empty() || p.runs.len() == 1)
        .map(|r| r.color)
        .collect();
    out.sort_by_key(|c| (c.0, c.1, c.2));
    out.dedup();
    out
}

fn para_spacing(s: crate::model::ParaSpacing) -> pub_write::ParaSpacing {
    pub_write::ParaSpacing {
        space_before_pt: s.space_before_pt,
        space_after_pt: s.space_after_pt,
        line: match s.line {
            crate::model::LineSpacing::Multiple(m) => Some(pub_write::LineRule::Multiple(m)),
            crate::model::LineSpacing::Exactly(pt) => Some(pub_write::LineRule::Exactly(pt)),
            crate::model::LineSpacing::Unknown(_) => None,
        },
    }
}

fn paragraphs_of(shape: &Shape) -> &[Paragraph] {
    match &shape.kind {
        ShapeKind::TextBox { paragraphs } => paragraphs,
        _ => &[],
    }
}

impl SaveSource {
    /// `bytes` are the file as read; `source` its parsed form; `model` the document that
    /// `from_pub_flowed(.., source, breaks)` produced (same page and shape order).
    pub fn new(
        bytes: Vec<u8>,
        source: &pf::Document,
        model: &Document,
        breaks: &[Vec<crate::StoryPos>],
    ) -> SaveSource {
        let mut links = HashMap::new();
        let streams = pub_format::Streams::from_cfb(&bytes).ok();
        for (pi, (sp, mp)) in source.pages.iter().zip(&model.pages).enumerate() {
            for (si, (ss, ms)) in sp.shapes.iter().zip(&mp.shapes).enumerate() {
                links.insert(
                    ms.id,
                    Link {
                        page: pi,
                        shape: si,
                        group_rotated: ss.group.is_some()
                            && streams
                                .as_ref()
                                .and_then(|st| pf::shape_slots(st, ss.object_number).ok().flatten())
                                .and_then(|sl| sl.child)
                                // Not a known child anchor: do not risk moving it.
                                .is_none_or(|c| c.group_rotation != 0.0),
                    },
                );
            }
        }
        let numbers = source.story_numbers();

        let id_at = |r: &pf::ShapeRef| {
            model
                .pages
                .get(r.page)
                .and_then(|p| p.shapes.get(r.shape))
                .map(|s| s.id)
        };
        let mut units = Vec::new();
        let mut covered: Vec<pf::ShapeRef> = Vec::new();
        let mut flowed = breaks.iter();
        for chain in source.chains() {
            let ends = if chain.len() > 1 { flowed.next() } else { None };
            covered.extend(chain.iter().copied());
            let shapes: Vec<&pf::Shape> = chain
                .iter()
                .map(|r| &source.pages[r.page].shapes[r.shape])
                .collect();
            let Some(number) = shapes.iter().find_map(|s| s.story) else {
                continue;
            };
            let Some(story_index) = numbers.iter().position(|&n| n == number) else {
                continue;
            };
            if shapes.iter().any(|s| s.table.is_some()) {
                continue;
            }
            let Some(frames) = chain.iter().map(id_at).collect::<Option<Vec<_>>>() else {
                continue;
            };
            let mut mid_para = vec![false; frames.len()];
            if let Some(ends) = ends {
                for (i, flag) in mid_para.iter_mut().enumerate().skip(1) {
                    *flag = ends.get(i - 1).is_some_and(|e| e.1 > 0);
                }
            }
            let (mut tail, mut tail_mid) = (Vec::new(), false);
            if let Some(last) = ends.and_then(|e| e.get(frames.len() - 1)).copied() {
                let story = shapes
                    .iter()
                    .find(|s| !s.paragraphs.is_empty())
                    .map(|s| &s.paragraphs);
                if let Some(story) = story {
                    let sch = source.color_scheme.as_ref();
                    tail = crate::convert::story_slice(story, last, (story.len(), 0))
                        .iter()
                        .map(|p| crate::convert::paragraph(p, sch))
                        .collect();
                    tail_mid = last.1 > 0 && !tail.is_empty();
                }
            }
            units.push(Unit {
                story_index,
                frames,
                mid_para,
                tail,
                tail_mid,
            });
        }
        // Text boxes with a story but no chain information.
        for (pi, page) in source.pages.iter().enumerate() {
            for (si, s) in page.shapes.iter().enumerate() {
                let r = pf::ShapeRef {
                    page: pi,
                    shape: si,
                };
                if covered.contains(&r) || s.table.is_some() {
                    continue;
                }
                let (Some(number), Some(id)) = (s.story, id_at(&r)) else {
                    continue;
                };
                if let Some(story_index) = numbers.iter().position(|&n| n == number) {
                    units.push(Unit {
                        story_index,
                        frames: vec![id],
                        mid_para: vec![false],
                        tail: Vec::new(),
                        tail_mid: false,
                    });
                }
            }
        }
        // Only text boxes carry editable text.
        units.retain(|u| {
            u.frames
                .iter()
                .all(|id| model_shape(model, *id).is_some_and(Shape::is_text_box))
        });
        let mut tables = HashMap::new();
        for (sp, mp) in source.pages.iter().zip(&model.pages) {
            for (ss, ms) in sp.shapes.iter().zip(&mp.shapes) {
                if let (Some(number), true) = (ss.story, ss.table.is_some()) {
                    if let Some(i) = numbers.iter().position(|&n| n == number) {
                        tables.insert(ms.id, i);
                    }
                }
            }
        }
        SaveSource {
            original: Arc::new(bytes),
            baseline: model.clone(),
            links,
            units,
            tables,
            counter_bumped: false,
        }
    }

    /// A source for a publication made in the editor that has no file yet: a generated
    /// blank with the same page count and size, whose pages take the editor pages' ids so
    /// that every object of `doc` plans as an addition.
    pub fn for_new(doc: &Document) -> Result<SaveSource, SaveError> {
        let first = doc
            .pages
            .first()
            .ok_or_else(|| SaveError::Verify("the publication has no pages".into()))?;
        // The blank has one page size; pages of other sizes are not written as such.
        let out_of_range = |_| SaveError::Verify("the page size is out of range".into());
        let spec = pub_write::NewPublication::new(
            u32::try_from(first.width_emu).map_err(out_of_range)?,
            u32::try_from(first.height_emu).map_err(out_of_range)?,
            u32::try_from(doc.pages.len()).map_err(out_of_range)?,
        );
        let bytes = pub_write::new_publication(&spec)?;
        let source = pf::read_bytes(&bytes).map_err(|e| SaveError::Verify(e.to_string()))?;
        let breaks = pub2pdf::chain_breaks(&source, &pub2pdf::Options::default());
        let mut model = crate::convert::from_pub_flowed(&doc.name, &source, &breaks);
        for (page, editor_page) in model.pages.iter_mut().zip(&doc.pages) {
            page.id = editor_page.id;
        }
        Ok(SaveSource::new(bytes, &source, &model, &breaks))
    }

    /// The bytes of the file as last read or saved.
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }

    fn unit_of(&self, id: ShapeId) -> Option<&Unit> {
        self.units.iter().find(|u| u.frames.contains(&id))
    }

    /// The paragraphs of a whole story reconstructed from its frames in chain order: a
    /// paragraph that continues into the next frame is joined back into one.
    ///
    /// A document that has a [`Story`] for the unit (what opening a linked file makes)
    /// supplies the whole text itself, overflow included: the frames' current texts are
    /// spliced into it. Without one the frames are joined and the overflow kept from the
    /// source file (`Unit::tail`).
    fn compose(&self, unit: &Unit, doc: &Document) -> Vec<Paragraph> {
        // The story of any frame the unit still has (a deleted frame leaves it).
        let story = unit
            .frames
            .iter()
            .find_map(|&id| doc.story_of(id))
            .filter(|s| s.frames.iter().all(|f| unit.frames.contains(f)));
        if let Some(story) = story {
            return doc.story_text(story);
        }
        self.compose_frames(unit, doc)
    }

    /// [`compose`](Self::compose) from the frames and the overflow remembered at open.
    fn compose_frames(&self, unit: &Unit, doc: &Document) -> Vec<Paragraph> {
        let mut out: Vec<Paragraph> = Vec::new();
        for (i, id) in unit.frames.iter().enumerate() {
            let Some(shape) = find_shape(doc, *id) else {
                continue;
            };
            let paras = paragraphs_of(shape);
            // An empty frame (one empty paragraph) contributes nothing.
            if paras.len() == 1 && paras[0].text().is_empty() {
                continue;
            }
            for (j, p) in paras.iter().enumerate() {
                let joined_to_previous = j == 0 && i > 0 && unit.mid_para[i];
                match out.last_mut() {
                    Some(prev) if joined_to_previous => prev.runs.extend(p.runs.iter().cloned()),
                    _ => out.push(p.clone()),
                }
            }
        }
        for (j, p) in unit.tail.iter().enumerate() {
            match out.last_mut() {
                Some(prev) if j == 0 && unit.tail_mid => prev.runs.extend(p.runs.iter().cloned()),
                _ => out.push(p.clone()),
            }
        }
        out
    }

    /// Page deletions, moves and additions that turn the opened file's page list into
    /// the editor's (`pairs`: file page index, editor page index of the surviving pages).
    fn plan_page_ops(
        &self,
        doc: &Document,
        pairs: &[(usize, usize)],
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        let base = &self.baseline;
        // 2. Deletions, highest page first.
        for bi in (0..base.pages.len()).rev() {
            if !pairs.iter().any(|p| p.0 == bi) {
                plan.page_ops.push(PageOp::Delete { index: bi });
            }
        }
        // 3. Moves: survivors in file order -> survivors in editor order.
        let mut order: Vec<PageId> = base
            .pages
            .iter()
            .filter(|p| doc.pages.iter().any(|d| d.id == p.id))
            .map(|p| p.id)
            .collect();
        let want: Vec<PageId> = doc
            .pages
            .iter()
            .filter(|d| order.contains(&d.id))
            .map(|d| d.id)
            .collect();
        for t in 0..want.len() {
            if order[t] != want[t] {
                if let Some(j) = order.iter().position(|x| *x == want[t]) {
                    let id = order.remove(j);
                    order.insert(t, id);
                    plan.page_ops.push(PageOp::Move { from: j, to: t });
                }
            }
        }
        // 4. Additions in final order. `cur` is the file's page list as the steps run.
        let (w, h) = (base.pages[0].width_emu, base.pages[0].height_emu);
        let mut cur: Vec<PageId> = want.clone();
        for (k, p) in doc.pages.iter().enumerate() {
            if base.pages.iter().any(|b| b.id == p.id) {
                continue;
            }
            if p.width_emu != w || p.height_emu != h {
                unsupported.add("added pages of a different size");
            }
            if let Some(src) = p.duplicate_of {
                match self.duplicable_source(doc, p, src, &cur) {
                    Ok(()) => {
                        let s = cur.iter().position(|x| *x == src).unwrap_or(0);
                        plan.page_ops.push(PageOp::Duplicate { index: s });
                        cur.insert(s + 1, p.id);
                        if s + 1 != k {
                            plan.page_ops.push(PageOp::Move { from: s + 1, to: k });
                            let id = cur.remove(s + 1);
                            cur.insert(k, id);
                        }
                        plan.duplicated.push((p.id, src));
                        continue;
                    }
                    Err(Some(why)) => unsupported.add(why),
                    Err(None) => {}
                }
            }
            plan.page_ops.push(PageOp::Add {
                after: k.checked_sub(1),
            });
            cur.insert(k.min(cur.len()), p.id);
            // 5. Shapes on the new page.
            let empty = Page {
                shapes: Vec::new(),
                ..p.clone()
            };
            self.plan_new_objects(&empty, p, k, plan, unsupported);
        }
    }

    /// Whether the new editor page `p` can be written as a duplicate of the file's page
    /// `src` (spec 4P.5): `src` is a page of the opened file with no unsaved object
    /// changes of its own, the copy still equals it (ignoring ids), and the writer can
    /// clone every object. `Err(Some(why))` names unsupported content; `Err(None)` just
    /// means the copy is saved as a new page with new objects.
    fn duplicable_source(
        &self,
        doc: &Document,
        p: &Page,
        src: PageId,
        cur: &[PageId],
    ) -> Result<(), Option<&'static str>> {
        let (Some(bp), Some(sp)) = (
            self.baseline.pages.iter().find(|b| b.id == src),
            doc.pages.iter().find(|d| d.id == src),
        ) else {
            return Err(None);
        };
        if !cur.contains(&src) {
            return Err(None);
        }
        if self.page_blocks_duplicate(sp) {
            return Err(Some("duplicated pages with groups, tables or linked text boxes"));
        }
        if sp.shapes.iter().any(has_field) {
            return Err(Some(FIELD_COPY));
        }
        let same_objects = sp.shapes.len() == bp.shapes.len()
            && sp.shapes.iter().zip(&bp.shapes).all(|(a, b)| {
                a.id == b.id
                    && self.links.contains_key(&a.id)
                    && (!a.is_text_box() || self.unit_of(a.id).is_some())
            });
        let neutral = |s: &Shape| Shape {
            id: 0,
            group: None,
            ..s.clone()
        };
        let same_copy = p.shapes.len() == sp.shapes.len()
            && p.shapes.iter().zip(&sp.shapes).all(|(a, b)| neutral(a) == neutral(b))
            && p.width_emu == sp.width_emu
            && p.height_emu == sp.height_emu;
        if same_objects && same_copy {
            Ok(())
        } else {
            Err(None)
        }
    }

    /// New text boxes and shapes of page `cp` (editor page `page_index`) that the file's
    /// page `bp` does not hold yet.
    fn plan_new_objects(
        &self,
        bp: &Page,
        cp: &Page,
        page_index: usize,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        let in_page = |id: &ShapeId, p: &Page| p.shapes.iter().any(|s| s.id == *id);
        let is_group = |s: &Shape| matches!(s.kind, ShapeKind::Group);
        for ns in cp
            .shapes
            .iter()
            .filter(|s| !is_group(s) && !in_page(&s.id, bp))
        {
            if has_field(ns) {
                unsupported.add(FIELD_COPY);
                continue;
            }
            let pt = EMU_PER_POINT as f64;
            let seq = plan.new_text_boxes.len()
                + plan.new_shapes.len()
                + plan.new_pictures.len()
                + plan.new_tables.len();
            match &ns.kind {
                ShapeKind::TextBox { paragraphs }
                    if ns.rotation_deg == 0.0 && ns.group.is_none() =>
                {
                    plan.new_text_boxes.push(NewTextBox {
                        id: ns.id,
                        page_index: page_index,
                        seq,
                        left_pt: ns.rect.x as f64 / pt,
                        top_pt: ns.rect.y as f64 / pt,
                        width_pt: ns.rect.w as f64 / pt,
                        height_pt: ns.rect.h as f64 / pt,
                        text: paragraphs
                            .iter()
                            .map(|p| p.text())
                            .collect::<Vec<_>>()
                            .join("\r"),
                        paragraphs: paragraphs.clone(),
                        columns: ns.columns,
                        column_gap_emu: ns.column_gap_emu,
                    });
                }
                ShapeKind::TextBox { .. } => {
                    unsupported.add("new text boxes that are rotated or grouped");
                }
                ShapeKind::Rectangle | ShapeKind::Ellipse | ShapeKind::Line => {
                    match self.new_shape(ns, page_index, seq) {
                        Ok(n) => plan.new_shapes.push(n),
                        Err(why) => unsupported.add(why),
                    }
                }
                ShapeKind::Picture { .. } => match new_picture(ns, page_index, seq) {
                    Ok(n) => plan.new_pictures.push(n),
                    Err(why) => unsupported.add(why),
                },
                ShapeKind::Table { .. } => match new_table_object(ns, page_index, seq) {
                    Ok(n) => plan.new_tables.push(n),
                    Err(why) => unsupported.add(why),
                },
                _ => unsupported.add("new objects"),
            }
        }
    }

    /// Compare `doc` with the baseline and describe what a save would do.
    /// The page size and margins the document wants, when they differ from the file's
    /// (taken from the document's first page: the editor keeps every page alike).
    fn page_setup_change(&self, doc: &Document) -> Option<PageSetupChange> {
        let (b, d) = (self.baseline.pages.first()?, doc.pages.first()?);
        let change = PageSetupChange {
            size: ((d.width_emu, d.height_emu) != (b.width_emu, b.height_emu))
                .then_some((d.width_emu, d.height_emu)),
            margins: (d.margins_emu != b.margins_emu).then_some(d.margins_emu),
        };
        (change != PageSetupChange::default()).then_some(change)
    }

    /// `doc` as it would be at the file's current page size and margins: every page's size
    /// and margins set back to the baseline's and its shapes shifted back by half the size
    /// change (the editor keeps shapes at their place from the page centre, spec 4Q.4).
    /// Everything but the final page-setup step is planned and written against this.
    fn rebased<'a>(&self, doc: &'a Document) -> std::borrow::Cow<'a, Document> {
        let Some(b) = self.baseline.pages.first() else {
            return std::borrow::Cow::Borrowed(doc);
        };
        let same = |p: &Page| {
            (p.width_emu, p.height_emu) == (b.width_emu, b.height_emu)
                && p.margins_emu == b.margins_emu
        };
        if doc.pages.iter().all(same) {
            return std::borrow::Cow::Borrowed(doc);
        }
        let mut d = doc.clone();
        // The masters follow the first page's size change (they keep one size).
        let (mdx, mdy) = doc.pages.first().map_or((0, 0), |p| {
            (b.width_emu / 2 - p.width_emu / 2, b.height_emu / 2 - p.height_emu / 2)
        });
        for m in &mut d.masters {
            m.margins_emu = b.margins_emu;
            for s in m.shapes.iter_mut().chain(m.left_shapes.iter_mut()) {
                s.rect = s.rect.translated(mdx, mdy);
            }
        }
        for p in &mut d.pages {
            let dx = b.width_emu / 2 - p.width_emu / 2;
            let dy = b.height_emu / 2 - p.height_emu / 2;
            for s in &mut p.shapes {
                s.rect = s.rect.translated(dx, dy);
            }
            (p.width_emu, p.height_emu, p.margins_emu) =
                (b.width_emu, b.height_emu, b.margins_emu);
        }
        std::borrow::Cow::Owned(d)
    }

    pub fn plan(&self, doc: &Document) -> SavePlan {
        let setup = self.page_setup_change(doc);
        let pages_differ = doc.pages.windows(2).any(|w| {
            (w[0].width_emu, w[0].height_emu) != (w[1].width_emu, w[1].height_emu)
        });
        let doc = self.rebased(doc);
        let (eff, skipped) = self.without_blocked_duplicates(&doc);
        let mut plan = self.plan_effective(&eff);
        plan.page_setup = setup;
        if pages_differ {
            plan.unsupported
                .push("pages of different sizes (the file has one page size)".to_string());
        }
        if skipped {
            let why = "duplicated pages with groups, tables or linked text boxes (the page is not saved)"
                .to_string();
            if !plan.unsupported.contains(&why) {
                plan.unsupported.push(why);
            }
        }
        plan
    }

    /// `doc` without the new pages that duplicate a page of the file whose content the
    /// writer cannot clone (groups, tables, linked text boxes), and without copies of
    /// such pages. They are left out of the plan altogether (the rest of the document
    /// still saves), so every later page index is the file's. The flag says whether any
    /// page was left out.
    fn without_blocked_duplicates<'a>(
        &self,
        doc: &'a Document,
    ) -> (std::borrow::Cow<'a, Document>, bool) {
        let mut left_out: Vec<PageId> = Vec::new();
        loop {
            let before = left_out.len();
            for p in &doc.pages {
                let Some(src) = p.duplicate_of else { continue };
                if left_out.contains(&p.id) || self.baseline.pages.iter().any(|b| b.id == p.id) {
                    continue;
                }
                let blocked = left_out.contains(&src)
                    || (self.baseline.pages.iter().any(|b| b.id == src)
                        && doc.pages.iter().find(|d| d.id == src).is_some_and(|sp| {
                            self.page_blocks_duplicate(sp)
                        }));
                if blocked {
                    left_out.push(p.id);
                }
            }
            if left_out.len() == before {
                break;
            }
        }
        if left_out.is_empty() {
            return (std::borrow::Cow::Borrowed(doc), false);
        }
        let mut d = doc.clone();
        d.pages.retain(|p| !left_out.contains(&p.id));
        (std::borrow::Cow::Owned(d), true)
    }

    fn page_blocks_duplicate(&self, sp: &Page) -> bool {
        sp.shapes.iter().any(|s| {
            let ok_kind = matches!(
                s.kind,
                ShapeKind::TextBox { .. }
                    | ShapeKind::Picture { .. }
                    | ShapeKind::Rectangle
                    | ShapeKind::Ellipse
                    | ShapeKind::Line
            );
            let chained = matches!(s.kind, ShapeKind::TextBox { .. })
                && self.unit_of(s.id).is_some_and(|u| u.frames.len() != 1);
            !ok_kind || s.group.is_some() || chained
        })
    }

    fn plan_effective(&self, doc: &Document) -> SavePlan {
        let mut plan = SavePlan::default();
        let mut unsupported = Unsupported::default();
        let base = &self.baseline;

        // Pages are matched by identity: a page of the editor is the page of the file it
        // was opened as, wherever it now sits.
        let pairs: Vec<(usize, usize)> = base
            .pages
            .iter()
            .enumerate()
            .filter_map(|(bi, bp)| {
                doc.pages
                    .iter()
                    .position(|p| p.id == bp.id)
                    .map(|ci| (bi, ci))
            })
            .collect();
        if pairs.is_empty() {
            unsupported.add("replacing all pages");
        } else {
            self.plan_page_ops(doc, &pairs, &mut plan, &mut unsupported);
        }
        let mut text_units: Vec<usize> = Vec::new();
        for &(bp_index, cp_index) in &pairs {
            let (bp, cp) = (&base.pages[bp_index], &doc.pages[cp_index]);
            if bp.width_emu != cp.width_emu || bp.height_emu != cp.height_emu {
                unsupported.add("page size changes");
            }
            let in_page = |id: &ShapeId, p: &Page| p.shapes.iter().any(|s| s.id == *id);
            let is_group = |s: &Shape| matches!(s.kind, ShapeKind::Group);
            // Group shapes are derived from their members: they take no part in the
            // object and stacking-order comparisons below.
            let deleted: Vec<&Shape> = bp
                .shapes
                .iter()
                .filter(|s| !is_group(s) && !in_page(&s.id, cp))
                .collect();
            if !deleted.is_empty() {
                let objects = object_numbers(&self.original);
                for s in deleted {
                    match self.picture_deletion(s, objects.as_deref()) {
                        Some(d) => plan.deleted_pictures.push(d),
                        None => match self.table_deletion(s, objects.as_deref()) {
                            Some(d) => plan.deleted_tables.push(d),
                            // Reported with the chain changes below.
                            None if self.unit_of(s.id).is_some_and(|u| u.frames.len() > 1) => {}
                            None => unsupported.add("deleted objects"),
                        },
                    }
                }
            }
            self.plan_new_objects(bp, cp, cp_index, &mut plan, &mut unsupported);
            // With the same groups on both sides the file's own order (groups included)
            // is permuted; `set_shape_order` keeps a group's members inside it.
            let gids = |p: &Page| -> Vec<ShapeId> {
                let mut v: Vec<ShapeId> = p
                    .shapes
                    .iter()
                    .filter(|s| is_group(s))
                    .map(|s| s.id)
                    .collect();
                v.sort_unstable();
                v
            };
            let same_groups = gids(bp) == gids(cp)
                && bp
                    .shapes
                    .iter()
                    .all(|b| cp.shape(b.id).is_none_or(|c| c.group == b.group));
            let ids_of = |p: &Page| -> Vec<ShapeId> {
                p.shapes
                    .iter()
                    .filter(|s| same_groups || !is_group(s))
                    .map(|s| s.id)
                    .collect()
            };
            let order_of = |a: &Page, b: &Page| -> Vec<ShapeId> {
                a.shapes
                    .iter()
                    .filter(|s| same_groups || !is_group(s))
                    .map(|s| s.id)
                    .filter(|id| in_page(id, b))
                    .collect()
            };
            let same_shapes =
                ids_of(bp).len() == ids_of(cp).len() && ids_of(bp).iter().all(|id| in_page(id, cp));
            if order_of(bp, cp) != order_of(cp, bp) && !same_shapes {
                unsupported.add("changes to the stacking order of objects");
            } else if same_shapes && ids_of(bp) != ids_of(cp) {
                let order: Option<Vec<usize>> = cp
                    .shapes
                    .iter()
                    .map(|s| self.links.get(&s.id).map(|l| l.shape))
                    .collect();
                match order {
                    Some(_) if !same_groups => {
                        unsupported.add("changes to the stacking order of grouped objects");
                    }
                    Some(order) => plan.orders.push(OrderChange {
                        page: bp_index,
                        ids: ids_of(cp),
                        order,
                    }),
                    None => unsupported.add("changes to the stacking order of objects"),
                }
            }
            self.plan_groups(bp_index, bp, cp, &mut plan, &mut unsupported);
            for cs in &cp.shapes {
                let Some(bs) = bp.shapes.iter().find(|s| s.id == cs.id) else {
                    continue;
                };
                if bs == cs || matches!(cs.kind, ShapeKind::Group) {
                    continue;
                }
                let Some(link) = self.links.get(&cs.id) else {
                    unsupported.add("changes to objects added after opening");
                    continue;
                };
                self.diff_shape(bs, cs, link, &mut plan, &mut unsupported, &mut text_units);
            }
        }
        // Chains: the file's links (items 39/40 of each frame) cannot be rewritten yet, so a
        // chain that lost a frame (deleted, or on a deleted page) or changed in any other
        // way is reported, never silently written differently.
        for unit in self.units.iter().filter(|u| u.frames.len() > 1) {
            if unit.frames.iter().any(|&id| find_shape(doc, id).is_none()) {
                unsupported.add(LINKED_DELETE);
            } else if doc
                .story_of(unit.frames[0])
                .is_some_and(|s| s.frames != unit.frames)
            {
                unsupported.add("changes to linked text boxes");
            }
        }
        // Stories: compare the reconstructed text, once per story.
        text_units.sort_unstable();
        text_units.dedup();
        for ui in text_units {
            let unit = &self.units[ui];
            // Without the story the text of a chain with a deleted frame cannot be put
            // together: leave the story in the file as it is.
            let story_kept = unit.frames.iter().any(|&id| doc.story_of(id).is_some());
            if !story_kept && unit.frames.iter().any(|&id| find_shape(doc, id).is_none()) {
                continue;
            }
            let (before, after) = (self.compose(unit, base), self.compose(unit, doc));
            let (before_text, after_text) = (story_text_of(&before), story_text_of(&after));
            let same_text = before_text == after_text;
            let flags = changed_flags(&before, &after);
            if flags.iter().any(|&c| c) {
                let (chars, paras, lists) = format_spans(&before, &after, &flags);
                plan.formats.push(FormatChange {
                    story_index: unit.story_index,
                    table: None,
                    chars,
                    paras,
                    lists,
                });
            }
            if !same_text {
                // A field lost from a file with several fields cannot be written (the
                // writer refuses): find out now.
                let had_field = before.iter().flat_map(|p| &p.runs).any(|r| r.field.is_some());
                let fields = field_offsets(&after);
                if had_field
                    && pub_write::set_story_text_with_fields(
                        &self.original,
                        unit.story_index,
                        &after_text,
                        &fields,
                    )
                    .is_err()
                {
                    unsupported.add("removing a page-number field when the file has several");
                }
                plan.stories.push(StoryChange {
                    story_index: unit.story_index,
                    text: after_text,
                    fields,
                });
            }
        }
        // Ungroups run before groups so freed shapes can be regrouped.
        plan.groups
            .sort_by_key(|g| !matches!(g, GroupChange::Ungroup { .. }));
        if doc.sections != base.sections {
            match section_records(&base.sections, &doc.sections) {
                Some(records) => plan.sections = records,
                None => unsupported.add("changes to page-number sections"),
            }
        }
        self.plan_masters(doc, &pairs, &mut unsupported);
        plan.unsupported = unsupported.into_list();
        plan
    }

    /// Master page changes: the writer cannot save any yet (adding, deleting or renaming
    /// masters, objects on masters, two-page masters, applying a master other than the first
    /// to a page, turning a page's master off), so each one is reported. A document whose
    /// masters were never touched (no masters in the editor model) has no changes.
    fn plan_masters(&self, doc: &Document, pairs: &[(usize, usize)], unsupported: &mut Unsupported) {
        let base = &self.baseline;
        if doc.masters.is_empty() {
            return;
        }
        if base.masters.len() < doc.masters.len() {
            unsupported.add("adding master pages");
        } else if base.masters.len() > doc.masters.len() {
            unsupported.add("deleting master pages");
        } else {
            for (b, d) in base.masters.iter().zip(&doc.masters) {
                if b.id != d.id || b.name != d.name || b.description != d.description {
                    unsupported.add("renaming master pages or changing their ID or description");
                }
                if b.two_page != d.two_page {
                    unsupported.add("two-page master pages");
                }
                if b.shapes != d.shapes || b.left_shapes != d.left_shapes {
                    unsupported.add("changes to the objects on master pages");
                }
            }
        }
        const APPLY: &str = "applying a master page other than the first to a page";
        const IGNORE: &str = "turning off the master page of a page";
        for &(bi, ci) in pairs {
            let (bp, cp) = (&base.pages[bi], &doc.pages[ci]);
            if bp.master != cp.master {
                unsupported.add(APPLY);
            }
            if bp.ignore_master != cp.ignore_master {
                unsupported.add(IGNORE);
            }
        }
        // A page added since opening is written with the first master.
        for p in &doc.pages {
            if base.pages.iter().any(|b| b.id == p.id) {
                continue;
            }
            if p.master.is_some_and(|m| m != 0) {
                unsupported.add(APPLY);
            }
            if p.ignore_master {
                unsupported.add(IGNORE);
            }
        }
    }

    /// Publisher's default outline colour: scheme colour 0 of the source file (spec 4N).
    fn default_line_color(&self) -> Rgb {
        pf::read_bytes(&self.original)
            .ok()
            .and_then(|d| d.color_scheme)
            .map_or(Rgb::BLACK, |c| {
                let [r, g, b] = c.colors[0];
                Rgb(r, g, b)
            })
    }

    /// The deletion of a picture of the file that sits directly on its page, when the
    /// writer can remove it.
    fn picture_deletion(
        &self,
        s: &Shape,
        objects: Option<&[Vec<u32>]>,
    ) -> Option<PictureDeletion> {
        if !matches!(s.kind, ShapeKind::Picture { .. }) || s.group.is_some() {
            return None;
        }
        let l = self.links.get(&s.id)?;
        let object = *objects?.get(l.page)?.get(l.shape)?;
        pub_write::delete_picture(&self.original, object).ok()?;
        Some(PictureDeletion {
            page: l.page,
            shape: l.shape,
            id: s.id,
            object,
        })
    }

    /// The deletion of a table of the file that sits directly on its page, when the writer
    /// can remove it.
    fn table_deletion(&self, s: &Shape, objects: Option<&[Vec<u32>]>) -> Option<TableDeletion> {
        if !matches!(s.kind, ShapeKind::Table { .. }) || s.group.is_some() {
            return None;
        }
        let l = self.links.get(&s.id)?;
        let object = *objects?.get(l.page)?.get(l.shape)?;
        pub_write::delete_table(&self.original, object).ok()?;
        Some(TableDeletion {
            page: l.page,
            shape: l.shape,
            id: s.id,
            object,
        })
    }

    /// A new rectangle, oval or line as a plan entry, or the reason it cannot be saved.
    fn new_shape(
        &self,
        s: &Shape,
        page_index: usize,
        seq: usize,
    ) -> Result<NewShape, &'static str> {
        use pub_write::NewShapeKind as K;
        let pt = EMU_PER_POINT as f64;
        let is_line = matches!(s.kind, ShapeKind::Line);
        if s.rotation_deg != 0.0 || s.group.is_some() {
            return Err("new shapes that are rotated or grouped");
        }
        if s.rect.w < 0 || s.rect.h < 0 || (!is_line && (s.rect.w == 0 || s.rect.h == 0)) {
            return Err("new shapes without a size");
        }
        if is_line && s.fill.is_some() {
            return Err("new lines with a fill");
        }
        // Publisher's defaults (spec 4N): no fill, a 0.75 pt scheme-colour outline.
        let default_weight = 9_525;
        let stroke = s.line.unwrap_or(Stroke {
            color: self.default_line_color(),
            width_emu: default_weight,
        });
        if is_line && s.line.is_none() {
            return Err("new lines without an outline");
        }
        let style = pub_write::ShapeStyle {
            no_line: s.line.is_none(),
            fill: s.fill.map(|c| [c.0, c.1, c.2]),
            line: (stroke.color != self.default_line_color()).then_some([
                stroke.color.0,
                stroke.color.1,
                stroke.color.2,
            ]),
            line_weight_pt: (stroke.width_emu != default_weight)
                .then(|| stroke.width_emu as f64 / pt),
        };
        if stroke.width_emu <= 0 {
            return Err("new shapes without an outline weight");
        }
        let kind = match s.kind {
            ShapeKind::Rectangle => K::Rectangle,
            ShapeKind::Ellipse => K::Oval,
            _ => K::Line {
                flip_h: s.flip_h,
                flip_v: s.flip_v,
            },
        };
        Ok(NewShape {
            id: s.id,
            page_index,
            seq,
            kind,
            left_pt: s.rect.x as f64 / pt,
            top_pt: s.rect.y as f64 / pt,
            width_pt: s.rect.w as f64 / pt,
            height_pt: s.rect.h as f64 / pt,
            style,
        })
    }

    /// Groups made or dissolved on one page since the baseline.
    fn plan_groups(
        &self,
        page: usize,
        bp: &Page,
        cp: &Page,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        let members = |p: &Page, g: ShapeId| -> Vec<ShapeId> {
            p.shapes
                .iter()
                .filter(|s| s.group == Some(g))
                .map(|s| s.id)
                .collect()
        };
        fn groups(p: &Page) -> Vec<&Shape> {
            p.shapes
                .iter()
                .filter(|s| matches!(s.kind, ShapeKind::Group))
                .collect()
        }
        for g in groups(cp) {
            let now = members(cp, g.id);
            match bp.shape(g.id) {
                Some(_) => {
                    if members(bp, g.id) != now && !plan_has_deleted(bp, cp, &now) {
                        unsupported.add("changes to group membership");
                    }
                }
                None => {
                    let shapes: Option<Vec<usize>> = now
                        .iter()
                        .map(|id| {
                            let b = bp.shape(*id)?;
                            (b.group.is_none() && !matches!(b.kind, ShapeKind::Group))
                                .then(|| self.links.get(id).map(|l| l.shape))
                                .flatten()
                        })
                        .collect();
                    match shapes {
                        Some(shapes) if g.group.is_none() && shapes.len() >= 2 => {
                            plan.groups.push(GroupChange::Group {
                                page,
                                group: g.id,
                                members: now,
                                shapes,
                            });
                        }
                        _ => unsupported.add("nested groups or grouping new objects"),
                    }
                }
            }
        }
        for g in groups(bp) {
            if cp.shape(g.id).is_some() {
                continue;
            }
            let was = members(bp, g.id);
            // Deleted together with its members: reported as deleted objects.
            if was.iter().any(|id| cp.shape(*id).is_none()) {
                continue;
            }
            let nested = g.group.is_some()
                || was
                    .iter()
                    .any(|id| matches!(bp.shape(*id).map(|s| &s.kind), Some(ShapeKind::Group)));
            match self.links.get(&g.id) {
                Some(l) if !nested => plan.groups.push(GroupChange::Ungroup {
                    page,
                    group: g.id,
                    shape: l.shape,
                    members: was,
                }),
                _ => unsupported.add("ungrouping nested groups"),
            }
        }
    }

    fn diff_shape(
        &self,
        bs: &Shape,
        cs: &Shape,
        link: &Link,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
        text_units: &mut Vec<usize>,
    ) {
        if std::mem::discriminant(&bs.kind) != std::mem::discriminant(&cs.kind) {
            unsupported.add("changes to the type of an object");
            return;
        }
        if bs.wrap != cs.wrap {
            if pub_write::set_shape_wrap(&self.original, link.page, link.shape, cs.wrap).is_ok() {
                plan.wraps.push(WrapChange {
                    page: link.page,
                    shape: link.shape,
                    id: cs.id,
                    wrap: cs.wrap,
                });
            } else {
                unsupported.add("wrap type changes");
            }
        }
        if bs.columns != cs.columns || bs.column_gap_emu != cs.column_gap_emu {
            let gap = u32::try_from(cs.column_gap_emu).unwrap_or(0);
            if pub_write::set_text_columns(&self.original, link.page, link.shape, cs.columns, gap)
                .is_ok()
            {
                plan.columns.push(ColumnChange {
                    page: link.page,
                    shape: link.shape,
                    id: cs.id,
                    columns: cs.columns,
                    gap_emu: cs.column_gap_emu,
                });
            } else {
                unsupported.add("text box column changes");
            }
        }
        let mut look = pub_write::ShapeLook::default();
        let rgb = |c: Rgb| [c.0, c.1, c.2];
        if bs.fill != cs.fill {
            look.fill = Some(cs.fill.map(rgb));
        }
        if bs.line != cs.line {
            look.line_on = (bs.line.is_some() != cs.line.is_some()).then_some(cs.line.is_some());
            if let Some(l) = cs.line {
                if bs.line.is_none_or(|b| b.color != l.color) {
                    look.line_color = Some(rgb(l.color));
                }
                if bs.line.is_none_or(|b| b.width_emu != l.width_emu) {
                    match u32::try_from(l.width_emu) {
                        Ok(w) => look.line_weight_emu = Some(w),
                        Err(_) => unsupported.add("outline weights out of range"),
                    }
                }
            }
        }
        if bs.rotation_deg != cs.rotation_deg {
            look.rotation_deg = Some(cs.rotation_deg);
        }
        if bs.flip_h != cs.flip_h {
            look.flip_h = Some(cs.flip_h);
        }
        if bs.flip_v != cs.flip_v {
            look.flip_v = Some(cs.flip_v);
        }
        if look != pub_write::ShapeLook::default() {
            plan.looks.push(LookChange {
                page: link.page,
                shape: link.shape,
                id: cs.id,
                look,
            });
        }
        if bs.rect != cs.rect {
            let is_table = matches!(cs.kind, ShapeKind::Table { .. });
            let resized = bs.rect.w != cs.rect.w || bs.rect.h != cs.rect.h;
            if link.group_rotated {
                unsupported.add("moving or resizing objects inside a rotated group");
            } else if is_table && table_grid_changed(&bs.kind, &cs.kind) {
                // The table is written again with its new grid and position (diff_table).
            } else if is_table && resized {
                unsupported.add("resizing tables");
            } else {
                plan.frames.push(FrameChange {
                    page: link.page,
                    shape: link.shape,
                    id: cs.id,
                    rect: cs.rect,
                });
            }
        }
        match (&bs.kind, &cs.kind) {
            (ShapeKind::TextBox { paragraphs: bp }, ShapeKind::TextBox { paragraphs: cp })
                if bp != cp =>
            {
                match self.unit_of(cs.id) {
                    Some(_) => {
                        let ui = self.units.iter().position(|u| u.frames.contains(&cs.id));
                        text_units.extend(ui);
                    }
                    None => unsupported.add("text typed into text boxes that had no text story"),
                }
            }
            (a @ ShapeKind::Table { .. }, b @ ShapeKind::Table { .. }) if a != b => {
                self.diff_table(a, b, cs, link, plan, unsupported);
            }
            (
                ShapeKind::Picture { data: before, .. },
                ShapeKind::Picture {
                    data, format, file, ..
                },
            ) => {
                if before != data {
                    self.diff_picture(cs.id, link, data, *format, file.as_ref(), plan, unsupported);
                }
            }
            (a, b) if a != b && !matches!(a, ShapeKind::TextBox { .. }) => {
                unsupported.add(match b {
                    ShapeKind::Table { .. } => "changes to table contents",
                    ShapeKind::Picture { .. } => "picture changes",
                    _ => "object changes",
                });
            }
            _ => {}
        }
    }

    /// A picture that shows another image: supported for PNG and JPEG when the writer can
    /// swap it.
    #[allow(clippy::too_many_arguments)]
    fn diff_picture(
        &self,
        id: ShapeId,
        link: &Link,
        data: &Arc<Vec<u8>>,
        format: PictureFormat,
        file: Option<&PictureFile>,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        let object = object_numbers(&self.original)
            .and_then(|o| o.get(link.page)?.get(link.shape).copied());
        let (Some(kind), Some(object)) = (picture_kind(format), object) else {
            unsupported.add("picture changes");
            return;
        };
        let (file_name, modified) = picture_file(file, kind);
        let source = pub_write::PictureSource {
            image: data,
            format: kind,
            file_name: &file_name,
            modified,
        };
        if pub_write::replace_picture(&self.original, object, source).is_err() {
            unsupported.add("picture changes");
            return;
        }
        plan.picture_changes.push(PictureChange {
            page: link.page,
            shape: link.shape,
            id,
            object,
            data: Arc::clone(data),
            format: kind,
            file_name,
            modified,
        });
    }

    /// A table whose cells changed: supported when only the text of cells changed.
    fn diff_table(
        &self,
        before: &ShapeKind,
        after: &ShapeKind,
        cs: &Shape,
        link: &Link,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        let id = cs.id;
        if table_grid_changed(before, after) {
            self.plan_table_rebuild(before, cs, link, plan, unsupported);
            return;
        }
        let (
            ShapeKind::Table {
                rows: br,
                cols: bc,
                col_widths_emu: bw,
                row_heights_emu: bh,
                cells: b,
                ..
            },
            ShapeKind::Table {
                rows: ar,
                cols: ac,
                col_widths_emu: aw,
                row_heights_emu: ah,
                cells: a,
                ..
            },
        ) = (before, after)
        else {
            return;
        };
        let same_grid = br == ar
            && bc == ac
            && bw == aw
            && bh == ah
            && b.len() == a.len()
            && b.iter().zip(a).all(|(x, y)| x.margins_emu == y.margins_emu);
        let Some(&story_index) = self.tables.get(&id).filter(|_| same_grid) else {
            unsupported.add("changes to table contents");
            return;
        };
        let text = |c: &crate::model::TableCell| {
            c.paragraphs
                .iter()
                .map(Paragraph::text)
                .collect::<Vec<_>>()
                .join("\n")
        };
        let (bt, at): (Vec<String>, Vec<String>) =
            (b.iter().map(text).collect(), a.iter().map(text).collect());
        let flat = |cells: &[crate::model::TableCell]| -> Vec<Paragraph> {
            cells.iter().flat_map(|c| c.paragraphs.clone()).collect()
        };
        let (bp, ap) = (flat(b), flat(a));
        let flags: Vec<bool> = b
            .iter()
            .zip(a)
            .flat_map(|(x, y)| changed_flags(&x.paragraphs, &y.paragraphs))
            .collect();
        if flags.iter().any(|&c| c) {
            let (chars, paras, _) = format_spans(&bp, &ap, &flags);
            plan.formats.push(FormatChange {
                story_index,
                table: Some(id),
                chars,
                paras,
                lists: Vec::new(),
            });
        }
        if bt != at {
            plan.tables.push(TableChange {
                id,
                story_index,
                cells: at,
            });
        }
    }

    /// A table of the file whose rows or columns changed: planned as a replacement, or
    /// reported when the writer cannot do it.
    fn plan_table_rebuild(
        &self,
        before: &ShapeKind,
        cs: &Shape,
        link: &Link,
        plan: &mut SavePlan,
        unsupported: &mut Unsupported,
    ) {
        const WHY: &str = "row and column changes in this table";
        if cs.rotation_deg != 0.0 || cs.group.is_some() || !self.tables.contains_key(&cs.id) {
            unsupported.add(WHY);
            return;
        }
        let merged_before = matches!(before, ShapeKind::Table { cells, .. } if cells.iter().any(|c| c.merged));
        let Ok(parts) = table_parts(cs) else {
            unsupported.add(WHY);
            return;
        };
        let object = object_numbers(&self.original)
            .and_then(|o| o.get(link.page)?.get(link.shape).copied());
        let Some(object) = object.filter(|_| !merged_before) else {
            unsupported.add(WHY);
            return;
        };
        let ok = pub_write::replace_table(
            &self.original,
            object,
            cs.rect.x,
            cs.rect.y,
            &parts.widths,
            &parts.heights,
            &parts.texts,
        )
        .is_ok();
        if !ok {
            unsupported.add(WHY);
            return;
        }
        plan.table_rebuilds.push(TableRebuild {
            page: link.page,
            shape: link.shape,
            id: cs.id,
            object,
            left_emu: cs.rect.x,
            top_emu: cs.rect.y,
            widths: parts.widths,
            heights: parts.heights,
            texts: parts.texts,
            paragraphs: parts.paragraphs,
            new_object: None,
            order_restored: false,
        });
    }

    /// Produce the saved file from the original bytes: apply every supported change.
    pub fn build(&self, doc: &Document) -> Result<(Vec<u8>, SavePlan), SaveError> {
        let mut plan = self.plan(doc);
        let rebased = self.rebased(doc);
        let eff = self.without_blocked_duplicates(&rebased).0;
        let doc = &*eff;
        let mut bytes: Vec<u8> = self.original.as_ref().clone();
        for c in &plan.picture_changes {
            let source = pub_write::PictureSource {
                image: &c.data,
                format: c.format,
                file_name: &c.file_name,
                modified: c.modified,
            };
            bytes = pub_write::replace_picture(&bytes, c.object, source)?;
        }
        for l in &plan.looks {
            bytes = pub_write::set_shape_style(&bytes, l.page, l.shape, &l.look)?;
        }
        for w in &plan.wraps {
            bytes = pub_write::set_shape_wrap(&bytes, w.page, w.shape, w.wrap)?;
        }
        for c in &plan.columns {
            let gap = u32::try_from(c.gap_emu).unwrap_or(0);
            bytes = pub_write::set_text_columns(&bytes, c.page, c.shape, c.columns, gap)?;
        }
        for f in &plan.frames {
            let page = doc
                .pages
                .iter()
                .find(|p| p.shape(f.id).is_some())
                .unwrap_or(&doc.pages[f.page.min(doc.pages.len() - 1)]);
            let (pw, ph) = (page.width_emu, page.height_emu);
            let to_i32 =
                |v: i64| i32::try_from(v).map_err(|_| pub_write::WriteError::OutOfRange(v));
            let to_u32 =
                |v: i64| u32::try_from(v).map_err(|_| pub_write::WriteError::OutOfRange(v));
            bytes = pub_write::set_shape_frame(
                &bytes,
                f.page,
                f.shape,
                to_i32(f.rect.x - pw / 2)?,
                to_i32(f.rect.y - ph / 2)?,
                to_u32(f.rect.w)?,
                to_u32(f.rect.h)?,
            )?;
        }
        for s in &plan.stories {
            bytes = pub_write::set_story_text_with_fields(&bytes, s.story_index, &s.text, &s.fields)?;
        }
        for t in &plan.tables {
            bytes = pub_write::set_table_text(&bytes, t.story_index, &t.cells)?;
        }
        for f in &plan.formats {
            bytes = pub_write::set_story_format(&bytes, f.story_index, &f.chars, &f.paras)?;
            if !f.lists.is_empty() {
                bytes = pub_write::set_story_lists(&bytes, f.story_index, &f.lists)?;
            }
        }
        for o in &plan.orders {
            bytes = pub_write::set_shape_order(&bytes, o.page, &o.order)?;
        }
        if !plan.groups.is_empty() {
            let objs = object_numbers(&self.original)
                .ok_or_else(|| SaveError::Verify("cannot read the source file".into()))?;
            for g in &plan.groups {
                // The earlier steps move shapes around in the file: find them by object.
                let now = object_numbers(&bytes)
                    .ok_or_else(|| SaveError::Verify("cannot read the edited file".into()))?;
                let find = |page: usize, shape: usize| -> Result<usize, SaveError> {
                    let want = objs[page][shape];
                    now[page]
                        .iter()
                        .position(|o| *o == want)
                        .ok_or_else(|| SaveError::Verify("a grouped object went missing".into()))
                };
                bytes = match g {
                    GroupChange::Group { page, shapes, .. } => {
                        let ix = shapes
                            .iter()
                            .map(|s| find(*page, *s))
                            .collect::<Result<Vec<_>, _>>()?;
                        pub_write::group_shapes(&bytes, *page, &ix)?
                    }
                    GroupChange::Ungroup { page, shape, .. } => {
                        pub_write::ungroup(&bytes, *page, find(*page, *shape)?)?
                    }
                };
            }
        }
        // Deleted pictures: found by object number, as the shape indexes of the source
        // file are no longer used.
        for d in &plan.deleted_pictures {
            bytes = pub_write::delete_picture(&bytes, d.object)?;
        }
        // Deleted tables, then replaced ones (also by object number). Everything that used the
        // source file's story indexes ran above: a table's story goes away with it.
        for d in &plan.deleted_tables {
            bytes = pub_write::delete_table(&bytes, d.object)?;
        }
        for r in &mut plan.table_rebuilds {
            let (b, new_object) = pub_write::replace_table(
                &bytes,
                r.object,
                r.left_emu,
                r.top_emu,
                &r.widths,
                &r.heights,
                &r.texts,
            )?;
            bytes = write_table_format(b, new_object, &r.paragraphs)?;
            r.new_object = Some(new_object);
        }
        if !plan.table_rebuilds.is_empty() {
            bytes = self.restore_stacking(bytes, doc, &mut plan)?;
        }
        // Page operations (see `SavePlan::page_ops` for the whole order).
        for op in &plan.page_ops {
            bytes = match *op {
                PageOp::Delete { index } => pub_write::delete_page(&bytes, index)?,
                PageOp::Move { from, to } => pub_write::move_page(&bytes, from, to)?,
                PageOp::Add { after } => pub_write::add_page(&bytes, after)?,
                PageOp::Duplicate { index } => pub_write::duplicate_page(&bytes, index)?,
            };
        }
        // Page-number sections follow the pages (spec 4T.4).
        if !plan.sections.is_empty() {
            bytes = pub_write::set_sections(&bytes, &plan.sections)?;
        }
        // New text boxes and shapes go LAST: every step above addresses stories and shapes by their
        // index in the original file, and adding a box appends a story and a shape, which
        // would otherwise shift those indices. Creation order is kept.
        for item in new_objects_in_order(&plan) {
            let b = match item {
                NewObject::Table(n) => {
                    let (b, object) = pub_write::add_table_tracks_emu(
                        &bytes,
                        n.page_index,
                        n.left_emu,
                        n.top_emu,
                        &n.widths,
                        &n.heights,
                        &n.texts,
                    )?;
                    bytes = write_table_format(b, object, &n.paragraphs)?;
                    continue;
                }
                NewObject::Picture(n) => {
                    let pic = pub_write::NewPicture {
                        image: &n.data,
                        format: n.format,
                        file_name: &n.file_name,
                        modified: n.modified,
                        left_pt: n.left_pt,
                        top_pt: n.top_pt,
                        width_pt: n.width_pt,
                        height_pt: n.height_pt,
                    };
                    bytes = pub_write::add_picture(&bytes, n.page_index, pic)?.0;
                    continue;
                }
                NewObject::Shape(n) => {
                    bytes = pub_write::add_shape(
                        &bytes,
                        n.page_index,
                        n.kind,
                        n.left_pt,
                        n.top_pt,
                        n.width_pt,
                        n.height_pt,
                        n.style,
                    )?;
                    continue;
                }
                NewObject::TextBox(b) => b,
            };
            bytes = pub_write::add_text_box(
                &bytes,
                b.page_index,
                b.left_pt,
                b.top_pt,
                b.width_pt,
                b.height_pt,
                &b.text,
            )?;
            if b.columns > 1 {
                // The new box is the last shape of its page.
                let doc = pf::read_bytes(&bytes).map_err(|e| SaveError::Verify(e.to_string()))?;
                let shape = doc
                    .pages
                    .get(b.page_index)
                    .and_then(|p| p.shapes.len().checked_sub(1))
                    .ok_or_else(|| SaveError::Verify("the new text box is missing".into()))?;
                let gap = u32::try_from(b.column_gap_emu).unwrap_or(0);
                bytes = pub_write::set_text_columns(&bytes, b.page_index, shape, b.columns, gap)?;
            }
            if b.paragraphs.iter().any(|p| !is_plain_para(p)) {
                // The new box is the last shape of its page; read its story back.
                let doc = pf::read_bytes(&bytes).map_err(|e| SaveError::Verify(e.to_string()))?;
                let numbers = doc.story_numbers();
                let story_index = doc
                    .pages
                    .get(b.page_index)
                    .and_then(|p| p.shapes.last())
                    .and_then(|s| s.story)
                    .and_then(|n| numbers.iter().position(|&x| x == n))
                    .ok_or_else(|| SaveError::Verify("the new text box has no story".into()))?;
                let flags = vec![true; b.paragraphs.len()];
                let (chars, paras, lists) = format_spans(&[], &b.paragraphs, &flags);
                bytes = pub_write::set_story_format(&bytes, story_index, &chars, &paras)?;
                if !lists.is_empty() {
                    bytes = pub_write::set_story_lists(&bytes, story_index, &lists)?;
                }
            }
        }
        // Page setup last (see `SavePlan::page_setup`). The writer refuses cases it has not
        // seen Publisher do (a picture, group, table or rotated shape leaving the page,
        // facing-page masters, ...): the change is then left out and reported.
        if let Some(ps) = plan.page_setup.take() {
            let mut applied = PageSetupChange::default();
            let to_u32 = |v: i64| u32::try_from(v).ok();
            if let Some((w, h)) = ps.size {
                let r = match (to_u32(w), to_u32(h)) {
                    (Some(w), Some(h)) => pub_write::set_page_size(&bytes, w, h),
                    _ => Err(pub_write::WriteError::PageSetup(
                        "the page size is out of range".into(),
                    )),
                };
                match r {
                    Ok(b) => {
                        bytes = b;
                        applied.size = Some((w, h));
                    }
                    Err(pub_write::WriteError::PageSetup(why)) => {
                        plan.unsupported.push(format!("page size changes ({why})"));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(m) = ps.margins {
                let r = match m.to_array().map(to_u32) {
                    [Some(l), Some(r), Some(t), Some(b)] => {
                        pub_write::set_margins(&bytes, l, r, t, b)
                    }
                    _ => Err(pub_write::WriteError::PageSetup(
                        "a margin is out of range".into(),
                    )),
                };
                match r {
                    Ok(b) => {
                        bytes = b;
                        applied.margins = Some(m);
                    }
                    Err(pub_write::WriteError::PageSetup(why)) => {
                        plan.unsupported.push(format!("margin changes ({why})"));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            plan.page_setup = (applied != PageSetupChange::default()).then_some(applied);
        }
        if !self.counter_bumped {
            bytes = pub_write::bump_save_counter(&bytes)?;
        }
        pub_format::read_bytes(&bytes).map_err(|e| SaveError::Verify(e.to_string()))?;
        Ok((bytes, plan))
    }

    /// Re-point the links at the saved file's shape order and bring the baseline's groups
    /// up to date. Must run before `original` is replaced.
    fn mark_groups_saved(&mut self, new_bytes: &[u8], doc: &Document, plan: &SavePlan) {
        let (Some(old), Some(new)) = (object_numbers(&self.original), object_numbers(new_bytes))
        else {
            return;
        };
        // Page operations of the same save move pages: `new` is indexed by final page.
        let pm = page_map(self.baseline.pages.len(), &plan.page_ops);
        let fin = |page: usize| pm.get(page).copied().flatten();
        // The editor page that a page of the opened file is.
        let doc_ix: Vec<Option<usize>> = self
            .baseline
            .pages
            .iter()
            .map(|b| doc.pages.iter().position(|d| d.id == b.id))
            .collect();
        let mut keep = HashMap::new();
        // Replaced tables have new object numbers.
        let renamed: HashMap<u32, u32> = plan
            .table_rebuilds
            .iter()
            .filter_map(|r| Some((r.object, r.new_object?)))
            .collect();
        for (id, l) in &self.links {
            let Some(obj) = old.get(l.page).and_then(|p| p.get(l.shape)) else {
                continue;
            };
            let obj = renamed.get(obj).unwrap_or(obj);
            let Some(np) = fin(l.page) else { continue };
            if let Some(shape) = new
                .get(np)
                .and_then(|p| p.iter().position(|o| o == obj))
            {
                keep.insert(*id, Link { shape, ..*l });
            }
        }
        let new_doc = pf::read_bytes(new_bytes).ok();
        for g in &plan.groups {
            match g {
                GroupChange::Group {
                    page,
                    group,
                    members,
                    ..
                } => {
                    // The new group is the parent of the members in the saved file.
                    let parent = members.first().and_then(|m| {
                        let l = keep.get(m)?;
                        new_doc
                            .as_ref()?
                            .pages
                            .get(fin(l.page)?)?
                            .shapes
                            .get(l.shape)?
                            .group
                    });
                    if let Some(shape) = parent.and_then(|o| {
                        new.get(fin(*page)?)?.iter().position(|x| *x == o)
                    }) {
                        keep.insert(
                            *group,
                            Link {
                                page: *page,
                                shape,
                                group_rotated: false,
                            },
                        );
                    }
                    if let (Some(gs), Some(bp)) = (
                        doc_ix[*page]
                            .and_then(|i| doc.pages.get(i))
                            .and_then(|p| p.shape(*group)),
                        self.baseline.pages.get_mut(*page),
                    ) {
                        bp.shapes.push(gs.clone());
                        for s in bp.shapes.iter_mut().filter(|s| members.contains(&s.id)) {
                            s.group = Some(*group);
                        }
                        // Keep the baseline in the document's stacking order.
                        if let Some(dp) = doc_ix[*page].and_then(|i| doc.pages.get(i)) {
                            bp.shapes
                                .sort_by_key(|s| dp.shapes.iter().position(|x| x.id == s.id));
                        }
                    }
                }
                GroupChange::Ungroup {
                    page,
                    group,
                    members,
                    ..
                } => {
                    keep.remove(group);
                    if let Some(bp) = self.baseline.pages.get_mut(*page) {
                        bp.shapes.retain(|s| s.id != *group);
                        for s in bp.shapes.iter_mut().filter(|s| members.contains(&s.id)) {
                            s.group = None;
                        }
                    }
                }
            }
        }
        self.links = keep;
    }

    /// Give the boxes just added a text unit, read back from the saved file, so later
    /// text edits to them save like any other box. Stories are numbered by sorted object
    /// number, so existing story indexes are remapped in case the new ones sort among them.
    fn register_new_stories(&mut self, new_bytes: &[u8], plan: &SavePlan, dup_text: &[ShapeId]) {
        let (Ok(old), Ok(new)) = (pf::read_bytes(&self.original), pf::read_bytes(new_bytes)) else {
            return;
        };
        let numbers = |d: &pf::Document| d.story_numbers();
        let (old_n, new_n) = (numbers(&old), numbers(&new));
        let remap = |i: usize| old_n.get(i).and_then(|n| new_n.iter().position(|x| x == n));
        for u in &mut self.units {
            if let Some(i) = remap(u.story_index) {
                u.story_index = i;
            }
        }
        for i in self.tables.values_mut() {
            if let Some(j) = remap(*i) {
                *i = j;
            }
        }
        let new_ids = plan
            .new_text_boxes
            .iter()
            .map(|b| b.id)
            .chain(dup_text.iter().copied());
        for id in new_ids {
            let Some(l) = self.links.get(&id) else {
                continue;
            };
            let Some(number) = new
                .pages
                .get(l.page)
                .and_then(|p| p.shapes.get(l.shape))
                .and_then(|s| s.story)
            else {
                continue;
            };
            if let Some(story_index) = new_n.iter().position(|&n| n == number) {
                self.units.push(Unit {
                    story_index,
                    frames: vec![id],
                    mid_para: vec![false],
                    tail: Vec::new(),
                    tail_mid: false,
                });
            }
        }
    }

    /// A replaced table is the top-most shape of its page: put the page's shapes back in the
    /// editor's stacking order. Pages with groups are left alone (the next save reports it).
    fn restore_stacking(
        &self,
        mut bytes: Vec<u8>,
        doc: &Document,
        plan: &mut SavePlan,
    ) -> Result<Vec<u8>, SaveError> {
        let Some(old) = object_numbers(&self.original) else {
            return Ok(bytes);
        };
        let renamed: HashMap<u32, u32> = plan
            .table_rebuilds
            .iter()
            .filter_map(|r| Some((r.object, r.new_object?)))
            .collect();
        let mut pages: Vec<usize> = plan.table_rebuilds.iter().map(|r| r.page).collect();
        pages.sort_unstable();
        pages.dedup();
        for pi in pages {
            let Some(bp) = self.baseline.pages.get(pi) else {
                continue;
            };
            let Some(dp) = doc.pages.iter().find(|p| p.id == bp.id) else {
                continue;
            };
            let group = |s: &Shape| matches!(s.kind, ShapeKind::Group);
            if dp.shapes.iter().chain(&bp.shapes).any(group) {
                continue;
            }
            let Some(now) = object_numbers(&bytes).and_then(|n| n.get(pi).cloned()) else {
                continue;
            };
            let want: Vec<u32> = dp
                .shapes
                .iter()
                .filter_map(|s| {
                    let l = self.links.get(&s.id).filter(|l| l.page == pi)?;
                    let o = *old.get(pi)?.get(l.shape)?;
                    Some(*renamed.get(&o).unwrap_or(&o))
                })
                .collect();
            let order: Option<Vec<usize>> = want
                .iter()
                .map(|o| now.iter().position(|x| x == o))
                .collect();
            let Some(order) = order.filter(|o| o.len() == now.len()) else {
                continue;
            };
            if order.iter().enumerate().any(|(i, &x)| i != x) {
                bytes = pub_write::set_shape_order(&bytes, pi, &order)?;
            }
            for r in plan.table_rebuilds.iter_mut().filter(|r| r.page == pi) {
                r.order_restored = true;
            }
        }
        Ok(bytes)
    }

    /// Point `self.tables` at the story of every table of the saved file (a table that was
    /// added, replaced or deleted changes the story list).
    fn refresh_tables(&mut self, new_bytes: &[u8]) {
        let Ok(new) = pf::read_bytes(new_bytes) else {
            return;
        };
        let numbers = new.story_numbers();
        let mut tables = HashMap::new();
        for (id, l) in &self.links {
            let Some(s) = new.pages.get(l.page).and_then(|p| p.shapes.get(l.shape)) else {
                continue;
            };
            if s.table.is_none() {
                continue;
            }
            if let Some(i) = s.story.and_then(|n| numbers.iter().position(|&x| x == n)) {
                tables.insert(*id, i);
            }
        }
        self.tables = tables;
    }

    /// Record that `bytes` (the result of [`build`](Self::build) for `doc` with `plan`)
    /// were written: the file now holds the supported changes, so only changes that are
    /// still unsaved remain in later plans.
    pub fn mark_saved(&mut self, bytes: Vec<u8>, doc: &Document, plan: &SavePlan) {
        let rebased = self.rebased(doc);
        let eff = self.without_blocked_duplicates(&rebased).0;
        let doc = &*eff;
        for d in &plan.deleted_pictures {
            for p in &mut self.baseline.pages {
                p.shapes.retain(|s| s.id != d.id);
            }
        }
        for d in &plan.deleted_tables {
            for p in &mut self.baseline.pages {
                p.shapes.retain(|s| s.id != d.id);
            }
        }
        // (see `mark_groups_saved` for the group steps; it also re-points the links
        // after pictures were deleted)
        // A size change can move shapes to the scratch area, which takes them out of their
        // page's shape list: the links follow the file's new order and drop those shapes.
        let resized = plan.page_setup.as_ref().is_some_and(|p| p.size.is_some());
        if !plan.groups.is_empty()
            || !plan.deleted_pictures.is_empty()
            || !plan.deleted_tables.is_empty()
            || !plan.table_rebuilds.is_empty()
            || resized
        {
            self.mark_groups_saved(&bytes, doc, plan);
        }
        if !plan.sections.is_empty() {
            self.baseline.sections = doc.sections.clone();
        }
        // Page operations: the baseline and the links follow the file's new page list
        // (links still hold the original page indexes here).
        let pm = page_map(self.baseline.pages.len(), &plan.page_ops);
        // Duplicated pages: each copy shape sits at the file index of its source shape (read
        // before the links follow the page moves).
        let mut dup_links: Vec<(ShapeId, usize, usize)> = Vec::new();
        let mut dup_pages: Vec<PageId> = Vec::new();
        for (copy, src) in &plan.duplicated {
            let (Some(k), Some(sp)) = (
                doc.pages.iter().position(|p| p.id == *copy),
                doc.pages.iter().find(|p| p.id == *src),
            ) else {
                continue;
            };
            dup_pages.push(*copy);
            for (c, s) in doc.pages[k].shapes.iter().zip(&sp.shapes) {
                if let Some(l) = self.links.get(&s.id) {
                    dup_links.push((c.id, k, l.shape));
                }
            }
        }
        if !plan.page_ops.is_empty() {
            let gone: Vec<ShapeId> = self
                .links
                .iter()
                .filter(|(_, l)| pm.get(l.page).copied().flatten().is_none())
                .map(|(id, _)| *id)
                .collect();
            self.links.retain(|id, _| !gone.contains(id));
            for l in self.links.values_mut() {
                l.page = pm[l.page].unwrap_or(l.page);
            }
            self.units.retain(|u| !u.frames.iter().any(|f| gone.contains(f)));
            self.tables.retain(|id, _| !gone.contains(id));
            let old = std::mem::take(&mut self.baseline.pages);
            self.baseline.pages = doc
                .pages
                .iter()
                .map(|dp| {
                    old.iter().find(|b| b.id == dp.id).cloned().unwrap_or_else(|| Page {
                        shapes: if dup_pages.contains(&dp.id) {
                            dp.shapes.clone()
                        } else {
                            Vec::new()
                        },
                        ..dp.clone()
                    })
                })
                .collect();
            for &(id, page, shape) in &dup_links {
                self.links.insert(
                    id,
                    Link {
                        page,
                        shape,
                        group_rotated: false,
                    },
                );
            }
        }
        // New text boxes and shapes were appended to the end of their page: the baseline
        // now holds them (so a second save does not add them again) and they get links at
        // their new positions (only the last `n` shapes of the page, in creation order).
        if !plan.new_text_boxes.is_empty()
            || !plan.new_shapes.is_empty()
            || !plan.new_pictures.is_empty()
            || !plan.new_tables.is_empty()
        {
            let counts = object_numbers(&bytes);
            let mut per_page: Vec<(usize, Vec<ShapeId>)> = Vec::new();
            for item in new_objects_in_order(plan) {
                let (page_index, id) = match item {
                    NewObject::TextBox(b) => (b.page_index, b.id),
                    NewObject::Shape(n) => (n.page_index, n.id),
                    NewObject::Picture(n) => (n.page_index, n.id),
                    NewObject::Table(n) => (n.page_index, n.id),
                };
                match per_page.iter_mut().find(|(p, _)| *p == page_index) {
                    Some((_, v)) => v.push(id),
                    None => per_page.push((page_index, vec![id])),
                }
            }
            for (page_index, ids) in per_page {
                let total = counts
                    .as_ref()
                    .and_then(|c| c.get(page_index))
                    .map_or(0, Vec::len);
                for (k, id) in ids.iter().enumerate() {
                    if let (Some(shape), Some(bp)) = (
                        find_shape(doc, *id).cloned(),
                        self.baseline.pages.get_mut(page_index),
                    ) {
                        bp.shapes.push(shape);
                    }
                    if total >= ids.len() {
                        self.links.insert(
                            *id,
                            Link {
                                page: page_index,
                                shape: total - ids.len() + k,
                                group_rotated: false,
                            },
                        );
                    }
                }
            }
        }
        let table_ops = !plan.new_tables.is_empty()
            || !plan.table_rebuilds.is_empty()
            || !plan.deleted_tables.is_empty();
        if !plan.new_text_boxes.is_empty() || !dup_links.is_empty() || table_ops {
            let dup_text: Vec<ShapeId> = dup_links
                .iter()
                .map(|d| d.0)
                .filter(|id| find_shape(doc, *id).is_some_and(Shape::is_text_box))
                .collect();
            self.register_new_stories(&bytes, plan, &dup_text);
            self.refresh_tables(&bytes);
        }
        self.counter_bumped = true;
        self.original = Arc::new(bytes);
        for l in &plan.looks {
            let now = doc
                .pages
                .iter()
                .flat_map(|p| &p.shapes)
                .find(|s| s.id == l.id);
            if let (Some(s), Some(n)) = (baseline_shape_mut(&mut self.baseline, l.id), now) {
                (s.fill, s.line, s.rotation_deg) = (n.fill, n.line, n.rotation_deg);
                (s.flip_h, s.flip_v) = (n.flip_h, n.flip_v);
            }
        }
        for c in &plan.picture_changes {
            if let (Some(cur), Some(b)) = (
                find_shape(doc, c.id),
                baseline_shape_mut(&mut self.baseline, c.id),
            ) {
                b.kind = cur.kind.clone();
            }
        }
        for w in &plan.wraps {
            if let Some(s) = baseline_shape_mut(&mut self.baseline, w.id) {
                s.wrap = w.wrap;
            }
        }
        for c in &plan.columns {
            if let Some(s) = baseline_shape_mut(&mut self.baseline, c.id) {
                (s.columns, s.column_gap_emu) = (c.columns, c.gap_emu);
            }
        }
        for f in &plan.frames {
            if let Some(s) = baseline_shape_mut(&mut self.baseline, f.id) {
                s.rect = f.rect;
            }
        }
        // A replaced table is the top-most shape of its page in the saved file unless its
        // page's stacking order was put back.
        for r in &plan.table_rebuilds {
            let Some(cur) = find_shape(doc, r.id) else {
                continue;
            };
            for p in &mut self.baseline.pages {
                if let Some(i) = p.shapes.iter().position(|s| s.id == r.id) {
                    if r.order_restored {
                        p.shapes[i] = cur.clone();
                    } else {
                        p.shapes.remove(i);
                        p.shapes.push(cur.clone());
                    }
                    break;
                }
            }
        }
        for o in &plan.orders {
            let fp = pm.get(o.page).copied().flatten().unwrap_or(o.page);
            if let Some(page) = self.baseline.pages.get_mut(fp) {
                page.shapes
                    .sort_by_key(|s| o.ids.iter().position(|i| *i == s.id));
            }
            // The file now lists this page's shapes in the new order.
            for (pos, id) in o.ids.iter().enumerate() {
                if let Some(l) = self.links.get_mut(id) {
                    l.shape = pos;
                }
            }
        }
        // Everything above was recorded at the file's old page size; now bring the baseline
        // to the new size and margins.
        if let Some(ps) = &plan.page_setup {
            for bp in &mut self.baseline.pages {
                if let Some((w, h)) = ps.size {
                    let (dx, dy) = (w / 2 - bp.width_emu / 2, h / 2 - bp.height_emu / 2);
                    for s in &mut bp.shapes {
                        s.rect = s.rect.translated(dx, dy);
                    }
                    (bp.width_emu, bp.height_emu) = (w, h);
                }
                if let Some(m) = ps.margins {
                    bp.margins_emu = m;
                }
            }
        }
        let table_ids = plan
            .tables
            .iter()
            .map(|t| t.id)
            .chain(plan.formats.iter().filter_map(|f| f.table));
        for id in table_ids {
            if let (Some(cur), Some(b)) = (
                find_shape(doc, id),
                baseline_shape_mut(&mut self.baseline, id),
            ) {
                b.kind = cur.kind.clone();
            }
        }
        let story_indexes = plan.stories.iter().map(|s| s.story_index).chain(
            plan.formats
                .iter()
                .filter(|f| f.table.is_none())
                .map(|f| f.story_index),
        );
        for story_index in story_indexes {
            let Some(unit) = self.units.iter().find(|u| u.story_index == story_index) else {
                continue;
            };
            for id in unit.frames.clone() {
                let Some(cur) = find_shape(doc, id).cloned() else {
                    continue;
                };
                if let (Some(b), ShapeKind::TextBox { paragraphs }) =
                    (baseline_shape_mut(&mut self.baseline, id), &cur.kind)
                {
                    b.kind = ShapeKind::TextBox {
                        paragraphs: paragraphs.clone(),
                    };
                }
            }
        }
    }
}

/// Everything the writer needs of a table shape.
struct TableParts {
    widths: Vec<u32>,
    heights: Vec<u32>,
    texts: Vec<String>,
    paragraphs: Vec<Paragraph>,
}

/// Has a table's grid (row or column count, track sizes or cell count) changed?
fn table_grid_changed(before: &ShapeKind, after: &ShapeKind) -> bool {
    match (before, after) {
        (
            ShapeKind::Table {
                rows: br,
                cols: bc,
                col_widths_emu: bw,
                row_heights_emu: bh,
                ..
            },
            ShapeKind::Table {
                rows: ar,
                cols: ac,
                col_widths_emu: aw,
                row_heights_emu: ah,
                ..
            },
        ) => !(br == ar && bc == ac && bw == aw && bh == ah),
        _ => false,
    }
}

fn script_out(s: crate::model::Script) -> pub_format::Script {
    match s {
        crate::model::Script::Normal => pub_format::Script::Normal,
        crate::model::Script::Superscript => pub_format::Script::Superscript,
        crate::model::Script::Subscript => pub_format::Script::Subscript,
    }
}

/// A paragraph with a table's defaults (Calibri 10, left, no spacing): what a new cell holds.
fn is_table_plain_para(p: &Paragraph) -> bool {
    p.align == Align::Left
        && p.spacing == ParaSpacing::default()
        && p.runs.iter().all(|r| {
            r.font == DEFAULT_FONT
                && r.size_pt == FILE_DEFAULT_SIZE_PT
                && !r.bold
                && !r.italic
                && !r.underline
                && r.script == crate::model::Script::Normal
                && r.color == Rgb::BLACK
        })
}

/// The tracks, texts and formatted paragraphs of a table shape, or why it cannot be written.
fn table_parts(s: &Shape) -> Result<TableParts, &'static str> {
    let ShapeKind::Table {
        rows,
        cols,
        col_widths_emu,
        row_heights_emu,
        cells,
        ..
    } = &s.kind
    else {
        return Err("new objects");
    };
    if cells.iter().any(|c| c.merged) {
        return Err("tables with merged cells");
    }
    if cells
        .iter()
        .any(|c| c.margins_emu != [TableCell::DEFAULT_MARGIN_EMU; 4])
    {
        return Err("tables with changed cell margins");
    }
    let (r, c) = ((*rows).max(1) as usize, (*cols).max(1) as usize);
    let to_u32 = |v: &i64| u32::try_from(*v).map_err(|_| "tables with a size out of range");
    let widths = crate::table::tracks_of(col_widths_emu, c, s.rect.w)
        .iter()
        .map(to_u32)
        .collect::<Result<Vec<_>, _>>()?;
    let heights = crate::table::tracks_of(row_heights_emu, r, s.rect.h)
        .iter()
        .map(to_u32)
        .collect::<Result<Vec<_>, _>>()?;
    let mut texts = Vec::with_capacity(r * c);
    let mut paragraphs = Vec::new();
    for i in 0..r * c {
        let cell_paras: Vec<Paragraph> = match cells.get(i) {
            Some(cell) if !cell.paragraphs.is_empty() => cell.paragraphs.clone(),
            _ => vec![Paragraph::plain("")],
        };
        texts.push(story_text_of(&cell_paras));
        paragraphs.extend(cell_paras);
    }
    Ok(TableParts {
        widths,
        heights,
        texts,
        paragraphs,
    })
}

/// A new table as a plan entry, or the reason it cannot be saved.
fn new_table_object(
    s: &Shape,
    page_index: usize,
    seq: usize,
) -> Result<NewTableObject, &'static str> {
    if s.rotation_deg != 0.0 || s.group.is_some() {
        return Err("new tables that are rotated or grouped");
    }
    let parts = table_parts(s)?;
    Ok(NewTableObject {
        id: s.id,
        page_index,
        seq,
        left_emu: s.rect.x,
        top_emu: s.rect.y,
        widths: parts.widths,
        heights: parts.heights,
        texts: parts.texts,
        paragraphs: parts.paragraphs,
    })
}

/// Give the cells of table `object` (just written) the character and paragraph formatting of
/// `paragraphs` (all cells in story order); paragraphs with a new table's defaults are left
/// as the writer made them.
fn write_table_format(
    bytes: Vec<u8>,
    object: u32,
    paragraphs: &[Paragraph],
) -> Result<Vec<u8>, SaveError> {
    let flags: Vec<bool> = paragraphs.iter().map(|p| !is_table_plain_para(p)).collect();
    if !flags.iter().any(|&f| f) {
        return Ok(bytes);
    }
    let doc = pf::read_bytes(&bytes).map_err(|e| SaveError::Verify(e.to_string()))?;
    let numbers = doc.story_numbers();
    let story_index = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| s.object_number == object && s.table.is_some())
        .and_then(|s| s.story)
        .and_then(|n| numbers.iter().position(|&x| x == n))
        .ok_or_else(|| SaveError::Verify("the table has no story".into()))?;
    let (chars, paras, _) = format_spans(&[], paragraphs, &flags);
    Ok(pub_write::set_story_format(&bytes, story_index, &chars, &paras)?)
}

fn picture_kind(f: PictureFormat) -> Option<pub_write::PictureKind> {
    match f {
        PictureFormat::Png => Some(pub_write::PictureKind::Png),
        PictureFormat::Jpeg => Some(pub_write::PictureKind::Jpeg),
        _ => None,
    }
}

/// The file name and modification time recorded with a picture; a picture without a known
/// file is named after its format.
fn picture_file(
    file: Option<&PictureFile>,
    kind: pub_write::PictureKind,
) -> (String, Option<u64>) {
    match file {
        Some(f) if !f.name.is_empty() => (f.name.clone(), f.modified),
        _ => (
            match kind {
                pub_write::PictureKind::Png => "picture.png",
                pub_write::PictureKind::Jpeg => "picture.jpg",
            }
            .to_owned(),
            file.and_then(|f| f.modified),
        ),
    }
}

/// A new picture as a plan entry, or the reason it cannot be saved.
fn new_picture(s: &Shape, page_index: usize, seq: usize) -> Result<NewPicture, &'static str> {
    let ShapeKind::Picture { data, format, file } = &s.kind else {
        return Err("new objects");
    };
    if s.rotation_deg != 0.0 || s.group.is_some() {
        return Err("new pictures that are rotated or grouped");
    }
    if s.fill.is_some() || s.line.is_some() {
        return Err("new pictures with a fill or an outline");
    }
    if s.rect.w <= 0 || s.rect.h <= 0 {
        return Err("new pictures without a size");
    }
    let Some(format) = picture_kind(*format) else {
        return Err("new pictures other than PNG or JPEG");
    };
    let (file_name, modified) = picture_file(file.as_ref(), format);
    let pt = EMU_PER_POINT as f64;
    Ok(NewPicture {
        id: s.id,
        page_index,
        seq,
        data: Arc::clone(data),
        format,
        file_name,
        modified,
        left_pt: s.rect.x as f64 / pt,
        top_pt: s.rect.y as f64 / pt,
        width_pt: s.rect.w as f64 / pt,
        height_pt: s.rect.h as f64 / pt,
    })
}

/// A paragraph with no formatting beyond the editor's defaults (what a new box starts with).
fn is_plain_para(p: &Paragraph) -> bool {
    let d = Run::plain("");
    p.align == Align::Left
        && p.spacing == ParaSpacing::default()
        && p.list.is_none()
        && p.indents == crate::model::ParaIndents::default()
        && p.runs.iter().all(|r| {
            r.font == d.font
                && r.size_pt == d.size_pt
                && r.bold == d.bold
                && r.italic == d.italic
                && r.underline == d.underline
                && r.script == d.script
                && r.color == d.color
        })
}

fn plan_has_deleted(bp: &Page, cp: &Page, now: &[ShapeId]) -> bool {
    bp.shapes
        .iter()
        .any(|b| cp.shape(b.id).is_none() && !now.contains(&b.id))
}

/// Object numbers of every page's shapes, in file order.
fn object_numbers(bytes: &[u8]) -> Option<Vec<Vec<u32>>> {
    let doc = pf::read_bytes(bytes).ok()?;
    Some(
        doc.pages
            .iter()
            .map(|p| p.shapes.iter().map(|s| s.object_number).collect())
            .collect(),
    )
}

fn model_shape(doc: &Document, id: ShapeId) -> Option<&Shape> {
    find_shape(doc, id)
}

fn find_shape(doc: &Document, id: ShapeId) -> Option<&Shape> {
    doc.pages.iter().find_map(|p| p.shape(id))
}

fn baseline_shape_mut(doc: &mut Document, id: ShapeId) -> Option<&mut Shape> {
    doc.pages
        .iter_mut()
        .flat_map(|p| p.shapes.iter_mut())
        .find(|s| s.id == id)
}

#[derive(Default)]
struct Unsupported(Vec<String>);

/// What the save reports when a linked text box was deleted.
const LINKED_DELETE: &str = "changes to linked text boxes (deleting a linked text box)";

impl Unsupported {
    fn add(&mut self, what: &str) {
        if !self.0.iter().any(|w| w == what) {
            self.0.push(what.to_owned());
        }
    }
    fn into_list(self) -> Vec<String> {
        self.0
    }
}

/// Write `bytes` to `path` safely: a temp file next to it, then an atomic rename. When
/// `backup` is set and `path` exists, it is first copied to `<path>.bak`.
pub fn write_atomic(path: &Path, bytes: &[u8], backup: bool) -> Result<(), SaveError> {
    let (mut f, tmp) = create_unique_temp(path)?;
    let result = (|| -> Result<(), SaveError> {
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if backup && path.exists() {
            std::fs::copy(path, sibling(path, ".bak"))?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Create a new, uniquely named temp file next to `path` (never follows an existing file or
/// link: `create_new`). The name is the target name plus process id and a counter/time suffix.
pub(crate) fn create_unique_temp(path: &Path) -> std::io::Result<(std::fs::File, PathBuf)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut last = None;
    for _ in 0..8 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = sibling(path, &format!(".{}-{n}-{nanos:x}.tmp", std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => return Ok((f, tmp)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::from(std::io::ErrorKind::AlreadyExists)))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
