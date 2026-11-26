//! Tables: Insert > Table (the 10 x 8 grid, Insert Table... and its Create Table dialog) and
//! Layout > Rows & Columns (Insert Above / Below / Left / Right, Delete Rows / Columns /
//! Table), docs/ui/TABLES.md. Everything is driven with real pointer clicks and key presses.

use super::support::*;
use crate::table_ui::probe;
use eframe::egui::{accesskit::Role, Key, Modifiers, Pos2, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::table::default_table_rect;
use kernpage_core::{ShapeId, ShapeKind};

const NONE: Modifiers = Modifiers::NONE;

fn blank() -> TestUi {
    TestUi::with_doc(letter_doc(vec![]))
}

/// Insert tab, open the Table drop-down.
fn open_table_menu(ui: &mut TestUi) {
    ui.click_tab("Insert");
    ui.click_ribbon("Table");
    assert!(probe::grid().is_some(), "the Table drop-down shows its grid");
}

/// Centre of grid square (`col`, `row`), 0-based, on screen.
fn square(col: usize, row: usize) -> Pos2 {
    let g = probe::grid().expect("grid is open");
    g.rect.min + Vec2::new((col as f32 + 0.5) * 20.0, (row as f32 + 0.5) * 20.0)
}

/// Pick the `cols` x `rows` square of the grid with the mouse.
fn pick(ui: &mut TestUi, cols: usize, rows: usize) {
    open_table_menu(ui);
    ui.click_at(square(cols - 1, rows - 1));
}

/// A new table of `cols` x `rows` inserted through the grid; returns its id.
fn table_ui(cols: usize, rows: usize) -> (TestUi, ShapeId) {
    let mut ui = blank();
    pick(&mut ui, cols, rows);
    let id = ui.selected()[0];
    (ui, id)
}

fn dims(ui: &TestUi, id: ShapeId) -> (u32, u32) {
    match &ui.shape(id).kind {
        ShapeKind::Table { rows, cols, .. } => (*rows, *cols),
        k => panic!("not a table: {k:?}"),
    }
}

fn cell_count(ui: &TestUi, id: ShapeId) -> usize {
    match &ui.shape(id).kind {
        ShapeKind::Table { cells, .. } => cells.len(),
        _ => 0,
    }
}

/// Screen position of the middle of a cell, from the stored track sizes.
fn cell_at(ui: &TestUi, id: ShapeId, row: usize, col: usize) -> Pos2 {
    let s = ui.shape(id);
    let ShapeKind::Table { col_widths_emu, row_heights_emu, .. } = &s.kind else { panic!() };
    let x: i64 = col_widths_emu[..col].iter().sum::<i64>() + col_widths_emu[col] / 2;
    let y: i64 = row_heights_emu[..row].iter().sum::<i64>() + row_heights_emu[row] / 2;
    ui.at_emu(s.rect.x + x, s.rect.y + y)
}

/// Put the text cursor in a cell (the table is selected, so one click enters the cell).
fn click_cell(ui: &mut TestUi, id: ShapeId, row: usize, col: usize) {
    let at = cell_at(ui, id, row, col);
    ui.click_at(at);
    assert_eq!(ui.app().edit.as_ref().map(|e| (e.id, e.cell)), Some((id, Some(row * dims(ui, id).1 as usize + col))));
}

fn caret_cell(ui: &mut TestUi) -> Option<usize> {
    ui.app().edit.as_ref().and_then(|e| e.cell)
}

fn on(ui: &TestUi, name: &str) -> bool {
    ui.ribbon(name).enabled
}

fn menu_item_enabled(ui: &TestUi, label: &str) -> bool {
    !ui.h.get_by_label(label).accesskit_node().is_disabled()
}

fn click_menu(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

/// Layout tab, then a Delete menu item.
fn delete_menu(ui: &mut TestUi, item: &str) {
    ui.click_tab("Layout");
    ui.click_ribbon("Delete");
    click_menu(ui, item);
}

fn dialog_fields(ui: &mut TestUi) -> Vec<Pos2> {
    ui.h.get_all_by_role(Role::TextInput)
        .map(|n| n.accesskit_node().raw_bounds().expect("bounds"))
        .map(|r| Pos2::new(((r.x0 + r.x1) / 2.0) as f32, ((r.y0 + r.y1) / 2.0) as f32))
        .collect()
}

/// Replace the text of dialog box `i` (0 rows, 1 columns) by typing.
fn fill(ui: &mut TestUi, i: usize, text: &str) {
    let at = dialog_fields(ui)[i];
    ui.click_at(at);
    ui.key(CTRL, Key::A);
    ui.type_text(text);
}

fn open_create_table(ui: &mut TestUi) {
    open_table_menu(ui);
    ui.h.get_by_label("Insert Table...").click();
    ui.settle();
    assert!(ui.app().table_dialog.is_some(), "the Create Table dialog is open");
}

// ---- the Table drop-down --------------------------------------------------------------------------------------

#[test]
fn the_table_button_opens_a_ten_by_eight_grid_named_columns_x_rows() {
    let mut ui = blank();
    open_table_menu(&mut ui);
    let g = probe::grid().unwrap();
    assert_eq!((g.rect.width(), g.rect.height()), (200.0, 160.0), "10 columns x 8 rows of squares");
    assert_eq!(g.label, "", "no square is hovered yet");
    assert!(ui.h.query_by_label("Insert Table...").is_some(), "the command row under the grid");
    assert!(ui.h.query_by_label("Insert Table").is_some(), "the section header");

    ui.mouse_to(square(2, 1));
    ui.settle();
    let g = probe::grid().unwrap();
    assert_eq!(g.label, "3x2 Table");
    assert_eq!(g.highlighted, (3, 2), "the block from the top left square to the hovered one");
    ui.mouse_to(square(9, 7));
    ui.settle();
    let g = probe::grid().unwrap();
    assert_eq!((g.label.as_str(), g.highlighted), ("10x8 Table", (10, 8)));
    ui.mouse_to(square(0, 0));
    ui.settle();
    assert_eq!(probe::grid().unwrap().label, "1x1 Table");
    ui.mouse_to(square(9, 7) + Vec2::new(40.0, 0.0));
    ui.settle();
    assert_eq!(probe::grid().unwrap().label, "", "off the grid nothing is highlighted");
    assert!(ui.shapes().is_empty(), "hovering inserts nothing");
}

#[test]
fn clicking_three_by_two_inserts_the_default_table_and_selects_it() {
    let mut ui = blank();
    pick(&mut ui, 3, 2);
    let (pw, ph) = ui.page_size(0);
    assert_eq!(ui.shapes().len(), 1);
    let id = ui.selected()[0];
    assert_eq!(dims(&ui, id), (2, 3), "2 rows x 3 columns");
    assert_eq!(cell_count(&ui, id), 6);
    assert_eq!(ui.rect_of(id), default_table_rect(pw, ph, 2, 3));
    assert_eq!(ui.app().tool, crate::app::Tool::Select);
    assert_eq!(ui.tab_names().iter().skip(7).cloned().collect::<Vec<_>>(), ["Table Design", "Layout"]);
    assert_eq!(ui.undo_steps(), 1, "one undo step");
    ui.undo();
    assert!(ui.shapes().is_empty(), "undo removes the table");
}

#[test]
fn the_biggest_square_inserts_ten_columns_and_eight_rows() {
    let (ui, id) = table_ui(10, 8);
    assert_eq!(dims(&ui, id), (8, 10));
    assert_eq!(cell_count(&ui, id), 80);
}

#[test]
fn the_table_is_inserted_on_the_current_page() {
    let mut ui = blank();
    ui.app().insert_page();
    ui.settle();
    assert_eq!(ui.app().page, 1);
    pick(&mut ui, 2, 2);
    assert_eq!(ui.doc().pages[0].shapes.len(), 0);
    assert_eq!(ui.doc().pages[1].shapes.len(), 1);
}

#[test]
fn the_table_button_is_live_on_home_too_and_does_the_same() {
    let mut ui = blank();
    ui.click_tab("Home");
    ui.click_ribbon("Table");
    ui.click_at(square(1, 0));
    let id = ui.selected()[0];
    assert_eq!(dims(&ui, id), (1, 2));
}

// ---- Insert Table... (Create Table) ----------------------------------------------------------------------------

#[test]
fn insert_table_dialog_starts_with_two_rows_and_six_columns() {
    let mut ui = blank();
    open_create_table(&mut ui);
    assert!(ui.h.query_by_label("Create Table").is_some());
    assert!(ui.h.query_by_label("Number of rows:").is_some());
    assert!(ui.h.query_by_label("Number of columns:").is_some());
    let d = ui.app().table_dialog.as_ref().unwrap();
    assert_eq!((d.rows.as_str(), d.cols.as_str()), ("2", "6"));
    assert!(ui.shapes().is_empty(), "nothing is inserted until OK");
}

#[test]
fn typed_values_and_enter_insert_the_table() {
    let mut ui = blank();
    open_create_table(&mut ui);
    fill(&mut ui, 0, "4");
    fill(&mut ui, 1, "3");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().table_dialog.is_none(), "the dialog closed");
    let id = ui.selected()[0];
    assert_eq!(dims(&ui, id), (4, 3));
    let (pw, ph) = ui.page_size(0);
    assert_eq!(ui.rect_of(id), default_table_rect(pw, ph, 4, 3));
    assert_eq!(ui.undo_steps(), 1);
}

#[test]
fn the_ok_button_inserts_too_and_the_untouched_defaults_make_two_by_six() {
    let mut ui = blank();
    open_create_table(&mut ui);
    ui.h.get_by_label_contains("OK").click();
    ui.settle();
    let id = ui.selected()[0];
    assert_eq!(dims(&ui, id), (2, 6));
}

#[test]
fn escape_and_cancel_close_the_dialog_without_a_table() {
    let mut ui = blank();
    open_create_table(&mut ui);
    fill(&mut ui, 0, "5");
    ui.key(NONE, Key::Escape);
    assert!(ui.app().table_dialog.is_none());
    assert!(ui.shapes().is_empty());
    assert!(!ui.can_undo());
    open_create_table(&mut ui);
    ui.h.get_by_label_contains("Cancel").click();
    ui.settle();
    assert!(ui.app().table_dialog.is_none());
    assert!(ui.shapes().is_empty());
}

#[test]
fn invalid_counts_keep_the_dialog_open_with_a_message_until_fixed() {
    let mut ui = blank();
    open_create_table(&mut ui);
    for bad in ["0", "abc", "", "-2"] {
        fill(&mut ui, 0, bad);
        ui.key(NONE, Key::Enter);
        assert!(ui.app().table_dialog.is_some(), "{bad:?} keeps the dialog open");
        assert!(ui.shapes().is_empty());
        assert!(ui.h.query_by_label("Enter a whole number of 1 or more.").is_some(), "{bad:?} is explained");
    }
    fill(&mut ui, 0, "2");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().table_dialog.is_none());
    assert_eq!(dims(&ui, ui.selected()[0]), (2, 6));
}

