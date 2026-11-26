//! Linked stories, phase 1 (docs/design/LINKED-STORIES.md): the model keeps the whole story
//! of every chain, saving writes it (overflow included), AutoRecover snapshots carry it.

use kernpage_core::autorecover::{decode, encode, Meta};
use kernpage_core::{
    from_pub, from_pub_flowed, Align, CharChange, Command, Document, Editor, Paragraph, Rect,
    SaveSource, Shape, ShapeId, ShapeKind, StoryPos, WrapType,
};

fn sample() -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen");
    std::fs::read(kernpage_samples::resolve(p.join("linked.pub"))).unwrap()
}

fn real_breaks(src: &pub_format::Document) -> Vec<Vec<StoryPos>> {
    pub2pdf::chain_breaks(src, &pub2pdf::Options::default())
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = real_breaks(&src);
    let model = from_pub_flowed("t.pub", &src, &breaks);
    (SaveSource::new(bytes.to_vec(), &src, &model, &breaks), Editor::new(model))
}

fn meta() -> Meta {
    Meta {
        original_path: Some("linked.pub".into()),
        display_name: "linked.pub".into(),
        saved_at: 1,
        pid: 1,
    }
}

fn frame_ids(doc: &Document) -> Vec<(usize, ShapeId)> {
    let src = pub_format::read_bytes(&sample()).unwrap();
    let chain = src.chains().into_iter().find(|c| c.len() > 1).unwrap();
    chain.iter().map(|r| (r.page, doc.pages[r.page].shapes[r.shape].id)).collect()
}

fn paragraphs(doc: &Document, page: usize, id: ShapeId) -> Vec<Paragraph> {
    let ShapeKind::TextBox { paragraphs } = &doc.shape(page, id).unwrap().kind else {
        panic!("not a text box")
    };
    paragraphs.clone()
}

fn prepend(ed: &mut Editor, page: usize, id: ShapeId, tag: &str) {
    let mut new = paragraphs(ed.doc(), page, id);
    new[0].runs[0].text = format!("{tag}{}", new[0].runs[0].text);
    ed.execute(Command::SetText { page, id, paragraphs: new }).unwrap();
}

/// Plain text of the first chain's story in a saved file, one entry per paragraph.
fn saved_story(bytes: &[u8]) -> Vec<String> {
    let d = pub_format::read_bytes(bytes).unwrap();
    let chain = d.chains().into_iter().find(|c| c.len() > 1).unwrap();
    chain
        .iter()
        .map(|r| &d.pages[r.page].shapes[r.shape])
        .find(|s| !s.paragraphs.is_empty())
        .map(|s| {
            s.paragraphs
                .iter()
                .map(|p| p.runs.iter().map(|r| r.text.replace('\r', "")).collect::<String>())
                .collect()
        })
        .unwrap()
}

fn save(source: &mut SaveSource, doc: &Document) -> Vec<u8> {
    let (out, plan) = source.build(doc).unwrap();
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    source.mark_saved(out.clone(), doc, &plan);
    out
}

#[kernpage_samples::test]
fn opening_a_linked_file_makes_one_story_with_the_overflow() {
    let bytes = sample();
    let src = pub_format::read_bytes(&bytes).unwrap();
    let breaks = real_breaks(&src);
    let doc = from_pub_flowed("t", &src, &breaks);
    assert_eq!(doc.stories.len(), 1);
    let story = &doc.stories[0];
    let frames = frame_ids(&doc);
    assert_eq!(story.frames, frames.iter().map(|f| f.1).collect::<Vec<_>>());
    assert_eq!(story.ends, breaks[0]);
    let source_paras = &src.chains()[0]
        .iter()
        .map(|r| &src.pages[r.page].shapes[r.shape])
        .find(|s| !s.paragraphs.is_empty())
        .unwrap()
        .paragraphs;
    assert_eq!(story.paragraphs.len(), source_paras.len());
    assert!(story.paragraphs.len() >= 30);
    // The overflow is what no frame shows.
    let overflow = story.overflow();
    assert!(!overflow.is_empty());
    assert!(overflow.last().unwrap().text().contains("Paragraph 30"));
    assert_eq!(story.end_of(2), *breaks[0].last().unwrap());
    // Every frame's cached paragraphs are its share of the story.
    for (i, &(page, id)) in frames.iter().enumerate() {
        let cache = paragraphs(&doc, page, id);
        let share = story.slice(i);
        let blank = |p: &[Paragraph]| p.is_empty() || (p.len() == 1 && p[0].text().is_empty());
        if blank(&share) {
            assert!(blank(&cache));
        } else {
            assert_eq!(cache, share, "frame {i}");
        }
        assert_eq!(doc.story_of(id).map(|s| s.id), Some(story.id));
        assert_eq!(story.frame_index(id), Some(i));
    }
    // The standalone box is in no story.
    let standalone = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| !story.frames.contains(&s.id) && s.is_text_box())
        .unwrap();
    assert!(doc.story_of(standalone.id).is_none());
    // The unedited document composes to exactly the story.
    assert_eq!(doc.story_text(story), story.paragraphs);
}

