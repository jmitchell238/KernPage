//! The update checker the window talks to. All network work runs on a worker thread; the UI
//! polls for the result. Nothing here touches the network unless a check is started, and a
//! check is started only by the user (Refresh / Check now / `--check-now`) or, in Automatic
//! mode, at start and every N hours (`tick`).

use crate::config::CheckMode;
use crate::feed::{Feed, FeedError};
use crate::net::{self, Cache, Fetch};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// Where the verified index is cached: next to `launcher.toml`.
pub fn cache_dir() -> PathBuf {
    crate::config::store_path()
        .and_then(|p| p.parent().map(|d| d.join("update-cache")))
        .unwrap_or_else(|| std::env::temp_dir().join("kernpage-update-cache"))
}

pub struct Snapshot {
    pub feed: Feed,
    /// Unix time the index was last confirmed.
    pub checked_at: i64,
    /// True when this came from the on-disk cache and no check has succeeded this session.
    pub from_cache: bool,
}

/// What a finished check changed.
pub enum Outcome {
    Updated { serial: u64 },
    Failed,
}

pub struct Updater {
    url: String,
    cache_dir: PathBuf,
    timeout: Duration,
    rx: Option<Receiver<Result<net::Fetched, FeedError>>>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
    pub snapshot: Option<Snapshot>,
    /// Why the last check failed (cleared by a success).
    pub error: Option<FeedError>,
    last_attempt: Option<i64>,
    last_failed: bool,
    /// Number of checks started (for tests and the log).
    pub started: usize,
}

/// After a failed automatic check, try again after this many seconds at most.
const RETRY_AFTER_FAILURE: i64 = 15 * 60;

impl Updater {
    pub fn new(url: String, cache_dir: PathBuf) -> Self {
        Updater {
            url,
            cache_dir,
            timeout: Duration::from_secs(15),
            rx: None,
            repaint: None,
            snapshot: None,
            error: None,
            last_attempt: None,
            last_failed: false,
            started: 0,
        }
    }

    pub fn with_timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    pub fn set_repaint(&mut self, f: Arc<dyn Fn() + Send + Sync>) {
        self.repaint = Some(f);
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn is_checking(&self) -> bool {
        self.rx.is_some()
    }

    /// Show the last verified index from disk. No network.
    pub fn load_cache(&mut self, now: i64, last_serial: u64) {
        if let Some(f) = Cache::new(&self.cache_dir).load_verified(&self.url, now, last_serial) {
            self.snapshot = Some(Snapshot { feed: f.feed, checked_at: f.checked_at, from_cache: true });
        }
    }

    /// Start a check on a worker thread (does nothing if one is already running).
    pub fn start_check(&mut self, now: i64, last_serial: u64) {
        if self.rx.is_some() {
            return;
        }
        self.started += 1;
        self.last_attempt = Some(now);
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let fetch = Fetch { url: self.url.clone(), cache_dir: self.cache_dir.clone(), timeout: self.timeout, last_serial };
        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            let _ = tx.send(net::check(&fetch, now));
            if let Some(r) = repaint {
                r();
            }
        });
    }

    /// Automatic mode only: start a check when none has run yet or the interval has passed.
    /// Manual mode never starts one here.
    pub fn tick(&mut self, mode: CheckMode, interval_hours: u32, now: i64, last_serial: u64) {
        if mode != CheckMode::Automatic || self.rx.is_some() {
            return;
        }
        let wait = i64::from(interval_hours.max(1)) * 3600;
        let wait = if self.last_failed { wait.min(RETRY_AFTER_FAILURE) } else { wait };
        let due = match self.last_attempt {
            None => true,
            Some(t) => now - t >= wait,
        };
        if due {
            self.start_check(now, last_serial);
        }
    }

    /// Collect a finished check, if any.
    pub fn poll(&mut self) -> Option<Outcome> {
        let rx = self.rx.as_ref()?;
        let res = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(FeedError::Unreachable("the check stopped unexpectedly".into())),
        };
        self.rx = None;
        match res {
            Ok(f) => {
                let serial = f.feed.serial;
                self.snapshot = Some(Snapshot { feed: f.feed, checked_at: f.checked_at, from_cache: false });
                self.error = None;
                self.last_failed = false;
                Some(Outcome::Updated { serial })
            }
            Err(e) => {
                self.error = Some(e);
                self.last_failed = true;
                Some(Outcome::Failed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devfeed::*;
    use crate::devserver::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn setup(name: &str) -> (Server, PathBuf) {
        let files: Files = Arc::new(Mutex::new(HashMap::new()));
        let b = sample_bytes(&SampleOpts::new(2, "http://127.0.0.1:1"));
        files.lock().unwrap().insert("/index.json.minisig".into(), sign_test(&b).into_bytes());
        files.lock().unwrap().insert("/index.json".into(), b);
        let dir = std::env::temp_dir().join(format!("pw-upd-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Server::start(0, static_handler(files)).unwrap(), dir)
    }

    fn wait(u: &mut Updater) -> Outcome {
        for _ in 0..200 {
            if let Some(o) = u.poll() {
                return o;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("check did not finish");
    }

    #[test]
    fn manual_mode_makes_zero_requests_until_the_user_checks() {
        let (server, dir) = setup("manual");
        let mut u = Updater::new(server.url("/index.json"), dir.clone());
        let mut now = 1_000_000;
        u.load_cache(now, 0);
        for _ in 0..50 {
            now += 3 * 3600; // hours pass, many frames
            u.tick(CheckMode::Manual, 6, now, 0);
            assert!(u.poll().is_none());
        }
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(server.hits(), 0, "manual mode must not touch the network");
        assert_eq!(u.started, 0);
        // The user clicks Refresh.
        u.start_check(crate::feed::now_unix(), 0);
        assert!(matches!(wait(&mut u), Outcome::Updated { serial: 2 }));
        assert!(server.hits() >= 1);
        assert!(!u.snapshot.as_ref().unwrap().from_cache);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn automatic_mode_checks_at_start_then_every_interval() {
        let (server, dir) = setup("auto");
        let mut u = Updater::new(server.url("/index.json"), dir.clone());
        let t0 = crate::feed::now_unix();
        u.tick(CheckMode::Automatic, 6, t0, 0);
        assert_eq!(u.started, 1);
        wait(&mut u);
        u.tick(CheckMode::Automatic, 6, t0 + 3600, 0);
        assert_eq!(u.started, 1, "not due yet");
        u.tick(CheckMode::Automatic, 6, t0 + 6 * 3600, 0);
        assert_eq!(u.started, 2);
        wait(&mut u);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offline_uses_the_cache_and_reports_the_reason() {
        let (server, dir) = setup("offline");
        let url = server.url("/index.json");
        let mut u = Updater::new(url.clone(), dir.clone());
        u.start_check(crate::feed::now_unix(), 0);
        wait(&mut u);
        drop(server); // network gone
        let mut u2 = Updater::new(url, dir.clone()).with_timeout(Duration::from_secs(2));
        u2.load_cache(crate::feed::now_unix(), 2);
        assert!(u2.snapshot.as_ref().unwrap().from_cache);
        u2.start_check(crate::feed::now_unix(), 2);
        assert!(matches!(wait(&mut u2), Outcome::Failed));
        assert!(u2.error.as_ref().unwrap().to_string().starts_with("Could not reach the update server"));
        assert!(u2.snapshot.is_some(), "the cached index stays on screen");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
