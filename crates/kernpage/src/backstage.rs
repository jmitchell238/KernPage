//! The File screen in the Publisher 2021 layout: a green command rail on the left
//! and one page per command (Home, New, Open, Info, Save As, Print, Share, Export, ...).

use crate::app::{App, Pending, Section};
use crate::icons::{self, Icon};
use crate::recent;
use crate::widgets::COMING_SOON;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use kernpage_core::template;
use std::path::{Path, PathBuf};

/// The command rail: neutral, on the window background.
fn rail_fill() -> Color32 {
    crate::theme::window_bg()
}
fn rail_text() -> Color32 {
    crate::theme::text()
}
fn rail_dim() -> Color32 {
    crate::theme::line()
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
            .stroke(Stroke::new(1.0, crate::theme::frame()))
            .corner_radius(0.0),
    );
    #[cfg(test)]
    seen(format!("flat:{text}"), resp.rect, enabled, false);
    resp
}

/// Draws a tile's page area (the preview), given the page rectangle.
type TileArt<'a> = &'a mut dyn FnMut(&Ui, Rect);

/// A page-sized tile: a bordered box with a page outline above a caption.
fn tile(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    landscape: bool,
    art: Option<TileArt>,
) -> bool {
    let size = Vec2::new(150.0, 176.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    #[cfg(test)]
    seen(format!("tile:{label}"), rect, enabled, false);
    let p = ui.painter();
    let frame = Rect::from_min_size(rect.min, Vec2::new(150.0, 150.0));
    // A rounded card holding both the page drawing and its caption.
    p.rect_filled(rect, 12.0, crate::theme::tile());
    p.rect_stroke(rect, 12.0, Stroke::new(1.0, crate::theme::tile_line()), StrokeKind::Inside);
    if resp.hovered() && enabled {
        p.rect_stroke(rect, 12.0, Stroke::new(2.0, crate::theme::accent()), StrokeKind::Inside);
    }
    let ink = if enabled { crate::theme::group_label() } else { crate::theme::frame() };
    let (w, h) = if landscape { (116.0, 90.0) } else { (92.0, 116.0) };
    let page = Rect::from_center_size(frame.center(), Vec2::new(w, h));
    p.rect_filled(page, 2.0, crate::theme::card());
    let fold = 20.0;
    let s = Stroke::new(1.0, ink);
    let (tl, tr, br, bl) = (page.left_top(), page.right_top(), page.right_bottom(), page.left_bottom());
    let (a, b, c) = (tr - Vec2::new(fold, 0.0), tr + Vec2::new(0.0, fold), tr + Vec2::new(-fold, fold));
    for (p1, p2) in [(tl, a), (a, b), (a, c), (c, b), (b, br), (br, bl), (bl, tl)] {
        p.line_segment([p1, p2], s);
    }
    if let Some(art) = art {
        art(ui, page);
    }
    p.text(
        Pos2::new(rect.center().x, rect.max.y - 8.0),
        Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(13.5),
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
        ui.spacing_mut().item_spacing = Vec2::new(16.0, 10.0);
        if tile(ui, "Blank 8.5 x 11\"", true, false, None) {
            chosen = Some(Tile::Blank { landscape: false });
        }
        if tile(ui, "Blank 11 x 8.5\"", true, true, None) {
            chosen = Some(Tile::Blank { landscape: true });
        }
        tile(ui, "More Blank Page Sizes", false, false, None);
        if let Some((thumbs, fonts)) = bulletin {
            if tile(ui, "Church Bulletin", true, false, Some(&mut bulletin_preview(thumbs, fonts))) {
                chosen = Some(Tile::Bulletin);
            }
        }
    });
    chosen
}

/// A row of tab labels with an underline under the selected one; returns the new selection.
fn tab_row(ui: &mut Ui, labels: &[(&str, bool)], selected: usize) -> usize {
    let mut sel = selected;
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
                let y = resp.rect.bottom() + 3.0;
                ui.painter().line_segment(
                    [Pos2::new(resp.rect.left(), y), Pos2::new(resp.rect.right(), y)],
                    Stroke::new(2.0, crate::theme::accent()),
                );
            }
            if *enabled && resp.clicked() {
                sel = i;
            }
        }
    });
    sel
}

