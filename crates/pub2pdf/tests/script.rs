//! Superscript and subscript in the PDF (spec 4S.3): glyphs at 0.65 x the nominal size, the
//! baseline raised 0.47 x (super) or lowered 0.145 x (sub) the nominal size.

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

/// Font sizes of the `Tf` operators, in drawing order.
fn sizes(pdf: &[u8]) -> Vec<f32> {
    String::from_utf8_lossy(pdf)
        .lines()
        .filter(|l| l.ends_with(" Tf"))
        .filter_map(|l| l.split_whitespace().rev().nth(1)?.parse().ok())
        .collect()
}

fn render_sample(name: &str) -> (Vec<(f32, f32, Vec<u8>)>, Vec<f32>) {
    let doc = pub_format::read(&sample(name)).unwrap();
    let pdf = render(&doc, &std_opts());
    (shown(&pdf), sizes(&pdf))
}

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn pieces(name: &str) -> Vec<(String, f32, f32)> {
    // (text, y, size) of every piece of the first box.
    let (items, sizes) = render_sample(name);
    assert_eq!(items.len(), sizes.len(), "{items:?} {sizes:?}");
    items.iter().zip(&sizes).map(|(i, s)| (text(&i.2), i.1, *s)).collect()
}

#[kernpage_samples::test]
fn plain_is_one_line_of_nominal_size() {
    let p = pieces("script-plain");
    assert_eq!(p.len(), 1, "{p:?}");
    assert!((p[0].2 - 10.0).abs() < 0.01);
}

#[kernpage_samples::test]
fn superscript_is_small_and_raised() {
    let p = pieces("script-super");
    assert_eq!(p.len(), 3, "{p:?}");
    assert_eq!(p[1].0, "2");
    assert!((p[1].2 - 6.5).abs() < 0.01, "{p:?}");
    assert!((p[1].1 - p[0].1 - 4.7).abs() < 0.02, "{p:?}");
    assert!((p[2].1 - p[0].1).abs() < 0.001 && (p[2].2 - 10.0).abs() < 0.01);
}

#[kernpage_samples::test]
fn subscript_is_small_and_lowered() {
    let p = pieces("script-sub");
    assert_eq!(p.len(), 3, "{p:?}");
    assert!((p[1].2 - 6.5).abs() < 0.01, "{p:?}");
    assert!((p[0].1 - p[1].1 - 1.45).abs() < 0.02, "{p:?}");
}

#[kernpage_samples::test]
fn shift_scales_with_the_nominal_size() {
    let p = pieces("script-super20");
    assert!((p[1].2 - 13.0).abs() < 0.01, "{p:?}");
    assert!((p[1].1 - p[0].1 - 9.4).abs() < 0.02, "{p:?}");
}
