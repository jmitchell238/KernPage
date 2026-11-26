//! Add text box (spec 4L): read-back tests on our samples and, when the git-ignored
//! Publisher-made evidence files are present, a structural comparison with them.

use pub_format::{contents::Contents, items, Streams};
use pub_write::add_text_box;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn bulletin() -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/gen/bulletin.pub"))).unwrap()
}

const EMU: f64 = 12_700.0;

/// The shape of `after` page `page` whose object number is not in `before`.
fn new_shapes(before: &[u8], after: &[u8], page: usize) -> Vec<pub_format::Shape> {
    let b = pub_format::read_bytes(before).unwrap();
    let a = pub_format::read_bytes(after).unwrap();
    let old: Vec<u32> = b.pages[page]
        .shapes
        .iter()
        .map(|s| s.object_number)
        .collect();
    a.pages[page]
        .shapes
        .iter()
        .filter(|s| !old.contains(&s.object_number))
        .cloned()
        .collect()
}

fn check_frame(s: &pub_format::Shape, doc: &pub_format::Document, l: f64, t: f64, w: f64, h: f64) {
    let cx = f64::from(doc.page_width_emu) / 2.0;
    let cy = f64::from(doc.page_height_emu) / 2.0;
    let near = |emu: i32, c: f64, pt: f64| ((f64::from(emu) + c) / EMU - pt).abs() <= 1.0;
    assert!(near(s.left_emu, cx, l), "left {}", s.left_emu);
    assert!(near(s.top_emu, cy, t), "top {}", s.top_emu);
    assert!(near(s.right_emu, cx, l + w), "right {}", s.right_emu);
    assert!(near(s.bottom_emu, cy, t + h), "bottom {}", s.bottom_emu);
    assert!((f64::from(s.width_emu.unwrap()) / EMU - w).abs() <= 1.0);
    assert!((f64::from(s.height_emu.unwrap()) / EMU - h).abs() <= 1.0);
}

fn text_of(s: &pub_format::Shape) -> String {
    s.text.clone().unwrap_or_default()
}

#[kernpage_samples::test]
fn hello_on_page_one() {
    let src = bulletin();
    let out = add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "Hello").unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let n = new_shapes(&src, &out, 0);
    assert_eq!(n.len(), 1);
    check_frame(&n[0], &doc, 72.0, 600.0, 200.0, 50.0);
    assert_eq!(text_of(&n[0]).trim_end_matches('\r'), "Hello");
    assert!(new_shapes(&src, &out, 1).is_empty());
    if let Some(theirs) = evidence("t1") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn two_paragraphs_on_page_two() {
    let src = bulletin();
    let out = add_text_box(&src, 1, 72.0, 600.0, 200.0, 50.0, "Hello\rWorld").unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let n = new_shapes(&src, &out, 1);
    assert_eq!(n.len(), 1);
    check_frame(&n[0], &doc, 72.0, 600.0, 200.0, 50.0);
    assert_eq!(n[0].paragraphs.len(), 2);
    assert_eq!(
        text_of(&n[0]).replace('\r', "\n").trim_end(),
        "Hello\nWorld"
    );
    assert!(new_shapes(&src, &out, 0).is_empty());
    if let Some(theirs) = evidence("t2p2") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn empty_text() {
    let src = bulletin();
    let out = add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "").unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let n = new_shapes(&src, &out, 0);
    assert_eq!(n.len(), 1);
    check_frame(&n[0], &doc, 72.0, 600.0, 200.0, 50.0);
    assert_eq!(text_of(&n[0]).trim_end_matches('\r'), "");
    if let Some(theirs) = evidence("t3") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn two_boxes_in_a_row() {
    let src = bulletin();
    let one = add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "AAA").unwrap();
    let two = add_text_box(&one, 0, 72.0, 660.0, 100.0, 40.0, "BBBB").unwrap();
    let doc = pub_format::read_bytes(&two).unwrap();
    let n = new_shapes(&src, &two, 0);
    assert_eq!(n.len(), 2);
    let s: Vec<u32> = n.iter().map(|s| s.chain_story.unwrap()).collect();
    assert!(s[0] < s[1], "story numbers {s:?}");
    check_frame(&n[0], &doc, 72.0, 600.0, 200.0, 50.0);
    check_frame(&n[1], &doc, 72.0, 660.0, 100.0, 40.0);
    assert_eq!(text_of(&n[0]).trim_end_matches('\r'), "AAA");
    assert_eq!(text_of(&n[1]).trim_end_matches('\r'), "BBBB");
}

#[kernpage_samples::test]
fn existing_stories_are_unchanged() {
    let src = bulletin();
    let before = pub_format::read_bytes(&src).unwrap();
    let out = add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "Hello\rWorld").unwrap();
    let after = pub_format::read_bytes(&out).unwrap();
    assert_eq!(after.stories.len(), before.stories.len() + 1);
    assert_eq!(&after.stories[..before.stories.len()], &before.stories[..]);
    for p in 0..before.pages.len() {
        for (a, b) in before.pages[p].shapes.iter().zip(&after.pages[p].shapes) {
            assert_eq!(a.object_number, b.object_number);
            assert_eq!(a.text, b.text);
            assert_eq!(a.runs, b.runs);
            assert_eq!(
                [a.left_emu, a.top_emu, a.right_emu, a.bottom_emu],
                [b.left_emu, b.top_emu, b.right_emu, b.bottom_emu]
            );
        }
    }
}

