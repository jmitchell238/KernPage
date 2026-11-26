//! A publication created from nothing (spec section 6). Uses only our own reader and
//! writer; no Publisher and no sample files.

use pub_write::{
    add_page, add_shape, add_text_box, new_publication, new_publication_with, BlankOptions, NewPublication,
    NewShapeKind, ShapeStyle,
};
use std::io::Read;

fn sizes() -> Vec<(&'static str, NewPublication)> {
    vec![
        ("letter", NewPublication::from_inches(8.5, 11.0, 1)),
        ("letter-landscape", NewPublication::from_inches(11.0, 8.5, 1)),
        ("legal", NewPublication::from_inches(8.5, 14.0, 1)),
        ("a4", NewPublication::from_points(595.3, 841.9, 1)),
        ("tabloid", NewPublication::from_inches(11.0, 17.0, 1)),
        ("custom-5.5x8.5", NewPublication::from_inches(5.5, 8.5, 1)),
        ("letter-2pg", NewPublication::from_inches(8.5, 11.0, 2)),
        ("letter-4pg", NewPublication::from_inches(8.5, 11.0, 4)),
    ]
}

fn check(spec: &NewPublication, bytes: &[u8]) {
    let doc = pub_format::read_bytes(bytes).unwrap();
    assert_eq!(doc.pages.len() as u32, spec.page_count);
    assert_eq!(doc.page_width_emu, spec.width_emu);
    assert_eq!(doc.page_height_emu, spec.height_emu);
    for p in &doc.pages {
        assert!(p.shapes.is_empty());
    }
    assert!(doc.text.is_empty());
}

#[test]
fn every_size_reads_back() {
    for (name, spec) in sizes() {
        let bytes = new_publication(&spec).unwrap_or_else(|e| panic!("{name}: {e}"));
        check(&spec, &bytes);
    }
}

#[test]
fn rejects_zero_pages_and_tiny_sizes() {
    assert!(new_publication(&NewPublication::new(7_772_400, 10_058_400, 0)).is_err());
    assert!(new_publication(&NewPublication::new(100, 100, 1)).is_err());
}

#[test]
fn nothing_machine_specific() {
    let bytes = new_publication(&NewPublication::letter(1)).unwrap();
    let streams = pub_format::Streams::from_cfb(&bytes).unwrap();
    // The path string of block 0 is empty: item 9 is just a terminator.
    let c = &streams.contents;
    let utf16: Vec<u8> = "\\Users".encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert!(!c.windows(utf16.len()).any(|w| w == utf16.as_slice()), "a user path");
    // Header: signature, block-0 offset, generation, total length.
    assert_eq!(&c[0..2], &[0xE8, 0xAC]);
    assert_eq!(u32::from_le_bytes(c[8..12].try_into().unwrap()) as usize, c.len());
}

fn stream(bytes: &[u8], path: &str) -> Vec<u8> {
    let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).unwrap();
    let mut v = Vec::new();
    cf.open_stream(path).unwrap().read_to_end(&mut v).unwrap();
    v
}

#[test]
fn stream_sizes_follow_the_spec() {
    let bytes = new_publication(&NewPublication::letter(1)).unwrap();
    let q = stream(&bytes, "/Quill/QuillSub/CONTENTS");
    assert_eq!(q.len() % 512, 0);
    assert_eq!(u32::from_le_bytes(q[0x14..0x18].try_into().unwrap()) as usize, q.len());
    let chunk = |tag: &[u8; 4], inst: u16| {
        for i in 0..8 {
            let e = &q[0x20 + 24 * i..0x20 + 24 * (i + 1)];
            if &e[2..6] == tag && u16::from_le_bytes([e[6], e[7]]) == inst {
                let off = u32::from_le_bytes(e[16..20].try_into().unwrap()) as usize;
                let len = u32::from_le_bytes(e[20..24].try_into().unwrap()) as usize;
                return q[off..off + len].to_vec();
            }
        }
        panic!("no chunk");
    };
    assert_eq!(chunk(b"STSH", 0).len(), 380);
    assert_eq!(chunk(b"STSH", 2).len(), 1460);
    assert_eq!(chunk(b"FONT", 0).len(), 936);
    assert_eq!(chunk(b"PL  ", 0).len(), 72);
    assert_eq!(stream(&bytes, "/Escher/EscherStm").len(), 972);
    assert_eq!(stream(&bytes, "/\u{1}CompObj").len(), 94);
    assert_eq!(stream(&bytes, "/Quill/QuillSub/\u{1}CompObj").len(), 86);
    assert_eq!(stream(&bytes, "/\u{5}DocumentSummaryInformation").len(), 152);
    let esc21 = new_publication_with(&NewPublication::letter(1), &BlankOptions { escher_2021: true, ..Default::default() }).unwrap();
    assert_eq!(stream(&esc21, "/Escher/EscherStm").len(), 1056);
}

