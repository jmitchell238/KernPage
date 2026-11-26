//! Filled, softly coloured ribbon icons (our own drawings, Office-like palette). Drawn on a
//! 16 x 16 grid and laid out in whole device pixels so they stay crisp at 16 px and 28-32 px:
//! bars and outlines are a whole number of device pixels thick, and stacks of parallel lines
//! share one thickness and one pitch. The same shapes are drawn flat light grey when disabled.

use crate::icons::Icon;
use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::cell::RefCell;

pub const INK: Color32 = Color32::from_rgb(0x42, 0x42, 0x42);
pub const BLUE: Color32 = Color32::from_rgb(0x2B, 0x79, 0xD3);
pub const LBLUE: Color32 = Color32::from_rgb(0xCF, 0xE4, 0xFA);
pub const GOLD: Color32 = Color32::from_rgb(0xE8, 0xB3, 0x3A);
pub const GREEN: Color32 = Color32::from_rgb(0x3A, 0x9B, 0x4A);
pub const RED: Color32 = Color32::from_rgb(0xD1, 0x34, 0x38);
pub const PINK: Color32 = Color32::from_rgb(0xF6, 0xB3, 0xB5);
pub const GREYMID: Color32 = Color32::from_rgb(0xA0, 0xA0, 0xA0);
pub const AMBER: Color32 = Color32::from_rgb(0xB9, 0x7F, 0x16);
pub const DBLUE: Color32 = Color32::from_rgb(0x1F, 0x5F, 0xA8);
const WHITE: Color32 = Color32::WHITE;

/// Thickness (grid units) of a thin text line; becomes a whole number of device pixels.
const TL: f32 = 1.0;

/// The flat colour a palette colour takes in the disabled state: dark and mid colours become the
/// theme's disabled grey, light fills stay light so shapes keep their structure.
pub fn grey_of(c: Color32) -> Color32 {
    let l = (0.3 * c.r() as f32 + 0.59 * c.g() as f32 + 0.11 * c.b() as f32) / 255.0;
    let v = if l < 0.7 { 0xA6 } else { (0xE4 as f32 + (l - 0.7) / 0.3 * 27.0) as u8 };
    Color32::from_rgb(v, v, v)
}

/// Lays `n` parallel lines of `thick` device px into `top..bottom`: identical thickness and
/// identical pitch (so identical gaps), centred in the span. Returns `(top, height)` per line.
pub fn stack_rows(top: i32, bottom: i32, n: usize, thick: i32) -> Vec<(i32, i32)> {
    if n == 0 {
        return vec![];
    }
    let total = bottom - top;
    if n == 1 {
        return vec![(top + (total - thick).max(0) / 2, thick)];
    }
    let pitch = ((total - thick) / (n as i32 - 1)).max(thick + 1);
    let used = (n as i32 - 1) * pitch + thick;
    let off = ((total - used) / 2).max(0);
    (0..n as i32).map(|i| (top + off + i * pitch, thick)).collect()
}

struct C<'a> {
    /// The painter (None when recording SVG).
    p: Option<&'a Painter>,
    /// Recording mode: SVG elements are appended here instead of painting.
    rec: Option<&'a RefCell<String>>,
    /// Icon origin in device pixels (whole numbers).
    od: (f32, f32),
    /// Device pixels per grid unit.
    k: f32,
    ppp: f32,
    grey: bool,
}

type P = (f32, f32);

