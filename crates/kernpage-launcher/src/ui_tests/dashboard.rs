//! Top bar, toolbar, footer, notices, the install report, announce dialog and the card states.

use super::support::*;
use crate::install::InstallResult;
use crate::views::State;

#[test]
fn topbar_and_toolbar_controls_are_present() {
    let mut ui = TestUi::start();
    for l in ["KERNPAGE", "LAUNCHER", "DASHBOARD", "Settings", "Refresh", "Open file...", "INSTALLED", "AVAILABLE TO INSTALL", "RECENT FILES", "Clear list"] {
        assert!(ui.has(l), "{l} is on screen");
    }
    ui.hover("Refresh");
    assert!(ui.has("Reload the recent files list and check for updates"));
    ui.hover("Open file...");
    assert!(ui.has("Open a publication in KernPage"));
}

#[test]
fn no_feed_yet_shows_the_installed_card_and_no_update_controls() {
    let mut ui = TestUi::start();
    assert_eq!(ui.badges(), ["INSTALLED"]);
    assert!(ui.has("OPEN") && !ui.has_text("UPDATE ALL") && !ui.has_text("UPDATE TO"));
    assert!(ui.has_text("Not checked yet"));
    assert!(ui.has("Manual updates: nothing is sent until you check"));
    assert!(ui.has("Add-ons such as template packs and dictionaries appear here after an update check."));
    assert!(ui.has_text("VERSION"));
    assert_eq!(ui.state(), State::Current);
    assert!(ui.fake(|f| f.launched.is_empty()));
}

#[test]
fn manual_mode_sends_nothing_until_refresh() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed));
    ui.h.run_steps(30);
    assert_eq!(feed.server.hits(), 0);
    ui.refresh();
    assert!(feed.server.hits() >= 2, "index and signature were fetched");
    assert!(ui.has_text("Checked just now"));
    assert!(ui.has_text("using local test feed"));
}

#[test]
fn update_available_state() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    assert_eq!(ui.badges()[0], "UPDATE AVAILABLE");
    assert!(ui.has("UPDATE ALL (1)") && ui.has("UPDATE TO 0.2.0") && ui.has("OPEN 0.1.2"));
    for l in ["DOWNLOAD", "LAST UPDATED", "Dec 1, 2026", "0.1.2", "0.2.0"] {
        assert!(ui.has(l), "{l}");
    }
    assert_eq!(ui.state(), State::Update);
    assert_eq!(ui.badges()[1], "NOT INSTALLED");
    assert!(ui.has("Spelling dictionary: English (US)"));
}

#[test]
fn up_to_date_state() {
    let (_f, ui) = checked(FeedOpts::default(), Setup::new().installed(NEW));
    assert_eq!(ui.badges()[0], "INSTALLED");
    assert!(!ui.has_text("UPDATE ALL") && !ui.has_text("UPDATE TO"));
    assert!(ui.has("OPEN"));
    assert!(ui.fake(|f| f.toasts.is_empty()));
    assert!(!ui.has_text("is available"));
}

#[test]
fn a_newer_installed_version_is_not_offered_a_downgrade() {
    let (_f, ui) = checked(FeedOpts::default(), Setup::new().installed("0.9.0"));
    assert_eq!(ui.badges()[0], "INSTALLED");
    assert!(!ui.has_text("UPDATE TO"));
}

#[test]
fn editor_missing_shows_not_found() {
    let mut ui = TestUi::with(Setup::new().no_editor());
    assert_eq!(ui.badges()[0], "NOT FOUND");
    assert_eq!(ui.state(), State::Missing);
    ui.click("OPEN");
    assert!(ui.has_text("was not found next to the launcher"));
    assert!(ui.fake(|f| f.launched.is_empty()));
}

#[test]
fn blocked_installed_version_is_flagged() {
    let (_f, ui) = checked(FeedOpts { block: Some("<0.3.0".into()), ..Default::default() }, Setup::new().installed(NEW));
    assert_eq!(ui.badges()[0], "BLOCKED");
    assert!(ui.has_text("This version is blocked: This version can lose data when saving."));
}

#[test]
fn blocked_old_version_still_offers_the_update_with_a_note() {
    let (_f, ui) = checked(FeedOpts { block: Some("<0.2.0".into()), ..Default::default() }, Setup::new().installed(OLD));
    assert_eq!(ui.badges()[0], "UPDATE AVAILABLE");
    assert!(ui.has_text("This version is blocked"));
}

