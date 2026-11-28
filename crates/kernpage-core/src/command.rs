//! Every edit goes through a [`Command`] run by the [`Editor`], which records a
//! reversible [`Edit`] so it can be undone and redone.

use crate::geometry::Rect;
use crate::model::*;
use crate::table::Axis;
use std::fmt;
use std::sync::Arc;

/// One character/paragraph format change applied to every run of a text box.
#[derive(Debug, Clone, PartialEq)]
pub enum TextFormat {
    FontName(String),
    Size(f32),
    Bold(bool),
    Italic(bool),
    Underline(bool),
    Script(crate::model::Script),
    Color(Rgb),
    Align(Align),
}

/// One character format change applied to a range of text.
#[derive(Debug, Clone, PartialEq)]
pub enum CharChange {
    FontName(String),
    Size(f32),
    Bold(bool),
    Italic(bool),
    Underline(bool),
    Script(crate::model::Script),
    Color(Rgb),
    /// Clear All Formatting: back to the default character formatting.
    Clear,
}

/// One look change applied to every selected shape it makes sense for (Shape Fill, Shape
/// Outline, Outline Weight, Rotate and Flip).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleChange {
    /// Fill colour, or `None` for No Fill (rectangles, ovals and text boxes).
    Fill(Option<Rgb>),
    /// Outline colour, or `None` for No Outline (not pictures). Keeps the weight.
    Outline(Option<Rgb>),
    /// Outline weight in EMU; turns a missing outline on in black.
    OutlineWeight(i64),
    /// Turn clockwise by this many degrees (not groups).
    Rotate(f32),
    /// Lines only (spec 4D covers the flip flags of lines only).
    FlipHorizontal,
    FlipVertical,
    /// Text wrap around the object (Wrap Text); not for groups.
    Wrap(crate::model::WrapType),
    /// Text box columns and the gap between them in EMU (text boxes only).
    Columns(u32, i64),
}

/// How Align / Distribute places objects (Publisher's Arrange > Align menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMode {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
    DistributeHorizontally,
    DistributeVertically,
}

/// A stacking-order change applied to a set of shapes at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZOrder {
    Forward,
    Backward,
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Align or distribute shapes by their visual bounds, relative to each other or (when
    /// `to_margins`) to the page margin guides.
    AlignShapes {
        page: usize,
        ids: Vec<ShapeId>,
        mode: AlignMode,
        to_margins: bool,
    },
    MoveShapes {
        page: usize,
        ids: Vec<ShapeId>,
        dx: i64,
        dy: i64,
    },
    SetRect {
        page: usize,
        id: ShapeId,
        rect: Rect,
    },
    /// Adds one shape on top; its id is replaced by a fresh one.
    AddShape {
        page: usize,
        shape: Shape,
    },
    /// Adds copies of shapes (fresh ids) on top, e.g. for paste.
    PasteShapes {
        page: usize,
        shapes: Vec<Shape>,
    },
    DeleteShapes {
        page: usize,
        ids: Vec<ShapeId>,
    },
    /// Change Picture: a picture frame shows another image, in the given frame. One undo step.
    ChangePicture {
        page: usize,
        id: ShapeId,
        data: Arc<Vec<u8>>,
        format: PictureFormat,
        file: Option<PictureFile>,
        rect: Rect,
    },
    /// Insert > Table: a new `rows` x `columns` table filling `rect` evenly (empty Calibri 10
    /// cells, default margins), on top. See [`crate::table::default_table_rect`] for
    /// Publisher's frame of a table picked in the ribbon grid.
    InsertTable {
        page: usize,
        rect: Rect,
        rows: u32,
        columns: u32,
    },
    /// Layout > Insert Above / Below: `count` empty rows before row `before` (0-based; the
    /// row count appends). The new rows copy the height of the row they go before (the last
    /// row when appending) and the table grows.
    InsertTableRows {
        page: usize,
        shape: ShapeId,
        before: usize,
        count: usize,
    },
    /// Layout > Insert Left / Right: as [`Command::InsertTableRows`] for columns.
    InsertTableColumns {
        page: usize,
        shape: ShapeId,
        before: usize,
        count: usize,
    },
    /// Layout > Delete > Delete Rows: rows `first..first + count` (0-based) with their
    /// text. Refused for the last remaining row. (Delete Table is `DeleteShapes`.)
    DeleteTableRows {
        page: usize,
        shape: ShapeId,
        first: usize,
        count: usize,
    },
    /// Layout > Delete > Delete Columns.
    DeleteTableColumns {
        page: usize,
        shape: ShapeId,
        first: usize,
        count: usize,
    },
    SetText {
        page: usize,
        id: ShapeId,
        paragraphs: Vec<Paragraph>,
    },
    /// Replaces the paragraphs of one table cell (row-major index).
    SetCellText {
        page: usize,
        id: ShapeId,
        cell: usize,
        paragraphs: Vec<Paragraph>,
    },
    /// Replaces the whole text of a linked story (editing across a frame boundary: Backspace
    /// at the start of a frame, Delete at the end). The story is laid out again afterwards.
    SetStoryText {
        story: StoryId,
        paragraphs: Vec<Paragraph>,
    },
    SetTextFormat {
        page: usize,
        ids: Vec<ShapeId>,
        format: TextFormat,
    },
    /// Character formatting of the characters `range` (offsets in the editable text: the
    /// paragraphs joined by one `\n`) of a text box, or of table cell `cell`. Runs are
    /// split at the range edges and identical neighbours merged.
    FormatRuns {
        page: usize,
        id: ShapeId,
        cell: Option<usize>,
        range: std::ops::Range<usize>,
        change: CharChange,
    },
    /// Alignment of the paragraphs that `range` touches (a caret touches its paragraph).
    FormatParagraphs {
        page: usize,
        id: ShapeId,
        cell: Option<usize>,
        range: std::ops::Range<usize>,
        align: Align,
    },
    /// Spacing (before, after, line) of the paragraphs that `range` touches.
    SpacingParagraphs {
        page: usize,
        id: ShapeId,
        cell: Option<usize>,
        range: std::ops::Range<usize>,
        change: crate::text::SpacingChange,
    },
    /// Bullets, numbering or indents of the paragraphs that `range` touches (spec 4R).
    ListParagraphs {
        page: usize,
        id: ShapeId,
        cell: Option<usize>,
        range: std::ops::Range<usize>,
        change: crate::text::ListChange,
    },
    /// Fill, outline, rotation or flip of several shapes at once.
    StyleShapes {
        page: usize,
        ids: Vec<ShapeId>,
        change: StyleChange,
    },
    BringForward {
        page: usize,
        id: ShapeId,
    },
    SendBackward {
        page: usize,
        id: ShapeId,
    },
    BringToFront {
        page: usize,
        id: ShapeId,
    },
    SendToBack {
        page: usize,
        id: ShapeId,
    },
    /// Z-order change of several shapes at once, keeping their relative order.
    ZOrderShapes {
        page: usize,
        ids: Vec<ShapeId>,
        op: ZOrder,
    },
    /// Groups two or more top-level objects (Publisher: Arrange > Group). Returns the
    /// new group's id.
    Group {
        page: usize,
        ids: Vec<ShapeId>,
    },
    /// Dissolves a group; its members stay where they are.
    Ungroup {
        page: usize,
        id: ShapeId,
    },
    /// Inserts a blank page (same size as `after`) after page index `after`.
    InsertPage {
        after: usize,
    },
    DeletePage {
        page: usize,
    },
    /// Duplicates page `page` with all its shapes (new ids); the copy becomes page
    /// `page + 1` (Pages pane > Insert Duplicate Page).
    DuplicatePage {
        page: usize,
    },
    /// Moves page `from` so that it becomes page `to` (0-based, final position).
    MovePage {
        from: usize,
        to: usize,
    },
    /// Page Design > Size and Orientation: every page gets this size. Shapes keep their
    /// position relative to the page centre (spec 4Q.4), so their position from the
    /// top-left corner moves by half the size change. Shapes that no longer touch the page
    /// stay where they are, off the page (the editor has no scratch area of its own).
    SetPageSize {
        width_emu: i64,
        height_emu: i64,
    },
    /// Page Design > Margins: the margin guides of every page.
    SetMargins {
        margins: Margins,
    },
    /// The Page Setup dialog's OK: page size and margins as one undo step. The size change
    /// works as [`Command::SetPageSize`]; the margins must fit the new size.
    SetPageSetup {
        width_emu: i64,
        height_emu: i64,
        margins: Margins,
    },
    /// Master Page view > Add Master Page (`copy_of` = `None`) or Duplicate (`copy_of` = the
    /// master to copy, shapes included). The new master goes last and gets the next free id
    /// letter unless `id` is given; its name is `Master Page <id>` (a copy: `Copy of <name>`).
    /// A document without masters first gets master A. The new master's index is the last.
    AddMaster {
        copy_of: Option<usize>,
        id: Option<char>,
        description: String,
        two_page: bool,
    },
    /// Rename Master Page: the id letter, name and description (at most 40 characters).
    SetMasterInfo {
        master: usize,
        id: char,
        name: String,
        description: String,
    },
    /// Delete a master. The last one is refused; pages that used it use the first remaining
    /// master (spec 4T, research 2026-11-28).
    DeleteMaster {
        master: usize,
    },
    /// Two-Page Master on or off. Turning it off drops the left half.
    SetTwoPageMaster {
        master: usize,
        two_page: bool,
    },
    /// Apply Master Page: the pages (0-based) use `master`; `None` makes them ignore their
    /// master (Page Design > Master Pages > None).
    ApplyMaster {
        pages: Vec<usize>,
        master: Option<usize>,
    },
    /// Make sure the document has a master: a document made in the editor has none until
    /// the Master Page view is first opened. Gives master A to every page without one.
    EnsureMaster,
}

impl Command {
    /// The page a shape command works on; `None` for the commands that do not have one.
    fn target_page(&self) -> Option<usize> {
        match self {
            Command::AlignShapes { page, .. }
            | Command::MoveShapes { page, .. }
            | Command::SetRect { page, .. }
            | Command::AddShape { page, .. }
            | Command::PasteShapes { page, .. }
            | Command::DeleteShapes { page, .. }
            | Command::ChangePicture { page, .. }
            | Command::InsertTable { page, .. }
            | Command::InsertTableRows { page, .. }
            | Command::InsertTableColumns { page, .. }
            | Command::DeleteTableRows { page, .. }
            | Command::DeleteTableColumns { page, .. }
            | Command::SetText { page, .. }
            | Command::SetCellText { page, .. }
            | Command::SetTextFormat { page, .. }
            | Command::FormatRuns { page, .. }
            | Command::FormatParagraphs { page, .. }
            | Command::SpacingParagraphs { page, .. }
            | Command::ListParagraphs { page, .. }
            | Command::StyleShapes { page, .. }
            | Command::BringForward { page, .. }
            | Command::SendBackward { page, .. }
            | Command::BringToFront { page, .. }
            | Command::SendToBack { page, .. }
            | Command::ZOrderShapes { page, .. }
            | Command::Group { page, .. }
            | Command::Ungroup { page, .. } => Some(*page),
            _ => None,
        }
    }

    /// The same command on another page (only for commands that have a [`target_page`]).
    fn retarget(&mut self, to: usize) {
        match self {
            Command::AlignShapes { page, .. }
            | Command::MoveShapes { page, .. }
            | Command::SetRect { page, .. }
            | Command::AddShape { page, .. }
            | Command::PasteShapes { page, .. }
            | Command::DeleteShapes { page, .. }
            | Command::ChangePicture { page, .. }
            | Command::InsertTable { page, .. }
            | Command::InsertTableRows { page, .. }
            | Command::InsertTableColumns { page, .. }
            | Command::DeleteTableRows { page, .. }
            | Command::DeleteTableColumns { page, .. }
            | Command::SetText { page, .. }
            | Command::SetCellText { page, .. }
            | Command::SetTextFormat { page, .. }
            | Command::FormatRuns { page, .. }
            | Command::FormatParagraphs { page, .. }
            | Command::SpacingParagraphs { page, .. }
            | Command::ListParagraphs { page, .. }
            | Command::StyleShapes { page, .. }
            | Command::BringForward { page, .. }
            | Command::SendBackward { page, .. }
            | Command::BringToFront { page, .. }
            | Command::SendToBack { page, .. }
            | Command::ZOrderShapes { page, .. }
            | Command::Group { page, .. }
            | Command::Ungroup { page, .. } => *page = to,
            _ => {}
        }
    }

