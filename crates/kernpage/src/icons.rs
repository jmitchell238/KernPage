//! Simple vector icons drawn in code (our own artwork; no icon fonts or images).

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Save,
    Undo,
    Redo,
    Paste,
    Cut,
    Copy,
    Brush,
    GrowFont,
    ShrinkFont,
    ClearFormat,
    FontColor,
    /// The Font Color "A" without its colour bar (the ribbon paints the bar itself).
    FontColorLetter,
    Bullets,
    Numbering,
    IndentLess,
    IndentMore,
    Columns,
    Pilcrow,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    LineSpacing,
    Shading,
    Styles,
    TextBox,
    Picture,
    Table,
    Shapes,
    Wrap,
    Forward,
    Backward,
    AlignObjects,
    Group,
    Rotate,
    Ungroup,
    Find,
    Replace,
    Select,
    Page,
    Pages,
    Blocks,
    Calendar,
    Frame,
    Link,
    Bookmark,
    Header,
    Footer,
    Hash,
    Gear,
    Margins,
    Orientation,
    Size,
    Guides,
    Trash,
    TableAbove,
    TableBelow,
    TableLeft,
    TableRight,
    TableDelete,
    Move,
    Rename,
    Palette,
    Background,
    Mail,
    People,
    Check,
    Globe,
    Clock,
    Arrow,
    Question,
    Folder,
    New,
    Close,
    #[allow(dead_code)]
    Exit,
    #[allow(dead_code)]
    Info,
    Print,
    Send,
    Window,
    Zoom,
    FitPage,
    PageWidth,
    SinglePage,
    TwoPage,
    Chevron,
    ChevronDown,
    Back,
    Star,
    Bold,
    Italic,
    Underline,
    Sub,
    Sup,
    Case,
    Spacing,
    Fonts,
    Abc,
    Pointer,
    ObjectSize,
    ZoomFit,
    Field,
    Sequence,
    RepeatArea,
    PerSheet,
    Eye,
    CreatePage,
    Warning,
    Lightbulb,
}

/// Every icon variant.
pub(crate) const ALL: &[Icon] = &[Icon::Save, Icon::Undo, Icon::Redo, Icon::Paste, Icon::Cut, Icon::Copy, Icon::Brush, Icon::GrowFont, Icon::ShrinkFont, Icon::ClearFormat, Icon::FontColor, Icon::FontColorLetter, Icon::Bullets, Icon::Numbering, Icon::IndentLess, Icon::IndentMore, Icon::Columns, Icon::Pilcrow, Icon::AlignLeft, Icon::AlignCenter, Icon::AlignRight, Icon::AlignJustify, Icon::LineSpacing, Icon::Shading, Icon::Styles, Icon::TextBox, Icon::Picture, Icon::Table, Icon::Shapes, Icon::Wrap, Icon::Forward, Icon::Backward, Icon::AlignObjects, Icon::Group, Icon::Rotate, Icon::Ungroup, Icon::Find, Icon::Replace, Icon::Select, Icon::Page, Icon::Pages, Icon::Blocks, Icon::Calendar, Icon::Frame, Icon::Link, Icon::Bookmark, Icon::Header, Icon::Footer, Icon::Hash, Icon::Gear, Icon::Margins, Icon::Orientation, Icon::Size, Icon::Guides, Icon::Trash, Icon::TableAbove, Icon::TableBelow, Icon::TableLeft, Icon::TableRight, Icon::TableDelete, Icon::Move, Icon::Rename, Icon::Palette, Icon::Background, Icon::Mail, Icon::People, Icon::Check, Icon::Globe, Icon::Clock, Icon::Arrow, Icon::Question, Icon::Folder, Icon::New, Icon::Close, Icon::Exit, Icon::Info, Icon::Print, Icon::Send, Icon::Window, Icon::Zoom, Icon::FitPage, Icon::PageWidth, Icon::SinglePage, Icon::TwoPage, Icon::Chevron, Icon::ChevronDown, Icon::Back, Icon::Star, Icon::Bold, Icon::Italic, Icon::Underline, Icon::Sub, Icon::Sup, Icon::Case, Icon::Spacing, Icon::Fonts, Icon::Abc, Icon::Pointer, Icon::ObjectSize, Icon::ZoomFit, Icon::Field, Icon::Sequence, Icon::RepeatArea, Icon::PerSheet, Icon::Eye, Icon::CreatePage, Icon::Warning, Icon::Lightbulb];

struct Canvas<'a> {
    p: &'a Painter,
    r: Rect,
    col: Color32,
    w: f32,
}

