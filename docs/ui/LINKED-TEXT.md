# Linked text frames in Publisher 2021 (observed behaviour)

Purpose: record what Microsoft Publisher 2021 (version 16.0, 64-bit, Windows 11) does around linked text frames, so that phases 4 and 5 of `docs/design/LINKED-STORIES.md` (overflow indicator, delete / paste / duplicate of frames, Next/Previous ribbon) match it.

Method: visible UI only (window screenshots, UI Automation names and enabled states of the ribbon), the public COM object model (`TextFrame.HasNextLink`, `HasPreviousLink`, `Overflowing`, `Story.TextRange`, `Selection`), and our own sample. Only the public automation interface, the visible program and our own files were used. All work was done on throw-away copies of `samples/gen/linked.pub`; nothing was saved.

Test file: `samples/gen/linked.pub`. Two pages. Page 1: "Text Box 1" (P1 to P4), "Text Box 2" (a standalone box, 15 characters), "Text Box 3" (P5 to P8). Page 2: "Text Box 4" (P9 to P16 visible, 4 inches high, overflowing). Chain: Text Box 1, then 3, then 4. Story length 2781 characters, 30 paragraphs ("Paragraph 1 ..." to "Paragraph 30 ..."). Initial COM readings: frames 1 and 3 have `HasNextLink = -1`; frames 3 and 4 have `HasPreviousLink = -1`; only Text Box 4 has `Overflowing = -1`. Below, "frame 1/2/3" mean the first, middle and last frame of the chain (Text Box 1/3/4).

Zoom during all observations was 54 %, window maximised (1920 x 1080 screen). Pixel figures below are screen pixels at that zoom (about 0.9 px per point, 1 inch = 65 px).

Wording in quotes is a short ribbon, menu or dialog label as seen on screen; longer messages are paraphrased.

## 1. Overflow indicator and link indicators

### 1.1 Overflow indicator

* Look: a small square, about 14 x 14 px at 54 % zoom, white fill, 1 px black border, containing three black dots in a row ("..."). It is a separate little box, not part of the frame outline.
* Position: just OUTSIDE the right edge of the frame, touching the edge, vertically in the lower part of the frame: the box centre is about 15 px above the frame bottom for a 72 pt high frame (about 0.76 of the height from the top), about 39 px above the bottom for the 288 pt high last frame (about 0.85 of the height). The exact rule is not determined (it is not a constant fraction; it behaves like a fixed margin plus a few percent of the height). Always on the right side, never on the left.
* Which frames: any text frame whose text does not fit: the LAST frame of a chain, and also a standalone (unlinked) box (checked: replacing the standalone box text by 8 short paragraphs showed the same indicator). A frame that is not last in a chain never shows it, because its overflow simply flows into the next frame (COM `Overflowing` was 0 for frames 1 and 2).
* When visible: ONLY while the frame is selected (whole-frame selection or text caret inside it). An unselected overflowing frame shows nothing at all (no indicator, no icon, no coloured outline; checked with the last frame deselected: plain text, the clipped text just stops).
* Selection handles of an overflowing frame turn RED (the corner circles and the side squares are filled red). When the frame does not overflow the handles are the usual white circles / squares with a blue-grey outline. So there are two overflow cues while selected: red handles and the "..." box.
* When it disappears: as soon as the text fits (checked by cutting the story from 30 to 15 paragraphs: the last frame then showed P9 to P15, `Overflowing = 0`, white handles, no "..." box). It also disappears with the Ctrl+Z that restores the original.
* Click: clicking the "..." box does nothing (selection and caret unchanged; no popup, no navigation, no menu). Not observed: a tooltip (hovering for 2 s showed none).
* It shows in text-edit mode too (caret in the last frame: the "..." box and the left arrow are visible).

### 1.2 Link indicators ("Go to Next / Previous Text Box" arrows)

