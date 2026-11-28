//! The Page Navigation pane.

use crate::app::App;
use crate::fontmgr::FontMgr;
use crate::icons::{self, Icon};
use crate::render::PageView;
use crate::render::PictureCache;
use crate::widgets::{ACCENT_COLOR};
use eframe::egui::{self, Color32, Sense, Stroke, StrokeKind, Ui, Vec2};
use kernpage_core::geometry::emu_to_px;

pub(crate) const THUMB_W: f32 = 104.0;

/// Test seam: the thumbnails and the close button drawn in the last frame.
#[cfg(test)]
pub mod probe {
    use eframe::egui::Rect;
    use std::cell::RefCell;

    #[derive(Clone, Default)]
    pub struct Pane {
        /// (page index, thumbnail rectangle, drawn as the current page)
        pub thumbs: Vec<(usize, Rect, bool)>,
        pub close: Option<Rect>,
        pub add_card: Option<Rect>,
    }

    thread_local! {
        static PANE: RefCell<Pane> = RefCell::new(Pane::default());
    }

    pub fn clear() {
        PANE.with(|p| *p.borrow_mut() = Pane::default());
    }
    pub fn get() -> Pane {
        PANE.with(|p| p.borrow().clone())
    }
    pub(crate) fn with(f: impl FnOnce(&mut Pane)) {
        PANE.with(|p| f(&mut p.borrow_mut()));
    }
}

#[derive(Clone, Copy)]
enum PageAct {
    Insert,
    Duplicate,
    Delete,
    Up,
    Down,
    Master(crate::masters::PaneMasterChoice),
}

impl App {
    pub fn nav_pane(&mut self, ui: &mut Ui) {
        let mut go = None;
        let mut hide = false;
        let mut add_page = false;
        // (page, action) chosen from a thumbnail's right-click menu.
        let mut act: Option<(usize, PageAct)> = None;
        let in_master_view = self.master_view.is_some();
        let mut master_act: Option<crate::masters::MasterAct> = None;
        egui::Panel::left("page_nav")
            .exact_size(THUMB_W + 34.0)
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::card())
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let title = if in_master_view { "Master Pages" } else { "Pages" };
                    ui.label(egui::RichText::new(title).color(crate::theme::text()).size(14.5).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                        icons::paint(
                            ui.painter(),
                            r.shrink(2.0),
                            Icon::Close,
                            if resp.hovered() {
                                ACCENT_COLOR
                            } else {
                                crate::theme::disabled()
                            },
                        );
                        #[cfg(test)]
                        probe::with(|p| p.close = Some(r));
                        if resp.on_hover_text("Hide Page Navigation").clicked() {
                            hide = true;
                        }
                    });
                });
                ui.add_space(10.0);
                let Some(ed) = self.editor.as_ref() else {
                    return;
                };
                let doc = ed.doc();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if in_master_view {
                            master_act = crate::masters::master_list(
                                ui,
                                doc,
                                self.page,
                                &self.fonts,
                                &mut self.pics,
                            );
                            return;
                        }
                        // Room for the selection highlight (drawn 5 px outside the thumbnail)
                        // so the first page's top edge is not clipped by the scroll area.
                        ui.add_space(6.0);
                        for (i, page) in doc.pages.iter().enumerate() {
                            let aspect = page.height_emu as f32 / page.width_emu.max(1) as f32;
                            let size = Vec2::new(THUMB_W, THUMB_W * aspect);
                            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                            let current = i == self.page;
                            #[cfg(test)]
                            probe::with(|p| p.thumbs.push((i, rect, current)));
                            thumb_label(ui, rect, &resp, current);
                            paint_thumb(ui, rect, doc, i, &self.fonts, &mut self.pics);
                            thumb_border(ui, rect, current);
                            ui.label(
                                {
                                    let t = egui::RichText::new(format!("{}", i + 1))
                                        .color(crate::theme::text())
                                        .size(12.0);
                                    if current {
                                        t.strong()
                                    } else {
                                        t
                                    }
                                },
                            );
                            ui.add_space(8.0);
                            if resp.clicked() {
                                go = Some(i);
                            }
                            // Publisher's Pages pane menu: Insert Blank Page, Insert Duplicate Page, Delete
                            // Page, Move. Publisher's Move opens a dialog; ours moves a
                            // page one place (deviation).
                            let n = doc.pages.len();
                            resp.context_menu(|ui| {
                                if ui.button("Insert Blank Page").clicked() {
                                    act = Some((i, PageAct::Insert));
                                }
                                if ui.button("Insert Duplicate Page").clicked() {
                                    act = Some((i, PageAct::Duplicate));
                                }
                                if ui.add_enabled(n > 1, egui::Button::new("Delete Page")).clicked() {
                                    act = Some((i, PageAct::Delete));
                                }
                                ui.menu_button("Master Pages", |ui| {
                                    let (items, ignore) = crate::masters::menu_items_of(doc, i);
                                    for (m, (label, current)) in items.iter().enumerate() {
                                        if ui.add(egui::Button::new(label).selected(*current)).clicked() {
                                            act = Some((i, PageAct::Master(crate::masters::PaneMasterChoice::Use(m))));
                                            ui.close();
                                        }
                                    }
                                    if ui.add(egui::Button::new("None").selected(ignore)).clicked() {
                                        act = Some((i, PageAct::Master(crate::masters::PaneMasterChoice::None)));
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Edit Master Pages").clicked() {
                                        act = Some((i, PageAct::Master(crate::masters::PaneMasterChoice::Edit)));
                                        ui.close();
                                    }
                                    if ui.button("Apply Master Page...").clicked() {
                                        act = Some((i, PageAct::Master(crate::masters::PaneMasterChoice::Dialog)));
                                        ui.close();
                                    }
                                });
                                ui.separator();
                                if ui.add_enabled(i > 0, egui::Button::new("Move Page Up")).clicked() {
                                    act = Some((i, PageAct::Up));
                                }
                                if ui.add_enabled(i + 1 < n, egui::Button::new("Move Page Down")).clicked() {
                                    act = Some((i, PageAct::Down));
                                }
                                if act.is_some() {
                                    ui.close();
                                }
                            });
                        }
                        if add_page_card(ui) {
                            add_page = true;
                        }
                    });
            });
        if let Some(i) = go {
            self.go_page(i);
        }
        if let Some((i, a)) = act {
            match a {
                PageAct::Insert => self.insert_page_after(i),
                PageAct::Duplicate => self.duplicate_page_at(i),
                PageAct::Delete => self.delete_page_at(i),
                PageAct::Up => self.move_page_by(i, -1),
                PageAct::Down => self.move_page_by(i, 1),
                PageAct::Master(choice) => self.pane_master_choice(i, choice),
            }
        }
        if let Some(a) = master_act {
            self.run_master_act(a);
        }
        if add_page {
            let last = self.page_count().saturating_sub(1);
            self.insert_page_after(last);
        }
        if hide {
            self.show.nav = false;
        }
    }
}

