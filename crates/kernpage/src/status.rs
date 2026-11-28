//! The status bar, laid out from measurements of Publisher 2021's (see out/figcraft/statusbar-measure.md).

use crate::app::{App, ZoomMode};
use crate::icons::Icon;
use crate::widgets::{Btn, LABEL_PX, STATUS_BAR_H};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use kernpage_core::geometry::{clamp_zoom, MAX_ZOOM, MIN_ZOOM};

const ZOOM_STEPS: [f64; 11] = [0.1, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0, 4.01];

/// Text and glyphs on the neutral status bar.
fn on_bar() -> Color32 {
    crate::theme::status_text()
}

fn dim() -> Color32 {
    crate::theme::status_dim()
}

fn bar_fill() -> Color32 {
    crate::theme::status_bg()
}

/// What the zoom pill asked for this frame.
enum PillAct {
    None,
    Out,
    In,
    Set(f64),
    Fit(ZoomMode),
}

thread_local! {
    /// Test and screenshot seam: keep the zoom list open.
    pub(crate) static ZOOM_MENU_OPEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// KernPage's own zoom control: a rounded pill "- 57% +". Clicking the percentage opens a
/// short list of zoom levels.
fn zoom_pill(ui: &mut Ui, zoom: f64, enabled: bool) -> PillAct {
    const CELL: f32 = 22.0;
    const PCT_W: f32 = 46.0;
    let pill_h = STATUS_BAR_H - 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(CELL * 2.0 + PCT_W, STATUS_BAR_H), Sense::hover());
    let pill = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), pill_h));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let minus = Rect::from_min_size(pill.min, Vec2::new(CELL, pill_h));
    let mid = Rect::from_min_size(Pos2::new(pill.min.x + CELL, pill.min.y), Vec2::new(PCT_W, pill_h));
    let plus = Rect::from_min_size(Pos2::new(mid.max.x, pill.min.y), Vec2::new(CELL, pill_h));
    let base = ui.id().with("zoom_pill");
    let r_minus = ui.interact(minus, base.with("out"), sense);
    let r_mid = ui.interact(mid, base.with("pct"), sense);
    let r_plus = ui.interact(plus, base.with("in"), sense);
    let col = if enabled { on_bar() } else { dim() };
    let p = ui.painter();
    let (fill, line) =
        (crate::theme::mix(bar_fill(), on_bar(), 0.07), crate::theme::mix(bar_fill(), on_bar(), 0.28));
    p.rect_filled(pill, pill_h / 2.0, fill);
    p.rect_stroke(pill, pill_h / 2.0, Stroke::new(1.0, line), egui::StrokeKind::Inside);
    for (r, resp) in [(minus, &r_minus), (mid, &r_mid), (plus, &r_plus)] {
        if enabled && resp.hovered() {
            let rad = if r == mid { 0.0 } else { pill_h / 2.0 };
            p.rect_filled(r.shrink(1.0), rad, crate::theme::hover_bg());
        }
    }
    let ppp = ui.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let st = Stroke::new(1.2, col);
    let (cm, cp) = (minus.center(), plus.center());
    let (cm, cp) = (Pos2::new(snap(cm.x), snap(cm.y)), Pos2::new(snap(cp.x), snap(cp.y)));
    let p = ui.painter();
    p.line_segment([Pos2::new(cm.x - 4.0, cm.y), Pos2::new(cm.x + 4.0, cm.y)], st);
    p.line_segment([Pos2::new(cp.x - 4.0, cp.y), Pos2::new(cp.x + 4.0, cp.y)], st);
    p.line_segment([Pos2::new(cp.x, cp.y - 4.0), Pos2::new(cp.x, cp.y + 4.0)], st);
    let pct = format!("{:.0}%", zoom * 100.0);
    p.text(mid.center(), egui::Align2::CENTER_CENTER, &pct, egui::FontId::proportional(LABEL_PX), col);
    r_minus.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, "\u{2212}"));
    r_plus.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, "+"));
    r_mid.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &pct));
    let mut act = PillAct::None;
    let (clicked_out, clicked_in, clicked_pct) = (r_minus.clicked(), r_plus.clicked(), r_mid.clicked());
    let _ = (r_minus.on_hover_text("Zoom Out"), r_plus.on_hover_text("Zoom In"));
    if clicked_out {
        act = PillAct::Out;
    }
    if clicked_in {
        act = PillAct::In;
    }
    let menu_id = base.with("open");
    let mut open: bool = ui.ctx().data(|d| d.get_temp(menu_id)).unwrap_or(false);
    if enabled && clicked_pct {
        open = !open;
    }
    if enabled && ZOOM_MENU_OPEN.with(|c| c.get()) {
        open = true;
    }
    let r_mid = r_mid.on_hover_text("Choose a zoom level");
    egui::Popup::menu(&r_mid)
        .align(egui::RectAlign::TOP_START)
        .open_bool(&mut open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_min_width(110.0);
            let mut pick = |ui: &mut Ui, label: &str, a: PillAct| {
                if ui.button(label).clicked() {
                    act = a;
                }
            };
            pick(ui, "Fit page", PillAct::Fit(ZoomMode::WholePage));
            pick(ui, "Fit width", PillAct::Fit(ZoomMode::PageWidth));
            ui.separator();
            for (label, z) in [("50%", 0.5), ("75%", 0.75), ("100%", 1.0), ("150%", 1.5), ("200%", 2.0)] {
                pick(ui, label, PillAct::Set(z));
            }
        });
    ui.ctx().data_mut(|d| d.insert_temp(menu_id, open));
    act
}

