//! Documents and prompts: Open, Save, Save As, Export PDF, Insert / Change / Remove Picture,
//! the unsaved-changes and unsupported-changes questions, the window title and the recent list.
//!
//! The native file dialogs are replaced by `support::Dialogs` (scripted answers; an unscripted
//! dialog panics), so nothing here ever shows a real dialog or touches the real %APPDATA%.

use super::support::*;
use crate::app::{Pending, Prompt, Section};
use crate::filedialog::{Filter, OpenRequest, SaveRequest};
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use kernpage_core::geometry::in_to_emu;
use kernpage_core::{Document, Page, ShapeKind};
use std::path::{Path, PathBuf};

const NONE: Modifiers = Modifiers::NONE;
const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: true,
};

const A: u64 = 1;

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/assets")
        .join(name)
}

/// A new, never-saved document "Test" holding two rectangles.
fn new_doc() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
    ]))
}

/// Make an edit through the UI (select A, nudge it right): the publication is now dirty.
fn nudge(ui: &mut TestUi) {
    ui.click_at(ui.center_of(A));
    ui.key(NONE, Key::ArrowRight);
}

/// A copy of a sample in the test's own folder (so saving never touches the repository).
fn copy_sample(ui: &TestUi, name: &str, as_name: &str) -> PathBuf {
    let to = ui.env.files.join(as_name);
    std::fs::copy(sample(name), &to).unwrap();
    to
}

fn open_sample_copy(name: &str) -> (TestUi, PathBuf) {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, name, "work.pub");
    ui.app().open_path(&copy);
    assert!(ui.app().error.is_none());
    ui.settle();
    (ui, copy)
}

/// File > <name>: the File tab, then the entry in the left column (real pointer clicks).
fn file_entry(ui: &mut TestUi, name: &str) {
    if !ui.h.state().backstage {
        ui.click_tab("File");
    }
    ui.click_ribbon(&format!("nav:{name}"));
}

fn click_button(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn is_pdf_file(p: &Path) -> bool {
    std::fs::read(p).is_ok_and(|b| b.starts_with(b"%PDF"))
}

fn is_pub_file(p: &Path) -> bool {
    // A Publisher file is an OLE compound document.
    std::fs::read(p).is_ok_and(|b| b.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]))
}

fn title(ui: &mut TestUi) -> String {
    ui.app().window_title().to_owned()
}

fn is_kern_file(p: &Path) -> bool {
    std::fs::read(p).is_ok_and(|b| kernpage_core::native::is_kern(&b))
}

fn open_filters() -> Vec<Filter> {
    vec![
        Filter::new("All KernPage and Publisher files (*.kern;*.pub)", &["kern", "pub"]),
        Filter::new("KernPage files (*.kern)", &["kern"]),
        Filter::new("Publisher files (*.pub)", &["pub"]),
        Filter::new("All files", &["*"]),
    ]
}

fn error_text(ui: &mut TestUi) -> String {
    ui.app().error.clone().unwrap_or_default()
}

// ---- Open ----------------------------------------------------------------------------------

#[test]
fn ctrl_o_asks_for_a_publication_file_and_opens_the_picked_one() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "boxes.pub");
    ui.dialogs.script_open(Some(copy.clone()));
    ui.key(CTRL, Key::O);
    assert_eq!(
        ui.dialogs.opens(),
        vec![OpenRequest {
            title: "Open Publication".into(),
            filters: open_filters(),
        }],
        "Open takes .kern and .pub files"
    );
    assert!(ui.app().editor.is_some(), "the file is open");
    assert!(!ui.app().backstage, "back in the editor");
    assert_eq!(ui.doc().name, "boxes.pub");
    assert_eq!(ui.app().path.as_deref(), Some(copy.as_path()));
    assert!(!ui.shapes().is_empty());
    assert!(!ui.app().is_dirty(), "a freshly opened file has no unsaved changes");
    assert!(ui.dialogs.all_used());
}

#[test]
fn cancelling_the_open_dialog_does_nothing() {
    let mut ui = new_doc();
    ui.dialogs.script_open(None);
    ui.key(CTRL, Key::O);
    assert_eq!(ui.dialogs.opens().len(), 1);
    assert_eq!(ui.doc().name, "Test", "the open publication stays");
    assert_eq!(ui.shapes().len(), 2);
    assert!(ui.app().error.is_none() && ui.app().prompt.is_none());
    assert!(!ui.app().backstage);
}

#[test]
fn cancelling_the_open_dialog_on_the_start_screen_stays_there() {
    let mut ui = TestUi::start();
    ui.dialogs.script_open(None);
    ui.key(CTRL, Key::O);
    assert!(ui.app().editor.is_none());
    assert!(ui.app().error.is_none());
}

#[test]
fn file_open_browse_opens_the_picked_file() {
    let mut ui = new_doc();
    let copy = copy_sample(&ui, "two-boxes.pub", "boxes.pub");
    file_entry(&mut ui, "Open");
    assert!(ui.h.state().section == Section::Open);
    ui.dialogs.script_open(Some(copy));
    click_button(&mut ui, "Browse");
    assert_eq!(ui.dialogs.opens().len(), 1);
    assert_eq!(ui.doc().name, "boxes.pub");
    assert!(!ui.app().backstage);
}

