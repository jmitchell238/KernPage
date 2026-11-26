//! Table writer (spec 4E.1): add / delete a table, read it back with `pub-format`, and
//! compare with the files Publisher 2021 saved (`samples/gen/table-*.pub`, spec 4E.1.9).

use pub_format::{contents::Contents, items, read_bytes, Streams};
use pub_write::{
    add_page, add_table, add_text_box, delete_table, new_publication, set_table_text,
    NewPublication, NewTable, WriteError,
};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap()
}

fn blank() -> Vec<u8> {
    gen("table-empty")
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

fn table(rows: u32, cols: u32, l: f64, t: f64, w: f64, h: f64, cells: &[String]) -> NewTable<'_> {
    NewTable::in_points(l, t, w, h, rows, cols, cells)
}

// ------------------------------------------------------------------ stream helpers

/// One Quill chunk (`tag`, `instance`) by walking the directory blocks.
fn chunk(q: &[u8], tag: &[u8; 4], instance: u16) -> Option<Vec<u8>> {
    let u16_at = |o: usize| usize::from(u16::from_le_bytes([q[o], q[o + 1]]));
    let u32_at = |o: usize| u32::from_le_bytes(q[o..o + 4].try_into().unwrap()) as usize;
    let mut block = 0x18;
    loop {
        let n = u16_at(block + 2);
        for i in 0..n {
            let e = block + 8 + 24 * i;
            if &q[e + 2..e + 6] == tag && u16_at(e + 6) == usize::from(instance) {
                let (off, len) = (u32_at(e + 16), u32_at(e + 20));
                return Some(q[off..off + len].to_vec());
            }
        }
        let next = u32_at(block + 4);
        if next == 0xFFFF_FFFF {
            return None;
        }
        block = next;
    }
}

/// Every chunk's (tag, instance) in directory order.
fn chunk_list(q: &[u8]) -> Vec<(String, u16)> {
    let u16_at = |o: usize| usize::from(u16::from_le_bytes([q[o], q[o + 1]]));
    let u32_at = |o: usize| u32::from_le_bytes(q[o..o + 4].try_into().unwrap()) as usize;
    let mut out = Vec::new();
    let mut block = 0x18;
    loop {
        for i in 0..u16_at(block + 2) {
            let e = block + 8 + 24 * i;
            out.push((
                String::from_utf8_lossy(&q[e + 2..e + 6]).into_owned(),
                u16_at(e + 6) as u16,
            ));
        }
        let next = u32_at(block + 4);
        if next == 0xFFFF_FFFF {
            return out;
        }
        block = next;
    }
}

/// (kind, parent, block bytes) of every object, with the volatile items zeroed: the save
/// counter (kind 68 item 60) and a table's session value (kind 16 item 4, spec 4E.1.1).
fn contents_view(bytes: &[u8]) -> Vec<(u16, Option<u32>, Vec<u8>)> {
    let st = Streams::from_cfb(bytes).unwrap();
    let c = Contents::parse(&st.contents).unwrap();
    c.objects
        .iter()
        .map(|o| {
            let len = u32::from_le_bytes(st.contents[o.offset..o.offset + 4].try_into().unwrap())
                as usize;
            let mut b = st.contents[o.offset..o.offset + len].to_vec();
            let volatile = match o.kind {
                68 => Some(60),
                16 => Some(4),
                _ => None,
            };
            if let Some(id) = volatile {
                if let Some(s) = items::locate_int(&b, 0, id).unwrap() {
                    b[s.offset..s.offset + s.len].fill(0);
                }
            }
            if o.kind == 75 {
                blank_printer_names(&mut b);
            }
            (o.kind, o.parent, b)
        })
        .collect()
}

