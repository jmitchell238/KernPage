//! Small ribbon-style widgets: icon buttons, groups and colour swatches.

use crate::icons::{self, Icon};
use crate::ribbon_fit as fit;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use kernpage_core::template::ACCENT;

pub const ACCENT_COLOR: Color32 = Color32::from_rgb(ACCENT.0, ACCENT.1, ACCENT.2);

pub const COMING_SOON: &str = "Coming soon";

/// Test seam: every ribbon button drawn (visibly) in the last frame, so UI tests can click
/// buttons that carry a text label (those have no AccessKit name) with real pointer events.
#[cfg(test)]
pub mod probe {
    use eframe::egui::Rect;
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
    pub struct Seen {
        pub label: String,
        pub tip: String,
        pub rect: Rect,
        pub enabled: bool,
        pub selected: bool,
    }

    thread_local! {
        static SEEN: RefCell<Vec<Seen>> = const { RefCell::new(Vec::new()) };
    }

    pub fn clear() {
        SEEN.with(|s| s.borrow_mut().clear());
    }
    pub fn record(s: Seen) {
        SEEN.with(|v| v.borrow_mut().push(s));
    }
    pub fn all() -> Vec<Seen> {
        SEEN.with(|s| s.borrow().clone())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Large,
    Small,
    /// Status bar button: flat 40 x 21 cell with a 15 px icon, dark fill when selected.
    Status,
}

/// A ribbon command button. Disabled buttons show the "Coming soon" tooltip unless a
/// different reason is given.
pub struct Btn<'a> {
    pub icon: Icon,
    pub label: &'a str,
    pub size: Size,
    pub enabled: bool,
    pub selected: bool,
    pub tip: &'a str,
    /// Draw a small drop-down arrow after the label.
    pub dropdown: bool,
    /// Text/icon colour override (for buttons on the green status bar).
    pub fg: Option<Color32>,
    /// Status bar buttons only: cell width override.
    pub cell_w: Option<f32>,
    /// Small buttons only: tighter padding (2 px left, 2 px icon gap, no right pad), for groups
    /// whose long labels would otherwise push the ribbon into a smaller layout.
    pub tight: bool,
}

