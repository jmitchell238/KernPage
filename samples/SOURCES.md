# Sample files

This repository keeps a small core of Publisher `.pub` samples, enough for every reading and saving
code path to have a public test. The remaining samples (the other generated variants, such as the wider
Page Setup, table, list, master page, picture and script sets, and most of the local third-party
cache in `samples/external/`) are kept outside the repository. Tests that need one of
them print `skipped: <file> (set KERNPAGE_EXTRA_SAMPLES)` and pass when the files are absent.
Set the environment variable `KERNPAGE_EXTRA_SAMPLES` to a folder that mirrors `samples/` (for
example `gen/list-none.pub` inside it) and those tests run too; the full set is run before each
release (see `docs/RELEASING.md`). `samples/gen/manifest.json` still lists every generated sample,
including the ones that are not in this repository.

| File | Source | License |
|---|---|---|
| `tika-testPUBLISHER.pub` | Apache Tika test documents (`tika-parser-microsoft-module/src/test/resources/test-documents/testPUBLISHER.pub`) | Apache-2.0 |
| `lo-fdo59355-1.pub` | LibreOffice core (`writerperfect/qa/unit/data/draw/libmspub/pass/fdo59355-1.pub`), bug fdo#59355 | MPL-2.0 |

Licence texts for the two third-party files are in `samples/LICENSES/Apache-2.0.txt` and `samples/LICENSES/MPL-2.0.txt`.
Apache Tika is Copyright The Apache Software Foundation (Apache-2.0). 
The LibreOffice file is MPL-2.0 licensed test data from The Document Foundation; its source is the path above.
Samples made with Publisher contain only content created by the project; Microsoft's container structure is the file format.

Deliberately NOT included: LibreOffice `EDB-*.pub` files (Exploit-DB malicious samples).

## Controlled samples (made by the project owner in Publisher 2010, 2026-11-04)
| File | Content |
|---|---|
| `blank.pub` | empty Letter page |
| `hello.pub` | one text box at top-left: `Hello` |
| `two-boxes.pub` | two text boxes, each: `Hello` / **`HELLO BOLD`** |

## Generated samples (`samples/gen/`)
Made by `tools/make_samples.ps1`, which automates a locally installed Publisher 2010 (COM).
`manifest.json` records exactly what the script set (positions in points, fonts, sizes,
styles, alignment): it is the test oracle. The picture is `tools/assets/test-picture.png`,
an original image drawn by `tools/make_test_picture.py`. Bulletin text is original placeholder
content (hymn titles and scripture references only). Embedded file paths/usernames are
scrubbed after generation.

| File | Content |
|---|---|
| `apple-banana.pub` | "Apple" box created first (lower), "Banana" second (upper) |
| `formatting.pub` | fonts, sizes, bold/italic/underline, red text, centre/right/justify |
| `picture.pub` | one PNG picture |
| `bulletin.pub` | two-page church bulletin: title, picture, order of service, announcements, prayer list |
| `colors.pub` | text in black, red, green, blue, orange, gray |
| `shapes.pub` | filled+bordered rectangle, oval without line, 3 pt red line, outline-only rectangle |
| `table.pub` | 2 × 2 table with cells A1, B1, A2, B2 |
| `linked.pub` | one 30-paragraph story flowing through three linked text boxes (two on page 1, one on page 2) plus a standalone box |
| `group.pub` | a text box and a rectangle grouped together, plus an ungrouped text box |
| `wrap.pub` | a 20-paragraph story with a square-wrapped rectangle, a tight-wrapped oval and a no-wrap rectangle over it |

## Page Setup samples (`samples/gen/page-*.pub`, Publisher 2021, 2026-11-14)
Made by driving a real Publisher 2021 (COM, and for the "preset", "dialog" and Custom Margins cases the ribbon through keyboard automation): a copy of `samples/blank.pub` was opened, edited and `Save()`d (never `SaveAs`). Every file is a copy of one of the three bases; the save path in Contents header item 9 was replaced by a same-length `C:\sample\...` path and every file was byte-searched (ASCII and UTF-16) for the user name, the Windows profile folder and the application-data folder. `manifest.json` holds the page size, margins and object positions that Publisher 2021 itself reports when the shipped file is reopened (ground truth). Spec: section 4Q; UI: `docs/ui/PAGE-SETUP.md`.

| File | Content |
|---|---|
| `page-base.pub` | Letter portrait; Rect A (72,72,144x72), Text A (72,200,216x72), Rect Far (400,700,150x60); base of the page-setup cases |
| `page-base-2pages.pub` | page-base plus page 2 with Rect P2 (100,100,200x100) and Text P2 (100,300,300x60) |
| `page-a4-landscape-margins-2pages.pub` | 2-page base, A4 landscape (841.9x595.3 pt) and margins 54/36/72/90 pt |

