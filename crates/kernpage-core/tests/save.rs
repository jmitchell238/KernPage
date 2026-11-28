//! The save pipeline without any GUI: open a sample, edit the model, build the bytes,
//! read them back with `pub_format`.

use kernpage_core::{
    from_pub_flowed, Command, Document, Editor, Paragraph, Rect, SaveSource, Shape, ShapeId,
    ShapeKind,
};

fn sample(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen");
    std::fs::read(kernpage_samples::resolve(p.join(name))).unwrap()
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    let source = SaveSource::new(bytes.to_vec(), &src, &model, &breaks);
    (source, Editor::new(model))
}

fn find_text(doc: &Document, needle: &str) -> (usize, ShapeId) {
    for (pi, p) in doc.pages.iter().enumerate() {
        for s in &p.shapes {
            if let ShapeKind::TextBox { paragraphs } = &s.kind {
                if paragraphs.iter().any(|q| q.text().contains(needle)) {
                    return (pi, s.id);
                }
            }
        }
    }
    panic!("no text box containing {needle:?}");
}

/// Replace the text of a text box the way the editor does, keeping its formatting.
fn retext(doc: &Document, page: usize, id: ShapeId, text: &str) -> Command {
    let ShapeKind::TextBox { paragraphs } = &doc.shape(page, id).unwrap().kind else {
        panic!("not a text box");
    };
    Command::SetText {
        page,
        id,
        paragraphs: kernpage_core::text::rebuild_paragraphs(paragraphs, text),
    }
}

fn shape_text(s: &pub_format::Shape) -> String {
    s.text.clone().unwrap_or_default().replace('\r', "\n")
}

#[kernpage_samples::test]
fn unchanged_document_has_empty_plan() {
    let bytes = sample("apple-banana.pub");
    let (source, ed) = open(&bytes);
    assert!(source.plan(ed.doc()).is_empty());
}

#[kernpage_samples::test]
fn apple_banana_move_and_edit() {
    let bytes = sample("apple-banana.pub");
    let orig = pub_format::read_bytes(&bytes).unwrap();
    let (source, mut ed) = open(&bytes);
    let (page, apple) = find_text(ed.doc(), "Apple");
    let rect = ed.doc().shape(page, apple).unwrap().rect;
    let new_rect = Rect::new(rect.x + 457_200, rect.y + 228_600, rect.w, rect.h);
    ed.execute(Command::SetRect {
        page,
        id: apple,
        rect: new_rect,
    })
    .unwrap();
    let c = retext(ed.doc(), page, apple, "Apple pie");
    ed.execute(c).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(plan.frames.len(), 1);
    assert_eq!(plan.stories.len(), 1);

    let back = pub_format::read_bytes(&out).unwrap();
    let (w, h) = (
        i64::from(back.page_width_emu),
        i64::from(back.page_height_emu),
    );
    let mut seen = 0;
    for (pi, p) in back.pages.iter().enumerate() {
        for (si, s) in p.shapes.iter().enumerate() {
            let before = &orig.pages[pi].shapes[si];
            if shape_text(s).contains("Apple pie") {
                seen += 1;
                assert_eq!(w / 2 + i64::from(s.left_emu), new_rect.x);
                assert_eq!(h / 2 + i64::from(s.top_emu), new_rect.y);
                assert_eq!(i64::from(s.right_emu - s.left_emu), new_rect.w);
            } else {
                // Everything else (Banana) is untouched.
                assert_eq!(s.left_emu, before.left_emu);
                assert_eq!(s.top_emu, before.top_emu);
                assert_eq!(shape_text(s), shape_text(before));
            }
        }
    }
    assert_eq!(seen, 1);
    assert!(back.text.contains("Banana"));
}

#[kernpage_samples::test]
fn bulletin_edit_order_of_service() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    let new: Vec<Paragraph> = paragraphs
        .iter()
        .map(|p| {
            let mut q = p.clone();
            for r in &mut q.runs {
                r.text = r.text.replace("Order of Service", "Order of Worship");
            }
            q
        })
        .collect();
    ed.execute(Command::SetText {
        page,
        id,
        paragraphs: new,
    })
    .unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    // Mixed formatting survives a text edit, so nothing is flagged.
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let back = pub_format::read_bytes(&out).unwrap();
    assert!(back.text.contains("Order of Worship"));
    assert!(!back.text.contains("Order of Service"));
    let sh = back
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| {
            s.text
                .as_deref()
                .is_some_and(|t| t.starts_with("Order of Worship"))
        })
        .unwrap();
    assert!(sh.runs[0].bold && (sh.runs[0].size_pt - 16.0).abs() < 0.01);
    assert!(sh.runs[1..]
        .iter()
        .all(|r| (r.size_pt - 10.0).abs() < 0.01 && !r.bold));
    // Other stories survive.
    let orig = pub_format::read_bytes(&bytes).unwrap();
    assert_eq!(back.stories.len(), orig.stories.len());
}

#[kernpage_samples::test]
fn linked_chain_edit_in_second_frame() {
    let bytes = sample("linked.pub");
    let orig = pub_format::read_bytes(&bytes).unwrap();
    let (source, mut ed) = open(&bytes);
    // The second frame of the first chain.
    let chain = orig.chains().into_iter().find(|c| c.len() > 1).unwrap();
    let second = &chain[1];
    let id = ed.doc().pages[second.page].shapes[second.shape].id;
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(second.page, id).unwrap().kind else {
        unreachable!()
    };
    let mut new = paragraphs.clone();
    assert!(!new.is_empty());
    let r = &mut new[0].runs[0];
    r.text = format!("ZZEDITED {}", r.text);
    ed.execute(Command::SetText {
        page: second.page,
        id,
        paragraphs: new,
    })
    .unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(plan.stories.len(), 1, "one story set once");
    let back = pub_format::read_bytes(&out).unwrap();
    assert!(back.text.contains("ZZEDITED"));
    // The text of the first frame is still there, in front of the edit.
    let first_word = orig.text.split_whitespace().next().unwrap();
    assert!(back.text.contains(first_word));
    assert!(back.text.find(first_word) < back.text.find("ZZEDITED"));
}

#[kernpage_samples::test]
fn deleting_a_shape_is_reported_and_supported_changes_still_save() {
    let bytes = sample("apple-banana.pub");
    let (source, mut ed) = open(&bytes);
    let (page, apple) = find_text(ed.doc(), "Apple");
    let (_, banana) = find_text(ed.doc(), "Banana");
    let c = retext(ed.doc(), page, apple, "Apricot");
    ed.execute(c).unwrap();
    ed.execute(Command::DeleteShapes {
        page,
        ids: vec![banana],
    })
    .unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(plan.unsupported, vec!["deleted objects".to_owned()]);
    assert_eq!(plan.stories.len(), 1);
    let back = pub_format::read_bytes(&out).unwrap();
    assert!(back.text.contains("Apricot"));
    assert!(
        back.text.contains("Banana"),
        "deleted shape stays in the file"
    );
}

#[kernpage_samples::test]
fn new_shapes_pages_and_formatting_are_unsupported() {
    let bytes = sample("apple-banana.pub");
    let (mut source, mut ed) = open(&bytes);
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::new(Rect::new(0, 0, 100, 100), ShapeKind::Ellipse),
    })
    .unwrap();
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    let (page, apple) = find_text(ed.doc(), "Apple");
    ed.execute(Command::SetTextFormat {
        page,
        ids: vec![apple],
        format: kernpage_core::TextFormat::Color(kernpage_core::Rgb(200, 0, 0)),
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    // An added page is saved (spec 4P); nothing is left unsupported.
    assert_eq!(
        plan.page_ops,
        vec![kernpage_core::PageOp::Add { after: Some(0) }]
    );
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert!(plan.frames.is_empty() && plan.stories.is_empty());
    // After a save the file matches the baseline again for what was supported.
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out, ed.doc(), &plan);
}

#[kernpage_samples::test]
fn mark_saved_clears_supported_changes() {
    let bytes = sample("apple-banana.pub");
    let (mut source, mut ed) = open(&bytes);
    let (page, apple) = find_text(ed.doc(), "Apple");
    let r = ed.doc().shape(page, apple).unwrap().rect;
    ed.execute(Command::MoveShapes {
        page,
        ids: vec![apple],
        dx: 91_440,
        dy: 0,
    })
    .unwrap();
    let c = retext(ed.doc(), page, apple, "Avocado");
    ed.execute(c).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out.clone(), ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    assert_eq!(source.original_bytes(), &out[..]);
    // A second edit saves on top of the first.
    ed.execute(Command::MoveShapes {
        page,
        ids: vec![apple],
        dx: 91_440,
        dy: 0,
    })
    .unwrap();
    let (out2, _) = source.build(ed.doc()).unwrap();
    let back = pub_format::read_bytes(&out2).unwrap();
    let w = i64::from(back.page_width_emu);
    let s = back
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| shape_text(s).contains("Avocado"))
        .unwrap();
    assert_eq!(w / 2 + i64::from(s.left_emu), r.x + 2 * 91_440);
}

