//! The table commands: insert table, insert and delete rows and columns, with undo/redo.

use kernpage_core::table::{default_table_rect, delete_tracks, insert_tracks, Axis};
use kernpage_core::{
    Command, CommandError, Document, Editor, Paragraph, Rect, Shape, ShapeId, ShapeKind,
    EMU_PER_POINT,
};

const PW: i64 = 7_772_400;
const PH: i64 = 10_058_400;

fn pt(v: f64) -> f64 {
    v * EMU_PER_POINT as f64
}

fn editor() -> Editor {
    Editor::new(Document::blank("t", PW, PH))
}

fn insert(e: &mut Editor, rows: u32, cols: u32) -> ShapeId {
    let rect = default_table_rect(PW, PH, rows, cols);
    e.execute(Command::InsertTable {
        page: 0,
        rect,
        rows,
        columns: cols,
    })
    .unwrap()[0]
}

type View = (Rect, u32, u32, Vec<i64>, Vec<i64>, Vec<String>);

fn table(e: &Editor, id: ShapeId) -> View {
    let s = e.doc().shape(0, id).unwrap();
    let ShapeKind::Table {
        rows,
        cols,
        col_widths_emu,
        row_heights_emu,
        cells,
        ..
    } = &s.kind
    else {
        panic!("not a table")
    };
    let texts = cells
        .iter()
        .map(|c| {
            c.paragraphs
                .iter()
                .map(|p| p.text())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();
    (
        s.rect,
        *rows,
        *cols,
        col_widths_emu.clone(),
        row_heights_emu.clone(),
        texts,
    )
}

fn set(e: &mut Editor, id: ShapeId, cell: usize, text: &str) {
    e.execute(Command::SetCellText {
        page: 0,
        id,
        cell,
        paragraphs: vec![Paragraph::plain(text)],
    })
    .unwrap();
}

#[test]
fn default_rect_matches_what_publisher_measured() {
    let r = default_table_rect(PW, PH, 2, 3);
    assert!((r.x as f64 - pt(196.875)).abs() < pt(0.01), "{}", r.x);
    assert!((r.y as f64 - pt(369.79)).abs() < pt(0.01), "{}", r.y);
    assert!((r.w as f64 - pt(216.0)).abs() < 1.0);
    assert!((r.h as f64 - pt(54.21)).abs() < pt(0.01));
    let r = default_table_rect(PW, PH, 4, 6);
    assert!((r.x as f64 - pt(88.875)).abs() < pt(0.01), "{}", r.x);
    assert!((r.y as f64 - pt(369.79)).abs() < pt(0.01));
    assert!((r.h as f64 - pt(108.42)).abs() < pt(0.01));
}

#[test]
fn insert_table_is_one_undo_step_with_publishers_cells() {
    let mut e = editor();
    let id = insert(&mut e, 2, 3);
    let (rect, rows, cols, w, h, texts) = table(&e, id);
    assert_eq!((rows, cols), (2, 3));
    assert_eq!(w, vec![914_400; 3]);
    assert_eq!(h, vec![344_238; 2]);
    assert_eq!((rect.w, rect.h), (914_400 * 3, 344_238 * 2));
    assert!(texts.iter().all(String::is_empty));
    let ShapeKind::Table { cells, .. } = &e.doc().shape(0, id).unwrap().kind else {
        panic!()
    };
    let run = cells[0].paragraphs[0].base_run();
    assert_eq!((run.font.as_str(), run.size_pt), ("Calibri", 10.0));
    assert_eq!(cells[0].margins_emu, [36_576; 4]);
    assert_eq!(e.undo_label(), Some("Insert Table"));
    assert!(e.undo());
    assert!(e.doc().pages[0].shapes.is_empty());
    assert!(e.redo());
    assert_eq!(e.doc().pages[0].shapes.len(), 1);
}

#[test]
fn insert_table_refuses_bad_sizes() {
    let mut e = editor();
    let r = Rect::new(0, 0, 914_400, 344_238);
    let run = |e: &mut Editor, rows, columns| {
        e.execute(Command::InsertTable {
            page: 0,
            rect: r,
            rows,
            columns,
        })
    };
    assert_eq!(run(&mut e, 0, 2).unwrap_err(), CommandError::BadTableSize);
    assert_eq!(run(&mut e, 2, 0).unwrap_err(), CommandError::BadTableSize);
    assert_eq!(run(&mut e, 300, 300).unwrap_err(), CommandError::BadTableSize);
    assert_eq!(run(&mut e, 5, 5).unwrap_err(), CommandError::TableTooSmall);
    assert!(!e.can_undo());
}

#[test]
fn insert_rows_copies_the_neighbour_and_grows_the_table() {
    let mut e = editor();
    let id = insert(&mut e, 2, 2);
    for (i, t) in ["a", "b", "c", "d"].iter().enumerate() {
        set(&mut e, id, i, t);
    }
    let (before_rect, ..) = table(&e, id);
    e.execute(Command::InsertTableRows { page: 0, shape: id, before: 1, count: 2 })
        .unwrap();
    let (rect, rows, cols, w, h, texts) = table(&e, id);
    assert_eq!((rows, cols), (4, 2));
    assert_eq!(h, vec![344_238; 4]);
    assert_eq!(w, vec![914_400; 2]);
    assert_eq!(texts, ["a", "b", "", "", "", "", "c", "d"]);
    assert_eq!((rect.x, rect.y, rect.w), (before_rect.x, before_rect.y, before_rect.w));
    assert_eq!(rect.h, before_rect.h + 2 * 344_238);
    assert_eq!(e.undo_label(), Some("Insert Rows"));
    e.undo();
    let (rect, rows, _, _, h, texts) = table(&e, id);
    assert_eq!((rect, rows, h.len()), (before_rect, 2, 2));
    assert_eq!(texts, ["a", "b", "c", "d"]);
    e.redo();
    assert_eq!(table(&e, id).1, 4);
    // Appending (before == rows) copies the last row.
    e.execute(Command::InsertTableRows { page: 0, shape: id, before: 4, count: 1 })
        .unwrap();
    assert_eq!(table(&e, id).1, 5);
    assert_eq!(table(&e, id).5[8..], ["", ""]);
}

#[test]
fn insert_columns_take_the_size_of_the_column_they_precede() {
    let mut e = editor();
    let id = insert(&mut e, 2, 2);
    for (i, t) in ["a", "b", "c", "d"].iter().enumerate() {
        set(&mut e, id, i, t);
    }
    e.execute(Command::InsertTableColumns { page: 0, shape: id, before: 2, count: 1 })
        .unwrap();
    e.execute(Command::InsertTableColumns { page: 0, shape: id, before: 0, count: 1 })
        .unwrap();
    let (rect, rows, cols, w, _, texts) = table(&e, id);
    assert_eq!((rows, cols), (2, 4));
    assert_eq!(w, vec![914_400; 4]);
    assert_eq!(rect.w, 4 * 914_400);
    assert_eq!(texts, ["", "a", "b", "", "", "c", "d", ""]);
    assert_eq!(e.undo_label(), Some("Insert Columns"));
    // A table with its own track sizes keeps them.
    let mut t = e.doc().shape(0, id).unwrap().clone();
    let ShapeKind::Table { col_widths_emu, .. } = &mut t.kind else {
        panic!()
    };
    *col_widths_emu = vec![300_000, 400_000, 500_000, 600_000];
    t.rect.w = 1_800_000;
    insert_tracks(&mut t, Axis::Columns, 2, 1).unwrap();
    let ShapeKind::Table { col_widths_emu, .. } = &t.kind else {
        panic!()
    };
    assert_eq!(col_widths_emu, &vec![300_000, 400_000, 500_000, 500_000, 600_000]);
    assert_eq!(t.rect.w, 2_300_000);
}

#[test]
fn delete_rows_and_columns_shrink_the_table_and_keep_other_text() {
    let mut e = editor();
    let id = insert(&mut e, 3, 3);
    for i in 0..9 {
        set(&mut e, id, i, &format!("t{i}"));
    }
    e.execute(Command::DeleteTableRows { page: 0, shape: id, first: 1, count: 1 })
        .unwrap();
    e.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 2 })
        .unwrap();
    let (rect, rows, cols, w, h, texts) = table(&e, id);
    assert_eq!((rows, cols), (2, 1));
    assert_eq!(texts, ["t2", "t8"]);
    assert_eq!((w, h), (vec![914_400], vec![344_238; 2]));
    assert_eq!((rect.w, rect.h), (914_400, 2 * 344_238));
    assert_eq!(e.undo_label(), Some("Delete Columns"));
    e.undo();
    e.undo();
    let (_, rows, cols, _, _, texts) = table(&e, id);
    assert_eq!((rows, cols, texts.len()), (3, 3, 9));
    assert_eq!(texts[4], "t4");
}

