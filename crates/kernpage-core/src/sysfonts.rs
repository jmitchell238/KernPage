//! Finds the Windows fonts the programs draw their own text with (the UI and the page fallback).
//!
//! Nothing is bundled on Windows: the fonts are read from the Windows fonts folder at start-up.
//! On a computer without them (a non-Windows development machine) [`embedded_fallback`] gives one
//! small GPL-compatible font (Hack, MIT) so egui always has something to draw with.

use std::path::PathBuf;

/// The font files found on this computer, by role. Each list is in priority order.
#[derive(Clone, Debug, Default)]
pub struct SystemFonts {
    /// Segoe UI, the UI font.
    pub ui: Option<PathBuf>,
    /// Consolas, for monospace text.
    pub mono: Option<PathBuf>,
    /// Segoe UI Symbol, then Segoe UI Emoji: arrows, check marks and other symbols.
    pub symbols: Vec<PathBuf>,
    /// Arial, then Segoe UI: what page text falls back to when its own font is missing.
    pub page: Vec<PathBuf>,
}

/// The Windows fonts folder.
pub fn fonts_dir() -> PathBuf {
    let win = std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    win.join("Fonts")
}

fn find_in(dir: &std::path::Path, file: &str) -> Option<PathBuf> {
    let p = dir.join(file);
    p.is_file().then_some(p)
}

/// Looks for the fonts in the Windows fonts folder.
pub fn locate() -> SystemFonts {
    locate_in(&fonts_dir())
}

/// Looks for the fonts in `dir` (tests point this at a temporary folder).
pub fn locate_in(dir: &std::path::Path) -> SystemFonts {
    let one = |f: &str| find_in(dir, f);
    let many = |fs: &[&str]| fs.iter().filter_map(|f| one(f)).collect::<Vec<_>>();
    SystemFonts {
        ui: one("segoeui.ttf"),
        mono: one("consola.ttf"),
        symbols: many(&["seguisym.ttf", "seguiemj.ttf"]),
        page: many(&["arial.ttf", "segoeui.ttf"]),
    }
}

/// A small font for computers that have none of the Windows fonts. `None` on Windows, where
/// nothing is embedded.
pub fn embedded_fallback() -> Option<&'static [u8]> {
    #[cfg(windows)]
    {
        None
    }
    #[cfg(not(windows))]
    {
        // Hack Regular: MIT (Source Foundry), with DejaVu and Bitstream Vera parts; see
        // assets/fonts/Hack-Regular.txt and THIRD_PARTY_NOTICES.md.
        Some(include_bytes!("../assets/fonts/Hack-Regular.ttf"))
    }
}

/// Key of the UI font (Segoe UI) in a [`FontPlan`].
pub const UI_FONT_KEY: &str = "ui-segoe";

/// Font data and the font lists of each family, ready to hand to egui (this crate has no GUI code).
#[derive(Clone, Debug, Default)]
pub struct FontPlan {
    /// Key and file bytes of every font to load.
    pub data: Vec<(String, std::borrow::Cow<'static, [u8]>)>,
    /// Keys for the UI (proportional) family, in priority order.
    pub proportional: Vec<String>,
    /// Keys for the monospace family.
    pub monospace: Vec<String>,
    /// Keys for page text that falls back from its own font: Arial, Segoe UI, symbols.
    pub page: Vec<String>,
}

/// Reads the fonts in `found` and arranges them for the three families. Fonts that cannot be read
/// are skipped. When nothing was found, the embedded font (non-Windows only) is the only one.
pub fn plan_from(found: &SystemFonts) -> FontPlan {
    let mut plan = FontPlan::default();
    let load = |plan: &mut FontPlan, key: &str, path: &std::path::Path| -> bool {
        match std::fs::read(path) {
            Ok(bytes) => {
                plan.data.push((key.to_owned(), bytes.into()));
                true
            }
            Err(_) => false,
        }
    };
    let (mut ui, mut mono, mut symbols, mut page) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    if let Some(p) = &found.ui {
        if load(&mut plan, UI_FONT_KEY, p) {
            ui.push(UI_FONT_KEY.to_owned());
            page.push(UI_FONT_KEY.to_owned());
        }
    }
    if let Some(p) = &found.mono {
        if load(&mut plan, "mono-consolas", p) {
            mono.push("mono-consolas".to_owned());
        }
    }
    for (i, p) in found.symbols.iter().enumerate() {
        let k = format!("symbols-{i}");
        if load(&mut plan, &k, p) {
            symbols.push(k);
        }
    }
    for (i, p) in found.page.iter().enumerate() {
        // Segoe UI is already loaded as the UI font.
        if found.ui.as_ref() == Some(p) {
            continue;
        }
        let k = format!("page-{i}");
        if load(&mut plan, &k, p) {
            page.insert(0, k);
        }
    }
    let mut embedded = Vec::new();
    if let Some(bytes) = embedded_fallback() {
        plan.data.push(("embedded-hack".to_owned(), bytes.into()));
        embedded.push("embedded-hack".to_owned());
    }
    // UI text: Segoe UI, or Arial when it is missing, then symbols, then the embedded font.
    plan.proportional = if ui.is_empty() { page.clone() } else { ui.clone() };
    plan.proportional.extend(symbols.iter().cloned());
    plan.proportional.extend(embedded.iter().cloned());
    plan.monospace = mono;
    plan.monospace.extend(symbols.iter().cloned());
    plan.monospace.extend(ui);
    plan.monospace.extend(embedded.iter().cloned());
    plan.page = page;
    plan.page.extend(symbols);
    plan.page.extend(embedded);
    plan
}

/// The plan for this computer's Windows fonts.
pub fn plan() -> FontPlan {
    plan_from(&locate())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_folder_finds_nothing() {
        let dir = std::env::temp_dir().join("kernpage-sysfonts-empty");
        std::fs::create_dir_all(&dir).unwrap();
        let f = locate_in(&dir);
        assert!(f.ui.is_none() && f.mono.is_none() && f.symbols.is_empty() && f.page.is_empty());
    }

    #[test]
    fn files_are_found_in_priority_order() {
        let dir = std::env::temp_dir().join("kernpage-sysfonts-some");
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["segoeui.ttf", "arial.ttf", "seguiemj.ttf", "seguisym.ttf"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let f = locate_in(&dir);
        assert!(f.ui.is_some() && f.mono.is_none());
        let names = |v: &[PathBuf]| v.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(names(&f.symbols), ["seguisym.ttf", "seguiemj.ttf"]);
        assert_eq!(names(&f.page), ["arial.ttf", "segoeui.ttf"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_embeds_nothing() {
        assert_eq!(embedded_fallback().is_none(), cfg!(windows));
    }
}
