//! First-page previews for the File screen's start page: the Recent cards and the Church
//! Bulletin tile. Files are parsed on a background thread so the UI never waits for them; until
//! a file is ready (or if it cannot be read) the caller draws its page glyph instead.

use crate::fontmgr::FontMgr;
use crate::render::PictureCache;
use eframe::egui::{Rect, Ui, Vec2};
use kernpage_core::Document;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};

/// How often a card's file is checked on disk for a change (not once per frame).
#[cfg(not(test))]
const RECHECK: Duration = Duration::from_secs(3);
/// Tests expect a change to show at once.
#[cfg(test)]
const RECHECK: Duration = Duration::ZERO;

enum Thumb {
    Loading,
    Ready(Box<Document>),
    Failed,
}

type Key = Option<SystemTime>;
type Loaded = (PathBuf, Key, Option<Document>);

pub struct StartThumbs {
    map: HashMap<PathBuf, (Key, Thumb)>,
    /// When each file's modified time was last read.
    checked: HashMap<PathBuf, Instant>,
    tx: Sender<Loaded>,
    rx: Receiver<Loaded>,
    /// Its own picture cache, so the start screen and the page view do not evict each other.
    /// Textures are keyed by the picture data's address, which stays valid while the document
    /// is cached here; the cache is cleared whenever a cached document is replaced.
    pics: PictureCache,
    bulletin: Option<Document>,
    loading: usize,
}

impl Default for StartThumbs {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { map: HashMap::new(), checked: HashMap::new(), tx, rx, pics: PictureCache::default(), bulletin: None, loading: 0 }
    }
}

/// The largest rectangle with the page's aspect ratio that fits `area`, centred in it.
fn fit(area: Rect, doc: &Document) -> Option<Rect> {
    let page = doc.pages.first()?;
    let (w, h) = (page.width_emu as f32, page.height_emu as f32);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let scale = (area.width() / w).min(area.height() / h);
    Some(Rect::from_center_size(area.center(), Vec2::new(w * scale, h * scale)))
}

impl StartThumbs {
    fn poll(&mut self) {
        while let Ok((path, key, doc)) = self.rx.try_recv() {
            self.loading = self.loading.saturating_sub(1);
            // Ignore a stale answer for a file that has changed again since.
            let Some(entry) = self.map.get_mut(&path) else { continue };
            if entry.0 != key {
                continue;
            }
            entry.1 = match doc {
                Some(d) => Thumb::Ready(Box::new(d)),
                None => Thumb::Failed,
            };
        }
    }

    /// Draw page 1 of the file at `path` into `area` (fitted, bordered); false when it is not
    /// ready, so the caller draws its glyph. Starts the load on first sight or after a change.
    pub fn paint_file(&mut self, ui: &Ui, area: Rect, path: &Path, fonts: &FontMgr) -> bool {
        if crate::demo::is_on() {
            return false; // the made-up files do not exist
        }
        self.poll();
        let due = self.checked.get(path).is_none_or(|t| t.elapsed() >= RECHECK) || !self.map.contains_key(path);
        let key: Key = if due {
            self.checked.insert(path.to_path_buf(), Instant::now());
            std::fs::metadata(path).and_then(|m| m.modified()).ok()
        } else {
            self.map.get(path).and_then(|(k, _)| *k)
        };
        let stale = due && self.map.get(path).is_none_or(|(k, _)| *k != key);
        if stale {
            if self.map.insert(path.to_path_buf(), (key, Thumb::Loading)).is_some() {
                // The old document's pictures are keyed by addresses that may be reused.
                self.pics.clear();
            }
            let (tx, ctx, p) = (self.tx.clone(), ui.ctx().clone(), path.to_path_buf());
            self.loading += 1;
            std::thread::spawn(move || {
                let doc = if key.is_some() { crate::app::load_document(&p).ok() } else { None };
                let _ = tx.send((p, key, doc));
                ctx.request_repaint();
            });
        }
        let Some((_, Thumb::Ready(doc))) = self.map.get(path) else { return false };
        let Some(r) = fit(area, doc) else { return false };
        let ppp = ui.ctx().pixels_per_point();
        let r = crate::widgets::snap_rect(ppp, r);
        crate::nav::paint_thumb(ui, r, doc, 0, fonts, &mut self.pics);
        // One crisp device pixel, on the page's own edge.
        ui.painter().rect_stroke(
            r,
            0.0,
            eframe::egui::Stroke::new(1.0 / ppp, crate::theme::frame()),
            eframe::egui::StrokeKind::Inside,
        );
        true
    }

    /// Draw page 1 of the Church Bulletin template into `area` (built once, then cached).
    pub fn paint_bulletin(&mut self, ui: &Ui, area: Rect, fonts: &FontMgr) -> bool {
        let doc = self.bulletin.get_or_insert_with(kernpage_core::template::church_bulletin);
        let Some(r) = fit(area, doc) else { return false };
        crate::nav::paint_thumb(ui, r, doc, 0, fonts, &mut self.pics);
        true
    }

    /// Test seam: block until every started load has finished.
    #[cfg(test)]
    pub fn wait_idle(&mut self) {
        let start = std::time::Instant::now();
        while self.loading > 0 && start.elapsed().as_secs() < 30 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.poll();
        }
        self.poll();
    }
}