/// One command on the rail. `icon`: `None` no icon, `Some(None)` the house, `Some(Some(i))` an icon.
fn rail_item(
    ui: &mut Ui,
    label: &str,
    icon: Option<Option<Icon>>,
    enabled: bool,
    tip: &str,
    active: bool,
) -> bool {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), sense);
    #[cfg(test)]
    crate::widgets::probe::record(crate::widgets::probe::Seen {
        label: format!("nav:{label}"),
        tip: tip.to_owned(),
        rect,
        enabled,
        selected: active,
    });
    let p = ui.painter();
    let pill = rect.shrink2(Vec2::new(8.0, 2.0));
    if active {
        p.rect_filled(pill, 6.0, crate::theme::accent_tint());
    } else if resp.hovered() && enabled {
        p.rect_filled(pill, 6.0, crate::theme::hover_bg());
    }
    let col = if !enabled {
        rail_dim()
    } else if active {
        crate::theme::accent_text()
    } else {
        crate::theme::text()
    };
    if let Some(icon) = icon {
        let r = Rect::from_center_size(Pos2::new(rect.min.x + 28.0, rect.center().y), Vec2::splat(17.0));
        match icon {
            Some(i) => icons::paint(p, r, i, col),
            None => paint_home(p, r, col),
        }
    }
    p.text(
        Pos2::new(rect.min.x + 43.0, rect.center().y),
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
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 14.0), Sense::hover());
    let y = rect.center().y;
    ui.painter().line_segment(
        [Pos2::new(rect.min.x + 25.0, y), Pos2::new(rect.max.x - 25.0, y)],
        Stroke::new(1.0, rail_dim()),
    );
}

