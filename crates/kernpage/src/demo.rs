//! `--demo`: invented, neutral data for screenshots (website and README pictures), so nothing
//! from the real profile (user name, folders, recent files, printers) can appear in a picture.
//! In demo mode the real recent list, settings, AutoRecover folder and update cache are neither
//! read nor written.

use kernpage_core::recent::Recent;
use std::path::{Path, PathBuf};

/// The made-up Windows user name.
pub const USER: &str = "Demo";
/// The only printer a demo run offers.
pub const PRINTER: &str = "Office Printer";

#[cfg(not(test))]
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
thread_local! { static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// Turn demo mode on (or off, in tests).
pub fn set(on: bool) {
    #[cfg(not(test))]
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(|c| c.set(on));
}

/// Is demo mode on?
pub fn is_on() -> bool {
    #[cfg(not(test))]
    return ON.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(std::cell::Cell::get)
}

/// The made-up home folder.
fn home() -> PathBuf {
    PathBuf::from(format!("C:\\Users\\{USER}"))
}

/// The folder every opened document is shown in.
fn doc_folder() -> PathBuf {
    home().join("Documents").join("Church Newsletter")
}

/// The made-up recent files, newest first: (folder under the home folder, file name, date).
const FILES: [(&str, &str, &str); 5] = [
    ("Documents\\Church Newsletter", "Riverbend Review - May.kern", "2026-11-24"),
    ("Documents\\Community Flyers", "Spring Flyer.kern", "2026-11-22"),
    ("Documents\\Community Flyers", "Bake Sale Poster.kern", "2026-11-19"),
    ("Documents", "Sunday Bulletin.pub", "2026-11-15"),
    ("Desktop", "Garden Club Newsletter.kern", "2026-11-09"),
];

/// The recent list of a demo run: fixed, never saved, never changed by opening files.
pub fn recent() -> Recent {
    Recent::fixed(FILES.iter().map(|(dir, name, _)| home().join(dir).join(name)).collect())
}

/// The modified date shown for a made-up recent file.
pub fn date_of(path: &Path) -> Option<&'static str> {
    FILES.iter().find(|(_, name, _)| path.file_name().is_some_and(|n| n == *name)).map(|f| f.2)
}

/// The date shown as "Last Saved" for the open document.
pub const OPEN_DATE: &str = "2026-11-24";

/// Where the open document is shown to be: its real name in the made-up folder.
pub fn shown_path(real: &Path) -> PathBuf {
    doc_folder().join(real.file_name().unwrap_or_default())
}

/// The printers a demo run offers.
pub fn printers() -> crate::printing::PrinterList {
    crate::printing::PrinterList { names: vec![PRINTER.to_owned()], default: Some(PRINTER.to_owned()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_lists_are_the_invented_ones() {
        let r = recent();
        let names: Vec<_> = r.items().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["Riverbend Review - May.kern", "Spring Flyer.kern", "Bake Sale Poster.kern", "Sunday Bulletin.pub", "Garden Club Newsletter.kern"]);
        let folders: Vec<_> = r.folders().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(folders, ["Church Newsletter", "Community Flyers", "Documents", "Desktop"]);
        assert!(r.items().iter().all(|p| date_of(p).is_some()));
    }

    #[test]
    fn a_demo_recent_list_ignores_changes() {
        let mut r = recent();
        r.add(Path::new("real.kern"));
        r.remove(&r.items()[0].clone());
        r.clear();
        assert_eq!(r.items().len(), 5);
    }

    #[test]
    fn an_opened_file_is_shown_in_the_demo_folder() {
        let p = shown_path(Path::new("C:\\somewhere\\else\\showcase.kern"));
        assert_eq!(p, PathBuf::from(r"C:\Users\Demo\Documents\Church Newsletter\showcase.kern"));
    }

    #[test]
    fn only_the_office_printer_is_offered() {
        let l = printers();
        assert_eq!((l.names.as_slice(), l.default.as_deref()), (&[PRINTER.to_owned()][..], Some(PRINTER)));
    }
}
