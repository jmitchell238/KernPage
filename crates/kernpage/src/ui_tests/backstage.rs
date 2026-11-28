//! The File (Backstage) view: the command column, and every page (Home, New, Open, Info, Save As,
//! Print, Share, Export, About, Feedback, Options). Entries and tiles are found through the
//! ribbon seam (`nav:`, `tile:`, `recent:`, `tabrow:`, `link:`, `flat:` and a few more labels,
//! see `backstage::seen`); page text is found through AccessKit. Native dialogs are the
//! scripted fakes, and Explorer / the PDF viewer are recorded (`output::test_os`), never run.

use super::support::*;
use crate::app::{Prompt, Section};
use eframe::egui::{self, Key, Modifiers, PointerButton};
use egui_kittest::kittest::{NodeT, Queryable};
use kernpage_core::template;
use std::path::PathBuf;

const NONE: Modifiers = Modifiers::NONE;
const A: u64 = 1;

const ALL_ENTRIES: [&str; 12] = [
    "Home", "New", "Open", "Info", "Save", "Save As", "Print", "Share", "Export", "Close",
    "About KernPage", "Feedback",
];

fn new_doc() -> TestUi {
    TestUi::with_doc(letter_doc(vec![
        rect_shape(1.0, 1.0, 2.0, 1.5),
        rect_shape(4.0, 3.0, 2.0, 1.5),
    ]))
}

/// A document with `n` Letter pages.
fn pages_doc(n: usize) -> TestUi {
    let mut ui = new_doc();
    for _ in 1..n {
        ui.app().exec(kernpage_core::Command::InsertPage { after: 0 });
    }
    ui.app().go_page(0);
    ui.settle();
    ui
}

fn nav(ui: &mut TestUi, name: &str) {
    ui.click_ribbon(&format!("nav:{name}"));
}

/// File tab, then the entry.
fn file_entry(ui: &mut TestUi, name: &str) {
    if !ui.h.state().backstage {
        ui.click_tab("File");
    }
    nav(ui, name);
}

fn in_section(ui: &mut TestUi, s: Section) -> bool {
    ui.app().section == s
}

fn nav_enabled(ui: &TestUi, name: &str) -> bool {
    ui.ribbon(&format!("nav:{name}")).enabled
}

fn has_label(ui: &TestUi, text: &str) -> bool {
    ui.h.query_all_by_label(text).count() > 0
}

fn has_label_containing(ui: &TestUi, text: &str) -> bool {
    ui.h.query_all_by_label_contains(text).count() > 0
}

fn click_button(ui: &mut TestUi, label: &str) {
    ui.h.get_by_label(label).click();
    ui.settle();
}

fn flat(ui: &TestUi, name: &str) -> crate::widgets::probe::Seen {
    ui.ribbon(&format!("flat:{name}"))
}

/// A real press and release on `name`, returning without settling (so the click frame's
/// platform output can be read).
fn click_unsettled(ui: &mut TestUi, name: &str) {
    let at = ui.ribbon(name).rect.center();
    ui.mouse_to(at);
    ui.press(PointerButton::Primary, NONE);
    ui.release(PointerButton::Primary, NONE);
}

fn copy_sample(ui: &TestUi, name: &str, as_name: &str) -> PathBuf {
    let to = ui.env.files.join(as_name);
    std::fs::copy(sample(name), &to).unwrap();
    to
}

/// `n` real publication files a0.pub, a1.pub, ... in the test's folder, added to the recent list
/// so that a(n-1) is the newest.
fn recent_files(ui: &mut TestUi, n: usize) -> Vec<PathBuf> {
    let files: Vec<PathBuf> = (0..n)
        .map(|i| copy_sample(ui, "two-boxes.pub", &format!("a{i}.pub")))
        .collect();
    for f in &files {
        ui.app().recent.add(f);
    }
    ui.settle();
    files
}

fn recent_rows(ui: &TestUi) -> Vec<String> {
    let mut rows: Vec<_> = ui
        .ribbon_buttons()
        .into_iter()
        .filter(|b| b.label.starts_with("recent:"))
        .collect();
    rows.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    rows.into_iter().map(|b| b.label["recent:".len()..].to_owned()).collect()
}

fn error_text(ui: &mut TestUi) -> String {
    ui.app().error.clone().unwrap_or_default()
}

/// The text the last frame put on the (fake) clipboard.
fn copied(ui: &TestUi) -> Option<String> {
    ui.h.output().platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    })
}

// ---- the command column -------------------------------------------------------------------------

#[test]
fn without_a_publication_only_the_always_available_entries_are_enabled() {
    let mut ui = TestUi::start();
    assert!(ui.app().backstage, "the app starts in the File view");
    assert!(in_section(&mut ui, Section::Home));
    for n in ["Home", "New", "Open", "Share", "About KernPage", "Feedback", "Options"] {
        assert!(nav_enabled(&ui, n), "{n} is available without a publication");
    }
    for n in ["Info", "Save", "Save As", "Print", "Export", "Close"] {
        assert!(!nav_enabled(&ui, n), "{n} needs a publication");
    }
    assert!(!nav_enabled(&ui, "Back"), "no publication to go back to");
    assert!(ui.ribbon("nav:Home").selected, "Home is the current page");
    assert!(!ui.ribbon("nav:New").selected);
}

#[test]
fn with_a_publication_every_entry_and_the_back_arrow_are_enabled() {
    let mut ui = new_doc();
    assert!(!ui.app().backstage);
    ui.click_tab("File");
    assert!(ui.app().backstage, "the File tab opens the File view");
    assert!(in_section(&mut ui, Section::Info), "with a publication it opens on Info");
    for n in ALL_ENTRIES.into_iter().chain(["Options", "Back"]) {
        assert!(nav_enabled(&ui, n), "{n}");
    }
    assert!(ui.ribbon("nav:Info").selected);
    assert!(!ui.ribbon("nav:Home").selected);
}

#[test]
fn clicking_a_disabled_entry_does_nothing() {
    let mut ui = TestUi::start();
    for n in ["Info", "Save", "Save As", "Print", "Export", "Close", "Back"] {
        nav(&mut ui, n);
        assert!(ui.app().backstage, "{n}: still in the File view");
        assert!(in_section(&mut ui, Section::Home), "{n}: still on Home");
        assert!(ui.app().editor.is_none(), "{n}: no publication appeared");
        assert!(ui.app().prompt.is_none() && ui.app().error.is_none(), "{n}: no dialog");
    }
    assert!(ui.dialogs.saves().is_empty() && ui.dialogs.opens().is_empty());
}

#[test]
fn each_entry_opens_its_page_and_is_drawn_selected() {
    let mut ui = new_doc();
    ui.click_tab("File");
    for (entry, section, title) in [
        ("Home", Section::Home, "Recent"),
        ("New", Section::New, "New"),
        ("Open", Section::Open, "Open"),
        ("Info", Section::Info, "Info"),
        ("Save As", Section::SaveAs, "Save As"),
        ("Print", Section::Print, "Print"),
        ("Share", Section::Share, "Share"),
        ("Export", Section::Export, "Export"),
        ("About KernPage", Section::About, "About KernPage"),
        ("Feedback", Section::Feedback, "Feedback"),
        ("Options", Section::Options, "Options"),
    ] {
        nav(&mut ui, entry);
        assert!(in_section(&mut ui, section), "{entry} selects its page");
        assert!(ui.ribbon(&format!("nav:{entry}")).selected, "{entry} is drawn selected");
        let others = ui
            .ribbon_buttons()
            .iter()
            .filter(|b| b.label.starts_with("nav:") && b.selected)
            .count();
        assert_eq!(others, 1, "{entry}: exactly one entry is selected");
        assert!(has_label(&ui, title), "{entry}: the page title {title:?} is shown");
        assert!(ui.app().backstage, "{entry}: still in the File view");
    }
}

#[test]
fn the_share_page_lists_three_coming_soon_items() {
    let mut ui = new_doc();
    ui.click_tab("File");
    nav(&mut ui, "Share");
    assert!(has_label(&ui, "Share") && has_label(&ui, "Coming soon"));
    for t in [
        "Share a PDF of this publication with other apps on your PC (email, chat, nearby devices).",
        "Email as PDF with your email app.",
        "Copy as PDF.",
    ] {
        assert!(ui.h.get_by_label(t).accesskit_node().is_disabled(), "{t} is greyed");
    }
    assert!(!has_label_containing(&ui, "not available"));
    // It works without a publication, too.
    let mut ui = TestUi::start();
    nav(&mut ui, "Share");
    assert!(has_label(&ui, "Copy as PDF."));
}

#[test]
fn the_feedback_page_offers_the_two_github_issue_links() {
    let mut ui = new_doc();
    ui.click_tab("File");
    nav(&mut ui, "Feedback");
    assert!(has_label_containing(&ui, "Nothing is sent from the editor"));
    crate::feedback::test_os::take_opened();
    ui.click_ribbon("link:Report a Problem");
    ui.click_ribbon("link:Suggest an Idea");
    let opened = crate::feedback::test_os::take_opened();
    assert_eq!(opened.len(), 2, "{opened:?}");
    assert!(opened[0].contains("labels=bug") && opened[1].contains("labels=idea"), "{opened:?}");
    assert!(ui.app().backstage, "the File view stays open");
}

#[test]
fn the_back_arrow_returns_to_the_publication_untouched() {
    let mut ui = new_doc();
    ui.click_at(ui.center_of(A));
    let (rev, sel) = (ui.revision(), ui.selected());
    ui.click_tab("File");
    nav(&mut ui, "Export");
    nav(&mut ui, "Back");
    assert!(!ui.app().backstage, "back in the editor");
    assert!(ui.has_ribbon("Cut"), "the ribbon is showing again");
    assert_eq!(ui.revision(), rev);
    assert_eq!(ui.selected(), sel);
    assert!(ui.app().editor.is_some());
}

