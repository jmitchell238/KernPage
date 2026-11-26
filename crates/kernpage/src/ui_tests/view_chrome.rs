//! The chrome around the page: the Pages pane (thumbnails and its right-click menu), the rulers,
//! and the status bar (page n of N, cursor position, zoom controls, view buttons).
//!
//! Thumbnails and rulers are painted, so they are found through test seams (`nav::probe`,
//! `canvas::Probe::ruler_h / ruler_v`); the status bar's labelled widgets are found through
//! AccessKit and the ribbon seam.

use super::support::*;
use crate::app::ZoomMode;
use eframe::egui::{accesskit::Role, Pos2, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::geometry::{MAX_ZOOM, MIN_ZOOM};
use kernpage_core::{Command, Rect};

const A: u64 = 1;

/// Three Letter pages; page 1 holds rectangle A, the others are blank. On page 1.
fn three_pages() -> TestUi {
    let mut ui = TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.0)]));
    ui.app().exec(Command::InsertPage { after: 0 });
    ui.app().exec(Command::InsertPage { after: 1 });
    ui.app().go_page(0);
    ui.settle();
    ui
}

fn one_page() -> TestUi {
    TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.0)]))
}

fn thumbs(ui: &TestUi) -> Vec<(usize, eframe::egui::Rect, bool)> {
    let _ = ui;
    crate::nav::probe::get().thumbs
}

fn thumb_center(ui: &TestUi, page: usize) -> Pos2 {
    thumbs(ui)
        .into_iter()
        .find(|(i, _, _)| *i == page)
        .unwrap_or_else(|| panic!("no thumbnail for page {page}"))
        .1
        .center()
}

fn current_page(ui: &mut TestUi) -> usize {
    ui.app().page
}

fn status_page_text(ui: &TestUi, cur: usize, total: usize) {
    ui.h.get_by_label(&format!("Page: {cur} of {total}"));
}

fn menu_item_enabled(ui: &TestUi, name: &str) -> bool {
    !ui.h.get_by_label(name).accesskit_node().is_disabled()
}

/// Right-click a thumbnail and click the menu item `name`.
fn thumb_menu(ui: &mut TestUi, page: usize, name: &str) {
    let at = thumb_center(ui, page);
    ui.right_click_at(at);
    ui.h.get_by_label(name).click();
    ui.settle();
}

// ---- Pages pane ------------------------------------------------------------------------------------

#[test]
fn the_pages_pane_has_a_header_and_one_thumbnail_per_page_in_order() {
    let ui = three_pages();
    ui.h.get_by_label("Pages");
    let t = thumbs(&ui);
    assert_eq!(t.iter().map(|(i, _, _)| *i).collect::<Vec<_>>(), [0, 1, 2]);
    // Thumbnails are stacked top to bottom and have the page's aspect (Letter: 11 / 8.5).
    assert!(t[0].1.max.y < t[1].1.min.y && t[1].1.max.y < t[2].1.min.y);
    let aspect = t[0].1.height() / t[0].1.width();
    assert!((aspect - 11.0 / 8.5).abs() < 0.01, "aspect {aspect}");
    // The page numbers are drawn under the thumbnails.
    for n in ["1", "2", "3"] {
        ui.h.get_by_label(n);
    }
}

#[test]
fn the_current_page_thumbnail_is_highlighted() {
    let mut ui = three_pages();
    let flags = |ui: &TestUi| thumbs(ui).into_iter().map(|(_, _, c)| c).collect::<Vec<_>>();
    assert_eq!(flags(&ui), [true, false, false]);
    ui.app().go_page(2);
    ui.settle();
    assert_eq!(flags(&ui), [false, false, true]);
}

