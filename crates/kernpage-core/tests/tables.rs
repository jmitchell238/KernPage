//! Tables in the editor, saved as real `.pub` files: Insert Table, row and column inserts
//! and deletes, Delete Table, cell text, chained edits, resaving. The saved files are read
//! back with `pub-format`; the main ones are also written to `out/tables-editor/` so they
//! can be opened in Publisher.

use kernpage_core::table::{default_table_rect, Axis};
use kernpage_core::{
    from_pub_flowed, Command, CommandError, Document, Editor, Paragraph, Rect, Run, SaveSource,
    ShapeId, ShapeKind,
};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples").join(name))).unwrap()
}

fn gen(name: &str) -> Vec<u8> {
    sample(&format!("gen/{name}.pub"))
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    let source = SaveSource::new(bytes.to_vec(), &src, &model, &breaks);
    (source, Editor::new(model))
}

/// Plan (nothing unsupported), build, mark saved; optionally keep the file in `out/`.
fn save(src: &mut SaveSource, doc: &Document, keep_as: Option<&str>) -> Vec<u8> {
    let plan = src.plan(doc);
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (bytes, plan) = src.build(doc).unwrap();
    src.mark_saved(bytes.clone(), doc, &plan);
    assert!(
        src.plan(doc).is_empty(),
        "after saving nothing is left to save: {:?}",
        src.plan(doc)
    );
    if let Some(name) = keep_as {
        let dir = root().join("out/tables-editor");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), &bytes).unwrap();
    }
    bytes
}

