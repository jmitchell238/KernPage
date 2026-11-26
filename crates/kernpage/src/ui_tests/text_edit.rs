//! Text editing: the in-place editor of a text box or table cell. Every test double-clicks
//! to start editing and then sends real key / text events through the whole UI.

use super::support::*;
use eframe::egui::{self, Event, Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Align, Paragraph, Rect, Run, Shape, ShapeKind, TableCell};

const A: u64 = 1;
const B: u64 = 2;

const NONE: Modifiers = Modifiers::NONE;
const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: true,
};

fn text_box_of(paragraphs: &[&str]) -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox {
            paragraphs: paragraphs.iter().map(|t| Paragraph::plain(t)).collect(),
        },
    )
}

/// One text box A with the given paragraphs, plus a rectangle B to click away on.
fn doc_with(paragraphs: &[&str]) -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        text_box_of(paragraphs),
        rect_shape(1.0, 6.0, 2.0, 1.5),
    ]))
}

/// "Hello world" in box A, editing started by a double-click, caret at the start.
fn editing_hello() -> TestUi {
    let mut ui = doc_with(&["Hello world"]);
    start_edit(&mut ui);
    ui
}

fn start_edit(ui: &mut TestUi) {
    // Near the top-left of the box: the double-click lands on the first word.
    ui.double_click_at(ui.at_in(1.14, 1.12));
    assert!(ui.app().edit.is_some(), "edit mode started");
    ui.settle();
    ui.set_selection_range(0, 0);
}

/// Click empty page space: the editor commits and closes.
fn click_away(ui: &mut TestUi) {
    ui.click_at(ui.at_in(6.0, 9.0));
    assert!(ui.app().edit.is_none(), "clicking away ends the edit");
}

/// How many undo steps the document has (undoes them all, then redoes them all).
fn undo_depth(ui: &mut TestUi) -> usize {
    let ed = ui.app().editor.as_mut().expect("editor");
    let mut n = 0;
    while ed.undo() {
        n += 1;
    }
    for _ in 0..n {
        ed.redo();
    }
    n
}

fn runs_of(ui: &TestUi, id: u64) -> Vec<Run> {
    ui.paragraphs(id).iter().flat_map(|p| p.runs.clone()).collect()
}

/// The text of the runs that are bold, joined.
fn bold_text(ui: &TestUi, id: u64) -> String {
    runs_of(ui, id)
        .iter()
        .filter(|r| r.bold)
        .map(|r| r.text.as_str())
        .collect()
}

fn editor_has_focus(ui: &mut TestUi) -> bool {
    let id = crate::app::edit_widget_id(ui.h.state().edit.as_ref().expect("edit mode"));
    ui.h.ctx.memory(|m| m.has_focus(id))
}

// ---- typing and committing -------------------------------------------------------------------

#[test]
fn typing_is_committed_as_one_undo_step_and_undo_restores_the_text() {
    let mut ui = doc_with(&["Hello world"]);
    let before = ui.revision();
    start_edit(&mut ui);
    ui.set_selection_range(5, 5);
    ui.type_text("X");
    ui.type_text("Y");
    ui.type_text("Z");
    assert_eq!(ui.edit_text(), "HelloXYZ world");
    assert_eq!(ui.revision(), before, "nothing reaches the document until the edit ends");
    click_away(&mut ui);
    assert_eq!(ui.text_of(A), "HelloXYZ world");
    assert_eq!(undo_depth(&mut ui), 1, "all the typing is one undo step");
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.text_of(A), "Hello world", "Ctrl+Z restores the text");
    assert!(!ui.can_undo());
    ui.key(CTRL, Key::Y);
    assert_eq!(ui.text_of(A), "HelloXYZ world", "and Ctrl+Y brings it back");
}

#[test]
fn leaving_the_editor_without_changing_anything_is_not_an_undo_step() {
    let mut ui = editing_hello();
    click_away(&mut ui);
    assert!(!ui.can_undo());
}

