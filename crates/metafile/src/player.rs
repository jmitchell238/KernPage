//! Device-context state shared by the WMF and EMF interpreters.

use crate::dib::{Dib, DibError};
use crate::*;
use std::collections::BTreeMap;

pub(crate) const MM_TEXT: u32 = 1;
pub(crate) const MM_ISOTROPIC: u32 = 7;
pub(crate) const MM_ANISOTROPIC: u32 = 8;

const MAX_STACK: usize = 4096;
pub(crate) const MAX_OBJECTS: usize = 65536;
/// Hostile-file limits (PW-SEC-003): records interpreted and display-list items kept.
pub(crate) const MAX_RECORDS: usize = 2_000_000;
pub(crate) const MAX_ITEMS: usize = 1_000_000;
const KAPPA: f64 = 0.552_284_749_8;

#[derive(Clone)]
pub(crate) struct Dc {
    pub map_mode: u32,
    pub win_org: (f64, f64),
    pub win_ext: (f64, f64),
    pub win_ext_set: bool,
    pub vp_org: (f64, f64),
    pub vp_ext: (f64, f64),
    /// EMF world transform [m11, m12, m21, m22, dx, dy].
    pub world: [f64; 6],
    pub pen: Option<Stroke>,
    pub brush: Option<Fill>,
    pub font: Font,
    pub text_color: Color,
    pub bk_color: Color,
    pub bk_opaque: bool,
    pub rule: FillRule,
    pub text_align: u32,
    pub pos: (f64, f64),
}

pub(crate) const BLACK: Color = Color { r: 0, g: 0, b: 0 };
pub(crate) const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

pub(crate) fn default_font() -> Font {
    Font {
        face: "System".into(),
        height: 0.0,
        cell_height: false,
        weight: 400,
        italic: false,
        underline: false,
        strikeout: false,
        charset: 0,
        escapement_deg: 0.0,
    }
}

impl Dc {
    fn new() -> Dc {
        Dc {
            map_mode: MM_TEXT,
            win_org: (0.0, 0.0),
            win_ext: (1.0, 1.0),
            win_ext_set: false,
            vp_org: (0.0, 0.0),
            vp_ext: (1.0, 1.0),
            world: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            pen: Some(Stroke {
                color: BLACK,
                width: 0.0,
                style: LineStyle::Solid,
            }),
            brush: Some(Fill {
                color: WHITE,
                approximated: false,
            }),
            font: default_font(),
            text_color: BLACK,
            bk_color: WHITE,
            bk_opaque: true,
            rule: FillRule::EvenOdd,
            text_align: 0,
            pos: (0.0, 0.0),
        }
    }

    /// Window-to-viewport scale for the current map mode.
    fn scale(&self) -> (f64, f64) {
        let nz = |v: f64| if v == 0.0 || !v.is_finite() { 1.0 } else { v };
        match self.map_mode {
            MM_ANISOTROPIC => (
                nz(self.vp_ext.0) / nz(self.win_ext.0),
                nz(self.vp_ext.1) / nz(self.win_ext.1),
            ),
            MM_ISOTROPIC => {
                let sx = nz(self.vp_ext.0) / nz(self.win_ext.0);
                let sy = nz(self.vp_ext.1) / nz(self.win_ext.1);
                let m = sx.abs().min(sy.abs());
                (m.copysign(sx), m.copysign(sy))
            }
            _ => (1.0, 1.0),
        }
    }
}

#[derive(Clone, Copy)]
struct Base {
    win_org: (f64, f64),
    vp_org: (f64, f64),
    scale: (f64, f64),
    window: Option<Rect>,
}

#[derive(Clone)]
pub(crate) enum Obj {
    Pen(Option<Stroke>),
    Brush(Option<Fill>),
    Font(Font),
    Other,
}

pub(crate) struct Player {
    pub dc: Dc,
    stack: Vec<Dc>,
    base: Option<Base>,
    pub items: Vec<Item>,
    pub objects: Vec<Option<Obj>>,
    /// No free slot below this index (keeps WMF object creation O(1) instead of a rescan).
    free_from: usize,
    pub path: Option<Vec<PathSeg>>,
    pub last_path: Vec<PathSeg>,
    pub records: usize,
    pub skipped: usize,
    pub skipped_types: BTreeMap<u32, usize>,
    pub ignored: usize,
    pub approximations: usize,
}