/// Zero the printer names in a kind 75 block (item 8 of each printer record). They name the
/// printer of the machine that saved the file; the repo's samples have a neutral name of the
/// same length instead, so the rest of the block still compares byte for byte.
fn blank_printer_names(b: &mut [u8]) {
    fn collect(list: &[items::Item], out: &mut Vec<Vec<u8>>) {
        for i in list {
            match &i.value {
                items::Value::Record(r) => {
                    for f in r.iter().filter(|f| f.id == 8) {
                        if let items::Value::Raw(raw) = &f.value {
                            out.push(raw.clone());
                        }
                    }
                    collect(r, out);
                }
                items::Value::List(l) => collect(l, out),
                _ => {}
            }
        }
    }
    let mut names = Vec::new();
    collect(&items::decode_block(b, 0).unwrap(), &mut names);
    for name in names.into_iter().filter(|n| !n.is_empty()) {
        let mut at = 0;
        while let Some(p) = b[at..].windows(name.len()).position(|w| w == name.as_slice()) {
            b[at + p..at + p + name.len()].fill(0);
            at += p + name.len();
        }
    }
}

fn assert_contents_equal(ours: &[u8], theirs: &[u8]) {
    let (a, b) = (contents_view(ours), contents_view(theirs));
    assert_eq!(a.len(), b.len(), "object count");
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!((x.0, x.1), (y.0, y.1), "object #{i} kind/parent");
        assert_eq!(x.2, y.2, "object #{i} (kind {}) block", x.0);
    }
}

/// The chunks that hold no timestamp or view cache must equal Publisher's (`SYID` too: the
/// story numbers follow Publisher's numbering). `MCLD` is compared except the four layout
/// cache items of every frame (spec 4E.1.3).
fn assert_quill_equal(ours: &[u8], theirs: &[u8]) {
    let (a, b) = (
        Streams::from_cfb(ours).unwrap().quill,
        Streams::from_cfb(theirs).unwrap().quill,
    );
    let list = chunk_list(&b);
    assert_eq!(chunk_list(&a), list, "chunk directory");
    for (tag, inst) in list {
        let t: [u8; 4] = tag.as_bytes().try_into().unwrap();
        if matches!(&t, b"STSH" | b"FONT") {
            continue; // timestamps / style-sheet housekeeping Publisher touches
        }
        let (x, y) = (chunk(&a, &t, inst).unwrap(), chunk(&b, &t, inst).unwrap());
        if &t == b"MCLD" {
            assert_eq!(x.len(), y.len(), "MCLD length");
            let (x, y) = (blank_cache(x), blank_cache(y));
            assert_eq!(x, y, "MCLD (without the layout caches)");
        } else {
            assert_eq!(x, y, "chunk {tag} {inst}");
        }
    }
}

/// Zero the values Publisher fills with a view cache or stale memory in every 0x82-byte
/// frame of an `MCLD` chunk: items 0x200-0x203 (layout cache, spec 4E.1.3) and 0x212/0x213.
fn blank_cache(mut m: Vec<u8>) -> Vec<u8> {
    let mut at = 0;
    while at + 4 < m.len() {
        if m[at..at + 4] == 0x82u32.to_le_bytes() && m.get(at + 4..at + 6) == Some(&[0x00, 0x22]) {
            for id in [0x200u16, 0x201, 0x202, 0x203, 0x212, 0x213] {
                let frame = &mut m[at..at + 0x82];
                if let Some(s) = items::locate_int(frame, 0, id).unwrap() {
                    frame[s.offset..s.offset + s.len].fill(0);
                }
            }
            at += 0x82;
        } else {
            at += 1;
        }
    }
    m
}

// -------------------------------------------------------- Publisher comparison

#[kernpage_samples::test]
fn two_by_two_matches_publisher() {
    let (out, id) = add_table(&blank(), 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &[])).unwrap();
    let theirs = gen("table-2x2");
    assert_eq!(id, 296);
    assert_contents_equal(&out, &theirs);
    assert_eq!(
        Streams::from_cfb(&out).unwrap().escher,
        Streams::from_cfb(&theirs).unwrap().escher
    );
    assert_quill_equal(&out, &theirs);
}