* Look: the same kind of small white square with 1 px black border (about 14 x 14 px), containing a solid black triangle: pointing RIGHT for the next-frame arrow, pointing LEFT for the previous-frame arrow.
* Position: the NEXT arrow sits outside the right edge, in the lower part of the frame, at the same place the "..." box would be (so the last frame, which has no next link, shows "..." instead of a right arrow when it overflows, and nothing there when it does not). The PREVIOUS arrow sits outside the LEFT edge, in the upper part of the frame (about 23 px below the top for a 72 pt high frame, about 39 px below the top for a 288 pt high frame; about 0.15 to 0.18 of the height).
* Which frames: first frame: right arrow only. Middle frame: left and right arrows. Last frame: left arrow only (plus "..." when overflowing). Standalone box: no arrows.
* When visible: only while the frame is selected (frame selection or caret inside). Unselected linked frames show no arrows. With a multiple selection (two or more frames selected) no arrows are drawn on any of them.
* Click (works exactly like the ribbon buttons Next / Previous, see 2.3): clicking the right arrow goes to the next frame in the chain with the caret at the START of its text; clicking the left arrow goes to the previous frame with the caret at the END of its text. If the target is on another page, that page is shown. Verified with the mouse on each arrow (frame 1 right arrow to Text Box 3 at story offset 368; frame 2 right arrow to Text Box 4 on page 2 at offset 736; frame 3 left arrow to Text Box 3 at 735; frame 2 left arrow to Text Box 1 at 367).
* Not observed: tooltip text of the arrows (none appeared).

## 2. Text Box tab, Linking group

The "Text Box" contextual tab (under "Text Box Tools", next to "Shape Format") appears when a text frame is selected. Its groups, left to right: "Text", "Font", "Alignment", "Linking", "WordArt Styles", "Typography". The "Linking" group has four buttons (UI Automation control type Button):

| Label | Position |
| --- | --- |
| "Create Link" | large button, left, icon of two chain links |
| "Break" | small button, top right |
| "Next" | small button, middle right |
| "Previous" | small button, bottom right |

Enabled states (UI Automation `IsEnabled`, confirmed visually: disabled buttons are grey):

| Selection | Create Link | Break | Next | Previous |
| --- | --- | --- | --- | --- |
| Standalone (unlinked) box | enabled | disabled | disabled | disabled |
| First frame of a chain (shape selected, or caret in it) | disabled | enabled | enabled | disabled |
| Middle frame | disabled | enabled | enabled | enabled |
| Last frame | ENABLED | disabled | disabled | enabled |
| Two or more shapes selected (any mix: frames 1+3, 1+standalone, standalone+3, all three) | disabled | disabled | disabled | disabled |

Notes:
* The last frame keeps "Create Link" enabled (a chain can be extended at its end); first and middle frames have it disabled (they already have a next link).
* "Break" is enabled exactly when the frame has a next link, i.e. it breaks the link FORWARD of the selected frame.
* With a multiple selection the whole Linking group is disabled and the "Text" group "Text Fit" and the font controls are disabled as well.

### 2.3 What Next / Previous do

Starting from frame 1 selected as a shape (selection type "shape"):

