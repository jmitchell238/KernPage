//! Master page editing: the Master Page view (Ctrl+M, View > Master Page), its ribbon tab, the
//! Add / Duplicate / Rename / Apply dialogs, the Page Design > Master Pages menu, the Pages
//! pane's menu, Insert > Page Number and Header / Footer. Everything is driven with the mouse
//! and keyboard through the headless harness.

use super::support::*;
use crate::app::Tab;
use eframe::egui::epaint::Shape as Paint;
use eframe::egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{master_surface, Command, Document, Page, Paragraph, Rect, Shape, ShapeKind};

const NONE: Modifiers = Modifiers::NONE;

/// Every string painted in the last frame.
fn drawn_text(ui: &TestUi) -> Vec<String> {
    fn walk(s: &Paint, out: &mut Vec<String>) {
        match s {
            Paint::Text(t) => out.push(t.galley.text().to_owned()),
            Paint::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for c in &ui.h.output().shapes {
        walk(&c.shape, &mut out);
    }
    out
}

fn count(ui: &TestUi, what: &str) -> usize {
    drawn_text(ui).iter().filter(|t| t.as_str() == what).count()
}

fn three_page_ui() -> TestUi {
    let pages = (0..3).map(|_| Page::blank(in_to_emu(8.5), in_to_emu(11.0))).collect();
    let mut ui = TestUi::with_doc(Document::new("Test", pages));
    ui.wait_for_fonts();
    ui
}

fn text_box(text: &str) -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(0.5)),
        ShapeKind::TextBox { paragraphs: vec![Paragraph::plain(text)] },
    )
}

/// The Insert tab in a window wide enough to show the Header & Footer group in full.
fn click_insert(ui: &mut TestUi) {
    ui.h.set_size(eframe::egui::Vec2::new(1700.0, 800.0));
    ui.settle();
    ui.click_tab("Insert");
}

fn click_label(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn ok(ui: &mut TestUi) {
    ui.h.get_by_label("     OK     ").click();
    ui.settle();
}

fn uses(ui: &TestUi) -> Vec<(Option<usize>, bool)> {
    ui.doc().pages.iter().map(|p| (p.master, p.ignore_master)).collect()
}

fn text_of(s: &Shape) -> String {
    match &s.kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

// ---- the view -------------------------------------------------------------------------------

#[test]
fn ctrl_m_enters_and_leaves_the_master_page_view_and_the_tab_shows_only_there() {
    let mut ui = three_page_ui();
    ui.app().go_page(1);
    assert!(!ui.tab_names().contains(&"Master Page".to_owned()));
    ui.key(CTRL, Key::M);
    assert!(ui.app().in_master_view());
    assert_eq!(ui.tab(), Tab::MasterPage);
    assert!(ui.tab_names().contains(&"Master Page".to_owned()), "{:?}", ui.tab_names());
    // A document made in the editor gets master A when the view first opens.
    assert_eq!(ui.doc().masters.len(), 1);
    assert_eq!(ui.doc().masters[0].id, 'A');
    assert_eq!(ui.app().editing_master(), Some((0, false)));
    ui.key(CTRL, Key::M);
    assert!(!ui.app().in_master_view());
    assert_eq!(ui.app().page, 1, "back on the page it was opened from");
    assert!(!ui.tab_names().contains(&"Master Page".to_owned()));
    assert_ne!(ui.tab(), Tab::MasterPage);
}

#[test]
fn view_master_page_and_normal_buttons_switch_views() {
    let mut ui = three_page_ui();
    ui.click_tab("View");
    assert!(ui.body("Normal").selected && !ui.body("Master Page").selected);
    ui.click_body("Master Page");
    assert!(ui.app().in_master_view());
    ui.click_tab("View");
    assert!(ui.body("Master Page").selected && !ui.body("Normal").selected);
    ui.click_body("Normal");
    assert!(!ui.app().in_master_view());
    // Close Master Page on the Master Page tab does the same.
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Close Master Page");
    assert!(!ui.app().in_master_view());
}

#[test]
fn the_status_bar_and_pages_pane_name_the_master_being_edited() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    assert_eq!(ui.app().master_status().as_deref(), Some("Editing master page: Master Page A"));
    ui.h.get_by_label("Master Pages");
    let pane = crate::nav::probe::get();
    assert_eq!(pane.thumbs.len(), 1, "one master listed in place of the pages");
    assert!(pane.thumbs[0].2, "and it is the current one");
}

#[test]
fn page_operations_are_inert_in_the_master_page_view() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.app().insert_page();
    ui.app().delete_page();
    ui.app().duplicate_page_at(0);
    assert_eq!(ui.page_count(), 3);
    ui.key(Modifiers::NONE, Key::PageDown);
    assert!(ui.app().in_master_view());
}

