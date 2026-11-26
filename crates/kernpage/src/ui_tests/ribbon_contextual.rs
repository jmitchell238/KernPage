//! The contextual ribbon tabs: Shape Format, Text Box, Picture Format, Table Design and Layout.
//! Each working control runs its command as one undo step; every greyed control is disabled and
//! does nothing. Buttons are found through the ribbon seam; the Height / Width boxes, the shape
//! gallery cells and the colour swatches through the seam labels `size:`, `shape_gal:` and
//! `swatch:` (see `backstage::seen`).

use super::support::*;
use crate::app::{Tab, Tool};
use eframe::egui::{accesskit::Role, Key, Modifiers, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::model::PictureFormat;
use kernpage_core::{
    Paragraph, Rect, Rgb, Shape, ShapeKind, Stroke, TableCell, WrapType,
};
use std::path::PathBuf;
use std::sync::Arc;

const A: u64 = 1;
const B: u64 = 2;
const C: u64 = 3;
const NONE: Modifiers = Modifiers::NONE;

fn text_box(x: f64, y: f64) -> Shape {
    Shape::new(
        Rect::new(in_to_emu(x), in_to_emu(y), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox { paragraphs: vec![Paragraph::plain("Hello world")] },
    )
}

fn picture() -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0), in_to_emu(2.0)),
        ShapeKind::Picture { data: Arc::new(vec![]), format: PictureFormat::Unknown, file: None },
    )
}

fn table() -> Shape {
    let cells = (0..4).map(|i| TableCell::plain(&format!("c{i}"))).collect();
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(7.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::Table { rows: 2, cols: 2, col_widths_emu: vec![], row_heights_emu: vec![], row_set_emu: vec![], cells },
    )
}

/// Text box A (1, 1) 4 x 2 in, rectangle B (1, 4) 2 x 1.5 in, rectangle C (5, 4) 2 x 1.5 in.
fn shapes_ui() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        text_box(1.0, 1.0),
        rect_shape(1.0, 4.0, 2.0, 1.5),
        rect_shape(5.0, 4.0, 2.0, 1.5),
    ]))
}

/// Picture A at (1, 4), table B at (1, 7).
fn pic_table_ui() -> TestUi {
    let mut p = picture();
    p.rect = Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(2.0));
    TestUi::with_doc(letter_doc(vec![p, table()]))
}

fn select(ui: &mut TestUi, id: u64) {
    ui.click_at(ui.center_of(id));
    assert_eq!(ui.selected(), vec![id], "shape {id} selected");
}

fn on(ui: &TestUi, name: &str) -> bool {
    ui.ribbon(name).enabled
}

fn ctx_tab_names(ui: &TestUi) -> Vec<String> {
    ui.tab_names().into_iter().skip(7).collect()
}

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/assets").join(name)
}

fn greyed(ui: &TestUi) -> Vec<Seen> {
    ui.greyed()
}

fn ribbon_body(ui: &TestUi) -> Vec<Seen> {
    ui.ribbon_body()
}

fn click_all_greyed_and_expect_nothing(ui: &mut TestUi) {
    ui.click_all_greyed_and_expect_nothing();
}

/// Open the menu of a split button's arrow, whether it is drawn large (arrow under the label)
/// or small (arrow right of the label).
fn click_split_arrow(ui: &mut TestUi, name: &str) {
    let b = ui.ribbon(name);
    if b.rect.height() > 40.0 {
        ui.click_at(b.rect.center_bottom() - Vec2::new(0.0, 6.0));
    } else {
        ui.click_ribbon_arrow(name);
    }
}

fn menu_disabled(ui: &TestUi, label: &str) -> bool {
    ui.h.get_by_label(label).accesskit_node().is_disabled()
}

