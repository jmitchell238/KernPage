//! The ribbon: one function per tab, groups exactly as listed in the UI checklist.

use crate::app::{App, Tab, Tool, ZoomMode};
use crate::icons::Icon;
use crate::ribbon_fit as fit;
use crate::widgets::{self, check, group, Btn, ACCENT_COLOR, COMING_SOON};
use eframe::egui::{self, Color32, Popup, RichText, Ui};
use kernpage_core::text::SpacingChange;
use kernpage_core::{Align, CharChange, LineSpacing, Rgb, Script};

const SIZES: [f32; 17] = [
    8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 60.0,
    72.0,
];

/// The sizes Grow Font and Shrink Font step through.
const GROW_STEPS: [f32; 17] = [
    8.0, 9.0, 10.0, 10.5, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0,
    72.0,
];

/// Width of the Font Size box in points: wide enough for "1638" and "10.5" with margins.
pub(crate) const SIZE_BOX_W: f32 = 52.0;
/// Width of the Font Name box.
pub(crate) const FONT_BOX_W: f32 = 132.0;

fn fmt_size(s: f32) -> String {
    if (s - s.round()).abs() < 0.05 {
        format!("{}", s.round() as i32)
    } else {
        format!("{s:.1}")
    }
}

/// Tooltip of the greyed Insert > Online Pictures button.
pub(crate) const ONLINE_PICTURES_TIP: &str =
    "Coming soon: search free-to-use photo sites and insert a picture.";

/// A disabled placeholder button.
pub(crate) fn soon(ui: &mut Ui, icon: Icon, label: &str, large: bool) {
    let b = if large {
        Btn::large(icon, label)
    } else {
        Btn::small(icon, label)
    };
    b.disabled().show(ui);
}

/// A disabled placeholder with a drop-down arrow.
pub(crate) fn soon_dd(ui: &mut Ui, icon: Icon, label: &str, large: bool) {
    let b = if large {
        Btn::large(icon, label)
    } else {
        Btn::small(icon, label)
    };
    b.dropdown().disabled().show(ui);
}

/// A disabled placeholder split button (main part plus a separate arrow).
pub(crate) fn soon_split(ui: &mut Ui, icon: Icon, label: &str) {
    widgets::split_button(ui, Btn::small(icon, label).disabled());
}