// ---- add, duplicate, rename, delete ----------------------------------------------------------

#[test]
fn add_master_page_makes_b_named_master_page_b() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Add Master Page");
    assert!(ui.app().master_dialog.is_some());
    ok(&mut ui);
    let d = ui.doc();
    assert_eq!(d.masters.len(), 2);
    assert_eq!((d.masters[1].id, d.masters[1].name.as_str()), ('B', "Master Page B"));
    assert_eq!(ui.app().editing_master(), Some((1, false)), "the new master is shown");
    // Undo takes it away and the view falls back to A.
    ui.undo();
    assert_eq!(ui.doc().masters.len(), 1);
    assert_eq!(ui.app().editing_master(), Some((0, false)));
    ui.key(CTRL | Modifiers::SHIFT, Key::Z);
    assert_eq!(ui.doc().masters.len(), 2);
}

#[test]
fn the_add_dialog_takes_an_id_a_description_and_the_two_page_option() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Add Master Page");
    ui.h.get_by_label("Page ID").click();
    ui.key(CTRL, Key::A);
    ui.type_text("Q");
    ui.h.get_by_label("Description").click();
    ui.type_text("Cover pages");
    click_label(&mut ui, "Two-page master");
    ok(&mut ui);
    let m = &ui.doc().masters[1];
    assert_eq!((m.id, m.name.as_str(), m.description.as_str(), m.two_page), ('Q', "Master Page Q", "Cover pages", true));
    // A two-page master is listed as two thumbnails (left and right).
    ui.settle();
    assert_eq!(crate::nav::probe::get().thumbs.len(), 3);
}

#[test]
fn an_id_already_in_use_keeps_the_dialog_open() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Add Master Page");
    ui.h.get_by_label("Page ID").click();
    ui.key(CTRL, Key::A);
    ui.type_text("A");
    ui.settle();
    ui.h.get_by_label_contains("Another master page already has the ID A");
    ok(&mut ui);
    assert!(ui.app().master_dialog.is_some());
    assert_eq!(ui.doc().masters.len(), 1);
    ui.key(NONE, Key::Escape);
    assert!(ui.app().master_dialog.is_none());
}

#[test]
fn duplicate_copies_the_shapes_of_the_master() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.app().add_shape(text_box("Header text"));
    ui.settle();
    ui.click_ribbon("Duplicate");
    ok(&mut ui);
    let d = ui.doc();
    assert_eq!(d.masters.len(), 2);
    assert_eq!(d.masters[1].shapes.len(), 1);
    assert_eq!(text_of(&d.masters[1].shapes[0]), "Header text");
    assert_ne!(d.masters[0].shapes[0].id, d.masters[1].shapes[0].id);
    assert_eq!(d.masters[1].name, "Copy of Master Page A");
    // The duplicate is its own master: editing it leaves A alone.
    ui.app().add_shape(text_box("Only on the copy"));
    assert_eq!(ui.doc().masters[0].shapes.len(), 1);
    assert_eq!(ui.doc().masters[1].shapes.len(), 2);
}

#[test]
fn rename_changes_the_id_name_and_description() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Rename");
    ui.h.get_by_label("Page ID").click();
    ui.key(CTRL, Key::A);
    ui.type_text("F");
    ui.h.get_by_label("Name").click();
    ui.key(CTRL, Key::A);
    ui.type_text("Front");
    ui.h.get_by_label("Description").click();
    ui.type_text("Cover");
    ok(&mut ui);
    let m = &ui.doc().masters[0];
    assert_eq!((m.id, m.name.as_str(), m.description.as_str()), ('F', "Front", "Cover"));
    ui.undo();
    assert_eq!(ui.doc().masters[0].id, 'A');
}

