//! The Insert, Page Design, Review, View and Help tabs, the ribbon's minimize button,
//! and how the ribbon fits narrow windows. Buttons are found through the ribbon seam; check boxes,
//! menus and dialogs through AccessKit. (The Page Setup group of Page Design is in `page_setup`.)

use super::support::*;
use crate::app::{Tab, Tool, ZoomMode};
use eframe::egui::{accesskit::Role, Key, Modifiers, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Paragraph, Rect, Shape, ShapeKind};

const A: u64 = 1;
const B: u64 = 2;
const NONE: Modifiers = Modifiers::NONE;

fn text_box_shape() -> Shape {
    Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::TextBox { paragraphs: vec![Paragraph::plain("Hello")] },
    )
}

/// Text box A (1, 1) and rectangle B (1, 4), one page.
fn ui_with_shapes() -> TestUi {
    TestUi::with_doc(letter_doc(vec![text_box_shape(), rect_shape(1.0, 4.0, 2.0, 1.5)]))
}

fn on(ui: &TestUi, name: &str) -> bool {
    ui.ribbon(name).enabled
}

fn click_menu(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn checkbox_disabled(ui: &TestUi, label: &str) -> bool {
    ui.h.get_all_by_label(label)
        .find(|n| n.accesskit_node().role() == Role::CheckBox)
        .unwrap_or_else(|| panic!("no check box {label:?}"))
        .accesskit_node()
        .is_disabled()
}

fn click_checkbox(ui: &mut TestUi, label: &str) {
    ui.h.get_all_by_label(label)
        .find(|n| n.accesskit_node().role() == Role::CheckBox)
        .unwrap_or_else(|| panic!("no check box {label:?}"))
        .click();
    ui.settle();
}

/// A real pointer click on a check box, which is what a user can do to a greyed one.
fn click_checkbox_with_the_mouse(ui: &mut TestUi, label: &str) {
    let n = ui
        .h
        .get_all_by_label(label)
        .find(|n| n.accesskit_node().role() == Role::CheckBox)
        .expect("check box");
    let r = n.accesskit_node().raw_bounds().expect("bounds");
    let at = eframe::egui::pos2(((r.x0 + r.x1) / 2.0) as f32, ((r.y0 + r.y1) / 2.0) as f32);
    ui.click_at(at);
}

// ---- Insert ---------------------------------------------------------------------------------------

const INSERT_GREYED: [&str; 18] = [
    "Catalog Pages", "Online Pictures", "Picture Placeholder", "Page Elements", "Calendars",
    "Borders & Accents", "Advertisements", "Contact Details", "Styled Text", "Insert File", "Symbol",
    "Date & Time", "Object", "Link", "Bookmark", "Header", "Footer", "Page Number",
];

#[test]
fn the_insert_tab_has_working_and_greyed_controls_as_planned() {
    let mut ui = ui_with_shapes();
    // Segoe UI 9pt labels are wider: the Insert tab needs a wider window to show every group in full.
    ui.h.set_size(eframe::egui::Vec2::new(1600.0, 800.0));
    ui.settle();
    ui.click_tab("Insert");
    for n in ["Page", "Table", "Pictures", "Shapes", "Draw Text Box"] {
        assert!(on(&ui, n), "{n} is live");
    }
    for n in INSERT_GREYED {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    assert_eq!(ui.greyed().len(), INSERT_GREYED.len(), "and nothing else on the tab is greyed");
}

#[test]
fn every_greyed_insert_control_does_nothing() {
    let mut ui = ui_with_shapes();
    ui.click_at(ui.center_of(A));
    ui.click_tab("Insert");
    ui.click_all_greyed_and_expect_nothing();
    assert!(!ui.can_undo());
    assert_eq!(ui.page_count(), 1);
}

#[test]
fn insert_blank_page_adds_a_page_after_this_one_and_shows_it() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Insert");
    ui.click_ribbon("Page");
    click_menu(&mut ui, "Insert Blank Page");
    assert_eq!(ui.page_count(), 2);
    assert_eq!(ui.app().page, 1, "the new page is current");
    assert!(ui.shapes().is_empty(), "and blank");
    assert_eq!(ui.doc().pages[0].shapes.len(), 2, "page 1 is untouched");
    assert_eq!(ui.undo_steps(), 1);
    ui.undo();
    assert_eq!(ui.page_count(), 1, "undo removes it");
}

#[test]
fn insert_blank_page_goes_after_the_current_page_not_at_the_end() {
    let mut ui = ui_with_shapes();
    ui.app().exec(kernpage_core::Command::InsertPage { after: 0 });
    ui.app().exec(kernpage_core::Command::InsertPage { after: 1 });
    ui.app().go_page(0);
    ui.settle();
    ui.click_tab("Insert");
    ui.click_ribbon("Page");
    click_menu(&mut ui, "Insert Blank Page");
    assert_eq!(ui.page_count(), 4);
    assert_eq!(ui.app().page, 1, "right after page 1");
}

#[test]
fn insert_duplicate_page_copies_the_page_and_its_shapes() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Insert");
    ui.click_ribbon("Page");
    click_menu(&mut ui, "Insert Duplicate Page");
    assert_eq!(ui.page_count(), 2);
    assert_eq!(ui.app().page, 1, "the copy is current");
    assert_eq!(ui.shapes().len(), 2, "with the same shapes");
    assert_eq!(ui.doc().pages[0].shapes.len(), 2);
    assert_eq!(ui.shapes()[1].rect, ui.doc().pages[0].shapes[1].rect);
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn the_insert_page_menu_lists_blank_and_duplicate_and_opening_it_changes_nothing() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Insert");
    ui.click_ribbon("Page");
    assert!(ui.h.query_by_label("Insert Blank Page").is_some());
    assert!(ui.h.query_by_label("Insert Duplicate Page").is_some());
    assert_eq!(ui.page_count(), 1);
    assert!(!ui.can_undo());
}