#[test]
fn escape_returns_to_the_publication_from_every_page() {
    let mut ui = new_doc();
    for entry in ["Info", "New", "Open", "Save As", "Print", "Share", "Export", "About KernPage", "Feedback", "Options"]
    {
        file_entry(&mut ui, entry);
        assert!(ui.app().backstage);
        ui.key(NONE, Key::Escape);
        assert!(!ui.app().backstage, "Esc leaves the File view from {entry}");
        assert!(ui.has_ribbon("Cut"), "{entry}: the ribbon is back");
    }
    assert!(ui.app().editor.is_some(), "and nothing was closed");
}

#[test]
fn escape_without_a_publication_stays_in_the_file_view() {
    let mut ui = TestUi::start();
    ui.key(NONE, Key::Escape);
    assert!(ui.app().backstage, "there is nothing to go back to");
    assert!(in_section(&mut ui, Section::Home));
}

#[test]
fn the_file_tab_opens_the_file_view_and_the_back_arrow_closes_it() {
    let mut ui = new_doc();
    ui.click_tab("File");
    assert!(ui.app().backstage);
    assert!(!ui.has_ribbon("tab:File"), "the tab row is hidden while the File screen is open");
    nav(&mut ui, "Back");
    assert!(!ui.app().backstage, "the back arrow goes back");
    assert!(ui.has_ribbon("tab:File"), "the tab row is restored");
    // Without a publication the File view cannot be left.
    let mut ui = TestUi::start();
    assert!(ui.app().backstage);
    assert!(in_section(&mut ui, Section::Home));
}

#[test]
fn the_tab_row_and_ribbon_are_hidden_in_the_file_view_and_restored_by_back_and_by_escape() {
    for by_escape in [false, true] {
        let mut ui = new_doc();
        for t in ["File", "Home", "Insert"] {
            assert!(ui.has_ribbon(&format!("tab:{t}")), "{t} tab before");
        }
        assert!(ui.has_ribbon("Cut"));
        ui.click_tab("File");
        for t in ["File", "Home", "Insert"] {
            assert!(!ui.has_ribbon(&format!("tab:{t}")), "{t} tab hidden in the File screen");
        }
        assert!(!ui.has_ribbon("Cut"), "no ribbon either");
        assert!(ui.has_ribbon("nav:Back"), "the back arrow is there");
        if by_escape {
            ui.key(NONE, Key::Escape);
        } else {
            nav(&mut ui, "Back");
        }
        assert!(!ui.app().backstage);
        for t in ["File", "Home", "Insert"] {
            assert!(ui.has_ribbon(&format!("tab:{t}")), "{t} tab restored");
        }
        assert!(ui.has_ribbon("Cut"), "the ribbon is restored");
    }
}

#[test]
fn the_ribbon_and_the_canvas_are_not_drawn_in_the_file_view() {
    let mut ui = new_doc();
    assert!(ui.has_ribbon("Cut"));
    ui.click_tab("File");
    assert!(!ui.has_ribbon("Cut"), "no Home ribbon behind the File view");
    assert!(!ui.has_ribbon("tab:Home"), "no ribbon tab is drawn");
}

#[test]
fn ctrl_n_and_ctrl_p_open_the_new_and_print_pages_from_anywhere() {
    let mut ui = new_doc();
    ui.key(CTRL, Key::P);
    assert!(ui.app().backstage && in_section(&mut ui, Section::Print));
    ui.key(CTRL, Key::N);
    assert!(in_section(&mut ui, Section::New));
    assert!(ui.ribbon("nav:New").selected);
}

// ---- Save, Save As, Close entries -------------------------------------------------------------

#[test]
fn the_save_entry_of_a_new_publication_asks_for_a_file_and_returns_to_the_editor() {
    let mut ui = new_doc();
    let target = ui.env.files.join("saved.pub");
    ui.dialogs.script_save(Some(target.clone()));
    file_entry(&mut ui, "Save");
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert_eq!(ui.dialogs.saves()[0].title, "Save As");
    assert!(target.is_file(), "the file was written");
    assert!(!ui.app().backstage, "saving returns to the publication");
    assert_eq!(ui.app().path.as_deref(), Some(target.as_path()));
}

#[test]
fn cancelling_the_save_entrys_dialog_stays_in_the_file_view() {
    let mut ui = new_doc();
    ui.dialogs.script_save(None);
    file_entry(&mut ui, "Save");
    assert!(ui.app().backstage, "still in the File view");
    assert!(ui.app().path.is_none());
    assert!(ui.dialogs.all_used());
}

#[test]
fn the_save_entry_writes_a_changed_saved_publication_without_asking() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "work.pub");
    ui.app().open_path(&copy);
    ui.settle();
    let before = std::fs::read(&copy).unwrap();
    let id = ui.shapes()[0].id;
    ui.click_at(ui.center_of(id));
    ui.key(NONE, Key::ArrowRight);
    file_entry(&mut ui, "Save");
    assert!(ui.dialogs.saves().is_empty(), "a saved publication is not asked for a name");
    assert_ne!(std::fs::read(&copy).unwrap(), before, "the change was written");
    assert!(!ui.app().backstage, "back in the editor");
}

#[test]
fn the_save_entry_of_an_unchanged_saved_publication_just_returns() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "work.pub");
    ui.app().open_path(&copy);
    ui.settle();
    let before = std::fs::read(&copy).unwrap();
    file_entry(&mut ui, "Save");
    assert!(ui.dialogs.saves().is_empty());
    assert_eq!(std::fs::read(&copy).unwrap(), before, "nothing was rewritten");
    assert!(!ui.app().backstage, "Save returns to the publication, like Publisher");
}

#[test]
fn the_save_as_page_offers_this_pc_and_browse() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Save As");
    assert!(ui.ribbon("place:This PC").selected, "This PC is the (only) place");
    assert!(flat(&ui, "Browse").enabled);
    assert!(has_label(&ui, "Recent Folders"));
    assert!(
        has_label_containing(&ui, "Folders of publications you open or save appear here"),
        "no folders yet: the hint is shown"
    );
}

#[test]
fn save_as_browse_asks_for_a_file_and_saves_there() {
    let mut ui = new_doc();
    let target = ui.env.files.join("browsed.pub");
    ui.dialogs.script_save(Some(target.clone()));
    file_entry(&mut ui, "Save As");
    ui.click_ribbon("flat:Browse");
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert!(target.is_file());
    assert!(!ui.app().backstage);
    assert_eq!(ui.app().path.as_deref(), Some(target.as_path()));
}

#[test]
fn save_as_lists_the_recent_folders_and_a_click_runs_the_save_dialog() {
    let mut ui = new_doc();
    let files = recent_files(&mut ui, 2);
    let folder = files[0].parent().unwrap().file_name().unwrap().to_string_lossy().into_owned();
    file_entry(&mut ui, "Save As");
    assert!(!has_label_containing(&ui, "Folders of publications you open"), "the hint is gone");
    let row = ui.ribbon(&format!("folder:{folder}"));
    ui.dialogs.script_save(None);
    ui.click_at(row.rect.center());
    assert_eq!(ui.dialogs.saves().len(), 1, "a recent folder row asks where to save");
    assert!(ui.app().backstage, "cancelled: still on the page");
    let folders = ui.ribbon_buttons().iter().filter(|b| b.label.starts_with("folder:")).count();
    assert_eq!(folders, 1, "both files are in one folder: listed once");
}

#[test]
fn the_close_entry_closes_a_clean_publication_and_shows_home() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Close");
    assert!(ui.app().editor.is_none(), "the publication is closed");
    assert!(ui.app().backstage && in_section(&mut ui, Section::Home));
    for n in ["Info", "Save", "Save As", "Print", "Export", "Close", "Back"] {
        assert!(!nav_enabled(&ui, n), "{n} is greyed again");
    }
}

#[test]
fn the_close_entry_asks_first_when_there_are_unsaved_changes() {
    let mut ui = new_doc();
    ui.click_at(ui.center_of(A));
    ui.key(NONE, Key::ArrowRight);
    file_entry(&mut ui, "Close");
    assert!(
        matches!(ui.app().prompt, Some(Prompt::Unsaved { .. })),
        "Save / Don't Save / Cancel"
    );
    assert!(ui.app().editor.is_some(), "not closed yet");
    click_button(&mut ui, "  Cancel  ");
    assert!(ui.app().editor.is_some() && ui.app().prompt.is_none());
    assert!(ui.app().backstage, "cancelling leaves the File view where it was");
}

// ---- Home page ----------------------------------------------------------------------------------

#[test]
fn the_home_page_shows_the_new_tiles_and_the_open_list() {
    let ui = TestUi::start();
    assert!(!has_label(&ui, "Welcome"), "no greeting under the logo");
    assert!(has_label(&ui, "New"), "the page heading");
    for (tile, on) in [
        ("Blank 8.5 x 11\"", true),
        ("Blank 11 x 8.5\"", true),
        ("More page sizes", false),
    ] {
        assert_eq!(ui.ribbon(&format!("tile:{tile}")).enabled, on, "{tile}");
    }
    assert!(!ui.has_ribbon("tile:Church Bulletin"), "templates are behind More templates");
    assert!(ui.ribbon("tabrow:Recent").selected && ui.ribbon("tabrow:Recent").enabled);
    assert!(ui.ribbon("tabrow:Pinned").enabled, "the Pinned tab is there");
    assert!(ui.ribbon("link:More templates").enabled);
    assert!(ui.ribbon("link:More publications").enabled);
    assert!(
        has_label_containing(&ui, "No recent publications yet"),
        "an empty recent list says so"
    );
}

