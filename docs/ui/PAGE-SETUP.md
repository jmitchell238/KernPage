# KernPage: Page Design > Page Setup (Margins, Orientation, Size), as Publisher 2021 shows it

Source: Publisher 2021 (version 16.0, Windows 11, 1920 x 1080 screen) driven by keyboard and read through screenshots and the UI Automation tree on 2026-11-14. This file describes behaviour and **labels in words only**; no screenshot, icon or artwork is stored in the repository (screenshots stayed in a scratch folder). Short command names, labels and values are quoted as seen on screen because users expect them. The file-format side is in `docs/spec/PUB-FORMAT-SPEC.md` section 4Q; ground-truth samples are `samples/gen/page-*.pub` and their `manifest.json` entries.

Nothing was saved to the Publisher installation: no custom page size or preset was created, every dialog was closed with Escape or Cancel, and every document was a throw-away copy.

## 1. Where the group is

- Ribbon tab **Page Design** (key tip **P** after pressing Alt). Groups from left to right: **Template** (Change Template, Options), **Page Setup**, **Layout** (Guides drop-down, an "Align To" label with check boxes **Guides** and **Objects**), **Pages** (Delete, Move, Rename), **Palettes** (the colour-palette gallery), **Fonts**, **Page Background** (Apply Image, Background, Master Pages).
- The **Page Setup** group holds three large buttons, left to right: **Margins**, **Orientation**, **Size**. Each shows an icon above its label and a small down-arrow under the label. They are **drop-down galleries** (the whole button opens the gallery; there is no separate split-button arrow and no click action on the button itself). Key tips: Margins **MR**, Orientation **OR**, Size **SZ** (so Alt, P, M, R / Alt, P, O, R / Alt, P, S, Z).
- The group's caption "Page Setup" sits under the buttons; to its **right end, at the bottom-right corner of the group,** is the small **dialog launcher** (a tiny square arrow). Its tooltip and automation name is **"Page Setup..."**; key tip **SP**. It opens the **Page Setup** dialog (section 5), the same dialog as Size > "Page Setup...".
- The three buttons stay enabled when a document is opened read-only.

## 2. Margins gallery (button "Margins")

A single-column gallery, each row = a small page icon (blue guide lines, the current choice has a highlighted frame around its icon), a bold title and two lines of values in a two-column layout "Top / Left" on the left and "Bottom / Right" on the right. The first row has keyboard focus when the gallery opens (a dark outline with a teal fill); Up/Down move, Enter applies.

| Row | Title | Top | Bottom | Left | Right |
|---|---|---|---|---|---|
| 1 | **Wide** | 1" | 1" | 1" | 1" |
| 2 | **Moderate** | 0.5" | 0.5" | 0.5" | 0.5" |
| 3 | **Narrow** | 0.25" | 0.25" | 0.25" | 0.25" |
| 4 | **None** | 0" | 0" | 0" | 0" |