#[kernpage_samples::test]
fn an_unsplit_chain_keeps_the_whole_story_too() {
    let src = pub_format::read_bytes(&sample()).unwrap();
    let doc = from_pub("t", &src);
    let story = &doc.stories[0];
    assert_eq!(story.slice(0), story.paragraphs);
    assert!(story.slice(1).is_empty() && story.slice(2).is_empty());
    assert!(story.overflow().is_empty());
    assert_eq!(doc.story_text(story), story.paragraphs);
}

#[kernpage_samples::test]
fn documents_without_chains_have_no_story_and_a_story_round_trips_through_serde() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen");
    let plain = from_pub("b", &pub_format::read(&kernpage_samples::resolve(root.join("bulletin.pub"))).unwrap());
    assert!(plain.stories.is_empty());
    let (_, ed) = open(&sample());
    let json = serde_json::to_string(ed.doc()).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(&back, ed.doc());
    assert_eq!(back.stories, ed.doc().stories);
    // A document written before stories existed has none.
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v.as_object_mut().unwrap().remove("stories");
    let old: Document = serde_json::from_value(v).unwrap();
    assert!(old.stories.is_empty());
}

/// Saving with the story in the model must write exactly what the frames-plus-remembered-tail
/// logic of phase 0 wrote (a document with no stories takes that path).
#[kernpage_samples::test]
fn saving_with_a_story_equals_what_phase_0_wrote() {
    let bytes = sample();
    let src = pub_format::read_bytes(&bytes).unwrap();
    for last_end in [None, Some((16usize, 5usize)), Some((30, 0))] {
        let mut breaks = real_breaks(&src);
        if let Some(e) = last_end {
            *breaks[0].last_mut().unwrap() = e;
        }
        for which in [0usize, 1, 2] {
            let with = from_pub_flowed("t.pub", &src, &breaks);
            let mut without = with.clone();
            without.stories.clear();
            let mut outs = Vec::new();
            for model in [with, without] {
                let mut source = SaveSource::new(bytes.clone(), &src, &model, &breaks);
                let mut ed = Editor::new(model);
                let f = frame_ids(ed.doc());
                prepend(&mut ed, f[which].0, f[which].1, "NEW ");
                outs.push(save(&mut source, ed.doc()));
            }
            assert_eq!(outs[0], outs[1], "frame {which}, last end {last_end:?}");
            assert!(saved_story(&outs[0]).iter().any(|p| p.contains("Paragraph 30")));
        }
    }
}

#[kernpage_samples::test]
fn snapshot_v2_recovers_the_whole_story_and_saving_it_writes_all_of_it() {
    let bytes = sample();
    let src = pub_format::read_bytes(&bytes).unwrap();
    let original = saved_story(&bytes);
    let (_, ed) = open(&bytes);
    let snap = decode(&encode(&meta(), ed.doc()).unwrap()).unwrap();
    let recovered = snap.document;
    assert_eq!(recovered.stories, ed.doc().stories);
    assert!(recovered.stories[0].overflow().last().unwrap().text().contains("Paragraph 30"));

    // Saved against a source that remembers no overflow (breaks that say the last frame
    // ends the story), so only the story can supply it.
    let mut breaks = real_breaks(&src);
    *breaks[0].last_mut().unwrap() = (original.len(), 0);
    let mut source = SaveSource::new(bytes.clone(), &src, &recovered, &breaks);
    let mut ed = Editor::new(recovered);
    let f = frame_ids(ed.doc());
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    let saved = saved_story(&save(&mut source, ed.doc()));
    assert_eq!(saved.len(), original.len());
    assert!(saved[0].starts_with("NEW "));
    assert_eq!(saved[1..], original[1..]);
    assert!(saved.last().unwrap().contains("Paragraph 30"));
}

#[kernpage_samples::test]
fn a_version_1_snapshot_recovers_independent_frames_and_still_saves_the_tail() {
    let bytes = sample();
    let (_, ed) = open(&bytes);
    let mut v: serde_json::Value =
        serde_json::from_slice(&encode(&meta(), ed.doc()).unwrap()).unwrap();
    v["version"] = 1.into();
    v["document"].as_object_mut().unwrap().remove("stories");
    let recovered = decode(&serde_json::to_vec(&v).unwrap()).unwrap().document;
    assert!(recovered.stories.is_empty());
    assert_eq!(recovered.pages.len(), ed.doc().pages.len());
    // Opened against its file again, phase 0's remembered tail keeps the overflow.
    let src = pub_format::read_bytes(&bytes).unwrap();
    let breaks = real_breaks(&src);
    let mut source = SaveSource::new(bytes.clone(), &src, &recovered, &breaks);
    let mut ed = Editor::new(recovered);
    let f = frame_ids(ed.doc());
    prepend(&mut ed, f[1].0, f[1].1, "ZZ ");
    let saved = saved_story(&save(&mut source, ed.doc()));
    assert_eq!(saved.len(), saved_story(&bytes).len());
    assert!(saved.last().unwrap().contains("Paragraph 30"));
}