#[test]
fn a_huge_table_reports_the_command_error() {
    let mut ui = blank();
    open_create_table(&mut ui);
    fill(&mut ui, 0, "1000");
    fill(&mut ui, 1, "1000");
    ui.key(NONE, Key::Enter);
    assert!(ui.shapes().is_empty(), "no table with a million cells");
    let msg = ui.app().error.clone().expect("the editor's error dialog");
    assert!(msg.contains("65,535 cells"), "{msg}");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().error.is_none());
}

// ---- Layout tab: enabled states ---------------------------------------------------------------------------------

#[test]
fn with_the_whole_table_selected_only_delete_table_is_enabled() {
    let (mut ui, _) = table_ui(3, 2);
    ui.click_tab("Layout");
    for n in ["Insert Above", "Insert Below", "Insert Left", "Insert Right"] {
        assert!(!on(&ui, n), "{n} needs a cell");
    }
    assert!(on(&ui, "Delete"), "the Delete menu is live");
    ui.click_ribbon("Delete");
    assert!(!menu_item_enabled(&ui, "Delete Rows"));
    assert!(!menu_item_enabled(&ui, "Delete Columns"));
    assert!(menu_item_enabled(&ui, "Delete Table"));
}

#[test]
fn with_a_cell_selected_everything_in_rows_and_columns_is_enabled() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 1);
    ui.click_tab("Layout");
    for n in ["Insert Above", "Insert Below", "Insert Left", "Insert Right", "Delete"] {
        assert!(on(&ui, n), "{n}");
    }
    ui.click_ribbon("Delete");
    for n in ["Delete Rows", "Delete Columns", "Delete Table"] {
        assert!(menu_item_enabled(&ui, n), "{n}");
    }
}

