//! Canvas mouse behaviour: select, move, resize, marquee, create. Every test sends real
//! pointer events (press, move, release, one frame each) through the whole editor UI.

use eframe::egui;
use super::support::*;
use crate::app::Tool;
use eframe::egui::Vec2;
use kernpage_core::geometry::px_to_emu;
use kernpage_core::{Rect, ShapeKind};

const A: u64 = 1;
const B: u64 = 2;

/// Two filled rectangles: A at (1, 1) 2x1.5 in, B at (4, 3) 2x1.5 in.
fn two_boxes() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
    ]))
}

/// A screen distance in pixels as EMU at the current zoom.
fn emu(ui: &TestUi, px: f32) -> i64 {
    px_to_emu(f64::from(px), ui.h.state().probe.zoom)
}

/// Rectangles agree to within one screen pixel on every side.
fn assert_rect_near(ui: &TestUi, got: Rect, want: Rect) {
    let tol = emu(ui, 1.5);
    for (name, g, w) in [
        ("x", got.x, want.x),
        ("y", got.y, want.y),
        ("w", got.w, want.w),
        ("h", got.h, want.h),
    ] {
        assert!((g - w).abs() <= tol, "{name}: got {g}, want {w} (+-{tol}); {got:?} vs {want:?}");
    }
}

fn moved(r: Rect, ui: &TestUi, d: Vec2) -> Rect {
    Rect::new(r.x + emu(ui, d.x), r.y + emu(ui, d.y), r.w, r.h)
}

// ---- select / deselect ----------------------------------------------------------------

#[test]
fn click_selects_a_shape_and_empty_space_deselects() {
    let mut ui = two_boxes();
    assert!(ui.selected().is_empty());
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A]);
    ui.click_at(ui.center_of(B));
    assert_eq!(ui.selected(), vec![B], "clicking another shape moves the selection");
    ui.click_at(ui.at_in(0.5, 8.0));
    assert!(ui.selected().is_empty(), "clicking empty page deselects");
    assert!(!ui.can_undo(), "selecting never edits the document");
}

#[test]
fn shift_click_adds_and_removes_from_the_selection() {
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    let mut sel = ui.selected();
    sel.sort();
    assert_eq!(sel, vec![A, B]);
    ui.click_mods(ui.center_of(A), SHIFT);
    assert_eq!(ui.selected(), vec![B], "Shift+click on a selected shape removes it");
}

#[test]
fn escape_clears_the_selection() {
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(A));
    ui.key(egui::Modifiers::NONE, egui::Key::Escape);
    assert!(ui.selected().is_empty());
}

// ---- move ---------------------------------------------------------------------------

#[test]
fn dragging_a_shape_moves_it_to_the_drop_point_as_one_undo_step() {
    let mut ui = two_boxes();
    let orig = ui.rect_of(A);
    let rev = ui.revision();
    let d = Vec2::new(120.0, 70.0);
    let from = ui.center_of(A);
    ui.drag(from, from + d);

    let want = moved(orig, &ui, d);
    assert_rect_near(&ui, ui.rect_of(A), want);
    assert_eq!(ui.rect_of(B), Rect::new(
        kernpage_core::geometry::in_to_emu(4.0),
        kernpage_core::geometry::in_to_emu(3.0),
        kernpage_core::geometry::in_to_emu(2.0),
        kernpage_core::geometry::in_to_emu(1.5),
    ), "the other shape does not move");
    assert_eq!(ui.selected(), vec![A]);
    assert_eq!(ui.revision(), rev + 1, "one command for the whole drag");

    // The shape is drawn at the drop position once the UI has settled.
    assert!((ui.painted(A).min - ui.screen_rect_of(ui.rect_of(A)).min).length() < 1.0);

    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.rect_of(A), orig, "one Ctrl+Z restores the original place");
    assert!(!ui.can_undo(), "the drag was a single undo step");
    ui.key(CTRL, egui::Key::Y);
    assert_rect_near(&ui, ui.rect_of(A), want);
}

