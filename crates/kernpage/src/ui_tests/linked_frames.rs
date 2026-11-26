//! Linked frames as Publisher 2021 shows and handles them (docs/ui/LINKED-TEXT.md; design in
//! docs/design/LINKED-STORIES.md, phases 4 and 5): the overflow box and link arrows beside a
//! selected frame, the Text Box tab's Linking group, Next / Previous, and what deleting,
//! copying and pasting frames and duplicating, deleting and moving pages do to a chain.
//! All through real pointer and key events on `samples/gen/linked.pub` (frames 1 and 2 and a
//! standalone box on page 1, frame 3 on page 2; only frame 3 overflows).

use super::support::*;
use crate::app::Prompt;
use crate::linkmarks::{Mark, OVERFLOW_RED};
use eframe::egui::{Color32, Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use kernpage_core::{Command, Document, Paragraph, ShapeId, ShapeKind};

const NONE: Modifiers = Modifiers::NONE;

struct Sample {
    ui: TestUi,
    /// The chain's frames in order.
    f: Vec<ShapeId>,
    standalone: ShapeId,
}

fn open() -> Sample {
    let ui = TestUi::with_sample("gen/linked.pub");
    finish(ui)
}

fn finish(ui: TestUi) -> Sample {
    let f = ui.doc().stories[0].frames.clone();
    let standalone = ui.doc().pages[0]
        .shapes
        .iter()
        .find(|s| s.is_text_box() && ui.doc().story_of(s.id).is_none())
        .expect("standalone box")
        .id;
    Sample { ui, f, standalone }
}

fn page_of(ui: &TestUi, id: ShapeId) -> usize {
    ui.doc().pages.iter().position(|p| p.shapes.iter().any(|s| s.id == id)).unwrap()
}

/// Show the page a shape is on.
fn show(ui: &mut TestUi, id: ShapeId) {
    let p = page_of(ui, id);
    ui.app().go_page(p);
    ui.settle();
}

/// Click a shape (selecting it as a shape, not editing).
fn select(ui: &mut TestUi, id: ShapeId) {
    show(ui, id);
    ui.click_at(ui.center_of(id));
    assert_eq!(ui.selected(), vec![id]);
    assert!(ui.app().edit.is_none());
}

fn edit(ui: &mut TestUi, id: ShapeId) {
    show(ui, id);
    ui.double_click_at(ui.center_of(id));
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(id));
}

fn marks(ui: &mut TestUi) -> Vec<Mark> {
    ui.app().probe.marks.iter().map(|m| m.0).collect()
}

fn handle_fill(ui: &mut TestUi) -> Option<Color32> {
    ui.app().probe.handle_fill
}

fn mark_rect(ui: &mut TestUi, m: Mark) -> eframe::egui::Rect {
    ui.app().probe.marks.iter().find(|x| x.0 == m).expect("mark").1
}

fn snapshot(ui: &TestUi) -> Document {
    ui.doc().clone()
}

fn texts(ps: &[Paragraph]) -> Vec<String> {
    ps.iter().map(Paragraph::text).collect()
}

fn cache(ui: &TestUi, id: ShapeId) -> Vec<String> {
    let s = ui.doc().pages.iter().flat_map(|p| &p.shapes).find(|s| s.id == id).unwrap();
    match &s.kind {
        ShapeKind::TextBox { paragraphs } => texts(paragraphs),
        _ => panic!("not text"),
    }
}

fn long_text() -> Vec<Paragraph> {
    (0..60).map(|i| Paragraph::plain(&format!("Line {i} of the standalone text"))).collect()
}

/// A real window turns Ctrl+C / Ctrl+V into these events.
fn copy(ui: &mut TestUi) {
    ui.event(Event::Copy);
    ui.settle();
}

fn paste(ui: &mut TestUi) {
    ui.event(Event::Paste(String::new()));
    ui.settle();
}

fn run(ui: &mut TestUi, cmd: Command) {
    ui.app().editor.as_mut().unwrap().execute(cmd).unwrap();
    ui.settle();
}

// ---- overflow box and red handles -------------------------------------------------------------------

