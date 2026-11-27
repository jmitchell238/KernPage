//! Release signing for the KernPage update feed (maintainers only; not part of any installer).
//!
//! Produces `index.json` + `index.json.minisig` in exactly the format `kernpage-launcher`
//! verifies (`crates/kernpage-launcher/src/feed.rs`): a schema-1 JSON index whose file entries
//! carry the size, SHA-256 and an inline minisign signature of the file bytes, signed as a whole
//! with a detached minisign signature.

use minisign::{PublicKey, SecretKey, SecretKeyBox};
use semver::Version;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, String>;

pub const APP_ID: &str = "kernpage";
pub const DEFAULT_REPO: &str = "jmitchell238/kernpage";
/// Two years: a release index must keep working through a long quiet stretch (see docs/RELEASING.md).
pub const DEFAULT_EXPIRES_DAYS: u64 = 730;
pub const DEFAULT_KEEP: usize = 10;

/// One installer to publish: `kind` is `msi` or `setup` (what the launcher's `file_for` matches).
pub struct FileSpec {
    pub kind: String,
    pub path: PathBuf,
}

pub struct BuildOpts {
    pub version: String,
    /// `YYYY-MM-DD`.
    pub released: String,
    pub notes: String,
    pub files: Vec<FileSpec>,
    /// `https://github.com/<repo>/releases/download/v<version>` unless overridden.
    pub base_url: String,
    pub now_unix: i64,
    pub expires_days: u64,
    /// Releases kept per channel (the new one plus older ones, for rollback).
    pub keep: usize,
}

pub struct Signed {
    pub index: Vec<u8>,
    pub sig: String,
    pub serial: u64,
}

// ---- keys -------------------------------------------------------------------------------------

/// Read a minisign secret key file's text. `password` None = an unencrypted key.
pub fn load_secret(text: &str, password: Option<&str>) -> Result<SecretKey> {
    let boxed = SecretKeyBox::from_string(text.trim_start_matches('\u{feff}')).map_err(|e| format!("unreadable secret key: {e}"))?;
    boxed
        .into_secret_key(Some(password.unwrap_or("").to_owned()))
        .map_err(|_| "could not unlock the secret key (wrong password, or it is not a minisign key)".to_owned())
}

pub fn public_of(sk: &SecretKey) -> Result<PublicKey> {
    PublicKey::from_secret_key(sk).map_err(|e| e.to_string())
}

/// The key id minisign prints, e.g. `9840E58C3F0BCDDA`.
pub fn key_id(pk: &PublicKey) -> String {
    pk.keynum().iter().rev().map(|b| format!("{b:02X}")).collect()
}

/// Generate a key pair. Returns `(secret key file text, public key file text, public base64, id)`.
pub fn generate(password: Option<&str>, label: &str) -> Result<(String, String, String, String)> {
    // `generate_unencrypted_keypair` leaves the key's checksum unwritten (such a key cannot be
    // loaded again), so a passwordless key is the encrypted form with an empty password, which
    // is what minisign's own `-W` makes and what the launcher's test keys use.
    let kp = minisign::KeyPair::generate_encrypted_keypair(Some(password.unwrap_or("").to_owned())).map_err(|e| e.to_string())?;
    let sk_text = kp.sk.to_box(Some(&format!("KernPage update signing key ({label}): SECRET"))).map_err(|e| e.to_string())?.to_string();
    let pk_box = kp.pk.to_box().map_err(|e| e.to_string())?;
    Ok((sk_text, pk_box.to_string(), kp.pk.to_base64(), key_id(&kp.pk)))
}