#[test]
fn deleting_the_last_track_or_bad_ranges_is_an_error_and_changes_nothing() {
    let mut e = editor();
    let id = insert(&mut e, 2, 1);
    let run = |e: &mut Editor, c: Command| e.execute(c).unwrap_err();
    assert_eq!(
        run(&mut e, Command::DeleteTableColumns { page: 0, shape: id, first: 0, count: 1 }),
        CommandError::LastTrack(Axis::Columns)
    );
    assert_eq!(
        run(&mut e, Command::DeleteTableRows { page: 0, shape: id, first: 0, count: 2 }),
        CommandError::LastTrack(Axis::Rows)
    );
    assert_eq!(
        run(&mut e, Command::DeleteTableRows { page: 0, shape: id, first: 2, count: 1 }),
        CommandError::BadTrackRange(Axis::Rows)
    );
    assert_eq!(
        run(&mut e, Command::InsertTableRows { page: 0, shape: id, before: 3, count: 1 }),
        CommandError::BadTrackRange(Axis::Rows)
    );
    assert_eq!(
        run(&mut e, Command::InsertTableColumns { page: 0, shape: id, before: 0, count: 0 }),
        CommandError::BadTrackRange(Axis::Columns)
    );
    assert_eq!(table(&e, id).1, 2);
    assert_eq!(e.undo_label(), Some("Insert Table"));
    // The messages are for the UI to show.
    assert!(CommandError::LastTrack(Axis::Rows).to_string().contains("last row"));
}