#[test]
fn the_overflow_box_shows_only_while_the_overflowing_last_frame_is_selected() {
    let Sample { mut ui, f, .. } = open();
    show(&mut ui, f[2]);
    assert!(marks(&mut ui).is_empty(), "nothing selected: nothing drawn");
    assert_eq!(handle_fill(&mut ui), None);
    select(&mut ui, f[2]);
    // The last frame has a previous link (left arrow) and overflows (the box).
    let m = marks(&mut ui);
    assert!(m.contains(&Mark::Overflow) && m.contains(&Mark::Prev) && !m.contains(&Mark::Next), "{m:?}");
    // Just outside the right edge, in the lower part; a 14 px white box.
    let frame = ui.painted(f[2]);
    let o = mark_rect(&mut ui, Mark::Overflow);
    assert_eq!(o.left(), frame.right());
    assert!(o.center().y > frame.center().y && o.center().y < frame.bottom());
    assert_eq!((o.width(), o.height()), (14.0, 14.0));
    // The handles of an overflowing frame are red.
    assert_eq!(handle_fill(&mut ui), Some(OVERFLOW_RED));
    // Deselected again: gone.
    ui.click_at(ui.at_in(6.0, 9.5));
    assert!(marks(&mut ui).is_empty());
    assert_eq!(handle_fill(&mut ui), None);
}

#[test]
fn no_overflow_box_on_a_frame_that_is_not_last_and_none_when_the_text_fits() {
    let Sample { mut ui, f, .. } = open();
    select(&mut ui, f[0]);
    assert_eq!(marks(&mut ui), vec![Mark::Next], "frame 1: next arrow only, no overflow");
    assert_eq!(handle_fill(&mut ui), Some(Color32::WHITE));
    select(&mut ui, f[1]);
    let mut m = marks(&mut ui);
    m.sort_by_key(|x| format!("{x:?}"));
    assert_eq!(m, vec![Mark::Next, Mark::Prev]);
    assert_eq!(handle_fill(&mut ui), Some(Color32::WHITE));
    // Cut the story to what fits: the last frame stops overflowing.
    let st = ui.doc().stories[0].clone();
    let short: Vec<Paragraph> = st.paragraphs[..10].to_vec();
    run(&mut ui, Command::SetStoryText { story: st.id, paragraphs: short });
    select(&mut ui, f[2]);
    assert_eq!(marks(&mut ui), vec![Mark::Prev], "the text fits: no box, only the left arrow");
    assert_eq!(handle_fill(&mut ui), Some(Color32::WHITE));
    // Ctrl+Z brings the overflow back.
    ui.key(CTRL, Key::Z);
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    assert_eq!(handle_fill(&mut ui), Some(OVERFLOW_RED));
}

#[test]
fn a_standalone_box_whose_text_does_not_fit_shows_the_box_when_selected() {
    let Sample { mut ui, standalone, .. } = open();
    select(&mut ui, standalone);
    assert!(marks(&mut ui).is_empty(), "the standalone box fits and has no links");
    assert_eq!(handle_fill(&mut ui), Some(Color32::WHITE));
    let id = standalone;
    run(&mut ui, Command::SetText { page: 0, id, paragraphs: long_text() });
    assert_eq!(marks(&mut ui), vec![Mark::Overflow]);
    assert_eq!(handle_fill(&mut ui), Some(OVERFLOW_RED));
    // Unselected: nothing.
    ui.click_at(ui.at_in(6.0, 9.5));
    assert!(marks(&mut ui).is_empty());
    ui.h.run_steps(40); // a double click needs its own time
    // Caret in the box (text edit): the box is there too, and follows what is typed.
    ui.double_click_at(ui.center_of(id));
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    ui.set_selection_range(0, 0);
    assert!(marks(&mut ui).contains(&Mark::Overflow));
}

#[test]
fn typing_into_a_standalone_box_turns_the_overflow_box_on_and_deleting_turns_it_off() {
    let Sample { mut ui, standalone, .. } = open();
    edit(&mut ui, standalone);
    assert!(marks(&mut ui).is_empty());
    ui.set_selection_range(0, 0);
    let lots = "more text\n".repeat(60);
    ui.type_text(&lots);
    assert_eq!(marks(&mut ui), vec![Mark::Overflow], "typed text that does not fit");
    ui.key(CTRL, Key::A);
    ui.key(NONE, Key::Delete);
    assert!(marks(&mut ui).is_empty(), "the text fits again");
}

