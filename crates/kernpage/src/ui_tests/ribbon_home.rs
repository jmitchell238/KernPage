//! Home tab of the ribbon, and the tab strip. Buttons are found through the ribbon seam
//! (`TestUi::ribbon`, by label or tooltip) and clicked with real pointer events; menus,
//! palettes and dialogs are then driven through their AccessKit labels.

use super::support::*;
use crate::app::{Tab, Tool};
use eframe::egui::{self, accesskit::Role, Key, Modifiers, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::model::PictureFormat;
use kernpage_core::{
    Align, LineSpacing, Paragraph, Rect, Rgb, Run, Shape, ShapeKind, TableCell, WrapType,
};
use std::sync::Arc;

const A: u64 = 1;
const B: u64 = 2;
const C: u64 = 3;
const NONE: Modifiers = Modifiers::NONE;

fn text_box_at(x: f64, y: f64, paragraphs: &[&str]) -> Shape {
    Shape::new(
        Rect::new(in_to_emu(x), in_to_emu(y), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox {
            paragraphs: paragraphs.iter().map(|t| Paragraph::plain(t)).collect(),
        },
    )
}

/// Text box A ("Hello world") at (1, 1), 4 x 2 in, and rectangle B at (1, 6).
fn text_ui() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        text_box_at(1.0, 1.0, &["Hello world"]),
        rect_shape(1.0, 6.0, 2.0, 1.5),
    ]))
}

/// Three rectangles A, B, C at different places.
fn three_rects() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.0),
        rect_shape(3.5, 3.0, 1.5, 1.0),
        rect_shape(6.0, 5.0, 1.0, 2.0),
    ]))
}

fn select(ui: &mut TestUi, id: u64) {
    ui.click_at(ui.center_of(id));
    assert_eq!(ui.selected(), vec![id], "shape {id} selected");
}

fn select_all(ui: &mut TestUi) {
    ui.click_at(ui.at_in(0.2, 0.2)); // empty page space first
    ui.key(CTRL, Key::A);
}

fn runs_of(ui: &TestUi, id: u64) -> Vec<Run> {
    ui.paragraphs(id).iter().flat_map(|p| p.runs.clone()).collect()
}

fn run0(ui: &TestUi, id: u64) -> Run {
    runs_of(ui, id).remove(0)
}

fn disabled(ui: &TestUi, name: &str) -> bool {
    !ui.ribbon(name).enabled
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

fn x_in(ui: &TestUi, id: u64) -> f64 {
    ui.rect_of(id).x as f64 / 914_400.0
}

fn order(ui: &TestUi) -> Vec<u64> {
    ui.shapes().iter().map(|s| s.id).collect()
}

/// A picture placeholder shape (bytes we cannot display are drawn as a placeholder).
fn picture() -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(2.0)),
        ShapeKind::Picture {
            data: Arc::new(vec![]),
            format: PictureFormat::Unknown,
            file: None,
        },
    )
}

fn table() -> Shape {
    let cells = (0..4).map(|i| TableCell::plain(&format!("c{i}"))).collect();
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::Table {
            rows: 2,
            cols: 2,
            col_widths_emu: vec![],
            row_heights_emu: vec![], row_set_emu: vec![],
            cells,
        },
    )
}

// ---- tabs ---------------------------------------------------------------------------------------

#[test]
fn clicking_each_tab_shows_that_tabs_groups() {
    let mut ui = text_ui();
    assert_eq!(ui.tab(), Tab::Home);
    assert!(ui.ribbon("tab:Home").selected, "Home starts selected");
    assert!(ui.has_ribbon("Cut"), "Home shows its Clipboard group");
    // A button that only that tab has, per tab.
    for (tab, name, marker) in [
        ("Insert", Tab::Insert, "Page Elements"),
        ("Page Design", Tab::PageDesign, "Margins"),
        ("Review", Tab::Review, "Spelling"),
        ("View", Tab::View, "Page Width"),
        ("Help", Tab::Help, "Report a Problem"),
    ]
    {
        ui.click_tab(tab);
        assert_eq!(ui.tab(), name, "{tab} tab selected");
        assert!(ui.ribbon(&format!("tab:{tab}")).selected, "{tab} is drawn selected");
        assert!(!ui.ribbon("tab:Home").selected, "Home is no longer selected");
        assert!(ui.has_ribbon(marker), "{tab} shows {marker}");
        assert!(!ui.has_ribbon("Cut"), "{tab} does not show the Home groups");
    }
    ui.click_tab("Home");
    assert_eq!(ui.tab(), Tab::Home);
    assert!(ui.has_ribbon("Cut"), "back on Home the Clipboard group is back");
}

