//! Downloading an update file: resumable, size-capped, and verified before it is trusted.
//!
//! What is requested, exactly: one `GET <file url>`, with a `Range: bytes=<n>-` header when a
//! partial download is being resumed. Only the fixed `User-Agent` is sent besides that: no
//! identifiers, no cookies. A download starts only when the user clicks Update / Update all /
//! Retry (see `Downloader::start`); nothing here runs on its own.
//!
//! The file goes to `<root>\<item id>-<version>\<name>.part` and is renamed to `<name>` only
//! after the size, SHA-256 and minisign signature all check out. The signature is made over
//! the whole file with a project key (the same key choice as the feed, `trusted_keys_for_url`).
//! A `<name>.verify.json` next to the finished file records size, SHA-256 and signature so the
//! update helper can verify the file again, from a locked handle, right before running it.

use crate::feed::{self, human_size, FileEntry};
use crate::net;
use minisign_verify::Signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CHUNK: usize = 64 * 1024;
const REPAINT_EVERY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    Unreachable(String),
    Http(u16),
    /// The release has no signature at all: releases must be signed.
    NoSignature,
    BadSignature,
    WrongKey,
    BadChecksum,
    TooLarge,
    Incomplete { got: u64, expected: u64 },
    NoSpace { need: u64, free: u64 },
    Disk(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Unreachable(d) => write!(f, "Could not download the update ({d}). Try again; the download resumes where it stopped."),
            DownloadError::Http(c) => write!(f, "The download server answered with an error (HTTP {c})."),
            DownloadError::NoSignature => write!(f, "This update has no signature, so it was not downloaded. Nothing was installed."),
            DownloadError::BadSignature => write!(f, "The downloaded update's signature could not be verified, so it was deleted. Nothing was installed."),
            DownloadError::WrongKey => write!(f, "The downloaded update was signed with a key this launcher does not trust, so it was deleted. Nothing was installed."),
            DownloadError::BadChecksum => write!(f, "The downloaded update did not match its checksum, so it was deleted. Nothing was installed."),
            DownloadError::TooLarge => write!(f, "The download is larger than the update list says, so it was stopped and deleted. Nothing was installed."),
            DownloadError::Incomplete { got, expected } => write!(f, "The download ended early ({} of {}). Try again; it resumes where it stopped.", human_size(*got), human_size(*expected)),
            DownloadError::NoSpace { need, free } => write!(f, "There is not enough free disk space for this update (it needs about {}, and {} is free). Free some space and try again.", human_size(*need), human_size(*free)),
            DownloadError::Disk(d) => write!(f, "Could not save the update ({d})."),
        }
    }
}

impl std::error::Error for DownloadError {}

/// What to download.
#[derive(Debug, Clone)]
pub struct Job {
    /// `id@version`, the key the UI looks the state up by.
    pub key: String,
    pub item_id: String,
    pub version: String,
    pub file: FileEntry,
    /// The feed URL, which decides the trusted public key.
    pub feed_url: String,
}

impl Job {
    pub fn key_for(item_id: &str, version: &str) -> String {
        format!("{item_id}@{version}")
    }
}

/// Bytes done and total, shared with the UI thread.
#[derive(Default)]
pub struct Progress {
    done: AtomicU64,
    total: AtomicU64,
}