#[test]
fn the_release_frame_paints_the_shape_at_the_drop_position() {
    // No flash: the move command runs after painting on the release frame, so that frame
    // must still draw the shape at the drop position, not back at its old place.
    let mut ui = two_boxes();
    let orig = ui.rect_of(A);
    let d = Vec2::new(150.0, 90.0);
    let from = ui.center_of(A);
    ui.drag_begin(from, from + d, egui::Modifiers::NONE);
    let during = ui.painted(A);
    ui.release_primary();
    let on_release = ui.painted(A);

    let drop = ui.screen_rect_of(moved(orig, &ui, d));
    assert!((during.min - drop.min).length() < 1.5, "preview follows the pointer");
    assert!(
        (on_release.min - drop.min).length() < 1.5,
        "release frame painted at {:?}, drop point is {:?}",
        on_release.min,
        drop.min
    );
    assert_ne!(ui.rect_of(A), orig, "the move was applied on the release frame");
}

#[test]
fn a_click_without_movement_does_not_move_or_edit() {
    let mut ui = two_boxes();
    let rev = ui.revision();
    ui.click_at(ui.center_of(A));
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.revision(), rev);
}

#[test]
fn dragging_a_multi_selection_moves_every_selected_shape() {
    let mut ui = two_boxes();
    let (a0, b0) = (ui.rect_of(A), ui.rect_of(B));
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    let d = Vec2::new(-60.0, 80.0);
    let from = ui.center_of(A);
    ui.drag(from, from + d);
    assert_rect_near(&ui, ui.rect_of(A), moved(a0, &ui, d));
    assert_rect_near(&ui, ui.rect_of(B), moved(b0, &ui, d));
    let mut sel = ui.selected();
    sel.sort();
    assert_eq!(sel, vec![A, B], "the multi-selection survives the drag");
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.rect_of(A), a0);
    assert_eq!(ui.rect_of(B), b0);
    assert!(!ui.can_undo(), "moving both shapes is one undo step");
}

// ---- resize -------------------------------------------------------------------------

#[test]
fn dragging_the_bottom_right_handle_resizes_the_shape() {
    let mut ui = two_boxes();
    let orig = ui.rect_of(A);
    ui.click_at(ui.center_of(A));
    let rev = ui.revision();
    let d = Vec2::new(60.0, 40.0);
    let corner = ui.corner_of(A, 1, 1);
    ui.drag(corner, corner + d);
    let want = Rect::new(orig.x, orig.y, orig.w + emu(&ui, d.x), orig.h + emu(&ui, d.y));
    assert_rect_near(&ui, ui.rect_of(A), want);
    assert_eq!(ui.revision(), rev + 1);
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.rect_of(A), orig);
}

#[test]
fn dragging_the_top_left_handle_moves_the_corner_and_keeps_the_opposite_one() {
    let mut ui = two_boxes();
    let orig = ui.rect_of(A);
    ui.click_at(ui.center_of(A));
    let d = Vec2::new(30.0, 20.0);
    let corner = ui.corner_of(A, 0, 0);
    ui.drag(corner, corner + d);
    let r = ui.rect_of(A);
    assert_eq!((r.right(), r.bottom()), (orig.right(), orig.bottom()));
    assert_rect_near(
        &ui,
        r,
        Rect::new(orig.x + emu(&ui, d.x), orig.y + emu(&ui, d.y), orig.w - emu(&ui, d.x), orig.h - emu(&ui, d.y)),
    );
}

#[test]
fn the_release_frame_paints_the_resized_shape() {
    let mut ui = two_boxes();
    let orig = ui.rect_of(A);
    ui.click_at(ui.center_of(A));
    let d = Vec2::new(80.0, 50.0);
    let corner = ui.corner_of(A, 1, 1);
    ui.drag_begin(corner, corner + d, egui::Modifiers::NONE);
    ui.release_primary();
    let want = ui.screen_rect_of(Rect::new(
        orig.x,
        orig.y,
        orig.w + emu(&ui, d.x),
        orig.h + emu(&ui, d.y),
    ));
    let got = ui.painted(A);
    assert!((got.max - want.max).length() < 1.5, "painted {got:?}, want {want:?}");
}

// ---- marquee ------------------------------------------------------------------------