#[kernpage_samples::test]
fn write_atomic_backs_up_and_replaces() {
    let dir = std::env::temp_dir().join(format!("pw-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("x.pub");
    std::fs::write(&path, b"old").unwrap();
    kernpage_core::write_atomic(&path, b"new", true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
    assert_eq!(std::fs::read(dir.join("x.pub.bak")).unwrap(), b"old");
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Writes the files used for the manual Publisher check (`out/save-test3`).
/// Run with `cargo test -p kernpage-core --test save -- --ignored write_publisher_samples`.
#[kernpage_samples::test]
#[ignore]
fn write_publisher_samples() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test3");
    std::fs::create_dir_all(&out).unwrap();

    let bytes = sample("apple-banana.pub");
    let (source, mut ed) = open(&bytes);
    let (page, apple) = find_text(ed.doc(), "Apple");
    let r = ed.doc().shape(page, apple).unwrap().rect;
    ed.execute(Command::SetRect {
        page,
        id: apple,
        rect: Rect::new(r.x + 457_200, r.y + 228_600, r.w, r.h),
    })
    .unwrap();
    let c = retext(ed.doc(), page, apple, "Apple pie");
    ed.execute(c).unwrap();
    let (bytes, _) = source.build(ed.doc()).unwrap();
    std::fs::write(out.join("apple-edit.pub"), bytes).unwrap();
    std::fs::write(out.join("apple-orig.pub"), sample("apple-banana.pub")).unwrap();

    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let c = retext(ed.doc(), page, id, "Order of Worship\nPrelude\nHymn");
    ed.execute(c).unwrap();
    let (bytes, _) = source.build(ed.doc()).unwrap();
    std::fs::write(out.join("bulletin-edit.pub"), bytes).unwrap();
    std::fs::write(out.join("bulletin-orig.pub"), sample("bulletin.pub")).unwrap();
}

#[kernpage_samples::test]
fn bulletin_appended_paragraph_keeps_body_style() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    let mut text = paragraphs
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n");
    text.push_str("\nClosing hymn");
    let c = retext(ed.doc(), page, id, &text);
    ed.execute(c).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let back = pub_format::read_bytes(&out).unwrap();
    let sh = back
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| {
            s.text
                .as_deref()
                .is_some_and(|t| t.starts_with("Order of Service"))
        })
        .unwrap();
    assert!(sh.text.as_deref().unwrap().ends_with("Closing hymn\r"));
    assert!(sh.runs[0].bold && (sh.runs[0].size_pt - 16.0).abs() < 0.01);
    let last = sh.runs.last().unwrap();
    assert!(!last.bold && (last.size_pt - 10.0).abs() < 0.01);
}

/// Files for the manual Publisher check of kept formatting (`out/save-test4`).
/// Run with `cargo test -p kernpage-core --test save -- --ignored write_keep_format_samples`.
#[kernpage_samples::test]
#[ignore]
fn write_keep_format_samples() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test4");
    std::fs::create_dir_all(&out).unwrap();
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    let mut lines: Vec<String> = paragraphs.iter().map(Paragraph::text).collect();
    lines[2] = "Hymn of praise and thanksgiving".into();
    lines.push("Closing hymn".into());
    let c = retext(ed.doc(), page, id, &lines.join("\n"));
    ed.execute(c).unwrap();
    let (bytes, _) = source.build(ed.doc()).unwrap();
    std::fs::write(out.join("bulletin-keep.pub"), bytes).unwrap();

    let bytes = sample("formatting.pub");
    let (source, mut ed) = open(&bytes);
    let mut target = None;
    for (pi, p) in ed.doc().pages.iter().enumerate() {
        for sh in &p.shapes {
            if let ShapeKind::TextBox { paragraphs } = &sh.kind {
                if paragraphs.len() > 2 && target.is_none() {
                    target = Some((pi, sh.id));
                }
            }
        }
    }
    let (page, id) = target.unwrap();
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    let mut lines: Vec<String> = paragraphs.iter().map(Paragraph::text).collect();
    let mid = lines.len() / 2;
    lines[mid].push_str(" EDITED");
    let c = retext(ed.doc(), page, id, &lines.join("\n"));
    ed.execute(c).unwrap();
    let (bytes, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    std::fs::write(out.join("formatting-keep.pub"), bytes).unwrap();
}

/// Move and resize every shape of `name` page 0 through the editor, save, read back: the
/// visual frames equal the edited rectangles within 1 EMU.
fn move_every_shape(name: &str, only: impl Fn(&pub_format::Shape) -> bool) -> usize {
    let bytes = sample(&format!("{name}.pub"));
    let orig = pub_format::read_bytes(&bytes).unwrap();
    let (source, mut ed) = open(&bytes);
    let ids: Vec<(usize, ShapeId, Rect)> = ed.doc().pages[0]
        .shapes
        .iter()
        .enumerate()
        .filter(|(i, _)| only(&orig.pages[0].shapes[*i]))
        .map(|(_, s)| (0, s.id, s.rect))
        .collect();
    let mut want = Vec::new();
    for (k, (page, id, r)) in ids.iter().enumerate() {
        let new = Rect::new(
            r.x + 111_111 * (k as i64 + 1),
            r.y + 77_777,
            r.w + 123_457,
            r.h + 54_321,
        );
        ed.execute(Command::SetRect {
            page: *page,
            id: *id,
            rect: new,
        })
        .unwrap();
        want.push(new);
    }
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(plan.frames.len(), ids.len());
    let back = pub_format::read_bytes(&out).unwrap();
    let (pw, ph) = (
        i64::from(back.page_width_emu),
        i64::from(back.page_height_emu),
    );
    let moved: Vec<usize> = ed.doc().pages[0]
        .shapes
        .iter()
        .enumerate()
        .filter(|(_, s)| ids.iter().any(|(_, id, _)| *id == s.id))
        .map(|(i, _)| i)
        .collect();
    for (i, w) in moved.iter().zip(&want) {
        let s = &back.pages[0].shapes[*i];
        let got = [
            pw / 2 + i64::from(s.left_emu),
            ph / 2 + i64::from(s.top_emu),
            i64::from(s.right_emu - s.left_emu),
            i64::from(s.bottom_emu - s.top_emu),
        ];
        for (g, e) in got.iter().zip([w.x, w.y, w.w, w.h]) {
            assert!((g - e).abs() <= 1, "{name} shape {i}: {got:?} vs {w:?}");
        }
        assert_eq!(s.rotation_deg, orig.pages[0].shapes[*i].rotation_deg);
    }
    // After saving, nothing is left to save.
    let mut source = source;
    source.mark_saved(out, ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    moved.len()
}

#[kernpage_samples::test]
fn rotated_shapes_move_and_resize() {
    assert_eq!(move_every_shape("rotate", |_| true), 4);
}

#[kernpage_samples::test]
fn group_children_move_and_resize() {
    // The group shape itself (kind 48) is not edited here; its two children are.
    assert_eq!(move_every_shape("group", |s| s.group.is_some()), 2);
}

#[kernpage_samples::test]
fn a_very_long_story_saves_through_the_editor() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let text: String = (0..200)
        .map(|i| format!("Order line number {i} with some words"))
        .collect::<Vec<_>>()
        .join("\n");
    let c = retext(ed.doc(), page, id, &text);
    ed.execute(c).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let back = pub_format::read_bytes(&out).unwrap();
    assert!(back.text.contains("Order line number 199 with some words"));
    assert_eq!(
        back.stories.len(),
        pub_format::read_bytes(&bytes).unwrap().stories.len()
    );
}

/// Writes the files used for the manual Publisher check (`out/save-test5`).
/// Run with `cargo test -p kernpage-core --test save -- --ignored write_save5_samples`.
#[kernpage_samples::test]
#[ignore]
fn write_save5_samples() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test5");
    std::fs::create_dir_all(&out).unwrap();

    // Task 1: a 200-paragraph story with alternating formatting (several FDP pages).
    let bytes = sample("bulletin.pub");
    let src = pub_format::read_bytes(&bytes).unwrap();
    let k = src
        .stories
        .iter()
        .position(|s| s.starts_with("Order of Service"))
        .unwrap();
    let recs = pub_write::story_records(&bytes, k).unwrap();
    let (nc, np) = (recs.char_records.len(), recs.para_records.len());
    let paras: Vec<pub_write::NewParagraph> = (0..200)
        .map(|i| pub_write::NewParagraph {
            runs: vec![
                pub_write::NewRun {
                    text: format!("Item {i} "),
                    char_record: pub_write::CharRecordRef::Original(i % nc),
                },
                pub_write::NewRun {
                    text: format!("detail {i}"),
                    char_record: pub_write::CharRecordRef::Original((i + 1) % nc),
                },
            ],
            para_record: pub_write::ParaRecordRef::Original(i % np),
        })
        .collect();
    std::fs::write(
        out.join("growth-alt.pub"),
        pub_write::set_story_runs(&bytes, k, &paras).unwrap(),
    )
    .unwrap();
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let text: String = (0..200)
        .map(|i| format!("Order line number {i} with some words"))
        .collect::<Vec<_>>()
        .join("\n");
    let c = retext(ed.doc(), page, id, &text);
    ed.execute(c).unwrap();
    std::fs::write(
        out.join("growth-editor.pub"),
        source.build(ed.doc()).unwrap().0,
    )
    .unwrap();

    // Tasks 2 and 3: move and resize through the editor.
    for (name, out_name) in [
        ("rotate", "rotate-moved"),
        ("group", "group-moved"),
        ("linked", "linked-resized"),
        ("columns", "columns-resized"),
    ] {
        let bytes = sample(&format!("{name}.pub"));
        let orig = pub_format::read_bytes(&bytes).unwrap();
        let (source, mut ed) = open(&bytes);
        let shapes: Vec<(ShapeId, Rect)> = ed.doc().pages[0]
            .shapes
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let s = &orig.pages[0].shapes[*i];
                s.kind != 48 && (name != "group" || s.group.is_some())
            })
            .map(|(_, s)| (s.id, s.rect))
            .collect();
        for (id, r) in shapes {
            ed.execute(Command::SetRect {
                page: 0,
                id,
                rect: Rect::new(r.x + 228_600, r.y + 152_400, r.w + 152_400, r.h + 76_200),
            })
            .unwrap();
        }
        let (bytes, plan) = source.build(ed.doc()).unwrap();
        assert!(plan.unsupported.is_empty());
        std::fs::write(out.join(format!("{out_name}.pub")), bytes).unwrap();
        std::fs::write(
            out.join(format!("{name}-orig.pub")),
            sample(&format!("{name}.pub")),
        )
        .unwrap();
    }
}

fn table_cells_of(bytes: &[u8]) -> Vec<String> {
    let d = pub_format::read_bytes(bytes).unwrap();
    d.pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find_map(|s| s.table.as_ref())
        .unwrap()
        .cells
        .clone()
}

/// Set the text of cell `index` in the (only) table of `doc`, keeping the cell's first run.
fn set_cell(doc: &mut Document, index: usize, text: &str) {
    for s in doc.pages.iter_mut().flat_map(|p| p.shapes.iter_mut()) {
        if let ShapeKind::Table { cells, .. } = &mut s.kind {
            let cell = &mut cells[index];
            let base = cell.paragraphs[0].clone();
            cell.paragraphs = text
                .split('\n')
                .map(|t| {
                    let mut p = base.clone();
                    p.runs = vec![kernpage_core::Run {
                        text: t.to_owned(),
                        ..base.base_run()
                    }];
                    p
                })
                .collect();
        }
    }
}

#[kernpage_samples::test]
fn table_cell_text_saves() {
    for name in ["table2.pub", "table.pub"] {
        let bytes = sample(name);
        let (mut source, ed) = open(&bytes);
        let before = table_cells_of(&bytes);
        let mut doc = ed.doc().clone();
        assert!(source.plan(&doc).is_empty());
        let last = before.len() - 1;
        set_cell(&mut doc, 0, "A longer first cell\nwith two paragraphs");
        set_cell(&mut doc, 1, "");
        set_cell(&mut doc, last, "short");
        let plan = source.plan(&doc);
        assert!(plan.unsupported.is_empty(), "{plan:?}");
        assert_eq!(plan.tables.len(), 1);
        let (out, plan) = source.build(&doc).unwrap();
        let mut want = before.clone();
        want[0] = "A longer first cell\rwith two paragraphs".into();
        want[1] = String::new();
        want[last] = "short".into();
        assert_eq!(table_cells_of(&out), want, "{name}");
        // The saved cells are now the baseline: nothing left to save.
        source.mark_saved(out, &doc, &plan);
        assert!(source.plan(&doc).is_empty());
        // A second edit on top of the first save.
        set_cell(&mut doc, 1, "again");
        let (out, _) = source.build(&doc).unwrap();
        want[1] = "again".into();
        assert_eq!(table_cells_of(&out), want, "{name}");
    }
}

#[kernpage_samples::test]
fn table_grid_changes_stay_unsupported() {
    let bytes = sample("table2.pub");
    let (source, ed) = open(&bytes);
    let mut doc = ed.doc().clone();
    for s in doc.pages.iter_mut().flat_map(|p| p.shapes.iter_mut()) {
        if let ShapeKind::Table { cells, .. } = &mut s.kind {
            cells.pop();
        }
    }
    assert_eq!(
        source.plan(&doc).unsupported,
        vec!["changes to table contents".to_owned()]
    );
}

#[kernpage_samples::test]
fn set_cell_text_command_saves_and_rereads() {
    let bytes = sample("table2.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .find_map(|(pi, p)| {
            p.shapes
                .iter()
                .find(|s| matches!(s.kind, ShapeKind::Table { .. }))
                .map(|s| (pi, s.id))
        })
        .unwrap();
    let ShapeKind::Table { cells, .. } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    let last = cells.len() - 1;
    for (cell, text) in [(0, "Edited\nsecond"), (last, "end")] {
        let ShapeKind::Table { cells, .. } = &ed.doc().shape(page, id).unwrap().kind else {
            unreachable!()
        };
        let paragraphs = kernpage_core::text::rebuild_paragraphs(&cells[cell].paragraphs, text);
        ed.execute(Command::SetCellText {
            page,
            id,
            cell,
            paragraphs,
        })
        .unwrap();
    }
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    // The second line may grow the row (pw-cok2): then the table is saved as a replacement.
    assert_eq!(plan.tables.len() + plan.table_rebuilds.len(), 1, "{plan:?}");
    let (out, _) = source.build(ed.doc()).unwrap();
    let got = table_cells_of(&out);
    assert_eq!(got[0], "Edited\rsecond");
    assert_eq!(got[last], "end");
}

/// `(text, bold, italic, underline, size, font)` runs and alignment per paragraph of the
/// text box whose text contains `needle`, read back from saved bytes.
type Para = (
    Vec<(String, bool, bool, bool, f32, String)>,
    pub_format::Alignment,
);

fn saved_paragraphs(bytes: &[u8], needle: &str) -> Vec<Para> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let s = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| s.text.as_deref().is_some_and(|t| t.contains(needle)))
        .unwrap();
    s.paragraphs
        .iter()
        .map(|p| {
            (
                p.runs
                    .iter()
                    .map(|r| {
                        (
                            r.text.replace('\r', ""),
                            r.bold,
                            r.italic,
                            r.underline,
                            r.size_pt,
                            r.font.clone(),
                        )
                    })
                    .filter(|r| !r.0.is_empty())
                    .collect(),
                p.alignment,
            )
        })
        .collect()
}

fn offset_of(doc: &Document, page: usize, id: ShapeId, needle: &str) -> usize {
    let ShapeKind::TextBox { paragraphs } = &doc.shape(page, id).unwrap().kind else {
        panic!()
    };
    let text = kernpage_core::text::paragraphs_to_text(paragraphs);
    text[..text.find(needle).unwrap()].chars().count()
}

#[kernpage_samples::test]
fn bulletin_formatting_saves_and_reads_back() {
    use kernpage_core::{Align, CharChange};
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let fmt = |ed: &mut Editor, needle: &str, len: usize, change| {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + len,
            change,
        })
        .unwrap();
    };
    fmt(&mut ed, "Prelude", 7, CharChange::Bold(true));
    fmt(&mut ed, "Hymn: Amazing", 4, CharChange::Italic(true));
    fmt(&mut ed, "Call to Worship", 15, CharChange::Size(18.0));
    fmt(
        &mut ed,
        "Call to Worship",
        15,
        CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, "Prayer", 6, CharChange::Underline(true));
    let at = offset_of(ed.doc(), page, id, "Call to Worship");
    ed.execute(Command::FormatParagraphs {
        page,
        id,
        cell: None,
        range: at..at + 15,
        align: Align::Center,
    })
    .unwrap();

    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.formats.len(), 1);
    assert!(plan.stories.is_empty(), "no text changed");
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(plan.formats.len(), 1);

    let want_model: Vec<Para> = {
        let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
            panic!()
        };
        paragraphs
            .iter()
            .map(|p| {
                (
                    kernpage_core::text::canonical(std::slice::from_ref(p))[0]
                        .runs
                        .iter()
                        .filter(|r| !r.text.is_empty())
                        .map(|r| {
                            (
                                r.text.clone(),
                                r.bold,
                                r.italic,
                                r.underline,
                                r.size_pt,
                                r.font.clone(),
                            )
                        })
                        .collect(),
                    match p.align {
                        Align::Left => pub_format::Alignment::Left,
                        Align::Center => pub_format::Alignment::Center,
                        Align::Right => pub_format::Alignment::Right,
                        Align::Justify => pub_format::Alignment::Justify,
                    },
                )
            })
            .collect()
    };
    assert_eq!(saved_paragraphs(&out, "Prelude"), want_model);

    // Re-open the saved file: the model equals the edited one, nothing is left to save.
    let (source2, ed2) = open(&out);
    assert!(source2.plan(ed2.doc()).is_empty());
    let (_, id2) = find_text(ed2.doc(), "Prelude");
    let ShapeKind::TextBox { paragraphs: reread } = &ed2.doc().shape(page, id2).unwrap().kind
    else {
        panic!()
    };
    let ShapeKind::TextBox { paragraphs: edited } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    assert_eq!(
        kernpage_core::text::canonical(reread),
        kernpage_core::text::canonical(edited)
    );

    // After the save the planner has nothing left, and a second save keeps the bytes valid.
    source.mark_saved(out.clone(), ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
}

#[kernpage_samples::test]
fn formatting_and_typing_together_save() {
    use kernpage_core::CharChange;
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let at = offset_of(ed.doc(), page, id, "Prelude");
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: None,
        range: at..at + 7,
        change: CharChange::Bold(true),
    })
    .unwrap();
    // Type into the bold word the way the editor does.
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    let text = kernpage_core::text::paragraphs_to_text(paragraphs).replace("Prelude", "Pre-lude");
    let cmd = retext(ed.doc(), page, id, &text);
    ed.execute(cmd).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let v = saved_paragraphs(&out, "Pre-lude");
    assert_eq!(v[1].0.len(), 1, "{:?}", v[1]);
    assert_eq!(v[1].0[0].0, "Pre-lude");
    assert!(v[1].0[0].1, "typed text inherits bold");
    assert!(!v[2].0[0].1);
}

