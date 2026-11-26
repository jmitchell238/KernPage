//! The quiet "Update available" notice. At start-up a background thread asks the launcher
//! (`kernpage-launcher --check-update`, which applies the user's update settings, the
//! signed feed and the offline cache) whether a newer KernPage is offered. Nothing
//! shows unless the answer is yes: offline, no launcher next to the editor, a dev build or
//! "up to date" all look the same as no check at all. The button starts the launcher, which
//! downloads and installs the update. The UI never waits for the check.

use eframe::egui::{self, Color32, RichText, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

/// What the launcher prints when it found an update: `update-available <version>`.
const PREFIX: &str = "update-available ";

pub struct UpdateNotice {
    rx: Option<Receiver<Option<String>>>,
    /// The version offered, once the check has answered yes.
    pub available: Option<String>,
    dismissed: bool,
    /// Starts the launcher; `true` when it started. Replaced by a fake in tests.
    launch: Box<dyn Fn() -> bool>,
}

impl UpdateNotice {
    /// No check (screenshots, tests).
    pub fn idle() -> UpdateNotice {
        UpdateNotice { rx: None, available: None, dismissed: false, launch: Box::new(start_launcher) }
    }

    /// Check once now, on a worker thread, with the real launcher.
    pub fn start(ctx: &egui::Context) -> UpdateNotice {
        UpdateNotice::with_probe(ctx, ask_launcher)
    }

    /// Check once now with `probe` (a test seam: no process, no network).
    pub fn with_probe(ctx: &egui::Context, probe: impl FnOnce() -> Option<String> + Send + 'static) -> UpdateNotice {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(probe());
            ctx.request_repaint();
        });
        UpdateNotice { rx: Some(rx), ..UpdateNotice::idle() }
    }

    /// Replace what the button runs (a test seam).
    pub fn set_launch(&mut self, f: impl Fn() -> bool + 'static) {
        self.launch = Box::new(f);
    }

    /// Collect the answer when it arrives.
    fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        match rx.try_recv() {
            Ok(v) => {
                self.available = v;
                self.rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => self.rx = None,
        }
    }

    fn shown(&self) -> Option<&str> {
        self.available.as_deref().filter(|_| !self.dismissed)
    }

    /// The notice bar along the bottom of the window; paints nothing when there is no update.
    pub fn bar(&mut self, ui: &mut Ui) {
        self.poll();
        let Some(version) = self.shown().map(str::to_owned) else { return };
        let ink = Color32::from_rgb(0x32, 0x2A, 0x00);
        let mut start = false;
        let mut dismiss = false;
        egui::Panel::bottom("update_notice")
            .exact_size(28.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(0xFF, 0xF4, 0xC2)).inner_margin(egui::Margin::symmetric(10, 3)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(format!("Update available: KernPage {version}")).color(ink).size(12.5));
                    if ui.button("Update...").clicked() {
                        start = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Dismiss").clicked() {
                            dismiss = true;
                        }
                    });
                });
            });
        if start {
            // The launcher takes over; the notice goes away only if it really started.
            self.dismissed = (self.launch)();
        }
        if dismiss {
            self.dismissed = true;
        }
    }
}

/// The launcher next to the editor (override with `KERNPAGE_LAUNCHER_EXE`, debug builds only); `None` when it
/// is not there, as in a development build.
fn launcher_exe() -> Option<PathBuf> {
    // The environment override exists for development only: a release build always runs the
    // launcher installed next to it (security audit PW-SEC-008).
    let over = if cfg!(debug_assertions) { std::env::var_os("KERNPAGE_LAUNCHER_EXE") } else { None };
    let p = match over {
        Some(p) => PathBuf::from(p),
        None => std::env::current_exe().ok()?.parent()?.join(format!("kernpage-launcher{}", std::env::consts::EXE_SUFFIX)),
    };
    p.is_file().then_some(p)
}

fn command(exe: &PathBuf) -> std::process::Command {
    let mut c = std::process::Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Run `kernpage-launcher --check-update` and read its answer. Any failure is "no update".
fn ask_launcher() -> Option<String> {
    let exe = launcher_exe()?;
    let out = command(&exe).arg("--check-update").stdin(std::process::Stdio::null()).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_answer(&String::from_utf8_lossy(&out.stdout))
}

fn parse_answer(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.trim().strip_prefix(PREFIX)).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

/// Open the launcher's window and have it refresh its update list.
fn start_launcher() -> bool {
    let Some(exe) = launcher_exe() else { return false };
    let mut c = std::process::Command::new(exe);
    c.arg("--check-now");
    c.spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_parsed() {
        assert_eq!(parse_answer("update-available 0.2.0\r\n"), Some("0.2.0".into()));
        assert_eq!(parse_answer(""), None);
        assert_eq!(parse_answer("update-available \n"), None);
        assert_eq!(parse_answer("something else\n"), None);
    }
}
