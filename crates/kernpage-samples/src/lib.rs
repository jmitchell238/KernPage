//! Test helper: finds sample files (mostly Publisher `.pub` files).
//!
//! The repository keeps a small core of samples in `samples/`. The rest live outside the repo, in a
//! folder that mirrors `samples/`; the environment variable `KERNPAGE_EXTRA_SAMPLES` names it.
//! A test that needs a file found in neither place prints one `skipped:` line and passes.
//!
//! Use `#[kernpage_samples::test]` instead of `#[test]` and ask for files through [`path`],
//! [`resolve`] or [`files`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use kernpage_samples_macros::test;

/// Name of the environment variable that points at the extra-samples folder.
pub const ENV: &str = "KERNPAGE_EXTRA_SAMPLES";

/// Panic payload that ends a test early because a sample file is not available.
pub struct Skipped;

/// The repository's `samples/` folder.
pub fn repo_samples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

/// The extra-samples folder, when the environment variable is set to an existing folder.
pub fn extra_samples() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os(ENV)?);
    p.is_dir().then_some(p)
}

/// `rel` (like `gen/list-none.pub`) in the repo's `samples/`, else in the extra folder.
pub fn find(rel: &str) -> Option<PathBuf> {
    let here = repo_samples().join(rel);
    if here.exists() {
        return Some(here);
    }
    let there = extra_samples()?.join(rel);
    there.exists().then_some(there)
}

/// Ends the running test as skipped.
pub fn skip(rel: &str) -> ! {
    println!("skipped: {rel} (set {ENV})");
    std::panic::resume_unwind(Box::new(Skipped))
}

/// Path of `rel` under `samples/`; skips the test when the file is not available.
pub fn path(rel: &str) -> PathBuf {
    find(rel).unwrap_or_else(|| skip(rel))
}

/// Bytes of `rel` under `samples/`; skips the test when the file is not available.
pub fn read(rel: &str) -> Vec<u8> {
    std::fs::read(path(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Bytes of `rel` under `samples/`, or `None` when the file is not available (for loops that
/// skip the files they do not have).
pub fn read_opt(rel: &str) -> Option<Vec<u8>> {
    std::fs::read(find(rel)?).ok()
}

/// For code that builds a full path to a sample itself (`.../samples/gen/x.pub`): the same path when
/// the file exists, else its twin in the extra folder, else the test is skipped.
pub fn resolve(p: impl AsRef<Path>) -> PathBuf {
    let p = p.as_ref();
    if p.exists() {
        return p.to_path_buf();
    }
    let comps: Vec<_> = p.components().collect();
    let Some(i) = comps.iter().rposition(|c| c.as_os_str() == "samples") else {
        skip(&p.display().to_string());
    };
    let rel: PathBuf = comps[i + 1..].iter().collect();
    let rel = rel.to_string_lossy().replace('\\', "/");
    path(&rel)
}

/// Every file with extension `ext` directly in `samples/<dir>`, in the repo and in the extra folder
/// (the repo's copy wins), sorted by name. Missing folders give an empty list.
pub fn files(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut found: BTreeMap<std::ffi::OsString, PathBuf> = BTreeMap::new();
    let roots = extra_samples().into_iter().chain([repo_samples()]); // repo last: it overwrites
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root.join(dir)) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext)) {
                found.insert(e.file_name(), p);
            }
        }
    }
    found.into_values().collect()
}

/// Runs a test body; a [`Skipped`] unwind ends it as a pass, anything else panics on as usual.
pub fn run(body: impl FnOnce()) {
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        if !e.is::<Skipped>() {
            std::panic::resume_unwind(e);
        }
    }
}