#[test]
fn the_overflow_box_shows_in_text_edit_mode_and_clicking_it_does_nothing() {
    let Sample { mut ui, f, .. } = open();
    edit(&mut ui, f[2]);
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    let rev = ui.revision();
    let before = ui.app().edit.as_ref().map(|e| (e.id, e.text.clone()));
    let at = mark_rect(&mut ui, Mark::Overflow).center();
    ui.click_at(at);
    assert_eq!(ui.revision(), rev);
    assert_eq!(ui.app().edit.as_ref().map(|e| (e.id, e.text.clone())), before, "the caret stays");
    // Shape selected: same.
    ui.app().commit_edit();
    ui.h.run_steps(40);
    select(&mut ui, f[2]);
    let at = mark_rect(&mut ui, Mark::Overflow).center();
    ui.click_at(at);
    assert_eq!(ui.selected(), vec![f[2]], "still selected");
    assert!(ui.app().edit.is_none());
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    assert_eq!(ui.revision(), rev);
}

#[test]
fn no_boxes_are_drawn_with_several_frames_selected() {
    let Sample { mut ui, f, .. } = open();
    select(&mut ui, f[0]);
    ui.click_mods(ui.center_of(f[1]), SHIFT);
    assert_eq!(ui.selected().len(), 2);
    assert!(marks(&mut ui).is_empty());
}

// ---- link arrows ----------------------------------------------------------------------------------------

#[test]
fn the_arrows_sit_on_the_edges_and_clicking_them_goes_next_and_previous() {
    let Sample { mut ui, f, .. } = open();
    select(&mut ui, f[1]);
    let frame = ui.painted(f[1]);
    let (next, prev) = (mark_rect(&mut ui, Mark::Next), mark_rect(&mut ui, Mark::Prev));
    assert_eq!(next.left(), frame.right());
    assert_eq!(prev.right(), frame.left());
    assert!(next.center().y > frame.center().y, "the right arrow is low");
    assert!(prev.center().y < frame.center().y, "the left arrow is high");
    // Right arrow: into frame 3 on page 2, caret at the start of its text.
    ui.click_at(next.center());
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[2]));
    assert_eq!(ui.app().page, 1);
    assert_eq!(ui.caret(), (0, 0));
    // Its left arrow: back to frame 2 on page 1, caret at the end of its text.
    let prev = mark_rect(&mut ui, Mark::Prev);
    ui.click_at(prev.center());
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[1]));
    assert_eq!(ui.app().page, 0);
    let end = ui.edit_text().chars().count();
    assert_eq!(ui.caret(), (end, end));
    // And again to frame 1.
    let prev = mark_rect(&mut ui, Mark::Prev);
    ui.click_at(prev.center());
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[0]));
    assert!(!marks(&mut ui).contains(&Mark::Prev), "the first frame has no left arrow");
}

// ---- Text Box tab, Linking group --------------------------------------------------------------------------

/// (Create Link, Break, Next, Previous) enabled.
fn linking(ui: &mut TestUi) -> [bool; 4] {
    if ui.tab() != crate::app::Tab::TextBox {
        ui.click_tab("Text Box");
    }
    ["Create Link", "Break", "Next", "Previous"].map(|n| ui.body(n).enabled)
}

