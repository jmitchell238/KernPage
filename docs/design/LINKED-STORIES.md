# Linked text boxes as one live story (design, pw-vo6i)

Status: phases 0 to 5 implemented (see section 7, 7a, 7b and 7c). Written 2026-11-15 after reading the code at commit 127ad55.

## 1. The problem

Publisher treats a chain of linked text boxes as ONE story that flows live through the frames: typing in any
frame pushes overflow into the next one, deleting pulls text back, and the last frame shows an overflow
indicator when text does not fit. KernPage cannot do this because the model has no story:
`core::convert::from_pub_flowed` (convert.rs:226) cuts the story once, at open, into per-frame slices and
makes every frame an ordinary independent `ShapeKind::TextBox { paragraphs }`.

The kittest test `text_typed_into_a_linked_frame_flows_on_into_the_next_frame`
(crates/kernpage/src/ui_tests/text_edit.rs:664) is ignored for this reason.

## 2. Current state (what each layer knows)

| Layer | Linked story today | Reference |
|---|---|---|
| pub-format | Real. Item 39 = story number (all frames of a chain share it), item 40 = chain position (absent = 0). The file does NOT store where each frame's text ends; Publisher re-flows on open. `Document::chains()` gives frames in order as `ShapeRef`s. `MCLD` is only a cross-check. | spec 4G (PUB-FORMAT-SPEC.md:734); pub-format/src/lib.rs:165 (`chain_story`, `chain_pos`), :260 (grouping) |
| pub2pdf | `flow_chains` lays the story of the chain out frame by frame (insets, columns, wrap obstacles, line spacing, fonts through `Resources`) and records where each frame ends as a `StoryPos` (paragraph index, glyph offset). `chain_breaks` exposes only that. Text left after the last frame is dropped. It builds `Resources::new(...)` (font loading) on every call. | pub2pdf/src/lib.rs:481 (`flow_chains`), :535 (`chain_breaks`), :446 (`layout_columns`) |
| core convert | Slices the story by those breaks into independent text boxes (`story_slice`, convert.rs:95). Frames after the first of a chain with no break info are empty. The text after the last break (overflow) is not kept anywhere. | convert.rs:226-285; doc comment at :224 admits the limitation |
| core model | `ShapeKind::TextBox { paragraphs }`. No story, no link, no chain. `Document` has pages and an id counter only. | model.rs:121, :361 |
| core command / undo | `SetText { page, id, paragraphs }` replaces one shape's paragraphs. Undo is patch based: `Patch::Replace { from: Shape, to: Shape }` per shape (command.rs:338), so one edit can already touch several shapes but cannot touch anything that is not a shape. | command.rs:891, :338, :441 |
| editor | `EditState { page, id, text: String, fresh, cell }` (app.rs:199). `begin_edit` loads the frame's paragraphs as plain text (app.rs:1224). The canvas puts an egui `TextEdit::multiline` over the frame rect (canvas.rs:860-917); the caret is a char offset in that string. `edit_command` rebuilds paragraphs from the string with `text::rebuild_paragraphs` and issues `SetText` (app.rs:1271). Formatting uses `format_target`/`format_info` (app.rs:1350+) on the same one-box assumption. | app.rs, canvas.rs |
| render | `render.rs` draws each text box from its own `paragraphs`; no overflow indicator, no cross-frame knowledge. | render.rs:103 `draw_shape` |
| export (PDF) | `export.rs` builds a pub_format::Document from the model with `chain_story: None` for every shape (export.rs:104), so every frame is an independent box. It is consistent only because the slices were cut with the same layout. | export.rs:95-106 |
| save | `SaveSource::new` re-derives the chains from the source file (`Unit` = story index + frames + `mid_para` flags, save.rs:60, :577-607). Text changes are detected per story: `compose()` (save.rs:692) re-joins the frames in chain order and the changed story is written once with `set_story_text_keep_formatting` (save.rs:1131-1155, :1630). |

## 3. Does saving an edit to a linked frame work today?

A quick test program run against `samples/gen/linked.pub` showed the following.

* Unedited document: `plan` is empty (no spurious change).
* Edit in frame 2 (existing test `linked_chain_edit_in_second_frame`, save.rs tests:155): works, story written once, text of the first frame still in front. Correct.
* Edit in frame 1 (prepend "NEW "): the story is written ONCE (good), `unsupported` is empty, reopening shows the right split
  (frame 1 starts "NEW Paragraph 1", frame 2 "Paragraph 5", frame 3 "Paragraph 9").