/// The highlight behind the current thumbnail (or under the pointer) and its shadow.
pub(crate) fn thumb_label(ui: &Ui, rect: egui::Rect, resp: &egui::Response, current: bool) {
    let p = ui.painter();
    let r = 6.0;
    if current {
        p.rect_filled(rect.expand(5.0), r, crate::theme::accent_tint());
    } else if resp.hovered() {
        p.rect_filled(rect.expand(5.0), r, crate::theme::hover_bg());
    }
    p.add(
        egui::Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: crate::theme::shadow(),
        }
        .as_shape(rect, egui::CornerRadius::same(3)),
    );
}

/// The outline of a thumbnail: accent and 2 px for the current one.
pub(crate) fn thumb_border(ui: &Ui, rect: egui::Rect, current: bool) {
    let border = if current {
        Stroke::new(2.0, crate::theme::accent())
    } else {
        Stroke::new(1.0, crate::theme::line())
    };
    ui.painter().rect_stroke(rect, 2.0, border, StrokeKind::Outside);
}

/// The "+ Add page" card at the end of the thumbnail list: a dashed rounded outline, a plus
/// and a label. Returns true when it was clicked.
fn add_page_card(ui: &mut Ui) -> bool {
    add_card(ui, "Add page", "Insert a blank page at the end")
}

/// A dashed "+" card with `label`; true when clicked.
pub(crate) fn add_card(ui: &mut Ui, label: &str, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(THUMB_W, 48.0), Sense::click());
    let hot = resp.hovered();
    let col = if hot { crate::theme::accent() } else { crate::theme::disabled() };
    let p = ui.painter();
    if hot {
        p.rect_filled(rect, 8.0, crate::theme::hover_bg());
    }
    // Dashed rounded outline: walk the rectangle's perimeter with quarter-circle corners.
    let (r, rr) = (rect.shrink(0.5), 8.0_f32);
    let mut pts: Vec<egui::Pos2> = Vec::new();
    let corners = [
        (egui::pos2(r.max.x - rr, r.min.y + rr), -90.0_f32),
        (egui::pos2(r.max.x - rr, r.max.y - rr), 0.0),
        (egui::pos2(r.min.x + rr, r.max.y - rr), 90.0),
        (egui::pos2(r.min.x + rr, r.min.y + rr), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=6 {
            let a = (start + 90.0 * k as f32 / 6.0).to_radians();
            pts.push(c + Vec2::new(a.cos(), a.sin()) * rr);
        }
    }
    pts.push(pts[0]);
    p.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.0, col), 4.0, 3.0));
    // Plus icon above the label.
    let c = egui::pos2(rect.center().x, rect.min.y + 17.0);
    let st = Stroke::new(1.6, col);
    p.line_segment([c - Vec2::new(6.0, 0.0), c + Vec2::new(6.0, 0.0)], st);
    p.line_segment([c - Vec2::new(0.0, 6.0), c + Vec2::new(0.0, 6.0)], st);
    p.text(
        egui::pos2(rect.center().x, rect.max.y - 12.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        if hot { crate::theme::accent_text() } else { crate::theme::text() },
    );
    #[cfg(test)]
    probe::with(|p| p.add_card = Some(rect));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    resp.on_hover_text(tip).clicked()
}

/// Paint a miniature of `page` (white paper and its shapes) into `rect`.
pub fn paint_thumb(
    ui: &Ui,
    rect: egui::Rect,
    doc: &kernpage_core::Document,
    index: usize,
    fonts: &FontMgr,
    pics: &mut PictureCache,
) {
    let Some(page) = doc.surface(index) else {
        return;
    };
    // A master shows its page-number fields as `#`.
    let number = if kernpage_core::master_surface_parts(index).is_some() {
        "#".to_owned()
    } else {
        doc.page_number_text(index)
    };
    let p = ui.painter();
    p.rect_filled(rect, 0.0, Color32::WHITE);
    let zoom = f64::from(rect.width()) / emu_to_px(page.width_emu, 1.0);
    let tp = p.with_clip_rect(rect);
    let mut view = PageView {
        painter: &tp,
        fonts,
        pics,
        origin: rect.min,
        zoom,
        boundaries: false,
        shapes: &page.shapes,
        editing_cell: None,
    };
    view.draw_master(ui.ctx(), doc.master_shapes(index), &number);
    for s in &page.shapes {
        view.draw_shape_numbered(ui.ctx(), s, &number);
    }
}
