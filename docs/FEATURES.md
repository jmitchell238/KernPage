# KernPage feature catalogue

This is the authoritative list of what KernPage must do. Goal: **cover the features people use in Publisher 2021 and keep `.pub` files round-tripping.** Each feature is one row below, and every row that is not done has its own issue (bead) in the project tracker, so progress is tracked one feature at a time. Online features that depend on a Microsoft service get open replacements (see [ROADMAP.md](../ROADMAP.md), Phase 2); the only feature we do not plan is Microsoft account sign-in (and product activation, which a free app does not need).

*Last updated: 2026-11-25.*

## Sources

The catalogue merges these research notes, removing duplicates by meaning (a handful of near-duplicate rows were merged and are noted in the surviving row):

- [object model and feature inventory](research/publisher-2021-object-model.md), about 400 features.
- [User interface inventory](research/publisher-2021-ui.md): ribbon groups, File screen pages, contextual tabs, right-click menus (`RIB-*`, `BS-*`, `UI-*`, `CTX-*`, `DLG-*`, `KEY-*` rows).
- [Competitor research](research/competitors.md) and [Launcher and add-ons plan](research/launcher-and-addons.md): the "Beyond Publisher" section.
- [`.pub` format specification](spec/PUB-FORMAT-SPEC.md) and the current code, to judge honestly what is already done.

## How to read the tables

| Column | Meaning |
|---|---|
| ID | Stable identifier (the prefix tells the area family) |
| Feature | The feature in plain words |
| Publisher behaviour | What Publisher 2021 does, in our own words |
| Read | KernPage reads and shows it from `.pub` files |
| Save | KernPage writes it back to `.pub` so Publisher 2021 opens the file without repair |
| UI | KernPage has the same control, dialog or menu as Publisher 2021 |
| Status | Overall status |
| Bead | The issue that tracks the remaining work (`bd show <id>`) |

Marks (same as the ROADMAP): ✅ done and checked against real Publisher 2021; 🟡 partly done; 🔬 needs research first (how Publisher stores it in the file; the research rules in CONTRIBUTING apply); 🛠️ ready to build, the spec already describes it; ⬜ not started; — not applicable or not planned. `[ONLINE]` rows were tied to a Microsoft service in Publisher and get an open replacement.

Status is judged from the code and the ROADMAP. A row is ✅ only when all of its applicable Read, Save and UI parts are done and verified. When in doubt the row is not ✅.

## Counts

**742 catalogued features** (plus 4 rows marked not planned). Done: 48. Partly done: 55. Needs research: 383. Ready to build: 0. Not started: 256. Rough overall progress: 10% (done rows plus half credit for partly done rows, out of 742).

