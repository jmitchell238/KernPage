# KernPage roadmap

KernPage is free, open-source desktop publishing for Windows: bulletins, newsletters, flyers, programs and
posters, with text, pictures, tables and shapes, printed or saved as a PDF. It is made to replace Microsoft
Publisher for people who used it before, so it opens and saves Publisher (`.pub`) files too (and has its own
`.kern` format) and uses a ribbon-style interface. Once it covers the features people use, the goal is to
keep improving it in the places Publisher stopped.

This file tracks where we are and what is left, so new contributors can find something to work on.

> **The authoritative list of features is [`docs/FEATURES.md`](docs/FEATURES.md).** It catalogues every Publisher 2021 feature one by one, with its status and the issue that tracks it. Goal: cover the features people use in Publisher 2021 and keep `.pub` files round-tripping. The catalogue tracks each feature and its status. This roadmap is the summary; when the two differ, the catalogue wins.

*Last updated: 2026-11-27. Progress is measured against the feature catalogue; the per-area percentages in the table are rough estimates.*

## How to read this

| Mark | Meaning |
|---|---|
| ✅ | Done, and saved files checked by opening them in Publisher 2021 |
| 🟡 | Partly done (see the note) |
| 🔬 | Needs research first: find out how the `.pub` format stores it (see the rules below) |
| 🛠️ | Ready to build: the spec already describes it |
| ⬜ | Not started |

**Research rules for contributors.** The file format is documented in our own words in
[`docs/spec/PUB-FORMAT-SPEC.md`](docs/spec/PUB-FORMAT-SPEC.md), and code is written only from that spec. Never
copy or translate Microsoft code, icons, artwork or templates. Research means observing what Publisher writes
for our own sample files; open only files you made yourself in Publisher. Never look inside Microsoft program code, do not copy Microsoft's help text, dialog text or screenshots, and do not use files you did not create (see [CONTRIBUTING.md](CONTRIBUTING.md)).

## Where we are

**Status: version 0.2.5 is released (public GitHub release, signed updates). Overall about 10% of the 742 catalogued Publisher 2021 features** (48 done, 55 partly done, counted at half; see [`docs/FEATURES.md`](docs/FEATURES.md)). KernPage already opens, edits and saves real `.pub` files, so the everyday basics are there; counting every feature one by one is deliberately strict, which is why the percentage is low. The area percentages below are rough estimates of the main file-reading, saving and look-and-feel work. **Next up:** master pages (headers, footers, page numbers), styles, text box margins and autofit, more of the greyed-out ribbon commands, then more 0.x releases.

| Area | Done | Summary |
|---|---|---|
| Opening and displaying `.pub` files | ~85% | Text and formatting, shapes, pictures, tables, groups, rotation, text wrap, columns, linked text boxes |
| Saving changes to existing content | ~75% | Text, formatting, bullets and numbering, move/resize, rotation, fill/outline, wrap, columns, stacking order, groups, table text, table rows and columns, pictures, pages, page size and margins |
| Creating new content | ~50% | Text boxes, rectangles, ovals, lines, tables, pictures, new and duplicate pages |
| Look and feel (ribbon-style window) | ~80% | Window, every ribbon tab, object tabs, File screen, ribbon shrinking, Light/Dark themes, accents, command search |
| Ribbon commands that actually work | ~45% | Most of Home, much of Insert, Page Design and the table tabs; the rest are shown greyed out in place |
| Printing and PDF | ~60% | PDF export; File > Print sends pages straight to a chosen printer |
| Data tab (data-driven publishing), spelling, more templates, connected features | ~0% | Not started; the Data tab is a mock-up and there is one template (Church Bulletin) |

## Phase 1: cover what Publisher users rely on

### Files: reading and saving

