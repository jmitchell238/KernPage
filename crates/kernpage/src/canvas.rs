//! The page view: scratch area, page, objects, selection, mouse editing and rulers.

use crate::app::{App, Drag, Tool, ZoomMode};
use crate::fontmgr::FontMgr;
use crate::render::{color, shape_inset_px, PageView};
use crate::widgets::ACCENT_COLOR;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{
    self, Color32, CursorIcon, Painter, PointerButton, Pos2, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2,
};
use kernpage_core::geometry::{
    emu_to_in, emu_to_px, handle_point, hit_handle, hit_test, in_to_emu, next_cell, pt_to_px,
    px_to_emu, resize, shapes_in_rect, table_cell_at, table_cell_rect, zoom_page_width,
    zoom_whole_page, Handle,
};
use kernpage_core::{
    Command, Paragraph, Rect, Rgb, Shape, ShapeId, ShapeKind, Stroke as DocStroke,
};

const RULER: f32 = 18.0;
/// Scratch-area padding around the page, in pixels.
const PAD: f32 = 70.0;
/// Scratch margin kept around the page in Whole Page / Page Width zoom, in pixels.
const FIT_PAD: f32 = 9.0;
/// The page is paper: white in every theme (only the application chrome is themed).
pub const PAGE_FILL: Color32 = Color32::WHITE;

fn ruler_bg() -> Color32 {
    if crate::theme::is_dark() { Color32::from_rgb(0x2B, 0x2D, 0x31) } else { Color32::from_rgb(0xE1, 0xE4, 0xE9) }
}
fn ruler_page() -> Color32 {
    if crate::theme::is_dark() { Color32::from_rgb(0x3C, 0x3E, 0x44) } else { Color32::from_rgb(0xF8, 0xF9, 0xFB) }
}
fn ruler_ink() -> Color32 {
    if crate::theme::is_dark() { Color32::from_rgb(0xB8, 0xBD, 0xC6) } else { Color32::from_rgb(0x4A, 0x51, 0x5C) }
}
fn ruler_edge() -> Color32 {
    if crate::theme::is_dark() { Color32::from_rgb(0x58, 0x5C, 0x63) } else { Color32::from_rgb(0xB3, 0xB8, 0xC1) }
}
const GUIDE: Color32 = Color32::from_rgb(0x6F, 0x8B, 0xC4);
const HANDLE_PX: f32 = 7.0;
const MIN_SIZE_IN: f64 = 0.1;

/// The editor's text as a layout job with each run's own font, size, style and colour.
/// `model` are the paragraphs the text was started from; the paragraphs for the current
/// buffer are rebuilt from it so typed text gets the formatting it will have.
fn edit_job(fonts: &FontMgr, model: &[Paragraph], text: &str, wrap: f32, zoom: f64) -> LayoutJob {
    let paras = kernpage_core::text::rebuild_paragraphs(model, text);
    let joined = paras
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n");
    let tf = |r: &kernpage_core::Run| {
        let col = color(r.color);
        TextFormat {
            font_id: crate::render::script_font(
                fonts,
                r,
                zoom,
                fonts.font_id(
                    &r.font,
                    r.bold,
                    r.italic,
                    pt_to_px(f64::from(r.size_pt), zoom) as f32,
                ),
            ),
            valign: crate::render::script_valign(r.script),
            color: col,
            italics: r.italic,
            underline: crate::render::underline_mark(r.underline, col),
            ..Default::default()
        }
    };
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let base = model
        .first()
        .map(Paragraph::base_run)
        .unwrap_or_else(|| kernpage_core::Run::plain(""));
    if joined != text {
        // Text the model cannot represent (e.g. a stray carriage return): one style.
        job.append(text, 0.0, tf(&base));
        return job;
    }
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            let prev = paras[i - 1].runs.last().unwrap_or(&base);
            job.append("\n", 0.0, tf(prev));
        }
        for r in &p.runs {
            job.append(&r.text, 0.0, tf(r));
        }
    }
    if job.sections.is_empty() {
        job.append("", 0.0, tf(&base));
    }
    job
}

enum Request {
    Run(Command),
    Create(Shape, bool),
    BeginEdit(ShapeId),
    BeginCellEdit(ShapeId, usize),
    CommitEdit,
    /// The text of the editor changed: flow a linked story on at once.
    LiveSync,
    /// A key at the edge of a linked frame (see `app::linked_key`).
    LinkedKey(crate::app::LinkedKey),
    /// A link arrow beside the selected frame: go to the next (true) / previous frame.
    LinkedGo(bool),
    Cut,
    Copy,
    Paste,
    Delete,
    Forward,
    Backward,
    Front,
    Back,
    Group,
    Ungroup,
    ChangePicture,
    RemovePicture,
    Zoom(ZoomMode),
    TextClip(TextClip),
    Spacing,
    Align(kernpage_core::Align),
}

/// A clipboard action for the in-place text editor, run on the next frame.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextClip {
    Cut,
    Copy,
    Paste,
}

