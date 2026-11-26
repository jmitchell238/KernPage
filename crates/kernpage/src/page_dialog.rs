//! The Page Setup dialog and the Layout Guides (Custom Margins) dialog of Publisher 2021
//! (docs/ui/PAGE-SETUP.md sections 5 and 8). The state, validation and the command to run
//! are plain code (tested below); only `show` touches egui.

use crate::app::App;
use crate::widgets::ACCENT_COLOR;
use eframe::egui::{self, Color32, Key, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use kernpage_core::pagesetup::{inches_text, parse_length, size_text};
use kernpage_core::{Command, Margins};

const EMU_PER_INCH: i64 = kernpage_core::model::EMU_PER_INCH;
const ERROR: Color32 = Color32::from_rgb(0xC0, 0x30, 0x30);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DialogKind {
    /// "Page Setup": page size and margin guides.
    PageSetup,
    /// "Layout Guides" (Margins > Custom Margins...): margin guides only.
    CustomMargins,
}

/// Which text box a message belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldId {
    Width,
    Height,
    Top,
    Left,
    Bottom,
    Right,
}

/// One length box: what is typed, and the exact value it started with (so an untouched
/// box never rounds the file's value to three decimals).
#[derive(Clone, Debug)]
pub struct Field {
    pub text: String,
    initial: String,
    orig: i64,
}

pub fn inch_text(emu: i64) -> String {
    format!("{}\"", inches_text(emu))
}

impl Field {
    pub fn new(emu: i64) -> Field {
        let t = inch_text(emu);
        Field {
            text: t.clone(),
            initial: t,
            orig: emu,
        }
    }

    pub fn value(&self) -> Result<i64, String> {
        if self.text == self.initial {
            Ok(self.orig)
        } else {
            parse_length(&self.text)
        }
    }

    /// The spin arrows: add `delta` EMU to the current value (when it can be read).
    pub fn step(&mut self, delta: i64) {
        if let Ok(v) = self.value() {
            self.text = inch_text((v + delta).max(0));
        }
    }
}

/// Messages per field, in plain English.
#[derive(Default, Debug, PartialEq)]
pub struct Errors(pub Vec<(FieldId, String)>);

impl Errors {
    pub fn of(&self, f: FieldId) -> Option<&str> {
        self.0
            .iter()
            .find(|(id, _)| *id == f)
            .map(|(_, m)| m.as_str())
    }
}

pub struct PageSetupDialog {
    pub kind: DialogKind,
    pub width: Field,
    pub height: Field,
    pub top: Field,
    pub left: Field,
    pub bottom: Field,
    pub right: Field,
    /// Select the first box on the first frame.
    pub fresh: bool,
}

impl PageSetupDialog {
    pub fn new(kind: DialogKind, size: (i64, i64), m: Margins) -> Self {
        PageSetupDialog {
            kind,
            width: Field::new(size.0),
            height: Field::new(size.1),
            top: Field::new(m.top),
            left: Field::new(m.left),
            bottom: Field::new(m.bottom),
            right: Field::new(m.right),
            fresh: true,
        }
    }

    pub fn field(&mut self, f: FieldId) -> &mut Field {
        match f {
            FieldId::Width => &mut self.width,
            FieldId::Height => &mut self.height,
            FieldId::Top => &mut self.top,
            FieldId::Left => &mut self.left,
            FieldId::Bottom => &mut self.bottom,
            FieldId::Right => &mut self.right,
        }
    }

    /// Check every box. Ok = the size and margins to apply.
    pub fn validate(&self) -> Result<((i64, i64), Margins), Errors> {
        let mut errs = Errors::default();
        let mut get = |f: &Field, id: FieldId| match f.value() {
            Ok(v) => Some(v),
            Err(m) => {
                errs.0.push((id, m));
                None
            }
        };
        let w = get(&self.width, FieldId::Width);
        let h = get(&self.height, FieldId::Height);
        let t = get(&self.top, FieldId::Top);
        let l = get(&self.left, FieldId::Left);
        let b = get(&self.bottom, FieldId::Bottom);
        let r = get(&self.right, FieldId::Right);
        let range = EMU_PER_INCH..=49 * EMU_PER_INCH;
        for (v, id, what) in [(w, FieldId::Width, "width"), (h, FieldId::Height, "height")] {
            if v.is_some_and(|v| !range.contains(&v)) {
                errs.0
                    .push((id, format!("The page {what} must be between 1\" and 49\".")));
            }
        }
        for (v, id) in [
            (t, FieldId::Top),
            (l, FieldId::Left),
            (b, FieldId::Bottom),
            (r, FieldId::Right),
        ] {
            if v.is_some_and(|v| v < 0) {
                errs.0.push((id, "A margin cannot be negative.".to_owned()));
            }
        }
        if errs.0.is_empty() {
            let (w, h) = (w.unwrap_or(0), h.unwrap_or(0));
            let m = Margins {
                left: l.unwrap_or(0),
                right: r.unwrap_or(0),
                top: t.unwrap_or(0),
                bottom: b.unwrap_or(0),
            };
            if m.left + m.right >= w {
                errs.0.push((
                    FieldId::Right,
                    "The left and right margins must leave room on the page.".to_owned(),
                ));
            }
            if m.top + m.bottom >= h {
                errs.0.push((
                    FieldId::Bottom,
                    "The top and bottom margins must leave room on the page.".to_owned(),
                ));
            }
            if errs.0.is_empty() {
                return Ok(((w, h), m));
            }
        }
        Err(errs)
    }

