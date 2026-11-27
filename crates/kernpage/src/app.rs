//! Application state, document actions and the per-frame layout.

/// Height of the tab strip in points (Publisher 2021: about 30 at 100%).
const TAB_H: f32 = 30.0;
/// Room kept at the right end of the tab strip for the Help and Minimize buttons.
const TAB_STRIP_RESERVED: f32 = 84.0;

use crate::filedialog::{FileDialogs, Filter, OpenRequest, SaveRequest};
use crate::fontmgr::FontMgr;
use crate::icons::Icon;
use crate::render::PictureCache;
use eframe::egui::{self, Color32, Context, Key, Modifiers, Ui};
use kernpage_core::geometry::{in_to_emu, Handle};
use kernpage_core::story::{caret_to_pos, delete_at, delete_before, pos_to_caret};
use kernpage_core::text::SelectionFormat;
use kernpage_core::{
    Align, CharChange, Command, Document, Editor, Paragraph, PictureFile, PictureFormat, Rect,
    SaveSource, Script, Shape, ShapeId, ShapeKind, StoryId, StoryPos,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A picture file chosen in the Insert Picture dialog.
struct PickedPicture {
    data: Arc<Vec<u8>>,
    format: PictureFormat,
    file: PictureFile,
    info: kernpage_core::picture::ImageInfo,
}
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Home,
    Insert,
    PageDesign,
    Data,
    Review,
    View,
    Help,
    /// Contextual tabs: shown only while a matching object is selected.
    ShapeFormat,
    TextBox,
    PictureFormat,
    TableDesign,
    Layout,
}

/// What kind of object a selected shape is, for choosing the contextual tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjKind {
    TextBox,
    /// Rectangle, oval, line or group.
    Shape,
    Picture,
    Table,
}

impl ObjKind {
    pub fn of(kind: &ShapeKind) -> ObjKind {
        match kind {
            ShapeKind::TextBox { .. } => ObjKind::TextBox,
            ShapeKind::Picture { .. } => ObjKind::Picture,
            ShapeKind::Table { .. } => ObjKind::Table,
            ShapeKind::Rectangle | ShapeKind::Ellipse | ShapeKind::Line | ShapeKind::Group => {
                ObjKind::Shape
            }
        }
    }

    fn tabs(self) -> &'static [Tab] {
        match self {
            ObjKind::TextBox => &[Tab::ShapeFormat, Tab::TextBox],
            ObjKind::Shape => &[Tab::ShapeFormat],
            ObjKind::Picture => &[Tab::PictureFormat],
            ObjKind::Table => &[Tab::TableDesign, Tab::Layout],
        }
    }
}

/// The contextual tabs shown for a selection: the tabs every selected kind has in common
/// (none for an empty selection), in tab order.
pub fn context_tabs(kinds: &[ObjKind]) -> Vec<Tab> {
    let Some((first, rest)) = kinds.split_first() else {
        return Vec::new();
    };
    first
        .tabs()
        .iter()
        .copied()
        .filter(|t| rest.iter().all(|k| k.tabs().contains(t)))
        .collect()
}

impl Tab {
    pub const CONTEXTUAL: [Tab; 5] = [
        Tab::ShapeFormat,
        Tab::TextBox,
        Tab::PictureFormat,
        Tab::TableDesign,
        Tab::Layout,
    ];

    pub fn is_contextual(self) -> bool {
        Tab::CONTEXTUAL.contains(&self)
    }

    pub const ALL: [Tab; 7] = [
        Tab::Home,
        Tab::Insert,
        Tab::PageDesign,
        Tab::Data,
        Tab::Review,
        Tab::View,
        Tab::Help,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Insert => "Insert",
            Tab::PageDesign => "Page Design",
            Tab::Data => "Data",
            Tab::Review => "Review",
            Tab::View => "View",
            Tab::Help => "Help",
            Tab::ShapeFormat => "Shape Format",
            Tab::TextBox => "Text Box",
            Tab::PictureFormat => "Picture Format",
            Tab::TableDesign => "Table Design",
            Tab::Layout => "Layout",
        }
    }

    pub fn parse(s: &str) -> Option<Tab> {
        let key: String = s
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        Tab::ALL
            .into_iter()
            .chain(Tab::CONTEXTUAL)
            .find(|t| t.name().to_lowercase().replace(' ', "") == key)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZoomMode {
    Custom,
    WholePage,
    PageWidth,
}

#[derive(Clone, Copy)]
pub struct Show {
    pub rulers: bool,
    pub guides: bool,
    pub nav: bool,
    pub scratch: bool,
    pub boundaries: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    TextBox,
    Rectangle,
    Oval,
    Line,
}

#[derive(Clone, Copy)]
pub enum Drag {
    Move {
        id: ShapeId,
        orig: Rect,
        /// Pointer position when the drag began (egui clears `press_origin` on release).
        press: egui::Pos2,
    },
    Resize {
        id: ShapeId,
        handle: Handle,
        orig: Rect,
        /// Pointer position when the drag began.
        press: egui::Pos2,
    },
    /// Rubber-band selection from `start`; `add` (Shift) keeps the current selection.
    Marquee {
        start: (i64, i64),
        add: bool,
    },
    Create {
        tool: Tool,
        start: (i64, i64),
    },
}

pub struct EditState {
    pub page: usize,
    pub id: ShapeId,
    pub text: String,
    pub fresh: bool,
    /// Row-major cell index when editing the text of a table cell.
    pub cell: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    New,
    Open,
    Info,
    SaveAs,
    Print,
    Share,
    Export,
    About,
    Feedback,
    Options,
}

impl Section {
    pub fn parse(name: &str) -> Section {
        match name
            .to_lowercase()
            .replace([' ', '&', '_', '-'], "")
            .as_str()
        {
            "new" => Section::New,
            "open" => Section::Open,
            "info" => Section::Info,
            "saveas" => Section::SaveAs,
            "print" => Section::Print,
            "share" => Section::Share,
            "export" | "send" | "savesend" => Section::Export,
            "about" | "aboutkernpage" => Section::About,
            "feedback" => Section::Feedback,
            "options" | "optionsappearance" => Section::Options,
            _ => Section::Home,
        }
    }
}

#[derive(Default)]
pub struct Options {
    pub screenshot: Option<PathBuf>,
    pub open: Option<PathBuf>,
    pub page: usize,
    pub tab: Option<String>,
    /// Open the File view on this page: home, new, open, info, saveas, print, share, export, about, feedback or options.
    pub backstage: Option<String>,
    /// Start editing a table cell: (index of the shape on the page, row-major cell).
    pub edit_cell: Option<(usize, usize)>,
    /// Start editing a text box: (index of the shape on the page, selected character range).
    pub edit_text: Option<(usize, std::ops::Range<usize>)>,
    /// Open the Line Spacing Options dialog at start-up.
    pub spacing_dialog: bool,
    /// Open the `page-setup` or `custom-margins` dialog at start-up.
    pub dialog: Option<String>,
    /// Open a ribbon drop-down at start-up (debug/screenshot option): `wrap` or `columns`.
    pub menu: Option<String>,
    /// Select these shapes (indices on the page) at start-up.
    pub select: Vec<usize>,
    /// Initial window size in points (debug/screenshot option).
    pub window_size: Option<(f32, f32)>,
    /// Force egui's pixels_per_point (debug/screenshot option `--scale`).
    pub scale: Option<f32>,
    /// Debug: draw this ribbon group collapsed with its popup open.
    pub open_group: Option<String>,
    /// Screenshot options: `--theme light|dark|system`, `--accent teal|blue|purple|graphite|orange`.
    pub theme: Option<String>,
    pub accent: Option<String>,
    /// `--demo`: invented data in place of the real profile (for screenshots); see `crate::demo`.
    pub demo: bool,
}

impl Options {
    /// The appearance the command line asks for, if it asks for any part of it.
    pub fn appearance_override(&self) -> Option<crate::theme::Appearance> {
        use crate::theme::{Accent, ThemeMode};
        if self.theme.is_none() && self.accent.is_none() {
            return None;
        }
        let mut a = crate::theme::Appearance::default();
        if let Some(m) = self.theme.as_deref().and_then(ThemeMode::parse) {
            a.mode = m;
        }
        if let Some(x) = self.accent.as_deref().and_then(Accent::parse) {
            a.accent = x;
        }
        Some(a)
    }
}

/// What the selected paragraphs have in common for bullets, numbering and indents.
pub struct ListSel {
    /// The editable-text range the selection covers.
    pub range: std::ops::Range<usize>,
    /// Their shared list and indents; `None` where they differ.
    pub list: Option<(Option<kernpage_core::ListInfo>, kernpage_core::ParaIndents)>,
}

/// State of the Line Spacing Options dialog.
pub struct SpacingDialog {
    /// Characters (in the editable text) the dialog applies to, fixed when it opened.
    pub range: std::ops::Range<usize>,
    pub before: f32,
    pub after: f32,
    /// Exact points per line instead of a multiple of the natural line height.
    pub exact: bool,
    /// Lines (multiple) or points (exact), depending on `exact`.
    pub line: f32,
}

struct Shot {
    path: PathBuf,
    requested: bool,
    done: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    started: Instant,
    frames: u32,
}

/// Formatting of the selected text (the selection in the in-place editor, or all text of
/// the selected text box), for the ribbon controls.
pub struct FormatInfo {
    pub fmt: SelectionFormat,
    /// Character range the controls apply to (in the editable text).
    pub range: std::ops::Range<usize>,
    /// Only a caret, no selected text: character controls have nothing to apply to.
    pub caret: bool,
}

/// The egui id of the in-place text editor of `edit`.
pub fn edit_widget_id(edit: &EditState) -> egui::Id {
    edit_widget_id_of(edit.id, edit.cell)
}

pub fn edit_widget_id_of(id: ShapeId, cell: Option<usize>) -> egui::Id {
    match cell {
        Some(c) => egui::Id::unique(("cell_edit", id, c)),
        None => egui::Id::unique(("text_edit", id)),
    }
}

/// Time spent in `live_sync` (tests measure the cost of a keystroke in a linked frame).
#[cfg(test)]
pub mod live_timing {
    use std::cell::Cell;
    use std::time::Duration;
    thread_local! {
        static TOTAL: Cell<(u64, Duration)> = const { Cell::new((0, Duration::ZERO)) };
    }
    pub fn record(d: Duration) {
        TOTAL.with(|t| {
            let (n, sum) = t.get();
            t.set((n + 1, sum + d));
        });
    }
    /// (calls, total time) on this thread since the last `take`.
    pub fn take() -> (u64, Duration) {
        TOTAL.with(|t| t.replace((0, Duration::ZERO)))
    }
}

/// The caret of the open text editor, as a character offset (the end when unknown).
fn caret_chars(ctx: &Context, edit: &EditState) -> usize {
    egui::TextEdit::load_state(ctx, edit_widget_id(edit))
        .and_then(|s| s.cursor.char_range())
        .map_or_else(|| edit.text.chars().count(), |r| r.primary.index.0)
}

/// A key that does something special at the edge of a frame of a linked story.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkedKey {
    /// Left / Up at the start: the caret goes to the end of the previous frame.
    Prev,
    /// Right / Down at the end: the caret goes to the start of the next frame.
    Next,
    /// Backspace at the start: delete what precedes the frame's text (the last glyph of
    /// the previous frame, or the paragraph break) and continue there.
    BackspaceJoin,
    /// Delete at the end: pull the next character in from the next frame.
    DeleteJoin,
}

/// Look for a key press that the text editor of a linked frame must not see because the
/// caret is on the edge of the frame; the key press is consumed. Selections (Shift+arrows)
/// stay inside the frame: they simply stop at the edge.
pub fn linked_key(ctx: &Context, doc: &kernpage_core::Document, edit: &EditState) -> Option<LinkedKey> {
    if edit.cell.is_some() {
        return None;
    }
    let story = doc.story_of(edit.id)?;
    let idx = story.frame_index(edit.id)?;
    let r = egui::TextEdit::load_state(ctx, edit_widget_id(edit))?
        .cursor
        .char_range()?;
    if r.primary != r.secondary {
        return None;
    }
    let caret = r.primary.index.0;
    let len = edit.text.chars().count();
    let (first, last) = (idx == 0, idx + 1 == story.frames.len());
    let pressed = |k: Key| ctx.input(|i| i.modifiers.is_none() && i.key_pressed(k));
    let found = if caret == 0 && !first && pressed(Key::Backspace) {
        Some((Key::Backspace, LinkedKey::BackspaceJoin))
    } else if caret == 0 && !first && pressed(Key::ArrowLeft) {
        Some((Key::ArrowLeft, LinkedKey::Prev))
    } else if caret == 0 && !first && pressed(Key::ArrowUp) {
        Some((Key::ArrowUp, LinkedKey::Prev))
    } else if caret == len && !last && pressed(Key::ArrowRight) {
        Some((Key::ArrowRight, LinkedKey::Next))
    } else if caret == len && !last && pressed(Key::ArrowDown) {
        Some((Key::ArrowDown, LinkedKey::Next))
    } else if caret == len && pressed(Key::Delete) {
        Some((Key::Delete, LinkedKey::DeleteJoin))
    } else {
        None
    };
    let (key, action) = found?;
    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    Some(action)
}

/// Which Linking buttons of the Text Box tab are enabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkState {
    pub create: bool,
    pub brk: bool,
    pub next: bool,
    pub prev: bool,
}

/// An action that replaces or closes the open publication; it waits for the unsaved-changes
/// question when there are unsaved changes.
pub enum Pending {
    Exit,
    Close,
    Open,
    OpenPath(PathBuf),
    New(Box<Document>),
}

/// A modal question to the user.
pub enum Prompt {
    /// Save / Don't Save / Cancel before running `then`.
    Unsaved { then: Pending },
    /// Some edits cannot be written into a .pub file yet.
    Unsupported {
        items: Vec<String>,
        target: PathBuf,
        then: Option<Pending>,
    },
}