| Area | Total | ✅ | 🟡 | 🔬 | 🛠️ | ⬜ | — | Epic |
|---|---|---|---|---|---|---|---|---|
| [Files and save](#files-and-save) | 12 |  | 4 | 8 |  |  |  | pw-usv |
| [Print and prepress](#print-and-prepress) | 59 |  | 2 |  |  | 57 |  | pw-85q |
| [PDF and export](#pdf-and-export) | 15 | 1 |  |  |  | 14 |  | pw-og4 |
| [Pages, masters, sections and guides](#pages-masters-sections-and-guides) | 47 | 7 | 10 | 30 |  |  |  | pw-2qn |
| [Text and typography](#text-and-typography) | 60 | 9 | 3 | 48 |  |  |  | pw-9n1 |
| [Styles](#styles) | 11 |  |  | 11 |  |  |  | pw-vlci |
| [Text boxes and story flow](#text-boxes-and-story-flow) | 14 | 3 | 2 | 9 |  |  |  | pw-r8ph |
| [Shapes and drawing](#shapes-and-drawing) | 41 | 8 | 4 | 29 |  |  |  | pw-3yvm |
| [Fill, line and effects](#fill-line-and-effects) | 31 | 4 |  | 27 |  |  |  | pw-506i |
| [Text Effects](#text-effects) | 6 |  |  | 6 |  |  |  | pw-k8a |
| [Pictures](#pictures) | 25 | 2 | 1 | 22 |  |  |  | pw-shtg |
| [Tables](#tables) | 24 | 6 | 3 | 15 |  |  |  | pw-ysrc |
| [Colour](#colour) | 21 |  | 1 | 20 |  |  |  | pw-lsps |
| [Templates, content blocks and business information](#templates-content-blocks-and-business-information) | 29 |  |  | 28 |  | 1 |  | pw-5a8b |
| [Mail, catalogue and email merge](#mail-catalogue-and-email-merge) | 49 |  |  | 49 |  |  |  | pw-kiry |
| [Web and forms](#web-and-forms) | 50 |  |  | 50 |  |  |  | pw-x4zi |
| [Review and proofing](#review-and-proofing) | 22 |  |  | 1 |  | 21 |  | pw-pe40 |
| [View and user interface](#view-and-user-interface) | 149 | 6 | 39 | 11 |  | 92 | 1 | pw-h12i |
| [Options and app](#options-and-app) | 23 | 2 |  |  |  | 21 |  | pw-y8jq |
| [Macros, add-ins and automation](#macros-add-ins-and-automation) | 9 |  |  |  |  | 9 |  | pw-oe5a |
| [Connected features](#connected-features) | 18 |  |  |  |  | 15 | 3 | pw-ogx9 |
| [Beyond Publisher](#beyond-publisher) | 14 |  |  |  |  | 14 |  | pw-01d6 |
| [Launcher, updater and add-ons](#launcher-updater-and-add-ons) | 12 |  | 1 |  |  | 11 |  | pw-hg18 |
| [Inventory gaps (research)](#inventory-gaps-research) | 10 |  |  | 10 |  |  |  | pw-xeey |
| **All** | 763 | 34 | 70 | 396 | 0 | 259 | 4 | |

## How to contribute

1. Pick a row that is 🔬 or 🛠️ and run `bd show <bead id>`; mark it in progress with `bd update <id> --status in_progress`.
2. For 🔬 rows, first observe how Publisher 2021 stores the feature, using **only files you made yourself**, and write the result into the [spec](spec/PUB-FORMAT-SPEC.md) in your own words. Never copy or translate Microsoft code, icons, artwork or templates.
3. Then build it: read and show it, save it, add the UI exactly where and how Publisher 2021 has it, and make PDF output match.
4. A row becomes ✅ only when it matches Publisher 2021 (same UI location and wording, same result) and a file with the feature opens in real Publisher 2021 after saving. Update the row and close the bead.
5. Found a missing feature? Add a row with a new ID in the right area and open a bead for it.

## Files and save

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| FILE-NEW | Create a new publication, blank or from a template | Create a new publication, blank or from a template | 🔬 | 🔬 | ⬜ | 🔬 | pw-cci |
| FILE-OPEN | Open a publication | Open a publication (several versions) *Note: Opens 2003-2021 files; very old and some extra settings not understood.* | 🟡 | — | ✅ | 🟡 | pw-dfu |
| FILE-SAVE | Save and Save As in the publication format | Save and Save As in the publication format *Note: Saves the edits listed as done in these rows; pictures, tables and masters not yet.* | — | 🟡 | ✅ | 🟡 | pw-z37 |
| FILE-KERN | Native KernPage format (.kern) | KernPage's own open format: new publications save as .kern, an opened .pub saves back to .pub, Save As offers both, and a .pub save that would leave edits out offers a .kern copy. Not a Publisher feature. | — | ✅ | ✅ | ✅ | pw-6a11 |
| FILE-SAVE-OLD | Save as Publisher 98 / 2000 formats | Save as Publisher 98 / 2000 formats | 🔬 | 🔬 | ⬜ | 🔬 | pw-w8x |
| FILE-SAVE-TEXT | Save as RTF, plain text or Unicode text | Save as RTF, plain text or Unicode text | 🔬 | 🔬 | ⬜ | 🔬 | pw-02k |
| FILE-CLOSE | Close with prompt, save or discard | Close with prompt, save or discard | 🔬 | 🔬 | ⬜ | 🔬 | pw-3ip |
| FILE-WORD-IMPORT | Create a publication from a Word document | Create a publication from a Word document | 🔬 | 🔬 | ⬜ | 🔬 | pw-iyn |
| FILE-PROPERTIES | Document properties | Document properties: general, summary (title, author, keywords...), statistics, custom, contents | 🔬 | 🔬 | ⬜ | 🔬 | pw-5sb |
| FILE-PROPERTIES-CUSTOM | Custom properties with types and link to content | Custom properties with types and link to content | 🔬 | 🔬 | ⬜ | 🔬 | pw-12g |
| FILE-SAVE-EDITS | Save every kind of edit back to .pub | Edits to text, positions, shapes, pictures and pages are written back so Publisher 2021 opens the result without repair. *Note: Umbrella for the per-feature Save column.* | 🟡 | 🟡 | — | 🟡 | pw-v32.3 |
| FILE-READ-OLD | Open Publisher 98 and 2000 files | Read the older generation-300 file layout. | 🔬 | 🔬 | — | 🔬 | pw-2dr.3 |
| FILE-READ-2013-2021 | Keep and understand settings added by 2013 to 2021 | Files saved by 2013-2021 open fine; extra settings are kept but not all understood. | 🟡 | 🟡 | — | 🟡 | pw-v31 |

## Print and prepress

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| PRT-PRINT | Print with copies, page range and collation | Print with copies, page range and collation *Note: File > Print sends the pages straight to the chosen printer (copies, all/current/custom pages such as 1-3, 5, collated or not; page drawn by Windows' own PDF renderer at the printer's resolution). "Open as PDF instead" keeps the PDF viewer route. Page drawing checked by rendering to a PNG at 300 and 600 dpi; not yet checked on a physical printer; duplex, paper size and colour mode are separate rows.* | — | — | 🟡 | 🟡 | pw-2sf |
| PRT-PRINTER-SELECT | Choose a printer and see its capabilities | Choose a printer and see its capabilities *Note: The Print page lists the installed printers with the default preselected, and Printer Properties opens the printer's own dialog (paper, quality, duplex). KernPage does not yet show the capabilities itself.* | — | — | 🟡 | 🟡 | pw-xw3 |
| PRT-DUPLEX | Duplex capability and back-side options | Duplex capability and back-side options (Also covers: PRT-DUPLEX-WIZARD.) | — | — | ⬜ | ⬜ | pw-9vy |
| PRT-PAPER | Paper size, source, orientation, printable area | Paper size, source, orientation, printable area | — | — | ⬜ | ⬜ | pw-7ts |
| PRT-COLOR-MODE | Print composite RGB, CMYK, greyscale or separations | Print composite RGB, CMYK, greyscale or separations | — | — | ⬜ | ⬜ | pw-vcy |
| PRT-STYLE-ONE | One page per sheet | One page per sheet | — | — | ⬜ | ⬜ | pw-1gf |
| PRT-STYLE-TILE | Tile large pages across sheets | Tile large pages across sheets | — | — | ⬜ | ⬜ | pw-xa0 |
| PRT-STYLE-COPIES | Multiple copies per sheet | Multiple copies per sheet | — | — | ⬜ | ⬜ | pw-3qm |
| PRT-STYLE-MULTI-PAGES | Multiple pages per sheet | Multiple pages per sheet | — | — | ⬜ | ⬜ | pw-ng2 |
| PRT-BOOKLET | Booklet printing | Booklet printing (side fold, top fold) | — | — | ⬜ | ⬜ | pw-b4o |
| PRT-FOLD | Half-fold and quarter-fold printing | Half-fold and quarter-fold printing | — | — | ⬜ | ⬜ | pw-rfx |
| PRT-ENVELOPE | Envelope printing options | Envelope printing options | — | — | ⬜ | ⬜ | pw-fyd |
| PRT-PREVIEW | Print preview | Print preview | — | — | ⬜ | ⬜ | pw-wvm |
| PRT-BACKGROUNDS | Choose whether to print page backgrounds | Choose whether to print page backgrounds | — | — | ⬜ | ⬜ | pw-duc |
| PRT-LINE-BY-LINE | Print line by line | Print line by line (troubleshooting) | — | — | ⬜ | ⬜ | pw-fh0 |
| PRT-TROUBLESHOOT | Print troubleshooter prompt | Print troubleshooter prompt | — | — | ⬜ | ⬜ | pw-95h |
| PRT-BW-MODE | Per-object black-and-white printing mode | Per-object black-and-white printing mode | — | — | ⬜ | ⬜ | pw-11v |
| PRT-MARKS-CROP | Crop marks | Crop marks | — | — | ⬜ | ⬜ | pw-r9f |
| PRT-MARKS-REG | Registration marks | Registration marks | — | — | ⬜ | ⬜ | pw-2aj |
| PRT-MARKS-BLEED | Bleed marks and allow bleeds | Bleed marks and allow bleeds | — | — | ⬜ | ⬜ | pw-g2d |
| PRT-MARKS-INFO | Job information, density bars, colour bars | Job information, density bars, colour bars | — | — | ⬜ | ⬜ | pw-adk |
| PRT-FLIP | Print mirrored | Print mirrored (horizontal or vertical) | — | — | ⬜ | ⬜ | pw-ke6 |
| PRT-NEGATIVE | Print as negative | Print as negative | — | — | ⬜ | ⬜ | pw-xlt |
| PRT-FONTS-PUB-ONLY | Use only publication fonts when printing | Use only publication fonts when printing | — | — | ⬜ | ⬜ | pw-9y7 |
| PRT-GFX-RES | Graphics resolution | Graphics resolution: full, low, none | — | — | ⬜ | ⬜ | pw-u04 |
| PRT-RESOLUTION | Output resolution | Output resolution | — | — | ⬜ | ⬜ | pw-hp3 |
| PRT-HALFTONE | Custom halftone | Custom halftone | — | — | ⬜ | ⬜ | pw-qyk |
| PRT-PS | PostScript printer options and detection | PostScript printer options and detection | — | — | ⬜ | ⬜ | pw-cpa |
| PRT-MANUAL-FEED | Manual feed alignment and direction, page rotation | Manual feed alignment and direction, page rotation | — | — | ⬜ | ⬜ | pw-xs7 |
| PRT-INKS-SELECT | Choose inks to print | Choose inks to print; convert spot to process | — | — | ⬜ | ⬜ | pw-4zy |
| PRT-BLANK-PLATES | Print blank plates | Print blank plates | — | — | ⬜ | ⬜ | pw-6sr |
| PRT-CMYK-DEFAULT | Print CMYK by default | Print CMYK by default | — | — | ⬜ | ⬜ | pw-bz5 |
| PRT-PLATE-SCREEN | Per-plate screen angle and frequency | Per-plate screen angle and frequency | — | — | ⬜ | ⬜ | pw-d58 |
| PRT-FONT-LICENSE | Font embedding permission check and font source report | Font embedding permission check and font source report | — | — | ⬜ | ⬜ | pw-3r8 |
| PACK-WIZARD | Package for Printing wizard | Package for Printing wizard: choose a folder, include fonts and linked graphics, finish | — | — | ⬜ | ⬜ | pw-k2a |
| PACK-EMBED-FONTS | Embed TrueType fonts in the packed file | Embed TrueType fonts in the packed file | — | — | ⬜ | ⬜ | pw-v0v |
| PACK-LINKED | Include linked graphics and create links for embedded ones | Include linked graphics and create links for embedded ones | — | — | ⬜ | ⬜ | pw-7jg |
| PACK-PROOF | Print a composite proof after packing | Print a composite proof after packing | — | — | ⬜ | ⬜ | pw-gna |
| PACK-FONT-WARN | Warn about fonts that cannot embed or are preview-only | Warn about fonts that cannot embed or are preview-only | — | — | ⬜ | ⬜ | pw-2d5 |
| FONT-EMBED-SAVE | Embed TrueType fonts when saving | Embed TrueType fonts when saving; skip common system fonts; subset | — | — | ⬜ | ⬜ | pw-097 |
| FONT-EMBED-LIST | List which fonts are embedded or unavailable | List which fonts are embedded or unavailable, load fonts, print the list | — | — | ⬜ | ⬜ | pw-e4s |
| FONT-SUBSTITUTE | Substitute a missing font temporarily or permanently | Substitute a missing font temporarily or permanently | — | — | ⬜ | ⬜ | pw-l0d |
| PRT-TO-FILE | Print to file | Print to file | — | — | ⬜ | ⬜ | pw-3op |
| PRT-FIND-PRINTER | Find a printer | Find a printer; advanced printer setup | — | — | ⬜ | ⬜ | pw-t8o |
| PRT-STATUS | Show printer status, type, location, comment, driver | Show printer status, type, location, comment, driver | — | — | ⬜ | ⬜ | pw-6a4 |
| PRT-MERGE-TEST | Print a single publication of a merge as a test | Print a single publication of a merge as a test | — | — | ⬜ | ⬜ | pw-553 |
| PRT-MERGE-SKIP-EMPTY | Do not print lines that contain only empty merge fields | Do not print lines that contain only empty merge fields | — | — | ⬜ | ⬜ | pw-26p |
| PRT-TILE-SINGLE | Print a single tile of a large page by row and column | Print a single tile of a large page by row and column | — | — | ⬜ | ⬜ | pw-658 |
| PRT-TILE-OVERLAP | Horizontal and vertical tile overlap | Horizontal and vertical tile overlap; start row and column; gaps | — | — | ⬜ | ⬜ | pw-211 |
| PRT-SHOW-PAPER | Show how to insert paper, and show paper after printing | Show how to insert paper, and show paper after printing | — | — | ⬜ | ⬜ | pw-04z |
| PRT-SECOND-SIDE | Prompt to reload and print the second side | Prompt to reload and print the second side | — | — | ⬜ | ⬜ | pw-h4x |
| PRT-FEED-DIRECTION | Bottom, top, wide feed direction for the printer | Bottom, top, wide feed direction for the printer | — | — | ⬜ | ⬜ | pw-9z5 |
| PRT-ENVELOPE-FEED | Envelope feed orientation and placement, auto envelope sizes | Envelope feed orientation and placement, auto envelope sizes | — | — | ⬜ | ⬜ | pw-gdz |
| PRT-TRAP-AUTO | Automatic trapping with thresholds | Automatic trapping with thresholds | — | — | ⬜ | ⬜ | pw-v3s |
| PRT-TRAP-THRESHOLDS | Trap rules | Trap rules: shared ink, centre-line luminance ratio, rich black keepaway | — | — | ⬜ | ⬜ | pw-r1u |
| PRT-TRAP-PER-OBJECT | Per-object trap type, width, placement | Per-object trap type, width, placement; trap details | — | — | ⬜ | ⬜ | pw-g3j |
| PRT-TRAP-SPOT | Spot colour trapping options and trap white as a colour | Spot colour trapping options and trap white as a colour | — | — | ⬜ | ⬜ | pw-uva |
| PRT-OVERPRINT | Overprint settings for text below a size | Overprint settings for text below a size, lines, fills, imported pictures | — | — | ⬜ | ⬜ | pw-xew |
| PRT-REG-RESET | Reset all registration settings | Reset all registration settings | — | — | ⬜ | ⬜ | pw-ye5 |

## PDF and export

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| FILE-EXPORT-PDF | Export PDF with intent | Export PDF with intent (minimum, standard, print, commercial) and page range (Also covers: FILE-PDF-RANGE.) | — | — | ✅ | ✅ | — |
| FILE-EXPORT-XPS | Export XPS | Export XPS | — | — | ⬜ | ⬜ | pw-awv |
| FILE-EXPORT-PIC-PAGE | Save a page as a picture | Save a page as a picture | — | — | ⬜ | ⬜ | pw-a1n |
| FILE-EXPORT-PIC-SHAPE | Save a selected object as a picture | Save a selected object as a picture | — | — | ⬜ | ⬜ | pw-cvj |
| FILE-EXPORT-HTMLMAIL | Export a page as email HTML | Export a page as email HTML | — | — | ⬜ | ⬜ | pw-21e |
| FILE-PUBLISH-INTENT | Publish pane that picks intent | Publish pane that picks intent (print, commercial, web, e-mail) and shows fixes needed | — | — | ⬜ | ⬜ | pw-cm2 |
| FILE-PUBLISH-DOWNSAMPLE | Downsample colour and greyscale pictures to a target dpi above a threshold | Downsample colour and greyscale pictures to a target dpi above a threshold | — | — | ⬜ | ⬜ | pw-5bj |
| FILE-PUBLISH-DC-WARN | Choose which Design Checker checks | Choose which Design Checker checks (transparency, colour) run at publish | — | — | ⬜ | ⬜ | pw-d13 |
| FILE-PDFA | PDF/A compliant output | PDF/A compliant output | — | — | ⬜ | ⬜ | pw-eny |
| FILE-PDF-ENCRYPT | Encrypt the PDF with a password | Encrypt the PDF with a password | — | — | ⬜ | ⬜ | pw-8t7 |
| FILE-PDF-TAGS | Document structure tags for accessibility | Document structure tags for accessibility | — | — | ⬜ | ⬜ | pw-h91 |
| FILE-PDF-BITMAP-TEXT | Bitmap text when fonts may not embed | Bitmap text when fonts may not embed | — | — | ⬜ | ⬜ | pw-a1w |
| FILE-PDF-BOOKMARKS | Create bookmarks from headings | Create bookmarks from headings; include document properties | — | — | ⬜ | ⬜ | pw-xkp |
| FILE-PDF-OPTIMIZE | Optimise for image quality or file size | Optimise for image quality or file size | — | — | ⬜ | ⬜ | pw-ee1 |
| FILE-XPS-PERMS | Preserve restricted permissions in XPS | Preserve restricted permissions in XPS | — | — | ⬜ | ⬜ | pw-i26 |

## Pages, masters, sections and guides

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| PG-ADD | Insert blank, duplicate or starter pages before or after | Insert blank, duplicate or starter pages before or after | ✅ | ✅ | ✅ | ✅ | — |
| PG-DELETE | Delete a page | Delete a page | ✅ | ✅ | ✅ | ✅ | — |
| PG-DUPLICATE | Duplicate a page | Duplicate a page *Note: Pages with groups, tables or linked boxes not yet supported.* | ✅ | 🟡 | ✅ | 🟡 | pw-ryt |
| PG-MOVE | Move a page | Move a page | ✅ | ✅ | ✅ | ✅ | — |
| PG-NAVIGATE | Go to page, active page | Go to page, active page | ✅ | — | ✅ | ✅ | — |
| PG-ID | Stable page identifier | Stable page identifier | 🔬 | 🔬 | ⬜ | 🔬 | pw-5y6 |
| PG-NAME | Page name | Page name | 🔬 | 🔬 | ⬜ | 🔬 | pw-5qk |
| PG-SPREAD | Facing pages and reader spreads | Facing pages and reader spreads | 🔬 | 🔬 | ⬜ | 🔬 | pw-8qe |
| PG-SCRATCH | Scratch area beside pages | Scratch area beside pages | 🔬 | 🔬 | ⬜ | 🔬 | pw-u53 |
| PG-MASTER-ADD | Create one-page or two-page master pages | Create one-page or two-page master pages *Note: Master pages are read and drawn behind the pages (master A and its text boxes); a second master is not yet researched. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-s81 |
| PG-MASTER-APPLY | Apply a master to a page | Apply a master to a page *Note: Each page reads its master reference (item 13) and draws that master; only master A was seen in a saved file. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-3l8 |
| PG-MASTER-IGNORE | Hide master objects on one page | Hide master objects on one page *Note: Ignore Master Page (page item 7) is read and honoured on screen, in thumbnails and in PDF. Displayed, not editable yet.* | ✅ | 🔬 | ⬜ | 🟡 | pw-pw0 |
| PG-MASTER-EDIT | Edit master shapes | Edit master shapes (page numbers, headers) *Note: Master text boxes are drawn on every page that uses the master, but cannot be selected or edited on a page. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-zo8 |
| PG-HEADER-FOOTER | Header and footer areas | Header and footer areas *Note: Headers and footers are ordinary master text boxes; they are drawn like any master shape. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-9jr |
| PG-BACKGROUND | Page background fill or picture | Page background fill or picture | 🔬 | 🔬 | ⬜ | 🔬 | pw-bdv |
| PG-SECTIONS | Section breaks with independent numbering | Section breaks with independent numbering *Note: Section records (start page, first number, format) are read and drive the displayed page numbers. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-z0q |
| PG-NUM-START | Start number and continue from previous | Start number and continue from previous *Note: Start number and continue-from-previous are read and shown. Displayed, not editable yet.* | ✅ | 🔬 | ⬜ | 🟡 | pw-l7v |
| PG-NUM-FORMAT | Number format | Number format (Arabic, Roman, letters, ordinals, regional) *Note: Arabic, upper/lower Roman and upper/lower letters are shown (letters past Z repeat: AA, BB, assumed); other formats fall back to Arabic. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-825 |
| PG-NUM-FIRSTPAGE | Show header and footer on first page | Show header and footer on first page *Note: The flag is read but not yet applied when drawing. Displayed, not editable yet.* | 🟡 | 🔬 | ⬜ | 🟡 | pw-43q |
| PG-SIZE | Page size presets and custom size | Page size presets and custom size | ✅ | ✅ | ✅ | ✅ | pw-a8r |
| PG-ORIENT | Portrait or landscape | Portrait or landscape | ✅ | ✅ | ✅ | ✅ | pw-u2n |
| PG-LAYOUT-TYPE | Publication layout | Publication layout (card, poster, banner, booklet, book and so on) | 🔬 | 🔬 | ⬜ | 🔬 | pw-a5l |
| PG-MARGINS | Margin guides | Margin guides | ✅ | ✅ | ✅ | ✅ | pw-dev |
| PG-MIRROR | Mirror guides for facing pages | Mirror guides for facing pages | 🔬 | 🔬 | ⬜ | 🔬 | pw-fxk |
| PG-GRID | Column and row guides with gutters | Column and row guides with gutters | 🔬 | 🔬 | ⬜ | 🔬 | pw-10a |
| PG-BASELINE | Baseline grid | Baseline grid | 🔬 | 🔬 | ⬜ | 🔬 | pw-m5f |
| PG-RULER-GUIDES | Draggable ruler guides | Draggable ruler guides | 🔬 | 🔬 | ⬜ | 🔬 | pw-1e2 |
| PG-LABEL-SHEETS | Label sheet definitions | Label sheet definitions (size, margins, gaps) | 🔬 | 🔬 | ⬜ | 🔬 | pw-4o0 |
| PG-SHEET-LAYOUT | Multiple pages per sheet with gaps and margins | Multiple pages per sheet with gaps and margins | 🔬 | 🔬 | ⬜ | 🔬 | pw-egc |
| PG-SIZE-BACKGROUND | Page sizes that carry a background image | Page sizes that carry a background image | 🔬 | 🔬 | ⬜ | 🔬 | pw-euf |
| PG-BLEED | Bleed area | Bleed area (allow bleeds) | 🔬 | 🔬 | ⬜ | 🔬 | pw-xon |
| PG-REPEAT-NEWSLETTER | Continued-on and continued-from notes | Continued-on and continued-from notes | 🔬 | 🔬 | ⬜ | 🔬 | pw-k4x |
| PG-INSERT-OPTIONS | Insert pages | Insert pages: before, after, between; blank, one text box each, duplicate objects; choose master | 🔬 | 🔬 | ⬜ | 🔬 | pw-tcu |
| PG-INSERT-SPREAD | Insert left and right page types independently | Insert left and right page types independently | 🔬 | 🔬 | ⬜ | 🔬 | pw-qfj |
| PG-INSERT-NAVBAR | Choose whether the new page links into navigation bars | Choose whether the new page links into navigation bars | 🔬 | 🔬 | ⬜ | 🔬 | pw-84x |
| PG-DELETE-SIDE | Delete only the left or right page of a spread | Delete only the left or right page of a spread | 🔬 | 🔬 | ⬜ | 🔬 | pw-4mo |
| PG-MOVE-SIDE | Move only left or right page | Move only left or right page; before or after another page | 🔬 | 🔬 | ⬜ | 🔬 | pw-607 |
| PG-TITLE-SIDES | Page titles per side of a folded layout | Page titles per side of a folded layout | 🔬 | 🔬 | ⬜ | 🔬 | pw-g9x |
| PG-NUM-INSERT | Insert page numbers at a chosen position and alignment | Insert page numbers at a chosen position and alignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-ywu |
| PG-MASTER-ID | Master page has a one-character id and description | Master page has a one-character id and description | 🔬 | 🔬 | ⬜ | 🔬 | pw-jvr |
| PG-MASTER-DUPLICATE | Duplicate and rename a master page | Duplicate and rename a master page | 🔬 | 🔬 | ⬜ | 🔬 | pw-zl4 |
| PG-MASTER-RANGE | Apply a master to all, current or a page range | Apply a master to all, current or a page range | 🔬 | 🔬 | ⬜ | 🔬 | pw-9op |
| PG-MASTER-IGNORE-SIDE | Ignore master for the left or right page only | Ignore master for the left or right page only | 🔬 | 🔬 | ⬜ | 🔬 | pw-4t0 |
| PG-GRID-CENTER | Add a centre guide between columns and rows | Add a centre guide between columns and rows; reset even widths | 🔬 | 🔬 | ⬜ | 🔬 | pw-687 |
| PG-RULER-NUMERIC | Enter ruler guide position numerically | Enter ruler guide position numerically; clear one or all | 🔬 | 🔬 | ⬜ | 🔬 | pw-ahi |
| PG-SIZE-SAVE | Save a custom page size under a name | Save a custom page size under a name | 🔬 | 🔬 | ⬜ | 🔬 | pw-4r3 |
| PG-SIZE-PRESETS | Preset page-size pane | Preset page-size pane | 🔬 | 🔬 | ⬜ | 🔬 | pw-00d |

## Text and typography

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| TXT-EDIT | Type, insert, delete, cut, copy, paste text | Type, insert, delete, cut, copy, paste text | ✅ | ✅ | ✅ | ✅ | — |
| TXT-SELECT-UNITS | Select by character, word, line, sentence, paragraph, story | Select by character, word, line, sentence, paragraph, story | 🔬 | 🔬 | ⬜ | 🔬 | pw-6m9 |
| TXT-SYMBOL | Insert symbols and special characters | Insert symbols and special characters (Also covers: TXT-SYMBOL-RECENT.) | 🔬 | 🔬 | ⬜ | 🔬 | pw-6ln |
| TXT-DATETIME | Insert date and time | Insert date and time (updating or fixed) (Also covers: TXT-DATETIME-CAL.) | 🔬 | 🔬 | ⬜ | 🔬 | pw-mjt |
| TXT-PAGENUM-FIELD | Insert page number fields | Insert page number fields (current, next, previous) | 🔬 | 🔬 | ⬜ | 🔬 | pw-cwd |
| TXT-FIELDS | Unlink fields to plain text | Unlink fields to plain text | 🔬 | 🔬 | ⬜ | 🔬 | pw-305 |
| TXT-FONT-FACE | Font name | Font name | ✅ | ✅ | ✅ | ✅ | — |
| TXT-FONT-SIZE | Font size, grow and shrink | Font size, grow and shrink | ✅ | ✅ | ✅ | ✅ | — |
| TXT-BOLD-ITALIC | Bold and italic | Bold and italic | ✅ | ✅ | ✅ | ✅ | — |
| TXT-UNDERLINE | Underline with 18 styles | Underline with 18 styles *Note: Single underline only; the other styles are not done.* | ✅ | ✅ | 🟡 | 🟡 | pw-4m4 |
| TXT-STRIKE | Strikethrough | Strikethrough | 🔬 | 🔬 | ⬜ | 🔬 | pw-h9y |
| TXT-SCRIPT | Superscript and subscript | Superscript and subscript *Note: Read, write, draw (canvas and PDF) and Home buttons done and tested; saved files not yet opened in real Publisher; no Font dialog yet; draw offsets are measured approximations.* | 🟡 | 🟡 | 🟡 | 🟡 | pw-df5 |
| TXT-CASE | All caps and small caps | All caps and small caps | 🔬 | 🔬 | ⬜ | 🔬 | pw-8po |
| TXT-EFFECTS-LEGACY | Outline, shadow, emboss, engrave | Outline, shadow, emboss, engrave | 🔬 | 🔬 | ⬜ | 🔬 | pw-e56 |
| TXT-COLOR | Text colour | Text colour (scheme, RGB, CMYK, spot) | ✅ | ✅ | ✅ | ✅ | — |
| TXT-TRACKING | Character spacing presets and custom | Character spacing presets and custom | 🔬 | 🔬 | ⬜ | 🔬 | pw-piy |
| TXT-SCALE | Character width scaling | Character width scaling | 🔬 | 🔬 | ⬜ | 🔬 | pw-kb6 |
| TXT-KERN | Pair kerning with minimum size | Pair kerning with minimum size | 🔬 | 🔬 | ⬜ | 🔬 | pw-d5q |
| TXT-OT-LIGATURE | OpenType ligatures | OpenType ligatures | 🔬 | 🔬 | ⬜ | 🔬 | pw-cau |
| TXT-OT-NUMSTYLE | OpenType number styles | OpenType number styles | 🔬 | 🔬 | ⬜ | 🔬 | pw-buj |
| TXT-OT-STYLISTIC | Stylistic sets and alternates, swashes, contextual alternates | Stylistic sets and alternates, swashes, contextual alternates | 🔬 | 🔬 | ⬜ | 🔬 | pw-1rxs |
| TXT-FILL-TEXT | Text fill and outline | Text fill and outline (gradient and picture fills) | 🔬 | 🔬 | ⬜ | 🔬 | pw-pxmk |
| TXT-EFFECTS-MODERN | Text shadow, glow, reflection, 3-D on text | Text shadow, glow, reflection, 3-D on text | 🔬 | 🔬 | ⬜ | 🔬 | pw-xqa5 |
| TXT-CLEAR-CHAR | Clear character formatting | Clear character formatting *Note: Done for character formatting; whether 2021 also resets paragraph formatting is unconfirmed.* | ✅ | ✅ | 🟡 | 🟡 | pw-x231 |
| TXT-COMPLEX-SCRIPT | Complex-script bold, italic, size, kashida, diacritic colour | Complex-script bold, italic, size, kashida, diacritic colour | 🔬 | 🔬 | ⬜ | 🔬 | pw-v053 |
| TXT-SCRIPT-FONTS | Per-script font assignment | Per-script font assignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-ws9e |
| TXT-ALIGN | Left, centre, right, justify, distribute | Left, centre, right, justify, distribute | ✅ | ✅ | ✅ | ✅ | — |
| TXT-ALIGN-EASTASIAN | East Asian and Arabic alignment options | East Asian and Arabic alignment options | 🔬 | 🔬 | ⬜ | 🔬 | pw-5rgb |
| TXT-INDENT | Left, right and first-line indents | Left, right and first-line indents | 🔬 | 🔬 | ⬜ | 🔬 | pw-2l4u |
| TXT-INDENT-CHAR | Indents in characters | Indents in characters | 🔬 | 🔬 | ⬜ | 🔬 | pw-6lny |
| TXT-SPACING | Space before and after paragraphs | Space before and after paragraphs | ✅ | ✅ | ✅ | ✅ | — |
| TXT-LINESPACING | Line spacing single, 1.5, double, exact, multiple | Line spacing single, 1.5, double, exact, multiple | ✅ | ✅ | ✅ | ✅ | — |
| TXT-TABS | Tab stops with alignment and leaders | Tab stops with alignment and leaders | 🔬 | 🔬 | ⬜ | 🔬 | pw-4pcq |
| TXT-TAB-DEFAULT | Default tab stop distance | Default tab stop distance | 🔬 | 🔬 | ⬜ | 🔬 | pw-rtsr |
| TXT-BULLET | Bullets with custom character and font | Bullets with custom character and font | ✅ | ✅ | ✅ | ✅ | pw-cibs |
| TXT-NUMBERING | Numbered lists in many numbering systems | Numbered lists in many numbering systems | 🔬 | 🔬 | ⬜ | 🔬 | pw-g5i8 |
| TXT-NUM-START | Number start value | Number start value | 🔬 | 🔬 | ⬜ | 🔬 | pw-2grf |
| TXT-NUM-SEPARATOR | Separator style after number | Separator style after number | 🔬 | 🔬 | ⬜ | 🔬 | pw-vg6d |
| TXT-LIST-INDENT | List indent | List indent | 🔬 | 🔬 | ⬜ | 🔬 | pw-4f86 |
| TXT-KEEP-TOGETHER | Keep lines together | Keep lines together | 🔬 | 🔬 | ⬜ | 🔬 | pw-bdhh |
| TXT-KEEP-NEXT | Keep with next paragraph | Keep with next paragraph | 🔬 | 🔬 | ⬜ | 🔬 | pw-p3fi |
| TXT-WIDOW | Widow and orphan control | Widow and orphan control | 🔬 | 🔬 | ⬜ | 🔬 | pw-djy0 |
| TXT-START-NEXT-BOX | Start paragraph in next text box | Start paragraph in next text box | 🔬 | 🔬 | ⬜ | 🔬 | pw-f2ys |
| TXT-BASELINE-LOCK | Align paragraph to baseline grid | Align paragraph to baseline grid | 🔬 | 🔬 | ⬜ | 🔬 | pw-xvtn |
| TXT-DIRECTION | Paragraph direction left to right or right to left | Paragraph direction left to right or right to left | 🔬 | 🔬 | ⬜ | 🔬 | pw-gjg3 |
| TXT-CLEAR-PARA | Clear paragraph formatting | Clear paragraph formatting | 🔬 | 🔬 | ⬜ | 🔬 | pw-xhmn |
| TXT-DROPCAP | Drop cap presets and custom | Drop cap presets and custom (lines, size, font, colour) | 🔬 | 🔬 | ⬜ | 🔬 | pw-tatp |
| TXT-PHONETIC | Phonetic guides over East Asian text | Phonetic guides over East Asian text | 🔬 | 🔬 | ⬜ | 🔬 | pw-cx4c |
| TXT-HORIZONTAL-IN-VERTICAL | Horizontal text inside vertical lines | Horizontal text inside vertical lines (tate-chu-yoko) | 🔬 | 🔬 | ⬜ | 🔬 | pw-ncnn |
| TXT-BARCODE | Insert a barcode | Insert a barcode | 🔬 | 🔬 | ⬜ | 🔬 | pw-da8k |
| TXT-RANGE-BOUNDS | Geometry of a text range | Geometry of a text range | 🔬 | 🔬 | ⬜ | 🔬 | pw-w81n |
| TXT-MAJORITY | Formatting shown for mixed selections | Formatting shown for mixed selections | 🔬 | 🔬 | ⬜ | 🔬 | pw-0qiu |
| TXT-INSERT-FILE | Insert a text file into a text box | Insert a text file into a text box | 🔬 | 🔬 | ⬜ | 🔬 | pw-vehk |
| TXT-RULES | Horizontal rules before and after paragraphs with thickness | Horizontal rules before and after paragraphs with thickness, colour, style, position | 🔬 | 🔬 | ⬜ | 🔬 | pw-v2h4 |
| TXT-EMPHASIS | Emphasis marks on East Asian text | Emphasis marks on East Asian text | 🔬 | 🔬 | ⬜ | 🔬 | pw-dchl |
| TXT-ENCLOSE | Enclose characters in shapes | Enclose characters in shapes | 🔬 | 🔬 | ⬜ | 🔬 | pw-rro9 |
| TXT-ASIAN-TYPO | Asian typography | Asian typography: line-break rules, hanging punctuation, compression, auto-spacing, alignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-c6yq |
| TXT-KASHIDA | Kashida justification ratio and kashida spacing | Kashida justification ratio and kashida spacing | 🔬 | 🔬 | ⬜ | 🔬 | pw-gi2f |
| TXT-CHINESE-CONVERT | Convert Traditional and Simplified Chinese with regional variants and terms | Convert Traditional and Simplified Chinese with regional variants and terms | 🔬 | 🔬 | ⬜ | 🔬 | pw-kxuo |
| TXT-CS-SEQUENCE | Complex-script sequence checking and type-and-replace | Complex-script sequence checking and type-and-replace | 🔬 | 🔬 | ⬜ | 🔬 | pw-kji3 |

## Styles

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| TXT-STYLE-APPLY | Apply a named style | Apply a named style | 🔬 | 🔬 | ⬜ | 🔬 | pw-zxk7 |
| TXT-STYLE-CREATE | Create, edit, delete styles with base and next style | Create, edit, delete styles with base and next style | 🔬 | 🔬 | ⬜ | 🔬 | pw-1zqn |
| TXT-STYLE-DESCR | Style description text | Style description text | 🔬 | 🔬 | ⬜ | 🔬 | pw-qde5 |
| FONT-SCHEME-APPLY | Font schemes | Font schemes: heading and body fonts per script, applied publication-wide | 🔬 | 🔬 | ⬜ | 🔬 | pw-f0el |
| FONT-SCHEME-CREATE | Create a custom font scheme | Create a custom font scheme (Latin, East Asian, complex script) | 🔬 | 🔬 | ⬜ | 🔬 | pw-9t11 |
| FONT-SCHEME-MANAGE | Duplicate and rename font schemes | Duplicate and rename font schemes | 🔬 | 🔬 | ⬜ | 🔬 | pw-xmzk |
| FONT-SCHEME-OPTIONS | Choose whether a scheme change updates custom styles | Choose whether a scheme change updates custom styles, overrides applied formatting and adjusts sizes | 🔬 | 🔬 | ⬜ | 🔬 | pw-ehho |
| TXT-STYLE-BYEXAMPLE | Create a style from the selection | Create a style from the selection | 🔬 | 🔬 | ⬜ | 🔬 | pw-o0d7 |
| TXT-STYLE-RENAME | Rename a style | Rename a style | 🔬 | 🔬 | ⬜ | 🔬 | pw-jjh2 |
| TXT-STYLE-REAPPLY | Choose between updating a style from the selection or returning the | Choose between updating a style from the selection or returning the selection to the style | 🔬 | 🔬 | ⬜ | 🔬 | pw-173b |
| TXT-STYLE-SYNC | Auto-synchronise formatting of synchronised objects | Auto-synchronise formatting of synchronised objects; prompt to update styles | 🔬 | 🔬 | ⬜ | 🔬 | pw-kpru |

## Text boxes and story flow

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| SHP-ADD-TEXTBOX | Draw a text box | Draw a text box | ✅ | ✅ | ✅ | ✅ | — |
| SHP-ADD-LABEL | Add a label | Add a label (auto-sized text) | 🔬 | 🔬 | ⬜ | 🔬 | pw-hxpz |
| SHP-OVERFLOW-LIST | Find objects excess to the page or text overflow | Find objects excess to the page or text overflow | 🔬 | 🔬 | ⬜ | 🔬 | pw-60cx |
| TB-MARGINS | Internal margins | Internal margins | 🔬 | 🔬 | ⬜ | 🔬 | pw-ws9v |
| TB-COLUMNS | Columns with spacing | Columns with spacing | ✅ | ✅ | ✅ | ✅ | — |
| TB-VALIGN | Vertical alignment | Vertical alignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-vghm |
| TB-ORIENT | Text direction | Text direction (horizontal, vertical, right to left) | 🔬 | 🔬 | ⬜ | 🔬 | pw-xg60 |
| TB-AUTOFIT | Shrink on overflow, best fit, grow to fit | Shrink on overflow, best fit, grow to fit | 🔬 | 🔬 | ⬜ | 🔬 | pw-azu3 |
| TB-LINK | Create link to next text box | Create link to next text box *Note: Linked boxes display and flow; creating and breaking links not yet.* | ✅ | 🔬 | ⬜ | 🟡 | pw-g0cd |
| TB-UNLINK | Break forward link | Break forward link | 🔬 | 🔬 | ⬜ | 🔬 | pw-v23b |
| TB-OVERFLOW | Detect text overflow | Detect text overflow | 🟡 | 🔬 | ⬜ | 🟡 | pw-pr0h |
| TB-CONTINUED | Continued-on and continued-from notes | Continued-on and continued-from notes | 🔬 | 🔬 | ⬜ | 🔬 | pw-ubrv |
| TB-STORY | Whole-story operations across linked boxes | Whole-story operations across linked boxes | ✅ | ✅ | ✅ | ✅ | — |
| TB-HAS-TEXT | Know if a frame has text | Know if a frame has text | 🔬 | 🔬 | ⬜ | 🔬 | pw-urz9 |

## Shapes and drawing

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| SHP-ADD-LINE | Draw a line | Draw a line *Note: Arrowheads and dashes are separate rows.* | ✅ | ✅ | ✅ | ✅ | — |
| SHP-ADD-AUTOSHAPE | Draw any AutoShape | Draw any AutoShape *Note: Rectangle and oval only; other AutoShapes need research.* | ✅ | 🟡 | 🟡 | 🟡 | pw-gqgc |
| SHP-AUTOSHAPE-ADJUST | Drag adjust handles on AutoShapes | Drag adjust handles on AutoShapes | 🔬 | 🔬 | ⬜ | 🔬 | pw-nxvv |
| SHP-ADD-CALLOUT | Callout shapes with leader line | Callout shapes with leader line | 🔬 | 🔬 | ⬜ | 🔬 | pw-huau |
| SHP-ADD-CONNECTOR | Connectors that follow shapes | Connectors that follow shapes | 🔬 | 🔬 | ⬜ | 🔬 | pw-lfi6 |
| SHP-ADD-CURVE | Draw a curve | Draw a curve | 🔬 | 🔬 | ⬜ | 🔬 | pw-29zw |
| SHP-ADD-POLYLINE | Draw a polyline or polygon | Draw a polyline or polygon | 🔬 | 🔬 | ⬜ | 🔬 | pw-uddg |
| SHP-FREEFORM | Build a freeform shape point by point | Build a freeform shape point by point | 🔬 | 🔬 | ⬜ | 🔬 | pw-a74l |
| SHP-EDIT-POINTS | Edit points | Edit points: add, delete, move, change segment or node type | 🔬 | 🔬 | ⬜ | 🔬 | pw-yu5e |
| SHP-ADD-OLE | Insert an OLE object | Insert an OLE object (embedded or linked) | 🔬 | 🔬 | ⬜ | 🔬 | pw-2mhr |
| SHP-OLE-VERBS | Edit or open an OLE object | Edit or open an OLE object | 🔬 | 🔬 | ⬜ | 🔬 | pw-slif |
| SHP-OLE-LINK-UPDATE | Update linked objects | Update linked objects | 🔬 | 🔬 | ⬜ | 🔬 | pw-n8zw |
| SHP-MOVE-SIZE | Position and size, with lock of aspect ratio | Position and size, with lock of aspect ratio | ✅ | ✅ | ✅ | ✅ | — |
| SHP-SCALE | Scale height or width relative to original or current | Scale height or width relative to original or current | 🔬 | 🔬 | ⬜ | 🔬 | pw-wv33 |
| SHP-ROTATE | Rotation to angle or by increments | Rotation to angle or by increments | ✅ | ✅ | ✅ | ✅ | — |
| SHP-FLIP | Flip horizontal or vertical | Flip horizontal or vertical *Note: Lines done; flipping other shapes needs research.* | ✅ | 🟡 | ✅ | 🟡 | pw-d0vv |
| SHP-ZORDER | Bring forward, send backward, to front, to back | Bring forward, send backward, to front, to back | ✅ | ✅ | ✅ | ✅ | — |
| SHP-ALIGN | Align to edges and centres, relative to page or selection | Align to edges and centres, relative to page or selection | ✅ | ✅ | ✅ | ✅ | — |
| SHP-DISTRIBUTE | Distribute horizontally or vertically | Distribute horizontally or vertically | ✅ | ✅ | ✅ | ✅ | — |
| SHP-GROUP | Group, ungroup, regroup | Group, ungroup, regroup *Note: Nested and rotated groups need research.* | ✅ | ✅ | ✅ | ✅ | — |
| SHP-GROUP-MEMBER | Select inside a group | Select inside a group | 🔬 | 🔬 | ⬜ | 🔬 | pw-t56l |
| SHP-CUT-COPY-PASTE | Cut, copy, paste and delete objects | Cut, copy, paste and delete objects | ✅ | 🟡 | 🟡 | 🟡 | pw-lmw2 |
| SHP-DUPLICATE | Duplicate with offset | Duplicate with offset | 🔬 | 🔬 | ⬜ | 🔬 | pw-vvhr |
| SHP-FORMAT-PAINTER | Copy formatting | Copy formatting (Pick Up, Apply) | 🔬 | 🔬 | ⬜ | 🔬 | pw-knlw |
| SHP-DEFAULTS | Set object defaults for new shapes | Set object defaults for new shapes | 🔬 | 🔬 | ⬜ | 🔬 | pw-c7v3 |
| SHP-SELECT-ALL | Select all objects on a page | Select all objects on a page | 🔬 | 🔬 | 🟡 | 🟡 | pw-1zvj |
| SHP-MOVE-TO-PAGE | Move object to another page | Move object to another page | 🔬 | 🔬 | ⬜ | 🔬 | pw-0hko |
| SHP-NAME | Name an object | Name an object | 🔬 | 🔬 | ⬜ | 🔬 | pw-ig86 |
| SHP-ALT-TEXT | Alternative text | Alternative text | 🔬 | 🔬 | ⬜ | 🔬 | pw-mvak |
| SHP-TAG | Custom tags on objects | Custom tags on objects | 🔬 | 🔬 | ⬜ | 🔬 | pw-ksae |
| SHP-INLINE | Anchor object inline in text | Anchor object inline in text (character, left, right) | 🔬 | 🔬 | ⬜ | 🔬 | pw-tpuc |
| SHP-LOCK | Lock aspect ratio | Lock aspect ratio | 🔬 | 🔬 | ⬜ | 🔬 | pw-3oc2 |
| SHP-CANVAS | Canvas arrangement for merged catalog items | Canvas arrangement for merged catalog items | 🔬 | 🔬 | ⬜ | 🔬 | pw-wmgg |
| FMT-WRAP-TYPE | Text wrap type | Text wrap type *Note: All five wrap types; custom wrap points are FMT-WRAP-EDIT.* | ✅ | ✅ | ✅ | ✅ | — |
| FMT-WRAP-SIDE | Wrap sides | Wrap sides | 🔬 | 🔬 | ⬜ | 🔬 | pw-sbac |
| FMT-WRAP-DISTANCE | Wrap distance, automatic or per side | Wrap distance, automatic or per side | 🔬 | 🔬 | ⬜ | 🔬 | pw-k86o |
| FMT-WRAP-EDIT | Edit wrap points | Edit wrap points | 🔬 | 🔬 | ⬜ | 🔬 | pw-x61e |
| FMT-CALLOUT-FORMAT | Callout angle, drop, length, border, accent bar | Callout angle, drop, length, border, accent bar | 🔬 | 🔬 | ⬜ | 🔬 | pw-am3b |
| FMT-CONNECTOR-TYPE | Straight, elbow, curved connectors | Straight, elbow, curved connectors | 🔬 | 🔬 | ⬜ | 🔬 | pw-ld5j |
| SHP-SELECT-STACKED | Select hidden or stacked objects from a list | Select hidden or stacked objects from a list | 🔬 | 🔬 | ⬜ | 🔬 | pw-84vx |
| SHP-TEXT-IN-SHAPE | Text inside ovals and other shapes | Add text to an AutoShape (right-click Add Text) and show/save it. | 🔬 | 🔬 | ⬜ | 🔬 | pw-2wv.5 |

## Fill, line and effects

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| FMT-FILL-SOLID | Solid fill with scheme, RGB, CMYK or spot colour | Solid fill with scheme, RGB, CMYK or spot colour | ✅ | ✅ | ✅ | ✅ | — |
| FMT-FILL-TRANSPARENCY | Fill transparency | Fill transparency | 🔬 | 🔬 | ⬜ | 🔬 | pw-6vth |
| FMT-FILL-GRADIENT | One- and two-colour gradients with style, variant and angle | One- and two-colour gradients with style, variant and angle | 🔬 | 🔬 | ⬜ | 🔬 | pw-vq0c |
| FMT-FILL-GRADIENT-PRESET | Preset gradients | Preset gradients | 🔬 | 🔬 | ⬜ | 🔬 | pw-jn1o |
| FMT-FILL-PATTERN | Pattern fills | Pattern fills | 🔬 | 🔬 | ⬜ | 🔬 | pw-8lda |
| FMT-FILL-TEXTURE | Preset textures | Preset textures | 🔬 | 🔬 | ⬜ | 🔬 | pw-1nem |
| FMT-FILL-TEXTURE-USER | Picture textures | Picture textures (tile with offset, scale, alignment) | 🔬 | 🔬 | ⬜ | 🔬 | pw-c2op |
| FMT-FILL-PICTURE | Picture fill | Picture fill | 🔬 | 🔬 | ⬜ | 🔬 | pw-h3e2 |
| FMT-FILL-ROTATE | Rotate fill with object | Rotate fill with object | 🔬 | 🔬 | ⬜ | 🔬 | pw-2m0u |
| FMT-FILL-NONE | No fill | No fill | ✅ | ✅ | ✅ | ✅ | — |
| FMT-LINE-COLOR | Outline colour and transparency | Outline colour and transparency | ✅ | ✅ | ✅ | ✅ | — |
| FMT-LINE-WEIGHT | Outline weight | Outline weight | ✅ | ✅ | ✅ | ✅ | — |
| FMT-LINE-DASH | Dash styles | Dash styles | 🔬 | 🔬 | ⬜ | 🔬 | pw-2ge7 |
| FMT-LINE-COMPOUND | Compound line styles | Compound line styles (double, thick-thin) | 🔬 | 🔬 | ⬜ | 🔬 | pw-jjhw |
| FMT-LINE-PATTERN | Patterned line | Patterned line | 🔬 | 🔬 | ⬜ | 🔬 | pw-9vy4 |
| FMT-LINE-ARROW | Arrowheads | Arrowheads: style, length, width at both ends | 🔬 | 🔬 | ⬜ | 🔬 | pw-peoe |
| FMT-LINE-CAP-JOIN | Cap and join style | Cap and join style | 🔬 | 🔬 | ⬜ | 🔬 | pw-qf17 |
| FMT-LINE-INSET | Inset pen | Inset pen (outline inside) | 🔬 | 🔬 | ⬜ | 🔬 | pw-v7f1 |
| FMT-LINE-GRADIENT | Gradient outlines | Gradient outlines | 🔬 | 🔬 | ⬜ | 🔬 | pw-2x8v |
| FMT-BORDERART | BorderArt picture borders with weight, colour, stretch | BorderArt picture borders with weight, colour, stretch | 🔬 | 🔬 | ⬜ | 🔬 | pw-ii4v |
| FMT-SHADOW | Shadows | Shadows: presets, colour, offset, blur, size, transparency | 🔬 | 🔬 | ⬜ | 🔬 | pw-5w43 |
| FMT-GLOW | Glow | Glow | 🔬 | 🔬 | ⬜ | 🔬 | pw-zk11 |
| FMT-REFLECTION | Reflection | Reflection | 🔬 | 🔬 | ⬜ | 🔬 | pw-tu1p |
| FMT-SOFTEDGE | Soft edges | Soft edges | 🔬 | 🔬 | ⬜ | 🔬 | pw-asu9 |
| FMT-3D-ROTATE | 3-D rotation X/Y, perspective, field of view | 3-D rotation X/Y, perspective, field of view | 🔬 | 🔬 | ⬜ | 🔬 | pw-gxdm |
| FMT-3D-EXTRUDE | Extrusion depth, colour, direction | Extrusion depth, colour, direction | 🔬 | 🔬 | ⬜ | 🔬 | pw-juro |
| FMT-3D-BEVEL | Bevel top and bottom, contour | Bevel top and bottom, contour | 🔬 | 🔬 | ⬜ | 🔬 | pw-2g9v |
| FMT-3D-MATERIAL-LIGHT | Material and lighting presets | Material and lighting presets | 🔬 | 🔬 | ⬜ | 🔬 | pw-fws1 |
| FMT-3D-PRESET | 3-D presets | 3-D presets | 🔬 | 🔬 | ⬜ | 🔬 | pw-m7oc |
| FMT-BORDERART-MANAGE | Rename, delete and restore BorderArt borders | Rename, delete and restore BorderArt borders; always apply at default size; stretch or not | 🔬 | 🔬 | ⬜ | 🔬 | pw-tzz3 |
| FMT-BORDERART-CUSTOM | Create a custom BorderArt border from a picture | Create a custom BorderArt border from a picture | 🔬 | 🔬 | ⬜ | 🔬 | pw-ne9i |

## Text Effects

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| WA-INSERT | Insert Text Effects with a preset style | Insert Text Effects with a preset style | 🔬 | 🔬 | ⬜ | 🔬 | pw-c7d |
| WA-SHAPE | Text Effects warp shape | Text Effects warp shape | 🔬 | 🔬 | ⬜ | 🔬 | pw-6ae |
| WA-TEXT-FORMAT | Text Effects font, size, bold, italic, alignment | Text Effects font, size, bold, italic, alignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-6uj |
| WA-SPACING | Tracking, kerned pairs, equalise letter height | Tracking, kerned pairs, equalise letter height | 🔬 | 🔬 | ⬜ | 🔬 | pw-fsq |
| WA-ROTATE-CHARS | Rotate characters 90 degrees | Rotate characters 90 degrees | 🔬 | 🔬 | ⬜ | 🔬 | pw-ehp |
| WA-VERTICAL | Vertical Text Effects | Vertical Text Effects | 🔬 | 🔬 | ⬜ | 🔬 | pw-b9b |

## Pictures

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| PIC-INSERT | Insert picture from file, embedded or linked | Insert picture from file, embedded or linked | ✅ | ✅ | ✅ | ✅ | pw-ncy8 |
| PIC-EMPTY-FRAME | Empty picture frame | Empty picture frame | 🔬 | 🔬 | ⬜ | 🔬 | pw-8oi0 |
| PIC-REPLACE | Change picture, keep size and formatting | Change picture, keep size and formatting | ✅ | ✅ | ✅ | ✅ | pw-aj01 |
| PIC-REMOVE | Remove picture, keep frame | Remove picture, keep frame | 🔬 | 🔬 | ⬜ | 🔬 | pw-wxev |
| PIC-CROP | Crop on all sides | Crop on all sides | 🟡 | 🔬 | ⬜ | 🟡 | pw-drhm |
| PIC-FIT | Fill frame or fit frame | Fill frame or fit frame | 🔬 | 🔬 | ⬜ | 🔬 | pw-72wk |
| PIC-LOCK | Picture anchoring when frame resizes | Picture anchoring when frame resizes | 🔬 | 🔬 | ⬜ | 🔬 | pw-v0qf |
| PIC-SCALE | Picture scale percent | Picture scale percent | 🔬 | 🔬 | ⬜ | 🔬 | pw-bufj |
| PIC-BRIGHTNESS | Brightness | Brightness | 🔬 | 🔬 | ⬜ | 🔬 | pw-hel5 |
| PIC-CONTRAST | Contrast | Contrast | 🔬 | 🔬 | ⬜ | 🔬 | pw-85ym |
| PIC-RECOLOR | Recolour with a colour | Recolour with a colour; keep black | 🔬 | 🔬 | ⬜ | 🔬 | pw-84yq |
| PIC-COLORTYPE | Automatic, greyscale, black and white, washout | Automatic, greyscale, black and white, washout | 🔬 | 🔬 | ⬜ | 🔬 | pw-yn6p |
| PIC-RESTORE | Reset picture colours | Reset picture colours | 🔬 | 🔬 | ⬜ | 🔬 | pw-zdrc |
| PIC-TRANSPARENT | Set transparent colour | Set transparent colour | 🔬 | 🔬 | ⬜ | 🔬 | pw-k9ca |
| PIC-LINK-STATUS | Linked picture status and relink | Linked picture status and relink | 🔬 | 🔬 | ⬜ | 🔬 | pw-ibgi |
| PIC-INFO | Picture information | Picture information: resolution, size, format, colour depth | 🔬 | 🔬 | ⬜ | 🔬 | pw-ybc5 |
| PIC-FORMATS | Support EMF, WMF, PICT, JPEG, PNG, DIB, GIF, TIFF, CMYK JPEG | Support EMF, WMF, PICT, JPEG, PNG, DIB, GIF, TIFF, CMYK JPEG | 🔬 | 🔬 | ⬜ | 🔬 | pw-dr24 |
| PIC-RESOLUTION-EXPORT | Resolution presets for output | Resolution presets for output (96, 150, 300 dpi) (Also covers: PIC-EXPORT-RES.) | 🔬 | 🔬 | ⬜ | 🔬 | pw-opjc |
| PIC-CAPTION | Picture captions with styles | Picture captions with styles | 🔬 | 🔬 | ⬜ | 🔬 | pw-o7ir |
| PIC-ALPHA | Alpha channel awareness | Alpha channel awareness | 🔬 | 🔬 | ⬜ | 🔬 | pw-5117 |
| PIC-COMPRESS | Compress pictures for target | Compress pictures for target (commercial, desktop, web) with size estimate | 🔬 | 🔬 | ⬜ | 🔬 | pw-onub |
| PIC-COMPRESS-OPTIONS | Delete cropped areas, remove OLE data, resample, convert to JPEG | Delete cropped areas, remove OLE data, resample, convert to JPEG; apply to all or selected | 🔬 | 🔬 | ⬜ | 🔬 | pw-mpce |
| PIC-GFXMGR | Picture Manager pane listing all pictures with status | Picture Manager pane listing all pictures with status, thumbnails, sort, per-picture display | 🔬 | 🔬 | ⬜ | 🔬 | pw-lb4m |
| PIC-DISPLAY | Display pictures at detailed or hidden | Display pictures at detailed or hidden | 🔬 | 🔬 | ⬜ | 🔬 | pw-7co5 |
| PIC-LINK-MISSING | Choose what to do at print time for a missing linked picture | Choose what to do at print time for a missing linked picture | 🔬 | 🔬 | ⬜ | 🔬 | pw-44ud |

## Tables

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| TBL-INSERT | Insert table of rows and columns | Insert table of rows and columns | ✅ | ✅ | ✅ | ✅ | pw-iq69 |
| TBL-ROW-ADD | Add rows | Add rows | ✅ | ✅ | ✅ | ✅ | pw-vxis |
| TBL-COL-ADD | Add columns | Add columns | ✅ | ✅ | ✅ | ✅ | pw-h2j0 |
| TBL-ROW-DELETE | Delete rows | Delete rows | ✅ | ✅ | ✅ | ✅ | pw-e4c8 |
| TBL-COL-DELETE | Delete columns | Delete columns | ✅ | ✅ | ✅ | ✅ | pw-i820 |
| TBL-ROW-HEIGHT | Row height | Row height | ✅ | 🔬 | ⬜ | 🟡 | pw-4avx |
| TBL-COL-WIDTH | Column width | Column width | ✅ | 🔬 | ⬜ | 🟡 | pw-8byq |
| TBL-MERGE | Merge cells | Merge cells | 🟡 | 🔬 | ⬜ | 🟡 | pw-2hqp |
| TBL-SPLIT | Split cell | Split cell | 🔬 | 🔬 | ⬜ | 🔬 | pw-y4yl |
| TBL-SELECT | Select cells, rows, columns, table | Select cells, rows, columns, table | 🔬 | 🔬 | ⬜ | 🔬 | pw-176s |
| TBL-BORDERS | Cell borders with weight and colour | Cell borders with weight and colour | 🔬 | 🔬 | ⬜ | 🔬 | pw-e7re |
| TBL-DIAGONAL | Diagonal borders | Diagonal borders | 🔬 | 🔬 | ⬜ | 🔬 | pw-3bij |
| TBL-FILL | Cell fill | Cell fill | 🔬 | 🔬 | ⬜ | 🔬 | pw-6jmo |
| TBL-MARGINS | Cell margins | Cell margins | 🔬 | 🔬 | ⬜ | 🔬 | pw-c8by |
| TBL-VALIGN | Cell vertical alignment | Cell vertical alignment | 🔬 | 🔬 | ⬜ | 🔬 | pw-dhzu |
| TBL-TEXT-ORIENT | Cell text direction | Cell text direction | 🔬 | 🔬 | ⬜ | 🔬 | pw-nm9u |
| TBL-AUTOFORMAT | Table formats | Table formats (24 named) | 🔬 | 🔬 | ⬜ | 🔬 | pw-32xt |
| TBL-GROW | Grow table to fit text | Grow table to fit text | 🔬 | 🔬 | ⬜ | 🔬 | pw-7mpu |
| TBL-DIRECTION | Table direction left-to-right or right-to-left | Table direction left-to-right or right-to-left | 🔬 | 🔬 | ⬜ | 🔬 | pw-oo70 |
| TBL-CELL-TEXT | Text, font and paragraph formatting inside cells | Text, font and paragraph formatting inside cells | ✅ | ✅ | ✅ | ✅ | — |
| TBL-AUTOFORMAT-OPTIONS | Choose which parts of a table format apply | Choose which parts of a table format apply: text, alignment, shading, borders | 🔬 | 🔬 | ⬜ | 🔬 | pw-5twp |
| TBL-NUMBER-FORMAT | Format numbers in cells | Format numbers in cells: decimals, percent | 🔬 | 🔬 | ⬜ | 🔬 | pw-db6y |
| TBL-CURRENCY-FORMAT | Currency format with symbol and locale | Currency format with symbol and locale | 🔬 | 🔬 | ⬜ | 🔬 | pw-nlnc |
| TBL-CREATE-DIRECTION | Choose table direction on creation | Choose table direction on creation | 🔬 | 🔬 | ⬜ | 🔬 | pw-ekbn |

## Colour

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| CLR-SCHEME-APPLY | Apply one of ~68 built-in colour schemes | Apply one of ~68 built-in colour schemes | 🔬 | 🔬 | ⬜ | 🔬 | pw-mqcj |
| CLR-SCHEME-CUSTOM | Create and edit a custom colour scheme | Create and edit a custom colour scheme | 🔬 | 🔬 | ⬜ | 🔬 | pw-m4yh |
| CLR-RGB | RGB colours | RGB colours | 🔬 | 🔬 | ⬜ | 🔬 | pw-ktug |
| CLR-CMYK | CMYK colours | CMYK colours | 🔬 | 🔬 | ⬜ | 🔬 | pw-xi7q |
| CLR-SPOT | Spot colours | Spot colours (12 spot inks) | 🔬 | 🔬 | ⬜ | 🔬 | pw-9znv |
| CLR-TINT | Tints and shades | Tints and shades | 🔬 | 🔬 | ⬜ | 🔬 | pw-ypjq |
| CLR-TRANSPARENCY | Colour transparency | Colour transparency | 🔬 | 🔬 | ⬜ | 🔬 | pw-e27j |
| CLR-SCHEME-SLOTS | Scheme-linked colours that follow the scheme | Scheme-linked colours that follow the scheme | ✅ | 🔬 | ⬜ | 🟡 | pw-rtti |
| CLR-IN-USE | Colours in use list | Colours in use list | 🔬 | 🔬 | ⬜ | 🔬 | pw-x5n2 |
| CLR-REPLACE | Replace a colour throughout, with tint rules | Replace a colour throughout, with tint rules | 🔬 | 🔬 | ⬜ | 🔬 | pw-s3tb |
| CLR-MODE | Document colour mode | Document colour mode: desktop, process, spot, black and white | 🔬 | 🔬 | ⬜ | 🔬 | pw-ap7f |
| CLR-PLATES | Create and manage ink plates | Create and manage ink plates | 🔬 | 🔬 | ⬜ | 🔬 | pw-foxb |
| CLR-SPOT-TO-PROCESS | Convert spot to process | Convert spot to process | 🔬 | 🔬 | ⬜ | 🔬 | pw-s326 |
| CLR-COLOR-MANAGED | Colour-managed colour type | Colour-managed colour type | 🔬 | 🔬 | ⬜ | 🔬 | pw-aj6i |
| CLR-MODEL-HSL | Define colour by HSL, RGB, CMYK or hex | Define colour by HSL, RGB, CMYK or hex | 🔬 | 🔬 | ⬜ | 🔬 | pw-qe5e |
| CLR-HEX | Hex colour entry | Hex colour entry | 🔬 | 🔬 | ⬜ | 🔬 | pw-q2p5 |
| CLR-PANTONE | Look up a PANTONE colour or swatch | Look up a PANTONE colour or swatch | 🔬 | 🔬 | ⬜ | 🔬 | pw-stsw |
| CLR-INK-NAME | Name a spot ink | Name a spot ink; tint percentage | 🔬 | 🔬 | ⬜ | 🔬 | pw-l00i |
| CLR-MAINTAIN-LUM | Keep luminosity or tint when changing colour | Keep luminosity or tint when changing colour | 🔬 | 🔬 | ⬜ | 🔬 | pw-y0m5 |
| CLR-SAMPLER | Print colour sampler for a scheme | Print colour sampler for a scheme | 🔬 | 🔬 | ⬜ | 🔬 | pw-y387 |
| CLR-SCHEME-DELETE | Delete a custom colour scheme | Delete a custom colour scheme | 🔬 | 🔬 | ⬜ | 🔬 | pw-8r3l |

## Templates, content blocks and business information

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| TPL-CATALOG | Template catalog on start and on demand | Template catalog on start and on demand | 🔬 | 🔬 | ⬜ | 🔬 | pw-rwrf |
| TPL-WIZARD-TYPES | Publication types | Publication types (business cards, newsletters, brochures and so on) | 🔬 | 🔬 | ⬜ | 🔬 | pw-ml11 |
| TPL-WIZARD-OPTIONS | Per-template design options pane | Per-template design options pane (layout, colour, font, business info) | 🔬 | 🔬 | ⬜ | 🔬 | pw-5g4t |
| TPL-WIZARD-PAGE | Add a starter page layout to newsletters, catalogs, web sites | Add a starter page layout to newsletters, catalogs, web sites | 🔬 | 🔬 | ⬜ | 🔬 | pw-ultr |
| TPL-WIZARD-GROUP | Design Gallery parts | Design Gallery parts (borders, mastheads, sidebars, calendars, coupons and so on) | 🔬 | 🔬 | ⬜ | 🔬 | pw-sp4c |
| TPL-WIZARD-TAGS | Template roles of objects, used to refill business information | Template roles of objects, used to refill business information | 🔬 | 🔬 | ⬜ | 🔬 | pw-9hbu |
| TPL-CHANGE-TEMPLATE | Change template without losing content | Change template without losing content | 🔬 | 🔬 | ⬜ | 🔬 | pw-2502 |
| TPL-ISWIZARD | Know whether a publication is template-based | Know whether a publication is template-based | 🔬 | 🔬 | ⬜ | 🔬 | pw-6nv5 |
| TPL-WIZARD-AFTER | React when a template option changes | React when a template option changes | 🔬 | 🔬 | ⬜ | 🔬 | pw-ofpm |
| TPL-FOLDER | Template folder location | Template folder location | 🔬 | 🔬 | ⬜ | 🔬 | pw-eg15 |
| BB-GALLERY | Content block galleries | Content block galleries (advertisements, accents, calendars, business info, page elements) | 🔬 | 🔬 | ⬜ | 🔬 | pw-2xvs |
| BB-INSERT | Insert a content block onto a page | Insert a content block onto a page | 🔬 | 🔬 | ⬜ | 🔬 | pw-sn4h |
| BB-SAVE | Save selected objects as a content block with name | Save selected objects as a content block with name, category, keywords | 🔬 | 🔬 | ⬜ | 🔬 | pw-w08v |
| BB-DELETE | Delete a user content block | Delete a user content block | 🔬 | 🔬 | ⬜ | 🔬 | pw-ge9q |
| BB-SHOW | Show or hide a block in its gallery | Show or hide a block in its gallery | 🔬 | 🔬 | ⬜ | 🔬 | pw-ej6l |
| BB-WORKGROUP | Shared workgroup content blocks | Shared workgroup content blocks | 🔬 | 🔬 | ⬜ | 🔬 | pw-s4im |
| BIZINFO-SETS | Four business-information sets | Four business-information sets (primary, secondary, other, home) | 🔬 | 🔬 | ⬜ | 🔬 | pw-jw9g |
| BIZINFO-EDIT | Edit business information and apply to publication | Edit business information and apply to publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-n3vx |
| BIZINFO-FIELDS | Insert business-information fields into text | Insert business-information fields into text | 🔬 | 🔬 | ⬜ | 🔬 | pw-vnby |
| BIZINFO-UPDATE | Update business info on save | Update business info on save | 🔬 | 🔬 | ⬜ | 🔬 | pw-qe1b |
| TPL-CALENDAR-DATES | Calendar template | Calendar template: choose date range, one page per month or year | 🔬 | 🔬 | ⬜ | 🔬 | pw-3kva |
| TPL-VERSE | Suggested verse picker for cards | Suggested verse picker for cards (category, message parts) | 🔬 | 🔬 | ⬜ | 🔬 | pw-03k5 |
| TPL-APPLY-PANE | Apply a template from a pane, to current or new publication | Apply a template from a pane, to current or new publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-4klg |
| TPL-WIZARD-BLANK | Option to use a wizard for blank publications | Option to use a wizard for blank publications | 🔬 | 🔬 | ⬜ | 🔬 | pw-gse5 |
| BB-LIBRARY-PANE | Content block library pane with title | Content block library pane with title, description, size, date, keywords | 🔬 | 🔬 | ⬜ | 🔬 | pw-b3u2 |
| BB-PUB-CONTENT | Content from your publication | Content from your publication: insert, delete, delete all | 🔬 | 🔬 | ⬜ | 🔬 | pw-a198 |
| BIZINFO-SET-CRUD | Create, edit, delete named business information sets | Create, edit, delete named business information sets | 🔬 | 🔬 | ⬜ | 🔬 | pw-bor4 |
| BIZINFO-LOGO | Logo in a business information set | Logo in a business information set | 🔬 | 🔬 | ⬜ | 🔬 | pw-nvm3 |
| TPL-OWN-PACK | Our own template collection (newsletters, flyers, cards, order of service, prayer list) | Bundled, freely licensed templates in the same categories Publisher offers. | — | 🔬 | ⬜ | ⬜ | pw-1qfy |

## Mail, catalogue and email merge

*Note 2026-11-24: merge features will be redesigned as our own Data tab (pw-3iuc) rather than copied from Publisher; the Mailings ribbon tab has been removed. The Data tab is now shown as a mock-up (every control inert, bead pw-3iuc.1; rows RIB-DATA-* in the ribbon section); none of the features below are built yet.*

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| MM-WIZARD | Mail merge step-by-step pane | Mail merge step-by-step pane | 🔬 | 🔬 | ⬜ | 🔬 | pw-ju88 |
| MM-TYPE | Choose letter/catalog/email merge | Choose letter/catalog/email merge | 🔬 | 🔬 | ⬜ | 🔬 | pw-0j6p |
| MM-SOURCE-OPEN | Connect to recipient list | Connect to recipient list (Excel, Access, text, Outlook, other) | 🔬 | 🔬 | ⬜ | 🔬 | pw-kphz |
| MM-SOURCE-CREATE | Create a new recipient list | Create a new recipient list | 🔬 | 🔬 | ⬜ | 🔬 | pw-7mpm |
| MM-RECIPIENTS | Select and exclude recipients | Select and exclude recipients | 🔬 | 🔬 | ⬜ | 🔬 | pw-ays6 |
| MM-SORT | Sort recipients | Sort recipients | 🔬 | 🔬 | ⬜ | 🔬 | pw-a26l |
| MM-FILTER | Filter recipients by conditions | Filter recipients by conditions | 🔬 | 🔬 | ⬜ | 🔬 | pw-pfqv |
| MM-EDIT-RECORD | Edit a recipient | Edit a recipient | 🔬 | 🔬 | ⬜ | 🔬 | pw-iw42 |
| MM-FIELD-INSERT | Insert merge fields in text | Insert merge fields in text | 🔬 | 🔬 | ⬜ | 🔬 | pw-7zic |
| MM-FIELD-PICTURE | Insert picture merge fields | Insert picture merge fields | 🔬 | 🔬 | ⬜ | 🔬 | pw-8wkt |
| MM-FIELD-MAP | Match fields to standard address fields | Match fields to standard address fields | 🔬 | 🔬 | ⬜ | 🔬 | pw-y3up |
| MM-FIELD-ADD | Add a recipient column | Add a recipient column | 🔬 | 🔬 | ⬜ | 🔬 | pw-3fy7 |
| MM-ADDRESS-BLOCK | Address block and greeting line insertion | Address block and greeting line insertion | 🔬 | 🔬 | ⬜ | 🔬 | pw-vmo5 |
| MM-PREVIEW | Preview merged results | Preview merged results | 🔬 | 🔬 | ⬜ | 🔬 | pw-2bsz |
| MM-SUPPRESS-BLANK | Suppress blank lines | Suppress blank lines | 🔬 | 🔬 | ⬜ | 🔬 | pw-td0h |
| MM-MERGE-PRINT | Merge to printer | Merge to printer | 🔬 | 🔬 | ⬜ | 🔬 | pw-r141 |
| MM-MERGE-NEW-PUB | Merge to a new publication | Merge to a new publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-ldrx |
| MM-MERGE-EXISTING | Merge to an existing publication | Merge to an existing publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-x3ef |
| MM-MERGE-EMAIL | Merge to email with To, subject, priority | Merge to email with To, subject, priority | 🔬 | 🔬 | ⬜ | 🔬 | pw-bvod |
| MM-EMAIL-ATTACH | Attach files to merged email | Attach files to merged email | 🔬 | 🔬 | ⬜ | 🔬 | pw-45dt |
| MM-EMAIL-CCBCC | Cc and Bcc | Cc and Bcc | 🔬 | 🔬 | ⬜ | 🔬 | pw-tidt |
| MM-EXPORT-LIST | Export recipient list to .mdb or .csv | Export recipient list to .mdb or .csv | 🔬 | 🔬 | ⬜ | 🔬 | pw-6ak1 |
| MM-SHORTCUT | Create shortcut to merge | Create shortcut to merge | 🔬 | 🔬 | ⬜ | 🔬 | pw-u6zd |
| MM-VALIDATE | Validate addresses | Validate addresses; flag invalid with comments **[ONLINE]** | 🔬 | 🔬 | ⬜ | 🔬 | pw-jztn |
| MM-FOLLOWUP | Third-party send and follow-up hooks | Third-party send and follow-up hooks **[ONLINE]** | 🔬 | 🔬 | ⬜ | 🔬 | pw-3xa4 |
| MM-BARCODE | Merge barcodes and postal codes | Merge barcodes and postal codes | 🔬 | 🔬 | ⬜ | 🔬 | pw-yaq9 |
| MM-EVENTS | Hooks before and after merge and records | Hooks before and after merge and records | 🔬 | 🔬 | ⬜ | 🔬 | pw-suwb |
| CAT-AREA | Catalog merge area on a page | Catalog merge area on a page | 🔬 | 🔬 | ⬜ | 🔬 | pw-730p |
| CAT-FIELD | Insert catalog text and picture fields | Insert catalog text and picture fields | 🔬 | 🔬 | ⬜ | 🔬 | pw-vdqk |
| CAT-REPEAT | Repeat counts across and down | Repeat counts across and down | 🔬 | 🔬 | ⬜ | 🔬 | pw-it03 |
| CAT-MEMBERS | Add and remove objects from the merge area | Add and remove objects from the merge area | 🔬 | 🔬 | ⬜ | 🔬 | pw-tqo5 |
| CAT-PAGES | Catalog starter page layouts | Catalog starter page layouts | 🔬 | 🔬 | ⬜ | 🔬 | pw-zw18 |
| MM-REMEMBER-MATCH | Remember field matching for a data source set | Remember field matching for a data source set | 🔬 | 🔬 | ⬜ | 🔬 | pw-y1kj |
| MM-FIND-DUPES | Find duplicate recipients and choose which to include | Find duplicate recipients and choose which to include | 🔬 | 🔬 | ⬜ | 🔬 | pw-8lio |
| MM-FIND-RECIPIENT | Find a recipient by field | Find a recipient by field | 🔬 | 🔬 | ⬜ | 🔬 | pw-5bvk |
| MM-CUSTOMIZE-COLUMNS | Add, delete, rename, move recipient columns | Add, delete, rename, move recipient columns | 🔬 | 🔬 | ⬜ | 🔬 | pw-3pn0 |
| MM-ADD-TO-LIST | Add another source to the recipient list with field matching | Add another source to the recipient list with field matching | 🔬 | 🔬 | ⬜ | 🔬 | pw-p4zr |
| MM-REFRESH | Refresh recipient data | Refresh recipient data | 🔬 | 🔬 | ⬜ | 🔬 | pw-se1v |
| MM-RECONNECT | Reconnect lost data sources or work without connection | Reconnect lost data sources or work without connection | 🔬 | 🔬 | ⬜ | 🔬 | pw-cxo7 |
| MM-DATA-CONNECT | Data connection wizard | Data connection wizard: ODBC, data feed, server, password | 🔬 | 🔬 | ⬜ | 🔬 | pw-pele |
| MM-TEXT-SEPARATOR | Choose separator for text file sources | Choose separator for text file sources | 🔬 | 🔬 | ⬜ | 🔬 | pw-o4xl |
| MM-SELECT-TABLE | Choose table or cube | Choose table or cube; multiple related tables | 🔬 | 🔬 | ⬜ | 🔬 | pw-r9l8 |
| MM-EMAIL-PREVIEW | Preview merged e-mails | Preview merged e-mails; subject personalisation | 🔬 | 🔬 | ⬜ | 🔬 | pw-9ukt |
| MM-EMAIL-STATUS | Sent and not sent view of merged e-mail | Sent and not sent view of merged e-mail | 🔬 | 🔬 | ⬜ | 🔬 | pw-pt6v |
| MM-PRINT-LIST | Print recipient list | Print recipient list: choose up to 15 columns, notes column, orientation, fit to width | 🔬 | 🔬 | ⬜ | 🔬 | pw-g6a7 |
| MM-DONE-PANE | Result pane after merge to new publication | Result pane after merge to new publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-lvfk |
| MM-ADDRESS-FORMAT | Address block format, country rules, greeting for invalid names | Address block format, country rules, greeting for invalid names | 🔬 | 🔬 | ⬜ | 🔬 | pw-rzdw |
| MM-FIELD-DB | Insert address or database fields from a field picker | Insert address or database fields from a field picker | 🔬 | 🔬 | ⬜ | 🔬 | pw-x80l |
| CAT-MULTI-AREA | Choose one merge area when several exist | Choose one merge area when several exist; others become static | 🔬 | 🔬 | ⬜ | 🔬 | pw-r6h0 |

## Web and forms

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| FILE-SAVE-WEB | Save as web page, filtered HTML, web archive | Save as web page, filtered HTML, web archive | 🔬 | 🔬 | ⬜ | 🔬 | pw-f8pm |
| FILE-WEB-PREVIEW | Preview a web publication in a browser | Preview a web publication in a browser | 🔬 | 🔬 | ⬜ | 🔬 | pw-00n3 |
| LNK-ADD | Add hyperlink to text or object | Add hyperlink to text or object | 🔬 | 🔬 | ⬜ | 🔬 | pw-88fx |
| LNK-URL | Web address link | Web address link | 🔬 | 🔬 | ⬜ | 🔬 | pw-s0b2 |
| LNK-EMAIL | Email link with subject | Email link with subject | 🔬 | 🔬 | ⬜ | 🔬 | pw-fq37 |
| LNK-PAGE | Link to a page, first, last, next, previous | Link to a page, first, last, next, previous | 🔬 | 🔬 | ⬜ | 🔬 | pw-vgq1 |
| LNK-PERSONALIZED | Personalised URL using merge data | Personalised URL using merge data | 🔬 | 🔬 | ⬜ | 🔬 | pw-9e9d |
| LNK-EDIT-REMOVE | Edit, move, remove hyperlink | Edit, move, remove hyperlink | 🔬 | 🔬 | ⬜ | 🔬 | pw-od06 |
| LNK-COLORS | Hyperlink and followed-hyperlink colours | Hyperlink and followed-hyperlink colours | 🔬 | 🔬 | ⬜ | 🔬 | pw-6kl7 |
| LNK-FILE | Link to a file | Link to a file | 🔬 | 🔬 | ⬜ | 🔬 | pw-eskf |
| WEB-PAGE-OPTIONS | Page title/description/keywords | Page title/description/keywords | 🔬 | 🔬 | ⬜ | 🔬 | pw-z5v7 |
| WEB-FILENAME | Published file name per page | Published file name per page | 🔬 | 🔬 | ⬜ | 🔬 | pw-77ly |
| WEB-SOUND | Background sound with loop | Background sound with loop | 🔬 | 🔬 | ⬜ | 🔬 | pw-snli |
| WEB-NAVBAR | Navigation bars with 25 designs, orientation, button style | Navigation bars with 25 designs, orientation, button style | 🔬 | 🔬 | ⬜ | 🔬 | pw-dev0 |
| WEB-NAVBAR-LINKS | Navigation bar links and auto-update | Navigation bar links and auto-update | 🔬 | 🔬 | ⬜ | 🔬 | pw-50j2 |
| WEB-NAVBAR-EVERY | Add navigation bar to every page | Add navigation bar to every page | 🔬 | 🔬 | ⬜ | 🔬 | pw-qlln |
| WEB-FORM-BUTTON | Submit and reset buttons | Submit and reset buttons | 🔬 | 🔬 | ⬜ | 🔬 | pw-mmpq |
| WEB-FORM-SUBMIT | Where form data goes | Where form data goes: server, email, program, with format | 🔬 | 🔬 | ⬜ | 🔬 | pw-ppqk |
| WEB-FORM-HIDDEN | Hidden form fields | Hidden form fields | 🔬 | 🔬 | ⬜ | 🔬 | pw-gdnc |
| WEB-FORM-TEXT | Single and multi-line text input | Single and multi-line text input | 🔬 | 🔬 | ⬜ | 🔬 | pw-a5fy |
| WEB-FORM-LIST | List box with items | List box with items | 🔬 | 🔬 | ⬜ | 🔬 | pw-l2a0 |
| WEB-FORM-OPTION | Option buttons | Option buttons | 🔬 | 🔬 | ⬜ | 🔬 | pw-dwlb |
| WEB-FORM-CHECK | Check boxes | Check boxes | 🔬 | 🔬 | ⬜ | 🔬 | pw-lw8s |
| WEB-HTML-FRAGMENT | HTML code fragment | HTML code fragment | 🔬 | 🔬 | ⬜ | 🔬 | pw-837p |
| WEB-HOTSPOT | Image hot spot | Image hot spot | 🔬 | 🔬 | ⬜ | 🔬 | pw-o4gq |
| WEB-COMPONENT | Web components | Web components | 🔬 | 🔬 | ⬜ | 🔬 | pw-bn1t |
| WEB-ENCODING | Save web pages with chosen encoding | Save web pages with chosen encoding | 🔬 | 🔬 | ⬜ | 🔬 | pw-hlu2 |
| WEB-FOLDER | Organise files in a folder | Organise files in a folder | 🔬 | 🔬 | ⬜ | 🔬 | pw-vyuh |
| WEB-VML | Rely on VML for shapes | Rely on VML for shapes | 🔬 | 🔬 | ⬜ | 🔬 | pw-2b2v |
| WEB-WEBFONTS | Show only web fonts | Show only web fonts | 🔬 | 🔬 | ⬜ | 🔬 | pw-o8dz |
| WEB-INCREMENTAL | Incremental upload | Incremental upload | 🔬 | 🔬 | ⬜ | 🔬 | pw-jvjp |
| WEB-TEMPLATES | Web site and email templates and page types | Web site and email templates and page types | 🔬 | 🔬 | ⬜ | 🔬 | pw-wt5l |
| WEB-EASYBUILDER | Easy Web Site Builder | Easy Web Site Builder: choose site content pages | 🔬 | 🔬 | ⬜ | 🔬 | pw-a1a1 |
| LNK-SCREENTIP | Hyperlink ScreenTip text | Hyperlink ScreenTip text | 🔬 | 🔬 | ⬜ | 🔬 | pw-yfda |
| LNK-BOOKMARK | Bookmarks | Bookmarks: add, delete, go to, sort by name or location | 🔬 | 🔬 | ⬜ | 🔬 | pw-xtgl |
| LNK-BOOKMARK-TARGET | Link to a bookmark in this publication | Link to a bookmark in this publication | 🔬 | 🔬 | ⬜ | 🔬 | pw-3154 |
| LNK-TARGET-FRAME | Target frame for a link | Target frame for a link | 🔬 | 🔬 | ⬜ | 🔬 | pw-lvx1 |
| LNK-REMOVE | Remove link from the edit dialog | Remove link from the edit dialog | 🔬 | 🔬 | ⬜ | 🔬 | pw-2qv2 |
| WEB-FORM-PASSWORD | Text field that hides input with asterisks, with character limit | Text field that hides input with asterisks, with character limit | 🔬 | 🔬 | ⬜ | 🔬 | pw-rj7o |
| WEB-FORM-IMAGE-BUTTON | Image button type | Image button type | 🔬 | 🔬 | ⬜ | 🔬 | pw-ubig |
| WEB-FORM-LABEL | Return-data label and value per control | Return-data label and value per control | 🔬 | 🔬 | ⬜ | 🔬 | pw-snif |
| WEB-FORM-METHOD | Choose GET or POST and form action | Choose GET or POST and form action | 🔬 | 🔬 | ⬜ | 🔬 | pw-4mdc |
| WEB-NAVBAR-INSERT | Insert an existing navigation bar | Insert an existing navigation bar; edit or delete | 🔬 | 🔬 | ⬜ | 🔬 | pw-s390 |
| WEB-NAVBAR-SCOPE | Insert on every page or this page only | Insert on every page or this page only | 🔬 | 🔬 | ⬜ | 🔬 | pw-y9gm |
| WEB-TARGET-BROWSER | Target browser compatibility level | Target browser compatibility level | 🔬 | 🔬 | ⬜ | 🔬 | pw-ffy3 |
| WEB-TARGET-MONITOR | Target screen size and pixels per inch | Target screen size and pixels per inch | 🔬 | 🔬 | ⬜ | 🔬 | pw-o45b |
| WEB-LINK-OPTIONS | Hyperlink colours, underline, update links on save, long file names | Hyperlink colours, underline, update links on save, long file names | 🔬 | 🔬 | ⬜ | 🔬 | pw-gnu4 |
| WEB-DEFAULT-FONTS | Default proportional and fixed fonts per character set | Default proportional and fixed fonts per character set | 🔬 | 🔬 | ⬜ | 🔬 | pw-im1k |
| WEB-EMAIL-JPEG | Send the whole page as one JPEG in e-mail | Send the whole page as one JPEG in e-mail | 🔬 | 🔬 | ⬜ | 🔬 | pw-1753 |
| WEB-CONVERT-PROMPT | Offer to save before converting between print and web | Offer to save before converting between print and web | 🔬 | 🔬 | ⬜ | 🔬 | pw-b9ym |

## Review and proofing

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| APP-AUTOFORMAT | Auto-format words, typing replacements, sequence checking | Auto-format words, typing replacements, sequence checking | — | — | ⬜ | ⬜ | pw-0rq9 |
| TXT-COUNT | Word, line, paragraph counts | Word, line, paragraph counts (word count) | — | — | ⬜ | ⬜ | pw-n0hx |
| TXT-LANGUAGE | Proofing language of text | Proofing language of text | — | — | ⬜ | ⬜ | pw-xmjj |
| TXT-FINDREPLACE | Find, find next, replace, replace all | Find, find next, replace, replace all | — | — | ⬜ | ⬜ | pw-8nsh |
| TXT-FIND-OPTIONS | Match case, whole word, direction | Match case, whole word, direction; East Asian match options | — | — | ⬜ | ⬜ | pw-iqj8 |
| TXT-HYPHENATION | Hyphenation | Hyphenation (auto) (Also covers: APP-HYPHEN, TXT-HYPHEN-MANUAL.) | — | — | ⬜ | ⬜ | pw-ciuv |
| TXT-SPELL-DIALOG | Spelling dialog | Spelling dialog: suggestions, ignore, ignore all, change, change all, add, check all stories | — | — | ⬜ | ⬜ | pw-05qt |
| TXT-SPELL-PANE | Check spelling as you type with options | Check spelling as you type with options | — | — | ⬜ | ⬜ | pw-heeg |
| TXT-AUTOCORRECT | AutoCorrect lists, exceptions, capitalisation rules, caps-lock fix | AutoCorrect lists, exceptions, capitalisation rules, caps-lock fix | — | — | ⬜ | ⬜ | pw-bx9c |
| TXT-AUTOCORRECT-MATH | Math AutoCorrect | Math AutoCorrect | — | — | ⬜ | ⬜ | pw-bmg2 |
| TXT-FIND-ARABIC | Find options | Find options: match diacritics, alef hamza, kashida, width | — | — | ⬜ | ⬜ | pw-pimy |
| TXT-FIND-RESEARCH | Research lookup from the Find pane | Research lookup from the Find pane | — | — | ⬜ | ⬜ | pw-vrrh |
| APP-DESIGN-CHECKER | Design Checker pane | Design Checker pane: problems list, fix, ignore, options, page range, master pages | — | — | ⬜ | ⬜ | pw-g2wv |
| APP-DC-SORT | Sort and filter Design Checker results | Sort and filter Design Checker results; remove fixed items | — | — | ⬜ | ⬜ | pw-j4ju |
| APP-DC-EMAIL | Design Checker for e-mail | Design Checker for e-mail: choose image quality | — | — | ⬜ | ⬜ | pw-ti4j |
| MM-DESIGN-CHECK | Run Design Checker from the merge pane | Run Design Checker from the merge pane | — | — | ⬜ | ⬜ | pw-3i3l |
| REV-THESAURUS | Thesaurus | Look up synonyms for the selected word (Shift+F7); offline word list. | — | — | ⬜ | ⬜ | pw-m3yg |
| REV-RESEARCH | Research pane and Smart Lookup | Look up a word in reference sources (open replacement: dictionary and Wikipedia lookup the user starts). | — | — | ⬜ | ⬜ | pw-rd63 |
| REV-TRANSLATE | Translate selection or publication | Translate text; offline models by default. | — | — | ⬜ | ⬜ | pw-ujh5 |
| REV-WORDCOUNT | Word count dialog | Count words, characters, lines and paragraphs, with or without footnote-like text. | — | — | ⬜ | ⬜ | pw-jjtd |
| REV-LANG | Set proofing language and language preferences | Set the language of selected text and the default editing languages. | 🔬 | 🔬 | ⬜ | 🔬 | pw-jmz8 |
| REV-DICT | Spelling dictionaries (offline Hunspell) and custom dictionary | Main and custom dictionaries; add and ignore words. | — | — | ⬜ | ⬜ | pw-7yhe |

## View and user interface

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| APP-WINDOW | Multiple publication windows | Multiple publication windows; move, resize, state | — | — | ✅ | ✅ | — |
| APP-ZOOM | Zoom presets, in, out | Zoom presets, in, out *Note: Zoom exists; Publisher dialogs and exact shortcuts not complete.* | — | — | 🟡 | 🟡 | pw-3yew |
| APP-VIEW-GUIDES | Show or hide boundaries and guides | Show or hide boundaries and guides | — | — | ⬜ | ⬜ | pw-joxu |
| APP-VIEW-TWOPAGE | Two-page spread view | Two-page spread view | — | — | ⬜ | ⬜ | pw-5yx9 |
| APP-VIEW-BASELINE | Show baseline guides | Show baseline guides | — | — | ⬜ | ⬜ | pw-j83k |
| APP-SCROLL-TO | Scroll an object into view | Scroll an object into view | — | — | ⬜ | ⬜ | pw-s4c1 |
| APP-SNAP | Snap to guides and to objects | Snap to guides and to objects | — | — | ⬜ | ⬜ | pw-b74y |
| APP-UNITS | Measurement units | Measurement units (inch, cm, mm, pica, point and more) | — | — | ⬜ | ⬜ | pw-rfn1 |
| APP-SELECT | Selection of shapes, text and cells | Selection of shapes, text and cells | — | — | ✅ | ✅ | — |
| APP-SELECT-WORD | Auto-select entire word | Auto-select entire word | — | — | ⬜ | ⬜ | pw-kvqy |
| APP-DRAGDROP | Drag-and-drop text editing | Drag-and-drop text editing | — | — | ⬜ | ⬜ | pw-9d5c |
| APP-UNDO | Undo and redo with availability queries and grouped actions | Undo and redo with availability queries and grouped actions *Note: Undo and redo work; the history list on the Undo button is missing.* | — | — | 🟡 | 🟡 | pw-ptxt |
| APP-STATUSBAR | Show or hide status bar | Show or hide status bar *Note: Shown; not all Publisher 2021 items.* | — | — | 🟡 | 🟡 | pw-g9h8 |
| APP-TIPS | Screen tips on objects, helpful mouse pointers, show tip pages | Screen tips on objects, helpful mouse pointers, show tip pages | — | — | ⬜ | ⬜ | pw-26fo |
| APP-CMDBARS | Customisable command bars and add-in toolbars | Customisable command bars and add-in toolbars | — | — | ⬜ | ⬜ | pw-8r0i |
| APP-NUDGE | Arrow-key nudge distance setting | Arrow-key nudge distance setting | — | — | ⬜ | ⬜ | pw-vsif |
| APP-PASTE-OPTIONS | Paste Options button | Paste Options button | — | — | ⬜ | ⬜ | pw-oiyl |
| APP-TIP-PAGES | Tip pages with Reset Tips | Tip pages with Reset Tips | — | — | ⬜ | ⬜ | pw-krys |
| APP-RIGHT-DRAG | Right-button drag popup | Right-button drag popup: Move Here, Copy Here, Cancel | — | — | ⬜ | ⬜ | pw-lhtt |
| APP-TOOLBAR-CUSTOMIZE | Legacy toolbar customiser dialogs | Legacy toolbar customiser dialogs (probably unused in 2021) | — | — | ⬜ | ⬜ | pw-j47o |
| RIB-HOME-CLIPBOAR | Ribbon: Home tab, Clipboard group | The Clipboard group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-enz9 |
| RIB-HOME-FONT | Ribbon: Home tab, Font group | The Font group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-if93 |
| RIB-HOME-PARAGRAP | Ribbon: Home tab, Paragraph group | The Paragraph group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-drbk |
| RIB-HOME-STYLES | Ribbon: Home tab, Styles group | The Styles group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-rphn |
| RIB-HOME-OBJECTS | Ribbon: Home tab, Objects group | The Objects group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-d5n1 |
| RIB-HOME-ARRANGE | Ribbon: Home tab, Arrange group | The Arrange group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-i0eh |
| RIB-HOME-EDITING | Ribbon: Home tab, Editing group | The Editing group of the Home ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-umg5 |
| RIB-INSERT-PAGES | Ribbon: Insert tab, Pages group | The Pages group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-6s82 |
| RIB-INSERT-TABLES | Ribbon: Insert tab, Tables group | The Tables group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-o0pw |
| RIB-INSERT-ILLUSTRA | Ribbon: Insert tab, Illustrations group | The Illustrations group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-lxdq |
| RIB-INSERT-BUILDING | Ribbon: Insert tab, Content Blocks group | The Content Blocks group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-d3y3 |
| RIB-INSERT-TEXT | Ribbon: Insert tab, Text group | The Text group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-54xn |
| RIB-INSERT-LINKS | Ribbon: Insert tab, Links group | The Links group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-gwmk |
| RIB-INSERT-HEADERFO | Ribbon: Insert tab, Header & Footer group | The Header & Footer group of the Insert ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-53o7 |
| RIB-PAGEDE-TEMPLATE | Ribbon: Page Design tab, Template group | The Template group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-sbgr |
| RIB-PAGEDE-PAGESETU | Ribbon: Page Design tab, Page Setup group | The Page Setup group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-vwk0 |
| RIB-PAGEDE-LAYOUT | Ribbon: Page Design tab, Layout group | The Layout group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-22fo |
| RIB-PAGEDE-PAGES | Ribbon: Page Design tab, Pages group | The Pages group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-i3be |
| RIB-PAGEDE-SCHEMES | Ribbon: Page Design tab, Palettes group | The Palettes group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-cl6f |
| RIB-PAGEDE-PAGEBACK | Ribbon: Page Design tab, Page Background group | The Page Background group of the Page Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-uhlp |
| RIB-MAILIN-START | Ribbon: Mailings tab, Start group | Removed 2026-11-24: replaced later by our own Data tab (pw-3iuc); not copied from Publisher. | — | — | ➖ | ➖ | pw-kkje |
| RIB-MAILIN-WRITEINS | Ribbon: Mailings tab, Write & Insert Fields group | Removed 2026-11-24: replaced later by our own Data tab (pw-3iuc); not copied from Publisher. | — | — | ➖ | ➖ | pw-1gyq |
| RIB-MAILIN-PREVIEWR | Ribbon: Mailings tab, Preview Results group | Removed 2026-11-24: replaced later by our own Data tab (pw-3iuc); not copied from Publisher. | — | — | ➖ | ➖ | pw-g1y7 |
| RIB-MAILIN-FINISH | Ribbon: Mailings tab, Finish group | Removed 2026-11-24: replaced later by our own Data tab (pw-3iuc); not copied from Publisher. | — | — | ➖ | ➖ | pw-vr0l |
| RIB-DATA-LIST | Ribbon: Data tab, List group (our own design) | Mock-up of our own Data tab: Data Source (drop-down) and Edit List, all greyed. Not a Publisher layout. | — | — | 🟡 | ⬜ | pw-3iuc.1 |
| RIB-DATA-FIELDS | Ribbon: Data tab, Fields group (our own design) | Mock-up: Insert Field (drop-down), Picture Field, Numbering, all greyed. | — | — | 🟡 | ⬜ | pw-3iuc.1 |
| RIB-DATA-LAYOUT | Ribbon: Data tab, Layout group (our own design) | Mock-up: Make Copies, Repeating Area, Per Sheet (drop-down), all greyed. | — | — | 🟡 | ⬜ | pw-3iuc.1 |
| RIB-DATA-CHECK | Ribbon: Data tab, Check group (our own design) | Mock-up: Preview, row navigator and Find Row, all greyed. | — | — | 🟡 | ⬜ | pw-3iuc.1 |
| RIB-DATA-CREATE | Ribbon: Data tab, Create group (our own design) | Mock-up: Create (drop-down), greyed. | — | — | 🟡 | ⬜ | pw-3iuc.1 |
| RIB-REVIEW-PROOFING | Ribbon: Review tab, Proofing group | The Proofing group of the Review ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-fjkt |
| RIB-REVIEW-LANGUAGE | Ribbon: Review tab, Language group | The Language group of the Review ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-dayr |
| RIB-VIEW-VIEWS | Ribbon: View tab, Views group | The Views group of the View ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-klev |
| RIB-VIEW-LAYOUT | Ribbon: View tab, Layout group | The Layout group of the View ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-tzs7 |
| RIB-VIEW-SHOW | Ribbon: View tab, Show group | The Show group of the View ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-zm6g |
| RIB-VIEW-ZOOM | Ribbon: View tab, Zoom group | The Zoom group of the View ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-i0lx |
| RIB-VIEW-WINDOW | Ribbon: View tab, Window group | The Window group of the View ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-7c80 |
| RIB-HELP-HELP | Ribbon: Help tab, Help group | The Help group of the Help ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-1v0l |
| RIB-TEXTBO-TEXT | Ribbon: Text Box tab, Text group (contextual tab) | The Text group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-3e67 |
| RIB-TEXTBO-FONT | Ribbon: Text Box tab, Font group (contextual tab) | The Font group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-hhm8 |
| RIB-TEXTBO-ALIGNMEN | Ribbon: Text Box tab, Alignment group (contextual tab) | The Alignment group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-zp6v |
| RIB-TEXTBO-LINKING | Ribbon: Text Box tab, Linking group (contextual tab) | The Linking group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-7a0u |
| RIB-TEXTBO-WORDARTS | Ribbon: Text Box tab, Text Effect Styles group (contextual tab) | The Text Effect Styles group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-5rrj |
| RIB-TEXTBO-TYPOGRAP | Ribbon: Text Box tab, Typography group (contextual tab) | The Typography group of the Text Box ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-52nr |
| RIB-SHAPEF-INSERTSH | Ribbon: Shape Format tab, Insert Shapes group (contextual tab) | The Insert Shapes group of the Shape Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-biv8 |
| RIB-SHAPEF-SHAPESTY | Ribbon: Shape Format tab, Shape Styles group (contextual tab) | The Shape Styles group of the Shape Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-u8k3 |
| RIB-SHAPEF-ARRANGE | Ribbon: Shape Format tab, Arrange group (contextual tab) | The Arrange group of the Shape Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-jun3 |
| RIB-SHAPEF-SIZE | Ribbon: Shape Format tab, Size group (contextual tab) | The Size group of the Shape Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | 🟡 | pw-egnn |
| RIB-PICTUR-INSERT | Ribbon: Picture Format tab, Insert group (contextual tab) | The Insert group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-y0j9 |
| RIB-PICTUR-ADJUST | Ribbon: Picture Format tab, Adjust group (contextual tab) | The Adjust group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-9si4 |
| RIB-PICTUR-SWAP | Ribbon: Picture Format tab, Swap group (contextual tab) | The Swap group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-wbqj |
| RIB-PICTUR-PICTURES | Ribbon: Picture Format tab, Picture Styles group (contextual tab) | The Picture Styles group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-3obe |
| RIB-PICTUR-ARRANGE | Ribbon: Picture Format tab, Arrange group (contextual tab) | The Arrange group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-2fdy |
| RIB-PICTUR-CROP | Ribbon: Picture Format tab, Crop group (contextual tab) | The Crop group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-50r8 |
| RIB-PICTUR-SIZE | Ribbon: Picture Format tab, Size group (contextual tab) | The Size group of the Picture Format ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-crxw |
| RIB-TABLED-TABLEFOR | Ribbon: Table Design tab, Table Formats group (contextual tab) | The Table Formats group of the Table Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-4yhi |
| RIB-TABLED-BORDERS | Ribbon: Table Design tab, Borders group (contextual tab) | The Borders group of the Table Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-f0o7 |
| RIB-TABLED-WORDARTS | Ribbon: Table Design tab, Text Effect Styles group (contextual tab) | The Text Effect Styles group of the Table Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-snxx |
| RIB-TABLED-TYPOGRAP | Ribbon: Table Design tab, Typography group (contextual tab) | The Typography group of the Table Design ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-zotw |
| RIB-TABLEL-TABLE | Ribbon: Table Layout tab, Table group (contextual tab) | The Table group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-yb0q |
| RIB-TABLEL-ROWSCOLU | Ribbon: Table Layout tab, Rows & Columns group (contextual tab) | The Rows & Columns group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-717u |
| RIB-TABLEL-MERGE | Ribbon: Table Layout tab, Merge group (contextual tab) | The Merge group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-xbs8 |
| RIB-TABLEL-ALIGNMEN | Ribbon: Table Layout tab, Alignment group (contextual tab) | The Alignment group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-bal3 |
| RIB-TABLEL-ARRANGE | Ribbon: Table Layout tab, Arrange group (contextual tab) | The Arrange group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-7v01 |
| RIB-TABLEL-SIZE | Ribbon: Table Layout tab, Size group (contextual tab) | The Size group of the Table Layout ribbon tab: same controls, order, sizes, KeyTips and shortcuts as Publisher 2021; every control works. *Note: Group is shown today; controls that need a feature below are greyed out until that feature is built.* | — | — | 🟡 | ⬜ | pw-95tg |
| BS-LOOK | File screen look (ribbon-style File screen with a left rail) | A File screen with a left rail and one page per section. | — | — | ✅ | ✅ | — |
| BS-HOME | File screen: Home page | Templates and recent publications. | — | — | 🟡 | 🟡 | pw-bsz7 |
| BS-NEW | File screen: New page | Office/Built-In template tabs, search, blank publication. | — | — | 🟡 | 🟡 | pw-8e9e |
| BS-OPEN | File screen: Open page | Places list (Recent, This PC, Browse) and recent files. | — | — | 🟡 | 🟡 | pw-fm7m |
| BS-INFO | File screen: Info page | Business information, embedded fonts, publication properties, compress pictures. | — | — | 🟡 | 🟡 | pw-nwzf |
| BS-SAVEAS | File screen: Save As page | Places list and file type choice. | — | — | 🟡 | 🟡 | pw-o5yf |
| BS-PRINT | File screen: Print page | Printer, copies, range, duplex, settings and preview. *Note: Printer, Printer Properties, copies, pages, collation and the preview work; duplex, paper size and colour mode are shown greyed.* | — | — | 🟡 | 🟡 | pw-tkvm |
| BS-SHARE | File screen: Share page | Email, cloud and share options (open replacements, ROADMAP Phase 2). | — | — | ⬜ | ⬜ | pw-gto1 |
| BS-EXPORT | File screen: Export page | Create PDF/XPS, Package for Printing, Save for Photo Printing, Publish HTML, Change File Type. | — | — | 🟡 | 🟡 | pw-zbqw |
| BS-CLOSE | File screen: Close command | Close with save prompt. | — | — | 🟡 | 🟡 | pw-vjhd |
| BS-FEEDBACK | File screen: Feedback page | Send feedback (replaced by the same two pre-filled GitHub issue links as the Help tab). | — | — | ✅ | ✅ | pw-q8i0 |
| BS-OPTIONS | File screen: Options window | Application options dialog. | — | — | ⬜ | ⬜ | pw-5xqu |
| BS-ACCOUNT | File screen: Account page | Microsoft account sign-in, product information and Office theme. *Note: Not planned: no Microsoft account sign-in.* | — | — | — | — | — |
| UI-THEME | Colorful theme window, tab row and colours | White ribbon, dark-green accents, flat icons (our own icons by design). | — | — | ✅ | ✅ | — |
| UI-RIBBON-SHRINK | Ribbon shrinks to fit the window | Groups collapse to drop-downs as the window narrows. | — | — | ✅ | ✅ | — |
| UI-QAT | Quick Access Toolbar and title bar | Save, Undo, Redo, Touch/Mouse Mode and Customize arrow left of the centred "name.pub - Publisher" title. *Note: QAT customisation and touch mode not done.* | — | — | 🟡 | 🟡 | pw-7owt |
| UI-TOUCHMODE | Touch/Mouse Mode | Switch the ribbon to wider touch spacing. | — | — | ⬜ | ⬜ | pw-mpeu |
| UI-STATUSBAR-FULL | Status bar items | Page button, object position and size, Single/Two-Page buttons, zoom slider, Whole Page button; dark green. | — | — | 🟡 | 🟡 | pw-x5fn |
| UI-PAGENAV | Pages navigation pane | Thumbnails with page numbers, "Pages" heading, collapse, section headings, right-click menu. | — | — | 🟡 | 🟡 | pw-rtq3 |
| UI-RULERS | Rulers and scroll bars | Horizontal and vertical rulers in inches, page-step buttons. *Note: Ruler label bug tracked in pw-f28.11.* | — | — | 🟡 | 🟡 | pw-gdtk |
| UI-RIBBON-COLLAPSE | Collapse and expand the ribbon (Ctrl+F1, tab double-click) | Hide the ribbon body and show it on demand. | — | — | ⬜ | ⬜ | pw-6nxf |
| UI-KEYTIPS | Alt key tips for every ribbon control | Alt then letters, as listed in the UI inventory. | — | — | ⬜ | ⬜ | pw-v1al |
| UI-CONTEXT-TABS | Contextual ribbon tabs | Shape Format, Text Box, Picture Format, Table Design and Layout appear by selection with coloured context labels. *Note: Tabs exist; most controls are greyed out.* | — | — | 🟡 | 🟡 | pw-pmlw |
| CTX-TEXTBOX | Right-click menu: text box | Context menu for a text box: same items, order, separators and enabled rules as Publisher 2021. | — | — | 🟡 | 🟡 | pw-gxoy |
| CTX-AUTOSHAPE | Right-click menu: AutoShape | Context menu for a AutoShape: same items, order, separators and enabled rules as Publisher 2021. | — | — | 🟡 | 🟡 | pw-5dn3 |
| CTX-PICTURE | Right-click menu: picture | Context menu for a picture: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-dung |
| CTX-TABLE | Right-click menu: table | Context menu for a table: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-k7s5 |
| CTX-PAGE | Right-click menu: page in the Pages pane | Context menu for a page in the Pages pane: same items, order, separators and enabled rules as Publisher 2021. | — | — | 🟡 | 🟡 | pw-nkoh |
| CTX-MASTER | Right-click menu: master page | Context menu for a master page: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-375d |
| CTX-MERGE | Right-click menu: mail merge field or recipient | Context menu for a mail merge field or recipient: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-er5a |
| CTX-RULER | Right-click menu: ruler | Context menu for a ruler: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-ivau |
| CTX-GUIDE | Right-click menu: guide | Context menu for a guide: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-39ms |
| CTX-LINE | Right-click menu: line and connector | Context menu for a line and connector: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-dxbk |
| CTX-GROUP | Right-click menu: group | Context menu for a group: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-xger |
| CTX-WORDART | Right-click menu: Text Effects | Context menu for a Text Effects object: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-9j9b |
| CTX-MULTI | Right-click menu: multi-object selection | Context menu for a multi-object selection: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-bt4o |
| CTX-TEXT-SPELL | Right-click menu: misspelled word | Context menu for a misspelled word: same items, order, separators and enabled rules as Publisher 2021. *Note: Item list not yet recorded from Publisher; research first.* | — | — | ⬜ | 🔬 | pw-4d06 |
| CTX-DRAGDROP | Right-click menu: right-button drag popup | Context menu for a right-button drag popup: same items, order, separators and enabled rules as Publisher 2021. | — | — | ⬜ | ⬜ | pw-s4s1 |
| DLG-FONT | Dialog: Font | The Font dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-uy8x |
| DLG-PARAGRAPH | Dialog: Paragraph | The Paragraph dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-t06a |
| DLG-FORMAT-SHAPE | Dialog: Format AutoShape / Format Shape (fill, line, size, layout, picture, text box tabs) | The Format AutoShape / Format Shape (fill, line, size, layout, picture, text box tabs) dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-trdg |
| DLG-FORMAT-PICTURE | Dialog: Format Picture | The Format Picture dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-24vh |
| DLG-FORMAT-TABLE | Dialog: Format Table | The Format Table dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-q2qd |
| DLG-PAGE-SETUP | Dialog: Page Setup | The Page Setup dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-l0kc |
| DLG-ZOOM | Dialog: Zoom | The Zoom dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-e3u5 |
| DLG-GOTO-PAGE | Dialog: Go To Page | The Go To Page dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-512v |
| DLG-INSERT-PAGE | Dialog: Insert Page | The Insert Page dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-1pap |
| DLG-TABS | Dialog: Tabs | The Tabs dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-gc3p |
| DLG-BULLETS | Dialog: Bullets and Numbering | The Bullets and Numbering dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ✅ | ✅ | pw-3v1k |
| DLG-COLORS | Dialog: Colors (standard, custom, scheme) | The Colors (standard, custom, scheme) dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-y3ul |
| DLG-SYMBOL | Dialog: Symbol | The Symbol dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-nvzv |
| DLG-HYPERLINK | Dialog: Insert Hyperlink | The Insert Hyperlink dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-lqkf |
| DLG-FINDREPL | Dialog: Find and Replace pane | The Find and Replace pane dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-u3n3 |
| DLG-OPTIONS | Dialog: Publisher Options (all pages) | The Publisher Options (all pages) dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-ywl4 |
| DLG-PUBLISH-OPTS | Dialog: Publish Options and Print Options | The Publish Options and Print Options dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-q90a |
| DLG-PRINTDLG | Dialog: Print dialog and Print Preview | The Print dialog and Print Preview dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-oq40 |
| DLG-STYLES-PANE | Dialog: Styles pane and Format Publication pane | The Styles pane and Format Publication pane dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-41bg |
| DLG-PROPERTIES | Dialog: Document properties dialog | The Document properties dialog dialog with every control, in the same layout and wording as Publisher 2021; its changes apply and save like the ribbon commands. | — | — | ⬜ | ⬜ | pw-vxcv |
| KEY-FILE | Keyboard shortcuts: File and window | Ctrl+N, O, S, P, W, F4 and Alt+F4 equivalents. Same keys and results as Publisher 2021. | — | — | 🟡 | 🟡 | pw-opbl |
| KEY-EDIT | Keyboard shortcuts: Editing and clipboard | Undo, redo, cut, copy, paste, paste special, select all, Copy Formatting. Same keys and results as Publisher 2021. | — | — | 🟡 | 🟡 | pw-0tnu |
| KEY-TEXT | Keyboard shortcuts: Text formatting | Ctrl+B/I/U, alignment, spacing, size, font focus, clear formatting. Same keys and results as Publisher 2021. | — | — | 🟡 | 🟡 | pw-m3o0 |
| KEY-NAV | Keyboard shortcuts: Navigation and zoom | Next/previous page, Go To, F5, F9, zoom steps, F6 object cycling. Same keys and results as Publisher 2021. | — | — | ⬜ | ⬜ | pw-xwva |
| KEY-OBJ | Keyboard shortcuts: Objects | Group, nudge with arrows and Alt+arrows, delete, Shift+F10 menu. Same keys and results as Publisher 2021. | — | — | ⬜ | ⬜ | pw-kl6j |
| KEY-PROOF | Keyboard shortcuts: Proofing and macros | F7, Shift+F7, F3/F4 find, Alt+F8, Alt+F11. Same keys and results as Publisher 2021. | — | — | ⬜ | ⬜ | pw-99gx |
| KEY-FIELDS | Keyboard shortcuts: Insert date, time and page number fields | Alt+Shift+T/D/P. Same keys and results as Publisher 2021. | — | — | ⬜ | ⬜ | pw-i3cs |

## Options and app

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| FILE-AUTORECOVER | Auto-recover information and interval | Auto-recover information and interval | — | — | ✅ | ✅ | pw-2o70 |
| FILE-BGSAVE | Background saving | Background saving | — | — | ⬜ | ⬜ | pw-yxrr |
| FILE-CONVERT-TYPE | Convert print publication to web or email and back | Convert print publication to web or email and back | — | — | ⬜ | ⬜ | pw-80ji |
| FILE-INFO-PERSONAL | Remove personal information on save | Remove personal information on save | — | — | ⬜ | ⬜ | pw-is98 |
| FILE-READONLY | Read-only handling | Read-only handling | — | — | ⬜ | ⬜ | pw-jscu |
| FILE-PATHS | Default folders for pictures and publications | Default folders for pictures and publications | — | — | ⬜ | ⬜ | pw-a3p2 |
| FILE-RECENT-PREVIEW | Template and file preview in the start screen | Template and file preview in the start screen | — | — | ⬜ | ⬜ | pw-hqm5 |
| APP-BASIC-COLORS | Show basic colours in colour pickers | Show basic colours in colour pickers | — | — | ⬜ | ⬜ | pw-ad66 |
| APP-FILEDIALOG | Standard open/save/folder pickers | Standard open/save/folder pickers | — | — | ⬜ | ⬜ | pw-vsns |
| APP-HELP | Help pane / Office Assistant | Help pane / Office Assistant | — | — | ⬜ | ⬜ | pw-05dr |
| APP-DIRECTION-DEFAULT | Default publication direction and text flow | Default publication direction and text flow (right-to-left support) | — | — | ⬜ | ⬜ | pw-f5bl |
| FILE-TEXT-ENCODING | Choose text encoding when saving as plain text | Choose text encoding when saving as plain text, with preview and formatting-loss warning | — | — | ⬜ | ⬜ | pw-2xdp |
| FILE-OVERWRITE-PROMPT | Replace existing file prompt with Yes to All | Replace existing file prompt with Yes to All | — | — | ⬜ | ⬜ | pw-kka2 |
| FILE-FORMAT-ALERT | Prompt when changing the save format | Prompt when changing the save format; recommended format | — | — | ⬜ | ⬜ | pw-g8a4 |
| FILE-CONVERT-FILTER | Choose an import file converter when opening | Choose an import file converter when opening | — | — | ⬜ | ⬜ | pw-iyh5 |
| FILE-INUSE | File-in-use dialogs | File-in-use dialogs: open read-only, be notified when free, read-write, discard or save as on external change | — | — | ⬜ | ⬜ | pw-wlbz |
| FILE-RECOVERY-PROMPT | Keep or discard recovered files at the end of document recovery | Keep or discard recovered files at the end of document recovery | — | — | ✅ | ✅ | pw-a3na |
| APP-KEYBOARD-AUTO | Auto switch keyboard to the language of surrounding text | Auto switch keyboard to the language of surrounding text | — | — | ⬜ | ⬜ | pw-xjry |
| APP-CHINESE-SIZES | Chinese font size names | Chinese font size names | — | — | ⬜ | ⬜ | pw-j7wu |
| APP-FONT-SUBST-EAST | Auto-substitute fonts for missing East Asian characters | Auto-substitute fonts for missing East Asian characters | — | — | ⬜ | ⬜ | pw-lh07 |
| APP-YEN | Display backslash as Yen sign | Display backslash as Yen sign | — | — | ⬜ | ⬜ | pw-gixr |
| APP-STARTUP-TYPES | Show publication types when starting | Show publication types when starting | — | — | ⬜ | ⬜ | pw-mgqa |
| APP-FIELD-UNDERLINE | Underline merge and business fields | Underline merge and business fields | — | — | ⬜ | ⬜ | pw-hm3k |

## Macros, add-ins and automation

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| APP-ADDINS | COM add-in loading | COM add-in loading | — | — | ⬜ | ⬜ | pw-qrwc |
| APP-SCRIPT-EVENTS | Document and application events for automation | Document and application events for automation | — | — | ⬜ | ⬜ | pw-bif9 |
| APP-SECURITY | Macro security level | Macro security level | — | — | ⬜ | ⬜ | pw-2adl |
| APP-FILESEARCH | File search service | File search service | — | — | ⬜ | ⬜ | pw-192k |
| APP-MACROS | Macro list | Macro list: run, step into, edit, create, delete | — | — | ⬜ | ⬜ | pw-lxzq |
| APP-MACRO-WARN | Enable or disable macros when opening | Enable or disable macros when opening | — | — | ⬜ | ⬜ | pw-au4k |
| APP-SIGN-VBA | Digital signature for a macro project | Digital signature for a macro project | — | — | ⬜ | ⬜ | pw-uxha |
| APP-DISABLED-ITEMS | List and re-enable disabled add-ins | List and re-enable disabled add-ins | — | — | ⬜ | ⬜ | pw-h2nx |
| APP-ADDINS-LIST | COM add-ins list with load behaviour | COM add-ins list with load behaviour | — | — | ⬜ | ⬜ | pw-hsjj |

## Connected features

Online features in Publisher 2021. Each gets an open replacement that needs no Microsoft account and sends nothing unless the user asks. Other `[ONLINE]` rows live in their own area (for example mail merge address validation).

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| APP-ONLINE-WEBSERVICE | Launch Microsoft web service pages | Launch Microsoft web service pages **[ONLINE]** | — | — | ⬜ | ⬜ | pw-zhqo |
| TPL-ONLINE | Online template and clip content window | Online template and clip content window **[ONLINE]** | — | — | ⬜ | ⬜ | pw-cgsu |
| BB-DOWNLOADED | Downloaded content blocks | Downloaded content blocks **[ONLINE]** | — | — | ⬜ | ⬜ | pw-n8b3 |
| PIC-ONLINE | Insert online pictures | Insert online pictures **[ONLINE]** | — | — | ⬜ | ⬜ | pw-eymq |
| FILE-CHECKOUT | Server check-out, check-in, discard check-out | Server check-out, check-in, discard check-out **[ONLINE]** | — | — | ⬜ | ⬜ | pw-nqtu |
| FILE-OPEN-URL | Open an address or document from a web address | Open an address or document from a web address; FTP login **[ONLINE]** | — | — | ⬜ | ⬜ | pw-mzrz |
| FILE-REPORT-PROBLEM | Send a problem file report to the vendor | Send a problem file report to the vendor **[ONLINE]** | — | — | ⬜ | ⬜ | pw-24ko |
| FILE-ACTIVATE | Product activation prompt | Product activation prompt. *Note: not planned: licensing only; KernPage is free and has no activation.* | — | — | — | — | — |
| FILE-PDF-ADDIN | Install missing PDF/XPS export components | Install missing PDF/XPS export components. *Note: not needed: PDF export is built in.* | — | — | — | — | — |
| CON-ONLINE-PICTURES | Online pictures from open image sources | Search Openverse, Wikimedia Commons, Pixabay, Pexels and Unsplash and insert with credit. | — | — | ⬜ | ⬜ | pw-rvhl |
| CON-ONLINE-TEMPLATES | Online template catalogue | Own bundled templates plus an online catalogue to browse and download. | — | — | ⬜ | ⬜ | pw-j3ib |
| CON-SHARE | Share | Email with attachment, Windows Share window, share sheets. | — | — | ⬜ | ⬜ | pw-y6ua |
| CON-CLOUD-SAVE | Save to cloud folders | Save to synced folders first; direct service sign-in later as an add-on. | — | — | ⬜ | ⬜ | pw-z6aw |
| CON-FEEDBACK | Feedback and Contact Support | Help tab: **Report a Problem** and **Suggest an Idea** open a pre-filled GitHub issue in the browser (version and Windows version filled in; nothing is sent until submitted there). Differs from Publisher on purpose: no Contact Support and no live support. | — | — | ✅ | ✅ | pw-sund |
| CON-HELP | Help | Built-in help pages and tutorials with links to online docs. | — | — | ⬜ | ⬜ | pw-k2s3 |
| CON-TRANSLATE | Translate (offline first) | Translate selected text or a whole publication. | — | — | ⬜ | ⬜ | pw-vc0v |
| CON-SMART-LOOKUP | Smart Lookup | Dictionary and Wikipedia lookup started on purpose. | — | — | ⬜ | ⬜ | pw-j5pw |
| CON-ACCOUNT | Microsoft account sign-in | Sign in to a Microsoft account. *Note: Not planned: KernPage is free and needs no account.* | — | — | — | — | — |

## Beyond Publisher

Differentiators from the competitor research and the ROADMAP Phase 3 ideas. Parity with Publisher 2021 comes first; these make KernPage worth choosing over the alternatives.

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| BEY-FIDELITY | Faithful .pub open and save, Publisher 98 to 2021, with a public fidelity report | Page-by-page comparison against real Publisher output; files round-trip into Publisher 2021 without repair prompts. | — | — | — | ⬜ | pw-b2m9 |
| BEY-BATCH-PDF | Batch .pub to PDF (images, text, index), command line and drag-and-drop | Folder recursion, parallel runs, problem log, no Office needed. | — | — | — | ⬜ | pw-cy9w |
| BEY-PREPRESS | Print-shop output: PDF/X-1a and X-4, CMYK and spot colours, bleed, crop marks, font embedding, preflight report | Print-ready PDF for commercial printers. | — | — | — | ⬜ | pw-rw5f |
| BEY-TAGGED-PDF | Accessible tagged PDF (PDF/UA), reading order, alt text and accessibility checker | Accessibility for schools, councils and non-profits. | — | — | — | ⬜ | pw-bxre |
| BEY-PRIVACY | Local and private by design: no telemetry, offline spelling and translation, AI-free by default | Everything works offline without an account. | — | — | — | ⬜ | pw-vluz |
| BEY-DATA-MERGE | Merge from CSV and Excel with labels, envelopes, name tags and repeatable data-driven layouts | Directories, menus and calendars from data. | — | — | — | ⬜ | pw-5bpf |
| BEY-TEMPLATES-LIB | Open template library through the Launcher (flyers, newsletters, bulletins, calendars, labels) | Freely licensed, signed packages; users keep and share their own packs. | — | — | — | ⬜ | pw-ynxi |
| BEY-MODERN-UX | Modern essentials: autosave and crash recovery, unlimited undo with history list, dark mode, high-DPI, touch, keyboard-only use | On top of the Publisher-familiar interface. | — | — | — | ⬜ | pw-dkzf |
| BEY-OPEN-FORMAT | Open formats and an exit path: published .pub spec, open native format, export to PDF, SVG, images, later IDML and ODF | Trust through being able to leave. | — | — | — | ⬜ | pw-urgv |
| BEY-CROSSPLATFORM | Mac and Linux versions, later tablets | Packaging and testing of the Rust and egui code. | — | — | — | ⬜ | pw-nhso |
| BEY-MINISTRATIVE | Church-management integration for bulletins | Build bulletins from service plans with KernPage templates. | — | — | — | ⬜ | pw-51ce |
| BEY-CHURCH-PACKS | Church and community template packs | Bulletins, newsletters, flyers, certificates and programs. | — | — | — | ⬜ | pw-mziu |
| BEY-MODERN-EXPORT | Modern export: EPUB, SVG, PNG and import from Scribus or Affinity | Extra formats beyond Publisher. | — | — | — | ⬜ | pw-p2q1 |
| BEY-PLUGINS | Plugins and scripting for automation | After a permission and sandbox design. | — | — | — | ⬜ | pw-orms |

## Launcher, updater and add-ons

The separate Launcher app, the updater and the add-ons catalogue (see [plan](research/launcher-and-addons.md)). The launcher and updater are tracked by the existing bead pw-dsi.

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| LCH-LAUNCHER | KernPage Launcher (window, Open, Recent files, settings, tray, start at login) | Separate launcher app; double-clicking a .pub still opens KernPage directly. *Note: Launcher crate is being built.* | — | — | 🟡 | 🟡 | pw-dsi |
| LCH-UPDATER | Auto-updater (signed feed, download, verify, install, relaunch, rollback, toast) | Update through the Launcher with checksum and signature checks. | — | — | ⬜ | ⬜ | pw-dsi |
| LCH-DETAILS-PANEL | Launcher details panel (version, size, what is new, release notes) | Honest details for each release. | — | — | ⬜ | ⬜ | pw-t2hj |
| LCH-FEED-SIGNING | Feed format, signing keys and blocklist | Signed index with serial and expiry; per-package signatures. | — | — | ⬜ | ⬜ | pw-i1su |
| LCH-ADDONS-CATALOGUE | Add-ons catalogue (install, update, remove, details) | Cards with status and a menu; filters and search. | — | — | ⬜ | ⬜ | pw-ceta |
| LCH-ADDON-DICT | Dictionary add-ons (Hunspell) | Per-language dictionaries with licence shown. | — | — | ⬜ | ⬜ | pw-rftn |
| LCH-ADDON-TEMPLATES | Template pack add-ons | ZIP of templates with manifest and licence. | — | — | ⬜ | ⬜ | pw-664g |
| LCH-ADDON-TRANSLATE | Offline translation model add-ons | Downloadable models run on the computer. | — | — | ⬜ | ⬜ | pw-jgg7 |
| LCH-ADDON-FONTS | Open font add-ons (OFL) | Per-user font install with licence file. | — | — | ⬜ | ⬜ | pw-r21u |
| LCH-ADDON-PLUGINS | Plugin add-ons | Only after a permission and sandbox design. | — | — | ⬜ | ⬜ | pw-zwn8 |
| LCH-PRIVACY-PAGE | Launcher privacy statement and code-signing policy | Published before any public release. | — | — | ⬜ | ⬜ | pw-uc2n |
| LCH-MSI | Installer: stable upgrade code, per-user install, launcher bundled | Windows installer changes for updates. | — | — | ⬜ | ⬜ | pw-dsi |

## Inventory gaps (research)

Things we know we do not yet know about Publisher 2021. Each is a research task.

| ID | Feature | Publisher behaviour | Read | Save | UI | Status | Bead |
|---|---|---|---|---|---|---|---|
| GAP-RIBBON-LABELS | Undecoded ribbon labels and drop-down contents | Some ribbon labels and the contents of galleries and drop-downs (Shapes, Table, Page Elements, Styles, Margins) were not expanded in the UI inventory. | 🔬 | 🔬 | — | 🔬 | pw-d8za |
| GAP-CTX-MENUS | Unrecorded right-click menus (picture, table, page, master, mail merge, ruler, guide) | These menus are built in code and were not captured; record them with UI automation or screenshots. | 🔬 | 🔬 | — | 🔬 | pw-ditl |
| GAP-SHORTCUTS | About 25 unmapped keyboard shortcuts | Accelerator command ids with no known meaning (Ctrl+M, Ctrl+Shift+W/Y/O/L/U/K/N/D/E/H/I/X/B, Ctrl+Q, Ctrl+T, Ctrl+F5, Ctrl+F10, Ctrl+Shift+J, Ctrl+F7 and others); find each by testing in Publisher 2021. | 🔬 | 🔬 | — | 🔬 | pw-3cje |
| GAP-MSODRAW-ENUMS | Drawing enumerations still to confirm | MSO drawing enum values (shape types, fill, line and effect kinds) must be confirmed against real files before use. | 🔬 | 🔬 | — | 🔬 | pw-tuo5 |
| GAP-COLLAPSED-RIBBON | Collapsed ribbon layouts at narrow widths | Layouts at narrow window widths were not examined. | 🔬 | 🔬 | — | 🔬 | pw-z185 |
| GAP-TABLE-EXTRA-GROUPS | Table Design groups shown for a cell range | Extra groups may appear when a table cell range is selected. | 🔬 | 🔬 | — | 🔬 | pw-cekv |
| GAP-SHARE-ACCOUNT-OPTIONS | Share, Account, Feedback and Options pages not opened | Online and personal File screen pages were not inventoried; record the non-personal layouts. | 🔬 | 🔬 | — | 🔬 | pw-9h9x |
| GAP-TYPE-FLAGS | Meaning of type-byte low flag bits | Unknown flag bits in the object type byte. | 🔬 | 🔬 | — | 🔬 | pw-2wv.6 |
| GAP-RENDER-CHECK | Render-check real-world files against Publisher output | Compare page images of real-world files with Publisher 2021 output to find display differences. | 🔬 | 🔬 | — | 🔬 | pw-2wv.7 |
| GAP-SPEC-NOT-TESTED | Items listed as not tested in the page research | Page operations not yet checked (see the spec). | 🔬 | 🔬 | — | 🔬 | pw-waa2 |
