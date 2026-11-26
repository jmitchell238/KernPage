//! Bullets and numbering (docs/ui/BULLETS.md, docs/testing/UI-TEST-PLAN.md section 7b): the
//! Bullets and Numbering galleries and split buttons, the Bullets and Numbering dialog with
//! its Bullet Character dialog, Decrease / Increase Indent. Everything is driven with real
//! pointer clicks and key presses; the tiles and character grids are painted shapes, so their
//! rectangles come from the `lists_ui::probe` seam.

use super::support::*;
use crate::lists_ui::{probe, ListTab};
use eframe::egui::{accesskit::Role, Key, Modifiers, Pos2, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::text::list_labels;
use kernpage_core::{ListInfo, ListType, Paragraph, Rect, Shape, ShapeKind, TableCell};

const A: u64 = 1;
const NONE: Modifiers = Modifiers::NONE;

fn text_box(paragraphs: &[&str]) -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox {
            paragraphs: paragraphs.iter().map(|t| Paragraph::plain(t)).collect(),
        },
    )
}

/// Box A with "One", "Two", "Three", selected as an object (so the whole story is formatted).
fn selected() -> TestUi {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box(&["One", "Two", "Three"])]));
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A]);
    ui
}

/// Box A with the caret editing; the text is "One\nTwo\nThree" (One 0..3, Two 4..7, Three 8..13).
fn editing() -> TestUi {
    let mut ui = TestUi::with_doc(letter_doc(vec![text_box(&["One", "Two", "Three"])]));
    ui.double_click_at(ui.at_in(1.14, 1.12));
    assert!(ui.app().edit.is_some(), "edit mode started");
    ui.settle();
    ui.set_selection_range(0, 0);
    ui
}

fn list_at(ui: &TestUi, i: usize) -> Option<ListInfo> {
    ui.paragraphs(A)[i].list.clone()
}

fn lists(ui: &TestUi) -> Vec<Option<ListInfo>> {
    ui.paragraphs(A).iter().map(|p| p.list.clone()).collect()
}

/// The label each paragraph draws: its number text, or its bullet as the canvas draws it
/// (`list_labels` holds the raw code, which the canvas maps to a Unicode character).
fn labels(ui: &TestUi) -> Vec<Option<String>> {
    let ps = ui.paragraphs(A);
    list_labels(&ps)
        .into_iter()
        .zip(&ps)
        .map(|(l, p)| match (&p.list, l) {
            (Some(i), Some(_)) if i.kind.is_bullet() => {
                Some(pub_format::bullet_display_char(i.bullet_char, &i.bullet_font).to_string())
            }
            (_, l) => l,
        })
        .collect()
}

fn s(x: &str) -> Option<String> {
    Some(x.to_owned())
}

fn indents(ui: &TestUi) -> Vec<(f32, f32)> {
    ui.paragraphs(A)
        .iter()
        .map(|p| (p.indents.left_pt, p.indents.first_line_pt))
        .collect()
}

// ---- tiles and dialog controls ----------------------------------------------------------------

fn open_bullets(ui: &mut TestUi) {
    ui.click_ribbon_arrow("Bullets");
    assert!(probe::find("bullets-gallery", "None").is_some(), "the Bullets gallery is open");
}

fn open_numbering(ui: &mut TestUi) {
    ui.click_ribbon_arrow("Numbering");
    assert!(probe::find("numbering-gallery", "None").is_some(), "the Numbering gallery is open");
}

fn click_tile(ui: &mut TestUi, area: &str, name: &str) {
    let t = probe::find(area, name).unwrap_or_else(|| panic!("no tile {name:?} in {area}"));
    ui.click_at(t.rect.center());
}

fn pick_bullet(ui: &mut TestUi, name: &str) {
    open_bullets(ui);
    click_tile(ui, "bullets-gallery", name);
}

fn pick_number(ui: &mut TestUi, name: &str) {
    open_numbering(ui);
    click_tile(ui, "numbering-gallery", name);
}

fn tile_names(area: &str) -> Vec<String> {
    probe::all().into_iter().filter(|i| i.area == area).map(|i| i.name).collect()
}

fn highlighted(area: &str) -> Vec<String> {
    probe::all()
        .into_iter()
        .filter(|i| i.area == area && i.selected)
        .map(|i| i.name)
        .collect()
}

