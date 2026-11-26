//! The settings dialog: every control, persistence to the temp config, the OS seams.

use super::support::*;
use eframe::egui::{self, Key};

fn open() -> TestUi {
    let mut ui = TestUi::start();
    ui.click("Settings");
    assert!(ui.has("UPDATES"), "the dialog is open: {:?}", ui.texts());
    ui
}

fn open_in(env: &TestEnv) -> TestUi {
    let mut ui = TestUi::with(Setup::new().env(env));
    ui.click("Settings");
    ui
}

#[test]
fn gear_opens_the_dialog_with_every_section() {
    let ui = open();
    for l in [
        "LANGUAGE", "More languages will come later.", "UPDATES", "Check for updates", "Automatically", "Manually",
        "Download and install updates automatically", "Notify me when an update is available", "Include pre-releases", "Version",
        "Check now", "APPEARANCE", "Match system", "Dark", "Light", "BEHAVIOUR", "Keep running in the tray when closed", "Start at login",
        "COMMUNITY", "Report a problem (GitHub issues)", "Quit launcher", "Done",
    ] {
        assert!(ui.has(l), "{l} in {:?}", ui.texts());
    }
    assert!(!ui.has("These two options are available on Windows only."), "this is Windows");
    assert_eq!(ui.combos().len(), 2, "language and version boxes");
}

#[test]
fn done_and_escape_close_the_dialog() {
    let mut ui = open();
    ui.click("Done");
    assert!(!ui.has("UPDATES"));
    ui.click("Settings");
    ui.key(Key::Escape);
    assert!(!ui.has("UPDATES"));
}

#[test]
fn the_settings_option_opens_it_at_start() {
    let ui = TestUi::with(Setup::new().settings());
    assert!(ui.has("UPDATES"));
}

#[test]
fn check_mode_radios_show_the_interval_only_when_automatic() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    assert!(ui.has("Manual: nothing is sent anywhere until you press Refresh or Check now."));
    assert!(!ui.has("Check every"));
    ui.click("Automatically");
    assert!(ui.has("Check every"));
    assert_eq!(ui.combos().len(), 3, "the interval box appears");
    assert_eq!(ui.app().cfg.check_hours, 6);
    assert!(!ui.has_text("Manual: nothing is sent"));
    assert!(env.config_text().contains("check_updates = \"automatic\""));
    assert!(ui.has("Automatic updates every 6 hours"), "the footer follows");
    ui.click("Manually");
    assert!(env.config_text().contains("check_updates = \"manual\""));
    assert!(ui.has("Manual updates: nothing is sent until you check"));
}

#[test]
fn check_every_offers_the_hours_and_saves_the_choice() {
    let env = TestEnv::new();
    let mut ui = TestUi::with(Setup::new().env(&env).config("check_updates = \"automatic\"\n"));
    ui.click("Settings");
    ui.click_combo(1);
    for h in ["1 hours", "3 hours", "12 hours", "24 hours"] {
        assert!(ui.has(h), "{h}");
    }
    ui.click("12 hours");
    assert!(env.config_text().contains("check_hours = 12"));
    assert!(ui.has("Automatic updates every 12 hours"));
}

#[test]
fn update_checkboxes_toggle_and_persist() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    for (label, key, on) in [
        ("Notify me when an update is available", "notify", false),
        ("Include pre-releases", "include_prereleases", true),
        ("Download and install updates automatically", "auto_install", true),
    ] {
        ui.click(label);
        assert!(env.config_text().contains(&format!("{key} = {on}")), "{label}: {}", env.config_text());
        ui.click(label);
        assert!(env.config_text().contains(&format!("{key} = {}", !on)), "{label} toggled back");
    }
}

#[test]
fn settings_persist_across_a_restart() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    ui.click("Automatically");
    ui.click("Include pre-releases");
    ui.click("Dark");
    ui.click("Done");
    drop(ui);
    let mut ui2 = TestUi::with(Setup::new().env(&env));
    assert!(ui2.has("Automatic updates every 6 hours"));
    ui2.click("Settings");
    assert!(ui2.app().cfg.include_prereleases);
    assert_eq!(ui2.app().cfg.appearance, crate::config::Appearance::Dark);
}

