//! Tables in the ribbon (docs/ui/TABLES.md): Insert > Table (the 10 x 8 grid and the Create
//! Table dialog) and the Rows & Columns group of the Layout tab (Insert Above / Below / Left
//! / Right, Delete Rows / Columns / Table). Every action is one core command, so one undo step.

use crate::app::App;
use crate::icons::Icon;
use crate::widgets::{group, Btn, ACCENT_COLOR};
use eframe::egui::{self, Color32, Key, Popup, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use kernpage_core::table::default_table_rect;
use kernpage_core::{Command, ShapeKind};

/// Squares of the Insert > Table grid: 10 columns wide, 8 rows high.
const GRID_COLS: usize = 10;
const GRID_ROWS: usize = 8;
const SQUARE: f32 = 20.0;
const ERROR: Color32 = Color32::from_rgb(0xC0, 0x30, 0x30);

/// The name Publisher gives the grid square at 0-based (`col`, `row`): `3x2 Table`.
pub fn grid_label(col: usize, row: usize) -> String {
    format!("{}x{} Table", col + 1, row + 1)
}

/// The grid square under `pos` (0-based column, row) in a grid whose top left is `origin`.
pub fn grid_square_at(origin: egui::Pos2, pos: egui::Pos2) -> Option<(usize, usize)> {
    let (dx, dy) = (pos.x - origin.x, pos.y - origin.y);
    if dx < 0.0 || dy < 0.0 {
        return None;
    }
    let (c, r) = ((dx / SQUARE) as usize, (dy / SQUARE) as usize);
    (c < GRID_COLS && r < GRID_ROWS).then_some((c, r))
}

/// Create Table dialog values: what is typed in the two boxes.
pub struct TableDialog {
    pub rows: String,
    pub cols: String,
    /// Put the cursor in the first box on the first frame.
    fresh: bool,
}

impl TableDialog {
    /// Publisher's first values (TABLES.md section 2): 2 rows, 6 columns.
    pub fn new() -> Self {
        TableDialog {
            rows: "2".into(),
            cols: "6".into(),
            fresh: true,
        }
    }

    /// (rows, columns), or a message per box that is not a usable count.
    pub fn values(&self) -> Result<(u32, u32), (Option<String>, Option<String>)> {
        let (r, c) = (parse_count(&self.rows), parse_count(&self.cols));
        match (r, c) {
            (Ok(r), Ok(c)) => Ok((r, c)),
            (r, c) => Err((r.err(), c.err())),
        }
    }
}

/// A whole number of 1 or more.
pub fn parse_count(text: &str) -> Result<u32, String> {
    match text.trim().parse::<u32>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err("Enter a whole number of 1 or more.".to_owned()),
    }
}

/// The cell the text cursor is in, with the size of its table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Caret {
    pub id: kernpage_core::ShapeId,
    pub rows: usize,
    pub cols: usize,
    pub row: usize,
    pub col: usize,
}

/// Which way a row or column goes in or out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Insert {
    Above,
    Below,
    Left,
    Right,
}

/// Test seam: the grid of the Table drop-down as drawn in the last frame.
#[cfg(test)]
pub mod probe {
    use eframe::egui::Rect;
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
    pub struct Grid {
        pub rect: Rect,
        /// The text shown for the hovered square (empty when none).
        pub label: String,
        /// Squares highlighted (columns, rows), from the top left.
        pub highlighted: (usize, usize),
    }

    thread_local! {
        static GRID: RefCell<Option<Grid>> = const { RefCell::new(None) };
    }

    pub fn set(g: Grid) {
        GRID.with(|v| *v.borrow_mut() = Some(g));
    }
    pub fn clear() {
        GRID.with(|v| *v.borrow_mut() = None);
    }
    pub fn grid() -> Option<Grid> {
        GRID.with(|v| v.borrow().clone())
    }
}

