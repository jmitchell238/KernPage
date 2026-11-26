//! The "Update available" notice: hidden unless the background check reports an update, and its
//! button starts the launcher. The check and the launcher are test seams (no process, no network).

use super::support::*;
use crate::update_notice::UpdateNotice;
use egui_kittest::kittest::Queryable;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Let the (instant) fake check answer and the frames after it run.
fn answer(ui: &mut TestUi) {
    let t = Instant::now();
    while ui.app().update.available.is_none() && t.elapsed() < Duration::from_millis(300) {
        std::thread::sleep(Duration::from_millis(5));
        ui.h.run_steps(1);
    }
    ui.settle();
}

fn with_probe(ui: &mut TestUi, found: Option<&'static str>) {
    let ctx = ui.h.ctx.clone();
    ui.app().update = UpdateNotice::with_probe(&ctx, move || found.map(str::to_owned));
}

#[test]
fn no_notice_without_an_update() {
    let mut ui = TestUi::start();
    with_probe(&mut ui, None);
    std::thread::sleep(Duration::from_millis(50));
    ui.settle();
    assert!(ui.h.query_by_label("Update...").is_none());
    assert!(ui.h.query_by_label_contains("Update available").is_none());
}

#[test]
fn notice_shows_the_version_when_an_update_is_reported() {
    let mut ui = TestUi::start();
    with_probe(&mut ui, Some("9.9.9"));
    answer(&mut ui);
    ui.h.get_by_label("Update available: KernPage 9.9.9");
    ui.h.get_by_label("Update...");
}

#[test]
fn the_button_starts_the_launcher_and_the_notice_goes_away() {
    let mut ui = TestUi::start();
    with_probe(&mut ui, Some("9.9.9"));
    let launched = Rc::new(Cell::new(0));
    let l = launched.clone();
    ui.app().update.set_launch(move || {
        l.set(l.get() + 1);
        true
    });
    answer(&mut ui);
    ui.h.get_by_label("Update...").click();
    ui.settle();
    assert_eq!(launched.get(), 1);
    assert!(ui.h.query_by_label("Update...").is_none());
}

#[test]
fn a_launcher_that_cannot_start_keeps_the_notice() {
    let mut ui = TestUi::start();
    with_probe(&mut ui, Some("9.9.9"));
    ui.app().update.set_launch(|| false);
    answer(&mut ui);
    ui.h.get_by_label("Update...").click();
    ui.settle();
    ui.h.get_by_label("Update...");
}

#[test]
fn dismiss_hides_the_notice_without_starting_anything() {
    let mut ui = TestUi::start();
    with_probe(&mut ui, Some("9.9.9"));
    let launched = Rc::new(Cell::new(0));
    let l = launched.clone();
    ui.app().update.set_launch(move || {
        l.set(l.get() + 1);
        true
    });
    answer(&mut ui);
    ui.h.get_by_label("Dismiss").click();
    ui.settle();
    assert_eq!(launched.get(), 0);
    assert!(ui.h.query_by_label("Update...").is_none());
}
