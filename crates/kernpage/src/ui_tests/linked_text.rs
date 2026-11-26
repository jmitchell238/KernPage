//! Typing in linked text frames (docs/design/LINKED-STORIES.md, phase 3): the story flows
//! live while typing, the caret follows the text into the next frame, and the keys at the
//! edge of a frame (Backspace, Delete, arrows) work across the boundary. All through real
//! key events on `samples/gen/linked.pub` (frame 1, frame 2 on page 1, frame 3 on page 2).

use super::support::*;
use eframe::egui::{Key, Modifiers};
use kernpage_core::{Paragraph, ShapeId};

const NONE: Modifiers = Modifiers::NONE;

/// Words typed one character at a time (one key press per frame, like a keyboard).
const WORDS: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor ";

/// The frames of the sample's story, in order (frame 3 is on page 2).
fn linked_frames(ui: &TestUi) -> Vec<ShapeId> {
    ui.doc().stories[0].frames.clone()
}

fn click_away(ui: &mut TestUi) {
    ui.click_at(ui.at_in(6.0, 9.5));
    assert!(ui.app().edit.is_none(), "clicking away ends the edit");
}

fn editing(ui: &mut TestUi) -> ShapeId {
    ui.app().edit.as_ref().expect("edit mode").id
}

fn story_texts(ui: &TestUi) -> Vec<String> {
    ui.doc().stories[0].paragraphs.iter().map(Paragraph::text).collect()
}

fn undo_depth(ui: &mut TestUi) -> usize {
    let ed = ui.app().editor.as_mut().expect("editor");
    let mut n = 0;
    while ed.undo() {
        n += 1;
    }
    for _ in 0..n {
        ed.redo();
    }
    n
}

/// The paragraphs of a frame on any page.
fn paragraphs_of(ui: &TestUi, id: ShapeId) -> Vec<Paragraph> {
    ui.doc()
        .pages
        .iter()
        .flat_map(|p| p.shapes.iter())
        .find_map(|s| match &s.kind {
            kernpage_core::ShapeKind::TextBox { paragraphs } if s.id == id => Some(paragraphs.clone()),
            _ => None,
        })
        .expect("frame")
}

/// Everything a re-flow touches: stories and the text of the three frames.
fn snapshot(ui: &TestUi, frames: &[ShapeId]) -> (Vec<kernpage_core::Story>, Vec<Vec<Paragraph>>) {
    (
        ui.doc().stories.clone(),
        frames.iter().map(|&f| paragraphs_of(ui, f)).collect(),
    )
}

/// Frame `n` (0-based) being edited, caret at character `caret`.
fn edit_frame(ui: &mut TestUi, frames: &[ShapeId], n: usize, caret: usize) {
    ui.double_click_at(ui.center_of(frames[n]));
    assert_eq!(editing(ui), frames[n]);
    ui.set_selection_range(caret, caret);
}

fn end_of_text(ui: &mut TestUi) -> usize {
    ui.edit_text().chars().count()
}

/// Type `WORDS` into the open editor one character at a time until the edit moved to another
/// frame; returns the characters typed.
fn type_until_handover(ui: &mut TestUi) -> String {
    let from = editing(ui);
    let mut typed = String::new();
    for ch in WORDS.chars().cycle().take(600) {
        ui.type_text(&ch.to_string());
        typed.push(ch);
        if editing(ui) != from {
            return typed;
        }
    }
    panic!("the caret never left the frame; typed {typed:?}");
}

/// A linked document with frame 1's caret at the end of its text.
fn at_end_of_frame_1() -> (TestUi, Vec<ShapeId>) {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    edit_frame(&mut ui, &frames, 0, 0);
    let end = end_of_text(&mut ui);
    ui.set_selection_range(end, end);
    (ui, frames)
}

// ---- live flow and the hand-over -----------------------------------------------------------------

