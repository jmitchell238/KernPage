//! WMF/EMF clip art for the page view: a parsed display list (`metafile::Metafile`) turned
//! once into zoom-independent drawing operations (filled polygons triangulated in the
//! metafile's own space, polylines, text, textures), then painted scaled into a picture
//! frame with egui shapes.

use crate::fontmgr::FontMgr;
use eframe::egui::{
    self, epaint::TextShape, Align2, Color32, Context, Mesh, Painter, Pos2, Rect, Stroke,
    TextureHandle,
};
use metafile::{FillRule, HAlign, Item, LineStyle, Metafile, PathSeg, Point, VAlign};

/// Line segments per curve when flattening ellipses and Bezier segments.
const CURVE_STEPS: usize = 16;
/// Metafile cell heights include leading; this converts to an em height.
const CELL_TO_EM: f32 = 0.8;

enum Op {
    /// Triangles in metafile space.
    Fill {
        verts: Vec<[f32; 2]>,
        indices: Vec<u32>,
        color: Color32,
    },
    Line {
        points: Vec<[f32; 2]>,
        closed: bool,
        /// Width in metafile units; 0 = hairline.
        width: f32,
        color: Color32,
        style: LineStyle,
    },
    Text {
        text: String,
        pos: [f32; 2],
        /// Em height in metafile units.
        size: f32,
        color: Color32,
        halign: HAlign,
        valign: VAlign,
        face: String,
        bold: bool,
        italic: bool,
        /// Degrees counter-clockwise.
        angle_deg: f32,
    },
    Image {
        /// left, top, right, bottom in metafile space.
        dest: [f32; 4],
        tex: TextureHandle,
    },
}

/// A metafile ready to paint.
pub struct MetaPic {
    /// left, top, width, height of the metafile's extent.
    bounds: [f32; 4],
    ops: Vec<Op>,
}

fn c32(c: metafile::Color) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

fn pt(p: Point) -> [f32; 2] {
    [p.x as f32, p.y as f32]
}

fn ellipse(r: &metafile::Rect) -> Vec<[f32; 2]> {
    let (cx, cy) = ((r.left + r.right) / 2.0, (r.top + r.bottom) / 2.0);
    let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
    let n = CURVE_STEPS * 4;
    (0..n)
        .map(|i| {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            [(cx + rx * a.cos()) as f32, (cy + ry * a.sin()) as f32]
        })
        .collect()
}

fn rounded_rect(r: &metafile::Rect, cw: f64, ch: f64) -> Vec<[f32; 2]> {
    let (rx, ry) = (
        (cw / 2.0).min(r.width().abs() / 2.0),
        (ch / 2.0).min(r.height().abs() / 2.0),
    );
    let corners = [
        (r.right - rx, r.top + ry, -90.0_f64),
        (r.right - rx, r.bottom - ry, 0.0),
        (r.left + rx, r.bottom - ry, 90.0),
        (r.left + rx, r.top + ry, 180.0),
    ];
    let mut out = Vec::new();
    for (cx, cy, start) in corners {
        for i in 0..=CURVE_STEPS / 2 {
            let a = (start + 90.0 * i as f64 / (CURVE_STEPS / 2) as f64).to_radians();
            out.push([(cx + rx * a.cos()) as f32, (cy + ry * a.sin()) as f32]);
        }
    }
    out
}

/// Flatten path segments into subpaths; the flag says whether the subpath was closed.
fn flatten(segments: &[PathSeg]) -> Vec<(Vec<[f32; 2]>, bool)> {
    let mut out: Vec<(Vec<[f32; 2]>, bool)> = Vec::new();
    let mut cur: Vec<[f32; 2]> = Vec::new();
    let mut flush = |cur: &mut Vec<[f32; 2]>, closed: bool| {
        if cur.len() > 1 {
            out.push((std::mem::take(cur), closed));
        } else {
            cur.clear();
        }
    };
    for seg in segments {
        match seg {
            PathSeg::MoveTo(p) => {
                flush(&mut cur, false);
                cur.push(pt(*p));
            }
            PathSeg::LineTo(p) => cur.push(pt(*p)),
            PathSeg::CubicTo(a, b, d) => {
                let Some(&s) = cur.last() else {
                    cur.push(pt(*d));
                    continue;
                };
                let (a, b, d) = (pt(*a), pt(*b), pt(*d));
                for i in 1..=CURVE_STEPS {
                    let t = i as f32 / CURVE_STEPS as f32;
                    let u = 1.0 - t;
                    let f = |k: usize| {
                        u * u * u * s[k]
                            + 3.0 * u * u * t * a[k]
                            + 3.0 * u * t * t * b[k]
                            + t * t * t * d[k]
                    };
                    cur.push([f(0), f(1)]);
                }
            }
            PathSeg::Close => {
                flush(&mut cur, true);
            }
        }
    }
    flush(&mut cur, false);
    out
}