#[test]
fn opening_a_file_adds_it_to_the_recent_list_at_the_top() {
    let mut ui = TestUi::start();
    let a = copy_sample(&ui, "two-boxes.pub", "a.pub");
    let b = copy_sample(&ui, "hello.pub", "b.pub");
    assert!(ui.app().recent.items().is_empty());
    ui.app().open_path(&a);
    ui.app().open_path(&b);
    ui.app().open_path(&a);
    assert_eq!(ui.app().recent.items(), &[a.clone(), b.clone()], "newest first, no duplicates");
    let stored = std::fs::read_to_string(&ui.env.recent).unwrap();
    assert!(stored.lines().next().unwrap().split('\t').next().unwrap().ends_with("a.pub"), "persisted to the temp recent file");
}

#[test]
fn opening_a_missing_file_shows_an_error_and_keeps_the_document() {
    let mut ui = new_doc();
    ui.dialogs.script_open(Some(ui.env.files.join("nowhere.pub")));
    ui.key(CTRL, Key::O);
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("KernPage cannot open \"nowhere.pub\""), "{msg}");
    assert_eq!(ui.doc().name, "Test");
    // The error dialog is modal: OK closes it.
    click_button(&mut ui, "  OK  ");
    assert!(ui.app().error.is_none());
    assert!(ui.app().recent.items().is_empty(), "a failed open is not remembered");
}

#[test]
fn opening_a_damaged_file_shows_an_error_and_keeps_the_document() {
    let mut ui = new_doc();
    let bad = ui.env.files.join("damaged.pub");
    std::fs::write(&bad, b"this is not a publication").unwrap();
    ui.dialogs.script_open(Some(bad));
    ui.key(CTRL, Key::O);
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("KernPage cannot open \"damaged.pub\""), "{msg}");
    assert_eq!(ui.doc().name, "Test");
    assert!(ui.app().recent.items().is_empty());
}

// ---- Save / Save As --------------------------------------------------------------------------

#[test]
fn save_of_a_never_saved_publication_is_save_as_with_kern_first_then_pub_and_pdf() {
    let mut ui = new_doc();
    let target = ui.env.files.join("My Flyer.kern");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert_eq!(
        ui.dialogs.saves(),
        vec![SaveRequest {
            title: "Save As".into(),
            default_dir: None,
            default_name: "Test.kern".into(),
            filters: vec![
                Filter::new("KernPage file (*.kern)", &["kern"]),
                Filter::new("Publisher file (*.pub)", &["pub"]),
                Filter::new("PDF (*.pdf)", &["pdf"]),
            ],
        }]
    );
    assert!(is_kern_file(&target), "a .kern file was written");
    assert_eq!(ui.doc().name, "My Flyer.kern", "the publication takes the file's name");
    assert_eq!(ui.app().path.as_deref(), Some(target.as_path()));
    assert!(!ui.app().is_dirty());
    assert!(ui.app().error.is_none());
    assert_eq!(ui.app().recent.items(), &[target]);
}

#[test]
fn cancelling_the_first_save_writes_nothing_and_stays_unsaved() {
    let mut ui = new_doc();
    nudge(&mut ui);
    ui.dialogs.script_save(None);
    ui.key(CTRL, Key::S);
    assert!(std::fs::read_dir(&ui.env.files).unwrap().next().is_none(), "no file written");
    assert!(ui.app().path.is_none());
    assert!(ui.app().is_dirty(), "still unsaved");
    assert!(ui.app().error.is_none());
    // And a second try works: the first save was not half done.
    let target = ui.env.files.join("again.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert!(is_pub_file(&target));
    assert!(!ui.app().is_dirty());
}

#[test]
fn save_as_offers_the_current_file_name_and_folder() {
    let (mut ui, copy) = open_sample_copy("two-boxes.pub");
    let other = ui.env.files.join("other.pub");
    ui.dialogs.script_save(Some(other.clone()));
    ui.key(CTRL_SHIFT, Key::S);
    let saves = ui.dialogs.saves();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].title, "Save As");
    assert_eq!(saves[0].default_name, "work.pub");
    assert_eq!(saves[0].default_dir.as_deref(), Some(ui.env.files.as_path()));
    assert_eq!(saves[0].filters[0], Filter::new("Publisher file (*.pub)", &["pub"]), "an opened .pub offers .pub first");
    assert_eq!(saves[0].filters[1], Filter::new("KernPage file (*.kern)", &["kern"]));
    assert_eq!(saves[0].filters[2], Filter::new("PDF (*.pdf)", &["pdf"]));
    assert!(is_pub_file(&other));
    assert_eq!(ui.app().path.as_deref(), Some(other.as_path()), "the publication moved to the new file");
    assert_eq!(ui.doc().name, "other.pub");
    assert!(copy.exists(), "the original stays");
}

#[test]
fn file_save_as_browse_runs_the_save_as_dialog() {
    let (mut ui, _) = open_sample_copy("two-boxes.pub");
    file_entry(&mut ui, "Save As");
    assert!(ui.h.state().section == Section::SaveAs);
    let target = ui.env.files.join("viabrowse.pub");
    ui.dialogs.script_save(Some(target.clone()));
    click_button(&mut ui, "Browse");
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert!(is_pub_file(&target));
}