#[test]
fn a_release_with_a_bad_checksum_entry_is_not_offered() {
    let (_f, ui) = checked(FeedOpts { bad_sha: true, real: false, ..Default::default() }, Setup::new().installed(OLD));
    assert_eq!(ui.badges()[0], "INSTALLED");
    assert!(ui.has_text("failed its checks"));
    assert!(!ui.has_text("UPDATE TO"));
}

fn failed_check(opts: FeedOpts, notice: &str, footer: &str) {
    let feed = FeedServer::start(opts);
    let mut ui = TestUi::with(Setup::new().feed(&feed).installed(OLD));
    ui.refresh();
    assert!(ui.has_text(notice), "notice {notice:?} in {:?}", ui.texts());
    assert!(ui.has_text(footer), "footer {footer:?}");
    assert!(!ui.has_text("UPDATE ALL") && !ui.has_text("UPDATE TO"), "a refused list offers nothing");
    ui.click("Dismiss");
    assert!(!ui.has_text(notice), "Dismiss closes the notice");
    assert!(ui.has_text(footer), "the footer keeps saying so");
}

#[test]
fn unreachable_feed_shows_notice_and_footer() {
    let mut ui = TestUi::start();
    ui.refresh();
    assert!(ui.has_text("Could not reach the update server"));
    assert!(ui.has_text("Couldn't check: update server not reachable"));
    ui.click("Dismiss");
    assert!(!ui.has_text("Could not reach the update server"));
    assert_eq!(ui.badges(), ["INSTALLED"]);
}

#[test]
fn tampered_feed_is_refused() {
    failed_check(FeedOpts { tamper: true, ..Default::default() }, "Could not verify the update list", "update list could not be verified");
}

#[test]
fn feed_signed_with_the_wrong_key_is_refused() {
    failed_check(FeedOpts { wrong_key: true, ..Default::default() }, "signed with a key this launcher does not trust", "update list could not be verified");
}

#[test]
fn expired_feed_is_refused() {
    failed_check(FeedOpts { expired: true, ..Default::default() }, "The update list has expired", "update list has expired");
}

#[test]
fn an_older_feed_than_already_seen_is_refused() {
    let feed = FeedServer::start(FeedOpts { serial: 3, ..Default::default() });
    let mut ui = TestUi::with(Setup::new().feed(&feed).config("last_serial = 9\n"));
    ui.refresh();
    assert!(ui.has_text("is older than one already seen"));
}

#[test]
fn a_successful_check_remembers_the_serial() {
    let env = TestEnv::new();
    let feed = FeedServer::start(FeedOpts { serial: 7, ..Default::default() });
    let mut ui = TestUi::with(Setup::new().env(&env).feed(&feed));
    ui.refresh();
    assert!(env.config_text().contains("last_serial = 7"));
}

#[test]
fn footer_says_checking_during_a_check() {
    let feed = FeedServer::start(FeedOpts::default());
    feed.close_gate();
    let mut ui = TestUi::with(Setup::new().feed(&feed));
    ui.app().check_now_by_user();
    ui.h.step();
    assert!(ui.app().updater.is_checking());
    assert!(ui.has("Checking for updates..."), "{:?}", ui.texts());
    assert!(ui.has("Checking..."));
    ui.wait_check();
    assert!(!ui.has("Checking for updates..."));
}

#[test]
fn footer_shows_the_automatic_interval() {
    let ui = TestUi::with(Setup::new().config("check_updates = \"automatic\"\ncheck_hours = 12\n"));
    assert!(ui.has("Automatic updates every 12 hours"));
}

#[test]
fn automatic_mode_checks_by_itself_at_start() {
    let feed = FeedServer::start(FeedOpts::default());
    let mut ui = TestUi::with(Setup::new().feed(&feed).config("check_updates = \"automatic\"\n"));
    ui.wait_until("the automatic check", |u| u.app().updater.snapshot.is_some());
    assert!(feed.server.hits() >= 2);
}

// ---- notice bar --------------------------------------------------------------------------------