// ---- phase 2: commands re-flow the story ----------------------------------------------------------

type Frames = Vec<(usize, ShapeId)>;

fn opened() -> (Vec<u8>, SaveSource, Editor, Frames) {
    let bytes = sample();
    let (source, ed) = open(&bytes);
    let f = frame_ids(ed.doc());
    (bytes, source, ed, f)
}

fn flat(p: &[Paragraph]) -> String {
    p.iter().map(Paragraph::text).collect::<Vec<_>>().join("\n")
}

fn texts(p: &[Paragraph]) -> Vec<String> {
    p.iter().map(Paragraph::text).collect()
}

type State = (Vec<kernpage_core::Story>, Vec<Vec<Paragraph>>);

/// A snapshot of everything a re-flow can touch: the stories and every frame's cache.
fn state(ed: &Editor, frames: &[(usize, ShapeId)]) -> State {
    (
        ed.doc().stories.clone(),
        frames.iter().map(|&(p, id)| paragraphs(ed.doc(), p, id)).collect(),
    )
}

const LONG: &str = "A whole new opening paragraph that pushes the old text down the story. It goes on for a while so that it takes several lines of the first frame.";

fn insert_first_paragraph(ed: &mut Editor, f: (usize, ShapeId)) {
    let mut new = paragraphs(ed.doc(), f.0, f.1);
    new.insert(0, Paragraph::plain(LONG));
    ed.execute(Command::SetText { page: f.0, id: f.1, paragraphs: new }).unwrap();
}

#[kernpage_samples::test]
fn typing_at_the_start_of_frame_1_pushes_its_last_lines_into_frame_2() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let old_story = before.0[0].paragraphs.clone();
    let old_last = before.1[0].last().unwrap().text();
    insert_first_paragraph(&mut ed, f[0]);
    let st = &ed.doc().stories[0];
    // The story is the old story with one paragraph in front, nothing else changed or lost.
    assert_eq!(st.paragraphs.len(), old_story.len() + 1);
    assert_eq!(st.paragraphs[0].text(), LONG);
    assert_eq!(texts(&st.paragraphs[1..]), texts(&old_story));
    // Frame 1 gave up its last line(s): its end moved back (in story paragraphs, +1 for
    // the new one, it now ends before where the old end was).
    assert!(st.ends[0] < (before.0[0].ends[0].0 + 1, 0), "{:?} {:?}", st.ends, before.0[0].ends);
    let f2 = paragraphs(ed.doc(), f[1].0, f[1].1);
    assert_ne!(f2, before.1[1]);
    let tail = &old_last[old_last.len() - 15..];
    assert!(flat(&f2).contains(tail), "frame 2 starts with what ended frame 1: {:?}", texts(&f2));
    // Frame 1 still starts with the typed text; every cache is its share of the story.
    assert_eq!(paragraphs(ed.doc(), f[0].0, f[0].1)[0].text(), LONG);
    assert!(ed.doc().story_caches_consistent());
}

#[kernpage_samples::test]
fn typing_and_then_deleting_it_again_restores_the_original_layout() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    insert_first_paragraph(&mut ed, f[0]);
    assert_ne!(state(&ed, &f), before);
    // Delete the typed paragraph again: what frame 1 shows now, without its first paragraph.
    let mut now = paragraphs(ed.doc(), f[0].0, f[0].1);
    now.remove(0);
    ed.execute(Command::SetText { page: f[0].0, id: f[0].1, paragraphs: now }).unwrap();
    // The model's own flow agrees with the real one: the story, its ends and all caches are
    // exactly what opening the file made.
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn deleting_from_frame_1_pulls_text_back_from_frame_2() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let mut new = before.1[0].clone();
    assert!(new.len() >= 4);
    new.drain(2..4);
    ed.execute(Command::SetText { page: f[0].0, id: f[0].1, paragraphs: new }).unwrap();
    let st = &ed.doc().stories[0];
    let old = &before.0[0].paragraphs;
    assert_eq!(st.paragraphs.len(), old.len() - 2);
    assert_eq!(texts(&st.paragraphs[..2]), texts(&old[..2]));
    assert_eq!(texts(&st.paragraphs[2..]), texts(&old[4..]));
    let f1 = paragraphs(ed.doc(), f[0].0, f[0].1);
    assert_eq!(f1.len(), before.1[0].len(), "frame 1 is full again");
    assert_eq!(f1[2].text(), old[4].text(), "pulled from frame 2");
    // Frame 2 starts with what followed in the story.
    assert_eq!(paragraphs(ed.doc(), f[1].0, f[1].1)[0].text(), old[6].text());
    assert!(ed.doc().story_caches_consistent());
    // Undo puts everything back, redo does it again, each in one step.
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
    assert!(!ed.can_undo());
    assert!(ed.redo());
    assert_ne!(state(&ed, &f), before);
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn undo_and_redo_restore_the_exact_story_caches_and_ends() {
    let (_, _, mut ed, f) = opened();
    let s0 = state(&ed, &f);
    insert_first_paragraph(&mut ed, f[0]);
    let s1 = state(&ed, &f);
    assert_ne!(s0, s1);
    // A second, different edit (frame 3), one more step.
    let mut p3 = paragraphs(ed.doc(), f[2].0, f[2].1);
    p3[0].runs[0].text.push_str(" and then some more words to lengthen it");
    ed.execute(Command::SetText { page: f[2].0, id: f[2].1, paragraphs: p3 }).unwrap();
    let s2 = state(&ed, &f);
    assert_ne!(s1, s2);
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), s1);
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), s0);
    assert!(ed.redo() && ed.redo());
    assert_eq!(state(&ed, &f), s2);
    assert!(!ed.can_redo());
}