pub struct App {
    pub editor: Option<Editor>,
    /// The original file of a publication opened from a .pub; `None` for new publications
    /// and for .kern documents.
    pub source: Option<SaveSource>,
    /// The current document is saved in KernPage's own format (.kern): new, recovered and
    /// opened .kern documents. False for a publication opened from (or saved as) a .pub.
    pub kern: bool,
    /// The editor revision last written to the .kern file (a .kern document has no
    /// `SaveSource` to tell whether it changed since).
    kern_saved: Option<u64>,
    /// The document id kept in the .kern manifest across saves of this document.
    kern_id: Option<String>,
    dirty_cache: Option<(u64, bool)>,
    backed_up: HashSet<PathBuf>,
    pub prompt: Option<Prompt>,
    allow_close: bool,
    ctx: Option<Context>,
    pub path: Option<PathBuf>,
    pub tab: Tab,
    pub backstage: bool,
    pub section: Section,
    pub recent: crate::recent::Recent,
    /// Sheet shown in the Print preview (0-based).
    pub print_sheet: usize,
    /// The Print page choices (printer, copies, pages, collation) and the running job.
    pub print: crate::output::PrintState,
    /// Selected tab on the File view's New and Open pages.
    pub bs_tab: usize,
    /// Selected tab (Recent / Pinned) on the File screen's Home page.
    pub home_tab: usize,
    pub page: usize,
    pub zoom: f64,
    pub zoom_mode: ZoomMode,
    pub show: Show,
    pub selection: Option<ShapeId>,
    /// All selected shapes when two or more are selected (else empty); `selection` is
    /// then one of them.
    pub multi: Vec<ShapeId>,
    /// Font Color split button: the colour its main part applies.
    pub last_font_color: kernpage_core::Rgb,
    /// Shape Fill / Shape Outline split buttons: the colour their main part applies.
    pub last_fill_color: kernpage_core::Rgb,
    pub last_outline_color: kernpage_core::Rgb,
    /// Arrange > Align > Relative to Margin Guides.
    pub align_to_margins: bool,
    pub tool: Tool,
    pub drag: Option<Drag>,
    pub edit: Option<EditState>,
    pub clipboard: Vec<Shape>,
    /// Ids (in `clipboard`) of linked text frames: their copies are empty, new, unlinked
    /// boxes and paste a little closer to the original (Publisher 2021).
    pub clipboard_blank: Vec<ShapeId>,
    /// The last overflow answer of the selected text box: (what it was asked for, answer).
    pub overflow_cache: std::cell::Cell<Option<(u64, bool)>>,
    pub error: Option<String>,
    /// The message in `error` is the About box (it shows the logo).
    pub fonts: FontMgr,
    pub pics: PictureCache,
    /// First-page previews for the File screen's start page.
    pub start_thumbs: crate::thumbs::StartThumbs,
    /// The screen's resolution (96 dpi x the display scale): Publisher sizes a picture that
    /// records no resolution at this many pixels per inch.
    pub screen_dpi: f64,
    fonts_needed: Vec<(String, bool, bool)>,
    fonts_rev: Option<u64>,
    pub cursor_emu: Option<(i64, i64)>,
    pub ribbon_collapsed: bool,
    pub reset_scroll: bool,
    pub paste_count: i64,
    pub spacing_dialog: Option<SpacingDialog>,
    pub page_dialog: Option<crate::page_dialog::PageSetupDialog>,
    /// The Create Table dialog (Insert > Table > Insert Table...).
    pub table_dialog: Option<crate::table_ui::TableDialog>,
    /// The Bullets and Numbering dialog (Home > Bullets / Numbering > Bullets and Numbering...).
    pub list_dialog: Option<crate::lists_ui::ListDialog>,
    /// Bullet characters picked in this session, newest first (Bullet Character dialog).
    pub recent_bullets: Vec<(u32, String)>,
    /// A ribbon drop-down still to be opened (debug option `--menu`).
    pub debug_menu: Option<String>,
    /// Select this character range in the in-place editor once it has been shown.
    pub pending_selection: Option<std::ops::Range<usize>>,
    pub text_clip: Option<crate::canvas::TextClip>,
    /// Text of the font size box while it is being typed in.
    pub size_text: String,
    /// The Font Size box had the keyboard (so its next focus loss is a real commit).
    pub size_box_typing: bool,
    /// The Font Name box's text and keyboard flag.
    pub font_text: String,
    pub font_typing: bool,
    shot: Option<Shot>,
    title: String,
    /// The chosen theme and accent (File > Options > Appearance).
    pub appearance: crate::theme::Appearance,
    /// The palette last pushed into egui's visuals.
    applied_palette: Option<crate::theme::Palette>,
    /// The title bar's "Search commands" box.
    pub search: crate::search::SearchState,
    /// AutoRecover timer, writer and settings.
    pub ar: crate::autorecover::AutoRecover,
    /// The publication was loaded from an AutoRecover snapshot and has not been saved
    /// since: it counts as having unsaved changes and Save is Save As.
    pub recovered: bool,
    /// The native file dialogs (a fake in UI tests).
    pub dialogs: Box<dyn FileDialogs>,
    /// File > Options > Save: the check box and the minutes box text while the page is open.
    pub options_buf: Option<(bool, String)>,
    /// File > Options: 0 = Save, 1 = Appearance.
    pub options_cat: usize,
    /// The quiet "Update available" notice (see `update_notice`).
    pub update: crate::update_notice::UpdateNotice,
    /// Test-only: what the last frame's canvas painted (see `crate::ui_tests`).
    #[cfg(test)]
    pub probe: crate::canvas::Probe,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Options) -> App {
        crate::demo::set(opts.demo);
        let appearance = opts.appearance_override().unwrap_or_else(|| {
            if opts.screenshot.is_some() || opts.demo {
                crate::theme::Appearance::default()
            } else {
                crate::theme::Appearance::load_from(&crate::theme::settings_file())
            }
        });
        if let Some(sc) = opts.scale {
            cc.egui_ctx.set_pixels_per_point(sc);
        }
        let mut app = App {
            editor: None,
            source: None,
            kern: true,
            kern_saved: None,
            kern_id: None,
            dirty_cache: None,
            backed_up: HashSet::new(),
            prompt: None,
            allow_close: false,
            ctx: None,
            path: None,
            tab: Tab::Home,
            backstage: true,
            section: Section::Home,
            recent: if opts.demo { crate::demo::recent() } else { crate::recent::Recent::load() },
            print_sheet: 0,
            print: crate::output::PrintState::default(),
            bs_tab: 0,
            home_tab: 0,
            page: 0,
            zoom: 1.0,
            zoom_mode: ZoomMode::WholePage,
            show: Show {
                rulers: true,
                guides: true,
                nav: true,
                scratch: true,
                boundaries: false,
            },
            selection: None,
            multi: Vec::new(),
            last_font_color: kernpage_core::Rgb(255, 0, 0),
            last_fill_color: kernpage_core::Rgb(0x4F, 0x81, 0xBD),
            last_outline_color: kernpage_core::Rgb(0x4F, 0x81, 0xBD),
            align_to_margins: false,
            tool: Tool::Select,
            drag: None,
            edit: None,
            clipboard: Vec::new(),
            clipboard_blank: Vec::new(),
            overflow_cache: std::cell::Cell::new(None),
            error: None,
            fonts: FontMgr::new(&cc.egui_ctx),
            pics: PictureCache::default(),
            start_thumbs: crate::thumbs::StartThumbs::default(),
            screen_dpi: kernpage_core::picture::DEFAULT_DPI,
            fonts_needed: Vec::new(),
            fonts_rev: None,
            cursor_emu: None,
            ribbon_collapsed: false,
            reset_scroll: true,
            paste_count: 0,
            spacing_dialog: None,
            page_dialog: None,
            table_dialog: None,
            list_dialog: None,
            recent_bullets: crate::lists_ui::default_recent(),
            debug_menu: None,
            pending_selection: None,
            text_clip: None,
            size_text: String::new(),
            size_box_typing: false,
            font_text: String::new(),
            font_typing: false,
            shot: None,
            title: String::new(),
            appearance: crate::theme::Appearance::default(),
            applied_palette: None,
            search: Default::default(),
            ar: crate::autorecover::AutoRecover::new(opts.screenshot.is_none() && !opts.demo),
            recovered: false,
            dialogs: Box::new(crate::filedialog::Native),
            options_buf: None,
            options_cat: 0,
            // No check in screenshot runs or tests; a real start asks the launcher in the background.
            update: if cfg!(test) || opts.screenshot.is_some() || opts.demo {
                crate::update_notice::UpdateNotice::idle()
            } else {
                crate::update_notice::UpdateNotice::start(&cc.egui_ctx)
            },
            #[cfg(test)]
            probe: Default::default(),
        };
        app.appearance = appearance;
        app.apply_theme(&cc.egui_ctx);
        // Recover a crashed session's publication first, before any file from the command line.
        if let Some(dir) = app.ar.dir().cloned() {
            let alive = crate::autorecover::is_kernpage_running;
            if let Some(snap) = kernpage_core::autorecover::take_most_recent(&dir, &alive) {
                app.open_recovered(snap);
            }
        }
        if let (Some(p), true) = (&opts.open, app.recovered) {
            // The recovered publication has unsaved changes: ask before replacing it.
            app.request(Pending::OpenPath(p.clone()));
        } else if let Some(p) = &opts.open {
            app.open_path(p);
            app.page = opts
                .page
                .saturating_sub(1)
                .min(app.page_count().saturating_sub(1));
        }
        let tab = opts.tab.as_deref();
        match tab.map(str::to_lowercase).as_deref() {
            Some("file") => app.backstage = true,
            Some(t) => {
                if let Some(t) = Tab::parse(t) {
                    app.tab = t;
                }
            }
            None => {}
        }
        if let Some(page) = opts.backstage.as_deref() {
            app.backstage = true;
            app.section = Section::parse(page);
            // `options-appearance` opens the Appearance category.
            app.options_cat = usize::from(page.to_lowercase().contains("appearance"));
        }
        if let Some((shape_ix, cell)) = opts.edit_cell {
            let id = app
                .editor
                .as_ref()
                .and_then(|e| e.doc().pages.get(app.page))
                .and_then(|p| p.shapes.get(shape_ix))
                .map(|s| s.id);
            if let Some(id) = id {
                app.begin_cell_edit(id, cell);
            }
        }
        if let Some((shape_ix, range)) = opts.edit_text {
            let id = app
                .editor
                .as_ref()
                .and_then(|e| e.doc().pages.get(app.page))
                .and_then(|p| p.shapes.get(shape_ix))
                .map(|s| s.id);
            if let Some(id) = id {
                app.begin_edit(id);
                app.pending_selection = Some(range);
            }
        }
        if !opts.select.is_empty() {
            let ids: Vec<ShapeId> = app
                .editor
                .as_ref()
                .and_then(|e| e.doc().pages.get(app.page))
                .map(|p| {
                    opts.select
                        .iter()
                        .filter_map(|&i| p.shapes.get(i).map(|s| s.id))
                        .collect()
                })
                .unwrap_or_default();
            app.set_selection(ids);
        }
        if opts.menu.as_deref() == Some("zoom") {
            crate::status::ZOOM_MENU_OPEN.with(|c| c.set(true));
        }
        app.debug_menu = opts.menu;
        match opts.dialog.as_deref() {
            Some("page-setup") => app.open_page_dialog(crate::page_dialog::DialogKind::PageSetup),
            Some("custom-margins") => {
                app.open_page_dialog(crate::page_dialog::DialogKind::CustomMargins)
            }
            _ => {}
        }
        if opts.spacing_dialog {
            let ctx = egui::Context::default();
            app.open_spacing_dialog(&ctx);
        }
        if let Some(path) = opts.screenshot {
            app.shot = Some(Shot {
                path,
                requested: false,
                done: Arc::new(AtomicBool::new(false)),
                failed: Arc::new(AtomicBool::new(false)),
                started: Instant::now(),
                frames: 0,
            });
        }
        app
    }

    /// Test-only: the window title the app last asked for.
    #[cfg(test)]
    pub fn window_title(&self) -> &str {
        &self.title
    }

    /// Test-only: the exit was confirmed (the window may close on the next close request).
    #[cfg(test)]
    pub fn exit_allowed(&self) -> bool {
        self.allow_close
    }

    pub fn page_count(&self) -> usize {
        self.editor.as_ref().map_or(0, |e| e.doc().pages.len())
    }

    pub fn cursor_in(&self) -> Option<(f64, f64)> {
        self.cursor_emu.map(|(x, y)| {
            (
                kernpage_core::geometry::emu_to_in(x),
                kernpage_core::geometry::emu_to_in(y),
            )
        })
    }

    // ---- documents ---------------------------------------------------------------

    pub fn new_document(
        &mut self,
        doc: Document,
        path: Option<PathBuf>,
        source: Option<SaveSource>,
    ) {
        self.ar.forget();
        self.recovered = false;
        self.editor = Some(Editor::new(doc));
        self.path = path;
        self.kern = source.is_none();
        self.kern_saved = None;
        self.kern_id = None;
        self.source = source;
        self.dirty_cache = None;
        self.page = 0;
        self.selection = None;
        self.edit = None;
        self.drag = None;
        self.tool = Tool::Select;
        self.pics.clear();
        self.fonts_rev = None;
        self.zoom_mode = ZoomMode::WholePage;
        self.reset_scroll = true;
        self.backstage = false;
        self.print_sheet = 0;
        self.tab = Tab::Home;
    }

    /// Open a publication from an AutoRecover snapshot: unsaved, original name and folder
    /// remembered, undo history empty, no source file (so Save is Save As).
    fn open_recovered(&mut self, snap: kernpage_core::autorecover::Snapshot) {
        let path = snap.meta.original_path.as_deref().map(PathBuf::from);
        let mut doc = snap.document;
        doc.name = snap.meta.display_name;
        self.new_document(doc, path, None);
        self.recovered = true;
        self.ar.last = self.autorecover_key();
    }

    /// What identifies the current content for AutoRecover's dirty-tracking.
    fn autorecover_key(&self) -> Option<kernpage_core::autorecover::ContentKey> {
        let ed = self.editor.as_ref()?;
        let pending = self.edit_command().is_some();
        Some(kernpage_core::autorecover::ContentKey {
            revision: ed.revision(),
            pending_text: self
                .edit
                .as_ref()
                .filter(|_| pending)
                .map(|e| e.text.clone()),
        })
    }

    fn autorecover_meta(&self, doc: &Document) -> kernpage_core::autorecover::Meta {
        kernpage_core::autorecover::Meta {
            original_path: self.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            display_name: doc.name.clone(),
            saved_at: kernpage_core::autorecover::now_unix(),
            pid: std::process::id(),
        }
    }

    /// One AutoRecover tick: snapshot the publication if it has unsaved changes that the
    /// last snapshot does not hold. Serialization happens here; the write is on another thread.
    fn autorecover_snapshot(&mut self) {
        let Some(key) = self.autorecover_key() else {
            return;
        };
        let pending = key.pending_text.is_some();
        let dirty = self.is_dirty() || pending;
        if !dirty {
            // Undone back to the saved state: nothing left to recover.
            self.ar.forget();
            return;
        }
        if !kernpage_core::autorecover::needs_snapshot(dirty, &key, self.ar.last.as_ref()) {
            return;
        }
        let Some(ed) = &self.editor else {
            return;
        };
        let mut doc = ed.doc().clone();
        if let Some(cmd) = self.edit_command() {
            // The text being typed is not in the document yet: apply it to the copy only.
            let mut tmp = Editor::new(doc);
            let _ = tmp.execute(cmd);
            doc = tmp.doc().clone();
        }
        let meta = self.autorecover_meta(&doc);
        self.ar.write(&meta, &doc, key);
    }

    /// Per frame: run the timer, keep the panic hook's copy current.
    fn autorecover_frame(&mut self, ctx: &Context, dirty: bool) {
        if !self.ar.active() {
            return;
        }
        let now = Instant::now();
        if self.ar.due(now) {
            self.autorecover_snapshot();
        }
        if self.ar.settings.enabled {
            ctx.request_repaint_after(self.ar.until_next(now));
        }
        match (&self.editor, dirty) {
            (Some(ed), true) => {
                let meta = self.autorecover_meta(ed.doc());
                self.ar.track_for_panic(ed.revision(), &meta, ed.doc());
            }
            _ => self.ar.clear_panic(),
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        let shown = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let bytes = match read_publication_file(path) {
            Ok(b) => b,
            Err(e) => {
                self.error = Some(format!("KernPage cannot open \"{shown}\".\n\n{e}"));
                return;
            }
        };
        if kernpage_core::native::is_kern(&bytes) {
            self.open_kern(path, &shown, &bytes);
            return;
        }
        let owned = bytes.clone();
        // The reader is meant to return errors, but a broken file must never crash the app.
        let result = std::panic::catch_unwind(move || {
            pub_format::read_bytes(&owned).map(|doc| {
                // Linked text boxes: split each story across its frames like the PDF export.
                let breaks = pub2pdf::chain_breaks(&doc, &pub2pdf::Options::default());
                (doc, breaks)
            })
        });
        match result {
            Ok(Ok((doc, breaks))) => {
                let model = kernpage_core::from_pub_flowed(&shown, &doc, &breaks);
                let source = SaveSource::new(bytes, &doc, &model, &breaks);
                self.new_document(model, Some(path.to_path_buf()), Some(source));
                self.recent.add(path);
            }
            Ok(Err(e)) => {
                self.error = Some(format!("KernPage cannot open \"{shown}\".\n\n{e}"));
            }
            Err(_) => {
                self.error = Some(format!(
                    "KernPage cannot open \"{shown}\".\n\nThe file is damaged or uses a format that is not supported yet."
                ));
            }
        }
    }

    /// Open a KernPage (.kern) file; errors are shown to the user.
    fn open_kern(&mut self, path: &Path, shown: &str, bytes: &[u8]) {
        match read_kern_document(shown, bytes) {
            Ok((doc, id)) => {
                self.new_document(doc, Some(path.to_path_buf()), None);
                self.kern_id = Some(id);
                self.kern_saved = self.editor.as_ref().map(|e| e.revision());
                self.recent.add(path);
            }
            Err(e) => {
                self.error = Some(format!("KernPage cannot open \"{shown}\".\n\n{e}"));
            }
        }
    }

    /// Ask for a file and open it, after the unsaved-changes question.
    pub fn open_dialog(&mut self) {
        self.request(Pending::Open);
    }

    fn open_dialog_now(&mut self) {
        let picked = self.dialogs.pick_open(&OpenRequest {
            title: "Open Publication".to_owned(),
            filters: vec![
                Filter::new("All KernPage and Publisher files (*.kern;*.pub)", &["kern", "pub"]),
                Filter::new("KernPage files (*.kern)", &["kern"]),
                Filter::new("Publisher files (*.pub)", &["pub"]),
                Filter::new("All files", &["*"]),
            ],
        });
        if let Some(p) = picked {
            self.open_path(&p);
        }
    }

    pub fn close_document(&mut self) {
        self.request(Pending::Close);
    }

    fn close_document_now(&mut self) {
        self.commit_edit();
        self.ar.forget();
        self.recovered = false;
        self.editor = None;
        self.source = None;
        self.kern = true;
        self.kern_saved = None;
        self.kern_id = None;
        self.dirty_cache = None;
        self.path = None;
        self.selection = None;
        self.pics.clear();
        self.backstage = true;
        self.section = Section::Home;
    }

    // ---- saving ------------------------------------------------------------------

    /// Whether there is a publication to save (a new one is saved as a new .kern file).
    pub fn can_save(&self) -> bool {
        self.editor.is_some()
    }

    pub fn save_tip(&self) -> &'static str {
        if self.can_save() {
            "Save (Ctrl+S)"
        } else {
            "There is no publication to save"
        }
    }

    /// Whether the publication has changes that are not in its file.
    pub fn is_dirty(&mut self) -> bool {
        let Some(ed) = &self.editor else {
            return false;
        };
        if self.recovered {
            return true;
        }
        let rev = ed.revision();
        if let Some((r, d)) = self.dirty_cache {
            if r == rev {
                return d;
            }
        }
        let dirty = match (&self.source, self.kern_saved) {
            (Some(src), _) => !src.plan(ed.doc()).is_empty(),
            (None, Some(saved)) => saved != rev,
            (None, None) => ed.can_undo(),
        };
        self.dirty_cache = Some((rev, dirty));
        dirty
    }

    /// Run `p` now, or after asking about unsaved changes.
    pub fn request(&mut self, p: Pending) {
        self.commit_edit();
        if self.is_dirty() {
            self.prompt = Some(Prompt::Unsaved { then: p });
        } else {
            self.perform(p);
        }
    }

    fn perform(&mut self, p: Pending) {
        match p {
            Pending::Exit => {
                self.allow_close = true;
                if let Some(ctx) = &self.ctx {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Pending::Close => self.close_document_now(),
            Pending::Open => self.open_dialog_now(),
            Pending::OpenPath(p) => self.open_path(&p),
            Pending::New(doc) => self.new_document(*doc, None, None),
        }
    }

    pub fn save(&mut self, then: Option<Pending>) {
        self.save_impl(false, then);
    }

    pub fn save_as(&mut self, then: Option<Pending>) {
        self.save_impl(true, then);
    }

    fn save_impl(&mut self, as_new: bool, then: Option<Pending>) {
        self.commit_edit();
        if self.editor.is_none() {
            return;
        }
        // A publication that was never saved (or was recovered) has no file to save into:
        // Save is Save As.
        let need_dialog =
            as_new || self.path.is_none() || self.recovered || (!self.kern && self.source.is_none());
        let target = match (&self.path, need_dialog) {
            (Some(p), false) => p.clone(),
            _ => match self.save_dialog(false) {
                Some(p) if crate::output::is_pdf(&p) => {
                    // Exporting is not saving: the publication stays as it is.
                    self.export_pdf_to(&p);
                    return;
                }
                Some(p) => p,
                None => return,
            },
        };
        // The file type decides the writer: the chosen type in Save As, otherwise the
        // document's own format.
        let to_kern = if need_dialog { !is_pub_path(&target) } else { self.kern };
        if to_kern {
            if !need_dialog && !self.is_dirty() && target.exists() {
                // Nothing to write; File > Save still returns to the publication.
                self.backstage = false;
                if let Some(t) = then {
                    self.perform(t);
                }
                return;
            }
            self.write_kern_to(target, then);
            return;
        }
        let fresh = self.source.is_none();
        let as_new = need_dialog || fresh;
        if fresh {
            // The source is assigned now so the Unsupported prompt can continue into
            // `write_to`; it is dropped again below unless the file was written.
            let Some(ed) = &self.editor else {
                return;
            };
            match SaveSource::for_new(ed.doc()) {
                Ok(s) => self.source = Some(s),
                Err(e) => {
                    let shown = target
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    self.error = Some(format!(
                        "KernPage cannot save \"{shown}\".\n\n{e}\n\nNothing was written."
                    ));
                    return;
                }
            }
        }
        let (Some(src), Some(ed)) = (&self.source, &self.editor) else {
            return;
        };
        let plan = src.plan(ed.doc());
        if !plan.unsupported.is_empty() {
            self.prompt = Some(Prompt::Unsupported {
                items: plan.unsupported,
                target,
                then,
            });
            return;
        }
        if !as_new && plan.is_empty() && target.exists() {
            // Nothing to write; File > Save still returns to the publication.
            self.backstage = false;
            if let Some(t) = then {
                self.perform(t);
            }
            return;
        }
        self.write_to(target, then);
        self.drop_unwritten_source();
    }

    /// Write the publication to `target` in KernPage's own format and make that its file.
    fn write_kern_to(&mut self, target: PathBuf, then: Option<Pending>) {
        let Some(ed) = &mut self.editor else {
            return;
        };
        let shown = target
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let id = self
            .kern_id
            .get_or_insert_with(kernpage_core::native::new_document_id)
            .clone();
        let bytes = match kernpage_core::native::write_kern_with_id(ed.doc(), &id) {
            Ok(b) => b,
            Err(e) => {
                self.error = Some(format!(
                    "KernPage cannot save \"{shown}\".\n\n{e}\n\nNothing was written."
                ));
                return;
            }
        };
        let backup = target.exists() && !self.backed_up.contains(&target);
        if let Err(e) = kernpage_core::native::write_file_checked(&target, &bytes, backup) {
            self.error = Some(format!(
                "KernPage cannot save \"{shown}\".\n\n{e}\n\nThe original file was not changed."
            ));
            return;
        }
        if backup {
            self.backed_up.insert(target.clone());
        }
        ed.set_name(&shown);
        self.kern_saved = Some(ed.revision());
        self.kern = true;
        self.source = None;
        self.recent.add(&target);
        self.path = Some(target);
        self.recovered = false;
        self.ar.forget();
        self.dirty_cache = None;
        self.backstage = false;
        if let Some(t) = then {
            self.perform(t);
        }
    }

    /// A never-saved publication has a source only once its first file is written; after a
    /// failed or cancelled first save it goes back to being a new publication.
    fn drop_unwritten_source(&mut self) {
        if self.kern || self.path.is_none() || self.recovered {
            self.source = None;
        }
    }

    /// The Save As dialog: KernPage (.kern), Publisher (.pub) and PDF types. The document's
    /// own type is the default (`prefer_kern` forces .kern); the default file type is listed
    /// first.
    fn save_dialog(&self, prefer_kern: bool) -> Option<PathBuf> {
        let ed = self.editor.as_ref()?;
        let base = self.path.as_ref().and_then(|p| p.file_name()).map_or_else(
            || ed.doc().name.clone(),
            |n| n.to_string_lossy().into_owned(),
        );
        // A recovered publication that was a .pub offers .pub, like the file it came from.
        let was_pub = self.recovered && base.to_ascii_lowercase().ends_with(".pub");
        let kern_default = prefer_kern || (self.kern && !was_pub);
        let stem = strip_native_ext(&base);
        let ext = if kern_default { "kern" } else { "pub" };
        let kern = Filter::new("KernPage file (*.kern)", &["kern"]);
        let publ = Filter::new("Publisher file (*.pub)", &["pub"]);
        let mut filters = if kern_default { vec![kern, publ] } else { vec![publ, kern] };
        filters.push(Filter::new("PDF (*.pdf)", &["pdf"]));
        let mut p = self.dialogs.pick_save(&SaveRequest {
            title: "Save As".to_owned(),
            default_dir: self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf),
            default_name: format!("{stem}.{ext}"),
            filters,
        })?;
        if p.extension().is_none() {
            p.set_extension(ext);
        }
        Some(p)
    }

    /// Build the file from the original bytes and replace `target` with it.
    fn write_to(&mut self, target: PathBuf, then: Option<Pending>) {
        let (Some(src), Some(ed)) = (&mut self.source, &mut self.editor) else {
            return;
        };
        let shown = target
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let (bytes, plan) = match src.build(ed.doc()) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(format!(
                    "KernPage cannot save \"{shown}\".\n\n{e}\n\nNothing was written."
                ));
                return;
            }
        };
        let backup = target.exists() && !self.backed_up.contains(&target);
        if let Err(e) = kernpage_core::write_atomic(&target, &bytes, backup) {
            self.error = Some(format!(
                "KernPage cannot save \"{shown}\".\n\n{e}\n\nThe original file was not changed."
            ));
            return;
        }
        if backup {
            self.backed_up.insert(target.clone());
        }
        src.mark_saved(bytes, ed.doc(), &plan);
        ed.set_name(&shown);
        self.kern = false;
        self.kern_saved = None;
        self.recent.add(&target);
        self.path = Some(target);
        self.recovered = false;
        self.ar.forget();
        self.dirty_cache = None;
        self.backstage = false;
        if let Some(t) = then {
            self.perform(t);
        }
    }

    fn prompts(&mut self, ctx: &Context) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let name = self
            .editor
            .as_ref()
            .map_or_else(String::new, |e| e.doc().name.clone());
        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        #[allow(deprecated)]
        let modal_id = egui::Id::new("save_prompt");
        match prompt {
            Prompt::Unsaved { then } => {
                let mut choice = None; // Some(true) = save, Some(false) = don't save
                let can_save = self.can_save();
                egui::Modal::new(modal_id).show(ctx, |ui| {
                    ui.set_width(400.0);
                    ui.heading("KernPage");
                    ui.add_space(8.0);
                    ui.label(format!(
                        "Do you want to save the changes you made to \"{name}\"?"
                    ));
                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button("  Cancel  ").clicked() {
                            choice = Some(None);
                        }
                        if ui.button("  Don't Save  ").clicked() {
                            choice = Some(Some(false));
                        }
                        if can_save && ui.button("  Save  ").clicked() {
                            choice = Some(Some(true));
                        }
                    });
                });
                if esc {
                    choice = Some(None);
                }
                match choice {
                    None => self.prompt = Some(Prompt::Unsaved { then }),
                    Some(None) => {}
                    Some(Some(false)) => self.perform(then),
                    Some(Some(true)) => self.save(Some(then)),
                }
            }
            Prompt::Unsupported {
                items,
                target,
                then,
            } => {
                let mut choice = None; // Some(true) = save supported changes, Some(false) = cancel
                let mut kern_copy = false;
                egui::Modal::new(modal_id).show(ctx, |ui| {
                    ui.set_width(440.0);
                    ui.heading("KernPage");
                    ui.add_space(8.0);
                    ui.label("KernPage can't save these changes into a .pub file yet:");
                    ui.add_space(4.0);
                    for it in &items {
                        ui.label(format!("   \u{2022} {it}"));
                    }
                    ui.add_space(4.0);
                    ui.label("Your other changes can be saved; the unsupported ones will be lost in the saved file.");
                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button("  Cancel  ").clicked() {
                            choice = Some(false);
                        }
                        if ui.button("  Save supported changes  ").clicked() {
                            choice = Some(true);
                        }
                        if ui.button("  Save a KernPage copy (.kern)  ").clicked() {
                            kern_copy = true;
                        }
                    });
                });
                if esc {
                    choice = Some(false);
                }
                if kern_copy {
                    // Save As with .kern preselected; cancelling it returns to this question.
                    if let Some(mut p) = self.save_dialog(true) {
                        if crate::output::is_pdf(&p) {
                            self.export_pdf_to(&p);
                        } else {
                            if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("kern")) {
                                p.set_extension("kern");
                            }
                            self.write_kern_to(p, then);
                            self.drop_unwritten_source();
                            return;
                        }
                    }
                    self.prompt = Some(Prompt::Unsupported { items, target, then });
                    return;
                }
                match choice {
                    None => {
                        self.prompt = Some(Prompt::Unsupported {
                            items,
                            target,
                            then,
                        })
                    }
                    Some(false) => self.drop_unwritten_source(),
                    Some(true) => {
                        self.write_to(target, then);
                        self.drop_unwritten_source();
                    }
                }
            }
        }
    }

    /// Ask for a PNG or JPEG and read it; errors are shown to the user.
    fn pick_picture(&mut self) -> Option<PickedPicture> {
        let path = self.dialogs.pick_open(&OpenRequest {
            title: "Insert Picture".to_owned(),
            filters: vec![Filter::new("Pictures", &["png", "jpg", "jpeg"])],
        })?;
        let shown = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.error = Some(format!("KernPage cannot read \"{shown}\".

{e}"));
                return None;
            }
        };
        let format = kernpage_core::picture::sniff_format(&bytes);
        let info = kernpage_core::picture::image_info(&bytes).filter(|i| {
            i.width_px > 0 && i.height_px > 0 && format != PictureFormat::Unknown
        });
        let Some(info) = info else {
            self.error = Some(format!("KernPage cannot insert \"{shown}\".

It is not a PNG or JPEG picture it can read."));
            return None;
        };
        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| kernpage_core::picture::filetime_from_unix(d.as_secs(), d.subsec_nanos()));
        Some(PickedPicture {
            data: Arc::new(bytes),
            format,
            file: PictureFile {
                name: shown,
                modified,
            },
            info,
        })
    }

    /// Insert > Picture: the picture at its natural size, centred on the page, scaled down
    /// proportionally only when it does not fit.
    pub fn insert_picture_dialog(&mut self) {
        let Some(p) = self.pick_picture() else { return };
        let (page_w, page_h) = self.page_size();
        let rect = kernpage_core::picture::insert_rect(p.info, self.screen_dpi, page_w, page_h);
        self.add_shape(Shape::new(
            rect,
            ShapeKind::Picture {
                data: p.data,
                format: p.format,
                file: Some(p.file),
            },
        ));
    }

    /// The selected picture, when exactly one picture is selected.
    pub fn selected_picture(&self) -> Option<(ShapeId, Rect)> {
        let ids = self.selected_ids();
        let [id] = ids[..] else { return None };
        let s = self.editor.as_ref()?.doc().shape(self.page, id)?;
        matches!(s.kind, ShapeKind::Picture { .. }).then_some((id, s.rect))
    }

    /// Change Picture...: another image in the same frame position, keeping the frame's
    /// height and centre and taking the width from the new image's proportions.
    pub fn change_picture_dialog(&mut self) {
        let Some((id, old)) = self.selected_picture() else {
            return;
        };
        let Some(p) = self.pick_picture() else { return };
        let rect = kernpage_core::picture::change_rect(old, p.info);
        self.exec(Command::ChangePicture {
            page: self.page,
            id,
            data: p.data,
            format: p.format,
            file: Some(p.file),
            rect,
        });
    }

    /// Remove Picture. Publisher leaves an empty picture frame; empty frames are not
    /// supported yet, so the picture object is deleted.
    pub fn remove_picture(&mut self) {
        if self.selected_picture().is_some() {
            self.delete_selection();
        }
    }

    pub fn page_size(&self) -> (i64, i64) {
        self.editor
            .as_ref()
            .and_then(|e| e.doc().pages.get(self.page))
            .map_or((0, 0), |p| (p.width_emu, p.height_emu))
    }

    // ---- editing -----------------------------------------------------------------

    /// Run a command, first committing any text being edited.
    pub fn exec(&mut self, cmd: Command) -> Vec<ShapeId> {
        let text_cmd = matches!(
            cmd,
            Command::SetText { .. }
                | Command::SetCellText { .. }
                | Command::FormatRuns { .. }
                | Command::FormatParagraphs { .. }
                | Command::SpacingParagraphs { .. }
                | Command::ListParagraphs { .. }
        );
        if !text_cmd {
            self.commit_edit();
        }
        let Some(ed) = &mut self.editor else {
            return Vec::new();
        };
        match ed.execute(cmd) {
            Ok(ids) => {
                // Formatting a linked frame can move text into the next frame (or pull it
                // back): the editor shows what the frame holds now.
                if text_cmd {
                    self.show_flowed_text();
                }
                ids
            }
            Err(e) => {
                self.error = Some(format!("KernPage could not do that: {e}."));
                Vec::new()
            }
        }
    }

    pub fn add_shape(&mut self, shape: Shape) -> Option<ShapeId> {
        let ids = self.exec(Command::AddShape {
            page: self.page,
            shape,
        });
        self.selection = ids.first().copied();
        self.tool = Tool::Select;
        self.selection
    }

    pub fn begin_edit(&mut self, id: ShapeId) {
        self.commit_edit();
        let Some(ed) = &self.editor else { return };
        let Some(shape) = ed.doc().shape(self.page, id) else {
            return;
        };
        if let ShapeKind::TextBox { paragraphs } = &shape.kind {
            self.edit = Some(EditState {
                page: self.page,
                id,
                text: kernpage_core::text::paragraphs_to_text(paragraphs),
                fresh: true,
                cell: None,
            });
            self.selection = Some(id);
        }
    }

    /// Put a text cursor in cell `cell` (row-major) of table `id`.
    pub fn begin_cell_edit(&mut self, id: ShapeId, cell: usize) {
        if self
            .edit
            .as_ref()
            .is_some_and(|e| e.id == id && e.cell == Some(cell) && e.page == self.page)
        {
            return;
        }
        self.commit_edit();
        let Some(ed) = &self.editor else { return };
        let Some(shape) = ed.doc().shape(self.page, id) else {
            return;
        };
        if let ShapeKind::Table { cells, .. } = &shape.kind {
            if let Some(c) = cells.get(cell) {
                self.edit = Some(EditState {
                    page: self.page,
                    id,
                    text: kernpage_core::text::paragraphs_to_text(&c.paragraphs),
                    fresh: true,
                    cell: Some(cell),
                });
                self.selection = Some(id);
            }
        }
    }

    /// The command that stores the text being edited in the document, if it changed.
    fn edit_command(&self) -> Option<Command> {
        let edit = self.edit.as_ref()?;
        let shape = self.editor.as_ref()?.doc().shape(edit.page, edit.id)?;
        match (&shape.kind, edit.cell) {
            (ShapeKind::TextBox { paragraphs }, None) => {
                let new = kernpage_core::text::rebuild_paragraphs(paragraphs, &edit.text);
                (&new != paragraphs).then_some(Command::SetText {
                    page: edit.page,
                    id: edit.id,
                    paragraphs: new,
                })
            }
            (ShapeKind::Table { cells, .. }, Some(cell)) => cells.get(cell).and_then(|c| {
                let new = kernpage_core::text::rebuild_paragraphs(&c.paragraphs, &edit.text);
                (new != c.paragraphs).then_some(Command::SetCellText {
                    page: edit.page,
                    id: edit.id,
                    cell,
                    paragraphs: new,
                })
            }),
            _ => None,
        }
    }

    /// Store the text typed so far in the document without leaving the editor. In a linked
    /// frame this is a live edit (see `Editor::execute_live`): the story re-flows, and all
    /// the changes of one editing session stay one undo step.
    pub fn sync_edit(&mut self) {
        // A table cell is live too: its row grows with the text as it is typed, and the
        // typing stays one undo step.
        let linked = self.edit_story_frame().is_some()
            || self.edit.as_ref().is_some_and(|e| e.cell.is_some());
        if let Some(cmd) = self.edit_command() {
            if let Some(ed) = &mut self.editor {
                let _ = if linked {
                    ed.execute_live(cmd)
                } else {
                    ed.execute(cmd)
                };
            }
            self.show_flowed_text();
        }
    }

    /// A linked frame may hold less than was typed (the rest flowed into the next frame) or
    /// more (text came back): the buffer shows what the frame holds now, so a later sync
    /// does not put the moved text back. Returns whether the buffer changed.
    fn show_flowed_text(&mut self) -> bool {
        let (Some(ed), Some(edit)) = (&self.editor, &mut self.edit) else {
            return false;
        };
        if edit.cell.is_some() || ed.doc().story_of(edit.id).is_none() {
            return false;
        }
        if let Some(Shape {
            kind: ShapeKind::TextBox { paragraphs },
            ..
        }) = ed.doc().shape(edit.page, edit.id)
        {
            let text = kernpage_core::text::paragraphs_to_text(paragraphs);
            if text != edit.text {
                edit.text = text;
                return true;
            }
        }
        false
    }

    /// (story index, frame index) of the text box being edited, if it is in a linked story.
    fn edit_story_frame(&self) -> Option<(usize, usize)> {
        let edit = self.edit.as_ref().filter(|e| e.cell.is_none())?;
        let doc = self.editor.as_ref()?.doc();
        let si = doc.stories.iter().position(|s| s.frames.contains(&edit.id))?;
        Some((si, doc.stories[si].frame_index(edit.id)?))
    }

    /// The text of the typed-in frame was changed (a key press): re-flow the story at once
    /// so the next frames show the new text, and follow the caret into another frame when
    /// the text it is in has moved there.
    pub fn live_sync(&mut self, ctx: &Context) {
        if self.edit.as_ref().is_some_and(|e| e.cell.is_some()) {
            self.sync_edit();
            return;
        }
        let Some((si, idx)) = self.edit_story_frame() else {
            return;
        };
        let (Some(edit), Some(ed)) = (&self.edit, &self.editor) else {
            return;
        };
        let story = &ed.doc().stories[si];
        let caret = caret_chars(ctx, edit);
        let pos = caret_to_pos(story.start_of(idx), &edit.text, caret);
        let sid = story.id;
        #[cfg(test)]
        let started = Instant::now();
        self.sync_edit();
        self.follow_caret(ctx, sid, pos, idx);
        #[cfg(test)]
        live_timing::record(started.elapsed());
    }

    /// Put the editor where story position `pos` is now (searching from frame `from` on):
    /// in the same frame (its text, and so the caret, may have changed because text
    /// flowed out of it) or in the frame the text moved to.
    fn follow_caret(&mut self, ctx: &Context, story: StoryId, pos: StoryPos, from: usize) {
        let (Some(ed), Some(edit)) = (&self.editor, &self.edit) else {
            return;
        };
        let doc = ed.doc();
        let Some(st) = doc.stories.iter().find(|s| s.id == story) else {
            return;
        };
        let j = st.frame_for(pos, from);
        let frame = st.frames[j];
        let Some((page, shape)) = doc
            .pages
            .iter()
            .enumerate()
            .find_map(|(pg, p)| p.shape(frame).map(|s| (pg, s)))
        else {
            return;
        };
        let ShapeKind::TextBox { paragraphs } = &shape.kind else {
            return;
        };
        let text = kernpage_core::text::paragraphs_to_text(paragraphs);
        let caret = pos_to_caret(st.start_of(j), &text, pos.min(st.end_of(j)));
        if frame == edit.id {
            if text != edit.text {
                if let Some(e) = &mut self.edit {
                    e.text = text;
                }
                self.pending_selection = Some(caret..caret);
                ctx.request_repaint();
            }
        } else {
            self.edit_frame(ctx, frame, page, text, caret);
        }
    }

    /// Continue editing in frame `frame` (on `page`) with the caret at character `caret`.
    fn edit_frame(&mut self, ctx: &Context, frame: ShapeId, page: usize, text: String, caret: usize) {
        self.edit = Some(EditState {
            page,
            id: frame,
            text,
            fresh: true,
            cell: None,
        });
        self.selection = Some(frame);
        self.multi.clear();
        if self.page != page {
            self.page = page;
            self.reset_scroll = true;
        }
        self.pending_selection = Some(caret..caret);
        ctx.request_repaint();
    }

    /// A key at the edge of a linked frame (see [`linked_key`]): move on into the next or
    /// previous frame, or delete across the boundary.
    pub fn linked_edge_key(&mut self, ctx: &Context, key: LinkedKey) {
        self.sync_edit();
        let Some((si, idx)) = self.edit_story_frame() else {
            return;
        };
        let (Some(edit), Some(ed)) = (&self.edit, &self.editor) else {
            return;
        };
        let doc = ed.doc();
        let story = &doc.stories[si];
        let sid = story.id;
        let text_of = |frame: ShapeId| {
            doc.pages.iter().enumerate().find_map(|(pg, p)| match p.shape(frame) {
                Some(Shape {
                    kind: ShapeKind::TextBox { paragraphs },
                    ..
                }) => Some((pg, kernpage_core::text::paragraphs_to_text(paragraphs))),
                _ => None,
            })
        };
        match key {
            LinkedKey::Prev | LinkedKey::Next => {
                let to = if key == LinkedKey::Prev {
                    idx.checked_sub(1)
                } else {
                    Some(idx + 1).filter(|&n| n < story.frames.len())
                };
                let Some(to) = to else { return };
                let frame = story.frames[to];
                if let Some((pg, text)) = text_of(frame) {
                    let caret = if key == LinkedKey::Prev {
                        text.chars().count()
                    } else {
                        0
                    };
                    self.edit_frame(ctx, frame, pg, text, caret);
                }
            }
            LinkedKey::BackspaceJoin | LinkedKey::DeleteJoin => {
                let start = story.start_of(idx);
                let (paras, pos, from) = if key == LinkedKey::BackspaceJoin {
                    match delete_before(&story.paragraphs, start) {
                        Some((p, at)) => (p, at, idx.saturating_sub(1)),
                        None => return,
                    }
                } else {
                    let end = caret_to_pos(start, &edit.text, edit.text.chars().count());
                    match delete_at(&story.paragraphs, end) {
                        Some(p) => (p, end, idx),
                        None => return,
                    }
                };
                if let Some(ed) = &mut self.editor {
                    let _ = ed.execute_live(Command::SetStoryText {
                        story: sid,
                        paragraphs: paras,
                    });
                }
                self.follow_caret(ctx, sid, pos, from);
                // The edited frame's text changed under the editor: show it.
                self.show_flowed_text();
            }
        }
    }

    /// The text box the Linking commands and the link marks are about: the one being
    /// edited, or the single selected one. `None` for several shapes or no text box.
    pub fn linked_selection(&self) -> Option<ShapeId> {
        let ed = self.editor.as_ref()?;
        let id = match &self.edit {
            Some(e) if e.cell.is_none() => e.id,
            Some(_) => return None,
            None if self.multi.is_empty() => self.selection?,
            None => return None,
        };
        ed.doc().shape(self.page, id).filter(|s| s.is_text_box()).map(|_| id)
    }

    /// Text Box tab > Linking: which of Create Link, Break, Next and Previous are enabled
    /// (Publisher 2021): Create Link where there is no next link, the other three by the
    /// links the frame has; all off for several shapes.
    pub fn link_state(&self) -> LinkState {
        let (Some(id), Some(ed)) = (self.linked_selection(), &self.editor) else {
            return LinkState::default();
        };
        let (prev, next) = ed.doc().frame_links(id);
        LinkState {
            create: !next,
            brk: next,
            next,
            prev,
        }
    }

    /// Whether the selected text box shows overflow (its text does not fit; for a chain
    /// only its last frame does). The answer is kept while nothing it depends on changes.
    pub fn selection_overflows(&self) -> bool {
        use std::hash::{Hash, Hasher};
        let (Some(id), Some(ed)) = (self.linked_selection(), &self.editor) else {
            return false;
        };
        let doc = ed.doc();
        // A standalone box being edited has its newest text in the editor only.
        let typed = self.edit.as_ref().filter(|e| e.id == id).map(|e| e.text.as_str());
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (ed.revision(), self.page, id, typed).hash(&mut h);
        let key = h.finish();
        if let Some((k, v)) = self.overflow_cache.get() {
            if k == key {
                return v;
            }
        }
        let Some(ShapeKind::TextBox { paragraphs }) = doc.shape(self.page, id).map(|s| &s.kind)
        else {
            return false;
        };
        let rebuilt;
        let paras: &[Paragraph] = match typed {
            Some(t) if doc.story_of(id).is_none() => {
                rebuilt = kernpage_core::text::rebuild_paragraphs(paragraphs, t);
                &rebuilt
            }
            _ => paragraphs,
        };
        let v = doc.frame_overflows(self.page, id, paras);
        self.overflow_cache.set(Some((key, v)));
        v
    }

    /// Linking > Next / Previous (and the arrows beside a frame): text editing continues in
    /// the next (previous) frame of the chain, the caret at the start (end) of its text,
    /// on whatever page that frame is.
    pub fn linked_step(&mut self, ctx: &Context, forward: bool) {
        let Some(id) = self.linked_selection() else {
            return;
        };
        self.commit_edit();
        let Some(ed) = &self.editor else { return };
        let doc = ed.doc();
        let Some(story) = doc.story_of(id) else { return };
        let Some(at) = story.frame_index(id) else { return };
        let to = if forward {
            Some(at + 1).filter(|&n| n < story.frames.len())
        } else {
            at.checked_sub(1)
        };
        let Some(frame) = to.map(|n| story.frames[n]) else {
            return;
        };
        let found = doc.pages.iter().enumerate().find_map(|(pg, p)| match p.shape(frame) {
            Some(Shape {
                kind: ShapeKind::TextBox { paragraphs },
                ..
            }) => Some((pg, kernpage_core::text::paragraphs_to_text(paragraphs))),
            _ => None,
        });
        if let Some((pg, text)) = found {
            let caret = if forward { 0 } else { text.chars().count() };
            self.edit_frame(ctx, frame, pg, text, caret);
        }
    }

    pub fn commit_edit(&mut self) {
        self.sync_edit();
        self.edit = None;
        if let Some(ed) = &mut self.editor {
            ed.end_live();
        }
    }

    /// The single selected shape (`None` when several are selected).
    pub fn selected(&self) -> Option<&Shape> {
        if !self.multi.is_empty() {
            return None;
        }
        let ed = self.editor.as_ref()?;
        ed.doc().shape(self.page, self.selection?)
    }

    /// Every selected shape id (stacking order is not implied).
    pub fn selected_ids(&self) -> Vec<ShapeId> {
        if self.multi.is_empty() {
            self.selection.into_iter().collect()
        } else {
            self.multi.clone()
        }
    }

    /// The contextual tabs for the current selection (see [`context_tabs`]).
    pub fn context_tabs(&self) -> Vec<Tab> {
        let Some(ed) = &self.editor else {
            return Vec::new();
        };
        let kinds: Vec<ObjKind> = self
            .selected_ids()
            .iter()
            .filter_map(|id| ed.doc().shape(self.page, *id))
            .map(|s| ObjKind::of(&s.kind))
            .collect();
        context_tabs(&kinds)
    }

    /// Select exactly `ids`.
    pub fn set_selection(&mut self, ids: Vec<ShapeId>) {
        crate::canvas::apply_selection(&mut self.selection, &mut self.multi, ids);
    }

    /// The paragraphs being formatted: the text as typed so far while editing, otherwise
    /// those of the selected text box.
    fn formatted_paragraphs(&self) -> Option<Vec<Paragraph>> {
        let ed = self.editor.as_ref()?;
        if let Some(edit) = &self.edit {
            let shape = ed.doc().shape(edit.page, edit.id)?;
            let base = match (&shape.kind, edit.cell) {
                (ShapeKind::TextBox { paragraphs }, None) => paragraphs,
                (ShapeKind::Table { cells, .. }, Some(c)) => &cells.get(c)?.paragraphs,
                _ => return None,
            };
            return Some(kernpage_core::text::rebuild_paragraphs(base, &edit.text));
        }
        match &self.selected()?.kind {
            ShapeKind::TextBox { paragraphs } => Some(paragraphs.clone()),
            _ => None,
        }
    }

    pub fn format_info(&self, ctx: &Context) -> Option<FormatInfo> {
        let paragraphs = self.formatted_paragraphs()?;
        let len = kernpage_core::text::text_len(&paragraphs);
        let range = match &self.edit {
            Some(edit) => egui::TextEdit::load_state(ctx, edit_widget_id(edit))
                .and_then(|s| s.cursor.char_range())
                .map_or(0..0, |r| {
                    let (a, b) = (r.primary.index.0, r.secondary.index.0);
                    a.min(b).min(len)..a.max(b).min(len)
                }),
            None => 0..len,
        };
        Some(FormatInfo {
            fmt: kernpage_core::text::selection_format(&paragraphs, range.clone()),
            caret: range.is_empty(),
            range,
        })
    }

    /// The size of the first selected character, for Grow / Shrink on mixed sizes.
    pub fn first_selected_size(&self, ctx: &Context, info: &FormatInfo) -> f32 {
        let _ = ctx;
        self.formatted_paragraphs()
            .map(|p| {
                let at = info.range.start;
                kernpage_core::text::selection_format(&p, at..at + 1)
                    .size
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0)
    }

    /// The text box (and table cell) that formatting applies to.
    fn format_target(&self) -> Option<(usize, ShapeId, Option<usize>)> {
        match &self.edit {
            Some(e) => Some((e.page, e.id, e.cell)),
            None => Some((self.page, self.selected()?.id, None)),
        }
    }

    /// Apply a character change to the selected text (undoable as one step).
    pub fn format_chars(&mut self, ctx: &Context, change: CharChange) {
        let Some(info) = self.format_info(ctx) else {
            return;
        };
        let Some((page, id, cell)) = self.format_target() else {
            return;
        };
        if info.caret {
            return;
        }
        self.sync_edit();
        self.exec(Command::FormatRuns {
            page,
            id,
            cell,
            range: info.range,
            change,
        });
        self.refocus_edit();
    }

    /// Set the alignment of the paragraphs the selection touches.
    pub fn format_align(&mut self, ctx: &Context, align: Align) {
        let Some(info) = self.format_info(ctx) else {
            return;
        };
        let Some((page, id, cell)) = self.format_target() else {
            return;
        };
        self.sync_edit();
        self.exec(Command::FormatParagraphs {
            page,
            id,
            cell,
            range: info.range,
            align,
        });
        self.refocus_edit();
    }

    /// The spacing shared by the selected paragraphs; `None` where they differ.
    pub fn format_spacing_info(&self, ctx: &Context) -> Option<kernpage_core::ParaSpacing> {
        let info = self.format_info(ctx)?;
        let paragraphs = self.formatted_paragraphs()?;
        kernpage_core::text::selection_spacing(&paragraphs, info.range)
    }

    /// Set the spacing of the paragraphs `range` touches (the current selection if `None`).
    pub fn format_spacing(
        &mut self,
        ctx: &Context,
        range: Option<std::ops::Range<usize>>,
        change: kernpage_core::text::SpacingChange,
    ) {
        let range = match range {
            Some(r) => r,
            None => match self.format_info(ctx) {
                Some(i) => i.range,
                None => return,
            },
        };
        let Some((page, id, cell)) = self.format_target() else {
            return;
        };
        self.sync_edit();
        self.exec(Command::SpacingParagraphs {
            page,
            id,
            cell,
            range,
            change,
        });
        self.refocus_edit();
    }

    /// The bullets, numbering and indents the selected paragraphs share (`list` is `None`
    /// where they differ); `None` when no text is selected or being edited.
    pub fn list_selection(&self, ctx: &Context) -> Option<ListSel> {
        let info = self.format_info(ctx)?;
        let paragraphs = self.formatted_paragraphs()?;
        let list = kernpage_core::text::selection_list(&paragraphs, info.range.clone());
        Some(ListSel {
            range: info.range,
            list,
        })
    }

    /// Apply a bullets, numbering or indent change to the paragraphs `range` touches (the
    /// current selection if `None`): one command, so one undo step.
    pub fn format_list(
        &mut self,
        ctx: &Context,
        range: Option<std::ops::Range<usize>>,
        change: kernpage_core::text::ListChange,
    ) {
        let range = match range {
            Some(r) => r,
            None => match self.format_info(ctx) {
                Some(i) => i.range,
                None => return,
            },
        };
        let Some((page, id, cell)) = self.format_target() else {
            return;
        };
        self.sync_edit();
        self.exec(Command::ListParagraphs {
            page,
            id,
            cell,
            range,
            change,
        });
        self.refocus_edit();
    }

    /// Open the Line Spacing Options dialog, prefilled from the selection.
    pub fn open_spacing_dialog(&mut self, ctx: &Context) {
        let Some(info) = self.format_info(ctx) else {
            return;
        };
        let sp = self.format_spacing_info(ctx).unwrap_or_default();
        let (exact, line) = match sp.line {
            kernpage_core::LineSpacing::Exactly(pt) => (true, pt),
            kernpage_core::LineSpacing::Multiple(m) => (false, m),
            kernpage_core::LineSpacing::Unknown(_) => (false, 1.0),
        };
        self.spacing_dialog = Some(SpacingDialog {
            range: info.range,
            before: sp.space_before_pt,
            after: sp.space_after_pt,
            exact,
            line,
        });
    }

    fn spacing_dialog_ui(&mut self, ctx: &Context) {
        let Some(d) = &mut self.spacing_dialog else {
            return;
        };
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new("Line Spacing Options")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("spacing_grid").show(ui, |ui| {
                    ui.label("Space before:");
                    ui.add(
                        egui::DragValue::new(&mut d.before)
                            .range(0.0..=1584.0)
                            .suffix(" pt"),
                    );
                    ui.end_row();
                    ui.label("Space after:");
                    ui.add(
                        egui::DragValue::new(&mut d.after)
                            .range(0.0..=1584.0)
                            .suffix(" pt"),
                    );
                    ui.end_row();
                    ui.label("Line spacing:");
                    ui.horizontal(|ui| {
                        let was = d.exact;
                        ui.selectable_value(&mut d.exact, false, "Lines");
                        ui.selectable_value(&mut d.exact, true, "Exactly");
                        if was != d.exact {
                            d.line = if d.exact { 12.0 } else { 1.0 };
                        }
                    });
                    ui.end_row();
                    ui.label(if d.exact { "Points:" } else { "Lines:" });
                    if d.exact {
                        ui.add(
                            egui::DragValue::new(&mut d.line)
                                .range(1.0..=1584.0)
                                .suffix(" pt"),
                        );
                    } else {
                        ui.add(
                            egui::DragValue::new(&mut d.line)
                                .range(0.1..=10.0)
                                .speed(0.05),
                        );
                    }
                    ui.end_row();
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ok = ui.button("  OK  ").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            cancel = true;
        }
        if ok {
            if let Some(d) = self.spacing_dialog.take() {
                let line = if d.exact {
                    kernpage_core::LineSpacing::Exactly(d.line)
                } else {
                    kernpage_core::LineSpacing::Multiple(d.line)
                };
                self.format_spacing(
                    ctx,
                    Some(d.range),
                    kernpage_core::text::SpacingChange {
                        before_pt: Some(d.before),
                        after_pt: Some(d.after),
                        line: Some(line),
                    },
                );
            }
        } else if cancel {
            self.spacing_dialog = None;
        }
    }

    /// Give the keyboard back to the in-place editor after a ribbon click.
    fn refocus_edit(&mut self) {
        if let Some(e) = &mut self.edit {
            e.fresh = true;
        }
    }

    /// Group needs two or more selected objects that are not in a group yet.
    pub fn can_group(&self) -> bool {
        let ids = self.selected_ids();
        let Some(pg) = self
            .editor
            .as_ref()
            .and_then(|e| e.doc().pages.get(self.page))
        else {
            return false;
        };
        ids.len() >= 2
            && ids
                .iter()
                .all(|i| pg.shape(*i).is_some_and(|s| s.group.is_none()))
    }

    /// Ungroup needs a group selected.
    pub fn can_ungroup(&self) -> bool {
        self.selected()
            .is_some_and(|s| matches!(s.kind, ShapeKind::Group))
    }

    /// Arrange > Group (Ctrl+Shift+G): the new group ends up selected.
    pub fn group_selection(&mut self) {
        self.commit_edit();
        if !self.can_group() {
            return;
        }
        let ids = self.selected_ids();
        let created = self.exec(Command::Group {
            page: self.page,
            ids,
        });
        if let Some(g) = created.first() {
            self.set_selection(vec![*g]);
        }
    }

    /// Arrange > Ungroup (Ctrl+Shift+H): the former members end up selected.
    pub fn ungroup_selection(&mut self) {
        self.commit_edit();
        let Some(id) = self
            .selected()
            .filter(|s| matches!(s.kind, ShapeKind::Group))
            .map(|s| s.id)
        else {
            return;
        };
        let members: Vec<ShapeId> = self
            .editor
            .as_ref()
            .and_then(|e| e.doc().pages.get(self.page))
            .map(|p| {
                p.shapes
                    .iter()
                    .filter(|s| s.group == Some(id))
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        self.exec(Command::Ungroup {
            page: self.page,
            id,
        });
        self.set_selection(members);
    }

    pub fn copy_selection(&mut self) {
        let ids = self.selected_ids();
        let Some(ed) = &self.editor else { return };
        let Some(pg) = ed.doc().pages.get(self.page) else {
            return;
        };
        // A copied group brings its members; pasting makes them ordinary objects.
        let ids = pg.with_descendants(&ids);
        // Keep stacking order so a paste restores it.
        let shapes: Vec<Shape> = pg
            .shapes
            .iter()
            .filter(|s| ids.contains(&s.id))
            .cloned()
            .collect();
        if !shapes.is_empty() {
            // A linked frame is copied without its text and without its links: the copy
            // is a new empty text box (Publisher 2021).
            self.clipboard_blank.clear();
            let doc = ed.doc();
            let shapes: Vec<Shape> = shapes
                .into_iter()
                .map(|mut s| {
                    if s.is_text_box() && doc.story_of(s.id).is_some() {
                        s.kind = ShapeKind::TextBox {
                            paragraphs: vec![kernpage_core::story::empty_like(&s.kind)],
                        };
                        self.clipboard_blank.push(s.id);
                    }
                    s
                })
                .collect();
            self.clipboard = shapes;
            self.paste_count = 0;
        }
    }

    pub fn paste(&mut self) {
        if self.clipboard.is_empty() || self.editor.is_none() {
            return;
        }
        self.paste_count += 1;
        let off = in_to_emu(0.25) * self.paste_count;
        // Publisher pastes a copy of a linked frame 9 pt right and down of the original.
        let off_linked = kernpage_core::geometry::pt_to_emu(9.0) * self.paste_count;
        let shapes: Vec<Shape> = self
            .clipboard
            .iter()
            .map(|s| {
                let mut c = s.clone();
                let d = if self.clipboard_blank.contains(&s.id) { off_linked } else { off };
                c.rect = c.rect.translated(d, d);
                c
            })
            .collect();
        let ids = self.exec(Command::PasteShapes {
            page: self.page,
            shapes,
        });
        self.set_selection(ids);
    }

    pub fn delete_selection(&mut self) {
        let ids = self.selected_ids();
        self.selection = None;
        self.multi.clear();
        if !ids.is_empty() {
            self.exec(Command::DeleteShapes {
                page: self.page,
                ids,
            });
        }
    }

    pub fn undo(&mut self) {
        self.commit_edit();
        if let Some(ed) = &mut self.editor {
            ed.undo();
        }
    }

    pub fn redo(&mut self) {
        self.commit_edit();
        if let Some(ed) = &mut self.editor {
            ed.redo();
        }
    }

    pub fn insert_page(&mut self) {
        if self.editor.is_none() {
            return;
        }
        self.exec(Command::InsertPage { after: self.page });
        self.go_page(self.page + 1);
    }

    pub fn delete_page(&mut self) {
        if self.page_count() > 1 {
            let page = self.page;
            self.exec(Command::DeletePage { page });
            self.go_page(page.min(self.page_count().saturating_sub(1)));
        }
    }

    /// Insert a blank page after page `page` (Pages pane > Insert Blank Page) and show it.
    pub fn insert_page_after(&mut self, page: usize) {
        if self.editor.is_none() || page >= self.page_count() {
            return;
        }
        self.exec(Command::InsertPage { after: page });
        self.go_page(page + 1);
    }

    /// Insert a copy of page `page` after it (Pages pane > Insert Duplicate Page) and show it.
    pub fn duplicate_page_at(&mut self, page: usize) {
        if self.editor.is_none() || page >= self.page_count() {
            return;
        }
        self.exec(Command::DuplicatePage { page });
        self.go_page(page + 1);
    }

    /// Delete page `page` (Pages pane > Delete Page); the last page cannot be deleted.
    pub fn delete_page_at(&mut self, page: usize) {
        if self.page_count() > 1 && page < self.page_count() {
            self.exec(Command::DeletePage { page });
            let cur = if self.page > page { self.page - 1 } else { self.page };
            self.go_page(cur.min(self.page_count().saturating_sub(1)));
        }
    }

    /// Move page `page` one place up (`-1`) or down (`1`) and show it there.
    pub fn move_page_by(&mut self, page: usize, delta: isize) {
        let to = page as isize + delta;
        if to < 0 || to as usize >= self.page_count() {
            return;
        }
        self.exec(Command::MovePage {
            from: page,
            to: to as usize,
        });
        self.go_page(to as usize);
    }

    pub fn go_page(&mut self, page: usize) {
        let page = page.min(self.page_count().saturating_sub(1));
        if page != self.page {
            self.commit_edit();
            self.page = page;
            self.selection = None;
            self.multi.clear();
            self.reset_scroll = true;
        }
    }

    pub fn toggle_100_whole(&mut self) {
        if self.zoom_mode == ZoomMode::Custom && (self.zoom - 1.0).abs() < 1e-6 {
            self.zoom_mode = ZoomMode::WholePage;
        } else {
            self.zoom_mode = ZoomMode::Custom;
            self.zoom = 1.0;
        }
        self.reset_scroll = true;
    }

    // ---- per-frame ---------------------------------------------------------------

    fn begin_frame(&mut self, ctx: &Context) {
        self.ctx = Some(ctx.clone());
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.editor.is_some() {
                self.commit_edit();
            }
            if self.is_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if self.prompt.is_none() {
                    self.prompt = Some(Prompt::Unsaved {
                        then: Pending::Exit,
                    });
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && (self.allow_close || !self.is_dirty()) {
            // A normal exit: the snapshot is deleted; only a dying process leaves one.
            self.ar.shutdown();
        }
        if let Some(ed) = &self.editor {
            if self.fonts_rev != Some(ed.revision()) {
                self.fonts_rev = Some(ed.revision());
                self.fonts_needed = ed.doc().fonts_used();
                for f in [("Calibri", false, false), ("Calibri", true, false)] {
                    self.fonts_needed.push((f.0.to_owned(), f.1, f.2));
                }
            }
            // Keep the selection, page and editor valid after undo, redo or page changes.
            let pages = ed.doc().pages.len();
            if self.page >= pages {
                self.page = pages.saturating_sub(1);
            }
            if let Some(id) = self.selection {
                if ed.doc().shape(self.page, id).is_none() {
                    self.selection = None;
                }
            }
            // Any other change of `selection` drops the multi-selection; so does the
            // loss of shapes (undo, page change).
            if !self.multi.is_empty() {
                let doc = ed.doc();
                self.multi.retain(|&i| doc.shape(self.page, i).is_some());
                if self.multi.len() < 2 || self.selection.is_none_or(|s| !self.multi.contains(&s)) {
                    self.multi.clear();
                }
            }
            if let Some(e) = &self.edit {
                if ed.doc().shape(e.page, e.id).is_none() || e.page != self.page {
                    self.edit = None;
                }
            }
        }
        self.fonts.ensure(ctx, &self.fonts_needed);
        let dirty = self.is_dirty();
        self.autorecover_frame(ctx, dirty);
        let title = match &self.editor {
            Some(e) if self.recovered => {
                format!("{} (Recovered) - KernPage", e.doc().name)
            }
            Some(e) => format!(
                "{}{} - KernPage",
                e.doc().name,
                if dirty { "*" } else { "" }
            ),
            None => "KernPage".to_owned(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    /// Subscript / Superscript toggle: turning one on turns the other off (one value).
    pub fn toggle_script(&mut self, ctx: &Context, want: Script) {
        let Some(info) = self.format_info(ctx) else {
            return;
        };
        let new = if info.fmt.script == Some(want) { Script::Normal } else { want };
        self.format_chars(ctx, CharChange::Script(new));
    }

    /// Ctrl+B / I / U and Ctrl+L / E / R / J.
    fn format_shortcut(&mut self, ctx: &Context, name: &str) {
        let Some(info) = self.format_info(ctx) else {
            return;
        };
        let f = &info.fmt;
        match name {
            "bold" => self.format_chars(ctx, CharChange::Bold(f.bold != Some(true))),
            "italic" => self.format_chars(ctx, CharChange::Italic(f.italic != Some(true))),
            "underline" => self.format_chars(ctx, CharChange::Underline(f.underline != Some(true))),
            "superscript" => self.toggle_script(ctx, Script::Superscript),
            "subscript" => self.toggle_script(ctx, Script::Subscript),
            "left" => self.format_align(ctx, Align::Left),
            "center" => self.format_align(ctx, Align::Center),
            "right" => self.format_align(ctx, Align::Right),
            _ => self.format_align(ctx, Align::Justify),
        }
    }

    fn shortcuts(&mut self, ctx: &Context) {
        // A modal dialog owns the keyboard (Esc closes the Line Spacing Options dialog).
        if self.error.is_some()
            || self.prompt.is_some()
            || self.spacing_dialog.is_some()
            || self.page_dialog.is_some()
            || self.table_dialog.is_some()
            || self.list_dialog.is_some()
        {
            return;
        }
        let editing = self.edit.is_some() || ctx.egui_wants_keyboard_input();
        let (copy, cut, paste) = ctx.input(|i| {
            (
                i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
            )
        });
        let cell_untouched = self.edit.as_ref().is_some_and(|e| e.cell.is_some())
            && self.edit_command().is_none();
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let mut keys = ctx.input_mut(|i| {
            let mut k = Vec::new();
            for (name, m, key) in [
                ("insert_page", ctrl_shift, Key::N),
                ("save_as", ctrl_shift, Key::S),
                ("save", ctrl, Key::S),
                ("redo", ctrl_shift, Key::Z),
                ("new", ctrl, Key::N),
                ("open", ctrl, Key::O),
                ("print", ctrl, Key::P),
                ("undo", ctrl, Key::Z),
                ("redo", ctrl, Key::Y),
            ] {
                // Undo and redo while typing belong to the text editor: leave the key unconsumed.
                // A table cell with nothing typed since it was entered has nothing to undo
                // itself, so the key undoes the document (an inserted row, say).
                if editing && !cell_untouched && matches!(name, "undo" | "redo") {
                    continue;
                }
                if i.consume_key(m, key) {
                    k.push(name);
                }
            }
            if i.consume_key(Modifiers::NONE, Key::F9) {
                k.push("f9");
            }
            if i.consume_key(Modifiers::NONE, Key::PageUp) {
                k.push("pgup");
            }
            if i.consume_key(Modifiers::NONE, Key::PageDown) {
                k.push("pgdn");
            }
            k
        });
        if self.editor.is_some() && !self.backstage {
            let fk = ctx.input_mut(|i| {
                let mut k = Vec::new();
                // Ctrl+Shift+= (arrives as Plus on some layouts) before Ctrl+=, which would
                // also match it.
                if i.consume_key(ctrl_shift, Key::Equals) || i.consume_key(ctrl_shift, Key::Plus) {
                    k.push("superscript");
                } else if i.consume_key(ctrl, Key::Equals) {
                    k.push("subscript");
                }
                for (name, key) in [
                    ("bold", Key::B),
                    ("italic", Key::I),
                    ("underline", Key::U),
                    ("left", Key::L),
                    ("center", Key::E),
                    ("right", Key::R),
                    ("justify", Key::J),
                ] {
                    if i.consume_key(ctrl, key) {
                        k.push(name);
                    }
                }
                k
            });
            for k in fk {
                self.format_shortcut(ctx, k);
            }
        }
        if editing {
            // While typing, only the document-level shortcuts that are not text editing.
            keys.retain(|k| {
                matches!(
                    *k,
                    "new" | "open" | "f9" | "insert_page" | "save" | "save_as" | "print"
                ) || (cell_untouched && matches!(*k, "undo" | "redo"))
            });
        }
        for k in keys {
            match k {
                "new" => {
                    self.backstage = true;
                    self.section = Section::New;
                }
                "open" => self.open_dialog(),
                "print" => {
                    if self.editor.is_some() {
                        self.commit_edit();
                        self.backstage = true;
                        self.section = Section::Print;
                    }
                }
                "save" => self.save(None),
                "save_as" => self.save_as(None),
                "undo" => self.undo(),
                "redo" => self.redo(),
                "insert_page" => self.insert_page(),
                "f9" => self.toggle_100_whole(),
                "pgup" => self.go_page(self.page.saturating_sub(1)),
                "pgdn" => self.go_page(self.page + 1),
                _ => {}
            }
        }
        // Esc in the File view goes back to the publication (like the back arrow).
        if self.backstage
            && self.editor.is_some()
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.backstage = false;
        }
        if editing || self.editor.is_none() || self.backstage {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::A)) {
            let ids: Vec<ShapeId> = self
                .editor
                .as_ref()
                .and_then(|e| e.doc().pages.get(self.page))
                .map(|p| {
                    p.shapes
                        .iter()
                        .filter(|s| s.group.is_none())
                        .map(|s| s.id)
                        .collect()
                })
                .unwrap_or_default();
            self.set_selection(ids);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::G)) {
            self.group_selection();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::H)) {
            self.ungroup_selection();
        }
        if copy {
            self.copy_selection();
        }
        if cut {
            self.copy_selection();
            self.delete_selection();
        }
        if paste {
            self.paste();
        }
        // Esc belongs to an open drop-down gallery or menu (it closes it), not to the canvas.
        // The same goes for the search box (egui drops its focus on Esc before we look).
        let popup_open = egui::Popup::is_any_open(ctx) || !self.search.text.is_empty();
        let (del, esc, dx, dy, big) = ctx.input_mut(|i| {
            let del = i.consume_key(Modifiers::NONE, Key::Delete)
                || i.consume_key(Modifiers::NONE, Key::Backspace);
            let esc = !popup_open && i.consume_key(Modifiers::NONE, Key::Escape);
            let big = i.modifiers.shift;
            let m = if big {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            };
            let mut dx = 0;
            let mut dy = 0;
            if i.consume_key(m, Key::ArrowLeft) {
                dx -= 1;
            }
            if i.consume_key(m, Key::ArrowRight) {
                dx += 1;
            }
            if i.consume_key(m, Key::ArrowUp) {
                dy -= 1;
            }
            if i.consume_key(m, Key::ArrowDown) {
                dy += 1;
            }
            (del, esc, dx, dy, big)
        });
        if del {
            self.delete_selection();
        }
        if esc {
            self.tool = Tool::Select;
            self.selection = None;
        }
        let ids = self.selected_ids();
        if (dx != 0 || dy != 0) && !ids.is_empty() {
            let step = if big { in_to_emu(0.1) } else { in_to_emu(0.02) };
            self.exec(Command::MoveShapes {
                page: self.page,
                ids,
                dx: dx * step,
                dy: dy * step,
            });
        }
    }

    fn error_dialog(&mut self, ctx: &Context) {
        let Some(msg) = self.error.clone() else {
            return;
        };
        let mut close = false;
        #[allow(deprecated)]
        let modal_id = egui::Id::new("error_dialog");
        egui::Modal::new(modal_id).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading("KernPage");
            ui.add_space(8.0);
            ui.label(msg);
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.button("  OK  ").clicked() {
                    close = true;
                }
            });
        });
        if close || ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) {
            self.error = None;
        }
    }

    fn screenshot_step(&mut self, ctx: &Context) {
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        ctx.request_repaint();
        let ready = self.fonts.scan_done() && self.fonts.settled() && shot.frames >= 8;
        if shot.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if shot.failed.load(Ordering::SeqCst) || shot.started.elapsed().as_secs() > 40 {
            eprintln!("screenshot failed");
            std::process::exit(3);
        }
        if ready && !shot.requested && self.error.is_none() {
            shot.requested = true;
            let path = shot.path.clone();
            let (done, failed) = (shot.done.clone(), shot.failed.clone());
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let mut rgba = Vec::with_capacity(w * h * 4);
                for p in &img.pixels {
                    rgba.extend_from_slice(&p.to_array());
                }
                let saved = (|| -> Result<(), Box<dyn std::error::Error>> {
                    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                        std::fs::create_dir_all(dir)?;
                    }
                    let file = std::io::BufWriter::new(std::fs::File::create(&path)?);
                    let mut enc = png::Encoder::new(file, w as u32, h as u32);
                    enc.set_color(png::ColorType::Rgba);
                    enc.set_depth(png::BitDepth::Eight);
                    enc.write_header()?.write_image_data(&rgba)?;
                    Ok(())
                })();
                match saved {
                    Ok(()) => done.store(true, Ordering::SeqCst),
                    Err(e) => {
                        eprintln!("cannot save screenshot: {e}");
                        failed.store(true, Ordering::SeqCst);
                    }
                }
            });
        }
    }

    /// Make the chosen appearance the current palette and push it into egui's visuals.
    pub fn apply_theme(&mut self, ctx: &Context) {
        let sys = ctx.system_theme().map(|t| t == egui::Theme::Dark);
        let pal = self.appearance.palette(sys);
        crate::theme::set(pal);
        if self.applied_palette == Some(pal) {
            return;
        }
        self.applied_palette = Some(pal);
        let mut v = if pal.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        v.selection.bg_fill = pal.text_selection;
        v.selection.stroke = egui::Stroke::new(1.0, pal.accent);
        v.hyperlink_color = pal.accent_text;
        v.panel_fill = pal.window_bg;
        v.window_fill = pal.card;
        v.extreme_bg_color = pal.card;
        v.faint_bg_color = pal.hover_bg;
        v.window_stroke = egui::Stroke::new(1.0, pal.line);
        if pal.dark {
            v.widgets.noninteractive.bg_fill = pal.card;
            v.widgets.inactive.bg_fill = pal.hover_bg;
            v.widgets.inactive.weak_bg_fill = pal.hover_bg;
            v.widgets.hovered.bg_fill = pal.selected_bg;
            v.widgets.hovered.weak_bg_fill = pal.selected_bg;
            v.widgets.active.bg_fill = pal.hover_pressed;
            v.widgets.active.weak_bg_fill = pal.hover_pressed;
        }
        v.widgets.noninteractive.fg_stroke.color = pal.text;
        v.widgets.inactive.fg_stroke.color = pal.text;
        v.widgets.hovered.fg_stroke.color = pal.text;
        v.widgets.active.fg_stroke.color = pal.text;
        ctx.set_visuals(v);
    }

    /// Change the appearance, apply it now and save it.
    pub fn set_appearance(&mut self, a: crate::theme::Appearance, ctx: &Context) {
        self.appearance = a;
        self.apply_theme(ctx);
        if self.ar.active() {
            a.save_to(&crate::theme::settings_file());
        }
    }

    /// The Help button: open File > About KernPage (there is no online help).
    pub fn show_about(&mut self) {
        self.commit_edit();
        self.backstage = true;
        self.section = Section::About;
    }

    // ---- top bar -----------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        egui::Panel::top("top")
            .frame(egui::Frame::new().fill(crate::theme::window_bg()))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                // Quick-access toolbar and title.
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.set_height(26.0);
                    if crate::widgets::Btn::small(Icon::Save, "")
                        .enabled(self.can_save())
                        .tip(self.save_tip())
                        .show(ui)
                        .clicked()
                    {
                        self.save(None);
                    }
                    let (can_undo, can_redo, ul, rl) = match &self.editor {
                        Some(e) => (e.can_undo(), e.can_redo(), e.undo_label(), e.redo_label()),
                        None => (false, false, None, None),
                    };
                    for (is_undo, can, label) in [(true, can_undo, ul), (false, can_redo, rl)] {
                        let (icon, name) = if is_undo {
                            (Icon::Undo, "Undo")
                        } else {
                            (Icon::Redo, "Redo")
                        };
                        let tip = label
                            .clone()
                            .map_or(name.to_owned(), |l| format!("{name} {l}"));
                        let (main, arrow) = crate::widgets::split_button(
                            ui,
                            crate::widgets::Btn::small(icon, "").enabled(can).tip(&tip),
                        );
                        if main.clicked() {
                            if is_undo {
                                self.undo();
                            } else {
                                self.redo();
                            }
                        }
                        let mut run = false;
                        egui::Popup::menu(&arrow).show(|ui| {
                            ui.set_min_width(140.0);
                            if ui.button(&tip).clicked() {
                                run = true;
                                ui.close();
                            }
                        });
                        if run {
                            if is_undo {
                                self.undo();
                            } else {
                                self.redo();
                            }
                        }
                    }
                    let title = self.title.clone();
                    let rect = ui.available_rect_before_wrap();
                    let box_w = 300.0_f32.min((ui.clip_rect().width() - 400.0).max(120.0));
                    let search_rect = egui::Rect::from_center_size(
                        egui::pos2(ui.clip_rect().center().x, rect.center().y),
                        egui::vec2(box_w, 22.0),
                    );
                    // The document title sits between the quick-access buttons and the box.
                    let title_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.min.x + 10.0, rect.min.y),
                        egui::pos2(search_rect.min.x - 10.0, rect.max.y),
                    );
                    if title_rect.width() > 20.0 {
                        ui.painter().with_clip_rect(title_rect).text(
                            egui::pos2(title_rect.min.x, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            title,
                            egui::FontId::proportional(12.0),
                            crate::theme::group_label(),
                        );
                    }
                    self.search_box(ui, search_rect);
                });
                // Tab strip. The File screen replaces the tab row and the ribbon until it is left.
                if !self.backstage {
                ui.horizontal(|ui| {
                    ui.add_space(0.0);
                    ui.set_height(TAB_H);
                    let file_resp = {
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(48.0, TAB_H), egui::Sense::click());
                        crate::theme::drew!("file:tab");
                        // A normal tab-styled entry: a soft pill while hovered or open.
                        let pill = crate::widgets::snap_rect(
                            ui.ctx().pixels_per_point(),
                            rect.shrink2(egui::vec2(2.0, 3.0)),
                        );
                        if self.backstage {
                            ui.painter().rect_filled(pill, 6.0, crate::theme::accent_tint());
                        } else if resp.hovered() {
                            ui.painter().rect_filled(pill, 6.0, crate::theme::hover_bg());
                        }
                        let _ink = crate::widgets::centered_label(
                            ui,
                            pill,
                            "File",
                            egui::FontId::proportional(13.3),
                            if self.backstage { crate::theme::accent_text() } else { crate::theme::text() },
                        );
                        #[cfg(test)]
                        for (l, r) in [("tabpill:File", pill), ("tabink:File", _ink)] {
                            crate::widgets::probe::record(crate::widgets::probe::Seen {
                                label: l.to_owned(),
                                tip: String::new(),
                                rect: r,
                                enabled: true,
                                selected: false,
                            });
                        }
                        #[cfg(test)]
                        crate::widgets::probe::record(crate::widgets::probe::Seen {
                            label: "tab:File".to_owned(),
                            tip: String::new(),
                            rect,
                            enabled: true,
                            selected: self.backstage,
                        });
                        resp
                    };
                    if file_resp.clicked() {
                        self.backstage = !self.backstage || !has_doc;
                        self.section = if has_doc {
                            Section::Info
                        } else {
                            Section::Home
                        };
                    }
                    ui.add_space(4.0);
                    let ctx_tabs = if has_doc {
                        self.context_tabs()
                    } else {
                        Vec::new()
                    };
                    if self.tab.is_contextual() && !ctx_tabs.contains(&self.tab) {
                        // The selection went away (or changed kind): fall back to Home.
                        self.tab = Tab::Home;
                    }
                    // The Help and Minimize buttons keep their room on the right. When the tabs
                    // (two more with a text box selected) do not fit beside them, the padding
                    // inside each tab shrinks first, then the gaps between tabs.
                    let all_tabs: Vec<Tab> = Tab::ALL.into_iter().chain(ctx_tabs.iter().copied()).collect();
                    let text_w: f32 = all_tabs
                        .iter()
                        .map(|t| {
                            ui.painter()
                                .layout_no_wrap(
                                    t.name().to_owned(),
                                    egui::FontId::proportional(13.3),
                                    crate::theme::text(),
                                )
                                .size()
                                .x
                        })
                        .sum();
                    let gap = ui.spacing().item_spacing.x;
                    let ctx_gap = if ctx_tabs.is_empty() { 0.0 } else { 10.0 + gap };
                    let budget = ui.max_rect().right() - TAB_STRIP_RESERVED - ui.cursor().min.x - ctx_gap;
                    let slack = (budget - text_w) / all_tabs.len().max(1) as f32;
                    let (tab_pad, tab_gap) = if slack >= 23.6 + gap {
                        (23.6, gap)
                    } else if slack - gap >= 6.0 {
                        (slack - gap, gap)
                    } else {
                        (slack.max(2.0), 0.0)
                    };
                    ui.spacing_mut().item_spacing.x = tab_gap;
                    for t in all_tabs.iter().copied() {
                        if t == ctx_tabs.first().copied().unwrap_or(Tab::Home) && t.is_contextual()
                        {
                            ui.add_space(10.0);
                        }
                        let selected = self.tab == t && !self.backstage && has_doc;
                        let w = ui
                            .painter()
                            .layout_no_wrap(
                                t.name().to_owned(),
                                egui::FontId::proportional(13.3),
                                crate::theme::text(),
                            )
                            .size()
                            .x
                            + tab_pad;
                        let sense = if has_doc {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        };
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, TAB_H), sense);
                        if selected {
                            crate::theme::drew!("tab:pill");
                        }
                        let pill = crate::widgets::snap_rect(
                            ui.ctx().pixels_per_point(),
                            rect.shrink2(egui::vec2(3.0, 3.0)),
                        );
                        if selected {
                            // A rounded pill in the accent tint.
                            ui.painter().rect_filled(pill, 7.0, crate::theme::accent_tint());
                        } else if resp.hovered() && has_doc {
                            ui.painter().rect_filled(pill, 7.0, crate::theme::hover_bg());
                        }
                        let col = if t.is_contextual() || selected {
                            crate::theme::accent_text()
                        } else if has_doc {
                            crate::theme::text()
                        } else {
                            crate::theme::disabled()
                        };
                        let font = egui::FontId::proportional(13.3);
                        let _ink = crate::widgets::centered_label(ui, pill, t.name(), font, col);
                        #[cfg(test)]
                        for (l, r) in [("tabpill", pill), ("tabink", _ink)] {
                            crate::widgets::probe::record(crate::widgets::probe::Seen {
                                label: format!("{l}:{}", t.name()),
                                tip: String::new(),
                                rect: r,
                                enabled: true,
                                selected,
                            });
                        }
                        #[cfg(test)]
                        crate::widgets::probe::record(crate::widgets::probe::Seen {
                            label: format!("tab:{}", t.name()),
                            tip: String::new(),
                            rect,
                            enabled: has_doc,
                            selected,
                        });
                        if resp.clicked() {
                            // The caret stays in a table cell: the Layout tab acts on it.
                            if self.edit.as_ref().is_some_and(|e| e.cell.is_some()) {
                                self.refocus_edit();
                            } else {
                                self.commit_edit();
                            }
                            self.tab = t;
                            self.backstage = false;
                        }
                    }
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        if crate::widgets::Btn::small(Icon::Question, "")
                            .tip("Help and About")
                            .show(ui)
                            .clicked()
                        {
                            self.show_about();
                        }
                        let chev = minimize_icon(self.ribbon_collapsed);
                        let tip = if self.ribbon_collapsed {
                            "Expand the Ribbon"
                        } else {
                            "Minimize the Ribbon"
                        };
                        let r = crate::widgets::Btn::small(chev, "")
                            .enabled(has_doc)
                            .selected(self.ribbon_collapsed)
                            .tip(tip)
                            .show(ui);
                        if r.clicked() {
                            self.ribbon_collapsed = !self.ribbon_collapsed;
                        }
                    });
                });
                }
                if has_doc && !self.backstage && !self.ribbon_collapsed {
                    crate::theme::drew!("ribbon:card");
                    // The ribbon floats as a rounded panel on the tinted window. The panel is
                    // painted by hand, on whole device pixels, so its edge and the cards inside
                    // it are crisp; its inner margin is the same GROUP_GAP as between the cards.
                    let ppp = ui.ctx().pixels_per_point();
                    let bg = ui.painter().add(egui::Shape::Noop);
                    crate::widgets::begin_cards();
                    let margin = ((crate::widgets::gap_px(ppp) + 1.0) / ppp).ceil() as i8;
                    let resp = egui::Frame::new()
                        .inner_margin(egui::Margin::same(margin))
                        .outer_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 6 })
                        .show(ui, |ui| {
                            // The panel can be wider than the window (a long tab row), so
                            // measure against the window.
                            let avail = ui
                                .available_width()
                                .min(ui.ctx().content_rect().right() - ui.cursor().min.x - 8.0);
                            egui::ScrollArea::horizontal()
                                .id_salt("ribbon_scroll")
                                .auto_shrink([false, true])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                )
                                .show(ui, |ui| self.ribbon(ui, avail));
                        });
                    // The response includes the outer margin (6 / 2 / 6 / 6 px). The panel hugs the
                    // cards: from a card's outline to the panel's border line there are exactly
                    // as many white pixels as between two cards, on the left, top and bottom.
                    let full = resp.response.rect;
                    let mut panel = crate::widgets::snap_rect(
                        ppp,
                        egui::Rect::from_min_max(
                            full.min + egui::vec2(6.0, 2.0),
                            full.max - egui::vec2(6.0, 6.0),
                        ),
                    );
                    if let Some(c) = crate::widgets::cards_bounds() {
                        let m = (crate::widgets::gap_px(ppp) + 1.0) / ppp;
                        panel.min = c.min - egui::vec2(m, m);
                        panel.max.y = c.max.y + m;
                    }
                    #[cfg(test)]
                    crate::widgets::probe::record(crate::widgets::probe::Seen {
                        label: "grouppanel".to_owned(),
                        tip: String::new(),
                        rect: panel,
                        enabled: true,
                        selected: false,
                    });
                    let radius = crate::widgets::PANEL_RADIUS;
                    let shadow = egui::Shadow {
                        offset: [0, 1],
                        blur: 6,
                        spread: 0,
                        color: crate::theme::shadow(),
                    };
                    ui.painter().set(
                        bg,
                        egui::Shape::Vec(vec![
                            shadow.as_shape(panel, radius).into(),
                            egui::epaint::RectShape::new(
                                panel,
                                radius,
                                crate::theme::card(),
                                egui::Stroke::new(1.0 / ppp, crate::theme::line()),
                                egui::StrokeKind::Inside,
                            )
                            .into(),
                        ]),
                    );
                }
            });
    }
}