fn click_label(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn dialog_open(ui: &mut TestUi) -> bool {
    ui.app().list_dialog.is_some()
}

/// Open the dialog from the footer of the Bullets (or Numbering) gallery.
fn open_dialog(ui: &mut TestUi, numbering: bool) {
    if numbering {
        open_numbering(ui);
    } else {
        open_bullets(ui);
    }
    click_label(ui, "Bullets and Numbering...");
    assert!(dialog_open(ui), "the Bullets and Numbering dialog is open");
}

/// A tab of the dialog (the ribbon has buttons with the same names; the dialog is drawn last).
fn tab_button(ui: &mut TestUi, name: &str) {
    ui.h.get_all_by_role_and_label(Role::Button, name).last().expect("a tab").click();
    ui.settle();
}

fn input_disabled(ui: &TestUi, label: &str) -> bool {
    ui.h.get_by_role_and_label(Role::TextInput, label).accesskit_node().is_disabled()
}

fn fill(ui: &mut TestUi, label: &str, text: &str) {
    ui.h.get_by_role_and_label(Role::TextInput, label).click();
    ui.settle();
    ui.key(CTRL, Key::A);
    ui.type_text(text);
}

fn dialog_bullet(ui: &mut TestUi, name: &str) {
    let b = probe::find("dialog-bullet", name).unwrap_or_else(|| panic!("no dialog button {name:?}"));
    ui.click_at(b.rect.center());
}

/// The OK of the dialog on top (the Bullet Character dialog is drawn after the main one).
fn ok(ui: &mut TestUi) {
    ui.h.get_all_by_label("OK").last().expect("an OK button").click();
    ui.settle();
}

fn cancel(ui: &mut TestUi) {
    ui.h.get_all_by_label("Cancel").last().expect("a Cancel button").click();
    ui.settle();
}

/// Open the dialog's combo box `n` places from the last one drawn (the ribbon's own combo
/// boxes come first): on the Numbering tab 1 = Format and 0 = Separator, in the Bullet
/// Character dialog 0 = Font. Then pick an item.
fn open_combo(ui: &mut TestUi, n: usize) {
    let all: Vec<_> = ui.h.get_all_by_role(Role::ComboBox).collect();
    all[all.len() - 1 - n].click();
    ui.settle();
}

fn pick_combo(ui: &mut TestUi, n: usize, item: &str) {
    open_combo(ui, n);
    ui.settle();
    ui.h.get_by_label(item).click();
    ui.settle();
}

// ---- the Bullets gallery -----------------------------------------------------------------------

#[test]
fn the_bullets_gallery_has_publishers_header_tiles_and_footer() {
    let mut ui = selected();
    open_bullets(&mut ui);
    assert_eq!(
        tile_names("bullets-gallery"),
        [
            "None",
            "Small Bullets",
            "Large Bullets",
            "Filled Diamond Bullets",
            "Arrow Bullets",
            "Open Diamond Bullets",
            "Asterisk Bullets"
        ]
    );
    assert!(ui.h.query_by_label("Bullet Styles").is_some(), "the section header");
    assert!(ui.h.query_by_label("Bullets and Numbering...").is_some(), "the footer command");
    // 90 x 90 tiles, three to a row.
    let t = |n: &str| probe::find("bullets-gallery", n).unwrap().rect;
    assert_eq!(t("None").size(), Vec2::splat(90.0));
    assert_eq!(t("None").min.y, t("Small Bullets").min.y);
    assert_eq!(t("None").min.y, t("Large Bullets").min.y);
    assert!(t("Filled Diamond Bullets").min.y > t("None").max.y - 1.0, "the fourth tile starts row two");
    assert_eq!(t("Filled Diamond Bullets").min.x, t("None").min.x);
    assert_eq!(highlighted("bullets-gallery"), ["None"], "no list: None has the orange border");
}

#[test]
fn each_bullet_tile_puts_its_bullet_on_every_paragraph_of_the_box() {
    // (tile, Symbol code, size, label drawn)
    for (name, code, size, label) in [
        ("Small Bullets", 183, 10.0, "\u{2022}"),
        ("Large Bullets", 183, 12.0, "\u{2022}"),
        ("Filled Diamond Bullets", 168, 10.0, "\u{2666}"),
        ("Arrow Bullets", 222, 10.0, "\u{21D2}"),
        ("Open Diamond Bullets", 224, 10.0, "\u{25CA}"),
        ("Asterisk Bullets", 42, 10.0, "*"),
    ] {
        let mut ui = selected();
        pick_bullet(&mut ui, name);
        for l in lists(&ui) {
            let l = l.unwrap_or_else(|| panic!("{name}: a paragraph has no list"));
            assert_eq!((l.kind, l.bullet_char, l.bullet_font.as_str()), (ListType::Bullet, code, "Symbol"), "{name}");
            assert_eq!(l.bullet_size_pt, Some(size), "{name}");
            assert_eq!(l.list_indent_pt, 18.0);
        }
        assert_eq!(labels(&ui), [s(label), s(label), s(label)], "{name}: the label that is drawn");
        assert_eq!(indents(&ui), [(18.0, -18.0); 3], "{name}: hanging indent");
        assert_eq!(ui.undo_steps(), 1, "{name}: one undo step");
        assert!(probe::find("bullets-gallery", name).is_none(), "{name}: the gallery closed");
    }
}

#[test]
fn the_tile_of_the_current_bullet_gets_the_orange_border_and_none_clears() {
    let mut ui = selected();
    pick_bullet(&mut ui, "Large Bullets");
    open_bullets(&mut ui);
    assert_eq!(highlighted("bullets-gallery"), ["Large Bullets"], "the 12 pt dot, not the 10 pt one");
    click_tile(&mut ui, "bullets-gallery", "Arrow Bullets");
    open_bullets(&mut ui);
    assert_eq!(highlighted("bullets-gallery"), ["Arrow Bullets"]);
    click_tile(&mut ui, "bullets-gallery", "None");
    assert!(lists(&ui).iter().all(Option::is_none), "None takes the paragraphs out of the list");
    assert_eq!(indents(&ui), [(0.0, 0.0); 3], "and its indents go");
    assert_eq!(labels(&ui), [None, None, None]);
    open_bullets(&mut ui);
    assert_eq!(highlighted("bullets-gallery"), ["None"]);
    assert_eq!(ui.undo_steps(), 3, "one undo step per choice");
}

#[test]
fn a_numbered_selection_highlights_no_bullet_tile() {
    let mut ui = selected();
    pick_number(&mut ui, "1. 2. 3.");
    open_bullets(&mut ui);
    assert_eq!(highlighted("bullets-gallery"), Vec::<String>::new());
}

#[test]
fn the_bullets_button_itself_toggles_the_default_bullet() {
    let mut ui = selected();
    assert!(!ui.ribbon("Bullets").selected);
    ui.click_ribbon("Bullets");
    assert_eq!(list_at(&ui, 0), Some(ListInfo::bullet()), "Symbol U+00B7 at 10 pt");
    assert!(ui.ribbon("Bullets").selected, "shown pressed while the selection is bulleted");
    ui.click_ribbon("Bullets");
    assert!(lists(&ui).iter().all(Option::is_none), "a second click takes the bullets off");
    assert_eq!(ui.undo_steps(), 2);
}

// ---- the Numbering gallery -----------------------------------------------------------------------

#[test]
fn the_numbering_gallery_has_none_and_eight_tiles_named_by_their_sample() {
    let mut ui = selected();
    open_numbering(&mut ui);
    assert_eq!(
        tile_names("numbering-gallery"),
        [
            "None",
            "1. 2. 3.",
            "1) 2) 3)",
            "I. II. III.",
            "A. B. C.",
            "a. b. c.",
            "a) b) c)",
            "i. ii. iii.",
            "i) ii) iii)"
        ]
    );
    assert!(ui.h.query_by_label("Bullets and Numbering...").is_some());
    assert_eq!(highlighted("numbering-gallery"), ["None"]);
}

#[test]
fn each_numbering_tile_numbers_the_paragraphs_and_is_highlighted_afterwards() {
    for (name, kind, sep, first) in [
        ("1. 2. 3.", ListType::Arabic, 2, ["1.", "2.", "3."]),
        ("1) 2) 3)", ListType::Arabic, 0, ["1)", "2)", "3)"]),
        ("I. II. III.", ListType::UpperRoman, 2, ["I.", "II.", "III."]),
        ("A. B. C.", ListType::UpperLetter, 2, ["A.", "B.", "C."]),
        ("a. b. c.", ListType::LowerLetter, 2, ["a.", "b.", "c."]),
        ("a) b) c)", ListType::LowerLetter, 0, ["a)", "b)", "c)"]),
        ("i. ii. iii.", ListType::LowerRoman, 2, ["i.", "ii.", "iii."]),
        ("i) ii) iii)", ListType::LowerRoman, 0, ["i)", "ii)", "iii)"]),
    ] {
        let mut ui = selected();
        pick_number(&mut ui, name);
        for l in lists(&ui) {
            let l = l.unwrap_or_else(|| panic!("{name}: a paragraph has no list"));
            assert_eq!((l.kind, l.separator, l.start, l.list_indent_pt), (kind, sep, 1, 18.0), "{name}");
        }
        assert_eq!(labels(&ui), first.map(s), "{name}: the numbers that are drawn");
        assert_eq!(indents(&ui), [(18.0, -18.0); 3]);
        open_numbering(&mut ui);
        assert_eq!(highlighted("numbering-gallery"), [name], "{name} is the current tile");
        assert_eq!(ui.undo_steps(), 1, "{name}: one undo step");
    }
}

#[test]
fn numbering_none_clears_and_the_numbering_button_toggles() {
    let mut ui = selected();
    ui.click_ribbon("Numbering");
    assert_eq!(labels(&ui), [s("1."), s("2."), s("3.")]);
    assert!(ui.ribbon("Numbering").selected);
    pick_number(&mut ui, "None");
    assert!(lists(&ui).iter().all(Option::is_none));
    ui.click_ribbon("Numbering");
    ui.click_ribbon("Numbering");
    assert!(lists(&ui).iter().all(Option::is_none), "the button turned it off again");
}

#[test]
fn choosing_a_tile_keeps_the_list_indent_of_the_paragraphs() {
    let mut ui = selected();
    ui.click_ribbon("Increase Indent");
    assert_eq!(indents(&ui), [(18.0, 0.0); 3], "plain paragraphs: left indent only");
    pick_bullet(&mut ui, "Small Bullets");
    ui.click_ribbon("Increase Indent");
    assert_eq!(list_at(&ui, 0).unwrap().list_indent_pt, 36.0);
    pick_number(&mut ui, "I. II. III.");
    assert_eq!(list_at(&ui, 0).unwrap().list_indent_pt, 36.0, "kept across bullets to numbering");
}

// ---- the dialog ---------------------------------------------------------------------------------------

#[test]
fn the_footer_of_each_gallery_opens_the_dialog_on_its_tab() {
    let mut ui = selected();
    open_dialog(&mut ui, false);
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().tab, ListTab::Bullets);
    assert!(ui.h.query_by_label("Bullets and Numbering").is_some(), "the title");
    cancel(&mut ui);
    assert!(!dialog_open(&mut ui));
    open_dialog(&mut ui, true);
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().tab, ListTab::Numbering);
    ui.key(NONE, Key::Escape);
    assert!(!dialog_open(&mut ui), "Escape closes it");
    assert_eq!(lists(&ui), [None, None, None], "nothing changed");
    assert_eq!(ui.undo_steps(), 0);
}

