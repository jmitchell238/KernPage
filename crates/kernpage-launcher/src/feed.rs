//! The signed update index: parsing, signature and freshness checks, and the version logic.
//! See docs/research/launcher-and-addons.md section 7 for the format.
//!
//! Order matters: the signature is verified over the raw bytes BEFORE anything is parsed, so
//! unverified data never reaches the JSON parser or the UI.

use minisign_verify::{PublicKey, Signature};
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// True in debug builds, in test builds and in builds made with the `dev-feed` cargo feature:
/// only then does the launcher know about loopback test feeds (`--feed-url`, the TEST-ONLY key,
/// plain-http file links). A normal release build has none of that: it trusts only
/// `PRODUCTION_PUBLIC_KEYS` and speaks only to its own `FEED_URL` (security review M3).
pub const DEV_FEEDS: bool = cfg!(any(debug_assertions, feature = "dev-feed"));

/// Production update-signing public keys, bare minisign base64 (the second line of a `.pub`
/// file). A feed or a download verifies when ANY listed key signed it, which is how a key is
/// rotated: ship a launcher that lists the new key next to the old one, then sign with the new
/// one. Keep a backup key listed so a lost or leaked primary can be retired without stranding
/// installed launchers (security review H1). See packaging/windows/INSTALLATION.md ("Update
/// keys and feeds").
///
/// Primary (signs releases in CI, key id 2AE2B2F3B1D1F1D0) first, then the offline backup (key id
/// 43F3DD9F4C08A821, kept for rotation). Secrets live outside the repo; see docs/RELEASING.md.
pub const PRODUCTION_PUBLIC_KEYS: &[&str] = &[
    // primary (2AE2B2F3B1D1F1D0)
    "RWTQ8dGx87LiKogXdeyOtwreLnhpgHKVj7k164NmnMZI/Hyf4ViDST+w",
    // backup (43F3DD9F4C08A821), offline
    "RWQhqAhMn93zQ8kMSER4rLeYLIsD+kccIEDzRP1JmsqCYQiuQAtqXtF+",
];

/// TEST-ONLY public key (see tests/fixtures). Trusted only for loopback feeds, so a local
/// fake feed can be exercised without the production secret. Not compiled into normal release
/// builds.
#[cfg(any(debug_assertions, feature = "dev-feed"))]
const TEST_PUBLIC_KEY_FILE: &str = include_str!("../tests/fixtures/test-signing.pub");

/// Production feed location: the `index.json` asset of the newest GitHub release (and its
/// `index.json.minisig` beside it), so no server is needed. GitHub redirects these downloads to
/// its asset hosts (HTTPS); the launcher follows redirects, and everything is verified by the
/// minisign signature, so the redirect target is not trusted. Set `KERNPAGE_FEED_URL` at build
/// time to use another feed.
pub const FEED_URL: &str = match option_env!("KERNPAGE_FEED_URL") {
    Some(u) => u,
    None => "https://github.com/jmitchell238/kernpage/releases/latest/download/index.json",
};

/// Lower-case hex of a digest.
pub fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

pub const SUPPORTED_SCHEMA: u32 = 1;

/// `kind` of the Setup.exe (Burn bundle) entry of a release, next to the `msi` entry.
pub const SETUP_KIND: &str = "setup";

// ---- errors --------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedError {
    /// Network problem (DNS, connect, timeout, proxy).
    Unreachable(String),
    /// The server answered with an error status.
    Http(u16),
    /// Signature missing, damaged or not matching the content.
    BadSignature,
    /// Signed by a key this launcher does not trust.
    WrongKey,
    Expired { expires: String },
    /// Older than an index already seen.
    Rollback { serial: u64, seen: u64 },
    Malformed(String),
    UnsupportedSchema(u32),
}

impl fmt::Display for FeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeedError::Unreachable(d) => write!(f, "Could not reach the update server ({d})."),
            FeedError::Http(c) => write!(f, "The update server answered with an error (HTTP {c})."),
            FeedError::BadSignature => write!(f, "Could not verify the update list, so it was ignored. Nothing was changed."),
            FeedError::WrongKey => write!(f, "The update list was signed with a key this launcher does not trust, so it was ignored."),
            FeedError::Expired { expires } => write!(f, "The update list has expired ({expires}). Check the computer's date and try again later."),
            FeedError::Rollback { serial, seen } => write!(f, "The update list is older than one already seen (number {serial}, newest seen {seen}), so it was ignored."),
            FeedError::Malformed(d) => write!(f, "The update list is damaged or in an unknown format ({d})."),
            FeedError::UnsupportedSchema(s) => write!(f, "The update list uses a newer format (version {s}). Update the launcher to read it."),
        }
    }
}

impl FeedError {
    /// A few words for the footer: "Couldn't check: <this>".
    pub fn short(&self) -> &'static str {
        match self {
            FeedError::Unreachable(_) => "update server not reachable",
            FeedError::Http(_) => "update server returned an error",
            FeedError::BadSignature | FeedError::WrongKey => "update list could not be verified",
            FeedError::Expired { .. } => "update list has expired",
            FeedError::Rollback { .. } => "update list is out of date",
            FeedError::Malformed(_) | FeedError::UnsupportedSchema(_) => "update list is damaged",
        }
    }
}

