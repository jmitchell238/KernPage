# Publisher 2010 `.pub` format: independent specification (draft)

Written in our own words from observation of sample files, of the files Publisher 2010 and 2021 write after
known edits (made through its public automation interface), and of the public Microsoft Open Specifications
([MS-CFB], [MS-ODRAW], [MS-OLEDS], [MS-OLEPS]). **No Microsoft code is reproduced here.**
Implementations must be written from this document only.
The research samples this was written from are kept in a private archive; this repository carries only the `.pub` files that tests need (see `samples/SOURCES.md`), so some sample names cited below are not in it.
Status legend: ✔ confirmed on samples · ~ inferred, needs confirmation · ? unknown

## 1. Container ✔
- A `.pub` file is an OLE Compound File (public spec: [MS-CFB]). Magic `D0 CF 11 E0 A1 B1 1A E1`.
- Evidence: sample files. The root storage carries the Publisher class id (§6 directory table) and a CompObj stream, as the public OLE structured-storage format ([MS-CFB]) prescribes.

## 2. Stream layout ✔ (Publisher 2003–2010 style sample)
| Path | Purpose |
|---|---|
| `Contents` | main document structure (pages, shapes, positions) ~ |
| `Quill/QuillSub/CONTENTS` | text engine ("Quill") data ✔ |
| `Quill/QuillSub/\x01CompObj` | OLE class info for the text sub-storage |
| `Escher/EscherStm`, `Escher/EscherDelayStm` | Office drawing records (shapes, images) ~ |
| `Objects/` | embedded OLE objects ~ |
| `Envelope` | ? (empty in samples) |
| `\x01CompObj`, `\x03Internal` | OLE class info / ? |
| `\x05SummaryInformation`, `\x05DocumentSummaryInformation` | standard OLE property sets ([MS-OLEPS]) |

A minimal/older sample (`lo-fdo59355-1.pub`) has only `Contents`, `Objects/`,
`\x01CompObj`, `\x03Internal` → older/variant layout; must be handled separately. ?

## 3. Quill text stream (`Quill/QuillSub/CONTENTS`)
> **Item ids in Quill records (corrected 2026-11-07, follows the §4.2 header correction):** the property items in
> FDPC/FDPP/MCLD/PL records use the same 16-bit item header (id = low 11 bits, class = high 5 bits), and their
> second byte's low bits are normally `010`, so the real ids carry the 0x200 bit. Where this section names an id
> by its low byte (e.g. bold 0x02, size 0x0C, underline 0x1E, font 0x24, colour 0x44, alignment 0x04, space
> before/after 0x12/0x13, line spacing 0x34; MCLD width/height 0x04/0x05, gap 0x0A, columns 0x0B; PL entry 0x01),
> the full id is 0x200 + that value (bold 0x202, font 0x224, MCLD width 0x204, …). Byte sequences quoted in this
> section are unchanged.

### 3.1 Header and directory blocks ✔ (`gen/*.pub`, POI newsletter)
- Bytes 0..8: ASCII `CHNKINK ` (with trailing space).
- 0x0C: u16 **total number of chunks** in the whole directory ✔ (15 in `gen/linked.pub`,
  46 in the POI newsletter).
- The directory is a **chain of blocks**. The first block's 8-byte header is at 0x18; each
  block header is: u16 capacity in bytes (observed 504; ≤20 entries usable, see §3.9), u16 **entries in this
  block**, u32 **offset of the next block** in the stream (`FFFFFFFF` = last block) ✔.
  Entries (§3.2) follow the block header directly, so the first block's entries start at 0x20.
- Read every block until the next offset is `FFFFFFFF`; the entry counts of all blocks add up
  to the total at 0x0C ✔. Small files fit in one block; the POI newsletter needs three
  (19 + 20 + 7), and its `STRS`, `MCLD` and `PL` entries are only in the third block.
  A reader that stops after the first block loses every story boundary in such files.
- A chunk type can appear several times (instance index in §3.2): the newsletter has
  `FDPC` 0-2 and `FDPP` 0-2 and many `TOKN` instances. All instances must be read ~
  (for FDPC/FDPP each is one more 512-byte page of runs; apply them all in instance order).

### 3.2 Chunk directory entries ✔ (24 bytes each, little-endian)
| Offset | Size | Meaning |
|---|---|---|
| 0 | u16 | entry size, observed `0x0018` |
| 2 | 4 bytes | chunk type tag (ASCII, e.g. `TEXT`) |
| 6 | u16 | instance index (0,1,2… for repeated types) |
| 8 | u32 | ? (observed 1) |
| 12 | 4 bytes | chunk format tag (e.g. `PLC ` for several typed tables) |
| 16 | u32 | byte offset of chunk within the stream |
| 20 | u32 | byte length of chunk |

Each entry's size field is `0x0018`; the number of entries per block comes from the block header (§3.1) ✔.

### 3.3 Chunk types
| Type | Meaning | Status |
|---|---|---|
| `TEXT` | all story text, UTF-16LE; paragraphs end with `\r` (U+000D) | ✔ |
| `STSH` (×3) | style sheet | ~ |
| `FDPP` | paragraph formatting pages (512-byte page) | ~ |
| `FDPC` | character formatting pages (512-byte page) | ~ |
| `BTEP`/`BTEC` (`PLC `) | index tables over FDPP/FDPC pages | ~ |
| `FONT` | font table | ~ |
| `STRS` (`PLC `) | story boundaries within TEXT | ~ |
| `SYID`, `SGP `, `INK `, `MCLD`, `PL  ` | ? | ? |

**Chunk tag list.** Tags seen in sample files written by Publisher 2010 and 2021 are listed in the table above (§3.3); other tag values were never observed.
Publisher 2021 files use tags from the same set (`BDR `, `BTEC`, `BTEP`, `EOBJ`, `EXTC`, `EXTP`, `FDPC`, `FDPP`, `FLAG`, `FONT`, `FREE`, `FRCD`, `INK `, `LONG`,
`MBD4`, `MBDR`, `MCLD`, `MEF4`, `META`, `PIC4`, `PICT`, `PL  `, `PRNT`, `PRR `, `SGCD`, `SGDF`, `SGP `, `STRS`, `STSH`, `STTB`, `SYID`, `TCD `, `TEXT`,
`TOKN`, `WBD4`, `WBDR`, `WNPR`) ~ (only the subset in the table was seen in files; the rest are listed as candidates). Meanings of tags not in the table are not known
(`TCD `, `FLAG`, `FREE`, `LONG`, `META`, `EOBJ`, `PRNT`, `PRR `, `BDR `... ?). `FDPC` pages are 0x200 bytes each, with 45 property slots per run ~ (observed on files).

### 3.4 `STRS`: story lengths ✔ (3 samples) / ~ for the header fields
Layout observed: `u32 story_count`, `u32` (always 8), `u32` (always 0xFF), then
`story_count × u32` **story length in characters (UTF-16 units)**, then trailing bytes ?.
Stories occupy consecutive ranges of the `TEXT` chunk in that order. Samples: `hello`
1 story of 6 (`Hello\r`); `bold` 1 story of 17; `two-boxes` 2 stories of 17 + 17.
**Story number → range** ✔ (confirmed by `gen/apple-banana.pub`; the POI newsletter has 44 `STRS` stories and exactly 44 distinct story numbers, and the lengths sum to the `TEXT` length). Sort the distinct story
numbers used by the document's text boxes; the k-th smallest gets the k-th range. Story
numbers can have gaps (apple-banana uses 2 and 4; Tika 1 and 3). Range order = **creation
order** of the boxes, not their position on the page (Apple was created first but placed lower;
it owns the first range). "N − 2" is wrong.

### 3.5 `FDPC`: character formatting ✔ (structure) / ~ (most property ids)
A 512-byte page that divides the text into **runs** of identical character formatting.
| Offset | Size | Meaning |
|---|---|---|
| 0 | u16 | run count *n* |
| 2 | u16 | ? (1) |
| 4 | u32 | ? (0) |
| 8 | *n* × u32 | run **end** positions, as byte offsets in the Quill stream. The first run starts at the `TEXT` chunk's offset; each later run starts where the previous ended. |
| 8 + 4*n* | *n* × u16 | for each run: offset, within this 512-byte page, of its **property record** |

