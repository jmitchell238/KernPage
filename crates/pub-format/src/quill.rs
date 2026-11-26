//! `Quill/QuillSub/CONTENTS`: chunk directory (spec 3), the TEXT chunk, the STRS story
//! table (3.4), the FDPC character formatting page (3.5), the FDPP paragraph formatting
//! page (3.6) and the FONT table (3.7).

use crate::color::Color;
use crate::error::PubError;
use crate::items::{self, Item, Value};
use crate::list::{ListInfo, ListType, ParaIndents, DEFAULT_LIST_INDENT_PT, SEPARATOR_PLAIN};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"CHNKINK ";
/// First directory block header (spec 3.1); its entries start 8 bytes later.
const DIR_BLOCK: usize = 0x18;
const NO_NEXT: u32 = 0xFFFF_FFFF;
const ENTRY_LEN: usize = 24;
const FDP_PAGE_LEN: usize = 512;
const FDP_RUNS_AT: usize = 8;
const EMU_PER_POINT: f32 = 12_700.0;
/// Publisher 2010's default character size when no size item is present (spec 3.5).
const DEFAULT_SIZE_PT: f32 = 10.0;
/// Character property ids (spec 3.5); the 0x200 bit is part of the 11-bit id (spec 4.2).
const PROP_BOLD: u16 = 0x202;
const PROP_ITALIC: u16 = 0x203;
const PROP_SIZE: u16 = 0x20C;
const PROP_SCRIPT: u16 = 0x20F;
const PROP_UNDERLINE: u16 = 0x21E;
const PROP_FONT: u16 = 0x224;
const PROP_COLOR: u16 = 0x244;
/// Paragraph property id of alignment (spec 3.6).
const PROP_ALIGNMENT: u16 = 0x204;
/// Paragraph property ids of space before, space after (EMU) and the packed line spacing
/// (spec 3.6).
const PROP_SPACE_BEFORE: u16 = 0x212;
const PROP_SPACE_AFTER: u16 = 0x213;
const PROP_LINE_SPACING: u16 = 0x234;
/// Paragraph property ids of lists and indents (spec 4R.1).
const PROP_LIST: u16 = 0x257;
const PROP_LIST_SEPARATOR: u16 = 0x258;
const PROP_LIST_START: u16 = 0x215;
const PROP_LIST_INDENT: u16 = 0x206;
const PROP_BULLET_SIZE: u16 = 0x202;
const PROP_BULLET_FONT: u16 = 0x203;
const PROP_LEFT_INDENT: u16 = 0x20D;
const PROP_FIRST_INDENT: u16 = 0x20C;
const PROP_RIGHT_INDENT: u16 = 0x20E;
/// Sub-items of the list descriptor (0x257).
const LIST_TYPE: u16 = 0x200;
const LIST_BULLET_CHAR: u16 = 0x201;
/// Line spacing rules (low 3 bits of item 0x34) and the EMU that make one "line".
const RULE_MULTIPLE: u64 = 2;
const RULE_EXACT: u64 = 1;
const EMU_PER_LINE: f32 = 152_400.0;
const FONT_TABLE_ENTRIES_AT: usize = 0x14;

/// Superscript / subscript of a run (character property 0x20F, spec 4S.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum Script {
    #[default]
    Normal,
    Superscript,
    Subscript,
}

/// A stretch of story text with uniform character formatting. Paragraph breaks stay as `\r`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// Size in points (default 10 when the file has no size item).
    pub size_pt: f32,
    /// Font name from the Quill FONT table (empty if the table has no such entry).
    pub font: String,
    pub underline: bool,
    pub script: Script,
    /// Text colour from the `PL` colour list (spec 3.5, 3.8); `None` = default (black).
    pub color: Option<Color>,
    /// Set on a one-character run that is a field (spec 4T.3): its text is a literal `#`,
    /// drawn as the number of the page being shown.
    pub field: Option<Field>,
}

/// A field in story text (spec 4T.3; `TOKN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field {
    /// The current page number.
    PageNumber,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// Line spacing of a paragraph (spec 3.6).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LineSpacing {
    /// A multiple of the font's natural line height (1.0 = single).
    Multiple(f32),
    /// Exactly this many points per line.
    Exactly(f32),
    /// A rule not decoded yet (spec 3.6: "at least" is not sampled); the raw packed value.
    Unknown(u32),
}

