//! KernPage's own file format, `.kern` (documented in `docs/spec/KERN-FORMAT.md`): a ZIP
//! container holding the document as JSON plus one file per distinct picture. Reading treats
//! the bytes as untrusted: entry count, names, compression and sizes are checked and nothing
//! is ever written to disk.

use crate::model::{media, Document};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::sync::Arc;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The format version this build writes (major.minor).
pub const FORMAT_VERSION: &str = "1.0";
/// The version written when the document has master pages (ids, descriptions, two-page
/// masters and margins of masters; spec KERN-FORMAT.md, "Version 1.1").
pub const FORMAT_VERSION_MASTERS: &str = "1.1";
/// The optional feature named in the manifest of such a file.
pub const FEATURE_MASTER_PAGES: &str = "master-pages";
const MAJOR: u32 = 1;
const MINOR: u32 = 1;
pub const MIMETYPE: &str = "application/vnd.kernpage.document+zip";
const DOCUMENT_ENTRY: &str = "document.json";
const IMAGE_DIR: &str = "assets/images/";
/// Most entries a file may hold.
pub const MAX_ENTRIES: usize = 4096;
/// Most bytes all entries may add up to once unpacked (same cap as an opened `.pub`).
pub const MAX_UNCOMPRESSED_BYTES: u64 = 512 << 20;

#[derive(Debug)]
pub enum KernError {
    /// Not a KernPage file at all.
    NotKern,
    /// Made by a newer KernPage (a later major version, or needing a newer reader).
    TooNew(String),
    /// Needs a feature this KernPage does not have.
    Unsupported(String),
    /// Over the entry-count or size limits.
    TooLarge,
    /// Damaged or malformed.
    Damaged(String),
}

impl std::fmt::Display for KernError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernError::NotKern => write!(f, "this is not a KernPage file"),
            KernError::TooNew(v) => write!(
                f,
                "this file was made by a newer KernPage (file format version {v}); update KernPage to open it"
            ),
            KernError::Unsupported(what) => write!(
                f,
                "this file needs a feature this KernPage does not have (\"{what}\"); update KernPage to open it"
            ),
            KernError::TooLarge => write!(f, "the file is too large to be a KernPage publication"),
            KernError::Damaged(e) => write!(f, "the KernPage file is damaged ({e})"),
        }
    }
}

impl std::error::Error for KernError {}

pub type Result<T> = std::result::Result<T, KernError>;

#[derive(Serialize)]
struct GeneratorOut<'a> {
    application: &'a str,
    version: &'a str,
}

#[derive(Serialize)]
struct ManifestOut<'a> {
    format: &'a str,
    format_version: &'a str,
    minimum_reader_version: &'a str,
    document_id: &'a str,
    generator: GeneratorOut<'a>,
    document: &'a str,
    required_features: Vec<String>,
    optional_features: Vec<String>,
}

#[derive(Deserialize)]
struct ManifestIn {
    #[serde(default)]
    format: String,
    #[serde(default)]
    format_version: String,
    #[serde(default)]
    minimum_reader_version: Option<String>,
    #[serde(default)]
    document_id: String,
    #[serde(default)]
    document: Option<String>,
    #[serde(default)]
    required_features: Vec<String>,
}

fn damaged(e: impl std::fmt::Display) -> KernError {
    KernError::Damaged(e.to_string())
}

/// "1.0" to (1, 0); a missing minor is 0.
fn parse_version(v: &str) -> Option<(u32, u32)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = match parts.next() {
        Some(m) => m.parse().ok()?,
        None => 0,
    };
    Some((major, minor))
}

/// Whether `bytes` start like a `.kern` file: the ZIP signature, then a stored first entry
/// named `mimetype` holding the KernPage type (a cheap check; `read_kern` does the real work).
pub fn is_kern(bytes: &[u8]) -> bool {
    const NAME: &[u8] = b"mimetype";
    let at = 30 + NAME.len();
    bytes.starts_with(b"PK\x03\x04")
        && bytes.get(30..at) == Some(NAME)
        && bytes.get(at..at + MIMETYPE.len()) == Some(MIMETYPE.as_bytes())
}