#[kernpage_samples::test]
fn formatting_a_run_reflows_and_stays_in_the_story() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let len = kernpage_core::text::text_len(&before.1[0]);
    ed.execute(Command::FormatRuns {
        page: f[0].0,
        id: f[0].1,
        cell: None,
        range: 0..len,
        change: CharChange::Size(20.0),
    })
    .unwrap();
    let st = &ed.doc().stories[0];
    // Bigger type: fewer paragraphs fit, so the end of frame 1 moved back.
    assert!(st.ends[0] < before.0[0].ends[0], "{:?} {:?}", st.ends, before.0[0].ends);
    // The text is the same; the first paragraphs carry the new size.
    assert_eq!(texts(&st.paragraphs), texts(&before.0[0].paragraphs));
    assert!(st.paragraphs[0].runs.iter().all(|r| r.size_pt == 20.0));
    assert!(ed.doc().story_caches_consistent());
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn centering_the_part_of_a_paragraph_in_frame_2_centres_the_paragraph() {
    // Bigger type moves the end of frame 1 into the middle of a paragraph (find a size).
    let mut found = None;
    for size in [11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0] {
        let (_, _, mut ed, f) = opened();
        let len = kernpage_core::text::text_len(&paragraphs(ed.doc(), f[0].0, f[0].1));
        ed.execute(Command::FormatRuns {
            page: f[0].0,
            id: f[0].1,
            cell: None,
            range: 0..len,
            change: CharChange::Size(size),
        })
        .unwrap();
        if ed.doc().stories[0].ends[0].1 > 0 {
            found = Some((ed, f));
            break;
        }
    }
    let (mut ed, f) = found.expect("some size ends frame 1 in the middle of a paragraph");
    let st = ed.doc().stories[0].clone();
    let k = st.ends[0].0;
    assert_eq!(st.paragraphs[k].align, Align::Left);
    ed.execute(Command::FormatParagraphs {
        page: f[1].0,
        id: f[1].1,
        cell: None,
        range: 0..1,
        align: Align::Center,
    })
    .unwrap();
    let st = &ed.doc().stories[0];
    assert_eq!(st.paragraphs[k].align, Align::Center, "the whole paragraph is centred");
    let f1 = paragraphs(ed.doc(), f[0].0, f[0].1);
    assert_eq!(f1.last().unwrap().align, Align::Center, "frame 1 shows its part centred too");
    assert!(ed.doc().story_caches_consistent());
}

#[kernpage_samples::test]
fn resizing_a_frame_reflows_the_story() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let rect = ed.doc().shape(f[0].0, f[0].1).unwrap().rect;
    ed.execute(Command::SetRect {
        page: f[0].0,
        id: f[0].1,
        rect: Rect::new(rect.x, rect.y, rect.w, rect.h / 2),
    })
    .unwrap();
    let st = &ed.doc().stories[0];
    assert!(st.ends[0] < before.0[0].ends[0], "half the height holds less");
    assert!(paragraphs(ed.doc(), f[0].0, f[0].1).len() < before.1[0].len());
    assert_eq!(texts(&st.paragraphs), texts(&before.0[0].paragraphs), "no text lost");
    assert!(ed.doc().story_caches_consistent());
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
    // Growing the frame pulls text back from frame 2.
    ed.execute(Command::SetRect {
        page: f[0].0,
        id: f[0].1,
        rect: Rect::new(rect.x, rect.y, rect.w, rect.h * 2),
    })
    .unwrap();
    assert!(ed.doc().stories[0].ends[0] > before.0[0].ends[0]);
    // A pure move does not change the story.
    let moved = state(&ed, &f).0;
    ed.execute(Command::MoveShapes { page: f[0].0, ids: vec![f[0].1], dx: 10_000, dy: 0 }).unwrap();
    assert_eq!(ed.doc().stories, moved);
}

