//! The little boxes beside a selected text frame (Publisher 2021, docs/ui/LINKED-TEXT.md):
//! a white square with a 1 px black border outside the right edge holding three dots when
//! the frame's text does not fit (overflow), or a right arrow when the frame has a next
//! link; a left arrow outside the left edge when it has a previous link. They are only on
//! screen while the frame is selected, never printed or exported.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// Side of a box in screen pixels (Publisher draws about 14 px at 54 % zoom; this keeps that
/// size at every zoom).
pub const BOX: f32 = 14.0;

/// Fill of the selection handles of a frame whose text overflows.
pub const OVERFLOW_RED: Color32 = Color32::from_rgb(0xD1, 0x2B, 0x2B);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// "..." : the text does not fit (last frame of a chain, or a standalone box).
    Overflow,
    /// Right arrow: go to the next frame of the chain.
    Next,
    /// Left arrow: go to the previous frame of the chain.
    Prev,
}

/// The boxes of the frame drawn at screen rectangle `sr`. The vertical places follow what
/// was measured in Publisher: the right box low in the frame (about 15 px above the bottom
/// of a 65 px frame, 39 px above the bottom of a 260 px one), the left one high (about 23 px
/// below the top of a 65 px frame, 39 px of a 260 px one).
pub fn marks(sr: Rect, prev: bool, next: bool, overflow: bool) -> Vec<(Mark, Rect)> {
    let h = sr.height();
    let half = BOX / 2.0;
    let clamp = |y: f32| y.min(sr.bottom() - half).max(sr.top() + half);
    let mut out = Vec::new();
    let right = if next {
        Some(Mark::Next)
    } else if overflow {
        Some(Mark::Overflow)
    } else {
        None
    };
    if let Some(m) = right {
        let y = clamp(sr.bottom() - (7.0 + 0.123 * h));
        out.push((m, Rect::from_center_size(Pos2::new(sr.right() + half, y), Vec2::splat(BOX))));
    }
    if prev {
        let y = clamp(sr.top() + 17.7 + 0.082 * h);
        out.push((Mark::Prev, Rect::from_center_size(Pos2::new(sr.left() - half, y), Vec2::splat(BOX))));
    }
    out
}

pub fn paint(painter: &Painter, marks: &[(Mark, Rect)]) {
    for &(m, r) in marks {
        painter.rect_filled(r, 0.0, Color32::WHITE);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
        let c = r.center();
        match m {
            Mark::Overflow => {
                for dx in [-3.5, 0.0, 3.5] {
                    painter.circle_filled(c + Vec2::new(dx, 0.0), 1.1, Color32::BLACK);
                }
            }
            Mark::Next | Mark::Prev => {
                let s = if m == Mark::Next { 1.0 } else { -1.0 };
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        c + Vec2::new(-2.5 * s, -4.0),
                        c + Vec2::new(-2.5 * s, 4.0),
                        c + Vec2::new(3.5 * s, 0.0),
                    ],
                    Color32::BLACK,
                    Stroke::NONE,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_sit_outside_the_edges_right_low_and_left_high() {
        let sr = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 260.0));
        let m = marks(sr, true, true, false);
        let (next, prev) = (m[0], m[1]);
        assert_eq!(next.0, Mark::Next);
        assert_eq!(next.1.left(), sr.right());
        assert!((sr.bottom() - next.1.center().y - 39.0).abs() < 1.0);
        assert_eq!(prev.0, Mark::Prev);
        assert_eq!(prev.1.right(), sr.left());
        assert!((prev.1.center().y - sr.top() - 39.0).abs() < 1.0);
        // The overflow box takes the place of the next arrow.
        let o = marks(sr, false, false, true);
        assert_eq!(o.len(), 1);
        assert_eq!((o[0].0, o[0].1), (Mark::Overflow, next.1));
        assert!(marks(sr, false, false, false).is_empty());
        // A tiny frame keeps the boxes on its edge.
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(30.0, 10.0));
        for (_, r) in marks(tiny, true, true, false) {
            assert!(r.center().y >= tiny.top() && r.center().y <= tiny.bottom());
        }
    }
}
