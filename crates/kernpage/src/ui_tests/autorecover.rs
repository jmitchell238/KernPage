//! AutoRecover (docs/ui/AUTORECOVER.md): the File > Options > Save page, the snapshot timer,
//! removal of snapshots, and recovery on start.
//!
//! Every test has its own AutoRecover folder and settings file (`TestEnv`); the timer is driven
//! with the test-only `AutoRecover::make_due` (the next frame is a tick) and `flush` (wait for
//! the background writer), so nothing depends on real time.

use super::support::*;
use crate::app::Section;
use eframe::egui::{self, accesskit::Role, Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::autorecover::{self as core, Meta, Settings};
use kernpage_core::geometry::in_to_emu;
use std::path::PathBuf;

const NONE: Modifiers = Modifiers::NONE;
const A: u64 = 1;
/// A process id no process has: its snapshots count as left by a crash.
const DEAD_PID: u32 = u32::MAX - 1;

fn new_doc() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
    ]))
}

fn nudge(ui: &mut TestUi) {
    ui.click_at(ui.center_of(A));
    ui.key(NONE, Key::ArrowRight);
}

fn click_button(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn file_entry(ui: &mut TestUi, name: &str) {
    if !ui.h.state().backstage {
        ui.click_tab("File");
    }
    ui.click_ribbon(&format!("nav:{name}"));
}

/// One AutoRecover timer tick, with the snapshot on disk when this returns.
fn tick(ui: &mut TestUi) {
    ui.app().ar.make_due();
    ui.h.step();
    ui.app().ar.flush();
}

fn snapshot_count(ui: &TestUi) -> usize {
    ui.env.snapshots().len()
}

fn read_snapshot(path: &std::path::Path) -> core::Snapshot {
    core::read_snapshot(path).expect("a readable snapshot")
}

fn close_request(ui: &mut TestUi) {
    ui.h
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    ui.h.step();
    ui.settle();
}

fn saved_settings(ui: &TestUi) -> Settings {
    Settings::load_from(&ui.env.settings)
}

// ---- File > Options > Save --------------------------------------------------------------------------

fn options(ui: &mut TestUi) {
    file_entry(ui, "Options");
    assert!(ui.h.state().section == Section::Options);
}

fn buf(ui: &mut TestUi) -> (bool, String) {
    ui.app().options_buf.clone().expect("the Options page is open")
}

fn click_minutes_box(ui: &mut TestUi) {
    ui.h.get_all_by_role(Role::TextInput).next().expect("the minutes box").click();
    ui.settle();
}

/// Type into the minutes box one character at a time, like keystrokes.
fn keys(ui: &mut TestUi, text: &str) {
    for c in text.chars() {
        ui.type_text(&c.to_string());
    }
}

/// Click the box, select everything and type `text`.
fn set_minutes(ui: &mut TestUi, text: &str) {
    click_minutes_box(ui);
    ui.key(CTRL, Key::A);
    keys(ui, text);
}

#[test]
fn the_save_page_shows_the_checkbox_and_the_current_interval() {
    let mut ui = new_doc();
    options(&mut ui);
    assert_eq!(buf(&mut ui), (true, "10".to_owned()), "Publisher's default: on, every 10 minutes");
    ui.h.get_by_label("Save AutoRecover information every");
    ui.h.get_by_label("minutes");
    ui.h.get_by_label("Customize how documents are saved.");
    // The other Publisher options are shown but greyed.
    for label in ["Allow background saves"] {
        assert!(ui.h.get_by_label(label).accesskit_node().is_disabled(), "{label} is greyed");
    }
}

#[test]
fn the_page_shows_the_saved_settings() {
    let env = TestEnv::new();
    Settings { enabled: false, minutes: 25 }.save_to(&env.settings).unwrap();
    let mut ui = TestUi::start_in(env);
    options(&mut ui);
    assert_eq!(buf(&mut ui), (false, "25".to_owned()));
}

#[test]
fn the_minutes_box_refuses_keystrokes_that_are_not_digits() {
    let mut ui = new_doc();
    options(&mut ui);
    set_minutes(&mut ui, "5");
    assert_eq!(buf(&mut ui).1, "5");
    keys(&mut ui, "x");
    keys(&mut ui, "-");
    keys(&mut ui, ".");
    keys(&mut ui, " ");
    assert_eq!(buf(&mut ui).1, "5", "letters, minus, dot and space are refused");
}

#[test]
fn the_minutes_box_refuses_a_keystroke_that_would_pass_120() {
    let mut ui = new_doc();
    options(&mut ui);
    set_minutes(&mut ui, "12");
    assert_eq!(buf(&mut ui).1, "12");
    keys(&mut ui, "1");
    assert_eq!(buf(&mut ui).1, "12", "121 is refused");
    keys(&mut ui, "0");
    assert_eq!(buf(&mut ui).1, "120", "120 is allowed");
    keys(&mut ui, "0");
    assert_eq!(buf(&mut ui).1, "120", "1200 is refused");
    set_minutes(&mut ui, "99");
    keys(&mut ui, "9");
    assert_eq!(buf(&mut ui).1, "99", "999 is refused");
}

#[test]
fn zero_is_allowed_while_typing_but_not_kept_by_ok() {
    let mut ui = new_doc();
    options(&mut ui);
    set_minutes(&mut ui, "0");
    assert_eq!(buf(&mut ui).1, "0", "0 can be typed");
    click_button(&mut ui, "OK");
    assert_eq!(ui.app().ar.settings.minutes, 10, "0 is not kept: the old value stays");
    assert_eq!(saved_settings(&ui).minutes, 10);
}

#[test]
fn an_empty_box_is_not_kept_either() {
    let mut ui = new_doc();
    options(&mut ui);
    click_minutes_box(&mut ui);
    ui.key(CTRL, Key::A);
    ui.key(NONE, Key::Backspace);
    assert_eq!(buf(&mut ui).1, "");
    click_button(&mut ui, "OK");
    assert_eq!(ui.app().ar.settings.minutes, 10);
}

#[test]
fn up_and_down_arrows_in_the_box_step_the_minutes() {
    let mut ui = new_doc();
    options(&mut ui);
    click_minutes_box(&mut ui);
    ui.key(NONE, Key::ArrowUp);
    assert_eq!(buf(&mut ui).1, "11");
    ui.key(NONE, Key::ArrowUp);
    assert_eq!(buf(&mut ui).1, "12");
    ui.key(NONE, Key::ArrowDown);
    assert_eq!(buf(&mut ui).1, "11");
}

#[test]
fn the_arrows_stop_at_1_and_120() {
    let mut ui = new_doc();
    options(&mut ui);
    set_minutes(&mut ui, "120");
    ui.key(NONE, Key::ArrowUp);
    assert_eq!(buf(&mut ui).1, "120", "up stops at 120");
    set_minutes(&mut ui, "1");
    ui.key(NONE, Key::ArrowDown);
    assert_eq!(buf(&mut ui).1, "1", "down stops at 1");
    ui.key(NONE, Key::ArrowUp);
    assert_eq!(buf(&mut ui).1, "2");
}

#[test]
fn the_spin_buttons_step_and_clamp_too() {
    let mut ui = new_doc();
    options(&mut ui);
    let (up, down) = crate::backstage::spin_probe::get();
    let (up, down) = (up.expect("up arrow").center(), down.expect("down arrow").center());
    ui.click_at(up);
    assert_eq!(buf(&mut ui).1, "11");
    ui.click_at(down);
    ui.click_at(down);
    assert_eq!(buf(&mut ui).1, "9");
    set_minutes(&mut ui, "120");
    let (up, _) = crate::backstage::spin_probe::get();
    ui.click_at(up.unwrap().center());
    assert_eq!(buf(&mut ui).1, "120");
    set_minutes(&mut ui, "1");
    let (_, down) = crate::backstage::spin_probe::get();
    ui.click_at(down.unwrap().center());
    assert_eq!(buf(&mut ui).1, "1");
}

#[test]
fn a_spin_button_starts_from_the_old_value_when_the_box_is_empty_or_zero() {
    let mut ui = new_doc();
    options(&mut ui);
    set_minutes(&mut ui, "0");
    let (up, _) = crate::backstage::spin_probe::get();
    ui.click_at(up.unwrap().center());
    assert_eq!(buf(&mut ui).1, "11", "0 counts as the old value 10");
}

#[test]
fn unchecking_the_box_greys_the_minutes_box_and_the_spin_buttons() {
    let mut ui = new_doc();
    options(&mut ui);
    let input = ui.h.get_all_by_role(Role::TextInput).next().unwrap();
    assert!(!input.accesskit_node().is_disabled());
    click_button(&mut ui, "Save AutoRecover information every");
    assert!(!buf(&mut ui).0);
    let input = ui.h.get_all_by_role(Role::TextInput).next().unwrap();
    assert!(input.accesskit_node().is_disabled(), "the minutes box is greyed");
    let (up, _) = crate::backstage::spin_probe::get();
    ui.click_at(up.unwrap().center());
    assert_eq!(buf(&mut ui).1, "10", "a greyed spin button does nothing");
}

#[test]
fn ok_applies_and_saves_the_settings_and_leaves_the_page() {
    let mut ui = new_doc();
    options(&mut ui);
    click_button(&mut ui, "Save AutoRecover information every");
    click_button(&mut ui, "Save AutoRecover information every"); // back on
    set_minutes(&mut ui, "25");
    click_button(&mut ui, "OK");
    assert_eq!(ui.app().ar.settings, Settings { enabled: true, minutes: 25 });
    assert_eq!(saved_settings(&ui), Settings { enabled: true, minutes: 25 }, "written to the settings file");
    assert!(!ui.app().backstage, "OK returns to the publication");
    assert!(ui.app().options_buf.is_none());
    // A new session reads them back.
    let again = TestUi::start_in(ui.env.clone());
    assert_eq!(again.h.state().ar.settings, Settings { enabled: true, minutes: 25 });
}

#[test]
fn ok_with_the_box_unchecked_turns_autorecover_off_and_persists_it() {
    let mut ui = new_doc();
    options(&mut ui);
    click_button(&mut ui, "Save AutoRecover information every");
    click_button(&mut ui, "OK");
    assert!(!ui.app().ar.settings.enabled);
    assert!(!saved_settings(&ui).enabled);
    assert_eq!(saved_settings(&ui).minutes, 10);
}

#[test]
fn cancel_discards_the_changes() {
    let mut ui = new_doc();
    options(&mut ui);
    click_button(&mut ui, "Save AutoRecover information every");
    set_minutes(&mut ui, "77");
    click_button(&mut ui, "Cancel");
    assert_eq!(ui.app().ar.settings, Settings::default());
    assert!(!ui.env.settings.exists(), "nothing was written");
    assert!(!ui.app().backstage, "Cancel returns to the publication");
    // Opening the page again shows the real settings, not the discarded edits.
    options(&mut ui);
    assert_eq!(buf(&mut ui), (true, "10".to_owned()));
}

#[test]
fn the_options_page_without_a_publication_goes_back_to_home() {
    let mut ui = TestUi::start();
    ui.h.state_mut().backstage = true;
    ui.h.state_mut().section = Section::Options;
    ui.settle();
    click_button(&mut ui, "Cancel");
    assert!(ui.h.state().section == Section::Home);
}

// ---- the timer ---------------------------------------------------------------------------------------------

#[test]
fn a_tick_writes_a_snapshot_of_a_publication_with_unsaved_changes() {
    let mut ui = new_doc();
    nudge(&mut ui);
    assert_eq!(snapshot_count(&ui), 0, "nothing before the tick");
    tick(&mut ui);
    let files = ui.env.snapshots();
    assert_eq!(files.len(), 1);
    let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("AutoRecover save - Test("), "{name}");
    let snap = read_snapshot(&files[0]);
    assert_eq!(snap.meta.display_name, "Test");
    assert_eq!(snap.meta.original_path, None);
    assert_eq!(snap.meta.pid, std::process::id());
    assert_eq!(snap.document.pages[0].shapes.len(), 2);
    let moved = snap.document.pages[0].shapes.iter().find(|s| s.id == A).unwrap().rect;
    assert_eq!(moved, ui.rect_of(A), "the snapshot holds the current content");
}

