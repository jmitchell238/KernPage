//! Superscript and subscript in the editor (spec 4S): the format command with undo, saving,
//! AutoRecover (including old snapshots) and the selection format. Text is `H2O x2 One`.

use kernpage_core::autorecover::{decode, encode, Meta};
use kernpage_core::text::selection_format;
use kernpage_core::{
    from_pub_flowed, CharChange, Command, Document, Editor, Paragraph, SaveSource, Script, ShapeId, ShapeKind,
};

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

fn the_box(doc: &Document) -> (usize, ShapeId) {
    for (pi, p) in doc.pages.iter().enumerate() {
        for s in &p.shapes {
            if let ShapeKind::TextBox { paragraphs } = &s.kind {
                if paragraphs.iter().any(|q| q.text().contains("H2O")) {
                    return (pi, s.id);
                }
            }
        }
    }
    panic!("no text box");
}

fn paras(ed: &Editor) -> Vec<Paragraph> {
    let (page, id) = the_box(ed.doc());
    match &ed.doc().shape(page, id).unwrap().kind {
        ShapeKind::TextBox { paragraphs } => paragraphs.clone(),
        _ => unreachable!(),
    }
}

/// (text, script) of the first paragraph's runs.
fn view(ps: &[Paragraph]) -> Vec<(String, Script)> {
    ps[0].runs.iter().filter(|r| !r.text.is_empty()).map(|r| (r.text.clone(), r.script)).collect()
}

fn set(ed: &mut Editor, range: std::ops::Range<usize>, s: Script) {
    let (page, id) = the_box(ed.doc());
    ed.execute(Command::FormatRuns { page, id, cell: None, range, change: CharChange::Script(s) })
        .unwrap();
}

fn t(s: &str, x: Script) -> (String, Script) {
    (s.to_owned(), x)
}

#[kernpage_samples::test]
fn reads_from_publisher_files() {
    let (_, ed) = open("script-super");
    assert_eq!(
        view(&paras(&ed)),
        [t("H2O x", Script::Normal), t("2", Script::Superscript), t(" One", Script::Normal)]
    );
}

#[kernpage_samples::test]
fn command_undo_redo_and_exclusive_value() {
    let (_, mut ed) = open("script-plain");
    let before = paras(&ed);
    set(&mut ed, 5..6, Script::Superscript);
    let sup = paras(&ed);
    assert_eq!(view(&sup)[1], t("2", Script::Superscript));
    set(&mut ed, 5..6, Script::Subscript);
    assert_eq!(view(&paras(&ed))[1], t("2", Script::Subscript));
    set(&mut ed, 5..6, Script::Normal);
    assert_eq!(paras(&ed), before);
    assert!(ed.undo() && ed.undo());
    assert_eq!(paras(&ed), sup);
    assert!(ed.undo());
    assert_eq!(paras(&ed), before);
    assert!(ed.redo());
    assert_eq!(paras(&ed), sup);
}

#[kernpage_samples::test]
fn selection_format_reports_script() {
    let (_, mut ed) = open("script-plain");
    set(&mut ed, 5..6, Script::Superscript);
    let ps = paras(&ed);
    assert_eq!(selection_format(&ps, 5..6).script, Some(Script::Superscript));
    assert_eq!(selection_format(&ps, 0..4).script, Some(Script::Normal));
    assert_eq!(selection_format(&ps, 0..10).script, None);
    // Clear All Formatting removes it.
    let (page, id) = the_box(ed.doc());
    ed.execute(Command::FormatRuns { page, id, cell: None, range: 0..10, change: CharChange::Clear }).unwrap();
    assert_eq!(selection_format(&paras(&ed), 0..10).script, Some(Script::Normal));
}

#[kernpage_samples::test]
fn survives_save_and_reparse() {
    let (source, mut ed) = open("script-plain");
    set(&mut ed, 1..2, Script::Subscript);
    set(&mut ed, 5..6, Script::Superscript);
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let back = pub_format::read_bytes(&out).unwrap();
    let runs: Vec<_> = back
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .flat_map(|s| &s.paragraphs)
        .flat_map(|p| &p.runs)
        .filter(|r| r.script != pub_format::Script::Normal)
        .map(|r| (r.text.clone(), r.script))
        .collect();
    assert_eq!(
        runs,
        [("2".to_owned(), pub_format::Script::Subscript), ("2".to_owned(), pub_format::Script::Superscript)]
    );
    // Clearing it again saves it away.
    set(&mut ed, 1..2, Script::Normal);
    set(&mut ed, 5..6, Script::Normal);
    let (out, _) = source.build(ed.doc()).unwrap();
    let back = pub_format::read_bytes(&out).unwrap();
    assert!(back.pages.iter().flat_map(|p| &p.shapes).flat_map(|s| &s.paragraphs).flat_map(|p| &p.runs)
        .all(|r| r.script == pub_format::Script::Normal));
}

#[kernpage_samples::test]
fn autorecover_keeps_script_and_old_snapshots_load() {
    let (_, mut ed) = open("script-plain");
    set(&mut ed, 5..6, Script::Superscript);
    let meta = Meta { original_path: None, display_name: "t.pub".into(), saved_at: 1, pid: 1 };
    let snap = decode(&encode(&meta, ed.doc()).unwrap()).unwrap();
    assert_eq!(&snap.document, ed.doc());
    // An old snapshot has no "script" key in its runs.
    let mut v: serde_json::Value = serde_json::from_slice(&encode(&meta, ed.doc()).unwrap()).unwrap();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("script");
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    strip(&mut v);
    let old = decode(&serde_json::to_vec(&v).unwrap()).unwrap();
    let (page, id) = the_box(&old.document);
    let ShapeKind::TextBox { paragraphs } = &old.document.shape(page, id).unwrap().kind else { unreachable!() };
    assert!(paragraphs[0].runs.iter().all(|r| r.script == Script::Normal));
}