#[test]
fn the_linking_group_follows_the_publisher_table() {
    let Sample { mut ui, f, standalone } = open();
    select(&mut ui, standalone);
    assert_eq!(linking(&mut ui), [true, false, false, false], "standalone");
    select(&mut ui, f[0]);
    assert_eq!(linking(&mut ui), [false, true, true, false], "first");
    select(&mut ui, f[1]);
    assert_eq!(linking(&mut ui), [false, true, true, true], "middle");
    select(&mut ui, f[2]);
    assert_eq!(linking(&mut ui), [true, false, false, true], "last");
    // The same with the caret in the frame.
    edit(&mut ui, standalone);
    assert_eq!(linking(&mut ui), [true, false, false, false], "caret, standalone");
    edit(&mut ui, f[0]);
    assert_eq!(linking(&mut ui), [false, true, true, false], "caret, first");
    edit(&mut ui, f[1]);
    assert_eq!(linking(&mut ui), [false, true, true, true], "caret, middle");
    edit(&mut ui, f[2]);
    assert_eq!(linking(&mut ui), [true, false, false, true], "caret, last");
    // Several shapes: all four off, whatever the mix.
    ui.app().commit_edit();
    for (a, b) in [(f[0], f[1]), (f[0], standalone), (standalone, f[1])] {
        select(&mut ui, a);
        ui.click_mods(ui.center_of(b), SHIFT);
        assert_eq!(ui.selected().len(), 2);
        assert_eq!(linking(&mut ui), [false; 4], "two selected");
    }
}

#[test]
fn next_and_previous_enter_the_neighbour_frame_changing_page() {
    let Sample { mut ui, f, .. } = open();
    select(&mut ui, f[0]);
    ui.click_tab("Text Box");
    // Next from frame 1 (shape selected): caret at the start of frame 2, page 1 stays.
    ui.click_body("Next");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[1]));
    assert_eq!((ui.app().page, ui.caret()), (0, (0, 0)));
    // Again: frame 3, on page 2.
    ui.click_body("Next");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[2]));
    assert_eq!((ui.app().page, ui.caret()), (1, (0, 0)));
    assert!(!ui.body("Next").enabled, "the end of the chain");
    let rev = ui.revision();
    ui.click_body("Next");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[2]), "no wrap-around");
    // Previous: frame 2 at the end of its text, page 1 shown again.
    ui.click_body("Previous");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[1]));
    let end = ui.edit_text().chars().count();
    assert_eq!((ui.app().page, ui.caret()), (0, (end, end)));
    ui.click_body("Previous");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[0]));
    let end = ui.edit_text().chars().count();
    assert_eq!(ui.caret(), (end, end));
    assert!(!ui.body("Previous").enabled);
    assert_eq!(ui.revision(), rev, "moving around changes nothing");
}

#[test]
fn create_link_and_break_show_their_state_but_do_nothing_yet() {
    let Sample { mut ui, f, standalone } = open();
    select(&mut ui, standalone);
    ui.click_tab("Text Box");
    let before = snapshot(&ui);
    let rev = ui.revision();
    let b = ui.body("Create Link");
    assert!(b.enabled);
    assert_eq!(b.tip, "Coming soon");
    ui.click_body("Create Link");
    assert_eq!(snapshot(&ui), before);
    select(&mut ui, f[1]);
    assert!(ui.body("Break").enabled);
    ui.click_body("Break");
    assert_eq!(snapshot(&ui), before);
    assert_eq!(ui.revision(), rev);
    assert!(ui.app().edit.is_none() && ui.app().prompt.is_none());
}

// ---- deleting frames ----------------------------------------------------------------------------------------

fn delete_frame(ui: &mut TestUi, id: ShapeId) {
    select(ui, id);
    ui.key(NONE, Key::Delete);
}

#[test]
fn deleting_a_middle_frame_relinks_the_chain_and_one_undo_restores_it() {
    let Sample { mut ui, f, .. } = open();
    let before = snapshot(&ui);
    let whole = texts(&before.stories[0].paragraphs);
    delete_frame(&mut ui, f[1]);
    let st = ui.doc().stories[0].clone();
    assert_eq!(st.frames, vec![f[0], f[2]]);
    assert_eq!(texts(&st.paragraphs), whole, "the text is never touched");
    assert!(cache(&ui, f[2])[0].starts_with("Paragraph 5 "), "frame 3 now starts where frame 2 did");
    // The marks follow: frame 1 still has a next arrow; frame 3 a previous one and overflows.
    select(&mut ui, f[0]);
    assert_eq!(marks(&mut ui), vec![Mark::Next]);
    select(&mut ui, f[2]);
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
    assert!(!ui.can_undo());
}