#[test]
fn marquee_around_both_shapes_selects_both() {
    let mut ui = two_boxes();
    ui.drag(ui.at_in(0.5, 0.5), ui.at_in(6.5, 5.0));
    let mut sel = ui.selected();
    sel.sort();
    assert_eq!(sel, vec![A, B]);
    assert!(!ui.can_undo(), "a marquee does not edit the document");
}

#[test]
fn marquee_around_one_shape_selects_only_that_shape() {
    let mut ui = two_boxes();
    ui.drag(ui.at_in(0.5, 0.5), ui.at_in(3.5, 3.0));
    assert_eq!(ui.selected(), vec![A]);
}

#[test]
fn shift_marquee_adds_to_the_existing_selection() {
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(B));
    ui.drag_mods(ui.at_in(0.5, 0.5), ui.at_in(3.5, 3.0), SHIFT);
    let mut sel = ui.selected();
    sel.sort();
    assert_eq!(sel, vec![A, B]);
}

#[test]
fn marquee_on_empty_space_deselects_what_was_selected() {
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(A));
    ui.drag(ui.at_in(0.5, 6.0), ui.at_in(3.0, 8.0));
    assert!(ui.selected().is_empty());
}

// ---- create with a tool -------------------------------------------------------------

#[test]
fn drawing_with_the_rectangle_tool_creates_a_rectangle_in_one_undo_step() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::Rectangle);
    let (from, to) = (ui.at_in(1.0, 6.0), ui.at_in(3.0, 7.5));
    ui.drag(from, to);
    assert_eq!(ui.shapes().len(), 3);
    let s = ui.shapes()[2].clone();
    assert!(matches!(s.kind, ShapeKind::Rectangle));
    assert_rect_near(
        &ui,
        s.rect,
        Rect::new(
            kernpage_core::geometry::in_to_emu(1.0),
            kernpage_core::geometry::in_to_emu(6.0),
            kernpage_core::geometry::in_to_emu(2.0),
            kernpage_core::geometry::in_to_emu(1.5),
        ),
    );
    assert_eq!(ui.selected(), vec![s.id], "the new shape is selected");
    assert!(ui.app().tool == Tool::Select, "the tool returns to Select");
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.shapes().len(), 2);
    assert!(!ui.can_undo());
}

#[test]
fn drawing_with_the_oval_tool_creates_an_ellipse() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::Oval);
    ui.drag(ui.at_in(1.0, 6.0), ui.at_in(2.5, 7.0));
    assert!(matches!(ui.shapes()[2].kind, ShapeKind::Ellipse));
}

#[test]
fn drawing_with_the_text_box_tool_creates_a_text_box_and_starts_editing() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::TextBox);
    ui.drag(ui.at_in(1.0, 6.0), ui.at_in(4.0, 7.0));
    assert_eq!(ui.shapes().len(), 3);
    let s = ui.shapes()[2].clone();
    assert!(s.is_text_box());
    assert!(ui.app().edit.as_ref().is_some_and(|e| e.id == s.id), "the new box is in edit mode");
}

#[test]
fn drawing_with_the_line_tool_keeps_the_drag_direction() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::Line);
    // Down-left to up-right: a line that runs bottom-left to top-right (one flip set).
    ui.drag(ui.at_in(2.0, 8.0), ui.at_in(4.0, 6.5));
    let s = ui.shapes()[2].clone();
    assert!(matches!(s.kind, ShapeKind::Line));
    assert!(s.flip_h != s.flip_v, "flips {:?}", (s.flip_h, s.flip_v));
}

#[test]
fn clicking_with_a_drawing_tool_creates_a_default_sized_shape() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::Rectangle);
    ui.click_at(ui.at_in(1.0, 6.0));
    assert_eq!(ui.shapes().len(), 3);
    let r = ui.shapes()[2].rect;
    assert_eq!(r.w, kernpage_core::geometry::in_to_emu(1.5));
    assert_eq!(r.h, kernpage_core::geometry::in_to_emu(1.0));
}

// ---- sample file ---------------------------------------------------------------------

