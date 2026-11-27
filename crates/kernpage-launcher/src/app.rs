//! The launcher window: dashboard, App details panel, settings and the recent files list.

use crate::config::{Appearance, CheckMode, Config};
use crate::download::{self, Downloader, Job, Status as Dl};
use crate::feed::{self, ago, human_size};
use crate::theme::{self, Palette};
use crate::updates::{Outcome, Updater};
use crate::views::{self, ItemView, State};
use crate::install::{self, InstallResult, InstallScope};
use crate::policy::{auto_install_plan, AutoInstallInput};
use crate::{instance, mark, notify, run_key, tray};
use std::cell::Cell;
use semver::Version;
use std::collections::HashSet;
use eframe::egui::{
    self, vec2, Align, Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Key, Layout, Margin,
    RichText, Sense, Stroke, StrokeKind, UiBuilder,
};
use kernpage_core::recent::{breadcrumb, date_text, Recent};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// The version shown to the user and compared by update checks: the `KERNPAGE_VERSION` the installer
/// build was given (package.ps1 -OverrideVersion), else the crate's own version.
pub(crate) const fn app_version() -> &'static str {
    match option_env!("KERNPAGE_VERSION") {
        Some(v) => v,
        None => env!("CARGO_PKG_VERSION"),
    }
}
pub(crate) const VERSION: &str = app_version();
const NEEDS_INSTALL: &str = "Available once installing updates arrives (next step)";
const NOT_PUBLIC: &str = "Available once the project is public";

#[derive(Default)]
pub struct Options {
    pub screenshot: Option<PathBuf>,
    pub window_size: Option<(f32, f32)>,
    pub details: bool,
    pub settings: bool,
    pub theme: Option<Appearance>,
    pub feed_url: Option<String>,
    pub check_now: bool,
    pub update_dialog: bool,
    /// `--update-now` / `--update-to <ver>` (dev builds only): headless download and install.
    pub update_now: bool,
    pub update_to: Option<String>,
    /// `--check-update`: print whether an update exists and exit (no window); see `quiet.rs`.
    pub check_update: bool,
    /// Helper mode (`--apply-update`); handled in `main` before any window exists.
    pub apply_update: Option<install::ApplyUpdate>,
}

pub(crate) struct Shot {
    path: PathBuf,
    requested: bool,
    done: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    started: Instant,
    frames: u32,
    /// Consecutive frames with no check running (so the picture shows the feed's data).
    idle: u32,
}

/// One line of the recent files list.
pub(crate) struct Row {
    path: PathBuf,
    name: String,
    folder: String,
    modified: Option<String>,
}

/// The "installing arrives in a later step" dialog for add-ons; not built yet.
pub(crate) enum Stub {
    Install { name: String },
}

pub struct App {
    pub(crate) cfg: Config,
    pub(crate) recent_rows: Vec<Row>,
    pub(crate) last_refresh: Instant,
    pub(crate) notice: Option<String>,
    pub(crate) details: bool,
    pub(crate) settings: bool,
    pub(crate) confirm_clear: bool,
    pub(crate) theme_override: Option<Appearance>,
    pub(crate) tray: Option<tray::Tray>,
    pub(crate) shot: Option<Shot>,
    pub(crate) updater: Updater,
    pub(crate) downloads: Downloader,
    pub(crate) installed: Version,
    pub(crate) details_idx: usize,
    pub(crate) show_all_changes: bool,
    /// The "<App> X is available" dialog is open for this item id.
    pub(crate) announce: Option<String>,
    /// `id@version` the user said "Later" to (or already looked at) this session.
    pub(crate) snoozed: HashSet<String>,
    pub(crate) stub: Option<Stub>,
    /// The current check was started by the user, so a failure is shown prominently.
    pub(crate) check_user: bool,
    pub(crate) force_announce: bool,
    /// How this copy is installed; installing is allowed only when it is not `NotInstalled`.
    pub(crate) scope: InstallScope,
    /// Last look for a running editor (at most once per second).
    pub(crate) editor_poll: Cell<Option<(Instant, bool)>>,
    /// `(item id, version)` the user chose "Update and restart" for: installs once downloaded.
    pub(crate) auto_install: Option<(String, String)>,
    /// How the update installed before this start went (read from `last-install.json`).
    pub(crate) report: Option<InstallResult>,
    /// "Go back to X?" is asking for confirmation (the older version's number).
    pub(crate) rollback_confirm: Option<String>,
    /// The older version being downloaded to go back to (shown in the "Older versions" section).
    pub(crate) rollback: Option<String>,
    /// The "Go back to an older version" list is open.
    pub(crate) rollback_list: bool,
    /// `(item id, version)` downloaded by "Download and install updates automatically": it
    /// installs by itself only once nobody is using the editor or the launcher window.
    pub(crate) auto_pending: Option<(String, String)>,
    /// The window was closed to the tray; cleared when it is focused again.
    pub(crate) hidden_to_tray: bool,
    /// The window is on screen (updated every frame).
    pub(crate) window_visible: bool,
}

/// The editor next to the launcher (override for tests with `KERNPAGE_EDITOR_EXE`).
pub(crate) fn editor_exe() -> PathBuf {
    #[cfg(test)]
    if let Some(p) = crate::testhooks::peek(|f| f.editor.clone()) {
        return p;
    }
    if let Some(p) = std::env::var_os("KERNPAGE_EDITOR_EXE") {
        return PathBuf::from(p);
    }
    let dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    dir.join(format!("kernpage{}", std::env::consts::EXE_SUFFIX))
}

fn launch_editor(file: Option<&Path>) -> Result<(), String> {
    let exe = editor_exe();
    if !exe.is_file() {
        return Err(format!(
            "KernPage was not found next to the launcher (looked for {}).",
            exe.display()
        ));
    }
    #[cfg(test)]
    return crate::testhooks::with(|f| match &f.launch_error {
        Some(e) => Err(e.clone()),
        None => {
            f.launched.push(file.map(Path::to_path_buf));
            Ok(())
        }
    });
    #[cfg(not(test))]
    let mut cmd = std::process::Command::new(&exe);
    #[cfg(not(test))]
    if let Some(f) = file {
        cmd.arg(f);
    }
    #[cfg(not(test))]
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start KernPage: {e}"))
}

/// "Quit launcher": ends the process (a test build only records the press).
fn quit_launcher() {
    #[cfg(test)]
    crate::testhooks::with(|f| f.quits += 1);
    #[cfg(not(test))]
    std::process::exit(0);
}

fn open_location() -> Result<(), String> {
    #[cfg(test)]
    return crate::testhooks::with(|f| {
        f.located += 1;
        Ok(())
    });
    #[cfg(not(test))]
    open_location_real()
}

#[cfg(not(test))]
fn open_location_real() -> Result<(), String> {
    let exe = editor_exe();
    #[cfg(windows)]
    let res = if exe.is_file() {
        std::process::Command::new(kernpage_core::sysexe::windows_exe("explorer.exe")).arg(format!("/select,{}", exe.display())).spawn()
    } else {
        std::process::Command::new(kernpage_core::sysexe::windows_exe("explorer.exe")).arg(exe.parent().unwrap_or(Path::new("."))).spawn()
    };
    #[cfg(not(windows))]
    let res = std::process::Command::new("xdg-open").arg(exe.parent().unwrap_or(Path::new("."))).spawn();
    res.map(|_| ()).map_err(|e| format!("Could not open the folder: {e}"))
}

/// The recent files list including missing files (the launcher shows them as "NOT FOUND").
fn recent_all() -> Recent {
    #[cfg(test)]
    if let Some(f) = crate::testhooks::peek(|f| f.recent.clone()) {
        return Recent::load_all_from(Some(f));
    }
    Recent::load_all()
}

