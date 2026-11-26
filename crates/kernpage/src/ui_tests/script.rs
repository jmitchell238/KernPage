//! Subscript and Superscript (spec 4S, docs/testing/UI-TEST-PLAN.md section 7c): the two Home
//! toggle buttons, their exclusivity, undo, partial selections and the keyboard shortcuts.

use super::support::*;
use eframe::egui::{Key, Modifiers};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Paragraph, Rect, Run, Script, Shape, ShapeKind};

const A: u64 = 1;
const SUB: &str = "Subscript (Ctrl+=)";
const SUP: &str = "Superscript (Ctrl+Shift+=)";

fn fresh() -> TestUi {
    let boxed = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox { paragraphs: vec![Paragraph::plain("Hello world")] },
    );
    TestUi::with_doc(letter_doc(vec![boxed, rect_shape(1.0, 6.0, 2.0, 1.5)]))
}

fn box_ui() -> TestUi {
    let mut ui = fresh();
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A]);
    ui
}

fn runs(ui: &TestUi) -> Vec<Run> {
    ui.paragraphs(A).iter().flat_map(|p| p.runs.clone()).collect()
}

fn all(ui: &TestUi, s: Script) -> bool {
    runs(ui).iter().all(|r| r.script == s)
}

#[test]
fn buttons_are_live_named_and_start_unpressed() {
    let ui = box_ui();
    for n in [SUB, SUP] {
        assert!(ui.ribbon(n).enabled, "{n}");
        assert!(!ui.ribbon(n).selected, "{n}");
    }
}

#[test]
fn buttons_are_greyed_for_a_rectangle() {
    let mut ui = box_ui();
    ui.click_at(ui.center_of(2));
    assert!(!ui.ribbon(SUP).enabled && !ui.ribbon(SUB).enabled);
}

#[test]
fn superscript_toggles_on_and_off() {
    let mut ui = box_ui();
    ui.click_ribbon(SUP);
    assert!(all(&ui, Script::Superscript));
    assert!(ui.ribbon(SUP).selected && !ui.ribbon(SUB).selected);
    ui.click_ribbon(SUP);
    assert!(all(&ui, Script::Normal));
    assert!(!ui.ribbon(SUP).selected);
}

#[test]
fn turning_one_on_turns_the_other_off() {
    let mut ui = box_ui();
    ui.click_ribbon(SUP);
    ui.click_ribbon(SUB);
    assert!(all(&ui, Script::Subscript));
    assert!(ui.ribbon(SUB).selected && !ui.ribbon(SUP).selected);
    ui.click_ribbon(SUP);
    assert!(all(&ui, Script::Superscript));
    assert!(ui.ribbon(SUP).selected && !ui.ribbon(SUB).selected);
}

#[test]
fn each_click_is_one_undo_step() {
    let mut ui = box_ui();
    ui.click_ribbon(SUP);
    ui.click_ribbon(SUB);
    ui.undo();
    assert!(all(&ui, Script::Superscript));
    ui.undo();
    assert!(all(&ui, Script::Normal));
    assert!(!ui.can_undo());
    assert!(!ui.ribbon(SUP).selected);
}

#[test]
fn a_partial_selection_in_edit_mode_splits_the_runs() {
    let mut ui = fresh();
    ui.double_click_at(ui.at_in(1.14, 1.12));
    assert!(ui.app().edit.is_some());
    ui.settle();
    ui.set_selection_range(6, 11);
    ui.click_ribbon(SUB);
    let got: Vec<(String, Script)> = runs(&ui).iter().map(|r| (r.text.clone(), r.script)).collect();
    assert_eq!(
        got,
        [("Hello ".to_owned(), Script::Normal), ("world".to_owned(), Script::Subscript)]
    );
}

#[test]
fn keyboard_shortcuts_toggle_script() {
    let mut ui = box_ui();
    let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    ui.key(ctrl_shift, Key::Equals);
    assert!(all(&ui, Script::Superscript), "Ctrl+Shift+= superscripts");
    ui.key(Modifiers::COMMAND, Key::Equals);
    assert!(all(&ui, Script::Subscript), "Ctrl+= subscripts and clears superscript");
    ui.key(Modifiers::COMMAND, Key::Equals);
    assert!(all(&ui, Script::Normal), "Ctrl+= again clears it");
    ui.key(ctrl_shift, Key::Plus);
    assert!(all(&ui, Script::Superscript), "Ctrl+Shift+Plus (layouts that report Plus)");
}
