# KernPage: Bullets and Numbering, as Publisher 2021 shows and behaves

Source: Publisher 2021 (version 16.0, Windows 11) driven through the public COM object model (`TextRange.Paragraphs(i,1).ParagraphFormat`: `SetListType`, `ListType`, `ListBulletText`, `ListBulletFontName`, `ListBulletFontSize`, `ListIndent`, `ListNumberSeparator`, `ListNumberStart`, `LeftIndent`, `FirstLineIndent`, `RightIndent`), PDF export of the live document, and byte diffs of the files it saved (`samples/gen/list-*.pub`, spec section 4R) on 2026-11-21. Words only; no screenshots or artwork are stored. Only the public automation interface, the visible program and our own files were used.

How the ribbon and dialog details were obtained: an **early part of this research read the Home tab's Bullets and Numbering galleries and the Bullets and Numbering dialog through the UI Automation tree** (names, enabled state, values; no clicks or keys; the dialog was closed with a window-close message). The research was then stopped because Publisher showed "not responding" popups while it was probed, and the rules were changed to **COM only**. Sections 2 and 3 therefore hold what was read before the stop; section 7 lists what still needs a person to look at it.

## 1. What COM exposes (the ParagraphFormat list properties)

There is **no `ListFormat` object** in Publisher; the list members are on `ParagraphFormat` itself:

| Member | Type | Notes |
|---|---|---|
| `ListType` | read-only, `PbListType` | 255 = none (also the value of an ordinary paragraph); 0 Arabic, 1 upper Roman, 2 lower Roman, 3 upper letters, 4 lower letters, 5 ordinal (1st), 6 cardinal text (One), 7 ordinal text (First), 22 leading-zero Arabic (01), 23 bullet |
| `SetListType(Type, BulletText)` | method | Sets the list type. Only the values above (and 255 = remove) are accepted; others raise "Value does not fall within the expected range". `BulletText` is used only with 23 (a one-character string; "" = the default bullet); with a numbering type a non-empty string is an error. Applying a type to an already-listed paragraph replaces the type and keeps the other list settings it still applies to. |
| `ListBulletText` | read-only string | The bullet character. Raises an error on numbered and plain paragraphs. |
| `ListBulletFontName` | string, read/write | Default `Symbol`; errors on non-bullet paragraphs. |
| `ListBulletFontSize` | float, read/write | Default 10 pt; accepted 1 to 400, 0 and 1000 fail. |
| `ListNumberSeparator` | `PbListSeparator`, read/write | Exactly nine values `n * 65536`, n = 0..8: 0 `1)`, 65536 `(1)`, 131072 `1.` (default), 196608 `1` (none), 262144 `1]`, 327680 `1:`, 393216 `[1]`, 458752 `-1-`, 524288 `1` plus a wide mark (not shown in the PDF text). Errors on bullets and plain paragraphs. |
| `ListNumberStart` | int, read/write | Default 1; 0 up to at least 100,000 accepted, -1 fails. Shared by the whole run of adjacent list paragraphs (setting it on one paragraph changes its neighbours' value too). |
| `ListIndent` | float (pt), read/write | Default 18 pt (0.25 in). Setting it sets `LeftIndent` = value and `FirstLineIndent` = -value. A plain paragraph reads 18 as well. |
| `LeftIndent`, `FirstLineIndent`, `RightIndent` | float (pt) | Ordinary paragraph indents; a list paragraph has left 18 and first-line -18 (a hanging indent) until changed. |

## 2. Ribbon: Home tab, Paragraph group (read through UI Automation before the stop)

- Two split buttons side by side, named **Bullets** and **Numbering**, followed by **Decrease Indent** and **Increase Indent** (all four are icon-only, no visible text). They are enabled when a text range is selected.
- **Bullets** drop-down gallery, section header **Bullet Styles**, tiles of 90 x 90 px, three per row, in this order: **None**, **Small Bullets**, **Large Bullets**, **Filled Diamond Bullets**, **Arrow Bullets**, **Open Diamond Bullets**, **Asterisk Bullets**; below a separator the command **Bullets and Numbering...**.
- Applying a tile through the object model gave: Small = Symbol U+00B7 at 10 pt; Large = Symbol at 12 pt (character code not read); Filled Diamond, Arrow, Open Diamond = Symbol 10 pt with other character codes (not read; the six default codes in the document's bullet table are 183 at 10 pt, 183 at 12 pt, 168 at 10 pt, 222, 224, 42 at 10 pt per spec section 5; **Asterisk = `*`**, code 42). All set `ListType` 23, indent 18 pt, left 18, first-line -18.
- **Numbering** drop-down gallery, tiles 90 x 90 px, three per row: **None**, **1. 2. 3.**, **1) 2) 3)**, **I. II. III.**, **A. B. C.**, **a. b. c.**, **a) b) c)**, **i. ii. iii.**, **i) ii) iii)**; then the command **Bullets and Numbering...** (the same dialog). The tiles map to type and separator: `1.` = type 0 with separator 131072, `1)` = type 0 with separator 0, `I.` = 1, `i.`/`i)` = 2, `A.` = 3, `a.`/`a)` = 4 (with `)` = separator 0).
- **None** in either gallery clears the list (type 255) and resets the indents to 0.
- Tooltips of the tiles are the names above (the numbering tiles are named by their sample text).

