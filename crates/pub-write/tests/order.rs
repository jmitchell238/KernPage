//! Stacking order: shape containers are permuted inside `EscherStm` (Publisher 2010 only
//! permutes them too; checked with `Shape.ZOrder` in out/save-test9).

use pub_write::set_shape_order;
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

fn objects(bytes: &[u8], page: usize) -> Vec<u32> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages[page]
        .shapes
        .iter()
        .map(|s| s.object_number)
        .collect()
}

#[kernpage_samples::test]
fn bulletin_reordered_shapes_read_back_in_the_new_order() {
    let bytes = gen("bulletin");
    let before = objects(&bytes, 0);
    assert!(before.len() >= 3);
    // Move the first shape to the front, the last to the back.
    let n = before.len();
    let mut order: Vec<usize> = (1..n - 1).collect();
    order.push(0);
    order.insert(0, n - 1);
    let out = set_shape_order(&bytes, 0, &order).unwrap();
    let want: Vec<u32> = order.iter().map(|&i| before[i]).collect();
    assert_eq!(objects(&out, 0), want);
    // Identity is a no-op on the shape data.
    let same: Vec<usize> = (0..n).collect();
    let out2 = set_shape_order(&bytes, 0, &same).unwrap();
    assert_eq!(objects(&out2, 0), before);
    // Everything else about the shapes survives.
    let a = pub_format::read_bytes(&bytes).unwrap();
    let b = pub_format::read_bytes(&out).unwrap();
    for s in &a.pages[0].shapes {
        let t = b.pages[0]
            .shapes
            .iter()
            .find(|t| t.object_number == s.object_number)
            .unwrap();
        assert_eq!(
            (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu, &s.text),
            (t.left_emu, t.top_emu, t.right_emu, t.bottom_emu, &t.text)
        );
    }
}

#[kernpage_samples::test]
fn bad_orders_are_rejected() {
    let bytes = gen("bulletin");
    assert!(set_shape_order(&bytes, 0, &[0]).is_err());
    assert!(set_shape_order(&bytes, 0, &[0, 0, 1]).is_err());
    assert!(set_shape_order(&bytes, 9, &[]).is_err());
}

#[kernpage_samples::test]
fn group_children_stay_inside_their_group() {
    let bytes = gen("group");
    let a = pub_format::read_bytes(&bytes).unwrap();
    let n = a.pages[0].shapes.len();
    let order: Vec<usize> = (0..n).rev().collect();
    let out = set_shape_order(&bytes, 0, &order).unwrap();
    let b = pub_format::read_bytes(&out).unwrap();
    assert_eq!(b.pages[0].shapes.len(), n);
    for s in &a.pages[0].shapes {
        let t = b.pages[0]
            .shapes
            .iter()
            .find(|t| t.object_number == s.object_number)
            .unwrap();
        assert_eq!(s.group, t.group);
        assert_eq!(
            (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu),
            (t.left_emu, t.top_emu, t.right_emu, t.bottom_emu)
        );
    }
}

/// Writes out/save-test9/ours-*.pub for checking with Publisher (`Shape.ZOrderPosition`).
#[kernpage_samples::test]
#[ignore = "writes files for manual Publisher verification"]
fn write_publisher_samples() {
    let bytes = gen("bulletin");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test9");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("ours-front0.pub"),
        set_shape_order(&bytes, 0, &[1, 2, 0]).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("ours-back-last.pub"),
        set_shape_order(&bytes, 0, &[2, 0, 1]).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("ours-reverse.pub"),
        set_shape_order(&bytes, 0, &[2, 1, 0]).unwrap(),
    )
    .unwrap();
}
