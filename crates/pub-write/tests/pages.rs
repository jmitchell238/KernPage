//! Add, move and delete pages (spec 4P).

use pub_write::{add_page, delete_page, duplicate_page, move_page, WriteError};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap()
}
/// Publisher 2021's own output from the (git-ignored) page research, when present.
fn research(name: &str) -> Option<Vec<u8>> {
    std::fs::read(root().join(format!("out/page-research/{name}.pub"))).ok()
}

/// Per page: the text of every shape, in order.
fn texts(bytes: &[u8]) -> Vec<Vec<String>> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages
        .iter()
        .map(|p| p.shapes.iter().map(|s| s.text.clone().unwrap_or_default()).collect())
        .collect()
}
fn frames(bytes: &[u8]) -> Vec<Vec<(u32, [i32; 4])>> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages
        .iter()
        .map(|p| {
            p.shapes
                .iter()
                .map(|s| (s.kind as u32, [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu]))
                .collect()
        })
        .collect()
}

/// Object numbers whose Contents block differs between two files (kind, number).
fn block_diffs(a: &[u8], b: &[u8]) -> Vec<(u32, u16)> {
    let sa = pub_format::Streams::from_cfb(a).unwrap();
    let sb = pub_format::Streams::from_cfb(b).unwrap();
    let ca = pub_format::contents::Contents::parse(&sa.contents).unwrap();
    let cb = pub_format::contents::Contents::parse(&sb.contents).unwrap();
    let blk = |s: &[u8], off: usize| {
        let n = u32::from_le_bytes(s[off..off + 4].try_into().unwrap()) as usize;
        s[off..off + n].to_vec()
    };
    let mut out = Vec::new();
    let n = ca.objects.iter().map(|o| o.number).chain(cb.objects.iter().map(|o| o.number)).max().unwrap();
    for i in 0..=n {
        match (ca.get(i), cb.get(i)) {
            (Some(x), Some(y)) => {
                if x.kind != y.kind || x.parent != y.parent || blk(&sa.contents, x.offset) != blk(&sb.contents, y.offset) {
                    out.push((i, x.kind));
                }
            }
            (None, None) => {}
            (x, y) => out.push((i, x.or(y).unwrap().kind)),
        }
    }
    out
}

/// The Contents items of every shape of page `p` (z-order) as text, without the items that
/// legitimately differ: story number (39), picture child reference (58), per-save noise
/// (4 = wrap flags that Publisher 2021 rewrites on save, 43, 44).
fn shape_items(b: &[u8], p: usize) -> Vec<Vec<String>> {
    let doc = pub_format::read_bytes(b).unwrap();
    let st = pub_format::Streams::from_cfb(b).unwrap();
    let c = pub_format::contents::Contents::parse(&st.contents).unwrap();
    doc.pages[p]
        .shapes
        .iter()
        .map(|s| {
            let o = c.get(s.object_number).unwrap();
            pub_format::items::decode_block(&st.contents, o.offset)
                .unwrap()
                .iter()
                .filter(|i| !matches!(i.id, 4 | 39 | 58 | 43 | 44))
                .map(|i| format!("{}={:?}", i.id, i.value))
                .collect()
        })
        .collect()
}

fn base2021() -> Option<Vec<u8>> {
    research("base2021")
}

/// Add-page cases against Publisher 2021's own output (skipped when out/page-research is
/// absent): the page, kind 96 and every shape object are byte-identical, and the reader
/// sees the same pages. Differences left: object 256 (save counter, bumped by the save
/// planner), per-save noise in shapes, the kind 119 first-page variant, the font table.
#[kernpage_samples::test]
fn add_matches_publisher_2021() {
    let Some(base) = base2021() else { return };
    for (name, after) in [("a_after2", Some(1usize)), ("a_after1", Some(0)), ("a_first", None)] {
        let theirs = research(name).unwrap();
        let ours = add_page(&base, after).unwrap();
        let diffs = block_diffs(&ours, &theirs);
        assert!(
            diffs.iter().all(|d| matches!(d.1, 68 | 1 | 108)),
            "{name}: {diffs:?}"
        );
        assert!(!diffs.iter().any(|d| d.1 == 67 || d.1 == 96), "{name}: {diffs:?}");
        assert_eq!(texts(&ours), texts(&theirs), "{name}");
        assert_eq!(frames(&ours), frames(&theirs), "{name}");
    }
    let ours = add_page(&add_page(&base, Some(1)).unwrap(), Some(2)).unwrap();
    let theirs = research("a_two").unwrap();
    assert_eq!(texts(&ours), texts(&theirs));
}

