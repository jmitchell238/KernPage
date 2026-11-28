//! Saving master page changes into `.pub` files (spec 4T.8 to 4T.12): added, renamed and
//! deleted masters, the master each page uses, the shapes on a master, and the plan that
//! follows each save.

use kernpage_core::{
    from_pub_flowed, master_surface, Command, Document, Editor, Paragraph, Rect, Rgb, SaveSource,
    Shape, ShapeKind,
};

fn open_bytes(bytes: Vec<u8>) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(&bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    (SaveSource::new(bytes, &src, &model, &breaks), Editor::new(model))
}

fn open(rel: &str) -> (SaveSource, Editor) {
    open_bytes(kernpage_samples::read(rel))
}

fn open_base3() -> (SaveSource, Editor) {
    open("masters/base-3p.pub")
}

/// Save `e` into `source`: asserts the save plan had nothing unsupported, marks it saved and
/// returns the bytes.
fn save(source: &mut SaveSource, e: &Editor) -> Vec<u8> {
    let (out, plan) = source.build(e.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "unsupported: {:?}", plan.unsupported);
    source.mark_saved(out.clone(), e.doc(), &plan);
    out
}

/// What is left to save after a save of everything: nothing.
fn assert_clean(source: &SaveSource, e: &Editor) {
    let plan = source.plan(e.doc());
    assert!(plan.is_empty(), "left to save: {plan:?}");
}

fn add_master(e: &mut Editor) {
    e.execute(Command::AddMaster {
        copy_of: None,
        id: None,
        description: String::new(),
        two_page: false,
    })
    .unwrap();
}

fn text_box(text: &str) -> Shape {
    Shape::text_box(Rect::new(914_400, 914_400, 2_743_200, 457_200), vec![Paragraph::plain(text)])
}

fn rectangle() -> Shape {
    let mut s = Shape::new(Rect::new(1_270_000, 1_270_000, 1_828_800, 914_400), ShapeKind::Rectangle);
    s.fill = Some(Rgb(200, 30, 30));
    s
}

fn reread(bytes: &[u8]) -> Document {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    from_pub_flowed("t.pub", &src, &breaks)
}

fn names(d: &Document) -> Vec<(char, String)> {
    d.masters.iter().map(|m| (m.id, m.name.clone())).collect()
}

fn page_masters(d: &Document) -> Vec<(Option<usize>, bool)> {
    d.pages.iter().map(|p| (p.master, p.ignore_master)).collect()
}

#[kernpage_samples::test]
fn an_added_master_applied_to_a_page_is_saved() {
    let (mut source, mut e) = open_base3();
    add_master(&mut e);
    e.execute(Command::ApplyMaster { pages: vec![1], master: Some(1) }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(names(&d), [('A', "Master Page A".to_string()), ('B', "Master Page B".to_string())]);
    assert_eq!(page_masters(&d), [(Some(0), false), (Some(1), false), (Some(0), false)]);
    assert_clean(&source, &e);
    // The file Publisher made for the same two steps has the same objects.
    let theirs = kernpage_samples::read("masters/m-apply.pub");
    assert_eq!(pub_format::read_bytes(&out).unwrap().masters.len(), pub_format::read_bytes(&theirs).unwrap().masters.len());
}

#[kernpage_samples::test]
fn ignoring_the_master_and_applying_it_again_is_saved() {
    let (mut source, mut e) = open_base3();
    e.execute(Command::ApplyMaster { pages: vec![2], master: None }).unwrap();
    let out = save(&mut source, &e);
    assert_eq!(page_masters(&reread(&out)), [(Some(0), false), (Some(0), false), (Some(0), true)]);
    assert_clean(&source, &e);
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(0) }).unwrap();
    let out = save(&mut source, &e);
    assert_eq!(page_masters(&reread(&out)), [(Some(0), false); 3]);
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_renamed_master_is_saved_and_the_description_is_left_alone() {
    let (mut source, mut e) = open("masters/m-add.pub");
    e.execute(Command::SetMasterInfo {
        master: 1,
        id: 'Q',
        name: "Cover".into(),
        description: "the cover page".into(),
    })
    .unwrap();
    let out = save(&mut source, &e);
    assert_eq!(names(&reread(&out))[1], ('Q', "Cover".to_string()));
    assert_clean(&source, &e);
    // A description alone changes nothing in the file and is not reported.
    e.execute(Command::SetMasterInfo {
        master: 1,
        id: 'Q',
        name: "Cover".into(),
        description: "another".into(),
    })
    .unwrap();
    let plan = source.plan(e.doc());
    assert!(plan.is_empty(), "{plan:?}");
}

