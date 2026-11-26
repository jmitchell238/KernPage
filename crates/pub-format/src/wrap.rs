//! Text wrap around objects (spec 4K): the regions a text frame must avoid, as convex
//! polygons in points, and the free spans of a line band. Pure geometry, shared by the
//! PDF converter and the editor.
//!
//! All coordinates are points from the page's top-left corner, y pointing down.

use crate::{ShapeGeometry, WrapType};

/// A region text keeps away from: a convex polygon, vertices in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Poly {
    pub pts: Vec<(f32, f32)>,
    /// `true`: the region counts only where it crosses the line's vertical centre (tight
    /// wrap, measured, spec 4K); `false`: anywhere in the line's whole height (square).
    pub centre_only: bool,
}

/// Number of vertices used to approximate an ellipse.
const ELLIPSE_STEPS: usize = 64;

/// Rotate `p` clockwise (y down) by `deg` degrees about `c`.
pub fn rotate(p: (f32, f32), c: (f32, f32), deg: f32) -> (f32, f32) {
    let (s, co) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co)
}

/// The region an object keeps text away from, or `None` when it wraps nothing.
///
/// `frame` is the object's **unrotated** frame `(left, top, right, bottom)`, `dist` the wrap
/// distances left, top, right, bottom. Square: the bounding box of the rotated frame grown
/// by the distances. Tight: for an oval the ellipse grown by the distances; any other shape
/// uses the square rule for now (its outline is not followed).
// Top and bottom blocks every line the frame's band touches across the whole width.
// Through is treated like tight (no wrap points are stored in the file reader), so an
// oval follows its ellipse and any other shape uses its frame.
// TODO(pw-2wv.16): follow non-oval outlines for tight/through wrap.
pub fn obstacle(
    frame: (f32, f32, f32, f32),
    rotation_deg: f32,
    wrap: WrapType,
    dist: [f32; 4],
    geometry: Option<ShapeGeometry>,
) -> Option<Poly> {
    let (l, t, r, b) = frame;
    let centre = ((l + r) / 2.0, (t + b) / 2.0);
    let oval = matches!(wrap, WrapType::Tight | WrapType::Through) && matches!(geometry, Some(ShapeGeometry::Ellipse));
    match wrap {
        WrapType::None | WrapType::Other(_) => None,
        _ if oval => {
            let (gl, gt, gr, gb) = (l - dist[0], t - dist[1], r + dist[2], b + dist[3]);
            let (cx, cy) = ((gl + gr) / 2.0, (gt + gb) / 2.0);
            let (rx, ry) = ((gr - gl) / 2.0, (gb - gt) / 2.0);
            Some(Poly {
                pts: (0..ELLIPSE_STEPS)
                    .map(|i| {
                        let a = std::f32::consts::TAU * i as f32 / ELLIPSE_STEPS as f32;
                        rotate((cx + rx * a.cos(), cy + ry * a.sin()), centre, rotation_deg)
                    })
                    .collect(),
                centre_only: true,
            })
        }
        _ => {
            let top_bottom = wrap == WrapType::TopAndBottom;
            let corners = [(l, t), (r, t), (r, b), (l, b)].map(|p| rotate(p, centre, rotation_deg));
            let min_x = corners.iter().map(|p| p.0).fold(f32::MAX, f32::min);
            let max_x = corners.iter().map(|p| p.0).fold(f32::MIN, f32::max);
            let min_y = corners.iter().map(|p| p.1).fold(f32::MAX, f32::min);
            let max_y = corners.iter().map(|p| p.1).fold(f32::MIN, f32::max);
            let (min_x, max_x) = if top_bottom {
                (-1.0e6, 1.0e6)
            } else {
                (min_x, max_x)
            };
            let (x0, y0, x1, y1) = (
                min_x - dist[0],
                min_y - dist[1],
                max_x + dist[2],
                max_y + dist[3],
            );
            Some(Poly {
                pts: vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
                centre_only: false,
            })
        }
    }
}

