//! Wrap type and text box columns of existing objects: write, read back (spec 4I, 4K).

use pub_format::WrapType;
use pub_write::{set_shape_wrap, set_text_columns};
use std::path::PathBuf;

fn sample(name: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read(kernpage_samples::resolve(root.join("samples/gen").join(name))).unwrap()
}

const ALL: [WrapType; 5] = [
    WrapType::None,
    WrapType::TopAndBottom,
    WrapType::Square,
    WrapType::Tight,
    WrapType::Through,
];

#[kernpage_samples::test]
fn every_wrap_type_round_trips_on_every_shape() {
    for name in ["wrap.pub", "bulletin.pub"] {
        let src = sample(name);
        let doc = pub_format::read_bytes(&src).unwrap();
        let mut n = 0;
        for (p, page) in doc.pages.iter().enumerate() {
            for s in 0..page.shapes.len() {
                // Walk through all types, in a chain, so insert/remove paths interact.
                let mut cur = src.clone();
                for w in ALL.into_iter().chain([WrapType::Square, WrapType::TopAndBottom]) {
                    cur = set_shape_wrap(&cur, p, s, w).unwrap();
                    let d = pub_format::read_bytes(&cur).unwrap();
                    assert_eq!(d.pages[p].shapes[s].wrap, w, "{name} {p}/{s}");
                    for (q, pg) in d.pages.iter().enumerate() {
                        for (t, o) in pg.shapes.iter().enumerate() {
                            if (q, t) != (p, s) {
                                assert_eq!(o.wrap, doc.pages[q].shapes[t].wrap, "{name}");
                                assert_eq!(o.left_emu, doc.pages[q].shapes[t].left_emu);
                            }
                        }
                    }
                }
                n += 1;
            }
        }
        assert!(n >= 1);
    }
}

#[kernpage_samples::test]
fn unchanged_wrap_is_byte_identical() {
    let src = sample("wrap.pub");
    let doc = pub_format::read_bytes(&src).unwrap();
    let (p, s) = (
        0,
        doc.pages[0]
            .shapes
            .iter()
            .position(|s| s.wrap == WrapType::Square)
            .unwrap(),
    );
    assert_eq!(set_shape_wrap(&src, p, s, WrapType::Square).unwrap(), src);
}

fn first_text_box(bytes: &[u8]) -> (usize, usize) {
    let d = pub_format::read_bytes(bytes).unwrap();
    for (p, page) in d.pages.iter().enumerate() {
        if let Some(s) = page
            .shapes
            .iter()
            .position(|s| s.story.is_some() && s.table.is_none())
        {
            return (p, s);
        }
    }
    panic!("no text box");
}

#[kernpage_samples::test]
fn column_counts_and_gap_round_trip() {
    for name in ["columns.pub", "wrap.pub", "bulletin.pub"] {
        let Some(src) = kernpage_samples::read_opt(&format!("gen/{name}")) else {
            continue;
        };
        let d = pub_format::read_bytes(&src).unwrap();
        let (p, s) = (0..d.pages.len())
            .flat_map(|p| (0..d.pages[p].shapes.len()).map(move |s| (p, s)))
            .find(|&(p, s)| {
                let sh = &d.pages[p].shapes[s];
                sh.story.is_some()
                    && sh.table.is_none()
                    && set_text_columns(&src, p, s, 1, 0).is_ok()
            })
            .unwrap_or_else(|| panic!("{name}: no writable text box"));
        let _ = first_text_box(&src);
        for (cols, gap) in [
            (1, 82_676 / 2),
            (2, 228_600),
            (3, 228_600),
            (4, 91_440),
            (1, 36_576),
        ] {
            let out = set_text_columns(&src, p, s, cols, gap).unwrap();
            let sh = &pub_format::read_bytes(&out).unwrap().pages[p].shapes[s];
            assert_eq!((sh.columns, sh.column_gap_emu), (cols, gap), "{name}");
        }
    }
}

#[kernpage_samples::test]
fn rejects_bad_columns() {
    let src = sample("columns.pub");
    let (p, s) = first_text_box(&src);
    assert!(set_text_columns(&src, p, s, 0, 0).is_err());
}

/// Writes the files for the Publisher 2021 check into `$WRAPCOLS_OUT` (ignored by default).
#[kernpage_samples::test]
#[ignore]
fn write_wrap_cols_samples() {
    let Ok(dir) = std::env::var("WRAPCOLS_OUT") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let bul = sample("bulletin.pub");
    let d = pub_format::read_bytes(&bul).unwrap();
    let pic = d.pages[0].shapes.iter().position(|s| s.picture.is_some()).unwrap();
    let tb = first_text_box(&bul).1;
    for (tag, w) in [
        ("none", WrapType::None),
        ("tb", WrapType::TopAndBottom),
        ("square", WrapType::Square),
        ("tight", WrapType::Tight),
        ("through", WrapType::Through),
    ] {
        std::fs::write(dir.join(format!("pic-{tag}.pub")), set_shape_wrap(&bul, 0, pic, w).unwrap()).unwrap();
        std::fs::write(dir.join(format!("box-{tag}.pub")), set_shape_wrap(&bul, 0, tb, w).unwrap()).unwrap();
    }
    for cols in [2u32, 3] {
        let o = set_text_columns(&bul, 0, tb, cols, 228_600).unwrap();
        std::fs::write(dir.join(format!("bulletin-cols{cols}.pub")), o).unwrap();
    }
    let c = sample("columns.pub");
    let (p, s) = (0, c.len());
    let _ = (p, s);
    let dc = pub_format::read_bytes(&c).unwrap();
    let ci = dc.pages[0].shapes.iter().position(|s| s.columns == 3).unwrap();
    std::fs::write(dir.join("columns-to1.pub"), set_text_columns(&c, 0, ci, 1, 36_576).unwrap()).unwrap();
    std::fs::write(dir.join("base-bulletin.pub"), &bul).unwrap();
    std::fs::write(dir.join("base-columns.pub"), &c).unwrap();
    println!("pic={pic} tb={tb} columns.pub shape={ci}");
}