impl std::error::Error for FeedError {}

// ---- data ----------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // fields kept for the format; icons/licence are used by later steps
pub struct Feed {
    pub schema: u32,
    pub serial: u64,
    #[serde(default)]
    pub generated: String,
    pub expires: String,
    #[serde(default)]
    pub blocklist: Vec<BlockEntry>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockEntry {
    pub id: String,
    /// A semver requirement such as `<1.2.0`.
    pub versions: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub releases_url: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub features: Vec<FeatureEntry>,
    #[serde(default)]
    pub channels: BTreeMap<String, Vec<Release>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeatureEntry {
    pub title: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub released: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub changelog_url: Option<String>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
    /// Filled in by validation: why this release is not offered (empty = fine).
    #[serde(skip)]
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FileEntry {
    pub os: String,
    #[serde(default = "any")]
    pub arch: String,
    #[serde(default)]
    pub kind: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub sig: String,
}

impl FileEntry {
    /// A plain Windows Installer package (never run on a Setup.exe install).
    pub fn is_msi(&self) -> bool {
        self.kind == "msi" || self.url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase().ends_with(".msi")
    }
}

fn any() -> String {
    "any".to_owned()
}

impl Release {
    pub fn semver(&self) -> Option<Version> {
        Version::parse(&self.version).ok()
    }

    /// The download for this computer (Windows x86_64 here, or an "any" file). On an install
    /// made by Setup.exe that is the `setup` file, otherwise the MSI (see `file_for`).
    pub fn file_for_this_platform(&self) -> Option<&FileEntry> {
        self.file_for(crate::install::installed_via_bundle())
    }

    /// The download for this platform. `bundle` = the app was installed by Setup.exe (a Burn
    /// bundle that must stay the owner of the install): only a file of kind `setup` will do,
    /// because running the plain MSI would replace the bundle's package and orphan it. For a
    /// plain MSI install any other file is used (the `setup` file never is). Files are matched
    /// in feed order, so an older launcher that knows nothing of `setup` still gets the MSI.
    pub fn file_for(&self, bundle: bool) -> Option<&FileEntry> {
        let os = if cfg!(windows) { "windows" } else { std::env::consts::OS };
        let arch = std::env::consts::ARCH;
        self.files
            .iter()
            .filter(|f| if bundle { !f.is_msi() } else { f.kind != SETUP_KIND })
            .find(|f| (f.os == os || f.os == "any") && (f.arch == arch || f.arch == "any"))
    }

    pub fn platforms(&self) -> String {
        let mut names: Vec<String> = Vec::new();
        for f in &self.files {
            let n = match (f.os.as_str(), f.arch.as_str()) {
                ("windows", "x86_64") => "Windows 64-bit".to_owned(),
                ("windows", _) => "Windows".to_owned(),
                ("any", _) => "All platforms".to_owned(),
                (os, a) => format!("{os} {a}"),
            };
            if !names.contains(&n) {
                names.push(n);
            }
        }
        names.join(", ")
    }
}

impl Item {
    pub fn is_app(&self) -> bool {
        self.kind == "app"
    }
}

// ---- verification ----------------------------------------------------------------------------

/// Decode a public key given either as a bare base64 key or as a `.pub` file's text.
pub fn parse_public_key(text: &str) -> Result<PublicKey, FeedError> {
    let t = text.trim();
    let key = if t.contains('\n') { PublicKey::decode(t) } else { PublicKey::from_base64(t) };
    key.map_err(|_| FeedError::BadSignature)
}

/// A loopback test-feed address. Always false in a normal release build (`DEV_FEEDS`).
pub fn is_loopback_url(url: &str) -> bool {
    DEV_FEEDS && host_is_loopback(url)
}

fn host_is_loopback(url: &str) -> bool {
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")).unwrap_or("");
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = if let Some(h) = host.strip_prefix('[') {
        h.split(']').next().unwrap_or("")
    } else {
        host.split(':').next().unwrap_or(host)
    };
    host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost") || host == "::1"
}

/// The keys a feed URL (and the files it lists) is verified against: only the test key for a
/// loopback test feed (dev builds), else every production key.
pub fn trusted_keys_for_url(url: &str) -> Result<Vec<PublicKey>, FeedError> {
    #[cfg(any(debug_assertions, feature = "dev-feed"))]
    if is_loopback_url(url) {
        return Ok(vec![parse_public_key(TEST_PUBLIC_KEY_FILE)?]);
    }
    let _ = url;
    PRODUCTION_PUBLIC_KEYS.iter().map(|k| parse_public_key(k)).collect()
}

/// Check `index` against its `.minisig` text. Any one of `keys` may have signed it.
pub fn verify_signature(index: &[u8], sig_text: &str, keys: &[PublicKey]) -> Result<(), FeedError> {
    let sig = Signature::decode(sig_text).map_err(|_| FeedError::BadSignature)?;
    let mut result = Err(FeedError::WrongKey);
    for key in keys {
        match key.verify(index, &sig, false) {
            Ok(()) => return Ok(()),
            Err(minisign_verify::Error::UnexpectedKeyId) => {}
            Err(_) => result = Err(FeedError::BadSignature),
        }
    }
    result
}

/// Verify, parse and sanity-check an index. `last_serial` is the highest serial seen before
/// (0 if none); an equal serial is fine (same index again), a lower one is refused.
pub fn parse_verified(index: &[u8], sig_text: &str, keys: &[PublicKey], now: i64, last_serial: u64) -> Result<Feed, FeedError> {
    verify_signature(index, sig_text, keys)?;
    let mut feed: Feed = serde_json::from_slice(index).map_err(|e| FeedError::Malformed(e.to_string()))?;
    if feed.schema != SUPPORTED_SCHEMA {
        return Err(FeedError::UnsupportedSchema(feed.schema));
    }
    let expires = parse_time(&feed.expires).ok_or_else(|| FeedError::Malformed("bad expiry date".into()))?;
    if expires <= now {
        return Err(FeedError::Expired { expires: feed.expires.clone() });
    }
    if feed.serial < last_serial {
        return Err(FeedError::Rollback { serial: feed.serial, seen: last_serial });
    }
    for b in &feed.blocklist {
        if VersionReq::parse(&b.versions).is_err() {
            return Err(FeedError::Malformed(format!("bad blocklist rule for {}", b.id)));
        }
    }
    validate_releases(&mut feed);
    Ok(feed)
}

/// Flag releases that must not be offered: bad version, no files, a SHA-256 that is not 64 hex
/// digits, a missing or undecodable signature, or a non-HTTPS link (loopback http is allowed).
fn validate_releases(feed: &mut Feed) {
    for item in &mut feed.items {
        for releases in item.channels.values_mut() {
            for r in releases.iter_mut() {
                if r.semver().is_none() {
                    r.problems.push(format!("\"{}\" is not a valid version", r.version));
                }
                if r.files.is_empty() {
                    r.problems.push("it has no download files".into());
                }
                for (i, f) in r.files.iter().enumerate() {
                    let n = i + 1;
                    if f.sha256.len() != 64 || !f.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                        r.problems.push(format!("file {n} has an invalid SHA-256 checksum"));
                    }
                    if f.sig.trim().is_empty() || Signature::decode(&f.sig).is_err() {
                        r.problems.push(format!("file {n} has no valid signature"));
                    }
                    if !(f.url.starts_with("https://") || (f.url.starts_with("http://") && is_loopback_url(&f.url))) {
                        r.problems.push(format!("file {n} is not served over HTTPS"));
                    }
                    if f.size == 0 {
                        r.problems.push(format!("file {n} has no size"));
                    }
                }
            }
        }
    }
}

// ---- version logic -----------------------------------------------------------------------------

pub struct Candidate<'a> {
    pub version: Version,
    pub release: &'a Release,
}

impl Feed {
    /// The blocklist rule matching this item version, if any.
    pub fn blocked(&self, id: &str, v: &Version) -> Option<&BlockEntry> {
        self.blocklist
            .iter()
            .find(|b| b.id == id && VersionReq::parse(&b.versions).map(|r| r.matches(v)).unwrap_or(false))
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Releases of `item` that may be offered, newest first: valid, not blocklisted, and
    /// pre-releases (and non-stable channels) only when `include_pre` is set.
    pub fn candidates<'a>(&'a self, item: &'a Item, include_pre: bool) -> Vec<Candidate<'a>> {
        let mut out = Vec::new();
        for (channel, releases) in &item.channels {
            for r in releases {
                let Some(v) = r.semver() else { continue };
                let pre = r.prerelease || !v.pre.is_empty() || channel != "stable";
                if (pre && !include_pre) || !r.problems.is_empty() || self.blocked(&item.id, &v).is_some() {
                    continue;
                }
                out.push(Candidate { version: v, release: r });
            }
        }
        out.sort_by(|a, b| b.version.cmp(&a.version));
        out
    }

    /// Older releases the user may go back to: strictly older than `installed`, offered
    /// (valid, not blocklisted), with a download for this computer, one entry per version,
    /// newest first. Pre-releases only when `include_pre` is set.
    pub fn rollback_candidates<'a>(&'a self, item: &'a Item, installed: &Version, include_pre: bool) -> Vec<Candidate<'a>> {
        let mut out: Vec<Candidate<'a>> = Vec::new();
        for c in self.candidates(item, include_pre) {
            if c.version < *installed && c.release.file_for_this_platform().is_some() && !out.iter().any(|o| o.version == c.version) {
                out.push(c);
            }
        }
        out
    }

    /// The offered release of `item` with exactly this version, if any.
    pub fn release_for<'a>(&'a self, item: &'a Item, version: &Version, include_pre: bool) -> Option<&'a Release> {
        self.candidates(item, include_pre).into_iter().find(|c| c.version == *version).map(|c| c.release)
    }

    pub fn latest<'a>(&'a self, item: &'a Item, include_pre: bool) -> Option<Candidate<'a>> {
        self.candidates(item, include_pre).into_iter().next()
    }

    /// Releases that look newer than `than` but were refused (flagged or blocklisted); the
    /// reason of the newest one. Lets the card say why no update is offered.
    pub fn refused_newer(&self, item: &Item, than: Option<&Version>, include_pre: bool) -> Option<String> {
        let mut best: Option<(Version, String)> = None;
        for (channel, releases) in &item.channels {
            for r in releases {
                let Some(v) = r.semver() else { continue };
                let pre = r.prerelease || !v.pre.is_empty() || channel != "stable";
                if pre && !include_pre {
                    continue;
                }
                if than.is_some_and(|t| v <= *t) {
                    continue;
                }
                let why = if !r.problems.is_empty() {
                    format!("Version {v} was found but failed its checks ({}), so it is not offered.", r.problems[0])
                } else if let Some(b) = self.blocked(&item.id, &v) {
                    format!("Version {v} is blocked: {}", if b.reason.is_empty() { "withdrawn by the publisher" } else { &b.reason })
                } else {
                    continue;
                };
                if best.as_ref().is_none_or(|(bv, _)| v > *bv) {
                    best = Some((v, why));
                }
            }
        }
        best.map(|(_, w)| w)
    }
}

// ---- notes (short structured Markdown) ----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub section: String,
    pub text: String,
}

/// Bullets of the release notes, each tagged with the `##` heading above it.
pub fn parse_notes(md: &str) -> Vec<Change> {
    let mut section = String::new();
    let mut out = Vec::new();
    for line in md.lines() {
        let l = line.trim();
        if let Some(h) = l.strip_prefix('#') {
            section = h.trim_start_matches('#').trim().to_owned();
        } else if let Some(b) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
            let text = b.replace("**", "").replace('`', "");
            if !text.trim().is_empty() {
                out.push(Change { section: section.clone(), text: text.trim().to_owned() });
            }
        }
    }
    out
}