#[test]
fn the_blank_portrait_tile_starts_a_new_letter_publication() {
    let mut ui = TestUi::start();
    ui.click_ribbon("tile:Blank 8.5 x 11\"");
    assert!(ui.app().editor.is_some());
    assert!(!ui.app().backstage, "the new publication opens in the editor");
    assert_eq!(ui.doc().name, "Publication1");
    assert_eq!(ui.page_size(0), (in_emu(8.5), in_emu(11.0)));
    assert_eq!(ui.page_count(), 1);
    assert!(ui.shapes().is_empty());
    assert!(ui.app().path.is_none(), "not saved yet");
    assert!(!ui.can_undo(), "creating a publication is not an undo step");
}

#[test]
fn the_blank_landscape_tile_starts_a_new_landscape_publication() {
    let mut ui = TestUi::start();
    ui.click_ribbon("tile:Blank 11 x 8.5\"");
    assert_eq!(ui.page_size(0), (in_emu(11.0), in_emu(8.5)));
    assert!(!ui.app().backstage);
}

#[test]
fn the_church_bulletin_tile_starts_the_bulletin_template() {
    let mut ui = TestUi::start();
    assert!(!ui.has_ribbon("tile:Church Bulletin"), "Home's New row has page sizes only, no templates");
    ui.click_ribbon("link:More templates");
    ui.settle();
    ui.click_ribbon("tile:Church Bulletin");
    let want = template::church_bulletin();
    assert!(!ui.app().backstage);
    assert_eq!(ui.page_count(), want.pages.len());
    assert_eq!(ui.page_size(0), (want.pages[0].width_emu, want.pages[0].height_emu));
    assert_eq!(ui.shapes().len(), want.pages[0].shapes.len());
    assert!(!ui.shapes().is_empty());
}

#[test]
fn more_blank_page_sizes_is_greyed_and_inert() {
    let mut ui = TestUi::start();
    ui.click_ribbon("tile:More page sizes");
    assert!(ui.app().editor.is_none() && ui.app().backstage);
    assert!(in_section(&mut ui, Section::Home));
    assert!(ui.app().error.is_none() && ui.app().prompt.is_none());
}

#[test]
fn a_new_tile_with_unsaved_changes_asks_first() {
    let mut ui = new_doc();
    ui.click_at(ui.center_of(A));
    ui.key(NONE, Key::ArrowRight);
    file_entry(&mut ui, "Home");
    ui.click_ribbon("tile:Blank 11 x 8.5\"");
    assert!(matches!(ui.app().prompt, Some(Prompt::Unsaved { .. })), "unsaved-changes question");
    assert_eq!(ui.page_size(0), (in_emu(8.5), in_emu(11.0)), "the old publication is still open");
    click_button(&mut ui, "  Cancel  ");
    assert_eq!(ui.page_size(0), (in_emu(8.5), in_emu(11.0)), "Cancel keeps it");
    ui.click_ribbon("tile:Blank 11 x 8.5\"");
    click_button(&mut ui, "  Don't Save  ");
    assert_eq!(ui.page_size(0), (in_emu(11.0), in_emu(8.5)), "Don't Save starts the new one");
    assert!(!ui.app().backstage);
}

#[test]
fn a_clean_publication_is_replaced_by_a_new_tile_without_asking() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Home");
    ui.click_ribbon("tile:Blank 11 x 8.5\"");
    assert!(ui.app().prompt.is_none(), "nothing to save, nothing to ask");
    assert_eq!(ui.page_size(0), (in_emu(11.0), in_emu(8.5)));
}

#[test]
fn more_templates_opens_the_new_page_and_more_publications_the_open_page() {
    let mut ui = TestUi::start();
    ui.click_ribbon("link:More templates");
    assert!(in_section(&mut ui, Section::New));
    assert!(ui.ribbon("tabrow:Built-in").selected, "More templates opens the templates that exist (Built-in)");
    nav(&mut ui, "Home");
    ui.click_ribbon("link:More publications");
    assert!(in_section(&mut ui, Section::Open));
}

#[test]
fn the_pinned_tab_shows_its_empty_state() {
    let mut ui = TestUi::start();
    recent_files(&mut ui, 2);
    assert!(has_seen(&ui, "recent:a1.pub"));
    ui.click_ribbon("tabrow:Pinned");
    assert!(ui.ribbon("tabrow:Pinned").selected && !ui.ribbon("tabrow:Recent").selected);
    assert!(has_label(&ui, "Pin a publication to keep it here."));
    assert!(!has_seen(&ui, "recent:a1.pub"), "the recent rows are not listed under Pinned");
    ui.click_ribbon("tabrow:Recent");
    assert!(has_seen(&ui, "recent:a1.pub"));
    assert!(!has_label(&ui, "Pin a publication to keep it here."));
}

#[test]
fn the_home_page_lists_the_eight_newest_recent_publications_and_a_click_opens_one() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 10);
    let rows = recent_rows(&ui);
    assert_eq!(rows.len(), 8, "Home shows 8 rows");
    assert_eq!(rows[0], "a9.pub", "newest first");
    assert_eq!(rows[7], "a2.pub");
    assert!(!has_label_containing(&ui, "No recent publications"));
    ui.click_ribbon("recent:a5.pub");
    assert_eq!(ui.app().path.as_deref(), Some(files[5].as_path()), "the row opened its file");
    assert_eq!(ui.doc().name, "a5.pub");
    assert!(!ui.app().backstage);
    assert!(!ui.shapes().is_empty());
}

#[test]
fn home_shows_the_new_items_above_a_full_width_recent_list() {
    let mut ui = TestUi::start();
    recent_files(&mut ui, 2);
    let recent = ui.ribbon("recent:a1.pub").rect;
    let tile = ui.ribbon("tile:Blank 8.5 x 11\"").rect;
    assert!(tile.max.y <= recent.min.y, "New comes first, the Recent list below it");
    assert!(recent.width() > 3.0 * tile.width() && recent.height() < 80.0, "a row, not a card");
    let link = ui.ribbon("link:More templates").rect;
    assert!(tile.max.y - 40.0 < link.min.y && link.max.y <= recent.min.y, "the link sits under the row");
    assert!(link.right() > tile.right() + 200.0, "and is right-aligned");
}

#[test]
fn recent_rows_show_name_folder_and_opened_text_newest_first() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 3); // a2 newest
    let times = [5_000_000u64, 4_000_000, 3_000_000]; // a2, a1, a0
    for (f, t) in files.iter().zip([3_000_000u64, 4_000_000, 5_000_000]) {
        ui.app().recent.add_at(f, t);
    }
    ui.settle();
    assert_eq!(recent_rows(&ui), ["a2.pub", "a1.pub", "a0.pub"]);
    let rows: Vec<_> = ["a2.pub", "a1.pub", "a0.pub"].iter().map(|n| ui.ribbon(&format!("recent:{n}")).rect).collect();
    assert!(rows[0].max.y <= rows[1].min.y + 0.01 && rows[1].max.y <= rows[2].min.y + 0.01, "in order, not overlapping");
    for (n, (f, t)) in ["a2.pub", "a1.pub", "a0.pub"].iter().zip(files.iter().rev().zip(times)) {
        let row = ui.ribbon(&format!("recent:{n}")).rect;
        let folder = ui.ribbon(&format!("rfolder:{n}"));
        assert_eq!(folder.tip, crate::recent::breadcrumb(f.parent().unwrap()), "{n}: folder under the name");
        let name = ui.ribbon(&format!("rname:{n}"));
        assert_eq!(name.tip, *n);
        assert!(folder.rect.min.y > name.rect.min.y, "{n}: the folder is under the name");
        let opened = ui.ribbon(&format!("ropened:{n}"));
        assert_eq!(opened.tip, crate::backstage::column_label(&crate::recent::opened_label_now(t)), "{n}: Opened text (without the repeated word)");
        assert!(opened.rect.min.x > name.rect.max.x && opened.rect.max.x <= row.max.x, "{n}: Opened is its own column on the right");
        let _ = f;
    }
}

#[test]
fn a_recent_row_whose_file_is_gone_shows_an_error() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 2);
    std::fs::remove_file(&files[1]).unwrap();
    ui.click_ribbon("recent:a1.pub");
    assert!(error_text(&mut ui).contains("cannot open \"a1.pub\""), "{}", error_text(&mut ui));
    assert!(ui.app().editor.is_none(), "nothing opened");
}

#[test]
fn a_recent_row_with_unsaved_changes_asks_first() {
    let mut ui = new_doc();
    let files = recent_files(&mut ui, 1);
    ui.click_at(ui.center_of(A));
    ui.key(NONE, Key::ArrowRight);
    file_entry(&mut ui, "Home");
    ui.click_ribbon("recent:a0.pub");
    assert!(matches!(ui.app().prompt, Some(Prompt::Unsaved { .. })));
    click_button(&mut ui, "  Don't Save  ");
    assert_eq!(ui.app().path.as_deref(), Some(files[0].as_path()));
}

// ---- New page -----------------------------------------------------------------------------------

#[test]
fn the_new_page_shows_blank_tiles_and_the_office_tab_is_offline() {
    let mut ui = TestUi::start();
    nav(&mut ui, "New");
    assert!(has_label(&ui, "New"));
    for (tile, on) in [
        ("Blank 8.5 x 11\"", true),
        ("Blank 11 x 8.5\"", true),
        ("More page sizes", false),
    ] {
        assert_eq!(ui.ribbon(&format!("tile:{tile}")).enabled, on, "{tile}");
    }
    assert!(!ui.has_ribbon("tile:Church Bulletin"), "templates live under Built-in");
    assert!(ui.ribbon("tabrow:Online").selected);
    assert!(ui.ribbon("tabrow:Built-in").enabled && !ui.ribbon("tabrow:Built-in").selected);
    assert!(has_label_containing(&ui, "Coming soon: browse and download free KernPage templates"));
    assert!(has_label(&ui, "Suggested searches:"));
}

