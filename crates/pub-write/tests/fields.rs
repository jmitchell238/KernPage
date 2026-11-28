//! Page-number fields written into `.pub` files (spec 4T.3): the `TOKN` field table of any
//! number of fields, one per story that has fields, compared with Publisher 2021's own files
//! (research 2026-11-28, `samples/gen/master-pn-*`).

use pub_write::{add_text_box, set_story_text_with_fields};

fn master_a() -> usize {
    pub_format::master_surface(0)
}

fn blank() -> Vec<u8> {
    kernpage_samples::read("blank.pub")
}

/// Every Quill chunk as (tag, instance, bytes), in directory order.
fn chunks(file: &[u8]) -> Vec<(String, u16, Vec<u8>)> {
    let q = pub_format::Streams::from_cfb(file).unwrap().quill;
    let total = u16::from_le_bytes([q[0x0C], q[0x0D]]) as usize;
    let mut out = Vec::new();
    let mut block = 0x18usize;
    loop {
        let n = u16::from_le_bytes([q[block + 2], q[block + 3]]) as usize;
        let next = u32::from_le_bytes(q[block + 4..block + 8].try_into().unwrap());
        for i in 0..n {
            let e = &q[block + 8 + 24 * i..block + 32 + 24 * i];
            let off = u32::from_le_bytes(e[16..20].try_into().unwrap()) as usize;
            let len = u32::from_le_bytes(e[20..24].try_into().unwrap()) as usize;
            out.push((
                String::from_utf8_lossy(&e[2..6]).into_owned(),
                u16::from_le_bytes([e[6], e[7]]),
                q[off..off + len].to_vec(),
            ));
        }
        if next == u32::MAX {
            break;
        }
        block = next as usize;
    }
    assert_eq!(out.len(), total);
    out
}

/// (tag, instance, offset, length) of every chunk.
fn layout(file: &[u8]) -> Vec<(String, u16, usize, usize)> {
    let q = pub_format::Streams::from_cfb(file).unwrap().quill;
    let mut out = Vec::new();
    let mut block = 0x18usize;
    loop {
        let n = u16::from_le_bytes([q[block + 2], q[block + 3]]) as usize;
        let next = u32::from_le_bytes(q[block + 4..block + 8].try_into().unwrap());
        for i in 0..n {
            let e = &q[block + 8 + 24 * i..block + 32 + 24 * i];
            out.push((
                String::from_utf8_lossy(&e[2..6]).into_owned(),
                u16::from_le_bytes([e[6], e[7]]),
                u32::from_le_bytes(e[16..20].try_into().unwrap()) as usize,
                u32::from_le_bytes(e[20..24].try_into().unwrap()) as usize,
            ));
        }
        if next == u32::MAX {
            break;
        }
        block = next as usize;
    }
    out
}

/// Publisher leaves 20 bytes behind every `TOKN` chunk; Publisher refuses a file without them.
fn assert_gaps(file: &[u8], what: &str) {
    let l = layout(file);
    let mut seen = 0;
    for (i, c) in l.iter().enumerate().filter(|(_, c)| c.0 == "TOKN") {
        assert_eq!(l[i + 1].2, c.2 + c.3 + 20, "{what}: gap behind TOKN {}", c.1);
        seen += 1;
    }
    assert!(seen > 0, "{what}: no TOKN");
}

fn chunk(file: &[u8], tag: &str, instance: u16) -> Option<Vec<u8>> {
    chunks(file)
        .into_iter()
        .find(|c| c.0 == tag && c.1 == instance)
        .map(|c| c.2)
}

fn directory(file: &[u8]) -> Vec<(String, u16)> {
    chunks(file).into_iter().map(|c| (c.0, c.1)).collect()
}

/// (text of each run, its raw record) of story `k`.
fn runs(file: &[u8], k: usize) -> Vec<(String, Vec<u8>)> {
    let r = pub_write::story_records(file, k).unwrap();
    r.runs
        .iter()
        .map(|run| {
            (
                String::from_utf16_lossy(&r.text[run.start..run.end]),
                r.char_records[run.record].clone(),
            )
        })
        .collect()
}

/// A master box with `text` and fields at `fields`, as KernPage writes it.
fn master_box(file: &[u8], y: f64, text: &str, fields: &[usize]) -> Vec<u8> {
    let b = add_text_box(file, master_a(), 72.0, y, 216.0, 36.0, text).unwrap();
    let story = pub_format::read_bytes(&b).unwrap().stories.len() - 1;
    set_story_text_with_fields(&b, story, text, fields).unwrap()
}

fn same_quill_as_publisher(ours: &[u8], theirs: &[u8], stories: usize, what: &str) {
    assert_eq!(directory(ours), directory(theirs), "{what}: directory");
    for tag in ["TEXT", "STRS", "SYID", "MCLD"] {
        // SYID and MCLD hold story numbers and frame ids that do not depend on the field.
        let (a, b) = (chunk(ours, tag, 0), chunk(theirs, tag, 0));
        if tag == "TEXT" || tag == "STRS" {
            assert_eq!(a, b, "{what}: {tag}");
        } else {
            assert_eq!(a.map(|v| v.len()), b.map(|v| v.len()), "{what}: {tag} length");
        }
    }
    for (a, b) in chunks(ours).iter().zip(chunks(theirs).iter()) {
        if a.0 == "TOKN" {
            assert_eq!(a, b, "{what}: TOKN {}", a.1);
        }
    }
    for k in 0..stories {
        assert_eq!(runs(ours, k), runs(theirs, k), "{what}: runs of story {k}");
    }
}