// ---------------------------------------------------------------------------------------
// Editing a generated file (spec 6.11): every editor works without a Publisher-made file.

type Frame = (f64, f64, f64, f64);

/// Frame of a shape in points from the page's top-left corner: left, top, width, height.
fn frame(doc: &pub_format::Document, s: &pub_format::Shape) -> Frame {
    let e = 12_700.0;
    let (cx, cy) = (f64::from(doc.page_width_emu) / 2.0, f64::from(doc.page_height_emu) / 2.0);
    (
        (cx + f64::from(s.left_emu)) / e,
        (cy + f64::from(s.top_emu)) / e,
        f64::from(s.right_emu - s.left_emu) / e,
        f64::from(s.bottom_emu - s.top_emu) / e,
    )
}

fn near(a: Frame, b: Frame) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01 && (a.2 - b.2).abs() < 0.01 && (a.3 - b.3).abs() < 0.01
}

fn texts(doc: &pub_format::Document, page: usize) -> Vec<String> {
    doc.pages[page]
        .shapes
        .iter()
        .filter_map(|s| s.text.as_ref().map(|t| t.trim_end_matches('\r').to_string()))
        .collect()
}

fn rect(b: &[u8], page: usize, x: f64, y: f64, w: f64, h: f64) -> Vec<u8> {
    add_shape(b, page, NewShapeKind::Rectangle, x, y, w, h, ShapeStyle::default()).unwrap()
}