#[kernpage_samples::test]
fn table_cell_formatting_saves() {
    use kernpage_core::CharChange;
    let bytes = sample("table2.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .find_map(|(pi, p)| {
            p.shapes
                .iter()
                .find(|s| matches!(s.kind, ShapeKind::Table { .. }))
                .map(|s| (pi, s.id))
        })
        .unwrap();
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: Some(4),
        range: 0..5,
        change: CharChange::Bold(true),
    })
    .unwrap();
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: Some(1),
        range: 0..2,
        change: CharChange::Size(20.0),
    })
    .unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.formats.len(), 1);
    let back = pub_format::read_bytes(&out).unwrap();
    let s = back.pages[page]
        .shapes
        .iter()
        .find(|s| s.table.is_some())
        .unwrap();
    let bold: Vec<&str> = s
        .runs
        .iter()
        .filter(|r| r.bold)
        .map(|r| r.text.trim_end_matches('\r'))
        .collect();
    assert!(bold.iter().any(|t| t.starts_with("First")), "{bold:?}");
    let big: Vec<&str> = s
        .runs
        .iter()
        .filter(|r| r.size_pt == 20.0)
        .map(|r| r.text.as_str())
        .collect();
    assert!(big.iter().any(|t| t.starts_with("R1")), "{big:?}");
    let (source2, ed2) = open(&out);
    assert!(source2.plan(ed2.doc()).is_empty());
}

/// Writes the formatted samples that were checked in Publisher 2010
/// (`out/save-test7`, fmt2-*.pub, see `verify.ps1` there). Run with `--ignored`.
#[kernpage_samples::test]
#[ignore = "writes sample files for manual checks in Publisher"]
fn write_format_samples_for_publisher() {
    use kernpage_core::{Align, CharChange};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test7");
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, bytes: &[u8]| std::fs::write(dir.join(name), bytes).unwrap();

    // bulletin: bold word, italic, underline, size + new font + centre, existing font.
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let fmt = |ed: &mut Editor, needle: &str, len: usize, change| {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + len,
            change,
        })
        .unwrap();
    };
    fmt(&mut ed, "Prelude", 7, CharChange::Bold(true));
    fmt(&mut ed, "Hymn: Amazing", 4, CharChange::Italic(true));
    fmt(&mut ed, "Call to Worship", 15, CharChange::Size(18.0));
    fmt(
        &mut ed,
        "Call to Worship",
        15,
        CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, "Prayer", 6, CharChange::Underline(true));
    fmt(
        &mut ed,
        "Benediction",
        11,
        CharChange::FontName("Times New Roman".into()),
    );
    fmt(&mut ed, "Benediction", 11, CharChange::Size(24.0));
    fmt(&mut ed, "Sermon", 6, CharChange::Bold(true));
    fmt(&mut ed, "Sermon", 6, CharChange::Italic(true));
    fmt(&mut ed, "Sermon", 6, CharChange::Underline(true));
    for (needle, align) in [
        ("Call to Worship", Align::Center),
        ("Scripture", Align::Right),
        ("Prayer", Align::Justify),
    ] {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatParagraphs {
            page,
            id,
            cell: None,
            range: at..at,
            align,
        })
        .unwrap();
    }
    write("fmt2-bulletin.pub", &source.build(ed.doc()).unwrap().0);

    // Formatting then typing inside the bold word.
    let (source, mut ed) = open(&bytes);
    let at = offset_of(ed.doc(), page, id, "Prelude");
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: None,
        range: at..at + 7,
        change: CharChange::Bold(true),
    })
    .unwrap();
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    let text = kernpage_core::text::paragraphs_to_text(paragraphs).replace("Prelude", "Pre-lude");
    let cmd = retext(ed.doc(), page, id, &text);
    ed.execute(cmd).unwrap();
    write("fmt2-typed.pub", &source.build(ed.doc()).unwrap().0);

    // formatting.pub: remove italic / underline / centre / right, add bold.
    let fbytes = sample("formatting.pub");
    let (source, mut ed) = open(&fbytes);
    let (page, id) = find_text(ed.doc(), "Times New Roman");
    let mut fmt2 = |needle: &str, len: usize, change| {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + len,
            change,
        })
        .unwrap();
    };
    fmt2("Times New Roman", 25, CharChange::Italic(false));
    fmt2("Courier New", 25, CharChange::Underline(false));
    fmt2("Calibri 14", 23, CharChange::Bold(false));
    fmt2("Arial 12", 8, CharChange::Bold(true));
    for needle in ["Centered", "Right aligned"] {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatParagraphs {
            page,
            id,
            cell: None,
            range: at..at,
            align: Align::Left,
        })
        .unwrap();
    }
    write("fmt2-formatting.pub", &source.build(ed.doc()).unwrap().0);

    // A table: bold one cell's word, bigger text in another, centred paragraph.
    let tbytes = sample("table2.pub");
    let (source, mut ed) = open(&tbytes);
    let (page, id) = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .find_map(|(pi, p)| {
            p.shapes
                .iter()
                .find(|s| matches!(s.kind, ShapeKind::Table { .. }))
                .map(|s| (pi, s.id))
        })
        .unwrap();
    for (cell, range, change) in [
        (4, 0..5, CharChange::Bold(true)),
        (1, 0..2, CharChange::Size(20.0)),
        (0, 0..5, CharChange::FontName("Georgia".into())),
    ] {
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: Some(cell),
            range,
            change,
        })
        .unwrap();
    }
    ed.execute(Command::FormatParagraphs {
        page,
        id,
        cell: Some(8),
        range: 0..0,
        align: Align::Center,
    })
    .unwrap();
    write("fmt2-table.pub", &source.build(ed.doc()).unwrap().0);
}

/// Writes one file per single formatting change on `bulletin.pub` into `out/hang2021`, to find
/// which change a viewer cannot open (`--ignored`).
#[kernpage_samples::test]
#[ignore = "writes out/hang2021 for manual Publisher checks"]
fn write_hang_bisect_variants() {
    use kernpage_core::{Align, CharChange, Rgb};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/hang2021");
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = sample("bulletin.pub");
    let runs: Vec<(&str, &str, usize, CharChange)> = vec![
        ("bold", "Prelude", 7, CharChange::Bold(true)),
        ("italic", "Hymn: Amazing", 4, CharChange::Italic(true)),
        ("underline", "Prayer", 6, CharChange::Underline(true)),
        ("size", "Call to Worship", 15, CharChange::Size(18.0)),
        (
            "font-new",
            "Call to Worship",
            15,
            CharChange::FontName("Georgia".into()),
        ),
        (
            "font-existing",
            "Call to Worship",
            15,
            CharChange::FontName("Calibri".into()),
        ),
        ("colour", "Prelude", 7, CharChange::Color(Rgb(200, 0, 0))),
    ];
    for (name, needle, len, change) in runs {
        let (source, mut ed) = open(&bytes);
        let (page, id) = find_text(ed.doc(), "Prelude");
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + len,
            change,
        })
        .unwrap();
        std::fs::write(
            dir.join(format!("v-{name}.pub")),
            source.build(ed.doc()).unwrap().0,
        )
        .unwrap();
    }
    for (name, needle, align) in [
        ("align-centre", "Call to Worship", Align::Center),
        ("align-right", "Scripture", Align::Right),
        ("align-justify", "Prayer", Align::Justify),
    ] {
        let (source, mut ed) = open(&bytes);
        let (page, id) = find_text(ed.doc(), "Prelude");
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatParagraphs {
            page,
            id,
            cell: None,
            range: at..at,
            align,
        })
        .unwrap();
        std::fs::write(
            dir.join(format!("v-{name}.pub")),
            source.build(ed.doc()).unwrap().0,
        )
        .unwrap();
    }
    // Spacing, edited in the model.
    let (source, ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let mut doc = ed.doc().clone();
    let shape = doc.pages[page]
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap();
    let ShapeKind::TextBox { paragraphs } = &mut shape.kind else {
        panic!()
    };
    let call = paragraphs
        .iter_mut()
        .find(|p| p.text().contains("Call to Worship"))
        .unwrap();
    call.spacing.space_before_pt = 12.0;
    call.spacing.space_after_pt = 20.0;
    call.spacing.line = kernpage_core::LineSpacing::Exactly(30.0);
    std::fs::write(dir.join("v-spacing.pub"), source.build(&doc).unwrap().0).unwrap();
    // Control: nothing changed.
    let (source, ed) = open(&bytes);
    std::fs::write(dir.join("v-control.pub"), source.build(ed.doc()).unwrap().0).unwrap();
}

#[kernpage_samples::test]
fn text_colour_and_paragraph_spacing_save_and_read_back() {
    use kernpage_core::{CharChange, Rgb};
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let at = offset_of(ed.doc(), page, id, "Prelude");
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: None,
        range: at..at + 7,
        change: CharChange::Color(Rgb(200, 0, 0)),
    })
    .unwrap();
    // Spacing has no editor command yet: edit the model directly.
    let mut doc = ed.doc().clone();
    let shape = doc.pages[page]
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap();
    let ShapeKind::TextBox { paragraphs } = &mut shape.kind else {
        panic!()
    };
    let call = paragraphs
        .iter_mut()
        .find(|p| p.text().contains("Call to Worship"))
        .unwrap();
    call.spacing.space_before_pt = 12.0;
    call.spacing.space_after_pt = 20.0;
    call.spacing.line = kernpage_core::LineSpacing::Exactly(30.0);
    let plan = source.plan(&doc);
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (out, _) = source.build(&doc).unwrap();
    let (source2, ed2) = open(&out);
    assert!(source2.plan(ed2.doc()).is_empty());
    let (_, id2) = find_text(ed2.doc(), "Prelude");
    let reread = |d: &Document, id: ShapeId| {
        let ShapeKind::TextBox { paragraphs } = &d.shape(page, id).unwrap().kind else {
            panic!()
        };
        kernpage_core::text::canonical(paragraphs)
    };
    assert_eq!(reread(ed2.doc(), id2), reread(&doc, id));
    let prelude = reread(ed2.doc(), id2)
        .into_iter()
        .find(|p| p.text().contains("Prelude"))
        .unwrap();
    assert!(prelude.runs.iter().any(|r| r.color == Rgb(200, 0, 0)));
}

/// Writes colour and spacing samples for checking in Publisher (`--ignored`).
#[kernpage_samples::test]
#[ignore = "writes out/save-test8 for manual Publisher checks"]
fn write_colour_spacing_samples_for_publisher() {
    use kernpage_core::{CharChange, Rgb};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/save-test8");
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    for (needle, len, c) in [
        ("Prelude", 7, Rgb(255, 0, 0)),
        ("Prayer", 6, Rgb(0, 0, 255)),
        ("Sermon", 6, Rgb(255, 128, 0)),
    ] {
        let at = offset_of(ed.doc(), page, id, needle);
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + len,
            change: CharChange::Color(c),
        })
        .unwrap();
    }
    let mut doc = ed.doc().clone();
    let shape = doc.pages[page]
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap();
    let ShapeKind::TextBox { paragraphs } = &mut shape.kind else {
        panic!()
    };
    let call = paragraphs
        .iter_mut()
        .find(|p| p.text().contains("Call to Worship"))
        .unwrap();
    call.spacing.space_before_pt = 18.0;
    call.spacing.space_after_pt = 24.0;
    call.spacing.line = kernpage_core::LineSpacing::Exactly(30.0);
    std::fs::write(dir.join("cs-bulletin.pub"), source.build(&doc).unwrap().0).unwrap();
}

#[kernpage_samples::test]
fn bulletin_paragraph_spacing_saves_and_reads_back() {
    use kernpage_core::text::SpacingChange;
    use kernpage_core::LineSpacing;
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let at = offset_of(ed.doc(), page, id, "Prelude");
    ed.execute(Command::SpacingParagraphs {
        page,
        id,
        cell: None,
        range: at..at,
        change: SpacingChange {
            before_pt: Some(12.0),
            line: Some(LineSpacing::Multiple(2.0)),
            ..Default::default()
        },
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");

    let doc = pub_format::read_bytes(&out).unwrap();
    let s = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| s.text.as_deref().is_some_and(|t| t.contains("Prelude")))
        .unwrap();
    let hit = s
        .paragraphs
        .iter()
        .find(|p| p.runs.iter().any(|r| r.text.contains("Prelude")))
        .unwrap();
    assert_eq!(hit.spacing.space_before_pt, 12.0);
    assert_eq!(hit.spacing.line, pub_format::LineSpacing::Multiple(2.0));
    // Only that paragraph changed.
    let changed = s
        .paragraphs
        .iter()
        .filter(|p| p.spacing.space_before_pt == 12.0)
        .count();
    assert_eq!(changed, 1);
    let (source2, ed2) = open(&out);
    assert!(source2.plan(ed2.doc()).is_empty());
}

#[kernpage_samples::test]
fn bulletin_stacking_order_saves_and_reads_back() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let ids: Vec<ShapeId> = ed.doc().pages[0].shapes.iter().map(|s| s.id).collect();
    assert!(ids.len() >= 3);
    let before: Vec<u32> = pub_format::read_bytes(&bytes).unwrap().pages[0]
        .shapes
        .iter()
        .map(|s| s.object_number)
        .collect();
    ed.execute(Command::BringToFront {
        page: 0,
        id: ids[0],
    })
    .unwrap();
    ed.execute(Command::SendToBack {
        page: 0,
        id: *ids.last().unwrap(),
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.orders.len(), 1);
    let (out, plan) = source.build(ed.doc()).unwrap();
    let after: Vec<u32> = pub_format::read_bytes(&out).unwrap().pages[0]
        .shapes
        .iter()
        .map(|s| s.object_number)
        .collect();
    let n = before.len();
    let mut want = vec![before[n - 1]];
    want.extend(&before[1..n - 1]);
    want.push(before[0]);
    assert_eq!(after, want);
    // Once saved, nothing is pending, and a later move still lands on the right shape.
    source.mark_saved(out, ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    let id = ids[1];
    let r = ed.doc().shape(0, id).unwrap().rect;
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: vec![id],
        dx: 914_400,
        dy: 0,
    })
    .unwrap();
    let (out2, _) = source.build(ed.doc()).unwrap();
    let moved = pub_format::read_bytes(&out2).unwrap();
    let s = moved.pages[0]
        .shapes
        .iter()
        .find(|s| s.object_number == before[1])
        .unwrap();
    let pw = ed.doc().pages[0].width_emu;
    assert_eq!(i64::from(s.left_emu) + pw / 2, r.x + 914_400);
}