#[kernpage_samples::test]
fn a_wrapping_object_over_a_frame_reflows_it_and_moving_it_away_reflows_again() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let r = ed.doc().shape(f[0].0, f[0].1).unwrap().rect;
    // A square-wrap rectangle (added on top of the frame) over the left half of frame 1.
    let mut wrapper = Shape::new(Rect::new(r.x, r.y, r.w / 2, r.h), ShapeKind::Rectangle);
    wrapper.wrap = WrapType::Square;
    let id = ed.execute(Command::AddShape { page: f[0].0, shape: wrapper }).unwrap()[0];
    let covered = state(&ed, &f);
    assert!(covered.0[0].ends[0] < before.0[0].ends[0], "text wraps around it: less fits");
    assert_eq!(texts(&covered.0[0].paragraphs), texts(&before.0[0].paragraphs));
    assert!(ed.doc().story_caches_consistent());
    // Move it off the frame: the original layout is back.
    ed.execute(Command::MoveShapes { page: f[0].0, ids: vec![id], dx: 0, dy: 20 * 914_400 }).unwrap();
    assert_eq!(state(&ed, &f), before);
    // Undo of the move and of the add each restore story and caches exactly.
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), covered);
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn boxes_that_are_not_linked_are_untouched() {
    let (_, _, mut ed, f) = opened();
    let standalone = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.shapes.iter().map(move |s| (i, s)))
        .find(|(_, s)| s.is_text_box() && !f.iter().any(|x| x.1 == s.id))
        .map(|(i, s)| (i, s.id))
        .unwrap();
    let before = state(&ed, &f);
    let mut new = paragraphs(ed.doc(), standalone.0, standalone.1);
    new[0].runs[0].text.push_str(&" word".repeat(80));
    ed.execute(Command::SetText { page: standalone.0, id: standalone.1, paragraphs: new.clone() })
        .unwrap();
    assert_eq!(paragraphs(ed.doc(), standalone.0, standalone.1), new);
    assert_eq!(state(&ed, &f), before);
    assert!(ed.doc().story_of(standalone.1).is_none());
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn saving_after_a_reflow_writes_the_whole_story() {
    let (bytes, mut source, mut ed, f) = opened();
    let original = saved_story(&bytes);
    insert_first_paragraph(&mut ed, f[0]);
    let saved = saved_story(&save(&mut source, ed.doc()));
    assert_eq!(saved.len(), original.len() + 1);
    assert_eq!(saved[0], LONG);
    assert_eq!(saved[1..], original[..]);
    assert!(saved.last().unwrap().contains("Paragraph 30"));
    // The same edit as Phase 1 wrote it ("NEW " in front of frame 1): same saved story.
    let (_, mut source2, mut ed2, f2) = opened();
    prepend(&mut ed2, f2[0].0, f2[0].1, "NEW ");
    let s = saved_story(&save(&mut source2, ed2.doc()));
    assert_eq!(s.len(), original.len());
    assert_eq!(s[0], format!("NEW {}", original[0]));
    assert_eq!(s[1..], original[1..]);
}

#[kernpage_samples::test]
fn a_story_whose_frame_was_deleted_is_left_alone() {
    let (_, _, mut ed, f) = opened();
    ed.execute(Command::DeleteShapes { page: f[1].0, ids: vec![f[1].1] }).unwrap();
    let story = ed.doc().stories[0].clone();
    // Editing a remaining frame must not panic or drop the story (phase 4 handles deletion).
    prepend(&mut ed, f[0].0, f[0].1, "X ");
    assert_eq!(ed.doc().stories[0].frames, story.frames);
}

/// Writes check files for Publisher 2021:
/// `cargo test -p kernpage-core --test linked_story '--' '--ignored' write_linked_story_files`
#[kernpage_samples::test]
#[ignore = "writes files for Publisher to open"]
fn write_linked_story_files() {
    let dir = &std::env::temp_dir().join("pw-linked1");
    std::fs::create_dir_all(dir).unwrap();
    let bytes = sample();
    let src = pub_format::read_bytes(&bytes).unwrap();
    let (mut source, mut ed) = open(&bytes);
    let f = frame_ids(ed.doc());
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    std::fs::write(dir.join("linked-edit-frame1.pub"), save(&mut source, ed.doc())).unwrap();

    // Open -> snapshot v2 -> recover -> edit -> save (overflow from the story alone).
    let (_, ed) = open(&bytes);
    let recovered = decode(&encode(&meta(), ed.doc()).unwrap()).unwrap().document;
    let mut breaks = real_breaks(&src);
    *breaks[0].last_mut().unwrap() = (saved_story(&bytes).len(), 0);
    let mut source = SaveSource::new(bytes.clone(), &src, &recovered, &breaks);
    let mut ed = Editor::new(recovered);
    let f = frame_ids(ed.doc());
    prepend(&mut ed, f[1].0, f[1].1, "RECOVERED ");
    std::fs::write(dir.join("linked-v2-recovered-saved.pub"), save(&mut source, ed.doc())).unwrap();
}