#[test]
fn a_shape_of_a_real_pub_file_can_be_dragged_and_undone() {
    let mut ui = TestUi::with_sample("hello.pub");
    let id = ui.shapes()[0].id;
    let orig = ui.rect_of(id);
    let from = ui.center_of(id);
    ui.drag(from, from + Vec2::new(100.0, 100.0));
    assert_ne!(ui.rect_of(id), orig);
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.rect_of(id), orig);
}


// ---- double-click, right-click, typing ---------------------------------------------------

#[test]
fn right_clicking_a_shape_selects_it_and_its_menu_deletes_it() {
    use egui_kittest::kittest::Queryable;
    let mut ui = two_boxes();
    ui.right_click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A], "right-click selects the shape under the pointer");
    ui.h.get_by_label("Bring to Front");
    ui.h.get_by_label("Delete Object").click();
    ui.settle();
    assert_eq!(ui.shapes().len(), 1);
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.shapes().len(), 2, "Delete Object is one undo step");
}

#[test]
fn right_clicking_empty_page_offers_paste_and_zoom_only() {
    use egui_kittest::kittest::Queryable;
    let mut ui = two_boxes();
    ui.right_click_at(ui.at_in(0.5, 8.0));
    assert!(ui.selected().is_empty());
    ui.h.get_by_label("Paste");
    ui.h.get_by_label("Zoom");
    assert!(ui.h.query_by_label("Delete Object").is_none());
}

#[test]
fn double_clicking_a_text_box_edits_it_and_typed_text_is_committed() {
    let mut ui = TestUi::with_sample("hello.pub");
    let id = ui.shapes()[0].id;
    ui.double_click_at(ui.center_of(id));
    assert!(ui.app().edit.as_ref().is_some_and(|e| e.id == id), "edit mode started");
    ui.type_text("XYZ");
    // Clicking empty page commits the text as one undoable change.
    ui.click_at(ui.at_in(0.3, 10.5));
    assert!(ui.app().edit.is_none());
    let text = match &ui.shape(id).kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join("\n"),
        _ => unreachable!(),
    };
    assert!(text.contains("XYZ"), "text is now {text:?}");
}

// ---- batch 2: toggle, groups, handles, line, menu items, cursor, wheel, scratch -----------

use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Shape, TableCell};

fn sorted(mut v: Vec<u64>) -> Vec<u64> {
    v.sort();
    v
}

/// Ids of the page's shapes in stacking order (bottom to top).
fn order(ui: &TestUi) -> Vec<u64> {
    ui.shapes().iter().map(|s| s.id).collect()
}

fn three_boxes() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
        rect_shape(1.0, 6.0, 2.0, 1.5),
    ]))
}

/// The entry `label` of the open popup menu: the lowest node with that label, because a ribbon
/// button can carry the same name (the Home tab's small Arrange buttons).
fn popup_item<'a>(ui: &'a TestUi, label: &'a str) -> egui_kittest::Node<'a> {
    use egui_kittest::kittest::Queryable;
    let mut all: Vec<_> = ui.h.get_all_by_label(label).collect();
    all.sort_by(|a, b| a.rect().top().total_cmp(&b.rect().top()));
    all.pop().unwrap_or_else(|| panic!("no menu entry {label:?}"))
}

/// Right-click shape `id` and click the menu entry `label`.
fn menu(ui: &mut TestUi, id: u64, label: &str) {
    use egui_kittest::kittest::Queryable;
    ui.right_click_at(ui.center_of(id));
    popup_item(ui, label).click();
    ui.settle();
}

/// A and B selected together (A clicked, B shift-clicked).
fn select_both(ui: &mut TestUi) {
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), SHIFT);
    assert_eq!(sorted(ui.selected()), vec![A, B]);
}

#[test]
fn ctrl_click_toggles_selection() {
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(A));
    ui.click_mods(ui.center_of(B), CTRL);
    assert_eq!(sorted(ui.selected()), vec![A, B], "Ctrl+click adds");
    ui.click_mods(ui.center_of(A), CTRL);
    assert_eq!(ui.selected(), vec![B], "Ctrl+click on a selected shape removes it");
}

