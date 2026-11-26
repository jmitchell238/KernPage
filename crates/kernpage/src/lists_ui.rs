//! Bullets and numbering in the ribbon (docs/ui/BULLETS.md): the Bullets and Numbering split
//! buttons with their galleries, Decrease / Increase Indent, the Bullets and Numbering dialog
//! and its Bullet Character dialog. Every action is one `ListParagraphs` command, so one undo
//! step. No keyboard list behaviour (Tab, Enter, Backspace) yet: the owner deferred it.
#![allow(deprecated)]

use crate::app::{App, ListSel};
use crate::icons::Icon;
use crate::widgets::{Btn, ACCENT_COLOR};
use eframe::egui::{
    self, Color32, FontId, Key, Painter, Pos2, Popup, Rect, RichText, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};
use kernpage_core::text::ListChange;
use kernpage_core::{ListInfo, ListType};
use std::ops::Range;

/// Gallery tiles are 90 x 90 (BULLETS.md section 2), three to a row.
const TILE: f32 = 90.0;
const ORANGE: Color32 = Color32::from_rgb(0xF0, 0x8C, 0x28);
const GREY_LINE: Color32 = Color32::from_gray(190);
const BLUE: Color32 = Color32::from_rgb(0x2B, 0x6C, 0xD3);
const ERROR: Color32 = Color32::from_rgb(0xC0, 0x30, 0x30);
/// What a step of Decrease / Increase Indent is: 0.25 in (assumed, BULLETS.md section 7).
pub const INDENT_STEP_PT: f32 = 18.0;
/// The Size and Indent spin boxes step by this much.
const SIZE_STEP: f32 = 1.0;
const INDENT_SPIN_STEP_IN: f32 = 0.1;

/// The six default bullets of the document's bullet table (spec section 6: Symbol codes 183,
/// 183, 168, 222, 224, 42), with the tile names of the Bullets gallery.
pub const BULLET_TILES: [(&str, u32, f32); 6] = [
    ("Small Bullets", 183, 10.0),
    ("Large Bullets", 183, 12.0),
    ("Filled Diamond Bullets", 168, 10.0),
    ("Arrow Bullets", 222, 10.0),
    ("Open Diamond Bullets", 224, 10.0),
    ("Asterisk Bullets", 42, 10.0),
];

/// The tiles of the Numbering gallery after "None", in Publisher's order: type and separator
/// index (0 `)`, 2 `.`).
pub const NUMBER_TILES: [(ListType, u8); 8] = [
    (ListType::Arabic, 2),
    (ListType::Arabic, 0),
    (ListType::UpperRoman, 2),
    (ListType::UpperLetter, 2),
    (ListType::LowerLetter, 2),
    (ListType::LowerLetter, 0),
    (ListType::LowerRoman, 2),
    (ListType::LowerRoman, 0),
];

/// The Format list of the Numbering tab (BULLETS.md section 7; the last three are the
/// remaining types of spec 4R.2, their wording is a guess).
pub const FORMATS: [(ListType, &str); 9] = [
    (ListType::Arabic, "1, 2, 3, \u{2026}"),
    (ListType::UpperRoman, "I, II, III, \u{2026}"),
    (ListType::LowerRoman, "i, ii, iii, \u{2026}"),
    (ListType::UpperLetter, "A, B, C, \u{2026}"),
    (ListType::LowerLetter, "a, b, c, \u{2026}"),
    (ListType::Ordinal, "1st, 2nd, 3rd \u{2026}"),
    (ListType::CardinalText, "One, Two, Three \u{2026}"),
    (ListType::OrdinalText, "First, Second, Third \u{2026}"),
    (ListType::LeadingZero, "01, 02, 03, \u{2026}"),
];

/// The Separator list: separator index (spec 4R.3) and its text. The order is the one the
/// dialog showed through UI Automation (BULLETS.md section 3); the texts are the bare marks.
pub const SEPARATORS: [(u8, &str); 9] = [
    (3, "None"),
    (2, "."),
    (0, ")"),
    (4, "]"),
    (5, ":"),
    (1, "()"),
    (6, "[]"),
    (7, "--"),
    (8, "\u{3001}"),
];

/// Fonts offered by the Bullet Character dialog.
pub const CHAR_FONTS: [&str; 8] = [
    "Symbol",
    "Wingdings",
    "Webdings",
    "Arial",
    "Calibri",
    "Courier New",
    "Times New Roman",
    "Segoe UI Symbol",
];

const GRID_COLS: usize = 32;
const GRID_ROWS: usize = 7;
const CELL: f32 = 17.0;
const FIRST_CODE: u32 = 32;
const RECENT_MAX: usize = 32;

