//! Insert and delete table rows and columns (spec 4E.1.7a): read back with `pub-format`.

use pub_write::{
    add_table, delete_table, delete_table_columns, delete_table_rows, insert_table_columns,
    insert_table_rows, set_table_text, NewTable, WriteError,
};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

#[derive(Debug, Clone, PartialEq)]
struct View {
    id: u32,
    rows: usize,
    cols: usize,
    cells: Vec<String>,
    widths: Vec<u32>,
    heights: Vec<u32>,
    /// left, top, right, bottom in EMU from the page centre
    frame: [i32; 4],
}

/// The `n`-th table on page 0.
fn view(bytes: &[u8], n: usize) -> View {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let s = doc.pages[0]
        .shapes
        .iter()
        .filter(|s| s.table.is_some())
        .nth(n)
        .expect("table")
        .clone();
    let t = s.table.clone().unwrap();
    View {
        id: s.object_number,
        rows: t.rows as usize,
        cols: t.columns as usize,
        cells: t.cells,
        widths: t.column_widths_emu,
        heights: t.row_heights_emu,
        frame: [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu],
    }
}

fn texts(rows: usize, cols: usize) -> Vec<String> {
    (0..rows * cols)
        .map(|i| format!("{}{}", (b'A' + (i % cols) as u8) as char, i / cols + 1))
        .collect()
}

/// A 3x3 table (A1..C3) of ours at 72,72, 360x120 pt.
fn ours() -> (Vec<u8>, u32) {
    let c = texts(3, 3);
    add_table(
        &gen("table-empty"),
        0,
        &NewTable::in_points(72.0, 72.0, 360.0, 120.0, 3, 3, &c),
    )
    .unwrap()
}

fn expect_cells(v: &View, want: &[&str], cols: usize) {
    assert_eq!(v.cols, cols);
    assert_eq!(v.rows, want.len() / cols);
    let want: Vec<String> = want.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(v.cells, want);
}

#[kernpage_samples::test]
fn insert_row_first_middle_end() {
    let (f, id) = ours();
    let v0 = view(&f, 0);
    let h = v0.heights[0];
    for (at, expect) in [
        (0, ["", "", "", "A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"]),
        (1, ["A1", "B1", "C1", "", "", "", "A2", "B2", "C2", "A3", "B3", "C3"]),
        (3, ["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3", "", "", ""]),
    ] {
        let (g, nid) = insert_table_rows(&f, id, at, 1).unwrap();
        let v = view(&g, 0);
        assert_eq!(v.id, nid);
        expect_cells(&v, &expect, 3);
        assert_eq!(v.heights, vec![h; 4]);
        assert_eq!(v.widths, v0.widths);
        // grew downwards by one row; left, top and right are unchanged
        assert_eq!(v.frame[..3], v0.frame[..3]);
        assert_eq!(v.frame[3], v0.frame[3] + h as i32);
    }
    let (g, _) = insert_table_rows(&f, id, 1, 2).unwrap();
    assert_eq!(view(&g, 0).rows, 5);
}

#[kernpage_samples::test]
fn insert_column_first_middle_end() {
    let (f, id) = ours();
    let v0 = view(&f, 0);
    let w = v0.widths[0];
    for (at, expect) in [
        (0, ["", "A1", "B1", "C1", "", "A2", "B2", "C2", "", "A3", "B3", "C3"]),
        (1, ["A1", "", "B1", "C1", "A2", "", "B2", "C2", "A3", "", "B3", "C3"]),
        (3, ["A1", "B1", "C1", "", "A2", "B2", "C2", "", "A3", "B3", "C3", ""]),
    ] {
        let (g, _) = insert_table_columns(&f, id, at, 1).unwrap();
        let v = view(&g, 0);
        expect_cells(&v, &expect, 4);
        assert_eq!(v.widths, vec![w; 4]);
        assert_eq!(v.heights, v0.heights);
        assert_eq!(
            [v.frame[0], v.frame[1], v.frame[3]],
            [v0.frame[0], v0.frame[1], v0.frame[3]]
        );
        assert_eq!(v.frame[2], v0.frame[2] + w as i32);
    }
}

