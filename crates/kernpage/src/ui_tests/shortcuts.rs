//! Keyboard shortcuts of the editor (not typing into a text box; see `text_edit`).
//! Every test presses real key events through the whole UI.

use super::support::*;
use crate::app::{Pending, Prompt, Section, ZoomMode};
use eframe::egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Align, Shape, ShapeKind};

const A: u64 = 1;
const B: u64 = 2;
const C: u64 = 3;

const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: true,
};

fn three_boxes() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
        rect_shape(1.0, 6.0, 2.0, 1.5),
    ]))
}

fn ids(ui: &TestUi) -> Vec<u64> {
    ui.shapes().iter().map(|s| s.id).collect()
}

fn sorted(mut v: Vec<u64>) -> Vec<u64> {
    v.sort();
    v
}

type RunFlag = fn(&kernpage_core::Run) -> bool;

fn text_box(x: f64, y: f64, text: &str) -> Shape {
    Shape::new(
        kernpage_core::Rect::new(in_to_emu(x), in_to_emu(y), in_to_emu(3.0), in_to_emu(1.0)),
        ShapeKind::TextBox {
            paragraphs: vec![kernpage_core::Paragraph::plain(text)],
        },
    )
}

// ---- undo / redo ---------------------------------------------------------------------------

#[test]
fn ctrl_z_undoes_and_ctrl_y_and_ctrl_shift_z_redo() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(Modifiers::NONE, Key::ArrowRight);
    let moved = ui.rect_of(A);
    ui.key(CTRL, Key::Z);
    assert_ne!(ui.rect_of(A), moved, "Ctrl+Z undoes the nudge");
    ui.key(CTRL, Key::Y);
    assert_eq!(ui.rect_of(A), moved, "Ctrl+Y redoes it");
    ui.key(CTRL, Key::Z);
    assert_ne!(ui.rect_of(A), moved);
    ui.key(CTRL_SHIFT, Key::Z);
    assert_eq!(ui.rect_of(A), moved, "Ctrl+Shift+Z redoes it too");
}

// ---- select all / delete -----------------------------------------------------------------

#[test]
fn ctrl_a_selects_every_top_level_shape() {
    let mut ui = three_boxes();
    ui.key(CTRL, Key::A);
    assert_eq!(sorted(ui.selected()), vec![A, B, C]);
    assert!(!ui.can_undo(), "selecting is not an edit");
}

#[test]
fn ctrl_a_selects_a_group_but_not_its_members() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    ui.key(CTRL_SHIFT, Key::G);
    let group = ui.selected()[0];
    ui.click_at(ui.at_in(0.5, 10.5));
    ui.key(CTRL, Key::A);
    assert_eq!(sorted(ui.selected()), sorted(vec![group, C]));
}

#[test]
fn delete_and_backspace_remove_the_selection_as_one_undo_step() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(Modifiers::NONE, Key::Delete);
    assert_eq!(ids(&ui), vec![B, C]);
    assert!(ui.selected().is_empty());
    ui.key(CTRL, Key::Z);
    assert_eq!(ids(&ui), vec![A, B, C], "one Ctrl+Z brings it back");

    ui.click_at(ui.center_of(B));
    ui.click_mods(ui.center_of(C), SHIFT);
    ui.key(Modifiers::NONE, Key::Backspace);
    assert_eq!(ids(&ui), vec![A], "Backspace deletes a multi-selection");
    ui.key(CTRL, Key::Z);
    assert_eq!(ids(&ui), vec![A, B, C]);
}

#[test]
fn delete_with_nothing_selected_does_nothing() {
    let mut ui = three_boxes();
    ui.key(Modifiers::NONE, Key::Delete);
    assert_eq!(ids(&ui), vec![A, B, C]);
    assert!(!ui.can_undo());
}

#[test]
fn delete_and_ctrl_a_edit_text_instead_while_typing() {
    let mut ui = TestUi::with_doc(letter_doc(vec![
        text_box(1.0, 1.0, "Hello"),
        rect_shape(4.0, 3.0, 2.0, 1.5),
    ]));
    ui.double_click_at(ui.center_of(A));
    assert!(ui.app().edit.is_some());
    ui.key(Modifiers::NONE, Key::Backspace);
    ui.key(Modifiers::NONE, Key::Delete);
    assert_eq!(ids(&ui), vec![A, B], "the shape survives Backspace / Delete in the editor");
    ui.key(CTRL, Key::A);
    assert_eq!(ui.selected(), vec![A], "Ctrl+A selects text, not all shapes");
}