#[kernpage_samples::test]
fn delete_matches_publisher_2021() {
    let Some(base) = base2021() else { return };
    for (name, idx) in [("x_del2", 1usize), ("x_del1", 0)] {
        let theirs = research(name).unwrap();
        let ours = delete_page(&base, idx).unwrap();
        assert_eq!(texts(&ours), texts(&theirs), "{name}");
        assert_eq!(frames(&ours), frames(&theirs), "{name}");
        let sa = pub_format::Streams::from_cfb(&ours).unwrap();
        let sb = pub_format::Streams::from_cfb(&theirs).unwrap();
        if idx == 1 {
            // No picture on page 2: Escher equals Publisher's except its byte 0x6A.
            assert_eq!(sa.escher.len(), sb.escher.len());
            let d: Vec<_> = (0..sa.escher.len()).filter(|&i| sa.escher[i] != sb.escher[i]).collect();
            assert!(d.iter().all(|&i| i == 106 || i == 107), "{d:?}");
        }
    }
}

#[kernpage_samples::test]
fn move_matches_publisher_2021() {
    let Some(base) = base2021() else { return };
    let ours = move_page(&base, 1, 0).unwrap();
    assert_eq!(texts(&ours), texts(&research("m_2to1").unwrap()));
    // Only object 256 changes (item 2 and nothing else).
    assert_eq!(block_diffs(&ours, &base), vec![(256, 68)]);
    let back = move_page(&ours, 0, 1).unwrap();
    assert_eq!(texts(&back), texts(&base));
}

#[kernpage_samples::test]
fn page_order_is_read_from_the_page_list() {
    // The moved page keeps its item 6, so only the list can give the order.
    let base = gen("bulletin");
    let moved = move_page(&base, 1, 0).unwrap();
    let t = texts(&moved);
    assert_eq!(t[0], texts(&base)[1]);
    assert_eq!(t[1], texts(&base)[0]);
    if let Some(theirs) = research("m_2to1") {
        assert_eq!(texts(&theirs), t);
    }
}

#[kernpage_samples::test]
fn add_after_last_first_middle_and_twice() {
    let base = gen("bulletin");
    let orig = texts(&base);
    let last = texts(&add_page(&base, Some(1)).unwrap());
    assert_eq!(last, vec![orig[0].clone(), orig[1].clone(), vec![]]);
    let first = texts(&add_page(&base, None).unwrap());
    assert_eq!(first, vec![vec![], orig[0].clone(), orig[1].clone()]);
    let mid = texts(&add_page(&base, Some(0)).unwrap());
    assert_eq!(mid, vec![orig[0].clone(), vec![], orig[1].clone()]);
    let twice = add_page(&add_page(&base, Some(0)).unwrap(), Some(0)).unwrap();
    assert_eq!(
        texts(&twice),
        vec![orig[0].clone(), vec![], vec![], orig[1].clone()]
    );
    assert!(add_page(&base, Some(2)).is_err());
    // One-page file.
    let ab = gen("apple-banana");
    let two = add_page(&ab, Some(0)).unwrap();
    assert_eq!(texts(&two).len(), 2);
    assert_eq!(texts(&two)[0], texts(&ab)[0]);
    // The file survives another round trip.
    pub_write::rewrite(&two).unwrap();
}