#[test]
fn deleting_the_first_or_the_last_frame() {
    let Sample { mut ui, f, .. } = open();
    let before = snapshot(&ui);
    let whole = texts(&before.stories[0].paragraphs);
    delete_frame(&mut ui, f[0]);
    assert_eq!(ui.doc().stories[0].frames, vec![f[1], f[2]]);
    assert!(cache(&ui, f[1])[0].starts_with("Paragraph 1 "), "frame 2 is the first now");
    select(&mut ui, f[1]);
    assert_eq!(marks(&mut ui), vec![Mark::Next], "no left arrow on the new first frame");
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
    // The last frame: the one before it becomes the end and overflows.
    delete_frame(&mut ui, f[2]);
    assert_eq!(ui.doc().stories[0].frames, vec![f[0], f[1]]);
    assert_eq!(texts(&ui.doc().stories[0].paragraphs), whole);
    select(&mut ui, f[1]);
    assert!(marks(&mut ui).contains(&Mark::Overflow));
    assert_eq!(handle_fill(&mut ui), Some(OVERFLOW_RED));
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
}

#[test]
fn deleting_every_frame_removes_the_story() {
    let Sample { mut ui, f, .. } = open();
    let before = snapshot(&ui);
    for &id in &f {
        delete_frame(&mut ui, id);
    }
    assert!(ui.doc().stories.is_empty());
    for _ in 0..3 {
        ui.key(CTRL, Key::Z);
    }
    assert_eq!(snapshot(&ui), before);
}

#[test]
fn saving_after_deleting_a_linked_frame_asks_and_loses_nothing() {
    let mut ui = TestUi::start();
    let copy = ui.env.files.join("work.pub");
    std::fs::copy(sample("gen/linked.pub"), &copy).unwrap();
    ui.app().open_path(&copy);
    ui.settle();
    let Sample { mut ui, f, .. } = finish(ui);
    let whole = texts(&ui.doc().stories[0].paragraphs);
    let original = std::fs::read(&copy).unwrap();
    delete_frame(&mut ui, f[1]);
    ui.key(CTRL, Key::S);
    match &ui.app().prompt {
        Some(Prompt::Unsupported { items, .. }) => {
            assert!(items.iter().any(|i| i.contains("linked text box")), "{items:?}");
            assert!(!items.iter().any(|i| i == "deleted objects"), "{items:?}");
        }
        _ => panic!("the unsupported-changes prompt is open"),
    }
    assert_eq!(std::fs::read(&copy).unwrap(), original, "nothing written before the answer");
    ui.h.get_by_label("  Save supported changes  ").click();
    ui.settle();
    // The file still holds the whole chain and the whole story.
    let mut again = TestUi::start();
    again.app().open_path(&copy);
    again.settle();
    assert_eq!(again.doc().stories.len(), 1);
    assert_eq!(texts(&again.doc().stories[0].paragraphs), whole, "no text lost");
}

// ---- copy, paste, duplicate ----------------------------------------------------------------------------------

#[test]
fn pasting_a_linked_frame_makes_a_new_empty_unlinked_box_nine_points_off() {
    let Sample { mut ui, f, standalone } = open();
    let before = snapshot(&ui);
    select(&mut ui, f[1]);
    let at = ui.rect_of(f[1]);
    copy(&mut ui);
    paste(&mut ui);
    let new = ui.selected();
    assert_eq!(new.len(), 1);
    let id = new[0];
    assert!(!f.contains(&id));
    let s = ui.shape(id);
    assert!(s.is_text_box());
    assert!(matches!(&s.kind, ShapeKind::TextBox { paragraphs } if paragraphs.len() == 1 && paragraphs[0].text().is_empty()));
    let nine = kernpage_core::geometry::pt_to_emu(9.0);
    assert_eq!((s.rect.x, s.rect.y), (at.x + nine, at.y + nine));
    assert_eq!((s.rect.w, s.rect.h), (at.w, at.h), "same size");
    assert!(ui.doc().story_of(id).is_none(), "its own, unlinked");
    assert_eq!(ui.doc().stories, before.stories, "the chain is untouched");
    assert_eq!(ui.doc().frame_links(id), (false, false));
    // One undo step removes it.
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
    // A standalone box is copied with its text, as before.
    select(&mut ui, standalone);
    copy(&mut ui);
    paste(&mut ui);
    let copied = ui.selected()[0];
    assert!(ui.text_of(copied).starts_with("Standalone"));
    // The first and the last frame: empty copies too.
    for &id in &[f[0], f[2]] {
        select(&mut ui, id);
        copy(&mut ui);
        paste(&mut ui);
        let c = ui.selected()[0];
        assert_eq!(ui.text_of(c), "");
        assert!(ui.doc().story_of(c).is_none());
        assert_eq!(ui.doc().stories, before.stories);
    }
}

