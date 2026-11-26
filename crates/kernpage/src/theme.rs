//! KernPage's own look: a palette for the application chrome (ribbon, tabs, panes, status
//! bar, backstage, dialogs, workspace) that can switch at run time.
//!
//! Light is the default; Dark and "Follow Windows" are chosen in File > Options > Appearance,
//! together with one of five accents (Teal is the default). The publication itself (page,
//! shapes, text colours) is never themed: the page stays white.
//!
//! The current palette lives in a thread-local cell, set once per frame by the app
//! ([`set`]), and is read through the cheap accessors below (`theme::text()` ...). A
//! thread-local keeps UI tests (one thread each) independent of one another.

use eframe::egui::Color32;
use std::cell::Cell;
use std::path::Path;

/// Light, dark, or whatever Windows is set to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    System,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System];

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
            ThemeMode::System => "Follow Windows",
        }
    }

    fn key(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "system",
        }
    }

    pub fn parse(s: &str) -> Option<ThemeMode> {
        ThemeMode::ALL.into_iter().find(|m| m.key().eq_ignore_ascii_case(s.trim()))
    }
}

/// The accent colour presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Accent {
    #[default]
    Teal,
    Blue,
    Purple,
    Graphite,
    Orange,
}

impl Accent {
    pub const ALL: [Accent; 5] =
        [Accent::Teal, Accent::Blue, Accent::Purple, Accent::Graphite, Accent::Orange];

    pub fn label(self) -> &'static str {
        match self {
            Accent::Teal => "Teal",
            Accent::Blue => "Blue",
            Accent::Purple => "Purple",
            Accent::Graphite => "Graphite",
            Accent::Orange => "Orange",
        }
    }

    /// The preset's base colour.
    pub fn base(self) -> Color32 {
        match self {
            Accent::Teal => Color32::from_rgb(0x1E, 0x8C, 0x99),
            Accent::Blue => Color32::from_rgb(0x2F, 0x6F, 0xDB),
            Accent::Purple => Color32::from_rgb(0x7B, 0x55, 0xC8),
            Accent::Graphite => Color32::from_rgb(0x55, 0x60, 0x6E),
            Accent::Orange => Color32::from_rgb(0xFF, 0x82, 0x00),
        }
    }

    /// Text and icons drawn on a solid fill of this accent: white, except on Orange, where
    /// Tennessee "Smokey" grey (#58595B) reads better than white (2.8:1 against 2.5:1).
    pub fn on_fill(self) -> Color32 {
        match self {
            Accent::Orange => Color32::from_rgb(0x58, 0x59, 0x5B),
            _ => Color32::WHITE,
        }
    }

    pub fn parse(s: &str) -> Option<Accent> {
        Accent::ALL.into_iter().find(|a| a.label().eq_ignore_ascii_case(s.trim()))
    }
}

/// The saved appearance choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Appearance {
    pub mode: ThemeMode,
    pub accent: Accent,
}

impl Appearance {
    pub fn to_text(self) -> String {
        format!("Theme={}\nAccent={}\n", self.mode.key(), self.accent.label())
    }

    pub fn parse(text: &str) -> Appearance {
        let mut a = Appearance::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "Theme" => a.mode = ThemeMode::parse(v).unwrap_or(a.mode),
                "Accent" => a.accent = Accent::parse(v).unwrap_or(a.accent),
                _ => {}
            }
        }
        a
    }

    pub fn load_from(path: &Path) -> Appearance {
        std::fs::read_to_string(path).map(|t| Appearance::parse(&t)).unwrap_or_default()
    }

    pub fn save_to(self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.to_text());
    }

    /// Whether the dark palette applies; `system_dark` is what Windows reports (None = unknown).
    pub fn is_dark(self, system_dark: Option<bool>) -> bool {
        match self.mode {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => system_dark.unwrap_or(false),
        }
    }

    pub fn palette(self, system_dark: Option<bool>) -> Palette {
        Palette::new(self.is_dark(system_dark), self.accent)
    }
}

/// Where the appearance is saved: `appearance.txt` next to the settings file.
pub fn settings_file() -> std::path::PathBuf {
    crate::autorecover::settings_file()
        .map_or_else(|| std::path::PathBuf::from("appearance.txt"), |p| p.with_file_name("appearance.txt"))
}