- Values are written in inches with a straight double-quote suffix, no leading zero trimming beyond `0.5"`, `0"`, `1"`.
- **Order is Wide, Moderate, Narrow, None** (largest to smallest), and the default for a new blank page (0.5", 36 pt) is **Moderate**, so Moderate is the highlighted "current" row on a fresh document.
- When the document's margins match none of the four, the gallery shows a **fifth row titled "Custom Margins"** (an icon with a small star) as the highlighted current choice, below None.
- A horizontal separator, then one command row: **"Custom Margins..."** (key tip **A**), which opens the Layout Guides dialog (section 4).
- Applying a row sets all four margins of the master page at once (Top, Bottom, Left, Right in the table). Nothing on the page moves.

## 3. Orientation gallery (button "Orientation")

Two rows, each a page icon (portrait page, landscape page) and a label, no size text:

1. **Portrait**
2. **Landscape**

The current orientation has a teal-framed icon; keyboard focus starts on row 1 (Portrait). Choosing the other orientation swaps the page's width and height (Letter 8.5 x 11 in becomes "Letter (Landscape)" 11 x 8.5 in; margins stay 0.5" on every side) and the objects keep their positions relative to the page centre (spec 4Q.4). There is no command row and no dialog in this gallery.

## 4. Size gallery (button "Size")

A tall drop-down (about 340 px wide at 100% scaling) with, from top to bottom:

1. A section header **"Built-In"** (bold, left aligned).
2. A scrolling list of large rows (eight rows are visible at once, a vertical scroll bar on the right scrolls to the ninth). Each row: a small blue-outlined page icon on the left (portrait icons are tall, landscape icons wide), a bold name with its orientation in brackets, and under it the size in inches with three decimals trimmed (as shown). The current size has a teal-framed row; keyboard focus starts on the first row.
3. A separator and three command rows, each with a key-tip letter:
   - **"Page Setup..."** (key tip **U**; opens the Page Setup dialog, section 5)
   - **"Create New Page Size..."** (key tip **N**; section 6)
   - **"More Preset Page Sizes..."** (key tip **M**; section 7)

Rows (document in portrait), top to bottom (alphabetical by name, nine rows):

| Name | Size shown |
|---|---|
| A3 (Portrait) | 11.693 x 16.535" |
| A4 (Portrait) | 8.268 x 11.693" |
| A5 (Portrait) | 5.827 x 8.268" |
| B4 (Portrait) | 9.843 x 13.898" |
| B5 (Portrait) | 6.929 x 9.843" |
| Executive (Portrait) | 7.25 x 10.5" |
| Legal (Portrait) | 8.5 x 14" |
| Letter (Portrait) | 8.5 x 11" |
| Tabloid (Portrait) | 11 x 17" |

- **When the document is landscape** the same nine rows are shown with "(Landscape)" and the dimensions swapped: A3 (Landscape) 16.535 x 11.693", A4 (Landscape) 11.693 x 8.268", A5 (Landscape) 8.268 x 5.827", B4 (Landscape) 13.898 x 9.843", B5 (Landscape) 9.843 x 6.929", Executive (Landscape) 10.5 x 7.25", Legal (Landscape) 14 x 8.5", Letter (Landscape) 11 x 8.5" (the current row in the sample); the ninth row was scrolled out of view in the capture and is presumably Tabloid (Landscape) 17 x 11". So Size and Orientation are one family: the gallery lists the orientation the page already has.
- **When the document has a custom size** (neither a preset nor a swap of one) the list starts with an extra first row named **"Current"** with the size under it (for example "5 x 7""), highlighted as the current size, followed by the nine presets.
- The sizes are exact: A4 is 21 x 29.7 cm (shown as 8.268 x 11.693"). Picking a row sets the page size and shifts the objects as in spec 4Q.4; margins stay as they are.

## 5. Page Setup dialog (launcher, or Size > "Page Setup...")

Modal Office-style dialog titled **Page Setup**, about 860 x 490 px, title-bar buttons: **?** (help) and close (X). It has no tabs and **no Orientation control** (orientation is only on the ribbon). Three columns:

**Left column**
- Group **Page**: spin boxes **Width:** and **Height:** (units shown as inches with a straight quote, `8.5"` and `11"` for Letter; typing `5` means 5 in; spin arrows step the value).
- Group **Margin guides** (this order!): **Top:**, **Left:**, **Bottom:**, **Right:** (all `0.5"` by default).

**Middle column**
- Group **Layout type**: a drop-down list with the seven items **One page per sheet**, **Booklet**, **Email**, **Envelope**, **Folded card**, **Multiple pages per sheet** (the default for a normal page), **Web page**.
- Group **Options**:
  - **Target paper size:** drop-down with **Letter (8.5" x 11")**, **A4 (8.268" x 11.693")** and **Custom** (list shown with three entries; the document's paper, Letter here).
  - **Paper width:** and **Paper height:** (dimmed while a fixed paper such as Letter is chosen; presumably editable for Custom, not tried) shown `8.5"` and `11"`.
  - **Side margin:**, **Top margin:**, **Horizontal gap:**, **Vertical gap:** (all `0"` by default; the label-sheet layout fields).

**Right column**
- Group **Preview**: a drawing of the page (outer rectangle) with the margin guides (inner rectangle), and three text lines under it: **Page size: 8.5" x 11"**, **Paper size: 8.5" x 11"**, **1 columns of 1** (sic: "1 columns of 1").
- Buttons at the bottom right: **OK** (default) and **Cancel**.

Initial focus is in Width with its text selected. Enter = OK, Escape = Cancel. Typing a new Width/Height and pressing OK sets a custom page size, and the preview line "Page size" follows the fields. Setting a custom size this way does **not** create or save a named size.

## 6. "Create New Page Size..." dialog

Same dialog as section 5 with the title **Create New Page Size** and one extra group at the top: group **Name** with a text box **Name:** pre-filled with the current size's name (for example "Letter (Portrait)", selected so typing replaces it). Everything below is the same (Page: Width, Height; Margin guides: Top, Left, Bottom, Right; Layout type; Options; Preview; OK; Cancel). OK is expected to store a named custom size in the user's Publisher profile and apply it (not exercised, to leave the installation unchanged).

## 7. "More Preset Page Sizes..." dialog

A resizable window titled **Preset Page Sizes** with minimise (dimmed), maximise and close buttons, a large heading **Select a Page Size**, a round **back** button and a breadcrumb that starts at **Home** (after opening a folder: "Home > Standard"), and at the bottom right **OK** (dimmed until a size is selected) and **Cancel**.

- **Home** shows folder tiles in two sections, scrolled in a single list:
  - **Publication Types**: Standard, Advertisements, Binder Divider Tab, Booklets, Business Cards, Designed Paper, E-mail, Envelopes, Greeting Cards, Mailing Labels, Media Labels, Name Tags, Other Labels, Photo Paper, Postcards, Posters, Small Publications.
  - **Manufacturers**: 3M / Post-it® Asia Pacific, 3M / Post-it® EU, 3M / Post-it® North America, Ace Label, Adetec, A-ONE, APLI, and more below (the catalogue holds 8,712 entries; the label and manufacturer folders make up most of it).
- **Standard** opens a section **Standard Page Sizes** with 18 tiles (icon with a page outline, name, size), in this order:
  Letter (Landscape) 11 x 8.5", Letter (Portrait) 8.5 x 11", Executive (Landscape) 10.5 x 7.25", Executive (Portrait) 7.25 x 10.5", Legal (Landscape) 14 x 8.5", Legal (Portrait) 8.5 x 14", Tabloid (Landscape) 17 x 11", Tabloid (Portrait) 11 x 17", A4 (Landscape) 11.693 x 8.268", A4 (Portrait) 8.268 x 11.693", A5 (Landscape) 8.268 x 5.827", A5 (Portrait) 5.827 x 8.268", A3 (Landscape) 16.535 x 11.693", A3 (Portrait) 11.693 x 16.535", B5 (Landscape) 9.843 x 6.929", B5 (Portrait) 6.929 x 9.843", B4 (Landscape) 13.898 x 9.843", B4 (Portrait) 9.843 x 13.898".
- Selecting a tile fills a **details pane** on the right with the name as a heading, **Description:** (e.g. "Letter Landscape"), **Page Size:** (e.g. 11 x 8.5"), **Sheet Size:** (11 x 8.5"), a link **"Find updates on Office.com"** and a drawn preview of the page. OK applies the size like the Size gallery rows do.

## 8. Custom Margins dialog (Margins > "Custom Margins...")

Titled **Layout Guides** (so the same dialog as Guides > Layout guides), title-bar buttons **?** and close, three tabs: **Margin Guides** (selected), **Grid Guides**, **Baseline Guides**. On the Margin Guides tab:

- Group **Master Pages**: check box **Two-page master** (unchecked by default).
- Group **Margin Guides**: spin boxes **Left:**, **Right:**, **Top:**, **Bottom:** (in this order; `0.5"` by default for a new blank; displayed with the inch mark). With **Two-page master** ticked the first two labels become **Inside:** and **Outside:** (the preview then shows two facing pages).
- Group **Preview** at the right: the page (or the facing pair) with the margin rectangle in teal.
- **OK** (default) and **Cancel** under the tab area.

Focus starts on the Two-page master check box; Tab goes to the first spin box. Typing `1.5` means 1.5 inches. OK writes the four values to the master page (spec 4Q.6). Ticking Two-page master adds a second master page (spec 4Q.6). (The Move command of the Pages group is dimmed with one page and enabled with two.)

## 9. Behaviour summary (for the KernPage implementation)

- Margins gallery/dialog: only the four master-page margins change; no object moves.
- Orientation and Size: the page rectangle changes around a fixed page centre, so objects shift by half the size change; shapes that no longer touch the page are parked on the scratch area (spec 4Q.4/4Q.5). No dialog or warning appears when this happens.
- Units: the UI shows inches (with `"`), the file stores EMU; COM reports points (72 per inch).
- Gallery contents depend on the document: Size shows "Current" first for custom sizes and "(Landscape)" rows for landscape documents; Margins shows "Custom Margins" as a fifth row when no preset matches.