impl<'a> Btn<'a> {
    pub fn large(icon: Icon, label: &'a str) -> Self {
        Btn {
            icon,
            label,
            size: Size::Large,
            enabled: true,
            selected: false,
            tip: "",
            dropdown: false,
            fg: None,
            cell_w: None,
            tight: false,
        }
    }
    pub fn small(icon: Icon, label: &'a str) -> Self {
        Btn {
            icon,
            label,
            size: Size::Small,
            enabled: true,
            selected: false,
            tip: "",
            dropdown: false,
            fg: None,
            cell_w: None,
            tight: false,
        }
    }
    pub fn status(icon: Icon) -> Self {
        Btn { size: Size::Status, ..Btn::small(icon, "") }
    }
    pub fn tight(mut self) -> Self {
        self.tight = true;
        self
    }
    pub fn cell_w(mut self, w: f32) -> Self {
        self.cell_w = Some(w);
        self
    }
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
    pub fn selected(mut self, on: bool) -> Self {
        self.selected = on;
        self
    }
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = tip;
        self
    }
    pub fn fg(mut self, c: Color32) -> Self {
        self.fg = Some(c);
        self
    }
    pub fn dropdown(mut self) -> Self {
        self.dropdown = true;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> Response {
        let font = FontId::proportional(LABEL_PX);
        let ppp = ui.ctx().pixels_per_point();
        let snap = |p: Pos2| Pos2::new((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
        // Ribbon scaling (see ribbon_fit): reduced groups show large buttons as small ones,
        // and the smallest groups drop the labels (kept as the tooltip).
        let level = fit::level();
        let mut label_buf = self.label.replace('\n', " ");
        let mut tip_buf = self.tip.to_owned();
        if level >= 1 && self.size == Size::Large {
            self.size = Size::Small;
        }
        if level >= 2 && !label_buf.is_empty() && self.fg.is_none() {
            if tip_buf.is_empty() {
                tip_buf = if self.enabled {
                    label_buf.clone()
                } else {
                    format!("{label_buf} ({COMING_SOON})")
                };
            }
            label_buf.clear();
        }
        let tip = tip_buf.as_str();
        let label_src = if self.size == Size::Large && level == 0 {
            self.label
        } else {
            label_buf.as_str()
        };
        let lines: Vec<&str> = if label_src.is_empty() {
            vec![]
        } else {
            label_src.split('\n').collect()
        };
        let widths: Vec<f32> = lines
            .iter()
            .map(|l| {
                ui.painter()
                    .layout_no_wrap((*l).to_owned(), font.clone(), crate::theme::text())
                    .size()
                    .x
            })
            .collect();
        let text_w = widths.iter().cloned().fold(0.0, f32::max);
        let arrow = if self.dropdown { 10.0 } else { 0.0 };
        let size = match self.size {
            Size::Large => Vec2::new((text_w + arrow + 6.0).max(44.0), 70.0),
            Size::Small => {
                let w = if lines.is_empty() {
                    24.0 + arrow
                } else {
                    let (l, g, r) = if self.tight { (2.0, 2.0, 0.0) } else { (4.0, 4.0, 4.0) };
                    l + 16.0 + g + text_w + arrow + r
                };
                Vec2::new(w, 22.0)
            }
            Size::Status => Vec2::new(self.cell_w.unwrap_or(STATUS_BTN_W), STATUS_BAR_H),
        };
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, resp) = ui.allocate_exact_size(size, sense);
        let p = ui.painter();
        if ui.is_rect_visible(rect) {
            let hovered = resp.hovered() && self.enabled;
            if self.size == Size::Status {
                if self.selected || hovered {
                    let fill = if self.selected { crate::theme::accent_tint() } else { crate::theme::hover_bg() };
                    p.rect_filled(rect.shrink2(Vec2::new(1.0, 3.0)), 4.0, fill);
                }
                let sel_col = if self.selected && self.enabled { crate::theme::accent_text() } else { crate::theme::on_accent() };
                let col = self.fg.map(|c| if self.selected && self.enabled { sel_col } else { c }).unwrap_or(sel_col);
                icons::paint(p, Rect::from_center_size(snap(rect.center()), Vec2::splat(15.0)), self.icon, col);
            } else if self.selected || hovered {
                if self.selected && self.fg.is_none() {
                    let (fill, line) = on_style(hovered);
                    p.rect_filled(rect, TOGGLE_RADIUS, fill);
                    p.rect_stroke(rect, TOGGLE_RADIUS, Stroke::new(1.0, line), StrokeKind::Inside);
                } else {
                    let bg = if self.fg.is_some() {
                        Color32::from_white_alpha(if self.selected { 70 } else { 40 })
                    } else {
                        crate::theme::hover_bg()
                    };
                    p.rect_filled(rect, TOGGLE_RADIUS, bg);
                    p.rect_stroke(
                        rect,
                        TOGGLE_RADIUS,
                        Stroke::new(1.0, crate::theme::button_outline()),
                        StrokeKind::Inside,
                    );
                }
            }
            if crate::search::flash::matches(self.label, tip, ui.input(|i| i.time)) {
                p.rect_stroke(rect, 4.0, Stroke::new(2.0, crate::theme::accent()), StrokeKind::Inside);
                ui.ctx().request_repaint();
            }
            let col = self.fg.unwrap_or(if self.enabled { crate::theme::text() } else { crate::theme::disabled() });
            let icon_col = self.fg.unwrap_or(if self.enabled { ACCENT_COLOR } else { crate::theme::disabled() });
            match self.size {
                Size::Large => {
                    let ir = Rect::from_center_size(
                        snap(Pos2::new(rect.center().x, rect.min.y + 20.0)),
                        Vec2::splat(28.0),
                    );
                    icons::paint(p, ir, self.icon, icon_col);
                    let mut y = rect.min.y + 37.0;
                    if self.dropdown && lines.len() == 1 {
                        // One-line label: the arrow sits centred on the line below it.
                        let c = snap(Pos2::new(rect.center().x, y + 19.0));
                        p.add(egui::Shape::convex_polygon(
                            vec![
                                c + Vec2::new(-3.0, -1.5),
                                c + Vec2::new(3.0, -1.5),
                                c + Vec2::new(0.0, 2.0),
                            ],
                            col,
                            Stroke::NONE,
                        ));
                    }
                    for (i, l) in lines.iter().enumerate() {
                        let mut x = rect.center().x;
                        if self.dropdown && lines.len() > 1 && i == lines.len() - 1 {
                            x -= arrow / 2.0;
                            let c = snap(Pos2::new(x + widths[i] / 2.0 + 6.0, y + 6.0));
                            p.add(egui::Shape::convex_polygon(
                                vec![
                                    c + Vec2::new(-3.0, -1.5),
                                    c + Vec2::new(3.0, -1.5),
                                    c + Vec2::new(0.0, 2.0),
                                ],
                                col,
                                Stroke::NONE,
                            ));
                        }
                        p.text(snap(Pos2::new(x, y)), Align2::CENTER_TOP, l, font.clone(), col);
                        y += LINE_PX;
                    }
                }
                Size::Status => {}
                Size::Small => {
                    if lines.is_empty() {
                        let ic = snap(if self.dropdown {
                            Pos2::new(rect.min.x + 12.0, rect.center().y)
                        } else {
                            rect.center()
                        });
                        icons::paint(
                            p,
                            Rect::from_center_size(ic, Vec2::splat(16.0)),
                            self.icon,
                            icon_col,
                        );
                        if self.dropdown {
                            let c = Pos2::new(rect.max.x - 7.0, rect.center().y);
                            p.add(egui::Shape::convex_polygon(
                                vec![
                                    c + Vec2::new(-3.0, -1.5),
                                    c + Vec2::new(3.0, -1.5),
                                    c + Vec2::new(0.0, 2.0),
                                ],
                                col,
                                Stroke::NONE,
                            ));
                        }
                    } else {
                        let ir = Rect::from_center_size(
                            snap(Pos2::new(rect.min.x + if self.tight { 2.0 } else { 4.0 } + 8.0, rect.center().y)),
                            Vec2::splat(16.0),
                        );
                        icons::paint(p, ir, self.icon, icon_col);
                        p.text(
                            snap(Pos2::new(ir.max.x + if self.tight { 2.0 } else { 4.0 }, rect.center().y)),
                            Align2::LEFT_CENTER,
                            lines[0],
                            font,
                            col,
                        );
                        if self.dropdown {
                            let c = Pos2::new(rect.max.x - 9.0, rect.center().y);
                            p.add(egui::Shape::convex_polygon(
                                vec![
                                    c + Vec2::new(-3.0, -1.5),
                                    c + Vec2::new(3.0, -1.5),
                                    c + Vec2::new(0.0, 2.0),
                                ],
                                col,
                                Stroke::NONE,
                            ));
                        }
                    }
                }
            }
        }
        #[cfg(test)]
        if ui.is_visible() {
            probe::record(probe::Seen {
                label: self.label.replace('\n', " "),
                tip: self.tip.to_owned(),
                rect,
                enabled: self.enabled,
                selected: self.selected,
            });
        }
        if lines.is_empty() && !tip.is_empty() {
            // Icon-only button: its tip is also its accessible name (screen readers, UI tests).
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, self.enabled, tip)
            });
        }
        if !self.enabled {
            resp.on_hover_text(if tip.is_empty() { COMING_SOON } else { tip })
        } else if !tip.is_empty() {
            resp.on_hover_text(tip)
        } else {
            resp
        }
    }
}