/// The "Recently used symbols" a new session starts with (the owner's screenshot).
pub fn default_recent() -> Vec<(u32, String)> {
    "\u{20AC}\u{A3}\u{A5}\u{A9}\u{AE}\u{2122}\u{B1}\u{2260}\u{2264}\u{2265}\u{F7}\u{D7}\u{221E}\u{B5}\u{3B1}\u{3B2}\u{3C0}\u{3A9}\u{2211}\u{263A}\u{2639}\u{A7}\u{2020}\u{2021}\u{2026}%\u{E9}\u{BF}\u{F1}\u{2190}\u{2191}\u{2192}"
        .chars()
        .map(|c| (c as u32, "Arial".to_owned()))
        .collect()
}

/// Test seam: the gallery tiles, dialog bullet buttons and character grids drawn this frame.
#[cfg(test)]
pub mod probe {
    use eframe::egui::Rect;
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
    pub struct Item {
        /// `bullets-gallery`, `numbering-gallery`, `dialog-bullet`, `chars-grid`, `chars-recent`.
        pub area: &'static str,
        pub name: String,
        pub rect: Rect,
        pub selected: bool,
    }

    thread_local! {
        static ITEMS: RefCell<Vec<Item>> = const { RefCell::new(Vec::new()) };
    }

    pub fn clear() {
        ITEMS.with(|v| v.borrow_mut().clear());
    }
    pub fn add(i: Item) {
        ITEMS.with(|v| v.borrow_mut().push(i));
    }
    pub fn all() -> Vec<Item> {
        ITEMS.with(|v| v.borrow().clone())
    }
    pub fn find(area: &str, name: &str) -> Option<Item> {
        all().into_iter().find(|i| i.area == area && i.name == name)
    }
}

#[cfg(test)]
fn record(area: &'static str, name: &str, rect: Rect, selected: bool) {
    probe::add(probe::Item {
        area,
        name: name.to_owned(),
        rect,
        selected,
    });
}
#[cfg(not(test))]
fn record(_: &'static str, _: &str, _: Rect, _: bool) {}

// ---- text of the spin boxes ---------------------------------------------------------------

/// `10pt`, `10.5pt`.
pub fn fmt_size(pt: f32) -> String {
    if (pt - pt.round()).abs() < 0.005 {
        format!("{}pt", pt.round() as i32)
    } else {
        format!("{pt:.1}pt")
    }
}

/// A bullet size in points: 1 to 400 (what Publisher accepts, spec 4R.4); `pt` optional.
pub fn parse_size(text: &str) -> Option<f32> {
    let t = text.trim().trim_end_matches("pt").trim();
    t.parse::<f32>().ok().filter(|v| (1.0..=400.0).contains(v))
}

/// `0.25"`.
pub fn fmt_inches(pt: f32) -> String {
    format!("{:.2}\"", pt / 72.0)
}

/// An indent in inches (`0.25"`, `.5`, `0.5 in`), returned in points; 0 to 22 inches.
pub fn parse_inches(text: &str) -> Option<f32> {
    let t = text.trim().trim_end_matches("in").trim_end_matches('"').trim();
    let v = t.parse::<f32>().ok().filter(|v| (0.0..=22.0).contains(v))?;
    Some((v * 72.0 * 100.0).round() / 100.0)
}

/// Start at: 0 up to 100,000 (Publisher took at least that).
pub fn parse_start(text: &str) -> Option<u32> {
    text.trim().parse::<u32>().ok().filter(|v| *v <= 100_000)
}

// ---- what a tile or the sample shows -------------------------------------------------------

/// The three sample labels of a numbering format.
fn sample_labels(kind: ListType, separator: u8, start: u32) -> Vec<String> {
    let info = ListInfo {
        separator,
        start,
        ..ListInfo::numbered(kind)
    };
    pub_format::list_labels_for([Some(&info), Some(&info), Some(&info)].map(|l| (l, false)))
        .into_iter()
        .map(|l| l.unwrap_or_default())
        .collect()
}

#[derive(Clone)]
enum Art {
    /// The word "None".
    NoList,
    Bullet(u32, String, f32),
    Numbers(Vec<String>),
}

/// Draw a bullet of Symbol code `code` centred on `c`, `size_pt` big (10 pt = 1 unit).
fn paint_bullet(p: &Painter, c: Pos2, code: u32, font: &str, size_pt: f32, color: Color32) {
    let u = size_pt / 10.0;
    let symbol = font.eq_ignore_ascii_case("symbol");
    let stroke = Stroke::new(1.2, color);
    match (symbol, code) {
        (true, 183) => {
            p.circle_filled(c, 2.6 * u, color);
        }
        (true, 168) => {
            let r = 4.0 * u;
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(0.0, -r),
                    c + Vec2::new(r * 0.75, 0.0),
                    c + Vec2::new(0.0, r),
                    c + Vec2::new(-r * 0.75, 0.0),
                ],
                color,
                Stroke::NONE,
            ));
        }
        (true, 224) => {
            let r = 4.0 * u;
            p.add(egui::Shape::closed_line(
                vec![
                    c + Vec2::new(0.0, -r),
                    c + Vec2::new(r * 0.75, 0.0),
                    c + Vec2::new(0.0, r),
                    c + Vec2::new(-r * 0.75, 0.0),
                ],
                stroke,
            ));
        }
        (true, 222) => {
            let r = 4.5 * u;
            for dy in [-1.4 * u, 1.4 * u] {
                p.line_segment(
                    [c + Vec2::new(-r, dy), c + Vec2::new(r * 0.5, dy)],
                    stroke,
                );
            }
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(r * 0.2, -r * 0.8),
                    c + Vec2::new(r, 0.0),
                    c + Vec2::new(r * 0.2, r * 0.8),
                ],
                color,
                Stroke::NONE,
            ));
        }
        (true, 42) => {
            let r = 3.6 * u;
            for a in [0.0_f32, std::f32::consts::FRAC_PI_3, 2.0 * std::f32::consts::FRAC_PI_3] {
                let d = Vec2::new(a.cos(), a.sin()) * r;
                p.line_segment([c - d, c + d], stroke);
            }
        }
        _ => {
            let ch = pub_format::bullet_display_char(code, font);
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                ch,
                FontId::proportional(12.0 * u),
                color,
            );
        }
    }
}