#[test]
fn contextual_tabs_appear_and_disappear_with_the_selection() {
    let mut ui = TestUi::with_doc(letter_doc(vec![
        text_box_at(1.0, 1.0, &["Hi"]),
        rect_shape(1.0, 6.0, 2.0, 1.5),
        {
            let mut p = picture();
            p.rect = Rect::new(in_to_emu(5.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(2.0));
            p
        },
        {
            let mut t = table();
            t.rect = Rect::new(in_to_emu(4.0), in_to_emu(6.0), in_to_emu(4.0), in_to_emu(2.0));
            t
        },
    ]));
    let ctx_tabs = |ui: &TestUi| -> Vec<String> {
        ui.tab_names()
            .into_iter()
            .skip(7)
            .collect()
    };
    assert_eq!(ui.tab_names().len(), 7, "no selection: only the 7 fixed tabs");
    select(&mut ui, A);
    assert_eq!(ctx_tabs(&ui), ["Shape Format", "Text Box"], "text box");
    select(&mut ui, B);
    assert_eq!(ctx_tabs(&ui), ["Shape Format"], "shape");
    select(&mut ui, 3);
    assert_eq!(ctx_tabs(&ui), ["Picture Format"], "picture");
    select(&mut ui, 4);
    assert_eq!(ctx_tabs(&ui), ["Table Design", "Layout"], "table");
    ui.click_at(ui.at_in(0.3, 4.5));
    assert!(ctx_tabs(&ui).is_empty(), "clicking empty page removes them");
}

#[test]
fn a_contextual_tab_can_be_opened_and_falls_back_to_home_when_the_selection_goes() {
    let mut ui = text_ui();
    select(&mut ui, A);
    ui.click_tab("Text Box");
    assert_eq!(ui.tab(), Tab::TextBox);
    ui.click_at(ui.at_in(6.0, 9.0));
    assert_eq!(ui.tab(), Tab::Home, "deselecting leaves the contextual tab for Home");
    assert!(ui.has_ribbon("Cut"));
}

#[test]
fn a_mixed_selection_keeps_only_the_tabs_both_kinds_have() {
    let mut ui = text_ui();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    assert_eq!(ui.selected().len(), 2);
    let names: Vec<String> = ui.tab_names().into_iter().skip(7).collect();
    assert_eq!(names, ["Shape Format"], "text box + rectangle: only the common tab");
}

// ---- clipboard -----------------------------------------------------------------------------------

#[test]
fn cut_and_copy_need_a_selection_and_paste_needs_something_copied() {
    let mut ui = three_rects();
    for b in ["Cut (Ctrl+X)", "Copy (Ctrl+C)", "Paste (Ctrl+V)"] {
        assert!(disabled(&ui, b), "{b} is greyed with nothing selected / copied");
    }
    select(&mut ui, A);
    assert!(!disabled(&ui, "Cut (Ctrl+X)"));
    assert!(!disabled(&ui, "Copy (Ctrl+C)"));
    assert!(disabled(&ui, "Paste (Ctrl+V)"), "still nothing on the clipboard");
}

#[test]
fn the_copy_button_copies_and_the_paste_button_pastes_a_duplicate() {
    let mut ui = three_rects();
    select(&mut ui, A);
    let before = ui.revision();
    ui.click_ribbon("Copy");
    assert_eq!(ui.app().clipboard.len(), 1, "one shape on the clipboard");
    assert_eq!(ui.revision(), before, "copying does not edit the document");
    assert_eq!(ui.shapes().len(), 3);
    assert!(!disabled(&ui, "Paste"), "Paste is live once something was copied");
    ui.click_ribbon("Paste");
    assert_eq!(ui.shapes().len(), 4, "a duplicate appears");
    let dup = ui.shapes().last().unwrap().clone();
    assert_eq!(dup.rect.w, ui.rect_of(A).w);
    assert_ne!((dup.rect.x, dup.rect.y), (ui.rect_of(A).x, ui.rect_of(A).y), "pasted with an offset");
    assert_eq!(ui.selected(), vec![dup.id], "the pasted copy is selected");
    ui.key(CTRL, Key::Z);
    assert_eq!(ui.shapes().len(), 3, "paste is one undo step");
}

#[test]
fn the_cut_button_removes_the_selection_and_puts_it_on_the_clipboard() {
    let mut ui = three_rects();
    select(&mut ui, B);
    ui.click_ribbon("Cut");
    assert_eq!(order(&ui), vec![A, C], "B is gone");
    assert_eq!(ui.app().clipboard.len(), 1);
    ui.click_ribbon("Paste");
    assert_eq!(ui.shapes().len(), 3, "and Paste brings it back");
    assert_eq!(ui.shapes().last().unwrap().rect.w, in_to_emu(1.5));
}

// ---- font: size box, name box, grow / shrink -----------------------------------------------------

fn size_box<'a>(ui: &'a TestUi) -> egui_kittest::Node<'a> {
    // The Font Name box is the first text field in the ribbon, the Font Size box the second.
    ui.h.get_all_by_role(Role::TextInput).nth(1).expect("font size box")
}

fn name_box<'a>(ui: &'a TestUi) -> egui_kittest::Node<'a> {
    ui.h.get_all_by_role(Role::TextInput).next().expect("font name box")
}

fn type_size(ui: &mut TestUi, text: &str) {
    size_box(ui).click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text(text);
    ui.key(NONE, Key::Enter);
}

#[test]
fn the_size_box_shows_the_current_size() {
    let mut ui = text_ui();
    select(&mut ui, A);
    assert_eq!(size_box(&ui).value().as_deref(), Some("12"));
}

#[test]
fn typing_a_size_and_pressing_enter_applies_it_as_one_undo_step() {
    let mut ui = text_ui();
    select(&mut ui, A);
    type_size(&mut ui, "20");
    assert_eq!(run0(&ui, A).size_pt, 20.0);
    assert_eq!(undo_depth(&mut ui), 1);
    assert_eq!(size_box(&ui).value().as_deref(), Some("20"), "the box shows the new size");
    type_size(&mut ui, "10.5");
    assert_eq!(run0(&ui, A).size_pt, 10.5, "half points are allowed");
}

#[test]
fn an_invalid_size_is_ignored_and_the_box_goes_back_to_the_real_size() {
    let mut ui = text_ui();
    select(&mut ui, A);
    for bad in ["abc", "", "0", "-5", "5000"] {
        type_size(&mut ui, bad);
        assert_eq!(run0(&ui, A).size_pt, 12.0, "{bad:?} changes nothing");
        assert_eq!(size_box(&ui).value().as_deref(), Some("12"), "{bad:?}: box reverts");
    }
    assert!(!ui.can_undo(), "no command ran");
}

