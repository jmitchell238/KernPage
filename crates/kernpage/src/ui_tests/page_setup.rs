//! Page Design > Page Setup: the Margins, Orientation and Size galleries, the Page Setup
//! dialog and the Layout Guides dialog (docs/ui/PAGE-SETUP.md).
//!
//! Gallery rows are painted (no AccessKit names), so they are found through the test seam
//! `page_setup::probe`; the dialogs are driven through their AccessKit labels.

use super::support::*;
use eframe::egui::{accesskit::Role, Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Command, Margins, Rect};

const NONE: Modifiers = Modifiers::NONE;
const IN: i64 = 914_400;
const MM: i64 = 36_000;

/// Letter page, margins 0.5" on every side, one rectangle A at (1, 1) size 2 x 1.
fn ui_with_shape() -> TestUi {
    TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.0)]))
}

fn mm(v: i64) -> i64 {
    v * MM
}

/// The gallery's "Page Setup..." command button (the ribbon launcher carries the same name).
fn page_setup_row<'a>(ui: &'a TestUi) -> egui_kittest::Node<'a> {
    let rows: Vec<_> = ui
        .h
        .get_all_by_label("Page Setup...")
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .collect();
    assert_eq!(rows.len(), 1, "one command button named Page Setup...");
    rows.into_iter().next().unwrap()
}

fn near(a: i64, b: i64) -> bool {
    (a - b).abs() <= 1500 // the presets are rounded to 1/1000 in
}

fn titles(ui: &TestUi) -> Vec<String> {
    ui.gallery_titles()
}

fn current_titles(ui: &TestUi) -> Vec<String> {
    ui.gallery_rows()
        .into_iter()
        .filter(|r| r.current)
        .map(|r| r.title)
        .collect()
}

// ---- Margins gallery -------------------------------------------------------------------------

#[test]
fn margins_gallery_lists_the_four_presets_in_order_with_their_values() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Margins");
    assert_eq!(titles(&ui), ["Wide", "Moderate", "Narrow", "None"]);
    let details: Vec<String> = ui.gallery_rows().into_iter().map(|r| r.detail).collect();
    assert_eq!(
        details,
        [
            "Top 1 Bottom 1 Left 1 Right 1",
            "Top 0.5 Bottom 0.5 Left 0.5 Right 0.5",
            "Top 0.25 Bottom 0.25 Left 0.25 Right 0.25",
            "Top 0 Bottom 0 Left 0 Right 0",
        ]
    );
    // The command row under the separator.
    ui.h.get_by_label("Custom Margins...");
}

#[test]
fn a_new_page_has_moderate_margins_highlighted_and_no_custom_row() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Margins");
    assert_eq!(current_titles(&ui), ["Moderate"]);
    assert!(!titles(&ui).contains(&"Custom Margins".to_owned()));
}

#[test]
fn each_margin_preset_applies_to_all_four_margins_as_one_undo_step() {
    for (name, inches) in [("Wide", 1.0), ("Moderate", 0.5), ("Narrow", 0.25), ("None", 0.0)] {
        let mut ui = ui_with_shape();
        let shape_before = ui.rect_of(1);
        // Start from a different margin so Moderate is a real change.
        ui.app().exec(Command::SetMargins {
            margins: Margins::all(in_to_emu(0.1)),
        });
        let base = ui.undo_steps();
        ui.open_gallery("Margins");
        ui.click_gallery_row(name);
        assert_eq!(ui.margins(), Margins::all(in_to_emu(inches)), "{name}");
        assert!(ui.gallery_rows().is_empty(), "{name}: the gallery closes after a pick");
        assert_eq!(ui.undo_steps(), base + 1, "{name}: one undo step");
        assert_eq!(ui.rect_of(1), shape_before, "{name}: nothing on the page moves");
        ui.undo();
        assert_eq!(ui.margins(), Margins::all(in_to_emu(0.1)), "{name}: undo restores");
    }
}

#[test]
fn the_current_margin_preset_is_highlighted_after_picking_it() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Margins");
    ui.click_gallery_row("Wide");
    ui.click_ribbon("Margins");
    assert_eq!(current_titles(&ui), ["Wide"]);
}

