//! The editor side of AutoRecover: the timer, the background writer, the live-process
//! check used by recovery on start-up. The logic lives in `kernpage_core::autorecover`.

use kernpage_core::autorecover::{
    self as core, ContentKey, Meta, PanicState, Settings, Writer,
};
use kernpage_core::Document;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The AutoRecover folder; in tests a per-thread folder when one was installed (tests run in
/// parallel, so the process-wide environment variables cannot be per test).
fn snapshot_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some((d, _)) = test_env::get() {
        return Some(d);
    }
    core::autorecover_dir()
}

pub(crate) fn settings_file() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some((_, s)) = test_env::get() {
        return Some(s);
    }
    core::settings_path()
}

#[cfg(test)]
pub mod test_env {
    use std::cell::RefCell;
    use std::path::PathBuf;
    thread_local! {
        static ENV: RefCell<Option<(PathBuf, PathBuf)>> = const { RefCell::new(None) };
    }
    /// Use `dir` / `settings` for the apps built on this thread from now on.
    pub fn install(dir: PathBuf, settings: PathBuf) {
        ENV.with(|e| *e.borrow_mut() = Some((dir, settings)));
    }
    pub fn get() -> Option<(PathBuf, PathBuf)> {
        ENV.with(|e| e.borrow().clone())
    }
}

pub struct AutoRecover {
    pub settings: Settings,
    dir: Option<PathBuf>,
    writer: Option<Writer>,
    /// The current document's snapshot file; chosen at its first write.
    file: Option<PathBuf>,
    has_snapshot: bool,
    /// The content the last snapshot (or the recovery that loaded the document) holds.
    pub last: Option<ContentKey>,
    next_tick: Instant,
    counter: u32,
    panic_rev: Option<u64>,
}

impl AutoRecover {
    /// `active` is false in screenshot mode: nothing is read from or written to disk.
    pub fn new(active: bool) -> AutoRecover {
        let settings = if active {
            settings_file().map_or_else(Settings::default, |p| Settings::load_from(&p))
        } else {
            Settings::default()
        };
        let dir = if active { snapshot_dir() } else { None };
        AutoRecover {
            writer: dir.as_ref().map(|_| Writer::spawn()),
            next_tick: Instant::now() + settings.interval(),
            settings,
            dir,
            file: None,
            has_snapshot: false,
            last: None,
            counter: 0,
            panic_rev: None,
        }
    }

    /// Write the Options > Save settings to the settings file.
    pub fn save_settings(s: &Settings) {
        if let Some(p) = settings_file() {
            let _ = s.save_to(&p);
        }
    }

    /// Test-only: wait until every queued snapshot write / delete has reached the disk.
    #[cfg(test)]
    pub fn flush(&mut self) {
        if let Some(w) = &mut self.writer {
            w.finish();
        }
        if self.dir.is_some() {
            self.writer = Some(Writer::spawn());
        }
    }

    /// Test-only clock: the timer fires on the next frame.
    #[cfg(test)]
    pub fn make_due(&mut self) {
        self.next_tick = Instant::now();
    }

    pub fn dir(&self) -> Option<&PathBuf> {
        self.dir.as_ref()
    }

    pub fn active(&self) -> bool {
        self.dir.is_some() && self.writer.is_some()
    }

    /// Apply new settings from Options > Save: the tick restarts with the new interval.
    pub fn apply_settings(&mut self, s: Settings) {
        self.settings = s;
        self.next_tick = Instant::now() + s.interval();
        if !s.enabled {
            self.forget();
        }
    }

    /// The document was saved, closed or replaced: its snapshot is no longer needed.
    pub fn forget(&mut self) {
        if let (Some(f), Some(w)) = (self.file.take(), &self.writer) {
            if self.has_snapshot {
                w.delete(f);
            }
        }
        self.has_snapshot = false;
        self.last = None;
        self.panic_rev = None;
        core::set_panic_state(None);
    }

    /// Normal exit: delete the snapshot and finish the queued file work.
    pub fn shutdown(&mut self) {
        self.forget();
        if let Some(w) = &mut self.writer {
            w.finish();
        }
        self.writer = None;
    }