impl Player {
    pub fn new() -> Player {
        Player {
            dc: Dc::new(),
            stack: Vec::new(),
            base: None,
            items: Vec::new(),
            objects: Vec::new(),
            free_from: 0,
            path: None,
            last_path: Vec::new(),
            records: 0,
            skipped: 0,
            skipped_types: BTreeMap::new(),
            ignored: 0,
            approximations: 0,
        }
    }

    pub fn skip(&mut self, ty: u32) {
        self.skipped += 1;
        *self.skipped_types.entry(ty).or_insert(0) += 1;
    }

    fn base(&mut self) -> Base {
        if let Some(b) = self.base {
            return b;
        }
        let dc = &self.dc;
        let window = (dc.win_ext_set && dc.win_ext.0 != 0.0 && dc.win_ext.1 != 0.0).then(|| {
            let (x1, y1) = (dc.win_org.0 + dc.win_ext.0, dc.win_org.1 + dc.win_ext.1);
            Rect {
                left: dc.win_org.0.min(x1),
                top: dc.win_org.1.min(y1),
                right: dc.win_org.0.max(x1),
                bottom: dc.win_org.1.max(y1),
            }
        });
        let b = Base {
            win_org: dc.win_org,
            vp_org: dc.vp_org,
            scale: dc.scale(),
            window,
        };
        self.base = Some(b);
        b
    }

    /// The base window rectangle, if the window extent was set before the first drawing.
    pub fn base_window(&self) -> Option<Rect> {
        self.base.and_then(|b| b.window)
    }

    /// Map a logical point into output (base window) space.
    pub fn map(&mut self, x: f64, y: f64) -> Point {
        let b = self.base();
        let w = &self.dc.world;
        let (x, y) = (x * w[0] + y * w[2] + w[4], x * w[1] + y * w[3] + w[5]);
        let s = self.dc.scale();
        let dx = s.0 * (x - self.dc.win_org.0) + self.dc.vp_org.0;
        let dy = s.1 * (y - self.dc.win_org.1) + self.dc.vp_org.1;
        Point {
            x: b.win_org.0 + (dx - b.vp_org.0) / b.scale.0,
            y: b.win_org.1 + (dy - b.vp_org.1) / b.scale.1,
        }
    }

    /// Factor converting logical lengths into output lengths (x, y).
    pub fn length_scale(&mut self) -> (f64, f64) {
        let b = self.base();
        let s = self.dc.scale();
        let w = &self.dc.world;
        let wx = w[0].hypot(w[1]);
        let wy = w[2].hypot(w[3]);
        ((s.0 / b.scale.0).abs() * wx, (s.1 / b.scale.1).abs() * wy)
    }

    pub fn rect(&mut self, l: f64, t: f64, r: f64, b: f64) -> Rect {
        let p = self.map(l, t);
        let q = self.map(r, b);
        Rect {
            left: p.x.min(q.x),
            top: p.y.min(q.y),
            right: p.x.max(q.x),
            bottom: p.y.max(q.y),
        }
    }

    pub fn stroke(&mut self) -> Option<Stroke> {
        let s = self.dc.pen?;
        let (sx, sy) = self.length_scale();
        Some(Stroke {
            width: s.width * (sx * sy).sqrt(),
            ..s
        })
    }

    pub fn fill(&self) -> Option<Fill> {
        self.dc.brush
    }

    pub fn font(&mut self) -> Font {
        let (_, sy) = self.length_scale();
        let mut f = self.dc.font.clone();
        f.height *= sy;
        f
    }

    pub fn save(&mut self) {
        if self.stack.len() < MAX_STACK {
            self.stack.push(self.dc.clone());
        }
    }

    /// RestoreDC: negative = relative, positive = absolute 1-based index.
    pub fn restore(&mut self, n: i32) -> bool {
        let len = self.stack.len() as i64;
        let target = if n < 0 {
            len + i64::from(n)
        } else {
            i64::from(n) - 1
        };
        if n == 0 || target < 0 || target >= len {
            return false;
        }
        self.stack.truncate(target as usize + 1);
        if let Some(dc) = self.stack.pop() {
            self.dc = dc;
        }
        true
    }

    pub fn push(&mut self, item: Item) {
        if self.items.len() < MAX_ITEMS {
            self.items.push(item);
        }
    }

    // --- object table ---