#[derive(Debug, Clone, PartialEq)]
struct View {
    rows: usize,
    cols: usize,
    cells: Vec<String>,
    widths: Vec<u32>,
    heights: Vec<u32>,
    /// Left and top from the page's top-left corner, EMU.
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

fn views(bytes: &[u8]) -> Vec<View> {
    let d = pub_format::read_bytes(bytes).unwrap();
    let (cx, cy) = (
        i64::from(d.page_width_emu) / 2,
        i64::from(d.page_height_emu) / 2,
    );
    d.pages[0]
        .shapes
        .iter()
        .filter_map(|s| {
            let t = s.table.as_ref()?;
            Some(View {
                rows: t.rows as usize,
                cols: t.columns as usize,
                cells: t.cells.clone(),
                widths: t.column_widths_emu.clone(),
                heights: t.row_heights_emu.clone(),
                left: cx + i64::from(s.left_emu),
                top: cy + i64::from(s.top_emu),
                width: i64::from(s.right_emu) - i64::from(s.left_emu),
                height: i64::from(s.bottom_emu) - i64::from(s.top_emu),
            })
        })
        .collect()
}

fn view(bytes: &[u8]) -> View {
    let mut v = views(bytes);
    assert_eq!(v.len(), 1, "one table");
    v.remove(0)
}

fn table_id(doc: &Document, n: usize) -> ShapeId {
    doc.pages[0]
        .shapes
        .iter()
        .filter(|s| matches!(s.kind, ShapeKind::Table { .. }))
        .nth(n)
        .expect("table")
        .id
}

fn set_text(ed: &mut Editor, id: ShapeId, cell: usize, text: &str) {
    let ShapeKind::Table { cells, .. } = &ed.doc().shape(0, id).unwrap().kind else {
        panic!("not a table");
    };
    let base = cells[cell].paragraphs[0].base_run();
    let paragraphs = text
        .split('\n')
        .map(|t| {
            let mut p = cells[cell].paragraphs[0].clone();
            p.runs = vec![Run {
                text: t.to_owned(),
                ..base.clone()
            }];
            p
        })
        .collect();
    ed.execute(Command::SetCellText {
        page: 0,
        id,
        cell,
        paragraphs,
    })
    .unwrap();
}

fn insert_default(ed: &mut Editor, rows: u32, cols: u32) -> ShapeId {
    let p = &ed.doc().pages[0];
    let rect = default_table_rect(p.width_emu, p.height_emu, rows, cols);
    ed.execute(Command::InsertTable {
        page: 0,
        rect,
        rows,
        columns: cols,
    })
    .unwrap()[0]
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[kernpage_samples::test]
fn new_table_saves_with_publishers_defaults_and_text() {
    let bytes = sample("blank.pub");
    let (mut src, mut ed) = open(&bytes);
    let id = insert_default(&mut ed, 2, 3);
    set_text(&mut ed, id, 0, "Name");
    set_text(&mut ed, id, 5, "Last\nTwo lines");
    let rect = ed.doc().shape(0, id).unwrap().rect;
    let out = save(&mut src, ed.doc(), Some("new-table-3x2"));
    let v = view(&out);
    assert_eq!((v.rows, v.cols), (2, 3));
    assert_eq!(v.widths, vec![914_400; 3]);
    // Two lines in the last row grow it (pw-cok2); the first row stays at its set height.
    assert_eq!(v.heights[0], 344_238);
    assert!(v.heights[1] > 344_238, "{:?}", v.heights);
    assert_eq!(
        v.cells,
        strs(&["Name", "", "", "", "", "Last\rTwo lines"])
    );
    assert_eq!((v.left, v.top, v.width, v.height), (rect.x, rect.y, rect.w, rect.h));

    // The saved table is the baseline: a later text edit saves as a text edit.
    set_text(&mut ed, id, 1, "again");
    let plan = src.plan(ed.doc());
    assert_eq!(plan.tables.len(), 1, "{plan:?}");
    let out = save(&mut src, ed.doc(), None);
    assert_eq!(view(&out).cells[1], "again");
    assert_eq!(view(&out).cells[0], "Name");
}

#[kernpage_samples::test]
fn new_table_with_row_and_column_edits_before_the_first_save() {
    let bytes = sample("blank.pub");
    let (mut src, mut ed) = open(&bytes);
    let id = insert_default(&mut ed, 3, 3);
    set_text(&mut ed, id, 4, "mid");
    ed.execute(Command::InsertTableRows { page: 0, shape: id, before: 0, count: 1 })
        .unwrap();
    ed.execute(Command::InsertTableColumns { page: 0, shape: id, before: 3, count: 2 })
        .unwrap();
    ed.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 })
        .unwrap();
    let out = save(&mut src, ed.doc(), Some("new-table-edited-before-save"));
    let v = view(&out);
    assert_eq!((v.rows, v.cols), (4, 4));
    // Row 0 is the inserted one, the text moved to (2, 0) after the column delete.
    assert_eq!(v.cells[2 * 4], "mid");
    assert_eq!(v.cells.iter().filter(|c| !c.is_empty()).count(), 1);
    assert_eq!(v.widths, vec![914_400; 4]);
    assert_eq!(v.heights, vec![344_238; 4]);
    let rect = ed.doc().shape(0, id).unwrap().rect;
    assert_eq!((v.left, v.top, v.width, v.height), (rect.x, rect.y, rect.w, rect.h));
}

#[kernpage_samples::test]
fn insert_then_delete_before_saving_writes_nothing() {
    let bytes = sample("blank.pub");
    let (src, mut ed) = open(&bytes);
    let id = insert_default(&mut ed, 2, 2);
    ed.execute(Command::DeleteShapes { page: 0, ids: vec![id] }).unwrap();
    assert!(src.plan(ed.doc()).is_empty());
    // Undo brings the table back as a new table to write.
    ed.undo();
    assert_eq!(src.plan(ed.doc()).new_tables.len(), 1);
}