| Feature | Status | Notes |
|---|---|---|
| Text editing, long stories | ✅ | |
| Character and paragraph formatting (font, size, bold/italic/underline, colour, alignment, spacing) | ✅ | |
| Clear Formatting | ✅ | Character formatting only; whether 2021 also resets paragraphs is 🔬 |
| Move and resize, including rotated and grouped objects | ✅ | |
| Rotation, line flips, fill, outline colour and weight | ✅ | Flipping non-line shapes 🔬 |
| Text wrap (all five types) and text box columns | ✅ | Custom wrap points 🔬 |
| Bring to front / send to back | ✅ | |
| Group and ungroup | ✅ | Nested and rotated groups 🔬 |
| Table cell text | ✅ | |
| New text boxes (with formatting) | ✅ | |
| New rectangles, ovals and lines | ✅ | Other AutoShapes 🔬, arrowheads and dashes 🔬 |
| Add, move and delete pages | ✅ | |
| Duplicate page | ✅ | Pages with groups, tables or linked boxes not yet supported |
| Document save counter | ✅ | |
| Insert, replace and delete pictures | ✅ | Linked pictures and cropping 🔬 |
| Insert tables; add/delete rows and columns; resize | ✅ | Merge and split cells 🔬, borders and shading 🔬 |
| Master pages (headers, footers, page numbers) | 🟡 | Shown, printed and kept when saving (0.2.0); editing them is 🔬, high priority |
| Bullets and numbering | ✅ | Bullet picker, numbering styles, indent buttons |
| Styles (apply, create, edit) | 🔬 | |
| Linking text boxes (create and break links), text overflow | 🟡 | Linked text flows between frames and shows overflow marks; see FEATURES.md for what is left |
| Text box margins, vertical alignment, text direction, autofit | 🔬 | |
| Colour schemes and font schemes (apply) | 🔬 | |
| Page size, margins, orientation | ✅ | Page Setup galleries and dialog; guides beyond the margins 🔬 |
| Text Effects | 🔬 | |
| Hyperlinks and bookmarks | 🔬 | |
| Page elements, calendars, borders and accents, advertisements, Contact Details | 🔬 | |
| Very old `.pub` files (Publisher 2000 and earlier) | 🔬 | |
| Fills beyond solid colour (gradient, pattern, texture, picture), transparency, shadows, glow, 3-D, border art | 🔬 | |
| Print and prepress settings (marks, bleed, colour separations, trapping, imposition, booklet and fold printing) | 🔬 | See the Print and prepress area |
| Colour models (RGB, CMYK, spot, tints), custom colour schemes, font schemes | 🔬 | |
| Fonts: embedding, substitution, Package for Printing | ⬜ | |
| Web publishing, forms, hotspots, navigation bars | 🔬 | Low priority |
| Repeating area (data-driven publishing; Publisher's catalog merge) | ⬜ | |
| Content blocks and Contact Details | 🔬 | |
| Text effects, OpenType features, East Asian and complex-script text | 🔬 | |
| Macros, add-ins and automation hooks | ⬜ | Low priority |
| Files saved by Publisher 2013–2021 | 🟡 | Open fine; extra settings 2021 adds are kept but not all understood |

### Editor and look

| Feature | Status | Notes |
|---|---|---|
| Ribbon-style window, ribbon tabs, object tabs, themes | ✅ | Our own icons and colours by design; Light, Dark, accents |
| File screen (Home, New, Open, Info, Save As, Print, Export, About, Feedback, Options) | ✅ | About page has a Support KernPage link |
| KernPage's own `.kern` format (new publications save as `.kern`; Save As can write `.pub`) | ✅ | Described in `docs/spec/KERN-FORMAT.md`; `.kern` files open by double-click |
| Signed updates and KernPage Launcher (open KernPage and recent files, check for updates, go back to an older version) | ✅ | Released in 0.2.0 |
| Demo screenshot mode (`--demo`) and install guide ([INSTALL.md](INSTALL.md)) | ✅ | |
| Ribbon shrinks to fit the window | ✅ | |
| Multi-select, right-click menus, split buttons, align | ✅ | |
| Snap to guides and objects | ⬜ | |
| Resize handles on groups | ⬜ | |
| Dialog boxes (Paragraph, Font, Format Shape, Zoom, Go To Page) | ⬜ | Page Setup, Layout Guides and Bullets and Numbering dialogs are done |
| Every greyed-out ribbon command made to work | 🟡 | About 45% work today |
| Undo history list on the Undo button | ⬜ | |
| Touch mode, Quick Access Toolbar customisation | ⬜ | |
| Right-click menus for pictures, tables, pages, masters, rulers and guides | 🔬 | The item lists are not yet recorded from Publisher |
| Keyboard shortcuts (about 25 still unmapped) | 🟡 | |
| Task panes (Styles, Format Publication, Picture Manager, Check Publication, Find and Replace) | ⬜ | |
| Options window and document properties | 🟡 | File > Options has Save (AutoRecover) and Appearance; document properties not yet |

### Output and tools

| Feature | Status | Notes |
|---|---|---|
| Create a PDF | ✅ | Under File > Export (XPS is not supported) |
| Print (printer, copies, page range, duplex) | 🟡 | Printer, copies, pages and collation print directly; duplex, paper size and colour mode are not yet |
| Spell check | ⬜ | Offline dictionaries such as Hunspell |
| Thesaurus | ⬜ | |
| Check Publication, AutoCorrect, hyphenation, word count, proofing language | ⬜ | |
| Print preview, booklet and fold printing, envelope and label printing | ⬜ | |
| Data tab (data-driven publishing): Data Source, Insert Field, Picture Field, Make copies (one copy per row), Repeating area, Preview, Create | 🟡 | Tab shown as a mock-up; features not built yet. Email sending is not planned: export and attach in your email client |
| Package for Printing | ⬜ | |
| XPS export | ⬜ | Low priority |

## Phase 2: connected features, done the open way

Publisher 2021 ties these to Microsoft services. We build them without a Microsoft account, using free and open
services, and only when the user asks. Nothing is sent anywhere unless the user chooses to.

| Feature | Plan | Status |
|---|---|---|
| **Online Pictures** | Search several free image sources from inside the app and insert the result with its licence and credit: Openverse (a search across openly licensed images), Wikimedia Commons, Pixabay, Pexels, Unsplash; possibly a general web image search (for example Google's or Bing's paid APIs) as an option the user configures. | ⬜ |
| **Online templates** | Our own template collection, bundled with the app and updated through app updates, plus an online catalogue (for example a templates repository on GitHub) the app can browse and download from; community submissions welcome. Add other open template sources if any exist. | ⬜ |
| **Share** | Open the user's email program with the file attached; on Windows use the built-in Share window (Nearby Sharing over Bluetooth and Wi-Fi); on Mac the Share menu with AirDrop; on phones and tablets the system share sheet. | ⬜ |
| **Save to OneDrive, Google Drive, Dropbox and similar** | First: save to any folder, including folders those services already sync, with step-by-step tutorials in Help. Later: direct sign-in to each service through its official API, as an optional add-on. | ⬜ |
| **Feedback (Report a Problem, Suggest an Idea)** | Help tab and File > Feedback open a pre-filled GitHub issue (bug report or idea) in the web browser. No live support, on purpose. | ✅ |
| **Help** | Built-in help pages and tutorials (including the cloud-folder guides above), a link to the online documentation and GitHub Discussions. | ⬜ |
| **Translate** | Translate selected text or a whole publication. Prefer an offline translator that runs on the computer (open models such as those behind Firefox Translations), with optional online services (for example LibreTranslate or DeepL) the user can choose. | ⬜ |
| **Smart Lookup** | Publisher's version searches the web for the selected word or phrase (definitions, encyclopedia articles, images). We could offer the same with a dictionary and Wikipedia lookup the user starts on purpose. | ⬜ |
| **Microsoft account sign-in** | Not planned. KernPage is free and needs no account. | — |

## Phase 3: beyond Publisher

Ideas for after parity. Not ordered.

- **Faithful `.pub` open and save for Publisher 98 through 2021**, with a public fidelity report (page-by-page comparison against real Publisher output on a shared sample set).
- **Launcher, updater and add-ons:** the KernPage Launcher with signed updates is released; still to come are automatic updates and an add-ons catalogue (spelling dictionaries, template packs, offline translation models, open fonts; plugins later). Tracked as pw-dsi and the Launcher area of the catalogue.
- **Print-shop output:** PDF/X-1a and X-4, CMYK and spot colours, bleed, crop marks, font embedding and a preflight report.
- **Batch `.pub` to PDF** with folder recursion, parallel runs and a problem log, with no Office needed.
- **Data Source from CSV and Excel** (Data tab) with labels, envelopes, name tags and repeatable data-driven layouts.
- **Private by design:** no telemetry, no account, offline spelling and translation, AI-free by default.
- **Modern essentials** on top of the Publisher interface: autosave and crash recovery, unlimited undo with a history list, dark mode, high-DPI, touch and keyboard-only use.
- **Mac and Linux** versions (the code is Rust and egui, so this is mostly packaging and testing), and later tablets.
- **Church-management integration:** let church-management apps build bulletins from service plans (order of service, sermon, announcements) using
  KernPage templates, then open them in KernPage for final touches or export straight to PDF.
- **Church and community template packs:** bulletins, newsletters, flyers, certificates and programs.
- **Modern formats:** export to EPUB, SVG, PNG and high-quality print PDF (PDF/X with bleed and crop marks), and
  import from other open formats such as Scribus or Affinity where possible.
- **Accessibility:** screen-reader support, alt text for pictures, a check for low-contrast colours, and tagged PDF.
- **An open `.pub` specification** that other programs (LibreOffice, Scribus) can use to read and write Publisher
  files.
- **Batch tools:** convert a folder of `.pub` files to PDF in one step, which helps anyone moving away from
  Publisher now that it is retired.
- **Plugins and scripting** for automation, after a permission and sandbox design.

These ideas are catalogued, with issues, in the "Beyond Publisher" and "Launcher, updater and add-ons" areas of [`docs/FEATURES.md`](docs/FEATURES.md).

## Releases

**0.2.5 is released** on [GitHub](https://github.com/jmitchell238/KernPage/releases/latest): master pages, headers and footers, and page numbers, saved in .pub and .kern files ([`docs/release-notes/0.2.5.md`](docs/release-notes/0.2.5.md)). 0.2.4 brought print preview, print layouts (copies or pages per sheet, booklets) and two-sided printing ([`docs/release-notes/0.2.4.md`](docs/release-notes/0.2.4.md)). 0.2.3 brought one bar at the top of the window and a new File screen ([`docs/release-notes/0.2.3.md`](docs/release-notes/0.2.3.md)). Before that, 0.2.2 fixed desktop shortcuts after updates and brought a sharper ribbon, a backup interface font and the new logo ([`docs/release-notes/0.2.2.md`](docs/release-notes/0.2.2.md)).

0.2.1 moved KernPage to the GPL-3.0 licence (with a contributor licence agreement, see [CLA.md](CLA.md)) and to the fonts that come with Windows for its own text. Versions up to and including 0.2.0 stay available under MIT OR Apache-2.0 at the `v0.2.0` tag. See [`docs/release-notes/0.2.1.md`](docs/release-notes/0.2.1.md).

0.2.0 was the first public release: signed updates through the KernPage Launcher, the `.kern` format, the About page with a Support KernPage link, Create a PDF, and an [install guide](INSTALL.md) ([`docs/release-notes/0.2.0.md`](docs/release-notes/0.2.0.md)). The release process is in [`docs/RELEASING.md`](docs/RELEASING.md). Further 0.x releases follow the "Next up" list above. Version 1.0 will come when KernPage covers everyday publishing tasks end to end.

## How to help

Pick a 🔬 or 🛠️ row in [`docs/FEATURES.md`](docs/FEATURES.md) (or an item above), open or claim its issue to say you are working on it, and read
[`docs/spec/PUB-FORMAT-SPEC.md`](docs/spec/PUB-FORMAT-SPEC.md) and the research rules in CONTRIBUTING first. Research results go
into the spec before any code that depends on them.