#[test]
fn cancelling_save_as_changes_nothing() {
    let (mut ui, copy) = open_sample_copy("two-boxes.pub");
    ui.dialogs.script_save(None);
    ui.key(CTRL_SHIFT, Key::S);
    assert_eq!(ui.app().path.as_deref(), Some(copy.as_path()));
    assert_eq!(ui.doc().name, "work.pub");
    assert!(ui.app().error.is_none());
}

#[test]
fn a_name_typed_without_an_extension_gets_dot_kern() {
    let mut ui = new_doc();
    ui.dialogs.script_save(Some(ui.env.files.join("plain")));
    ui.key(CTRL, Key::S);
    let saved = ui.env.files.join("plain.kern");
    assert!(is_kern_file(&saved), "plain.kern written");
    assert!(!ui.env.files.join("plain").exists());
    assert_eq!(ui.app().path.as_deref(), Some(saved.as_path()));
}

#[test]
fn saving_as_pdf_exports_a_pdf_and_leaves_the_publication_unsaved() {
    let mut ui = new_doc();
    nudge(&mut ui);
    let pdf = ui.env.files.join("flyer.pdf");
    ui.dialogs.script_save(Some(pdf.clone()));
    ui.key(CTRL, Key::S);
    assert!(is_pdf_file(&pdf), "a PDF was written");
    assert!(ui.app().path.is_none(), "exporting is not saving");
    assert!(ui.app().is_dirty(), "the publication still has unsaved changes");
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("The PDF \"flyer.pdf\" was created."), "{msg}");
}

#[test]
fn a_saved_publication_reopens_with_the_same_content() {
    let mut text_box = kernpage_core::Shape::new(
        kernpage_core::Rect::new(in_to_emu(1.0), in_to_emu(4.0), in_to_emu(3.0), in_to_emu(1.0)),
        ShapeKind::TextBox {
            paragraphs: vec![kernpage_core::Paragraph::plain("Hello bulletin")],
        },
    );
    text_box.fill = None;
    let mut ui = TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.5), text_box]));
    let target = ui.env.files.join("roundtrip.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert!(ui.app().error.is_none());
    let before: Vec<_> = ui.shapes().iter().map(|s| (s.rect, std::mem::discriminant(&s.kind))).collect();

    let mut again = TestUi::start();
    again.app().open_path(&target);
    assert!(again.app().error.is_none(), "{:?}", again.app().error);
    again.settle();
    let after: Vec<_> = again.shapes().iter().map(|s| (s.rect, std::mem::discriminant(&s.kind))).collect();
    assert_eq!(after.len(), before.len(), "same number of objects");
    for (a, b) in after.iter().zip(&before) {
        assert_eq!(a.1, b.1, "same kind");
        for (x, y) in [(a.0.x, b.0.x), (a.0.y, b.0.y), (a.0.w, b.0.w), (a.0.h, b.0.h)] {
            assert!((x - y).abs() < in_to_emu(0.01), "rectangle {:?} vs {:?}", a.0, b.0);
        }
    }
    let text = again
        .shapes()
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::TextBox { paragraphs } => {
                Some(paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>().join("\n"))
            }
            _ => None,
        })
        .expect("the text box survived");
    assert_eq!(text, "Hello bulletin");
    assert_eq!(again.page_size(0), ui.page_size(0));
}

#[test]
fn ctrl_s_on_an_unchanged_file_writes_nothing() {
    let (mut ui, copy) = open_sample_copy("two-boxes.pub");
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    assert!(ui.dialogs.saves().is_empty());
}

// ---- .kern ---------------------------------------------------------------------------------

#[test]
fn save_as_choosing_pub_writes_a_pub() {
    let mut ui = new_doc();
    let target = ui.env.files.join("flyer.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert!(is_pub_file(&target) && !is_kern_file(&target));
    assert!(!ui.app().kern, "the document is now a .pub");
    assert!(!ui.app().is_dirty());
    // Plain Save keeps writing the .pub without asking again.
    nudge(&mut ui);
    ui.key(CTRL, Key::S);
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert!(is_pub_file(&target) && !ui.app().is_dirty());
}

#[test]
fn an_opened_pub_saves_back_to_pub_without_asking() {
    let (mut ui, copy) = open_sample_copy("two-boxes.pub");
    assert!(!ui.app().kern);
    let before = std::fs::read(&copy).unwrap();
    nudge(&mut ui);
    ui.key(CTRL, Key::S);
    assert!(ui.dialogs.saves().is_empty(), "no Save As dialog");
    assert!(is_pub_file(&copy) && !is_kern_file(&copy));
    assert_ne!(std::fs::read(&copy).unwrap(), before);
    assert!(!ui.app().is_dirty());
}

#[test]
fn an_opened_kern_saves_back_to_kern_and_tracks_changes() {
    let mut ui = new_doc();
    let target = ui.env.files.join("keep.kern");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert!(is_kern_file(&target));
    assert!(ui.app().source.is_none(), "a .kern document has no .pub source");
    // A fresh session opens it.
    let mut ui = TestUi::start();
    let target2 = ui.env.files.join("opened.kern");
    std::fs::copy(&target, &target2).unwrap();
    ui.app().open_path(&target2);
    ui.settle();
    assert!(ui.app().error.is_none());
    assert!(ui.app().kern && ui.app().source.is_none());
    assert_eq!(ui.shapes().len(), 2);
    assert_eq!(ui.doc().name, "opened.kern");
    assert!(!ui.app().is_dirty());
    assert_eq!(ui.app().recent.items(), &[target2.clone()]);
    nudge(&mut ui);
    assert!(ui.app().is_dirty());
    assert!(ui.app().window_title().contains('*'));
    let before = std::fs::read(&target2).unwrap();
    ui.key(CTRL, Key::S);
    assert!(ui.dialogs.saves().is_empty(), "no Save As dialog");
    assert!(is_kern_file(&target2));
    assert_ne!(std::fs::read(&target2).unwrap(), before);
    assert!(!ui.app().is_dirty(), "clean after the save");
    let back = kernpage_core::native::read_kern(&std::fs::read(&target2).unwrap()).unwrap();
    assert_eq!(back.pages[0].shapes.len(), 2);
    // Save As to a .pub switches the document over to the Publisher writer.
    let pubf = ui.env.files.join("now.pub");
    ui.dialogs.script_save(Some(pubf.clone()));
    ui.key(CTRL_SHIFT, Key::S);
    assert!(is_pub_file(&pubf) && !ui.app().kern);
}

#[test]
fn the_start_screen_previews_load_a_kern_file() {
    let mut ui = new_doc();
    let target = ui.env.files.join("preview.kern");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    let doc = crate::app::load_document(&target).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 2);
    assert!(crate::app::load_document(&ui.env.files).is_err());
}