#[kernpage_samples::test]
fn new_track_copies_the_one_it_goes_before() {
    // table-2x2-resized: column 1 = 50 pt, row 1 = 70 pt.
    let f = gen("table-2x2-resized");
    let v0 = view(&f, 0);
    assert_ne!(v0.widths[0], v0.widths[1]);
    assert_ne!(v0.heights[0], v0.heights[1]);
    let (g, _) = insert_table_columns(&f, v0.id, 0, 1).unwrap();
    assert_eq!(view(&g, 0).widths, [v0.widths[0], v0.widths[0], v0.widths[1]]);
    let (g, _) = insert_table_columns(&f, v0.id, 2, 1).unwrap(); // appending copies the last
    assert_eq!(view(&g, 0).widths, [v0.widths[0], v0.widths[1], v0.widths[1]]);
    let (g, _) = insert_table_rows(&f, v0.id, 0, 1).unwrap();
    assert_eq!(view(&g, 0).heights, [v0.heights[0], v0.heights[0], v0.heights[1]]);
    let (g, _) = insert_table_rows(&f, v0.id, 2, 1).unwrap();
    assert_eq!(view(&g, 0).heights, [v0.heights[0], v0.heights[1], v0.heights[1]]);
}

#[kernpage_samples::test]
fn delete_row_and_column() {
    let (f, id) = ours();
    let v0 = view(&f, 0);
    for (at, expect) in [
        (0, ["A2", "B2", "C2", "A3", "B3", "C3"]),
        (1, ["A1", "B1", "C1", "A3", "B3", "C3"]),
        (2, ["A1", "B1", "C1", "A2", "B2", "C2"]),
    ] {
        let (g, _) = delete_table_rows(&f, id, at, 1).unwrap();
        let v = view(&g, 0);
        expect_cells(&v, &expect, 3);
        assert_eq!(v.heights.len(), 2);
        assert_eq!(v.frame[..3], v0.frame[..3]);
        assert_eq!(v.frame[3], v0.frame[3] - v0.heights[0] as i32);
    }
    for (at, expect) in [
        (0, ["B1", "C1", "B2", "C2", "B3", "C3"]),
        (1, ["A1", "C1", "A2", "C2", "A3", "C3"]),
        (2, ["A1", "B1", "A2", "B2", "A3", "B3"]),
    ] {
        let (g, _) = delete_table_columns(&f, id, at, 1).unwrap();
        let v = view(&g, 0);
        expect_cells(&v, &expect, 2);
        assert_eq!(v.frame[3], v0.frame[3]);
        assert_eq!(v.frame[2], v0.frame[2] - v0.widths[0] as i32);
    }
    let (g, _) = delete_table_rows(&f, id, 0, 2).unwrap();
    expect_cells(&view(&g, 0), &["A3", "B3", "C3"], 3);
}

#[kernpage_samples::test]
fn refusals() {
    let (f, id) = ours();
    assert!(delete_table_rows(&f, id, 0, 3).is_err());
    assert!(delete_table_columns(&f, id, 0, 3).is_err());
    assert!(insert_table_rows(&f, id, 4, 1).is_err());
    assert!(insert_table_columns(&f, id, 4, 1).is_err());
    assert!(insert_table_rows(&f, id, 0, 0).is_err());
    assert!(delete_table_rows(&f, id, 3, 1).is_err());
    assert!(delete_table_columns(&f, id, 2, 2).is_err());
    assert!(matches!(insert_table_rows(&f, 1, 0, 1), Err(WriteError::Table(_))));
    // a 1x1 table can lose neither its row nor its column
    let (one, oid) = add_table(
        &gen("table-empty"),
        0,
        &NewTable::in_points(72.0, 72.0, 100.0, 40.0, 1, 1, &[]),
    )
    .unwrap();
    assert!(delete_table_rows(&one, oid, 0, 1).is_err());
    assert!(delete_table_columns(&one, oid, 0, 1).is_err());
    assert_eq!(view(&one, 0).rows, 1);
}