fn click_menu(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

// ---- the tabs and their selections -----------------------------------------------------------------------

#[test]
fn each_kind_of_object_gets_its_own_contextual_tabs() {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box(1.0, 1.0), rect_shape(1.0, 4.0, 2.0, 1.0), {
        let mut p = picture();
        p.rect = Rect::new(in_to_emu(5.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(2.0));
        p
    }, {
        let mut t = table();
        t.rect = Rect::new(in_to_emu(4.0), in_to_emu(6.0), in_to_emu(4.0), in_to_emu(2.0));
        t
    }]));
    for (id, tabs, markers) in [
        (1, vec!["Shape Format", "Text Box"], vec!["Edit Text", "Columns"]),
        (2, vec!["Shape Format"], vec!["Shape Fill"]),
        (3, vec!["Picture Format"], vec!["Change Picture"]),
        (4, vec!["Table Design", "Layout"], vec!["Line Weight", "Insert Above"]),
    ] {
        select(&mut ui, id);
        assert_eq!(ctx_tab_names(&ui), tabs, "shape {id}");
        for (tab, marker) in tabs.iter().zip(markers) {
            ui.click_tab(tab);
            assert!(ui.ribbon(&format!("tab:{tab}")).selected, "{tab} is selected");
            assert!(ui.has_ribbon(marker), "{tab} shows {marker}");
        }
    }
}

#[test]
fn a_contextual_tab_stays_while_the_selection_still_has_it() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    select(&mut ui, B);
    assert_eq!(ui.tab(), Tab::ShapeFormat, "a rectangle has Shape Format too");
    ui.click_tab("Shape Format");
    ui.click_at(ui.center_of(C));
    assert_eq!(ui.tab(), Tab::ShapeFormat);
    // A text box opened on its own tab loses it when a rectangle is picked.
    select(&mut ui, A);
    ui.click_tab("Text Box");
    select(&mut ui, B);
    assert_eq!(ui.tab(), Tab::Home, "the Text Box tab is gone, so back to Home");
}

#[test]
fn picking_the_contextual_tab_does_not_change_the_selection_or_the_document() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    let rev = ui.revision();
    for t in ["Shape Format", "Text Box", "Home"] {
        ui.click_tab(t);
    }
    assert_eq!(ui.selected(), vec![A]);
    assert_eq!(ui.revision(), rev);
    assert!(!ui.can_undo());
}

// ---- Shape Format -----------------------------------------------------------------------------------------

#[test]
fn shape_format_has_working_and_greyed_controls_as_planned() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    for n in ["Shape Fill", "Shape Outline", "Wrap Text", "Bring Forward", "Send Backward", "Align", "Rotate"] {
        assert!(on(&ui, n), "{n} is live for a rectangle");
    }
    for n in ["Edit Shape", "Change Shape", "Edit Text", "Shape Effects", "Measurement"] {
        assert!(!on(&ui, n), "{n} is greyed for a rectangle");
    }
    assert!(!on(&ui, "Group") && !on(&ui, "Ungroup"), "one object cannot be grouped, a non-group cannot be ungrouped");
}

#[test]
fn shape_format_greyed_controls_do_nothing() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    let off = greyed(&ui);
    // Edit Shape, Change Shape, Edit Text, the style gallery, Shape Effects, Group, Ungroup,
    // Measurement, and 17 of the 21 Insert Shapes gallery cells.
    assert_eq!(off.iter().filter(|b| b.label.starts_with("shape_gal:")).count(), 17);
    assert_eq!(off.iter().filter(|b| b.label == "gallery").count(), 1, "the Shape Styles gallery");
    for n in ["Edit Shape", "Change Shape", "Edit Text", "Shape Effects", "Group", "Ungroup", "Measurement"] {
        assert!(off.iter().any(|b| b.label == n), "{n} is in the greyed set");
    }
    click_all_greyed_and_expect_nothing(&mut ui);
    assert!(!ui.can_undo());
}

#[test]
fn edit_text_starts_editing_a_selected_text_box_only() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    assert!(!on(&ui, "Edit Text"));
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    assert!(on(&ui, "Edit Text"));
    ui.click_ribbon("Edit Text");
    assert!(ui.app().edit.is_some(), "the text box is now in edit mode");
    assert_eq!(ui.edit_text(), "Hello world");
}

