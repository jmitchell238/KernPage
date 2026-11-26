//! Row and column edits keep the formatting of the cell text (spec 4E.1.7a, bead pw-u3e4).

use pub_format::{Alignment, Color, ListInfo, ListType, Paragraph, ParaIndents};
use pub_write::{
    add_table, delete_table_columns, delete_table_rows, insert_table_columns, insert_table_rows,
    set_story_format, set_story_lists, CharFormat, CharSpan, ListSpan, NewTable, ParaAlign,
    ParaSpan,
};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

/// What a run looks like without its text.
type RunFmt = (bool, bool, bool, u32, String, Option<Color>);
/// A paragraph: alignment, its runs (text, formatting), its list and its indents.
type ParaDesc = (Alignment, Vec<(String, RunFmt)>, Option<ListInfo>, ParaIndents);

fn run_fmt(r: &pub_format::TextRun) -> RunFmt {
    (r.bold, r.italic, r.underline, (r.size_pt * 100.0) as u32, r.font.clone(), r.color)
}

fn desc(p: &Paragraph) -> ParaDesc {
    (
        p.alignment,
        p.runs.iter().map(|r| (r.text.clone(), run_fmt(r))).collect(),
        p.list.clone(),
        p.indents,
    )
}

/// The table on page 0: its object number, columns and every cell's paragraphs.
fn read(bytes: &[u8]) -> (u32, usize, Vec<Vec<ParaDesc>>) {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let s = doc.pages[0].shapes.iter().find(|s| s.table.is_some()).unwrap();
    let t = s.table.as_ref().unwrap();
    (
        s.object_number,
        t.columns as usize,
        t.cell_paragraphs.iter().map(|c| c.iter().map(desc).collect()).collect(),
    )
}

fn table_story(bytes: &[u8]) -> usize {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let n = doc.pages[0].shapes.iter().find(|s| s.table.is_some()).unwrap().story.unwrap();
    doc.story_numbers().iter().position(|&x| x == n).unwrap()
}

fn fmt(bold: bool, size: f32, font: &str, color: Option<[u8; 3]>) -> CharFormat {
    CharFormat {
        bold,
        italic: false,
        underline: false,
        script: pub_format::Script::Normal,
        size_pt: size,
        font: font.into(),
        color,
        scheme_color: None,
    }
}

/// A 3x3 table A1..C3 (each cell "Xn\r", 3 units) with distinct formatting on some cells:
/// A1 bold; B1 has two runs (bold "B", plain "1"); C1 centred; B2 14 pt Arial; C3 red italic-free.
fn formatted() -> Vec<u8> {
    let c: Vec<String> = (0..9)
        .map(|i| format!("{}{}", (b'A' + (i % 3) as u8) as char, i / 3 + 1))
        .collect();
    let (f, _) = add_table(
        &gen("table-empty"),
        0,
        &NewTable::in_points(72.0, 72.0, 360.0, 120.0, 3, 3, &c),
    )
    .unwrap();
    let st = table_story(&f);
    let span = |cell: usize, from: usize, to: usize, format| CharSpan {
        start: cell * 3 + from,
        end: cell * 3 + to,
        format,
    };
    let chars = [
        span(0, 0, 3, fmt(true, 10.0, "Calibri", None)),
        span(1, 0, 1, fmt(true, 10.0, "Calibri", None)),
        span(4, 0, 3, fmt(false, 14.0, "Arial", None)),
        span(8, 0, 3, fmt(false, 10.0, "Calibri", Some([255, 0, 0]))),
    ];
    let paras = [ParaSpan { start: 6, end: 9, align: ParaAlign::Center, spacing: None }];
    let mut chars = chars.to_vec();
    chars.push(CharSpan {
        start: 21,
        end: 24,
        format: CharFormat { scheme_color: Some(2), ..fmt(false, 10.0, "Calibri", None) },
    });
    let out = set_story_format(&f, st, &chars, &paras).unwrap();
    let mut numbered = ListInfo::numbered(ListType::UpperRoman);
    numbered.start = 4;
    let lists = [
        ListSpan::new(9, 12, Some(ListInfo::bullet())),
        ListSpan::new(15, 18, Some(numbered)),
        ListSpan {
            start: 18,
            end: 21,
            list: None,
            indents: ParaIndents { left_pt: 20.0, first_line_pt: 10.0, right_pt: 0.0 },
        },
    ];
    let out = set_story_lists(&out, st, &lists).unwrap();
    // The setup itself must have taken.
    let (_, _, cells) = read(&out);
    assert_eq!(cells[3][0].2.as_ref().map(|l| l.kind), Some(ListType::Bullet));
    assert_eq!(cells[5][0].2.as_ref().map(|l| (l.kind, l.start)), Some((ListType::UpperRoman, 4)));
    assert_eq!(cells[6][0].3.left_pt, 20.0);
    assert_eq!(cells[6][0].3.first_line_pt, 10.0);
    assert_eq!(cells[7][0].1[0].1 .5, Some(Color::Scheme(2)), "B3 scheme colour");
    assert!(cells[0][0].1[0].1 .0, "A1 bold");
    assert_eq!(cells[1][0].1.len(), 2, "B1 has two runs");
    assert_eq!(cells[2][0].0, Alignment::Center);
    assert_eq!(cells[4][0].1[0].1 .3, 1400);
    assert!(matches!(cells[8][0].1[0].1 .5, Some(_)));
    out
}

