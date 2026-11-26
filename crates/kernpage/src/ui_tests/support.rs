//! Test harness: builds the real `App` inside an egui_kittest `Harness` and provides
//! mouse / keyboard helpers that send one input event per frame, like a real window.

use crate::app::{App, Options, Tool};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use kernpage_core::geometry::{emu_to_px, in_to_emu};
use kernpage_core::{Document, Page, Rect, Shape, ShapeId, ShapeKind};
use std::path::PathBuf;
use std::sync::Once;

/// Ctrl (Command on other platforms), as egui-winit reports it on Windows.
pub const CTRL: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: false,
    mac_cmd: false,
    command: true,
};
pub const SHIFT: Modifiers = Modifiers::SHIFT;

/// Point every side-effect location (AutoRecover, settings, recent files) at a temp folder
/// so tests never touch the real %APPDATA%. Runs once per test process.
pub fn init_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("kernpage-uitest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::env::set_var("KERNPAGE_AUTORECOVER_DIR", dir.join("AutoRecover"));
        std::env::set_var("KERNPAGE_SETTINGS_FILE", dir.join("settings.txt"));
        std::env::set_var("KERNPAGE_RECENT_FILE", dir.join("recent.txt"));
    });
}

/// The path of a file in the repository's `samples/` folder.
pub fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples")
        .join(name)
}

/// A filled rectangle at (x, y) with size (w, h), all in inches.
pub fn rect_shape(x: f64, y: f64, w: f64, h: f64) -> Shape {
    let mut s = Shape::new(
        Rect::new(in_to_emu(x), in_to_emu(y), in_to_emu(w), in_to_emu(h)),
        ShapeKind::Rectangle,
    );
    s.fill = Some(kernpage_core::Rgb(0xCC, 0xDD, 0xEE));
    s
}

/// A one-page Letter document holding `shapes` (ids are assigned in order, from 1).
pub fn letter_doc(shapes: Vec<Shape>) -> Document {
    let mut page = Page::blank(in_to_emu(8.5), in_to_emu(11.0));
    page.shapes = shapes;
    Document::new("Test", vec![page])
}

pub struct TestUi {
    pub h: Harness<'static, App>,
    pointer: Pos2,
    /// The scripted file dialogs the app was given (panics on an unscripted dialog).
    pub dialogs: Dialogs,
    pub env: TestEnv,
}

/// A private folder per test: the AutoRecover folder, the settings file, the recent list and a
/// scratch folder for the files a test saves or opens. Installed for the current thread, so
/// tests running in parallel never see each other's snapshots.
#[derive(Clone)]
pub struct TestEnv {
    pub autorecover: PathBuf,
    pub settings: PathBuf,
    pub recent: PathBuf,
    pub files: PathBuf,
}

impl TestEnv {
    pub fn new() -> TestEnv {
        use std::sync::atomic::{AtomicU32, Ordering};
        init_env();
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir()
            .join(format!("kernpage-uitest-{}", std::process::id()))
            .join(format!("t{}", N.fetch_add(1, Ordering::SeqCst)));
        let env = TestEnv {
            autorecover: root.join("AutoRecover"),
            settings: root.join("settings.txt"),
            recent: root.join("recent.txt"),
            files: root.join("files"),
        };
        std::fs::create_dir_all(&env.autorecover).expect("temp dir");
        std::fs::create_dir_all(&env.files).expect("temp dir");
        crate::autorecover::test_env::install(env.autorecover.clone(), env.settings.clone());
        env
    }

    /// The `.kprec` (or legacy `.pwrec`) snapshot files currently in the AutoRecover folder.
    pub fn snapshots(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&self.autorecover)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| kernpage_core::autorecover::is_snapshot_path(p))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }
}

#[derive(Default)]
struct DialogState {
    open: std::collections::VecDeque<Option<PathBuf>>,
    save: std::collections::VecDeque<Option<PathBuf>>,
    opens: Vec<crate::filedialog::OpenRequest>,
    saves: Vec<crate::filedialog::SaveRequest>,
}