#[test]
fn nothing_is_written_for_an_unchanged_publication() {
    let mut ui = new_doc();
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 0, "a new publication nobody edited");

    let mut opened = TestUi::start();
    opened.app().open_path(&sample("two-boxes.pub"));
    opened.settle();
    tick(&mut opened);
    assert_eq!(snapshot_count(&opened), 0, "a file nobody changed");
}

#[test]
fn a_tick_before_the_interval_is_over_writes_nothing() {
    let mut ui = new_doc();
    nudge(&mut ui);
    ui.settle();
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 0, "the 10-minute timer has not fired");
}

#[test]
fn an_unchanged_publication_is_not_written_again() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    let file = ui.env.snapshots().remove(0);
    std::fs::remove_file(&file).unwrap();
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 0, "same content as the last snapshot: no new write");
    // A further edit is written, to the same file name.
    ui.key(NONE, Key::ArrowRight);
    tick(&mut ui);
    assert_eq!(ui.env.snapshots(), vec![file], "one snapshot per document");
}

#[test]
fn a_snapshot_of_a_saved_file_remembers_where_it_came_from() {
    let mut ui = TestUi::start();
    let copy = ui.env.files.join("church.pub");
    std::fs::copy(sample("two-boxes.pub"), &copy).unwrap();
    ui.app().open_path(&copy);
    ui.settle();
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(SHIFT, Key::ArrowRight);
    tick(&mut ui);
    let snap = read_snapshot(&ui.env.snapshots()[0]);
    assert_eq!(snap.meta.display_name, "church.pub");
    assert_eq!(snap.meta.original_path.as_deref(), Some(copy.to_string_lossy().as_ref()));
}