/// Corner radius of a ribbon button's hover and "on" highlight.
pub const TOGGLE_RADIUS: f32 = 3.0;

/// The one "on" look of every ribbon toggle button (View > Normal, Single Page, Whole Page ...):
/// a light tint of the accent colour with a thin accent border, drawn over the button's own
/// rectangle (the same one the hover highlight uses, so it stays inside the group card's
/// padding). Returns `(fill, border)`; hovering an ON button deepens the tint.
pub fn on_style(hovered: bool) -> (Color32, Color32) {
    let tint = crate::theme::accent_tint();
    let fill = if hovered { crate::theme::mix(tint, crate::theme::accent(), 0.18) } else { tint };
    (fill, crate::theme::accent())
}

/// The icon a collapsed group shows on its drop-down button.
fn group_icon(title: &str) -> Icon {
    match title {
        "Clipboard" => Icon::Paste,
        "Font" => Icon::FontColor,
        "Paragraph" => Icon::Pilcrow,
        "Styles" => Icon::Styles,
        "Objects" | "Insert Shapes" => Icon::Shapes,
        "Arrange" => Icon::AlignObjects,
        "Editing" => Icon::Find,
        "Pages" => Icon::Pages,
        "Tables" | "Table" | "Table Formats" | "Merge" => Icon::Table,
        "Illustrations" | "Insert" | "Picture Styles" => Icon::Picture,
        "Content Blocks" => Icon::Blocks,
        "Text" => Icon::TextBox,
        "Links" | "Linking" => Icon::Link,
        "Header & Footer" => Icon::Header,
        "Template" => Icon::Page,
        "Page Setup" => Icon::Margins,
        "Layout" => Icon::Guides,
        "Palettes" | "Shape Styles" | "Adjust" => Icon::Palette,
        "Page Background" => Icon::Background,
        "List" => Icon::Table,
        "Fields" => Icon::Field,
        "Check" => Icon::Eye,
        "Create" => Icon::CreatePage,
        "Start" => Icon::Mail,
        "Write & Insert Fields" => Icon::Hash,
        "Preview Results" | "Typography" => Icon::Abc,
        "Finish" => Icon::Send,
        "Proofing" => Icon::Check,
        "Language" => Icon::Globe,
        "Help" => Icon::Question,
        "Views" => Icon::Page,
        "Show" => Icon::Check,
        "Zoom" => Icon::Zoom,
        "Window" => Icon::Window,
        "Size" => Icon::Size,
        "Alignment" => Icon::AlignCenter,
        "Text Styles" => Icon::FontColor,
        "Swap" => Icon::Replace,
        "Crop" | "Borders" => Icon::Frame,
        "Rows & Columns" => Icon::Columns,
        _ => Icon::Gear,
    }
}

