//! The contextual ribbon tabs (Shape Format, Text Box, Picture Format, Table Design, Layout).
//! Controls that match an existing command call the same actions the Home tab uses; the rest
//! are shown in place, disabled.

use crate::app::App;
use crate::icons::Icon;
use crate::ribbon::{soon, soon_dd, soon_split};
use crate::widgets::{check, group, Btn};
use eframe::egui::{self, Color32, Sense, Ui, Vec2};
use kernpage_core::geometry::{emu_to_in, in_to_emu};
use kernpage_core::{Command, Rect, ShapeKind};

/// One spinner step of the Height / Width boxes, in inches.
const STEP_IN: f64 = 0.1;
/// Smallest size the boxes accept, in inches.
const MIN_IN: f64 = 0.01;

/// Inches as shown in the boxes: `7"`, `1.25"`.
pub(crate) fn fmt_inches(emu: i64) -> String {
    let t = format!("{:.2}", emu_to_in(emu));
    format!("{}\"", t.trim_end_matches('0').trim_end_matches('.'))
}

/// Parse a typed size (`2`, `2.5"`, `2 in`) into EMU; `None` when not a usable size.
pub(crate) fn parse_inches(text: &str) -> Option<i64> {
    let t = text
        .trim()
        .trim_end_matches('"')
        .trim_end_matches("in")
        .trim();
    let v: f64 = t.parse().ok()?;
    (MIN_IN..=300.0).contains(&v).then(|| in_to_emu(v))
}

/// A disabled placeholder for an in-ribbon gallery (swatch cells with a frame).
fn gallery(ui: &mut Ui, width: f32) {
    let width = match crate::ribbon_fit::level() {
        0 => width,
        1 => (width * 0.66).max(52.0),
        _ => (width * 0.33).max(52.0),
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 56.0), Sense::hover());
    #[cfg(test)]
    if ui.is_visible() {
        crate::backstage::seen("gallery".to_owned(), rect, false, false);
    }
    let p = ui.painter();
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::button_outline()),
        egui::StrokeKind::Inside,
    );
    let n = ((width - 8.0) / 44.0).floor().max(1.0) as usize;
    for i in 0..n {
        let r = egui::Rect::from_min_size(
            rect.min + Vec2::new(8.0 + i as f32 * 44.0, 12.0),
            Vec2::splat(32.0),
        );
        p.rect_stroke(
            r,
            4.0,
            egui::Stroke::new(1.5, Color32::from_gray(0xD6)),
            egui::StrokeKind::Inside,
        );
    }
}

/// The nine "Align <vertical> <horizontal>" buttons (disabled: no vertical text anchor yet).
fn anchor_grid(ui: &mut Ui) {
    crate::widgets::col(ui, |ui| {
        for _ in 0..3 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for icon in [Icon::AlignLeft, Icon::AlignCenter, Icon::AlignRight] {
                    soon(ui, icon, "", false);
                }
            });
        }
    });
}

/// Glyphs of the Insert Shapes gallery; `Some(tool)` ones start that draw tool.
#[derive(Clone, Copy)]
enum Glyph {
    TextBox,
    Line,
    Diagonal,
    Rect,
    Oval,
    Round,
    Triangle,
    Arrow,
    Curve,
    Star,
    Brace,
}