#[test]
fn merged_tables_and_other_shapes_are_refused() {
    let mut e = editor();
    let id = insert(&mut e, 2, 2);
    let mut t = e.doc().shape(0, id).unwrap().clone();
    if let ShapeKind::Table { cells, .. } = &mut t.kind {
        cells[1].merged = true;
    }
    assert_eq!(
        insert_tracks(&mut t, Axis::Rows, 0, 1),
        Err(CommandError::MergedTable(id))
    );
    assert_eq!(
        delete_tracks(&mut t, Axis::Columns, 0, 1),
        Err(CommandError::MergedTable(id))
    );
    let rect = Rect::new(0, 0, 100, 100);
    let other = e
        .execute(Command::AddShape {
            page: 0,
            shape: Shape::new(rect, ShapeKind::Rectangle),
        })
        .unwrap()[0];
    assert_eq!(
        e.execute(Command::InsertTableRows { page: 0, shape: other, before: 0, count: 1 })
            .unwrap_err(),
        CommandError::NotTable(other)
    );
}

#[test]
fn delete_table_is_the_ordinary_delete() {
    let mut e = editor();
    let id = insert(&mut e, 2, 2);
    e.execute(Command::DeleteShapes { page: 0, ids: vec![id] }).unwrap();
    assert!(e.doc().pages[0].shapes.is_empty());
    e.undo();
    assert_eq!(table(&e, id).1, 2);
}