/// Writes check files for Publisher 2021 (phase 2: a long insertion in frame 1 pushes text
/// across the frames; Publisher re-flows on open and should show the same split):
/// `cargo test -p kernpage-core --test linked_story '--' '--ignored' write_linked2_files`
#[kernpage_samples::test]
#[ignore = "writes files for Publisher to open"]
fn write_linked2_files() {
    let dir = &std::env::temp_dir().join("pw-linked2");
    std::fs::create_dir_all(dir).unwrap();
    let (_, mut source, mut ed, f) = opened();
    insert_first_paragraph(&mut ed, f[0]);
    let st = &ed.doc().stories[0];
    let shares: Vec<String> = (0..3).map(|i| format!("frame {}: {:?}..{:?}", i + 1, st.start_of(i), st.end_of(i))).collect();
    std::fs::write(dir.join("linked-long-insert-frame1.txt"), shares.join("\n")).unwrap();
    std::fs::write(dir.join("linked-long-insert-frame1.pub"), save(&mut source, ed.doc())).unwrap();
}

// ---- phase 3: live edits are one undo step; editing across a frame boundary ----------------------

#[kernpage_samples::test]
fn a_run_of_live_edits_is_one_undo_step() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let (page, id) = f[0];
    for tag in ["A", "AB", "ABC"] {
        let mut new = paragraphs(ed.doc(), page, id);
        let text = new[0].runs[0].text.trim_start_matches(['A', 'B', 'C']).to_owned();
        new[0].runs[0].text = format!("{tag}{text}");
        ed.execute_live(Command::SetText { page, id, paragraphs: new }).unwrap();
    }
    assert_ne!(state(&ed, &f), before);
    assert!(ed.undo(), "one step");
    assert_eq!(state(&ed, &f), before, "undo restores story and caches exactly");
    assert!(!ed.can_undo(), "and there was only one step");
    assert!(ed.redo());
    assert!(paragraphs(ed.doc(), page, id)[0].text().starts_with("ABC"));
}

#[kernpage_samples::test]
fn live_edits_that_cancel_out_leave_no_undo_step() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let (page, id) = f[0];
    let orig = paragraphs(ed.doc(), page, id);
    let mut typed = orig.clone();
    typed[0].runs[0].text.insert(0, 'X');
    ed.execute_live(Command::SetText { page, id, paragraphs: typed }).unwrap();
    assert!(ed.can_undo());
    ed.execute_live(Command::SetText { page, id, paragraphs: orig }).unwrap();
    assert!(!ed.can_undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn an_ordinary_command_or_end_live_starts_a_new_step() {
    let (_, _, mut ed, f) = opened();
    let (page, id) = f[0];
    let mut a = paragraphs(ed.doc(), page, id);
    a[0].runs[0].text.insert(0, 'X');
    ed.execute_live(Command::SetText { page, id, paragraphs: a.clone() }).unwrap();
    ed.end_live();
    a[0].runs[0].text.insert(0, 'Y');
    ed.execute_live(Command::SetText { page, id, paragraphs: a.clone() }).unwrap();
    assert!(ed.undo() && ed.undo());
    assert!(!ed.can_undo(), "two separate steps");
}

#[kernpage_samples::test]
fn set_story_text_replaces_the_story_and_reflows() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let st = &before.0[0];
    // Join paragraph 4 and 5 (the frame 1 / frame 2 boundary).
    let (paras, at) = kernpage_core::story::delete_before(&st.paragraphs, st.start_of(1)).unwrap();
    assert_eq!(paras.len(), st.paragraphs.len() - 1);
    ed.execute(Command::SetStoryText { story: st.id, paragraphs: paras.clone() }).unwrap();
    assert_eq!(ed.doc().stories[0].paragraphs, paras);
    assert_eq!(at.0, 3);
    assert!(ed.doc().story_caches_consistent());
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

// ---- phases 4 and 5: deleting frames, page operations, overflow and link queries ------------------------------

fn story_frames(ed: &Editor) -> Vec<ShapeId> {
    ed.doc().stories.first().map(|s| s.frames.clone()).unwrap_or_default()
}

fn page_of(doc: &Document, id: ShapeId) -> usize {
    doc.pages.iter().position(|p| p.shapes.iter().any(|s| s.id == id)).expect("frame exists")
}

/// Every frame's cache is exactly its share of the story, and the story text is unchanged.
fn assert_consistent_and_text_kept(ed: &Editor, whole: &[Paragraph]) {
    assert!(ed.doc().story_caches_consistent());
    let st = &ed.doc().stories[0];
    assert_eq!(texts(&st.paragraphs), texts(whole), "the story text never changes");
    for (i, &id) in st.frames.iter().enumerate() {
        let share = st.slice(i);
        let cache = paragraphs(ed.doc(), page_of(ed.doc(), id), id);
        if !share.is_empty() {
            assert_eq!(cache, share, "frame {i}");
        }
    }
}

#[kernpage_samples::test]
fn deleting_the_middle_frame_relinks_and_flows_the_text_into_the_rest() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let whole = before.0[0].paragraphs.clone();
    let first_text = texts(&before.1[0]);
    ed.execute(Command::DeleteShapes { page: f[1].0, ids: vec![f[1].1] }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[0].1, f[2].1]);
    assert_consistent_and_text_kept(&ed, &whole);
    // Frame 1 is unchanged; the last frame now starts where the deleted one started.
    assert_eq!(texts(&paragraphs(ed.doc(), f[0].0, f[0].1)), first_text);
    let last = texts(&paragraphs(ed.doc(), f[2].0, f[2].1));
    assert!(last[0].starts_with("Paragraph 5 "), "{:?}", last[0]);
    assert!(ed.doc().frame_overflows(f[2].0, f[2].1, &paragraphs(ed.doc(), f[2].0, f[2].1)));
    assert_eq!(ed.doc().frame_links(f[0].1), (false, true));
    assert_eq!(ed.doc().frame_links(f[2].1), (true, false));
    // One undo step restores everything exactly; redo deletes again.
    let after = (ed.doc().stories.clone(), paragraphs(ed.doc(), f[2].0, f[2].1));
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
    assert!(!ed.can_undo());
    assert!(ed.redo());
    assert_eq!((ed.doc().stories.clone(), paragraphs(ed.doc(), f[2].0, f[2].1)), after);
    assert_eq!(story_frames(&ed), vec![f[0].1, f[2].1]);
}

