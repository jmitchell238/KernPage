//! Superscript and subscript (spec 4S) read from the Publisher-saved `samples/gen/script-*.pub`.
//! Text `H2O x2 One`: the sub sample marks char 1 ('2' of H2O), the super sample char 5 ('2' of x2).

use pub_format::Script;
use std::path::PathBuf;

/// (text, script) of each run of the box's first paragraph, paragraph mark dropped.
pub fn runs(name: &str) -> Vec<(String, Script)> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub"));
    let doc = pub_format::read(&kernpage_samples::resolve(&p)).unwrap();
    let shape = doc
        .pages
        .iter()
        .flat_map(|pg| pg.shapes.iter())
        .find(|s| s.text.as_deref().is_some_and(|t| t.contains("One")))
        .unwrap();
    shape.paragraphs[0]
        .runs
        .iter()
        .map(|r| (r.text.replace('\r', ""), r.script))
        .filter(|r| !r.0.is_empty())
        .collect()
}

fn s(t: &str, x: Script) -> (String, Script) {
    (t.to_owned(), x)
}

#[kernpage_samples::test]
fn plain_and_cleared_have_no_script() {
    for n in ["script-plain", "script-superoff", "script-size20"] {
        assert!(runs(n).iter().all(|r| r.1 == Script::Normal), "{n}");
    }
}

#[kernpage_samples::test]
fn single_characters() {
    assert_eq!(
        runs("script-super"),
        [s("H2O x", Script::Normal), s("2", Script::Superscript), s(" One", Script::Normal)]
    );
    assert_eq!(
        runs("script-sub"),
        [s("H", Script::Normal), s("2", Script::Subscript), s("O x2 One", Script::Normal)]
    );
}

#[kernpage_samples::test]
fn both_in_one_box() {
    let r = runs("script-both");
    assert!(r.contains(&s("2", Script::Subscript)) && r.contains(&s("2", Script::Superscript)));
}

#[kernpage_samples::test]
fn script_keeps_the_nominal_size() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen/script-super20.pub");
    let doc = pub_format::read(&kernpage_samples::resolve(&p)).unwrap();
    let sup: Vec<_> = doc
        .pages
        .iter()
        .flat_map(|pg| pg.shapes.iter())
        .flat_map(|s| s.paragraphs.iter())
        .flat_map(|p| p.runs.iter())
        .filter(|r| r.script == Script::Superscript)
        .collect();
    assert_eq!(sup.len(), 1, "{sup:?}");
    assert_eq!(sup[0].size_pt, 20.0);
}

#[kernpage_samples::test]
fn whole_range_superscript() {
    let r = runs("script-superall");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], s("H2O x2 One", Script::Superscript));
}