// ---- nudge -------------------------------------------------------------------------------

#[test]
fn arrow_keys_nudge_by_0_02_in_and_shift_arrows_by_0_1_in() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    let r = ui.rect_of(A);
    ui.key(Modifiers::NONE, Key::ArrowRight);
    assert_eq!(ui.rect_of(A).x, r.x + in_to_emu(0.02));
    ui.key(Modifiers::NONE, Key::ArrowDown);
    assert_eq!(ui.rect_of(A).y, r.y + in_to_emu(0.02));
    ui.key(Modifiers::NONE, Key::ArrowLeft);
    ui.key(Modifiers::NONE, Key::ArrowUp);
    assert_eq!(ui.rect_of(A), r, "left and up undo right and down");
    ui.key(SHIFT, Key::ArrowRight);
    assert_eq!(ui.rect_of(A).x, r.x + in_to_emu(0.1), "Shift+arrow moves 0.1 in");
    ui.key(SHIFT, Key::ArrowUp);
    assert_eq!(ui.rect_of(A).y, r.y - in_to_emu(0.1));
    assert_eq!((ui.rect_of(A).w, ui.rect_of(A).h), (r.w, r.h), "nudging never resizes");
}

#[test]
fn every_nudge_is_one_undo_step() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    let r = ui.rect_of(A);
    ui.key(Modifiers::NONE, Key::ArrowRight);
    ui.key(Modifiers::NONE, Key::ArrowRight);
    ui.key(SHIFT, Key::ArrowDown);
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.rect_of(A).y, r.y, "first Ctrl+Z reverts only the last nudge");
    assert_eq!(ui.rect_of(A).x, r.x + 2 * in_to_emu(0.02));
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.rect_of(A).x, r.x + in_to_emu(0.02));
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.rect_of(A), r);
}

#[test]
fn arrows_nudge_a_multi_selection_together() {
    let mut ui = three_boxes();
    ui.key(CTRL, Key::A);
    let before: Vec<_> = ui.shapes().iter().map(|s| s.rect).collect();
    ui.key(Modifiers::NONE, Key::ArrowLeft);
    for (s, b) in ui.shapes().iter().zip(&before) {
        assert_eq!(s.rect.x, b.x - in_to_emu(0.02));
    }
    ui.key(CTRL, Key::Z);
    let after: Vec<_> = ui.shapes().iter().map(|s| s.rect).collect();
    assert_eq!(after, before, "one undo step for all");
}

#[test]
fn arrows_with_nothing_selected_do_nothing() {
    let mut ui = three_boxes();
    ui.key(Modifiers::NONE, Key::ArrowRight);
    assert!(!ui.can_undo());
}

// ---- clipboard (shapes) ------------------------------------------------------------------

#[test]
fn ctrl_c_then_ctrl_v_pastes_a_copy_offset_by_a_quarter_inch_and_selects_it() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    let r = ui.rect_of(A);
    // A real window turns Ctrl+C into Event::Copy.
    ui.event(Event::Copy);
    assert_eq!(ids(&ui), vec![A, B, C], "copy changes nothing");
    assert!(!ui.can_undo());
    ui.event(Event::Paste(String::new()));
    assert_eq!(ids(&ui).len(), 4);
    let pasted = ui.shapes()[3].clone();
    assert_eq!(pasted.rect.x, r.x + in_to_emu(0.25));
    assert_eq!(pasted.rect.y, r.y + in_to_emu(0.25));
    assert_eq!(ui.selected(), vec![pasted.id], "the paste is selected");
    ui.event(Event::Paste(String::new()));
    assert_eq!(ui.shapes()[4].rect.x, r.x + in_to_emu(0.5), "each further paste steps again");
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.shapes().len(), 4, "a paste is one undo step");
}

#[test]
fn ctrl_x_cuts_and_ctrl_v_pastes_it_back() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(B));
    ui.event(Event::Cut);
    assert_eq!(ids(&ui), vec![A, C]);
    ui.event(Event::Paste(String::new()));
    assert_eq!(ui.shapes().len(), 3);
    assert!(ui
        .shapes()
        .iter()
        .any(|s| s.rect.w == in_to_emu(2.0) && s.rect.y > in_to_emu(3.0)));
}

#[test]
fn paste_with_an_empty_clipboard_does_nothing() {
    let mut ui = three_boxes();
    ui.event(Event::Paste(String::new()));
    assert_eq!(ids(&ui), vec![A, B, C]);
    assert!(!ui.can_undo());
}

// ---- group / ungroup ---------------------------------------------------------------------