## 3. Bullets and Numbering dialog (read through UI Automation before the stop)

A modal Win32 dialog titled **Bullets and Numbering** (about 387 x 440 px) with a two-tab control; **the Bullets item of the Bullets menu opens it on the Bullets tab, the Bullets and Numbering item of the Numbering menu opens it on the Numbering tab** (so it is the same dialog, tab chosen by the entry used). Buttons: **OK** (default), **Cancel**; a hidden **Apply** and **Help** exist in the template.

**Bullets tab** (page title "Bullets"):
- Group **Bullet character**: **No Bullet** plus six owner-drawn buttons **Bullet type 1** to **Bullet type 6** (the six default bullets).
- **Size:** edit box with spinner (shows `10pt`; disabled until a bullet type is chosen), **Character...** button (opens a character picker; disabled until a bullet is on; not opened).
- **Indent list by:** edit box with spinner (shows `0.25"`), disabled while "No Bullet" is selected.
- **Sample** preview box.
- With the paragraph having no list, Size, Character... and Indent are disabled.

**Numbering tab** (page title "Numbering"):
- **Format:** combo box (its list was not read).
- **Separator:** combo box (value `.` for the default), list of nine entries in this order: **None**, **.**, **)**, **]**, **:**, **()**, **[]**, **--**, and a ninth entry that did not read as text (wide mark). (The list order differs from the COM enumeration order of section 1.)
- **Start at:** edit box with spinner (1). **Indent list by:** edit box with spinner (`0.25"`). A hidden check box **Add double quote for Hebrew alphabet numbering**. **Sample** preview.

The dialog's accelerators are the usual ones (`&Format`, `&Separator`, `Start a&t`, `&Indent list by`, `B&ullet character`, `&Size`, `&Character...`).

## 4. Behaviour

- **Applying a list** sets the paragraph's left indent to the list indent (default 0.25 in / 18 pt) and the first-line indent to minus that, so the bullet or number hangs at the old left edge and the text starts 18 pt in; wrapped lines align with the first line's text (hanging indent) ✔ (live render, wrapped paragraph).
- **Bullet**: the character is drawn in its own font and size (default Symbol bullet, 10 pt) at the paragraph's first-line position; the text starts at the left indent. The bullet's size is independent of the text size.
- **Number**: formatted as number-in-format plus separator; text font is the paragraph's. Numbering is by **runs of adjacent list paragraphs**: a non-list paragraph ends the run and numbering restarts (at the start value) with the next list paragraph. An empty list paragraph still shows its number. A different start value starts a new run (5, 6 then 10, 11).
- **Start at** is applied to the whole run of adjacent list paragraphs; it is stored as an item on every paragraph of the run.
- **Mixed paragraphs**: each paragraph carries its own list settings; a text box can freely mix plain, bulleted and numbered paragraphs.
- **Removing** (None) removes the list and the hanging indents; KernPage must ignore the leftovers Publisher keeps in the file (spec 4R.1).
- **Decrease/Increase Indent** and **Tab**-based list level changes were not observed (no list levels exist in the object model: only one indent per list paragraph).
- **A live-numbering quirk**: when `SetListType` is called on adjacent paragraphs one at a time, Publisher's live layout showed 1 1 2 3 although the saved records equal the whole-range case. Do not copy it.

## 5. File format summary

Full detail in `docs/spec/PUB-FORMAT-SPEC.md` section 4R. In short: lists are paragraph properties in the Quill `FDPP` run record: list descriptor 0x257 {0x200 type, 0x201 bullet character}, 0x258 separator (`index << 16`), 0x215 start, 0x206 list indent, 0x20C/0x20D/0x20E first-line/left/right indents, 0x202 bullet size (EMU) and 0x203 bullet font (index into the Quill `FONT` table). No text characters are stored.

## 6. Current state in KernPage

Nothing implemented: the reader skips these items, the writer never writes them, `pub2pdf` draws no bullets or numbers, and the ribbon's Bullets and Numbering buttons are "soon" placeholders. See the implementation steps in the research report (reader and writer; editor commands and save; ribbon gallery and dialog).