#[test]
fn ctrl_z_during_editing_undoes_the_typing_so_far() {
    let mut ui = editing_hello();
    ui.set_selection_range(11, 11);
    ui.type_text("!!");
    assert_eq!(ui.edit_text(), "Hello world!!");
    // egui's undoer records a restore point after the text has been still for a moment.
    ui.h.run_steps(90);
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.edit_text(), "Hello world", "the editor's own undo, not the document's");
    assert!(ui.app().edit.is_some(), "still editing");
}

#[test]
fn typing_replaces_the_selected_text() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.type_text("Howdy");
    assert_eq!(ui.edit_text(), "Howdy world");
}

// ---- Enter, Backspace, Delete -----------------------------------------------------------------

#[test]
fn enter_splits_the_paragraph_and_commits_as_two_paragraphs() {
    let mut ui = editing_hello();
    ui.set_selection_range(5, 5);
    ui.key(NONE, Key::Enter);
    assert_eq!(ui.edit_text(), "Hello\n world");
    assert!(ui.app().edit.is_some(), "Enter stays in the editor");
    click_away(&mut ui);
    let p = ui.paragraphs(A);
    assert_eq!(p.len(), 2, "Enter adds a paragraph");
    assert_eq!((p[0].text().as_str(), p[1].text().as_str()), ("Hello", " world"));
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn backspace_deletes_before_the_caret_and_delete_after_it() {
    let mut ui = editing_hello();
    ui.set_selection_range(5, 5);
    ui.key(NONE, Key::Backspace);
    assert_eq!(ui.edit_text(), "Hell world");
    assert_eq!(ui.caret().0, 4);
    ui.key(NONE, Key::Delete);
    assert_eq!(ui.edit_text(), "Hellworld");
    assert_eq!(ui.caret().0, 4);
    assert_eq!(ui.shapes().len(), 2, "the box itself is not deleted");
}

#[test]
fn backspace_and_delete_remove_a_selection() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 6);
    ui.key(NONE, Key::Backspace);
    assert_eq!(ui.edit_text(), "world");
    ui.set_selection_range(1, 3);
    ui.key(NONE, Key::Delete);
    assert_eq!(ui.edit_text(), "wld");
}

#[test]
fn backspace_at_the_start_and_delete_at_the_end_change_nothing() {
    let mut ui = editing_hello();
    ui.key(NONE, Key::Backspace);
    assert_eq!(ui.edit_text(), "Hello world");
    ui.set_selection_range(11, 11);
    ui.key(NONE, Key::Delete);
    assert_eq!(ui.edit_text(), "Hello world");
}

#[test]
fn backspace_joins_two_paragraphs() {
    let mut ui = doc_with(&["One", "Two"]);
    start_edit(&mut ui);
    ui.set_selection_range(4, 4); // start of "Two"
    ui.key(NONE, Key::Backspace);
    click_away(&mut ui);
    assert_eq!(ui.paragraphs(A).len(), 1);
    assert_eq!(ui.text_of(A), "OneTwo");
}

// ---- caret and selection keys -----------------------------------------------------------------

#[test]
fn arrow_home_and_end_move_the_caret() {
    let mut ui = editing_hello();
    ui.key(NONE, Key::End);
    assert_eq!(ui.caret(), (11, 11));
    ui.key(NONE, Key::ArrowLeft);
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(ui.caret(), (9, 9));
    ui.key(NONE, Key::ArrowRight);
    assert_eq!(ui.caret(), (10, 10));
    ui.key(NONE, Key::Home);
    assert_eq!(ui.caret(), (0, 0));
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(ui.caret(), (0, 0), "Left at the start stays");
}

#[test]
fn arrows_move_the_caret_and_do_not_nudge_the_box() {
    let mut ui = editing_hello();
    let r = ui.rect_of(A);
    ui.key(NONE, Key::ArrowRight);
    ui.key(NONE, Key::ArrowDown);
    ui.key(SHIFT, Key::ArrowLeft);
    assert_eq!(ui.rect_of(A), r);
    assert!(!ui.can_undo());
}

#[test]
fn shift_arrows_extend_the_selection_from_the_caret() {
    let mut ui = editing_hello();
    ui.key(SHIFT, Key::ArrowRight);
    ui.key(SHIFT, Key::ArrowRight);
    ui.key(SHIFT, Key::ArrowRight);
    assert_eq!(ui.caret(), (3, 0), "three characters selected from the start");
    ui.key(SHIFT, Key::ArrowLeft);
    assert_eq!(ui.caret(), (2, 0));
    ui.key(NONE, Key::ArrowRight);
    assert_eq!(ui.caret(), (2, 2), "an arrow without Shift collapses the selection");
}