#[test]
fn text_being_typed_is_in_the_snapshot() {
    let mut text_box = kernpage_core::Shape::new(
        kernpage_core::Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(1.0)),
        kernpage_core::ShapeKind::TextBox { paragraphs: vec![kernpage_core::Paragraph::plain("Hello")] },
    );
    text_box.fill = None;
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box]));
    ui.double_click_at(ui.center_of(A));
    ui.key(CTRL, Key::A);
    ui.type_text("Typed but not committed");
    assert_eq!(ui.text_of(A), "Hello", "not committed yet");
    tick(&mut ui);
    let snap = read_snapshot(&ui.env.snapshots()[0]);
    let text = match &snap.document.pages[0].shapes[0].kind {
        kernpage_core::ShapeKind::TextBox { paragraphs } => paragraphs[0].text(),
        _ => unreachable!(),
    };
    assert_eq!(text, "Typed but not committed", "a crash would not lose the text being typed");
}

#[test]
fn with_the_checkbox_off_no_snapshot_is_written() {
    let mut ui = new_doc();
    options(&mut ui);
    click_button(&mut ui, "Save AutoRecover information every");
    click_button(&mut ui, "OK");
    nudge(&mut ui);
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 0);
}

#[test]
fn turning_autorecover_off_deletes_the_existing_snapshot() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 1);
    options(&mut ui);
    click_button(&mut ui, "Save AutoRecover information every");
    click_button(&mut ui, "OK");
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 0);
}