#[test]
fn the_new_pages_blank_tiles_create_publications() {
    let mut ui = TestUi::start();
    nav(&mut ui, "New");
    ui.click_ribbon("tile:Blank 11 x 8.5\"");
    assert_eq!(ui.page_size(0), (in_emu(11.0), in_emu(8.5)));
    assert!(!ui.app().backstage);
    file_entry(&mut ui, "New");
    ui.click_ribbon("tile:Blank 8.5 x 11\"");
    assert_eq!(ui.page_size(0), (in_emu(8.5), in_emu(11.0)));
}

#[test]
fn the_built_in_tab_offers_the_church_bulletin_and_greys_the_rest() {
    let mut ui = TestUi::start();
    nav(&mut ui, "New");
    ui.click_ribbon("tabrow:Built-in");
    assert!(ui.ribbon("tabrow:Built-in").selected && !ui.ribbon("tabrow:Online").selected);
    assert!(!has_label_containing(&ui, "Coming soon: browse and download free KernPage templates"));
    assert!(has_label(&ui, "Church") && has_label(&ui, "More Templates"));
    assert!(ui.ribbon("tile:Church Bulletin").enabled);
    for t in ["Order of Service", "Prayer List"] {
        assert!(!ui.ribbon(&format!("tile:{t}")).enabled, "{t} is greyed");
    }
    let cats = [
        "Brochures", "Business Cards", "Calendars", "Greeting Cards", "Labels", "Newsletters",
        "Postcards", "Advertisements", "Award Certificates", "Banners", "Business Forms",
        "Catalogs", "Envelopes", "Flyers", "Invitation Cards", "Letterhead", "Menus", "Programs",
        "Resumes", "Signs",
    ];
    for c in cats {
        assert!(!flat(&ui, c).enabled, "category {c} is greyed");
    }
    // Clicking any greyed tile or category does nothing.
    let mut targets: Vec<_> = ["Order of Service", "Prayer List"]
        .iter()
        .map(|t| ui.ribbon(&format!("tile:{t}")).rect.center())
        .collect();
    targets.extend(cats.iter().map(|c| flat(&ui, c).rect.center()));
    for p in targets {
        ui.click_at(p);
        assert!(ui.app().editor.is_none(), "a greyed template started a publication");
        assert!(ui.app().error.is_none() && ui.app().prompt.is_none());
    }
    assert!(in_section(&mut ui, Section::New));
}

#[test]
fn the_built_in_church_bulletin_tile_creates_the_bulletin() {
    let mut ui = TestUi::start();
    nav(&mut ui, "New");
    ui.click_ribbon("tabrow:Built-in");
    ui.click_ribbon("tile:Church Bulletin");
    let want = template::church_bulletin();
    assert!(!ui.app().backstage);
    assert_eq!(ui.shapes().len(), want.pages[0].shapes.len());
}

#[test]
fn the_new_page_remembers_its_tab_and_can_go_back_to_office() {
    let mut ui = TestUi::start();
    nav(&mut ui, "New");
    ui.click_ribbon("tabrow:Built-in");
    nav(&mut ui, "Open");
    nav(&mut ui, "New");
    assert!(ui.ribbon("tabrow:Built-in").selected, "the tab is remembered");
    ui.click_ribbon("tabrow:Online");
    assert!(ui.ribbon("tabrow:Online").selected);
    assert!(has_label_containing(&ui, "Coming soon: browse and download free KernPage templates"));
}

// ---- Open page ----------------------------------------------------------------------------------

#[test]
fn the_open_page_shows_places_tabs_and_an_empty_recent_list() {
    let mut ui = TestUi::start();
    nav(&mut ui, "Open");
    assert!(ui.ribbon("place:Recent").selected, "Recent is the selected place");
    assert!(!ui.ribbon("place:This PC").selected);
    assert!(flat(&ui, "Browse").enabled);
    assert!(has_label(&ui, "Recent Publications"));
    assert!(ui.ribbon("tabrow:Publications").selected);
    assert!(!ui.ribbon("tabrow:Folders").enabled, "Folders is not available");
    assert!(has_label_containing(&ui, "No recent publications yet"));
}

#[test]
fn the_open_pages_browse_button_runs_the_open_dialog() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "browse.pub");
    ui.dialogs.script_open(Some(copy.clone()));
    nav(&mut ui, "Open");
    ui.click_ribbon("flat:Browse");
    assert_eq!(ui.dialogs.opens().len(), 1);
    assert_eq!(ui.app().path.as_deref(), Some(copy.as_path()));
    assert!(!ui.app().backstage);
}

#[test]
fn the_open_page_lists_all_recent_publications_and_a_click_opens_one() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 10);
    nav(&mut ui, "Open");
    let rows = recent_rows(&ui);
    assert_eq!(rows.len(), 10, "Open shows every recent file (up to 20)");
    assert_eq!(rows[0], "a9.pub");
    assert_eq!(rows[9], "a0.pub");
    ui.click_ribbon("recent:a1.pub");
    assert_eq!(ui.app().path.as_deref(), Some(files[1].as_path()));
    assert!(!ui.app().backstage);
    // The opened file moved to the top of the list.
    file_entry(&mut ui, "Open");
    assert_eq!(recent_rows(&ui)[0], "a1.pub");
}

#[test]
fn a_recent_file_that_no_longer_exists_is_not_listed_after_a_restart() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 3);
    std::fs::remove_file(&files[2]).unwrap();
    let reloaded = crate::recent::Recent::load_from(Some(ui.env.recent.clone()));
    assert_eq!(reloaded.items().len(), 2, "the missing file is left out");
}

// ---- Info page ----------------------------------------------------------------------------------

#[test]
fn info_of_a_new_publication_shows_its_facts_and_greys_the_file_buttons() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Info");
    assert!(has_label(&ui, "Test"), "the publication name");
    assert!(has_label(&ui, "Not saved yet"));
    assert!(has_label(&ui, "Publication Properties"));
    assert!(has_label(&ui, "Pages") && has_label(&ui, "Page Size"));
    assert!(has_label(&ui, "8.5 x 11\""), "the page size");
    assert!(!has_label(&ui, "Last Saved"), "no file, no file facts");
    assert!(!flat(&ui, "Copy path").enabled);
    assert!(!flat(&ui, "Open file location").enabled);
}

#[test]
fn info_counts_the_pages() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Info");
    assert!(has_label(&ui, "3"), "three pages");
}

#[test]
fn info_of_a_saved_publication_shows_the_folder_size_and_date() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "facts.pub");
    ui.app().open_path(&copy);
    ui.settle();
    file_entry(&mut ui, "Info");
    assert!(has_label(&ui, "facts"), "the name without .pub: {:?}", ui.doc().name);
    assert!(!has_label(&ui, "Not saved yet"));
    let len = std::fs::metadata(&copy).unwrap().len();
    assert!(has_label(&ui, &format!("{:.1} KB", len as f64 / 1024.0)), "file size");
    assert!(has_label(&ui, "Size") && has_label(&ui, "Last Saved"));
    let when = std::fs::metadata(&copy).unwrap().modified().unwrap();
    assert!(has_label(&ui, &crate::recent::date_text(when)), "last saved date");
    assert!(
        has_label(&ui, &crate::recent::breadcrumb(copy.parent().unwrap())),
        "the folder as a breadcrumb"
    );
    assert!(flat(&ui, "Copy path").enabled && flat(&ui, "Open file location").enabled);
}

#[test]
fn info_copy_path_puts_the_file_path_on_the_clipboard() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "copyme.pub");
    ui.app().open_path(&copy);
    ui.settle();
    file_entry(&mut ui, "Info");
    click_unsettled(&mut ui, "flat:Copy path");
    assert_eq!(copied(&ui), Some(copy.to_string_lossy().into_owned()));
}

#[test]
fn info_open_file_location_asks_the_system_to_open_the_folder() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "where.pub");
    ui.app().open_path(&copy);
    ui.settle();
    file_entry(&mut ui, "Info");
    crate::output::test_os::take_opened();
    ui.click_ribbon("flat:Open file location");
    assert_eq!(crate::output::test_os::take_opened(), vec![copy.parent().unwrap().to_path_buf()]);
}

#[test]
fn info_unsupported_rows_are_described_and_their_buttons_are_greyed() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Info");
    for title in ["Contact Details", "Check Publication", "Embedded Font Information"] {
        assert!(has_label(&ui, title), "{title}");
    }
    assert!(has_label_containing(&ui, "Saved sets of contact details are not supported yet"));
    // The three big buttons are disabled buttons.
    let found: Vec<(bool, egui::Pos2)> = ui
        .h
        .query_all_by_role(egui::accesskit::Role::Button)
        .filter(|b| {
            let l = b.accesskit_node().label().unwrap_or_default();
            l.contains("Edit Contact") || l.contains("Check\nPublication") || l.contains("Fonts in")
        })
        .map(|b| {
            let r = b.accesskit_node().raw_bounds().expect("bounds");
            (b.accesskit_node().is_disabled(), egui::pos2(((r.x0 + r.x1) / 2.0) as f32, ((r.y0 + r.y1) / 2.0) as f32))
        })
        .collect();
    assert_eq!(found.len(), 3, "three buttons");
    assert!(found.iter().all(|f| f.0), "all greyed");
    let rev = ui.revision();
    for (_, at) in found {
        ui.click_at(at);
    }
    assert_eq!(ui.revision(), rev);
    assert!(ui.app().error.is_none() && ui.app().prompt.is_none());
}