fn paint_art(p: &Painter, rect: Rect, art: &Art, indent_pt: f32) {
    match art {
        Art::NoList => {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "None",
                FontId::proportional(17.0),
                crate::theme::text(),
            );
        }
        Art::Bullet(..) | Art::Numbers(_) => {
            let rows = 3;
            let step = rect.height() / (rows as f32 + 1.0);
            let pad = (indent_pt * 0.5).clamp(0.0, 24.0);
            for i in 0..rows {
                let y = rect.min.y + step * (i as f32 + 1.0);
                let x0 = rect.min.x + 10.0;
                match art {
                    Art::Bullet(code, font, size) => {
                        paint_bullet(p, Pos2::new(x0 + 4.0, y), *code, font, *size, crate::theme::text())
                    }
                    Art::Numbers(l) => {
                        p.text(
                            Pos2::new(x0, y),
                            egui::Align2::LEFT_CENTER,
                            &l[i],
                            FontId::proportional(10.0),
                            crate::theme::text(),
                        );
                    }
                    Art::NoList => {}
                }
                p.line_segment(
                    [
                        Pos2::new(x0 + 16.0 + pad * 0.4, y),
                        Pos2::new(rect.max.x - 8.0, y),
                    ],
                    Stroke::new(1.5, GREY_LINE),
                );
            }
        }
    }
}

/// One gallery tile; true when clicked.
fn gallery_tile(ui: &mut Ui, area: &'static str, name: &str, selected: bool, art: &Art) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(TILE), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let resp = resp.on_hover_text(name);
    if ui.is_rect_visible(rect) {
        let p = ui.painter_at(rect);
        let bg = if resp.hovered() {
            Color32::from_rgb(0xF4, 0xF2, 0xF0)
        } else {
            Color32::WHITE
        };
        p.rect_filled(rect.shrink(1.0), 2.0, bg);
        p.rect_stroke(
            rect.shrink(1.0),
            2.0,
            if selected {
                Stroke::new(2.5, ORANGE)
            } else {
                Stroke::new(1.0, crate::theme::button_outline())
            },
            StrokeKind::Inside,
        );
        paint_art(&p, rect, art, 0.0);
    }
    record(area, name, rect, selected);
    resp.clicked()
}

// ---- the dialog ------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListTab {
    Bullets,
    Numbering,
}

/// The Bullet Character dialog: a font, a character code in it.
pub struct CharDialog {
    pub font: String,
    pub code: u32,
    pub code_text: String,
}

/// State of the Bullets and Numbering dialog. Nothing reaches the document until OK.
pub struct ListDialog {
    /// Characters (in the editable text) the dialog applies to, fixed when it opened.
    pub range: Range<usize>,
    pub tab: ListTab,
    /// Bullets tab: the bullet (code, font); `None` is the blank "no bullet" button.
    pub bullet: Option<(u32, String)>,
    pub size: String,
    pub bullet_indent: String,
    /// Numbering tab.
    pub format: ListType,
    pub separator: u8,
    pub start: String,
    pub num_indent: String,
    pub chars: Option<CharDialog>,
}