/// The Minimize the Ribbon button's arrow: up while the ribbon is open (click to collapse),
/// down once it is collapsed (click to expand).
fn minimize_icon(collapsed: bool) -> Icon {
    if collapsed {
        Icon::ChevronDown
    } else {
        Icon::Chevron
    }
}

#[cfg(test)]
mod minimize_icon_tests {
    use super::*;

    #[test]
    fn the_chevron_points_up_when_expanded_and_down_when_collapsed() {
        assert_eq!(minimize_icon(false), Icon::Chevron);
        assert_eq!(minimize_icon(true), Icon::ChevronDown);
    }
}

/// When egui should take the keyboard focus away from the focused widget after a click
/// elsewhere. While a text box is being edited the ribbon, its drop-downs and popups must
/// not take the focus (or the selection) from the text, as in Publisher; the canvas ends
/// the editing session itself when the page is clicked. Otherwise egui's default applies.
pub fn surrender_policy(editing: bool) -> egui::SurrenderFocusOn {
    if editing {
        egui::SurrenderFocusOn::Never
    } else {
        egui::SurrenderFocusOn::Clicks
    }
}

/// Diagnostic for idle CPU (pw-9iee): with the environment variable KP_FRAME_LOG set, append
/// one line per second to `kernpage-frames.log` in the temp folder with the number of frames
/// drawn and the code that last asked for a repaint. An idle window shows 0 to 2 frames.
fn frame_log(ctx: &Context) {
    use std::cell::RefCell;
    thread_local! {
        static STATE: RefCell<(Option<bool>, Instant, u32)> = RefCell::new((None, Instant::now(), 0));
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let on = *s.0.get_or_insert_with(|| std::env::var_os("KP_FRAME_LOG").is_some());
        if !on {
            return;
        }
        s.2 += 1;
        if s.1.elapsed() >= Duration::from_secs(1) {
            let causes = ctx.repaint_causes();
            let line = format!("frames={} causes={:?}
", s.2, causes.iter().rev().take(3).collect::<Vec<_>>());
            let path = std::env::temp_dir().join("kernpage-frames.log");
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                use std::io::Write;
                let _ = f.write_all(line.as_bytes());
            }
            *s = (Some(true), Instant::now(), 0);
        }
    });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        #[cfg(test)]
        crate::widgets::probe::clear();
        #[cfg(test)]
        crate::page_setup::probe::clear();
        #[cfg(test)]
        crate::nav::probe::clear();
        #[cfg(test)]
        crate::backstage::spin_probe::clear();
        #[cfg(test)]
        crate::theme::probe::clear();
        #[cfg(test)]
        crate::lists_ui::probe::clear();
        let ctx = ui.ctx().clone();
        frame_log(&ctx);
        self.apply_theme(&ctx);
        self.screen_dpi = f64::from(ctx.native_pixels_per_point().unwrap_or(1.0)) * 96.0;
        let policy = surrender_policy(self.edit.is_some());
        ctx.options_mut(|o| o.input_options.surrender_focus_on = policy);
        self.begin_frame(&ctx);
        self.shortcuts(&ctx);
        self.top_bar(ui);
        self.update.bar(ui);
        if self.backstage || self.editor.is_none() {
            self.backstage_view(ui);
        } else {
            self.status_bar(ui);
            if self.show.nav {
                self.nav_pane(ui);
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(crate::theme::workspace()))
                .show(ui, |ui| self.canvas(ui));
        }
        self.poll_print(&ctx);
        self.error_dialog(&ctx);
        self.prompts(&ctx);
        self.spacing_dialog_ui(&ctx);
        self.page_dialog_ui(&ctx);
        self.table_dialog_ui(&ctx);
        self.list_dialog_ui(&ctx);
        self.screenshot_step(&ctx);
    }
}

