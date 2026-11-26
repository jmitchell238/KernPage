//! Moving and resizing rotated shapes and children of groups (spec 4H, 4J, 4G).

use pub_format::{items, Document, Streams};
use pub_write::{set_shape_frame, WriteError};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

/// All `MCLD` frame (width, height) pairs, per story group, decoded with the public item
/// codec (spec 4G).
fn mcld(bytes: &[u8]) -> Vec<Vec<(u64, u64)>> {
    let q = Streams::from_cfb(bytes).unwrap().quill;
    let u = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize;
    let mut blk = 0x18;
    let (off, len) = 'dir: loop {
        let n = u16::from_le_bytes([q[blk + 2], q[blk + 3]]) as usize;
        for i in 0..n {
            let e = blk + 8 + 24 * i;
            if &q[e + 2..e + 6] == b"MCLD" {
                break 'dir (u(&q, e + 16), u(&q, e + 20));
            }
        }
        let next = u(&q, blk + 4);
        assert_ne!(next, 0xFFFF_FFFF, "no MCLD");
        blk = next;
    };
    let b = &q[off..off + len];
    let groups = u(b, 4);
    let mut pos = 8 + 4 * groups;
    let mut out = Vec::new();
    for _ in 0..groups {
        pos += u(b, pos);
        let cnt = u(b, pos);
        pos += 4;
        let mut g = Vec::new();
        for _ in 0..cnt {
            let l = u(b, pos);
            let it = items::decode_block(&b[..pos + l], pos).unwrap();
            g.push((
                items::find_int(&it, 0x204).unwrap(),
                items::find_int(&it, 0x205).unwrap(),
            ));
            pos += l;
        }
        out.push(g);
    }
    out
}

fn near(a: i32, b: i32) -> bool {
    (i64::from(a) - i64::from(b)).abs() <= 1
}

/// Everything except the moved shape must read back unchanged.
fn assert_others_same(a: &Document, b: &Document, page: usize, shape: usize) {
    let mut a = a.clone();
    a.pages[page].shapes[shape] = b.pages[page].shapes[shape].clone();
    assert_eq!(&a, b);
}

#[kernpage_samples::test]
fn rotated_boxes_keep_the_swapped_anchor_form() {
    let x = gen("rotate");
    let before = pub_format::read_bytes(&x).unwrap();
    let mcld_before = mcld(&x);
    for (si, s) in before.pages[0].shapes.iter().enumerate() {
        // Visual (unrotated) box: move right/down and make it a different size.
        let (l, t) = (s.left_emu + 300_001, s.top_emu + 123_457);
        let (w, h) = (
            s.right_emu - s.left_emu + 400_001,
            s.bottom_emu - s.top_emu + 99_999,
        );
        let y = set_shape_frame(&x, 0, si, l, t, w as u32, h as u32).unwrap();
        let after = pub_format::read_bytes(&y).unwrap();
        let n = &after.pages[0].shapes[si];
        assert!(
            near(n.left_emu, l) && near(n.top_emu, t),
            "{} {:?}",
            s.rotation_deg,
            (n.left_emu, n.top_emu)
        );
        assert!(near(n.right_emu, l + w) && near(n.bottom_emu, t + h));
        assert_eq!(n.rotation_deg, s.rotation_deg);
        assert_eq!(n.width_emu, Some(w as u32));
        assert_eq!(n.height_emu, Some(h as u32));
        assert_others_same(&before, &after, 0, si);
        // The stored anchor is swapped for quarter-turn rotations.
        let quarter =
            (45.0..135.0).contains(&s.rotation_deg) || (225.0..315.0).contains(&s.rotation_deg);
        let slots = pub_format::shape_slots(&Streams::from_cfb(&y).unwrap(), s.object_number)
            .unwrap()
            .unwrap();
        let st = Streams::from_cfb(&y).unwrap();
        let a: Vec<i32> = slots
            .anchor
            .iter()
            .map(|sl| {
                let o = sl.unwrap().offset;
                i32::from_le_bytes(st.escher[o..o + 4].try_into().unwrap())
            })
            .collect();
        let (aw, ah) = (a[2] - a[0], a[3] - a[1]);
        if quarter {
            assert!(near(aw, h) && near(ah, w), "{aw}x{ah} for {w}x{h}");
        } else {
            assert!(near(aw, w) && near(ah, h));
        }
        // MCLD keeps the unrotated size of this story's frame.
        if s.story.is_some() {
            let m = mcld(&y);
            assert_ne!(m, mcld_before);
            assert!(
                m.iter().flatten().any(|&f| f == (w as u64, h as u64)),
                "{m:?}"
            );
            assert_eq!(after.columns_unmapped, before.columns_unmapped);
        }
    }
}

