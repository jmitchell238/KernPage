//! `kernpage-launcher --update-now [--update-to <version>] --feed-url <url>`: a headless
//! "check, download, verify, install" for the end-to-end test of a dev build against a local
//! test feed. It runs the same pieces the window does (`Release::file_for_this_platform` picks
//! the MSI or the Setup.exe, `download::run` verifies the file, `install::start_helper` hands it
//! to the update helper, which verifies it again and runs it). Compiled only in debug builds and
//! with the `dev-feed` cargo feature, never in a normal release build.

use crate::app::VERSION;
use crate::download::{self, Finish, Job, Progress};
use crate::feed;
use crate::install;
use crate::updates::{self, Outcome, Updater};
use crate::views::APP_ID;
use semver::Version;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

pub fn run(feed_url: Option<String>, to: Option<String>) -> i32 {
    match go(feed_url, to) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("update-now failed: {e}");
            1
        }
    }
}

fn go(feed_url: Option<String>, to: Option<String>) -> Result<String, String> {
    let url = feed_url.unwrap_or_else(|| feed::FEED_URL.to_owned());
    let mut updater = Updater::new(url.clone(), updates::cache_dir()).with_timeout(Duration::from_secs(10));
    updater.start_check(feed::now_unix(), 0);
    let started = Instant::now();
    loop {
        match updater.poll() {
            Some(Outcome::Updated { .. }) => break,
            Some(Outcome::Failed) => return Err(updater.error.as_ref().map_or("check failed".to_owned(), |e| e.to_string())),
            None if started.elapsed() > Duration::from_secs(30) => return Err("the check timed out".into()),
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let snapshot = updater.snapshot.as_ref().ok_or("no update list")?;
    let item = snapshot.feed.item(APP_ID).ok_or("the feed has no KernPage")?;
    let (version, release) = match to {
        Some(t) => {
            let v = Version::parse(&t).map_err(|e| e.to_string())?;
            (v.clone(), snapshot.feed.release_for(item, &v, true).ok_or("that version is not offered")?)
        }
        None => {
            let c = snapshot.feed.latest(item, false).ok_or("nothing is offered")?;
            if c.version <= Version::parse(VERSION).map_err(|e| e.to_string())? {
                return Ok(format!("up to date ({VERSION})"));
            }
            (c.version, c.release)
        }
    };
    let file = release
        .file_for_this_platform()
        .ok_or("the update list has no download of that version for this install")?
        .clone();
    let version = version.to_string();
    let kind = file.kind.clone();
    let job = Job { key: Job::key_for(APP_ID, &version), item_id: APP_ID.to_owned(), version: version.clone(), file, feed_url: url.clone() };
    let path = match download::run(&job, &download::updates_dir(), &Progress::default(), &AtomicBool::new(false), &|| {}) {
        Ok(Finish::Done(p)) => p,
        Ok(Finish::Cancelled) => return Err("cancelled".into()),
        Err(e) => return Err(e.to_string()),
    };
    install::start_helper(&path, &version, &download::updates_dir(), Some(&url))?;
    Ok(format!("handed {} ({kind}) for {version} to the update helper", path.display()))
}
