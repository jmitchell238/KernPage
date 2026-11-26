//! The KernPage logo drawn in the app: the K mark or the wordmark, transparent, in the current
//! accent colour and the light or dark variant of the current theme. The SVGs are the static
//! files from `brand/` (made by brand/make_brand.py), rasterised at the exact device-pixel size.

use crate::theme::{self, Accent};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// The K alone.
    K,
    /// The full "Kernpage" wordmark.
    Wordmark,
}

macro_rules! svg {
    ($a:literal, $k:literal) => {
        include_str!(concat!("../../../brand/", $a, "/kernpage-", $k, ".svg"))
    };
}

/// The static SVG source for a mark in an accent, for light or dark backgrounds.
pub fn svg(mark: Mark, accent: Accent, dark: bool) -> &'static str {
    match (accent, mark, dark) {
        (Accent::Teal, Mark::K, false) => svg!("teal", "k-light"),
        (Accent::Teal, Mark::K, true) => svg!("teal", "k-dark"),
        (Accent::Teal, Mark::Wordmark, false) => svg!("teal", "wordmark-light"),
        (Accent::Teal, Mark::Wordmark, true) => svg!("teal", "wordmark-dark"),
        (Accent::Blue, Mark::K, false) => svg!("blue", "k-light"),
        (Accent::Blue, Mark::K, true) => svg!("blue", "k-dark"),
        (Accent::Blue, Mark::Wordmark, false) => svg!("blue", "wordmark-light"),
        (Accent::Blue, Mark::Wordmark, true) => svg!("blue", "wordmark-dark"),
        (Accent::Purple, Mark::K, false) => svg!("purple", "k-light"),
        (Accent::Purple, Mark::K, true) => svg!("purple", "k-dark"),
        (Accent::Purple, Mark::Wordmark, false) => svg!("purple", "wordmark-light"),
        (Accent::Purple, Mark::Wordmark, true) => svg!("purple", "wordmark-dark"),
        (Accent::Graphite, Mark::K, false) => svg!("graphite", "k-light"),
        (Accent::Graphite, Mark::K, true) => svg!("graphite", "k-dark"),
        (Accent::Graphite, Mark::Wordmark, false) => svg!("graphite", "wordmark-light"),
        (Accent::Graphite, Mark::Wordmark, true) => svg!("graphite", "wordmark-dark"),
        (Accent::Orange, Mark::K, false) => svg!("orange", "k-light"),
        (Accent::Orange, Mark::K, true) => svg!("orange", "k-dark"),
        (Accent::Orange, Mark::Wordmark, false) => svg!("orange", "wordmark-light"),
        (Accent::Orange, Mark::Wordmark, true) => svg!("orange", "wordmark-dark"),
    }
}

/// The accent the current palette was built from.
fn current_accent() -> Accent {
    let base = theme::pal().accent;
    Accent::ALL.into_iter().find(|a| a.base() == base).unwrap_or_default()
}

/// Width / height of a mark.
fn aspect(mark: Mark) -> f32 {
    match mark {
        Mark::K => 780.0 / 914.0,
        Mark::Wordmark => 1083.886 / 274.0,
    }
}

/// The mark rasterised `width_px` wide.
pub fn image(mark: Mark, accent: Accent, dark: bool, width_px: u32) -> ColorImage {
    kernicons::rasterise_svg(svg(mark, accent, dark), width_px).expect("the brand SVGs are valid")
}

/// Test seam: what the last frames drew, as (mark, accent, dark).
#[cfg(test)]
pub mod probe {
    use super::*;
    use std::cell::RefCell;
    thread_local! {
        static DREW: RefCell<Vec<(Mark, Accent, bool)>> = const { RefCell::new(Vec::new()) };
    }
    pub fn clear() {
        DREW.with(|d| d.borrow_mut().clear());
    }
    pub fn note(m: Mark, a: Accent, dark: bool) {
        DREW.with(|d| d.borrow_mut().push((m, a, dark)));
    }
    pub fn last(m: Mark) -> Option<(Accent, bool)> {
        DREW.with(|d| d.borrow().iter().rev().find(|e| e.0 == m).map(|e| (e.1, e.2)))
    }
}

/// Draw `mark` `height` points tall (its width follows), in the current accent and theme mode.
pub fn show(ui: &mut Ui, mark: Mark, height: f32) -> egui::Response {
    let size = Vec2::new(height * aspect(mark), height);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let (accent, dark) = (current_accent(), theme::is_dark());
    #[cfg(test)]
    probe::note(mark, accent, dark);
    let ppp = ui.ctx().pixels_per_point();
    let px = ((rect.width() * ppp).round() as u32).max(1);
    #[allow(deprecated)]
    let key = egui::Id::new(("kernpage-logo", mark, accent as u8, dark, px));
    let tex = match ui.ctx().data(|d| d.get_temp::<TextureHandle>(key)) {
        Some(t) => t,
        None => {
            let t = ui.ctx().load_texture(format!("kp-logo-{mark:?}-{accent:?}-{dark}-{px}"), image(mark, accent, dark, px), TextureOptions::LINEAR);
            ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
            t
        }
    };
    ui.painter().image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    resp
}