#[cfg(test)]
mod context_tab_tests {
    use super::*;
    use ObjKind::*;

    #[test]
    fn ribbon_clicks_keep_text_focus_while_editing() {
        assert_eq!(surrender_policy(true), egui::SurrenderFocusOn::Never);
        assert_eq!(surrender_policy(false), egui::SurrenderFocusOn::Clicks);
    }

    #[test]
    fn contextual_tabs_follow_the_selection() {
        assert!(context_tabs(&[]).is_empty());
        assert_eq!(context_tabs(&[TextBox]), [Tab::ShapeFormat, Tab::TextBox]);
        assert_eq!(context_tabs(&[Shape]), [Tab::ShapeFormat]);
        assert_eq!(context_tabs(&[Picture]), [Tab::PictureFormat]);
        assert_eq!(context_tabs(&[Table]), [Tab::TableDesign, Tab::Layout]);
    }

    #[test]
    fn mixed_selections_keep_only_common_tabs() {
        assert_eq!(context_tabs(&[TextBox, Shape]), [Tab::ShapeFormat]);
        assert_eq!(
            context_tabs(&[TextBox, TextBox]),
            [Tab::ShapeFormat, Tab::TextBox]
        );
        assert!(context_tabs(&[Picture, Shape]).is_empty());
        assert!(context_tabs(&[Table, TextBox]).is_empty());
    }