/// Spacing of one paragraph (spec 3.6). `Default` is Publisher 2010's style default:
/// space before 0, space after 6 pt, 1.19 lines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParaSpacing {
    pub space_before_pt: f32,
    pub space_after_pt: f32,
    pub line: LineSpacing,
}

impl Default for ParaSpacing {
    fn default() -> Self {
        ParaSpacing {
            space_before_pt: 0.0,
            space_after_pt: 6.0,
            line: LineSpacing::Multiple(1.19),
        }
    }
}

/// One paragraph: its runs (the final run normally ends with `\r`), its alignment and
/// its spacing, its indents and the list it is in (spec 4R).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paragraph {
    pub runs: Vec<TextRun>,
    pub alignment: Alignment,
    pub spacing: ParaSpacing,
    /// `Some` exactly when the paragraph has the list descriptor `0x257`.
    pub list: Option<ListInfo>,
    pub indents: ParaIndents,
    /// Never set by the reader. Set by the editor's PDF export for the piece after a line
    /// break inside a paragraph: it continues the previous paragraph (no label, and it does
    /// not interrupt a numbering run).
    pub continued: bool,
}

/// Everything decoded from the Quill stream.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Quill {
    /// `SYID` (spec 4L, 4P): the story number of each story, in `TEXT`/`STRS` order.
    /// Empty when the chunk is missing or inconsistent.
    pub story_ids: Vec<u32>,
    /// The whole `TEXT` chunk.
    pub text: String,
    /// One entry per story, each as paragraphs of formatted runs.
    pub stories: Vec<Vec<Paragraph>>,
    /// `MCLD` (spec 4G, 4I): per story group, its frames in chain order. Empty when the
    /// chunk is missing or unreadable.
    pub frames: Vec<Vec<Frame>>,
    /// `TCD ` chunks (spec 4E.1.3) indexed by chunk INSTANCE number, which is the 0-based
    /// ordinal of the table's story among all stories. Missing or unreadable instances
    /// are empty vectors.
    pub cell_ends: Vec<Vec<u32>>,
}

/// One text frame of the Quill `MCLD` chunk (spec 4G, 4I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Frame {
    /// Item 4: frame width, EMU.
    pub width_emu: u32,
    /// Item 5: frame height, EMU.
    pub height_emu: u32,
    /// Item 0x0B + 1.
    pub columns: u32,
    /// Gap between columns, EMU: `(item 0x0A - 9525) / 2` rounded up, at least 0.
    pub column_gap_emu: u32,
}

/// Frame item ids (spec 4G, 4I).
/// Frame item ids (MCLD frame blocks); they carry the 0x200 id bit (spec 4.2).
const FRAME_WIDTH: u16 = 0x204;
const FRAME_HEIGHT: u16 = 0x205;
const FRAME_GAP: u16 = 0x20A;
const FRAME_COLUMNS: u16 = 0x20B;
const FRAME_GAP_BASE_EMU: u64 = 9_525;
/// Sanity bound on columns so a corrupt value cannot explode layout.
const MAX_COLUMNS: u64 = 64;

/// `MCLD` (spec 4G): `u32` (?), `u32` group count, that many `u32` (an index list), then
/// per group a small block, a `u32` frame count and one item block per frame. Returns
/// `None` when anything does not fit.
fn mcld(b: &[u8]) -> Option<Vec<Vec<Frame>>> {
    let groups = u32_at(b, 4)? as usize;
    let mut pos = groups.checked_mul(4)?.checked_add(8)?;
    let mut out = Vec::new();
    for _ in 0..groups {
        pos = pos.checked_add(u32_at(b, pos)? as usize)?; // the group's own small block
        let count = u32_at(b, pos)? as usize;
        pos += 4;
        let mut frames = Vec::new();
        for _ in 0..count {
            let len = u32_at(b, pos)? as usize;
            let items = items::decode_block(b.get(..pos.checked_add(len)?)?, pos).ok()?;
            let int = |id| items::find_int(&items, id);
            frames.push(Frame {
                width_emu: int(FRAME_WIDTH).unwrap_or(0) as u32,
                height_emu: int(FRAME_HEIGHT).unwrap_or(0) as u32,
                columns: (int(FRAME_COLUMNS).unwrap_or(0) + 1).min(MAX_COLUMNS) as u32,
                column_gap_emu: (int(FRAME_GAP)
                    .unwrap_or(0)
                    .saturating_sub(FRAME_GAP_BASE_EMU)
                    .div_ceil(2)) as u32,
            });
            pos += len;
        }
        out.push(frames);
    }
    Some(out)
}