    /// Short name for the Undo/Redo tooltips.
    pub fn label(&self) -> &'static str {
        match self {
            Command::MoveShapes { .. } => "Move",
            Command::AlignShapes { mode, .. } => match mode {
                AlignMode::DistributeHorizontally | AlignMode::DistributeVertically => "Distribute",
                _ => "Align",
            },
            Command::SetRect { .. } => "Resize",
            Command::AddShape { .. } => "Add Object",
            Command::PasteShapes { .. } => "Paste",
            Command::DeleteShapes { .. } => "Delete",
            Command::ChangePicture { .. } => "Change Picture",
            Command::InsertTable { .. } => "Insert Table",
            Command::InsertTableRows { .. } => "Insert Rows",
            Command::InsertTableColumns { .. } => "Insert Columns",
            Command::DeleteTableRows { .. } => "Delete Rows",
            Command::DeleteTableColumns { .. } => "Delete Columns",
            Command::SetText { .. } | Command::SetStoryText { .. } => "Edit Text",
            Command::SetCellText { .. } => "Edit Cell Text",
            Command::SetTextFormat { .. } | Command::FormatRuns { .. } => "Format Text",
            Command::FormatParagraphs { .. } => "Format Paragraph",
            Command::SpacingParagraphs { .. } => "Paragraph Spacing",
            Command::ListParagraphs { change, .. } => match change {
                crate::text::ListChange::Indents { .. } | crate::text::ListChange::StepIndent(_) => {
                    "Paragraph Indents"
                },
                _ => "Bullets and Numbering",
            },
            Command::StyleShapes { change, .. } => match change {
                StyleChange::Fill(_) => "Shape Fill",
                StyleChange::Outline(_) => "Shape Outline",
                StyleChange::OutlineWeight(_) => "Outline Weight",
                StyleChange::Rotate(_) => "Rotate",
                StyleChange::FlipHorizontal | StyleChange::FlipVertical => "Flip",
                StyleChange::Wrap(_) => "Wrap Text",
                StyleChange::Columns(..) => "Columns",
            },
            Command::BringForward { .. } => "Bring Forward",
            Command::SendBackward { .. } => "Send Backward",
            Command::BringToFront { .. } => "Bring to Front",
            Command::SendToBack { .. } => "Send to Back",
            Command::ZOrderShapes { op, .. } => match op {
                ZOrder::Forward => "Bring Forward",
                ZOrder::Backward => "Send Backward",
                ZOrder::Front => "Bring to Front",
                ZOrder::Back => "Send to Back",
            },
            Command::Group { .. } => "Group",
            Command::Ungroup { .. } => "Ungroup",
            Command::InsertPage { .. } => "Insert Page",
            Command::DeletePage { .. } => "Delete Page",
            Command::DuplicatePage { .. } => "Duplicate Page",
            Command::MovePage { .. } => "Move Page",
            Command::SetPageSize { .. } => "Page Size",
            Command::SetMargins { .. } => "Margins",
            Command::SetPageSetup { .. } => "Page Setup",
            Command::AddMaster { copy_of: None, .. } => "Add Master Page",
            Command::AddMaster { .. } => "Duplicate Master Page",
            Command::SetMasterInfo { .. } => "Rename Master Page",
            Command::DeleteMaster { .. } => "Delete Master Page",
            Command::SetTwoPageMaster { .. } => "Two-Page Master",
            Command::ApplyMaster { .. } => "Apply Master Page",
            Command::EnsureMaster => "Master Page",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    NoSuchPage(usize),
    NoSuchShape(ShapeId),
    NotText(ShapeId),
    NotTable(ShapeId),
    /// A table needs 1 x 1 to 65,535 cells.
    BadTableSize,
    /// The columns or rows would be smaller than Publisher allows.
    TableTooSmall,
    /// Row/column commands do not support tables with merged cells.
    MergedTable(ShapeId),
    /// The row/column numbers (or the count) are outside the table.
    BadTrackRange(Axis),
    /// The last remaining row or column of a table cannot be deleted.
    LastTrack(Axis),
    NotPicture(ShapeId),
    NoSuchCell(ShapeId, usize),
    NoSuchStory(StoryId),
    CannotDeleteLastPage,
    /// Group needs two or more objects that are not already inside a group.
    NotGroupable,
    NotAGroup(ShapeId),
    /// A page size outside 1 in .. 49 in (what the file writer accepts).
    BadPageSize,
    /// Margins that are negative or leave no room on the page.
    BadMargins,
    NoSuchMaster(usize),
    /// "You cannot delete the default master page." (Publisher's message.)
    CannotDeleteLastMaster,
    /// The id letter is already used by another master.
    MasterIdInUse(char),
    /// An id must be one visible character.
    BadMasterId(char),
    /// Every id letter is in use.
    NoMasterIdLeft,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::NoSuchPage(p) => write!(f, "there is no page {}", p + 1),
            CommandError::NoSuchShape(id) => write!(f, "there is no object {id}"),
            CommandError::NotText(id) => write!(f, "object {id} does not contain text"),
            CommandError::NotPicture(id) => write!(f, "object {id} is not a picture"),
            CommandError::NotTable(id) => write!(f, "object {id} is not a table"),
            CommandError::BadTableSize => {
                write!(f, "a table needs at least 1 row, 1 column and at most 65,535 cells")
            }
            CommandError::TableTooSmall => {
                write!(f, "the table is too small for that many rows and columns")
            }
            CommandError::MergedTable(_) => {
                write!(f, "rows and columns of a table with merged cells cannot be changed yet")
            }
            CommandError::BadTrackRange(a) => write!(f, "that {} does not exist", a.name()),
            CommandError::LastTrack(a) => {
                write!(f, "the last {} of a table cannot be deleted (delete the table instead)", a.name())
            }
            CommandError::NoSuchCell(id, c) => write!(f, "table {id} has no cell {c}"),
            CommandError::NoSuchStory(id) => write!(f, "there is no story {id}"),
            CommandError::NotGroupable => write!(
                f,
                "select two or more objects that are not already in a group"
            ),
            CommandError::NotAGroup(id) => write!(f, "object {id} is not a group"),
            CommandError::BadPageSize => write!(f, "a page must be between 1 and 49 inches"),
            CommandError::BadMargins => write!(f, "the margins do not fit on the page"),
            CommandError::NoSuchMaster(m) => write!(f, "there is no master page {}", m + 1),
            CommandError::CannotDeleteLastMaster => {
                write!(f, "you cannot delete the default master page")
            }
            CommandError::MasterIdInUse(c) => {
                write!(f, "another master page already has the ID {c}")
            }
            CommandError::BadMasterId(c) => {
                write!(f, "{c:?} cannot be a master page ID (use one letter or digit)")
            }
            CommandError::NoMasterIdLeft => write!(f, "there is no free master page ID left"),
            CommandError::CannotDeleteLastPage => {
                write!(f, "a publication needs at least one page")
            }
        }
    }
}

impl std::error::Error for CommandError {}

/// One reversible low-level change.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
enum Patch {
    /// Replace the shape with id `from.id` by `to`.
    Replace {
        page: usize,
        from: Shape,
        to: Shape,
    },
    Insert {
        page: usize,
        index: usize,
        shape: Shape,
    },
    Remove {
        page: usize,
        index: usize,
        shape: Shape,
    },
    /// Z-order change: take the shape at `from` and put it at `to`.
    Reorder {
        page: usize,
        from: usize,
        to: usize,
    },
    InsertPage {
        index: usize,
        page: Page,
    },
    RemovePage {
        index: usize,
        page: Page,
    },
    /// Take page `from` and put it at `to`.
    MovePage {
        from: usize,
        to: usize,
    },
    /// Page size and margin changes: the whole page list before and after.
    SetPages {
        from: Vec<Page>,
        to: Vec<Page>,
    },
    /// A linked story after a re-flow: its whole text and frame ends (the story with id
    /// `from.id`), before and after.
    SetStory {
        from: Story,
        to: Story,
    },
    /// A story that did not exist (a duplicated page's new chain) / that no longer does (its
    /// last frame was deleted), at `index` in `Document::stories`.
    InsertStory {
        index: usize,
        story: Story,
    },
    RemoveStory {
        index: usize,
        story: Story,
    },
    /// The page-number sections (spec 4T.4) before and after a page was inserted or
    /// removed.
    SetSections {
        from: Vec<Section>,
        to: Vec<Section>,
    },
    /// The whole master list before and after (add, duplicate, rename, delete, two-page).
    SetMasters {
        from: Vec<MasterPage>,
        to: Vec<MasterPage>,
    },
    /// Which master each page uses and whether it ignores it, before and after.
    SetPageMasters {
        from: Vec<(Option<usize>, bool)>,
        to: Vec<(Option<usize>, bool)>,
    },
}

impl Patch {
    /// The same shape patch on another page (pseudo page index of a master).
    fn retarget(&mut self, to: usize) {
        match self {
            Patch::Replace { page, .. }
            | Patch::Insert { page, .. }
            | Patch::Remove { page, .. }
            | Patch::Reorder { page, .. } => *page = to,
            _ => {}
        }
    }

    fn inverse(&self) -> Patch {
        match self.clone() {
            Patch::Replace { page, from, to } => Patch::Replace {
                page,
                from: to,
                to: from,
            },
            Patch::Insert { page, index, shape } => Patch::Remove { page, index, shape },
            Patch::Remove { page, index, shape } => Patch::Insert { page, index, shape },
            Patch::Reorder { page, from, to } => Patch::Reorder {
                page,
                from: to,
                to: from,
            },
            Patch::InsertPage { index, page } => Patch::RemovePage { index, page },
            Patch::RemovePage { index, page } => Patch::InsertPage { index, page },
            Patch::MovePage { from, to } => Patch::MovePage { from: to, to: from },
            Patch::SetPages { from, to } => Patch::SetPages { from: to, to: from },
            Patch::SetStory { from, to } => Patch::SetStory { from: to, to: from },
            Patch::InsertStory { index, story } => Patch::RemoveStory { index, story },
            Patch::RemoveStory { index, story } => Patch::InsertStory { index, story },
            Patch::SetSections { from, to } => Patch::SetSections { from: to, to: from },
            Patch::SetMasters { from, to } => Patch::SetMasters { from: to, to: from },
            Patch::SetPageMasters { from, to } => Patch::SetPageMasters { from: to, to: from },
        }
    }

