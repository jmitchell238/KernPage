//! AutoRecover and crash recovery (docs/ui/AUTORECOVER.md).
//!
//! Everything here is plain logic with injectable folders and an injectable "is this
//! process alive" function, so it is unit-testable without a GUI:
//!
//! * [`Settings`]: the "Save AutoRecover information every N minutes" option and the input
//!   rules of its minutes box.
//! * Snapshot format: a versioned JSON serialization of the [`Document`] plus metadata
//!   (original path, display name, time, process id). It is not a `.pub`.
//! * [`Writer`]: one background thread that writes snapshots atomically and deletes them,
//!   in order.
//! * [`scan`]: which snapshots a starting editor should recover.
//! * [`needs_snapshot`]: the dirty-tracking decision.
//! * A panic-time synchronous snapshot ([`set_panic_state`], [`install_panic_hook`]).

use crate::model::Document;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

/// Version of the snapshot format written. Version 2 adds the linked stories (the whole
/// text of every chain, overflow included); version 1 snapshots still load, their linked
/// frames as independent text boxes. Any other version is not loaded.
pub const FORMAT_VERSION: u32 = 2;
const MAGIC: &str = "kernpage-autorecover";
/// File extension of snapshots (not `.pub`).
pub const EXTENSION: &str = "kprec";
/// Extension of snapshots written by earlier versions; still found, read and cleaned up.
pub const LEGACY_EXTENSION: &str = "pwrec";

/// Is `path` a snapshot file (current or legacy extension)?
pub fn is_snapshot_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION) || e.eq_ignore_ascii_case(LEGACY_EXTENSION))
}
/// Folder (inside the AutoRecover folder) that receives snapshots that cannot be read.
pub const UNREADABLE_DIR: &str = "unreadable";

// ---- settings ----------------------------------------------------------------------

pub const DEFAULT_MINUTES: u32 = 10;
pub const MIN_MINUTES: u32 = 1;
pub const MAX_MINUTES: u32 = 120;

/// The Options > Save settings that KernPage supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// "Save AutoRecover information every N minutes" is checked.
    pub enabled: bool,
    pub minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: true,
            minutes: DEFAULT_MINUTES,
        }
    }
}

impl Settings {
    /// Parse the settings file (`Key=value` lines, same key names as Publisher's registry
    /// values). Unknown lines and bad values fall back to the defaults.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "AutoRecoverSave" => match v {
                    "0" => s.enabled = false,
                    "1" => s.enabled = true,
                    _ => {}
                },
                "AutoRecoverSaveInterval" => {
                    if let Some(m) = commit_minutes(v, DEFAULT_MINUTES) {
                        s.minutes = m;
                    }
                }
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        format!(
            "AutoRecoverSave={}\nAutoRecoverSaveInterval={}\n",
            u8::from(self.enabled),
            self.minutes
        )
    }

    pub fn load_from(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .map(|t| Settings::parse(&t))
            .unwrap_or_default()
    }

    /// The saved settings, or the defaults when there is no (readable) file.
    pub fn load() -> Settings {
        settings_path().map_or_else(Settings::default, |p| Settings::load_from(&p))
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::write_atomic(path, self.to_text().as_bytes(), false)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    pub fn save(&self) -> std::io::Result<()> {
        match settings_path() {
            Some(p) => self.save_to(&p),
            None => Ok(()),
        }
    }

    /// The tick interval: the minutes setting, unless `KERNPAGE_AUTORECOVER_SECONDS` (a
    /// test hook for crash tests) overrides it in seconds.
    pub fn interval(&self) -> Duration {
        if let Some(s) = std::env::var("KERNPAGE_AUTORECOVER_SECONDS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|&s| s > 0)
        {
            return Duration::from_secs(s);
        }
        Duration::from_secs(u64::from(self.minutes) * 60)
    }
}

/// The settings file: `KERNPAGE_SETTINGS_FILE` when set, else `settings.txt` in the
/// "KernPage" config folder (next to `recent.txt`).
pub fn settings_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("KERNPAGE_SETTINGS_FILE") {
        return Some(PathBuf::from(p));
    }
    Some(crate::recent::config_dir()?.join("settings.txt"))
}