#[kernpage_samples::test]
fn bulletin_align_to_margins_saves_as_frame_moves_only() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Order of Service");
    let before = ed.doc().shape(page, id).unwrap().rect;
    ed.execute(Command::AlignShapes {
        page,
        ids: vec![id],
        mode: kernpage_core::AlignMode::Bottom,
        to_margins: true,
    })
    .unwrap();
    assert_ne!(ed.doc().shape(page, id).unwrap().rect, before);
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert!(!plan.frames.is_empty(), "{plan:?}");
}

#[kernpage_samples::test]
fn bulletin_multi_move_and_align_have_no_unsupported_items() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let ids: Vec<ShapeId> = ed.doc().pages[0]
        .shapes
        .iter()
        .take(3)
        .map(|s| s.id)
        .collect();
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: ids.clone(),
        dx: 91_440,
        dy: 45_720,
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert!(!plan.frames.is_empty());
    ed.execute(Command::AlignShapes {
        page: 0,
        ids,
        mode: kernpage_core::AlignMode::Left,
        to_margins: false,
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
}

fn groups_of(bytes: &[u8]) -> Vec<(u32, Vec<u32>)> {
    let d = pub_format::read_bytes(bytes).unwrap();
    let s = &d.pages[0].shapes;
    s.iter()
        .filter(|g| s.iter().any(|c| c.group == Some(g.object_number)))
        .map(|g| {
            (
                g.object_number,
                s.iter()
                    .filter(|c| c.group == Some(g.object_number))
                    .map(|c| c.object_number)
                    .collect(),
            )
        })
        .collect()
}

#[kernpage_samples::test]
fn group_and_move_in_one_session_saves_and_regroups_after_save() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let ids: Vec<ShapeId> = ed.doc().pages[0].shapes.iter().map(|s| s.id).collect();
    let g = ed
        .execute(Command::Group {
            page: 0,
            ids: vec![ids[0], ids[1]],
        })
        .unwrap()[0];
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: vec![g],
        dx: 914_400,
        dy: 457_200,
    })
    .unwrap();
    let before = source.plan(ed.doc());
    assert!(before.unsupported.is_empty(), "{:?}", before.unsupported);
    assert_eq!(before.groups.len(), 1);
    let (out, plan) = source.build(ed.doc()).unwrap();
    let saved = pub_format::read_bytes(&out).unwrap();
    assert_eq!(groups_of(&out).len(), 1);
    assert_eq!(groups_of(&out)[0].1.len(), 2);
    let orig = pub_format::read_bytes(&bytes).unwrap();
    for (i, s) in orig.pages[0].shapes[..2].iter().enumerate() {
        let n = saved.pages[0]
            .shapes
            .iter()
            .find(|t| t.object_number == s.object_number)
            .unwrap();
        assert!((n.left_emu - s.left_emu - 914_400).abs() <= 1, "shape {i}");
        assert!((n.top_emu - s.top_emu - 457_200).abs() <= 1, "shape {i}");
    }
    source.mark_saved(out.clone(), ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    // Ungroup the group that was just saved, in the same session.
    ed.execute(Command::Ungroup { page: 0, id: g }).unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (out2, _) = source.build(ed.doc()).unwrap();
    assert!(groups_of(&out2).is_empty());
}

#[kernpage_samples::test]
fn ungrouping_an_existing_group_saves() {
    let bytes = sample("group.pub");
    let (source, mut ed) = open(&bytes);
    let g = ed.doc().pages[0]
        .shapes
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Group))
        .unwrap()
        .id;
    ed.execute(Command::Ungroup { page: 0, id: g }).unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (out, _) = source.build(ed.doc()).unwrap();
    assert!(groups_of(&out).is_empty());
}

#[kernpage_samples::test]
fn deleting_a_group_is_still_reported() {
    let bytes = sample("group.pub");
    let (source, mut ed) = open(&bytes);
    let g = ed.doc().pages[0]
        .shapes
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Group))
        .unwrap()
        .id;
    ed.execute(Command::DeleteShapes {
        page: 0,
        ids: vec![g],
    })
    .unwrap();
    assert!(source
        .plan(ed.doc())
        .unsupported
        .contains(&"deleted objects".to_string()));
}

#[kernpage_samples::test]
fn reordering_on_a_page_with_a_group_saves() {
    let bytes = sample("group.pub");
    let (source, mut ed) = open(&bytes);
    let before = groups_of(&bytes);
    let ids: Vec<ShapeId> = ed.doc().pages[0].shapes.iter().map(|s| s.id).collect();
    let g = ed.doc().pages[0]
        .shapes
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Group))
        .unwrap()
        .id;
    ed.execute(Command::ZOrderShapes {
        page: 0,
        ids: vec![g],
        op: kernpage_core::ZOrder::Front,
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert!(!ids.is_empty());
    let (out, _) = source.build(ed.doc()).unwrap();
    assert_eq!(groups_of(&out).len(), before.len());
    assert_eq!(groups_of(&out)[0].1.len(), before[0].1.len());
}

// --- New text boxes (spec 4L) ---

const PT: i64 = 12_700;

fn add_box(ed: &mut Editor, page: usize, r: Rect, text: &str) -> ShapeId {
    let paras: Vec<Paragraph> = text.split('\n').map(Paragraph::plain).collect();
    ed.execute(Command::AddShape {
        page,
        shape: Shape::text_box(r, paras),
    })
    .unwrap();
    ed.doc().pages[page].shapes.last().unwrap().id
}

fn boxes_with(bytes: &[u8], page: usize, text: &str) -> Vec<(f64, f64, f64, f64)> {
    let d = pub_format::read_bytes(bytes).unwrap();
    let (w, h) = (f64::from(d.page_width_emu), f64::from(d.page_height_emu));
    d.pages[page]
        .shapes
        .iter()
        .filter(|s| {
            s.text
                .as_deref()
                .is_some_and(|t| t.trim_end_matches('\r') == text)
        })
        .map(|s| {
            (
                (w / 2.0 + f64::from(s.left_emu)) / PT as f64,
                (h / 2.0 + f64::from(s.top_emu)) / PT as f64,
                f64::from(s.width_emu.unwrap_or(0)) / PT as f64,
                f64::from(s.height_emu.unwrap_or(0)) / PT as f64,
            )
        })
        .collect()
}

fn near(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    (a.0 - b.0).abs() <= 1.0
        && (a.1 - b.1).abs() <= 1.0
        && (a.2 - b.2).abs() <= 1.0
        && (a.3 - b.3).abs() <= 1.0
}

fn all_text(bytes: &[u8]) -> Vec<String> {
    pub_format::read_bytes(bytes)
        .unwrap()
        .pages
        .iter()
        .flat_map(|p| p.shapes.iter().filter_map(|s| s.text.clone()))
        .collect()
}

#[kernpage_samples::test]
fn new_text_box_page_one_saved() {
    let bytes = sample("bulletin.pub");
    let before = all_text(&bytes);
    let (source, mut ed) = open(&bytes);
    add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Hello",
    );
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.new_text_boxes.len(), 1);
    let (out, _) = source.build(ed.doc()).unwrap();
    let got = boxes_with(&out, 0, "Hello");
    assert_eq!(got.len(), 1);
    assert!(near(got[0], (72.0, 100.0, 200.0, 50.0)), "{got:?}");
    let after = all_text(&out);
    for t in &before {
        assert!(after.contains(t), "old text kept: {t:?}");
    }
}

#[kernpage_samples::test]
fn new_text_box_page_two_two_paragraphs_moved_before_save() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        1,
        Rect::new(50 * PT, 60 * PT, 180 * PT, 80 * PT),
        "One\nTwo",
    );
    ed.execute(Command::MoveShapes {
        page: 1,
        ids: vec![id],
        dx: 10 * PT,
        dy: 20 * PT,
    })
    .unwrap();
    let (out, _) = source.build(ed.doc()).unwrap();
    let d = pub_format::read_bytes(&out).unwrap();
    let hit = d.pages[1]
        .shapes
        .iter()
        .find(|s| s.text.as_deref().is_some_and(|t| t.starts_with("One\rTwo")))
        .expect("two-paragraph box");
    let (w, h) = (f64::from(d.page_width_emu), f64::from(d.page_height_emu));
    let l = (w / 2.0 + f64::from(hit.left_emu)) / PT as f64;
    let t = (h / 2.0 + f64::from(hit.top_emu)) / PT as f64;
    assert!(
        (l - 60.0).abs() <= 1.0 && (t - 80.0).abs() <= 1.0,
        "{l} {t}"
    );
}

#[kernpage_samples::test]
fn new_empty_and_two_boxes_and_save_twice() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    add_box(
        &mut ed,
        0,
        Rect::new(10 * PT, 10 * PT, 100 * PT, 30 * PT),
        "",
    );
    add_box(
        &mut ed,
        0,
        Rect::new(20 * PT, 300 * PT, 120 * PT, 40 * PT),
        "Second",
    );
    let n_before = pub_format::read_bytes(&bytes).unwrap().pages[0]
        .shapes
        .len();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(plan.new_text_boxes.len(), 2);
    assert_eq!(
        pub_format::read_bytes(&out).unwrap().pages[0].shapes.len(),
        n_before + 2
    );
    assert_eq!(boxes_with(&out, 0, "Second").len(), 1);
    source.mark_saved(out.clone(), ed.doc(), &plan);
    assert!(
        source.plan(ed.doc()).is_empty(),
        "{:?}",
        source.plan(ed.doc())
    );
    let (out2, plan2) = source.build(ed.doc()).unwrap();
    assert!(plan2.new_text_boxes.is_empty());
    assert_eq!(
        pub_format::read_bytes(&out2).unwrap().pages[0].shapes.len(),
        n_before + 2
    );
    // An existing edit still saves on top, and a third box adds only itself.
    add_box(
        &mut ed,
        0,
        Rect::new(30 * PT, 400 * PT, 90 * PT, 30 * PT),
        "Third",
    );
    let (out3, _) = source.build(ed.doc()).unwrap();
    assert_eq!(
        pub_format::read_bytes(&out3).unwrap().pages[0].shapes.len(),
        n_before + 3
    );
}

fn story_texts(bytes: &[u8]) -> Vec<String> {
    let mut v: Vec<String> = all_text(bytes)
        .iter()
        .map(|t| t.trim_end().to_string())
        .collect();
    v.sort();
    v
}

#[kernpage_samples::test]
fn new_box_text_edit_after_save() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Hello",
    );
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out.clone(), ed.doc(), &plan);
    let cmd = retext(ed.doc(), 0, id, "Hello again\nLine two");
    ed.execute(cmd).unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert!(plan.new_text_boxes.is_empty());
    let (out2, _) = source.build(ed.doc()).unwrap();
    let got = story_texts(&out2);
    assert!(got.iter().any(|t| t == "Hello again\rLine two"), "{got:?}");
    assert!(!got.iter().any(|t| t == "Hello"));
    let fr = boxes_with(&out2, 0, "Hello again\rLine two");
    assert_eq!(fr.len(), 1);
    assert!(near(fr[0], (72.0, 100.0, 200.0, 50.0)), "{fr:?}");
    // Old stories unchanged: everything but the new box matches the original.
    let mut old = story_texts(&bytes);
    old.push("Hello again\rLine two".into());
    old.sort();
    assert_eq!(story_texts(&out2), old);
}

#[kernpage_samples::test]
fn two_new_boxes_text_edit_after_save() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let a = add_box(
        &mut ed,
        0,
        Rect::new(10 * PT, 10 * PT, 100 * PT, 30 * PT),
        "Alpha",
    );
    let b = add_box(
        &mut ed,
        0,
        Rect::new(20 * PT, 300 * PT, 120 * PT, 40 * PT),
        "Beta",
    );
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out, ed.doc(), &plan);
    for (id, t) in [(a, "Alpha two"), (b, "Beta two")] {
        let cmd = retext(ed.doc(), 0, id, t);
        ed.execute(cmd).unwrap();
    }
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (out2, plan2) = source.build(ed.doc()).unwrap();
    source.mark_saved(out2.clone(), ed.doc(), &plan2);
    assert!(source.plan(ed.doc()).is_empty());
    let got = story_texts(&out2);
    assert!(got.iter().any(|t| t == "Alpha two") && got.iter().any(|t| t == "Beta two"));
    assert!(!got.iter().any(|t| t == "Alpha" || t == "Beta"));
    let mut old = story_texts(&bytes);
    old.extend(["Alpha two".into(), "Beta two".into()]);
    old.sort();
    assert_eq!(story_texts(&out2), old);
}

#[kernpage_samples::test]
fn new_box_bold_after_save() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        0,
        Rect::new(10 * PT, 10 * PT, 100 * PT, 30 * PT),
        "Bold me",
    );
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out, ed.doc(), &plan);
    ed.execute(Command::SetTextFormat {
        page: 0,
        ids: vec![id],
        format: kernpage_core::TextFormat::Bold(true),
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.formats.len(), 1);
    source.build(ed.doc()).unwrap();
}

/// The paragraphs of the text box holding `needle`, read back from saved bytes.
fn reopened(bytes: &[u8], needle: &str) -> Vec<Paragraph> {
    let (_, ed) = open(bytes);
    let (page, id) = find_text(ed.doc(), needle);
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        unreachable!()
    };
    paragraphs.clone()
}

fn fmt(
    ed: &mut Editor,
    id: ShapeId,
    range: std::ops::Range<usize>,
    change: kernpage_core::CharChange,
) {
    ed.execute(Command::FormatRuns {
        page: 0,
        id,
        cell: None,
        range,
        change,
    })
    .unwrap();
}

fn align(ed: &mut Editor, id: ShapeId, range: std::ops::Range<usize>, a: kernpage_core::Align) {
    ed.execute(Command::FormatParagraphs {
        page: 0,
        id,
        cell: None,
        range,
        align: a,
    })
    .unwrap();
}

/// Draw "Hello world", "Hello" bold, centred (the sample the Publisher check uses).
fn bold_centred(ed: &mut Editor) -> ShapeId {
    let id = add_box(
        ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Hello world",
    );
    fmt(ed, id, 0..5, kernpage_core::CharChange::Bold(true));
    align(ed, id, 0..0, kernpage_core::Align::Center);
    id
}