#[test]
fn dragging_a_group_moves_its_members() {
    let mut ui = two_boxes();
    let (a0, b0) = (ui.rect_of(A), ui.rect_of(B));
    select_both(&mut ui);
    ui.app().group_selection();
    ui.settle();
    assert_eq!(ui.shapes().len(), 3);
    let g = ui.shapes().iter().find(|s| matches!(s.kind, ShapeKind::Group)).unwrap().id;
    ui.click_at(ui.at_in(0.5, 8.0));
    // Clicking a member selects the whole group.
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![g]);
    let d = Vec2::new(40.0, 60.0);
    let from = ui.center_of(A);
    ui.drag(from, from + d);
    assert_rect_near(&ui, ui.rect_of(A), moved(a0, &ui, d));
    assert_rect_near(&ui, ui.rect_of(B), moved(b0, &ui, d));
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.rect_of(A), a0);
    assert_eq!(ui.rect_of(B), b0);
    assert!(ui.can_undo(), "only the move was undone, the grouping remains");
}

/// Select A, drag the handle at fraction (fx, fy) of its frame by `d`.
fn drag_handle(ui: &mut TestUi, fx: f64, fy: f64, d: Vec2) -> (Rect, Rect) {
    ui.click_at(ui.center_of(A));
    let orig = ui.rect_of(A);
    let at = ui.at_emu(
        orig.x + (orig.w as f64 * fx) as i64,
        orig.y + (orig.h as f64 * fy) as i64,
    );
    ui.drag(at, at + d);
    (orig, ui.rect_of(A))
}

#[test]
fn edge_handles_resize_one_dimension() {
    let d = Vec2::new(40.0, 30.0);
    // East: only the width grows.
    let mut ui = two_boxes();
    let (o, r) = drag_handle(&mut ui, 1.0, 0.5, d);
    assert_rect_near(&ui, r, Rect::new(o.x, o.y, o.w + emu(&ui, d.x), o.h));
    // West: left edge moves, right edge stays, vertical untouched.
    let mut ui = two_boxes();
    let (o, r) = drag_handle(&mut ui, 0.0, 0.5, d);
    assert_eq!(r.right(), o.right());
    assert_rect_near(&ui, r, Rect::new(o.x + emu(&ui, d.x), o.y, o.w - emu(&ui, d.x), o.h));
    // South: only the height grows.
    let mut ui = two_boxes();
    let (o, r) = drag_handle(&mut ui, 0.5, 1.0, d);
    assert_rect_near(&ui, r, Rect::new(o.x, o.y, o.w, o.h + emu(&ui, d.y)));
    // North: top edge moves, bottom stays, horizontal untouched.
    let mut ui = two_boxes();
    let (o, r) = drag_handle(&mut ui, 0.5, 0.0, d);
    assert_eq!(r.bottom(), o.bottom());
    assert_rect_near(&ui, r, Rect::new(o.x, o.y + emu(&ui, d.y), o.w, o.h - emu(&ui, d.y)));
}

#[test]
fn resize_stops_at_minimum_size() {
    let mut ui = two_boxes();
    // Drag the bottom-right corner far past the top-left one.
    let (o, r) = drag_handle(&mut ui, 1.0, 1.0, Vec2::new(-250.0, -200.0));
    assert_eq!((r.x, r.y), (o.x, o.y), "the opposite corner stays put");
    assert_eq!((r.w, r.h), (in_to_emu(0.1), in_to_emu(0.1)), "stops at the minimum size");
}

#[test]
fn line_end_handle_moves_the_end_and_has_no_minimum() {
    let mut line = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(1.0)),
        ShapeKind::Line,
    );
    line.line = Some(kernpage_core::Stroke { color: kernpage_core::Rgb(0, 0, 0), width_emu: 9_525 });
    let mut ui = TestUi::with_doc(letter_doc(vec![line]));
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.selected(), vec![A], "a line is selected by clicking on it");
    let o = ui.rect_of(A);
    let d = Vec2::new(30.0, 20.0);
    let end = ui.corner_of(A, 1, 1);
    ui.drag(end, end + d);
    let r = ui.rect_of(A);
    assert_eq!((r.x, r.y), (o.x, o.y), "the start end does not move");
    assert_rect_near(&ui, r, Rect::new(o.x, o.y, o.w + emu(&ui, d.x), o.h + emu(&ui, d.y)));
    // Drag the end up past the start's height: a horizontal line of zero height, not 0.1 in.
    let end = ui.corner_of(A, 1, 1);
    ui.drag(end, ui.at_emu(r.right(), r.y - in_to_emu(0.3)));
    assert_eq!(ui.rect_of(A).h, 0, "lines have no minimum size");
}