#[test]
fn the_insert_tabs_draw_text_box_arms_the_tool_and_clicking_again_disarms_it() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Insert");
    ui.click_ribbon("Draw Text Box");
    assert_eq!(ui.app().tool, Tool::TextBox);
    assert!(ui.ribbon("Draw Text Box").selected, "drawn pressed");
    ui.drag(ui.at_in(4.0, 6.0), ui.at_in(6.0, 8.0));
    assert_eq!(ui.shapes().len(), 3, "the drag drew a text box");
    assert!(matches!(ui.shapes()[2].kind, ShapeKind::TextBox { .. }));
    assert_eq!(ui.app().tool, Tool::Select);
    ui.click_tab("Insert");
    ui.click_ribbon("Draw Text Box");
    ui.click_ribbon("Draw Text Box");
    assert_eq!(ui.app().tool, Tool::Select, "a second click disarms it");
}

#[test]
fn the_insert_tabs_shapes_menu_arms_each_tool() {
    for (item, tool) in [("Rectangle", Tool::Rectangle), ("Oval", Tool::Oval), ("Line", Tool::Line)] {
        let mut ui = ui_with_shapes();
        ui.click_tab("Insert");
        ui.click_ribbon("Shapes");
        click_menu(&mut ui, item);
        assert_eq!(ui.app().tool, tool, "{item}");
    }
}

#[test]
fn the_tabs_are_greyed_without_a_publication() {
    let ui = TestUi::start();
    for t in ["Home", "Insert", "Page Design", "Data", "Review", "View", "Help"] {
        assert!(!ui.ribbon(&format!("tab:{t}")).enabled, "{t} needs a publication");
    }
    assert!(ui.ribbon("tab:File").enabled);
}

// ---- Page Design ------------------------------------------------------------------------------------

/// Open the collapsed "Page Background" group if the window is too narrow to show it.
fn open_page_background(ui: &mut TestUi) {
    if !ui.has_ribbon("Fonts") {
        ui.click_ribbon("Page Background");
    }
}

#[test]
fn page_design_greyed_controls_are_disabled_and_do_nothing() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Page Design");
    for n in ["Change Template", "Options", "Guides", "Delete", "Move", "Rename"] {
        assert!(!on(&ui, n), "{n} is greyed (a one-page publication cannot delete its page)");
    }
    for n in ["Margins", "Orientation", "Size"] {
        assert!(on(&ui, n), "{n} is live");
    }
    assert!(checkbox_disabled(&ui, "Guides") && checkbox_disabled(&ui, "Objects"), "Align To");
    let before = ui.revision();
    ui.click_all_greyed_and_expect_nothing();
    click_checkbox_with_the_mouse(&mut ui, "Guides");
    click_checkbox_with_the_mouse(&mut ui, "Objects");
    assert_eq!(ui.revision(), before);
    assert!(ui.app().page_dialog.is_none());
}

#[test]
fn the_page_background_group_holds_greyed_fonts_images_backgrounds_and_master_pages() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Page Design");
    open_page_background(&mut ui);
    for n in ["Fonts", "Apply Image", "Background", "Master Pages"] {
        assert!(ui.has_ribbon(n), "{n}");
        assert!(!on(&ui, n), "{n} is greyed");
    }
    let rev = ui.revision();
    for n in ["Fonts", "Apply Image", "Background", "Master Pages"] {
        let p = ui.ribbon(n).rect.center();
        ui.click_at(p);
    }
    assert_eq!(ui.revision(), rev);
    assert!(ui.app().error.is_none());
}

#[test]
fn page_design_delete_removes_the_current_page_and_is_undoable() {
    let mut ui = ui_with_shapes();
    ui.app().exec(kernpage_core::Command::InsertPage { after: 0 });
    ui.app().go_page(1);
    ui.settle();
    ui.click_tab("Page Design");
    assert!(on(&ui, "Delete"), "two pages: Delete is live");
    ui.click_ribbon("Delete");
    assert_eq!(ui.page_count(), 1);
    assert_eq!(ui.app().page, 0, "the view moves to a page that exists");
    assert_eq!(ui.shapes().len(), 2, "the first page kept its shapes");
    ui.undo();
    assert_eq!(ui.page_count(), 2, "undo brings the page back");
}

#[test]
fn the_palettes_gallery_is_shown_as_a_large_inert_gallery() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Page Design");
    let g = ui.ribbon("scheme_gallery");
    assert!(!g.enabled, "no colour palettes yet");
    assert!(g.rect.width() > 300.0 && g.rect.height() > 60.0, "six columns by three rows of swatches: {:?}", g.rect);
    let rev = ui.revision();
    ui.click_at(g.rect.center());
    ui.click_at(g.rect.min + Vec2::new(10.0, 10.0));
    assert_eq!(ui.revision(), rev);
    assert!(ui.app().error.is_none() && ui.app().page_dialog.is_none());
}

