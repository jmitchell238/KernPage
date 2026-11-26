//! Bullets and numbering in the editor (spec 4R): the list commands with undo and redo,
//! numbering restarts, saving through `set_story_lists`, AutoRecover, and unchanged saves.

use kernpage_core::autorecover::{decode, encode, Meta};
use kernpage_core::text::{list_labels, ListChange};
use kernpage_core::{
    from_pub_flowed, Command, Document, Editor, ListInfo, ListType, ParaIndents, Paragraph,
    SaveSource, ShapeId, ShapeKind,
};
use std::ops::Range;

fn sample(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen");
    std::fs::read(kernpage_samples::resolve(p.join(format!("{name}.pub")))).unwrap()
}

fn open(name: &str) -> (SaveSource, Editor) {
    let bytes = sample(name);
    let src = pub_format::read_bytes(&bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    let source = SaveSource::new(bytes, &src, &model, &breaks);
    (source, Editor::new(model))
}

/// The text box holding "One" (every list sample has one).
fn the_box(doc: &Document) -> (usize, ShapeId) {
    for (pi, p) in doc.pages.iter().enumerate() {
        for s in &p.shapes {
            if let ShapeKind::TextBox { paragraphs } = &s.kind {
                if paragraphs.iter().any(|q| q.text() == "One") {
                    return (pi, s.id);
                }
            }
        }
    }
    panic!("no list text box");
}

fn paras(ed: &Editor) -> Vec<Paragraph> {
    let (page, id) = the_box(ed.doc());
    match &ed.doc().shape(page, id).unwrap().kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.clone(),
        _ => unreachable!(),
    }
}

/// The editable-text range covering paragraphs `from..=to` (paragraphs joined by `\n`).
fn range_of(ps: &[Paragraph], from: usize, to: usize) -> Range<usize> {
    let mut at = 0;
    let mut out = 0..0;
    for (i, p) in ps.iter().enumerate() {
        let n = p.text().chars().count();
        if i == from {
            out.start = at;
        }
        if i == to {
            out.end = at + n;
        }
        at += n + 1;
    }
    out
}

fn list_cmd(ed: &mut Editor, from: usize, to: usize, change: ListChange) {
    let (page, id) = the_box(ed.doc());
    let range = range_of(&paras(ed), from, to);
    ed.execute(Command::ListParagraphs {
        page,
        id,
        cell: None,
        range,
        change,
    })
    .unwrap();
}

/// Run `change` on paragraphs `from..=to` and check undo and redo restore exactly.
fn with_undo(ed: &mut Editor, from: usize, to: usize, change: ListChange) -> Vec<Paragraph> {
    let before = paras(ed);
    list_cmd(ed, from, to, change);
    let after = paras(ed);
    assert_ne!(before, after, "the command changed nothing");
    assert!(ed.undo());
    assert_eq!(paras(ed), before, "undo restores exactly");
    assert!(ed.redo());
    assert_eq!(paras(ed), after, "redo restores exactly");
    after
}

fn labels(ps: &[Paragraph]) -> Vec<Option<String>> {
    list_labels(ps)
}

fn s(x: &str) -> Option<String> {
    Some(x.to_owned())
}

/// Build the saved bytes, with nothing unsupported, and read them back.
fn saved(source: &SaveSource, ed: &Editor) -> (Vec<u8>, pub_format::Document) {
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let back = pub_format::read_bytes(&out).unwrap();
    (out, back)
}

/// The paragraphs of the box holding "One" in a read-back document.
fn read_paras(back: &pub_format::Document) -> Vec<pub_format::Paragraph> {
    back.pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| s.paragraphs.iter().any(|p| p.runs.iter().any(|r| r.text.starts_with("One"))))
        .expect("the list box")
        .paragraphs
        .clone()
}

/// What a paragraph of the model says about its list, for comparing with the file.
fn model_state(p: &Paragraph) -> (Option<ListInfo>, ParaIndents) {
    (p.list.clone(), p.indents)
}

// ---------------------------------------------------------------- commands