// ---- Print page ---------------------------------------------------------------------------------

#[test]
fn the_print_page_shows_the_first_sheet_and_the_page_counter() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    assert!(has_label(&ui, "Sheet 1 of 3"));
    assert!(has_label(&ui, "Printer") && has_label(&ui, "Settings"));
    assert!(has_label_containing(&ui, "Copies of print job"));
    assert!(ui.ribbon("print:Print").enabled);
}

#[test]
fn the_print_preview_arrows_step_through_the_sheets_and_stop_at_both_ends() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    assert!(!ui.ribbon("printnav:prev").enabled, "no sheet before the first");
    assert!(ui.ribbon("printnav:next").enabled);
    ui.click_ribbon("printnav:next");
    assert!(has_label(&ui, "Sheet 2 of 3"));
    assert_eq!(ui.app().print_sheet, 1);
    ui.click_ribbon("printnav:next");
    assert!(has_label(&ui, "Sheet 3 of 3"));
    assert!(!ui.ribbon("printnav:next").enabled, "no sheet after the last");
    ui.click_ribbon("printnav:next");
    assert_eq!(ui.app().print_sheet, 2, "clicking the greyed arrow does nothing");
    ui.click_ribbon("printnav:prev");
    ui.click_ribbon("printnav:prev");
    assert!(has_label(&ui, "Sheet 1 of 3"));
    ui.click_ribbon("printnav:prev");
    assert_eq!(ui.app().print_sheet, 0);
    // The preview does not change the publication or the current page.
    assert_eq!(ui.app().page, 0);
}

#[test]
fn a_single_page_publication_has_both_preview_arrows_greyed() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    assert!(has_label(&ui, "Sheet 1 of 1"));
    assert!(!ui.ribbon("printnav:prev").enabled && !ui.ribbon("printnav:next").enabled);
}

/// The Print page draws four combo boxes; counting back from the last one drawn.
const DUPLEX: usize = 0;
const LAYOUT: usize = 1;
const COLLATION: usize = 2;
const PAGES: usize = 3;
const PRINTER: usize = 4;

fn open_print_combo(ui: &mut TestUi, from_last: usize) {
    let all: Vec<_> = ui.h.get_all_by_role(egui::accesskit::Role::ComboBox).collect();
    all[all.len() - 1 - from_last].click();
    ui.settle();
}

#[test]
fn the_print_page_offers_printers_copies_pages_and_collation() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    assert!(has_label(&ui, "Printer") && has_label(&ui, "Settings"));
    assert!(has_label_containing(&ui, "Copies of print job"));
    // The default printer is preselected in the Printer drop-down.
    assert_eq!(ui.app().print.printer.as_deref(), Some("Test Inkjet"));
    assert_eq!(ui.app().print.scope, crate::printing::PageScope::All);
    assert!(ui.app().print.collate);
    assert_eq!(ui.h.get_all_by_role(egui::accesskit::Role::ComboBox).count(), 5, "printer, pages, collation, layout, two-sided");
    assert!(has_label(&ui, "Pages:"));
    assert!(has_label_containing(&ui, "Portrait Orientation"));
    assert!(ui.ribbon("link:Printer Properties").enabled);
    assert!(ui.ribbon("link:Open as PDF instead").enabled);
    // Every installed printer is in the drop-down.
    open_print_combo(&mut ui, PRINTER);
    assert!(has_label(&ui, "Test Laser") && has_label(&ui, "Test Inkjet"));
}

#[test]
fn the_pages_dropdown_lists_all_current_and_custom_and_custom_enables_the_pages_box() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    let boxed = |ui: &TestUi| ui.h.get_by_role(egui::accesskit::Role::TextInput).accesskit_node().is_disabled();
    assert!(boxed(&ui), "the Pages box waits for Custom Print");
    open_print_combo(&mut ui, PAGES);
    assert!(has_label(&ui, "Print All Pages") && has_label(&ui, "Print Current Page") && has_label(&ui, "Custom Print"));
    click_button(&mut ui, "Custom Print");
    assert!(!boxed(&ui));
    assert_eq!(ui.app().print.scope, crate::printing::PageScope::Custom);
}

#[test]
fn the_remaining_print_settings_are_greyed_and_inert() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    let rows = ["Publication page size", "Composite RGB"];
    for r in rows {
        assert!(!ui.ribbon(&format!("setting:{r}")).enabled, "{r}");
    }
    let rev = ui.revision();
    crate::output::test_os::take_opened();
    crate::printing::test_os::take_printed();
    for r in rows {
        let p = ui.ribbon(&format!("setting:{r}")).rect.center();
        ui.click_at(p);
    }
    assert_eq!(ui.revision(), rev);
    assert!(ui.app().error.is_none() && ui.app().prompt.is_none());
    assert!(crate::output::test_os::take_opened().is_empty());
    assert!(crate::printing::test_os::take_printed().is_empty(), "nothing was printed");
    for b in ui.h.query_all_by_role(egui::accesskit::Role::CheckBox) {
        assert!(b.accesskit_node().is_disabled(), "{:?}", b.accesskit_node().label());
    }
}

#[test]
fn the_print_button_sends_all_pages_to_the_default_printer_without_opening_a_pdf() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    let rev = ui.revision();
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1);
    let j = &jobs[0];
    assert_eq!((j.printer.as_str(), j.copies, j.collate, j.landscape), ("Test Inkjet", 1, true, false));
    assert_eq!(j.pages, vec![0, 1, 2]);
    assert_eq!(j.doc_name, "Test");
    assert!(crate::output::test_os::take_opened().is_empty(), "no PDF viewer");
    assert!(ui.app().error.is_none());
    assert!(ui.app().backstage, "the File view stays open");
    assert_eq!(ui.revision(), rev, "printing changes nothing");
    ui.settle();
    assert!(ui.app().print.running.is_none(), "the finished job is noticed");
}

#[test]
fn custom_print_of_page_two_with_three_copies_on_another_printer_records_that_job() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    open_print_combo(&mut ui, PRINTER);
    click_button(&mut ui, "Test Laser");
    open_print_combo(&mut ui, PAGES);
    click_button(&mut ui, "Custom Print");
    ui.h.get_by_role(egui::accesskit::Role::TextInput).click();
    ui.type_text("2");
    ui.settle();
    ui.app().print.copies = 3;
    ui.settle();
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1, "{:?}", ui.app().error);
    let j = &jobs[0];
    assert_eq!((j.printer.as_str(), j.copies, j.collate), ("Test Laser", 3, true));
    assert_eq!(j.pages, vec![1]);
}

#[test]
fn print_current_page_and_uncollated_are_recorded() {
    let mut ui = pages_doc(3);
    ui.app().go_page(2);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    open_print_combo(&mut ui, PAGES);
    click_button(&mut ui, "Print Current Page");
    open_print_combo(&mut ui, COLLATION);
    click_button(&mut ui, "Uncollated");
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].pages, vec![2]);
    assert!(!jobs[0].collate);
}

#[test]
fn a_bad_page_range_shows_a_message_and_prints_nothing() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    open_print_combo(&mut ui, PAGES);
    click_button(&mut ui, "Custom Print");
    ui.h.get_by_role(egui::accesskit::Role::TextInput).click();
    ui.type_text("9");
    ui.settle();
    assert!(has_label_containing(&ui, "The publication has only 3 pages"), "shown beside the box");
    ui.click_ribbon("print:Print");
    assert!(ui.app().error.as_deref().is_some_and(|e| e.contains("only 3 pages")));
    assert!(crate::printing::test_os::take_printed().is_empty());
}

#[test]
fn printer_properties_asks_for_the_selected_printers_dialog() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_properties();
    ui.click_ribbon("link:Printer Properties");
    assert_eq!(crate::printing::test_os::take_properties(), vec!["Test Inkjet".to_owned()]);
    assert!(crate::printing::test_os::take_printed().is_empty());
}

#[test]
fn open_as_pdf_instead_writes_a_pdf_and_opens_it_in_the_viewer() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    ui.click_ribbon("link:Open as PDF instead");
    let pdf = ui.env.files.join("print").join("Test.pdf");
    assert!(std::fs::read(&pdf).is_ok_and(|b| b.starts_with(b"%PDF")), "the PDF was created");
    assert_eq!(crate::output::test_os::take_opened(), vec![pdf], "and handed to the viewer");
    assert!(crate::printing::test_os::take_printed().is_empty(), "nothing was sent to a printer");
    assert!(ui.app().error.is_none());
}

#[test]
fn ctrl_p_then_the_print_button_prints_unsaved_text_edits_too() {
    let mut ui = new_doc();
    ui.key(CTRL, Key::P);
    assert!(in_section(&mut ui, Section::Print));
    crate::printing::test_os::take_printed();
    ui.click_ribbon("print:Print");
    assert_eq!(crate::printing::test_os::take_printed().len(), 1);
}

// ---- Export, Share ------------------------------------------------------------------------------

#[test]
fn the_export_page_offers_create_pdf_and_greys_the_other_rows() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    assert!(ui.ribbon("exportrow:Create a PDF").enabled, "the supported row");
    assert!(has_label(&ui, "Create a PDF"));
    assert!(has_label(&ui, "Package for Printing"));
    let off = [
        "Publish HTML", "Change File Type", "Save for Photo Printing",
        "Save for a Commercial Printer", "Save for Another Computer",
    ];
    for r in off {
        assert!(!ui.ribbon(&format!("exportrow:{r}")).enabled, "{r} is greyed");
    }
}