// ---- Data (our own mock-up; every control inert) ------------------------------------------------------------------

#[test]
fn the_data_tab_sits_between_page_design_and_review() {
    let ui = ui_with_shapes();
    let names = ui.tab_names();
    let at = |n: &str| names.iter().position(|x| x == n).unwrap();
    assert_eq!(at("Data"), at("Page Design") + 1);
    assert_eq!(at("Data") + 1, at("Review"));
}

#[test]
fn the_data_tab_is_greyed_without_a_publication() {
    let ui = TestUi::start();
    assert!(!ui.ribbon("tab:Data").enabled);
}

#[test]
fn data_has_every_control_greyed_and_none_does_anything() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Data");
    assert_eq!(ui.tab(), Tab::Data);
    for n in [
        "Data Source", "Edit List", "Insert Field", "Picture Field", "Numbering", "Make Copies",
        "Repeating Area", "Per Sheet", "Preview", "Find Row", "Create",
    ] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    let body = ui.ribbon_body();
    assert_eq!(body.len(), 11 + 4, "11 named controls and the 4 row-navigation icons");
    assert!(body.iter().all(|b| !b.enabled), "nothing on Data is live: {body:?}");
    let boxes: Vec<_> = ui.h.get_all_by_role(Role::TextInput).collect();
    assert_eq!(boxes.len(), 1, "the row number box");
    assert!(boxes[0].accesskit_node().is_disabled());
    ui.click_all_greyed_and_expect_nothing();
    assert!(!ui.can_undo());
}

#[test]
fn search_finds_the_data_tab_and_its_numbering_command() {
    let mut ui = ui_with_shapes();
    ui.click_ribbon("search");
    ui.type_text("data");
    assert!(ui.has_ribbon("search-result:Data"));
    ui.key(NONE, Key::Escape);
    ui.click_ribbon("search");
    ui.type_text("numbering");
    assert!(ui.has_ribbon("search-result:Numbering"));
    ui.click_ribbon("search-result:Numbering");
    assert_eq!(ui.tab(), Tab::Data);
}

// ---- Review ------------------------------------------------------------------------------

#[test]
fn review_has_every_control_greyed_and_none_does_anything() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Review");
    for n in ["Spelling", "Thesaurus", "Language"] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    let body = ui.ribbon_body();
    assert_eq!(body.len(), 3);
    for gone in ["Research", "Translate Selected Text"] {
        assert!(!ui.has_ribbon(gone), "{gone} was removed");
    }
    assert!(body.iter().all(|b| !b.enabled), "{body:?}");
    ui.click_all_greyed_and_expect_nothing();
    assert!(!ui.can_undo());
}

// ---- Help -----------------------------------------------------------------------------------------------

#[test]
fn the_help_button_opens_the_about_page_and_back_returns() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Help");
    assert!(ui.ribbon("About KernPage").enabled);
    ui.click_ribbon("About KernPage");
    assert!(ui.app().backstage && ui.app().error.is_none());
    ui.key(NONE, Key::Escape);
    assert!(!ui.app().backstage, "Esc returns to the publication");
    assert!(!ui.can_undo());
}

#[test]
fn help_is_greyed_with_a_coming_soon_tip_and_about_is_live() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Help");
    let h = ui.ribbon_buttons().into_iter().find(|b| b.label == "Help").expect("Help button");
    assert!(!h.enabled);
    assert_eq!(h.tip, "Coming soon: tutorials, how-to articles and answers to common questions.");
    assert!(ui.ribbon("About").enabled);
    ui.click_ribbon("About");
    assert!(ui.app().backstage && ui.app().error.is_none());
}

#[test]
fn online_pictures_is_greyed_with_its_own_tip() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Insert");
    let b = ui.ribbon("Online Pictures");
    assert!(!b.enabled);
    assert_eq!(b.tip, "Coming soon: search free-to-use photo sites and insert a picture.");
}

const REPO_ISSUES: &str = "https://github.com/jmitchell238/kernpage/issues/new?";

#[test]
fn help_report_a_problem_and_suggest_an_idea_are_enabled_with_tooltips() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Help");
    let r = ui.ribbon("Report a Problem");
    assert!(r.enabled);
    assert!(r.tip.starts_with("Report a Problem \u{2014} Opens a new GitHub issue in your web browser"), "{}", r.tip);
    assert!(r.tip.ends_with("Nothing is sent until you submit it there."));
    let i = ui.ribbon("Suggest an Idea");
    assert!(i.enabled);
    assert!(i.tip.starts_with("Suggest an Idea \u{2014} Opens a new GitHub issue in your web browser"), "{}", i.tip);
    assert!(!ui.app().backstage);
}

#[test]
fn clicking_report_a_problem_opens_a_prefilled_bug_issue_once() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Help");
    crate::feedback::test_os::take_opened();
    ui.click_ribbon("Report a Problem");
    let opened = crate::feedback::test_os::take_opened();
    assert_eq!(opened.len(), 1, "{opened:?}");
    assert!(opened[0].starts_with(&format!("{REPO_ISSUES}labels=bug&body=")), "{}", opened[0]);
    assert!(opened[0].contains(&crate::feedback::url_encode(&format!("KernPage version: {}", crate::app_version()))), "{}", opened[0]);
    assert!(opened[0].contains("Windows%3A"), "{}", opened[0]);
    assert!(!ui.can_undo());
}