    #[test]
    fn contextual_tabs_parse_by_name() {
        assert_eq!(Tab::parse("Shape Format"), Some(Tab::ShapeFormat));
        assert_eq!(Tab::parse("layout"), Some(Tab::Layout));
        assert!(Tab::Layout.is_contextual() && !Tab::Home.is_contextual());
    }
}

/// Largest publication file the editor will read into memory (security audit PW-SEC-007: a
/// multi-gigabyte file, or a device name, must not exhaust memory before any parsing starts).
pub const MAX_PUB_FILE_BYTES: u64 = 512 << 20;

/// Read and convert a publication file without touching the app (used off the UI thread for the
/// start screen's previews); the same steps as `App::open_path`.
pub fn load_document(path: &Path) -> Result<kernpage_core::Document, String> {
    let shown = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let bytes = read_publication_file(path).map_err(|e| e.to_string())?;
    if kernpage_core::native::is_kern(&bytes) {
        return read_kern_document(&shown, &bytes).map(|(doc, _)| doc);
    }
    let parsed = std::panic::catch_unwind(move || {
        pub_format::read_bytes(&bytes).map(|doc| {
            let breaks = pub2pdf::chain_breaks(&doc, &pub2pdf::Options::default());
            kernpage_core::from_pub_flowed(&shown, &doc, &breaks)
        })
    });
    match parsed {
        Ok(Ok(d)) => Ok(d),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("damaged file".to_owned()),
    }
}