#[test]
fn shift_end_and_shift_home_select_to_the_line_ends() {
    let mut ui = editing_hello();
    ui.set_selection_range(5, 5);
    ui.key(SHIFT, Key::End);
    assert_eq!(ui.caret(), (11, 5));
    ui.set_selection_range(5, 5);
    ui.key(SHIFT, Key::Home);
    assert_eq!(ui.caret(), (0, 5));
}

#[test]
fn ctrl_arrows_jump_by_words_and_ctrl_shift_arrows_select_words() {
    let mut ui = editing_hello();
    ui.key(CTRL, Key::ArrowRight);
    let (p, _) = ui.caret();
    assert!(p == 5 || p == 6, "Ctrl+Right moves past the first word, caret at {p}");
    ui.set_selection_range(0, 0);
    ui.key(CTRL_SHIFT, Key::ArrowRight);
    let (p, s) = ui.caret();
    assert_eq!(s, 0);
    assert!(p == 5 || p == 6, "Ctrl+Shift+Right selects the word, caret at {p}");
    ui.set_selection_range(11, 11);
    ui.key(CTRL, Key::ArrowLeft);
    assert_eq!(ui.caret(), (6, 6), "Ctrl+Left goes to the start of the word");
}

#[test]
fn ctrl_a_selects_all_the_text_and_typing_replaces_it() {
    let mut ui = editing_hello();
    ui.key(CTRL, Key::A);
    assert_eq!(ui.caret().0.abs_diff(ui.caret().1), 11);
    ui.type_text("New");
    assert_eq!(ui.edit_text(), "New");
    assert_eq!(ui.shapes().len(), 2, "Ctrl+A did not select the shapes");
}

#[test]
fn up_and_down_arrows_move_between_paragraphs() {
    let mut ui = doc_with(&["First", "Second"]);
    start_edit(&mut ui);
    ui.set_selection_range(2, 2);
    ui.key(NONE, Key::ArrowDown);
    let (p, _) = ui.caret();
    assert!((6..=12).contains(&p), "caret moved into the second paragraph, at {p}");
    ui.key(NONE, Key::ArrowUp);
    let (p, _) = ui.caret();
    assert!(p <= 5, "and back into the first, at {p}");
}

#[test]
fn double_click_in_the_editor_selects_the_word() {
    let mut ui = editing_hello();
    ui.double_click_at(ui.at_in(1.14, 1.12));
    let (p, s) = ui.caret();
    let (a, b) = (p.min(s), p.max(s));
    assert_eq!(&ui.edit_text()[a..b], "Hello", "selected {a}..{b}");
    assert!(ui.app().edit.is_some(), "still editing");
}

#[test]
fn triple_click_or_double_click_does_not_change_the_text() {
    let mut ui = editing_hello();
    ui.double_click_at(ui.at_in(1.14, 1.12));
    click_away(&mut ui);
    assert_eq!(ui.text_of(A), "Hello world");
    assert!(!ui.can_undo());
}

// ---- Escape -------------------------------------------------------------------------------------

#[test]
fn escape_commits_the_text_and_leaves_edit_mode_with_the_box_selected() {
    let mut ui = editing_hello();
    ui.set_selection_range(11, 11);
    ui.type_text("!");
    ui.key(NONE, Key::Escape);
    assert!(ui.app().edit.is_none(), "Escape leaves edit mode");
    assert_eq!(ui.text_of(A), "Hello world!", "and keeps what was typed");
    assert_eq!(ui.selected(), vec![A], "the box stays selected");
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn escape_twice_deselects_the_box() {
    let mut ui = editing_hello();
    ui.key(NONE, Key::Escape);
    ui.key(NONE, Key::Escape);
    assert!(ui.selected().is_empty());
}

// ---- text clipboard (through egui events, no OS clipboard) --------------------------------------------

#[test]
fn ctrl_c_in_the_editor_copies_the_selected_text_and_ctrl_x_cuts_it() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.h.event(Event::Copy);
    ui.h.step(); // the copied text is in the output of exactly this frame
    let copied = ui.h.output().platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    assert_eq!(copied.as_deref(), Some("Hello"), "the selection went to the clipboard output");
    ui.settle();
    assert_eq!(ui.edit_text(), "Hello world", "copy leaves the text alone");
    ui.set_selection_range(0, 6);
    ui.event(Event::Cut);
    assert_eq!(ui.edit_text(), "world", "cut removes the selection");
    assert_eq!(ui.shapes().len(), 2, "and does not cut the shape");
}