/// The fake native file dialogs: scripted answers (a path, or `None` for Cancel) and a record
/// of what each dialog was asked. An unscripted dialog panics: no test may show a real one.
#[derive(Clone, Default)]
pub struct Dialogs(std::rc::Rc<std::cell::RefCell<DialogState>>);

impl Dialogs {
    /// The next Open / Insert Picture dialog returns `answer`.
    pub fn script_open(&self, answer: Option<PathBuf>) {
        self.0.borrow_mut().open.push_back(answer);
    }
    /// The next Save As / Publish as PDF dialog returns `answer`.
    pub fn script_save(&self, answer: Option<PathBuf>) {
        self.0.borrow_mut().save.push_back(answer);
    }
    pub fn opens(&self) -> Vec<crate::filedialog::OpenRequest> {
        self.0.borrow().opens.clone()
    }
    pub fn saves(&self) -> Vec<crate::filedialog::SaveRequest> {
        self.0.borrow().saves.clone()
    }
    /// Every scripted answer was used.
    pub fn all_used(&self) -> bool {
        let s = self.0.borrow();
        s.open.is_empty() && s.save.is_empty()
    }
}

impl crate::filedialog::FileDialogs for Dialogs {
    fn pick_open(&self, req: &crate::filedialog::OpenRequest) -> Option<PathBuf> {
        let mut s = self.0.borrow_mut();
        s.opens.push(req.clone());
        s.open
            .pop_front()
            .unwrap_or_else(|| panic!("unscripted Open dialog: {req:?}"))
    }
    fn pick_save(&self, req: &crate::filedialog::SaveRequest) -> Option<PathBuf> {
        let mut s = self.0.borrow_mut();
        s.saves.push(req.clone());
        s.save
            .pop_front()
            .unwrap_or_else(|| panic!("unscripted Save dialog: {req:?}"))
    }
}

impl TestUi {
    /// The app as it starts with no file: the File view's Home page.
    pub fn start() -> TestUi {
        TestUi::start_in(TestEnv::new())
    }

    /// Like `start`, in a prepared environment (for example with a snapshot already in its
    /// AutoRecover folder).
    pub fn start_in(env: TestEnv) -> TestUi {
        init_env();
        crate::autorecover::test_env::install(env.autorecover.clone(), env.settings.clone());
        crate::output::test_os::set_print_dir(env.files.join("print"));
        crate::output::test_os::take_opened();
        let mut h = Harness::builder()
            .with_size(Vec2::new(1280.0, 800.0))
            // The ribbon's Character Spacing icon uses U+2194, which egui's bundled fonts lack.
            .allow_missing_glyphs()
            // Real-time frame spacing, so double-click and press/release timing behave.
            .with_step_dt(1.0 / 60.0)
            // A recovered publication is already open at the first frame, and the harness's
            // accessibility check of that frame names the ribbon's unlabelled combo boxes
            // (the other tests open their document after construction, which is not checked).
            .with_accessibility_check(env.snapshots().is_empty())
            .build_eframe(|cc| App::new(cc, Options::default()));
        let dialogs = Dialogs::default();
        h.state_mut().dialogs = Box::new(dialogs.clone());
        h.state_mut().recent = crate::recent::Recent::load_from(Some(env.recent.clone()));
        h.run_steps(3);
        TestUi {
            h,
            pointer: Pos2::ZERO,
            dialogs,
            env,
        }
    }

    /// The editor open on a new in-memory document.
    pub fn with_doc(doc: Document) -> TestUi {
        let mut ui = TestUi::start();
        ui.app().new_document(doc, None, None);
        ui.settle();
        ui
    }

    /// The editor open on a `.kern` fixture from `samples/kern/` (through the same path as
    /// File > Open). `name` is the sample's file name (`"hello.pub"`, `"gen/linked.pub"`); the
    /// fixture is `samples/kern/<stem>.kern`. Tests of the .pub reader and writer use
    /// `with_pub_sample` instead.
    pub fn with_sample(name: &str) -> TestUi {
        let stem = name.rsplit('/').next().unwrap().trim_end_matches(".pub");
        TestUi::with_pub_sample(&format!("kern/{stem}.kern"))
    }