/// A new random 128-bit document identifier (32 hex digits).
pub fn new_document_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h = Sha256::new();
    h.update(nanos.to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    h.update(N.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    // The address of a stack value adds a little more per-process variation.
    let local = 0u8;
    h.update((&local as *const u8 as usize).to_le_bytes());
    let digest = h.finalize();
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// Serialize a document to `.kern` bytes with a fresh document id.
pub fn write_kern(doc: &Document) -> Result<Vec<u8>> {
    write_kern_with_id(doc, &new_document_id())
}

/// Serialize a document, keeping `document_id` (the same id on every save of one document).
/// Each distinct picture is written once.
pub fn write_kern_with_id(doc: &Document, document_id: &str) -> Result<Vec<u8>> {
    let guard = media::Guard::install(media::Mode::Write(Default::default()));
    let json = serde_json::to_vec(doc).map_err(damaged);
    let pictures = guard.take_written();
    let json = json?;
    // Master pages carry more than a 1.0 reader knows (an id, a description, two-page masters,
    // margins): such a file is 1.1 and says so in an optional feature, so a 1.0 reader still
    // opens it (and shows the shapes behind the pages, as before).
    let masters = !doc.masters.is_empty();
    let manifest = serde_json::to_vec(&ManifestOut {
        format: "KERN",
        format_version: if masters { FORMAT_VERSION_MASTERS } else { FORMAT_VERSION },
        minimum_reader_version: FORMAT_VERSION,
        document_id,
        generator: GeneratorOut { application: "KernPage", version: env!("CARGO_PKG_VERSION") },
        document: DOCUMENT_ENTRY,
        required_features: Vec::new(),
        optional_features: if masters { vec![FEATURE_MASTER_PAGES.to_owned()] } else { Vec::new() },
    })
    .map_err(damaged)?;

    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let mut put = |name: &str, opts: SimpleFileOptions, data: &[u8]| -> Result<()> {
        zip.start_file(name, opts).map_err(damaged)?;
        zip.write_all(data).map_err(damaged)
    };
    put("mimetype", stored, MIMETYPE.as_bytes())?;
    put("manifest.json", deflated, &manifest)?;
    put(DOCUMENT_ENTRY, deflated, &json)?;
    for (name, bytes) in &pictures {
        // Pictures are already compressed (PNG, JPEG).
        put(name, stored, bytes)?;
    }
    Ok(zip.finish().map_err(damaged)?.into_inner())
}

/// A name that cannot climb out of or point outside the archive.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && !name.starts_with('/')
        && !name.starts_with('\\')
        && !name.contains(['\0', '\\', ':'])
        && name.split('/').all(|part| part != ".." && part != ".")
}

/// Read the whole entry, never more than `budget` bytes (which is reduced by what was read).
fn read_entry(file: &mut impl Read, budget: &mut u64) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let n = file.take(*budget + 1).read_to_end(&mut out).map_err(damaged)? as u64;
    if n > *budget {
        return Err(KernError::TooLarge);
    }
    *budget -= n;
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Load a document from `.kern` bytes. Never panics; every problem is an error.
pub fn read_kern(bytes: &[u8]) -> Result<Document> {
    read_kern_with_id(bytes).map(|(doc, _)| doc)
}

/// Like `read_kern`, also returning the document id from the manifest.
pub fn read_kern_with_id(bytes: &[u8]) -> Result<(Document, String)> {
    if !is_kern(bytes) {
        return Err(KernError::NotKern);
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(damaged)?;
    if zip.len() > MAX_ENTRIES {
        return Err(KernError::TooLarge);
    }
    // First pass: look at every entry header without unpacking anything.
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut declared: u64 = 0;
    for i in 0..zip.len() {
        let file = zip.by_index_raw(i).map_err(damaged)?;
        let name = file.name().map_err(damaged)?.into_owned();
        if !safe_name(&name) {
            return Err(damaged(format!("unsafe entry name \"{}\"", name.escape_default())));
        }
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(damaged(format!("entry \"{name}\" appears twice")));
        }
        if file.encrypted() {
            return Err(damaged("an entry is encrypted"));
        }
        if !matches!(file.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            return Err(damaged("an entry uses an unsupported compression method"));
        }
        declared = declared.saturating_add(file.size());
        if declared > MAX_UNCOMPRESSED_BYTES {
            return Err(KernError::TooLarge);
        }
        index.insert(name, i);
    }
    // The limit that counts is on the bytes actually unpacked, whatever the headers say.
    let mut budget = MAX_UNCOMPRESSED_BYTES;
    let mut read = |name: &str, budget: &mut u64| -> Result<Option<Vec<u8>>> {
        match index.get(name) {
            Some(&i) => {
                let mut file = zip.by_index(i).map_err(damaged)?;
                read_entry(&mut file, budget).map(Some)
            }
            None => Ok(None),
        }
    };

    let manifest_bytes = read("manifest.json", &mut budget)?.ok_or_else(|| damaged("no manifest"))?;
    let manifest: ManifestIn = serde_json::from_slice(&manifest_bytes).map_err(damaged)?;
    if manifest.format != "KERN" {
        return Err(KernError::NotKern);
    }
    let (major, _) = parse_version(&manifest.format_version)
        .ok_or_else(|| damaged("the format version is not understood"))?;
    if major != MAJOR {
        return Err(if major > MAJOR {
            KernError::TooNew(manifest.format_version.clone())
        } else {
            KernError::Unsupported(format!("format version {}", manifest.format_version))
        });
    }
    if let Some(min) = &manifest.minimum_reader_version {
        let min = parse_version(min).ok_or_else(|| damaged("the minimum reader version is not understood"))?;
        if min > (MAJOR, MINOR) {
            return Err(KernError::TooNew(manifest.format_version.clone()));
        }
    }
    if let Some(f) = manifest.required_features.first() {
        return Err(KernError::Unsupported(f.clone()));
    }
    let doc_name = manifest.document.as_deref().unwrap_or(DOCUMENT_ENTRY);
    let json = read(doc_name, &mut budget)?.ok_or_else(|| damaged("no document"))?;

    let mut pictures: HashMap<String, Arc<Vec<u8>>> = HashMap::new();
    let image_names: Vec<String> = index.keys().filter(|n| n.starts_with(IMAGE_DIR)).cloned().collect();
    for name in image_names {
        let data = read(&name, &mut budget)?.unwrap_or_default();
        // The name is the picture's SHA-256; a different content is damage.
        let stem = name[IMAGE_DIR.len()..].split('.').next().unwrap_or("");
        if stem.len() == 64 && hex(&Sha256::digest(&data)) != stem.to_ascii_lowercase() {
            return Err(damaged(format!("picture \"{name}\" does not match its name")));
        }
        pictures.insert(name, Arc::new(data));
    }
    let guard = media::Guard::install(media::Mode::Read(pictures));
    let parsed = serde_json::from_slice::<Document>(&json);
    drop(guard);
    let mut doc = parsed.map_err(damaged)?;
    if doc.pages.is_empty() {
        return Err(damaged("the document has no pages"));
    }
    doc.renew_page_ids();
    doc.repair_next_id();
    doc.repair_masters();
    let id = if manifest.document_id.is_empty() { new_document_id() } else { manifest.document_id };
    Ok((doc, id))
}

/// Write `.kern` bytes to `path` safely: a temp file in the same folder, read back and checked
/// with `read_kern`, and only then renamed over the destination. On any failure the old file is
/// untouched. When `backup` is set and `path` exists it is first copied to `<path>.bak`.
pub fn write_file_checked(path: &Path, bytes: &[u8], backup: bool) -> std::result::Result<(), String> {
    let (mut f, tmp) = crate::save::create_unique_temp(path).map_err(|e| e.to_string())?;
    let result = (|| -> std::result::Result<(), String> {
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        let back = std::fs::read(&tmp).map_err(|e| e.to_string())?;
        read_kern(&back).map_err(|e| format!("the saved file did not check out: {e}"))?;
        if backup && path.exists() {
            let mut bak = path.file_name().unwrap_or_default().to_os_string();
            bak.push(".bak");
            std::fs::copy(path, path.with_file_name(bak)).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