#[test]
fn resizing_a_group_scales_members() {
    let mut ui = two_boxes();
    select_both(&mut ui);
    ui.app().group_selection();
    ui.settle();
    let g = ui.selected()[0];
    let (a0, b0, g0) = (ui.rect_of(A), ui.rect_of(B), ui.rect_of(g));
    let d = Vec2::new(60.0, 40.0);
    let corner = ui.corner_of(g, 1, 1);
    ui.drag(corner, corner + d);
    let g1 = ui.rect_of(g);
    assert_rect_near(&ui, g1, Rect::new(g0.x, g0.y, g0.w + emu(&ui, d.x), g0.h + emu(&ui, d.y)));
    use kernpage_core::geometry::scale_into;
    assert_eq!(ui.rect_of(A), scale_into(a0, g0, g1), "member A scales with the group");
    assert_eq!(ui.rect_of(B), scale_into(b0, g0, g1), "member B scales with the group");
    assert_ne!(ui.rect_of(B), b0);
}

#[test]
fn tiny_drag_with_a_tool_uses_default_size() {
    let mut ui = two_boxes();
    ui.set_tool(Tool::Rectangle);
    let from = ui.at_in(1.0, 6.0);
    ui.drag(from, from + Vec2::new(7.0, 7.0));
    assert_eq!(ui.shapes().len(), 3);
    let r = ui.shapes()[2].rect;
    assert_eq!((r.w, r.h), (in_to_emu(1.5), in_to_emu(1.0)));
}

#[test]
fn shift_constrains_line_angle() {
    let line = |ui: &mut TestUi, to: (f64, f64)| {
        ui.set_tool(Tool::Line);
        ui.drag_mods(ui.at_in(2.0, 6.0), ui.at_in(to.0, to.1), SHIFT);
        ui.shapes().last().unwrap().rect
    };
    let mut ui = two_boxes();
    assert_eq!(line(&mut ui, (5.0, 6.3)).h, 0, "nearly horizontal snaps to 0 degrees");
    let mut ui = two_boxes();
    assert_eq!(line(&mut ui, (2.3, 8.0)).w, 0, "nearly vertical snaps to 90 degrees");
    let mut ui = two_boxes();
    let r = line(&mut ui, (4.5, 8.3));
    assert_eq!(r.w, r.h, "in between snaps to 45 degrees");
    assert!(r.w > 0);
}

#[test]
fn right_click_in_multi_selection_keeps_it() {
    use egui_kittest::kittest::Queryable;
    let mut ui = two_boxes();
    select_both(&mut ui);
    ui.right_click_at(ui.center_of(A));
    assert_eq!(sorted(ui.selected()), vec![A, B], "the multi-selection survives");
    ui.h.get_by_label("Delete Objects");
}

#[test]
fn context_menu_cut() {
    let mut ui = two_boxes();
    menu(&mut ui, A, "Cut");
    assert_eq!(order(&ui), vec![B], "the shape is removed");
    assert_eq!(ui.app().clipboard.len(), 1, "and held for Paste");
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(order(&ui), vec![A, B]);
}

#[test]
fn context_menu_copy_then_paste() {
    use egui_kittest::kittest::Queryable;
    let mut ui = two_boxes();
    let rev = ui.revision();
    menu(&mut ui, A, "Copy");
    assert_eq!(ui.revision(), rev, "Copy edits nothing");
    assert_eq!(ui.app().clipboard.len(), 1);
    ui.right_click_at(ui.center_of(B));
    ui.h.get_by_label("Paste").click();
    ui.settle();
    assert_eq!(ui.shapes().len(), 3);
    let new = ui.shapes()[2].clone();
    assert_eq!(ui.selected(), vec![new.id], "the pasted copy is selected");
    let a = ui.rect_of(A);
    assert_eq!(new.rect, Rect::new(a.x + in_to_emu(0.25), a.y + in_to_emu(0.25), a.w, a.h));
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.shapes().len(), 2, "Paste is one undo step");
}