#[test]
fn opening_a_damaged_kern_file_shows_an_error() {
    let mut ui = new_doc();
    let target = ui.env.files.join("bad.kern");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    let mut bytes = std::fs::read(&target).unwrap();
    bytes.truncate(bytes.len() / 2);
    std::fs::write(&target, bytes).unwrap();
    ui.app().open_path(&target);
    assert!(error_text(&mut ui).contains("bad.kern"));
}

#[test]
fn the_unsupported_notice_offers_a_kern_copy() {
    let (mut ui, copy) = with_unsupported_change();
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    ui.h.get_by_label("  Save a KernPage copy (.kern)  ");
    let target = ui.env.files.join("copy.kern");
    ui.dialogs.script_save(Some(target.clone()));
    click_button(&mut ui, "  Save a KernPage copy (.kern)  ");
    let saves = ui.dialogs.saves();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].default_name, "work.kern");
    assert_eq!(saves[0].filters[0], Filter::new("KernPage file (*.kern)", &["kern"]), ".kern preselected");
    assert!(ui.app().prompt.is_none() && ui.app().error.is_none());
    assert!(is_kern_file(&target));
    let kern = kernpage_core::native::read_kern(&std::fs::read(&target).unwrap()).unwrap();
    assert_eq!(kern.pages[0].shapes.len(), 1, "the deletion is kept in the .kern copy");
    assert_eq!(std::fs::read(&copy).unwrap(), before, "the .pub was not touched");
    assert!(ui.app().kern && ui.app().path.as_deref() == Some(target.as_path()));
    assert!(!ui.app().is_dirty());
}

#[test]
fn cancelling_the_kern_copy_dialog_returns_to_the_notice() {
    let (mut ui, copy) = with_unsupported_change();
    ui.key(CTRL, Key::S);
    ui.dialogs.script_save(None);
    click_button(&mut ui, "  Save a KernPage copy (.kern)  ");
    assert!(matches!(ui.app().prompt, Some(Prompt::Unsupported { .. })));
    assert!(ui.app().is_dirty());
    assert_eq!(ui.app().path.as_deref(), Some(copy.as_path()));
}

// ---- unsupported changes prompt ---------------------------------------------------------------

/// A copy of two-boxes.pub with its first box deleted: KernPage cannot write deletions yet.
fn with_unsupported_change() -> (TestUi, PathBuf) {
    let (mut ui, copy) = open_sample_copy("two-boxes.pub");
    let first = ui.shapes()[0].id;
    ui.click_at(ui.center_of(first));
    ui.key(NONE, Key::Delete);
    assert_eq!(ui.shapes().len(), 1);
    (ui, copy)
}

#[test]
fn saving_changes_that_cannot_be_written_asks_first() {
    let (mut ui, copy) = with_unsupported_change();
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    match &ui.h.state().prompt {
        Some(Prompt::Unsupported { items, target, .. }) => {
            assert!(items.iter().any(|i| i.contains("deleted objects")), "{items:?}");
            assert_eq!(target, &copy);
        }
        _ => panic!("the unsupported-changes prompt is open"),
    }
    assert_eq!(std::fs::read(&copy).unwrap(), before, "nothing written before the answer");
    ui.h.get_by_label("KernPage can't save these changes into a .pub file yet:");
    ui.h.get_by_label("   \u{2022} deleted objects");
    ui.h.get_by_label("  Cancel  ");
    ui.h.get_by_label("  Save supported changes  ");
}

#[test]
fn cancel_on_the_unsupported_prompt_writes_nothing() {
    let (mut ui, copy) = with_unsupported_change();
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    click_button(&mut ui, "  Cancel  ");
    assert!(ui.app().prompt.is_none());
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    assert!(ui.app().is_dirty(), "still unsaved");
}