#[test]
fn editing_functions_work_on_a_generated_file() {
    for (name, spec) in sizes() {
        let bytes = new_publication(&spec).unwrap();
        let with_box = add_text_box(&bytes, 0, 72.0, 72.0, 200.0, 50.0, "Hello").unwrap_or_else(|e| panic!("{name}: {e}"));
        let with_shape = add_shape(&with_box, 0, NewShapeKind::Rectangle, 100.0, 200.0, 80.0, 40.0, ShapeStyle::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc = pub_format::read_bytes(&with_shape).unwrap();
        assert_eq!(doc.pages.len() as u32, spec.page_count, "{name}");
        assert_eq!(doc.pages[0].shapes.len(), 2, "{name}");
        assert_eq!(texts(&doc, 0), ["Hello"], "{name}");
        assert!(near(frame(&doc, &doc.pages[0].shapes[0]), (72.0, 72.0, 200.0, 50.0)), "{name}");
        assert!(near(frame(&doc, &doc.pages[0].shapes[1]), (100.0, 200.0, 80.0, 40.0)), "{name}");
        let more = add_page(&with_shape, None).unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc = pub_format::read_bytes(&more).unwrap();
        assert_eq!(doc.pages.len() as u32, spec.page_count + 1, "{name}");
        assert_eq!(doc.pages[1].shapes.len(), 2, "{name}: the old first page moved to second place");
        assert!(doc.pages[0].shapes.is_empty(), "{name}");
    }
}

#[test]
fn a_text_box_gets_its_text_engine_and_font_table() {
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    assert!(pub_format::read_bytes(&blank).unwrap().stories.is_empty());
    let out = add_text_box(&blank, 0, 72.0, 72.0, 200.0, 50.0, "Hello").unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    assert_eq!(doc.stories, ["Hello\r"]);
    assert_eq!(doc.story_ids, [2]);
    let q = stream(&out, "/Quill/QuillSub/CONTENTS");
    assert_eq!(q.len() % 512, 0);
    assert_eq!(u32::from_le_bytes(q[0x14..0x18].try_into().unwrap()) as usize, q.len());
    assert_eq!(u16::from_le_bytes(q[0x0C..0x0E].try_into().unwrap()), 15);
    // The text engine stays editable: a second story and a text change work.
    let two = add_text_box(&out, 0, 72.0, 200.0, 200.0, 50.0, "World").unwrap();
    assert_eq!(pub_format::read_bytes(&two).unwrap().stories, ["Hello\r", "World\r"]);
    let changed = pub_write::set_story_text(&two, 0, "Changed").unwrap();
    assert_eq!(pub_format::read_bytes(&changed).unwrap().stories, ["Changed\r", "World\r"]);
}

#[test]
fn a_file_without_text_gets_no_font_table_from_shapes_and_pages() {
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let kinds = |b: &[u8]| {
        let s = pub_format::Streams::from_cfb(b).unwrap();
        let c = pub_format::contents::Contents::parse(&s.contents).unwrap();
        c.objects.iter().map(|o| o.kind).collect::<Vec<_>>()
    };
    let shaped = add_page(&rect(&blank, 0, 10.0, 10.0, 50.0, 50.0), None).unwrap();
    assert!(!kinds(&shaped).contains(&108));
    let texted = add_text_box(&shaped, 0, 10.0, 10.0, 50.0, 50.0, "x").unwrap();
    assert_eq!(kinds(&texted).iter().filter(|&&k| k == 108).count(), 1);
    assert_eq!(*kinds(&texted).last().unwrap(), 108);
}

#[test]
fn two_text_boxes_are_two_stories() {
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let a = add_text_box(&blank, 0, 72.0, 72.0, 200.0, 50.0, "First").unwrap();
    let b = add_text_box(&a, 0, 72.0, 300.0, 250.0, 80.0, "Second").unwrap();
    let doc = pub_format::read_bytes(&b).unwrap();
    assert_eq!(doc.stories, ["First\r", "Second\r"]);
    assert_eq!(texts(&doc, 0), ["First", "Second"]);
    let s = &doc.pages[0].shapes;
    assert_ne!(s[0].chain_story, s[1].chain_story);
    assert!(near(frame(&doc, &s[1]), (72.0, 300.0, 250.0, 80.0)));
    // Overlapping boxes keep both stories.
    let c = add_text_box(&b, 0, 100.0, 80.0, 100.0, 20.0, "Third").unwrap();
    assert_eq!(texts(&pub_format::read_bytes(&c).unwrap(), 0), ["First", "Second", "Third"]);
}

#[test]
fn a_text_box_with_several_paragraphs_and_formatting() {
    use pub_write::{set_story_format, CharFormat, CharSpan};
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let out = add_text_box(&blank, 0, 72.0, 72.0, 300.0, 200.0, "One\nTwo\r\nThree").unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    assert_eq!(doc.stories, ["One\rTwo\rThree\r"]);
    assert_eq!(doc.pages[0].shapes[0].paragraphs.len(), 3);
    let fmt = CharFormat { bold: true, italic: false, underline: false, script: pub_format::Script::Normal, size_pt: 18.0, font: "Arial".into(), color: Some([255, 0, 0]), scheme_color: None };
    let styled = set_story_format(&out, 0, &[CharSpan { start: 0, end: 3, format: fmt }], &[]).unwrap();
    let doc = pub_format::read_bytes(&styled).unwrap();
    assert_eq!(doc.stories, ["One\rTwo\rThree\r"]);
    let runs = &doc.pages[0].shapes[0].runs;
    assert!(runs[0].bold && runs[0].text.starts_with("One"), "{runs:?}");
    assert!(runs.iter().any(|r| !r.bold && r.text.contains("Two")), "{runs:?}");
    assert_eq!(runs[0].font, "Arial");
    assert_eq!(runs[0].size_pt, 18.0);
}

#[test]
fn rectangle_oval_and_line_on_a_generated_file() {
    use pub_format::ShapeGeometry as G;
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let a = rect(&blank, 0, 72.0, 72.0, 100.0, 60.0);
    let b = add_shape(&a, 0, NewShapeKind::Oval, 200.0, 72.0, 90.0, 90.0, ShapeStyle { fill: Some([255, 0, 0]), ..Default::default() }).unwrap();
    let c = add_shape(&b, 0, NewShapeKind::Line { flip_h: false, flip_v: true }, 72.0, 300.0, 200.0, 40.0, ShapeStyle::default()).unwrap();
    let doc = pub_format::read_bytes(&c).unwrap();
    let s = &doc.pages[0].shapes;
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].geometry, Some(G::Rectangle));
    assert_eq!(s[1].geometry, Some(G::Ellipse));
    assert_eq!(s[2].geometry, Some(G::Line));
    assert!(s[1].fill.is_some() && s[0].fill.is_none());
    assert!(s[2].flip_v && !s[2].flip_h);
    assert!(near(frame(&doc, &s[0]), (72.0, 72.0, 100.0, 60.0)));
    assert!(near(frame(&doc, &s[1]), (200.0, 72.0, 90.0, 90.0)));
    assert!(near(frame(&doc, &s[2]), (72.0, 300.0, 200.0, 40.0)));
}