#[test]
fn ctrl_shift_g_groups_and_ctrl_shift_h_ungroups() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    ui.key(CTRL_SHIFT, Key::G);
    assert_eq!(ui.shapes().len(), 4);
    let g = ui.selected();
    assert_eq!(g.len(), 1, "the new group is selected");
    assert!(matches!(ui.shape(g[0]).kind, ShapeKind::Group));
    assert_eq!(ui.shape(A).group, Some(g[0]));
    ui.key(CTRL_SHIFT, Key::H);
    assert_eq!(ui.shapes().len(), 3);
    assert_eq!(sorted(ui.selected()), vec![A, B], "the former members are selected");
    assert_eq!(ui.shape(A).group, None);
}

#[test]
fn ctrl_shift_g_needs_two_objects() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(CTRL_SHIFT, Key::G);
    assert_eq!(ui.shapes().len(), 3);
    assert!(!ui.can_undo());
}

// ---- formatting shortcuts (whole box; selection cases are in text_edit) -------------------------

#[test]
fn ctrl_b_i_u_toggle_the_whole_selected_text_box() {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box(1.0, 1.0, "Hello")]));
    ui.click_at(ui.center_of(A));
    let getters: [(Key, RunFlag); 3] = [
        (Key::B, |r| r.bold),
        (Key::I, |r| r.italic),
        (Key::U, |r| r.underline),
    ];
    for (key, get) in getters {
        ui.key(CTRL, key);
        assert!(ui.paragraphs(A)[0].runs.iter().all(get), "{key:?} on");
        ui.key(CTRL, key);
        assert!(ui.paragraphs(A)[0].runs.iter().all(|r| !get(r)), "{key:?} toggles off");
    }
}

#[test]
fn format_shortcuts_do_nothing_for_a_rectangle() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(CTRL, Key::B);
    assert!(!ui.can_undo());
}

#[test]
fn ctrl_l_e_r_j_set_the_alignment_of_the_selected_text_box() {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box(1.0, 1.0, "Hello")]));
    ui.click_at(ui.center_of(A));
    for (key, want) in [
        (Key::E, Align::Center),
        (Key::R, Align::Right),
        (Key::J, Align::Justify),
        (Key::L, Align::Left),
    ] {
        ui.key(CTRL, key);
        assert!(ui.paragraphs(A).iter().all(|p| p.align == want), "{key:?}");
    }
}

// ---- file / page / view shortcuts ------------------------------------------------------------

#[test]
fn ctrl_n_opens_the_new_publication_page_of_the_file_view() {
    let mut ui = three_boxes();
    ui.key(CTRL, Key::N);
    assert!(ui.h.state().backstage);
    assert!(ui.h.state().section == Section::New);
    assert_eq!(ui.shapes().len(), 3, "the open publication is untouched");
}

#[test]
fn ctrl_n_also_works_from_the_start_screen() {
    let mut ui = TestUi::start();
    ui.key(CTRL, Key::N);
    assert!(ui.h.state().section == Section::New);
}

#[test]
fn ctrl_o_with_unsaved_changes_asks_first() {
    // With nothing unsaved Ctrl+O goes straight to the native file dialog (not testable);
    // with unsaved changes the Save / Don't Save / Cancel question comes first.
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(Modifiers::NONE, Key::ArrowRight);
    ui.key(CTRL, Key::O);
    assert!(matches!(
        ui.h.state().prompt,
        Some(Prompt::Unsaved { then: Pending::Open })
    ));
    ui.h.get_by_label("  Cancel  ").click();
    ui.settle();
    assert!(ui.h.state().prompt.is_none());
    assert!(ui.h.state().editor.is_some(), "Cancel keeps the publication");
}