    /// The command OK runs: one undo step.
    pub fn command(&self) -> Result<Command, Errors> {
        let ((w, h), margins) = self.validate()?;
        Ok(match self.kind {
            DialogKind::PageSetup => Command::SetPageSetup {
                width_emu: w,
                height_emu: h,
                margins,
            },
            DialogKind::CustomMargins => Command::SetMargins { margins },
        })
    }

    /// Values for the preview: what is valid now, else what the box started with.
    fn preview(&self) -> ((i64, i64), Margins) {
        let g = |f: &Field| f.value().unwrap_or(f.orig).max(0);
        (
            (g(&self.width).max(1), g(&self.height).max(1)),
            Margins {
                left: g(&self.left),
                right: g(&self.right),
                top: g(&self.top),
                bottom: g(&self.bottom),
            },
        )
    }
}

/// A length box with the spin arrows: label, text box, up/down. Returns the text response.
fn spin(ui: &mut Ui, label: &str, f: &mut Field, err: Option<&str>) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([64.0, 20.0], egui::Label::new(label));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut f.text)
                .desired_width(80.0)
                .text_color(if err.is_some() { ERROR } else { crate::theme::text() }),
        );
        let step = EMU_PER_INCH / 10;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for (up, delta) in [(true, step), (false, -step)] {
                // Not focusable: Tab goes from box to box.
                let (r, a) = ui.allocate_exact_size(Vec2::new(16.0, 10.0), Sense::CLICK);
                let bg = if a.hovered() {
                    ACCENT_COLOR.gamma_multiply(0.3)
                } else {
                    Color32::from_gray(228)
                };
                ui.painter().rect_filled(r, 1.0, bg);
                let c = r.center();
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
                    f.step(delta);
                }
            }
        });
        resp
    })
    .inner
}

fn error_line(ui: &mut Ui, err: Option<&str>) {
    if let Some(e) = err {
        ui.horizontal(|ui| {
            ui.add_space(68.0);
            ui.add(egui::Label::new(egui::RichText::new(e).size(11.0).color(ERROR)).wrap());
        });
    }
}

/// The page outline with the margin guides in teal, scaled into a 170 x 170 box.
fn preview(ui: &mut Ui, size: (i64, i64), m: Margins) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(160.0, 160.0), Sense::hover());
    let scale = (140.0 / size.0 as f32).min(140.0 / size.1 as f32);
    let page = Rect::from_center_size(
        rect.center(),
        Vec2::new(size.0 as f32 * scale, size.1 as f32 * scale),
    );
    let p = ui.painter();
    p.rect_filled(
        page.translate(Vec2::new(3.0, 3.0)),
        0.0,
        Color32::from_black_alpha(40),
    );
    p.rect_filled(page, 0.0, Color32::WHITE);
    p.rect_stroke(
        page,
        0.0,
        Stroke::new(1.0, Color32::from_gray(90)),
        StrokeKind::Inside,
    );
    let g = Rect::from_min_max(
        page.min + Vec2::new(m.left as f32 * scale, m.top as f32 * scale),
        page.max - Vec2::new(m.right as f32 * scale, m.bottom as f32 * scale),
    );
    if g.width() > 0.0 && g.height() > 0.0 {
        p.rect_stroke(g, 0.0, Stroke::new(1.0, ACCENT_COLOR), StrokeKind::Inside);
    }
}

fn disabled_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_enabled_ui(false, |ui| {
            ui.add_sized([100.0, 18.0], egui::Label::new(label));
            ui.add(egui::TextEdit::singleline(&mut value.to_owned()).desired_width(60.0));
        });
    });
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).strong().color(crate::theme::text()));
}