#[test]
fn escape_on_the_unsupported_prompt_cancels_it() {
    let (mut ui, copy) = with_unsupported_change();
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    ui.key(NONE, Key::Escape);
    assert!(ui.app().prompt.is_none());
    assert_eq!(std::fs::read(&copy).unwrap(), before);
}

#[test]
fn save_supported_changes_writes_the_file() {
    let (mut ui, copy) = with_unsupported_change();
    // Also make a supported change: move the remaining box.
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(SHIFT, Key::ArrowRight);
    let before = std::fs::read(&copy).unwrap();
    ui.key(CTRL, Key::S);
    click_button(&mut ui, "  Save supported changes  ");
    assert!(ui.app().prompt.is_none() && ui.app().error.is_none());
    assert_ne!(std::fs::read(&copy).unwrap(), before, "the file changed");
    assert!(is_pub_file(&copy));
}

#[test]
fn the_unsupported_prompt_blocks_the_keyboard_shortcuts() {
    let (mut ui, _) = with_unsupported_change();
    ui.key(CTRL, Key::S);
    let n = ui.shapes().len();
    ui.key(CTRL, Key::A);
    ui.key(NONE, Key::Delete);
    assert_eq!(ui.shapes().len(), n, "Delete did nothing behind the prompt");
    assert!(ui.app().prompt.is_some());
}

// ---- the unsaved-changes question ---------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Trigger {
    New,
    Open,
    Close,
    Exit,
}

fn close_request(ui: &mut TestUi) {
    ui.h
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    ui.h.step();
    ui.settle();
}

fn fire(ui: &mut TestUi, t: Trigger) {
    match t {
        Trigger::New => ui.app().request(Pending::New(Box::new(Document::new(
            "Fresh",
            vec![Page::blank(in_to_emu(8.5), in_to_emu(11.0))],
        )))),
        Trigger::Open => ui.key(CTRL, Key::O),
        Trigger::Close => file_entry(ui, "Close"),
        Trigger::Exit => close_request(ui),
    }
    ui.settle();
}

fn performed(ui: &mut TestUi, t: Trigger) -> bool {
    match t {
        Trigger::New => ui.app().editor.is_some() && ui.doc().name == "Fresh",
        Trigger::Open => ui.app().editor.is_some() && ui.doc().name == "opened.pub",
        Trigger::Close => ui.app().editor.is_none(),
        Trigger::Exit => ui.app().exit_allowed(),
    }
}

/// A dirty, never-saved "Test" publication, and the trigger fired: the question is showing.
fn asked(t: Trigger) -> TestUi {
    let mut ui = new_doc();
    nudge(&mut ui);
    if matches!(t, Trigger::Open) {
        let p = copy_sample(&ui, "two-boxes.pub", "opened.pub");
        ui.dialogs.script_open(Some(p));
    }
    fire(&mut ui, t);
    assert!(
        matches!(ui.h.state().prompt, Some(Prompt::Unsaved { .. })),
        "{t:?}: the Save / Don't Save / Cancel question is showing"
    );
    assert!(!performed(&mut ui, t), "{t:?}: nothing happened yet");
    assert!(ui.dialogs.opens().is_empty(), "{t:?}: no dialog before the answer");
    ui
}

fn cancel_keeps_everything(t: Trigger) {
    let mut ui = asked(t);
    click_button(&mut ui, "  Cancel  ");
    assert!(ui.app().prompt.is_none(), "{t:?}: the question closed");
    assert!(!performed(&mut ui, t), "{t:?}: Cancel does not go on");
    assert!(ui.app().editor.is_some() && ui.doc().name == "Test");
    assert!(ui.app().is_dirty(), "{t:?}: the changes are still there");
    assert!(ui.dialogs.opens().is_empty() && ui.dialogs.saves().is_empty());
}

fn dont_save_goes_on(t: Trigger) {
    let mut ui = asked(t);
    click_button(&mut ui, "  Don't Save  ");
    assert!(ui.app().prompt.is_none());
    assert!(performed(&mut ui, t), "{t:?}: Don't Save goes on");
    assert!(ui.dialogs.saves().is_empty(), "{t:?}: and does not offer to save");
    assert!(ui.dialogs.all_used());
    assert!(std::fs::read_dir(&ui.env.files)
        .unwrap()
        .all(|e| e.unwrap().file_name() == "opened.pub"));
}

fn save_writes_then_goes_on(t: Trigger) {
    let mut ui = asked(t);
    let target = ui.env.files.join("saved.pub");
    ui.dialogs.script_save(Some(target.clone()));
    click_button(&mut ui, "  Save  ");
    assert_eq!(ui.dialogs.saves().len(), 1, "{t:?}: never saved, so Save asks where");
    assert!(is_pub_file(&target), "{t:?}: the file was written");
    assert!(ui.app().prompt.is_none());
    assert!(performed(&mut ui, t), "{t:?}: and then it goes on");
    assert!(ui.dialogs.all_used());
}