fn load_rows() -> Vec<Row> {
    let list = recent_all();
    list.items()
        .iter()
        .map(|p| Row {
            path: p.clone(),
            name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            folder: p.parent().map(breadcrumb).unwrap_or_default(),
            modified: std::fs::metadata(p)
                .ok()
                .filter(|m| m.is_file())
                .and_then(|m| m.modified().ok().map(|t| (t, m)))
                .map(|(t, _)| match list.opened_at(p) {
                    Some(at) => kernpage_core::recent::opened_label_now(at),
                    None => date_text(t),
                }),
        })
        .collect()
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Options) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let mut cfg = Config::load();
        if opts.screenshot.is_none() && cfg!(windows) {
            // The registry is the truth for "start at login".
            cfg.start_at_login = run_key::is_enabled(&run_key::RegistryRunStore);
        }
        let url = opts.feed_url.clone().unwrap_or_else(|| feed::FEED_URL.to_owned());
        let cache_dir = crate::updates::cache_dir();
        let mut updater = Updater::new(url, cache_dir);
        let ctx = cc.egui_ctx.clone();
        updater.set_repaint(Arc::new(move || ctx.request_repaint()));
        updater.load_cache(feed::now_unix(), cfg.last_serial); // disk only, no network
        let mut downloads = Downloader::new(download::updates_dir());
        let ctx = cc.egui_ctx.clone();
        downloads.set_repaint(Arc::new(move || ctx.request_repaint()));
        let check_now = opts.check_now;
        let mut app = App {
            updater,
            downloads,
            installed: Version::parse(VERSION).unwrap_or_else(|_| Version::new(0, 0, 0)),
            details_idx: 0,
            show_all_changes: false,
            announce: None,
            snoozed: HashSet::new(),
            stub: None,
            check_user: false,
            force_announce: opts.update_dialog,
            scope: install::install_scope(),
            editor_poll: Cell::new(None),
            auto_install: None,
            report: None,
            rollback_confirm: None,
            rollback: None,
            rollback_list: false,
            auto_pending: None,
            hidden_to_tray: false,
            window_visible: true,
            tray: None,
            recent_rows: load_rows(),
            last_refresh: Instant::now(),
            notice: None,
            details: opts.details,
            settings: opts.settings,
            confirm_clear: false,
            theme_override: opts.theme,
            shot: opts.screenshot.map(|path| Shot {
                path,
                requested: false,
                done: Arc::new(AtomicBool::new(false)),
                failed: Arc::new(AtomicBool::new(false)),
                started: Instant::now(),
                frames: 0,
                idle: 0,
            }),
            cfg,
        };
        app.apply_appearance(&cc.egui_ctx);
        if app.cfg.keep_in_tray && app.shot.is_none() {
            app.tray = tray::create();
        }
        if app.shot.is_none() {
            app.report = install::take_result(&download::updates_dir());
        }
        if check_now {
            app.check_now_by_user();
        }
        app
    }

    fn appearance(&self) -> Appearance {
        self.theme_override.unwrap_or(self.cfg.appearance)
    }

    pub(crate) fn apply_appearance(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.appearance() {
            Appearance::System => egui::ThemePreference::System,
            Appearance::Dark => egui::ThemePreference::Dark,
            Appearance::Light => egui::ThemePreference::Light,
        });
    }

    /// Reload the recent files list (disk only; never touches the network).
    fn refresh(&mut self) {
        self.recent_rows = load_rows();
        self.last_refresh = Instant::now();
    }

    /// The Refresh button, Settings "Check now" and `--check-now`: the only ways, besides
    /// Automatic mode, to make a network request.
    pub(crate) fn check_now_by_user(&mut self) {
        self.refresh();
        if !self.updater.is_checking() {
            self.check_user = true;
            self.updater.start_check(feed::now_unix(), self.cfg.last_serial);
        }
    }

    /// The download for the newest offered release of `id`, for this computer.
    fn job_for(&self, id: &str) -> Option<Job> {
        let feed = &self.updater.snapshot.as_ref()?.feed;
        let item = feed.item(id)?;
        let c = feed.latest(item, self.cfg.include_prereleases)?;
        let file = c.release.file_for_this_platform()?.clone();
        let version = c.version.to_string();
        Some(Job { key: Job::key_for(id, &version), item_id: id.to_owned(), version, file, feed_url: self.updater.url().to_owned() })
    }

    /// The download for one specific offered version (used to go back to an older one). It is
    /// the same job as an update, so it gets the same size, SHA-256 and signature checks.
    fn job_for_version(&self, id: &str, version: &Version) -> Option<Job> {
        let feed = &self.updater.snapshot.as_ref()?.feed;
        let item = feed.item(id)?;
        let release = feed.release_for(item, version, self.cfg.include_prereleases)?;
        let file = release.file_for_this_platform()?.clone();
        let version = version.to_string();
        Some(Job { key: Job::key_for(id, &version), item_id: id.to_owned(), version, file, feed_url: self.updater.url().to_owned() })
    }

    /// "Go back" was confirmed: download that older version and install it like an update.
    pub(crate) fn start_rollback(&mut self, version: &str) {
        let job = Version::parse(version).ok().and_then(|v| self.job_for_version(views::APP_ID, &v));
        match job {
            Some(job) => {
                self.downloads.start(job);
                self.rollback = Some(version.to_owned());
                self.auto_install = Some((views::APP_ID.to_owned(), version.to_owned()));
            }
            None => self.notice = Some("The update list has no download of that version for this computer.".to_owned()),
        }
    }

    /// Update / Update and restart / Retry / UPDATE ALL: the only ways a download starts.
    fn start_download(&mut self, id: &str) {
        match self.job_for(id) {
            Some(job) => self.downloads.start(job),
            None => self.notice = Some("The update list has no download for this computer.".to_owned()),
        }
    }

    /// Whether the editor is open (it must be closed before an update can replace it). The
    /// process list is read at most once per second.
    fn editor_running(&self) -> bool {
        if let Some((at, running)) = self.editor_poll.get() {
            if at.elapsed() < Duration::from_secs(1) {
                return running;
            }
        }
        let name = editor_exe().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let running = install::process_running(&name);
        self.editor_poll.set(Some((Instant::now(), running)));
        running
    }

    /// Hand the verified MSI to the update helper and close the launcher so it can proceed.
    fn begin_install(&mut self, ctx: &egui::Context, msi: &Path, version: &str) {
        if self.shot.is_some() {
            return;
        }
        match install::start_helper(msi, version, &download::updates_dir(), Some(self.updater.url())) {
            Ok(()) => {
                self.tray = None; // remove the tray icon before the window closes
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => self.notice = Some(e),
        }
    }

    /// "Update and restart": install as soon as that download is ready (and the editor is closed).
    fn process_auto_install(&mut self, ctx: &egui::Context) {
        let Some((id, version)) = self.auto_install.clone() else { return };
        match self.downloads.status(&Job::key_for(&id, &version)) {
            Dl::Ready(path) => {
                if self.scope == InstallScope::NotInstalled {
                    self.auto_install = None;
                } else if self.editor_running() {
                    ctx.request_repaint_after(Duration::from_secs(1));
                } else {
                    self.auto_install = None;
                    self.begin_install(ctx, &path, &version);
                }
            }
            Dl::Failed(_) | Dl::Idle => self.auto_install = None,
            Dl::Queued | Dl::Running { .. } => {}
        }
    }

    /// "Download and install updates automatically": after a check, fetch a newer KernPage
    /// Studio in the background (never an add-on, a skipped or blocked version, or a downgrade).
    fn maybe_auto_download(&mut self) {
        if self.shot.is_some() {
            return;
        }
        let views = self.make_views();
        let Some(app) = views.iter().find(|v| v.is_app) else { return };
        let newer = app.latest.as_ref().filter(|l| app.state == State::Update && app.installed.as_ref().is_some_and(|i| l.version > *i));
        let plan = auto_install_plan(&AutoInstallInput {
            setting: self.cfg.auto_install,
            is_newer: newer.is_some(),
            is_downgrade: false, // a download here is always the newest offered version, never older
            scope: self.scope,
            editor_running: self.editor_running(),
            window_visible: self.window_visible,
        });
        let (true, Some(l)) = (plan.download, newer) else { return };
        let key = Job::key_for(&app.id, &l.version.to_string());
        if self.downloads.status(&key) == Dl::Idle {
            self.start_download(&app.id);
        }
        self.auto_pending = Some((app.id.clone(), l.version.to_string()));
    }

    /// Install a background-downloaded update, but only when it cannot get in anyone's way:
    /// the editor is closed and the launcher window is hidden. Otherwise the normal
    /// "Ready to install" card is left for the user to click.
    fn process_auto_pending(&mut self, ctx: &egui::Context) {
        let Some((id, version)) = self.auto_pending.clone() else { return };
        match self.downloads.status(&Job::key_for(&id, &version)) {
            Dl::Ready(path) => {
                let parsed = Version::parse(&version).ok();
                let plan = auto_install_plan(&AutoInstallInput {
                    setting: self.cfg.auto_install,
                    is_newer: parsed.as_ref().is_some_and(|v| *v > self.installed),
                    is_downgrade: parsed.as_ref().is_some_and(|v| *v < self.installed),
                    scope: self.scope,
                    editor_running: self.editor_running(),
                    window_visible: self.window_visible,
                });
                if plan.install {
                    self.auto_pending = None;
                    self.begin_install(ctx, &path, &version);
                } else if plan.download {
                    ctx.request_repaint_after(Duration::from_secs(3));
                } else {
                    self.auto_pending = None;
                }
            }
            Dl::Failed(_) | Dl::Idle => self.auto_pending = None,
            Dl::Queued | Dl::Running { .. } => {}
        }
    }

    fn dl_key(v: &ItemView) -> Option<String> {
        v.latest.as_ref().map(|l| Job::key_for(&v.id, &l.version.to_string()))
    }

    fn dl_status(&self, v: &ItemView) -> Dl {
        Self::dl_key(v).map(|k| self.downloads.status(&k)).unwrap_or(Dl::Idle)
    }

    /// The one-line state shown above the buttons (card and details panel).
    fn dl_line(&self, p: &Palette, v: &ItemView) -> Option<(String, Color32)> {
        self.dl_line_for(p, self.dl_status(v))
    }

    fn dl_line_for(&self, p: &Palette, status: Dl) -> Option<(String, Color32)> {
        match status {
            Dl::Idle => None,
            Dl::Queued => Some(("Waiting for another download to finish...".to_owned(), p.muted)),
            Dl::Running { done, total } => Some((format!("Downloading {} of {}", human_size(done), human_size(total)), p.muted)),
            Dl::Ready(_) if self.scope != InstallScope::NotInstalled && self.editor_running() => {
                Some(("Close KernPage to install this update".to_owned(), p.attn))
            }
            Dl::Ready(_) => Some(("Ready to install".to_owned(), p.ok)),
            Dl::Failed(m) => Some((m, p.err)),
        }
    }

    /// Progress bar + Cancel, the disabled install button, or Retry, in place of the Update button.
    fn dl_controls(&mut self, ui: &mut egui::Ui, p: &Palette, v: &ItemView, bar_width: f32) {
        let Some(l) = &v.latest else { return };
        self.dl_controls_for(ui, p, &v.id, &l.version.to_string(), false, bar_width);
    }

    /// Controls for the download of `version` of `id`; `rollback` = it is an older version.
    fn dl_controls_for(&mut self, ui: &mut egui::Ui, p: &Palette, id: &str, version: &str, rollback: bool, bar_width: f32) {
        let key = Job::key_for(id, version);
        match self.downloads.status(&key) {
            Dl::Idle => {}
            Dl::Queued => {
                if ui.button("Cancel").clicked() {
                    self.downloads.cancel(&key);
                }
            }
            Dl::Running { done, total } => {
                let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
                ui.add(egui::ProgressBar::new(frac.clamp(0.0, 1.0)).desired_width(bar_width).fill(p.accent));
                if ui.button("Cancel").on_hover_text("Stop downloading; the next Update resumes where this stopped").clicked() {
                    self.downloads.cancel(&key);
                }
            }
            Dl::Ready(path) => {
                let dev = self.scope == InstallScope::NotInstalled;
                let blocked = !dev && self.editor_running();
                let can = !dev && !blocked;
                if blocked {
                    ui.ctx().request_repaint_after(Duration::from_secs(1));
                }
                let b = Button::new(RichText::new("Install and restart").strong().color(if can { p.on_accent } else { p.muted }))
                    .min_size(vec2(96.0, 32.0));
                let b = if can { b.fill(p.accent) } else { b };
                let r = ui.add_enabled(can, b);
                let r = if dev {
                    r.on_disabled_hover_text("Installing works only in an installed copy")
                } else if blocked {
                    r.on_disabled_hover_text("Close KernPage to install this update")
                } else {
                    r.on_hover_text("Close the launcher, install the update and open it again")
                };
                if can && r.clicked() {
                    self.begin_install(&ui.ctx().clone(), &path, version);
                }
            }
            Dl::Failed(_) => {
                let b = Button::new(RichText::new("Retry").strong().color(p.on_accent)).fill(p.accent).min_size(vec2(96.0, 32.0));
                if ui.add(b).on_hover_text("Download again (resumes if possible)").clicked() {
                    if rollback {
                        self.start_rollback(version);
                    } else {
                        self.start_download(id);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn make_views_for_test(&self) -> Vec<ItemView> {
        self.make_views()
    }

    fn make_views(&self) -> Vec<ItemView> {
        views::build(
            self.updater.snapshot.as_ref().map(|s| &s.feed),
            &self.cfg,
            &self.installed,
            editor_exe().is_file(),
        )
    }

    /// Collect a finished check: remember the serial, show the notice on a failed manual
    /// check, and announce a newly found update (dialog and toast).
    fn process_updates(&mut self) {
        let Some(outcome) = self.updater.poll() else { return };
        let by_user = std::mem::take(&mut self.check_user);
        match outcome {
            Outcome::Failed => {
                if by_user {
                    self.notice = self.updater.error.as_ref().map(|e| e.to_string());
                }
            }
            Outcome::Updated { serial } => {
                if serial > self.cfg.last_serial {
                    self.cfg.last_serial = serial;
                    self.cfg.save();
                }
                self.announce_update();
                self.maybe_auto_download();
            }
        }
    }

    fn announce_update(&mut self) {
        let views = self.make_views();
        let Some(app) = views.iter().find(|v| v.is_app && v.state == State::Update) else { return };
        let Some(latest) = &app.latest else { return };
        let key = format!("{}@{}", app.id, latest.version);
        if self.shot.is_some() {
            if self.force_announce {
                self.announce = Some(app.id.clone());
            }
            return; // screenshot runs never toast or touch the saved "notified" state
        }
        if !self.snoozed.contains(&key) {
            self.announce = Some(app.id.clone());
        }
        if self.cfg.notify && self.cfg.last_notified != key {
            notify::update_available(&app.name, &latest.version.to_string());
            self.cfg.last_notified = key;
            self.cfg.save();
        }
    }

    fn open_editor(&mut self, file: Option<&Path>) {
        match launch_editor(file) {
            Ok(()) => self.notice = None,
            Err(e) => self.notice = Some(e),
        }
    }

    fn open_file_dialog(&mut self) {
        #[cfg(test)]
        let picked = crate::testhooks::with(|f| {
            f.picks_asked += 1;
            f.picks.pop_front().expect("unscripted Open file dialog")
        });
        #[cfg(not(test))]
        let picked = rfd::FileDialog::new()
            .set_title("Open Publication")
            .add_filter("All KernPage and Publisher files (*.kern;*.pub)", &["kern", "pub"])
            .add_filter("KernPage files (*.kern)", &["kern"])
            .add_filter("Publisher files (*.pub)", &["pub"])
            .add_filter("All files", &["*"])
            .pick_file();
        if let Some(p) = picked {
            self.open_editor(Some(&p));
        }
    }

    fn installed_date() -> Option<String> {
        std::fs::metadata(editor_exe()).ok()?.modified().ok().map(|t: SystemTime| date_text(t))
    }

    // ---- pieces ---------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal_centered(|ui| {
            ui.add_space(8.0);
            let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            mark::paint(ui.painter(), r, mark::TEAL);
            ui.add_space(4.0);
            ui.label(RichText::new("KERNPAGE").size(19.0).strong());
            ui.label(RichText::new("LAUNCHER").monospace().size(12.0).color(p.muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                let gear = ui.add(Button::new("").min_size(vec2(36.0, 32.0)));
                paint_gear(ui.painter(), gear.rect.center(), 8.0, p.text);
                if gear.on_hover_text("Settings").clicked() {
                    self.settings = true;
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, p: &Palette, views: &[ItemView]) {
        let updates = views::update_count(views);
        ui.horizontal(|ui| {
            section_label(ui, p, "DASHBOARD");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Refresh").on_hover_text("Reload the recent files list and check for updates").clicked() {
                    self.check_now_by_user();
                }
                if ui.button("Open file...").on_hover_text("Open a publication in KernPage").clicked() {
                    self.open_file_dialog();
                }
                if updates > 0 {
                    let b = Button::new(RichText::new(format!("UPDATE ALL ({updates})")).strong().color(p.on_accent)).fill(p.accent);
                    if ui.add(b).on_hover_text("Download every update, one after another").clicked() {
                        for v in views.iter().filter(|v| v.state == State::Update) {
                            self.start_download(&v.id);
                        }
                    }
                }
                if self.updater.is_checking() {
                    ui.spinner();
                    ui.label(RichText::new("Checking...").color(p.muted));
                }
            });
        });
    }

    fn notice_bar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let Some(msg) = self.notice.clone() else { return };
        egui::Frame::new()
            .fill(p.err_bg)
            .stroke(Stroke::new(1.0, p.err))
            .corner_radius(6)
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.scope(|ui| {
                        ui.set_max_width((ui.available_width() - 90.0).max(120.0));
                        // wrap(): a label in a horizontal row would otherwise run past the window.
                        ui.add(egui::Label::new(msg).wrap());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Dismiss").clicked() {
                            self.notice = None;
                        }
                    });
                });
            });
    }

    /// How the last install went, shown once after the launcher starts again.
    fn report_bar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let Some(r) = self.report.clone() else { return };
        let (text, fill, edge) = if r.ok {
            let mut t = format!("KernPage {} is installed", r.target_version);
            if r.exit_code == Some(3010) {
                t.push_str(". Restart Windows to finish the update.");
            }
            (t, p.ok_bg, p.ok)
        } else {
            (r.message.clone(), p.err_bg, p.err)
        };
        egui::Frame::new().fill(fill).stroke(Stroke::new(1.0, edge)).corner_radius(6).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    ui.set_max_width((ui.available_width() - 190.0).max(120.0));
                    ui.add(egui::Label::new(text).wrap());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Dismiss").clicked() {
                        self.report = None;
                    }
                    if !r.ok && !r.log_path.is_empty() && ui.button("Show log").on_hover_text("Open the installer log in Notepad").clicked() {
                        install::show_log(&r.log_path);
                    }
                });
            });
        });
    }

    fn show_details(&mut self, idx: usize) {
        self.details = true;
        self.details_idx = idx;
        self.show_all_changes = false;
    }

    fn browse(&mut self, url: &str) {
        if let Err(e) = open_url(url) {
            self.notice = Some(e);
        }
    }

    /// One card: a header strip (version, badge), name and tagline, facts, buttons.
    fn item_card(&mut self, ui: &mut egui::Ui, p: &Palette, idx: usize, v: &ItemView) {
        let size = vec2(400.0, 250.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        #[cfg(test)]
        crate::testhooks::seen(|f| f.cards.push((idx, rect)));
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        let fill = if resp.hovered() { p.card_hover } else { p.card };
        let stroke = if resp.has_focus() {
            Stroke::new(2.0, p.accent)
        } else if resp.hovered() || v.state == State::Update {
            Stroke::new(1.0, if v.state == State::Update { p.attn } else { p.accent })
        } else {
            Stroke::new(1.0, p.border)
        };
        ui.painter().rect_filled(rect, 10.0, fill);
        // Header strip.
        let strip = egui::Rect::from_min_size(rect.min, vec2(size.x, 36.0));
        let strip_fill = if v.state == State::Update { p.attn_bg } else { p.bg };
        ui.painter().rect_filled(strip, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, strip_fill);
        ui.painter().line_segment([strip.left_bottom(), strip.right_bottom()], Stroke::new(1.0, p.border));
        ui.painter().rect_stroke(rect, 10.0, stroke, StrokeKind::Inside);
        if resp.clicked() {
            self.show_details(idx);
        }
        {
            let mut s = ui.new_child(UiBuilder::new().max_rect(strip.shrink2(vec2(16.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
            let new = v.latest.as_ref().map(|l| l.version.to_string());
            match (v.state, &v.installed, &new) {
                (State::Update, Some(old), Some(new)) => {
                    s.label(RichText::new(old.to_string()).monospace().size(13.0).color(p.muted));
                    s.label(RichText::new("\u{2192}").monospace().size(13.0).color(p.muted));
                    s.label(RichText::new(new).monospace().size(13.0).strong().color(p.accent));
                }
                (_, Some(old), _) => {
                    s.label(RichText::new(format!("VERSION {old}")).monospace().size(12.0).color(p.muted));
                }
                (_, None, Some(new)) => {
                    s.label(RichText::new(format!("VERSION {new}")).monospace().size(12.0).color(p.muted));
                }
                _ => {}
            }
            s.with_layout(Layout::right_to_left(Align::Center), |s| status_badge(s, p, v.state, v.skipped));
        }
        // Body.
        let mut body = ui.new_child(
            UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(rect.min + vec2(16.0, 36.0 + 12.0), rect.max - vec2(16.0, 60.0)))
                .layout(Layout::top_down(Align::Min)),
        );
        let ui = &mut body;
        // Selectable labels would swallow clicks: the whole card (name and text too) opens the details.
        ui.style_mut().interaction.selectable_labels = false;
        ui.horizontal(|ui| {
            icon_tile(ui, p, v, 52.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.add(egui::Label::new(RichText::new(&v.name).size(17.0).strong()).truncate());
                ui.add(egui::Label::new(RichText::new(&v.tagline).color(p.muted).size(12.5)));
            });
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            match (&v.latest, v.state) {
                (Some(l), State::Update | State::Available) => {
                    fact(ui, p, "DOWNLOAD", &l.size.map(human_size).unwrap_or_else(|| "-".into()));
                    fact(ui, p, "LAST UPDATED", &l.released);
                }
                _ if v.is_app => {
                    if let Some(d) = Self::installed_date() {
                        fact(ui, p, "INSTALLED", &d);
                    }
                }
                _ => {}
            }
        });
        if let Some(n) = &v.note {
            let c = if v.state == State::Blocked { p.err } else { p.attn };
            ui.add(egui::Label::new(RichText::new(n).size(12.0).color(c)));
        }
        if let Some((t, c)) = self.dl_line(p, v) {
            ui.add(egui::Label::new(RichText::new(t).size(12.0).color(c)));
        }
        // Buttons.
        let brect = egui::Rect::from_min_size(egui::pos2(rect.left() + 16.0, rect.bottom() - 16.0 - 32.0), vec2(size.x - 32.0, 32.0));
        let mut b = ui_child_in(ui, brect);
        let ui = &mut b;
        if v.is_app {
            let label = match &v.installed {
                Some(ver) if v.state == State::Update => format!("OPEN {ver}"),
                _ => "OPEN".to_owned(),
            };
            let primary = v.state != State::Update;
            let open = if primary {
                Button::new(RichText::new(label).strong().color(p.on_accent)).fill(p.accent)
            } else {
                Button::new(RichText::new(label).strong())
            }
            .min_size(vec2(96.0, 32.0));
            if ui.add(open).on_hover_text("Open KernPage").clicked() {
                self.open_editor(None);
            }
            if let (State::Update, Some(l)) = (v.state, &v.latest) {
                let b = Button::new(RichText::new(format!("UPDATE TO {}", l.version)).strong().color(p.on_accent))
                    .fill(p.accent)
                    .min_size(vec2(96.0, 32.0));
                if self.dl_status(v) != Dl::Idle {
                    self.dl_controls(ui, p, v, 110.0);
                } else if ui.add(b).on_hover_text("Download the new version").clicked() {
                    self.start_download(&v.id);
                }
            }
        } else {
            let enabled = v.state == State::Available;
            let b = Button::new(RichText::new("INSTALL").strong().color(if enabled { p.on_accent } else { p.muted }))
                .fill(if enabled { p.accent } else { p.card })
                .min_size(vec2(96.0, 32.0));
            if self.dl_status(v) != Dl::Idle {
                self.dl_controls(ui, p, v, 110.0);
            } else {
                let r = ui.add_enabled(enabled, b);
                if r.on_hover_text("Installing add-ons arrives in a later step").clicked() {
                    self.stub = Some(Stub::Install { name: v.name.clone() });
                }
            }
        }
        self.more_menu(ui, idx, v);
    }

    /// The "..." menu shared by the card and the details header.
    fn more_menu(&mut self, ui: &mut egui::Ui, idx: usize, v: &ItemView) {
        ui.menu_button("...", |ui| {
            if ui.button("What's new").clicked() {
                self.show_details(idx);
                ui.close();
            }
            let has_url = v.releases_url.is_some();
            if ui.add_enabled(has_url, Button::new("Release page")).clicked() {
                if let Some(u) = &v.releases_url {
                    self.browse(u);
                }
                ui.close();
            }
            if ui.add_enabled(has_url, Button::new("Copy link")).clicked() {
                if let Some(u) = &v.releases_url {
                    ui.ctx().copy_text(u.clone());
                }
                ui.close();
            }
            if v.is_app {
                if let Some(l) = &v.latest {
                    let ver = l.version.to_string();
                    if v.skipped {
                        if ui.button(format!("Stop skipping {ver}")).clicked() {
                            self.cfg.unskip(&v.id, &ver);
                            self.cfg.save();
                            ui.close();
                        }
                    } else if v.state == State::Update && ui.button("Skip this version").clicked() {
                        self.cfg.skip(&v.id, &ver);
                        self.cfg.save();
                        ui.close();
                    }
                }
                if ui.button("Open file location").clicked() {
                    if let Err(e) = open_location() {
                        self.notice = Some(e);
                    }
                    ui.close();
                }
                ui.add_enabled(false, Button::new("Reinstall")).on_disabled_hover_text(NEEDS_INSTALL);
                let older = self.older_candidates(v);
                let r = ui.add_enabled(!older.is_empty(), Button::new("Roll back"));
                let why = if self.updater.snapshot.is_none() { "Check for updates first" } else { "No older versions are available" };
                if r.on_disabled_hover_text(why).clicked() {
                    self.rollback_list = true;
                    ui.close();
                }
                ui.add_enabled(false, Button::new("Uninstall")).on_disabled_hover_text(NEEDS_INSTALL);
            } else if ui.add_enabled(v.website.is_some(), Button::new("Website")).clicked() {
                if let Some(u) = &v.website {
                    self.browse(u);
                }
                ui.close();
            }
        });
    }

    fn recent_section(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            section_label(ui, p, "RECENT FILES");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has = !self.recent_rows.is_empty();
                if ui.add_enabled(has, Button::new("Clear list")).clicked() {
                    self.confirm_clear = true;
                }
            });
        });
        if self.recent_rows.is_empty() {
            empty_state(ui, p, "No recent files yet. Files you open or save in KernPage appear here.");
            return;
        }
        let mut open: Option<PathBuf> = None;
        let mut remove: Option<PathBuf> = None;
        for row in &self.recent_rows {
            let missing = row.modified.is_none();
            ui.add_space(4.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
            let resp = if missing {
                resp.on_hover_text("Not found. Remove it from the list, or restore the file.")
            } else {
                resp.on_hover_text(row.path.to_string_lossy()).on_hover_cursor(CursorIcon::PointingHand)
            };
            #[cfg(test)]
            crate::testhooks::seen(|f| f.rows.push((row.name.clone(), row.folder.clone(), row.modified.clone().unwrap_or_else(|| "NOT FOUND".to_owned()), rect)));
            let fill = if resp.hovered() && !missing { p.card_hover } else { p.card };
            let stroke = if resp.has_focus() { Stroke::new(2.0, p.accent) } else { Stroke::new(1.0, p.border) };
            ui.painter().rect_filled(rect, 8.0, fill);
            ui.painter().rect_stroke(rect, 8.0, stroke, StrokeKind::Inside);
            let (main, sub) = if missing { (p.muted, p.muted.gamma_multiply(0.8)) } else { (p.text, p.muted) };
            let painter = ui.painter();
            let right = rect.right() - 56.0;
            let (date_text, date_color) = match &row.modified {
                Some(d) => (format!("Modified {d}"), p.muted),
                None => ("NOT FOUND".to_owned(), Color32::from_rgb(0xC2, 0x4B, 0x45)),
            };
            let date = painter.layout_no_wrap(date_text, FontId::monospace(12.0), date_color);
            // Long names and folders are cut with "..." instead of running under the date.
            let name_w = (rect.width() - 16.0 - 56.0 - date.size().x - 16.0).max(60.0);
            let folder_w = (rect.width() - 16.0 - 56.0).max(60.0);
            let name = fit_text(painter, &row.name, FontId::proportional(15.0), main, name_w);
            let folder = fit_text(painter, &row.folder, FontId::proportional(12.0), sub, folder_w);
            painter.galley(rect.min + vec2(16.0, 10.0), name, main);
            painter.galley(rect.min + vec2(16.0, 30.0), folder, sub);
            painter.galley(egui::pos2(right - date.size().x, rect.center().y - date.size().y / 2.0), date, date_color);
            let x_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 28.0, rect.center().y), vec2(30.0, 30.0));
            let x = ui.put(x_rect, Button::new(RichText::new("\u{00D7}").size(18.0)).frame(false));
            if x.on_hover_text("Remove from list").clicked() {
                remove = Some(row.path.clone());
            } else if resp.clicked() && !missing {
                open = Some(row.path.clone());
            }
        }
        if let Some(path) = remove {
            let mut r = recent_all();
            r.remove(&path);
            self.refresh();
        }
        if let Some(path) = open {
            self.open_editor(Some(&path));
        }
    }

    fn available_section(&mut self, ui: &mut egui::Ui, p: &Palette, views: &[ItemView]) {
        section_label(ui, p, "AVAILABLE TO INSTALL");
        if views.iter().all(|v| v.is_app) {
            empty_state(ui, p, "Add-ons such as template packs and dictionaries appear here after an update check.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, v) in views.iter().enumerate().filter(|(_, v)| !v.is_app) {
                self.item_card(ui, p, i, v);
            }
        });
    }

    // ---- modals ---------------------------------------------------------------------

    fn details_modal(&mut self, ctx: &egui::Context, p: &Palette, views: &[ItemView]) {
        let idx = self.details_idx.min(views.len().saturating_sub(1));
        let Some(v) = views.get(idx) else {
            self.details = false;
            return;
        };
        let margin = 24;
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.bg).inner_margin(Margin::same(margin));
        let width = fit_width(ctx, 880.0, margin);
        let modal = modal_for(ctx, egui::Id::new("details")).frame(frame).show(ctx, |ui| {
            ui.set_width(width);
            egui::ScrollArea::vertical().id_salt("details-body").auto_shrink([false, true]).max_height(fit_height(ctx, margin, 0.0)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    icon_tile(ui, p, v, 84.0);
                    ui.vertical(|ui| {
                        ui.set_max_width((width - 84.0 - 140.0).max(160.0));
                        ui.spacing_mut().item_spacing.y = 5.0;
                        ui.label(RichText::new(&v.name).size(26.0).strong());
                        ui.label(RichText::new(&v.tagline).color(p.muted));
                        let version = match (&v.installed, &v.latest) {
                            (Some(old), Some(l)) if v.state == State::Update => format!("Version {old} \u{2192} {} \u{00B7} {}", l.version, l.released),
                            (Some(old), _) => match Self::installed_date() {
                                Some(d) => format!("Version {old} \u{00B7} {d}"),
                                None => format!("Version {old}"),
                            },
                            (None, Some(l)) => format!("Version {} \u{00B7} {}", l.version, l.released),
                            _ => String::new(),
                        };
                        ui.label(RichText::new(version).monospace().size(12.5).color(p.muted));
                        if let Some((t, c)) = self.dl_line(p, v) {
                            ui.label(RichText::new(t).size(12.5).color(c));
                        }
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            if v.is_app {
                                let label = match (&v.installed, v.state) {
                                    (Some(ver), State::Update) => format!("OPEN {ver}"),
                                    _ => "OPEN".to_owned(),
                                };
                                let open = if v.state == State::Update {
                                    Button::new(RichText::new(label).strong())
                                } else {
                                    Button::new(RichText::new(label).strong().color(p.on_accent)).fill(p.accent)
                                }
                                .min_size(vec2(96.0, 32.0));
                                if ui.add(open).clicked() {
                                    self.open_editor(None);
                                }
                                if let (State::Update, Some(l)) = (v.state, &v.latest) {
                                    let b = Button::new(RichText::new(format!("UPDATE TO {}", l.version)).strong().color(p.on_accent)).fill(p.accent).min_size(vec2(96.0, 32.0));
                                    if self.dl_status(v) != Dl::Idle {
                                        self.dl_controls(ui, p, v, 220.0);
                                    } else if ui.add(b).clicked() {
                                        self.start_download(&v.id);
                                    }
                                }
                            } else {
                                let enabled = v.state == State::Available;
                                let b = Button::new(RichText::new("INSTALL").strong().color(if enabled { p.on_accent } else { p.muted }))
                                    .fill(if enabled { p.accent } else { p.card })
                                    .min_size(vec2(96.0, 32.0));
                                if self.dl_status(v) != Dl::Idle {
                                    self.dl_controls(ui, p, v, 220.0);
                                } else if ui.add_enabled(enabled, b).clicked() {
                                    self.stub = Some(Stub::Install { name: v.name.clone() });
                                }
                            }
                            self.more_menu(ui, idx, v);
                            for (label, url) in [("Website", &v.website), ("GitHub", &v.repo), ("Releases", &v.releases_url)] {
                                let r = ui.add_enabled(url.is_some(), Button::new(label));
                                let r = if url.is_none() { r.on_disabled_hover_text("Not provided by the update list") } else { r };
                                if r.clicked() {
                                    if let Some(u) = url {
                                        self.browse(u);
                                    }
                                }
                            }
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        if ui.button("\u{00D7}").on_hover_text("Close (Esc)").clicked() {
                            self.details = false;
                        }
                        if ui.add_enabled(idx + 1 < views.len(), Button::new("\u{203A}")).on_hover_text("Next").clicked() {
                            self.show_details(idx + 1);
                        }
                        if ui.add_enabled(idx > 0, Button::new("\u{2039}")).on_hover_text("Previous").clicked() {
                            self.show_details(idx - 1);
                        }
                    });
                });
                if !v.description.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&v.description).color(p.muted));
                }
                if let Some(n) = &v.note {
                    ui.add_space(6.0);
                    ui.label(RichText::new(n).size(12.5).color(if v.state == State::Blocked { p.err } else { p.attn }));
                }
                ui.add_space(14.0);
                if width >= 760.0 {
                    ui.columns(2, |cols| {
                        self.details_features(&mut cols[0], p, v);
                        self.details_release(&mut cols[1], p, v);
                    });
                } else {
                    self.details_features(ui, p, v);
                    ui.add_space(14.0);
                    self.details_release(ui, p, v);
                }
            });
        });
        if modal.should_close() {
            self.details = false;
        }
    }

    /// The details panel's "01 / FEATURES" column.
    fn details_features(&mut self, ui: &mut egui::Ui, p: &Palette, v: &ItemView) {
        section_label(ui, p, "01 / FEATURES");
        if v.features.is_empty() {
            ui.label(RichText::new("No feature list was provided.").color(p.muted));
        }
        for (i, (title, text)) in v.features.iter().enumerate() {
            card_frame(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{:02}", i + 1)).monospace().color(p.accent));
                    ui.label(RichText::new(title).strong());
                });
                ui.label(RichText::new(text).color(p.muted));
            });
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "KernPage is an independent project. It is not affiliated with, endorsed by or sponsored by Microsoft. Microsoft, Publisher, Office and Windows are trademarks of the Microsoft group of companies. Privacy: nothing is sent unless you check for updates (PRIVACY.md).",
            )
            .small()
            .color(p.muted),
        );
    }

    /// The details panel's "02 / LATEST RELEASE" column: info boxes and what changed.
    fn details_release(&mut self, ui: &mut egui::Ui, p: &Palette, v: &ItemView) {
        section_label(ui, p, "02 / LATEST RELEASE");
        let version = v.latest.as_ref().map(|l| l.version.to_string()).or_else(|| v.installed.as_ref().map(|i| i.to_string())).unwrap_or_else(|| "-".into());
        let released = v.latest.as_ref().map(|l| l.released.clone()).or_else(Self::installed_date).unwrap_or_else(|| "Unknown".into());
        let download = v.latest.as_ref().and_then(|l| l.size).map(human_size).unwrap_or_else(|| "Not checked yet".into());
        let platforms = match &v.latest {
            Some(l) if !l.platforms.is_empty() => l.platforms.clone(),
            _ => "Windows 64-bit".to_owned(),
        };
        let per_row = if ui.available_width() >= 340.0 { 2 } else { 1 };
        let tile_w = (ui.available_width() - 8.0 * (per_row as f32 - 1.0)) / per_row as f32;
        let tiles = [("VERSION", &version), ("RELEASED", &released), ("DOWNLOAD", &download), ("PLATFORMS", &platforms)];
        for row in tiles.chunks(per_row) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (label, value) in row {
                    tile(ui, p, label, value, tile_w);
                }
            });
        }
        ui.add_space(6.0);
        section_label(ui, p, "WHAT'S CHANGED");
        {
            card_frame(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                match &v.latest {
                    None => {
                        ui.label(RichText::new("Check for updates to see what's changed.").color(p.muted));
                    }
                    Some(l) if l.changes.is_empty() => {
                        ui.label(RichText::new("This release has no notes.").color(p.muted));
                    }
                    Some(l) => {
                        const FIRST: usize = 6;
                        let shown = if self.show_all_changes { l.changes.len() } else { FIRST.min(l.changes.len()) };
                        for c in &l.changes[..shown] {
                            change_row(ui, p, c);
                        }
                        let more = l.changes.len() - shown;
                        if more > 0 && ui.button(format!("Show {more} more changes")).clicked() {
                            self.show_all_changes = true;
                        } else if self.show_all_changes && l.changes.len() > FIRST && ui.button("Show fewer").clicked() {
                            self.show_all_changes = false;
                        }
                    }
                }
            });
        }
        if let Some(url) = v.latest.as_ref().and_then(|l| l.changelog_url.clone()) {
            if ui.link("Full changelog").clicked() {
                self.browse(&url);
            }
        }
    }

    /// Releases older than the installed one that the update list still offers (version, date),
    /// newest first.
    fn older_candidates(&self, v: &ItemView) -> Vec<(String, String)> {
        let Some(installed) = v.installed.clone() else { return Vec::new() };
        self.updater
            .snapshot
            .as_ref()
            .and_then(|s| {
                let item = s.feed.item(&v.id)?;
                Some(
                    s.feed
                        .rollback_candidates(item, &installed, self.cfg.include_prereleases)
                        .into_iter()
                        .map(|c| (c.version.to_string(), feed::pretty_date(&c.release.released)))
                        .collect(),
                )
            })
            .unwrap_or_default()
    }

    /// "Go back to an older version": the list behind the "..." menu's Roll back item; once a
    /// version is chosen and confirmed it shows that download's progress.
    fn rollback_list_modal(&mut self, ctx: &egui::Context, p: &Palette, views: &[ItemView]) {
        let Some(v) = views.iter().find(|v| v.is_app) else {
            self.rollback_list = false;
            return;
        };
        let older = self.older_candidates(v);
        let active = self.rollback.clone().filter(|ver| self.downloads.status(&Job::key_for(&v.id, ver)) != Dl::Idle);
        let margin = 20;
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.card).inner_margin(Margin::same(margin));
        let mut close = false;
        let modal = modal_for(ctx, egui::Id::NULL.with("rollback-list")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 480.0, margin));
            ui.label(RichText::new("Go back to an older version").size(17.0).strong());
            ui.label(RichText::new("Go back if a newer version is giving you trouble. Your publications are not changed.").color(p.muted));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("older").auto_shrink([false, true]).max_height(fit_height(ctx, margin, 140.0)).show(ui, |ui| {
                if let Some(ver) = &active {
                    ui.label(format!("Going back to version {ver}"));
                    if let Some((t, c)) = self.dl_line_for(p, self.downloads.status(&Job::key_for(&v.id, ver))) {
                        ui.label(RichText::new(t).size(12.5).color(c));
                    }
                    ui.horizontal_wrapped(|ui| self.dl_controls_for(ui, p, &v.id, ver, true, 220.0));
                } else if older.is_empty() {
                    ui.label(RichText::new("No older versions are available.").color(p.muted));
                } else {
                    for (ver, released) in &older {
                        card_frame(p).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(format!("Version {ver} \u{00B7} {released}")).monospace().size(12.5));
                                if ui.button("Go back to this version").clicked() {
                                    self.rollback_confirm = Some(ver.clone());
                                }
                            });
                        });
                    }
                }
            });
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(Button::new("Close").min_size(vec2(90.0, 32.0))).clicked() {
                    close = true;
                }
            });
        });
        if close || modal.should_close() {
            self.rollback_list = false;
        }
    }

    /// "Go back to KernPage X?" before anything is downloaded.
    fn rollback_modal(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(ver) = self.rollback_confirm.clone() else { return };
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.card).inner_margin(Margin::same(20));
        let mut close = false;
        let modal = modal_for(ctx, egui::Id::unique("confirm-rollback")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 400.0, 20));
            ui.label(RichText::new(format!("Go back to KernPage {ver}?")).size(17.0).strong());
            ui.label(RichText::new("Your publications are not changed.").color(p.muted));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let b = Button::new(RichText::new("Go back and restart").strong().color(p.on_accent)).fill(p.accent);
                    if ui.add(b).clicked() {
                        self.start_rollback(&ver);
                        close = true;
                    }
                });
            });
        });
        if close || modal.should_close() {
            self.rollback_confirm = None;
        }
    }

    /// "<App> X is available": What's new / Later / Update and restart.
    fn announce_modal(&mut self, ctx: &egui::Context, p: &Palette, views: &[ItemView]) {
        let Some(v) = views.iter().find(|v| Some(&v.id) == self.announce.as_ref()) else {
            self.announce = None;
            return;
        };
        let (Some(l), Some(old)) = (&v.latest, &v.installed) else {
            self.announce = None;
            return;
        };
        let key = format!("{}@{}", v.id, l.version);
        let idx = views.iter().position(|x| x.id == v.id).unwrap_or(0);
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.card).inner_margin(Margin::same(22));
        let mut later = false;
        let modal = modal_for(ctx, egui::Id::new("announce")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 470.0, 22));
            ui.horizontal(|ui| {
                icon_tile(ui, p, v, 56.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new(format!("{} {} is available", v.name, l.version)).size(19.0).strong());
                    ui.label(RichText::new(format!("You have {old}. Released {}.", l.released)).color(p.muted));
                });
            });
            ui.add_space(8.0);
            for c in l.changes.iter().take(3) {
                change_row(ui, p, c);
            }
            if l.changes.len() > 3 {
                ui.label(RichText::new(format!("and {} more changes", l.changes.len() - 3)).color(p.muted));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("What's new").clicked() {
                    self.show_details(idx);
                    later = true;
                }
                if ui.button("Later").on_hover_text("Ask again next time the launcher starts").clicked() {
                    later = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let b = Button::new(RichText::new("Update and restart").strong().color(p.on_accent)).fill(p.accent).min_size(vec2(150.0, 32.0));
                    if ui.add(b).clicked() {
                        self.start_download(&v.id);
                        self.auto_install = Some((v.id.clone(), l.version.to_string()));
                        later = true;
                    }
                });
            });
        });
        if later || modal.should_close() {
            self.snoozed.insert(key);
            self.announce = None;
        }
    }

    /// Installing is the next step; say so plainly rather than pretend.
    fn stub_modal(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(stub) = &self.stub else { return };
        let (title, body) = match stub {
            Stub::Install { name } => (format!("Install {name}"), "Installing add-ons arrives in a later step of the launcher. Nothing was downloaded or changed.".to_owned()),
        };
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.card).inner_margin(Margin::same(20));
        let modal = modal_for(ctx, egui::Id::new("stub")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 420.0, 20));
            ui.label(RichText::new(title).size(17.0).strong());
            ui.label(RichText::new("NOT BUILT YET").monospace().size(11.0).color(p.attn));
            ui.label(RichText::new(body).color(p.muted));
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let b = Button::new(RichText::new("OK").strong().color(p.on_accent)).fill(p.accent).min_size(vec2(90.0, 32.0));
                if ui.add(b).clicked() {
                    self.stub = None;
                }
            });
        });
        if modal.should_close() {
            self.stub = None;
        }
    }

    fn settings_modal(&mut self, ctx: &egui::Context, p: &Palette) {
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.bg).inner_margin(Margin::same(24));
        let modal = modal_for(ctx, egui::Id::new("settings")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 560.0, 24));
            ui.label(RichText::new("Settings").size(22.0).strong());
            ui.add_space(6.0);
            egui::ScrollArea::vertical().auto_shrink([false, true]).max_height(fit_height(ctx, 24, 120.0).min(560.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let mut changed = false;

                section_label(ui, p, "LANGUAGE");
                egui::ComboBox::from_id_salt("language")
                    .selected_text("Auto (system language)")
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "Auto (system language)");
                    });
                note(ui, p, "More languages will come later.");

                ui.add_space(6.0);
                section_label(ui, p, "UPDATES");
                ui.horizontal(|ui| {
                    ui.label("Check for updates");
                    for (v, l) in [(CheckMode::Automatic, "Automatically"), (CheckMode::Manual, "Manually")] {
                        changed |= ui.radio_value(&mut self.cfg.check_updates, v, l).changed();
                    }
                });
                if self.cfg.check_updates == CheckMode::Automatic {
                    ui.horizontal(|ui| {
                        ui.label("Check every");
                        egui::ComboBox::from_id_salt("hours")
                            .selected_text(format!("{} hours", self.cfg.check_hours))
                            .show_ui(ui, |ui| {
                                for h in [1u32, 3, 6, 12, 24] {
                                    changed |= ui.selectable_value(&mut self.cfg.check_hours, h, format!("{h} hours")).changed();
                                }
                            });
                    });
                } else {
                    note(ui, p, "Manual: nothing is sent anywhere until you press Refresh or Check now.");
                }
                changed |= ui.checkbox(&mut self.cfg.auto_install, "Download and install updates automatically").changed();
                changed |= ui.checkbox(&mut self.cfg.notify, "Notify me when an update is available").changed();
                changed |= ui.checkbox(&mut self.cfg.include_prereleases, "Include pre-releases").changed();
                ui.add_enabled_ui(false, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Version");
                        egui::ComboBox::from_id_salt("version")
                            .selected_text(format!("{VERSION} (installed)"))
                            .show_ui(ui, |_| {});
                    });
                })
                .response
                .on_hover_text(NEEDS_INSTALL);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.updater.is_checking(), Button::new("Check now")).clicked() {
                        self.check_now_by_user();
                    }
                    if self.updater.is_checking() {
                        ui.spinner();
                    }
                });
                note(ui, p, "Installing updates and choosing an older version arrive in the next step.");

                ui.add_space(6.0);
                section_label(ui, p, "APPEARANCE");
                ui.horizontal(|ui| {
                    for (v, l) in [(Appearance::System, "Match system"), (Appearance::Dark, "Dark"), (Appearance::Light, "Light")] {
                        if ui.radio_value(&mut self.cfg.appearance, v, l).changed() {
                            self.theme_override = None;
                            changed = true;
                            self.apply_appearance(ui.ctx());
                        }
                    }
                });

                ui.add_space(6.0);
                section_label(ui, p, "BEHAVIOUR");
                let tray_ok = cfg!(windows);
                if ui
                    .add_enabled(tray_ok, egui::Checkbox::new(&mut self.cfg.keep_in_tray, "Keep running in the tray when closed"))
                    .changed()
                {
                    changed = true;
                    self.tray = if self.cfg.keep_in_tray { tray::create() } else { None };
                    if self.cfg.keep_in_tray && self.tray.is_none() {
                        self.cfg.keep_in_tray = false;
                        self.notice = Some("The tray icon could not be created, so the launcher will close normally.".into());
                    }
                }
                if ui
                    .add_enabled(tray_ok, egui::Checkbox::new(&mut self.cfg.start_at_login, "Start at login"))
                    .changed()
                {
                    changed = true;
                    let exe = std::env::current_exe().unwrap_or_default();
                    if let Err(e) = run_key::set_enabled(&mut run_key::RegistryRunStore, self.cfg.start_at_login, &exe) {
                        self.cfg.start_at_login = !self.cfg.start_at_login;
                        self.notice = Some(format!("Could not change the start at login setting: {e}"));
                    }
                }
                if !tray_ok {
                    note(ui, p, "These two options are available on Windows only.");
                }

                ui.add_space(6.0);
                section_label(ui, p, "COMMUNITY");
                ui.add_enabled(false, Button::new("Report a problem (GitHub issues)")).on_disabled_hover_text(NOT_PUBLIC);

                if changed {
                    self.cfg.save();
                }
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Quit launcher").clicked() {
                    quit_launcher();
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let done = Button::new(RichText::new("Done").strong().color(p.on_accent)).fill(p.accent).min_size(vec2(90.0, 32.0));
                    if ui.add(done).clicked() {
                        self.settings = false;
                    }
                });
            });
        });
        if modal.should_close() {
            self.settings = false;
        }
    }

    fn confirm_modal(&mut self, ctx: &egui::Context, p: &Palette) {
        let frame = egui::Frame::popup(&ctx.global_style()).fill(p.card).inner_margin(Margin::same(20));
        let modal = modal_for(ctx, egui::Id::new("confirm-clear")).frame(frame).show(ctx, |ui| {
            ui.set_width(fit_width(ctx, 380.0, 20));
            ui.label(RichText::new("Clear the recent files list?").size(17.0).strong());
            ui.label(RichText::new("Only the list is cleared. Your files are not touched.").color(p.muted));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.confirm_clear = false;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let b = Button::new(RichText::new("Clear list").strong().color(p.on_accent)).fill(p.accent);
                    if ui.add(b).clicked() {
                        recent_all().clear();
                        self.refresh();
                        self.confirm_clear = false;
                    }
                });
            });
        });
        if modal.should_close() {
            self.confirm_clear = false;
        }
    }

    /// Footer: (left text, right text, left colour).
    fn footer_text(&self, p: &Palette) -> (String, String, Color32) {
        let source = if feed::is_loopback_url(self.updater.url()) { "local test feed" } else { "GitHub" };
        let now = feed::now_unix();
        let mode = match self.cfg.check_updates {
            CheckMode::Automatic => format!("Automatic updates every {} hours", self.cfg.check_hours),
            CheckMode::Manual => "Manual updates: nothing is sent until you check".to_owned(),
        };
        let checked = self.updater.snapshot.as_ref().filter(|s| s.checked_at > 0).map(|s| ago(now - s.checked_at));
        if self.updater.is_checking() {
            return ("Checking for updates...".into(), mode, p.muted);
        }
        if let Some(e) = &self.updater.error {
            let tail = checked.map(|c| format!(" \u{00B7} last checked {c}")).unwrap_or_default();
            return (format!("Couldn't check: {}{tail}", e.short()), mode, p.err);
        }
        match checked {
            Some(c) => (format!("Checked {c} \u{00B7} using {source}"), mode, p.muted),
            None => (format!("Not checked yet \u{00B7} version {VERSION}"), mode, p.muted),
        }
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        shot.idle = if self.updater.is_checking() { 0 } else { shot.idle + 1 };
        ctx.request_repaint();
        if shot.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if shot.failed.load(Ordering::SeqCst) || shot.started.elapsed().as_secs() > 30 {
            eprintln!("screenshot failed");
            std::process::exit(3);
        }
        if shot.frames >= 10 && shot.idle >= 8 && !shot.requested {
            shot.requested = true;
            let path = shot.path.clone();
            let (done, failed) = (shot.done.clone(), shot.failed.clone());
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let mut rgba = Vec::with_capacity(w * h * 4);
                for px in &img.pixels {
                    rgba.extend_from_slice(&px.to_array());
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
}

// ---- small widgets ---------------------------------------------------------------------

/// A small gear: a ring with eight teeth (the default fonts have no gear glyph).
fn paint_gear(painter: &egui::Painter, c: egui::Pos2, r: f32, color: Color32) {
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let d = vec2(a.cos(), a.sin());
        painter.line_segment([c + d * (r * 0.8), c + d * (r * 1.3)], Stroke::new(r * 0.45, color));
    }
    painter.circle_stroke(c, r * 0.8, Stroke::new(r * 0.38, color));
}

/// A modal that may grow to the whole window (minus a 24 px margin): by default egui sizes a
/// new popup for 600x400, which would cap the height of a scrolling body.
fn modal_for(ctx: &egui::Context, id: egui::Id) -> egui::Modal {
    let r = ctx.content_rect();
    egui::Modal::new(id).area(egui::Modal::default_area(id).default_size(vec2(r.width() - 48.0, r.height() - 48.0)))
}

/// A modal's content width: its design width, but never wider than the window minus a 24 px
/// margin on each side (and the modal's own padding).
fn fit_width(ctx: &egui::Context, design: f32, padding: i8) -> f32 {
    design.min(ctx.content_rect().width() - 48.0 - 2.0 * f32::from(padding)).max(200.0)
}

/// The most height a modal's scrolling body may take: the window minus a 24 px margin on each
/// side, the modal's padding and `fixed` (the parts that stay above or below the body).
fn fit_height(ctx: &egui::Context, padding: i8, fixed: f32) -> f32 {
    (ctx.content_rect().height() - 48.0 - 2.0 * f32::from(padding) - fixed).max(100.0)
}

/// One line of text no wider than `max_width`, ending in "..." when it had to be cut.
fn fit_text(painter: &egui::Painter, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    painter.layout_job(job)
}

fn section_label(ui: &mut egui::Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).monospace().size(11.5).color(p.muted));
}