    /// The editor open on a .pub from `samples/` (through the same path as File > Open).
    pub fn with_pub_sample(name: &str) -> TestUi {
        let mut ui = TestUi::start();
        ui.app().open_path(&sample(name));
        assert!(ui.app().error.is_none(), "sample failed to open");
        ui.settle();
        ui
    }

    pub fn app(&mut self) -> &mut App {
        self.h.state_mut()
    }

    pub fn doc(&self) -> &Document {
        self.h.state().editor.as_ref().expect("editor").doc()
    }

    pub fn shapes(&self) -> &[Shape] {
        &self.doc().pages[self.h.state().page].shapes
    }

    pub fn shape(&self, id: ShapeId) -> &Shape {
        self.shapes().iter().find(|s| s.id == id).expect("shape")
    }

    pub fn rect_of(&self, id: ShapeId) -> Rect {
        self.shape(id).rect
    }

    pub fn selected(&self) -> Vec<ShapeId> {
        self.h.state().selected_ids()
    }

    pub fn revision(&self) -> u64 {
        self.h.state().editor.as_ref().expect("editor").revision()
    }

    pub fn can_undo(&self) -> bool {
        self.h.state().editor.as_ref().expect("editor").can_undo()
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.app().tool = tool;
        self.settle();
    }

    /// Run a few frames so layout (zoom, page origin) is stable.
    pub fn settle(&mut self) {
        self.h.run_steps(8);
    }