#[test]
fn the_size_list_picks_a_size() {
    let mut ui = text_ui();
    select(&mut ui, A);
    let combos: Vec<_> = ui.h.get_all_by_role(Role::ComboBox).collect();
    assert_eq!(combos.len(), 2, "font name and size lists");
    combos[1].click();
    ui.settle();
    // (The list is taller than its pop-up, so only the first sizes are on screen.)
    ui.h.get_by_label("14").click();
    ui.settle();
    assert_eq!(run0(&ui, A).size_pt, 14.0);
    assert_eq!(size_box(&ui).value().as_deref(), Some("14"), "the size box follows");
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn the_font_name_list_applies_the_chosen_family() {
    let mut ui = text_ui();
    ui.wait_for_fonts();
    select(&mut ui, A);
    let box_ = ui.h.get_all_by_role(Role::ComboBox).next().expect("font name box");
    box_.click();
    ui.settle();
    let families = ui.app().fonts.families.clone();
    let target = families.iter().find(|f| **f != "Calibri").expect("a second family").clone();
    ui.h.get_by_label(&target).click();
    ui.settle();
    assert_eq!(run0(&ui, A).font, target);
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn font_controls_are_greyed_when_no_text_is_selected() {
    let mut ui = three_rects();
    select(&mut ui, A);
    for b in ["Grow Font", "Shrink Font", "Clear Formatting", "Bold (Ctrl+B)", "Italic (Ctrl+I)", "Underline (Ctrl+U)", "Font Color"] {
        assert!(disabled(&ui, b), "{b} is greyed for a rectangle");
    }
}

#[test]
fn grow_and_shrink_font_step_through_the_size_list() {
    let mut ui = text_ui();
    select(&mut ui, A);
    let mut seen = vec![];
    for _ in 0..3 {
        ui.click_ribbon("Grow Font");
        seen.push(run0(&ui, A).size_pt);
    }
    assert_eq!(seen, [14.0, 16.0, 18.0], "12 -> 14 -> 16 -> 18");
    for want in [16.0, 14.0, 12.0, 11.0, 10.5, 10.0] {
        ui.click_ribbon("Shrink Font");
        assert_eq!(run0(&ui, A).size_pt, want);
    }
}

#[test]
fn grow_font_stops_at_72_and_shrink_font_at_8() {
    let mut ui = text_ui();
    select(&mut ui, A);
    type_size(&mut ui, "72");
    assert!(disabled(&ui, "Grow Font"), "nothing bigger than 72 in the list");
    assert!(!disabled(&ui, "Shrink Font"));
    ui.click_ribbon("Shrink Font");
    assert_eq!(run0(&ui, A).size_pt, 48.0);
    type_size(&mut ui, "8");
    assert!(disabled(&ui, "Shrink Font"), "nothing smaller than 8");
}

// ---- bold / italic / underline --------------------------------------------------------------------

#[test]
fn bold_italic_and_underline_toggle_and_show_the_pressed_state() {
    let mut ui = text_ui();
    select(&mut ui, A);
    type CheckFn = fn(&Run) -> bool;
    let cases: [(&str, CheckFn); 3] = [
        ("Bold (Ctrl+B)", |r| r.bold),
        ("Italic (Ctrl+I)", |r| r.italic),
        ("Underline (Ctrl+U)", |r| r.underline),
    ];
    for (name, get) in cases {
        assert!(!ui.ribbon(name).selected, "{name} starts up");
        ui.click_ribbon(name);
        assert!(runs_of(&ui, A).iter().all(get), "{name} applied");
        assert!(ui.ribbon(name).selected, "{name} is shown pressed");
        ui.click_ribbon(name);
        assert!(runs_of(&ui, A).iter().all(|r| !get(r)), "{name} removed");
        assert!(!ui.ribbon(name).selected, "{name} is up again");
    }
    assert_eq!(undo_depth(&mut ui), 6);
}

#[test]
fn mixed_bold_shows_unpressed_and_a_click_makes_everything_bold() {
    let mut ui = TestUi::with_sample("bold.pub");
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    assert!(runs_of(&ui, id).iter().any(|r| r.bold) && runs_of(&ui, id).iter().any(|r| !r.bold));
    assert!(!ui.ribbon("Bold (Ctrl+B)").selected, "mixed is not pressed");
    ui.click_ribbon("Bold (Ctrl+B)");
    assert!(runs_of(&ui, id).iter().all(|r| r.bold));
    assert!(ui.ribbon("Bold (Ctrl+B)").selected);
}

// ---- font colour --------------------------------------------------------------------------------------

/// Click swatch number `n` (row-major, 8 per row) of the open Font Color palette. The swatches
/// are the unnamed 18 x 18 pt nodes of the pop-up.
fn pick_swatch(ui: &mut TestUi, n: usize) {
    let mut swatches: Vec<(f64, f64)> = ui
        .h
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().raw_bounds())
        .filter(|r| r.width() == 18.0 && r.height() == 18.0 && r.y0 > 100.0)
        .map(|r| ((r.y0 * 10.0).round() / 10.0, r.x0))
        .collect();
    swatches.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(swatches.len() >= 40, "palette swatches found: {}", swatches.len());
    let (y, x) = swatches[n];
    ui.click_at(Pos2::new(x as f32 + 9.0, y as f32 + 9.0));
}

#[test]
fn the_font_color_main_button_applies_the_last_colour() {
    let mut ui = text_ui();
    select(&mut ui, A);
    assert_eq!(ui.app().last_font_color, Rgb(255, 0, 0), "red until another is picked");
    ui.click_ribbon("Font Color");
    assert!(runs_of(&ui, A).iter().all(|r| r.color == Rgb(255, 0, 0)));
    assert_eq!(undo_depth(&mut ui), 1);
}

#[test]
fn the_font_color_arrow_opens_the_palette_and_a_swatch_applies_and_becomes_the_last_colour() {
    let mut ui = text_ui();
    select(&mut ui, A);
    ui.click_ribbon_arrow("Font Color");
    assert!(
        ui.h.get_all_by_label("Font Color").any(|n| n.accesskit_node().role() == Role::Label),
        "the arrow opens the palette"
    );
    assert!(runs_of(&ui, A).iter().all(|r| r.color == Rgb::BLACK), "opening changes nothing");
    pick_swatch(&mut ui, 10);
    let want = crate::widgets::palette()[10];
    assert_eq!(run0(&ui, A).color, Rgb(want[0], want[1], want[2]), "swatch 10 (row 2, column 3) applied");
    assert_eq!(ui.app().last_font_color, Rgb(want[0], want[1], want[2]));
    assert!(
        !ui.h.get_all_by_label("Font Color").any(|n| n.accesskit_node().role() == Role::Label),
        "picking closes the palette"
    );
    // The main button now uses the picked colour.
    ui.click_ribbon("Clear Formatting");
    assert_eq!(run0(&ui, A).color, Rgb::BLACK);
    ui.click_ribbon("Font Color");
    assert_eq!(run0(&ui, A).color, Rgb(want[0], want[1], want[2]), "main part repeats the last colour");
}

// ---- clear formatting -----------------------------------------------------------------------------------

#[test]
fn clear_formatting_resets_the_runs_in_one_undo_step() {
    let mut ui = text_ui();
    select(&mut ui, A);
    type_size(&mut ui, "30");
    ui.click_ribbon("Bold (Ctrl+B)");
    ui.click_ribbon("Italic (Ctrl+I)");
    ui.click_ribbon("Underline (Ctrl+U)");
    ui.click_ribbon("Font Color");
    let before = undo_depth(&mut ui);
    ui.click_ribbon("Clear Formatting");
    let r = run0(&ui, A);
    assert!(!r.bold && !r.italic && !r.underline, "styles removed");
    assert_eq!(r.color, Rgb::BLACK);
    assert_eq!(r.font, "Calibri");
    assert_eq!(r.size_pt, kernpage_core::model::FILE_DEFAULT_SIZE_PT, "back to the default size");
    assert_eq!(undo_depth(&mut ui), before + 1);
    assert_eq!(runs_of(&ui, A).iter().map(|r| r.text.as_str()).collect::<String>(), "Hello world", "text kept");
}

// ---- paragraph ------------------------------------------------------------------------------------------

#[test]
fn the_align_buttons_align_every_paragraph_of_the_selected_box_and_show_pressed() {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box_at(1.0, 1.0, &["One", "Two"])]));
    select(&mut ui, A);
    assert!(ui.ribbon("Align Left (Ctrl+L)").selected, "left is the starting alignment");
    for (tip, want) in [
        ("Center (Ctrl+E)", Align::Center),
        ("Align Right (Ctrl+R)", Align::Right),
        ("Justify (Ctrl+J)", Align::Justify),
        ("Align Left (Ctrl+L)", Align::Left),
    ] {
        ui.click_ribbon(tip);
        assert!(ui.paragraphs(A).iter().all(|p| p.align == want), "{tip}");
        assert!(ui.ribbon(tip).selected, "{tip} is shown pressed");
        let pressed = ui
            .ribbon_buttons()
            .iter()
            .filter(|b| b.rect.min.y > 78.0 && b.selected && b.tip.contains("(Ctrl+") && ["L", "E", "R", "J"].iter().any(|k| b.tip.ends_with(&format!("(Ctrl+{k})"))))
            .count();
        assert_eq!(pressed, 1, "only one alignment is pressed");
    }
    assert_eq!(undo_depth(&mut ui), 4);
}