#[test]
fn clicking_suggest_an_idea_opens_a_prefilled_idea_issue_once() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Help");
    crate::feedback::test_os::take_opened();
    ui.click_ribbon("Suggest an Idea");
    let opened = crate::feedback::test_os::take_opened();
    assert_eq!(opened.len(), 1, "{opened:?}");
    assert!(opened[0].starts_with(&format!("{REPO_ISSUES}labels=idea&body=")), "{}", opened[0]);
    assert!(opened[0].contains(&crate::feedback::url_encode(&format!("KernPage version: {}", crate::app_version()))), "{}", opened[0]);
}

#[test]
fn search_finds_report_a_problem_and_suggest_an_idea() {
    let mut ui = ui_with_shapes();
    ui.click_ribbon("search");
    ui.type_text("problem");
    assert!(ui.has_ribbon("search-result:Report a Problem"));
    ui.key(NONE, Key::Escape);
    ui.click_ribbon("search");
    ui.type_text("idea");
    assert!(ui.has_ribbon("search-result:Suggest an Idea"));
    ui.click_ribbon("search-result:Suggest an Idea");
    assert_eq!(ui.tab(), Tab::Help);
}

#[test]
fn the_question_mark_beside_the_ribbon_opens_the_same_about_page() {
    let mut ui = ui_with_shapes();
    ui.click_ribbon("Help and About");
    assert!(ui.app().backstage && ui.app().error.is_none());
    assert!(ui.h.query_all_by_label(&format!("KernPage {}", crate::app_version())).count() > 0);
}

// ---- View -------------------------------------------------------------------------------------------------

#[test]
fn the_view_tab_has_working_and_greyed_controls_as_planned() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    for n in ["Normal", "Single Page", "100%", "Whole Page", "Page Width"] {
        assert!(ui.body(n).enabled, "{n} is live");
    }
    for n in ["Master Page", "Two-Page Spread", "Selected Objects", "Arrange All", "Cascade", "Switch Windows"] {
        assert!(!ui.body(n).enabled, "{n} is greyed");
    }
    assert!(ui.body("Normal").selected && ui.body("Single Page").selected, "Normal and Single Page are current");
    assert!(ui.greyed().iter().all(|b| !b.label.is_empty()), "only named controls are greyed here");
    ui.click_all_greyed_and_expect_nothing();
    assert!(ui.body("Normal").selected, "still in Normal view");
    assert!(ui.body("Single Page").selected);
    assert!(!ui.can_undo());
}

#[test]
fn clicking_normal_and_single_page_keeps_the_current_views() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    let zoom = ui.app().zoom;
    ui.click_body("Normal");
    ui.click_body("Single Page");
    assert!(ui.body("Normal").selected && ui.body("Single Page").selected);
    assert_eq!(ui.app().zoom, zoom);
    assert!(!ui.can_undo());
}

#[test]
fn the_show_check_boxes_toggle_boundaries_guides_rulers_navigation_and_scratch_area() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    type Get = fn(&crate::app::Show) -> bool;
    let boxes: [(&str, Get); 5] = [
        ("Boundaries", |s| s.boundaries),
        ("Guides", |s| s.guides),
        ("Rulers", |s| s.rulers),
        ("Page Navigation", |s| s.nav),
        ("Scratch Area", |s| s.scratch),
    ];
    for (label, get) in boxes {
        // Publisher 2021 starts with boundaries off; the other four start on.
        let start = label != "Boundaries";
        assert_eq!(get(&ui.app().show), start, "{label} starting state");
        assert!(!checkbox_disabled(&ui, label), "{label} is live");
        click_checkbox(&mut ui, label);
        assert_eq!(get(&ui.app().show), !start, "{label} flips after one click");
        click_checkbox(&mut ui, label);
        assert_eq!(get(&ui.app().show), start, "{label} is back after a second click");
    }
    assert!(!ui.can_undo(), "view options are not document edits");
}

#[test]
fn the_unavailable_show_check_boxes_are_greyed_and_inert() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    for label in ["Fields", "Picture Manager", "Baselines"] {
        assert!(checkbox_disabled(&ui, label), "{label} is greyed");
        click_checkbox_with_the_mouse(&mut ui, label);
        assert!(checkbox_disabled(&ui, label), "{label} is still greyed");
    }
    let s = ui.app().show;
    assert!(s.rulers && s.nav && s.guides && s.scratch && !s.boundaries, "nothing else changed");
    assert!(ui.app().error.is_none());
}

#[test]
fn the_view_tab_zoom_buttons_set_the_zoom_mode() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    ui.click_body("100%");
    assert!(ui.app().zoom_mode == ZoomMode::Custom);
    assert!((ui.app().zoom - 1.0).abs() < 1e-9, "100% is exactly 1.0");
    assert!(ui.body("100%").selected && !ui.body("Whole Page").selected);
    ui.click_body("Page Width");
    assert!(ui.app().zoom_mode == ZoomMode::PageWidth);
    assert!(ui.body("Page Width").selected && !ui.body("100%").selected);
    ui.click_body("Whole Page");
    assert!(ui.app().zoom_mode == ZoomMode::WholePage);
    assert!(ui.body("Whole Page").selected);
    assert!(ui.app().zoom < 1.0, "a whole Letter page in an 800 pt tall window is below 100%");
    assert!(!ui.can_undo());
}