#[test]
fn clicking_a_thumbnail_selects_that_page_and_drops_the_selection() {
    let mut ui = three_pages();
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A]);
    let steps = ui.undo_steps();
    ui.click_at(thumb_center(&ui, 2));
    assert_eq!(current_page(&mut ui), 2);
    assert!(ui.selected().is_empty(), "the selection belongs to the page that was left");
    status_page_text(&ui, 3, 3);
    assert_eq!(
        thumbs(&ui).into_iter().filter(|(_, _, c)| *c).map(|(i, _, _)| i).collect::<Vec<_>>(),
        [2]
    );
    ui.click_at(thumb_center(&ui, 0));
    assert_eq!(current_page(&mut ui), 0);
    status_page_text(&ui, 1, 3);
    assert_eq!(ui.undo_steps(), steps, "changing page is not an edit");
}

#[test]
fn the_close_button_hides_the_pane_and_the_view_tab_shows_it_again() {
    let mut ui = three_pages();
    let origin_with = ui.h.state().probe.origin.x;
    let close = crate::nav::probe::get().close.expect("close button");
    ui.click_at(close.center());
    assert!(!ui.app().show.nav);
    assert!(thumbs(&ui).is_empty(), "no thumbnails drawn");
    assert!(ui.h.state().probe.origin.x < origin_with, "the page area uses the freed space");
    ui.click_tab("View");
    ui.h.get_by_label("Page Navigation").click();
    ui.settle();
    assert!(ui.app().show.nav);
    assert_eq!(thumbs(&ui).len(), 3);
    // And the same checkbox hides it again.
    ui.h.get_by_label("Page Navigation").click();
    ui.settle();
    assert!(thumbs(&ui).is_empty());
}

#[test]
fn the_pages_menu_lists_its_commands() {
    let mut ui = three_pages();
    let at = thumb_center(&ui, 1);
    ui.right_click_at(at);
    for name in [
        "Insert Blank Page",
        "Insert Duplicate Page",
        "Delete Page",
        "Move Page Up",
        "Move Page Down",
    ] {
        ui.h.get_by_label(name);
    }
}

#[test]
fn delete_and_move_are_greyed_when_they_cannot_apply() {
    // One page: nothing to delete or move.
    let mut ui = one_page();
    let at = thumb_center(&ui, 0);
    ui.right_click_at(at);
    assert!(menu_item_enabled(&ui, "Insert Blank Page"));
    assert!(menu_item_enabled(&ui, "Insert Duplicate Page"));
    assert!(!menu_item_enabled(&ui, "Delete Page"), "the last page cannot be deleted");
    assert!(!menu_item_enabled(&ui, "Move Page Up"));
    assert!(!menu_item_enabled(&ui, "Move Page Down"));
    // Three pages: the first cannot go up, the last cannot go down, the middle can do both.
    let mut ui = three_pages();
    for (page, up, down) in [(0, false, true), (1, true, true), (2, true, false)] {
        let at = thumb_center(&ui, page);
        ui.right_click_at(at);
        assert!(menu_item_enabled(&ui, "Delete Page"), "page {page}");
        assert_eq!(menu_item_enabled(&ui, "Move Page Up"), up, "page {page} up");
        assert_eq!(menu_item_enabled(&ui, "Move Page Down"), down, "page {page} down");
        ui.key(NONE_MODS, eframe::egui::Key::Escape);
    }
}

const NONE_MODS: eframe::egui::Modifiers = eframe::egui::Modifiers::NONE;

#[test]
fn insert_blank_page_adds_a_page_after_the_clicked_one_selects_it_and_is_one_undo_step() {
    let mut ui = three_pages();
    ui.app().go_page(2);
    ui.settle();
    // Right-click page 1 (not the current page): the command applies to that thumbnail.
    thumb_menu(&mut ui, 0, "Insert Blank Page");
    assert_eq!(ui.page_count(), 4);
    assert_eq!(current_page(&mut ui), 1, "the new page is shown");
    assert!(ui.doc().pages[1].shapes.is_empty());
    assert_eq!(ui.doc().pages[0].shapes.len(), 1);
    status_page_text(&ui, 2, 4);
    assert_eq!(thumbs(&ui).len(), 4);
    assert_eq!(ui.undo_steps(), 3, "two setup inserts and this one");
    ui.undo();
    assert_eq!(ui.page_count(), 3);
}