#[test]
fn pages_can_be_added_deleted_moved_and_duplicated() {
    use pub_write::{delete_page, duplicate_page, move_page};
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let p1 = add_text_box(&blank, 0, 72.0, 72.0, 200.0, 50.0, "Page one").unwrap();
    let p2 = add_page(&p1, Some(0)).unwrap();
    let p2 = add_text_box(&p2, 1, 72.0, 72.0, 200.0, 50.0, "Page two").unwrap();
    let p3 = add_page(&p2, Some(1)).unwrap();
    let p3 = rect(&p3, 2, 50.0, 50.0, 40.0, 40.0);
    let doc = pub_format::read_bytes(&p3).unwrap();
    assert_eq!(doc.pages.len(), 3);
    assert_eq!((texts(&doc, 0), texts(&doc, 1), doc.pages[2].shapes.len()), (vec!["Page one".to_string()], vec!["Page two".to_string()], 1));
    // Move the last page first.
    let moved = move_page(&p3, 2, 0).unwrap();
    let doc = pub_format::read_bytes(&moved).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 1);
    assert_eq!(texts(&doc, 1), ["Page one"]);
    assert_eq!(texts(&doc, 2), ["Page two"]);
    // Duplicate page two (a text page).
    let dup = duplicate_page(&moved, 2).unwrap();
    let doc = pub_format::read_bytes(&dup).unwrap();
    assert_eq!(doc.pages.len(), 4);
    assert_eq!(texts(&doc, 3), ["Page two"]);
    assert_eq!(doc.stories.len(), 3);
    // Delete the rectangle page and the first text page.
    let del = delete_page(&dup, 0).unwrap();
    let del = delete_page(&del, 0).unwrap();
    let doc = pub_format::read_bytes(&del).unwrap();
    assert_eq!(doc.pages.len(), 2);
    assert_eq!((texts(&doc, 0), texts(&doc, 1)), (vec!["Page two".to_string()], vec!["Page two".to_string()]));
    // (delete_page leaves the deleted pages' stories in Quill as unused text, spec 6.7.)
    // Pages of a file with shapes only.
    let r = rect(&blank, 0, 10.0, 10.0, 30.0, 30.0);
    let d = duplicate_page(&r, 0).unwrap();
    let doc = pub_format::read_bytes(&d).unwrap();
    assert_eq!((doc.pages.len(), doc.pages[0].shapes.len(), doc.pages[1].shapes.len()), (2, 1, 1));
    let d = delete_page(&d, 0).unwrap();
    assert_eq!(pub_format::read_bytes(&d).unwrap().pages.len(), 1);
    let two = new_publication(&NewPublication::letter(2)).unwrap();
    let two = delete_page(&move_page(&two, 0, 1).unwrap(), 1).unwrap();
    assert_eq!(pub_format::read_bytes(&two).unwrap().pages.len(), 1);
}

#[test]
fn shapes_can_be_grouped_and_ungrouped() {
    use pub_write::{group_shapes, ungroup};
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let a = rect(&blank, 0, 72.0, 72.0, 100.0, 60.0);
    let b = add_shape(&a, 0, NewShapeKind::Oval, 200.0, 100.0, 90.0, 90.0, ShapeStyle::default()).unwrap();
    let g = group_shapes(&b, 0, &[0, 1]).unwrap();
    let doc = pub_format::read_bytes(&g).unwrap();
    let kids = doc.pages[0].shapes.iter().filter(|s| s.group.is_some()).count();
    assert_eq!(kids, 2);
    let gi = doc.pages[0].shapes.iter().position(|s| s.kind == 48).expect("the group");
    let u = ungroup(&g, 0, gi).unwrap();
    let doc = pub_format::read_bytes(&u).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 2);
    assert!(doc.pages[0].shapes.iter().all(|s| s.group.is_none()));
    assert!(near(frame(&doc, &doc.pages[0].shapes[0]), (72.0, 72.0, 100.0, 60.0)));
    assert!(near(frame(&doc, &doc.pages[0].shapes[1]), (200.0, 100.0, 90.0, 90.0)));
    // A group with a text box inside.
    let t = add_text_box(&b, 0, 300.0, 300.0, 100.0, 40.0, "In group").unwrap();
    let g = group_shapes(&t, 0, &[0, 2]).unwrap();
    let doc = pub_format::read_bytes(&g).unwrap();
    assert_eq!(doc.pages[0].shapes.iter().filter(|s| s.group.is_some()).count(), 2);
    assert_eq!(texts(&doc, 0), ["In group"]);
}

#[test]
fn wrap_columns_and_style_work_on_generated_shapes() {
    use pub_format::WrapType;
    use pub_write::{set_shape_style, set_shape_wrap, set_text_columns, ShapeLook};
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let t = add_text_box(&blank, 0, 72.0, 72.0, 300.0, 100.0, "Columns").unwrap();
    let t = rect(&t, 0, 100.0, 300.0, 80.0, 40.0);
    let cols = set_text_columns(&t, 0, 0, 3, 91_440).unwrap();
    let doc = pub_format::read_bytes(&cols).unwrap();
    assert_eq!((doc.pages[0].shapes[0].columns, doc.pages[0].shapes[0].column_gap_emu), (3, 91_440));
    let w = set_shape_wrap(&cols, 0, 1, WrapType::Square).unwrap();
    let doc = pub_format::read_bytes(&w).unwrap();
    assert_eq!(doc.pages[0].shapes[1].wrap, WrapType::Square);
    let look = ShapeLook { fill: Some(Some([0, 128, 0])), line_on: Some(true), line_color: Some([0, 0, 255]), line_weight_emu: Some(38_100), ..Default::default() };
    let st = set_shape_style(&w, 0, 1, &look).unwrap();
    let doc = pub_format::read_bytes(&st).unwrap();
    let s = &doc.pages[0].shapes[1];
    assert!(s.fill.is_some());
    assert_eq!(s.line.map(|l| l.1), Some(38_100));
    assert_eq!(texts(&doc, 0), ["Columns"]);
}

