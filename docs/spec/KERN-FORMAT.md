# The KernPage file format (.kern)

`.kern` is KernPage's own document format. It is open: anyone may read or write it. This page
describes version 1.0. The reference implementation is `crates/kernpage-core/src/native.rs`.

## What a .kern file is

A `.kern` file is a ZIP file. It holds a few small files:

| Entry | Contents |
|---|---|
| `mimetype` | The text `application/vnd.kernpage.document+zip` (ASCII, no line ending). It is the first entry and is stored without compression, so a program can recognise a KernPage file by reading its first bytes (the ZIP signature, then the name `mimetype`, then this text). |
| `manifest.json` | Says what the file is, which version of the format it uses, and what a reader needs. Deflated. |
| `document.json` | The whole publication as one JSON file: pages, shapes, text, tables, linked text, master pages and page-number sections. Deflated. It is never split into several files. |
| `assets/images/<sha256>.<ext>` | One file per distinct picture, named by the SHA-256 hash of its bytes (lower-case hex) and an extension (`png`, `jpg`, or `bin` for anything else). A picture used several times is stored once. Stored without compression, because pictures are already compressed. |
| `previews/thumbnail.png` | Optional preview of page 1. It is a convenience for file browsers, never authoritative, and readers may ignore it. KernPage does not write one yet. |

Other entries may appear in later versions. A reader ignores entries it does not know.

## manifest.json

```json
{
  "format": "KERN",
  "format_version": "1.0",
  "minimum_reader_version": "1.0",
  "document_id": "00112233445566778899aabbccddeeff",
  "generator": { "application": "KernPage", "version": "0.1.2" },
  "document": "document.json",
  "preview": "previews/thumbnail.png",
  "required_features": [],
  "optional_features": []
}
```

- `format` is always `KERN`.
- `format_version` is `major.minor`. This page describes `1.0`.
- `minimum_reader_version` is the oldest reader version that may open the file.
- `document_id` is a random 128-bit value (32 hex digits, or a UUID) made when a document is first
  saved. It stays the same across later saves of the same document.
- `generator` says which program wrote the file. It is information only; readers must not change
  how they read because of it.
- `document` names the document entry (`document.json`).
- `preview` names the thumbnail. Leave it out when there is none.
- `required_features` lists features a reader must understand to open the file correctly.
  `optional_features` lists features a reader may safely ignore.

Reader rules:

- Refuse a file whose `format_version` has a different major number, or whose
  `minimum_reader_version` is newer than the reader, with a clear message ("made by a newer
  KernPage").
- Refuse a file whose `required_features` names anything the reader does not know, and say which.
- Ignore unknown `optional_features`, unknown manifest fields, and unknown entries.
- A new minor version only adds things a reader can ignore. Anything else needs a new major number.

## document.json

The document is one JSON object. The top-level fields are:

- `name`: the title shown in the window.
- `pages`: the list of pages, each with its size, margins, shapes (drawn in order, the last one on
  top), the index of its master page, and whether it ignores the master.
- `default_line`: the default line colour, as `[red, green, blue]`.
- `stories`: linked text box chains.
- `masters`: master pages (shapes drawn behind the pages that use them).
- `sections`: page-number sections.
- `next_id`: the next unused shape number.

Conventions used throughout:

- **Units.** Every length and position is in EMU: 914,400 to the inch, 12,700 to the point.
  Font sizes are in points. Rotation is in degrees, clockwise.
- **Origin and axes.** Positions are measured from the top-left corner of the page, x to the right
  and y downward.
- **Text.** Text is a list of paragraphs; a paragraph is a list of runs, each with its own font,
  size, style and colour. A run's text has no paragraph ending; a `\n` inside a run is a line break
  inside the paragraph. A position in a text is a pair (paragraph index, character offset in that
  paragraph), both counted from 0, counting characters (Unicode scalar values), not bytes. A
  page-number field is a one-character run of `#` marked with a `field`.
- **Pictures.** A picture shape does not contain its bytes. Its `data` field is the entry name, for
  example `"assets/images/9f86d0...0a08.png"`, and the bytes are in that entry.

The field names follow the document model in `crates/kernpage-core/src/model.rs`, which is written
with serde. A field missing from the file gets a sensible default where one exists (an empty name,
black default line, no stories, no masters), and a field a reader does not know is ignored.

### Changing the model

Because the file is the serde form of the model, a careless change to the model changes the format.
Two tests guard this: one loads a committed version 1 file
(`crates/kernpage-core/tests/fixtures/v1-sample.kern`), and one fails if `document.json` for a fixed
sample document differs by a single byte (`v1-sample.document.json`). Any change to a field's name,
meaning or units, or removing a field, requires a new `format_version` (and this page updated);
the golden files are regenerated on purpose, never to quiet a failing test.

## Reading safely

A `.kern` file can come from anyone, so a reader must treat it as untrusted:

- Never extract entries to disk. Read what you need into memory.
- Refuse entry names that are empty, absolute, contain `..` or `.` parts, a backslash, a colon or a
  NUL, or are very long.
- Refuse two entries with the same name once case and slashes are normalised.
- Refuse encrypted entries and compression methods other than stored and deflate.
- Limit the number of entries (KernPage allows 4,096) and the total unpacked size (512 MiB, the same
  as the largest `.pub` file KernPage opens). Count the bytes actually unpacked; do not trust the
  sizes the ZIP headers declare.
- Every picture the JSON refers to must exist, and its SHA-256 must match its file name.
- Report damaged input as an error. Never crash.

## Writing

KernPage builds the whole file in memory and writes it to a temporary file in the destination's
folder. It reads that file back and checks it with the same reader, and only then renames it over
the destination, so a failed save leaves the old file untouched. The first save over an existing
file also keeps a `.bak` copy, as it does for `.pub` files. Files are written without timestamps,
so the same document and `document_id` always produce the same bytes.

## Compatibility promise

Every file written as version 1 will keep opening in future versions of KernPage.