#[kernpage_samples::test]
fn one_field_matches_publisher() {
    let ours = master_box(&blank(), 700.0, "#", &[0]);
    let theirs = kernpage_samples::read("gen/master-pn-2p.pub");
    // Publisher's file holds the text "Page #"; compare the field table layout instead.
    let t = chunk(&theirs, "TOKN", 0).unwrap();
    assert_eq!(t.len(), 52);
    let o = chunk(&ours, "TOKN", 0).unwrap();
    assert_eq!(o.len(), 52);
    assert_eq!(o[..12], t[..12]);
    assert_eq!(o[20..], t[20..]);
    assert_eq!(u32::from_le_bytes(o[12..16].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(o[16..20].try_into().unwrap()), 2);
    let r = pub_format::read_bytes(&ours).unwrap();
    assert!(r.masters[0].shapes[0].runs.iter().any(|r| r.field.is_some()));
}

#[kernpage_samples::test]
fn two_fields_with_text_match_publisher() {
    let ours = master_box(&blank(), 700.0, "Page # of #", &[5, 10]);
    let theirs = kernpage_samples::read("gen/master-pn-text-2fields.pub");
    same_quill_as_publisher(&ours, &theirs, 1, "Page # of #");
    assert_gaps(&ours, "ours");
    assert_gaps(&theirs, "theirs");
    assert_eq!(chunk(&ours, "TOKN", 0).unwrap().len(), 64);
}

#[kernpage_samples::test]
fn three_fields_match_publisher() {
    let ours = master_box(&blank(), 700.0, "A # B #  C #", &[2, 6, 11]);
    assert_gaps(&ours, "three fields");
    let theirs = kernpage_samples::read("gen/master-pn-3fields.pub");
    same_quill_as_publisher(&ours, &theirs, 1, "A # B #  C #");
    assert_eq!(chunk(&ours, "TOKN", 0).unwrap().len(), 76);
}

#[kernpage_samples::test]
fn two_boxes_have_one_table_each_by_story_index() {
    let one = master_box(&blank(), 700.0, "#", &[0]);
    let ours = master_box(&one, 600.0, "#", &[0]);
    let theirs = kernpage_samples::read("gen/master-pn-2boxes.pub");
    same_quill_as_publisher(&ours, &theirs, 2, "two boxes");
    assert_gaps(&ours, "ours");
    assert!(chunk(&ours, "TOKN", 0).is_some() && chunk(&ours, "TOKN", 1).is_some());
}

#[kernpage_samples::test]
fn a_field_in_a_later_story_counts_from_its_own_start() {
    // A plain story first, then a box with a field: TOKN instance 1, cps from the story start.
    let plain = add_text_box(&blank(), 0, 72.0, 200.0, 216.0, 36.0, "Hello world").unwrap();
    let ours = master_box(&plain, 700.0, "#", &[0]);
    assert_eq!(chunk(&ours, "TOKN", 0), None);
    let t = chunk(&ours, "TOKN", 1).unwrap();
    assert_eq!(u32::from_le_bytes(t[12..16].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(t[16..20].try_into().unwrap()), 2);
    let doc = pub_format::read_bytes(&ours).unwrap();
    assert!(doc.pages[0].shapes[0].runs.iter().all(|r| r.field.is_none()));
    assert!(doc.masters[0].shapes[0].runs.iter().any(|r| r.field.is_some()));
}

#[kernpage_samples::test]
fn fields_are_added_and_removed_one_by_one() {
    let one = master_box(&blank(), 700.0, "Page # of #", &[5]);
    assert_eq!(chunk(&one, "TOKN", 0).unwrap().len(), 52);
    let story = pub_format::read_bytes(&one).unwrap().stories.len() - 1;
    let two = set_story_text_with_fields(&one, story, "Page # of #", &[5, 10]).unwrap();
    assert_eq!(chunk(&two, "TOKN", 0).unwrap().len(), 64);
    let none = set_story_text_with_fields(&two, story, "Page # of #", &[]).unwrap();
    assert_eq!(chunk(&none, "TOKN", 0), None);
    assert_eq!(directory(&none), directory(&master_box_plain()));
    let quill = |f: &[u8]| pub_format::Streams::from_cfb(f).unwrap().quill.len();
    assert_eq!(quill(&none), quill(&master_box_plain()));

}

fn master_box_plain() -> Vec<u8> {
    add_text_box(&blank(), master_a(), 72.0, 700.0, 216.0, 36.0, "Page # of #").unwrap()
}

#[kernpage_samples::test]
fn a_file_without_fields_is_not_touched() {
    let plain = master_box_plain();
    let story = pub_format::read_bytes(&plain).unwrap().stories.len() - 1;
    let same = pub_write::set_story_text_keep_formatting(&plain, story, "Page # of #").unwrap();
    assert_eq!(directory(&same), directory(&plain));
    assert_eq!(chunk(&same, "TOKN", 0), None);
}