/// Writes the edited files of stage B2 for the manual Publisher check when `PW_B2_OUT` names a
/// folder (spec 6.11).
#[test]
#[ignore]
fn write_b2_samples_for_publisher() {
    use pub_write::{duplicate_page, group_shapes, move_page};
    let Ok(dir) = std::env::var("PW_B2_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let w = |name: &str, b: &[u8]| std::fs::write(dir.join(format!("{name}.pub")), b).unwrap();
    // 1. Letter, one text box "Hello" and a rectangle.
    let l = new_publication(&NewPublication::letter(1)).unwrap();
    w("b2-1-blank", &l);
    let c1 = add_text_box(&l, 0, 72.0, 72.0, 200.0, 50.0, "Hello").unwrap();
    let c1 = rect(&c1, 0, 100.0, 200.0, 80.0, 40.0);
    w("b2-1-letter-hello-rect", &c1);
    // 2. Two pages, text on each.
    let two = new_publication(&NewPublication::letter(2)).unwrap();
    let c2 = add_text_box(&two, 0, 72.0, 72.0, 200.0, 50.0, "Page one text").unwrap();
    let c2 = add_text_box(&c2, 1, 100.0, 300.0, 250.0, 60.0, "Page two text").unwrap();
    w("b2-2-two-pages-text", &c2);
    // 2b. The same built by adding a page to a one-page file.
    let c2b = add_page(&add_text_box(&l, 0, 72.0, 72.0, 200.0, 50.0, "Page one text").unwrap(), Some(0)).unwrap();
    let c2b = add_text_box(&c2b, 1, 100.0, 300.0, 250.0, 60.0, "Page two text").unwrap();
    w("b2-2b-added-page-text", &c2b);
    // 3. A4 landscape with an oval, a line and a group.
    let a4 = new_publication(&NewPublication::from_points(841.9, 595.3, 1)).unwrap();
    let c3 = add_shape(&a4, 0, NewShapeKind::Oval, 60.0, 60.0, 150.0, 100.0, ShapeStyle { fill: Some([255, 200, 0]), ..Default::default() }).unwrap();
    let c3 = add_shape(&c3, 0, NewShapeKind::Line { flip_h: false, flip_v: false }, 60.0, 250.0, 300.0, 0.0, ShapeStyle::default()).unwrap();
    let c3 = rect(&c3, 0, 400.0, 60.0, 100.0, 80.0);
    let c3 = add_shape(&c3, 0, NewShapeKind::Oval, 450.0, 120.0, 100.0, 100.0, ShapeStyle::default()).unwrap();
    let c3 = group_shapes(&c3, 0, &[2, 3]).unwrap();
    w("b2-3-a4-landscape-oval-line-group", &c3);
    // 4. A text box with several paragraphs.
    let c4 = add_text_box(&l, 0, 72.0, 72.0, 400.0, 200.0, "First paragraph\nSecond paragraph\nThird paragraph, a little longer than the others").unwrap();
    w("b2-4-paragraphs", &c4);
    // 5. Page operations on text pages, and two boxes.
    let c5 = add_text_box(&c2b, 1, 100.0, 400.0, 200.0, 40.0, "Second box on page two").unwrap();
    let c5 = move_page(&c5, 1, 0).unwrap();
    let c5 = duplicate_page(&c5, 0).unwrap();
    w("b2-5-pages-moved-duplicated", &c5);
}

/// Writes the files used for the manual Publisher check when `PW_BLANK_OUT` names a folder.
#[test]
#[ignore]
fn write_samples_for_publisher() {
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    for (name, spec) in sizes() {
        let bytes = match new_publication(&spec) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{name}: {e}");
                continue;
            }
        };
        std::fs::write(dir.join(format!("{name}.pub")), &bytes).unwrap();
        if name == "letter" || name == "a4" {
            let b = add_text_box(&bytes, 0, 72.0, 72.0, 200.0, 50.0, "Hello")
                .and_then(|b| add_shape(&b, 0, NewShapeKind::Rectangle, 100.0, 200.0, 80.0, 40.0, ShapeStyle::default()));
            match b {
                Ok(b) => std::fs::write(dir.join(format!("{name}-edited.pub")), b).unwrap(),
                Err(e) => eprintln!("{name} edit: {e}"),
            }
        }
    }
    // The reductions of spec 6.8/6.9.
    let l = NewPublication::letter(1);
    let variants: Vec<(&str, BlankOptions)> = vec![
        ("v-esc2021", BlankOptions { escher_2021: true, ..Default::default() }),
        ("v-nothumb", BlankOptions { thumbnail: false, ..Default::default() }),
        ("v-nosummary", BlankOptions { summary_information: false, ..Default::default() }),
        ("v-nodocsummary", BlankOptions { document_summary: false, ..Default::default() }),
        ("v-emptysetup", BlankOptions { empty_page_setup: true, ..Default::default() }),
        ("v-0x258", BlankOptions { style_0x258: true, ..Default::default() }),
        ("v-nocustompreset", BlankOptions { letter_preset: false, ..Default::default() }),
        ("v-nohelpers", BlankOptions { omit_helper_pages: true, ..Default::default() }),
        ("v-nomisc", BlankOptions { omit_misc_objects: true, ..Default::default() }),
        ("v-core", BlankOptions { omit: &[257, 259, 261, 262, 278, 282, 283, 284, 285, 287, 288, 291, 292, 293, 294, 295], omit_helper_pages: true, ..Default::default() }),
        ("v-core-a", BlankOptions { omit: &[257, 259, 261, 262, 278, 283, 284, 288, 294, 295], omit_helper_pages: true, ..Default::default() }),
        ("v-core-b", BlankOptions { omit: &[282, 283, 284, 285, 287, 288, 291, 292, 293, 294, 295], omit_helper_pages: true, ..Default::default() }),
        ("v-core-c", BlankOptions { omit: &[257, 259, 261, 262, 278, 282, 283, 284, 285, 287, 288, 291, 292, 294, 295], omit_helper_pages: true, ..Default::default() }),
        ("v-core-d", BlankOptions { omit: &[257, 259, 261, 262, 278, 282, 283, 284, 285, 287, 288, 291, 292, 294, 295, 289], omit_helper_pages: true, ..Default::default() }),
        ("v-o257", BlankOptions { omit: &[257], ..Default::default() }),
        ("v-o259", BlankOptions { omit: &[259], ..Default::default() }),
        ("v-o261", BlankOptions { omit: &[261], ..Default::default() }),
        ("v-o262", BlankOptions { omit: &[262], ..Default::default() }),
        ("v-o278", BlankOptions { omit: &[278], ..Default::default() }),
        ("v-o282", BlankOptions { omit: &[282], ..Default::default() }),
        ("v-o285", BlankOptions { omit: &[285], ..Default::default() }),
        ("v-o287", BlankOptions { omit: &[287], ..Default::default() }),
        ("v-o291", BlankOptions { omit: &[291], ..Default::default() }),
        ("v-o292", BlankOptions { omit: &[292], ..Default::default() }),
        ("v-no288", BlankOptions { omit: &[288], ..Default::default() }),
        ("v-no293", BlankOptions { omit: &[293, 294], ..Default::default() }),
        ("v-no294", BlankOptions { omit: &[294], ..Default::default() }),
        ("v-no295", BlankOptions { omit: &[295], ..Default::default() }),
        ("v-nostorages", BlankOptions { omit_empty_storages: true, ..Default::default() }),
        (
            "v-h1",
            BlankOptions { thumbnail: false, empty_page_setup: true, ..Default::default() },
        ),
        (
            "v-h2",
            BlankOptions {
                thumbnail: false,
                empty_page_setup: true,
                omit_helper_pages: true,
                omit_misc_objects: true,
                omit_empty_storages: true,
                document_summary: false,
                ..Default::default()
            },
        ),
    ];
    for (name, o) in variants {
        let bytes = new_publication_with(&l, &o).unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), bytes).unwrap();
    }
}