fn save_then_cancelled_dialog_stays(t: Trigger) {
    let mut ui = asked(t);
    ui.dialogs.script_save(None);
    click_button(&mut ui, "  Save  ");
    assert!(!performed(&mut ui, t), "{t:?}: cancelling Save As cancels the whole action");
    assert!(ui.app().editor.is_some() && ui.doc().name == "Test");
    assert!(ui.app().is_dirty());
    assert!(ui.app().prompt.is_none());
    assert!(ui.dialogs.opens().is_empty());
}

#[test]
fn new_cancel() {
    cancel_keeps_everything(Trigger::New);
}
#[test]
fn new_dont_save() {
    dont_save_goes_on(Trigger::New);
}
#[test]
fn new_save() {
    save_writes_then_goes_on(Trigger::New);
}
#[test]
fn new_save_then_cancel_the_dialog() {
    save_then_cancelled_dialog_stays(Trigger::New);
}
#[test]
fn open_cancel() {
    cancel_keeps_everything(Trigger::Open);
}
#[test]
fn open_dont_save() {
    dont_save_goes_on(Trigger::Open);
}
#[test]
fn open_save() {
    save_writes_then_goes_on(Trigger::Open);
}
#[test]
fn open_save_then_cancel_the_dialog() {
    save_then_cancelled_dialog_stays(Trigger::Open);
}
#[test]
fn close_cancel() {
    cancel_keeps_everything(Trigger::Close);
}
#[test]
fn close_dont_save() {
    dont_save_goes_on(Trigger::Close);
}
#[test]
fn close_save() {
    save_writes_then_goes_on(Trigger::Close);
}
#[test]
fn close_save_then_cancel_the_dialog() {
    save_then_cancelled_dialog_stays(Trigger::Close);
}
#[test]
fn exit_cancel() {
    cancel_keeps_everything(Trigger::Exit);
}
#[test]
fn exit_dont_save() {
    dont_save_goes_on(Trigger::Exit);
}
#[test]
fn exit_save() {
    save_writes_then_goes_on(Trigger::Exit);
}
#[test]
fn exit_save_then_cancel_the_dialog() {
    save_then_cancelled_dialog_stays(Trigger::Exit);
}

#[test]
fn escape_on_the_unsaved_question_cancels_it() {
    let mut ui = asked(Trigger::Close);
    ui.key(NONE, Key::Escape);
    assert!(ui.app().prompt.is_none());
    assert!(ui.app().editor.is_some());
}

#[test]
fn nothing_is_asked_when_there_are_no_unsaved_changes() {
    let (mut ui, _) = open_sample_copy("two-boxes.pub");
    fire(&mut ui, Trigger::Close);
    assert!(ui.app().prompt.is_none());
    assert!(ui.app().editor.is_none(), "closed without a question");

    let (mut ui, _) = open_sample_copy("two-boxes.pub");
    close_request(&mut ui);
    assert!(ui.app().prompt.is_none(), "a clean publication closes the window straight away");
}

#[test]
fn a_saved_publication_is_dirty_again_after_an_edit_but_clean_after_undo_to_the_saved_state() {
    let (mut ui, _) = open_sample_copy("two-boxes.pub");
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(SHIFT, Key::ArrowRight);
    assert!(ui.app().is_dirty());
    ui.key(CTRL, Key::S);
    assert!(!ui.app().is_dirty(), "saved");
    ui.key(SHIFT, Key::ArrowRight);
    assert!(ui.app().is_dirty(), "edited again");
    ui.undo();
    assert!(!ui.app().is_dirty(), "undo back to the saved state is clean again");
}

// ---- window title ---------------------------------------------------------------------------------

#[test]
fn the_window_title_is_the_product_name_without_a_publication() {
    let mut ui = TestUi::start();
    assert_eq!(title(&mut ui), "KernPage");
}

#[test]
fn the_window_title_shows_a_star_while_there_are_unsaved_changes() {
    let mut ui = new_doc();
    assert_eq!(title(&mut ui), "Test - KernPage");
    nudge(&mut ui);
    assert_eq!(title(&mut ui), "Test* - KernPage");
    ui.dialogs.script_save(Some(ui.env.files.join("titled.pub")));
    ui.key(CTRL, Key::S);
    assert_eq!(title(&mut ui), "titled.pub - KernPage", "saved: new name, no star");
    ui.key(SHIFT, Key::ArrowRight);
    assert!(ui.app().is_dirty());
    assert_eq!(title(&mut ui), "titled.pub* - KernPage");
}

// ---- recent list ------------------------------------------------------------------------------------

#[test]
fn saving_adds_the_file_to_the_recent_list() {
    let mut ui = new_doc();
    let target = ui.env.files.join("recent-save.pub");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert_eq!(ui.app().recent.items(), &[target]);
    assert!(std::fs::read_to_string(&ui.env.recent).unwrap().contains("recent-save.pub"));
}

// ---- Insert Picture -------------------------------------------------------------------------------------

fn picture_filters() -> Vec<Filter> {
    vec![Filter::new("Pictures", &["png", "jpg", "jpeg"])]
}

/// The one picture on the page.
fn picture_id(ui: &TestUi) -> u64 {
    ui.shapes()
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Picture { .. }))
        .expect("a picture")
        .id
}

fn with_picture() -> TestUi {
    let mut ui = new_doc();
    ui.dialogs.script_open(Some(asset("red4x3.png")));
    ui.click_ribbon("Pictures");
    assert_eq!(ui.shapes().len(), 3);
    ui
}