#[kernpage_samples::test]
fn three_by_three_with_text_matches_publisher() {
    let cells = strs(&["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"]);
    let (out, _) = add_table(&blank(), 0, &table(3, 3, 72.0, 72.0, 360.0, 120.0, &cells)).unwrap();
    let theirs = gen("table-3x3-text");
    assert_contents_equal(&out, &theirs);
    assert_eq!(
        Streams::from_cfb(&out).unwrap().escher,
        Streams::from_cfb(&theirs).unwrap().escher
    );
    // Publisher's later edit of the cell text burned two more story numbers in SYID.
    let (a, b) = (
        Streams::from_cfb(&out).unwrap().quill,
        Streams::from_cfb(&theirs).unwrap().quill,
    );
    for (tag, inst) in [(b"TEXT", 0), (b"FDPP", 0), (b"FDPC", 0), (b"TCD ", 0), (b"STRS", 0)] {
        assert_eq!(chunk(&a, tag, inst), chunk(&b, tag, inst), "{tag:?}");
    }
}

#[kernpage_samples::test]
fn two_tables_match_publisher() {
    let a = strs(&["A", "", "", ""]);
    let b = strs(&["B", "", "", ""]);
    let (one, _) = add_table(&blank(), 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &a)).unwrap();
    let (two, id2) = add_table(&one, 0, &table(2, 2, 72.0, 300.0, 300.0, 100.0, &b)).unwrap();
    let theirs = gen("table-two");
    assert_eq!(id2, 299);
    assert_contents_equal(&two, &theirs);
    assert_eq!(
        Streams::from_cfb(&two).unwrap().escher,
        Streams::from_cfb(&theirs).unwrap().escher
    );
    assert_quill_equal(&two, &theirs);
}

#[kernpage_samples::test]
fn add_then_delete_matches_publishers_delete() {
    let cells = strs(&["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"]);
    let (with, id) = add_table(&blank(), 0, &table(3, 3, 72.0, 72.0, 360.0, 120.0, &cells)).unwrap();
    let gone = delete_table(&with, id).unwrap();
    let theirs = gen("table-deleted");
    assert_contents_equal(&gone, &theirs);
    assert_eq!(
        Streams::from_cfb(&gone).unwrap().escher,
        Streams::from_cfb(&theirs).unwrap().escher
    );
    let (a, b) = (
        Streams::from_cfb(&gone).unwrap().quill,
        Streams::from_cfb(&theirs).unwrap().quill,
    );
    assert_eq!(chunk_list(&a), chunk_list(&b), "chunk directory");
    // The story-number high-water mark: Publisher's own run burned one more number.
    assert_eq!(chunk(&a, b"SYID", 0).unwrap()[4..], chunk(&b, b"SYID", 0).unwrap()[4..]);
    for (tag, inst) in [(b"MCLD", 0), (b"SGP ", 0), (b"INK ", 0), (b"PL  ", 0)] {
        assert_eq!(chunk(&a, tag, inst), chunk(&b, tag, inst), "{tag:?}");
    }
}

// ------------------------------------------------------------- read back

fn only_table(bytes: &[u8], page: usize) -> pub_format::Shape {
    let doc = read_bytes(bytes).unwrap();
    doc.pages[page]
        .shapes
        .iter()
        .find(|s| s.table.is_some())
        .cloned()
        .expect("a table")
}

fn frame_pt(bytes: &[u8], s: &pub_format::Shape) -> [f64; 4] {
    let doc = read_bytes(bytes).unwrap();
    let (cx, cy) = (
        f64::from(doc.page_width_emu) / 2.0,
        f64::from(doc.page_height_emu) / 2.0,
    );
    [
        (f64::from(s.left_emu) + cx) / 12700.0,
        (f64::from(s.top_emu) + cy) / 12700.0,
        f64::from(s.width_emu.unwrap()) / 12700.0,
        f64::from(s.height_emu.unwrap()) / 12700.0,
    ]
}