fn note(ui: &mut egui::Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(p.muted));
}

fn card_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(8)
        .inner_margin(Margin::same(12))
}

fn empty_state(ui: &mut egui::Ui, p: &Palette, text: &str) {
    egui::Frame::new()
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(16, 18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).color(p.muted));
        });
}

fn tile(ui: &mut egui::Ui, p: &Palette, label: &str, value: &str, width: f32) {
    card_frame(p).show(ui, |ui| {
        // The frame adds its 12 px padding and 1 px edge on both sides.
        ui.set_width((width - 26.0).max(60.0));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new(label).monospace().size(11.0).color(p.muted));
            ui.label(RichText::new(value).strong());
        });
    });
}

fn status_badge(ui: &mut egui::Ui, p: &Palette, s: State, skipped: bool) {
    let (text, fg, bg) = match s {
        State::Current if skipped => ("UPDATE SKIPPED", p.muted, p.bg),
        State::Current => ("INSTALLED", p.ok, p.ok_bg),
        State::Update => ("UPDATE AVAILABLE", p.attn, p.attn_bg),
        State::Missing => ("NOT FOUND", p.err, p.err_bg),
        State::Available => ("NOT INSTALLED", p.muted, p.card_hover),
        State::Blocked => ("BLOCKED", p.err, p.err_bg),
    };
    #[cfg(test)]
    crate::testhooks::seen(|f| f.badges.push(text.to_owned()));
    // A fixed-height pill, centred in the row, so it never fills the header strip.
    let galley = ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::monospace(10.5), fg);
    let size = vec2(galley.size().x + 14.0, 20.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    let r = egui::Rect::from_center_size(r.center(), size);
    ui.painter().rect_filled(r, 10.0, bg);
    ui.painter().rect_stroke(r, 10.0, Stroke::new(1.0, fg.gamma_multiply(0.35)), StrokeKind::Inside);
    let pos = r.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, fg);
}