/// The AutoRecover folder: `KERNPAGE_AUTORECOVER_DIR` when set, else `AutoRecover` in the
/// config folder.
pub fn autorecover_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("KERNPAGE_AUTORECOVER_DIR") {
        return Some(PathBuf::from(p));
    }
    Some(crate::recent::config_dir()?.join("AutoRecover"))
}

/// The minutes box while typing: `proposed` is the text after a keystroke, `current` the
/// text before it. A keystroke that would make the number invalid (not digits, more than
/// 120) is silently refused. "0" and an empty box are allowed while editing.
pub fn accept_minutes_text(current: &str, proposed: &str) -> String {
    let ok = proposed.len() <= 3
        && proposed.chars().all(|c| c.is_ascii_digit())
        && proposed.parse::<u32>().map_or(true, |v| v <= MAX_MINUTES);
    if ok {
        proposed.to_owned()
    } else {
        current.to_owned()
    }
}

/// The value OK keeps: the box's number when it is 1 to 120, else `None` (the old value
/// stays; 0 and an empty box are not kept).
pub fn commit_minutes(text: &str, _old: u32) -> Option<u32> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|v| (MIN_MINUTES..=MAX_MINUTES).contains(v))
}

/// Spin arrows: one step up or down, clamped to 1..=120. A box that is empty or 0 starts
/// from the old value.
pub fn step_minutes(text: &str, old: u32, up: bool) -> u32 {
    let cur = commit_minutes(text, old).unwrap_or(old);
    if up {
        (cur + 1).min(MAX_MINUTES)
    } else {
        cur.saturating_sub(1).max(MIN_MINUTES)
    }
}

// ---- snapshot format -----------------------------------------------------------------

/// What a snapshot says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// The file the publication was opened from or saved to, if any.
    pub original_path: Option<String>,
    /// The name shown in the title bar.
    pub display_name: String,
    /// When the snapshot was taken, seconds since the Unix epoch.
    pub saved_at: u64,
    /// The editor process that wrote it.
    pub pid: u32,
}

#[derive(Serialize)]
struct EnvelopeOut<'a> {
    magic: &'a str,
    version: u32,
    meta: &'a Meta,
    document: &'a Document,
}

#[derive(Deserialize)]
struct Header {
    magic: String,
    version: u32,
}

#[derive(Deserialize)]
struct EnvelopeIn {
    meta: Meta,
    document: Document,
}

/// A loaded snapshot.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub meta: Meta,
    pub document: Document,
}

#[derive(Debug)]
pub enum SnapshotError {
    /// Not a snapshot at all.
    NotASnapshot,
    /// A snapshot of another format version.
    Version(u32),
    Parse(String),
    Io(std::io::Error),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::NotASnapshot => write!(f, "not an AutoRecover snapshot"),
            SnapshotError::Version(v) => write!(f, "snapshot format version {v} is not supported"),
            SnapshotError::Parse(e) => write!(f, "the snapshot is damaged ({e})"),
            SnapshotError::Io(e) => write!(f, "{e}"),
        }
    }
}

/// Serialize a document and its metadata (done on the UI thread; the write is separate).
pub fn encode(meta: &Meta, doc: &Document) -> Result<Vec<u8>, SnapshotError> {
    serde_json::to_vec(&EnvelopeOut {
        magic: MAGIC,
        version: FORMAT_VERSION,
        meta,
        document: doc,
    })
    .map_err(|e| SnapshotError::Parse(e.to_string()))
}

/// Load a snapshot; a wrong magic or version is rejected before the document is parsed.
pub fn decode(bytes: &[u8]) -> Result<Snapshot, SnapshotError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|_| SnapshotError::NotASnapshot)?;
    if header.magic != MAGIC {
        return Err(SnapshotError::NotASnapshot);
    }
    if !(1..=FORMAT_VERSION).contains(&header.version) {
        return Err(SnapshotError::Version(header.version));
    }
    let env: EnvelopeIn =
        serde_json::from_slice(bytes).map_err(|e| SnapshotError::Parse(e.to_string()))?;
    let mut document = env.document;
    document.renew_page_ids();
    document.repair_masters();
    Ok(Snapshot {
        meta: env.meta,
        document,
    })
}

