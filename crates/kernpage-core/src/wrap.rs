//! Text wrap around objects (spec 4K) for the editor model: which shapes of a page keep
//! the text of a box away, as regions in points from the page's top-left corner.

use crate::geometry::emu_to_pt;
use crate::model::{Shape, ShapeId, ShapeKind, WrapType};
use pub_format::wrap::{self, Poly};
use pub_format::ShapeGeometry;

pub use pub_format::wrap::free_spans;

/// The region `shape` keeps text away from, or `None` when it wraps nothing.
///
/// Only pictures and drawn shapes wrap (as in the converter): text boxes and tables carry
/// wrap type 3 by default and are not treated as wrappers yet.
// TODO(pw-2wv.16): text boxes as wrappers (spec 4K: they carry type 3 by default).
fn wrapper_region(shape: &Shape) -> Option<Poly> {
    let geometry = match shape.kind {
        ShapeKind::Picture { .. } | ShapeKind::Rectangle => None,
        ShapeKind::Ellipse => Some(ShapeGeometry::Ellipse),
        _ => return None,
    };
    if shape.wrap == WrapType::None {
        return None;
    }
    let r = shape.rect;
    let frame = (
        emu_to_pt(r.x) as f32,
        emu_to_pt(r.y) as f32,
        emu_to_pt(r.right()) as f32,
        emu_to_pt(r.bottom()) as f32,
    );
    let dist = shape.wrap_dist_emu.map(|d| emu_to_pt(d) as f32);
    wrap::obstacle(frame, shape.rotation_deg, shape.wrap, dist, geometry)
}

/// The regions the text of shape `id` among the page's `shapes` (z-order) must avoid: every shape in front of it
/// (later in z-order) that wraps and overlaps its frame, in points from the page's
/// top-left, in the text shape's own unrotated frame (spec 4K).
pub fn obstacles(shapes: &[Shape], id: ShapeId) -> Vec<Poly> {
    let Some(ix) = shapes.iter().position(|s| s.id == id) else {
        return Vec::new();
    };
    let me = &shapes[ix];
    let r = me.rect;
    let mine = (
        emu_to_pt(r.x) as f32,
        emu_to_pt(r.y) as f32,
        emu_to_pt(r.right()) as f32,
        emu_to_pt(r.bottom()) as f32,
    );
    let centre = ((mine.0 + mine.2) / 2.0, (mine.1 + mine.3) / 2.0);
    shapes[ix + 1..]
        .iter()
        .filter_map(wrapper_region)
        .map(|p| wrap::to_frame(&p, centre, me.rotation_deg))
        .filter(|p| {
            let xs = || p.pts.iter().map(|q| q.0);
            let ys = || p.pts.iter().map(|q| q.1);
            xs().fold(f32::MIN, f32::max) > mine.0
                && xs().fold(f32::MAX, f32::min) < mine.2
                && ys().fold(f32::MIN, f32::max) > mine.1
                && ys().fold(f32::MAX, f32::min) < mine.3
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::model::{Document, Paragraph};

    #[test]
    fn only_wrapping_shapes_in_front_are_obstacles() {
        let e = |pt: i64| pt * 12_700;
        let mut text = Shape::text_box(
            Rect::new(e(72), e(72), e(468), e(648)),
            vec![Paragraph::plain("x")],
        );
        text.id = 1;
        let mut square = Shape::new(
            Rect::new(e(216), e(144), e(144), e(108)),
            ShapeKind::Rectangle,
        );
        square.id = 2;
        square.wrap = WrapType::Square;
        let mut plain = Shape::new(
            Rect::new(e(216), e(300), e(144), e(108)),
            ShapeKind::Rectangle,
        );
        plain.id = 3;
        let mut behind = Shape::new(
            Rect::new(e(216), e(450), e(144), e(108)),
            ShapeKind::Rectangle,
        );
        behind.id = 4;
        behind.wrap = WrapType::Square;
        let mut doc = Document::blank("t", e(612), e(792));
        doc.pages[0].shapes = vec![behind, text, square, plain];
        let o = obstacles(&doc.pages[0].shapes, 1);
        assert_eq!(o.len(), 1);
        let spans = free_spans(&o, 150.0, 164.0, 75.6, 540.0, 1.0);
        assert_eq!(spans.len(), 2);
    }
}