#[test]
fn the_bullets_tab_has_seven_buttons_and_no_bullet_greys_size_character_and_indent() {
    let mut ui = selected();
    open_dialog(&mut ui, false);
    let names: Vec<String> = probe::all().into_iter().filter(|i| i.area == "dialog-bullet").map(|i| i.name).collect();
    assert_eq!(
        names,
        ["No Bullet", "Bullet type 1", "Bullet type 2", "Bullet type 3", "Bullet type 4", "Bullet type 5", "Bullet type 6"]
    );
    assert_eq!(highlighted("dialog-bullet"), ["No Bullet"], "the blank button is chosen (thick border)");
    assert!(input_disabled(&ui, "Size:"));
    assert!(input_disabled(&ui, "Indent list by:"));
    assert!(ui.h.get_by_label("Character...").accesskit_node().is_disabled());
    assert!(ui.h.query_by_label("Sample").is_some());
    // Choosing a bullet turns them on and shows 10pt and 0.25".
    dialog_bullet(&mut ui, "Bullet type 2");
    assert!(!input_disabled(&ui, "Size:"));
    assert!(!input_disabled(&ui, "Indent list by:"));
    assert!(!ui.h.get_by_label("Character...").accesskit_node().is_disabled());
    let d = ui.app().list_dialog.as_ref().unwrap();
    assert_eq!((d.size.as_str(), d.bullet_indent.as_str()), ("12pt", "0.25\""), "the large dot's own size");
    assert_eq!(highlighted("dialog-bullet"), ["Bullet type 2"]);
    dialog_bullet(&mut ui, "No Bullet");
    assert!(input_disabled(&ui, "Size:"), "back to no bullet");
}