#[test]
fn the_paragraph_buttons_are_greyed_for_a_shape_without_text() {
    let mut ui = three_rects();
    select(&mut ui, A);
    for b in ["Align Left (Ctrl+L)", "Center (Ctrl+E)", "Align Right (Ctrl+R)", "Justify (Ctrl+J)", "Line Spacing"] {
        assert!(disabled(&ui, b), "{b}");
    }
}

fn spacing_of(ui: &TestUi, id: u64, para: usize) -> kernpage_core::ParaSpacing {
    ui.paragraphs(id)[para].spacing
}

#[test]
fn the_line_spacing_menu_applies_the_chosen_multiple() {
    let mut ui = text_ui();
    select(&mut ui, A);
    for (label, want) in [("1.5", 1.5f32), ("2", 2.0), ("1.15", 1.15), ("3", 3.0), ("2.5", 2.5), ("1", 1.0)] {
        ui.click_ribbon("Line Spacing");
        let item = ui
            .h
            .get_all_by_label_contains(label)
            .find(|n| n.accesskit_node().label().is_some_and(|t| t.trim_start_matches(['\u{2713}', ' ']) == label))
            .expect("menu item");
        item.click();
        ui.settle();
        assert_eq!(spacing_of(&ui, A, 0).line, LineSpacing::Multiple(want), "{label}");
    }
}

#[test]
fn the_line_spacing_menu_marks_the_current_spacing() {
    let mut ui = text_ui();
    select(&mut ui, A);
    ui.click_ribbon("Line Spacing");
    ui.h.get_by_label_contains("1.5").click();
    ui.settle();
    ui.click_ribbon("Line Spacing");
    let checked: Vec<String> = ["1.15", "1.5", "2.5"]
        .iter()
        .filter(|l| ui.h.get_by_label_contains(l).accesskit_node().label().is_some_and(|t| t.contains('\u{2713}')))
        .map(|l| l.to_string())
        .collect();
    assert_eq!(checked, ["1.5"], "only the current multiple has the check mark");
}

#[test]
fn line_spacing_options_dialog_opens_prefilled_and_ok_applies_in_one_undo_step() {
    let mut ui = text_ui();
    select(&mut ui, A);
    ui.click_ribbon("Line Spacing");
    ui.h.get_by_label("Line Spacing Options...").click();
    ui.settle();
    ui.h.get_by_label("Line Spacing Options"); // the window title
    {
        let d = ui.app().spacing_dialog.as_ref().expect("dialog open");
        assert_eq!(
            (d.before, d.after, d.exact, d.line),
            (0.0, 6.0, false, 1.19),
            "prefilled from the paragraph (Publisher's default: 6 pt after, 1.19 lines)"
        );
    }
    set_spin(&mut ui, 0, "6");
    set_spin(&mut ui, 1, "12");
    set_spin(&mut ui, 2, "1.5");
    ui.h.get_by_label("  OK  ").click();
    ui.settle();
    assert!(ui.app().spacing_dialog.is_none(), "OK closes the dialog");
    let sp = spacing_of(&ui, A, 0);
    assert_eq!((sp.space_before_pt, sp.space_after_pt), (6.0, 12.0));
    assert_eq!(sp.line, LineSpacing::Multiple(1.5));
    assert_eq!(undo_depth(&mut ui), 1, "one undo step for all three values");
}

/// Type `text` into the `n`th spin box (drag value) of the Line Spacing Options dialog.
fn set_spin(ui: &mut TestUi, n: usize, text: &str) {
    let node = ui.h.get_all_by_role(Role::SpinButton).nth(n).expect("spin box");
    node.click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text(text);
    ui.key(NONE, Key::Enter);
}

#[test]
fn line_spacing_options_cancel_and_escape_discard_the_changes() {
    for use_escape in [false, true] {
        let mut ui = text_ui();
        select(&mut ui, A);
        ui.click_ribbon("Line Spacing");
        ui.h.get_by_label("Line Spacing Options...").click();
        ui.settle();
        set_spin(&mut ui, 0, "30");
        if use_escape {
            ui.key(NONE, Key::Escape);
        } else {
            ui.h.get_by_label("Cancel").click();
            ui.settle();
        }
        assert!(ui.app().spacing_dialog.is_none(), "dialog closed (escape: {use_escape})");
        assert_eq!(spacing_of(&ui, A, 0).space_before_pt, 0.0, "nothing applied");
        assert!(!ui.can_undo());
    }
}

