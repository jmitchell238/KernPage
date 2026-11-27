//! The File screen: a teal command rail on the left and one page per command (Home, New, Open,
//! Info, Save As, Print, Share, Export, ...). While it is open the tab row and ribbon are hidden.

use crate::app::{App, Pending, Section};
use crate::icons::{self, Icon};
use crate::recent;
use crate::widgets::COMING_SOON;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use kernpage_core::template;
use std::path::{Path, PathBuf};

/// The command rail: KernPage teal with white text, the same in light and dark themes (the
/// dark theme only deepens it a little).
fn rail_fill() -> Color32 {
    if crate::theme::is_dark() {
        Color32::from_rgb(0x0F, 0x4E, 0x54)
    } else {
        Color32::from_rgb(0x12, 0x63, 0x6B)
    }
}
/// The selected rail item's darker teal.
fn rail_selected() -> Color32 {
    if crate::theme::is_dark() {
        Color32::from_rgb(0x0A, 0x3A, 0x3F)
    } else {
        Color32::from_rgb(0x0D, 0x4D, 0x54)
    }
}
fn rail_text() -> Color32 {
    Color32::WHITE
}
/// White at reduced opacity: disabled rail items.
fn rail_dim() -> Color32 {
    Color32::WHITE.gamma_multiply(0.45)
}
/// A thin light tint on the rail: dividers.
fn rail_line() -> Color32 {
    crate::theme::mix(rail_fill(), Color32::WHITE, 0.28)
}
fn rail_hover() -> Color32 {
    crate::theme::mix(rail_fill(), Color32::WHITE, 0.10)
}

/// Rail width in logical px.
const RAIL_W: f32 = 190.0;
/// Rail row height.
const RAIL_ROW: f32 = 36.0;
const RAIL_SEP: f32 = 14.0;

/// A rectangle on the device-pixel grid.
fn snap(ui: &Ui, r: Rect) -> Rect {
    crate::widgets::snap_rect(ui.ctx().pixels_per_point(), r)
}

/// A one-device-pixel horizontal line at `y`, centred on a pixel row so it is crisp.
fn hline(ui: &Ui, x0: f32, x1: f32, y: f32, col: Color32) {
    let ppp = ui.ctx().pixels_per_point();
    let y = ((y * ppp).floor() + 0.5) / ppp;
    let x = |v: f32| (v * ppp).round() / ppp;
    ui.painter().line_segment([Pos2::new(x(x0), y), Pos2::new(x(x1), y)], Stroke::new(1.0 / ppp, col));
}

/// A one-device-pixel border on the inside edge of `r`.
fn outline(ui: &Ui, r: Rect, col: Color32) {
    let ppp = ui.ctx().pixels_per_point();
    ui.painter().rect_stroke(snap(ui, r), 0.0, Stroke::new(1.0 / ppp, col), StrokeKind::Inside);
}

/// `text` shortened with an ellipsis until it fits `max_w`.
fn elide(ui: &Ui, text: &str, font: &FontId, max_w: f32) -> String {
    let fits = |s: &str| ui.painter().layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE).size().x <= max_w;
    if fits(text) {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.pop().is_some() {
        let s = format!("{}\u{2026}", chars.iter().collect::<String>());
        if fits(&s) {
            return s;
        }
    }
    "\u{2026}".to_owned()
}

/// Selected row/tab fill on the Export, Open and Save As pages.
pub fn selected_row() -> Color32 {
    crate::theme::accent_tint()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    Back,
    Home,
    New,
    Open,
    Info,
    Save,
    SaveAs,
    Print,
    Share,
    Export,
    Close,
    About,
    Feedback,
    Options,
}

impl Entry {
    fn section(self) -> Option<Section> {
        Some(match self {
            Entry::Home => Section::Home,
            Entry::New => Section::New,
            Entry::Open => Section::Open,
            Entry::Info => Section::Info,
            Entry::SaveAs => Section::SaveAs,
            Entry::Print => Section::Print,
            Entry::Share => Section::Share,
            Entry::Export => Section::Export,
            Entry::About => Section::About,
            Entry::Feedback => Section::Feedback,
            Entry::Options => Section::Options,
            _ => return None,
        })
    }
}

enum Tile {
    Blank { landscape: bool },
    Bulletin,
}

fn paint_home(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.4, col);
    let c = r.center();
    let w = r.width() * 0.42;
    let h = r.height() * 0.42;
    let pt = |x: f32, y: f32| Pos2::new(c.x + x * w, c.y + y * h);
    for (a, b) in [
        (pt(-1.15, -0.1), pt(0.0, -1.1)),
        (pt(0.0, -1.1), pt(1.15, -0.1)),
        (pt(-0.8, -0.3), pt(-0.8, 1.0)),
        (pt(0.8, -0.3), pt(0.8, 1.0)),
        (pt(-0.8, 1.0), pt(0.8, 1.0)),
        (pt(-0.25, 1.0), pt(-0.25, 0.2)),
        (pt(0.25, 1.0), pt(0.25, 0.2)),
        (pt(-0.25, 0.2), pt(0.25, 0.2)),
    ] {
        p.line_segment([a, b], s);
    }
}

fn paint_back(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.4, col);
    let c = r.center();
    p.circle_stroke(c, r.width() / 2.0, s);
    p.line_segment([Pos2::new(c.x + 6.0, c.y), Pos2::new(c.x - 6.0, c.y)], s);
    p.line_segment([Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x - 1.0, c.y - 5.0)], s);
    p.line_segment([Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x - 1.0, c.y + 5.0)], s);
}

/// Test seam: record a backstage widget (label prefixes: `tile:`, `recent:`, `tabrow:`, `link:`,
/// `flat:`, `place:`, `folder:`, `nav:Back`) in the ribbon probe.
#[cfg(test)]
pub(crate) fn seen(label: String, rect: Rect, enabled: bool, selected: bool) {
    crate::widgets::probe::record(crate::widgets::probe::Seen {
        label,
        tip: String::new(),
        rect,
        enabled,
        selected,
    });
}

/// The big light page title ("Print", "Export", ...).
pub fn page_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(32.0).color(crate::theme::text()));
    ui.add_space(8.0);
}

/// The page heading of Home, New and Open ("New", "Open").
pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(22.0).color(crate::theme::text()));
}

