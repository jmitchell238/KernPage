//! Download progress, cancel, retry, verification errors, installing and the auto-install path.

use super::support::*;
use crate::install::InstallScope;
use eframe::egui::{self, Key};

const FILE: &str = "/files/kernpage-0.2.0-x64.msi";

fn start(setup: Setup) -> (FeedServer, TestUi) {
    checked(FeedOpts::default(), setup.installed(OLD))
}

#[test]
fn download_shows_progress_and_cancel_then_resumes() {
    let (f, mut ui) = start(Setup::new());
    f.close_gate();
    ui.click("UPDATE TO 0.2.0");
    ui.h.run_steps(5);
    assert!(ui.has_text("Downloading"), "{:?}", ui.texts());
    assert!(ui.has("Cancel"));
    assert!(!ui.has("UPDATE TO 0.2.0"));
    ui.hover("Cancel");
    assert!(ui.has("Stop downloading; the next Update resumes where this stopped"));
    ui.click("Cancel");
    f.open_gate();
    ui.wait_until("the cancelled download to end", |u| u.has("UPDATE TO 0.2.0"));
    assert!(!ui.has_text("Downloading") && !ui.has("Ready to install"));
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.has("Ready to install"));
}

#[test]
fn a_second_download_waits_in_the_queue_and_can_be_cancelled() {
    let feed = FeedServer::start(FeedOpts { older: Some("0.0.9".into()), ..Default::default() });
    let mut ui = TestUi::with(Setup::new().feed(&feed).installed(OLD));
    ui.refresh();
    ui.click("Later");
    feed.close_gate();
    ui.click("UPDATE TO 0.2.0");
    ui.app().start_rollback("0.0.9");
    ui.app().rollback_list = true;
    ui.h.run_steps(5);
    assert!(ui.has("Waiting for another download to finish..."));
    assert!(ui.count("Cancel") >= 1);
    ui.click_last("Cancel");
    ui.h.run_steps(5);
    assert!(!ui.has("Waiting for another download to finish..."));
    feed.open_gate();
}

#[test]
fn download_failure_shows_the_error_and_retry_resumes() {
    let (f, mut ui) = start(Setup::new());
    let bytes = f.files.lock().unwrap().get(FILE).cloned().unwrap();
    f.remove(FILE);
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(ui.has("The download server answered with an error (HTTP 404)."));
    assert!(!ui.has("Install and restart"));
    ui.hover("Retry");
    assert!(ui.has("Download again (resumes if possible)"));
    f.files.lock().unwrap().insert(FILE.into(), bytes);
    ui.click("Retry");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(!ui.has("Retry"));
}

#[test]
fn a_file_that_does_not_match_its_checksum_is_refused_and_deleted() {
    let (f, mut ui) = start(Setup::new());
    let mut bytes = f.files.lock().unwrap().get(FILE).cloned().unwrap();
    bytes[0] ^= 0xFF; // same size, different content
    f.files.lock().unwrap().insert(FILE.into(), bytes);
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(ui.has("The downloaded update did not match its checksum, so it was deleted. Nothing was installed."));
    assert!(!ui.has("Install and restart"));
    let dir = ui.env.updates.join("kernpage-0.2.0");
    assert!(!dir.join("kernpage-0.2.0-x64.msi").exists(), "no finished file");
    assert!(!dir.join("kernpage-0.2.0-x64.msi.part").exists(), "the part file is gone");
}

#[test]
fn a_file_shorter_than_listed_ends_early_with_a_message() {
    let (f, mut ui) = start(Setup::new());
    let bytes = f.files.lock().unwrap().get(FILE).cloned().unwrap();
    f.files.lock().unwrap().insert(FILE.into(), bytes[..10].to_vec());
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(ui.has_text("The download ended early"));
}

#[test]
fn a_file_larger_than_listed_is_stopped() {
    let (f, mut ui) = start(Setup::new());
    let mut bytes = f.files.lock().unwrap().get(FILE).cloned().unwrap();
    bytes.extend_from_slice(b"extra");
    f.files.lock().unwrap().insert(FILE.into(), bytes);
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(ui.has("The download is larger than the update list says, so it was stopped and deleted. Nothing was installed."));
}

#[test]
fn a_placeholder_file_with_the_wrong_size_cannot_be_installed() {
    // The sample feed's placeholder entry lists 29 MB but serves a few bytes.
    let (_f, mut ui) = checked(FeedOpts { real: false, ..Default::default() }, Setup::new().installed(OLD));
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("failure", |u| u.has("Retry"));
    assert!(!ui.has("Install and restart") && ui.fake(|f| f.helpers.is_empty()));
}

#[test]
fn ready_to_install_hands_the_installer_to_the_helper_and_closes_the_launcher() {
    let (f, mut ui) = start(Setup::new());
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.enabled("Install and restart"));
    let cmds = ui.click_peek("Install and restart");
    let call = ui.fake(|x| x.helpers.clone());
    assert_eq!(call.len(), 1);
    assert_eq!(call[0].version, "0.2.0");
    assert_eq!(std::fs::read(&call[0].msi).unwrap(), INSTALLER);
    assert_eq!(call[0].feed_url.as_deref(), Some(f.url().as_str()));
    assert!(cmds.iter().any(|c| matches!(c, egui::ViewportCommand::Close)), "the launcher closes: {cmds:?}");
    assert!(ui.fake(|x| x.launched.is_empty()), "the editor is not started by the launcher");
}

