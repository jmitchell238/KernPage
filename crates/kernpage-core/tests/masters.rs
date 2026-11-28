//! Master page editing: commands with undo and redo, the master surfaces that let every shape
//! command edit a master, the `.kern` round trip and what the `.pub` save planner refuses.

use kernpage_core::native::{read_kern, write_kern};
use kernpage_core::{
    from_pub_flowed, master_surface, Command, CommandError, Document, Editor, MasterPage, Page,
    Paragraph, Rect, SaveSource, Shape, ShapeKind,
};

fn three_pages() -> Editor {
    let pages = (0..3).map(|_| Page::blank(7_772_400, 10_058_400)).collect();
    Editor::new(Document::new("t", pages))
}

fn text_box(text: &str) -> Shape {
    Shape::text_box(Rect::new(914_400, 914_400, 2_743_200, 457_200), vec![Paragraph::plain(text)])
}

/// Run `cmd`, check undo restores the document and redo repeats it; leaves it applied.
fn round_trip(e: &mut Editor, cmd: Command) {
    let before = e.doc().clone();
    e.execute(cmd).unwrap();
    let after = e.doc().clone();
    assert_ne!(after, before, "the command changed nothing");
    assert!(e.undo());
    assert_eq!(e.doc(), &before, "undo");
    assert!(e.redo());
    assert_eq!(e.doc(), &after, "redo");
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

#[test]
fn adding_a_master_to_a_document_without_one_makes_a_and_b() {
    let mut e = three_pages();
    assert!(e.doc().masters.is_empty());
    round_trip(
        &mut e,
        Command::AddMaster {
            copy_of: None,
            id: None,
            description: "  Cover with a long long long description that goes on and on  ".into(),
            two_page: false,
        },
    );
    let d = e.doc();
    assert_eq!(d.masters.len(), 2);
    assert_eq!((d.masters[0].id, d.masters[0].name.as_str()), ('A', "Master Page A"));
    assert_eq!((d.masters[1].id, d.masters[1].name.as_str()), ('B', "Master Page B"));
    assert_eq!(d.masters[1].description.chars().count(), MasterPage::MAX_DESCRIPTION);
    // Every page now uses master A.
    assert!(d.pages.iter().all(|p| p.master == Some(0) && !p.ignore_master));
    add_master(&mut e);
    assert_eq!(e.doc().masters[2].id, 'C');
}

#[test]
fn a_chosen_id_must_be_free_and_visible() {
    let mut e = three_pages();
    add_master(&mut e);
    let add = |id| Command::AddMaster { copy_of: None, id: Some(id), description: String::new(), two_page: false };
    assert_eq!(e.execute(add('A')), Err(CommandError::MasterIdInUse('A')));
    assert_eq!(e.execute(add(' ')), Err(CommandError::BadMasterId(' ')));
    e.execute(add('Z')).unwrap();
    assert_eq!(e.doc().masters.last().unwrap().name, "Master Page Z");
}

#[test]
fn duplicate_copies_the_shapes_with_new_ids() {
    let mut e = three_pages();
    e.execute(Command::EnsureMaster).unwrap();
    e.execute(Command::AddShape { page: master_surface(0, false), shape: text_box("Header") }).unwrap();
    e.execute(Command::AddShape { page: master_surface(0, false), shape: text_box("Footer") }).unwrap();
    round_trip(
        &mut e,
        Command::AddMaster { copy_of: Some(0), id: Some('Q'), description: "copy".into(), two_page: false },
    );
    let d = e.doc();
    assert_eq!(d.masters.len(), 2);
    assert_eq!(d.masters[1].id, 'Q');
    assert_eq!(d.masters[1].name, "Copy of Master Page A");
    assert_eq!(d.masters[1].shapes.len(), 2);
    let ids0: Vec<_> = d.masters[0].shapes.iter().map(|s| s.id).collect();
    for s in &d.masters[1].shapes {
        assert!(!ids0.contains(&s.id), "copies get fresh ids");
    }
    assert_eq!(d.masters[1].shapes[0].kind, d.masters[0].shapes[0].kind);
}

#[test]
fn rename_changes_name_description_and_id() {
    let mut e = three_pages();
    add_master(&mut e);
    round_trip(
        &mut e,
        Command::SetMasterInfo { master: 1, id: 'F', name: "Front".into(), description: "Cover".into() },
    );
    let m = &e.doc().masters[1];
    assert_eq!((m.id, m.name.as_str(), m.description.as_str()), ('F', "Front", "Cover"));
    assert_eq!(
        e.execute(Command::SetMasterInfo { master: 1, id: 'A', name: "x".into(), description: String::new() }),
        Err(CommandError::MasterIdInUse('A'))
    );
    // An empty name falls back to the default.
    e.execute(Command::SetMasterInfo { master: 1, id: 'F', name: " ".into(), description: String::new() })
        .unwrap();
    assert_eq!(e.doc().masters[1].name, "Master Page F");
}

#[test]
fn deleting_refuses_the_last_master_and_repoints_pages() {
    let mut e = three_pages();
    add_master(&mut e);
    add_master(&mut e); // A B C
    e.execute(Command::ApplyMaster { pages: vec![1], master: Some(1) }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(2) }).unwrap();
    let uses = |e: &Editor| e.doc().pages.iter().map(|p| p.master).collect::<Vec<_>>();
    assert_eq!(uses(&e), [Some(0), Some(1), Some(2)]);
    // Delete B: its page goes to the first master, C moves down one index.
    round_trip(&mut e, Command::DeleteMaster { master: 1 });
    assert_eq!(uses(&e), [Some(0), Some(0), Some(1)]);
    assert_eq!(e.doc().masters.iter().map(|m| m.id).collect::<Vec<_>>(), ['A', 'C']);
    // Delete the first: everything uses what is left.
    e.execute(Command::DeleteMaster { master: 0 }).unwrap();
    assert_eq!(uses(&e), [Some(0), Some(0), Some(0)]);
    assert_eq!(e.doc().masters[0].id, 'C');
    assert_eq!(e.execute(Command::DeleteMaster { master: 0 }), Err(CommandError::CannotDeleteLastMaster));
    assert_eq!(e.execute(Command::DeleteMaster { master: 5 }), Err(CommandError::NoSuchMaster(5)));
    // The ids are handed out again: A is free.
    add_master(&mut e);
    assert_eq!(e.doc().masters[1].id, 'A');
}