#[kernpage_samples::test]
fn new_box_bold_and_centred_saved_on_first_save() {
    let bytes = sample("bulletin.pub");
    let old = story_texts(&bytes);
    let (mut source, mut ed) = open(&bytes);
    bold_centred(&mut ed);
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (out, plan) = source.build(ed.doc()).unwrap();
    let paras = reopened(&out, "Hello world");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].align, kernpage_core::Align::Center);
    let runs: Vec<(&str, bool)> = paras[0]
        .runs
        .iter()
        .map(|r| (r.text.as_str(), r.bold))
        .collect();
    assert_eq!(runs, [("Hello", true), (" world", false)]);
    let mut want = old;
    want.push("Hello world".into());
    want.sort();
    assert_eq!(story_texts(&out), want);
    // Saving again adds nothing.
    source.mark_saved(out, ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    let (out2, _) = source.build(ed.doc()).unwrap();
    assert_eq!(story_texts(&out2), want);
}

#[kernpage_samples::test]
fn new_box_font_size_colour_and_two_aligned_paragraphs() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 80 * PT),
        "Left para\nRight para",
    );
    fmt(
        &mut ed,
        id,
        0..9,
        kernpage_core::CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, id, 0..9, kernpage_core::CharChange::Size(24.0));
    fmt(
        &mut ed,
        id,
        0..9,
        kernpage_core::CharChange::Color(kernpage_core::Rgb(200, 0, 0)),
    );
    align(&mut ed, id, 10..20, kernpage_core::Align::Right);
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let paras = reopened(&out, "Left para");
    assert_eq!(paras.len(), 2);
    let r = &paras[0].runs[0];
    assert_eq!(
        (r.font.as_str(), r.size_pt, r.color),
        ("Georgia", 24.0, kernpage_core::Rgb(200, 0, 0))
    );
    assert_eq!(paras[0].align, kernpage_core::Align::Left);
    assert_eq!(paras[1].align, kernpage_core::Align::Right);
    assert_ne!(paras[1].runs[0].font, "Georgia");
}

/// (font, size) of the run holding the final paragraph mark of the last story.
fn final_mark(bytes: &[u8], needle: &str) -> (String, f32) {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let shape = doc
        .pages
        .iter()
        .flat_map(|p| p.shapes.iter())
        .find(|s| s.text.as_deref().is_some_and(|t| t.contains(needle)))
        .unwrap();
    let r = shape.paragraphs.last().unwrap().runs.last().unwrap();
    assert!(r.text.ends_with('\r'), "{:?}", r.text);
    (r.font.clone(), r.size_pt)
}

/// Publisher (spec 4L.6): formatting part of a new box's text leaves its final mark at
/// the defaults; formatting the whole last paragraph also formats the mark.
#[kernpage_samples::test]
fn new_box_final_mark_follows_publisher() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Georgia text",
    );
    fmt(
        &mut ed,
        id,
        0..7,
        kernpage_core::CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, id, 0..7, kernpage_core::CharChange::Size(24.0));
    let (out, _) = source.build(ed.doc()).unwrap();
    let partial = final_mark(&out, "Georgia text");
    assert_ne!(partial.0, "Georgia");
    assert_ne!(partial.1, 24.0);

    let (source, mut ed) = open(&bytes);
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Georgia text",
    );
    fmt(
        &mut ed,
        id,
        0..12,
        kernpage_core::CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, id, 0..12, kernpage_core::CharChange::Size(24.0));
    let (out, _) = source.build(ed.doc()).unwrap();
    assert_eq!(
        final_mark(&out, "Georgia text"),
        ("Georgia".to_string(), 24.0)
    );
}

#[kernpage_samples::test]
fn new_box_other_shapes_unsupported() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::new(Rect::new(0, 0, 100, 100), ShapeKind::Rectangle),
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(plan.new_text_boxes.is_empty());
    assert_eq!(plan.new_shapes.len(), 1);
    assert!(plan.new_shapes[0].style.no_line);
    assert!(plan.unsupported.is_empty(), "{plan:?}");
}

/// Writes the files for the manual Publisher check of formatted new boxes
/// (`out/textbox-format`). Run with
/// `cargo test -p kernpage-core --test save -- --ignored write_textbox_format_samples`.
#[kernpage_samples::test]
#[ignore]
fn write_textbox_format_samples() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/textbox-format");
    std::fs::create_dir_all(&out).unwrap();
    let (source, mut ed) = open(&sample("bulletin.pub"));
    bold_centred(&mut ed);
    std::fs::write(
        out.join("bold-centred.pub"),
        source.build(ed.doc()).unwrap().0,
    )
    .unwrap();
    let (source, mut ed) = open(&sample("bulletin.pub"));
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Georgia text",
    );
    fmt(
        &mut ed,
        id,
        0..7,
        kernpage_core::CharChange::FontName("Georgia".into()),
    );
    fmt(&mut ed, id, 0..7, kernpage_core::CharChange::Size(24.0));
    fmt(
        &mut ed,
        id,
        0..7,
        kernpage_core::CharChange::Color(kernpage_core::Rgb(200, 0, 0)),
    );
    std::fs::write(
        out.join("font-size-colour.pub"),
        source.build(ed.doc()).unwrap().0,
    )
    .unwrap();
    let (source, mut ed) = open(&sample("bulletin.pub"));
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 80 * PT),
        "Left para\nRight para",
    );
    align(&mut ed, id, 10..20, kernpage_core::Align::Right);
    std::fs::write(
        out.join("two-paras-mixed.pub"),
        source.build(ed.doc()).unwrap().0,
    )
    .unwrap();
}

/// Writes the files for the manual Publisher check of new text boxes (`out/textbox-editor`).
/// Run with `cargo test -p kernpage-core --test save -- --ignored write_textbox_samples`.
#[kernpage_samples::test]
#[ignore]
fn write_textbox_samples() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/textbox-editor");
    std::fs::create_dir_all(&out).unwrap();
    for (name, page, rect, text) in [
        (
            "hello-p1.pub",
            0,
            Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
            "Hello",
        ),
        (
            "two-paras-p2.pub",
            1,
            Rect::new(60 * PT, 80 * PT, 180 * PT, 80 * PT),
            "First paragraph\nSecond paragraph",
        ),
    ] {
        let (source, mut ed) = open(&sample("bulletin.pub"));
        add_box(&mut ed, page, rect, text);
        let (bytes, _) = source.build(ed.doc()).unwrap();
        std::fs::write(out.join(name), bytes).unwrap();
    }
}

/// Writes the file for the manual Publisher check of editing a new box after saving it
/// (`out/textbox-edit-after-save`). Run with
/// `cargo test -p kernpage-core --test save -- --ignored write_edit_after_save_sample`.
#[kernpage_samples::test]
#[ignore]
fn write_edit_after_save_sample() {
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/textbox-edit-after-save");
    std::fs::create_dir_all(&out).unwrap();
    let (mut source, mut ed) = open(&sample("bulletin.pub"));
    let id = add_box(
        &mut ed,
        0,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 50 * PT),
        "Hello",
    );
    let (b1, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(b1, ed.doc(), &plan);
    let cmd = retext(ed.doc(), 0, id, "Hello again\nLine two");
    ed.execute(cmd).unwrap();
    let (b2, _) = source.build(ed.doc()).unwrap();
    std::fs::write(out.join("save-twice.pub"), b2).unwrap();
}

/// The document save counter (Contents object 256 item 60).
fn save_counter(bytes: &[u8]) -> u32 {
    let s = pub_format::Streams::from_cfb(bytes).unwrap();
    let c = pub_format::contents::Contents::parse(&s.contents).unwrap();
    let e = c.get(256).unwrap();
    let slot = pub_format::items::locate_int(&s.contents, e.offset, 60)
        .unwrap()
        .unwrap();
    u32::from_le_bytes(s.contents[slot.offset..slot.offset + 4].try_into().unwrap())
}

#[kernpage_samples::test]
fn save_counter_rises_on_the_first_save_of_a_session_only() {
    let bytes = sample("bulletin.pub");
    assert_eq!(save_counter(&bytes), 2);
    let (mut source, ed) = open(&bytes);

    // An unchanged file still counts as a session save, as in Publisher.
    let (first, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(save_counter(&first), 3);
    // Building without marking it saved (a failed write) does not use up the bump.
    let (again, _) = source.build(ed.doc()).unwrap();
    assert_eq!(save_counter(&again), 3);

    source.mark_saved(first.clone(), ed.doc(), &plan);
    let (second, plan2) = source.build(ed.doc()).unwrap();
    assert_eq!(save_counter(&second), 3);
    source.mark_saved(second, ed.doc(), &plan2);

    // A re-opened file is a new session.
    let (reopened, ed2) = open(&first);
    let (third, _) = reopened.build(ed2.doc()).unwrap();
    assert_eq!(save_counter(&third), 4);
}

// --- New rectangles, ovals and lines (spec 4N) ---

fn add_plain_shape(
    ed: &mut Editor,
    page: usize,
    kind: ShapeKind,
    r: Rect,
    fill: Option<kernpage_core::Rgb>,
    line: Option<kernpage_core::Stroke>,
) -> ShapeId {
    let mut s = Shape::new(r, kind);
    s.fill = fill;
    s.line = line;
    ed.execute(Command::AddShape { page, shape: s }).unwrap();
    ed.doc().pages[page].shapes.last().unwrap().id
}

fn editor_stroke() -> Option<kernpage_core::Stroke> {
    Some(kernpage_core::Stroke {
        color: kernpage_core::Rgb(0x1F, 0x4E, 0x79),
        width_emu: 2 * PT,
    })
}

#[kernpage_samples::test]
fn new_shapes_of_each_kind_are_saved_on_both_pages() {
    use kernpage_core::Rgb;
    let bytes = sample("bulletin.pub");
    let before = pub_format::read_bytes(&bytes).unwrap();
    for page in 0..2 {
        let (source, mut ed) = open(&bytes);
        let fill = Some(Rgb(0xC6, 0xD7, 0xF1));
        add_plain_shape(
            &mut ed,
            page,
            ShapeKind::Rectangle,
            Rect::new(72 * PT, 660 * PT, 150 * PT, 60 * PT),
            fill,
            editor_stroke(),
        );
        add_plain_shape(
            &mut ed,
            page,
            ShapeKind::Ellipse,
            Rect::new(300 * PT, 660 * PT, 100 * PT, 40 * PT),
            fill,
            editor_stroke(),
        );
        add_plain_shape(
            &mut ed,
            page,
            ShapeKind::Line,
            Rect::new(72 * PT, 700 * PT, 200 * PT, 40 * PT),
            None,
            editor_stroke(),
        );
        let plan = source.plan(ed.doc());
        assert!(plan.unsupported.is_empty(), "{plan:?}");
        assert_eq!(plan.new_shapes.len(), 3);
        let (out, _) = source.build(ed.doc()).unwrap();
        let d = pub_format::read_bytes(&out).unwrap();
        let old = before.pages[page].shapes.len();
        let new = &d.pages[page].shapes[old..];
        assert_eq!(new.len(), 3);
        use pub_format::ShapeGeometry as G;
        assert_eq!(new[0].geometry, Some(G::Rectangle));
        assert_eq!(new[1].geometry, Some(G::Ellipse));
        assert_eq!(new[2].geometry, Some(G::Line));
        assert_eq!(new[0].fill, Some(pub_format::Color::Rgb(0xC6, 0xD7, 0xF1)));
        assert_eq!(
            new[0].line,
            Some((pub_format::Color::Rgb(0x1F, 0x4E, 0x79), 25_400))
        );
        assert_eq!(new[2].fill, None);
        // The other page did not change.
        assert_eq!(
            d.pages[1 - page].shapes.len(),
            before.pages[1 - page].shapes.len()
        );
        // Re-opening gives the same kinds back.
        let (_, ed2) = open(&out);
        let kinds: Vec<_> = ed2.doc().pages[page].shapes[old..]
            .iter()
            .map(|s| s.kind.clone())
            .collect();
        assert_eq!(
            kinds,
            vec![ShapeKind::Rectangle, ShapeKind::Ellipse, ShapeKind::Line]
        );
    }
}

#[kernpage_samples::test]
fn publisher_default_style_writes_defaults() {
    let bytes = sample("bulletin.pub");
    let scheme0 = {
        let d = pub_format::read_bytes(&bytes).unwrap();
        let [r, g, b] = d.color_scheme.map_or([0, 0, 0], |c| c.colors[0]);
        kernpage_core::Rgb(r, g, b)
    };
    let (source, mut ed) = open(&bytes);
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 150 * PT, 60 * PT),
        None,
        Some(kernpage_core::Stroke {
            color: scheme0,
            width_emu: 9_525,
        }),
    );
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.new_shapes[0].style, pub_write::ShapeStyle::default());
    let (out, _) = source.build(ed.doc()).unwrap();
    // Identical to the writer's default call.
    let direct = pub_write::add_shape(
        &bytes,
        0,
        pub_write::NewShapeKind::Rectangle,
        72.0,
        660.0,
        150.0,
        60.0,
        pub_write::ShapeStyle::default(),
    )
    .unwrap();
    let (a, b) = (
        pub_format::Streams::from_cfb(&out).unwrap(),
        pub_format::Streams::from_cfb(&direct).unwrap(),
    );
    assert_eq!(a.escher, b.escher);
}

#[kernpage_samples::test]
fn literal_style_is_written_literally() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 150 * PT, 60 * PT),
        Some(kernpage_core::Rgb(255, 0, 0)),
        Some(kernpage_core::Stroke {
            color: kernpage_core::Rgb(0, 0, 255),
            width_emu: 3 * PT,
        }),
    );
    let plan = source.plan(ed.doc());
    assert_eq!(
        plan.new_shapes[0].style,
        pub_write::ShapeStyle {
            fill: Some([255, 0, 0]),
            line: Some([0, 0, 255]),
            line_weight_pt: Some(3.0),
            no_line: false
        }
    );
}