/// Character formatting of one UTF-16 unit; `font` is an index into the FONT table.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CharFmt {
    bold: bool,
    italic: bool,
    size_pt: f32,
    font: usize,
    underline: bool,
    script: Script,
    /// Index into the `PL` colour list.
    color: Option<usize>,
    /// The unit is a `TOKN` page-number field (spec 4T.3).
    field: bool,
}

impl Default for CharFmt {
    fn default() -> Self {
        CharFmt {
            bold: false,
            italic: false,
            size_pt: DEFAULT_SIZE_PT,
            font: 0,
            underline: false,
            script: Script::Normal,
            color: None,
            field: false,
        }
    }
}

struct Chunk {
    tag: [u8; 4],
    offset: usize,
    len: usize,
    /// Instance index among chunks of the same type (spec 3.2).
    instance: u16,
    /// Position of the directory entry, for error messages.
    entry_pos: usize,
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    let s = b.get(o..o.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn directory(data: &[u8]) -> Result<Vec<Chunk>, PubError> {
    if data.get(..8) != Some(&MAGIC[..]) {
        return Err(PubError::format(
            0,
            "Quill stream does not start with `CHNKINK `",
        ));
    }
    let total = data
        .get(0x0C..0x0E)
        .map(|s| u16::from_le_bytes([s[0], s[1]]) as usize)
        .ok_or_else(|| PubError::format(0x0C, "Quill stream too short for a directory"))?;
    let mut chunks = Vec::new();
    let mut visited: Vec<usize> = Vec::new();
    let mut block = DIR_BLOCK;
    loop {
        if visited.contains(&block) {
            return Err(PubError::format(block, "directory block chain loops"));
        }
        visited.push(block);
        let hdr = block
            .checked_add(8)
            .and_then(|end| data.get(block..end))
            .ok_or_else(|| PubError::format(block, "directory block lies outside the stream"))?;
        let count = u16::from_le_bytes([hdr[2], hdr[3]]) as usize;
        let next = u32_at(hdr, 4).unwrap_or(NO_NEXT);
        for i in 0..count {
            let pos = block + 8 + i * ENTRY_LEN;
            let e = pos
                .checked_add(ENTRY_LEN)
                .and_then(|end| data.get(pos..end))
                .ok_or_else(|| PubError::format(pos, "directory entry lies outside the stream"))?;
            if u16::from_le_bytes([e[0], e[1]]) as usize != ENTRY_LEN {
                return Err(PubError::format(pos, "directory entry size is not 0x18"));
            }
            chunks.push(Chunk {
                tag: [e[2], e[3], e[4], e[5]],
                offset: u32_at(e, 16).unwrap_or(0) as usize,
                len: u32_at(e, 20).unwrap_or(0) as usize,
                instance: u16::from_le_bytes([e[6], e[7]]),
                entry_pos: pos,
            });
        }
        if next == NO_NEXT {
            break;
        }
        block = next as usize;
    }
    if chunks.len() != total {
        return Err(PubError::format(
            0x0C,
            format!(
                "directory holds {} entries but the header says {total}",
                chunks.len()
            ),
        ));
    }
    Ok(chunks)
}

fn chunk_bytes<'a>(data: &'a [u8], c: &Chunk) -> Result<&'a [u8], PubError> {
    c.offset
        .checked_add(c.len)
        .and_then(|end| data.get(c.offset..end))
        .ok_or_else(|| {
            PubError::format(
                c.entry_pos,
                format!(
                    "{} chunk lies outside the stream",
                    String::from_utf8_lossy(&c.tag)
                ),
            )
        })
}