/// Every colour the chrome uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// The preset's own colour: outlines, slider, selected thumbnail frame.
    pub accent: Color32,
    /// Accent for solid fills that carry white text (backstage rail), contrast >= 4.5 with white.
    pub accent_strong: Color32,
    pub accent_hover: Color32,
    pub accent_pressed: Color32,
    /// Accent as text on the card / tint: contrast >= 4.5.
    pub accent_text: Color32,
    /// Selected-tab pill, selected rows and thumbnails.
    pub accent_tint: Color32,
    pub accent_tint_hover: Color32,
    /// Window background behind the cards (title row, tab strip).
    pub window_bg: Color32,
    /// The surface of the ribbon card, panes and backstage content.
    pub card: Color32,
    pub line: Color32,
    pub workspace: Color32,
    pub text: Color32,
    pub disabled: Color32,
    pub group_label: Color32,
    pub hover_bg: Color32,
    pub selected_bg: Color32,
    pub hover_pressed: Color32,
    pub button_outline: Color32,
    /// A stronger frame (text boxes, flat buttons).
    pub frame: Color32,
    pub text_selection: Color32,
    pub status_bg: Color32,
    pub status_text: Color32,
    pub status_dim: Color32,
    /// The ink of icon outlines (`currentColor`).
    pub ink: Color32,
    pub shadow: Color32,
    pub on_accent: Color32,
}