fn paint_glyph(p: &egui::Painter, r: egui::Rect, g: Glyph, col: Color32) {
    let st = egui::Stroke::new(1.3, col);
    let r = r.shrink(3.5);
    let pt = |x: f32, y: f32| egui::pos2(r.min.x + x * r.width(), r.min.y + y * r.height());
    match g {
        Glyph::TextBox => {
            p.rect_stroke(r, 0.0, st, egui::StrokeKind::Middle);
            p.line_segment([pt(0.3, 0.25), pt(0.7, 0.25)], st);
            p.line_segment([pt(0.5, 0.25), pt(0.5, 0.8)], st);
        }
        Glyph::Line => {
            p.line_segment([pt(0.0, 1.0), pt(1.0, 0.0)], st);
        }
        Glyph::Diagonal => {
            p.line_segment([pt(0.0, 0.0), pt(1.0, 1.0)], st);
        }
        Glyph::Rect => {
            p.rect_stroke(r, 0.0, st, egui::StrokeKind::Middle);
        }
        Glyph::Oval => {
            p.circle_stroke(r.center(), r.width() / 2.0, st);
        }
        Glyph::Round => {
            p.rect_stroke(r, 3.0, st, egui::StrokeKind::Middle);
        }
        Glyph::Triangle => {
            p.add(egui::Shape::closed_line(
                vec![pt(0.5, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)],
                st,
            ));
        }
        Glyph::Arrow => {
            p.add(egui::Shape::closed_line(
                vec![
                    pt(0.0, 0.3),
                    pt(0.6, 0.3),
                    pt(0.6, 0.0),
                    pt(1.0, 0.5),
                    pt(0.6, 1.0),
                    pt(0.6, 0.7),
                    pt(0.0, 0.7),
                ],
                st,
            ));
        }
        Glyph::Curve => {
            p.add(egui::Shape::line(
                vec![pt(0.0, 1.0), pt(0.3, 0.2), pt(0.7, 0.8), pt(1.0, 0.0)],
                st,
            ));
        }
        Glyph::Star => {
            p.add(egui::Shape::closed_line(
                vec![
                    pt(0.5, 0.0),
                    pt(0.62, 0.38),
                    pt(1.0, 0.38),
                    pt(0.7, 0.62),
                    pt(0.8, 1.0),
                    pt(0.5, 0.76),
                    pt(0.2, 1.0),
                    pt(0.3, 0.62),
                    pt(0.0, 0.38),
                    pt(0.38, 0.38),
                ],
                st,
            ));
        }
        Glyph::Brace => {
            p.add(egui::Shape::line(
                vec![
                    pt(0.8, 0.0),
                    pt(0.4, 0.1),
                    pt(0.4, 0.4),
                    pt(0.1, 0.5),
                    pt(0.4, 0.6),
                    pt(0.4, 0.9),
                    pt(0.8, 1.0),
                ],
                st,
            ));
        }
    }
}