#[kernpage_samples::test]
fn reads_back_rows_columns_text_and_frame() {
    let cells = strs(&["a", "b\nc", "", "ünï\u{1F600}", "five", "", "7", "", "nine"]);
    let (out, id) = add_table(&blank(), 0, &table(3, 3, 100.5, 150.0, 400.0, 200.0, &cells)).unwrap();
    let s = only_table(&out, 0);
    assert_eq!(s.object_number, id);
    let t = s.table.as_ref().unwrap();
    assert_eq!((t.rows, t.columns), (3, 3));
    let want: Vec<String> = ["a", "b\rc", "", "ünï\u{1F600}", "five", "", "7", "", "nine"]
        .iter()
        .map(|x| (*x).to_owned())
        .collect();
    assert_eq!(t.cells, want);
    assert_eq!(t.cell_props.len(), 9);
    for (i, p) in t.cell_props.iter().enumerate() {
        assert_eq!(
            (p.first_row, p.first_column, p.last_row, p.last_column),
            (i as u32 / 3, i as u32 % 3, i as u32 / 3, i as u32 % 3)
        );
        assert_eq!(p.margin_left_emu, 36_576);
    }
    let [l, tp, w, h] = frame_pt(&out, &s);
    assert!((l - 100.5).abs() < 0.01 && (tp - 150.0).abs() < 0.01, "{l} {tp}");
    assert!((w - 400.0).abs() < 0.01 && (h - 200.0).abs() < 0.01, "{w} {h}");
    assert_eq!(t.column_widths_emu.iter().sum::<u32>(), 400 * 12700);
    assert_eq!(t.row_heights_emu.iter().sum::<u32>(), 200 * 12700);
}

#[kernpage_samples::test]
fn sizes_that_do_not_divide_evenly_add_up() {
    // 100 pt over 3 columns = 423,333.3 EMU each.
    let (out, _) = add_table(&blank(), 0, &table(2, 3, 72.0, 72.0, 100.0, 80.0, &[])).unwrap();
    let t = only_table(&out, 0).table.unwrap();
    assert_eq!(t.column_widths_emu.len(), 3);
    assert_eq!(t.column_widths_emu.iter().sum::<u32>(), 1_270_000);
    assert!(t.column_widths_emu.iter().all(|&w| (423_333..=423_334).contains(&w)));
}

#[kernpage_samples::test]
fn one_cell_table_and_a_long_one() {
    let (out, _) = add_table(&blank(), 0, &table(1, 1, 10.0, 10.0, 100.0, 40.0, &strs(&["only"]))).unwrap();
    assert_eq!(only_table(&out, 0).table.unwrap().cells, strs(&["only"]));
    let many: Vec<String> = (0..60).map(|i| format!("c{i}")).collect();
    let (out, _) = add_table(&blank(), 0, &table(12, 5, 10.0, 10.0, 500.0, 480.0, &many)).unwrap();
    let t = only_table(&out, 0).table.unwrap();
    assert_eq!((t.rows, t.columns), (12, 5));
    assert_eq!(t.cells, many);
}

#[kernpage_samples::test]
fn table_after_a_text_box_gets_the_right_tcd() {
    let (with_box, _) = (add_text_box(&blank(), 0, 72.0, 400.0, 200.0, 40.0, "Hello").unwrap(), 0);
    let (out, _) = add_table(&with_box, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["", "", "", "Z"]))).unwrap();
    let doc = read_bytes(&out).unwrap();
    let shapes = &doc.pages[0].shapes;
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].text.as_deref().map(str::trim_end), Some("Hello"));
    assert_eq!(shapes[1].table.as_ref().unwrap().cells, strs(&["", "", "", "Z"]));
    // The TCD instance is the story's ordinal (spec 4E.1.3): the box is story 0.
    let q = Streams::from_cfb(&out).unwrap().quill;
    assert!(chunk(&q, b"TCD ", 1).is_some() && chunk(&q, b"TCD ", 0).is_none());
    // Same structure as Publisher's `table-after-textbox` (story numbers 2 and 5).
    let theirs = Streams::from_cfb(&gen("table-after-textbox")).unwrap().quill;
    assert_eq!(chunk(&q, b"TCD ", 1), chunk(&theirs, b"TCD ", 1));
    assert_eq!(chunk(&q, b"SYID", 0).unwrap()[..4], chunk(&theirs, b"SYID", 0).unwrap()[..4]);
    assert_eq!(chunk(&q, b"STRS", 0), chunk(&theirs, b"STRS", 0));
}