impl App {
    pub fn open_page_dialog(&mut self, kind: DialogKind) {
        let Some(p) = self
            .editor
            .as_ref()
            .and_then(|e| e.doc().pages.first().cloned())
        else {
            return;
        };
        self.page_dialog = Some(PageSetupDialog::new(
            kind,
            (p.width_emu, p.height_emu),
            p.margins_emu,
        ));
    }

    pub(crate) fn page_dialog_ui(&mut self, ctx: &egui::Context) {
        // The Page Setup group's launcher asks through the context.
        #[allow(deprecated)]
        let asked = ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new("open_page_setup")));
        if asked == Some(true) {
            self.open_page_dialog(DialogKind::PageSetup);
        }
        let Some(d) = &mut self.page_dialog else {
            return;
        };
        let (mut ok, mut cancel) = (false, false);
        let errors = d.validate().err().unwrap_or_default();
        let kind = d.kind;
        let title = match kind {
            DialogKind::PageSetup => "Page Setup",
            DialogKind::CustomMargins => "Layout Guides",
        };
        let fresh = std::mem::take(&mut d.fresh);
        #[allow(deprecated)]
        let modal_id = egui::Id::new("page_dialog");
        egui::Modal::new(modal_id).show(ctx, |ui| {
            ui.heading(title);
            ui.separator();
            if kind == DialogKind::CustomMargins {
                ui.horizontal(|ui| {
                    let _ = ui.selectable_label(true, "Margin Guides");
                    ui.add_enabled(false, egui::Button::selectable(false, "Grid Guides"))
                        .on_disabled_hover_text("Not available yet");
                    ui.add_enabled(false, egui::Button::selectable(false, "Baseline Guides"))
                        .on_disabled_hover_text("Not available yet");
                });
                ui.separator();
            }
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    if kind == DialogKind::CustomMargins {
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            heading(ui, "Master Pages");
                            ui.add_enabled(
                                false,
                                egui::Checkbox::new(&mut false, "Two-page master"),
                            )
                            .on_disabled_hover_text("Two-page masters cannot be saved yet");
                        });
                    } else {
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            heading(ui, "Page");
                            let r = spin(ui, "Width:", &mut d.width, errors.of(FieldId::Width));
                            if fresh {
                                r.request_focus();
                                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                                    st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(d.width.text.chars().count()),
                                    )));
                                    st.store(ui.ctx(), r.id);
                                }
                            }
                            error_line(ui, errors.of(FieldId::Width));
                            spin(ui, "Height:", &mut d.height, errors.of(FieldId::Height));
                            error_line(ui, errors.of(FieldId::Height));
                        });
                    }
                    ui.group(|ui| {
                        ui.set_width(230.0);
                        heading(ui, "Margin guides");
                        let order = if kind == DialogKind::PageSetup {
                            [
                                ("Top:", FieldId::Top),
                                ("Left:", FieldId::Left),
                                ("Bottom:", FieldId::Bottom),
                                ("Right:", FieldId::Right),
                            ]
                        } else {
                            [
                                ("Left:", FieldId::Left),
                                ("Right:", FieldId::Right),
                                ("Top:", FieldId::Top),
                                ("Bottom:", FieldId::Bottom),
                            ]
                        };
                        for (label, id) in order {
                            spin(ui, label, d.field(id), errors.of(id));
                            error_line(ui, errors.of(id));
                        }
                    });
                });
                if kind == DialogKind::PageSetup {
                    ui.vertical(|ui| {
                        ui.set_width(230.0);
                        ui.group(|ui| {
                            ui.set_width(210.0);
                            heading(ui, "Layout type");
                            egui::ComboBox::from_id_salt("layout_type")
                                .selected_text("One page per sheet")
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    let _ = ui.selectable_label(true, "One page per sheet");
                                    for t in [
                                        "Booklet",
                                        "Email",
                                        "Envelope",
                                        "Folded card",
                                        "Multiple pages per sheet",
                                        "Web page",
                                    ] {
                                        ui.add_enabled(false, egui::Button::selectable(false, t));
                                    }
                                });
                        });
                        ui.group(|ui| {
                            ui.set_width(210.0);
                            heading(ui, "Options");
                            ui.add_enabled_ui(false, |ui| {
                                ui.label("Target paper size:");
                                egui::ComboBox::from_id_salt("paper")
                                    .selected_text("Custom")
                                    .width(180.0)
                                    .show_ui(ui, |_| {});
                            });
                            let (w, h) = (d.width.text.clone(), d.height.text.clone());
                            disabled_row(ui, "Paper width:", &w);
                            disabled_row(ui, "Paper height:", &h);
                            disabled_row(ui, "Side margin:", "0\"");
                            disabled_row(ui, "Top margin:", "0\"");
                            disabled_row(ui, "Horizontal gap:", "0\"");
                            disabled_row(ui, "Vertical gap:", "0\"");
                        });
                    });
                }
                ui.vertical(|ui| {
                    ui.set_width(190.0);
                    ui.group(|ui| {
                        ui.set_width(170.0);
                        heading(ui, "Preview");
                        let (size, m) = d.preview();
                        preview(ui, size, m);
                        if kind == DialogKind::PageSetup {
                            let t = size_text(size.0, size.1);
                            ui.label(egui::RichText::new(format!("Page size: {t}")).size(11.5));
                            ui.label(egui::RichText::new(format!("Paper size: {t}")).size(11.5));
                            ui.label(egui::RichText::new("1 columns of 1").size(11.5));
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel = ui.button("  Cancel  ").clicked();
                ok = ui
                    .add(egui::Button::new("     OK     ").stroke(Stroke::new(1.5, ACCENT_COLOR)))
                    .clicked();
            });
        });
        ctx.input(|i| {
            if i.key_pressed(Key::Enter) {
                ok = true;
            }
            if i.key_pressed(Key::Escape) {
                cancel = true;
            }
        });
        if ok {
            if let Some(d) = &self.page_dialog {
                // Invalid input keeps the dialog open; the messages are already shown.
                if let Ok(cmd) = d.command() {
                    self.page_dialog = None;
                    self.exec(cmd);
                }
            }
        } else if cancel {
            self.page_dialog = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER: (i64, i64) = (7_772_400, 10_058_400);

    fn dlg(kind: DialogKind) -> PageSetupDialog {
        PageSetupDialog::new(kind, LETTER, Margins::all(457_200))
    }

    #[test]
    fn untouched_boxes_keep_exact_values() {
        let mut d = PageSetupDialog::new(
            DialogKind::PageSetup,
            (7_560_000, 10_692_000),
            Margins::all(457_200),
        );
        assert_eq!(d.width.text, "8.268\"");
        assert_eq!(d.validate().unwrap().0, (7_560_000, 10_692_000));
        d.left.text = "1.5".into();
        let (_, m) = d.validate().unwrap();
        assert_eq!(m.left, 1_371_600);
        assert_eq!(m.right, 457_200);
    }

    #[test]
    fn typed_units_set_a_custom_size() {
        let mut d = dlg(DialogKind::PageSetup);
        d.width.text = "5.5\"".into();
        d.height.text = "21.59 cm".into();
        let Command::SetPageSetup {
            width_emu,
            height_emu,
            margins,
        } = d.command().unwrap()
        else {
            panic!()
        };
        assert_eq!((width_emu, height_emu), (5_029_200, 7_772_400));
        assert_eq!(margins, Margins::all(457_200));
    }

    #[test]
    fn bad_input_is_reported_per_field() {
        let mut d = dlg(DialogKind::PageSetup);
        d.width.text = "wide".into();
        d.height.text = "60".into();
        d.top.text = "-1".into();
        let e = d.command().unwrap_err();
        assert!(e.of(FieldId::Width).unwrap().contains("not a length"));
        assert!(e
            .of(FieldId::Height)
            .unwrap()
            .contains("between 1\" and 49\""));
        assert!(e.of(FieldId::Top).unwrap().contains("negative"));
        assert!(e.of(FieldId::Left).is_none());
    }

    #[test]
    fn margins_must_leave_room() {
        let mut d = dlg(DialogKind::PageSetup);
        d.width.text = "2".into();
        d.left.text = "1".into();
        d.right.text = "1".into();
        let e = d.command().unwrap_err();
        assert!(e.of(FieldId::Right).unwrap().contains("leave room"));
        assert!(e.of(FieldId::Bottom).is_none());
    }

    #[test]
    fn custom_margins_runs_set_margins_only() {
        let mut d = dlg(DialogKind::CustomMargins);
        d.top.text = "0.75".into();
        assert_eq!(
            d.command().unwrap(),
            Command::SetMargins {
                margins: Margins {
                    left: 457_200,
                    right: 457_200,
                    top: 685_800,
                    bottom: 457_200
                }
            }
        );
    }

    #[test]
    fn spin_arrows_step_a_tenth_of_an_inch() {
        let mut f = Field::new(457_200);
        f.step(EMU_PER_INCH / 10);
        assert_eq!(f.text, "0.6\"");
        f.text = "junk".into();
        f.step(1);
        assert_eq!(f.text, "junk");
    }
}