fn units(bytes: &[u8]) -> Vec<u16> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs.iter().map(|c| u16::from_le_bytes(*c)).collect()
}

/// Parse the whole Quill stream. A missing `TEXT` chunk means no text (blank.pub).
pub fn read(data: &[u8]) -> Result<Quill, PubError> {
    let chunks = directory(data)?;
    let find = |tag: &[u8; 4]| chunks.iter().find(|c| &c.tag == tag);
    let all = |tag: &[u8; 4]| {
        let mut v: Vec<&Chunk> = chunks.iter().filter(|c| &c.tag == tag).collect();
        v.sort_by_key(|c| c.instance);
        v
    };
    let Some(text_chunk) = find(b"TEXT") else {
        return Ok(Quill::default());
    };
    let text = units(chunk_bytes(data, text_chunk)?);
    let fonts = match find(b"FONT") {
        Some(c) => font_names(chunk_bytes(data, c)?, c)?,
        None => Vec::new(),
    };
    let palette = match find(b"PL  ") {
        Some(c) => palette(chunk_bytes(data, c)?),
        None => Vec::new(),
    };
    let mut fmts = vec![CharFmt::default(); text.len()];
    // Every instance is one more page of runs; apply them in instance order (spec 3.1).
    // Runs continue across pages: each page starts where the previous one ended.
    let mut next_run = text_chunk.offset;
    for c in all(b"FDPC") {
        for (from, to, record) in fdp_spans(data, c, text_chunk, text.len(), &mut next_run)? {
            fmts[from..to].fill(char_fmt(&record));
        }
    }
    // `TOKN` (spec 4T.3): a `PLC ` whose first `u32` is the field count n and whose cps start
    // at offset 12 (n + 1 of them: a field per cp, then the end). ~ A cp is a position in
    // the whole TEXT chunk; it is a field only when that unit is a literal `#`.
    if let Some(c) = find(b"TOKN") {
        let b = chunk_bytes(data, c)?;
        let n = u32_at(b, 0).unwrap_or(0) as usize;
        for i in 0..n.min(1 << 12) {
            let Some(cp) = u32_at(b, 12 + 4 * i).map(|v| v as usize) else {
                break;
            };
            if text.get(cp) == Some(&0x23) {
                fmts[cp].field = true;
            }
        }
    }
    let mut aligns = vec![Alignment::Left; text.len()];
    let mut spacings = vec![ParaSpacing::default(); text.len()];
    let mut lists = vec![(None, ParaIndents::default()); text.len()];
    let mut next_run = text_chunk.offset;
    for c in all(b"FDPP") {
        for (from, to, record) in fdp_spans(data, c, text_chunk, text.len(), &mut next_run)? {
            aligns[from..to].fill(alignment(&record));
            spacings[from..to].fill(spacing(&record));
            lists[from..to].fill(list_and_indents(&record, &fonts));
        }
    }
    let lengths = match find(b"STRS") {
        Some(c) => story_lengths(chunk_bytes(data, c)?, c, text.len())?,
        // UNCONFIRMED: with no STRS chunk the whole text is treated as one story.
        None => vec![text.len()],
    };
    let mut stories = Vec::new();
    let mut start = 0;
    for len in lengths {
        // story_lengths guarantees start + len <= text.len().
        let end = start + len;
        stories.push(paragraphs(
            &text[start..end],
            &fmts[start..end],
            &aligns[start..end],
            &spacings[start..end],
            &lists[start..end],
            &fonts,
            &palette,
        ));
        start = end;
    }
    let frames = find(b"MCLD")
        .and_then(|c| mcld(chunk_bytes(data, c).ok()?))
        .unwrap_or_default();
    let mut cell_ends: Vec<Vec<u32>> = Vec::new();
    for c in all(b"TCD ") {
        let slot = usize::from(c.instance);
        if slot > 1 << 16 {
            continue;
        }
        if cell_ends.len() <= slot {
            cell_ends.resize(slot + 1, Vec::new());
        }
        if let Some(ends) = chunk_bytes(data, c).ok().and_then(tcd) {
            cell_ends[slot] = ends;
        }
    }
    let story_ids = find(b"SYID")
        .and_then(|c| {
            let b = chunk_bytes(data, c).ok()?;
            let n = u32_at(b, 4)? as usize;
            (b.len() >= 8 + 4 * n).then_some(())?;
            (0..n).map(|i| u32_at(b, 8 + 4 * i)).collect::<Option<Vec<u32>>>()
        })
        .unwrap_or_default();
    Ok(Quill {
        story_ids,
        text: String::from_utf16_lossy(&text),
        stories,
        frames,
        cell_ends,
    })
}

