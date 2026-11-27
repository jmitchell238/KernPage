//! The item cards: buttons, the "..." menu, add-on cards and the install stub.

use super::support::*;
use eframe::egui::{self, Key};

#[test]
fn open_button_launches_the_editor() {
    let mut ui = TestUi::start();
    ui.hover("OPEN");
    assert!(ui.has("Open KernPage"));
    ui.click("OPEN");
    assert_eq!(ui.fake(|f| f.launched.clone()), vec![None]);
    assert!(!ui.has_text("was not found"));
}

#[test]
fn open_with_an_update_pending_still_opens_the_installed_one() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click("OPEN 0.1.2");
    assert_eq!(ui.fake(|f| f.launched.clone()), vec![None]);
}

#[test]
fn update_button_downloads_the_new_version() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.hover("UPDATE TO 0.2.0");
    assert!(ui.has("Download the new version"));
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("download ready", |u| u.has("Install and restart"));
    assert!(!ui.has("UPDATE TO 0.2.0"), "the button is replaced by the install button");
    assert!(ui.has("Ready to install"));
    assert!(ui.hover_ok("Install and restart", "Close the launcher, install the update and open it again"));
}

#[test]
fn clicking_the_card_body_opens_the_details() {
    let mut ui = TestUi::start();
    let r = ui.card_rect(0);
    ui.click_at(r.min + egui::vec2(220.0, 18.0));
    assert!(ui.has("01 / FEATURES"));
}

#[test]
fn more_menu_items_on_an_installed_card_before_any_check() {
    let mut ui = TestUi::start();
    ui.open_more();
    for l in ["What's new", "Release page", "Copy link", "Open file location", "Reinstall", "Roll back", "Uninstall"] {
        assert!(ui.has(l), "{l}");
    }
    assert!(!ui.has("Skip this version") && !ui.has("Website"));
    assert!(ui.enabled("What's new") && ui.enabled("Open file location"));
    // No feed yet: no links.
    assert!(!ui.enabled("Release page") && !ui.enabled("Copy link"));
    for l in ["Reinstall", "Roll back", "Uninstall"] {
        assert!(!ui.enabled(l), "{l} is greyed");
    }
    ui.hover("Reinstall");
    assert!(ui.has("Available once installing updates arrives (next step)"));
    ui.hover("Roll back");
    assert!(ui.has("Check for updates first"));
    ui.hover("Uninstall");
    assert!(ui.has("Available once installing updates arrives (next step)"));
}

#[test]
fn more_menu_roll_back_reason_after_a_check_without_older_versions() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.open_more();
    assert!(!ui.enabled("Roll back"));
    ui.hover("Roll back");
    assert!(ui.has("No older versions are available"));
}

#[test]
fn greyed_menu_items_do_nothing_when_clicked() {
    let mut ui = TestUi::start();
    for l in ["Reinstall", "Roll back", "Uninstall"] {
        ui.open_more();
        ui.click(l);
        assert!(!ui.has_text("Go back to an older version"), "{l}");
        assert!(ui.fake(|f| f.urls.is_empty() && f.helpers.is_empty() && f.launched.is_empty()));
        ui.key(Key::Escape);
    }
    assert!(!ui.has_text("Reinstall?"));
}

#[test]
fn more_menu_whats_new_opens_the_details() {
    let mut ui = TestUi::start();
    ui.open_more();
    ui.click("What's new");
    assert!(ui.has("01 / FEATURES"));
}

#[test]
fn more_menu_release_page_opens_the_url() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.open_more();
    assert!(ui.enabled("Release page") && ui.enabled("Copy link"));
    ui.click("Release page");
    assert_eq!(ui.fake(|f| f.urls.clone()), ["https://github.com/example/kernpage/releases"]);
}

#[test]
fn more_menu_copy_link_copies_the_release_url() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.open_more();
    let at = ui.rect("Copy link").center();
    ui.mouse_to(at);
    ui.h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    ui.h.step();
    ui.h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    ui.h.step();
    let copied = ui.h.output().platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == "https://github.com/example/kernpage/releases"));
    assert!(copied, "the link was put on the clipboard");
    assert!(ui.fake(|f| f.urls.is_empty()), "copying does not open anything");
}

#[test]
fn more_menu_open_file_location() {
    let mut ui = TestUi::start();
    ui.open_more();
    ui.click("Open file location");
    assert_eq!(ui.fake(|f| f.located), 1);
}

#[test]
fn more_menu_skip_only_when_an_update_is_offered() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(NEW));
    ui.open_more();
    assert!(!ui.has("Skip this version"));
    let (_f2, mut ui2) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui2.open_more();
    assert!(ui2.has("Skip this version"));
}

// ---- add-on cards ----------------------------------------------------------------------------------

#[test]
fn add_on_card_shows_its_facts() {
    let (_f, ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    for l in ["Spelling dictionary: English (US)", "Hunspell dictionary for American English", "VERSION 2026.10.0", "1.2 MB", "Oct 20, 2026", "INSTALL"] {
        assert!(ui.has(l), "{l}");
    }
    assert!(ui.enabled("INSTALL"));
}

#[test]
fn add_on_install_opens_the_stub_dialog() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.hover("INSTALL");
    assert!(ui.has("Installing add-ons arrives in a later step"));
    ui.click("INSTALL");
    assert!(ui.has("Install Spelling dictionary: English (US)"));
    assert!(ui.has("NOT BUILT YET"));
    assert!(ui.has("Installing add-ons arrives in a later step of the launcher. Nothing was downloaded or changed."));
    ui.click("OK");
    assert!(!ui.has("NOT BUILT YET"));
    assert!(ui.fake(|f| f.helpers.is_empty() && f.launched.is_empty()));
    assert!(ui.app().downloads.status("dict-en-us@2026.10.0") == crate::download::Status::Idle, "nothing was downloaded");
}