#[kernpage_samples::test]
fn rows_and_columns_of_publisher_tables_match_the_writer() {
    for name in [
        "table-3x3-text",
        "table-addrow-before",
        "table-addcol-after",
        "table-delrow",
        "table-2x2-grown",
        "table-after-textbox",
    ] {
        let bytes = gen(name);
        let before = view(&bytes);
        let (mut src, mut ed) = open(&bytes);
        let id = table_id(ed.doc(), 0);
        ed.execute(Command::InsertTableRows { page: 0, shape: id, before: 1, count: 1 })
            .unwrap();
        let out = save(&mut src, ed.doc(), None);
        let v = view(&out);
        // What the writer's own row insert makes (same table, same cells, same sizes).
        let object = pub_format::read_bytes(&bytes).unwrap().pages[0]
            .shapes
            .iter()
            .find(|s| s.table.is_some())
            .unwrap()
            .object_number;
        let (w, _) = pub_write::insert_table_rows(&bytes, object, 1, 1).unwrap();
        let want = view(&w);
        assert_eq!(v, want, "{name}");
        assert_eq!(v.rows, before.rows + 1, "{name}");
        assert_eq!(&v.cells[..before.cols], &before.cells[..before.cols], "{name}");

        // Then a column on the right and the first column deleted, saved again.
        let cols = before.cols as u32;
        ed.execute(Command::InsertTableColumns { page: 0, shape: id, before: cols as usize, count: 1 })
            .unwrap();
        ed.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 })
            .unwrap();
        let out2 = save(&mut src, ed.doc(), None);
        let (w2, o2) = pub_write::insert_table_columns(&w, object_of(&w), cols, 1).unwrap();
        let (w2, _) = pub_write::delete_table_columns(&w2, o2, 0, 1).unwrap();
        assert_eq!(view(&out2), view(&w2), "{name} chained");
    }
}

/// pw-u3e4: row and column edits keep the formatting of the cell text through save and
/// reopen, and new cells copy the first run of the neighbour they are sized from.
#[kernpage_samples::test]
fn row_and_column_edits_keep_cell_formatting() {
    let bytes = gen("table-3x3-text");
    let (mut src, mut ed) = open(&bytes);
    let id = table_id(ed.doc(), 0);
    // Cell 0: bold 14 pt Arial; cell 4: red text, centred.
    let styled = |ed: &Editor, cell: usize, f: &dyn Fn(&mut Paragraph)| {
        let ShapeKind::Table { cells, .. } = &ed.doc().shape(0, id).unwrap().kind else {
            panic!("not a table");
        };
        let mut p = cells[cell].paragraphs.clone();
        f(&mut p[0]);
        p
    };
    let p0 = styled(&ed, 0, &|p| {
        for r in &mut p.runs {
            r.bold = true;
            r.size_pt = 14.0;
            r.font = "Arial".into();
        }
    });
    ed.execute(Command::SetCellText { page: 0, id, cell: 0, paragraphs: p0 }).unwrap();
    let p4 = styled(&ed, 4, &|p| {
        p.align = kernpage_core::Align::Center;
        for r in &mut p.runs {
            r.color = kernpage_core::Rgb(255, 0, 0);
        }
    });
    ed.execute(Command::SetCellText { page: 0, id, cell: 4, paragraphs: p4 }).unwrap();
    let first = save(&mut src, ed.doc(), None);

    let look = |bytes: &[u8]| -> Vec<(String, bool, f32, String, Option<pub_format::Color>, pub_format::Alignment)> {
        let d = pub_format::read_bytes(bytes).unwrap();
        let t = d.pages[0].shapes.iter().find_map(|s| s.table.as_ref()).unwrap();
        t.cell_paragraphs
            .iter()
            .map(|c| {
                let r = &c[0].runs[0];
                (r.text.clone(), r.bold, r.size_pt, r.font.clone(), r.color, c[0].alignment)
            })
            .collect()
    };
    let before = look(&first);
    assert!(before[0].1 && before[0].2 == 14.0 && before[0].3 == "Arial", "setup: {:?}", before[0]);
    assert_eq!(before[4].5, pub_format::Alignment::Center);
    let cols = view(&first).cols;

    // Insert a row above the first (copies row 0), a column on the right (copies the last).
    ed.execute(Command::InsertTableRows { page: 0, shape: id, before: 0, count: 1 }).unwrap();
    ed.execute(Command::InsertTableColumns { page: 0, shape: id, before: cols, count: 1 }).unwrap();
    let out = save(&mut src, ed.doc(), None);
    let after = look(&out);
    let w = cols + 1;
    for r in 0..3 {
        for c in 0..cols {
            let (old, new) = (&before[r * cols + c], &after[(r + 1) * w + c]);
            assert_eq!(old, new, "survivor row {r} col {c}");
        }
    }
    for c in 0..cols {
        // The new top row copies row 0's first run and alignment, with empty text.
        let (old, new) = (&before[c], &after[c]);
        assert_eq!(new.0, "\r");
        assert_eq!((old.1, old.2, &old.3, old.4, old.5), (new.1, new.2, &new.3, new.4, new.5));
    }
    // The new right-hand column copies the last column (row 1 of the old table is row 2 now).
    let (old, new) = (&before[cols - 1], &after[w + cols]);
    assert_eq!((old.1, old.2, &old.3, old.4, old.5), (new.1, new.2, &new.3, new.4, new.5));
    assert_eq!(new.0, "\r");

    // Delete the first row again and the first column: the rest keeps its look.
    ed.execute(Command::DeleteTableRows { page: 0, shape: id, first: 0, count: 1 }).unwrap();
    ed.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 }).unwrap();
    let out2 = save(&mut src, ed.doc(), None);
    let last = look(&out2);
    for r in 0..3 {
        for c in 1..cols {
            assert_eq!(before[r * cols + c], last[r * cols + c - 1], "row {r} col {c}");
        }
    }
}