impl Progress {
    pub fn get(&self) -> (u64, u64) {
        (self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Finish {
    Done(PathBuf),
    /// The user cancelled; the `.part` file is kept so a later Update resumes.
    Cancelled,
}

/// `%LOCALAPPDATA%\KernPage\updates` (elsewhere: the user data folder).
pub fn updates_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(p) = crate::testhooks::peek(|f| f.updates.clone()) {
        return p;
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("KernPage").join("updates")
}

fn sanitize(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+') { c } else { '_' }).collect();
    if t.is_empty() || t.chars().all(|c| c == '.') {
        "download".to_owned()
    } else {
        t
    }
}

fn file_name(url: &str) -> String {
    let rest = url.split(['?', '#']).next().unwrap_or("");
    sanitize(rest.rsplit('/').next().unwrap_or(""))
}

#[cfg(windows)]
fn free_space(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut avail = 0u64;
    // SAFETY: `wide` is NUL-terminated and outlives the call; the out pointers are valid or null.
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, std::ptr::null_mut(), std::ptr::null_mut())
    };
    (ok != 0).then_some(avail)
}

#[cfg(not(windows))]
fn free_space(_dir: &Path) -> Option<u64> {
    None
}

fn disk(e: std::io::Error) -> DownloadError {
    DownloadError::Disk(e.to_string())
}

/// Which of `keys` signed `sig` (the key id is checked first, so a mismatch is `WrongKey`).
fn stream_verifier<'a>(keys: &'a [minisign_verify::PublicKey], sig: &'a Signature) -> Result<minisign_verify::StreamVerifier<'a>, DownloadError> {
    let mut result = Err(DownloadError::WrongKey);
    for key in keys {
        match key.verify_stream(sig) {
            Ok(v) => return Ok(v),
            Err(minisign_verify::Error::UnexpectedKeyId) => {}
            Err(_) => result = Err(DownloadError::BadSignature),
        }
    }
    result
}

/// Size, SHA-256 and signature of an open file, read from its start, in one pass. The signature
/// may be from any of `keys`.
fn verify_reader(f: &mut std::fs::File, size: u64, sha256: &str, keys: &[minisign_verify::PublicKey], sig: &Signature) -> Result<(), DownloadError> {
    use std::io::Seek;
    let len = f.metadata().map_err(disk)?.len();
    if len != size {
        return Err(DownloadError::Incomplete { got: len, expected: size });
    }
    let mut verifier = stream_verifier(keys, sig)?;
    let mut sha = Sha256::new();
    f.seek(std::io::SeekFrom::Start(0)).map_err(disk)?;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = f.read(&mut buf).map_err(disk)?;
        if n == 0 {
            break;
        }
        sha.update(&buf[..n]);
        verifier.update(&buf[..n]);
    }
    if !crate::feed::hex(sha.finalize()).eq_ignore_ascii_case(sha256.trim()) {
        return Err(DownloadError::BadChecksum);
    }
    verifier.finalize().map_err(|_| DownloadError::BadSignature)
}

/// Size, SHA-256 and signature of a finished file, in one pass.
fn verify_file(path: &Path, file: &FileEntry, keys: &[minisign_verify::PublicKey], sig: &Signature) -> Result<(), DownloadError> {
    let mut f = std::fs::File::open(path).map_err(disk)?;
    verify_reader(&mut f, file.size, &file.sha256, keys, sig)
}

// ---- verifying again right before installing (security review M2) ------------------------------

/// What `run` records next to a verified download (`<name>.verify.json`) so the update helper
/// can check the file again. The sidecar is not itself trusted: the minisign signature over the
/// file is the trust anchor, and nobody without the signing key can make a file that passes.
#[derive(Debug, Serialize, Deserialize)]
struct Sidecar {
    size: u64,
    sha256: String,
    sig: String,
}

fn sidecar_path(file: &Path) -> PathBuf {
    let mut name = file.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".verify.json");
    file.with_file_name(name)
}

fn write_sidecar(file: &Path, entry: &FileEntry) -> Result<(), DownloadError> {
    let text = serde_json::to_string(&Sidecar { size: entry.size, sha256: entry.sha256.clone(), sig: entry.sig.clone() }).map_err(|e| DownloadError::Disk(e.to_string()))?;
    std::fs::write(sidecar_path(file), text).map_err(disk)
}

/// Why an installer file is not run.
pub fn refusal_text(e: &DownloadError) -> String {
    format!("The update file was not installed: {e}")
}

/// True when `path` is inside `root` once both are resolved (`..` and links followed).
pub fn is_inside(path: &Path, root: &Path) -> bool {
    match (std::fs::canonicalize(path), std::fs::canonicalize(root)) {
        (Ok(p), Ok(r)) => p.starts_with(r),
        _ => false,
    }
}