/// `TCD ` (spec 4E): 12-byte header (`u32` cell count - 1, then two more), then one `u32`
/// per cell. `None` when the chunk is too short for its own count.
fn tcd(b: &[u8]) -> Option<Vec<u32>> {
    let n = (u32_at(b, 0)? as usize).checked_add(1)?;
    let end = n.checked_mul(4)?.checked_add(12)?;
    if end > b.len() {
        return None;
    }
    (0..n).map(|i| u32_at(b, 12 + 4 * i)).collect()
}

/// STRS (spec 3.4): `u32 count`, `u32` (8), `u32` (0xFF), then `count` u32 lengths in
/// UTF-16 units. Returns the lengths after checking they fit in the text.
fn story_lengths(b: &[u8], c: &Chunk, text_len: usize) -> Result<Vec<usize>, PubError> {
    let bad = |m: &str| PubError::format(c.entry_pos, format!("STRS: {m}"));
    let count = u32_at(b, 0).ok_or_else(|| bad("chunk too short"))? as usize;
    count
        .checked_mul(4)
        .and_then(|n| n.checked_add(12))
        .filter(|&n| n <= b.len())
        .ok_or_else(|| bad("story count does not fit in the chunk"))?;
    let mut out = Vec::with_capacity(count);
    let mut total = 0usize;
    for i in 0..count {
        let len = u32_at(b, 12 + 4 * i).ok_or_else(|| bad("truncated"))? as usize;
        total = total
            .checked_add(len)
            .filter(|&t| t <= text_len)
            .ok_or_else(|| bad("story lengths exceed the TEXT chunk"))?;
        out.push(len);
    }
    Ok(out)
}

/// FDPC/FDPP (spec 3.5, 3.6): consecutive 512-byte pages. Returns, for every run, the
/// UTF-16 unit range of TEXT it covers (clamped to the text) and its decoded property
/// record.
fn fdp_spans(
    data: &[u8],
    c: &Chunk,
    text_chunk: &Chunk,
    text_len: usize,
    run_start: &mut usize,
) -> Result<Vec<(usize, usize, Vec<Item>)>, PubError> {
    let bytes = chunk_bytes(data, c)?;
    let name = String::from_utf8_lossy(&c.tag).into_owned();
    if bytes.len() % FDP_PAGE_LEN != 0 {
        return Err(PubError::format(
            c.entry_pos,
            format!(
                "{name} chunk is {} bytes; it must be whole 512-byte pages",
                bytes.len()
            ),
        ));
    }
    let bad = |m: &str| PubError::format(c.offset, format!("{name}: {m}"));
    let mut out = Vec::new();
    for page in bytes.as_chunks::<FDP_PAGE_LEN>().0 {
        let n = u16::from_le_bytes([page[0], page[1]]) as usize;
        if FDP_RUNS_AT + 6 * n > FDP_PAGE_LEN {
            return Err(bad("run count does not fit in the page"));
        }
        let ends = FDP_RUNS_AT;
        let props = FDP_RUNS_AT + 4 * n;
        for i in 0..n {
            let end = u32_at(page, ends + 4 * i).ok_or_else(|| bad("truncated"))? as usize;
            let rec = u16::from_le_bytes([page[props + 2 * i], page[props + 2 * i + 1]]) as usize;
            if end < *run_start {
                return Err(bad("run ends are not increasing"));
            }
            let record = items::decode_block(page, rec)?;
            // Convert stream byte offsets to unit indexes within TEXT, clamped to it.
            let from = ((*run_start - text_chunk.offset) / 2).min(text_len);
            let to = ((end - text_chunk.offset) / 2).min(text_len);
            out.push((from, to, record));
            *run_start = end;
        }
    }
    Ok(out)
}

