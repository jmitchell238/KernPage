//! Saving a publication made in the editor (no file behind it) as a real `.pub`.

use kernpage_core::{
    Command, Document, Editor, Paragraph, Rect, SaveSource, Shape, ShapeKind, Stroke,
};
use kernpage_core::geometry::in_to_emu;
use kernpage_core::Rgb;

fn editor_with_content(doc: Document) -> Editor {
    let mut ed = Editor::new(doc);
    let text = Shape::text_box(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(1.0)),
        vec![Paragraph::plain("Hello new world")],
    );
    ed.execute(Command::AddShape { page: 0, shape: text }).unwrap();
    let mut rect = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(3.0), in_to_emu(2.0), in_to_emu(1.0)),
        ShapeKind::Rectangle,
    );
    rect.line = Some(Stroke {
        color: Rgb(0, 0, 0),
        width_emu: 9_525,
    });
    ed.execute(Command::AddShape { page: 0, shape: rect }).unwrap();
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    ed
}

fn check_saved(bytes: &[u8], doc: &Document, text: &str) {
    let back = pub_format::read_bytes(bytes).unwrap();
    assert_eq!(back.pages.len(), 2);
    assert_eq!(i64::from(back.page_width_emu), doc.pages[0].width_emu);
    assert_eq!(i64::from(back.page_height_emu), doc.pages[0].height_emu);
    let shapes = &back.pages[0].shapes;
    assert!(shapes
        .iter()
        .any(|s| s.text.as_deref().is_some_and(|t| t.contains(text))));
    assert_eq!(shapes.len(), 2, "a text box and a rectangle");
    assert!(back.pages[1].shapes.is_empty());
}

fn save_twice(doc: Document) {
    let mut ed = editor_with_content(doc);
    let mut src = SaveSource::for_new(ed.doc()).unwrap();
    let plan = src.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert!(plan.page_ops.is_empty(), "{:?}", plan.page_ops);
    assert_eq!(plan.new_text_boxes.len(), 1);
    assert_eq!(plan.new_shapes.len(), 1);
    let (bytes, plan) = src.build(ed.doc()).unwrap();
    check_saved(&bytes, ed.doc(), "Hello new world");
    src.mark_saved(bytes, ed.doc(), &plan);
    assert!(src.plan(ed.doc()).is_empty());

    // A second save after an edit.
    let id = ed.doc().pages[0]
        .shapes
        .iter()
        .find(|s| s.is_text_box())
        .unwrap()
        .id;
    ed.execute(Command::SetText {
        page: 0,
        id,
        paragraphs: vec![Paragraph::plain("Changed text")],
    })
    .unwrap();
    let plan = src.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert!(!plan.is_empty());
    let (bytes, _) = src.build(ed.doc()).unwrap();
    check_saved(&bytes, ed.doc(), "Changed text");
}

#[test]
fn new_letter_publication_saves_and_resaves() {
    save_twice(kernpage_core::template::blank_portrait());
}

#[test]
fn new_a4_landscape_publication_saves_and_resaves() {
    // A4 landscape: 297 x 210 mm.
    let doc = Document::blank("Publication2", 297 * 36_000, 210 * 36_000);
    save_twice(doc);
}

/// pw-z641: the columns and gutter of a new text box survive save and reopen.
fn new_box_columns_round_trip(columns: u32, gap_emu: i64) {
    let mut ed = Editor::new(kernpage_core::template::blank_portrait());
    let mut tb = Shape::text_box(
        Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(4.0), in_to_emu(2.0)),
        vec![Paragraph::plain("Column text")],
    );
    tb.columns = columns;
    tb.column_gap_emu = gap_emu;
    ed.execute(Command::AddShape { page: 0, shape: tb }).unwrap();
    let mut src = SaveSource::for_new(ed.doc()).unwrap();
    let plan = src.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (bytes, _) = src.build(ed.doc()).unwrap();
    let back = pub_format::read_bytes(&bytes).unwrap();
    let s = &back.pages[0].shapes[0];
    assert_eq!(s.columns, columns);
    if columns > 1 {
        assert_eq!(i64::from(s.column_gap_emu), gap_emu);
    }
}

#[test]
fn new_text_box_keeps_two_columns() {
    new_box_columns_round_trip(2, 91_440);
}

#[test]
fn new_text_box_keeps_three_columns_and_gutter() {
    new_box_columns_round_trip(3, 182_880);
}