* **DATA LOSS, confirmed.** The sample story has 30 paragraphs; the last frame overflows (manifest: `overflowing: true`)
  and `chain_breaks` ends at paragraph 16. The model holds only paragraphs 1-16 (convert drops the overflow tail).
  After the same frame-1 edit the saved story text is 1,498 characters, 17 paragraphs, against 2,796 characters /
  31 paragraphs in the original: **"Paragraph 17 ... Paragraph 30" are deleted from the user's file**, silently
  (`plan.unsupported` is empty). Any text edit in any frame of a chain whose last frame overflows loses the hidden tail.
  (A frame boundary in the middle of a paragraph is handled by `mid_para`; that part is fine.)

So: text content is correct when nothing overflows, but saving is lossy whenever a chain overflows, and it never
re-flows (the saved file is right only because Publisher re-flows on open).

## 4. Other places that assume one box = one story

* Copy / paste (app.rs:1660-1700): `clipboard: Vec<Shape>`; a pasted linked frame is a plain independent text box holding its slice.
* Delete a frame (`DeleteShapes`): the slice text disappears with the frame; the neighbours are not re-flowed.
* Moving a frame to another page (arrange / pages pane): harmless today (shapes are independent); with a story the chain order must be kept by frame order, not page order (the sample already has a chain across pages 1 and 2).
* `DuplicatePage` (command.rs:1108): clones become independent boxes. Save already refuses pages with linked boxes
  (`page_blocks_duplicate`, save.rs:998; FEATURES PG-DUPLICATE note).
* AutoRecover: snapshot is `{magic, version: 1, meta, document}` of the serialized model (autorecover.rs:25, :197). A story field needs a version bump.
* PDF export: see table above; frames are independent and therefore agree with the screen only while nothing is edited.
* Undo: one edit = one `Edit` of shape patches; a story change that moves text across frames touches several shapes, which the patch model supports, but story text itself is not a shape.
* Text formatting commands (`FormatRuns`, `FormatParagraphs`, `SpacingParagraphs`), `format_info`, Select All in edit mode: all act on one box's paragraphs.
* `SaveSource` (save.rs:586) picks the story from the SOURCE file, so it already handles one-story-many-frames on the way out; nothing in it knows about frames the user added or removed.

## 5. What Publisher 2021 does (as recorded in the repo)

The repo records little visual detail; nothing here was driven live.

* Text Box contextual tab, **Linking** group (docs/research/publisher-2021-ui.md:536): Create Link (large), Break, Next, Previous (small). KernPage shows the group with all four greyed out (ctx_ribbon.rs:518).
* Object model (publisher-2021-object-model.md:250): `NextLinkedTextFrame`, `PreviousLinkedTextFrame`, `HasNextLink`, `HasPreviousLink`, `BreakForwardLink`, `ValidLinkTarget`, `Overflowing`, `IncludeContinuedOnPage`, `IncludeContinuedFromPage` ("Continued on page" / "Continued from page" notes, a Format Object > Text Box option, publisher-2021-dialogs.md:119), `Story`, `HasText`.
* FEATURES.md rows: TB-LINK (create link; note "Linked boxes display and flow; creating and breaking links not yet" ) = pw-g0cd; TB-UNLINK = pw-v23b; TB-OVERFLOW (detect overflow) pw-pr0h, partly done; TB-AUTOFIT pw-azu3; SHP-OVERFLOW-LIST pw-60cx; TB-STORY (whole-story operations) marked done; RIB-TEXTBO-LINKING pw-7a0u.
* spec 4G: the file gives the last frame `overflowing = true`; leftover text is "not shown".
* Not recorded anywhere (must be observed before building): the exact look and position of the overflow indicator, what Next/Previous do to the selection (expected: select the next/previous frame, going to its page), the cursor shape and flow of Create Link, what a pasted or deleted frame does to the chain, the exact "Continued on page" wording and where it sits.

## 6. Proposed design (smallest sound version)

Principle: the **story is the truth, each frame's `paragraphs` is a derived cache**. That keeps render.rs, canvas
hit-testing, export.rs, text formatting reads and every existing test untouched, and puts all new logic in one place.

### 6.1 Model

