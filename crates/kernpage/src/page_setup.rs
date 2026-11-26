//! Page Design > Page Setup: the Margins, Orientation and Size galleries (docs/ui/PAGE-SETUP.md).
//!
//! Each button is a drop-down gallery. A row applies one command (one undo step) and closes
//! the gallery. "Custom Margins..." and "Page Setup..." open their dialogs (page_dialog.rs);
//! "Create New Page Size..." and "More Preset Page Sizes..." stay disabled.

use crate::app::App;
use crate::page_dialog::DialogKind;
use crate::icons::Icon;
use crate::widgets::{Btn, ACCENT_COLOR, COMING_SOON};
use eframe::egui::{self, Align2, FontId, Painter, Pos2, Popup, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use kernpage_core::pagesetup::{
    inches_text, is_landscape, margin_preset_of, preset_of, MARGIN_PRESETS, PAGE_SIZES,
};
use kernpage_core::{Command, Margins};

const BLUE: egui::Color32 = egui::Color32::from_rgb(0x2B, 0x7C, 0xD3);
const ROW_W: f32 = 300.0;

/// A small page icon: the page outline and, when `margins` is given, the margin guides in
/// blue (inset in proportion to the page).
fn page_icon(p: &Painter, cell: Rect, landscape: bool, guides: Option<Margins>, page: (i64, i64)) {
    let (w, h) = if landscape { (30.0, 22.0) } else { (22.0, 30.0) };
    let r = Rect::from_center_size(cell.center(), Vec2::new(w, h));
    p.rect_filled(r, 0.0, egui::Color32::WHITE);
    p.rect_stroke(r, 0.0, Stroke::new(1.0, BLUE), StrokeKind::Inside);
    if let Some(m) = guides {
        let (pw, ph) = (page.0.max(1) as f32, page.1.max(1) as f32);
        let g = Rect::from_min_max(
            r.min + Vec2::new(m.left as f32 / pw * w, m.top as f32 / ph * h),
            r.max - Vec2::new(m.right as f32 / pw * w, m.bottom as f32 / ph * h),
        );
        if g.width() > 0.0 && g.height() > 0.0 {
            p.rect_stroke(g, 0.0, Stroke::new(1.0, BLUE), StrokeKind::Inside);
        }
    }
}

/// One gallery row: an icon cell, a bold title and text under it. The current choice has a
/// teal frame around its icon cell. Returns the response.
fn row(
    ui: &mut Ui,
    height: f32,
    current: bool,
    icon: impl FnOnce(&Painter, Rect),
    title: &str,
    body: impl FnOnce(&Painter, Pos2),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ROW_W, height), Sense::click());
    let p = ui.painter();
    if resp.hovered() || resp.has_focus() {
        p.rect_filled(rect, 2.0, crate::theme::hover_bg());
    }
    let cell = Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::splat(height - 8.0));
    if current {
        p.rect_stroke(cell, 2.0, Stroke::new(2.0, ACCENT_COLOR), StrokeKind::Inside);
    }
    icon(p, cell);
    let x = cell.max.x + 10.0;
    p.text(
        Pos2::new(x, rect.min.y + 6.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.0),
        crate::theme::text(),
    );
    // Bold look: the same text drawn twice, a pixel apart.
    p.text(
        Pos2::new(x + 0.6, rect.min.y + 6.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.0),
        crate::theme::text(),
    );
    body(p, Pos2::new(x, rect.min.y + 24.0));
    resp
}

/// A disabled command row at the bottom of a gallery.
/// Returns true when clicked.
fn command_row(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    let r = ui.add_enabled(enabled, egui::Button::new(label).min_size(Vec2::new(ROW_W, 0.0)));
    if !enabled {
        r.on_disabled_hover_text(COMING_SOON);
        return false;
    }
    r.clicked()
}