fn object_of(bytes: &[u8]) -> u32 {
    pub_format::read_bytes(bytes).unwrap().pages[0]
        .shapes
        .iter()
        .find(|s| s.table.is_some())
        .unwrap()
        .object_number
}

#[kernpage_samples::test]
fn delete_rows_and_columns_of_a_publisher_table() {
    let bytes = gen("table-3x3-text");
    let before = view(&bytes);
    let (mut src, mut ed) = open(&bytes);
    let id = table_id(ed.doc(), 0);
    ed.execute(Command::DeleteTableRows { page: 0, shape: id, first: 1, count: 1 }).unwrap();
    ed.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 }).unwrap();
    let out = save(&mut src, ed.doc(), Some("delete-row-and-column-3x3"));
    let v = view(&out);
    assert_eq!((v.rows, v.cols), (2, 2));
    let at = |r: usize, c: usize| before.cells[r * 3 + c].clone();
    assert_eq!(v.cells, vec![at(0, 1), at(0, 2), at(2, 1), at(2, 2)]);
    assert_eq!((v.left, v.top), (before.left, before.top));
    assert_eq!(v.width, before.width - i64::from(before.widths[0]));
    assert_eq!(v.height, before.height - i64::from(before.heights[1]));

    // Text edits after the replacement still save (the table's story moved).
    set_text(&mut ed, id, 3, "end");
    let out = save(&mut src, ed.doc(), Some("delete-row-and-column-3x3-then-text"));
    assert_eq!(view(&out).cells[3], "end");
}