#[test]
fn the_delete_menu_lists_columns_rows_table_in_publishers_order() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 0);
    ui.click_tab("Layout");
    ui.click_ribbon("Delete");
    let ys: Vec<f32> = ["Delete Columns", "Delete Rows", "Delete Table"]
        .iter()
        .map(|l| ui.h.get_by_label(l).accesskit_node().raw_bounds().unwrap().y0 as f32)
        .collect();
    assert!(ys[0] < ys[1] && ys[1] < ys[2], "{ys:?}");
}

#[test]
fn the_rest_of_the_layout_tab_stays_coming_soon() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 0);
    ui.click_tab("Layout");
    for n in ["Select", "View Gridlines", "Merge Cells", "Split Cells...", "Diagonals"] {
        assert!(!on(&ui, n), "{n} is greyed");
    }
    let rev = ui.revision();
    for n in ["Select", "View Gridlines", "Merge Cells", "Split Cells...", "Diagonals"] {
        ui.click_ribbon(n);
        assert_eq!(ui.revision(), rev, "{n} does nothing");
    }
}

// ---- Insert Above / Below / Left / Right ------------------------------------------------------------------------------

#[test]
fn insert_above_adds_a_row_above_the_caret_cell_and_moves_the_caret_into_it() {
    let (mut ui, id) = table_ui(3, 2);
    let h0 = ui.rect_of(id).h;
    click_cell(&mut ui, id, 1, 1);
    ui.click_tab("Layout");
    ui.click_ribbon("Insert Above");
    assert_eq!(dims(&ui, id), (3, 3));
    assert_eq!(cell_count(&ui, id), 9);
    assert!(ui.rect_of(id).h > h0, "the table grows by the new row");
    assert_eq!(caret_cell(&mut ui), Some(3 + 1), "the new row 1, same column");
    assert_eq!(ui.selected(), vec![id]);
    assert_eq!(ui.tab(), crate::app::Tab::Layout);
    ui.undo();
    assert_eq!(dims(&ui, id), (2, 3), "Ctrl+Z takes the row away again");
    assert_eq!(ui.rect_of(id).h, h0);
}