fn lin(c: u8) -> f32 {
    let v = f32::from(c) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn rel_lum(c: Color32) -> f32 {
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG contrast ratio of two opaque colours (1..21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (rel_lum(a), rel_lum(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `a` moved `t` (0..1) of the way to `b`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// `c`, moved toward `toward` in small steps until it reaches `ratio` contrast against `bg`.
fn reach_contrast(c: Color32, toward: Color32, bg: Color32, ratio: f32) -> Color32 {
    let mut t = 0.0;
    let mut out = c;
    while contrast(out, bg) < ratio && t < 1.0 {
        t += 0.02;
        out = mix(c, toward, t);
    }
    out
}

impl Palette {
    pub fn new(dark: bool, accent: Accent) -> Palette {
        let base = accent.base();
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        // Orange keeps its own colour for solid fills (smokey text on it); the others are darkened
        // until white text reaches 4.8:1.
        let orange = accent == Accent::Orange;
        let strong = if orange { base } else { reach_contrast(base, black, white, 4.8) };
        let (hover, pressed) = if orange {
            (mix(strong, white, 0.2), mix(strong, white, 0.4))
        } else {
            (mix(strong, black, 0.15), mix(strong, black, 0.3))
        };
        let (window_bg, card) = if dark {
            (Color32::from_rgb(0x1A, 0x1B, 0x1E), Color32::from_rgb(0x2A, 0x2B, 0x30))
        } else {
            (Color32::from_rgb(0xF3, 0xF4, 0xF6), white)
        };
        let tint = if dark { mix(card, base, 0.32) } else { mix(white, base, 0.16) };
        let tint_hover = if dark { mix(card, base, 0.2) } else { mix(white, base, 0.09) };
        let accent_text = if dark {
            reach_contrast(base, white, tint, 5.0)
        } else {
            reach_contrast(base, black, tint, 4.8)
        };
        let g = |v: u8| Color32::from_rgb(v, v, v + 2);
        Palette {
            dark,
            accent: base,
            accent_strong: strong,
            accent_hover: hover,
            accent_pressed: pressed,
            accent_text,
            accent_tint: tint,
            accent_tint_hover: tint_hover,
            window_bg,
            card,
            line: if dark { g(0x3E) } else { Color32::from_rgb(0xDA, 0xDD, 0xE2) },
            workspace: if dark { Color32::from_rgb(0x1F, 0x1F, 0x1F) } else { Color32::from_rgb(0xED, 0xEE, 0xF0) },
            text: if dark { Color32::from_rgb(0xE8, 0xEA, 0xED) } else { Color32::from_rgb(0x1F, 0x23, 0x28) },
            disabled: if dark { Color32::from_rgb(0x74, 0x78, 0x80) } else { Color32::from_rgb(0xA6, 0xAB, 0xB3) },
            group_label: if dark { Color32::from_rgb(0xA6, 0xAC, 0xB6) } else { Color32::from_rgb(0x5B, 0x64, 0x70) },
            hover_bg: if dark { g(0x3A) } else { Color32::from_rgb(0xE9, 0xEB, 0xEF) },
            selected_bg: if dark { g(0x4A) } else { Color32::from_rgb(0xDC, 0xDF, 0xE5) },
            hover_pressed: if dark { g(0x56) } else { Color32::from_rgb(0xCF, 0xD3, 0xDA) },
            button_outline: if dark { g(0x55) } else { Color32::from_rgb(0xC6, 0xCA, 0xD1) },
            frame: if dark { g(0x6A) } else { Color32::from_rgb(0xA3, 0xA8, 0xB0) },
            text_selection: if dark { mix(card, base, 0.55) } else { mix(white, base, 0.35) },
            status_bg: if dark { Color32::from_rgb(0x22, 0x23, 0x27) } else { Color32::from_rgb(0xEA, 0xEC, 0xF0) },
            status_text: if dark { Color32::from_rgb(0xD5, 0xD8, 0xDD) } else { Color32::from_rgb(0x34, 0x39, 0x40) },
            status_dim: if dark { Color32::from_rgb(0x6B, 0x70, 0x78) } else { Color32::from_rgb(0xA0, 0xA5, 0xAD) },
            ink: if dark { Color32::from_rgb(0xDD, 0xE0, 0xE5) } else { Color32::from_rgb(0x42, 0x42, 0x42) },
            shadow: Color32::from_black_alpha(if dark { 90 } else { 30 }),
            on_accent: accent.on_fill(),
        }
    }

    /// Every colour, for tests.
    #[cfg(test)]
    pub fn colours(&self) -> Vec<Color32> {
        vec![
            self.accent, self.accent_strong, self.accent_hover, self.accent_pressed,
            self.accent_text, self.accent_tint, self.accent_tint_hover, self.window_bg,
            self.card, self.line, self.workspace, self.text, self.disabled, self.group_label,
            self.hover_bg, self.selected_bg, self.hover_pressed, self.button_outline,
            self.frame, self.text_selection, self.status_bg, self.status_text,
            self.status_dim, self.ink, self.on_accent,
        ]
    }
}

impl Default for Palette {
    fn default() -> Palette {
        Palette::new(false, Accent::Teal)
    }
}

thread_local! {
    static CURRENT: Cell<Palette> = Cell::new(Palette::default());
}

/// Make `p` the palette every accessor returns (on this thread).
pub fn set(p: Palette) {
    CURRENT.with(|c| c.set(p));
}

/// The current palette.
pub fn pal() -> Palette {
    CURRENT.with(Cell::get)
}

macro_rules! accessors {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("The current palette's `", stringify!($name), "`.")]
            pub fn $name() -> Color32 { pal().$name }
        )*
    };
}

accessors!(
    accent, accent_text, accent_tint,
    window_bg, card, line, workspace, text, disabled, group_label, hover_bg,
    selected_bg, hover_pressed, button_outline, frame, status_bg, status_text,
    status_dim, ink, shadow, on_accent,
);

/// Fill of a Modern ribbon group tile: a hair off the card colour.
pub fn tile() -> Color32 {
    mix(card(), text(), if is_dark() { 0.07 } else { 0.035 })
}

/// Outline of a Modern ribbon group tile.
pub fn tile_line() -> Color32 {
    mix(card(), text(), if is_dark() { 0.14 } else { 0.09 })
}

pub fn is_dark() -> bool {
    pal().dark
}

/// Test seam: which chrome variants the last frame drew ("file:solid", "tab:pill", ...).
#[cfg(test)]
pub mod probe {
    use std::cell::RefCell;
    thread_local! {
        static NOTES: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }
    pub fn note(s: &'static str) {
        NOTES.with(|n| n.borrow_mut().push(s));
    }
    pub fn clear() {
        NOTES.with(|n| n.borrow_mut().clear());
    }
    pub fn has(s: &str) -> bool {
        NOTES.with(|n| n.borrow().contains(&s))
    }
}

/// Record a drawn chrome variant (tests only).
macro_rules! drew {
    ($s:expr) => {
        #[cfg(test)]
        $crate::theme::probe::note($s);
    };
}
pub(crate) use drew;

#[cfg(test)]
mod tests {
    use super::*;

    /// Publisher's exact brand green (and its hover / pressed shades) must not appear anywhere.
    const PUBLISHER_GREENS: [Color32; 3] = [
        Color32::from_rgb(0x07, 0x75, 0x68),
        Color32::from_rgb(0x05, 0x5F, 0x55),
        Color32::from_rgb(0x04, 0x4E, 0x46),
    ];

    #[test]
    fn no_chrome_colour_is_publishers_green() {
        for dark in [false, true] {
            for a in Accent::ALL {
                for c in Palette::new(dark, a).colours() {
                    assert!(!PUBLISHER_GREENS.contains(&c), "{c:?} in {dark} {a:?}");
                }
            }
        }
    }

    #[test]
    fn text_colours_reach_wcag_aa() {
        for dark in [false, true] {
            for a in Accent::ALL {
                let p = Palette::new(dark, a);
                // Smokey grey on Tennessee orange is the owner's choice: 2.8:1, better than white.
                let min = if a == Accent::Orange { 2.8 } else { 4.5 };
                assert!(contrast(p.text, p.card) >= 7.0);
                assert!(contrast(p.text, p.window_bg) >= 7.0);
                assert!(contrast(p.group_label, p.card) >= 4.5, "{dark} group label");
                assert!(contrast(p.accent_text, p.accent_tint) >= 4.5, "{dark} {a:?} text on tint");
                assert!(contrast(p.accent_text, p.card) >= 4.5, "{dark} {a:?} text on card");
                assert!(contrast(p.status_text, p.status_bg) >= 7.0);
                assert!(contrast(p.on_accent, p.accent_strong) >= min, "{a:?} on-fill on strong");
                assert!(contrast(p.on_accent, p.accent_hover) >= min);
            }
        }
    }

    #[test]
    fn dark_and_light_differ() {
        let (l, d) = (Palette::new(false, Accent::Teal), Palette::new(true, Accent::Teal));
        assert_ne!(l.card, d.card);
        assert_ne!(l.window_bg, d.window_bg);
        assert_ne!(l.text, d.text);
    }

    #[test]
    fn appearance_round_trips_through_text() {
        let a = Appearance { mode: ThemeMode::System, accent: Accent::Purple };
        assert_eq!(Appearance::parse(&a.to_text()), a);
        assert_eq!(Appearance::parse("garbage"), Appearance::default());
        assert_eq!(Appearance::default().accent, Accent::Teal);
        assert_eq!(Appearance::default().mode, ThemeMode::Light);
    }

    #[test]
    fn old_style_setting_is_ignored() {
        let a = Appearance::parse("Theme=Dark\nAccent=Purple\nStyle=Classic\n");
        assert_eq!((a.mode, a.accent), (ThemeMode::Dark, Accent::Purple));
        assert_eq!(Appearance::parse("Style=Bogus\n"), Appearance::default());
        assert!(!Appearance { mode: ThemeMode::Light, accent: Accent::Teal }.to_text().contains("Style"));
    }

    #[test]
    fn follow_windows_uses_the_system_theme() {
        let a = Appearance { mode: ThemeMode::System, accent: Accent::Teal };
        assert!(a.is_dark(Some(true)));
        assert!(!a.is_dark(Some(false)));
        assert!(!a.is_dark(None));
    }

    #[test]
    fn orange_is_the_fifth_accent_and_older_values_still_load() {
        assert_eq!(Accent::ALL.len(), 5);
        assert_eq!(Accent::parse("orange"), Some(Accent::Orange));
        assert_eq!(Accent::Orange.base(), Color32::from_rgb(0xFF, 0x82, 0x00));
        let a = Appearance { accent: Accent::Orange, ..Appearance::default() };
        assert_eq!(Appearance::parse(&a.to_text()), a);
        assert_eq!(Appearance::parse("Accent=Teal\n").accent, Accent::Teal);
        assert_eq!(Appearance::parse("Accent=Magenta\n").accent, Accent::Teal);
    }
}