#[test]
fn line_spacing_options_exactly_mode_applies_points() {
    let mut ui = text_ui();
    select(&mut ui, A);
    ui.click_ribbon("Line Spacing");
    ui.h.get_by_label("Line Spacing Options...").click();
    ui.settle();
    ui.h.get_by_label("Exactly").click();
    ui.settle();
    ui.h.get_by_label("Points:");
    assert_eq!(ui.app().spacing_dialog.as_ref().unwrap().line, 12.0, "starts at 12 pt");
    set_spin(&mut ui, 2, "18");
    ui.h.get_by_label("  OK  ").click();
    ui.settle();
    assert_eq!(spacing_of(&ui, A, 0).line, LineSpacing::Exactly(18.0));
}

// ---- objects ------------------------------------------------------------------------------------------

fn drag_box(ui: &mut TestUi) {
    ui.drag(ui.at_in(4.0, 3.0), ui.at_in(6.0, 4.0));
}

#[test]
fn draw_text_box_arms_the_tool_shows_pressed_and_the_next_drag_creates_a_text_box() {
    let mut ui = three_rects();
    assert!(!ui.ribbon("Draw Text Box").selected);
    ui.click_ribbon("Draw Text Box");
    assert_eq!(ui.app().tool, Tool::TextBox);
    assert!(ui.ribbon("Draw Text Box").selected, "shown pressed while armed");
    ui.drag(ui.at_in(1.0, 8.0), ui.at_in(3.0, 9.0));
    let n = ui.shapes().len();
    assert_eq!(n, 4);
    assert!(matches!(ui.shapes().last().unwrap().kind, ShapeKind::TextBox { .. }));
    assert_eq!(ui.app().tool, Tool::Select, "the tool resets after one use");
    assert!(!ui.ribbon("Draw Text Box").selected);
}

#[test]
fn clicking_draw_text_box_twice_disarms_it() {
    let mut ui = three_rects();
    ui.click_ribbon("Draw Text Box");
    ui.click_ribbon("Draw Text Box");
    assert_eq!(ui.app().tool, Tool::Select);
    drag_box(&mut ui);
    assert_eq!(ui.shapes().len(), 3, "a drag now only draws a marquee");
}

#[test]
fn the_shapes_menu_arms_rectangle_oval_and_line() {
    for (item, tool, kind) in [
        ("Rectangle", Tool::Rectangle, ShapeKind::Rectangle),
        ("Oval", Tool::Oval, ShapeKind::Ellipse),
        ("Line", Tool::Line, ShapeKind::Line),
    ] {
        let mut ui = three_rects();
        ui.click_ribbon("Shapes");
        ui.h.get_by_label(item).click();
        ui.settle();
        assert_eq!(ui.app().tool, tool, "{item} armed");
        ui.drag(ui.at_in(1.0, 8.0), ui.at_in(3.0, 9.5));
        assert_eq!(ui.shapes().len(), 4, "{item}: one shape drawn");
        assert_eq!(ui.shapes().last().unwrap().kind, kind, "{item}");
        assert_eq!(ui.app().tool, Tool::Select);
    }
}

#[test]
fn the_pictures_button_is_available_with_a_document() {
    let ui = three_rects();
    assert!(!disabled(&ui, "Pictures"), "enabled (its file dialog is native, not clicked here)");
    assert!(!disabled(&ui, "Draw Text Box"));
    assert!(!disabled(&ui, "Shapes"));
}

// ---- arrange: group, order, align ------------------------------------------------------------------------

#[test]
fn group_and_ungroup_follow_the_enabled_rules() {
    let mut ui = three_rects();
    assert!(disabled(&ui, "Group") && disabled(&ui, "Ungroup"), "nothing selected");
    select(&mut ui, A);
    assert!(disabled(&ui, "Group"), "one object cannot be grouped");
    assert!(disabled(&ui, "Ungroup"), "and it is not a group");
    ui.click_mods(ui.center_of(B), SHIFT);
    assert!(!disabled(&ui, "Group"), "two objects can");
    assert!(disabled(&ui, "Ungroup"));
    ui.click_ribbon("Group");
    assert_eq!(ui.shapes().len(), 4, "a group shape was added");
    let g = ui.shapes().iter().find(|s| matches!(s.kind, ShapeKind::Group)).expect("group").id;
    assert_eq!(ui.selected(), vec![g], "the group is selected");
    assert!(disabled(&ui, "Group"), "a lone group cannot be grouped");
    assert!(!disabled(&ui, "Ungroup"));
    ui.click_ribbon("Ungroup");
    assert_eq!(ui.shapes().len(), 3, "the group shape is gone");
    assert!(ui.shapes().iter().all(|s| s.group.is_none()), "members are free again");
    assert_eq!(undo_depth(&mut ui), 2);
}

#[test]
fn bring_forward_and_send_backward_move_one_step() {
    let mut ui = three_rects();
    assert!(disabled(&ui, "Bring Forward") && disabled(&ui, "Send Backward"), "needs a selection");
    select(&mut ui, A);
    ui.click_ribbon("Bring Forward");
    assert_eq!(order(&ui), vec![B, A, C]);
    ui.click_ribbon("Send Backward");
    assert_eq!(order(&ui), vec![A, B, C]);
}

#[test]
fn the_split_arrows_offer_bring_to_front_and_send_to_back() {
    let mut ui = three_rects();
    select(&mut ui, B);
    ui.click_ribbon_arrow("Bring Forward");
    {
        // The Home tab's small Arrange buttons carry the same names, so look only below the ribbon.
        use egui_kittest::kittest::Queryable;
        let in_menu = |l: &str| ui.h.query_all_by_label(l).any(|n| n.rect().top() > 116.0);
        assert!(!in_menu("Send Backward"), "only its own two choices");
        assert!(in_menu("Bring Forward"));
    }
    ui.h.get_by_label("Bring to Front").click();
    ui.settle();
    assert_eq!(order(&ui), vec![A, C, B]);
    ui.click_ribbon_arrow("Send Backward");
    ui.h.get_by_label("Send to Back").click();
    ui.settle();
    assert_eq!(order(&ui), vec![B, A, C]);
    assert_eq!(undo_depth(&mut ui), 2);
}

fn open_align_menu(ui: &mut TestUi) {
    ui.click_ribbon("Align");
}

fn menu_item_disabled(ui: &TestUi, label: &str) -> bool {
    ui.h.get_by_label(label).accesskit_node().is_disabled()
}