/// Open a downloaded installer for running and verify it, without a window for a swap between
/// the check and the use: the file must be inside `root` (the updates folder), and it is opened
/// so that nobody else can write to it, rename it or delete it (`FILE_SHARE_READ` only) until the
/// returned handle is dropped. Size, SHA-256 and signature (any of `keys`) are checked on THAT
/// handle. The caller keeps the handle open for as long as the installer runs.
pub fn open_verified_for_install(path: &Path, root: &Path, keys: &[minisign_verify::PublicKey]) -> Result<std::fs::File, DownloadError> {
    if !is_inside(path, root) {
        return Err(DownloadError::Disk("the file is not in the updates folder".into()));
    }
    let side: Sidecar = std::fs::read_to_string(sidecar_path(path))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or(DownloadError::NoSignature)?;
    let sig = Signature::decode(&side.sig).map_err(|_| DownloadError::BadSignature)?;
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(1); // FILE_SHARE_READ: no writers, no rename, no delete while open
    }
    let mut f = opts.open(path).map_err(disk)?;
    verify_reader(&mut f, side.size, &side.sha256, keys, &sig)?;
    Ok(f)
}

/// Run one download to completion (blocking; the caller supplies the thread).
pub fn run(job: &Job, root: &Path, progress: &Progress, cancel: &AtomicBool, repaint: &dyn Fn()) -> Result<Finish, DownloadError> {
    let file = &job.file;
    if file.sig.trim().is_empty() {
        return Err(DownloadError::NoSignature);
    }
    let key = feed::trusted_keys_for_url(&job.feed_url).map_err(|_| DownloadError::BadSignature)?;
    let sig = Signature::decode(&file.sig).map_err(|_| DownloadError::BadSignature)?;

    let dir = root.join(format!("{}-{}", sanitize(&job.item_id), sanitize(&job.version)));
    std::fs::create_dir_all(&dir).map_err(disk)?;
    let name = file_name(&file.url);
    let final_path = dir.join(&name);
    let part = dir.join(format!("{name}.part"));
    progress.total.store(file.size, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);

    // Already downloaded and still good: nothing to fetch.
    if final_path.is_file() {
        if verify_file(&final_path, file, &key, &sig).is_ok() && write_sidecar(&final_path, file).is_ok() {
            progress.done.store(file.size, Ordering::Relaxed);
            return Ok(Finish::Done(final_path));
        }
        let _ = std::fs::remove_file(&final_path);
    }

    let need = file.size.saturating_add(file.size / 10);
    if let Some(free) = free_space(&dir) {
        if free < need {
            return Err(DownloadError::NoSpace { need, free });
        }
    }

    let mut have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if have > file.size {
        let _ = std::fs::remove_file(&part);
        have = 0;
    }
    if have < file.size {
        let agent = net::download_agent(&file.url);
        let mut req = agent.get(&file.url);
        if have > 0 {
            req = req.header("Range", &format!("bytes={have}-"));
        }
        let resp = req.call().map_err(|e| DownloadError::Unreachable(net::short_error(&e)))?;
        let status = resp.status().as_u16();
        let header = |n: &str| resp.headers().get(n).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let append = match status {
            206 => {
                let start = header("content-range")
                    .and_then(|r| r.strip_prefix("bytes ").and_then(|r| r.split('-').next().and_then(|s| s.trim().parse::<u64>().ok())));
                if start != Some(have) {
                    return Err(DownloadError::Unreachable("the server sent the wrong part of the file".into()));
                }
                true
            }
            200 => false, // Range ignored: start again from zero.
            other => return Err(DownloadError::Http(other)),
        };
        if !append {
            have = 0;
        }
        if let Some(len) = header("content-length").and_then(|l| l.parse::<u64>().ok()) {
            if have + len > file.size {
                let _ = std::fs::remove_file(&part);
                return Err(DownloadError::TooLarge);
            }
        }
        let mut out = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(&part).map_err(disk)?;
        let mut body = resp.into_body().into_reader();
        let mut buf = vec![0u8; CHUNK];
        let mut done = have;
        progress.done.store(done, Ordering::Relaxed);
        let mut last_repaint: Option<Instant> = None;
        loop {
            if cancel.load(Ordering::SeqCst) {
                let _ = out.flush();
                return Ok(Finish::Cancelled);
            }
            let n = body.read(&mut buf).map_err(|e| DownloadError::Unreachable(e.to_string()))?;
            if n == 0 {
                break;
            }
            done += n as u64;
            if done > file.size {
                drop(out);
                let _ = std::fs::remove_file(&part);
                return Err(DownloadError::TooLarge);
            }
            out.write_all(&buf[..n]).map_err(disk)?;
            progress.done.store(done, Ordering::Relaxed);
            if last_repaint.is_none_or(|t| t.elapsed() >= REPAINT_EVERY) {
                last_repaint = Some(Instant::now());
                repaint();
            }
        }
        out.flush().map_err(disk)?;
        drop(out);
        if done != file.size {
            // Interrupted: keep the .part so the next try resumes.
            return Err(DownloadError::Incomplete { got: done, expected: file.size });
        }
    }

    if let Err(e) = verify_file(&part, file, &key, &sig) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    // Authenticode (WinVerifyTrust) check of the finished file goes here once releases are
    // signed with a code-signing certificate.
    let _ = std::fs::remove_file(&final_path);
    std::fs::rename(&part, &final_path).map_err(disk)?;
    write_sidecar(&final_path, file)?;
    progress.done.store(file.size, Ordering::Relaxed);
    Ok(Finish::Done(final_path))
}

