//! Saving a linked chain whose last frame overflows must keep the hidden overflow text
//! (pw-7vgx): the model only holds the visible frame slices, the tail lives in `SaveSource`.

use kernpage_core::{Command, Editor, Paragraph, SaveSource, ShapeId, ShapeKind};

fn sample() -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen");
    std::fs::read(kernpage_samples::resolve(p.join("linked.pub"))).unwrap()
}

/// Open with the real chain breaks, or with `last_end` replacing the end of the last frame.
fn open_with(bytes: &[u8], last_end: Option<(usize, usize)>) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let mut breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    if let Some(e) = last_end {
        *breaks[0].last_mut().unwrap() = e;
    }
    let model = kernpage_core::from_pub_flowed("t.pub", &src, &breaks);
    let source = SaveSource::new(bytes.to_vec(), &src, &model, &breaks);
    (source, Editor::new(model))
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    open_with(bytes, None)
}

/// The whole story of the first chain, one entry per character with its formatting;
/// paragraph breaks are `'\n'` entries.
type Flat = Vec<(char, bool, bool, u32, bool)>;

fn story_flat(bytes: &[u8]) -> (Flat, usize) {
    let d = pub_format::read_bytes(bytes).unwrap();
    let chain = d.chains().into_iter().find(|c| c.len() > 1).unwrap();
    let paras = chain
        .iter()
        .map(|r| &d.pages[r.page].shapes[r.shape])
        .find(|s| !s.paragraphs.is_empty())
        .map(|s| &s.paragraphs)
        .unwrap();
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            out.push(('\n', false, false, 0, false));
        }
        for r in &p.runs {
            for ch in r.text.chars().filter(|c| *c != '\r' && *c != '\n') {
                out.push((ch, r.bold, r.italic, (r.size_pt * 100.0) as u32, r.underline));
            }
        }
    }
    (out, paras.len())
}

fn text_of(f: &[(char, bool, bool, u32, bool)]) -> String {
    f.iter().map(|e| e.0).collect()
}

fn frames(ed: &Editor) -> Vec<(usize, ShapeId)> {
    let src = pub_format::read_bytes(&sample()).unwrap();
    let chain = src.chains().into_iter().find(|c| c.len() > 1).unwrap();
    chain
        .iter()
        .map(|r| (r.page, ed.doc().pages[r.page].shapes[r.shape].id))
        .collect()
}

fn prepend(ed: &mut Editor, page: usize, id: ShapeId, tag: &str) {
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    let mut new: Vec<Paragraph> = paragraphs.clone();
    new[0].runs[0].text = format!("{tag}{}", new[0].runs[0].text);
    ed.execute(Command::SetText { page, id, paragraphs: new }).unwrap();
}

fn append(ed: &mut Editor, page: usize, id: ShapeId, tag: &str) {
    let ShapeKind::TextBox { paragraphs } = &ed.doc().shape(page, id).unwrap().kind else {
        panic!()
    };
    let mut new: Vec<Paragraph> = paragraphs.clone();
    new.last_mut().unwrap().runs.last_mut().unwrap().text.push_str(tag);
    ed.execute(Command::SetText { page, id, paragraphs: new }).unwrap();
}

fn save(source: &mut SaveSource, ed: &Editor) -> Vec<u8> {
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    source.mark_saved(out.clone(), ed.doc(), &plan);
    out
}

/// The saved story must be the original with `tag` inserted at `at` (a char index) and
/// every other character and its formatting unchanged.
fn assert_only_inserted(orig: &Flat, new: &Flat, at: usize, tag: &str) {
    let n = tag.chars().count();
    assert_eq!(new.len(), orig.len() + n, "length changed by exactly the edit");
    assert_eq!(&new[..at], &orig[..at]);
    assert_eq!(text_of(&new[at..at + n]), tag);
    assert_eq!(&new[at + n..], &orig[at..], "tail incl. formatting identical");
}

#[kernpage_samples::test]
fn edit_frame_one_keeps_the_overflow_tail() {
    let bytes = sample();
    let (orig, orig_paras) = story_flat(&bytes);
    assert!(orig_paras >= 30, "sample story has an overflow tail");
    let (mut source, mut ed) = open(&bytes);
    let f = frames(&ed);
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    let out = save(&mut source, &ed);
    let (new, new_paras) = story_flat(&out);
    assert_eq!(new_paras, orig_paras);
    assert_only_inserted(&orig, &new, 0, "NEW ");
    assert!(text_of(&new).contains("Paragraph 30"));
    // Saved twice: nothing left to write, and a second edit keeps the tail too.
    assert!(source.plan(ed.doc()).is_empty());
    append(&mut ed, f[0].0, f[0].1, "!");
    let out2 = save(&mut source, &ed);
    let (new2, p2) = story_flat(&out2);
    assert_eq!(p2, orig_paras);
    assert_eq!(new2.len(), orig.len() + 5);
    assert_eq!(new2[new2.len() - 100..], orig[orig.len() - 100..]);
}