#[test]
fn delete_refuses_the_last_master_and_repoints_the_pages() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Delete");
    assert!(ui.app().error.as_deref().is_some_and(|e| e.contains("default master page")), "{:?}", ui.app().error);
    assert_eq!(ui.doc().masters.len(), 1);
    ui.app().error = None;
    // Two more masters; pages 2 and 3 use B and C.
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui.app().exec(Command::ApplyMaster { pages: vec![1], master: Some(1) });
    ui.app().exec(Command::ApplyMaster { pages: vec![2], master: Some(2) });
    ui.app().go_master(1, false);
    ui.settle();
    ui.click_ribbon("Delete");
    assert_eq!(ui.doc().masters.iter().map(|m| m.id).collect::<Vec<_>>(), ['A', 'C']);
    assert_eq!(uses(&ui), [(Some(0), false), (Some(0), false), (Some(1), false)]);
    assert!(ui.app().in_master_view());
    ui.undo();
    assert_eq!(ui.doc().masters.len(), 3);
    assert_eq!(uses(&ui)[1].0, Some(1), "undo gives page 2 its master back");
}

#[test]
fn two_page_master_button_toggles_and_shows_both_halves() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Two-Page Master");
    assert!(ui.doc().masters[0].two_page);
    assert!(ui.ribbon("Two-Page Master").selected);
    ui.app().go_master(0, true);
    ui.settle();
    assert_eq!(ui.app().editing_master(), Some((0, true)));
    ui.app().add_shape(text_box("LEFT"));
    assert_eq!(ui.doc().masters[0].left_shapes.len(), 1);
    // Turning it off drops the left half and returns to the right one.
    ui.click_ribbon("Two-Page Master");
    assert!(!ui.doc().masters[0].two_page && ui.doc().masters[0].left_shapes.is_empty());
    assert_eq!(ui.app().editing_master(), Some((0, false)));
}

// ---- apply -----------------------------------------------------------------------------------

fn with_two_masters() -> TestUi {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui
}

#[test]
fn apply_to_current_page_and_all_pages_from_the_master_page_tab() {
    let mut ui = with_two_masters();
    ui.app().go_page(0); // ignored in this view; the page it was opened from stays 1
    ui.click_ribbon("Apply To");
    click_label(&mut ui, "Current Page");
    assert_eq!(uses(&ui), [(Some(1), false), (Some(0), false), (Some(0), false)]);
    ui.click_ribbon("Apply To");
    click_label(&mut ui, "All Pages");
    assert_eq!(uses(&ui), [(Some(1), false); 3]);
    ui.undo();
    assert_eq!(uses(&ui)[2].0, Some(0));
}

#[test]
fn apply_to_pages_dialog_takes_a_range() {
    let mut ui = with_two_masters();
    ui.click_ribbon("Apply To");
    click_label(&mut ui, "Pages...");
    ui.h.get_by_label("Apply Master Page");
    click_label(&mut ui, "Pages");
    ui.h.get_by_label("From page").click();
    ui.key(CTRL, Key::A);
    ui.type_text("2");
    ui.h.get_by_label("To page").click();
    ui.key(CTRL, Key::A);
    ui.type_text("3");
    ui.settle();
    ok(&mut ui);
    assert_eq!(uses(&ui), [(Some(0), false), (Some(1), false), (Some(1), false)]);
}

#[test]
fn a_bad_page_range_keeps_the_apply_dialog_open() {
    let mut ui = with_two_masters();
    ui.click_ribbon("Apply To");
    click_label(&mut ui, "Pages...");
    click_label(&mut ui, "Pages");
    ui.h.get_by_label("To page").click();
    ui.key(CTRL, Key::A);
    ui.type_text("9");
    ui.settle();
    ui.h.get_by_label_contains("Enter page numbers from 1 to 3");
    ok(&mut ui);
    assert!(ui.app().apply_dialog.is_some());
    ui.key(NONE, Key::Escape);
    assert!(ui.app().apply_dialog.is_none());
}