/// The text-free look of a new empty cell copied from `src`: its first run, alignment, "\r".
fn blank_of(src: &[ParaDesc]) -> Vec<ParaDesc> {
    let (a, runs, ..) = &src[0];
    vec![(*a, vec![("\r".to_string(), runs[0].1.clone())], None, ParaIndents::default())]
}

fn assert_grid(got: &[Vec<ParaDesc>], want: &[Vec<ParaDesc>]) {
    assert_eq!(got.len(), want.len());
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert_eq!(g, w, "cell {i}");
    }
}

#[kernpage_samples::test]
fn insert_row_above_and_below() {
    let f = formatted();
    let (id, cols, old) = read(&f);
    assert_eq!(cols, 3);
    let row = |r: usize| old[r * 3..r * 3 + 3].to_vec();
    let blank_row = |r: usize| -> Vec<Vec<ParaDesc>> { row(r).iter().map(|c| blank_of(c)).collect() };

    // Above row 1: the new row copies row 1's formatting, rows 0, 1, 2 keep theirs.
    let (g, _) = insert_table_rows(&f, id, 1, 1).unwrap();
    let (_, _, got) = read(&g);
    let want: Vec<_> = [row(0), blank_row(1), row(1), row(2)].concat();
    assert_grid(&got, &want);

    // Below the last row: copies the last row.
    let (g, _) = insert_table_rows(&f, id, 3, 1).unwrap();
    let (_, _, got) = read(&g);
    let want: Vec<_> = [row(0), row(1), row(2), blank_row(2)].concat();
    assert_grid(&got, &want);

    // Above the first row, two at once.
    let (g, _) = insert_table_rows(&f, id, 0, 2).unwrap();
    let (_, _, got) = read(&g);
    let want: Vec<_> = [blank_row(0), blank_row(0), row(0), row(1), row(2)].concat();
    assert_grid(&got, &want);
}

#[kernpage_samples::test]
fn insert_column_left_and_right() {
    let f = formatted();
    let (id, _, old) = read(&f);
    let at = |r: usize, c: usize| old[r * 3 + c].clone();

    let (g, _) = insert_table_columns(&f, id, 0, 1).unwrap();
    let (_, cols, got) = read(&g);
    assert_eq!(cols, 4);
    let mut want = Vec::new();
    for r in 0..3 {
        want.push(blank_of(&at(r, 0)));
        want.extend((0..3).map(|c| at(r, c)));
    }
    assert_grid(&got, &want);

    let (g, _) = insert_table_columns(&f, id, 3, 1).unwrap();
    let (_, cols, got) = read(&g);
    assert_eq!(cols, 4);
    let mut want = Vec::new();
    for r in 0..3 {
        want.extend((0..3).map(|c| at(r, c)));
        want.push(blank_of(&at(r, 2)));
    }
    assert_grid(&got, &want);
}

#[kernpage_samples::test]
fn delete_row_and_column_keep_the_rest() {
    let f = formatted();
    let (id, _, old) = read(&f);

    let (g, _) = delete_table_rows(&f, id, 0, 1).unwrap();
    assert_grid(&read(&g).2, &old[3..]);

    let (g, _) = delete_table_columns(&f, id, 1, 1).unwrap();
    let (_, cols, got) = read(&g);
    assert_eq!(cols, 2);
    let want: Vec<_> = (0..3).flat_map(|r| [old[r * 3].clone(), old[r * 3 + 2].clone()]).collect();
    assert_grid(&got, &want);
}

#[kernpage_samples::test]
fn chained_edits_survive_a_second_rebuild() {
    let f = formatted();
    let (id, _, old) = read(&f);
    let (g, id) = insert_table_rows(&f, id, 0, 1).unwrap();
    let (g, id) = insert_table_columns(&g, id, 3, 1).unwrap();
    let (g, _) = delete_table_columns(&g, id, 0, 1).unwrap();
    let (_, cols, got) = read(&g);
    assert_eq!(cols, 3);
    // Row 1 of the result is old row 0 without its first column, then the copy of old C.
    assert_eq!(got[3], old[1]);
    assert_eq!(got[4], old[2]);
    assert_eq!(got[5], blank_of(&old[2]));
    // Row 0 is the new top row: copies of old row 0 (columns B, C) and of the copied column.
    assert_eq!(got[0], blank_of(&old[1]));
    assert_eq!(got[1], blank_of(&old[2]));
    assert_eq!(got[2], blank_of(&old[2]));
}