#[test]
fn the_view_tab_zoom_list_picks_a_zoom() {
    let mut ui = ui_with_shapes();
    ui.click_tab("View");
    let combo = ui.h.get_all_by_role(Role::ComboBox).next().expect("zoom list");
    combo.click();
    ui.settle();
    click_menu(&mut ui, "200%");
    assert!(ui.app().zoom_mode == ZoomMode::Custom);
    assert!((ui.app().zoom - 2.0).abs() < 1e-9);
    let combo = ui.h.get_all_by_role(Role::ComboBox).next().expect("zoom list");
    combo.click();
    ui.settle();
    click_menu(&mut ui, "25%");
    assert!((ui.app().zoom - 0.25).abs() < 1e-9);
}

// ---- Ribbon minimize ------------------------------------------------------------------------------------------

#[test]
fn the_minimize_button_hides_the_ribbon_and_the_expand_button_brings_it_back() {
    let mut ui = ui_with_shapes();
    assert!(ui.has_ribbon("Cut"));
    let origin_before = ui.app().probe.origin.y;
    ui.click_ribbon("Minimize the Ribbon");
    assert!(ui.app().ribbon_collapsed);
    assert!(!ui.has_ribbon("Cut"), "the groups are hidden");
    assert!(ui.has_ribbon("tab:Home") && ui.has_ribbon("tab:View"), "the tab strip stays");
    assert!(ui.ribbon("Expand the Ribbon").selected, "the button shows pressed and says Expand");
    assert!(!ui.has_ribbon("Minimize the Ribbon"));
    assert!(ui.app().probe.origin.y < origin_before - 40.0, "the page area moved up into the freed space");
    ui.click_ribbon("Expand the Ribbon");
    assert!(!ui.app().ribbon_collapsed);
    assert!(ui.has_ribbon("Cut"), "the ribbon is back");
    assert!((ui.app().probe.origin.y - origin_before).abs() < 1.0, "and the page area is where it was");
    assert!(!ui.can_undo());
}

#[test]
fn a_tab_picked_while_the_ribbon_is_minimized_is_shown_when_it_is_expanded() {
    let mut ui = ui_with_shapes();
    ui.click_ribbon("Minimize the Ribbon");
    ui.click_tab("View");
    assert_eq!(ui.tab(), Tab::View);
    assert!(ui.ribbon("tab:View").selected, "the tab is drawn selected");
    assert!(!ui.has_ribbon("Page Width"), "but the ribbon stays hidden");
    ui.click_ribbon("Expand the Ribbon");
    assert!(ui.has_ribbon("Page Width"), "the picked tab is showing");
}

#[test]
fn the_canvas_still_works_with_the_ribbon_minimized() {
    let mut ui = ui_with_shapes();
    ui.click_ribbon("Minimize the Ribbon");
    ui.settle();
    ui.click_at(ui.center_of(B));
    assert_eq!(ui.selected(), vec![B]);
    ui.key(NONE, Key::ArrowRight);
    assert!(ui.can_undo(), "keyboard editing works too");
    assert_eq!(ui.tab_names().len(), 7 + 1, "the selection's contextual tab is in the strip");
}

#[test]
fn the_minimize_button_is_greyed_without_a_publication() {
    let ui = TestUi::start();
    assert!(!ui.ribbon("Minimize the Ribbon").enabled);
}

// ---- Ribbon fit -------------------------------------------------------------------------------------------------------

const FIXED_TABS: [&str; 7] = ["Home", "Insert", "Page Design", "Data", "Review", "View", "Help"];

fn resize(ui: &mut TestUi, w: f32) {
    ui.h.set_size(Vec2::new(w, 800.0));
    ui.settle();
}

/// Everything wrong with the ribbon body right now: a control beyond the window's right edge, or
/// two controls overlapping. (The Height / Width boxes draw their spin arrows inside the field.)
fn fit_problems(ui: &TestUi, width: f32) -> Vec<String> {
    let body: Vec<Seen> = ui.ribbon_body();
    let mut bad = Vec::new();
    for b in &body {
        if b.rect.max.x > width + 0.5 || b.rect.min.x < -0.5 {
            bad.push(format!("{:?} sticks out of the window: {:?}", b.label, b.rect));
        }
    }
    for (i, a) in body.iter().enumerate() {
        for b in &body[i + 1..] {
            let nested = a.label.starts_with("size:") && b.label.starts_with("size:");
            if !nested && a.rect.shrink(0.6).intersects(b.rect.shrink(0.6)) {
                bad.push(format!("{:?} overlaps {:?}", a.label, b.label));
            }
        }
    }
    bad
}

/// The tab strip must not run under the Help and Minimize buttons at the right.
fn strip_problems(ui: &TestUi) -> Vec<String> {
    let all = ui.ribbon_buttons();
    let tabs: Vec<&Seen> = all.iter().filter(|b| b.label.starts_with("tab:")).collect();
    let right: Vec<&Seen> = all
        .iter()
        .filter(|b| {
            (b.tip == "Help and About" || b.tip == "Minimize the Ribbon" || b.tip == "Expand the Ribbon")
                && b.rect.min.y < 52.0
        })
        .collect();
    let mut bad = Vec::new();
    for t in &tabs {
        for r in &right {
            if t.rect.shrink(0.6).intersects(r.rect.shrink(0.6)) {
                bad.push(format!("{} runs under {:?}", t.label, r.tip));
            }
        }
    }
    bad
}

