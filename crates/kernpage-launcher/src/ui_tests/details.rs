//! The details modal and the rollback ("Go back to an older version") dialogs.

use super::support::*;
use eframe::egui::Key;

/// A feed with the full sample notes (12 changes) and a placeholder-sized download.
fn notes_feed() -> FeedOpts {
    FeedOpts { real: false, ..Default::default() }
}

fn details(ui: &mut TestUi) {
    ui.open_more();
    ui.click("What's new");
    assert!(ui.has("01 / FEATURES"));
}

#[test]
fn details_sections_before_any_check() {
    let mut ui = TestUi::start();
    details(&mut ui);
    for l in ["01 / FEATURES", "02 / LATEST RELEASE", "VERSION", "RELEASED", "DOWNLOAD", "PLATFORMS", "WHAT'S CHANGED", "Opens Publisher files", "Open source, works offline", "Not checked yet", "Windows 64-bit", "Check for updates to see what's changed."] {
        assert!(ui.has(l), "{l} in {:?}", ui.texts());
    }
    // The app's links are unknown until the update list says.
    for l in ["Website", "GitHub", "Releases"] {
        assert!(!ui.enabled_last(l), "{l} is greyed");
    }
    ui.hover_last("GitHub");
    assert!(ui.has("Not provided by the update list"));
    assert!(!ui.has("Full changelog"));
}

#[test]
fn details_sections_with_an_update() {
    let (_f, mut ui) = checked(notes_feed(), Setup::new().installed(OLD));
    details(&mut ui);
    assert!(ui.has("Version 0.1.2 \u{2192} 0.2.0 \u{00B7} Dec 1, 2026"));
    assert!(ui.has("Desktop publishing for bulletins, newsletters and flyers. Opens Publisher (.pub) files too."));
    // The update list's feature list replaces the built-in one.
    assert!(ui.has("Spell check") && ui.has("Templates"));
    // Info boxes.
    assert!(ui.has("0.2.0") && ui.has("Dec 1, 2026") && ui.has("29.4 MB") && ui.has("Windows 64-bit"));
    assert!(ui.has("Full changelog"));
    for l in ["Website", "GitHub", "Releases"] {
        assert!(ui.enabled_last(l), "{l}");
    }
}

#[test]
fn details_link_buttons_open_their_urls() {
    let (_f, mut ui) = checked(notes_feed(), Setup::new().installed(OLD));
    details(&mut ui);
    ui.click_last("Website");
    ui.click_last("GitHub");
    ui.click_last("Releases");
    ui.click_last("Full changelog");
    assert_eq!(
        ui.fake(|f| f.urls.clone()),
        [
            "https://example.org/kernpage",
            "https://github.com/example/kernpage",
            "https://github.com/example/kernpage/releases",
            "https://github.com/example/kernpage/blob/main/CHANGELOG.md",
        ]
    );
}

#[test]
fn details_whats_changed_shows_six_then_all() {
    let (_f, mut ui) = checked(notes_feed(), Setup::new().installed(OLD));
    details(&mut ui);
    assert!(ui.has_text("Spell check with downloadable dictionaries (#212)"));
    assert!(ui.has_text("Zoom to fit page and zoom to selection (#176)"));
    assert!(!ui.has_text("Linked text boxes lost their link"), "only the first six at first");
    assert!(ui.has("Show 6 more changes"));
    ui.click_last("Show 6 more changes");
    assert!(ui.has_text("Linked text boxes lost their link after a table was resized (#219)"));
    assert!(ui.has_text("Clearer messages when a .pub file cannot be opened (#190)"));
    assert!(ui.has("Show fewer") && !ui.has("Show 6 more changes"));
    ui.click_last("Show fewer");
    assert!(!ui.has_text("Linked text boxes lost their link"));
    assert!(ui.has("Show 6 more changes"));
}

