//! The single top bar: the quick-access row is the title bar, with the window's minimise,
//! maximise and close buttons at its right end (see `caption.rs`).

use super::support::*;
use crate::caption::{self, BAR_H, BUTTON_W};
use eframe::egui::{Pos2, Rect, ViewportCommand};

fn button(name: &str) -> Rect {
    crate::widgets::probe::all()
        .into_iter()
        .rev()
        .find(|s| s.label == format!("caption:{name}"))
        .unwrap_or_else(|| panic!("no caption button {name}"))
        .rect
}

fn started() -> TestUi {
    let mut ui = TestUi::with_sample("showcase-newsletter.pub");
    ui.settle();
    caption::take_sent();
    ui
}

#[test]
fn buttons_sit_at_the_right_edge_in_order() {
    let _ui = started();
    let (mn, mx, cl) = (button("Minimize"), button("Maximize"), button("Close"));
    for r in [mn, mx, cl] {
        assert_eq!(r.width(), BUTTON_W);
        assert_eq!(r.height(), BAR_H);
    }
    assert_eq!(mn.max.x, mx.min.x);
    assert_eq!(mx.max.x, cl.min.x);
    assert_eq!(cl.max.x, 1280.0);
}

#[test]
fn clicking_minimise_maximise_close_sends_viewport_commands() {
    let mut ui = started();
    ui.click_at(button("Minimize").center());
    ui.click_at(button("Maximize").center());
    ui.click_at(button("Close").center());
    ui.settle();
    let sent = caption::take_sent();
    assert!(matches!(sent[0], ViewportCommand::Minimized(true)), "{sent:?}");
    assert!(matches!(sent[1], ViewportCommand::Maximized(true)), "{sent:?}");
    assert!(matches!(sent[2], ViewportCommand::Close), "{sent:?}");
}

#[test]
fn the_record_marks_widgets_interactive_and_empty_space_as_caption() {
    let _ui = started();
    let l = caption::last_layout();
    let hit = |p: Pos2| caption::hit_test(p * l.ppp, &l, false, 1280.0 * l.ppp, 8.0);
    // Quick-access buttons (Save, Undo, Redo) at the left.
    assert_eq!(hit(Pos2::new(20.0, 16.0)), caption::HTCLIENT);
    // The search box, centred.
    assert_eq!(hit(Pos2::new(640.0, 16.0)), caption::HTCLIENT);
    // Empty bar space and the title text.
    assert_eq!(hit(Pos2::new(250.0, 16.0)), caption::HTCAPTION);
    assert_eq!(hit(Pos2::new(1000.0, 16.0)), caption::HTCAPTION);
    // The caption buttons.
    assert_eq!(hit(button("Minimize").center()), caption::HTMINBUTTON);
    assert_eq!(hit(button("Maximize").center()), caption::HTMAXBUTTON);
    assert_eq!(hit(button("Close").center()), caption::HTCLOSE);
    // Below the bar is the page area.
    assert_eq!(hit(Pos2::new(250.0, BAR_H + 20.0)), caption::HTCLIENT);
}

#[test]
fn the_bar_stays_the_same_on_the_file_screen() {
    let mut ui = started();
    ui.app().backstage = true;
    ui.settle();
    assert_eq!(button("Close").width(), BUTTON_W);
    assert_eq!(caption::last_layout().bar_h, BAR_H);
}
