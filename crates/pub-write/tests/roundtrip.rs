use pub_write::{rewrite, set_shape_frame, WriteError};
use std::io::Read;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn samples() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["gen", ""] {
        v.extend(kernpage_samples::files(dir, "pub"));
    }
    v.sort();
    v
}

type Tree = Vec<(String, bool, [u8; 16], u32, Vec<u8>)>;

fn tree(bytes: &[u8]) -> Tree {
    let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).unwrap();
    let entries: Vec<_> = cf
        .walk()
        .map(|e| {
            (
                e.path().to_path_buf(),
                e.is_stream(),
                *e.clsid().as_bytes(),
                e.state_bits(),
            )
        })
        .collect();
    entries
        .into_iter()
        .map(|(p, is_stream, clsid, bits)| {
            let mut data = Vec::new();
            if is_stream {
                cf.open_stream(&p).unwrap().read_to_end(&mut data).unwrap();
            }
            (p.display().to_string(), is_stream, clsid, bits, data)
        })
        .collect()
}

#[kernpage_samples::test]
fn rewrite_preserves_everything() {
    let all = samples();
    assert!(all.len() >= 15);
    for p in all {
        let x = std::fs::read(&p).unwrap();
        let y = rewrite(&x).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(tree(&x), tree(&y), "{}", p.display());
        // Older-generation samples cannot be read by pub_format at all; compare only
        // when the original is readable.
        if let Ok(a) = pub_format::read_bytes(&x) {
            let b = pub_format::read_bytes(&y).unwrap();
            assert_eq!(a, b, "{}", p.display());
        }
    }
}

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap()
}

#[kernpage_samples::test]
fn move_text_box_changes_only_the_frame() {
    let x = gen("apple-banana");
    let before = pub_format::read_bytes(&x).unwrap();
    let (l, t, w, h) = (-1_000_000, -2_000_000, 2_540_000u32, 1_270_000u32);
    let y = set_shape_frame(&x, 0, 0, l, t, w, h).unwrap();
    let after = pub_format::read_bytes(&y).unwrap();
    let s = &after.pages[0].shapes[0];
    assert_eq!((s.left_emu, s.top_emu), (l, t));
    assert_eq!((s.right_emu, s.bottom_emu), (l + w as i32, t + h as i32));
    assert_eq!((s.width_emu, s.height_emu), (Some(w), Some(h)));
    // Everything else unchanged.
    let mut expect = before.clone();
    let e = &mut expect.pages[0].shapes[0];
    (e.left_emu, e.top_emu, e.right_emu, e.bottom_emu) =
        (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu);
    (e.width_emu, e.height_emu) = (s.width_emu, s.height_emu);
    // The story's MCLD frame size follows the box, so columns stay matched.
    assert_eq!(expect, after);
    // Same tree and stream lengths.
    let (ta, tb) = (tree(&x), tree(&y));
    assert_eq!(ta.len(), tb.len());
    for (a, b) in ta.iter().zip(&tb) {
        assert_eq!((&a.0, a.4.len()), (&b.0, b.4.len()));
    }
}

#[kernpage_samples::test]
fn bad_indexes_are_errors() {
    let x = gen("apple-banana");
    assert!(matches!(
        set_shape_frame(&x, 9, 0, 0, 0, 1, 1),
        Err(WriteError::NoSuchPage(9))
    ));
    assert!(matches!(
        set_shape_frame(&x, 0, 99, 0, 0, 1, 1),
        Err(WriteError::NoSuchShape { .. })
    ));
}
