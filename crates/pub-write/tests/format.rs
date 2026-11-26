//! Per-run character and paragraph formatting (spec 3.5, 3.6, 3.7).

use pub_format::Alignment;
use pub_write::{
    set_story_format, CharFormat, CharSpan, LineRule, ParaAlign, ParaSpacing, ParaSpan, WriteError,
};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

fn fmt(bold: bool, italic: bool, underline: bool, size_pt: f32, font: &str) -> CharFormat {
    CharFormat {
        bold,
        italic,
        underline,
        script: pub_format::Script::Normal,
        size_pt,
        font: font.to_owned(),
        color: None,
        scheme_color: None,
    }
}

/// (text, bold, italic, underline, size, font, alignment) per run of every paragraph of
/// the first text box.
type RunView = (String, bool, bool, bool, f32, String);

fn view(bytes: &[u8], shape: usize) -> Vec<(Vec<RunView>, Alignment)> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages[0].shapes[shape]
        .paragraphs
        .iter()
        .map(|p| {
            (
                p.runs
                    .iter()
                    .map(|r| {
                        (
                            r.text.replace('\r', ""),
                            r.bold,
                            r.italic,
                            r.underline,
                            r.size_pt,
                            r.font.clone(),
                        )
                    })
                    .filter(|r| !r.0.is_empty())
                    .collect(),
                p.alignment,
            )
        })
        .collect()
}

#[kernpage_samples::test]
fn bold_a_word_and_change_size_font_alignment() {
    let bytes = gen("bulletin");
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let idx = doc.pages[0]
        .shapes
        .iter()
        .position(|s| s.text.as_deref().is_some_and(|t| t.contains("Prelude")))
        .unwrap();
    let story = doc
        .stories
        .iter()
        .position(|t| t.contains("Prelude"))
        .unwrap();
    // "Order of Service\r" is 17 units; "Prelude\r" 17..25; "Call to Worship\r" 25..41.
    let out = set_story_format(
        &bytes,
        story,
        &[
            CharSpan {
                start: 17,
                end: 24,
                format: fmt(true, false, false, 10.0, "Calibri"),
            },
            CharSpan {
                start: 25,
                end: 41,
                format: fmt(false, true, true, 18.0, "Georgia"),
            },
        ],
        &[ParaSpan {
            start: 25,
            end: 41,
            align: ParaAlign::Center,
            spacing: None,
        }],
    )
    .unwrap();
    let v = view(&out, idx);
    // Prelude: bold, other formatting unchanged.
    assert_eq!(v[1].0.len(), 1);
    assert_eq!(v[1].0[0].0, "Prelude");
    assert!(v[1].0[0].1 && !v[1].0[0].2);
    assert_eq!(v[1].1, Alignment::Left);
    // Call to Worship: italic underline 18 pt Georgia centred; Georgia was not in the font
    // table (or is now appended) and reads back by name.
    let (runs, align) = &v[2];
    assert_eq!(runs.len(), 1, "{runs:?}");
    let r = &runs[0];
    assert_eq!(r.0, "Call to Worship");
    assert!(!r.1 && r.2 && r.3);
    assert_eq!(r.4, 18.0);
    assert_eq!(r.5, "Georgia");
    assert_eq!(*align, Alignment::Center);
    // Everything else is unchanged.
    assert_eq!(v[0], view(&bytes, idx)[0]);
    assert_eq!(v[3..], view(&bytes, idx)[3..]);
    // The text is untouched.
    let back = pub_format::read_bytes(&out).unwrap();
    assert_eq!(back.stories, doc.stories);
    // Paragraph runs still end at every paragraph break, one run per paragraph (Publisher
    // hangs when one spans several paragraphs).
    for (i, _) in doc.stories.iter().enumerate() {
        let recs = pub_write::story_records(&out, i).unwrap();
        let before = pub_write::story_records(&bytes, i).unwrap();
        assert_eq!(recs.paragraphs.len(), before.paragraphs.len(), "story {i}");
        for p in &recs.paragraphs {
            assert_eq!(recs.text[p.end - 1], 0x0D, "story {i}: {p:?}");
        }
    }
}

#[kernpage_samples::test]
fn existing_font_is_reused_and_formatting_can_be_removed() {
    let bytes = gen("formatting");
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let before = view(&bytes, 0);
    // "Times New Roman 18 italic\r" (9..35): make it plain Arial 12; the centred paragraph
    // ("Centered\r") back to left.
    let text = &doc.stories[0];
    let centred = text.find("Centered").unwrap();
    let out = set_story_format(
        &bytes,
        0,
        &[CharSpan {
            start: 9,
            end: 35,
            format: fmt(false, false, false, 12.0, "Arial"),
        }],
        &[ParaSpan {
            start: centred,
            end: centred + 9,
            align: ParaAlign::Left,
            spacing: None,
        }],
    )
    .unwrap();
    let after = view(&out, 0);
    let r = &after[1].0[0];
    assert_eq!(
        (r.1, r.2, r.3, r.4, r.5.as_str()),
        (false, false, false, 12.0, "Arial")
    );
    assert_eq!(after[5].1, Alignment::Left);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[6], before[6]);
}