#[test]
fn custom_margins_add_a_fifth_highlighted_row() {
    let mut ui = ui_with_shape();
    let m = Margins {
        left: in_to_emu(0.75),
        right: in_to_emu(0.5),
        top: in_to_emu(0.5),
        bottom: in_to_emu(1.0),
    };
    ui.app().exec(Command::SetMargins { margins: m });
    ui.open_gallery("Margins");
    assert_eq!(titles(&ui), ["Wide", "Moderate", "Narrow", "None", "Custom Margins"]);
    assert_eq!(current_titles(&ui), ["Custom Margins"], "only the custom row is highlighted");
    // Picking a preset replaces the custom margins and the extra row goes away.
    ui.click_gallery_row("Narrow");
    ui.click_ribbon("Margins");
    assert_eq!(titles(&ui).len(), 4);
    assert_eq!(current_titles(&ui), ["Narrow"]);
}

#[test]
fn custom_margins_command_row_opens_the_layout_guides_dialog() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Margins");
    ui.h.get_by_label("Custom Margins...").click();
    ui.settle();
    assert!(ui.app().page_dialog.is_some());
    ui.h.get_by_label("Layout Guides");
    assert!(ui.gallery_rows().is_empty(), "the gallery closed");
}

// ---- Orientation gallery -----------------------------------------------------------------------

#[test]
fn orientation_gallery_has_portrait_and_landscape_with_the_current_one_highlighted() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Orientation");
    assert_eq!(titles(&ui), ["Portrait", "Landscape"]);
    assert_eq!(current_titles(&ui), ["Portrait"]);
}

#[test]
fn choosing_landscape_swaps_width_and_height_keeps_margins_and_is_one_undo_step() {
    let mut ui = ui_with_shape();
    let m = ui.margins();
    ui.open_gallery("Orientation");
    ui.click_gallery_row("Landscape");
    assert_eq!(ui.page_size(0), (11 * IN, in_to_emu(8.5)));
    assert_eq!(ui.margins(), m, "margins stay 0.5\" on every side");
    assert!(ui.gallery_rows().is_empty(), "the gallery closes");
    assert_eq!(ui.undo_steps(), 1);
    ui.click_ribbon("Orientation");
    assert_eq!(current_titles(&ui), ["Landscape"]);
    // And back again.
    ui.click_gallery_row("Portrait");
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    assert_eq!(ui.undo_steps(), 2);
    ui.undo();
    assert_eq!(ui.page_size(0), (11 * IN, in_to_emu(8.5)), "undo goes back to landscape");
}

#[test]
fn choosing_the_orientation_the_page_already_has_does_nothing() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Orientation");
    ui.click_gallery_row("Portrait");
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    assert!(!ui.can_undo(), "no undo step for a no-op");
    // Landscape page, choose Landscape.
    ui.click_ribbon("Orientation");
    ui.click_gallery_row("Landscape");
    let steps = ui.undo_steps();
    ui.click_ribbon("Orientation");
    ui.click_gallery_row("Landscape");
    assert_eq!(ui.undo_steps(), steps);
    assert_eq!(ui.page_size(0), (11 * IN, in_to_emu(8.5)));
}

#[test]
fn orientation_applies_to_every_page_and_keeps_shapes_relative_to_the_page_centre() {
    let mut ui = ui_with_shape();
    ui.app().insert_page_after(0);
    ui.app().go_page(0);
    ui.settle();
    ui.open_gallery("Orientation");
    ui.click_gallery_row("Landscape");
    assert_eq!(ui.page_size(1), (11 * IN, in_to_emu(8.5)), "page 2 turned too");
    // Letter portrait -> landscape: +1.25 in across, -1.25 in down (the page centre is fixed).
    assert_eq!(ui.rect_of(1), Rect::new(in_to_emu(2.25), in_to_emu(-0.25), 2 * IN, IN));
    ui.undo();
    assert_eq!(ui.rect_of(1), Rect::new(IN, IN, 2 * IN, IN));
    assert_eq!(ui.page_size(1), (in_to_emu(8.5), 11 * IN));
}

// ---- Size gallery --------------------------------------------------------------------------------

const PORTRAIT_SIZES: [(&str, &str); 9] = [
    ("A3 (Portrait)", "11.693 x 16.535\""),
    ("A4 (Portrait)", "8.268 x 11.693\""),
    ("A5 (Portrait)", "5.827 x 8.268\""),
    ("B4 (Portrait)", "9.843 x 13.898\""),
    ("B5 (Portrait)", "6.929 x 9.843\""),
    ("Executive (Portrait)", "7.25 x 10.5\""),
    ("Legal (Portrait)", "8.5 x 14\""),
    ("Letter (Portrait)", "8.5 x 11\""),
    ("Tabloid (Portrait)", "11 x 17\""),
];