#[test]
fn the_dialog_applies_nothing_until_ok_and_ok_is_one_undo_step() {
    let mut ui = selected();
    open_dialog(&mut ui, false);
    dialog_bullet(&mut ui, "Bullet type 4");
    fill(&mut ui, "Size:", "14");
    fill(&mut ui, "Indent list by:", "0.5");
    assert_eq!(lists(&ui), [None, None, None], "still nothing while the dialog is open");
    ok(&mut ui);
    assert!(!dialog_open(&mut ui));
    let l = list_at(&ui, 2).unwrap();
    assert_eq!((l.bullet_char, l.bullet_font.as_str(), l.bullet_size_pt, l.list_indent_pt), (222, "Symbol", Some(14.0), 36.0));
    assert_eq!(indents(&ui), [(36.0, -36.0); 3]);
    assert_eq!(labels(&ui), [s("\u{21D2}"), s("\u{21D2}"), s("\u{21D2}")]);
    assert_eq!(ui.undo_steps(), 1);
    ui.undo();
    assert_eq!(lists(&ui), [None, None, None], "Ctrl+Z undoes the whole dialog result");
}

#[test]
fn cancel_leaves_the_document_alone() {
    let mut ui = selected();
    open_dialog(&mut ui, true);
    tab_button(&mut ui, "Bullets");
    dialog_bullet(&mut ui, "Bullet type 3");
    cancel(&mut ui);
    assert_eq!(lists(&ui), [None, None, None]);
    assert_eq!(ui.undo_steps(), 0);
}