#[test]
fn the_home_pictures_button_inserts_the_picked_picture_at_its_natural_size() {
    let mut ui = new_doc();
    let steps = ui.undo_steps();
    ui.dialogs.script_open(Some(asset("red4x3.png")));
    ui.click_ribbon("Pictures");
    assert_eq!(
        ui.dialogs.opens(),
        vec![OpenRequest { title: "Insert Picture".into(), filters: picture_filters() }]
    );
    assert_eq!(ui.shapes().len(), 3);
    let id = picture_id(&ui);
    assert_eq!(ui.selected(), vec![id], "the new picture is selected");
    // red4x3.png records no resolution: 4 x 3 pixels at the screen's resolution, centred.
    let dpi = ui.app().screen_dpi;
    let r = ui.rect_of(id);
    let (w, h) = ((4.0 / dpi * 914_400.0).round() as i64, (3.0 / dpi * 914_400.0).round() as i64);
    assert_eq!((r.w, r.h), (w, h), "natural size");
    let (pw, ph) = ui.page_size(0);
    assert_eq!((r.x, r.y), ((pw - w) / 2, (ph - h) / 2), "centred on the page");
    assert_eq!(ui.undo_steps(), steps + 1, "one undo step");
    ui.undo();
    assert_eq!(ui.shapes().len(), 2, "undoable");
    assert!(ui.dialogs.all_used());
}

#[test]
fn the_insert_tab_pictures_button_does_the_same() {
    let mut ui = new_doc();
    ui.click_tab("Insert");
    ui.dialogs.script_open(Some(asset("red4x3.png")));
    ui.click_ribbon("Pictures");
    assert_eq!(ui.dialogs.opens().len(), 1);
    assert_eq!(ui.shapes().len(), 3);
    assert!(matches!(ui.shape(picture_id(&ui)).kind, ShapeKind::Picture { .. }));
}

#[test]
fn cancelling_insert_picture_adds_nothing() {
    let mut ui = new_doc();
    ui.dialogs.script_open(None);
    ui.click_ribbon("Pictures");
    assert_eq!(ui.shapes().len(), 2);
    assert!(!ui.app().is_dirty());
    assert!(ui.app().error.is_none());
}

#[test]
fn a_file_that_is_not_a_picture_shows_an_error() {
    let mut ui = new_doc();
    let bad = ui.env.files.join("fake.png");
    std::fs::write(&bad, b"not an image at all").unwrap();
    ui.dialogs.script_open(Some(bad));
    ui.click_ribbon("Pictures");
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("KernPage cannot insert \"fake.png\""), "{msg}");
    assert_eq!(ui.shapes().len(), 2);
}

#[test]
fn an_unreadable_picture_file_shows_an_error() {
    let mut ui = new_doc();
    ui.dialogs.script_open(Some(ui.env.files.join("gone.png")));
    ui.click_ribbon("Pictures");
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("KernPage cannot read \"gone.png\""), "{msg}");
    assert_eq!(ui.shapes().len(), 2);
}

#[test]
fn a_jpeg_can_be_inserted_too() {
    let mut ui = new_doc();
    ui.dialogs.script_open(Some(asset("green8x6.jpg")));
    ui.click_ribbon("Pictures");
    assert_eq!(ui.shapes().len(), 3);
    assert!(matches!(
        &ui.shape(picture_id(&ui)).kind,
        ShapeKind::Picture { format: kernpage_core::PictureFormat::Jpeg, .. }
    ));
}

// ---- Change Picture / Remove Picture -----------------------------------------------------------------------

#[test]
fn change_picture_is_enabled_only_while_one_picture_is_selected() {
    let mut ui = with_picture();
    ui.click_tab("Picture Format");
    assert!(ui.ribbon("Change Picture").enabled, "a picture is selected");
    ui.click_at(ui.center_of(A));
    assert!(
        !ui.tab_names().contains(&"Picture Format".to_owned()),
        "the picture tab goes away with the selection"
    );
}

#[test]
fn the_change_picture_button_swaps_the_image_keeping_height_and_centre() {
    let mut ui = with_picture();
    let id = picture_id(&ui);
    let old = ui.rect_of(id);
    let steps = ui.undo_steps();
    ui.click_tab("Picture Format");
    ui.dialogs.script_open(Some(asset("blue5x5.png")));
    ui.click_ribbon("Change Picture");
    assert_eq!(ui.dialogs.opens().len(), 2, "insert + change");
    assert_eq!(
        ui.dialogs.opens()[1],
        OpenRequest { title: "Insert Picture".into(), filters: picture_filters() }
    );
    let new = ui.rect_of(id);
    assert_eq!(new.h, old.h, "same height");
    assert_eq!(new.w, new.h, "width follows the new picture's 1:1 proportions");
    assert!(((new.x + new.w / 2) - (old.x + old.w / 2)).abs() <= 1, "same centre");
    assert_eq!(new.y, old.y);
    assert_eq!(ui.undo_steps(), steps + 1);
    ui.undo();
    assert_eq!(ui.rect_of(id), old, "undoable");
}