/// A section heading on Home, New and Open ("Church", "Recent Publications").
pub fn subheading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(16.0).color(crate::theme::group_label()));
    ui.add_space(6.0);
}

/// Paper is white whatever the theme, like the page on the canvas.
const PAPER: Color32 = Color32::WHITE;
/// The grey text lines drawn on a page picture.
const PAPER_LINE: Color32 = Color32::from_rgb(0xC9, 0xCE, 0xD3);

/// A plain page outline: the stand-in for a first-page picture that is loading or unreadable.
fn outline_page_glyph(ui: &Ui, area: Rect) {
    let page = snap(ui, area);
    let p = ui.painter();
    p.rect_filled(page, 0.0, PAPER);
    p.rect_filled(
        Rect::from_min_size(page.min + Vec2::new(4.0, 5.0), Vec2::new(page.width() - 12.0, 3.0)),
        0.0,
        crate::theme::accent(),
    );
    for k in 0..3 {
        let w = if k == 2 { page.width() * 0.4 } else { page.width() - 8.0 };
        p.rect_filled(
            Rect::from_min_size(page.min + Vec2::new(4.0, 12.0 + k as f32 * 5.0), Vec2::new(w, 2.0)),
            0.0,
            PAPER_LINE,
        );
    }
    outline(ui, area, crate::theme::frame());
}

/// A green section heading ("Printer", "Settings", ...).
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(20.0).color(crate::theme::accent_text()));
    ui.add_space(4.0);
}

/// A flat bordered button in the backstage style.
pub fn flat_button(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    let resp = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).color(if enabled { crate::theme::text() } else { crate::theme::disabled() }))
            .fill(crate::theme::card())
            .stroke(Stroke::new(1.0 / ui.ctx().pixels_per_point(), crate::theme::frame()))
            .corner_radius(0.0),
    );
    #[cfg(test)]
    seen(format!("flat:{text}"), resp.rect, enabled, false);
    resp
}

/// Draws a tile's page area (the preview), given the page rectangle.
type TileArt<'a> = &'a mut dyn FnMut(&Ui, Rect);

/// Width of a start item (page picture and caption).
const TILE_W: f32 = 124.0;
/// The tallest page picture; portrait pages are 84 wide, landscape 108.
const TILE_PAGE_H: f32 = 108.0;
const TILE_PAD: f32 = 8.0;
const TILE_LABEL_H: f32 = 22.0;

/// A start item: a page picture with one thin crisp border and its caption centred below.
/// A light tint shows behind the whole item while it is hovered.
fn tile(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    landscape: bool,
    art: Option<TileArt>,
) -> bool {
    let size = Vec2::new(TILE_W, TILE_PAD + TILE_PAGE_H + 6.0 + TILE_LABEL_H + TILE_PAD);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    #[cfg(test)]
    seen(format!("tile:{label}"), rect, enabled, false);
    if resp.hovered() && enabled {
        ui.painter().rect_filled(snap(ui, rect), 0.0, crate::theme::accent_tint_hover());
    }
    let (w, h) = if landscape { (108.0, 84.0) } else { (84.0, TILE_PAGE_H) };
    // Pictures sit on a common baseline, centred across the item.
    let bottom = rect.min.y + TILE_PAD + TILE_PAGE_H;
    let page = snap(
        ui,
        Rect::from_min_size(Pos2::new(rect.center().x - w / 2.0, bottom - h), Vec2::new(w, h)),
    );
    #[cfg(test)]
    seen(format!("page:{label}"), page, enabled, false);
    // Page pictures are paper: white in both themes (a disabled item is only a faint outline).
    ui.painter().rect_filled(page, 0.0, if enabled { PAPER } else { Color32::TRANSPARENT });
    if let Some(art) = art {
        art(ui, page);
    }
    outline(ui, page, if enabled { crate::theme::frame() } else { crate::theme::line() });
    let font = FontId::proportional(13.0);
    let text = elide(ui, label, &font, TILE_W - 8.0);
    ui.painter().text(
        Pos2::new(rect.center().x, bottom + 6.0 + TILE_LABEL_H / 2.0),
        Align2::CENTER_CENTER,
        text,
        font,
        if enabled { crate::theme::text() } else { crate::theme::disabled() },
    );
    if enabled {
        resp.clicked()
    } else {
        resp.on_hover_text(COMING_SOON);
        false
    }
}

/// Draws page 1 of the Church Bulletin template inside a tile's page area.
fn bulletin_preview<'a>(
    thumbs: &'a mut crate::thumbs::StartThumbs,
    fonts: &'a crate::fontmgr::FontMgr,
) -> impl FnMut(&Ui, Rect) + 'a {
    move |ui, page| {
        let drew = thumbs.paint_bulletin(ui, page.shrink(1.0), fonts);
        #[cfg(test)]
        seen(format!("preview:{}", if drew { "Church Bulletin" } else { "none" }), page, true, false);
        let _ = drew;
    }
}

/// The blank-page tiles shared by Home and New; returns a clicked tile.
fn blank_tiles(
    ui: &mut Ui,
    bulletin: Option<(&mut crate::thumbs::StartThumbs, &crate::fontmgr::FontMgr)>,
) -> Option<Tile> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 10.0);
        if tile(ui, "Blank 8.5 x 11\"", true, false, None) {
            chosen = Some(Tile::Blank { landscape: false });
        }
        if tile(ui, "Blank 11 x 8.5\"", true, true, None) {
            chosen = Some(Tile::Blank { landscape: true });
        }
        tile(ui, "More page sizes", false, false, None);
        if let Some((thumbs, fonts)) = bulletin {
            if tile(ui, "Church Bulletin", true, false, Some(&mut bulletin_preview(thumbs, fonts))) {
                chosen = Some(Tile::Bulletin);
            }
        }
    });
    chosen
}