#[test]
fn notice_bar_shows_a_launch_error_and_dismisses() {
    let mut ui = TestUi::start();
    ui.fake(|f| f.launch_error = Some("Could not start KernPage: boom".into()));
    ui.click("OPEN");
    assert!(ui.has_text("Could not start KernPage: boom"));
    ui.click("Dismiss");
    assert!(!ui.has_text("boom"));
    ui.fake(|f| f.launch_error = None);
    ui.click("OPEN");
    assert_eq!(ui.fake(|f| f.launched.clone()), vec![None]);
}

// ---- install report bar -------------------------------------------------------------------------------

fn report(ok: bool, code: u32, msg: &str, log: &str) -> InstallResult {
    InstallResult { target_version: "0.2.0".into(), ok, exit_code: Some(code), message: msg.into(), log_path: log.into(), finished_at: 1 }
}

#[test]
fn report_success_banner() {
    let mut ui = TestUi::with(Setup::new().report(report(true, 0, "", "")));
    assert!(ui.has("KernPage 0.2.0 is installed"));
    assert!(!ui.has("Show log"));
    ui.click("Dismiss");
    assert!(!ui.has_text("is installed"));
    assert!(!ui.env.updates.join(crate::install::RESULT_FILE).exists(), "shown once: the result file was consumed");
}

#[test]
fn report_restart_needed_banner() {
    let ui = TestUi::with(Setup::new().report(report(true, 3010, "", "")));
    assert!(ui.has("KernPage 0.2.0 is installed. Restart Windows to finish the update."));
}

#[test]
fn report_failure_banner_with_show_log() {
    let mut ui = TestUi::with(Setup::new().report(report(false, 1603, "The update could not be installed (error 1603).", r"C:\logs\install.log")));
    assert!(ui.has("The update could not be installed (error 1603)."));
    assert!(ui.has("Show log") && ui.has("Dismiss"));
    ui.click("Show log");
    assert_eq!(ui.fake(|f| f.logs.clone()), [r"C:\logs\install.log"]);
    ui.click("Dismiss");
    assert!(!ui.has_text("error 1603"));
}

#[test]
fn report_failure_without_a_log_has_no_show_log() {
    let ui = TestUi::with(Setup::new().report(report(false, 1602, "The update was cancelled.", "")));
    assert!(ui.has("The update was cancelled.") && !ui.has("Show log"));
}

// ---- toasts -----------------------------------------------------------------------------------------

#[test]
fn an_update_toasts_once_and_is_saved() {
    let env = TestEnv::new();
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().env(&env).installed(OLD));
    assert_eq!(ui.fake(|f| f.toasts.clone()), [("KernPage".to_owned(), "0.2.0".to_owned())]);
    assert!(env.config_text().contains("last_notified = \"kernpage@0.2.0\""));
    ui.refresh();
    assert_eq!(ui.fake(|f| f.toasts.len()), 1, "not again for the same version");
}

#[test]
fn no_toast_when_notifications_are_off() {
    let (_f, ui) = checked(FeedOpts::default(), Setup::new().installed(OLD).config("notify = false\n"));
    assert!(ui.fake(|f| f.toasts.is_empty()));
}

// ---- announce dialog -----------------------------------------------------------------------------------

#[test]
fn announce_dialog_content_and_later() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD));
    assert!(ui.has("KernPage 0.2.0 is available"));
    assert!(ui.has("You have 0.1.2. Released Dec 1, 2026."));
    assert!(ui.has("What's new") && ui.has("Later") && ui.has("Update and restart"));
    ui.hover("Later");
    assert!(ui.has("Ask again next time the launcher starts"));
    ui.click("Later");
    assert!(!ui.has_text("is available"));
    ui.refresh();
    assert!(!ui.has_text("0.2.0 is available"), "Later holds for this session");
}

#[test]
fn announce_dialog_whats_new_opens_the_details() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click("What's new");
    assert!(!ui.has_text("0.2.0 is available"));
    assert!(ui.has("01 / FEATURES"));
}

#[test]
fn announce_dialog_escape_acts_like_later() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD));
    ui.key(eframe::egui::Key::Escape);
    assert!(!ui.has_text("0.2.0 is available"));
}

