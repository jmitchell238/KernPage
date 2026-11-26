//! Fill, outline, rotation and flip of existing shapes: write, read back (spec 4D, 4H).

use pub_format::Color;
use pub_write::{set_shape_style, ShapeLook};
use std::path::PathBuf;

fn sample(name: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read(kernpage_samples::resolve(root.join("samples/gen").join(name))).unwrap()
}

/// (page, shape) of the first shape matching `pred`.
fn find(bytes: &[u8], pred: impl Fn(&pub_format::Shape) -> bool) -> (usize, usize) {
    let d = pub_format::read_bytes(bytes).unwrap();
    for (p, page) in d.pages.iter().enumerate() {
        if let Some(s) = page.shapes.iter().position(&pred) {
            return (p, s);
        }
    }
    panic!("no such shape");
}

fn shape(bytes: &[u8], p: usize, s: usize) -> pub_format::Shape {
    pub_format::read_bytes(bytes).unwrap().pages[p].shapes[s].clone()
}

fn frame(s: &pub_format::Shape) -> (i32, i32, i32, i32) {
    (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu)
}

#[kernpage_samples::test]
fn fill_and_outline_round_trip_on_shapes_pub() {
    let src = sample("shapes.pub");
    let (p, s) = find(&src, |s| s.fill.is_some() || s.line.is_some());
    let look = ShapeLook {
        fill: Some(Some([10, 20, 30])),
        line_on: Some(true),
        line_color: Some([200, 100, 50]),
        line_weight_emu: Some(38_100),
        ..Default::default()
    };
    let out = set_shape_style(&src, p, s, &look).unwrap();
    let got = shape(&out, p, s);
    assert_eq!(got.fill, Some(Color::Rgb(10, 20, 30)));
    assert_eq!(got.line, Some((Color::Rgb(200, 100, 50), 38_100)));
    assert_eq!(frame(&got), frame(&shape(&src, p, s)));
    let off = ShapeLook {
        fill: Some(None),
        line_on: Some(false),
        ..Default::default()
    };
    let out2 = set_shape_style(&out, p, s, &off).unwrap();
    let got = shape(&out2, p, s);
    assert_eq!((got.fill, got.line), (None, None));
    let out3 = set_shape_style(&out2, p, s, &look).unwrap();
    assert_eq!(shape(&out3, p, s).fill, Some(Color::Rgb(10, 20, 30)));
    assert_eq!(shape(&out3, p, s).line, Some((Color::Rgb(200, 100, 50), 38_100)));
}

#[kernpage_samples::test]
fn weight_change_on_every_sample_shape_leaves_the_others_alone() {
    let src = sample("shapes.pub");
    let d = pub_format::read_bytes(&src).unwrap();
    for (p, page) in d.pages.iter().enumerate() {
        for (s, sh) in page.shapes.iter().enumerate() {
            if sh.line.is_none() {
                continue;
            }
            let look = ShapeLook {
                line_weight_emu: Some(25_400),
                ..Default::default()
            };
            let out = set_shape_style(&src, p, s, &look).unwrap();
            let n = pub_format::read_bytes(&out).unwrap();
            for (i, other) in n.pages[p].shapes.iter().enumerate() {
                if i != s {
                    assert_eq!(other.line, page.shapes[i].line);
                    assert_eq!(other.fill, page.shapes[i].fill);
                }
            }
            assert_eq!(n.pages[p].shapes[s].line.map(|l| l.1), Some(25_400));
        }
    }
}

#[kernpage_samples::test]
fn line_flip_flags_round_trip() {
    let plain = sample("bulletin.pub");
    let src = pub_write::add_shape(
        &plain,
        0,
        pub_write::NewShapeKind::Line {
            flip_h: false,
            flip_v: false,
        },
        72.0,
        680.0,
        200.0,
        40.0,
        pub_write::ShapeStyle::default(),
    )
    .unwrap();
    let (p, s) = find(&src, |s| s.kind == 32);
    for (h, v) in [(true, false), (false, true), (true, true), (false, false)] {
        let look = ShapeLook {
            flip_h: Some(h),
            flip_v: Some(v),
            ..Default::default()
        };
        let got = shape(&set_shape_style(&src, p, s, &look).unwrap(), p, s);
        assert_eq!((got.flip_h, got.flip_v), (h, v));
    }
}

fn rotated(src: &[u8], p: usize, s: usize, deg: f32) -> Vec<u8> {
    let look = ShapeLook {
        rotation_deg: Some(deg),
        ..Default::default()
    };
    set_shape_style(src, p, s, &look).unwrap()
}