#[test]
fn ctrl_s_saves_a_publication_that_has_a_file_without_a_dialog() {
    let dir = std::env::temp_dir().join(format!("kernpage-uitest-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("two-boxes-copy.pub");
    std::fs::copy(sample("two-boxes.pub"), &copy).unwrap();
    let before = std::fs::read(&copy).unwrap();

    let mut ui = TestUi::start();
    ui.app().open_path(&copy);
    ui.settle();
    assert!(ui.app().error.is_none());
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(SHIFT, Key::ArrowRight);
    assert!(ui.app().is_dirty());
    ui.key(CTRL, Key::S);
    assert!(ui.app().prompt.is_none() && ui.app().error.is_none());
    assert!(!ui.app().is_dirty(), "saved");
    assert_ne!(std::fs::read(&copy).unwrap(), before, "the file on disk changed");
    // The saved file opens again with the box where we put it.
    let moved = ui.rect_of(id);
    let mut again = TestUi::start();
    again.app().open_path(&copy);
    again.settle();
    let off = (again.rect_of(id).x - moved.x).abs();
    assert!(off < in_to_emu(0.01), "reopened box is {off} EMU off");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ctrl_p_opens_the_print_page_and_needs_a_publication() {
    let mut ui = TestUi::start();
    ui.key(CTRL, Key::P);
    assert!(ui.h.state().section != Section::Print, "no publication, no Print page");
    let mut ui = three_boxes();
    ui.key(CTRL, Key::P);
    assert!(ui.h.state().backstage && ui.h.state().section == Section::Print);
}

#[test]
fn ctrl_shift_n_inserts_a_blank_page_after_the_current_one_and_shows_it() {
    let mut ui = three_boxes();
    assert_eq!(ui.doc().pages.len(), 1);
    ui.key(CTRL_SHIFT, Key::N);
    assert_eq!(ui.doc().pages.len(), 2);
    assert_eq!(ui.h.state().page, 1, "the new page is shown");
    assert!(ui.shapes().is_empty(), "and it is blank");
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.doc().pages.len(), 1, "undoable");
}

#[test]
fn f9_toggles_between_100_percent_and_whole_page() {
    let mut ui = three_boxes();
    assert!(ui.h.state().zoom_mode == ZoomMode::WholePage);
    ui.key(Modifiers::NONE, Key::F9);
    assert!(ui.h.state().zoom_mode == ZoomMode::Custom);
    assert!((ui.h.state().zoom - 1.0).abs() < 1e-6);
    ui.key(Modifiers::NONE, Key::F9);
    assert!(ui.h.state().zoom_mode == ZoomMode::WholePage);
}

#[test]
fn page_up_and_page_down_move_between_pages_and_stop_at_the_ends() {
    let mut ui = TestUi::with_sample("two-pages.pub");
    assert_eq!(ui.h.state().page, 0);
    ui.key(Modifiers::NONE, Key::PageUp);
    assert_eq!(ui.h.state().page, 0, "PageUp on the first page stays");
    ui.key(Modifiers::NONE, Key::PageDown);
    assert_eq!(ui.h.state().page, 1);
    ui.key(Modifiers::NONE, Key::PageDown);
    assert_eq!(ui.h.state().page, 1, "PageDown on the last page stays");
    ui.key(Modifiers::NONE, Key::PageUp);
    assert_eq!(ui.h.state().page, 0);
}

#[test]
fn changing_page_clears_the_selection() {
    let mut ui = TestUi::with_sample("two-boxes.pub");
    ui.key(CTRL_SHIFT, Key::N);
    ui.key(Modifiers::NONE, Key::PageUp);
    ui.key(CTRL, Key::A);
    assert!(!ui.selected().is_empty());
    ui.key(Modifiers::NONE, Key::PageDown);
    assert!(ui.selected().is_empty());
}

// ---- modal prompts block shortcuts ----------------------------------------------------------

#[test]
fn shortcuts_do_nothing_behind_the_unsaved_prompt() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(Modifiers::NONE, Key::ArrowRight);
    ui.key(CTRL, Key::O);
    assert!(ui.h.state().prompt.is_some());
    let r = ui.rect_of(A);
    ui.key(Modifiers::NONE, Key::ArrowRight);
    ui.key(Modifiers::NONE, Key::Delete);
    ui.key(CTRL, Key::A);
    ui.key(CTRL_SHIFT, Key::N);
    ui.key(Modifiers::NONE, Key::F9);
    assert_eq!(ui.rect_of(A), r);
    assert_eq!(ids(&ui), vec![A, B, C]);
    assert_eq!(ui.selected(), vec![A]);
    assert_eq!(ui.doc().pages.len(), 1);
    assert!(ui.h.state().zoom_mode == ZoomMode::WholePage);
    assert!(ui.h.state().prompt.is_some(), "the question is still open");
}

#[test]
fn shortcuts_do_nothing_behind_an_error_dialog() {
    let mut ui = three_boxes();
    ui.click_at(ui.center_of(A));
    ui.app().error = Some("Something went wrong".into());
    ui.settle();
    ui.key(Modifiers::NONE, Key::Delete);
    ui.key(CTRL, Key::A);
    ui.key(CTRL_SHIFT, Key::N);
    assert_eq!(ids(&ui), vec![A, B, C]);
    assert_eq!(ui.doc().pages.len(), 1);
}