#[kernpage_samples::test]
fn delete_pages_and_refuse_the_last() {
    let base = gen("bulletin");
    let orig = texts(&base);
    let fr = frames(&base);
    // Page 1 holds the picture.
    let no_first = delete_page(&base, 0).unwrap();
    assert_eq!(texts(&no_first), vec![orig[1].clone()]);
    assert_eq!(frames(&no_first), vec![fr[1].clone()]);
    let no_second = delete_page(&base, 1).unwrap();
    assert_eq!(texts(&no_second), vec![orig[0].clone()]);
    assert_eq!(frames(&no_second), vec![fr[0].clone()]);
    // Middle page of three.
    let three = add_page(&base, Some(0)).unwrap();
    let mid = delete_page(&three, 1).unwrap();
    assert_eq!(texts(&mid), orig);
    assert_eq!(frames(&mid), fr);
    // The only page cannot be deleted.
    assert!(delete_page(&no_first, 0).is_err());
    assert!(delete_page(&base, 2).is_err());
    // Delete, then add, then add a text box on the new page's neighbour.
    let again = add_page(&no_first, Some(0)).unwrap();
    assert_eq!(texts(&again), vec![orig[1].clone(), vec![]]);
    let boxed = pub_write::add_text_box(&again, 0, 72.0, 72.0, 200.0, 50.0, "New").unwrap();
    assert_eq!(texts(&boxed)[0].last().map(String::as_str), Some("New\r"));
    assert_eq!(texts(&boxed)[1], Vec::<String>::new());
}

#[kernpage_samples::test]
fn delete_picture_page_lowers_blip_reference() {
    let base = gen("bulletin");
    let refs = |b: &[u8]| {
        let st = pub_format::Streams::from_cfb(b).unwrap();
        let mut out = Vec::new();
        bse_refs(&st.escher, 0, st.escher.len(), &mut out);
        out
    };
    let before = refs(&base);
    assert_eq!(before.len(), 1);
    let out = delete_page(&base, 0).unwrap();
    assert_eq!(refs(&out), vec![before[0] - 1]);
    assert!(pub_format::read_bytes(&out).unwrap().pages[0]
        .shapes
        .iter()
        .all(|s| s.picture.is_none()));
}

