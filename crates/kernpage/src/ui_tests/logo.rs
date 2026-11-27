//! The KernPage logo in the app: the white wordmark on the File screen's rail, the wordmark on the
//! About page and the K in the About box,
//! in the current accent colour and the light or dark variant of the current theme.

use super::support::*;
use crate::app::Section;
use crate::logo::{self, Mark};
use crate::theme::{Accent, Appearance, ThemeMode};
use eframe::egui::Color32;

fn set(ui: &mut TestUi, a: Appearance) {
    let ctx = ui.h.ctx.clone();
    ui.app().set_appearance(a, &ctx);
    logo::probe::clear();
    ui.settle();
}

fn has_colour(img: &eframe::egui::ColorImage, c: Color32) -> bool {
    img.pixels.iter().any(|p| p.a() == 255 && (p.r(), p.g(), p.b()) == (c.r(), c.g(), c.b()))
}

#[test]
fn the_home_page_shows_the_wordmark_in_the_default_teal_light() {
    let mut ui = TestUi::start();
    assert!(ui.app().backstage && ui.app().section == Section::Home, "starts on Home");
    // The File screen's teal rail carries the white (dark-background) wordmark, whatever the theme.
    assert_eq!(logo::probe::last(Mark::Wordmark), Some((Accent::Teal, true)));
    assert!(ui.app().error.is_none());
}

#[test]
fn the_wordmark_follows_the_accent_and_dark_mode() {
    let mut ui = TestUi::start();
    // The About page draws the wordmark in the current accent and theme.
    ui.app().section = Section::About;
    for accent in Accent::ALL {
        for (mode, dark) in [(ThemeMode::Light, false), (ThemeMode::Dark, true)] {
            set(&mut ui, Appearance { mode, accent, ..Appearance::default() });
            assert_eq!(logo::probe::last(Mark::Wordmark), Some((accent, dark)), "{accent:?} dark={dark}");
        }
    }
}

#[test]
fn each_accent_variant_is_drawn_in_its_own_colours_on_a_transparent_background() {
    // The light wordmark's letters are the accent's ink colour; the dark one's are white.
    let ink = [
        (Accent::Teal, Color32::from_rgb(0x0B, 0x5A, 0x73)),
        (Accent::Blue, Color32::from_rgb(0x0E, 0x39, 0xC3)),
        (Accent::Purple, Color32::from_rgb(0x6D, 0x2A, 0xBA)),
        (Accent::Graphite, Color32::from_rgb(0x3A, 0x41, 0x50)),
        (Accent::Orange, Color32::from_rgb(0x58, 0x59, 0x5B)),
    ];
    for (accent, c) in ink {
        let light = logo::image(Mark::Wordmark, accent, false, 400);
        assert!(has_colour(&light, c), "{accent:?} light wordmark has its ink {c:?}");
        assert_eq!(light.pixels[0].a(), 0, "transparent corner");
        let dark = logo::image(Mark::Wordmark, accent, true, 400);
        assert!(has_colour(&dark, Color32::WHITE), "{accent:?} dark wordmark has white letters");
        assert!(!has_colour(&dark, c), "{accent:?} dark wordmark does not use the light-background ink");
        let k = logo::image(Mark::K, accent, false, 200);
        assert_eq!(k.pixels[0].a(), 0, "K corner is transparent");
    }
}

#[test]
fn the_about_page_shows_the_wordmark_in_the_current_accent_and_mode() {
    let mut ui = TestUi::start();
    set(&mut ui, Appearance { mode: ThemeMode::Dark, accent: Accent::Purple, ..Appearance::default() });
    ui.app().show_about();
    ui.settle();
    assert_eq!(logo::probe::last(Mark::Wordmark), Some((Accent::Purple, true)));
}