/// Triangulate the filled area of closed polygons by scanline slabs: between two
/// consecutive vertex heights the inside spans are quads between edges (exact for
/// non-crossing edges, any fill rule, holes included).
fn triangulate(polys: &[&[[f32; 2]]], rule: FillRule) -> (Vec<[f32; 2]>, Vec<u32>) {
    struct Edge {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        dir: i32,
    }
    let mut edges: Vec<Edge> = Vec::new();
    let mut ys: Vec<f32> = Vec::new();
    for poly in polys {
        for (i, a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            ys.push(a[1]);
            if a[1] == b[1] {
                continue;
            }
            let (lo, hi, dir) = if a[1] < b[1] { (*a, b, 1) } else { (b, *a, -1) };
            edges.push(Edge {
                x0: lo[0],
                y0: lo[1],
                x1: hi[0],
                y1: hi[1],
                dir,
            });
        }
    }
    edges.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    let mut verts = Vec::new();
    let mut indices = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    let mut next = 0;
    let x_at = |e: &Edge, y: f32| e.x0 + (e.x1 - e.x0) * (y - e.y0) / (e.y1 - e.y0);
    for w in ys.windows(2) {
        let (ya, yb) = (w[0], w[1]);
        active.retain(|&i| edges[i].y1 > ya);
        while next < edges.len() && edges[next].y0 <= ya {
            if edges[next].y1 > ya {
                active.push(next);
            }
            next += 1;
        }
        let ym = (ya + yb) / 2.0;
        let mut xs: Vec<(f32, usize)> = active.iter().map(|&i| (x_at(&edges[i], ym), i)).collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0;
        let mut start: Option<usize> = None;
        for &(_, i) in &xs {
            let was_in = match rule {
                FillRule::EvenOdd => winding & 1 != 0,
                FillRule::NonZero => winding != 0,
            };
            winding += edges[i].dir;
            let now_in = match rule {
                FillRule::EvenOdd => winding & 1 != 0,
                FillRule::NonZero => winding != 0,
            };
            if !was_in && now_in {
                start = Some(i);
            } else if was_in && !now_in {
                if let Some(l) = start.take() {
                    let base = verts.len() as u32;
                    let (el, er) = (&edges[l], &edges[i]);
                    verts.extend([
                        [x_at(el, ya), ya],
                        [x_at(er, ya), ya],
                        [x_at(er, yb), yb],
                        [x_at(el, yb), yb],
                    ]);
                    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }
    (verts, indices)
}

impl MetaPic {
    /// Convert a parsed metafile. Bitmaps become textures, so this needs the egui context.
    pub fn build(ctx: &Context, mf: &Metafile, key: usize) -> MetaPic {
        let mut ops = Vec::new();
        let mut textures = 0;
        for item in &mf.items {
            match item {
                Item::Polygon {
                    points,
                    fill,
                    stroke,
                    rule,
                } => {
                    let pts: Vec<[f32; 2]> = points.iter().copied().map(pt).collect();
                    shape_ops(&mut ops, &[(pts, true)], *fill, *stroke, *rule);
                }
                Item::Polyline { points, stroke } => {
                    let pts: Vec<[f32; 2]> = points.iter().copied().map(pt).collect();
                    shape_ops(&mut ops, &[(pts, false)], None, *stroke, FillRule::NonZero);
                }
                Item::PolyPolygon {
                    polygons,
                    fill,
                    stroke,
                    rule,
                } => {
                    let subs: Vec<_> = polygons
                        .iter()
                        .map(|p| (p.iter().copied().map(pt).collect::<Vec<_>>(), true))
                        .collect();
                    shape_ops(&mut ops, &subs, *fill, *stroke, *rule);
                }
                Item::Rect {
                    rect,
                    corner,
                    fill,
                    stroke,
                } => {
                    let pts = match corner {
                        Some((cw, ch)) => rounded_rect(rect, *cw, *ch),
                        None => vec![
                            [rect.left as f32, rect.top as f32],
                            [rect.right as f32, rect.top as f32],
                            [rect.right as f32, rect.bottom as f32],
                            [rect.left as f32, rect.bottom as f32],
                        ],
                    };
                    shape_ops(&mut ops, &[(pts, true)], *fill, *stroke, FillRule::NonZero);
                }
                Item::Ellipse { rect, fill, stroke } => {
                    shape_ops(
                        &mut ops,
                        &[(ellipse(rect), true)],
                        *fill,
                        *stroke,
                        FillRule::NonZero,
                    );
                }
                Item::Path {
                    segments,
                    fill,
                    stroke,
                    rule,
                } => shape_ops(&mut ops, &flatten(segments), *fill, *stroke, *rule),
                Item::Text(t) => {
                    let em = if t.font.cell_height {
                        t.font.height as f32 * CELL_TO_EM
                    } else {
                        t.font.height as f32
                    };
                    ops.push(Op::Text {
                        text: t.text.chars().filter(|c| !c.is_control()).collect(),
                        pos: pt(t.pos),
                        size: em,
                        color: c32(t.color),
                        halign: t.halign,
                        valign: t.valign,
                        face: t.font.face.clone(),
                        bold: t.font.weight >= 600,
                        italic: t.font.italic,
                        angle_deg: t.font.escapement_deg as f32,
                    });
                }
                Item::Bitmap {
                    dest,
                    width,
                    height,
                    rgba,
                } => {
                    let size = [*width as usize, *height as usize];
                    if size[0] * size[1] * 4 == rgba.len() && size[0] > 0 && size[1] > 0 {
                        textures += 1;
                        let tex = ctx.load_texture(
                            format!("meta{key}-{textures}"),
                            egui::ColorImage::from_rgba_unmultiplied(size, rgba),
                            egui::TextureOptions::LINEAR,
                        );
                        ops.push(Op::Image {
                            dest: [
                                dest.left as f32,
                                dest.top as f32,
                                dest.right as f32,
                                dest.bottom as f32,
                            ],
                            tex,
                        });
                    }
                }
                Item::BitmapPlaceholder { dest, .. } => {
                    let pts = vec![
                        [dest.left as f32, dest.top as f32],
                        [dest.right as f32, dest.top as f32],
                        [dest.right as f32, dest.bottom as f32],
                        [dest.left as f32, dest.bottom as f32],
                    ];
                    let fill = metafile::Fill {
                        color: metafile::Color {
                            r: 217,
                            g: 217,
                            b: 217,
                        },
                        approximated: true,
                    };
                    shape_ops(
                        &mut ops,
                        &[(pts, true)],
                        Some(fill),
                        None,
                        FillRule::NonZero,
                    );
                }
            }
        }
        let b = mf.bounds;
        MetaPic {
            bounds: [
                b.left as f32,
                b.top as f32,
                b.width() as f32,
                b.height() as f32,
            ],
            ops,
        }
    }

    /// Paint into `rect` (screen). `xf` maps a screen point of the unturned frame to its
    /// final position (the picture's rotation); `angle` is that rotation in radians.
    pub fn draw(
        &self,
        painter: &Painter,
        fonts: &FontMgr,
        rect: Rect,
        xf: &dyn Fn(Pos2) -> Pos2,
        angle: f32,
    ) {
        let [bl, bt, bw, bh] = self.bounds;
        let (sx, sy) = (
            if bw > 0.0 { rect.width() / bw } else { 1.0 },
            if bh > 0.0 { rect.height() / bh } else { 1.0 },
        );
        let scale = (sx.abs() + sy.abs()) / 2.0;
        let to = |p: [f32; 2]| {
            xf(Pos2::new(
                rect.min.x + (p[0] - bl) * sx,
                rect.min.y + (p[1] - bt) * sy,
            ))
        };
        for op in &self.ops {
            match op {
                Op::Fill {
                    verts,
                    indices,
                    color,
                } => {
                    let mut mesh = Mesh::default();
                    for v in verts {
                        mesh.colored_vertex(to(*v), *color);
                    }
                    mesh.indices.clone_from(indices);
                    painter.add(egui::Shape::mesh(mesh));
                }
                Op::Line {
                    points,
                    closed,
                    width,
                    color,
                    style,
                } => {
                    let pts: Vec<Pos2> = points.iter().map(|p| to(*p)).collect();
                    let stroke = Stroke::new((width * scale).max(0.75), *color);
                    let unit = stroke.width.max(1.0);
                    let (dash, gap) = match style {
                        LineStyle::Solid => (0.0, 0.0),
                        LineStyle::Dash | LineStyle::DashDot | LineStyle::DashDotDot => {
                            (4.0 * unit, 3.0 * unit)
                        }
                        LineStyle::Dot => (unit, 2.0 * unit),
                    };
                    if dash == 0.0 {
                        let shape = if *closed {
                            egui::Shape::closed_line(pts, stroke)
                        } else {
                            egui::Shape::line(pts, stroke)
                        };
                        painter.add(shape);
                    } else {
                        let mut pts = pts;
                        if *closed {
                            pts.push(pts[0]);
                        }
                        for seg in egui::Shape::dashed_line(&pts, stroke, dash, gap) {
                            painter.add(seg);
                        }
                    }
                }
                Op::Image { dest, tex } => {
                    let a = to([dest[0], dest[1]]);
                    let b = to([dest[2], dest[1]]);
                    let c = to([dest[2], dest[3]]);
                    let d = to([dest[0], dest[3]]);
                    let mut mesh = Mesh::with_texture(tex.id());
                    for (pos, uv) in [a, b, c, d].into_iter().zip([
                        Pos2::new(0.0, 0.0),
                        Pos2::new(1.0, 0.0),
                        Pos2::new(1.0, 1.0),
                        Pos2::new(0.0, 1.0),
                    ]) {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos,
                            uv,
                            color: Color32::WHITE,
                        });
                    }
                    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                    painter.add(egui::Shape::mesh(mesh));
                }
                Op::Text {
                    text,
                    pos,
                    size,
                    color,
                    halign,
                    valign,
                    face,
                    bold,
                    italic,
                    angle_deg,
                } => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    let px = (size * sy.abs()).max(2.0);
                    let font = fonts.font_id(face, *bold, *italic, px);
                    let galley = painter.layout_no_wrap(text.clone(), font, *color);
                    let anchor = Align2([
                        match halign {
                            HAlign::Left => egui::Align::Min,
                            HAlign::Center => egui::Align::Center,
                            HAlign::Right => egui::Align::Max,
                        },
                        match valign {
                            VAlign::Top => egui::Align::Min,
                            VAlign::Baseline | VAlign::Bottom => egui::Align::Max,
                        },
                    ]);
                    let origin = to(*pos);
                    // Baseline-aligned text sits about 0.2 em above the galley's bottom.
                    let lift = if *valign == VAlign::Baseline {
                        px * 0.2
                    } else {
                        0.0
                    };
                    let top_left = anchor
                        .anchor_size(origin + egui::vec2(0.0, lift), galley.size())
                        .min;
                    // Clockwise radians on screen: the picture's turn plus the escapement.
                    let total = angle - angle_deg.to_radians();
                    let mut shape = TextShape::new(top_left, galley, *color);
                    if total != 0.0 {
                        let rot = egui::emath::Rot2::from_angle(total);
                        shape.pos = origin + rot * (top_left - origin);
                        shape = shape.with_angle(total);
                    }
                    painter.add(shape);
                }
            }
        }
    }
}

/// Fill (triangulated) and stroke ops for subpaths.
fn shape_ops(
    ops: &mut Vec<Op>,
    subs: &[(Vec<[f32; 2]>, bool)],
    fill: Option<metafile::Fill>,
    stroke: Option<metafile::Stroke>,
    rule: FillRule,
) {
    if let Some(f) = fill {
        let polys: Vec<&[[f32; 2]]> = subs
            .iter()
            .filter(|(p, _)| p.len() >= 3)
            .map(|(p, _)| p.as_slice())
            .collect();
        let (verts, indices) = triangulate(&polys, rule);
        if !indices.is_empty() {
            ops.push(Op::Fill {
                verts,
                indices,
                color: c32(f.color),
            });
        }
    }
    if let Some(s) = stroke {
        for (pts, closed) in subs {
            if pts.len() > 1 {
                ops.push(Op::Line {
                    points: pts.clone(),
                    closed: *closed,
                    width: s.width as f32,
                    color: c32(s.color),
                    style: s.style,
                });
            }
        }
    }
}