impl App {
    /// The three large buttons of the Page Setup group.
    pub(crate) fn page_setup_buttons(&mut self, ui: &mut Ui) {
        let page = self
            .editor
            .as_ref()
            .and_then(|e| e.doc().pages.first())
            .map(|p| (p.width_emu, p.height_emu, p.margins_emu));
        let enabled = page.is_some();
        let (w, h, margins) = page.unwrap_or((0, 0, Margins::all(0)));
        self.margins_menu(ui, enabled, (w, h), margins);
        self.orientation_menu(ui, enabled, (w, h));
        self.size_menu(ui, enabled, (w, h));
    }

    fn open_for_debug(&self, ui: &Ui, resp: &egui::Response, name: &str) {
        if ui.is_visible() && self.debug_menu.as_deref() == Some(name) {
            Popup::open_id(ui.ctx(), Popup::default_response_id(resp));
        }
    }

    fn margins_menu(&mut self, ui: &mut Ui, enabled: bool, page: (i64, i64), cur: Margins) {
        let r = Btn::large(Icon::Margins, "Margins")
            .enabled(enabled)
            .dropdown()
            .show(ui);
        self.open_for_debug(ui, &r, "margins");
        let mut chosen = None;
        let mut open_dialog = None;
        let landscape = is_landscape(page.0, page.1);
        let preset = margin_preset_of(cur);
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(ROW_W);
            let mut item = |ui: &mut Ui, name: &str, m: Margins, current: bool| {
                let resp = row(
                    ui,
                    54.0,
                    current,
                    |p, cell| page_icon(p, cell, landscape, Some(m), page),
                    name,
                    |p, at| {
                        let f = FontId::proportional(11.5);
                        let col2 = at.x + 112.0;
                        let t = |a: &str, v: i64| format!("{a}: {}\"", inches_text(v));
                        p.text(at, Align2::LEFT_TOP, t("Top", m.top), f.clone(), crate::theme::text());
                        p.text(
                            at + Vec2::new(0.0, 14.0),
                            Align2::LEFT_TOP,
                            t("Left", m.left),
                            f.clone(),
                            crate::theme::text(),
                        );
                        p.text(
                            Pos2::new(col2, at.y),
                            Align2::LEFT_TOP,
                            t("Bottom", m.bottom),
                            f.clone(),
                            crate::theme::text(),
                        );
                        p.text(
                            Pos2::new(col2, at.y + 14.0),
                            Align2::LEFT_TOP,
                            t("Right", m.right),
                            f,
                            crate::theme::text(),
                        );
                    },
                );
                #[cfg(test)]
                probe::record(probe::Row {
                    title: name.to_owned(),
                    detail: format!(
                        "Top {} Bottom {} Left {} Right {}",
                        inches_text(m.top),
                        inches_text(m.bottom),
                        inches_text(m.left),
                        inches_text(m.right)
                    ),
                    current,
                    rect: resp.rect,
                });
                if resp.clicked() {
                    chosen = Some(m);
                    ui.close();
                }
            };
            for (i, mp) in MARGIN_PRESETS.iter().enumerate() {
                item(ui, mp.name, mp.margins, preset == Some(i));
            }
            if preset.is_none() {
                item(ui, "Custom Margins", cur, true);
            }
            ui.separator();
            if command_row(ui, "Custom Margins...", true) {
                open_dialog = Some(DialogKind::CustomMargins);
                ui.close();
            }
        });
        if let Some(margins) = chosen {
            self.exec(Command::SetMargins { margins });
        }
        if let Some(k) = open_dialog {
            self.open_page_dialog(k);
        }
    }

    fn orientation_menu(&mut self, ui: &mut Ui, enabled: bool, page: (i64, i64)) {
        let r = Btn::large(Icon::Orientation, "Orientation")
            .enabled(enabled)
            .dropdown()
            .show(ui);
        self.open_for_debug(ui, &r, "orientation");
        let now_landscape = is_landscape(page.0, page.1);
        let mut chosen = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(ROW_W);
            for (name, landscape) in [("Portrait", false), ("Landscape", true)] {
                let resp = row(
                    ui,
                    40.0,
                    now_landscape == landscape,
                    |p, cell| page_icon(p, cell, landscape, None, page),
                    name,
                    |_, _| {},
                );
                #[cfg(test)]
                probe::record(probe::Row {
                    title: name.to_owned(),
                    detail: String::new(),
                    current: now_landscape == landscape,
                    rect: resp.rect,
                });
                if resp.clicked() {
                    chosen = Some(landscape);
                    ui.close();
                }
            }
        });
        if let Some(landscape) = chosen {
            // The page keeps its size, turned: width and height swap (a no-op when the page
            // already has that orientation).
            if landscape != now_landscape {
                self.exec(Command::SetPageSize {
                    width_emu: page.1,
                    height_emu: page.0,
                });
            }
        }
    }

    fn size_menu(&mut self, ui: &mut Ui, enabled: bool, page: (i64, i64)) {
        let r = Btn::large(Icon::Size, "Size")
            .enabled(enabled)
            .dropdown()
            .show(ui);
        self.open_for_debug(ui, &r, "size");
        let landscape = is_landscape(page.0, page.1);
        let preset = preset_of(page.0, page.1);
        let mut chosen = None;
        let mut open_dialog = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(ROW_W);
            ui.label(egui::RichText::new("Built-In").strong().color(crate::theme::text()));
            egui::ScrollArea::vertical()
                .max_height(8.0 * 46.0)
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    let mut item = |ui: &mut Ui, title: String, text: String, size: (i64, i64), current: bool| {
                        let resp = row(
                            ui,
                            46.0,
                            current,
                            |p, cell| page_icon(p, cell, size.0 > size.1, None, size),
                            &title,
                            |p, at| {
                                p.text(at, Align2::LEFT_TOP, text.clone(), FontId::proportional(11.5), crate::theme::text());
                            },
                        );
                        #[cfg(test)]
                        probe::record(probe::Row {
                            title: title.clone(),
                            detail: text.clone(),
                            current,
                            rect: resp.rect,
                        });
                        if resp.clicked() {
                            chosen = Some(size);
                            ui.close();
                        }
                    };
                    if preset.is_none() {
                        item(
                            ui,
                            "Current".into(),
                            kernpage_core::pagesetup::size_text(page.0, page.1),
                            page,
                            true,
                        );
                    }
                    for (i, ps) in PAGE_SIZES.iter().enumerate() {
                        item(
                            ui,
                            ps.label(landscape),
                            ps.size_text(landscape),
                            ps.size(landscape),
                            preset == Some(i),
                        );
                    }
                });
            ui.separator();
            if command_row(ui, "Page Setup...", true) {
                open_dialog = Some(DialogKind::PageSetup);
                ui.close();
            }
            command_row(ui, "Create New Page Size...", false);
            command_row(ui, "More Preset Page Sizes...", false);
        });
        if let Some((width_emu, height_emu)) = chosen {
            self.exec(Command::SetPageSize {
                width_emu,
                height_emu,
            });
        }
        if let Some(k) = open_dialog {
            self.open_page_dialog(k);
        }
    }
}

/// Test seam: the gallery rows drawn in the last frame (title, detail text, current, rect),
/// because the rows are painted, not labelled widgets.
#[cfg(test)]
pub mod probe {
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
    pub struct Row {
        pub title: String,
        pub detail: String,
        pub current: bool,
        pub rect: eframe::egui::Rect,
    }

    thread_local! {
        static ROWS: RefCell<Vec<Row>> = const { RefCell::new(Vec::new()) };
    }

    pub fn clear() {
        ROWS.with(|r| r.borrow_mut().clear());
    }
    pub fn record(r: Row) {
        ROWS.with(|v| v.borrow_mut().push(r));
    }
    pub fn rows() -> Vec<Row> {
        ROWS.with(|r| r.borrow().clone())
    }
}