#[kernpage_samples::test]
fn bad_input_is_an_error() {
    let src = bulletin();
    assert!(add_text_box(&src, 9, 0.0, 0.0, 10.0, 10.0, "x").is_err());
    assert!(add_text_box(&src, 0, 0.0, 0.0, 0.0, 10.0, "x").is_err());
    assert!(add_text_box(&src, 0, f64::NAN, 0.0, 10.0, 10.0, "x").is_err());
    assert!(add_text_box(b"not a pub file", 0, 0.0, 0.0, 10.0, 10.0, "x").is_err());
}

#[kernpage_samples::test]
fn overlapped_older_box_gets_bookkeeping_and_still_reads() {
    let src = bulletin();
    let doc = pub_format::read_bytes(&src).unwrap();
    let tb = doc.pages[0]
        .shapes
        .iter()
        .find(|s| s.kind == 1 && s.chain_story.is_some())
        .unwrap();
    let cx = f64::from(doc.page_width_emu) / 2.0;
    let cy = f64::from(doc.page_height_emu) / 2.0;
    let l = (f64::from(tb.left_emu) + cx) / EMU + 5.0;
    let t = (f64::from(tb.top_emu) + cy) / EMU + 5.0;
    let out = add_text_box(&src, 0, l, t, 50.0, 20.0, "Over").unwrap();
    let again = add_text_box(&out, 0, l, t, 40.0, 10.0, "Over2").unwrap();
    let a = pub_format::read_bytes(&again).unwrap();
    assert_eq!(a.pages[0].shapes.len(), doc.pages[0].shapes.len() + 2);
    let old = a.pages[0]
        .shapes
        .iter()
        .find(|s| s.object_number == tb.object_number)
        .unwrap();
    assert_eq!(old.text, tb.text);
}

// ---- comparison with Publisher's own output (git-ignored evidence)

fn evidence(name: &str) -> Option<Vec<u8>> {
    std::fs::read(root().join(format!("out/textbox-research/{name}.pub"))).ok()
}

fn contents_view(bytes: &[u8]) -> Vec<(u16, Option<u32>, Vec<u8>)> {
    let st = Streams::from_cfb(bytes).unwrap();
    let c = Contents::parse(&st.contents).unwrap();
    c.objects
        .iter()
        .map(|o| {
            let len = u32::from_le_bytes(st.contents[o.offset..o.offset + 4].try_into().unwrap())
                as usize;
            let mut b = st.contents[o.offset..o.offset + len].to_vec();
            if o.kind == 68 {
                if let Some(s) = items::locate_int(&b, 0, 60).unwrap() {
                    b[s.offset..s.offset + s.len].fill(0);
                }
            }
            (o.kind, o.parent, b)
        })
        .collect()
}

fn same_as_publisher(ours: &[u8], theirs: &[u8]) {
    let mut a = Streams::from_cfb(ours).unwrap();
    let mut b = Streams::from_cfb(theirs).unwrap();
    // Bytes 106-107 are the BSE record's two unused bytes; Publisher writes stale memory.
    a.escher[106..108].fill(0);
    b.escher[106..108].fill(0);
    if a.escher != b.escher {
        let at = a.escher.iter().zip(&b.escher).position(|(x, y)| x != y);
        panic!(
            "EscherStm differs from Publisher's: lengths {} vs {}, first diff at {at:?}: {:02x?} vs {:02x?}",
            a.escher.len(),
            b.escher.len(),
            at.map(|i| &a.escher[i..(i + 16).min(a.escher.len())]),
            at.map(|i| &b.escher[i..(i + 16).min(b.escher.len())]),
        );
    }
    let (va, vb) = (contents_view(ours), contents_view(theirs));
    assert_eq!(va.len(), vb.len(), "object count");
    for (i, (x, y)) in va.iter().zip(&vb).enumerate() {
        assert_eq!((x.0, x.1), (y.0, y.1), "object #{i} kind/parent");
        assert_eq!(x.2, y.2, "object #{i} (kind {}) block", x.0);
    }
}

/// With `TEXTBOX_OUT=<dir>` set, write the cases there for opening in Publisher.
#[kernpage_samples::test]
fn write_cases_when_asked() {
    let Ok(dir) = std::env::var("TEXTBOX_OUT") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = bulletin();
    let put = |n: &str, b: Vec<u8>| std::fs::write(dir.join(format!("{n}.pub")), b).unwrap();
    put(
        "a",
        add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "Hello").unwrap(),
    );
    put(
        "b",
        add_text_box(&src, 1, 72.0, 600.0, 200.0, 50.0, "Hello\rWorld").unwrap(),
    );
    put(
        "c",
        add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "").unwrap(),
    );
    let one = add_text_box(&src, 0, 72.0, 600.0, 200.0, 50.0, "AAA").unwrap();
    put(
        "d",
        add_text_box(&one, 0, 72.0, 660.0, 100.0, 40.0, "BBBB").unwrap(),
    );
    put(
        "e",
        add_text_box(&src, 0, 100.0, 300.0, 150.0, 60.0, "Mid page").unwrap(),
    );
}
