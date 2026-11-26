//! Conversion of the generated sample publications, checked against the manifest.

use kernpage_core::{from_pub, Align, Document, ShapeKind};
use serde_json::Value;
use std::path::PathBuf;

const TOLERANCE_PT: f64 = 0.5;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load(name: &str) -> Document {
    let path = kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")));
    from_pub(name, &pub_format::read(&path).unwrap())
}

fn manifest() -> Value {
    let text = std::fs::read_to_string(root().join("samples/gen/manifest.json")).unwrap();
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}

fn pt(emu: i64) -> f64 {
    emu as f64 / 12_700.0
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE_PT
}

#[kernpage_samples::test]
fn bulletin_converts_with_manifest_positions() {
    let doc = load("bulletin");
    let m = manifest();
    assert_eq!(
        doc.pages.len(),
        m["bulletin"]["pages"].as_u64().unwrap() as usize
    );
    assert_eq!(
        (doc.pages[0].width_emu, doc.pages[0].height_emu),
        (7_772_400, 10_058_400)
    );

    for b in m["bulletin"]["boxes"].as_array().unwrap() {
        let want = |k: &str| b[k].as_f64().unwrap();
        let first = b["paragraphs"][0]["text"]
            .as_str()
            .unwrap()
            .trim_end_matches('\r');
        let found = doc.pages.iter().flat_map(|p| &p.shapes).any(|s| {
            let ShapeKind::TextBox { paragraphs } = &s.kind else {
                return false;
            };
            paragraphs[0].text() == first
                && close(pt(s.rect.x), want("left_pt"))
                && close(pt(s.rect.y), want("top_pt"))
                && close(pt(s.rect.w), want("width_pt"))
                && close(pt(s.rect.h), want("height_pt"))
        });
        assert!(found, "no text box for manifest entry {b}");
    }

    // The title box: paragraphs, formatting and alignment survive, `\r` is stripped.
    let title = doc.pages[0]
        .shapes
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::TextBox { paragraphs }
                if paragraphs[0].text() == "Grace Community Church" =>
            {
                Some(paragraphs)
            }
            _ => None,
        })
        .expect("title box");
    assert_eq!(title.len(), 2);
    assert_eq!(title[0].align, Align::Center);
    let run = &title[0].runs[0];
    assert_eq!(
        (run.font.as_str(), run.size_pt, run.bold),
        ("Georgia", 24.0, true)
    );
    assert!(title[1].runs[0].italic);
    assert!(!title[0].text().contains('\r'));

    // The picture on page 1 is carried over with its bytes.
    let pic = doc.pages[0]
        .shapes
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Picture { .. }))
        .expect("picture");
    let m_pic = &m["bulletin"]["picture"];
    assert!(close(pt(pic.rect.x), m_pic["left_pt"].as_f64().unwrap()));
    assert!(close(pt(pic.rect.y), m_pic["top_pt"].as_f64().unwrap()));
    // Shape ids are unique across the document.
    let mut ids: Vec<_> = doc
        .pages
        .iter()
        .flat_map(|p| p.shapes.iter().map(|s| s.id))
        .collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n);
    assert!(ids.iter().all(|&i| i > 0));
}

#[kernpage_samples::test]
fn shapes_and_table_samples_convert_to_their_kinds() {
    let doc = load("shapes");
    let kinds: Vec<_> = doc.pages[0].shapes.iter().map(|s| &s.kind).collect();
    assert!(kinds.iter().any(|k| matches!(k, ShapeKind::Ellipse)));
    assert!(kinds.iter().any(|k| matches!(k, ShapeKind::Line)));
    assert!(doc.pages[0].shapes.iter().any(|s| s.fill.is_some()));

    let doc = load("table");
    let t = doc.pages[0]
        .shapes
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::Table {
                rows, cols, cells, ..
            } => Some((*rows, *cols, cells.len())),
            _ => None,
        })
        .expect("a table");
    assert_eq!(t.2 as u32, t.0 * t.1);
}

#[kernpage_samples::test]
fn formatting_sample_keeps_fonts_and_styles() {
    let doc = load("formatting");
    let fonts = doc.fonts_used();
    assert!(fonts.iter().any(|(f, _, _)| f == "Arial"));
    assert!(fonts.iter().any(|(f, _, i)| f == "Times New Roman" && *i));
}

fn frame_text(s: &kernpage_core::Shape) -> String {
    match &s.kind {
        ShapeKind::TextBox { paragraphs } => paragraphs
            .iter()
            .map(|p| p.text())
            .collect::<Vec<_>>()
            .join("\n"),
        other => panic!("not a text box: {other:?}"),
    }
}