#[test]
fn add_on_stub_closes_with_escape() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click("INSTALL");
    assert!(ui.has("NOT BUILT YET"));
    ui.key(Key::Escape);
    assert!(!ui.has("NOT BUILT YET"));
}

#[test]
fn add_on_more_menu_has_website_and_no_app_only_items() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click_nth("...", 1);
    assert!(ui.has("Website") && ui.has("Release page") && ui.has("Copy link") && ui.has("What's new"));
    for l in ["Open file location", "Reinstall", "Roll back", "Uninstall", "Skip this version"] {
        assert!(!ui.has(l), "{l} is for the app only");
    }
    ui.click("Website");
    assert_eq!(ui.fake(|f| f.urls.clone()), ["https://example.org/kernpage/dictionaries"]);
}

#[test]
fn add_on_release_page_opens_its_own_url() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click_nth("...", 1);
    ui.click("Release page");
    assert_eq!(ui.fake(|f| f.urls.clone()), ["https://github.com/example/kernpage-addons/releases"]);
}

#[test]
fn add_on_blocked_when_nothing_compatible_is_offered() {
    let (_f, ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    assert_eq!(ui.badges()[1], "NOT INSTALLED");
}

// ---- recent files --------------------------------------------------------------------------------------

#[test]
fn recent_list_renders_rows_and_not_found() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let gone = env.files.join("gone.pub");
    let ui = TestUi::with(Setup::new().env(&env).recent(&[a.clone(), gone]));
    let rows = ui.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "alpha.pub");
    assert!(rows[0].2.starts_with(|c: char| c.is_ascii_digit()) || !rows[0].2.is_empty());
    assert_ne!(rows[0].2, "NOT FOUND");
    assert_eq!(rows[1].0, "gone.pub");
    assert_eq!(rows[1].2, "NOT FOUND");
    assert!(!ui.has("No recent files yet. Files you open or save in KernPage appear here."));
    assert!(ui.enabled("Clear list"));
}

#[test]
fn recent_empty_state_greys_clear_list() {
    let ui = TestUi::start();
    assert!(ui.rows().is_empty());
    assert!(!ui.enabled("Clear list"));
}

#[test]
fn clicking_a_recent_row_opens_it_in_the_editor() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(std::slice::from_ref(&a)));
    let r = ui.rows()[0].3;
    ui.click_at(r.min + egui::vec2(100.0, 26.0));
    assert_eq!(ui.fake(|f| f.launched.clone()), vec![Some(a)]);
}

#[test]
fn clicking_a_not_found_row_does_nothing() {
    let env = TestEnv::new();
    let gone = env.files.join("gone.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(&[gone]));
    let r = ui.rows()[0].3;
    ui.click_at(r.min + egui::vec2(100.0, 26.0));
    assert!(ui.fake(|f| f.launched.is_empty()));
    assert!(!ui.has_text("was not found"));
}

#[test]
fn remove_from_list_keeps_the_file_and_removes_one_row() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let b = env.pub_file("beta.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(&[a.clone(), b.clone()]));
    ui.hover_last("\u{00D7}");
    assert!(ui.has("Remove from list"));
    ui.click_nth("\u{00D7}", 0);
    let rows = ui.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "beta.pub");
    assert!(a.is_file() && b.is_file(), "files are untouched");
    assert!(!std::fs::read_to_string(&env.recent).unwrap().contains("alpha.pub"));
    assert!(ui.fake(|f| f.launched.is_empty()), "the x does not open the file");
}

#[test]
fn clear_list_asks_first_and_cancel_keeps_the_list() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(&[a]));
    ui.click("Clear list");
    assert!(ui.has("Clear the recent files list?"));
    assert!(ui.has("Only the list is cleared. Your files are not touched."));
    ui.click("Cancel");
    assert!(!ui.has("Clear the recent files list?"));
    assert_eq!(ui.rows().len(), 1);
}

#[test]
fn clear_list_confirmed_empties_the_list_but_not_the_files() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(std::slice::from_ref(&a)));
    ui.click("Clear list");
    ui.click_last("Clear list");
    assert!(!ui.has("Clear the recent files list?"));
    assert!(ui.rows().is_empty());
    assert!(a.is_file());
    assert!(ui.has("No recent files yet. Files you open or save in KernPage appear here."));
}

#[test]
fn clear_list_closes_with_escape() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let mut ui = TestUi::with(Setup::new().env(&env).recent(&[a]));
    ui.click("Clear list");
    ui.key(Key::Escape);
    assert!(!ui.has("Clear the recent files list?"));
    assert_eq!(ui.rows().len(), 1);
}

#[test]
fn refresh_reloads_the_recent_list_from_disk() {
    let env = TestEnv::new();
    let a = env.pub_file("alpha.pub");
    let mut ui = TestUi::with(Setup::new().env(&env));
    assert!(ui.rows().is_empty());
    std::fs::write(&env.recent, a.to_string_lossy().as_bytes()).unwrap();
    ui.click("Refresh");
    assert_eq!(ui.rows().len(), 1);
}

#[test]
fn clicking_the_card_name_or_tagline_opens_the_details() {
    // Regression: selectable labels used to swallow the click.
    for l in ["KernPage", "Desktop publishing for bulletins, newsletters and flyers. Opens Publisher (.pub) files too."] {
        let mut ui = TestUi::start();
        ui.click(l);
        assert!(ui.has("01 / FEATURES"), "clicking {l:?} opens the details");
    }
}