#[test]
fn a_helper_that_cannot_start_shows_a_notice_and_keeps_the_launcher_open() {
    let (_f, mut ui) = start(Setup::new());
    ui.fake(|x| x.helper_result = Err("Could not start the update: denied".into()));
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    let cmds = ui.click_peek("Install and restart");
    assert!(ui.has_text("Could not start the update: denied"));
    assert!(!cmds.iter().any(|c| matches!(c, egui::ViewportCommand::Close)));
    assert!(ui.has("Install and restart"), "can try again");
}

#[test]
fn install_waits_while_the_editor_is_running() {
    let (_f, mut ui) = start(Setup::new().editor_running());
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.has("Close KernPage to install this update"));
    assert!(!ui.has("Ready to install"));
    assert!(!ui.enabled("Install and restart"));
    ui.hover("Install and restart");
    assert!(ui.has("Close KernPage to install this update"));
    ui.click_last("Install and restart");
    assert!(ui.fake(|x| x.helpers.is_empty()), "a greyed button does nothing");
    // The editor closes: the card notices within a second.
    ui.fake(|x| x.editor_running = false);
    ui.app().editor_poll.set(None);
    ui.settle();
    assert!(ui.has("Ready to install") && ui.enabled_last("Install and restart"));
}

#[test]
fn update_and_restart_installs_once_the_editor_is_closed() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD).editor_running());
    ui.click("Update and restart");
    ui.wait_until("ready", |u| u.has("Close KernPage to install this update"));
    ui.h.run_steps(10);
    assert!(ui.fake(|x| x.helpers.is_empty()), "waits for the editor");
    ui.fake(|x| x.editor_running = false);
    ui.app().editor_poll.set(None);
    ui.wait_until("the helper", |u| u.fake(|x| !x.helpers.is_empty()));
}

#[test]
fn dev_build_downloads_but_install_is_disabled_with_its_tooltip() {
    let (_f, mut ui) = start(Setup::new().scope(InstallScope::NotInstalled));
    ui.click("UPDATE TO 0.2.0");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.has("Ready to install"), "no 'close the editor' line in a dev copy");
    assert!(!ui.enabled("Install and restart"));
    ui.hover("Install and restart");
    assert!(ui.has("Installing works only in an installed copy"));
    ui.click_last("Install and restart");
    assert!(ui.fake(|x| x.helpers.is_empty()));
}

#[test]
fn dev_build_update_and_restart_never_installs() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD).scope(InstallScope::NotInstalled));
    ui.click("Update and restart");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    ui.h.run_steps(20);
    assert!(ui.fake(|x| x.helpers.is_empty()));
    assert!(ui.app().auto_install.is_none());
}

// ---- Download and install updates automatically ---------------------------------------------------------

const AUTO: &str = "auto_install = true\n";

#[test]
fn auto_install_with_a_visible_window_downloads_but_leaves_the_install_to_the_user() {
    let (_f, mut ui) = start(Setup::new().config(AUTO));
    ui.wait_until("downloaded", |u| u.has("Install and restart"));
    ui.h.run_steps(20);
    assert!(ui.has("Ready to install"));
    assert!(ui.fake(|x| x.helpers.is_empty()), "the window is on screen: no surprise install");
}

#[test]
fn auto_install_with_the_window_hidden_installs_by_itself() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed).installed(OLD).config(AUTO));
    ui.app().hidden_to_tray = true;
    ui.app().check_now_by_user();
    ui.wait_until("the helper", |u| {
        u.app().hidden_to_tray = true;
        u.fake(|x| !x.helpers.is_empty())
    });
    assert_eq!(ui.fake(|x| x.helpers[0].version.clone()), "0.2.0");
}

#[test]
fn auto_install_with_the_window_hidden_waits_for_a_running_editor() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed).installed(OLD).config(AUTO).editor_running());
    ui.app().hidden_to_tray = true;
    ui.app().check_now_by_user();
    ui.wait_until("downloaded", |u| {
        u.app().hidden_to_tray = true;
        u.has("Close KernPage to install this update")
    });
    ui.h.run_steps(10);
    assert!(ui.fake(|x| x.helpers.is_empty()));
}

#[test]
fn auto_install_does_nothing_when_the_setting_is_off() {
    let (_f, mut ui) = start(Setup::new());
    ui.h.run_steps(30);
    assert!(ui.has("UPDATE TO 0.2.0"), "no download started");
    assert!(ui.fake(|x| x.helpers.is_empty()));
}

#[test]
fn auto_install_skips_a_skipped_version() {
    let (_f, mut ui) = start(Setup::new().config("auto_install = true\nskipped = \"kernpage@0.2.0\"\n"));
    ui.h.run_steps(30);
    assert!(!ui.has("Install and restart") && !ui.has_text("Downloading"));
}

#[test]
fn closing_to_the_tray_hides_the_window_when_keep_in_tray_is_on() {
    let mut ui = TestUi::with(Setup::new().config("keep_in_tray = true\n"));
    assert_eq!(ui.fake(|x| x.tray_creates), 1);
    ui.h.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
    ui.h.step();
    let cmds = ui.h.output().viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
    assert!(cmds.iter().any(|c| matches!(c, egui::ViewportCommand::CancelClose)), "{cmds:?}");
    assert!(cmds.iter().any(|c| matches!(c, egui::ViewportCommand::Visible(false))));
    assert!(ui.app().hidden_to_tray);
}

#[test]
fn closing_without_keep_in_tray_just_closes() {
    let mut ui = TestUi::start();
    assert_eq!(ui.fake(|x| x.tray_creates), 0);
    ui.h.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
    ui.h.step();
    let cmds = ui.h.output().viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
    assert!(!cmds.iter().any(|c| matches!(c, egui::ViewportCommand::CancelClose)));
    let _ = Key::A;
}