fn is_flag(record: &[Item], id: u16) -> bool {
    items::find(record, id).is_some_and(|it| it.class == 1)
}

/// Item `id` of a record-valued item, e.g. the font record's sub-record or the colour
/// record's item 0.
fn inner_int(record: &[Item], id: u16, inner: u16) -> Option<u64> {
    match &items::find(record, id)?.value {
        Value::Record(v) => items::find_int(v, inner),
        _ => None,
    }
}

fn char_fmt(record: &[Item]) -> CharFmt {
    // Font: item 0x24 is a record of per-script sub-records; sub-record 0 (Latin) holds
    // item 0 = FONT table index. Absent = index 0 (spec 3.5).
    let font = match items::find(record, PROP_FONT).map(|i| &i.value) {
        Some(Value::Record(subs)) => match items::find(subs, 0).map(|i| &i.value) {
            Some(Value::Record(sub)) => items::find_int(sub, 0),
            Some(Value::Int(v)) => Some(*v),
            _ => None,
        },
        _ => None,
    };
    CharFmt {
        bold: is_flag(record, PROP_BOLD),
        italic: is_flag(record, PROP_ITALIC),
        size_pt: items::find_int(record, PROP_SIZE)
            .map_or(DEFAULT_SIZE_PT, |emu| emu as f32 / EMU_PER_POINT),
        font: font.unwrap_or(0) as usize,
        // UNCONFIRMED: underline = item 0x1E present with a non-zero value (one sample,
        // value 0x0401 for single underline).
        underline: items::find_int(record, PROP_UNDERLINE).is_some_and(|v| v != 0),
        // Only the low byte counts: runs with an explicit size carry 0xE001 / 0xE002 (spec 4S.1).
        script: match items::find_int(record, PROP_SCRIPT).map(|v| v & 0xFF) {
            Some(1) => Script::Superscript,
            Some(2) => Script::Subscript,
            _ => Script::Normal,
        },
        color: inner_int(record, PROP_COLOR, 0x200).map(|v| v as usize),
        field: false,
    }
}

fn alignment(record: &[Item]) -> Alignment {
    // Spec 3.6: absent = left, 1 = centre, 3 = right, 4 = justify. UNCONFIRMED: value 2
    // (never seen) and anything else map to left.
    match items::find_int(record, PROP_ALIGNMENT) {
        Some(1) => Alignment::Center,
        Some(3) => Alignment::Right,
        Some(4) => Alignment::Justify,
        _ => Alignment::Left,
    }
}

/// Spec 3.6: absent items take the style default (before 0, after 6 pt, 1.19 lines).
fn spacing(record: &[Item]) -> ParaSpacing {
    let mut out = ParaSpacing::default();
    if let Some(v) = items::find_int(record, PROP_SPACE_BEFORE) {
        out.space_before_pt = v as f32 / EMU_PER_POINT;
    }
    if let Some(v) = items::find_int(record, PROP_SPACE_AFTER) {
        out.space_after_pt = v as f32 / EMU_PER_POINT;
    }
    if let Some(v) = items::find_int(record, PROP_LINE_SPACING) {
        let amount = (v >> 3) as f32;
        out.line = match v & 7 {
            RULE_MULTIPLE => LineSpacing::Multiple(amount / EMU_PER_LINE),
            RULE_EXACT => LineSpacing::Exactly(amount / EMU_PER_POINT),
            _ => LineSpacing::Unknown(v as u32),
        };
    }
    out
}