```text
Document.stories: Vec<Story>            // #[serde(default)]
Story { id: StoryId, frames: Vec<ShapeId>, paragraphs: Vec<Paragraph> /* the whole story, incl. overflow */,
        ends: Vec<StoryPos> /* where each frame's text ends, from the last layout */ }
```

* A text box in no story is unchanged (no Story entry; single boxes stay as they are, no per-keystroke layout cost).
* A shape belongs to at most one story (`Document::story_of(id)`).
* The cached `TextBox.paragraphs` of a frame = `story_slice(story.paragraphs, start, end)` (convert already has this, convert.rs:95: move it to core `story` module).
* Overflow = text after `ends.last()`; the last frame is "overflowing" iff that remainder is not empty.

### 6.2 Layout step (the "flow")

* New public API in pub2pdf (the only pub2pdf change): `flow_story(frames: &[FrameGeom], paragraphs: &[pub_format::Paragraph], opts) -> Vec<StoryPos>`, a thin wrapper over `layout_columns` that takes frame geometry (rect, columns, gap, wrap obstacles) instead of a whole `Document`. Same code as PDF export, so editor, PDF and Publisher agree on breaks. A cached `Resources`/font handle object is needed so fonts are not reloaded on every call (today `chain_breaks` creates `Resources::new` each time).
* core `story::flow(doc, story_id)`: build the geometry from the model, call `flow_story`, store `ends`, return the new slices. Wrap obstacles come from the other shapes of the page (same as `Document::wrap_obstacles`); easiest is the model-to-pf conversion `export.rs` already has, restricted to the pages of the chain.
* Triggered inside `Editor::execute` after any command that can change a story or a frame's geometry (SetText on a story frame, FormatRuns/Paragraphs/Spacing, frame move/resize, columns, wrap changes of ANY shape on those pages, paste/delete of a frame). The re-flow becomes part of the same `Edit` (patches: `Replace` for each frame whose cache changed, plus a new `Patch::SetStory { from, to }`), so undo/redo is one step and restores text and cache together.

### 6.3 Editing and the caret across frames

Keep one egui `TextEdit` overlay at a time, over the frame being edited (reuses all of canvas.rs:860-917). `EditState` gains `story_range: Option<(StoryPos, StoryPos)>` (the slice shown).

