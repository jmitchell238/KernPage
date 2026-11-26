//! Responsive layout: nothing clipped at the default, a medium and the smallest window size,
//! and the light / dark palettes.

use super::support::*;
use crate::config::Appearance;
use eframe::egui::Rect;

const SIZES: [(f32, f32); 3] = [(1093.0, 798.0), (900.0, 650.0), (760.0, 520.0)];

fn inside(win: Rect, r: Rect) -> bool {
    r.min.x >= win.min.x - 0.5 && r.max.x <= win.max.x + 0.5 && r.min.y >= win.min.y - 0.5 && r.max.y <= win.max.y + 0.5
}

/// Nothing sticks out to the left or right of the window, and the controls named in `pinned`
/// (a modal's footer buttons, which do not scroll) are wholly inside it.
fn assert_fits(ui: &TestUi, what: &str, pinned: &[&str]) {
    let win = ui.window();
    for (l, r) in ui.controls() {
        assert!(r.min.x >= -0.5 && r.max.x <= win.max.x + 0.5, "{what}: {l:?} is cut off horizontally: {r:?} in {win:?}");
    }
    for l in pinned {
        let all: Vec<Rect> = ui.controls().into_iter().filter(|(x, _)| x == l).map(|(_, r)| r).collect();
        let r = all.last().unwrap_or_else(|| panic!("{what}: no {l:?}"));
        assert!(inside(win, *r), "{what}: {l:?} is outside the window: {r:?} in {win:?}");
    }
}

fn dashboard_at(w: f32, h: f32) -> (FeedServer, TestUi) {
    let env = TestEnv::new();
    let long = env.pub_file(&format!("{}.pub", "A very long publication name that must be cut with dots ".repeat(3)));
    let gone = env.files.join("missing.pub");
    let setup = Setup::new().env(&env).size(w, h).installed(OLD).recent(&[long, gone]);
    checked(FeedOpts::default(), setup)
}

#[test]
fn dashboard_fits_at_every_size() {
    for (w, h) in SIZES {
        let (_f, mut ui) = dashboard_at(w, h);
        let win = ui.window();
        assert_fits(&ui, &format!("dashboard {w}x{h}"), &["Settings", "Refresh", "Open file..."]);
        // The cards keep their buttons inside themselves.
        let card = ui.card_rect(0);
        for l in ["OPEN 0.1.2", "UPDATE TO 0.2.0"] {
            let r = ui.rect(l);
            assert!(card.contains_rect(r), "{w}x{h}: {l} inside the card: {r:?} vs {card:?}");
        }
        assert!(card.max.x <= win.max.x - 24.0 + 0.5, "{w}x{h}: the card keeps the page margin");
        // Recent rows stay within the window and the name is cut, not run over the date.
        for (name, _, _, r) in ui.rows() {
            assert!(r.max.x <= win.max.x - 24.0 + 0.5 && r.min.x >= 0.0, "{w}x{h}: row {name:?} {r:?}");
        }
        // The add-on card is reachable by scrolling.
        ui.node("INSTALL").scroll_to_me();
        ui.settle();
        assert!(ui.has("INSTALL"));
    }
}

#[test]
fn notice_and_report_bars_fit_at_every_size() {
    for (w, h) in SIZES {
        let r = crate::install::InstallResult {
            target_version: "0.2.0".into(),
            ok: false,
            exit_code: Some(1603),
            message: "The update could not be installed (error 1603). Windows put your current version back, so KernPage still works as before.".into(),
            log_path: r"C:\logs\x.log".into(),
            finished_at: 1,
        };
        let mut ui = TestUi::with(Setup::new().size(w, h).report(r));
        ui.refresh(); // the dead feed adds a notice too
        assert!(ui.has_text("Could not reach the update server"));
        assert_fits(&ui, &format!("bars {w}x{h}"), &["Dismiss", "Show log"]);
        let all = ui.controls();
        let dismiss: Vec<_> = all.iter().filter(|(l, _)| l == "Dismiss").collect();
        assert_eq!(dismiss.len(), 2);
    }
}