#[kernpage_samples::test]
fn chained_edits_then_text() {
    let (f, id) = ours();
    let (f, id) = insert_table_rows(&f, id, 3, 2).unwrap();
    let (f, id) = delete_table_columns(&f, id, 1, 1).unwrap();
    let (f, id) = insert_table_columns(&f, id, 0, 1).unwrap();
    let (f, _) = delete_table_rows(&f, id, 0, 1).unwrap();
    let v = view(&f, 0);
    expect_cells(&v, &["", "A2", "C2", "", "A3", "C3", "", "", "", "", "", ""], 3);
    // the table is the only story, so story index 0
    let mut cells = v.cells.clone();
    cells[11] = "Z".into();
    let g = set_table_text(&f, 0, &cells).unwrap();
    assert_eq!(view(&g, 0).cells[11], "Z");
    let (h, nid) = insert_table_rows(&g, view(&g, 0).id, 1, 1).unwrap();
    assert_eq!(view(&h, 0).cells[14], "Z");
    let d = delete_table(&h, nid).unwrap();
    assert!(pub_format::read_bytes(&d).unwrap().pages[0]
        .shapes
        .iter()
        .all(|s| s.table.is_none()));
}

#[kernpage_samples::test]
fn two_tables_only_the_second_is_edited() {
    let f = gen("table-two");
    let a = view(&f, 0);
    let b = view(&f, 1);
    let (g, nid) = insert_table_rows(&f, b.id, 1, 1).unwrap();
    let va = view(&g, 0);
    assert_eq!((va.cells, va.frame, va.heights), (a.cells.clone(), a.frame, a.heights.clone()));
    let vb = view(&g, 1);
    assert_eq!(vb.id, nid);
    assert_eq!(vb.rows, 3);
    assert_eq!(vb.cells[0], b.cells[0]);
    let (g, _) = delete_table_columns(&g, nid, 0, 1).unwrap();
    assert_eq!(view(&g, 0).cells, a.cells);
    assert_eq!(view(&g, 1).cols, 1);
}

#[kernpage_samples::test]
fn edits_on_publisher_made_tables() {
    for (name, rows, cols) in [
        ("table-3x3-text", 3, 3),
        ("table-addrow-before", 4, 3),
        ("table-addrow-after", 4, 3),
        ("table-addcol-before", 3, 4),
        ("table-addcol-after", 3, 4),
        ("table-delrow", 2, 3),
        ("table-delcol", 3, 2),
        ("table-2x2-grown", 2, 2),
        ("table-after-textbox", 2, 2),
    ] {
        let Some(f) = kernpage_samples::read_opt(&format!("gen/{name}.pub")) else {
            continue;
        };
        let v0 = view(&f, 0);
        assert_eq!((v0.rows, v0.cols), (rows, cols), "{name}");
        let ch = |what: &str, r: Result<(Vec<u8>, u32), WriteError>| {
            r.unwrap_or_else(|e| panic!("{name} {what}: {e}"))
        };
        let (g, _) = ch("insert row", insert_table_rows(&f, v0.id, 1, 1));
        let v = view(&g, 0);
        assert_eq!(v.rows, rows + 1, "{name}");
        // existing text stays in its cells
        assert_eq!(v.cells[..cols], v0.cells[..cols], "{name}");
        assert_eq!(v.cells[2 * cols..], v0.cells[cols..], "{name}");
        assert!(v.cells[cols..2 * cols].iter().all(String::is_empty), "{name}");
        let (g2, _) = ch("insert col", insert_table_columns(&g, v.id, cols as u32, 1));
        assert_eq!(view(&g2, 0).cols, cols + 1);
        let (g3, _) = ch("delete col", delete_table_columns(&g2, view(&g2, 0).id, 0, 1));
        assert_eq!(view(&g3, 0).rows, rows + 1, "{name}");
        let (g4, _) = ch("delete row", delete_table_rows(&g3, view(&g3, 0).id, 1, 1));
        let w = view(&g4, 0);
        assert_eq!((w.rows, w.cols), (rows, cols), "{name}");
        assert_eq!(w.frame[..2], v0.frame[..2], "{name}");
    }
}