#[kernpage_samples::test]
fn text_outside_the_spans_and_equal_formats_change_nothing() {
    let bytes = gen("formatting");
    // Spans that already match: the formatting reads back identically.
    let out = set_story_format(
        &bytes,
        0,
        &[CharSpan {
            start: 0,
            end: 8,
            format: fmt(false, false, false, 12.0, "Arial"),
        }],
        &[],
    )
    .unwrap();
    assert_eq!(view(&out, 0), view(&bytes, 0));
}

#[kernpage_samples::test]
fn errors_for_bad_story_and_new_font_names_round_trip() {
    let bytes = gen("apple-banana");
    assert!(matches!(
        set_story_format(&bytes, 9, &[], &[]),
        Err(WriteError::NoSuchStory { .. })
    ));
    // Two new fonts in a row: both are appended and stay readable.
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let story = doc
        .stories
        .iter()
        .position(|t| t.contains("Apple"))
        .unwrap();
    let a = set_story_format(
        &bytes,
        story,
        &[CharSpan {
            start: 0,
            end: 6,
            format: fmt(false, false, false, 10.0, "Cooper Black"),
        }],
        &[],
    )
    .unwrap();
    let b = set_story_format(
        &a,
        story,
        &[CharSpan {
            start: 0,
            end: 6,
            format: fmt(false, false, false, 10.0, "Garamond"),
        }],
        &[],
    )
    .unwrap();
    for (bytes, name) in [(&a, "Cooper Black"), (&b, "Garamond")] {
        let d = pub_format::read_bytes(bytes).unwrap();
        assert!(d.pages[0]
            .shapes
            .iter()
            .any(|s| s.runs.iter().any(|r| r.font == name)));
    }
}

fn story_and_shape(doc: &pub_format::Document, needle: &str) -> (usize, usize) {
    (
        doc.stories.iter().position(|t| t.contains(needle)).unwrap(),
        doc.pages[0]
            .shapes
            .iter()
            .position(|s| s.text.as_deref().is_some_and(|t| t.contains(needle)))
            .unwrap(),
    )
}

#[kernpage_samples::test]
fn colours_and_paragraph_spacing_round_trip() {
    let bytes = gen("bulletin");
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let (story, shape) = story_and_shape(&doc, "Prelude");
    let text = &doc.stories[story];
    let at = |w: &str| text[..text.find(w).unwrap()].encode_utf16().count();
    let (prelude, call) = (at("Prelude"), at("Call to Worship"));
    let red = [255, 0, 0];
    let rgb_of = |bytes: &[u8], word: &str| {
        let d = pub_format::read_bytes(bytes).unwrap();
        let sch = d.color_scheme.as_ref();
        let r = d.pages[0].shapes[shape]
            .paragraphs
            .iter()
            .flat_map(|p| p.runs.iter())
            .find(|r| r.text.contains(word))
            .unwrap()
            .clone();
        r.color.map(|c| c.to_rgb(sch))
    };
    let spacing = ParaSpacing {
        space_before_pt: 18.0,
        space_after_pt: 24.0,
        line: Some(LineRule::Multiple(2.0)),
    };
    let mut f = fmt(false, false, false, 10.0, "Calibri");
    f.font = view(&bytes, shape)[1].0[0].5.clone();
    f.size_pt = view(&bytes, shape)[1].0[0].4;
    f.color = Some(red);
    let out = set_story_format(
        &bytes,
        story,
        &[CharSpan {
            start: prelude,
            end: prelude + 7,
            format: f.clone(),
        }],
        &[ParaSpan {
            start: prelude,
            end: prelude + 8,
            align: ParaAlign::Left,
            spacing: Some(spacing),
        }],
    )
    .unwrap();
    assert_eq!(rgb_of(&out, "Prelude"), Some(red));
    let d = pub_format::read_bytes(&out).unwrap();
    let p = d.pages[0].shapes[shape]
        .paragraphs
        .iter()
        .find(|p| p.runs.iter().any(|r| r.text.contains("Prelude")))
        .unwrap();
    assert_eq!(p.spacing.space_before_pt, 18.0);
    assert_eq!(p.spacing.space_after_pt, 24.0);
    assert_eq!(p.spacing.line, pub_format::LineSpacing::Multiple(2.0));
    // A second word takes the same colour (entry reused), a third goes back to default.
    let again = set_story_format(
        &out,
        story,
        &[
            CharSpan {
                start: call,
                end: call + 4,
                format: f.clone(),
            },
            CharSpan {
                start: prelude,
                end: prelude + 7,
                format: CharFormat {
                    color: Some([0, 0, 0]),
                    ..f.clone()
                },
            },
        ],
        &[],
    )
    .unwrap();
    assert_eq!(rgb_of(&again, "Call"), Some(red));
    assert_eq!(rgb_of(&again, "Prelude"), None);
    // An exact line spacing, then unchanged spacing leaves the bytes' reading alone.
    let exact = set_story_format(
        &again,
        story,
        &[],
        &[ParaSpan {
            start: prelude,
            end: prelude + 8,
            align: ParaAlign::Left,
            spacing: Some(ParaSpacing {
                line: Some(LineRule::Exactly(30.0)),
                ..spacing
            }),
        }],
    )
    .unwrap();
    let d = pub_format::read_bytes(&exact).unwrap();
    let p = d.pages[0].shapes[shape]
        .paragraphs
        .iter()
        .find(|p| p.runs.iter().any(|r| r.text.contains("Prelude")))
        .unwrap();
    assert_eq!(p.spacing.line, pub_format::LineSpacing::Exactly(30.0));
}