impl C<'_> {
    fn pt(&self) -> &Painter {
        self.p.expect("painter")
    }
    /// Device px -> SVG units (recording only).
    fn u(&self, v: f32) -> String {
        num((v / self.k * 4.0).round() / 4.0)
    }
    /// SVG colour: INK is `currentColor`.
    fn sc(&self, c: Color32) -> String {
        if c == INK { "currentColor".into() } else { format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()) }
    }
    fn emit(&self, s: String) {
        self.rec.unwrap().borrow_mut().push_str(&s);
    }
    fn col(&self, c: Color32) -> Color32 {
        if self.grey { grey_of(c) } else { c }
    }
    fn dx(&self, x: f32) -> f32 {
        self.od.0 + x * self.k
    }
    fn dy(&self, y: f32) -> f32 {
        self.od.1 + y * self.k
    }
    /// A grid coordinate snapped to a whole device pixel.
    fn rx(&self, x: f32) -> f32 {
        self.dx(x).round()
    }
    fn ry(&self, y: f32) -> f32 {
        self.dy(y).round()
    }
    fn pp(&self, d: P) -> Pos2 {
        Pos2::new(d.0 / self.ppp, d.1 / self.ppp)
    }
    /// Whole device pixels for a thickness given in grid units (at least 1).
    fn thick(&self, t: f32) -> f32 {
        (t * self.k).round().max(1.0)
    }
    /// Outline width in device pixels.
    fn lw(&self) -> f32 {
        if self.rec.is_some() {
            return self.k;
        }
        (self.k * 0.6).round().max(1.0)
    }
    /// Stroke width in points for lines and curves, at least 1.5 device px.
    fn sw(&self, w_units: f32) -> f32 {
        (w_units * self.k).max(1.5) / self.ppp
    }
    fn fill_dev(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        if self.rec.is_some() {
            let (a, b) = (self.u(x0), self.u(y0));
            let (w, h) = (num(((x1 - x0) / self.k * 4.0).round() / 4.0), num(((y1 - y0) / self.k * 4.0).round() / 4.0));
            self.emit(format!("<rect x=\"{a}\" y=\"{b}\" width=\"{w}\" height=\"{h}\" fill=\"{}\"/>", self.sc(c)));
            return;
        }
        self.pt().rect_filled(Rect::from_min_max(self.pp((x0, y0)), self.pp((x1, y1))), 0.0, self.col(c));
    }
    /// A pixel-snapped rectangle with an optional fill and an inside outline.
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Option<Color32>, line: Option<Color32>) {
        let (x0, y0) = (self.rx(x0), self.ry(y0));
        let (x1, y1) = (self.rx(x1).max(x0 + 1.0), self.ry(y1).max(y0 + 1.0));
        if let Some(f) = fill {
            self.fill_dev(x0, y0, x1, y1, f);
        }
        if let Some(l) = line {
            let w = self.lw().min(((x1 - x0) / 2.0).floor()).min(((y1 - y0) / 2.0).floor()).max(1.0);
            self.fill_dev(x0, y0, x1, y0 + w, l);
            self.fill_dev(x0, y1 - w, x1, y1, l);
            self.fill_dev(x0, y0 + w, x0 + w, y1 - w, l);
            self.fill_dev(x1 - w, y0 + w, x1, y1 - w, l);
        }
    }
    fn hbar(&self, y: f32, x0: f32, x1: f32, t: f32, c: Color32) {
        let tp = self.thick(t);
        let top = (self.dy(y) - tp / 2.0).round();
        self.fill_dev(self.rx(x0), top, self.rx(x1), top + tp, c);
    }
    fn vbar(&self, x: f32, y0: f32, y1: f32, t: f32, c: Color32) {
        let tp = self.thick(t);
        let left = (self.dx(x) - tp / 2.0).round();
        self.fill_dev(left, self.ry(y0), left + tp, self.ry(y1), c);
    }
    /// `n` equal parallel horizontal lines across `y0..y1`, as device-pixel `(top, height)`.
    fn rows(&self, y0: f32, y1: f32, n: usize, t: f32) -> Vec<(f32, f32)> {
        stack_rows(self.ry(y0) as i32, self.ry(y1) as i32, n, self.thick(t) as i32)
            .into_iter()
            .map(|(a, h)| (a as f32, h as f32))
            .collect()
    }
    /// A horizontal line on a device row.
    fn hrow(&self, row: (f32, f32), x0: f32, x1: f32, c: Color32) {
        self.fill_dev(self.rx(x0), row.0, self.rx(x1), row.0 + row.1, c);
    }
    /// A filled square marker `m` device px wide, centred on a device row, starting at grid x.
    fn marker(&self, row: (f32, f32), x: f32, m: f32, round: bool, c: Color32) {
        let top = row.0 + ((row.1 - m) / 2.0).floor();
        let left = self.rx(x);
        if round {
            if self.rec.is_some() {
                let (cx, cy) = (self.u(left + m / 2.0), self.u(top + m / 2.0));
                self.emit(format!("<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"{}\"/>", self.u(m / 2.0), self.sc(c)));
                return;
            }
            let ctr = self.pp((left + m / 2.0, top + m / 2.0));
            self.pt().circle_filled(ctr, m / 2.0 / self.ppp, self.col(c));
        } else {
            self.fill_dev(left, top, left + m, top + m, c);
        }
    }
    fn poly_dev(&self, pts: &[P], fill: Option<Color32>, line: Option<Color32>) {
        if self.rec.is_some() {
            let pts: Vec<String> = pts.iter().map(|d| format!("{},{}", self.u(d.0), self.u(d.1))).collect();
            let f = fill.map_or("none".to_string(), |f| self.sc(f));
            let st = line.map_or(String::new(), |l| format!(" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"", self.sc(l), self.u(self.lw())));
            self.emit(format!("<polygon points=\"{}\" fill=\"{f}\"{st}/>", pts.join(" ")));
            return;
        }
        let v: Vec<Pos2> = pts.iter().map(|&d| self.pp(d)).collect();
        let st = match line {
            Some(l) => Stroke::new(self.lw() / self.ppp, self.col(l)),
            None => Stroke::NONE,
        };
        self.pt().add(Shape::convex_polygon(v, fill.map_or(Color32::TRANSPARENT, |f| self.col(f)), st));
    }
    fn poly(&self, pts: &[P], fill: Option<Color32>, line: Option<Color32>) {
        let v: Vec<P> = pts.iter().map(|&(x, y)| (self.dx(x), self.dy(y))).collect();
        self.poly_dev(&v, fill, line);
    }
    /// A polygon whose vertices sit on whole device pixels (crisp straight edges).
    fn polys(&self, pts: &[P], fill: Option<Color32>, line: Option<Color32>) {
        let off = if line.is_some() && self.lw() % 2.0 == 1.0 { 0.5 } else { 0.0 };
        let v: Vec<P> = pts.iter().map(|&(x, y)| (self.rx(x) + off, self.ry(y) + off)).collect();
        self.poly_dev(&v, fill, line);
    }
    fn path(&self, pts: &[P], c: Color32, w: f32) {
        if self.rec.is_some() {
            let pts: Vec<String> = pts.iter().map(|&(x, y)| format!("{},{}", self.u(self.dx(x)), self.u(self.dy(y)))).collect();
            let w = num(((w * self.k).max(1.5) / self.k * 4.0).round() / 4.0);
            self.emit(format!("<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{w}\" stroke-linejoin=\"round\"/>", pts.join(" "), self.sc(c)));
            return;
        }
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pp((self.dx(x), self.dy(y)))).collect();
        self.pt().add(Shape::line(v, Stroke::new(self.sw(w), self.col(c))));
    }
    fn seg(&self, a: P, b: P, c: Color32, w: f32) {
        self.path(&[a, b], c, w);
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: Option<Color32>, line: Option<Color32>, w: f32) {
        if self.rec.is_some() {
            let f = fill.map_or("none".to_string(), |f| self.sc(f));
            let st = line.map_or(String::new(), |l| format!(" stroke=\"{}\" stroke-width=\"{}\"", self.sc(l), num(((w * self.k).max(1.5) / self.k * 4.0).round() / 4.0)));
            self.emit(format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{f}\"{st}/>", self.u(self.dx(x)), self.u(self.dy(y)), num((r * 4.0).round() / 4.0)));
            return;
        }
        let ctr = self.pp((self.dx(x), self.dy(y)));
        if let Some(f) = fill {
            self.pt().circle_filled(ctr, r * self.k / self.ppp, self.col(f));
        }
        if let Some(l) = line {
            self.pt().circle_stroke(ctr, r * self.k / self.ppp, Stroke::new(self.sw(w), self.col(l)));
        }
    }
    fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<P> {
        (0..=16)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / 16.0).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }
    /// A solid arrow head: tip, direction (degrees, screen coordinates), length, half width.
    fn head(&self, tip: P, ang: f32, len: f32, half: f32, c: Color32) {
        let a = ang.to_radians();
        let (dx, dy) = (a.cos(), a.sin());
        let bx = (tip.0 - dx * len, tip.1 - dy * len);
        self.poly(&[tip, (bx.0 - dy * half, bx.1 + dx * half), (bx.0 + dy * half, bx.1 - dx * half)], Some(c), None);
    }
    /// A dashed rectangle outline made of whole-pixel dashes.
    fn dashed(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        let (x0, y0, x1, y1) = (self.rx(x0), self.ry(y0), self.rx(x1), self.ry(y1));
        let w = self.lw();
        let dash = (1.5 * self.k).round().max(2.0);
        let gap = (1.2 * self.k).round().max(1.0);
        let run = |from: f32, to: f32, f: &dyn Fn(f32, f32)| {
            let mut a = from;
            while a < to {
                f(a, (a + dash).min(to));
                a += dash + gap;
            }
        };
        run(x0, x1, &|a, b| {
            self.fill_dev(a, y0, b, y0 + w, c);
            self.fill_dev(a, y1 - w, b, y1, c);
        });
        run(y0, y1, &|a, b| {
            self.fill_dev(x0, a, x0 + w, b, c);
            self.fill_dev(x1 - w, a, x1, b, c);
        });
    }
    /// A capital A (strokes `t` units thick, crossbar a little lighter).
    fn letter_a(&self, x0: f32, x1: f32, top: f32, bot: f32, t: f32, c: Color32) {
        let mx = (x0 + x1) / 2.0;
        self.path(&[(x0, bot), (mx, top), (x1, bot)], c, t);
        let by = top + (bot - top) * 0.62;
        let k = (by - top) / (bot - top) * (x1 - x0) / 2.0;
        self.hbar(by, mx - k + 0.4, mx + k - 0.4, t * 0.8, c);
    }
    /// Device pixels per cell of a pixel glyph of `rows` rows meant to span about `units` grid units.
    fn gu(&self, rows: usize, units: f32) -> f32 {
        (units * self.k / rows as f32).round().max(1.0)
    }
    /// A pixel glyph (`#` = lit cell) with its top left at device `(left, top)`, `u` device px per cell.
    fn glyph(&self, pat: &[&str], left: f32, top: f32, u: f32, c: Color32) {
        for (gy, line) in pat.iter().enumerate() {
            let y = top + gy as f32 * u;
            let b = line.as_bytes();
            let mut gx = 0;
            while gx < b.len() {
                if b[gx] == b'#' {
                    let start = gx;
                    while gx < b.len() && b[gx] == b'#' {
                        gx += 1;
                    }
                    self.fill_dev(left + start as f32 * u, y, left + gx as f32 * u, y + u, c);
                } else {
                    gx += 1;
                }
            }
        }
    }
    /// A pixel glyph centred on a grid point.
    fn glyph_c(&self, pat: &[&str], cx: f32, cy: f32, u: f32, c: Color32) {
        let w = pat.iter().map(|l| l.len()).max().unwrap_or(0) as f32;
        let left = (self.dx(cx) - w * u / 2.0).round();
        let top = (self.dy(cy) - pat.len() as f32 * u / 2.0).round();
        self.glyph(pat, left, top, u, c);
    }
    fn page(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32) {
        self.rect(x0, y0, x1, y1, Some(fill), Some(INK));
    }
}