#[kernpage_samples::test]
fn text_boxes_and_tables_interleave() {
    let mut f = blank();
    f = add_text_box(&f, 0, 10.0, 10.0, 100.0, 20.0, "one").unwrap();
    f = add_table(&f, 0, &table(2, 2, 10.0, 100.0, 200.0, 60.0, &strs(&["t1"]))).unwrap().0;
    f = add_text_box(&f, 0, 10.0, 300.0, 100.0, 20.0, "two").unwrap();
    f = add_table(&f, 0, &table(2, 2, 10.0, 400.0, 200.0, 60.0, &strs(&["t2"]))).unwrap().0;
    f = add_table(&f, 0, &table(2, 2, 10.0, 500.0, 200.0, 60.0, &strs(&["t3"]))).unwrap().0;
    let doc = read_bytes(&f).unwrap();
    let s = &doc.pages[0].shapes;
    assert_eq!(s.len(), 5);
    let text = |i: usize| s[i].text.as_deref().unwrap().trim_end().to_owned();
    assert_eq!((text(0), text(2)), ("one".to_owned(), "two".to_owned()));
    for (i, name) in [(1, "t1"), (3, "t2"), (4, "t3")] {
        assert_eq!(s[i].table.as_ref().unwrap().cells[0], name, "table {i}");
    }
    // Same-size tables behind text boxes must each find their own TCD chunk.
    let story_cells = |i: usize| s[i].table.as_ref().unwrap().cells.clone();
    assert_eq!(story_cells(1)[1..], strs(&["", "", ""]));
}

#[kernpage_samples::test]
fn table_on_page_two() {
    let two = add_page(&blank(), Some(0)).unwrap();
    assert_eq!(read_bytes(&two).unwrap().pages.len(), 2);
    let (out, _) = add_table(&two, 1, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["p2", "", "", ""]))).unwrap();
    let doc = read_bytes(&out).unwrap();
    assert!(doc.pages[0].shapes.is_empty());
    assert_eq!(doc.pages[1].shapes.len(), 1);
    assert_eq!(doc.pages[1].shapes[0].table.as_ref().unwrap().cells[0], "p2");
}

#[kernpage_samples::test]
fn table_on_a_new_publication_of_ours() {
    let spec = NewPublication::new(7_772_400, 10_058_400, 1);
    let f = new_publication(&spec).unwrap();
    let (out, _) = add_table(&f, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["x", "y", "z", "w"]))).unwrap();
    assert_eq!(only_table(&out, 0).table.unwrap().cells, strs(&["x", "y", "z", "w"]));
}

#[kernpage_samples::test]
fn written_files_stay_valid_for_every_other_writer() {
    let cells = strs(&["A1", "B1", "C1", "A2"]);
    let (out, id) = add_table(&blank(), 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &cells)).unwrap();
    // The generic rewrite and the reader accept it.
    read_bytes(&pub_write::rewrite(&out).unwrap()).unwrap();
    // Cell text, a text box and a picture-free page edit all work on top of it.
    let edited = set_table_text(&out, 0, &strs(&["x", "yy\nzz", "", "end"])).unwrap();
    assert_eq!(
        only_table(&edited, 0).table.unwrap().cells,
        strs(&["x", "yy\rzz", "", "end"])
    );
    let boxed = add_text_box(&edited, 0, 72.0, 400.0, 100.0, 20.0, "after").unwrap();
    assert_eq!(read_bytes(&boxed).unwrap().pages[0].shapes.len(), 2);
    // Moving the table with the existing frame writer (page-centre coordinates) keeps it a table.
    let d = read_bytes(&boxed).unwrap();
    let idx = d.pages[0].shapes.iter().position(|s| s.object_number == id).unwrap();
    let moved = pub_write::set_shape_frame_pt(&boxed, 0, idx, 150.0 - 306.0, 200.0 - 396.0, 300.0, 100.0).unwrap();
    let t = only_table(&moved, 0);
    assert_eq!(t.table.as_ref().unwrap().cells[0], "x");
    let [l, tp, ..] = frame_pt(&moved, &t);
    assert!((l - 150.0).abs() < 0.01 && (tp - 200.0).abs() < 0.01, "{l} {tp}");
}