/// A row of tab labels with a 2 px underline under the selected one; returns the new selection.
fn tab_row(ui: &mut Ui, labels: &[(&str, bool)], selected: usize) -> usize {
    let mut sel = selected;
    let ppp = ui.ctx().pixels_per_point();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 22.0;
        for (i, (label, enabled)) in labels.iter().enumerate() {
            let text = RichText::new(*label)
                .size(15.0)
                .color(if *enabled { crate::theme::text() } else { crate::theme::disabled() });
            let sense = if *enabled { Sense::click() } else { Sense::hover() };
            let resp = ui.add(egui::Label::new(text).sense(sense));
            #[cfg(test)]
            seen(format!("tabrow:{label}"), resp.rect, *enabled, i == selected);
            if i == selected {
                let y = ((resp.rect.bottom() + 4.0) * ppp).round() / ppp;
                let x = |v: f32| (v * ppp).round() / ppp;
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x(resp.rect.left()), y),
                        Pos2::new(x(resp.rect.right()), y + 2.0 / ppp),
                    ),
                    0.0,
                    crate::theme::accent(),
                );
            }
            if *enabled && resp.clicked() {
                sel = i;
            }
        }
    });
    sel
}

/// What a rail row draws before its label. The library icons turn into solid blobs in one
/// colour when the detail is the same colour as the shape, so a few are drawn by hand.
#[derive(Clone, Copy)]
enum RailIcon {
    Home,
    Lib(Icon),
    Info,
    Question,
    Save,
}

fn paint_rail_icon(p: &egui::Painter, r: Rect, icon: RailIcon, col: Color32) {
    let s = Stroke::new(1.4, col);
    match icon {
        RailIcon::Home => paint_home(p, r, col),
        RailIcon::Lib(i) => icons::paint(p, r, i, col),
        RailIcon::Info | RailIcon::Question => {
            p.circle_stroke(r.center(), r.width() / 2.0 - 0.7, s);
            let t = if matches!(icon, RailIcon::Info) { "i" } else { "?" };
            p.text(r.center() + Vec2::new(0.0, 0.5), Align2::CENTER_CENTER, t, FontId::proportional(11.5), col);
        }
        RailIcon::Save => {
            let b = r.shrink(1.5);
            p.rect_stroke(b, 1.5, s, StrokeKind::Inside);
            let top = Rect::from_min_max(b.min + Vec2::new(4.0, 0.0), Pos2::new(b.max.x - 4.0, b.min.y + 4.5));
            p.rect_stroke(top, 0.0, s, StrokeKind::Inside);
            let bot = Rect::from_min_max(Pos2::new(b.min.x + 3.5, b.max.y - 6.5), Pos2::new(b.max.x - 3.5, b.max.y));
            p.rect_stroke(bot, 0.0, s, StrokeKind::Inside);
        }
    }
}

/// One command on the rail (`icon`: `None` for no icon).
/// The selected item is a darker teal with a 3 px white marker on its left edge.
fn rail_item(
    ui: &mut Ui,
    label: &str,
    icon: Option<RailIcon>,
    enabled: bool,
    tip: &str,
    active: bool,
) -> bool {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), RAIL_ROW), sense);
    #[cfg(test)]
    crate::widgets::probe::record(crate::widgets::probe::Seen {
        label: format!("nav:{label}"),
        tip: tip.to_owned(),
        rect,
        enabled,
        selected: active,
    });
    let ppp = ui.ctx().pixels_per_point();
    let r = snap(ui, rect);
    let p = ui.painter();
    if active {
        p.rect_filled(r, 0.0, rail_selected());
        p.rect_filled(Rect::from_min_size(r.min, Vec2::new(3.0 / ppp, r.height())), 0.0, Color32::WHITE);
    } else if resp.hovered() && enabled {
        p.rect_filled(r, 0.0, rail_hover());
    }
    let col = if enabled { rail_text() } else { rail_dim() };
    if let Some(icon) = icon {
        let ir = Rect::from_center_size(Pos2::new(rect.min.x + 26.0, rect.center().y), Vec2::splat(17.0));
        paint_rail_icon(p, ir, icon, col);
    }
    p.text(
        Pos2::new(rect.min.x + 48.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        col,
    );
    if !enabled && !tip.is_empty() {
        return resp.on_hover_text(tip).clicked();
    }
    resp.clicked()
}

fn separator(ui: &mut Ui) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, RAIL_SEP), Sense::hover());
    hline(ui, rect.min.x + 16.0, rect.max.x - 16.0, rect.center().y, rail_line());
}

