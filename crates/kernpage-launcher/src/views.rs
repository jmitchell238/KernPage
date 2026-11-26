//! What the window shows for each card, worked out from the verified index, the installed
//! version and the user's settings (pre-releases, skipped versions). Pure data, no UI.

use crate::config::Config;
use crate::content::KERNPAGE;
use crate::feed::{self, Change, Feed};
use semver::Version;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// KernPage is installed and up to date.
    Current,
    /// KernPage is installed and a newer version is offered.
    Update,
    /// KernPage was not found next to the launcher.
    Missing,
    /// An add-on that can be installed.
    Available,
    /// Nothing can be offered (blocklisted or failed its checks).
    Blocked,
}

#[derive(Clone, Debug)]
pub struct Latest {
    pub version: Version,
    /// Display form, e.g. "Dec 1, 2026".
    pub released: String,
    pub size: Option<u64>,
    pub platforms: String,
    pub changes: Vec<Change>,
    pub changelog_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ItemView {
    pub id: String,
    pub is_app: bool,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub features: Vec<(String, String)>,
    pub website: Option<String>,
    pub repo: Option<String>,
    pub releases_url: Option<String>,
    pub installed: Option<Version>,
    pub latest: Option<Latest>,
    pub state: State,
    /// A one-line explanation shown on the card (skipped, blocked, failed checks).
    pub note: Option<String>,
    pub skipped: bool,
}

pub const APP_ID: &str = "kernpage";

fn latest_of(feed: &Feed, item: &feed::Item, include_pre: bool) -> Option<Latest> {
    let c = feed.latest(item, include_pre)?;
    Some(Latest {
        released: feed::pretty_date(&c.release.released),
        size: c.release.file_for_this_platform().map(|f| f.size),
        platforms: c.release.platforms(),
        changes: feed::parse_notes(&c.release.notes),
        changelog_url: c.release.changelog_url.clone(),
        version: c.version,
    })
}

fn features_of(item: &feed::Item) -> Vec<(String, String)> {
    item.features.iter().map(|f| (f.title.clone(), f.text.clone())).collect()
}

/// The KernPage card first, then every add-on in the index.
pub fn build(feed: Option<&Feed>, cfg: &Config, installed: &Version, app_present: bool) -> Vec<ItemView> {
    let mut out = Vec::new();
    let app_item = feed.and_then(|f| f.item(APP_ID)).filter(|i| i.is_app());
    let mut app = ItemView {
        id: APP_ID.into(),
        is_app: true,
        name: KERNPAGE.name.into(),
        tagline: KERNPAGE.tagline.into(),
        description: String::new(),
        features: KERNPAGE.features.iter().map(|f| (f.title.to_owned(), f.text.to_owned())).collect(),
        website: None,
        repo: None,
        releases_url: None,
        installed: Some(installed.clone()),
        latest: None,
        state: if app_present { State::Current } else { State::Missing },
        note: None,
        skipped: false,
    };
    if let (Some(feed), Some(item)) = (feed, app_item) {
        app.name = item.name.clone();
        if !item.tagline.is_empty() {
            app.tagline = item.tagline.clone();
        }
        if !item.features.is_empty() {
            app.features = features_of(item);
        }
        app.website = item.website.clone();
        app.repo = item.repo.clone();
        app.releases_url = item.releases_url.clone();
        app.latest = latest_of(feed, item, cfg.include_prereleases);
        if let Some(b) = feed.blocked(APP_ID, installed) {
            let why = if b.reason.is_empty() { "withdrawn by the publisher" } else { &b.reason };
            app.note = Some(format!("This version is blocked: {why}"));
        }
        match &app.latest {
            Some(l) if l.version > *installed => {
                if cfg.is_skipped(APP_ID, &l.version.to_string()) {
                    app.skipped = true;
                    app.note.get_or_insert_with(|| format!("You skipped version {}.", l.version));
                } else if app_present {
                    app.state = State::Update;
                }
            }
            Some(_) => {}
            None => {
                if let Some(why) = feed.refused_newer(item, Some(installed), cfg.include_prereleases) {
                    app.note.get_or_insert(why);
                }
            }
        }
        if app.state == State::Current && feed.blocked(APP_ID, installed).is_some() {
            app.state = State::Blocked;
        }
    }
    out.push(app);

    if let Some(feed) = feed {
        for item in feed.items.iter().filter(|i| !i.is_app()) {
            let latest = latest_of(feed, item, cfg.include_prereleases);
            let (state, note) = match &latest {
                Some(_) => (State::Available, None),
                None => (
                    State::Blocked,
                    Some(feed.refused_newer(item, None, cfg.include_prereleases).unwrap_or_else(|| "No compatible release is available.".into())),
                ),
            };
            out.push(ItemView {
                id: item.id.clone(),
                is_app: false,
                name: item.name.clone(),
                tagline: item.tagline.clone(),
                description: item.description.clone(),
                features: features_of(item),
                website: item.website.clone(),
                repo: item.repo.clone(),
                releases_url: item.releases_url.clone(),
                installed: None,
                latest,
                state,
                note,
                skipped: false,
            });
        }
    }
    out
}

pub fn update_count(views: &[ItemView]) -> usize {
    views.iter().filter(|v| v.state == State::Update).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devfeed::*;

    fn feed(o: &SampleOpts) -> Feed {
        let b = sample_bytes(o);
        let key = vec![feed::parse_public_key(TEST_PUBLIC).unwrap()];
        feed::parse_verified(&b, &sign_test(&b), &key, 1_800_000_000, 0).unwrap()
    }

    fn installed() -> Version {
        Version::new(0, 1, 2)
    }

    #[test]
    fn update_is_offered_and_counted() {
        let f = feed(&SampleOpts::new(1, "http://127.0.0.1:1"));
        let v = build(Some(&f), &Config::default(), &installed(), true);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].state, State::Update);
        assert_eq!(v[0].latest.as_ref().unwrap().version, Version::new(0, 2, 0));
        assert_eq!(v[0].latest.as_ref().unwrap().changes.len(), 12);
        assert_eq!(v[0].features.len(), 6);
        assert_eq!(v[1].state, State::Available);
        assert_eq!(v[1].name, "Spelling dictionary: English (US)");
        assert_eq!(update_count(&v), 1);
    }

    #[test]
    fn up_to_date_and_newer_installed_show_no_update() {
        let f = feed(&SampleOpts::new(1, "http://127.0.0.1:1"));
        for ver in [Version::new(0, 2, 0), Version::new(0, 9, 0)] {
            let v = build(Some(&f), &Config::default(), &ver, true);
            assert_eq!(v[0].state, State::Current);
            assert_eq!(update_count(&v), 0);
        }
    }

    #[test]
    fn skipping_a_version_hides_the_update_until_a_newer_one() {
        let f = feed(&SampleOpts::new(1, "http://127.0.0.1:1"));
        let mut cfg = Config::default();
        cfg.skip(APP_ID, "0.2.0");
        let v = build(Some(&f), &cfg, &installed(), true);
        assert_eq!(v[0].state, State::Current);
        assert!(v[0].skipped);
        assert_eq!(update_count(&v), 0);
        // A newer release is not covered by the skip.
        let mut o = SampleOpts::new(2, "http://127.0.0.1:1");
        o.kernpage_version = "0.2.1".into();
        let v = build(Some(&feed(&o)), &cfg, &installed(), true);
        assert_eq!(v[0].state, State::Update);
        cfg.unskip(APP_ID, "0.2.0");
        assert!(!cfg.is_skipped(APP_ID, "0.2.0"));
    }

    #[test]
    fn prerelease_setting_changes_the_offer() {
        let f = feed(&SampleOpts::new(1, "http://127.0.0.1:1"));
        let mut cfg = Config::default();
        assert_eq!(build(Some(&f), &cfg, &installed(), true)[0].latest.as_ref().unwrap().version, Version::new(0, 2, 0));
        cfg.include_prereleases = true;
        assert_eq!(build(Some(&f), &cfg, &installed(), true)[0].latest.as_ref().unwrap().version, Version::parse("0.3.0-beta.1").unwrap());
    }

    #[test]
    fn blocklisted_installed_version_is_marked_and_flagged_release_not_offered() {
        let mut o = SampleOpts::new(1, "http://127.0.0.1:1");
        o.block_kernpage = Some("<0.2.0".into());
        let v = build(Some(&feed(&o)), &Config::default(), &installed(), true);
        assert_eq!(v[0].state, State::Update);
        assert!(v[0].note.as_ref().unwrap().contains("blocked"));

        let mut o = SampleOpts::new(1, "http://127.0.0.1:1");
        o.bad_sha = true;
        let v = build(Some(&feed(&o)), &Config::default(), &installed(), true);
        assert_eq!(v[0].state, State::Current);
        assert!(v[0].note.as_ref().unwrap().contains("failed its checks"));
    }

    #[test]
    fn without_a_feed_the_card_still_shows() {
        let v = build(None, &Config::default(), &installed(), true);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].state, State::Current);
        assert_eq!(build(None, &Config::default(), &installed(), false)[0].state, State::Missing);
    }
}
