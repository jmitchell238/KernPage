//! PDF export of the editor model (including unsaved edits and new documents).

use kernpage_core::{
    export_pdf, Command, Document, Editor, Paragraph, Rect, Shape, ShapeKind,
};
use std::path::PathBuf;

fn bulletin() -> Document {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/kern/bulletin.kern");
    kernpage_core::native::read_kern(&std::fs::read(p).unwrap()).unwrap()
}

/// Standard-14 fonts only, so the text can be read from the content streams.
fn pdf(doc: &Document) -> String {
    String::from_utf8_lossy(&export_pdf(doc, Vec::new())).into_owned()
}

/// Every `x y Td (text) Tj` in the file as (x, y, text).
fn shown(pdf: &str) -> Vec<(f32, f32, String)> {
    let mut out = Vec::new();
    let mut rest = pdf;
    while let Some(i) = rest.find(" Td") {
        let nums: Vec<f32> = rest[..i]
            .split_whitespace()
            .rev()
            .take(2)
            .filter_map(|t| t.parse().ok())
            .collect();
        rest = &rest[i + 3..];
        let (Some(a), Some(b)) = (rest.find('('), rest.find(") Tj")) else {
            break;
        };
        if nums.len() == 2 && a < b {
            out.push((nums[1], nums[0], rest[a + 1..b].to_owned()));
        }
        rest = &rest[b..];
    }
    out
}

fn all_text(pdf: &str) -> String {
    shown(pdf)
        .into_iter()
        .map(|t| t.2)
        .collect::<Vec<_>>()
        .join(" ")
}

#[kernpage_samples::test]
fn bulletin_model_exports_its_text() {
    let doc = bulletin();
    let out = pdf(&doc);
    assert!(out.starts_with("%PDF"));
    let text = all_text(&out);
    let mut checked = 0;
    for shape in doc.pages.iter().flat_map(|p| &p.shapes) {
        if let ShapeKind::TextBox { paragraphs } = &shape.kind {
            for word in paragraphs
                .iter()
                .flat_map(|p| {
                    p.text()
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .take(3)
            {
                let word = word.replace(['(', ')', '\\'], "");
                if word.chars().all(|c| c.is_ascii_alphanumeric()) {
                    assert!(text.contains(&word), "missing {word:?}");
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 5, "only {checked} words checked");
}

#[kernpage_samples::test]
fn model_export_matches_pub_export() {
    // The same page count as the original file.
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen/bulletin.pub");
    let src = pub_format::read(&kernpage_samples::resolve(&p)).unwrap();
    let doc = bulletin();
    let out = pdf(&doc);
    assert_eq!(out.matches("/Type /Page\n").count(), src.pages.len().max(1));
}

#[kernpage_samples::test]
fn edited_model_exports_the_edit() {
    let doc = bulletin();
    let (page, id, first) = doc
        .pages
        .iter()
        .enumerate()
        .find_map(|(pi, p)| {
            p.shapes.iter().find_map(|s| match &s.kind {
                ShapeKind::TextBox { paragraphs } if paragraphs[0].text().len() > 3 => {
                    Some((pi, s.id, paragraphs[0].clone()))
                }
                _ => None,
            })
        })
        .unwrap();
    let retext = Command::SetText {
        page,
        id,
        paragraphs: vec![Paragraph {
            runs: vec![first.base_run().with_text("Zebrafish Roundup")],
            ..first.clone()
        }],
    };
    // The same edit without the move gives the reference position.
    let mut plain = Editor::new(doc.clone());
    plain.execute(retext.clone()).unwrap();
    let before = pdf(plain.doc());
    let (x0, y0) = {
        let t = shown(&before)
            .into_iter()
            .find(|t| t.2.contains("Zebrafish"))
            .unwrap();
        (t.0, t.1)
    };

    let mut ed = Editor::new(doc);
    let rect = ed.doc().shape(page, id).unwrap().rect;
    ed.execute(Command::SetRect {
        page,
        id,
        rect: Rect::new(rect.x + 457_200, rect.y, rect.w, rect.h),
    })
    .unwrap();
    ed.execute(retext).unwrap();
    let after = pdf(ed.doc());
    let text = all_text(&after);
    assert!(text.contains("Zebrafish Roundup"));
    let (x1, y1, _) = shown(&after)
        .into_iter()
        .find(|t| t.2.contains("Zebrafish"))
        .unwrap();
    // Moved right by 0.5 inch = 36 pt, same height.
    assert!((x1 - x0 - 36.0).abs() < 0.1, "{x0} -> {x1}");
    assert!((y1 - y0).abs() < 0.1);
}

#[kernpage_samples::test]
fn new_blank_document_with_one_text_box_exports() {
    let mut ed = Editor::new(Document::blank("Publication1", 7_772_400, 10_058_400));
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::text_box(
            Rect::new(914_400, 914_400, 3_657_600, 914_400),
            vec![Paragraph::plain("Hello from a new publication")],
        ),
    })
    .unwrap();
    let out = pdf(ed.doc());
    assert!(out.contains("%PDF"));
    assert!(all_text(&out).contains("Hello from a new publication"));
    // Letter page: 612 x 792 pt.
    assert!(out.contains("/MediaBox [0 0 612 792]"));
}

#[kernpage_samples::test]
fn line_breaks_and_tables_export() {
    let mut p = Paragraph::plain("");
    p.runs[0].text = "one\ntwo".into();
    let mut doc = Document::blank("t", 7_772_400, 10_058_400);
    doc.pages[0].shapes.push(Shape::text_box(
        Rect::new(914_400, 914_400, 3_657_600, 914_400),
        vec![p],
    ));
    doc.pages[0].shapes.push(Shape::new(
        Rect::new(914_400, 3_000_000, 3_657_600, 914_400),
        ShapeKind::Table {
            rows: 1,
            cols: 2,
            col_widths_emu: vec![],
            row_heights_emu: vec![],
            row_set_emu: vec![],
            cells: vec![
                kernpage_core::TableCell::plain("Cell A"),
                kernpage_core::TableCell::plain("Cell B"),
            ],
        },
    ));
    let text = all_text(&pdf(&doc));
    for w in ["one", "two", "Cell A", "Cell B"] {
        assert!(text.contains(w), "{w}");
    }
}