/// A group title as two lines when it is long ("Write & Insert\nFields").
fn two_line(title: &str) -> String {
    if title.chars().count() <= 10 {
        return title.to_owned();
    }
    let mid = title.len() / 2;
    let split = title
        .match_indices(' ')
        .map(|(i, _)| i)
        .min_by_key(|&i| i.abs_diff(mid));
    match split {
        Some(i) => format!("{}\n{}", &title[..i], &title[i + 1..]),
        None => title.to_owned(),
    }
}

/// A labelled ribbon group with an optional dialog launcher in its corner.
///
/// The group is drawn at the size level the ribbon fitter chose for it (see `ribbon_fit`):
/// at the collapsed level it is a single drop-down button whose popup holds the same
/// controls at full size.
pub fn group(ui: &mut Ui, title: &str, launcher: bool, add: impl FnOnce(&mut Ui)) {
    let lvl = fit::enter_group(title);
    let x0 = ui.cursor().min.x;
    // Each group is its own soft rounded tile, painted behind the controls.
    ui.add_space(TILE_PAD);
    let tile = ui.painter().add(egui::Shape::Noop);
    let inner = if lvl == fit::COLLAPSED {
        ui.vertical(|ui| {
            ui.set_min_height(100.0);
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(4.0);
            let resp = fit::with_level(0, || {
                ui.allocate_ui_with_layout(
                    Vec2::new(0.0, 72.0),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.set_min_height(72.0);
                        Btn::large(group_icon(title), &two_line(title))
                            .dropdown()
                            .tip(title)
                            .show(ui)
                    },
                )
                .inner
            });
            // The open state is ours (not egui's single "open popup" slot), so a combo box
            // opened inside the popup does not close it. Clicks inside the popup, or in lists
            // opened from it, keep it open; a click anywhere else closes it.
            let ctx = ui.ctx().clone();
            let open_id = resp.id.with("collapsed_open");
            let mut open: bool = ctx.data(|d| d.get_temp(open_id)).unwrap_or(false);
            if resp.clicked() {
                open = !open;
            }
            if fit::debug_open_now(title) {
                ctx.all_styles_mut(|st| st.animation_time = 0.0);
                open = true;
            }
            let shown = egui::Popup::menu(&resp)
                .open_bool(&mut open)
                .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                .show(|ui| {
                    fit::with_level(0, || group_body(ui, title, launcher, 0, add));
                });
            if let Some(shown) = shown {
                let click = ctx.input(|i| i.pointer.any_click());
                if let (true, Some(pos)) = (click, ctx.input(|i| i.pointer.interact_pos())) {
                    let over_popup = ctx
                        .layer_id_at(pos)
                        .is_some_and(|l| l.order == egui::Order::Foreground);
                    if !over_popup && !shown.response.rect.contains(pos) && !resp.rect.contains(pos)
                    {
                        open = false;
                    }
                }
            }
            ctx.data_mut(|d| d.insert_temp(open_id, open));
        })
    } else {
        ui.vertical(|ui| group_body(ui, title, launcher, lvl, add))
    };
    let r = inner.response.rect;
    // Leave exactly GROUP_GAP between this card and the next (see the spacing arithmetic:
    // the cursor advances by the item spacing after every allocation).
    let sp = ui.spacing().item_spacing.x;
    ui.add_space((TILE_PAD + GROUP_GAP + 4.0 - 3.0 * sp).max(0.0));
    let rect = snap_rect(
        ui.ctx().pixels_per_point(),
        Rect::from_min_max(
            Pos2::new(r.min.x - TILE_PAD, r.min.y),
            Pos2::new(r.max.x + TILE_PAD, r.max.y),
        ),
    );
    #[cfg(test)]
    if ui.is_visible() {
        probe::record(probe::Seen {
            label: format!("grouptile:{title}"),
            tip: String::new(),
            rect,
            enabled: true,
            selected: false,
        });
    }
    // One crisp outline per card: the rect is on whole device pixels and the stroke is exactly
    // one device pixel wide, drawn inside the rect, so its centre is a pixel centre and it never
    // straddles two pixels. The fill is the panel's own colour (no second tone, so no second
    // soft edge beside the line); the small radius keeps the 1 px curve from smearing.
    let ppp = ui.ctx().pixels_per_point();
    let stroke_w = 1.0 / ppp;
    #[cfg(test)]
    if ui.is_visible() {
        probe::record(probe::Seen {
            label: format!("groupstroke:{title}"),
            tip: stroke_w.to_string(),
            rect,
            enabled: true,
            selected: false,
        });
    }
    ui.painter().set(
        tile,
        egui::epaint::RectShape::new(
            rect,
            TILE_RADIUS,
            crate::theme::card(),
            Stroke::new(stroke_w, crate::theme::card_line()),
            StrokeKind::Inside,
        ),
    );
    fit::leave_group(title, ui.cursor().min.x - x0);
}

