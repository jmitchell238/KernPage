//! Idle CPU (pw-9iee): once the app has settled it must stop asking for immediate repaints, so
//! an idle window uses ~0% CPU. Timers may ask for a repaint later (`repaint_after`), never now.

use super::support::*;

/// Fonts loaded, frames settled, and then no immediate repaint is requested.
fn assert_idle(ui: &mut TestUi, what: &str) {
    ui.wait_for_fonts();
    ui.settle();
    // Ten more frames: each must end without an immediate repaint request.
    for n in 0..10 {
        ui.h.step();
        let causes = ui.h.ctx.repaint_causes();
        assert!(
            !ui.h.ctx.requested_repaint_last_pass(),
            "{what}: asked for an immediate repaint on idle frame {n}: {causes:?}"
        );
    }
}

#[test]
fn start_screen_is_idle() {
    let mut ui = TestUi::start();
    assert_idle(&mut ui, "start screen");
}

#[test]
fn every_file_screen_is_idle() {
    let mut ui = TestUi::with_sample("hello.pub");
    ui.click_tab("File");
    for name in ["Info", "New", "Open", "Print", "Share", "Export", "About KernPage", "Feedback", "Options"] {
        ui.click_ribbon(&format!("nav:{name}"));
        assert_idle(&mut ui, &format!("File > {name}"));
    }
}

#[test]
fn editor_is_idle_on_every_tab() {
    let mut ui = TestUi::with_sample("hello.pub");
    assert_idle(&mut ui, "editor");
    for tab in ui.tab_names() {
        ui.click_tab(&tab);
        assert_idle(&mut ui, &format!("editor, tab {tab}"));
    }
}

#[test]
fn text_editing_is_idle() {
    let mut ui = TestUi::with_sample("hello.pub");
    let id = ui.shapes().iter().map(|s| s.id).find(|&i| !ui.text_of(i).is_empty());
    if let Some(id) = id {
        let p = ui.center_of(id);
        ui.double_click_at(p);
        assert_idle(&mut ui, "editing text");
    }
}

#[test]
fn editor_is_idle_at_other_window_sizes() {
    let mut ui = TestUi::with_sample("showcase-newsletter.pub");
    for (w, h) in [(800.0, 500.0), (1000.0, 700.0), (1920.0, 1080.0), (600.0, 400.0)] {
        ui.h.set_size(eframe::egui::vec2(w, h));
        for tab in ui.tab_names() {
            ui.click_tab(&tab);
            assert_idle(&mut ui, &format!("{w}x{h}, tab {tab}"));
        }
    }
}

#[test]
fn selected_shapes_and_dirty_document_are_idle() {
    let mut ui = TestUi::with_sample("showcase-newsletter.pub");
    let ids: Vec<_> = ui.shapes().iter().map(|s| s.id).collect();
    for id in ids {
        let p = ui.center_of(id);
        ui.click_at(p);
        assert_idle(&mut ui, "a shape selected");
        for tab in ui.tab_names() {
            ui.click_tab(&tab);
            assert_idle(&mut ui, &format!("shape selected, tab {tab}"));
        }
    }
}