#[test]
fn context_menu_paste_is_disabled_with_an_empty_clipboard() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut ui = two_boxes();
    ui.right_click_at(ui.center_of(A));
    assert!(ui.h.get_by_label("Paste").accesskit_node().is_disabled());
}

#[test]
fn context_menu_bring_forward() {
    let mut ui = three_boxes();
    menu(&mut ui, A, "Bring Forward");
    assert_eq!(order(&ui), vec![2, 1, 3], "one step up");
}

#[test]
fn context_menu_bring_to_front() {
    let mut ui = three_boxes();
    menu(&mut ui, A, "Bring to Front");
    assert_eq!(order(&ui), vec![2, 3, 1]);
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(order(&ui), vec![1, 2, 3]);
}

#[test]
fn context_menu_send_backward() {
    let mut ui = three_boxes();
    menu(&mut ui, 3, "Send Backward");
    assert_eq!(order(&ui), vec![1, 3, 2], "one step down");
}

#[test]
fn context_menu_send_to_back() {
    let mut ui = three_boxes();
    menu(&mut ui, 3, "Send to Back");
    assert_eq!(order(&ui), vec![3, 1, 2]);
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(order(&ui), vec![1, 2, 3]);
}

#[test]
fn context_menu_group_and_ungroup() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut ui = two_boxes();
    select_both(&mut ui);
    menu(&mut ui, A, "Group");
    assert_eq!(ui.shapes().len(), 3);
    let g = ui.shapes().iter().find(|s| matches!(s.kind, ShapeKind::Group)).expect("a group").id;
    assert_eq!(ui.shape(A).group, Some(g));
    assert_eq!(ui.shape(B).group, Some(g));
    assert_eq!(ui.selected(), vec![g], "the group is selected");

    ui.right_click_at(ui.center_of(A));
    assert!(popup_item(&ui, "Group").accesskit_node().is_disabled(), "a lone group cannot be grouped");
    popup_item(&ui, "Ungroup").click();
    ui.settle();
    assert_eq!(ui.shapes().len(), 2, "the group object is gone");
    assert_eq!((ui.shape(A).group, ui.shape(B).group), (None, None));
    assert_eq!(sorted(ui.selected()), vec![A, B], "the members end up selected");
}

#[test]
fn context_menu_delete_objects_removes_a_multi_selection() {
    let mut ui = two_boxes();
    select_both(&mut ui);
    menu(&mut ui, A, "Delete Objects");
    assert!(ui.shapes().is_empty());
    ui.key(CTRL, egui::Key::Z);
    assert_eq!(ui.shapes().len(), 2, "one undo step brings both back");
}

