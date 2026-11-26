//! Bullets and numbering in the PDF (spec 4R): the label is drawn at the first-line
//! position and the text starts at the list indent; wrapped lines line up with the text.

use pub2pdf::{render, Options};
use std::path::PathBuf;

fn sample(name: &str) -> PathBuf {
    kernpage_samples::path(&format!("gen/{name}.pub"))
}

/// Standard-14 fonts only, so the text of the PDF is plain bytes.
fn std_opts() -> Options {
    Options {
        font_dirs: Vec::new(),
        outlines: false,
        ..Options::default()
    }
}

/// Decode the string of a `(...) Tj` line (PDF literal string escapes).
fn decode(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i] != b'\\' {
            out.push(s[i]);
            i += 1;
            continue;
        }
        i += 1;
        match s.get(i) {
            Some(d) if d.is_ascii_digit() => {
                let mut v = 0u32;
                let mut n = 0;
                while n < 3 && s.get(i).is_some_and(u8::is_ascii_digit) {
                    v = v * 8 + u32::from(s[i] - b'0');
                    i += 1;
                    n += 1;
                }
                out.push(v as u8);
            }
            Some(b'n') => {
                out.push(b'\n');
                i += 1;
            }
            Some(&c) => {
                out.push(c);
                i += 1;
            }
            None => {}
        }
    }
    out
}

/// Every shown string of the PDF as (x, y, bytes), in drawing order.
fn shown(pdf: &[u8]) -> Vec<(f32, f32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = (0.0f32, 0.0f32);
    for line in pdf.split(|&b| b == b'\n') {
        if line.ends_with(b" Td") {
            let t = String::from_utf8_lossy(line);
            let mut it = t.split_whitespace();
            if let (Some(x), Some(y)) = (it.next(), it.next()) {
                if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                    at = (x, y);
                }
            }
        } else if line.starts_with(b"(") && line.ends_with(b") Tj") {
            out.push((at.0, at.1, decode(&line[1..line.len() - 4])));
        } else if line.starts_with(b"<") && line.ends_with(b"> Tj") {
            let hex = String::from_utf8_lossy(&line[1..line.len() - 4]).into_owned();
            let bytes = (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
                .collect();
            out.push((at.0, at.1, bytes));
        }
    }
    out
}

fn render_sample(name: &str) -> Vec<(f32, f32, Vec<u8>)> {
    let doc = pub_format::read(&sample(name)).unwrap();
    shown(&render(&doc, &std_opts()))
}

/// The left edge of the text of a box at 72 pt (2.88 pt text-frame margin + 0.72 pt).
const LEFT: f32 = 75.6;

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

#[kernpage_samples::test]
fn bullet_list_draws_a_dot_at_the_first_line_and_text_at_the_list_indent() {
    // One (plain), Two (bullet), Three (plain).
    let items = render_sample("list-bullet-default");
    assert_eq!(items.len(), 4, "{items:?}");
    assert_eq!(text(&items[0].2), "One");
    assert!((items[0].0 - LEFT).abs() < 0.05, "{items:?}");
    // The bullet (U+2022, WinAnsi 0x95) sits at the first-line indent (left + (-18)).
    let (label, body) = (&items[1], &items[2]);
    assert_eq!(label.2, vec![0x95], "{items:?}");
    assert!((label.0 - LEFT).abs() < 0.05, "{items:?}");
    // The text starts at the list indent, 18 pt right of the label, on the same line.
    assert_eq!(text(&body.2), "Two");
    assert!((body.0 - (LEFT + 18.0)).abs() < 0.05, "{items:?}");
    assert!((label.1 - body.1).abs() < 0.01, "{items:?}");
    assert_eq!(text(&items[3].2), "Three");
    assert!((items[3].0 - LEFT).abs() < 0.05, "{items:?}");
}

#[kernpage_samples::test]
fn numbered_list_draws_each_number_and_restarts_after_a_plain_paragraph() {
    let items = render_sample("list-num-1dot");
    let labels: Vec<String> = items.iter().step_by(2).map(|i| text(&i.2)).collect();
    assert_eq!(labels, ["1.", "2.", "3."], "{items:?}");
    // 1. One, 2. Two, Three (plain), 1. Four, 2. Five.
    let items = render_sample("list-num-mixed");
    let labels: Vec<String> = items
        .iter()
        .filter(|i| (i.0 - LEFT).abs() < 0.05)
        .map(|i| text(&i.2))
        .collect();
    assert_eq!(labels, ["1.", "2.", "Three", "1.", "2."], "{items:?}");
    for (i, word) in items.iter().filter(|i| (i.0 - (LEFT + 18.0)).abs() < 0.05).zip(["One", "Two", "Four", "Five"]) {
        assert_eq!(text(&i.2), word);
    }
}

#[kernpage_samples::test]
fn list_indent_moves_the_text() {
    let items = render_sample("list-num-indent36");
    // The first paragraph has list indent 36: label at the left edge, text 36 pt in.
    assert_eq!(text(&items[0].2), "1.");
    assert!((items[0].0 - LEFT).abs() < 0.05, "{items:?}");
    assert!((items[1].0 - (LEFT + 36.0)).abs() < 0.05, "{items:?}");
}

#[kernpage_samples::test]
fn wrapped_lines_line_up_with_the_text_not_the_label() {
    let mut doc = pub_format::read(&sample("list-num-1dot")).unwrap();
    let long = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron";
    for s in doc.pages.iter_mut().flat_map(|p| p.shapes.iter_mut()) {
        if let Some(p) = s.paragraphs.first_mut() {
            p.runs[0].text = format!("{long}\r");
            for r in p.runs.iter_mut().skip(1) {
                r.text.clear();
            }
        }
    }
    let items = shown(&render(&doc, &std_opts()));
    // One label (the first paragraph's) and at least two lines of its text.
    let first_label = items.iter().position(|i| text(&i.2) == "1.").unwrap();
    assert!((items[first_label].0 - LEFT).abs() < 0.05);
    let first_para: Vec<_> = items[first_label + 1..]
        .iter()
        .take_while(|i| (i.0 - LEFT).abs() > 0.05)
        .collect();
    assert!(first_para.len() >= 2, "{items:?}");
    // Every line of the paragraph, wrapped or not, starts at the list indent.
    for line in &first_para {
        assert!((line.0 - (LEFT + 18.0)).abs() < 0.05, "{items:?}");
    }
}