    /// WMF: store in the lowest free slot.
    pub fn create_lowest(&mut self, obj: Obj) {
        let from = self.free_from.min(self.objects.len());
        if let Some(i) = self.objects[from..].iter().position(|s| s.is_none()) {
            self.objects[from + i] = Some(obj);
            self.free_from = from + i + 1;
        } else if self.objects.len() < MAX_OBJECTS {
            self.objects.push(Some(obj));
            self.free_from = self.objects.len();
        }
    }

    /// EMF: store at a given handle.
    pub fn create_at(&mut self, idx: usize, obj: Obj) -> bool {
        if idx == 0 || idx >= MAX_OBJECTS {
            return false;
        }
        if self.objects.len() <= idx {
            self.objects.resize(idx + 1, None);
        }
        self.objects[idx] = Some(obj);
        true
    }

    pub fn delete(&mut self, idx: usize) {
        if let Some(slot) = self.objects.get_mut(idx) {
            *slot = None;
            self.free_from = self.free_from.min(idx);
        }
    }

    pub fn select(&mut self, obj: Obj) {
        match obj {
            Obj::Pen(p) => self.dc.pen = p,
            Obj::Brush(b) => self.dc.brush = b,
            Obj::Font(f) => self.dc.font = f,
            Obj::Other => {}
        }
    }

    pub fn select_index(&mut self, idx: usize) -> bool {
        match self.objects.get(idx).cloned().flatten() {
            Some(o) => {
                self.select(o);
                true
            }
            None => false,
        }
    }

    // --- shared object construction ---

    pub fn make_pen(&self, style: u32, width: f64, color: Color) -> Obj {
        let st = match style & 0xF {
            1 => LineStyle::Dash,
            2 => LineStyle::Dot,
            3 => LineStyle::DashDot,
            4 => LineStyle::DashDotDot,
            5 => return Obj::Pen(None),
            _ => LineStyle::Solid,
        };
        Obj::Pen(Some(Stroke {
            color,
            width: width.max(0.0),
            style: st,
        }))
    }

    /// Brush styles: 0 solid, 1 null, 2 hatched, anything else a pattern.
    pub fn make_brush(&mut self, style: u32, color: Color) -> Obj {
        match style {
            0 => Obj::Brush(Some(Fill {
                color,
                approximated: false,
            })),
            1 => Obj::Brush(None),
            2 => {
                self.approximations += 1;
                let bk = self.dc.bk_color;
                let mix = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
                Obj::Brush(Some(Fill {
                    color: Color {
                        r: mix(color.r, bk.r),
                        g: mix(color.g, bk.g),
                        b: mix(color.b, bk.b),
                    },
                    approximated: true,
                }))
            }
            _ => {
                self.approximations += 1;
                Obj::Brush(Some(Fill {
                    color: Color {
                        r: 128,
                        g: 128,
                        b: 128,
                    },
                    approximated: true,
                }))
            }
        }
    }

    // --- emitters ---

    pub fn emit_polygon(&mut self, pts: &[(f64, f64)]) {
        if self.in_path() {
            self.path_poly(pts, true);
            return;
        }
        let points = pts.iter().map(|&(x, y)| self.map(x, y)).collect();
        let (fill, stroke, rule) = (self.fill(), self.stroke(), self.dc.rule);
        self.push(Item::Polygon {
            points,
            fill,
            stroke,
            rule,
        });
    }

    pub fn emit_polyline(&mut self, pts: &[(f64, f64)]) {
        if self.in_path() {
            self.path_poly(pts, false);
            return;
        }
        let points = pts.iter().map(|&(x, y)| self.map(x, y)).collect();
        let stroke = self.stroke();
        self.push(Item::Polyline { points, stroke });
    }

    pub fn emit_polypolygon(&mut self, polys: &[Vec<(f64, f64)>]) {
        if self.in_path() {
            for p in polys {
                self.path_poly(p, true);
            }
            return;
        }
        let polygons = polys
            .iter()
            .map(|p| p.iter().map(|&(x, y)| self.map(x, y)).collect())
            .collect();
        let (fill, stroke, rule) = (self.fill(), self.stroke(), self.dc.rule);
        self.push(Item::PolyPolygon {
            polygons,
            fill,
            stroke,
            rule,
        });
    }