/// Probes for the manual Publisher check: which stream damage gives which refusal message.
#[test]
#[ignore]
fn write_probes_for_publisher() {
    use std::io::Write;
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let base = new_publication(&NewPublication::letter(1)).unwrap();
    let mutate = |name: &str, f: &dyn Fn(&mut cfb::CompoundFile<std::io::Cursor<Vec<u8>>>)| {
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
        f(&mut cf);
        cf.flush().unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), cf.into_inner().into_inner()).unwrap();
    };
    mutate("p-noescher", &|cf| cf.remove_stream("/Escher/EscherStm").unwrap());
    mutate("p-noquill", &|cf| cf.remove_stream("/Quill/QuillSub/CONTENTS").unwrap());
    mutate("p-nocontents", &|cf| cf.remove_stream("/Contents").unwrap());
    mutate("p-badsig", &|cf| {
        let mut s = cf.open_stream("/Contents").unwrap();
        s.write_all(&[0, 0]).unwrap();
    });
    mutate("p-badquilltag", &|cf| {
        let mut s = cf.open_stream("/Quill/QuillSub/CONTENTS").unwrap();
        s.write_all(b"X").unwrap();
    });
    mutate("p-noenvelope", &|cf| cf.remove_stream("/Envelope").unwrap());
    mutate("p-nocompobj", &|cf| cf.remove_stream("/\u{1}CompObj").unwrap());
    mutate("p-nointernal", &|cf| cf.remove_stream("/\u{3}Internal").unwrap());
    mutate("p-noquillsub", &|cf| {
        cf.remove_stream("/Quill/QuillSub/CONTENTS").unwrap();
        cf.remove_stream("/Quill/QuillSub/\u{1}CompObj").unwrap();
        cf.remove_storage("/Quill/QuillSub").unwrap();
    });
}