## Table samples (`samples/gen/table-*.pub`, Publisher 2021, 2026-11-15)
Made by driving a real Publisher 2021 through COM (`Shapes.AddTable`, `Table.Rows/Columns.Add`, `.Delete`, `Shape.Delete`, `Cell.TextRange.Text`): a copy of `samples/blank.pub` (or of the previous case) was opened, edited and `Save()`d (never `SaveAs`). The save path in Contents header item 9 was replaced by a same-length `C:\sample\...` path and every file was byte-searched (ASCII and UTF-16, whole file and each stream) for the user name, the Windows profile folder and the application-data folder, with zero hits. Spec: section 4E.1; UI: `docs/ui/TABLES.md`. Sizes in points; "T" = table.

| File | Content |
|---|---|
| `table-3x3-text.pub` | T 3x3 at 72,72, 360 x 120 pt, cells A1..C3 |
| `table-empty.pub` | blank page re-saved by Publisher 2021 (the base of all table cases) |

## List samples (`samples/gen/list-*.pub`, Publisher 2021, 2026-11-21)
Made by driving a real Publisher 2021 through COM only (`TextRange.Paragraphs(i,1).ParagraphFormat.SetListType / ListNumberSeparator / ListNumberStart / ListIndent / ListBulletFontName / ListBulletFontSize / LeftIndent / FirstLineIndent / RightIndent`): `list-empty.pub` is `samples/blank.pub` opened and saved by Publisher 2021; every other file is a copy of it with one text box at 72,72 (300 x 300 pt) holding paragraphs "One", "Two", "Three" (more where noted), formatted and `Save()`d (never `SaveAs`). The save path in Contents header item 9 was replaced by a same-length `C:\sample\...` path and every file was byte-searched (ASCII and UTF-16LE, whole file and each stream, case-insensitive) for the user name, the Windows profile folder, the application-data folder and the temp folder, with zero hits. Spec: section 4R; UI: `docs/ui/BULLETS.md`. "p2" = second paragraph. Default bullet = Symbol U+00B7 at 10 pt; default separator `.`.

| File | Content |
|---|---|
| `list-bullet-default.pub` | default bullet on p2 only |
| `list-num-1dot.pub` | type 0 `1.` on the whole range |
| `list-num-types.pub` | 9 paragraphs, types 0,1,2,3,4,5,6,7,22 (default separator) |

## Master page samples (`samples/gen/master-*.pub`, Publisher 2021, 2026-11-25)
Made by driving a real Publisher 2021 through COM only (master text boxes, `Page.IgnoreMaster`, `TextRange.InsertPageNumber`, `Sections(1).PageNumberStart / PageNumberFormat`, `Sections.Add`), one change against `samples/blank.pub` or its 2-page version, then `Save()` (never `SaveAs`). The save path in Contents header item 9 was replaced by a same-length `C:\sample\...` path and every file was byte-searched (ASCII and UTF-16LE, case-insensitive) for the user name, the profile folder and the application-data folder, with zero hits. Spec: section 4T. Used by `crates/pub-format/tests/masters.rs`, `crates/pub2pdf/tests/pdf.rs`, `crates/kernpage-core/tests/save.rs` and the `masters` UI tests.

| File | Content |
|---|---|
| `master-pn-2p.pub` | master text box holding only a page-number field, 2 pages |
| `master-pn-section2-fmt.pub` | second section from page 2, start 1, lower Roman |

## Script samples (`samples/gen/script-*.pub`, Publisher 2021, 2026-11-22)
Made by driving a real Publisher 2021 through COM only: a copy of an empty publication got a text box holding `H2O x2 One`, individual characters were formatted through `TextRange.Characters(i, 1).Font` (`Superscript`, `Subscript`, `Size`), and the file was `Save()`d (see spec section 4S). No script in `tools/` generates them and they are not listed in `manifest.json`; the tests in `crates/pub-format/tests/script.rs`, `crates/pub-write/tests/script.rs` and `crates/kernpage-core/tests/script.rs` describe what each contains.

| File | Content |
|---|---|
| `script-plain.pub` | the text `H2O x2 One` with no superscript or subscript |
| `script-super.pub` | the same text with the `2` of `x2` set to superscript |

## KernPage fixtures (`samples/kern/`)
Only the tests of the `.pub` reader and writer open `.pub` files. Every other test (UI, export,
rendering) uses these `.kern` documents, converted from the samples above with the same code the
app runs when it opens a `.pub`. Regenerate with
`cargo test -p kernpage-core --test kern_fixtures -- --ignored regenerate_kern_fixtures`
(sources missing from the repo are skipped).

`showcase-newsletter.kern` is an original, made-up newsletter used for the README pictures, built by KernPage itself (`cargo run -p kernpage-core --example make_showcase -- out.pub`, then Save As .kern). All names, places and dates are invented. Project-owned, same licence as the code.

The research samples this project was developed against are kept in a private archive; only the files that tests need are in this repository.

## Printer name scrub
The project-made `.pub` samples (`blank.pub`, `hello.pub`, `two-boxes.pub`, `gen/*.pub`) carried the name of the printer they were saved on, stored as UTF-16LE. `tools/scrub_printer_name.py` overwrites it in place with the neutral name `Sample Printer`, padded with spaces to the same length, so file sizes do not change. Run it again after adding a sample saved on a real machine, giving the printer name to remove as the first argument (`python tools/scrub_printer_name.py "<printer name>"`); it never touches `tika-testPUBLISHER.pub` or `lo-fdo59355-1.pub`.