#[kernpage_samples::test]
fn set_bullet_numbering_and_clear_with_undo_redo() {
    let (_, mut ed) = open("list-none");
    assert!(paras(&ed).iter().all(|p| p.list.is_none()));

    // A bullet on the middle paragraph only.
    let after = with_undo(&mut ed, 1, 1, ListChange::Set(ListInfo::bullet()));
    assert_eq!(after[0].list, None);
    assert_eq!(after[1].list, Some(ListInfo::bullet()));
    assert_eq!(after[1].indents, ParaIndents::for_list(18.0));
    assert_eq!(after[2].list, None);
    assert_eq!(labels(&after), [None, s("\u{b7}"), None]);

    // Clear it again: no list, no left/first-line indent.
    let after = with_undo(&mut ed, 1, 1, ListChange::Clear);
    assert!(after.iter().all(|p| p.list.is_none()));
    assert_eq!(after[1].indents, ParaIndents::default());
}

#[kernpage_samples::test]
fn bullet_with_character_font_and_size() {
    let (_, mut ed) = open("list-none");
    let b = ListInfo::bullet_with(0xA7, "Wingdings", 14.0);
    let after = with_undo(&mut ed, 0, 2, ListChange::Set(b.clone()));
    for p in &after {
        assert_eq!(p.list, Some(b.clone()));
    }
}

#[kernpage_samples::test]
fn numbering_continues_and_restarts_by_the_rules() {
    let (_, mut ed) = open("list-num-types");
    // Make a plain base: clear everything, then number paragraphs 0,1 and 3,4,5.
    list_cmd(&mut ed, 0, 8, ListChange::Clear);
    let num = ListInfo::numbered(ListType::Arabic);
    list_cmd(&mut ed, 0, 1, ListChange::Set(num.clone()));
    list_cmd(&mut ed, 3, 5, ListChange::Set(num.clone()));
    let ps = paras(&ed);
    // A plain paragraph (2) ends the run: numbering starts again at the start value.
    assert_eq!(
        labels(&ps)[..6],
        [s("1."), s("2."), None, s("1."), s("2."), s("3.")]
    );
    // A bullet in the middle of a run ends it too.
    list_cmd(&mut ed, 4, 4, ListChange::Set(ListInfo::bullet()));
    assert_eq!(
        labels(&paras(&ed))[3..6],
        [s("1."), s("\u{b7}"), s("1.")]
    );
    // A different separator or type starts a new run; a different start value too.
    list_cmd(&mut ed, 5, 5, ListChange::Set(ListInfo::numbered(ListType::UpperRoman)));
    list_cmd(&mut ed, 6, 6, ListChange::Set(ListInfo::numbered(ListType::UpperRoman)));
    assert_eq!(labels(&paras(&ed))[5..7], [s("I."), s("II.")]);
    let after = with_undo(&mut ed, 6, 6, ListChange::Separator(0));
    assert_eq!(labels(&after)[5..7], [s("I."), s("I)")]);
    let after = with_undo(&mut ed, 6, 6, ListChange::Start(5));
    assert_eq!(labels(&after)[6], s("V)"));
}

#[kernpage_samples::test]
fn separator_start_list_indent_and_indents_commands() {
    let (_, mut ed) = open("list-none");
    list_cmd(&mut ed, 0, 2, ListChange::Set(ListInfo::numbered(ListType::LowerLetter)));
    let after = with_undo(&mut ed, 0, 2, ListChange::Separator(1));
    assert_eq!(labels(&after), [s("(a)"), s("(b)"), s("(c)")]);
    let after = with_undo(&mut ed, 0, 2, ListChange::Start(3));
    assert_eq!(labels(&after), [s("(c)"), s("(d)"), s("(e)")]);
    // List indent: label and text move together (left = list indent, first = -list indent).
    let after = with_undo(&mut ed, 0, 0, ListChange::ListIndent(36.0));
    assert_eq!(after[0].list.as_ref().unwrap().list_indent_pt, 36.0);
    assert_eq!(after[0].indents, ParaIndents::for_list(36.0));
    // List indent 0 gives no indents at all.
    let after = with_undo(&mut ed, 1, 1, ListChange::ListIndent(0.0));
    assert_eq!(after[1].indents, ParaIndents::default());
    // Separator and start do nothing on a bullet.
    list_cmd(&mut ed, 2, 2, ListChange::Set(ListInfo::bullet()));
    let before = paras(&ed);
    list_cmd(&mut ed, 2, 2, ListChange::Separator(1));
    list_cmd(&mut ed, 2, 2, ListChange::Start(9));
    assert_eq!(paras(&ed)[2], before[2]);
    // Left / first-line / right indents on their own; the list indent stays.
    let after = with_undo(
        &mut ed,
        0,
        0,
        ListChange::Indents {
            left_pt: Some(54.0),
            first_line_pt: Some(-10.0),
            right_pt: Some(12.0),
        },
    );
    assert_eq!(
        after[0].indents,
        ParaIndents {
            left_pt: 54.0,
            first_line_pt: -10.0,
            right_pt: 12.0
        }
    );
    assert_eq!(after[0].list.as_ref().unwrap().list_indent_pt, 36.0);
}