#[test]
fn the_shape_gallery_cells_arm_the_draw_tools() {
    for (cell, tool) in [
        ("shape_gal:0,0", Tool::TextBox),
        ("shape_gal:0,1", Tool::Line),
        ("shape_gal:0,3", Tool::Rectangle),
        ("shape_gal:0,4", Tool::Oval),
    ] {
        let mut ui = shapes_ui();
        select(&mut ui, B);
        ui.click_tab("Shape Format");
        assert!(on(&ui, cell), "{cell} is live");
        ui.click_ribbon(cell);
        assert_eq!(ui.app().tool, tool, "{cell}");
        assert!(ui.selected().is_empty(), "arming a tool drops the selection");
        assert_eq!(ui.tab(), Tab::Home, "and with it the contextual tab");
    }
}

#[test]
fn a_gallery_tool_draws_the_shape_on_the_next_drag_as_one_undo_step() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon("shape_gal:0,4");
    let before = ui.shapes().len();
    ui.drag(ui.at_in(3.0, 7.0), ui.at_in(5.0, 9.0));
    assert_eq!(ui.shapes().len(), before + 1);
    assert!(matches!(ui.shapes().last().unwrap().kind, ShapeKind::Ellipse));
    assert_eq!(ui.app().tool, Tool::Select, "the tool resets after drawing");
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn escape_disarms_a_gallery_tool_and_the_cell_arms_it_again() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon("shape_gal:0,3");
    assert_eq!(ui.app().tool, Tool::Rectangle);
    // Arming dropped the selection (and the tab); Esc goes back to the Select tool.
    ui.key(NONE, Key::Escape);
    assert_eq!(ui.app().tool, Tool::Select);
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon("shape_gal:0,3");
    assert_eq!(ui.app().tool, Tool::Rectangle, "the cell arms the tool again");
}

// ---- Height / Width ---------------------------------------------------------------------------------------

fn box_values(ui: &TestUi) -> Vec<String> {
    ui.h.get_all_by_role(Role::TextInput).map(|n| n.accesskit_node().value().unwrap_or_default()).collect()
}

/// Click into the Height / Width box, replace its text and press Enter.
fn type_in_box(ui: &mut TestUi, which: &str, text: &str) {
    let f = ui.ribbon(&format!("size:{which}")).rect;
    ui.click_at(f.min + Vec2::new(10.0, 11.0));
    ui.key(CTRL, Key::A);
    ui.type_text(text);
    ui.key(NONE, Key::Enter);
}

fn size_in(ui: &TestUi, id: u64) -> (f64, f64) {
    let r = ui.rect_of(id);
    (r.w as f64 / 914_400.0, r.h as f64 / 914_400.0)
}

#[test]
fn the_height_and_width_boxes_show_the_selected_objects_size() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    assert_eq!(box_values(&ui), ["1.5\"", "2\""], "Height then Width, in inches");
    ui.click_tab("Home");
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    assert_eq!(box_values(&ui), ["2\"", "4\""], "follows the selection");
}

#[test]
fn typing_a_width_and_pressing_enter_resizes_in_one_undo_step() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    let before = ui.rect_of(B);
    type_in_box(&mut ui, "Width", "3");
    assert_eq!(size_in(&ui, B), (3.0, 1.5), "width changed, height kept");
    assert_eq!((ui.rect_of(B).x, ui.rect_of(B).y), (before.x, before.y), "the top left stays");
    assert_eq!(box_values(&ui), ["1.5\"", "3\""], "the box shows the new size");
    assert_eq!(ui.undo_steps(), 1);
    ui.undo();
    assert_eq!(ui.rect_of(B), before, "Ctrl+Z restores the size");
}

#[test]
fn typing_a_height_with_a_unit_is_accepted() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    type_in_box(&mut ui, "Height", "2.5\"");
    assert_eq!(size_in(&ui, B), (2.0, 2.5));
    type_in_box(&mut ui, "Height", "3 in");
    assert_eq!(size_in(&ui, B), (2.0, 3.0));
    assert_eq!(ui.undo_steps(), 2);
}