#[test]
fn size_gallery_lists_the_nine_built_in_sizes_in_portrait_with_letter_highlighted() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Size");
    let got: Vec<(String, String)> = ui
        .gallery_rows()
        .into_iter()
        .map(|r| (r.title, r.detail))
        .collect();
    let want: Vec<(String, String)> = PORTRAIT_SIZES
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    assert_eq!(got, want);
    assert_eq!(current_titles(&ui), ["Letter (Portrait)"]);
}

#[test]
fn size_gallery_lists_landscape_sizes_for_a_landscape_page() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Orientation");
    ui.click_gallery_row("Landscape");
    ui.click_ribbon("Size");
    let rows = ui.gallery_rows();
    assert_eq!(rows.len(), 9);
    assert_eq!(rows[0].title, "A3 (Landscape)");
    assert_eq!(rows[0].detail, "16.535 x 11.693\"");
    assert_eq!(rows[6].title, "Legal (Landscape)");
    assert_eq!(rows[6].detail, "14 x 8.5\"");
    assert_eq!(rows[8].title, "Tabloid (Landscape)");
    assert_eq!(rows[8].detail, "17 x 11\"");
    assert_eq!(current_titles(&ui), ["Letter (Landscape)"]);
}

#[test]
fn picking_a_size_sets_the_page_size_keeps_the_margins_and_is_one_undo_step() {
    let mut ui = ui_with_shape();
    let margins = ui.margins();
    ui.open_gallery("Size");
    ui.click_gallery_row("A4 (Portrait)");
    let (w, h) = ui.page_size(0);
    assert!(near(w, mm(210)) && near(h, mm(297)), "A4 is 210 x 297 mm, got {w} x {h}");
    assert_eq!(ui.margins(), margins);
    assert!(ui.gallery_rows().is_empty(), "the gallery closes");
    assert_eq!(ui.undo_steps(), 1);
    ui.click_ribbon("Size");
    assert_eq!(current_titles(&ui), ["A4 (Portrait)"]);
    ui.click_gallery_row("Legal (Portrait)");
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 14 * IN));
    assert_eq!(ui.undo_steps(), 2);
    ui.undo();
    assert!(near(ui.page_size(0).0, mm(210)));
}

#[test]
fn the_ninth_size_is_reached_by_scrolling_the_list() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Size");
    let first = ui.gallery_rows()[0].rect.center();
    ui.wheel_at(first, eframe::egui::Vec2::new(0.0, -300.0), NONE);
    ui.click_gallery_row("Tabloid (Portrait)");
    assert_eq!(ui.page_size(0), (11 * IN, 17 * IN));
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn a_size_change_applies_to_every_page_and_shapes_keep_their_place_relative_to_the_centre() {
    let mut ui = ui_with_shape();
    ui.app().insert_page_after(0);
    ui.app().go_page(0);
    ui.settle();
    ui.open_gallery("Size");
    ui.click_gallery_row("Legal (Portrait)"); // 8.5 x 14: taller by 3 in
    assert_eq!(ui.page_size(1), (in_to_emu(8.5), 14 * IN));
    // Same width, 3 in taller: the shape moves down 1.5 in, sideways not at all.
    assert_eq!(ui.rect_of(1), Rect::new(IN, in_to_emu(2.5), 2 * IN, IN));
    ui.undo();
    assert_eq!(ui.rect_of(1), Rect::new(IN, IN, 2 * IN, IN));
    ui.open_gallery("Size");
    ui.click_gallery_row("A5 (Portrait)"); // smaller on both axes
    let (w, h) = ui.page_size(0);
    let r = ui.rect_of(1);
    let (dx, dy) = ((w - in_to_emu(8.5)) / 2, (h - 11 * IN) / 2);
    assert!(near(r.x, IN + dx) && near(r.y, IN + dy), "moved by half the change: {r:?} {dx} {dy}");
}