#[kernpage_samples::test]
fn deleting_a_master_repoints_its_pages() {
    let (mut source, mut e) = open("masters/m-apply.pub");
    assert_eq!(page_masters(e.doc())[1].0, Some(1));
    e.execute(Command::DeleteMaster { master: 1 }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters.len(), 1);
    assert_eq!(page_masters(&d), [(Some(0), false); 3]);
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn deleting_the_first_master_moves_every_page_to_the_next() {
    let (mut source, mut e) = open("masters/m-apply.pub");
    e.execute(Command::DeleteMaster { master: 0 }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(names(&d), [('B', "Master Page B".to_string())]);
    assert_eq!(page_masters(&d), [(Some(0), false); 3]);
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn shapes_on_a_master_are_added_edited_moved_and_deleted() {
    let (mut source, mut e) = open_base3();
    let s = master_surface(0, false);
    let ids = e.execute(Command::AddShape { page: s, shape: rectangle() }).unwrap();
    let rect = ids[0];
    let ids = e.execute(Command::AddShape { page: s, shape: text_box("Footer") }).unwrap();
    let text = ids[0];
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters[0].shapes.len(), 2);
    assert!(d.pages.iter().all(|p| p.shapes.is_empty()));
    assert!(matches!(d.masters[0].shapes[1].kind, ShapeKind::TextBox { .. }));
    assert_clean(&source, &e);

    // Move and resize the rectangle, change the text, then save again.
    e.execute(Command::SetRect { page: s, id: rect, rect: Rect::new(2_000_000, 1_500_000, 2_500_000, 1_000_000) })
        .unwrap();
    e.execute(Command::SetText { page: s, id: text, paragraphs: vec![Paragraph::plain("Footer text")] })
        .unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters[0].shapes[0].rect, Rect::new(2_000_000, 1_500_000, 2_500_000, 1_000_000));
    match &d.masters[0].shapes[1].kind {
        ShapeKind::TextBox { paragraphs } => assert_eq!(paragraphs[0].text().trim_end(), "Footer text"),
        k => panic!("{k:?}"),
    }
    assert_clean(&source, &e);

    // Stacking order, then delete both.
    e.execute(Command::BringToFront { page: s, id: rect }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert!(matches!(d.masters[0].shapes[1].kind, ShapeKind::Rectangle));
    assert_clean(&source, &e);
    e.execute(Command::DeleteShapes { page: s, ids: vec![rect, text] }).unwrap();
    // As on a page, the writer cannot remove a plain shape or text box yet (only pictures and
    // tables); the save says so instead of dropping the change.
    let plan = source.plan(e.doc());
    assert_eq!(plan.unsupported, ["deleted objects"]);
}

#[kernpage_samples::test]
fn shapes_on_a_new_master_and_the_master_after_it_are_saved() {
    let (mut source, mut e) = open_base3();
    add_master(&mut e);
    let s = master_surface(1, false);
    e.execute(Command::AddShape { page: s, shape: rectangle() }).unwrap();
    e.execute(Command::AddShape { page: s, shape: text_box("B header") }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![0, 2], master: Some(1) }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters.len(), 2);
    assert_eq!(d.masters[1].shapes.len(), 2);
    assert!(d.masters[0].shapes.is_empty());
    assert_eq!(page_masters(&d), [(Some(1), false), (Some(0), false), (Some(1), false)]);
    assert_clean(&source, &e);
    // The master that was just added now delivers the text edit of its box.
    let id = e.doc().masters[1].shapes[1].id;
    e.execute(Command::SetText { page: s, id, paragraphs: vec![Paragraph::plain("B footer")] })
        .unwrap();
    let out = save(&mut source, &e);
    match &reread(&out).masters[1].shapes[1].kind {
        ShapeKind::TextBox { paragraphs } => assert_eq!(paragraphs[0].text().trim_end(), "B footer"),
        k => panic!("{k:?}"),
    }
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_duplicated_master_with_its_shapes_is_saved() {
    let (mut source, mut e) = open_base3();
    e.execute(Command::AddShape { page: master_surface(0, false), shape: rectangle() }).unwrap();
    e.execute(Command::AddMaster { copy_of: Some(0), id: None, description: String::new(), two_page: false })
        .unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters.len(), 2);
    assert_eq!(d.masters[0].shapes.len(), 1);
    assert_eq!(d.masters[1].shapes.len(), 1);
    assert_eq!(d.masters[1].name, "Copy of Master Page A");
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_master_with_a_shape_is_deleted_with_its_shape() {
    let (mut source, mut e) = open("masters/m-add-box.pub");
    assert_eq!(e.doc().masters[1].shapes.len(), 1);
    e.execute(Command::DeleteMaster { master: 1 }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters.len(), 1);
    assert!(d.masters[0].shapes.is_empty());
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn the_page_number_footer_text_of_a_master_is_edited_and_keeps_its_field() {
    let (mut source, mut e) = open("gen/master-pn-2p.pub");
    let s = master_surface(0, false);
    let footer = e
        .doc()
        .masters[0]
        .shapes
        .iter()
        .find(|sh| match &sh.kind {
            ShapeKind::TextBox { paragraphs } => {
                paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.field.is_some())
            }
            _ => false,
        })
        .cloned();
    let Some(footer) = footer else {
        println!("no page-number box on the master of this sample");
        return;
    };
    let ShapeKind::TextBox { paragraphs } = &footer.kind else { unreachable!() };
    let mut paras = paragraphs.clone();
    let mut first = paras[0].runs[0].clone();
    first.text = "Page ".into();
    first.field = None;
    paras[0].runs.insert(0, first);
    e.execute(Command::SetText { page: s, id: footer.id, paragraphs: paras }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    let has_field = d.masters[0].shapes.iter().any(|sh| match &sh.kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.field.is_some()),
        _ => false,
    });
    assert!(has_field);
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn two_page_masters_are_still_refused_with_a_reason() {
    let (source, mut e) = open_base3();
    e.execute(Command::SetTwoPageMaster { master: 0, two_page: true }).unwrap();
    let plan = source.plan(e.doc());
    assert!(plan.unsupported.iter().any(|u| u.contains("two-page master pages")), "{:?}", plan.unsupported);
}

#[kernpage_samples::test]
fn page_setup_changes_reach_every_master() {
    let (mut source, mut e) = open("masters/m-add.pub");
    let m = kernpage_core::Margins::all(609_600);
    e.execute(Command::SetMargins { margins: m }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert!(d.pages.iter().all(|p| p.margins_emu == m));
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_new_publication_with_two_masters_saves() {
    let pages = (0..2).map(|_| kernpage_core::Page::blank(7_772_400, 10_058_400)).collect();
    let mut e = Editor::new(Document::new("t", pages));
    e.execute(Command::EnsureMaster).unwrap();
    add_master(&mut e);
    e.execute(Command::AddShape { page: master_surface(0, false), shape: rectangle() }).unwrap();
    e.execute(Command::AddShape { page: master_surface(1, false), shape: text_box("B") }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![1], master: Some(1) }).unwrap();
    let mut source = SaveSource::for_new(e.doc()).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.masters.len(), 2);
    assert_eq!(d.masters[0].shapes.len(), 1);
    assert_eq!(d.masters[1].shapes.len(), 1);
    assert_eq!(page_masters(&d)[1].0, Some(1));
    assert_clean(&source, &e);
}

// ---- pictures, tables and groups on a master -------------------------------------------------

fn asset(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/assets").join(name)).unwrap()
}

#[kernpage_samples::test]
fn a_picture_on_a_master_is_added_and_deleted() {
    use kernpage_core::picture::sniff_format;
    let (mut source, mut e) = open_base3();
    let s = master_surface(0, false);
    let bytes = asset("red4x3.png");
    let shape = Shape::new(
        Rect::new(914_400, 914_400, 1_219_200, 914_400),
        ShapeKind::Picture {
            data: std::sync::Arc::new(bytes.clone()),
            format: sniff_format(&bytes),
            file: Some(kernpage_core::PictureFile { name: "red4x3.png".into(), modified: Some(132_000_000_000_000_000) }),
        },
    );
    let id = e.execute(Command::AddShape { page: s, shape }).unwrap()[0];
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.masters[0].shapes.iter().any(|sh| sh.picture.is_some()));
    assert!(d.pages.iter().all(|p| p.shapes.is_empty()));
    assert_clean(&source, &e);
    e.execute(Command::DeleteShapes { page: s, ids: vec![id] }).unwrap();
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.masters[0].shapes.is_empty());
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_table_on_a_master_is_added_edited_and_deleted() {
    let (mut source, mut e) = open_base3();
    let s = master_surface(0, false);
    let rect = kernpage_core::table::default_table_rect(7_772_400, 10_058_400, 2, 2);
    let id = e.execute(Command::InsertTable { page: s, rect, rows: 2, columns: 2 }).unwrap()[0];
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.masters[0].shapes.iter().any(|sh| sh.table.is_some()));
    assert_clean(&source, &e);
    e.execute(Command::SetCellText { page: s, id, cell: 1, paragraphs: vec![Paragraph::plain("cell")] })
        .unwrap();
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    let t = d.masters[0].shapes.iter().find_map(|sh| sh.table.as_ref()).unwrap();
    assert!(t.cells[1].contains("cell"), "{:?}", t.cells);
    assert_clean(&source, &e);
    e.execute(Command::InsertTableRows { page: s, shape: id, before: 2, count: 1 }).unwrap();
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    let t = d.masters[0].shapes.iter().find_map(|sh| sh.table.as_ref()).unwrap();
    assert_eq!(t.rows, 3);
    assert_clean(&source, &e);
    e.execute(Command::DeleteShapes { page: s, ids: vec![e.doc().masters[0].shapes[0].id] }).unwrap();
    let out = save(&mut source, &e);
    assert!(pub_format::read_bytes(&out).unwrap().masters[0].shapes.is_empty());
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_group_on_a_master_is_made_and_dissolved() {
    let (mut source, mut e) = open_base3();
    let s = master_surface(0, false);
    let a = e.execute(Command::AddShape { page: s, shape: rectangle() }).unwrap()[0];
    let mut second = rectangle();
    second.rect = Rect::new(3_000_000, 3_000_000, 1_000_000, 700_000);
    let b = e.execute(Command::AddShape { page: s, shape: second }).unwrap()[0];
    save(&mut source, &e);
    assert_clean(&source, &e);
    e.execute(Command::Group { page: s, ids: vec![a, b] }).unwrap();
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    assert_eq!(d.masters[0].shapes.iter().filter(|sh| sh.group.is_some()).count(), 2);
    assert_clean(&source, &e);
    let group = e.doc().masters[0].shapes.iter().find(|sh| matches!(sh.kind, ShapeKind::Group)).unwrap().id;
    e.execute(Command::Ungroup { page: s, id: group }).unwrap();
    let out = save(&mut source, &e);
    let d = pub_format::read_bytes(&out).unwrap();
    assert!(d.masters[0].shapes.iter().all(|sh| sh.group.is_none()));
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn a_new_page_after_a_page_on_another_master_gets_the_first_master() {
    let (mut source, mut e) = open("masters/m-apply.pub");
    // Page 3 uses A; use B on the last page, then add a page after it.
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(1) }).unwrap();
    save(&mut source, &e);
    e.execute(Command::InsertPage { after: 2 }).unwrap();
    assert_eq!(e.doc().pages[3].master, Some(0));
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(d.pages.len(), 4);
    assert_eq!(page_masters(&d), [(Some(0), false), (Some(1), false), (Some(1), false), (Some(0), false)]);
    assert_clean(&source, &e);
}