// -------------------------------------------------------------------- save

#[kernpage_samples::test]
fn numbered_types_and_separators_survive_save_and_reparse() {
    use ListType::*;
    let kinds = [Arabic, UpperRoman, LowerRoman, UpperLetter, LowerLetter];
    for kind in kinds {
        for chunk in [0u8, 3, 6].map(|c| c..(c + 3).min(8)) {
            let (source, mut ed) = open("list-num-types");
            for (i, sep) in chunk.clone().enumerate() {
                let mut info = ListInfo::numbered(kind);
                info.separator = sep;
                list_cmd(&mut ed, i * 3, i * 3 + 2, ListChange::Set(info));
            }
            let want = paras(&ed);
            let (_, back) = saved(&source, &ed);
            let got = read_paras(&back);
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(&want) {
                assert_eq!((g.list.clone(), g.indents), model_state(w), "{kind:?}");
            }
            assert_eq!(
                pub_format::list_labels(&got),
                labels(&want),
                "{kind:?} {chunk:?}"
            );
        }
    }
}

#[kernpage_samples::test]
fn bullets_start_at_and_indents_survive_save_and_reparse() {
    let (source, mut ed) = open("list-num-types");
    list_cmd(&mut ed, 0, 8, ListChange::Clear);
    list_cmd(&mut ed, 0, 0, ListChange::Set(ListInfo::bullet()));
    list_cmd(&mut ed, 1, 1, ListChange::Set(ListInfo::bullet_with(0xA7, "Wingdings", 14.0)));
    let mut start5 = ListInfo::numbered(ListType::Arabic);
    start5.start = 5;
    list_cmd(&mut ed, 2, 3, ListChange::Set(start5));
    list_cmd(&mut ed, 3, 3, ListChange::ListIndent(36.0));
    list_cmd(
        &mut ed,
        4,
        4,
        ListChange::Indents {
            left_pt: Some(54.0),
            first_line_pt: Some(-10.0),
            right_pt: Some(12.0),
        },
    );
    let want = paras(&ed);
    let (_, back) = saved(&source, &ed);
    let got = read_paras(&back);
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        assert_eq!((g.list.clone(), g.indents), model_state(w), "paragraph {i}");
    }
    assert_eq!(pub_format::list_labels(&got)[..4], [s("\u{b7}"), s("\u{a7}"), s("5."), s("6.")]);
}

#[kernpage_samples::test]
fn clearing_a_list_saves_as_plain_text() {
    let (source, mut ed) = open("list-num-mixed");
    list_cmd(&mut ed, 0, 1, ListChange::Clear);
    let (_, back) = saved(&source, &ed);
    let got = read_paras(&back);
    assert!(got[0].list.is_none() && got[1].list.is_none());
    assert_eq!(got[0].indents, ParaIndents::default());
    // The paragraphs after the plain one keep their numbering.
    assert!(got[3].list.is_some() && got[4].list.is_some());
    assert_eq!(pub_format::list_labels(&got)[3..5], [s("1."), s("2.")]);
    // And the text is untouched.
    assert_eq!(got[0].runs[0].text, "One\r");
}

#[kernpage_samples::test]
fn list_survives_text_edits_and_a_new_paragraph() {
    let (source, mut ed) = open("list-num-1dot");
    let (page, id) = the_box(ed.doc());
    let ps = paras(&ed);
    // Retype "Two" and add a paragraph after it: the new one is in the list too.
    let text = "One\nTwo!\nTwo b\nThree\nFour\nFive";
    let _ = ps;
    let new = kernpage_core::text::rebuild_paragraphs(&paras(&ed), text);
    ed.execute(Command::SetText {
        page,
        id,
        paragraphs: new,
    })
    .unwrap();
    let want = paras(&ed);
    assert!(want[2].list.is_some(), "a split paragraph keeps the list");
    let (_, back) = saved(&source, &ed);
    let got = read_paras(&back);
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert_eq!((g.list.clone(), g.indents), model_state(w));
    }
}

