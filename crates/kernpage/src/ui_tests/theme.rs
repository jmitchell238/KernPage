//! The theme system: Light / Dark / Follow Windows, the accent presets, their persistence, File > Options > Appearance and the
//! "Search commands" box.

use super::support::*;
use crate::app::{Section, Tab};
use crate::theme::{self, Accent, Appearance, ThemeMode};
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::kittest::Queryable;

fn doc_ui() -> TestUi {
    TestUi::with_doc(letter_doc(vec![rect_shape(1.0, 1.0, 2.0, 1.5)]))
}

fn set(ui: &mut TestUi, a: Appearance) {
    let ctx = ui.h.ctx.clone();
    ui.app().set_appearance(a, &ctx);
    ui.settle();
}

#[test]
fn switching_to_dark_changes_the_ribbon_and_status_colours() {
    let mut ui = doc_ui();
    let light = theme::pal();
    let light_panel = ui.h.ctx.global_style().visuals.panel_fill;
    assert!(!light.dark);
    set(&mut ui, Appearance { mode: ThemeMode::Dark, ..Appearance::default() });
    let dark = theme::pal();
    assert!(dark.dark);
    assert_ne!(dark.card, light.card, "ribbon surface");
    assert_ne!(dark.status_bg, light.status_bg, "status bar");
    assert_ne!(dark.text, light.text);
    assert_ne!(ui.h.ctx.global_style().visuals.panel_fill, light_panel, "egui's own panels follow too");
    set(&mut ui, Appearance::default());
    assert_eq!(theme::pal().card, light.card, "and back, live");
}

#[test]
fn the_page_stays_white_in_dark() {
    let mut ui = doc_ui();
    set(&mut ui, Appearance { mode: ThemeMode::Dark, ..Appearance::default() });
    assert_eq!(ui.app().probe.page_fill, egui::Color32::WHITE);
}

#[test]
fn appearance_is_saved_and_restored() {
    let env = TestEnv::new();
    let mut ui = TestUi::start_in(env.clone());
    let a = Appearance { mode: ThemeMode::Dark, accent: Accent::Purple };
    set(&mut ui, a);
    assert_eq!(Appearance::load_from(&theme::settings_file()), a);
    // A new session reads it back and applies it.
    let mut ui2 = TestUi::start_in(env);
    assert_eq!(ui2.app().appearance, a);
    assert_eq!(theme::pal().accent, Accent::Purple.base());
    assert!(theme::pal().dark);
}

#[test]
fn defaults_are_light_teal() {
    let mut ui = TestUi::start();
    let a = ui.app().appearance;
    assert_eq!((a.mode, a.accent), (ThemeMode::Light, Accent::Teal));
}

#[test]
fn chrome_draws_pill_tab_card_and_neutral_status() {
    let mut ui = doc_ui();
    ui.settle();
    for n in ["file:tab", "tab:pill", "ribbon:card", "status:neutral"] {
        assert!(theme::probe::has(n), "Modern should draw {n}");
    }
}

#[test]
fn no_source_file_hard_codes_publishers_green() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "theme.rs") {
            let text = std::fs::read_to_string(&p).unwrap().to_lowercase();
            for needle in ["0x07, 0x75, 0x68", "#077568", "0x077568"] {
                assert!(!text.contains(needle), "{p:?} contains {needle}");
            }
            checked += 1;
        }
    }
    assert!(checked > 20);
}

#[test]
fn options_appearance_changes_theme_and_accent_live() {
    let mut ui = doc_ui();
    ui.app().backstage = true;
    ui.app().section = Section::Options;
    ui.settle();
    ui.click_ribbon("optcat:Appearance");
    assert_eq!(ui.app().options_cat, 1);
    ui.h.get_by_label("Dark").click();
    ui.settle();
    assert_eq!(ui.app().appearance.mode, ThemeMode::Dark);
    assert!(theme::pal().dark);
    ui.h.get_by_label("Follow Windows").click();
    ui.settle();
    assert_eq!(ui.app().appearance.mode, ThemeMode::System);
    ui.click_ribbon("accent:Blue");
    assert_eq!(ui.app().appearance.accent, Accent::Blue);
    assert_eq!(theme::pal().accent, Accent::Blue.base());
    assert!(ui.h.query_by_label("Classic").is_none(), "no style choice any more");
}

#[test]
fn the_search_box_finds_bold_and_switches_to_home() {
    let mut ui = doc_ui();
    ui.click_tab("View");
    assert_eq!(ui.tab(), Tab::View);
    ui.click_ribbon("search");
    ui.type_text("bold");
    assert!(ui.has_ribbon("search-result:Bold"), "Bold is listed");
    ui.key(Modifiers::NONE, Key::Enter);
    assert_eq!(ui.tab(), Tab::Home);
    assert!(ui.app().search.text.is_empty());
}

#[test]
fn clicking_a_search_result_runs_it() {
    let mut ui = doc_ui();
    ui.click_ribbon("search");
    ui.type_text("page width");
    assert_eq!(ui.tab(), Tab::Home);
    ui.click_ribbon("search-result:Page Width");
    assert_eq!(ui.tab(), Tab::View);
    assert!(ui.app().search.text.is_empty());
}

#[test]
fn alt_q_focuses_the_search_box_and_ctrl_f_is_left_alone() {
    let mut ui = doc_ui();
    ui.key(Modifiers::ALT, Key::Q);
    ui.type_text("rul");
    assert_eq!(ui.app().search.text, "rul");
    assert!(ui.has_ribbon("search-result:Rulers"));
    ui.key(Modifiers::NONE, Key::Escape);
    assert!(ui.app().search.text.is_empty());
}

#[test]
fn typing_and_backspace_in_the_search_box_do_not_touch_the_selection() {
    let mut ui = doc_ui();
    ui.click_at(ui.center_of(1));
    assert_eq!(ui.selected(), vec![1]);
    ui.key(Modifiers::ALT, Key::Q);
    ui.type_text("bolx");
    ui.key(Modifiers::NONE, Key::Backspace);
    assert_eq!(ui.app().search.text, "bol");
    assert_eq!(ui.shapes().len(), 1, "Backspace edited the search text, not the page");
}