/// Bare base64 public keys listed in `PRODUCTION_PUBLIC_KEYS` of the launcher's `feed.rs`.
pub fn launcher_trusted_keys(feed_rs: &str) -> Result<Vec<String>> {
    let start = feed_rs.find("PRODUCTION_PUBLIC_KEYS: &[&str]").ok_or("PRODUCTION_PUBLIC_KEYS not found in feed.rs")?;
    let body = &feed_rs[start..];
    let end = body.find("];").ok_or("PRODUCTION_PUBLIC_KEYS list is not closed")?;
    let mut keys = Vec::new();
    for line in body[..end].lines() {
        let l = line.trim();
        if l.starts_with("//") {
            continue;
        }
        if let Some(rest) = l.strip_prefix('"') {
            if let Some(k) = rest.split('"').next() {
                if k.starts_with("RW") {
                    keys.push(k.to_owned());
                }
            }
        }
    }
    if keys.is_empty() {
        return Err("no production keys listed in feed.rs".into());
    }
    Ok(keys)
}

// ---- signing ------------------------------------------------------------------------------------

fn sign_bytes(sk: &SecretKey, data: &[u8], what: &str) -> Result<String> {
    minisign::sign(None, sk, std::io::Cursor::new(data), Some(&format!("kernpage update: {what}")), Some(what))
        .map(|s| s.to_string())
        .map_err(|e| format!("signing {what} failed: {e}"))
}

fn file_entry(sk: &SecretKey, spec: &FileSpec, base_url: &str) -> Result<Value> {
    let name = spec.path.file_name().and_then(|n| n.to_str()).ok_or("bad file name")?;
    let mut bytes = Vec::new();
    std::fs::File::open(&spec.path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| format!("cannot read {}: {e}", spec.path.display()))?;
    if bytes.is_empty() {
        return Err(format!("{} is empty", spec.path.display()));
    }
    Ok(json!({
        "os": "windows", "arch": "x86_64", "kind": spec.kind,
        "url": format!("{}/{}", base_url.trim_end_matches('/'), name),
        "size": bytes.len() as u64,
        "sha256": hex(Sha256::digest(&bytes)),
        "sig": sign_bytes(sk, &bytes, name)?
    }))
}

/// Check a previous index (and its `.minisig`) was signed by one of `keys` (the signing key, or
/// every key the launcher trusts, so the first release after a key rotation can build on the
/// last one); returns it parsed.
pub fn verify_previous(index: &[u8], sig_text: &str, keys: &[PublicKey]) -> Result<Value> {
    let sig = minisign::SignatureBox::from_string(sig_text).map_err(|e| format!("previous signature unreadable: {e}"))?;
    let ok = keys.iter().any(|k| minisign::verify(k, &sig, std::io::Cursor::new(index), true, false, false).is_ok());
    if !ok {
        return Err("the previous index was not signed by a trusted key; refusing to build on it".to_owned());
    }
    serde_json::from_slice(index).map_err(|e| format!("previous index is not valid JSON: {e}"))
}

fn default_item() -> Value {
    json!({
        "id": APP_ID, "kind": "app", "name": "KernPage",
        "tagline": "Desktop publishing for bulletins, newsletters and flyers. Opens Publisher (.pub) files too.",
        "website": "https://github.com/jmitchell238/kernpage",
        "repo": "https://github.com/jmitchell238/kernpage",
        "releases_url": "https://github.com/jmitchell238/kernpage/releases",
        "license": "MIT OR Apache-2.0",
        "features": [
            {"title": "Opens Publisher files", "text": "Reads .pub files and shows text, shapes, pictures, tables, groups and linked text boxes."},
            {"title": "Edits and saves .pub", "text": "Change text and formatting, move and resize objects, and save back into the same file."},
            {"title": "Open source, works offline", "text": "Licensed MIT or Apache-2.0. Nothing is sent anywhere unless you ask for it."}
        ],
        "channels": {}
    })
}

