//! Draw a parsed WMF/EMF display list (`metafile::Metafile`) as PDF vector graphics,
//! stretched from the metafile bounds into a picture frame (spec 4C).

use super::{compress_to_vec_zlib, show, unit, ImageColor, ImageDef, Resources, Style, KAPPA};
use metafile::{
    Color, FillRule, Font, HAlign, Item, LineStyle, Metafile, PathSeg, Point, Rect, Stroke, VAlign,
};
use pdf_writer::{Content, Name};

/// Hairline pens (width 0) are drawn this wide, in points.
const HAIRLINE_PT: f32 = 0.25;
/// Metafile font heights are mostly em heights; a cell height includes leading.
const CELL_TO_EM: f32 = 0.8;

/// Maps metafile output space (y down) into page space (y up) inside a frame.
#[derive(Clone, Copy)]
struct Map {
    bounds: Rect,
    /// Frame: left, bottom, right, top.
    frame: (f32, f32, f32, f32),
}

impl Map {
    fn sx(&self) -> f32 {
        let w = self.bounds.width();
        if w == 0.0 {
            1.0
        } else {
            (self.frame.2 - self.frame.0) / w as f32
        }
    }
    fn sy(&self) -> f32 {
        let h = self.bounds.height();
        if h == 0.0 {
            1.0
        } else {
            (self.frame.3 - self.frame.1) / h as f32
        }
    }
    fn pt(&self, p: Point) -> (f32, f32) {
        (
            self.frame.0 + (p.x - self.bounds.left) as f32 * self.sx(),
            self.frame.3 - (p.y - self.bounds.top) as f32 * self.sy(),
        )
    }
    /// Mean scale for lengths that have no direction (pen widths, font sizes).
    fn scale(&self) -> f32 {
        (self.sx().abs() + self.sy().abs()) / 2.0
    }
    /// A rectangle as (left, bottom, width, height) in page space.
    fn rect(&self, r: &Rect) -> (f32, f32, f32, f32) {
        let (x0, y0) = self.pt(Point {
            x: r.left,
            y: r.top,
        });
        let (x1, y1) = self.pt(Point {
            x: r.right,
            y: r.bottom,
        });
        (x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs())
    }
}

fn rgb(c: Color) -> (f32, f32, f32) {
    (unit(c.r), unit(c.g), unit(c.b))
}

/// Set the stroke state for a pen; call inside a saved graphics state.
fn set_stroke(c: &mut Content, s: &Stroke, m: &Map) {
    let (r, g, b) = rgb(s.color);
    let width = if s.width > 0.0 {
        (s.width as f32 * m.scale()).max(HAIRLINE_PT)
    } else {
        HAIRLINE_PT
    };
    c.set_stroke_rgb(r, g, b).set_line_width(width);
    let d = width.max(1.0);
    let pattern: &[f32] = match s.style {
        LineStyle::Solid => &[],
        LineStyle::Dash => &[4.0 * d, 3.0 * d],
        LineStyle::Dot => &[d, 2.0 * d],
        LineStyle::DashDot => &[4.0 * d, 2.0 * d, d, 2.0 * d],
        LineStyle::DashDotDot => &[4.0 * d, 2.0 * d, d, 2.0 * d, d, 2.0 * d],
    };
    if !pattern.is_empty() {
        c.set_dash_pattern(pattern.iter().copied(), 0.0);
    }
}

/// Emit fill/stroke state and the painting operator for a path built by `build`.
fn draw_path(
    c: &mut Content,
    fill: Option<metafile::Fill>,
    stroke: Option<Stroke>,
    rule: FillRule,
    m: &Map,
    build: impl FnOnce(&mut Content),
) {
    c.save_state();
    if let Some(f) = fill {
        let (r, g, b) = rgb(f.color);
        c.set_fill_rgb(r, g, b);
    }
    if let Some(s) = &stroke {
        set_stroke(c, s, m);
    }
    build(c);
    match (fill.is_some(), stroke.is_some(), rule) {
        (true, true, FillRule::EvenOdd) => c.fill_even_odd_and_stroke(),
        (true, true, FillRule::NonZero) => c.fill_nonzero_and_stroke(),
        (true, false, FillRule::EvenOdd) => c.fill_even_odd(),
        (true, false, FillRule::NonZero) => c.fill_nonzero(),
        (false, true, _) => c.stroke(),
        (false, false, _) => c.end_path(),
    };
    c.restore_state();
}