impl ListDialog {
    /// A dialog pre-filled from the selection's list (`None` for none or a mixed selection);
    /// it opens on the tab of the list it shows, else on `tab`.
    pub fn new(tab: ListTab, range: Range<usize>, list: Option<&ListInfo>) -> Self {
        let mut d = ListDialog {
            range,
            tab,
            bullet: None,
            size: fmt_size(10.0),
            bullet_indent: fmt_inches(18.0),
            format: ListType::Arabic,
            separator: 2,
            start: "1".into(),
            num_indent: fmt_inches(18.0),
            chars: None,
        };
        if let Some(l) = list {
            let indent = fmt_inches(l.list_indent_pt);
            if l.kind.is_bullet() {
                d.tab = ListTab::Bullets;
                let font = if l.bullet_font.is_empty() { "Symbol" } else { &l.bullet_font };
                d.bullet = Some((l.bullet_char, font.to_owned()));
                d.size = fmt_size(l.bullet_size_pt.unwrap_or(10.0));
                d.bullet_indent = indent;
            } else {
                d.tab = ListTab::Numbering;
                d.format = match l.kind {
                    ListType::Other(_) => ListType::Arabic,
                    k => k,
                };
                d.separator = l.separator;
                d.start = l.start.to_string();
                d.num_indent = indent;
            }
        }
        d
    }

    /// What OK applies, or `None` while a box holds something unusable.
    pub fn change(&self) -> Option<ListChange> {
        match self.tab {
            ListTab::Bullets => match &self.bullet {
                None => Some(ListChange::Clear),
                Some((code, font)) => Some(ListChange::Set(ListInfo {
                    list_indent_pt: parse_inches(&self.bullet_indent)?,
                    ..ListInfo::bullet_with(*code, font, parse_size(&self.size)?)
                })),
            },
            ListTab::Numbering => Some(ListChange::Set(ListInfo {
                separator: self.separator,
                start: parse_start(&self.start)?,
                list_indent_pt: parse_inches(&self.num_indent)?,
                ..ListInfo::numbered(self.format)
            })),
        }
    }

    /// The bullet slot (0 = blank, 1..=6) the Bullets tab shows as chosen.
    fn slot(&self) -> Option<usize> {
        let (code, font) = self.bullet.as_ref().map_or((None, ""), |(c, f)| (Some(*c), f));
        let Some(code) = code else { return Some(0) };
        let size = parse_size(&self.size)?;
        if !font.eq_ignore_ascii_case("symbol") {
            return None;
        }
        BULLET_TILES
            .iter()
            .position(|(_, c, s)| *c == code && (*s - size).abs() < 0.01)
            .map(|i| i + 1)
    }
}

/// An edit box with a spinner: `text` is what is typed, `step(text, up)` gives the new text
/// when an arrow is clicked.
fn spin_box(
    ui: &mut Ui,
    id: &'static str,
    label: &str,
    text: &mut String,
    enabled: bool,
    valid: bool,
    step: impl Fn(&str, bool) -> Option<String>,
) {
    ui.horizontal(|ui| {
        let l = ui.add_sized([110.0, 20.0], egui::Label::new(label));
        ui.add_enabled_ui(enabled, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .id(egui::Id::new(("list_dialog", id)))
                    .desired_width(70.0)
                    .text_color(if valid { crate::theme::text() } else { ERROR }),
            )
            .labelled_by(l.id);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for up in [true, false] {
                    let (ar, a) = ui.allocate_exact_size(Vec2::new(16.0, 10.0), Sense::click());
                    a.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            if up { format!("{label} up") } else { format!("{label} down") },
                        )
                    });
                    let bg = if a.hovered() && enabled {
                        ACCENT_COLOR.gamma_multiply(0.3)
                    } else {
                        Color32::from_gray(228)
                    };
                    ui.painter().rect_filled(ar, 1.0, bg);
                    let c = ar.center();
                    let (tip, base) = if up { (-2.0, 2.0) } else { (2.0, -2.0) };
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![
                            c + Vec2::new(0.0, tip),
                            c + Vec2::new(3.5, base),
                            c + Vec2::new(-3.5, base),
                        ],
                        Color32::from_gray(60),
                        Stroke::NONE,
                    ));
                    if a.clicked() {
                        if let Some(t) = step(text, up) {
                            *text = t;
                        }
                    }
                }
            });
        });
    });
}

fn step_size(t: &str, up: bool) -> Option<String> {
    let v = parse_size(t)?;
    Some(fmt_size((v + if up { SIZE_STEP } else { -SIZE_STEP }).clamp(1.0, 400.0)))
}

fn step_inches(t: &str, up: bool) -> Option<String> {
    let v = parse_inches(t)? / 72.0;
    let n = v + if up { INDENT_SPIN_STEP_IN } else { -INDENT_SPIN_STEP_IN };
    Some(fmt_inches(n.clamp(0.0, 22.0) * 72.0))
}

fn step_start(t: &str, up: bool) -> Option<String> {
    let v = parse_start(t)?;
    Some(if up { (v + 1).min(100_000) } else { v.saturating_sub(1) }.to_string())
}