#[test]
fn applying_to_pages_all_pages_a_range_and_none() {
    let mut e = three_pages();
    add_master(&mut e);
    let uses = |e: &Editor| e.doc().pages.iter().map(|p| (p.master, p.ignore_master)).collect::<Vec<_>>();
    round_trip(&mut e, Command::ApplyMaster { pages: vec![1], master: Some(1) });
    assert_eq!(uses(&e), [(Some(0), false), (Some(1), false), (Some(0), false)]);
    round_trip(&mut e, Command::ApplyMaster { pages: (0..3).collect(), master: Some(1) });
    assert_eq!(uses(&e), [(Some(1), false); 3]);
    round_trip(&mut e, Command::ApplyMaster { pages: vec![1, 2], master: Some(0) });
    assert_eq!(uses(&e), [(Some(1), false), (Some(0), false), (Some(0), false)]);
    // None: the page ignores its master (and keeps the assignment).
    round_trip(&mut e, Command::ApplyMaster { pages: vec![2], master: None });
    assert_eq!(uses(&e)[2], (Some(0), true));
    // Applying a master again turns it back on.
    e.execute(Command::ApplyMaster { pages: vec![2], master: Some(0) }).unwrap();
    assert_eq!(uses(&e)[2], (Some(0), false));
    assert_eq!(
        e.execute(Command::ApplyMaster { pages: vec![9], master: Some(0) }),
        Err(CommandError::NoSuchPage(9))
    );
    assert_eq!(
        e.execute(Command::ApplyMaster { pages: vec![0], master: Some(7) }),
        Err(CommandError::NoSuchMaster(7))
    );
}

#[test]
fn shape_commands_edit_a_master_and_every_page_using_it_shows_it() {
    let mut e = three_pages();
    e.execute(Command::EnsureMaster).unwrap();
    let surface = master_surface(0, false);
    let ids = e.execute(Command::AddShape { page: surface, shape: text_box("Footer") }).unwrap();
    let id = ids[0];
    for i in 0..3 {
        assert_eq!(e.doc().master_shapes(i).len(), 1, "page {i}");
    }
    // Move, resize, edit text and style the master shape, each undoable.
    round_trip(&mut e, Command::MoveShapes { page: surface, ids: vec![id], dx: 100, dy: 200 });
    round_trip(&mut e, Command::SetRect { page: surface, id, rect: Rect::new(0, 0, 1_000_000, 300_000) });
    round_trip(&mut e, Command::SetText { page: surface, id, paragraphs: vec![Paragraph::plain("New")] });
    round_trip(
        &mut e,
        Command::StyleShapes {
            page: surface,
            ids: vec![id],
            change: kernpage_core::command::StyleChange::Fill(Some(kernpage_core::Rgb(1, 2, 3))),
        },
    );
    let r = e.doc().master_shapes(2)[0].rect;
    assert_eq!(r, Rect::new(0, 0, 1_000_000, 300_000));
    // A second shape, z-order and delete.
    let b = e.execute(Command::AddShape { page: surface, shape: text_box("B") }).unwrap()[0];
    round_trip(&mut e, Command::SendToBack { page: surface, id: b });
    assert_eq!(e.doc().masters[0].shapes[0].id, b);
    round_trip(&mut e, Command::DeleteShapes { page: surface, ids: vec![b] });
    assert_eq!(e.doc().masters[0].shapes.len(), 1);
    // The page itself is untouched.
    assert!(e.doc().pages.iter().all(|p| p.shapes.is_empty()));
    // None on a page hides the master shapes there.
    e.execute(Command::ApplyMaster { pages: vec![1], master: None }).unwrap();
    assert!(e.doc().master_shapes(1).is_empty());
    assert_eq!(e.doc().master_shapes(0).len(), 1);
}