#[test]
fn insert_below_left_and_right_place_the_caret_in_the_new_track() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 1);
    ui.click_tab("Layout");
    ui.click_ribbon("Insert Below");
    assert_eq!(dims(&ui, id), (3, 3));
    assert_eq!(caret_cell(&mut ui), Some(3 + 1), "row 1, column 1");
    ui.click_ribbon("Insert Left");
    assert_eq!(dims(&ui, id), (3, 4));
    assert_eq!(caret_cell(&mut ui), Some(4 + 1), "row 1, the new column 1");
    ui.click_ribbon("Insert Right");
    assert_eq!(dims(&ui, id), (3, 5));
    assert_eq!(caret_cell(&mut ui), Some(5 + 2), "row 1, the new column 2");
    assert_eq!(cell_count(&ui, id), 15);
    assert_eq!(ui.undo_steps(), 4, "the insert and the three more are one undo step each");
    for (want_rows, want_cols) in [(3, 4), (3, 3), (2, 3)] {
        ui.undo();
        assert_eq!(dims(&ui, id), (want_rows, want_cols));
    }
}

#[test]
fn inserting_keeps_the_text_of_the_other_cells_and_adds_empty_ones() {
    let (mut ui, id) = table_ui(2, 2);
    click_cell(&mut ui, id, 1, 0);
    ui.type_text("Bottom");
    ui.click_tab("Layout");
    ui.click_ribbon("Insert Above");
    let ShapeKind::Table { cells, .. } = &ui.shape(id).kind else { panic!() };
    let texts: Vec<String> = cells.iter().map(|c| c.paragraphs.iter().map(|p| p.text()).collect()).collect();
    assert_eq!(texts, ["", "", "", "", "Bottom", ""], "the text shifted down with its cell");
}