#[kernpage_samples::test]
fn set_table_text_finds_the_right_tcd_behind_a_text_box() {
    // Two 2x2 tables behind a text box: the old "rank among table stories" guess picks the
    // wrong TCD chunk for the second one (spec 4E.1.3).
    let mut f = add_text_box(&blank(), 0, 10.0, 10.0, 100.0, 20.0, "box").unwrap();
    f = add_table(&f, 0, &table(2, 2, 10.0, 100.0, 200.0, 60.0, &strs(&["a", "b", "c", "d"]))).unwrap().0;
    f = add_table(&f, 0, &table(2, 2, 10.0, 300.0, 200.0, 60.0, &strs(&["e", "f", "g", "h"]))).unwrap().0;
    let doc = read_bytes(&f).unwrap();
    let story_of = |i: usize| {
        let n = doc.pages[0].shapes[i].story.unwrap();
        doc.story_numbers().iter().position(|&x| x == n).unwrap()
    };
    let out = set_table_text(&f, story_of(2), &strs(&["E", "FF", "", "H\nH"])).unwrap();
    let d = read_bytes(&out).unwrap();
    assert_eq!(d.pages[0].shapes[1].table.as_ref().unwrap().cells, strs(&["a", "b", "c", "d"]));
    assert_eq!(d.pages[0].shapes[2].table.as_ref().unwrap().cells, strs(&["E", "FF", "", "H\rH"]));
}

// ----------------------------------------------------------------- delete

#[kernpage_samples::test]
fn delete_one_of_two_leaves_the_other_intact() {
    let (one, id1) = add_table(&blank(), 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["A"]))).unwrap();
    let (two, id2) = add_table(&one, 0, &table(3, 2, 72.0, 300.0, 300.0, 100.0, &strs(&["B", "b"]))).unwrap();
    let del_first = delete_table(&two, id1).unwrap();
    let doc = read_bytes(&del_first).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 1);
    let t = doc.pages[0].shapes[0].table.as_ref().unwrap();
    assert_eq!((t.rows, t.columns), (3, 2));
    assert_eq!(&t.cells[..2], &strs(&["B", "b"])[..]);
    let del_second = delete_table(&two, id2).unwrap();
    let doc = read_bytes(&del_second).unwrap();
    assert_eq!(doc.pages[0].shapes[0].table.as_ref().unwrap().cells[0], "A");
    // And the survivor can still be edited and the other deleted too.
    let rest = delete_table(&del_second, doc.pages[0].shapes[0].object_number).unwrap();
    assert!(read_bytes(&rest).unwrap().pages[0].shapes.is_empty());
}

#[kernpage_samples::test]
fn delete_keeps_text_boxes_and_other_tables() {
    let mut f = add_text_box(&blank(), 0, 10.0, 10.0, 100.0, 20.0, "keep me").unwrap();
    let (g, mid) = add_table(&f, 0, &table(2, 2, 10.0, 100.0, 200.0, 60.0, &strs(&["gone"]))).unwrap();
    f = add_text_box(&g, 0, 10.0, 300.0, 100.0, 20.0, "and me").unwrap();
    f = add_table(&f, 0, &table(2, 2, 10.0, 400.0, 200.0, 60.0, &strs(&["stay"]))).unwrap().0;
    let out = delete_table(&f, mid).unwrap();
    let doc = read_bytes(&out).unwrap();
    let s = &doc.pages[0].shapes;
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].text.as_deref().unwrap().trim_end(), "keep me");
    assert_eq!(s[1].text.as_deref().unwrap().trim_end(), "and me");
    assert_eq!(s[2].table.as_ref().unwrap().cells[0], "stay");
    // The surviving table's TCD instance moved down with its story.
    let q = Streams::from_cfb(&out).unwrap().quill;
    let list = chunk_list(&q);
    assert_eq!(list.iter().filter(|(t, _)| t == "TCD ").count(), 1);
    assert!(chunk(&q, b"TCD ", 2).is_some(), "table is story 2 now (box, box, table)");
    assert!(chunk(&q, b"TCD ", 3).is_none());
}