/// The "Sample" box: grey lines above and below, the list paragraphs between.
fn sample_box(ui: &mut Ui, art: &Art, indent_pt: f32) {
    ui.label("Sample");
    let (rect, _) = ui.allocate_exact_size(Vec2::new(300.0, 100.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, Color32::WHITE);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::button_outline()), StrokeKind::Inside);
    let line = |y: f32, col: Color32| {
        p.line_segment(
            [Pos2::new(rect.min.x + 12.0, y), Pos2::new(rect.max.x - 12.0, y)],
            Stroke::new(2.0, col),
        );
    };
    line(rect.min.y + 12.0, GREY_LINE);
    line(rect.max.y - 12.0, GREY_LINE);
    let inner = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.min.y + 12.0),
        Pos2::new(rect.max.x, rect.max.y - 12.0),
    );
    let step = inner.height() / 4.0;
    let off = (indent_pt * 0.8).clamp(0.0, 60.0);
    for i in 0..3 {
        let y = inner.min.y + step * (i as f32 + 1.0);
        let x = rect.min.x + 14.0;
        match art {
            Art::Bullet(code, font, size) => paint_bullet(
                &p,
                Pos2::new(x + 4.0, y),
                *code,
                font,
                size.min(14.0),
                Color32::BLACK,
            ),
            Art::Numbers(l) => {
                p.text(
                    Pos2::new(x, y),
                    egui::Align2::LEFT_CENTER,
                    &l[i],
                    FontId::proportional(11.0),
                    Color32::BLACK,
                );
            }
            Art::NoList => {}
        }
        p.line_segment(
            [
                Pos2::new(x + 6.0 + off, y),
                Pos2::new(rect.max.x - 12.0, y),
            ],
            Stroke::new(2.0, Color32::BLACK),
        );
    }
}

/// The character `code` shows as in the grid of `font`, if it has a glyph there.
fn grid_glyph(code: u32, font: &str) -> Option<char> {
    let f = font.to_ascii_lowercase();
    if f == "symbol" {
        return pub_format::symbol_unicode(code);
    }
    if pub_format::is_symbol_font(font) {
        return Some(pub_format::bullet_display_char(code, font));
    }
    char::from_u32(code).filter(|c| !c.is_control())
}

