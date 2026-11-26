//! Test harness for the launcher: the real `App` inside an egui_kittest `Harness`, a private
//! folder per test (settings, recent list, updates, editor stub), a fake for everything that
//! would touch the real world (see `crate::testhooks`) and a local signed feed (loopback,
//! signed with the test key, as the `fake-feed` example).

use crate::app::{App, Options};
use crate::devfeed::*;
use crate::devserver::*;
use crate::install::{InstallResult, InstallScope};
use crate::testhooks::{self, Fake};
use eframe::egui::{self, Pos2, Rect, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use semver::Version;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// The version the sample feed offers as the newest stable KernPage.
pub const NEW: &str = "0.2.0";
/// A version older than `NEW` (an update is available).
pub const OLD: &str = "0.1.2";
/// A feed address nothing listens on: connecting is refused at once ("could not reach").
pub const DEAD_FEED: &str = "http://127.0.0.1:1/index.json";

// ---- the private folders of one test ---------------------------------------------------------------

#[derive(Clone)]
pub struct TestEnv {
    pub config: PathBuf,
    pub recent: PathBuf,
    pub updates: PathBuf,
    /// The stand-in for kernpage.exe (an empty file).
    pub editor: PathBuf,
    /// Scratch folder for the .pub files a test lists or opens.
    pub files: PathBuf,
}

impl TestEnv {
    pub fn new() -> TestEnv {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir()
            .join(format!("kernpage-launcher-uitest-{}", std::process::id()))
            .join(format!("t{}", N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&root);
        let env = TestEnv {
            config: root.join("config").join("launcher.toml"),
            recent: root.join("config").join("recent.txt"),
            updates: root.join("data").join("KernPage").join("updates"),
            editor: root.join("app").join("kernpage.exe"),
            files: root.join("files"),
        };
        for d in [env.config.parent().unwrap(), &env.updates, env.editor.parent().unwrap(), &env.files] {
            std::fs::create_dir_all(d).expect("temp dir");
        }
        std::fs::write(&env.editor, b"").expect("editor stub");
        env
    }

    /// A real (tiny) .pub file in the scratch folder.
    pub fn pub_file(&self, name: &str) -> PathBuf {
        let p = self.files.join(name);
        std::fs::write(&p, b"x").expect("pub file");
        p
    }

    pub fn config_text(&self) -> String {
        std::fs::read_to_string(&self.config).unwrap_or_default()
    }
}

// ---- the local feed -----------------------------------------------------------------------------------

/// What the local feed serves.
#[derive(Clone)]
pub struct FeedOpts {
    pub serial: u64,
    pub version: String,
    pub bad_sha: bool,
    pub block: Option<String>,
    /// Serve a real (tiny) installer for the newest release: downloads can complete.
    pub real: bool,
    /// One more, older stable release with a real file.
    pub older: Option<String>,
    pub expired: bool,
    pub tamper: bool,
    pub wrong_key: bool,
    /// Also list a Setup.exe (`setup` file) beside each MSI.
    pub setup: bool,
}

impl Default for FeedOpts {
    fn default() -> Self {
        FeedOpts { serial: 5, version: NEW.into(), bad_sha: false, block: None, real: true, older: None, expired: false, tamper: false, wrong_key: false, setup: false }
    }
}

pub const INSTALLER: &[u8] = b"pretend installer bytes for the local test feed";
pub const SETUP_INSTALLER: &[u8] = b"pretend Setup.exe bytes for the local test feed";

struct Gate(Mutex<bool>, Condvar);

/// A running local feed: `/index.json`, its `.minisig` and the files, on a free loopback port.
pub struct FeedServer {
    pub server: Server,
    pub files: Files,
    gate: Arc<Gate>,
}

impl FeedServer {
    pub fn start(o: FeedOpts) -> FeedServer {
        let files: Files = Arc::new(Mutex::new(HashMap::new()));
        let gate = Arc::new(Gate(Mutex::new(true), Condvar::new()));
        let inner = static_handler(files.clone());
        let g = gate.clone();
        let handler: Handler = Arc::new(move |req: &Request| {
            if req.path.starts_with("/files/") {
                let mut open = g.0.lock().unwrap();
                while !*open {
                    open = g.1.wait(open).unwrap();
                }
            }
            inner(req)
        });
        let server = Server::start(0, handler).expect("bind");
        let base = format!("http://127.0.0.1:{}", server.port);
        let mut so = SampleOpts::new(o.serial, &base);
        so.kernpage_version = o.version.clone();
        so.bad_sha = o.bad_sha;
        so.block_kernpage = o.block.clone();
        if o.real {
            so.app_real = Some(RealFile { version: o.version.clone(), bytes: INSTALLER.to_vec(), setup: o.setup.then(|| SETUP_INSTALLER.to_vec()) });
        }
        if let Some(v) = &o.older {
            so.app_older = Some(RealFile { version: v.clone(), bytes: format!("older installer {v}").into_bytes(), setup: o.setup.then(|| format!("older setup {v}").into_bytes()) });
        }
        if o.expired {
            so.expires = "2020-01-01T00:00:00Z".into();
        }
        let mut index = sample_bytes(&so);
        let sig = if o.wrong_key { sign_with(OTHER_SECRET, &index) } else { sign_test(&index) };
        if o.tamper {
            index.push(b'\n');
        }
        {
            let mut f = files.lock().unwrap();
            f.insert("/index.json".into(), index);
            f.insert("/index.json.minisig".into(), sig.into_bytes());
            for (name, bytes) in served_files(&so) {
                f.insert(format!("/files/{name}"), bytes);
            }
        }
        FeedServer { server, files, gate }
    }

    pub fn url(&self) -> String {
        self.server.url("/index.json")
    }

    /// Hold every file download (not the index) until [`FeedServer::open_gate`].
    pub fn close_gate(&self) {
        *self.gate.0.lock().unwrap() = false;
    }

    pub fn open_gate(&self) {
        *self.gate.0.lock().unwrap() = true;
        self.gate.1.notify_all();
    }

    /// Stop serving one path (it answers 404 from now on).
    pub fn remove(&self, path: &str) {
        self.files.lock().unwrap().remove(path);
    }
}

impl Drop for FeedServer {
    fn drop(&mut self) {
        self.open_gate(); // a held download must not block the server's shutdown
    }
}

// ---- the setup of one test ------------------------------------------------------------------------------

pub struct Setup {
    pub size: Vec2,
    pub feed_url: String,
    pub config: Option<String>,
    pub installed: Option<Version>,
    pub editor_present: bool,
    pub scope: InstallScope,
    /// Installed by Setup.exe: updates must come from the `setup` file.
    pub bundle: bool,
    pub editor_running: bool,
    pub recent: Vec<PathBuf>,
    pub report: Option<InstallResult>,
    pub opts: Options,
    pub env: Option<TestEnv>,
    /// The Run key already holds the launcher's value.
    pub run_key_on: bool,
}

impl Setup {
    pub fn new() -> Setup {
        Setup {
            size: Vec2::new(1093.0, 798.0),
            feed_url: DEAD_FEED.into(),
            config: None,
            installed: None,
            editor_present: true,
            scope: InstallScope::PerUser,
            bundle: false,
            editor_running: false,
            recent: Vec::new(),
            report: None,
            opts: Options::default(),
            env: None,
            run_key_on: false,
        }
    }
    pub fn size(mut self, w: f32, h: f32) -> Setup {
        self.size = Vec2::new(w, h);
        self
    }
    pub fn feed(mut self, f: &FeedServer) -> Setup {
        self.feed_url = f.url();
        self
    }
    pub fn config(mut self, text: &str) -> Setup {
        self.config = Some(text.to_owned());
        self
    }
    pub fn installed(mut self, v: &str) -> Setup {
        self.installed = Some(Version::parse(v).unwrap());
        self
    }
    pub fn scope(mut self, s: InstallScope) -> Setup {
        self.scope = s;
        self
    }
    pub fn bundle(mut self) -> Setup {
        self.bundle = true;
        self
    }
    pub fn editor_running(mut self) -> Setup {
        self.editor_running = true;
        self
    }
    pub fn no_editor(mut self) -> Setup {
        self.editor_present = false;
        self
    }
    pub fn recent(mut self, paths: &[PathBuf]) -> Setup {
        self.recent = paths.to_vec();
        self
    }
    pub fn report(mut self, r: InstallResult) -> Setup {
        self.report = Some(r);
        self
    }
    pub fn settings(mut self) -> Setup {
        self.opts.settings = true;
        self
    }
    pub fn run_key_on(mut self) -> Setup {
        self.run_key_on = true;
        self
    }
    pub fn env(mut self, e: &TestEnv) -> Setup {
        self.env = Some(e.clone());
        self
    }
}

pub struct TestUi {
    pub h: Harness<'static, App>,
    pub env: TestEnv,
    pointer: Pos2,
}

impl TestUi {
    /// The launcher as it starts: no feed reachable, manual updates, an installed copy.
    pub fn start() -> TestUi {
        TestUi::with(Setup::new())
    }

    pub fn with(s: Setup) -> TestUi {
        let env = s.env.clone().unwrap_or_else(TestEnv::new);
        if let Some(c) = &s.config {
            std::fs::write(&env.config, c).expect("config");
        }
        if !s.editor_present {
            let _ = std::fs::remove_file(&env.editor);
        }
        if !s.recent.is_empty() {
            let lines: Vec<String> = s.recent.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            std::fs::write(&env.recent, lines.join("\n")).expect("recent");
        }
        if let Some(r) = &s.report {
            crate::install::write_result(&env.updates, r).expect("report");
        }
        testhooks::install(Fake {
            config: env.config.clone(),
            recent: env.recent.clone(),
            updates: env.updates.clone(),
            editor: env.editor.clone(),
            editor_running: s.editor_running,
            scope: s.scope,
            bundle: s.bundle,
            launched: Vec::new(),
            launch_error: None,
            located: 0,
            urls: Vec::new(),
            logs: Vec::new(),
            toasts: Vec::new(),
            helpers: Vec::new(),
            helper_result: Ok(()),
            tray_creates: 0,
            tray_fails: false,
            run_key: if s.run_key_on { BTreeMap::from([(crate::run_key::VALUE_NAME.to_owned(), r#""C:\old\launcher.exe""#.to_owned())]) } else { BTreeMap::new() },
            run_key_fails: false,
            picks: VecDeque::new(),
            picks_asked: 0,
            quits: 0,
            badges: Vec::new(),
            cards: Vec::new(),
            rows: Vec::new(),
        });
        let settings_open = s.opts.settings;
        let mut opts = s.opts;
        opts.feed_url = Some(s.feed_url.clone());
        let mut h = Harness::builder()
            .with_size(s.size)
            .allow_missing_glyphs()
            .with_step_dt(1.0 / 60.0)
            // The settings dialog's combo boxes have no accessible name (reported, not changed);
            // the harness checks only the first frame, which holds the dialog when it starts open.
            .with_accessibility_check(!settings_open)
            .build_eframe(move |cc| App::new(cc, opts));
        if let Some(v) = s.installed {
            h.state_mut().installed = v;
        }
        let mut ui = TestUi { h, env, pointer: Pos2::ZERO };
        ui.settle();
        ui
    }

    pub fn app(&mut self) -> &mut App {
        self.h.state_mut()
    }

    /// Read or change the fake OS (calls recorded, scripted answers).
    pub fn fake<R>(&self, f: impl FnOnce(&mut Fake) -> R) -> R {
        testhooks::with(f)
    }

    pub fn settle(&mut self) {
        self.h.run_steps(6);
    }

    /// Step frames (sleeping a little between them) until `cond` holds; panics after 15 s.
    pub fn wait_until(&mut self, what: &str, mut cond: impl FnMut(&mut TestUi) -> bool) {
        let t0 = Instant::now();
        loop {
            self.h.step();
            if cond(self) {
                return;
            }
            if t0.elapsed() > Duration::from_secs(15) {
                panic!("timed out waiting for: {what}\nscreen text: {:?}", self.texts());
            }
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    /// Wait until no update check is running, then settle.
    pub fn wait_check(&mut self) {
        self.h.run_steps(2);
        self.wait_until("the update check to finish", |u| !u.app().updater.is_checking());
        self.settle();
    }

    // ---- looking at the screen -------------------------------------------------------------------

    /// Every non-empty label on screen.
    pub fn texts(&self) -> Vec<String> {
        self.h
            .root()
            .children_recursive()
            .filter_map(|n| n.accesskit_node().label().or_else(|| n.accesskit_node().value()))
            .filter(|l| !l.is_empty())
            .collect()
    }

    pub fn has(&self, label: &str) -> bool {
        self.h.query_all_by_label(label).next().is_some()
    }

    /// Some label on screen contains `part`.
    pub fn has_text(&self, part: &str) -> bool {
        self.texts().iter().any(|t| t.contains(part))
    }

    pub fn count(&self, label: &str) -> usize {
        self.h.query_all_by_label(label).count()
    }

    /// The one control labelled exactly `label`; panics if there is none or several.
    pub fn node<'a>(&'a self, label: &'a str) -> egui_kittest::Node<'a> {
        let all: Vec<_> = self.h.query_all_by_label(label).collect();
        assert_eq!(all.len(), 1, "{label:?}: {} found among {:?}", all.len(), self.texts());
        all.into_iter().next().unwrap()
    }

    pub fn rect(&self, label: &str) -> Rect {
        self.node(label).rect()
    }

    pub fn enabled(&self, label: &str) -> bool {
        !self.node(label).accesskit_node().is_disabled()
    }

    /// A real pointer click on the control (move, press, release, settle).
    pub fn click(&mut self, label: &str) {
        let at = self.rect(label).center();
        self.click_at(at);
    }

    pub fn click_nth(&mut self, label: &str, n: usize) {
        let at = {
            let all: Vec<_> = self.h.query_all_by_label(label).collect();
            all.get(n).unwrap_or_else(|| panic!("no {n}th {label:?}")).rect().center()
        };
        self.click_at(at);
    }

    /// Click the last control with this label in the tree: the top-most one when a modal is
    /// open over the page (the page behind has the same labels).
    pub fn click_last(&mut self, label: &str) {
        self.h.query_all_by_label(label).last().unwrap_or_else(|| panic!("no {label:?}")).scroll_to_me();
        self.settle();
        let at = {
            let all: Vec<_> = self.h.query_all_by_label(label).collect();
            all.last().unwrap_or_else(|| panic!("no {label:?}")).rect().center()
        };
        self.click_at(at);
    }

    /// Whether the last control with this label is enabled.
    pub fn enabled_last(&self, label: &str) -> bool {
        let all: Vec<_> = self.h.query_all_by_label(label).collect();
        !all.last().unwrap_or_else(|| panic!("no {label:?}")).accesskit_node().is_disabled()
    }

    pub fn hover_last(&mut self, label: &str) {
        let at = {
            let all: Vec<_> = self.h.query_all_by_label(label).collect();
            all.last().unwrap_or_else(|| panic!("no {label:?}")).rect().center()
        };
        self.mouse_to(at);
        self.h.run_steps(45);
    }

    pub fn click_at(&mut self, p: Pos2) {
        self.mouse_to(p);
        self.h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
        self.h.step();
        self.h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
        self.h.step();
        self.settle();
    }

    pub fn mouse_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.h.event(egui::Event::PointerMoved(p));
        self.h.step();
    }

    /// Hover a control and settle (long enough for its tooltip).
    pub fn hover(&mut self, label: &str) {
        let at = self.rect(label).center();
        self.mouse_to(at);
        self.h.run_steps(45);
    }

    /// Click the last control with this label and return the viewport commands (such as Close)
    /// the click frame produced, before later frames replace them.
    pub fn click_peek(&mut self, label: &str) -> Vec<egui::ViewportCommand> {
        let at = {
            let all: Vec<_> = self.h.query_all_by_label(label).collect();
            all.last().unwrap_or_else(|| panic!("no {label:?}")).rect().center()
        };
        self.mouse_to(at);
        self.h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
        self.h.step();
        self.h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
        self.h.step();
        let cmds = self.h.output().viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
        self.settle();
        cmds
    }

    /// Hover a control and report whether its tooltip `text` is shown.
    pub fn hover_ok(&mut self, label: &str, text: &str) -> bool {
        self.hover_last(label);
        self.has(text)
    }

    pub fn key(&mut self, key: egui::Key) {
        self.h.key_press(key);
        self.h.step();
        self.settle();
    }

    pub fn window(&self) -> Rect {
        self.h.ctx.content_rect()
    }

    /// The combo boxes on screen (they have no accessible name): `(rect, enabled)`, top first.
    pub fn combos(&self) -> Vec<(Rect, bool)> {
        use egui::accesskit::Role;
        let mut v: Vec<(Rect, bool)> = self.h.query_all_by_role(Role::ComboBox).map(|n| (n.rect(), !n.accesskit_node().is_disabled())).collect();
        v.sort_by(|a, b| a.0.min.y.total_cmp(&b.0.min.y));
        v
    }

    pub fn click_combo(&mut self, n: usize) {
        let at = self.combos()[n].0.center();
        self.click_at(at);
    }

    /// Every labelled control on screen with its rectangle.
    pub fn controls(&self) -> Vec<(String, Rect)> {
        self.h
            .root()
            .children_recursive()
            .filter(|n| n.accesskit_node().bounding_box().is_some())
            .filter_map(|n| {
                let l = n.accesskit_node().label()?;
                (!l.is_empty()).then(|| (l.to_string(), n.rect()))
            })
            .collect()
    }

    /// The status badges painted in the last frame (app card first).
    pub fn badges(&self) -> Vec<String> {
        testhooks::with(|f| f.badges.clone())
    }

    pub fn card_rect(&self, idx: usize) -> Rect {
        testhooks::with(|f| f.cards.iter().find(|(i, _)| *i == idx).map(|(_, r)| *r)).unwrap_or_else(|| panic!("no card {idx}"))
    }

    /// The recent-file rows painted in the last frame: (name, folder, date text, rect).
    pub fn rows(&self) -> Vec<(String, String, String, Rect)> {
        testhooks::with(|f| f.rows.clone())
    }

    // ---- common shortcuts ----------------------------------------------------------------------------

    /// Press Refresh and wait for the check to end.
    pub fn refresh(&mut self) {
        self.click("Refresh");
        self.wait_check();
    }

    /// Open the app card's "..." menu (the first on the page).
    pub fn open_more(&mut self) {
        self.click_nth("...", 0);
    }

    pub fn state(&mut self) -> crate::views::State {
        self.app().make_views_for_test()[0].state
    }
}

/// A started feed plus a launcher using it, with the update checked and the "X is available"
/// dialog (if the check brought one up) dismissed with Later.
pub fn checked(opts: FeedOpts, setup: Setup) -> (FeedServer, TestUi) {
    let (feed, mut ui) = checked_raw(opts, setup);
    if ui.has("Later") {
        ui.click("Later");
    }
    (feed, ui)
}

/// Like [`checked`], leaving the announcement dialog open.
pub fn checked_raw(opts: FeedOpts, setup: Setup) -> (FeedServer, TestUi) {
    let feed = FeedServer::start(opts);
    let mut ui = TestUi::with(setup.feed(&feed));
    ui.refresh();
    (feed, ui)
}