#[test]
fn clicking_the_greyed_export_rows_does_nothing() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    let rev = ui.revision();
    for r in [
        "Publish HTML", "Change File Type", "Save for Photo Printing",
        "Save for a Commercial Printer", "Save for Another Computer",
    ] {
        let p = ui.ribbon(&format!("exportrow:{r}")).rect.center();
        ui.click_at(p);
        assert!(in_section(&mut ui, Section::Export), "{r}: still on Export");
        assert!(ui.app().error.is_none() && ui.app().prompt.is_none(), "{r}");
    }
    assert!(ui.dialogs.saves().is_empty(), "no dialog opened");
    assert_eq!(ui.revision(), rev);
}

#[test]
fn clicking_the_selected_create_pdf_row_itself_does_nothing_the_big_button_creates() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Export");
    let p = ui.ribbon("exportrow:Create a PDF").rect.center();
    ui.click_at(p);
    assert!(ui.dialogs.saves().is_empty(), "the row is only the selected category");
    let target = ui.env.files.join("x.pdf");
    ui.dialogs.script_save(Some(target.clone()));
    ui.click_ribbon("export:Create PDF");
    assert_eq!(ui.dialogs.saves().len(), 1);
    assert!(std::fs::read(&target).is_ok_and(|b| b.starts_with(b"%PDF")));
}

// ---- Options ------------------------------------------------------------------------------------

#[test]
fn the_options_page_has_the_save_category_and_greyed_extra_options() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Options");
    assert!(has_label(&ui, "Save documents"));
    assert!(has_label_containing(&ui, "Customize how documents are saved"));
    for l in [
        "Allow background saves",
        "Don't show the File screen when opening or saving files with keyboard shortcuts",
    ] {
        let n = ui.h.get_by_label(l);
        assert!(n.accesskit_node().is_disabled(), "{l} is greyed");
    }
    assert!(!has_label_containing(&ui, "sign-in") && !has_label_containing(&ui, "Save to Computer"));
    let live = ui.h.get_by_label("Save AutoRecover information every");
    assert!(!live.accesskit_node().is_disabled(), "the AutoRecover box is live");
    assert!(flat(&ui, "OK").enabled && flat(&ui, "Cancel").enabled);
    assert!(has_label_containing(&ui, "Default personal templates location"));
}

#[test]
fn ok_and_cancel_on_the_options_page_return_to_the_publication() {
    let mut ui = new_doc();
    file_entry(&mut ui, "Options");
    ui.click_ribbon("flat:Cancel");
    assert!(!ui.app().backstage);
    file_entry(&mut ui, "Options");
    ui.click_ribbon("flat:OK");
    assert!(!ui.app().backstage);
    // Without a publication they return to Home instead.
    let mut ui = TestUi::start();
    nav(&mut ui, "Options");
    ui.click_ribbon("flat:Cancel");
    assert!(ui.app().backstage && in_section(&mut ui, Section::Home));
}

#[test]
fn opening_a_publication_from_any_page_closes_the_file_view() {
    let mut ui = TestUi::start();
    let copy = copy_sample(&ui, "two-boxes.pub", "any.pub");
    ui.app().open_path(&copy);
    ui.settle();
    assert!(!ui.app().backstage);
    ui.click_tab("File");
    assert!(ui.ribbon("nav:Info").selected);
}

fn in_emu(v: f64) -> i64 {
    kernpage_core::geometry::in_to_emu(v)
}

fn has_seen(ui: &TestUi, label: &str) -> bool {
    ui.ribbon_buttons().iter().any(|b| b.label == label)
}

#[test]
fn a_recent_card_shows_the_first_page_once_the_file_has_loaded() {
    let mut ui = TestUi::start();
    recent_files(&mut ui, 1);
    ui.app().start_thumbs.wait_idle();
    ui.settle();
    assert!(has_seen(&ui, "preview:a0.pub"), "a loaded file draws its page");
    assert!(!has_seen(&ui, "glyph:a0.pub"));
}

#[test]
fn a_recent_card_for_a_missing_file_keeps_the_glyph() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 1);
    std::fs::remove_file(&files[0]).unwrap();
    ui.app().start_thumbs.wait_idle();
    ui.settle();
    assert!(has_seen(&ui, "glyph:a0.pub"));
    assert!(!has_seen(&ui, "preview:a0.pub"));
}

#[test]
fn the_church_bulletin_tile_draws_a_preview() {
    let mut ui = TestUi::start();
    ui.click_ribbon("link:More templates");
    ui.settle();
    ui.app().start_thumbs.wait_idle();
    ui.settle();
    assert!(has_seen(&ui, "preview:Church Bulletin"));
}

#[test]
fn the_about_page_shows_version_licence_privacy_and_independence() {
    let mut ui = new_doc();
    ui.click_tab("File");
    nav(&mut ui, "About KernPage");
    assert!(in_section(&mut ui, Section::About));
    assert!(has_label(&ui, &format!("KernPage {}", crate::app_version())));
    assert!(has_label_containing(&ui, "Free and open source under the GNU General Public License, version 3 (GPL-3.0)."));
    assert!(has_label_containing(&ui, "Nothing is sent from KernPage."));
    assert!(has_label_containing(&ui, "only if you turn them on or press Check."));
    assert!(has_label_containing(&ui, "not affiliated with, endorsed by or sponsored by Microsoft"));
    assert!(has_label_containing(&ui, "Opens and saves KernPage (.kern) and Microsoft Publisher (.pub) files."));
    crate::feedback::test_os::take_opened();
    ui.click_ribbon("link:Website: https://238apps.com/kernpage");
    ui.click_ribbon("link:Source code: https://github.com/jmitchell238/KernPage");
    ui.click_ribbon("link:Report a problem: https://github.com/jmitchell238/KernPage/issues");
    ui.click_ribbon("link:Buy me a RedBull: https://buymeacoffee.com/238apps");
    assert_eq!(
        crate::feedback::test_os::take_opened(),
        [
            "https://238apps.com/kernpage",
            "https://github.com/jmitchell238/KernPage",
            "https://github.com/jmitchell238/KernPage/issues",
            "https://buymeacoffee.com/238apps"
        ]
    );
}

#[test]
fn help_about_opens_the_about_page() {
    let mut ui = new_doc();
    ui.click_tab("Help");
    ui.click_ribbon("About KernPage");
    assert!(ui.app().backstage);
    assert!(in_section(&mut ui, Section::About));
    assert!(has_label(&ui, &format!("KernPage {}", crate::app_version())));
    assert!(ui.app().error.is_none(), "no separate About box");
}

#[test]
fn no_account_label_remains_on_the_file_screen() {
    let mut ui = TestUi::start();
    for n in ALL_ENTRIES {
        assert!(!ui.ribbon_buttons().iter().any(|b| b.label.contains("Account")), "{n}");
    }
    assert!(!has_label_containing(&ui, "Account"));
    nav(&mut ui, "About KernPage");
    assert!(!has_label_containing(&ui, "Account"));
    assert!(!ui.ribbon_buttons().iter().any(|b| b.label.contains("Account")));
}

#[test]
fn the_notices_link_needs_the_installed_file() {
    let dir = std::env::temp_dir().join(format!("kp-notices-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(crate::backstage::notices_in(&dir), None);
    std::fs::write(dir.join("THIRD_PARTY_NOTICES.txt"), "x").unwrap();
    assert!(crate::backstage::notices_in(&dir).is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newly_saved_kern_file_appears_as_a_recent_card_with_its_first_page() {
    let mut ui = TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 2.0)]));
    let target = ui.env.files.join("fresh.kern");
    ui.dialogs.script_save(Some(target.clone()));
    ui.key(CTRL, Key::S);
    assert_eq!(ui.app().recent.items(), &[target]);
    ui.click_tab("File");
    ui.app().section = Section::Home;
    ui.settle();
    ui.app().start_thumbs.wait_idle();
    ui.settle();
    assert!(has_seen(&ui, "preview:fresh.kern"), "the saved file's first page is drawn on its Recent card");
}

#[test]
fn the_start_items_have_pixel_snapped_page_pictures_inside_their_items() {
    // Every page picture on Home's New row is drawn by `tile` (page sizes only; templates are on New).
    for ppp in [1.0f32, 1.25, 1.5] {
        let mut ui = TestUi::start();
        ui.h.set_pixels_per_point(ppp);
        ui.settle();
        let all = ui.ribbon_buttons();
        let mut n = 0;
        for name in ["Blank 8.5 x 11\"", "Blank 11 x 8.5\"", "More page sizes"] {
            let tile = all.iter().find(|b| b.label == format!("tile:{name}")).expect("tile").rect;
            let page = all.iter().find(|b| b.label == format!("page:{name}")).expect("page").rect;
            assert!(tile.contains_rect(page), "{name}: the page stays inside its item");
            assert!(page.min.x - tile.min.x >= 7.0, "{name}: room at the side");
            for v in [page.min.x, page.min.y, page.max.x, page.max.y] {
                assert!((v * ppp - (v * ppp).round()).abs() < 1e-3, "{name}@{ppp}: page edge {v} off the device pixels");
            }
            n += 1;
        }
        assert_eq!(n, 3);
    }
}

#[test]
fn the_rail_items_still_navigate() {
    let mut ui = TestUi::start();
    for (entry, section) in [
        ("New", Section::New),
        ("Open", Section::Open),
        ("Options", Section::Options),
        ("Home", Section::Home),
    ] {
        nav(&mut ui, entry);
        assert!(in_section(&mut ui, section), "{entry}");
        assert!(ui.ribbon(&format!("nav:{entry}")).selected, "{entry} is drawn selected");
    }
}

