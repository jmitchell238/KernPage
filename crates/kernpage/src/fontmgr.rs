//! Loads the system TrueType fonts that the document uses into egui.

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use pub2pdf::Fonts;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

pub struct FontMgr {
    rx: Option<mpsc::Receiver<Fonts>>,
    db: Option<Fonts>,
    /// Sorted family names found on this computer (for the font menu).
    pub families: Vec<String>,
    defs: FontDefinitions,
    data_keys: HashMap<(PathBuf, u32), String>,
    /// Keys whose family is usable in this frame.
    active: HashSet<String>,
    /// Keys handed to egui this frame; usable from the next frame.
    activating: HashSet<String>,
    attempted: HashSet<String>,
}

/// Family holding egui's bundled fonts: what page text falls back to (the UI font is
/// Segoe UI when installed, and the page must not change with it).
pub const PAGE_FALLBACK: &str = "pw-page-fallback";

/// Name of the UI font's data in the font definitions.
pub const UI_FONT_KEY: &str = "ui-segoe";

/// The system's Segoe UI (Publisher's UI font); never bundled, it is not ours to ship.
pub fn ui_font_file() -> Option<PathBuf> {
    let win = std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let p = win.join("Fonts").join("segoeui.ttf");
    p.is_file().then_some(p)
}

/// egui's fonts with Segoe UI first in the proportional (UI) family when it is installed.
pub fn base_definitions() -> FontDefinitions {
    base_definitions_with(ui_font_file())
}

fn base_definitions_with(ui: Option<PathBuf>) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let page = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    defs.families
        .insert(FontFamily::Name(PAGE_FALLBACK.into()), page);
    if let Some(bytes) = ui.and_then(|p| std::fs::read(p).ok()) {
        defs.font_data
            .insert(UI_FONT_KEY.to_owned(), Arc::new(FontData::from_owned(bytes)));
        if let Some(list) = defs.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, UI_FONT_KEY.to_owned());
        }
    }
    defs
}

fn key(family: &str, bold: bool, italic: bool) -> String {
    format!(
        "{}|{}{}",
        family.trim().to_lowercase(),
        u8::from(bold),
        u8::from(italic)
    )
}

impl FontMgr {
    /// Starts scanning the system font folders on a background thread.
    pub fn new(ctx: &Context) -> FontMgr {
        let (tx, rx) = mpsc::channel();
        let defs = base_definitions();
        ctx.set_fonts(defs.clone());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let dirs = pub2pdf::default_font_dirs();
            let _ = tx.send(Fonts::scan(&dirs));
            ctx.request_repaint();
        });
        FontMgr {
            rx: Some(rx),
            db: None,
            families: Vec::new(),
            defs,
            data_keys: HashMap::new(),
            active: HashSet::new(),
            activating: HashSet::new(),
            attempted: HashSet::new(),
        }
    }

    /// `true` once the system font scan has finished.
    pub fn scan_done(&self) -> bool {
        self.db.is_some()
    }

    /// `true` when no font load is waiting to become active.
    pub fn settled(&self) -> bool {
        self.activating.is_empty()
    }

    /// The font to lay text out with; falls back to egui's default when the family is not
    /// installed (or not loaded yet).
    pub fn font_id(&self, family: &str, bold: bool, italic: bool, size_px: f32) -> FontId {
        let k = key(family, bold, italic);
        if self.active.contains(&k) {
            FontId::new(size_px, FontFamily::Name(k.into()))
        } else {
            FontId::new(size_px, FontFamily::Name(PAGE_FALLBACK.into()))
        }
    }

    /// Call at the start of every frame with the fonts the document needs.
    pub fn ensure(&mut self, ctx: &Context, needed: &[(String, bool, bool)]) {
        if let Some(rx) = &self.rx {
            if let Ok(db) = rx.try_recv() {
                self.families = db.family_names();
                self.db = Some(db);
                self.rx = None;
            }
        }
        self.active.extend(self.activating.drain());
        let Some(db) = &self.db else { return };
        let mut changed = false;
        for (family, bold, italic) in needed {
            let k = key(family, *bold, *italic);
            if !self.attempted.insert(k.clone()) {
                continue;
            }
            let Some(face) = db.find(family, *bold, *italic) else {
                continue;
            };
            let id = (face.path.clone(), face.index);
            let data_key = match self.data_keys.get(&id) {
                Some(k) => k.clone(),
                None => {
                    let Ok(bytes) = std::fs::read(&face.path) else {
                        continue;
                    };
                    let dk = format!("{}#{}", face.path.display(), face.index);
                    let mut data = FontData::from_owned(bytes);
                    data.index = face.index;
                    self.defs.font_data.insert(dk.clone(), Arc::new(data));
                    self.data_keys.insert(id, dk.clone());
                    dk
                }
            };
            let mut list = vec![data_key];
            if let Some(fallback) = self.defs.families.get(&FontFamily::Name(PAGE_FALLBACK.into())) {
                list.extend(fallback.iter().cloned());
            }
            self.defs
                .families
                .insert(FontFamily::Name(k.clone().into()), list);
            self.activating.insert(k);
            changed = true;
        }
        if changed {
            ctx.set_fonts(self.defs.clone());
        }
    }
}