impl App {
    /// The ribbon controls: Bullets and Numbering split buttons, Decrease and Increase Indent.
    pub(crate) fn list_buttons(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let sel = self.list_selection(&ctx);
        let has_text = sel.is_some();
        // `None` = paragraphs differ, `Some(None)` = no list.
        let shared: Option<Option<ListInfo>> = sel
            .as_ref()
            .and_then(|s| s.list.as_ref())
            .map(|(l, _)| l.clone());
        let ListSel { range: _, list } = sel.unwrap_or(ListSel { range: 0..0, list: None });
        let keep_indent = |mut info: ListInfo| {
            if let Some(Some(cur)) = &shared {
                info.list_indent_pt = cur.list_indent_pt;
            }
            info
        };
        let bullet_on = matches!(&shared, Some(Some(l)) if l.kind.is_bullet());
        let number_on = matches!(&shared, Some(Some(l)) if !l.kind.is_bullet());
        let mut change: Option<ListChange> = None;
        let mut dialog: Option<ListTab> = None;

        // Bullets
        let (main, arrow) = crate::widgets::split_button(
            ui,
            Btn::small(Icon::Bullets, "")
                .enabled(has_text)
                .selected(bullet_on)
                .tip("Bullets"),
        );
        if main.clicked() {
            change = Some(if bullet_on {
                ListChange::Clear
            } else {
                ListChange::Set(keep_indent(ListInfo::bullet()))
            });
        }
        Popup::menu(&arrow).show(|ui| {
            ui.label(RichText::new("Bullet Styles").strong().color(crate::theme::text()));
            let cur_none = matches!(shared, Some(None));
            ui.spacing_mut().item_spacing = Vec2::splat(4.0);
            let mut tiles: Vec<(String, bool, Art, Option<ListChange>)> = vec![(
                "None".to_owned(),
                cur_none,
                Art::NoList,
                Some(ListChange::Clear),
            )];
            for (name, code, size) in BULLET_TILES {
                let on = matches!(&shared, Some(Some(l)) if l.kind.is_bullet()
                    && l.bullet_char == code
                    && (l.bullet_size_pt.unwrap_or(10.0) - size).abs() < 0.01
                    && (l.bullet_font.is_empty() || l.bullet_font.eq_ignore_ascii_case("symbol")));
                tiles.push((
                    name.to_owned(),
                    on,
                    Art::Bullet(code, "Symbol".into(), size),
                    Some(ListChange::Set(keep_indent(ListInfo::bullet_with(code, "Symbol", size)))),
                ));
            }
            egui::Grid::new("bullet_tiles").spacing([4.0, 4.0]).show(ui, |ui| {
                for (n, (name, on, art, chg)) in tiles.into_iter().enumerate() {
                    if gallery_tile(ui, "bullets-gallery", &name, on, &art) {
                        change = chg;
                        ui.close();
                    }
                    if n % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.separator();
            if ui.button("Bullets and Numbering...").clicked() {
                dialog = Some(ListTab::Bullets);
                ui.close();
            }
        });

        // Numbering
        let (main, arrow) = crate::widgets::split_button(
            ui,
            Btn::small(Icon::Numbering, "")
                .enabled(has_text)
                .selected(number_on)
                .tip("Numbering"),
        );
        if main.clicked() {
            change = Some(if number_on {
                ListChange::Clear
            } else {
                ListChange::Set(keep_indent(ListInfo::numbered(ListType::Arabic)))
            });
        }
        Popup::menu(&arrow).show(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.0);
            let cur_none = matches!(shared, Some(None));
            let mut tiles: Vec<(String, bool, Art, Option<ListChange>)> = vec![(
                "None".to_owned(),
                cur_none,
                Art::NoList,
                Some(ListChange::Clear),
            )];
            for (kind, sep) in NUMBER_TILES {
                let labels = sample_labels(kind, sep, 1);
                let on = matches!(&shared, Some(Some(l)) if l.kind == kind && l.separator == sep);
                tiles.push((
                    labels.join(" "),
                    on,
                    Art::Numbers(labels),
                    Some(ListChange::Set(keep_indent(ListInfo {
                        separator: sep,
                        ..ListInfo::numbered(kind)
                    }))),
                ));
            }
            egui::Grid::new("number_tiles").spacing([4.0, 4.0]).show(ui, |ui| {
                for (n, (name, on, art, chg)) in tiles.into_iter().enumerate() {
                    if gallery_tile(ui, "numbering-gallery", &name, on, &art) {
                        change = chg;
                        ui.close();
                    }
                    if n % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.separator();
            if ui.button("Bullets and Numbering...").clicked() {
                dialog = Some(ListTab::Numbering);
                ui.close();
            }
        });

        crate::widgets::divider(ui);
        // Decrease / Increase Indent: a step is 0.25 in; Decrease is greyed at 0.
        let at_zero = matches!(&list, Some((l, ind)) if match l {
            Some(l) => l.list_indent_pt <= 0.0,
            None => ind.left_pt <= 0.0,
        });
        if Btn::small(Icon::IndentLess, "")
            .enabled(has_text && !at_zero)
            .tip("Decrease Indent")
            .show(ui)
            .clicked()
        {
            change = Some(ListChange::StepIndent(-INDENT_STEP_PT));
        }
        if Btn::small(Icon::IndentMore, "")
            .enabled(has_text)
            .tip("Increase Indent")
            .show(ui)
            .clicked()
        {
            change = Some(ListChange::StepIndent(INDENT_STEP_PT));
        }

        if let Some(c) = change {
            self.format_list(&ctx, None, c);
        }
        if let Some(tab) = dialog {
            self.open_list_dialog(&ctx, tab);
        }
    }

    /// Open the Bullets and Numbering dialog, pre-filled from the selection.
    pub fn open_list_dialog(&mut self, ctx: &egui::Context, tab: ListTab) {
        let Some(sel) = self.list_selection(ctx) else {
            return;
        };
        let info = sel.list.and_then(|(l, _)| l);
        self.list_dialog = Some(ListDialog::new(tab, sel.range, info.as_ref()));
    }

    #[allow(deprecated)]
    pub(crate) fn list_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.list_dialog else {
            return;
        };
        let (mut ok, mut cancel) = (false, false);
        let chars_open = d.chars.is_some();
        let usable = d.change().is_some();
        egui::Modal::new(egui::Id::new("list_dialog")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.heading("Bullets and Numbering");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut d.tab, ListTab::Bullets, "Bullets");
                ui.selectable_value(&mut d.tab, ListTab::Numbering, "Numbering");
            });
            ui.separator();
            match d.tab {
                ListTab::Bullets => bullets_tab(ui, d),
                ListTab::Numbering => numbering_tab(ui, d),
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel = ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 0.0))).clicked();
                ok = ui
                    .add_enabled(
                        usable,
                        egui::Button::new("OK")
                            .min_size(Vec2::new(80.0, 0.0))
                            .stroke(Stroke::new(1.5, ACCENT_COLOR)),
                    )
                    .clicked();
            });
        });
        let mut char_ok = false;
        let mut char_cancel = false;
        if let Some(c) = d.chars.as_mut() {
            let recent = self.recent_bullets.clone();
            egui::Modal::new(egui::Id::new("bullet_char_dialog")).show(ctx, |ui| {
                char_dialog_body(ui, c, &recent, &mut char_ok, &mut char_cancel);
            });
        }
        let (enter, esc) = ctx.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if chars_open {
            char_ok |= enter;
            char_cancel |= esc;
        } else {
            ok |= enter && usable;
            cancel |= esc;
        }
        if char_ok {
            if let Some(c) = d.chars.take() {
                if let Some(code) = Some(c.code).filter(|c| (1..=65535).contains(c)) {
                    d.bullet = Some((code, c.font.clone()));
                    self.recent_bullets.retain(|(rc, rf)| !(*rc == code && *rf == c.font));
                    self.recent_bullets.insert(0, (code, c.font));
                    self.recent_bullets.truncate(RECENT_MAX);
                }
            }
        } else if char_cancel {
            d.chars = None;
        }
        if ok {
            if let Some(d) = self.list_dialog.take() {
                if let Some(change) = d.change() {
                    self.format_list(ctx, Some(d.range), change);
                }
            }
        } else if cancel {
            self.list_dialog = None;
        }
    }
}

