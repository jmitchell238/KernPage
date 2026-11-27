//! Builds and signs a sample update index with the TEST-ONLY key in `tests/fixtures/`.
//! Used by the unit tests and by `examples/fake-feed.rs`; never part of the shipped launcher.
//! Self-contained on purpose: the example includes this file with `#[path]`.
#![allow(dead_code)]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const TEST_PUBLIC: &str = include_str!("../tests/fixtures/test-signing.pub");
pub const TEST_SECRET: &str = include_str!("../tests/fixtures/test-signing.key");
pub const OTHER_PUBLIC: &str = include_str!("../tests/fixtures/test-other.pub");
pub const OTHER_SECRET: &str = include_str!("../tests/fixtures/test-other.key");

/// The bytes served for every file in the sample feed (a stand-in, not a real installer).
pub const PLACEHOLDER_FILE: &[u8] = b"placeholder download for the local test feed; not a real installer\n";

/// The bytes served for the Setup.exe of the placeholder release (`SampleOpts::setup`).
pub const PLACEHOLDER_SETUP: &[u8] = b"placeholder Setup.exe for the local test feed; not a real installer
";

/// A real installer served by the fake feed (`--app-file`): its true size, SHA-256 and a
/// minisign signature made with the test key.
#[derive(Clone)]
pub struct RealFile {
    pub version: String,
    pub bytes: Vec<u8>,
    /// The version's Setup.exe (Burn bundle), served as a second, `setup` file of the release.
    pub setup: Option<Vec<u8>>,
}

impl RealFile {
    pub fn from_path(path: &std::path::Path, version: &str) -> std::io::Result<RealFile> {
        Ok(RealFile { version: version.to_owned(), bytes: std::fs::read(path)?, setup: None })
    }

    pub fn with_setup_from_path(mut self, path: &std::path::Path) -> std::io::Result<RealFile> {
        self.setup = Some(std::fs::read(path)?);
        Ok(self)
    }

    /// The name the Setup.exe is served under: `/files/<name>`.
    pub fn setup_name(&self) -> String {
        format!("KernPage-{}-Setup.exe", self.version)
    }

    /// The name it is served under: `/files/<name>`.
    pub fn name(&self) -> String {
        format!("kernpage-{}-x64.msi", self.version)
    }
}

pub struct SampleOpts {
    pub serial: u64,
    pub generated: String,
    pub expires: String,
    pub kernpage_version: String,
    /// Base URL for file links, e.g. `http://127.0.0.1:8750`.
    pub base_url: String,
    /// Break the SHA-256 of the KernPage file (an integrity failure the launcher must flag).
    pub bad_sha: bool,
    /// Block this versions requirement for KernPage, e.g. `<0.1.0`.
    pub block_kernpage: Option<String>,
    /// Serve this real installer as the KernPage stable release (instead of the placeholder).
    pub app_real: Option<RealFile>,
    /// Add this real installer as one more, older stable release (for testing "go back").
    pub app_older: Option<RealFile>,
    /// Also list a `setup` file (Setup.exe) next to the MSI of the placeholder release.
    pub setup: bool,
}

impl SampleOpts {
    pub fn new(serial: u64, base_url: &str) -> Self {
        SampleOpts {
            serial,
            generated: "2026-11-12T00:00:00Z".into(),
            expires: "2099-01-01T00:00:00Z".into(),
            kernpage_version: "0.2.0".into(),
            base_url: base_url.into(),
            bad_sha: false,
            block_kernpage: None,
            app_real: None,
            app_older: None,
            setup: false,
        }
    }
}

fn secret(text: &str) -> minisign::SecretKey {
    // Reading a secret key runs scrypt, which is slow in debug builds: do it once per key.
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<Vec<(String, minisign::SecretKey)>>> = OnceLock::new();
    let mut c = CACHE.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    if let Some((_, k)) = c.iter().find(|(t, _)| t == text) {
        return k.clone();
    }
    let k = minisign::SecretKeyBox::from_string(text).unwrap().into_secret_key(Some(String::new())).unwrap();
    c.push((text.to_owned(), k.clone()));
    k
}

pub fn sign_with(secret_text: &str, data: &[u8]) -> String {
    let sk = secret(secret_text);
    minisign::sign(None, &sk, std::io::Cursor::new(data), Some("kernpage test feed"), Some("TEST-ONLY signature"))
        .unwrap()
        .to_string()
}