/// What Publisher's COM should report for every table on page 0 (points, as `tblread.ps1`).
fn expected_json(bytes: &[u8]) -> String {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let (cx, cy) = (f64::from(doc.page_width_emu) / 2.0, f64::from(doc.page_height_emu) / 2.0);
    let pt = |e: f64| e / 12700.0;
    let list = |v: &[u32]| v.iter().map(|&e| format!("{:.3}", pt(f64::from(e)))).collect::<Vec<_>>().join(",");
    let mut out = Vec::new();
    for s in doc.pages[0].shapes.iter().filter(|s| s.table.is_some()) {
        let t = s.table.as_ref().unwrap();
        let cells: Vec<String> = t
            .cells
            .iter()
            .map(|c| format!("\"{}\"", c.replace('\\', "\\\\").replace('"', "\\\"").replace('\r', "|")))
            .collect();
        out.push(format!(
            "{{\"rows\":{},\"cols\":{},\"l\":{:.3},\"t\":{:.3},\"w\":{:.3},\"h\":{:.3},\"cw\":[{}],\"rh\":[{}],\"cells\":[{}]}}",
            t.rows,
            t.columns,
            pt(f64::from(s.left_emu) + cx),
            pt(f64::from(s.top_emu) + cy),
            pt(f64::from(t.column_widths_emu.iter().sum::<u32>())),
            pt(f64::from(t.row_heights_emu.iter().sum::<u32>())),
            list(&t.column_widths_emu),
            list(&t.row_heights_emu),
            cells.join(",")
        ));
    }
    format!("[{}]", out.join(","))
}

/// Writes the files for the Publisher 2021 check (`TABLE_RC_DIR=out/pub2021/tables-rc`).
#[kernpage_samples::test]
fn write_cases_when_asked() {
    let Ok(dir) = std::env::var("TABLE_RC_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let save = |name: &str, b: &[u8]| {
        std::fs::write(dir.join(format!("{name}.pub")), b).unwrap();
        std::fs::write(dir.join(format!("{name}.exp.json")), expected_json(b)).unwrap();
    };
    let (f, id) = ours();
    for (n, at) in [("first", 0), ("middle", 1), ("end", 3)] {
        save(&format!("ours-addrow-{n}"), &insert_table_rows(&f, id, at, 1).unwrap().0);
        save(&format!("ours-addcol-{n}"), &insert_table_columns(&f, id, at, 1).unwrap().0);
    }
    for (n, at) in [("first", 0), ("middle", 1), ("last", 2)] {
        save(&format!("ours-delrow-{n}"), &delete_table_rows(&f, id, at, 1).unwrap().0);
        save(&format!("ours-delcol-{n}"), &delete_table_columns(&f, id, at, 1).unwrap().0);
    }
    let (g, i) = insert_table_rows(&f, id, 3, 2).unwrap();
    let (g, i) = delete_table_columns(&g, i, 1, 1).unwrap();
    let mut cells = view(&g, 0).cells;
    cells[9] = "edited".into();
    let g = set_table_text(&g, 0, &cells).unwrap();
    save("ours-chain", &g);
    let (g, _) = insert_table_columns(&g, i, 1, 1).unwrap();
    save("ours-chain2", &g);
    for name in [
        "table-addrow-before",
        "table-addrow-after",
        "table-addcol-before",
        "table-addcol-after",
        "table-delrow",
        "table-delcol",
        "table-2x2-resized",
        "table-2x2-grown",
    ] {
        let Some(f) = kernpage_samples::read_opt(&format!("gen/{name}.pub")) else {
            continue;
        };
        let v = view(&f, 0);
        let n = name.trim_start_matches("table-");
        save(&format!("pub-{n}-addrow-mid"), &insert_table_rows(&f, v.id, 1, 1).unwrap().0);
        save(
            &format!("pub-{n}-addcol-end"),
            &insert_table_columns(&f, v.id, v.cols as u32, 1).unwrap().0,
        );
        save(&format!("pub-{n}-delrow-first"), &delete_table_rows(&f, v.id, 0, 1).unwrap().0);
        save(
            &format!("pub-{n}-delcol-last"),
            &delete_table_columns(&f, v.id, v.cols as u32 - 1, 1).unwrap().0,
        );
    }
    let f = gen("table-two");
    save("pub-two-second-addrow", &insert_table_rows(&f, view(&f, 1).id, 1, 1).unwrap().0);
    let f = gen("table-after-textbox");
    save("pub-textbox-addcol-first", &insert_table_columns(&f, view(&f, 0).id, 0, 1).unwrap().0);
    let c = texts(2, 2);
    let (f, _) = add_table(
        &gen("table-empty"),
        0,
        &NewTable::in_points(72.0, 72.0, 200.0, 60.0, 2, 2, &c),
    )
    .unwrap();
    let (f, _) = add_table(&f, 0, &NewTable::in_points(72.0, 300.0, 200.0, 60.0, 2, 2, &c)).unwrap();
    save("ours-two-second-addrow", &insert_table_rows(&f, view(&f, 1).id, 1, 1).unwrap().0);
}