pub fn read_snapshot(path: &Path) -> Result<Snapshot, SnapshotError> {
    decode(&std::fs::read(path).map_err(SnapshotError::Io)?)
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ---- file names --------------------------------------------------------------------

/// `AutoRecover save - <name without extension>(<4 hex>).kprec`; the hex field is four
/// characters wide, space-padded on the left when short (as Publisher's).
pub fn snapshot_file_name(display_name: &str, code: u32) -> String {
    let stem = display_name
        .rsplit_once('.')
        .filter(|(_, ext)| ext.eq_ignore_ascii_case("pub"))
        .map_or(display_name, |(s, _)| s);
    let clean: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(80)
        .collect();
    let clean = clean.trim();
    let clean = if clean.is_empty() { "Untitled" } else { clean };
    format!("AutoRecover save - {clean}({:>4X}).{EXTENSION}", code & 0xFFFF)
}

/// A four-hex-digit code that differs per process and per document opened.
pub fn new_code(pid: u32, counter: u32) -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let h = pid
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(counter.wrapping_mul(0x85EB_CA6B))
        ^ nanos;
    (h ^ (h >> 16)) & 0xFFFF
}

// ---- dirty tracking ----------------------------------------------------------------

/// What identifies the content of a document for dirty-tracking: the editor revision, and
/// the text being typed that is not yet stored in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentKey {
    pub revision: u64,
    pub pending_text: Option<String>,
}

/// Whether a tick should write a snapshot: the document has unsaved changes and its
/// content differs from what the last snapshot (or the recovery that loaded it) holds.
pub fn needs_snapshot(dirty: bool, key: &ContentKey, last: Option<&ContentKey>) -> bool {
    dirty && last != Some(key)
}

// ---- background writer ---------------------------------------------------------------

enum Job {
    Write(PathBuf, Vec<u8>),
    Delete(PathBuf),
}

/// One background thread that writes snapshots atomically (temp file + rename) and deletes
/// them, in the order they were queued, so a delete is never overtaken by an older write.
pub struct Writer {
    tx: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    pub fn spawn() -> Writer {
        let (tx, rx) = channel::<Job>();
        let thread = std::thread::Builder::new()
            .name("autorecover-writer".into())
            .spawn(move || {
                for job in rx {
                    match job {
                        Job::Write(path, bytes) => {
                            if let Some(dir) = path.parent() {
                                let _ = std::fs::create_dir_all(dir);
                            }
                            let _ = crate::write_atomic(&path, &bytes, false);
                        }
                        Job::Delete(path) => {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            })
            .ok();
        Writer {
            tx: Some(tx),
            thread,
        }
    }

    pub fn write(&self, path: PathBuf, bytes: Vec<u8>) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Job::Write(path, bytes));
        }
    }

    pub fn delete(&self, path: PathBuf) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Job::Delete(path));
        }
    }

    /// Finish every queued job and stop the thread.
    pub fn finish(&mut self) {
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.finish();
    }
}

// ---- recovery on start ---------------------------------------------------------------

/// A snapshot that can be recovered.
#[derive(Debug)]
pub struct Found {
    pub path: PathBuf,
    pub snapshot: Snapshot,
}

/// What a scan of the AutoRecover folder found.
#[derive(Debug, Default)]
pub struct Scan {
    /// Snapshots whose process is no longer running, newest first.
    pub recoverable: Vec<Found>,
    /// Snapshots that could not be read, now moved into the `unreadable` subfolder.
    pub moved_aside: Vec<PathBuf>,
}