// ---- the manager the window talks to -----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idle,
    Queued,
    Running { done: u64, total: u64 },
    Ready(PathBuf),
    Failed(String),
}

struct Active {
    key: String,
    progress: Arc<Progress>,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Result<Finish, DownloadError>>,
}

/// One download at a time; further jobs wait in order. Starts only through `start`.
pub struct Downloader {
    root: PathBuf,
    active: Option<Active>,
    queue: VecDeque<Job>,
    finished: HashMap<String, Result<PathBuf, String>>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Downloader {
    pub fn new(root: PathBuf) -> Self {
        Downloader { root, active: None, queue: VecDeque::new(), finished: HashMap::new(), repaint: None }
    }

    pub fn set_repaint(&mut self, f: Arc<dyn Fn() + Send + Sync>) {
        self.repaint = Some(f);
    }

    /// Start (or queue) a download. Also the Retry path: it clears an earlier failure.
    pub fn start(&mut self, job: Job) {
        if self.active.as_ref().is_some_and(|a| a.key == job.key) || self.queue.iter().any(|j| j.key == job.key) {
            return;
        }
        self.finished.remove(&job.key);
        if self.active.is_some() {
            self.queue.push_back(job);
        } else {
            self.spawn(job);
        }
    }

    fn spawn(&mut self, job: Job) {
        let progress = Arc::new(Progress::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let (p, c, root, repaint) = (progress.clone(), cancel.clone(), self.root.clone(), self.repaint.clone());
        let key = job.key.clone();
        std::thread::spawn(move || {
            let tick = || {
                if let Some(r) = &repaint {
                    r();
                }
            };
            let _ = tx.send(run(&job, &root, &p, &c, &tick));
            tick();
        });
        self.active = Some(Active { key, progress, cancel, rx });
    }

    /// Stop a running download (its `.part` stays for a later resume) or drop a queued one.
    pub fn cancel(&mut self, key: &str) {
        if let Some(a) = self.active.as_ref().filter(|a| a.key == key) {
            a.cancel.store(true, Ordering::SeqCst);
        }
        self.queue.retain(|j| j.key != key);
    }

    /// Collect a finished download and start the next queued one.
    pub fn poll(&mut self) {
        let Some(a) = &self.active else { return };
        let res = match a.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(DownloadError::Unreachable("the download stopped unexpectedly".into())),
        };
        let key = a.key.clone();
        self.active = None;
        match res {
            Ok(Finish::Done(p)) => {
                self.finished.insert(key, Ok(p));
            }
            Ok(Finish::Cancelled) => {}
            Err(e) => {
                self.finished.insert(key, Err(e.to_string()));
            }
        }
        if let Some(next) = self.queue.pop_front() {
            self.spawn(next);
        }
    }

    pub fn status(&self, key: &str) -> Status {
        if let Some(a) = self.active.as_ref().filter(|a| a.key == key) {
            let (done, total) = a.progress.get();
            return Status::Running { done, total };
        }
        if self.queue.iter().any(|j| j.key == key) {
            return Status::Queued;
        }
        match self.finished.get(key) {
            Some(Ok(p)) => Status::Ready(p.clone()),
            Some(Err(m)) => Status::Failed(m.clone()),
            None => Status::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devfeed::*;
    use crate::devserver::*;
    use std::sync::Mutex;

    fn body_bytes(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i.wrapping_mul(31) ^ (i >> 8)) as u8).collect()
    }

    fn entry(server: &Server, body: &[u8], sig: String) -> FileEntry {
        FileEntry {
            os: "any".into(),
            arch: "any".into(),
            kind: "msi".into(),
            url: server.url("/files/update.msi"),
            size: body.len() as u64,
            sha256: crate::feed::hex(Sha256::digest(body)),
            sig,
        }
    }

    fn job(server: &Server, file: FileEntry) -> Job {
        Job { key: "kernpage@0.2.0".into(), item_id: "kernpage".into(), version: "0.2.0".into(), file, feed_url: server.url("/index.json") }
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pw-dl-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    type Seen = Arc<Mutex<Vec<Vec<(String, String)>>>>;

    /// Serves `body` at /files/update.msi, honouring Range when `ranges`; records request headers.
    fn serve(body: Vec<u8>, ranges: bool) -> (Server, Seen) {
        let seen: Seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        let h: Handler = Arc::new(move |req: &Request| {
            s.lock().unwrap().push(req.headers.clone());
            if req.path != "/files/update.msi" {
                return Response::status(404);
            }
            if let (true, Some(r)) = (ranges, req.header("Range")) {
                let start: usize = r.trim_start_matches("bytes=").trim_end_matches('-').parse().unwrap();
                let mut resp = Response::status(206);
                resp.body = body[start..].to_vec();
                resp.headers.push(("Content-Range".into(), format!("bytes {start}-{}/{}", body.len() - 1, body.len())));
                return resp;
            }
            Response::ok(body.clone())
        });
        (Server::start(0, h).unwrap(), seen)
    }

    fn go(job: &Job, root: &Path) -> Result<Finish, DownloadError> {
        run(job, root, &Progress::default(), &AtomicBool::new(false), &|| {})
    }

    fn final_and_part(root: &Path) -> (PathBuf, PathBuf) {
        let d = root.join("kernpage-0.2.0");
        (d.join("update.msi"), d.join("update.msi.part"))
    }

    #[test]
    fn full_download_is_verified_and_renamed() {
        let body = body_bytes(300_000);
        let (server, seen) = serve(body.clone(), true);
        let root = temp("full");
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        let (fin, part) = final_and_part(&root);
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(std::fs::read(&fin).unwrap(), body);
        assert!(!part.exists());
        // Privacy: a fixed User-Agent and nothing identifying.
        let headers = seen.lock().unwrap()[0].clone();
        assert!(headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("user-agent") && v.starts_with("KernPageLauncher/")));
        for (k, _) in &headers {
            let k = k.to_ascii_lowercase();
            assert!(!matches!(k.as_str(), "cookie" | "authorization" | "referer" | "x-forwarded-for"), "unexpected header {k}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_fake_feeds_real_file_release_passes_normal_verification() {
        let body = body_bytes(250_000);
        let mut o = SampleOpts::new(1, "http://127.0.0.1:1");
        o.app_real = Some(RealFile { version: "0.4.0".into(), bytes: body.clone(), setup: None });
        let older = body_bytes(1_000);
        o.app_older = Some(RealFile { version: "0.1.0".into(), bytes: older.clone(), setup: None });
        let files: Files = Arc::new(Mutex::new(std::collections::HashMap::new()));
        for (name, bytes) in served_files(&o) {
            files.lock().unwrap().insert(format!("/files/{name}"), bytes);
        }
        let server = Server::start(0, static_handler(files)).unwrap();
        o.base_url = format!("http://127.0.0.1:{}", server.port);

        let index = sample_bytes(&o);
        let key = vec![feed::parse_public_key(TEST_PUBLIC).unwrap()];
        let f = feed::parse_verified(&index, &sign_test(&index), &key, 1_800_000_000, 0).unwrap();
        let app = f.item("kernpage").unwrap();
        let root = temp("realfeed");
        for (version, bytes, resume) in [("0.4.0", &body, true), ("0.1.0", &older, false)] {
            let release = f.release_for(app, &semver::Version::parse(version).unwrap(), false).expect("offered");
            let file = release.file_for_this_platform().unwrap().clone();
            assert_eq!(file.size, bytes.len() as u64);
            assert_eq!(file.sha256, crate::feed::hex(Sha256::digest(bytes)));
            let j = Job { key: format!("kernpage@{version}"), item_id: "kernpage".into(), version: version.into(), file, feed_url: server.url("/index.json") };
            if resume {
                // Also exercises Range support in the fake server: a partial file is completed.
                let dir = root.join(format!("kernpage-{version}"));
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join(format!("kernpage-{version}-x64.msi.part")), &bytes[..100_000]).unwrap();
            }
            let Ok(Finish::Done(path)) = go(&j, &root) else { panic!("download of {version} failed") };
            assert_eq!(&std::fs::read(path).unwrap(), bytes);
        }
        // A damaged copy is still refused by the same checks.
        let mut bad = f.release_for(app, &semver::Version::new(0, 1, 0), false).unwrap().file_for_this_platform().unwrap().clone();
        bad.sha256 = crate::feed::hex(Sha256::digest(b"something else"));
        let j = Job { key: "kernpage@0.1.0".into(), item_id: "kernpage".into(), version: "0.1.0".into(), file: bad, feed_url: server.url("/index.json") };
        assert!(go(&j, &temp("realfeed-bad")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resumes_a_partial_download_with_a_range_request() {
        let body = body_bytes(300_000);
        let (server, seen) = serve(body.clone(), true);
        let root = temp("resume");
        let (fin, part) = final_and_part(&root);
        std::fs::create_dir_all(part.parent().unwrap()).unwrap();
        std::fs::write(&part, &body[..100_000]).unwrap();
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(std::fs::read(&fin).unwrap(), body);
        let headers = seen.lock().unwrap()[0].clone();
        assert!(headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("range") && v == "bytes=100000-"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_server_that_ignores_range_restarts_from_zero() {
        let body = body_bytes(300_000);
        let (server, _) = serve(body.clone(), false);
        let root = temp("norange");
        let (fin, part) = final_and_part(&root);
        std::fs::create_dir_all(part.parent().unwrap()).unwrap();
        std::fs::write(&part, vec![7u8; 100_000]).unwrap(); // junk prefix must not survive
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(std::fs::read(&fin).unwrap(), body);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cancel_keeps_the_part_file() {
        let body = body_bytes(1_000_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("cancel");
        let (fin, part) = final_and_part(&root);
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        let progress = Progress::default();
        let cancel = AtomicBool::new(false);
        // The first progress tick (after the first chunk) cancels.
        let r = run(&j, &root, &progress, &cancel, &|| cancel.store(true, Ordering::SeqCst));
        assert_eq!(r, Ok(Finish::Cancelled));
        let kept = std::fs::metadata(&part).unwrap().len();
        assert!(kept > 0 && kept < body.len() as u64);
        assert!(!fin.exists());
        // A later run resumes and finishes.
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(std::fs::read(&fin).unwrap(), body);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checksum_mismatch_is_refused_and_the_part_deleted() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("sha");
        let (fin, part) = final_and_part(&root);
        let mut f = entry(&server, &body, sign_test(&body));
        f.sha256 = crate::feed::hex(Sha256::digest(b"something else"));
        let r = go(&job(&server, f), &root);
        assert_eq!(r, Err(DownloadError::BadChecksum));
        assert!(r.unwrap_err().to_string().contains("did not match its checksum"));
        assert!(!part.exists() && !fin.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_larger_than_declared_is_refused() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("big");
        let (fin, part) = final_and_part(&root);
        let mut f = entry(&server, &body, sign_test(&body));
        f.size -= 10;
        assert_eq!(go(&job(&server, f), &root), Err(DownloadError::TooLarge));
        assert!(!part.exists() && !fin.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_short_download_is_not_accepted_and_keeps_its_part() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("short");
        let (fin, part) = final_and_part(&root);
        let mut f = entry(&server, &body, sign_test(&body));
        f.size += 50;
        assert!(matches!(go(&job(&server, f), &root), Err(DownloadError::Incomplete { .. })));
        assert!(part.exists() && !fin.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wrong_key_and_missing_signatures_are_refused() {
        let body = body_bytes(100_000);
        let (server, seen) = serve(body.clone(), true);
        let root = temp("sig");
        let (fin, part) = final_and_part(&root);
        let r = go(&job(&server, entry(&server, &body, sign_with(OTHER_SECRET, &body))), &root);
        assert_eq!(r, Err(DownloadError::WrongKey));
        assert!(!part.exists() && !fin.exists());
        // A signature over different bytes (right key) is a bad signature.
        let r = go(&job(&server, entry(&server, &body, sign_test(b"other"))), &root);
        assert_eq!(r, Err(DownloadError::BadSignature));
        assert!(!part.exists() && !fin.exists());
        // No signature: refused before any request.
        let before = seen.lock().unwrap().len();
        let r = go(&job(&server, entry(&server, &body, String::new())), &root);
        assert_eq!(r, Err(DownloadError::NoSignature));
        assert_eq!(seen.lock().unwrap().len(), before);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_already_complete_file_is_not_downloaded_again() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("again");
        let (fin, _) = final_and_part(&root);
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(server.hits(), 1);
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(server.hits(), 1, "a verified file must not be fetched again");
        // A damaged copy is replaced.
        std::fs::write(&fin, vec![0u8; body.len()]).unwrap();
        assert_eq!(go(&j, &root), Ok(Finish::Done(fin.clone())));
        assert_eq!(server.hits(), 2);
        assert_eq!(std::fs::read(&fin).unwrap(), body);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_is_requested_unless_a_download_is_started() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("manual");
        let mut d = Downloader::new(root.clone());
        for _ in 0..50 {
            d.poll();
        }
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(server.hits(), 0);
        assert_eq!(d.status("kernpage@0.2.0"), Status::Idle);
        let j = job(&server, entry(&server, &body, sign_test(&body)));
        d.start(j.clone());
        let mut st = Status::Idle;
        for _ in 0..400 {
            d.poll();
            st = d.status(&j.key);
            if matches!(st, Status::Ready(_) | Status::Failed(_)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(matches!(st, Status::Ready(_)), "{st:?}");
        assert_eq!(server.hits(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_manager_reports_a_failure_in_plain_english_and_runs_jobs_one_at_a_time() {
        let body = body_bytes(100_000);
        let (server, _) = serve(body.clone(), true);
        let root = temp("mgr");
        let mut d = Downloader::new(root.clone());
        let mut bad = job(&server, entry(&server, &body, sign_test(&body)));
        bad.file.sha256 = "0".repeat(64);
        let mut second = job(&server, entry(&server, &body, sign_test(&body)));
        second.key = "dict-en-us@1".into();
        second.item_id = "dict-en-us".into();
        second.version = "1".into();
        d.start(bad.clone());
        d.start(second.clone());
        assert_eq!(d.status(&second.key), Status::Queued);
        for _ in 0..400 {
            d.poll();
            if matches!(d.status(&second.key), Status::Ready(_)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        match d.status(&bad.key) {
            Status::Failed(m) => assert!(m.contains("checksum")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(d.status(&second.key), Status::Ready(_)));
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- verifying again right before install (M2) ----

    fn downloaded(tag: &str, body: &[u8]) -> (PathBuf, PathBuf, Vec<minisign_verify::PublicKey>) {
        let (server, _) = serve(body.to_vec(), false);
        let root = temp(tag);
        let j = job(&server, entry(&server, body, sign_test(body)));
        let Ok(Finish::Done(path)) = go(&j, &root) else { panic!("download failed") };
        (root, path, vec![feed::parse_public_key(TEST_PUBLIC).unwrap()])
    }

    #[test]
    fn a_verified_download_is_rechecked_and_locked_for_install() {
        let body = body_bytes(50_000);
        let (root, path, keys) = downloaded("rc1", &body);
        assert!(sidecar_path(&path).is_file(), "the download leaves its verification record");
        let held = open_verified_for_install(&path, &root, &keys).expect("verifies");
        #[cfg(windows)]
        {
            // While the handle is held nobody can overwrite, delete or rename the file.
            assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err(), "no writers");
            assert!(std::fs::remove_file(&path).is_err(), "no delete");
            assert!(std::fs::rename(&path, path.with_extension("x")).is_err(), "no rename");
            // Reading is fine, which is what msiexec / Setup.exe need.
            assert_eq!(std::fs::read(&path).unwrap(), body);
        }
        drop(held);
        assert!(std::fs::remove_file(&path).is_ok(), "released afterwards");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_swapped_after_download_is_refused_at_install_time() {
        let body = body_bytes(50_000);
        let (root, path, keys) = downloaded("rc2", &body);
        let mut evil = body.clone();
        evil[10] ^= 0xFF; // same size, different bytes
        std::fs::write(&path, &evil).unwrap();
        assert_eq!(open_verified_for_install(&path, &root, &keys).unwrap_err(), DownloadError::BadChecksum);
        // A grown or shortened file fails the size check.
        std::fs::write(&path, &body[..100]).unwrap();
        assert!(matches!(open_verified_for_install(&path, &root, &keys).unwrap_err(), DownloadError::Incomplete { .. }));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_swapped_file_with_a_forged_record_still_needs_a_valid_signature() {
        let body = body_bytes(50_000);
        let (root, path, keys) = downloaded("rc3", &body);
        let evil = b"an attacker's installer".to_vec();
        std::fs::write(&path, &evil).unwrap();
        // The attacker rewrites the record to match their file; the signature cannot match.
        let side = Sidecar { size: evil.len() as u64, sha256: crate::feed::hex(Sha256::digest(&evil)), sig: sign_with(OTHER_SECRET, &evil) };
        std::fs::write(sidecar_path(&path), serde_json::to_string(&side).unwrap()).unwrap();
        assert_eq!(open_verified_for_install(&path, &root, &keys).unwrap_err(), DownloadError::WrongKey);
        let side = Sidecar { sig: sign_test(&body), ..side };
        std::fs::write(sidecar_path(&path), serde_json::to_string(&side).unwrap()).unwrap();
        assert_eq!(open_verified_for_install(&path, &root, &keys).unwrap_err(), DownloadError::BadSignature);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn install_refuses_files_outside_the_updates_folder_or_without_a_record() {
        let body = body_bytes(5_000);
        let (root, path, keys) = downloaded("rc4", &body);
        // Same bytes, record and signature, but copied outside the updates folder.
        let elsewhere = temp("recheck-elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let copy = elsewhere.join("update.msi");
        std::fs::copy(&path, &copy).unwrap();
        std::fs::copy(sidecar_path(&path), sidecar_path(&copy)).unwrap();
        assert!(open_verified_for_install(&copy, &root, &keys).is_err(), "outside the updates folder");
        // `..` tricks resolve to the real place.
        let sneaky = root.join("kernpage-0.2.0").join("..").join("..").join(elsewhere.file_name().unwrap()).join("update.msi");
        assert!(open_verified_for_install(&sneaky, &root, &keys).is_err());
        // No record: refused even though the file is fine.
        std::fs::remove_file(sidecar_path(&path)).unwrap();
        assert_eq!(open_verified_for_install(&path, &root, &keys).unwrap_err(), DownloadError::NoSignature);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn install_accepts_a_signature_from_any_trusted_key() {
        let body = body_bytes(5_000);
        let (root, path, test_keys) = downloaded("rc5", &body);
        let placeholder = feed::parse_public_key(feed::PRODUCTION_PUBLIC_KEYS[0]).unwrap();
        let both = vec![placeholder.clone(), test_keys[0].clone()];
        assert!(open_verified_for_install(&path, &root, &both).is_ok());
        assert_eq!(open_verified_for_install(&path, &root, &[placeholder]).unwrap_err(), DownloadError::WrongKey);
        let _ = std::fs::remove_dir_all(&root);
    }
}