    pub fn emit_rect(&mut self, l: f64, t: f64, r: f64, b: f64, corner: Option<(f64, f64)>) {
        if self.in_path() {
            let (l, r) = (l.min(r), l.max(r));
            let (t, b) = (t.min(b), t.max(b));
            self.path_poly(&[(l, t), (r, t), (r, b), (l, b)], true);
            return;
        }
        let rect = self.rect(l, t, r, b);
        let (sx, sy) = self.length_scale();
        let corner = corner.map(|(w, h)| (w.abs() * sx, h.abs() * sy));
        let (fill, stroke) = (self.fill(), self.stroke());
        self.push(Item::Rect {
            rect,
            corner,
            fill,
            stroke,
        });
    }

    pub fn emit_ellipse(&mut self, l: f64, t: f64, r: f64, b: f64) {
        if self.in_path() {
            let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
            let (rx, ry) = ((r - l).abs() / 2.0, (b - t).abs() / 2.0);
            let (kx, ky) = (rx * KAPPA, ry * KAPPA);
            let mut m = |x: f64, y: f64| self.map(x, y);
            let seg = vec![
                PathSeg::MoveTo(m(cx + rx, cy)),
                PathSeg::CubicTo(m(cx + rx, cy + ky), m(cx + kx, cy + ry), m(cx, cy + ry)),
                PathSeg::CubicTo(m(cx - kx, cy + ry), m(cx - rx, cy + ky), m(cx - rx, cy)),
                PathSeg::CubicTo(m(cx - rx, cy - ky), m(cx - kx, cy - ry), m(cx, cy - ry)),
                PathSeg::CubicTo(m(cx + kx, cy - ry), m(cx + rx, cy - ky), m(cx + rx, cy)),
                PathSeg::Close,
            ];
            if let Some(p) = self.path.as_mut() {
                p.extend(seg);
            }
            return;
        }
        let rect = self.rect(l, t, r, b);
        let (fill, stroke) = (self.fill(), self.stroke());
        self.push(Item::Ellipse { rect, fill, stroke });
    }

    pub fn emit_text(&mut self, text: String, x: f64, y: f64) {
        if text.is_empty() {
            return;
        }
        let a = self.dc.text_align;
        let halign = match a & 6 {
            6 => HAlign::Center,
            2 => HAlign::Right,
            _ => HAlign::Left,
        };
        let valign = match a & 24 {
            24 => VAlign::Baseline,
            8 => VAlign::Bottom,
            _ => VAlign::Top,
        };
        let (x, y) = if a & 1 != 0 { self.dc.pos } else { (x, y) };
        let pos = self.map(x, y);
        let font = self.font();
        let background = self.dc.bk_opaque.then_some(self.dc.bk_color);
        let color = self.dc.text_color;
        self.push(Item::Text(Text {
            text,
            pos,
            font,
            color,
            halign,
            valign,
            background,
        }));
    }

    /// Emit a decoded (or failed) bitmap into the logical destination rectangle
    /// `(x, y, w, h)`; negative `w`/`h` mirror the image.
    pub fn emit_bitmap(
        &mut self,
        dib: Result<Dib, DibError>,
        dest: (f64, f64, f64, f64),
        src: Option<(i32, i32, i32, i32)>,
    ) {
        let (x, y, w, h) = dest;
        let p = self.map(x, y);
        let q = self.map(x + w, y + h);
        let rect = Rect {
            left: p.x.min(q.x),
            top: p.y.min(q.y),
            right: p.x.max(q.x),
            bottom: p.y.max(q.y),
        };
        match dib {
            Ok(mut d) => {
                if let Some((sx, sy, sw, sh)) = src {
                    d = d.crop(sx, sy, sw, sh);
                }
                d.flip(q.x < p.x, q.y < p.y);
                self.push(Item::Bitmap {
                    dest: rect,
                    width: d.width,
                    height: d.height,
                    rgba: d.rgba,
                });
            }
            Err(e) => {
                let reason = match e {
                    DibError::Unsupported(s) => s,
                    DibError::Malformed => "malformed DIB".into(),
                };
                self.push(Item::BitmapPlaceholder { dest: rect, reason });
            }
        }
    }

    // --- paths ---

    pub fn in_path(&self) -> bool {
        self.path.is_some()
    }

    fn path_poly(&mut self, pts: &[(f64, f64)], close: bool) {
        let mapped: Vec<Point> = pts.iter().map(|&(x, y)| self.map(x, y)).collect();
        if let (Some(p), Some(first)) = (self.path.as_mut(), mapped.first()) {
            p.push(PathSeg::MoveTo(*first));
            p.extend(mapped[1..].iter().map(|&q| PathSeg::LineTo(q)));
            if close {
                p.push(PathSeg::Close);
            }
        }
    }