#[test]
fn cancelling_change_picture_changes_nothing() {
    let mut ui = with_picture();
    let id = picture_id(&ui);
    let old = ui.rect_of(id);
    ui.click_tab("Picture Format");
    ui.dialogs.script_open(None);
    ui.click_ribbon("Change Picture");
    assert_eq!(ui.rect_of(id), old);
    assert!(ui.app().error.is_none());
}

#[test]
fn the_picture_format_pictures_button_inserts_another_picture() {
    let mut ui = with_picture();
    ui.click_tab("Picture Format");
    ui.dialogs.script_open(Some(asset("blue5x5.png")));
    ui.click_ribbon("Pictures...");
    assert_eq!(ui.shapes().len(), 4);
}

#[test]
fn the_change_picture_arrow_menu_has_change_and_remove() {
    let mut ui = with_picture();
    let id = picture_id(&ui);
    let old = ui.rect_of(id);
    ui.click_tab("Picture Format");
    ui.click_ribbon_arrow("Change Picture");
    ui.dialogs.script_open(Some(asset("blue5x5.png")));
    click_button(&mut ui, "Change Picture...");
    assert_ne!(ui.rect_of(id).w, old.w, "the menu's Change Picture... changed it");

    ui.click_ribbon_arrow("Change Picture");
    click_button(&mut ui, "Remove Picture");
    assert_eq!(ui.shapes().len(), 2, "Remove Picture deletes the picture");
    assert!(ui.shapes().iter().all(|s| !matches!(s.kind, ShapeKind::Picture { .. })));
    ui.undo();
    assert_eq!(ui.shapes().len(), 3, "undoable");
}

#[test]
fn right_clicking_a_picture_offers_the_change_picture_submenu() {
    let mut ui = with_picture();
    let id = picture_id(&ui);
    let old = ui.rect_of(id);
    ui.click_at(ui.center_of(A));
    ui.right_click_at(ui.center_of(id));
    assert_eq!(ui.selected(), vec![id], "right-click selects the picture");
    click_button(&mut ui, "Change Picture");
    ui.dialogs.script_open(Some(asset("blue5x5.png")));
    click_button(&mut ui, "Change Picture...");
    assert_eq!(ui.dialogs.opens().len(), 2);
    let new = ui.rect_of(id);
    assert_eq!(new.h, old.h);
    assert_eq!(new.w, new.h);
}

#[test]
fn right_click_remove_picture_deletes_it() {
    let mut ui = with_picture();
    let id = picture_id(&ui);
    ui.right_click_at(ui.center_of(id));
    click_button(&mut ui, "Change Picture");
    click_button(&mut ui, "Remove Picture");
    assert_eq!(ui.shapes().len(), 2);
    assert!(ui.dialogs.all_used());
}

#[test]
fn a_rectangle_has_no_change_picture_submenu() {
    let mut ui = new_doc();
    ui.right_click_at(ui.center_of(A));
    assert!(ui.h.query_by_label("Change Picture").is_none());
    assert!(ui.h.query_by_label("Delete Object").is_some());
}

// ---- Export PDF ---------------------------------------------------------------------------------------------

#[test]
fn export_create_pdf_asks_for_a_name_and_writes_the_pdf() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    assert!(ui.h.state().section == Section::Export);
    let pdf = ui.env.files.join("out.pdf");
    ui.dialogs.script_save(Some(pdf.clone()));
    ui.click_ribbon("export:Create PDF");
    assert_eq!(
        ui.dialogs.saves(),
        vec![SaveRequest {
            title: "Publish as PDF".into(),
            default_dir: None,
            default_name: "Test.pdf".into(),
            filters: vec![Filter::new("PDF (*.pdf)", &["pdf"])],
        }]
    );
    assert!(is_pdf_file(&pdf));
    let msg = error_text(&mut ui);
    assert!(msg.starts_with("The PDF \"out.pdf\" was created."), "{msg}");
    assert!(ui.app().path.is_none(), "exporting is not saving");
}

#[test]
fn export_pdf_for_a_saved_publication_starts_in_its_folder() {
    let (mut ui, _) = open_sample_copy("two-boxes.pub");
    file_entry(&mut ui, "Export");
    let pdf = ui.env.files.join("work.pdf");
    ui.dialogs.script_save(Some(pdf.clone()));
    ui.click_ribbon("export:Create PDF");
    let s = &ui.dialogs.saves()[0];
    assert_eq!(s.default_name, "work.pdf");
    assert_eq!(s.default_dir.as_deref(), Some(ui.env.files.as_path()));
    assert!(is_pdf_file(&pdf));
}

#[test]
fn a_pdf_name_without_an_extension_gets_dot_pdf() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    ui.dialogs.script_save(Some(ui.env.files.join("noext")));
    ui.click_ribbon("export:Create PDF");
    assert!(is_pdf_file(&ui.env.files.join("noext.pdf")));
}

#[test]
fn cancelling_export_pdf_writes_nothing() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    ui.dialogs.script_save(None);
    ui.click_ribbon("export:Create PDF");
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert!(std::fs::read_dir(&ui.env.files).unwrap().next().is_none());
    assert!(ui.app().error.is_none());
}

#[test]
fn an_unscripted_dialog_is_a_test_failure() {
    // The seam's safety net: a dialog nobody scripted panics instead of showing a real one.
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ui = TestUi::start();
        ui.key(CTRL, Key::O);
    }));
    assert!(r.is_err());
}