impl App {
    /// Draw the current tab's groups, scaled to fit `avail` points of width (see `ribbon_fit`).
    pub fn ribbon(&mut self, ui: &mut Ui, avail: f32) {
        let key = format!("{:?}", self.tab);
        if fit::cached(&key).is_none() {
            // First visit: measure every group at every size level in hidden passes.
            let mut per_level = Vec::new();
            for level in 0..fit::LEVELS {
                fit::begin_measure(level);
                let area = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(20000.0, 100.0));
                let mut hidden = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("ribbon_measure", level))
                        .max_rect(area)
                        .invisible(),
                );
                self.tab_body(&mut hidden);
                per_level.push(fit::end_measure());
            }
            fit::store(&key, &per_level);
        }
        let specs = fit::cached(&key).unwrap_or_default();
        fit::begin_render(fit::pick_variants(&specs, avail), fit::cached_titles(&key));
        self.tab_body(ui);
        if fit::end_render(&key) {
            ui.ctx().request_repaint();
        }
    }

    fn tab_body(&mut self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            match self.tab {
                Tab::Home => self.home_tab(ui),
                Tab::Insert => self.insert_tab(ui),
                Tab::PageDesign => self.page_design_tab(ui),
                Tab::Data => Self::data_tab(ui),
                Tab::Review => Self::review_tab(ui),
                Tab::View => self.view_tab(ui),
                Tab::Help => self.help_tab(ui),
                Tab::ShapeFormat => self.shape_format_tab(ui),
                Tab::TextBox => self.text_box_tab(ui),
                Tab::PictureFormat => self.picture_format_tab(ui),
                Tab::TableDesign => Self::table_design_tab(ui),
                Tab::Layout => self.table_layout_tab(ui),
                Tab::MasterPage => self.master_page_tab(ui),
            }
        });
    }

    pub(crate) fn start_tool(&mut self, tool: Tool) {
        self.commit_edit();
        self.tool = if self.tool == tool {
            Tool::Select
        } else {
            tool
        };
        self.selection = None;
    }

    pub(crate) fn shapes_menu(&mut self, ui: &mut Ui, large: bool) {
        let b = if large {
            Btn::large(Icon::Shapes, "Shapes").dropdown()
        } else {
            Btn::small(Icon::Shapes, "Shapes").dropdown()
        };
        let resp = b.show(ui);
        Popup::menu(&resp).show(|ui| {
            ui.set_min_width(120.0);
            for (label, tool) in [
                ("Rectangle", Tool::Rectangle),
                ("Oval", Tool::Oval),
                ("Line", Tool::Line),
            ] {
                if ui.button(label).clicked() {
                    self.start_tool(tool);
                    ui.close();
                }
            }
        });
    }

    /// The Font group (shared by Home and the Text Box tab): the same actions in both places.
    pub(crate) fn font_group(&mut self, ui: &mut Ui, title: &str) {
        let info = self.format_info(ui.ctx());
        let fmt = info.as_ref().map(|i| i.fmt.clone()).unwrap_or_default();
        let can_char = info.as_ref().is_some_and(|i| !i.caret);
        group(ui, title, true, |ui| {
            let ctx = ui.ctx().clone();
            widgets::col2(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 2.0);
                    let cur = fmt.font.clone().unwrap_or_default();
                    let mut chosen = None;
                    let mut name_text = std::mem::take(&mut self.font_text);
                    let mut name_typing = self.font_typing;
                    let typed = widgets::combo_field(
                        ui,
                        "font_name",
                        "Font Name",
                        FONT_BOX_W,
                        &mut name_text,
                        &mut name_typing,
                        &cur,
                        can_char,
                        |ui| {
                            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                for fam in &self.fonts.families {
                                    if ui.selectable_label(*fam == cur, fam).clicked() {
                                        chosen = Some(fam.clone());
                                        ui.close();
                                    }
                                }
                            });
                        },
                    );
                    self.font_text = name_text;
                    self.font_typing = name_typing;
                    if let Some(t) = typed.filter(|t| !t.is_empty()) {
                        let lc = t.to_lowercase();
                        let fams = &self.fonts.families;
                        chosen = fams
                            .iter()
                            .find(|f| f.to_lowercase() == lc)
                            .or_else(|| fams.iter().find(|f| f.to_lowercase().starts_with(&lc)))
                            .cloned();
                    }
                    if let Some(f) = chosen {
                        self.format_chars(&ctx, CharChange::FontName(f));
                    }
                    // Size: type in the box, or pick from the list.
                    let cur_size = fmt.size;
                    let mut chosen = None;
                    let shown = cur_size.map(fmt_size).unwrap_or_default();
                    let mut size_text = std::mem::take(&mut self.size_text);
                    let mut size_typing = self.size_box_typing;
                    let typed = widgets::combo_field(
                        ui,
                        "font_size_box",
                        "Font Size",
                        SIZE_BOX_W,
                        &mut size_text,
                        &mut size_typing,
                        &shown,
                        can_char,
                        |ui| {
                            for s in SIZES {
                                if ui
                                    .selectable_label(
                                        cur_size.is_some_and(|c| (s - c).abs() < 0.05),
                                        fmt_size(s),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(s);
                                    ui.close();
                                }
                            }
                        },
                    );
                    self.size_text = size_text;
                    self.size_box_typing = size_typing;
                    if let Some(t) = typed {
                        chosen = t.parse::<f32>().ok();
                    }
                    if let Some(s) = chosen.filter(|s| (1.0..=1638.0).contains(s)) {
                        self.format_chars(&ctx, CharChange::Size(s));
                    }
                    // Grow / Shrink step from the first selected size when sizes are mixed.
                    let base = cur_size.unwrap_or_else(|| {
                        info.as_ref()
                            .map_or(0.0, |i| self.first_selected_size(&ctx, i))
                    });
                    let grow = GROW_STEPS.iter().copied().find(|&s| s > base + 0.05);
                    let shrink = GROW_STEPS.iter().rev().copied().find(|&s| s < base - 0.05);
                    if Btn::small(Icon::GrowFont, "")
                        .enabled(can_char && grow.is_some())
                        .tip("Grow Font")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Size(grow.unwrap_or(base)));
                    }
                    if Btn::small(Icon::ShrinkFont, "")
                        .enabled(can_char && shrink.is_some())
                        .tip("Shrink Font")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Size(shrink.unwrap_or(base)));
                    }
                    widgets::divider(ui);
                    if Btn::small(Icon::ClearFormat, "")
                        .enabled(can_char)
                        .tip("Clear Formatting")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Clear);
                    }
                });
                ui.horizontal(|ui| {
                    if Btn::small(Icon::Bold, "")
                        .enabled(can_char)
                        .selected(fmt.bold == Some(true))
                        .tip("Bold (Ctrl+B)")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Bold(fmt.bold != Some(true)));
                    }
                    if Btn::small(Icon::Italic, "")
                        .enabled(can_char)
                        .selected(fmt.italic == Some(true))
                        .tip("Italic (Ctrl+I)")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Italic(fmt.italic != Some(true)));
                    }
                    if Btn::small(Icon::Underline, "")
                        .enabled(can_char)
                        .selected(fmt.underline == Some(true))
                        .tip("Underline (Ctrl+U)")
                        .show(ui)
                        .clicked()
                    {
                        self.format_chars(&ctx, CharChange::Underline(fmt.underline != Some(true)));
                    }
                    if Btn::small(Icon::Sub, "")
                        .enabled(can_char)
                        .selected(fmt.script == Some(Script::Subscript))
                        .tip("Subscript (Ctrl+=)")
                        .show(ui)
                        .clicked()
                    {
                        self.toggle_script(&ctx, Script::Subscript);
                    }
                    if Btn::small(Icon::Sup, "")
                        .enabled(can_char)
                        .selected(fmt.script == Some(Script::Superscript))
                        .tip("Superscript (Ctrl+Shift+=)")
                        .show(ui)
                        .clicked()
                    {
                        self.toggle_script(&ctx, Script::Superscript);
                    }
                    soon_dd(ui, Icon::Case, "", false);
                    widgets::divider(ui);
                    soon_dd(ui, Icon::Spacing, "", false);
                    widgets::divider(ui);
                    let (main, r) = widgets::split_button(
                        ui,
                        Btn::small(Icon::FontColorLetter, "")
                            .enabled(can_char)
                            .tip("Font Color"),
                    );
                    // The one colour bar: where the icon's own bar sits (bottom of its 16 x 16
                    // rect, x 1.5..14.5, y 13..15.5), in the last font colour, or grey when disabled.
                    let bar = if can_char {
                        Color32::from_rgb(
                            self.last_font_color.0,
                            self.last_font_color.1,
                            self.last_font_color.2,
                        )
                    } else {
                        crate::theme::disabled()
                    };
                    let ppp = ui.ctx().pixels_per_point();
                    let c0 = main.rect.center();
                    let ic = egui::pos2((c0.x * ppp).round() / ppp, (c0.y * ppp).round() / ppp);
                    let at = |x: f32, y: f32| {
                        let q = ic + egui::vec2(x - 8.0, y - 8.0);
                        egui::pos2((q.x * ppp).round() / ppp, (q.y * ppp).round() / ppp)
                    };
                    let bar_rect = egui::Rect::from_min_max(at(1.5, 13.0), at(14.5, 15.5));
                    ui.painter().rect_filled(bar_rect, 0.0, bar);
                    if main.clicked() {
                        self.format_chars(&ctx, CharChange::Color(self.last_font_color));
                    }
                    let mut chosen = None;
                    Popup::menu(&r).show(|ui| {
                        ui.label("Font Color");
                        egui::Grid::new("palette")
                            .spacing([3.0, 3.0])
                            .show(ui, |ui| {
                                for (n, c) in widgets::palette().into_iter().enumerate() {
                                    let (rect, resp) = ui.allocate_exact_size(
                                        egui::vec2(18.0, 18.0),
                                        egui::Sense::click(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        Color32::from_rgb(c[0], c[1], c[2]),
                                    );
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(1.0, Color32::GRAY),
                                        egui::StrokeKind::Inside,
                                    );
                                    if resp.clicked() {
                                        chosen = Some(Rgb(c[0], c[1], c[2]));
                                        ui.close();
                                    }
                                    if (n + 1) % 8 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                    if let Some(c) = chosen {
                        self.last_font_color = c;
                        self.format_chars(&ctx, CharChange::Color(c));
                    }
                });
            });
        });
    }

    fn home_tab(&mut self, ui: &mut Ui) {
        let info = self.format_info(ui.ctx());
        let fmt = info.as_ref().map(|i| i.fmt.clone()).unwrap_or_default();
        let has_sel = !self.selected_ids().is_empty();
        let has_text = info.is_some();
        let has_doc = self.editor.is_some();
        group(ui, "Clipboard", true, |ui| {
            // Tight padding: the long "Copy Formatting" label must not widen Clipboard enough
            // to push Arrange into icon-only buttons.
            ui.spacing_mut().item_spacing.x = 0.0;
            let can_paste = !self.clipboard.is_empty();
            if Btn::large(Icon::Paste, "Paste")
                .dropdown()
                .enabled(can_paste)
                .tip("Paste (Ctrl+V)")
                .show(ui)
                .clicked()
            {
                self.paste();
            }
            widgets::col(ui, |ui| {
                if Btn::small(Icon::Cut, "Cut")
                    .tight()
                    .enabled(has_sel)
                    .tip("Cut (Ctrl+X)")
                    .show(ui)
                    .clicked()
                {
                    self.copy_selection();
                    self.delete_selection();
                }
                if Btn::small(Icon::Copy, "Copy")
                    .tight()
                    .enabled(has_sel)
                    .tip("Copy (Ctrl+C)")
                    .show(ui)
                    .clicked()
                {
                    self.copy_selection();
                }
                Btn::small(Icon::Brush, "Copy Formatting").tight().disabled().show(ui);
            });
        });
        self.font_group(ui, "Font");
        group(ui, "Paragraph", true, |ui| {
            let ctx = ui.ctx().clone();
            widgets::col2(ui, |ui| {
                ui.horizontal(|ui| {
                    self.list_buttons(ui);
                    widgets::divider(ui);
                    self.columns_menu(ui, false);
                    widgets::divider(ui);
                    soon(ui, Icon::Pilcrow, "", false);
                });
                ui.horizontal(|ui| {
                    for (icon, a, tip) in [
                        (Icon::AlignLeft, Align::Left, "Align Left (Ctrl+L)"),
                        (Icon::AlignCenter, Align::Center, "Center (Ctrl+E)"),
                        (Icon::AlignRight, Align::Right, "Align Right (Ctrl+R)"),
                        (Icon::AlignJustify, Align::Justify, "Justify (Ctrl+J)"),
                    ] {
                        if Btn::small(icon, "")
                            .enabled(has_text)
                            .selected(fmt.align == Some(a))
                            .tip(tip)
                            .show(ui)
                            .clicked()
                        {
                            self.format_align(&ctx, a);
                        }
                    }
                    widgets::divider(ui);
                    let r = Btn::small(Icon::LineSpacing, "")
                        .enabled(has_text)
                        .dropdown()
                        .tip("Line Spacing")
                        .show(ui);
                    let cur = self.format_spacing_info(&ctx);
                    let mut preset = None;
                    let mut options = false;
                    Popup::menu(&r).show(|ui| {
                        ui.set_min_width(140.0);
                        for m in [1.0_f32, 1.15, 1.5, 2.0, 2.5, 3.0] {
                            let on = matches!(
                                cur.map(|c| c.line),
                                Some(LineSpacing::Multiple(v)) if (v - m).abs() < 0.005
                            );
                            let label = format!("{}{m}", if on { "\u{2713}  " } else { "     " });
                            if ui.button(label).clicked() {
                                preset = Some(m);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Line Spacing Options...").clicked() {
                            options = true;
                            ui.close();
                        }
                    });
                    if let Some(m) = preset {
                        self.format_spacing(
                            &ctx,
                            None,
                            SpacingChange {
                                line: Some(LineSpacing::Multiple(m)),
                                ..Default::default()
                            },
                        );
                    }
                    if options {
                        self.open_spacing_dialog(&ctx);
                    }
                    soon_dd(ui, Icon::Shading, "", false);
                });
            });
        });
        group(ui, "Styles", false, |ui| {
            soon_dd(ui, Icon::Styles, "Styles", true)
        });
        group(ui, "Objects", false, |ui| {
            if Btn::large(Icon::TextBox, "Draw\nText Box")
                .enabled(has_doc)
                .selected(self.tool == Tool::TextBox)
                .show(ui)
                .clicked()
            {
                self.start_tool(Tool::TextBox);
            }
            if Btn::large(Icon::Picture, "Pictures")
                .enabled(has_doc)
                .show(ui)
                .clicked()
            {
                self.insert_picture_dialog();
            }
            self.table_menu(ui);
            self.shapes_menu(ui, true);
        });
        group(ui, "Arrange", false, |ui| {
            self.wrap_menu(ui);
            widgets::col(ui, |ui| {
                self.z_order_buttons(ui, has_sel, false);
                self.align_menu(ui, has_sel, false);
            });
            widgets::col(ui, |ui| {
                let (can_group, can_ungroup) = (self.can_group(), self.can_ungroup());
                if Btn::small(Icon::Group, "Group")
                    .enabled(can_group)
                    .show(ui)
                    .on_hover_text("Group (Ctrl+Shift+G)")
                    .clicked()
                {
                    self.group_selection();
                }
                if Btn::small(Icon::Ungroup, "Ungroup")
                    .enabled(can_ungroup)
                    .show(ui)
                    .on_hover_text("Ungroup (Ctrl+Shift+H)")
                    .clicked()
                {
                    self.ungroup_selection();
                }
                self.rotate_menu(ui, false);
            });
        });
        group(ui, "Editing", false, |ui| {
            widgets::col(ui, |ui| {
                soon_split(ui, Icon::Find, "Find");
                soon(ui, Icon::Replace, "Replace", false);
                soon_dd(ui, Icon::Select, "Select", false);
            });
        });
    }

    /// Publisher 2010's Arrange > Align drop-down. The editor selects one object at a time,
    /// so only "Relative to Margin Guides" can enable the items; the 2+/3+ rules otherwise.
    pub(crate) fn align_menu(&mut self, ui: &mut Ui, has_sel: bool, large: bool) {
        use kernpage_core::{AlignMode, Command};
        let ids = self.selected_ids();
        let count = ids.len();
        let to_margins = self.align_to_margins;
        let page = self.page;
        let align_ok = count >= if to_margins { 1 } else { 2 };
        let dist_ok = count >= if to_margins { 2 } else { 3 };
        let btn = if large {
            Btn::large(Icon::AlignObjects, "Align")
        } else {
            Btn::small(Icon::AlignObjects, "Align")
        };
        let r = btn.enabled(has_sel).dropdown().show(ui);
        let mut run = None;
        let mut toggle = false;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(190.0);
            let items = [
                ("Align Left", AlignMode::Left),
                ("Align Center", AlignMode::Center),
                ("Align Right", AlignMode::Right),
                ("Align Top", AlignMode::Top),
                ("Align Middle", AlignMode::Middle),
                ("Align Bottom", AlignMode::Bottom),
            ];
            for (text, mode) in items {
                if ui.add_enabled(align_ok, egui::Button::new(text)).clicked() {
                    run = Some(mode);
                    ui.close();
                }
            }
            ui.separator();
            for (text, mode) in [
                ("Distribute Horizontally", AlignMode::DistributeHorizontally),
                ("Distribute Vertically", AlignMode::DistributeVertically),
            ] {
                if ui.add_enabled(dist_ok, egui::Button::new(text)).clicked() {
                    run = Some(mode);
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .add(egui::Button::new("Relative to Margin Guides").selected(to_margins))
                .clicked()
            {
                toggle = true;
                ui.close();
            }
        });
        if toggle {
            self.align_to_margins = !self.align_to_margins;
        }
        if let Some(mode) = run {
            self.exec(Command::AlignShapes {
                page,
                ids,
                mode,
                to_margins,
            });
        }
    }

    fn style_cmd(&mut self, change: kernpage_core::command::StyleChange) {
        let ids = self.selected_ids();
        if !ids.is_empty() {
            self.exec(kernpage_core::Command::StyleShapes {
                page: self.page,
                ids,
                change,
            });
        }
    }

    /// The first selected shape's wrap type and column count (for the menus' check marks).
    fn selection_wrap_columns(&self) -> Option<(kernpage_core::WrapType, u32, bool)> {
        let ids = self.selected_ids();
        let ed = self.editor.as_ref()?;
        let pg = ed.doc().surface(self.page)?;
        let s = pg.shapes.iter().find(|s| ids.contains(&s.id))?;
        Some((s.wrap, s.columns, s.is_text_box()))
    }

    /// Arrange > Wrap Text. Only the types the file format is understood for are live
    /// (spec 4K): all five.
    pub(crate) fn wrap_menu(&mut self, ui: &mut Ui) {
        use kernpage_core::command::StyleChange as S;
        use kernpage_core::WrapType as W;
        let has_sel = !self.selected_ids().is_empty();
        let cur = self.selection_wrap_columns().map(|c| c.0);
        let r = Btn::large(Icon::Wrap, "Wrap\nText")
            .enabled(has_sel)
            .dropdown()
            .show(ui);
        if ui.is_visible() && self.debug_menu.as_deref() == Some("wrap") {
            Popup::open_id(ui.ctx(), Popup::default_response_id(&r));
        }
        let mut run = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(180.0);
            for (text, w) in [
                ("Square", W::Square),
                ("Tight", W::Tight),
                ("Through", W::Through),
                ("Top and Bottom", W::TopAndBottom),
            ] {
                if ui
                    .add(egui::Button::new(text).selected(cur == Some(w)))
                    .clicked()
                {
                    run = Some(S::Wrap(w));
                    ui.close();
                }
            }
            if ui
                .add(egui::Button::new("None").selected(cur == Some(W::None)))
                .clicked()
            {
                run = Some(S::Wrap(W::None));
                ui.close();
            }
            ui.separator();
            ui.add_enabled(false, egui::Button::new("Edit Wrap Points"));
            ui.add_enabled(false, egui::Button::new("More Layout Options..."));
        });
        if let Some(ch) = run {
            self.style_cmd(ch);
        }
    }

    /// Columns drop-down for text boxes (Text Box tab and Home > Paragraph).
    pub(crate) fn columns_menu(&mut self, ui: &mut Ui, large: bool) {
        use kernpage_core::command::StyleChange as S;
        let info = self.selection_wrap_columns();
        let is_tb = info.is_some_and(|c| c.2);
        let cur = info.map(|c| c.1);
        let btn = if large {
            Btn::large(Icon::Columns, "Columns")
        } else {
            Btn::small(Icon::Columns, "")
        };
        let r = btn.enabled(is_tb).dropdown().show(ui);
        if ui.is_visible() && large && self.debug_menu.as_deref() == Some("columns") {
            Popup::open_id(ui.ctx(), Popup::default_response_id(&r));
        }
        let mut run = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(150.0);
            for (text, n) in [("One", 1u32), ("Two", 2), ("Three", 3)] {
                if ui
                    .add(egui::Button::new(text).selected(cur == Some(n)))
                    .clicked()
                {
                    // Publisher's default gap is 0.04 in (36,576 EMU).
                    run = Some(S::Columns(n, 36_576));
                    ui.close();
                }
            }
            ui.separator();
            ui.add_enabled(false, egui::Button::new("More Columns..."));
        });
        if let Some(ch) = run {
            self.style_cmd(ch);
        }
    }

    /// Arrange > Rotate (Publisher 2021's items; More Rotation Options is not built yet).
    pub(crate) fn rotate_menu(&mut self, ui: &mut Ui, large: bool) {
        use kernpage_core::command::StyleChange as S;
        let has_sel = !self.selected_ids().is_empty();
        let any_line = self.editor.as_ref().is_some_and(|ed| {
            let ids = self.selected_ids();
            ed.doc().surface(self.page).is_some_and(|p| {
                p.shapes.iter().any(|s| {
                    ids.contains(&s.id) && matches!(s.kind, kernpage_core::ShapeKind::Line)
                })
            })
        });
        let btn = if large {
            Btn::large(Icon::Rotate, "Rotate")
        } else {
            Btn::small(Icon::Rotate, "Rotate")
        };
        let r = btn.enabled(has_sel).dropdown().show(ui);
        let mut run = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(180.0);
            for (text, ch) in [
                ("Rotate Right 90\u{b0}", S::Rotate(90.0)),
                ("Rotate Left 90\u{b0}", S::Rotate(270.0)),
            ] {
                if ui.button(text).clicked() {
                    run = Some(ch);
                    ui.close();
                }
            }
            for (text, ch) in [
                ("Flip Vertical", S::FlipVertical),
                ("Flip Horizontal", S::FlipHorizontal),
            ] {
                if ui
                    .add_enabled(any_line, egui::Button::new(text))
                    .on_disabled_hover_text("Flipping is supported for lines only so far")
                    .clicked()
                {
                    run = Some(ch);
                    ui.close();
                }
            }
            ui.separator();
            ui.add_enabled(false, egui::Button::new("More Rotation Options..."));
        });
        if let Some(ch) = run {
            self.style_cmd(ch);
        }
    }

    /// Shape Format > Shape Fill and Shape Outline split buttons.
    pub(crate) fn fill_outline_buttons(&mut self, ui: &mut Ui) {
        use kernpage_core::command::StyleChange as S;
        let has_sel = !self.selected_ids().is_empty();
        for outline in [false, true] {
            let (icon, label) = if outline {
                (Icon::Frame, "Shape Outline")
            } else {
                (Icon::Palette, "Shape Fill")
            };
            let (main, arrow) = widgets::split_button(ui, Btn::small(icon, label).enabled(has_sel));
            let last = if outline {
                self.last_outline_color
            } else {
                self.last_fill_color
            };
            let bar_rect = egui::Rect::from_min_size(
                main.rect.left_bottom() + egui::vec2(4.0, -4.0),
                egui::vec2(main.rect.width() - 8.0, 3.0),
            );
            ui.painter()
                .rect_filled(bar_rect, 0.0, Color32::from_rgb(last.0, last.1, last.2));
            let mk = |c: Option<Rgb>| if outline { S::Outline(c) } else { S::Fill(c) };
            let mut chosen: Option<S> = None;
            if main.clicked() {
                chosen = Some(mk(Some(last)));
            }
            Popup::menu(&arrow).show(|ui| {
                ui.set_min_width(170.0);
                if ui
                    .button(if outline { "No Outline" } else { "No Fill" })
                    .clicked()
                {
                    chosen = Some(mk(None));
                    ui.close();
                }
                ui.label("Colors");
                egui::Grid::new(("shape_palette", outline))
                    .spacing([3.0, 3.0])
                    .show(ui, |ui| {
                        for (n, c) in widgets::palette().into_iter().enumerate() {
                            let (rect, resp) = ui
                                .allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                            #[cfg(test)]
                            crate::backstage::seen(format!("swatch:{n}"), rect, true, false);
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                Color32::from_rgb(c[0], c[1], c[2]),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, Color32::GRAY),
                                egui::StrokeKind::Inside,
                            );
                            if resp.clicked() {
                                chosen = Some(mk(Some(Rgb(c[0], c[1], c[2]))));
                                ui.close();
                            }
                            if (n + 1) % 8 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                if outline {
                    ui.separator();
                    ui.menu_button("Weight", |ui| {
                        for (text, pt) in [
                            ("\u{bc} pt", 0.25),
                            ("\u{bd} pt", 0.5),
                            ("\u{be} pt", 0.75),
                            ("1 pt", 1.0),
                            ("1\u{bd} pt", 1.5),
                            ("2\u{bc} pt", 2.25),
                            ("3 pt", 3.0),
                            ("4\u{bd} pt", 4.5),
                            ("6 pt", 6.0),
                        ] {
                            if ui.button(text).clicked() {
                                chosen = Some(S::OutlineWeight((pt * 12_700.0_f64).round() as i64));
                                ui.close();
                            }
                        }
                    });
                }
            });
            if let Some(ch) = chosen {
                match ch {
                    S::Fill(Some(c)) => self.last_fill_color = c,
                    S::Outline(Some(c)) => self.last_outline_color = c,
                    _ => {}
                }
                self.style_cmd(ch);
            }
        }
    }

    pub(crate) fn z_order_buttons(&mut self, ui: &mut Ui, has_sel: bool, large: bool) {
        use kernpage_core::{Command, ZOrder};
        let page = self.page;
        let ids = self.selected_ids();
        type ZItem = (Icon, &'static str, [(&'static str, ZOrder); 2]);
        let items: [ZItem; 2] = [
            (
                Icon::Forward,
                "Bring Forward",
                [
                    ("Bring Forward", ZOrder::Forward),
                    ("Bring to Front", ZOrder::Front),
                ],
            ),
            (
                Icon::Backward,
                "Send Backward",
                [
                    ("Send Backward", ZOrder::Backward),
                    ("Send to Back", ZOrder::Back),
                ],
            ),
        ];
        for (icon, label, choices) in items {
            let (main, arrow) = if large {
                let two_line = label.replacen(' ', "\n", 1);
                widgets::large_split(ui, Btn::large(icon, &two_line).enabled(has_sel))
            } else {
                widgets::split_button(ui, Btn::small(icon, label).enabled(has_sel))
            };
            if main.clicked() {
                self.exec(Command::ZOrderShapes {
                    page,
                    ids: ids.clone(),
                    op: choices[0].1,
                });
            }
            Popup::menu(&arrow).show(|ui| {
                ui.set_min_width(120.0);
                for (text, op) in choices {
                    if ui.button(text).clicked() {
                        self.exec(Command::ZOrderShapes {
                            page,
                            ids: ids.clone(),
                            op,
                        });
                        ui.close();
                    }
                }
            });
        }
    }

    fn insert_tab(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        group(ui, "Pages", false, |ui| {
            let resp = Btn::large(Icon::Page, "Page")
                .dropdown()
                .enabled(has_doc && self.master_view.is_none())
                .tip("Insert a page after this one")
                .show(ui);
            Popup::menu(&resp).show(|ui| {
                ui.set_min_width(160.0);
                if ui.button("Insert Blank Page").clicked() {
                    self.insert_page();
                    ui.close();
                }
                if ui.button("Insert Duplicate Page").clicked() {
                    self.duplicate_page_at(self.page);
                    ui.close();
                }
            });
            soon(ui, Icon::Pages, "Catalog\nPages", true);
        });
        group(ui, "Tables", false, |ui| self.table_menu(ui));
        group(ui, "Illustrations", false, |ui| {
            if Btn::large(Icon::Picture, "Pictures")
                .enabled(has_doc)
                .show(ui)
                .clicked()
            {
                self.insert_picture_dialog();
            }
            Btn::large(Icon::Star, "Online\nPictures")
                .tip(ONLINE_PICTURES_TIP)
                .disabled()
                .show(ui);
            self.shapes_menu(ui, true);
            soon(ui, Icon::Frame, "Picture\nPlaceholder", true);
        });
        group(ui, "Content Blocks", true, |ui| {
            soon_dd(ui, Icon::Blocks, "Page\nElements", true);
            soon_dd(ui, Icon::Calendar, "Calendars", true);
            soon_dd(ui, Icon::Frame, "Borders &\nAccents", true);
            soon_dd(ui, Icon::Send, "Advertisements", true);
        });
        group(ui, "Text", false, |ui| {
            if Btn::large(Icon::TextBox, "Draw\nText Box")
                .enabled(has_doc)
                .selected(self.tool == Tool::TextBox)
                .show(ui)
                .clicked()
            {
                self.start_tool(Tool::TextBox);
            }
            soon_dd(ui, Icon::People, "Contact\nDetails", true);
            soon_dd(ui, Icon::Star, "Styled Text", true);
            soon(ui, Icon::Page, "Insert\nFile", true);
            soon_dd(ui, Icon::Hash, "Symbol", true);
            soon(ui, Icon::Clock, "Date &\nTime", true);
            soon(ui, Icon::Frame, "Object", true);
        });
        group(ui, "Links", false, |ui| {
            soon(ui, Icon::Link, "Link", true);
            soon(ui, Icon::Bookmark, "Bookmark", true);
        });
        self.header_footer_group(ui);
    }

    fn page_design_tab(&mut self, ui: &mut Ui) {
        let can_delete = self.page_count() > 1 && self.master_view.is_none();
        group(ui, "Template", false, |ui| {
            soon(ui, Icon::Page, "Change\nTemplate", true);
            soon(ui, Icon::Gear, "Options", true);
        });
        group(ui, "Page Setup", true, |ui| {
            self.page_setup_buttons(ui);
        });
        group(ui, "Layout", false, |ui| {
            soon_dd(ui, Icon::Guides, "Guides", true);
            widgets::col(ui, |ui| {
                ui.label(RichText::new("Align To").size(12.0).color(crate::theme::text()));
                let (mut a, mut b) = (true, true);
                check(ui, "Guides", &mut a, false);
                check(ui, "Objects", &mut b, false);
            });
        });
        group(ui, "Pages", false, |ui| {
            widgets::col(ui, |ui| {
                if Btn::small(Icon::Trash, "Delete")
                    .enabled(can_delete)
                    .tip("Delete this page")
                    .show(ui)
                    .clicked()
                {
                    self.delete_page();
                }
                soon(ui, Icon::Move, "Move", false);
                soon(ui, Icon::Rename, "Rename", false);
            });
        });
        group(ui, "Palettes", false, |ui| scheme_gallery(ui));
        group(ui, "Page Background", false, |ui| {
            soon_dd(ui, Icon::Fonts, "Fonts", true);
            soon_dd(ui, Icon::Background, "Apply\nImage", true);
            soon_dd(ui, Icon::Palette, "Background", true);
            self.master_pages_dropdown(ui);
        });
    }

    /// The Data tab: our own mock-up of a list-driven layout workflow (nothing here works yet).
    fn data_tab(ui: &mut Ui) {
        fn inert<'a>(btn: Btn<'a>, dd: bool, tip: &'a str, ui: &mut Ui) {
            let b = btn.disabled().tip(tip);
            if dd { b.dropdown() } else { b }.show(ui);
        }
        group(ui, "List", false, |ui| {
            inert(
                Btn::large(Icon::Table, "Data\nSource"),
                true,
                "Coming soon: open a CSV file, type a list, or make a number sequence",
                ui,
            );
            inert(Btn::small(Icon::Rename, "Edit List"), false, "Coming soon: change the rows and columns of the list", ui);
        });
        group(ui, "Fields", false, |ui| {
            inert(
                Btn::large(Icon::Field, "Insert\nField"),
                true,
                "Coming soon: put a column of the list into the selected text box",
                ui,
            );
            inert(
                Btn::large(Icon::Picture, "Picture\nField"),
                false,
                "Coming soon: show a different picture for each row, from a file name in the list",
                ui,
            );
            inert(
                Btn::large(Icon::Sequence, "Numbering"),
                false,
                "Coming soon: ticket or certificate numbers like 001-500 without a spreadsheet",
                ui,
            );
        });
        group(ui, "Layout", false, |ui| {
            inert(
                Btn::large(Icon::Copy, "Make\nCopies"),
                false,
                "Coming soon: one copy of the design for each row of the list",
                ui,
            );
            inert(
                Btn::large(Icon::RepeatArea, "Repeating\nArea"),
                false,
                "Coming soon: a directory or menu built from the list",
                ui,
            );
            inert(
                Btn::large(Icon::PerSheet, "Per\nSheet"),
                true,
                "Coming soon: several copies on one printed sheet, such as 6 name badges",
                ui,
            );
        });
        group(ui, "Check", false, |ui| {
            inert(Btn::large(Icon::Eye, "Preview"), false, "Coming soon: show the real list values in place of the fields", ui);
            widgets::col(ui, |ui| {
                ui.horizontal(|ui| {
                    inert(Btn::small(Icon::Back, ""), false, "Coming soon: first row", ui);
                    inert(Btn::small(Icon::Back, ""), false, "Coming soon: previous row", ui);
                    let mut rec = String::from("1 of 0");
                    ui.add_enabled_ui(false, |ui| ui.add_sized([64.0, 20.0], egui::TextEdit::singleline(&mut rec)));
                    inert(Btn::small(Icon::Arrow, ""), false, "Coming soon: next row", ui);
                    inert(Btn::small(Icon::Arrow, ""), false, "Coming soon: last row", ui);
                });
                inert(Btn::small(Icon::Find, "Find Row"), false, "Coming soon: jump to a row that contains some text", ui);
            });
        });
        group(ui, "Create", false, |ui| {
            inert(
                Btn::large(Icon::CreatePage, "Create"),
                true,
                "Coming soon: a new publication, one PDF, one PDF per row, or print",
                ui,
            );
        });
    }

    fn review_tab(ui: &mut Ui) {        group(ui, "Proofing", false, |ui| {
            soon(ui, Icon::Check, "Spelling", true);
            soon(ui, Icon::Folder, "Thesaurus", true);
        });
        group(ui, "Language", false, |ui| {
            soon_dd(ui, Icon::Globe, "Language", true);
        });
    }

    fn help_tab(&mut self, ui: &mut Ui) {
        group(ui, "Help", false, |ui| {
            Btn::large(Icon::Question, "Help")
                .tip("Coming soon: tutorials, how-to articles and answers to common questions.")
                .disabled()
                .show(ui);
            if Btn::large(Icon::Info, "About")
                .tip("About KernPage")
                .show(ui)
                .clicked()
            {
                self.show_about();
            }
            if Btn::large(Icon::Warning, "Report a\nProblem")
                .tip("Report a Problem \u{2014} Opens a new GitHub issue in your web browser, with your KernPage and Windows version filled in. Nothing is sent until you submit it there.")
                .show(ui)
                .clicked()
            {
                crate::feedback::report_a_problem();
            }
            if Btn::large(Icon::Lightbulb, "Suggest\nan Idea")
                .tip("Suggest an Idea \u{2014} Opens a new GitHub issue in your web browser, with your KernPage version filled in. Nothing is sent until you submit it there.")
                .show(ui)
                .clicked()
            {
                crate::feedback::suggest_an_idea();
            }
        });
    }

    fn view_tab(&mut self, ui: &mut Ui) {
        self.views_group(ui);
        group(ui, "Layout", false, |ui| {
            Btn::large(Icon::SinglePage, "Single\nPage")
                .selected(true)
                .tip("Single Page")
                .show(ui);
            soon(ui, Icon::TwoPage, "Two-Page\nSpread", true);
        });
        group(ui, "Show", false, |ui| {
            let mut dummy = [false; 3];
            let reset_fit = |app: &mut App| {
                if app.zoom_mode != ZoomMode::Custom {
                    app.reset_scroll = true;
                }
            };
            widgets::col(ui, |ui| {
                ui.checkbox(&mut self.show.boundaries, "Boundaries");
                ui.checkbox(&mut self.show.guides, "Guides");
                check(ui, "Fields", &mut dummy[0], false);
            });
            widgets::col(ui, |ui| {
                ui.checkbox(&mut self.show.rulers, "Rulers");
                ui.checkbox(&mut self.show.nav, "Page Navigation");
                ui.checkbox(&mut self.show.scratch, "Scratch Area");
            });
            widgets::col(ui, |ui| {
                check(ui, "Picture Manager", &mut dummy[1], false);
                check(ui, "Baselines", &mut dummy[2], false);
            });
            reset_fit(self);
        });
        group(ui, "Zoom", false, |ui| {
            if Btn::large(Icon::Zoom, "100%")
                .selected(self.zoom_mode == ZoomMode::Custom && (self.zoom - 1.0).abs() < 1e-6)
                .show(ui)
                .clicked()
            {
                self.zoom_mode = ZoomMode::Custom;
                self.zoom = 1.0;
                self.reset_scroll = true;
            }
            if Btn::large(Icon::FitPage, "Whole\nPage")
                .selected(self.zoom_mode == ZoomMode::WholePage)
                .show(ui)
                .clicked()
            {
                self.zoom_mode = ZoomMode::WholePage;
            }
            widgets::col(ui, |ui| {
                let pct = (self.zoom * 100.0).round();
                let mut pick = None;
                egui::ComboBox::from_id_salt("zoom_pct")
                    .width(104.0)
                    .selected_text(format!("{pct}%"))
                    .show_ui(ui, |ui| {
                        for v in [10.0_f64, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 400.0] {
                            if ui
                                .selectable_label((v - pct).abs() < 0.5, format!("{v}%"))
                                .clicked()
                            {
                                pick = Some(v);
                            }
                        }
                    });
                if let Some(v) = pick {
                    self.zoom_mode = ZoomMode::Custom;
                    self.zoom = v / 100.0;
                }
                if Btn::small(Icon::PageWidth, "Page Width")
                    .selected(self.zoom_mode == ZoomMode::PageWidth)
                    .show(ui)
                    .clicked()
                {
                    self.zoom_mode = ZoomMode::PageWidth;
                }
                soon(ui, Icon::Select, "Selected Objects", false);
            });
        });
        group(ui, "Window", false, |ui| {
            soon(ui, Icon::Window, "Arrange\nAll", true);
            soon(ui, Icon::Window, "Cascade", true);
            soon_dd(ui, Icon::Window, "Switch\nWindows", true);
        });
        let _ = (COMING_SOON, ACCENT_COLOR);
    }
}

