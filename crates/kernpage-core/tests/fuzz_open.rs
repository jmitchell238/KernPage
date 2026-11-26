//! Robustness ("fuzz-style") test for the open path (security audit pw-ax78.8, PW-SEC-001).
//!
//! Every `samples/**/*.pub` is fed to the same chain the editor runs when a file is opened
//! (`pub_format::read_bytes`, text flow, model conversion, `SaveSource`) and to the PDF
//! exporter, after truncation and deterministic byte mutation. Each case runs on a worker
//! thread with
//!
//! * a time cap (the case must finish within [`TIME_CAP`], or the test fails),
//! * a memory cap (peak live heap above the case's start must stay under [`MEM_CAP`];
//!   measured by a counting global allocator), and
//! * `catch_unwind` (as `App::open_path` does): a panic is tolerated by the app, but the
//!   parser itself (`read_bytes`) must not panic, so that is asserted; panics further down
//!   the chain are counted and must stay at zero too.
//!
//! The mutations are deterministic (fixed seed), so a failure is reproducible.

use kernpage_core::{from_pub_flowed, SaveSource};
use std::alloc::{GlobalAlloc, Layout, System};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TIME_CAP: Duration = Duration::from_secs(60);
const MEM_CAP: usize = 1536 << 20;

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // SAFETY: forwards to the system allocator with the caller's layout.
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let now = LIVE.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        // SAFETY: `p` came from `alloc`/`realloc` below with this layout.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        // SAFETY: forwards to the system allocator unchanged.
        let q = unsafe { System.realloc(p, l, new) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), Ordering::Relaxed);
            let now = LIVE.fetch_add(new, Ordering::Relaxed) + new;
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        q
    }
}

#[global_allocator]
static A: Counting = Counting;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[derive(Default)]
struct Outcome {
    parse_panic: bool,
    later_panic: bool,
}

/// The editor's open path plus the PDF export, on one input.
fn open_chain(bytes: Vec<u8>, export: bool) -> Outcome {
    let mut out = Outcome::default();
    let parsed = catch_unwind(AssertUnwindSafe(|| pub_format::read_bytes(&bytes)));
    let doc = match parsed {
        Err(_) => {
            out.parse_panic = true;
            return out;
        }
        Ok(Err(_)) => return out,
        Ok(Ok(d)) => d,
    };
    let later = catch_unwind(AssertUnwindSafe(|| {
        let breaks = pub2pdf::chain_breaks(&doc, &pub2pdf::Options::default());
        let model = from_pub_flowed("fuzz.pub", &doc, &breaks);
        let _source = SaveSource::new(bytes.clone(), &doc, &model, &breaks);
        if export {
            let _ = kernpage_core::export_pdf(&model, Vec::new());
        }
    }));
    out.later_panic = later.is_err();
    out
}

/// Panics are counted by the tests, not printed (the hook is process-wide and stays set).
fn silence_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}

fn samples() -> Vec<PathBuf> {
    // `samples/external` is a local, git-ignored cache of third-party files; skip it so the test
    // gives the same result everywhere.
    let mut v = kernpage_samples::files("gen", "pub");
    v.extend(kernpage_samples::files("", "pub"));
    v.sort();
    v
}

/// Run one case on a worker thread; fail on timeout or memory blow-up.
fn run_case(label: &str, bytes: Vec<u8>, export: bool) -> Outcome {
    let (tx, rx) = mpsc::channel();
    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || {
            let _ = tx.send(open_chain(bytes, export));
        })
        .unwrap();
    let started = Instant::now();
    let out = rx
        .recv_timeout(TIME_CAP)
        .unwrap_or_else(|_| panic!("{label}: still running after {TIME_CAP:?} (possible hang)"));
    let used = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    assert!(used < MEM_CAP, "{label}: peak heap {} MB (cap {} MB)", used >> 20, MEM_CAP >> 20);
    assert!(started.elapsed() < TIME_CAP, "{label}: too slow");
    out
}