#[test]
fn commands_on_a_missing_master_fail_cleanly() {
    let mut e = three_pages();
    assert!(matches!(
        e.execute(Command::AddShape { page: master_surface(0, false), shape: text_box("x") }),
        Err(CommandError::NoSuchPage(_))
    ));
    e.execute(Command::EnsureMaster).unwrap();
    // The left half exists only for a two-page master.
    assert!(e.execute(Command::AddShape { page: master_surface(0, true), shape: text_box("x") }).is_err());
}

#[test]
fn two_page_masters_show_the_left_half_on_even_pages() {
    let mut e = three_pages();
    e.execute(Command::EnsureMaster).unwrap();
    round_trip(&mut e, Command::SetTwoPageMaster { master: 0, two_page: true });
    e.execute(Command::AddShape { page: master_surface(0, true), shape: text_box("LEFT") }).unwrap();
    e.execute(Command::AddShape { page: master_surface(0, false), shape: text_box("RIGHT") }).unwrap();
    let text = |s: &Shape| match &s.kind {
        ShapeKind::TextBox { paragraphs } => paragraphs[0].text(),
        _ => String::new(),
    };
    assert_eq!(text(&e.doc().master_shapes(0)[0]), "RIGHT");
    assert_eq!(text(&e.doc().master_shapes(1)[0]), "LEFT");
    assert_eq!(text(&e.doc().master_shapes(2)[0]), "RIGHT");
    // A duplicate of a two-page master keeps both halves.
    e.execute(Command::AddMaster { copy_of: Some(0), id: None, description: String::new(), two_page: true })
        .unwrap();
    assert_eq!(e.doc().masters[1].left_shapes.len(), 1);
    // Turning it off drops the left half.
    round_trip(&mut e, Command::SetTwoPageMaster { master: 0, two_page: false });
    assert!(e.doc().masters[0].left_shapes.is_empty());
    assert_eq!(text(&e.doc().master_shapes(1)[0]), "RIGHT");
}

#[test]
fn page_size_and_margins_follow_into_the_masters() {
    let mut e = three_pages();
    e.execute(Command::EnsureMaster).unwrap();
    let surface = master_surface(0, false);
    e.execute(Command::AddShape { page: surface, shape: text_box("x") }).unwrap();
    let before = e.doc().masters[0].shapes[0].rect;
    round_trip(
        &mut e,
        Command::SetPageSetup {
            width_emu: 7_772_400 + 914_400,
            height_emu: 10_058_400,
            margins: kernpage_core::Margins::all(457_200 * 2),
        },
    );
    let m = &e.doc().masters[0];
    assert_eq!(m.shapes[0].rect.x, before.x + 457_200);
    assert_eq!(m.margins_emu, kernpage_core::Margins::all(914_400));
}

#[test]
fn new_pages_use_the_first_master() {
    let mut e = three_pages();
    add_master(&mut e);
    e.execute(Command::ApplyMaster { pages: vec![0], master: Some(1) }).unwrap();
    e.execute(Command::InsertPage { after: 0 }).unwrap();
    assert_eq!(e.doc().pages[1].master, Some(0));
}

#[test]
fn kern_round_trip_keeps_masters_pages_and_flags() {
    let mut e = three_pages();
    add_master(&mut e);
    e.execute(Command::SetMasterInfo { master: 1, id: 'F', name: "Front".into(), description: "Cover".into() })
        .unwrap();
    e.execute(Command::AddShape { page: master_surface(1, false), shape: text_box("Front matter") }).unwrap();
    e.execute(Command::SetTwoPageMaster { master: 1, two_page: true }).unwrap();
    e.execute(Command::AddShape { page: master_surface(1, true), shape: text_box("Left") }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![1], master: Some(1) }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![2], master: None }).unwrap();
    let bytes = write_kern(e.doc()).unwrap();
    let back = read_kern(&bytes).unwrap();
    assert_eq!(back.masters, e.doc().masters);
    let flags = |d: &Document| d.pages.iter().map(|p| (p.master, p.ignore_master)).collect::<Vec<_>>();
    assert_eq!(flags(&back), flags(e.doc()));
    // The manifest says 1.1 with the optional feature; a document without masters stays 1.0.
    let manifest = |b: &[u8]| -> serde_json::Value {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(b)).unwrap();
        let f = z.by_name("manifest.json").unwrap();
        serde_json::from_reader(f).unwrap()
    };
    let m = manifest(&bytes);
    assert_eq!(m["format_version"], "1.1");
    assert_eq!(m["minimum_reader_version"], "1.0");
    assert_eq!(m["optional_features"], serde_json::json!(["master-pages"]));
    let plain = write_kern(three_pages().doc()).unwrap();
    assert_eq!(manifest(&plain)["format_version"], "1.0");
}