## 7. Open questions: needs a look by hand

Please check these in the real Publisher 2021 window (with a text box and a paragraph selected) and write the answers into this file:

- [ ] Bullets gallery: exact glyph shown in each of the six tiles and the exact character codes behind "Large", "Filled Diamond", "Arrow" and "Open Diamond"; whether a tile shows a highlight for the current bullet.
- [ ] Bullets and Numbering dialog, **Bullets tab**: the glyph on each of the six type buttons, which one is selected on opening for an existing bullet, what the Size box accepts (range, units, rounding), what **Character...** opens (title, font list, symbol grid, recently used characters, OK/Cancel), what the Indent box accepts (units, range) and how the Sample looks.
- [ ] **Numbering tab**: the items of the **Format** list (the nine numbering formats? the other ones from the type list?), the exact text of every Separator entry including the ninth, default values on a fresh paragraph.
- [ ] Which tab opens when the paragraph has no list and the dialog is opened from the Bullets menu vs the Numbering menu; whether the choice is remembered.
- [ ] Tab names and key order, the **Add double quote for Hebrew alphabet numbering** check box (when it is shown: only with a Hebrew locale?).
- [ ] Keyboard behaviour: Tab at the start of a list item, Enter on an empty list item (ends the list?), Backspace at the start, Shift+Tab; Decrease/Increase Indent steps (0.25 in?) and limits.
- [ ] Whether Bullets/Numbering act on every paragraph of a selection, on the caret's paragraph only, or on the whole story when a text box (not text) is selected (COM: `TextRange` of the whole box applies to all).
- [ ] Undo behaviour (one step per command?).
- [ ] Key tips of the two buttons (Alt, H, B / Alt, H, N were noted from the ribbon key tips earlier) and the tooltips' wording.
- [x] Reopening a saved list file in Publisher: all 24 `samples/gen/list-*.pub` open (COM verify, 2026-11-21).

### Answers from the owner's screenshots (2026-11-21)

**Bullets gallery** (Home > Bullets arrow):
- Header "Bullet Styles". Tiles in a 3-column grid, each tile shows three bullets with a line after each:
  None (the word "None" in large type), small round dot, large round dot, filled diamond (♦), double arrow (⇒), open diamond (◊), asterisk (✱-like, six points).
- The current style's tile has an orange border (with no list, "None" is highlighted).
- Below a separator: list icon + "Bullets and Numbering..." (B underlined as access key).

**Bullets and Numbering dialog, Bullets tab** (title "Bullets and Numbering", "?" and close buttons; tabs "Bullets", "Numbering"):
- Group "Bullet character": 7 square buttons in a row — blank (no bullet; thick border when selected), small dot, larger dot, ♦, ⇒, ◊, *.
- "Size:" spin box and "Character..." button (both disabled when no bullet), "Indent list by:" spin box showing `0.25"` (disabled when no bullet).
- Group "Sample": grey lines above and below, black lines for the list paragraphs.
- OK, Cancel.

**Bullet Character dialog** (opened by Character...):
- Title "Bullet Character", "?" and close. "Font:" combo (Symbol), with F underlined.
- Symbol grid 32 columns wide, about 7 rows visible, current cell highlighted blue.
- "Recently used symbols:" one row of about 30 cells (€ £ ¥ © ® ™ ± ≠ ≤ ≥ ÷ × ∞ µ α β π Ω ∑ ☺ ☹ § † ‡ … % é ¿ ñ ← ↑ →).
- Bottom: "Symbol: 32" (name/code of the selected symbol), "Character code:" text box (32), "from:" combo "Symbol (decimal)". OK, Cancel.
- Looks like Office's shared Symbol dialog.

**Numbering tab:**
- "Format:" combo (default `1, 2, 3, …`), "Separator:" small combo (default `.`), "Start at:" spin (1), "Indent list by:" spin (`0.25"`), Sample showing `1.` `2.` `3.`, OK, Cancel.
- Format list items (first six seen, list scrolls for more): `1, 2, 3, …`, `I, II, III, …`, `i, ii, iii, …`, `A, B, C, …`, `a, b, c, …`, `1st, 2nd, 3rd …`. The rest are presumably One/First/01 — matching COM types 6, 7, 22 (not seen).

**Still open:** the Separator list's exact entries; keyboard behaviour (Tab, Shift+Tab, Enter on an empty item, Backspace at the start); undo granularity. Owner skipped these for now — we use the usual Office behaviour (Enter on an empty item ends the list, Backspace at the start removes the bullet first, Tab/Shift+Tab change the indent) until checked.