pub fn sign_test(data: &[u8]) -> String {
    sign_with(TEST_SECRET, data)
}

fn placeholder_sig() -> String {
    static SIG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SIG.get_or_init(|| sign_test(PLACEHOLDER_FILE)).clone()
}

fn file(base: &str, name: &str, kind: &str, os: &str, bad_sha: bool, size: u64) -> Value {
    let mut sha = hex(Sha256::digest(PLACEHOLDER_FILE));
    if bad_sha {
        sha = sha[..60].to_owned(); // too short: not a SHA-256
    }
    json!({
        "os": os, "arch": if os == "windows" { "x86_64" } else { "any" }, "kind": kind,
        "url": format!("{base}/files/{name}"),
        "size": size, "sha256": sha, "sig": placeholder_sig()
    })
}

fn real_release(base: &str, rf: &RealFile, released: &str) -> Value {
    let mut files = vec![json!({
        "os": "windows", "arch": "x86_64", "kind": "msi",
        "url": format!("{base}/files/{}", rf.name()),
        "size": rf.bytes.len() as u64,
        "sha256": hex(Sha256::digest(&rf.bytes)),
        "sig": sign_test(&rf.bytes)
    })];
    if let Some(setup) = &rf.setup {
        files.push(setup_entry(base, &rf.setup_name(), setup));
    }
    json!({
        "version": rf.version, "released": released, "prerelease": false,
        "notes": "## Changed
- A real installer served by the local test feed
",
        "files": files
    })
}

/// The `setup` (Setup.exe) file entry: same fields as the MSI entry, `kind` = `setup`.
fn setup_entry(base: &str, name: &str, bytes: &[u8]) -> Value {
    json!({
        "os": "windows", "arch": "x86_64", "kind": "setup",
        "url": format!("{base}/files/{name}"),
        "size": bytes.len() as u64,
        "sha256": hex(Sha256::digest(bytes)),
        "sig": sign_test(bytes)
    })
}

/// Every file the sample feed links to, as `(path under /files, bytes)`.
pub fn served_files(o: &SampleOpts) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let v = &o.kernpage_version;
    match &o.app_real {
        Some(r) => out.push((r.name(), r.bytes.clone())),
        None => out.push((format!("kernpage-{v}-x64.msi"), PLACEHOLDER_FILE.to_vec())),
    }
    if let Some(r) = &o.app_older {
        out.push((r.name(), r.bytes.clone()));
        if let Some(b) = &r.setup {
            out.push((r.setup_name(), b.clone()));
        }
    }
    if let Some(r) = &o.app_real {
        if let Some(b) = &r.setup {
            out.push((r.setup_name(), b.clone()));
        }
    } else if o.setup {
        out.push((format!("KernPage-{v}-Setup.exe"), PLACEHOLDER_SETUP.to_vec()));
    }
    for name in ["kernpage-0.3.0-beta.1-x64.msi", "dict-en-us-2026.10.0.zip"] {
        out.push((name.to_owned(), PLACEHOLDER_FILE.to_vec()));
    }
    out
}