#[test]
fn an_invalid_size_is_ignored_and_the_box_shows_the_real_size_again() {
    for bad in ["abc", "0", "-2", "400", ""] {
        let mut ui = shapes_ui();
        select(&mut ui, B);
        ui.click_tab("Shape Format");
        type_in_box(&mut ui, "Width", bad);
        assert_eq!(size_in(&ui, B), (2.0, 1.5), "{bad:?} changes nothing");
        assert!(!ui.can_undo(), "{bad:?}: no undo step");
        assert_eq!(box_values(&ui), ["1.5\"", "2\""], "{bad:?}: the box is back to the real size");
    }
}

#[test]
fn the_spinner_arrows_step_the_size_by_a_tenth_of_an_inch() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon("size:Width:up");
    assert!((size_in(&ui, B).0 - 2.1).abs() < 1e-6, "{:?}", size_in(&ui, B));
    ui.click_ribbon("size:Width:down");
    ui.click_ribbon("size:Width:down");
    assert!((size_in(&ui, B).0 - 1.9).abs() < 1e-6);
    ui.click_ribbon("size:Height:up");
    assert!((size_in(&ui, B).1 - 1.6).abs() < 1e-6);
    ui.click_ribbon("size:Height:down");
    assert!((size_in(&ui, B).1 - 1.5).abs() < 1e-6);
    assert_eq!(ui.undo_steps(), 5, "each click is one undo step");
}

#[test]
fn the_spinner_never_goes_below_the_smallest_size() {
    let mut ui = TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 0.05, 0.05)]));
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    for _ in 0..3 {
        ui.click_ribbon("size:Width:down");
    }
    assert!(size_in(&ui, A).0 >= 0.01 - 1e-9, "{:?}", size_in(&ui, A));
    assert!(size_in(&ui, A).0 > 0.0);
}

#[test]
fn the_size_boxes_are_greyed_for_a_multiple_selection_and_do_nothing() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_mods(ui.center_of(C), SHIFT);
    assert_eq!(ui.selected().len(), 2);
    ui.click_tab("Shape Format");
    for n in ["size:Height", "size:Width", "size:Height:up", "size:Width:down"] {
        assert!(!on(&ui, n), "{n}");
    }
    assert_eq!(box_values(&ui), ["", ""], "no size is shown for several objects");
    let before = (ui.rect_of(B), ui.rect_of(C));
    ui.click_ribbon("size:Width:up");
    ui.click_ribbon("size:Height:down");
    assert_eq!((ui.rect_of(B), ui.rect_of(C)), before);
    assert!(!ui.can_undo());
}

#[test]
fn the_size_boxes_resize_pictures_and_tables_too() {
    let mut ui = pic_table_ui();
    select(&mut ui, A);
    ui.click_tab("Picture Format");
    type_in_box(&mut ui, "Width", "3");
    assert_eq!(size_in(&ui, A).0, 3.0);
    select(&mut ui, B);
    ui.click_tab("Layout");
    assert_eq!(box_values(&ui), ["2\"", "4\""]);
    type_in_box(&mut ui, "Height", "3");
    assert_eq!(size_in(&ui, B), (4.0, 3.0));
    assert_eq!(ui.undo_steps(), 2);
}

// ---- Shape Fill / Shape Outline ---------------------------------------------------------------------------

#[test]
fn the_shape_fill_button_applies_the_last_colour_as_one_undo_step() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    let last = ui.app().last_fill_color;
    ui.click_ribbon("Shape Fill");
    assert_eq!(ui.shape(B).fill, Some(last));
    assert_eq!(ui.undo_steps(), 1);
    assert_eq!(ui.shape(C).fill, rect_shape(0.0, 0.0, 1.0, 1.0).fill, "other shapes keep their fill");
}

