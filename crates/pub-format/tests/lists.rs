//! Bullets and numbering (spec 4R) read from the 24 Publisher-saved `samples/gen/list-*.pub`.

use pub_format::{list_labels, ListInfo, ListType, ParaIndents, Paragraph};
use std::path::PathBuf;

fn paras(name: &str) -> Vec<Paragraph> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/list-{name}.pub"));
    let doc = pub_format::read(&kernpage_samples::resolve(&p)).unwrap();
    doc.pages
        .iter()
        .flat_map(|pg| pg.shapes.iter())
        .find(|s| s.text.as_deref().is_some_and(|t| t.contains("One")))
        .map(|s| s.paragraphs.clone())
        .unwrap_or_default()
}

fn labels(name: &str) -> Vec<String> {
    list_labels(&paras(name))
        .into_iter()
        .map(|l| l.unwrap_or_else(|| "-".into()))
        .collect()
}

fn ind(l: f32, f: f32, r: f32) -> ParaIndents {
    ParaIndents { left_pt: l, first_line_pt: f, right_pt: r }
}

/// Labels per sample. Where the spec has live PDF-text observations (4R.2, 4R.3, 4R.5) they
/// are the source; the rest follows the stored records.
#[kernpage_samples::test]
fn labels_of_every_sample() {
    let cases: &[(&str, &[&str])] = &[
        ("empty", &[]),
        ("none", &["-", "-", "-"]),
        ("bullet-default", &["-", "\u{b7}", "-"]),
        ("bullet-all", &["\u{b7}", "\u{b7}", "\u{b7}"]),
        ("bullet-wingdings", &["\u{a7}", "\u{fc}", "-"]),
        ("bullet-chars", &["\u{2022}", "*", "o"]),
        ("bullet-size", &["\u{b7}", "\u{b7}", "-"]),
        ("bullet-indent", &["\u{b7}", "\u{b7}", "-"]),
        ("bullet-then-none", &["-", "-", "-"]),
        ("bullet-font-then-num", &["-", "1.", "-"]),
        ("num-1dot", &["1.", "2.", "3."]),
        ("num-1dot-each", &["1.", "2.", "3."]),
        ("num-1paren", &["1)", "2)", "3)"]),
        ("num-aparen", &["a)", "b)", "c)"]),
        ("num-idot", &["i.", "ii.", "iii."]),
        (
            "num-types",
            &["1.", "I.", "i.", "A.", "a.", "1st.", "One.", "First.", "01."],
        ),
        (
            "num-seps",
            &["1)", "(1)", "1.", "1", "1]", "1:", "[1]", "-1-", "1\u{3001}"],
        ),
        ("num-start5", &["5.", "6.", "7."]),
        ("num-start-p2", &["1.", "7.", "8."]),
        ("num-indent36", &["1.", "2.", "3."]),
        ("num-mixed", &["1.", "2.", "-", "1.", "2."]),
        ("num-empty", &["1.", "2.", "3."]),
        ("num-bullet-mix", &["1.", "\u{b7}", "1."]),
        ("indent-plain", &["-", "-", "-"]),
    ];
    for (name, want) in cases {
        assert_eq!(labels(name), *want, "{name}");
    }
}

#[kernpage_samples::test]
fn bullet_properties() {
    let b = |n| paras(n);
    let d = b("bullet-default");
    assert!(d[0].list.is_none() && d[2].list.is_none());
    let l = d[1].list.as_ref().unwrap();
    assert_eq!(
        (l.kind, l.bullet_char, l.bullet_font.as_str(), l.bullet_size_pt, l.start, l.list_indent_pt),
        (ListType::Bullet, 183, "Symbol", Some(10.0), 1, 18.0)
    );
    assert_eq!(d[1].indents, ind(18.0, -18.0, 0.0));
    assert_eq!(d[0].indents, ParaIndents::default());

    let w = b("bullet-wingdings");
    for (p, ch) in w[..2].iter().zip([0xA7, 0xFC]) {
        let l = p.list.as_ref().unwrap();
        assert_eq!((l.bullet_char, l.bullet_font.as_str()), (ch, "Wingdings"));
    }
    let c = b("bullet-chars");
    let fonts: Vec<_> = c.iter().map(|p| p.list.as_ref().unwrap().bullet_font.clone()).collect();
    assert_eq!(fonts, ["Symbol", "Symbol", "Courier New"]);
    assert_eq!(c[0].list.as_ref().unwrap().bullet_char, 8226);

    let s = b("bullet-size");
    assert_eq!(s[0].list.as_ref().unwrap().bullet_size_pt, Some(20.0));
    assert_eq!(s[1].list.as_ref().unwrap().bullet_size_pt, Some(5.0));
    assert!(s[2].list.is_none());

    let i = b("bullet-indent");
    assert_eq!(i[0].list.as_ref().unwrap().list_indent_pt, 36.0);
    assert_eq!(i[0].indents, ind(36.0, -36.0, 0.0));
    assert_eq!(i[1].list.as_ref().unwrap().list_indent_pt, 0.0);
    assert_eq!(i[1].indents, ind(0.0, 0.0, 0.0));
}

#[kernpage_samples::test]
fn numbering_properties() {
    let n = paras("num-1dot");
    for p in &n {
        assert_eq!(p.list, Some(ListInfo::numbered(ListType::Arabic)));
        assert_eq!(p.indents, ind(18.0, -18.0, 0.0));
    }
    // Absent 0x258 is the "plain" separator (index 3), absent 0x215 is start 1.
    let seps: Vec<u8> = paras("num-seps").iter().map(|p| p.list.as_ref().unwrap().separator).collect();
    assert_eq!(seps, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    let kinds: Vec<u32> = paras("num-types").iter().map(|p| p.list.as_ref().unwrap().kind.code()).collect();
    assert_eq!(kinds, [0, 1, 2, 3, 4, 5, 6, 7, 22]);
    let starts: Vec<u32> = paras("num-start-p2").iter().map(|p| p.list.as_ref().unwrap().start).collect();
    assert_eq!(starts, [1, 7, 7]);
    assert_eq!(paras("num-indent36")[0].list.as_ref().unwrap().list_indent_pt, 36.0);
    // Switching a bullet to numbering drops the bullet font; the leftover size is ignored.
    let f = paras("bullet-font-then-num");
    let l = f[1].list.as_ref().unwrap();
    assert_eq!((l.kind, l.bullet_font.as_str(), l.bullet_size_pt), (ListType::Arabic, "", None));
    // An empty list paragraph has a full record and is numbered.
    assert_eq!(paras("num-empty")[1].list.as_ref().unwrap().kind, ListType::Arabic);
}

#[kernpage_samples::test]
fn removed_list_leaves_residue_the_reader_ignores() {
    for p in paras("bullet-then-none") {
        assert!(p.list.is_none());
        assert_eq!(p.indents, ParaIndents::default());
    }
}

#[kernpage_samples::test]
fn plain_indents_without_a_list() {
    let p = paras("indent-plain");
    assert!(p.iter().all(|p| p.list.is_none()));
    assert_eq!(p[0].indents, ind(36.0, -18.0, 0.0));
    assert_eq!(p[1].indents, ind(18.0, 18.0, 0.0));
    assert_eq!(p[2].indents, ind(0.0, 0.0, 18.0));
}