#[test]
fn announce_update_and_restart_downloads_then_hands_over_to_the_helper() {
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click("Update and restart");
    ui.wait_until("the helper to be started", |u| u.fake(|f| !f.helpers.is_empty()));
    let call = ui.fake(|f| f.helpers[0].clone());
    assert_eq!(call.version, "0.2.0");
    assert!(call.msi.is_file(), "the verified installer is on disk");
    assert!(call.msi.starts_with(&ui.env.updates));
    assert_eq!(call.updates_dir, ui.env.updates);
    assert!(call.feed_url.unwrap().starts_with("http://127.0.0.1:"));
}

#[test]
fn skip_this_version_hides_the_update_and_can_be_undone() {
    let env = TestEnv::new();
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().env(&env).installed(OLD));
    ui.open_more();
    ui.click("Skip this version");
    assert_eq!(ui.badges()[0], "UPDATE SKIPPED");
    assert!(ui.has_text("You skipped version 0.2.0."));
    assert!(!ui.has_text("UPDATE ALL"));
    assert!(env.config_text().contains("skipped = \"kernpage@0.2.0\""));
    ui.open_more();
    ui.click("Stop skipping 0.2.0");
    assert_eq!(ui.badges()[0], "UPDATE AVAILABLE");
    assert!(env.config_text().contains("skipped = \"\""));
}

#[test]
fn skipped_version_is_not_announced() {
    let (_f, ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD).config("skipped = \"kernpage@0.2.0\"\n"));
    assert!(!ui.has_text("0.2.0 is available"));
}

// ---- toolbar actions ------------------------------------------------------------------------------------

#[test]
fn open_file_launches_the_editor_with_the_picked_file() {
    let mut ui = TestUi::start();
    let f = ui.env.pub_file("a.pub");
    ui.fake(|x| x.picks.push_back(Some(f.clone())));
    ui.click("Open file...");
    assert_eq!(ui.fake(|x| (x.picks_asked, x.launched.clone())), (1, vec![Some(f)]));
}

#[test]
fn open_file_cancel_launches_nothing() {
    let mut ui = TestUi::start();
    ui.fake(|x| x.picks.push_back(None));
    ui.click("Open file...");
    assert_eq!(ui.fake(|x| (x.picks_asked, x.launched.len())), (1, 0));
}

#[test]
fn update_all_starts_every_download() {
    let (_f, mut ui) = checked(FeedOpts::default(), Setup::new().installed(OLD));
    ui.click("UPDATE ALL (1)");
    ui.wait_until("ready", |u| u.has("Install and restart"));
    assert!(ui.has("Ready to install"));
    assert!(ui.fake(|f| f.helpers.is_empty()), "downloading is not installing");
}

#[test]
fn update_on_a_setup_exe_install_downloads_the_setup_file_not_the_msi() {
    let (_f, mut ui) = checked_raw(FeedOpts { setup: true, ..Default::default() }, Setup::new().installed(OLD).bundle());
    ui.click("Update and restart");
    ui.wait_until("the helper to be started", |u| u.fake(|f| !f.helpers.is_empty()));
    let call = ui.fake(|f| f.helpers[0].clone());
    assert_eq!(call.version, "0.2.0");
    assert_eq!(call.msi.file_name().unwrap().to_string_lossy(), "KernPage-0.2.0-Setup.exe");
    assert_eq!(std::fs::read(&call.msi).unwrap(), SETUP_INSTALLER, "the Setup.exe bytes, verified");
    assert!(call.msi.starts_with(&ui.env.updates));
}

#[test]
fn update_on_a_plain_msi_install_still_downloads_the_msi() {
    let (_f, mut ui) = checked_raw(FeedOpts { setup: true, ..Default::default() }, Setup::new().installed(OLD));
    ui.click("Update and restart");
    ui.wait_until("the helper to be started", |u| u.fake(|f| !f.helpers.is_empty()));
    let call = ui.fake(|f| f.helpers[0].clone());
    assert_eq!(call.msi.extension().unwrap(), "msi");
    assert_eq!(std::fs::read(&call.msi).unwrap(), INSTALLER);
}

#[test]
fn a_setup_exe_install_is_not_offered_an_msi_only_release() {
    // The feed has no Setup.exe: installing the MSI would orphan the bundle, so nothing is downloaded.
    let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().installed(OLD).bundle());
    ui.click("Update and restart");
    ui.h.run_steps(20);
    assert!(ui.fake(|f| f.helpers.is_empty()), "no installer was started");
    assert!(ui.has_text("The update list has no download for this computer."));
}