/// Look in `dir` for snapshots to recover. Snapshots of a process that `is_alive` (another
/// running editor's, or this one's) are left alone. Unreadable or incompatible ones are
/// moved to `dir/unreadable` (never deleted). Nothing is deleted here.
pub fn scan(dir: &Path, is_alive: &dyn Fn(u32) -> bool) -> Scan {
    let mut out = Scan::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file() && is_snapshot_path(p)
        })
        .collect();
    paths.sort();
    for path in paths {
        match read_snapshot(&path) {
            Ok(snapshot) => {
                if !is_alive(snapshot.meta.pid) {
                    out.recoverable.push(Found { path, snapshot });
                }
            }
            Err(_) => {
                if let Some(to) = move_aside(dir, &path) {
                    out.moved_aside.push(to);
                }
            }
        }
    }
    out.recoverable
        .sort_by_key(|f| std::cmp::Reverse(f.snapshot.meta.saved_at));
    out
}

fn move_aside(dir: &Path, path: &Path) -> Option<PathBuf> {
    let side = dir.join(UNREADABLE_DIR);
    std::fs::create_dir_all(&side).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let mut to = side.join(&name);
    let mut n = 1;
    while to.exists() {
        to = side.join(format!("{name}.{n}"));
        n += 1;
    }
    std::fs::rename(path, &to).ok().map(|_| to)
}

/// Single-document recovery: take the most recent recoverable snapshot, delete its file at
/// once (as Publisher does), and leave the others for the next start.
pub fn take_most_recent(dir: &Path, is_alive: &dyn Fn(u32) -> bool) -> Option<Snapshot> {
    let mut scan = scan(dir, is_alive);
    if scan.recoverable.is_empty() {
        return None;
    }
    let found = scan.recoverable.remove(0);
    let _ = std::fs::remove_file(&found.path);
    Some(found.snapshot)
}

// ---- panic safety ----------------------------------------------------------------------

/// The state the panic hook needs: a copy of the document as of its last change.
pub struct PanicState {
    pub file: PathBuf,
    pub meta: Meta,
    pub document: Document,
}

static PANIC_STATE: Mutex<Option<PanicState>> = Mutex::new(None);

/// Record (or, with `None`, clear) the dirty document the panic hook should write.
pub fn set_panic_state(state: Option<PanicState>) {
    if let Ok(mut g) = PANIC_STATE.lock() {
        *g = state;
    }
}

/// Write the recorded document synchronously. Best effort: never panics, gives up when
/// the state is locked. Returns whether a file was written.
pub fn write_panic_snapshot() -> bool {
    std::panic::catch_unwind(|| {
        let Ok(mut g) = PANIC_STATE.try_lock() else {
            return false;
        };
        let Some(state) = g.take() else {
            return false;
        };
        let mut meta = state.meta;
        meta.saved_at = now_unix();
        let Ok(bytes) = encode(&meta, &state.document) else {
            return false;
        };
        if let Some(dir) = state.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        crate::write_atomic(&state.file, &bytes, false).is_ok()
    })
    .unwrap_or(false)
}