    /// Whether the free-running timer has fired (and arms the next tick).
    pub fn due(&mut self, now: Instant) -> bool {
        if !self.active() || !self.settings.enabled {
            return false;
        }
        if now >= self.next_tick {
            self.next_tick = now + self.settings.interval();
            true
        } else {
            false
        }
    }

    pub fn until_next(&self, now: Instant) -> Duration {
        self.next_tick.saturating_duration_since(now)
    }

    fn file_for(&mut self, name: &str) -> Option<PathBuf> {
        if self.file.is_none() {
            self.counter += 1;
            let code = core::new_code(std::process::id(), self.counter);
            self.file = Some(self.dir.as_ref()?.join(core::snapshot_file_name(name, code)));
        }
        self.file.clone()
    }

    /// Queue a snapshot of already-serialized content for the background writer.
    pub fn write(&mut self, meta: &Meta, doc: &Document, key: ContentKey) {
        let Some(file) = self.file_for(&meta.display_name) else {
            return;
        };
        let Ok(bytes) = core::encode(meta, doc) else {
            return;
        };
        if let Some(w) = &self.writer {
            w.write(file, bytes);
            self.has_snapshot = true;
            self.last = Some(key);
        }
    }

    /// Keep the panic hook's copy of a dirty document current (`rev` = editor revision).
    pub fn track_for_panic(&mut self, rev: u64, meta: &Meta, doc: &Document) {
        if !self.active() || !self.settings.enabled || self.panic_rev == Some(rev) {
            return;
        }
        let Some(file) = self.file_for(&meta.display_name) else {
            return;
        };
        self.panic_rev = Some(rev);
        core::set_panic_state(Some(PanicState {
            file,
            meta: meta.clone(),
            document: doc.clone(),
        }));
    }

    pub fn clear_panic(&mut self) {
        if self.panic_rev.take().is_some() {
            core::set_panic_state(None);
        }
    }
}

/// Whether `pid` is a running KernPage process (so a second editor never takes another's
/// live snapshot, and a reused process id of some other program does not hide a crash).
#[cfg(windows)]
pub fn is_kernpage_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0u32;
        let alive = GetExitCodeProcess(h, &mut code) != 0 && code == STILL_ACTIVE as u32;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let named = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(h);
        alive && named && image_is_kernpage(&String::from_utf16_lossy(&buf[..len as usize]))
    }
}

#[cfg(not(windows))]
pub fn is_kernpage_running(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|c| c.trim().to_lowercase().starts_with("kernpage"))
        .unwrap_or(false)
}

/// Whether an executable path is a KernPage program (`kernpage.exe`, ...).
pub fn image_is_kernpage(path: &str) -> bool {
    path.rsplit(['\\', '/'])
        .next()
        .is_some_and(|n| n.to_lowercase().starts_with("kernpage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_names() {
        assert!(image_is_kernpage(r"C:\Program Files\KernPage\kernpage.exe"));
        assert!(image_is_kernpage("/x/kernpage-1a2b"));
        assert!(!image_is_kernpage(r"C:\Windows\notepad.exe"));
    }

    #[test]
    fn own_process_is_found_only_when_named_kernpage() {
        // The test binary is called kernpage-<hash>, so it counts; a pid nobody has does not.
        assert!(is_kernpage_running(std::process::id()));
        assert!(!is_kernpage_running(u32::MAX - 1));
    }

    #[test]
    fn timer_is_free_running_and_respects_the_checkbox() {
        let mut a = AutoRecover::new(false);
        assert!(!a.due(Instant::now() + Duration::from_secs(10_000)), "inactive");
        let dir = std::env::temp_dir().join(format!("pw-ar-timer-{}", std::process::id()));
        a.dir = Some(dir.clone());
        a.writer = Some(Writer::spawn());
        a.settings = Settings { enabled: true, minutes: 1 };
        a.next_tick = Instant::now() + Duration::from_secs(60);
        let t0 = Instant::now();
        assert!(!a.due(t0));
        assert!(a.due(t0 + Duration::from_secs(61)));
        assert!(!a.due(t0 + Duration::from_secs(62)));
        a.settings.enabled = false;
        assert!(!a.due(t0 + Duration::from_secs(10_000)));
        a.shutdown();
        let _ = std::fs::remove_dir_all(dir);
    }
}