#[kernpage_samples::test]
fn edit_middle_and_last_frame_keep_the_tail() {
    let bytes = sample();
    let (orig, orig_paras) = story_flat(&bytes);
    for which in [1usize, 2] {
        let (mut source, mut ed) = open(&bytes);
        let f = frames(&ed);
        prepend(&mut ed, f[which].0, f[which].1, "ZZ ");
        let out = save(&mut source, &ed);
        let (new, paras) = story_flat(&out);
        assert_eq!(paras, orig_paras, "frame {which}");
        assert_eq!(new.len(), orig.len() + 3, "frame {which}");
        assert_eq!(
            new[new.len() - 100..],
            orig[orig.len() - 100..],
            "tail identical (frame {which})"
        );
    }
    // Editing the end of the last frame keeps the tail after it.
    let (mut source, mut ed) = open(&bytes);
    let f = frames(&ed);
    append(&mut ed, f[2].0, f[2].1, "QQ");
    let (new, paras) = story_flat(&save(&mut source, &ed));
    assert_eq!(paras, orig_paras);
    assert_eq!(new.len(), orig.len() + 2);
    assert_eq!(new[new.len() - 100..], orig[orig.len() - 100..]);
}

#[kernpage_samples::test]
fn unedited_document_plans_nothing() {
    let (source, ed) = open(&sample());
    assert!(source.plan(ed.doc()).is_empty());
}

#[kernpage_samples::test]
fn chain_without_overflow_is_unchanged() {
    let bytes = sample();
    let (orig, n) = story_flat(&bytes);
    // Pretend the last frame ends at the end of the story: no tail.
    let (mut source, mut ed) = open_with(&bytes, Some((n, 0)));
    let f = frames(&ed);
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    let (new, paras) = story_flat(&save(&mut source, &ed));
    assert_eq!(paras, n);
    assert_only_inserted(&orig, &new, 0, "NEW ");
}

#[kernpage_samples::test]
fn break_in_the_middle_of_a_paragraph_joins_the_tail() {
    let bytes = sample();
    let (orig, n) = story_flat(&bytes);
    // The last frame ends 5 glyphs into paragraph 16 (0-based): the tail continues it.
    let (mut source, mut ed) = open_with(&bytes, Some((16, 5)));
    let f = frames(&ed);
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    let (new, paras) = story_flat(&save(&mut source, &ed));
    assert_eq!(paras, n, "no extra paragraph mark");
    assert_only_inserted(&orig, &new, 0, "NEW ");
}

#[kernpage_samples::test]
fn deleting_a_frame_is_reported_not_silently_lossy() {
    let bytes = sample();
    let (source, mut ed) = open(&bytes);
    let f = frames(&ed);
    ed.execute(Command::DeleteShapes { page: f[1].0, ids: vec![f[1].1] }).unwrap();
    assert!(!source.plan(ed.doc()).unsupported.is_empty(), "frame deletion must be flagged");
    // Deleting every frame is flagged too.
    let (source, mut ed) = open(&bytes);
    for (p, id) in frames(&ed) {
        ed.execute(Command::DeleteShapes { page: p, ids: vec![id] }).unwrap();
    }
    assert!(!source.plan(ed.doc()).unsupported.is_empty());
}

/// Writes the edited files for a Publisher 2021 check:
/// `cargo test -p kernpage-core --test linked_save '--' '--ignored' write_linked_fix_files`
#[kernpage_samples::test]
#[ignore = "writes files for Publisher to open"]
fn write_linked_fix_files() {
    let dir = &std::env::temp_dir().join("pw-linkedfix");
    std::fs::create_dir_all(dir).unwrap();
    let bytes = sample();
    for (name, which) in [("linked-edit-frame1.pub", 0usize), ("linked-edit-frame3.pub", 2)] {
        let (mut source, mut ed) = open(&bytes);
        let f = frames(&ed);
        prepend(&mut ed, f[which].0, f[which].1, "NEW ");
        std::fs::write(dir.join(name), save(&mut source, &ed)).unwrap();
    }
}