    fn apply(&self, doc: &mut Document) {
        match self {
            Patch::Replace { page, from, to } => {
                if let Some(s) = doc
                    .shapes_mut(*page)
                    .and_then(|v| v.iter_mut().find(|s| s.id == from.id))
                {
                    *s = to.clone();
                }
            }
            Patch::Insert { page, index, shape } => {
                if let Some(v) = doc.shapes_mut(*page) {
                    v.insert(*index, shape.clone());
                }
            }
            Patch::Remove { page, index, .. } => {
                if let Some(v) = doc.shapes_mut(*page) {
                    v.remove(*index);
                }
            }
            Patch::Reorder { page, from, to } => {
                if let Some(v) = doc.shapes_mut(*page) {
                    let s = v.remove(*from);
                    v.insert(*to, s);
                }
            }
            Patch::SetMasters { to, .. } => doc.masters = to.clone(),
            Patch::SetPageMasters { to, .. } => {
                for (p, (m, ignore)) in doc.pages.iter_mut().zip(to) {
                    (p.master, p.ignore_master) = (*m, *ignore);
                }
            }
            Patch::InsertPage { index, page } => doc.pages.insert(*index, page.clone()),
            Patch::RemovePage { index, .. } => {
                doc.pages.remove(*index);
            }
            Patch::MovePage { from, to } => {
                let p = doc.pages.remove(*from);
                doc.pages.insert(*to, p);
            }
            Patch::SetPages { to, .. } => doc.pages = to.clone(),
            Patch::SetSections { to, .. } => doc.sections = to.clone(),
            Patch::SetStory { from, to } => {
                if let Some(s) = doc.stories.iter_mut().find(|s| s.id == from.id) {
                    *s = to.clone();
                }
            }
            Patch::InsertStory { index, story } => {
                doc.stories.insert((*index).min(doc.stories.len()), story.clone());
            }
            Patch::RemoveStory { index, .. } => {
                if *index < doc.stories.len() {
                    doc.stories.remove(*index);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Edit {
    label: &'static str,
    patches: Vec<Patch>,
}

/// A document plus its undo and redo history.
#[derive(Debug)]
pub struct Editor {
    doc: Document,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    revision: u64,
    /// The top undo step is still being extended by [`Editor::execute_live`].
    live: bool,
}

/// The paragraphs of a text box (`cell` = `None`) or of one table cell.
fn paragraphs_mut(
    kind: &mut ShapeKind,
    id: ShapeId,
    cell: Option<usize>,
) -> Result<&mut Vec<Paragraph>, CommandError> {
    match (kind, cell) {
        (ShapeKind::TextBox { paragraphs }, None) => Ok(paragraphs),
        (ShapeKind::Table { cells, .. }, Some(c)) => cells
            .get_mut(c)
            .map(|c| &mut c.paragraphs)
            .ok_or(CommandError::NoSuchCell(id, c)),
        (ShapeKind::Table { .. }, None) => Err(CommandError::NotText(id)),
        (_, Some(_)) => Err(CommandError::NotTable(id)),
        _ => Err(CommandError::NotText(id)),
    }
}

/// Fold a step's patches so it holds at most one `Replace` per shape and one `SetStory`
/// per story (earliest `from`, latest `to`); patches that end up changing nothing go away.
/// Steps with any other kind of patch are left as they are.
fn compact_patches(patches: &mut Vec<Patch>) {
    if !patches
        .iter()
        .all(|p| matches!(p, Patch::Replace { .. } | Patch::SetStory { .. }))
    {
        return;
    }
    let mut out: Vec<Patch> = Vec::new();
    for p in patches.drain(..) {
        let merged = out.iter_mut().find_map(|q| match (q, &p) {
            (
                Patch::Replace { page: a, from, to },
                Patch::Replace { page: b, to: t2, from: f2 },
            ) if a == b && from.id == f2.id => {
                *to = t2.clone();
                Some(())
            }
            (Patch::SetStory { from, to }, Patch::SetStory { to: t2, .. })
                if from.id == t2.id =>
            {
                *to = t2.clone();
                Some(())
            }
            _ => None,
        });
        if merged.is_none() {
            out.push(p);
        }
    }
    out.retain(|p| match p {
        Patch::Replace { from, to, .. } => from != to,
        Patch::SetStory { from, to } => from != to,
        _ => true,
    });
    *patches = out;
}

fn apply_format(paragraphs: &mut [Paragraph], format: &TextFormat) {
    for p in paragraphs {
        if let TextFormat::Align(a) = format {
            p.align = *a;
        }
        for r in &mut p.runs {
            match format {
                TextFormat::FontName(n) => r.font = n.clone(),
                TextFormat::Size(s) => r.size_pt = *s,
                TextFormat::Bold(b) => r.bold = *b,
                TextFormat::Italic(i) => r.italic = *i,
                TextFormat::Underline(u) => r.underline = *u,
                TextFormat::Script(s) => r.script = *s,
                TextFormat::Color(c) => r.color = *c,
                TextFormat::Align(_) => {}
            }
        }
    }
}

impl Editor {
    pub fn new(doc: Document) -> Editor {
        Editor {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            live: false,
        }
    }

    /// Rename the document (after Save As). Not an undoable edit.
    pub fn set_name(&mut self, name: &str) {
        self.doc.name = name.to_owned();
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// Increases with every change (including undo and redo); use it to invalidate caches.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|e| e.label)
    }
    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|e| e.label)
    }

    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.undo.pop() else {
            return false;
        };
        for p in edit.patches.iter().rev() {
            p.inverse().apply(&mut self.doc);
        }
        self.redo.push(edit);
        self.live = false;
        self.revision += 1;
        debug_assert!(self.doc.story_caches_consistent());
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(edit) = self.redo.pop() else {
            return false;
        };
        for p in &edit.patches {
            p.apply(&mut self.doc);
        }
        self.undo.push(edit);
        self.live = false;
        self.revision += 1;
        debug_assert!(self.doc.story_caches_consistent());
        true
    }

    /// Add a `SetSections` patch when `f` changes the document's page-number sections.
    fn shift_sections(
        &self,
        patches: &mut Vec<Patch>,
        f: impl Fn(&[Section]) -> Vec<Section>,
    ) {
        let to = f(&self.doc.sections);
        if to != self.doc.sections {
            patches.push(Patch::SetSections {
                from: self.doc.sections.clone(),
                to,
            });
        }
    }

    /// The master each page uses and whether it ignores it.
    fn page_masters(&self) -> Vec<(Option<usize>, bool)> {
        self.doc.pages.iter().map(|p| (p.master, p.ignore_master)).collect()
    }

    /// Add a `SetPageMasters` patch when `to` differs from what the pages have.
    fn push_page_masters(&self, patches: &mut Vec<Patch>, to: Vec<(Option<usize>, bool)>) {
        let from = self.page_masters();
        if from != to {
            patches.push(Patch::SetPageMasters { from, to });
        }
    }

    /// A document with no master gets master A (margins of the first page), used by every
    /// page that had none.
    fn ensure_master(
        &self,
        masters: &mut Vec<MasterPage>,
        pages: &mut [(Option<usize>, bool)],
    ) {
        if masters.is_empty() {
            let margins = self
                .doc
                .pages
                .first()
                .map_or(Margins::all(DEFAULT_MARGIN_EMU), |p| p.margins_emu);
            masters.push(MasterPage::new('A', margins));
            for p in pages.iter_mut().filter(|p| p.0.is_none()) {
                p.0 = Some(0);
            }
        }
    }

    /// The masters after the page size changed to `width` x `height` (their shapes keep their
    /// place from the page centre, like those of pages) and, when given, new margins.
    fn masters_resized(
        &self,
        width: i64,
        height: i64,
        margins: Option<Margins>,
    ) -> Vec<MasterPage> {
        let (ow, oh) = self.doc.pages.first().map_or((width, height), |p| (p.width_emu, p.height_emu));
        let (dx, dy) = (width / 2 - ow / 2, height / 2 - oh / 2);
        let mut out = self.doc.masters.clone();
        for m in &mut out {
            if let Some(mg) = margins {
                m.margins_emu = mg;
            }
            for s in m.shapes.iter_mut().chain(m.left_shapes.iter_mut()) {
                s.rect = s.rect.translated(dx, dy);
            }
        }
        out
    }

    fn push_masters(&self, patches: &mut Vec<Patch>, to: Vec<MasterPage>) {
        if to != self.doc.masters {
            patches.push(Patch::SetMasters {
                from: self.doc.masters.clone(),
                to,
            });
        }
    }

    fn check_page(&self, page: usize) -> Result<(), CommandError> {
        if page < self.doc.pages.len() {
            Ok(())
        } else {
            Err(CommandError::NoSuchPage(page))
        }
    }

    fn index_of(&self, page: usize, id: ShapeId) -> Result<usize, CommandError> {
        self.check_page(page)?;
        self.doc.pages[page]
            .shapes
            .iter()
            .position(|s| s.id == id)
            .ok_or(CommandError::NoSuchShape(id))
    }

    /// Replace one shape by a modified copy, if the copy differs.
    fn modify(
        &self,
        page: usize,
        id: ShapeId,
        f: impl FnOnce(&mut Shape) -> Result<(), CommandError>,
    ) -> Result<Option<Patch>, CommandError> {
        let ix = self.index_of(page, id)?;
        let from = self.doc.pages[page].shapes[ix].clone();
        let mut to = from.clone();
        f(&mut to)?;
        Ok((from != to).then_some(Patch::Replace { page, from, to }))
    }

    /// Run a command. Returns the ids of any shapes it created (add/paste). A command
    /// that changes nothing is not recorded in the history.
    pub fn execute(&mut self, cmd: Command) -> Result<Vec<ShapeId>, CommandError> {
        self.live = false;
        self.run(cmd)
    }

    /// Like [`Editor::execute`], but a run of these calls is ONE undo step: the changes
    /// are folded into the step the first call created, until [`Editor::end_live`], an
    /// ordinary `execute`, undo or redo. The editor uses it to show text as it is typed
    /// into a linked frame (the story re-flows into the next frames at once) without a
    /// history entry per keystroke. If the changes cancel out, the step disappears.
    pub fn execute_live(&mut self, cmd: Command) -> Result<Vec<ShapeId>, CommandError> {
        let was_live = self.live && !self.undo.is_empty();
        let depth = self.undo.len();
        let created = self.run(cmd)?;
        if self.undo.len() > depth {
            if was_live {
                let new = self.undo.pop().expect("just pushed");
                let top = self.undo.last_mut().expect("live step");
                top.patches.extend(new.patches);
                compact_patches(&mut top.patches);
                if top.patches.is_empty() {
                    self.undo.pop();
                    self.live = false;
                    return Ok(created);
                }
            }
            self.live = true;
        } else {
            self.live = was_live;
        }
        Ok(created)
    }

    /// The next [`Editor::execute_live`] starts a new undo step.
    pub fn end_live(&mut self) {
        self.live = false;
    }

    fn run(&mut self, mut cmd: Command) -> Result<Vec<ShapeId>, CommandError> {
        let Some(surface) = cmd.target_page().filter(|p| master_surface_parts(*p).is_some()) else {
            return self.run_inner(cmd);
        };
        // A shape command on a master: it runs on a stand-in page that holds the master half's
        // shapes (so every shape command works unchanged), and the patches it made are then
        // pointed at the master and applied to the real document.
        let scratch = self.doc.surface(surface).ok_or(CommandError::NoSuchPage(surface))?.into_owned();
        let real = std::mem::replace(&mut self.doc.pages, vec![scratch]);
        cmd.retarget(0);
        let depth = self.undo.len();
        let result = self.run_inner(cmd);
        self.doc.pages = real;
        if result.is_ok() && self.undo.len() > depth {
            let mut edit = self.undo.pop().expect("just pushed");
            for p in &mut edit.patches {
                p.retarget(surface);
                p.apply(&mut self.doc);
            }
            self.undo.push(edit);
        }
        result
    }

    fn run_inner(&mut self, cmd: Command) -> Result<Vec<ShapeId>, CommandError> {
        let label = cmd.label();
        let mut created = Vec::new();
        let mut patches: Vec<Patch> = Vec::new();
        // Chains a command makes (a duplicated page's linked frames): laid out and added
        // once the command's patches are applied.
        let mut new_stories: Vec<Story> = Vec::new();
        match cmd {
            Command::AlignShapes {
                page,
                ids,
                mode,
                to_margins,
            } => {
                self.check_page(page)?;
                let mut items: Vec<(ShapeId, Rect)> = Vec::new();
                for id in ids {
                    let ix = self.index_of(page, id)?;
                    let b = crate::geometry::visual_bounds(&self.doc.pages[page].shapes[ix]);
                    items.push((id, b));
                }
                let pg = &self.doc.pages[page];
                let area = if to_margins {
                    Rect::new(
                        pg.margins_emu.left,
                        pg.margins_emu.top,
                        pg.width_emu - pg.margins_emu.left - pg.margins_emu.right,
                        pg.height_emu - pg.margins_emu.top - pg.margins_emu.bottom,
                    )
                } else {
                    let l = items.iter().map(|i| i.1.x).min().unwrap_or(0);
                    let t = items.iter().map(|i| i.1.y).min().unwrap_or(0);
                    let r = items.iter().map(|i| i.1.right()).max().unwrap_or(0);
                    let b = items.iter().map(|i| i.1.bottom()).max().unwrap_or(0);
                    Rect::new(l, t, r - l, b - t)
                };
                let mut moves: Vec<(ShapeId, i64, i64)> = Vec::new();
                match mode {
                    AlignMode::Left => {
                        moves.extend(items.iter().map(|(id, b)| (*id, area.x - b.x, 0)));
                    }
                    AlignMode::Center => moves.extend(
                        items
                            .iter()
                            .map(|(id, b)| (*id, area.x + (area.w - b.w) / 2 - b.x, 0)),
                    ),
                    AlignMode::Right => {
                        moves.extend(
                            items
                                .iter()
                                .map(|(id, b)| (*id, area.right() - b.right(), 0)),
                        );
                    }
                    AlignMode::Top => {
                        moves.extend(items.iter().map(|(id, b)| (*id, 0, area.y - b.y)));
                    }
                    AlignMode::Middle => moves.extend(
                        items
                            .iter()
                            .map(|(id, b)| (*id, 0, area.y + (area.h - b.h) / 2 - b.y)),
                    ),
                    AlignMode::Bottom => {
                        moves.extend(
                            items
                                .iter()
                                .map(|(id, b)| (*id, 0, area.bottom() - b.bottom())),
                        );
                    }
                    AlignMode::DistributeHorizontally | AlignMode::DistributeVertically => {
                        let horiz = mode == AlignMode::DistributeHorizontally;
                        let lo = |b: &Rect| if horiz { b.x } else { b.y };
                        let len = |b: &Rect| if horiz { b.w } else { b.h };
                        items.sort_by_key(|i| lo(&i.1));
                        let (start, span) = if horiz {
                            (area.x, area.w)
                        } else {
                            (area.y, area.h)
                        };
                        let total: i64 = items.iter().map(|i| len(&i.1)).sum();
                        let gaps = items.len() as i64 - 1;
                        if gaps > 0 {
                            let gap = (span - total) / gaps;
                            let mut pos = start;
                            for (id, b) in &items {
                                let d = pos - lo(b);
                                moves.push(if horiz { (*id, d, 0) } else { (*id, 0, d) });
                                pos += len(b) + gap;
                            }
                        }
                    }
                }
                for (id, dx, dy) in moves {
                    if dx != 0 || dy != 0 {
                        for m in self.doc.pages[page].with_descendants(&[id]) {
                            patches.extend(self.modify(page, m, |s| {
                                s.rect = s.rect.translated(dx, dy);
                                Ok(())
                            })?);
                        }
                    }
                }
            }
            Command::MoveShapes { page, ids, dx, dy } => {
                self.check_page(page)?;
                for id in self.doc.pages[page].with_descendants(&ids) {
                    patches.extend(self.modify(page, id, |s| {
                        s.rect = s.rect.translated(dx, dy);
                        Ok(())
                    })?);
                }
            }
            Command::SetRect { page, id, rect } => {
                let ix = self.index_of(page, id)?;
                let old = self.doc.pages[page].shapes[ix].rect;
                if matches!(self.doc.pages[page].shapes[ix].kind, ShapeKind::Group)
                    && old.w > 0
                    && old.h > 0
                {
                    // Resizing a group scales its members (and member groups) with it.
                    for d in self.doc.pages[page].descendants(id) {
                        patches.extend(self.modify(page, d, |s| {
                            s.rect = crate::geometry::scale_into(s.rect, old, rect);
                            Ok(())
                        })?);
                    }
                }
                patches.extend(self.modify(page, id, |s| {
                    s.rect = rect;
                    Ok(())
                })?);
            }
            Command::StyleShapes { page, ids, change } => {
                self.check_page(page)?;
                for id in ids {
                    patches.extend(self.modify(page, id, |s| {
                        let is_line = matches!(s.kind, ShapeKind::Line);
                        let is_pic = matches!(s.kind, ShapeKind::Picture { .. });
                        let is_group = matches!(s.kind, ShapeKind::Group);
                        let is_table = matches!(s.kind, ShapeKind::Table { .. });
                        match change {
                            StyleChange::Fill(c) => {
                                if !(is_line || is_pic || is_group || is_table) {
                                    s.fill = c;
                                }
                            }
                            StyleChange::Outline(c) => {
                                if !(is_pic || is_group || is_table) {
                                    s.line = match (c, s.line) {
                                        (None, _) => None,
                                        (Some(color), old) => Some(Stroke {
                                            color,
                                            width_emu: old.map_or(9_525, |o| o.width_emu),
                                        }),
                                    };
                                }
                            }
                            StyleChange::OutlineWeight(w) => {
                                if !(is_pic || is_group || is_table) && w > 0 {
                                    s.line = Some(Stroke {
                                        color: s.line.map_or(Rgb::BLACK, |o| o.color),
                                        width_emu: w,
                                    });
                                }
                            }
                            StyleChange::Rotate(d) => {
                                if !is_group {
                                    s.rotation_deg = (s.rotation_deg + d).rem_euclid(360.0);
                                }
                            }
                            StyleChange::Wrap(w) => {
                                if !is_group {
                                    s.wrap = w;
                                }
                            }
                            StyleChange::Columns(n, gap) => {
                                if s.is_text_box() && (1..=64).contains(&n) {
                                    s.columns = n;
                                    s.column_gap_emu = gap.max(0);
                                }
                            }
                            StyleChange::FlipHorizontal => {
                                if is_line {
                                    s.flip_h = !s.flip_h;
                                }
                            }
                            StyleChange::FlipVertical => {
                                if is_line {
                                    s.flip_v = !s.flip_v;
                                }
                            }
                        }
                        Ok(())
                    })?);
                }
            }
            Command::Group { page, ids } => {
                self.check_page(page)?;
                let mut ix: Vec<usize> = Vec::new();
                for &id in &ids {
                    let i = self.index_of(page, id)?;
                    let s = &self.doc.pages[page].shapes[i];
                    if s.group.is_some() || ix.contains(&i) {
                        return Err(CommandError::NotGroupable);
                    }
                    ix.push(i);
                }
                if ix.len() < 2 {
                    return Err(CommandError::NotGroupable);
                }
                let shapes = &self.doc.pages[page].shapes;
                let bounds: Vec<Rect> = ix
                    .iter()
                    .map(|&i| crate::geometry::visual_bounds(&shapes[i]))
                    .collect();
                let l = bounds.iter().map(|b| b.x).min().unwrap_or(0);
                let t = bounds.iter().map(|b| b.y).min().unwrap_or(0);
                let r = bounds.iter().map(|b| b.right()).max().unwrap_or(0);
                let b = bounds.iter().map(|b| b.bottom()).max().unwrap_or(0);
                let top = ix.iter().copied().max().unwrap_or(0);
                let mut group = Shape::new(Rect::new(l, t, r - l, b - t), ShapeKind::Group);
                let gid = self.doc.alloc_id();
                group.id = gid;
                // The group sits just above its topmost member.
                patches.push(Patch::Insert {
                    page,
                    index: top + 1,
                    shape: group,
                });
                for &id in &ids {
                    patches.extend(self.modify(page, id, |s| {
                        s.group = Some(gid);
                        Ok(())
                    })?);
                }
                created.push(gid);
            }
            Command::Ungroup { page, id } => {
                let ix = self.index_of(page, id)?;
                let g = self.doc.pages[page].shapes[ix].clone();
                if !matches!(g.kind, ShapeKind::Group) {
                    return Err(CommandError::NotAGroup(id));
                }
                let members: Vec<ShapeId> = self.doc.pages[page]
                    .shapes
                    .iter()
                    .filter(|s| s.group == Some(id))
                    .map(|s| s.id)
                    .collect();
                for m in members {
                    patches.extend(self.modify(page, m, |s| {
                        s.group = g.group;
                        Ok(())
                    })?);
                }
                patches.push(Patch::Remove {
                    page,
                    index: ix,
                    shape: g,
                });
            }
            Command::AddShape { page, mut shape } => {
                self.check_page(page)?;
                shape.group = None;
                patches.push(self.insert_new(page, shape, &mut created));
            }
            Command::PasteShapes { page, shapes } => {
                self.check_page(page)?;
                // Groups are not pasted: the copies become ordinary objects.
                for mut s in shapes
                    .into_iter()
                    .filter(|s| !matches!(s.kind, ShapeKind::Group))
                {
                    s.group = None;
                    patches.push(self.insert_new(page, s, &mut created));
                }
            }
            Command::DeleteShapes { page, ids } => {
                self.check_page(page)?;
                let ids = self.doc.pages[page].with_descendants(&ids);
                let mut found: Vec<(usize, Shape)> = Vec::new();
                for id in ids {
                    let ix = self.index_of(page, id)?;
                    if !found.iter().any(|(i, _)| *i == ix) {
                        found.push((ix, self.doc.pages[page].shapes[ix].clone()));
                    }
                }
                // Highest index first so the earlier removals do not shift later ones.
                found.sort_by_key(|(i, _)| std::cmp::Reverse(*i));
                patches.extend(found.into_iter().map(|(index, shape)| Patch::Remove {
                    page,
                    index,
                    shape,
                }));
            }
            Command::ChangePicture {
                page,
                id,
                data,
                format,
                file,
                rect,
            } => {
                patches.extend(self.modify(page, id, |s| match &mut s.kind {
                    ShapeKind::Picture {
                        data: d,
                        format: f,
                        file: n,
                    } => {
                        (*d, *f, *n) = (data, format, file);
                        s.rect = rect;
                        Ok(())
                    }
                    _ => Err(CommandError::NotPicture(s.id)),
                })?);
            }
            Command::InsertTable {
                page,
                rect,
                rows,
                columns,
            } => {
                self.check_page(page)?;
                let kind = crate::table::new_table(rect, rows, columns)?;
                patches.push(self.insert_new(page, Shape::new(rect, kind), &mut created));
            }
            Command::InsertTableRows {
                page,
                shape,
                before,
                count,
            } => {
                patches.extend(self.modify(page, shape, |s| {
                    crate::table::insert_tracks(s, Axis::Rows, before, count)
                })?);
            }
            Command::InsertTableColumns {
                page,
                shape,
                before,
                count,
            } => {
                patches.extend(self.modify(page, shape, |s| {
                    crate::table::insert_tracks(s, Axis::Columns, before, count)
                })?);
            }
            Command::DeleteTableRows {
                page,
                shape,
                first,
                count,
            } => {
                patches.extend(self.modify(page, shape, |s| {
                    crate::table::delete_tracks(s, Axis::Rows, first, count)
                })?);
            }
            Command::DeleteTableColumns {
                page,
                shape,
                first,
                count,
            } => {
                patches.extend(self.modify(page, shape, |s| {
                    crate::table::delete_tracks(s, Axis::Columns, first, count)
                })?);
            }
            Command::SetText {
                page,
                id,
                paragraphs,
            } => {
                patches.extend(self.modify(page, id, |s| match &mut s.kind {
                    ShapeKind::TextBox { paragraphs: p } => {
                        *p = paragraphs;
                        Ok(())
                    }
                    _ => Err(CommandError::NotText(s.id)),
                })?);
            }
            Command::SetStoryText { story, paragraphs } => {
                let from = self
                    .doc
                    .stories
                    .iter()
                    .find(|s| s.id == story)
                    .cloned()
                    .ok_or(CommandError::NoSuchStory(story))?;
                let mut to = from.clone();
                to.paragraphs = paragraphs;
                // The frames show their old shares of the new text until the re-flow below;
                // that keeps the story (not the stale frame text) the truth.
                let mut caches = Vec::new();
                for (i, &fid) in to.frames.iter().enumerate() {
                    let at = self.doc.pages.iter().enumerate().find_map(|(pg, p)| {
                        p.shapes.iter().position(|s| s.id == fid).map(|_| pg)
                    });
                    if let Some(pg) = at {
                        caches.push((pg, fid, to.cache(i)));
                    }
                }
                if to != from {
                    patches.push(Patch::SetStory { from, to });
                }
                for (pg, fid, paragraphs) in caches {
                    patches.extend(self.modify(pg, fid, |s| {
                        s.kind = ShapeKind::TextBox { paragraphs };
                        Ok(())
                    })?);
                }
            }
            Command::SetCellText {
                page,
                id,
                cell,
                paragraphs,
            } => {
                patches.extend(self.modify(page, id, |s| {
                    match &mut s.kind {
                    ShapeKind::Table { cells, .. } => {
                        let c = cells
                            .get_mut(cell)
                            .ok_or(CommandError::NoSuchCell(s.id, cell))?;
                        c.paragraphs = paragraphs;
                    }
                    _ => return Err(CommandError::NotTable(s.id)),
                }
                crate::table::fit_rows(s);
                Ok(())
                })?);
            }
            Command::SetTextFormat { page, ids, format } => {
                let mut last_err = None;
                let mut any_text = false;
                for id in ids {
                    match self.modify(page, id, |s| match &mut s.kind {
                        ShapeKind::TextBox { paragraphs } => {
                            apply_format(paragraphs, &format);
                            Ok(())
                        }
                        _ => Err(CommandError::NotText(s.id)),
                    }) {
                        Ok(p) => {
                            any_text = true;
                            patches.extend(p);
                        }
                        Err(e @ CommandError::NotText(_)) => last_err = Some(e),
                        Err(e) => return Err(e),
                    }
                }
                if !any_text {
                    if let Some(e) = last_err {
                        return Err(e);
                    }
                }
            }
            Command::FormatRuns {
                page,
                id,
                cell,
                range,
                change,
            } => {
                patches.extend(self.modify(page, id, |s| {
                    let id = s.id;
                    let paras = paragraphs_mut(&mut s.kind, id, cell)?;
                    crate::text::format_range(paras, range, &change);
                    crate::table::fit_rows(s);
                    Ok(())
                })?);
            }
            Command::FormatParagraphs {
                page,
                id,
                cell,
                range,
                align,
            } => {
                patches.extend(self.modify(page, id, |s| {
                    let id = s.id;
                    let paras = paragraphs_mut(&mut s.kind, id, cell)?;
                    crate::text::align_range(paras, range, align);
                    crate::table::fit_rows(s);
                    Ok(())
                })?);
            }
            Command::SpacingParagraphs {
                page,
                id,
                cell,
                range,
                change,
            } => {
                patches.extend(self.modify(page, id, |s| {
                    let id = s.id;
                    let paras = paragraphs_mut(&mut s.kind, id, cell)?;
                    crate::text::spacing_range(paras, range, change);
                    crate::table::fit_rows(s);
                    Ok(())
                })?);
            }
            Command::ListParagraphs {
                page,
                id,
                cell,
                range,
                change,
            } => {
                patches.extend(self.modify(page, id, |s| {
                    let id = s.id;
                    let paras = paragraphs_mut(&mut s.kind, id, cell)?;
                    crate::text::list_range(paras, range, &change);
                    crate::table::fit_rows(s);
                    Ok(())
                })?);
            }
            Command::BringForward { page, id } => {
                let ix = self.index_of(page, id)?;
                if ix + 1 < self.doc.pages[page].shapes.len() {
                    patches.push(Patch::Reorder {
                        page,
                        from: ix,
                        to: ix + 1,
                    });
                }
            }
            Command::SendBackward { page, id } => {
                let ix = self.index_of(page, id)?;
                if ix > 0 {
                    patches.push(Patch::Reorder {
                        page,
                        from: ix,
                        to: ix - 1,
                    });
                }
            }
            Command::BringToFront { page, id } => {
                let ix = self.index_of(page, id)?;
                let last = self.doc.pages[page].shapes.len() - 1;
                if ix < last {
                    patches.push(Patch::Reorder {
                        page,
                        from: ix,
                        to: last,
                    });
                }
            }
            Command::SendToBack { page, id } => {
                let ix = self.index_of(page, id)?;
                if ix > 0 {
                    patches.push(Patch::Reorder {
                        page,
                        from: ix,
                        to: 0,
                    });
                }
            }
            Command::ZOrderShapes { page, ids, op } => {
                self.check_page(page)?;
                let ids = self.doc.pages[page].with_descendants(&ids);
                for &id in &ids {
                    self.index_of(page, id)?;
                }
                let mut order: Vec<ShapeId> =
                    self.doc.pages[page].shapes.iter().map(|s| s.id).collect();
                let picked: Vec<usize> = (0..order.len())
                    .filter(|&i| ids.contains(&order[i]))
                    .collect();
                let mut moves: Vec<(usize, usize)> = Vec::new();
                let last = order.len().saturating_sub(1);
                let mut mv = |order: &mut Vec<ShapeId>, from: usize, to: usize| {
                    let v = order.remove(from);
                    order.insert(to, v);
                    moves.push((from, to));
                };
                match op {
                    ZOrder::Front => {
                        for ix in picked {
                            let id = self.doc.pages[page].shapes[ix].id;
                            if let Some(ix) = order.iter().position(|i| *i == id) {
                                if ix < last {
                                    mv(&mut order, ix, last);
                                }
                            }
                        }
                    }
                    ZOrder::Back => {
                        for ix in picked.into_iter().rev() {
                            let id = self.doc.pages[page].shapes[ix].id;
                            if let Some(ix) = order.iter().position(|i| *i == id) {
                                if ix > 0 {
                                    mv(&mut order, ix, 0);
                                }
                            }
                        }
                    }
                    ZOrder::Forward => {
                        for ix in picked.into_iter().rev() {
                            let id = self.doc.pages[page].shapes[ix].id;
                            if let Some(ix) = order.iter().position(|i| *i == id) {
                                if ix < last && !ids.contains(&order[ix + 1]) {
                                    mv(&mut order, ix, ix + 1);
                                }
                            }
                        }
                    }
                    ZOrder::Backward => {
                        for ix in picked {
                            let id = self.doc.pages[page].shapes[ix].id;
                            if let Some(ix) = order.iter().position(|i| *i == id) {
                                if ix > 0 && !ids.contains(&order[ix - 1]) {
                                    mv(&mut order, ix, ix - 1);
                                }
                            }
                        }
                    }
                }
                patches.extend(moves.into_iter().map(|(from, to)| Patch::Reorder {
                    page,
                    from,
                    to,
                }));
            }
            Command::InsertPage { after } => {
                self.check_page(after)?;
                let src = &self.doc.pages[after];
                // A page added later uses the first master (Publisher gives it master A, even
                // next to a page that uses another one).
                let (master, ignore_master) = if self.doc.masters.is_empty() {
                    (src.master, src.ignore_master)
                } else {
                    (Some(0), false)
                };
                patches.push(Patch::InsertPage {
                    index: after + 1,
                    page: Page {
                        id: new_page_id(),
                        shapes: Vec::new(),
                        duplicate_of: None,
                        master,
                        ignore_master,
                        ..src.clone()
                    },
                });
                self.shift_sections(&mut patches, |s| sections_after_insert(s, after + 1));
            }
            Command::DuplicatePage { page } => {
                self.check_page(page)?;
                let mut copy = self.doc.pages[page].clone();
                copy.id = new_page_id();
                copy.duplicate_of = Some(self.doc.pages[page].id);
                // New ids for every shape; group references follow.
                let map: Vec<(ShapeId, ShapeId)> = copy
                    .shapes
                    .iter()
                    .map(|s| (s.id, self.doc.alloc_id()))
                    .collect();
                for s in &mut copy.shapes {
                    s.id = map.iter().find(|m| m.0 == s.id).map_or(s.id, |m| m.1);
                    s.group = s
                        .group
                        .and_then(|g| map.iter().find(|m| m.0 == g).map(|m| m.1));
                }
                // Linked frames are copied without their text; the copies that sit on this
                // page are linked to each other, in chain order, as a new chain.
                for story in &self.doc.stories {
                    let copies: Vec<ShapeId> = story
                        .frames
                        .iter()
                        .filter_map(|f| map.iter().find(|m| m.0 == *f).map(|m| m.1))
                        .collect();
                    for s in copy.shapes.iter_mut().filter(|s| copies.contains(&s.id)) {
                        s.kind = ShapeKind::TextBox {
                            paragraphs: vec![crate::story::empty_like(&s.kind)],
                        };
                    }
                    if copies.len() > 1 {
                        let id = self
                            .doc
                            .stories
                            .iter()
                            .chain(&new_stories)
                            .map(|s| s.id)
                            .max()
                            .unwrap_or(0)
                            + 1;
                        new_stories.push(Story {
                            id,
                            frames: copies,
                            paragraphs: vec![Paragraph::plain("")],
                            ends: Vec::new(),
                        });
                    }
                }
                patches.push(Patch::InsertPage {
                    index: page + 1,
                    page: copy,
                });
                self.shift_sections(&mut patches, |s| sections_after_insert(s, page + 1));
            }
            Command::DeletePage { page } => {
                self.check_page(page)?;
                if self.doc.pages.len() == 1 {
                    return Err(CommandError::CannotDeleteLastPage);
                }
                patches.push(Patch::RemovePage {
                    index: page,
                    page: self.doc.pages[page].clone(),
                });
                let count = self.doc.pages.len();
                self.shift_sections(&mut patches, |s| sections_after_remove(s, page, count));
            }
            Command::MovePage { from, to } => {
                self.check_page(from)?;
                self.check_page(to)?;
                if from != to {
                    patches.push(Patch::MovePage { from, to });
                }
            }
            Command::SetPageSize {
                width_emu,
                height_emu,
            } => {
                let ok = EMU_PER_INCH..=49 * EMU_PER_INCH;
                if !ok.contains(&width_emu) || !ok.contains(&height_emu) {
                    return Err(CommandError::BadPageSize);
                }
                let masters = self.masters_resized(width_emu, height_emu, None);
                let mut to = self.doc.pages.clone();
                for p in &mut to {
                    // The centre is where `W / 2` lands in whole EMU, as a saved file reads it.
                    let dx = width_emu / 2 - p.width_emu / 2;
                    let dy = height_emu / 2 - p.height_emu / 2;
                    p.width_emu = width_emu;
                    p.height_emu = height_emu;
                    for s in &mut p.shapes {
                        s.rect = s.rect.translated(dx, dy);
                    }
                }
                if to != self.doc.pages {
                    patches.push(Patch::SetPages {
                        from: self.doc.pages.clone(),
                        to,
                    });
                }
                self.push_masters(&mut patches, masters);
            }
            Command::SetMargins { margins } => {
                let m = margins.to_array();
                if m.iter().any(|v| *v < 0)
                    || self.doc.pages.iter().any(|p| {
                        margins.left + margins.right >= p.width_emu
                            || margins.top + margins.bottom >= p.height_emu
                    })
                {
                    return Err(CommandError::BadMargins);
                }
                let mut to = self.doc.pages.clone();
                for p in &mut to {
                    p.margins_emu = margins;
                }
                if to != self.doc.pages {
                    patches.push(Patch::SetPages {
                        from: self.doc.pages.clone(),
                        to,
                    });
                }
                let mut masters = self.doc.masters.clone();
                for m in &mut masters {
                    m.margins_emu = margins;
                }
                self.push_masters(&mut patches, masters);
            }
            Command::SetPageSetup {
                width_emu,
                height_emu,
                margins,
            } => {
                let ok = EMU_PER_INCH..=49 * EMU_PER_INCH;
                if !ok.contains(&width_emu) || !ok.contains(&height_emu) {
                    return Err(CommandError::BadPageSize);
                }
                if margins.to_array().iter().any(|v| *v < 0)
                    || margins.left + margins.right >= width_emu
                    || margins.top + margins.bottom >= height_emu
                {
                    return Err(CommandError::BadMargins);
                }
                let masters = self.masters_resized(width_emu, height_emu, Some(margins));
                let mut to = self.doc.pages.clone();
                for p in &mut to {
                    let dx = width_emu / 2 - p.width_emu / 2;
                    let dy = height_emu / 2 - p.height_emu / 2;
                    p.width_emu = width_emu;
                    p.height_emu = height_emu;
                    p.margins_emu = margins;
                    for s in &mut p.shapes {
                        s.rect = s.rect.translated(dx, dy);
                    }
                }
                if to != self.doc.pages {
                    patches.push(Patch::SetPages {
                        from: self.doc.pages.clone(),
                        to,
                    });
                }
                self.push_masters(&mut patches, masters);
            }
            Command::AddMaster {
                copy_of,
                id,
                description,
                two_page,
            } => {
                let mut masters = self.doc.masters.clone();
                let mut pages = self.page_masters();
                if masters.is_empty() {
                    self.ensure_master(&mut masters, &mut pages);
                }
                if let Some(c) = copy_of {
                    if c >= masters.len() {
                        return Err(CommandError::NoSuchMaster(c));
                    }
                }
                let id = match id {
                    Some(c) if c.is_control() || c.is_whitespace() => {
                        return Err(CommandError::BadMasterId(c))
                    }
                    Some(c) if masters.iter().any(|m| m.id == c) => {
                        return Err(CommandError::MasterIdInUse(c))
                    }
                    Some(c) => c,
                    None => next_master_id(&masters).ok_or(CommandError::NoMasterIdLeft)?,
                };
                let margins = self
                    .doc
                    .pages
                    .first()
                    .map_or(Margins::all(DEFAULT_MARGIN_EMU), |p| p.margins_emu);
                let mut new = MasterPage::new(id, margins);
                if let Some(c) = copy_of {
                    let src = &masters[c];
                    new.name = format!("Copy of {}", src.name);
                    new.margins_emu = src.margins_emu;
                    let mut map: Vec<(ShapeId, ShapeId)> = Vec::new();
                    let mut copy_shapes = |shapes: &[Shape], doc: &mut Document| -> Vec<Shape> {
                        let start = map.len();
                        for s in shapes {
                            map.push((s.id, doc.alloc_id()));
                        }
                        let mut out: Vec<Shape> = shapes.to_vec();
                        for s in &mut out {
                            s.id = map[start..].iter().find(|m| m.0 == s.id).map_or(s.id, |m| m.1);
                            s.group = s
                                .group
                                .and_then(|g| map[start..].iter().find(|m| m.0 == g).map(|m| m.1));
                        }
                        out
                    };
                    new.shapes = copy_shapes(&src.shapes, &mut self.doc);
                    if two_page {
                        new.left_shapes = copy_shapes(&src.left_shapes, &mut self.doc);
                    }
                }
                new.two_page = two_page;
                new.description = description.trim().chars().take(MasterPage::MAX_DESCRIPTION).collect();
                masters.push(new);
                patches.push(Patch::SetMasters {
                    from: self.doc.masters.clone(),
                    to: masters,
                });
                self.push_page_masters(&mut patches, pages);
            }
            Command::SetMasterInfo {
                master,
                id,
                name,
                description,
            } => {
                let mut masters = self.doc.masters.clone();
                let m = masters.get_mut(master).ok_or(CommandError::NoSuchMaster(master))?;
                if id.is_control() || id.is_whitespace() {
                    return Err(CommandError::BadMasterId(id));
                }
                if self.doc.masters.iter().enumerate().any(|(i, x)| i != master && x.id == id) {
                    return Err(CommandError::MasterIdInUse(id));
                }
                m.id = id;
                let name = name.trim();
                m.name = if name.is_empty() { MasterPage::default_name(id) } else { name.to_owned() };
                m.description = description.trim().chars().take(MasterPage::MAX_DESCRIPTION).collect();
                if masters != self.doc.masters {
                    patches.push(Patch::SetMasters {
                        from: self.doc.masters.clone(),
                        to: masters,
                    });
                }
            }
            Command::DeleteMaster { master } => {
                if master >= self.doc.masters.len() {
                    return Err(CommandError::NoSuchMaster(master));
                }
                if self.doc.masters.len() == 1 {
                    return Err(CommandError::CannotDeleteLastMaster);
                }
                let mut masters = self.doc.masters.clone();
                masters.remove(master);
                let mut pages = self.page_masters();
                for (m, _) in &mut pages {
                    *m = match *m {
                        Some(x) if x == master => Some(0),
                        Some(x) if x > master => Some(x - 1),
                        other => other,
                    };
                }
                patches.push(Patch::SetMasters {
                    from: self.doc.masters.clone(),
                    to: masters,
                });
                self.push_page_masters(&mut patches, pages);
            }
            Command::SetTwoPageMaster { master, two_page } => {
                let mut masters = self.doc.masters.clone();
                let m = masters.get_mut(master).ok_or(CommandError::NoSuchMaster(master))?;
                m.two_page = two_page;
                if !two_page {
                    m.left_shapes.clear();
                }
                if masters != self.doc.masters {
                    patches.push(Patch::SetMasters {
                        from: self.doc.masters.clone(),
                        to: masters,
                    });
                }
            }
            Command::ApplyMaster { pages: targets, master } => {
                if let Some(m) = master {
                    if m >= self.doc.masters.len() {
                        return Err(CommandError::NoSuchMaster(m));
                    }
                }
                let mut pages = self.page_masters();
                for t in targets {
                    let slot = pages.get_mut(t).ok_or(CommandError::NoSuchPage(t))?;
                    *slot = match master {
                        Some(m) => (Some(m), false),
                        None => (slot.0, true),
                    };
                }
                self.push_page_masters(&mut patches, pages);
            }
            Command::EnsureMaster => {
                if self.doc.masters.is_empty() {
                    let mut masters = Vec::new();
                    let mut pages = self.page_masters();
                    self.ensure_master(&mut masters, &mut pages);
                    patches.push(Patch::SetMasters {
                        from: Vec::new(),
                        to: masters,
                    });
                    self.push_page_masters(&mut patches, pages);
                }
            }
        }
        if patches.is_empty() {
            return Ok(created);
        }
        for p in &patches {
            p.apply(&mut self.doc);
        }
        // Frames that are gone (deleted, or on a deleted page) leave their chain; new chains
        // join the document.
        let relink = self.relink_patches(new_stories);
        patches.extend(relink);
        // A change to a linked story's text or frames lays the whole story out again, as
        // part of the same undo step.
        let reflow = self.reflow_patches(&patches);
        patches.extend(reflow);
        debug_assert!(self.doc.story_caches_consistent());
        self.undo.push(Edit { label, patches });
        self.redo.clear();
        self.revision += 1;
        Ok(created)
    }

    /// Indices (into `doc.stories`) of the stories that `patches`, already applied, can
    /// have changed: a frame's text or layout, or a wrapping object on a frame's page.
    fn affected_stories(&self, patches: &[Patch]) -> Vec<usize> {
        let stories = &self.doc.stories;
        let on_page = |page: usize, story: &Story| {
            story.frames.iter().any(|&f| {
                self.doc.pages.get(page).is_some_and(|p| p.shapes.iter().any(|s| s.id == f))
            })
        };
        // The fields the layout of a frame reads.
        let layout = |a: &Shape, b: &Shape| {
            a.kind != b.kind
                || a.rect != b.rect
                || a.columns != b.columns
                || a.column_gap_emu != b.column_gap_emu
                || a.rotation_deg != b.rotation_deg
                || a.wrap != b.wrap
                || a.wrap_dist_emu != b.wrap_dist_emu
                || a.inset_emu != b.inset_emu
        };
        let wraps = |s: &Shape| s.wrap != WrapType::None;
        let mut out: Vec<usize> = Vec::new();
        for p in patches {
            for (si, story) in stories.iter().enumerate() {
                let hit = match p {
                    Patch::Replace { page, from, to } => {
                        if story.frames.contains(&from.id) {
                            layout(from, to)
                        } else {
                            (wraps(from) || wraps(to)) && from != to && on_page(*page, story)
                        }
                    }
                    Patch::Insert { page, shape, .. } | Patch::Remove { page, shape, .. } => {
                        wraps(shape) && on_page(*page, story)
                    }
                    Patch::Reorder { page, .. } => {
                        on_page(*page, story)
                            && self.doc.pages[*page].shapes.iter().any(wraps)
                    }
                    Patch::SetPages { .. } => true,
                    Patch::InsertPage { .. }
                    | Patch::RemovePage { .. }
                    | Patch::MovePage { .. }
                    | Patch::InsertStory { .. }
                    | Patch::RemoveStory { .. }
                    | Patch::SetSections { .. }
                    | Patch::SetMasters { .. }
                    | Patch::SetPageMasters { .. } => false,
                    // Only a command (SetStoryText) gets here; a re-flow's own patch is
                    // never passed back in.
                    Patch::SetStory { to, .. } => to.id == story.id,
                };
                if hit && !out.contains(&si) {
                    out.push(si);
                }
            }
        }
        out
    }

    /// Re-flow the stories `patches` (already applied) affect. The edited frame's new text
    /// goes into the story, the story is laid out again and every frame's cached text is
    /// rewritten from the new frame ends. The returned patches are already applied.
    fn reflow_patches(&mut self, patches: &[Patch]) -> Vec<Patch> {
        let mut out = Vec::new();
        if self.doc.stories.is_empty() {
            return out;
        }
        for si in self.affected_stories(patches) {
            let Some((story, caches)) = self.doc.reflow_story(si) else {
                continue;
            };
            self.put_reflowed(si, story, caches, &mut out);
        }
        out
    }

    /// Make the flowed `story` (index `si`) and its frames' caches the document's, as
    /// applied patches pushed on `out`.
    fn put_reflowed(
        &mut self,
        si: usize,
        story: Story,
        caches: Vec<(usize, ShapeId, Vec<Paragraph>)>,
        out: &mut Vec<Patch>,
    ) {
        for (page, id, paragraphs) in caches {
            let Some(from) = self.doc.shape(page, id).cloned() else {
                continue;
            };
            let mut to = from.clone();
            to.kind = ShapeKind::TextBox { paragraphs };
            if to != from {
                let p = Patch::Replace { page, from, to };
                p.apply(&mut self.doc);
                out.push(p);
            }
        }
        if story != self.doc.stories[si] {
            let p = Patch::SetStory {
                from: self.doc.stories[si].clone(),
                to: story,
            };
            p.apply(&mut self.doc);
            out.push(p);
        }
    }

    /// After a command that removed shapes or pages: every story that lost frames keeps its
    /// text and the frames it still has (the chain closes up around the deleted ones and
    /// the text flows into the rest; the last frame going makes the one before it the end
    /// of the story); a story with no frame left is dropped. Then the chains in
    /// `new_stories` are laid out and added. The returned patches are already applied.
    fn relink_patches(&mut self, new_stories: Vec<Story>) -> Vec<Patch> {
        let mut out = Vec::new();
        let mut si = 0;
        while si < self.doc.stories.len() {
            let story = self.doc.stories[si].clone();
            let alive: Vec<ShapeId> = story
                .frames
                .iter()
                .copied()
                .filter(|&f| crate::story::locate(&self.doc, f).is_some())
                .collect();
            if alive.len() == story.frames.len() {
                si += 1;
            } else if alive.is_empty() {
                let p = Patch::RemoveStory { index: si, story };
                p.apply(&mut self.doc);
                out.push(p);
            } else {
                // The story is the truth (a delete does not touch any text).
                let mut kept = Story {
                    frames: alive,
                    ..story.clone()
                };
                match self.doc.reflow_value(&kept, kept.paragraphs.clone()) {
                    Some((flowed, caches)) => self.put_reflowed(si, flowed, caches, &mut out),
                    None => {
                        kept.ends = vec![(kept.paragraphs.len(), 0); kept.frames.len()];
                        let p = Patch::SetStory { from: story, to: kept };
                        p.apply(&mut self.doc);
                        out.push(p);
                    }
                }
                si += 1;
            }
        }
        for story in new_stories {
            let Some((flowed, _)) = self.doc.reflow_value(&story, story.paragraphs.clone()) else {
                continue;
            };
            let p = Patch::InsertStory {
                index: self.doc.stories.len(),
                story: flowed,
            };
            p.apply(&mut self.doc);
            out.push(p);
        }
        out
    }

    /// Build the insert patch for a new top-most shape, allocating its id now.
    fn insert_new(&mut self, page: usize, mut shape: Shape, created: &mut Vec<ShapeId>) -> Patch {
        shape.id = self.doc.alloc_id();
        created.push(shape.id);
        // Earlier patches of the same command are applied only after all are built, so
        // account for shapes already queued on this page.
        let queued = created.len() - 1;
        Patch::Insert {
            page,
            index: self.doc.pages[page].shapes.len() + queued,
            shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let mut p = Page::blank(7_772_400, 10_058_400);
        let mut a = Shape::text_box(
            Rect::new(100, 100, 1000, 500),
            vec![Paragraph::plain("Hello")],
        );
        a.id = 1;
        let mut b = Shape::new(Rect::new(2000, 2000, 800, 800), ShapeKind::Rectangle);
        b.id = 2;
        p.shapes = vec![a, b];
        Editor::new(Document::new("t", vec![p]))
    }

    /// Run `cmd`, check the document changed, undo and check it is restored, redo and
    /// check the changed state returns.
    fn round_trip(mut e: Editor, cmd: Command) -> Editor {
        let before = e.doc().clone();
        e.execute(cmd).unwrap();
        let after = e.doc().clone();
        assert_ne!(before, after, "command changed nothing");
        assert!(e.can_undo() && !e.can_redo());
        assert!(e.undo());
        assert_eq!(e.doc(), &before, "undo did not restore");
        assert!(e.can_redo());
        assert!(e.redo());
        assert_eq!(e.doc(), &after, "redo did not reapply");
        e
    }

    #[test]
    fn move_round_trip() {
        let e = round_trip(
            editor(),
            Command::MoveShapes {
                page: 0,
                ids: vec![1, 2],
                dx: 50,
                dy: -20,
            },
        );
        assert_eq!(
            e.doc().shape(0, 1).unwrap().rect,
            Rect::new(150, 80, 1000, 500)
        );
        assert_eq!(
            e.doc().shape(0, 2).unwrap().rect,
            Rect::new(2050, 1980, 800, 800)
        );
    }

    #[test]
    fn multi_move_is_one_undo_step() {
        let mut e = editor();
        let before = e.doc().clone();
        e.execute(Command::MoveShapes {
            page: 0,
            ids: vec![1, 2],
            dx: 10,
            dy: 20,
        })
        .unwrap();
        assert!(e.undo());
        assert_eq!(e.doc(), &before);
        assert!(!e.can_undo());
    }

    #[test]
    fn align_and_distribute_three_objects() {
        let mut p = Page::blank(7_772_400, 10_058_400);
        for (id, x, w) in [(1, 0, 100), (2, 150, 100), (3, 1000, 100)] {
            let mut s = Shape::new(Rect::new(x, id * 10, w, 50), ShapeKind::Rectangle);
            s.id = id as ShapeId;
            p.shapes.push(s);
        }
        let mut e = Editor::new(Document::new("t", vec![p]));
        e.execute(Command::AlignShapes {
            page: 0,
            ids: vec![1, 2, 3],
            mode: AlignMode::DistributeHorizontally,
            to_margins: false,
        })
        .unwrap();
        let xs: Vec<i64> = (1..=3)
            .map(|i| e.doc().shape(0, i).unwrap().rect.x)
            .collect();
        assert_eq!(xs, vec![0, 500, 1000]);
        e.execute(Command::AlignShapes {
            page: 0,
            ids: vec![1, 2, 3],
            mode: AlignMode::Right,
            to_margins: false,
        })
        .unwrap();
        for i in 1..=3 {
            assert_eq!(e.doc().shape(0, i).unwrap().rect.right(), 1100);
        }
        assert_eq!(e.undo_label(), Some("Align"));
    }

    #[test]
    fn zorder_group_keeps_relative_order() {
        let mut p = Page::blank(1000, 1000);
        for id in 1..=4 {
            let mut s = Shape::new(Rect::new(0, 0, 10, 10), ShapeKind::Rectangle);
            s.id = id;
            p.shapes.push(s);
        }
        let mut e = Editor::new(Document::new("t", vec![p]));
        let order =
            |e: &Editor| -> Vec<ShapeId> { e.doc().pages[0].shapes.iter().map(|s| s.id).collect() };
        let run = |e: &mut Editor, op| {
            e.execute(Command::ZOrderShapes {
                page: 0,
                ids: vec![2, 3],
                op,
            })
            .unwrap();
        };
        run(&mut e, ZOrder::Front);
        assert_eq!(order(&e), vec![1, 4, 2, 3]);
        run(&mut e, ZOrder::Back);
        assert_eq!(order(&e), vec![2, 3, 1, 4]);
        run(&mut e, ZOrder::Forward);
        assert_eq!(order(&e), vec![1, 2, 3, 4]);
        run(&mut e, ZOrder::Backward);
        assert_eq!(order(&e), vec![2, 3, 1, 4]);
        assert!(e.undo());
        assert_eq!(order(&e), vec![1, 2, 3, 4]);
    }

    #[test]
    fn align_to_margins_and_undo() {
        // Page 7_772_400 x 10_058_400, margin 457_200; shape 1 is 1000x500 at (100,100).
        for (mode, want) in [
            (AlignMode::Left, (457_200, 100)),
            (AlignMode::Center, ((7_772_400 - 1000) / 2, 100)),
            (AlignMode::Bottom, (100, 10_058_400 - 457_200 - 500)),
        ] {
            let mut e = editor();
            let before = e.doc().clone();
            e.execute(Command::AlignShapes {
                page: 0,
                ids: vec![1],
                mode,
                to_margins: true,
            })
            .unwrap();
            let r = e.doc().shape(0, 1).unwrap().rect;
            assert_eq!((r.x, r.y), want, "{mode:?}");
            assert_eq!(e.undo_label(), Some("Align"));
            assert!(e.undo());
            assert_eq!(e.doc(), &before);
        }
    }

    #[test]
    fn align_and_distribute_relative_to_each_other() {
        let mut e = editor();
        e.execute(Command::AlignShapes {
            page: 0,
            ids: vec![1, 2],
            mode: AlignMode::Left,
            to_margins: false,
        })
        .unwrap();
        assert_eq!(e.doc().shape(0, 2).unwrap().rect.x, 100);
    }

    #[test]
    fn resize_round_trip() {
        let e = round_trip(
            editor(),
            Command::SetRect {
                page: 0,
                id: 2,
                rect: Rect::new(0, 0, 10, 10),
            },
        );
        assert_eq!(e.doc().shape(0, 2).unwrap().rect, Rect::new(0, 0, 10, 10));
    }

    #[test]
    fn add_shape_round_trip_assigns_fresh_ids() {
        let mut e = editor();
        let ids = e
            .execute(Command::AddShape {
                page: 0,
                shape: Shape::new(Rect::new(0, 0, 5, 5), ShapeKind::Ellipse),
            })
            .unwrap();
        assert_eq!(ids.len(), 1);
        assert!(ids[0] > 2);
        assert_eq!(e.doc().pages[0].shapes.last().unwrap().id, ids[0]);
        let e = round_trip(
            e,
            Command::PasteShapes {
                page: 0,
                shapes: vec![
                    Shape::new(Rect::new(0, 0, 5, 5), ShapeKind::Line),
                    Shape::new(Rect::new(1, 1, 5, 5), ShapeKind::Line),
                ],
            },
        );
        let shapes = &e.doc().pages[0].shapes;
        assert_eq!(shapes.len(), 5);
        let mut ids: Vec<_> = shapes.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5, "ids must be unique");
    }

    #[test]
    fn change_picture_is_one_undo_step_and_rejects_non_pictures() {
        let mut e = editor();
        let pic = |data: u8| ShapeKind::Picture {
            data: Arc::new(vec![data]),
            format: PictureFormat::Png,
            file: None,
        };
        let id = e
            .execute(Command::AddShape {
                page: 0,
                shape: Shape::new(Rect::new(0, 0, 400, 200), pic(1)),
            })
            .unwrap()[0];
        let new_rect = Rect::new(50, 0, 300, 200);
        let e = round_trip(
            e,
            Command::ChangePicture {
                page: 0,
                id,
                data: Arc::new(vec![2]),
                format: PictureFormat::Jpeg,
                file: Some(PictureFile {
                    name: "b.jpg".into(),
                    modified: None,
                }),
                rect: new_rect,
            },
        );
        let s = e.doc().shape(0, id).unwrap();
        assert_eq!(s.rect, new_rect);
        assert!(matches!(&s.kind, ShapeKind::Picture { data, format: PictureFormat::Jpeg, .. } if **data == [2]));
        let mut e = e;
        let err = e.execute(Command::ChangePicture {
            page: 0,
            id: 2,
            data: Arc::new(vec![]),
            format: PictureFormat::Png,
            file: None,
            rect: new_rect,
        });
        assert_eq!(err, Err(CommandError::NotPicture(2)));
    }

    #[test]
    fn delete_round_trip_restores_z_order() {
        let mut e = editor();
        e.execute(Command::AddShape {
            page: 0,
            shape: Shape::new(Rect::new(0, 0, 5, 5), ShapeKind::Ellipse),
        })
        .unwrap();
        let e = round_trip(
            e,
            Command::DeleteShapes {
                page: 0,
                ids: vec![1, 3],
            },
        );
        assert_eq!(e.doc().pages[0].shapes.len(), 1);
        assert_eq!(e.doc().pages[0].shapes[0].id, 2);
    }

    #[test]
    fn set_text_round_trip_and_rejects_non_text() {
        let mut e = editor();
        let err = e
            .execute(Command::SetText {
                page: 0,
                id: 2,
                paragraphs: vec![],
            })
            .unwrap_err();
        assert_eq!(err, CommandError::NotText(2));
        assert!(!e.can_undo());
        let e = round_trip(
            e,
            Command::SetText {
                page: 0,
                id: 1,
                paragraphs: vec![Paragraph::plain("Bye")],
            },
        );
        match &e.doc().shape(0, 1).unwrap().kind {
            ShapeKind::TextBox { paragraphs } => assert_eq!(paragraphs[0].text(), "Bye"),
            _ => panic!(),
        }
    }

    #[test]
    fn set_cell_text_round_trip_and_rejects_bad_targets() {
        let mut e = editor();
        let mut t = Shape::new(
            Rect::new(0, 0, 2000, 1000),
            ShapeKind::Table {
                rows: 1,
                cols: 2,
                col_widths_emu: vec![],
                row_heights_emu: vec![],
                row_set_emu: vec![],
                cells: vec![TableCell::plain("a"), TableCell::plain("b")],
            },
        );
        t.id = 9;
        e.execute(Command::AddShape { page: 0, shape: t }).unwrap();
        let id = e.doc().pages[0].shapes.last().unwrap().id;
        let cmd = |id, cell| Command::SetCellText {
            page: 0,
            id,
            cell,
            paragraphs: vec![Paragraph::plain("x")],
        };
        assert_eq!(e.execute(cmd(2, 0)).unwrap_err(), CommandError::NotTable(2));
        assert_eq!(
            e.execute(cmd(id, 2)).unwrap_err(),
            CommandError::NoSuchCell(id, 2)
        );
        let e = round_trip(e, cmd(id, 1));
        match &e.doc().shape(0, id).unwrap().kind {
            ShapeKind::Table { cells, .. } => {
                assert_eq!(cells[0].paragraphs[0].text(), "a");
                assert_eq!(cells[1].paragraphs[0].text(), "x");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn every_text_format_round_trips() {
        let formats = [
            (TextFormat::FontName("Georgia".into()), "font"),
            (TextFormat::Size(30.0), "size"),
            (TextFormat::Bold(true), "bold"),
            (TextFormat::Italic(true), "italic"),
            (TextFormat::Underline(true), "underline"),
            (TextFormat::Color(Rgb(200, 0, 0)), "color"),
            (TextFormat::Align(Align::Right), "align"),
        ];
        for (format, what) in formats {
            let e = round_trip(
                editor(),
                Command::SetTextFormat {
                    page: 0,
                    ids: vec![1, 2],
                    format,
                },
            );
            let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
                panic!()
            };
            let (p, r) = (&paragraphs[0], &paragraphs[0].runs[0]);
            match what {
                "font" => assert_eq!(r.font, "Georgia"),
                "size" => assert_eq!(r.size_pt, 30.0),
                "bold" => assert!(r.bold),
                "italic" => assert!(r.italic),
                "underline" => assert!(r.underline),
                "color" => assert_eq!(r.color, Rgb(200, 0, 0)),
                _ => assert_eq!(p.align, Align::Right),
            }
        }
        // Only non-text shapes selected: an error and no history entry.
        let mut e = editor();
        assert!(e
            .execute(Command::SetTextFormat {
                page: 0,
                ids: vec![2],
                format: TextFormat::Bold(true)
            })
            .is_err());
        assert!(!e.can_undo());
    }

    #[test]
    fn z_order_round_trip_and_edges() {
        let e = round_trip(editor(), Command::SendBackward { page: 0, id: 2 });
        assert_eq!(e.doc().pages[0].shapes[0].id, 2);
        let e = round_trip(editor(), Command::BringForward { page: 0, id: 1 });
        assert_eq!(e.doc().pages[0].shapes[1].id, 1);
        // Already at the edge: not an error, not recorded.
        let mut e = editor();
        e.execute(Command::BringForward { page: 0, id: 2 }).unwrap();
        e.execute(Command::SendBackward { page: 0, id: 1 }).unwrap();
        assert!(!e.can_undo());
    }

    #[test]
    fn front_and_back_round_trip_and_edges() {
        let e = round_trip(editor(), Command::BringToFront { page: 0, id: 1 });
        let n = e.doc().pages[0].shapes.len();
        assert_eq!(e.doc().pages[0].shapes[n - 1].id, 1);
        let e = round_trip(editor(), Command::SendToBack { page: 0, id: 2 });
        assert_eq!(e.doc().pages[0].shapes[0].id, 2);
        let mut e = editor();
        let (back, front) = (e.doc().pages[0].shapes[0].id, e.doc().pages[0].shapes[1].id);
        e.execute(Command::BringToFront { page: 0, id: front })
            .unwrap();
        e.execute(Command::SendToBack { page: 0, id: back })
            .unwrap();
        assert!(!e.can_undo());
    }

    #[test]
    fn page_commands_round_trip() {
        let e = round_trip(editor(), Command::InsertPage { after: 0 });
        assert_eq!(e.doc().pages.len(), 2);
        assert!(e.doc().pages[1].shapes.is_empty());
        assert_eq!(e.doc().pages[1].width_emu, e.doc().pages[0].width_emu);

        let mut e = editor();
        assert_eq!(
            e.execute(Command::DeletePage { page: 0 }).unwrap_err(),
            CommandError::CannotDeleteLastPage
        );
        e.execute(Command::InsertPage { after: 0 }).unwrap();
        let e = round_trip(e, Command::DeletePage { page: 0 });
        assert_eq!(e.doc().pages.len(), 1);
        assert!(e.doc().pages[0].shapes.is_empty());
    }

    #[test]
    fn duplicate_page_copies_shapes_with_new_ids() {
        let mut e = editor();
        e.execute(Command::AddShape {
            page: 0,
            shape: Shape::new(Rect::new(0, 0, 100, 100), ShapeKind::Rectangle),
        })
        .unwrap();
        let e = round_trip(e, Command::DuplicatePage { page: 0 });
        let (a, b) = (&e.doc().pages[0], &e.doc().pages[1]);
        assert_eq!(a.shapes[0].rect, b.shapes[0].rect);
        assert_eq!(a.shapes.len(), b.shapes.len());
        assert_ne!(a.id, b.id);
        assert_eq!(b.duplicate_of, Some(a.id));
        assert_ne!(a.shapes[0].id, b.shapes[0].id);
    }

    #[test]
    fn errors_for_bad_targets() {
        let mut e = editor();
        assert_eq!(
            e.execute(Command::SetRect {
                page: 3,
                id: 1,
                rect: Rect::new(0, 0, 1, 1)
            })
            .unwrap_err(),
            CommandError::NoSuchPage(3)
        );
        assert_eq!(
            e.execute(Command::DeleteShapes {
                page: 0,
                ids: vec![99]
            })
            .unwrap_err(),
            CommandError::NoSuchShape(99)
        );
    }

    #[test]
    fn new_command_clears_redo_and_multi_step_history() {
        let mut e = editor();
        for dx in [10, 20, 30] {
            e.execute(Command::MoveShapes {
                page: 0,
                ids: vec![1],
                dx,
                dy: 0,
            })
            .unwrap();
        }
        assert_eq!(e.undo_label(), Some("Move"));
        e.undo();
        e.undo();
        assert_eq!(e.doc().shape(0, 1).unwrap().rect.x, 110);
        assert!(e.can_redo());
        e.execute(Command::SetRect {
            page: 0,
            id: 1,
            rect: Rect::new(7, 7, 7, 7),
        })
        .unwrap();
        assert!(!e.can_redo());
        while e.undo() {}
        assert_eq!(e.doc().shape(0, 1).unwrap().rect.x, 100);
        while e.redo() {}
        assert_eq!(e.doc().shape(0, 1).unwrap().rect, Rect::new(7, 7, 7, 7));
    }

    fn two_paragraph_editor() -> Editor {
        let mut p = Page::blank(7_772_400, 10_058_400);
        let mut a = Shape::text_box(
            Rect::new(100, 100, 1000, 500),
            vec![
                Paragraph::plain("Hello big world"),
                Paragraph::plain("Second"),
            ],
        );
        a.id = 1;
        let mut t = Shape::new(
            Rect::new(0, 2000, 900, 900),
            ShapeKind::Table {
                rows: 1,
                cols: 2,
                col_widths_emu: vec![],
                row_heights_emu: vec![],
                row_set_emu: vec![],
                cells: vec![TableCell::plain("left"), TableCell::plain("right")],
            },
        );
        t.id = 2;
        p.shapes = vec![a, t];
        Editor::new(Document::new("t", vec![p]))
    }

    fn runs_of(e: &Editor, para: usize) -> Vec<(String, bool, f32)> {
        let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
            panic!()
        };
        paragraphs[para]
            .runs
            .iter()
            .map(|r| (r.text.clone(), r.bold, r.size_pt))
            .collect()
    }

    #[test]
    fn format_runs_splits_runs_and_round_trips() {
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatRuns {
                page: 0,
                id: 1,
                cell: None,
                range: 6..9,
                change: CharChange::Bold(true),
            },
        );
        assert_eq!(
            runs_of(&e, 0),
            vec![
                ("Hello ".to_owned(), false, DEFAULT_SIZE_PT),
                ("big".to_owned(), true, DEFAULT_SIZE_PT),
                (" world".to_owned(), false, DEFAULT_SIZE_PT),
            ]
        );
        // The second paragraph is untouched.
        assert_eq!(runs_of(&e, 1).len(), 1);
    }

    #[test]
    fn format_runs_merges_identical_neighbours() {
        let mut e = two_paragraph_editor();
        let bold = |range| Command::FormatRuns {
            page: 0,
            id: 1,
            cell: None,
            range,
            change: CharChange::Bold(true),
        };
        e.execute(bold(0..5)).unwrap();
        e.execute(bold(5..15)).unwrap();
        assert_eq!(
            runs_of(&e, 0),
            vec![("Hello big world".to_owned(), true, DEFAULT_SIZE_PT)]
        );
        // Switching it off again merges back into one plain run.
        e.execute(Command::FormatRuns {
            page: 0,
            id: 1,
            cell: None,
            range: 0..15,
            change: CharChange::Bold(false),
        })
        .unwrap();
        assert_eq!(runs_of(&e, 0).len(), 1);
        assert!(!runs_of(&e, 0)[0].1);
        // Applying what is already there records nothing.
        let n = e.undo_label();
        e.execute(Command::FormatRuns {
            page: 0,
            id: 1,
            cell: None,
            range: 0..15,
            change: CharChange::Bold(false),
        })
        .unwrap();
        assert_eq!(e.undo_label(), n);
    }

    #[test]
    fn format_runs_across_paragraphs_and_size_font() {
        // 12..20 covers "rld", the paragraph break and "Seco".
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatRuns {
                page: 0,
                id: 1,
                cell: None,
                range: 12..20,
                change: CharChange::Size(24.0),
            },
        );
        assert_eq!(
            runs_of(&e, 0),
            vec![
                ("Hello big wo".to_owned(), false, DEFAULT_SIZE_PT),
                ("rld".to_owned(), false, 24.0),
            ]
        );
        assert_eq!(
            runs_of(&e, 1),
            vec![
                ("Seco".to_owned(), false, 24.0),
                ("nd".to_owned(), false, DEFAULT_SIZE_PT)
            ]
        );
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatRuns {
                page: 0,
                id: 1,
                cell: None,
                range: 0..15,
                change: CharChange::FontName("Arial".into()),
            },
        );
        assert_eq!(e.doc().shape(0, 1).unwrap().kind, {
            let mut p = Paragraph::plain("Hello big world");
            p.runs[0].font = "Arial".into();
            ShapeKind::TextBox {
                paragraphs: vec![p, Paragraph::plain("Second")],
            }
        });
    }