/// Whether `path` names a Publisher file (by extension).
fn is_pub_path(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pub"))
}

/// A file name without a trailing .pub or .kern.
fn strip_native_ext(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in [".pub", ".kern"] {
        if lower.ends_with(ext) {
            return &name[..name.len() - ext.len()];
        }
    }
    name
}

/// Parse `.kern` bytes (never panics) and name the document after its file.
fn read_kern_document(shown: &str, bytes: &[u8]) -> Result<(kernpage_core::Document, String), String> {
    let owned = bytes.to_vec();
    match std::panic::catch_unwind(move || kernpage_core::native::read_kern_with_id(&owned)) {
        Ok(Ok((mut doc, id))) => {
            doc.name = shown.to_owned();
            Ok((doc, id))
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("The file is damaged.".to_owned()),
    }
}

/// Read a publication file, refusing anything that is not a regular file or is over the cap.
pub fn read_publication_file(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(std::io::Error::other("this is not a file"));
    }
    if meta.len() > MAX_PUB_FILE_BYTES {
        return Err(std::io::Error::other("the file is too large to be a publication"));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    // `take` guards against a file that grows while it is being read.
    file.take(MAX_PUB_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PUB_FILE_BYTES {
        return Err(std::io::Error::other("the file is too large to be a publication"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod read_limit_tests {
    use super::*;

    #[test]
    fn a_folder_is_not_read_as_a_publication() {
        assert!(read_publication_file(&std::env::temp_dir()).is_err());
    }

    #[test]
    fn a_small_file_is_read_whole() {
        let p = std::env::temp_dir().join(format!("pw-read-limit-{}.bin", std::process::id()));
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(read_publication_file(&p).unwrap(), b"abc");
        let _ = std::fs::remove_file(&p);
    }
}
