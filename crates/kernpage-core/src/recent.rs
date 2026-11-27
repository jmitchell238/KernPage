//! The list of recently opened or saved publications, kept in a small text file
//! (one path per line, newest first, each followed by a tab and the Unix time KernPage last
//! opened or saved it; older files without the time still load) in the user's configuration folder.
//!
//! Shared by the editor (which writes it) and the KernPage Launcher (which reads it and
//! can remove entries), so both always agree on the file and its format.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX: usize = 20;

pub struct Recent {
    items: Vec<PathBuf>,
    /// When KernPage last opened or saved each item (same order); `None` for lines written by
    /// older versions.
    times: Vec<Option<u64>>,
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
        let entries: Vec<(PathBuf, Option<u64>)> = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|t| {
                t.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| match l.split_once('\t') {
                        Some((p, secs)) => (PathBuf::from(p), secs.trim().parse().ok()),
                        None => (PathBuf::from(l), None),
                    })
                    .filter(|(p, _)| !existing_only || p.is_file())
                    .take(MAX)
                    .collect()
            })
            .unwrap_or_default();
        let (items, times) = entries.into_iter().unzip();
        Recent { items, times, file, fixed: false }
    }

    /// A list that is never saved and that `add`, `remove` and `clear` leave alone.
    pub fn fixed(items: Vec<PathBuf>) -> Self {
        let times = vec![None; items.len()];
        Recent { items, times, file: None, fixed: true }
    }

    /// Is this a fixed list (see `fixed`)?
    pub fn is_fixed(&self) -> bool {
        self.fixed
    }

    pub fn items(&self) -> &[PathBuf] {
        &self.items
    }

    /// When KernPage last opened or saved `path` (Unix seconds), if the list knows.
    pub fn opened_at(&self, path: &Path) -> Option<u64> {
        let i = self.items.iter().position(|p| p == path)?;
        self.times[i]
    }

    /// Put `path` at the top of the list, stamped with the current time, and save the list.
    pub fn add(&mut self, path: &Path) {
        self.add_at(path, now_secs());
    }

    /// Like [`Recent::add`] with an explicit time (Unix seconds).
    pub fn add_at(&mut self, path: &Path, secs: u64) {
        if self.fixed {
            return;
        }
        if let Some(i) = self.items.iter().position(|p| p == path) {
            self.items.remove(i);
            self.times.remove(i);
        }
        self.items.insert(0, path.to_path_buf());
        self.times.insert(0, Some(secs));
        self.items.truncate(MAX);
        self.times.truncate(MAX);
        self.save();
    }

    /// Take `path` out of the list and save the list.
    pub fn remove(&mut self, path: &Path) {
        if self.fixed {
            return;
        }
        if let Some(i) = self.items.iter().position(|p| p == path) {
            self.items.remove(i);
            self.times.remove(i);
        }
        self.save();
    }

    /// Empty the list and save it.
    pub fn clear(&mut self) {
        if self.fixed {
            return;
        }
        self.items.clear();
        self.times.clear();
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
                .zip(&self.times)
                .map(|(p, t)| match t {
                    Some(t) => format!("{}\t{t}", p.to_string_lossy()),
                    None => p.to_string_lossy().into_owned(),
                })
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

/// The current time, Unix seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Seconds to add to UTC to get the user's local time now (0 where it cannot be found).
#[cfg(windows)]
pub fn local_utc_offset_secs() -> i64 {
    #[repr(C)]
    struct Tzi {
        bias: i32,
        std_name: [u16; 32],
        std_date: [u16; 8],
        std_bias: i32,
        dst_name: [u16; 32],
        dst_date: [u16; 8],
        dst_bias: i32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTimeZoneInformation(info: *mut Tzi) -> u32;
    }
    // SAFETY: `Tzi` matches TIME_ZONE_INFORMATION and the call only fills it in.
    unsafe {
        let mut t: Tzi = std::mem::zeroed();
        let id = GetTimeZoneInformation(&mut t);
        let extra = if id == 2 { t.dst_bias } else { t.std_bias };
        -(i64::from(t.bias) + i64::from(extra)) * 60
    }
}

#[cfg(not(windows))]
pub fn local_utc_offset_secs() -> i64 {
    0
}

/// The label under a Recent card for `at` (Unix seconds), relative to the current time and zone.
pub fn opened_label_now(at: u64) -> String {
    opened_label(at, now_secs(), local_utc_offset_secs())
}

/// (year, month, day) of a day count since 1970-01-01.
fn civil(days: i64) -> (i64, i64, i64) {
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
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// "YYYY-MM-DD" (UTC) for a file's modification time.
pub fn date_text(t: SystemTime) -> String {
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The short label under a Recent card: "Opened today 10:24", "Opened yesterday" or
/// "Opened 23 Nov 2026". `at` and `now` are Unix seconds; `utc_offset_secs` converts them to
/// the user's local day (local time = UTC + offset).
pub fn opened_label(at: u64, now: u64, utc_offset_secs: i64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (la, ln) = (at as i64 + utc_offset_secs, now as i64 + utc_offset_secs);
    let (da, dn) = (la.div_euclid(86_400), ln.div_euclid(86_400));
    if da == dn {
        let s = la.rem_euclid(86_400);
        format!("Opened today {:02}:{:02}", s / 3600, s % 3600 / 60)
    } else if da == dn - 1 {
        "Opened yesterday".to_owned()
    } else {
        let (y, m, d) = civil(da);
        format!("Opened {d} {} {y}", MONTHS[(m - 1) as usize])
    }
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

#[cfg(test)]
mod opened_tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pw-core-opened-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // 2026-11-27 10:24:00 UTC
    const NOW: u64 = 1_795_775_040;

    #[test]
    fn the_label_says_today_yesterday_or_the_date() {
        assert_eq!(opened_label(NOW, NOW, 0), "Opened today 10:24");
        assert_eq!(opened_label(NOW - 3600, NOW, 0), "Opened today 09:24");
        assert_eq!(opened_label(NOW - 86_400, NOW, 0), "Opened yesterday");
        assert_eq!(opened_label(NOW - 4 * 86_400, NOW, 0), "Opened 23 Nov 2026");
        // The same instants in a zone 14 hours ahead: 10:24 UTC is already 00:24 the next day.
        assert_eq!(opened_label(NOW, NOW, 14 * 3600), "Opened today 00:24");
        assert_eq!(opened_label(NOW - 3600, NOW, 14 * 3600), "Opened yesterday");
    }

    #[test]
    fn times_are_stored_after_a_tab_and_old_lines_still_load_in_order() {
        let dir = temp("fmt");
        let (a, b, c) = (dir.join("a.pub"), dir.join("b.pub"), dir.join("c.pub"));
        for f in [&a, &b, &c] {
            std::fs::write(f, b"x").unwrap();
        }
        let store = dir.join("recent.txt");
        // An old-format file: paths only, newest first.
        std::fs::write(&store, format!("{}\n{}\n{}", a.display(), b.display(), c.display())).unwrap();
        let r = Recent::load_from(Some(store.clone()));
        assert_eq!(r.items(), &[a.clone(), b.clone(), c.clone()], "order as stored");
        assert_eq!(r.opened_at(&a), None);
        // Mixed: a timed line followed by an untimed one.
        std::fs::write(&store, format!("{}\t{NOW}\n{}", a.display(), b.display())).unwrap();
        let r = Recent::load_from(Some(store.clone()));
        assert_eq!(r.items(), &[a.clone(), b.clone()]);
        assert_eq!(r.opened_at(&a), Some(NOW));
        assert_eq!(r.opened_at(&b), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adding_stamps_the_time_orders_newest_first_and_persists() {
        let dir = temp("add");
        let (a, b) = (dir.join("a.pub"), dir.join("b.pub"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let store = dir.join("recent.txt");
        let mut r = Recent::load_from(Some(store.clone()));
        r.add_at(&a, 100);
        r.add_at(&b, 200);
        assert_eq!(r.items(), &[b.clone(), a.clone()]);
        // Saving a again moves it to the top with the new time.
        r.add_at(&a, 300);
        assert_eq!(r.items(), &[a.clone(), b.clone()]);
        assert_eq!((r.opened_at(&a), r.opened_at(&b)), (Some(300), Some(200)));
        let again = Recent::load_from(Some(store));
        assert_eq!(again.items(), &[a.clone(), b.clone()]);
        assert_eq!((again.opened_at(&a), again.opened_at(&b)), (Some(300), Some(200)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