#[test]
fn the_shape_fill_palette_applies_a_swatch_and_it_becomes_the_last_colour() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon_arrow("Shape Fill");
    assert!(ui.has_ribbon("swatch:0"), "the arrow opens the palette");
    assert_eq!(ui.revision(), ui.revision());
    let want = crate::widgets::palette()[13];
    ui.click_ribbon("swatch:13");
    assert_eq!(ui.shape(B).fill, Some(Rgb(want[0], want[1], want[2])));
    assert_eq!(ui.app().last_fill_color, Rgb(want[0], want[1], want[2]));
    assert!(!ui.has_ribbon("swatch:0"), "picking closes the palette");
    assert_eq!(ui.undo_steps(), 1);
    // The main button now repeats it on another shape.
    select(&mut ui, C);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Shape Fill");
    assert_eq!(ui.shape(C).fill, Some(Rgb(want[0], want[1], want[2])));
}

#[test]
fn no_fill_removes_the_fill() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    assert!(ui.shape(B).fill.is_some());
    ui.click_ribbon_arrow("Shape Fill");
    click_menu(&mut ui, "No Fill");
    assert_eq!(ui.shape(B).fill, None);
    assert_eq!(ui.undo_steps(), 1);
    ui.undo();
    assert!(ui.shape(B).fill.is_some(), "undo brings the fill back");
}

#[test]
fn shape_fill_applies_to_a_text_box_too() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    ui.click_ribbon_arrow("Shape Fill");
    ui.click_ribbon("swatch:3");
    let want = crate::widgets::palette()[3];
    assert_eq!(ui.shape(A).fill, Some(Rgb(want[0], want[1], want[2])));
}

#[test]
fn the_shape_outline_button_adds_an_outline_in_the_last_colour() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    assert_eq!(ui.shape(B).line, None);
    let last = ui.app().last_outline_color;
    ui.click_ribbon("Shape Outline");
    assert_eq!(ui.shape(B).line, Some(Stroke { color: last, width_emu: 9_525 }));
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn the_outline_palette_picks_a_colour_and_no_outline_removes_it() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon_arrow("Shape Outline");
    ui.click_ribbon("swatch:5");
    let want = crate::widgets::palette()[5];
    assert_eq!(ui.shape(B).line.map(|l| l.color), Some(Rgb(want[0], want[1], want[2])));
    assert_eq!(ui.app().last_outline_color, Rgb(want[0], want[1], want[2]));
    ui.click_ribbon_arrow("Shape Outline");
    click_menu(&mut ui, "No Outline");
    assert_eq!(ui.shape(B).line, None);
    assert_eq!(ui.undo_steps(), 2);
}

#[test]
fn the_outline_weight_menu_sets_the_line_width_and_keeps_the_colour() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Shape Outline");
    let colour = ui.shape(B).line.unwrap().color;
    for (text, pt) in [("3 pt", 3.0_f64), ("\u{bd} pt", 0.5), ("4\u{bd} pt", 4.5)] {
        ui.click_ribbon_arrow("Shape Outline");
        ui.h.get_by_label("Weight").click();
        ui.settle();
        click_menu(&mut ui, text);
        let l = ui.shape(B).line.expect("outline");
        assert_eq!(l.width_emu, (pt * 12_700.0).round() as i64, "{text}");
        assert_eq!(l.color, colour, "{text}: the colour is kept");
    }
    assert_eq!(ui.undo_steps(), 4);
}

#[test]
fn a_weight_turns_a_missing_outline_on_in_black() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_tab("Shape Format");
    ui.click_ribbon_arrow("Shape Outline");
    ui.h.get_by_label("Weight").click();
    ui.settle();
    click_menu(&mut ui, "6 pt");
    assert_eq!(ui.shape(B).line, Some(Stroke { color: Rgb::BLACK, width_emu: 76_200 }));
}

#[test]
fn fill_and_outline_apply_to_every_selected_shape_in_one_undo_step() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_mods(ui.center_of(C), SHIFT);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Shape Outline");
    assert!(ui.shape(B).line.is_some() && ui.shape(C).line.is_some());
    ui.click_ribbon_arrow("Shape Fill");
    click_menu(&mut ui, "No Fill");
    assert!(ui.shape(B).fill.is_none() && ui.shape(C).fill.is_none());
    assert_eq!(ui.undo_steps(), 2);
}