#[test]
fn a_custom_size_shows_a_highlighted_current_row_first() {
    let mut ui = ui_with_shape();
    ui.app().exec(Command::SetPageSize {
        width_emu: 5 * IN,
        height_emu: 7 * IN,
    });
    ui.open_gallery("Size");
    let rows = ui.gallery_rows();
    assert_eq!(rows.len(), 10);
    assert_eq!((rows[0].title.as_str(), rows[0].detail.as_str()), ("Current", "5 x 7\""));
    assert_eq!(current_titles(&ui), ["Current"], "no preset is highlighted");
    assert_eq!(rows[1].title, "A3 (Portrait)");
}

#[test]
fn page_setup_is_enabled_and_the_other_two_size_commands_are_greyed() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Size");
    assert!(!page_setup_row(&ui).accesskit_node().is_disabled());
    for name in ["Create New Page Size...", "More Preset Page Sizes..."] {
        if ui.gallery_rows().is_empty() {
            ui.click_ribbon("Size"); // a click on a greyed row may close the drop-down
        }
        let node = ui.h.get_by_label(name);
        assert!(node.accesskit_node().is_disabled(), "{name} is greyed");
        node.click();
        ui.settle();
        assert!(ui.app().page_dialog.is_none(), "{name} opens nothing");
        assert!(!ui.can_undo());
    }
}

#[test]
fn a_gallery_closes_on_escape_and_on_an_outside_click_without_changing_anything() {
    for name in ["Margins", "Orientation", "Size"] {
        let mut ui = ui_with_shape();
        ui.open_gallery(name);
        ui.key(NONE, Key::Escape);
        assert!(ui.gallery_rows().is_empty(), "{name}: Escape closes it");
        ui.click_ribbon(name);
        assert!(!ui.gallery_rows().is_empty(), "{name}: opens again");
        ui.click_at(ui.at_in(4.0, 6.0)); // somewhere on the page
        assert!(ui.gallery_rows().is_empty(), "{name}: an outside click closes it");
        assert!(!ui.can_undo(), "{name}: nothing was applied");
    }
}

// ---- Page Setup dialog --------------------------------------------------------------------------

/// The text of the six boxes of the open Page Setup dialog (model state).
fn fields(ui: &TestUi) -> Vec<String> {
    let d = ui.h.state().page_dialog.as_ref().expect("dialog open");
    vec![
        d.width.text.clone(),
        d.height.text.clone(),
        d.top.text.clone(),
        d.left.text.clone(),
        d.bottom.text.clone(),
        d.right.text.clone(),
    ]
}

/// Replace the text of the `n`th text box of the dialog by clicking it and typing.
fn type_into(ui: &mut TestUi, n: usize, text: &str) {
    let node = ui.h.get_all_by_role(Role::TextInput).nth(n).expect("text box");
    node.click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text(text);
}

fn open_page_setup_by_launcher(ui: &mut TestUi) {
    ui.click_tab("Page Design");
    ui.click_ribbon("Page Setup...");
    assert!(ui.app().page_dialog.is_some(), "the launcher opened the dialog");
}

fn ok(ui: &mut TestUi) {
    ui.h.get_by_label("     OK     ").click();
    ui.settle();
}

fn cancel(ui: &mut TestUi) {
    ui.h.get_by_label("  Cancel  ").click();
    ui.settle();
}

#[test]
fn the_launcher_arrow_opens_the_page_setup_dialog_prefilled() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    ui.h.get_by_label("Page Setup");
    ui.h.get_by_label("Margin guides");
    ui.h.get_by_label("Layout type");
    assert_eq!(fields(&ui), ["8.5\"", "11\"", "0.5\"", "0.5\"", "0.5\"", "0.5\""]);
    ui.h.get_by_label("Page size: 8.5 x 11\"");
    ui.h.get_by_label("Paper size: 8.5 x 11\"");
    ui.h.get_by_label("1 columns of 1");
}

#[test]
fn size_gallery_page_setup_row_opens_the_same_dialog() {
    let mut ui = ui_with_shape();
    ui.open_gallery("Size");
    page_setup_row(&ui).click();
    ui.settle();
    ui.h.get_by_label("Page Setup");
    assert_eq!(fields(&ui)[0], "8.5\"");
    assert!(ui.gallery_rows().is_empty(), "the gallery closed");
}

#[test]
fn the_dialog_starts_with_the_width_box_selected_so_typing_replaces_it() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    ui.type_text("5");
    assert_eq!(fields(&ui)[0], "5");
}