#[kernpage_samples::test]
fn delete_then_add_again_and_numbers_never_go_back() {
    let (one, id) = add_table(&blank(), 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["A"]))).unwrap();
    let gone = delete_table(&one, id).unwrap();
    let before = read_bytes(&one).unwrap();
    let story1 = before.pages[0].shapes[0].story.unwrap();
    let (again, _) = add_table(&gone, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["B"]))).unwrap();
    let after = read_bytes(&again).unwrap();
    assert_eq!(after.pages[0].shapes.len(), 1);
    assert_eq!(after.pages[0].shapes[0].table.as_ref().unwrap().cells[0], "B");
    assert!(after.pages[0].shapes[0].story.unwrap() > story1, "story numbers keep rising");
    // Shape ids keep rising too (spec 4E.1.4): the second table's cell spids follow the first's.
    let esc = Streams::from_cfb(&again).unwrap().escher;
    let first = Streams::from_cfb(&one).unwrap().escher;
    assert!(esc.len() == first.len());
}

#[kernpage_samples::test]
fn delete_a_table_of_publisher() {
    for name in ["table-2x2", "table-3x3-text", "table-two", "table-after-textbox"] {
        let Some(f) = kernpage_samples::read_opt(&format!("gen/{name}.pub")) else {
            continue;
        };
        let doc = read_bytes(&f).unwrap();
        let ids: Vec<u32> = doc.pages[0]
            .shapes
            .iter()
            .filter(|s| s.table.is_some())
            .map(|s| s.object_number)
            .collect();
        let mut cur = f;
        let mut left = doc.pages[0].shapes.len();
        for id in ids {
            cur = delete_table(&cur, id).unwrap_or_else(|e| panic!("{name}: {e}"));
            left -= 1;
            assert_eq!(read_bytes(&cur).unwrap().pages[0].shapes.len(), left, "{name}");
        }
    }
}

// ----------------------------------------------------------------- errors

#[kernpage_samples::test]
fn bad_input_is_an_error() {
    let b = blank();
    assert!(add_table(&b, 5, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &[])).is_err());
    assert!(add_table(&b, 0, &table(0, 2, 72.0, 72.0, 300.0, 100.0, &[])).is_err());
    assert!(add_table(&b, 0, &table(2, 0, 72.0, 72.0, 300.0, 100.0, &[])).is_err());
    assert!(add_table(&b, 0, &table(2, 2, 72.0, 72.0, 10.0, 100.0, &[])).is_err(), "columns too narrow");
    assert!(add_table(&b, 0, &table(2, 2, 72.0, 72.0, 300.0, 10.0, &[])).is_err(), "rows too short");
    assert!(add_table(&b, 0, &table(1, 1, 72.0, 72.0, 300.0, 100.0, &strs(&["a", "b"]))).is_err());
    assert!(add_table(b"not a pub file", 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &[])).is_err());
    assert!(matches!(
        delete_table(&b, 296),
        Err(WriteError::Table(_)) | Err(WriteError::Group(_))
    ));
    let (f, _) = add_table(&b, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &[])).unwrap();
    assert!(delete_table(&f, 3).is_err(), "not a table");
}

// ------------------------------------------------- files for Publisher 2021