/// A 2x2 table at (4, 2), 4 x 2 in: cells are 2 x 1 in, row-major. Box A is id 1, table is 2.
fn table_doc() -> TestUi {
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

#[test]
fn double_click_table_cell_edits_it() {
    let mut ui = table_doc();
    ui.double_click_at(ui.at_in(7.0, 3.5)); // bottom-right cell
    let e = ui.app().edit.as_ref().expect("edit mode");
    assert_eq!((e.id, e.cell), (B, Some(3)));
    let mut ui = table_doc();
    ui.double_click_at(ui.at_in(5.0, 2.5)); // top-left cell
    assert_eq!(ui.app().edit.as_ref().map(|e| e.cell), Some(Some(0)));
}

#[test]
fn click_selected_table_cell_edits_it() {
    let mut ui = table_doc();
    ui.click_at(ui.at_in(7.0, 2.5));
    assert_eq!(ui.selected(), vec![B]);
    assert!(ui.app().edit.is_none(), "the first click only selects the table");
    ui.click_at(ui.at_in(7.0, 2.5)); // top-right cell
    let e = ui.app().edit.as_ref().expect("edit mode");
    assert_eq!((e.id, e.cell), (B, Some(1)));
}

#[test]
fn cursor_icon_follows_hover() {
    use egui::CursorIcon as C;
    let mut ui = two_boxes();
    ui.click_at(ui.center_of(A));
    assert_eq!(ui.cursor_icon_at(ui.at_in(0.5, 8.0)), C::Default, "empty page");
    assert_eq!(ui.cursor_icon_at(ui.center_of(B)), C::Move, "over an object");
    assert_eq!(ui.cursor_icon_at(ui.corner_of(A, 1, 1)), C::ResizeNwSe);
    assert_eq!(ui.cursor_icon_at(ui.corner_of(A, 1, 0)), C::ResizeNeSw);
    assert_eq!(ui.cursor_icon_at(ui.corner_of(A, 0, 1)), C::ResizeNeSw);
    assert_eq!(ui.cursor_icon_at(ui.corner_of(A, 0, 0)), C::ResizeNwSe);
    let r = ui.rect_of(A);
    assert_eq!(ui.cursor_icon_at(ui.at_emu(r.right(), r.y + r.h / 2)), C::ResizeHorizontal);
    assert_eq!(ui.cursor_icon_at(ui.at_emu(r.x + r.w / 2, r.bottom())), C::ResizeVertical);
    ui.set_tool(Tool::Rectangle);
    assert_eq!(ui.cursor_icon_at(ui.at_in(0.5, 8.0)), C::Crosshair, "a drawing tool");
}

/// 100% zoom, scrolled to the top, so the page is bigger than the view.
fn at_100_percent() -> TestUi {
    let mut ui = two_boxes();
    ui.app().zoom_mode = crate::app::ZoomMode::Custom;
    ui.app().zoom = 1.0;
    ui.settle();
    ui
}

#[test]
fn wheel_scrolls_the_page() {
    let mut ui = at_100_percent();
    let before = ui.h.state().probe.origin;
    let zoom = ui.h.state().probe.zoom;
    ui.wheel_at(ui.at_in(4.0, 5.0), Vec2::new(0.0, -120.0), egui::Modifiers::NONE);
    let after = ui.h.state().probe.origin;
    assert!(after.y < before.y - 20.0, "page moved up: {before:?} -> {after:?}");
    assert_eq!(ui.h.state().probe.zoom, zoom, "plain wheel does not zoom");
}

#[test]
fn ctrl_wheel_zooms() {
    let mut ui = at_100_percent();
    let z0 = ui.h.state().probe.zoom;
    // A fixed point in the middle of the window: after zooming the page moves under it.
    let mid = egui::pos2(640.0, 400.0);
    ui.wheel_at(mid, Vec2::new(0.0, 120.0), CTRL);
    let z_in = ui.h.state().probe.zoom;
    assert!(z_in > z0, "Ctrl+wheel up zooms in: {z0} -> {z_in}");
    ui.wheel_at(mid, Vec2::new(0.0, -240.0), CTRL);
    let z_out = ui.h.state().probe.zoom;
    assert!(z_out < z_in, "Ctrl+wheel down zooms out: {z0} {z_in} {z_out}");
}

#[test]
fn scratch_area_shapes_hidden_when_off() {
    let mut ui = two_boxes();
    // Drag a box off the right edge of the page (8.5 in wide) onto the scratch area.
    let from = ui.center_of(A);
    ui.drag(from, ui.at_in(10.0, 2.0));
    assert!(ui.rect_of(A).x > in_to_emu(8.5), "the shape can sit on the scratch area");
    let page = |ui: &TestUi| egui::Rect::from_min_max(ui.at_in(0.0, 0.0), ui.at_in(8.5, 11.0));
    let clip = ui.h.state().probe.content_clip;
    assert!(clip.contains_rect(ui.painted(A)), "Scratch Area on: drawn outside the page");
    assert!(clip.width() > page(&ui).width());

    ui.app().show.scratch = false;
    ui.settle();
    let clip = ui.h.state().probe.content_clip;
    assert_eq!(clip, page(&ui), "Scratch Area off: objects are clipped to the page");
    assert!(!clip.intersects(ui.painted(A)), "so the off-page box is not visible");
}

