//! Finds the Windows fonts the programs draw their own text with (the UI and the page fallback).
//!
//! Nothing is compiled into the Windows programs: the fonts are read from the Windows fonts folder at
//! start-up. The one bundled font, Noto Sans (OFL 1.1), is a separate file in the `fonts` folder next to
//! the program; it is the interface font only when Segoe UI is missing. On a computer without them (a non-Windows development machine) [`embedded_fallback`] gives one
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
    /// The bundled Noto Sans file (next to the program), the interface font when Segoe UI is missing.
    pub fallback_ui: Option<PathBuf>,
}

/// File name of the bundled fallback interface font (Noto Sans Regular, unmodified).
pub const FALLBACK_UI_FILE: &str = "NotoSans-Regular.ttf";

/// Folders searched for [`FALLBACK_UI_FILE`]: `fonts` next to the running program (where the installer
/// puts it) and, in debug and test builds only, `assets/fonts/noto-sans` of this repository.
pub fn bundled_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("fonts"))) {
        dirs.push(dir);
    }
    #[cfg(any(test, debug_assertions))]
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/noto-sans"));
    dirs
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

/// Looks for the fonts in the Windows fonts folder and the bundled font next to the program.
pub fn locate() -> SystemFonts {
    locate_with(&fonts_dir(), &bundled_font_dirs())
}

/// Looks for the fonts in `dir` (tests point this at a temporary folder); no bundled font.
pub fn locate_in(dir: &std::path::Path) -> SystemFonts {
    locate_with(dir, &[])
}