#[test]
fn every_fixed_tab_fits_the_window_at_the_minimum_width_and_wider() {
    let mut ui = ui_with_shapes();
    for w in [1280.0, 1000.0, 800.0] {
        resize(&mut ui, w);
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            assert!(!ui.ribbon_body().is_empty(), "{tab} at {w}: the ribbon is drawn");
            assert_eq!(fit_problems(&ui, w), Vec::<String>::new(), "{tab} at {w}");
            assert_eq!(strip_problems(&ui), Vec::<String>::new(), "{tab} at {w}: tab strip");
        }
    }
}

#[test]
fn the_fixed_tabs_still_fit_below_the_minimum_window_width() {
    let mut ui = ui_with_shapes();
    for w in [700.0, 600.0] {
        resize(&mut ui, w);
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            assert!(!ui.ribbon_body().is_empty(), "{tab} at {w}: the ribbon is drawn");
            assert_eq!(fit_problems(&ui, w), Vec::<String>::new(), "{tab} at {w}");
            assert_eq!(strip_problems(&ui), Vec::<String>::new(), "{tab} at {w}: tab strip");
        }
    }
}

#[test]
fn the_contextual_tabs_fit_the_window_at_the_minimum_width_and_wider() {
    let picture = Shape::new(
        Rect::new(in_to_emu(5.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(2.0)),
        ShapeKind::Picture {
            data: std::sync::Arc::new(vec![]),
            format: kernpage_core::model::PictureFormat::Unknown,
            file: None,
        },
    );
    let cells = (0..4).map(|i| kernpage_core::TableCell::plain(&format!("c{i}"))).collect();
    let table = Shape::new(
        Rect::new(in_to_emu(4.0), in_to_emu(6.0), in_to_emu(4.0), in_to_emu(2.0)),
        ShapeKind::Table { rows: 2, cols: 2, col_widths_emu: vec![], row_heights_emu: vec![], row_set_emu: vec![], cells },
    );
    let mut ui = TestUi::with_doc(letter_doc(vec![
        text_box_shape(),
        rect_shape(1.0, 4.0, 2.0, 1.0),
        picture,
        table,
    ]));
    for w in [1280.0, 1000.0, 800.0] {
        resize(&mut ui, w);
        for (id, tabs) in [
            (1, vec!["Shape Format", "Text Box"]),
            (2, vec!["Shape Format"]),
            (3, vec!["Picture Format"]),
            (4, vec!["Table Design", "Layout"]),
        ] {
            ui.click_at(ui.center_of(id));
            for tab in tabs {
                ui.click_tab(tab);
                ui.settle();
                assert!(!ui.ribbon_body().is_empty(), "{tab} at {w}: the ribbon is drawn");
                assert_eq!(fit_problems(&ui, w), Vec::<String>::new(), "{tab} at {w}");
                assert_eq!(strip_problems(&ui), Vec::<String>::new(), "{tab} at {w}: tab strip");
            }
        }
    }
}

#[test]
fn the_tab_strip_does_not_run_under_the_help_and_minimize_buttons_at_700() {
    let mut ui = ui_with_shapes();
    ui.click_at(ui.center_of(A));
    resize(&mut ui, 700.0);
    assert_eq!(ui.tab_names().len(), 9, "Shape Format and Text Box are in the strip");
    assert_eq!(strip_problems(&ui), Vec::<String>::new());
}

#[test]
fn narrowing_the_window_shrinks_groups_and_widening_it_restores_them() {
    let mut ui = ui_with_shapes();
    ui.click_tab("Home");
    resize(&mut ui, 1600.0);
    assert!(
        ui.has_ribbon("Bold (Ctrl+B)") && ui.has_ribbon("Italic (Ctrl+I)"),
        "wide: the Font group is shown in full"
    );
    let wide_cut = ui.ribbon("Cut").rect;
    resize(&mut ui, 700.0);
    // The narrowest layout turns whole groups into single drop-down buttons named after the group.
    for g in ["Font", "Paragraph", "Arrange"] {
        let b = ui.ribbon(g);
        assert!(b.enabled && b.tip == g, "{g} is collapsed into one button");
    }
    assert!(!ui.has_ribbon("Bold (Ctrl+B)"), "the Font controls are inside the pop-up now");
    assert_eq!(fit_problems(&ui, 700.0), Vec::<String>::new());
    resize(&mut ui, 1600.0);
    assert!(ui.has_ribbon("Bold (Ctrl+B)"), "wide again: the Font group is back in full");
    assert_eq!(ui.ribbon("Cut").rect, wide_cut, "and everything is where it was");
}

#[test]
fn a_collapsed_group_opens_a_pop_up_with_its_controls_and_closes_on_an_outside_click() {
    let mut ui = ui_with_shapes();
    ui.click_at(ui.center_of(B));
    resize(&mut ui, 700.0);
    ui.click_ribbon("Arrange");
    assert!(
        ui.has_ribbon("Send Backward") && ui.has_ribbon("Align") && ui.has_ribbon("Rotate"),
        "the group's controls are in the pop-up"
    );
    // A control in the pop-up works.
    ui.click_ribbon("Send Backward");
    assert_eq!(ui.shapes().iter().map(|s| s.id).collect::<Vec<_>>(), vec![B, A], "Send Backward ran");
    assert_eq!(ui.undo_steps(), 1);
    // A click elsewhere closes the pop-up.
    ui.click_at(ui.at_in(7.0, 9.5));
    assert!(!ui.has_ribbon("Send Backward"), "the pop-up is closed");
}

#[test]
fn the_ribbon_fit_survives_switching_tabs_while_narrow() {
    let mut ui = ui_with_shapes();
    resize(&mut ui, 800.0);
    for _ in 0..2 {
        for tab in FIXED_TABS {
            ui.click_tab(tab);
        }
    }
    ui.click_tab("Home");
    ui.settle();
    assert_eq!(fit_problems(&ui, 800.0), Vec::<String>::new());
    assert!(ui.has_ribbon("Paste"));
}

#[test]
fn no_group_title_is_wider_than_its_group_on_any_tab_at_any_width() {
    let mut ui = ui_with_shapes();
    for w in [900.0, 1024.0, 1280.0, 1920.0] {
        resize(&mut ui, w);
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            let all = ui.ribbon_buttons();
            let mut titles = 0;
            for t in all.iter().filter(|b| b.label.starts_with("grouptitle:")) {
                titles += 1;
                let name = &t.label["grouptitle:".len()..];
                let tile = all
                    .iter()
                    .find(|b| b.label == format!("grouptile:{name}"))
                    .unwrap_or_else(|| panic!("{tab} at {w}: no frame for {name:?}"));
                assert!(
                    t.rect.max.x <= tile.rect.max.x + 0.5 && t.rect.min.x >= tile.rect.min.x - 0.5,
                    "{tab} at {w}: title {name:?} {:?} sticks out of its group {:?}",
                    t.rect,
                    tile.rect
                );
            }
            assert!(titles > 0, "{tab} at {w}: group titles are drawn");
        }
    }
}

