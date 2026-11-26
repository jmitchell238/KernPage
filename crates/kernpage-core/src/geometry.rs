//! Unit conversions (EMU, points, inches, screen pixels at a zoom), hit-testing and
//! resize-handle maths.

use crate::model::{Page, Shape, ShapeId, ShapeKind, EMU_PER_INCH, EMU_PER_POINT};

/// Logical pixels per inch at 100% zoom.
pub const SCREEN_DPI: f64 = 96.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    pub fn new(x: i64, y: i64, w: i64, h: i64) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> i64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i64 {
        self.y + self.h
    }
    pub fn contains(&self, px: i64, py: i64) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
    pub fn translated(&self, dx: i64, dy: i64) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
    /// The rectangle spanned by two corner points, in either order.
    pub fn from_corners(x1: i64, y1: i64, x2: i64, y2: i64) -> Rect {
        Rect::new(x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs())
    }
}

pub fn emu_to_pt(emu: i64) -> f64 {
    emu as f64 / EMU_PER_POINT as f64
}
pub fn pt_to_emu(pt: f64) -> i64 {
    (pt * EMU_PER_POINT as f64).round() as i64
}
pub fn emu_to_in(emu: i64) -> f64 {
    emu as f64 / EMU_PER_INCH as f64
}
pub fn in_to_emu(inches: f64) -> i64 {
    (inches * EMU_PER_INCH as f64).round() as i64
}
pub fn pt_to_in(pt: f64) -> f64 {
    pt / 72.0
}

/// Screen pixels for a length in EMU at `zoom` (1.0 = 100%).
pub fn emu_to_px(emu: i64, zoom: f64) -> f64 {
    emu_to_in(emu) * SCREEN_DPI * zoom
}
/// EMU for a length in screen pixels at `zoom`.
pub fn px_to_emu(px: f64, zoom: f64) -> i64 {
    in_to_emu(px / (SCREEN_DPI * zoom))
}
/// Pixels per point at `zoom` (font sizes).
pub fn pt_to_px(pt: f64, zoom: f64) -> f64 {
    pt / 72.0 * SCREEN_DPI * zoom
}