#[kernpage_samples::test]
fn children_of_a_group_are_mapped_through_the_group() {
    let x = gen("group");
    let before = pub_format::read_bytes(&x).unwrap();
    let kids: Vec<usize> = before.pages[0]
        .shapes
        .iter()
        .enumerate()
        .filter(|(_, s)| s.group.is_some())
        .map(|(i, _)| i)
        .collect();
    assert_eq!(kids.len(), 2);
    for si in kids {
        let s = &before.pages[0].shapes[si];
        let (l, t) = (s.left_emu + 250_001, s.top_emu + 100_003);
        let (w, h) = (
            (s.right_emu - s.left_emu) as u32 / 2 + 7,
            (s.bottom_emu - s.top_emu) as u32 / 2 + 11,
        );
        let y = set_shape_frame(&x, 0, si, l, t, w, h).unwrap();
        let after = pub_format::read_bytes(&y).unwrap();
        let n = &after.pages[0].shapes[si];
        assert!(near(n.left_emu, l) && near(n.top_emu, t), "{n:?}");
        assert!(near(n.right_emu, l + w as i32) && near(n.bottom_emu, t + h as i32));
        assert_eq!(n.group, s.group);
        // The group follows its children (spec 4J) and nothing else changes.
        let g = before.pages[0]
            .shapes
            .iter()
            .position(|q| q.kind == 48)
            .unwrap();
        let ag = &after.pages[0].shapes[g];
        let kids_after: Vec<_> = after.pages[0]
            .shapes
            .iter()
            .filter(|q| q.group == Some(ag.object_number))
            .collect();
        assert_eq!(
            ag.left_emu,
            kids_after.iter().map(|q| q.left_emu).min().unwrap()
        );
        assert_eq!(
            ag.top_emu,
            kids_after.iter().map(|q| q.top_emu).min().unwrap()
        );
        assert!(near(
            ag.right_emu,
            kids_after.iter().map(|q| q.right_emu).max().unwrap()
        ));
        assert!(near(
            ag.bottom_emu,
            kids_after.iter().map(|q| q.bottom_emu).max().unwrap()
        ));
        let mut expect = before.clone();
        expect.pages[0].shapes[si] = n.clone();
        expect.pages[0].shapes[g] = ag.clone();
        assert_eq!(expect, after);
    }
}

#[kernpage_samples::test]
fn group_grows_when_a_child_leaves_its_bounds_and_children_stay_put() {
    let x = gen("group");
    let before = pub_format::read_bytes(&x).unwrap();
    let sh = &before.pages[0].shapes;
    let g = sh.iter().position(|q| q.kind == 48).unwrap();
    let kids: Vec<usize> = (0..sh.len()).filter(|&i| sh[i].group.is_some()).collect();
    let (si, other) = (kids[0], kids[1]);
    let s = &sh[si];
    // Move the first child far left/up and make it bigger: outside the old group box.
    let (l, t) = (s.left_emu - 3_000_001, s.top_emu - 1_500_003);
    let (w, h) = (4_000_000u32, 1_000_000u32);
    let y = set_shape_frame(&x, 0, si, l, t, w, h).unwrap();
    let after = pub_format::read_bytes(&y).unwrap();
    let ag = &after.pages[0].shapes[g];
    let o = &after.pages[0].shapes[other];
    // Union of the moved child and the untouched one.
    assert!(near(ag.left_emu, l) && near(ag.top_emu, t), "{ag:?}");
    assert!(near(
        ag.right_emu,
        sh[g].right_emu.max(sh[other].right_emu).max(l + w as i32)
    ));
    assert!(near(ag.bottom_emu, sh[g].bottom_emu));
    // The other child keeps its page position (within 1 EMU) and the moved one is exact.
    let b = &sh[other];
    assert!(near(o.left_emu, b.left_emu) && near(o.top_emu, b.top_emu));
    assert!(near(o.right_emu, b.right_emu) && near(o.bottom_emu, b.bottom_emu));
    let n = &after.pages[0].shapes[si];
    assert!(near(n.left_emu, l) && near(n.top_emu, t));
    assert!(near(n.right_emu, l + w as i32) && near(n.bottom_emu, t + h as i32));
    // The shape outside the group is untouched.
    let free = (0..sh.len())
        .find(|i| sh[*i].group.is_none() && *i != g)
        .unwrap();
    assert_eq!(sh[free], after.pages[0].shapes[free]);

    // Moving it back inside shrinks the group to the union again.
    let z = set_shape_frame(
        &y,
        0,
        si,
        s.left_emu,
        s.top_emu,
        (s.right_emu - s.left_emu) as u32,
        (s.bottom_emu - s.top_emu) as u32,
    )
    .unwrap();
    let back = pub_format::read_bytes(&z).unwrap();
    let bg = &back.pages[0].shapes[g];
    assert!(near(bg.left_emu, sh[g].left_emu) && near(bg.top_emu, sh[g].top_emu));
    assert!(near(bg.right_emu, sh[g].right_emu) && near(bg.bottom_emu, sh[g].bottom_emu));
}

#[kernpage_samples::test]
fn linked_frame_resize_updates_only_its_mcld_frame() {
    let x = gen("linked");
    let before = pub_format::read_bytes(&x).unwrap();
    let m0 = mcld(&x);
    // obj301 = second frame of the 3-frame chain (page 0, shape 2).
    let s = &before.pages[0].shapes[2];
    let y = set_shape_frame(&x, 0, 2, s.left_emu, s.top_emu, 2_000_000, 1_000_000).unwrap();
    let mut m1 = mcld(&y);
    assert_eq!(m1[0][1], (2_000_000, 1_000_000));
    m1[0][1] = m0[0][1];
    assert_eq!(m0, m1);
    let after = pub_format::read_bytes(&y).unwrap();
    assert_eq!(after.columns_unmapped, before.columns_unmapped);
    assert_eq!(after.columns_mapped, before.columns_mapped);
}

#[kernpage_samples::test]
fn errors_stay_errors() {
    let x = gen("rotate");
    assert!(matches!(
        set_shape_frame(&x, 0, 0, i32::MAX, 0, 10, 10),
        Err(WriteError::OutOfRange(_))
    ));
}
