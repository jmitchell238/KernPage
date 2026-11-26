//! Master pages (spec 4T): drawn behind the page, never selectable, page-number fields show
//! the number of the page being drawn.

use super::support::*;
use eframe::egui::epaint::Shape as Paint;

/// Every string painted in the last frame (text galleys, however nested).
fn drawn_text(ui: &TestUi) -> Vec<String> {
    fn walk(s: &Paint, out: &mut Vec<String>) {
        match s {
            Paint::Text(t) => out.push(t.galley.text().to_owned()),
            Paint::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for c in &ui.h.output().shapes {
        walk(&c.shape, &mut out);
    }
    out
}

fn open(name: &str) -> TestUi {
    let mut ui = TestUi::with_sample(name);
    ui.wait_for_fonts();
    ui
}

#[test]
fn master_shapes_are_drawn_on_the_page_but_cannot_be_selected() {
    let mut ui = open("gen/master-box-1p.pub");
    assert_eq!(ui.doc().masters.len(), 1);
    assert!(ui.shapes().is_empty(), "the page itself has no shapes");
    assert!(
        drawn_text(&ui).iter().any(|t| t.contains("MASTERTEXT")),
        "{:?}",
        drawn_text(&ui)
    );
    // Click and double-click in the middle of the master box (72, 72, 216 x 36 pt).
    let p = ui.at_in(1.0 + 1.5, 1.0 + 0.25);
    ui.click_at(p);
    assert!(ui.selected().is_empty(), "master shapes are not selectable");
    ui.double_click_at(p);
    assert!(ui.selected().is_empty());
    assert!(ui.app().edit.is_none(), "master text is not editable here");
    ui.key(Default::default(), eframe::egui::Key::Delete);
    assert_eq!(ui.doc().masters[0].shapes.len(), 1, "nothing was deleted");
}

#[test]
fn ignore_master_page_hides_the_master_shapes() {
    let mut ui = open("gen/master-box-2p-ignore.pub");
    // The page-navigation thumbnails draw text too: page 1 (canvas + its thumbnail) shows the
    // master twice, page 2 only through page 1's thumbnail.
    let count = |ui: &TestUi| drawn_text(ui).iter().filter(|t| t.contains("MASTERTEXT")).count();
    let on_page_1 = count(&ui);
    assert!(on_page_1 >= 1);
    ui.app().go_page(1);
    ui.settle();
    assert_eq!(count(&ui), on_page_1 - 1, "page 2 ignores the master");
}

#[test]
fn page_number_field_shows_the_number_of_the_page_drawn() {
    let mut ui = open("gen/master-pn-text-2p.pub");
    // Each page also shows in a navigation thumbnail, so the page being edited is drawn twice
    // (canvas + thumbnail) and the other page once.
    let n = |ui: &TestUi, want: &str| drawn_text(ui).iter().filter(|t| t.contains(want)).count();
    assert_eq!((n(&ui, "Page 1"), n(&ui, "Page 2")), (2, 1), "{:?}", drawn_text(&ui));
    assert_eq!(n(&ui, "#"), 0, "the field is not drawn as #");
    ui.app().go_page(1);
    ui.settle();
    assert_eq!((n(&ui, "Page 1"), n(&ui, "Page 2")), (1, 2), "{:?}", drawn_text(&ui));
    // The document still stores the literal # (the field marker), not the number.
    let kernpage_core::ShapeKind::TextBox { paragraphs } = &ui.doc().masters[0].shapes[0].kind else {
        panic!("master shape is not a text box");
    };
    let stored: String = paragraphs.iter().map(|p| p.text()).collect();
    assert!(stored.contains('#'), "{stored}");
}

#[test]
fn section_start_number_is_drawn_in_its_format() {
    let ui = open("gen/master-pn-2p.pub");
    let mut doc = ui.doc().clone();
    doc.sections = vec![kernpage_core::Section {
        start_page: 1,
        first_number: Some(3),
        format: kernpage_core::NumberFormat::UpperRoman,
        hide_first_page_hf: false,
    }];
    let mut ui2 = TestUi::with_doc(doc);
    ui2.wait_for_fonts();
    assert!(drawn_text(&ui2).iter().any(|t| t == "III"), "{:?}", drawn_text(&ui2));
    ui2.app().go_page(1);
    ui2.settle();
    assert!(drawn_text(&ui2).iter().any(|t| t == "IV"), "{:?}", drawn_text(&ui2));
}