/// Publisher 2021's thin light-grey vertical divider between control clusters inside a
/// group: 1 physical px wide, the height of a small-button row, colour #E1DFDD.
pub fn divider(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(Vec2::new(1.0, 22.0));
    let ppp = ui.ctx().pixels_per_point();
    let x = ((rect.center().x * ppp).floor() + 0.5) / ppp;
    ui.painter().line_segment(
        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
        Stroke::new(1.0 / ppp, crate::theme::line()),
    );
}

/// A vertical stack of controls inside a group. In a reduced group (buttons flowing down in
/// columns) a stack that cannot fit under the items already in the column starts a new one.
pub fn col<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    col_padded(ui, COL3_TOP, None, add)
}

/// Top padding (logical px) of a three-row stack: Publisher 2021 starts the first small row
/// ~4 px lower than the top of the controls area.
pub const COL3_TOP: f32 = 4.0;
/// Two-row stacks (Font, Paragraph): Publisher spreads the rows to a 32 px pitch, the first
/// row 11.2 px below the top of the controls area.
pub const COL2_TOP: f32 = 11.2;
pub const COL2_GAP: f32 = 9.2;

/// A two-row stack as Publisher lays out the Font and Paragraph groups.
pub fn col2<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    col_padded(ui, COL2_TOP, Some(COL2_GAP), add)
}

/// Extra space between two neighbouring stacks, so the widest label of one never touches the
/// check box or icon of the next.
pub const COL_GAP: f32 = 8.0;

fn col_padded<R>(ui: &mut Ui, top_pad: f32, gap: Option<f32>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let end_id = ui.id().with("col_end_x");
    let r = col_inner(ui, top_pad, gap, add, end_id);
    let end = ui.min_rect().max.x;
    ui.data_mut(|d| d.insert_temp(end_id, end));
    r
}

fn col_inner<R>(ui: &mut Ui, top_pad: f32, gap: Option<f32>, add: impl FnOnce(&mut Ui) -> R, end_id: egui::Id) -> R {
    let wrap = ui.layout().main_wrap();
    // Directly right of the previous stack (not the first one of the group): leave a gap.
    let prev_end: Option<f32> = ui.data(|d| d.get_temp(end_id));
    let adjacent = prev_end.is_some_and(|x| (ui.cursor().min.x - (x + ui.spacing().item_spacing.x)).abs() < 0.5);
    if adjacent && !wrap {
        ui.add_space(COL_GAP);
    }
    let body = move |ui: &mut Ui| {
        if !wrap {
            ui.add_space(top_pad);
            if let Some(g) = gap {
                ui.spacing_mut().item_spacing.y = g;
            }
        }
        add(ui)
    };
    if wrap {
        let top = ui.max_rect().min.y;
        let cur = ui.cursor();
        if cur.min.y > top + 1.0 {
            let at = Rect::from_min_size(
                Pos2::new(cur.max.x + ui.spacing().item_spacing.x, top),
                Vec2::ZERO,
            );
            let builder = egui::UiBuilder::new().max_rect(at).layout(egui::Layout::top_down(egui::Align::Min));
            return ui.scope_builder(builder, body).inner;
        }
    }
    ui.vertical(body).inner
}