impl Canvas<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(
            self.r.min.x + x * self.r.width(),
            self.r.min.y + y * self.r.height(),
        )
    }
    fn st(&self) -> Stroke {
        Stroke::new(self.w, self.col)
    }
    fn line(&self, a: (f32, f32), b: (f32, f32)) {
        self.p
            .line_segment([self.pt(a.0, a.1), self.pt(b.0, b.1)], self.st());
    }
    fn poly(&self, pts: &[(f32, f32)], closed: bool) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        if closed {
            self.p.add(Shape::closed_line(v, self.st()));
        } else {
            self.p.add(Shape::line(v, self.st()));
        }
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        self.p.rect_stroke(r, 1.0, self.st(), StrokeKind::Middle);
    }
    fn fill_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        self.p.rect_filled(r, 1.0, self.col);
    }
    fn fill_tint(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        self.p.rect_filled(r, 0.0, self.col.gamma_multiply(0.3));
    }
    fn fill_poly(&self, pts: &[(f32, f32)], alpha: f32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        self.p
            .add(Shape::convex_polygon(v, self.col.gamma_multiply(alpha), Stroke::NONE));
    }
    fn fill_bg(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        self.p.rect_filled(r, 0.0, crate::theme::card());
    }
    /// A dotted selection frame with small square handles at the corners.
    fn selection_frame(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let thin = Stroke::new((self.w * 0.6).max(1.0), self.col.gamma_multiply(0.75));
        let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
        for w in pts.windows(2) {
            let (a, b) = (self.pt(w[0].0, w[0].1), self.pt(w[1].0, w[1].1));
            self.p.add(Shape::dashed_line(&[a, b], thin, 1.5, 1.5));
        }
        let h = (self.r.width() * 0.07).max(2.0);
        for &(x, y) in &pts[..4] {
            let c = self.pt(x, y);
            self.p.rect_filled(Rect::from_center_size(c, Vec2::splat(h)), 0.0, self.col);
        }
    }
    fn circle(&self, x: f32, y: f32, rad: f32) {
        self.p
            .circle_stroke(self.pt(x, y), rad * self.r.width(), self.st());
    }
    fn dot(&self, x: f32, y: f32, rad: f32) {
        self.p
            .circle_filled(self.pt(x, y), rad * self.r.width(), self.col);
    }
    fn lines(&self, ys: &[f32], x0: f32, x1: f32) {
        for &y in ys {
            self.line((x0, y), (x1, y));
        }
    }
}