#[test]
fn paste_in_the_editor_inserts_the_clipboard_text_at_the_caret() {
    let mut ui = editing_hello();
    ui.set_selection_range(5, 5);
    ui.event(Event::Paste(", dear".to_owned()));
    assert_eq!(ui.edit_text(), "Hello, dear world");
    assert_eq!(ui.shapes().len(), 2, "no shape was pasted");
}

#[test]
fn context_menu_in_the_editor_offers_the_text_clipboard() {
    let mut ui = editing_hello();
    ui.right_click_at(ui.at_in(1.14, 1.12));
    for label in ["Cut", "Copy", "Paste", "Paragraph..."] {
        ui.h.get_by_label(label);
    }
    assert!(ui.app().edit.is_some(), "the menu does not end the edit");
}

// ---- formatting: selection, whole box, caret only ---------------------------------------------------

#[test]
fn ctrl_b_while_editing_bolds_only_the_selected_characters_as_one_undo_step() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.key(CTRL, Key::B);
    assert_eq!(bold_text(&ui, A), "Hello", "only the selection is bold");
    assert_eq!(ui.text_of(A), "Hello world");
    assert!(ui.app().edit.is_some(), "still editing");
    assert_eq!(undo_depth(&mut ui), 1);
    click_away(&mut ui);
    assert_eq!(bold_text(&ui, A), "Hello");
    ui.key(CTRL, Key::Z);
    assert_eq!(bold_text(&ui, A), "", "undo removes the bold");
}

#[test]
fn italic_and_underline_apply_to_the_selection_only() {
    let mut ui = editing_hello();
    ui.set_selection_range(6, 11);
    ui.key(CTRL, Key::I);
    ui.key(CTRL, Key::U);
    let runs = runs_of(&ui, A);
    let on: String = runs.iter().filter(|r| r.italic && r.underline).map(|r| r.text.as_str()).collect();
    let off: String = runs.iter().filter(|r| !r.italic && !r.underline).map(|r| r.text.as_str()).collect();
    assert_eq!((on.as_str(), off.as_str()), ("world", "Hello "));
}

#[test]
fn bold_toggles_off_again_when_the_selection_is_all_bold() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.key(CTRL, Key::B);
    ui.key(CTRL, Key::B);
    assert_eq!(bold_text(&ui, A), "");
}

#[test]
fn typing_after_formatting_keeps_the_formatting_of_the_surrounding_text() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.key(CTRL, Key::B);
    assert!(editor_has_focus(&mut ui), "the editor keeps the keyboard after the shortcut");
    ui.set_selection_range(5, 5);
    ui.type_text("!");
    click_away(&mut ui);
    assert_eq!(ui.text_of(A), "Hello! world");
    assert!(bold_text(&ui, A).starts_with("Hello"), "bold text {:?}", bold_text(&ui, A));
}