/// The Page Design > Palettes gallery: 6 columns by 3 rows of colour swatches with names,
/// plus the scroll/expand strip. Decorative until colour palettes exist, so it is inert.
/// The names and colours are our own (checked against Publisher 2021's 93 scheme names and
/// colours on 2026-11-24, none shared).
fn scheme_gallery(ui: &mut Ui) {
    const NAMES: [[&str; 3]; 6] = [
        ["Bayside", "Linen", "Clover"],
        ["Pewter", "Ember", "Thistle"],
        ["Granite", "Dune", "Tidewater"],
        ["Cedar", "Haze", "Damson"],
        ["Copper", "Inlet", "Parchment"],
        ["Fern", "Cobalt", "Dusk"],
    ];
    const COLS: [[[u8; 3]; 4]; 18] = [
        [
            [96, 96, 112],
            [196, 176, 88],
            [150, 170, 90],
            [170, 160, 200],
        ],
        [
            [40, 100, 130],
            [30, 150, 200],
            [40, 190, 230],
            [210, 240, 250],
        ],
        [
            [40, 80, 140],
            [190, 70, 70],
            [80, 130, 190],
            [225, 215, 190],
        ],
        [
            [110, 118, 128],
            [150, 160, 172],
            [96, 140, 160],
            [228, 232, 236],
        ],
        [
            [110, 130, 100],
            [150, 170, 120],
            [190, 205, 160],
            [235, 235, 220],
        ],
        [
            [160, 60, 120],
            [200, 60, 90],
            [170, 50, 70],
            [240, 225, 235],
        ],
        [
            [120, 120, 120],
            [150, 150, 150],
            [190, 190, 190],
            [220, 220, 220],
        ],
        [
            [130, 90, 70],
            [210, 120, 70],
            [140, 170, 200],
            [225, 215, 190],
        ],
        [
            [220, 110, 40],
            [100, 120, 170],
            [250, 200, 80],
            [250, 235, 150],
        ],
        [
            [70, 80, 110],
            [60, 160, 90],
            [190, 60, 190],
            [180, 220, 230],
        ],
        [
            [60, 70, 100],
            [200, 50, 120],
            [120, 200, 60],
            [225, 235, 240],
        ],
        [
            [80, 90, 120],
            [100, 150, 170],
            [150, 160, 190],
            [225, 230, 235],
        ],
        [[90, 90, 110], [40, 150, 190], [250, 140, 30], [200, 40, 50]],
        [
            [70, 110, 120],
            [90, 170, 190],
            [240, 170, 30],
            [235, 225, 200],
        ],
        [
            [70, 90, 40],
            [120, 190, 100],
            [200, 220, 110],
            [245, 240, 200],
        ],
        [
            [130, 60, 60],
            [170, 50, 40],
            [220, 120, 30],
            [190, 170, 150],
        ],
        [
            [80, 140, 200],
            [240, 160, 40],
            [255, 205, 0],
            [230, 225, 215],
        ],
        [
            [100, 40, 30],
            [240, 180, 20],
            [230, 130, 20],
            [205, 190, 170],
        ],
    ];
    let (cell_w, row_h) = (104.0, 24.0);
    let cols = fit::gallery_cols(6);
    let size = egui::vec2(cell_w * cols as f32 + 18.0, row_h * 3.0 + 4.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    #[cfg(test)]
    if ui.is_visible() {
        crate::backstage::seen("scheme_gallery".to_owned(), rect, false, false);
    }
    let p = ui.painter();
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::line()),
        egui::StrokeKind::Inside,
    );
    for (ci, col) in NAMES.iter().enumerate().take(cols) {
        for (ri, name) in col.iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                rect.min + egui::vec2(2.0 + ci as f32 * cell_w, 2.0 + ri as f32 * row_h),
                egui::vec2(cell_w - 2.0, row_h),
            );
            if ci == 0 && ri == 2 {
                p.rect_filled(cell, 0.0, Color32::from_rgb(150, 205, 195));
            }
            let sw = &COLS[ci * 3 + ri];
            for (k, c) in sw.iter().enumerate() {
                p.rect_filled(
                    egui::Rect::from_min_size(
                        cell.min + egui::vec2(2.0 + k as f32 * 9.0, 4.0),
                        egui::vec2(9.0, 15.0),
                    ),
                    0.0,
                    Color32::from_rgb(c[0], c[1], c[2]),
                );
            }
            p.text(
                cell.min + egui::vec2(46.0, row_h / 2.0),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(12.0),
                crate::theme::text(),
            );
        }
    }
    // Scroll up / scroll down / more strip.
    let sx = rect.max.x - 14.0;
    p.line_segment(
        [
            egui::pos2(sx, rect.min.y + 2.0),
            egui::pos2(sx, rect.max.y - 2.0),
        ],
        egui::Stroke::new(1.0, crate::theme::line()),
    );
    for (k, up) in [(0, true), (1, false), (2, false)] {
        let c = egui::pos2(sx + 7.0, rect.min.y + 9.0 + k as f32 * 24.0);
        let d = if up { -1.5 } else { 1.5 };
        p.line_segment(
            [c + egui::vec2(-3.0, -d), c + egui::vec2(0.0, d)],
            egui::Stroke::new(1.3, crate::theme::disabled()),
        );
        p.line_segment(
            [c + egui::vec2(0.0, d), c + egui::vec2(3.0, -d)],
            egui::Stroke::new(1.3, crate::theme::disabled()),
        );
    }
    resp.on_hover_text(COMING_SOON);
}