impl App {
    /// The Table drop-down (Insert > Tables, Home > Objects): the grid and "Insert Table...".
    pub(crate) fn table_menu(&mut self, ui: &mut Ui) {
        let has_doc = self.editor.is_some();
        let r = Btn::large(Icon::Table, "Table")
            .enabled(has_doc)
            .dropdown()
            .show(ui);
        #[cfg(test)]
        if ui.is_visible() {
            probe::clear();
        }
        let mut pick = None;
        let mut dialog = false;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(GRID_COLS as f32 * SQUARE);
            ui.label(RichText::new("Insert Table").strong().color(crate::theme::text()));
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(GRID_COLS as f32 * SQUARE, GRID_ROWS as f32 * SQUARE),
                Sense::click(),
            );
            let hover = resp
                .hover_pos()
                .and_then(|p| grid_square_at(rect.min, p));
            let p = ui.painter();
            for row in 0..GRID_ROWS {
                for col in 0..GRID_COLS {
                    let sq = egui::Rect::from_min_size(
                        rect.min + Vec2::new(col as f32 * SQUARE, row as f32 * SQUARE),
                        Vec2::splat(SQUARE),
                    )
                    .shrink(1.0);
                    let on = hover.is_some_and(|(hc, hr)| col <= hc && row <= hr);
                    p.rect_filled(
                        sq,
                        0.0,
                        if on {
                            ACCENT_COLOR.gamma_multiply(0.35)
                        } else {
                            Color32::WHITE
                        },
                    );
                    p.rect_stroke(
                        sq,
                        0.0,
                        Stroke::new(1.0, if on { ACCENT_COLOR } else { crate::theme::button_outline() }),
                        StrokeKind::Inside,
                    );
                }
            }
            let label = hover.map(|(c, r)| grid_label(c, r)).unwrap_or_default();
            ui.label(RichText::new(if label.is_empty() { " " } else { &label }).color(crate::theme::text()));
            ui.separator();
            let insert_table = ui.add(
                egui::Button::new("Insert Table...").min_size(Vec2::new(GRID_COLS as f32 * SQUARE, 0.0)),
            );
            #[cfg(test)]
            probe::set(probe::Grid {
                rect,
                label: label.clone(),
                highlighted: hover.map_or((0, 0), |(c, r)| (c + 1, r + 1)),
            });
            if resp.clicked() {
                pick = resp
                    .interact_pointer_pos()
                    .and_then(|p| grid_square_at(rect.min, p))
                    .map(|(c, r)| (r as u32 + 1, c as u32 + 1));
                ui.close();
            }
            if insert_table.clicked() {
                dialog = true;
                ui.close();
            }
        });
        if let Some((rows, cols)) = pick {
            self.insert_table(rows, cols);
        }
        if dialog {
            self.table_dialog = Some(TableDialog::new());
        }
    }

    /// Insert a `rows` x `columns` table the way Publisher's grid does (default size, centred
    /// on the current page) and select it.
    pub fn insert_table(&mut self, rows: u32, columns: u32) {
        if self.editor.is_none() {
            return;
        }
        let (w, h) = self.page_size();
        let ids = self.exec(Command::InsertTable {
            page: self.page,
            rect: default_table_rect(w, h, rows, columns),
            rows,
            columns,
        });
        if let Some(&id) = ids.first() {
            self.selection = Some(id);
            self.multi.clear();
            self.tool = crate::app::Tool::Select;
        }
    }

    /// The Create Table dialog.
    #[allow(deprecated)]
    pub(crate) fn table_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.table_dialog else {
            return;
        };
        let (mut ok, mut cancel) = (false, false);
        let errors = d.values().err().unwrap_or((None, None));
        let fresh = std::mem::take(&mut d.fresh);
        egui::Modal::new(egui::Id::new("table_dialog")).show(ctx, |ui| {
            ui.set_width(250.0);
            ui.heading("Create Table");
            ui.separator();
            for (i, (label, err)) in [("Number of rows:", &errors.0), ("Number of columns:", &errors.1)]
                .into_iter()
                .enumerate()
            {
                let text = if i == 0 { &mut d.rows } else { &mut d.cols };
                ui.horizontal(|ui| {
                    ui.add_sized([120.0, 20.0], egui::Label::new(label));
                    let r = ui.add(
                        egui::TextEdit::singleline(text)
                            .id(egui::Id::new(("table_dialog_field", i)))
                            .desired_width(60.0)
                            .text_color(if err.is_some() { ERROR } else { crate::theme::text() }),
                    );
                    if fresh && i == 0 {
                        r.request_focus();
                        if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                            st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(text.chars().count()),
                            )));
                            st.store(ui.ctx(), r.id);
                        }
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for up in [true, false] {
                            // Not focusable: Tab goes from box to box.
                            let (ar, a) = ui.allocate_exact_size(Vec2::new(16.0, 10.0), Sense::CLICK);
                            let bg = if a.hovered() {
                                ACCENT_COLOR.gamma_multiply(0.3)
                            } else {
                                Color32::from_gray(228)
                            };
                            ui.painter().rect_filled(ar, 1.0, bg);
                            let c = ar.center();
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
                                if let Ok(n) = parse_count(text) {
                                    *text = if up { n.saturating_add(1) } else { n.saturating_sub(1).max(1) }
                                        .to_string();
                                }
                            }
                        }
                    });
                });
                if let Some(e) = err {
                    ui.add(egui::Label::new(RichText::new(e.as_str()).size(11.0).color(ERROR)).wrap());
                }
            }
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
            // Invalid input keeps the dialog open; the messages are already shown.
            if let Some(Ok((rows, cols))) = self.table_dialog.as_ref().map(TableDialog::values) {
                self.table_dialog = None;
                self.insert_table(rows, cols);
            }
        } else if cancel {
            self.table_dialog = None;
        }
    }

    /// The cell with the text cursor, when a table is selected and one of its cells is being
    /// edited (Publisher enables Insert and Delete Rows / Columns only for a cell).
    pub(crate) fn table_caret(&self) -> Option<Caret> {
        let edit = self.edit.as_ref()?;
        let cell = edit.cell?;
        if edit.page != self.page || self.selected()?.id != edit.id {
            return None;
        }
        let shape = self.editor.as_ref()?.doc().shape(self.page, edit.id)?;
        let ShapeKind::Table { rows, cols, .. } = &shape.kind else {
            return None;
        };
        let (rows, cols) = ((*rows).max(1) as usize, (*cols).max(1) as usize);
        (cell < rows * cols).then_some(Caret {
            id: edit.id,
            rows,
            cols,
            row: cell / cols,
            col: cell % cols,
        })
    }

    /// Layout > Insert Above / Below / Left / Right: one empty row or column next to the
    /// caret cell; the caret moves into the new row or column (same column or row).
    fn table_insert(&mut self, at: Insert) {
        let Some(c) = self.table_caret() else { return };
        let (page, shape) = (self.page, c.id);
        let (cmd, row, col, cols) = match at {
            Insert::Above => (
                Command::InsertTableRows { page, shape, before: c.row, count: 1 },
                c.row,
                c.col,
                c.cols,
            ),
            Insert::Below => (
                Command::InsertTableRows { page, shape, before: c.row + 1, count: 1 },
                c.row + 1,
                c.col,
                c.cols,
            ),
            Insert::Left => (
                Command::InsertTableColumns { page, shape, before: c.col, count: 1 },
                c.row,
                c.col,
                c.cols + 1,
            ),
            Insert::Right => (
                Command::InsertTableColumns { page, shape, before: c.col + 1, count: 1 },
                c.row,
                c.col + 1,
                c.cols + 1,
            ),
        };
        let rev = self.revision();
        self.exec(cmd);
        let cell = if self.revision() != rev {
            row * cols + col
        } else {
            c.row * c.cols + c.col
        };
        self.begin_cell_edit(shape, cell);
    }

    /// Layout > Delete > Delete Rows / Delete Columns: the caret cell's row or column; the
    /// caret goes to the row or column that took its place (the one before when it was last).
    fn table_delete_track(&mut self, rows: bool) {
        let Some(c) = self.table_caret() else { return };
        let (page, shape) = (self.page, c.id);
        let cmd = if rows {
            Command::DeleteTableRows { page, shape, first: c.row, count: 1 }
        } else {
            Command::DeleteTableColumns { page, shape, first: c.col, count: 1 }
        };
        let rev = self.revision();
        self.exec(cmd);
        let cell = if self.revision() == rev {
            c.row * c.cols + c.col
        } else if rows {
            c.row.min(c.rows - 2) * c.cols + c.col
        } else {
            c.row * (c.cols - 1) + c.col.min(c.cols - 2)
        };
        self.begin_cell_edit(shape, cell);
    }

    fn revision(&self) -> u64 {
        self.editor.as_ref().map_or(0, |e| e.revision())
    }

    /// Layout > Rows & Columns: Delete (menu), Insert Above, Insert Below / Left / Right.
    pub(crate) fn rows_columns_group(&mut self, ui: &mut Ui) {
        let caret = self.table_caret();
        let has_table = self.selected().is_some_and(|s| matches!(s.kind, ShapeKind::Table { .. }));
        let mut insert = None;
        let (mut del_rows, mut del_cols, mut del_table) = (false, false, false);
        group(ui, "Rows & Columns", false, |ui| {
            let r = Btn::large(Icon::TableDelete, "Delete")
                .enabled(has_table)
                .dropdown()
                .show(ui);
            Popup::menu(&r).show(|ui| {
                ui.set_min_width(140.0);
                // Publisher's order (TABLES.md section 3.2).
                if ui.add_enabled(caret.is_some(), egui::Button::new("Delete Columns")).clicked() {
                    del_cols = true;
                    ui.close();
                }
                if ui.add_enabled(caret.is_some(), egui::Button::new("Delete Rows")).clicked() {
                    del_rows = true;
                    ui.close();
                }
                if ui.add_enabled(has_table, egui::Button::new("Delete Table")).clicked() {
                    del_table = true;
                    ui.close();
                }
            });
            let on = caret.is_some();
            if Btn::large(Icon::TableAbove, "Insert\nAbove").enabled(on).show(ui).clicked() {
                insert = Some(Insert::Above);
            }
            crate::widgets::col(ui, |ui| {
                for (icon, label, at) in [
                    (Icon::TableBelow, "Insert Below", Insert::Below),
                    (Icon::TableLeft, "Insert Left", Insert::Left),
                    (Icon::TableRight, "Insert Right", Insert::Right),
                ] {
                    if Btn::small(icon, label).enabled(on).show(ui).clicked() {
                        insert = Some(at);
                    }
                }
            });
        });
        if let Some(at) = insert {
            self.table_insert(at);
        }
        if del_rows {
            self.table_delete_track(true);
        } else if del_cols {
            self.table_delete_track(false);
        } else if del_table {
            self.delete_selection();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_squares_are_named_columns_x_rows() {
        assert_eq!(grid_label(2, 1), "3x2 Table");
        assert_eq!(grid_label(9, 7), "10x8 Table");
        let o = egui::pos2(100.0, 50.0);
        assert_eq!(grid_square_at(o, egui::pos2(110.0, 60.0)), Some((0, 0)));
        assert_eq!(grid_square_at(o, egui::pos2(100.0 + 9.5 * SQUARE, 50.0 + 7.5 * SQUARE)), Some((9, 7)));
        assert_eq!(grid_square_at(o, egui::pos2(100.0 + 10.0 * SQUARE, 60.0)), None);
        assert_eq!(grid_square_at(o, egui::pos2(99.0, 60.0)), None);
    }

    #[test]
    fn dialog_starts_with_publishers_values_and_checks_counts() {
        let mut d = TableDialog::new();
        assert_eq!(d.values(), Ok((2, 6)));
        d.cols = " 4 ".into();
        assert_eq!(d.values(), Ok((2, 4)));
        d.rows = "0".into();
        d.cols = "x".into();
        let (r, c) = d.values().unwrap_err();
        assert!(r.is_some() && c.is_some());
        d.rows = "-3".into();
        assert!(parse_count(&d.rows).is_err());
        assert!(parse_count("").is_err());
    }
}