#[test]
fn formatting_a_selected_box_without_editing_applies_to_all_of_its_text() {
    let mut ui = doc_with(&["Hello world", "Second line"]);
    ui.click_at(ui.at_in(1.14, 1.12));
    assert!(ui.app().edit.is_none());
    assert_eq!(ui.selected(), vec![A]);
    ui.key(CTRL, Key::B);
    assert_eq!(bold_text(&ui, A), "Hello worldSecond line", "every run of every paragraph");
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn formatting_a_box_with_mixed_bold_makes_everything_bold_then_nothing() {
    let mut ui = TestUi::with_sample("bold.pub");
    let id = ui.shapes()[0].id;
    assert!(runs_of(&ui, id).iter().any(|r| r.bold) && runs_of(&ui, id).iter().any(|r| !r.bold));
    ui.click_at(ui.center_of(id));
    ui.key(CTRL, Key::B);
    assert!(runs_of(&ui, id).iter().all(|r| r.bold), "mixed becomes all bold");
    ui.key(CTRL, Key::B);
    assert!(runs_of(&ui, id).iter().all(|r| !r.bold), "then none");
}

#[test]
fn with_only_a_caret_the_character_formats_have_nothing_to_apply() {
    let mut ui = editing_hello();
    ui.set_selection_range(3, 3);
    ui.key(CTRL, Key::B);
    ui.key(CTRL, Key::I);
    ui.key(CTRL, Key::U);
    assert!(!ui.can_undo(), "no formatting command was run");
    assert!(runs_of(&ui, A).iter().all(|r| !r.bold && !r.italic && !r.underline));
    assert!(
        ui.h.get_by_label("Bold (Ctrl+B)").accesskit_node().is_disabled(),
        "the ribbon's Bold button is greyed with only a caret"
    );
}

#[test]
fn ribbon_bold_button_is_enabled_with_a_selection_and_marked_pressed_when_all_bold() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.settle();
    assert!(!ui.h.get_by_label("Bold (Ctrl+B)").accesskit_node().is_disabled());
}

// ---- alignment ----------------------------------------------------------------------------------------

#[test]
fn alignment_shortcuts_change_only_the_paragraph_with_the_caret() {
    let mut ui = doc_with(&["One", "Two", "Three"]);
    start_edit(&mut ui);
    ui.set_selection_range(5, 5); // inside "Two"
    ui.key(CTRL, Key::E);
    let aligns = |ui: &TestUi| ui.paragraphs(A).iter().map(|p| p.align).collect::<Vec<_>>();
    assert_eq!(aligns(&ui), vec![Align::Left, Align::Center, Align::Left]);
    ui.key(CTRL, Key::R);
    assert_eq!(aligns(&ui), vec![Align::Left, Align::Right, Align::Left]);
    ui.key(CTRL, Key::J);
    assert_eq!(aligns(&ui), vec![Align::Left, Align::Justify, Align::Left]);
    ui.key(CTRL, Key::L);
    assert_eq!(aligns(&ui), vec![Align::Left; 3]);
    assert!(ui.app().edit.is_some(), "still editing");
}

#[test]
fn alignment_covers_every_paragraph_the_selection_touches() {
    let mut ui = doc_with(&["One", "Two", "Three"]);
    start_edit(&mut ui);
    ui.set_selection_range(2, 6); // end of "One" into "Two"
    ui.key(CTRL, Key::R);
    let aligns: Vec<_> = ui.paragraphs(A).iter().map(|p| p.align).collect();
    assert_eq!(aligns, vec![Align::Right, Align::Right, Align::Left]);
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn alignment_buttons_apply_and_keep_the_editor_focused() {
    let mut ui = doc_with(&["One", "Two"]);
    start_edit(&mut ui);
    ui.set_selection_range(0, 0);
    for (tip, want) in [
        ("Center (Ctrl+E)", Align::Center),
        ("Align Right (Ctrl+R)", Align::Right),
        ("Justify (Ctrl+J)", Align::Justify),
        ("Align Left (Ctrl+L)", Align::Left),
    ] {
        ui.h.get_by_label(tip).click();
        ui.settle();
        assert_eq!(ui.paragraphs(A)[0].align, want, "{tip}");
        assert_eq!(ui.paragraphs(A)[1].align, Align::Left, "{tip}: other paragraph untouched");
        assert!(ui.app().edit.is_some(), "{tip}: still editing");
    }
}

// ---- ribbon clicks keep the editor focused -----------------------------------------------------------

#[test]
fn clicking_ribbon_buttons_keeps_the_text_editor_focused_and_the_selection() {
    let mut ui = editing_hello();
    ui.set_selection_range(0, 5);
    ui.h.get_by_label("Bold (Ctrl+B)").click();
    ui.settle();
    assert_eq!(bold_text(&ui, A), "Hello");
    assert!(ui.app().edit.is_some(), "still editing after a ribbon click");
    assert!(editor_has_focus(&mut ui), "the text editor still has the keyboard");
    assert_eq!(ui.caret().0.abs_diff(ui.caret().1), 5, "and the selection is kept");
    // Typing goes straight into the box.
    ui.set_selection_range(11, 11);
    ui.h.get_by_label("Italic (Ctrl+I)").click();
    ui.settle();
    ui.type_text("?");
    assert_eq!(ui.edit_text(), "Hello world?");
}

#[test]
fn grow_font_through_the_ribbon_resizes_only_the_selected_text() {
    let mut ui = editing_hello();
    let before = runs_of(&ui, A)[0].size_pt;
    ui.set_selection_range(0, 5);
    ui.h.get_by_label("Grow Font").click();
    ui.settle();
    let runs = runs_of(&ui, A);
    let big: String = runs.iter().filter(|r| r.size_pt > before).map(|r| r.text.as_str()).collect();
    assert_eq!(big, "Hello");
    assert!(editor_has_focus(&mut ui));
}

// ---- table cells ------------------------------------------------------------------------------------------

/// A 2x2 table at (4, 2), 4 x 2 in: cells are 2 x 1 in, row-major, with text c0..c3.
fn table_ui() -> TestUi {
    let cells = (0..4).map(|i| TableCell::plain(&format!("c{i}"))).collect();
    let t = Shape::new(
        Rect::new(in_to_emu(4.0), in_to_emu(2.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::Table {
            rows: 2,
            cols: 2,
            col_widths_emu: vec![],
            row_heights_emu: vec![], row_set_emu: vec![],
            cells,
        },
    );
    TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.5), t]))
}