/// A context-menu item: disabled when it does not apply; true when clicked (menu closes).
fn menu_item(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    let clicked = ui.add_enabled(enabled, egui::Button::new(label)).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// Set the selection fields (usable where `self` is partly borrowed).
pub fn apply_selection(
    selection: &mut Option<ShapeId>,
    multi: &mut Vec<ShapeId>,
    ids: Vec<ShapeId>,
) {
    *selection = ids.last().copied();
    *multi = if ids.len() > 1 { ids } else { Vec::new() };
}

/// Shift/Ctrl+click: add `id` to the selection, or remove it when already in.
fn toggle_id(ids: &mut Vec<ShapeId>, id: ShapeId) {
    if let Some(i) = ids.iter().position(|x| *x == id) {
        ids.remove(i);
    } else {
        ids.push(id);
    }
}

fn cell_count(s: &Shape) -> Option<usize> {
    match &s.kind {
        ShapeKind::Table { cells, .. } => Some(cells.len()),
        _ => None,
    }
}

fn handle_cursor(h: Handle) -> CursorIcon {
    match h {
        Handle::NorthWest | Handle::SouthEast => CursorIcon::ResizeNwSe,
        Handle::NorthEast | Handle::SouthWest => CursorIcon::ResizeNeSw,
        Handle::North | Handle::South => CursorIcon::ResizeVertical,
        Handle::East | Handle::West => CursorIcon::ResizeHorizontal,
    }
}

/// A new shape of the tool's kind in `rect`.
/// Publisher's defaults (spec 4N): no fill, a 0.75 pt outline in scheme colour 0.
fn new_shape(tool: Tool, rect: Rect, flips: (bool, bool), line_color: Rgb) -> Shape {
    let line = Some(DocStroke {
        color: line_color,
        width_emu: 9_525,
    });
    match tool {
        Tool::TextBox => Shape::text_box(rect, vec![Paragraph::plain("")]),
        Tool::Rectangle | Tool::Oval => {
            let kind = if tool == Tool::Rectangle {
                ShapeKind::Rectangle
            } else {
                ShapeKind::Ellipse
            };
            let mut s = Shape::new(rect, kind);
            s.line = line;
            s
        }
        _ => {
            let mut s = Shape::new(rect, ShapeKind::Line);
            s.line = line;
            (s.flip_h, s.flip_v) = flips;
            s
        }
    }
}

/// A line dragged from `start` to `cur`: its box and the flips that keep the drag direction.
/// `snap` (Shift) constrains the angle to 0, 45 or 90 degrees.
fn line_drag(start: (i64, i64), cur: (i64, i64), snap: bool) -> (Rect, (bool, bool)) {
    let (mut dx, mut dy) = (cur.0 - start.0, cur.1 - start.1);
    if snap {
        let (ax, ay) = (dx.abs(), dy.abs());
        if ay * 5 < ax * 2 {
            dy = 0;
        } else if ax * 5 < ay * 2 {
            dx = 0;
        } else {
            let m = ax.max(ay);
            dx = m * dx.signum();
            dy = m * dy.signum();
        }
    }
    let end = (start.0 + dx, start.1 + dy);
    (
        Rect::from_corners(start.0, start.1, end.0, end.1),
        (dx < 0, dy < 0),
    )
}

fn default_rect(tool: Tool, at: (i64, i64)) -> Rect {
    let (w, h) = match tool {
        Tool::TextBox => (2.0, 0.6),
        Tool::Line => (1.5, 0.0),
        _ => (1.5, 1.0),
    };
    Rect::new(at.0, at.1, in_to_emu(w), in_to_emu(h))
}

/// Test-only record of what the canvas painted in its last frame: lets UI tests convert
/// page coordinates to screen positions and check where each shape was drawn.
#[cfg(test)]
#[derive(Clone)]
pub struct Probe {
    /// Screen position of the page's top-left corner.
    pub origin: Pos2,
    pub zoom: f64,
    /// Screen rectangle each shape of the page was painted at (after any drag preview).
    pub painted: Vec<(ShapeId, egui::Rect)>,
    /// The clip rectangle shapes are painted through: the whole scratch area, or just the
    /// page when Scratch Area is off.
    pub content_clip: egui::Rect,
    /// The horizontal / vertical ruler rectangles (None when rulers are hidden) and where the
    /// cursor marker was drawn on each (screen x / y), if it was.
    pub ruler_h: Option<(egui::Rect, Option<f32>)>,
    pub ruler_v: Option<(egui::Rect, Option<f32>)>,
    /// The overflow / link boxes painted beside the selected frame.
    pub marks: Vec<(crate::linkmarks::Mark, egui::Rect)>,
    /// The fill of the selection handles as painted (None: no handles drawn).
    pub handle_fill: Option<Color32>,
    /// The colour the page itself was filled with (never themed).
    pub page_fill: Color32,
}

impl App {
    pub fn canvas(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        let ruler = if self.show.rulers { RULER } else { 0.0 };
        let canvas_rect = egui::Rect::from_min_max(full.min + Vec2::splat(ruler), full.max);
        let Some(ed) = self.editor.as_ref() else {
            return;
        };
        let Some(page) = ed.doc().pages.get(self.page) else {
            return;
        };
        let (pw, ph, margin) = (page.width_emu, page.height_emu, page.margins_emu);
        let line_color = ed.doc().default_line;

        // Fit modes use a thin scratch margin so the whole page really fits.
        let pad = if self.zoom_mode == ZoomMode::Custom {
            PAD
        } else {
            FIT_PAD
        };
        match self.zoom_mode {
            ZoomMode::WholePage => {
                self.zoom = zoom_whole_page(
                    f64::from(canvas_rect.width()) - 2.0 * f64::from(pad),
                    f64::from(canvas_rect.height()) - 2.0 * f64::from(pad),
                    pw,
                    ph,
                );
            }
            ZoomMode::PageWidth => {
                self.zoom = zoom_page_width(
                    f64::from(canvas_rect.width()) - 2.0 * f64::from(pad) - 14.0,
                    pw,
                );
            }
            ZoomMode::Custom => {}
        }
        let zoom = self.zoom;
        let page_px = Vec2::new(emu_to_px(pw, zoom) as f32, emu_to_px(ph, zoom) as f32);

        let mut child = ui.new_child(UiBuilder::new().max_rect(canvas_rect));
        child.set_clip_rect(canvas_rect.intersect(ui.clip_rect()));
        let mut area = egui::ScrollArea::both()
            .id_salt("page_scroll")
            .auto_shrink([false, false])
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: egui::scroll_area::DragScroll::Never,
                ..Default::default()
            });
        if self.reset_scroll || self.zoom_mode != ZoomMode::Custom {
            area = area.scroll_offset(Vec2::ZERO);
            self.reset_scroll = false;
        }

        let mut requests: Vec<Request> = Vec::new();
        let tool = self.tool;
        let show = self.show;
        let editing = self.edit.as_ref().map(|e| e.id);
        let edit_cell = self.edit.as_ref().and_then(|e| e.cell);
        let sel_all = self.selected_ids();
        let multi_on = sel_all.len() > 1;
        // Resize handles and the single-object shortcuts only apply to a lone selection.
        let selection = if multi_on { None } else { self.selection };
        let mut origin_out = Pos2::ZERO;
        // The selected text frame's link arrows and overflow box (only for one selected
        // frame, in Select mode).
        let (link_prev, link_next, overflow) = match self.linked_selection() {
            Some(id) if tool == Tool::Select && !multi_on && selection == Some(id) => {
                let (p, n) = ed.doc().frame_links(id);
                (p, n, self.selection_overflows())
            }
            _ => (false, false, false),
        };
        #[cfg(test)]
        let mut marks_seen: Vec<(crate::linkmarks::Mark, egui::Rect)> = Vec::new();
        #[cfg(test)]
        let mut handle_fill: Option<Color32> = None;
        #[cfg(test)]
        let mut painted: Vec<(ShapeId, egui::Rect)> = Vec::new();
        #[cfg(test)]
        let mut content_clip = egui::Rect::NOTHING;

        let output = area.show(&mut child, |ui| {
            let avail = ui.available_size();
            let content = Vec2::new(
                avail.x.max(page_px.x + 2.0 * pad),
                avail.y.max(page_px.y + 2.0 * pad),
            );
            let (resp, painter) = ui.allocate_painter(content, Sense::click_and_drag());
            let origin = resp.rect.min + (content - page_px) / 2.0;
            origin_out = origin;
            let page_rect = egui::Rect::from_min_size(origin, page_px);
            let to_emu = |p: Pos2| {
                (
                    px_to_emu(f64::from(p.x - origin.x), zoom),
                    px_to_emu(f64::from(p.y - origin.y), zoom),
                )
            };
            let tol = px_to_emu(6.0, zoom);
            let min_size = in_to_emu(MIN_SIZE_IN);

            // --- Pointer handling -------------------------------------------------
            let doc = ed.doc();
            let Some(pg) = doc.pages.get(self.page) else {
                return;
            };
            let hover = resp.hover_pos();
            let sel_rect = |id: Option<ShapeId>| id.and_then(|i| pg.shape(i)).map(|s| s.rect);
            if let Some(pp) = hover {
                let (x, y) = to_emu(pp);
                let mut icon = CursorIcon::Default;
                if tool != Tool::Select {
                    icon = CursorIcon::Crosshair;
                } else if let Some(h) = sel_rect(selection).and_then(|r| hit_handle(r, x, y, tol)) {
                    icon = handle_cursor(h);
                } else if hit_test(pg, x, y, tol).is_some() {
                    icon = CursorIcon::Move;
                }
                if self.drag.is_none() || matches!(self.drag, Some(Drag::Move { .. })) {
                    ui.ctx().set_cursor_icon(icon);
                }
            }
            let toggle_mod = ui.input(|i| i.modifiers.shift || i.modifiers.command);
            // The link arrows and the overflow box beside the selected frame take the
            // click (the overflow box does nothing, as in Publisher).
            let marks_now: Vec<(crate::linkmarks::Mark, egui::Rect)> =
                match selection.and_then(|id| pg.shape(id)) {
                    Some(s) if link_prev || link_next || overflow => {
                        let tl = origin
                            + Vec2::new(emu_to_px(s.rect.x, zoom) as f32, emu_to_px(s.rect.y, zoom) as f32);
                        let size = Vec2::new(
                            emu_to_px(s.rect.w, zoom) as f32,
                            emu_to_px(s.rect.h, zoom) as f32,
                        );
                        crate::linkmarks::marks(
                            egui::Rect::from_min_size(tl, size),
                            link_prev,
                            link_next,
                            overflow,
                        )
                    }
                    _ => Vec::new(),
                };
            let mark_at = |p: Option<Pos2>| {
                p.and_then(|p| marks_now.iter().find(|(_, r)| r.contains(p)).map(|m| m.0))
            };
            let on_mark = if tool != Tool::Select {
                None
            } else if resp.drag_started_by(PointerButton::Primary) {
                mark_at(ui.input(|i| i.pointer.press_origin()).or(hover))
            } else if resp.clicked_by(PointerButton::Primary) {
                mark_at(resp.interact_pointer_pos())
            } else {
                None
            };
            if let Some(m) = on_mark {
                match m {
                    crate::linkmarks::Mark::Next => requests.push(Request::LinkedGo(true)),
                    crate::linkmarks::Mark::Prev => requests.push(Request::LinkedGo(false)),
                    crate::linkmarks::Mark::Overflow => {}
                }
            } else if resp.drag_started_by(PointerButton::Primary) {
                let pos = ui
                    .input(|i| i.pointer.press_origin())
                    .or(hover)
                    .unwrap_or(origin);
                let (x, y) = to_emu(pos);
                if tool == Tool::Select {
                    requests.push(Request::CommitEdit);
                    if let Some((rect, h)) =
                        sel_rect(selection).and_then(|r| hit_handle(r, x, y, tol).map(|h| (r, h)))
                    {
                        self.drag = Some(Drag::Resize {
                            id: selection.unwrap_or(0),
                            handle: h,
                            orig: rect,
                            press: pos,
                        });
                    } else if let Some(id) = hit_test(pg, x, y, tol) {
                        if toggle_mod {
                            let mut ids = sel_all.clone();
                            toggle_id(&mut ids, id);
                            apply_selection(&mut self.selection, &mut self.multi, ids);
                        } else {
                            if !sel_all.contains(&id) {
                                apply_selection(&mut self.selection, &mut self.multi, vec![id]);
                            } else {
                                self.selection = Some(id);
                            }
                            if let Some(r) = sel_rect(Some(id)) {
                                self.drag = Some(Drag::Move {
                                    id,
                                    orig: r,
                                    press: pos,
                                });
                            }
                        }
                    } else {
                        if !toggle_mod {
                            apply_selection(&mut self.selection, &mut self.multi, Vec::new());
                        }
                        self.drag = Some(Drag::Marquee {
                            start: (x, y),
                            add: toggle_mod,
                        });
                    }
                } else {
                    self.drag = Some(Drag::Create {
                        tool,
                        start: (x, y),
                    });
                }
            } else if resp.clicked_by(PointerButton::Primary) {
                if let Some(pp) = resp.interact_pointer_pos() {
                    let (x, y) = to_emu(pp);
                    if tool == Tool::Select {
                        requests.push(Request::CommitEdit);
                        let hit = hit_test(pg, x, y, tol);
                        // Clicking a cell of the already-selected table edits that cell.
                        if let (Some(id), true) = (hit, hit == selection) {
                            if let Some(c) = pg.shape(id).and_then(|s| table_cell_at(s, x, y)) {
                                requests.push(Request::BeginCellEdit(id, c));
                            }
                        }
                        if toggle_mod {
                            if let Some(id) = hit {
                                let mut ids = sel_all.clone();
                                toggle_id(&mut ids, id);
                                apply_selection(&mut self.selection, &mut self.multi, ids);
                            }
                        } else {
                            apply_selection(
                                &mut self.selection,
                                &mut self.multi,
                                hit.into_iter().collect(),
                            );
                        }
                    } else {
                        let r = default_rect(tool, (x, y));
                        let is_text = tool == Tool::TextBox;
                        requests.push(Request::Create(
                            new_shape(tool, r, (false, false), line_color),
                            is_text,
                        ));
                    }
                }
            }
            if resp.double_clicked_by(PointerButton::Primary) {
                if let Some(pp) = resp.interact_pointer_pos() {
                    let (x, y) = to_emu(pp);
                    if let Some(id) = hit_test(pg, x, y, tol) {
                        if pg.shape(id).is_some_and(|s| s.is_text_box()) {
                            requests.push(Request::BeginEdit(id));
                        } else if let Some(c) = pg.shape(id).and_then(|s| table_cell_at(s, x, y)) {
                            requests.push(Request::BeginCellEdit(id, c));
                        }
                    }
                }
            }

            // Right-click menu on an object or the empty page (Publisher 2010 wording).
            if tool == Tool::Select && editing.is_none() {
                if resp.secondary_clicked() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        let (x, y) = to_emu(pp);
                        let hit = hit_test(pg, x, y, tol);
                        // Right-clicking inside the selection keeps it whole.
                        if !hit.is_some_and(|h| sel_all.contains(&h)) {
                            apply_selection(
                                &mut self.selection,
                                &mut self.multi,
                                hit.into_iter().collect(),
                            );
                        }
                    }
                }
                let has_sel = self.selection.is_some();
                let many = self.multi.len() > 1;
                let now: Vec<ShapeId> = if many {
                    self.multi.clone()
                } else {
                    self.selection.into_iter().collect()
                };
                let can_group = now.len() >= 2
                    && now
                        .iter()
                        .all(|i| pg.shape(*i).is_some_and(|s| s.group.is_none()));
                let can_ungroup = now.len() == 1
                    && pg
                        .shape(now[0])
                        .is_some_and(|s| matches!(s.kind, ShapeKind::Group));
                let can_paste = !self.clipboard.is_empty();
                let one_picture = now.len() == 1
                    && pg
                        .shape(now[0])
                        .is_some_and(|s| matches!(s.kind, ShapeKind::Picture { .. }));
                resp.context_menu(|ui| {
                    if has_sel {
                        if menu_item(ui, "Cut", true) {
                            requests.push(Request::Cut);
                        }
                        if menu_item(ui, "Copy", true) {
                            requests.push(Request::Copy);
                        }
                        if menu_item(ui, "Paste", can_paste) {
                            requests.push(Request::Paste);
                        }
                        ui.separator();
                        if one_picture {
                            ui.menu_button("Change Picture", |ui| {
                                if menu_item(ui, "Change Picture...", true) {
                                    requests.push(Request::ChangePicture);
                                }
                                if menu_item(ui, "Remove Picture", true) {
                                    requests.push(Request::RemovePicture);
                                }
                            });
                            ui.separator();
                        }
                        if menu_item(ui, "Bring Forward", true) {
                            requests.push(Request::Forward);
                        }
                        if menu_item(ui, "Bring to Front", true) {
                            requests.push(Request::Front);
                        }
                        if menu_item(ui, "Send Backward", true) {
                            requests.push(Request::Backward);
                        }
                        if menu_item(ui, "Send to Back", true) {
                            requests.push(Request::Back);
                        }
                        ui.separator();
                        if menu_item(ui, "Group", can_group) {
                            requests.push(Request::Group);
                        }
                        if menu_item(ui, "Ungroup", can_ungroup) {
                            requests.push(Request::Ungroup);
                        }
                        ui.separator();
                        if menu_item(
                            ui,
                            if many {
                                "Delete Objects"
                            } else {
                                "Delete Object"
                            },
                            true,
                        ) {
                            requests.push(Request::Delete);
                        }
                    } else {
                        if menu_item(ui, "Paste", can_paste) {
                            requests.push(Request::Paste);
                        }
                        ui.separator();
                        ui.menu_button("Zoom", |ui| {
                            if menu_item(ui, "Whole Page", true) {
                                requests.push(Request::Zoom(ZoomMode::WholePage));
                            }
                            if menu_item(ui, "Page Width", true) {
                                requests.push(Request::Zoom(ZoomMode::PageWidth));
                            }
                            if menu_item(ui, "100%", true) {
                                requests.push(Request::Zoom(ZoomMode::Custom));
                            }
                        });
                    }
                });
            }

            // The rectangle shown for the shape being dragged, and a rubber band.
            let mut preview: Option<(ShapeId, Rect)> = None;
            let mut band: Option<Rect> = None;
            let mut line_flips = (false, false);
            let mut marquee: Option<Rect> = None;
            // Offset applied to every selected shape while a multi-selection is dragged.
            let mut group_move: Option<(i64, i64)> = None;
            if let Some(drag) = self.drag {
                let cur = ui.input(|i| i.pointer.latest_pos()).unwrap_or(origin);
                let (dx, dy) = match drag {
                    Drag::Move { press, .. } | Drag::Resize { press, .. } => {
                        drag_delta(press, cur, zoom)
                    }
                    _ => (0, 0),
                };
                match drag {
                    Drag::Move { id, orig, .. } => {
                        preview = Some((id, orig.translated(dx, dy)));
                        let is_group = pg
                            .shape(id)
                            .is_some_and(|s| matches!(s.kind, ShapeKind::Group));
                        if multi_on || is_group {
                            group_move = Some((dx, dy));
                        }
                    }
                    Drag::Marquee { start, .. } => {
                        let (cx, cy) = to_emu(cur);
                        marquee = Some(Rect::from_corners(start.0, start.1, cx, cy));
                    }
                    Drag::Resize {
                        id, handle, orig, ..
                    } => {
                        let is_line = pg
                            .shape(id)
                            .is_some_and(|s| matches!(s.kind, ShapeKind::Line));
                        preview = Some((
                            id,
                            resize(orig, handle, dx, dy, if is_line { 0 } else { min_size }),
                        ));
                    }
                    Drag::Create { tool, start } => {
                        let (cx, cy) = to_emu(cur);
                        if tool == Tool::Line {
                            let snap = ui.input(|i| i.modifiers.shift);
                            let (r, f) = line_drag(start, (cx, cy), snap);
                            band = Some(r);
                            line_flips = f;
                        } else {
                            band = Some(Rect::from_corners(start.0, start.1, cx, cy));
                        }
                    }
                }
                if resp.drag_stopped() {
                    self.drag = None;
                    match drag {
                        Drag::Move { id, .. } if dx != 0 || dy != 0 => {
                            requests.push(Request::Run(Command::MoveShapes {
                                page: self.page,
                                ids: if multi_on { sel_all.clone() } else { vec![id] },
                                dx,
                                dy,
                            }));
                        }
                        Drag::Marquee { add, .. } => {
                            if let Some(m) = marquee {
                                let mut ids = if add { sel_all.clone() } else { Vec::new() };
                                for id in shapes_in_rect(pg, m) {
                                    if !ids.contains(&id) {
                                        ids.push(id);
                                    }
                                }
                                apply_selection(&mut self.selection, &mut self.multi, ids);
                            }
                        }
                        Drag::Resize { id, .. } => {
                            if let Some((_, r)) = preview {
                                requests.push(Request::Run(Command::SetRect {
                                    page: self.page,
                                    id,
                                    rect: r,
                                }));
                            }
                        }
                        Drag::Create { tool, start } => {
                            let r = band.unwrap_or_else(|| default_rect(tool, start));
                            let tiny = r.w < in_to_emu(0.15) && r.h < in_to_emu(0.15);
                            let r = if tiny { default_rect(tool, start) } else { r };
                            let flips = if tiny { (false, false) } else { line_flips };
                            requests.push(Request::Create(
                                new_shape(tool, r, flips, line_color),
                                tool == Tool::TextBox,
                            ));
                        }
                        _ => {}
                    }
                    // `preview` and `group_move` stay set for this frame: the move or resize
                    // command only runs after painting, so clearing them here would draw the
                    // shape back at its old place for one frame (a visible flash).
                    band = None;
                    marquee = None;
                }
            }

            // --- Painting -----------------------------------------------------------
            // A soft drop shadow under the page.
            painter.add(
                egui::Shadow {
                    offset: [0, 3],
                    blur: 14,
                    spread: 0,
                    color: Color32::from_black_alpha(if crate::theme::is_dark() { 120 } else { 55 }),
                }
                .as_shape(page_rect, egui::CornerRadius::ZERO),
            );
            painter.rect_filled(page_rect, 0.0, PAGE_FILL);
            let content_painter: Painter = if show.scratch {
                painter.clone()
            } else {
                painter.with_clip_rect(page_rect)
            };
            if show.guides {
                let px = |v: i64| emu_to_px(v, zoom) as f32;
                let g = egui::Rect::from_min_max(
                    page_rect.min + Vec2::new(px(margin.left), px(margin.top)),
                    page_rect.max - Vec2::new(px(margin.right), px(margin.bottom)),
                );
                if g.width() > 0.0 && g.height() > 0.0 {
                    let st = Stroke::new(1.0, GUIDE);
                    for seg in egui::Shape::dashed_line(
                        &[
                            g.left_top(),
                            g.right_top(),
                            g.right_bottom(),
                            g.left_bottom(),
                            g.left_top(),
                        ],
                        st,
                        2.0,
                        3.0,
                    ) {
                        painter.add(seg);
                    }
                }
            }
            #[cfg(test)]
            {
                content_clip = content_painter.clip_rect();
            }
            // The master page's shapes sit behind the page's own; they are never selected,
            // hit-tested or edited (spec 4T.6), only drawn.
            let page_number = doc.page_number_text(self.page);
            {
                let mut master_view = PageView {
                    painter: &content_painter,
                    fonts: &self.fonts,
                    pics: &mut self.pics,
                    origin,
                    zoom,
                    boundaries: false,
                    shapes: &[],
                    editing_cell: None,
                };
                master_view.draw_master(ui.ctx(), doc.master_shapes(self.page), &page_number);
            }
            let mut view = PageView {
                painter: &content_painter,
                fonts: &self.fonts,
                pics: &mut self.pics,
                origin,
                zoom,
                boundaries: show.boundaries,
                shapes: &pg.shapes,
                editing_cell: edit_cell,
            };
            // Members travel with a selected group, and scale with a group being resized.
            let moving = pg.with_descendants(&sel_all);
            let resized_group = match (self.drag, preview) {
                (Some(Drag::Resize { id, orig, .. }), Some((_, new)))
                    if pg
                        .shape(id)
                        .is_some_and(|s| matches!(s.kind, ShapeKind::Group)) =>
                {
                    Some((pg.descendants(id), orig, new))
                }
                _ => None,
            };
            for s in &pg.shapes {
                let at = match group_move {
                    Some((dx, dy)) if moving.contains(&s.id) => Some(s.rect.translated(dx, dy)),
                    _ => match &resized_group {
                        Some((members, from, to)) if members.contains(&s.id) => {
                            Some(kernpage_core::geometry::scale_into(s.rect, *from, *to))
                        }
                        _ => preview.filter(|(id, _)| *id == s.id).map(|(_, r)| r),
                    },
                };
                #[cfg(test)]
                painted.push((s.id, view.screen_rect(at.unwrap_or(s.rect))));
                match s.with_page_number(&page_number).filter(|_| editing != Some(s.id)) {
                    Some(shown) => view.draw_shape(ui.ctx(), &shown, at, false),
                    None => view.draw_shape(ui.ctx(), s, at, editing == Some(s.id)),
                }
            }

            // Selection outline and handles.
            if let Some(id) = selection {
                if let Some(s) = pg.shape(id) {
                    let r = preview.filter(|(i, _)| *i == id).map_or(s.rect, |(_, r)| r);
                    let sr = view.screen_rect(r);
                    if editing == Some(id) {
                        for seg in egui::Shape::dashed_line(
                            &[
                                sr.left_top(),
                                sr.right_top(),
                                sr.right_bottom(),
                                sr.left_bottom(),
                                sr.left_top(),
                            ],
                            Stroke::new(1.5, ACCENT_COLOR),
                            4.0,
                            3.0,
                        ) {
                            painter.add(seg);
                        }
                    } else if tool == Tool::Select {
                        if !matches!(s.kind, ShapeKind::Line) {
                            painter.rect_stroke(
                                sr,
                                0.0,
                                Stroke::new(1.5, ACCENT_COLOR),
                                StrokeKind::Middle,
                            );
                        }
                        // The handles of a frame whose text overflows are red.
                        let handle_color = if overflow {
                            crate::linkmarks::OVERFLOW_RED
                        } else {
                            Color32::WHITE
                        };
                        #[cfg(test)]
                        {
                            handle_fill = Some(handle_color);
                        }
                        for h in Handle::ALL {
                            let (hx, hy) = handle_point(r, h);
                            let c = origin
                                + Vec2::new(emu_to_px(hx, zoom) as f32, emu_to_px(hy, zoom) as f32);
                            let hr = egui::Rect::from_center_size(c, Vec2::splat(HANDLE_PX));
                            painter.rect_filled(hr, 1.0, handle_color);
                            painter.rect_stroke(
                                hr,
                                1.0,
                                Stroke::new(1.5, ACCENT_COLOR),
                                StrokeKind::Middle,
                            );
                        }
                    }
                }
            }
            if link_prev || link_next || overflow {
                if let Some(s) = selection.and_then(|id| pg.shape(id)) {
                    let r = preview
                        .filter(|(i, _)| Some(*i) == selection)
                        .map_or(s.rect, |(_, r)| r);
                    let ms = crate::linkmarks::marks(
                        view.screen_rect(r),
                        link_prev,
                        link_next,
                        overflow,
                    );
                    crate::linkmarks::paint(&painter, &ms);
                    #[cfg(test)]
                    {
                        marks_seen = ms;
                    }
                }
            }
            // Several selected: each object's own outline plus one overall box.
            if multi_on && tool == Tool::Select {
                let (gx, gy) = group_move.unwrap_or((0, 0));
                let mut all: Option<Rect> = None;
                for id in &sel_all {
                    let Some(s) = pg.shape(*id) else { continue };
                    let b = kernpage_core::geometry::visual_bounds(s).translated(gx, gy);
                    let sr = view.screen_rect(b);
                    painter.rect_stroke(
                        sr,
                        0.0,
                        Stroke::new(1.0, ACCENT_COLOR),
                        StrokeKind::Middle,
                    );
                    all = Some(match all {
                        None => b,
                        Some(a) => {
                            let (l, t) = (a.x.min(b.x), a.y.min(b.y));
                            Rect::new(
                                l,
                                t,
                                a.right().max(b.right()) - l,
                                a.bottom().max(b.bottom()) - t,
                            )
                        }
                    });
                }
                if let Some(a) = all {
                    let sr = view.screen_rect(a).expand(2.0);
                    for seg in egui::Shape::dashed_line(
                        &[
                            sr.left_top(),
                            sr.right_top(),
                            sr.right_bottom(),
                            sr.left_bottom(),
                            sr.left_top(),
                        ],
                        Stroke::new(1.5, ACCENT_COLOR),
                        6.0,
                        3.0,
                    ) {
                        painter.add(seg);
                    }
                }
            }
            if let Some(m) = marquee {
                let sr = view.screen_rect(m);
                for seg in egui::Shape::dashed_line(
                    &[
                        sr.left_top(),
                        sr.right_top(),
                        sr.right_bottom(),
                        sr.left_bottom(),
                        sr.left_top(),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(0x30, 0x30, 0x30)),
                    2.0,
                    2.0,
                ) {
                    painter.add(seg);
                }
            }
            if let Some(b) = band {
                let sr = view.screen_rect(b);
                let st = Stroke::new(1.0, ACCENT_COLOR);
                if matches!(self.drag, Some(Drag::Create { tool: Tool::Line, .. })) {
                    let (a, b) = if line_flips.0 != line_flips.1 {
                        (sr.left_bottom(), sr.right_top())
                    } else {
                        (sr.left_top(), sr.right_bottom())
                    };
                    painter.line_segment([a, b], st);
                } else {
                    painter.rect_stroke(sr, 0.0, st, StrokeKind::Middle);
                }
            }

            // In-place text editor (a text box, or one cell of a table).
            if let Some(edit) = &mut self.edit {
                if let Some(s) = pg.shape(edit.id) {
                    let target = match (&s.kind, edit.cell) {
                        (ShapeKind::TextBox { paragraphs }, None) => {
                            Some((s.rect, paragraphs.as_slice(), shape_inset_px(s, zoom)))
                        }
                        (ShapeKind::Table { cells, .. }, Some(c)) => {
                            table_cell_rect(s, c).zip(cells.get(c)).map(|(r, cell)| {
                                (
                                    r,
                                    cell.paragraphs.as_slice(),
                                    [emu_to_px(cell.margins_emu[0], zoom) as f32; 4],
                                )
                            })
                        }
                        _ => None,
                    };
                    if let Some((rect, paragraphs, inset)) = target {
                        let sr = view.screen_rect(rect);
                        if edit.cell.is_some() {
                            painter.rect_stroke(
                                sr,
                                0.0,
                                Stroke::new(1.5, ACCENT_COLOR),
                                StrokeKind::Middle,
                            );
                        }
                        let base = paragraphs
                            .first()
                            .map(|p| p.base_run())
                            .unwrap_or_else(|| kernpage_core::Run::plain(""));
                        let font = self.fonts.font_id(
                            &base.font,
                            base.bold,
                            base.italic,
                            pt_to_px(f64::from(base.size_pt), zoom) as f32,
                        );
                        // At the edge of a linked frame some keys move on into the next or
                        // previous frame instead of the editor handling them.
                        if let Some(k) = self
                            .editor
                            .as_ref()
                            .and_then(|e| crate::app::linked_key(ui.ctx(), e.doc(), edit))
                        {
                            requests.push(Request::LinkedKey(k));
                        }
                        // Tab / Shift+Tab move between cells; never insert a tab.
                        let mut tab: Option<bool> = None;
                        if edit.cell.is_some() {
                            ui.input_mut(|i| {
                                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
                                    tab = Some(true);
                                } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                                    tab = Some(false);
                                }
                            });
                        }
                        // Lay the text out run by run so typing shows the formatting it
                        // will get: the paragraphs are rebuilt from the buffer each frame.
                        let fonts = &self.fonts;
                        let mut layouter =
                            |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                                let job = edit_job(fonts, paragraphs, text.as_str(), wrap, zoom);
                                ui.fonts_mut(|f| f.layout_job(job))
                            };
                        let te = egui::TextEdit::multiline(&mut edit.text)
                            .id(crate::app::edit_widget_id_of(edit.id, edit.cell))
                            .font(font)
                            .text_color(color(base.color))
                            .layouter(&mut layouter)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin {
                                left: inset[0].round().clamp(0.0, 100.0) as i8,
                                top: inset[1].round().clamp(0.0, 100.0) as i8,
                                right: inset[2].round().clamp(0.0, 100.0) as i8,
                                bottom: inset[3].round().clamp(0.0, 100.0) as i8,
                            })
                            .desired_width(sr.width())
                            .lock_focus(true)
                            .background_color(Color32::TRANSPARENT);
                        let edit_id = crate::app::edit_widget_id_of(edit.id, edit.cell);
                        if let Some(clip) = self.text_clip.take() {
                            // A menu click took the focus; give it back so the editor
                            // sees the clipboard event.
                            ui.memory_mut(|m| m.request_focus(edit_id));
                            ui.input_mut(|i| match clip {
                                TextClip::Cut => i.events.push(egui::Event::Cut),
                                TextClip::Copy => i.events.push(egui::Event::Copy),
                                TextClip::Paste => {}
                            });
                        }
                        let te_out = ui
                            .scope_builder(egui::UiBuilder::new().max_rect(sr), |ui| te.show(ui))
                            .inner;
                        crate::render::paint_edit_underlines(
                            ui.painter(),
                            &te_out.galley,
                            te_out.galley_pos,
                        );
                        let out = te_out.response;
                        if out.changed() {
                            requests.push(Request::LiveSync);
                        }
                        out.context_menu(|ui| {
                            if menu_item(ui, "Cut", true) {
                                requests.push(Request::TextClip(TextClip::Cut));
                            }
                            if menu_item(ui, "Copy", true) {
                                requests.push(Request::TextClip(TextClip::Copy));
                            }
                            if menu_item(ui, "Paste", true) {
                                requests.push(Request::TextClip(TextClip::Paste));
                            }
                            ui.separator();
                            if menu_item(ui, "Paragraph...", true) {
                                requests.push(Request::Spacing);
                            }
                            ui.menu_button("Align Text", |ui| {
                                for (label, a) in [
                                    ("Align Left", kernpage_core::Align::Left),
                                    ("Center", kernpage_core::Align::Center),
                                    ("Align Right", kernpage_core::Align::Right),
                                    ("Justify", kernpage_core::Align::Justify),
                                ] {
                                    if menu_item(ui, label, true) {
                                        requests.push(Request::Align(a));
                                    }
                                }
                            });
                        });
                        if edit.fresh {
                            out.request_focus();
                            edit.fresh = false;
                        }
                        if let Some(r) = self.pending_selection.take() {
                            let id = crate::app::edit_widget_id_of(edit.id, edit.cell);
                            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
                                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(r.start),
                                    egui::text::CCursor::new(r.end),
                                )));
                                st.store(ui.ctx(), id);
                            }
                        }
                        if let (Some(back), Some(c), Some(count)) = (tab, edit.cell, cell_count(s))
                        {
                            if let Some(n) = next_cell(c, count, back) {
                                requests.push(Request::BeginCellEdit(edit.id, n));
                            }
                        }
                        let opening = requests.iter().any(|r| {
                            matches!(r, Request::BeginEdit(_) | Request::BeginCellEdit(..))
                        });
                        // A click on a link arrow or the overflow box does not end the edit
                        // (the arrows continue it in another frame).
                        if (resp.clicked() || resp.drag_started()) && !opening && on_mark.is_none() {
                            requests.push(Request::CommitEdit);
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            requests.push(Request::CommitEdit);
                        }
                    }
                }
            }

            // Zoom with Ctrl + wheel.
            if resp.hovered() {
                let z = ui.input(|i| i.zoom_delta());
                if (z - 1.0).abs() > 1e-4 {
                    self.zoom = kernpage_core::geometry::clamp_zoom(zoom * f64::from(z));
                    self.zoom_mode = ZoomMode::Custom;
                }
            }
        });
        let _ = output;

        // Cursor position for the status bar.
        let hover = ui
            .ctx()
            .pointer_hover_pos()
            .filter(|p| canvas_rect.contains(*p));
        #[cfg(test)]
        {
            self.probe = Probe {
                origin: origin_out,
                zoom,
                painted,
                content_clip,
                ruler_h: None,
                ruler_v: None,
                marks: marks_seen,
                handle_fill,
                page_fill: PAGE_FILL,
            };
        }
        self.cursor_emu = hover.map(|p| {
            (
                px_to_emu(f64::from(p.x - origin_out.x), zoom),
                px_to_emu(f64::from(p.y - origin_out.y), zoom),
            )
        });

        if self.show.rulers {
            let p = ui.painter_at(full);
            p.rect_filled(
                egui::Rect::from_min_size(full.min, Vec2::splat(RULER)),
                0.0,
                ruler_bg(),
            );
            let h_rect = egui::Rect::from_min_max(
                Pos2::new(full.min.x + RULER, full.min.y),
                Pos2::new(full.max.x, full.min.y + RULER),
            );
            let v_rect = egui::Rect::from_min_max(
                Pos2::new(full.min.x, full.min.y + RULER),
                Pos2::new(full.min.x + RULER, full.max.y),
            );
            draw_ruler(
                &p,
                h_rect,
                true,
                origin_out.x,
                page_px.x,
                zoom,
                hover.map(|p| p.x),
            );
            #[cfg(test)]
            {
                let mark = |c: Option<f32>, lo: f32, hi: f32| c.filter(|c| *c >= lo && *c <= hi);
                self.probe.ruler_h = Some((h_rect, mark(hover.map(|p| p.x), h_rect.min.x, h_rect.max.x)));
                self.probe.ruler_v = Some((v_rect, mark(hover.map(|p| p.y), v_rect.min.y, v_rect.max.y)));
            }
            draw_ruler(
                &p,
                v_rect,
                false,
                origin_out.y,
                page_px.y,
                zoom,
                hover.map(|p| p.y),
            );
        }

        for r in requests {
            match r {
                Request::Run(c) => {
                    self.exec(c);
                }
                Request::Create(shape, edit) => {
                    if let Some(id) = self.add_shape(shape) {
                        if edit {
                            self.begin_edit(id);
                        }
                    }
                }
                Request::BeginEdit(id) => self.begin_edit(id),
                Request::BeginCellEdit(id, cell) => self.begin_cell_edit(id, cell),
                Request::CommitEdit => self.commit_edit(),
                Request::LiveSync => self.live_sync(ui.ctx()),
                Request::LinkedKey(k) => self.linked_edge_key(ui.ctx(), k),
                Request::LinkedGo(forward) => self.linked_step(ui.ctx(), forward),
                Request::Cut => {
                    self.copy_selection();
                    self.delete_selection();
                }
                Request::Copy => self.copy_selection(),
                Request::Paste => self.paste(),
                Request::Delete => self.delete_selection(),
                Request::Group => self.group_selection(),
                Request::Ungroup => self.ungroup_selection(),
                Request::ChangePicture => self.change_picture_dialog(),
                Request::RemovePicture => self.remove_picture(),
                Request::Forward | Request::Backward | Request::Front | Request::Back => {
                    let ids = self.selected_ids();
                    if !ids.is_empty() {
                        use kernpage_core::ZOrder;
                        let op = match r {
                            Request::Forward => ZOrder::Forward,
                            Request::Front => ZOrder::Front,
                            Request::Backward => ZOrder::Backward,
                            _ => ZOrder::Back,
                        };
                        self.exec(Command::ZOrderShapes {
                            page: self.page,
                            ids,
                            op,
                        });
                    }
                }
                Request::Zoom(mode) => {
                    self.zoom_mode = mode;
                    if mode == ZoomMode::Custom {
                        self.zoom = 1.0;
                    }
                    self.reset_scroll = true;
                }
                Request::TextClip(clip) => {
                    if clip == TextClip::Paste {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                        self.text_clip = None;
                    } else {
                        self.text_clip = Some(clip);
                    }
                    if let Some(e) = &mut self.edit {
                        e.fresh = true;
                    }
                    ui.ctx().request_repaint();
                }
                Request::Spacing => self.open_spacing_dialog(ui.ctx()),
                Request::Align(a) => self.format_align(ui.ctx(), a),
            }
        }
    }
}