pub fn sample_index(o: &SampleOpts) -> Value {
    let v = match &o.app_real {
        Some(r) => &r.version,
        None => &o.kernpage_version,
    };
    let b = &o.base_url;
    let notes = "## Added\n\
- Spell check with downloadable dictionaries (#212)\n\
- Page templates for newsletters, flyers and bulletins (#205)\n\
- Export to PDF now embeds the fonts used in the publication (#198)\n\
- Find and Replace across all pages (#187)\n\
- Rotate and flip objects from the Arrange group (#181)\n\
- Zoom to fit page and zoom to selection (#176)\n\
## Fixed\n\
- Linked text boxes lost their link after a table was resized (#219)\n\
- Recent files list showed files twice when the path differed only by case (#214)\n\
- Picture crop handles could not be dragged near the page edge (#208)\n\
- Saving a publication with more than 99 pages could fail (#201)\n\
## Changed\n\
- The File screen opens faster on large publications (#193)\n\
- Clearer messages when a .pub file cannot be opened (#190)\n";
    let mut placeholder_files = vec![file(b, &format!("kernpage-{v}-x64.msi"), "msi", "windows", o.bad_sha, 30_801_920)];
    if o.setup {
        placeholder_files.push(setup_entry(b, &format!("KernPage-{v}-Setup.exe"), PLACEHOLDER_SETUP));
    }
    let mut stable = vec![match &o.app_real {
        Some(r) => real_release(b, r, "2026-12-01"),
        None => json!({
            "version": v, "released": "2026-12-01", "prerelease": false,
            "notes": notes,
            "changelog_url": "https://github.com/example/kernpage/blob/main/CHANGELOG.md",
            "files": placeholder_files
        }),
    }];
    if let Some(r) = &o.app_older {
        stable.push(real_release(b, r, "2026-06-01"));
    }
    let mut blocklist = Vec::new();
    if let Some(req) = &o.block_kernpage {
        blocklist.push(json!({"id": "kernpage", "versions": req, "reason": "This version can lose data when saving."}));
    }
    json!({
        "schema": 1,
        "serial": o.serial,
        "generated": o.generated,
        "expires": o.expires,
        "keys": { "current": TEST_PUBLIC.lines().nth(1).unwrap_or(""), "next": null },
        "blocklist": blocklist,
        "items": [
            {
                "id": "kernpage", "kind": "app", "name": "KernPage",
                "tagline": "Desktop publishing for bulletins, newsletters and flyers. Opens Publisher (.pub) files too.",
                "icon": format!("{b}/icons/kernpage.png"),
                "website": "https://example.org/kernpage",
                "repo": "https://github.com/example/kernpage",
                "releases_url": "https://github.com/example/kernpage/releases",
                "license": "GPL-3.0-only",
                "features": [
                    {"title": "Opens Publisher files", "text": "Reads .pub files and shows text, shapes, pictures, tables, groups and linked text boxes."},
                    {"title": "Edits and saves .pub", "text": "Change text and formatting, move and resize objects, and save back into the same file."},
                    {"title": "Spell check", "text": "Underlines mistakes as you type. Dictionaries for other languages install from the launcher."},
                    {"title": "Templates", "text": "Start a newsletter, flyer or bulletin from a ready-made page design."},
                    {"title": "Export to PDF", "text": "Save a publication as a PDF with its fonts embedded, ready for a print shop."},
                    {"title": "Open source, works offline", "text": "Licensed GPL-3.0. Nothing is sent anywhere unless you ask for it."}
                ],
                "channels": {
                    "stable": stable,
                    "beta": [{
                        "version": "0.3.0-beta.1", "released": "2027-01-10", "prerelease": true,
                        "notes": "## Added\n- Data tab preview (#240)\n",
                        "changelog_url": "https://github.com/example/kernpage/blob/main/CHANGELOG.md",
                        "files": [file(b, "kernpage-0.3.0-beta.1-x64.msi", "msi", "windows", false, 31_100_000)]
                    }]
                }
            },
            {
                "id": "dict-en-us", "kind": "dictionary", "name": "Spelling dictionary: English (US)",
                "tagline": "Hunspell dictionary for American English",
                "description": "Spelling and hyphenation for American English. Installs per user; no administrator rights needed.",
                "icon": format!("{b}/icons/dictionary.png"),
                "website": "https://example.org/kernpage/dictionaries",
                "repo": "https://github.com/example/kernpage-addons",
                "releases_url": "https://github.com/example/kernpage-addons/releases",
                "license": "see add-on",
                "languages": ["en-US"],
                "features": [
                    {"title": "American spelling", "text": "Words such as color and center are accepted; British forms are flagged."},
                    {"title": "Hyphenation", "text": "Includes hyphenation patterns for justified text."}
                ],
                "channels": {
                    "stable": [{
                        "version": "2026.10.0", "released": "2026-10-20", "prerelease": false,
                        "notes": "## Added\n- 1,200 new words from current usage (#12)\n## Fixed\n- Removed duplicate entries (#9)\n",
                        "changelog_url": "https://github.com/example/kernpage-addons/blob/main/CHANGELOG.md",
                        "files": [file(b, "dict-en-us-2026.10.0.zip", "zip", "any", false, 1_240_000)]
                    }]
                }
            }
        ]
    })
}

pub fn sample_bytes(o: &SampleOpts) -> Vec<u8> {
    serde_json::to_vec_pretty(&sample_index(o)).unwrap()
}

fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