// ---- Arrange group on the contextual tabs ----------------------------------------------------------------------

fn order(ui: &TestUi) -> Vec<u64> {
    ui.shapes().iter().map(|s| s.id).collect()
}

#[test]
fn the_shape_format_arrange_group_orders_wraps_rotates_and_aligns() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Bring Forward");
    assert_eq!(order(&ui), [B, A, C], "one step forward");
    ui.click_ribbon("Send Backward");
    assert_eq!(order(&ui), [A, B, C]);
    click_split_arrow(&mut ui, "Bring Forward");
    click_menu(&mut ui, "Bring to Front");
    assert_eq!(order(&ui), [B, C, A]);
    click_split_arrow(&mut ui, "Send Backward");
    click_menu(&mut ui, "Send to Back");
    assert_eq!(order(&ui), [A, B, C]);
    ui.click_ribbon("Wrap Text");
    click_menu(&mut ui, "Tight");
    assert_eq!(ui.shape(A).wrap, WrapType::Tight);
    ui.click_ribbon("Rotate");
    click_menu(&mut ui, "Rotate Right 90\u{b0}");
    assert_eq!(ui.shape(A).rotation_deg, 90.0);
    assert_eq!(ui.undo_steps(), 6, "each command is one undo step");
}

#[test]
fn the_shape_format_group_and_ungroup_buttons_work_on_two_shapes() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_mods(ui.center_of(C), SHIFT);
    ui.click_tab("Shape Format");
    assert!(on(&ui, "Group") && !on(&ui, "Ungroup"));
    let n = ui.shapes().len();
    ui.click_ribbon("Group");
    assert_eq!(ui.shapes().len(), n + 1, "a group shape was added");
    assert!(ui.shapes().iter().any(|s| matches!(s.kind, ShapeKind::Group)));
    ui.click_tab("Shape Format");
    assert!(!on(&ui, "Group") && on(&ui, "Ungroup"));
    ui.click_ribbon("Ungroup");
    assert_eq!(ui.shapes().len(), n, "ungrouped");
    assert_eq!(ui.undo_steps(), 2);
}

#[test]
fn the_align_menu_on_the_shape_format_tab_needs_two_objects() {
    let mut ui = shapes_ui();
    select(&mut ui, B);
    ui.click_mods(ui.center_of(C), SHIFT);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Align");
    click_menu(&mut ui, "Align Top");
    assert_eq!(ui.rect_of(B).y, ui.rect_of(C).y);
    select(&mut ui, A);
    ui.click_tab("Shape Format");
    ui.click_ribbon("Align");
    assert!(menu_disabled(&ui, "Align Left"), "one object has nothing to align with");
}

// ---- Text Box tab ---------------------------------------------------------------------------------------

#[test]
fn the_text_box_tab_font_group_formats_the_whole_box() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Text Box");
    ui.click_ribbon("Bold (Ctrl+B)");
    let runs = |ui: &TestUi| match &ui.shape(A).kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.iter().flat_map(|p| p.runs.clone()).collect::<Vec<_>>(),
        _ => unreachable!(),
    };
    assert!(runs(&ui).iter().all(|r| r.bold), "bold");
    ui.click_ribbon("Italic (Ctrl+I)");
    ui.click_ribbon("Underline (Ctrl+U)");
    assert!(runs(&ui).iter().all(|r| r.italic && r.underline));
    ui.click_ribbon("Grow Font");
    assert!(runs(&ui).iter().all(|r| r.size_pt > 12.0), "grown");
    ui.click_ribbon("Font Color");
    assert!(runs(&ui).iter().all(|r| r.color != Rgb::BLACK));
    ui.click_ribbon("Clear Formatting");
    assert!(runs(&ui).iter().all(|r| !r.bold && !r.italic && !r.underline && r.color == Rgb::BLACK));
    assert_eq!(ui.undo_steps(), 6);
}