    /// Steps until the background system-font scan has finished and every font load has become
    /// active (the scan runs on its own thread, so how many frames that takes depends on load).
    pub fn wait_for_fonts(&mut self) {
        let start = std::time::Instant::now();
        loop {
            self.h.run_steps(2);
            let f = &self.h.state().fonts;
            if f.scan_done() && f.settled() {
                break;
            }
            assert!(start.elapsed().as_secs() < 60, "the font scan did not finish");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        self.settle();
    }

    // ---- page <-> screen ---------------------------------------------------------

    /// Screen position of a point given in page inches.
    pub fn at_in(&self, x: f64, y: f64) -> Pos2 {
        self.at_emu(in_to_emu(x), in_to_emu(y))
    }

    pub fn at_emu(&self, x: i64, y: i64) -> Pos2 {
        let p = &self.h.state().probe;
        p.origin + Vec2::new(emu_to_px(x, p.zoom) as f32, emu_to_px(y, p.zoom) as f32)
    }

    /// Screen position of the centre of a shape (as stored in the model).
    pub fn center_of(&self, id: ShapeId) -> Pos2 {
        let r = self.rect_of(id);
        self.at_emu(r.x + r.w / 2, r.y + r.h / 2)
    }

    /// Screen position of a corner of a shape: (0,0) top-left, (1,1) bottom-right.
    pub fn corner_of(&self, id: ShapeId, fx: i64, fy: i64) -> Pos2 {
        let r = self.rect_of(id);
        self.at_emu(r.x + r.w * fx, r.y + r.h * fy)
    }

    /// The rectangle the canvas painted for a shape in the last frame, on screen.
    pub fn painted(&self, id: ShapeId) -> egui::Rect {
        self.h
            .state()
            .probe
            .painted
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, r)| *r)
            .expect("shape was not painted")
    }

    /// The screen rectangle a model rectangle should be painted at.
    pub fn screen_rect_of(&self, r: Rect) -> egui::Rect {
        egui::Rect::from_min_max(self.at_emu(r.x, r.y), self.at_emu(r.x + r.w, r.y + r.h))
    }

    // ---- input: one event, one frame ----------------------------------------------

    fn send(&mut self, e: Event) {
        self.h.event(e);
        self.h.step();
    }

    pub fn mouse_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.send(Event::PointerMoved(p));
    }

    pub fn press(&mut self, button: PointerButton, mods: Modifiers) {
        let pos = self.pointer;
        self.send(Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: mods,
        });
    }

    pub fn release(&mut self, button: PointerButton, mods: Modifiers) {
        let pos = self.pointer;
        self.send(Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: mods,
        });
    }

    fn hold(&mut self, mods: Modifiers) {
        if mods != Modifiers::NONE {
            self.send(Event::ModifiersChanged(mods));
        }
    }

    pub fn click_at(&mut self, p: Pos2) {
        self.click_mods(p, Modifiers::NONE);
    }

    pub fn click_mods(&mut self, p: Pos2, mods: Modifiers) {
        self.mouse_to(p);
        self.hold(mods);
        self.press(PointerButton::Primary, mods);
        self.release(PointerButton::Primary, mods);
        self.hold_off(mods);
        self.settle();
    }

    fn hold_off(&mut self, mods: Modifiers) {
        if mods != Modifiers::NONE {
            self.send(Event::ModifiersChanged(Modifiers::NONE));
        }
    }

    pub fn double_click_at(&mut self, p: Pos2) {
        self.mouse_to(p);
        for _ in 0..2 {
            self.press(PointerButton::Primary, Modifiers::NONE);
            self.release(PointerButton::Primary, Modifiers::NONE);
        }
        self.settle();
    }

    pub fn right_click_at(&mut self, p: Pos2) {
        self.mouse_to(p);
        self.press(PointerButton::Secondary, Modifiers::NONE);
        self.release(PointerButton::Secondary, Modifiers::NONE);
        self.settle();
    }

    /// Press at `from` and move in steps to `to`; stops before the release so the caller can
    /// inspect the drag preview. Finish with [`TestUi::release_primary`].
    pub fn drag_begin(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.mouse_to(from);
        self.hold(mods);
        self.press(PointerButton::Primary, mods);
        let steps = 6;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            self.mouse_to(from + (to - from) * t);
        }
    }

    /// Release the primary button: exactly one frame, not settled, so the caller can inspect
    /// what that frame painted (the "release frame").
    pub fn release_primary(&mut self) {
        self.release(PointerButton::Primary, Modifiers::NONE);
    }

    /// A complete drag (press, moves, release) followed by a few settling frames.
    pub fn drag(&mut self, from: Pos2, to: Pos2) {
        self.drag_mods(from, to, Modifiers::NONE);
    }

    pub fn drag_mods(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.drag_begin(from, to, mods);
        self.release_primary();
        self.hold_off(mods);
        self.settle();
    }

    /// Move the pointer to `p`, let the frame settle, and return the cursor icon the app asked
    /// the platform for (what the real window would show).
    pub fn cursor_icon_at(&mut self, p: Pos2) -> egui::CursorIcon {
        self.mouse_to(p);
        self.settle();
        self.h.output().platform_output.cursor_icon
    }

    /// One mouse-wheel notch of `delta` points with the pointer at `p`, modifiers held.
    pub fn wheel_at(&mut self, p: Pos2, delta: Vec2, mods: Modifiers) {
        self.mouse_to(p);
        self.hold(mods);
        self.send(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: mods,
        });
        self.hold_off(mods);
        self.settle();
    }

    /// A key chord such as Ctrl+Z.
    pub fn key(&mut self, mods: Modifiers, key: egui::Key) {
        self.h.key_press_modifiers(mods, key);
        self.h.step();
        self.settle();
    }

    pub fn type_text(&mut self, text: &str) {
        self.send(Event::Text(text.to_owned()));
        self.settle();
    }
}

// ---- batch 3 helpers: raw events, text of shapes --------------------------------------------

impl TestUi {
    /// Send one raw egui event (such as `Event::Copy`, which a real window produces for Ctrl+C)
    /// and settle.
    pub fn event(&mut self, e: Event) {
        self.send(e);
        self.settle();
    }

    /// The paragraphs of a text box.
    pub fn paragraphs(&self, id: ShapeId) -> Vec<kernpage_core::Paragraph> {
        match &self.shape(id).kind {
            ShapeKind::TextBox { paragraphs } => paragraphs.clone(),
            other => panic!("shape {id} is not a text box: {other:?}"),
        }
    }