#[test]
fn align_is_greyed_without_a_selection_and_its_items_need_two_objects() {
    let mut ui = three_rects();
    assert!(disabled(&ui, "Align"));
    select(&mut ui, A);
    open_align_menu(&mut ui);
    for item in ["Align Left", "Align Center", "Align Right", "Align Top", "Align Middle", "Align Bottom", "Distribute Horizontally", "Distribute Vertically"] {
        assert!(menu_item_disabled(&ui, item), "{item} needs more than one object");
    }
}

#[test]
fn align_items_line_up_the_selected_objects() {
    // (menu item, which edge/centre must match on the shapes)
    type Get = fn(&Shape) -> i64;
    let cases: [(&str, Get); 6] = [
        ("Align Left", |s| s.rect.x),
        ("Align Center", |s| s.rect.x + s.rect.w / 2),
        ("Align Right", |s| s.rect.x + s.rect.w),
        ("Align Top", |s| s.rect.y),
        ("Align Middle", |s| s.rect.y + s.rect.h / 2),
        ("Align Bottom", |s| s.rect.y + s.rect.h),
    ];
    for (item, get) in cases {
        let mut ui = three_rects();
        select_all(&mut ui);
        assert_eq!(ui.selected().len(), 3);
        open_align_menu(&mut ui);
        ui.h.get_by_label(item).click();
        ui.settle();
        let vals: Vec<i64> = ui.shapes().iter().map(get).collect();
        assert!(vals.windows(2).all(|w| (w[0] - w[1]).abs() <= 1), "{item}: {vals:?}");
        assert_eq!(undo_depth(&mut ui), 1, "{item} is one undo step");
    }
}

#[test]
fn align_left_moves_to_the_leftmost_edge_of_the_selection() {
    let mut ui = three_rects();
    select_all(&mut ui);
    open_align_menu(&mut ui);
    ui.h.get_by_label("Align Left").click();
    ui.settle();
    for id in [A, B, C] {
        assert!((x_in(&ui, id) - 1.0).abs() < 0.001, "shape {id} at the left-most x (1.0)");
    }
}

#[test]
fn distribute_needs_three_objects_and_spaces_the_gaps_evenly() {
    let mut ui = three_rects();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    open_align_menu(&mut ui);
    assert!(menu_item_disabled(&ui, "Distribute Horizontally"), "two objects: greyed");
    ui.key(NONE, Key::Escape);
    select_all(&mut ui);
    open_align_menu(&mut ui);
    assert!(!menu_item_disabled(&ui, "Distribute Horizontally"));
    ui.h.get_by_label("Distribute Horizontally").click();
    ui.settle();
    let gap1 = ui.rect_of(B).x - (ui.rect_of(A).x + ui.rect_of(A).w);
    let gap2 = ui.rect_of(C).x - (ui.rect_of(B).x + ui.rect_of(B).w);
    assert!((gap1 - gap2).abs() <= 1, "gaps {gap1} and {gap2} are equal");
    open_align_menu(&mut ui);
    ui.h.get_by_label("Distribute Vertically").click();
    ui.settle();
    let g1 = ui.rect_of(B).y - (ui.rect_of(A).y + ui.rect_of(A).h);
    let g2 = ui.rect_of(C).y - (ui.rect_of(B).y + ui.rect_of(B).h);
    assert!((g1 - g2).abs() <= 1, "vertical gaps {g1} and {g2} are equal");
}

#[test]
fn relative_to_margin_guides_aligns_a_single_object_to_the_margins() {
    let mut ui = three_rects();
    select(&mut ui, B);
    open_align_menu(&mut ui);
    ui.h.get_by_label("Relative to Margin Guides").click();
    ui.settle();
    assert!(ui.app().align_to_margins, "the option is on");
    assert!(!ui.can_undo(), "toggling the option edits nothing");
    open_align_menu(&mut ui);
    assert!(!menu_item_disabled(&ui, "Align Left"), "one object is enough relative to margins");
    assert!(menu_item_disabled(&ui, "Distribute Horizontally"), "distribute still needs two");
    ui.h.get_by_label("Align Left").click();
    ui.settle();
    assert!((x_in(&ui, B) - 0.5).abs() < 0.001, "at the 0.5 in left margin, got {}", x_in(&ui, B));
    open_align_menu(&mut ui);
    ui.h.get_by_label("Align Right").click();
    ui.settle();
    let right = (ui.rect_of(B).x + ui.rect_of(B).w) as f64 / 914_400.0;
    assert!((right - 8.0).abs() < 0.001, "right edge at the right margin (8.0), got {right}");
    open_align_menu(&mut ui);
    ui.h.get_by_label("Relative to Margin Guides").click();
    ui.settle();
    assert!(!ui.app().align_to_margins, "choosing it again turns it off");
}

#[test]
fn distribute_relative_to_margins_needs_two_objects() {
    let mut ui = three_rects();
    ui.app().align_to_margins = true;
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    open_align_menu(&mut ui);
    assert!(!menu_item_disabled(&ui, "Distribute Horizontally"));
    ui.h.get_by_label("Distribute Horizontally").click();
    ui.settle();
    let left = ui.rect_of(A).x.min(ui.rect_of(B).x);
    let right = (ui.rect_of(A).x + ui.rect_of(A).w).max(ui.rect_of(B).x + ui.rect_of(B).w);
    assert_eq!(left, in_to_emu(0.5), "first object at the left margin");
    assert_eq!(right, in_to_emu(8.0), "last object at the right margin");
}

// ---- wrap text, columns, rotate ----------------------------------------------------------------------------------

#[test]
fn wrap_text_is_greyed_without_a_selection() {
    let ui = three_rects();
    assert!(disabled(&ui, "Wrap Text"));
}

#[test]
fn the_wrap_text_menu_sets_each_wrap_type_and_marks_the_current_one() {
    let mut ui = three_rects();
    select(&mut ui, A);
    let start = ui.shape(A).wrap;
    for (item, want) in [
        ("Square", WrapType::Square),
        ("Tight", WrapType::Tight),
        ("Through", WrapType::Through),
        ("Top and Bottom", WrapType::TopAndBottom),
        ("None", WrapType::None),
    ] {
        ui.click_ribbon("Wrap Text");
        ui.h.get_by_label(item).click();
        ui.settle();
        assert_eq!(ui.shape(A).wrap, want, "{item}");
    }
    assert_ne!(start, WrapType::Square);
    ui.click_ribbon("Wrap Text");
    for item in ["Edit Wrap Points", "More Layout Options..."] {
        assert!(menu_item_disabled(&ui, item), "{item} is not built yet");
    }
}