* Typing: the buffer is the frame's slice. `edit_command` becomes a `StoryEdit { story, from, to, new_paragraphs }` that splices the rebuilt slice into the story (`rebuild_paragraphs` on the slice, same formatting inheritance as today), then re-flows. This is what un-ignores the test.
* After the re-flow, if the caret's story offset is now past this frame's end (typing overflowed), move the edit overlay to the next frame at the matching offset; if the caret is before this frame's start (deleting pulled text), move to the previous frame. The `EditState.id` change is the only UI work; no multi-frame text widget is needed.
* Arrow keys / Page keys at a frame boundary, Ctrl+A, and mouse selection across frames: MVP limitation (selection stays inside one frame; Ctrl+A selects that frame's slice). Phase 6 adds Up/Down/Left/Right crossing frames; whole-story select / Find-Replace stay out.
* Click in another frame of the same story while editing: commit and begin edit in that frame (already how `begin_edit` works).

### 6.4 Overflow indicator

`Story::overflowing(frames.last())` drives a small marker at the bottom-right of the last frame (draw in render.rs, not part of the print/PDF output). Exact look: observe Publisher first (section 5). Also covers single boxes later (TB-OVERFLOW) if single boxes are given a one-frame layout check.

### 6.5 Undo granularity

One user edit = one undo step as today: an `Edit` made of the `SetStory` patch plus the frame `Replace` patches. Typing bursts that are merged today by `sync_edit`/`commit_edit` are unchanged.

### 6.6 Save

* `SaveSource::compose` is replaced by the story: unit text = `doc.stories[i].paragraphs` joined (including the overflow tail, which fixes the data loss). The mid-paragraph join flags and `compose` disappear for stories that have a `Story`.
* One story in the file, written once: already the case (`StoryChange` per `story_index`, save.rs:1150). The frames are NOT written as separate stories.
* Formatting diff: `changed_flags`/`format_spans` already take before/after paragraph lists; feed them the story paragraphs.
* Links created or broken in the UI (later phase) change item 39/40 of the frames: pub-write has no writer for that yet. Until it does, a changed chain is reported through `plan.unsupported` (no silent loss).

### 6.7 AutoRecover

Add `Document.stories` (serde default) and bump `FORMAT_VERSION` 1 to 2. Accept version 1 snapshots (the field is simply empty: frames then load as independent boxes, exactly as v1 saved them) rather than discarding them; a version above 2 is rejected as today. Add the round-trip test in autorecover.rs for a doc with a story.

### 6.8 Other migrations (small)

* convert.rs: `from_pub_flowed` builds `Story` entries (frames = chain ids, paragraphs = full story incl. overflow, ends = breaks). `from_pub(name, doc)` (no breaks) makes the story but runs the flow itself, or keeps today's fallback.
* export.rs: unchanged (uses the cached slices). Optional later: emit `chain_story`/`chain_pos` so PDF export reflows with its own layout.
* Copy/paste: pasting frames makes independent boxes with their current text (cache copied; the `Story` is not). Cutting or copying a whole story is later.
* Delete a frame: remove the id from `Story.frames`, re-flow the rest (text moves into the remaining frames, nothing is lost; if one frame is left it stays a one-frame story until saved). If it was the last frame, the story is removed.
* Duplicate page / move to other page: frames of a story duplicated onto a new page become independent boxes (same as today); moving between pages keeps `Story.frames` order.
* Text-format commands on a story frame operate on the story slice offsets then re-flow; `FormatRuns` etc. need the slice-to-story offset translation (`story_range.start`).

## 7. Phase plan (each phase independently shippable and testable)

| # | Phase | Size | Result |
|---|---|---|---|
| 0 | **Stop the data loss in save.** In `SaveSource::new` keep the overflow tail of each chain (source story after the last break) in `Unit`, and make `compose` append it (joined to the last paragraph when the last break is mid-paragraph). No model change. | S (1 file, ~60 lines) | saving a linked chain no longer deletes the hidden text |
| 1 | **DONE (2026-11-15).** `Document.stories` + `Story` + `StoryId`/`StoryPos` (model.rs), serde default; `convert::from_pub_flowed` fills it (frames, full paragraphs incl. overflow, ends); core `story` module (`slice_range`/`splice_range`, `Story::slice/splice/compose/overflow`, `Document::story_text`). Save `compose` reads the story (frame caches spliced in at save time, last frame first); Phase 0's `Unit.tail` stays as the fallback for documents without a story. AutoRecover `FORMAT_VERSION` 2 (v1 still loads). Frames' cache unchanged, no UI change. Deviations: slicing works on the model `Paragraph` type (convert keeps its pf slicer for the caches); the frame caches stay authoritative for edits until phase 2 (`compose` splices them into the story), so `Story.ends` is not changed by edits yet. | M | model has the truth |
| 2 | **DONE (2026-11-15).** pub2pdf: `FrameGeom` (`FrameGeom::of_shape(doc, page, shape)`: origin, size, columns, gap, wrap obstacles as the PDF path computes them), `FlowContext` (keeps the font/metrics `Resources` between calls; `flow(&[FrameGeom], &[pf::Paragraph]) -> Vec<StoryPos>`) and one-shot `flow_story(frames, story, opts)`. `flow_chains` now builds `FrameGeom`s and calls the same `layout_columns`, so PDF export, `chain_breaks` and `flow_story` share one code path. Core: `Document::reflow_story` (story.rs) + `Patch::SetStory` + `Editor::reflow_patches` in `Editor::execute` (section 6.2 below as built). Save reads `story_text`, which returns `story.paragraphs` when every cache equals its slice and falls back to compose's splice otherwise. App: `sync_edit` refreshes the edit buffer from the frame after a re-flow. Tests: core `tests/linked_story.rs` (phase 2 block), pub2pdf `tests/pdf.rs`, kittest in `ui_tests/text_edit.rs` (the ignored test is un-ignored and tightened). | L (the risky one) | commands re-flow at core level; typing in a linked frame re-flows on commit |
| 3 | **DONE (2026-11-15).** Live flow while typing and caret hand-over between frames (see 7b). | M | live flow while typing |
| 4 | **DONE (2026-11-15).** Overflow indicator, delete / paste / duplicate / page behaviour as recorded in docs/ui/LINKED-TEXT.md (see 7c); AutoRecover v2 was already done in phase 1. | M | feature complete for existing chains |
| 5 | **DONE (2026-11-15).** Linking group (Create Link / Break / Next / Previous), link arrows, Next / Previous (see 7c). | S | |
| 6 | Caret and selection across frames (arrow keys, Ctrl+A over the story). | M | |
| 7 (later) | Create Link / Break: `Story` edits (merge two stories / split), pub-write writer for items 39/40, "Continued on page" notes. | L, needs Publisher observation first | |

Recommendation: include phases 0-4 in the MVP of this bug, 5 as a cheap extra; 6 and 7 are not MVP. Creating and breaking links in the UI should come after a pub-write link writer exists (phase 7), otherwise a user-made chain cannot be saved.

## 7a. Phase 2 as built (2026-11-15), deviations

* **Which commands re-flow.** After the command's patches are applied, `Editor::affected_stories` looks at them: a `Replace` of a story frame whose kind (text), rect, columns, column gap, rotation, wrap or wrap distance changed (so SetText, FormatRuns, FormatParagraphs, SpacingParagraphs, SetTextFormat, SetRect, MoveShapes, AlignShapes, StyleShapes Rotate/Columns/Wrap ...); a `Replace` of any other shape that wraps (`wrap != None`) on a page holding a frame of the story; `Insert`/`Remove` of a wrapping shape on such a page; `Reorder` on such a page when it has a wrapping shape; `SetPages` (page setup) for every story. Everything else (fill/outline, other pages, non-wrapping shapes, page insert/move) does not. Stories are only looked at when `Document.stories` is not empty, so documents without chains pay nothing.
* **What a re-flow does.** `Document::reflow_story`: (a) `story_text` = the story with the frames' current caches spliced in (an edit lands in the edited frame's cache first, then goes into the story, which is the truth from then on); (b) `flow_geometry` (export.rs) builds the frame geometry and wrap obstacles from a layout-only `pf::Document` of just the pages that hold frames (no picture bytes, no text) and `FlowContext::flow` gives the new ends (one process-wide context, `story::FLOW`, so fonts load once); (c) every frame cache becomes `Story::cache(i)` (its slice, or one empty paragraph). Caches whose text is unchanged produce no patch.
* **Undo.** One edit = the command's own patches + one `Patch::Replace` per frame whose cache the re-flow changed + one `Patch::SetStory { from, to }` (whole story before/after, replaced by story id). `undo`/`redo` just apply the inverse/forward patches, so story, caches and ends come back exactly. Cost: each flowing edit keeps two copies of the story's paragraphs in the history (fine for newsletter-size stories; a diff patch would be the optimisation).
* **Model paragraph to layout paragraph is 1:1** (`export::flow_paragraphs`, line breaks `
` stay in the text and are not glyphs). PDF export of the model (`to_pub`) instead splits a paragraph at every `
`, which changes paragraph numbers; the flow must not, because story positions count model paragraphs. Consequence: a model paragraph with `
` is flowed as a single line stream (matching `chain_breaks` on a `.pub`), while PDF export of the frame caches draws the break.
* **Mid-paragraph frame starts.** `Story::compose` now also copies a changed alignment/spacing of a frame's first paragraph onto the whole paragraph when the frame starts in the middle of it (otherwise `splice_range` keeps the earlier part's attributes and the change would be lost at the next re-flow).
* **Frame deletion is not handled** (phase 4): a story with a frame that no longer exists is skipped by the re-flow and by the debug assertion (`debug_assert!(doc.story_caches_consistent())` at the end of `execute`, `undo`, `redo`), exactly as in phase 1 (save's `compose` treats the missing frame as contributing nothing). Paste and duplicate are unchanged (independent boxes).
* **Editor.** `sync_edit` (commit, click away, format commands) issues `SetText` as before; the command now re-flows. If the edited frame is in a story, `show_flowed_text` then reloads the edit buffer from the frame (text that moved on is no longer in it), so a second sync cannot put the overflow back. The caret is not handed to the next frame (phase 3).
* **Measured** (debug build, linked.pub, 3 frames, 30 paragraphs): `chain_breaks` / one-shot `flow_story` after the process has loaded fonts once: about 2.0 ms (builds `Resources`, reloads the font faces); `FlowContext::flow` with a warm context: about 0.8 ms; the very first call in a process also scans the font folders (about 145 ms, once, already cached by `pub2pdf::font_db`).

## 7b. Phase 3 as built (2026-11-15)

* **Live flow.** In a frame that belongs to a story, every change of the edit buffer (`TextEdit` response `changed()`) pushes `Request::LiveSync`; `App::live_sync` stores the text at once (`sync_edit`, which for linked frames uses `Editor::execute_live`) so the story re-flows and the next frames update on screen. No debounce: one re-flow per key press. Boxes that are not in a story (and table cells) are unchanged: nothing reaches the document until the edit ends.
* **Undo.** `Editor::execute_live` runs the command, then folds the new step into the previous live step (patches are compacted to one `Replace` per shape and one `SetStory` per story, earliest `from`, latest `to`); a session that ends where it started leaves no step at all. The session ends on `commit_edit` (click away, Escape, page change, any non-text command), `Editor::end_live`, an ordinary `execute`, undo or redo. So typing, a hand-over, Backspace/Delete joins and all, are ONE undo step, and one Ctrl+Z restores story, ends and caches exactly. Formatting while editing (FormatRuns/Paragraphs/Spacing via the ribbon) is an ordinary command: it ends the live step and is its own step, as before.
* **Caret hand-over.** The caret is converted to a story position (`story::caret_to_pos`: paragraph, glyph) before the sync and found again after the re-flow (`Story::frame_for`, `pos_to_caret`): in the same frame (the buffer is reloaded and the caret restored if text flowed out of it) or in a later one. A frame owns the positions up to its end; the end position belongs to the next frame when it is the start of a paragraph. The overlay moves by changing `EditState.id`/`page` (`App::edit_frame`); the page follows (frame 3 is on page 2), the new editor takes the focus and the caret on the next frame.
* **Keys at the edge** (`app::linked_key`, consumed before the `TextEdit` sees them; only with a collapsed caret and no modifiers): Left/Up at the start of a frame (not the first) go to the end of the previous frame; Right/Down at the end (not the last) go to the start of the next frame (offset 0, even when the frame ends mid paragraph, so the same story position is passed over once); Backspace at the start of a frame (not the first) deletes what precedes the frame's text in the story, which is the last glyph when the frame starts mid paragraph and the paragraph break when it starts a paragraph (the two paragraphs join), and editing continues at that point in the previous frame; Delete at the end of a frame deletes the glyph that follows in the story (or the paragraph break at a paragraph end), and the frame pulls the next text in. These two use the new `Command::SetStoryText { story, paragraphs }` (replaces the story text, caches follow, then the normal re-flow) with `story::delete_before` / `delete_at`.
* **Selections.** Stay inside one frame. Shift+arrows at an edge do nothing (clamped); Ctrl+A selects the frame's text. Mouse selection across frames and Shift+arrows crossing frames are phase 6.
* **Formatting.** The ribbon commands act on the selection in the current frame (`format_info` ranges are buffer offsets), then re-flow; `exec` reloads the edit buffer afterwards (text may have moved on or come back).
* **Measured** (kittest, 50 typed characters at the end of frame 1 of linked.pub, one key press per frame, includes a hand-over): `live_sync` (store + re-flow + caret) about 1.4 ms per key in a debug build, 0.22 ms in release; a whole key including the egui frames about 55 ms debug, 4 ms release. Typing stays smooth, so no debounce.
* **Deviations / notes.** (1) Backspace at the start of a frame that starts a paragraph removes the paragraph break (Publisher's paragraph mark), not a visible glyph of the previous frame; at a frame that starts mid paragraph it removes the last glyph. (2) Right/Down at the end of a frame lands at offset 0 of the next frame. (3) Up/Down only cross the boundary when the caret is at the very start/end of the frame's text, not on the first/last line. (4) A live step keeps one pair of whole-story copies in the history however long the session (compaction), so memory does not grow per key. (5) Test counters: `app::live_timing` (cfg(test)). Tests: `ui_tests/linked_text.rs` (13), core `tests/linked_story.rs` phase 3 block (4) and `story.rs` unit tests (3).
* **Left for later phases.** Overflow indicator and delete/paste/duplicate of frames (phase 4); Next/Previous ribbon (5); selection and Ctrl+A across frames, Up/Down by visual line across the boundary, Page keys (6); Delete/Backspace of a selection that spans a boundary (6).

## 7c. Phases 4 and 5 as built (2026-11-15), follow docs/ui/LINKED-TEXT.md

* **Overflow box and link arrows** (`linkmarks.rs`, drawn by `canvas.rs`): only while ONE text frame is selected (shape or caret). A 14 px (fixed screen size) white square, 1 px black border, outside the right edge in the lower part (offset above the bottom = 7 + 0.123 x height px, fitted to the two measured frames): three dots when the frame overflows (last frame of a chain with story text left, or a standalone box whose text does not fit), a right arrow in the same place when it has a next link; a left arrow outside the left edge, upper part (17.7 + 0.082 x height below the top), when it has a previous link. The selection handles are red (`OVERFLOW_RED`) while overflowing. Arrow click = Next / Previous (a click on any box does not end the text edit); the overflow box swallows its click. Overflow: `Document::frame_overflows` (chain: last frame and `Story::overflow()` has text; standalone: `Document::text_overflows` flows the text through the one frame with the same `FlowContext` as the chain flow). While typing in a standalone box the editor buffer is flowed (the model still has the old text); the answer is cached per (revision, page, frame, typed text).
* **Linking group** (Text Box tab): Create Link / Break / Next / Previous with the table of the doc (`App::link_state`, `Document::frame_links`). Next enters text edit in the next frame (caret at the start of its text), Previous in the previous frame (caret at the end), switching page; no wrap-around. Create Link and Break show the Publisher enabled state but do nothing (tooltip "Coming soon"; they need the phase 7 link writer).
* **Delete.** `DeleteShapes` and `DeletePage` just remove shapes; afterwards `Editor::relink_patches` (same undo step) closes every story that lost frames: the story text stays, the frames that are left are laid out again (`Document::reflow_value`), a story with no frame left is removed. New patches `InsertStory` / `RemoveStory`. A chain that is down to one frame stays a one-frame story ("not linked": no arrows, but it overflows / shows the start of the story as in Publisher).
* **Save.** A chain that lost a frame (also through a deleted page) is reported in `plan.unsupported` as "changes to linked text boxes (deleting a linked text box)" (instead of "deleted objects"); the text of the story is taken from the document's story (so edits made before the delete are still written); if no story is left at all nothing is written for it. The writer cannot re-link (items 39/40), so the file keeps the frame, which comes back at the next open. Deleting a page with linked frames still runs the supported page deletion (`PageOp::Delete`, which leaves the other frames' chain positions as they were) and is reported the same way.
* **Copy / paste.** `App::copy_selection` blanks the copies of linked frames (one empty paragraph with the first paragraph's formatting, not in any story); `paste` puts those copies 9 pt right and down per paste (other shapes keep the 0.25 in step).
* **Duplicate page.** `DuplicatePage` copies linked frames without text; the copies on the new page become a new story (own id, in chain order, empty text) when there are two or more, or an empty box when there is one. Save already leaves out duplicated pages with linked frames (`page_blocks_duplicate`). Moving a page changes nothing in the stories.
* **Deviations from LINKED-TEXT.md.** (1) The pasted box is selected but not in text-edit mode (the editor's paste never enters edit). (2) The overflow / arrow boxes keep 14 px at every zoom and the vertical rule is fitted to the two frames measured at 54 % zoom. (3) While editing text, the frame outline is the dashed edit outline (no handles), so "red handles" shows only for a shape selection. (4) Text Fit is a greyed placeholder in the editor, so nothing was changed. (5) Duplicate and Ctrl+drag copy do not exist in the editor (Ctrl+D does nothing, tested). (6) No "contains objects" prompt on deleting a page (the editor has none). (7) The copy-of-the-last-frame oddity (source text slice growing by a line) is not reproduced. (8) Deleting the last frame of a story drops the story as Publisher does, with no question asked (the doc leaves that to the owner). (9) Create Link / Break do not act yet (phase 7).
* **Tests.** kittest `ui_tests/linked_frames.rs` (19), core `tests/linked_story.rs` phase 4 / 5 block (9), `linkmarks` unit test (1).

## 8. Tests per phase

* Phase 0 (core integration, tests/save.rs): open `linked.pub`, edit frame 1, `build`, assert the saved story still contains "Paragraph 30" and its length changed by exactly the inserted text; same for an edit in frame 3 (the overflowing frame); a no-op plan stays empty.
* Phase 1 (core): `from_pub_flowed(linked.pub)` gives one `Story` with 3 frames, `paragraphs.len() == 30`, `ends == [(4,0),(8,0),(16,0)]`, remainder = paragraphs 17-30; serde round-trip of a doc with a story; frames not in a story are unchanged.
* Phase 2 (core): `SetText` growing frame 1 pushes lines into frame 2 and shifts `ends`; shrinking pulls text back; the story text is unchanged except for the edit; undo restores both frames and the story; moving or resizing a frame re-flows; a frame with a wrap obstacle re-flows when the obstacle moves; `flow_story` equals `chain_breaks` for the unmodified sample (single source of truth test).
* Phase 3 (kittest): un-ignore `text_typed_into_a_linked_frame_flows_on_into_the_next_frame` and tighten its stray assertion (`starts_with(&second_before[..0])` is always true; assert the second frame now begins with the text that used to end frame 1). Add: typing so much that the caret lands in frame 2 moves the edit overlay (`ui.edit` id changes); Backspace at the start of an overflowed frame pulls text back; one Ctrl+Z restores both frames; save after the edit keeps the whole story (reopen, compare).
* Phase 4 (kittest): the overflow marker appears on the last frame of `linked.pub` and not on frame 1; disappears after deleting text; delete a middle frame and the text re-flows with no loss; paste gives an independent box; AutoRecover snapshot round-trip keeps the chain (core test) and a v1 snapshot still loads.
* Phase 5 (kittest): Next/Previous enabled only with a linked neighbour, select the neighbour and switch page (frame 3 is on page 2).
* Existing tests: `a_linked_story_is_shown_split_across_its_frames` stays as is; `docs/testing/UI-TEST-PLAN.md:129` updates its "ignored: BUG" note.

## 9. Risks

* Performance: re-flow runs the whole chain on every edit command; today `chain_breaks` builds a font `Resources` each call. Needs a cached resources handle and flow only the touched story; a 10-frame chain must stay interactive while typing.
* Editor/Publisher agreement: breaks come from pub2pdf's line breaking; pub2pdf matches Publisher only as well as its metrics do. If they differ, the saved file is still right (Publisher re-flows from the single story), only the on-screen split differs.
* Glyph-offset semantics: `StoryPos` counts glyphs without `\r`, `\n` and controls (convert.rs:57); splice/offset code must use the same definition as pub2pdf and as `text::rebuild_paragraphs` or caret hand-over will be off by control characters and tabs.
* Cache/truth drift: the frame `paragraphs` cache must only be written by `flow`. Add a debug assertion in `Editor::execute` that each cache equals its slice.
* Formatting commands on a slice need offset translation; easy to get wrong at a mid-paragraph frame start (run formatting and paragraph spacing of the continuing paragraph).
* Overflow-only story fragments: a frame's slice can be empty; an empty frame's `TextBox` keeps one empty paragraph (convert does this now) and edit must still find its story offset.
* AutoRecover: stale v1 snapshots of linked files reload as independent frames (acceptable; same as what v1 saved).
* Save of user-made chain changes (create/break/delete frame, move text between stories) has no writer: must go through `unsupported` and never silently drop text.
* Wrap obstacles on other pages or inside rotated groups: the flow must use the same obstacle rules as the PDF path.

## 10. Out of scope for the MVP (phases 0-4)

* Create Link / Break / "Continued on page" notes (phase 7), Find/Replace and spell check across the story, selecting/copying a whole story, autofit (TB-AUTOFIT) interplay, tables inside chains (already excluded in save.rs:590), text in groups, drop caps.
* Writing a chain structure the file did not have (needs pub-write items 39/40).

## 11. Open questions for the owner

1. Observe in Publisher (read-only, per the research rules): overflow indicator look/position, what deleting a middle frame does to the chain, what copy-paste of a linked frame does, behaviour of Next/Previous and Create Link, "Continued on page" format. May I add a short observation task before phases 4, 5 and 7?
2. Ship phase 0 now as its own commit (it removes silent data loss that exists today)?
3. Should a v1 AutoRecover snapshot of a linked file be kept loadable (proposed) or discarded?
4. Is interactive speed on a 10-frame chain (the POI newsletter sample) a hard requirement for phase 2, or is "re-flow on commit" (when the edit is synced, not every keystroke) acceptable at first? That is cheaper and still fixes the bug as the test drives it.
