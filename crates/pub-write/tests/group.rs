//! Group / Ungroup (spec 4J-b): round trips on our samples and, when the git-ignored
//! Publisher-made evidence files are present, a structural comparison with them.

use pub_format::{contents::Contents, items, Streams};
use pub_write::{group_shapes, ungroup, ungroup_with};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap()
}
fn evidence(name: &str) -> Option<Vec<u8>> {
    std::fs::read(root().join(format!("out/group-research/{name}.pub"))).ok()
}

fn rects(bytes: &[u8], page: usize) -> Vec<(u32, [i32; 4], Option<u32>)> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages[page]
        .shapes
        .iter()
        .map(|s| {
            (
                s.object_number,
                [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu],
                s.group,
            )
        })
        .collect()
}

fn near(a: &[i32; 4], b: &[i32; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1)
}

/// Per-object (kind, parent, block bytes with the document save counter zeroed).
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
    if let Ok(d) = std::env::var("GROUP_DUMP") {
        std::fs::write(d, ours).unwrap();
    }
    let a = Streams::from_cfb(ours).unwrap();
    let b = Streams::from_cfb(theirs).unwrap();
    // Bytes 106-107 are the BSE record's two unused bytes; Publisher writes stale memory.
    let mask = |mut e: Vec<u8>| {
        e[106..108].fill(0);
        e
    };
    assert_eq!(
        mask(a.escher),
        mask(b.escher),
        "EscherStm differs from Publisher's"
    );
    assert_eq!(a.delay, b.delay);
    let (va, vb) = (contents_view(ours), contents_view(theirs));
    assert_eq!(va.len(), vb.len(), "object count");
    for (i, (x, y)) in va.iter().zip(&vb).enumerate() {
        assert_eq!((x.0, x.1), (y.0, y.1), "object #{i} kind/parent");
        assert_eq!(x.2, y.2, "object #{i} (kind {}) block", x.0);
    }
}

#[kernpage_samples::test]
fn bulletin_group_of_two_text_boxes_round_trips() {
    let bytes = gen("bulletin");
    let before = rects(&bytes, 0);
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let tbs: Vec<usize> = doc.pages[0]
        .shapes
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == 1)
        .map(|(i, _)| i)
        .collect();
    assert!(tbs.len() >= 3);
    // Publisher's b_tb_tb grouped the first and the last of the three text boxes.
    let tbs = [tbs[0], tbs[2], tbs[1]];
    let out = group_shapes(&bytes, 0, &tbs[..2]).unwrap();
    let after = rects(&out, 0);
    assert_eq!(after.len(), before.len() + 1);
    let g = after
        .iter()
        .find(|(o, _, _)| !before.iter().any(|b| b.0 == *o))
        .unwrap();
    for &i in &tbs[..2] {
        let (o, r, _) = &before[i];
        let n = after.iter().find(|a| a.0 == *o).unwrap();
        assert!(near(r, &n.1), "child moved");
        assert_eq!(n.2, Some(g.0));
    }
    for (o, r, p) in &before {
        if !tbs[..2].iter().any(|&i| before[i].0 == *o) {
            let n = after.iter().find(|a| a.0 == *o).unwrap();
            assert!(near(r, &n.1) && n.2 == *p);
        }
    }
    if let Some(theirs) = evidence("b_tb_tb") {
        same_as_publisher(&out, &theirs);
    }
    // And back.
    let gi = pub_format::read_bytes(&out).unwrap().pages[0]
        .shapes
        .iter()
        .position(|s| s.object_number == g.0)
        .unwrap();
    let back = ungroup(&out, 0, gi).unwrap();
    let r = rects(&back, 0);
    assert_eq!(r.len(), before.len());
    for (o, rc, p) in &before {
        let n = r.iter().find(|a| a.0 == *o).unwrap();
        assert!(near(rc, &n.1) && n.2 == *p, "object {o}");
    }
}

#[kernpage_samples::test]
fn bulletin_text_box_and_picture() {
    let bytes = gen("bulletin");
    let doc = pub_format::read_bytes(&bytes).unwrap();
    let s = &doc.pages[0].shapes;
    let tb = s.iter().position(|x| x.kind == 1).unwrap();
    let pic = s.iter().position(|x| x.picture.is_some()).unwrap();
    let out = group_shapes(&bytes, 0, &[tb, pic]).unwrap();
    if let Some(theirs) = evidence("b_pic") {
        same_as_publisher(&out, &theirs);
    }
    assert_eq!(rects(&out, 0).len(), s.len() + 1);
    assert!(group_shapes(&bytes, 0, &[tb]).is_err());
    assert!(group_shapes(&bytes, 0, &[tb, tb]).is_err());
}