#[kernpage_samples::test]
fn mixed_new_objects_keep_creation_order_and_save_twice_adds_nothing() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let n0 = pub_format::read_bytes(&bytes).unwrap().pages[0]
        .shapes
        .len();
    let r = add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 100 * PT, 40 * PT),
        None,
        editor_stroke(),
    );
    let t = add_box(
        &mut ed,
        0,
        Rect::new(200 * PT, 660 * PT, 100 * PT, 30 * PT),
        "Mixed",
    );
    let l = add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Line,
        Rect::new(72 * PT, 720 * PT, 150 * PT, 0),
        None,
        editor_stroke(),
    );
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (out, plan) = source.build(ed.doc()).unwrap();
    let d = pub_format::read_bytes(&out).unwrap();
    let new = &d.pages[0].shapes[n0..];
    assert_eq!(new.len(), 3);
    assert!(
        new[0].chain_story.is_none()
            && new[1].chain_story.is_some()
            && new[2].chain_story.is_none()
    );
    assert_eq!(
        new[1].text.as_deref().map(|s| s.trim_end_matches('\r')),
        Some("Mixed")
    );
    source.mark_saved(out.clone(), ed.doc(), &plan);
    assert!(
        source.plan(ed.doc()).is_empty(),
        "{:?}",
        source.plan(ed.doc())
    );
    let (again, plan2) = source.build(ed.doc()).unwrap();
    assert!(plan2.new_shapes.is_empty() && plan2.new_text_boxes.is_empty());
    assert_eq!(
        pub_format::read_bytes(&again).unwrap().pages[0]
            .shapes
            .len(),
        n0 + 3
    );

    // Moving the saved shapes afterwards goes through the frame path like any shape.
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: vec![r, l, t],
        dx: 10 * PT,
        dy: 20 * PT,
    })
    .unwrap();
    let plan3 = source.plan(ed.doc());
    assert!(plan3.unsupported.is_empty(), "{plan3:?}");
    assert_eq!(plan3.frames.len(), 3);
    let (moved, plan3) = source.build(ed.doc()).unwrap();
    source.mark_saved(moved.clone(), ed.doc(), &plan3);
    assert!(source.plan(ed.doc()).is_empty());
    let m = pub_format::read_bytes(&moved).unwrap();
    assert_eq!(m.pages[0].shapes.len(), n0 + 3);
    let (w, h) = (f64::from(m.page_width_emu), f64::from(m.page_height_emu));
    let s = &m.pages[0].shapes[n0];
    assert!(((w / 2.0 + f64::from(s.left_emu)) / PT as f64 - 82.0).abs() <= 1.0);
    assert!(((h / 2.0 + f64::from(s.top_emu)) / PT as f64 - 680.0).abs() <= 1.0);
}

#[kernpage_samples::test]
fn unsupported_new_shapes_are_reported() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let id = add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 100 * PT, 40 * PT),
        None,
        editor_stroke(),
    );
    let mut s = ed.doc().pages[0].shape(id).unwrap().clone();
    s.rotation_deg = 30.0;
    s.id = 0;
    ed.execute(Command::AddShape { page: 0, shape: s }).unwrap();
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Ellipse,
        Rect::new(72 * PT, 660 * PT, 100 * PT, 40 * PT),
        None,
        None,
    );
    let plan = source.plan(ed.doc());
    assert_eq!(plan.new_shapes.len(), 2);
    assert!(plan.new_shapes[1].style.no_line);
    assert!(
        plan.unsupported
            .contains(&"new shapes that are rotated or grouped".to_string()),
        "{plan:?}"
    );
}

#[kernpage_samples::test]
fn colour_change_of_a_saved_new_shape_is_saved() {
    let bytes = sample("bulletin.pub");
    let (mut source, mut ed) = open(&bytes);
    let id = add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 100 * PT, 40 * PT),
        None,
        editor_stroke(),
    );
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out, ed.doc(), &plan);
    let mut changed = ed.doc().clone();
    changed.pages[0]
        .shapes
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap()
        .fill = Some(kernpage_core::Rgb(1, 2, 3));
    let plan = source.plan(&changed);
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.looks.len(), 1);
    let (out, _) = source.build(&changed).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let last = doc.pages[0].shapes.last().unwrap();
    assert_eq!(last.fill, Some(pub_format::Color::Rgb(1, 2, 3)));
}

#[kernpage_samples::test]
fn untouched_new_shapes_use_publisher_defaults() {
    use kernpage_core::Stroke;
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    // What the editor's new_shape gives: no fill, 0.75 pt, the document's scheme colour 0.
    let stroke = Some(Stroke {
        color: ed.doc().default_line,
        width_emu: 9_525,
    });
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 660 * PT, 150 * PT, 60 * PT),
        None,
        stroke,
    );
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.new_shapes[0].style, pub_write::ShapeStyle::default());
    let (out, _) = source.build(ed.doc()).unwrap();
    let direct = pub_write::add_shape(
        &bytes,
        0,
        pub_write::NewShapeKind::Rectangle,
        72.0,
        660.0,
        150.0,
        60.0,
        pub_write::ShapeStyle::default(),
    )
    .unwrap();
    // Same Escher bytes as the writer's default call: no literal fill (0x181/0x1C0) or colour.
    assert_eq!(
        pub_format::Streams::from_cfb(&out).unwrap().escher,
        pub_format::Streams::from_cfb(&direct).unwrap().escher
    );
}

#[kernpage_samples::test]
fn line_direction_round_trips() {
    use kernpage_core::Stroke;
    let bytes = sample("bulletin.pub");
    let n0 = pub_format::read_bytes(&bytes).unwrap().pages[0]
        .shapes
        .len();
    let (source, mut ed) = open(&bytes);
    let stroke = Some(Stroke {
        color: ed.doc().default_line,
        width_emu: 9_525,
    });
    // (flip_h, flip_v) for up-right, down-left, up-left, down-right drags.
    let cases = [(false, true), (true, false), (true, true), (false, false)];
    for (i, (h, v)) in cases.into_iter().enumerate() {
        let mut s = Shape::new(
            Rect::new(72 * PT, (500 + 40 * i as i64) * PT, 100 * PT, 30 * PT),
            ShapeKind::Line,
        );
        s.line = stroke;
        (s.flip_h, s.flip_v) = (h, v);
        ed.execute(Command::AddShape { page: 0, shape: s }).unwrap();
    }
    let (out, _) = source.build(ed.doc()).unwrap();
    let d = pub_format::read_bytes(&out).unwrap();
    let flips: Vec<_> = d.pages[0].shapes[n0..]
        .iter()
        .map(|s| (s.flip_h, s.flip_v))
        .collect();
    assert_eq!(flips, cases);
    let (_, ed2) = open(&out);
    let back: Vec<_> = ed2.doc().pages[0].shapes[n0..]
        .iter()
        .map(|s| (s.flip_h, s.flip_v))
        .collect();
    assert_eq!(back, cases);
}

#[kernpage_samples::test]
fn opened_sample_lines_report_their_flips() {
    let bytes = sample("shapes.pub");
    let src = pub_format::read_bytes(&bytes).unwrap();
    let model = from_pub_flowed("t.pub", &src, &[]);
    let mut lines = 0;
    for (ps, pm) in src.pages.iter().zip(&model.pages) {
        for (s, m) in ps.shapes.iter().zip(&pm.shapes) {
            if m.kind == ShapeKind::Line {
                lines += 1;
                assert_eq!((m.flip_h, m.flip_v), (s.flip_h, s.flip_v));
                eprintln!("line flips {:?}", (s.flip_h, s.flip_v));
            }
        }
    }
    assert!(lines > 0);
}

#[kernpage_samples::test]
#[ignore = "writes out/shape-defaults/ for the Publisher check"]
fn write_shape_default_samples() {
    use kernpage_core::Stroke;
    let bytes = kernpage_samples::read("hello.pub");
    let (mut source, mut ed) = open(&bytes);
    let stroke = Some(Stroke {
        color: ed.doc().default_line,
        width_emu: 9_525,
    });
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Rectangle,
        Rect::new(72 * PT, 100 * PT, 200 * PT, 100 * PT),
        None,
        stroke,
    );
    add_plain_shape(
        &mut ed,
        0,
        ShapeKind::Ellipse,
        Rect::new(300 * PT, 100 * PT, 200 * PT, 100 * PT),
        None,
        stroke,
    );
    let mut s = Shape::new(
        Rect::new(72 * PT, 250 * PT, 200 * PT, 100 * PT),
        ShapeKind::Line,
    );
    s.line = stroke;
    s.flip_v = true; // dragged up-right
    ed.execute(Command::AddShape { page: 0, shape: s }).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    source.mark_saved(out.clone(), ed.doc(), &plan);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/shape-defaults");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shape-defaults.pub"), out).unwrap();
}

#[kernpage_samples::test]
fn style_commands_on_existing_shapes_are_saved_and_undoable() {
    use kernpage_core::command::StyleChange as S;
    let bytes = sample("shapes.pub");
    let (mut source, mut ed) = open(&bytes);
    let (page, id) = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .find_map(|(p, pg)| {
            pg.shapes
                .iter()
                .find(|s| {
                    matches!(s.kind, ShapeKind::Rectangle | ShapeKind::Ellipse) && s.line.is_some()
                })
                .map(|s| (p, s.id))
        })
        .unwrap();
    let before = ed.doc().pages[page].shape(id).unwrap().clone();
    for change in [
        S::Fill(Some(kernpage_core::Rgb(1, 2, 3))),
        S::Outline(Some(kernpage_core::Rgb(4, 5, 6))),
        S::OutlineWeight(3 * PT),
        S::Rotate(90.0),
    ] {
        ed.execute(Command::StyleShapes {
            page,
            ids: vec![id],
            change,
        })
        .unwrap();
    }
    let plan = source.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    assert_eq!(plan.looks.len(), 1);
    let (out, plan) = source.build(ed.doc()).unwrap();
    let saved = pub_format::read_bytes(&out).unwrap();
    let ix = ed.doc().pages[page]
        .shapes
        .iter()
        .position(|s| s.id == id)
        .unwrap();
    let got = &saved.pages[page].shapes[ix];
    assert_eq!(got.fill, Some(pub_format::Color::Rgb(1, 2, 3)));
    assert_eq!(
        got.line,
        Some((pub_format::Color::Rgb(4, 5, 6), (3 * PT) as u32))
    );
    assert_eq!(got.rotation_deg, 90.0);
    // The saved file now matches the editor: nothing left to save.
    source.mark_saved(out, ed.doc(), &plan);
    assert!(source.plan(ed.doc()).is_empty());
    // No fill / no outline, then undo everything back to the original.
    for change in [S::Fill(None), S::Outline(None)] {
        ed.execute(Command::StyleShapes {
            page,
            ids: vec![id],
            change,
        })
        .unwrap();
    }
    let (out, _) = source.build(ed.doc()).unwrap();
    let got = pub_format::read_bytes(&out).unwrap().pages[page].shapes[ix].clone();
    assert_eq!((got.fill, got.line), (None, None));
    while ed.undo() {}
    let now = ed.doc().pages[page].shape(id).unwrap();
    assert_eq!(
        (now.fill, now.line, now.rotation_deg),
        (before.fill, before.line, before.rotation_deg)
    );
}

#[kernpage_samples::test]
fn wrap_and_column_changes_save_and_read_back() {
    use kernpage_core::{StyleChange, WrapType};
    // Wrap types on wrap.pub.
    let bytes = sample("wrap.pub");
    let (source, mut ed) = open(&bytes);
    let ids: Vec<(ShapeId, WrapType)> = ed.doc().pages[0]
        .shapes
        .iter()
        .filter(|s| !s.is_text_box() && s.wrap != WrapType::None)
        .map(|s| (s.id, s.wrap))
        .collect();
    assert!(!ids.is_empty());
    for target in [
        WrapType::None,
        WrapType::Square,
        WrapType::Tight,
        WrapType::Through,
        WrapType::TopAndBottom,
    ] {
        let (_, mut ed2) = open(&bytes);
        let want: Vec<ShapeId> = ids.iter().map(|x| x.0).collect();
        ed2.execute(Command::StyleShapes {
            page: 0,
            ids: want.clone(),
            change: StyleChange::Wrap(target),
        })
        .unwrap();
        let (src2, _) = open(&bytes);
        let (out, plan) = src2.build(ed2.doc()).unwrap();
        assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
        let back = pub_format::read_bytes(&out).unwrap();
        let (_, ed3) = {
            let b = pub2pdf::chain_breaks(&back, &pub2pdf::Options::default());
            let m = from_pub_flowed("t.pub", &back, &b);
            (0, Editor::new(m))
        };
        for id in &want {
            assert_eq!(ed3.doc().pages[0].shape(*id).unwrap().wrap, target);
        }
    }
    let _ = (source, &mut ed);

    // Columns on columns.pub / bulletin.pub.
    for name in ["columns.pub", "bulletin.pub"] {
        let bytes = sample(name);
        let (src, mut ed) = open(&bytes);
        let ids: Vec<ShapeId> = ed.doc().pages[0]
            .shapes
            .iter()
            .filter(|s| s.is_text_box())
            .map(|s| s.id)
            .collect();
        ed.execute(Command::StyleShapes {
            page: 0,
            ids: ids.clone(),
            change: StyleChange::Columns(2, 182_880),
        })
        .unwrap();
        let (out, plan) = src.build(ed.doc()).unwrap();
        assert!(plan.unsupported.is_empty(), "{name}: {:?}", plan.unsupported);
        assert!(!plan.columns.is_empty(), "{name}: no columns planned");
        let back = pub_format::read_bytes(&out).unwrap();
        let two = back
            .pages
            .iter()
            .flat_map(|p| &p.shapes)
            .filter(|s| s.story.is_some() && s.table.is_none() && s.columns == 2)
            .count();
        assert!(two >= plan.columns.len(), "{name}: {two} two-column boxes");
    }
}

fn box_paragraphs(doc: &Document, page: usize, id: ShapeId) -> Vec<Paragraph> {
    let ShapeKind::TextBox { paragraphs } = &doc.shape(page, id).unwrap().kind else {
        panic!("not a text box");
    };
    paragraphs.clone()
}