#[test]
fn the_dialog_opens_pre_filled_from_a_bulleted_selection() {
    let mut ui = selected();
    pick_bullet(&mut ui, "Open Diamond Bullets");
    open_dialog(&mut ui, true); // even from the Numbering menu: it shows the list the text has
    let d = ui.app().list_dialog.as_ref().unwrap();
    assert_eq!(d.tab, ListTab::Bullets);
    assert_eq!(d.bullet, Some((224, "Symbol".to_owned())));
    assert_eq!((d.size.as_str(), d.bullet_indent.as_str()), ("10pt", "0.25\""));
    assert_eq!(highlighted("dialog-bullet"), ["Bullet type 5"]);
    assert!(!input_disabled(&ui, "Size:"));
}

#[test]
fn the_numbering_tab_lists_the_formats_and_separators_and_opens_pre_filled() {
    let mut ui = selected();
    pick_number(&mut ui, "i) ii) iii)");
    ui.click_ribbon("Increase Indent");
    open_dialog(&mut ui, false);
    let d = ui.app().list_dialog.as_ref().unwrap();
    assert_eq!(d.tab, ListTab::Numbering, "a numbered selection opens on Numbering");
    assert_eq!((d.format, d.separator), (ListType::LowerRoman, 0));
    assert_eq!((d.start.as_str(), d.num_indent.as_str()), ("1", "0.50\""));
    // The Format list, in Publisher's order.
    open_combo(&mut ui, 1);
    for f in [
        "1, 2, 3, \u{2026}",
        "I, II, III, \u{2026}",
        "A, B, C, \u{2026}",
        "a, b, c, \u{2026}",
        "1st, 2nd, 3rd \u{2026}",
    ] {
        assert!(ui.h.query_by_label(f).is_some(), "format {f}");
    }
    ui.key(NONE, Key::Escape);
}