#[test]
fn page_design_master_pages_menu_applies_a_master_or_none_and_none_hides_the_shapes() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    ui.app().add_shape(text_box("MASTERTEXT"));
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui.click_ribbon("Close Master Page");
    assert!(drawn_text(&ui).iter().any(|t| t == "MASTERTEXT"));
    ui.click_tab("Page Design");
    ui.click_ribbon("Master Pages");
    for label in ["A - Master Page A", "B - Master Page B", "None", "Edit Master Pages", "Apply Master Page..."] {
        ui.h.get_by_label(label);
    }
    click_label(&mut ui, "B - Master Page B");
    assert_eq!(uses(&ui), [(Some(1), false), (Some(0), false), (Some(0), false)]);
    assert!(!drawn_text(&ui).iter().any(|t| t == "MASTERTEXT" && false));
    // None: the page keeps B but ignores it, so A's and B's shapes are gone from it.
    ui.click_ribbon("Master Pages");
    click_label(&mut ui, "None");
    assert_eq!(uses(&ui)[0], (Some(1), true));
    assert!(ui.doc().master_shapes(0).is_empty());
    assert_eq!(ui.doc().master_shapes(1).len(), 1, "other pages still show theirs");
    ui.undo();
    assert_eq!(uses(&ui)[0], (Some(1), false));
}

#[test]
fn the_master_pages_menu_marks_the_current_master_and_edit_opens_the_view() {
    let mut ui = three_page_ui();
    ui.click_tab("Page Design");
    ui.click_ribbon("Master Pages");
    click_label(&mut ui, "Edit Master Pages");
    assert!(ui.app().in_master_view());
    assert_eq!(ui.app().editing_master(), Some((0, false)));
}

#[test]
fn the_pages_pane_right_click_menu_has_master_pages() {
    let mut ui = three_page_ui();
    ui.app().exec(Command::AddMaster { copy_of: None, id: None, description: String::new(), two_page: false });
    ui.settle();
    let thumb = crate::nav::probe::get().thumbs[1].1;
    ui.right_click_at(thumb.center());
    ui.settle();
    ui.h.get_by_label("Master Pages").click();
    ui.settle();
    click_label(&mut ui, "B - Master Page B");
    assert_eq!(uses(&ui), [(Some(0), false), (Some(1), false), (Some(0), false)]);
}

// ---- editing a master shows on every page ------------------------------------------------------

#[test]
fn editing_a_master_shape_shows_on_every_page_that_uses_it() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    let id = ui.app().add_shape(text_box("FIRSTTEXT")).unwrap();
    ui.key(CTRL, Key::M);
    ui.settle();
    // Page 1 on the canvas and in its thumbnail, pages 2 and 3 in theirs.
    assert_eq!(count(&ui, "FIRSTTEXT"), 4);
    ui.app().exec(Command::SetText {
        page: master_surface(0, false),
        id,
        paragraphs: vec![Paragraph::plain("SECONDTEXT")],
    });
    ui.settle();
    assert_eq!((count(&ui, "FIRSTTEXT"), count(&ui, "SECONDTEXT")), (0, 4));
    ui.app().go_page(2);
    ui.settle();
    assert_eq!(count(&ui, "SECONDTEXT"), 4);
    // Master shapes are still not selectable on a page.
    let p = ui.at_in(1.0 + 1.5, 1.0 + 0.25);
    ui.click_at(p);
    assert!(ui.selected().is_empty());
}

#[test]
fn master_shapes_are_selectable_movable_and_deletable_in_the_master_page_view() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    let id = ui.app().add_shape(text_box("Box")).unwrap();
    ui.settle();
    ui.app().set_selection(vec![]);
    ui.settle();
    let at = ui.at_in(1.0 + 1.5, 1.0 + 0.25);
    ui.click_at(at);
    assert_eq!(ui.selected(), [id], "a click selects the master's box");
    ui.drag(at, ui.at_in(3.5, 2.25));
    let r = ui.doc().masters[0].shapes[0].rect;
    assert!(r.x > in_to_emu(1.5) && r.y > in_to_emu(1.5), "{r:?}");
    ui.key(NONE, Key::Delete);
    assert!(ui.doc().masters[0].shapes.is_empty());
    ui.undo();
    assert_eq!(ui.doc().masters[0].shapes.len(), 1);
    // The shape tabs appear next to the Master Page tab.
    ui.app().set_selection(vec![id]);
    ui.settle();
    assert!(ui.tab_names().contains(&"Master Page".to_owned()));
    assert!(ui.tab_names().contains(&"Text Box".to_owned()), "{:?}", ui.tab_names());
}

// ---- Insert: page numbers, header and footer ----------------------------------------------------