#[kernpage_samples::test]
fn truncated_and_mutated_samples_never_panic_hang_or_blow_memory() {
    silence_panics();
    let files = samples();
    let want = if kernpage_samples::extra_samples().is_some() { 20 } else { 10 };
    assert!(files.len() >= want, "samples missing: {} files", files.len());
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut cases, mut parse_panics, mut later_panics) = (0usize, Vec::new(), Vec::new());
    for (n, f) in files.iter().enumerate() {
        let data = std::fs::read(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
        // Truncations (every file; the PDF export only on the first few to keep the test fast).
        for k in 0..8 {
            let cut = data.len() * k / 8 + rng.below(data.len() / 8 + 1);
            inputs.push((format!("{name} cut {cut}"), data[..cut.min(data.len())].to_vec()));
        }
        // Mutations: a few random bytes, a burst, and header damage.
        for k in 0..10 {
            let mut m = data.clone();
            let flips = 1 + rng.below(if k < 5 { 4 } else { 64 });
            for _ in 0..flips {
                // Half the flips land in the first 4 KB (CFB header, FAT, directory).
                let at = if rng.below(2) == 0 { rng.below(4096.min(m.len())) } else { rng.below(m.len()) };
                m[at] = (rng.next() & 0xFF) as u8;
            }
            inputs.push((format!("{name} mut {k}"), m));
        }
        // Length-field style damage: 0xFFFFFFFF / 0 over a random aligned dword.
        for k in 0..6 {
            let mut m = data.clone();
            let at = rng.below(m.len().saturating_sub(4)) & !3;
            let v: [u8; 4] = if k % 2 == 0 { [0xFF; 4] } else { [0; 4] };
            m[at..at + 4].copy_from_slice(&v);
            inputs.push((format!("{name} dword {k} @{at}"), m));
        }
        for (i, (label, bytes)) in inputs.into_iter().enumerate() {
            let o = run_case(&label, bytes.clone(), n < 6 && i % 4 == 0);
            cases += 1;
            if o.parse_panic {
                parse_panics.push(label.clone());
            }
            if o.later_panic {
                later_panics.push(label);
            }
        }
    }
    eprintln!("fuzz_open: {cases} cases over {} files", files.len());
    assert!(parse_panics.is_empty(), "pub_format::read_bytes panicked on: {parse_panics:#?}");
    assert!(later_panics.is_empty(), "flow/convert/save-source/PDF panicked on: {later_panics:#?}");
}

/// The parser proper (past the compound-file layer, where most random damage is caught): mutate
/// the four extracted streams and run `assemble` plus the conversion on them.
#[kernpage_samples::test]
fn mutated_streams_never_panic_hang_or_blow_memory() {
    silence_panics();
    let files = samples();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let (mut cases, mut panics) = (0usize, Vec::new());
    for f in &files {
        let data = std::fs::read(f).unwrap();
        let Ok(base) = pub_format::Streams::from_cfb(&data) else { continue };
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        for k in 0..40usize {
            let mut st = pub_format::Streams {
                contents: base.contents.clone(),
                escher: base.escher.clone(),
                quill: base.quill.clone(),
                delay: base.delay.clone(),
            };
            let which = rng.below(4);
            let v = match which {
                0 => &mut st.contents,
                1 => &mut st.escher,
                2 => &mut st.quill,
                _ => &mut st.delay,
            };
            if v.is_empty() {
                continue;
            }
            match k % 4 {
                0 => v.truncate(rng.below(v.len())),
                1 => {
                    for _ in 0..1 + rng.below(6) {
                        let at = rng.below(v.len());
                        v[at] = (rng.next() & 0xFF) as u8;
                    }
                }
                2 => {
                    let at = rng.below(v.len().saturating_sub(4).max(1)) & !3;
                    if at + 4 <= v.len() {
                        v[at..at + 4].copy_from_slice(&[0xFF; 4]);
                    }
                }
                _ => {
                    for _ in 0..64 {
                        let at = rng.below(v.len());
                        v[at] = (rng.next() & 0xFF) as u8;
                    }
                }
            }
            let label = format!("{name} stream {which} case {k}");
            let (tx, rx) = mpsc::channel();
            let base_live = LIVE.load(Ordering::Relaxed);
            PEAK.store(base_live, Ordering::Relaxed);
            std::thread::Builder::new()
                .stack_size(8 << 20)
                .spawn(move || {
                    let r = catch_unwind(AssertUnwindSafe(|| {
                        if let Ok(doc) = pub_format::assemble(&st) {
                            let breaks = pub2pdf::chain_breaks(&doc, &pub2pdf::Options::default());
                            let model = from_pub_flowed("fuzz.pub", &doc, &breaks);
                            let _ = SaveSource::new(Vec::new(), &doc, &model, &breaks);
                        }
                    }));
                    let _ = tx.send(r.is_err());
                })
                .unwrap();
            let panicked = rx
                .recv_timeout(TIME_CAP)
                .unwrap_or_else(|_| panic!("{label}: still running after {TIME_CAP:?} (possible hang)"));
            let used = PEAK.load(Ordering::Relaxed).saturating_sub(base_live);
            assert!(used < MEM_CAP, "{label}: peak heap {} MB", used >> 20);
            cases += 1;
            if panicked {
                panics.push(label);
            }
        }
    }
    eprintln!("fuzz_open (streams): {cases} cases");
    assert!(cases > 500, "too few cases ran: {cases}");
    assert!(panics.is_empty(), "panicked on: {panics:#?}");
}

#[kernpage_samples::test]
fn degenerate_inputs_are_refused_not_crashed() {
    let cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![0u8; 1],
        vec![0u8; 511],
        vec![0u8; 512],
        vec![0xFF; 4096],
        // The CFB signature then nothing sensible.
        {
            let mut v = vec![0u8; 1024];
            v[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
            v
        },
        {
            let mut v = vec![0xFFu8; 4096];
            v[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
            v
        },
    ];
    for (i, c) in cases.into_iter().enumerate() {
        let o = run_case(&format!("degenerate {i}"), c, true);
        assert!(!o.parse_panic && !o.later_panic, "degenerate input {i} panicked");
    }
}