/// Inner horizontal padding of a Modern group tile.
const TILE_PAD: f32 = 8.0;
/// Corner radius of a group card.
const TILE_RADIUS: f32 = 4.0;
/// Inner margin of the big ribbon panel: the same as the gap between cards.
pub const PANEL_MARGIN: i8 = 6;
/// Corner radius of the ribbon panel: the card radius plus the gap, so the corners are concentric.
pub const PANEL_RADIUS: f32 = TILE_RADIUS + GROUP_GAP;
/// Space between two neighbouring group cards.
pub const GROUP_GAP: f32 = 6.0;

/// `rect` with every edge on a whole device pixel at `ppp` pixels per point, so fills and
/// 1 px strokes are crisp at any scaling (1.25x, 1.5x ...).
pub fn snap_rect(ppp: f32, r: Rect) -> Rect {
    let s = |v: f32| (v * ppp).round() / ppp;
    Rect::from_min_max(Pos2::new(s(r.min.x), s(r.min.y)), Pos2::new(s(r.max.x), s(r.max.y)))
}

/// Status bar geometry (logical px), measured from Publisher 2021 at 125%.
pub const STATUS_BAR_H: f32 = 21.0;
pub const STATUS_BTN_W: f32 = 40.0;

/// Ribbon text sizes in points, as Publisher 2021 draws them (Segoe UI 9pt button labels,
/// 8pt group names).
pub const LABEL_PX: f32 = 12.0;
pub const GROUP_PX: f32 = 10.7;
/// Distance between the lines of a two-line button label.
pub const LINE_PX: f32 = 16.0;