impl App {
    pub fn backstage_view(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        let can_save = self.can_save();
        let save_tip = self.save_tip();
        let cur = self.section;
        let mut action = None;
        egui::Panel::left("backstage_column")
            .exact_size(175.0)
            .frame(egui::Frame::new().fill(rail_fill()))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let width = ui.available_width();
                let bottom = ui.max_rect().bottom();
                ui.add_space(14.0);
                let sense = if has_doc { Sense::click() } else { Sense::hover() };
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 40.0), sense);
                #[cfg(test)]
                seen("nav:Back".to_owned(), rect, has_doc, false);
                let col = if has_doc { rail_text() } else { rail_dim() };
                paint_back(
                    ui.painter(),
                    Rect::from_center_size(Pos2::new(rect.min.x + 28.0, rect.center().y), Vec2::splat(26.0)),
                    col,
                );
                if has_doc && resp.on_hover_text("Back to Publication").clicked() {
                    action = Some(Entry::Back);
                }
                ui.add_space(8.0);
                let need = "Open or create a publication first";
                let top = [
                    (Entry::Home, "Home", Some(None), true, ""),
                    (Entry::New, "New", Some(Some(Icon::Page)), true, ""),
                    (Entry::Open, "Open", Some(Some(Icon::Folder)), true, ""),
                ];
                for (e, label, icon, enabled, tip) in top {
                    if rail_item(ui, label, icon, enabled, tip, e.section() == Some(cur)) {
                        action = Some(e);
                    }
                }
                separator(ui);
                for (e, label, enabled, tip) in [
                    (Entry::Info, "Info", has_doc, need),
                    (Entry::Save, "Save", can_save, save_tip),
                    (Entry::SaveAs, "Save As", has_doc, need),
                    (Entry::Print, "Print", has_doc, need),
                    (Entry::Share, "Share", true, ""),
                    (Entry::Export, "Export", has_doc, need),
                    (Entry::Close, "Close", has_doc, need),
                ] {
                    if rail_item(ui, label, None, enabled, tip, e.section() == Some(cur)) {
                        action = Some(e);
                    }
                }
                let used = ui.cursor().min.y;
                ui.add_space((bottom - used - 3.0 * 34.0 - 28.0).max(8.0));
                separator(ui);
                for (e, label) in [
                    (Entry::About, "About KernPage"),
                    (Entry::Feedback, "Feedback"),
                    (Entry::Options, "Options"),
                ] {
                    if rail_item(ui, label, None, true, "", e.section() == Some(cur)) {
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
        crate::logo::show(ui, crate::logo::Mark::Wordmark, 34.0);
        ui.add_space(10.0);
        let mut picked = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Start screen: recent publications first, as cards.
            section_title(ui, "Open");
            tab_row(ui, &[("Recent", true), ("Pinned", false)], 0);
            ui.add_space(8.0);
            if let Some(p) = self.recent_cards(ui, 8) {
                self.request(Pending::OpenPath(p));
            }
            ui.add_space(6.0);
            if green_link(ui, "More publications") {
                self.section = Section::Open;
            }
            ui.add_space(18.0);
            section_title(ui, "New");
            picked = blank_tiles(ui, Some((&mut self.start_thumbs, &self.fonts)));
            ui.add_space(6.0);
            if green_link(ui, "More templates") {
                self.section = Section::New;
            }
        });
        if let Some(t) = picked {
            self.start_tile(t);
        }
    }

    /// The recent publications as a wrapping row of cards (Modern Home). Returns a clicked path.
    /// Each card shows a miniature of the file's first page. Files are parsed on a background
    /// thread (see `thumbs`); while one loads, or if it cannot be read, the card shows a page glyph.
    fn recent_cards(&mut self, ui: &mut Ui, limit: usize) -> Option<PathBuf> {
        let items: Vec<PathBuf> = self.recent.items().to_vec();
        if items.is_empty() {
            ui.label(
                RichText::new("No recent publications yet. Publications you open or save appear here.")
                    .color(crate::theme::disabled()),
            );
            return None;
        }
        let mut hit = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
            for path in items.iter().take(limit) {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(150.0, 168.0), Sense::click());
                let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                #[cfg(test)]
                seen(format!("recent:{name}"), rect, true, false);
                let p = ui.painter();
                p.rect_filled(rect, 12.0, crate::theme::tile());
                let edge = if resp.hovered() {
                    Stroke::new(2.0, crate::theme::accent())
                } else {
                    Stroke::new(1.0, crate::theme::tile_line())
                };
                p.rect_stroke(rect, 12.0, edge, StrokeKind::Inside);
                // The first page when it is ready; otherwise (loading, missing, unreadable) the glyph.
                let area = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.min.y + 60.0),
                    Vec2::new(120.0, 104.0),
                );
                let drew = self.start_thumbs.paint_file(ui, area, path, &self.fonts);
                #[cfg(test)]
                seen(format!("{}:{name}", if drew { "preview" } else { "glyph" }), area, true, false);
                let p = ui.painter();
                if !drew {
                    let page = Rect::from_center_size(
                        Pos2::new(rect.center().x, rect.min.y + 60.0),
                        Vec2::new(66.0, 84.0),
                    );
                    p.rect_filled(page, 3.0, crate::theme::card());
                    p.rect_stroke(page, 3.0, Stroke::new(1.0, crate::theme::line()), StrokeKind::Inside);
                    for k in 0..5 {
                        let y = page.min.y + 26.0 + k as f32 * 9.0;
                        let w = if k == 4 { 28.0 } else { 46.0 };
                        p.rect_filled(
                            Rect::from_min_size(Pos2::new(page.min.x + 10.0, y), Vec2::new(w, 3.0)),
                            1.5,
                            crate::theme::line(),
                        );
                    }
                    p.rect_filled(
                        Rect::from_min_size(page.min + Vec2::new(10.0, 10.0), Vec2::new(46.0, 8.0)),
                        2.0,
                        crate::theme::accent(),
                    );
                }
                let short = if name.chars().count() > 20 {
                    format!("{}…", name.chars().take(19).collect::<String>())
                } else {
                    name.clone()
                };
                p.text(
                    Pos2::new(rect.min.x + 12.0, rect.max.y - 38.0),
                    Align2::LEFT_TOP,
                    short,
                    FontId::proportional(13.5),
                    crate::theme::text(),
                );
                if let Some(date) = modified_text(path) {
                    p.text(
                        Pos2::new(rect.min.x + 12.0, rect.max.y - 20.0),
                        Align2::LEFT_TOP,
                        date,
                        FontId::proportional(11.5),
                        crate::theme::group_label(),
                    );
                }
                let resp = resp.on_hover_text(path.display().to_string());
                if resp.clicked() {
                    hit = Some(path.clone());
                }
            }
        });
        hit
    }

    /// The recent publications as rows (name and folder, date modified). Returns a clicked path.
    fn recent_list(&self, ui: &mut Ui, limit: usize) -> Option<PathBuf> {
        let items = self.recent.items();
        if items.is_empty() {
            ui.label(
                RichText::new("No recent publications yet. Publications you open or save appear here.")
                    .color(crate::theme::disabled()),
            );
            return None;
        }
        let mut hit = None;
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new("Name").size(12.0).color(crate::theme::disabled()));
        });
        for path in items.iter().take(limit) {
            let w = ui.available_width().min(900.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
            let p = ui.painter();
            if resp.hovered() {
                p.rect_filled(rect, 0.0, crate::theme::hover_bg());
            }
            let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            #[cfg(test)]
            seen(format!("recent:{name}"), rect, true, false);
            p.text(rect.min + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, name, FontId::proportional(14.0), crate::theme::text());
            let crumb = path.parent().map(recent::breadcrumb).unwrap_or_default();
            p.text(rect.min + Vec2::new(10.0, 27.0), Align2::LEFT_TOP, crumb, FontId::proportional(11.5), crate::theme::group_label());
            if let Some(date) = modified_text(path) {
                p.text(
                    Pos2::new(rect.max.x - 14.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    date,
                    FontId::proportional(12.5),
                    crate::theme::group_label(),
                );
            }
            p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, crate::theme::line()));
            if resp.clicked() {
                hit = Some(path.clone());
            }
        }
        hit
    }

    fn new_page(&mut self, ui: &mut Ui) {
        page_title(ui, "New");
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
                section_title(ui, "Church");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(16.0, 10.0);
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
                section_title(ui, "More Templates");
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

    /// A places column (selected place plus Browse) used by Open and Save As.
    fn places(ui: &mut Ui, labels: &[&str]) -> bool {
        let mut browse = false;
        ui.allocate_ui(Vec2::new(230.0, 400.0), |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, label) in labels.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 38.0), Sense::hover());
                    #[cfg(test)]
                    seen(format!("place:{label}"), rect, true, i == 0);
                    if i == 0 {
                        ui.painter().rect_filled(rect, 0.0, selected_row());
                    }
                    ui.painter().text(rect.min + Vec2::new(14.0, 19.0), Align2::LEFT_CENTER, *label, FontId::proportional(14.5), crate::theme::text());
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
        page_title(ui, "Open");
        let mut browse = false;
        let mut hit = None;
        ui.horizontal_top(|ui| {
            browse = Self::places(ui, &["Recent", "This PC"]);
            ui.add_space(16.0);
            ui.vertical(|ui| {
                section_title(ui, "Recent Publications");
                tab_row(ui, &[("Publications", true), ("Folders", false)], 0);
                ui.add_space(8.0);
                hit = self.recent_list(ui, 20);
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

/// The modified date of a recent file as text (the made-up date in demo mode).
fn modified_text(path: &Path) -> Option<String> {
    if crate::demo::is_on() {
        return crate::demo::date_of(path).map(str::to_owned);
    }
    std::fs::metadata(path).and_then(|m| m.modified()).ok().map(recent::date_text)
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
        crate::logo::show(ui, crate::logo::Mark::Wordmark, 34.0);
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
            ("Source code: https://github.com/jmitchell238/kernpage", crate::feedback::REPO_URL),
            ("Report a problem: https://github.com/jmitchell238/kernpage/issues", "https://github.com/jmitchell238/kernpage/issues"),
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
    let resp = ui.add(egui::Label::new(RichText::new(text).color(crate::theme::accent_text()).underline()).sense(Sense::click()));
    #[cfg(test)]
    seen(format!("link:{text}"), resp.rect, true, false);
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
