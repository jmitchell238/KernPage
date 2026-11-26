//! The `.kern` fixtures in `samples/kern/`, used by tests that only need "a document".
//!
//! Only the tests of the `.pub` reader and writer open `.pub` files; every other test (UI,
//! export, rendering) opens one of these. They are made from the Publisher-saved samples with the
//! same conversion the app uses when it opens a `.pub`, then written with `write_kern`.
//!
//! Regenerate (needs the source `.pub` files in `samples/`; a missing source is skipped, so a
//! fixture whose `.pub` was removed from the repo stays as committed):
//! `cargo test -p kernpage-core --test kern_fixtures -- --ignored regenerate_kern_fixtures`

use kernpage_core::native::{read_kern, write_kern};
use std::path::PathBuf;

/// (source under `samples/`, fixture name under `samples/kern/`).
const FIXTURES: &[(&str, &str)] = &[
    ("hello.pub", "hello.kern"),
    ("bold.pub", "bold.kern"),
    ("two-pages.pub", "two-pages.kern"),
    ("two-boxes.pub", "two-boxes.kern"),
    ("showcase-newsletter.pub", "showcase-newsletter.kern"),
    ("gen/linked.pub", "linked.kern"),
    ("gen/bulletin.pub", "bulletin.kern"),
    ("gen/group.pub", "group.kern"),
    ("gen/picture.pub", "picture.kern"),
    ("gen/table.pub", "table.kern"),
    ("gen/master-box-1p.pub", "master-box-1p.kern"),
    ("gen/master-box-2p-ignore.pub", "master-box-2p-ignore.kern"),
    ("gen/master-pn-text-2p.pub", "master-pn-text-2p.kern"),
    ("gen/master-pn-2p.pub", "master-pn-2p.kern"),
];

fn samples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

#[test]
#[ignore = "regenerates the committed fixtures"]
fn regenerate_kern_fixtures() {
    std::fs::create_dir_all(samples().join("kern")).unwrap();
    for (src, out) in FIXTURES {
        let Some(path) = kernpage_samples::find(src) else {
            eprintln!("skipped {src}: source not available (set KERNPAGE_EXTRA_SAMPLES)");
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipped {src}: source not in the repo");
            continue;
        };
        let doc = pub_format::read_bytes(&bytes).unwrap();
        let breaks = pub2pdf::chain_breaks(&doc, &pub2pdf::Options::default());
        let model = kernpage_core::from_pub_flowed(src.rsplit('/').next().unwrap(), &doc, &breaks);
        std::fs::write(samples().join("kern").join(out), write_kern(&model).unwrap()).unwrap();
    }
}

#[test]
fn every_kern_fixture_loads() {
    for (_, out) in FIXTURES {
        let bytes = std::fs::read(samples().join("kern").join(out)).unwrap();
        let doc = read_kern(&bytes).unwrap();
        assert!(!doc.pages.is_empty(), "{out} has no pages");
    }
}