// ---- removal ------------------------------------------------------------------------------------------------------

#[test]
fn saving_deletes_the_snapshot() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 1);
    ui.dialogs.script_save(Some(ui.env.files.join("kept.pub")));
    ui.key(CTRL, Key::S);
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 0, "the saved file replaces the snapshot");
}

#[test]
fn undoing_back_to_the_saved_state_deletes_the_snapshot_at_the_next_tick() {
    let mut ui = TestUi::start();
    ui.app().open_path(&sample("two-boxes.pub"));
    ui.settle();
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(SHIFT, Key::ArrowRight);
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 1);
    ui.undo();
    tick(&mut ui);
    assert_eq!(snapshot_count(&ui), 0, "nothing left to recover");
}

#[test]
fn dont_save_on_close_deletes_the_snapshot() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    file_entry(&mut ui, "Close");
    click_button(&mut ui, "  Don't Save  ");
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 0);
}

#[test]
fn cancel_on_close_keeps_the_snapshot() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    file_entry(&mut ui, "Close");
    click_button(&mut ui, "  Cancel  ");
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 1);
}

#[test]
fn a_normal_exit_deletes_the_snapshot() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    close_request(&mut ui);
    assert_eq!(snapshot_count(&ui), 1, "still there while the question is open");
    click_button(&mut ui, "  Don't Save  ");
    close_request(&mut ui); // the window closes: the real window sends the request again
    assert_eq!(snapshot_count(&ui), 0, "a normal exit leaves nothing behind");
}

