//! Master pages, page-number fields and sections (spec 4T), read from Publisher-made samples.

use pub_format::{
    page_number_in, page_number_text, read, Document, Field, NumberFormat, Section,
};
use std::path::PathBuf;

fn load(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/gen")
        .join(format!("{name}.pub"));
    read(&kernpage_samples::resolve(&path)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn first_text(doc: &Document, master: usize) -> String {
    doc.masters[master].shapes[0].text.clone().unwrap_or_default()
}

#[kernpage_samples::test]
fn master_box_is_on_the_master_not_on_the_pages() {
    let doc = load("master-box-1p");
    assert_eq!(doc.masters.len(), 1);
    assert_eq!(doc.masters[0].name, "Master Page A");
    assert_eq!(doc.masters[0].letter, "A");
    assert_eq!(doc.masters[0].shapes.len(), 1);
    assert!(first_text(&doc, 0).contains("MASTERTEXT"));
    assert_eq!(doc.pages.len(), 1);
    assert!(doc.pages[0].shapes.is_empty());
    assert_eq!(doc.pages[0].master, Some(0));
    assert!(!doc.pages[0].ignore_master);
    // 216 x 36 pt at 72, 72.
    let s = &doc.masters[0].shapes[0];
    assert_eq!(s.right_emu - s.left_emu, 2_743_200);
    assert_eq!(s.bottom_emu - s.top_emu, 457_200);
}

#[kernpage_samples::test]
fn a_box_on_the_page_is_not_a_master_shape() {
    let doc = load("master-pn-boxpage-1p");
    assert!(doc.masters[0].shapes.is_empty());
    assert_eq!(doc.pages[0].shapes.len(), 1);
}

#[kernpage_samples::test]
fn two_pages_share_the_master_and_ignore_flag_is_per_page() {
    let doc = load("master-box-2p");
    assert_eq!(doc.pages.len(), 2);
    assert!(doc.pages.iter().all(|p| p.master == Some(0) && !p.ignore_master));
    let doc = load("master-box-2p-ignore");
    assert_eq!(doc.pages[0].ignore_master, false);
    assert_eq!(doc.pages[1].ignore_master, true);
    assert_eq!(doc.pages[1].master, Some(0));
}

#[kernpage_samples::test]
fn page_number_field_is_marked_from_tokn() {
    let doc = load("master-pn-1p");
    let s = &doc.masters[0].shapes[0];
    let fields: Vec<_> = s.runs.iter().filter(|r| r.field.is_some()).collect();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].text, "#");
    assert_eq!(fields[0].field, Some(Field::PageNumber));
    // The same story in a normal page box also has the field.
    let doc = load("master-pn-boxpage-1p");
    assert!(doc.pages[0].shapes[0].runs.iter().any(|r| r.field.is_some()));
}

#[kernpage_samples::test]
fn field_inside_longer_text_splits_the_run() {
    let doc = load("master-pn-text-2p");
    let s = &doc.masters[0].shapes[0];
    let texts: Vec<_> = s.runs.iter().map(|r| (r.text.as_str(), r.field.is_some())).collect();
    assert!(texts.contains(&("#", true)), "{texts:?}");
    assert!(texts.iter().any(|(t, f)| !f && t.starts_with("Page ")), "{texts:?}");
    // Substituting shows the page's number in place.
    let shown = s.with_page_number("2").unwrap();
    assert_eq!(shown.text.as_deref().map(|t| t.trim_end()), Some("Page 2"));
    // A shape without a field is not copied.
    assert!(load("master-box-1p").masters[0].shapes[0].with_page_number("1").is_none());
}

#[kernpage_samples::test]
fn default_numbering_is_arabic_from_one() {
    let doc = load("master-pn-2p");
    assert!(doc.sections.is_empty());
    assert_eq!(page_number_text(&doc, 0), "1");
    assert_eq!(page_number_text(&doc, 1), "2");
}

#[kernpage_samples::test]
fn section_start_five_shifts_every_page() {
    let doc = load("master-pn-start5");
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].start_page, 1);
    assert_eq!(doc.sections[0].first_number, Some(5));
    let shown: Vec<_> = (0..3).map(|i| page_number_text(&doc, i)).collect();
    assert_eq!(shown, ["5", "6", "7"]);
}

#[kernpage_samples::test]
fn number_formats_from_the_samples() {
    let doc = load("master-pnfmt-2");
    assert_eq!(doc.sections[0].format, NumberFormat::LowerRoman);
    assert_eq!(page_number_text(&doc, 2), "iii");
    let doc = load("master-pnfmt-3");
    assert_eq!(doc.sections[0].format, NumberFormat::UpperLetter);
    assert_eq!(page_number_text(&doc, 1), "B");
}

#[kernpage_samples::test]
fn second_section_restarts_with_its_own_format() {
    let doc = load("master-pn-section2-fmt");
    assert_eq!(doc.sections.len(), 2);
    assert_eq!(doc.sections[1].start_page, 2);
    assert_eq!(doc.sections[1].first_number, Some(1));
    let shown: Vec<_> = (0..3).map(|i| page_number_text(&doc, i)).collect();
    assert_eq!(shown, ["1", "i", "ii"]);
}

#[kernpage_samples::test]
fn formats_and_continuing_sections() {
    let f = NumberFormat::UpperRoman;
    assert_eq!((f.format(1), f.format(4), f.format(9), f.format(1994)), ("I".into(), "IV".into(), "IX".into(), "MCMXCIV".into()));
    assert_eq!(NumberFormat::LowerRoman.format(4), "iv");
    let l = NumberFormat::UpperLetter;
    assert_eq!((l.format(1), l.format(26), l.format(27), l.format(28)), ("A".into(), "Z".into(), "AA".into(), "BB".into()));
    assert_eq!(NumberFormat::LowerLetter.format(27), "aa");
    assert_eq!(NumberFormat::from_stored(5), NumberFormat::Arabic);
    // A continuing second section keeps counting.
    let sec = |start, first| Section {
        start_page: start,
        first_number: first,
        format: NumberFormat::Arabic,
        hide_first_page_hf: false,
    };
    let s = [sec(1, Some(5)), sec(3, None)];
    let shown: Vec<_> = (0..4).map(|i| page_number_in(&s, i)).collect();
    assert_eq!(shown, ["5", "6", "7", "8"]);
}
