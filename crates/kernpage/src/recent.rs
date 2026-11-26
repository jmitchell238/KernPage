//! The list of recently opened or saved publications. The logic lives in
//! `kernpage_core::recent` so the KernPage Launcher shares the same file and format.

pub use kernpage_core::recent::{breadcrumb, date_text, Recent};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::SystemTime;

    #[test]
    fn dates_and_breadcrumbs() {
        assert_eq!(date_text(SystemTime::UNIX_EPOCH), "1970-01-01");
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_709_164_800);
        assert_eq!(date_text(t), "2024-02-29");
        assert_eq!(breadcrumb(Path::new("/a/b/c")), "a \u{00bb} b \u{00bb} c");
    }

    #[test]
    fn add_dedups_and_persists() {
        let dir = std::env::temp_dir().join(format!("pw-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
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