#[kernpage_samples::test]
fn text_and_grid_edits_in_one_save_then_more_edits() {
    let bytes = gen("table-3x3-text");
    let (mut src, mut ed) = open(&bytes);
    let id = table_id(ed.doc(), 0);
    set_text(&mut ed, id, 0, "first");
    ed.execute(Command::InsertTableRows { page: 0, shape: id, before: 3, count: 2 }).unwrap();
    set_text(&mut ed, id, 12, "tail");
    let out = save(&mut src, ed.doc(), Some("table-chained-edits"));
    let v = view(&out);
    assert_eq!((v.rows, v.cols), (5, 3));
    assert_eq!(v.cells[0], "first");
    assert_eq!(v.cells[12], "tail");
    assert_eq!(v.heights[3], v.heights[2]);
    assert_eq!(v.heights[4], v.heights[2]);

    // Undo and redo are ordinary edits for the next save.
    ed.undo();
    ed.undo();
    ed.undo();
    let out = save(&mut src, ed.doc(), None);
    let v = view(&out);
    assert_eq!((v.rows, v.cols), (3, 3));
    assert_eq!(v.cells[0], "A1");
    ed.redo();
    ed.redo();
    let out = save(&mut src, ed.doc(), None);
    assert_eq!(view(&out).rows, 5);
}

#[kernpage_samples::test]
fn cell_formatting_survives_a_row_insert() {
    let bytes = gen("table-3x3-text");
    let (mut src, mut ed) = open(&bytes);
    let id = table_id(ed.doc(), 0);
    let ShapeKind::Table { cells, .. } = &ed.doc().shape(0, id).unwrap().kind else {
        panic!()
    };
    let mut p: Paragraph = cells[4].paragraphs[0].clone();
    for r in &mut p.runs {
        r.bold = true;
        r.size_pt = 18.0;
    }
    ed.execute(Command::SetCellText { page: 0, id, cell: 4, paragraphs: vec![p] }).unwrap();
    ed.execute(Command::InsertTableRows { page: 0, shape: id, before: 0, count: 1 }).unwrap();
    let out = save(&mut src, ed.doc(), None);
    // Reopen: the formatted cell is now cell 7 (row 2, column 1).
    let (_, back) = open(&out);
    let t = &back.doc().pages[0].shapes.iter().find(|s| matches!(s.kind, ShapeKind::Table { .. })).unwrap().kind;
    let ShapeKind::Table { cells, .. } = t else { panic!() };
    let run = cells[7].paragraphs[0].base_run();
    assert!(run.bold && run.size_pt == 18.0, "{run:?}");
    assert!(!cells[4].paragraphs[0].base_run().bold);
}

#[kernpage_samples::test]
fn delete_table_keeps_the_rest_and_shifted_indexes_work() {
    let bytes = gen("table-after-textbox");
    let (mut src, mut ed) = open(&bytes);
    let tid = table_id(ed.doc(), 0);
    let text_id = ed.doc().pages[0].shapes.iter().find(|s| s.is_text_box()).unwrap().id;
    ed.execute(Command::DeleteShapes { page: 0, ids: vec![tid] }).unwrap();
    // The text box is edited in the same save (its story index is the file's old one).
    ed.execute(Command::SetText {
        page: 0,
        id: text_id,
        paragraphs: vec![Paragraph::plain("still here")],
    })
    .unwrap();
    let out = save(&mut src, ed.doc(), Some("table-deleted-text-edited"));
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.pages[0].shapes.iter().all(|s| s.table.is_none()));
    assert!(d.pages[0].shapes.iter().any(|s| s.text.as_deref().is_some_and(|t| t.contains("still here"))));
    // And once more on top of the saved file.
    ed.execute(Command::SetText {
        page: 0,
        id: text_id,
        paragraphs: vec![Paragraph::plain("and again")],
    })
    .unwrap();
    let out = save(&mut src, ed.doc(), None);
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.pages[0].shapes.iter().any(|s| s.text.as_deref().is_some_and(|t| t.contains("and again"))));
}

#[kernpage_samples::test]
fn delete_one_of_two_tables_then_edit_the_other() {
    let bytes = gen("table-two");
    let both = views(&bytes);
    assert_eq!(both.len(), 2);
    let (mut src, mut ed) = open(&bytes);
    let (first, second) = (table_id(ed.doc(), 0), table_id(ed.doc(), 1));
    ed.execute(Command::DeleteShapes { page: 0, ids: vec![first] }).unwrap();
    set_text(&mut ed, second, 0, "kept");
    let out = save(&mut src, ed.doc(), Some("two-tables-first-deleted"));
    let v = view(&out);
    assert_eq!(v.cells[0], "kept");
    assert_eq!((v.left, v.top), (both[1].left, both[1].top));
    set_text(&mut ed, second, 1, "more");
    let out = save(&mut src, ed.doc(), None);
    assert_eq!(view(&out).cells[1], "more");
}