/// More probes: damage inside the text engine stream and combinations.
#[test]
#[ignore]
fn write_probes2_for_publisher() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let base = new_publication(&NewPublication::letter(1)).unwrap();
    let mutate = |name: &str, f: &dyn Fn(&mut cfb::CompoundFile<std::io::Cursor<Vec<u8>>>)| {
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
        f(&mut cf);
        cf.flush().unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), cf.into_inner().into_inner()).unwrap();
    };
    mutate("q-noq-noe", &|cf| {
        cf.remove_stream("/Quill/QuillSub/CONTENTS").unwrap();
        cf.remove_stream("/Escher/EscherStm").unwrap();
    });
    // Overwrite the body of one Quill chunk with 0xFF.
    let smash = |tag: &'static [u8; 4], inst: u16| {
        move |cf: &mut cfb::CompoundFile<std::io::Cursor<Vec<u8>>>| {
            let mut s = cf.open_stream("/Quill/QuillSub/CONTENTS").unwrap();
            let mut q = Vec::new();
            s.read_to_end(&mut q).unwrap();
            for i in 0..8 {
                let e = &q[0x20 + 24 * i..0x20 + 24 * (i + 1)];
                if &e[2..6] == tag && u16::from_le_bytes([e[6], e[7]]) == inst {
                    let off = u32::from_le_bytes(e[16..20].try_into().unwrap()) as u64;
                    let len = u32::from_le_bytes(e[20..24].try_into().unwrap()) as usize;
                    s.seek(SeekFrom::Start(off + 4)).unwrap();
                    s.write_all(&vec![0xFF; len - 4]).unwrap();
                }
            }
        }
    };
    mutate("q-smash-stsh1", &smash(b"STSH", 1));
    mutate("q-smash-font", &smash(b"FONT", 0));
    mutate("q-smash-pl", &smash(b"PL  ", 0));
    // Smash the Escher stream.
    mutate("q-smash-escher", &|cf| {
        let mut s = cf.open_stream("/Escher/EscherStm").unwrap();
        s.write_all(&vec![0xFF; 100]).unwrap();
    });
}

/// Probes 3: damage inside Contents.
#[test]
#[ignore]
fn write_probes3_for_publisher() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let base = new_publication(&NewPublication::letter(1)).unwrap();
    let mutate = |name: &str, off_len: &dyn Fn(&[u8]) -> (usize, usize)| {
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
        let mut c = Vec::new();
        cf.open_stream("/Contents").unwrap().read_to_end(&mut c).unwrap();
        let (off, len) = off_len(&c);
        let mut s = cf.open_stream("/Contents").unwrap();
        s.seek(SeekFrom::Start(off as u64)).unwrap();
        s.write_all(&vec![0xFF; len]).unwrap();
        drop(s);
        cf.flush().unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), cf.into_inner().into_inner()).unwrap();
    };
    let u32at = |c: &[u8], o: usize| u32::from_le_bytes(c[o..o + 4].try_into().unwrap()) as usize;
    mutate("c-smash-table", &|c| {
        let t = u32at(c, 0x1E);
        (t + 40, 200)
    });
    mutate("c-smash-256", &|c| {
        let t = u32at(c, 0x1E);
        let o = t + u32at(c, t);
        (o + 6, 40)
    });
    mutate("c-smash-index", &|c| {
        let i = u32at(c, 0x1A);
        (i + 4000, 60)
    });
    mutate("c-smash-block0", &|_| (0x2C + 8, 20));
    mutate("c-smash-266", &|c| {
        let t = u32at(c, 0x1E);
        let mut o = t + u32at(c, t);
        for _ in 0..(1 + 4 + 3) {
            o += u32at(c, o);
        }
        (o + 6, 30)
    });
}