#[test]
fn the_add_page_card_ends_the_list_and_appends_a_blank_page() {
    let mut ui = three_pages();
    let card = crate::nav::probe::get().add_card.expect("the Add page card is drawn");
    let last = thumbs(&ui).last().unwrap().1;
    assert!(card.min.y > last.max.y, "below the last thumbnail");
    assert!((card.width() - last.width()).abs() < 0.5, "as wide as a thumbnail");
    ui.app().go_page(0);
    ui.settle();
    ui.h.get_by_label("Add page").click();
    ui.settle();
    assert_eq!(ui.page_count(), 4);
    assert_eq!(current_page(&mut ui), 3, "the new last page is shown");
    assert!(ui.doc().pages[3].shapes.is_empty());
    ui.undo();
    assert_eq!(ui.page_count(), 3);
}

#[test]
fn insert_duplicate_page_copies_the_page_and_its_shapes() {
    let mut ui = three_pages();
    thumb_menu(&mut ui, 0, "Insert Duplicate Page");
    assert_eq!(ui.page_count(), 4);
    assert_eq!(current_page(&mut ui), 1);
    assert_eq!(ui.doc().pages[1].shapes.len(), 1, "the copy has the shape");
    assert_eq!(ui.doc().pages[1].shapes[0].rect, ui.doc().pages[0].shapes[0].rect);
    assert_eq!(ui.doc().pages[2].shapes.len(), 0, "the old page 2 moved down");
    ui.undo();
    assert_eq!(ui.page_count(), 3);
}

#[test]
fn delete_page_removes_it_keeps_the_view_sensible_and_undo_restores_it() {
    let mut ui = three_pages();
    ui.app().go_page(2);
    ui.settle();
    // Deleting a page before the current one keeps the same page in view (now one earlier).
    thumb_menu(&mut ui, 0, "Delete Page");
    assert_eq!(ui.page_count(), 2);
    assert_eq!(current_page(&mut ui), 1);
    status_page_text(&ui, 2, 2);
    assert_eq!(thumbs(&ui).len(), 2);
    // Deleting the current last page falls back to the new last page.
    thumb_menu(&mut ui, 1, "Delete Page");
    assert_eq!(ui.page_count(), 1);
    assert_eq!(current_page(&mut ui), 0);
    ui.undo();
    assert_eq!(ui.page_count(), 2);
    ui.undo();
    assert_eq!(ui.page_count(), 3);
    assert_eq!(ui.doc().pages[0].shapes.len(), 1, "the page with the shape is back first");
}

#[test]
fn move_page_up_and_down_reorder_the_pages_and_follow_the_moved_page() {
    let mut ui = three_pages();
    let marker = ui.doc().pages[0].id;
    thumb_menu(&mut ui, 0, "Move Page Down");
    assert_eq!(ui.doc().pages[1].id, marker, "the page with the shape is second now");
    assert_eq!(current_page(&mut ui), 1, "the moved page stays in view");
    thumb_menu(&mut ui, 1, "Move Page Down");
    assert_eq!(ui.doc().pages[2].id, marker);
    assert_eq!(current_page(&mut ui), 2);
    thumb_menu(&mut ui, 2, "Move Page Up");
    assert_eq!(ui.doc().pages[1].id, marker);
    assert_eq!(current_page(&mut ui), 1);
    assert_eq!(ui.undo_steps(), 5, "two inserts and three moves, one step each");
    ui.undo();
    assert_eq!(ui.doc().pages[2].id, marker);
}

// ---- Rulers ------------------------------------------------------------------------------------------

#[test]
fn rulers_are_drawn_along_the_top_and_left_of_the_page_area() {
    let ui = one_page();
    let p = ui.h.state().probe.clone();
    let (h, _) = p.ruler_h.expect("horizontal ruler");
    let (v, _) = p.ruler_v.expect("vertical ruler");
    assert_eq!(h.height(), 18.0);
    assert_eq!(v.width(), 18.0);
    assert!(h.width() > 500.0 && v.height() > 300.0);
    assert!((h.min.y - v.min.y).abs() < 20.0 + 0.1, "both start at the top of the page area");
    assert_eq!(h.min.x, v.max.x, "the horizontal ruler starts where the vertical one ends");
    assert_eq!(h.max.y, v.min.y, "the vertical ruler starts under the horizontal one");
    // The page sits under both rulers' ends.
    assert!(p.origin.x > v.max.x && p.origin.y > h.max.y);
}