#[test]
fn details_without_notes_says_so() {
    // The "real" release in the feed has one note; a missing note list is covered by views tests.
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    details(&mut ui);
    assert!(ui.has_text("A real installer served by the local test feed"));
    assert!(!ui.has("Show fewer"));
}

#[test]
fn details_close_button_and_escape() {
    let mut ui = TestUi::start();
    details(&mut ui);
    ui.hover_last("\u{00D7}");
    assert!(ui.has("Close (Esc)"));
    ui.click_last("\u{00D7}");
    assert!(!ui.has("01 / FEATURES"));
    details(&mut ui);
    ui.key(Key::Escape);
    assert!(!ui.has("01 / FEATURES"));
}

#[test]
fn details_previous_and_next_move_between_the_app_and_the_add_on() {
    let (_f, mut ui) = checked(notes_feed(), Setup::new().installed(OLD));
    details(&mut ui);
    assert!(!ui.enabled_last("\u{2039}"), "no previous before the first card");
    assert!(ui.enabled_last("\u{203A}"));
    ui.hover_last("\u{203A}");
    assert!(ui.has("Next"));
    ui.click_last("\u{203A}");
    assert!(ui.has("Spelling and hyphenation for American English. Installs per user; no administrator rights needed."));
    assert!(ui.has("Version 2026.10.0 \u{00B7} Oct 20, 2026"));
    assert!(ui.has("American spelling"));
    assert!(!ui.enabled_last("\u{203A}"), "no next after the last card");
    assert!(ui.enabled_last("\u{2039}"));
    ui.hover_last("\u{2039}");
    assert!(ui.has("Previous"));
    ui.click_last("\u{2039}");
    assert!(ui.has("Opens Publisher files") || ui.has("Spell check"));
    assert!(!ui.has("American spelling"));
}

#[test]
fn details_of_an_add_on_has_install_and_opens_the_stub() {
    let (_f, mut ui) = checked(notes_feed(), Setup::new().installed(OLD));
    ui.click_nth("...", 1);
    ui.click("What's new");
    assert!(ui.has("American spelling"));
    ui.click_last("INSTALL");
    assert!(ui.has("NOT BUILT YET"));
}

#[test]
fn details_open_and_update_buttons_work_in_the_modal() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    details(&mut ui);
    ui.click_last("OPEN 0.1.2");
    assert_eq!(ui.fake(|f| f.launched.clone()), vec![None]);
    ui.click_last("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.has("Ready to install"));
}

#[test]
fn details_blocked_note_is_shown() {
    let (_f, mut ui) = checked(FeedOpts { block: Some("<0.2.0".into()), ..Default::default() }, Setup::new().installed(OLD));
    details(&mut ui);
    assert!(ui.has_text("This version is blocked: This version can lose data when saving."));
}

// ---- rollback --------------------------------------------------------------------------------------------

fn older_feed() -> FeedOpts {
    FeedOpts { older: Some("0.0.9".into()), ..Default::default() }
}

fn open_list(ui: &mut TestUi) {
    ui.open_more();
    ui.click("Roll back");
    assert!(ui.has("Go back to an older version"));
}