/// `cRef` of every blip store entry (0xF007) of the drawing group.
fn bse_refs(d: &[u8], mut pos: usize, end: usize, out: &mut Vec<u32>) {
    while pos + 8 <= end {
        let ty = u16::from_le_bytes([d[pos + 2], d[pos + 3]]);
        let len = u32::from_le_bytes(d[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if d[pos] & 0xF == 0xF {
            bse_refs(d, pos + 8, pos + 8 + len, out);
        } else if ty == 0xF007 {
            out.push(u32::from_le_bytes(d[pos + 8 + 24..pos + 8 + 28].try_into().unwrap()));
        }
        pos += 8 + len;
        if ty == 0xF002 {
            break; // the drawing group ends before the first drawing
        }
    }
}

fn refs_of(b: &[u8]) -> Vec<u32> {
    let st = pub_format::Streams::from_cfb(b).unwrap();
    let mut out = Vec::new();
    bse_refs(&st.escher, 0, st.escher.len(), &mut out);
    out
}

type ShapeSummary = (u16, [i32; 4], String, Vec<(String, bool, bool, f32)>);

/// Kind, frame, text and the formatting runs of every shape of page `i`.
fn page_summary(b: &[u8], i: usize) -> Vec<ShapeSummary> {
    let doc = pub_format::read_bytes(b).unwrap();
    doc.pages[i]
        .shapes
        .iter()
        .map(|s| {
            (
                s.kind,
                [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu],
                s.text.clone().unwrap_or_default(),
                s.runs
                    .iter()
                    .map(|r| (format!("{r:?}"), s.picture.is_some(), false, 0.0))
                    .collect(),
            )
        })
        .collect()
}

#[kernpage_samples::test]
fn duplicate_pages_of_the_bulletin() {
    let b = gen("bulletin");
    let before = pub_format::read_bytes(&b).unwrap();
    for src in [0usize, 1] {
        let out = duplicate_page(&b, src).unwrap();
        let doc = pub_format::read_bytes(&out).unwrap();
        assert_eq!(doc.pages.len(), 3);
        // The copy sits right after the source; the other page keeps its place.
        assert_eq!(page_summary(&out, src), page_summary(&out, src + 1));
        assert_eq!(page_summary(&out, src), page_summary(&b, src));
        let other = if src == 0 { 2 } else { 0 };
        assert_eq!(page_summary(&out, other), page_summary(&b, 1 - src));
        // New objects, new stories (no story number is shared).
        let shapes = &doc.pages[src].shapes;
        let copy = &doc.pages[src + 1].shapes;
        for (a, c) in shapes.iter().zip(copy) {
            assert_ne!(a.object_number, c.object_number);
            assert_eq!(a.story.is_some(), c.story.is_some());
            assert_eq!(a.picture.is_some(), c.picture.is_some());
            if let (Some(x), Some(y)) = (a.chain_story, c.chain_story) {
                assert_ne!(x, y);
            }
        }
        // A picture clone bumps the blip reference count.
        let pics = shapes.iter().filter(|s| s.picture.is_some()).count() as u32;
        let (r0, r1) = (refs_of(&b), refs_of(&out));
        assert_eq!(r1.iter().sum::<u32>(), r0.iter().sum::<u32>() + pics);
        let _ = &before;
        // Saving the copy again (a second duplicate, of the copy) still works.
        let again = duplicate_page(&out, src + 1).unwrap();
        assert_eq!(pub_format::read_bytes(&again).unwrap().pages.len(), 4);
        assert_eq!(page_summary(&again, src + 2), page_summary(&b, src));
    }
}

#[kernpage_samples::test]
fn duplicate_keeps_full_formatting_and_text_edits_stay_separate() {
    let b = gen("formatting");
    let out = duplicate_page(&b, 0).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let (a, c) = (&doc.pages[0], &doc.pages[1]);
    assert_eq!(a.shapes.len(), c.shapes.len());
    let mut formatted = 0;
    for (x, y) in a.shapes.iter().zip(&c.shapes) {
        assert_eq!(x.text, y.text);
        assert_eq!(format!("{:?}", x.runs), format!("{:?}", y.runs));
        assert_eq!(format!("{:?}", x.paragraphs), format!("{:?}", y.paragraphs));
        formatted += usize::from(x.runs.len() > 1);
    }
    assert!(formatted > 0, "the sample has a multi-run story");
    // Editing the copy's text leaves the source alone.
    let numbers = doc.story_numbers();
    let si = numbers
        .iter()
        .position(|&n| Some(n) == c.shapes[0].story)
        .unwrap();
    let edited = pub_write::set_story_text(&out, si, "Changed on the copy").unwrap();
    let d2 = pub_format::read_bytes(&edited).unwrap();
    assert_eq!(d2.pages[1].shapes[0].text.as_deref().map(str::trim), Some("Changed on the copy"));
    assert_eq!(d2.pages[0].shapes[0].text, a.shapes[0].text);
}

#[kernpage_samples::test]
fn duplicate_refuses_groups_tables_and_linked_boxes() {
    for name in ["group", "table", "table2", "linked"] {
        let Some(b) = kernpage_samples::read_opt(&format!("gen/{name}.pub")) else {
            continue;
        };
        let doc = pub_format::read_bytes(&b).unwrap();
        for p in 0..doc.pages.len() {
            let r = duplicate_page(&b, p);
            assert!(
                matches!(r, Err(WriteError::Unsupported(_))),
                "{name} page {p}: {:?}",
                r.map(|v| v.len())
            );
        }
    }
    assert!(matches!(
        duplicate_page(&gen("bulletin"), 5),
        Err(WriteError::NoSuchPage(5))
    ));
}

/// The copy's structure against Publisher 2021's own `Page.Duplicate` output (skipped
/// when out/page-research is absent): same pages, shapes, text and frames; the Contents
/// objects that differ are only the ones Publisher renumbers (story numbers, ids, save
/// noise).
#[kernpage_samples::test]
fn duplicate_matches_publisher_2021() {
    let Some(base) = base2021() else { return };
    for (name, src) in [("d_dup1", 0usize), ("d_dup2", 1)] {
        let theirs = research(name).unwrap();
        let ours = duplicate_page(&base, src).unwrap();
        assert_eq!(texts(&ours), texts(&theirs), "{name}");
        assert_eq!(frames(&ours), frames(&theirs), "{name}");
        assert_eq!(refs_of(&ours), refs_of(&theirs), "{name}");
        let d = |b: &[u8]| {
            let doc = pub_format::read_bytes(b).unwrap();
            doc.pages
                .iter()
                .map(|p| p.shapes.iter().map(|s| (s.runs.len(), s.paragraphs.len())).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        assert_eq!(d(&ours), d(&theirs), "{name}");
        let (so, st) = (shape_items(&ours, src + 1), shape_items(&theirs, src + 1));
        assert_eq!(so.len(), st.len(), "{name}");
        for (k, (a, t)) in so.iter().zip(&st).enumerate() {
            assert_eq!(a, t, "{name} shape {k}");
        }
    }
}

/// Writes the page-operation files that are checked in real Publisher 2021 by hand
/// (git-ignored out/page-write/); run with `--ignored`.
#[kernpage_samples::test]
#[ignore = "writes files for a manual Publisher check"]
fn write_page_samples_for_publisher() {
    let dir = root().join("out/page-write");
    std::fs::create_dir_all(&dir).unwrap();
    let b = gen("bulletin");
    let ab = gen("apple-banana");
    let w = |name: &str, bytes: Vec<u8>| std::fs::write(dir.join(format!("{name}.pub")), bytes).unwrap();
    w("w_add_last", add_page(&b, Some(1)).unwrap());
    w("w_add_first", add_page(&b, None).unwrap());
    w("w_add_mid", add_page(&b, Some(0)).unwrap());
    w("w_add_two", add_page(&add_page(&b, Some(0)).unwrap(), Some(0)).unwrap());
    w("w_ab_add", add_page(&ab, Some(0)).unwrap());
    w("w_move", move_page(&b, 1, 0).unwrap());
    w("w_del_pic", delete_page(&b, 0).unwrap());
    w("w_del_p2", delete_page(&b, 1).unwrap());
    let three = add_page(&b, Some(0)).unwrap();
    w("w_del_mid", delete_page(&three, 1).unwrap());
    w("w_move3", move_page(&three, 0, 2).unwrap());
    let del_add = add_page(&delete_page(&b, 0).unwrap(), Some(0)).unwrap();
    w("w_del_add", del_add);
    let boxed = pub_write::add_text_box(&three, 1, 72.0, 72.0, 200.0, 40.0, "New page box").unwrap();
    w("w_newpage_box", boxed);
    let both = pub_write::add_shape(
        &pub_write::add_text_box(&three, 1, 72.0, 72.0, 200.0, 40.0, "On page two").unwrap(),
        1,
        pub_write::NewShapeKind::Rectangle,
        100.0,
        200.0,
        150.0,
        60.0,
        pub_write::ShapeStyle::default(),
    )
    .unwrap();
    w("w_newpage_shape", both);
}

#[kernpage_samples::test]
#[ignore = "writes files for a manual Publisher check"]
fn write_base2021_samples_for_publisher() {
    let Some(base) = base2021() else { return };
    let dir = root().join("out/page-write");
    let w = |name: &str, bytes: Vec<u8>| std::fs::write(dir.join(format!("{name}.pub")), bytes).unwrap();
    w("v_add_last", add_page(&base, Some(1)).unwrap());
    w("v_del_p2", delete_page(&base, 1).unwrap());
    w("v_move", move_page(&base, 1, 0).unwrap());
}

/// Writes duplicate-page files that are checked in real Publisher 2021 by hand
/// (git-ignored out/page-dup/); run with `--ignored`.
#[kernpage_samples::test]
#[ignore = "writes files for a manual Publisher check"]
fn write_duplicate_samples_for_publisher() {
    let dir = root().join("out/page-dup");
    std::fs::create_dir_all(&dir).unwrap();
    let w = |name: &str, bytes: Vec<u8>| std::fs::write(dir.join(format!("{name}.pub")), bytes).unwrap();
    let b = gen("bulletin");
    w("w_dup1", duplicate_page(&b, 0).unwrap());
    w("w_dup2", duplicate_page(&b, 1).unwrap());
    w("w_dup1_twice", duplicate_page(&duplicate_page(&b, 0).unwrap(), 1).unwrap());
    w("w_dup_fmt", duplicate_page(&gen("formatting"), 0).unwrap());
    if let Some(base) = base2021() {
        w("b_dup1", duplicate_page(&base, 0).unwrap());
        w("b_dup2", duplicate_page(&base, 1).unwrap());
    }
}