impl App {
    /// The Insert Shapes gallery: 3 rows of shapes with scroll arrows. Text Box, Line,
    /// Rectangle and Oval start the same draw tools as Home > Shapes; the rest are disabled.
    fn shapes_gallery(&mut self, ui: &mut Ui) {
        use crate::app::Tool;
        let rows: [[(Glyph, Option<Tool>); 7]; 3] = [
            [
                (Glyph::TextBox, Some(Tool::TextBox)),
                (Glyph::Line, Some(Tool::Line)),
                (Glyph::Diagonal, None),
                (Glyph::Rect, Some(Tool::Rectangle)),
                (Glyph::Oval, Some(Tool::Oval)),
                (Glyph::Round, None),
                (Glyph::Triangle, None),
            ],
            [
                (Glyph::Triangle, None),
                (Glyph::Curve, None),
                (Glyph::Arrow, None),
                (Glyph::Arrow, None),
                (Glyph::Round, None),
                (Glyph::Star, None),
                (Glyph::Brace, None),
            ],
            [
                (Glyph::Curve, None),
                (Glyph::Curve, None),
                (Glyph::Curve, None),
                (Glyph::Brace, None),
                (Glyph::Brace, None),
                (Glyph::Star, None),
                (Glyph::Arrow, None),
            ],
        ];
        let mut start = None;
        let cols = crate::ribbon_fit::gallery_cols(7);
        let (frame, _) =
            ui.allocate_exact_size(Vec2::new(cols as f32 * 22.0 + 16.0, 66.0), Sense::hover());
        ui.painter().rect_stroke(
            frame,
            2.0,
            egui::Stroke::new(1.0, crate::theme::button_outline()),
            egui::StrokeKind::Inside,
        );
        for (ri, row) in rows.iter().enumerate() {
            for (ci, (g, tool)) in row.iter().enumerate().take(cols) {
                let cell = egui::Rect::from_min_size(
                    frame.min + Vec2::new(1.0 + ci as f32 * 22.0, ri as f32 * 22.0),
                    Vec2::splat(22.0),
                );
                let sense = if tool.is_some() {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let resp = ui.interact(cell, ui.scope_id().with(("shape_gal", ri, ci)), sense);
                #[cfg(test)]
                if ui.is_visible() {
                    crate::backstage::seen(format!("shape_gal:{ri},{ci}"), cell, tool.is_some(), false);
                }
                if resp.hovered() && tool.is_some() {
                    ui.painter().rect_filled(cell, 2.0, crate::theme::hover_bg());
                }
                let col = if tool.is_some() {
                    crate::widgets::ACCENT_COLOR
                } else {
                    crate::theme::disabled()
                };
                paint_glyph(ui.painter(), cell, *g, col);
                if resp.clicked() {
                    start = *tool;
                }
            }
        }
        // Scroll arrows (inert): up, down, more.
        for (i, up) in [true, false, false].into_iter().enumerate() {
            let c = egui::pos2(frame.max.x - 8.0, frame.min.y + 11.0 + i as f32 * 22.0);
            let s = if up { -1.0 } else { 1.0 };
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(-3.0, 1.5 * s),
                    c + Vec2::new(3.0, 1.5 * s),
                    c + Vec2::new(0.0, -1.5 * s),
                ],
                crate::theme::disabled(),
                egui::Stroke::NONE,
            ));
        }
        if let Some(t) = start {
            self.start_tool(t);
        }
    }

    /// Height or Width box with a spinner. Returns the new size (EMU) when the user commits one.
    fn size_box(&self, ui: &mut Ui, label: &str, icon: Icon, emu: Option<i64>) -> Option<i64> {
        let mut out = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let (ir, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            crate::icons::paint(
                ui.painter(),
                ir,
                icon,
                if emu.is_some() {
                    crate::widgets::ACCENT_COLOR
                } else {
                    crate::theme::disabled()
                },
            );
            let (lr, _) = ui.allocate_exact_size(Vec2::new(40.0, 20.0), Sense::hover());
            ui.painter().text(
                lr.left_center(),
                egui::Align2::LEFT_CENTER,
                format!("{label}:"),
                egui::FontId::proportional(12.0),
                if emu.is_some() { crate::theme::text() } else { crate::theme::disabled() },
            );
            let shown = emu.map(fmt_inches).unwrap_or_default();
            let id = ui
                .scope_id()
                .with(("size_box", label, self.selected().map(|s| s.id)));
            let mut text = ui
                .data_mut(|d| d.get_temp::<String>(id))
                .unwrap_or_else(|| shown.clone());
            let on = emu.is_some();
            // One bordered field; the stacked spinner arrows sit inside its right end.
            let (field, _) = ui.allocate_exact_size(Vec2::new(78.0, 22.0), Sense::hover());
            ui.painter().rect_filled(field, 2.0, Color32::WHITE);
            ui.painter().rect_stroke(
                field,
                2.0,
                egui::Stroke::new(
                    1.0,
                    if on {
                        Color32::from_gray(0x8A)
                    } else {
                        crate::theme::button_outline()
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let edit_rect = egui::Rect::from_min_max(
                field.min + Vec2::new(3.0, 1.0),
                field.max - Vec2::new(17.0, 1.0),
            );
            #[cfg(test)]
            if ui.is_visible() {
                crate::backstage::seen(format!("size:{label}"), field, on, false);
            }
            let r = ui.put(
                edit_rect,
                egui::TextEdit::singleline(&mut text)
                    .id(id.with("edit"))
                    .frame(egui::Frame::NONE)
                    .interactive(on),
            );
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, text.clone()));
            } else {
                ui.data_mut(|d| d.remove_temp::<String>(id));
            }
            if r.lost_focus() {
                out = parse_inches(&text);
            }
            for (up, dir) in [(true, 1.0), (false, -1.0)] {
                let top = field.min.y + if up { 1.0 } else { 11.0 };
                let rect = egui::Rect::from_min_size(
                    egui::pos2(field.max.x - 16.0, top),
                    Vec2::new(15.0, 10.0),
                );
                let sense = if on { Sense::click() } else { Sense::hover() };
                let resp = ui.interact(rect, id.with(("spin", up)), sense);
                #[cfg(test)]
                if ui.is_visible() {
                    crate::backstage::seen(
                        format!("size:{label}:{}", if up { "up" } else { "down" }),
                        rect,
                        on,
                        false,
                    );
                }
                if resp.hovered() && on {
                    ui.painter().rect_filled(rect, 1.0, crate::theme::hover_bg());
                }
                let c = rect.center();
                let s = if up { -1.0 } else { 1.0 };
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        c + Vec2::new(-3.0, 1.5 * s),
                        c + Vec2::new(3.0, 1.5 * s),
                        c + Vec2::new(0.0, -1.5 * s),
                    ],
                    if on { crate::theme::text() } else { crate::theme::disabled() },
                    egui::Stroke::NONE,
                ));
                if resp.clicked() {
                    if let Some(e) = emu {
                        out = Some((e + in_to_emu(STEP_IN * dir)).max(in_to_emu(MIN_IN)));
                    }
                }
            }
        });
        out
    }

    /// Resize the single selected object through the same command the canvas handles use.
    fn resize_selected(&mut self, w: Option<i64>, h: Option<i64>) {
        let Some(shape) = self.selected() else { return };
        let (id, mut rect): (_, Rect) = (shape.id, shape.rect);
        if let Some(w) = w {
            rect.w = w;
        }
        if let Some(h) = h {
            rect.h = h;
        }
        let page = self.page;
        self.exec(Command::SetRect { page, id, rect });
    }

    /// The Size group: Height and Width of the selected object (one object at a time).
    fn size_group(&mut self, ui: &mut Ui, measurement: bool, grow_to_fit: bool) {
        let dims = self.selected().map(|s| (s.rect.h, s.rect.w));
        let (mut nh, mut nw) = (None, None);
        group(ui, "Size", true, |ui| {
            crate::widgets::col(ui, |ui| {
                if grow_to_fit {
                    let mut on = false;
                    check(ui, "Grow to Fit Text", &mut on, false);
                }
                nh = self.size_box(ui, "Height", Icon::Size, dims.map(|d| d.0));
                nw = self.size_box(ui, "Width", Icon::Size, dims.map(|d| d.1));
                if measurement {
                    soon(ui, Icon::Size, "Measurement", false);
                }
            });
        });
        if nh.is_some() || nw.is_some() {
            self.resize_selected(nw, nh);
        }
    }

    /// Arrange with large buttons (all contextual tabs but Picture Format).
    fn arrange_group(&mut self, ui: &mut Ui, small_rest: bool) {
        let has_sel = !self.selected_ids().is_empty();
        group(ui, "Arrange", false, |ui| {
            if small_rest {
                soon(ui, Icon::Table, "Arrange\nThumbnails", true);
            }
            self.wrap_menu(ui);
            if small_rest {
                crate::widgets::col(ui, |ui| {
                    self.z_order_buttons(ui, has_sel, false);
                    self.align_menu(ui, has_sel, false);
                });
                crate::widgets::col(ui, |ui| {
                    self.group_buttons(ui, false);
                    self.rotate_menu(ui, false);
                });
            } else {
                self.z_order_buttons(ui, has_sel, true);
                self.align_menu(ui, has_sel, true);
                self.group_buttons(ui, true);
                self.rotate_menu(ui, true);
            }
        });
    }

    fn group_buttons(&mut self, ui: &mut Ui, large: bool) {
        let (can_group, can_ungroup) = (self.can_group(), self.can_ungroup());
        for (label, on, tip) in [
            ("Group", can_group, "Group (Ctrl+Shift+G)"),
            ("Ungroup", can_ungroup, "Ungroup (Ctrl+Shift+H)"),
        ] {
            let icon = if label == "Ungroup" { Icon::Ungroup } else { Icon::Group };
            let b = if large {
                Btn::large(icon, label)
            } else {
                Btn::small(icon, label)
            };
            if b.enabled(on).show(ui).on_hover_text(tip).clicked() {
                if label == "Group" {
                    self.group_selection();
                } else {
                    self.ungroup_selection();
                }
            }
        }
    }

    pub(crate) fn shape_format_tab(&mut self, ui: &mut Ui) {
        let text_box = self
            .selected()
            .filter(|s| matches!(s.kind, ShapeKind::TextBox { .. }))
            .map(|s| s.id);
        group(ui, "Insert Shapes", false, |ui| {
            self.shapes_gallery(ui);
            crate::widgets::col(ui, |ui| {
                soon_dd(ui, Icon::Shapes, "Edit Shape", false);
                soon_dd(ui, Icon::Shapes, "Change Shape", false);
                if Btn::small(Icon::TextBox, "Edit Text")
                    .enabled(text_box.is_some())
                    .show(ui)
                    .clicked()
                {
                    if let Some(id) = text_box {
                        self.begin_edit(id);
                    }
                }
            });
        });
        group(ui, "Shape Styles", true, |ui| {
            gallery(ui, 230.0);
            crate::widgets::col(ui, |ui| {
                self.fill_outline_buttons(ui);
                soon_dd(ui, Icon::Shading, "Shape Effects", false);
            });
        });
        self.arrange_group(ui, false);
        self.size_group(ui, true, false);
    }

    pub(crate) fn text_box_tab(&mut self, ui: &mut Ui) {
        group(ui, "Text", true, |ui| {
            soon_dd(ui, Icon::TextBox, "Text\nFit", true);
            soon(ui, Icon::Orientation, "Text\nDirection", true);
            soon(ui, Icon::Abc, "Hyphen-\nation", true);
        });
        self.font_group(ui, "Font");
        group(ui, "Alignment", false, |ui| {
            anchor_grid(ui);
            self.columns_menu(ui, true);
            soon_dd(ui, Icon::Margins, "Margins", true);
        });
        group(ui, "Linking", false, |ui| {
            // Create Link and Break show Publisher's enabled states but do nothing yet:
            // saving a chain someone made needs a writer for the links (docs/design/LINKED-STORIES.md, phase 7).
            let st = self.link_state();
            Btn::large(Icon::Link, "Create\nLink")
                .enabled(st.create)
                .tip(crate::widgets::COMING_SOON)
                .show(ui);
            crate::widgets::col(ui, |ui| {
                Btn::small(Icon::Link, "Break")
                    .enabled(st.brk)
                    .tip(crate::widgets::COMING_SOON)
                    .show(ui);
                if Btn::small(Icon::Arrow, "Next").enabled(st.next).show(ui).clicked() {
                    self.linked_step(ui.ctx(), true);
                }
                if Btn::small(Icon::Back, "Previous").enabled(st.prev).show(ui).clicked() {
                    self.linked_step(ui.ctx(), false);
                }
            });
        });
        group(ui, "Text Styles", true, |ui| {
            gallery(ui, 110.0);
            crate::widgets::col(ui, |ui| {
                soon_split(ui, Icon::FontColor, "Text Fill");
                soon_split(ui, Icon::Frame, "Text Outline");
                soon_dd(ui, Icon::Shading, "Text Effects", false);
            });
        });
        group(ui, "Typography", true, |ui| {
            soon_dd(ui, Icon::Abc, "Drop\nCap", true);
        });
    }

    /// Adjust > Change Picture: the button runs "Change Picture...", the arrow also offers
    /// "Remove Picture".
    fn change_picture_button(&mut self, ui: &mut Ui) {
        let one = self.selected_picture().is_some();
        let (main, arrow) =
            crate::widgets::split_button(ui, Btn::small(Icon::Picture, "Change Picture").enabled(one));
        let (mut change, mut remove) = (main.clicked(), false);
        egui::Popup::menu(&arrow).show(|ui| {
            ui.set_min_width(160.0);
            if ui.button("Change Picture...").clicked() {
                change = true;
                ui.close();
            }
            if ui.button("Remove Picture").clicked() {
                remove = true;
                ui.close();
            }
        });
        if change {
            self.change_picture_dialog();
        } else if remove {
            self.remove_picture();
        }
    }

    pub(crate) fn picture_format_tab(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        group(ui, "Insert", false, |ui| {
            if Btn::large(Icon::Picture, "Pictures...")
                .enabled(has_doc)
                .show(ui)
                .clicked()
            {
                self.insert_picture_dialog();
            }
        });
        group(ui, "Adjust", true, |ui| {
            soon_dd(ui, Icon::Palette, "Corrections", true);
            soon_dd(ui, Icon::Palette, "Recolor", true);
            crate::widgets::col(ui, |ui| {
                soon(ui, Icon::Picture, "Compress Pictures...", false);
                self.change_picture_button(ui);
                soon(ui, Icon::Replace, "Reset Picture", false);
            });
        });
        group(ui, "Swap", false, |ui| {
            soon_dd(ui, Icon::Replace, "Swap", true);
        });
        group(ui, "Picture Styles", true, |ui| {
            gallery(ui, 230.0);
            crate::widgets::col(ui, |ui| {
                soon_split(ui, Icon::Frame, "Picture Border");
                soon_dd(ui, Icon::Shading, "Picture Effects", false);
                soon_dd(ui, Icon::Pilcrow, "Caption", false);
            });
        });
        self.arrange_group(ui, true);
        group(ui, "Crop", false, |ui| {
            soon_split(ui, Icon::Frame, "Crop");
            crate::widgets::col(ui, |ui| {
                soon(ui, Icon::FitPage, "Fit", false);
                soon(ui, Icon::PageWidth, "Fill", false);
                soon(ui, Icon::ClearFormat, "Clear Crop", false);
            });
        });
        self.size_group(ui, false, false);
    }

    pub(crate) fn table_design_tab(ui: &mut Ui) {
        group(ui, "Table Formats", true, |ui| {
            gallery(ui, 230.0);
            soon_split(ui, Icon::Palette, "Fill");
        });
        group(ui, "Borders", false, |ui| {
            crate::widgets::col(ui, |ui| {
                soon_dd(ui, Icon::Spacing, "Line Weight", false);
                soon_dd(ui, Icon::Palette, "Line Color", false);
                soon_split(ui, Icon::Frame, "Borders");
            });
        });
        group(ui, "Text Styles", true, |ui| {
            gallery(ui, 110.0);
            crate::widgets::col(ui, |ui| {
                soon_split(ui, Icon::FontColor, "Text Fill");
                soon_split(ui, Icon::Frame, "Text Outline");
                soon_dd(ui, Icon::Shading, "Text Effects", false);
            });
        });
        group(ui, "Typography", true, |ui| {
            crate::widgets::col(ui, |ui| {
                soon_dd(ui, Icon::Abc, "Drop Cap", false);
                soon_dd(ui, Icon::Hash, "Number Style", false);
                soon_dd(ui, Icon::Abc, "Ligatures", false);
            });
        });
    }

    pub(crate) fn table_layout_tab(&mut self, ui: &mut Ui) {
        group(ui, "Table", false, |ui| {
            soon_dd(ui, Icon::Select, "Select", true);
            soon(ui, Icon::Table, "View\nGridlines", true);
        });
        self.rows_columns_group(ui);
        group(ui, "Merge", false, |ui| {
            soon(ui, Icon::Table, "Merge\nCells", true);
            soon(ui, Icon::Table, "Split\nCells...", true);
            soon_dd(ui, Icon::Table, "Diagonals", true);
        });
        group(ui, "Alignment", true, |ui| {
            anchor_grid(ui);
            soon(ui, Icon::Orientation, "Text\nDirection", true);
            soon(ui, Icon::Abc, "Hyphen-\nation", true);
            soon_dd(ui, Icon::Margins, "Cell\nMargins", true);
        });
        self.arrange_group(ui, false);
        self.size_group(ui, false, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inch_text_round_trips() {
        assert_eq!(fmt_inches(in_to_emu(7.0)), "7\"");
        assert_eq!(fmt_inches(in_to_emu(1.25)), "1.25\"");
        assert_eq!(parse_inches("2.5\""), Some(in_to_emu(2.5)));
        assert_eq!(parse_inches(" 3 in"), Some(in_to_emu(3.0)));
        assert_eq!(parse_inches("abc"), None);
        assert_eq!(parse_inches("0"), None);
    }
}