fn bullets_tab(ui: &mut Ui, d: &mut ListDialog) {
    ui.label("Bullet character");
    let slot = d.slot();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for i in 0..=BULLET_TILES.len() {
            let name = if i == 0 {
                "No Bullet".to_owned()
            } else {
                format!("Bullet type {i}")
            };
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
            let on = slot == Some(i);
            let p = ui.painter_at(rect);
            p.rect_filled(
                rect,
                1.0,
                if resp.hovered() { Color32::from_rgb(0xF4, 0xF2, 0xF0) } else { Color32::WHITE },
            );
            p.rect_stroke(
                rect,
                1.0,
                if on { Stroke::new(2.5, Color32::BLACK) } else { Stroke::new(1.0, crate::theme::button_outline()) },
                StrokeKind::Inside,
            );
            if i > 0 {
                let (_, code, size) = BULLET_TILES[i - 1];
                paint_bullet(&p, rect.center(), code, "Symbol", size, Color32::BLACK);
            }
            record("dialog-bullet", &name, rect, on);
            if resp.clicked() {
                if i == 0 {
                    d.bullet = None;
                } else {
                    let (_, code, size) = BULLET_TILES[i - 1];
                    d.bullet = Some((code, "Symbol".to_owned()));
                    d.size = fmt_size(size);
                }
            }
        }
    });
    ui.add_space(6.0);
    let has = d.bullet.is_some();
    let (size_ok, indent_ok) = (parse_size(&d.size).is_some(), parse_inches(&d.bullet_indent).is_some());
    spin_box(ui, "size", "Size:", &mut d.size, has, size_ok, step_size);
    ui.horizontal(|ui| {
        ui.add_space(114.0);
        if ui.add_enabled(has, egui::Button::new("Character...")).clicked() {
            if let Some((code, font)) = &d.bullet {
                d.chars = Some(CharDialog {
                    font: font.clone(),
                    code: *code,
                    code_text: code.to_string(),
                });
            }
        }
    });
    spin_box(ui, "indent", "Indent list by:", &mut d.bullet_indent, has, indent_ok, step_inches);
    let art = match &d.bullet {
        Some((c, f)) => Art::Bullet(*c, f.clone(), parse_size(&d.size).unwrap_or(10.0)),
        None => Art::NoList,
    };
    let indent = if has { parse_inches(&d.bullet_indent).unwrap_or(18.0) } else { 0.0 };
    sample_box(ui, &art, indent);
}

fn numbering_tab(ui: &mut Ui, d: &mut ListDialog) {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new("Format:"));
        let cur = FORMATS.iter().find(|(k, _)| *k == d.format).map_or("", |(_, t)| *t);
        egui::ComboBox::from_id_salt("list_format")
            .selected_text(cur)
            .width(170.0)
            .show_ui(ui, |ui| {
                for (k, t) in FORMATS {
                    ui.selectable_value(&mut d.format, k, t);
                }
            });
    });
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new("Separator:"));
        let cur = SEPARATORS.iter().find(|(i, _)| *i == d.separator).map_or("", |(_, t)| *t);
        egui::ComboBox::from_id_salt("list_separator")
            .selected_text(cur)
            .width(70.0)
            .show_ui(ui, |ui| {
                for (i, t) in SEPARATORS {
                    ui.selectable_value(&mut d.separator, i, t);
                }
            });
    });
    let (start_ok, indent_ok) = (parse_start(&d.start).is_some(), parse_inches(&d.num_indent).is_some());
    spin_box(ui, "start", "Start at:", &mut d.start, true, start_ok, step_start);
    spin_box(ui, "num_indent", "Indent list by:", &mut d.num_indent, true, indent_ok, step_inches);
    ui.add_space(6.0);
    let art = Art::Numbers(sample_labels(d.format, d.separator, parse_start(&d.start).unwrap_or(1)));
    sample_box(ui, &art, parse_inches(&d.num_indent).unwrap_or(18.0));
}