#[test]
fn details_fits_at_every_size() {
    for (w, h) in SIZES {
        let (_f, mut ui) = checked(FeedOpts { real: false, ..Default::default() }, Setup::new().size(w, h).installed(OLD));
        ui.open_more();
        ui.click("What's new");
        assert_fits(&ui, &format!("details {w}x{h}"), &[]);
        let win = ui.window();
        let close = ui.controls().into_iter().rfind(|(l, _)| l == "\u{00D7}").unwrap().1;
        assert!(inside(win, close), "{w}x{h}: the close button is on screen: {close:?}");
    }
}

#[test]
fn settings_fits_at_every_size_and_keeps_its_buttons_on_screen() {
    for (w, h) in SIZES {
        let mut ui = TestUi::with(Setup::new().size(w, h));
        ui.click("Settings");
        assert_fits(&ui, &format!("settings {w}x{h}"), &["Done", "Quit launcher"]);
    }
}

#[test]
fn announce_and_stub_and_confirm_dialogs_fit_at_every_size() {
    for (w, h) in SIZES {
        let (_f, mut ui) = checked_raw(FeedOpts::default(), Setup::new().size(w, h).installed(OLD));
        assert_fits(&ui, &format!("announce {w}x{h}"), &["Later", "Update and restart", "What's new"]);
        ui.click("Later");
        ui.click_last("INSTALL");
        assert_fits(&ui, &format!("stub {w}x{h}"), &["OK"]);
        ui.click("OK");
        let mut ui2 = TestUi::with(Setup::new().size(w, h).recent(&[TestEnv::new().pub_file("a.pub")]));
        ui2.click_last("Clear list");
        assert_fits(&ui2, &format!("confirm {w}x{h}"), &["Cancel"]);
    }
}

#[test]
fn rollback_dialogs_fit_at_every_size() {
    for (w, h) in SIZES {
        let (_f, mut ui) = checked(FeedOpts { older: Some("0.0.9".into()), ..Default::default() }, Setup::new().size(w, h).installed(OLD));
        ui.open_more();
        ui.click("Roll back");
        assert_fits(&ui, &format!("rollback list {w}x{h}"), &["Close", "Go back to this version"]);
        ui.click("Go back to this version");
        assert_fits(&ui, &format!("rollback confirm {w}x{h}"), &["Cancel", "Go back and restart"]);
    }
}

#[test]
fn the_dashboard_scrolls_to_reach_everything_at_the_smallest_size() {
    let (_f, mut ui) = dashboard_at(760.0, 520.0);
    for l in ["Clear list", "INSTALL"] {
        ui.node(l).scroll_to_me();
        ui.settle();
        let r = ui.rect(l);
        assert!(inside(ui.window(), r), "{l} can be scrolled into view: {r:?}");
    }
}

#[test]
fn light_and_dark_palettes_apply() {
    for (cfg, theme) in [("appearance = \"dark\"
", eframe::egui::Theme::Dark), ("appearance = \"light\"
", eframe::egui::Theme::Light)] {
        let mut ui = TestUi::with(Setup::new().config(cfg));
        assert_eq!(ui.app().cfg.appearance, if theme == eframe::egui::Theme::Dark { Appearance::Dark } else { Appearance::Light });
        // (The harness sets its own theme after building the app, so apply the setting again.)
        let ctx = ui.h.ctx.clone();
        ui.app().apply_appearance(&ctx);
        ui.settle();
        assert_eq!(ui.h.ctx.theme(), theme);
        let pal = crate::theme::Palette::for_theme(theme);
        assert_eq!(pal.dark, theme == eframe::egui::Theme::Dark);
        assert_eq!(ui.h.ctx.global_style().visuals.dark_mode, pal.dark);
        assert_fits(&ui, "themed dashboard", &["Refresh"]);
    }
}