#[kernpage_samples::test]
fn inserting_a_table_into_a_file_with_a_table_and_editing_both() {
    let bytes = gen("table-3x3-text");
    let (mut src, mut ed) = open(&bytes);
    let old = table_id(ed.doc(), 0);
    let new = insert_default(&mut ed, 2, 2);
    set_text(&mut ed, new, 0, "fresh");
    ed.execute(Command::InsertTableColumns { page: 0, shape: old, before: 0, count: 1 }).unwrap();
    set_text(&mut ed, old, 1, "old-edit");
    let out = save(&mut src, ed.doc(), Some("old-table-edited-new-table-added"));
    let vs = views(&out);
    assert_eq!(vs.len(), 2);
    let (o, n) = (
        vs.iter().find(|v| v.cols == 4).expect("old table"),
        vs.iter().find(|v| v.cols == 2).expect("new table"),
    );
    assert_eq!(o.cells[1], "old-edit");
    assert_eq!(n.cells[0], "fresh");
    // Both tables keep saving text edits.
    set_text(&mut ed, new, 3, "n3");
    set_text(&mut ed, old, 2, "o2");
    let out = save(&mut src, ed.doc(), None);
    let vs = views(&out);
    assert_eq!(vs.iter().find(|v| v.cols == 2).unwrap().cells[3], "n3");
    assert_eq!(vs.iter().find(|v| v.cols == 4).unwrap().cells[2], "o2");
}

#[kernpage_samples::test]
fn a_new_publication_gets_a_table() {
    let doc = kernpage_core::template::blank_portrait();
    let mut ed = Editor::new(doc);
    let id = insert_default(&mut ed, 4, 6);
    set_text(&mut ed, id, 0, "x");
    let mut src = SaveSource::for_new(ed.doc()).unwrap();
    let out = save(&mut src, ed.doc(), None);
    let v = view(&out);
    assert_eq!((v.rows, v.cols, v.width), (4, 6, 6 * 914_400));
    assert_eq!(v.cells[0], "x");
}

#[kernpage_samples::test]
fn refusals_are_reported_by_the_commands() {
    let bytes = sample("blank.pub");
    let (_, mut ed) = open(&bytes);
    let id = insert_default(&mut ed, 1, 1);
    assert_eq!(
        ed.execute(Command::DeleteTableRows { page: 0, shape: id, first: 0, count: 1 }).unwrap_err(),
        CommandError::LastTrack(Axis::Rows)
    );
    assert_eq!(
        ed.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 }).unwrap_err(),
        CommandError::LastTrack(Axis::Columns)
    );
    assert_eq!(
        ed.execute(Command::InsertTable { page: 0, rect: Rect::new(0, 0, 100, 100), rows: 2, columns: 2 })
            .unwrap_err(),
        CommandError::TableTooSmall
    );
}

// ---- pw-cok2: rows grow to fit their text ------------------------------------------------------

fn row_heights(ed: &Editor, id: ShapeId) -> Vec<i64> {
    match &ed.doc().shape(0, id).unwrap().kind {
        ShapeKind::Table { row_heights_emu, .. } => row_heights_emu.clone(),
        _ => panic!("not a table"),
    }
}

fn frame_height(ed: &Editor, id: ShapeId) -> i64 {
    ed.doc().shape(0, id).unwrap().rect.h
}

const LONG: &str = "This sentence is far too long to fit on one line of a one inch wide cell";