/// The list descriptor and indents of a paragraph record (spec 4R.1). The list exists
/// exactly when `0x257` does; leftover items (`0x202`, `0x206`, `0x258` after removing a
/// list) are ignored then.
fn list_and_indents(record: &[Item], fonts: &[String]) -> (Option<ListInfo>, ParaIndents) {
    let pt = |id| items::find_int(record, id).map(|v| v as u32 as f32 / EMU_PER_POINT);
    let indents = ParaIndents {
        left_pt: pt(PROP_LEFT_INDENT).unwrap_or(0.0),
        first_line_pt: items::find_int(record, PROP_FIRST_INDENT)
            .map_or(0.0, |v| v as u32 as i32 as f32 / EMU_PER_POINT),
        right_pt: pt(PROP_RIGHT_INDENT).unwrap_or(0.0),
    };
    let list = match items::find(record, PROP_LIST).map(|i| &i.value) {
        Some(Value::Record(sub)) => {
            let kind = ListType::from_code(items::find_int(sub, LIST_TYPE).unwrap_or(0) as u32);
            let bullet = kind.is_bullet();
            Some(ListInfo {
                kind,
                bullet_char: items::find_int(sub, LIST_BULLET_CHAR).unwrap_or(0) as u32,
                bullet_font: if bullet {
                    items::find_int(record, PROP_BULLET_FONT)
                        .and_then(|i| fonts.get(i as usize).cloned())
                        .unwrap_or_default()
                } else {
                    String::new()
                },
                bullet_size_pt: if bullet { pt(PROP_BULLET_SIZE) } else { None },
                // Absent 0x258 = index 3 (plain) for numbering (spec 4R.3).
                separator: if bullet {
                    SEPARATOR_PLAIN
                } else {
                    items::find_int(record, PROP_LIST_SEPARATOR)
                        .map_or(SEPARATOR_PLAIN, |v| (v >> 16).min(255) as u8)
                },
                start: items::find_int(record, PROP_LIST_START).map_or(1, |v| v as u32),
                list_indent_pt: pt(PROP_LIST_INDENT).unwrap_or(DEFAULT_LIST_INDENT_PT),
            })
        }
        _ => None,
    };
    (list, indents)
}

/// PL (spec 3.8): `u32 count`, two more u32 (ignored), then `count` blocks in the
/// ordinary item encoding; item 1 (4 bytes) of each is the colour.
///
/// UNCONFIRMED: an entry without a decodable item 1 is read as black so later indexes
/// stay aligned; a block that does not decode ends the list (colours past it resolve to
/// `None`, i.e. the default).
fn palette(b: &[u8]) -> Vec<Color> {
    const HEADER: usize = 12;
    let count = u32_at(b, 0).unwrap_or(0) as usize;
    let mut out = Vec::new();
    let mut pos = HEADER;
    for _ in 0..count {
        let Some(len) = u32_at(b, pos).map(|l| l as usize).filter(|&l| l >= 4) else {
            break;
        };
        let Some(block) = b.get(..pos.saturating_add(len)) else {
            break;
        };
        let Ok(entry) = items::decode_block(block, pos) else {
            break;
        };
        out.push(
            items::find_int(&entry, 0x201).map_or(Color::Rgb(0, 0, 0), |v| Color::from_u32(v as u32)),
        );
        pos += len;
    }
    out
}

/// FONT (spec 3.7): the names, in table order.
fn font_names(b: &[u8], c: &Chunk) -> Result<Vec<String>, PubError> {
    let bad = |m: &str| PubError::format(c.offset, format!("FONT: {m}"));
    let n = u32_at(b, 4).ok_or_else(|| bad("chunk too short"))? as usize;
    n.checked_mul(4)
        .and_then(|x| x.checked_add(FONT_TABLE_ENTRIES_AT))
        .filter(|&x| x <= b.len())
        .ok_or_else(|| bad("font count does not fit in the chunk"))?;
    let mut names = Vec::with_capacity(n);
    for i in 0..n {
        let off = u32_at(b, FONT_TABLE_ENTRIES_AT + 4 * i).ok_or_else(|| bad("truncated"))?;
        let entry = (off as usize)
            .checked_add(FONT_TABLE_ENTRIES_AT)
            .ok_or_else(|| bad("entry offset overflow"))?;
        let chars = b
            .get(entry..entry.saturating_add(2))
            .map(|s| u16::from_le_bytes([s[0], s[1]]) as usize)
            .ok_or_else(|| bad("entry lies outside the chunk"))?;
        let name = b
            .get(entry + 2..entry + 2 + 2 * chars)
            .ok_or_else(|| bad("font name lies outside the chunk"))?;
        names.push(utf16le(name));
    }
    Ok(names)
}