/// The controls of a group plus its title row.
fn group_body(ui: &mut Ui, title: &str, launcher: bool, lvl: usize, add: impl FnOnce(&mut Ui)) {
    ui.spacing_mut().item_spacing = Vec2::new(2.0, 1.2);
    // Reduced groups flow their buttons down in columns of three.
    let layout = if lvl == 0 {
        egui::Layout::left_to_right(egui::Align::Min)
    } else {
        egui::Layout::top_down(egui::Align::Min).with_main_wrap(true)
    };
    // The group name sits in a row above the controls, left-aligned and muted.
    let inner_spacing = ui.spacing().item_spacing;
    // Whole-pixel structure: no fractional gaps between the name row and the controls.
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.add_space(4.0);
    let name_at = ui.cursor().min;
    ui.add_space(16.0);
    ui.allocate_ui_with_layout(Vec2::new(0.0, 72.0), layout, |ui| {
        ui.set_min_height(72.0);
        ui.spacing_mut().item_spacing = inner_spacing;
        add(ui);
    });
    let label_w = ui
        .painter()
        .layout_no_wrap(title.to_owned(), FontId::proportional(GROUP_PX), crate::theme::disabled())
        .size()
        .x;
    let w = ui
        .min_rect()
        .width()
        .max(label_w + if launcher { 22.0 } else { 6.0 });
    // The title is painted, not laid out: claim its width so the group is never narrower.
    ui.allocate_exact_size(Vec2::new(w, 0.0), Sense::hover());
    ui.add_space(8.0);
    let r = Rect::from_min_size(name_at, Vec2::new(w, 16.0));
    #[cfg(test)]
    if ui.is_visible() {
        probe::record(probe::Seen {
            label: format!("grouptitle:{title}"),
            tip: String::new(),
            rect: Rect::from_min_size(Pos2::new(r.min.x, r.min.y), Vec2::new(label_w, 16.0)),
            enabled: true,
            selected: false,
        });
    }
    ui.painter().text(
        Pos2::new(r.min.x, r.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(GROUP_PX),
        crate::theme::group_label(),
    );
    if launcher {
        let lr = Rect::from_center_size(Pos2::new(r.max.x - 5.0, r.center().y), Vec2::splat(10.0));
        let live = title == "Page Setup";
        let resp = ui.interact(
            lr,
            ui.scope_id().with(("launcher", title)),
            if live { Sense::click() } else { Sense::hover() },
        );
        if live && resp.clicked() {
            #[allow(deprecated)]
            let id = egui::Id::new("open_page_setup");
            ui.ctx().data_mut(|d| d.insert_temp(id, true));
        }
        // Office-style launcher: a small square corner with a diagonal arrow pointing down-right.
        let ppp = ui.ctx().pixels_per_point();
        let sn = |v: f32| ((v * ppp).round() + 0.5) / ppp;
        let (x0, y0) = (sn(lr.center().x - 4.5), sn(lr.center().y - 4.5));
        let (x1, y1) = (x0 + (9.0 * ppp).round() / ppp, y0 + (9.0 * ppp).round() / ppp);
        let col = crate::theme::group_label();
        let st = Stroke::new(1.0 / ppp, col);
        let pt = ui.painter();
        pt.line_segment([Pos2::new(x0, y0), Pos2::new(x1, y0)], st);
        pt.line_segment([Pos2::new(x0, y0), Pos2::new(x0, y1)], st);
        pt.line_segment([Pos2::new(x0, y1), Pos2::new(x0 + 3.0, y1)], st);
        pt.line_segment([Pos2::new(x1, y0), Pos2::new(x1, y0 + 3.0)], st);
        pt.line_segment([Pos2::new(x0 + 3.0, y0 + 3.0), Pos2::new(x1, y1)], st);
        pt.add(egui::Shape::convex_polygon(
            vec![Pos2::new(x1 + 0.5, y1 + 0.5), Pos2::new(x1 - 3.5, y1 + 0.5), Pos2::new(x1 + 0.5, y1 - 3.5)],
            col,
            Stroke::NONE,
        ));
        #[cfg(test)]
        if ui.is_visible() {
            probe::record(probe::Seen {
                label: format!("launcher:{title}"),
                tip: if live { "Page Setup...".to_owned() } else { String::new() },
                rect: lr,
                enabled: live,
                selected: false,
            });
        }
        resp.on_hover_text(if live { "Page Setup..." } else { COMING_SOON });
    }
}

/// A checkbox that is drawn in ribbon style; disabled ones show "Coming soon".
pub fn check(ui: &mut Ui, label: &str, value: &mut bool, enabled: bool) -> bool {
    let mut changed = false;
    ui.add_enabled_ui(enabled, |ui| {
        let r = ui.checkbox(value, label);
        changed = r.changed();
        if !enabled {
            r.on_hover_text(COMING_SOON);
        }
    });
    changed
}

/// Our own 8 x 5 colour palette for text colours.
pub fn palette() -> Vec<[u8; 3]> {
    let base: [[u8; 3]; 8] = [
        [0, 0, 0],
        [0x1F, 0x3A, 0x6E],
        [0x2E, 0x6F, 0xC4],
        [0x1E, 0x8A, 0x5A],
        [0xC0, 0x8A, 0x10],
        [0xC0, 0x3A, 0x2B],
        [0x7A, 0x3E, 0x9D],
        [0x6B, 0x72, 0x80],
    ];
    let mut out = Vec::new();
    for shade in [0.0f32, 0.25, 0.5, 0.7, 0.88] {
        for c in base {
            out.push(if shade == 0.0 {
                c
            } else {
                [0, 1, 2]
                    .map(|i| (f32::from(c[i]) + (255.0 - f32::from(c[i])) * shade).round() as u8)
            });
        }
    }
    out.push([255, 255, 255]);
    out
}

/// A Publisher/Office-style split button (small size): the icon+label part runs the command,
/// a separate narrow arrow segment opens a menu. The two segments highlight independently.
/// Returns `(main, arrow)` responses; attach `Popup::menu(&arrow)` for the menu.
pub fn split_button(ui: &mut Ui, btn: Btn<'_>) -> (Response, Response) {
    let enabled = btn.enabled;
    let height = 22.0;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            let main = Btn {
                dropdown: false,
                ..btn
            }
            .show(ui);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let (rect, arrow) = ui.allocate_exact_size(Vec2::new(13.0, height), sense);
            if ui.is_rect_visible(rect) {
                let p = ui.painter();
                if arrow.hovered() && enabled {
                    p.rect_filled(rect, 3.0, crate::theme::hover_bg());
                    p.rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.0, crate::theme::button_outline()),
                        StrokeKind::Inside,
                    );
                }
                let col = if enabled { crate::theme::text() } else { crate::theme::disabled() };
                let c = rect.center();
                p.add(egui::Shape::convex_polygon(
                    vec![
                        c + Vec2::new(-3.0, -1.5),
                        c + Vec2::new(3.0, -1.5),
                        c + Vec2::new(0.0, 2.0),
                    ],
                    col,
                    Stroke::NONE,
                ));
            }
            (main, arrow)
        })
        .inner
    })
    .inner
}

