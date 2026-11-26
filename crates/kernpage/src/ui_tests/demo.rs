//! `--demo`: the screenshots show invented data only (see `crate::demo`).

use super::support::*;
use egui_kittest::kittest::Queryable;

fn rows(ui: &TestUi, prefix: &str) -> Vec<String> {
    let mut v: Vec<_> = ui.ribbon_buttons().into_iter().filter(|b| b.label.starts_with(prefix)).collect();
    v.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y).then(a.rect.min.x.total_cmp(&b.rect.min.x)));
    v.into_iter().map(|b| b.label[prefix.len()..].to_owned()).collect()
}

/// The real profile strings that must never be on screen.
fn real_strings() -> Vec<String> {
    ["USERPROFILE", "USERNAME", "HOME", "USER"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .filter(|v| v.len() > 3 && !v.eq_ignore_ascii_case("Demo"))
        .collect()
}

fn assert_clean(ui: &TestUi, page: &str) {
    for s in real_strings() {
        assert_eq!(ui.h.query_all_by_label_contains(&s).count(), 0, "{page} shows {s:?}");
        for b in ui.ribbon_buttons() {
            assert!(!b.label.contains(&s) && !b.tip.contains(&s), "{page}: {:?} contains {s:?}", b.label);
        }
    }
}

#[test]
fn the_start_screen_and_open_page_list_only_the_invented_files() {
    let mut ui = TestUi::start_demo();
    let names = ["Riverbend Review - May.kern", "Spring Flyer.kern", "Bake Sale Poster.kern", "Sunday Bulletin.pub", "Garden Club Newsletter.kern"];
    assert_eq!(rows(&ui, "recent:"), names);
    assert_clean(&ui, "Home");
    ui.click_ribbon("nav:Open");
    assert_eq!(rows(&ui, "recent:"), names);
    assert_clean(&ui, "Open");
}

#[test]
fn save_as_lists_only_the_invented_folders() {
    let mut ui = TestUi::start_demo();
    ui.app().new_document(letter_doc(vec![]), None, None);
    ui.settle();
    ui.click_tab("File");
    ui.click_ribbon("nav:Save As");
    assert_eq!(rows(&ui, "folder:"), ["Church Newsletter", "Community Flyers", "Documents", "Desktop"]);
    assert_clean(&ui, "Save As");
}

#[test]
fn the_info_page_shows_a_demo_path_and_print_offers_only_the_office_printer() {
    let mut ui = TestUi::start_demo();
    ui.app().open_path(&sample("kern/hello.kern"));
    ui.settle();
    assert_eq!(ui.app().recent.items().len(), 5, "opening a file does not change the demo list");
    ui.click_tab("File");
    ui.click_ribbon("nav:Info");
    assert_clean(&ui, "Info");
    assert!(ui.h.query_all_by_label_contains("Church Newsletter").count() > 0, "the demo folder");
    ui.click_ribbon("nav:Print");
    let print = &ui.app().print;
    assert_eq!(print.printer.as_deref(), Some("Office Printer"));
    let list = print.printers.as_ref().expect("the list was asked for");
    assert_eq!(list.names, ["Office Printer"]);
    assert_clean(&ui, "Print");
}

#[test]
fn demo_mode_leaves_the_real_recent_list_alone() {
    let mut ui = TestUi::start_demo();
    ui.app().open_path(&sample("kern/hello.kern"));
    ui.settle();
    assert!(!ui.env.recent.exists(), "nothing was written to the recent file");
}