#[test]
fn opening_another_publication_replaces_the_snapshot_of_the_first() {
    let mut ui = new_doc();
    nudge(&mut ui);
    tick(&mut ui);
    ui.app().open_path(&sample("two-boxes.pub"));
    ui.app().ar.flush();
    assert_eq!(snapshot_count(&ui), 0, "the first publication's snapshot is gone");
}

// ---- recovery on start --------------------------------------------------------------------------------------------------

fn write_snapshot(env: &TestEnv, name: &str, pid: u32, original: Option<&PathBuf>, saved_at: u64) -> PathBuf {
    let mut doc = letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.5), rect_shape(4.0, 3.0, 2.0, 1.5)]);
    doc.name = name.to_owned();
    let meta = Meta {
        original_path: original.map(|p| p.to_string_lossy().into_owned()),
        display_name: name.to_owned(),
        saved_at,
        pid,
    };
    let file = env.autorecover.join(core::snapshot_file_name(name, saved_at as u32));
    std::fs::create_dir_all(&env.autorecover).unwrap();
    std::fs::write(&file, core::encode(&meta, &doc).unwrap()).unwrap();
    file
}

#[test]
fn a_crashed_sessions_snapshot_is_recovered_on_start() {
    let env = TestEnv::new();
    let original = env.files.join("Flyer.pub");
    let file = write_snapshot(&env, "Flyer.pub", DEAD_PID, Some(&original), 1000);
    let mut ui = TestUi::start_in(env);
    assert!(ui.app().editor.is_some(), "the publication opened by itself");
    assert!(!ui.app().backstage);
    assert_eq!(ui.doc().name, "Flyer.pub");
    assert_eq!(ui.shapes().len(), 2);
    assert!(ui.app().recovered);
    assert!(ui.app().is_dirty(), "a recovered publication counts as having unsaved changes");
    assert_eq!(title(&mut ui), "Flyer.pub (Recovered) - KernPage");
    assert!(!file.exists(), "the snapshot file is taken at once");
    assert!(ui.app().path.as_deref() == Some(original.as_path()), "the original folder and name are remembered");
}

fn title(ui: &mut TestUi) -> String {
    ui.app().window_title().to_owned()
}