fn poly(c: &mut Content, points: &[Point], m: &Map, close: bool) {
    for (i, p) in points.iter().enumerate() {
        let (x, y) = m.pt(*p);
        if i == 0 {
            c.move_to(x, y);
        } else {
            c.line_to(x, y);
        }
    }
    if close && !points.is_empty() {
        c.close_path();
    }
}

/// Ellipse inscribed in a page-space box using four Bezier arcs.
fn ellipse(c: &mut Content, (x, y, w, h): (f32, f32, f32, f32)) {
    let (cx, cy, rx, ry) = (x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    c.move_to(cx + rx, cy)
        .cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
        .cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
        .cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
        .cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
        .close_path();
}

fn rounded_rect(c: &mut Content, (x, y, w, h): (f32, f32, f32, f32), rx: f32, ry: f32) {
    let rx = rx.min(w / 2.0).max(0.0);
    let ry = ry.min(h / 2.0).max(0.0);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let (r, t) = (x + w, y + h);
    c.move_to(x + rx, y)
        .line_to(r - rx, y)
        .cubic_to(r - rx + kx, y, r, y + ry - ky, r, y + ry)
        .line_to(r, t - ry)
        .cubic_to(r, t - ry + ky, r - rx + kx, t, r - rx, t)
        .line_to(x + rx, t)
        .cubic_to(x + rx - kx, t, x, t - ry + ky, x, t - ry)
        .line_to(x, y + ry)
        .cubic_to(x, y + ry - ky, x + rx - kx, y, x + rx, y)
        .close_path();
}

fn text_style(f: &Font, color: Color, m: &Map, res: &mut Resources) -> Style {
    let bold = f.weight >= 600;
    let em = if f.cell_height {
        f.height as f32 * CELL_TO_EM
    } else {
        f.height as f32
    };
    let size = (em * m.sy().abs()).max(1.0);
    Style {
        font: res.resolve_font(&f.face, bold, f.italic),
        bold,
        italic: f.italic,
        size,
        raise: 0.0,
        underline: f.underline,
        color: [color.r, color.g, color.b],
    }
}

fn draw_text(c: &mut Content, t: &metafile::Text, m: &Map, res: &mut Resources) {
    if t.text.trim().is_empty() {
        return;
    }
    let style = text_style(&t.font, t.color, m, res);
    let text: String = t.text.chars().filter(|ch| !ch.is_control()).collect();
    let width: f32 = text.chars().map(|ch| res.char_width(ch, &style)).sum();
    let (_, ascent) = res.metrics(&style);
    let (px, py) = m.pt(t.pos);
    let x = match t.halign {
        HAlign::Left => px,
        HAlign::Center => px - width / 2.0,
        HAlign::Right => px - width,
    };
    let baseline = match t.valign {
        VAlign::Baseline => py,
        VAlign::Top => py - ascent,
        VAlign::Bottom => py + (style.size * 0.2),
    };
    c.save_state();
    if t.font.escapement_deg != 0.0 {
        // Counter-clockwise about the reference point.
        let (s, co) = (t.font.escapement_deg as f32).to_radians().sin_cos();
        c.transform([co, s, -s, co, px - co * px + s * py, py - s * px - co * py]);
    }
    if let Some(bg) = t.background {
        let (r, g, b) = rgb(bg);
        c.set_fill_rgb(r, g, b)
            .rect(x, baseline - 0.2 * style.size, width, style.size * 1.2)
            .fill_nonzero();
    }
    show(c, &text, &style, x, baseline, res);
    let [r, g, b] = style.color;
    if style.underline {
        c.set_stroke_rgb(unit(r), unit(g), unit(b))
            .set_line_width((0.05 * style.size).max(0.3))
            .move_to(x, baseline - 0.12 * style.size)
            .line_to(x + width, baseline - 0.12 * style.size)
            .stroke();
    }
    if t.font.strikeout {
        c.set_stroke_rgb(unit(r), unit(g), unit(b))
            .set_line_width((0.05 * style.size).max(0.3))
            .move_to(x, baseline + 0.3 * style.size)
            .line_to(x + width, baseline + 0.3 * style.size)
            .stroke();
    }
    c.restore_state();
}

/// An RGBA bitmap as an image XObject (colour plus an alpha soft mask when needed).
fn rgba_image(width: u32, height: u32, rgba: &[u8], res: &mut Resources) -> Option<ImageDef> {
    let n = (width as usize).checked_mul(height as usize)?;
    if n == 0 || rgba.len() < n.checked_mul(4)? {
        return None;
    }
    let mut samples = Vec::with_capacity(n * 3);
    let mut mask = Vec::with_capacity(n);
    for px in rgba.as_chunks::<4>().0.iter().take(n) {
        samples.extend_from_slice(&px[..3]);
        mask.push(px[3]);
    }
    let smask = mask
        .iter()
        .any(|&a| a != 255)
        .then(|| (res.alloc(), compress_to_vec_zlib(&mask, 6)));
    Some(ImageDef {
        id: res.alloc(),
        width: i32::try_from(width).ok()?,
        height: i32::try_from(height).ok()?,
        color: ImageColor::Rgb,
        jpeg: false,
        data: compress_to_vec_zlib(&samples, 6),
        smask,
    })
}

/// Draw `mf` stretched from its bounds into `frame` (left, bottom, right, top; PDF space),
/// clipped to the frame. Bitmaps are registered in `res.images` and `used_images`.
pub(super) fn draw(
    c: &mut Content,
    mf: &Metafile,
    frame: (f32, f32, f32, f32),
    res: &mut Resources,
    used_images: &mut Vec<usize>,
) {
    let m = Map {
        bounds: mf.bounds,
        frame,
    };
    c.save_state()
        .rect(frame.0, frame.1, frame.2 - frame.0, frame.3 - frame.1)
        .clip_nonzero()
        .end_path();
    for item in &mf.items {
        match item {
            Item::Polygon {
                points,
                fill,
                stroke,
                rule,
            } => draw_path(c, *fill, *stroke, *rule, &m, |c| poly(c, points, &m, true)),
            Item::Polyline { points, stroke } => {
                draw_path(c, None, *stroke, FillRule::NonZero, &m, |c| {
                    poly(c, points, &m, false)
                })
            }
            Item::PolyPolygon {
                polygons,
                fill,
                stroke,
                rule,
            } => draw_path(c, *fill, *stroke, *rule, &m, |c| {
                for p in polygons {
                    poly(c, p, &m, true);
                }
            }),
            Item::Rect {
                rect,
                corner,
                fill,
                stroke,
            } => {
                let r = m.rect(rect);
                draw_path(c, *fill, *stroke, FillRule::NonZero, &m, |c| match corner {
                    Some((cw, ch)) => rounded_rect(
                        c,
                        r,
                        (*cw as f32 * m.sx().abs()) / 2.0,
                        (*ch as f32 * m.sy().abs()) / 2.0,
                    ),
                    None => {
                        c.rect(r.0, r.1, r.2, r.3);
                    }
                });
            }
            Item::Ellipse { rect, fill, stroke } => {
                let r = m.rect(rect);
                draw_path(c, *fill, *stroke, FillRule::NonZero, &m, |c| ellipse(c, r));
            }
            Item::Path {
                segments,
                fill,
                stroke,
                rule,
            } => draw_path(c, *fill, *stroke, *rule, &m, |c| {
                for seg in segments {
                    match seg {
                        PathSeg::MoveTo(p) => {
                            let (x, y) = m.pt(*p);
                            c.move_to(x, y);
                        }
                        PathSeg::LineTo(p) => {
                            let (x, y) = m.pt(*p);
                            c.line_to(x, y);
                        }
                        PathSeg::CubicTo(a, b, d) => {
                            let (a, b, d) = (m.pt(*a), m.pt(*b), m.pt(*d));
                            c.cubic_to(a.0, a.1, b.0, b.1, d.0, d.1);
                        }
                        PathSeg::Close => {
                            c.close_path();
                        }
                    }
                }
            }),
            Item::Text(t) => draw_text(c, t, &m, res),
            Item::Bitmap {
                dest,
                width,
                height,
                rgba,
            } => {
                let (x, y, w, h) = m.rect(dest);
                if let Some(def) = rgba_image(*width, *height, rgba, res) {
                    res.images.push(def);
                    let ix = res.images.len() - 1;
                    used_images.push(ix);
                    c.save_state()
                        .transform([w, 0.0, 0.0, h, x, y])
                        .x_object(Name(format!("Im{}", ix + 1).as_bytes()))
                        .restore_state();
                }
            }
            Item::BitmapPlaceholder { dest, .. } => {
                let (x, y, w, h) = m.rect(dest);
                c.save_state()
                    .set_fill_gray(0.85)
                    .rect(x, y, w, h)
                    .fill_nonzero()
                    .restore_state();
            }
        }
    }
    c.restore_state();
}