fn set_size(ed: &mut Editor, id: ShapeId, cell: usize, size: f32) {
    let ShapeKind::Table { cells, .. } = &ed.doc().shape(0, id).unwrap().kind else {
        panic!("not a table");
    };
    let len = cells[cell].paragraphs.iter().map(|p| p.text().chars().count()).sum::<usize>();
    ed.execute(Command::FormatRuns {
        page: 0,
        id,
        cell: Some(cell),
        range: 0..len,
        change: kernpage_core::CharChange::Size(size),
    })
    .unwrap();
}

#[kernpage_samples::test]
fn wrapping_text_grows_the_row_and_the_frame() {
    let (_, mut ed) = open(&sample("blank.pub"));
    let id = insert_default(&mut ed, 2, 2);
    let before = row_heights(&ed, id);
    let frame = frame_height(&ed, id);
    set_text(&mut ed, id, 0, LONG);
    let after = row_heights(&ed, id);
    assert!(after[0] > before[0] + 12_700 * 20, "row 0 grew: {before:?} -> {after:?}");
    assert_eq!(after[1], before[1], "the other row is untouched");
    assert_eq!(frame_height(&ed, id), after.iter().sum::<i64>(), "frame follows the rows");
    assert!(frame_height(&ed, id) > frame);
}

#[kernpage_samples::test]
fn deleting_text_shrinks_the_row_back_but_not_below_its_set_height() {
    let (_, mut ed) = open(&sample("blank.pub"));
    let id = insert_default(&mut ed, 2, 2);
    let before = row_heights(&ed, id);
    let frame = frame_height(&ed, id);
    set_text(&mut ed, id, 0, LONG);
    set_text(&mut ed, id, 0, "short");
    assert_eq!(row_heights(&ed, id), before);
    assert_eq!(frame_height(&ed, id), frame);
    set_text(&mut ed, id, 0, "");
    assert_eq!(row_heights(&ed, id), before);
}

#[kernpage_samples::test]
fn a_larger_font_grows_the_row() {
    let (_, mut ed) = open(&sample("blank.pub"));
    let id = insert_default(&mut ed, 1, 2);
    let before = row_heights(&ed, id);
    set_text(&mut ed, id, 1, "Big");
    assert_eq!(row_heights(&ed, id), before, "one 10 pt line fits");
    set_size(&mut ed, id, 1, 40.0);
    let grown = row_heights(&ed, id);
    assert!(grown[0] > before[0] + 12_700 * 20, "{before:?} -> {grown:?}");
    set_size(&mut ed, id, 1, 10.0);
    assert_eq!(row_heights(&ed, id), before, "and shrinks back");
}

#[kernpage_samples::test]
fn growing_with_its_edit_is_one_undo_step() {
    let (_, mut ed) = open(&sample("blank.pub"));
    let id = insert_default(&mut ed, 2, 2);
    let before = row_heights(&ed, id);
    set_text(&mut ed, id, 0, LONG);
    assert!(ed.undo());
    assert_eq!(row_heights(&ed, id), before);
    assert_eq!(frame_height(&ed, id), before.iter().sum::<i64>());
    assert!(ed.redo());
    assert!(row_heights(&ed, id)[0] > before[0]);
}

#[kernpage_samples::test]
fn the_grown_height_is_saved_and_survives_reopening() {
    let (mut src, mut ed) = open(&sample("blank.pub"));
    let id = insert_default(&mut ed, 2, 2);
    set_text(&mut ed, id, 0, LONG);
    let grown = row_heights(&ed, id);
    let out = save(&mut src, ed.doc(), None);
    let v = view(&out);
    assert_eq!(v.heights.iter().map(|&h| i64::from(h)).collect::<Vec<_>>(), grown);
    assert_eq!(v.height, grown.iter().sum::<i64>());

    let (_, ed2) = open(&out);
    let id2 = table_id(ed2.doc(), 0);
    assert_eq!(row_heights(&ed2, id2), grown);
}