#[kernpage_samples::test]
fn deleting_the_first_frame_makes_the_next_one_first() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let whole = before.0[0].paragraphs.clone();
    ed.execute(Command::DeleteShapes { page: f[0].0, ids: vec![f[0].1] }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[1].1, f[2].1]);
    assert_consistent_and_text_kept(&ed, &whole);
    let t = texts(&paragraphs(ed.doc(), f[1].0, f[1].1));
    assert!(t[0].starts_with("Paragraph 1 "), "{:?}", t[0]);
    assert_eq!(ed.doc().frame_links(f[1].1), (false, true));
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn deleting_the_last_frame_makes_the_one_before_it_the_end_and_it_overflows() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let whole = before.0[0].paragraphs.clone();
    ed.execute(Command::DeleteShapes { page: f[2].0, ids: vec![f[2].1] }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[0].1, f[1].1]);
    assert_consistent_and_text_kept(&ed, &whole);
    let (p, id) = f[1];
    assert!(ed.doc().frame_overflows(p, id, &paragraphs(ed.doc(), p, id)));
    assert!(!ed.doc().frame_overflows(f[0].0, f[0].1, &paragraphs(ed.doc(), f[0].0, f[0].1)));
    assert!(ed.doc().stories[0].overflow().last().unwrap().text().contains("Paragraph 30"));
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn deleting_every_frame_removes_the_story_and_each_step_undoes_exactly() {
    let (_, _, mut ed, f) = opened();
    let s0 = state(&ed, &f);
    ed.execute(Command::DeleteShapes { page: f[0].0, ids: vec![f[0].1, f[1].1] }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[2].1], "one frame left: it is alone");
    assert_eq!(ed.doc().frame_links(f[2].1), (false, false));
    let t = texts(&paragraphs(ed.doc(), f[2].0, f[2].1));
    assert!(t[0].starts_with("Paragraph 1 "), "{:?}", t[0]);
    assert!(ed.doc().frame_overflows(f[2].0, f[2].1, &paragraphs(ed.doc(), f[2].0, f[2].1)));
    let s1 = (ed.doc().stories.clone(), paragraphs(ed.doc(), f[2].0, f[2].1));
    ed.execute(Command::DeleteShapes { page: f[2].0, ids: vec![f[2].1] }).unwrap();
    assert!(ed.doc().stories.is_empty(), "no frame left, no story");
    assert!(ed.undo());
    assert_eq!((ed.doc().stories.clone(), paragraphs(ed.doc(), f[2].0, f[2].1)), s1);
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), s0);
    assert!(!ed.can_undo());
}

#[kernpage_samples::test]
fn a_standalone_box_and_the_chain_report_links_and_overflow() {
    let (_, _, ed, f) = opened();
    let doc = ed.doc();
    assert_eq!(doc.frame_links(f[0].1), (false, true));
    assert_eq!(doc.frame_links(f[1].1), (true, true));
    assert_eq!(doc.frame_links(f[2].1), (true, false));
    let overflows = |p: usize, id: ShapeId| doc.frame_overflows(p, id, &paragraphs(doc, p, id));
    assert!(!overflows(f[0].0, f[0].1) && !overflows(f[1].0, f[1].1));
    assert!(overflows(f[2].0, f[2].1));
    let standalone = doc.pages[0]
        .shapes
        .iter()
        .find(|s| s.is_text_box() && doc.story_of(s.id).is_none())
        .unwrap()
        .id;
    assert_eq!(doc.frame_links(standalone), (false, false));
    assert!(!overflows(0, standalone));
    // The same box with lots of text does not fit.
    let long: Vec<Paragraph> = (0..40).map(|i| Paragraph::plain(&format!("Line {i} of text"))).collect();
    assert!(doc.frame_overflows(0, standalone, &long));
}

