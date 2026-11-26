use pub_write::{set_table_text, WriteError};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

fn cells_of(bytes: &[u8]) -> Vec<String> {
    let d = pub_format::read_bytes(bytes).unwrap();
    d.pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find_map(|s| s.table.as_ref())
        .unwrap()
        .cells
        .iter()
        .map(|c| c.replace('\r', "\n"))
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

fn check(sample: &str, cells: Vec<String>) -> Vec<u8> {
    let x = gen(sample);
    let out = set_table_text(&x, 0, &cells).unwrap();
    assert_eq!(cells_of(&out), cells);
    // The result is a valid file for the generic rewrite as well.
    pub_format::read_bytes(&pub_write::rewrite(&out).unwrap()).unwrap();
    out
}

#[kernpage_samples::test]
fn unchanged_cells_are_identical_in_content() {
    let x = gen("table2");
    let cells = cells_of(&x);
    let out = set_table_text(&x, 0, &cells).unwrap();
    assert_eq!(cells_of(&out), cells);
    let (a, b) = (
        pub_format::read_bytes(&x).unwrap(),
        pub_format::read_bytes(&out).unwrap(),
    );
    assert_eq!(a.stories, b.stories);
}

#[kernpage_samples::test]
fn shorter_longer_empty_and_multi_paragraph_cells() {
    let base = cells_of(&gen("table2"));
    assert_eq!(base.len(), 9);
    let mut c = base.clone();
    c[4] = "x".into(); // three paragraphs -> one
    c[6] = "y".into(); // two -> one
    check("table2", c);

    let mut c = base.clone();
    c[1] = "A much longer cell text than before".into();
    c[5] = "l1\nl2\nl3\nl4".into();
    check("table2", c);

    let mut c = base.clone();
    c[0] = String::new();
    check("table2", c);

    let c = vec![String::new(); 9];
    check("table2", c);

    let mut c = base;
    c[8] = String::new(); // the last cell
    c[2] = "one\ntwo\n\nfour".into(); // an empty paragraph in the middle
    check("table2", c);
}

#[kernpage_samples::test]
fn small_table_and_unicode() {
    check("table", strs(&["a", "bb\ncc", "", "ünï\u{1F600}"]));
}

#[kernpage_samples::test]
fn tcd_matches_what_publisher_writes_for_the_same_edit() {
    // Publisher-made variant (out/save-test6/pub-long): cells[1] and cells[5] edited.
    let mut c = cells_of(&gen("table2"));
    c[1] = "A much longer cell text than before".into();
    c[5] = "l1\nl2\nl3\nl4".into();
    let out = set_table_text(&gen("table2"), 0, &c).unwrap();
    let streams = pub_format::Streams::from_cfb(&out).unwrap();
    let d = &streams.quill;
    let u32at = |o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize;
    let n = u16::from_le_bytes([d[0x1A], d[0x1B]]) as usize;
    let tcd = (0..n)
        .map(|i| 0x18 + 8 + 24 * i)
        .find(|&e| &d[e + 2..e + 6] == b"TCD ")
        .expect("TCD entry in block 0");
    let (off, len) = (u32at(tcd + 16), u32at(tcd + 20));
    let ends: Vec<usize> = (12..len).step_by(4).map(|k| u32at(off + k)).collect();
    assert_eq!(ends, vec![5, 41, 47, 53, 104, 116, 126, 132, 139]);
}

#[kernpage_samples::test]
fn errors() {
    let x = gen("table2");
    assert!(matches!(
        set_table_text(&x, 0, &strs(&["a"])),
        Err(WriteError::Quill(_))
    ));
    let a = gen("apple-banana");
    assert!(matches!(
        set_table_text(&a, 0, &strs(&["a"])),
        Err(WriteError::Quill(_))
    ));
    assert!(matches!(
        set_table_text(&a, 9, &strs(&["a"])),
        Err(WriteError::NoSuchStory { .. })
    ));
}
