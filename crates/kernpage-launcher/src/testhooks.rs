//! Test-only seams: where the launcher would touch the real world (the editor process, the
//! installer helper, the browser, Notepad, Explorer, toasts, the tray icon, the Run key, the
//! settings / recent / updates folders) a test build asks this module instead. The fake is
//! per thread, so tests running in parallel never see each other's folders or calls.
//! Compiled only for tests.

use crate::install::InstallScope;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

/// One `install::start_helper` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperCall {
    pub msi: PathBuf,
    pub version: String,
    pub updates_dir: PathBuf,
    pub feed_url: Option<String>,
}

pub struct Fake {
    pub config: PathBuf,
    pub recent: PathBuf,
    pub updates: PathBuf,
    pub editor: PathBuf,
    pub editor_running: bool,
    pub scope: InstallScope,
    /// The app was installed by Setup.exe (the update must then be a `setup` file).
    pub bundle: bool,
    /// Editor launches: the file argument of each.
    pub launched: Vec<Option<PathBuf>>,
    pub launch_error: Option<String>,
    /// "Open file location" calls.
    pub located: u32,
    pub urls: Vec<String>,
    pub logs: Vec<String>,
    pub toasts: Vec<(String, String)>,
    pub helpers: Vec<HelperCall>,
    pub helper_result: Result<(), String>,
    pub tray_creates: u32,
    pub tray_fails: bool,
    pub run_key: BTreeMap<String, String>,
    pub run_key_fails: bool,
    /// Scripted answers of the Open file dialog (`None` = Cancel); unscripted use panics.
    pub picks: VecDeque<Option<PathBuf>>,
    pub picks_asked: u32,
    /// "Quit launcher" presses (the process is not really ended).
    pub quits: u32,
    /// What the last frame painted without accessibility labels: status badges, card
    /// rectangles (by card index) and recent-file rows (name, folder, date text, rectangle).
    pub badges: Vec<String>,
    pub cards: Vec<(usize, eframe::egui::Rect)>,
    pub rows: Vec<(String, String, String, eframe::egui::Rect)>,
}

thread_local! {
    static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) };
}

pub fn install(f: Fake) {
    FAKE.with(|c| *c.borrow_mut() = Some(f));
}

/// Read or change the fake of this thread; panics when none is installed.
pub fn with<R>(f: impl FnOnce(&mut Fake) -> R) -> R {
    FAKE.with(|c| f(c.borrow_mut().as_mut().expect("no launcher test environment installed on this thread")))
}

/// Like `with`, but `None` when no fake is installed (plain unit tests).
pub fn peek<R>(f: impl FnOnce(&Fake) -> R) -> Option<R> {
    FAKE.with(|c| c.borrow().as_ref().map(f))
}

/// Record something the frame painted; does nothing when no fake is installed.
pub fn seen(f: impl FnOnce(&mut Fake)) {
    FAKE.with(|c| {
        if let Some(x) = c.borrow_mut().as_mut() {
            f(x);
        }
    });
}

/// A new frame starts: forget what the last one painted.
pub fn frame_start() {
    seen(|f| {
        f.badges.clear();
        f.cards.clear();
        f.rows.clear();
    });
}