#[test]
fn the_open_page_lists_recent_rows_like_home() {
    let mut ui = TestUi::start();
    recent_files(&mut ui, 3);
    nav(&mut ui, "Open");
    assert_eq!(recent_rows(&ui), ["a2.pub", "a1.pub", "a0.pub"]);
    assert!(has_seen(&ui, "place:Recent"));
    let row = ui.ribbon("recent:a2.pub").rect;
    assert!(row.height() < 80.0 && row.width() > 300.0);
}

#[test]
fn saving_moves_the_file_to_the_top_of_recent_with_the_save_time() {
    let mut ui = TestUi::start();
    let files = recent_files(&mut ui, 2); // a1.pub newest, then a0.pub
    assert_eq!(ui.app().recent.items(), &[files[1].clone(), files[0].clone()]);
    // An old stamp for a1, so the save below visibly refreshes it.
    ui.app().recent.add_at(&files[1], 1_000);
    ui.app().recent.add_at(&files[0], 2_000);
    ui.app().new_document(letter_doc(vec![]), None, None);
    ui.settle();
    let target = ui.env.files.join("saved-now.kern");
    ui.dialogs.script_save(Some(target.clone()));
    let before = kernpage_core::recent::now_secs();
    ui.key(CTRL, Key::S);
    let r = &ui.app().recent;
    assert_eq!(r.items(), &[target.clone(), files[0].clone(), files[1].clone()], "the saved file is on top");
    assert!(r.opened_at(&target).unwrap() >= before, "stamped with the save time");
    // recent.txt stays paths only; the stamps are in the file next to it.
    let stored = std::fs::read_to_string(&ui.env.recent).unwrap();
    assert!(stored.lines().next().unwrap().ends_with("saved-now.kern") && !stored.contains('\t'), "{stored}");
    let times = std::fs::read_to_string(ui.env.recent.with_file_name("recent-times.txt")).unwrap();
    assert!(times.lines().next().unwrap().contains('\t') && times.contains("saved-now.kern"), "{times}");
}

// ---- Print layouts and the print preview ---------------------------------------------------------

/// A document of `n` pages of `w` x `h` inches; page k carries k rectangles so pages differ.
fn sized_doc(w: f64, h: f64, n: usize) -> TestUi {
    use kernpage_core::geometry::in_to_emu;
    let pages = (1..=n)
        .map(|k| {
            let mut p = kernpage_core::Page::blank(in_to_emu(w), in_to_emu(h));
            p.shapes = (0..k).map(|i| rect_shape(0.2 + 0.3 * i as f64, 0.3, 0.2, 0.4)).collect();
            p
        })
        .collect();
    TestUi::with_doc(kernpage_core::Document::new("Layouts", pages))
}

fn layout_texts() -> [&'static str; 4] {
    ["One page per sheet", "Multiple copies per sheet", "Multiple pages per sheet", "Booklet, side-fold"]
}

fn count_label(ui: &TestUi, text: &str) -> usize {
    ui.h.query_all_by_label(text).count()
}

/// The page numbers drawn on the previewed sheet side, left to right.
fn drawn_pages(ui: &TestUi) -> Vec<usize> {
    let mut v: Vec<(f32, usize)> = ui
        .ribbon_buttons()
        .iter()
        .filter_map(|b| b.label.strip_prefix("printpage:").map(|n| (b.rect.min.x, n.parse().unwrap())))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v.into_iter().map(|p| p.1).collect()
}

#[test]
fn a_letter_page_offers_only_one_page_per_sheet() {
    let mut ui = pages_doc(2);
    file_entry(&mut ui, "Print");
    assert_eq!(ui.app().print_layout(), kernpage_core::PrintLayout::OnePerSheet);
    open_print_combo(&mut ui, LAYOUT);
    assert!(has_label(&ui, "One page per sheet"));
    for other in &layout_texts()[1..] {
        assert_eq!(count_label(&ui, other), 0, "{other}");
    }
}

#[test]
fn a_business_card_page_defaults_to_copies_per_sheet_and_offers_all_four_layouts() {
    let mut ui = sized_doc(3.5, 2.0, 3);
    file_entry(&mut ui, "Print");
    assert_eq!(ui.app().print_layout(), kernpage_core::PrintLayout::CopiesPerSheet);
    // One sheet per page, each full of one page: 2 x 5 cells.
    assert!(has_label(&ui, "Sheet 1 of 3"));
    assert_eq!(drawn_pages(&ui), vec![1; 10]);
    open_print_combo(&mut ui, LAYOUT);
    for t in layout_texts() {
        assert!(has_label(&ui, t), "{t}");
    }
}

#[test]
fn half_letter_pages_default_to_one_page_per_sheet_and_offer_all_four_layouts() {
    let mut ui = sized_doc(5.5, 8.5, 4);
    file_entry(&mut ui, "Print");
    assert_eq!(ui.app().print_layout(), kernpage_core::PrintLayout::OnePerSheet);
    open_print_combo(&mut ui, LAYOUT);
    for t in layout_texts() {
        assert!(has_label(&ui, t), "{t}");
    }
    click_button(&mut ui, "Multiple pages per sheet");
    assert_eq!(ui.app().print.layout, Some(kernpage_core::PrintLayout::PagesPerSheet));
    assert!(has_label(&ui, "Sheet 1 of 2"), "two pages per sheet: 4 pages make 2 sheets");
    assert_eq!(drawn_pages(&ui), vec![1, 2]);
}

#[test]
fn the_preview_arrows_step_through_the_sheet_sides_of_a_layout() {
    let mut ui = sized_doc(5.5, 8.5, 5);
    file_entry(&mut ui, "Print");
    open_print_combo(&mut ui, LAYOUT);
    click_button(&mut ui, "Multiple pages per sheet");
    // 5 pages, 2 per sheet: 3 sheets, the last with an empty cell.
    assert!(has_label(&ui, "Sheet 1 of 3"));
    assert!(!ui.ribbon("printnav:prev").enabled);
    ui.click_ribbon("printnav:next");
    ui.click_ribbon("printnav:next");
    assert!(has_label(&ui, "Sheet 3 of 3"));
    assert_eq!(drawn_pages(&ui), vec![5], "the empty cell draws nothing");
    assert!(!ui.ribbon("printnav:next").enabled);
    // Copies change the sheet count.
    ui.app().print.copies = 2;
    ui.settle();
    assert!(has_label(&ui, "Sheet 3 of 6"));
}

#[test]
fn an_eight_page_half_letter_booklet_previews_in_folding_order() {
    let mut ui = sized_doc(5.5, 8.5, 8);
    file_entry(&mut ui, "Print");
    open_print_combo(&mut ui, LAYOUT);
    click_button(&mut ui, "Booklet, side-fold");
    assert_eq!(ui.app().print_layout(), kernpage_core::PrintLayout::Booklet);
    let expected = [vec![8, 1], vec![2, 7], vec![6, 3], vec![4, 5]];
    let names = ["Sheet 1, front", "Sheet 1, back", "Sheet 2, front", "Sheet 2, back"];
    for (i, (pages, name)) in expected.iter().zip(names).enumerate() {
        assert!(has_label(&ui, name), "{name}");
        assert_eq!(&drawn_pages(&ui), pages, "side {}", i + 1);
        // The sheet is landscape: the two pages sit side by side.
        let sheet = ui.ribbon("printsheet").rect;
        assert!(sheet.width() > sheet.height());
        if i < 3 {
            ui.click_ribbon("printnav:next");
        }
    }
    assert!(!ui.ribbon("printnav:next").enabled, "four sides in all");
}

#[test]
fn changing_the_page_range_updates_the_preview() {
    let mut ui = sized_doc(5.5, 8.5, 8);
    file_entry(&mut ui, "Print");
    open_print_combo(&mut ui, LAYOUT);
    click_button(&mut ui, "Booklet, side-fold");
    open_print_combo(&mut ui, PAGES);
    click_button(&mut ui, "Custom Print");
    ui.h.get_by_role(egui::accesskit::Role::TextInput).click();
    ui.type_text("1-4");
    ui.settle();
    assert_eq!(drawn_pages(&ui), vec![4, 1]);
    ui.click_ribbon("printnav:next");
    assert_eq!(drawn_pages(&ui), vec![2, 3]);
    assert!(!ui.ribbon("printnav:next").enabled);
}

// ---- Printing a layout, two-sided printing, paper size ---------------------------------------------

use crate::printing::Duplex;

fn pick(ui: &mut TestUi, from_last: usize, label: &str) {
    open_print_combo(ui, from_last);
    click_button(ui, label);
}

fn select_printer(ui: &mut TestUi, name: &str) {
    pick(ui, PRINTER, name);
}

fn booklet_ui(printer: &str) -> TestUi {
    let mut ui = sized_doc(5.5, 8.5, 8);
    file_entry(&mut ui, "Print");
    select_printer(&mut ui, printer);
    pick(&mut ui, LAYOUT, "Booklet, side-fold");
    crate::printing::test_os::take_printed();
    ui
}

/// The page numbers (1-based) in each placement of each side, left to right.
fn job_pages(job: &crate::printing::PrintJob) -> Vec<Vec<Option<usize>>> {
    job.sides.iter().map(|s| s.placements.iter().map(|p| p.page.map(|n| n + 1)).collect()).collect()
}

fn expect_booklet_8() -> Vec<Vec<Option<usize>>> {
    [[8, 1], [2, 7], [6, 3], [4, 5]].iter().map(|p| p.iter().map(|&n| Some(n)).collect()).collect()
}

