//! Master pages written into `.pub` files (spec 4T.8 to 4T.12), compared with the files
//! Publisher 2021 itself made (`samples/masters/`, research 2026-11-28).

use pub_write::{
    add_master, add_shape, delete_master, rename_master, set_page_masters, NewShapeKind, PageMaster,
    ShapeStyle,
};

fn sample(name: &str) -> Vec<u8> {
    kernpage_samples::read(&format!("masters/{name}.pub"))
}

/// Every Contents object as one text line (number, kind, parent, items), leaving out the
/// items that change by themselves on every save: object 256's save counter (item 60) and
/// the wrap flags of a shape (item 4).
fn listing(bytes: &[u8]) -> Vec<String> {
    let st = pub_format::Streams::from_cfb(bytes).unwrap();
    let c = pub_format::contents::Contents::parse(&st.contents).unwrap();
    let mut out = Vec::new();
    let n = c.objects.iter().map(|o| o.number).max().unwrap();
    for i in 0..=n {
        // Kinds 75 and 79 (the printer list) are rewritten by every Publisher save.
        let Some(o) = c.get(i).filter(|o| !matches!(o.kind, 75 | 79)) else { continue };
        let items = pub_format::items::decode_block(&st.contents, o.offset).unwrap();
        let text: Vec<String> = items
            .iter()
            .filter(|it| !(o.number == 256 && it.id == 60) && !(o.kind == 1 && it.id == 4))
            .map(|it| format!("{}={:?}", it.id, it.value))
            .collect();
        out.push(format!("{} kind {} parent {:?}: {}", o.number, o.kind, o.parent, text.join(" ")));
    }
    out
}

fn assert_same_listing(ours: &[u8], theirs: &[u8], what: &str) {
    let (a, b) = (listing(ours), listing(theirs));
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x, y, "{what}");
    }
    assert_eq!(a.len(), b.len(), "{what}: object count");
}

fn masters(bytes: &[u8]) -> Vec<(String, String)> {
    pub_format::read_bytes(bytes)
        .unwrap()
        .masters
        .iter()
        .map(|m| (m.letter.clone(), m.name.clone()))
        .collect()
}

fn page_masters(bytes: &[u8]) -> Vec<(Option<usize>, bool)> {
    pub_format::read_bytes(bytes)
        .unwrap()
        .pages
        .iter()
        .map(|p| (p.master, p.ignore_master))
        .collect()
}

#[kernpage_samples::test]
fn add_master_matches_publisher() {
    let ours = add_master(&sample("base-3p"), 0, 'B', "Master Page B").unwrap();
    assert_same_listing(&ours, &sample("m-add"), "add master");
    assert_eq!(masters(&ours).len(), 2);
}

#[kernpage_samples::test]
fn apply_master_matches_publisher() {
    let added = add_master(&sample("base-3p"), 0, 'B', "Master Page B").unwrap();
    let ours = set_page_masters(
        &added,
        &[PageMaster { page: 1, master: Some(1), ignore: false }],
    )
    .unwrap();
    assert_same_listing(&ours, &sample("m-apply"), "apply master");
    assert_eq!(page_masters(&ours), [(Some(0), false), (Some(1), false), (Some(0), false)]);
}

#[kernpage_samples::test]
fn ignore_master_matches_publisher() {
    let ours = set_page_masters(
        &sample("m-apply"),
        &[PageMaster { page: 1, master: None, ignore: true }],
    )
    .unwrap();
    assert_same_listing(&ours, &sample("m-ignore-b"), "ignore master");
    // Turning it off again writes the file as it was.
    let back = set_page_masters(
        &ours,
        &[PageMaster { page: 1, master: None, ignore: false }],
    )
    .unwrap();
    assert_same_listing(&back, &sample("m-apply"), "ignore off");
}

#[kernpage_samples::test]
fn rename_master_matches_publisher() {
    let added = add_master(&sample("base-3p"), 0, 'B', "Master Page B").unwrap();
    let ours = rename_master(&added, 1, 'B', "Renamed").unwrap();
    assert_same_listing(&ours, &sample("m-rename"), "rename master");
    assert_eq!(masters(&ours)[1], ("B".to_string(), "Renamed".to_string()));
    // The id letter is free to change (item 14).
    let lettered = rename_master(&ours, 1, 'Z', "Renamed").unwrap();
    assert_eq!(masters(&lettered)[1].0, "Z");
}

#[kernpage_samples::test]
fn delete_master_matches_publisher() {
    // Page 2 used B: it goes back to A.
    let used = delete_master(&sample("m-apply"), 1).unwrap();
    assert_same_listing(&used, &sample("m-del-used"), "delete B");
    assert_same_listing(&used, &sample("base-3p"), "delete B gives the file before the add");
    // Deleting A: every page moves to the master that remains.
    let first = delete_master(&sample("m-apply"), 0).unwrap();
    assert_same_listing(&first, &sample("m-del-a-used"), "delete A");
    assert_eq!(page_masters(&first), [(Some(0), false); 3]);
}

#[kernpage_samples::test]
fn the_last_master_cannot_be_deleted() {
    let e = delete_master(&sample("base-3p"), 0).unwrap_err();
    assert!(e.to_string().contains("default master page"), "{e}");
}

#[kernpage_samples::test]
fn a_shape_on_a_master_is_a_shape_on_a_page_with_another_parent() {
    let style = ShapeStyle::default();
    let on_master = add_shape(
        &sample("base-1p"),
        pub_format::master_surface(0),
        NewShapeKind::Rectangle,
        100.0,
        100.0,
        144.0,
        72.0,
        style,
    )
    .unwrap();
    let on_page = add_shape(
        &sample("base-1p"),
        0,
        NewShapeKind::Rectangle,
        100.0,
        100.0,
        144.0,
        72.0,
        style,
    )
    .unwrap();
    // Escher is the same for both.
    let (sm, sp) = (
        pub_format::Streams::from_cfb(&on_master).unwrap(),
        pub_format::Streams::from_cfb(&on_page).unwrap(),
    );
    assert_eq!(sm.escher, sp.escher);
    // Publisher's own file for the master version.
    assert_same_listing(&on_master, &sample("m-rect-M"), "rectangle on master");
    let doc = pub_format::read_bytes(&on_master).unwrap();
    assert_eq!(doc.masters[0].shapes.len(), 1);
    assert!(doc.pages[0].shapes.is_empty());
}

#[kernpage_samples::test]
fn a_text_box_on_the_second_master_matches_publisher() {
    let added = add_master(&sample("base-3p"), 0, 'B', "Master Page B").unwrap();
    let ours = pub_write::add_text_box(&added, pub_format::master_surface(1), 72.0, 72.0, 216.0, 36.0, "B-TEXT")
        .unwrap();
    let theirs = sample("m-add-box");
    assert_same_listing(&ours, &theirs, "text box on master B");
    let doc = pub_format::read_bytes(&ours).unwrap();
    assert_eq!(doc.masters[1].shapes.len(), 1);
    assert_eq!(doc.masters[1].shapes[0].text.as_deref().map(str::trim_end), Some("B-TEXT"));
    let doc2 = pub_format::read_bytes(&theirs).unwrap();
    assert_eq!(doc2.masters[1].shapes[0].text.as_deref().map(str::trim_end), Some("B-TEXT"));
}