/// The `kernicons` file name of an icon: its variant name in kebab-case (`AlignLeft` -> `align-left`).
pub fn name(icon: Icon) -> &'static str {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static NAMES: OnceLock<HashMap<Icon, &'static str>> = OnceLock::new();
    NAMES.get_or_init(|| ALL.iter().map(|&i| (i, kebab(&format!("{i:?}")).leak() as &'static str)).collect())[&icon]
}

fn kebab(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Paint `icon` filling `r` (roughly square).
pub fn paint(p: &Painter, r: Rect, icon: Icon, col: Color32) {
    // The ribbon's accent colour draws the coloured icons, its disabled grey the flat grey ones,
    // any other colour a one-colour tint. The outline drawing below is only the fallback for
    // names kernicons does not have.
    let mode = if col == crate::widgets::ACCENT_COLOR {
        // In the dark theme the outline ink is light, so icons read on dark surfaces.
        if crate::theme::is_dark() {
            kernicons::Mode::ColourInk(crate::theme::ink())
        } else {
            kernicons::Mode::Colour
        }
    } else if col == crate::theme::disabled() {
        // The flat grey keeps light fills light, which glares on a dark surface.
        if crate::theme::is_dark() {
            kernicons::Mode::Mono(col)
        } else {
            kernicons::Mode::Disabled
        }
    } else {
        kernicons::Mode::Mono(col)
    };
    if kernicons::paint(p, r, name(icon), mode) {
        return;
    }
    let side = r.width().min(r.height());
    let r = Rect::from_center_size(r.center(), Vec2::splat(side));
    let c = Canvas {
        p,
        r,
        col,
        w: (side / 14.0).clamp(1.0, 2.2),
    };
    use Icon::*;
    match icon {
        Save => {
            c.poly(
                &[
                    (0.12, 0.12),
                    (0.78, 0.12),
                    (0.88, 0.22),
                    (0.88, 0.88),
                    (0.12, 0.88),
                ],
                true,
            );
            c.rect(0.28, 0.12, 0.7, 0.4);
            c.rect(0.25, 0.55, 0.75, 0.88);
        }
        Undo | Redo => {
            let s = if icon == Undo { 1.0 } else { -1.0 };
            let x = |v: f32| if s > 0.0 { v } else { 1.0 - v };
            c.poly(&[(x(0.15), 0.4), (x(0.38), 0.2), (x(0.38), 0.6)], true);
            c.poly(
                &[
                    (x(0.38), 0.4),
                    (x(0.65), 0.4),
                    (x(0.88), 0.6),
                    (x(0.8), 0.85),
                ],
                false,
            );
        }
        Paste => {
            c.rect(0.15, 0.18, 0.75, 0.9);
            c.fill_rect(0.33, 0.1, 0.57, 0.28);
            c.rect(0.5, 0.45, 0.92, 0.95);
            c.lines(&[0.6, 0.72, 0.84], 0.58, 0.84);
        }
        Cut => {
            c.line((0.3, 0.1), (0.7, 0.65));
            c.line((0.7, 0.1), (0.3, 0.65));
            c.circle(0.28, 0.8, 0.12);
            c.circle(0.72, 0.8, 0.12);
        }
Copy => {
            c.rect(0.12, 0.1, 0.58, 0.7);
            c.fill_bg(0.4, 0.3, 0.88, 0.92);
            c.rect(0.4, 0.3, 0.88, 0.92);
            c.lines(&[0.5, 0.62, 0.74], 0.5, 0.78);
        }
Brush => {
            // A diagonal paintbrush: handle, ferrule, tinted bristles at the lower left.
            c.fill_poly(&[(0.418, 0.498), (0.502, 0.582), (0.28, 0.88), (0.1, 0.92), (0.14, 0.72)], 0.3);
            c.poly(&[(0.418, 0.498), (0.502, 0.582), (0.28, 0.88), (0.1, 0.92), (0.14, 0.72)], true);
            c.poly(&[(0.578, 0.338), (0.662, 0.422), (0.502, 0.582), (0.418, 0.498)], true);
            c.line((0.92, 0.08), (0.64, 0.36));
        }
        GrowFont | ShrinkFont => {
            c.poly(&[(0.1, 0.9), (0.38, 0.15), (0.66, 0.9)], false);
            c.line((0.2, 0.65), (0.56, 0.65));
            if icon == GrowFont {
                c.poly(&[(0.74, 0.35), (0.84, 0.2), (0.94, 0.35)], false);
            } else {
                c.poly(&[(0.74, 0.2), (0.84, 0.35), (0.94, 0.2)], false);
            }
        }
ClearFormat => {
            // An A with an eraser at the lower right.
            c.poly(&[(0.04, 0.88), (0.3, 0.12), (0.56, 0.88)], false);
            c.line((0.13, 0.64), (0.47, 0.64));
            c.fill_poly(&[(0.6, 0.78), (0.76, 0.46), (0.94, 0.56), (0.78, 0.88)], 0.3);
            c.poly(&[(0.6, 0.78), (0.76, 0.46), (0.94, 0.56), (0.78, 0.88)], true);
            c.line((0.68, 0.62), (0.86, 0.72));
        }
FontColor | FontColorLetter => {
            c.poly(&[(0.2, 0.68), (0.5, 0.06), (0.8, 0.68)], false);
            c.line((0.32, 0.46), (0.68, 0.46));
            c.fill_rect(0.12, 0.8, 0.88, 0.94);
        }
        Bullets => {
            for y in [0.2, 0.5, 0.8] {
                c.dot(0.15, y, 0.06);
                c.line((0.32, y), (0.9, y));
            }
        }
Numbering => {
            for (i, y) in [0.2, 0.5, 0.8].into_iter().enumerate() {
                p.text(
                    c.pt(0.15, y),
                    eframe::egui::Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    eframe::egui::FontId::proportional(side * 0.32),
                    col,
                );
                c.line((0.38, y), (0.92, y));
            }
        }
        IndentLess | IndentMore => {
            c.lines(&[0.15, 0.85], 0.1, 0.9);
            c.lines(&[0.4, 0.6], 0.45, 0.9);
            if icon == IndentMore {
                c.poly(&[(0.1, 0.38), (0.28, 0.5), (0.1, 0.62)], true);
            } else {
                c.poly(&[(0.28, 0.38), (0.1, 0.5), (0.28, 0.62)], true);
            }
        }
Columns => {
            c.rect(0.1, 0.12, 0.9, 0.88);
            c.lines(&[0.28, 0.4, 0.52, 0.64, 0.76], 0.2, 0.44);
            c.lines(&[0.28, 0.4, 0.52, 0.64, 0.76], 0.56, 0.8);
        }
Pilcrow => {
            c.fill_poly(&[(0.52, 0.12), (0.32, 0.12), (0.2, 0.3), (0.32, 0.5), (0.52, 0.5)], 1.0);
            c.line((0.52, 0.12), (0.52, 0.88));
            c.line((0.7, 0.12), (0.7, 0.88));
            c.line((0.52, 0.12), (0.85, 0.12));
        }
        AlignLeft => {
            c.line((0.1, 0.15), (0.9, 0.15));
            c.line((0.1, 0.38), (0.6, 0.38));
            c.line((0.1, 0.62), (0.9, 0.62));
            c.line((0.1, 0.85), (0.6, 0.85));
        }
        AlignCenter => {
            c.line((0.1, 0.15), (0.9, 0.15));
            c.line((0.25, 0.38), (0.75, 0.38));
            c.line((0.1, 0.62), (0.9, 0.62));
            c.line((0.25, 0.85), (0.75, 0.85));
        }
        AlignRight => {
            c.line((0.1, 0.15), (0.9, 0.15));
            c.line((0.4, 0.38), (0.9, 0.38));
            c.line((0.1, 0.62), (0.9, 0.62));
            c.line((0.4, 0.85), (0.9, 0.85));
        }
        AlignJustify => c.lines(&[0.15, 0.38, 0.62, 0.85], 0.1, 0.9),
        LineSpacing => {
            c.lines(&[0.2, 0.5, 0.8], 0.4, 0.9);
            c.poly(&[(0.1, 0.35), (0.2, 0.2), (0.3, 0.35)], false);
            c.poly(&[(0.1, 0.65), (0.2, 0.8), (0.3, 0.65)], false);
            c.line((0.2, 0.2), (0.2, 0.8));
        }
        Shading => {
            c.rect(0.12, 0.12, 0.88, 0.88);
            c.fill_rect(0.12, 0.5, 0.88, 0.88);
        }
Styles => {
            // A large A crossed by a paintbrush.
            c.poly(&[(0.06, 0.92), (0.34, 0.08), (0.62, 0.92)], false);
            c.line((0.15, 0.7), (0.53, 0.7));
            c.fill_poly(&[(0.68, 0.62), (0.78, 0.72), (0.62, 0.92), (0.5, 0.92), (0.56, 0.76)], 0.3);
            c.poly(&[(0.68, 0.62), (0.78, 0.72), (0.62, 0.92), (0.5, 0.92), (0.56, 0.76)], true);
            c.line((0.95, 0.3), (0.72, 0.58));
        }
TextBox => {
            // A frame with corner handles and an A inside.
            c.rect(0.1, 0.18, 0.9, 0.82);
            for (x, y) in [(0.1, 0.18), (0.9, 0.18), (0.1, 0.82), (0.9, 0.82)] {
                c.fill_rect(x - 0.045, y - 0.045, x + 0.045, y + 0.045);
            }
            c.poly(&[(0.34, 0.72), (0.5, 0.28), (0.66, 0.72)], false);
            c.line((0.4, 0.6), (0.6, 0.6));
        }
        Picture => {
            c.rect(0.08, 0.15, 0.92, 0.85);
            c.poly(
                &[
                    (0.12, 0.8),
                    (0.4, 0.45),
                    (0.58, 0.65),
                    (0.7, 0.52),
                    (0.88, 0.8),
                ],
                false,
            );
            c.dot(0.7, 0.32, 0.06);
        }
Table => {
            c.rect(0.1, 0.15, 0.9, 0.85);
            c.fill_rect(0.1, 0.15, 0.9, 0.28);
            c.line((0.1, 0.43), (0.9, 0.43));
            c.line((0.1, 0.65), (0.9, 0.65));
            c.line((0.37, 0.15), (0.37, 0.85));
            c.line((0.63, 0.15), (0.63, 0.85));
        }
        Shapes => {
            // A tinted square behind a white circle (Publisher's layout, our drawing).
            c.fill_tint(0.1, 0.1, 0.6, 0.6);
            c.rect(0.1, 0.1, 0.6, 0.6);
            let ctr = c.pt(0.62, 0.62);
            let rad = 0.28 * c.r.width();
            p.circle_filled(ctr, rad + c.w * 1.5, crate::theme::card());
            c.circle(0.62, 0.62, 0.28);
        }
Wrap => {
            // Text lines above and below an arch (the wrapped-around object).
            c.lines(&[0.12, 0.27, 0.78, 0.92], 0.08, 0.92);
            let arch: Vec<(f32, f32)> = (0..=12)
                .map(|i| {
                    let t = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
                    (0.5 + 0.26 * t.cos(), 0.66 + 0.26 * t.sin())
                })
                .collect();
            c.fill_poly(&arch, 0.3);
            c.poly(&arch, true);
        }
        Forward | Backward => {
            let (back, front) = if icon == Forward {
                ((0.1, 0.1, 0.6, 0.6), (0.4, 0.4, 0.9, 0.9))
            } else {
                ((0.4, 0.4, 0.9, 0.9), (0.1, 0.1, 0.6, 0.6))
            };
            c.rect(back.0, back.1, back.2, back.3);
            let r = Rect::from_min_max(c.pt(front.0, front.1), c.pt(front.2, front.3));
            p.rect_filled(r, 1.0, col.gamma_multiply(0.35));
            c.rect(front.0, front.1, front.2, front.3);
        }
        AlignObjects => {
            c.line((0.12, 0.1), (0.12, 0.9));
            c.fill_rect(0.2, 0.2, 0.8, 0.4);
            c.fill_rect(0.2, 0.6, 0.55, 0.8);
        }
        Group => {
            // Two overlapping squares inside one dotted selection frame with handles.
            c.selection_frame(0.06, 0.06, 0.94, 0.94);
            c.rect(0.22, 0.22, 0.6, 0.6);
            c.fill_bg(0.4, 0.4, 0.78, 0.78);
            c.fill_tint(0.4, 0.4, 0.78, 0.78);
            c.rect(0.4, 0.4, 0.78, 0.78);
        }
        Ungroup => {
            // Two separate squares, each with its own dotted frame and handles.
            c.selection_frame(0.04, 0.04, 0.56, 0.56);
            c.rect(0.14, 0.14, 0.46, 0.46);
            c.selection_frame(0.44, 0.44, 0.96, 0.96);
            c.fill_bg(0.54, 0.54, 0.86, 0.86);
            c.fill_tint(0.54, 0.54, 0.86, 0.86);
            c.rect(0.54, 0.54, 0.86, 0.86);
        }
        Rotate => {
            // A curved arrow over a wedge.
            let wedge = [(0.08, 0.9), (0.92, 0.9), (0.92, 0.52)];
            let v: Vec<Pos2> = wedge.iter().map(|&(x, y)| c.pt(x, y)).collect();
            p.add(Shape::convex_polygon(v, c.col.gamma_multiply(0.3), Stroke::NONE));
            c.poly(&wedge, true);
            let arc: Vec<(f32, f32)> = (0..=12)
                .map(|i| {
                    let t = std::f32::consts::PI * (1.0 - i as f32 / 12.0 * 0.62);
                    (0.5 + 0.34 * t.cos(), 0.5 - 0.34 * t.sin())
                })
                .collect();
            c.poly(&arc, false);
            let (ex, ey) = *arc.last().unwrap();
            c.poly(&[(ex - 0.16, ey - 0.06), (ex, ey), (ex - 0.06, ey + 0.15)], false);
        }
Find => {
            c.circle(0.6, 0.4, 0.26);
            c.line((0.42, 0.58), (0.1, 0.9));
        }
        Replace => {
            c.line((0.1, 0.3), (0.8, 0.3));
            c.poly(&[(0.68, 0.15), (0.85, 0.3), (0.68, 0.45)], false);
            c.line((0.9, 0.7), (0.2, 0.7));
            c.poly(&[(0.32, 0.55), (0.15, 0.7), (0.32, 0.85)], false);
        }
        Select => {
            c.poly(
                &[
                    (0.2, 0.1),
                    (0.2, 0.8),
                    (0.4, 0.62),
                    (0.55, 0.92),
                    (0.66, 0.86),
                    (0.52, 0.58),
                    (0.78, 0.55),
                ],
                true,
            );
        }
        Page => {
            c.poly(
                &[
                    (0.2, 0.08),
                    (0.62, 0.08),
                    (0.82, 0.28),
                    (0.82, 0.92),
                    (0.2, 0.92),
                ],
                true,
            );
            c.poly(&[(0.62, 0.08), (0.62, 0.28), (0.82, 0.28)], false);
        }
        Pages => {
            c.poly(
                &[
                    (0.3, 0.08),
                    (0.62, 0.08),
                    (0.78, 0.24),
                    (0.78, 0.8),
                    (0.3, 0.8),
                ],
                true,
            );
            c.line((0.18, 0.2), (0.18, 0.92));
            c.line((0.18, 0.92), (0.64, 0.92));
        }
Blocks => {
            // A page with a tinted block above a plain one.
            c.rect(0.2, 0.08, 0.8, 0.92);
            c.fill_poly(&[(0.3, 0.2), (0.7, 0.2), (0.7, 0.42), (0.3, 0.42)], 0.3);
            c.rect(0.3, 0.2, 0.7, 0.42);
            c.rect(0.3, 0.56, 0.7, 0.78);
        }
        Calendar => {
            c.rect(0.1, 0.2, 0.9, 0.9);
            c.line((0.1, 0.4), (0.9, 0.4));
            c.line((0.3, 0.1), (0.3, 0.28));
            c.line((0.7, 0.1), (0.7, 0.28));
            c.dot(0.35, 0.58, 0.04);
            c.dot(0.55, 0.58, 0.04);
            c.dot(0.35, 0.75, 0.04);
        }
        Frame => {
            c.rect(0.1, 0.1, 0.9, 0.9);
            c.rect(0.25, 0.25, 0.75, 0.75);
        }
        Link => {
            c.rect(0.08, 0.35, 0.55, 0.65);
            c.rect(0.45, 0.35, 0.92, 0.65);
        }
Bookmark => {
            // A page with a ribbon marker at the lower right.
            c.poly(&[(0.15, 0.06), (0.55, 0.06), (0.75, 0.26), (0.75, 0.92), (0.15, 0.92)], true);
            c.poly(&[(0.55, 0.06), (0.55, 0.26), (0.75, 0.26)], false);
            c.fill_bg(0.58, 0.5, 0.84, 0.97);
            c.fill_poly(&[(0.58, 0.5), (0.84, 0.5), (0.84, 0.96), (0.71, 0.84), (0.58, 0.96)], 0.3);
            c.poly(&[(0.58, 0.5), (0.84, 0.5), (0.84, 0.96), (0.71, 0.84), (0.58, 0.96)], true);
        }
Header => {
            c.poly(&[(0.2, 0.08), (0.62, 0.08), (0.82, 0.28), (0.82, 0.92), (0.2, 0.92)], true);
            c.poly(&[(0.62, 0.08), (0.62, 0.28), (0.82, 0.28)], false);
            c.fill_rect(0.3, 0.2, 0.7, 0.3);
        }
Footer => {
            c.poly(&[(0.2, 0.08), (0.62, 0.08), (0.82, 0.28), (0.82, 0.92), (0.2, 0.92)], true);
            c.poly(&[(0.62, 0.08), (0.62, 0.28), (0.82, 0.28)], false);
            c.fill_rect(0.3, 0.72, 0.7, 0.82);
        }
Hash => {
            // A page with a # inside (Page Number).
            c.poly(&[(0.2, 0.08), (0.62, 0.08), (0.82, 0.28), (0.82, 0.92), (0.2, 0.92)], true);
            c.poly(&[(0.62, 0.08), (0.62, 0.28), (0.82, 0.28)], false);
            c.line((0.47, 0.44), (0.43, 0.78));
            c.line((0.6, 0.44), (0.56, 0.78));
            c.line((0.36, 0.54), (0.68, 0.54));
            c.line((0.35, 0.68), (0.67, 0.68));
        }
        Gear => {
            c.circle(0.5, 0.5, 0.2);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                c.line(
                    (0.5 + 0.28 * a.cos(), 0.5 + 0.28 * a.sin()),
                    (0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin()),
                );
            }
        }
        Margins => {
            c.rect(0.15, 0.1, 0.85, 0.9);
            c.rect(0.28, 0.24, 0.72, 0.76);
        }
        Orientation => {
            c.rect(0.3, 0.1, 0.7, 0.55);
            c.rect(0.1, 0.62, 0.55, 0.9);
        }
        Size => {
            c.rect(0.2, 0.2, 0.8, 0.8);
            c.line((0.2, 0.08), (0.8, 0.08));
            c.line((0.92, 0.2), (0.92, 0.8));
        }
        Guides => {
            c.line((0.1, 0.35), (0.9, 0.35));
            c.line((0.35, 0.1), (0.35, 0.9));
        }
        TableAbove | TableBelow | TableLeft | TableRight | TableDelete => {
            // A 3 x 3 table. Insert icons tint the new track and put a plus in it; Delete
            // tints the right-hand column and crosses it out.
            let (x0, y0, x1, y1) = match icon {
                TableAbove => (0.1, 0.1, 0.9, 0.37),
                TableBelow => (0.1, 0.63, 0.9, 0.9),
                TableLeft => (0.1, 0.1, 0.37, 0.9),
                _ => (0.63, 0.1, 0.9, 0.9),
            };
            c.fill_tint(x0, y0, x1, y1);
            c.rect(0.1, 0.1, 0.9, 0.9);
            c.lines(&[0.37, 0.63], 0.1, 0.9);
            for x in [0.37, 0.63] {
                c.line((x, 0.1), (x, 0.9));
            }
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            if icon == TableDelete {
                c.line((cx - 0.1, cy - 0.1), (cx + 0.1, cy + 0.1));
                c.line((cx - 0.1, cy + 0.1), (cx + 0.1, cy - 0.1));
            } else {
                c.line((cx - 0.09, cy), (cx + 0.09, cy));
                c.line((cx, cy - 0.09), (cx, cy + 0.09));
            }
        }
        Trash => {
            c.line((0.15, 0.25), (0.85, 0.25));
            c.poly(&[(0.25, 0.25), (0.3, 0.9), (0.7, 0.9), (0.75, 0.25)], false);
            c.line((0.4, 0.1), (0.6, 0.1));
        }
        Move => {
            c.line((0.5, 0.1), (0.5, 0.9));
            c.line((0.1, 0.5), (0.9, 0.5));
        }
        Rename => {
            c.rect(0.08, 0.3, 0.92, 0.7);
            c.line((0.5, 0.38), (0.5, 0.62));
        }
        Palette => {
            c.fill_rect(0.1, 0.1, 0.45, 0.45);
            c.rect(0.55, 0.1, 0.9, 0.45);
            c.rect(0.1, 0.55, 0.45, 0.9);
            c.fill_rect(0.55, 0.55, 0.9, 0.9);
        }
        Background => {
            c.rect(0.1, 0.1, 0.9, 0.9);
            c.poly(&[(0.1, 0.9), (0.9, 0.1)], false);
            c.poly(&[(0.1, 0.55), (0.55, 0.1)], false);
        }
        Mail => {
            c.rect(0.08, 0.22, 0.92, 0.78);
            c.poly(&[(0.08, 0.22), (0.5, 0.55), (0.92, 0.22)], false);
        }
        People => {
            c.circle(0.5, 0.3, 0.14);
            c.poly(&[(0.2, 0.9), (0.2, 0.65), (0.8, 0.65), (0.8, 0.9)], false);
        }
        Check => c.poly(&[(0.12, 0.55), (0.4, 0.82), (0.9, 0.2)], false),
        Globe => {
            c.circle(0.5, 0.5, 0.4);
            c.line((0.1, 0.5), (0.9, 0.5));
            c.line((0.5, 0.1), (0.5, 0.9));
        }
        Clock => {
            c.circle(0.5, 0.5, 0.4);
            c.line((0.5, 0.5), (0.5, 0.25));
            c.line((0.5, 0.5), (0.7, 0.58));
        }
        Arrow => {
            c.line((0.1, 0.5), (0.85, 0.5));
            c.poly(&[(0.6, 0.25), (0.88, 0.5), (0.6, 0.75)], false);
        }
        Question => {
            c.poly(
                &[
                    (0.3, 0.3),
                    (0.35, 0.15),
                    (0.65, 0.15),
                    (0.7, 0.32),
                    (0.5, 0.52),
                    (0.5, 0.65),
                ],
                false,
            );
            c.dot(0.5, 0.85, 0.05);
        }
        Folder => {
            c.poly(
                &[
                    (0.08, 0.2),
                    (0.38, 0.2),
                    (0.48, 0.32),
                    (0.92, 0.32),
                    (0.92, 0.85),
                    (0.08, 0.85),
                ],
                true,
            );
        }
        New => {
            c.poly(
                &[
                    (0.2, 0.08),
                    (0.62, 0.08),
                    (0.82, 0.28),
                    (0.82, 0.92),
                    (0.2, 0.92),
                ],
                true,
            );
            c.line((0.5, 0.42), (0.5, 0.74));
            c.line((0.34, 0.58), (0.66, 0.58));
        }
        Close => {
            c.rect(0.12, 0.12, 0.88, 0.88);
            c.line((0.32, 0.32), (0.68, 0.68));
            c.line((0.68, 0.32), (0.32, 0.68));
        }
        Exit => {
            c.poly(
                &[(0.5, 0.12), (0.12, 0.12), (0.12, 0.88), (0.5, 0.88)],
                false,
            );
            c.line((0.35, 0.5), (0.9, 0.5));
            c.poly(&[(0.72, 0.3), (0.92, 0.5), (0.72, 0.7)], false);
        }
        Info => {
            c.circle(0.5, 0.5, 0.4);
            c.line((0.5, 0.45), (0.5, 0.72));
            c.dot(0.5, 0.3, 0.04);
        }
        Print => {
            c.rect(0.25, 0.1, 0.75, 0.4);
            c.rect(0.1, 0.4, 0.9, 0.75);
            c.rect(0.25, 0.6, 0.75, 0.92);
        }
        Send => {
            c.poly(&[(0.1, 0.15), (0.9, 0.5), (0.1, 0.85), (0.25, 0.5)], true);
        }
        Window => {
            c.rect(0.1, 0.15, 0.9, 0.85);
            c.line((0.1, 0.3), (0.9, 0.3));
        }
        Zoom => {
            c.circle(0.42, 0.42, 0.28);
            c.line((0.64, 0.64), (0.9, 0.9));
            c.line((0.3, 0.42), (0.54, 0.42));
        }
        FitPage => {
            c.rect(0.3, 0.12, 0.7, 0.88);
            c.poly(&[(0.08, 0.3), (0.08, 0.08), (0.3, 0.08)], false);
            c.poly(&[(0.92, 0.7), (0.92, 0.92), (0.7, 0.92)], false);
        }
        PageWidth => {
            c.rect(0.25, 0.12, 0.75, 0.88);
            c.line((0.05, 0.5), (0.95, 0.5));
            c.poly(&[(0.2, 0.4), (0.05, 0.5), (0.2, 0.6)], false);
            c.poly(&[(0.8, 0.4), (0.95, 0.5), (0.8, 0.6)], false);
        }
        SinglePage => c.rect(0.3, 0.15, 0.7, 0.85),
        TwoPage => {
            c.rect(0.1, 0.2, 0.48, 0.8);
            c.rect(0.52, 0.2, 0.9, 0.8);
        }
        Pointer => c.poly(&[(0.28, 0.15), (0.28, 0.78), (0.44, 0.64), (0.55, 0.88), (0.66, 0.83), (0.54, 0.6), (0.75, 0.58)], true),
        ObjectSize => {
            c.rect(0.15, 0.22, 0.85, 0.78);
            c.line((0.34, 0.65), (0.66, 0.35));
        }
        ZoomFit => {
            c.rect(0.34, 0.34, 0.66, 0.66);
            c.poly(&[(0.2, 0.34), (0.1, 0.5), (0.2, 0.66)], false);
            c.poly(&[(0.8, 0.34), (0.9, 0.5), (0.8, 0.66)], false);
            c.poly(&[(0.34, 0.2), (0.5, 0.1), (0.66, 0.2)], false);
            c.poly(&[(0.34, 0.8), (0.5, 0.9), (0.66, 0.8)], false);
        }
        // Fallback drawings for the Data tab icons (the SVGs are the real artwork).
        Field | Sequence | RepeatArea | PerSheet | Eye | CreatePage | Warning | Lightbulb => c.rect(0.2, 0.2, 0.8, 0.8),
        Chevron => c.poly(&[(0.2, 0.6), (0.5, 0.3), (0.8, 0.6)], false),
        ChevronDown => c.poly(&[(0.2, 0.4), (0.5, 0.7), (0.8, 0.4)], false),
        Back => {
            c.line((0.85, 0.5), (0.15, 0.5));
            c.poly(&[(0.4, 0.22), (0.12, 0.5), (0.4, 0.78)], false);
        }
        // Character spacing: an "A" over a drawn left-right arrow (U+2194 is not in egui's fonts).
        Spacing => {
            let font = eframe::egui::FontId::proportional(side * 0.62);
            p.text(c.pt(0.5, 0.36), eframe::egui::Align2::CENTER_CENTER, "A", font, col);
            c.line((0.12, 0.82), (0.88, 0.82));
            c.poly(&[(0.28, 0.68), (0.12, 0.82), (0.28, 0.96)], false);
            c.poly(&[(0.72, 0.68), (0.88, 0.82), (0.72, 0.96)], false);
        }
        Bold | Italic | Underline | Sub | Sup | Case | Fonts | Abc => {
            let (text, size) = match icon {
                Bold => ("B", 0.95),
                Italic => ("I", 0.95),
                Underline => ("U", 0.95),
                Sub => ("x\u{2082}", 0.8),
                Sup => ("x\u{00b2}", 0.8),
                Case => ("Aa", 0.75),
                Fonts => ("Aa", 1.0),
                Abc => ("ABC", 0.5),
                _ => ("Abc", 0.75),
            };
            let font = eframe::egui::FontId::proportional(side * size);
            let ctr = c.pt(0.5, 0.5);
            p.text(
                ctr,
                eframe::egui::Align2::CENTER_CENTER,
                text,
                font.clone(),
                col,
            );
            if icon == Bold {
                p.text(
                    ctr + Vec2::new(0.8, 0.0),
                    eframe::egui::Align2::CENTER_CENTER,
                    text,
                    font,
                    col,
                );
            }
            if icon == Underline {
                c.line((0.25, 0.9), (0.75, 0.9));
            }
        }
        Star => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|k| {
                    let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                    let rad = if k % 2 == 0 { 0.42 } else { 0.18 };
                    (0.5 + rad * a.cos(), 0.52 + rad * a.sin())
                })
                .collect();
            c.poly(&pts, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Pos2, Rect, Vec2};
    use std::collections::HashMap;

    use super::ALL;

    fn paint_all(ppp: f32, size: f32, labels: bool, col: egui::Color32) -> (Vec<u8>, usize, usize) {
        let cols = 10usize;
        let cell = Vec2::new(size + 40.0, size + 26.0);
        let rows = ALL.len().div_ceil(cols);
        let (w, h) = ((cell.x * cols as f32) as usize, (cell.y * rows as f32) as usize);
        let ctx = egui::Context::default();
        let mut tex: HashMap<egui::TextureId, (usize, usize, Vec<egui::Color32>)> = HashMap::new();
        let mut prims = Vec::new();
        for _ in 0..2 {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w as f32, h as f32)));
            input.viewports.get_mut(&input.viewport_id).unwrap().native_pixels_per_point = Some(ppp);
            let mut out = ctx.run_ui(input, |ui| {
                let p = ui.painter().clone();
                p.rect_filled(ui.max_rect(), 0.0, egui::Color32::WHITE);
                for (i, &ic) in ALL.iter().enumerate() {
                    let o = Pos2::new((i % cols) as f32 * cell.x, (i / cols) as f32 * cell.y);
                    let ir = Rect::from_center_size(o + Vec2::new(cell.x / 2.0, size / 2.0 + 6.0), Vec2::splat(size));
                    paint(&p, ir, ic, col);
                    if labels {
                        p.text(o + Vec2::new(cell.x / 2.0, cell.y - 4.0), egui::Align2::CENTER_BOTTOM,
                            format!("{ic:?}"), egui::FontId::proportional(9.0), egui::Color32::BLACK);
                    }
                }
            });
            for (id, ds) in out.textures_delta.set.clone() { for d in ds {
                let egui::ImageData::Color(img) = d.image;
                let e = tex.entry(id).or_insert((img.size[0], img.size[1], vec![egui::Color32::TRANSPARENT; img.size[0] * img.size[1]]));
                let pos = d.pos.unwrap_or([0, 0]);
                if d.pos.is_none() && (e.0, e.1) != (img.size[0], img.size[1]) {
                    *e = (img.size[0], img.size[1], vec![egui::Color32::TRANSPARENT; img.size[0] * img.size[1]]);
                }
                for y in 0..img.size[1] { for x in 0..img.size[0] {
                    e.2[(pos[1] + y) * e.0 + pos[0] + x] = img.pixels[y * img.size[0] + x];
                } }
            } }
            out.textures_delta.clear();
            prims = ctx.tessellate(out.shapes, ppp);
        }
        let (pw, ph) = ((w as f32 * ppp) as usize, (h as f32 * ppp) as usize);
        let mut px = vec![[0f32; 4]; pw * ph];
        for cp in &prims {
            let egui::epaint::Primitive::Mesh(m) = &cp.primitive else { continue };
            let Some(t) = tex.get(&m.texture_id) else { continue };
            for tri in m.indices.chunks(3) {
                let v: Vec<_> = tri.iter().map(|&i| m.vertices[i as usize]).collect();
                let s = |q: Pos2| (q.x * ppp, q.y * ppp);
                let (a, b, c) = (s(v[0].pos), s(v[1].pos), s(v[2].pos));
                let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
                if area.abs() < 1e-6 { continue; }
                let x0 = a.0.min(b.0).min(c.0).floor().max(0.0) as usize;
                let x1 = (a.0.max(b.0).max(c.0).ceil() as usize).min(pw);
                let y0 = a.1.min(b.1).min(c.1).floor().max(0.0) as usize;
                let y1 = (a.1.max(b.1).max(c.1).ceil() as usize).min(ph);
                for y in y0..y1 { for x in x0..x1 {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    if fx < cp.clip_rect.min.x * ppp || fx > cp.clip_rect.max.x * ppp || fy < cp.clip_rect.min.y * ppp || fy > cp.clip_rect.max.y * ppp { continue; }
                    let w0 = ((b.0 - fx) * (c.1 - fy) - (b.1 - fy) * (c.0 - fx)) / area;
                    let w1 = ((c.0 - fx) * (a.1 - fy) - (c.1 - fy) * (a.0 - fx)) / area;
                    let w2 = 1.0 - w0 - w1;
                    // Small tolerance: pixel centres exactly on an edge shared by two triangles
                    // must not be dropped by both (that left white dots on the sheets).
                    const EPS: f32 = -1e-4;
                    if w0 < EPS || w1 < EPS || w2 < EPS { continue; }
                    let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                    let vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                    let tx = ((u * t.0 as f32) as usize).min(t.0 - 1);
                    let ty = ((vv * t.1 as f32) as usize).min(t.1 - 1);
                    let tc = t.2[ty * t.0 + tx];
                    let mut col = [0f32; 4];
                    for k in 0..4 {
                        let vc = [v[0].color, v[1].color, v[2].color].map(|c| c.to_array()[k] as f32 / 255.0);
                        col[k] = (w0 * vc[0] + w1 * vc[1] + w2 * vc[2]) * tc.to_array()[k] as f32 / 255.0;
                    }
                    let d = &mut px[y * pw + x];
                    for k in 0..4 { d[k] = col[k] + d[k] * (1.0 - col[3]); }
                } }
            }
        }
        let mut out = Vec::with_capacity(pw * ph * 3);
        for d in &px { for k in 0..3 { out.push((d[k].clamp(0.0, 1.0) * 255.0) as u8); } }
        (out, pw, ph)
    }

    /// Every variant paints into a rect (any size) without panicking.
    #[test]
    fn every_icon_paints() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let p = ui.painter().clone();
            for &ic in ALL {
                for s in [10.0, 16.0, 28.0, 64.0] {
                    paint(&p, Rect::from_min_size(Pos2::new(5.0, 5.0), Vec2::splat(s)), ic, egui::Color32::BLUE);
                }
            }
        });
        out.textures_delta.clear();
    }

    /// The Character Spacing fallback icon draws its arrow as shapes, so it needs no glyph that
    /// egui's fonts lack (the harness panics on a missing glyph unless told to allow it).
    #[test]
    fn the_spacing_icon_needs_no_missing_glyph() {
        let mut h = egui_kittest::Harness::builder()
            .with_size(Vec2::new(100.0, 100.0))
            .build_ui(|ui| {
                let p = ui.painter().clone();
                paint(&p, Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(28.0)), Icon::Spacing, egui::Color32::BLUE);
            });
        h.run();
    }

    #[test]
    fn names_are_unique_kebab_case() {
        let mut seen = std::collections::HashSet::new();
        for &i in ALL {
            let n = name(i);
            assert!(seen.insert(n), "duplicate name {n}");
            assert!(n.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{n}");
            assert!(kernicons::svg(n).is_some(), "{i:?} -> {n} has no embedded svg");
        }
    }

    /// Contact sheets for eyeballing: `PW_ICON_SHEET=<dir> cargo test -p kernpage icon_sheet -- --ignored`.
    #[test]
    #[ignore]
    fn icon_sheet() {
        let dir = std::env::var("PW_ICON_SHEET").expect("set PW_ICON_SHEET");
        for (ppp, size, name, col) in [
            (1.0, 28.0, "sheet-1x", crate::widgets::ACCENT_COLOR),
            (2.0, 28.0, "sheet-2x", crate::widgets::ACCENT_COLOR),
            (1.25, 28.0, "sheet-125", crate::widgets::ACCENT_COLOR),
            (1.5, 28.0, "sheet-15", crate::widgets::ACCENT_COLOR),
            (1.0, 16.0, "sheet-1x-16", crate::widgets::ACCENT_COLOR),
            (1.25, 16.0, "sheet-125-16", crate::widgets::ACCENT_COLOR),
            (1.25, 28.0, "sheet-125-disabled", crate::theme::disabled()),
        ] {
            let (px, w, h) = paint_all(ppp, size, true, col);
            let f = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
            let mut enc = png::Encoder::new(f, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&px).unwrap();
        }
    }
}