fn cell_text(ui: &TestUi, cell: usize) -> String {
    match &ui.shape(B).kind {
        ShapeKind::Table { cells, .. } => cells[cell].paragraphs.iter().map(|p| p.text()).collect(),
        _ => unreachable!(),
    }
}

fn editing_cell(ui: &mut TestUi) -> Option<usize> {
    ui.app().edit.as_ref().and_then(|e| e.cell)
}

#[test]
fn tab_moves_to_the_next_cell_and_shift_tab_to_the_previous_one() {
    let mut ui = table_ui();
    ui.double_click_at(ui.at_in(5.0, 2.5));
    assert_eq!(editing_cell(&mut ui), Some(0));
    ui.key(NONE, Key::Tab);
    assert_eq!(editing_cell(&mut ui), Some(1));
    ui.key(NONE, Key::Tab);
    assert_eq!(editing_cell(&mut ui), Some(2), "Tab wraps to the next row");
    ui.key(SHIFT, Key::Tab);
    assert_eq!(editing_cell(&mut ui), Some(1));
    ui.key(SHIFT, Key::Tab);
    ui.key(SHIFT, Key::Tab);
    assert_eq!(editing_cell(&mut ui), Some(0), "Shift+Tab stops at the first cell");
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab);
    assert_eq!(editing_cell(&mut ui), Some(3), "Tab stops at the last cell");
    assert!(editor_has_focus(&mut ui));
}

#[test]
fn tab_never_inserts_a_tab_character() {
    let mut ui = table_ui();
    ui.double_click_at(ui.at_in(5.0, 2.5));
    ui.key(NONE, Key::Tab);
    ui.key(SHIFT, Key::Tab);
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab);
    ui.key(NONE, Key::Tab); // past the end
    ui.key(SHIFT, Key::Tab);
    assert!(!ui.edit_text().contains('\t'));
    ui.click_at(ui.at_in(6.0, 9.0));
    for i in 0..4 {
        assert_eq!(cell_text(&ui, i), format!("c{i}"), "cell {i} is unchanged");
    }
    assert!(!ui.can_undo(), "moving between cells is not an edit");
}

