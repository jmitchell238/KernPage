//! The list of recently opened or saved publications, kept in a small text file
//! (one path per line, newest first) in the user's configuration folder.
//!
//! Shared by the editor (which writes it) and the KernPage Launcher (which reads it and
//! can remove entries), so both always agree on the file and its format.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX: usize = 20;

pub struct Recent {
    items: Vec<PathBuf>,
    file: Option<PathBuf>,
    /// A fixed list (demo mode): never saved, never changed.
    fixed: bool,
}

/// The "KernPage" folder inside the user's configuration folder.
pub fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("KernPage"))
}

/// Where the list is kept: `KERNPAGE_RECENT_FILE` when set, else the user's config folder.
pub fn store_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("KERNPAGE_RECENT_FILE") {
        return Some(PathBuf::from(p));
    }
    Some(config_dir()?.join("recent.txt"))
}

impl Recent {
    /// The list as the editor wants it: files that no longer exist are left out.
    pub fn load() -> Self {
        Self::load_from(store_path())
    }

    /// The list including files that no longer exist (the launcher shows them as "Not found").
    pub fn load_all() -> Self {
        Self::read(store_path(), false)
    }

    pub fn load_from(file: Option<PathBuf>) -> Self {
        Self::read(file, true)
    }

    pub fn load_all_from(file: Option<PathBuf>) -> Self {
        Self::read(file, false)
    }

    fn read(file: Option<PathBuf>, existing_only: bool) -> Self {
        let items = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|t| {
                t.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(PathBuf::from)
                    .filter(|p| !existing_only || p.is_file())
                    .take(MAX)
                    .collect()
            })
            .unwrap_or_default();
        Recent { items, file, fixed: false }
    }

    /// A list that is never saved and that `add`, `remove` and `clear` leave alone.
    pub fn fixed(items: Vec<PathBuf>) -> Self {
        Recent { items, file: None, fixed: true }
    }

    /// Is this a fixed list (see `fixed`)?
    pub fn is_fixed(&self) -> bool {
        self.fixed
    }

    pub fn items(&self) -> &[PathBuf] {
        &self.items
    }

    /// Put `path` at the top of the list and save the list.
    pub fn add(&mut self, path: &Path) {
        if self.fixed {
            return;
        }
        self.items.retain(|p| p != path);
        self.items.insert(0, path.to_path_buf());
        self.items.truncate(MAX);
        self.save();
    }

    /// Take `path` out of the list and save the list.
    pub fn remove(&mut self, path: &Path) {
        if self.fixed {
            return;
        }
        self.items.retain(|p| p != path);
        self.save();
    }

    /// Empty the list and save it.
    pub fn clear(&mut self) {
        if self.fixed {
            return;
        }
        self.items.clear();
        self.save();
    }

    fn save(&self) {
        if let Some(f) = &self.file {
            if let Some(dir) = f.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let text: Vec<String> = self
                .items
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            let _ = std::fs::write(f, text.join("\n"));
        }
    }

    /// The folders of the listed files, newest first, without repeats.
    pub fn folders(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for p in &self.items {
            if let Some(d) = p.parent() {
                if !out.iter().any(|o| o == d) {
                    out.push(d.to_path_buf());
                }
            }
        }
        out
    }
}

/// "C: » Folder » Sub" for a folder path.
pub fn breadcrumb(dir: &Path) -> String {
    dir.components()
        .filter_map(|c| match c {
            std::path::Component::RootDir => None,
            c => Some(c.as_os_str().to_string_lossy().into_owned()),
        })
        .collect::<Vec<_>>()
        .join(" \u{00bb} ")
}

/// "YYYY-MM-DD" (UTC) for a file's modification time.
pub fn date_text(t: SystemTime) -> String {
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = (secs / 86_400) as i64;
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pw-core-recent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dates_and_breadcrumbs() {
        assert_eq!(date_text(SystemTime::UNIX_EPOCH), "1970-01-01");
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_709_164_800);
        assert_eq!(date_text(t), "2024-02-29");
        assert_eq!(breadcrumb(Path::new("/a/b/c")), "a \u{00bb} b \u{00bb} c");
    }

    #[test]
    fn editor_writes_launcher_reads_and_removes() {
        let dir = temp("share");
        let (a, b, gone) = (dir.join("a.pub"), dir.join("b.pub"), dir.join("gone.pub"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let store = dir.join("recent.txt");
        // The editor adds three files, then one of them disappears.
        let mut editor = Recent::load_from(Some(store.clone()));
        editor.add(&gone);
        editor.add(&a);
        editor.add(&b);
        // The editor view skips the missing file; the launcher view keeps it.
        let editor_view = Recent::load_from(Some(store.clone()));
        assert_eq!(editor_view.items(), &[b.clone(), a.clone()]);
        let mut launcher = Recent::load_all_from(Some(store.clone()));
        assert_eq!(launcher.items(), &[b.clone(), a.clone(), gone.clone()]);
        // The launcher removes the missing one; the editor sees the same file.
        launcher.remove(&gone);
        assert_eq!(Recent::load_all_from(Some(store.clone())).items(), &[b.clone(), a.clone()]);
        launcher.clear();
        assert!(Recent::load_all_from(Some(store)).items().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_dedups_and_persists() {
        let dir = temp("add");
        let (a, b) = (dir.join("a.pub"), dir.join("b.pub"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let store = dir.join("recent.txt");
        let mut r = Recent::load_from(Some(store.clone()));
        r.add(&a);
        r.add(&b);
        r.add(&a);
        assert_eq!(r.items(), &[a.clone(), b.clone()]);
        assert_eq!(r.folders(), vec![dir.clone()]);
        let again = Recent::load_from(Some(store));
        assert_eq!(again.items(), &[a, b]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