/// Install a panic hook that writes a final snapshot, then runs the previous hook.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = write_panic_snapshot();
        prev(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::model::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "pw-ar-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn meta(pid: u32, at: u64) -> Meta {
        Meta {
            original_path: Some("C:\\x\\a.pub".into()),
            display_name: "a.pub".into(),
            saved_at: at,
            pid,
        }
    }

    fn rich_doc() -> Document {
        let mut run = Run::plain("Hello ");
        run.bold = true;
        run.size_pt = 17.5;
        run.color = Rgb(1, 2, 3);
        let mut run2 = Run::plain("world");
        run2.italic = true;
        run2.underline = true;
        run2.font = "Arial".into();
        let para = Paragraph {
            runs: vec![run, run2],
            align: Align::Justify,
            spacing: ParaSpacing {
                space_before_pt: 3.0,
                space_after_pt: 0.5,
                line: LineSpacing::Exactly(14.0),
            },
            list: Some(ListInfo::numbered(ListType::LowerRoman)),
            indents: ParaIndents::for_list(18.0),
        };
        let mut text = Shape::text_box(Rect::new(10, 20, 3_000_000, 1_000_000), vec![para]);
        text.columns = 2;
        text.column_gap_emu = 5000;
        text.wrap = WrapType::Square;
        text.rotation_deg = 12.5;
        text.fill = Some(Rgb(9, 8, 7));
        text.line = Some(Stroke {
            color: Rgb(4, 5, 6),
            width_emu: 12700,
        });
        let mut rect = Shape::new(Rect::new(1, 2, 3, 4), ShapeKind::Rectangle);
        rect.fill = Some(Rgb::WHITE);
        let ell = Shape::new(Rect::new(5, 6, 7, 8), ShapeKind::Ellipse);
        let mut line = Shape::new(Rect::new(5, 6, 700, 800), ShapeKind::Line);
        line.flip_h = true;
        let pic_bytes: Vec<u8> = (0..=255u8).chain([0, 255, 128]).collect();
        let pic = Shape::new(
            Rect::new(50, 60, 700, 800),
            ShapeKind::Picture {
                data: std::sync::Arc::new(pic_bytes),
                format: PictureFormat::Png,
                file: Some(PictureFile {
                    name: "p.png".into(),
                    modified: Some(133_000_000_000_000_000),
                }),
            },
        );
        let table = Shape::new(
            Rect::new(0, 0, 4_000_000, 2_000_000),
            ShapeKind::Table {
                rows: 1,
                cols: 2,
                col_widths_emu: vec![1_000_000, 3_000_000],
                row_heights_emu: vec![2_000_000],
                row_set_emu: vec![],
                cells: vec![TableCell::plain("one"), TableCell::plain("two")],
            },
        );
        let mut doc = Document::blank("a.pub", 7_772_400, 10_058_400);
        let mut page2 = Page::blank(10_058_400, 7_772_400);
        page2.margins_emu = Margins {
            left: 1,
            right: 2,
            top: 3,
            bottom: 4,
        };
        doc.pages.push(page2);
        doc.default_line = Rgb(10, 20, 30);
        let mut ids = Vec::new();
        for s in [text, rect, ell, line, pic, table] {
            let id = doc.alloc_id();
            let mut s = s;
            s.id = id;
            ids.push(id);
            doc.pages[0].shapes.push(s);
        }
        let mut g = Shape::new(Rect::new(0, 0, 100, 100), ShapeKind::Group);
        g.id = doc.alloc_id();
        let gid = g.id;
        doc.pages[0].shapes[1].group = Some(gid);
        doc.pages[0].shapes[2].group = Some(gid);
        doc.pages[0].shapes.push(g);
        doc
    }

    #[test]
    fn round_trips_every_kind_of_content() {
        let doc = rich_doc();
        let bytes = encode(&meta(42, 1000), &doc).unwrap();
        let snap = decode(&bytes).unwrap();
        assert_eq!(snap.document, doc);
        assert_eq!(snap.meta, meta(42, 1000));
        assert_eq!(snap.document.default_line, Rgb(10, 20, 30));
        assert_eq!(snap.document.pages[1].margins_emu, Margins { left: 1, right: 2, top: 3, bottom: 4 });
        assert_eq!(snap.document.pages[1].width_emu, 10_058_400);
        // The id counter survives, so new shapes do not collide.
        let mut d = snap.document;
        let max = d.pages.iter().flat_map(|p| &p.shapes).map(|s| s.id).max().unwrap();
        assert!(d.alloc_id() > max);
        // Picture bytes and group links.
        let ShapeKind::Picture { data, .. } = &d.pages[0].shapes[4].kind else {
            panic!()
        };
        assert_eq!(data.len(), 259);
        assert_eq!(d.pages[0].shapes[1].group, Some(d.pages[0].shapes[6].id));
    }

    #[test]
    fn tables_with_edited_tracks_round_trip_and_old_snapshots_without_merged_load() {
        use crate::command::{Command, Editor};
        let mut e = Editor::new(Document::blank("t.pub", 7_772_400, 10_058_400));
        let rect = crate::table::default_table_rect(7_772_400, 10_058_400, 2, 3);
        let id = e
            .execute(Command::InsertTable { page: 0, rect, rows: 2, columns: 3 })
            .unwrap()[0];
        e.execute(Command::InsertTableRows { page: 0, shape: id, before: 1, count: 1 })
            .unwrap();
        e.execute(Command::DeleteTableColumns { page: 0, shape: id, first: 1, count: 1 })
            .unwrap();
        let mut doc = e.doc().clone();
        if let ShapeKind::Table { cells, .. } = &mut doc.pages[0].shapes[0].kind {
            cells[0] = TableCell::plain("kept");
            cells[1].merged = true;
        }
        let bytes = encode(&meta(1, 1), &doc).unwrap();
        assert_eq!(decode(&bytes).unwrap().document, doc);
        // A snapshot written before cells knew `merged` still loads (cells not merged).
        let text = String::from_utf8(bytes)
            .unwrap()
            .replace(",\"merged\":false", "")
            .replace(",\"merged\":true", "");
        assert!(!text.contains("merged"));
        let snap = decode(text.as_bytes()).unwrap();
        let ShapeKind::Table { rows, cols, cells, .. } = &snap.document.pages[0].shapes[0].kind
        else {
            panic!()
        };
        assert_eq!((*rows, *cols, cells.len()), (3, 2, 6));
        assert!(cells.iter().all(|c| !c.merged));
    }

    fn linked_doc() -> Document {
        let frame = |paras: Vec<Paragraph>| {
            Shape::new(Rect::new(0, 0, 100, 100), ShapeKind::TextBox { paragraphs: paras })
        };
        let mut doc = Document::new(
            "linked",
            vec![Page::blank(1000, 1000), Page::blank(1000, 1000)],
        );
        let (a, b) = (doc.alloc_id(), doc.alloc_id());
        let mut fa = frame(vec![Paragraph::plain("one")]);
        fa.id = a;
        let mut fb = frame(vec![Paragraph::plain("two")]);
        fb.id = b;
        doc.pages[0].shapes.push(fa);
        doc.pages[1].shapes.push(fb);
        doc.stories.push(Story {
            id: 1,
            frames: vec![a, b],
            paragraphs: ["one", "two", "hidden overflow"].map(Paragraph::plain).to_vec(),
            ends: vec![(1, 0), (2, 0)],
        });
        doc
    }

    #[test]
    fn a_version_2_snapshot_keeps_the_whole_story_with_its_overflow() {
        let doc = linked_doc();
        let bytes = encode(&meta(1, 1), &doc).unwrap();
        assert!(String::from_utf8(bytes.clone()).unwrap().contains("\"version\":2"));
        let snap = decode(&bytes).unwrap();
        assert_eq!(snap.document.stories, doc.stories);
        assert_eq!(snap.document, doc);
        let story = &snap.document.stories[0];
        assert_eq!(story.paragraphs.len(), 3);
        assert_eq!(story.overflow()[0].text(), "hidden overflow");
        assert_eq!(snap.document.story_of(story.frames[1]).unwrap().id, 1);
    }

    #[test]
    fn a_version_1_snapshot_still_loads_with_independent_frames() {
        let doc = linked_doc();
        let mut v: serde_json::Value = serde_json::from_slice(&encode(&meta(1, 1), &doc).unwrap()).unwrap();
        v["version"] = 1.into();
        v["document"].as_object_mut().unwrap().remove("stories");
        let snap = decode(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(snap.document.stories.is_empty());
        assert_eq!(snap.document.pages[0].shapes.len(), 1);
        assert_eq!(snap.document.pages[1].shapes.len(), 1);
    }

    #[test]
    fn rejects_wrong_version_and_garbage() {
        let doc = rich_doc();
        let bytes = encode(&meta(1, 1), &doc).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let newer = text.replacen("\"version\":2", "\"version\":3", 1);
        assert!(matches!(decode(newer.as_bytes()), Err(SnapshotError::Version(3))));
        let zero = text.replacen("\"version\":2", "\"version\":0", 1);
        assert!(matches!(decode(zero.as_bytes()), Err(SnapshotError::Version(0))));
        assert!(matches!(decode(b"not json"), Err(SnapshotError::NotASnapshot)));
        assert!(matches!(decode(b"{\"magic\":\"x\",\"version\":1}"), Err(SnapshotError::NotASnapshot)));
        let cut = &text.as_bytes()[..text.len() / 2];
        assert!(decode(cut).is_err());
        let hdr = format!("{{\"magic\":\"{MAGIC}\",\"version\":1,\"meta\":1}}");
        assert!(matches!(decode(hdr.as_bytes()), Err(SnapshotError::Parse(_))));
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let d = temp_dir("atomic");
        let f = d.join("x.pwrec");
        crate::write_atomic(&f, b"one", false).unwrap();
        crate::write_atomic(&f, b"two", false).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn writer_writes_then_deletes_in_order() {
        let d = temp_dir("writer");
        let f = d.join("sub").join("a.pwrec");
        let mut w = Writer::spawn();
        w.write(f.clone(), b"1".to_vec());
        w.write(f.clone(), b"22".to_vec());
        w.finish();
        assert_eq!(std::fs::read(&f).unwrap(), b"22");
        let mut w = Writer::spawn();
        w.write(f.clone(), b"333".to_vec());
        w.delete(f.clone());
        w.finish();
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    fn put(dir: &Path, name: &str, pid: u32, at: u64) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, encode(&meta(pid, at), &Document::blank(name, 1, 1)).unwrap()).unwrap();
        p
    }

    #[test]
    fn scan_picks_only_dead_pids_newest_first_and_moves_unreadable() {
        let d = temp_dir("scan");
        let live = put(&d, "live.pwrec", 100, 500);
        let old = put(&d, "old.pwrec", 200, 10);
        let new = put(&d, "new.pwrec", 201, 99);
        std::fs::write(d.join("bad.pwrec"), b"junk").unwrap();
        std::fs::write(d.join("other.txt"), b"x").unwrap();
        std::fs::write(d.join("half.pwrec.tmp"), b"x").unwrap();
        let alive = |pid: u32| pid == 100;
        let s = scan(&d, &alive);
        let got: Vec<_> = s.recoverable.iter().map(|f| f.path.clone()).collect();
        assert_eq!(got, vec![new.clone(), old.clone()]);
        assert_eq!(s.moved_aside.len(), 1);
        assert!(d.join(UNREADABLE_DIR).join("bad.pwrec").exists());
        assert!(!d.join("bad.pwrec").exists());
        assert!(live.exists());
        // A scan deletes nothing it recovers.
        assert!(old.exists() && new.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn new_snapshots_are_kprec_and_legacy_pwrec_files_are_still_found_and_read() {
        assert!(snapshot_file_name("a.pub", 1).ends_with(".kprec"));
        let d = temp_dir("legacy");
        let legacy = put(&d, "old.pwrec", 200, 10);
        let current = put(&d, &snapshot_file_name("n.pub", 2), 201, 99);
        std::fs::write(d.join("note.txt"), b"x").unwrap();
        let s = scan(&d, &|_| false);
        let got: Vec<_> = s.recoverable.iter().map(|f| f.path.clone()).collect();
        assert_eq!(got, vec![current.clone(), legacy.clone()]);
        // Legacy snapshots are read and removed like current ones.
        let snap = take_most_recent(&d, &|_| false).unwrap();
        assert_eq!(snap.meta.saved_at, 99);
        let snap = take_most_recent(&d, &|_| false).unwrap();
        assert_eq!(snap.meta.saved_at, 10);
        assert!(!legacy.exists() && !current.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn take_most_recent_deletes_only_that_one() {
        let d = temp_dir("take");
        let old = put(&d, "old.pwrec", 200, 10);
        let new = put(&d, "new.pwrec", 201, 99);
        let snap = take_most_recent(&d, &|_| false).unwrap();
        assert_eq!(snap.meta.saved_at, 99);
        assert!(!new.exists());
        assert!(old.exists(), "the others wait for the next start");
        let snap = take_most_recent(&d, &|_| false).unwrap();
        assert_eq!(snap.meta.saved_at, 10);
        assert!(take_most_recent(&d, &|_| false).is_none());
        assert!(take_most_recent(&d.join("missing"), &|_| false).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unreadable_names_do_not_collide() {
        let d = temp_dir("collide");
        for _ in 0..2 {
            std::fs::write(d.join("bad.pwrec"), b"junk").unwrap();
            let s = scan(&d, &|_| false);
            assert_eq!(s.moved_aside.len(), 1);
        }
        assert_eq!(std::fs::read_dir(d.join(UNREADABLE_DIR)).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn dirty_tracking() {
        let k = |r: u64, t: Option<&str>| ContentKey {
            revision: r,
            pending_text: t.map(str::to_owned),
        };
        assert!(needs_snapshot(true, &k(1, None), None));
        assert!(!needs_snapshot(true, &k(1, None), Some(&k(1, None))));
        assert!(needs_snapshot(true, &k(2, None), Some(&k(1, None))));
        assert!(needs_snapshot(true, &k(1, Some("a")), Some(&k(1, None))));
        assert!(!needs_snapshot(true, &k(1, Some("a")), Some(&k(1, Some("a")))));
        assert!(!needs_snapshot(false, &k(5, None), None), "clean documents are never written");
    }

    #[test]
    fn settings_parse_and_save() {
        assert_eq!(Settings::parse(""), Settings::default());
        let s = Settings::parse("AutoRecoverSave=0\nAutoRecoverSaveInterval=45\njunk\n");
        assert_eq!(s, Settings { enabled: false, minutes: 45 });
        assert_eq!(Settings::parse("AutoRecoverSaveInterval=0").minutes, 10);
        assert_eq!(Settings::parse("AutoRecoverSaveInterval=121").minutes, 10);
        assert_eq!(Settings::parse("AutoRecoverSaveInterval=abc").minutes, 10);
        let d = temp_dir("settings");
        let f = d.join("deep").join("settings.txt");
        assert_eq!(Settings::load_from(&f), Settings::default());
        s.save_to(&f).unwrap();
        assert_eq!(Settings::load_from(&f), s);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn minutes_input_rules() {
        // typing 121 leaves 12; 999 leaves 99
        assert_eq!(accept_minutes_text("12", "121"), "12");
        assert_eq!(accept_minutes_text("99", "999"), "99");
        assert_eq!(accept_minutes_text("1", "12"), "12");
        assert_eq!(accept_minutes_text("12", "120"), "120");
        assert_eq!(accept_minutes_text("5", "5a"), "5");
        assert_eq!(accept_minutes_text("5", "-5"), "5");
        // 0 and empty are accepted while editing but not kept
        assert_eq!(accept_minutes_text("", "0"), "0");
        assert_eq!(accept_minutes_text("5", ""), "");
        assert_eq!(commit_minutes("0", 10), None);
        assert_eq!(commit_minutes("", 10), None);
        assert_eq!(commit_minutes("121", 10), None);
        assert_eq!(commit_minutes("1", 10), Some(1));
        assert_eq!(commit_minutes("120", 10), Some(120));
        // spin arrows clamp
        assert_eq!(step_minutes("120", 10, true), 120);
        assert_eq!(step_minutes("1", 10, false), 1);
        assert_eq!(step_minutes("99", 10, true), 100);
        assert_eq!(step_minutes("", 10, true), 11);
    }

    #[test]
    fn file_names() {
        assert_eq!(snapshot_file_name("t2.pub", 0x7BB), "AutoRecover save - t2( 7BB).kprec");
        assert_eq!(snapshot_file_name("Publication1", 0x42D4), "AutoRecover save - Publication1(42D4).kprec");
        assert_eq!(snapshot_file_name("a/b:c.pub", 1), "AutoRecover save - a_b_c(   1).kprec");
        assert_eq!(snapshot_file_name("", 0), "AutoRecover save - Untitled(   0).kprec");
        assert!(new_code(1, 1) <= 0xFFFF);
    }

    #[test]
    fn panic_snapshot_is_written_and_loadable() {
        let d = temp_dir("panic");
        let f = d.join("p.pwrec");
        set_panic_state(Some(PanicState {
            file: f.clone(),
            meta: meta(7, 0),
            document: rich_doc(),
        }));
        assert!(write_panic_snapshot());
        let snap = read_snapshot(&f).unwrap();
        assert_eq!(snap.document, rich_doc());
        assert!(snap.meta.saved_at > 0);
        assert!(!write_panic_snapshot(), "the state is consumed");
        let _ = std::fs::remove_dir_all(&d);
    }
}