impl App {
    pub fn backstage_view(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        let can_save = self.can_save();
        let save_tip = self.save_tip();
        let cur = self.section;
        let mut action = None;
        egui::Panel::left("backstage_column")
            .exact_size(RAIL_W)
            .frame(egui::Frame::new().fill(rail_fill()))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let width = ui.available_width();
                let bottom = ui.max_rect().bottom();
                ui.add_space(10.0);
                let sense = if has_doc { Sense::click() } else { Sense::hover() };
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 40.0), sense);
                #[cfg(test)]
                seen("nav:Back".to_owned(), rect, has_doc, false);
                let col = if has_doc { rail_text() } else { rail_dim() };
                paint_back(
                    ui.painter(),
                    Rect::from_center_size(Pos2::new(rect.min.x + 26.0, rect.center().y), Vec2::splat(26.0)),
                    col,
                );
                if has_doc && resp.on_hover_text("Back to Publication").clicked() {
                    action = Some(Entry::Back);
                }
                ui.add_space(8.0);
                // The wordmark in white (the dark-background variant) on the teal.
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    crate::logo::show_as(ui, crate::logo::Mark::Wordmark, crate::theme::Accent::Teal, true, 28.0);
                });
                ui.add_space(16.0);
                let need = "Open or create a publication first";
                let top = [
                    (Entry::Home, "Home", Some(RailIcon::Home), true, ""),
                    (Entry::New, "New", Some(RailIcon::Lib(Icon::New)), true, ""),
                    (Entry::Open, "Open", Some(RailIcon::Lib(Icon::Folder)), true, ""),
                ];
                for (e, label, icon, enabled, tip) in top {
                    if rail_item(ui, label, icon, enabled, tip, e.section() == Some(cur)) {
                        action = Some(e);
                    }
                }
                separator(ui);
                for (e, label, icon, enabled, tip) in [
                    (Entry::Info, "Info", RailIcon::Info, has_doc, need),
                    (Entry::Save, "Save", RailIcon::Save, can_save, save_tip),
                    (Entry::SaveAs, "Save As", RailIcon::Lib(Icon::Pages), has_doc, need),
                    (Entry::Print, "Print", RailIcon::Lib(Icon::Print), has_doc, need),
                    (Entry::Share, "Share", RailIcon::Lib(Icon::Send), true, ""),
                    (Entry::Export, "Export", RailIcon::Lib(Icon::Arrow), has_doc, need),
                    (Entry::Close, "Close", RailIcon::Lib(Icon::Close), has_doc, need),
                ] {
                    if rail_item(ui, label, Some(icon), enabled, tip, e.section() == Some(cur)) {
                        action = Some(e);
                    }
                }
                let used = ui.cursor().min.y;
                ui.add_space((bottom - used - 3.0 * RAIL_ROW - RAIL_SEP - 8.0).max(8.0));
                separator(ui);
                for (e, label, icon) in [
                    (Entry::About, "About KernPage", RailIcon::Question),
                    (Entry::Feedback, "Feedback", RailIcon::Lib(Icon::Mail)),
                    (Entry::Options, "Options", RailIcon::Lib(Icon::Gear)),
                ] {
                    if rail_item(ui, label, Some(icon), true, "", e.section() == Some(cur)) {
                        action = Some(e);
                    }
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(crate::theme::card()).inner_margin(egui::Margin {
                left: 36,
                right: 28,
                top: 14,
                bottom: 8,
            }))
            .show(ui, |ui| match self.section {
                Section::Print => self.print_page(ui),
                Section::Export => self.export_page(ui),
                Section::Home => self.home_page(ui),
                Section::New => self.new_page(ui),
                Section::Open => self.open_page(ui),
                Section::Info => self.info_page(ui),
                Section::SaveAs => self.save_as_page(ui),
                Section::Share => share_page(ui),
                Section::About => about_page(ui),
                Section::Feedback => feedback_page(ui),
                Section::Options => self.options_page(ui),
            });
        match action {
            Some(Entry::Back) => self.backstage = false,
            Some(Entry::Save) => self.save(None),
            Some(Entry::Close) => self.close_document(),
            Some(e) => {
                if let Some(s) = e.section() {
                    self.section = s;
                }
            }
            None => {}
        }
    }

    /// File > Options > Save: Publisher's "Save documents" group, with the controls
    /// KernPage supports live and the rest shown greyed.
    fn options_page(&mut self, ui: &mut Ui) {
        use kernpage_core::autorecover as ar;
        let (mut enabled, mut text) = self.options_buf.take().unwrap_or_else(|| {
            (self.ar.settings.enabled, self.ar.settings.minutes.to_string())
        });
        page_title(ui, "Options");
        let (mut ok, mut cancel) = (false, false);
        ui.horizontal_top(|ui| {
            // The category list: Save and Appearance.
            ui.vertical(|ui| {
                ui.set_width(150.0);
                for (i, name) in ["Save", "Appearance"].into_iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(150.0, 28.0), Sense::click());
                    #[cfg(test)]
                    seen(format!("optcat:{name}"), r, true, self.options_cat == i);
                    if self.options_cat == i {
                        ui.painter().rect_filled(r, 0.0, selected_row());
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 0.0, crate::theme::hover_bg());
                    }
                    ui.painter().text(
                        Pos2::new(r.left() + 10.0, r.center().y),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(15.0),
                        crate::theme::text(),
                    );
                    if resp.clicked() {
                        self.options_cat = i;
                    }
                }
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                if self.options_cat == 1 {
                    cancel = self.appearance_section(ui);
                    return;
                }
                ui.label(RichText::new("Customize how documents are saved.").size(15.0).color(crate::theme::text()));
                ui.add_space(10.0);
                section_title(ui, "Save documents");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut enabled, "Save AutoRecover information every");
                    let before = text.clone();
                    let edit = egui::Frame::new()
                        .stroke(Stroke::new(1.0, crate::theme::frame()))
                        .inner_margin(egui::Margin::symmetric(3, 1))
                        .show(ui, |ui| {
                            ui.add_enabled(
                                enabled,
                                egui::TextEdit::singleline(&mut text)
                                    .frame(egui::Frame::NONE)
                                    .desired_width(34.0)
                                    .char_limit(4),
                            )
                        })
                        .inner;
                    text = ar::accept_minutes_text(&before, &text);
                    // Spin arrows (and Up/Down while the box has focus).
                    let old = self.ar.settings.minutes;
                    let (mut up, mut down) = (false, false);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        up = spin_arrow(ui, true, enabled);
                        down = spin_arrow(ui, false, enabled);
                    });
                    if edit.has_focus() {
                        ui.input(|i| {
                            up |= i.key_pressed(egui::Key::ArrowUp);
                            down |= i.key_pressed(egui::Key::ArrowDown);
                        });
                    }
                    if up || down {
                        text = ar::step_minutes(&text, old, up).to_string();
                    }
                    ui.label(RichText::new("minutes").color(if enabled { crate::theme::text() } else { crate::theme::disabled() }));
                });
                ui.add_space(4.0);
                for label in [
                    "Allow background saves",
                    "Don't show the File screen when opening or saving files with keyboard shortcuts",
                ] {
                    let mut on = label == "Allow background saves";
                    ui.add_enabled(false, egui::Checkbox::new(&mut on, label))
                        .on_disabled_hover_text(COMING_SOON);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_enabled(false, egui::Label::new("Default personal templates location:"));
                    let mut none = String::new();
                    egui::Frame::new()
                        .stroke(Stroke::new(1.0, crate::theme::frame()))
                        .inner_margin(egui::Margin::symmetric(3, 1))
                        .show(ui, |ui| {
                            ui.add_enabled(
                                false,
                                egui::TextEdit::singleline(&mut none).frame(egui::Frame::NONE).desired_width(240.0),
                            );
                        });
                });
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    ok = flat_button(ui, "OK", true).clicked();
                    cancel = flat_button(ui, "Cancel", true).clicked();
                });
            });
        });
        if ok {
            let old = self.ar.settings;
            let minutes = ar::commit_minutes(&text, old.minutes).unwrap_or(old.minutes);
            let new = ar::Settings { enabled, minutes };
            crate::autorecover::AutoRecover::save_settings(&new);
            self.ar.apply_settings(new);
        }
        if ok || cancel {
            self.options_buf = None;
            if self.editor.is_some() {
                self.backstage = false;
            } else {
                self.section = Section::Home;
            }
        } else {
            self.options_buf = Some((enabled, text));
        }
    }

    /// File > Options > Appearance: theme and accent, applied (and saved) at once.
    /// Returns true when Done was pressed.
    fn appearance_section(&mut self, ui: &mut Ui) -> bool {
        use crate::theme::{Accent, ThemeMode};
        let mut a = self.appearance;
        ui.label(RichText::new("Personalize how KernPage looks.").size(15.0).color(crate::theme::text()));
        ui.add_space(10.0);
        section_title(ui, "Theme");
        ui.horizontal(|ui| {
            for m in ThemeMode::ALL {
                if ui.radio(a.mode == m, m.label()).clicked() {
                    a.mode = m;
                }
                ui.add_space(10.0);
            }
        });
        ui.add_space(8.0);
        section_title(ui, "Accent");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            for x in Accent::ALL {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Accent {}", x.label())));
                #[cfg(test)]
                seen(format!("accent:{}", x.label()), r, true, a.accent == x);
                let p = ui.painter();
                p.circle_filled(r.center(), 12.0, x.base());
                if a.accent == x {
                    p.circle_stroke(r.center(), 14.5, Stroke::new(2.0, crate::theme::text()));
                } else if resp.hovered() {
                    p.circle_stroke(r.center(), 14.0, Stroke::new(1.0, crate::theme::frame()));
                }
                if resp.on_hover_text(x.label()).clicked() {
                    a.accent = x;
                }
            }
        });
        if a != self.appearance {
            let ctx = ui.ctx().clone();
            self.set_appearance(a, &ctx);
        }
        ui.add_space(24.0);
        flat_button(ui, "Done", true).clicked()
    }

    fn start_tile(&mut self, tile: Tile) {
        let doc = match tile {
            Tile::Blank { landscape: false } => template::blank_portrait(),
            Tile::Blank { landscape: true } => template::blank_landscape(),
            Tile::Bulletin => template::church_bulletin(),
        };
        self.request(Pending::New(Box::new(doc)));
    }

    fn home_page(&mut self, ui: &mut Ui) {
        let mut picked = None;
        let mut opened = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            heading(ui, "New");
            ui.add_space(8.0);
            picked = blank_tiles(ui, Some((&mut self.start_thumbs, &self.fonts)));
            ui.add_space(2.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if link_as(ui, "More templates", "More templates \u{2192}") {
                    self.section = Section::New;
                }
            });
            ui.add_space(14.0);
            self.home_tab = tab_row(ui, &[("Recent", true), ("Pinned", true)], self.home_tab.min(1));
            ui.add_space(8.0);
            if self.home_tab == 0 {
                opened = self.recent_list(ui, 8);
                ui.add_space(8.0);
                if green_link(ui, "More publications") {
                    self.section = Section::Open;
                }
            } else {
                ui.label(RichText::new("Pin a publication to keep it here.").color(crate::theme::disabled()));
            }
        });
        if let Some(p) = opened {
            self.request(Pending::OpenPath(p));
        }
        if let Some(t) = picked {
            self.start_tile(t);
        }
    }

    /// The recent publications as a full-width list: a small picture of the first page, the
    /// name with its folder under it, and when it was opened. Returns a clicked path.
    /// Files are parsed on a background thread (see `thumbs`); while one loads, or if it cannot
    /// be read, the row shows a plain page outline.
    fn recent_list(&mut self, ui: &mut Ui, limit: usize) -> Option<PathBuf> {
        let items: Vec<PathBuf> = self.recent.items().to_vec();
        if items.is_empty() {
            ui.label(
                RichText::new("No recent publications yet. Publications you open or save appear here.")
                    .color(crate::theme::disabled()),
            );
            return None;
        }
        const ROW_H: f32 = 52.0;
        const TEXT_X: f32 = 56.0;
        const DATE_W: f32 = 150.0;
        let w = ui.available_width();
        let (head, _) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::hover());
        let p = ui.painter();
        p.text(
            Pos2::new(head.min.x + TEXT_X, head.center().y),
            Align2::LEFT_CENTER,
            "Name",
            FontId::proportional(12.0),
            crate::theme::group_label(),
        );
        p.text(
            Pos2::new(head.max.x - 12.0, head.center().y),
            Align2::RIGHT_CENTER,
            "Opened",
            FontId::proportional(12.0),
            crate::theme::group_label(),
        );
        hline(ui, head.min.x, head.max.x, head.max.y - 1.0, crate::theme::card_line());
        let mut hit = None;
        for path in items.iter().take(limit) {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, ROW_H), Sense::click());
            let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            #[cfg(test)]
            seen(format!("recent:{name}"), rect, true, false);
            if resp.hovered() {
                ui.painter().rect_filled(snap(ui, rect), 0.0, crate::theme::accent_tint_hover());
            }
            // The first page when it is ready; otherwise (loading, missing, unreadable) an outline.
            let area = Rect::from_min_size(rect.min + Vec2::new(14.0, (ROW_H - 34.0) / 2.0), Vec2::new(26.0, 34.0));
            let drew = self.start_thumbs.paint_file(ui, area, path, &self.fonts);
            #[cfg(test)]
            seen(format!("{}:{name}", if drew { "preview" } else { "glyph" }), area, true, false);
            if !drew {
                outline_page_glyph(ui, area);
            }
            let max_w = (w - TEXT_X - DATE_W - 12.0).max(40.0);
            let name_font = FontId::proportional(14.0);
            let crumb_font = FontId::proportional(12.0);
            let crumb = path.parent().map(recent::breadcrumb).unwrap_or_default();
            let (shown, shown_crumb) = (elide(ui, &name, &name_font, max_w), elide(ui, &crumb, &crumb_font, max_w));
            #[cfg(test)]
            {
                let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                let text_w = |t: &str, f: &FontId| ui.painter().layout_no_wrap(t.to_owned(), f.clone(), Color32::WHITE).size().x;
                let tip = |label: String, tip: String, r: Rect| {
                    crate::widgets::probe::record(crate::widgets::probe::Seen { label, tip, rect: r, enabled: true, selected: false });
                };
                tip(
                    format!("rname:{name}"),
                    shown.clone(),
                    at(rect.min.x + TEXT_X, rect.min.y + 10.0, text_w(&shown, &name_font), 17.0),
                );
                tip(
                    format!("rfolder:{name}"),
                    shown_crumb.clone(),
                    at(rect.min.x + TEXT_X, rect.min.y + 30.0, text_w(&shown_crumb, &crumb_font), 14.0),
                );
                if let Some(d) = opened_text(&self.recent, path) {
                    let f = FontId::proportional(12.5);
                    let wd = text_w(&d, &f);
                    tip(format!("ropened:{name}"), d, at(rect.max.x - 12.0 - wd, rect.center().y - 8.0, wd, 16.0));
                }
            }
            let p = ui.painter();
            p.text(
                Pos2::new(rect.min.x + TEXT_X, rect.min.y + 10.0),
                Align2::LEFT_TOP,
                shown,
                name_font,
                crate::theme::text(),
            );
            p.text(
                Pos2::new(rect.min.x + TEXT_X, rect.min.y + 30.0),
                Align2::LEFT_TOP,
                shown_crumb,
                crumb_font,
                crate::theme::group_label(),
            );
            if let Some(date) = opened_text(&self.recent, path) {
                let date = elide(ui, &date, &FontId::proportional(12.5), DATE_W - 12.0);
                ui.painter().text(
                    Pos2::new(rect.max.x - 12.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    date,
                    FontId::proportional(12.5),
                    crate::theme::group_label(),
                );
            }
            hline(ui, rect.min.x, rect.max.x, rect.max.y - 1.0, crate::theme::line());
            let resp = resp.on_hover_text(path.display().to_string());
            if resp.clicked() {
                hit = Some(path.clone());
            }
        }
        hit
    }

    fn new_page(&mut self, ui: &mut Ui) {
        heading(ui, "New");
        ui.add_space(8.0);
        let mut chosen = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            chosen = blank_tiles(ui, None);
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            self.bs_tab = tab_row(ui, &[("Online", true), ("Built-in", true)], self.bs_tab.min(1));
            ui.add_space(12.0);
            if self.bs_tab == 0 {
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut String::new())
                        .hint_text("Coming soon: search free KernPage templates")
                        .desired_width(760.0)
                        .margin(egui::Margin::symmetric(8, 6)),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Suggested searches:");
                    for s in ["Personal", "Design Sets", "Labels", "Event", "Paper", "Business", "Cards"] {
                        ui.label(RichText::new(s).color(crate::theme::disabled()));
                    }
                });
                ui.add_space(14.0);
                ui.label(
                    RichText::new(
                        "Coming soon: browse and download free KernPage templates. \
                         Built-in templates are on the Built-in tab.",
                    )
                    .color(crate::theme::text()),
                );
            } else {
                subheading(ui, "Church");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 10.0);
                    if tile(
                        ui,
                        "Church Bulletin",
                        true,
                        false,
                        Some(&mut bulletin_preview(&mut self.start_thumbs, &self.fonts)),
                    ) {
                        chosen = Some(Tile::Bulletin);
                    }
                    tile(ui, "Order of Service", false, false, None);
                    tile(ui, "Prayer List", false, false, None);
                });
                ui.add_space(10.0);
                subheading(ui, "More Templates");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 6.0);
                    for name in [
                        "Brochures", "Business Cards", "Calendars", "Greeting Cards", "Labels",
                        "Newsletters", "Postcards", "Advertisements", "Award Certificates", "Banners",
                        "Business Forms", "Catalogs", "Envelopes", "Flyers", "Invitation Cards",
                        "Letterhead", "Menus", "Programs", "Resumes", "Signs",
                    ] {
                        flat_button(ui, name, false).on_disabled_hover_text(COMING_SOON);
                    }
                });
            }
        });
        if let Some(t) = chosen {
            self.start_tile(t);
        }
    }

    /// A places column (selected place plus Browse) used by Open and Save As: icon and label rows.
    fn places(ui: &mut Ui, labels: &[&str]) -> bool {
        let mut browse = false;
        ui.allocate_ui(Vec2::new(230.0, 400.0), |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, label) in labels.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 38.0), Sense::hover());
                    #[cfg(test)]
                    seen(format!("place:{label}"), rect, true, i == 0);
                    let ppp = ui.ctx().pixels_per_point();
                    let r = snap(ui, rect);
                    let col = if i == 0 { crate::theme::accent_text() } else { crate::theme::text() };
                    if i == 0 {
                        ui.painter().rect_filled(r, 0.0, selected_row());
                        ui.painter().rect_filled(
                            Rect::from_min_size(r.min, Vec2::new(3.0 / ppp, r.height())),
                            0.0,
                            crate::theme::accent_text(),
                        );
                    }
                    let icon = if *label == "Recent" { Icon::Clock } else { Icon::Folder };
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(Pos2::new(rect.min.x + 24.0, rect.center().y), Vec2::splat(17.0)),
                        icon,
                        col,
                    );
                    ui.painter().text(
                        Pos2::new(rect.min.x + 46.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        FontId::proportional(14.5),
                        crate::theme::text(),
                    );
                }
                ui.add_space(8.0);
                if flat_button(ui, "Browse", true).clicked() {
                    browse = true;
                }
            });
        });
        browse
    }

    fn open_page(&mut self, ui: &mut Ui) {
        heading(ui, "Open");
        ui.add_space(8.0);
        let mut browse = false;
        let mut hit = None;
        ui.horizontal_top(|ui| {
            browse = Self::places(ui, &["Recent", "This PC"]);
            ui.add_space(16.0);
            ui.vertical(|ui| {
                subheading(ui, "Recent Publications");
                tab_row(ui, &[("Publications", true), ("Folders", false)], 0);
                ui.add_space(8.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    hit = self.recent_list(ui, 20);
                });
            });
        });
        if browse {
            self.open_dialog();
        }
        if let Some(p) = hit {
            self.request(Pending::OpenPath(p));
        }
    }

    fn info_page(&mut self, ui: &mut Ui) {
        page_title(ui, "Info");
        let Some(ed) = self.editor.as_ref() else {
            ui.label(RichText::new("No publication is open.").color(crate::theme::text()));
            return;
        };
        let name = ed.doc().name.trim_end_matches(".pub").trim_end_matches(".kern").to_owned();
        let pages = ed.doc().pages.len();
        let size = ed.doc().pages.first().map(|p| (p.width_emu, p.height_emu));
        // Demo mode shows a made-up folder instead of the real one.
        let real = self.path.clone();
        let path = real.as_deref().map(|p| if crate::demo::is_on() { crate::demo::shown_path(p) } else { p.to_path_buf() });
        ui.label(RichText::new(name).size(24.0).color(crate::theme::accent_text()));
        match path.as_deref().and_then(Path::parent) {
            Some(dir) => ui.label(RichText::new(recent::breadcrumb(dir)).color(crate::theme::text())),
            None => ui.label(RichText::new("Not saved yet").color(crate::theme::text())),
        };
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if flat_button(ui, "Copy path", path.is_some()).clicked() {
                if let Some(p) = &path {
                    ui.ctx().copy_text(p.to_string_lossy().into_owned());
                }
            }
            if flat_button(ui, "Open file location", path.is_some() && !crate::demo::is_on()).clicked() {
                if let Some(d) = path.as_deref().and_then(Path::parent) {
                    let _ = crate::output::open_with_system(d);
                }
            }
        });
        ui.add_space(14.0);
        let meta = real.as_ref().and_then(|p| std::fs::metadata(p).ok());
        let mut rows = vec![("Pages", pages.to_string())];
        if let Some((w, h)) = size {
            rows.push((
                "Page Size",
                format!("{} x {}\"", trim(w as f64 / 914_400.0), trim(h as f64 / 914_400.0)),
            ));
        }
        if let Some(m) = &meta {
            rows.push(("Size", format!("{:.1} KB", m.len() as f64 / 1024.0)));
            if crate::demo::is_on() {
                rows.push(("Last Saved", crate::demo::OPEN_DATE.to_owned()));
            } else if let Ok(t) = m.modified() {
                rows.push(("Last Saved", recent::date_text(t)));
            }
        }
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(700.0, 400.0), |ui| {
                ui.vertical(|ui| {
                    for (title, desc, button) in [
                        ("Contact Details", "Saved sets of contact details are not supported yet.", "Edit Contact\nDetails"),
                        ("Check Publication", "Look for layout problems before you print or share.", "Check\nPublication"),
                        ("Embedded Font Information", "Fonts are embedded automatically when you create a PDF.", "Fonts in\nThis File"),
                    ] {
                        ui.horizontal_top(|ui| {
                            ui.add_enabled_ui(false, |ui| {
                                ui.add_sized(
                                    [124.0, 70.0],
                                    egui::Button::new(RichText::new(button).size(12.5))
                                        .fill(crate::theme::card())
                                        .stroke(Stroke::new(1.0, crate::theme::frame()))
                                        .corner_radius(0.0),
                                )
                            })
                            .response
                            .on_disabled_hover_text(COMING_SOON);
                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(title).size(20.0).color(crate::theme::text()));
                                ui.label(RichText::new(desc).color(crate::theme::text()));
                            });
                        });
                        ui.add_space(22.0);
                    }
                });
            });
            ui.vertical(|ui| {
                ui.label(RichText::new("Publication Properties").size(15.0).color(crate::theme::text()));
                ui.add_space(4.0);
                egui::Grid::new("info_props").spacing(Vec2::new(24.0, 6.0)).show(ui, |ui| {
                    for (k, v) in &rows {
                        ui.label(RichText::new(*k).color(crate::theme::text()));
                        ui.label(RichText::new(v).color(crate::theme::text()));
                        ui.end_row();
                    }
                });
            });
        });
    }

    fn save_as_page(&mut self, ui: &mut Ui) {
        page_title(ui, "Save As");
        let mut browse = false;
        ui.horizontal_top(|ui| {
            browse = Self::places(ui, &["This PC"]);
            ui.add_space(16.0);
            ui.vertical(|ui| {
                section_title(ui, "Recent Folders");
                let folders = self.recent.folders();
                if folders.is_empty() {
                    ui.label(RichText::new("Folders of publications you open or save appear here.").color(crate::theme::disabled()));
                }
                for d in folders.iter().take(10) {
                    let name = d
                        .file_name()
                        .map_or_else(|| recent::breadcrumb(d), |n| n.to_string_lossy().into_owned());
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(560.0, 46.0), Sense::click());
                    #[cfg(test)]
                    seen(format!("folder:{name}"), rect, true, false);
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 0.0, crate::theme::hover_bg());
                    }
                    ui.painter().text(rect.min + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, name, FontId::proportional(14.0), crate::theme::text());
                    ui.painter().text(rect.min + Vec2::new(10.0, 27.0), Align2::LEFT_TOP, recent::breadcrumb(d), FontId::proportional(11.5), crate::theme::group_label());
                    if resp.clicked() {
                        browse = true;
                    }
                }
            });
        });
        if browse {
            self.save_as(None);
        }
    }
}

