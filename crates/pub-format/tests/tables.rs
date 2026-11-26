//! Table reading over the Publisher 2021 table samples (spec 4E.1): cells are placed by
//! their row/column items (not story order), the `TCD ` is chosen by story ordinal and the
//! height comes from the row grid (item 105 and the grid's item 2 go stale on row growth).

use pub_format::{read, Document, Table};
use std::path::PathBuf;

fn load(name: &str) -> Document {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub"));
    read(&kernpage_samples::resolve(&p)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// All tables of the document with their shape height (EMU), in file order.
fn tables(doc: &Document) -> Vec<(Option<u32>, &Table)> {
    doc.pages
        .iter()
        .flat_map(|p| p.shapes.iter())
        .filter_map(|s| s.table.as_ref().map(|t| (s.height_emu, t)))
        .collect()
}

fn one(doc: &Document) -> (Option<u32>, &Table) {
    let mut t = tables(doc);
    assert_eq!(t.len(), 1, "exactly one table");
    t.remove(0)
}

/// Check structure: size, row-major cell text, grid and height.
fn check(name: &str, rows: u32, cols: u32, cells: &[&str], widths: &[u32], heights: &[u32]) {
    let d = load(name);
    let (h, t) = one(&d);
    check_table(name, h, t, rows, cols, cells, widths, heights);
}

#[allow(clippy::too_many_arguments)]
fn check_table(
    name: &str,
    h: Option<u32>,
    t: &Table,
    rows: u32,
    cols: u32,
    cells: &[&str],
    widths: &[u32],
    heights: &[u32],
) {
    assert_eq!((t.rows, t.columns), (rows, cols), "{name} size");
    assert_eq!(t.cells, cells, "{name} cells (row-major)");
    assert_eq!(t.column_widths_emu, widths, "{name} widths");
    assert_eq!(t.row_heights_emu, heights, "{name} heights");
    assert_eq!(h, Some(heights.iter().sum()), "{name} height = sum of rows");
    assert_eq!(t.cell_paragraphs.len(), cells.len(), "{name} paragraphs");
    // Props are paired with the same slots: first row/column are the slot's coordinates.
    assert_eq!(t.cell_props.len(), cells.len(), "{name} props");
    for (i, p) in t.cell_props.iter().enumerate() {
        assert_eq!(
            (p.first_row, p.first_column),
            (i as u32 / cols, i as u32 % cols),
            "{name} prop {i}"
        );
    }
}

const E: &str = "";
const W3: [u32; 3] = [1_524_000; 3]; // 120 pt
const H3: u32 = 508_000; // 40 pt
const TEXT3: [&str; 9] = ["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"];

#[kernpage_samples::test]
fn fresh_2x2() {
    check("table-2x2", 2, 2, &[E; 4], &[1_905_000; 2], &[635_000; 2]);
}

#[kernpage_samples::test]
fn fresh_3x3_text() {
    check("table-3x3-text", 3, 3, &TEXT3, &W3, &[H3; 3]);
}

#[kernpage_samples::test]
fn add_row_before_puts_new_row_second() {
    // Story order is A1 B1 C1 A2 B2 C2 A3 B3 C3 + three empties (appended); the new cells
    // belong to row index 1.
    check(
        "table-addrow-before",
        4,
        3,
        &["A1", "B1", "C1", E, E, E, "A2", "B2", "C2", "A3", "B3", "C3"],
        &W3,
        &[H3; 4],
    );
}

#[kernpage_samples::test]
fn add_row_after_appends_last_row() {
    check(
        "table-addrow-after",
        4,
        3,
        &["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3", E, E, E],
        &W3,
        &[H3; 4],
    );
}

#[kernpage_samples::test]
fn add_column_before_puts_new_column_second() {
    check(
        "table-addcol-before",
        3,
        4,
        &["A1", E, "B1", "C1", "A2", E, "B2", "C2", "A3", E, "B3", "C3"],
        &[1_524_000; 4],
        &[H3; 3],
    );
}

#[kernpage_samples::test]
fn add_column_after_appends_last_column() {
    check(
        "table-addcol-after",
        3,
        4,
        &["A1", "B1", "C1", E, "A2", "B2", "C2", E, "A3", "B3", "C3", E],
        &[1_524_000; 4],
        &[H3; 3],
    );
}

#[kernpage_samples::test]
fn delete_row_removes_middle_row() {
    check(
        "table-delrow",
        2,
        3,
        &["A1", "B1", "C1", "A3", "B3", "C3"],
        &W3,
        &[H3; 2],
    );
}

#[kernpage_samples::test]
fn delete_column_removes_middle_column() {
    check(
        "table-delcol",
        3,
        2,
        &["A1", "C1", "A2", "C2", "A3", "C3"],
        &[1_524_000; 2],
        &[H3; 3],
    );
}

#[kernpage_samples::test]
fn two_tables_each_get_their_own_tcd() {
    let d = load("table-two");
    let t = tables(&d);
    assert_eq!(t.len(), 2);
    let widths = [1_905_000; 2];
    let heights = [635_000; 2];
    let (first, second) = if t[0].1.cells[0] == "A" { (0, 1) } else { (1, 0) };
    check_table("two/A", t[first].0, t[first].1, 2, 2, &["A", E, E, E], &widths, &heights);
    check_table("two/B", t[second].0, t[second].1, 2, 2, &["B", E, E, E], &widths, &heights);
}

#[kernpage_samples::test]
fn table_after_text_box_uses_story_ordinal_for_tcd() {
    // Only TCD 1 exists (the text box is story ordinal 0).
    check("table-after-textbox", 2, 2, &[E, E, E, "Z"], &[1_905_000; 2], &[635_000; 2]);
}

#[kernpage_samples::test]
fn grown_row_height_comes_from_the_grid_edges() {
    // Item 105 stays 1,270,000 and the grid's item 2 stays 635,000, but the running edges
    // are 781,634 and 1,416,634 (row 1 = 61.55 pt).
    check(
        "table-2x2-grown",
        2,
        2,
        &[
            "One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty.",
            E,
            E,
            E,
        ],
        &[1_905_000; 2],
        &[781_634, 635_000],
    );
}