#[test]
fn typing_at_the_end_of_frame_1_shows_in_frame_2_before_commit_and_hands_the_caret_over() {
    let (mut ui, frames) = at_end_of_frame_1();
    let before = snapshot(&ui, &frames);
    let (first, second) = (frames[0], frames[1]);
    let rev = ui.revision();
    // The first characters fit in the frame: live, so the document already has them.
    ui.type_text("x");
    assert!(ui.revision() > rev, "a keystroke reaches the document at once in a linked frame");
    assert!(ui.text_of(first).ends_with('x'));
    assert_eq!(editing(&mut ui), first);
    let typed = type_until_handover(&mut ui);
    // Still editing (nothing was committed), now in frame 2, and frame 2 already shows the text.
    assert_eq!(editing(&mut ui), second);
    let head = ui.text_of(second).lines().next().unwrap().to_owned();
    assert!(!head.is_empty() && typed.contains(head.trim_end()), "{head:?} was typed ({typed:?})");
    assert_ne!(ui.text_of(second), before.1[1].iter().map(Paragraph::text).collect::<Vec<_>>().join("\n"));
    // The caret is at the end of the text that flowed on (the typed words), not at the start.
    let first_line = ui.edit_text().lines().next().unwrap().chars().count();
    assert_eq!(ui.caret(), (first_line, first_line));
    // Typing continues at the caret, in frame 2, at the right place in the story.
    let at = ui.app().edit.as_ref().unwrap().text.clone();
    ui.type_text("ZZ");
    assert_eq!(editing(&mut ui), second);
    let t = story_texts(&ui);
    let p4 = t.iter().find(|s| s.starts_with("Paragraph 4 ")).unwrap();
    assert!(p4.ends_with("ZZ"), "ZZ landed at the end of paragraph 4: {p4:?}");
    assert!(p4.contains(typed.trim_end()), "{p4:?}");
    assert_eq!(t.len(), before.0[0].paragraphs.len(), "no paragraph appeared or went");
    assert_ne!(ui.edit_text(), at);
    // One undo step for the whole session, and one Ctrl+Z restores story and frames exactly.
    click_away(&mut ui);
    assert_eq!(undo_depth(&mut ui), 1);
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui, &frames), before);
    assert!(!ui.can_undo());
    ui.key(CTRL, Key::Y);
    assert!(story_texts(&ui).iter().any(|s| s.ends_with("ZZ")));
}

#[test]
fn typing_in_a_linked_frame_makes_no_undo_entry_until_it_is_one_session() {
    let (mut ui, _) = at_end_of_frame_1();
    for ch in "abc".chars() {
        ui.type_text(&ch.to_string());
    }
    assert_eq!(undo_depth(&mut ui), 1, "three live keystrokes are one step");
    click_away(&mut ui);
    assert_eq!(undo_depth(&mut ui), 1, "and committing adds nothing");
}

#[test]
fn typing_and_deleting_it_again_leaves_no_undo_step() {
    let (mut ui, frames) = at_end_of_frame_1();
    let before = snapshot(&ui, &frames);
    ui.type_text("a");
    ui.type_text("b");
    ui.key(NONE, Key::Backspace);
    ui.key(NONE, Key::Backspace);
    assert_eq!(snapshot(&ui, &frames), before);
    assert!(!ui.can_undo(), "nothing changed, nothing to undo");
}

#[test]
fn typing_in_a_box_that_is_not_linked_still_waits_for_the_commit() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let standalone = ui
        .shapes()
        .iter()
        .find(|s| s.is_text_box() && ui.text_of(s.id).starts_with("Standalone"))
        .unwrap()
        .id;
    let rev = ui.revision();
    ui.double_click_at(ui.center_of(standalone));
    ui.set_selection_range(0, 0);
    ui.type_text("Q");
    ui.type_text("R");
    assert_eq!(ui.revision(), rev, "nothing reaches the document until the edit ends");
    click_away(&mut ui);
    assert!(ui.text_of(standalone).starts_with("QR"));
    assert_eq!(undo_depth(&mut ui), 1);
}

// ---- Backspace, Delete and arrows at the edge of a frame ----------------------------------------------

#[test]
fn backspace_at_the_start_of_frame_2_joins_the_paragraph_break_and_continues_in_frame_1() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    let before = snapshot(&ui, &frames);
    let first_len = ui.text_of(frames[0]).chars().count();
    let n = story_texts(&ui).len();
    edit_frame(&mut ui, &frames, 1, 0);
    ui.key(NONE, Key::Backspace);
    // Frame 2 starts a paragraph, so what precedes it is the paragraph break: paragraph 4
    // and 5 are one paragraph now, and editing goes on at the join, in frame 1.
    let t = story_texts(&ui);
    assert_eq!(t.len(), n - 1);
    assert!(t[3].starts_with("Paragraph 4 ") && t[3].contains("Paragraph 5 "), "{:?}", t[3]);
    assert_eq!(editing(&mut ui), frames[0]);
    assert_eq!(ui.caret(), (first_len, first_len), "the caret is where the two paragraphs met");
    ui.type_text("!");
    assert!(story_texts(&ui)[3].contains("!Paragraph 5 "));
    click_away(&mut ui);
    assert_eq!(undo_depth(&mut ui), 1);
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui, &frames), before);
}