/// The label under a Recent card: when KernPage last opened or saved the file ("Opened today
/// 10:24"). Entries without a stored time show nothing, so a date is always the opened/saved
/// date. In demo mode the made-up date is shown.
fn opened_text(list: &recent::Recent, path: &Path) -> Option<String> {
    if crate::demo::is_on() {
        return crate::demo::date_of(path).map(demo_date);
    }
    list.opened_at(path).map(|at| column_label(&recent::opened_label_now(at)))
}

/// The Opened column already has the heading "Opened": "Opened today 10:24" -> "Today 10:24".
pub(crate) fn column_label(label: &str) -> String {
    let rest = label.strip_prefix("Opened ").unwrap_or(label);
    let mut c = rest.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Demo dates are stored as "2026-11-24"; show them the way real dates look ("24 Nov 2026").
fn demo_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let p: Vec<&str> = iso.split('-').collect();
    match (p.first(), p.get(1).and_then(|m| m.parse::<usize>().ok()), p.get(2).and_then(|d| d.parse::<u32>().ok())) {
        (Some(y), Some(m @ 1..=12), Some(d)) => format!("{d} {} {y}", MONTHS[m - 1]),
        _ => iso.to_owned(),
    }
}

#[cfg(test)]
mod column_label_tests {
    #[test]
    fn the_opened_column_drops_the_repeated_word() {
        assert_eq!(super::column_label("Opened today 10:24"), "Today 10:24");
        assert_eq!(super::column_label("Opened yesterday"), "Yesterday");
        assert_eq!(super::column_label("Opened 23 Nov 2026"), "23 Nov 2026");
        assert_eq!(super::demo_date("2026-11-09"), "9 Nov 2026");
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// One small up or down arrow button of the minutes box.
fn spin_arrow(ui: &mut Ui, up: bool, enabled: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(
        Vec2::new(18.0, 11.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    #[cfg(test)]
    spin_probe::record(up, r);
    let fill = if enabled && resp.hovered() { crate::theme::hover_bg() } else { crate::theme::card() };
    ui.painter().rect(r, 0.0, fill, Stroke::new(1.0, crate::theme::frame()), StrokeKind::Inside);
    let c = r.center();
    let d = if up { -1.5 } else { 1.5 };
    let col = if enabled { crate::theme::text() } else { crate::theme::disabled() };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(c.x - 3.5, c.y - d),
            Pos2::new(c.x + 3.5, c.y - d),
            Pos2::new(c.x, c.y + d),
        ],
        col,
        Stroke::NONE,
    ));
    resp.clicked()
}

/// File > Share: the planned sharing features, listed greyed until they exist.
fn share_page(ui: &mut Ui) {
    page_title(ui, "Share");
    ui.label(RichText::new("Coming soon").color(crate::theme::text()).size(14.0));
    ui.add_space(8.0);
    for item in [
        "Share a PDF of this publication with other apps on your PC (email, chat, nearby devices).",
        "Email as PDF with your email app.",
        "Copy as PDF.",
    ] {
        ui.add_enabled(false, egui::Label::new(item));
        ui.add_space(4.0);
    }
}

/// The installed third-party licence list or notices, if it sits next to `exe_dir`'s exe.
pub(crate) fn notices_in(dir: &Path) -> Option<PathBuf> {
    ["THIRD_PARTY_LICENSES.html", "THIRD_PARTY_NOTICES.txt"]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
}

/// File > About KernPage: version, licence, privacy, links and the independence line.
fn about_page(ui: &mut Ui) {
    let text = |ui: &mut Ui, t: &str| {
        ui.label(RichText::new(t).color(crate::theme::text()).size(14.0));
    };
    page_title(ui, "About KernPage");
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        crate::logo::show(ui, crate::logo::Mark::Wordmark, 40.0);
        ui.add_space(6.0);
        ui.label(RichText::new(format!("KernPage {}", crate::app_version())).size(18.0).color(crate::theme::text()));
        ui.add_space(4.0);
        text(ui, "A free desktop publishing app for Windows. Opens and saves KernPage (.kern) and Microsoft Publisher (.pub) files.");
        ui.add_space(14.0);
        section_title(ui, "Licence");
        text(ui, "Free and open source under the GNU General Public License, version 3 (GPL-3.0).");
        let notices = std::env::current_exe().ok().and_then(|e| notices_in(e.parent()?));
        if let Some(path) = notices {
            ui.add_space(4.0);
            if green_link(ui, "Third-party notices") {
                crate::feedback::open_url(&path.to_string_lossy());
            }
        }
        ui.add_space(14.0);
        section_title(ui, "Privacy");
        text(ui, "Nothing is sent from KernPage. Update checks are made only by the separate KernPage Launcher, and only if you turn them on or press Check.");
        ui.add_space(14.0);
        section_title(ui, "Links");
        for (label, url) in [
            ("Website: https://238apps.com/kernpage", "https://238apps.com/kernpage"),
            ("Source code: https://github.com/jmitchell238/KernPage", crate::feedback::REPO_URL),
            ("Report a problem: https://github.com/jmitchell238/KernPage/issues", "https://github.com/jmitchell238/KernPage/issues"),
        ] {
            if green_link(ui, label) {
                crate::feedback::open_url(url);
            }
            ui.add_space(2.0);
        }
        ui.add_space(14.0);
        section_title(ui, "Support KernPage");
        text(ui, "KernPage is free and always will be. If it helps you, you can say thanks with a small donation. The link opens in your web browser.");
        ui.add_space(4.0);
        if green_link(ui, "Buy me a RedBull: https://buymeacoffee.com/238apps") {
            crate::feedback::open_url(crate::feedback::DONATE_URL);
        }
        ui.add_space(14.0);
        text(ui, "KernPage is an independent project. It is not affiliated with, endorsed by or sponsored by Microsoft.");
    });
}

