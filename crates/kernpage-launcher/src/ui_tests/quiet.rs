//! `--check-update` (the editor's quiet check): which version it reports, and that it keeps to
//! the update settings and never fails loudly.

use super::support::*;
use crate::quiet::newer_version;
use semver::Version;

fn installed() -> Version {
    Version::parse(crate::app::VERSION).unwrap()
}

#[test]
fn automatic_mode_reports_a_newer_version() {
    // One minor version above this build, so the test keeps working after every release.
    let i = installed();
    let newer = Version::new(i.major, i.minor + 1, 0);
    let feed = FeedServer::start(FeedOpts { version: newer.to_string(), ..FeedOpts::default() });
    let _ui = TestUi::with(Setup::new().config("check_updates = automatic"));
    assert_eq!(newer_version(Some(feed.url())), Some(newer));
}

#[test]
fn nothing_is_reported_when_up_to_date() {
    let feed = FeedServer::start(FeedOpts { version: installed().to_string(), ..FeedOpts::default() });
    let _ui = TestUi::with(Setup::new().config("check_updates = automatic"));
    assert_eq!(newer_version(Some(feed.url())), None);
}

#[test]
fn manual_mode_never_touches_the_network() {
    let feed = FeedServer::start(FeedOpts::default());
    let _ui = TestUi::with(Setup::new());
    assert_eq!(newer_version(Some(feed.url())), None);
    assert_eq!(feed.server.hits(), 0);
}

#[test]
fn offline_reports_nothing_and_does_not_fail() {
    let _ui = TestUi::with(Setup::new().config("check_updates = automatic"));
    assert_eq!(newer_version(Some(DEAD_FEED.to_owned())), None);
}

#[test]
fn a_skipped_version_is_not_reported() {
    let feed = FeedServer::start(FeedOpts::default());
    let _ui = TestUi::with(Setup::new().config(&format!("check_updates = automatic\nskipped = kernpage@{NEW}")));
    assert_eq!(newer_version(Some(feed.url())), None);
}