    #[test]
    fn format_paragraphs_touches_only_selected_paragraphs() {
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatParagraphs {
                page: 0,
                id: 1,
                cell: None,
                range: 3..3,
                align: Align::Center,
            },
        );
        let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
            panic!()
        };
        assert_eq!(paragraphs[0].align, Align::Center);
        assert_eq!(paragraphs[1].align, Align::Left);
        // A selection that ends exactly where the next paragraph begins leaves it alone,
        // while one that reaches into it changes both.
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatParagraphs {
                page: 0,
                id: 1,
                cell: None,
                range: 0..16,
                align: Align::Right,
            },
        );
        let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
            panic!()
        };
        assert_eq!(paragraphs[0].align, Align::Right);
        assert_eq!(paragraphs[1].align, Align::Left);
        let e = round_trip(
            two_paragraph_editor(),
            Command::FormatParagraphs {
                page: 0,
                id: 1,
                cell: None,
                range: 10..20,
                align: Align::Justify,
            },
        );
        let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
            panic!()
        };
        assert!(paragraphs.iter().all(|p| p.align == Align::Justify));
    }

    #[test]
    fn spacing_paragraphs_round_trip_and_target() {
        use crate::text::SpacingChange;
        let change = SpacingChange {
            before_pt: Some(12.0),
            after_pt: None,
            line: Some(crate::model::LineSpacing::Multiple(2.0)),
        };
        let e = round_trip(
            two_paragraph_editor(),
            Command::SpacingParagraphs {
                page: 0,
                id: 1,
                cell: None,
                range: 3..3,
                change,
            },
        );
        let ShapeKind::TextBox { paragraphs } = &e.doc().shape(0, 1).unwrap().kind else {
            panic!()
        };
        assert_eq!(paragraphs[0].spacing.space_before_pt, 12.0);
        assert_eq!(paragraphs[0].spacing.space_after_pt, 6.0);
        assert_eq!(
            paragraphs[0].spacing.line,
            crate::model::LineSpacing::Multiple(2.0)
        );
        assert_eq!(paragraphs[1].spacing, Default::default());
        assert_eq!(
            crate::text::selection_spacing(paragraphs, 0..paragraphs.len() * 20),
            None
        );
        assert!(crate::text::selection_spacing(paragraphs, 3..3).is_some());
    }

    #[test]
    fn format_commands_work_in_table_cells_and_reject_bad_targets() {
        let mut e = round_trip(
            two_paragraph_editor(),
            Command::FormatRuns {
                page: 0,
                id: 2,
                cell: Some(1),
                range: 1..3,
                change: CharChange::Italic(true),
            },
        );
        let ShapeKind::Table { cells, .. } = &e.doc().shape(0, 2).unwrap().kind else {
            panic!()
        };
        assert_eq!(cells[0].paragraphs[0].runs.len(), 1);
        let runs = &cells[1].paragraphs[0].runs;
        assert_eq!(runs.len(), 3);
        assert!(runs[1].italic && runs[1].text == "ig");
        assert_eq!(
            e.execute(Command::FormatRuns {
                page: 0,
                id: 2,
                cell: Some(5),
                range: 0..1,
                change: CharChange::Bold(true)
            }),
            Err(CommandError::NoSuchCell(2, 5))
        );
        assert_eq!(
            e.execute(Command::FormatRuns {
                page: 0,
                id: 1,
                cell: Some(0),
                range: 0..1,
                change: CharChange::Bold(true)
            }),
            Err(CommandError::NotTable(1))
        );
        assert_eq!(
            e.execute(Command::FormatParagraphs {
                page: 0,
                id: 2,
                cell: None,
                range: 0..1,
                align: Align::Left
            }),
            Err(CommandError::NotText(2))
        );
    }

    #[test]
    fn group_move_resize_ungroup_and_undo() {
        let mut e = editor();
        let ids: Vec<ShapeId> = e.doc().pages[0].shapes.iter().map(|s| s.id).collect();
        let before = e.doc().clone();
        let created = e
            .execute(Command::Group {
                page: 0,
                ids: ids[..2].to_vec(),
            })
            .unwrap();
        let g = created[0];
        let doc = e.doc().clone();
        assert!(matches!(doc.shape(0, g).unwrap().kind, ShapeKind::Group));
        assert_eq!(doc.shape(0, ids[0]).unwrap().group, Some(g));
        assert_eq!(doc.pages[0].top_group(ids[1]), g);
        assert!(e
            .execute(Command::Group {
                page: 0,
                ids: vec![ids[0], ids[1]]
            })
            .is_err());
        let r0 = doc.shape(0, ids[0]).unwrap().rect;
        e.execute(Command::MoveShapes {
            page: 0,
            ids: vec![g],
            dx: 10,
            dy: 20,
        })
        .unwrap();
        assert_eq!(
            e.doc().shape(0, ids[0]).unwrap().rect,
            r0.translated(10, 20)
        );
        let gr = e.doc().shape(0, g).unwrap().rect;
        e.execute(Command::SetRect {
            page: 0,
            id: g,
            rect: Rect::new(gr.x, gr.y, gr.w * 2, gr.h * 2),
        })
        .unwrap();
        let moved = e.doc().shape(0, ids[0]).unwrap().rect;
        assert_eq!(moved.w, r0.w * 2);
        e.execute(Command::Ungroup { page: 0, id: g }).unwrap();
        assert!(e.doc().shape(0, g).is_none());
        assert_eq!(e.doc().shape(0, ids[0]).unwrap().group, None);
        while e.undo() {}
        assert_eq!(e.doc(), &before);
        e.execute(Command::Group {
            page: 0,
            ids: ids[..2].to_vec(),
        })
        .unwrap();
        let g2 = e.doc().pages[0]
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Group))
            .unwrap()
            .id;
        e.execute(Command::DeleteShapes {
            page: 0,
            ids: vec![g2],
        })
        .unwrap();
        assert_eq!(
            e.doc().pages[0].shapes.len(),
            before.pages[0].shapes.len() - 2
        );
    }

    #[test]
    fn page_size_moves_shapes_by_half_the_change() {
        // Letter -> A4: the page edges move around the shapes (spec 4Q.4).
        let (w, h) = (7_560_000, 10_692_000);
        let e = round_trip(
            editor(),
            Command::SetPageSize {
                width_emu: w,
                height_emu: h,
            },
        );
        let p = &e.doc().pages[0];
        assert_eq!((p.width_emu, p.height_emu), (w, h));
        let dx = w / 2 - 7_772_400 / 2;
        let dy = h / 2 - 10_058_400 / 2;
        assert_eq!(p.shapes[0].rect, Rect::new(100 + dx, 100 + dy, 1000, 500));
        assert_eq!(p.shapes[1].rect, Rect::new(2000 + dx, 2000 + dy, 800, 800));
    }

    #[test]
    fn orientation_swap_is_one_undo_step_and_applies_to_every_page() {
        let mut e = editor();
        e.execute(Command::InsertPage { after: 0 }).unwrap();
        let (w, h) = (10_058_400, 7_772_400);
        e.execute(Command::SetPageSize {
            width_emu: w,
            height_emu: h,
        })
        .unwrap();
        assert!(e
            .doc()
            .pages
            .iter()
            .all(|p| (p.width_emu, p.height_emu) == (w, h)));
        assert_eq!(e.undo_label(), Some("Page Size"));
        // The shapes of a Letter page moved right by 1.25 in and up by 1.25 in.
        assert_eq!(e.doc().pages[0].shapes[0].rect.x, 100 + 1_143_000);
        assert_eq!(e.doc().pages[0].shapes[0].rect.y, 100 - 1_143_000);
        assert!(e.undo());
        assert!(e
            .doc()
            .pages
            .iter()
            .all(|p| (p.width_emu, p.height_emu) == (7_772_400, 10_058_400)));
        assert_eq!(e.doc().pages[0].shapes[0].rect, Rect::new(100, 100, 1000, 500));
        assert!(e.can_undo(), "the inserted page is still one step back");
        // Same size again: nothing to record.
        let mut e = editor();
        e.execute(Command::SetPageSize {
            width_emu: 7_772_400,
            height_emu: 10_058_400,
        })
        .unwrap();
        assert!(!e.can_undo());
    }

    #[test]
    fn page_size_keeps_off_page_shapes_and_rejects_absurd_sizes() {
        let mut e = editor();
        let big = Shape::new(
            Rect::new(7_000_000, 9_500_000, 500_000, 500_000),
            ShapeKind::Rectangle,
        );
        e.execute(Command::AddShape {
            page: 0,
            shape: big,
        })
        .unwrap();
        e.execute(Command::SetPageSize {
            width_emu: 4_572_000,
            height_emu: 6_400_800,
        })
        .unwrap();
        // 7,000,000 - 1,600,200 = 5,399,800: beyond the 4,572,000 wide page, but kept.
        assert_eq!(e.doc().pages[0].shapes[2].rect.x, 7_000_000 - 1_600_200);
        assert_eq!(
            e.execute(Command::SetPageSize {
                width_emu: 100,
                height_emu: 100
            })
            .unwrap_err(),
            CommandError::BadPageSize
        );
    }

    #[test]
    fn margins_round_trip_and_leave_shapes_alone() {
        let m = Margins::all(228_600);
        let e = round_trip(editor(), Command::SetMargins { margins: m });
        assert_eq!(e.doc().pages[0].margins_emu, m);
        assert_eq!(e.doc().pages[0].shapes[0].rect, Rect::new(100, 100, 1000, 500));
        assert_eq!(e.undo_label(), Some("Margins"));
        let mut e = editor();
        assert_eq!(
            e.execute(Command::SetMargins {
                margins: Margins::all(5_000_000)
            })
            .unwrap_err(),
            CommandError::BadMargins
        );
    }

    #[test]
    fn page_setup_is_one_undo_step_for_size_and_margins() {
        let mut e = editor();
        let m = Margins {
            left: 400_000,
            right: 300_000,
            top: 200_000,
            bottom: 100_000,
        };
        e.execute(Command::SetPageSetup {
            width_emu: 5_029_200,
            height_emu: 7_772_400,
            margins: m,
        })
        .unwrap();
        let p = &e.doc().pages[0];
        assert_eq!((p.width_emu, p.height_emu), (5_029_200, 7_772_400));
        assert_eq!(p.margins_emu, m);
        assert_eq!(e.undo_label(), Some("Page Setup"));
        assert!(e.undo());
        let p = &e.doc().pages[0];
        assert_eq!((p.width_emu, p.height_emu), (7_772_400, 10_058_400));
        assert_ne!(p.margins_emu, m);
        assert!(!e.can_undo(), "size and margins were a single step");
        // Margins must fit the NEW size; nothing changes on error.
        assert_eq!(
            e.execute(Command::SetPageSetup {
                width_emu: 1_000_000,
                height_emu: 1_000_000,
                margins: Margins::all(600_000),
            })
            .unwrap_err(),
            CommandError::BadMargins
        );
        assert_eq!(
            e.execute(Command::SetPageSetup {
                width_emu: 100,
                height_emu: 1_000_000,
                margins: Margins::all(0),
            })
            .unwrap_err(),
            CommandError::BadPageSize
        );
        assert!(!e.can_undo());
    }

    #[test]
    fn align_to_margins_uses_the_four_margins() {
        let mut e = editor();
        e.execute(Command::SetMargins {
            margins: Margins {
                left: 914_400,
                right: 0,
                top: 457_200,
                bottom: 0,
            },
        })
        .unwrap();
        e.execute(Command::AlignShapes {
            page: 0,
            ids: vec![1],
            mode: AlignMode::Left,
            to_margins: true,
        })
        .unwrap();
        assert_eq!(e.doc().pages[0].shapes[0].rect.x, 914_400);
        e.execute(Command::AlignShapes {
            page: 0,
            ids: vec![1],
            mode: AlignMode::Top,
            to_margins: true,
        })
        .unwrap();
        assert_eq!(e.doc().pages[0].shapes[0].rect.y, 457_200);
    }
}