impl App {
    /// "W x H in." of the selected object(s), or empty when nothing is selected.
    pub(crate) fn selection_size_text(&self) -> String {
        let Some(ed) = self.editor.as_ref() else { return String::new() };
        let rects: Vec<_> = self
            .selected_ids()
            .into_iter()
            .filter_map(|id| ed.doc().shape(self.page, id).map(|s| s.rect))
            .collect();
        let Some(first) = rects.first() else { return String::new() };
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x + first.w, first.y + first.h);
        for r in &rects {
            x0 = x0.min(r.x);
            y0 = y0.min(r.y);
            x1 = x1.max(r.x + r.w);
            y1 = y1.max(r.y + r.h);
        }
        let inch = kernpage_core::geometry::emu_to_in;
        format!("{:.2} x {:.2} in.", inch(x1 - x0), inch(y1 - y0))
    }

    pub fn status_bar(&mut self, ui: &mut Ui) {
        crate::theme::drew!("status:neutral");
        egui::Panel::bottom("status")
            .exact_size(STATUS_BAR_H)
            .frame(egui::Frame::new().fill(bar_fill()).inner_margin(egui::Margin::symmetric(9, 0)))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.horizontal_centered(|ui| {
                    let pages = self.editor.as_ref().map_or(0, |e| e.doc().pages.len());
                    let text = |s: String| RichText::new(s).color(on_bar()).size(LABEL_PX);
                    {
                        let v = ui.visuals_mut();
                        v.override_text_color = Some(on_bar());
                    }
                    let page_text = self.master_status().unwrap_or_else(|| {
                        format!("Page: {} of {}", if pages == 0 { 0 } else { self.page + 1 }, pages)
                    });
                    ui.label(text(page_text));
                    ui.add_space(19.0);
                    let icon_cell = |ui: &mut Ui, icon: Icon| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(15.0, STATUS_BAR_H), Sense::hover());
                        crate::icons::paint(ui.painter(), Rect::from_center_size(r.center(), Vec2::splat(15.0)), icon, on_bar());
                    };
                    icon_cell(ui, Icon::Pointer);
                    ui.add_space(4.0);
                    let pos = match self.cursor_in() {
                        Some((x, y)) if pages > 0 => format!("{x:.2}, {y:.2} in."),
                        _ => String::new(),
                    };
                    ui.allocate_ui(Vec2::new(0.0, STATUS_BAR_H), |ui| {
                        ui.label(text(pos))
                    });
                    ui.add_space(14.0);
                    icon_cell(ui, Icon::ObjectSize);
                    ui.add_space(4.0);
                    let size = self.selection_size_text();
                    ui.allocate_ui(Vec2::new(0.0, STATUS_BAR_H), |ui| {
                        ui.label(text(size))
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let enabled = pages > 0;
                        let fg = |on: bool| if on { on_bar() } else { dim() };
                        // Right to left: Whole Page, the zoom pill, view buttons.
                        if Btn::status(Icon::ZoomFit)
                            .cell_w(24.0)
                            .enabled(enabled)
                            .fg(fg(enabled))
                            .tip("Whole Page")
                            .show(ui)
                            .clicked()
                        {
                            self.zoom_mode = ZoomMode::WholePage;
                        }
                        let mut z = self.zoom;
                        match zoom_pill(ui, self.zoom, enabled) {
                            PillAct::None => {}
                            PillAct::In => {
                                z = ZOOM_STEPS
                                    .iter()
                                    .copied()
                                    .find(|&s| s > self.zoom + 1e-6)
                                    .unwrap_or(MAX_ZOOM)
                                    .min(MAX_ZOOM);
                                self.zoom_mode = ZoomMode::Custom;
                            }
                            PillAct::Out => {
                                z = ZOOM_STEPS
                                    .iter()
                                    .rev()
                                    .copied()
                                    .find(|&s| s < self.zoom - 1e-6)
                                    .unwrap_or(MIN_ZOOM)
                                    .max(MIN_ZOOM);
                                self.zoom_mode = ZoomMode::Custom;
                            }
                            PillAct::Set(v) => {
                                z = v;
                                self.zoom_mode = ZoomMode::Custom;
                            }
                            PillAct::Fit(m) => self.zoom_mode = m,
                        }
                        if self.zoom_mode == ZoomMode::Custom {
                            self.zoom = clamp_zoom(z);
                        }
                        Btn::status(Icon::TwoPage)
                            .disabled()
                            .fg(fg(false))
                            .tip("Two-Page Spread: coming soon")
                            .show(ui);
                        Btn::status(Icon::SinglePage)
                            .selected(true)
                            .enabled(enabled)
                            .fg(fg(enabled))
                            .tip("Single Page")
                            .show(ui);
                    });
                });
            });
    }
}
