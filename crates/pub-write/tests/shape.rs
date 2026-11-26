//! Add shape (spec 4N): read-back tests on our samples and, when the git-ignored
//! Publisher-made evidence files are present, a byte comparison with them.

use pub_format::{contents::Contents, items, Streams};
use pub_write::{add_shape, add_text_box, NewShapeKind, ShapeStyle};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn bulletin() -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/gen/bulletin.pub"))).unwrap()
}

const EMU: f64 = 12_700.0;
use NewShapeKind::{Line, Oval, Rectangle};

fn plain() -> ShapeStyle {
    ShapeStyle::default()
}

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
}

fn one(src: &[u8], out: &[u8], page: usize) -> pub_format::Shape {
    let n = new_shapes(src, out, page);
    assert_eq!(n.len(), 1);
    n[0].clone()
}

#[kernpage_samples::test]
fn rectangle_on_page_one() {
    let src = bulletin();
    let out = add_shape(&src, 0, Rectangle, 72.0, 660.0, 150.0, 60.0, plain()).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let s = one(&src, &out, 0);
    check_frame(&s, &doc, 72.0, 660.0, 150.0, 60.0);
    assert_eq!(s.geometry, Some(pub_format::ShapeGeometry::Rectangle));
    assert!(s.fill.is_none() && s.chain_story.is_none());
    assert!(new_shapes(&src, &out, 1).is_empty());
    if let Some(theirs) = evidence("r1") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn oval_on_page_one() {
    let src = bulletin();
    let out = add_shape(&src, 0, Oval, 72.0, 660.0, 150.0, 60.0, plain()).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let s = one(&src, &out, 0);
    check_frame(&s, &doc, 72.0, 660.0, 150.0, 60.0);
    assert_eq!(s.geometry, Some(pub_format::ShapeGeometry::Ellipse));
    if let Some(theirs) = evidence("r2") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn rectangle_on_page_two() {
    let src = bulletin();
    let out = add_shape(&src, 1, Rectangle, 72.0, 660.0, 150.0, 60.0, plain()).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    check_frame(&one(&src, &out, 1), &doc, 72.0, 660.0, 150.0, 60.0);
    assert!(new_shapes(&src, &out, 0).is_empty());
    if let Some(theirs) = evidence("rp2") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn oval_on_page_two() {
    let src = bulletin();
    let out = add_shape(&src, 1, Oval, 10.0, 20.0, 30.0, 40.0, plain()).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    check_frame(&one(&src, &out, 1), &doc, 10.0, 20.0, 30.0, 40.0);
}

fn line_case(flip_h: bool, flip_v: bool, (l, t, w, h): (f64, f64, f64, f64), name: &str) {
    let src = bulletin();
    let out = add_shape(&src, 0, Line { flip_h, flip_v }, l, t, w, h, plain()).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let s = one(&src, &out, 0);
    check_frame(&s, &doc, l, t, w, h);
    assert_eq!(s.geometry, Some(pub_format::ShapeGeometry::Line));
    if let Some(theirs) = evidence(name) {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn horizontal_line() {
    line_case(false, false, (72.0, 690.0, 200.0, 0.0), "l1");
}
#[kernpage_samples::test]
fn up_right_line() {
    line_case(false, true, (72.0, 680.0, 200.0, 40.0), "l2");
}
#[kernpage_samples::test]
fn down_right_line() {
    line_case(false, false, (72.0, 680.0, 200.0, 40.0), "l3");
}
#[kernpage_samples::test]
fn up_left_line() {
    line_case(true, true, (72.0, 680.0, 200.0, 40.0), "l4");
}
#[kernpage_samples::test]
fn down_left_line() {
    line_case(true, false, (72.0, 680.0, 200.0, 40.0), "l5");
}

#[kernpage_samples::test]
fn styled_rectangle() {
    let src = bulletin();
    let style = ShapeStyle {
        fill: Some([255, 0, 0]),
        line: Some([0, 0, 255]),
        line_weight_pt: Some(3.0),
        no_line: false,
    };
    let out = add_shape(&src, 0, Rectangle, 72.0, 660.0, 150.0, 60.0, style).unwrap();
    let s = one(&src, &out, 0);
    assert_eq!(s.fill, Some(pub_format::Color::Rgb(255, 0, 0)));
    assert_eq!(s.line, Some((pub_format::Color::Rgb(0, 0, 255), 38_100)));
    if let Some(theirs) = evidence("sty") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn styled_line() {
    let src = bulletin();
    let style = ShapeStyle {
        fill: None,
        line: Some([255, 0, 0]),
        line_weight_pt: Some(2.0),
        no_line: false,
    };
    let out = add_shape(
        &src,
        0,
        Line {
            flip_h: false,
            flip_v: false,
        },
        72.0,
        690.0,
        200.0,
        0.0,
        style,
    )
    .unwrap();
    let s = one(&src, &out, 0);
    assert_eq!(s.line, Some((pub_format::Color::Rgb(255, 0, 0), 25_400)));
    if let Some(theirs) = evidence("lsty") {
        same_as_publisher(&out, &theirs);
    }
}

fn text_box_frame(src: &[u8]) -> (pub_format::Shape, [f64; 4]) {
    let doc = pub_format::read_bytes(src).unwrap();
    let tb = doc.pages[0]
        .shapes
        .iter()
        .find(|s| s.kind == 1 && s.chain_story.is_some())
        .unwrap()
        .clone();
    let cx = f64::from(doc.page_width_emu) / 2.0;
    let cy = f64::from(doc.page_height_emu) / 2.0;
    let l = (f64::from(tb.left_emu) + cx) / EMU;
    let t = (f64::from(tb.top_emu) + cy) / EMU;
    let r = (f64::from(tb.right_emu) + cx) / EMU;
    let b = (f64::from(tb.bottom_emu) + cy) / EMU;
    (tb, [l, t, r - l, b - t])
}

#[kernpage_samples::test]
fn rectangle_over_a_text_box_adds_overlap_items_and_keeps_text() {
    let src = bulletin();
    let (tb, [l, t, _, _]) = text_box_frame(&src);
    let out = add_shape(&src, 0, Rectangle, l + 5.0, t + 5.0, 50.0, 20.0, plain()).unwrap();
    let a = pub_format::read_bytes(&out).unwrap();
    let old = a.pages[0]
        .shapes
        .iter()
        .find(|s| s.object_number == tb.object_number)
        .unwrap();
    assert_eq!(old.text, tb.text);
    assert_eq!(old.runs, tb.runs);
    // The older box body carries items 45 (flag), 70 (count) and 71 (list).
    let st = Streams::from_cfb(&out).unwrap();
    let c = Contents::parse(&st.contents).unwrap();
    let o = c
        .objects
        .iter()
        .find(|o| o.number == tb.object_number)
        .unwrap();
    let len = u32::from_le_bytes(st.contents[o.offset..o.offset + 4].try_into().unwrap()) as usize;
    let body = &st.contents[o.offset..o.offset + len];
    for pat in [[0x2du8, 0x08], [0x46, 0x20], [0x47, 0xa0]] {
        assert!(body.windows(2).any(|w| w == pat), "item {:#x}", pat[0]);
    }
}

#[kernpage_samples::test]
fn rectangle_over_text_box_302_matches_publisher() {
    let src = bulletin();
    let out = add_shape(&src, 0, Rectangle, 100.0, 300.0, 150.0, 80.0, plain()).unwrap();
    if let Some(theirs) = evidence("ov") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn line_over_text_box_302_matches_publisher() {
    let src = bulletin();
    let out = add_shape(
        &src,
        0,
        Line {
            flip_h: false,
            flip_v: false,
        },
        100.0,
        400.0,
        200.0,
        100.0,
        plain(),
    )
    .unwrap();
    if let Some(theirs) = evidence("lov") {
        same_as_publisher(&out, &theirs);
    }
}

#[kernpage_samples::test]
fn rectangle_over_picture_and_shape_matches_publisher() {
    let src = bulletin();
    let out = add_shape(&src, 0, Rectangle, 250.0, 150.0, 100.0, 50.0, plain()).unwrap();
    if let Some(theirs) = evidence("pov") {
        same_as_publisher(&out, &theirs);
    }
    let a = add_shape(&src, 0, Rectangle, 72.0, 660.0, 150.0, 60.0, plain()).unwrap();
    let b = add_shape(&a, 0, Oval, 100.0, 670.0, 150.0, 60.0, plain()).unwrap();
    if let Some(theirs) = evidence("ovs") {
        same_as_publisher(&b, &theirs);
    }
}

#[kernpage_samples::test]
fn line_over_a_text_box_sets_flag_only() {
    let src = bulletin();
    let (tb, [l, t, _, _]) = text_box_frame(&src);
    let out = add_shape(
        &src,
        0,
        Line {
            flip_h: false,
            flip_v: false,
        },
        l - 5.0,
        t + 2.0,
        40.0,
        10.0,
        plain(),
    )
    .unwrap();
    let a = pub_format::read_bytes(&out).unwrap();
    let old = a.pages[0]
        .shapes
        .iter()
        .find(|s| s.object_number == tb.object_number)
        .unwrap();
    assert_eq!(old.text, tb.text);
    assert_eq!(
        a.pages[0].shapes.len(),
        pub_format::read_bytes(&src).unwrap().pages[0].shapes.len() + 1
    );
}

#[kernpage_samples::test]
fn two_shapes_and_a_text_box_in_a_row() {
    let src = bulletin();
    let a = add_shape(&src, 0, Rectangle, 72.0, 660.0, 100.0, 40.0, plain()).unwrap();
    let b = add_shape(&a, 0, Oval, 300.0, 660.0, 100.0, 40.0, plain()).unwrap();
    let c = add_text_box(&b, 0, 72.0, 720.0, 100.0, 30.0, "Hi").unwrap();
    let d = add_shape(
        &c,
        1,
        Line {
            flip_h: false,
            flip_v: true,
        },
        50.0,
        50.0,
        100.0,
        100.0,
        plain(),
    )
    .unwrap();
    let doc = pub_format::read_bytes(&d).unwrap();
    let n = new_shapes(&src, &d, 0);
    assert_eq!(n.len(), 3);
    check_frame(&n[0], &doc, 72.0, 660.0, 100.0, 40.0);
    check_frame(&n[1], &doc, 300.0, 660.0, 100.0, 40.0);
    check_frame(&n[2], &doc, 72.0, 720.0, 100.0, 30.0);
    assert_eq!(
        n[2].text.as_deref().map(|t| t.trim_end_matches('\r')),
        Some("Hi")
    );
    assert_eq!(new_shapes(&src, &d, 1).len(), 1);
    let ids: std::collections::HashSet<u32> = doc
        .pages
        .iter()
        .flat_map(|p| p.shapes.iter().map(|s| s.object_number))
        .collect();
    assert_eq!(
        ids.len(),
        doc.pages.iter().map(|p| p.shapes.len()).sum::<usize>()
    );
    if let Some(theirs) = evidence("two") {
        // "two" = rectangle 72,660,100,40 + oval 300,660,100,40 in one session.
        same_as_publisher(&b, &theirs);
    }
}

#[kernpage_samples::test]
fn existing_content_is_unchanged() {
    let src = bulletin();
    let before = pub_format::read_bytes(&src).unwrap();
    let out = add_shape(&src, 0, Oval, 72.0, 660.0, 100.0, 40.0, plain()).unwrap();
    let after = pub_format::read_bytes(&out).unwrap();
    assert_eq!(after.stories, before.stories);
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
    // Quill is untouched.
    assert_eq!(
        Streams::from_cfb(&src).unwrap().quill,
        Streams::from_cfb(&out).unwrap().quill
    );
}

#[kernpage_samples::test]
fn bad_input_is_an_error() {
    let src = bulletin();
    assert!(add_shape(&src, 9, Rectangle, 0.0, 0.0, 10.0, 10.0, plain()).is_err());
    assert!(add_shape(&src, 0, Rectangle, 0.0, 0.0, 0.0, 10.0, plain()).is_err());
    assert!(add_shape(&src, 0, Oval, f64::NAN, 0.0, 10.0, 10.0, plain()).is_err());
    assert!(add_shape(
        &src,
        0,
        Line {
            flip_h: false,
            flip_v: false
        },
        0.0,
        0.0,
        -1.0,
        10.0,
        plain()
    )
    .is_err());
    assert!(add_shape(
        b"not a pub file",
        0,
        Rectangle,
        0.0,
        0.0,
        10.0,
        10.0,
        plain()
    )
    .is_err());
}

// ---- comparison with Publisher's own output (git-ignored evidence)

fn evidence(name: &str) -> Option<Vec<u8>> {
    std::fs::read(root().join(format!("out/shape-research/{name}.pub"))).ok()
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
    // Byte 106-107 are unused bytes of a BSE record; Publisher writes stale memory.
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

/// With `SHAPE_OUT=<dir>` set, write the cases there for opening in Publisher.
#[kernpage_samples::test]
fn write_cases_when_asked() {
    let Ok(dir) = std::env::var("SHAPE_OUT") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = bulletin();
    let put = |n: &str, b: Vec<u8>| std::fs::write(dir.join(format!("{n}.pub")), b).unwrap();
    put(
        "rect",
        add_shape(&src, 0, Rectangle, 72.0, 660.0, 150.0, 60.0, plain()).unwrap(),
    );
    put(
        "oval",
        add_shape(&src, 0, Oval, 72.0, 660.0, 150.0, 60.0, plain()).unwrap(),
    );
    put(
        "line",
        add_shape(
            &src,
            0,
            Line {
                flip_h: false,
                flip_v: true,
            },
            72.0,
            680.0,
            200.0,
            40.0,
            plain(),
        )
        .unwrap(),
    );
    let style = ShapeStyle {
        fill: Some([255, 0, 0]),
        line: Some([0, 0, 255]),
        line_weight_pt: Some(3.0),
        no_line: false,
    };
    put(
        "styled",
        add_shape(&src, 0, Rectangle, 72.0, 660.0, 150.0, 60.0, style).unwrap(),
    );
    let a = add_shape(&src, 0, Rectangle, 72.0, 660.0, 100.0, 40.0, plain()).unwrap();
    let b = add_shape(&a, 0, Oval, 300.0, 660.0, 100.0, 40.0, plain()).unwrap();
    let c = add_text_box(&b, 0, 72.0, 720.0, 100.0, 30.0, "Hi").unwrap();
    put(
        "mixed",
        add_shape(
            &c,
            1,
            Line {
                flip_h: false,
                flip_v: true,
            },
            50.0,
            50.0,
            100.0,
            100.0,
            plain(),
        )
        .unwrap(),
    );
}