/// A large split button (Bring Forward, Send Backward on the contextual tabs): clicking the
/// icon runs the command, clicking the label area (with its arrow) opens a menu.
/// Returns `(main, arrow)`; attach `Popup::menu(&arrow)` for the menu.
pub fn large_split(ui: &mut Ui, btn: Btn<'_>) -> (Response, Response) {
    if fit::level() >= 1 {
        // Reduced groups show it as a small split button.
        return split_button(ui, Btn { size: Size::Small, ..btn });
    }
    let enabled = btn.enabled;
    let label = btn.label.to_owned();
    let main = Btn { dropdown: true, ..btn }.show(ui);
    let r = main.rect;
    let lower = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 38.0), r.max);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let arrow = ui.interact(lower, ui.scope_id().with(("large_split", label)), sense);
    (main, arrow)
}

/// Publisher 2021's editable drop-down: one white field with a thin border, the value
/// left-aligned and a chevron inside the right edge. Typing edits the value; the chevron opens
/// the list. Returns the typed text once it is committed (Enter or the box losing focus).
///
/// `shown` is the real value: while the box is not being typed in it shows that. `typing` is
/// the caller's "had the keyboard" flag (egui reports `lost_focus` for two frames).
#[allow(clippy::too_many_arguments)]
pub fn combo_field(
    ui: &mut Ui,
    salt: &str,
    label: &str,
    width: f32,
    text: &mut String,
    typing: &mut bool,
    shown: &str,
    enabled: bool,
    list: impl FnOnce(&mut Ui),
) -> Option<String> {
    const H: f32 = 22.0;
    const CHEV_W: f32 = 16.0;
    let (_, rect) = ui.allocate_space(Vec2::new(width, H));
    ui.painter().rect_filled(rect, 2.0, crate::theme::card());
    let chev = Rect::from_min_max(Pos2::new(rect.max.x - CHEV_W, rect.min.y), rect.max);
    // The text, left-aligned inside.
    let inner = Rect::from_min_max(
        rect.min + Vec2::new(1.0, 1.0),
        Pos2::new(chev.min.x, rect.max.y - 1.0),
    );
    let mut out = None;
    let mut focused = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            let r = ui.put(
                inner,
                egui::TextEdit::singleline(text)
                    .id_salt(salt)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::symmetric(5, 0))
                    .align(egui::Align2::LEFT_CENTER)
                    .text_color(if enabled { crate::theme::text() } else { crate::theme::disabled() })
                    .font(FontId::proportional(LABEL_PX))
                    .min_size(Vec2::ZERO)
                    .desired_width(inner.width() - 10.0),
            );
            focused = r.has_focus();
            if r.has_focus() {
                *typing = true;
            }
            if r.lost_focus() && *typing {
                *typing = false;
                out = Some(text.trim().to_string());
                *text = shown.to_string();
            } else if !r.has_focus() {
                *text = shown.to_string();
            }
        });
    });
    let hovered = enabled && ui.rect_contains_pointer(rect);
    let border = if !enabled {
        crate::theme::line()
    } else if focused {
        crate::theme::accent()
    } else if hovered {
        crate::theme::group_label()
    } else {
        crate::theme::button_outline()
    };
    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, border), StrokeKind::Inside);
    // The chevron: part of the field, with no button background of its own.
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let cr = ui
        .scope_builder(egui::UiBuilder::new().max_rect(chev), |ui| ui.allocate_exact_size(chev.size(), sense).1)
        .inner;
    cr.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, enabled, label));
    let c = chev.center();
    let col = if enabled { crate::theme::text() } else { crate::theme::disabled() };
    ui.painter().add(egui::Shape::line(
        vec![c + Vec2::new(-3.5, -1.5), c + Vec2::new(0.0, 2.0), c + Vec2::new(3.5, -1.5)],
        Stroke::new(1.2, col),
    ));
    let _ = egui::Popup::menu(&cr).width(width).show(|ui| {
        ui.set_min_width(width - 8.0);
        list(ui);
    });

    out
}
