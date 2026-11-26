//! `kernpage-launcher --check-update`: the editor's quiet update check. No window, no instance
//! lock. It applies the same rules as the window: Manual mode never touches the network (the
//! cached index is used), Automatic mode asks the update server once; skipped versions,
//! pre-release setting and blocked versions are honoured. Prints `update-available <version>`
//! when a newer KernPage is offered and nothing otherwise (offline, no feed, up to date).

use crate::app::{editor_exe, VERSION};
use crate::config::{CheckMode, Config};
use crate::feed;
use crate::updates::{self, Outcome, Updater};
use crate::views::{self, State};
use semver::Version;
use std::time::{Duration, Instant};

/// The line printed when an update exists (the editor looks for this prefix).
pub const PREFIX: &str = "update-available ";

pub fn run(feed_url: Option<String>) -> i32 {
    if let Some(v) = newer_version(feed_url) {
        println!("{PREFIX}{v}");
    }
    0
}

pub(crate) fn newer_version(feed_url: Option<String>) -> Option<Version> {
    let mut cfg = Config::load();
    let url = feed_url.unwrap_or_else(|| feed::FEED_URL.to_owned());
    let mut updater = Updater::new(url, updates::cache_dir()).with_timeout(Duration::from_secs(10));
    updater.load_cache(feed::now_unix(), cfg.last_serial);
    if cfg.check_updates == CheckMode::Automatic {
        updater.start_check(feed::now_unix(), cfg.last_serial);
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            match updater.poll() {
                Some(Outcome::Updated { serial }) => {
                    if serial > cfg.last_serial {
                        cfg.last_serial = serial;
                        cfg.save();
                    }
                    break;
                }
                Some(Outcome::Failed) => break, // offline: the cached index (if any) still answers
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }
    let installed = Version::parse(VERSION).ok()?;
    let items = views::build(updater.snapshot.as_ref().map(|s| &s.feed), &cfg, &installed, editor_exe().is_file());
    let app = items.iter().find(|v| v.is_app && v.state == State::Update)?;
    app.latest.as_ref().map(|l| l.version.clone())
}