fn fact(ui: &mut egui::Ui, p: &Palette, label: &str, value: &str) {
    ui.label(RichText::new(label).monospace().size(11.0).color(p.muted));
    ui.label(RichText::new(value).size(13.0).strong());
}

/// A child `Ui` in `rect` with a left-to-right layout (the card's button row).
fn ui_child_in(ui: &mut egui::Ui, rect: egui::Rect) -> egui::Ui {
    ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)))
}

/// The KernPage mark for the app; a lettered tile for add-ons (icons are not downloaded in
/// this step: nothing is fetched except the index).
fn icon_tile(ui: &mut egui::Ui, p: &Palette, v: &ItemView, size: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if v.is_app {
        mark::paint(ui.painter(), r, mark::TEAL);
    } else {
        ui.painter().rect_filled(r, size * 0.18, p.accent.gamma_multiply(0.18));
        ui.painter().rect_stroke(r, size * 0.18, Stroke::new(1.0, p.accent.gamma_multiply(0.6)), StrokeKind::Inside);
        let glyph = if v.name.to_lowercase().contains("dictionary") {
            "Aa".to_owned()
        } else {
            v.name.chars().next().map(|c| c.to_string()).unwrap_or_default()
        };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(size * 0.42), p.accent);
    }
}

fn change_row(ui: &mut egui::Ui, p: &Palette, c: &feed::Change) {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    let body = FontId::proportional(13.5);
    if !c.section.is_empty() {
        job.append(
            &format!("{}  ", c.section.to_uppercase()),
            0.0,
            TextFormat { font_id: FontId::monospace(10.5), color: p.muted, ..Default::default() },
        );
    }
    job.append(&c.text, 0.0, TextFormat { font_id: body, color: p.text, ..Default::default() });
    ui.add(egui::Label::new(job).wrap());
}