#[test]
fn typed_units_set_a_custom_size_and_ok_is_one_undo_step() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "5.5");
    type_into(&mut ui, 1, "21.59 cm");
    type_into(&mut ui, 3, "36 pt"); // Left margin: 0.5 in
    ui.h.get_by_label("Page size: 5.5 x 8.5\""); // the preview follows the boxes
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_none(), "OK closes the dialog");
    assert_eq!(ui.page_size(0), (in_to_emu(5.5), in_to_emu(8.5)));
    assert_eq!(ui.margins(), Margins::all(IN / 2));
    assert_eq!(ui.undo_steps(), 1, "size and margins are one step");
    ui.undo();
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
}

#[test]
fn page_setup_ok_without_changes_keeps_the_exact_values_and_the_shapes_in_place() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    let before = ui.rect_of(1);
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    assert_eq!(ui.rect_of(1), before);
}

#[test]
fn page_setup_ok_moves_shapes_by_half_the_size_change() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "10.5");
    type_into(&mut ui, 1, "13");
    ok(&mut ui);
    assert_eq!(ui.rect_of(1), Rect::new(2 * IN, 2 * IN, 2 * IN, IN));
}

#[test]
fn invalid_input_shows_a_message_and_keeps_the_dialog_open() {
    let cases: [(usize, &str, &str); 4] = [
        (0, "wide", "is not a length"),
        (1, "60", "between 1\" and 49\""),
        (2, "-1", "A margin cannot be negative."),
        (5, "9", "The left and right margins must leave room on the page."),
    ];
    for (field, text, message) in cases {
        let mut ui = ui_with_shape();
        open_page_setup_by_launcher(&mut ui);
        type_into(&mut ui, field, text);
        ui.settle();
        ui.h.get_by_label_contains(message);
        ok(&mut ui);
        assert!(ui.app().page_dialog.is_some(), "{text:?}: OK keeps the dialog open");
        assert!(!ui.can_undo(), "{text:?}: nothing applied");
        assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    }
    // Top and bottom that do not leave room.
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 2, "6");
    type_into(&mut ui, 4, "5");
    ui.h.get_by_label_contains("The top and bottom margins must leave room on the page.");
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_some());
    // Fixing the boxes clears the message and OK now works.
    type_into(&mut ui, 2, "1");
    type_into(&mut ui, 4, "1");
    assert!(ui.h.query_by_label_contains("must leave room").is_none());
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(ui.margins().top, IN);
}

#[test]
fn cancel_discards_the_dialog() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "5");
    cancel(&mut ui);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    assert!(!ui.can_undo());
}

#[test]
fn escape_discards_the_page_setup_dialog() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "5");
    ui.key(NONE, Key::Escape);
    assert!(ui.app().page_dialog.is_none(), "Esc closes the dialog");
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN));
    assert!(!ui.can_undo());
}

#[test]
fn escape_goes_to_the_dialog_not_to_the_canvas_selection() {
    let mut ui = ui_with_shape();
    ui.click_at(ui.center_of(1));
    assert_eq!(ui.selected(), vec![1]);
    open_page_setup_by_launcher(&mut ui);
    ui.key(NONE, Key::Escape);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(ui.selected(), vec![1], "Esc went to the dialog, not to the canvas");
}

#[test]
fn enter_is_ok() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "6");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().page_dialog.is_none(), "Enter closes the dialog");
    assert_eq!(ui.page_size(0).0, 6 * IN);
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn enter_with_an_invalid_box_keeps_the_dialog_open() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    type_into(&mut ui, 0, "wide");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().page_dialog.is_some());
    assert!(!ui.can_undo());
}

#[test]
fn tab_moves_through_width_height_top_left_bottom_right() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    // Width has focus and its text is selected: typing replaces it; then Tab to the next box.
    for (i, v) in ["9", "8", "7", "6", "5", "4"].iter().enumerate() {
        if i > 0 {
            ui.key(NONE, Key::Tab);
            ui.key(CTRL, Key::A);
        }
        ui.type_text(v);
    }
    assert_eq!(fields(&ui), ["9", "8", "7", "6", "5", "4"]);
}

#[test]
fn the_dialog_is_modal_the_canvas_does_not_react_behind_it() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    ui.click_at(ui.center_of(1));
    assert!(ui.selected().is_empty(), "a click on the canvas is blocked");
    assert!(ui.app().page_dialog.is_some());
}