/// A ruler label: the distance in inches from the page origin, never signed (Publisher shows
/// the same number on both sides of the origin).
fn ruler_label(v: f32) -> String {
    let v = v.abs();
    if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.2}")
    }
}

fn draw_ruler(
    p: &Painter,
    rect: egui::Rect,
    horizontal: bool,
    origin: f32,
    page_len: f32,
    zoom: f64,
    cursor: Option<f32>,
) {
    let p = p.with_clip_rect(rect);
    p.rect_filled(rect, 0.0, ruler_bg());
    let (lo, hi) = if horizontal {
        (rect.min.x, rect.max.x)
    } else {
        (rect.min.y, rect.max.y)
    };
    // The part of the ruler over the page is lighter.
    let page_part = if horizontal {
        egui::Rect::from_min_max(
            Pos2::new(origin, rect.min.y),
            Pos2::new(origin + page_len, rect.max.y),
        )
    } else {
        egui::Rect::from_min_max(
            Pos2::new(rect.min.x, origin),
            Pos2::new(rect.max.x, origin + page_len),
        )
    };
    p.rect_filled(
        page_part.intersect(rect),
        0.0,
        ruler_page(),
    );

    let ppi = (96.0 * zoom) as f32;
    let label_step = [0.25f32, 0.5, 1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .find(|s| s * ppi >= 44.0)
        .unwrap_or(10.0);
    let minor = [
        label_step / 8.0,
        label_step / 4.0,
        label_step / 2.0,
        label_step,
    ]
    .into_iter()
    .find(|m| m * ppi >= 5.0)
    .unwrap_or(label_step);
    let k0 = ((lo - origin) / ppi / minor).floor() as i64;
    let k1 = ((hi - origin) / ppi / minor).ceil() as i64;
    let ink = Stroke::new(1.0, ruler_ink());
    let thick = if horizontal {
        rect.height()
    } else {
        rect.width()
    };
    let near = |v: f32, step: f32| ((v / step) - (v / step).round()).abs() < 1e-3;
    for k in k0..=k1 {
        let v = k as f32 * minor;
        let pos = origin + v * ppi;
        if pos < lo || pos > hi {
            continue;
        }
        let len = if near(v, label_step) {
            thick * 0.65
        } else if near(v, label_step / 2.0) {
            thick * 0.45
        } else {
            thick * 0.25
        };
        let (a, b) = if horizontal {
            (Pos2::new(pos, rect.max.y), Pos2::new(pos, rect.max.y - len))
        } else {
            (Pos2::new(rect.max.x, pos), Pos2::new(rect.max.x - len, pos))
        };
        p.line_segment([a, b], ink);
        if near(v, label_step) {
            let text = ruler_label(v);
            let font = egui::FontId::proportional(9.0);
            if horizontal {
                p.text(
                    Pos2::new(pos + 2.0, rect.min.y + 1.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    font,
                    ruler_ink(),
                );
            } else {
                p.text(
                    Pos2::new(rect.min.x + 2.0, pos + 1.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    font,
                    ruler_ink(),
                );
            }
        }
    }
    // Edge lines and the cursor marker.
    let edge = Stroke::new(1.0, ruler_edge());
    if horizontal {
        p.line_segment([rect.left_bottom(), rect.right_bottom()], edge);
    } else {
        p.line_segment([rect.right_top(), rect.right_bottom()], edge);
    }
    if let Some(c) = cursor.filter(|c| *c >= lo && *c <= hi) {
        let m = Stroke::new(1.5, ACCENT_COLOR);
        if horizontal {
            p.line_segment([Pos2::new(c, rect.min.y), Pos2::new(c, rect.max.y)], m);
        } else {
            p.line_segment([Pos2::new(rect.min.x, c), Pos2::new(rect.max.x, c)], m);
        }
    }
    let _ = emu_to_in(0);
}

/// EMU offset of a drag from where the press began. The press position is stored when the
/// drag starts because egui clears `press_origin()` on the frame the button is released.
fn drag_delta(press: Pos2, cur: Pos2, zoom: f64) -> (i64, i64) {
    (
        px_to_emu(f64::from(cur.x - press.x), zoom),
        px_to_emu(f64::from(cur.y - press.y), zoom),
    )
}

#[cfg(test)]
mod drag_delta_tests {
    use super::*;

    #[test]
    fn delta_is_cursor_minus_press_scaled_by_zoom() {
        let (dx, dy) = drag_delta(Pos2::new(10.0, 20.0), Pos2::new(10.0, 20.0), 1.0);
        assert_eq!((dx, dy), (0, 0));
        let (dx, dy) = drag_delta(Pos2::new(10.0, 20.0), Pos2::new(106.0, 20.0), 1.0);
        assert_eq!(dx, px_to_emu(96.0, 1.0));
        assert_eq!(dy, 0);
        let (dx, _) = drag_delta(Pos2::new(0.0, 0.0), Pos2::new(96.0, 0.0), 2.0);
        assert_eq!(dx, px_to_emu(96.0, 2.0));
    }

    #[test]
    fn release_frame_uses_stored_press_not_current_position() {
        // On release egui reports no press_origin, so the old code fell back to `cur`
        // and got zero. With the press stored at drag start the delta is non-zero.
        let stored_press = Pos2::new(100.0, 100.0);
        let release = Pos2::new(150.0, 130.0);
        let (dx, dy) = drag_delta(stored_press, release, 1.0);
        assert!(dx > 0 && dy > 0);
        assert_eq!(drag_delta(release, release, 1.0), (0, 0));
    }
}

#[cfg(test)]
mod shape_default_tests {
    use super::*;

    #[test]
    fn new_shapes_use_publisher_defaults() {
        let scheme0 = Rgb(10, 20, 30);
        for tool in [Tool::Rectangle, Tool::Oval, Tool::Line] {
            let s = new_shape(tool, Rect::new(0, 0, 100, 100), (false, false), scheme0);
            assert_eq!(s.fill, None);
            assert_eq!(s.line.unwrap().color, scheme0);
            assert_eq!(s.line.unwrap().width_emu, 9_525);
        }
    }

    #[test]
    fn dragged_lines_keep_their_direction() {
        let o = (1000, 1000);
        let (r, f) = line_drag(o, (3000, 400), false);
        assert_eq!((r, f), (Rect::new(1000, 400, 2000, 600), (false, true)));
        assert_eq!(line_drag(o, (400, 3000), false).1, (true, false));
        assert_eq!(line_drag(o, (400, 400), false).1, (true, true));
        assert_eq!(line_drag(o, (3000, 3000), false).1, (false, false));
        // Shift: nearly horizontal snaps flat, diagonal-ish snaps to 45 degrees.
        assert_eq!(line_drag(o, (3000, 1200), true).0, Rect::new(1000, 1000, 2000, 0));
        assert_eq!(line_drag(o, (3000, 2500), true).0, Rect::new(1000, 1000, 2000, 2000));
    }
}

#[cfg(test)]
impl Default for Probe {
    fn default() -> Self {
        Probe {
            origin: Pos2::ZERO,
            zoom: 0.0,
            painted: Vec::new(),
            content_clip: egui::Rect::NOTHING,
            ruler_h: None,
            ruler_v: None,
            marks: Vec::new(),
            handle_fill: None,
            page_fill: Color32::TRANSPARENT,
        }
    }
}

#[cfg(test)]
mod ruler_tests {
    use super::ruler_label;

    #[test]
    fn labels_are_distances_without_a_minus_sign() {
        assert_eq!(ruler_label(-2.0), "2");
        assert_eq!(ruler_label(2.0), "2");
        assert_eq!(ruler_label(-0.5), "0.50");
        assert_eq!(ruler_label(0.0), "0");
    }
}