    pub fn path_move(&mut self, x: f64, y: f64) {
        let p = self.map(x, y);
        if let Some(path) = self.path.as_mut() {
            path.push(PathSeg::MoveTo(p));
        }
    }

    pub fn path_line(&mut self, x: f64, y: f64) {
        let need_start = self
            .path
            .as_ref()
            .is_some_and(|p| matches!(p.last(), None | Some(PathSeg::Close)));
        if need_start {
            let (px, py) = self.dc.pos;
            self.path_move(px, py);
        }
        let p = self.map(x, y);
        if let Some(path) = self.path.as_mut() {
            path.push(PathSeg::LineTo(p));
        }
    }

    /// Cubic Bezier segments. With `start` the first point is a move-to; otherwise the
    /// curve continues from the current point.
    pub fn path_bezier(&mut self, pts: &[(f64, f64)], start: bool) {
        let mut i = 0;
        if start {
            if let Some(&(x, y)) = pts.first() {
                self.path_move(x, y);
            }
            i = 1;
        } else if self
            .path
            .as_ref()
            .is_some_and(|p| matches!(p.last(), None | Some(PathSeg::Close)))
        {
            let (px, py) = self.dc.pos;
            self.path_move(px, py);
        }
        while i + 3 <= pts.len() {
            let a = self.map(pts[i].0, pts[i].1);
            let b = self.map(pts[i + 1].0, pts[i + 1].1);
            let c = self.map(pts[i + 2].0, pts[i + 2].1);
            if let Some(path) = self.path.as_mut() {
                path.push(PathSeg::CubicTo(a, b, c));
            }
            i += 3;
        }
    }

    pub fn path_close(&mut self) {
        if let Some(path) = self.path.as_mut() {
            path.push(PathSeg::Close);
        }
    }

    pub fn finish_bounds(&self, fallback: Option<Rect>) -> Rect {
        if let Some(r) = fallback {
            return r;
        }
        if let Some(r) = self.base_window() {
            return r;
        }
        let mut b: Option<Rect> = None;
        let mut add = |p: Point| {
            let r = b.get_or_insert(Rect {
                left: p.x,
                top: p.y,
                right: p.x,
                bottom: p.y,
            });
            r.left = r.left.min(p.x);
            r.right = r.right.max(p.x);
            r.top = r.top.min(p.y);
            r.bottom = r.bottom.max(p.y);
        };
        let add_rect = |r: &Rect, add: &mut dyn FnMut(Point)| {
            add(Point {
                x: r.left,
                y: r.top,
            });
            add(Point {
                x: r.right,
                y: r.bottom,
            });
        };
        for it in &self.items {
            match it {
                Item::Polygon { points, .. } | Item::Polyline { points, .. } => {
                    points.iter().for_each(|&p| add(p))
                }
                Item::PolyPolygon { polygons, .. } => {
                    polygons.iter().flatten().for_each(|&p| add(p))
                }
                Item::Rect { rect, .. }
                | Item::Ellipse { rect, .. }
                | Item::Bitmap { dest: rect, .. }
                | Item::BitmapPlaceholder { dest: rect, .. } => add_rect(rect, &mut add),
                Item::Path { segments, .. } => {
                    for s in segments {
                        match *s {
                            PathSeg::MoveTo(p) | PathSeg::LineTo(p) => add(p),
                            PathSeg::CubicTo(a, c, d) => {
                                add(a);
                                add(c);
                                add(d);
                            }
                            PathSeg::Close => {}
                        }
                    }
                }
                Item::Text(t) => add(t.pos),
            }
        }
        b.unwrap_or(Rect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        })
    }
}

/// Windows-1252 decode (bytes 0x80..0x9F mapped, the rest Latin-1).
pub(crate) fn ansi(bytes: &[u8]) -> String {
    const HI: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}',
        '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}',
        '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}',
        '\u{178}',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HI[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

pub(crate) fn colorref(b: &[u8]) -> Color {
    Color {
        r: b.first().copied().unwrap_or(0),
        g: b.get(1).copied().unwrap_or(0),
        b: b.get(2).copied().unwrap_or(0),
    }
}