#[kernpage_samples::test]
fn clear_formatting_round_trips_to_plain_default() {
    use kernpage_core::{CharChange, Rgb};
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let plain = box_paragraphs(ed.doc(), page, id);
    let at = offset_of(ed.doc(), page, id, "Prelude");
    for change in [
        CharChange::Bold(true),
        CharChange::FontName("Georgia".into()),
        CharChange::Color(Rgb(255, 0, 0)),
    ] {
        ed.execute(Command::FormatRuns {
            page,
            id,
            cell: None,
            range: at..at + 7,
            change,
        })
        .unwrap();
    }
    let (out, _) = source.build(ed.doc()).unwrap();
    let (source2, mut ed2) = open(&out);
    let formatted = box_paragraphs(ed2.doc(), page, id);
    assert_ne!(formatted, plain);
    ed2.execute(Command::FormatRuns {
        page,
        id,
        cell: None,
        range: at..at + 7,
        change: CharChange::Clear,
    })
    .unwrap();
    assert_eq!(box_paragraphs(ed2.doc(), page, id), plain);
    // Undo restores the formatted look.
    ed2.undo();
    assert_eq!(box_paragraphs(ed2.doc(), page, id), formatted);
    ed2.redo();
    let (out2, plan) = source2.build(ed2.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    if let Ok(dir) = std::env::var("PW_CLEAR_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("formatted.pub"), &out).unwrap();
        std::fs::write(std::path::Path::new(&dir).join("cleared.pub"), &out2).unwrap();
    }
    let (_, ed3) = open(&out2);
    assert_eq!(box_paragraphs(ed3.doc(), page, id), plain);
}

#[kernpage_samples::test]
fn clear_formatting_whole_box() {
    use kernpage_core::CharChange;
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "Prelude");
    let len = kernpage_core::text::text_len(&box_paragraphs(ed.doc(), page, id));
    // The heading is 16 pt bold: clearing the whole box makes every run plain.
    ed.execute(Command::FormatRuns {
        page,
        id,
        cell: None,
        range: 0..len,
        change: CharChange::Clear,
    })
    .unwrap();
    let cleared = box_paragraphs(ed.doc(), page, id);
    assert!(cleared.iter().all(|p| p.runs.len() == 1
        && !p.runs[0].bold
        && p.runs[0].size_pt == 10.0
        && p.runs[0].font == "Calibri"));
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{plan:?}");
    let (_, back) = open(&out);
    assert_eq!(box_paragraphs(back.doc(), page, id), cleared);
}

// ---------------------------------------------------------------- page operations (spec 4P)

/// Per page, the text of every shape of the saved file.
fn file_texts(bytes: &[u8]) -> Vec<Vec<String>> {
    pub_format::read_bytes(bytes)
        .unwrap()
        .pages
        .iter()
        .map(|p| p.shapes.iter().map(shape_text).collect())
        .collect()
}

fn saved(source: &mut SaveSource, ed: &Editor) -> Vec<u8> {
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    source.mark_saved(out.clone(), ed.doc(), &plan);
    out
}