#[test]
fn roll_back_is_enabled_only_when_an_older_version_is_offered() {
    let (_f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    ui.open_more();
    assert!(ui.enabled("Roll back"));
    let (_f2, mut ui2) = checked(older_feed(), Setup::new().installed("0.0.5"));
    ui2.open_more();
    assert!(!ui2.enabled("Roll back"), "nothing is older than 0.0.5");
    ui2.hover("Roll back");
    assert!(ui2.has("No older versions are available"));
}

#[test]
fn rollback_list_shows_older_versions() {
    let (_f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    open_list(&mut ui);
    assert!(ui.has("Go back if a newer version is giving you trouble. Your publications are not changed."));
    assert!(ui.has("Version 0.0.9 \u{00B7} Jun 1, 2026"));
    assert!(ui.has("Go back to this version"));
    assert!(ui.has("Close"));
}

#[test]
fn rollback_list_closes_with_close_and_escape() {
    let (_f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    open_list(&mut ui);
    ui.click("Close");
    assert!(!ui.has("Go back to an older version"));
    open_list(&mut ui);
    ui.key(Key::Escape);
    assert!(!ui.has("Go back to an older version"));
}

#[test]
fn rollback_confirmation_cancel_starts_nothing() {
    let (f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    open_list(&mut ui);
    ui.click("Go back to this version");
    assert!(ui.has("Go back to KernPage 0.0.9?"));
    assert!(ui.has("Your publications are not changed."));
    let hits = f.server.hits();
    ui.click("Cancel");
    assert!(!ui.has("Go back to KernPage 0.0.9?"));
    assert!(ui.has("Go back to this version"), "back to the list");
    ui.h.run_steps(20);
    assert_eq!(f.server.hits(), hits, "nothing was downloaded");
    assert!(ui.fake(|x| x.helpers.is_empty()));
}

#[test]
fn rollback_go_back_and_restart_downloads_and_hands_over_to_the_helper() {
    let (f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    f.close_gate();
    open_list(&mut ui);
    ui.click("Go back to this version");
    ui.click("Go back and restart");
    assert!(!ui.has("Go back to KernPage 0.0.9?"));
    ui.h.run_steps(5);
    assert!(ui.has("Going back to version 0.0.9"));
    assert!(ui.has("Cancel"));
    f.open_gate();
    ui.wait_until("the helper", |u| u.fake(|x| !x.helpers.is_empty()));
    let call = ui.fake(|x| x.helpers[0].clone());
    assert_eq!(call.version, "0.0.9");
    assert!(call.msi.is_file());
    assert_eq!(std::fs::read(&call.msi).unwrap(), b"older installer 0.0.9");
}

#[test]
fn rollback_download_failure_offers_retry() {
    let (f, mut ui) = checked(older_feed(), Setup::new().installed(OLD));
    let path = "/files/kernpage-0.0.9-x64.msi".to_owned();
    let bytes = f.files.lock().unwrap().get(&path).cloned().expect("older file served");
    f.remove(&path);
    open_list(&mut ui);
    ui.click("Go back to this version");
    ui.click("Go back and restart");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(ui.has_text("The download server answered with an error (HTTP 404)."));
    f.files.lock().unwrap().insert(path, bytes);
    ui.click("Retry");
    ui.wait_until("the helper", |u| u.fake(|x| !x.helpers.is_empty()));
    assert_eq!(ui.fake(|x| x.helpers[0].version.clone()), "0.0.9");
}

#[test]
fn rollback_in_a_dev_build_downloads_but_cannot_install() {
    let (_f, mut ui) = checked(older_feed(), Setup::new().installed(OLD).scope(crate::install::InstallScope::NotInstalled));
    open_list(&mut ui);
    ui.click("Go back to this version");
    ui.click("Go back and restart");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(!ui.enabled_last("Install and restart"));
    ui.hover_last("Install and restart");
    assert!(ui.has("Installing works only in an installed copy"));
    assert!(ui.fake(|x| x.helpers.is_empty()));
}

#[test]
fn rollback_on_a_setup_exe_install_goes_back_with_the_older_setup_file() {
    let (_f, mut ui) = checked(FeedOpts { setup: true, older: Some("0.0.9".into()), ..Default::default() }, Setup::new().installed(OLD).bundle());
    open_list(&mut ui);
    ui.click("Go back to this version");
    ui.click("Go back and restart");
    ui.wait_until("the helper", |u| u.fake(|x| !x.helpers.is_empty()));
    let call = ui.fake(|x| x.helpers[0].clone());
    assert_eq!(call.version, "0.0.9");
    assert_eq!(call.msi.file_name().unwrap().to_string_lossy(), "KernPage-0.0.9-Setup.exe");
    assert_eq!(std::fs::read(&call.msi).unwrap(), b"older setup 0.0.9");
}