#[test]
fn the_layout_type_box_offers_one_page_per_sheet_and_greys_the_rest() {
    let mut ui = ui_with_shape();
    open_page_setup_by_launcher(&mut ui);
    ui.h.get_all_by_role(Role::ComboBox).next().expect("layout type box").click();
    ui.settle();
    for name in [
        "Booklet",
        "Email",
        "Envelope",
        "Folded card",
        "Multiple pages per sheet",
        "Web page",
    ] {
        assert!(ui.h.get_by_label(name).accesskit_node().is_disabled(), "{name} is greyed");
    }
}

// ---- Layout Guides dialog ------------------------------------------------------------------------

fn open_layout_guides(ui: &mut TestUi) {
    ui.click_tab("Page Design");
    ui.click_ribbon("Margins");
    ui.h.get_by_label("Custom Margins...").click();
    ui.settle();
    assert!(ui.app().page_dialog.is_some());
}

#[test]
fn layout_guides_dialog_shows_the_margin_guides_tab_and_the_unavailable_ones() {
    let mut ui = ui_with_shape();
    open_layout_guides(&mut ui);
    ui.h.get_by_label("Layout Guides");
    ui.h.get_by_label("Margin Guides");
    for name in ["Grid Guides", "Baseline Guides"] {
        assert!(
            ui.h.get_by_label(name).accesskit_node().is_disabled(),
            "{name} is not available yet"
        );
    }
    assert!(ui.h.get_all_by_label("Master Pages").count() >= 1); // the dialog heading (and the Page Design button behind it)
    assert!(ui.h.get_by_label("Two-page master").accesskit_node().is_disabled());
    // No page size boxes here, only the four margin boxes.
    assert!(ui.h.query_by_label("Width:").is_none());
    ui.h.get_by_label("Left:");
    ui.h.get_by_label("Right:");
    ui.h.get_by_label("Top:");
    ui.h.get_by_label("Bottom:");
    assert_eq!(ui.h.get_all_by_role(Role::TextInput).count(), 4);
}

#[test]
fn layout_guides_box_order_is_left_right_top_bottom_and_ok_sets_only_the_margins() {
    let mut ui = ui_with_shape();
    let before = ui.rect_of(1);
    open_layout_guides(&mut ui);
    // Boxes in order: Left, Right, Top, Bottom (Page Setup uses Top, Left, Bottom, Right).
    for (i, v) in ["1.5", "0.25", "0.75", "2"].iter().enumerate() {
        type_into(&mut ui, i, v);
    }
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(
        ui.margins(),
        Margins {
            left: in_to_emu(1.5),
            right: in_to_emu(0.25),
            top: in_to_emu(0.75),
            bottom: 2 * IN
        }
    );
    assert_eq!(ui.page_size(0), (in_to_emu(8.5), 11 * IN), "the page size is untouched");
    assert_eq!(ui.rect_of(1), before, "no object moves");
    assert_eq!(ui.undo_steps(), 1);
    ui.click_ribbon("Margins");
    assert_eq!(current_titles(&ui), ["Custom Margins"]);
}

#[test]
fn layout_guides_cancel_and_escape_discard() {
    for use_escape in [false, true] {
        let mut ui = ui_with_shape();
        open_layout_guides(&mut ui);
        type_into(&mut ui, 0, "2");
        if use_escape {
            ui.key(NONE, Key::Escape);
        } else {
            cancel(&mut ui);
        }
        assert!(ui.app().page_dialog.is_none(), "closed (escape: {use_escape})");
        assert_eq!(ui.margins(), Margins::all(IN / 2));
        assert!(!ui.can_undo());
    }
}

#[test]
fn layout_guides_enter_is_ok_and_validation_keeps_it_open() {
    let mut ui = ui_with_shape();
    open_layout_guides(&mut ui);
    type_into(&mut ui, 2, "-1");
    ui.h.get_by_label_contains("A margin cannot be negative.");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().page_dialog.is_some(), "invalid: stays open");
    type_into(&mut ui, 2, "1");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().page_dialog.is_none());
    assert_eq!(ui.margins().top, IN);
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn layout_guides_margins_must_leave_room_on_the_page() {
    let mut ui = ui_with_shape();
    open_layout_guides(&mut ui);
    type_into(&mut ui, 0, "5");
    type_into(&mut ui, 1, "4");
    ui.h.get_by_label_contains("The left and right margins must leave room on the page.");
    ok(&mut ui);
    assert!(ui.app().page_dialog.is_some());
    assert_eq!(ui.margins(), Margins::all(IN / 2));
}