fn columns_button(ui: &TestUi) -> Seen {
    // Icon-only and nameless: the one enabled, unnamed button on a text box's Home tab.
    let hits: Vec<Seen> = ui
        .ribbon_buttons()
        .into_iter()
        .filter(|b| b.label.is_empty() && b.tip.is_empty() && b.enabled && b.rect.min.y > 60.0 && b.rect.min.y < 106.0)
        .collect();
    assert_eq!(hits.len(), 1, "exactly one enabled unnamed button: {hits:?}");
    hits.into_iter().next().unwrap()
}

#[test]
fn the_columns_menu_sets_one_two_or_three_columns_on_a_text_box() {
    let mut ui = text_ui();
    select(&mut ui, A);
    for (item, want) in [("Two", 2), ("Three", 3), ("One", 1)] {
        let b = columns_button(&ui);
        ui.click_at(b.rect.center());
        ui.h.get_by_label(item).click();
        ui.settle();
        assert_eq!(ui.shape(A).columns, want, "{item}");
    }
    let b = columns_button(&ui);
    ui.click_at(b.rect.center());
    assert!(menu_item_disabled(&ui, "More Columns..."));
}

#[test]
fn the_columns_button_is_greyed_for_a_shape_that_is_not_a_text_box() {
    let mut ui = three_rects();
    select(&mut ui, A);
    let enabled_unnamed = ui
        .ribbon_buttons()
        .into_iter()
        .filter(|b| b.label.is_empty() && b.tip.is_empty() && b.enabled && b.rect.min.y > 60.0 && b.rect.min.y < 106.0)
        .count();
    assert_eq!(enabled_unnamed, 0, "no unnamed button is live, columns included");
}

#[test]
fn rotate_right_and_left_turn_the_selection_by_90_degrees() {
    let mut ui = three_rects();
    assert!(disabled(&ui, "Rotate"));
    select(&mut ui, A);
    ui.click_ribbon("Rotate");
    ui.h.get_by_label("Rotate Right 90\u{b0}").click();
    ui.settle();
    assert_eq!(ui.shape(A).rotation_deg, 90.0);
    ui.click_ribbon("Rotate");
    ui.h.get_by_label("Rotate Left 90\u{b0}").click();
    ui.settle();
    assert_eq!(ui.shape(A).rotation_deg % 360.0, 0.0, "right then left is back where it started");
    assert_eq!(undo_depth(&mut ui), 2);
}

#[test]
fn flip_is_available_for_lines_only_and_more_rotation_options_is_greyed() {
    let mut line = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(1.0)),
        ShapeKind::Line,
    );
    line.line = Some(kernpage_core::Stroke { color: Rgb::BLACK, width_emu: 12_700 });
    let mut ui = TestUi::with_doc(letter_doc(vec![line, rect_shape(1.0, 6.0, 2.0, 1.5)]));
    select(&mut ui, B);
    ui.click_ribbon("Rotate");
    assert!(menu_item_disabled(&ui, "Flip Horizontal") && menu_item_disabled(&ui, "Flip Vertical"), "a rectangle cannot flip yet");
    ui.key(NONE, Key::Escape);
    ui.click_at(ui.at_in(0.2, 4.5));
    select(&mut ui, A);
    ui.click_ribbon("Rotate");
    assert!(!menu_item_disabled(&ui, "Flip Horizontal"));
    assert!(menu_item_disabled(&ui, "More Rotation Options..."));
    ui.h.get_by_label("Flip Horizontal").click();
    ui.settle();
    assert!(ui.shape(A).flip_h && !ui.shape(A).flip_v);
    ui.click_ribbon("Rotate");
    ui.h.get_by_label("Flip Vertical").click();
    ui.settle();
    assert!(ui.shape(A).flip_h && ui.shape(A).flip_v);
}

// ---- greyed "coming soon" controls ------------------------------------------------------------------------------

#[test]
fn every_coming_soon_control_on_home_is_disabled_and_does_nothing() {
    let mut ui = text_ui();
    select(&mut ui, A);
    let named = ["Copy Formatting", "Styles", "Find", "Replace", "Select"];
    for n in named {
        assert!(disabled(&ui, n), "{n} is greyed");
    }
    // Icon-only ones (Change Case, Character Spacing, Show Formatting Marks, Shading; Subscript
    // and Superscript are live now, module `script`): no tooltip, no name. Bullets, Numbering and the indent buttons are live
    // and named now (module `lists`).
    let unnamed: Vec<Seen> = ui
        .ribbon_buttons()
        .into_iter()
        .filter(|b| b.label.is_empty() && b.tip.is_empty() && b.rect.min.y > 60.0 && b.rect.min.y < 136.0)
        .collect();
    let off: Vec<&Seen> = unnamed.iter().filter(|b| !b.enabled).collect();
    assert_eq!(off.len(), 4, "four greyed icon-only controls: {off:?}");
    assert_eq!(unnamed.len() - off.len(), 1, "and only Columns is live among the unnamed");
    let (rev, tab, tool) = (ui.revision(), ui.tab(), ui.app().tool);
    let mut targets: Vec<Pos2> = off.iter().map(|b| b.rect.center()).collect();
    for n in named {
        targets.push(ui.ribbon(n).rect.center());
    }
    for p in targets {
        ui.click_at(p);
        assert_eq!(ui.revision(), rev, "a click at {p:?} changed the document");
        assert_eq!(ui.tab(), tab);
        assert_eq!(ui.app().tool, tool);
        assert!(ui.app().spacing_dialog.is_none());
        assert_eq!(ui.selected(), vec![A], "selection kept");
        assert!(ui.app().edit.is_none());
    }
    assert!(!ui.can_undo());
}

#[test]
fn the_coming_soon_controls_stay_disabled_with_no_selection_too() {
    let ui = three_rects();
    for n in ["Copy Formatting", "Styles", "Find", "Replace", "Select"] {
        assert!(disabled(&ui, n), "{n}");
    }
}

#[test]
fn the_size_list_scrolls_to_the_largest_sizes() {
    let mut ui = text_ui();
    select(&mut ui, A);
    let combos: Vec<_> = ui.h.get_all_by_role(Role::ComboBox).collect();
    combos[1].click();
    ui.settle();
    ui.wheel_at(Pos2::new(345.0, 150.0), eframe::egui::vec2(0.0, -400.0), NONE);
    ui.settle();
    ui.h.get_by_label("72").click();
    ui.settle();
    assert_eq!(run0(&ui, A).size_pt, 72.0, "72 pt is reachable by scrolling the list");
}