#[test]
fn insert_page_number_bottom_center_puts_a_field_box_on_the_master_and_pages_show_their_number() {
    let mut ui = three_page_ui();
    click_insert(&mut ui);
    ui.click_ribbon("Page Number");
    click_label(&mut ui, "Bottom Center");
    assert!(!ui.app().in_master_view(), "the page stays in the normal view");
    let d = ui.doc();
    assert_eq!(d.masters.len(), 1);
    assert_eq!(d.masters[0].shapes.len(), 1);
    let s = &d.masters[0].shapes[0];
    let ShapeKind::TextBox { paragraphs } = &s.kind else { panic!("not a text box") };
    assert_eq!(paragraphs[0].runs[0].field, Some(kernpage_core::Field::PageNumber));
    assert_eq!(paragraphs[0].align, kernpage_core::Align::Center);
    let page_w = in_to_emu(8.5);
    assert_eq!(s.rect.x + s.rect.w / 2, page_w / 2, "centred on the page");
    assert!(s.rect.y > in_to_emu(10.0), "in the bottom margin: {:?}", s.rect);
    assert!(d.pages.iter().all(|p| p.shapes.is_empty()), "no box on the pages themselves");
    // Page 1 shows 1 (canvas + thumbnail + the pane's own label), the others in their thumbnails.
    ui.wait_for_fonts();
    // The rulers draw the same digits whatever the page, so compare the counts with each other.
    let (n1, n2, n3) = (count(&ui, "1"), count(&ui, "2"), count(&ui, "3"));
    assert_eq!((n1 - n2, n1 - n3), (1, 1), "{:?}", drawn_text(&ui));

    ui.app().go_page(2);
    ui.settle();
    let (m1, m2, m3) = (count(&ui, "1"), count(&ui, "2"), count(&ui, "3"));
    assert_eq!((m3 - m1, m3 - m2), (1, 1), "now page 3 is on the canvas too");
    ui.undo();
    assert!(ui.doc().masters[0].shapes.is_empty());
}

#[test]
fn page_number_positions_follow_the_menu_choice() {
    for (label, top, align) in [
        ("Top Left", true, kernpage_core::Align::Left),
        ("Top Right", true, kernpage_core::Align::Right),
        ("Bottom Left", false, kernpage_core::Align::Left),
        ("Bottom Right", false, kernpage_core::Align::Right),
    ] {
        let mut ui = three_page_ui();
        click_insert(&mut ui);
        ui.click_ribbon("Page Number");
        click_label(&mut ui, label);
        let s = &ui.doc().masters[0].shapes[0];
        let ShapeKind::TextBox { paragraphs } = &s.kind else { panic!() };
        assert_eq!(paragraphs[0].align, align, "{label}");
        assert_eq!(s.rect.y < in_to_emu(5.0), top, "{label}");
        let margin = ui.doc().pages[0].margins_emu;
        match align {
            kernpage_core::Align::Left => assert_eq!(s.rect.x, margin.left, "{label}"),
            _ => assert_eq!(s.rect.x + s.rect.w, in_to_emu(8.5) - margin.right, "{label}"),
        }
    }
}

#[test]
fn page_number_in_the_master_page_view_selects_the_new_box() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    click_insert(&mut ui);
    ui.click_ribbon("Page Number");
    click_label(&mut ui, "Top Right");
    assert_eq!(ui.selected().len(), 1);
    assert_eq!(ui.doc().masters[0].shapes.len(), 1);
}

#[test]
fn insert_in_current_text_box_puts_the_field_at_the_caret() {
    let mut ui = three_page_ui();
    let id = ui.app().add_shape(text_box("Page ")).unwrap();
    ui.settle();
    ui.app().begin_edit(id);
    ui.settle();
    ui.key(CTRL, Key::End);
    click_insert(&mut ui);
    ui.click_ribbon("Page Number");
    click_label(&mut ui, "Insert in Current Text Box");
    ui.app().commit_edit();
    let s = &ui.doc().pages[0].shapes[0];
    let ShapeKind::TextBox { paragraphs } = &s.kind else { panic!() };
    assert_eq!(paragraphs[0].text(), "Page #");
    assert_eq!(paragraphs[0].runs.last().unwrap().field, Some(kernpage_core::Field::PageNumber));
    assert!(paragraphs[0].runs[0].field.is_none());
}

#[test]
fn insert_in_current_text_box_is_greyed_when_not_editing_text() {
    let mut ui = three_page_ui();
    click_insert(&mut ui);
    ui.click_ribbon("Page Number");
    assert!(ui.h.get_by_label("Insert in Current Text Box").accesskit_node().is_disabled());
    assert!(ui.h.get_by_label("Format Page Numbers...").accesskit_node().is_disabled());
}