#[test]
fn changing_format_separator_start_and_indent_applies_on_ok() {
    let mut ui = selected();
    open_dialog(&mut ui, true);
    let d = ui.app().list_dialog.as_ref().unwrap();
    assert_eq!((d.format, d.separator, d.start.as_str(), d.num_indent.as_str()), (ListType::Arabic, 2, "1", "0.25\""), "defaults: 1, 2, 3, ...  .  1  0.25\"");
    pick_combo(&mut ui, 1, "A, B, C, \u{2026}");
    pick_combo(&mut ui, 0, ")");
    fill(&mut ui, "Start at:", "3");
    fill(&mut ui, "Indent list by:", "0.5\"");
    assert_eq!(lists(&ui), [None, None, None], "not applied yet");
    ok(&mut ui);
    let l = list_at(&ui, 0).unwrap();
    assert_eq!((l.kind, l.separator, l.start, l.list_indent_pt), (ListType::UpperLetter, 0, 3, 36.0));
    assert_eq!(labels(&ui), [s("C)"), s("D)"), s("E)")], "start at 3, letters, paren");
    assert_eq!(indents(&ui), [(36.0, -36.0); 3]);
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn the_separator_list_has_publishers_nine_entries() {
    let mut ui = selected();
    open_dialog(&mut ui, true);
    open_combo(&mut ui, 0);
    for sep in ["None", ")", "]", ":", "()", "[]", "--", "\u{3001}"] {
        assert!(ui.h.query_by_label(sep).is_some(), "separator {sep:?}");
    }
    ui.h.get_by_label("None").click();
    ui.settle();
    ok(&mut ui);
    assert_eq!(labels(&ui), [s("1"), s("2"), s("3")], "separator None shows the bare number");
}

#[test]
fn the_spinner_arrows_step_the_boxes() {
    let mut ui = selected();
    open_dialog(&mut ui, true);
    ui.h.get_by_role_and_label(Role::Button, "Start at: up").click();
    ui.settle();
    ui.h.get_by_role_and_label(Role::Button, "Start at: up").click();
    ui.settle();
    ui.h.get_by_role_and_label(Role::Button, "Start at: down").click();
    ui.settle();
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().start, "2");
    ui.h.get_by_role_and_label(Role::Button, "Indent list by: up").click();
    ui.settle();
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().num_indent, "0.35\"");
}

#[test]
fn a_box_that_holds_nothing_usable_keeps_ok_from_closing_the_dialog() {
    let mut ui = selected();
    open_dialog(&mut ui, true);
    fill(&mut ui, "Start at:", "abc");
    assert!(ui.h.get_by_label("OK").accesskit_node().is_disabled());
    ui.key(NONE, Key::Enter);
    assert!(dialog_open(&mut ui), "Enter does not apply a bad value either");
    assert_eq!(lists(&ui), [None, None, None]);
    fill(&mut ui, "Start at:", "7");
    ui.key(NONE, Key::Enter);
    assert!(!dialog_open(&mut ui));
    assert_eq!(list_at(&ui, 0).unwrap().start, 7);
}

#[test]
fn the_numbering_tab_with_the_no_bullet_button_clears_the_list() {
    let mut ui = selected();
    pick_number(&mut ui, "1. 2. 3.");
    open_dialog(&mut ui, false);
    tab_button(&mut ui, "Bullets");
    ok(&mut ui);
    assert_eq!(lists(&ui), [None, None, None], "the Bullets tab's blank button means no list");
    assert_eq!(indents(&ui), [(0.0, 0.0); 3]);
}

// ---- the Bullet Character dialog ----------------------------------------------------------------

fn open_characters(ui: &mut TestUi) {
    open_dialog(ui, false);
    dialog_bullet(ui, "Bullet type 1");
    click_label(ui, "Character...");
    assert!(ui.app().list_dialog.as_ref().unwrap().chars.is_some(), "the Bullet Character dialog is open");
}

/// Centre of the grid cell of character `code` (32 to 255, 32 columns).
fn cell(code: u32) -> Pos2 {
    let g = probe::find("chars-grid", "grid").expect("the character grid").rect;
    let i = (code - 32) as f32;
    g.min + Vec2::new((i % 32.0 + 0.5) * 17.0, ((i / 32.0).floor() + 0.5) * 17.0)
}

#[test]
fn the_bullet_character_dialog_has_a_32_column_grid_recent_symbols_and_the_code_boxes() {
    let mut ui = selected();
    open_characters(&mut ui);
    assert!(ui.h.query_by_label("Bullet Character").is_some(), "title");
    assert!(ui.h.query_by_label("Font:").is_some());
    assert!(ui.h.query_by_label("Recently used symbols:").is_some());
    assert!(ui.h.query_by_label("Symbol: 183").is_some(), "the current symbol's name and code");
    assert!(ui.h.query_by_label("Symbol (decimal)").is_some(), "from: Symbol (decimal)");
    let g = probe::find("chars-grid", "grid").unwrap().rect;
    assert_eq!((g.width(), g.height()), (32.0 * 17.0, 7.0 * 17.0), "32 columns, 7 rows");
    let r = probe::find("chars-recent", "recent").unwrap().rect;
    assert_eq!(r.height(), 17.0, "one row");
    ui.h.get_by_role_and_label(Role::TextInput, "Character code:");
}