/// Split a story into paragraphs at `\r` (kept in the paragraph) and each paragraph
/// into runs of identical formatting. A paragraph's alignment and spacing are those of its
/// first unit.
fn paragraphs(
    text: &[u16],
    fmts: &[CharFmt],
    aligns: &[Alignment],
    spacings: &[ParaSpacing],
    lists: &[(Option<ListInfo>, ParaIndents)],
    fonts: &[String],
    palette: &[Color],
) -> Vec<Paragraph> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = text[start..]
            .iter()
            .position(|&u| u == 0x0D)
            .map_or(text.len(), |p| start + p + 1);
        let mut runs: Vec<TextRun> = Vec::new();
        let mut run_start = start;
        while run_start < end {
            let f = fmts[run_start];
            let mut run_end = run_start + 1;
            while run_end < end && fmts[run_end] == f {
                run_end += 1;
            }
            runs.push(TextRun {
                text: String::from_utf16_lossy(&text[run_start..run_end]),
                bold: f.bold,
                italic: f.italic,
                size_pt: f.size_pt,
                font: fonts.get(f.font).cloned().unwrap_or_default(),
                underline: f.underline,
                script: f.script,
                color: f.color.and_then(|i| palette.get(i).copied()),
                field: f.field.then_some(Field::PageNumber),
            });
            run_start = run_end;
        }
        out.push(Paragraph {
            runs,
            alignment: aligns[start],
            spacing: spacings[start],
            list: lists[start].0.clone(),
            indents: lists[start].1,
            continued: false,
        });
        start = end;
    }
    out
}

/// Decode UTF-16LE bytes (a trailing odd byte is ignored, bad units become U+FFFD).
pub fn utf16le(bytes: &[u8]) -> String {
    String::from_utf16_lossy(&units(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Streams;
    fn quill_stream(rel: &str) -> Option<Vec<u8>> {
        let bytes = std::fs::read(kernpage_samples::find(rel)?).ok()?;
        Some(Streams::from_cfb(&bytes).unwrap().quill)
    }

    #[kernpage_samples::test]
    fn single_block_directory_in_committed_sample() {
        let q = quill_stream("gen/linked.pub").unwrap();
        let dir = directory(&q).unwrap();
        assert_eq!(dir.len(), 15);
        assert!(dir.iter().any(|c| &c.tag == b"TEXT"));
        assert!(!read(&q).unwrap().stories.is_empty());
    }

    #[test]
    fn chained_directory_blocks_poi_newsletter() {
        let Some(q) = quill_stream("external/poi/SampleNewsletter.pub") else {
            eprintln!("SampleNewsletter.pub absent; skipping");
            return;
        };
        let dir = directory(&q).unwrap();
        assert_eq!(dir.len(), 46);
        let strs = dir.iter().find(|c| &c.tag == b"STRS").expect("STRS");
        let text = dir.iter().find(|c| &c.tag == b"TEXT").expect("TEXT");
        let lengths = story_lengths(chunk_bytes(&q, strs).unwrap(), strs, text.len / 2).unwrap();
        assert_eq!(lengths.len(), 44);
        assert_eq!(lengths.iter().sum::<usize>(), text.len / 2);
        assert_eq!(read(&q).unwrap().stories.len(), 44);
    }

    #[test]
    fn looping_or_inconsistent_directory_is_an_error() {
        let mut q = quill_stream("gen/linked.pub").unwrap();
        // Point the first block's next offset back at itself.
        q[0x1C..0x20].copy_from_slice(&0x18u32.to_le_bytes());
        assert!(directory(&q).is_err());
        // Next offset out of range.
        q[0x1C..0x20].copy_from_slice(&0x7FFF_FFF0u32.to_le_bytes());
        assert!(directory(&q).is_err());
        // Total disagreeing with the header count.
        let mut q = quill_stream("gen/linked.pub").unwrap();
        q[0x0C] = 99;
        assert!(directory(&q).is_err());
        // Entry count overrunning the stream.
        let mut q = quill_stream("gen/linked.pub").unwrap();
        q[0x1A] = 0xFF;
        q[0x1B] = 0xFF;
        assert!(directory(&q).is_err());
    }
}