#[kernpage_samples::test]
fn rotation_90_and_270_swap_the_stored_anchor_and_keep_the_frame() {
    let src = sample("bulletin.pub");
    let (p, s) = find(&src, |s| {
        s.rotation_deg == 0.0
            && s.group.is_none()
            && s.right_emu - s.left_emu != s.bottom_emu - s.top_emu
    });
    let before = shape(&src, p, s);
    let w = before.right_emu - before.left_emu;
    let h = before.bottom_emu - before.top_emu;
    for deg in [90.0, 270.0] {
        let out = rotated(&src, p, s, deg);
        let got = shape(&out, p, s);
        assert_eq!(got.rotation_deg, deg);
        let (a, b) = (frame(&got), frame(&before));
        for (x, y) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
            assert!((x - y).abs() <= 1, "{x} vs {y}");
        }
        // The raw anchor is stored with width and height swapped (spec 4H).
        let st = pub_format::Streams::from_cfb(&out).unwrap();
        let raw = pub_format::shape_slots(&st, got.object_number).unwrap().unwrap();
        let read = |i: usize| {
            let at = raw.anchor[i].unwrap().offset;
            i32::from_le_bytes(st.escher[at..at + 4].try_into().unwrap())
        };
        assert_eq!((read(2) - read(0), read(3) - read(1)), (h, w));
        // Back to 0 restores the original frame.
        let g = shape(&rotated(&out, p, s, 0.0), p, s);
        assert_eq!(g.rotation_deg, 0.0);
        assert_eq!(frame(&g), frame(&before));
    }
}

#[kernpage_samples::test]
fn rotate_pub_round_trips_and_30_degrees_keeps_the_anchor() {
    let src = sample("rotate.pub");
    let d = pub_format::read_bytes(&src).unwrap();
    for (p, page) in d.pages.iter().enumerate() {
        for (s, sh) in page.shapes.iter().enumerate() {
            let g = shape(&rotated(&src, p, s, sh.rotation_deg), p, s);
            assert_eq!(g.rotation_deg, sh.rotation_deg);
            assert_eq!(frame(&g), frame(sh));
        }
    }
    let (p, s) = find(&src, |s| s.rotation_deg == 90.0);
    let g = shape(&rotated(&src, p, s, 30.0), p, s);
    let o = shape(&src, p, s);
    assert_eq!(g.rotation_deg, 30.0);
    assert_eq!(frame(&g), frame(&o));
}

#[kernpage_samples::test]
fn new_shape_without_outline_reads_back_without_one() {
    let src = sample("bulletin.pub");
    let style = pub_write::ShapeStyle {
        fill: Some([9, 8, 7]),
        no_line: true,
        ..Default::default()
    };
    let out = pub_write::add_shape(&src, 0, pub_write::NewShapeKind::Rectangle, 72.0, 660.0, 100.0, 40.0, style).unwrap();
    let d = pub_format::read_bytes(&out).unwrap();
    let s = d.pages[0].shapes.last().unwrap();
    assert_eq!((s.fill, s.line), (Some(Color::Rgb(9, 8, 7)), None));
}

/// Writes the files for the manual Publisher 2021 check into `out/style-save/` (git-ignored).
#[kernpage_samples::test]
#[ignore]
fn write_publisher_check_files() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out/style-save");
    std::fs::create_dir_all(&dir).unwrap();
    let put = |name: &str, bytes: &[u8]| std::fs::write(dir.join(name), bytes).unwrap();
    let src = sample("shapes.pub");
    let (p, s) = find(&src, |s| s.fill.is_some() || s.line.is_some());
    println!("shapes.pub target page {p} shape {s}");
    let on = ShapeLook {
        fill: Some(Some([10, 20, 30])),
        line_on: Some(true),
        line_color: Some([200, 100, 50]),
        line_weight_emu: Some(38_100),
        ..Default::default()
    };
    put("st-fill.pub", &set_shape_style(&src, p, s, &on).unwrap());
    let off = ShapeLook {
        fill: Some(None),
        line_on: Some(false),
        ..Default::default()
    };
    put("st-nofill.pub", &set_shape_style(&src, p, s, &off).unwrap());
    put("st-rot90.pub", &rotated(&src, p, s, 90.0));
    put("st-rot270.pub", &rotated(&src, p, s, 270.0));
    put("st-rot30.pub", &rotated(&src, p, s, 30.0));
    let line = pub_write::add_shape(
        &sample("bulletin.pub"),
        0,
        pub_write::NewShapeKind::Line { flip_h: false, flip_v: false },
        72.0,
        680.0,
        200.0,
        40.0,
        pub_write::ShapeStyle::default(),
    )
    .unwrap();
    let (lp, ls) = find(&line, |s| s.kind == 32);
    println!("line page {lp} shape {ls}");
    for (name, h, v) in [("st-flip-h", true, false), ("st-flip-v", false, true), ("st-flip-hv", true, true)] {
        let look = ShapeLook { flip_h: Some(h), flip_v: Some(v), ..Default::default() };
        put(&format!("{name}.pub"), &set_shape_style(&line, lp, ls, &look).unwrap());
    }
    let nl = pub_write::ShapeStyle { fill: Some([9, 8, 7]), no_line: true, ..Default::default() };
    put(
        "st-new-noline.pub",
        &pub_write::add_shape(&sample("bulletin.pub"), 0, pub_write::NewShapeKind::Rectangle, 72.0, 660.0, 100.0, 40.0, nl).unwrap(),
    );
}