fn char_dialog_body(
    ui: &mut Ui,
    c: &mut CharDialog,
    recent: &[(u32, String)],
    ok: &mut bool,
    cancel: &mut bool,
) {
    ui.heading("Bullet Character");
    ui.horizontal(|ui| {
        ui.label("Font:");
        egui::ComboBox::from_id_salt("bullet_char_font")
            .selected_text(c.font.clone())
            .width(180.0)
            .show_ui(ui, |ui| {
                for f in CHAR_FONTS {
                    ui.selectable_value(&mut c.font, f.to_owned(), f);
                }
            });
    });
    // The grid: codes 32 to 255, 32 to a row, 7 rows.
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(GRID_COLS as f32 * CELL, GRID_ROWS as f32 * CELL),
        Sense::click(),
    );
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, Color32::WHITE);
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let code = FIRST_CODE + i as u32;
        let r = Rect::from_min_size(
            rect.min + Vec2::new((i % GRID_COLS) as f32 * CELL, (i / GRID_COLS) as f32 * CELL),
            Vec2::splat(CELL),
        );
        let on = code == c.code;
        p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(215)), StrokeKind::Inside);
        if on {
            p.rect_filled(r.shrink(1.0), 0.0, BLUE);
        }
        if let Some(g) = grid_glyph(code, &c.font) {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                g,
                FontId::proportional(13.0),
                if on { Color32::WHITE } else { Color32::BLACK },
            );
        }
    }
    record("chars-grid", "grid", rect, false);
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let d = pos - rect.min;
            let (col, row) = ((d.x / CELL) as usize, (d.y / CELL) as usize);
            if col < GRID_COLS && row < GRID_ROWS {
                c.code = FIRST_CODE + (row * GRID_COLS + col) as u32;
                c.code_text = c.code.to_string();
            }
        }
    }
    ui.label("Recently used symbols:");
    let (rrect, rresp) = ui.allocate_exact_size(
        Vec2::new(RECENT_MAX as f32 * CELL, CELL),
        Sense::click(),
    );
    let rp = ui.painter_at(rrect);
    rp.rect_filled(rrect, 0.0, Color32::WHITE);
    for (i, (code, _)) in recent.iter().take(RECENT_MAX).enumerate() {
        let r = Rect::from_min_size(rrect.min + Vec2::new(i as f32 * CELL, 0.0), Vec2::splat(CELL));
        rp.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(215)), StrokeKind::Inside);
        if let Some(ch) = char::from_u32(*code) {
            rp.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                ch,
                FontId::proportional(13.0),
                Color32::BLACK,
            );
        }
    }
    record("chars-recent", "recent", rrect, false);
    if rresp.clicked() {
        if let Some(pos) = rresp.interact_pointer_pos() {
            let i = ((pos.x - rrect.min.x) / CELL) as usize;
            if let Some((code, font)) = recent.get(i) {
                c.code = *code;
                c.font = font.clone();
                c.code_text = code.to_string();
            }
        }
    }
    ui.add_space(4.0);
    ui.label(format!("Symbol: {}", c.code));
    ui.horizontal(|ui| {
        let l = ui.label("Character code:");
        let r = ui
            .add(
                egui::TextEdit::singleline(&mut c.code_text)
                    .id(egui::Id::new("bullet_char_code"))
                    .desired_width(60.0),
            )
            .labelled_by(l.id);
        if r.changed() {
            if let Ok(v) = c.code_text.trim().parse::<u32>() {
                if (1..=65535).contains(&v) {
                    c.code = v;
                }
            }
        }
        ui.label("from:");
        ui.label(format!("{} (decimal)", c.font));
    });
    ui.add_space(6.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        *cancel = ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 0.0))).clicked();
        *ok = ui
            .add(
                egui::Button::new("OK")
                    .min_size(Vec2::new(80.0, 0.0))
                    .stroke(Stroke::new(1.5, ACCENT_COLOR)),
            )
            .clicked();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spin_box_texts_round_trip() {
        assert_eq!(fmt_size(10.0), "10pt");
        assert_eq!(fmt_size(10.5), "10.5pt");
        assert_eq!(parse_size("12pt"), Some(12.0));
        assert_eq!(parse_size(" 7 "), Some(7.0));
        assert_eq!(parse_size("0"), None);
        assert_eq!(parse_size("401"), None);
        assert_eq!(fmt_inches(18.0), "0.25\"");
        assert_eq!(parse_inches("0.25\""), Some(18.0));
        assert_eq!(parse_inches("1"), Some(72.0));
        assert_eq!(parse_inches("-1"), None);
        assert_eq!(parse_inches("x"), None);
        assert_eq!(parse_start("5"), Some(5));
        assert_eq!(parse_start("-1"), None);
        assert_eq!(parse_start("100001"), None);
    }

    #[test]
    fn sample_labels_follow_format_and_separator() {
        assert_eq!(sample_labels(ListType::Arabic, 2, 1), ["1.", "2.", "3."]);
        assert_eq!(sample_labels(ListType::LowerLetter, 0, 1), ["a)", "b)", "c)"]);
        assert_eq!(sample_labels(ListType::UpperRoman, 3, 4), ["IV", "V", "VI"]);
    }
}