#[test]
fn printing_a_booklet_submits_every_side_in_folding_order_on_landscape_sheets() {
    let mut ui = booklet_ui("Test Inkjet");
    assert_eq!(ui.app().print_duplex(), Duplex::ShortEdge, "booklets default to both sides, short edge");
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1, "{:?}", ui.app().error);
    let j = &jobs[0];
    assert_eq!(j.layout, kernpage_core::PrintLayout::Booklet);
    assert_eq!(job_pages(j), expect_booklet_8());
    assert!(j.landscape);
    assert!(j.sides.iter().all(|s| s.paper == (792.0, 612.0)));
    assert_eq!(j.duplex, Duplex::ShortEdge);
    assert_eq!(j.pages, (0..8).collect::<Vec<_>>());
    // The placements are the preview's: half a landscape sheet each.
    let r = j.sides[0].placements[1].rect;
    assert_eq!((r.x, r.y, r.w, r.h), (396.0, 0.0, 396.0, 612.0));
    assert!(ui.app().prompt.is_none(), "automatic two-sided printing asks nothing");
}

#[test]
fn one_page_per_sheet_prints_as_before() {
    let mut ui = pages_doc(3);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1);
    let j = &jobs[0];
    assert_eq!(j.layout, kernpage_core::PrintLayout::OnePerSheet);
    assert_eq!(j.duplex, Duplex::OneSided);
    assert!(!j.landscape);
    assert_eq!(job_pages(j), vec![vec![Some(1)], vec![Some(2)], vec![Some(3)]]);
    assert!(j.sides.iter().all(|s| s.paper == (612.0, 792.0)));
    assert!(ui.app().prompt.is_none() && ui.app().print.pending_backs.is_none());
}

#[test]
fn copies_per_sheet_prints_the_grid() {
    let mut ui = sized_doc(3.5, 2.0, 2);
    file_entry(&mut ui, "Print");
    crate::printing::test_os::take_printed();
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].sides.len(), 2);
    assert!(jobs[0].sides.iter().all(|s| s.placements.len() == 10));
    assert_eq!(jobs[0].duplex, Duplex::OneSided);
}

#[test]
fn the_two_sided_choice_sets_the_duplex_flag_of_the_job() {
    let cases = [
        ("Print One Sided", Duplex::OneSided),
        (Duplex::LongEdge.label(), Duplex::LongEdge),
        (Duplex::ShortEdge.label(), Duplex::ShortEdge),
    ];
    for (label, want) in cases {
        let mut ui = booklet_ui("Test Inkjet");
        pick(&mut ui, DUPLEX, label);
        assert_eq!(ui.app().print_duplex(), want, "{label}");
        ui.click_ribbon("print:Print");
        let jobs = crate::printing::test_os::take_printed();
        assert_eq!(jobs.len(), 1, "{label}: {:?}", ui.app().error);
        assert_eq!(jobs[0].duplex, want, "{label}");
        assert_eq!(jobs[0].sides.len(), 4);
    }
}

#[test]
fn automatic_two_sided_choices_are_hidden_when_the_printer_cannot_duplex() {
    let mut ui = sized_doc(5.5, 8.5, 4);
    file_entry(&mut ui, "Print");
    // The recorded inkjet can duplex, the recorded laser cannot.
    assert_eq!(ui.app().duplex_choices(), Duplex::ALL.to_vec());
    open_print_combo(&mut ui, DUPLEX);
    for d in Duplex::ALL {
        assert!(has_label(&ui, d.label()), "{}", d.label());
    }
    click_button(&mut ui, "Print One Sided");
    select_printer(&mut ui, "Test Laser");
    assert_eq!(ui.app().duplex_choices(), vec![Duplex::OneSided, Duplex::Manual]);
    open_print_combo(&mut ui, DUPLEX);
    assert!(has_label(&ui, Duplex::Manual.label()));
    assert_eq!(count_label(&ui, Duplex::LongEdge.label()), 0);
    assert_eq!(count_label(&ui, Duplex::ShortEdge.label()), 0);
}

#[test]
fn a_chosen_automatic_mode_falls_back_when_the_printer_cannot_duplex() {
    let mut ui = booklet_ui("Test Inkjet");
    pick(&mut ui, DUPLEX, Duplex::LongEdge.label());
    select_printer(&mut ui, "Test Laser");
    assert_eq!(ui.app().print_duplex(), Duplex::Manual, "the booklet default without duplex support");
}

#[test]
fn manual_two_sided_prints_the_fronts_asks_then_prints_the_backs_last_sheet_first() {
    let mut ui = booklet_ui("Test Laser");
    assert_eq!(ui.app().print_duplex(), Duplex::Manual, "no duplex support: booklets default to by hand");
    ui.click_ribbon("print:Print");
    ui.settle();
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1, "only the fronts so far: {:?}", ui.app().error);
    assert_eq!(job_pages(&jobs[0]), vec![vec![Some(8), Some(1)], vec![Some(6), Some(3)]]);
    assert_eq!(jobs[0].duplex, Duplex::OneSided, "the printer prints one side");
    assert!(jobs[0].landscape);
    assert!(has_label_containing(&ui, "turn the stack over without changing its order"));
    assert!(has_label(&ui, "  Continue  ") && has_label(&ui, "  Cancel  "));
    click_button(&mut ui, "  Continue  ");
    ui.settle();
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs.len(), 1, "{:?}", ui.app().error);
    // The back sides, last sheet first.
    assert_eq!(job_pages(&jobs[0]), vec![vec![Some(4), Some(5)], vec![Some(2), Some(7)]]);
    assert_eq!(jobs[0].duplex, Duplex::OneSided);
    assert!(ui.app().prompt.is_none() && ui.app().print.pending_backs.is_none());
    assert!(!has_label_containing(&ui, "turn the stack over"));
}

#[test]
fn cancelling_the_turn_over_message_prints_no_backs() {
    let mut ui = booklet_ui("Test Laser");
    ui.click_ribbon("print:Print");
    ui.settle();
    assert_eq!(crate::printing::test_os::take_printed().len(), 1);
    click_button(&mut ui, "  Cancel  ");
    ui.settle();
    assert!(crate::printing::test_os::take_printed().is_empty());
    assert!(ui.app().prompt.is_none() && ui.app().print.pending_backs.is_none());
    assert!(!has_label_containing(&ui, "turn the stack over"));
}

#[test]
fn manual_two_sided_for_an_odd_booklet_leaves_a_blank_back() {
    let mut ui = sized_doc(5.5, 8.5, 6);
    file_entry(&mut ui, "Print");
    select_printer(&mut ui, "Test Laser");
    pick(&mut ui, LAYOUT, "Booklet, side-fold");
    crate::printing::test_os::take_printed();
    ui.click_ribbon("print:Print");
    ui.settle();
    let fronts = crate::printing::test_os::take_printed();
    assert_eq!(job_pages(&fronts[0]), vec![vec![Some(6), Some(1)], vec![Some(4), Some(3)]]);
    click_button(&mut ui, "  Continue  ");
    ui.settle();
    let backs = crate::printing::test_os::take_printed();
    // Three sides pad to four; the second sheet's back is blank and goes first.
    assert_eq!(job_pages(&backs[0]), vec![vec![], vec![Some(2), Some(5)]]);
}

#[test]
fn the_preview_of_a_two_sided_job_counts_sheets_front_and_back() {
    let mut ui = sized_doc(5.5, 8.5, 3);
    file_entry(&mut ui, "Print");
    pick(&mut ui, DUPLEX, Duplex::LongEdge.label());
    // 3 pages one per side, padded to 4 sides = 2 sheets.
    assert!(has_label(&ui, "Sheet 1, front"));
    ui.click_ribbon("printnav:next");
    assert!(has_label(&ui, "Sheet 1, back"));
    assert_eq!(drawn_pages(&ui), vec![2]);
    ui.click_ribbon("printnav:next");
    ui.click_ribbon("printnav:next");
    assert!(has_label(&ui, "Sheet 2, back"));
    assert!(drawn_pages(&ui).is_empty(), "the padded back is blank");
    assert!(!ui.ribbon("printnav:next").enabled);
}

#[test]
fn the_printers_paper_size_drives_the_preview_and_the_layout() {
    use crate::printing::PrinterCaps;
    // Legal paper (8.5 x 14 in): a business-card grid gets another row.
    crate::printing::test_os::set_caps("Test Inkjet", PrinterCaps { paper: Some((612.0, 1008.0)), duplex: true });
    let mut ui = sized_doc(3.5, 2.0, 1);
    file_entry(&mut ui, "Print");
    assert_eq!(ui.app().print_paper(), (612.0, 1008.0));
    assert_eq!(drawn_pages(&ui).len(), 12, "2 x 6 cards on legal, not 2 x 5 on letter");
    assert!(has_label_containing(&ui, "Paper: 8.5 x 14\""));
    let sheet = ui.ribbon("printsheet").rect;
    assert!((sheet.height() / sheet.width() - 1008.0 / 612.0).abs() < 0.01);
    crate::printing::test_os::take_printed();
    ui.click_ribbon("print:Print");
    let jobs = crate::printing::test_os::take_printed();
    assert_eq!(jobs[0].sides[0].paper, (612.0, 1008.0));
    assert_eq!(jobs[0].sides[0].placements.len(), 12);
    // Restore the default for later tests on this thread.
    crate::printing::test_os::set_caps("Test Inkjet", PrinterCaps { paper: Some((612.0, 792.0)), duplex: true });
}

#[test]
fn an_unknown_paper_size_falls_back_to_letter() {
    use crate::printing::PrinterCaps;
    crate::printing::test_os::set_caps("Test Inkjet", PrinterCaps { paper: None, duplex: false });
    let mut ui = pages_doc(1);
    file_entry(&mut ui, "Print");
    assert_eq!(ui.app().print_paper(), (612.0, 792.0));
    crate::printing::test_os::set_caps("Test Inkjet", PrinterCaps { paper: Some((612.0, 792.0)), duplex: true });
}