#[test]
fn the_rulers_marker_follows_the_cursor_and_disappears_off_the_canvas() {
    let mut ui = one_page();
    let at = ui.at_in(3.0, 4.0);
    ui.mouse_to(at);
    ui.settle();
    let p = ui.h.state().probe.clone();
    assert_eq!(p.ruler_h.unwrap().1, Some(at.x), "horizontal marker at the cursor's x");
    assert_eq!(p.ruler_v.unwrap().1, Some(at.y), "vertical marker at the cursor's y");
    let at2 = ui.at_in(5.5, 7.0);
    ui.mouse_to(at2);
    ui.settle();
    let p = ui.h.state().probe.clone();
    assert_eq!(p.ruler_h.unwrap().1, Some(at2.x));
    assert_eq!(p.ruler_v.unwrap().1, Some(at2.y));
    // Over the status bar the cursor is not on the page area.
    ui.mouse_to(Pos2::new(640.0, 795.0));
    ui.settle();
    let p = ui.h.state().probe.clone();
    assert_eq!(p.ruler_h.unwrap().1, None);
    assert_eq!(p.ruler_v.unwrap().1, None);
}

#[test]
fn view_rulers_hides_and_shows_both_rulers() {
    let mut ui = one_page();
    ui.click_tab("View");
    ui.h.get_by_label("Rulers").click();
    ui.settle();
    assert!(!ui.app().show.rulers);
    assert!(ui.h.state().probe.ruler_h.is_none() && ui.h.state().probe.ruler_v.is_none());
    ui.h.get_by_label("Rulers").click();
    ui.settle();
    assert!(ui.app().show.rulers);
    assert!(ui.h.state().probe.ruler_h.is_some() && ui.h.state().probe.ruler_v.is_some());
}

// ---- Status bar -----------------------------------------------------------------------------------------

#[test]
fn the_status_bar_shows_the_page_number_and_count() {
    let mut ui = one_page();
    status_page_text(&ui, 1, 1);
    ui.app().insert_page_after(0);
    ui.settle();
    status_page_text(&ui, 2, 2);
    ui.key(NONE_MODS, eframe::egui::Key::PageUp);
    status_page_text(&ui, 1, 2);
}

#[test]
fn the_status_bar_shows_the_cursor_position_in_inches_over_the_page() {
    let mut ui = one_page();
    let at = ui.at_in(2.0, 3.0);
    ui.mouse_to(at);
    ui.settle();
    let (x, y) = ui.app().cursor_in().expect("cursor position");
    assert!((x - 2.0).abs() < 0.03 && (y - 3.0).abs() < 0.03, "({x}, {y})");
    ui.h.get_by_label(&format!("{x:.2}, {y:.2} in."));
    // Off the page area (over the status bar) the position is blank.
    ui.mouse_to(Pos2::new(640.0, 795.0));
    ui.settle();
    assert!(ui.app().cursor_in().is_none());
    assert!(ui.h.query_by_label_contains(" in.").is_none());
}

// ---- Zoom -------------------------------------------------------------------------------------------------

const MINUS: &str = "\u{2212}";

fn set_zoom(ui: &mut TestUi, zoom: f64) {
    ui.app().zoom_mode = ZoomMode::Custom;
    ui.app().zoom = zoom;
    ui.settle();
}

fn percent_label(ui: &mut TestUi) {
    let z = ui.app().zoom;
    ui.h.get_by_label(&format!("{:.0}%", z * 100.0));
}

