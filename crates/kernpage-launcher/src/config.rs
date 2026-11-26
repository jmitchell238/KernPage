//! Launcher settings, kept in `launcher.toml` next to `recent.txt` in the "KernPage"
//! configuration folder. The file is a flat list of `key = value` lines (a small subset of
//! TOML) so no parser dependency is needed. Unknown keys and bad values are ignored.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    System,
    Dark,
    Light,
}

impl Appearance {
    fn key(self) -> &'static str {
        match self {
            Appearance::System => "system",
            Appearance::Dark => "dark",
            Appearance::Light => "light",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Appearance::System),
            "dark" => Some(Appearance::Dark),
            "light" => Some(Appearance::Light),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckMode {
    Automatic,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Interface language; only "auto" exists for now.
    pub language: String,
    pub check_updates: CheckMode,
    pub auto_install: bool,
    pub notify: bool,
    pub include_prereleases: bool,
    pub appearance: Appearance,
    pub keep_in_tray: bool,
    pub start_at_login: bool,
    /// Hours between automatic checks (Automatic mode only).
    pub check_hours: u32,
    /// Highest index serial seen; an older index is refused.
    pub last_serial: u64,
    /// Versions the user chose to skip, as `id@version`.
    pub skipped: Vec<String>,
    /// The `id@version` last announced with a notification (so it is shown once).
    pub last_notified: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            language: "auto".to_owned(),
            check_updates: CheckMode::Manual,
            auto_install: false,
            notify: true,
            include_prereleases: false,
            appearance: Appearance::System,
            keep_in_tray: false,
            start_at_login: false,
            check_hours: 6,
            last_serial: 0,
            skipped: Vec::new(),
            last_notified: String::new(),
        }
    }
}

impl Config {
    pub fn is_skipped(&self, id: &str, version: &str) -> bool {
        let key = format!("{id}@{version}");
        self.skipped.iter().any(|s| *s == key)
    }

    pub fn skip(&mut self, id: &str, version: &str) {
        if !self.is_skipped(id, version) {
            self.skipped.push(format!("{id}@{version}"));
        }
    }

    pub fn unskip(&mut self, id: &str, version: &str) {
        let key = format!("{id}@{version}");
        self.skipped.retain(|s| *s != key);
    }
}

/// Where the settings are kept: `KERNPAGE_LAUNCHER_CONFIG` when set, else next to `recent.txt`.
pub fn store_path() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(p) = crate::testhooks::peek(|f| f.config.clone()) {
        return Some(p);
    }
    if let Some(p) = std::env::var_os("KERNPAGE_LAUNCHER_CONFIG") {
        return Some(PathBuf::from(p));
    }
    Some(kernpage_core::recent::config_dir()?.join("launcher.toml"))
}

impl Config {
    pub fn load() -> Config {
        store_path().map(|p| Config::load_from(&p)).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = store_path() {
            let _ = self.save_to(&p);
        }
    }

    pub fn load_from(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .map(|t| Config::parse(&t))
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_text())
    }

    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim().trim_matches('"'));
            let flag = |v: &str| match v {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
            match k {
                "language" => c.language = v.to_owned(),
                "check_updates" => match v {
                    "automatic" => c.check_updates = CheckMode::Automatic,
                    "manual" => c.check_updates = CheckMode::Manual,
                    _ => {}
                },
                "auto_install" => c.auto_install = flag(v).unwrap_or(c.auto_install),
                "notify" => c.notify = flag(v).unwrap_or(c.notify),
                "include_prereleases" => {
                    c.include_prereleases = flag(v).unwrap_or(c.include_prereleases)
                }
                "appearance" => c.appearance = Appearance::parse(v).unwrap_or(c.appearance),
                "keep_in_tray" => c.keep_in_tray = flag(v).unwrap_or(c.keep_in_tray),
                "start_at_login" => c.start_at_login = flag(v).unwrap_or(c.start_at_login),
                "check_hours" => {
                    if let Ok(h) = v.parse::<u32>() {
                        c.check_hours = h.clamp(1, 168);
                    }
                }
                "last_serial" => c.last_serial = v.parse().unwrap_or(c.last_serial),
                "skipped" => c.skipped = v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect(),
                "last_notified" => c.last_notified = v.to_owned(),
                _ => {}
            }
        }
        c
    }

    pub fn to_text(&self) -> String {
        format!(
            "# KernPage Launcher settings\n\
             language = \"{}\"\n\
             check_updates = \"{}\"\n\
             auto_install = {}\n\
             notify = {}\n\
             include_prereleases = {}\n\
             appearance = \"{}\"\n\
             keep_in_tray = {}\n\
             start_at_login = {}\n\
             check_hours = {}\n\
             last_serial = {}\n\
             skipped = \"{}\"\n\
             last_notified = \"{}\"\n",
            self.language.replace('"', ""),
            match self.check_updates {
                CheckMode::Automatic => "automatic",
                CheckMode::Manual => "manual",
            },
            self.auto_install,
            self.notify,
            self.include_prereleases,
            self.appearance.key(),
            self.keep_in_tray,
            self.start_at_login,
            self.check_hours,
            self.last_serial,
            self.skipped.join(",").replace('"', ""),
            self.last_notified.replace('"', ""),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_conservative() {
        let c = Config::default();
        assert_eq!(c.check_updates, CheckMode::Manual);
        assert!(!c.auto_install && !c.include_prereleases && !c.keep_in_tray && !c.start_at_login);
        assert!(c.notify);
        assert_eq!(c.appearance, Appearance::System);
        assert_eq!(c.language, "auto");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("pw-launcher-cfg-{}", std::process::id()));
        let file = dir.join("sub").join("launcher.toml");
        let c = Config {
            language: "auto".into(),
            check_updates: CheckMode::Automatic,
            auto_install: true,
            notify: false,
            include_prereleases: true,
            appearance: Appearance::Dark,
            keep_in_tray: true,
            start_at_login: true,
            check_hours: 12,
            last_serial: 42,
            skipped: vec!["kernpage@0.2.0".into(), "dict-en-us@2026.10".into()],
            last_notified: "kernpage@0.2.0".into(),
        };
        c.save_to(&file).unwrap();
        assert_eq!(Config::load_from(&file), c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_and_junk_fall_back_to_defaults() {
        assert_eq!(Config::load_from(Path::new("/no/such/launcher.toml")), Config::default());
        let c = Config::parse("appearance = \"purple\"\nnotify = maybe\nbogus\nkeep_in_tray = true\n");
        assert_eq!(c.appearance, Appearance::System);
        assert!(c.notify);
        assert!(c.keep_in_tray);
    }

    #[test]
    fn skip_and_unskip() {
        let mut c = Config::default();
        c.skip("a", "1.0.0");
        c.skip("a", "1.0.0");
        assert_eq!(c.skipped.len(), 1);
        assert!(c.is_skipped("a", "1.0.0") && !c.is_skipped("a", "1.0.1"));
        c.unskip("a", "1.0.0");
        assert!(c.skipped.is_empty());
    }
}