/// Open an http(s) link in the default browser.
fn open_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links can be opened.".to_owned());
    }
    #[cfg(test)]
    return crate::testhooks::with(|f| {
        f.urls.push(url.to_owned());
        Ok(())
    });
    #[cfg(not(test))]
    open_url_real(url)
}

#[cfg(not(test))]
fn open_url_real(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    let res = {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(kernpage_core::sysexe::system32("rundll32.exe"))
            .args(["url.dll,FileProtocolHandler", url])
            .creation_flags(0x0800_0000)
            .spawn()
    };
    #[cfg(not(windows))]
    let res = std::process::Command::new("xdg-open").arg(url).spawn();
    res.map(|_| ()).map_err(|e| format!("Could not open the link: {e}"))
}

// ---- frame loop --------------------------------------------------------------------------

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        #[cfg(test)]
        crate::testhooks::frame_start();
        let pal = Palette::for_theme(ctx.theme());
        theme::apply(ctx, &pal);

        if ctx.input(|i| i.viewport().close_requested()) && self.cfg.keep_in_tray && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden_to_tray = true;
        }
        let (focused, minimized) = ctx.input(|i| (i.viewport().focused, i.viewport().minimized));
        if focused == Some(true) {
            self.hidden_to_tray = false;
        }
        self.window_visible = !self.hidden_to_tray && minimized != Some(true);
        if self.last_refresh.elapsed() > Duration::from_secs(3) {
            self.refresh();
        }
        ctx.request_repaint_after(Duration::from_secs(3));
        // Automatic mode only; in Manual mode this never starts a request.
        self.updater.tick(self.cfg.check_updates, self.cfg.check_hours, feed::now_unix(), self.cfg.last_serial);
        self.process_updates();
        self.downloads.poll();
        self.process_auto_install(ctx);
        self.process_auto_pending(ctx);
        let views = self.make_views();

        egui::Panel::top("top")
            .exact_size(54.0)
            .frame(egui::Frame::new().fill(pal.bar).stroke(Stroke::new(1.0, pal.border)))
            .show(ui, |ui| self.top_bar(ui, &pal));
        let (left, right, left_color) = self.footer_text(&pal);
        egui::Panel::bottom("footer")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(pal.bar).stroke(Stroke::new(1.0, pal.border)).inner_margin(Margin::symmetric(16, 6)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(left).monospace().size(11.5).color(left_color));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(right).monospace().size(11.5).color(pal.muted));
                    });
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(pal.bg).inner_margin(Margin::symmetric(24, 16))).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(1180.0));
                self.toolbar(ui, &pal, &views);
                self.notice_bar(ui, &pal);
                self.report_bar(ui, &pal);
                ui.add_space(4.0);
                section_label(ui, &pal, "INSTALLED");
                ui.horizontal_wrapped(|ui| self.item_card(ui, &pal, 0, &views[0]));
                ui.add_space(14.0);
                self.available_section(ui, &pal, &views);
                ui.add_space(14.0);
                self.recent_section(ui, &pal);
            });
        });

        if self.stub.is_some() {
            self.stub_modal(ctx, &pal);
        } else if self.rollback_confirm.is_some() {
            self.rollback_modal(ctx, &pal);
        } else if self.rollback_list {
            self.rollback_list_modal(ctx, &pal, &views);
        } else if self.confirm_clear {
            self.confirm_modal(ctx, &pal);
        } else if self.announce.is_some() {
            self.announce_modal(ctx, &pal, &views);
        } else if self.settings {
            self.settings_modal(ctx, &pal);
        } else if self.details {
            self.details_modal(ctx, &pal, &views);
        }
        self.screenshot_step(ctx);
        let _ = (Key::Escape, instance::WINDOW_TITLE);
    }
}