/// Build and sign the index for one release. `previous` is an already verified earlier index
/// whose older releases (and other items) are carried over so people can go back.
pub fn build_and_sign(o: &BuildOpts, sk: &SecretKey, previous: Option<&Value>) -> Result<Signed> {
    let ver = Version::parse(&o.version).map_err(|e| format!("bad version {:?}: {e}", o.version))?;
    if o.files.is_empty() {
        return Err("no installer files given".into());
    }
    let channel = if ver.pre.is_empty() { "stable" } else { "beta" };
    let files = o.files.iter().map(|f| file_entry(sk, f, &o.base_url)).collect::<Result<Vec<_>>>()?;
    let release = json!({
        "version": o.version, "released": o.released, "prerelease": !ver.pre.is_empty(),
        "notes": o.notes,
        "changelog_url": format!("https://github.com/{DEFAULT_REPO}/releases/tag/v{}", o.version),
        "files": files
    });

    let mut items: Vec<Value> = previous.and_then(|p| p["items"].as_array().cloned()).unwrap_or_default();
    let pos = match items.iter().position(|i| i["id"] == APP_ID) {
        Some(p) => p,
        None => {
            items.insert(0, default_item());
            0
        }
    };
    let app = &mut items[pos];
    if !app["channels"].is_object() {
        app["channels"] = json!({});
    }
    let channels = app["channels"].as_object_mut().ok_or("previous index: channels is not an object")?;
    let list = channels.entry(channel).or_insert_with(|| json!([]));
    let mut releases: Vec<Value> = list.as_array().cloned().unwrap_or_default();
    releases.retain(|r| r["version"] != o.version.as_str());
    releases.push(release);
    releases.sort_by(|a, b| {
        let v = |r: &Value| r["version"].as_str().and_then(|s| Version::parse(s).ok());
        v(b).cmp(&v(a))
    });
    releases.truncate(o.keep.max(1));
    *list = Value::Array(releases);

    let prev_serial = previous.and_then(|p| p["serial"].as_u64()).unwrap_or(0);
    // The serial is the signing time, so it rises with every release without any stored state.
    let serial = (o.now_unix.max(0) as u64).max(prev_serial + 1);
    let pk = public_of(sk)?;
    let blocklist = previous.map(|p| p["blocklist"].clone()).filter(|b| b.is_array()).unwrap_or_else(|| json!([]));
    let index = json!({
        "schema": 1,
        "serial": serial,
        "generated": iso(o.now_unix),
        "expires": iso(o.now_unix + (o.expires_days as i64) * 86_400),
        "keys": { "current": pk.to_base64(), "next": null },
        "blocklist": blocklist,
        "items": items
    });
    let index = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
    let sig = sign_bytes(sk, &index, "index.json")?;
    Ok(Signed { index, sig, serial })
}

pub fn write_outputs(s: &Signed, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("index.json"), &s.index).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("index.json.minisig"), &s.sig).map_err(|e| e.to_string())
}

// ---- time -----------------------------------------------------------------------------------------

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DDTHH:MM:SSZ` (UTC) from Unix seconds.
pub fn iso(t: i64) -> String {
    let (y, m, d) = civil_from_days(t.div_euclid(86_400));
    let s = t.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

pub fn date_of(t: i64) -> String {
    iso(t)[..10].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_formats_known_times() {
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso(1_800_000_000), "2027-01-15T08:00:00Z");
        assert_eq!(date_of(1_800_000_000), "2027-01-15");
    }

    #[test]
    fn reads_the_keys_listed_in_feed_rs() {
        let src = "pub const PRODUCTION_PUBLIC_KEYS: &[&str] = &[\n    // c\n    \"RWAAAA\",\n    \"RWBBBB\", // x\n];\nconst X: &str = \"RWZZZZ\";";
        assert_eq!(launcher_trusted_keys(src).unwrap(), vec!["RWAAAA", "RWBBBB"]);
    }

    #[test]
    fn unencrypted_and_encrypted_keys_load() {
        let (sk, pk, b64, id) = generate(None, "t").unwrap();
        assert!(pk.contains(&b64) && id.len() == 16);
        assert_eq!(public_of(&load_secret(&sk, None).unwrap()).unwrap().to_base64(), b64);
        let (sk2, _, _, _) = generate(Some("pw"), "t").unwrap();
        assert!(load_secret(&sk2, Some("pw")).is_ok());
        assert!(load_secret(&sk2, Some("nope")).is_err());
    }
}

/// Lower-case hex of a digest.
pub fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