#[test]
fn the_status_bar_starts_at_whole_page_and_shows_the_zoom_percentage() {
    let mut ui = one_page();
    assert_eq!(ui.app().zoom_mode, ZoomMode::WholePage);
    assert!(ui.ribbon("Whole Page").enabled);
    let z = ui.app().zoom;
    assert!(z > 0.3 && z < 1.0, "a Letter page fits an 800 px window below 100%: {z}");
    percent_label(&mut ui);
}

#[test]
fn zoom_in_and_zoom_out_step_through_the_zoom_levels() {
    let mut ui = one_page();
    set_zoom(&mut ui, 1.0);
    percent_label(&mut ui);
    ui.h.get_by_label("+").click();
    ui.settle();
    assert_eq!(ui.app().zoom, 1.25);
    assert_eq!(ui.app().zoom_mode, ZoomMode::Custom);
    ui.h.get_by_label("125%");
    ui.h.get_by_label("+").click();
    ui.settle();
    assert_eq!(ui.app().zoom, 1.5);
    ui.h.get_by_label(MINUS).click();
    ui.settle();
    ui.h.get_by_label(MINUS).click();
    ui.settle();
    assert_eq!(ui.app().zoom, 1.0);
    ui.h.get_by_label(MINUS).click();
    ui.settle();
    assert_eq!(ui.app().zoom, 0.75);
    ui.h.get_by_label("75%");
    assert!(!ui.can_undo(), "zooming is not an edit");
}

#[test]
fn zoom_in_from_a_fit_mode_goes_to_the_next_level_above_the_fitted_zoom() {
    let mut ui = one_page();
    let fitted = ui.app().zoom;
    ui.h.get_by_label("+").click();
    ui.settle();
    assert_eq!(ui.app().zoom_mode, ZoomMode::Custom);
    assert!(ui.app().zoom > fitted + 1e-6, "zoomed in from {fitted} to {}", ui.app().zoom);
}

#[test]
fn zoom_buttons_stop_at_the_largest_and_smallest_zoom() {
    let mut ui = one_page();
    set_zoom(&mut ui, 3.0);
    for _ in 0..4 {
        ui.h.get_by_label("+").click();
        ui.settle();
    }
    assert_eq!(ui.app().zoom, MAX_ZOOM);
    ui.h.get_by_label("400%");
    set_zoom(&mut ui, 0.25);
    for _ in 0..4 {
        ui.h.get_by_label(MINUS).click();
        ui.settle();
    }
    assert_eq!(ui.app().zoom, MIN_ZOOM);
    ui.h.get_by_label("10%");
}

#[test]
fn there_is_no_zoom_slider_only_the_pill_with_minus_percent_plus() {
    let ui = one_page();
    assert!(ui.h.query_by_role(Role::Slider).is_none(), "the slider is gone");
    ui.h.get_by_label("+");
    ui.h.get_by_label(MINUS);
}

#[test]
fn the_zoom_pill_percentage_opens_a_list_of_zoom_levels() {
    let mut ui = one_page();
    percent_label(&mut ui);
    let pct = format!("{:.0}%", ui.app().zoom * 100.0);
    ui.h.get_by_label(&pct).click();
    ui.settle();
    for item in ["Fit page", "Fit width", "50%", "75%", "150%", "200%"] {
        ui.h.get_by_label(item);
    }
    ui.h.get_by_label("150%").click();
    ui.settle();
    assert_eq!(ui.app().zoom, 1.5);
    assert_eq!(ui.app().zoom_mode, ZoomMode::Custom);
    ui.h.get_by_label("150%");
    // Fit width, then Fit page.
    ui.h.get_by_label("150%").click();
    ui.settle();
    ui.h.get_by_label("Fit width").click();
    ui.settle();
    assert_eq!(ui.app().zoom_mode, ZoomMode::PageWidth);
    let pct = format!("{:.0}%", ui.app().zoom * 100.0);
    ui.h.get_by_label(&pct).click();
    ui.settle();
    ui.h.get_by_label("Fit page").click();
    ui.settle();
    assert_eq!(ui.app().zoom_mode, ZoomMode::WholePage);
}