#[test]
fn saving_a_recovered_publication_is_save_as_at_the_original_folder_and_name() {
    let env = TestEnv::new();
    let original = env.files.join("Flyer.pub");
    write_snapshot(&env, "Flyer.pub", DEAD_PID, Some(&original), 1000);
    let mut ui = TestUi::start_in(env);
    ui.dialogs.script_save(Some(original.clone()));
    ui.key(CTRL, Key::S);
    let saves = ui.dialogs.saves();
    assert_eq!(saves.len(), 1, "Save asked where, it did not silently overwrite");
    assert_eq!(saves[0].title, "Save As");
    assert_eq!(saves[0].default_name, "Flyer.pub");
    assert_eq!(saves[0].default_dir.as_deref(), Some(ui.env.files.as_path()));
    assert!(original.exists(), "saved");
    assert!(!ui.app().recovered);
    assert!(!ui.app().is_dirty());
    assert_eq!(title(&mut ui), "Flyer.pub - KernPage", "no longer (Recovered)");
}

#[test]
fn cancelling_the_save_as_of_a_recovered_publication_keeps_it_recovered() {
    let env = TestEnv::new();
    write_snapshot(&env, "Flyer.pub", DEAD_PID, None, 1000);
    let mut ui = TestUi::start_in(env);
    ui.dialogs.script_save(None);
    ui.key(CTRL, Key::S);
    assert!(ui.app().recovered);
    assert!(ui.app().is_dirty());
    assert_eq!(ui.dialogs.saves()[0].default_dir, None, "no original folder recorded");
    assert_eq!(ui.dialogs.saves()[0].default_name, "Flyer.pub");
}

#[test]
fn closing_a_recovered_publication_asks_to_save() {
    let env = TestEnv::new();
    write_snapshot(&env, "Flyer.pub", DEAD_PID, None, 1000);
    let mut ui = TestUi::start_in(env);
    file_entry(&mut ui, "Close");
    assert!(matches!(ui.h.state().prompt, Some(crate::app::Prompt::Unsaved { .. })));
}

#[test]
fn a_snapshot_of_a_running_editor_is_left_alone() {
    let env = TestEnv::new();
    // This test process is a running KernPage (the test binary is called kernpage-<hash>).
    let file = write_snapshot(&env, "Live.pub", std::process::id(), None, 1000);
    let mut ui = TestUi::start_in(env);
    assert!(ui.app().editor.is_none(), "nothing recovered");
    assert!(!ui.app().recovered);
    assert!(file.exists(), "another editor's snapshot is never taken");
}

#[test]
fn an_unreadable_snapshot_is_moved_to_the_unreadable_folder() {
    let env = TestEnv::new();
    let bad = env.autorecover.join("AutoRecover save - Broken(1234).pwrec");
    std::fs::write(&bad, b"garbage, not a snapshot").unwrap();
    let mut ui = TestUi::start_in(env.clone());
    assert!(ui.app().editor.is_none(), "nothing to recover");
    assert!(ui.app().error.is_none(), "start-up is not interrupted");
    assert!(!bad.exists(), "moved out of the way");
    let moved = env.autorecover.join("unreadable").join("AutoRecover save - Broken(1234).pwrec");
    assert_eq!(std::fs::read(&moved).unwrap(), b"garbage, not a snapshot", "kept, never deleted");
}

#[test]
fn the_newest_snapshot_is_recovered_and_the_others_wait_for_the_next_start() {
    let env = TestEnv::new();
    let old = write_snapshot(&env, "Old.pub", DEAD_PID, None, 1000);
    let new = write_snapshot(&env, "New.pub", DEAD_PID, None, 2000);
    let ui = TestUi::start_in(env);
    assert_eq!(ui.doc().name, "New.pub");
    assert!(!new.exists());
    assert!(old.exists(), "the older one stays for later");
}

#[test]
fn without_a_snapshot_the_app_starts_on_the_start_screen() {
    let ui = TestUi::start();
    assert!(ui.h.state().editor.is_none(), "no snapshot: the start screen");
    assert!(ui.h.state().backstage);
}