#[test]
fn picking_a_cell_and_ok_sets_the_bullet_character_back_in_the_bullets_tab() {
    let mut ui = selected();
    open_characters(&mut ui);
    ui.click_at(cell(168)); // the diamond
    assert!(ui.h.query_by_label("Symbol: 168").is_some());
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().chars.as_ref().unwrap().code_text, "168");
    // Cancel keeps the old bullet.
    cancel(&mut ui);
    assert!(dialog_open(&mut ui), "only the Bullet Character dialog closed");
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().bullet, Some((183, "Symbol".to_owned())));
    click_label(&mut ui, "Character...");
    ui.click_at(cell(224));
    ok(&mut ui); // the Bullet Character dialog's OK
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().bullet, Some((224, "Symbol".to_owned())));
    assert!(ui.app().list_dialog.as_ref().unwrap().chars.is_none());
    ok(&mut ui); // the main dialog's OK
    assert!(!dialog_open(&mut ui));
    assert_eq!(list_at(&ui, 0).unwrap().bullet_char, 224);
    assert_eq!(labels(&ui)[0], s("\u{25CA}"));
}

#[test]
fn typing_a_character_code_selects_that_character() {
    let mut ui = selected();
    open_characters(&mut ui);
    fill(&mut ui, "Character code:", "42");
    assert!(ui.h.query_by_label("Symbol: 42").is_some());
    ok(&mut ui);
    assert_eq!(ui.app().list_dialog.as_ref().unwrap().bullet, Some((42, "Symbol".to_owned())));
}

#[test]
fn the_font_combo_changes_the_font_and_the_recent_row_remembers_picks_for_the_session() {
    let mut ui = selected();
    open_characters(&mut ui);
    pick_combo(&mut ui, 0, "Wingdings");
    assert!(ui.h.query_by_label("Wingdings (decimal)").is_some());
    ui.click_at(cell(167));
    ok(&mut ui);
    ok(&mut ui);
    let l = list_at(&ui, 0).unwrap();
    assert_eq!((l.bullet_char, l.bullet_font.as_str()), (167, "Wingdings"));
    // Reopen: the pick is the first recent symbol; clicking it sets code and font.
    open_dialog(&mut ui, false);
    click_label(&mut ui, "Character...");
    fill(&mut ui, "Character code:", "65");
    let r = probe::find("chars-recent", "recent").unwrap().rect;
    ui.click_at(r.min + Vec2::new(8.5, 8.5));
    let c = ui.app().list_dialog.as_ref().unwrap().chars.as_ref().unwrap();
    assert_eq!((c.code, c.font.as_str()), (167, "Wingdings"), "the newest recent symbol is first");
    assert_eq!(ui.app().recent_bullets[0], (167, "Wingdings".to_owned()));
    cancel(&mut ui);
    cancel(&mut ui);
}

// ---- Decrease / Increase Indent ----------------------------------------------------------------------

#[test]
fn increase_and_decrease_indent_step_a_quarter_inch_and_stop_at_zero() {
    let mut ui = selected();
    assert!(!ui.ribbon("Decrease Indent").enabled, "nothing to decrease at 0");
    assert!(ui.ribbon("Increase Indent").enabled);
    ui.click_ribbon("Increase Indent");
    assert_eq!(indents(&ui), [(18.0, 0.0); 3]);
    ui.click_ribbon("Increase Indent");
    assert_eq!(indents(&ui), [(36.0, 0.0); 3]);
    ui.click_ribbon("Decrease Indent");
    ui.click_ribbon("Decrease Indent");
    assert_eq!(indents(&ui), [(0.0, 0.0); 3]);
    assert!(!ui.ribbon("Decrease Indent").enabled, "greyed again at 0");
    let rev = ui.revision();
    ui.click_ribbon("Decrease Indent");
    assert_eq!(ui.revision(), rev, "a click on the greyed button does nothing");
    assert_eq!(indents(&ui), [(0.0, 0.0); 3]);
    assert_eq!(ui.undo_steps(), 4, "one undo step per click");
}