/// The zoom at which the whole page fits in `avail_w` x `avail_h` pixels.
pub fn zoom_whole_page(avail_w: f64, avail_h: f64, page_w: i64, page_h: i64) -> f64 {
    let z = (avail_w / emu_to_px(page_w, 1.0)).min(avail_h / emu_to_px(page_h, 1.0));
    clamp_zoom(z)
}
/// The zoom at which the page is as wide as `avail_w` pixels.
pub fn zoom_page_width(avail_w: f64, page_w: i64) -> f64 {
    clamp_zoom(avail_w / emu_to_px(page_w, 1.0))
}
pub const MIN_ZOOM: f64 = 0.1;
pub const MAX_ZOOM: f64 = 4.0;
pub fn clamp_zoom(z: f64) -> f64 {
    if z.is_finite() {
        z.clamp(MIN_ZOOM, MAX_ZOOM)
    } else {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl Handle {
    pub const ALL: [Handle; 8] = [
        Handle::NorthWest,
        Handle::North,
        Handle::NorthEast,
        Handle::East,
        Handle::SouthEast,
        Handle::South,
        Handle::SouthWest,
        Handle::West,
    ];

    fn moves_left(self) -> bool {
        matches!(self, Handle::NorthWest | Handle::West | Handle::SouthWest)
    }
    fn moves_right(self) -> bool {
        matches!(self, Handle::NorthEast | Handle::East | Handle::SouthEast)
    }
    fn moves_top(self) -> bool {
        matches!(self, Handle::NorthWest | Handle::North | Handle::NorthEast)
    }
    fn moves_bottom(self) -> bool {
        matches!(self, Handle::SouthWest | Handle::South | Handle::SouthEast)
    }
}

/// Where a handle sits on `r`.
pub fn handle_point(r: Rect, h: Handle) -> (i64, i64) {
    let x = if h.moves_left() {
        r.x
    } else if h.moves_right() {
        r.right()
    } else {
        r.x + r.w / 2
    };
    let y = if h.moves_top() {
        r.y
    } else if h.moves_bottom() {
        r.bottom()
    } else {
        r.y + r.h / 2
    };
    (x, y)
}

/// The handle within `tol` EMU (on both axes) of the point, if any. Corners win over
/// edge handles when they overlap on tiny shapes.
pub fn hit_handle(r: Rect, px: i64, py: i64, tol: i64) -> Option<Handle> {
    let order = [
        Handle::NorthWest,
        Handle::NorthEast,
        Handle::SouthEast,
        Handle::SouthWest,
        Handle::North,
        Handle::East,
        Handle::South,
        Handle::West,
    ];
    order.into_iter().find(|&h| {
        let (hx, hy) = handle_point(r, h);
        (hx - px).abs() <= tol && (hy - py).abs() <= tol
    })
}

/// Resize `r` by dragging `handle` by (`dx`, `dy`). The dragged edges never cross the
/// opposite edges: the size is clamped to `min` and the opposite edge stays put.
pub fn resize(r: Rect, handle: Handle, dx: i64, dy: i64, min: i64) -> Rect {
    let (mut left, mut top, mut right, mut bottom) = (r.x, r.y, r.right(), r.bottom());
    if handle.moves_left() {
        left = (left + dx).min(right - min);
    }
    if handle.moves_right() {
        right = (right + dx).max(left + min);
    }
    if handle.moves_top() {
        top = (top + dy).min(bottom - min);
    }
    if handle.moves_bottom() {
        bottom = (bottom + dy).max(top + min);
    }
    Rect::new(left, top, right - left, bottom - top)
}

fn dist_to_segment(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// The axis-aligned bounds of `s` as drawn: its frame, or for a rotated shape the box around
/// the rotated outline.
pub fn visual_bounds(s: &Shape) -> Rect {
    let r = s.rect;
    if s.rotation_deg.rem_euclid(360.0) == 0.0 {
        return r;
    }
    let (sin, cos) = f64::from(s.rotation_deg).to_radians().sin_cos();
    let (hw, hh) = (r.w as f64 / 2.0, r.h as f64 / 2.0);
    let ex = hw * cos.abs() + hh * sin.abs();
    let ey = hw * sin.abs() + hh * cos.abs();
    let (cx, cy) = (r.x as f64 + hw, r.y as f64 + hh);
    Rect::new(
        (cx - ex).round() as i64,
        (cy - ey).round() as i64,
        (2.0 * ex).round() as i64,
        (2.0 * ey).round() as i64,
    )
}

/// The two end points of a line in `r`: top-left to bottom-right, or bottom-left to top-right
/// when exactly one of the flips is set.
pub fn line_ends(r: Rect, flip_h: bool, flip_v: bool) -> ((i64, i64), (i64, i64)) {
    if flip_h != flip_v {
        ((r.x, r.bottom()), (r.right(), r.y))
    } else {
        ((r.x, r.y), (r.right(), r.bottom()))
    }
}

/// Whether the point hits `s`. Lines are hit near the line itself (within `tol` EMU plus
/// half the line width); every other shape is hit anywhere in its rectangle.
pub fn shape_hit(s: &Shape, px: i64, py: i64, tol: i64) -> bool {
    // A rotated shape (spec 4H) is hit inside its rotated outline: turn the point back
    // about the centre into the unrotated frame.
    let (px, py) = if s.rotation_deg.rem_euclid(360.0) == 0.0 {
        (px, py)
    } else {
        let (cx, cy) = (
            s.rect.x as f64 + s.rect.w as f64 / 2.0,
            s.rect.y as f64 + s.rect.h as f64 / 2.0,
        );
        let (sin, cos) = (-f64::from(s.rotation_deg)).to_radians().sin_cos();
        let (dx, dy) = (px as f64 - cx, py as f64 - cy);
        (
            (cx + dx * cos - dy * sin).round() as i64,
            (cy + dx * sin + dy * cos).round() as i64,
        )
    };
    if matches!(s.kind, ShapeKind::Line) {
        let (a, b) = line_ends(s.rect, s.flip_h, s.flip_v);
        let reach = tol as f64 + s.line.map_or(0, |l| l.width_emu) as f64 / 2.0;
        dist_to_segment(
            px as f64,
            py as f64,
            (a.0 as f64, a.1 as f64),
            (b.0 as f64, b.1 as f64),
        ) <= reach
    } else {
        s.rect.contains(px, py)
    }
}

/// The topmost shape under the point.
pub fn hit_test(page: &Page, px: i64, py: i64, tol: i64) -> Option<ShapeId> {
    // A group draws nothing itself; hitting a member hits the outermost group (Publisher).
    page.shapes
        .iter()
        .rev()
        .find(|s| !matches!(s.kind, ShapeKind::Group) && shape_hit(s, px, py, tol))
        .map(|s| page.top_group(s.id))
}

/// Where rectangle `r`, laid out inside `from`, lands when `from` becomes `to` (scaling
/// both axes); used to resize a group's members.
pub fn scale_into(r: Rect, from: Rect, to: Rect) -> Rect {
    if from.w <= 0 || from.h <= 0 {
        return r;
    }
    let map = |v: i64, f0: i64, t0: i64, fl: i64, tl: i64| {
        t0 + (i128::from(v - f0) * i128::from(tl) / i128::from(fl)) as i64
    };
    let (x, y) = (
        map(r.x, from.x, to.x, from.w, to.w),
        map(r.y, from.y, to.y, from.h, to.h),
    );
    let (x2, y2) = (
        map(r.right(), from.x, to.x, from.w, to.w),
        map(r.bottom(), from.y, to.y, from.h, to.h),
    );
    Rect::new(x, y, x2 - x, y2 - y)
}

/// The shapes lying entirely inside `marquee` (Publisher's rubber-band rule), by visual
/// bounds, in stacking order.
pub fn shapes_in_rect(page: &Page, marquee: Rect) -> Vec<ShapeId> {
    page.shapes
        .iter()
        // Members are picked through their top-level group.
        .filter(|s| s.group.is_none())
        .filter(|s| {
            let b = visual_bounds(s);
            b.x >= marquee.x
                && b.y >= marquee.y
                && b.right() <= marquee.right()
                && b.bottom() <= marquee.bottom()
        })
        .map(|s| s.id)
        .collect()
}

/// The smallest rectangle holding the visual bounds of all `ids` on the page.
pub fn union_bounds(page: &Page, ids: &[ShapeId]) -> Option<Rect> {
    let mut it = ids.iter().filter_map(|&i| page.shape(i)).map(visual_bounds);
    let first = it.next()?;
    let (mut l, mut t, mut r, mut b) = (first.x, first.y, first.right(), first.bottom());
    for v in it {
        l = l.min(v.x);
        t = t.min(v.y);
        r = r.max(v.right());
        b = b.max(v.bottom());
    }
    Some(Rect::new(l, t, r - l, b - t))
}

/// Cumulative cell edges (EMU from the table's top-left) for `n` tracks: the stored
/// sizes when they match `n`, else equal tracks filling `total`.
fn track_edges(sizes: &[i64], n: usize, total: i64) -> Vec<i64> {
    let mut e = vec![0i64];
    for i in 0..n {
        let step = if sizes.len() == n {
            sizes[i]
        } else {
            total / n as i64
        };
        e.push(e[i] + step);
    }
    e
}

/// The page-space rectangle of cell `cell` (row-major) of a table shape, laid out the way
/// the renderer does. `None` when the shape is not a table or the cell does not exist.
pub fn table_cell_rect(shape: &Shape, cell: usize) -> Option<Rect> {
    let ShapeKind::Table {
        rows,
        cols,
        col_widths_emu,
        row_heights_emu,
        ..
    } = &shape.kind
    else {
        return None;
    };
    let (rows, cols) = (
        (*rows).clamp(1, 1000) as usize,
        (*cols).clamp(1, 1000) as usize,
    );
    if cell >= rows * cols {
        return None;
    }
    let (r, c) = (cell / cols, cell % cols);
    let xs = track_edges(col_widths_emu, cols, shape.rect.w);
    let ys = track_edges(row_heights_emu, rows, shape.rect.h);
    Some(Rect::new(
        shape.rect.x + xs[c],
        shape.rect.y + ys[r],
        xs[c + 1] - xs[c],
        ys[r + 1] - ys[r],
    ))
}

/// The cell (row-major index) of a table shape under the page-space point.
pub fn table_cell_at(shape: &Shape, px: i64, py: i64) -> Option<usize> {
    let ShapeKind::Table { rows, cols, .. } = &shape.kind else {
        return None;
    };
    let n = (*rows).clamp(1, 1000) as usize * (*cols).clamp(1, 1000) as usize;
    (0..n).find(|&i| table_cell_rect(shape, i).is_some_and(|r| r.contains(px, py)))
}

/// The cell Tab (or Shift+Tab when `back`) moves to from `cell` in a table of
/// `count` cells, row-major. `None` at either end: the caret stays put.
pub fn next_cell(cell: usize, count: usize, back: bool) -> Option<usize> {
    if back {
        cell.checked_sub(1)
    } else {
        (cell + 1 < count).then_some(cell + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Shape;

    #[test]
    fn rotated_shape_is_hit_inside_its_rotated_outline() {
        // A 100 x 20 box turned 90 degrees clockwise about (50, 10) is 20 wide, 100 tall.
        let mut s = Shape::new(Rect::new(0, 0, 100, 20), ShapeKind::Rectangle);
        s.rotation_deg = 90.0;
        assert!(shape_hit(&s, 50, 55, 0), "above the centre, now inside");
        assert!(!shape_hit(&s, 95, 10, 0), "old right end, now outside");
    }

    #[test]
    fn table_cell_geometry_and_tab_order() {
        let t = Shape::new(
            Rect::new(100, 200, 600, 400),
            ShapeKind::Table {
                rows: 2,
                cols: 2,
                col_widths_emu: vec![100, 500],
                row_heights_emu: vec![],
                row_set_emu: vec![],
                cells: vec![],
            },
        );
        assert_eq!(table_cell_rect(&t, 0), Some(Rect::new(100, 200, 100, 200)));
        assert_eq!(table_cell_rect(&t, 3), Some(Rect::new(200, 400, 500, 200)));
        assert_eq!(table_cell_rect(&t, 4), None);
        assert_eq!(table_cell_at(&t, 150, 250), Some(0));
        assert_eq!(table_cell_at(&t, 400, 500), Some(3));
        assert_eq!(table_cell_at(&t, 0, 0), None);
        assert_eq!(next_cell(0, 4, false), Some(1));
        assert_eq!(next_cell(1, 4, false), Some(2));
        assert_eq!(next_cell(3, 4, false), None);
        assert_eq!(next_cell(2, 4, true), Some(1));
        assert_eq!(next_cell(0, 4, true), None);
    }

    #[test]
    fn unit_conversions() {
        assert_eq!(pt_to_emu(72.0), EMU_PER_INCH);
        assert_eq!(in_to_emu(0.5), 457_200);
        assert!((emu_to_pt(EMU_PER_INCH) - 72.0).abs() < 1e-9);
        assert!((emu_to_in(1_371_600) - 1.5).abs() < 1e-9);
        assert!((pt_to_in(36.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn screen_conversions_round_trip() {
        assert!((emu_to_px(EMU_PER_INCH, 1.0) - 96.0).abs() < 1e-9);
        assert!((emu_to_px(EMU_PER_INCH, 0.5) - 48.0).abs() < 1e-9);
        assert!((emu_to_px(EMU_PER_INCH, 2.0) - 192.0).abs() < 1e-9);
        for zoom in [0.25, 0.75, 1.0, 1.7, 3.0] {
            let emu = 3_456_789;
            let back = px_to_emu(emu_to_px(emu, zoom), zoom);
            assert!((back - emu).abs() <= 1, "zoom {zoom}: {back}");
        }
        assert!((pt_to_px(72.0, 1.0) - 96.0).abs() < 1e-9);
    }

    #[test]
    fn fit_zooms() {
        let (w, h) = (in_to_emu(8.5), in_to_emu(11.0));
        // 8.5in = 816px, 11in = 1056px at 100%.
        let z = zoom_whole_page(816.0, 528.0, w, h);
        assert!((z - 0.5).abs() < 1e-9);
        let z = zoom_whole_page(408.0, 5000.0, w, h);
        assert!((z - 0.5).abs() < 1e-9);
        assert!((zoom_page_width(1632.0, w) - 2.0).abs() < 1e-9);
        assert_eq!(zoom_page_width(f64::NAN, w), 1.0);
        assert_eq!(zoom_page_width(1.0, w), MIN_ZOOM);
    }

    #[test]
    fn handle_points_and_hits() {
        let r = Rect::new(100, 200, 400, 300);
        assert_eq!(handle_point(r, Handle::NorthWest), (100, 200));
        assert_eq!(handle_point(r, Handle::SouthEast), (500, 500));
        assert_eq!(handle_point(r, Handle::North), (300, 200));
        assert_eq!(handle_point(r, Handle::West), (100, 350));
        assert_eq!(hit_handle(r, 103, 198, 5), Some(Handle::NorthWest));
        assert_eq!(hit_handle(r, 500, 350, 5), Some(Handle::East));
        assert_eq!(hit_handle(r, 300, 350, 5), None);
        // A tiny rectangle: corners win over edges.
        let tiny = Rect::new(0, 0, 2, 2);
        assert_eq!(hit_handle(tiny, 1, 1, 5), Some(Handle::NorthWest));
    }

    #[test]
    fn resize_math() {
        let r = Rect::new(100, 100, 400, 300);
        assert_eq!(
            resize(r, Handle::SouthEast, 50, 20, 10),
            Rect::new(100, 100, 450, 320)
        );
        assert_eq!(
            resize(r, Handle::NorthWest, 50, 20, 10),
            Rect::new(150, 120, 350, 280)
        );
        assert_eq!(
            resize(r, Handle::East, -100, 999, 10),
            Rect::new(100, 100, 300, 300)
        );
        assert_eq!(
            resize(r, Handle::North, 999, -40, 10),
            Rect::new(100, 60, 400, 340)
        );
        // Dragging past the opposite edge clamps to the minimum size.
        assert_eq!(
            resize(r, Handle::West, 1000, 0, 10),
            Rect::new(490, 100, 10, 300)
        );
        assert_eq!(
            resize(r, Handle::South, 0, -1000, 10),
            Rect::new(100, 100, 400, 10)
        );
    }

    #[test]
    fn hit_testing_prefers_topmost_and_handles_lines() {
        let mut page = Page::blank(1000, 1000);
        let mut a = Shape::new(Rect::new(0, 0, 500, 500), ShapeKind::Rectangle);
        a.id = 1;
        let mut b = Shape::new(Rect::new(250, 250, 500, 500), ShapeKind::Rectangle);
        b.id = 2;
        page.shapes = vec![a, b];
        assert_eq!(hit_test(&page, 100, 100, 0), Some(1));
        assert_eq!(hit_test(&page, 300, 300, 0), Some(2));
        assert_eq!(hit_test(&page, 900, 900, 0), None);

        let mut l = Shape::new(Rect::new(0, 0, 1000, 1000), ShapeKind::Line);
        l.id = 3;
        page.shapes = vec![l];
        assert_eq!(hit_test(&page, 500, 500, 10), Some(3));
        assert_eq!(hit_test(&page, 500, 520, 30), Some(3));
        assert_eq!(hit_test(&page, 900, 100, 10), None);
    }
}

#[cfg(test)]
mod marquee_tests {
    use super::*;

    #[test]
    fn marquee_selects_only_objects_entirely_inside() {
        let mut p = Page::blank(10_000, 10_000);
        for (id, r) in [
            (1, Rect::new(100, 100, 200, 200)),
            (2, Rect::new(250, 250, 200, 200)),
            (3, Rect::new(600, 600, 50, 50)),
        ] {
            let mut s = Shape::new(r, ShapeKind::Rectangle);
            s.id = id;
            p.shapes.push(s);
        }
        let m = Rect::from_corners(50, 50, 460, 460);
        assert_eq!(shapes_in_rect(&p, m), vec![1, 2]);
        assert_eq!(
            shapes_in_rect(&p, Rect::from_corners(50, 50, 440, 440)),
            vec![1]
        );
        assert_eq!(
            union_bounds(&p, &[1, 3]),
            Some(Rect::new(100, 100, 550, 550))
        );
    }
}