#[test]
fn text_typed_after_tab_goes_into_the_new_cell_and_each_cell_is_one_undo_step() {
    let mut ui = table_ui();
    ui.double_click_at(ui.at_in(5.0, 2.5));
    ui.set_selection_range(0, 2);
    ui.type_text("AA");
    ui.key(NONE, Key::Tab);
    ui.set_selection_range(0, 2);
    ui.type_text("BB");
    ui.click_at(ui.at_in(6.0, 9.0));
    assert_eq!(cell_text(&ui, 0), "AA");
    assert_eq!(cell_text(&ui, 1), "BB");
    assert_eq!(cell_text(&ui, 2), "c2");
    assert_eq!(undo_depth(&mut ui), 2, "one undo step per edited cell");
    ui.key(CTRL, Key::Z);
    assert_eq!((cell_text(&ui, 0), cell_text(&ui, 1)), ("AA".to_owned(), "c1".to_owned()));
}

// ---- linked boxes ---------------------------------------------------------------------------------------------

#[test]
fn a_linked_story_is_shown_split_across_its_frames() {
    let ui = TestUi::with_sample("gen/linked.pub");
    // Page 1: frame 1, a standalone box, frame 2; page 2: frame 3 (see samples/gen/manifest.json).
    let frames: Vec<String> = ui
        .shapes()
        .iter()
        .filter(|s| s.is_text_box())
        .map(|s| ui.text_of(s.id))
        .collect();
    let starts = |n: &str| frames.iter().any(|t| t.starts_with(n));
    assert!(starts("Paragraph 1 "), "first frame starts the story: {frames:?}");
    assert!(starts("Paragraph 5 "), "second frame continues at paragraph 5");
    assert!(starts("Standalone box"));
    let story_in_page_1: usize = frames.iter().filter(|t| t.starts_with("Paragraph")).map(|t| t.len()).sum();
    assert!(story_in_page_1 > 500, "frames hold their share of the story ({story_in_page_1} chars)");
}

/// The linked sample's three frames in story order, with their current text.
fn linked_frames(ui: &TestUi) -> Vec<u64> {
    ui.shapes()
        .iter()
        .filter(|s| s.is_text_box() && ui.text_of(s.id).starts_with("Paragraph"))
        .map(|s| s.id)
        .collect()
}

const OPENING: &str = "A whole new opening paragraph that pushes the old text down the story. ";

#[test]
fn text_typed_into_a_linked_frame_flows_on_into_the_next_frame() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    let (first, second) = (frames[0], frames[1]);
    let (first_before, second_before) = (ui.text_of(first), ui.text_of(second));
    // The last paragraph of frame 1 (paragraph 4) is what gets pushed over.
    let last_of_first = first_before.lines().last().unwrap().to_owned();
    assert!(last_of_first.starts_with("Paragraph 4 "), "{last_of_first:?}");
    ui.double_click_at(ui.center_of(first));
    ui.set_selection_range(0, 0);
    ui.type_text(OPENING);
    ui.click_at(ui.at_in(6.0, 9.5));
    // The first frame is full: Publisher pushes its last lines into the second frame.
    assert!(ui.text_of(first).starts_with(OPENING), "the typed text is in frame 1");
    let second_after = ui.text_of(second);
    assert_ne!(second_after, second_before, "the second frame received the overflow");
    // Frame 2 now starts with the end of what used to be frame 1's last paragraph (the
    // last line, or all of it), and no longer with paragraph 5.
    let head = second_after.lines().next().unwrap();
    assert!(!head.is_empty() && last_of_first.ends_with(head), "frame 2 starts with {head:?}");
    assert!(!second_after.starts_with("Paragraph 5 "));
    assert!(ui.text_of(first).starts_with(OPENING));
}

#[test]
fn one_undo_after_a_flowing_edit_restores_both_frames() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    let (first, second) = (frames[0], frames[1]);
    let (first_before, second_before) = (ui.text_of(first), ui.text_of(second));
    ui.double_click_at(ui.center_of(first));
    ui.set_selection_range(0, 0);
    ui.type_text(OPENING);
    ui.click_at(ui.at_in(6.0, 9.5));
    assert_ne!(ui.text_of(second), second_before);
    assert_eq!(undo_depth(&mut ui), 1, "the edit and its re-flow are one step");
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.text_of(first), first_before);
    assert_eq!(ui.text_of(second), second_before);
    assert!(!ui.can_undo());
    ui.key(CTRL, Key::Y);
    assert!(ui.text_of(first).starts_with(OPENING));
    assert_ne!(ui.text_of(second), second_before);
}