#[test]
fn the_caret_keeps_working_after_an_insert() {
    let (mut ui, id) = table_ui(2, 1);
    click_cell(&mut ui, id, 0, 0);
    ui.click_tab("Layout");
    ui.click_ribbon("Insert Below");
    ui.type_text("new");
    ui.click_at(ui.at_in(0.5, 0.5)); // away: commits the typing
    let ShapeKind::Table { cells, .. } = &ui.shape(id).kind else { panic!() };
    assert_eq!(cells[2].paragraphs[0].text(), "new", "typed into the new row's first cell");
}

// ---- rows grow to fit their text (pw-cok2) ---------------------------------------------------------------------------

fn row_heights(ui: &TestUi, id: ShapeId) -> Vec<i64> {
    match &ui.shape(id).kind {
        ShapeKind::Table { row_heights_emu, .. } => row_heights_emu.clone(),
        k => panic!("not a table: {k:?}"),
    }
}

#[test]
fn typing_text_that_wraps_grows_the_drawn_row_and_the_frame() {
    let (mut ui, id) = table_ui(2, 2);
    let before = row_heights(&ui, id);
    let frame = ui.shape(id).rect.h;
    click_cell(&mut ui, id, 0, 0);
    ui.type_text("This sentence is far too long to fit on one line of a one inch wide cell");
    let after = row_heights(&ui, id);
    assert!(after[0] > before[0] + 12_700 * 20, "row 0 grew while typing: {before:?} -> {after:?}");
    assert_eq!(after[1], before[1], "row 1 is untouched");
    assert_eq!(ui.shape(id).rect.h, after.iter().sum::<i64>(), "the frame follows the rows");
    assert!(ui.shape(id).rect.h > frame);
    // The grown row is what is on screen: a click in the middle of row 1 lands in row 1.
    let at = cell_at(&ui, id, 1, 1);
    ui.click_at(at);
    assert_eq!(caret_cell(&mut ui), Some(3));
}

#[test]
fn deleting_the_typed_text_shrinks_the_row_back_and_typing_is_one_undo_step() {
    let (mut ui, id) = table_ui(2, 2);
    let before = row_heights(&ui, id);
    click_cell(&mut ui, id, 0, 0);
    ui.type_text("This sentence is far too long to fit on one line of a one inch wide cell");
    assert!(row_heights(&ui, id)[0] > before[0]);
    ui.key(CTRL, Key::A);
    ui.key(NONE, Key::Backspace);
    assert_eq!(row_heights(&ui, id), before, "back to the set height, not below");
    ui.type_text("This sentence is far too long to fit on one line of a one inch wide cell");
    ui.click_at(ui.at_in(0.5, 0.5)); // away: commits the typing
    ui.undo();
    assert_eq!(row_heights(&ui, id), before, "one undo takes the text and the growth back");
    let ShapeKind::Table { cells, .. } = &ui.shape(id).kind else { panic!() };
    assert_eq!(cells[0].paragraphs[0].text(), "");
}

// ---- Delete Rows / Columns / Table --------------------------------------------------------------------------------------

#[test]
fn delete_rows_removes_the_caret_row_and_keeps_a_caret_cell() {
    let (mut ui, id) = table_ui(3, 3);
    let h0 = ui.rect_of(id).h;
    click_cell(&mut ui, id, 1, 2);
    delete_menu(&mut ui, "Delete Rows");
    assert_eq!(dims(&ui, id), (2, 3));
    assert_eq!(cell_count(&ui, id), 6);
    assert!(ui.rect_of(id).h < h0);
    assert_eq!(caret_cell(&mut ui), Some(3 + 2), "the row that took its place, same column");
    // The last row: the caret goes to the row before.
    click_cell(&mut ui, id, 1, 0);
    delete_menu(&mut ui, "Delete Rows");
    assert_eq!(dims(&ui, id), (1, 3));
    assert_eq!(caret_cell(&mut ui), Some(0));
    ui.undo();
    assert_eq!(dims(&ui, id), (2, 3));
    ui.undo();
    assert_eq!(dims(&ui, id), (3, 3));
}