#[kernpage_samples::test]
fn unchanged_samples_save_with_their_lists_untouched() {
    let mut n = 0;
    for path in kernpage_samples::files("gen", "pub") {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if !name.starts_with("list-") {
            continue;
        }
        let (source, ed) = open(&name);
        let plan = source.plan(ed.doc());
        assert!(plan.is_empty(), "{name}: {:?}", plan.unsupported);
        // Saving rewrites nothing in any story (only the document save counter changes):
        // every paragraph record, lists and indents included, is byte for byte the same.
        let (out, _) = source.build(ed.doc()).unwrap();
        let count = pub_format::read_bytes(source.original_bytes())
            .unwrap()
            .stories
            .len();
        for story in 0..count {
            let a = pub_write::story_records(source.original_bytes(), story).unwrap();
            let b = pub_write::story_records(&out, story).unwrap();
            assert_eq!(a.para_records, b.para_records, "{name} story {story}");
        }
        n += 1;
    }
    let want = if kernpage_samples::extra_samples().is_some() { 20 } else { 1 };
    assert!(n >= want, "{n}");
}

// ------------------------------------------------------------- AutoRecover

fn meta() -> Meta {
    Meta {
        original_path: None,
        display_name: "t.pub".into(),
        saved_at: 1,
        pid: 1,
    }
}

#[kernpage_samples::test]
fn autorecover_keeps_lists_and_indents() {
    let (_, mut ed) = open("list-num-types");
    list_cmd(&mut ed, 0, 8, ListChange::Clear);
    list_cmd(&mut ed, 0, 1, ListChange::Set(ListInfo::bullet_with(0xB7, "Symbol", 12.0)));
    let mut roman = ListInfo::numbered(ListType::LowerRoman);
    roman.separator = 6;
    roman.start = 4;
    list_cmd(&mut ed, 2, 4, ListChange::Set(roman));
    list_cmd(
        &mut ed,
        5,
        5,
        ListChange::Indents {
            left_pt: Some(30.0),
            first_line_pt: Some(-5.0),
            right_pt: Some(7.0),
        },
    );
    let snap = decode(&encode(&meta(), ed.doc()).unwrap()).unwrap();
    assert_eq!(&snap.document, ed.doc());
    let (page, id) = the_box(&snap.document);
    let ShapeKind::TextBox { paragraphs } = &snap.document.shape(page, id).unwrap().kind else {
        unreachable!()
    };
    assert_eq!(paragraphs, &paras(&ed));
    assert_eq!(labels(paragraphs)[2..5], [s("[iv]"), s("[v]"), s("[vi]")]);
}

#[kernpage_samples::test]
fn an_old_snapshot_without_list_fields_still_loads() {
    let (_, ed) = open("list-none");
    let mut v: serde_json::Value =
        serde_json::from_slice(&encode(&meta(), ed.doc()).unwrap()).unwrap();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("list");
                m.remove("indents");
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    strip(&mut v);
    let snap = decode(&serde_json::to_vec(&v).unwrap()).unwrap();
    let (page, id) = the_box(&snap.document);
    let ShapeKind::TextBox { paragraphs } = &snap.document.shape(page, id).unwrap().kind else {
        unreachable!()
    };
    assert!(paragraphs
        .iter()
        .all(|p| p.list.is_none() && p.indents == ParaIndents::default()));
}

#[kernpage_samples::test]
fn formatting_a_list_paragraph_keeps_its_list_data() {
    for name in ["list-bullet-wingdings", "list-num-start5", "list-bullet-indent", "list-num-indent36"] {
        let (source, mut ed) = open(name);
        let want = paras(&ed);
        let (page, id) = the_box(ed.doc());
        let range = range_of(&want, 0, want.len() - 1);
        ed.execute(Command::FormatParagraphs {
            page,
            id,
            cell: None,
            range,
            align: kernpage_core::Align::Center,
        })
        .unwrap();
        let (_, back) = saved(&source, &ed);
        let got = read_paras(&back);
        for (g, w) in got.iter().zip(&want) {
            assert_eq!((g.list.clone(), g.indents), model_state(w), "{name}");
            assert_eq!(g.alignment, pub_format::Alignment::Center, "{name}");
        }
    }
}