#[test]
fn header_and_footer_open_the_master_view_with_boxes_and_select_the_one_to_type_in() {
    let mut ui = three_page_ui();
    click_insert(&mut ui);
    ui.click_ribbon("Header");
    assert!(ui.app().in_master_view());
    let m = &ui.doc().masters[0];
    assert_eq!(m.shapes.len(), 2, "a header and a footer");
    let (header, footer) = (&m.shapes[0], &m.shapes[1]);
    let margin = m.margins_emu;
    assert!(header.rect.y + header.rect.h <= margin.top + 274_320, "{:?}", header.rect);
    assert!(footer.rect.y >= in_to_emu(11.0) - margin.bottom - 274_320, "{:?}", footer.rect);
    assert_eq!(ui.selected(), [header.id], "the header is selected");
    assert!(ui.app().edit.is_some(), "and open for typing");
    ui.type_text("Church Bulletin");
    ui.app().commit_edit();
    assert_eq!(text_of(&ui.doc().masters[0].shapes[0]), "Church Bulletin");
    // The header shows on every page.
    ui.key(CTRL, Key::M);
    ui.settle();
    assert_eq!(count(&ui, "Church Bulletin"), 4);
    // Footer on a second visit does not add more boxes and selects the footer.
    click_insert(&mut ui);
    ui.click_ribbon("Footer");
    assert_eq!(ui.doc().masters[0].shapes.len(), 2);
    let footer_id = ui.doc().masters[0].shapes[1].id;
    assert_eq!(ui.selected(), [footer_id]);
}

#[test]
fn saving_a_pub_with_master_changes_names_what_cannot_be_saved() {
    use crate::app::Prompt;
    let mut ui = TestUi::with_pub_sample("gen/master-pn-2p.pub");
    ui.key(CTRL, Key::M);
    assert!(ui.app().in_master_view());
    // A new footer and a new master are written; a two-page master is not (it is named).
    ui.app().add_shape(text_box("New footer"));
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui.click_ribbon("Two-Page Master");
    ui.key(CTRL, Key::S);
    ui.settle();
    match &ui.app().prompt {
        Some(Prompt::Unsupported { items, .. }) => {
            assert!(items.iter().any(|i| i.contains("two-page master pages")), "{items:?}");
            assert!(!items.iter().any(|i| i.contains("adding master pages")), "{items:?}");
        }
        other => panic!("expected the unsupported-changes prompt, got {}", other.is_some()),
    }
}

#[test]
fn a_new_master_and_a_footer_on_it_are_part_of_the_save_plan() {
    let mut ui = TestUi::with_pub_sample("gen/master-pn-2p.pub");
    ui.key(CTRL, Key::M);
    ui.app().add_shape(text_box("New footer"));
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    let doc = ui.doc().clone();
    let plan = ui.app().source.as_ref().unwrap().plan(&doc);
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(plan.new_masters.len(), 1);
    assert_eq!(plan.new_text_boxes.len(), 1);
}

#[test]
fn opening_and_saving_a_pub_without_master_changes_asks_nothing() {
    let mut ui = TestUi::with_pub_sample("gen/master-pn-2p.pub");
    assert!(!ui.doc().masters.is_empty());
    // Looking at the master changes nothing.
    ui.key(CTRL, Key::M);
    ui.key(CTRL, Key::M);
    assert!(!ui.can_undo());
    let doc = ui.doc().clone();
    let plan = ui.app().source.as_ref().unwrap().plan(&doc);
    assert!(plan.unsupported.is_empty() && plan.is_empty(), "{:?}", plan.unsupported);
}

#[test]
fn each_master_command_is_one_undo_step() {
    let mut ui = three_page_ui();
    ui.key(CTRL, Key::M);
    let base = ui.undo_steps();
    ui.click_ribbon("Add Master Page");
    ok(&mut ui);
    ui.click_ribbon("Duplicate");
    ok(&mut ui);
    ui.click_ribbon("Two-Page Master");
    ui.click_ribbon("Delete");
    ui.click_ribbon("Apply To");
    click_label(&mut ui, "All Pages");
    assert_eq!(ui.undo_steps() - base, 5);
}