/// Express a page-space polygon in the coordinates of a frame turned by `rotation_deg`
/// about `centre` (the inverse turn), so text laid out upright in that frame can be
/// tested against it.
pub fn to_frame(poly: &Poly, centre: (f32, f32), rotation_deg: f32) -> Poly {
    if rotation_deg.rem_euclid(360.0) == 0.0 {
        return poly.clone();
    }
    Poly {
        pts: poly
            .pts
            .iter()
            .map(|&p| rotate(p, centre, -rotation_deg))
            .collect(),
        centre_only: poly.centre_only,
    }
}

/// Horizontal extent `(min_x, max_x)` of the convex polygon within the band
/// `y0 <= y <= y1`, or `None` when it does not reach the band.
fn span_in_band(poly: &[(f32, f32)], y0: f32, y1: f32) -> Option<(f32, f32)> {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    let mut note = |x: f32| {
        lo = lo.min(x);
        hi = hi.max(x);
    };
    for (i, &(ax, ay)) in poly.iter().enumerate() {
        let (bx, by) = poly[(i + 1) % poly.len()];
        if (y0..=y1).contains(&ay) {
            note(ax);
        }
        // Where the edge crosses each band boundary.
        for y in [y0, y1] {
            if (ay < y) != (by < y) {
                note(ax + (bx - ax) * (y - ay) / (by - ay));
            }
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// The parts of `[left, right]` left free for text on the band `y0..y1` after removing
/// every obstacle's span, left to right. Pieces narrower than `min_width` are dropped
/// (spec: ? no rule for the narrowest usable piece).
pub fn free_spans(
    obstacles: &[Poly],
    y0: f32,
    y1: f32,
    left: f32,
    right: f32,
    min_width: f32,
) -> Vec<(f32, f32)> {
    let mut blocked: Vec<(f32, f32)> = obstacles
        .iter()
        .filter_map(|p| {
            if p.centre_only {
                let m = (y0 + y1) / 2.0;
                span_in_band(&p.pts, m, m)
            } else {
                span_in_band(&p.pts, y0, y1)
            }
        })
        .collect();
    blocked.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut at = left;
    for (bl, br) in blocked {
        if bl > at {
            out.push((at, bl.min(right)));
        }
        at = at.max(br);
        if at >= right {
            break;
        }
    }
    if at < right {
        out.push((at, right));
    }
    out.retain(|(a, b)| b - a >= min_width && b > a);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_splits_the_band_in_two() {
        let o = obstacle_sq();
        let s = free_spans(&[o], 150.0, 164.0, 75.6, 540.0, 1.0);
        assert_eq!(s.len(), 2);
        assert!((s[0].1 - 213.12).abs() < 0.01 && (s[1].0 - 362.88).abs() < 0.01);
        // Above the grown frame the band is whole.
        assert_eq!(
            free_spans(&[obstacle_sq()], 100.0, 114.0, 75.6, 540.0, 1.0).len(),
            1
        );
    }

    fn obstacle_sq() -> Poly {
        obstacle(
            (216.0, 144.0, 360.0, 252.0),
            0.0,
            WrapType::Square,
            [2.88; 4],
            None,
        )
        .unwrap()
    }

    #[test]
    fn oval_is_narrowest_at_the_middle_and_none_wraps_nothing() {
        let o = obstacle(
            (216.0, 360.0, 360.0, 468.0),
            0.0,
            WrapType::Tight,
            [2.88; 4],
            Some(ShapeGeometry::Ellipse),
        )
        .unwrap();
        let mid = free_spans(std::slice::from_ref(&o), 410.0, 412.0, 0.0, 600.0, 1.0);
        let edge = free_spans(&[o], 360.0, 370.0, 0.0, 600.0, 1.0);
        let gap = |s: &[(f32, f32)]| s[1].0 - s[0].1;
        assert!(gap(&mid) > gap(&edge));
        assert!(obstacle((0.0, 0.0, 9.0, 9.0), 0.0, WrapType::None, [0.0; 4], None).is_none());
    }
}