/// Probes 4: truncated drawing streams.
#[test]
#[ignore]
fn write_probes4_for_publisher() {
    use std::io::{Read, Write};
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let base = new_publication(&NewPublication::letter(1)).unwrap();
    let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
    let mut e = Vec::new();
    cf.open_stream("/Escher/EscherStm").unwrap().read_to_end(&mut e).unwrap();
    let dgg_len = 8 + u32::from_le_bytes(e[4..8].try_into().unwrap()) as usize;
    let variants: Vec<(&str, Vec<u8>)> = vec![
        ("e-empty", vec![]),
        ("e-dgg-only", e[..dgg_len].to_vec()),
        ("e-d3-removed", {
            // drop the first drawing (prefix + container)
            let p = dgg_len;
            let l = 4 + 8 + u32::from_le_bytes(e[p + 8..p + 12].try_into().unwrap()) as usize;
            [&e[..p], &e[p + l..]].concat()
        }),
    ];
    for (name, bytes) in variants {
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
        let mut s = cf.open_stream("/Escher/EscherStm").unwrap();
        s.set_len(0).unwrap();
        s.write_all(&bytes).unwrap();
        drop(s);
        cf.flush().unwrap();
        std::fs::write(dir.join(format!("{name}.pub")), cf.into_inner().into_inner()).unwrap();
    }
}

/// Test-only hybrids for the manual Publisher check (evidence only, never shipped): a
/// Publisher-made blank with ONE of our streams, and our file with ONE of Publisher's.
#[test]
#[ignore]
fn write_hybrids_for_publisher() {
    use std::io::{Read, Write};
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(refb) = std::fs::read(root.join("out/blank-research/letter_p.pub")) else { return };
    let ours = new_publication(&NewPublication::letter(1)).unwrap();
    let paths = [
        "/Contents", "/Quill/QuillSub/CONTENTS", "/Escher/EscherStm", "/\u{1}CompObj",
        "/Quill/QuillSub/\u{1}CompObj", "/\u{3}Internal", "/Envelope",
        "/\u{5}SummaryInformation", "/\u{5}DocumentSummaryInformation",
    ];
    let read = |b: &[u8], p: &str| {
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(b)).unwrap();
        let mut v = Vec::new();
        cf.open_stream(p).unwrap().read_to_end(&mut v).unwrap();
        v
    };
    for (i, p) in paths.iter().enumerate() {
        for (tag, base, donor) in [("ours-in-ref", &refb, &ours), ("ref-in-ours", &ours, &refb)] {
            let data = read(donor, p);
            let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(base.clone())).unwrap();
            let mut s = cf.open_stream(p).unwrap();
            s.set_len(0).unwrap();
            s.write_all(&data).unwrap();
            drop(s);
            cf.flush().unwrap();
            std::fs::write(dir.join(format!("h{i}-{tag}.pub")), cf.into_inner().into_inner()).unwrap();
        }
    }
    std::fs::write(dir.join("h-ref-untouched.pub"), &refb).unwrap();
}

/// Test-only: a Publisher-made blank whose Contents is replaced by each file of
/// `<PW_BLANK_OUT>/../cts/*.bin` (structural hybrids made by a script).
#[test]
#[ignore]
fn write_cts_hybrids_for_publisher() {
    use std::io::Write;
    let Ok(dir) = std::env::var("PW_BLANK_OUT") else { return };
    let dir = std::path::Path::new(&dir);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(refb) = std::fs::read(root.join("out/blank-research/letter_p.pub")) else { return };
    for e in std::fs::read_dir(dir.join("../cts")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) != Some("bin") {
            continue;
        }
        let data = std::fs::read(&p).unwrap();
        let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(refb.clone())).unwrap();
        let mut s = cf.open_stream("/Contents").unwrap();
        s.set_len(0).unwrap();
        s.write_all(&data).unwrap();
        drop(s);
        cf.flush().unwrap();
        let name = p.file_stem().unwrap().to_str().unwrap().to_string();
        std::fs::write(dir.join(format!("x-{name}.pub")), cf.into_inner().into_inner()).unwrap();
    }
}

/// A new text box carries Publisher's default text-frame margin (0.04 in on every side),
/// and the reader returns it per box (Escher 0x81..0x84).
#[test]
fn new_text_box_margins_read_back_as_0_04_in() {
    let blank = new_publication(&NewPublication::letter(1)).unwrap();
    let t = add_text_box(&blank, 0, 72.0, 72.0, 200.0, 50.0, "Hello").unwrap();
    let doc = pub_format::read_bytes(&t).unwrap();
    assert_eq!(doc.pages[0].shapes[0].inset_emu, [36_576; 4]);
}