#[test]
fn the_indent_buttons_move_the_list_indent_of_list_paragraphs() {
    let mut ui = selected();
    pick_bullet(&mut ui, "Small Bullets");
    ui.click_ribbon("Increase Indent");
    assert_eq!(list_at(&ui, 1).unwrap().list_indent_pt, 36.0);
    assert_eq!(indents(&ui), [(36.0, -36.0); 3], "left follows the list indent, first line hangs");
    ui.click_ribbon("Decrease Indent");
    ui.click_ribbon("Decrease Indent");
    assert_eq!(list_at(&ui, 1).unwrap().list_indent_pt, 0.0, "stops at 0, never negative");
    assert_eq!(indents(&ui), [(0.0, 0.0); 3]);
    assert!(!ui.ribbon("Decrease Indent").enabled);
    ui.undo();
    assert_eq!(list_at(&ui, 1).unwrap().list_indent_pt, 18.0, "Ctrl+Z undoes one step");
}

// ---- selections, undo, other places ----------------------------------------------------------------------

#[test]
fn a_selection_formats_only_the_paragraphs_it_touches_and_a_caret_only_its_own() {
    let mut ui = editing();
    ui.set_selection_range(2, 5); // from "One" into "Two"
    pick_bullet(&mut ui, "Small Bullets");
    assert_eq!(lists(&ui).iter().map(Option::is_some).collect::<Vec<_>>(), [true, true, false]);
    assert!(ui.app().edit.is_some(), "still editing");
    ui.set_selection_range(10, 10); // a caret in "Three"
    pick_number(&mut ui, "A. B. C.");
    assert_eq!(lists(&ui).iter().map(Option::is_some).collect::<Vec<_>>(), [true, true, true]);
    assert_eq!(list_at(&ui, 2).unwrap().kind, ListType::UpperLetter);
    assert_eq!(list_at(&ui, 0).unwrap().kind, ListType::Bullet, "the others are untouched");
    assert_eq!(ui.app().editor.as_ref().unwrap().doc().pages[0].shapes.len(), 1);
}

#[test]
fn the_dialog_applies_to_the_paragraphs_selected_when_it_opened() {
    let mut ui = editing();
    ui.set_selection_range(4, 7); // "Two"
    open_dialog(&mut ui, true);
    ok(&mut ui);
    assert_eq!(lists(&ui).iter().map(Option::is_some).collect::<Vec<_>>(), [false, true, false]);
    assert_eq!(labels(&ui), [None, s("1."), None]);
    assert_eq!(ui.text_of(A), "One\nTwo\nThree", "the text is untouched");
}

#[test]
fn ctrl_z_while_editing_text_undoes_one_list_step_after_the_edit_is_committed() {
    let mut ui = selected();
    pick_number(&mut ui, "1. 2. 3.");
    pick_bullet(&mut ui, "Asterisk Bullets");
    ui.undo();
    assert_eq!(labels(&ui), [s("1."), s("2."), s("3.")]);
    ui.undo();
    assert_eq!(labels(&ui), [None, None, None]);
}

#[test]
fn lists_work_inside_a_table_cell() {
    let cells: Vec<TableCell> = (0..4).map(|i| TableCell::plain(&format!("c{i}"))).collect();
    let mut ui = TestUi::with_doc(letter_doc(vec![Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::Table { rows: 2, cols: 2, col_widths_emu: vec![], row_heights_emu: vec![], row_set_emu: vec![], cells },
    )]));
    ui.click_at(ui.center_of(A));
    assert!(!ui.ribbon("Bullets").enabled, "a selected table: no text to format");
    // One click more puts the caret in the cell under it.
    ui.click_at(ui.at_in(1.5, 1.4));
    assert_eq!(ui.app().edit.as_ref().map(|e| e.cell), Some(Some(0)), "caret in cell 0");
    ui.click_tab("Home");
    pick_bullet(&mut ui, "Large Bullets");
    let ShapeKind::Table { cells, .. } = &ui.shape(A).kind else { panic!() };
    assert!(cells[0].paragraphs[0].list.is_some(), "the cell's paragraph is bulleted");
    assert!(cells[1].paragraphs[0].list.is_none(), "the other cells are not");
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn the_buttons_are_greyed_with_no_text_to_format() {
    let mut ui = TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.5)]));
    ui.click_at(ui.center_of(1));
    for b in ["Bullets", "Numbering", "Decrease Indent", "Increase Indent"] {
        assert!(!ui.ribbon(b).enabled, "{b} is greyed for a rectangle");
        ui.click_ribbon(b);
    }
    assert!(ui.app().list_dialog.is_none());
    assert!(!ui.can_undo());
}