// ---- ribbon look: UI font and size box width (pw-r3i0, pw-23e6) --------------------------------

#[test]
fn the_ui_font_is_segoe_ui_when_installed() {
    let ui = TestUi::start();
    let ctx = ui.h.ctx.clone();
    let installed = crate::fontmgr::ui_font_file().is_some();
    let first = ctx.fonts(|f| {
        f.definitions()
            .families
            .get(&egui::FontFamily::Proportional)
            .and_then(|l| l.first().cloned())
    });
    if installed {
        assert_eq!(first.as_deref(), Some(crate::fontmgr::UI_FONT_KEY));
    } else {
        assert_ne!(first.as_deref(), Some(crate::fontmgr::UI_FONT_KEY));
    }
    // Page text keeps egui's own fonts.
    let page = ctx.fonts(|f| {
        f.definitions()
            .families
            .get(&egui::FontFamily::Name(crate::fontmgr::PAGE_FALLBACK.into()))
            .and_then(|l| l.first().cloned())
    });
    assert_ne!(page.as_deref(), Some(crate::fontmgr::UI_FONT_KEY));
}

#[test]
fn fonts_come_from_the_system_and_every_family_has_one() {
    // No bundled egui fonts: every family must still end up with at least one usable font.
    let defs = crate::fontmgr::base_definitions();
    for fam in [
        egui::FontFamily::Proportional,
        egui::FontFamily::Monospace,
        egui::FontFamily::Name(crate::fontmgr::PAGE_FALLBACK.into()),
    ] {
        let list = defs.families.get(&fam).cloned().unwrap_or_default();
        assert!(!list.is_empty(), "{fam:?} has no font");
        assert!(list.iter().all(|k| defs.font_data.contains_key(k)), "{fam:?} names a missing font");
    }
    for name in ["Hack", "Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"] {
        assert!(!defs.font_data.contains_key(name), "{name} must not be bundled");
    }
}

/// The size of `text` drawn with the UI font of `plan` (two frames: fonts apply at the start of the next).
fn ui_text_size(plan: &kernpage_core::sysfonts::FontPlan, text: &str) -> egui::Vec2 {
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::fontmgr::definitions_from(plan));
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    let mut size = egui::Vec2::ZERO;
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        size = ui
            .painter()
            .layout_no_wrap(text.to_owned(), egui::FontId::proportional(14.0), egui::Color32::BLACK)
            .size();
    });
    out.textures_delta.clear();
    size
}

#[test]
fn adding_noto_sans_to_the_fallback_chain_does_not_change_segoe_ui_text() {
    use kernpage_core::sysfonts;
    let mut found = sysfonts::locate();
    if found.ui.is_none() || found.fallback_ui.is_none() {
        return; // needs both Segoe UI and the bundled Noto Sans on this computer
    }
    let with = sysfonts::plan_from(&found);
    found.fallback_ui = None;
    let without = sysfonts::plan_from(&found);
    for text in ["Home Insert Page Design 12 10.5", "Paste Cut Copy Format Painter", "The quick brown fox"] {
        assert_eq!(ui_text_size(&with, text), ui_text_size(&without, text), "{text}");
    }
}

#[test]
fn the_page_font_family_does_not_gain_noto_sans() {
    let defs = crate::fontmgr::base_definitions();
    let page = defs.families.get(&egui::FontFamily::Name(crate::fontmgr::PAGE_FALLBACK.into())).unwrap();
    assert!(!page.iter().any(|k| k == kernpage_core::sysfonts::FALLBACK_UI_KEY));
}

#[test]
fn the_size_box_is_wide_enough_for_12_and_10_5() {
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::fontmgr::base_definitions());
    let mut widest = 0.0f32;
    // Two frames: the fonts are applied at the start of the next one.
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        for t in ["12", "10.5", "1638"] {
            let g = ui.painter().layout_no_wrap(
                t.to_owned(),
                egui::FontId::proportional(crate::widgets::LABEL_PX),
                egui::Color32::BLACK,
            );
            widest = widest.max(g.size().x);
        }
    });
    out.textures_delta.clear();
    // The box also has a margin on each side.
    assert!(crate::ribbon::SIZE_BOX_W >= widest + 8.0, "{} vs {widest}", crate::ribbon::SIZE_BOX_W);
}

#[test]
fn the_size_box_in_the_ribbon_is_at_least_that_wide() {
    let mut ui = text_ui();
    select(&mut ui, A);
    // The text area plus the chevron (16) and the border (2) make up the whole box.
    let w = size_box(&ui).rect().width() + 18.0;
    assert!(w >= crate::ribbon::SIZE_BOX_W - 0.5, "box is {w} wide");
}

#[test]
fn typing_11_and_enter_in_the_size_box_sets_11_pt() {
    let mut ui = text_ui();
    select(&mut ui, A);
    type_size(&mut ui, "11");
    assert_eq!(run0(&ui, A).size_pt, 11.0);
    assert_eq!(size_box(&ui).value().as_deref(), Some("11"));
}

#[test]
fn typing_a_font_name_and_enter_applies_it_and_junk_reverts() {
    let mut ui = text_ui();
    ui.wait_for_fonts();
    select(&mut ui, A);
    let target = ui.app().fonts.families.iter().find(|f| **f != "Calibri").expect("family").clone();
    name_box(&ui).click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text(&target.to_lowercase());
    ui.key(NONE, Key::Enter);
    assert_eq!(run0(&ui, A).font, target);
    let before = run0(&ui, A).font.clone();
    name_box(&ui).click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text("zzzz no such font");
    ui.key(NONE, Key::Enter);
    assert_eq!(run0(&ui, A).font, before);
    assert_eq!(name_box(&ui).value().as_deref(), Some(before.as_str()));
}

#[test]
fn the_size_chevron_opens_the_list() {
    let mut ui = text_ui();
    select(&mut ui, A);
    let combos: Vec<_> = ui.h.get_all_by_role(Role::ComboBox).collect();
    assert_eq!(combos.len(), 2);
    assert!(ui.h.query_by_label("14").is_none(), "closed at first");
    combos[1].click();
    ui.settle();
    assert!(ui.h.query_by_label("14").is_some(), "the size list is open");
}