// ---- Ribbon look: no overlapping labels, one "on" style ------------------------------------------------------------

/// Every labelled control of the tab body: the ribbon buttons (seam) plus check boxes and combo
/// boxes (AccessKit), as `(name, rect)`.
fn labelled_controls(ui: &TestUi) -> Vec<(String, eframe::egui::Rect)> {
    let mut v: Vec<(String, eframe::egui::Rect)> = ui
        .ribbon_body()
        .into_iter()
        .map(|b| (b.label, b.rect))
        .collect();
    for role in [Role::CheckBox, Role::ComboBox] {
        for n in ui.h.query_all_by_role(role) {
            let a = n.accesskit_node();
            let Some(r) = a.raw_bounds() else { continue };
            let rect = eframe::egui::Rect::from_min_max(
                eframe::egui::pos2(r.x0 as f32, r.y0 as f32),
                eframe::egui::pos2(r.x1 as f32, r.y1 as f32),
            );
            if rect.min.y > 52.0 && rect.min.y < 150.0 {
                v.push((a.label().unwrap_or_else(|| format!("{role:?}")), rect));
            }
        }
    }
    v
}

#[test]
fn no_two_ribbon_controls_overlap_on_any_tab() {
    for w in [1400.0, 1000.0] {
        let mut ui = ui_with_shapes();
        ui.h.set_size(Vec2::new(w, 800.0));
        ui.settle();
        ui.wait_for_fonts();
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            let all = labelled_controls(&ui);
            assert!(!all.is_empty(), "{tab} at {w}: controls are drawn");
            for (i, (na, a)) in all.iter().enumerate() {
                for (nb, b) in &all[i + 1..] {
                    let rows = a.min.y < b.max.y - 2.0 && b.min.y < a.max.y - 2.0;
                    let across = a.min.x < b.max.x - 0.5 && b.min.x < a.max.x - 0.5;
                    assert!(!(rows && across), "{tab} at {w}: {na:?} {a:?} overlaps {nb:?} {b:?}");
                }
            }
        }
    }
}

#[test]
fn the_show_group_columns_keep_a_gap_between_their_labels() {
    let mut ui = ui_with_shapes();
    ui.h.set_size(Vec2::new(1400.0, 800.0));
    ui.settle();
    ui.wait_for_fonts();
    ui.click_tab("View");
    let all = labelled_controls(&ui);
    let rect = |n: &str| all.iter().find(|(l, _)| l == n).unwrap_or_else(|| panic!("no {n:?}")).1;
    for (left, right) in [("Boundaries", "Rulers"), ("Guides", "Rulers"), ("Page Navigation", "Picture Manager"), ("Scratch Area", "Picture Manager")] {
        let gap = rect(right).min.x - rect(left).max.x;
        assert!(gap >= 6.0, "{left:?} and {right:?} are only {gap} px apart");
    }
}