/// File > Feedback: the same two actions as the Help tab (there is no live support).
fn feedback_page(ui: &mut Ui) {
    page_title(ui, "Feedback");
    ui.label(
        RichText::new("Found a problem or have an idea? Each link opens a new GitHub issue in your web browser. Nothing is sent from the editor; you read it and submit it there.")
            .color(crate::theme::text())
            .size(14.0),
    );
    ui.add_space(10.0);
    if green_link(ui, "Report a Problem") {
        crate::feedback::report_a_problem();
    }
    ui.add_space(6.0);
    if green_link(ui, "Suggest an Idea") {
        crate::feedback::suggest_an_idea();
    }
}

pub(crate) fn green_link(ui: &mut Ui, text: &str) -> bool {
    link_as(ui, text, text)
}

/// An accent link that shows `shown` and is recorded (in tests) as `link:{key}`.
fn link_as(ui: &mut Ui, key: &str, shown: &str) -> bool {
    let resp = ui.add(egui::Label::new(RichText::new(shown).color(crate::theme::accent_text()).underline()).sense(Sense::click()));
    #[cfg(test)]
    seen(format!("link:{key}"), resp.rect, true, false);
    #[cfg(not(test))]
    let _ = key;
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Test seam: where the minutes box's spin arrows were drawn in the last frame.
#[cfg(test)]
pub mod spin_probe {
    use eframe::egui::Rect;
    use std::cell::Cell;
    thread_local! {
        static ARROWS: Cell<(Option<Rect>, Option<Rect>)> = const { Cell::new((None, None)) };
    }
    pub fn record(up: bool, r: Rect) {
        ARROWS.with(|a| {
            let (u, d) = a.get();
            a.set(if up { (Some(r), d) } else { (u, Some(r)) });
        });
    }
    pub fn clear() {
        ARROWS.with(|a| a.set((None, None)));
    }
    /// (up arrow, down arrow)
    pub fn get() -> (Option<Rect>, Option<Rect>) {
        ARROWS.with(Cell::get)
    }
}