#[test]
fn the_text_box_columns_menu_sets_the_column_count() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Text Box");
    assert!(on(&ui, "Columns"));
    for (item, n) in [("Two", 2), ("Three", 3), ("One", 1)] {
        ui.click_ribbon("Columns");
        click_menu(&mut ui, item);
        assert_eq!(ui.shape(A).columns, n, "{item}");
    }
    assert_eq!(ui.undo_steps(), 3);
}

#[test]
fn text_box_greyed_controls_do_nothing() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Text Box");
    for n in [
        "Text Fit", "Text Direction", "Hyphen- ation", "Margins", "Break", "Next",
        "Previous", "Text Fill", "Text Outline", "Text Effects", "Drop Cap",
    ] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    // A box that is not linked can start a link (the button does nothing yet).
    assert!(on(&ui, "Create Link"));
    assert_eq!(greyed(&ui).iter().filter(|b| b.label == "gallery").count(), 1, "the text styles gallery");
    let off = greyed(&ui);
    // Change Case, Character Spacing and the nine vertical-anchor buttons.
    let unnamed = off.iter().filter(|b| b.label.is_empty() && b.tip.is_empty()).count();
    assert_eq!(unnamed, 11, "{off:?}");
    click_all_greyed_and_expect_nothing(&mut ui);
    assert!(!ui.can_undo());
}

#[test]
fn text_box_live_controls_are_enabled() {
    let mut ui = shapes_ui();
    select(&mut ui, A);
    ui.click_tab("Text Box");
    for n in [
        "Bold (Ctrl+B)", "Italic (Ctrl+I)", "Underline (Ctrl+U)", "Grow Font", "Shrink Font",
        "Font Color", "Clear Formatting", "Columns", "Subscript (Ctrl+=)", "Superscript (Ctrl+Shift+=)",
    ] {
        assert!(on(&ui, n), "{n}");
    }
}

// ---- Picture Format -------------------------------------------------------------------------------------