/// Writes edited files for checking by hand in Publisher: set `KERNPAGE_LIST_OUT` to a
/// folder. Does nothing otherwise.
#[kernpage_samples::test]
fn write_files_for_manual_check() {
    let Some(dir) = std::env::var_os("KERNPAGE_LIST_OUT") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, source: &SaveSource, ed: &Editor| {
        let (out, _) = saved(source, ed);
        std::fs::write(dir.join(format!("{name}.pub")), out).unwrap();
    };
    let num = |k, sep: u8, start: u32| {
        let mut i = ListInfo::numbered(k);
        i.separator = sep;
        i.start = start;
        i
    };
    use ListType::*;

    let (src, mut ed) = open("list-none");
    list_cmd(&mut ed, 1, 1, ListChange::Set(ListInfo::bullet()));
    write("out-1-bullet-middle", &src, &ed);

    let (src, mut ed) = open("list-none");
    list_cmd(&mut ed, 0, 2, ListChange::Set(ListInfo::bullet_with(0xA7, "Wingdings", 14.0)));
    write("out-2-bullet-wingdings", &src, &ed);

    let (src, mut ed) = open("list-num-types");
    list_cmd(&mut ed, 0, 2, ListChange::Set(num(Arabic, 0, 1)));
    list_cmd(&mut ed, 3, 5, ListChange::Set(num(UpperRoman, 2, 1)));
    list_cmd(&mut ed, 6, 8, ListChange::Set(num(LowerLetter, 1, 1)));
    write("out-3-types-and-separators", &src, &ed);

    let (src, mut ed) = open("list-none");
    list_cmd(&mut ed, 0, 2, ListChange::Set(num(Arabic, 2, 5)));
    write("out-4-start-at-5", &src, &ed);

    let (src, mut ed) = open("list-num-types");
    list_cmd(&mut ed, 0, 8, ListChange::Clear);
    list_cmd(&mut ed, 0, 1, ListChange::Set(num(Arabic, 2, 1)));
    list_cmd(&mut ed, 3, 4, ListChange::Set(num(Arabic, 2, 1)));
    list_cmd(&mut ed, 5, 5, ListChange::Set(ListInfo::bullet()));
    list_cmd(&mut ed, 6, 7, ListChange::Set(num(Arabic, 2, 1)));
    write("out-5-restarts", &src, &ed);

    let (src, mut ed) = open("list-num-mixed");
    list_cmd(&mut ed, 0, 1, ListChange::Clear);
    write("out-6-cleared-first-two", &src, &ed);

    let (src, mut ed) = open("list-none");
    list_cmd(&mut ed, 0, 1, ListChange::Set(num(Arabic, 2, 1)));
    list_cmd(&mut ed, 0, 0, ListChange::ListIndent(36.0));
    list_cmd(
        &mut ed,
        1,
        2,
        ListChange::Indents {
            left_pt: Some(54.0),
            first_line_pt: Some(-10.0),
            right_pt: Some(12.0),
        },
    );
    write("out-7-indents", &src, &ed);

    let (src, mut ed) = open("list-num-1dot");
    let (page, id) = the_box(ed.doc());
    let new = kernpage_core::text::rebuild_paragraphs(
        &paras(&ed),
        "One\nTwo edited\nTwo and a half\nThree\nFour\nFive",
    );
    ed.execute(Command::SetText { page, id, paragraphs: new }).unwrap();
    write("out-8-text-edit-new-paragraph", &src, &ed);

    let (src, mut ed) = open("list-bullet-wingdings");
    let ps = paras(&ed);
    let (page, id) = the_box(ed.doc());
    ed.execute(Command::FormatParagraphs {
        page,
        id,
        cell: None,
        range: range_of(&ps, 0, ps.len() - 1),
        align: kernpage_core::Align::Center,
    })
    .unwrap();
    write("out-9-wingdings-centered", &src, &ed);
}

#[kernpage_samples::test]
fn step_indent_steps_each_paragraph_from_its_own_value_and_stops_at_zero() {
    let (_, mut ed) = open("list-none");
    list_cmd(&mut ed, 0, 0, ListChange::Set(ListInfo::bullet()));
    let after = with_undo(&mut ed, 0, 1, ListChange::StepIndent(18.0));
    assert_eq!(after[0].list.as_ref().unwrap().list_indent_pt, 36.0);
    assert_eq!(after[0].indents.left_pt, 36.0);
    assert_eq!(after[0].indents.first_line_pt, -36.0);
    assert_eq!(after[1].list, None);
    assert_eq!(after[1].indents.left_pt, 18.0, "a plain paragraph steps its left indent");
    list_cmd(&mut ed, 0, 1, ListChange::StepIndent(-18.0));
    let after = with_undo(&mut ed, 0, 1, ListChange::StepIndent(-18.0));
    assert_eq!(after[0].list.as_ref().unwrap().list_indent_pt, 0.0);
    assert_eq!(after[1].indents.left_pt, 0.0);
}