#[kernpage_samples::test]
fn changes_that_have_nothing_to_do_with_masters_leave_them_alone() {
    let (mut source, mut e) = open("masters/m-apply.pub");
    e.execute(Command::AddShape { page: 0, shape: text_box("on page 1") }).unwrap();
    let out = save(&mut source, &e);
    let d = reread(&out);
    assert_eq!(names(&d).len(), 2);
    assert_eq!(page_masters(&d), [(Some(0), false), (Some(1), false), (Some(0), false)]);
    assert_eq!(d.pages[0].shapes.len(), 1);
    assert!(d.masters.iter().all(|m| m.shapes.is_empty()));
    assert_clean(&source, &e);
}

// ---- files for Publisher 2021 to open -----------------------------------------------------------

/// What Publisher should read back from a written file (written next to it as `<name>.json`).
fn expectation(d: &Document) -> serde_json::Value {
    let top = |shapes: &[Shape]| shapes.iter().filter(|s| s.group.is_none()).count();
    serde_json::json!({
        "masters": d.masters.iter().map(|m| serde_json::json!({
            "name": m.name,
            "shapes": top(&m.shapes),
        })).collect::<Vec<_>>(),
        "pages": d.pages.iter().enumerate().map(|(i, p)| serde_json::json!({
            "number": i + 1,
            "master": p.master.and_then(|m| d.masters.get(m)).map(|m| m.name.clone()),
            "ignore": p.ignore_master,
            "shapes": top(&p.shapes),
        })).collect::<Vec<_>>(),
    })
}