#[test]
fn include_prereleases_changes_what_is_offered() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed).installed(OLD));
    ui.refresh();
    ui.click("Later");
    assert!(ui.has("UPDATE TO 0.2.0"));
    ui.click("Settings");
    ui.click("Include pre-releases");
    ui.click("Done");
    assert!(ui.has("UPDATE TO 0.3.0-beta.1"), "{:?}", ui.texts());
}

#[test]
fn check_now_checks_and_is_disabled_while_running() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed));
    ui.click("Settings");
    assert_eq!(feed.server.hits(), 0);
    ui.click("Check now");
    ui.wait_check();
    assert!(feed.server.hits() >= 2);
    assert!(ui.has_text("Checked just now"));
}

#[test]
fn version_box_is_greyed() {
    let ui = open();
    assert!(ui.has("Version"));
    let combos = ui.combos();
    assert!(combos[0].1, "language box is live");
    assert!(!combos.last().unwrap().1, "version box is greyed");
}

#[test]
fn report_a_problem_is_greyed_with_its_reason() {
    let mut ui = open();
    assert!(!ui.enabled("Report a problem (GitHub issues)"));
    ui.hover("Report a problem (GitHub issues)");
    assert!(ui.has("Available once the project is public"));
    ui.click("Report a problem (GitHub issues)");
    assert!(ui.fake(|f| f.urls.is_empty()));
}

#[test]
fn language_box_offers_only_auto() {
    let mut ui = open();
    ui.click_combo(0);
    assert_eq!(ui.count("Auto (system language)"), 1, "the one choice");
    assert!(!ui.has_text("English") && !ui.has_text("Deutsch"));
}

#[test]
fn appearance_radios_apply_and_save() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    ui.click("Dark");
    assert_eq!(ui.h.ctx.theme(), egui::Theme::Dark);
    assert!(env.config_text().contains("appearance = \"dark\""));
    ui.click("Light");
    assert_eq!(ui.h.ctx.theme(), egui::Theme::Light);
    assert!(env.config_text().contains("appearance = \"light\""));
    ui.click("Match system");
    assert!(env.config_text().contains("appearance = \"system\""));
}

#[test]
fn keep_in_tray_creates_and_removes_the_tray_icon() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    ui.click("Keep running in the tray when closed");
    assert_eq!(ui.fake(|f| f.tray_creates), 1);
    assert!(ui.app().tray.is_some());
    assert!(env.config_text().contains("keep_in_tray = true"));
    ui.click("Keep running in the tray when closed");
    assert!(ui.app().tray.is_none());
    assert!(env.config_text().contains("keep_in_tray = false"));
}

#[test]
fn a_tray_icon_that_cannot_be_made_unchecks_the_box_and_says_so() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    ui.fake(|f| f.tray_fails = true);
    ui.click("Keep running in the tray when closed");
    assert!(!ui.app().cfg.keep_in_tray);
    assert!(ui.has_text("The tray icon could not be created, so the launcher will close normally."));
    assert!(env.config_text().contains("keep_in_tray = false"));
}

#[test]
fn start_at_login_writes_and_removes_the_run_key_value() {
    let env = TestEnv::new();
    let mut ui = open_in(&env);
    ui.click("Start at login");
    let v = ui.fake(|f| f.run_key.get(crate::run_key::VALUE_NAME).cloned()).expect("value written");
    assert!(v.starts_with('"') && v.ends_with('"'), "quoted path: {v}");
    assert!(env.config_text().contains("start_at_login = true"));
    ui.click("Start at login");
    assert!(ui.fake(|f| f.run_key.is_empty()));
    assert!(env.config_text().contains("start_at_login = false"));
}

#[test]
fn a_run_key_failure_reverts_the_box_and_says_so() {
    let mut ui = open();
    ui.fake(|f| f.run_key_fails = true);
    ui.click("Start at login");
    assert!(!ui.app().cfg.start_at_login);
    assert!(ui.has_text("Could not change the start at login setting: reg add failed"));
    assert!(ui.fake(|f| f.run_key.is_empty()));
}

#[test]
fn start_at_login_follows_the_registry_at_start() {
    let ui = TestUi::with(Setup::new().run_key_on().settings());
    assert!(ui.h.state().cfg.start_at_login, "the Run key is the truth, whatever the file says");
}

#[test]
fn quit_launcher_ends_the_program() {
    let mut ui = open();
    ui.click("Quit launcher");
    assert_eq!(ui.fake(|f| f.quits), 1);
}