#[test]
fn an_old_kern_master_without_an_id_gets_one() {
    let mut doc = three_pages().doc().clone();
    let mut m = MasterPage::new('\0', kernpage_core::Margins::all(457_200));
    m.name = String::new();
    doc.masters.push(m.clone());
    doc.masters.push(m);
    for p in &mut doc.pages {
        p.master = Some(1);
    }
    let back = read_kern(&write_kern(&doc).unwrap()).unwrap();
    assert_eq!(back.masters.iter().map(|m| m.id).collect::<Vec<_>>(), ['A', 'B']);
    assert_eq!(back.masters[1].name, "Master Page B");
}

// ---- the .pub save planner ------------------------------------------------------------

fn open_pub(name: &str) -> (SaveSource, Editor, Vec<u8>) {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen").join(name);
    let bytes = std::fs::read(kernpage_samples::resolve(p)).unwrap();
    let src = pub_format::read_bytes(&bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    (SaveSource::new(bytes.clone(), &src, &model, &breaks), Editor::new(model), bytes)
}

fn plan_says(source: &SaveSource, e: &Editor, needle: &str) {
    let plan = source.plan(e.doc());
    assert!(
        plan.unsupported.iter().any(|u| u.contains(needle)),
        "{needle:?} not in {:?}",
        plan.unsupported
    );
}

#[kernpage_samples::test]
fn an_untouched_file_with_masters_saves_byte_for_byte_like_before() {
    let (source, e, bytes) = open_pub("master-pn-2p.pub");
    assert!(!e.doc().masters.is_empty());
    assert!(e.doc().masters[0].id.is_alphanumeric());
    assert!(source.plan(e.doc()).unsupported.is_empty());
    let (out, plan) = source.build(e.doc()).unwrap();
    assert!(plan.unsupported.is_empty());
    // Only the save counter may differ from the original.
    assert_eq!(out.len(), bytes.len());
}

#[kernpage_samples::test]
fn the_pub_save_refuses_master_changes_it_cannot_write() {
    let (source, mut e, _) = open_pub("master-pn-2p.pub");
    let surface = master_surface(0, false);
    // Objects on a master.
    e.execute(Command::AddShape { page: surface, shape: text_box("x") }).unwrap();
    plan_says(&source, &e, "objects on master pages");
    e.undo();
    assert!(source.plan(e.doc()).unsupported.is_empty());
    // Renaming, two-page.
    e.execute(Command::SetMasterInfo { master: 0, id: 'A', name: "Other".into(), description: String::new() })
        .unwrap();
    plan_says(&source, &e, "renaming master pages");
    e.execute(Command::SetTwoPageMaster { master: 0, two_page: true }).unwrap();
    plan_says(&source, &e, "two-page master pages");
    // Adding.
    let (source, mut e, _) = open_pub("master-pn-2p.pub");
    add_master(&mut e);
    plan_says(&source, &e, "adding master pages");
    // Applying another master / ignoring.
    let (source, mut e, _) = open_pub("master-pn-2p.pub");
    add_master(&mut e);
    e.execute(Command::ApplyMaster { pages: vec![0], master: Some(1) }).unwrap();
    plan_says(&source, &e, "other than the first");
    e.execute(Command::ApplyMaster { pages: vec![0], master: Some(0) }).unwrap();
    e.execute(Command::ApplyMaster { pages: vec![0], master: None }).unwrap();
    plan_says(&source, &e, "turning off the master page");
    // And deleting.
    let (source, mut e, _) = open_pub("master-pn-2p.pub");
    add_master(&mut e);
    e.execute(Command::DeleteMaster { master: 0 }).unwrap();
    plan_says(&source, &e, "master pages");
}

#[kernpage_samples::test]
fn adding_a_page_to_a_file_with_masters_is_still_supported() {
    let (source, mut e, _) = open_pub("master-pn-2p.pub");
    e.execute(Command::InsertPage { after: 0 }).unwrap();
    let plan = source.plan(e.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
}
