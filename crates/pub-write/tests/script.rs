//! Writing superscript / subscript (spec 4S) onto Publisher-saved `samples/gen/script-*.pub`.

use pub_format::Script;
use pub_write::{set_story_format, CharFormat, CharSpan};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")))).unwrap()
}

fn story(bytes: &[u8]) -> usize {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.stories.iter().position(|t| t.contains("H2O")).unwrap()
}

fn fmt(script: Script) -> CharFormat {
    CharFormat { bold: false, italic: false, underline: false, script, size_pt: 10.0, font: "Calibri".into(), color: None, scheme_color: None }
}

fn scripts(bytes: &[u8]) -> Vec<(String, Script)> {
    let doc = pub_format::read_bytes(bytes).unwrap();
    doc.pages[0].shapes.iter().flat_map(|s| s.paragraphs.iter())
        .flat_map(|p| p.runs.iter())
        .map(|r| (r.text.replace('\r', ""), r.script))
        .filter(|r| !r.0.is_empty())
        .collect()
}

fn span(start: usize, end: usize, script: Script) -> CharSpan {
    CharSpan { start, end, format: fmt(script) }
}

#[kernpage_samples::test]
fn superscript_matches_publisher_bytes() {
    let plain = gen("script-plain");
    let out = set_story_format(&plain, story(&plain), &[span(5, 6, Script::Superscript)], &[]).unwrap();
    assert_eq!(scripts(&out), scripts(&gen("script-super")));
}

#[kernpage_samples::test]
fn subscript_and_both_round_trip() {
    let plain = gen("script-plain");
    let st = story(&plain);
    let out = set_story_format(&plain, st, &[span(1, 2, Script::Subscript), span(5, 6, Script::Superscript)], &[]).unwrap();
    let r = scripts(&out);
    assert!(r.contains(&("2".into(), Script::Subscript)) && r.contains(&("2".into(), Script::Superscript)));
    assert_eq!(r, scripts(&gen("script-both")));
}

#[kernpage_samples::test]
fn clearing_removes_the_item() {
    let sup = gen("script-super");
    let out = set_story_format(&sup, story(&sup), &[span(5, 6, Script::Normal)], &[]).unwrap();
    assert!(scripts(&out).iter().all(|r| r.1 == Script::Normal));
}

#[kernpage_samples::test]
fn super_keeps_explicit_size_and_switches_to_sub() {
    let sup = gen("script-super20");
    let st = story(&sup);
    let mut f = fmt(Script::Subscript);
    f.size_pt = 20.0;
    let out = set_story_format(&sup, st, &[CharSpan { start: 5, end: 6, format: f }], &[]).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let run = doc.pages[0].shapes.iter().flat_map(|s| s.paragraphs.iter()).flat_map(|p| p.runs.iter())
        .find(|r| r.script == Script::Subscript).unwrap();
    assert_eq!(run.size_pt, 20.0);
}

/// Writes edited files for the Publisher check when `PW_SCRIPT_OUT` names a folder.
#[kernpage_samples::test]
fn write_check_files() {
    let Ok(dir) = std::env::var("PW_SCRIPT_OUT") else { return };
    let dir = PathBuf::from(dir);
    let plain = gen("script-plain");
    let st = story(&plain);
    let both = set_story_format(&plain, st, &[span(1, 2, Script::Subscript), span(5, 6, Script::Superscript)], &[]).unwrap();
    std::fs::write(dir.join("w-both.pub"), both).unwrap();
    let sup = gen("script-super20");
    let mut f = fmt(Script::Subscript);
    f.size_pt = 20.0;
    let sub20 = set_story_format(&sup, story(&sup), &[CharSpan { start: 5, end: 6, format: f }], &[]).unwrap();
    std::fs::write(dir.join("w-sub20.pub"), sub20).unwrap();
    let cleared = set_story_format(&gen("script-super"), st, &[span(5, 6, Script::Normal)], &[]).unwrap();
    std::fs::write(dir.join("w-cleared.pub"), cleared).unwrap();
}