#[test]
fn backspace_at_the_start_of_a_frame_that_starts_mid_paragraph_deletes_the_last_glyph_before_it() {
    let (mut ui, frames) = at_end_of_frame_1();
    type_until_handover(&mut ui);
    let second = frames[1];
    assert_eq!(editing(&mut ui), second);
    let story = story_texts(&ui);
    let st = ui.doc().stories[0].clone();
    assert!(st.ends[0].1 > 0, "frame 1 now ends in the middle of a paragraph: {:?}", st.ends);
    let before_glyphs: usize = story.iter().map(|p| p.chars().count()).sum();
    ui.set_selection_range(0, 0);
    ui.key(NONE, Key::Backspace);
    let after = story_texts(&ui);
    let after_glyphs: usize = after.iter().map(|p| p.chars().count()).sum();
    assert_eq!(after_glyphs, before_glyphs - 1, "exactly one character went");
    assert_eq!(after.len(), story.len());
    assert_eq!(editing(&mut ui), frames[0], "editing goes on in frame 1");
    let end = end_of_text(&mut ui);
    let (c, _) = ui.caret();
    assert!(c <= end && c > 0);
}

#[test]
fn arrows_at_the_edge_of_a_frame_move_into_the_next_and_previous_frame() {
    let (mut ui, frames) = at_end_of_frame_1();
    let rev = ui.revision();
    ui.key(NONE, Key::ArrowRight);
    assert_eq!(editing(&mut ui), frames[1]);
    assert_eq!(ui.caret(), (0, 0), "at the start of the next frame");
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(editing(&mut ui), frames[0]);
    let end = end_of_text(&mut ui);
    assert_eq!(ui.caret(), (end, end), "at the end of the previous frame");
    ui.key(NONE, Key::ArrowDown);
    assert_eq!(editing(&mut ui), frames[1]);
    assert_eq!(ui.caret(), (0, 0));
    ui.key(NONE, Key::ArrowUp);
    assert_eq!(editing(&mut ui), frames[0]);
    // Moving the caret changes nothing.
    assert_eq!(ui.revision(), rev);
    assert!(!ui.can_undo());
    // Inside a frame the arrows still just move the caret.
    ui.set_selection_range(3, 3);
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(editing(&mut ui), frames[0]);
    assert_eq!(ui.caret().0, 2);
    // The first frame has no previous frame, the last no next one.
    ui.set_selection_range(0, 0);
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(editing(&mut ui), frames[0]);
}

#[test]
fn right_arrow_from_the_end_of_frame_2_reaches_frame_3_on_the_next_page() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    edit_frame(&mut ui, &frames, 1, 0);
    let end = end_of_text(&mut ui);
    ui.set_selection_range(end, end);
    ui.key(NONE, Key::ArrowRight);
    assert_eq!(editing(&mut ui), frames[2]);
    assert_eq!(ui.app().page, 1, "the editor followed the caret to page 2");
    assert_eq!(ui.caret(), (0, 0));
    ui.key(NONE, Key::ArrowLeft);
    assert_eq!(editing(&mut ui), frames[1]);
    assert_eq!(ui.app().page, 0);
}

#[test]
fn shift_arrows_stop_at_the_edge_of_the_frame() {
    let (mut ui, frames) = at_end_of_frame_1();
    ui.key(Modifiers::SHIFT, Key::ArrowRight);
    assert_eq!(editing(&mut ui), frames[0], "a selection never leaves the frame");
    let end = end_of_text(&mut ui);
    assert_eq!(ui.caret(), (end, end));
}

