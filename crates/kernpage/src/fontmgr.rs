//! Loads the system TrueType fonts that the document uses into egui.

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use kernpage_core::sysfonts;
use pub2pdf::Fonts;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

pub struct FontMgr {
    rx: Option<mpsc::Receiver<Fonts>>,
    /// The scan thread; it sends its result and then asks for one repaint before ending.
    #[cfg_attr(not(test), allow(dead_code))]
    scan_thread: Option<std::thread::JoinHandle<()>>,
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

/// Family for page text that falls back from its own font: Arial, then Segoe UI, then symbols.
/// The UI font is Segoe UI when installed, and the page must not change with it.
pub const PAGE_FALLBACK: &str = "pw-page-fallback";

/// Name of the UI font's data in the font definitions (tests check it comes first).
#[cfg(test)]
pub const UI_FONT_KEY: &str = sysfonts::UI_FONT_KEY;

/// The system's Segoe UI (Publisher's UI font); never bundled, it is not ours to ship.
#[cfg(test)]
pub fn ui_font_file() -> Option<PathBuf> {
    sysfonts::locate().ui
}

/// The fonts egui starts with. Nothing is bundled on Windows: they all come from the Windows
/// fonts folder (Segoe UI, Consolas, Segoe UI Symbol and Emoji, Arial). Without any of them
/// (a non-Windows development machine) one small embedded font is used so egui can still draw text.
pub fn base_definitions() -> FontDefinitions {
    definitions_from(&sysfonts::plan())
}

/// Turns a font plan into egui font definitions.
pub fn definitions_from(plan: &sysfonts::FontPlan) -> FontDefinitions {
    let mut defs = FontDefinitions::empty();
    for (k, bytes) in &plan.data {
        let data = match bytes {
            std::borrow::Cow::Borrowed(b) => FontData::from_static(b),
            std::borrow::Cow::Owned(v) => FontData::from_owned(v.clone()),
        };
        defs.font_data.insert(k.clone(), Arc::new(data));
    }
    defs.families.insert(FontFamily::Proportional, plan.proportional.clone());
    defs.families.insert(FontFamily::Monospace, plan.monospace.clone());
    defs.families
        .insert(FontFamily::Name(PAGE_FALLBACK.into()), plan.page.clone());
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
        let scan_thread = std::thread::spawn(move || {
            let dirs = pub2pdf::default_font_dirs();
            let _ = tx.send(Fonts::scan(&dirs));
            ctx.request_repaint();
        });
        FontMgr {
            rx: Some(rx),
            scan_thread: Some(scan_thread),
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

    /// `true` once the scan thread has ended, so its final repaint request has been made
    /// (tests wait for this before checking that the window is idle).
    #[cfg(test)]
    pub fn scan_thread_ended(&self) -> bool {
        self.scan_thread.as_ref().is_none_or(|h| h.is_finished())
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