    /// The text of a text box, paragraphs joined by `\n`.
    pub fn text_of(&self, id: ShapeId) -> String {
        self.paragraphs(id)
            .iter()
            .map(|p| p.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The text being edited in the in-place editor (not yet committed to the document).
    pub fn edit_text(&mut self) -> String {
        self.app().edit.as_ref().expect("edit mode").text.clone()
    }

    /// The editor's selection as character indexes (`primary`, `secondary`): `secondary` is
    /// where the selection started, `primary` is the caret end.
    pub fn caret(&mut self) -> (usize, usize) {
        let id = crate::app::edit_widget_id(self.h.state().edit.as_ref().expect("edit mode"));
        let st = egui::TextEdit::load_state(&self.h.ctx, id).expect("text edit state");
        let r = st.cursor.char_range().expect("cursor");
        (r.primary.index.0, r.secondary.index.0)
    }

    /// Put the editor's caret / selection at `from..to` (characters; `from == to` is a caret).
    pub fn set_selection_range(&mut self, from: usize, to: usize) {
        let id = crate::app::edit_widget_id(self.h.state().edit.as_ref().expect("edit mode"));
        let mut st = egui::TextEdit::load_state(&self.h.ctx, id).expect("text edit state");
        st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(from),
            egui::text::CCursor::new(to),
        )));
        st.store(&self.h.ctx, id);
        self.settle();
    }
}

// ---- batch 4 helpers: ribbon buttons ------------------------------------------------------------

pub use crate::widgets::probe::Seen;

impl TestUi {
    /// Every ribbon button drawn in the last frame (see `widgets::probe`).
    pub fn ribbon_buttons(&self) -> Vec<Seen> {
        crate::widgets::probe::all()
    }

    /// The ribbon button whose label or tooltip is exactly `name` ("Cut", "Bold (Ctrl+B)");
    /// panics unless exactly one is on screen.
    pub fn ribbon(&self, name: &str) -> Seen {
        let all = self.ribbon_buttons();
        let hits: Vec<_> = all
            .iter()
            .filter(|b| b.label == name || b.tip == name)
            .cloned()
            .collect();
        assert_eq!(hits.len(), 1, "ribbon button {name:?}: {} found among {:?}", hits.len(),
            all.iter().map(|b| format!("{}|{}", b.label, b.tip)).collect::<Vec<_>>());
        hits.into_iter().next().unwrap()
    }

    pub fn has_ribbon(&self, name: &str) -> bool {
        self.ribbon_buttons().iter().any(|b| b.label == name || b.tip == name)
    }

    /// Click a ribbon button (main part) with a real pointer click.
    pub fn click_ribbon(&mut self, name: &str) {
        let b = self.ribbon(name);
        self.click_at(b.rect.center());
    }

    /// Click the drop-down arrow of a split button: the 13 pt strip right of its main part.
    pub fn click_ribbon_arrow(&mut self, name: &str) {
        let b = self.ribbon(name);
        self.click_at(b.rect.right_center() + Vec2::new(7.0, 0.0));
    }

    /// Click a tab by its name ("Home", "Shape Format").
    pub fn click_tab(&mut self, name: &str) {
        self.click_ribbon(&format!("tab:{name}"));
    }

    pub fn tab(&mut self) -> crate::app::Tab {
        self.app().tab
    }

    pub fn tab_names(&self) -> Vec<String> {
        self.ribbon_buttons()
            .iter()
            .filter_map(|b| b.label.strip_prefix("tab:").map(str::to_owned))
            .filter(|n| n != "File")
            .collect()
    }
}

// ---- batch 4 helpers: page setup, pages pane, view chrome -------------------------------------

impl TestUi {
    /// Width and height of page `n` in EMU.
    pub fn page_size(&self, n: usize) -> (i64, i64) {
        let p = &self.doc().pages[n];
        (p.width_emu, p.height_emu)
    }