#[kernpage_samples::test]
fn group_pub_ungroup_matches_publisher_and_regroups() {
    let bytes = gen("group");
    let before = rects(&bytes, 0);
    let gi = before
        .iter()
        .position(|b| before.iter().any(|c| c.2 == Some(b.0)))
        .unwrap();
    let out = ungroup(&bytes, 0, gi).unwrap();
    if let Some(theirs) = evidence("g_ung") {
        same_as_publisher(&out, &theirs);
    }
    let after = rects(&out, 0);
    assert_eq!(after.len(), before.len() - 1);
    for (o, r, p) in &before {
        if let Some(n) = after.iter().find(|a| a.0 == *o) {
            if *p.as_ref().unwrap_or(&0) != 0 {
                assert!(near(r, &n.1), "child {o}");
                assert_eq!(n.2, None);
            }
        }
    }
    // Without Publisher's extras the file still reads the same.
    let lean = ungroup_with(&bytes, 0, gi, false).unwrap();
    assert_eq!(rects(&lean, 0), after);
    assert!(lean.len() <= out.len() + 4096);
    // Regroup the children.
    let kids: Vec<usize> = before
        .iter()
        .filter(|b| b.2.is_some())
        .map(|b| after.iter().position(|a| a.0 == b.0).unwrap())
        .collect();
    let again = group_shapes(&out, 0, &kids).unwrap();
    let r = rects(&again, 0);
    assert_eq!(r.len(), before.len());
    for (o, rc, _) in &before {
        if let Some(n) = r.iter().find(|a| a.0 == *o) {
            assert!(near(rc, &n.1), "regrouped {o}");
        }
    }
}

#[kernpage_samples::test]
fn nested_and_rotated_are_refused() {
    if let Some(nest) = evidence("g_nest") {
        let r = rects(&nest, 0);
        let outer = r
            .iter()
            .position(|b| r.iter().any(|c| c.2 == Some(b.0)) && b.2.is_none())
            .unwrap();
        assert!(ungroup(&nest, 0, outer).is_err());
        let all: Vec<usize> = (0..r.len()).collect();
        assert!(group_shapes(&nest, 0, &all).is_err());
    }
}

#[kernpage_samples::test]
fn bulletin_group_of_three_matches_publisher() {
    let bytes = gen("bulletin");
    let n = pub_format::read_bytes(&bytes).unwrap().pages[0]
        .shapes
        .len();
    let all: Vec<usize> = (0..n).collect();
    let out = group_shapes(&bytes, 0, &all).unwrap();
    if let Some(theirs) = evidence("b_3") {
        same_as_publisher(&out, &theirs);
    }
}

/// Writes the samples that are opened in Publisher by hand (`out/save-test10`):
/// `cargo test -p pub-write --test group -- --ignored`.
#[kernpage_samples::test]
#[ignore]
fn write_publisher_samples() {
    let dir = root().join("out/save-test10");
    std::fs::create_dir_all(&dir).unwrap();
    let bulletin = gen("bulletin");
    let doc = pub_format::read_bytes(&bulletin).unwrap();
    let s = &doc.pages[0].shapes;
    let pic = s.iter().position(|x| x.picture.is_some()).unwrap();
    let tbs: Vec<usize> = (0..s.len()).filter(|&i| s[i].kind == 1).collect();
    let w = |name: &str, b: Vec<u8>| std::fs::write(dir.join(name), b).unwrap();
    w(
        "g_tb_tb.pub",
        group_shapes(&bulletin, 0, &[tbs[0], tbs[2]]).unwrap(),
    );
    w(
        "g_tb_pic.pub",
        group_shapes(&bulletin, 0, &[tbs[0], pic]).unwrap(),
    );
    let group = gen("group");
    let r = rects(&group, 0);
    let gi = r
        .iter()
        .position(|b| r.iter().any(|c| c.2 == Some(b.0)))
        .unwrap();
    w("u_full.pub", ungroup(&group, 0, gi).unwrap());
    w("u_lean.pub", ungroup_with(&group, 0, gi, false).unwrap());
}

#[kernpage_samples::test]
#[ignore]
fn write_gap_sample() {
    let dir = root().join("out/save-test10");
    // Find a sample with two pages that each hold 2+ top-level shapes.
    for name in kernpage_samples::files("gen", "pub") {
        let b = std::fs::read(&name).unwrap();
        let Ok(d) = pub_format::read_bytes(&b) else {
            continue;
        };
        let tops: Vec<Vec<usize>> = d
            .pages
            .iter()
            .map(|p| {
                (0..p.shapes.len())
                    .filter(|&i| p.shapes[i].group.is_none())
                    .collect()
            })
            .collect();
        println!(
            "{:?} {:?}",
            name.file_name(),
            tops.iter().map(Vec::len).collect::<Vec<_>>()
        );
        if tops.len() >= 2 && tops[0].len() >= 2 && tops[1].len() >= 2 {
            let a = group_shapes(&b, 0, &tops[0][..2]).unwrap();
            let c = group_shapes(&a, 1, &tops[1][..2]).unwrap();
            let r = rects(&c, 0);
            let gi = r
                .iter()
                .position(|x| r.iter().any(|y| y.2 == Some(x.0)))
                .unwrap();
            std::fs::write(dir.join("gap_both.pub"), &c).unwrap();
            std::fs::write(
                dir.join("gap_ungrouped_first.pub"),
                ungroup(&c, 0, gi).unwrap(),
            )
            .unwrap();
            return;
        }
    }
    panic!("no sample");
}