/// `cargo test -p kernpage-core --test master_save -- --ignored write_publisher_check_files`
/// writes files made with the master page writer to open in Publisher 2021 (folder in
/// `MASTERS_OUT_DIR`), each with the read-back it should give.
#[kernpage_samples::test]
#[ignore = "writes files for Publisher to open"]
fn write_publisher_check_files() {
    let out = std::path::PathBuf::from(
        std::env::var("MASTERS_OUT_DIR")
            .unwrap_or_else(|_| std::env::temp_dir().join("pw-masters").to_string_lossy().into_owned()),
    );
    std::fs::create_dir_all(&out).unwrap();
    let put = |name: &str, source: &mut SaveSource, e: &Editor| {
        let bytes = save(source, e);
        std::fs::write(out.join(format!("{name}.pub")), &bytes).unwrap();
        // What the file says when KernPage reads it back, and what we meant.
        let back = reread(&bytes);
        assert_eq!(expectation(&back), expectation(e.doc()), "{name}");
        std::fs::write(
            out.join(format!("{name}.json")),
            serde_json::to_string_pretty(&expectation(e.doc())).unwrap(),
        )
        .unwrap();
    };

    // 1. A footer page number (the file's own) edited, and a rectangle, on master A.
    let (mut source, mut e) = open("gen/master-pn-2p.pub");
    let s = master_surface(0, false);
    e.execute(Command::AddShape { page: s, shape: rectangle() }).unwrap();
    put("k1-footer-and-rectangle", &mut source, &e);

    // 2. A second master applied to page 2, page 3 ignoring its master.
    let (mut source, mut e) = open_base3();
    add_master(&mut e);
    e.execute(Command::AddShape { page: master_surface(1, false), shape: text_box("Master B header") }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![1], master: Some(1) }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![2], master: None }).unwrap();
    put("k2-second-master-and-ignore", &mut source, &e);

    // 3. A master deleted (page 2 used it).
    let (mut source, mut e) = open("masters/m-apply.pub");
    e.execute(Command::DeleteMaster { master: 1 }).unwrap();
    put("k3-deleted-master", &mut source, &e);

    // 4. Renamed, with another letter.
    let (mut source, mut e) = open("masters/m-add.pub");
    e.execute(Command::SetMasterInfo { master: 1, id: 'Z', name: "Renamed".into(), description: String::new() })
        .unwrap();
    put("k4-renamed-master", &mut source, &e);

    // 5. Every kind of shape on a master: rectangle, text box, picture, table, group.
    let (mut source, mut e) = open_base3();
    add_master(&mut e);
    let s = master_surface(1, false);
    e.execute(Command::AddShape { page: s, shape: text_box("Text on B") }).unwrap();
    let a = e.execute(Command::AddShape { page: s, shape: rectangle() }).unwrap()[0];
    let mut second = rectangle();
    second.rect = Rect::new(3_000_000, 3_000_000, 1_000_000, 700_000);
    let b = e.execute(Command::AddShape { page: s, shape: second }).unwrap()[0];
    // (New shapes are grouped after they are in the file, as on a page.)
    save(&mut source, &e);
    e.execute(Command::Group { page: s, ids: vec![a, b] }).unwrap();
    let bytes = asset("red4x3.png");
    let pic = Shape::new(
        Rect::new(914_400, 6_000_000, 1_219_200, 914_400),
        ShapeKind::Picture {
            data: std::sync::Arc::new(bytes.clone()),
            format: kernpage_core::picture::sniff_format(&bytes),
            file: Some(kernpage_core::PictureFile { name: "red4x3.png".into(), modified: Some(132_000_000_000_000_000) }),
        },
    );
    e.execute(Command::AddShape { page: s, shape: pic }).unwrap();
    let trect = kernpage_core::table::default_table_rect(7_772_400, 10_058_400, 2, 3);
    e.execute(Command::InsertTable { page: s, rect: trect, rows: 2, columns: 3 }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![0, 1], master: Some(1) }).unwrap();
    put("k5-shapes-on-a-master", &mut source, &e);

    // 6. A duplicated master with its shapes.
    let (mut source, mut e) = open_base3();
    e.execute(Command::AddShape { page: master_surface(0, false), shape: rectangle() }).unwrap();
    e.execute(Command::AddMaster { copy_of: Some(0), id: None, description: String::new(), two_page: false })
        .unwrap();
    put("k6-duplicated-master", &mut source, &e);

    // 7. A publication made in the editor with two masters.
    let pages = (0..3).map(|_| kernpage_core::Page::blank(7_772_400, 10_058_400)).collect();
    let mut e = Editor::new(Document::new("t", pages));
    e.execute(Command::EnsureMaster).unwrap();
    add_master(&mut e);
    e.execute(Command::AddShape { page: master_surface(0, false), shape: rectangle() }).unwrap();
    e.execute(Command::AddShape { page: master_surface(1, false), shape: text_box("B") }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(1) }).unwrap();
    let mut source = SaveSource::for_new(e.doc()).unwrap();
    put("k7-new-publication", &mut source, &e);

    // 8. A new page after the last page (which uses master B).
    let (mut source, mut e) = open("masters/m-apply.pub");
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(1) }).unwrap();
    e.execute(Command::InsertPage { after: 2 }).unwrap();
    put("k8-new-page-after-b", &mut source, &e);
}