fn num(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Horizontal extents (grid units) of line `i` of 4 in an alignment icon.
fn align_span(icon: Icon, i: usize) -> (f32, f32) {
    let short = i % 2 == 1 && icon != Icon::AlignJustify;
    if !short {
        return (1.5, 14.5);
    }
    match icon {
        Icon::AlignLeft => (1.5, 10.0),
        Icon::AlignCenter => (4.0, 12.0),
        _ => (6.0, 14.5),
    }
}

/// Paints `icon` in colour (or flat grey when `grey`); false if this icon has no coloured version.
pub fn paint(p: &Painter, r: Rect, icon: Icon, grey: bool) -> bool {
    let side = r.width().min(r.height());
    let ppp = p.ctx().pixels_per_point();
    let o = r.center() - Vec2::splat(side / 2.0);
    let od = ((o.x * ppp).round(), (o.y * ppp).round());
    let c = C { p: Some(p), rec: None, od, k: side / 16.0 * ppp, ppp, grey };
    draw(&c, icon)
}

fn draw(c: &C, icon: Icon) -> bool {
    use Icon::*;
    match icon {
        Save => {
            c.rect(1.5, 1.5, 14.5, 14.5, Some(BLUE), Some(INK));
            c.rect(4.0, 3.0, 12.0, 6.5, Some(LBLUE), None);
            c.rect(3.5, 9.0, 12.5, 14.0, Some(WHITE), None);
            for row in c.rows(10.0, 13.0, 2, TL) {
                c.hrow(row, 5.0, 11.0, GREYMID);
            }
        }
        Undo | Redo => {
            let m = |x: f32| if icon == Undo { x } else { 16.0 - x };
            // Head at the top left pointing outward; the shaft runs straight
            // from it, then curls round the right side and down.
            let mut pts: Vec<P> = vec![(5.0, 5.0)];
            pts.extend(c.arc(8.5, 9.5, 4.5, 270.0, 430.0));
            let pts: Vec<P> = pts.iter().map(|&(x, y)| (m(x), y)).collect();
            c.path(&pts, BLUE, 2.0);
            let dir = if icon == Undo { 180.0 } else { 0.0 };
            c.head((m(1.2), 5.0), dir, 4.2, 3.2, BLUE);
        }
        Paste => {
            c.rect(2.0, 2.5, 12.0, 15.0, Some(GOLD), Some(INK));
            c.rect(4.5, 1.0, 9.5, 4.0, Some(GREYMID), Some(INK));
            c.page(7.0, 6.0, 14.5, 15.0, WHITE);
            for row in c.rows(8.0, 13.0, 3, TL) {
                c.hrow(row, 8.5, 13.0, BLUE);
            }
        }
        Cut => {
            c.seg((4.5, 1.0), (10.5, 9.5), INK, 1.8);
            c.seg((11.5, 1.0), (5.5, 9.5), GREYMID, 1.8);
            c.circle(5.0, 12.5, 2.4, Some(WHITE), Some(BLUE), 1.7);
            c.circle(11.0, 12.5, 2.4, Some(WHITE), Some(BLUE), 1.7);
        }
        Copy => {
            c.page(1.5, 1.0, 9.5, 11.0, WHITE);
            c.page(6.0, 5.0, 14.5, 15.0, LBLUE);
            for row in c.rows(8.0, 13.0, 3, TL) {
                c.hrow(row, 8.0, 12.5, BLUE);
            }
        }
        Brush => {
            c.seg((14.0, 2.0), (9.6, 6.4), GOLD, 2.6);
            c.seg((9.6, 6.4), (7.6, 8.4), GREYMID, 3.4);
            c.poly(&[(6.2, 7.8), (8.4, 10.0), (5.0, 14.0), (2.0, 14.6), (2.5, 11.4)], Some(BLUE), Some(INK));
        }
        GrowFont | ShrinkFont => {
            c.letter_a(0.8, 10.0, 3.5, 14.5, 1.5, INK);
            let tri: [P; 3] = if icon == GrowFont {
                [(10.5, 7.0), (15.0, 7.0), (12.75, 3.0)]
            } else {
                [(10.5, 3.0), (15.0, 3.0), (12.75, 7.0)]
            };
            c.poly(&tri, Some(BLUE), None);
        }
        ClearFormat => {
            c.letter_a(0.8, 9.5, 2.0, 13.5, 1.5, INK);
            c.poly(&[(8.5, 13.5), (11.5, 7.5), (15.0, 9.5), (12.0, 15.0)], Some(PINK), Some(RED));
        }
        FontColor | FontColorLetter => {
            c.letter_a(2.5, 13.5, 1.5, 11.5, 1.8, INK);
            if icon == FontColor {
                c.rect(1.5, 13.0, 14.5, 15.5, Some(RED), None);
            }
        }
        Bullets | Numbering => {
            let m = c.thick(2.4).max(3.0);
            let rows = c.rows(1.5, 14.5, 3, TL);
            // Digits: a 3 x 4 pixel font, whole device pixels per cell, at least 2 px between digits.
            let u = ((rows[1].0 - rows[0].0 - 2.0) / 4.0).floor().max(1.0);
            const GLYPHS: [[&str; 4]; 3] = [
                [".#.", "##.", ".#.", "###"],
                ["##.", "..#", ".#.", "###"],
                ["##.", "..#", "..#", "##."],
            ];
            for (i, row) in rows.iter().enumerate() {
                if icon == Bullets {
                    c.marker(*row, 1.5, m, true, BLUE);
                } else {
                    let top = row.0 + (row.1 / 2.0).floor() - (2.0 * u).floor();
                    c.glyph(&GLYPHS[i], c.rx(1.5), top, u, BLUE);
                }
                c.hrow(*row, 6.5, 14.5, INK);
            }
        }
        IndentLess | IndentMore => {
            let rows = c.rows(2.0, 14.0, 4, TL);
            for (i, row) in rows.iter().enumerate() {
                let x0 = if i == 1 || i == 2 { 7.0 } else { 1.5 };
                c.hrow(*row, x0, 14.5, INK);
            }
            // Arrow between the two indented lines.
            let (ya, yb) = (rows[1].0 - 1.0, rows[2].0 + rows[2].1 + 1.0);
            let ym = ((ya + yb) / 2.0).round();
            let (xa, xb) = (c.rx(1.5), c.rx(5.0));
            let tri = if icon == IndentMore {
                [(xa, ya), (xb, ym), (xa, yb)]
            } else {
                [(xb, ya), (xa, ym), (xb, yb)]
            };
            c.poly_dev(&tri, Some(BLUE), None);
        }
        AlignLeft | AlignCenter | AlignRight | AlignJustify => {
            for (i, row) in c.rows(2.0, 14.0, 4, TL).into_iter().enumerate() {
                let (x0, x1) = align_span(icon, i);
                c.hrow(row, x0, x1, INK);
            }
        }
        LineSpacing => {
            for row in c.rows(2.0, 14.0, 3, TL) {
                c.hrow(row, 7.5, 14.5, INK);
            }
            c.vbar(3.5, 4.5, 11.5, TL, BLUE);
            c.poly(&[(1.0, 5.0), (6.0, 5.0), (3.5, 1.0)], Some(BLUE), None);
            c.poly(&[(1.0, 11.0), (6.0, 11.0), (3.5, 15.0)], Some(BLUE), None);
        }
        Styles => {
            c.rect(1.0, 2.0, 15.0, 14.0, Some(WHITE), Some(INK));
            c.letter_a(2.8, 7.8, 4.5, 11.5, 1.9, INK);
            for (i, row) in c.rows(4.5, 11.5, 3, TL).into_iter().enumerate() {
                c.hrow(row, 9.5, if i == 1 { 12.0 } else { 13.0 }, if i == 0 { BLUE } else { GREYMID });
            }
        }
        TextBox => {
            c.rect(2.0, 3.5, 14.0, 12.5, Some(WHITE), Some(BLUE));
            for (x, y) in [(2.0, 3.5), (14.0, 3.5), (2.0, 12.5), (14.0, 12.5)] {
                c.rect(x - 1.4, y - 1.4, x + 1.4, y + 1.4, Some(WHITE), Some(BLUE));
            }
            c.letter_a(5.0, 11.0, 5.5, 11.0, 1.4, INK);
        }
        Picture => {
            c.rect(1.0, 2.5, 15.0, 13.5, Some(LBLUE), Some(INK));
            c.circle(11.5, 5.8, 1.8, Some(GOLD), None, 0.0);
            c.poly(&[(1.6, 13.0), (5.5, 7.0), (9.4, 13.0)], Some(GREEN), None);
            c.poly(&[(7.0, 13.0), (10.8, 9.0), (14.4, 13.0)], Some(Color32::from_rgb(0x2B, 0x7A, 0x3A)), None);
        }
        Table => {
            c.rect(1.0, 2.5, 15.0, 13.5, Some(WHITE), Some(INK));
            c.rect(1.0, 2.5, 15.0, 6.0, Some(BLUE), Some(INK));
            c.hbar(9.5, 1.0, 15.0, TL, GREYMID);
            for x in [5.5, 10.5] {
                c.vbar(x, 6.0, 13.5, TL, GREYMID);
            }
        }
        Shapes => {
            c.rect(1.0, 1.0, 9.0, 9.0, Some(LBLUE), Some(BLUE));
            c.circle(10.3, 10.3, 4.5, Some(GOLD), Some(INK), 1.0);
        }
        Find => {
            c.seg((10.0, 10.0), (14.5, 14.5), INK, 2.4);
            c.circle(6.3, 6.3, 4.4, Some(LBLUE), Some(INK), 1.7);
        }
        Replace => {
            c.page(1.0, 1.5, 8.5, 9.5, WHITE);
            c.page(7.5, 6.5, 15.0, 14.5, LBLUE);
            c.path(&c.arc(8.0, 6.0, 3.0, 200.0, 330.0), GREEN, 1.8);
            c.head((12.0, 6.2), 75.0, 3.4, 2.2, GREEN);
        }
        Select => {
            c.poly(&[(3.0, 1.5), (3.0, 13.0), (6.4, 9.8), (12.2, 9.3)], Some(INK), None);
            c.poly(&[(6.6, 9.8), (8.6, 9.0), (11.0, 14.0), (9.0, 14.8)], Some(INK), None);
        }
        Calendar => {
            c.rect(1.5, 2.5, 14.5, 14.5, Some(WHITE), Some(INK));
            c.rect(1.5, 2.5, 14.5, 6.0, Some(RED), Some(INK));
            c.rect(4.0, 1.0, 5.3, 4.0, Some(INK), None);
            c.rect(10.7, 1.0, 12.0, 4.0, Some(INK), None);
            let cell = c.thick(2.0).max(2.0) as i32;
            let xs = stack_rows(c.rx(3.0) as i32, c.rx(13.0) as i32, 3, cell);
            let ys = stack_rows(c.ry(7.5) as i32, c.ry(13.0) as i32, 2, cell);
            for &(y, h) in &ys {
                for &(x, w) in &xs {
                    c.fill_dev(x as f32, y as f32, (x + w) as f32, (y + h) as f32, BLUE);
                }
            }
        }
        Page => {
            c.polys(&[(3.5, 1.0), (9.5, 1.0), (12.5, 4.0), (12.5, 15.0), (3.5, 15.0)], Some(WHITE), Some(INK));
            c.polys(&[(9.5, 1.0), (9.5, 4.0), (12.5, 4.0)], Some(LBLUE), Some(INK));
            for row in c.rows(7.0, 12.5, 3, TL) {
                c.hrow(row, 5.5, 10.5, BLUE);
            }
        }
        Pages => {
            c.page(1.5, 1.5, 9.5, 12.0, LBLUE);
            c.page(5.5, 4.0, 13.5, 14.5, WHITE);
            for row in c.rows(7.0, 12.0, 3, TL) {
                c.hrow(row, 7.5, 11.5, BLUE);
            }
        }
        Header | Footer => {
            c.page(3.0, 1.0, 13.0, 15.0, WHITE);
            let (y0, y1) = if icon == Header { (1.0, 5.0) } else { (11.0, 15.0) };
            c.rect(3.0, y0, 13.0, y1, Some(LBLUE), Some(INK));
            let (a, b) = if icon == Header { (7.0, 13.0) } else { (3.0, 9.0) };
            for row in c.rows(a, b, 3, TL) {
                c.hrow(row, 5.0, 11.0, GREYMID);
            }
        }
        Link => {
            c.seg((6.5, 9.5), (9.5, 6.5), INK, 1.8);
            c.circle(5.0, 11.0, 3.2, None, Some(BLUE), 2.0);
            c.circle(11.0, 5.0, 3.2, None, Some(BLUE), 2.0);
        }
        Bookmark => {
            c.polys(&[(4.0, 1.0), (12.0, 1.0), (12.0, 15.0), (8.0, 11.0)], Some(RED), None);
            c.polys(&[(4.0, 1.0), (8.0, 11.0), (4.0, 15.0)], Some(RED), None);
        }
        Hash => {
            c.seg((6.0, 2.0), (4.5, 14.0), BLUE, 1.3);
            c.seg((11.5, 2.0), (10.0, 14.0), BLUE, 1.3);
            c.hbar(5.5, 2.0, 14.0, 1.3, BLUE);
            c.hbar(10.5, 1.5, 13.5, 1.3, BLUE);
        }
        Wrap => {
            for row in c.rows(2.0, 5.5, 2, TL).into_iter().chain(c.rows(10.5, 14.0, 2, TL)) {
                c.hrow(row, 1.0, 15.0, INK);
            }
            c.circle(8.0, 8.0, 2.8, Some(LBLUE), Some(BLUE), 1.0);
        }
        Forward | Backward => {
            if icon == Forward {
                c.rect(1.0, 1.0, 10.0, 10.0, Some(WHITE), Some(INK));
                c.rect(6.0, 6.0, 15.0, 15.0, Some(BLUE), Some(INK));
            } else {
                c.rect(6.0, 6.0, 15.0, 15.0, Some(BLUE), Some(INK));
                c.rect(1.0, 1.0, 10.0, 10.0, Some(WHITE), Some(INK));
            }
        }
        AlignObjects => {
            c.vbar(2.0, 1.0, 15.0, TL, INK);
            c.rect(4.0, 3.0, 14.5, 7.0, Some(BLUE), None);
            c.rect(4.0, 9.5, 10.0, 13.5, Some(LBLUE), Some(BLUE));
        }
        Group => {
            c.dashed(1.5, 1.5, 14.5, 14.5, INK);
            c.rect(4.0, 4.0, 9.0, 9.0, Some(LBLUE), Some(BLUE));
            c.rect(7.0, 7.0, 12.0, 12.0, Some(BLUE), Some(INK));
        }
        Ungroup => {
            c.rect(1.5, 1.5, 7.0, 7.0, Some(LBLUE), Some(BLUE));
            c.rect(9.0, 9.0, 14.5, 14.5, Some(BLUE), Some(INK));
        }
        Rotate => {
            c.rect(5.0, 5.0, 11.0, 11.0, Some(LBLUE), Some(BLUE));
            c.path(&c.arc(8.0, 8.0, 6.2, -150.0, 50.0), GREEN, 1.9);
            c.head((5.6, 14.6), 140.0, 3.6, 2.2, GREEN);
        }
        Columns => {
            c.page(1.5, 1.0, 14.5, 15.0, WHITE);
            for (x0, x1) in [(3.5, 7.2), (8.8, 12.5)] {
                for row in c.rows(3.0, 13.0, 4, TL) {
                    c.hrow(row, x0, x1, BLUE);
                }
            }
        }
        Pilcrow => {
            c.circle(6.6, 5.4, 3.4, Some(BLUE), None, 0.0);
            c.vbar(9.0, 2.0, 14.0, 1.5, BLUE);
            c.vbar(11.8, 2.0, 14.0, 1.5, BLUE);
            c.hbar(2.0, 6.0, 12.5, 1.5, BLUE);
        }
        Shading => {
            c.rect(1.5, 1.5, 14.5, 14.5, Some(LBLUE), Some(INK));
            let bands = [LBLUE, Color32::from_rgb(0x9C, 0xC6, 0xF0), Color32::from_rgb(0x5E, 0xA0, 0xE3), BLUE];
            let (x0, x1) = (c.rx(1.5) + c.lw(), c.rx(14.5) - c.lw());
            let (y0, y1) = (c.ry(1.5) + c.lw(), c.ry(14.5) - c.lw());
            let bw = ((x1 - x0) / 4.0).floor();
            for (i, b) in bands.iter().enumerate() {
                let right = if i == 3 { x1 } else { x0 + bw * (i as f32 + 1.0) };
                c.fill_dev(x0 + bw * i as f32, y0, right, y1, *b);
            }
        }
        Frame => {
            c.rect(1.5, 1.5, 14.5, 14.5, Some(WHITE), Some(INK));
            c.rect(4.5, 4.5, 11.5, 11.5, Some(LBLUE), Some(BLUE));
        }
        Blocks => {
            c.page(2.5, 1.0, 13.5, 15.0, WHITE);
            c.rect(4.5, 3.0, 11.5, 7.5, Some(LBLUE), Some(BLUE));
            for row in c.rows(9.5, 13.0, 2, TL) {
                c.hrow(row, 4.5, 11.5, GREYMID);
            }
        }
        Gear => {
            for i in 0..8 {
                let a = (i as f32 * 45.0).to_radians();
                let (ca, sa) = (a.cos(), a.sin());
                c.seg((8.0 + 4.5 * ca, 8.0 + 4.5 * sa), (8.0 + 7.2 * ca, 8.0 + 7.2 * sa), INK, 2.4);
            }
            c.circle(8.0, 8.0, 5.0, Some(GREYMID), Some(INK), 1.2);
            c.circle(8.0, 8.0, 2.2, Some(WHITE), Some(INK), 1.2);
        }
        Margins => {
            c.page(2.5, 1.0, 13.5, 15.0, WHITE);
            c.dashed(4.8, 3.3, 11.2, 12.7, BLUE);
        }
        Orientation => {
            c.page(2.5, 1.0, 8.5, 10.0, WHITE);
            c.page(6.0, 8.0, 14.5, 14.5, LBLUE);
        }
        Size => {
            c.page(2.0, 4.5, 11.0, 14.5, WHITE);
            c.hbar(2.0, 2.0, 11.0, TL, BLUE);
            c.vbar(2.0, 0.8, 3.2, TL, BLUE);
            c.vbar(11.0, 0.8, 3.2, TL, BLUE);
            c.vbar(14.0, 4.5, 14.5, TL, BLUE);
            c.hbar(4.5, 12.8, 15.2, TL, BLUE);
            c.hbar(14.5, 12.8, 15.2, TL, BLUE);
        }
        Guides => {
            c.page(1.5, 1.5, 14.5, 14.5, WHITE);
            c.hbar(6.0, 0.8, 15.2, TL, BLUE);
            c.vbar(6.0, 0.8, 15.2, TL, BLUE);
        }
        Trash => {
            c.rect(6.0, 1.0, 10.0, 3.5, None, Some(INK));
            c.polys(&[(4.0, 5.0), (12.0, 5.0), (11.0, 15.0), (5.0, 15.0)], Some(LBLUE), Some(INK));
            c.hbar(4.2, 2.0, 14.0, 1.6, INK);
            c.vbar(6.5, 7.0, 13.0, TL, BLUE);
            c.vbar(9.5, 7.0, 13.0, TL, BLUE);
        }
        TableAbove | TableBelow | TableLeft | TableRight | TableDelete => {
            let (x0, y0, x1, y1) = match icon {
                TableAbove => (1.5, 2.0, 14.5, 6.0),
                TableBelow => (1.5, 10.0, 14.5, 14.0),
                TableLeft => (1.5, 2.0, 6.0, 14.0),
                _ => (10.0, 2.0, 14.5, 14.0),
            };
            let track = if icon == TableDelete { PINK } else { LBLUE };
            c.rect(x0, y0, x1, y1, Some(track), None);
            c.rect(1.5, 2.0, 14.5, 14.0, None, Some(INK));
            for v in [6.0, 10.0] {
                c.hbar(v, 1.5, 14.5, TL, GREYMID);
                c.vbar(v, 2.0, 14.0, TL, GREYMID);
            }
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            if icon == TableDelete {
                c.seg((cx - 1.4, cy - 1.4), (cx + 1.4, cy + 1.4), RED, 1.2);
                c.seg((cx - 1.4, cy + 1.4), (cx + 1.4, cy - 1.4), RED, 1.2);
            } else {
                c.hbar(cy, cx - 1.6, cx + 1.6, TL, GREEN);
                c.vbar(cx, cy - 1.6, cy + 1.6, TL, GREEN);
            }
        }
        Move => {
            c.hbar(8.0, 3.0, 13.0, 1.2, BLUE);
            c.vbar(8.0, 3.0, 13.0, 1.2, BLUE);
            c.head((8.0, 0.8), 270.0, 3.4, 2.6, BLUE);
            c.head((8.0, 15.2), 90.0, 3.4, 2.6, BLUE);
            c.head((0.8, 8.0), 180.0, 3.4, 2.6, BLUE);
            c.head((15.2, 8.0), 0.0, 3.4, 2.6, BLUE);
        }
        Rename => {
            c.rect(1.0, 4.5, 15.0, 11.5, Some(WHITE), Some(BLUE));
            c.vbar(4.5, 6.0, 10.0, TL, INK);
            c.hbar(8.0, 7.0, 12.5, TL, GREYMID);
        }
        Palette => {
            for (x0, y0, col) in [(1.5, 1.5, RED), (8.5, 1.5, GOLD), (1.5, 8.5, GREEN), (8.5, 8.5, BLUE)] {
                c.rect(x0, y0, x0 + 6.0, y0 + 6.0, Some(col), None);
            }
        }
        Background => {
            c.page(2.5, 1.0, 13.5, 15.0, LBLUE);
            c.circle(9.5, 5.0, 1.7, Some(GOLD), None, 0.0);
            c.polys(&[(3.1, 14.4), (3.1, 10.0), (7.0, 7.5), (10.5, 10.0), (10.5, 14.4)], Some(GREEN), None);
            c.polys(&[(8.0, 14.4), (12.9, 9.5), (12.9, 14.4)], Some(Color32::from_rgb(0x2B, 0x7A, 0x3A)), None);
        }
        Mail => {
            c.rect(1.0, 3.0, 15.0, 13.0, Some(LBLUE), Some(INK));
            c.path(&[(1.8, 3.8), (8.0, 9.0), (14.2, 3.8)], INK, 1.2);
        }
        People => {
            c.circle(8.0, 4.8, 2.8, Some(BLUE), None, 0.0);
            let v: Vec<P> = c.arc(8.0, 14.5, 5.8, 180.0, 360.0).iter().map(|&(x, y)| (c.dx(x), c.dy(y))).collect();
            c.poly_dev(&v, Some(BLUE), None);
        }
        Check => {
            c.path(&[(2.5, 8.5), (6.5, 12.5), (13.5, 3.5)], GREEN, 2.3);
        }
        Globe => {
            c.circle(8.0, 8.0, 6.6, Some(LBLUE), Some(BLUE), 1.4);
            let mer: Vec<P> = (0..=24)
                .map(|i| {
                    let a = (i as f32 * 15.0).to_radians();
                    (8.0 + 2.8 * a.cos(), 8.0 + 6.6 * a.sin())
                })
                .collect();
            c.path(&mer, BLUE, 1.0);
            c.hbar(8.0, 1.5, 14.5, TL, BLUE);
            c.hbar(4.6, 3.0, 13.0, TL, BLUE);
            c.hbar(11.4, 3.0, 13.0, TL, BLUE);
        }
        Clock => {
            c.circle(8.0, 8.0, 6.6, Some(WHITE), Some(BLUE), 1.6);
            c.vbar(8.0, 3.5, 8.5, TL, INK);
            c.hbar(8.0, 7.5, 11.5, TL, INK);
        }
        Arrow => {
            c.hbar(8.0, 1.5, 9.5, 2.0, BLUE);
            c.polys(&[(8.5, 3.5), (14.5, 8.0), (8.5, 12.5)], Some(BLUE), None);
        }
        Back => {
            c.hbar(8.0, 6.5, 14.5, 2.0, BLUE);
            c.polys(&[(7.5, 3.5), (1.5, 8.0), (7.5, 12.5)], Some(BLUE), None);
        }
        Question => {
            c.circle(8.0, 8.0, 7.0, Some(BLUE), None, 0.0);
            let pat = [".###.", "##.##", "...##", "..##.", ".##..", ".##..", ".....", ".##.."];
            c.glyph_c(&pat, 8.0, 8.0, c.gu(8, 9.0), WHITE);
        }
        Info => {
            c.circle(8.0, 8.0, 7.0, Some(BLUE), None, 0.0);
            let pat = [".##.", ".##.", "....", ".##.", ".##.", ".##.", ".##.", ".##."];
            c.glyph_c(&pat, 8.0, 8.0, c.gu(8, 9.0), WHITE);
        }
        Folder => {
            c.rect(1.0, 3.0, 7.0, 7.0, Some(AMBER), Some(INK));
            c.rect(1.0, 5.5, 15.0, 14.0, Some(GOLD), Some(INK));
        }
        New | Close => {
            c.polys(&[(2.5, 1.0), (8.5, 1.0), (11.5, 4.0), (11.5, 15.0), (2.5, 15.0)], Some(WHITE), Some(INK));
            c.polys(&[(8.5, 1.0), (8.5, 4.0), (11.5, 4.0)], Some(LBLUE), Some(INK));
            let (col, cx, cy) = (if icon == New { GREEN } else { RED }, 11.5, 11.5);
            c.circle(cx, cy, 4.0, Some(col), None, 0.0);
            if icon == New {
                c.hbar(cy, cx - 2.0, cx + 2.0, TL, WHITE);
                c.vbar(cx, cy - 2.0, cy + 2.0, TL, WHITE);
            } else {
                c.seg((cx - 1.6, cy - 1.6), (cx + 1.6, cy + 1.6), WHITE, 1.2);
                c.seg((cx - 1.6, cy + 1.6), (cx + 1.6, cy - 1.6), WHITE, 1.2);
            }
        }
        Exit => {
            c.rect(1.5, 1.5, 8.5, 14.5, Some(LBLUE), Some(INK));
            c.hbar(8.0, 5.5, 11.5, 2.0, BLUE);
            c.polys(&[(10.5, 4.5), (15.0, 8.0), (10.5, 11.5)], Some(BLUE), None);
        }
        Print => {
            c.page(4.5, 1.0, 11.5, 6.0, WHITE);
            c.rect(1.0, 5.5, 15.0, 11.5, Some(GREYMID), Some(INK));
            c.rect(12.0, 7.0, 13.5, 8.5, Some(GREEN), None);
            c.page(4.0, 9.5, 12.0, 15.0, WHITE);
            for row in c.rows(11.0, 14.0, 2, TL) {
                c.hrow(row, 5.5, 10.5, BLUE);
            }
        }
        Send => {
            c.poly(&[(1.0, 2.0), (15.0, 8.0), (5.5, 8.0)], Some(BLUE), None);
            c.poly(&[(1.0, 14.0), (15.0, 8.0), (5.5, 8.0)], Some(DBLUE), None);
        }
        Window => {
            c.rect(1.0, 2.0, 15.0, 14.0, Some(LBLUE), Some(INK));
            c.rect(1.0, 2.0, 15.0, 5.5, Some(BLUE), Some(INK));
            for row in c.rows(7.5, 12.5, 2, TL) {
                c.hrow(row, 3.0, 11.0, WHITE);
            }
        }
        Zoom => {
            c.seg((10.0, 10.0), (14.5, 14.5), INK, 2.6);
            c.circle(6.3, 6.3, 4.6, Some(LBLUE), Some(INK), 1.6);
            c.hbar(6.3, 4.0, 8.6, TL, BLUE);
            c.vbar(6.3, 4.0, 8.6, TL, BLUE);
        }
        FitPage => {
            c.page(5.0, 3.5, 11.0, 12.5, WHITE);
            for (x, y, sx, sy) in [(1.0f32, 1.0f32, 1.0f32, 1.0f32), (15.0, 1.0, -1.0, 1.0), (1.0, 15.0, 1.0, -1.0), (15.0, 15.0, -1.0, -1.0)] {
                c.hbar(y, x.min(x + 4.0 * sx), x.max(x + 4.0 * sx), TL, BLUE);
                c.vbar(x, y.min(y + 4.0 * sy), y.max(y + 4.0 * sy), TL, BLUE);
            }
        }
        PageWidth => {
            c.page(4.5, 1.5, 11.5, 14.5, WHITE);
            c.hbar(8.0, 4.0, 12.0, 1.4, BLUE);
            c.polys(&[(4.5, 4.5), (0.8, 8.0), (4.5, 11.5)], Some(BLUE), None);
            c.polys(&[(11.5, 4.5), (15.2, 8.0), (11.5, 11.5)], Some(BLUE), None);
        }
        SinglePage => {
            c.page(3.5, 1.0, 12.5, 15.0, WHITE);
            for row in c.rows(4.0, 12.0, 3, TL) {
                c.hrow(row, 5.5, 10.5, GREYMID);
            }
        }
        TwoPage => {
            c.page(1.0, 2.5, 7.5, 13.5, WHITE);
            c.page(8.5, 2.5, 15.0, 13.5, WHITE);
            for row in c.rows(5.0, 11.0, 3, TL) {
                c.hrow(row, 2.8, 5.8, GREYMID);
                c.hrow(row, 10.3, 13.3, GREYMID);
            }
        }
        Chevron => {
            // Points up, like the old outline Chevron (Minimize the Ribbon).
            c.path(&[(3.5, 10.5), (8.0, 6.0), (12.5, 10.5)], INK, 1.8);
        }
        ChevronDown => {
            // Points down: the ribbon is minimised (click to expand).
            c.path(&[(3.5, 6.0), (8.0, 10.5), (12.5, 6.0)], INK, 1.8);
        }
        Star => {
            let pts: Vec<P> = (0..10)
                .map(|i| {
                    let a = (-90.0 + i as f32 * 36.0).to_radians();
                    let r = if i % 2 == 0 { 7.2 } else { 3.0 };
                    (8.0 + r * a.cos(), 8.6 + r * a.sin())
                })
                .collect();
            let inner: Vec<P> = pts.iter().skip(1).step_by(2).copied().collect();
            c.poly(&inner, Some(GOLD), None);
            for i in (0..10).step_by(2) {
                c.poly(&[pts[(i + 9) % 10], pts[i], pts[i + 1]], Some(GOLD), None);
            }
            let mut ring: Vec<P> = pts.clone();
            ring.push(pts[0]);
            c.path(&ring, AMBER, 1.0);
        }
        Bold => {
            let mut top: Vec<P> = vec![(4.5, 3.0), (8.0, 3.0)];
            top.extend(c.arc(8.0, 5.6, 2.6, -90.0, 90.0));
            top.push((4.5, 8.2));
            c.path(&top, INK, 2.0);
            let mut bot: Vec<P> = vec![(4.5, 8.2), (8.6, 8.2)];
            bot.extend(c.arc(8.6, 11.1, 2.9, -90.0, 90.0));
            bot.push((4.5, 14.0));
            c.path(&bot, INK, 2.0);
            c.vbar(4.5, 2.0, 15.0, 2.2, INK);
        }
        Italic => {
            c.seg((9.4, 3.0), (6.6, 13.0), INK, 2.2);
            c.hbar(3.0, 7.0, 12.5, 1.4, INK);
            c.hbar(13.0, 3.5, 9.0, 1.4, INK);
        }
        Underline => {
            c.path(&[(4.0, 1.5), (4.0, 8.0)], INK, 2.0);
            c.path(&[(12.0, 1.5), (12.0, 8.0)], INK, 2.0);
            c.path(&c.arc(8.0, 8.0, 4.0, 0.0, 180.0), INK, 2.0);
            c.hbar(14.0, 2.5, 13.5, 1.4, BLUE);
        }
        Sub | Sup => {
            let two = [".##.", "#..#", "..#.", ".#..", "####"];
            let u = c.gu(5, 6.0);
            let (xy, ty) = if icon == Sub { (2.0, 9.5) } else { (6.5, 1.0) };
            c.seg((1.5, xy), (8.5, xy + 7.5), INK, 1.8);
            c.seg((1.5, xy + 7.5), (8.5, xy), INK, 1.8);
            c.glyph(&two, c.rx(10.0), c.ry(ty), u, BLUE);
        }
        Case => {
            c.letter_a(0.8, 8.5, 3.0, 14.0, 1.5, INK);
            let a = [".###.", "....#", ".####", "#...#", "#..##", ".##.#"];
            let u = c.gu(10, 11.0);
            c.glyph(&a, c.rx(9.5), (c.dy(14.0) - 6.0 * u).round(), u, BLUE);
        }
        Fonts => {
            c.rect(1.0, 1.5, 15.0, 14.5, Some(LBLUE), Some(BLUE));
            c.letter_a(2.8, 8.0, 3.5, 12.5, 1.4, INK);
            let a = [".###.", "....#", ".####", "#...#", "#..##", ".##.#"];
            let u = c.gu(10, 11.0).min(c.gu(6, 6.5));
            c.glyph(&a, c.rx(9.0), (c.dy(12.5) - 6.0 * u).round(), u, INK);
        }
        Spacing => {
            c.letter_a(4.5, 11.5, 1.5, 10.0, 1.5, INK);
            c.hbar(13.0, 3.5, 12.5, TL, BLUE);
            c.polys(&[(4.0, 10.8), (1.0, 13.0), (4.0, 15.2)], Some(BLUE), None);
            c.polys(&[(12.0, 10.8), (15.0, 13.0), (12.0, 15.2)], Some(BLUE), None);
        }
        Abc => {
            c.letter_a(0.8, 8.0, 2.0, 11.0, 1.5, INK);
            for row in c.rows(2.5, 11.0, 3, TL) {
                c.hrow(row, 9.5, 15.0, BLUE);
            }
            c.hbar(14.0, 1.0, 15.0, TL, GOLD);
        }
        Pointer => c.poly(
            &[(4.5, 2.5), (4.5, 12.5), (7.0, 10.3), (8.8, 14.0), (10.5, 13.2), (8.7, 9.6), (12.0, 9.4)],
            Some(WHITE),
            Some(INK),
        ),
        ObjectSize => {
            c.rect(2.5, 3.5, 13.5, 12.5, None, Some(INK));
            c.path(&[(5.5, 10.5), (10.5, 5.5)], INK, 1.0);
        }
        ZoomFit => {
            c.rect(5.5, 5.5, 10.5, 10.5, None, Some(INK));
            c.path(&[(3.0, 5.5), (1.5, 8.0), (3.0, 10.5)], INK, 1.0);
            c.path(&[(13.0, 5.5), (14.5, 8.0), (13.0, 10.5)], INK, 1.0);
            c.path(&[(5.5, 3.0), (8.0, 1.5), (10.5, 3.0)], INK, 1.0);
            c.path(&[(5.5, 13.0), (8.0, 14.5), (10.5, 13.0)], INK, 1.0);
        }
        Field => {
            c.path(&[(5.5, 2.5), (4.0, 3.5), (4.0, 6.5), (2.5, 8.0), (4.0, 9.5), (4.0, 12.5), (5.5, 13.5)], INK, 1.0);
            c.path(&[(10.5, 2.5), (12.0, 3.5), (12.0, 6.5), (13.5, 8.0), (12.0, 9.5), (12.0, 12.5), (10.5, 13.5)], INK, 1.0);
            c.path(&[(6.5, 8.0), (9.5, 8.0)], BLUE, 1.5);
        }
        Sequence => {
            c.path(&[(2.5, 5.0), (3.5, 4.0), (3.5, 10.5)], INK, 1.0);
            c.path(&[(6.0, 4.5), (8.0, 4.0), (8.5, 5.5), (6.0, 10.5), (8.8, 10.5)], INK, 1.0);
            c.path(&[(11.3, 4.2), (13.8, 4.2), (12.5, 6.8), (13.8, 8.5), (12.0, 10.5), (11.3, 10.5)], INK, 1.0);
            c.path(&[(2.0, 13.5), (14.0, 13.5)], BLUE, 1.25);
        }
        RepeatArea => {
            for (x, y) in [(2.5, 2.5), (9.0, 2.5), (2.5, 9.0), (9.0, 9.0)] {
                c.rect(x, y, x + 4.5, y + 4.5, Some(LBLUE), Some(INK));
                c.path(&[(x + 1.3, y + 2.25), (x + 3.2, y + 2.25)], BLUE, 1.0);
            }
        }
        PerSheet => {
            c.rect(2.5, 1.5, 13.5, 14.5, Some(WHITE), Some(INK));
            for (x, y) in [(4.25, 3.25), (8.75, 3.25), (4.25, 6.75), (8.75, 6.75), (4.25, 10.25), (8.75, 10.25)] {
                c.rect(x, y, x + 3.0, y + 3.0, Some(LBLUE), Some(BLUE));
            }
        }
        Eye => {
            c.poly(&[(1.5, 8.0), (4.5, 5.2), (8.0, 4.5), (11.5, 5.2), (14.5, 8.0), (11.5, 10.8), (8.0, 11.5), (4.5, 10.8)], Some(WHITE), Some(INK));
            c.circle(8.0, 8.0, 2.25, Some(BLUE), None, 0.0);
        }
        CreatePage => {
            c.page(3.5, 1.5, 12.5, 8.5, WHITE);
            c.path(&[(3.5, 8.5), (3.5, 14.5), (9.0, 14.5)], INK, 1.0);
            c.path(&[(8.0, 11.5), (14.5, 11.5)], BLUE, 1.25);
            c.path(&[(12.0, 9.0), (14.5, 11.5), (12.0, 14.0)], BLUE, 1.25);
        }
        Warning => {
            c.poly(&[(8.0, 1.5), (15.0, 14.0), (1.0, 14.0)], Some(GOLD), Some(INK));
            c.vbar(8.0, 5.5, 10.0, 1.5, INK);
            c.vbar(8.0, 11.0, 12.5, 1.5, INK);
        }
        Lightbulb => {
            c.circle(8.0, 6.0, 4.5, Some(GOLD), Some(INK), 1.0);
            c.rect(5.5, 9.5, 10.5, 11.5, Some(GOLD), None);
            c.rect(5.5, 12.5, 10.5, 13.5, Some(INK), None);
            c.rect(6.5, 14.25, 9.5, 15.25, Some(INK), None);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    fn run(ppp: f32, f: impl Fn(&Painter)) -> Vec<egui::epaint::ClippedShape> {
        let ctx = egui::Context::default();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let mut input = egui::RawInput::default();
            input.viewports.get_mut(&input.viewport_id).unwrap().native_pixels_per_point = Some(ppp);
            let mut out = ctx.run_ui(input, |ui| f(&ui.painter().clone()));
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        shapes
    }

    /// Distinct non-white, non-transparent fill colours drawn for `icon`.
    fn colours(icon: Icon, grey: bool) -> Vec<Color32> {
        let mut cols = Vec::new();
        let shapes = run(1.0, |p| {
            let r = Rect::from_min_size(Pos2::new(4.0, 4.0), Vec2::splat(28.0));
            assert!(paint(p, r, icon, grey), "{icon:?} has a coloured version");
        });
        for cs in &shapes {
            collect(&cs.shape, &mut cols);
        }
        cols.sort_by_key(|c| c.to_array());
        cols.dedup();
        cols
    }

    fn collect(s: &Shape, out: &mut Vec<Color32>) {
        match s {
            Shape::Rect(r) => out.push(r.fill),
            Shape::Circle(c) => out.push(c.fill),
            Shape::Path(p) => out.push(p.fill),
            Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }

    #[test]
    fn enabled_icons_are_coloured_and_disabled_are_grey() {
        for icon in [Icon::Save, Icon::Paste, Icon::Table, Icon::Picture, Icon::FontColor, Icon::Calendar] {
            let on = colours(icon, false);
            assert!(on.iter().any(|c| c.r() != c.g() || c.g() != c.b()), "{icon:?} should use colour");
            let off = colours(icon, true);
            assert!(off.iter().all(|c| c.r() == c.g() && c.g() == c.b()), "{icon:?} disabled must be grey only");
        }
    }

    #[test]
    fn grey_mapping_keeps_light_fills_lighter_than_ink() {
        assert_eq!(grey_of(INK), Color32::from_rgb(0xA6, 0xA6, 0xA6));
        assert!(grey_of(LBLUE).r() > grey_of(BLUE).r());
        assert!(grey_of(WHITE).r() >= grey_of(LBLUE).r());
    }

    #[test]
    fn paint_routes_accent_to_colour_and_disabled_to_grey() {
        let ctx = egui::Context::default();
        let mut shape_counts = Vec::new();
        for col in [crate::widgets::ACCENT_COLOR, crate::theme::disabled(), Color32::RED] {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let p = ui.painter().clone();
                crate::icons::paint(&p, Rect::from_min_size(Pos2::ZERO, Vec2::splat(28.0)), Icon::Save, col);
            });
            out.textures_delta.clear();
            shape_counts.push(out.shapes.len());
        }
        // Accent, disabled and any other (mono) colour all use the SVG icon: one image shape each.
        assert_eq!(shape_counts[0], shape_counts[1]);
        assert_eq!(shape_counts[0], shape_counts[2]);
    }

    #[test]
    fn stack_rows_have_equal_thickness_and_gaps() {
        for thick in 1..4 {
            for n in 2..5 {
                for total in 8..60 {
                    let rows = stack_rows(3, 3 + total, n, thick);
                    assert_eq!(rows.len(), n);
                    assert!(rows.iter().all(|r| r.1 == thick));
                    let gaps: Vec<i32> = rows.windows(2).map(|w| w[1].0 - (w[0].0 + w[0].1)).collect();
                    assert!(gaps.iter().all(|g| *g == gaps[0] && *g >= 1), "{rows:?}");
                }
            }
        }
    }

    #[test]
    fn align_left_bars_snap_to_equal_device_rows() {
        for ppp in [1.0f32, 1.25, 1.5, 2.0] {
            for side in [16.0f32, 28.0, 32.0] {
                let shapes = run(ppp, |p| {
                    let r = Rect::from_min_size(Pos2::new(4.3, 4.7), Vec2::splat(side));
                    paint(p, r, Icon::AlignLeft, false);
                });
                let mut bars: Vec<(i32, i32)> = Vec::new();
                for cs in &shapes {
                    if let Shape::Rect(r) = &cs.shape {
                        if r.fill == INK {
                            let (t, b) = (r.rect.min.y * ppp, r.rect.max.y * ppp);
                            assert!((t - t.round()).abs() < 1e-3 && (b - b.round()).abs() < 1e-3, "edge off grid");
                            bars.push((t.round() as i32, (b - t).round() as i32));
                        }
                    }
                }
                bars.sort();
                assert_eq!(bars.len(), 4, "ppp {ppp} side {side}");
                assert!(bars.iter().all(|b| b.1 == bars[0].1), "heights {bars:?}");
                let gaps: Vec<i32> = bars.windows(2).map(|w| w[1].0 - (w[0].0 + w[0].1)).collect();
                assert!(gaps.iter().all(|g| *g == gaps[0] && *g >= 1), "gaps {gaps:?} for {bars:?}");
            }
        }
    }

    #[test]
    fn every_icon_has_a_colour_version_enabled_and_disabled() {
        for &icon in crate::icons::ALL {
            for grey in [false, true] {
                let cols = colours(icon, grey);
                assert!(!cols.is_empty(), "{icon:?} draws nothing");
                if grey {
                    assert!(cols.iter().all(|c| c.r() == c.g() && c.g() == c.b()), "{icon:?} disabled must be grey");
                }
            }
        }
    }
}