#[kernpage_samples::test]
fn linked_frames_get_only_their_share() {
    let pf_doc = pub_format::read(&kernpage_samples::resolve(root().join("samples/gen/linked.pub"))).unwrap();
    // Breaks as pub2pdf would report them: frame 1 ends in paragraph 6, frame 2 after 9.
    let breaks = vec![vec![(5, 48), (9, 0), (12, 0)]];
    let doc = kernpage_core::from_pub_flowed("linked", &pf_doc, &breaks);
    let chain = &pf_doc.chains()[0];
    let texts: Vec<String> = chain
        .iter()
        .map(|r| frame_text(&doc.pages[r.page].shapes[r.shape]))
        .collect();
    assert!(texts[0].starts_with("Paragraph 1 of"));
    assert!(texts[0].contains("Paragraph 6 of"));
    assert!(!texts[0].contains("Paragraph 7 of"));
    // The break fell inside paragraph 6: frame 2 continues it, then holds 7-9.
    let (head, tail) = texts[0].rsplit_once("Paragraph 6").unwrap();
    let rest = &texts[1];
    assert_eq!(tail.len() + "Paragraph 6".len(), 48);
    assert!(head.ends_with('\n'));
    assert!(rest.contains("Paragraph 9 of") && !rest.contains("Paragraph 10 of"));
    assert!(texts[2].starts_with("Paragraph 10 of") && !texts[2].contains("Paragraph 13"));
    // The standalone box is untouched.
    let alone = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| frame_text(s) == "Standalone box")
        .expect("standalone box");
    assert!(alone.is_text_box());
}

#[kernpage_samples::test]
fn unsplit_linked_chain_keeps_whole_story_in_first_frame() {
    let pf_doc = pub_format::read(&kernpage_samples::resolve(root().join("samples/gen/linked.pub"))).unwrap();
    let doc = from_pub("linked", &pf_doc);
    let chain = &pf_doc.chains()[0];
    let texts: Vec<String> = chain
        .iter()
        .map(|r| frame_text(&doc.pages[r.page].shapes[r.shape]))
        .collect();
    assert!(texts[0].contains("Paragraph 30 of"));
    assert!(texts[1].is_empty() && texts[2].is_empty());
}

#[kernpage_samples::test]
fn rotation_and_columns_reach_the_model() {
    let angles: Vec<f32> = load("rotate").pages[0]
        .shapes
        .iter()
        .map(|s| s.rotation_deg)
        .collect();
    for want in [90.0, 270.0, 30.0, 45.0] {
        assert!(angles.contains(&want), "{want} in {angles:?}");
    }
    let cols = load("columns");
    let mut got: Vec<(u32, i64)> = cols.pages[0]
        .shapes
        .iter()
        .filter(|s| s.is_text_box())
        .map(|s| (s.columns, s.column_gap_emu))
        .collect();
    got.sort();
    assert_eq!(got, [(2, 36_576), (3, 228_600)]);
}

#[kernpage_samples::test]
fn table2_keeps_sizes_and_cell_paragraphs() {
    let doc = load("table2");
    let (cw, rh, cells) = doc.pages[0]
        .shapes
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::Table {
                col_widths_emu,
                row_heights_emu,
                cells,
                ..
            } => Some((col_widths_emu, row_heights_emu, cells)),
            _ => None,
        })
        .expect("a table");
    assert_eq!(cw, &vec![914_400, 2_743_200, 1_828_800]);
    assert_eq!(rh, &vec![457_200, 1_371_600, 914_400]);
    let counts: Vec<usize> = cells.iter().map(|c| c.paragraphs.len()).collect();
    assert_eq!(counts, vec![1, 1, 1, 1, 3, 1, 2, 1, 1]);
    assert_eq!(cells[4].paragraphs[1].text(), "Second paragraph in B2");
}

/// The per-box text-frame margins of a file come through to the model, in EMU.
#[kernpage_samples::test]
fn text_box_margins_are_read_from_the_file() {
    let d = load("columns");
    let boxes: Vec<_> = d.pages[0].shapes.iter().filter(|s| s.is_text_box()).collect();
    assert!(!boxes.is_empty());
    for s in boxes {
        assert_eq!(s.inset_emu, [36_576; 4]);
        let pt = s.inset_pt();
        assert!((pt[0] - 2.88).abs() < 1e-9);
    }
}