/// With `TABLES_OUT=<dir>` set, write the cases there for opening in Publisher (`out/` is
/// git-ignored). The names are the ones `docs/spec` 4E.1.9 lists.
#[kernpage_samples::test]
fn write_cases_when_asked() {
    let Ok(dir) = std::env::var("TABLES_OUT") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let put = |n: &str, b: &[u8]| std::fs::write(dir.join(format!("{n}.pub")), b).unwrap();
    let b = blank();
    let c9 = strs(&["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"]);
    put("t-2x2", &add_table(&b, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &[])).unwrap().0);
    put("t-3x3-text", &add_table(&b, 0, &table(3, 3, 72.0, 72.0, 360.0, 120.0, &c9)).unwrap().0);
    let (one, _) = add_table(&b, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["A"]))).unwrap();
    put("t-two", &add_table(&one, 0, &table(2, 2, 72.0, 300.0, 300.0, 100.0, &strs(&["B"]))).unwrap().0);
    let boxed = add_text_box(&b, 0, 72.0, 400.0, 200.0, 40.0, "Hello").unwrap();
    put("t-after-textbox", &add_table(&boxed, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["", "", "", "Z"]))).unwrap().0);
    let two_pages = add_page(&b, Some(0)).unwrap();
    put("t-page2", &add_table(&two_pages, 1, &table(2, 3, 72.0, 72.0, 300.0, 100.0, &strs(&["p", "a", "g", "e", "2", ""]))).unwrap().0);
    let (with, id) = add_table(&b, 0, &table(3, 3, 72.0, 72.0, 360.0, 120.0, &c9)).unwrap();
    put("t-add-delete", &delete_table(&with, id).unwrap());
    let (m, _) = add_table(&boxed, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["gone"]))).unwrap();
    let (m, _) = add_table(&m, 0, &table(2, 2, 72.0, 200.0, 300.0, 100.0, &strs(&["stay", "", "", "me"]))).unwrap();
    let ids: Vec<u32> = read_bytes(&m).unwrap().pages[0]
        .shapes
        .iter()
        .filter(|s| s.table.as_ref().is_some_and(|t| t.cells[0] == "gone"))
        .map(|s| s.object_number)
        .collect();
    put("t-box-two-tables", &m);
    put("t-delete-first-of-two", &delete_table(&m, ids[0]).unwrap());
    let last: Vec<u32> = read_bytes(&m).unwrap().pages[0]
        .shapes
        .iter()
        .filter(|s| s.table.as_ref().is_some_and(|t| t.cells[0] == "stay"))
        .map(|s| s.object_number)
        .collect();
    put("t-delete-last-of-two", &delete_table(&m, last[0]).unwrap());
    let (two_a, ia) = add_table(&b, 0, &table(2, 2, 72.0, 72.0, 300.0, 100.0, &strs(&["A"]))).unwrap();
    let (two_ab, ib) = add_table(&two_a, 0, &table(2, 2, 72.0, 300.0, 300.0, 100.0, &strs(&["B"]))).unwrap();
    put("t-two-delete-first", &delete_table(&two_ab, ia).unwrap());
    put("t-two-delete-second", &delete_table(&two_ab, ib).unwrap());
    put(
        "t-many-cells",
        &add_table(
            &b,
            0,
            &table(5, 4, 36.0, 36.0, 500.0, 300.0, &(0..20).map(|i| format!("r{}c{}\nline2", i / 4, i % 4)).collect::<Vec<_>>()),
        )
        .unwrap()
        .0,
    );
    // Edited cell text in Publisher-saved tables.
    put(
        "t-edit-publisher-3x3",
        &set_table_text(&gen("table-3x3-text"), 0, &strs(&["one", "two\nlines", "", "4", "5", "6", "", "8", "nine nine nine"])).unwrap(),
    );
    put(
        "t-edit-publisher-two",
        &set_table_text(&gen("table-two"), 1, &strs(&["second", "table", "edited", "!"])).unwrap(),
    );
    put(
        "t-edit-publisher-after-textbox",
        &set_table_text(&gen("table-after-textbox"), 1, &strs(&["a", "b", "c", "d d d"])).unwrap(),
    );
}