#[test]
fn every_toggle_button_on_every_tab_uses_the_same_on_style_inside_its_card() {
    use crate::widgets::{on_style, TOGGLE_RADIUS};
    let mut ui = ui_with_shapes();
    ui.h.set_size(Vec2::new(1400.0, 800.0));
    ui.settle();
    let mut seen_on = Vec::new();
    for tab in FIXED_TABS {
        ui.click_tab(tab);
        ui.settle();
        let all = ui.ribbon_buttons();
        for b in ui.ribbon_body().into_iter().filter(|b| b.selected) {
            let tile = all
                .iter()
                .filter(|t| t.label.starts_with("grouptile:") && t.rect.contains_rect(b.rect))
                .min_by(|x, y| x.rect.area().total_cmp(&y.rect.area()))
                .unwrap_or_else(|| panic!("{tab}: {:?} is outside every group card", b.label));
            // Kept inside the card's padding: never touching its edge.
            assert!(tile.rect.contains_rect(b.rect.shrink(3.0)), "{tab}: {:?} {:?} touches its card {:?}", b.label, b.rect, tile.rect);
            seen_on.push(b.label);
        }
    }
    for n in ["Normal", "Single Page", "Whole Page"] {
        assert!(seen_on.iter().any(|l| l == n), "{n} is an ON toggle on the View tab: {seen_on:?}");
    }
    // The one on-style: accent tint with an accent border; hovering deepens it.
    let (fill, line) = on_style(false);
    let (hover_fill, hover_line) = on_style(true);
    assert_eq!(fill, crate::theme::accent_tint());
    assert_eq!(line, crate::theme::accent());
    assert_eq!(hover_line, line);
    assert_ne!(hover_fill, fill, "hover on an ON button stays visible");
    assert_ne!(fill, crate::theme::tile(), "the on fill is not the card's grey");
    assert_eq!(TOGGLE_RADIUS, 3.0);
}

// ---- Ribbon cards: padding, launcher, crisp edges -----------------------------------------------------------------

fn tiles(ui: &TestUi) -> Vec<(String, eframe::egui::Rect)> {
    ui.ribbon_buttons()
        .into_iter()
        .filter_map(|b| b.label.strip_prefix("grouptile:").map(|n| (n.to_owned(), b.rect)))
        .collect()
}

fn card_of(tiles: &[(String, eframe::egui::Rect)], r: eframe::egui::Rect) -> Option<eframe::egui::Rect> {
    tiles.iter().filter(|(_, t)| t.contains(r.center())).map(|(_, t)| *t).min_by(|a, b| a.area().total_cmp(&b.area()))
}

#[test]
fn every_control_keeps_the_card_padding_on_both_sides_at_any_scaling() {
    for ppp in [1.0, 1.25, 1.5] {
        for w in [1500.0, 1000.0] {
            let mut ui = ui_with_shapes();
            ui.h.set_pixels_per_point(ppp);
            ui.h.set_size(Vec2::new(w, 800.0));
            ui.settle();
            ui.wait_for_fonts();
            for tab in FIXED_TABS {
                ui.click_tab(tab);
                ui.settle();
                let t = tiles(&ui);
                for (name, r) in labelled_controls(&ui) {
                    let Some(card) = card_of(&t, r) else { continue };
                    assert!(r.min.x - card.min.x >= 7.0 - 1e-3, "{tab} {w}@{ppp}: {name:?} starts {} px inside its card", r.min.x - card.min.x);
                    assert!(card.max.x - r.max.x >= 7.0 - 1e-3, "{tab} {w}@{ppp}: {name:?} ends {} px inside its card", card.max.x - r.max.x);
                }
            }
        }
    }
}

#[test]
fn the_dialog_launcher_is_inset_like_the_title_and_centred_on_its_line() {
    for ppp in [1.0, 1.25] {
        let mut ui = ui_with_shapes();
        ui.h.set_pixels_per_point(ppp);
        ui.h.set_size(Vec2::new(1500.0, 800.0));
        ui.settle();
        let mut found = 0;
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            let all = ui.ribbon_buttons();
            let t = tiles(&ui);
            for l in all.iter().filter(|b| b.label.starts_with("launcher:")) {
                let name = &l.label["launcher:".len()..];
                let card = t.iter().find(|(n, _)| n == name).unwrap().1;
                let title = all.iter().find(|b| b.label == format!("grouptitle:{name}")).unwrap().rect;
                let right = card.max.x - l.rect.max.x;
                let left = title.min.x - card.min.x;
                assert!((right - left).abs() <= 1.0, "{name}@{ppp}: launcher {right} px from the right edge, title {left} px from the left");
                assert!(left >= 7.0, "{name}: title inset {left}");
                assert!((l.rect.center().y - title.center().y).abs() <= 0.5, "{name}: launcher not centred on the title line");
                assert!(l.rect.min.y - card.min.y >= 6.0, "{name}: launcher crowds the top edge");
                found += 1;
            }
        }
        assert!(found >= 3, "Clipboard, Font and Paragraph have launchers");
    }
}

#[test]
fn group_cards_sit_on_whole_pixels_with_an_even_gap() {
    for ppp in [1.0f32, 1.25, 1.5] {
        let mut ui = ui_with_shapes();
        ui.h.set_pixels_per_point(ppp);
        ui.h.set_size(Vec2::new(1500.0, 800.0));
        ui.settle();
        ui.wait_for_fonts();
        for tab in FIXED_TABS {
            ui.click_tab(tab);
            ui.settle();
            let mut t = tiles(&ui);
            t.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
            for (n, r) in &t {
                for v in [r.min.x, r.min.y, r.max.x, r.max.y] {
                    let px = v * ppp;
                    assert!((px - px.round()).abs() < 1e-3, "{tab}@{ppp}: card {n:?} edge {v} is not on a device pixel");
                }
            }
            for p in t.windows(2) {
                let gap = p[1].1.min.x - p[0].1.max.x;
                assert!((gap - 6.0).abs() <= 1.0 / ppp + 1e-3, "{tab}@{ppp}: gap {gap} between {:?} and {:?}", p[0].0, p[1].0);
            }
        }
    }
}