/// Looks for the system fonts in `dir` and the bundled Noto Sans in the first of `bundled` that has it.
/// Nothing here fails when a file is missing: the field is simply empty.
pub fn locate_with(dir: &std::path::Path, bundled: &[PathBuf]) -> SystemFonts {
    let one = |f: &str| find_in(dir, f);
    let many = |fs: &[&str]| fs.iter().filter_map(|f| one(f)).collect::<Vec<_>>();
    SystemFonts {
        ui: one("segoeui.ttf"),
        mono: one("consola.ttf"),
        symbols: many(&["seguisym.ttf", "seguiemj.ttf"]),
        page: many(&["arial.ttf", "segoeui.ttf"]),
        fallback_ui: bundled.iter().find_map(|d| find_in(d, FALLBACK_UI_FILE)),
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
/// Key of the bundled Noto Sans in a [`FontPlan`].
pub const FALLBACK_UI_KEY: &str = "ui-noto-sans";

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
    // The bundled Noto Sans; a file that is missing or unreadable is skipped, never an error.
    let mut noto = Vec::new();
    if let Some(p) = &found.fallback_ui {
        if load(&mut plan, FALLBACK_UI_KEY, p) {
            noto.push(FALLBACK_UI_KEY.to_owned());
        }
    }
    let mut embedded = Vec::new();
    if let Some(bytes) = embedded_fallback() {
        plan.data.push(("embedded-hack".to_owned(), bytes.into()));
        embedded.push("embedded-hack".to_owned());
    }
    // UI text. With Segoe UI: Segoe UI, symbols, then Noto Sans (it only supplies glyphs the others lack,
    // so Segoe UI text is drawn exactly as before: egui takes the line height from the first font only),
    // then the embedded font. Without Segoe UI: Noto Sans, then Arial, symbols, the embedded font.
    // Without Noto Sans either, this is the old plan (Arial, symbols, embedded).
    if ui.is_empty() {
        plan.proportional = noto.clone();
        plan.proportional.extend(page.iter().cloned());
        plan.proportional.extend(symbols.iter().cloned());
    } else {
        plan.proportional = ui.clone();
        plan.proportional.extend(symbols.iter().cloned());
        plan.proportional.extend(noto.iter().cloned());
    }
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

    fn scratch(name: &str) -> PathBuf {
        // A folder of its own per call: the tests run in parallel.
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("kernpage-sysfonts-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn found(with_segoe: bool, noto: Option<&std::path::Path>) -> SystemFonts {
        let sys = scratch(if with_segoe { "sys-segoe" } else { "sys-nosegoe" });
        let mut files = vec!["arial.ttf", "seguisym.ttf"];
        if with_segoe {
            files.push("segoeui.ttf");
        }
        for n in files {
            std::fs::write(sys.join(n), n.as_bytes()).unwrap();
        }
        let bundled: Vec<PathBuf> = noto.map(|p| vec![p.to_path_buf()]).unwrap_or_default();
        locate_with(&sys, &bundled)
    }

    fn bundle() -> PathBuf {
        let d = scratch("bundle");
        std::fs::write(d.join(FALLBACK_UI_FILE), b"noto").unwrap();
        d
    }

    fn bytes_of<'a>(plan: &'a FontPlan, key: &str) -> &'a [u8] {
        &plan.data.iter().find(|(k, _)| k == key).unwrap().1
    }

    #[test]
    fn noto_sans_is_the_ui_font_when_segoe_ui_is_absent() {
        let plan = plan_from(&found(false, Some(&bundle())));
        assert_eq!(plan.proportional.first().map(String::as_str), Some(FALLBACK_UI_KEY));
        assert_eq!(bytes_of(&plan, FALLBACK_UI_KEY), b"noto");
        assert!(!plan.data.iter().any(|(k, _)| k == UI_FONT_KEY));
        // Arial and the symbol fonts still follow it for glyphs Noto Sans lacks.
        assert!(plan.proportional.len() >= 3);
        assert!(plan.proportional.iter().all(|k| plan.data.iter().any(|(d, _)| d == k)));
    }

    #[test]
    fn without_segoe_ui_and_noto_sans_the_plan_is_the_old_one() {
        let plan = plan_from(&found(false, None));
        assert!(!plan.proportional.iter().any(|k| k == FALLBACK_UI_KEY));
        assert_eq!(plan.proportional.first().map(String::as_str), Some("page-0"));
    }

    #[test]
    fn with_segoe_ui_the_plan_only_gains_noto_sans_at_the_end_of_the_ui_list() {
        let without = plan_from(&found(true, None));
        let with = plan_from(&found(true, Some(&bundle())));
        assert_eq!(with.proportional.first().map(String::as_str), Some(UI_FONT_KEY));
        let mut expected = without.proportional.clone();
        expected.push(FALLBACK_UI_KEY.to_owned());
        assert_eq!(with.proportional, expected, "Noto Sans is appended after Segoe UI and the symbols");
        // The monospace and page lists (the document must not change) are exactly as before.
        assert_eq!(with.monospace, without.monospace);
        assert_eq!(with.page, without.page);
        assert!(!with.page.iter().chain(&with.monospace).any(|k| k == FALLBACK_UI_KEY));
    }

    #[test]
    fn a_missing_or_unreadable_noto_file_does_not_panic() {
        // No such folder: nothing found, the plan is the old one.
        let nowhere = std::env::temp_dir().join("kernpage-sysfonts-no-such-folder");
        let f = found(false, Some(&nowhere));
        assert!(f.fallback_ui.is_none());
        let _ = plan_from(&f);
        // A path that is named but cannot be read (a folder where the file should be).
        let mut f = found(false, None);
        f.fallback_ui = Some(scratch("unreadable"));
        let plan = plan_from(&f);
        assert!(!plan.proportional.iter().any(|k| k == FALLBACK_UI_KEY));
        assert!(!plan.data.iter().any(|(k, _)| k == FALLBACK_UI_KEY));
    }

    #[test]
    fn the_bundled_font_is_in_the_repo_for_tests_and_is_noto_sans() {
        let f = locate_with(&std::env::temp_dir().join("kernpage-sysfonts-none"), &bundled_font_dirs());
        let p = f.fallback_ui.expect("assets/fonts/noto-sans/NotoSans-Regular.ttf");
        let len = std::fs::metadata(p).unwrap().len();
        assert!(len > 100_000, "a real font file, {len} bytes");
    }

    #[test]
    fn windows_embeds_nothing() {
        assert_eq!(embedded_fallback().is_none(), cfg!(windows));
    }
}