#[kernpage_samples::test]
fn page_add_move_delete_save_and_read_back() {
    use kernpage_core::PageOp;
    let bytes = sample("bulletin.pub");
    let orig = file_texts(&bytes);
    // Add after the last, the first (middle) and, via two adds, twice.
    for (after, want_at) in [(1usize, 2usize), (0, 1)] {
        let (mut source, mut ed) = open(&bytes);
        ed.execute(Command::InsertPage { after }).unwrap();
        let out = saved(&mut source, &ed);
        let t = file_texts(&out);
        assert_eq!(t.len(), 3);
        assert!(t[want_at].is_empty());
        assert!(source.plan(ed.doc()).is_empty());
    }
    let (mut source, mut ed) = open(&bytes);
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    let (_, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(plan.page_ops.len(), 2);
    let out = saved(&mut source, &ed);
    let t = file_texts(&out);
    assert_eq!((t.len(), &t[0], &t[3]), (4, &orig[0], &orig[1]));
    // Move both directions.
    let (mut source, mut ed) = open(&bytes);
    ed.execute(Command::MovePage { from: 1, to: 0 }).unwrap();
    let (_, plan) = source.build(ed.doc()).unwrap();
    assert_eq!(plan.page_ops, vec![PageOp::Move { from: 1, to: 0 }]);
    let out = saved(&mut source, &ed);
    assert_eq!(file_texts(&out), vec![orig[1].clone(), orig[0].clone()]);
    ed.execute(Command::MovePage { from: 0, to: 1 }).unwrap();
    let out = saved(&mut source, &ed);
    assert_eq!(file_texts(&out), orig);
    // Delete the picture page and the second page.
    let (mut source, mut ed) = open(&bytes);
    ed.execute(Command::DeletePage { page: 0 }).unwrap();
    let out = saved(&mut source, &ed);
    assert_eq!(file_texts(&out), vec![orig[1].clone()]);
    let (mut source, mut ed) = open(&bytes);
    ed.execute(Command::DeletePage { page: 1 }).unwrap();
    let out = saved(&mut source, &ed);
    assert_eq!(file_texts(&out), vec![orig[0].clone()]);
    // Saving twice repeats nothing.
    assert!(source.plan(ed.doc()).is_empty());
    let again = saved(&mut source, &ed);
    assert_eq!(file_texts(&again), file_texts(&out));
}

#[kernpage_samples::test]
fn page_operations_combine_with_edits_and_new_objects() {
    let bytes = sample("bulletin.pub");
    let orig = file_texts(&bytes);
    let (mut source, mut ed) = open(&bytes);
    // Edit a box on page 2, move page 2 first, delete nothing, add a page at the end
    // holding a new text box.
    let (page, id) = find_text(ed.doc(), "Prayer List");
    let c = retext(ed.doc(), page, id, "Prayers");
    ed.execute(c).unwrap();
    ed.execute(Command::MovePage { from: 1, to: 0 }).unwrap();
    ed.execute(Command::InsertPage { after: 1 }).unwrap();
    let mut b = Shape::new(
        Rect::new(914_400, 914_400, 2_000_000, 500_000),
        ShapeKind::TextBox {
            paragraphs: vec![Paragraph::plain("Fresh")],
        },
    );
    b.id = 0;
    ed.execute(Command::AddShape { page: 2, shape: b }).unwrap();
    let out = saved(&mut source, &ed);
    let t = file_texts(&out);
    assert_eq!(t.len(), 3);
    assert!(t[0][1].starts_with("Prayers"), "{t:?}");
    assert_eq!(t[1], orig[0]);
    assert!(t[2][0].starts_with("Fresh"), "{t:?}");
    // Frames of untouched shapes are unchanged.
    let (a, b) = (
        pub_format::read_bytes(&bytes).unwrap(),
        pub_format::read_bytes(&out).unwrap(),
    );
    for (x, y) in a.pages[0].shapes.iter().zip(&b.pages[1].shapes) {
        assert_eq!(
            (x.left_emu, x.top_emu, x.right_emu, x.bottom_emu),
            (y.left_emu, y.top_emu, y.right_emu, y.bottom_emu)
        );
    }
    assert!(source.plan(ed.doc()).is_empty());
    // A second, text-only edit after the page operations still lands on the right box.
    let (page, id) = find_text(ed.doc(), "Fresh");
    let c = retext(ed.doc(), page, id, "Fresher");
    ed.execute(c).unwrap();
    let (page, id) = find_text(ed.doc(), "Announcements");
    let c = retext(ed.doc(), page, id, "News");
    ed.execute(c).unwrap();
    let out = saved(&mut source, &ed);
    let t = file_texts(&out);
    assert!(t[2][0].starts_with("Fresher") && t[1].len() == 3, "{t:?}");
    assert!(t[0][0].starts_with("News"), "{t:?}");
}

#[kernpage_samples::test]
fn deleting_every_page_but_one_and_undo() {
    let bytes = sample("bulletin.pub");
    let (source, mut ed) = open(&bytes);
    ed.execute(Command::DeletePage { page: 0 }).unwrap();
    assert!(!source.plan(ed.doc()).page_ops.is_empty());
    ed.undo();
    // Undo restores the same page: nothing to save.
    assert!(source.plan(ed.doc()).is_empty());
}

fn file_runs(bytes: &[u8]) -> Vec<Vec<String>> {
    pub_format::read_bytes(bytes)
        .unwrap()
        .pages
        .iter()
        .map(|p| p.shapes.iter().map(|s| format!("{:?}", s.runs)).collect())
        .collect()
}

#[kernpage_samples::test]
fn duplicate_page_is_saved_as_a_page_duplicate() {
    use kernpage_core::PageOp;
    let bytes = sample("bulletin.pub");
    let orig = file_texts(&bytes);
    for src in [0usize, 1] {
        let (mut source, mut ed) = open(&bytes);
        ed.execute(Command::DuplicatePage { page: src }).unwrap();
        let (_, plan) = source.build(ed.doc()).unwrap();
        assert_eq!(plan.page_ops, vec![PageOp::Duplicate { index: src }]);
        assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
        assert!(plan.new_text_boxes.is_empty() && plan.new_shapes.is_empty());
        let out = saved(&mut source, &ed);
        let t = file_texts(&out);
        assert_eq!(t.len(), 3);
        assert_eq!(t[src], orig[src]);
        assert_eq!(t[src + 1], orig[src]);
        let doc = pub_format::read_bytes(&out).unwrap();
        assert_eq!(
            doc.pages[src].shapes.iter().filter(|s| s.picture.is_some()).count(),
            doc.pages[src + 1].shapes.iter().filter(|s| s.picture.is_some()).count()
        );
        // A second save repeats nothing and gives the same file content.
        assert!(source.plan(ed.doc()).is_empty());
        let again = saved(&mut source, &ed);
        assert_eq!(file_texts(&again), t);
        // Editing text on the copy after saving lands on the copy only.
        let copy_id = ed.doc().pages[src + 1]
            .shapes
            .iter()
            .find(|s| s.is_text_box())
            .unwrap()
            .id;
        let c = retext(ed.doc(), src + 1, copy_id, "Edited on the copy");
        ed.execute(c).unwrap();
        let out2 = saved(&mut source, &ed);
        let t2 = file_texts(&out2);
        assert_eq!(t2[src], orig[src]);
        assert!(t2[src + 1].iter().any(|x| x.starts_with("Edited on the copy")), "{t2:?}");
        assert!(source.plan(ed.doc()).is_empty());
    }
}

#[kernpage_samples::test]
fn duplicate_page_keeps_formatting_follows_edits_and_moves() {
    use kernpage_core::PageOp;
    let bytes = sample("formatting.pub");
    let (mut source, mut ed) = open(&bytes);
    // Edit the source first; the copy (made afterwards) carries the edit.
    let (page, id) = find_text(ed.doc(), "");
    let c = retext(ed.doc(), page, id, "Changed before copying");
    ed.execute(c).unwrap();
    ed.execute(Command::DuplicatePage { page: 0 }).unwrap();
    // Move the copy to the front: Duplicate then Move.
    ed.execute(Command::MovePage { from: 1, to: 0 }).unwrap();
    let (_, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(plan.page_ops, vec![PageOp::Duplicate { index: 0 }, PageOp::Move { from: 1, to: 0 }]);
    let out = saved(&mut source, &ed);
    let (t, r) = (file_texts(&out), file_runs(&out));
    assert_eq!(t[0], t[1]);
    assert_eq!(r[0], r[1]);
    assert!(t[0].iter().any(|x| x.starts_with("Changed before copying")));
    assert!(source.plan(ed.doc()).is_empty());
}

#[kernpage_samples::test]
fn duplicate_page_with_unsupported_content_is_noted_and_the_rest_still_saves() {
    for name in ["group.pub", "table.pub", "linked.pub"] {
        let bytes = sample(name);
        let n = pub_format::read_bytes(&bytes).unwrap().pages.len();
        let (mut source, mut ed) = open(&bytes);
        ed.execute(Command::DuplicatePage { page: 0 }).unwrap();
        // An ordinary blank page after it still saves.
        ed.execute(Command::InsertPage { after: 1 }).unwrap();
        let plan = source.plan(ed.doc());
        assert!(!plan.unsupported.is_empty(), "{name}");
        assert!(
            !plan.page_ops.iter().any(|o| matches!(o, kernpage_core::PageOp::Duplicate { .. })),
            "{name}"
        );
        assert_eq!(plan.page_ops, vec![kernpage_core::PageOp::Add { after: Some(0) }], "{name}");
        let (out, plan) = source.build(ed.doc()).unwrap();
        assert_eq!(pub_format::read_bytes(&out).unwrap().pages.len(), n + 1, "{name}");
        source.mark_saved(out, ed.doc(), &plan);
        // The left-out page stays unsaved (still noted, nothing else planned).
        let again = source.plan(ed.doc());
        assert!(again.page_ops.is_empty(), "{name}: {:?}", again.page_ops);
        assert!(!again.unsupported.is_empty(), "{name}");
    }
}

fn tokn_bytes(file: &[u8]) -> Option<Vec<u8>> {
    let quill = pub_format::Streams::from_cfb(file).unwrap().quill;
    let tag = [0x01, 0, 0, 0, 0x0c, 0, 0, 0, 0xff, 0xff, 0x01, 0];
    let at = quill.windows(tag.len()).position(|w| w == tag)?;
    Some(quill[at..at + 0x34].to_vec())
}

/// Master pages (spec 4T): they are shown, never edited, and a save with an unrelated edit
/// keeps their shapes and the page-number field data exactly as they were.
#[kernpage_samples::test]
fn saving_keeps_master_shapes_and_page_number_fields() {
    for name in ["master-pn-text-2p.pub", "master-box-2p-ignore.pub", "master-pn-1p.pub"] {
        let bytes = sample(name);
        let before = pub_format::read_bytes(&bytes).unwrap();
        let (mut source, mut ed) = open(&bytes);
        assert!(source.plan(ed.doc()).is_empty(), "{name}: opening changes nothing");
        // The editor knows the masters, but they are not shapes of any page.
        assert_eq!(ed.doc().masters.len(), 1);
        assert!(ed.doc().pages.iter().all(|p| p.shapes.is_empty()), "{name}");
        // An unrelated edit: a new shape on page 1 and a new page.
        ed.execute(Command::AddShape {
            page: 0,
            shape: Shape::new(Rect::new(0, 0, 914_400, 914_400), ShapeKind::Rectangle),
        })
        .unwrap();
        ed.execute(Command::InsertPage { after: 0 }).unwrap();
        let (out, plan) = source.build(ed.doc()).unwrap();
        assert!(plan.unsupported.is_empty(), "{name}: {plan:?}");
        let after = pub_format::read_bytes(&out).unwrap();
        assert_eq!(after.masters, before.masters, "{name}: master shapes are unchanged");
        assert_eq!(after.pages.len(), before.pages.len() + 1, "{name}");
        assert_eq!(tokn_bytes(&out), tokn_bytes(&bytes), "{name}: TOKN data is unchanged");
        assert_eq!(tokn_bytes(&bytes).is_some(), name.contains("-pn-"), "{name}");
        source.mark_saved(out, ed.doc(), &plan);
    }
}

// ---------------------------------------------------------------------------------------
// pw-96om: page-number fields (Quill TOKN) and sections stay consistent after edits.

/// (cp_start, cp_end) of the single-field TOKN entry of a saved file, if it has one.
fn tokn_cps(file: &[u8]) -> Option<(u32, u32)> {
    let t = tokn_bytes(file)?;
    let u = |o: usize| u32::from_le_bytes(t[o..o + 4].try_into().unwrap());
    Some((u(12), u(16)))
}

/// Character offset of the page-number field run of the first text shape of page 1, if any.
fn field_offset(file: &[u8]) -> Option<usize> {
    let d = pub_format::read_bytes(file).unwrap();
    let shape = d.pages[0].shapes.iter().find(|s| s.runs.iter().any(|r| r.field.is_some()))?;
    let mut at = 0;
    for r in &shape.runs {
        if r.field.is_some() {
            return Some(at);
        }
        at += r.text.chars().count();
    }
    None
}

/// Open the page-box sample, retype its text box (one edit per entry of `texts`, as typing
/// does) and save; returns the saved bytes.
fn save_field_texts(texts: &[&str]) -> Vec<u8> {
    let bytes = sample("master-pn-boxpage-1p.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "#");
    for text in texts {
        let c = retext(ed.doc(), page, id, text);
        ed.execute(c).unwrap();
    }
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    out
}

fn save_field_text(text: &str) -> Vec<u8> {
    save_field_texts(&[text])
}

#[kernpage_samples::test]
fn field_follows_text_typed_before_it() {
    let out = save_field_text("Page #");
    assert_eq!(tokn_cps(&out), Some((5, 7)));
    assert_eq!(field_offset(&out), Some(5));
}

#[kernpage_samples::test]
fn field_follows_text_typed_after_it() {
    let out = save_field_text("# of 3");
    assert_eq!(tokn_cps(&out), Some((0, 7)));
    assert_eq!(field_offset(&out), Some(0));
}

#[kernpage_samples::test]
fn field_follows_text_typed_around_it() {
    let out = save_field_texts(&["Page #", "Page # of 3"]);
    assert_eq!(tokn_cps(&out), Some((5, 12)));
    assert_eq!(field_offset(&out), Some(5));
}

#[kernpage_samples::test]
fn text_replaced_on_both_sides_at_once_drops_the_field_consistently() {
    // One edit that changes both sides of the field is a replacement of the field: the
    // editor model and the file agree that it is plain text now.
    let bytes = sample("master-pn-boxpage-1p.pub");
    let (source, mut ed) = open(&bytes);
    let (page, id) = find_text(ed.doc(), "#");
    let c = retext(ed.doc(), page, id, "Page # of 3");
    ed.execute(c).unwrap();
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    assert!(paragraphs.iter().flat_map(|p| &p.runs).all(|r| r.field.is_none()));
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(tokn_bytes(&out), None);
}

#[kernpage_samples::test]
fn deleting_the_field_removes_tokn() {
    let out = save_field_text("gone");
    assert_eq!(tokn_bytes(&out), None);
    assert_eq!(field_offset(&out), None);
    let d = pub_format::read_bytes(&out).unwrap();
    assert_eq!(d.pages[0].shapes[0].text.as_deref(), Some("gone\r"));
}

#[kernpage_samples::test]
fn a_typed_hash_is_not_a_field() {
    let out = save_field_text("# and #");
    assert_eq!(tokn_cps(&out), Some((0, 8)));
    assert_eq!(field_offset(&out), Some(0));
}

#[kernpage_samples::test]
fn text_edit_after_a_field_edit_round_trips() {
    // Save, reopen the saved file, edit again.
    let out = save_field_text("Page #");
    let (source, mut ed) = open(&out);
    let (page, id) = find_text(ed.doc(), "Page");
    for text in ["The Page #", "The Page # now"] {
        let c = retext(ed.doc(), page, id, text);
        ed.execute(c).unwrap();
    }
    let (out2, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    assert_eq!(tokn_cps(&out2), Some((9, 15)));
    assert_eq!(field_offset(&out2), Some(9));
}

/// (text of the run, raw character record) of every run of the first story whose text has a
/// `#`, in story order; the decoded form of the FDPC runs of that story.
fn field_story_runs(file: &[u8]) -> Vec<(String, Vec<u8>)> {
    for k in 0..32 {
        let Ok(r) = pub_write::story_records(file, k) else { continue };
        if !r.text.contains(&u16::from(b'#')) {
            continue;
        }
        return r
            .runs
            .iter()
            .map(|run| {
                (
                    String::from_utf16_lossy(&r.text[run.start..run.end]),
                    r.char_records[run.record].clone(),
                )
            })
            .collect();
    }
    panic!("no story with a #");
}

/// Whether a raw character record carries the field-only item 0x200 (spec 4T.3).
fn has_field_marks(rec: &[u8]) -> bool {
    // item header 0x00 0x12 = id 0x200, class 2
    rec[4..].windows(2).any(|w| w == [0x00, 0x12])
}

#[kernpage_samples::test]
fn typing_before_a_field_gives_publishers_runs_and_records() {
    let out = save_field_text("Page #");
    let ours = field_story_runs(&out);
    let publisher = field_story_runs(&sample("master-pn-text-2p.pub"));
    assert_eq!(ours, publisher);
    assert_eq!(ours.len(), 3);
    assert!(!has_field_marks(&ours[0].1) && has_field_marks(&ours[1].1) && !has_field_marks(&ours[2].1));
}

#[kernpage_samples::test]
fn only_the_field_character_carries_the_field_marks() {
    for text in ["# of 3", "Page # of 3"] {
        let out = if text.starts_with("Page") {
            save_field_texts(&["Page #", text])
        } else {
            save_field_text(text)
        };
        let runs = field_story_runs(&out);
        let marked: Vec<&str> =
            runs.iter().filter(|(_, r)| has_field_marks(r)).map(|(t, _)| t.as_str()).collect();
        assert_eq!(marked, ["#"], "{text}: {runs:?}");
    }
}

#[kernpage_samples::test]
fn a_deleted_field_leaves_no_field_marks() {
    let out = save_field_text("gone #x");
    // "#" typed as plain text is not a field; no run may carry the marks
    let runs = field_story_runs(&out);
    assert!(runs.iter().all(|(_, r)| !has_field_marks(r)), "{runs:?}");
}

fn starts(sections: &[pub_format::Section]) -> Vec<u32> {
    sections.iter().map(|s| s.start_page).collect()
}

/// Run `cmds` on the 3-page, 2-section sample; returns (saved bytes, editor model sections).
fn save_sections(cmds: &[Command]) -> (Vec<u8>, Vec<pub_format::Section>) {
    let bytes = sample("master-pn-section2-fmt.pub");
    let (source, mut ed) = open(&bytes);
    assert_eq!(starts(&ed.doc().sections), vec![1, 2]);
    for c in cmds {
        ed.execute(c.clone()).unwrap();
    }
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    (out, ed.doc().sections.clone())
}

/// The saved file's sections and the page-number text of every page.
fn read_sections(out: &[u8]) -> (Vec<pub_format::Section>, Vec<String>) {
    let d = pub_format::read_bytes(out).unwrap();
    let numbers = (0..d.pages.len()).map(|i| pub_format::page_number_text(&d, i)).collect();
    (d.sections, numbers)
}

fn check_sections(cmds: &[Command], want_starts: &[u32], want_numbers: &[&str]) {
    let (out, model) = save_sections(cmds);
    let (sections, numbers) = read_sections(&out);
    assert_eq!(starts(&sections), want_starts, "saved start pages after {cmds:?}");
    assert_eq!(starts(&model), want_starts, "editor model start pages after {cmds:?}");
    assert_eq!(numbers, want_numbers, "page numbers after {cmds:?}");
    // Section data other than the start page is untouched.
    let orig = pub_format::read_bytes(&sample("master-pn-section2-fmt.pub")).unwrap();
    if sections.len() == 2 {
        assert_eq!(sections[1].first_number, orig.sections[1].first_number);
        assert_eq!(sections[1].format, orig.sections[1].format);
    }
    // The editor shows the same numbers before saving.
    let (_, ed) = open(&sample("master-pn-section2-fmt.pub"));
    let mut ed = ed;
    for c in cmds {
        ed.execute(c.clone()).unwrap();
    }
    let shown: Vec<String> = (0..ed.doc().pages.len()).map(|i| ed.doc().page_number_text(i)).collect();
    assert_eq!(shown, want_numbers, "editor numbers after {cmds:?}");
}

// Original: pages 1 | 2 3 -> numbers "1", "i", "ii".
#[kernpage_samples::test]
fn sections_insert_page_before_after_and_inside_section_two() {
    // New page belongs to the section of the page it follows.
    check_sections(&[Command::InsertPage { after: 0 }], &[1, 3], &["1", "2", "i", "ii"]);
    check_sections(&[Command::InsertPage { after: 1 }], &[1, 2], &["1", "i", "ii", "iii"]);
    check_sections(&[Command::InsertPage { after: 2 }], &[1, 2], &["1", "i", "ii", "iii"]);
}

#[kernpage_samples::test]
fn sections_duplicate_page() {
    check_sections(&[Command::DuplicatePage { page: 0 }], &[1, 3], &["1", "2", "i", "ii"]);
    check_sections(&[Command::DuplicatePage { page: 1 }], &[1, 2], &["1", "i", "ii", "iii"]);
    check_sections(&[Command::DuplicatePage { page: 2 }], &[1, 2], &["1", "i", "ii", "iii"]);
}

#[kernpage_samples::test]
fn sections_delete_page() {
    // Section 2 loses its first page: it now starts at the next page (same index).
    check_sections(&[Command::DeletePage { page: 1 }], &[1, 2], &["1", "i"]);
    check_sections(&[Command::DeletePage { page: 2 }], &[1, 2], &["1", "i"]);
    // Section 1's only page goes: the first section must start at page 1.
    check_sections(&[Command::DeletePage { page: 0 }], &[1], &["i", "ii"]);
}

#[kernpage_samples::test]
fn sections_delete_every_page_of_section_two() {
    check_sections(
        &[Command::DeletePage { page: 2 }, Command::DeletePage { page: 1 }],
        &[1],
        &["1"],
    );
}

#[kernpage_samples::test]
fn sections_insert_then_delete() {
    check_sections(
        &[Command::InsertPage { after: 0 }, Command::DeletePage { page: 0 }],
        &[1, 2],
        &["1", "i", "ii"],
    );
}

#[kernpage_samples::test]
fn undo_restores_sections() {
    let bytes = sample("master-pn-section2-fmt.pub");
    let (_, mut ed) = open(&bytes);
    let before = ed.doc().sections.clone();
    ed.execute(Command::DeletePage { page: 0 }).unwrap();
    assert_eq!(starts(&ed.doc().sections), vec![1]);
    ed.undo();
    assert_eq!(ed.doc().sections, before);
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    ed.undo();
    assert_eq!(ed.doc().sections, before);
}

#[kernpage_samples::test]
fn duplicating_a_page_with_a_page_number_field_is_saved_with_its_own_table() {
    let bytes = sample("master-pn-boxpage-1p.pub");
    let (source, mut ed) = open(&bytes);
    ed.execute(Command::DuplicatePage { page: 0 }).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let d = pub_format::read_bytes(&out).unwrap();
    assert_eq!(d.pages.len(), 2);
    for p in &d.pages {
        assert!(p.shapes[0].runs.iter().any(|r| r.field.is_some()));
    }
}

#[kernpage_samples::test]
fn a_single_section_that_starts_at_page_one_keeps_its_first_number() {
    let bytes = sample("master-pn-start5.pub");
    let (source, mut ed) = open(&bytes);
    ed.execute(Command::InsertPage { after: 0 }).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let d = pub_format::read_bytes(&out).unwrap();
    assert_eq!(d.sections.len(), 1);
    assert_eq!(d.sections[0].first_number, Some(5));
    assert_eq!(pub_format::page_number_text(&d, 1), "6");
}

/// Writes the pw-96om outputs for a manual check in Publisher (out/ is not committed).
#[kernpage_samples::test]
#[ignore = "writes out/pw-96om for manual Publisher checks"]
fn write_pw96om_samples() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/pw-96om");
    std::fs::create_dir_all(&dir).unwrap();
    let save = |name: &str, out: Vec<u8>| std::fs::write(dir.join(name), out).unwrap();
    save("field-before.pub", save_field_text("Page #"));
    save("field-after.pub", save_field_text("# of 3"));
    save("field-around.pub", save_field_texts(&["Page #", "Page # of 3"]));
    save("field-deleted.pub", save_field_text("gone"));
    let cases: [(&str, Vec<Command>); 7] = [
        ("sec-insert-before-s2", vec![Command::InsertPage { after: 0 }]),
        ("sec-insert-inside-s2", vec![Command::InsertPage { after: 1 }]),
        ("sec-insert-after-s2", vec![Command::InsertPage { after: 2 }]),
        ("sec-duplicate-p1", vec![Command::DuplicatePage { page: 0 }]),
        ("sec-delete-first-of-s2", vec![Command::DeletePage { page: 1 }]),
        ("sec-delete-s1-only-page", vec![Command::DeletePage { page: 0 }]),
        (
            "sec-delete-all-of-s2",
            vec![Command::DeletePage { page: 2 }, Command::DeletePage { page: 1 }],
        ),
    ];
    for (name, cmds) in cases {
        save(&format!("{name}.pub"), save_sections(&cmds).0);
    }
}