#[test]
fn delete_columns_removes_the_caret_column_and_keeps_a_caret_cell() {
    let (mut ui, id) = table_ui(3, 2);
    let w0 = ui.rect_of(id).w;
    click_cell(&mut ui, id, 1, 2);
    delete_menu(&mut ui, "Delete Columns");
    assert_eq!(dims(&ui, id), (2, 2));
    assert_eq!(cell_count(&ui, id), 4);
    assert!(ui.rect_of(id).w < w0);
    assert_eq!(caret_cell(&mut ui), Some(2 + 1), "the last column is gone: the one before");
    ui.undo();
    assert_eq!(dims(&ui, id), (2, 3));
    assert_eq!(ui.rect_of(id).w, w0);
}

#[test]
fn delete_table_removes_the_whole_object() {
    let (mut ui, id) = table_ui(3, 2);
    delete_menu(&mut ui, "Delete Table");
    assert!(ui.shapes().is_empty());
    assert!(ui.selected().is_empty());
    ui.undo();
    assert_eq!(ui.shapes().len(), 1, "undo brings it back");
    assert_eq!(dims(&ui, id), (2, 3));
}

#[test]
fn delete_table_works_from_a_cell_too() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 0);
    delete_menu(&mut ui, "Delete Table");
    assert!(ui.shapes().is_empty());
    assert!(ui.app().edit.is_none());
}

#[test]
fn deleting_the_last_row_or_column_shows_the_editors_error_and_changes_nothing() {
    let (mut ui, id) = table_ui(2, 1);
    click_cell(&mut ui, id, 0, 1);
    let steps = ui.undo_steps();
    delete_menu(&mut ui, "Delete Rows");
    let msg = ui.app().error.clone().expect("an error is shown");
    assert!(msg.starts_with("KernPage could not do that:"), "{msg}");
    assert!(msg.contains("last row"), "{msg}");
    assert!(ui.h.query_by_label_contains("last row").is_some(), "the error dialog is on screen");
    assert_eq!(dims(&ui, id), (1, 2));
    assert_eq!(ui.undo_steps(), steps, "nothing was recorded");
    ui.key(NONE, Key::Enter);
    assert!(ui.app().error.is_none(), "OK dismisses it");
    assert_eq!(caret_cell(&mut ui), Some(1), "the caret stays where it was");

    // One column left: same for Delete Columns.
    delete_menu(&mut ui, "Delete Columns");
    assert_eq!(dims(&ui, id), (1, 1));
    ui.click_ribbon("Delete");
    click_menu(&mut ui, "Delete Columns");
    let msg = ui.app().error.clone().expect("an error is shown");
    assert!(msg.contains("last column"), "{msg}");
    assert_eq!(dims(&ui, id), (1, 1));
}

// ---- save --------------------------------------------------------------------------------------------------------------------

#[test]
fn a_table_edited_in_the_ribbon_saves_and_reopens_with_the_same_grid() {
    let (mut ui, id) = table_ui(3, 2);
    click_cell(&mut ui, id, 0, 1);
    ui.click_tab("Layout");
    ui.click_ribbon("Insert Below");
    ui.click_ribbon("Insert Right");
    delete_menu(&mut ui, "Delete Columns");
    let want = dims(&ui, id);
    assert_eq!(want, (3, 3));
    let target = ui.env.files.join("tables.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert!(ui.app().error.is_none(), "{:?}", ui.app().error);

    let mut again = TestUi::start();
    again.app().open_path(&target);
    assert!(again.app().error.is_none());
    again.settle();
    let t = again.shapes().iter().find(|s| matches!(s.kind, ShapeKind::Table { .. })).expect("the table came back");
    let ShapeKind::Table { rows, cols, cells, .. } = &t.kind else { unreachable!() };
    assert_eq!((*rows, *cols), want);
    assert_eq!(cells.len(), 9);
}

#[test]
fn a_table_inserted_from_the_dialog_saves_and_reopens() {
    let mut ui = blank();
    open_create_table(&mut ui);
    ui.key(NONE, Key::Enter);
    let target = ui.env.files.join("dialog.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    let mut again = TestUi::start();
    again.app().open_path(&target);
    again.settle();
    let t = again.shapes().iter().find(|s| matches!(s.kind, ShapeKind::Table { .. })).expect("table");
    let ShapeKind::Table { rows, cols, .. } = &t.kind else { unreachable!() };
    assert_eq!((*rows, *cols), (2, 6));
}