A property record is a block in the **same block/item encoding as `Contents` (§4.2)**.
Several runs may share one record. Character property items seen:
| id | type | Meaning |
|---|---|---|
| 2 | 0x0A flag | **bold** ✔ (present only on the bold run and its paragraph mark) |
| 0x12 | 0x22 (4 bytes) | language ID, 0x0409 = English (US) ~ |
| 0x3E | 0x22 (4 bytes) | second language ID (likely another script's) ~ |
| 0x37 | 0x0A flag | appears together with bold ? |
| 3 | 0x0A flag | **italic** ✔ (`gen/formatting.pub`) |
| 0x0C | 0x22 (4 bytes) | **font size in EMU**, 12,700 per point (152,400 = 12 pt) ✔; absent = default 10 pt ~ |
| 0x24 | 0x8A record | **font selection**: a record whose sub-records (ids 0–5, 0x14, 0x2A, 0x2C; one per writing system) each hold item 0 (type 0x18) = an index into the Quill `FONT` table (§3.7) ✔. Use the sub-record with id 0 for Latin text; absent = index 0 (default font) ~ |
| 0x1E | 0x12 (2 bytes) | **underline** ✔ (0x0401 = single underline; written by our tool, Publisher reads Font.Underline = 1; absent = none) |
| 0x44 | 0x8A record | **text colour** ✔: item 0 (type 0x22) = index into the Quill `PL` colour list (§3.8). Absent = default (black). `gen/colors.pub`: red 2, green 3, blue 4, orange 5, gray 6 (indices assigned in order of first use) |

Extra type codes seen in property records: **0x0A = flag (no payload)**, **0x12 = 2 bytes**,
**0x22 = 4 bytes**, **0x8A = nested record** (u32 length + items, like 0x88).
`BTEC`/`BTEP` index the FDPC/FDPP pages ~. All observations so far fit in one 512-byte page.

### 3.6 `FDPP`: paragraph formatting ✔ (structure, alignment)
Same 512-byte page layout as `FDPC` (§3.5); its runs end at paragraph ends. Paragraph
property items seen (`gen/formatting.pub`):
| id | type | Meaning |
|---|---|---|
| 4 | 0x12 (2 bytes) | **alignment**: absent = left, 1 = centred, 3 = right, 4 = justified ✔ (2 not seen) |
| 0x3C | 0x12 (2 bytes) | ? (seen only on the justified paragraph, value 80) |
| 0x12 | 0x22 (4 bytes) | **space before**, EMU ✔ (`gen/spacing.pub`: 18 pt = 228,600) |
| 0x13 | 0x22 (4 bytes) | **space after**, EMU ✔ (24 pt = 304,800) |
| 0x34 | 0x22 (4 bytes) | **line spacing**, packed: low 3 bits = rule, `value >> 3` = amount ✔ |

**Line spacing (item 0x34)** ✔ (`gen/spacing.pub`, checked against Publisher's own paragraph
positions in the manifest):
- rule **2** = a multiple of lines; the amount is in units where **152,400 = 1 line**
  (so 1.19 lines = 181,356; double = 304,800). One "line" is the font's natural line height
  (ascent + descent + line gap from the font's metrics; about 12.2 pt for Calibri 10 pt).
- rule **1** = exactly that many EMU per line (30 pt = 381,000).
- other rule values: ? (Publisher also offers "at least"; not sampled yet).

**Defaults** ✔: an empty paragraph record (just its 4-byte length) means "use the style".
For a new text box in Publisher 2010 that is: space before 0, **space after 6 pt**,
**line spacing 1.19 lines** (the value 181,356 with rule 2 is stored 33 times in `STSH1`).
How a style is chosen and read from the `STSH` chunks is ? — until that is decoded, use these
defaults. Layout: each paragraph takes space before + (lines × line height) + space after;
measured positions in `gen/spacing.pub` agree to within Publisher's 1.8 pt reporting step.
Whether space after the last paragraph in a frame counts toward the frame break is ~ (the
linked-box sample only fits 4 paragraphs per frame if it does not).

### 3.5b / 3.6b Quill property id sets (observed)
Property item ids seen in formatting records of files saved by Publisher 2021 (item ids with the 0x200 bit, see the note at the top of §3) fall into two sets.
Which record type each belongs to (character, paragraph, style) was **not** established
(both contain character and paragraph ids; A looks like the union):
- A: 0x200, 0x202, 0x203, 0x204, 0x205, 0x208, 0x20C, 0x20F-0x218, 0x21A-0x220, 0x222-0x224, 0x229, 0x22A, 0x22B, 0x22D, 0x232-0x240, 0x242-0x248, 0x24B, 0x24D-0x25E
  (exact: 200 202 203 204 205 208 20c 20f 210 211 212 213 214 215 216 217 218 21a 21b 21c 21d 21e 21f 220 222 223 224 229 22a 22b 22d 232 233 234 235 236 237 238 239 23a 23b 23c 23d 23e 23f 240 242 243 244 245 246 247 248 24b 24d 24e 24f 250 251 252 253 254 255 256 257 258 259 25a 25b 25c 25d 25e)
- B: 200 202 203 204 205 206 208 20a 20c 20d 20e 212 213 215 216 217 218 219 21a 21b 21c 21d 21e 220 221 222 225 22a 22c 22d 230 231 232 233 234 236 237 239 23a 23c 23d 23e 23f 241 242 243 244 245 246 248 249 24a 24b 24c 24d 24e 24f 250 251 252 253 254 255 256 257 258 25a 25d
Membership of the ids already used in §3.5/§3.6: bold 0x202, italic 0x203, alignment 0x204, size 0x20C, space before/after 0x212/0x213, underline 0x21E,
line spacing 0x234 and colour 0x244 are in **both** lists (seen as valid ids; their meanings remain as established from samples). The font id 0x224 is in A only (B has 0x225 instead).
Other ids (0x205, 0x208, 0x20F-0x218, ...): no names are known; meanings (small caps, strikethrough, kerning, tabs) were **not** resolved in this phase ?. **Indents and bullets/numbering are resolved in section 4R** (0x20C first-line, 0x20D left, 0x20E right indent, 0x206 list indent, 0x215 start, 0x257 list descriptor, 0x258 separator, 0x202/0x203 bullet size/font) ✔.

### 3.7 `FONT`: the text's font table ✔ (`gen/formatting.pub`)
| Offset | Size | Meaning |
|---|---|---|
| 0x00 | u32 | byte length of everything after this 20-byte header |
| 0x04 | u32 | font count *n* |
| 0x08 | 3 × u32 | ? |
| 0x14 | *n* × u32 | for each font: offset of its entry, measured from 0x14 |
| entry | u16 + UTF-16 | name length in characters, then the name (no terminator), then a u32 ? |
### 3.8 `PL`: the text's colour list ✔ (`gen/colors.pub`)
| Offset | Size | Meaning |
|---|---|---|
| 0x00 | u32 | entry count *n* (7 in the sample) |
| 0x04 | u32 | a value carried over from the program's in-memory list (40 here); not needed to read the list ✔ (text engine) |
| 0x08 | u32 | always written as 0 ✔ (text engine) |
| 0x0C | *n* blocks | one block per entry, in the §4.2 block/item encoding ✔ |

The `PL` chunk has the tag value 0x20204C50 ("PL  "), then the 12-byte header above and one block of items per entry; entries use the ordinary block/item encoding ✔ (sample files).

Each entry block: item 1 (type 0x22, 4 bytes) = **colour** as bytes R, G, B, *flag*;
item 6 (type 0x82 — a new type code: u32 length including itself, then raw bytes, like 0x80)
= ? (2 zero bytes). Observed entries: 0 = R,G,B,flag = 00 00 00 **08** (special, see below;
also has items 2–5), 1 = black, 2 = red, 3 = green, 4 = blue, 5 = orange (FF 80 00),
6 = gray (80 80 80). Text runs refer to these by index (§3.5 item 0x44).
- A *flag* byte of 0 means the R, G, B bytes are the colour ✔.
- Writing a new text colour: append a literal entry (`01 22 R G B 00` plus the `06 82 …` item, as
  existing entries have) and increase the list count; Publisher 2010 reads the colours back ✔
  (local evidence files, COM Font.Color.RGB). Default/black text has no item 0x44.
- Line spacing rules via COM: stored rule 1 (exact) reads as LineSpacingRule 4, stored rule 2
  (multiple) as 5 ✔.
- A *flag* byte of 8 (entry 0) means something other than a literal colour (likely a
  colour-scheme or "automatic" reference) — **? to be resolved by experiment (set the colour/scheme through COM and compare files)**.

Index 0 of the font table is the default font (Calibri in Publisher 2010). The character-format font index
(§3.5 item 0x24) indexes this table: 28 = Times New Roman, 32 = Arial, 33 = Courier New in
`gen/formatting.pub` ✔.

### 3.9 Stream layout rules (needed for writing) ✔ structure / ~ where noted
Observed in every sample (Publisher-made and POI):
- `TEXT` always starts at offset **0x200**; the directory blocks live in 0x18–0x1FF (and in
  extra blocks placed later in the stream when there are many chunks).
- Chunks follow each other **back to back** in this order: `TEXT`, `STSH` 0–2, then the
  `FDPP` pages, the `FDPC` pages, `SYID`, `SGP `, `INK `, `BTEP`, `BTEC`, `FONT`, any `TOKN`,
  `TCD `, `STRS`, `MCLD`, `PL  ` ~ (order as observed; whether Publisher requires it is ?).
- Every `FDPP`/`FDPC` page is 512 bytes and starts on a **512-byte boundary**; the gap before
  the first page is padding (zeros) ✔.
- The stream length is a multiple of 512 ✔.
- Directory entry fields ✔: +8 u32 = 1 in every entry seen; +12 = format tag (`PLC ` for
  `BTEP`/`BTEC`/`STRS`/`TOKN`; otherwise the chunk's own tag).
- **`BTEP` / `BTEC`** (format `PLC `) = index of the paragraph / character pages ✔: u32 page
  count n, u32 4, u32 0, then n + 1 u32 positions, then n u32 page offsets. The positions are
  stream byte offsets into `TEXT`: the first is 0 (= start of text), the last is the end of
  `TEXT` (0x200 + its length); page i covers positions [i, i + 1) and lives at page offset i.
  (bulletin: 1 page, positions 0 and 0x5B4, page at 0x4E00; POI newsletter `BTEC`: 3 pages at
  0x7600, 0x7800, 0x7A00.)
- Header u32 at **0x14** = total Quill stream length ✔ (0x5200 apple-banana, 0xB200 POI
  newsletter) — update it when the stream grows. u32 at **0x10** must be at least
  0x200 × (number of directory blocks) ✔: with 2–3 blocks and 0x200 left in place Publisher
  says it cannot open the file; 0x400/0x600 (2 blocks) and 0x600/0x800 (3 blocks) open.
  Writers use 0x200 × blocks (matches the newsletter's 0x600 with 3 blocks). The u16 at
  **0x0E** must stay 0x0300 ✔ (0 or 1 → "detected a problem in the file").
- A block holds at most 20 entries in practice ✔ (block 0 is exactly full at 0x200 with 20
  entries; writers use ≤19 in block 0 and ≤20 in later blocks, all accepted by Publisher).
- Writing formatting (verified by Publisher COM read-back, local evidence files) ✔:
  - `FDPP` (paragraph) runs must end at every `\r`; merging equal neighbouring paragraph runs
    into one run that spans several paragraphs makes Publisher hang on Open. `FDPC` (character)
    runs with equal records may be merged.
  - Alignment item 4: 1 = centre, 3 = right, 4 = justify; absent = left.
  - A new font is added by appending to the `FONT` chunk: grow its offset table by 4 bytes,
    shift existing offsets by +4, append u16 length + UTF-16 name + u32 0, update the count and
    the length field. Font item 0x24 written with sub-records 0–5, 0x2A, 0x2C reads back.
- Extra directory blocks (§3.1) sit **after the last chunk**, each 512-aligned and exactly
  8 + 504 = 512 bytes ✔ (newsletter: 0xAE00, 0xB000; the stream ends at the last block's end).
- Padding before the first FDP page is not always zero (apple-banana has leftover bytes) ✔;
  writing zeros there is accepted by Publisher ✔.
- **FDP page layout** ✔: header u16 n, u16 1, u32 0; n u32 run ends; n u16 record offsets;
  property records are packed **downward from the page end** (0x200), in order of first use,
  identical records shared. Each record starts with a u32 = its own length including itself;
  `04 00 00 00` = empty record. `BTEP`/`BTEC` interior positions = end of the last run on the
  preceding page ✔ (POI Sample.pub).
- Publisher gives each paragraph mark `\r` its **own `FDPC` run** with a leaner record (e.g.
  only items 0x12 and 0x3E) ✔; a writer that lets the mark share the text's run is still
  accepted by Publisher ✔.
- `STRS` trailer ✔: u32 0, a 10-byte block (`0a 00 00 00 00 22 05 00 00 00`), then
  (stories − 1) × `04 00 00 00` — it depends only on the story count.
- Replacing a story's text needs **no** change in `Contents` or `Escher` ✔ (Publisher 2010
  opened edited apple-banana and bulletin files with the new text, other boxes intact).
- So after changing text, a writer must rewrite `TEXT`, `STRS` (story lengths), the FDP pages'
  run ends, `BTEP`/`BTEC`, `TCD ` (tables), every later chunk's offset in the directory, and
  re-pad the FDP pages to 512-byte boundaries.

## 4. `Contents` stream (Publisher 2003–2010 variant)

Verified on `tika-testPUBLISHER.pub`: every byte of the 9,806-byte stream is accounted for
and all 36 indexed objects decode to their exact declared lengths with no unknown type codes.
Single-sample evidence; confirm on more samples.

### 4.1 Header (first 0x2C bytes) ✔
| Offset | Size | Meaning |
|---|---|---|
| 0x00 | u16 | signature `0xACE8` |
| 0x02 | u16 | offset of the first block (0x2C here) |
| 0x04 | u16 | format generation: 1000 in this sample; 300 in the older sample ~ |
| 0x08 | u32 | total stream length in bytes ✔ (both samples) |
| 0x1A | u32 | offset of the **object index** block ✔ |
| 0x1E | u32 | offset of the second top-level block ~ |

### 4.2 Blocks and items ✔
A **block** = `u32 length` (counting the 4 length bytes themselves), followed by **items**
until the length is used up. An **item** = `u8 id`, `u8 type`, then a payload.

**Size rule ✔ (checked on 32 of 32 generation-1000 sample files and on files saved by Publisher 2021).**
The item header is one little-endian 16-bit value (id in the low byte, type in the high byte); the **class** is the top five bits, `type >> 3` (0–31),
equivalently `type & 0xF8`. **Correction (seen on files saved by Publisher 2021, same on the 2010 files):** the
low three bits of the type byte are NOT flags: they are bits 8-10 of the **item id**. An item header is a
16-bit little-endian word: bits 0-10 = id (0..0x7FE; 0x7FF is an end sentinel), bits 11-15 = class.
So `0x0A 0x0A`-style bytes mean "class 1, id 0x2xx". Ids >= 256 exist (e.g. type byte 0x0A = class 1, id
bits 10:8 = 2; 0x5C = class 11, id bits 10:8 = 4). A reader should compute `id = byte0 | (byte1 & 7) << 8`.
Items inside a block are stored in **ascending id order** ✔ (every sample block), so a reader can stop at the first id above the wanted one.
A reader steps from item to item with a fixed **32-entry table indexed by the class**. Each entry gives the item's total size including
its 2-byte header, plus a yes/no "a length follows" mark. When the mark is set, the 4 bytes
right after the header are a `u32 length` that counts itself, and the item occupies
2 + that length bytes.

| Class | Type byte range | Payload after the 2-byte header |
|---|---|---|
| 0 | 0x00–0x07 | none: boolean **false** ✔ |
| 1 | 0x08–0x0F | none: boolean **true** ✔ (boolean items have no payload: class 0 is false, class 1 is true) |
| 2 | 0x10–0x17 | 2 bytes (16-bit value; unsigned 16-bit values use class 3 below) |
| 3 | 0x18–0x1F | 2 bytes: **u16** ✔ |
| 4 | 0x20–0x27 | 4 bytes: **u32** ✔ |
| 5 | 0x28–0x2F | 8 bytes |
| 6 | 0x30–0x37 | 12 bytes (not yet seen in files) |
| 7 | 0x38–0x3F | 16 bytes |
| 8 | 0x40–0x47 | 20 bytes (not yet seen in generation-1000 files) |
| 9 | 0x48–0x4F | 24 bytes (seen holding a Windows font-signature structure) |
| 10 | 0x50–0x57 | 32 bytes (not yet seen) |
| 11 | 0x58–0x5F | 4 bytes |
| 12 | 0x60–0x67 | 8 bytes (not yet seen in generation-1000 files) |
| 13, 14 | 0x68–0x77 | 4 bytes |
| 15 | 0x78–0x7F | none (empty list slot) |
| 16 | 0x80–0x87 | length-prefixed raw bytes |
| 17, 19 | 0x88–0x8F, 0x98–0x9F | length-prefixed nested block of items (a record) |
| 18, 20 | 0x90–0x97, 0xA0–0xA7 | length-prefixed nested block used as a list (each element has id 0, then its own type) |
| 21, 22 | 0xA8–0xB7 | none (not yet seen) |
| 23 | 0xB8–0xBF | 4 bytes (holds an offset into this stream) |
| 24–27 | 0xC0–0xDF | length-prefixed raw bytes (0xC0 seen holding UTF-16 text) |
| 28 | 0xE0–0xE7 | none: this class means "no value" (an empty slot; seen as runs of empty list slots) |
| 29 | 0xE8–0xEF | none (not yet seen) |
| 30 | 0xF0–0xF7 | none; treated specially, apparently as a continuation marker ? |
| 31 | 0xF8–0xFF | none (not yet seen) |

The same 32-entry size table holds for files saved by Publisher 2021 ✔ (all 2021 sample files decode with it; no new item classes seen). Class-30 items behave as a placeholder (empty slot) ✔ (seen in 2010 and 2021 files).
In list blocks (classes 18/20) the element count is read instead of comparing ids ✔.

Every type byte therefore has a defined size; a reader never needs to reject a type for being
unknown. It should still stop with a clear error when a size runs past the end of its block.
Classes 18 and 20 are list containers ~ (seen in sample files). Real-world files use flagged forms such as 0x01, 0x02, 0x06, 0x07, 0x0A, 0x12, 0x1A,
0x22, 0x23, 0x5C, 0x82 and 0x8A.

Generation-300 files (Publisher 98/2000) still fail with this table (a block's items end a few
bytes off its stated length), so something else differs there too ?.

Item ids are local to the enclosing record; their meaning depends on the record's kind.
Integers are little-endian.

### 4.3 First block (document header record) ✔ structure / ~ meaning
Observed: id 1 = `0xACE8` again; id 8 = stream length; id 9 (0xC0) = original file path
(UTF-16); id 10 (0xB8) = offset of the object index; id 11 (0xB8) = offset of the second
top-level block.

### 4.3b Document header object (object 256): item ids and value kinds (observed)
Source: items present in the document-header block of files saved by Publisher 2010 and 2021; value kinds are the item classes seen (72 ids). Kinds: **B** = boolean (type class 0/1; default as shown), **U32** = u32 (class 4),
**U16** = u16 (class 3), **N** = non-scalar (nested record, list, blob or string). Ids not listed were never seen. Ids not present in every sample file are unconfirmed ~.
Defaults are what Publisher appears to assume when the item is absent. (The item 0x3C/u32/default 0 fact was seen in 2010 and 2021 files.)
Defaults are what Publisher assumes when the item is absent. (2010 equivalents not re-read; the item 0x3C/u32/default 0 fact was already seen in 2010.)

| id | kind | default | | id | kind | default | | id | kind | default |
|---|---|---|---|---|---|---|---|---|---|---|
| 0x01 | U32 | 0 | | 0x20-0x22 | N | | | 0x3C | **U32 (save counter, §4M)** | 0 |
| 0x02-0x04 | N | | | 0x23 | U32 | 0 | | 0x3D | B | false |
| 0x05-0x0E | B | false | | 0x24, 0x25 | B | false | | 0x3E | B | false |
| 0x0F | U16 | 1 | | 0x26 | U16 | 0 | | 0x3F | N | |
| 0x10, 0x11 | U32 | 0 | | 0x27-0x2B | N | | | 0x40 | B | false |
| 0x12-0x14 | N | | | 0x2C, 0x2D | U16 | 0 | | 0x41 | U32 | 0xFFFFFFFF |
| 0x15 | U32 | 0x6F9F0 | | 0x2E | B | false | | 0x42 | N | |
| 0x16-0x1B | N | | | 0x2F-0x31 | N | | | 0x43 | B | false |
| | | | | 0x32 | U32 | 0 | | 0x44 | N | |
| | | | | 0x33 | U32 | 3 | | 0x46 | U32 | 0 |
| | | | | 0x34 | B | false | | 0x47, 0x48 | B | false |
| | | | | 0x35 | N | | | 0x49 | N | |
| | | | | 0x36 | B | false | | 0x4A, 0x4C, 0x4D | B | false |
| | | | | 0x37-0x39 | B | true | | | | |
| | | | | 0x3A, 0x3B | N | | | | | |

(No meaning is known for the individual ids; names would need the UI/COM side. Id 0x01 is the only u32 at the start, matching the 0xACE8 signature convention.)

### 4.3c Shape object (Contents kind 1, also line kind 32): item ids, value kinds, defaults (observed)
Source: the items present in shape objects of files saved by Publisher 2021 after known edits (made through COM) and in the 2010 samples. Observed rule: an item is present in the file only when its value differs from a default; that explains which items appear in files (e.g. item 183 and 52 are present with value 0 because their default is 1; item 4 is present
with 3 or 0 because its default is 1). Value kinds: F = flag (class 0/1), U16 = 2-byte value (class 2), U32 = 4 bytes, V8 = 8-byte value (class 5), REF = object
reference (24-bit object number), L = list.
| id | kind | default | meaning (my words) | conf |
|---|---|---|---|---|
| 1, 2, 3 | F | 0 | three object flag bits (bits 0-2 of the object's flag word); item 2 is true in every box/shape/line seen | ~ / ~ meaning |
| 4 | U16 (3-bit field) | 1 | **text wrap type** (§4K, ✔ 2021 edit test): 0 none, 2 square, 3 tight (text box default), absent = 1 top and bottom; values above 3 make 2021 reject the file; upper byte is noise. 0 is also what lines carry (stored whenever not 1) | ✔ 2021 edit test |
| 5 | U16 (3-bit field) | 0 | second small enum (bits 6-8 of the flag word); with item 4 absent, **4 = wrap through** (§4K, ✔ 2021 test); other values read as top and bottom | ✔ 2021 edit test |
| 7 | F | 0 | flag bit 10 | ~ |
| 9 | U16 | 0 | 16-bit field of the object | ~ |
| 12, 13 | V8 | written always | two 8-byte values, always zero in our samples | ~ / ? meaning |
| 14 | REF | 0 | reference to another object (24-bit number) | ~ |
| 15 | U32 | 0 | 32-bit field of the object | ~ |
| 16, 18 | F | 0 | flag bits 9 and 11 | ~ |
| 39 (0x27) | U32 | 0 | **story number** the text box shows (plain number, not an object reference) | ~ (plain u32) / ✔ samples |
| 40 (0x28) | U32 | 0 | **position in a linked chain** (only written when non-zero) | ~ (u32 field) / ~ meaning |
| 41-45 (0x29-0x2D) | F x5 | 0 | five overlap/wrap bookkeeping flags kept on the shape data; **43 = a line crosses this box, 45 = a shape overlaps this box** (experiments in §4L.2/§4N.2) | ~ (five flags) / ~ meaning |
| 47-50 (0x2F-0x32) | F x4 | 0 | four flag bits (bits 0-3 of a second flag word) | ~ |
| 52 (0x34) | U32 | **1** | **text-frame indicator**: 1 (default, not written) for a text box; **0 written for a shape that has no story** | ~ (default 1) / ~ meaning |
| 53 (0x35) | U32 | 0 | 32-bit field | ~ |
| 56, 57 (0x38, 0x39) | U32 | 0 | two 32-bit fields | ~ |
| 58, 59 (0x3A, 0x3B) | REF | 0 | two object references | ~ |
| 62, 63 (0x3E, 0x3F) | U16 (2-bit fields) | 0 | two 2-bit enums | ~ |
| 64 (0x40) | F | 0 | flag | ~ |
| 70 (0x46) | U32 | not written when the list is empty | **number of objects in the overlap list** (count of the list below) | ~ |
| 71 (0x47) | L of REF | | **overlap list**: the objects (24-bit numbers, each stored as an id-0 element with 4-byte value) that overlap this box; written only when non-empty | ~ / ~ purpose (wrap cache) |
| 72, 74 (0x48, 0x4A) | U32 | 0 | two 32-bit fields | ~ |
| 73 (0x49) | F | 0 | flag | ~ |
| 170 (0xAA) | U32 | 0 | **width in EMU** | ~ |
| 171 (0xAB) | U32 | 0 | **height in EMU** | ~ |
| 179, 180 (0xB3, 0xB4) | U32 | 0 | two 32-bit fields of the base object | ~ |
| 181, 182 (0xB5, 0xB6) | U32 | 0 | two more 32-bit fields | ~ |
| 183 (0xB7) | U32 | **1** | last of the five-field base group; written with value 0 in every file we saw (default 1 is not written) ; meaning unknown | ~ (default 1) / ? meaning |
Notes:
- Items 1-18, 179-183 are the **common object header**; a **line (kind 32)** carries only these (observed items 2, 4, 12, 13, 183), consistent with the files ✔.
- Further items depend on the object's own kind: **kind 49** adds items 92 (0x5C, REF), 172 (0xAC) and 93 (0x5D); **kind 33** adds items 80-82 (0x50-0x52); other shape classes use a third group (ids 0x65-0x6B, 0x71, 0x83). These were not decoded further (ids not seen in a sample file are ?).
- Order in file is ascending id (§4.2).

### 4.4 Object index ✔ structure / ~ meaning
At the header offset 0x1A. A block with items 1 and 2 (4 bytes each) and item 3, a list
(0x90). Each list slot is either empty (0x78) or a record (0x88); the slot position is the
object's **number**. Record items:
| id | Meaning |
|---|---|
| 2 | object **kind** ~ |
| 4 | offset of the object's data in this stream ✔ |
| 5 | number of the **parent** object (objects form a tree) ~ |
| 6 | 16-bit value increasing through the index; sequence key ? |
| 8 | flag ? |
| 11 | small count ? |

Every object's data is itself a block (starts with its own u32 length) ✔.

### 4.5 Object kinds seen
Confirmed by controlled samples (`blank`, `hello`, `hello-moved`, `two-pages`, saved by
Publisher 2010 on 2026-11-04 with exactly one change between each):
- **Kinds 67 + 96 + 119 = one page-like surface** ✔. Adding one page adds exactly one of
  each; 67 is the parent of the 96 and the 119. **Correction (found while implementing):** not
  every kind-67 object is a document page. A 1-page document has six (one is "Master Page A";
  four are unexplained ?). **Document pages = kind-67 objects whose item 6 (8 bytes) is
  non-zero**, ordered by the **order of the ids in object 256 item 2** (CHANGED, see 4P.1: the low 32 bits of item 6 are not the page order; the old rule only held for files that never had a page added or moved).
- A shape's page is found by following its `Contents` parent chain up to a page ~ (verified on
  1-page samples only). Escher drawings could not be matched to pages (always 3 drawings).
- **Kind 1 = a shape on a page (here: a text box)** ✔. Adding a text box adds one kind-1
  object. Its item **170 = width** and item **171 = height**, in EMU (914,400 per inch) ✔.
- **Kind 108 = a font table** ✔: item 2 is a list (0xA0) of records; in each record item 4
  (0xC0) = font name as UTF-16, plus PANOSE bytes (0x80) and font signature (0x48).
- **Page size** ✔ (corrected twice): the **kind-68 object (document settings)** contains a
  nested record (item 18, type 0x88) whose items 1 and 2 (type 0x20) = page **width** and
  **height** in EMU; the kind-76 object has the same pair nested in its item 3. Practical rule:
  the first record, searched depth-first, whose items 1 and 2 are both 4-byte integers. Verified: `gen/legal.pub` = 8.5 × 14 in, Tika = A4 (8.27 × 11.69 in), others Letter.
  The kind-75 object always says Letter (it kept the template's/printer's paper size even
  in the Legal and A4 files), so it must **not** be used for the page size. Rule: kind 68,
  else kind 76, else Letter.
- The Quill `TEXT` chunk is **absent** when the document has no text (`blank.pub`) ✔ → treat
  as empty text.
- All other kinds: meaning not yet assigned ?.

Type table confirmed on all 5 modern samples (0 decode failures), plus type 0x00 = flag
with no payload (seen in the document-settings object, kind 68).

## 4A. Shapes stream (`Escher/EscherStm`) ✔
Uses Microsoft's **publicly documented** Office Drawing record format ([MS-ODRAW]):
each record has an 8-byte header (u16 version+instance, u16 record type, u32 length).
Publisher-specific layout, observed on 4 samples (each decodes exactly to the last byte):
1. One drawing-group container (record type 0xF000).
2. Then, repeated per drawing: a **u32** (values seen: 1, 2, 0 / 2, 1, 0; purpose ?), followed
   by one drawing container (record type 0xF002). 3 drawings in every sample (pages and
   master pages; exact mapping ?).

Inside a shape, three standard [MS-ODRAW] record types carry Publisher's own data, encoded
with the **same block/item encoding as `Contents` (§4.2)**:
| Record | Payload (Publisher item block) |
|---|---|
| 0xF010 client anchor | items 1, 2, 3, 4 = **left, top, right, bottom**, signed 4-byte EMU, measured from the **page centre** ✔ |
| 0xF011 client data | item 1 = the **`Contents` object number** of this shape ✔ |
| 0xF00D client textbox | item 1 = text story number ~ (2 and 3 in our samples; but 1 and 3 in the Tika sample, so the story-number → `STRS` range mapping in §3.4 is not settled) |

Worked example (`hello.pub`, Letter page, centre = 4.25 in, 5.5 in): anchor = −3.65, −4.93,
+0.01, −3.41 in → on the page, x 0.60–4.26 in, y 0.57–2.09 in (the user put it at top left);
width 3.66 in = `Contents` object 296 item 170 ✔.

Drawings: every sample has several 0xF002 drawing containers (one per page/master).
Shapes from **all** drawings must be collected; a drawing may hold shapes whose records are
standard [MS-ODRAW] property tables (0xF00B) and others, which must be skipped by record
type, never decoded as Publisher items. Only the three client records above carry Publisher
item blocks.

## 4C. Pictures ✔ (`gen/picture.pub`; all records are public [MS-ODRAW])
- The drawing-group container (0xF000) holds a picture store container (0xF001) with one
  entry record (0xF007) per picture. In each entry: byte 0 = picture type (6 = PNG; the
  standard [MS-ODRAW] values apply, e.g. 5 = JPEG), and at entry-payload offset 20:
  u32 size, u32 reference count, u32 **offset in `Escher/EscherDelayStm`**.
- At that offset in `EscherDelayStm` is a standard picture record: 8-byte record header,
  16-byte identifier, 1 tag byte, then the **original image file bytes unchanged** (the PNG
  starts 25 bytes in; it is byte-identical to the inserted file).
- A picture shape's property table (0xF00B) has property 0x0104 = **1-based index** into the
  picture store. Its position comes from the usual client anchor (§4A), its object number
  from client data.

## 4C.1 Writing pictures (insert, share, replace, delete, crop, move) (real Publisher 2021 COM then Save(); our own tiny images and samples; evidence = `samples/gen/picture-*.pub` and their `manifest.json` entries; 2026-11-14)
Publisher 2021 (16.0) was driven through COM: a copy of `samples/blank.pub` was opened, edited and `Save()`d (never `SaveAs`). Cases (each a fresh copy of the blank; the first group saved once,
the second group is a second save of the file named): `picture-new` (`Shapes.AddPicture(png, LinkToFile=false, SaveWithDocument=true, 100,150,120,90)`, a 4x3 red PNG) | `picture-two` (the red PNG, then a 5x5 blue
PNG at 300,300,100x100) | `picture-same-twice` (red PNG twice) | `picture-jpeg` (8x6 JPEG) | `picture-big` (40x30 PNG) | a no-op open+Save of the blank as baseline. Second group: `picture-replaced`
(`picture-new` + `Shape.PictureFormat.Replace(blue)`) | `picture-deleted` (`Shape.Delete()`) | `picture-two-deleted-first` | `picture-same-replaced-second` | `picture-cropped` (`picture-big` +
`PictureFormat.CropLeft/Top/Right/Bottom`) | `picture-moved` (Left/Top/Width/Height set). `picture-over-text` = `formatting.pub` + a picture overlapping its text box. Two more cases are described but not shipped
because they embed a local path (a linked picture, an auto-sized insert); `picture-same-deleted-first` and a JPEG replace were analysed but not shipped. Marks: ✔ seen in 2 or more files, ~ seen once, ? unknown.
**COM calls that worked:** insert = `Shapes.AddPicture(FileName, LinkToFile, SaveWithDocument, Left, Top, Width, Height)`; replace ("Change Picture") = **`Shape.PictureFormat.Replace(FileName)`** ✔ (it worked on the first
attempt; `ReplaceEx` also exists and was not needed). Every picture reports `Shape.Type = 13` (pbPicture), unlike drawn shapes (6) and text boxes (17).

### 4C.1.0 What changes (streams)
Only these streams change, the same way in every insert ✔ (sizes in bytes):
| Stream | blank (after one save) | + 1 picture | + 2 pictures | same PNG twice | JPEG | replaced (2nd save) | deleted (2nd save) |
|---|---|---|---|---|---|---|---|
| `Escher/EscherStm` | 1056 | 1480 | 1898 | 1852 | 1484 | 1520 | 1056 |
| `Escher/EscherDelayStm` | 0 | 98 | 200 | 98 | 693 | 102 | 0 |
| `Contents` | 9594 | 9800 | 9980 | 9992 | 9806 | 9816 | 9618 |
`Quill/QuillSub/CONTENTS` keeps its size and meaning (only the usual volatile bytes differ, as in §4N.3; leave it alone); `\x05SummaryInformation` changes only in its thumbnail (same size); every other
stream is untouched ✔. Delay-stream size = sum over live entries of (8 + 16 + 1 + file size).

### 4C.1.1 Picture store in the drawing-group container (0xF000) ✔
- The picture store container **0xF001** (record version 15, **instance = number of 0xF007 entries**) is the child of 0xF000 that comes **right after the 0xF006 header record
  and before the 0xF00B default-property table** ✔. A document with no picture has **no 0xF001 at all** ✔ (the blank has none; `picture-deleted` has none again).
- Entry = record 0xF007 (version 2, **instance = picture type**), 36-byte payload ✔ (every entry in 12 files):
  `type_win u8 | type_mac u8 | uid [16] | tag u16 = 0x00FF | size u32 | ref_count u32 | delay_offset u32 | 4 bytes`
  - `type_win` = `type_mac` = the instance = **6 for PNG, 5 for JPEG** (standard [MS-ODRAW] blip types) ✔.
  - `uid` = **MD4 of the complete original file bytes** ✔ (checked on the red PNG, the blue PNG and the JPEG with a hand-written MD4; it is not MD5/SHA-1 and not a hash of pixels).
  - `size` = length of the matching delay-stream record **including its 8-byte header** = 8 + 17 + file size (98 = 8+17+73, 693 = 8+17+668) ✔.
  - `ref_count` = how many picture shapes use this entry ✔ (1; 2 for the same PNG twice).
  - `delay_offset` = byte offset of that record in `EscherDelayStm` ✔ (0 for the first; 98 for the second in `picture-two`).
  - Last 4 bytes: unused, name length (always 0), then two bytes that are different garbage on every save (0B12, C5A6, 760E ...). A writer should write **00 00 00 00** ~ (zeros were tried by patching
    `picture-new`: Publisher 2021 opened it and read the picture back; saving it again was not tried).
  There is no name field (name length 0) ✔.
- Entry order = creation order; the picture shape's property 0x0104 is the **1-based index** into this list ✔ (§4C).
- **Same bytes share one entry** ✔: `picture-same-twice` has one entry (ref_count 2, one delay record) and both shapes carry pib = 1. A writer should keep a uid -> entry map and bump `ref_count`.
- The 0xF000 length grows by 36 per new entry, plus 8 (the 0xF001 header) when the first entry is created; `EscherStm` total grows by the same amount plus the shape container below ✔ (blank 1056 -> 1480 =
  +52 +372).

### 4C.1.2 `EscherDelayStm` ✔
- Concatenation of one record per **live** entry, in store order, no padding: `ver/instance u16 | type u16 | length u32 | uid [16] | tag byte 0xFF | original file bytes unchanged`. `length` = 17 + file size
  (so an entry's `size` field is length + 8).
- PNG: header bytes `00 6E 1E F0` (instance 0x6E0 = PNG with one uid, type 0xF01E) ✔; JPEG: `A0 46 1D F0` (instance 0x46A, type 0xF01D) ✔. The record's uid equals the entry's uid.
- **Appending** (`picture-two`): the second record starts at the end of the first (offset 98) ✔. **Publisher rewrites the whole stream on save**: after a delete or replace only the live entries remain and
  their `delay_offset`s are recomputed from 0 (`picture-replaced`: one 102-byte record, offset 0; `picture-two-deleted-first`: 102 bytes, offset 0) ✔. Whether a writer may leave stale (dead) records in the stream
  is untested ? (experiment: write one, open in Publisher 2021, save, compare).
- Other image formats (BMP, GIF, TIFF, EMF, WMF): not tested ? (Publisher may convert; experiment: AddPicture of each, read the entry type and the delay record header).

### 4C.1.3 Header record 0xF006 and the drawing's 0xF008 (same rules as §4N.1 / §4L.1) ✔
- New picture shape: spid = the drawing's last spid + 1 (1025 = 0x401 in the blank's page-1 drawing); 0xF008: `cspids += 1`, `lastspid = spid`; 0xF006: `cspSaved += 1` (5 -> 6) and the FIDCL
  count of this drawing += 1 (1 -> 2); max spid, `cidcl`, `cdgSaved` unchanged ✔ (picture-new, -two, -same-twice, -jpeg, -over-text).
- **Delete** (`picture-deleted`, `picture-two-deleted-first`): `cspSaved -= 1`, `cspids -= 1`, but **`lastspid` and the FIDCL count stay raised** ✔ (the byte diff of `picture-deleted` against the blank is exactly two
  bytes in this region: FIDCL count 1 -> 2 and lastspid 0x400 -> 0x401). So spids are never reused.

### 4C.1.4 The picture shape container (0xF004, last child of the page's 0xF003) ✔
Same placement as every added shape (§4N.1: appended as the last child of the single group container 0xF003 whatever the page). Children in order: FSP 0xF00A, property table 0xF00B, tertiary table
0xF122, client anchor 0xF010, client data 0xF011. **No client textbox (0xF00D).**
- **FSP**: `spid u32 | flags u32 = 0x0A00`, **instance 1 = rectangle** (a picture is a rectangle filled by a blip, not shape type 75) ✔. No flip/rotation bits.
- **Property table 0xF00B**, instance = property count = **26** (28 with two crops, 30 with four), 6 bytes per property (`id u16, value u32`, ascending ids), followed by the **complex data** of
  property 0x0105. A property id u16 = id | 0x4000 when the value is a blip id, | 0x8000 when complex ✔. Values (hex):
  | id | value | meaning |
  |---|---|---|
  | 0x007F | 0x00800080 | protection flags (constant) |
  | 0x0081..0x0084 | 0x00008EE0 (36576) | text insets (as every shape, §4N.1) |
  | 0x0100 / 0x0101 / 0x0102 / 0x0103 | 16.16 fixed point | **crop** from top / bottom / left / right as a fraction of the whole image (0x4000 = 25 %); a zero crop is **omitted** ✔ (picture-cropped) |
  | **0x4104** (id 0x0104, blip flag) | picture-store index (1-based) | the picture ✔ |
  | **0xC105** (id 0x0105, blip+complex flags) | byte length of the name below | picture name |
  | 0x0106 | 1 | flags; **absent after Replace** ✔; 9 for a linked picture ~ |
  | 0x0181, 0x0183 | 0x08000007 | fill colours (scheme), as a rectangle |
  | 0x01BF | 0x00100000 | not filled |
  | 0x01C0 / 0x01C2 | 0x08000000 / 0x08000007 | line colours |
  | 0x01CB | 25400 | line width 2 pt (a plain rectangle omits it, §4N.1) |
  | 0x01FF | 0x00080000 | no line |
  | 0x0201 | 0x08000004 | shadow colour |
  | 0x0285, 0x02CB, 0x02CC, 0x02CE, 0x02CF | 0 | zero-valued (constant) |
  | 0x033F | 0x00100010 | group/shape booleans (constant) |
  | 0x0384..0x0387 | 0x00008EE0 | wrap distances |
  Complex data of 0x0105: the picture's **file name without its extension, UTF-16LE with a terminating NUL** (`red4x3` -> 14 bytes; the property value is that byte length) ✔ (picture-new, -two, -jpeg: green8x6 = 18).
  After **Replace** the name is the new file's name (blue5x5, 16 bytes) and 0x0106 is gone ✔. The name is cosmetic (nothing else reads it) ~.
- **Tertiary table 0xF122**: instance 18 (108 bytes) ✔, constant in every picture: 0x01FF = 0x00400000; 0x0295..0x029A = 0 (six); 0x0540 = 0x08000000, 0x0542 = 0x08000007, 0x0580 = 0x08000000,
  0x0582 = 0x08000007, 0x05C0 = 0x08000000, 0x05C2 = 0x08000007, 0x0600 = 0x08000000, 0x0602 = 0x08000007, 0x0640 = 0x08000000, 0x0642 = 0x08000007; 0x06FF = 0x00020002.
  (A rectangle's 0xF122 has 12 properties, 0x01FF = 0x00400040 and no 0x0295..0x029A.)
- **Client anchor 0xF010** and **client data 0xF011**: identical to §4L.1 / §4N.1 (`1c000000 | 01 20 L | 02 20 T | 03 20 R | 04 20 B`, EMU from the page centre; client data = the picture's Contents object id) ✔.
  After a crop the anchor is the **cropped frame**; after Replace the anchor changes too (4C.1.6).
- Container length (without the 8-byte header) = **350 + name bytes** (364 for a 6-character name), +6 per crop property, -6 when 0x0106 is absent ✔. 0xF003, 0xF002 and `EscherStm` grow by that + 8
  (372 for `red4x3`).
- Compared with the 4N rectangle, the picture adds: 0x007F, 0x0100-0x0103 (only if cropped), 0x0104, 0x0105 (+ complex data), 0x0106, 0x01CB (here with the 2 pt value), 0x0285, 0x02CB, 0x02CC, 0x02CE,
  0x02CF, 0x033F; and has a different 0xF122 (above).

### 4C.1.5 `Contents` ✔
- **Two new objects per picture**, taking the next two free ids N and N+1 (in the blank: 296, 297; in `bulletin.pub`: 305, 306). If the document has a **font table (kind 108, always last)** it is the slot that is
  taken: the picture gets N, the info object N+1 and the font table moves to **N+2**, and object 282 item 4 follows ✔ (`picture-over-text`: 297, 298, font table 299, 282[4] 297 -> 299; same rule as §4L.2).
  A second picture gets N+2 and N+3.
- **Picture object = kind 1** (the same kind as a text box or rectangle), body **50 bytes** ✔ (identical for every picture; only 170/171 differ):
  `32000000 | 02 08 | 0c 28 00*8 | 0d 28 00*8 | 34 20 00000000 | 3a 70 <u32 id of the info object> | aa 20 <width EMU> | ab 20 <height EMU>`
  i.e. items 2 = true, 12 = 0, 13 = 0, **52 = 0**, **58 = reference to the picture-info object (kind 102)**, 170 = width EMU (120 pt = 1524000), 171 = height EMU. Compared with the 54-byte rectangle (§4N.2) it has
  **no item 4 and no item 183, and adds item 58**. Items 170/171 are written for a new picture and follow Move/Resize/Crop ✔ (moved: 60 x 45 pt; cropped: the 45 x 18 pt frame), but are **not updated by Replace**
  (left at the old 120 x 90 pt while the Escher anchor became 90 x 90) ✔. The anchor is what Publisher displays; write both consistently. (Note: `pub-format` currently reads the size from 170/171, which is stale
  after a Replace.)
- **Picture-info object = kind 102**, body ✔ (embedded picture):
  `len u32 | 02 88 <len> { 02 20 <file size u32> | 03 88 <len=16> { 01 20 <FILETIME low u32> | 02 20 <FILETIME high u32> } } | 03 c0 <len u32 incl. itself> <file name WITH extension, UTF-16LE + NUL>`
  - item 2 / sub-item 2 = the source file's size in bytes (73 = red PNG, 668 = JPEG) ✔;
  - item 2 / sub-item 3 = the source file's **last-write time as a Windows FILETIME**, low dword then high dword ✔ (matched the file's mtime to within one 100 ns tick for all three images). A writer can store
    the real mtime or the insertion time ✔, but **never 0**: Publisher 2021 refuses a file whose picture-info FILETIME is zero
    (both dwords) with "Publisher cannot open the file" ✔ (found by swapping single fields between our file and `picture-new`, 2026-11-14).
    A zero store-entry tail and the 0x018A client anchor/data instance (instead of the 0x01AA Publisher stamps on save) are both accepted ✔;
  - item 3 = file name with extension (`red4x3.png`) ✔. A **linked** picture (`AddPicture(..., LinkToFile=true, SaveWithDocument=false)`) instead has item 1 = true and item 3 = the **full path** (252 bytes there),
    pibName holds the same full path and 0x0106 = 9, yet the image bytes were still embedded in the delay stream ~ (one case; not shipped because it contains a local path).
- Index entries (30 bytes each, §4N.2 form): picture `{2: 1, 5: parent page object, 6: 258, 11: 1}` (a text box has 6 = 259, 11 = 22; a drawn shape 258/22), info `{2: 102, 5: the picture object, 6: 258, 11: 1}` ✔. The
  index block's items 1 and 2 (slot count, last slot) += 2.
- Page object (kind 67): item 1 (child count) += 1 and the **picture id** (not the info id) appended to the item-2 child list ✔ (page body 96 -> 114 for the first child; 120 for two).
- **No story registries, no Quill change** ✔ (like shapes, §4N.2/4N.3).
- **Overlap bookkeeping** (~ one case, `picture-over-text`): a new picture overlapping an **older text box** gives that box items **70 = 1** and **71 = [picture id]** (`00 68 <u32>`), inserted between item 39
  and item 170, but **not item 45** (which §4L.2 adds when the overlapping object is a text box). A picture that overlaps nothing changes nothing ✔ (picture-new, `bulletin` cases). Whether the items are needed
  when absent is untested ?.
- Save counter (object 256 item 60) +1 per Publisher save ✔ (§4M); header 0x08 / 0x1A / 0x1E, the index offsets and all later bodies shift (rebuild the stream).
- Auto-sized insert (`AddPicture(file, 0, -1, Left, Top)` without width/height): the frame is the image at 120 dpi (4x3 px -> 2.4 x 1.8 pt); nothing else differs ✔ (one case).

### 4C.1.6 Replace ("Change Picture") ✔ (`picture-replaced`, `picture-same-replaced-second`, one JPEG replace)
- **Contents**: the picture object keeps its id and its place in the page list but gains **item 3 = true** (`03 08`, between items 2 and 12; meaning "picture was replaced" ~); the old info object is **removed** (its slot
  becomes empty, ids are not reused) and a **new info object takes the next free id** (297 -> 298); item 58 is updated; the info object describes the new file. Index slot count / last slot grow by 1. 170/171 unchanged (above).
- **Picture store**: the old entry is **not edited in place**. If nothing else uses it, its 36-byte entry becomes an **empty placeholder** (all fields zero except tag 0x00FF; type 0, uid zero, ref_count 0,
  delay_offset 0) and a **new entry is appended** ✔; the shape's 0x4104 becomes the new index (2). If another shape still uses it (`picture-same-replaced-second`), the old entry stays with ref_count 2 -> 1 and the
  new entry is appended. The 0xF001 instance counts the placeholder (2 entries). The delay stream is rewritten (live records only).
- **Escher shape**: name and 0x0106 as in 4C.1.4. The anchor changed: Publisher kept the picture's **height and centre** and took the width from the new image's aspect ratio (120 x 90 pt at 100,150 replaced by a
  square -> 90 x 90 at 115,150; a JPEG with the same aspect -> unchanged). A writer may choose any frame; this is just what Publisher did.
- Replacing with a different format only changes the entry type (5 / 6) and the delay record header ✔.

### 4C.1.7 Delete ✔ (`picture-deleted`, `picture-two-deleted-first`, `picture-same-deleted-first`)
- Escher: the shape container is removed (0xF003/0xF002/stream lengths shrink by its size + 8); `ref_count -= 1` on its store entry ✔ (`picture-same-deleted-first`: 2 -> 1, entry and delay unchanged).
  If that was the **last** user the entry is zeroed to a placeholder (`picture-two-deleted-first`: slot 1 is a placeholder, the surviving shape keeps index 2) ✔; if **no entries are left, the whole 0xF001 is
  removed and the delay stream is empty** ✔ (`picture-deleted` has exactly the blank's Escher bytes except the two bytes in 4C.1.3).
- Contents: both objects are removed, the page's child list shrinks (an empty list removes items 1 and 2 of the page object entirely: the page body is byte-identical to the blank's ✔) and their index slots
  become empty slots (class-0x78 slots) ✔; **trailing empty slots are trimmed** (the slot count returns to 296 when the picture was the last object, `picture-deleted`) ✔, otherwise the count stays
  (`picture-two-deleted-first`: slots 296/297 empty, count 300).
- Whether a later insert reuses an empty placeholder slot in the picture store: not tested ? (experiment: delete the first of two, insert a third picture, look at the store).

### 4C.1.8 Crop and move/resize (✔ move, ~ one crop case)
- **Move / resize**: only the anchor (0xF010) and Contents items 170/171 change ✔. Nothing in the store or delay stream.
- **Crop** (`PictureFormat.CropLeft=6, CropTop=3, CropRight=9, CropBottom=12` on a 40x30 px PNG shown at 120 x 90 pt): Publisher's crop values are in points **of the image's natural size** (here 40 x 30 px = 24 x 18 pt),
  not of the frame. The four properties become fractions of the whole image: 0x0100 top = 0x2AAB (3/18), 0x0101 bottom = 0xA222 (0.633: Publisher clamped 12 to 11.4), 0x0102 left = 0x4000 (6/24),
  0x0103 right = 0x6000 (9/24) ✔. The anchor and 170/171 became the cropped frame (130,165, 45 x 18 pt). Exact frame maths for a general crop: ? (derive from `picture-cropped` plus one more crop case).

### 4C.1.9 Writer checklist (what is different from "add rectangle", §4N.5)
**Insert a picture** (embedded PNG/JPEG file bytes B, frame L,T,W,H in pt, name = file stem):
1. `uid = MD4(B)`. If the store already has that uid: `ref_count += 1`, reuse its index and add no delay record. Else append a 0xF007 (type 6 PNG / 5 JPEG, size = 25 + len(B), ref_count 1, offset = end of the
   current delay stream, tail zero) to the 0xF001 (create it after 0xF006 when absent; instance = entry count) and append the delay record (4C.1.2).
2. Append a shape container (4C.1.4) as the last child of the page group: spid = lastspid + 1, pib = store index, the name, anchor, client data = new object id; fix 0xF008 (cspids, lastspid), 0xF006
   (cspSaved +1, FIDCL count +1) and the 0xF000 / 0xF003 / 0xF002 / stream lengths.
3. Contents: objects N (kind 1, 50 bytes, item 58 = N+1) and N+1 (kind 102) in the next free slots; move a kind-108 font table to N+2 and fix 282[4]; two index entries and header counts; page child count and
   list (append N); overlap items 70/71 on older text boxes it overlaps; rebuild lengths and offsets; bump the save counter to look like Publisher.
4. Quill: nothing.
**Replace**: 4C.1.6. **Delete**: 4C.1.7 (decrement ref_count; zero the entry when it reaches 0; drop 0xF001 when every entry is empty; rewrite the delay stream with live records only; keep lastspid and the
FIDCL count; remove both Contents objects and trim trailing empty slots).
**Not tested**: BMP/GIF/TIFF/metafile inputs; large images (the delay stream is a normal stream either way, but untested); "save with document" off beyond the one linked case; pictures on master pages, inside groups
or inline in text (`MoveIntoTextFlow`); rotated/flipped/recoloured pictures; brightness/contrast/transparency effects (expect extra properties ?); text wrap around pictures (§4K probably applies the same way ~);
whether Publisher tolerates stale delay data or non-zero garbage in the entry tail.

## 4D. Drawn shapes ✔ (`gen/shapes.pub`; public [MS-ODRAW] properties)
Shapes are ordinary [MS-ODRAW] shape records; their kind and look come from standard fields:
| Where | Meaning | Observed |
|---|---|---|
| shape record (0xF00A) instance field | shape type | 1 rectangle, 3 oval, 20 line ✔ |
| property 0x0181 | fill colour | 0xE6D8AD light blue, 0x00FFFF yellow ✔ |
| property 0x01C0 | line colour | 0x8B0000 dark blue, 0x0000FF red ✔ |
| property 0x01CB | line width, EMU | 25,400 = 2 pt; 38,100 = 3 pt; 12,700 = 1 pt ✔; absent = 0.75 pt ~ |
| property 0x01BF bit 0x10 (use-bit 0x100000) | filled | set on filled shapes, clear on the outline-only rectangle ✔ |
| property 0x01FF bit 0x08 (use-bit 0x80000) | has line | clear on the oval (line turned off) ✔ |
Position is the usual client anchor (§4A); a line's top = bottom for a horizontal line ✔.
Other shape-record types seen (found while implementing): **202 = text box**, **201 = table**;
a picture shape is type 1 (rectangle) with no fill and no line. `Contents` kinds: text box 1,
**line 32**, table 16. Tables use items 104/105 for size, not 170/171.
Text colour-list entry 0 (flag 8 → scheme 0) is not referenced by default-coloured text: runs
with no colour item simply use the default; the first explicitly coloured run gets index 2.

**Colour values** (fill, line, and the text colour list §3.8) are 4 bytes: R, G, B, flag.
Flag 0 → literal colour. Flag **0x08** → **colour-scheme entry**, index in the low byte
(this is the published [MS-ODRAW] scheme-colour convention). Seen: shape defaults
0x08000007 (scheme colour 7), text colour-list entry 0 = 0x08000000 (scheme colour 0).
Scheme slot numbering ✔ (`gen/schemes.pub`): Publisher's scheme slots 1–8 are stored as
scheme indices **0–7** (0x08000000 … 0x08000007).

## 4F. Colour scheme ✔ (`gen/schemes.pub`, `gen/colors.pub`, Tika)
The document's colour scheme is the `Contents` object of **kind 92**:
| item | type | Meaning |
|---|---|---|
| 1 | 4 bytes | number of scheme colours (8) |
| 2 | list | scheme colours in index order. Element 0 is a bare flag element (no record) = **black** (the main text colour); elements 1–7 are records whose item 1 (4 bytes) is the colour as R, G, B, 0 ✔ |
| 3 | 4 bytes | ? (4 for "Black & Gray", 0x4B for "Office") — possibly a built-in scheme id |
| 5 | flag | ? |
| 6 | raw | scheme **name**, UTF-16 with a terminating zero ("Black & Gray", "Office") ✔ |
Verified: "Black & Gray" → 000000, 666666, 999999, CCCCCC, CCCCCC, then 0000FF-blue,
660099-purple (stored as COLORREF bytes R,G,B), FFFFFF — matching Publisher's own report
for all 8 slots. Resolve a scheme-colour reference *n* to element *n* of this list; index 7 is
white in all three samples.

- Implemented and checked against `gen/schemes.pub` (all 8 slots) and `gen/colors.pub` (black at 0, white at 7) ✔. With no kind-92 object, fall back to black for every index except 7 (white) ~.
- Cross-check ✔: the POI newsletter uses the built-in "Berry" scheme; its stored entries match Publisher's own Berry colours slot for slot (asked Publisher through automation), including the bright magenta FF00FF in slot 3 (index 2). An element of type 0x78 (empty) in position 0 also means black.

## 4E. Tables ✔ (`gen/table.pub`, `gen/table2.pub`)
- A table is a `Contents` object of **kind 16** (a shape on the page, with the usual Escher
  anchor and client data). Items: 102 = **row count**, 103 = **column count**, 104 = total
  **width**, 105 = total **height** (EMU), 39 = story number ✔.
- Item **109** (list) = the grid lines ✔: first one record per **column**, then one per
  **row**. Each record: item 1 = running total (the column's right edge / the row's bottom
  edge, measured from the table's left/top), item 2 = that column's width / row's height (EMU).
  `gen/table2.pub`: columns 72/216/144 pt (edges 72, 288, 432), rows 36/108/72 pt
  (edges 36, 144, 216).
- Item **107** = object number of a **kind-99** object = the cells ✔: item 1 = cell count,
  item 2 = a list with one record per cell in **row-major order** (fresh tables; after Insert Row/Column it is creation order, see 4E.1.5). Per cell: items 1/2 =
  first/last row index, items 3/4 = first/last column index (absent = 0; a merged cell would
  span more than one ~), items 10–13 = the four cell margins (36,576 EMU = 0.04 in default) ✔,
  item 9 = height of the cell's text as last laid out ~ (one line of Calibri 10 = 228,180;
  three paragraphs = 749,550), item 14 = 114,300 (9 pt) on every cell ?.
- All cell text is **one story**, cells in row-major order (fresh tables; see 4E.1.5). Paragraphs inside a cell and the
  end of a cell are both just `\r` in `TEXT`; the cell boundaries come from the Quill **`TCD `**
  chunk ✔: 12-byte header (u32 = number of cells − 1, u32 0, u32 0xFF00 ?), then one u32 per
  cell. For every cell but the last, the value is the position (from the start of the table's
  story) of the `\r` that **ends** that cell; for the last cell it is the story length (one past
  its final `\r`). Cell k's text is therefore `story[prev + 1 .. value]` (prev = −1 for the
  first cell), dropping a trailing `\r`. `gen/table2.pub` (story of 103 characters):
  5, 11, 17, 23, 74, 80, 90, 96, 103 — cell B2 holds three paragraphs and ends at 74, cell A3
  holds two and ends at 90. `gen/table.pub` (12 characters): 2, 5, 8, 12 ✔.
- An **empty cell** is a single `\r` ✔ (Publisher-made variants of `table2.pub`): emptying cell 1
  gives ends 0, 6, 12, …; emptying all nine gives `TEXT` = nine `\r` and ends 0, 1, 2, …, 7, 9
  (last = story length). The `TCD ` header stays (cells − 1, 0, 0xFF00) and its size depends only
  on the cell count, so a text edit rewrites the ends in place. A writer using these rules
  produced ends byte-identical to Publisher's for the same edit ✔.
- After cell text changes, Publisher recomputes the table's row heights on open; no `Contents`
  change is needed for a text-only edit ✔.
- Which `TCD ` belongs to which table when a document has several tables: ? (one table per
  sample so far; probably one `TCD ` instance per table in story order).

## 4E.1 Tables: insert, add/delete rows and columns, delete table (real Publisher 2021 COM and ribbon, then Save(); our own samples; evidence `samples/gen/table-*.pub`; UI in `docs/ui/TABLES.md`; 2026-11-15)
Method: copies of `samples/blank.pub` were opened in Publisher 2021 (16.0), edited through COM (`Shapes.AddTable`, `Table.Rows/Columns.Add`, `.Delete`, `Shape.Delete`, `Cell.TextRange.Text`), saved with `Save()`, decoded with `tools/contents_tree.py` and `tools/cfbls.py` plus throw-away Escher and Quill dumpers (same walkers as in 4L), and diffed. Insert > Table > 3x2 grid cell and Insert Table... were also driven through the ribbon (UI Automation only, no keystrokes) and give the same cell defaults as `AddTable`. Marks: ✔ seen in at least two files or fully explained, ~ seen once or inferred, ? unknown. The base table block (§4E, items 102-109, `TCD `) still holds; this section corrects it for edited tables and adds the Escher side and the object/story bookkeeping.

Samples (each from `blank.pub`, one `Save()`): `table-empty` (blank re-saved), `table-2x2` (`AddTable(2,2,72,72,300,100)`), `table-3x3-text` (`AddTable(3,3,72,72,360,120)`, cells A1..C3), `table-addrow-before` (3x3 then `Rows.Add(2)`), `table-addrow-after` (`Rows.Add(-1)`), `table-addcol-before` (`Columns.Add(2)`), `table-addcol-after` (`Columns.Add(-1)`), `table-delrow` (`Rows(2).Delete`), `table-delcol` (`Columns(2).Delete`), `table-deleted` (the 3x3 table, then `Shape.Delete`), `table-2x2-moved` (2x2 at 200,300), `table-2x2-resized` (2x2, column 1 = 50 pt, row 1 = 70 pt), `table-2x2-grown` (2x2 with a long text that wraps in cell (1,1)), `table-two` (two 2x2 tables at 72,72 and 72,300, "A" and "B" in their first cells), `table-after-textbox` (a text box "Hello" at 72,400, then a 2x2 table at 72,72 with "Z" in cell (2,2)). The older hand-made `table.pub` / `table2.pub` are described in §4E.

### 4E.1.0 A table is four things
1. **Contents**: a kind-16 table object N (child of the page), an **empty index slot N+1** (no object; the index list holds an empty list slot), and a kind-99 **cells** object N+2 (parent = the table) ✔ (`table-2x2`: 296 / 297 empty / 298; with a text box first 297 / 298 empty / 299; two tables 296..298 and 299..301). The font table (kind 108, always last) moves up exactly as for a text box (§4L.2) ✔; a "Calibri" font object is added when the file had none.
2. **Escher**: one **rectangle shape per cell** in the *cell drawing* (the drawing whose group has spids 3072+, 0xF008 inst 3, in `blank.pub`) and one **frame shape** (a text-box-like container whose FSP instance is **0xC9 = 201**, a text box is 0xCA) as the last child of the *page drawing* (spids 1024+, 0xF008 inst 1) ✔.
3. **Quill**: ONE story for all cells (cell texts in cell order, each cell ending in `\r`), a `TCD ` chunk whose instance number is the story's 0-based ordinal, and an `MCLD` record with one frame per cell ✔.
4. A record in the story registry object 284 (kind 101) ✔; **no** record in 283 (kind 97) (a text box gets both) ✔.

### 4E.1.1 Table object (kind 16) ✔
Items (`table-2x2`): `[2]=true`, `[4]=u16` (a **session value**: 0x3C03 in one Publisher session, 0xA003 and 0x9603 in others; not tied to size, position or text; any value will do ~), `[12]=0`, `[13]=0` (8-byte), `[39]=story number`, `[42]=true`, `[102]=rows`, `[103]=columns`, `[104]=width EMU`, `[105]=height EMU`, `[107]=object id of the cells object`, `[109]=grid list`, `[112]=0xFFFFFFFD` (u32), `[183]=0`.
- **Item 3 = true** appears once the table was edited after creation: rows/columns added or deleted, a row or column resized, a row auto-grown for text ✔ (`addrow-*`, `addcol-*`, `delrow`, `delcol`, `resized`, `grown`; absent in `2x2`, `3x3-text`, `moved`, `two`). Meaning ~ "not the uniform created grid".
- **The position is NOT in this object.** It is the anchor of the frame shape (4E.1.4). `table-2x2-moved` (created at 200,300) and a table created at 72,72 and moved afterwards with `Left/Top` differ only by the anchor (and the session value) ✔.
- Grid `[109]`: first one record per **column**, then one per **row**; item 1 = running edge from the table's left/top, item 2 = that column's width / row's height, EMU ✔ (`table-2x2`: 1905000/1905000, 3810000/1905000, 635000/635000, 1270000/635000). Non-integer point sizes are rounded to 1 EMU (61.5459824 pt = 781634).
- Items 104/105 follow the grid after `Rows/Columns.Add` and `.Delete` and after explicit resizing (3x3 360x120 pt = 4572000 x 1524000; plus a 40 pt row: 105 = 2032000) ✔. **When a row auto-grows for text, item 105 is NOT updated** while the grid is ✔ (`table-2x2-grown`: 105 stays 1270000 = 100 pt; COM and the frame anchor report 111.55 pt). **Correction (pw-h1ee, checked against the file): in the grid record only the running edge (item 1) is updated: row 1 has item 1 = 781634, item 2 = 635000 (stale), row 2 item 1 = 1416634, item 2 = 635000 (stale).** So a track's real size is the difference of consecutive running edges, and the real table height is the last row edge (1416634 = 111.55 pt). A reader must take sizes from the edges (item 1), not from item 2 or item 105. Confirmed on the 11 samples in `tests/tables.rs` (edges and item 2 agree everywhere except `grown`; `resized` not tested ~).
- `AddTable(rows, cols, L, T, W, H)` divides W and H evenly (300x100 pt, 2x2: 150 pt and 50 pt) ✔. Very small cells are clamped: minimum column width 16.93843 pt, row height 17.96693 pt (2 rows) but 26.74614 pt (5 and 50 rows) ~ (rule not understood). Insert > Table from the ribbon grid (3 columns x 2 rows) creates 72 pt columns and 27.10543 pt rows (216 x 54.21 pt) roughly at the page centre ✔ (`docs/ui/TABLES.md`).

### 4E.1.2 Cells object (kind 99, parent = the table) ✔
- `[1]` (u16, class 3) = number of cells; `[2]` = list with **one record per cell, in STORY order** (this is not row-major once the table has been edited, 4E.1.5).
- Cell record items (u32; zero is omitted): `[1]`/`[2]` = first/last **row**, `[3]`/`[4]` = first/last **column** (0-based; the cell in row 1, column 2 has `[1]=1 [2]=1 [3]=2 [4]=2`; cell (0,0) has none of the four), `[9]` = text height as last laid out (228180 = one Calibri 10 line; in `grown` it holds the new row height 781634), `[10]..[13]` = the four margins, all 36576 EMU (0.04 in = 2.88 pt; COM `MarginLeft/Top/Right/Bottom` = 2.88), `[14]` = 114300 always ✔. Merged cells (a record spanning several rows or columns) were not tested.
- Default cell look (COM, identical for `AddTable` and the ribbon): white fill (fill visible), no cell borders, Calibri 10, left aligned, top aligned, horizontal text, `GrowToFitText` true ✔. Nothing of this is stored in the cells object (it is the default).

### 4E.1.3 Quill side ✔
- `TEXT`: cell texts in cell (story) order, each cell = its paragraphs, last one ending in `\r`; an empty cell is a single `\r`. `STRS` gets the story length; `SYID` (`u32 H, u32 n, n story numbers`) the new story; `FDPP` one run per paragraph (`04000000` = defaults); `FDPC` one run for all-empty cells (`0a000000 | 12 22 0409 0000`) or a text run record (`10000000 | 12 22 0409 0000 3e 22 0409 0000`) plus the `\r` run record; `BTEP`/`BTEC` as for text (§4L.3).
- `TCD `: `u32 cells-1, u32 0, u32 0xFF00`, then one u32 per cell = position, relative to the **start of the table's story**, of the `\r` that ends the cell; the last cell holds the story length (rule of §4E) ✔. **Which `TCD ` belongs to which table: the chunk's instance number (directory entry "TCD 0", "TCD 1") is the 0-based ordinal of the table's story among all stories sorted by story number** ✔ (`table-two` has TCD 0 and TCD 1; `table-after-textbox` has only TCD 1 because its text box is story ordinal 0). A reader that takes "the first TCD whose cell count fits" (current `pub-format`) only works when no two tables have the same cell count.
- `MCLD` record of the story: the usual 12-byte block, then `u32 frame count = number of cells`, then one 0x82-byte frame per cell in cell order ✔. Frame items 4/5 = the cell's width / height in EMU ✔ (150 pt = 1905000; the frame of a 50 pt column has 635000), item 10 = 0 (text boxes: 82676). Items 0-3 (top, left, bottom, right) are a **view-dependent layout cache** ~: neighbouring cells differ by 1 unit, about 0.89 units per point, and the same 2x2 table gave 7541/7622/7585/7756 in one Publisher session and 4293/4339/4307/4389 (smaller table) in another. Copying plausible values should be enough ~ (as for text boxes, §4L.3; untested).
- Story numbers: first table in `blank.pub` = 3, a second table = 6, a table after a text box (story 2) = 5 ✔; `SYID` H = the newest number. Any number above all existing ones keeps the ordering rule (§4L.4).

### 4E.1.4 Escher side ✔
- **Cell shapes** (one per cell): container 0xF004; FSP inst **1** (rectangle), flags 0xA00, `spid` = the cell drawing's last spid + 1; 0xF00B with 8 properties (`0x183=0, 0x1BF=0x1F001C, 0x1FF=0x80000, 0x285=0, 0x2CB=0, 0x2CC=0, 0x2CE=0, 0x2CF=0`); 0xF122 with 8 (`0x1BF=0x600000, 0x1FF=0x400040, 0x295..0x29A=0`); then ONE record of type 0xF010 (version 0xA, instance 0x1A) that holds a Contents item block: cell 0 `0a000000 | 02 68 <table object id>`; cell k > 0 `10000000 | 02 68 <table object id> | 03 20 <k>` (k = 1..n-1 = the cell's position in the story) ✔. Containers are 146 / 152 bytes. Cell shapes carry **no geometry**; cell positions come from the grid in Contents.
- **Frame shape** (one per table): container 0xF004 (296 bytes) appended as the last child of the page drawing's group; FSP inst **0xC9**, flags 0xA00; 0xF00B with 15 properties (`0x80 = story number` (lTxid), `0x81..0x84 = 0` insets, `0x17F=0x300000, 0x1C0=0x8000000, 0x1C2=0x8000007, 0x1CB=0x6338, 0x1FF=0x80000, 0x201=0x8000004, 0x384..0x387=0x8EE0`); 0xF122 with 17 (scheme colour triplets, `0x17F=0x3800000, 0x1FF=0x400000, 0x6FF=0x20002`); a real client anchor 0xF010 of 28 bytes (`1c000000 | 01 20 L | 02 20 T | 03 20 R | 04 20 B`, EMU relative to the page centre, the text-box rule of §4L.1: `L = left_pt*12700 - 3886200`, `T = top_pt*12700 - 5029200`, `R = (left+width)*12700 - 3886200`, `B = (top+height)*12700 - 5029200` for Letter) ✔ (checked on `table-2x2` 72,72 300x100; `moved` 200,300; two sizes; `grown`); client data `0a000000 | 01 68 <table object id>`; textbox record `0a000000 | 01 20 <story number>`. The anchor's bottom uses the **actual** (auto-grown) height ✔.
- Counters (as §4L.1): 0xF008 of the cell drawing: cspids += cells, lastspid += cells; 0xF008 of the page drawing: += 1 and += 1; 0xF006: maxspid = cell-drawing lastspid + 1, cspSaved += cells + 1, FIDCL pair of the cell drawing (dgid 3) count += cells, of the page drawing (dgid 1) += 1; all container lengths re-summed ✔ (`table-2x2`: maxspid 3075 -> 3079, cspSaved 5 -> 10, pairs (1,1)->(1,2), (3,3)->(3,7); 3x3: cspSaved 15, pair (3,12)).
- **Escher is not clean after row/column edits** ✔: `Rows.Add` / `Columns.Add` re-create cell shapes with new spids and leave stale or duplicate ones (`table-addrow-before`: 17 shapes in the cell drawing for a 12-cell table, cell indexes 1 and 2 twice, 7 and 8 twice; `table-delrow`: 12 shapes for 6 cells). Publisher still reloads and re-saves such files, so **the Escher cell shapes are not authoritative; read the table structure from the Contents cells object only**. A writer can emit exactly one fresh shape per cell as in a new table (not yet tested in Publisher).
- **Delete table** removes the cell shapes and the frame (cspids back to 3 and 1, cspSaved back to 5) but spids are not reused: `lastspid` (3083, 1025), `maxspid` (3084) and the FIDCL pair counts ((1,2), (3,12)) keep their high water ✔ (`table-deleted`).

### 4E.1.5 Row and column edits ✔
(COM indexes are 1-based. `Rows.Add(n)` = ribbon Insert Above on row n; `Columns.Add(n)` = Insert Left on column n; `Add(-1)` or `Add()` appends = Insert Below / Right of the last row / column. `Add(0)` and `Add(Count+1)` raise "Invalid index".)
- **The table grows, nothing else moves**: adding a 40 pt row to the 3x3 (120 pt high) gives 160 pt; adding a 120 pt column gives 480 pt wide; the left/top stay. The new row copies the height of the row it is inserted before (row 1 = 70 pt, insert before it -> 70 pt), a new column the width (column 1 = 50 pt -> 50 pt); appended ones had the common 40 / 120 pt ~. New cells are empty with the default format.
- **Existing cell text keeps its place in the story; inserted cells are appended at the END of the story.** `table-addrow-before` (Rows.Add(2)): `TEXT` = `A1 B1 C1 A2 B2 C2 A3 B3 C3` then three empty cells; `TCD ` count 11 and ends 2,5,8,11,14,17,20,23,26,27,28,30; cells object count 12; the first nine records have their row indexes renumbered (old rows 2, 3 become rows 2, 3 of 0..3) and the three new records have row 1. `table-addcol-before` likewise: new cells are last, column 1, rows 0,1,2 ✔. `table-addrow-after` / `table-addcol-after` look the same with row 3 / column 3 for the new cells ✔. **So story order is creation order, not row-major, and the cell records' row/column items give the true position.** The "row-major" statement of §4E holds for freshly created tables only.
- **Delete row or column**: the cells of that row/column leave the story (their text is gone), the other cells keep their relative story order and are renumbered, the grid loses a record, items 102/103/104/105 shrink, item 3 = true, **a new cells object id is allocated** (298 -> 300; the font table moved into a freed slot, 299 -> 286) ✔ (`delrow`: `TEXT` = `A1 B1 C1 A3 B3 C3`, TCD count 5; `delcol`: `A1 C1 A2 C2 A3 C3`). `SYID` H rises (4 -> 6), the table's story number is unchanged. Deleting the only remaining row raises `E_ACCESSDENIED` ✔.
- **Delete table**: `Contents` of `table-deleted` equals `table-empty` except the save counter (object 256 item 60, §4M) ✔; Quill returns to no story except an empty `MCLD` chunk (`01000000 00000000`) and `SYID` = (H=4, n=0), i.e. the story-number high water mark stays ✔; Escher: see 4E.1.4.
- **Text growth** (`grown`): the new row height is written to the grid, cell item 9, MCLD frame item 5 and the frame anchor bottom; item 105 is not ✔. **Resize** (`resized`): grid records, 104/105, MCLD frame items 4/5 and the anchor right/bottom change; item 3 = true ✔.

### 4E.1.6 What `pub-format` already reads, and where it is wrong for these files
Reads: kind 16 / kind 99, items 102/103/104/105/107/109, cell spans and margins (`CellProps`), the `TCD ` ends (`Quill.cell_ends`), cell text and paragraphs (`Table.cells`, `cell_paragraphs`, `cell_props`); table position and size come from the frame shape's anchor (`pubdump` on all new samples shows e.g. `table 4x3` at 1.00-6.00 in) ✔. Mismatches with the files above (**fixed in pw-h1ee**; the reader now does all of this, tests in `crates/pub-format/tests/tables.rs`): (1) `Table.cells` is the story order treated as row-major, so after Insert Row/Column cells land in the wrong row (`pubdump table-addrow-before.pub` lists the three empty cells as the last row, although the new row is row 2); `cell_props` row/column must be used to place cells; (2) the `TCD ` is chosen as "first whose count fits", not by the story's ordinal (now: instance = rank of the table's story number among the document's story numbers, as used for story attachment; `Quill.cell_ends` is indexed by instance); (3) item 105 (and grid item 2) is stale after row growth, so the height is the sum of the row sizes taken from the grid's running edges; the Escher cell shapes are not used at all by the reader (confirmed: structure comes only from items 102/103/107/109, the cells object and `TCD `); (4) item 3 is ignored (harmless).

### 4E.1.7 Writing tables (`pub-write` `add_table`, `delete_table`; verified in Publisher 2021 on 2026-11-15)
Implemented in `crates/pub-write/src/table.rs` (+ `quill.rs`: `add_table_story`, `remove_story`, `TCD ` add/remove in the directory rebuild). Method: our writer applied to `samples/gen/table-empty.pub` reproduces Publisher's `table-2x2`, `table-3x3-text`, `table-two` and (add + delete) `table-deleted` **byte for byte** in the Escher stream, in every Contents object (compared per object: kind, parent, block; save counter and table item 4 zeroed) and in the Quill chunks `TEXT FDPP FDPC BTEP BTEC SYID TCD STRS` (`MCLD` equal except the items listed below). Tests: `crates/pub-write/tests/table_write.rs`. 15 written files (2x2, 3x3 with text, two tables, table behind a text box, table on page 2, 5x4 with two-line cells, add+delete, delete first/last of two with and without a text box, cell-text edits of Publisher-saved tables) all open in Publisher 2021 (`Application.Open`), read back through COM (`HasTable`, `Rows.Count`, `Columns.Count`, `Cells(r,c,r,c)`, `Left/Top/Width/Height`) with exactly the written values, and survive `Save()` + reopen unchanged.

What the writer does, in order (N = first free slot = the font table's slot):
1. Quill: story text = cell texts, each cell's paragraphs then `\r`; `STRS`/`SYID`/`BTEP`/`BTEC`/`FDPP`/`FDPC` as for a text box, except the character runs are one text run per non-empty paragraph and one mark run per stretch of consecutive `\r` (paragraph runs never merge); new `TCD ` chunk (instance = number of stories before, i.e. ordinal; placed after the last `TCD ` or after `FONT`, payload `cells-1, 0, 0xFF00, ends`); `MCLD` record = 12-byte head, frame count = cells, one 0x82-byte frame per cell cloned from Publisher's default frame with items 0x200-0x203 = layout cache, 0x204/0x205 = cell width/height, **0x20A = 0 and 0x216 = 0** (text box: 82676 and 9525).
2. Escher: cells appended to the cell drawing (dgid 3, spids from lastspid+1), frame appended to the page drawing; `cspids`, `lastspid`, `maxspid` (= cell lastspid + 1), `cspSaved`, FIDCL counts as 4E.1.4.
3. Contents: font table ensured (as for a first text box), registry 284 only (record `[1]` story, `[3]` u16 0, `[7]` ordinal, `[8]`/`[9]` constant; 284's index entry gets item 11 on its first record), objects N / empty slot N+1 / N+2 in front of the font table, page child list gets N.
`delete_table` reverses it: cell rectangles (found by the item 2 of their 0xF010 record) and the frame go (`cspids`, `cspSaved` drop; `lastspid`, `maxspid`, FIDCL counts stay), page child list entry, the 284 record (**the ordinals `[7]` of the records behind it move down by one; without this Publisher says "cannot open the file"**; an emptied registry loses items 1/2 and the index entry's item 11), the table and cells objects and trailing empty slots (as `delete_picture`), and the story: text, runs, `STRS` entry, `SYID` entry (high-water mark kept), `MCLD` record (header `m,m,1..m`), its `TCD ` chunk (later instances move down by one). When it was the **last story**, `TEXT FDPP FDPC BTEP BTEC STRS` leave the Quill directory, `MCLD` becomes `01 00 00 00 00 00 00 00`, and the Contents font table object and the story root's item 4 disappear (the file equals the blank again).

Publisher requires (found by experiment or required to match its files):
- ordinals `[7]` in registry 284 contiguous after a delete (see above); everything else we copy is accepted as is;
- the `TCD ` instance = story ordinal (a wrong instance is not tested in Publisher, but the reader and `set_table_text` use it; `set_table_text` was fixed to use the ordinal instead of "rank among table stories", which picked the wrong chunk for two same-size tables behind a text box).
Just copied / not needed by Publisher as far as seen: table item 4 (any u16; we write 0xA003), cell item 9 (we write one line, 228,180, for every cell; Publisher re-lays out), `MCLD` layout cache items 0x200-0x203 (fitted, ~0.8935 units/pt, 7476.7 / 7557.7 offsets), items 0x212/0x213 (stale memory in Publisher's files; we write zeros/0xFF00), the `STSH`/`FONT` timestamps and the `STSH` housekeeping (0 -> 0xFFFF in 14 places) Publisher does when it inserts a table (we leave `STSH` alone; Publisher opens, shows and re-saves fine).
Story numbers: Publisher numbers a table's story H+2 in a file without stories and H+3 otherwise (`table-2x2`: 3; `table-after-textbox`: 5; `table-two`: 3 and 6); the writer copies that; `SYID` H = the story number. Publisher's own later COM operations burn further numbers (H=4 after the 3x3 text edit); we do not.
Limits: cell size at least 215,118 EMU wide and 228,180 EMU high (Publisher's smallest `AddTable` cells) else error; at most 65,535 cells; files without the cell drawing (dgid 3, e.g. old-version files like `bulletin.pub`) are refused; files whose font table is not the last slot (Publisher-saved after Insert/Delete Row/Column, spec 4E.1.5) are accepted by the table adder and deleter since 4E.1.7a (the text-box adder shares the code, untested; other delete paths still refuse them); no overlap items 45/70/71 are written for text boxes under a new table (not observed, not needed to open).

### 4E.1.7a Writing row and column edits (`insert_table_rows`, `delete_table_rows`, `insert_table_columns`, `delete_table_columns`; verified in Publisher 2021 on 2026-11-15)
Approach chosen: **rebuild a clean table**, not Publisher's own edit result (creation-order story, new cells object id, stale Escher cell shapes of 4E.1.5). The writer reads the table with `pub-format` (track sizes from the grid's running edges, cell texts placed by row/column, frame position from the anchor), edits the in-memory layout, then runs `delete_table` and adds the table again at the same left/top with the edited tracks (internal `add_table_tracks`): fresh row-major story, one fresh Escher rectangle per cell, `TCD ` and `MCLD` rebuilt, table item 3 = true (Publisher sets it on every edited table), grid = the edited tracks (edge, size), items 104/105 = their sums. Rules (all as `Rows.Add`/`Columns.Add`/`.Delete` do, 4E.1.5):
- Insert `count` rows before 0-based row `n` (`n` = row count appends): each new row has the height of row `n` (the last row when appending), the table grows by that, left/top/width unchanged, existing texts stay in their cells, new cells empty. Columns the same with widths.
- Delete: the rows/columns and their text go, the table shrinks, the rest keeps its order. Deleting all rows (or all columns), a range outside the table, `count` 0, or a table with merged cells is refused (`WriteError::Table`); the input is never modified.
- The table object number changes (it is deleted and re-created); each function returns `(bytes, new object number)`. Z-order: the table becomes the last child of its page (front-most); other shapes do not move. Cell text formatting is carried through the rebuild (bead pw-u3e4): the writer reads each cell's paragraphs (runs: bold, italic, underline, script, size, font, colour; paragraph alignment and spacing) and, after the clean table is added, puts them back with `set_story_format`, run for run, onto the cells' new positions. New cells from an insert are empty and copy the first run, alignment and spacing of the cell next to them in the track whose size they copy (the row/column inserted before, the last when appending); Publisher's own choice for new cells is not specified (~). Not carried: list/bullet items and indents, scheme-colour links (written as literal colours), and cell/table formatting not stored in the default items (none exists yet in our samples). Tests: `crates/pub-write/tests/table_row_format.rs`, `crates/kernpage-core/tests/tables.rs`. Story number rises (high-water mark), so the table's story ordinal may move behind other stories.
- Files Publisher saved after Insert/Delete Row/Column keep the **font table in the middle of the object list** (a freed slot below the cells object) and an **`MCLD` header of the form `high, n, ids...`** (`02 00 00 00 | 01 00 00 00 | 02 00 00 00` for one story: record-id high-water mark, story count, one id per story; a fresh file is `n, n, 1..n`). Both are now handled: `Cts::font_slot_any` (new objects are appended at the end of the object list when the font table is not the last slot, delete empties slots in place and only compacts below a *last* font table) and `mcld_header` in `quill.rs` (add writes `high+1, n+1, ids.., high+1`; remove drops the id and keeps the mark unless the header was regular, in which case it is renumbered as before). Text boxes and the other writers still refuse such files where they use the strict `font_slot_opt` (delete group/page); that limit is lifted only for the paths table edits use plus `tail()`.
Verification (Publisher 2021 COM only, local evidence files, generated by `TABLE_RC_DIR=... cargo test -p pub-write --test table_rows write_cases`, read with `tblcheck.ps1`: `HasTable`, `Rows/Columns.Count`, every cell's text, each `Rows(r).Height`, `Columns(c).Width`, shape Left/Top/Width/Height compared with `pub-format`'s reading): 49 files, all open, all read back as expected (no differences above 0.1 pt), and all survive `Save()` + reopen with the same readings, also under `run2021.ps1 -Mode verify`. Cases: our own 3x3 with a row inserted before first/middle/end, column inserted before first/middle/end, row delete first/middle/last, column delete first/middle/last, chain (2 rows added, column deleted, cell text edited, column added), two of our tables with only the second edited; Publisher's `table-2x2-resized`, `table-2x2-grown`, `table-addrow-before/after`, `table-addcol-before/after`, `table-delrow`, `table-delcol` each with a row inserted in the middle, a column appended, the first row deleted and the last column deleted; `table-two` (second table edited) and `table-after-textbox` (column inserted first). Tests: `crates/pub-write/tests/table_rows.rs`.

### 4E.1.8 Open questions
- Merge/split cells, cell fill and borders, the Table Style gallery (65 styles), vertical alignment, text direction, diagonals: not tested; none of it appears in the default cells object, so it lives in extra cell items or objects ?.
- (answered in 4E.1.7: Publisher opens and re-saves files with one clean Escher shape per cell and a fitted MCLD cache.)
- Meaning of table items 3, 4, 112, 183 and cell item 14; the minimum cell size rule of `AddTable`; why a second table burns three story numbers.
- Tables on a master page (page 2 works, 4E.1.7); older-version files without the cell drawing; the story-number burn rule beyond the three cases seen.
- Whether `TCD ` instances of later stories renumber when an earlier story is added or removed ~.

## 4B. Text in a text box ✔
`hello.pub` Quill `TEXT` chunk = `Hello` followed by `\r`. Which part of `TEXT` belongs to
which text box: via the client-textbox story number + the Quill `STRS` table ~ (to confirm with
a two-text-box sample).

## 4G. Linked text boxes (one story through several frames) ✔ (`gen/linked.pub`, POI newsletter)
A long story can flow through a **chain** of text boxes, possibly on different pages.
- Every text-box object (`Contents` kind 1) carries item **39** (4 bytes) = the **story number**
  it shows ✔. All boxes of one chain have the same item 39; an ordinary box has a story to itself.
- Item **40** (4 bytes) = the box's **position in the chain**: absent on the first box, then 1,
  2, … ✔ (`gen/linked.pub`: three boxes = absent/1/2; the POI newsletter has ten 2- and 3-box
  chains with the same pattern). Order the boxes of a story by item 40, treating "absent" as 0.
- The file does **not** store where each box's text ends ✔ (frame 2 starts at character 368 and
  frame 3 at 736 in `gen/linked.pub`; neither value appears as a break table anywhere). Publisher
  re-flows the text when it opens the file. A reader must therefore **lay the story out itself**:
  fill the first box line by line (inside its text insets), continue with the next line in the
  next box of the chain, and so on. Text left over after the last box is **not shown** (Publisher
  marks that box as overflowing) ✔ (manifest: last frame `overflowing = true`).
- Breaks are by line, not by paragraph: a paragraph may start in one box and finish in the next ~.
- Quill chunk **`MCLD`** ~: one entry per story that is shown in text boxes. Layout: u32 4 (?),
  u32 count of stories (?), then per story a 12-byte block (item 0 flag, item 1 4-byte value ?),
  a u32 **frame count**, and one 0x82-byte item block per frame **in chain order**. In each frame
  block item 4 = frame width and item 5 = frame height (EMU), items 6–9 = the four text insets
  (36,576 EMU = 0.04 in, Publisher's default) ✔ (`gen/linked.pub`); items 0–3 and 10–29 are ?.
  Items 39/40 above are enough to rebuild a chain; `MCLD` is a cross-check.
- In `gen/linked.pub` frame 1 (3 × 2 in, Calibri 10 pt) holds paragraphs 1–4 and frame 2 holds
  paragraphs 5–8. That gives a layout test: with matching font metrics and line spacing, the
  first break should fall exactly after paragraph 4.

## 4H. Rotation ✔ (`gen/rotate.pub`; [MS-ODRAW] public property)
- A shape's rotation is Escher property **0x0004**, a 16.16 fixed-point angle in degrees,
  **clockwise** (90° = 0x005A0000) ✔. Absent = 0.
- The client anchor (0xF010, §4A) then describes the frame as follows ✔:
  - for angles in [45°, 135°) and [225°, 315°) (normalised to 0–360), the anchor rectangle
    has **width and height swapped** around the frame's centre (a 180 × 54 pt box at 90°
    has an anchor 54 wide and 180 tall, same centre);
  - for other angles (e.g. 30°) the anchor is the unrotated frame.
  To draw: take the anchor's centre, use the unrotated size (swap back in the ranges above),
  then rotate the shape and its text about that centre by the angle.
- `Contents` items 170/171 of a text box keep the **unrotated** width/height ✔.
- The story's `MCLD` frame width/height (items 4/5, §4G) are also the **unrotated** size ✔
  (`gen/rotate.pub`: the 90° and 270° boxes both store 2,286,000 × 685,800). A writer moving or
  resizing a rotated shape writes the anchor in the swapped form above (verified: Publisher opens
  the result with the expected position, size and angle).

## 4I. Text box columns ✔ (Escher), MCLD is a mirror (`gen/columns.pub`; 2021 edit+diff, 2026-11-07)
> **Where Publisher 2021 reads the column settings (found 2026-11-07, local evidence files):** from the text box's
> **Escher tertiary option record** (record type 0xF122 of the shape's SpContainer), not from `Quill/MCLD`. Publisher 2021
> itself was driven through automation (`TextFrame.Columns` = 2, 3, and 2 plus `ColumnSpacing` = 18 pt on `gen/bulletin.pub`;
> 1 on the 3-column box of `gen/columns.pub`), saved, and diffed against a no-op 2021 save of the same sample. The only
> real changes were: the Escher property below, the `MCLD` mirror values below, and per-save noise (see §5). `Contents` did
> not change at all.

- **Escher property 0x008C** (u32, in the 0xF122 record, no "blip/complex" flag bits) = **number of columns**. It is
  **absent for one column** ✔ (2021 removes it when going back to 1; `gen/columns.pub` as written by 2010 already had it
  for its 2- and 3-column boxes).
- **Escher property 0x008D** (u32) = **column spacing in EMU × 2** ✔. 2021 returns 2.88 pt for 73,152 and 18 pt for
  457,200 (so spacing_pt = value / 2 / 12,700). Every text box carries it (default 73,152 = the 0.04 in gap);
  with one column it is kept and still holds the last spacing ✔.
- Properties inside the record are sorted by id; 0x8C goes right before 0x8D. Adding or removing 0x8C changes the record's
  property count (the 4-bit-version / 12-bit-instance word: instance = count), its length (±6) and the length of **every
  enclosing container** (SpContainer 0xF004, SpgrContainer 0xF003, DgContainer 0xF002) by ±6 ✔. Nothing outside the Escher
  stream refers to Escher offsets (the `Contents` stream was byte-identical apart from the document path).
- **Quill `MCLD` mirror** (written by both 2010 and 2021): item **0x20B** (u16) = columns − 1, item **0x20A** (u32) =
  0x8D value + 9,524 EMU (466,724 = 457,200 + 9,524; 82,676 = 73,152 + 9,524) ✔ (so the earlier "2 × gap + 9,525" is the
  same rule within 1 EMU). **Publisher 2021 ignores it on load**: files with only these two items changed still report the
  old `TextFrame.Columns`/`ColumnSpacing` ✔ (tested both directions: 1→2/3 and 3→1). A writer should still keep it
  consistent with the Escher value (2021 does).
- Per-save layout cache: `MCLD` items 0x200–0x203 (u32 each) change whenever the column count/layout changes
  (2021 recomputes them); they are derived values, a writer may copy them ~ (not tested whether stale values are repaired
  on open, but the confirmation files below had stale ones and read correctly).
- Text fills column 1 top to bottom, then column 2, etc. (normal newspaper flow); column width
  = (frame width − insets − (n − 1) × gap) / n ~.
- Which `MCLD` group belongs to which story: groups appear in story order (the k-th group = the
  k-th story in `STRS`), with frames in chain order ~ (true for `gen/linked.pub` and
  `gen/columns.pub`; check against the POI newsletter before relying on it).
- **Confirmed ✔ (2021, read-only open of patched copies of the ORIGINAL 2010 samples):** adding only property 0x8C=2 to
  the first text box of `bulletin.pub` reads Columns 2; 0x8C=3 + 0x8D=457,200 reads 3 columns / 18 pt; 0x8D alone reads
  spacing 18 pt; in `columns.pub` removing 0x8C reads Columns 1 (spacing stays 18), changing it 3→2 reads 2; MCLD-only
  patches read the old values. The patched files had their CFB rewritten (stream grown by 6 bytes) and 2021 opened them.
- **Writer rule (✔):** to set N columns and gap G (EMU) on a text box: (1) in that shape's 0xF122 record set/insert/remove
  property 0x8C (= N, remove when N = 1) and set 0x8D = 2·G, fixing the instance count, the record length and the three
  ancestor container lengths; (2) set `MCLD` frame items 0x20B = N−1 and 0x20A = 2·G + 9,524; (3) leave `Contents` alone.
  The Escher stream may grow, so the CFB stream must be rewritten (not patched in place).

## 4I-b. Stacking order ✔ (local evidence files, Publisher COM ZOrder on `gen/bulletin.pub`)
- Shapes draw in the order of their shape containers (0xF004) in the drawing's group container
  (0xF003), first = back ([MS-ODRAW]). Publisher's Bring to Front / Send to Back only permute
  those containers (stream length, spids and container contents unchanged); `Contents` keeps no
  order list (only a page-object counter, item 0x20, changed: 2 → 3, likely a save count ?).
- One drawing holds containers for several pages; only the relative order of a page's own
  containers matters. A writer may permute within the page's slots ✔ (Publisher reads our
  reordered files with the expected `ZOrderPosition`). Group children stay inside their group.

## 4J. Grouped shapes ✔ (`gen/group.pub`; [MS-ODRAW] public records)
- A group is a nested shape-group container (0xF003) inside the page's group container. Its
  first shape container holds an **FSPGR** record (0xF009: left, top, right, bottom, signed
  32-bit) = the group's own coordinate box, an FSP with the group flag, the group's normal
  **client anchor** (0xF010, page-centre-relative, §4A) and its client data (`Contents` object).
- Each **child** shape container has a **child anchor** (0xF00F: left, top, right, bottom in the
  group's coordinate box) instead of a client anchor ✔. Children keep their own client data and
  client textbox (story number) as usual.
- Child position on the page: map the child anchor from the FSPGR box onto the group's client
  anchor rectangle (scale x and y independently, then offset) ✔. `gen/group.pub`: FSPGR
  x 108,127,800–110,413,800 and y 106,984,800–109,270,800 EMU, group anchor = 144,144 pt, 180 × 180;
  child text box child-anchor y 106,984,800–107,670,600 → page 144–198 pt; rectangle → 216–324 pt,
  matching Publisher.
- In `Contents`, the group itself is an object of **kind 48**, and each child's parent (index
  item 5) is the group object rather than the page ✔ (`gen/group.pub`: children 296/297 → parent 299 = the kind-48 group).
- Writing a moved child: map the page rectangle back through the group anchor onto the FSPGR
  box and write the child anchor ✔ (Publisher opens it with the child at the new place). The
  group's own anchor and FSPGR box are **not** recomputed by Publisher on open ✔: after children
  move outside the old bounds, Publisher still reports the group at its old box. So a writer
  updates the group box itself ✔: set the group's client anchor to the union of the children's
  page rectangles and shift each FSPGR edge by the anchor change × the group's scale (FSPGR
  size / anchor size), which keeps the page mapping of every other child unchanged. Publisher
  then reports the group at the union (grown and shrunk cases verified). The kind-48 `Contents`
  object has no width/height items to update ✔.
- Groups can nest: apply the mapping from the innermost group outwards ~ (not sampled yet).
- Group rotation/flip (Escher property 4 and FSP flip flags on the group) applies to the whole
  group about its centre ~ (not sampled yet).
- A shape without a client anchor must never be placed at the page centre with zero size; if
  it has a child anchor it belongs to its group as above ✔ (cause of the overprinted text in the
  POI newsletter).

## 4J-b. Creating and removing groups (Publisher COM Group()/Ungroup() then Save(), our samples only)
Marks: ✔ seen in 2 or more cases, ~ seen once. Evidence files (git-ignored, local evidence files: b_orig (copy of gen/bulletin.pub), b_tb_tb (group of 2 text
boxes), b_pic (text box + picture), b_p2 (group on page 2), b_3 (3 shapes), g_orig (gen/group.pub), g_ung (after
Ungroup), g_nest (group + text box regrouped = nested). *.esc = Escher walk, *.tree = Contents listing, walk.py/shp.py = scratch walkers.

## A. Group (Escher, EscherStm)
- Spid: new group spid = drawing's lastspid + 1 (b: 1029 -> 1030; g_nest: 1028 -> 1029) ✔. Drawing record 0xF008 gets
  cspids +1 and lastspid = new spid ✔. Drawing group record 0xF006: cspSaved (3rd u32) +1 (10->11, 9 -> 10) ✔; maxspid
  (3075), cidcl and the 3 other FIDCL pairs are unchanged ✔ (FIDCL pair for this drawing = (dgid 1, count 7) after
  group: it equals lastspid - 1024 + 1, i.e. number of spids ever allocated; it was 6 and became 7) ✔.
- New nested shape-group container 0xF003 (len = 8-byte first child container + children) is inserted into the page's
  group container at the slot of the TOPMOST (last in z-order) selected shape; all selected shapes are moved into it,
  keeping their relative z-order (bottom first); unselected shapes keep their slots ✔ (b_tb_tb: [pic, G{tb1,tb3}];
  b_pic: [G{tb1,pic}, tb3]; b_3; b_p2). COM ZOrderPosition agrees.
- Group's first child = 0xF004 shape container (len 108) holding, in order: FSPGR 0xF009 (16 bytes), FSP 0xF00A
  (inst 0, spid, flags 0x201 = fGroup|fHaveAnchor), 0xF122 (6 bytes `ff 06 02 00 02 00`, same as group.pub), client
  anchor 0xF010 (28 bytes, the usual `1c000000 | 01 20 L | 02 20 T | 03 20 R | 04 20 B`, page-centre relative EMU),
  client data 0xF011 (10 bytes `0a000000 01 68 <u32 Contents slot of the group object>`). No property table. ✔ (5 cases)
- Each grouped shape: container otherwise identical, except (a) FSP flags |= 0x002 (fChild: 0xA00 -> 0xA02) ✔,
  (b) its client anchor 0xF010 (8+28) is replaced by child anchor 0xF00F (len 16: left, top, right, bottom i32) ✔,
  (c) property table, 0xF122, 0xF011 client data, 0xF00D etc. untouched ✔. Each child therefore shrinks by 12 bytes.
- FSPGR / group anchor choice ✔ (b_tb_tb, b_pic, b_3, b_p2, g_orig, g_nest):
  group client anchor = union of the children's page rectangles (page-centre relative, e.g. 54..558 x 54..648 pt ->
  -3200400,-4343400,3200400,3200400 EMU). FSPGR is NOT page-centre relative: FSPGR = page top-left coordinates in EMU
  + constant, FSPGR.left = left_pt*12700 + 106,299,000 and FSPGR.top = top_pt*12700 + 105,156,000 (same constants for
  both files and for page 1 and page 2; checks: left 54 pt -> 106,984,800; 144 pt -> 108,127,800; top 54 -> 105,841,800;
  144 -> 106,984,800). Width/height of FSPGR = anchor size in EMU exactly (scale 1:1). Not tested: other page sizes
  (legal.pub), so the constants may depend on the page ~.
- Child anchor = child's page rectangle through the same offset: left = left_pt*12700 + 106,299,000, top = top_pt*12700 +
  105,156,000, right/bottom likewise (b_pic text box: 106,984,800,105,841,800,113,385,600,106,756,200). Because scale is 1:1
  the children's coordinates equal their (page + constant) position ✔. So a writer can compute FSPGR from the union and
  child anchors from each page rect, no scale needed.
- Nested (g_nest, ~): existing group's FSP flags 0x201 -> 0x203 (fGroup|fChild|fHaveAnchor), its client anchor 0xF010
  -> child anchor 0xF00F (equal to its own, unchanged FSPGR box); its inner children untouched. New outer group
  container wraps [inner group container, other shape] in z-order; new spid 1029; outer FSPGR = union (108127800,
  106984800, 112699800, 109270800).
- Enclosing lengths: group container (0xF003 of the page), drawing container 0xF002 length fields all re-summed
  (+100 bytes for a 2-child group: +8 container header +116 group shape container -12 per child) ✔; EscherStm 2654 ->
  2754 in both bulletin cases; EscherDelayStm unchanged (817 / 0).

## B. Group (Contents)
- New object of kind 48 inserted in the object table (slot = the object id used in client data 0xF011): takes the
  next free id (b: 305 = the former slot of the kind-108 font-table object, which moved to 306 and object 282 item 4,
  the reference to it, was updated; g_nest: 300, font table 300 -> 301) ✔. Rule seen: the font-table object (kind 108)
  stays the last slot; new object inserted before it ✔ (4 cases). Appending after it is untested.
- Group object body (60 bytes, identical in all 5): `3c000000 | 02 08 | 04 10 00 00 | 0c 28 00*8 | 0d 28 00*8 |
  14 20 <u32 child count> | 15 a0 <u32 len> [00 70 <u32 child id>]* | b7 20 00000000`. i.e. items 2=true, 4=0
  (16 bit), 12=0, 13=0 (8-byte zero), 20 = number of children, 21 = list of child object ids in z-order (bottom
  first), 183 = 0. No width/height items ✔.
- Table entry (index) for the group: {2: 48, 5: parent (page object, 266), 6: 258, 11: 22} (same item keys/values in all
  cases; the 30-byte entry is added to the index block, whose length field grew 1936 -> 1966) ✔.
- Children: object ids and bodies unchanged and NOT moved in the stream; only their index entry item 5 (parent) changes
  from the page object to the group object ✔ (5 children).
- Page object (kind 67, 266): item 1 (count of direct children) = old - (n-1) (3 -> 2 for 2 shapes, 3 -> 1 for 3); item 2
  list of child ids: the n grouped ids removed, group id inserted at the slot of the last grouped child ✔.
  For a group-in-group the outer group lists [inner group id, other id] ✔.
- Document counter: item 60 of object 256 incremented (2 -> 3) in every save, grouping or not (same as the §4I-b
  counter, so it is a save counter), not group related ✔.
- Contents stream length: +96 (tb_tb) / +92 (pic) / group body 60 + index entry 30 + small list shrink and id digits.
  Names shown by COM ("Group 6") are generated by Publisher, not stored (no name item found).

## C. Ungroup (g_orig -> g_ung)
- The group container 0xF003 and its group shape container (FSPGR, FSP 1028, 0xF122, anchor, client data) are removed; the
  children are placed directly into the page group container at the group's slot, in their z-order ✔ (~ one case).
- Child FSP flags 0xA02 -> 0xA00; child anchor 0xF00F replaced by client anchor 0xF010 holding the page rectangle
  (page-centre relative: left -162 pt = 0xFFE09B48 etc., i.e. inverse of A) ✔ (2 children).
- Each ungrouped child gains Escher property 0x388 = 1 (the property table grows by 6 bytes, 18 -> 19 properties) ~
  (both children of the single sample; not present before or after Group).
- A new record 0xF118 (inst 1, len 4, data `01 00 00 00`) appears in the drawing container right after the drawing
  record 0xF008 (this looks like the "regroup items" counter) ~ (one case). Absent after Group.
- 0xF008: cspids 5 -> 4, lastspid unchanged (1028, the old group spid is not reused/lowered). 0xF006: cspSaved 9 -> 8; FIDCL
  counter for the drawing stays 5 (no decrement) ✔ vs. group (+1). Ungroup does not renumber other spids.
- Contents: kind-48 object and its index entry deleted; children's parent (item 5) -> page object; page object item 1
  count + (n-1), item 2 list: group id replaced by the children ids at the same position; the font-table object slides down
  by one id (300 -> 299) and 282 item 4 follows ✔ (mirror of group). Counter item 60 +1 (save counter).
  Contents length 10168 -> 10098 (-60 body, -30 index entry, +20 list/count).
- EscherStm 1984 -> 1908.

## D. What to implement in pub-write
Group: allocate spid=lastspid+1; update 0xF008 (cspids+1, lastspid), 0xF006 (cspSaved+1, FIDCL count +1); build the
group container as above (flags 0x201, FSPGR = union + constants, anchor 0xF010, client data -> new Contents slot);
convert each child's anchor to 0xF00F (+constant) and set flags |= 2; place at topmost child's slot; re-sum all container lengths. Contents: insert kind-48 object before the font
table (renumber it and fix 282[4]), set children's parent, edit page object 266 items 1 and 2.
Ungroup: reverse; remember property 0x388=1 and 0xF118 (not yet proven necessary; Publisher happily re-reads either way is
UNTESTED - test our own output with COM before shipping).

### 4J-b writer results (our own files opened in Publisher 2010, local evidence files)
- Group: our output equals Publisher's for 2 text boxes, text box + picture and 3 shapes, apart
  from the document save counter (object 256 item 60, bumped on every Publisher save; a writer
  may leave it) and EscherStm bytes 106–107 (unused BSE bytes that change on every save) ✔.
- Ungroup: Escher property 0x388 and record 0xF118 are **not required**; Publisher opens an
  ungroup without them identically ✔ (whether it needs them on its own re-save is ?).
- The FSPGR / child-anchor offset is the same on both axes: child anchor = page-centre-relative
  value + 110,185,200 EMU, which equals the master page's item 17 ~ (only Letter portrait sampled).
- The page's `Contents` child list is **not** z-order: the group takes the old index of the
  last grouped child (b_pic: list [302, 305] while Escher order is [group, text box]) ✔.
- Removing a group object that is not directly before the font table may leave an empty
  `Contents` slot instead of renumbering; Publisher opens such files normally ✔.

## 4K. Text wrap around objects ✔ type incl. top-and-bottom and through (`gen/wrap.pub`; 2021 edit+diff, 2026-11-07)
> **2026-11-07, Publisher 2021 (edit via `TextWrap.Type` then diff against a no-op 2021 save, local evidence files):** the wrap
> type lives only in `Contents` shape-object items **4 and 5** (both 2-byte values, class 2; Escher and Quill did not change
> for any wrap value). Rule as read back by 2021 (files patched from the ORIGINAL samples, read-only open ✔):
>
> | TextWrap.Type | stored |
> |---|---|
> | 0 none | item 4 present, low byte 0 |
> | 1 top and bottom | **item 4 absent** (and item 5 absent or not 4) |
> | 2 square | item 4 = 2 |
> | 3 tight | item 4 = 3 (the usual text-box value) |
> | 4 through | item 4 **absent** and **item 5 = 4** |
>
> So "absent item 4" means **top and bottom (1)** ✔ (a picture with no item 4 reads 1; that is why the pictures in our samples
> read 1, not square). Item 4 wins when present: item 4 = 3 plus item 5 = 4 still reads 3 ✔. Item 5 values 1–3 with item 4
> absent read 1 ✔. A raw item 4 = 4 or 5 makes 2021 **refuse the file** ("cannot open"/"problem in the file") ✔ — never write
> a value above 3 in item 4. Values above 4 are rejected by `TextWrap.Type` itself (range error). The constants in the
> COM documentation are None, Square, Tight, Through and TopAndBottom (plus Mixed); numeric values are the ones in the table ✔.
> The upper byte of item 4 is noise: 2021 stores arbitrary bytes there on save (e.g. 0x0A03, 0x0103, 0xA903) and also
> rewrites it on shapes you did not touch, so mask with 0xFF when reading and write 0 ✔. 2021 also adds the true flag item 3
> to a shape whose wrap you set through automation; not needed for the type to read correctly ✔ (our patches omitted it).
> Writer rule: type 0/2/3 → item 4 = type (insert the item after item 2, before item 12, if absent: object length +4, and
> shift later offsets; or edit in place when the item exists); type 1 → remove item 4 (and item 5); type 4 → remove item 4 and
> write item 5 = 4 (same bytes as replacing item 4 by item 5, size unchanged). The earlier note below that "absent =
> square" is wrong.

- Wrap distances = Escher properties **0x0384–0x0387** (left, top, right, bottom; EMU),
  public [MS-ODRAW] `dxWrapDist…` ✔ (36,576 EMU = 2.88 pt by default).
- Layout ~ (Publisher's documented behaviour, not yet measured): a text box's lines avoid every
  object that is **in front of it** (later in drawing order), overlaps it and has a wrap type
  other than none. Square = avoid the object's frame grown by the wrap distances, on the line's
  whole height; tight = follow the object's outline (for an oval, its ellipse) grown by the
  distances. Text uses the remaining space on the side(s) with room (both sides allowed by
  default).
- Measured ✔ (Publisher's own line positions for `gen/wrap.pub`, via automation): lines that do
  not touch a wrapper are full width (one paragraph per line, 333.9 pt of text); lines whose
  vertical band meets the square-wrapped rectangle (frame 144–252 pt grown by 2.88 pt) are split
  into a **left piece and a right piece on the same baseline**, text filling the left piece
  first, then the right piece, then the next line (Publisher reports one line whose bounds span
  both pieces). Around the tight-wrapped oval the usable width changes line by line with the
  ellipse. The no-wrap rectangle does not affect the text. Lines start at the box's inner edge
  (box 72 pt + 0.05 in inset = 75.6 pt).
- Tight wrap is applied at the **vertical centre of each line** (the ellipse's width at the
  line's middle), while square wrap blocks the line's whole band ✔ (whole-band sampling around
  the oval gave 6–7 pt errors on three lines; centre sampling matches Publisher to < 2 pt).
- Pictures (COM-made and POI) have **no item 4**, which means top and bottom ✔ (not square, see the table above; this
  supersedes the old guess that matched how the POI newsletter looked). Layout of top-and-bottom (text stops above the
  object and resumes below it, nothing beside it ~) and through (text runs over the object) is not measured ?. Whether
  text boxes (default 3) push other text aside is unsampled.

### 4.6 Older variant (`lo-fdo59355-1.pub`, generation 300) ?
Same signature and length field, but a different item encoding (its first item uses type
0x5C, which is not in the table above). Treat as a separate format; analysis pending.

### 4M. Document save counter (object 256 item 60)
Evidence: COM on copies of samples/gen/bulletin.pub (local evidence files (not in the repository), Publisher 2010; Open -> Save -> Close only), item read from the Contents stream.
- Item 60 (0x3C) of the document-header object 256 is stored as type 0x20 (class 4, u32). Brand-new/our generated files carry 2 ✔ (bulletin.pub = 2).
- Item 60 is read as a plain 32-bit integer (class 4), default 0 ✔ (seen in all samples); it is not a flag or an id reference.
- Counts **edit sessions that were saved**, not individual Save calls ✔: open + Save + Close -> 3; open + Save + Save -> 3; open + add text box + Save + add second box + Save -> 3; two separate open/Save/Close sessions -> 4; open + Close without saving -> unchanged (2). Pure content changes do not touch it otherwise.
- It does not feed File > Properties "Revision number" (the COM revision property is not available for .pub files here, error) ~.
- Where it is read or what depends on it: unknown ?; nothing else in the files refers to it. No visible behaviour difference for files whose counter is unchanged: Publisher 2010 opens our writer output with the counter untouched without complaint ✔ (earlier writer tests, 4J-b). Behaviour with counter 0 or the item absent is untested ?.
- Likely purpose (inferred) ~: a modification/save-session count used for change tracking (AutoRecover/merge style bookkeeping); no evidence of conflict checks.
- Writer rule: copy the item from the source document and add 1 on the first save of an editing session only (later saves in the same session leave it), matching Publisher; never above u32; a fresh document gets 2. Leaving it unchanged is harmless as far as observed.

## 5. How this document was produced

Sources: sample files (Apache Tika, LibreOffice, and files made by the owner in Publisher 2010 and 2021), byte-level comparison of files before and after known edits made through Publisher's public COM automation interface, and the public Microsoft Open Specifications [MS-CFB], [MS-ODRAW], [MS-OLEDS], [MS-OLEPS].

### 5.1 Observation notes

- 2026-11-07 (columns and wrap type, observed by editing through COM): drove real Publisher 2021 (Office16, version 16.0)
  to set `TextFrame.Columns`/`ColumnSpacing` and `TextWrap.Type`, saved, diffed all streams against a no-op 2021 save: columns = Escher 0xF122
  properties 0x8C/0x8D (§4I), wrap = Contents items 4/5 (§4K); confirmed by patching only those bytes into copies of the original samples and
  reading them back read-only in 2021. **2021 plain save versus our 2010-made samples (no-op Open/Save/Close of `gen/bulletin.pub` and `gen/columns.pub`)**
  changes structure, not only noise: (a) Escher grows (+204 / +132 bytes): the first shape's 0xF00B gains 0x180, 0x1CB (=25,400), 0x285, 0x2CB, 0x2CC, 0x2CE,
  0x2CF and the page-level 0xF122 gains 0x1FF (=0x400000), 0x295–0x29A; every text box's 0xF122 gains 0x23F (=0x40000), 0x83F, 0x87F, 0x8BF (=0x10000 each);
  (b) Quill stream +512 bytes (STSH instance 1 +792 bytes), `MCLD` items 0x200–0x203 recomputed, BTEC/BTEP positions shift by +2/+2;
  (c) Contents: object 256 item 60 (save counter) +1, kind-1 shapes gain flag items 43 and 44, the kind-79 object loses items 7, 9, 10, 11, 18, 19, one kind-108 object
  changes a 16-bit value, object-index item 11 of two kind-77 objects 24 → 26; (d) both summary property streams are rewritten (version word 0x0206 → 0x000A);
  (e) per-save noise: three bytes at offset 0x11–0x13 of FONT and every STSH chunk, the high byte of item 4 on kind-1 shapes, a 16-bit value in the kind-108 object, 2 bytes
  in an Escher blob record (offset 0x6A). Writers that must re-save an already 2021-saved file need none of this; writers starting from 2010 files get a file 2021 reads
  fine without any of (a)-(e) (all confirmation files were pure 2010-style files with only the bytes above changed, plus a rewritten CFB container, and 2021 opened them).

## Sample corpus
- `tika-testPUBLISHER.pub`: Apache Tika test data (Apache-2.0).
- `lo-fdo59355-1.pub`: LibreOffice bug fdo#59355 sample (MPL-2.0 repo).
- Excluded on purpose: LibreOffice `EDB-*.pub` (Exploit-DB malicious samples).

## 4L. Adding a new text box (Publisher COM AddTextbox then Save(), our samples only; evidence in local evidence files (not in the repository))
Evidence: local evidence files Base file = copy of samples/gen/bulletin.pub (orig.pub, 2 pages, text boxes = Contents objects
299, 302 on page 1 (obj 266), 303, 304 on page 2 (obj 296), picture 300). Cases (each a fresh copy, Open -> AddTextbox -> Save -> Close):
t1 "Hello" p1 (200x50 pt at 72,600) | t2 "Hello\rWorld" p1 | t2p2 same on page 2 | t3 empty box | t4 two boxes ("AAA", "BBBB")
| t5 three boxes (100x40 pt at y=660) | t6 box fully inside box 302 | t7 box overlapping 299 and picture 300 | t8 two overlapping new
boxes | t9 reopen t1 and add a second box in a second session. Marks: ✔ seen in >=2 cases, ~ seen once. Scripts: run.ps1, run2.ps1,
dump.py (Escher/Quill dump), idx.py, mcld.py; *.esc *.q *.tree = dumps.
Text content changes nothing outside Quill: Contents and Escher of t1, t2, t3 are byte-identical ✔.

### 4L.1 Escher (EscherStm)
- New shape container 0xF004 (len 320) is APPENDED as the last child of the single page-group container 0xF003 of the drawing ✔ (t1..t9).
  It goes to the end of the whole drawing even for a page-2 box (t2p2 Escher == t1 Escher) ✔; the page is expressed only in Contents.
- Container content, in order: FSP 0xF00A (inst 0xCA = 202 text box, flags 0x0A00 = fHaveSpt|fHaveAnchor) ✔; property table 0xF00B
  (18 props, 108 bytes); tertiary property table 0xF122 (18 props, 108 bytes); client anchor 0xF010 (28 bytes);
  client data 0xF011 (10 bytes); client textbox 0xF00D (10 bytes). 10 shapes of this kind checked ✔: all existing text boxes and
  all new ones are byte-identical except 4 fields: spid, anchor numbers, client-data object id, lTxid / 0xF00D story. So a writer can
  clone any existing text-box container and patch those 4 fields ✔. Everything else is a Publisher default (copy):
  - 0xF00B props (id=value): 0x80 lTxid = STORY NUMBER (computed, see 4) ; 0x81..0x84 text insets L/T/R/B = 36576 (0.04 in);
    0x85 wrap = 1; 0xBF 0x20000; 0x181 0x8000007; 0x183 0x8000007; 0x1BF 0x100000; 0x1C0 0x8000000; 0x1C2 0x8000007; 0x1FF 0x80000;
    0x201 0x8000004; 0x384..0x387 (wrap distances) = 36576. (fill/line/shadow = scheme colours, no line.)
  - 0xF122 props: 0x8D=73152, 0x17F=0x400040, 0x1FF=0x400040, 0x540/0x542, 0x57F..0x642 (colour-scheme triplets 0x8000000, 0x8000007,
    0x80000), 0x6FF=0x20002. All defaults, copy verbatim.
  - FSP flags 0xA00, no fChild.
- spid = drawing's lastspid + 1 (1029 -> 1030; second box 1031, third 1032) ✔ (t1,t4,t5,t8,t9). 0xF008: cspids += n, lastspid = new spid ✔.
  0xF006 (drawing group): cspSaved (3rd u32) += n (10 -> 11) ✔ and the FIDCL count for this drawing (2nd u32 of the pair, 6 -> 7) += n ✔;
  maxspid (3075), dgid and the other FIDCL pairs unchanged ✔. Same as the group experiments (4J-b).
- Client anchor 0xF010 = `1c000000 | 01 20 L | 02 20 T | 03 20 R | 04 20 B` in EMU relative to the PAGE CENTRE (Letter: 3886200, 5029200):
  L = left_pt*12700 - 3886200, T = top_pt*12700 - 5029200, R = (left+w)*12700 - 3886200, B = (top+h)*12700 - 5029200 ✔
  (t1: 72,600,200,50 pt -> -2971800, 2590800, -431800, 3225800). Page 2 uses identical numbers ✔.
- Client data 0xF011 = `0a000000 01 68 <u32 Contents object id>` (305 for the first new box; next free id) ✔.
- Client textbox 0xF00D = `0a000000 01 20 <u32 story number>` ✔ (equals lTxid).
- Container lengths re-summed: 0xF003 and 0xF002 += 328 per box (+8 header +320) ✔. EscherStm 2654 -> 2982 (+328); 3310 with two. EscherDelayStm untouched.

### 4L.2 Contents
- New object (kind 1) takes the next free id = the old font-table slot (kind 108, always last); font table moves to id+1, and object 282
  item 4 (reference to it) follows ✔ (305->306 for one box, ->307 for two, ->308 for three). Same rule as groups.
- Body of the new object (54 bytes, identical for every box ✔ and identical in form to existing boxes):
  `36000000 | 02 08 | 04 10 03 00 | 0c 28 00*8 | 0d 28 00*8 | 27 20 <story> | aa 20 <width EMU> | ab 20 <height EMU> | b7 20 00000000`
  (seen in 2021 files: full meanings in §4.3c; item 52 defaults to 1 so a text box omits it)
  i.e. items 2=true, 4=3 (u16), 12=0, 13=0, 39=story number, 170 = width EMU (200 pt = 2540000), 171 = height EMU (50 pt = 635000), 183=0.
  NO item 40 (chain position) for a single box ✔; item 39 is the only link to the story. Insets/wrap/fill are NOT here (Escher props).
  Body is stored in slot order, i.e. directly after the last old text-box body and before the font table object ✔.
- Index entry (30 bytes) for the new slot: {2: 1, 5: parent page object, 6: 259, 11: 22} ✔ (same constants as existing boxes);
  parent = page 1 object 266 or page 2 object 296 ✔ (t2p2). Index block length += 30 (1936 -> 1966); index block items 1 and 2
  (both u32 = 306/305 in orig) each += 1 (slot count / last slot) ✔.
- Page object (kind 67; 266 or 296): item 1 (child count) += 1; item 2 child-id list gets the new id APPENDED at the end ✔ (t1,t2p2,t4,t5).
- Story registries (objects 283 kind 97 and 284 kind 101, children of 282): both have item 1 = entry count (+1 per box, 4 -> 5) ✔ and an
  item-2 list of records in creation order (append):
  - 283: record {1: story number, 3: new object id} ✔.
  - 284: record {1: story number, 7: ordinal (1-based = count after adding; 5 for first new), 8: 3407386236 = 0xCB18967C (constant,
    copied from existing entries), 9: same constant} ✔ (t1,t4,t5,t8,t9). Copy items 8/9 from an existing record.
- Overlap bookkeeping on OLDER text boxes (~ reproducible but meaning inferred; seen in 2021 files: item 70 is the size of an overlap list and item 71 the list of object numbers, both written only when the list is non-empty, item 45 is one of five per-shape flag bytes, §4.3c): when the new box's page rectangle intersects an existing
  TEXT box (kind 1 with item 39) on the same page, that old box body gains items 45 (flag true), 70 (u32 count of overlapping newer
  objects) and 71 (list of those object ids, `00 68 <u32>` each), inserted between item 39 and item 170 (body grows 20 bytes for one id, +6 per extra id)
  ✔ (t1 box 302 partial overlap; t6 full containment in 302; t7 box 299; t8 older NEW box gets the newer new box id; t4 box 302 lists
  [305,306], count 2; t9 keeps the earlier [305] and adds nothing as the new box does not overlap). The picture (300, also kind 1) overlapped in
  t7 was NOT modified, only the text box; no overlap (t2p2 on page 2, t5, t9) -> no change anywhere ✔. Not tested whether Publisher
  needs it when absent (probably it is wrap/overlap cache); safest for a writer: emit it exactly as above.
- Document header object 256 item 60 (save counter) +1 per Publisher save (2 -> 3) - not box related, a writer may leave it. The header
  record also stores the file's original path, so offsets after it shift by path length; irrelevant.
- Contents header: stream length (0x08) and index offset (0x1A) updated; every body after the changed page object shifts (rebuild stream).

### 4L.3 Quill (QuillSub/CONTENTS)
- Chunk set/order and the directory are unchanged: still 15 entries, header 0x0C = 15, 0x0E = 0x300, 0x10 = 0x200, no new chunk ✔.
  0x14 = stream length; unchanged for 1-3 boxes (0x5800) but 0x5A00 (+0x200) for two boxes - the last chunks (MCLD grows 146 bytes
  per box, STRS +8) crossed the 512 boundary; the length stays a multiple of 512 ✔ (t4, t5).
- The new story is APPENDED at the end of TEXT ✔ (it owns the highest story number, §3.4 rule): text + final `\r` (UTF-16LE).
  "Hello" -> `Hello\r` (6 chars); "Hello\rWorld" -> `Hello\rWorld\r` (12); EMPTY box -> a single `\r` (1 char) ✔ (t3). So an empty box still has a story.
- STRS: count += 1, new u32 = story length in chars appended after the last length, the trailer gains one `04 00 00 00` (len 0x36 -> 0x3E) ✔.
  Later chunk offsets shift by the TEXT growth (STSH0..2 etc. +12 bytes for 6 chars) while FDPP/FDPC pages stay at their 512-aligned offsets
  if still room (0x4C00/0x4E00) ✔; BTEP/BTEC second position = 0x200 + TEXT length ✔.
- FDPC (character runs): the page keeps its old run records; new runs appended at the end, `end` values are absolute stream offsets
  (0x200 + 2*chars): for "Hello" -> ends 1470 (text), 1472 (`\r`); the text run record is the 16-byte `10000000 | 12 22 0409 0000 | 3e 22 0409 0000`
  (items 0x12 and 0x3E = LCID 0x0409), the `\r` run record the 10-byte `0a000000 | 12 22 09040000` (item 0x12 only) - both are the SAME records
  already used by the existing plain paragraphs, so no new record bytes needed ✔ (t1,t2,t4,t5,t8). Empty box = just the `\r` run ✔ (t3).
  "Hello\rWorld": ONE text run for `Hello\rWorld` (end 1482), then a `\r` run (1484) - the inner `\r` shares the text run ✔ (t2, t2p2).
  The page header (n) += runs; offset array grows; record region unchanged.
- FDPP (paragraph runs): one run per paragraph ending at each `\r`, record = the 4-byte empty record `04000000` (all defaults) ✔ (ends
  1472 for Hello; 1472 and 1484 for Hello/World; 1462 for empty).
- SYID (~ meaning inferred): `u32 H, u32 n, n x u32 story numbers ascending`; n += 1, the new story number appended ✔ (t1,t4,t5,t8,t9).
  H = a story-number high-water mark: 6 (orig), 8 (t1: story 7), 9 (t4: stories 7,9), 10 (t5: 7,9,10), 9 (t8), 10 (t9: 7,9). Fits
  "next number = H+1; the FIRST AddTextbox in a session also burns one number" (t4/t5/t8/t9 second box = 9 not 8; t9 started from H=8 and got 9).
  Writer rule that is consistent with all files: story = H + 1, then set H = story (Publisher itself sets H one higher for a lone box).
  Story numbers therefore have gaps (8 never used) - reading must keep using the sorted-distinct-numbers rule.
- MCLD: header `u32 n, u32 n, n x u32 (1..n)`, then n records in story order; the new record is appended ✔. Record = 12-byte block
  `0c000000 | 00 0a | 01 22 f87c0300` (item1 = 228600, constant), `u32 1` (frame count), then ONE 0x82-byte frame block - the same 146 bytes per
  story for every story. Frame block = copy of an existing frame (items 6..9 insets 36576, 10 = 82676, 21 = 1, 22 = 9525 ... all defaults)
  with: item 4 = box width EMU, item 5 = box height EMU ✔ (2540000 / 635000), and items 0..3 (see below). Text content does not matter (t1 == t2 == t3).
  Items 0..3 = top, left, bottom, right of the box in an unknown unit, linear in the page-point position (fit over 10 boxes, max residual 0.8,
  i.e. rounding): item0 = 0.5696*top_pt + 4715.1; item2 = 0.5702*bottom_pt + 4714.2 (Publisher gave 5057/5085 for top 600/bottom 650);
  item1 = 0.5692*left_pt + 4766.2; item3 = 0.5695*right_pt + 4766.1 (4807/4921 for left 72/right 272). ~ (cached layout; whether Publisher
  accepts a wrong value is untested; copy values from a same-size existing frame if exact reproduction is not required.)
- PL, FONT, STSH, SGP, INK unchanged ✔. No change in other CFB streams except SummaryInformation (thumbnail etc.).

### 4L.4 Story number allocation (writer summary)
First free number above the highest used (here 7 after {2,4,5,6}); Publisher's second box in a session got 9 (8 skipped). Any number
greater than all existing ones keeps the new text last in TEXT/STRS/SYID/MCLD order, which is what the sorted-number mapping requires.
Story number must be identical in: Escher lTxid (0x80), Escher 0xF00D, Contents body item 39, registry 283 and 284 records, SYID list.
Object count/ids: page child count, 283/284 item 1, index items 1/2, object 282 item 4.

### 4L.5 Checklist for pub-write "add text box"
1. Escher: allocate spid, clone text-box container, patch spid/anchor/object id/story (lTxid + 0xF00D), append, bump 0xF008, 0xF006 (cspSaved, FIDCL), lengths.
2. Contents: new kind-1 object before font table (renumber font table, fix 282[4]), index entry + header counts, page list/count, 283, 284, overlap items on older text boxes.
3. Quill: append story text (+`\r`), STRS, SYID, MCLD record, FDPC/FDPP runs + BTEP/BTEC, shift chunk offsets, keep 512 alignment and stream length multiple of 512, header 0x14.
### 4L.6 Formatting of a new box's final paragraph mark (COM on our own copies; evidence in local evidence files (not in the repository))
Cases: (i) text "Georgia text" set, whole TextRange -> Georgia 24 red; (ii) first word bold; (iii) two paragraphs, first centred, second right;
(iv) size/font on `Characters(1, Length-1)`; (v) box saved, REOPENED, then the whole paragraph formatted.
- In the session that created the box, TextRange.Length does not count the final `\r`, so a range format covers the text only: the
  mark keeps the default character format (Calibri 10, no bold, black) in (i), (ii), (iv) ✔. In FDPC the mark is its own run (text run
  ends 1484, mark run 1486 in (i); (ii) bold run, plain run, mark run) ✔.
- Paragraph (FDPP) formatting does include the mark: in (iii) the second paragraph's run ends at its `\r` and carries right alignment ✔.
- After reopening, the same paragraph carries the mark in its range (Length counts it); formatting the whole paragraph (v) gave the mark
  Georgia 24 as well and the FDPC text+mark ended in one run ~. This is the rule already used for existing boxes (whole paragraph = text + mark).
- Writer rule: a whole-paragraph format (what the editor produces) extends to the mark; a partial format leaves the mark at the defaults.
  Both match what Publisher stores for the equivalent operation.

Not tested: other page sizes (centre constants), rotated/other-orientation boxes (AddTextbox orientation != 1), formatting, boxes added to a master page, files that already have >20 chunks.


## 4N. Adding a new shape (AutoShape / line) (Publisher COM AddShape / AddLine then Save(), our samples only; evidence in local evidence files (not in the repository))
Base = a copy of `gen/bulletin.pub` (2 Letter pages; shapes already present: 3 text boxes + picture on page 1, 2 text boxes on page 2).
Cases, each a fresh copy (Open -> add -> Save -> Close): r1 rectangle p1 (72,660,150x60 pt) | r2 oval, same box | l1 horizontal line (72,690)-(272,690)
| l2 line (72,720)-(272,680), i.e. going up-right | l3 line (72,680)-(272,720), down-right | l4 line (272,720)-(72,680) | l5 line (272,680)-(72,720)
| rp2 rectangle on page 2 | two rectangle + oval in one session | ov rectangle over text box 302 | lov line crossing text box 302 | pov rectangle over the
picture only | ovs rectangle then oval overlapping it | sty rectangle with Fill.ForeColor.RGB, Line.ForeColor.RGB, Line.Weight=3 | lsty line with
colour and weight 2 | nop open + Save only (baseline). Marks: ✔ seen in 2 or more cases, ~ seen once or inferred, ? unknown.
Publisher reports every one of these as Type 6 (autoshape); a default shape from COM has Fill.Visible = false and a 0.75 pt black outline (it may differ
for a shape drawn with the mouse, untested ~).

### 4N.1 Escher (EscherStm)
- One new shape container 0xF004 is APPENDED as the last child of the single page-group container 0xF003, for page 1 and page 2 alike ✔ (the page
  is expressed only in Contents). Order inside: FSP 0xF00A, property table 0xF00B, tertiary table 0xF122, client anchor 0xF010, client data 0xF011.
  There is NO client textbox 0xF00D and no lTxid (0x80) property; the text insets 0x81..0x84 are still present.
- FSP (8 bytes): `spid u32 | flags u32`. Instance (shape type) = 1 rectangle, 3 oval, 20 line ✔ (matches §4D). Flags = 0x0A00 (fHaveSpt|fHaveAnchor), plus
  for a line **0x40 = flip horizontal, 0x80 = flip vertical** ✔ (l2 = 0xA80, l5 = 0xA40, l4 = 0xAC0, l1/l3 = 0xA00).
  Flip convention (from Publisher's own HorizontalFlip/VerticalFlip read-back): the anchor is always the normalised box (left <= right, top <= bottom);
  start-left/end-right going DOWN = no flip, start-left going UP (l2) = flipV, start-right going DOWN (l5) = flipH, start-right going UP (l4) = both ✔.
  A horizontal line is height 0 with no flip (top = bottom) ✔.
- spid = drawing's lastspid + 1 (1029 -> 1030; second shape 1031) ✔. 0xF008: cspids += n, lastspid = new spid ✔; 0xF006: cspSaved (3rd u32) += n (10 -> 11,
  12 for two) and the FIDCL count of this drawing (2nd u32 of its pair, 6 -> 7, 8 for two) += n ✔; maxspid (3075) and the other pairs unchanged ✔. Same as §4L/§4J-b.
- Client anchor 0xF010: identical formula to §4L ✔: `1c000000 | 01 20 L | 02 20 T | 03 20 R | 04 20 B`, L = left_pt*12700 - 3886200, T = top_pt*12700 - 5029200,
  R = (left+w)*12700 - 3886200, B = (top+h)*12700 - 5029200 (Letter page centre; checked for the rectangle, oval and four line cases, page 2 identical numbers).
- Client data 0xF011 = `0a000000 01 68 <u32 Contents object id>` (305 for the first new object) ✔.
- Property table 0xF00B (instance field = property count; 6 bytes per property `id u16, value u32`, ascending ids). Rectangle/oval = 15 properties (len 98 with header),
  identical for both ✔:
  0x81..0x84 = 36576 (insets) | 0x181 = 0x08000007 | 0x183 = 0x08000007 | 0x1BF = 0x00100000 (not filled) | 0x1C0 = 0x08000000 | 0x1C2 = 0x08000007 |
  0x1FF = 0x00080008 (line on) | 0x201 = 0x08000004 | 0x384..0x387 = 36576.
  **Line** = the same table WITHOUT 0x181 and 0x183 (13 properties) ✔ (l1..l5, lsty).
- Tertiary table 0xF122: rectangle/oval 12 properties (len 80 with header): 0x1FF = 0x00400040, 0x540 = 0x08000000, 0x542 = 0x08000007, 0x580 = 0x08000000,
  0x582 = 0x08000007, 0x5C0 = 0x08000000, 0x5C2 = 0x08000007, 0x600 = 0x08000000, 0x602 = 0x08000007, 0x640 = 0x08000000, 0x642 = 0x08000007, 0x6FF = 0x00020002.
  Line: the same WITHOUT 0x1FF (11 properties, len 74) ✔. All constants, copy verbatim.
- Per-shape styling (what the editor needs, sty / lsty ✔ plus §4D): a literal fill colour replaces 0x181 with `R | G<<8 | B<<16` (flag byte 0; COM RGB 255 = red stored 0x000000FF)
  and turns 0x1BF into 0x00100010 (filled); a literal line colour replaces 0x1C0 the same way; a non-default line weight ADDS 0x1CB = weight in EMU
  (3 pt = 38100, 2 pt = 25400) between 0x1C2 and 0x1FF (+6 bytes). Absent 0x1CB = 0.75 pt. Everything else stays at the defaults above. 0x183 (fill back colour) did not change when the fill was set.
- Sizes: container (without its 8-byte header) = 248 bytes rectangle/oval, 230 line, +6 per added property. 0xF003 and 0xF002 lengths and EscherStm grow by container + 8:
  +256 (rectangle/oval), +238 (line), +262 with 0x1CB or colour+weight on a rectangle (2654 -> 2910 / 2892 / 2916 seen) ✔.
- Cloning: every existing shape container of the same kind can be cloned and patched in exactly these places: spid, FSP flags (line flip), anchor numbers, client-data object id,
  and the 0xF00B/0xF122 styling values above ✔. If the file has no shape of that kind, nothing needs further research: build from the tables above (the whole container is listed, nothing unknown).

### 4N.2 Contents
- The new object takes the next free id = the old font-table slot; the font table (kind 108, always last) moves to id+1 and object 282 item 4 follows (305 -> 306; 307 for two) ✔ (same rule as §4L).
- Rectangle/oval = kind 1, body 54 bytes, identical for rectangle and oval ✔ (r1 = r2 = sty = rp2):
  `36000000 | 02 08 | 04 10 03 00 | 0c 28 00*8 | 0d 28 00*8 | 34 20 00000000 | aa 20 <width EMU> | ab 20 <height EMU> | b7 20 00000000`
  i.e. items 2 = true, 4 = 3, 12 = 0, 13 = 0, **52 = 0** (the slot a text box fills with item 39; a shape has no story), 170 = width EMU (150 pt = 1905000), 171 = height EMU, 183 = 0.
  This is also the form of the existing drawn shapes in `gen/shapes.pub` and `gen/bulletin.pub` ✔.
- Line = kind **32**, body 36 bytes ✔ (l1..l5, lsty, lov): `24000000 | 02 08 | 04 10 00 00 | 0c 28 00*8 | 0d 28 00*8 | b7 20 00000000` (items 2, 4 = 0, 12, 13, 183; NO items 52/170/171:
  the size comes from the Escher anchor only). Line colour, weight and flips are not in Contents at all.
- Index entry (30 bytes): rectangle/oval `{2: 1, 5: parent page object, 6: 258, 11: 22}`; line `{2: 32, 5: parent, 6: 258, 11: 22}` ✔ (item 6 is 258 for shapes; text boxes use 259, the picture
  258 with item 11 = 1). Parent = page 1 object 266 or page 2 object 296 ✔ (rp2). Index block length += 30, its items 1 and 2 (slot count / last slot) += 1 each, per object ✔.
- Page object (kind 67): item 1 (child count) += 1 and the new id appended to the item-2 list ✔ (6 bytes per child; page body 126 -> 132).
- NO story registries: objects 283 and 284 are unchanged for shapes ✔ (they are touched only by text boxes); no Quill/story number is allocated.
- Document header object 256 item 60 (save counter) +1 per Publisher save, as before; header 0x1E and the index offset/length move with the rebuild (~ derived values, recompute).
- Overlap bookkeeping on OLDER text boxes (kind 1 with item 39 only):
  - New rectangle over text box 302 (ov) -> that box gains items 45 = true, 70 = 1, 71 = [new id], the same triple as §4L ✔ (identical bytes in two experiments: ov here, t1/t6/t7 in §4L).
  - New LINE whose bounding box crosses text box 302 (lov) -> that box gains only **item 43 = true** (no 45/70/71) ~ (one case; whether it is the bounding box or the true crossing is untested).
  - A shape over the picture (pov), a shape over another shape (ovs, two), and shapes with no overlap (r1, r2, l1..l5, rp2, sty) change nothing ✔. The new shape's own body never gets overlap items.
  - Not tested: whether Publisher misbehaves when the items are absent (they look like a wrap/overlap cache); safest is to write them as above.

### 4N.3 Quill (QuillSub/CONTENTS)
- No change ✔ (all 15 cases): chunk set, directory, TEXT, STRS, SYID, MCLD, FDPC/FDPP are semantically identical. The raw bytes still differ in about 420 places in every case
  INCLUDING the no-op open+Save baseline (423 bytes: pointer-like garbage inside the STSH chunks, FONT and the unused tail after the last chunk, different on every save); a writer
  should leave the stream untouched. The Escher stream has one similar volatile byte (offset 106) that also changes on a no-op save.
- Other streams: only SummaryInformation (thumbnail) changed besides the above; EscherDelayStm, Objects, VBA etc. untouched.

### 4N.4 Constant / per-shape / unknown summary
- Constant (copy): both property tables except the styling properties, FSP flags 0x0A00, client-data header, body bytes of Contents objects, index items 6 and 11, the flip-less line.
- Per shape (formula): spid = lastspid+1; FSP instance by kind (1 / 3 / 20); line flags 0x40/0x80 from direction; anchor from the page rectangle (above); object id = next free id; width/height EMU
  (rectangle/oval only) = pt*12700; parent page; fill/line colour, 0x1CB weight.
- Unknown: shapes other than rectangle/oval/line (other AutoShape types probably only change the FSP instance and may add adjust-value properties 0x147..), arrows/dash styles on lines,
  what item 52 means (observed: a u32 whose default is 1, so 0 is written for a shape without a story; "text-frame indicator" ~, see §4.3c), whether a mouse-drawn shape starts with a visible fill, behaviour of the overlap items when absent.

### 4N.5 Checklist for pub-write "add shape / line"
1. Escher: allocate spid = lastspid+1; clone an existing shape of the same kind, or build the container from 4N.1; set FSP instance (1/3/20) and, for a line, flip flags; patch the anchor,
   the client-data object id and any styling properties; append to the page group; bump 0xF008 (cspids, lastspid), 0xF006 (cspSaved, FIDCL count) and the 0xF003/0xF002/stream lengths.
2. Contents: new object (kind 1 body 54 bytes with item 52 = 0 and 170/171, or kind 32 body 36 bytes) in the font table's slot; move the font table to id+1 and fix 282[4]; index entry
   (items 2, 5, 6 = 258, 11 = 22) and index header counts; page child count and list; add overlap items 45/70/71 (rectangle/oval) or 43 (line) to older text boxes it overlaps; rebuild offsets/lengths.
3. Quill: nothing. Do not touch 283/284.

Not tested: other page sizes (centre constants), the other AutoShape types, line arrowheads and dashes, vertical lines, rotated shapes, shape text (AddShape then typing text, which would need a story),
overlap with a line crossing a text box in other geometries, shapes drawn with the mouse, adding on a document with no existing shapes (the tables above should be enough).


## 4P. Adding, duplicating, deleting and moving pages (real Publisher 2021 COM, then Save(); our samples only; evidence in local evidence files (not in the repository))
Publisher 2021 (Office16, version 16.0, COM) drove copies of a **2021 baseline** (`base2021.pub` = `gen/bulletin.pub` opened, saved, closed with no edit; 2 pages: page 1 = 2 text boxes + picture, page 2 = 2 text boxes)
and `base2021-ab.pub` (the same for the 1-page `gen/apple-banana.pub`). Cases (each a fresh copy, Open -> one operation -> Save -> Close): `Pages.Add(1, after 2)`, `Add(1, after 1)` (middle), `Add(1, after 0)` (first),
`Add(2, after 2)`, `Pages(2).Duplicate()`, `Pages(1).Duplicate()` (page with a picture), `Pages(2).Delete()`, `Pages(1).Delete()`, `Pages(2).Move(1, False)`, `Pages(1).Move(2, True)`, and `Add(1, after 1)` on the 1-page file.
API facts (COM object model, observed): `Pages.Add(count, after)` (after = 0 puts it first), `Page.Duplicate()` (copy lands right after the source), `Page.Delete()`, `Page.Move(index, after_bool)` (False = before page `index`, True = after it).
Marks: ✔ seen in two or more cases and **confirmed by patching** a copy of the baseline and reading it back in 2021 (page count, order, shape names/positions/text, re-save); ~ seen once or inferred; ? unknown.
Each operation also rewrites the usual per-save noise (save counter, kind-1 items 43/44, three bytes in FONT/STSH, Escher byte 0x6A, thumbnail); a writer ignores it.

### 4P.0 Overview: which streams change
| Operation | Contents | Escher | Quill | other |
|---|---|---|---|---|
| Add blank page | yes (3 new objects + document list) | **no change** (a page with no shapes has no Escher presence) ✔ | no (only save noise) ✔ | none |
| Move page | **only the page order list of object 256** ✔ | no | no | none |
| Delete page | yes (page + all descendants removed, registries, counts) | the page's shape containers removed, counts lowered ✔ | stories **may stay as orphans** ✔ (Publisher itself removes them) | picture: BSE dropped, delay stream emptied (Publisher) |
| Duplicate page | yes (page + clones of every child, registries) | one cloned shape container per child ✔ | one new story per cloned text box ✔ (TEXT, STRS, SYID, MCLD, FDPC/FDPP, BTEP/BTEC) | picture: BSE `cRef` +1 |

### 4P.1 What a page is, and how order is stored ✔
- A document page is a kind-67 object (parent 256) with three children **kind 96** (referenced by page item 9) and **kind 119** (item 11), plus its shapes. Page body items: 5 = size record {1,2} (22860000 x 22860000, copy), 6 = 8 bytes, 9 / 11 = refs to the 96 / 119 children, 13 = ref to master page A (object 263), 14 / 15 = empty strings (`0000`), 16 = 5, 17 = size record {110185200 x 110185200}; **item 1 (shape count) and item 2 (list of the page's direct shape children, ids in z-order) are present only when the page has shapes** (a blank page has neither) ✔. Page 1 of the original files also has item 8 = a kind-77 child; Publisher does not copy it to new pages ✔.
- **The page order is the order of the ids in object 256 item 2** (a list of refs: master page 263, then the document pages in order, then the 4-5 kind-67 objects whose item 6 is zero) ✔ (all add/move/delete/duplicate cases). Object 256 item 1 = length of that list ✔.
- **Correction to §4.5 and to `pub-format` (`assemble`): the page order is NOT the low 32 bits of page item 6.** Item 6 (8 bytes) is `(serial << 32) | 2` for every page Publisher adds, where `serial` is the value object 256 item 35 had when the page was created (original pages: serial 0 / 1 with low word 1 / 2). So a page inserted first, in the middle or at the end all carry low word 2, and a moved page keeps its item 6 unchanged ✔ (a_first, a_after1, m_*). Reader rule that fits every file: **document pages = the kind-67 objects in 256 item 2 with a non-zero item 6, in the order of that list** (the "non-zero item 6" test for telling pages from master/helper pages still holds ✔).
- Object 256 **item 35** = a page serial counter: +1 for every added/duplicated page (2 -> 3 -> 4 for `Add(2)`), **not decremented by Delete** ✔ (x_del1, x_del2 leave it at 2). Item 60 (save counter) +1 per session as in §4M.
- Object ids are slot numbers in the index; deleted slots are left empty (a `0x78` list slot); trailing empty slots are dropped from the index (index items 1 and 2 = last used slot + 1 and last used slot) ✔.

### 4P.2 Add a blank page (`Pages.Add`) ✔
New objects (Contents only) take the ids of the font table slot (kind 108, last object) and upwards, the font table moves behind them, and object 282 item 4 follows (same rule as §4L/§4N): for one page `page = F, kind96 = F+1, kind119 = F+2, font table = F+3`; for `Add(2)` the second page takes `F+3, F+4, F+5` and the font table `F+6` ✔ (a_after2/a_after1/a_first/a_two/ab_add).
- Page body (90 bytes in our case): items 5, 6, 9, 11, 13, 14, 15, 16, 17 as in 4P.1, item 6 = `(old item 35 << 32) | 2` (8589934594 = serial 2 on the 2-page file; 4294967298 = serial 1 on the 1-page file) ✔. Index entry `{2: 67, 4: offset, 5: 256, 6: 258, 8: true, 11: 1}`.
- Kind 96 body `0a000000 | 05 20 01000000` (item 5 = 1); index `{2: 96, 5: page, 6: 258, 11: 1}` ✔. Kind 119 body: always the 296-byte variant `{12: 2, 17?, 23: 1}` with the web-form strings (items 14, 15, 16, 19, 24, same text as the 2010 pages), byte-identical to the other pages' kind 119 (every kind 119 in every case file is 296 bytes). **CORRECTED: a bare 20-byte `{12: 2, 23: 1}` body (the earlier note said 2021 read it fine) makes Publisher 2021 refuse the file ("cannot open"); pub-write copies an existing page's kind 119.** Index `{2: 119, 5: page, 6: 258, 11: 1}`.
- Object 256: item 2 gets the new id inserted at the requested position (after page N means directly after document page N's id; after 0 = directly before the first document page) ✔; item 1 +1; item 35 +1; item 60 +1.
- Master page link: new page item 13 = 263 (master "A"), unchanged ✔. Nothing else references the page.
- Escher, Quill, EscherDelayStm: byte-identical apart from save noise ✔ (the empty page has no drawing, no spid, no story). The Escher stream has only three drawings (dgid 3 = master, dgid 2, dgid 1 = all document pages); every document-page shape lives in dgid 1, in one flat list, and the page is expressed only in Contents (as §4L).
- Reproduced: patching base2021 as above gives Contents that equals Publisher's in every item except noise (a_after2, a_after1, a_first, a_two); 2021 reads the right count/order. Not needed for 2021 to accept the file (tested, still opens and orders by the list): item 35 +1, the save counter, a unique serial (a duplicate serial was tolerated) ✔ (p_min). Keep them anyway.
- 1-page file: same recipe; the new page gets serial 1 (`4294967298`) ✔ (ab_add, p_ab).

### 4P.3 Move a page (`Page.Move`) ✔
Contents: **only object 256 item 2 changes** (the id is removed and re-inserted); no item of the page or of any shape changes (item 6 is NOT renumbered), nothing in Escher/Quill ✔ (m_2to1, m_1to2b; patched: p_m21, p_m12 opened and ordered correctly, Contents equal to Publisher's except noise).
`Move(index, after)` semantics: `Move(1, False)` on page 2 = before page 1 (swap); `Move(1, True)` = after page 1 (no change); `Move(2, True)` on page 1 = after page 2 (swap); `Move(2, False)` on page 1 = no change.

### 4P.4 Delete a page (`Page.Delete`) ✔
Publisher removes the page object and **every descendant by index parent** (kind 96, kind 119, shapes, a picture's kind-102 child, the kind-77 object under page 1): in x_del2 objects 296, 297, 298, 303, 304 (5 objects), in x_del1 266, 267, 268, 295, 299-302 (8 objects). Their slots become empty (and trailing empties are trimmed from the index).
Other Contents changes ✔: object 256 item 2 loses the id, item 1 -1, **item 35 unchanged**, item 60 +1; **registry 283 (kind 97) drops the records whose item 3 is a deleted object, registry 284 (kind 101) drops the records of those stories; item 1 of both = new count; the remaining 284 ordinals (item 7) are NOT renumbered** ✔ (x_del1, x_del2). Publisher also moves the font table into the lowest freed slot and updates 282 item 4 (296 / 266): not required (below).
Escher ✔: the page's shape containers (found by their client data = Contents object id) are removed; 0xF008 cspids -n, **lastspid unchanged**; 0xF006 cspSaved -n, **FIDCL count of the drawing unchanged** (it follows lastspid, not the live count); 0xF003 / 0xF002 lengths -= removed bytes ✔.
Picture on the page (x_del1): Publisher also deletes the BSE blip container and empties EscherDelayStm (the default shape properties record is written in the drawing group); we only tested leaving the BSE in place with `cRef` lowered by 1 (0): accepted ✔.
Quill: Publisher removes the deleted stories (TEXT shrinks 948 -> ... , STRS, SYID, MCLD records, FDPC/FDPP runs; the story high-water mark H stays 6). **Leaving the Quill stream completely untouched is accepted by 2021** ✔ (p_del2_B, p_del1_B: orphan stories 5, 6 / 2, 4 not referenced by any shape).
Patch evidence (what is required): ✔ Contents objects removed + registries cleaned + Escher containers removed -> opens, one page, right shapes (p_del2_B, p_del1_B, also with Publisher's own Quill, p_del2_A, p_del1_A). ✘ **Registries left with dangling records -> 2021 refuses the file** ("Publisher has detected a problem", p_del2_C). ✘ **Escher containers left for deleted Contents objects -> "cannot open"** (p_del2_D). The font-table slot move is not needed.

### 4P.5 Duplicate a page (`Page.Duplicate`) ✔ (patched with Publisher's Quill; Quill logic ~)
The copy is inserted after the source in 256 item 2 and **is a new blank-style page object plus clones of every child**:
- New ids: Publisher used `page = F`, a gap (F+1 unused), `font table = F+2`, the cloned shapes in z-order from F+3 (a picture's kind-102 child directly after the picture), then kind 96 and kind 119 (d_dup2: 305, [306 empty], 307 font, 308/309 boxes, 310, 311). Our patch used `page F, 96 F+1, 119 F+2, clones F+3.., font table last`: 2021 accepts it ✔ (the allocation order is not significant).
- Page body = the blank page body of 4P.2 (item 6 = `(old item 35 << 32) | 2`) plus item 1 = child count and item 2 = new child ids in the source's z-order ✔. Item 8 (page 1's kind-77 child) is not copied.
- A cloned shape body = the source body with: item 39 (story) = the new story number, and **overlap items 45 / 70 / 71 left as in the source (Publisher drops them: clones show none; we dropped them too ✔)**; the other items identical (items 43/44 present as in the baseline). A picture's body (kind 1 with item 52 = 0, item 58 = ref to its kind-102 child) is cloned with item 58 pointing at the cloned child; the kind-102 child (file name) is copied verbatim ✔ (d_dup1/p_dup1). Index entries are copies of the source entries with item 5 = the new page (`{2: 1, 5: page, 6: 259, 11: 22}` text box; `{2: 1, 5: page, 6: 258, 11: 1}` picture; `{2: 102, 5: picture, 6: 258, 11: 1}`).
- Kind 96 / 119 children: the clone of page 1 gets the *source's* empty kind-96 form (no item 5, no index item 11); the clone of page 2 gets `{5: 1}`. Both fine.
- **Stories**: every cloned text box gets a NEW story (a copy of the source text, character runs and paragraph runs). Story numbers Publisher chose: the shapes are processed last-to-first: the LAST text box in z-order gets H+1 (7), the one before gets H+3 (9) (8 skipped, like the second box in §4L.4); with H = 6 for both pages. SYID becomes `[H' = 9, n, ...old..., 7, 9]`; TEXT gets the new story texts appended in ascending story-number order (so the 7 text first); STRS gets their lengths appended in the same order (+4 trailer words); MCLD gets one 146-byte record per story, byte-identical to the source story's record except one byte at offset 113 (10 vs 2) ~; FDPC / FDPP gain copies of the source runs shifted to the new text offsets; BTEP / BTEC move; the Quill length grows by 1024 (two 512-byte pages) ✔ (d_dup2, d_dup1). Story numbers must be consistent between Escher (lTxid and the 0xF00D record), the cloned body item 39, 283, 284, and Quill (SYID, TEXT order).
- **Registries**: 283 gets `{1: new story, 3: new object id}` and 284 gets `{1: new story, 7: ordinal, 8, 9: copy of the usual constant 0xCB18967C}` for each cloned text box, in ascending story order; 284 ordinal = next numbers (5, 6) ✔; item 1 of both = new count.
- **Escher** ✔: for every child one container is cloned, appended at the end of the dgid-1 page group (in source order), with spid = lastspid+1.., the client-data object id = the new object id, lTxid (property 0x80) and the 0xF00D record = the new story. Everything else in the container is identical to the source (the clone's anchor is the same page-relative rectangle; in Publisher's own output the default 0xF122 properties of a fresh box also changed to the 2021 values, not needed). 0xF008 cspids +n, lastspid; 0xF006 cspSaved +n and the FIDCL count of dgid 1 +n (here = lastspid - 1024 + 1); 0xF003/0xF002 lengths +bytes. **Picture clone: the same container (a longer one; same properties) and the blip store entry's `cRef` +1** (offset 24 of the BSE data; Publisher does it; leaving it was accepted by 2021 ✔); EscherDelayStm unchanged (the clone shares the blip) ✔.
- Patch evidence: ✔ with Publisher's Quill, our Contents + Escher (and ids) -> 3 pages, right shapes, text and positions, re-saved by 2021 without change of structure (p_dup2, p_dup1, rt_dup2). ✘ Without the new stories in Quill -> refused ("detected a problem", p_dup2_noquill). ✘ Without 283/284 records -> "cannot open" (p_dup2_noreg). ~ Without the Escher containers the file opens but the new page has **no shapes** (p_dup2_noesc: Escher is what puts a shape on the page; a Contents object without a container is ignored). ✔ `cRef` not bumped: opens.

### 4P.6 Writer checklist
1. **Reader fix first** (`pub-format::assemble`): page order = order in 256 item 2, filtered to kind-67 with non-zero item 6.
2. Add blank page: new objects page/96/119 at the font table's slot (font table after them, fix 282[4]); page body from the template (item 6 = `(item35 << 32) | 2`); 256: insert id in item 2 at the position, item 1 +1, item 35 +1, item 60 +1 (once per session); rebuild index (3 new entries, counts, offsets), header lengths. No Escher/Quill work.
3. Move page: reorder 256 item 2 only.
4. Delete page: collect the page and all descendants by index parent; remove them; clean 283 and 284 of the records for their text boxes (do not renumber); remove their Escher containers (cspids/cspSaved -n, lastspid and FIDCL unchanged, lengths); 256: item 2, item 1 (not item 35); drop trailing empty index slots; for a picture lower the BSE `cRef`. Quill stories may stay (an orphan story is harmless; cleaning them is an optional later step).
5. Duplicate page: add page as in 2 plus children (clone bodies with new story and new refs, overlap items removed), 283/284 records, Escher containers (new spid/object/story), BSE `cRef` +1, and for each cloned text box **new story in Quill** (the §4L machinery: copy text + runs + MCLD record; SYID / STRS / TEXT / FDPC / FDPP / BTEP / BTEC).
6. Always: bump the document save counter once per session (§4M), rebuild offsets/lengths, keep empty slots as `0x78`.

Implementation notes (pub-write `add_page` / `move_page` / `delete_page`, changed 2026-11-08): delete leaves the Quill stream untouched (orphan stories) and keeps the BSE with `cRef` lowered; because the reader used to map story numbers to story indexes by RANK among the numbers the shapes use, which breaks with orphans, `pub-format` now maps through the Quill `SYID` list (story number of each story, in TEXT order) when it has one number per story and covers every used number (rank otherwise; `Document::story_numbers`). Deleting closes empty slots directly below the font table. A blank page has no item 1 / 2, so adding the first shape to it inserts both (`Cts::set_child_ids`). Verified in real Publisher 2021 (read-only open, 16.0): add last/first/middle/twice, 1-page file, move, delete picture page / last page / middle page, delete then add, text box and rectangle on an added page.

### 4P.7 Not tested
Master pages (adding/deleting/applying one), two-page spreads and `IsTwoPageMaster`, page names and `PageNumber` formats, deleting the only page, `Pages.Add` with a master or layout argument, duplicating a page with tables, groups, linked text boxes (story chains) or overlapping shapes (overlap items 45/70/71 and line-crossing item 43 were dropped, not remapped), pages in a file that already has deleted slots below the font table, how Publisher 2010 reads our patched files, adding after a delete in the same file (item 35 reuse; assumed from 4P.1), whether leaving orphan stories in Quill causes problems on later edits, exact Quill delete/duplicate algorithm for formatted multi-run stories (only observed), picture pages when the BSE is dropped by Publisher on delete (we left it), other page sizes, files with more than one picture sharing a blip, and the Publisher "page move" for facing pages. The meaning of the low word of item 6 (1 vs 2) and of Contents objects 269-279 (hidden kind-67 helpers) remains unknown ?.

## 4T. Master pages, page numbers and headers/footers (real Publisher 2021 COM, then Save(); our samples only; evidence `out/research/masters/*.pub` + `manifest.json`, recipes `master-*` in `tools/make_samples.ps1`; 2026-11-25; pw-s81 and children)

Markers: ✔ = verified across samples, ~ = likely, ? = unknown. Samples are one change against `samples/blank.pub` (1 page) or its 2/3-page version (`Pages.Add`). **The session ended early (a COM call hung and the instance could not be closed from the research session), so everything about a second master page, applying a master, two-page masters and backgrounds is NOT tested (4T.7).**

### 4T.1 Where master shapes live ✔
- **A shape on a master page is an ordinary Contents object (kind 1 for a text box) whose parent is the master page object 263 instead of a document page** ✔ (`master-box-1p`, `-2p`). Master page 263 (kind 67) gets **item 1 = child count and item 2 = list of the child ids (z-order)**, exactly as a document page with shapes (4P.1); a master without shapes has neither item ✔.
- The new shape object body is identical to the same box on a document page: `{2: true, 4: u16 session value, 12: 0, 13: 0, 39: story, 170: width EMU, 171: height EMU, 183: 0}` (216 x 36 pt = 2743200 x 457200). Item 4 differs between sessions (23811 vs 25859) and is not significant ✔ (`master-box-1p` vs `master-boxpage-1p`: the ONLY differences are the parent id, the 263 child list versus the page child list, and item 4). Object ids and the font-table move follow 4L (new box at the font-table slot, font table behind it, 282 item 4 follows) ✔.
- **Escher and Quill are byte-identical (apart from save noise) to a document-page box** ✔: the container goes into the same dgid-1 drawing's page group, spid = lastspid + 1, client data = the Contents object id, lTxid = story. **Escher has no master/page distinction for shapes**; the page a shape belongs to (master or document) is expressed only by its Contents parent chain ✔. Registries 283/284 get the usual records ✔.
- A master shape is shown on every document page that uses the master. Nothing per page is stored: 1-page and 2-page files with the same master box have no page-specific data ✔ (`master-box-2p` vs `master-box-1p`: only the Contents/Escher/Quill additions of the box itself).

### 4T.2 Which master a page uses; Ignore Master ✔ / ?
- Every document page has **item 13 = ref to its master page** (4P.1). **A page's item 13 holds the object id of the master it uses** (263 for A, 302 for a second master, and so on) ✔ (4T.8). A new page gets master A; a duplicated page keeps the master of the original ~.
- Master page body (object 263): item 14 = short label (`'A'`), **item 15 = the master's name** (`'Master Page A'`), item 16 = 3, item 9 / 11 = kind 96 / 119 children ✔. **Renaming** (`MasterPages(1).Name = 'Front'`) changes **only item 15** (item 14 stays `'A'`) ✔ (`master-rename-a`).
- **`Page.IgnoreMaster = true` sets item 7 = true (bool, 0x08) on that page's object; absent = false** ✔ (`master-box-2p-ignore`, page 2 of 2; page 1 unchanged). Nothing in Escher/Quill/the master changes ✔.

### 4T.3 Page-number field in text (Quill) ✔ / ~
- COM: `TextRange.InsertPageNumber(1)` (`pbPageNumberCurrent`; 0 is rejected); the text then reads `#` where the field is. Only the *current page* type was tested.
- **Contents and Escher contain nothing about the field**: a master box with the field differs from a master box with plain text only in the Quill stream (and item 4) ✔ (`master-pn-1p` vs `master-box-1p`). The same bytes are produced for a field in a normal page text box ✔ (`master-pn-boxpage-1p`: TEXT and TOKN identical to `master-pn-1p`).
- Quill ✔: the story text contains a **literal `#` (U+0023)** at the field position (`master-pn-1p`: TEXT = `# \r`, i.e. `23 00 0d 00`; `master-pn-text-2p`: `Page #\r`). The story also gets a **new directory entry `TOKN`** (a `PLC ` structure, 16th directory entry: the entry count at offsets 0x0c / 0x1a goes 15 -> 16, the entry sits **between FONT and STRS**, the data (0x34 bytes) is placed where STRS was and STRS, MCLD, PL and all later data move by 0x34; stream length unchanged, it fits in the padding) ✔.
- **TOKN is one chunk per story that has fields; its instance is the story's index in `STRS`** ✔ (`master-pn-2boxes`: instances 0 and 1; a field box that comes second in `STRS` after a plain story: only instance 1, with no instance 0). Stories without fields have none. **cps count from the start of the story**, not of the whole TEXT chunk ✔ (the second story `#` + CR has cps 0, 2 like the first). Instances follow their story when stories are added, copied or removed: removing story k drops instance k and moves higher ones down by one (as `TCD `); a copy of a story gets a copy of its table ~.
- **TOKN data for n fields** ✔ (n = 1, 2, 3, adjacent fields, a field at the start/middle/end; `master-pn-text-2fields`, `master-pn-3fields`, `pn-a-adj`, `pn-a-lead`): `n | 0000000c | 0001ffff | cp_1 .. cp_n | end | A | B`, all u32 little endian. `cp_i` = index of the i-th `#` in the story (ascending), `end` = the story length including the final paragraph mark. Then two tables, **A** and **B**, of n property blocks each (each block starts with its own byte length, 4 = empty). A = `[16000000 0022 00000000 0122 01000000 0222 fbffffff]` (22 bytes, the first field's: type 0, 1, -5) followed by n-1 empty blocks `04000000`; B = `[0a000000 0022 ffffffff]` (10 bytes) followed by n-1 empty blocks. Total size 40 + 12n bytes (52, 64, 76, ...). Only the first field has non-empty blocks; the type argument of `InsertPageNumber` (1, 2, 3) changed nothing in the file (the object model treated all three as the current page), so other field types are **not seen ?**. A two-field story: `Page # of #` -> `02 | 0c | 0001ffff | 5 | 10 | 12 | A(22) 04000000 | B(10) 04000000`.
- **After every TOKN chunk Publisher leaves 20 bytes that the directory does not count** (the next chunk starts 20 bytes after its end, also between two TOKN chunks) ✔: eight zeros, `0e ca xx xx` (a stamp that changes between Publisher sessions, not understood), four zeros and the FONT chunk's bytes 16 to 20 (its time stamp). **A file whose TOKN chunk has no such gap is refused by Publisher 2021 ("cannot open the file")**, with everything else identical to Publisher's own file ✔; with the gap (any stamp value we wrote: `0e ca a6 b1`) it opens and reports one field in the box ✔. An existing file's gap is kept when a table is rewritten; removing the last TOKN removes the gap.
- Opening such a file in Publisher through COM: `TextRange.Fields.Count` = the number of fields (1 for `#`, 2 for `Page # of #`) and `TextRange.Text` shows `#` for each ✔.
- A reader therefore finds a field by: the story's Quill directory has `TOKN` and the text contains `#` at one of its cps ~. A `#` typed as plain text in a story without `TOKN` is not a field ? (not tested).
- Character runs of a field story ~: the field character has **its own FDPC run**, whose character record carries two items no other text has: `0x200` (4 bytes, value 5; header bytes `00 12`) and `0x222` (6 bytes, value `ffffffff`; header `22 22`). Text next to the field is a separate run whose record is the field's record **without** those two items, and the paragraph mark keeps its own record (language item only). `master-pn-boxpage-1p`: `#
` = 2 runs (field record, `
` record); Publisher's own `master-pn-text-2p` (`Page #
`) = 3 runs: `Page ` -> field record minus the two items, `#` -> field record, `
` -> the plain `
` record. A writer must not merge the field run with neighbours of equal visible formatting, or the marks spread over the whole story and Publisher shows the box empty.
- Other fields in the same story: the Quill TEXT/FDPC/FDPP bookkeeping of the story is as for plain text (STSH/FDPC/FDPP/SYID unchanged apart from the shifted offsets and the text length) ✔.

### 4T.4 Page-number sections: format, start, first page ✔ (values) / ? (meaning of the format numbers)
- A file with default numbering has **no section object**. The first time a section property changes (or `Sections.Add`), Publisher adds **object 256 item 47 = ref to a new kind-117 object (parent 256)** at the next free slot (font table moves behind it) ✔. Body: **item 1 = number of sections, item 2 = list of section records** ✔.
- Section record ✔: `{1: start page index (1-based; the first section is 1), 2: first page number (present only when "continue from previous" is off), 3: number format (absent = 0 = Arabic), 4 and 5 = true (both, bool 0x00 type) when ShowHeaderFooterOnFirstPage = false}`.
  - `PageNumberStart = 5` with ContinueNumbers off -> `{1: 1, 2: 5}`; `Page.PageNumber` then reads 5, 6, 7 ✔ (`master-pn-start5`).
  - `PageNumberFormat = n` -> `{1: 1, 3: n}` for n = 1..6 (COM accepted all six and read them back; the stored item 3 is the same number) ✔ (`master-pnfmt-1` .. `-6`; these files differ only in that value). **Meaning of the stored numbers = the public `PbPageNumberFormat` enum (Publisher type library names, read without running Publisher) ~: 0 Arabic, 1 upper-case Roman, 2 lower-case Roman, 3 upper-case letter, 4 lower-case letter, 5 ordinal (1st), 6 cardinal text (One), 7 ordinal text, 10/11 DbNum1/2, 12 Aiueo, 13 Iroha, further East Asian/other values up to about 44.** The stored item 3 equals the COM value in all six samples ✔; the displayed text was not checked.
  - `Sections.Add(2)` on 3 pages -> two records `{1: 1}` and `{1: 2}`, item 1 = 2; with "continue" on, `PageNumber` stays 1, 2, 3 ✔ (`master-pn-section2`). Setting section 2 to start 1, format 2 -> second record `{1: 2, 2: 1, 3: 2}`; page numbers read 1, 1, 2 ✔ (`master-pn-section2-fmt`).
  - `ShowHeaderFooterOnFirstPage = false` -> record `{1: 1, 4: true, 5: true}` ✔ (`master-pn-first-hf`). Which of the two flags means which part (header or footer / first page of the section) is not known ?.
- The number a page shows = (start of its section, else the previous section's last + 1) + its index inside the section, formatted with the section's format ~ (consistent with `Page.PageNumber` in all samples; the file itself stores only the section records, not per-page numbers) ✔.

### 4T.5 Headers and footers
- `Page.Header` / `Page.Footer` are **Nothing (null) on every sample** (`master-header`): the object model exposes no way to create them, and Publisher 2021's header/footer in the ribbon is a master text box (our `master-box` / `master-pn` samples are the stored form) ~. Nothing was stored differently for a "header" ✔ (nothing was created). Not tested: the Insert > Header and Footer command (not driven; we do not send UI input).

### 4T.6 Reader and writer needs
**Reader** (to draw a document page): (1) the page's master = object in page item 13 (4T.2); (2) skip the master shapes if page item 7 is true; (3) the master's shapes = the children of that master object (items 1/2 of the master page, kind-1 etc., same decoding as page shapes), drawn behind the page's own shapes ~ (z-order across master and page not tested ?), anchors from Escher are page-relative as for page shapes; (4) a story with `TOKN` + `#` shows the page number of the page being drawn, formatted by the section containing it (4T.4); the displayed text width is that of the formatted number, not of `#` ?; (5) numbering: section records in 256 item 47, otherwise numbering is Arabic from 1.
**Writer** (minimum): *add a master shape* = add a text box as in 4L but with parent 263 and the id added to 263 items 1/2 (create them if absent); Escher/Quill/registries exactly as for a page box ✔. *Ignore master* = page item 7 = true. *Insert a page-number field* (any number per story, any story) = a story whose text has `#` at the field positions, the field characters in their own FDPC runs (the first field of a file takes the character's record plus the two field items), and a `TOKN` chunk of 4T.3 with instance = story index, cps relative to the story, its 20-byte gap, the directory count +1 and later data shifted ✔ (written files open in Publisher 2021, field count and page numbers read back; `crates/pub-write/tests/fields.rs`). *Sections* = kind-117 object + 256 item 47 (4T.4) ~.
**Writer, masters (research 2026-11-28; Publisher's own outputs in `samples/masters/`):** *add master* = 4T.8 (four objects, list and count edits). *Apply* = page item 13. *Rename* = item 15 (and 14 for the letter). *Delete* = the inverse of add plus re-pointing item 13 (4T.9). *Shapes on a master* = as on a page with the master as parent (4T.11). *Two-page* = 4T.10. Opening a file with a second master that KernPage wrote in Publisher 2021 is checked in 4T.13.

Implementation notes (reader, 2026-11-25): `pub-format` lists as masters the kind-67 objects with item 6 = 0 and a non-empty item 15 (`Document::masters`, shapes decoded like page shapes through the Contents parent chain); `Page::master` = position of the object in page item 13; `Page::ignore_master` = item 7 present. Fields: `TOKN` = `n` (u32), 12, `ffff0100`, then cps; each of the first `n` cps (a position in the whole TEXT chunk ~) that holds a literal `#` becomes a one-character run with `field = Some(Field::PageNumber)`. Sections come from 256 item 47; the shown number = section first number (else previous section's count carried on) + page index in the section, formatted by `NumberFormat` (0-4; 5 and above fall back to Arabic; letters past Z are written AA, BB, ... ~). Drawing (editor canvas, thumbnails, print preview, PDF) draws the master's shapes first, then the page's, replacing field runs by the page's number; master shapes are not selectable. `hide_first_page_hf` is parsed but not yet applied. A file saved after an unrelated edit keeps the master shapes and `TOKN` bytes unchanged (`kernpage-core/tests/save.rs`); editing the text of a story that holds a field: see the writer notes below.

**Writer, field and section upkeep (pw-96om, 2026-11-26, ~ until Publisher opens the outputs `out/pw-96om/*.pub`):** (1) every Quill rebuild that changes TEXT rewrites `TOKN`: the cps are positions in the whole TEXT chunk (as the reader assumes; only single-story files were seen ?), cps before the edited story stay, later ones move with the length change, the edited story's own become the `#` offsets the editor model holds (the editor and the file agree), and the last cp (the text length) follows. A story left with no field loses the TOKN entry (directory count 16 -> 15, later data moves up) and looks like a story that never had one. The number of fields in the file may only change to zero (layout with several fields unknown); an edit that would lose one of several, and copies of text boxes that hold a field (duplicate page, paste), are refused (`plan.unsupported`). One edit that replaces text on both sides of the field at once counts as replacing the field (plain `#` text afterwards). (2) Sections: the editor keeps `Document::sections` in step with page commands: an inserted or duplicated page joins the section of the page before it (later sections start one page later), a deleted page moves later sections one page earlier, a section whose only page was deleted is dropped, the first section always starts at page 1, moving a page changes nothing. A save rewrites item 1 (start page) of the kept records and item 1 of the kind-117 object (record count), keeping every other record byte; records are matched to the editor's sections in order.

### 4T.7 Not tested / unknown
What stays unknown after 4T.8 to 4T.12: backgrounds (`Background.Create()` hung the session); the rendering order of master versus page shapes (assumed: master behind, not seen in a render); the id letter after a delete-then-add; where a master description is stored (no COM property, nothing in the file that we know of); facing-page documents; the purpose of the helper kind-67 objects 269/272/275/279 (only that deleting a master re-points their item 13 like a page's). Still open from the first session: the page-number format meaning (4T.4); field types other than the current page; fields in the middle of text (the writer handling is ~ in 4T.6); several fields per story (a second `TOKN` layout is unknown; the writer refuses to lose one); fonts/colours on the field; fields in tables or groups; page numbers on page 1 of a spread; whether Publisher 2021 accepts a file patched by hand with the `TOKN`/section additions; Publisher 2010 reading these files.

### 4T.8 Adding a second master ✔
- A master is a kind-67 object with parent 256, item 6 = 0, item 14 = its one-letter id (`'B'`), item 15 = its name (`'Master Page B'`), item 16 = 3, plus the same items as master A: `{3: ref guide object, 5: {1: 22860000, 2: 22860000}, 9: ref kind-96, 11: ref kind-119, 17: {1: 110185200, 2: 110185200}}`. It takes the next free object id and three helpers after it: kind 96 `{5: 1}`, kind 119 (a copy of A's), kind 76 (the margin/guide object, a copy of A's whose items 8 to 11 read 196469, the value of a master Publisher adds). With 3 pages: 302 / 303 / 304 / 305.
- Object 256: **item 2 (child list) = all masters first (leading half first for a two-page master), then the document pages, then the four helper kind-67 objects**. The new master goes after the last master. Item 1 (count) +1; **item 45 = number of master entries** (+1; a two-page master counts 2). Item 60 is a save counter and goes up by 1 on every saved change; it carries no meaning.
- Nothing changes in Escher, Quill or the pages ✔. A master with no shapes has no items 1/2 ✔. Letters are given out automatically (A, B, C ...); COM has no property to set the letter or a description ✔ (the file stores the letter as a plain string, so a chosen letter can be written).
- `Duplicate()` makes the same four objects with the name `Copy of A` and the next letter ~.

### 4T.9 Applying, renaming, deleting ✔
- **Apply**: the page's item 13 = the master's object id. Nothing else is stored (4 files) ✔. `IgnoreMaster` still sets item 7 = true on the page ✔.
- **Rename**: only the master's item 15 changes; item 14 (the letter) is independent of the name ✔.
- **Delete**: the master, its three helpers and its shapes vanish from the file; the result is exactly the file before the master was added (apart from item 60) ✔. Pages (and the four helper kind-67 objects) that pointed at it get item 13 = master A when A still exists, or the first remaining master when A itself was deleted ✔; pages on other masters keep theirs ✔. Item 1 -1, item 45 -1. The last master cannot be deleted (COM error "You cannot delete the default master page.") ✔. Deleting one half of a two-page master deletes both ✔.

### 4T.10 Two-page masters ✔ / ~
- Two kind-67 objects, each with its own helpers and guide object, the same name and letter; **item 16 = 0 (leading/left), 1 (trailing/right), 3 (single page)**; the x origin items 5.1 and 17.1 are the single-page values -/+ half a page (3886200 EMU): leading 18973800 / 106299000, trailing 26746200 / 114071400 ✔. The leading half comes first in 256 item 2 ✔.
- Converting an existing master to two-page (`IsTwoPageMaster = true`, or turning on mirrored guides) creates the leading half as a new object placed before the original, which becomes the trailing half, and sets **256 item 6 = true** ✔ (`Add(1)` does not set item 6). Converting back deletes the leading half and the trailing object becomes single (item 16 = 3, origin reset) ✔.
- A page's item 13 names one half (leading or trailing) ✔; which half pages use automatically in a facing document is unknown ?.
- A shape belongs to the half that its position falls in, whichever master entry was used to add it ✔ (observed boundary: right edge past about 306 pt = trailing). The anchor in Escher is measured from the centre of that half: for a leading shape L = COM left, for a trailing shape L = COM left - 612 pt (checked on a single-page-per-view document only) ~.

### 4T.11 Other shapes on a master ✔
Rectangles (kind 1), lines (32), pictures (kind 1 + the picture store), tables (16) and groups (48) are stored **exactly as on a document page except for the parent**: the Contents object's parent is the master object, the master's items 1/2 hold the child ids (for a group only the group id; its children are parented to the group), Escher and EscherDelayStm are byte-identical to the page version, Quill differs only in save noise ✔ (five M/P sample pairs). Works the same on master B (302) ✔. Text boxes: the same, also on B ✔ (a text box written into master B by KernPage equals Publisher's object for object, font table included).

### 4T.12 Z-order, margins, guides
- `ZOrderPosition` from COM is one creation-order counter shared by master and page shapes, so it does not say which is drawn on top. The master's own children are a separate list from the page's. That master shapes are drawn behind page shapes was **not** verified from a render ? (assumed; a PDF export hung the session).
- **Margins and column/row guides are per master**, in the kind-76 object referenced by master item 3: item 2 = records `{1: position EMU from the page's top-left, 2/3/4 flags}`, vertical guides first (left, column guides, right) then horizontal (top, row guides, bottom); right and bottom are stored as page size minus margin ✔. Near margins have flag 3 and 4, far margins 2 and 4, column/row guides 2 and 3. Item 1 = count, 3 = page size, 6 = vertical count, 7 = horizontal count ✔.
- **Ruler guides** go in another kind-76 object referenced by **item 4** of the master or of a document page, records `{1: position, 4: true}`, header counts in 6/7 ✔ (orientation by type not read back ?).
- A page has no margins of its own ✔.

### 4T.13 Writer implementation (KernPage, 2026-11-28)
`pub-write` (`masters.rs`) writes: **add** a one-page master (copy of the first master's body and guides, letter and name given, new objects at the next four free ids in front of the font table, 256 item 2/1/45); **rename** (items 14/15); **delete** (the master and every descendant through the page-delete machinery, Escher containers stripped, pages and helper pages re-pointed to the first remaining master, 256 item 1/2/45); **apply / ignore** (page item 13, page item 7 added or removed); **margins of one master** (the guide records of that master). The shape writers address a master with a surface index (`pub_format::master_surface`) in place of a page index and write the shape with the master as parent, creating items 1/2 on a master that has none. The page-setup writers (margins, page size) change the guides of every master. KernPage's save planner matches masters by an identity of its own, writes them in the order: per-shape edits, page operations, master deletions, renames, additions, each page's master, then the new objects. Written files in the form above were opened in Publisher 2021 (log: `research/masters-2026-11-28/kernpage-written/`).
Not written (reported when saving): two-page masters (4T.10: the half a shape belongs to, the anchor origin and the 256 item 6 flag were seen on a single-page-per-view document only, and a file's two halves are read as two masters), a margin that differs between masters, ruler guides, plain shapes and text boxes deleted from a master (as on a page: only pictures and tables are removed), a page-number field in a new or duplicated text box (4T.6). A master's description has no place in the file and is kept only in a `.kern` file; it is neither written nor reported.

## 4Q. Page Setup: page size, orientation, margins (real Publisher 2021 COM and ribbon, then Save(); our samples only; evidence `samples/gen/page-*.pub` + `manifest.json`; UI in `docs/ui/PAGE-SETUP.md`; 2026-11-14)
Publisher 2021 (16.0) opened copies of a Publisher-saved blank (a 1-page base with a rectangle, a text box and a second rectangle; a 2-page base; a base with objects around the page edges), one change was made, then `Save()` (never `SaveAs`). Changes were made three ways and compared byte for byte: COM (`PageSetup.PageWidth/PageHeight`, `PageSetup.PageSize = <entry of AvailablePageSizes>`, `LayoutGuides.Margin*`), the ribbon through keyboard automation (Page Design > Margins / Orientation / Size galleries) and the dialogs (Page Setup, Custom Margins). Marks: ✔ seen in two or more files, ~ seen once, ? unknown. Ground-truth positions are in `manifest.json` (entries `page-*`).
**COM facts:** `PageSetup.Orientation` is **read-only in 2021** (the setter fails with "This functionality is not supported in the current version of Publisher"; value 1 = portrait, 2 = landscape, derived from the page size) ✔. `PageSetup.PageWidth/PageHeight` are settable (points) ✔. `PageSetup.PageSize` (read/write, an object with `.Name`, `PageWidth`, `PageHeight`) can be set to any entry of `PageSetup.AvailablePageSizes` (8,712 entries: the "More Preset Page Sizes" catalogue, with duplicate names) and gives **exactly** the file the ribbon gallery gives ✔. `Document.LayoutGuides.MarginLeft/Right/Top/Bottom` equal `Document.MasterPages(1).LayoutGuides` (there is one master, "Master Page A"); `Page.LayoutGuides` has no margin properties (margins are per master, not per page) ✔. `PageSetup.TopMargin/LeftMargin/HorizontalGap/VerticalGap` belong to label layouts (0 here). A new Letter blank (Backstage "Blank 8.5 x 11 in") reports margins 36 pt on all sides (0.5 in = the "Moderate" gallery item) and `PageSize.Name` "Letter (Portrait)"; "Blank 11 x 8.5 in" reports 792 x 612, orientation 2, "Letter (Landscape)", margins 36 ✔.

### 4Q.0 What changes, per setting (streams)
| Change | `Contents` | `Escher/EscherStm` | Quill | `\x05SummaryInformation` | other |
|---|---|---|---|---|---|
| Page size (any way) | yes: §4Q.2 (objects 256, 289, 292, the kind-67 objects, scratch page 269, shapes' item 3) | **byte-identical** unless a shape no longer touches the page (§4Q.4: its anchor is rewritten and a 4-byte item 5 appears) ✔ | save noise only ✔ | thumbnail replaced (size depends on the page) | none |
| Orientation | as page size (W and H swapped) | same | same | same | none |
| Margins | **only object 289** (kind 76, the layout guides under the master page) ✔ | identical ✔ | noise only ✔ | identical ✔ | none |
`Contents` sizes (1-page base, 10,378 bytes): A4 10,380, Legal 10,386, 11x17 10,386, landscape 10,388, margins 10,380-10,396 (a margin of 0 drops the value item, §4Q.6). Escher stays 2,248 bytes in every case that leaves all shapes on the page.

### 4Q.1 Which values are where (summary)
| Value | Where | Unit |
|---|---|---|
| Page width and height | object 256 (kind 68) item 18 {1: W, 2: H}; object 289 (kind 76) item 3 {1: W, 2: H}; object 292 (kind 138) item 4 > item 18 > element {3: W, 4: H} | EMU (pt x 12,700), u32 ✔ |
| Orientation | **not stored**: landscape = W > H (a preset record also names it, §4Q.3) | ✔ |
| Margins | object 289 item 2 = four guide records (§4Q.6); nothing in 256/292/page objects | EMU ✔ |
| Preset name | object 292, §4Q.3 | ✔ |
| Two-page (facing) master | §4Q.6 (a second master page object) | ~ |

### 4Q.2 Page size fields (Letter -> X) ✔ (49 files, 0 exceptions)
Let W, H be the new size in EMU (`pt x 12,700`, u32, taken as given: 595.3 pt -> 7,560,310; the A4 preset is exactly 21 x 29.7 cm = 7,560,000 x 10,692,000 = 595.2756 x 841.8898 pt). Everything else in `Contents` is unchanged except the usual save noise (save counter item 60 of object 256, the upper byte of shapes' item 4, offsets):
- **Object 256 item 18** = {1: W, 2: H}. **Object 289 item 3** = {1: W, 2: H}; its guide records for the right and bottom margin = W - right margin and H - bottom margin (§4Q.6).
- **Object 292 (kind 138)**: preset or "Current" form, §4Q.3.
- **Every kind-67 object** (master page 263, all document pages, the helper pages 272, 275, 279 and the scratch page 269): item 5 = {1: w5, 2: h5} and item 17 = {1: w5 + 87,325,200, 2: h5 + 87,325,200}. Define `ceil600(x)` = x rounded **up** to a multiple of 7,620 EMU (0.6 pt = 1/120 inch), a = ceil600(W/2) and b = ceil600(H/2) (W/2 may be a half EMU). **Scratch page 269:** w5 = 22,860,000 - a, h5 = 22,860,000 - b (Letter 18,973,800 x 17,830,800; A4 19,072,860 x 17,510,760; 5x7 20,574,000 x 19,659,600). **All other kind-67 objects:** w5 = (22,860,000 - a) + floor(W/2), h5 = (22,860,000 - b) + floor(H/2). When W/2 and H/2 are multiples of 0.6 pt (Letter, Legal, 5x7, 11x17, Tabloid) these are exactly 22,860,000 (1,800 pt) and 110,185,200 (8,676 pt); for other sizes they are a little smaller (595.3 x 841.9 pt: 22,853,015 x 22,856,825; the exact A4: 22,852,860 x 22,856,760). This **settles the "rounding rule not understood" remark of §6.2.6** (verified on 49 files including 612.25 x 792.5, 300.5 x 400.7, 595.44 x 841.68). Whether Publisher cares if a writer uses 22,860,000 for every size is untested (it rejected no value in these files).
- Orientation change = W and H swapped everywhere above (and the w5/h5 pairs swapped); **no other field**. The guides list keeps its order (left, right, top, bottom).
- **Thumbnail** (`\x05SummaryInformation` property 17, §6.5): 24-bit bitmap of the page; pixel sizes seen: Letter 114 x 147 (stream 50,852 bytes), Letter landscape 147 x 114 (50,900), Legal 89 x 144 (38,876), A4 103 x 145 (45,524), A4 landscape 145 x 103 (45,192), 5x7 104 x 144 (45,212), 11x17 103 x 157 (49,268), A5 103 x 144, 300.5 x 400.7 104 x 137 (43,028). The rule is not understood (e); the thumbnail is optional (§6.5).

### 4Q.3 Object 292 (kind 138, "page-setup record") ✔: two forms
Item 4 is a record. **Form C "Current"** (anything set through `PageWidth`/`PageHeight`, and the Page Setup dialog with typed numbers): {2: u32 161, 5: string "Current", 8: u32 2, 10: 0, 17: 1, 18: list[record {3: W, 4: H, 6, 7, 8, 9: 457,200}], 19: 1, 20: list[record {1: 7,772,400, 2: 10,058,400, 5: flag true, 8: record {2: 0, 3: 0}}]}. In form C, items **6, 7, 8, 9 of the item-18 element are the margins top, left, bottom, right** in EMU (457,200 = the 0.5 in default; `page-a4-landscape-margins-2pages` has 914,400 / 685,800 / 1,143,000 / 457,200); margin-only changes on a form-P file leave object 292 untouched ✔. Converting a form-P file to form C without restoring objects 293-295 is accepted by Publisher 2021 (open and resave ✔, `pub-write` check file from `page-a4-preset`). Item 20 keeps the **template paper (Letter) in portrait order** in every case seen (A4, Legal, landscape; this corrects §6.2.4/§6.2.6 where "the same paper" was assumed for A4). The Page Setup dialog variant uses item 2 = 37, item 4 = 32,768, adds item 9 = flag true and writes the page size again as item 20 element {8: W, 9: H} ✔ (`page-edge-5x7-dialog`). **Form P "preset"** (ribbon Size gallery, ribbon Orientation, or `PageSize = <entry>`): the same shape but item 5 and item 8 (when present) replaced by {2: catalogue id, 3: id, 4: u32 flags, 6: string preset name, 7: u32 catalogue index, optional 8: u32 1} and no flag 5 in the item-20 element. Observed:
| Preset | item 2 | item 3 | item 4 | item 6 (stored name) | item 7 | item 8 | size W x H EMU |
|---|---|---|---|---|---|---|---|
| Letter portrait (the blank's own) | 104 | 109 | 0x80010000 | "Letter" | 16,937,216 | - | 7,772,400 x 10,058,400 |
| Letter landscape | 106 | 110 | 0x80010000 | "Letter Landscape" (no brackets; the UI shows "Letter (Landscape)") | 16,937,217 | - | 10,058,400 x 7,772,400 |
| Legal portrait | 103 | 107 | 0x80010000 | "Legal" | 16,937,276 | - | 7,772,400 x 12,801,600 |
| A4 portrait | 1 | 2 | 0x80010000 | "A4" | 16,937,226 | 1 | 7,560,000 x 10,692,000 |
| A5 portrait | 3 | 4 | 0x00010000 | "A5" | 16,937,246 | 1 | 5,328,000 x 7,560,000 |
| Tabloid portrait | 163 | 181 | 0x00010000 | "Tabloid" | 16,937,286 | - | 10,058,400 x 15,544,800 |
(~: the ids are a catalogue the writer must copy; for sizes without a catalogue entry use form C.) **COM reads** `PageSetup.PageSize.Name`: "Letter (Portrait)" (blank), "Current" (form C), and "Letter (Landscape)", "A4 (Portrait)", "A5 (Portrait)", "Legal (Portrait)", "Tabloid (Portrait)" (form P) ✔. **A preset write also rewrites the document differently from form C** (✔ A4, Legal, Tabloid, landscape; identical through the ribbon and through `PageSize =`): objects 293 (kind 88) and 294 (its kind-77 child) and the kind-77 child 295 of page 1 disappear, object 256 items 14 and 20 and page item 8 (the refs to them) are dropped, the font table (kind 108) is renumbered from 299 to 295 and object 282 item 4 follows (so **kind 88/77 are not required** by Publisher 2021 after all; §6.10 found them required for a hand-written file, the reason is unknown ?). Form-C files keep 293/294/295. Both open in Publisher 2021 ✔. A writer may write form C for every size; whether a preset record together with the old objects 293-295 is accepted was not tested.

### 4Q.4 What happens to the objects on the pages ✔ (COM and UI, 1 and 2 pages)
1. **On-page shapes keep their anchor.** The Escher client anchor (0xF010: left, top, right, bottom, signed EMU) is measured from the **page centre** (§4A), so a size change leaves the Escher bytes identical and the page edges move around the shapes: a shape's position from the page's top-left corner changes by **(new W - old W)/2 horizontally and (new H - old H)/2 vertically** (Letter -> A4: left 72 -> 63.649; Letter -> Legal: top 72 -> 180; Letter -> landscape: left 72 -> 162, top 72 -> -18). Shapes are **not scaled or resized**; text boxes keep their size and text and **nothing reflows** (Quill unchanged, items 170/171 unchanged) ✔. A shape stays on the page even when it is partly outside it (Rect Far at (400,700), 150 x 60 pt on Letter is at (490,610) on the landscape page and still on the page).
2. **Shapes that no longer touch the page move to the scratch area.** The test is on the shape's rectangle: no overlap with the page rectangle [-W/2, W/2] x [-H/2, H/2] (the boundary case of a shape exactly touching the edge was not tested ?). Such a shape (a) changes its `Contents` parent from the page (e.g. 266) to **object 269** (kind 67; the same object that receives shapes created outside the page: `Shapes.AddShape` outside the page lands there too), (b) is removed from the page's item 2 list (item 1 = count lowered; a page without shapes has neither item) and appended to 269's item 2 list (item 1 = count; the order of the appended refs is not simply creation order), (c) keeps its Escher shape container (same drawing, same spid) but **its anchor is rewritten** in page-corner coordinates (§4Q.5), (d) keeps its size, text and story. A shape of page 1 and the scratch area of a 2-page file: same object 269 (✔ `page-a4-landscape-margins-2pages`; the scratch area is per document). COM `Page.Shapes` no longer lists such shapes; `Document.ScratchArea.Shapes` does (manifest `scratch_shapes`).
3. **Scratch shapes stay in the scratch area** when the page grows again (`page-edge-5x7-then-11x17`: 8 shapes still on the scratch area; nothing is rescued) ✔; later size changes do not touch them except that their anchors follow the edges (§4Q.5).
4. **Flag:** every kind-1 shape that was **on the page** when the size changed (also those that moved to the scratch area) gains item 3 = flag true (the third object flag bit of §4.3c); shapes that were already on the scratch area and margin changes do not set it. Meaning ? (a reader ignores it) ✔.
5. **Not tested:** shapes on master pages, grouped shapes, rotated shapes, tables, lines and pictures during a size change; autofit text frames; linked text boxes across the page/scratch boundary. Margin changes never move or resize anything ✔.

### 4Q.5 Scratch-area anchors (Escher 0xF010 of a shape whose Contents parent is 269) ✔ (`page-base-edge`, `page-edge-*`)
The anchor's items 1-4 (left, top, right, bottom, signed EMU) are **not** from the page centre but from the **page's top-left corner**, plus an **item 5 (u16, written only when non-zero)**: bit 0 (value 1) = the horizontal values are measured from the page's **right edge** (the shape is entirely right of the page), bit 1 (value 2) = the vertical values are measured from the **bottom edge** (entirely below). Rule (corrected 2026-11-14 from writing tests; fits every sample): with the page-relative left x_p (= centre-relative left + W/2) and the shape width w: if the **right edge x_p + w > W** then x := x_p - W and bit 0 is set, else x := x_p (negative for shapes left of the page); likewise y_p + h against H with bit 1. (An earlier "x_p >= W" wording did not fit Rect Far on 5x7 and 300x400: its left is inside W but its right edge is not, and both bits are set.) Examples (pt): on Letter a shape at (650,100) is stored as left 38, top 100, item 5 = 1; at (100,830): 100, 38, item 5 = 2; at (-150,300): -150, 300, no item 5; on a 360 x 504 page the shape that was at (400,700) on Letter is stored as left -86, top 52, item 5 = 3 (page-relative 274, 556). Because the edge is the origin, **a later resize moves bit-0 shapes with the right edge and bit-1 shapes with the bottom edge, the others stay at the top-left** (✔ 5x7 -> 11x17: a shape stored with left 14 and bit 0 is at 806 pt on the 792 pt wide page). Conversion when a shape leaves the page: x_p = anchor_left_from_centre + W/2 and y_p = anchor_top_from_centre + H/2 (new W, H), then the rule above. **UI versus COM:** the ribbon and the Page Setup dialog convert both axes (`page-edge-5x7-dialog`, `page-edge-a5-preset`); with `PageWidth/PageHeight` through COM the y of shapes that left over the left or right edge kept its old page-relative value (`page-edge-5x7-com`: top 300 instead of 156) ✔: a COM quirk, **implement the UI behaviour**.

### 4Q.6 Margins ✔
Margins are **layout guides on the master page**: object 289 (kind 76, parent = master page 263), item 2 = a list of exactly four records in the order **left, right, top, bottom**, each {1: u32 position of the guide from the page's top-left corner in EMU; flags}: left = left margin, right = **W - right margin**, top = top margin, bottom = **H - bottom margin**. Flags: left and top records carry items 3 and 4 (true), right and bottom records carry items 2 and 4 (true) ✔. **A guide at position 0 has no item 1** (margin 0: the left and top records are just their flags; the right and bottom records hold W and H) ✔ (`page-margins-none`). Item 1 of 289 = 4, item 3 = {W, H}, items 4-11 unchanged (§6.2.4). Examples (Letter, EMU; left / right / top / bottom): Moderate (default, 36 pt) 457,200 / 7,315,200 / 457,200 / 9,601,200; Narrow (18 pt) 228,600 / 7,543,800 / 228,600 / 9,829,800; Wide (72 pt) 914,400 / 6,858,000 / 914,400 / 9,144,000; None 0 / 7,772,400 / 0 / 10,058,400; custom L 54, R 36, T 72, B 90 pt: 685,800 / 7,315,200 / 914,400 / 8,915,400; dialog L 1.5, R 1, T 2, B 0.75 in: 1,371,600 / 6,858,000 / 1,828,800 / 9,372,600. **Nothing else in the file changes**: the ribbon presets, Custom Margins, `Document.LayoutGuides` and `MasterPages(1).LayoutGuides` give identical bytes ✔. Margins are independent of the page size (a size change keeps the margins and moves the right/bottom records). A 2-page file keeps one master: both pages use it ✔.
**Two-page master** (Custom Margins "Two-page master" tick box, `LayoutGuides.MirrorGuides = true`; "Left" and "Right" become "Inside" and "Outside"): the file gets a **second master-page object** (a new kind-67 object, 304 in `page-margins-twopage-master`, listed in object 256 item 2, with its own layout-guides object, name "A" and "Master Page A"), object 256 item 1 +1, item 45 1 -> 2, item 6 = true, the first master's item 16 3 -> 1 and its item 5/17 width 22,860,000 -> 26,746,200 (114,071,400); the document pages' item 13 (master ref) points to 304, object 282 item 4 follows. Facing-page masters are **not decoded further** (~).

### 4Q.7 Writer checklist
1. **Size (custom):** set 256 item 18, 289 item 3, the right/bottom guide records of 289 (W - right, H - bottom), 292 form C (item 18 element {3: W, 4: H}), item 5/17 of every kind-67 object (§4Q.2), the thumbnail (or omit it); leave the on-page shapes' Escher bytes untouched; add item 3 = true to every shape that was on the page.
2. **Then, for every on-page shape**, test its rectangle against the new page; if it no longer touches the page: move it to object 269 (parent in the index, the page's lists, 269's lists) and rewrite its anchor per §4Q.5; keep the shape container.
3. **Orientation:** swap W and H (item 18s, 289, 292, the 5/17 pairs); positions follow rule 1 of §4Q.4. Write the preset form (`Letter (Landscape)`) only when the new size equals a catalogue entry; otherwise form C.
4. **Margins:** rewrite the four records of 289; nothing else.
5. Read back with COM (`PageSetup.PageWidth/Height`, `LayoutGuides.Margin*`, `Pages(i).Shapes`, `ScratchArea.Shapes`) and compare with the manifest ground truth.

### 4Q.8 Not tested / open
Overlap items on scratch shapes: Publisher recomputes the text-box overlap items (42 replaced by 45/70/71) when moved shapes overlap on the scratch area; `pub-write` does not, and Publisher 2021 opens and resaves its files anyway ~. Facing-page masters beyond §4Q.6; masters that carry objects; a preset record combined with the old objects 293-295; the thumbnail pixel-size rule; catalogue ids for sizes other than the six above; "Create New Page Size..." (it stores a named custom size in the user's Publisher profile: not exercised so as not to change the installation); the other layout types of the Page Setup dialog (One page per sheet, Booklet, Email, Envelope, Folded card, Multiple pages per sheet, Web page) and label layouts (`PageSetup.Label`, gaps, side/top margin); sizes larger than the 1,800 pt scratch frame; objects on page 2 or on a master during a size change.

## 4R. Bullets and numbering (real Publisher 2021 COM, then Save(); our samples only; evidence `samples/gen/list-*.pub`, UI in `docs/ui/BULLETS.md`; 2026-11-21)
Method: copies of `list-empty.pub` (= `blank.pub` opened and saved by Publisher 2021, 16.0) got a text box with 3-9 paragraphs ("One", "Two", ...), were formatted through `TextRange.Paragraphs(i,1).ParagraphFormat` (`SetListType(type, bulletText)`, `ListNumberSeparator`, `ListNumberStart`, `ListIndent`, `ListBulletFontName`, `ListBulletFontSize`, `LeftIndent`, `FirstLineIndent`, `RightIndent`), saved with `Save()`, and the Quill `FDPP`/`FDPC` records and `FONT` table decoded with a throw-away Python walker (item encoding of 4.2) and diffed. Numbers were also checked by exporting the live document to PDF through COM (`ExportAsFixedFormat`) and reading the text. Marks: ✔ seen in two or more files or fully explained by the COM read-back, ~ seen once or inferred, ? unknown. Only the public automation interface, the visible program and our own files were used.

### 4R.1 Where a list lives: paragraph properties (`FDPP`) ✔
Lists are **paragraph properties** only. A paragraph that is not in a list has an empty `FDPP` record (just the length word), exactly as before; **nothing** is added to `Contents`, `Escher` or the Quill chunk list, the `STSH` style sheet is untouched, and the `TEXT` chunk has no list characters (the bullet or number is generated at layout time, never stored in the text) ✔ (`list-none` vs `list-bullet-default`: only the second `FDPP` record differs, and `Contents`/`EscherStm` are unchanged apart from the save counter). The `FDPP` record of a list paragraph is a block in the 4.2 encoding; item ids carry the 0x200 bit (ids below are written as the Quill code reads them, 0x2xx):

| id | class / type | Meaning | |
|---|---|---|---|
| 0x257 | 17, record (0x88) | **list descriptor**, present exactly when the paragraph is in a list. Sub-items: 0x200 = **list type** (u32, table 4R.2); 0x201 = **bullet character code** (u32; 0 for numbering); 0x202 = u32, always 0 in direct formatting (the two built-in list styles in `STSH` store 0xFFFF; meaning ?) | ✔ |
| 0x258 | 4, u32 | **number separator**: `index << 16`, index table 4R.3. Written for numbering paragraphs, **absent for bullets**; absent also for index 3 ("plain": `2`, no mark) | ✔ |
| 0x215 | 4, u32 | **start-at number** (`ListNumberStart`). Absent = 1 | ✔ |
| 0x20D | 4, u32 | **left indent** in EMU (12,700 per point), also for plain paragraphs | ✔ |
| 0x20C | 4, u32 (two's complement) | **first-line indent** in EMU, signed: a hanging indent is negative | ✔ |
| 0x20E | 4, u32 | **right indent** in EMU | ✔ |
| 0x206 | 4, u32 | **list indent** (`ListIndent`) in EMU, written **only when it is not the default 18 pt** (228,600); the value 0 is written too (`ListIndent` = 0) | ✔ |
| 0x202 | 4, u32 | **bullet size** in EMU (127,000 = 10 pt, 254,000 = 20 pt, 63,500 = 5 pt); bullets only | ✔ |
| 0x203 | 3, u16 | **bullet font**: index into the Quill `FONT` table (4R.4); bullets only | ✔ |

Absent items mean the default: indents 0, `ListIndent` 18 pt, start 1, bullet size/font from the style. Items are stored in ascending id order (4.2), so a writer emits 0x202, 0x203, 0x206, 0x20C, 0x20D, 0x215, 0x257, 0x258 in that order ✔ (every record seen).

**List indent and the indents ✔.** Applying a list (any type) sets left indent = `ListIndent` and first-line indent = -`ListIndent` (default 18 pt: 0x20D = 228,600, 0x20C = 0xFFFC8308, i.e. -228,600). `ListIndent` = 36 pt writes 0x206 = 457,200, 0x20D = 457,200, 0x20C = -457,200. `ListIndent` = 0 writes `0x206 = 0` and **neither** 0x20C nor 0x20D. Changing `LeftIndent`/`FirstLineIndent` afterwards changes only 0x20D/0x20C (COM `ListIndent` then still reads 18 and 0x206 stays absent) ✔ (`list-bullet-indent`, `list-num-indent36`, probe with `LeftIndent` = 54 and `FirstLineIndent` = -10). Plain indents without a list are the same items: `list-indent-plain` has {0x20C = -228,600, 0x20D = 457,200} (left 36 pt, first-line -18 pt), {0x20C = +228,600, 0x20D = 228,600} (left 18, first-line +18) and {0x20E = 228,600} (right 18 pt) ✔. Whether these items replace the indent bits that were "?" in 3.5b/3.6b: yes, 0x20C/0x20D/0x20E are the first-line, left and right indents.

**Removing a list leaves residue ✔.** `SetListType(255)` (none) deletes 0x257 and the indents (0x20C/0x20D) but **leaves** 0x202 (bullet size), 0x206 (= 0 when COM `ListIndent` reads 0) and 0x258 (when the paragraph had one). A reader must key "is in a list" on **0x257 alone**; the leftover items mean nothing (`list-bullet-then-none`: `0x202 = 127000, 0x206 = 0, 0x258 = 0x20000`, no 0x257). Switching a bullet to numbering keeps the old bullet size (0x202) and drops the font index (0x203) ✔ (`list-bullet-font-then-num`).

### 4R.2 List type (0x257 / 0x200) ✔
Read-back of `ParagraphFormat.ListType` (read-only property; set with `SetListType`) equals the stored value; none = 255 (no 0x257 item). Rendering checked by PDF text:

| value | meaning | renders (separator ".") |
|---|---|---|
| 255 | none | |
| 0 | Arabic | 1. 2. 3. |
| 1 | upper-case Roman | I. II. III. |
| 2 | lower-case Roman | i. ii. iii. |
| 3 | upper-case letters | A. B. C. |
| 4 | lower-case letters | a. b. c. |
| 5 | ordinal | 1st. 2nd. 3rd. |
| 6 | cardinal text | One. Two. Three. |
| 7 | ordinal text | First. Second. Third. |
| 22 | leading-zero Arabic | 01. 02. 03. |
| 23 | **bullet** | the character in 0x201 |

`SetListType` accepts only 0-7, 22, 23 and 255 (anything else: "Value does not fall within the expected range"); the object model documents about 47 types, so other numbering systems are not available in this build or need other arguments ? (probed 0-60 and 255/256/-1). The second argument of `SetListType` is the **bullet text** and is accepted only for type 23 (for numbering types a non-empty string is an error) ✔. `ListBulletText` has the bullet character as a one-character string (Symbol bullet = U+00B7 = 183; `*` = 42; U+2022 = 8226 is stored as 8226, so the code is a UTF-16 code unit, not limited to a byte) ✔.

### 4R.3 Number separator (0x258) ✔ / ~
`ParagraphFormat.ListNumberSeparator` takes exactly the nine values `index << 16`, index 0-8 (all others: E_INVALIDARG). Rendering (PDF text, type 0):

| index | stored 0x258 | renders |
|---|---|---|
| 0 | 0 | `1)` |
| 1 | 0x10000 | `(1)` |
| 2 | 0x20000 | `1.` (the default after `SetListType`) |
| 3 | item **absent** | `1` (no mark) ✔ (the COM read-back is still 196,608; absent is how the file says it) |
| 4 | 0x40000 | `1]` |
| 5 | 0x50000 | `1:` |
| 6 | 0x60000 | `[1]` |
| 7 | 0x70000 | `-1-` |
| 8 | 0x80000 | `1` followed by a character the PDF text did not show (wide comma, ~) |

### 4R.4 Bullet font and size ✔
New bullet fonts are appended to the Quill `FONT` table in order of first use (the base file already has Symbol at index 31, so Symbol bullets add nothing; `Wingdings` became 32, `Courier New` 32 in `list-bullet-chars`, `Arial` 33 in a probe); 0x203 is that index. The default bullet (`SetListType(23, "")`) is **U+00B7 in Symbol at 10 pt** (0x201 = 183, 0x203 = 31, 0x202 = 127,000), which is the first of the six default bullets of the document's bullet table (Contents object 291, see 5). `ListBulletFontSize` accepts 1 to 400 (0 and 1000 fail); 5 pt stores 63,500 and 400 pt 5,080,000 ✔. The bullet is not formatted like the text: size and font come from 0x202/0x203, and the number's font is the paragraph text's (no item).

### 4R.5 Numbering semantics ✔ / ~
- **A list is a run of adjacent list paragraphs.** The number of a numbered paragraph is *start* + the count of directly preceding paragraphs of the same list; a paragraph without 0x257 ends the run and the next list paragraph restarts at its start value (`list-num-mixed`: 1 2, plain, 1 2 in a live export) ✔ ~ (the saved bytes only show records; the numbers come from the live PDF export, and an earlier reopen attempt hung, but reopening is fine, see the last bullets of this section).
- **A different type or separator starts a new run** ✔ (live PDF export of Publisher-reopened files written by `pub-write`, 2026-11-21): nine adjacent type-0 paragraphs with separators 0..8 all showed their first number (`1)`, `(1)`, `1.`, `1`, `1]`, `1:`, `[1]`, `-1-`, `1` + invisible mark), and nine paragraphs of types 0,1,2,3,4,5,6,7,22 showed `1.`, `I.`, `i.`, `A.`, `a.`, `1st.`, `One.`, `First.`, `01.`. A run is therefore the adjacent list paragraphs with the same type, separator **and** start (`pub_format::list_labels`).
- **Reopening works** ✔: Publisher 2021 opened (read-only) all eleven files written by `pub-write` (local evidence files, 2026-11-21) in under 0.6 s each, and the COM read-back of every paragraph matched what was written (4R.8). The earlier hang (4R.5) was not caused by the records.
- **Start-at is shared by the run.** `ListNumberStart` = 5 on paragraph 1 wrote 0x215 = 5 on every adjacent list paragraph (all three in `list-num-start5`) and set on paragraph 2 only wrote 7 on paragraphs 2 **and** 3, not on paragraph 1 (`list-num-start-p2`); a different start value begins a new numbering run (a probe with 5 then 10 rendered 5. 6. 10. 11.) ✔.
- **Empty paragraphs are numbered.** An empty list paragraph has the same record and shows its number (a live render showed a bare `1.`) ✔ (`list-num-empty`: the middle paragraph is `\r` only and has a full record).
- A bullet paragraph between two numbered ones ends the numbering run (`list-num-bullet-mix`; the live render restarted at 1) ~.
- **Do not trust Publisher's live numbers after applying the list one paragraph at a time**: with `SetListType` called per paragraph the live layout showed 1 1 2 3 for four adjacent paragraphs whose saved records are identical to the whole-range case (1 2 3 4). `list-num-1dot` (whole range) and `list-num-1dot-each` (per paragraph) have byte-identical `FDPP` records ✔; the glitch is an in-memory layout artefact.
- Runs and `FDPP` (as in 4B/3.6): every paragraph has its own run record; consecutive identical records are **not** merged by Publisher (merging made Publisher hang on open in an earlier test) ✔.

### 4R.6 What our code reads and writes today
- **Reads (done, pw-cibs step 1):** `pub-format` `list.rs` + `quill.rs`: `Paragraph { list: Option<ListInfo>, indents: ParaIndents, .. }`. `ListInfo { kind: ListType, bullet_char, bullet_font (name through the Quill `FONT` table), bullet_size_pt: Option, separator (index 0-8; absent 0x258 = 3 for numbering), start (absent = 1), list_indent_pt (absent = 18) }`; the list exists exactly when 0x257 does (leftover 0x202/0x206/0x258 are ignored); `ParaIndents { left_pt, first_line_pt (signed), right_pt }` from 0x20D/0x20C/0x20E also for plain paragraphs. `list_labels(&[Paragraph]) -> Vec<Option<String>>` computes the displayed label of each paragraph (bullet character, or number text with separator, runs as in 4R.5); `number_text(kind, n)` formats Arabic, Roman, letters (`AA` after `Z`, Word's rule ~), ordinal (`1st`), cardinal / ordinal text (words up to 99, Word's capitalisation ~) and leading zero. Index 8 shows `1` plus U+3001 (a guess ~; the PDF text showed no mark). Styles are **not** consulted: a paragraph that uses a list style (4R.6 note 3) without direct items reads as not in a list (no sample has one).
- **Writes (done, step 1):** `pub-write` `set_story_lists(input, story_index, &[ListSpan])`, see 4R.8. Rendering (`pub2pdf`) and UI are still not done.
- **Renders:** `pub2pdf` draws no bullets or numbers; **UI:** the Home ribbon has "soon" placeholders for Bullets and Numbering (`crates/kernpage/src/ribbon.rs`), no dialog, no commands. FEATURES rows TXT-BULLET (pw-cibs), TXT-NUMBERING (pw-g5i8), TXT-LIST-INDENT (pw-4f86), DLG-BULLETS (pw-3v1k) are research only.
- Implementation notes: (1) reader: `Paragraph { list: Option<ListInfo { kind, bullet_char, bullet_font, bullet_size, separator, start }>, left_indent, first_indent, right_indent, list_indent }` from 0x257 etc., ignoring leftovers without 0x257; the bullet font index maps through the Quill `FONT` table; number text is computed at layout by counting the run; (2) writer: emit the items in id order, append new bullet fonts to `FONT` (and extend the font object in `Contents` the way text fonts already are), keep per-paragraph runs unmerged; (3) STSH list styles (styles -55 and -49) already carry bullet items (0x202, 0x203, 0x206, 0x20C, 0x20D, 0x257), so a paragraph using one of them without direct items must be read through the style: ?.

### 4R.7 Open items
- 0x257 / 0x202 (0 directly, 0xFFFF in the two bullet styles): meaning unknown. (Resolved 2026-11-21: a different type or separator restarts numbering, and Publisher reopens list files fine, see 4R.5.) Open: the character shown by separator index 8; Contents font table (kind 108) entries for bullet fonts (4R.8); whether clearing a numbered paragraph also writes 0x202; a paragraph that uses an STSH list style without direct items. The nine other `ListType` systems and the second bullet-text argument for numbers: not available through COM here. The Bullets and Numbering dialog's own Hebrew "double quote" option has no COM property.

### 4R.8 Writing lists (pw-cibs step 1; 2026-11-21)
`pub_write::set_story_lists(input, story_index, &[ListSpan { start, end, list: Option<ListInfo>, indents: ParaIndents }])`; offsets are UTF-16 units of the story and each span is one paragraph including its `\r` (`ListSpan::new` fills the indents Publisher gives: left = list indent, first line = -list indent). It is absolute for the items below and leaves every other item of the record alone; character runs, text and `STRS` do not change; each paragraph keeps its own `FDPP` run (the writer never merges them).
- **Items written, in id order** (the same order Publisher uses): 0x202 bullet size (bullets only, default 10 pt), 0x203 bullet font index (bullets only), 0x206 list indent (only when not 18 pt; 0 is written), 0x20C first-line (only when not 0; two's complement), 0x20D left (only when not 0), 0x20E right (only when not 0), 0x215 start (numbering, only when not 1), 0x257 list descriptor (`0x200` type, `0x201` bullet character or 0, `0x202` = 0; 22 bytes), 0x258 separator (numbering, only when the index is not 3).
- **Fonts:** a bullet font that is not in the Quill `FONT` table is appended (first use = next index), an empty name means Symbol; an existing name (any case) is reused. **Not done:** Publisher also adds the font to the **Contents font table (kind 108, item 2 list)** when it is used (`list-bullet-wingdings` -> `Calibri, Wingdings`; `list-bullet-chars` -> `Calibri, Symbol, Courier New`; it is removed again when the last use goes, `list-bullet-font-then-num` -> `Calibri`). `pub-write` does not touch kind 108 (it does not for character fonts either); Publisher 2021 opens such files and reads the bullet font and size correctly, so the table looks advisory (a "fonts in use" list). Adding it would need PANOSE and font-signature bytes per font ~.
- **Switching a bullet to numbering** keeps the old 0x202 (as Publisher: 4R.1) and drops 0x203 and any 0x258; numbering to a bullet drops 0x215/0x258 and writes 0x202/0x203.
- **Clearing a list: Publisher's residue is mimicked** (it is simple, and the result is byte-identical to `list-bullet-then-none`): a paragraph that had 0x257 loses 0x257, 0x203 and the indents it had, gets **0x206 = 0**, keeps 0x202, and a cleared *bullet* gets **0x258 = 0x20000** (index 2) when it had none. A cleared *numbered* paragraph keeps its own 0x258/0x215; whether Publisher also adds a 0x202 there was not tried ~. COM then reads `ListIndent` = 0 and `ListType` = 255 (`w-cleared`). A paragraph with no list that is set to "no list" is left without residue. The reader ignores all of it (4R.1).
- **Byte-identity with Publisher** ✔: onto `list-empty.pub` + a text box made by `add_text_box`, the paragraph records written equal Publisher's in `list-bullet-default/-all/-wingdings/-chars/-size/-indent`, `list-num-1dot/-1dot-each/-1paren/-aparen/-idot/-start5/-start-p2/-indent36/-mixed/-empty/-types/-seps/-bullet-mix`, `list-indent-plain`, and (two-step edits) `list-bullet-then-none`, `list-bullet-font-then-num`; the read-back through `pub-format` (labels, `ListInfo`, indents) equals the one of Publisher's file. Tests: `crates/pub-write/tests/lists.rs`, `crates/pub-format/tests/lists.rs`.
- **Publisher 2021 check (COM only, read-only open; 2026-11-21)** ✔: eleven files (`w-bullet-default`, `w-bullet-wingdings` (U+00A7 14 pt, U+00FC 10 pt), `w-num-1dot`, `w-num-aparen` (a)), `w-num-idot`, `w-num-start5`, `w-mixed` (3 lists with a plain paragraph and a bullet), `w-indent` (list indent 36, 0; plain left 54 / first -10 / right 12), `w-num-types`, `w-num-seps`, `w-cleared`) all open; `ParagraphFormat.ListType`, `ListBulletText`, `ListBulletFontName`, `ListBulletFontSize`, `ListNumberStart`, `ListNumberSeparator`, `LeftIndent`, `FirstLineIndent`, `RightIndent` and `ListIndent` of every paragraph equal the written values (47 of 47; a phantom paragraph after the last `\r` repeats the last one). PDF export (`ExportAsFixedFormat`) of `w-mixed`, `w-num-types`, `w-num-seps` shows the labels `list_labels` computes (4R.5).

## 4S. Superscript and subscript (real Publisher 2021 COM, then Save(); our samples only; evidence local evidence files (scrubbed copies local evidence files); 2026-11-22; pw-df5, row TXT-SCRIPT)
Method: copies of `list-empty.pub` got a text box with `H2O x2 One` (one paragraph, 10 characters, text run ends at 0x200 + 2 x 10 = 532 when untouched) and were formatted through `TextRange.Characters(i, 1).Font` (`Superscript`, `Subscript`, `Size`), saved with `Save()`, and the Quill `FDPC` character runs dumped and diffed (item encoding of 4.2). Read back with a read-only open, and rendering measured from a COM `ExportAsFixedFormat` PDF (glyph size and baseline per character). Only the public automation interface, the visible program and our own files were used. Marks as in 4R.

### 4S.1 The property: one u16 item, id 0x20F ✔
Superscript and subscript are one **character property**, in the run's property record in `FDPC` (not in `Contents`, `Escher`, `STSH` or `FONT`; the text is unchanged). The item is `0F 12 vv vv`: id byte 0x0F (full id **0x20F**), type 0x12 (2-byte payload), value **1 = superscript, 2 = subscript**; absent = normal ✔. It sits in ascending id order, i.e. **after** the size item 0x0C and before the language items 0x12/0x3E (and before 0x1E, 0x24, 0x44 when present).

| sample (text `H2O x2 One`) | Font call | FDPC runs (end, record bytes; language items `12 22 0409 0000 3e 22 0409 0000` omitted) |
|---|---|---|
| `s-plain` | none | text run to 532; mark run `...10000000` |
| `s-super` | `Characters(6,1).Superscript = True` | to 522 plain; to 524 `0f 12 0100`; to 532 plain; mark |
| `s-sub` | `Characters(2,1).Subscript = True` | to 514 plain; to 516 `0f 12 0200`; ... |
| `s-both` | both of the above | `0f 12 0200` on char 2, `0f 12 0100` on char 6 |
| `s-super20` | char 6 `Size = 20`, then `Superscript = True` | `0c 22 30e00300` (254,000 EMU = 20 pt) then `0f 12 0100` |
| `s-sub20` | char 2 size 20 + `Subscript` | `0c 22 30e00300`, `0f 12 0200` |
| `s-size20` | char 6 size 20 only | `0c 22 30e00300`, no 0x20F |
| `s-superall` | whole `TextRange.Font.Superscript = True` | one run to 532 with `0f 12 0100`; the paragraph-mark run has **no** 0x20F (the range excludes the mark) |
| `s-superoff` | `Superscript = True` then `False` | identical to `s-plain` (item removed, not written as 0) ✔ |

**Size is not changed ✔.** Publisher writes only 0x20F: no size item is added (a superscript char in a 10 pt run has no 0x0C item), and an explicit size (0x0C = 20 pt) is kept unchanged next to it. COM `Font.Size` of a superscript character still reads the nominal size (10, or 20 for `s-super20`). No baseline-offset item exists either; `Font.Position` reads 0 and `Scaling` 100 for super, sub and plain characters ✔. The reduced size and the raise/drop are applied at layout time from the value of 0x20F.

**Value detail found in the clean samples:** in runs that carry an explicit size item (0x0C) the 0x20F value reads 0xE001 / 0xE002 (`0f 12 01 e0` / `0f 12 02 e0`), not 0x0001 / 0x0002 as in plain-size runs. Our reader uses the low byte only; the writer emits 0x0001 / 0x0002 (not yet checked in Publisher for sized runs).

### 4S.2 COM ✔
`TextRange.Font` has `Superscript` and `Subscript` (the typelib spells them `SuperScript`/`SubScript`; Boolean-like Long, True = -1), alongside `Position` (0), `Scaling` (100), `Tracking`, `Size`, `SizeBi` etc. Read-back: set characters read `-1`, others `0`; a range that mixes them reads `-2` (mixed); the paragraph mark counts as part of a whole-box range (so a whole-box range with the mark unformatted reads -2). Setting both on the same character was not tried (the property is one value; a writer never emits both). Setting Superscript on a character that is already Subscript replaces the value (one item) ~ (inferred from the item being a single u16; not run).

### 4S.3 Rendering (PDF export, 10 pt Calibri, ratios from the PDF) ✔
| | glyph size | baseline vs the line's normal baseline |
|---|---|---|
| normal | 9.96 (10 pt, the PDF rounds) | 0 |
| superscript | 6.48 = **0.65 x** nominal (20 pt nominal: 12.96 = 0.647 x 20.04) | raised **0.47 x** nominal size (4.68 pt at 10 pt; 9.48 pt at 20 pt: 0.473) |
| subscript | 6.48 = **0.65 x** nominal | lowered **0.145 x** nominal size (1.44 pt at 10 pt) |

The nominal size (the run's 0x0C or the style size) is what scales, not the reduced size. Following characters move left because the superscript glyph is narrower (advance scales with size). Line height grows when a superscript is raised (the 20 pt sample line sits lower); exact leading effect not measured. Baselines were derived assuming the PDF glyph box bottom = baseline - 0.25 x size (fit from the 10/20 pt plain glyphs of `s-size20`); two independent sizes gave the same raise ratio, so the values are consistent, but they are measurements of the PDF, not a documented rule (rounded: 65 %, +47 %, -14.5 %).

### 4S.4 Writer / reader checklist
- Reader: character property 0x20F (u16) in `FDPC` run record: 1 super, 2 sub, other values unknown; absent = normal. Do not look for a size or offset item.
- Writer: add `0f 12 vv vv` in id order (after 0x0C); do not touch the size item; clear = omit. Split runs at the edges as for bold (4.3 / 3.5): the paragraph-mark run is never changed by a range that excludes the mark.
- Layout/PDF: draw at 0.65 x nominal size, baseline +0.47 x nominal (super) or -0.145 x nominal (sub); text metrics/line height as above ~.

### 4S.5 Open
Values other than 1 and 2 (none exposed by COM); superscript on the paragraph mark; the same property in `STSH` character styles (not looked at); Font.Position (raise/lower by points, a separate Publisher feature with its own item, not tested here); the exact leading change of a line with a superscript; whether the Quill style value is reached through the `FONT`/kind tables (no sign of it).

## 6. A complete minimal publication (for writing from nothing)

Purpose: everything a **blank one-page publication** contains, written so that a writer can emit a `.pub` without ever reading another `.pub`. Earlier sections describe the formats; this one lists every stream, block and record of the blank file, classifies every field, and says what we believe is the smallest set a writer must emit. Status: **Stage A (specification and hypothesis); nothing here has yet been tested by writing a file from scratch.** Stage B tests §6.9.

**Field classes** used in all tables of this section: **(a)** constant (identical in every file we made), **(b)** derived from the page size, **(c)** derived from other values in the file (counts, offsets, lengths, ids, object numbers, page count), **(d)** volatile (timestamps, save counters, version/build stamps, random-looking stamps, machine data), **(e)** unknown (constant in our files, meaning unknown; a writer copies the observed value).

### 6.0 Evidence and method
- Baseline: `samples/blank.pub` (owner-made, Publisher 2010). Publisher 2021 (16.0, COM) was driven on **copies** of it (file copy, `Open`, change, `Save`, `Close`; never SaveAs): `nochange` (open and save only), Letter portrait (page size set explicitly), Letter landscape (width 792 pt, height 612 pt), Legal (612 x 1008 pt), A4 (595.3 x 841.9 pt) and a 2-page Letter (`Pages.Add`). Files, dumps and diffs are in the local evidence files (not in the repository).
- Publisher 2021 re-writes every stream completely, so these files show what 2021 emits; differences between the 2010 baseline and `nochange` are the 2010-versus-2021 writer differences, and differences between the 2021 files separate page-size-dependent fields from constants and volatile ones.
- Limit: all five variants start from the same 2010 template, so a field that 2021 keeps from the template (for example the "creator" build numbers in §6.2.2) is not proven to be what a brand-new 2021 document carries. Setting the size through `PageSetup` is not identical to choosing a preset in the new-document dialog (§6.2.6).
- Totals for `nochange` (Publisher 2021): `Contents` 9,686 bytes, `Escher/EscherStm` 1,056, `Quill/QuillSub/CONTENTS` 19,968, summary information 50,852 (a thumbnail), the rest tiny. Landscape, Legal and A4 differ from Letter portrait only where §6.2.6 and §6.5 say so; **`EscherStm` is byte-identical for all five sizes and for one or two pages**, and so is every Quill byte except four 3-byte stamps (§6.3).

### 6.1 The compound file ([MS-CFB]) and its directory
- Header: format version 3 (512-byte sectors, sector shift 9), mini sector 64 bytes (shift 6), mini-stream cutoff 4096, byte order marker FFFE, CLSID field zero, no transaction signature. Observed sizes: 2 FAT sectors, 4 directory sectors (16 entries), 1 mini-FAT sector, no DIFAT sectors. A writer may choose any valid layout; sector placement, free sectors and the tail of the last directory sector carry no meaning (class d for the whole sector map).
- Streams under 4096 bytes live in the mini stream (EscherStm, both CompObj streams, Internal, DocumentSummaryInformation); `Contents`, `Quill/QuillSub/CONTENTS` and `SummaryInformation` are in normal sectors. **Zero-length streams (`Envelope`, `Escher/EscherDelayStm`) have start sector ENDOFCHAIN (0xFFFFFFFE)** in the files Publisher writes.
- Directory tree (all names are UTF-16; red-black tree as required by [MS-CFB], ordering = shorter name first, then upper-cased code-unit comparison; Publisher's own trees are valid, but any valid tree is expected to do):

| Path | Type | Class | Notes |
|---|---|---|---|
| Root Entry | root | CLSID {00021201-0000-0000-C000-000000000046} (a) | the Publisher document class (the same CLSID is inside the root `\x01CompObj`). A writer must set it ✔ (every sample). Root creation time 0, modification time = last save (d). |
| `\x01CompObj` | stream, 94 bytes | a | §6.5 |
| `\x03Internal` | stream, 12 bytes | a / e | §6.5 |
| `\x05SummaryInformation` | stream | b, d | §6.5; carries the thumbnail (size depends on the page) |
| `\x05DocumentSummaryInformation` | stream, 152 bytes | a, d | §6.5 |
| `Contents` | stream | | §6.2 |
| `Envelope` | stream, **0 bytes** | a | present in every Publisher file we have; purpose unknown (e) |
| `Objects` | storage, empty | a | embedded-object storage; **empty in a blank file**; creation/modification time = save time (d) |
| `VBA` | storage, empty | a | macro storage; empty and present in a blank file ~ (e: whether needed) |
| `Quill` | storage, empty itself | a | |
| `Quill/QuillSub` | storage, **CLSID {08C8F6DA-969D-11D1-8E02-00C04FB6FECE}** | a | the text engine's class ("Quill96 Story Group Class", from its CompObj). Expected to be checked by the text engine ~. |
| `Quill/QuillSub/\x01CompObj` | stream, 86 bytes | a | §6.5 |
| `Quill/QuillSub/CONTENTS` | stream | | §6.3 |
| `Escher` | storage | | |
| `Escher/EscherStm` | stream | | §6.4 |
| `Escher/EscherDelayStm` | stream, 0 bytes | a | holds picture data when pictures exist; empty in a blank file |

- Timestamps: stream entries have creation and modification time 0. Storage entries (`Objects`, `Quill`, `QuillSub`, `Escher`, `VBA`) carry FILETIME values within about one second of the save (class d); for `VBA` creation = modification. **Class (d); the writer may write the save time or zero.** State bits are 0 everywhere (a).
- Per-entry unknowns: none besides the above. Directory entries other than those listed (for example Quill-level or Escher-level extra storages) do not exist in a blank file.

### 6.2 `Contents` stream
Encoding rules are §4.1 and §4.2 (item = 16-bit word, id in bits 0-10, class in bits 11-15; ids below are **decimal** unless written `0x`). "ref" = a 4-byte object reference (class 13/14, i.e. type byte 0x68/0x70; value = object number); "u32" = class 4 (type 0x20); "u16" = class 3 (0x18); "flag" = class 0/1 (no payload; class 1 = true, class 0 = false); "string" = class 24 (0xC0), UTF-16 with a terminating zero; "offset" = class 23 (0xB8), a u32 stream offset. All numbers little-endian.

#### 6.2.1 Overall layout and the 0x2C-byte header
The stream is, in this order and with **no padding or alignment between parts**: header (0x2C bytes) | block 0 (§6.2.2) | block 2, the kind table (§6.2.3) | the objects in ascending object-number order, each one a block (§6.2.4; empty slots between numbers have no bytes) | the object index block (§6.2.5), which ends the stream (no trailing bytes). Header offsets of the two big blocks are in the header and in block 0 (both copies must agree).

| Offset | Size | Field | Class | Value / rule |
|---|---|---|---|---|
| 0x00 | u16 | signature | a | 0xACE8 |
| 0x02 | u16 | offset of block 0 | a | 0x2C |
| 0x04 | u16 | format generation | a | 1000 |
| 0x06 | u16 | writer sub-version | d | 0x000A written by 2021, 0x0206 by the 2010 sample. The meaning is unknown (e); copy 0x000A. |
| 0x08 | u32 | total stream length | c | |
| 0x0C | u16 | writer minor version | d | 0x1A (26) by 2021, 0x18 (24) by the 2010 sample; equals the low half of block-0 items 3/4 |
| 0x0E | u16 | | a | 0x0100 (the byte at 0x0F is 1) (e) |
| 0x10 | 10 bytes | | a | zero (e) |
| 0x1A | u32 | offset of the object index (§6.2.5) | c | |
| 0x1E | u32 | offset of the kind table, block 2 (§6.2.3) | c | = end of block 0 |
| 0x22 | u16 | | a | 14 (e) |
| 0x24 | u16 | writer major version | d | 16 by 2021, 14 by the 2010 sample |
| 0x26 | 6 bytes | | a | zero (e) |

#### 6.2.2 Block 0 (at 0x2C): stream identity
| Item | Kind | Value | Class | Notes |
|---|---|---|---|---|
| 1 | u16 | 0xACE8 | a | |
| 2 | ref | 256 | a | the root (document) object |
| 3, 4 | u32 | 0x03E8001A | d | (generation 1000 << 16) \| writer minor version 26 (the 2010 sample has 0x03E80018). |
| 5, 7 | u32 | 0x03E80018 | d | kept by 2021 from the template; probably the creator's version (~). A brand-new 2021 file is expected to carry its own; untested. |
| 8 | u32 | total stream length | c | equals header 0x08 |
| 9 | string | the file path at the time of the last save | d | **machine data; contains the user's path and therefore possibly a user name. The writer emits an empty string (just the terminator).** Untested whether an empty string is accepted (§6.9). |
| 10 | offset | object index offset | c | equals header 0x1A |
| 11 | offset | kind table offset | c | equals header 0x1E |
| 12 | u16 | 1 | a | (e) |
| 13, 14 | u32 | 14334 | d | build number of the program that last saved (16.0.14334 here); the 2010 sample has 4750 |
| 15, 16 | u32 | 4750 | d | build number of the program that created the template (the 2010 sample's build), kept by 2021 (~) |

#### 6.2.3 Block 2: the kind table (143 records)
A block with item 1 = u32 143 (the highest kind number, c), item 2 = u32 0x03E8001A (d; `(1000 << 16) | writer minor`, 0x03E80018 in the 2010 file) and item 3 = a **list** (class 18) of 143 elements, each an id-0 record (class 17). It is a constant table (class a) in all six files made by either program, regardless of page size or page count. Element *k* (k = 1..143) carries item 1 = u16 *k* (the kind number, c). Its meaning (my reading, ~): a registry that tells the loader, per object kind, which schema revision to use and how it is stored; objects in §6.2.4 use kinds from this range. The other items of every record are, per kind, **constants** whose meaning is unknown (e): item 3 u32 (`(1000 << 16) | schema revision`, all 1000-based; the revision is the number in the "rev" column), items 6, 7, 8, 10, 11 flags (an "x" means present), item 9 u16 (the "grp" column), item 12 u32 (the "store" column). **Kinds 65, 66, 80-82, 98, 100, 110, 111 are empty records (only item 1).** A kind not listed in the table of runs below does not exist.

| Kinds | rev | 6 | 7 | 8 | grp (9) | 10 11 | store (12) |
|---|---|---|---|---|---|---|---|
| 1-15 | 0 | x | | x | 1 | | 1 |
| 16-31 | 0 | | | x | 1 | | 2 |
| 32 | 0 | | | x | | | 3 |
| 33 | 0 | | | x | 4 | | 3 |
| 34-47 | 0 | | | x | | | 3 |
| 48-63 | 0 | x | x | x | 1 | | 4 |
| 64 | 0 | | | x | 1 | 11 | 5 |
| 65-66 | empty | | | | | | |
| 67-68 | 0 | x | x | x | 1 | | 5 |
| 69 | 0 | | | x | 1 | | 5 |
| 70 | 0 | | | x | 3 | | 5 |
| 71 | 0 | | | | | | 5 |
| 72 | 0 | | | | | 11 | 5 |
| 73 | 0 | | | x | 4 | | 5 |
| 74-75 | 0 | | | x | 3 | | 5 |
| 76 | 0 | | | x | 1 | | 5 |
| 77 | 0 | | | x | 4 | | 5 |
| 78 | 0 | | | x | 1 | | 5 |
| 79 | 0 | | | x | 2 | | 5 |
| 80-82 | empty | | | | | | |
| 83 | 0 | x | | x | 2 | 10 | 5 |
| 84 | 0 | | | x | | | 5 |
| 85 | 0 | | | | 4 | | 5 |
| 86 | 0 | | | x | 3 | | 5 |
| 87 | 0 | | | x | 4 | | 5 |
| 88 | 0 | | | x | | | 5 |
| 89 | 0 | x | x | x | 1 | | 5 |
| 90 | 0 | x | | x | 3 | 11 | 0xFFFFFFFF |
| 91 | 0 | | | x | | | 5 |
| 92 | 0 | | | x | 1 | | 5 |
| 93-94 | 0 | | | x | 3 | | 5 |
| 95-96 | 0 | | | x | 4 | | 5 |
| 97 | 0 | x | | x | 2 | | 5 |
| 98 | empty | | | | | | |
| 99 | 0 | | | x | 1 | | 5 |
| 100 | empty | | | | | | |
| 101 | 0 | | | x | 2 | | 5 |
| 102-104 | 0 | | | x | 4 | | 5 |
| 105 | 0 | | | x | | | 5 |
| 106-107 | 0 | | | x | 4 | | 5 |
| 108-109 | 0 | | | x | 3 | | 5 |
| 110-111 | empty | | | | | | |
| 112 | 0 | x | | x | 2 | | 5 |
| 113-115 | 0 | | | x | 2 | 10 | 5 |
| 116 | 0 | | | x | 4 | | 5 |
| 117 | 0 | x | | x | 2 | | 5 |
| 118 | 0 | | | x | | | 5 |
| 119-125 | 0 | | | x | 4 | | 5 |
| 126 | 0 | x | | x | 1 | | 5 |
| 127 | 0 | x | x | x | 1 | | 5 |
| 128 | 0 | | | x | 2 | | 5 |
| 129 | 0 | x | x | x | 1 | | 5 |
| 130 | 0 | | | x | 4 | | 5 |
| 131 | 15 | | | x | 4 | | 5 |
| 132 | 17 | | | x | 1 | | 5 |
| 133-134 | 17 | x | | x | 1 | | 0xFFFFFFFF |
| 135-136 | 17 | | | x | 1 | | 0xFFFFFFFF |
| 137 | 17 | x | | x | 1 | | 0xFFFFFFFF |
| 138 | 20 | | | x | 4 | | 5 |
| 139 | 20 | x | x | x | 1 | | 5 |
| 140 | 20 | | | x | 1 | | 5 |
| 141 | 1 | x | | x | 1 | | 5 |
| 142 | 1 | | | x | 1 | | 5 |
| 143 | 20 | | | x | 4 | | 5 |

Field count: 143 records; every non-key field of the table is class (a) with unknown meaning (e).

#### 6.2.4 Objects of a blank one-page document
Object numbers below are the numbers Publisher assigns in the blank file; a writer should reuse them (they are referenced from other objects, class c, and the parent links in the index must agree). **Numbers 258, 260, 286 and 290 are unused slots** (list slot `00 78` in the index, no body). An object that is just `04 00 00 00` is an **empty body** (kind present, no items); those are 257 (kind 114), 259 (kind 115), 261 (kind 70), 262 (kind 84), 267 (kind 96, page 1), 278 (kind 127), 283 (kind 97: story/object registry, §4P), 284 (kind 101: story registry, §4P).

**256, kind 68, the document object** (§4.3b lists the item table). Items present:
| Item | Value | Class |
|---|---|---|
| 1 u32 | number of kind-67 objects in item 2 (6; 7 with two pages) | c |
| 2 list of refs | 263, 266, 269, 272, 275, 279: master page, then the document pages in order (§4P.1), then four more kind-67 objects (§6.2.4 "helpers") | c |
| 3 ref | 287 (printer and paper list) | c |
| 4 ref | 291 (bullet table) | c |
| 8, 14 flags | true | e |
| 18 record | {1: u32 page width EMU, 2: u32 page height EMU} | b |
| 20 ref | 293 | c |
| 24, 25, 26 refs | 259, 261, 257 (empty kinds 115, 70, 114) | c |
| 32 ref | 282 | c |
| 33 ref | 262 (empty kind 84) | c |
| 34 ref | 285 (colour scheme) | c |
| 35 u32 | page serial counter: **1 for one page, 2 for two pages** (§4P.1) | c |
| 42 | 16 bytes, all zero (a GUID slot that is unused in a blank file) | a |
| 44 u16 | 5 | e |
| 45 u16 | 1 | e |
| 49 ref | 278 (empty kind 127) | c |
| 57 | flag, false (class 0) | e |
| 60 u32 | save counter: **1 in the 2010 sample, 2 after one 2021 save** (§4M) | d |
| 65 u32 | 0 | e |
| 68 ref | 292 (page-setup record) | c |
| 77 flag | true | e |

**263, kind 67, master page "A"** (parent 256): item 3 ref 289 (layout guides); 5 record {1: u32 22,860,000, 2: u32 22,860,000} (25 in; b: see §6.2.6); 6 an 8-byte value, 0 (a); 9 ref 264 (kind 96 child); 11 ref 265 (kind 119 child); 14 string "A" (name); 15 string "Master Page A"; 16 u32 3 (e; a kind-3 "master" page type, ~); 17 record {1: u32 110,185,200, 2: u32 110,185,200} (120.5 in; b).
**264, kind 96** (parent 263): item 5 u32 1 (e). **265, kind 119** (parent 263): the "web-form defaults" record described under 268.
**266, kind 67, page 1** (parent 256): items 5 {22,860,000 x 22,860,000} (b), 6 = 8-byte value **1** (c: page serial, §4P.1), 8 ref 295, 9 ref 267, 11 ref 268, 13 ref 263 (the master), 14 and 15 = empty strings (a), 16 u32 5 (e), 17 {110,185,200 x 110,185,200} (b).
**267, kind 96** (parent 266): empty body. **268, kind 119** (parent 266): items 12 u32 2, 14 string = an example mailing address, 15 string = a form-response subject line, 16 string = a thank-you message, 19 string = the data file name "FORMDATA.HTM", 23 u32 1, 24 string = an example script URL on the reserved example.com domain (all class a; they are the defaults of Publisher's web-form feature and a blank page carries them anyway; **a bare record without items 14-24 makes 2021 refuse the file**, §4P.2). Page 1 of the 2021 baseline file carries exactly these (the literals are the placeholder texts "someone@example.com", "Web Site Form Response", "Your information was received", "FORMDATA.HTM" and "http://example.com/~user/ispscript.cgi").
**269, 272, 275, 279, kind 67, four "helper" pages** (parent 256; unexplained, §4P.7): same items as page 1 except 6 = 0, no item 8, and each with its own kind-96 child (270, 273, 276, 280, empty) and kind-119 child (271, 274, 277, 281, the same web-form record). Object 269 carries different sizes: item 5 = {(1800 - W/2) pt, (1800 - H/2) pt} and item 17 = {(8676 - W/2) pt, (8676 - H/2) pt} (W, H = page size in pt, converted to EMU at 12,700 per pt; class b). 272, 275 and 279 carry 25 in / 120.5 in like the master. Their children also use item 13 = ref 263.
**282, kind 91** (parent 256): item 1 ref 283, item 2 ref 284, item 10 record {1: u32 0x08000000} (e).
**285, kind 92, colour scheme** (§4F): item 1 u32 8; item 2 list = [empty slot (black/automatic), then 7 records {1: u32 colour as bytes R, G, B, 0}]; item 3 u32 75 (built-in scheme id "Office"); item 6 string "Office". Colours in order: 1F497D, C0504D, 4F81BD, EEECE1, 0000FF (hyperlink), 800080 (followed hyperlink), FFFFFF. All class a.
**287, kind 75, printer and paper list** (parent 256): item 1 u32 2; item 2 list of **two identical records** {1: u16 1; 2: record {1: u32 7,772,400, 2: u32 10,058,400} (the template's Letter paper, **not** the page size; unchanged in the Legal/A4/landscape files); 3: u16 7; 6: flag true; 8: string = **a printer device name** (machine data, d; leave empty); 9, 10: empty strings; 14: 2 zero bytes}; item 3 ref 288; item 10 flag true; item 11 u32 1; item 16 u32 228,600; items 23, 24, 26, 28, 29 flags true; items 34, 35 u16 0. Except the printer name all fields are a/e.
**288, kind 79** (parent 287): item 2 list with one record {2: record {1: u32 0xFFFFFFFF}}; 3 u32 304,800; 4 u32 243; 12 and 13 u32 3,175; 14 u32 70; 15, 16, 17 flags true. 2010 additionally has items 7, 9, 10, 11, 18, 19 (a flag, three u32 128, 77, 255, two flags); 2021 omits them (so they are optional). Meaning of all: e.
**289, kind 76, layout guides** (parent 263): item 1 u32 4 (count, c); item 2 list of four records, in order {1: u32 left margin = 457,200 (flags 3 and 4)}, {1: u32 W - 457,200 (flags 2 and 4)}, {1: u32 top margin = 457,200 (flags 3, 4)}, {1: u32 H - 457,200 (flags 2, 4)} (the margins are 0.5 in; right and bottom guides b); item 3 record {1: u32 W, 2: u32 H} (b); items 4, 5 u32 45,720 (0.05 in); 6, 7 u16 2; 8-11 u32 165,100 (13 pt). Meaning of flags 2/3/4 and of items 4-11: e.
**291, kind 74, bullet table** (parent 256): item 1 u16 6 (count, c); item 2 list of six records {1: u16 character code, 2: u32 size EMU, 3: string "Symbol"}: (183, 127,000), (183, 152,400), (168, 127,000), (222, 127,000), (224, 127,000), (42, 127,000) (the six default bullet glyphs and their sizes; a).
**292, kind 138, page-setup record** (parent 256): one item, 4 = a record; **the 2010 sample's body is empty**, so the record is optional for reading ~. Two forms were seen. *Letter preset* (portrait): {2: u32 104, 3: u32 109, 4: u32 0x80010000, 6: string "Letter", 7: u32 0x01027100, 10: u32 0, 17: u32 1, 18: list[record {3: u32 W, 4: u32 H, 6, 7, 8, 9: u32 457,200}], 19: u32 1, 20: list[record {1: u32 W, 2: u32 H, 8: record {2: u32 0, 3: u32 0}}]}. *Custom size* (every size set through `PageWidth`/`PageHeight`: landscape Letter, Legal, A4): {2: u32 161, 5: string "Current", 8: u32 2, 10: 0, 17: 1, 18: list[record {3: W, 4: H, 6-9: 457,200}], 19: 1, 20: list[record {1: W', 2: H', 5: flag true, 8: record {2: 0, 3: 0}}]}. W and H in item 18 are the page size as shown (landscape = swapped, class b); **W' and H' in item 20 are the same paper in portrait order** (landscape Letter keeps 7,772,400 x 10,058,400 there). Items 2, 3, 4, 6, 7, 8 identify a paper catalogue entry (e); 457,200 are the four 0.5 in margins (class a); items 10, 17, 19 (e).
**293, kind 88** (parent 256): item 1 u32 178, item 4 ref 294, item 11 u32 179 (e; 178/179 did not change with page size or page count).
**294, kind 77** (parent 293) and **295, kind 77** (parent 266): items 0 u32 1; 1 record {0: u32 1, 20: u32 32, 22: u32 1, 38: u32 1, 58: u32 15}; 9 u32 1 (e). The same constants in both.

#### 6.2.5 Object index (last block of the stream)
A block: item 1 = u32 **next free object number** (296 in the one-page file, 299 in the two-page file; c), item 2 = u32 **last used object number** (295; 298; c), item 3 = a list (class 18) whose element *n* describes object number *n*. **Elements 0-255 are empty slots** (the 2-byte item `00 78`), and so are the unused numbers 258, 260, 286, 290. A used element is `00 88 | u32 length | items`:
| Item | Meaning | Class |
|---|---|---|
| 2 | u16 kind number | c |
| 4 | offset (class 23, u32) of the object's body in this stream | c |
| 5 | ref: parent object (absent for 256) | c |
| 6 | u16: 0x1002 for object 256, 0x0102 for every other object | a (e) |
| 8 | flag true: present on 256, 257, 259, 261, 262, 278, 282-285 and on kinds 67, 75, 79, 76, 74, 138, 88; **absent** on kinds 96, 119, 77 | a (e) |
| 11 | u16, constant per kind: 256 -> 23; 287, 292, 293 -> 20; 289 -> 15; 294, 295 -> 26; present with value 1 on all kind-67 objects, on kinds 96 and 119 (except page 1's empty kind-96 child 267, which has none), and on 282, 285, 288, 291; absent on 257, 259, 261, 262, 278, 283, 284 | a (e) |

Parents: 114, 115, 70, 84, 127, 91, 92, 75, 74, 138, 88 and all kind-67 objects have parent 256; 96 and 119 have their page; 79 has 287; 76 has 263; 97 and 101 have 282; 77 has 293 (294) or page 1 (295). The font-table slot of §4P.2 does not exist in a blank file (no kind 108 object). Object bodies are contiguous in number order, so each offset is the previous offset plus the previous body's length (c).

#### 6.2.6 What depends on the page size, what on the page count
Only the following change between Letter, Legal, A4 and landscape (all class b unless noted); everything else in `Contents` is identical except offsets/lengths (c), the path (d), the save counter (d) and the version stamps (d):
- Object 256 item 18 {W, H} (EMU: pt x 12,700; Letter 7,772,400 x 10,058,400; Legal 7,772,400 x 12,801,600; A4 7,560,310 x 10,692,130; landscape swaps W and H).
- Object 289: the guides W - 457,200 and H - 457,200 and item 3 {W, H}.
- Object 292 item 18 {W, H} (and item 20's portrait-order {W', H'}), and its form (preset "Letter" versus "Current", see above). **No separate orientation field exists in `Contents`**: landscape is item 18 swapped relative to item 20 (a landscape page chosen as a preset, or through the new-document dialog, was not tested).
- Object 269 items 5 and 17 (the half-page formula above).
- Objects 263, 266, 272, 275, 279 items 5 and 17 and 269's: for the sizes that are exact multiples of 0.25 pt (Letter, Legal and their landscape) the values are exactly 22,860,000 and 110,185,200 (1800 pt and 8676 pt). **For A4 they are smaller by 6,985 EMU (0.55 pt) in the first value and by 3,175 EMU (0.25 pt) in the second** (A4: 22,853,015 x 22,856,825 and 110,178,215 x 110,182,025; 269: 19,072,860 x 17,510,760 and 106,398,060 x 104,835,960). **The rounding rule is now solved, §4Q.2** (a = W/2 rounded up to a multiple of 0.6 pt); a writer should use the exact-multiple formula and subtract the same sub-point deficits only if Stage B shows Publisher cares (hypothesis: it does not).
- Two pages: object 256 items 1 (+1), 2 (new ref at the page's position), 35 (+1); a new kind-67 page object (296) with item 6 = (old item 35 << 32) \| 2 = 4,294,967,298 and its children 297 (kind 96, empty) and 298 (kind 119, the web-form record), index entries, and the larger offsets. **Escher and Quill do not change.**

### 6.3 `Quill/QuillSub/CONTENTS` (text engine) of a blank publication
Container rules: §3.1-§3.3, §3.9. **A blank file has no text, so it has none of `TEXT`, `FDPP`, `FDPC`, `BTEP`, `BTEC`, `STRS`, `MCLD`, `TOKN`, `TCD `**; those appear with the first story (§4L). The directory has **8 chunks** in one block.

#### 6.3.1 Stream header, directory and chunk placement
| Offset | Size | Field | Class | Value |
|---|---|---|---|---|
| 0x00 | 8 | tag | a | `CHNKINK ` |
| 0x08 | u16 | | a | 4 (e) |
| 0x0A | u16 | | a | 7 (e) |
| 0x0C | u16 | number of chunks | c | 8 |
| 0x0E | u16 | | a | 0x0300 (must be 0x0300, §3.9) |
| 0x10 | u32 | 0x200 x number of directory blocks | c | 0x200 |
| 0x14 | u32 | stream length | c | 19,968 here: the end of the last chunk rounded up to a multiple of 512, zero filled |
| 0x18 | 8 | block header | c | capacity 504, entries 8, next = 0xFFFFFFFF |
| 0x20 | 8 x 24 | directory entries (§3.2) | c | each entry: size 0x18, 4-character tag, instance 0, u32 1, format tag = the same 4 characters, offset, length; bytes after the last entry up to 0x200 are zero (a) |

Chunks, in this order, **packed back to back from offset 0x200 without alignment** (there is no `TEXT` chunk, so the first chunk starts at 0x200): `STSH` 0 (380 bytes), `STSH` 1 (16,442 bytes in 2021; 15,650 in the 2010 file), `STSH` 2 (1,460), `SYID` (8), `SGP ` (4), `INK ` (4), `FONT` (936), `PL  ` (72). Offsets and lengths are class c; the stream is zero filled to a 512 multiple after `PL  `.

The three style-sheet chunks and `FONT` share a **20-byte chunk header**: u32 payload length (= chunk length - 20, c), u32 entry count (c), u32 x, u32 y, u32 **stamp**. The stamp's low byte is always 0 and its other three bytes change with every save (volatile, class d; we saw dozens of different values; a writer may write 0, hypothesis). x and y: `STSH0` 0 and 4, `STSH1`/`STSH2` 0 and 0, `FONT` 0xB8119816 and 4 (class a, e: x of `FONT` is constant across all files and is probably a checksum of the font list). After the header comes a table of *count* u32 offsets, each **relative to the start of that table** (= byte 20 of the chunk), then the entries back to back.

#### 6.3.2 Style sheet (`STSH` 0, 1, 2): 36 built-in styles
The default style sheet is 36 styles. Each style has one entry in each chunk:
- **`STSH` 0** (36 entries of 6 bytes): u16 0 (a) and an i32 **style identifier** (all negative: built-in styles). In order: -1, -3, -6, -55, -5, -82, -7, -67, -49, -102, -107, -108, -109, -111, -112, -114, -115, -116, -117, -118, -119, -122, -123, -103, -104, -105, -4, -8, -81, -106, -110, -113, -120, -121, -63, -2 (class a; the names of these styles are not stored in the file, which is why the meaning is e).
- **`STSH` 2** (72 entries of 16 bytes): for style *k* (0-based, in the `STSH0` order) two entries, 2k and 2k+1, each {u16 7, i32 -1, u32 *k*, u16 4, u32 0} (a, except *k* which is c; the meaning of 7/-1/4 is e).
- **`STSH` 1** (72 entries): entry 2k = the style's **character properties**, entry 2k+1 = its **paragraph properties**. An entry is a u16 word count W (c), a block in the ordinary item encoding (§4.2; the u32 length at its start equals 2W - 2, c), then two trailing bytes whose values differ from entry to entry (they look like the first bytes of the next item, not data) (e; a writer copies the observed values, no effect is known). Items are the property items of §3.5/§3.6 (ids with the 0x200 bit).

Character entries carry: 0x20C size (u32 EMU; absent = inherit), 0x239 second size (u32, always equal to 0x20C when present; e), 0x224 **font map** (a record with one sub-record per writing system, each {0: u16 index into `FONT`}), 0x244 colour {0x200: u32 index into `PL`}, 0x248 {0x200: u32 0} (e), and (2021 only, optional because the 2010 file lacks it and 2021 accepts that file) 0x258 {0x200: u16 1, 0x201: u32 = the colour index again, 0x202: u32 100,000} (e; 100,000 probably means "100 %"). Some styles add 0x21F u32 1120 (e), flag 0x203 or 0x238 (italic-like, e). **Font map pattern A (20 styles: indexes 0, 5, 7, 10-18, 21, 22, 28-33):** sub-record ids 1-4 -> font 0 (Calibri, the default, so sub-record 0 is absent), 5 -> 1, 6 and 7 -> 2, 8 -> 3, 9 -> 4, 10 -> 5, 11 -> 6, 12 -> 7, 13 -> 8, 14 -> 9, 15 -> 10, 16 -> 11, 17 -> 12, 18 -> 13, 19 -> 14, 20 -> 1, 21 -> 15, 22 -> 16, 23 -> 17, 24 -> 18, 25 -> 16, 30 -> 19, 31 -> 20, 33 -> 21, 34 -> 22, 35 -> 23, 36 -> 24, 39 -> 25, 40 -> 26, 42 -> 0, 44 -> 0. **Pattern B (16 styles; the heading font, index 27 = Cambria):** the same, except ids 0-4, 42 and 44 -> 27, ids 6 and 7 -> 28 (Times New Roman), 17 -> 29, 21 -> 30. All class a; the sub-record ids are writing systems (Latin = 0).
Paragraph entries carry: 0x213 space after (u32 EMU), 0x230 u16 (packed paragraph flags, values 10501, 21253, 6405, 50693 or 5; e), 0x234 line spacing in the packed form of §3.6 (rule 2, `value >> 3` / 152,400 = lines); the two list styles also have 0x202, 0x203, 0x206, 0x20D, 0x20C (written as the 32-bit two's complement of -137,160, a hanging indent of 0.15 in) and 0x257 {0x200: u32 23, 0x201: u32 bullet character code, 0x202: u32 0xFFFF}.

Data table (EMU / 12,700 = pt; "lines" = line-spacing multiple; "-" = item absent = inherited; **column order = `STSH0` order; every cell is class a, meaning of the 0x230 value e**):

| k | style id | size pt | font map | space after pt | lines | 0x230 | extras |
|---|---|---|---|---|---|---|---|
| 0 | -1 | 10 | A | 6 | 1.19 | 10501 | the default body style (10 pt Calibri) |
| 1 | -3 | 16 | B | 6 | 1.19 | 10501 | |
| 2 | -6 | 14 | B | 6 | 1.19 | 10501 | |
| 3 | -55 | 9 | B | 12 | 1.19 | 21253 | list: bullet 183, size 10 pt (0x202 = 127,000) |
| 4 | -5 | - | B | 16 | 1.19 | 6405 | |
| 5 | -82 | 9.5 | A | 6 | 1.19 | 10501 | |
| 6 | -7 | 13 | B | 12 | 1.19 | 21253 | |
| 7 | -67 | 10 | A | 6 | 1.10 | 10501 | |
| 8 | -49 | 10 | B | 12 | 1.19 | 21253 | list: bullet 168, 0x202 = 114,300 |
| 9 | -102 | 36 | B | - | 1.19 | 5 | |
| 10 | -107 | 11 | A | - | 1.75 | 5 | |
| 11 | -108 | 8 | A | - | 1.19 | 5 | 0x21F = 1120 |
| 12 | -109 | 8 | A | 4 | 1.19 | 50693 | 0x21F = 1120 |
| 13 | -111 | 8 | A | - | 1.19 | 5 | |
| 14 | -112 | 9 | A | - | 1.19 | 5 | |
| 15 | -114 | 10 | A | - | 1.19 | 5 | |
| 16 | -115 | 10 | A | - | 1.19 | 5 | |
| 17 | -116 | 8 | A | - | 1.19 | 5 | |
| 18 | -117 | - | A | - | 1.19 | 5 | |
| 19 | -118 | 11 | B | - | 1.19 | 5 | |
| 20 | -119 | 11 | B | - | 1.19 | 5 | |
| 21 | -122 | 9 | A | - | 1.19 | 5 | |
| 22 | -123 | - | A | - | 1.19 | 5 | |
| 23 | -103 | 28 | B | - | 1.19 | 5 | |
| 24 | -104 | 18 | B | - | 1.19 | 5 | |
| 25 | -105 | 16 | B | - | 1.19 | 5 | |
| 26 | -4 | 14 | B | - | 1.19 | 5 | |
| 27 | -8 | 14 | B | 6 | 1.19 | 10501 | |
| 28 | -81 | - | A | 6 | 1.10 | 10501 | |
| 29 | -106 | 9.5 | A | 6 | 1.19 | 10501 | italic-like flags 0x203 and 0x238 |
| 30 | -110 | 10 | A | - | 1.19 | 5 | |
| 31 | -113 | - | A | - | 1.19 | 5 | |
| 32 | -120 | 10 | A | - | 1.19 | 5 | |
| 33 | -121 | 9 | A | - | 1.19 | 5 | |
| 34 | -63 | 48 | B | - | 1.19 | 5 | |
| 35 | -2 | 18 | B | - | 1.19 | 5 | |

Colour index in every character entry is 1 (black) except style 0 (index 0, the automatic colour). The font cells come from sub-record 0 (Latin; pattern A has none, which means the default font, Calibri). Line spacing 1.19 = 1,450,850 packed, 1.10 = 1,341,122, 1.75 = 2,133,602. Other writing-system sub-records, fine property details and the remaining 4-byte tails are exactly as in the table's two font maps; a writer builds each entry from this table plus the shared structure above.

#### 6.3.3 `FONT` (936 bytes), `PL` (72), `SYID` (8), `SGP ` (4), `INK ` (4)
- **`FONT`** (§3.7): 20-byte chunk header (above), 32 offsets, then 32 entries {u16 name length in characters, UTF-16 name, u32 tag}. Index: name (tag): 0 Calibri (3), 1 Sylfaen (4), 2 Tahoma (0x362), 3 Mangal (9), 4 Vrinda (10), 5 Raavi (20), 6 Shruti (21), 7 Kalinga (22), 8 Latha (23), 9 Gautami (24), 10 Tunga (25), 11 Kartika (11), 12 BrowalliaUPC (0x0202016B), 13 DokChampa (14), 14 Microsoft Himalaya (13), 15 Malgun Gothic (6), 16 MS Mincho (0x01030005), 17 PMingLiU (7), 18 SimSun (8), 19 Estrangelo Edessa (27), 20 MV Boli (28), 21 Iskoola Pota (15), 22 Nyala (18), 23 Plantagenet Cherokee (16), 24 Euphemia Regular CAS (17), 25 DaunPenh (29), 26 Mongolian Baiti (12), 27 Cambria (0x02010041), 28 Times New Roman (0), 29 EucrosiaUPC (0x02010187), 30 Batang (0x02010163), 31 Symbol (0x02010097). **Names are class a (the default fonts of the writing systems, plus the Latin body, heading and symbol fonts); the tag is e** (a number that looks like a script or charset code: small numbers for most fonts, packed larger values for indexes 12, 16, 27 and 29-31).
- **`PL`** (§3.8): header {u32 2 (entry count, c), u32 0x28 (a), u32 0}, then two entry blocks. Entry 0 (42 bytes) = {1: u32 bytes 00,00,00,08 (flag 8 = "automatic", a), 2: u32 0xFFFFFFFF, 3: u32 0x20000000, 4: u32 0xFFFFFFFF, 5: u32 0xFFFFFFFF, 6: raw 2 zero bytes}; entry 1 (18 bytes) = {1: u32 0 (black, flag 0), 6: raw 2 zero bytes}. All class a (the meanings of items 2-5 are e). The styles use index 0 (style 0) and 1 (all others).
- **`SYID`** (8 bytes): u32 1 and u32 0 (a; the list of story numbers, which is empty, plus a count-like first word; for a file with stories see §4L.4) (e).
- **`SGP `** (4 bytes) = `04 00 00 00` (an empty block, a). **`INK `** (4 bytes) = two u16 0x0074, 0x0074 (a, e).
- **Volatile bytes in this stream: the 24-bit stamps in the headers of the `STSH` 0, 1, 2 and `FONT` chunks only** (bytes 17-19 of each); everything else was identical across all six 2021 files, including the 2-page file and all page sizes.

### 6.4 `Escher/EscherStm` (drawings) of a blank publication
Public record format [MS-ODRAW] (§4A: 8-byte header = u16 version/instance, u16 type, u32 length; container records have version 15). **The stream was byte-identical for Letter portrait, Letter landscape, Legal, A4, and for one or two pages**, i.e. every field is class a except the lengths, counts and shape ids marked c. A page with no shape has no drawing of its own. Layout (1,056 bytes in 2021; 972 in the 2010 file):

1. **Drawing-group container 0xF000** (length 328; 244 in the 2010 form), holding in order:
   - **0xF006 drawing-group header** (40 bytes): u32 highest shape id + 1 = 0x0C03 (c), u32 cluster count + 1 = 4 (c), u32 shapes saved = 5 (c: sum of the three drawings' shape counts), u32 drawings saved = 3 (c), then three {drawing id, shape ids used} pairs: (1, 1), (2, 2), (3, 3) (c; the second number is *last shape id of that drawing - drawing id x 1024 + 1*).
   - **0xF00B shape-property table, instance = entry count** = the **default shape properties** of the document: 23 entries in 2021 (16 in the 2010 form). Entries are {u16 property id (bit 14/15 flags), u32 value}, ascending by id, no complex data. Values (2021 form): 0x081, 0x082, 0x083, 0x084 = 36,576 (text insets, 0.04 in); 0x180 = 0 (fill type); 0x181 = 0x08000007 and 0x183 = 0x08000007 (fill colours; a 0x08 top byte probably marks a colour-scheme reference, here scheme index 7 = white ~); 0x1BF = 0x00100000 (fill flags); 0x1C0 = 0x08000000 (line colour, scheme 0); 0x1C2 = 0x08000007; 0x1CB = 25,400 (line width, 2 pt); 0x1D7 = 2 (a line style enum, e); 0x1FF = 0x00080008 (line flags); 0x201 = 0x08000004 (shadow colour); 0x285, 0x2CB, 0x2CC, 0x2CE, 0x2CF = 0 (e); 0x384-0x387 = 36,576 (e). The 2010 form omits 0x180, 0x1CB, 0x285 and 0x2CB-0x2CF; it is accepted by 2021 (our 2010 baseline opens in 2021).
   - **0xF122 tertiary property table** (instance = 17 entries; 10 in the 2010 form, which omits 0x1FF and 0x295-0x29A): 0x1FF = 0x00400000, 0x295-0x29A = 0 (e), and five pairs of Publisher colour-scheme references with ids 0x540/0x542, 0x580/0x582, 0x5C0/0x5C2, 0x600/0x602, 0x640/0x642 whose values are always 0x08000000 and 0x08000007 (e).
   - **0xF11E split-menu colours** (16 bytes = four u32): 0x08000001, 0x08000000, 0x08000004, 0x100000F7 (a; the "no fill", "line", "shadow" and "3-D" colours of the colour menus).
2. Then three drawings, each **{u32 prefix, 0xF002 drawing container}**; the prefix is the drawing id minus 1 (2, 1, 0; c) and the containers come in the order drawing 3, 2, 1:
   - **Drawing 3** (container length 482): 0xF008 drawing header, **instance = drawing id 3**, data {u32 shape count 3, u32 last shape id 0x0C02} (c); a 0xF003 group container (458) holding (i) a 0xF004 shape container (40) with a 0xF009 group record (16 zero bytes) and a 0xF00A shape record (instance 0 = group, data {spid 0x0C00, flags 5 = group and patriarch}); (ii) a 0xF004 (200) with 0xF00A (instance 1 = rectangle, spid 0x0C01, flags 0x0A00), a 0xF00B table of 14 entries (0x081-0x084 = 36,576; 0x181 = 0xFFFFFFFF; 0x1BF = 0x001F000C; 0x1C0 = 0x08000000; 0x1C2 = 0x08000007; 0x1FF = 0x00080008; 0x201 = 0x08000004; 0x384-0x387 = 36,576), a 0xF122 table of 12 entries (0x1BF = 0x00600000, 0x1FF = 0x00400040 and the five scheme pairs above) and a 0xF010 **client anchor** whose data is an **empty Publisher block** (`04 00 00 00`; record version 10, **instance 0x1A in 2021, 0x18 in 2010**, class d); (iii) a 0xF004 (194) with 0xF00A (instance 20 = line, spid 0x0C02, flags 0x0A00), a 0xF00B table of 14 entries (0x081-0x084 = 36,576; 0x1BF = 0x00100000; 0x1C0 = 0xFFFFFFFF; 0x1C2 = 0x08000007; 0x1CB = 0; 0x1FF = 0x00080000; 0x201 = 0x08000004; 0x384-0x387 = 36,576), a 0xF122 table of 10 entries (the five scheme pairs) and a 0xF010 anchor with the Publisher block {1: u32 1} (10 bytes). These two shapes are the master page's built-in rectangle and line; the Contents objects that own them are not referenced by a 0xF011 client-data record, which is why a blank file has no Contents objects for them (e).
   - **Drawing 2** (130): 0xF008 instance 2, data {1, 0x0801}; a 0xF003 (48) with the patriarch 0xF004 (group record + 0xF00A spid 0x0800, flags 5), then a 0xF004 shape container directly in the drawing (50) with 0xF00A (instance 1, spid 0x0801, flags 0x0C00), a 0xF00B of 2 entries {0x304 = 9, 0x33F = 0x00010001} and a 0xF122 of 1 entry {0x1FF = 0x00400040} (e: the page-background shape of the document pages).
   - **Drawing 1** (72): 0xF008 instance 1, data {1, 0x0400}; a 0xF003 (48) with the patriarch only (spid 0x0400, flags 5). All **document pages' shapes are added to this drawing** (§4P.2, §4L).
- Unknown (e) in this stream: 0x1D7, 0x285, 0x2CB-0x2CF, 0x384-0x387, 0x295-0x29A, the colour-scheme pair ids 0x540-0x642, 0x304, 0x33F, and the purpose of the three drawings' prefix numbers and of drawing 2's second shape. Publisher 2021 writes no 0xF011 (client data) record and no 0xF00D (text box) in a blank file.
- `EscherDelayStm` is empty (a): pictures put data there later (§4C).

### 6.5 Small streams and the property sets
- **Root `\x01CompObj`** (94 bytes; [MS-OLEDS] "CompObj" layout, all class a): header {u16 1, u16 0xFFFE, u32 0x00000A03, u32 0xFFFFFFFF}, CLSID {00021201-0000-0000-C000-000000000046}, ANSI user-type string (u32 length 0x18, then the text "Microsoft Publisher 3.0" and a terminating zero), ANSI clipboard/ProgID string (u32 0x0E, "MSPublisher.3" and a terminator), u32 0 (reserved), the Unicode marker 0x71B239F4, then three u32 0 (empty Unicode strings). The 2010 and 2021 files are identical.
- **`Quill/QuillSub/\x01CompObj`** (86 bytes, a): same header, CLSID {08C8F6DA-969D-11D1-8E02-00C04FB6FECE}, user-type string "Quill96 Story Group Class" (u32 length 0x1A), then the clipboard-format marker u32 0xFFFFFFFF followed by u32 1, u32 0 (reserved), the Unicode marker 0x71B239F4, three u32 0.
- **`\x03Internal`** (12 bytes, a): u16 0, u16 1, then 8 zero bytes (e).
- **`Envelope`**: 0 bytes. **`Escher/EscherDelayStm`**: 0 bytes.
- **`\x05DocumentSummaryInformation`** (152 bytes; [MS-OLEPS]): header {u16 0xFFFE, u16 0, u32 system id (**d**: 0x0002000A written by 2021 = OS version and platform; 0x00020206 in the 2010 file), 16 zero bytes (CLSID), u32 1 (one section)}, section FMTID {D5CDD502-2E9C-101B-9397-08002B2CF9AE}, offset 0x30; the section is {u32 size 0x68, u32 6 properties}, property table (id, offset) = (1, 0x38), (0x17, 0x40), (0x0B, 0x48), (0x10, 0x50), (0x13, 0x58), (0x16, 0x60), then values: id 1 code page (VT_I2 1252, a), id 0x17 application version (VT_I4 = 0x00100000 written by 2021, 0x000E0000 by 2010; class d: major version << 16), ids 0x0B, 0x10, 0x13, 0x16 (VT_BOOL false each; a).
- **`\x05SummaryInformation`** (50,852 bytes for Letter): same header form (system id d, FMTID {F29F85E0-4FF9-1068-AB91-08002B27B3D9}, offset 0x30), then a section of 3 properties: id 1 code page (VT_I2 1252, a); id 4 **author** (VT_LPSTR; in all our files a short placeholder text from the template, not the user's name; **the writer leaves it empty or writes a fixed string**; d); id 17 **thumbnail** (VT_CF: u32 length, u32 format -1, u32 clipboard format 3 = metafile, then a metafile). The thumbnail is a standard memory metafile, wrapped in an 8-byte header {u16 mapping mode 8, u16 x extent, u16 y extent, u16 0} with extents = pixel size x 21.17 (the 120-dpi hundredth-millimetre scale), holding: set window origin, set window extent (height then width in pixels), a white brush fill of the whole window, and one **device-independent-bitmap blit of a 24-bit uncompressed bitmap** showing the page (white with a thin outline and the margin guides; most pixels are white), then the end record. Pixel size, bitmap size (c) and the whole stream length depend on the page: Letter 114 x 147 px (50,852 bytes), Letter landscape 147 x 114 (50,900), Legal 89 x 144 (38,876), A4 103 x 145 (45,524); the rule that picks the pixel size is e (all are close to 144-147 px high, width by aspect ratio). Class b for the thumbnail as a whole; **optional ~** (it only feeds Explorer previews; Stage B tests omitting it and a plain white bitmap).

### 6.6 Volatile and machine-specific fields (summary; class d)
| Where | Field | Writer rule |
|---|---|---|
| Directory | storage creation/modification FILETIMEs; root modification time | write the save time, or 0 |
| `Contents` header 0x06, 0x0C, 0x24; block 0 items 3-5, 7, 13-16; block 2 item 2 | version and build stamps (2021: generation 1000, minor 26, build 14334; creator build kept from the template) | write the 2021 values of this table (hypothesis: any of the 2010 or 2021 sets works) |
| Block 0 item 9 | **file path at save time (contains the user's profile path and so possibly a user name)** | **empty string** |
| Object 287 item 8 (two elements) | **printer device name** | **empty string** (or one fixed generic name); untested |
| Object 256 item 60 | save counter (1 in the 2010 sample, 2 after one 2021 save; §4M) | write 1 (hypothesis: any value) |
| `Contents` object 256 item 42 | GUID slot, all zero in every file | write zeros |
| Quill chunks `STSH` 0-2 and `FONT`: bytes 17-19 of the chunk header | 24-bit stamp that changes on every save | write 0 (hypothesis) |
| `\x05SummaryInformation`, `\x05DocumentSummaryInformation` | system id; application version; author string; thumbnail | see §6.5 |

### 6.7 Fields that matter to Publisher 2021 (evidence so far)
Seen to be validated (refusal or "cannot open" when wrong): `Quill` header fields 0x0E and 0x10 (§3.9); the kind-119 child bodies of pages (§4P.2); registries 283/284 and Escher containers that refer to missing Contents objects (§4P.4); unknown story numbers (§4P.5). Seen **not** to matter (file still opens): the save counter, a duplicate page serial, the font-table slot (§4P); the Quill stream left with orphan stories; **the whole file written by 2010** (2021 opens the 2010 baseline, with a `kind 138` body that is empty, no style item 0x258, a smaller Escher property table, 2010 version stamps and a printer name on a different machine). What is **not tested** and may be validated: root and QuillSub CLSIDs, the path string, the kind table, the stamps, the helper pages, the thumbnail, the `\x03Internal`/`Envelope`/`VBA`/`Objects` entries, the style-sheet tail bytes, the `FONT` constant 0xB8119816.

### 6.8 Hypothesis for the smallest set a writer must emit (Stage B will test it)
Layered, from safest to boldest. Stage B should first prove **H0**, then peel layers one at a time and keep each reduction that still opens, re-saves in 2021 without change of structure, and round-trips in our own reader.
- **H0 (full blank, nothing machine-specific):** every stream and record of §6.1-§6.5 with the field rules above: path string empty, printer names empty, thumbnail a plain white bitmap of the right size, stamps 0, save counter 1, version stamps copied, 2010-form Escher property tables, kind 138 in the "Letter" or "Current" form, no style item 0x258, no kind 79 optional items. **Expectation: opens in 2021.**
- **H1 (drop what 2010 files also lack or what only feeds previews):** omit the thumbnail and `\x05SummaryInformation`'s property 17 (keep id 1 and 4) or the whole summary stream; empty body for object 292; keep `\x03Internal`, `Envelope`, `VBA`, `Objects`, both `CompObj` streams, both CLSIDs.
- **H2 (boldest, likely too far):** also omit the four helper pages 269/272/275/279 and their children, object 288, the kind-77 objects 294/295, object 293, the `Objects`/`VBA` storages, `DocumentSummaryInformation`. The kind table (143 records), the three `STSH` chunks, the 32-entry `FONT` list, `PL`, the three Escher drawings, the master page 263 with its children and guides 289, the colour scheme 285, the document object 256 with its references, and the object index are expected to be **required** (Publisher dereferences these by number).
- Expected to be **required in all cases**: the three streams `Contents`, `Quill/QuillSub/CONTENTS`, `Escher/EscherStm`; root CLSID; QuillSub CLSID; the Quill header rules of §3.9; the kind-119 record body.

**Biggest risks for Stage B**
1. The style sheet is a 16 KB table whose per-entry trailing two bytes and several flag values are unexplained (class e); an error may corrupt text later rather than fail on open.
2. Unexplained constants might be checksums: the `FONT` header word 0xB8119816, the three-byte stamps, Contents item 6/11 of the index entries, header fields 0x06/0x0E/0x22.
3. The helper pages (269-279), their sizes and the 0.55/0.25 pt A4 deficit show that the sizes of kind-67 objects have a rule we do not know; Publisher might use them for layout of unusual sizes.
4. Only sizes set by `PageWidth`/`PageHeight` were sampled; the new-document dialog's presets (and their paper catalogue ids, object 292 items 2-8) and two-page spreads are untested.
5. The kind-table values come from one family of files (template 2010, writer 2010/2021); a brand-new 2021 document could carry other stamps or extra 2021-only records (the 2010 -> 2021 additions we saw are all optional: style item 0x258, Escher table entries, object 292 body).
6. Publisher validates cross references strictly (§4P); a hand-written object table with a wrong parent, ref or index item 11 may be refused.

### 6.9 What Stage B must test (checklist)
H0 file from nothing for Letter portrait; then each of: empty path; empty printer names; no thumbnail; all-zero stamps; omit 292 body; omit 0x258; 2010-form Escher; omit helper pages; omit summary streams; A4 (exact-multiple formula versus the observed sub-point deficits); a second page (§6.2.6); landscape. Evidence tools: local evidence files, `an_blk2.py`, `an_escher.py`, `an_quill.py`, `an_stsh.py` (git-ignored analysis scripts used for this section) and the repo's `tools/contents_tree.py`.

### 6.10 Stage B1 results (writer `pub-write::new_publication`; Publisher 2021, read-only open) ✔
Method: files generated by our writer and opened read-only in Publisher 2021 (16.0). When the first version was refused, hybrids (a Publisher-made blank with ONE of our streams, and the reverse) isolated `Contents` as the only faulty stream; then a structural rebuild of `Contents` (header, block 0, kind table, each object, index entries taken from either side) isolated the items below. The hybrids were evidence only; the writer is written from this text.
**Corrections to 6.2 (item classes; these were wrong or unspecified before and make 2021 refuse the file):**
- **References come in two classes.** A reference to an object the record *owns* (a child by the index parent link) is class 14 (type byte 0x70): object 256 items 3, 4, 20, 24, 25, 26, 32, 33, 34, 68 and the elements of the item-2 page list; kind-67 items 3 (master: layout guides), 8, 9, 11; kind 91 items 1, 2; kind 75 item 3; kind 88 item 4. A reference to a non-child (a page's item 13 = master page; 256 item 49; block-0 item 2) is class 13 (0x68).
- **Lists** inside object bodies are class 20 for 256 item 2, 285 item 2 (colour list), 288 item 2, 289 item 2 (guides), 291 item 2 (bullets); class 18 for 287 item 2 (printers), the kind-table list and the index list.
- **Records of class 19** (type 0x98) instead of 17: 282 item 10, the paper-size record (item 2) inside each 287 element, and the item-2 record inside the 288 element. All other nested records stay class 17.
- The 2-byte item 14 of a 287 element is class 16 (length-prefixed raw, 2 bytes), not class 2.
- Kind-table record item 9 (group) is class 2 (2-byte value), not class 3; object-index item 6 (0x1002 / 0x0102) is class 2, not class 3.
**Result: H0 opens** for Letter, Letter landscape, Legal, A4 (exact-multiple formula, no sub-point deficits needed; reported 595.3 x 841.9 pt), Tabloid, custom 5.5 x 8.5 in, and 2- and 4-page Letter (extra pages generated directly: objects 296+3k, 297+3k, 298+3k as in 4P.2, item 6 = (k << 32) | 2, 256 item 35 = page count, item 1 +1 each, page ids after page 1 and before the helper pages). Publisher reports the size we wrote and the page count.
**Reductions tried (one-page Letter):** opens: no thumbnail; no `\x05SummaryInformation`; no `\x05DocumentSummaryInformation`; empty body for 292; Letter preset versus "Current" form of 292; 2021-form Escher tables; style item 0x258 added; H1 (no thumbnail + empty 292). **Refused:** omitting the helper pages 269-281 ("cannot open"), omitting 293/294/295 ("0x80FF000D"; 293 is required), omitting the empty `Objects`/`VBA` storages ("cannot locate the file"), hence H2 is too far. Objects 257-262, 278, 282-285, 287-292, 294, 295 were individually tolerated as absent in the refused-file probing, but are kept. Unexplained values written as guessed constants (zeros) and accepted: the 2-byte trailer of each STSH1 entry, the author string (empty), STSH stamps (0), `FONT`/chunk constants as listed, thumbnail white bitmap.
**Refusal messages as an oracle:** missing `Contents`/`\x03Internal`/`QuillSub`/`Objects`/`VBA` -> "cannot locate the file"; bad Contents signature -> "not a Publisher file"; garbled Escher/Quill/kind-table bytes -> "detected a problem"; wrong item classes or missing required objects in Contents -> plain "cannot open" or HRESULT 0x80FF000D.
**Not covered by the writer in stage B1 (closed in stage B2, section 6.11):** `add_page`, `add_text_box`, `add_shape`, group and duplicate needed a kind-108 font-table object last in the index and (text) an existing `TEXT` chunk, which a blank file does not have; they failed on a generated file ("no font-table object", "no TEXT chunk").

### 6.11 Stage B2: what Publisher 2021 adds to a blank file with its first content (editing a generated file) ✔
Method (stage B2): copies of our generated blank (and of the Publisher-made `samples/blank.pub`) were opened in Publisher 2021 through COM, ONE change was made and the copy was saved with `Save()`; the streams before and after were compared (local evidence files (not in the repository)). Changes: a text box holding "Hello" (page 1, 72/72 pt, 200 x 50 pt); a rectangle; a line; `Pages.Add`; two text boxes; a rectangle plus a text box; a text box on page 2 of a two-page file. The generated file and the Publisher-made one behaved identically. Marks: the findings are ✔ (seen on both bases, several cases) unless stated. Publisher's save also tidies unrelated things (it drops unused entries from `STSH` 1, `FONT` and `PL`, removes the printer name and optional items of objects 287/288); none of that is needed and none is written.

**What each kind of edit adds**
| Edit | Contents objects added | Quill |
|---|---|---|
| rectangle / oval / line | the shape object only (next free number, kind 1 or 32, parent = page) | unchanged (8 chunks) |
| new page | the three page objects of 4P.2 only | unchanged |
| first text box | the text-box object, then **a font table (kind 108)** | gains seven chunks (below) |
| further text boxes | the text-box object, before the font table (4L) | story appended (4L.3) |

- **The font table (kind 108) exists only once a story exists.** A file with shapes or pages but no text has none. When it appears it is the last object in the index; every later new object takes the font table's number and the font table moves up by the number of new objects (this is the rule of 4L.2/4P, so it only needs a font table to exist first). In a file without one, a new object simply takes the next free number (`index item 1`) and its body goes at the end of the object blocks, before the index.
- **Font table body** (class a, constant; Calibri is the only record): item 1 = u32 1 (count); item 2 = class-20 list of one class-17 record with items 3 = u16 0, 4 = string "Calibri", 6 = u32 0x001000FF, 7 = u32 0x0005BE03, 11 = 24 raw bytes (Calibri's Windows font signature: six u32 0xE4007EFF, 0xC200247B, 9, 0, 0x200001FF, 0), 13 = class-16 raw 10 bytes (the PANOSE digits 2 15 5 2 2 2 4 3 2 4), 14 = u32 0x1400. Index entry {2: 108, 4: offset, 5: 282 (the kind-91 story root), 6: 0x0102, 8: true, 11: 1}. The story root (282) gains **item 4 = reference class 14 to the font table**, between its items 2 and 10 (so the one reference that the editors repoint when the font table moves is created here).
- **Registries 283 (kind 97) and 284 (kind 101)** are the 4-byte empty body in a blank file. With the first story each gets item 1 (u32 count = 1) and item 2 (class-20 list with one class-17 record): 283 record {1: story number, 3: reference (class 13) to the text-box object}; 284 record {1: story number, 7: 1, 8 and 9: class-11 u32 0xCB18967C (the constant of 4L.2)}. Both index entries gain **item 11 = u16 1**. (Later records follow 4L.2.)
- **The page object** of a blank page has neither item 1 nor item 2 (4P.1); the first child creates item 1 (count) and item 2 (class-20 list of class-14 references), before item 5.
- **Text-box index entry**: {2: 1, 4: offset, 5: page, 6: **0x0103**, 11: 22}; rectangles, lines, groups and the font table have item 6 = 0x0102. The body of a text box, rectangle or line is as in 4L.2/4N.2 (the high byte of the u16 in item 4 differs between saves, 0x00/0xA2/0xE2; it is a volatile flag, left 0). Index item 11 of the kind-77 objects 294/295 and item 6 of 287 are saved differently by 2021 and are irrelevant.
- **Escher**: a shape of a page goes into the drawing whose drawing id is 1 (the last drawing of the stream, spec 6.4; it has no shape of its own in a blank file); counters as in 4L.1. The 2021 text-box container is 392 bytes: 0xF00A (instance 0xCA, flags 0x0A00), 0xF00B with 24 properties, 0xF122 with 24 properties, 0xF010 (anchor), 0xF011 (client data) and 0xF00D (story). 0xF00B: 0x80 story number; 0x81-0x84 = 36,576; 0x85 = 1; 0xBF = 0x20000; 0x181 = 0x183 = 0x08000007; 0x1BF = 0x100000; 0x1C0 = 0x08000000; 0x1C2 = 0x08000007; 0x1CB = 25,400; 0x1FF = 0x80000; 0x201 = 0x08000004; 0x285, 0x2CB, 0x2CC, 0x2CE, 0x2CF = 0; 0x384-0x387 = 36,576. 0xF122: 0x8D = 73,152; 0x17F = 0x400040; 0x1FF = 0x400000; 0x295-0x29A = 0; the five scheme pairs 0x540/0x542, 0x580/0x582, 0x5C0/0x5C2, 0x600/0x602, 0x640/0x642 = 0x08000000/0x08000007 each preceded (all but the first) by 0x57F, 0x5BF, 0x5FF, 0x63F = 0x80000; 0x6FF = 0x20002. (The 2010 container of 4L.1 is accepted too; a writer that has one to copy copies it.)
- **Quill: the seven chunks of the first story.** A blank file's text stream has 8 chunks (`STSH` 0-2, `SYID`, `SGP `, `INK `, `FONT`, `PL`). With the first text it has 15, in this directory order: `TEXT` (at 0x200), `STSH` 0, 1, 2, `FDPP`, `FDPC` (each page at a 512 boundary), `SYID`, `SGP `, `INK `, `BTEP`, `BTEC` (format tag "PLC "), `FONT`, `STRS` ("PLC "), `MCLD`, `PL`. Header 0x0C = 15; 0x0E, 0x10 unchanged; the stream length stays a multiple of 512. Contents of the new chunks for "Hello": `TEXT` = "Hello\r"; `STRS` = {u32 1 (stories), u32 8, u32 0xFF, u32 6 (length), then a fixed 14-byte tail: u32 0, then a 10-byte block {item 0x200 = u32 5}}; each further story appends its length after the lengths and a 4-byte `04 00 00 00` to the tail (4L.3), so the tail of the first story is part of the chunk's fixed form; `SYID` = {H, 1, story} with the first story number **2** (blank `SYID` is {1, 0}; Publisher writes H = 3, the writer writes H = 2, either is accepted); `FDPC`/`FDPP`/`BTEP`/`BTEC` as in 4L.3 for a lone box (runs end at 0x200 + 2 x chars; `BTE` header bytes 4..12 = `04 00 00 00 00 00 00 00`); `MCLD` = {u32 1, u32 1, u32 1} then the one story record of 4L.3: a 12-byte block {item 0x200 flag, item 0x201 = 228,600}, u32 1 frame, and the 130-byte frame whose items are 0x200-0x203 (cached position, view dependent: for the box at 72/72 pt Publisher stored 7540, 7621, 7585, 7801, which do not follow the linear fit of 4L.3, so the value is not layout data), 0x204/0x205 = width/height EMU, 0x206-0x209 = 36,576, 0x20A = 82,676, 0x20B = u16 0, 0x20D, 0x211, 0x212 = 0, 0x213 = u16 0xFF, 0x214 = flag true, 0x215 = 1, 0x216 = 9,525, 0x218 = 0, 0x21A = flag false, 0x21D = record {0x200 = u32 0xFFFFFFFC}. A box on page 2 of a two-page generated file has the same frame.
- **Writer rules (implemented in `pub-write`).** (1) An editor that adds shapes or pages takes the font table's number if there is one and the next free number otherwise; it never creates a font table. (2) `add_text_box` first makes the font table if it is missing (appended, 282 item 4 added), creates the item 1/2 of both registries and the item 11 of their entries if the registries are empty, builds the Escher container from the lists above when the file has no text box to copy, and turns a text-less Quill stream into the story-less form of the 15 chunks (empty `TEXT`, empty formatting pages, `STRS` with no story and the fixed tail, empty `MCLD`) before appending the first story by the ordinary rule of 4L.3. (3) Deleting pages tolerates registries without items and a missing font table. Story numbers of deleted pages stay in Quill as before (6.7).