#[test]
fn delete_at_the_end_of_a_frame_pulls_the_next_character_in() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    let n = story_texts(&ui).len();
    edit_frame(&mut ui, &frames, 0, 0);
    let end = end_of_text(&mut ui);
    ui.set_selection_range(end, end);
    // Frame 1 ends at a paragraph end: Delete takes the paragraph break away, so paragraph 5
    // joins paragraph 4 and its first words are pulled into frame 1 where they fit.
    ui.key(NONE, Key::Delete);
    let t = story_texts(&ui);
    assert_eq!(t.len(), n - 1);
    assert!(t[3].contains("Paragraph 5 "));
    assert_eq!(editing(&mut ui), frames[0]);
    assert_eq!(ui.caret(), (end, end), "the caret stays where it was");
    // Delete again: now the first glyph of what follows the caret goes (the space).
    let glyphs: usize = story_texts(&ui).iter().map(|p| p.chars().count()).sum();
    let end2 = end_of_text(&mut ui);
    ui.set_selection_range(end2, end2);
    if ui.caret().0 == end2 && ui.doc().stories[0].end_of(0).1 > 0 {
        ui.key(NONE, Key::Delete);
        let after: usize = story_texts(&ui).iter().map(|p| p.chars().count()).sum();
        assert_eq!(after, glyphs - 1);
    }
    click_away(&mut ui);
    assert_eq!(undo_depth(&mut ui), 1);
}

// ---- save and formatting ------------------------------------------------------------------------------------

#[test]
fn saving_after_typing_across_a_hand_over_keeps_the_whole_story() {
    let mut ui = TestUi::start();
    let copy = ui.env.files.join("work.pub");
    std::fs::copy(sample("gen/linked.pub"), &copy).unwrap();
    ui.app().open_path(&copy);
    ui.settle();
    let frames = linked_frames(&ui);
    edit_frame(&mut ui, &frames, 0, 0);
    let end = end_of_text(&mut ui);
    ui.set_selection_range(end, end);
    type_until_handover(&mut ui);
    ui.type_text("END");
    let expected = story_texts(&ui);
    assert!(expected.last().unwrap().contains("Paragraph 30"), "the hidden tail is in the story");
    click_away(&mut ui);
    ui.key(CTRL, Key::S);
    assert!(ui.app().error.is_none(), "{:?}", ui.app().error);
    assert!(!ui.app().is_dirty());
    // Read the saved file back: the full story, typed text and overflow included.
    let mut again = TestUi::start();
    again.app().open_path(&copy);
    assert!(again.app().error.is_none());
    again.settle();
    assert_eq!(story_texts(&again), expected);
}

#[test]
fn bold_applied_to_a_selection_in_a_linked_frame_applies_and_reflows() {
    let mut ui = TestUi::with_sample("gen/linked.pub");
    let frames = linked_frames(&ui);
    edit_frame(&mut ui, &frames, 0, 0);
    ui.set_selection_range(0, 40);
    ui.click_ribbon("Bold (Ctrl+B)");
    let bold: String = ui
        .paragraphs(frames[0])
        .iter()
        .flat_map(|p| p.runs.iter())
        .filter(|r| r.bold)
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(bold.chars().count(), 40, "{bold:?}");
    let st = &ui.doc().stories[0];
    assert!(st.paragraphs.iter().flat_map(|p| p.runs.iter()).any(|r| r.bold), "the story has it too");
    assert!(ui.doc().story_caches_consistent());
    // The editor keeps working on the frame's current text.
    assert_eq!(ui.edit_text(), ui.text_of(frames[0]));
    click_away(&mut ui);
}

// ---- cost -----------------------------------------------------------------------------------------------------------

#[test]
fn a_keystroke_in_a_linked_frame_is_cheap() {
    let (mut ui, _) = at_end_of_frame_1();
    crate::app::live_timing::take();
    let t0 = std::time::Instant::now();
    let words: String = WORDS.chars().cycle().take(50).collect();
    for ch in words.chars() {
        ui.type_text(&ch.to_string());
    }
    let whole = t0.elapsed();
    let (n, live) = crate::app::live_timing::take();
    println!(
        "LIVE-FLOW 50 keystrokes: {n} live syncs, {:.2} ms per sync, {:.2} ms per key including the UI frames",
        live.as_secs_f64() * 1000.0 / n.max(1) as f64,
        whole.as_secs_f64() * 1000.0 / 50.0
    );
    assert_eq!(n, 50);
    assert!(live.as_secs_f64() / (n as f64) < 0.05, "a live sync takes under 50 ms");
}