#[test]
fn the_picture_format_pictures_button_inserts_a_picture() {
    let mut ui = pic_table_ui();
    select(&mut ui, A);
    ui.click_tab("Picture Format");
    assert!(on(&ui, "Pictures..."));
    ui.dialogs.script_open(Some(asset("red4x3.png")));
    ui.click_ribbon("Pictures...");
    assert_eq!(ui.dialogs.opens().len(), 1);
    assert_eq!(ui.shapes().len(), 3, "a picture was added");
    assert_eq!(ui.selected().len(), 1, "the new picture is selected");
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn cancelling_pictures_on_the_picture_format_tab_adds_nothing() {
    let mut ui = pic_table_ui();
    select(&mut ui, A);
    ui.click_tab("Picture Format");
    ui.dialogs.script_open(None);
    ui.click_ribbon("Pictures...");
    assert_eq!(ui.shapes().len(), 2);
    assert!(!ui.can_undo());
}

#[test]
fn picture_format_greyed_controls_do_nothing() {
    let mut ui = pic_table_ui();
    select(&mut ui, A);
    ui.click_tab("Picture Format");
    for n in [
        "Corrections", "Recolor", "Compress Pictures...", "Reset Picture", "Swap", "Picture Border",
        "Picture Effects", "Caption", "Arrange Thumbnails", "Crop", "Fit", "Fill", "Clear Crop",
        "Group", "Ungroup",
    ] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    for n in ["Pictures...", "Change Picture", "Wrap Text", "Bring Forward", "Send Backward", "Align", "Rotate"] {
        assert!(on(&ui, n), "{n} is live");
    }
    click_all_greyed_and_expect_nothing(&mut ui);
    assert!(!ui.can_undo());
    assert!(ui.app().tool == Tool::Select);
}

#[test]
fn the_picture_format_arrange_group_works_on_a_picture() {
    let mut ui = pic_table_ui();
    select(&mut ui, A);
    ui.click_tab("Picture Format");
    ui.click_ribbon("Bring Forward");
    assert_eq!(order(&ui), [B, A]);
    ui.click_ribbon("Send Backward");
    assert_eq!(order(&ui), [A, B]);
    ui.click_ribbon("Wrap Text");
    click_menu(&mut ui, "Square");
    assert_eq!(ui.shape(A).wrap, WrapType::Square);
    ui.click_ribbon("Rotate");
    click_menu(&mut ui, "Rotate Left 90\u{b0}");
    assert_eq!(ui.shape(A).rotation_deg, 270.0);
    assert_eq!(ui.undo_steps(), 4);
}

// ---- Table Design / Layout ------------------------------------------------------------------------------

#[test]
fn table_design_is_entirely_greyed_and_inert() {
    let mut ui = pic_table_ui();
    select(&mut ui, B);
    ui.click_tab("Table Design");
    for n in [
        "Fill", "Line Weight", "Line Color", "Borders", "Text Fill", "Text Outline", "Text Effects",
        "Drop Cap", "Number Style", "Ligatures",
    ] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    let body = ribbon_body(&ui);
    assert_eq!(body.iter().filter(|b| b.label == "gallery").count(), 2, "table formats and text styles galleries");
    assert!(!body.is_empty() && body.iter().all(|b| !b.enabled), "nothing on Table Design is live: {body:?}");
    click_all_greyed_and_expect_nothing(&mut ui);
    assert!(!ui.can_undo());
}

#[test]
fn table_layout_has_only_rows_columns_arrange_and_size_controls_live() {
    let mut ui = pic_table_ui();
    select(&mut ui, B);
    ui.click_tab("Layout");
    for n in [
        "Select", "View Gridlines", "Insert Above", "Insert Below", "Insert Left", "Insert Right",
        "Merge Cells", "Split Cells...", "Diagonals", "Text Direction", "Hyphen- ation", "Cell Margins",
        "Group", "Ungroup",
    ] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    for n in ["Delete", "Wrap Text", "Bring Forward", "Send Backward", "Align", "Rotate", "size:Height", "size:Width"] {
        assert!(on(&ui, n), "{n} is live");
    }
    let unnamed = greyed(&ui).iter().filter(|b| b.label.is_empty() && b.tip.is_empty()).count();
    assert_eq!(unnamed, 9, "the nine vertical-anchor buttons");
    let grow = ui.h.get_by_label("Grow to Fit Text");
    assert!(grow.accesskit_node().is_disabled(), "Grow to Fit Text is greyed");
    click_all_greyed_and_expect_nothing(&mut ui);
    assert!(!ui.can_undo(), "the greyed controls changed nothing");
}

#[test]
fn the_table_layout_arrange_group_works_on_a_table() {
    let mut ui = pic_table_ui();
    select(&mut ui, B);
    ui.click_tab("Layout");
    ui.click_ribbon("Send Backward");
    assert_eq!(order(&ui), [B, A]);
    ui.click_ribbon("Wrap Text");
    click_menu(&mut ui, "Square");
    assert_eq!(ui.shape(B).wrap, WrapType::Square);
    ui.click_ribbon("Rotate");
    click_menu(&mut ui, "Rotate Right 90\u{b0}");
    assert_eq!(ui.shape(B).rotation_deg, 90.0);
    assert_eq!(ui.undo_steps(), 3);
}

#[test]
fn a_table_has_no_fill_or_outline_command_and_its_tabs_stay_while_editing_a_cell() {
    let mut ui = pic_table_ui();
    select(&mut ui, B);
    // Editing a cell keeps the table selected and its tabs available.
    ui.double_click_at(ui.at_in(2.0, 7.5));
    assert!(ui.app().edit.is_some(), "a cell is being edited");
    assert_eq!(ctx_tab_names(&ui), ["Table Design", "Layout"]);
    ui.click_tab("Layout");
    assert_eq!(ui.tab(), Tab::Layout);
    assert!(on(&ui, "Insert Above"), "a cell is being edited, so Insert Above is live");
}