#[test]
fn ctrl_d_does_nothing() {
    let Sample { mut ui, f, .. } = open();
    select(&mut ui, f[1]);
    let before = snapshot(&ui);
    let rev = ui.revision();
    ui.key(CTRL, Key::D);
    assert_eq!(ui.revision(), rev);
    assert_eq!(snapshot(&ui), before);
    assert!(ui.app().prompt.is_none() && ui.app().error.is_none());
}

// ---- pages ------------------------------------------------------------------------------------------------------

#[test]
fn duplicating_a_page_with_linked_frames_makes_empty_frames_in_a_new_chain() {
    let Sample { mut ui, f, standalone } = open();
    let before = snapshot(&ui);
    ui.app().duplicate_page_at(0);
    ui.settle();
    assert_eq!(ui.page_count(), 3);
    assert_eq!(ui.doc().stories.len(), 2);
    assert_eq!(ui.doc().stories[0], before.stories[0], "the original chain is untouched");
    let copy = ui.doc().stories[1].clone();
    assert_eq!(copy.frames.len(), 2);
    for &id in &copy.frames {
        assert_eq!(page_of(&ui, id), 1);
        assert_eq!(cache(&ui, id), vec![String::new()]);
    }
    // The copies are linked to each other: the first has a next arrow, the second a previous one.
    ui.app().go_page(1);
    ui.settle();
    select(&mut ui, copy.frames[0]);
    assert_eq!(marks(&mut ui), vec![Mark::Next]);
    select(&mut ui, copy.frames[1]);
    assert_eq!(marks(&mut ui), vec![Mark::Prev]);
    // The standalone box was copied with its text.
    let dup = ui.doc().pages[1]
        .shapes
        .iter()
        .find(|s| s.is_text_box() && ui.doc().story_of(s.id).is_none())
        .unwrap()
        .id;
    assert!(ui.text_of(dup).starts_with("Standalone"));
    assert_ne!(dup, standalone);
    assert_eq!(ui.doc().stories[0].frames, f);
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
}

#[test]
fn deleting_a_page_removes_its_frames_from_the_chain() {
    let Sample { mut ui, f, .. } = open();
    let before = snapshot(&ui);
    ui.app().delete_page_at(1);
    ui.settle();
    assert_eq!(ui.doc().stories[0].frames, vec![f[0], f[1]]);
    ui.app().go_page(0);
    ui.settle();
    select(&mut ui, f[1]);
    assert!(marks(&mut ui).contains(&Mark::Overflow), "frame 2 is the end now");
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
    // Page 1: frames 1 and 2 go, frame 3 is alone and shows the start of the story.
    ui.app().delete_page_at(0);
    ui.settle();
    assert_eq!(ui.doc().stories[0].frames, vec![f[2]]);
    assert!(cache(&ui, f[2])[0].starts_with("Paragraph 1 "));
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
}

#[test]
fn moving_a_page_does_not_change_the_chain() {
    let Sample { mut ui, f, .. } = open();
    let before = snapshot(&ui);
    ui.app().move_page_by(0, 1);
    ui.settle();
    assert_eq!(ui.doc().stories, before.stories);
    assert_eq!(ui.doc().stories[0].frames, f);
    assert_eq!(page_of(&ui, f[2]), 0, "the last frame is on the first page now");
    // The chain runs backwards across the pages: Previous from it goes to page 2.
    select(&mut ui, f[2]);
    ui.click_tab("Text Box");
    ui.click_body("Previous");
    assert_eq!(ui.app().edit.as_ref().map(|e| e.id), Some(f[1]));
    assert_eq!(ui.app().page, 1);
    ui.app().commit_edit();
    ui.key(CTRL, Key::Z);
    assert_eq!(snapshot(&ui), before);
}