// ---- time ----------------------------------------------------------------------------------------

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ` (UTC) to Unix seconds.
pub fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let (date, time) = match s.split_once('T') {
        Some((d, t)) => (d, t.trim_end_matches('Z')),
        None => (s, "00:00:00"),
    };
    let mut dp = date.split('-');
    let (y, m, d): (i64, i64, i64) = (dp.next()?.parse().ok()?, dp.next()?.parse().ok()?, dp.next()?.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut tp = time.split(':');
    let (hh, mm, ss): (i64, i64, i64) = (
        tp.next()?.parse().ok()?,
        tp.next().unwrap_or("0").parse().ok()?,
        tp.next().unwrap_or("0").split('.').next()?.parse().ok()?,
    );
    Some(days_from_civil(y, m, d) * 86400 + hh * 3600 + mm * 60 + ss)
}

/// "Dec 1, 2026" from `2026-12-01` (or the input unchanged when it is not a date).
pub fn pretty_date(s: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let date = s.split('T').next().unwrap_or(s);
    let mut p = date.split('-');
    match (p.next().and_then(|y| y.parse::<i32>().ok()), p.next().and_then(|m| m.parse::<usize>().ok()), p.next().and_then(|d| d.parse::<u32>().ok())) {
        (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) => format!("{} {d}, {y}", MONTHS[m - 1]),
        _ => s.to_owned(),
    }
}

pub fn human_size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= MB {
        format!("{:.1} MB", bytes as f64 / MB)
    } else {
        format!("{} KB", (bytes / 1024).max(1))
    }
}

/// "just now", "5 minutes ago", "3 hours ago", "2 days ago".
pub fn ago(secs: i64) -> String {
    let plural = |n: i64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match secs.max(0) {
        s if s < 60 => "just now".to_owned(),
        s if s < 3600 => plural(s / 60, "minute"),
        s if s < 86400 => plural(s / 3600, "hour"),
        s => plural(s / 86400, "day"),
    }
}

// ---- tests ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devfeed::*;
    use sha2::Digest;

    const NOW: i64 = 1_800_000_000; // 2027-01-15

    fn key() -> Vec<PublicKey> {
        vec![parse_public_key(TEST_PUBLIC).unwrap()]
    }

    fn signed(o: &SampleOpts) -> (Vec<u8>, String) {
        let b = sample_bytes(o);
        let s = sign_test(&b);
        (b, s)
    }

    fn opts() -> SampleOpts {
        SampleOpts::new(5, "http://127.0.0.1:1")
    }

    #[test]
    fn good_index_is_accepted() {
        let (b, s) = signed(&opts());
        let f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        assert_eq!(f.serial, 5);
        assert_eq!(f.items.len(), 2);
        let app = f.item("kernpage").unwrap();
        assert_eq!(app.features.len(), 6);
        assert!(app.channels["stable"][0].problems.is_empty());
    }

    #[test]
    fn tampered_index_is_refused() {
        let (mut b, s) = signed(&opts());
        let at = b.windows(5).position(|w| w == b"0.2.0").unwrap();
        b[at + 2] = b'9';
        assert_eq!(parse_verified(&b, &s, &key(), NOW, 0).unwrap_err(), FeedError::BadSignature);
    }

    #[test]
    fn wrong_key_is_refused() {
        let b = sample_bytes(&opts());
        let s = sign_with(OTHER_SECRET, &b);
        assert_eq!(parse_verified(&b, &s, &key(), NOW, 0).unwrap_err(), FeedError::WrongKey);
    }

    #[test]
    fn missing_or_garbage_signature_is_refused() {
        let b = sample_bytes(&opts());
        assert_eq!(parse_verified(&b, "", &key(), NOW, 0).unwrap_err(), FeedError::BadSignature);
        assert_eq!(parse_verified(&b, "not a signature", &key(), NOW, 0).unwrap_err(), FeedError::BadSignature);
    }

    #[test]
    fn production_keys_never_verify_a_test_feed() {
        let (b, s) = signed(&opts());
        let prod = trusted_keys_for_url(FEED_URL).unwrap();
        assert_eq!(prod.len(), PRODUCTION_PUBLIC_KEYS.len());
        assert_eq!(parse_verified(&b, &s, &prod, NOW, 0).unwrap_err(), FeedError::WrongKey);
    }

    #[test]
    fn any_listed_key_verifies_so_keys_can_be_rotated() {
        let (b, s) = signed(&opts());
        let placeholder = parse_public_key(PRODUCTION_PUBLIC_KEYS[0]).unwrap(); // the primary
        let test = parse_public_key(TEST_PUBLIC).unwrap();
        // Signed with the second listed key (a backup / the new key): accepted, either order.
        assert!(parse_verified(&b, &s, &[placeholder.clone(), test.clone()], NOW, 0).is_ok());
        assert!(parse_verified(&b, &s, &[test.clone(), placeholder.clone()], NOW, 0).is_ok());
        // Signed with a key that is in neither slot: refused as the wrong key.
        let other = parse_public_key(OTHER_PUBLIC).unwrap();
        assert_eq!(parse_verified(&b, &s, &[placeholder.clone(), other], NOW, 0).unwrap_err(), FeedError::WrongKey);
        // A tampered index fails with the right key listed among others.
        let mut bad = b.clone();
        bad.push(b'x');
        assert_eq!(parse_verified(&bad, &s, &[placeholder, test], NOW, 0).unwrap_err(), FeedError::BadSignature);
        assert_eq!(parse_verified(&b, &s, &[], NOW, 0).unwrap_err(), FeedError::WrongKey);
    }

    #[test]
    fn production_keys_are_valid_and_never_the_test_key() {
        assert!(!PRODUCTION_PUBLIC_KEYS.is_empty());
        let test = TEST_PUBLIC.lines().nth(1).unwrap().trim();
        for k in PRODUCTION_PUBLIC_KEYS {
            parse_public_key(k).unwrap();
            assert_ne!(*k, test);
        }
    }

    // ---- round trip with the production release tool (kernpage-release) ----------------------

    fn temp_file(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pw-release-rt-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("KernPage-{tag}"));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    fn release_opts(version: &str, msi: &[u8], setup: &[u8], now: i64) -> kernpage_release::BuildOpts {
        use kernpage_release::*;
        BuildOpts {
            version: version.into(),
            released: "2027-01-15".into(),
            notes: "## Added
- A thing
".into(),
            files: vec![
                FileSpec { kind: "msi".into(), path: temp_file(&format!("{version}-x64.msi"), msi) },
                FileSpec { kind: "setup".into(), path: temp_file(&format!("{version}-Setup.exe"), setup) },
            ],
            base_url: format!("https://github.com/jmitchell238/kernpage/releases/download/v{version}"),
            now_unix: now,
            expires_days: DEFAULT_EXPIRES_DAYS,
            keep: DEFAULT_KEEP,
        }
    }

    #[test]
    fn release_tool_output_verifies_with_the_launchers_own_code() {
        let (sk_text, _, b64, _) = kernpage_release::generate(None, "throwaway").unwrap();
        let sk = kernpage_release::load_secret(&sk_text, None).unwrap();
        let keys = vec![parse_public_key(&b64).unwrap()];
        let signed = kernpage_release::build_and_sign(&release_opts("0.2.0", b"msi bytes", b"setup bytes", NOW), &sk, None).unwrap();
        let f = parse_verified(&signed.index, &signed.sig, &keys, NOW, 0).unwrap();
        let item = f.item("kernpage").unwrap();
        let r = &item.channels["stable"][0];
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(f.latest(item, false).unwrap().version.to_string(), "0.2.0");
        // MSI for a plain install, Setup.exe for a bundle install; sizes and hashes match the files.
        let msi = r.file_for(false).unwrap();
        let setup = r.file_for(true).unwrap();
        assert!(msi.is_msi() && msi.url.starts_with("https://github.com/jmitchell238/kernpage/releases/download/v0.2.0/"));
        assert_eq!(setup.kind, SETUP_KIND);
        assert_eq!(msi.size, 9);
        assert_eq!(setup.sha256, hex(sha2::Sha256::digest(b"setup bytes")));
        // The inline file signature verifies over the file bytes, with the same key list.
        Signature::decode(&msi.sig).and_then(|s| keys[0].verify(b"msi bytes", &s, false)).unwrap();
        assert!(Signature::decode(&msi.sig).and_then(|s| keys[0].verify(b"other", &s, false)).is_err());
        // The index is valid for about two years, and a production-keys-only launcher refuses it.
        assert!(parse_time(&f.expires).unwrap() - NOW > 700 * 86_400);
        let prod = trusted_keys_for_url(FEED_URL).unwrap();
        assert_eq!(parse_verified(&signed.index, &signed.sig, &prod, NOW, 0).unwrap_err(), FeedError::WrongKey);
        // A tampered index fails.
        let mut bad = signed.index.clone();
        bad.push(b' ');
        assert_eq!(parse_verified(&bad, &signed.sig, &keys, NOW, 0).unwrap_err(), FeedError::BadSignature);
    }

    #[test]
    fn release_tool_keeps_older_releases_and_raises_the_serial() {
        let (sk_text, _, b64, _) = kernpage_release::generate(Some("pw"), "throwaway2").unwrap();
        let sk = kernpage_release::load_secret(&sk_text, Some("pw")).unwrap();
        let pk = kernpage_release::public_of(&sk).unwrap();
        let keys = vec![parse_public_key(&b64).unwrap()];
        let first = kernpage_release::build_and_sign(&release_opts("0.2.0", b"a", b"b", NOW), &sk, None).unwrap();
        let prev = kernpage_release::verify_previous(&first.index, &first.sig, &[pk.clone()]).unwrap();
        // Same second on purpose: the serial must still go up.
        let second = kernpage_release::build_and_sign(&release_opts("0.3.0", b"c", b"d", NOW), &sk, Some(&prev)).unwrap();
        assert!(second.serial > first.serial);
        let f = parse_verified(&second.index, &second.sig, &keys, NOW, first.serial).unwrap();
        let item = f.item("kernpage").unwrap();
        assert_eq!(f.latest(item, false).unwrap().version.to_string(), "0.3.0");
        let installed = Version::parse("0.3.0").unwrap();
        let back = f.rollback_candidates(item, &installed, false);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].version.to_string(), "0.2.0");
        // The older index (lower serial) is refused as a rollback once the newer one was seen.
        assert!(matches!(parse_verified(&first.index, &first.sig, &keys, NOW, second.serial).unwrap_err(), FeedError::Rollback { .. }));
        // A previous index signed by a different key is not built upon.
        let (other_text, _, _, _) = kernpage_release::generate(None, "other").unwrap();
        let other = kernpage_release::public_of(&kernpage_release::load_secret(&other_text, None).unwrap()).unwrap();
        assert!(kernpage_release::verify_previous(&first.index, &first.sig, &[other]).is_err());
    }

    #[test]
    fn production_keys_and_feed_url_are_the_real_ones() {
        assert_eq!(PRODUCTION_PUBLIC_KEYS.len(), 2, "primary + backup");
        assert!(FEED_URL.starts_with("https://github.com/jmitchell238/kernpage/releases/latest/download/"));
        assert!(FEED_URL.ends_with("/index.json"));
        // The release tool reads the same list out of this very file.
        let src = include_str!("feed.rs");
        let listed = kernpage_release::launcher_trusted_keys(src).unwrap();
        assert_eq!(listed, PRODUCTION_PUBLIC_KEYS);
    }

    #[test]
    fn expired_index_is_refused() {
        let mut o = opts();
        o.expires = "2020-01-01T00:00:00Z".into();
        let (b, s) = signed(&o);
        assert!(matches!(parse_verified(&b, &s, &key(), NOW, 0).unwrap_err(), FeedError::Expired { .. }));
    }

    #[test]
    fn older_serial_is_refused_equal_is_fine() {
        let (b, s) = signed(&opts());
        assert_eq!(parse_verified(&b, &s, &key(), NOW, 9).unwrap_err(), FeedError::Rollback { serial: 5, seen: 9 });
        assert!(parse_verified(&b, &s, &key(), NOW, 5).is_ok());
    }

    #[test]
    fn malformed_json_is_refused_even_when_signed() {
        for body in [&b"{ not json"[..], b"{}", b"[]", b""] {
            let s = sign_test(body);
            assert!(matches!(parse_verified(body, &s, &key(), NOW, 0).unwrap_err(), FeedError::Malformed(_)), "{body:?}");
        }
        let mut v = sample_index(&opts());
        v["schema"] = 2.into();
        let b = serde_json::to_vec(&v).unwrap();
        assert_eq!(parse_verified(&b, &sign_test(&b), &key(), NOW, 0).unwrap_err(), FeedError::UnsupportedSchema(2));
    }

    #[test]
    fn wrong_sha_in_item_is_flagged_and_not_offered() {
        let mut o = opts();
        o.bad_sha = true;
        let (b, s) = signed(&o);
        let f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        let app = f.item("kernpage").unwrap();
        assert!(app.channels["stable"][0].problems.iter().any(|p| p.contains("SHA-256")));
        assert!(f.latest(app, false).is_none());
        let why = f.refused_newer(app, Some(&Version::new(0, 1, 2)), false).unwrap();
        assert!(why.contains("failed its checks"), "{why}");
        // The add-on next to it is still fine.
        assert!(f.latest(f.item("dict-en-us").unwrap(), false).is_some());
    }

    #[test]
    fn file_without_signature_is_flagged() {
        let mut v = sample_index(&opts());
        v["items"][0]["channels"]["stable"][0]["files"][0]["sig"] = "".into();
        let b = serde_json::to_vec(&v).unwrap();
        let f = parse_verified(&b, &sign_test(&b), &key(), NOW, 0).unwrap();
        assert!(!f.item("kernpage").unwrap().channels["stable"][0].problems.is_empty());
    }

    #[test]
    fn prerelease_filtering() {
        let (b, s) = signed(&opts());
        let f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        let app = f.item("kernpage").unwrap();
        assert_eq!(f.latest(app, false).unwrap().version, Version::new(0, 2, 0));
        assert_eq!(f.latest(app, true).unwrap().version, Version::parse("0.3.0-beta.1").unwrap());
        assert_eq!(f.candidates(app, false).len(), 1);
        assert_eq!(f.candidates(app, true).len(), 2);
    }

    #[test]
    fn blocklist_disables_matching_versions() {
        let mut o = opts();
        o.block_kernpage = Some("<0.3.0".into());
        let (b, s) = signed(&o);
        let f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        let app = f.item("kernpage").unwrap();
        assert!(f.latest(app, false).is_none());
        assert!(f.blocked("kernpage", &Version::new(0, 1, 2)).is_some());
        assert!(f.blocked("kernpage", &Version::new(0, 3, 0)).is_none());
        assert!(f.refused_newer(app, Some(&Version::new(0, 1, 2)), false).unwrap().contains("blocked"));
    }

    #[test]
    fn rollback_candidates_are_older_offered_this_platform_newest_first() {
        let mut o = opts();
        o.app_real = Some(RealFile { version: "0.2.0".into(), bytes: b"new".to_vec(), setup: None });
        o.app_older = Some(RealFile { version: "0.1.5".into(), bytes: b"old".to_vec(), setup: None });
        let (b, s) = signed(&o);
        let mut f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        let app = f.items.iter_mut().find(|i| i.id == "kernpage").unwrap();
        let template = app.channels["stable"][1].clone(); // the 0.1.5 release
        let stable = app.channels.get_mut("stable").unwrap();
        for v in ["0.1.0", "0.1.7", "0.2.0", "0.2.5"] {
            let mut r = template.clone();
            r.version = v.into();
            stable.push(r);
        }
        let mut other = template.clone();
        other.version = "0.1.8".into();
        other.files[0].arch = "aarch64".into(); // no download for this computer (Windows)
        stable.push(other);
        f.blocklist.push(BlockEntry { id: "kernpage".into(), versions: "<0.1.2".into(), reason: "lost data".into() });

        let app = f.item("kernpage").unwrap();
        let versions = |installed: &str, pre: bool| -> Vec<String> {
            f.rollback_candidates(app, &Version::parse(installed).unwrap(), pre).iter().map(|c| c.version.to_string()).collect()
        };
        // Newest first; 0.1.0 is blocklisted; 0.2.0 and 0.2.5 are not older than 0.2.0.
        let mut expect = vec!["0.1.7", "0.1.5"];
        if !cfg!(windows) {
            expect.insert(0, "0.1.8"); // other platforms fall back to the first file
        }
        assert_eq!(versions("0.2.0", false), expect);
        // The installed version itself is never listed; a newer installed one lists more.
        assert_eq!(versions("0.1.7", false), vec!["0.1.5"]);
        assert!(versions("0.1.5", false).is_empty());
        assert!(versions("0.2.5", false).contains(&"0.2.0".to_owned()));
        // The offered release for a version can be found again for the download.
        assert!(f.release_for(app, &Version::new(0, 1, 5), false).is_some());
        assert!(f.release_for(app, &Version::new(0, 1, 0), false).is_none());
    }

    #[test]
    fn semver_ordering() {
        let v = |s: &str| Version::parse(s).unwrap();
        assert!(v("0.10.0") > v("0.9.0"));
        assert!(v("0.2.0") > v("0.2.0-beta.2"));
        assert!(v("0.2.0-beta.10") > v("0.2.0-beta.2"));
        assert!(v("1.0.0") > v("0.99.99"));
    }

    #[test]
    fn notes_become_tagged_bullets() {
        let c = parse_notes("## Added\n- **Spell** check (#1)\n* `Find` (#2)\n\n## Fixed\n- Crash (#3)\ntext\n");
        assert_eq!(c.len(), 3);
        assert_eq!(c[0], Change { section: "Added".into(), text: "Spell check (#1)".into() });
        assert_eq!(c[1].text, "Find (#2)");
        assert_eq!(c[2].section, "Fixed");
    }

    #[test]
    fn dates_and_sizes() {
        assert_eq!(parse_time("1970-01-02"), Some(86400));
        assert_eq!(parse_time("2027-01-15T08:00:00Z"), Some(NOW));
        assert_eq!(parse_time("garbage"), None);
        assert_eq!(pretty_date("2026-12-01"), "Dec 1, 2026");
        assert_eq!(human_size(30_801_920), "29.4 MB");
        assert_eq!(ago(30), "just now");
        assert_eq!(ago(300), "5 minutes ago");
        assert_eq!(ago(3600), "1 hour ago");
    }

    #[test]
    fn loopback_detection_and_key_choice() {
        assert!(is_loopback_url("http://127.0.0.1:8750/index.json"));
        assert!(is_loopback_url("http://localhost/index.json"));
        assert!(!is_loopback_url("https://127.0.0.1.evil.example/index.json"));
        assert!(!is_loopback_url("http://user@evil.example/x?h=127.0.0.1"));
        assert!(!is_loopback_url(FEED_URL));
        assert!(DEV_FEEDS, "tests are dev builds");
        // A loopback feed is verified against the TEST key only; every other URL against production.
        let test = parse_public_key(TEST_PUBLIC_KEY_FILE).unwrap();
        let (b, s) = signed(&opts());
        let loop_keys = trusted_keys_for_url("http://127.0.0.1:1/index.json").unwrap();
        assert_eq!(loop_keys.len(), 1);
        assert!(parse_verified(&b, &s, &loop_keys, NOW, 0).is_ok());
        let _ = test;
        assert_eq!(trusted_keys_for_url(FEED_URL).unwrap().len(), PRODUCTION_PUBLIC_KEYS.len());
        assert!(host_is_loopback("http://[::1]:8/x"));
    }

    #[test]
    fn setup_file_is_chosen_only_on_a_bundle_install() {
        let mut o = opts();
        o.setup = true;
        let (b, s) = signed(&o);
        let f = parse_verified(&b, &s, &key(), NOW, 0).unwrap();
        let app = f.item("kernpage").unwrap();
        let r = f.latest(app, false).unwrap().release;
        assert_eq!(r.files.len(), 2);
        let plain = r.file_for(false).unwrap();
        assert!(plain.is_msi() && plain.kind == "msi");
        let bundled = r.file_for(true).unwrap();
        assert_eq!(bundled.kind, SETUP_KIND);
        assert!(bundled.url.ends_with("-Setup.exe"));
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        // A release that has no Setup.exe is not installable on a bundle install (the MSI would orphan it),
        // and a release with only a Setup.exe is not installable on a plain MSI install.
        let mut msi_only = r.clone();
        msi_only.files.retain(|f| f.kind != SETUP_KIND);
        assert!(msi_only.file_for(true).is_none() && msi_only.file_for(false).is_some());
        let mut setup_only = r.clone();
        setup_only.files.retain(|f| f.kind == SETUP_KIND);
        assert!(setup_only.file_for(false).is_none() && setup_only.file_for(true).is_some());
        // An older launcher (no kind logic) takes the first file: the MSI comes first in the feed.
        assert!(r.files[0].is_msi());
    }

    #[test]
    fn a_setup_entry_gets_the_same_checks_as_the_msi() {
        let mut o = opts();
        o.setup = true;
        let b = sample_bytes(&o);
        let mut v: serde_json::Value = serde_json::from_slice(&b).unwrap();
        let files = &mut v["items"][0]["channels"]["stable"][0]["files"];
        files[1]["url"] = "http://example.com/KernPage-0.2.0-Setup.exe".into();
        files[1]["sha256"] = "abc".into();
        files[1]["sig"] = "".into();
        let b = serde_json::to_vec(&v).unwrap();
        let f = parse_verified(&b, &sign_test(&b), &key(), NOW, 0).unwrap();
        let r = &f.item("kernpage").unwrap().channels["stable"][0];
        assert!(r.problems.iter().any(|p| p.contains("file 2") && p.contains("SHA-256")), "{:?}", r.problems);
        assert!(r.problems.iter().any(|p| p.contains("file 2") && p.contains("signature")));
        assert!(r.problems.iter().any(|p| p.contains("file 2") && p.contains("HTTPS")));
        assert!(f.latest(f.item("kernpage").unwrap(), false).is_none(), "a release with a bad file is not offered");
    }
}