1. "Next" once: the selection becomes a text caret (insertion point) in the middle frame (Text Box 3), at the start of its text (story offset 368 = first character of "Paragraph 5"). Page 1 stays shown.
2. "Next" again: the caret moves into the last frame (Text Box 4) at its text start (offset 736), and the view switches to page 2 (status bar "Page: 2 of 2"). "Next" is then disabled; clicking where it was does nothing.
3. "Previous" from the last frame: caret in the middle frame, at the END of its text (offset 735, i.e. just before the final paragraph mark of that frame's slice); page 1 is shown again.
4. "Previous" again: caret in frame 1 at its text end (offset 367). "Previous" is then disabled.

So: Next and Previous do not just select the frame; they enter text-edit mode in the neighbour frame (a caret, the frame outline shown with its handles) and change page if needed. Next puts the caret at the beginning of the neighbour's text, Previous at its end. The Linking button states update for the new frame (see the table). The `Selection` is a text selection of length 0 (no highlighted text).

### 2.4 Text Fit on linked frames

See section 7.

## 3. Deleting a linked frame

Method: select the frame as a shape (single click on its border, not in the text), press Delete. Read back with COM. In every case the text of the story is kept in full (story length stays 2781 characters, 30 paragraphs, and there is still exactly one story for the chain). Deleting a frame never deletes any of the chain's text; only deleting the very last frame of a story removes the story.

| Action | Result |
| --- | --- |
| Delete the MIDDLE frame (Text Box 3) | The chain is re-linked: Text Box 1 -> Text Box 4 (`HasNextLink` of 1 stays -1, `HasPreviousLink` of 4 stays -1). Text Box 1 still shows P1 to P4. Text Box 4 now shows P5 to P12 (8 paragraphs, 739 characters; it was P9 to P16) and still overflows. Story unchanged. The text flows on into the remaining frames. |
| Delete the FIRST frame (Text Box 1) | Text Box 3 becomes the first frame (`HasPreviousLink = 0`, `HasNextLink = -1`) and now shows P1 to P4 (368 chars). Text Box 4 now shows P5 to P12. Story unchanged (starts at "Paragraph 1"). |
| Delete the LAST frame (Text Box 4) | Text Box 3 becomes the last frame (`HasNextLink = 0`) and now overflows (`Overflowing = -1`, shows P5 to P8). Story unchanged (all 30 paragraphs still there, hidden). Page 2 stays (empty). |
| Delete first, then (new) first (two of three frames) | The remaining frame (Text Box 4, on page 2) is alone: no links, shows P1 to P8 and overflows. Story unchanged. |
| Delete all three frames | The story disappears (`Stories.Count` drops from 2 to 1; only the standalone box's story is left). The 30 paragraphs are lost. |

There was no confirmation dialog for deleting a frame (Delete key). Overflow-only text is never offered back to the user.

Undo (Ctrl+Z), one step each: restores the frame in its position and page, its links, and its text slice (middle, first, last, and each single delete in the sequence above). Redo (Ctrl+Y) after the middle delete re-applied the delete correctly; Ctrl+Z again restored the chain.

Observed identity: after deleting Text Box 1 the shape names of the survivors are unchanged (Text Box 3, 4); the names are not renumbered.

## 4. Copy / paste and duplicate of a linked frame

Tried on the middle frame, the first frame and the last frame.

* Copy (Ctrl+C) then paste (Ctrl+V): the pasted shape is a NEW, EMPTY, UNLINKED text box. COM: `len = 1` (just a paragraph mark), `HasNextLink = 0`, `HasPreviousLink = 0`, `Overflowing = 0`, and a new separate story of 1 character (`Stories.Count` went from 2 to 3). It does not carry the text, neither the whole story nor the frame's slice. Size equals the source frame (216 x 144 pt or 216 x 288 pt). Position: offset by +9 pt right and +9 pt down from the source (0.125 in), on the same page as the source. The pasted box is selected and in edit mode (insertion point at offset 0). The original frames and their links are unchanged, except an oddity when the LAST (overflowing) frame was copied: afterwards its text slice read P9 to P17 (792 characters instead of P9 to P16, 743), with a partly visible extra line at the bottom; undo restored P9 to P16. (This was seen both right after the paste and after Esc.)
* Ctrl+D: has no effect in Publisher 2021 (checked: nothing changed in the document; no dialog). There is no "Duplicate" button in the ribbon.
* COM `Shape.Duplicate` (the same operation that the Ctrl+drag copy performs): same result as paste: an empty, unlinked box with one 1-character story, placed exactly on top of the source (same position, same size).
* Ctrl + drag copy (mouse press on the frame border with Ctrl held, drag 200 px to the right, release): a new empty unlinked box (1 character, own story) at the drop position (here 222 pt to the right of the source); the source is left in place with its chain intact.
* Undo: one Ctrl+Z removes the pasted / duplicated box, and the story count and chain return to the starting state. Checked for paste (three frames), COM duplicate and Ctrl+drag.

## 5. Duplicate / delete / move a page that holds linked frames

Page menu (right-click a page thumbnail in the "Pages" pane): "Insert Page...", "Insert Duplicate Page", "Delete", "Move...", "Rename...", "Page Numbers" (submenu), "Master Pages" (submenu), "View Two-Page Spread". The COM calls `Page.Duplicate`, `Page.Delete`, `Page.Move` behave the same as the menu items (checked for the duplicate; the delete was checked through both).

### 5.1 Insert Duplicate Page (page 1: frames 1 and 3 of the chain plus the standalone box)

* A new page 2 is inserted after the source, so the old page 2 (frame 4) becomes page 3.
* The duplicated frames are copies of the source frames with NO text: the copy of frame 1 and the copy of frame 3 are LINKED TO EACH OTHER (copy of 1 has `HasNextLink = -1`, copy of 3 has `HasPreviousLink = -1`), forming a new two-frame chain that is separate from the original chain. That new chain's story is 1 character (empty). The copy of the standalone box is a separate unlinked box with its original 15 characters ("Standalone box"; standalone boxes keep their text, linked frames do not).
* The original chain (1, 3, 4) and its 30-paragraph story are untouched (it still runs from page 1 to the old page 2, now page 3).
* The same happens via the menu item "Insert Duplicate Page" (verified) and COM.
* Undo: one Ctrl+Z removes the page and everything on it.

### 5.2 Delete page

* Menu "Delete" on a page that contains objects shows an information dialog that says the page has objects and asks whether to delete it anyway, with buttons "Yes" and "No" (answered "No" in the observation; "Yes" is the COM `Page.Delete` result below).
* Delete page 2 (holds frame 3 of the chain, Text Box 4): the frames on that page are removed. Text Box 3 becomes the last frame (`HasNextLink = 0`), it now overflows, the story is unchanged (30 paragraphs, 2781 characters). This is the same as deleting the last frame.
* Delete page 1 (holds Text Box 1, the standalone box and Text Box 3): the page is removed with its frames. Text Box 4 (page 2, now page 1) is the only remaining frame of the chain: unlinked, shows P1 to P8 (736 characters) and overflows; story unchanged (30 paragraphs). This is the same as deleting the first and middle frames.
* Undo: one Ctrl+Z restores the page, the frames, the chain and all text slices.

### 5.3 Move page

* Moving page 1 to position 2 (`Page.Move(2)`): the pages swap order. The chain does NOT change: it is still Text Box 1 -> 3 -> 4 (Text Box 4 is now on page 1 but still last in the chain). Text is unchanged (frame 4 still shows P9 to P16, frame 1 P1 to P4, frame 3 P5 to P8, story 2781 characters). The chain order is a property of the links, not of the page order, so a chain may run backwards across pages after a page move.
* The "Move..." menu item was not run (it opens a dialog); only the COM `Page.Move`. Undo: one Ctrl+Z restored the original page order.
* Not observed: moving a page by dragging the thumbnail.

## 6. Undo

One Ctrl+Z (single step) restored the previous state for each of: delete first / middle / last frame, deleting the other two frames of a chain (each delete is its own step), paste of a linked frame, COM duplicate, Ctrl+drag copy, Insert Duplicate Page, delete page 1, delete page 2, move page. Chain links, text slices, shape positions, page count and story count all returned to the starting values (checked through COM after each Ctrl+Z). Ctrl+Y (redo) was checked once, for the middle-frame delete, and worked.

## 7. Text autofit on a linked frame

* "Text Fit" is the first control of the "Text" group on the Text Box tab (a drop-down with the icon of an "a" with arrows). Its menu has four items: "Best Fit", "Shrink Text On Overflow", "Grow Text Box to Fit", "Do Not Autofit". The current choice is marked with a tick; the default is "Do Not Autofit" (COM `TextFrame.AutoFitText = 0`).
* It is DISABLED for every frame that is part of a chain: first, middle and last, whether the frame is selected as a shape or the caret is inside it (UI Automation `IsEnabled = False`, greyed out). It is also disabled for a multiple selection.
* It is ENABLED for a standalone text box (checked with the caret in the standalone box; the menu opened and showed the four items).
* "Text Direction" is enabled for a linked frame with the caret inside it.
* So linked frames always behave as "Do Not Autofit": no best fit, no shrink, no grow.

## 8. Behaviour summary for the KernPage implementation

Overflow indicator (phase 4)
1. Draw a small white square with a 1 px black border and three black dots, just outside the right edge of a selected text frame, in its lower part (about 0.75 to 0.85 of the height; 1 px border, about 14 px at 54 % zoom; scale it with the zoom or keep a fixed screen size, which of the two is not observed), when the frame overflows. Applies to the last frame of a chain and to standalone boxes.
2. Draw the selection handles red while the frame overflows, white / blue-grey otherwise.
3. Only when the frame is selected (frame or caret). Nothing is drawn for unselected frames.
4. A non-last frame of a chain is never "overflowing"; overflow is only ever at the end of the chain.
5. Clicking the indicator does nothing.

Link arrows (phase 5)
6. While a linked frame is selected (shape or caret, not a multiple selection) draw: a left arrow box on the left edge (upper part, about 0.15 to 0.18 of the height) if it has a previous link; a right arrow box on the right edge (lower part, same place as the overflow box) if it has a next link. Same look as the overflow box, with a black triangle.
7. Clicking an arrow = the ribbon Next / Previous.

Ribbon "Text Box" tab, "Linking" group (phase 5)
8. Buttons "Create Link", "Break", "Next", "Previous". Enable: Create Link = no next link (standalone box or last frame); Break = has next link; Next = has next link; Previous = has previous link. All four disabled for a multiple selection.
9. Next: enter text edit in the next frame, caret at the start of its text (story offset = frame start), change page if the frame is elsewhere. Previous: enter edit in the previous frame, caret at the end of its text, change page if needed. At the ends the button is disabled (no wrap-around).

Frame delete (phase 4)
10. Deleting a frame removes only the frame. The story text is never changed. Re-link the neighbours (previous.next = next) and re-flow: the text moves on into the remaining frames; if it was the last frame the previous frame becomes the end and gets the overflow indicator. Deleting the first frame makes the next frame the first. Only deleting the last remaining frame of a story drops the story (text lost); that is Publisher's behaviour, but the owner should decide whether to keep the story or ask first (no confirmation dialog in Publisher).
11. One undo step restores frame, links and flow.

Copy / paste / duplicate (phase 4)
12. Pasting, duplicating (Ctrl+drag) or COM-duplicating a linked frame yields an empty, unlinked text box (a single empty paragraph, its own new story), same size. Paste: +9 pt right, +9 pt down, same page, selected and in edit mode. Duplicate: same position. Ctrl+D does nothing. The source chain is untouched. This matches the current design note "Paste and duplicate are unchanged (independent boxes)", except that the independent box is EMPTY, not a copy of the slice (standalone boxes, in contrast, are copied with their text).
13. One undo step removes the new box.

Pages (phase 4)
14. Duplicating a page copies the frames without their text; copied frames of a chain that sit on the duplicated page are linked among themselves in the same order into a new chain with an empty story; the original chain is untouched; the new page goes right after the source.
15. Deleting a page asks for confirmation (Yes / No) when the page has objects, then behaves as deleting each of its frames (see 10). Undo restores everything in one step.
16. Moving a page does not touch the links or the text; chain order is independent of page order.

Autofit
17. The "Text Fit" drop-down ("Best Fit", "Shrink Text On Overflow", "Grow Text Box to Fit", "Do Not Autofit") is disabled for linked frames, enabled for standalone boxes; linked frames always use "Do Not Autofit".

## 9. Not observed

* Tooltips of the link arrows and of the overflow box (none appeared after 2 s).
* Exact positioning rule of the arrow and overflow boxes (only a few sample heights) and their behaviour under zoom changes.
* "Create Link" and "Break" actions (cursor change, effect on text): not part of this task (phase 7).
* "Move..." page dialog, drag-moving a page thumbnail, "Yes" on the delete-page dialog through the UI (the COM call was used for the effect).
* Drag-copy with the mouse across pages; paste on a different page than the source (the last frame was pasted on its own page 2).
* The nature of the "P9 to P17" change on the source when the overflowing last frame is copied (reproduced twice; undo restores).
* Keyboard shortcuts for Next / Previous frame.
* Behaviour with text-edit Delete or cut (Ctrl+X) of a whole linked frame shape (only the Delete key was used).
* The effect of any of these on a file that was Saved (nothing was saved; the writer side is a later phase).