    pub fn margins(&self) -> kernpage_core::Margins {
        self.doc().pages[0].margins_emu
    }

    pub fn page_count(&self) -> usize {
        self.doc().pages.len()
    }

    /// How many undo steps the document has (undoes them all, then redoes them all).
    pub fn undo_steps(&mut self) -> usize {
        let ed = self.app().editor.as_mut().expect("editor");
        let mut n = 0;
        while ed.undo() {
            n += 1;
        }
        for _ in 0..n {
            ed.redo();
        }
        n
    }

    /// Undo once through the real shortcut.
    pub fn undo(&mut self) {
        self.key(CTRL, egui::Key::Z);
    }

    /// The page-setup gallery rows drawn in the last frame (empty when no gallery is open).
    pub fn gallery_rows(&self) -> Vec<crate::page_setup::probe::Row> {
        crate::page_setup::probe::rows()
    }

    pub fn gallery_titles(&self) -> Vec<String> {
        self.gallery_rows().into_iter().map(|r| r.title).collect()
    }

    /// Click a gallery row by title (a real pointer click on the row).
    pub fn click_gallery_row(&mut self, title: &str) {
        let row = self
            .gallery_rows()
            .into_iter()
            .find(|r| r.title == title)
            .unwrap_or_else(|| panic!("no gallery row {title:?} in {:?}", self.gallery_titles()));
        self.click_at(row.rect.center());
    }

    /// Page Design tab, then open the gallery button `name` ("Margins", "Orientation", "Size").
    pub fn open_gallery(&mut self, name: &str) {
        self.click_tab("Page Design");
        self.click_ribbon(name);
        assert!(!self.gallery_rows().is_empty(), "the {name} gallery opened");
    }
}

// ---- batch 6 helpers: whole-tab checks ----------------------------------------------------------------

impl TestUi {
    /// The controls of the ribbon's tab body: below the tab strip, above the status bar, without
    /// the group launchers.
    pub fn ribbon_body(&self) -> Vec<Seen> {
        self.ribbon_buttons()
            .into_iter()
            .filter(|b| b.rect.min.y > 52.0 && b.rect.min.y < 150.0)
            .filter(|b| !b.label.starts_with("launcher:") && !b.label.starts_with("group"))
            .collect()
    }

    /// The tab-body control labelled exactly `name` (the status bar has buttons with the same
    /// tooltips, such as "Whole Page", which `ribbon` cannot tell apart).
    pub fn body(&self, name: &str) -> Seen {
        let hits: Vec<Seen> = self.ribbon_body().into_iter().filter(|b| b.label == name).collect();
        assert_eq!(hits.len(), 1, "tab body control {name:?}: {} found", hits.len());
        hits.into_iter().next().unwrap()
    }

    pub fn click_body(&mut self, name: &str) {
        let at = self.body(name).rect.center();
        self.click_at(at);
    }

    /// The greyed controls of the tab body.
    pub fn greyed(&self) -> Vec<Seen> {
        self.ribbon_body().into_iter().filter(|b| !b.enabled).collect()
    }

    /// Click the centre of every greyed control of the tab body and check that nothing changed:
    /// no document edit, same tab, tool, selection, no dialog, no edit mode.
    pub fn click_all_greyed_and_expect_nothing(&mut self) {
        let (rev, tab, tool, sel) = (self.revision(), self.tab(), self.app().tool, self.selected());
        for b in self.greyed() {
            self.click_at(b.rect.center());
            assert_eq!(self.revision(), rev, "{:?} changed the document", b.label);
            assert_eq!(self.tab(), tab, "{:?} left the tab", b.label);
            assert_eq!(self.app().tool, tool, "{:?} armed a tool", b.label);
            assert_eq!(self.selected(), sel, "{:?} changed the selection", b.label);
            assert!(self.app().error.is_none() && self.app().prompt.is_none(), "{:?} opened a dialog", b.label);
            assert!(self.app().edit.is_none(), "{:?} started editing", b.label);
        }
    }
}