#[kernpage_samples::test]
fn duplicating_a_page_copies_linked_frames_empty_and_links_the_copies() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    let pages = ed.doc().pages.len();
    ed.execute(Command::DuplicatePage { page: 0 }).unwrap();
    assert_eq!(ed.doc().pages.len(), pages + 1);
    assert_eq!(ed.doc().stories.len(), 2, "the original chain and the copies' own");
    assert_eq!(ed.doc().stories[0], before.0[0], "the original chain is untouched");
    let copy = ed.doc().stories[1].clone();
    assert_ne!(copy.id, ed.doc().stories[0].id);
    assert_eq!(copy.frames.len(), 2);
    assert!(texts(&copy.paragraphs).iter().all(String::is_empty));
    let on_new_page: Vec<ShapeId> = ed.doc().pages[1].shapes.iter().map(|s| s.id).collect();
    assert!(copy.frames.iter().all(|id| on_new_page.contains(id)));
    for &id in &copy.frames {
        let t = paragraphs(ed.doc(), 1, id);
        assert_eq!(t.len(), 1);
        assert!(t[0].text().is_empty(), "copies of linked frames have no text");
    }
    assert_eq!(ed.doc().frame_links(copy.frames[0]), (false, true));
    assert_eq!(ed.doc().frame_links(copy.frames[1]), (true, false));
    // The standalone box keeps its text.
    let standalone = ed.doc().pages[1]
        .shapes
        .iter()
        .find(|s| s.is_text_box() && ed.doc().story_of(s.id).is_none())
        .unwrap();
    assert!(matches!(&standalone.kind, ShapeKind::TextBox { paragraphs } if paragraphs[0].text().starts_with("Standalone")));
    assert!(ed.doc().story_caches_consistent());
    assert!(ed.undo());
    assert_eq!(ed.doc().pages.len(), pages);
    assert_eq!(state(&ed, &f), before);
    assert!(ed.redo());
    assert_eq!(ed.doc().stories.len(), 2);
}

#[kernpage_samples::test]
fn deleting_or_moving_a_page_with_linked_frames() {
    let (_, _, mut ed, f) = opened();
    let before = state(&ed, &f);
    // Move: the chain is a property of the links, not of the page order.
    ed.execute(Command::MovePage { from: 0, to: 1 }).unwrap();
    assert_eq!(ed.doc().stories, before.0);
    assert!(ed.undo());
    // Delete page 2 (frame 3): like deleting the last frame.
    ed.execute(Command::DeletePage { page: 1 }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[0].1, f[1].1]);
    assert!(ed.doc().story_caches_consistent());
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
    // Delete page 1 (frames 1 and 2): the last frame is alone and shows the story's start.
    ed.execute(Command::DeletePage { page: 0 }).unwrap();
    assert_eq!(story_frames(&ed), vec![f[2].1]);
    let t = texts(&paragraphs(ed.doc(), 0, f[2].1));
    assert!(t[0].starts_with("Paragraph 1 "));
    assert!(ed.undo());
    assert_eq!(state(&ed, &f), before);
}

#[kernpage_samples::test]
fn saving_after_deleting_a_linked_frame_reports_it_and_keeps_the_story() {
    let (bytes, source, mut ed, f) = opened();
    let original = saved_story(&bytes);
    ed.execute(Command::DeleteShapes { page: f[1].0, ids: vec![f[1].1] }).unwrap();
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(
        plan.unsupported.iter().any(|u| u.contains("linked text box")),
        "{:?}",
        plan.unsupported
    );
    assert!(!plan.unsupported.iter().any(|u| u == "deleted objects"), "{:?}", plan.unsupported);
    assert!(plan.stories.is_empty(), "the story text did not change");
    assert_eq!(saved_story(&out), original, "no text lost, no text corrupted");
    // Editing the first frame as well: the edit is written, the whole story kept.
    prepend(&mut ed, f[0].0, f[0].1, "NEW ");
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.iter().any(|u| u.contains("linked text box")));
    let saved = saved_story(&out);
    assert_eq!(saved.len(), original.len());
    assert_eq!(saved[0], format!("NEW {}", original[0]));
    assert!(saved.last().unwrap().contains("Paragraph 30"));
    // Deleting every frame never writes an empty story.
    for (p, id) in [f[0], f[2]] {
        ed.execute(Command::DeleteShapes { page: p, ids: vec![id] }).unwrap();
    }
    assert!(ed.doc().stories.is_empty());
    let (out, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.iter().any(|u| u.contains("linked text box")));
    assert!(saved_story(&out).last().unwrap().contains("Paragraph 30"));
}

#[kernpage_samples::test]
fn saving_after_deleting_a_page_with_linked_frames_reports_it() {
    let (_, source, mut ed, f) = opened();
    ed.execute(Command::DeletePage { page: f[2].0 }).unwrap();
    let (_, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.unsupported.iter().any(|u| u.contains("linked text box")), "{:?}", plan.unsupported);
    assert!(plan.stories.is_empty());
}