#[test]
fn the_whole_page_button_fits_the_page_in_the_window() {
    let mut ui = one_page();
    set_zoom(&mut ui, 3.0);
    ui.click_ribbon("Whole Page");
    assert_eq!(ui.app().zoom_mode, ZoomMode::WholePage);
    let p = ui.h.state().probe.clone();
    let page = Vec2::new(8.5 * 96.0 * p.zoom as f32, 11.0 * 96.0 * p.zoom as f32);
    assert!(p.zoom < 1.0, "the page is shown smaller than life");
    assert!(p.origin.x >= 0.0 && p.origin.x + page.x <= 1280.0, "fits across");
    assert!(p.origin.y >= 0.0 && p.origin.y + page.y <= 800.0 - 26.0, "fits down");
    percent_label(&mut ui);
}

#[test]
fn whole_page_and_page_width_follow_the_window_size() {
    let mut ui = one_page();
    let big = ui.app().zoom;
    ui.h.set_size(Vec2::new(900.0, 560.0));
    ui.settle();
    let small = ui.app().zoom;
    assert!(small < big, "a smaller window gives a smaller whole-page zoom ({small} < {big})");
    assert_eq!(ui.app().zoom_mode, ZoomMode::WholePage);
    let p = ui.h.state().probe.clone();
    assert!(p.origin.y + 11.0 * 96.0 * p.zoom as f32 <= 560.0 - 26.0, "still fits down");
    // Page Width (View tab): fits the width, so a wider window zooms in further.
    ui.click_tab("View");
    ui.click_ribbon("Page Width");
    assert_eq!(ui.app().zoom_mode, ZoomMode::PageWidth);
    let narrow = ui.app().zoom;
    ui.h.set_size(Vec2::new(1280.0, 800.0));
    ui.settle();
    assert!(ui.app().zoom > narrow, "page width zoom grows with the window");
    let p = ui.h.state().probe.clone();
    assert!(p.origin.x + 8.5 * 96.0 * p.zoom as f32 <= 1280.0, "the page width fits");
}

#[test]
fn the_view_buttons_single_page_is_selected_and_two_page_spread_is_greyed() {
    let mut ui = one_page();
    let single = ui.ribbon("Single Page");
    assert!(single.enabled && single.selected);
    let two = ui.ribbon("Two-Page Spread: coming soon");
    assert!(!two.enabled, "greyed");
    let mode = ui.app().zoom_mode;
    ui.click_ribbon("Two-Page Spread: coming soon");
    assert_eq!(ui.app().zoom_mode, mode, "does nothing");
    assert!(!ui.can_undo());
    assert!(ui.ribbon("Single Page").selected);
}

#[test]
fn zoom_widgets_do_not_change_the_document_or_the_selection() {
    let mut ui = one_page();
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A]);
    ui.h.get_by_label("+").click();
    ui.settle();
    ui.click_ribbon("Whole Page");
    assert_eq!(ui.selected(), vec![A]);
    assert_eq!(ui.rect_of(A), Rect::new(914_400, 914_400, 2 * 914_400, 914_400));
    assert!(!ui.can_undo());
}

#[test]
fn the_status_bar_shows_page_size_cursor_and_zoom_items() {
    let mut ui = one_page();
    status_page_text(&ui, 1, 1);
    // Nothing selected: no object size text.
    assert!(ui.app().selection_size_text().is_empty());
    ui.app().set_selection(vec![A]);
    ui.settle();
    ui.h.get_by_label("2.00 x 1.00 in.");
    ui.h.get_by_label("Single Page");
}

#[test]
fn whole_page_zoom_leaves_only_a_small_margin_on_the_tight_side() {
    let mut ui = one_page();
    assert_eq!(ui.app().zoom_mode, ZoomMode::WholePage);
    let p = ui.h.state().probe.clone();
    let (h, _) = p.ruler_h.expect("horizontal ruler");
    let (v, _) = p.ruler_v.expect("vertical ruler");
    let tight = (p.origin.x - v.max.x).min(p.origin.y - h.max.y);
    assert!((8.0..=10.0).contains(&tight), "the page keeps an 8-10 px margin: {tight}");
}
