//! Quill stream (`Quill/QuillSub/CONTENTS`) rewriting for a changed story text. Written from
//! spec sections 3.1-3.6, 3.9 and 4E only.

use crate::{Result, WriteError};

const TEXT_OFFSET: usize = 0x200;
const PAGE: usize = 512;
const HEADER_END_OFFSET: usize = 0x14;
const DIR_SPAN_OFFSET: usize = 0x10;

fn bad(msg: impl Into<String>) -> WriteError {
    WriteError::Quill(msg.into())
}

fn u16_at(d: &[u8], o: usize) -> Result<u16> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| bad(format!("truncated at {o:#x}")))
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| bad(format!("truncated at {o:#x}")))
}

fn put_u32(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn align(v: usize) -> usize {
    v.div_ceil(PAGE) * PAGE
}

/// One directory entry (spec 3.2) and where it sits in the stream.
#[derive(Debug, Clone)]
struct Chunk {
    tag: [u8; 4],
    instance: u16,
    /// Stream offset of the 24-byte entry.
    entry_pos: usize,
    offset: usize,
    len: usize,
}

/// Directory block (spec 3.1): stream offset of its 8-byte header and its capacity.
#[derive(Debug, Clone)]
struct Block {
    offset: usize,
    capacity: usize,
}

/// A run of formatting: absolute stream end position and its property record bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    end: usize,
    record: Vec<u8>,
}

struct Directory {
    blocks: Vec<Block>,
    chunks: Vec<Chunk>,
}

fn read_directory(d: &[u8]) -> Result<Directory> {
    if d.get(0..8) != Some(b"CHNKINK ") {
        return Err(bad("stream does not start with `CHNKINK `"));
    }
    let total = usize::from(u16_at(d, 0x0C)?);
    let (mut blocks, mut chunks) = (Vec::new(), Vec::new());
    let mut off = 0x18usize;
    loop {
        let capacity = usize::from(u16_at(d, off)?);
        let n = usize::from(u16_at(d, off + 2)?);
        let next = u32_at(d, off + 4)?;
        blocks.push(Block {
            offset: off,
            capacity,
        });
        for i in 0..n {
            let e = off + 8 + 24 * i;
            let tag = d
                .get(e + 2..e + 6)
                .ok_or_else(|| bad("truncated directory entry"))?;
            chunks.push(Chunk {
                tag: [tag[0], tag[1], tag[2], tag[3]],
                instance: u16_at(d, e + 6)?,
                entry_pos: e,
                offset: u32_at(d, e + 16)? as usize,
                len: u32_at(d, e + 20)? as usize,
            });
        }
        if next == 0xFFFF_FFFF {
            break;
        }
        if blocks.len() > 64 {
            return Err(bad("directory chain too long"));
        }
        off = next as usize;
    }
    if chunks.len() != total {
        return Err(bad(format!(
            "directory holds {} entries but the header says {total}",
            chunks.len()
        )));
    }
    Ok(Directory { blocks, chunks })
}

fn find<'a>(dir: &'a Directory, tag: &[u8; 4], instance: u16) -> Option<&'a Chunk> {
    dir.chunks
        .iter()
        .find(|c| &c.tag == tag && c.instance == instance)
}

fn data<'a>(d: &'a [u8], c: &Chunk) -> Result<&'a [u8]> {
    d.get(c.offset..c.offset + c.len)
        .ok_or_else(|| bad(format!("chunk {} runs past the stream", tag_str(&c.tag))))
}

fn tag_str(t: &[u8; 4]) -> String {
    String::from_utf8_lossy(t).into_owned()
}

/// Pages of one FDP type in instance order -> runs with absolute end positions.
fn read_runs(d: &[u8], dir: &Directory, tag: &[u8; 4]) -> Result<(Vec<Run>, usize)> {
    let mut pages: Vec<&Chunk> = dir.chunks.iter().filter(|c| &c.tag == tag).collect();
    pages.sort_by_key(|c| c.instance);
    let mut runs = Vec::new();
    for c in &pages {
        let p = data(d, c)?;
        if p.len() != PAGE {
            return Err(bad(format!("{} page is not 512 bytes", tag_str(tag))));
        }
        let n = usize::from(u16_at(p, 0)?);
        if 8 + 6 * n > PAGE {
            return Err(bad("run count does not fit the page"));
        }
        for i in 0..n {
            let end = u32_at(p, 8 + 4 * i)? as usize;
            let off = usize::from(u16_at(p, 8 + 4 * n + 2 * i)?);
            let len = u32_at(p, off)? as usize;
            let record = p
                .get(off..off + len)
                .ok_or_else(|| bad("property record runs past the page"))?
                .to_vec();
            runs.push(Run { end, record });
        }
    }
    Ok((runs, pages.len()))
}

/// Pack runs into 512-byte pages: header, run ends, record offsets, and records allocated
/// downward from the page end in order of first use (as seen in every sample).
fn pack_pages(runs: &[Run]) -> Result<Vec<[u8; PAGE]>> {
    let mut pages = Vec::new();
    let mut i = 0;
    while i < runs.len() {
        let mut chosen: Vec<usize> = Vec::new();
        let mut records: Vec<&[u8]> = Vec::new();
        while i < runs.len() {
            let r = &runs[i];
            let new_rec = !records.contains(&r.record.as_slice());
            let rec_total: usize = records.iter().map(|x| x.len()).sum::<usize>()
                + if new_rec { r.record.len() } else { 0 };
            if 8 + 6 * (chosen.len() + 1) + rec_total > PAGE {
                break;
            }
            if new_rec {
                records.push(&r.record);
            }
            chosen.push(i);
            i += 1;
        }
        if chosen.is_empty() {
            return Err(bad("a property record is too large for a page"));
        }
        let n = chosen.len();
        let mut page = [0u8; PAGE];
        page[0..2].copy_from_slice(&(n as u16).to_le_bytes());
        page[2..4].copy_from_slice(&1u16.to_le_bytes());
        let mut rec_off: Vec<usize> = Vec::new();
        let mut top = PAGE;
        for rec in &records {
            top -= rec.len();
            page[top..top + rec.len()].copy_from_slice(rec);
            rec_off.push(top);
        }
        for (k, &ri) in chosen.iter().enumerate() {
            let r = &runs[ri];
            page[8 + 4 * k..12 + 4 * k].copy_from_slice(&(r.end as u32).to_le_bytes());
            let idx = records
                .iter()
                .position(|x| *x == r.record.as_slice())
                .expect("record was registered");
            page[8 + 4 * n + 2 * k..10 + 4 * n + 2 * k]
                .copy_from_slice(&(rec_off[idx] as u16).to_le_bytes());
        }
        pages.push(page);
    }
    Ok(pages)
}

/// Replace the byte range `[s, e)` (old story) by `new_runs`, shifting later runs.
fn splice_runs(runs: &[Run], s: usize, e: usize, delta: isize, new_runs: Vec<Run>) -> Vec<Run> {
    let mut out = Vec::new();
    let mut pending = Some(new_runs);
    let mut start = TEXT_OFFSET;
    for r in runs {
        if start < s {
            out.push(Run {
                end: r.end.min(s),
                record: r.record.clone(),
            });
        }
        if r.end > e {
            if let Some(n) = pending.take() {
                out.extend(n);
            }
            out.push(Run {
                end: (r.end as isize + delta) as usize,
                record: r.record.clone(),
            });
        }
        start = r.end;
    }
    if let Some(n) = pending.take() {
        out.extend(n);
    }
    out
}

/// Split user text into paragraphs (no terminators). A trailing newline just terminates the
/// last paragraph.
pub(crate) fn split_paragraphs(text: &str) -> Vec<String> {
    let norm = text.replace("\r\n", "\n").replace('\r', "\n");
    let body = norm.strip_suffix('\n').unwrap_or(&norm);
    body.split('\n').map(str::to_owned).collect()
}

/// Character offsets are UTF-16 code units within the story; record indices point into
/// `char_records` / `para_records` (the distinct property records the story uses).
#[derive(Debug, Clone, Default)]
pub(crate) struct StoryRecs {
    pub text: Vec<u16>,
    pub char_records: Vec<Vec<u8>>,
    pub para_records: Vec<Vec<u8>>,
    /// `(start, end, record)` of each character run, clipped to the story.
    pub runs: Vec<(usize, usize, usize)>,
    /// `(start, end, record)` of each paragraph run, clipped to the story.
    pub paras: Vec<(usize, usize, usize)>,
}

/// A new paragraph: runs of `(text units, char record index)` plus a paragraph record index.
#[derive(Debug, Clone)]
pub(crate) struct QPara {
    pub runs: Vec<(Vec<u16>, usize)>,
    pub para: usize,
}

struct StoryLoc {
    s: usize,
    e: usize,
    strs: Vec<u8>,
}

fn locate(q: &[u8], dir: &Directory, story_index: usize) -> Result<StoryLoc> {
    let text = find(dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let strs = find(dir, b"STRS", 0).ok_or_else(|| bad("no STRS chunk"))?;
    if text.offset != TEXT_OFFSET {
        return Err(bad(format!(
            "TEXT is at {:#x}, expected 0x200",
            text.offset
        )));
    }
    let strs_data = data(q, strs)?.to_vec();
    let count = u32_at(&strs_data, 0)? as usize;
    if story_index >= count {
        return Err(WriteError::NoSuchStory {
            index: story_index,
            count,
        });
    }
    let lens: Vec<usize> = (0..count)
        .map(|i| u32_at(&strs_data, 12 + 4 * i).map(|v| v as usize))
        .collect::<Result<_>>()?;
    if lens.iter().sum::<usize>() * 2 != text.len {
        return Err(bad("STRS lengths do not add up to the TEXT length"));
    }
    let s = TEXT_OFFSET + 2 * lens[..story_index].iter().sum::<usize>();
    let e = s + 2 * lens[story_index];
    Ok(StoryLoc {
        s,
        e,
        strs: strs_data,
    })
}

type Clipped = (Vec<(usize, usize, usize)>, Vec<Vec<u8>>);

fn clip_runs(runs: &[Run], s: usize, e: usize) -> Clipped {
    let mut records: Vec<Vec<u8>> = Vec::new();
    let mut out = Vec::new();
    let mut start = TEXT_OFFSET;
    for r in runs {
        let (a, b) = (start.max(s), r.end.min(e));
        if a < b {
            let idx = records
                .iter()
                .position(|x| *x == r.record)
                .unwrap_or_else(|| {
                    records.push(r.record.clone());
                    records.len() - 1
                });
            out.push(((a - s) / 2, (b - s) / 2, idx));
        }
        start = r.end;
    }
    (out, records)
}

/// The story's text and the distinct FDPC/FDPP records of its runs.
pub(crate) fn story_records(q: &[u8], story_index: usize) -> Result<StoryRecs> {
    let dir = read_directory(q)?;
    let loc = locate(q, &dir, story_index)?;
    let text = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let tdata = data(q, text)?;
    let units = tdata[loc.s - TEXT_OFFSET..loc.e - TEXT_OFFSET]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    let (cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns, _) = read_runs(q, &dir, b"FDPP")?;
    let (runs, char_records) = clip_runs(&cruns, loc.s, loc.e);
    let (paras, para_records) = clip_runs(&pruns, loc.s, loc.e);
    Ok(StoryRecs {
        text: units,
        char_records,
        para_records,
        runs,
        paras,
    })
}

fn bte(
    old: &[u8],
    runs: &[Run],
    pages_per_run: &[usize],
    offsets: &[usize],
    text_end: usize,
) -> Result<Vec<u8>> {
    let n = offsets.len();
    let mut out = Vec::with_capacity(12 + 4 * (2 * n + 1));
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(old.get(4..12).ok_or_else(|| bad("BTE chunk too short"))?);
    out.extend_from_slice(&0u32.to_le_bytes());
    // positions: 0, then the end of the last run of each page but the last, then text end
    let mut idx = 0;
    for &count in pages_per_run.iter().take(n.saturating_sub(1)) {
        idx += count;
        out.extend_from_slice(&(runs[idx - 1].end as u32).to_le_bytes());
    }
    out.extend_from_slice(&(text_end as u32).to_le_bytes());
    for &o in offsets {
        out.extend_from_slice(&(o as u32).to_le_bytes());
    }
    Ok(out)
}

/// Rebuild the Quill stream with story `story_index` replaced by `new_paras`.
pub(crate) fn set_story_runs(
    q: &[u8],
    story_index: usize,
    new_paras: &[QPara],
    fields: Option<&[usize]>,
) -> Result<Vec<u8>> {
    let dir = read_directory(q)?;
    let text = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let loc = locate(q, &dir, story_index)?;
    let (s, e) = (loc.s, loc.e);
    let strs_data = loc.strs;
    let recs = story_records(q, story_index)?;

    let (cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns, _) = read_runs(q, &dir, b"FDPP")?;

    // Flatten the new paragraphs into text units, character runs and paragraph runs.
    let mut units: Vec<u16> = Vec::new();
    let mut crun_list: Vec<Run> = Vec::new();
    let mut prun_list: Vec<Run> = Vec::new();
    for p in new_paras {
        let prec = recs
            .para_records
            .get(p.para)
            .ok_or_else(|| bad("paragraph record index out of range"))?;
        let nruns = p.runs.len();
        if nruns == 0 {
            return Err(bad("a paragraph needs at least one run"));
        }
        for (ri, (t, c)) in p.runs.iter().enumerate() {
            let crec = recs
                .char_records
                .get(*c)
                .ok_or_else(|| bad("character record index out of range"))?;
            let last = ri + 1 == nruns;
            let cr_at = t.iter().position(|&u| u == 0x0D);
            if cr_at.is_some_and(|k| !(last && k + 1 == t.len())) {
                return Err(bad("a paragraph break inside a run's text"));
            }
            let mut t = t.clone();
            if last && cr_at.is_none() {
                t.push(0x0D);
            }
            if t.is_empty() {
                continue;
            }
            units.extend_from_slice(&t);
            let end = s + 2 * units.len();
            match crun_list.last_mut() {
                Some(prev) if prev.record == *crec => prev.end = end,
                _ => crun_list.push(Run {
                    end,
                    record: crec.clone(),
                }),
            }
        }
        prun_list.push(Run {
            end: s + 2 * units.len(),
            record: prec.clone(),
        });
    }
    if let Some(f) = fields {
        crun_list = isolate_field_runs(crun_list, &recs, &units, s, f)?;
    }
    let new_chars = units.len();
    let new_len = new_chars * 2;
    let delta = new_len as isize - (e - s) as isize;
    let mut new_text_bytes = Vec::with_capacity(text.len);
    let old_text = data(q, text)?;
    new_text_bytes.extend_from_slice(&old_text[..s - TEXT_OFFSET]);
    for u in &units {
        new_text_bytes.extend_from_slice(&u.to_le_bytes());
    }
    new_text_bytes.extend_from_slice(&old_text[e - TEXT_OFFSET..]);

    let new_cruns = splice_runs(&cruns, s, e, delta, crun_list);
    let new_pruns = splice_runs(&pruns, s, e, delta, prun_list);
    // STRS: only the one length changes.
    let mut new_strs = strs_data;
    put_u32(&mut new_strs, 12 + 4 * story_index, (new_chars) as u32);
    let edit = DirEdit {
        fields: fields.map(|f| FieldEdit {
            start: (s - TEXT_OFFSET) / 2,
            old_len: (e - s) / 2,
            cps: f.to_vec(),
        }),
        ..DirEdit::default()
    };
    rebuild_with(
        q,
        &dir,
        new_text_bytes,
        new_strs,
        &new_cruns,
        &new_pruns,
        None,
        None,
        &[],
        &edit,
    )
}

/// Character-format items only a page-number field character carries (spec 4T.3, mark ~):
/// 0x200 (value 5) and 0x222 (value 0xFFFFFFFF).
const FIELD_ONLY_ITEMS: [u16; 2] = [0x200, 0x222];

fn has_field_marks(rec: &[u8]) -> bool {
    split_items(rec).is_ok_and(|it| it.iter().any(|i| i.id == FIELD_ONLY_ITEMS[0]))
}

/// The record without the field-only items (what Publisher gives text next to a field).
fn without_field_marks(rec: &[u8]) -> Result<Vec<u8>> {
    let mut items = split_items(rec)?;
    items.retain(|i| !FIELD_ONLY_ITEMS.contains(&i.id));
    Ok(join_items(&items))
}

/// Give every field character (story offsets `fields`) its own run with the field's record
/// and take the field-only items off every other character's record, so the marks never
/// spread onto neighbouring text. A paragraph mark that carried the marks gets the story's
/// original paragraph-mark record.
fn isolate_field_runs(
    runs: Vec<Run>,
    recs: &StoryRecs,
    units: &[u16],
    s: usize,
    fields: &[usize],
) -> Result<Vec<Run>> {
    let mut per: Vec<Vec<u8>> = Vec::with_capacity(units.len());
    let mut at = 0;
    for r in &runs {
        let end = (r.end - s) / 2;
        per.extend(std::iter::repeat_n(r.record.clone(), end.saturating_sub(at)));
        at = end;
    }
    let field_rec = fields
        .iter()
        .filter_map(|&c| per.get(c))
        .find(|r| has_field_marks(r))
        .or_else(|| recs.char_records.iter().find(|r| has_field_marks(r)))
        .cloned();
    let Some(field_rec) = field_rec else {
        return Ok(runs);
    };
    let old_cr = recs
        .text
        .iter()
        .rposition(|&u| u == 0x0D)
        .and_then(|cp| recs.runs.iter().find(|&&(a, b, _)| a <= cp && cp < b))
        .and_then(|&(_, _, i)| recs.char_records.get(i))
        .filter(|r| !has_field_marks(r));
    let mut out: Vec<Run> = Vec::new();
    for (i, rec) in per.iter().enumerate() {
        let is_field = fields.contains(&i);
        let rec = if is_field {
            field_rec.clone()
        } else if !has_field_marks(rec) {
            rec.clone()
        } else if units[i] == 0x0D && old_cr.is_some() {
            old_cr.cloned().unwrap_or_default()
        } else {
            without_field_marks(rec)?
        };
        let end = s + 2 * (i + 1);
        match out.last_mut() {
            Some(prev) if !is_field && prev.record == rec && !fields.contains(&(i - 1)) => {
                prev.end = end;
            }
            _ => out.push(Run { end, record: rec }),
        }
    }
    Ok(out)
}

/// Header bytes for item `id` with the type byte `ty` (class in bits 3-7; its low 3 bits are
/// replaced by id bits 8-10).
fn ih(id: u16, ty: u8) -> [u8; 2] {
    pub_format::items::header(id, ty >> 3)
}

/// One item of a property record, kept as raw bytes (header and payload).
struct RawItem {
    id: u16,
    bytes: Vec<u8>,
}

/// Total size of the item starting at `b[0]` (spec 4.2 size table), including its header.
fn item_size(b: &[u8]) -> Result<usize> {
    let ty = *b.get(1).ok_or_else(|| bad("truncated property item"))?;
    let size = match ty >> 3 {
        0 | 1 | 15 | 21 | 22 | 28..=31 => 2,
        2 | 3 => 4,
        4 | 11 | 13 | 14 | 23 => 6,
        5 | 12 => 10,
        6 => 14,
        7 => 18,
        8 => 22,
        9 => 26,
        10 => 34,
        _ => 2 + u32_at(b, 2)? as usize,
    };
    if size > b.len() {
        return Err(bad("property item runs past its record"));
    }
    Ok(size)
}

fn split_items(rec: &[u8]) -> Result<Vec<RawItem>> {
    let total = u32_at(rec, 0)? as usize;
    if total < 4 || total > rec.len() {
        return Err(bad("bad property record length"));
    }
    let mut out = Vec::new();
    let mut pos = 4;
    while pos < total {
        let size = item_size(&rec[pos..total])?;
        out.push(RawItem {
            id: pub_format::items::split_header(rec[pos], rec[pos + 1]).0,
            bytes: rec[pos..pos + size].to_vec(),
        });
        pos += size;
    }
    Ok(out)
}

fn join_items(items: &[RawItem]) -> Vec<u8> {
    let len = 4 + items.iter().map(|i| i.bytes.len()).sum::<usize>();
    let mut out = (len as u32).to_le_bytes().to_vec();
    for i in items {
        out.extend_from_slice(&i.bytes);
    }
    out
}

fn put_item(items: &mut Vec<RawItem>, id: u16, bytes: Vec<u8>) {
    items.retain(|i| i.id != id);
    let at = items.iter().position(|i| i.id > id).unwrap_or(items.len());
    items.insert(at, RawItem { id, bytes });
}

fn drop_item(items: &mut Vec<RawItem>, id: u16) {
    items.retain(|i| i.id != id);
}

fn item_int(items: &[RawItem], id: u16) -> Option<u64> {
    let it = items.iter().find(|i| i.id == id)?;
    match it.bytes.len() {
        4 => Some(u64::from(u16::from_le_bytes([it.bytes[2], it.bytes[3]]))),
        6 => Some(u64::from(u32::from_le_bytes([
            it.bytes[2],
            it.bytes[3],
            it.bytes[4],
            it.bytes[5],
        ]))),
        _ => None,
    }
}

const ITEM_BOLD: u16 = 0x202;
const ITEM_ITALIC: u16 = 0x203;
const ITEM_SIZE: u16 = 0x20C;
const ITEM_SCRIPT: u16 = 0x20F;
const ITEM_UNDERLINE: u16 = 0x21E;
const ITEM_FONT: u16 = 0x224;
const ITEM_ALIGN: u16 = 0x204;
/// Single underline value seen in `gen/formatting.pub`.
const UNDERLINE_SINGLE: [u8; 2] = [0x01, 0x04];
/// Writing-system sub-records Publisher gives a font selection (spec 3.5).
const FONT_SUBRECORDS: [u8; 8] = [0, 1, 2, 3, 4, 5, 0x2A, 0x2C];
const DEFAULT_SIZE_PT: f32 = 10.0;

fn is_flag(items: &[RawItem], id: u16) -> bool {
    items.iter().any(|i| i.id == id && i.bytes[1] >> 3 == 1)
}

/// Index into the FONT table of a record's Latin font (0 when absent).
fn font_index(items: &[RawItem]) -> usize {
    let Some(it) = items.iter().find(|i| i.id == ITEM_FONT) else {
        return 0;
    };
    // `24 8a len(4)` then sub-records; sub-record 0 is `00 88 len(4) 00 18 idx(2)`.
    let mut pos = 6;
    while pos + 2 <= it.bytes.len() {
        let Ok(size) = item_size(&it.bytes[pos..]) else {
            return 0;
        };
        if pub_format::items::split_header(it.bytes[pos], it.bytes[pos + 1]).0 == 0 && size >= 10 {
            let v = &it.bytes[pos + 6..pos + size];
            if v.len() >= 4 {
                return usize::from(u16::from_le_bytes([v[2], v[3]]));
            }
        }
        pos += size;
    }
    0
}

fn font_item(index: usize) -> Vec<u8> {
    let idx = (index as u16).to_le_bytes();
    let mut out = ih(ITEM_FONT, 0x8A).to_vec();
    out.extend_from_slice(&((4 + 10 * FONT_SUBRECORDS.len()) as u32).to_le_bytes());
    for sub in FONT_SUBRECORDS {
        out.extend_from_slice(&[sub, 0x88, 8, 0, 0, 0, 0, 0x18, idx[0], idx[1]]);
    }
    out
}

/// The names of the FONT table, in index order (spec 3.7).
fn font_names(b: &[u8]) -> Result<Vec<String>> {
    let n = u32_at(b, 4)? as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let entry = 0x14 + u32_at(b, 0x14 + 4 * i)? as usize;
        let chars = usize::from(u16_at(b, entry)?);
        let raw = b
            .get(entry + 2..entry + 2 + 2 * chars)
            .ok_or_else(|| bad("font name runs past the FONT chunk"))?;
        let units: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        out.push(String::from_utf16_lossy(&units));
    }
    Ok(out)
}

/// The FONT chunk with `name` appended as the last entry.
fn font_table_with(b: &[u8], name: &str) -> Result<Vec<u8>> {
    let n = u32_at(b, 4)? as usize;
    let entries_at = 0x14 + 4 * n;
    if b.len() < entries_at {
        return Err(bad("FONT chunk too short"));
    }
    let old_entries = &b[entries_at..];
    let mut out = b[..0x14].to_vec();
    put_u32(&mut out, 4, (n + 1) as u32);
    for i in 0..n {
        out.extend_from_slice(&(u32_at(b, 0x14 + 4 * i)? + 4).to_le_bytes());
    }
    out.extend_from_slice(&((4 * (n + 1) + old_entries.len()) as u32).to_le_bytes());
    out.extend_from_slice(old_entries);
    let units: Vec<u16> = name.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    let len = (out.len() - 0x14) as u32;
    put_u32(&mut out, 0, len);
    Ok(out)
}

/// Colour item (spec 3.5 item 0x44) and the PL colour list (spec 3.8).
const ITEM_COLOR: u16 = 0x244;
const PL_HEADER: usize = 12;

/// Each PL entry's colour bytes R,G,B,flag (when it has a 4-byte item 1).
fn pl_entries(b: &[u8]) -> Result<Vec<Option<[u8; 4]>>> {
    let n = u32_at(b, 0)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut pos = PL_HEADER;
    for _ in 0..n {
        let len = u32_at(b, pos)? as usize;
        if len < 4 || pos + len > b.len() {
            return Err(bad("bad PL entry"));
        }
        let items = split_items(&b[pos..pos + len])?;
        let col = item_int(&items, 0x201).map(|v| (v as u32).to_le_bytes());
        out.push(col);
        pos += len;
    }
    Ok(out)
}

/// A new PL entry holding the colour `c` (spec 3.8: item 1 = R,G,B,flag; item 6 = two zero
/// bytes).
fn pl_entry(c: [u8; 4]) -> Vec<u8> {
    let mut e = vec![0, 0, 0, 0, 1, 0x22, c[0], c[1], c[2], c[3]];
    e.extend_from_slice(&[6, 0x82, 6, 0, 0, 0, 0, 0]);
    let len = e.len() as u32;
    e[..4].copy_from_slice(&len.to_le_bytes());
    e
}

/// The PL list index a record's colour item points at, if it has one.
fn color_index(items: &[RawItem]) -> Option<usize> {
    let it = items.iter().find(|i| i.id == ITEM_COLOR)?;
    if it.bytes.len() >= 12 && pub_format::items::split_header(it.bytes[6], it.bytes[7]).0 == 0x200
    {
        Some(u32::from_le_bytes([it.bytes[8], it.bytes[9], it.bytes[10], it.bytes[11]]) as usize)
    } else {
        None
    }
}

fn color_item(index: usize) -> Vec<u8> {
    let mut out = ih(ITEM_COLOR, 0x8A).to_vec();
    out.extend_from_slice(&[10, 0, 0, 0, 0, 0x22]);
    out.extend_from_slice(&(index as u32).to_le_bytes());
    out
}

const ITEM_SPACE_BEFORE: u16 = 0x212;
const ITEM_SPACE_AFTER: u16 = 0x213;
const ITEM_LINE: u16 = 0x234;
const EMU_PT: f64 = 12_700.0;
const EMU_LINE: f64 = 152_400.0;
/// Style defaults (spec 3.6): before 0, after 6 pt, 1.19 lines (rule 2).
const DEFAULT_AFTER_EMU: u64 = 76_200;
const DEFAULT_LINE: u64 = (181_356 << 3) | 2;

fn emu_item(id: u16, v: u64) -> Vec<u8> {
    let mut b = ih(id, 0x22).to_vec();
    b.extend_from_slice(&(v as u32).to_le_bytes());
    b
}

/// Set item `id` to `want` when the effective value (`default` when absent) differs.
fn set_emu(items: &mut Vec<RawItem>, id: u16, want: u64, default: u64) -> bool {
    if item_int(items, id).unwrap_or(default) == want {
        return false;
    }
    put_item(items, id, emu_item(id, want));
    true
}

/// Split `runs` at `cuts` and give each piece the record `edit` returns for it (it is given
/// the piece's `[start, end)` and the original record). Equal neighbours are merged when
/// `merge` is set; paragraph runs must not be: each one ends at a paragraph break.
fn remap_runs(
    runs: &[Run],
    cuts: &[usize],
    merge: bool,
    mut edit: impl FnMut(usize, usize, &[u8]) -> Result<Vec<u8>>,
) -> Result<Vec<Run>> {
    let mut out: Vec<Run> = Vec::new();
    let mut start = TEXT_OFFSET;
    for r in runs {
        let mut points: Vec<usize> = cuts
            .iter()
            .copied()
            .filter(|&c| c > start && c < r.end)
            .collect();
        points.sort_unstable();
        points.dedup();
        points.push(r.end);
        let mut x = start;
        for y in points {
            let record = edit(x, y, &r.record)?;
            match out.last_mut() {
                Some(prev) if merge && prev.record == record => prev.end = y,
                _ => out.push(Run { end: y, record }),
            }
            x = y;
        }
        start = r.end;
    }
    Ok(out)
}

/// Make the character and paragraph formatting of story `story_index` match the given
/// spans (offsets in UTF-16 units of the story). Only properties that differ are edited;
/// everything else in a record is kept byte for byte. The text does not change.
pub(crate) fn set_story_format(
    q: &[u8],
    story_index: usize,
    chars: &[crate::CharSpan],
    paras: &[crate::ParaSpan],
) -> Result<Vec<u8>> {
    let dir = read_directory(q)?;
    let loc = locate(q, &dir, story_index)?;
    let (s, e) = (loc.s, loc.e);
    let text = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let (cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns, _) = read_runs(q, &dir, b"FDPP")?;
    let font_chunk = find(&dir, b"FONT", 0).ok_or_else(|| bad("no FONT chunk"))?;
    let mut font_bytes = data(q, font_chunk)?.to_vec();
    let mut names = font_names(&font_bytes)?;
    let mut font_changed = false;
    let pl_chunk = find(&dir, b"PL  ", 0);
    let mut pl_bytes = match pl_chunk {
        Some(c) => data(q, c)?.to_vec(),
        None => Vec::new(),
    };
    let mut pl_changed = false;

    let abs = |off: usize| s + 2 * off;
    let mut cuts: Vec<usize> = Vec::new();
    for c in chars {
        cuts.push(abs(c.start));
        cuts.push(abs(c.end));
    }
    let char_runs = remap_runs(&cruns, &cuts, true, |x, y, rec| {
        if x < s || y > e {
            return Ok(rec.to_vec());
        }
        let Some(span) = chars.iter().find(|c| abs(c.start) <= x && y <= abs(c.end)) else {
            return Ok(rec.to_vec());
        };
        let want = &span.format;
        let mut items = split_items(rec)?;
        let mut changed = false;
        if is_flag(&items, ITEM_BOLD) != want.bold {
            drop_item(&mut items, ITEM_BOLD);
            if want.bold {
                put_item(&mut items, ITEM_BOLD, ih(ITEM_BOLD, 0x0A).to_vec());
            }
            changed = true;
        }
        if is_flag(&items, ITEM_ITALIC) != want.italic {
            drop_item(&mut items, ITEM_ITALIC);
            if want.italic {
                put_item(&mut items, ITEM_ITALIC, ih(ITEM_ITALIC, 0x0A).to_vec());
            }
            changed = true;
        }
        if item_int(&items, ITEM_UNDERLINE).is_some_and(|v| v != 0) != want.underline {
            drop_item(&mut items, ITEM_UNDERLINE);
            if want.underline {
                put_item(
                    &mut items,
                    ITEM_UNDERLINE,
                    [&ih(ITEM_UNDERLINE, 0x12)[..], &UNDERLINE_SINGLE[..]].concat(),
                );
            }
            changed = true;
        }
        let want_script: u64 = match want.script {
            pub_format::Script::Normal => 0,
            pub_format::Script::Superscript => 1,
            pub_format::Script::Subscript => 2,
        };
        if item_int(&items, ITEM_SCRIPT).map_or(0, |v| v & 0xFF) != want_script {
            drop_item(&mut items, ITEM_SCRIPT);
            if want_script != 0 {
                let mut b = ih(ITEM_SCRIPT, 0x12).to_vec();
                b.extend_from_slice(&(want_script as u16).to_le_bytes());
                put_item(&mut items, ITEM_SCRIPT, b);
            }
            changed = true;
        }
        let cur_size = item_int(&items, ITEM_SIZE).map_or(DEFAULT_SIZE_PT, |v| v as f32 / 12_700.0);
        if (cur_size - want.size_pt).abs() > 0.01 {
            if (want.size_pt - DEFAULT_SIZE_PT).abs() <= 0.01 {
                // The default size is the absence of the item (what Publisher writes).
                drop_item(&mut items, ITEM_SIZE);
            } else {
                let emu = (f64::from(want.size_pt) * 12_700.0).round() as u32;
                let mut b = ih(ITEM_SIZE, 0x22).to_vec();
                b.extend_from_slice(&emu.to_le_bytes());
                put_item(&mut items, ITEM_SIZE, b);
            }
            changed = true;
        }
        let cur_font = names.get(font_index(&items));
        if cur_font.map(String::as_str) != Some(want.font.as_str()) && !want.font.is_empty() {
            let index = match names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(&want.font))
            {
                Some(i) => i,
                None => {
                    font_bytes = font_table_with(&font_bytes, &want.font)?;
                    names.push(want.font.clone());
                    font_changed = true;
                    names.len() - 1
                }
            };
            if index == 0 {
                // Font table entry 0 is the default: the item is simply absent.
                drop_item(&mut items, ITEM_FONT);
            } else {
                put_item(&mut items, ITEM_FONT, font_item(index));
            }
            changed = true;
        }
        // The wanted colour as a stored colour value R, G, B, flag (a scheme colour: index,
        // 0, 0, 0x08, spec 4D).
        let want_color = match (want.scheme_color, want.color) {
            (Some(i), _) => Some([i, 0, 0, 8]),
            (None, Some([r, g, b])) => Some([r, g, b, 0]),
            (None, None) => None,
        };
        if let Some(want4) = want_color {
            let entries = if pl_bytes.is_empty() {
                Vec::new()
            } else {
                pl_entries(&pl_bytes)?
            };
            let stored = |i: usize| match entries.get(i) {
                Some(Some(c)) if c[3] == 0 || c[3] == 8 => Some(*c),
                _ => None,
            };
            let cur = match color_index(&items) {
                None => Some([0, 0, 0, 0]),
                Some(i) => stored(i),
            };
            if cur != Some(want4) {
                if want4 == [0, 0, 0, 0] {
                    drop_item(&mut items, ITEM_COLOR);
                } else {
                    let index = match (0..entries.len()).find(|&i| stored(i) == Some(want4)) {
                        Some(i) => i,
                        None => {
                            if pl_bytes.is_empty() {
                                return Err(bad("no PL chunk to add a colour to"));
                            }
                            pl_bytes.extend_from_slice(&pl_entry(want4));
                            let n = u32_at(&pl_bytes, 0)? + 1;
                            put_u32(&mut pl_bytes, 0, n);
                            pl_changed = true;
                            entries.len()
                        }
                    };
                    put_item(&mut items, ITEM_COLOR, color_item(index));
                }
                changed = true;
            }
        }
        Ok(if changed {
            join_items(&items)
        } else {
            rec.to_vec()
        })
    })?;

    let mut pcuts: Vec<usize> = Vec::new();
    for p in paras {
        pcuts.push(abs(p.start));
        pcuts.push(abs(p.end));
    }
    let para_runs = remap_runs(&pruns, &pcuts, false, |x, y, rec| {
        if x < s || y > e {
            return Ok(rec.to_vec());
        }
        let Some(span) = paras.iter().find(|p| abs(p.start) <= x && y <= abs(p.end)) else {
            return Ok(rec.to_vec());
        };
        let mut items = split_items(rec)?;
        let mut changed = false;
        let cur = match item_int(&items, ITEM_ALIGN) {
            Some(1) => crate::ParaAlign::Center,
            Some(3) => crate::ParaAlign::Right,
            Some(4) => crate::ParaAlign::Justify,
            _ => crate::ParaAlign::Left,
        };
        if cur != span.align {
            drop_item(&mut items, ITEM_ALIGN);
            let v: Option<u8> = match span.align {
                crate::ParaAlign::Left => None,
                crate::ParaAlign::Center => Some(1),
                crate::ParaAlign::Right => Some(3),
                crate::ParaAlign::Justify => Some(4),
            };
            if let Some(v) = v {
                put_item(
                    &mut items,
                    ITEM_ALIGN,
                    [&ih(ITEM_ALIGN, 0x12)[..], &[v, 0][..]].concat(),
                );
            }
            changed = true;
        }
        if let Some(sp) = span.spacing {
            let emu = |pt: f32| (f64::from(pt) * EMU_PT).round() as u64;
            changed |= set_emu(&mut items, ITEM_SPACE_BEFORE, emu(sp.space_before_pt), 0);
            changed |= set_emu(
                &mut items,
                ITEM_SPACE_AFTER,
                emu(sp.space_after_pt),
                DEFAULT_AFTER_EMU,
            );
            let packed = match sp.line {
                Some(crate::LineRule::Multiple(m)) => {
                    Some((((f64::from(m) * EMU_LINE).round() as u64) << 3) | 2)
                }
                Some(crate::LineRule::Exactly(pt)) => Some((emu(pt) << 3) | 1),
                None => None,
            };
            if let Some(v) = packed {
                changed |= set_emu(&mut items, ITEM_LINE, v, DEFAULT_LINE);
            }
        }
        Ok(if changed {
            join_items(&items)
        } else {
            rec.to_vec()
        })
    })?;

    rebuild(
        q,
        &dir,
        data(q, text)?.to_vec(),
        loc.strs,
        &char_runs,
        &para_runs,
        font_changed.then_some(font_bytes),
        pl_changed.then_some(pl_bytes),
    )
}

// ---- Lists and indents (spec 4R) ----

const ITEM_LIST: u16 = 0x257;
const ITEM_LIST_SEP: u16 = 0x258;
const ITEM_LIST_START: u16 = 0x215;
const ITEM_LIST_INDENT: u16 = 0x206;
const ITEM_BULLET_SIZE: u16 = 0x202;
const ITEM_BULLET_FONT: u16 = 0x203;
const ITEM_LEFT: u16 = 0x20D;
const ITEM_FIRST: u16 = 0x20C;
const ITEM_RIGHT: u16 = 0x20E;
const DEFAULT_LIST_INDENT_EMU: u64 = 228_600;
/// Separator index 2 (`1.`) as stored: what a cleared bullet leaves behind.
const SEP_DOT_STORED: u64 = 2 << 16;

fn emu_of(pt: f32) -> i64 {
    (f64::from(pt) * EMU_PT).round() as i64
}

/// A 4-byte item holding `v` (two's complement for negative values).
fn signed_emu_item(id: u16, v: i64) -> Vec<u8> {
    emu_item(id, u64::from(v as i32 as u32))
}

/// The list descriptor record `0x257` (type, bullet character, constant 0).
fn list_item(kind: u32, bullet: u32) -> Vec<u8> {
    let mut b = ih(ITEM_LIST, 0x88).to_vec();
    b.extend_from_slice(&22u32.to_le_bytes());
    for (id, v) in [(0x200u16, kind), (0x201, bullet), (0x202, 0)] {
        b.extend_from_slice(&ih(id, 0x22));
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// Make the list formatting and indents of the paragraphs in `spans` match (spec 4R).
/// Only the items of 4R.1 are edited, in id order; everything else in a record is kept.
/// Clearing a list mimics Publisher's residue (4R.1). The text and character formatting do
/// not change; bullet fonts missing from the FONT table are appended.
pub(crate) fn set_story_lists(
    q: &[u8],
    story_index: usize,
    spans: &[crate::ListSpan],
) -> Result<Vec<u8>> {
    use pub_format::list::{DEFAULT_BULLET_FONT, DEFAULT_BULLET_SIZE_PT, SEPARATOR_PLAIN};
    let dir = read_directory(q)?;
    let loc = locate(q, &dir, story_index)?;
    let (s, e) = (loc.s, loc.e);
    let text = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let (cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns, _) = read_runs(q, &dir, b"FDPP")?;
    let font_chunk = find(&dir, b"FONT", 0).ok_or_else(|| bad("no FONT chunk"))?;
    let mut font_bytes = data(q, font_chunk)?.to_vec();
    let mut names = font_names(&font_bytes)?;
    let mut font_changed = false;
    let abs = |off: usize| s + 2 * off;
    let mut cuts: Vec<usize> = Vec::new();
    for p in spans {
        cuts.push(abs(p.start));
        cuts.push(abs(p.end));
    }
    let para_runs = remap_runs(&pruns, &cuts, false, |x, y, rec| {
        if x < s || y > e {
            return Ok(rec.to_vec());
        }
        let Some(span) = spans.iter().find(|p| abs(p.start) <= x && y <= abs(p.end)) else {
            return Ok(rec.to_vec());
        };
        let mut items = split_items(rec)?;
        let prior_kind = items
            .iter()
            .find(|i| i.id == ITEM_LIST)
            .map(|i| u32_at(&i.bytes, 8).unwrap_or(0));
        for id in [ITEM_LEFT, ITEM_FIRST, ITEM_RIGHT] {
            drop_item(&mut items, id);
        }
        match &span.list {
            Some(l) => {
                let bullet = l.kind.is_bullet();
                for id in [
                    ITEM_LIST,
                    ITEM_LIST_SEP,
                    ITEM_LIST_START,
                    ITEM_BULLET_FONT,
                    ITEM_LIST_INDENT,
                ] {
                    drop_item(&mut items, id);
                }
                if bullet {
                    let size = l.bullet_size_pt.unwrap_or(DEFAULT_BULLET_SIZE_PT);
                    put_item(
                        &mut items,
                        ITEM_BULLET_SIZE,
                        signed_emu_item(ITEM_BULLET_SIZE, emu_of(size)),
                    );
                    let name = if l.bullet_font.is_empty() {
                        DEFAULT_BULLET_FONT
                    } else {
                        l.bullet_font.as_str()
                    };
                    let index = match names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
                        Some(i) => i,
                        None => {
                            font_bytes = font_table_with(&font_bytes, name)?;
                            names.push(name.to_owned());
                            font_changed = true;
                            names.len() - 1
                        }
                    };
                    let mut b = ih(ITEM_BULLET_FONT, 0x1A).to_vec();
                    b.extend_from_slice(&(index as u16).to_le_bytes());
                    put_item(&mut items, ITEM_BULLET_FONT, b);
                } else {
                    // Publisher keeps a leftover bullet size when a bullet becomes numbering
                    // (4R.1); it means nothing, so it is left alone.
                    if l.start != 1 {
                        put_item(
                            &mut items,
                            ITEM_LIST_START,
                            emu_item(ITEM_LIST_START, u64::from(l.start)),
                        );
                    }
                    if l.separator != SEPARATOR_PLAIN {
                        put_item(
                            &mut items,
                            ITEM_LIST_SEP,
                            emu_item(ITEM_LIST_SEP, u64::from(l.separator) << 16),
                        );
                    }
                }
                let indent = emu_of(l.list_indent_pt);
                if indent as u64 != DEFAULT_LIST_INDENT_EMU {
                    put_item(
                        &mut items,
                        ITEM_LIST_INDENT,
                        signed_emu_item(ITEM_LIST_INDENT, indent),
                    );
                }
                let bullet_char = if bullet { l.bullet_char } else { 0 };
                put_item(&mut items, ITEM_LIST, list_item(l.kind.code(), bullet_char));
            }
            None => {
                if let Some(kind) = prior_kind {
                    // Residue of Publisher's SetListType(255): list indent 0, the separator
                    // of a bullet (index 2) and the bullet size stay; list, bullet font gone.
                    drop_item(&mut items, ITEM_LIST);
                    drop_item(&mut items, ITEM_BULLET_FONT);
                    put_item(&mut items, ITEM_LIST_INDENT, emu_item(ITEM_LIST_INDENT, 0));
                    if kind == 23 && item_int(&items, ITEM_LIST_SEP).is_none() {
                        put_item(
                            &mut items,
                            ITEM_LIST_SEP,
                            emu_item(ITEM_LIST_SEP, SEP_DOT_STORED),
                        );
                    }
                }
            }
        }
        let ind = span.indents;
        for (id, pt) in [
            (ITEM_FIRST, ind.first_line_pt),
            (ITEM_LEFT, ind.left_pt),
            (ITEM_RIGHT, ind.right_pt),
        ] {
            if emu_of(pt) != 0 {
                put_item(&mut items, id, signed_emu_item(id, emu_of(pt)));
            }
        }
        let out = join_items(&items);
        Ok(if out == rec { rec.to_vec() } else { out })
    })?;
    rebuild(
        q,
        &dir,
        data(q, text)?.to_vec(),
        loc.strs,
        &cruns,
        &para_runs,
        font_changed.then_some(font_bytes),
        None,
    )
}

/// Write the stream again with new text bytes, `STRS` payload, formatting runs and (when
/// given) a new `FONT` payload; every chunk keeps its place and later offsets are shifted.
#[allow(clippy::too_many_arguments)]
fn rebuild(
    q: &[u8],
    dir: &Directory,
    new_text_bytes: Vec<u8>,
    new_strs: Vec<u8>,
    new_cruns: &[Run],
    new_pruns: &[Run],
    new_font: Option<Vec<u8>>,
    new_pl: Option<Vec<u8>>,
) -> Result<Vec<u8>> {
    rebuild_with(
        q,
        dir,
        new_text_bytes,
        new_strs,
        new_cruns,
        new_pruns,
        new_font,
        new_pl,
        &[],
        &DirEdit::default(),
    )
}

/// How the chunk directory changes besides the text engine's usual chunks: `TCD ` chunks
/// (spec 4E.1.3) are added or removed, whole chunks dropped.
#[derive(Default)]
struct DirEdit {
    tcd: TcdEdit,
    /// Chunks (any instance) that are left out.
    drop: Vec<[u8; 4]>,
    /// The exact page-number fields of the edited story (spec 4T.3), when the caller knows
    /// them; otherwise the `TOKN` table follows the text by [`adjust_tokn`]'s diff rule.
    fields: Option<FieldEdit>,
}

/// The page-number fields of one edited story: `start` and `old_len` are the story's
/// position and length (in text units) before the edit, `cps` the units of its fields
/// inside the story after it.
struct FieldEdit {
    start: usize,
    old_len: usize,
    cps: Vec<usize>,
}

#[derive(Default)]
enum TcdEdit {
    #[default]
    None,
    /// A new `TCD ` chunk with these cell ends, after the last `TCD ` (else after `FONT`).
    /// `instance` must be above every existing `TCD ` instance.
    Add { instance: u16, ends: Vec<u32> },
    /// Remove this instance; higher instances move down by one.
    Remove(u16),
}

/// [`rebuild`] that also replaces whole chunks (instance 0) by tag, e.g. `SYID`, `MCLD`,
/// and applies a [`DirEdit`].
#[allow(clippy::too_many_arguments)]
fn rebuild_with(
    q: &[u8],
    dir: &Directory,
    new_text_bytes: Vec<u8>,
    new_strs: Vec<u8>,
    new_cruns: &[Run],
    new_pruns: &[Run],
    new_font: Option<Vec<u8>>,
    new_pl: Option<Vec<u8>>,
    extra: &[([u8; 4], Vec<u8>)],
    edit: &DirEdit,
) -> Result<Vec<u8>> {
    let text_end = TEXT_OFFSET + new_text_bytes.len();
    let cpacked = pack_pages(new_cruns)?;
    let ppacked = pack_pages(new_pruns)?;

    let per_page = |pages: &[[u8; PAGE]]| -> Vec<usize> {
        pages
            .iter()
            .map(|p| usize::from(u16::from_le_bytes([p[0], p[1]])))
            .collect()
    };

    // The chunk list in directory order. FDP pages are renumbered 0..k and extra pages are
    // cloned from the last existing page's directory entry (spec 3.9, FDP page growth).
    let rank = |c: &Chunk| {
        dir.chunks
            .iter()
            .filter(|x| x.tag == c.tag && x.instance < c.instance)
            .count()
    };
    let last_of = |tag: &[u8; 4]| {
        dir.chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| &c.tag == tag)
            .max_by_key(|(_, c)| c.instance)
            .map(|(i, _)| i)
    };
    let tcd_anchor = match &edit.tcd {
        TcdEdit::Add { instance, .. } => {
            if dir
                .chunks
                .iter()
                .any(|c| &c.tag == b"TCD " && c.instance >= *instance)
            {
                return Err(bad("a TCD chunk with this instance exists"));
            }
            Some(
                dir.chunks
                    .iter()
                    .rposition(|c| &c.tag == b"TCD ")
                    .or_else(|| dir.chunks.iter().position(|c| &c.tag == b"FONT"))
                    .ok_or_else(|| bad("no FONT chunk to place a TCD chunk after"))?,
            )
        }
        _ => None,
    };
    let mut items: Vec<Item> = Vec::new();
    for (i, c) in dir.chunks.iter().enumerate() {
        let mut entry = [0u8; 24];
        entry.copy_from_slice(&q[c.entry_pos..c.entry_pos + 24]);
        let mut instance = c.instance;
        let mut dropped = edit.drop.contains(&c.tag);
        if let (b"TCD ", TcdEdit::Remove(k)) = (&c.tag, &edit.tcd) {
            if c.instance == *k {
                dropped = true;
            } else if c.instance > *k {
                instance -= 1;
                entry[6..8].copy_from_slice(&instance.to_le_bytes());
            }
        }
        let fdp = match &c.tag {
            b"FDPP" => Some(&ppacked),
            b"FDPC" => Some(&cpacked),
            _ => None,
        };
        let repl = extra
            .iter()
            .find(|(t, _)| *t == c.tag && c.instance == 0)
            .map(|(_, d)| d.clone());
        let payload = match (&c.tag, fdp) {
            _ if dropped => None,
            _ if repl.is_some() => repl,
            (_, Some(packed)) => packed.get(rank(c)).map(|p| p.to_vec()),
            (b"TEXT", _) => Some(new_text_bytes.clone()),
            (b"TOKN", _) if c.instance == 0 => {
                let old_text = find(dir, b"TEXT", 0).map(|t| data(q, t)).transpose()?;
                adjust_tokn(
                    old_text.unwrap_or_default(),
                    &new_text_bytes,
                    data(q, c)?,
                    edit.fields.as_ref(),
                )?
            }
            (b"STRS", _) if c.instance == 0 => Some(new_strs.clone()),
            (b"FONT", _) if c.instance == 0 && new_font.is_some() => new_font.clone(),
            (b"PL  ", _) if c.instance == 0 && new_pl.is_some() => new_pl.clone(),
            // BTEP/BTEC need the final page offsets; filled in below.
            _ => Some(data(q, c)?.to_vec()),
        };
        items.push(Item {
            entry,
            tag: c.tag,
            instance,
            old_offset: c.offset,
            old_len: c.len,
            payload,
        });
        if tcd_anchor == Some(i) {
            if let TcdEdit::Add { instance, ends } = &edit.tcd {
                let mut e = [0u8; 24];
                e[0] = 0x18;
                e[2..6].copy_from_slice(b"TCD ");
                e[6..8].copy_from_slice(&instance.to_le_bytes());
                e[8] = 1;
                e[12..16].copy_from_slice(b"PLC ");
                // Header: cells - 1, 0, 0xFF00; then one end per cell (spec 4E).
                let mut p = (ends.len().saturating_sub(1) as u32).to_le_bytes().to_vec();
                p.extend_from_slice(&0u32.to_le_bytes());
                p.extend_from_slice(&0xFF00u32.to_le_bytes());
                for v in ends {
                    p.extend_from_slice(&v.to_le_bytes());
                }
                items.push(Item {
                    entry: e,
                    tag: *b"TCD ",
                    instance: *instance,
                    old_offset: c.offset + c.len,
                    old_len: 0,
                    payload: Some(p),
                });
            }
        }
        if let Some(packed) = fdp {
            if last_of(&c.tag) == Some(i) {
                for (k, page) in packed.iter().enumerate().skip(rank(c) + 1) {
                    let mut e = entry;
                    e[6..8].copy_from_slice(&(k as u16).to_le_bytes());
                    items.push(Item {
                        entry: e,
                        tag: c.tag,
                        instance: k as u16,
                        old_offset: c.offset,
                        old_len: c.len,
                        payload: Some(page.to_vec()),
                    });
                }
            }
        }
    }

    // Layout: chunks in their old order, back to back (old gaps kept), FDP pages aligned.
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| items[i].old_offset); // stable: new pages follow their anchor
    let mut new_off = vec![0usize; items.len()];
    let mut cursor = TEXT_OFFSET;
    let mut prev_old_end = TEXT_OFFSET;
    let page_offsets_of = |items: &[Item], new_off: &[usize], tag: &[u8; 4]| -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter(|(_, it)| &it.tag == tag && it.payload.is_some())
            .map(|(i, _)| new_off[i])
            .collect()
    };
    // BTEP/BTEC sizes depend only on the page counts, so lay out with placeholders first.
    let (n_pp, n_pc) = (ppacked.len(), cpacked.len());
    for it in items.iter_mut() {
        if it.instance == 0 && it.payload.is_some() && (&it.tag == b"BTEP" || &it.tag == b"BTEC") {
            let n = if &it.tag == b"BTEP" { n_pp } else { n_pc };
            it.payload = Some(vec![0u8; 12 + 4 * (2 * n + 1)]);
        }
    }
    for &i in &order {
        let it = &items[i];
        let is_fdp = &it.tag == b"FDPP" || &it.tag == b"FDPC";
        let start = match &it.payload {
            None => {
                prev_old_end = it.old_offset + it.old_len;
                continue;
            }
            Some(_) if is_fdp => align(cursor),
            Some(_) => cursor + it.old_offset.saturating_sub(prev_old_end),
        };
        new_off[i] = start;
        cursor = start + it.payload.as_ref().map_or(0, Vec::len);
        prev_old_end = it.old_offset + it.old_len;
    }
    let new_btep = bte(
        data(
            q,
            find(dir, b"BTEP", 0).ok_or_else(|| bad("no BTEP chunk"))?,
        )?,
        new_pruns,
        &per_page(&ppacked),
        &page_offsets_of(&items, &new_off, b"FDPP"),
        text_end,
    )?;
    let new_btec = bte(
        data(
            q,
            find(dir, b"BTEC", 0).ok_or_else(|| bad("no BTEC chunk"))?,
        )?,
        new_cruns,
        &per_page(&cpacked),
        &page_offsets_of(&items, &new_off, b"FDPC"),
        text_end,
    )?;
    for it in items
        .iter_mut()
        .filter(|it| it.instance == 0 && it.payload.is_some())
    {
        match &it.tag {
            b"BTEP" => it.payload = Some(new_btep.clone()),
            b"BTEC" => it.payload = Some(new_btec.clone()),
            _ => {}
        }
    }

    // Directory blocks: keep each block's old entry count as its capacity (at least 19 for
    // the first block, which must end before TEXT, and 20 for later ones), spill into new
    // blocks placed after the last chunk, each 512-aligned and 512 bytes (spec 3.9).
    let live: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].payload.is_some())
        .collect();
    let caps: Vec<usize> = dir
        .blocks
        .iter()
        .enumerate()
        .map(|(k, b)| {
            let n = usize::from(u16::from_le_bytes([q[b.offset + 2], q[b.offset + 3]]));
            n.max(if k == 0 { 19 } else { 20 })
        })
        .collect();
    // Blocks: how many entries each holds.
    let mut counts: Vec<usize> = Vec::new();
    let mut left = live.len();
    let mut k = 0;
    while left > 0 {
        let cap = if k < caps.len() { caps[k] } else { 20 };
        let take = cap.min(left);
        counts.push(take);
        left -= take;
        k += 1;
    }
    if counts.is_empty() {
        counts.push(0);
    }
    let chunk_end = cursor;
    let tail = align(chunk_end);
    let n_blocks = counts.len();
    let block_off = |k: usize| if k == 0 { 0x18 } else { tail + PAGE * (k - 1) };
    let total = tail + PAGE * (n_blocks - 1);

    let mut out = vec![0u8; total.max(TEXT_OFFSET)];
    out[..0x18].copy_from_slice(
        q.get(..0x18)
            .ok_or_else(|| bad("stream shorter than 0x18"))?,
    );
    let mut li = 0;
    for (k, &cnt) in counts.iter().enumerate() {
        let bo = block_off(k);
        let cap_bytes = if k == 0 { dir.blocks[0].capacity } else { 504 };
        out[bo..bo + 2].copy_from_slice(&(cap_bytes as u16).to_le_bytes());
        out[bo + 2..bo + 4].copy_from_slice(&(cnt as u16).to_le_bytes());
        let next = if k + 1 < n_blocks {
            block_off(k + 1) as u32
        } else {
            0xFFFF_FFFF
        };
        put_u32(&mut out, bo + 4, next);
        for j in 0..cnt {
            let i = live[li];
            li += 1;
            let it = &items[i];
            let payload = it.payload.as_ref().expect("live item");
            let mut e = it.entry;
            e[16..20].copy_from_slice(&(new_off[i] as u32).to_le_bytes());
            e[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            let ep = bo + 8 + 24 * j;
            out[ep..ep + 24].copy_from_slice(&e);
            let end = new_off[i] + payload.len();
            if end > out.len() {
                out.resize(align(end), 0);
            }
            out[new_off[i]..end].copy_from_slice(payload);
        }
    }
    out[0x0C..0x0E].copy_from_slice(&(live.len() as u16).to_le_bytes());
    let final_len = align(out.len().max(chunk_end)).max(total);
    out.resize(final_len, 0);
    put_u32(&mut out, HEADER_END_OFFSET, final_len as u32);
    // Header u32 at 0x10 = 512 x number of directory blocks (0x200 for one block, 0x600 for
    // three as in the POI newsletter). Publisher rejects a multi-block file that keeps 0x200
    // (found by experiment: two blocks open with 0x400 or more, three need 0x600 or more).
    put_u32(&mut out, DIR_SPAN_OFFSET, (PAGE * n_blocks) as u32);
    Ok(out)
}

/// The `TOKN` field table (spec 4T.3) for a changed `TEXT` chunk. Each field cp is a position in
/// the whole text. With `exact` (the fields of the edited story as the caller knows them)
/// cps before the story stay, cps after it move with the length change and the story's
/// own are replaced. Without it the old and new text are compared by their common prefix and
/// suffix (the rule the editor uses to carry a field through one edit): a field in the
/// prefix stays, one in the suffix moves with it, one in the changed middle is gone. The
/// last cp is the text length. `None` = no field left, the chunk is dropped (as in a story
/// without fields). The number of fields may only change to zero (the layout behind the cps
/// of more than one field is not known).
fn adjust_tokn(
    old_text: &[u8],
    new_text: &[u8],
    tokn: &[u8],
    exact: Option<&FieldEdit>,
) -> Result<Option<Vec<u8>>> {
    if old_text == new_text && exact.is_none() {
        return Ok(Some(tokn.to_vec()));
    }
    let units = |b: &[u8]| -> Vec<u16> {
        b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
    };
    let (old, new) = (units(old_text), units(new_text));
    let n = u32_at(tokn, 0)? as usize;
    if tokn.len() < 12 + 4 * (n + 1) {
        return Err(bad("TOKN is shorter than its cps"));
    }
    let delta = new.len() as i64 - old.len() as i64;
    let old_cps: Vec<usize> = (0..n)
        .map(|i| u32_at(tokn, 12 + 4 * i).map(|v| v as usize))
        .collect::<Result<_>>()?;
    let mut cps: Vec<usize> = Vec::new();
    match exact {
        Some(e) => {
            let after = e.start + e.old_len;
            for &cp in &old_cps {
                if cp < e.start {
                    cps.push(cp);
                } else if cp >= after {
                    cps.push(usize::try_from(cp as i64 + delta).map_err(|_| bad("TOKN cp"))?);
                }
            }
            cps.extend(e.cps.iter().map(|c| e.start + c));
            cps.sort_unstable();
        }
        None => {
            let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
            let room = old.len().min(new.len()) - prefix;
            let suffix = old
                .iter()
                .rev()
                .zip(new.iter().rev())
                .take(room)
                .take_while(|(a, b)| a == b)
                .count();
            for &cp in &old_cps {
                if cp < prefix {
                    cps.push(cp);
                } else if cp >= old.len() - suffix {
                    cps.extend(usize::try_from(cp as i64 + delta).ok());
                }
            }
        }
    }
    cps.retain(|&m| new.get(m) == Some(&0x23));
    if cps.is_empty() {
        return Ok(None);
    }
    if cps.len() != n {
        return Err(bad(
            "an edit changes the number of page-number fields of a file that has some",
        ));
    }
    let mut out = tokn.to_vec();
    for (i, cp) in cps.iter().enumerate() {
        put_u32(&mut out, 12 + 4 * i, *cp as u32);
    }
    let end = u32_at(tokn, 12 + 4 * n)? as i64;
    let end = if end == old.len() as i64 { new.len() as i64 } else { end + delta };
    put_u32(&mut out, 12 + 4 * n, end.max(0) as u32);
    Ok(Some(out))
}

struct Item {
    entry: [u8; 24],
    tag: [u8; 4],
    instance: u16,
    /// Old stream offset/length: the layout key (new items inherit their predecessor's).
    old_offset: usize,
    old_len: usize,
    /// `None` = dropped (formatting shrank by a page).
    payload: Option<Vec<u8>>,
}

/// The `MCLD` header: (record-id high-water mark, record ids), one id per story. A fresh file
/// has `n, n, 1..n`; after Publisher inserted rows or columns of a table the mark is higher
/// (`02 00 00 00 | 01 00 00 00 | 02 00 00 00` for one story, spec 4E.1.5).
fn mcld_header(mcld: &[u8], stories: usize) -> Result<(u32, Vec<u32>)> {
    let high = u32_at(mcld, 0)?;
    let n = u32_at(mcld, 4)? as usize;
    if n != stories {
        return Err(bad("MCLD does not hold one record per story"));
    }
    let ids = (0..n)
        .map(|i| u32_at(mcld, 8 + 4 * i))
        .collect::<Result<Vec<_>>>()?;
    Ok((high, ids))
}

/// Absolute stream offsets and byte lengths of the integer items `ids` of one `MCLD` frame
/// (`None` unless every one is present).
pub(crate) fn mcld_frame_slots<const N: usize>(
    q: &[u8],
    group: usize,
    frame: usize,
    frames_expected: usize,
    ids: [u16; N],
) -> Result<Option<[(usize, usize); N]>> {
    let dir = read_directory(q)?;
    let Some(c) = find(&dir, b"MCLD", 0) else {
        return Ok(None);
    };
    let b = data(q, c)?;
    let groups = u32_at(b, 4)? as usize;
    let mut pos = 8 + 4 * groups;
    for g in 0..groups {
        pos += u32_at(b, pos)? as usize;
        let count = u32_at(b, pos)? as usize;
        pos += 4;
        if g == group && count != frames_expected {
            return Ok(None);
        }
        for f in 0..count {
            let len = u32_at(b, pos)? as usize;
            if g == group && f == frame {
                let found: Vec<(usize, usize)> = ids
                    .iter()
                    .filter_map(|&id| {
                        pub_format::items::locate_int(b, pos, id)
                            .ok()
                            .flatten()
                            .map(|s| (c.offset + s.offset, s.len))
                    })
                    .collect();
                return Ok(<[(usize, usize); N]>::try_from(found).ok());
            }
            pos += len;
        }
    }
    Ok(None)
}

/// Cell-end positions stored in a `TCD ` chunk (spec 4E): 12-byte header, one u32 per cell.
fn tcd_ends(b: &[u8]) -> Option<Vec<u32>> {
    let n = (u32_at(b, 0).ok()? as usize).checked_add(1)?;
    if n.checked_mul(4)?.checked_add(12)? != b.len() {
        return None;
    }
    (0..n).map(|i| u32_at(b, 12 + 4 * i).ok()).collect()
}

/// Whether `ends` splits `text` into cells: strictly increasing, every end but the last at a
/// `\r`, the last end = the story length.
fn ends_fit(text: &[u16], ends: &[u32]) -> bool {
    let Some((&last, rest)) = ends.split_last() else {
        return false;
    };
    let mut prev: i64 = -1;
    for &e in rest {
        if i64::from(e) <= prev || text.get(e as usize) != Some(&0x0D) {
            return false;
        }
        prev = i64::from(e);
    }
    last as usize == text.len() && i64::from(last) > prev
}

/// The `TCD ` instance that describes story `story_index` with `cells` cells: the preferred
/// instance when it fits, otherwise the first one that does.
pub(crate) fn find_tcd(q: &[u8], story_index: usize, cells: usize, prefer: usize) -> Result<u16> {
    let dir = read_directory(q)?;
    let text = story_records(q, story_index)?.text;
    let fits = |c: &Chunk| {
        data(q, c)
            .ok()
            .and_then(tcd_ends)
            .is_some_and(|e| e.len() == cells && ends_fit(&text, &e))
    };
    let tcds: Vec<&Chunk> = dir.chunks.iter().filter(|c| &c.tag == b"TCD ").collect();
    tcds.iter()
        .find(|c| usize::from(c.instance) == prefer && fits(c))
        .or_else(|| tcds.iter().find(|c| fits(c)))
        .map(|c| c.instance)
        .ok_or_else(|| {
            bad(format!(
                "no TCD chunk fits story {story_index} with {cells} cells"
            ))
        })
}

/// Overwrite the cell ends of `TCD ` instance `instance` in place (same cell count, so the
/// chunk keeps its size and position).
pub(crate) fn set_tcd(q: &[u8], instance: u16, ends: &[u32]) -> Result<Vec<u8>> {
    let dir = read_directory(q)?;
    let c = find(&dir, b"TCD ", instance).ok_or_else(|| bad("no such TCD chunk"))?;
    let old = tcd_ends(data(q, c)?).ok_or_else(|| bad("malformed TCD chunk"))?;
    if old.len() != ends.len() {
        return Err(bad("TCD cell count cannot change"));
    }
    let mut out = q.to_vec();
    for (i, &e) in ends.iter().enumerate() {
        put_u32(&mut out, c.offset + 12 + 4 * i, e);
    }
    Ok(out)
}

/// Frame of a new story's `MCLD` record, in points on the page.
#[derive(Clone, Copy)]
pub(crate) struct NewFrame {
    /// The layout cache (items 0-3: top, left, bottom, right) to write instead of the one
    /// fitted from the page position, for frames that are not text boxes.
    pub cache: Option<[u32; 4]>,
    pub left_pt: f64,
    pub top_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
    pub width_emu: u32,
    pub height_emu: u32,
}

/// Plain character record of text and of the paragraph mark, and the empty paragraph
/// record (spec 4L.3).
const REC_TEXT: [u8; 16] = [
    0x10, 0, 0, 0, 0x12, 0x22, 0x09, 0x04, 0, 0, 0x3e, 0x22, 0x09, 0x04, 0, 0,
];
const REC_MARK: [u8; 10] = [0x0a, 0, 0, 0, 0x12, 0x22, 0x09, 0x04, 0, 0];
const REC_PARA: [u8; 4] = [4, 0, 0, 0];

/// The chunks of Publisher's text engine in the order Publisher writes them (spec 6.11).
const TEXT_ENGINE_ORDER: [(&[u8; 4], u16); 15] = [
    (b"TEXT", 0),
    (b"STSH", 0),
    (b"STSH", 1),
    (b"STSH", 2),
    (b"FDPP", 0),
    (b"FDPC", 0),
    (b"SYID", 0),
    (b"SGP ", 0),
    (b"INK ", 0),
    (b"BTEP", 0),
    (b"BTEC", 0),
    (b"FONT", 0),
    (b"STRS", 0),
    (b"MCLD", 0),
    (b"PL  ", 0),
];

/// Give a Quill stream that has no story yet (a blank publication's, spec 6.3) the seven
/// chunks Publisher adds with its first text: an empty `TEXT`, empty formatting pages
/// (`FDPP`, `FDPC`), the two page tables (`BTEP`, `BTEC`), `STRS` with no story and an empty
/// `MCLD`. The result is a transient, story-less skeleton that `add_story` fills in.
fn seed_text_engine(q: &[u8]) -> Result<Vec<u8>> {
    let dir = read_directory(q)?;
    if dir.blocks.len() != 1 {
        return Err(bad("a text-less stream with several directory blocks"));
    }
    if let Some(c) = dir
        .chunks
        .iter()
        .find(|c| !TEXT_ENGINE_ORDER.iter().any(|(t, i)| **t == c.tag && *i == c.instance))
    {
        return Err(bad(format!("unexpected chunk {}", tag_str(&c.tag))));
    }
    let new_entry = |tag: &[u8; 4], inst: u16| {
        let plc = matches!(tag, b"BTEP" | b"BTEC" | b"STRS");
        let mut e = [0u8; 24];
        e[0] = 0x18;
        e[2..6].copy_from_slice(tag);
        e[6..8].copy_from_slice(&inst.to_le_bytes());
        e[8] = 1;
        e[12..16].copy_from_slice(if plc { b"PLC " } else { tag });
        e
    };
    let mut page = vec![0u8; PAGE];
    page[2] = 1;
    // BTE: page count 0, the constant 8 bytes at 4..12, then the first position.
    let mut bte0 = vec![0u8; 16];
    bte0[4] = 4;
    // STRS: no story, 8, 0xff, then the tail that belongs to the first story (spec 6.11).
    let mut strs = vec![0u8; 12];
    strs[4] = 8;
    strs[8] = 0xff;
    strs.extend_from_slice(&[0, 0, 0, 0, 10, 0, 0, 0, 0x00, 0x22, 5, 0, 0, 0]);
    let mut out = vec![0u8; TEXT_OFFSET];
    let mut entries: Vec<[u8; 24]> = Vec::new();
    for (tag, inst) in TEXT_ENGINE_ORDER {
        let (mut entry, payload) = match dir
            .chunks
            .iter()
            .find(|c| &c.tag == tag && c.instance == inst)
        {
            Some(c) => {
                let mut e = [0u8; 24];
                e.copy_from_slice(&q[c.entry_pos..c.entry_pos + 24]);
                (e, data(q, c)?.to_vec())
            }
            None => (
                new_entry(tag, inst),
                match tag {
                    b"TEXT" => Vec::new(),
                    b"FDPP" | b"FDPC" => page.clone(),
                    b"BTEP" | b"BTEC" => bte0.clone(),
                    b"STRS" => strs.clone(),
                    b"MCLD" => vec![0u8; 8],
                    _ => return Err(bad("no default for a text-engine chunk")),
                },
            ),
        };
        if matches!(tag, b"FDPP" | b"FDPC") {
            out.resize(align(out.len()), 0);
        }
        entry[16..20].copy_from_slice(&(out.len() as u32).to_le_bytes());
        entry[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&payload);
        entries.push(entry);
    }
    let total = align(out.len());
    out.resize(total, 0);
    out[..0x18].copy_from_slice(&q[..0x18]);
    out[0x0C..0x0E].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    put_u32(&mut out, HEADER_END_OFFSET, total as u32);
    out[0x18..0x1A].copy_from_slice(&u16_at(q, 0x18)?.to_le_bytes());
    out[0x1A..0x1C].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    put_u32(&mut out, 0x1C, 0xFFFF_FFFF);
    for (i, e) in entries.iter().enumerate() {
        out[0x20 + 24 * i..0x20 + 24 * (i + 1)].copy_from_slice(e);
    }
    Ok(out)
}

/// The `MCLD` record of one single-frame story with Publisher's default frame properties
/// (spec 6.11): a 12-byte header block, the frame count 1 and the 130-byte frame. The frame's
/// position and size items are filled in by the caller.
fn default_mcld_record() -> Vec<u8> {
    let item = |id: u16, ty: u8, v: &[u8]| {
        let mut b = ih(id, ty).to_vec();
        b.extend_from_slice(v);
        b
    };
    let u32i = |id: u16, v: u32| item(id, 0x20, &v.to_le_bytes());
    let mut frame: Vec<u8> = Vec::new();
    for id in 0x200..=0x205 {
        frame.extend(u32i(id, 0));
    }
    for id in 0x206..=0x209 {
        frame.extend(u32i(id, 36_576));
    }
    frame.extend(u32i(0x20A, 82_676));
    frame.extend(item(0x20B, 0x18, &[0, 0]));
    frame.extend(u32i(0x20D, 0));
    frame.extend(u32i(0x211, 0));
    frame.extend(u32i(0x212, 0));
    frame.extend(item(0x213, 0x10, &[0xff, 0]));
    frame.extend(item(0x214, 0x08, &[]));
    frame.extend(u32i(0x215, 1));
    frame.extend(u32i(0x216, 9_525));
    frame.extend(u32i(0x218, 0));
    frame.extend(item(0x21A, 0x00, &[]));
    let nested = u32i(0x200, 0xFFFF_FFFC);
    let mut n = ((nested.len() + 4) as u32).to_le_bytes().to_vec();
    n.extend(nested);
    frame.extend(item(0x21D, 0x88, &n));
    let mut rec = 12u32.to_le_bytes().to_vec();
    rec.extend(item(0x200, 0x08, &[]));
    rec.extend(u32i(0x201, 228_600));
    rec.extend(1u32.to_le_bytes());
    rec.extend(((frame.len() + 4) as u32).to_le_bytes());
    rec.extend(frame);
    rec
}

/// Append a new story (spec 4L.3) holding `text` + `\r`; returns the stream and the new
/// story number (`SYID` high-water mark + 1, never below an existing number).
pub(crate) fn add_story(q: &[u8], text: &str, frame: &NewFrame) -> Result<(Vec<u8>, u32)> {
    // Story text: paragraphs end at every \r; the final \r is always present.
    let norm = text.replace("\r\n", "\r").replace('\n', "\r");
    let mut units: Vec<u16> = norm.encode_utf16().collect();
    units.push(0x0D);
    add_story_impl(q, units, false, std::slice::from_ref(frame), false, None)
}

/// Append the one story of a table (spec 4E.1.3): the cells' texts in order, each cell
/// ending in `\r` (paragraphs inside a cell are separated by `\n` or `\r` like a text box's),
/// a `TCD ` chunk with the cell ends and an `MCLD` record with one frame per cell.
/// Returns the stream and the new story number.
pub(crate) fn add_table_story(
    q: &[u8],
    cells: &[String],
    frames: &[NewFrame],
) -> Result<(Vec<u8>, u32)> {
    if cells.len() != frames.len() || cells.is_empty() {
        return Err(bad("a table story needs one frame per cell"));
    }
    let mut units: Vec<u16> = Vec::new();
    let mut ends: Vec<u32> = Vec::with_capacity(cells.len());
    for cell in cells {
        let norm = cell.replace("\r\n", "\r").replace('\n', "\r");
        units.extend(norm.encode_utf16());
        units.push(0x0D);
        ends.push(units.len() as u32 - 1);
    }
    // The last cell's end is the story length (one past its final `\r`).
    if let Some(last) = ends.last_mut() {
        *last += 1;
    }
    add_story_impl(q, units, true, frames, true, Some(ends))
}

/// `table`: the story is a table's (spec 4E.1.3): its frames have item 0x20A = 0 instead of
/// the text box's 82,676, and Publisher skips story numbers before it (below).
fn add_story_impl(
    q: &[u8],
    units: Vec<u16>,
    split_runs: bool,
    frames: &[NewFrame],
    table: bool,
    tcd_ends: Option<Vec<u32>>,
) -> Result<(Vec<u8>, u32)> {
    let seeded;
    let q = if find(&read_directory(q)?, b"TEXT", 0).is_none() {
        seeded = seed_text_engine(q)?;
        &seeded[..]
    } else {
        q
    };
    let dir = read_directory(q)?;
    let text_c = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let strs_c = find(&dir, b"STRS", 0).ok_or_else(|| bad("no STRS chunk"))?;
    let syid_c = find(&dir, b"SYID", 0).ok_or_else(|| bad("no SYID chunk"))?;
    let mcld_c = find(&dir, b"MCLD", 0).ok_or_else(|| bad("no MCLD chunk"))?;
    if text_c.offset != TEXT_OFFSET {
        return Err(bad("TEXT is not at 0x200"));
    }
    // STRS: count, 8, 0xff, lengths, trailer (one 4-byte record per story, spec 4L.3).
    let mut strs = data(q, strs_c)?.to_vec();
    let count = u32_at(&strs, 0)? as usize;
    let mut lens = Vec::with_capacity(count);
    for i in 0..count {
        lens.push(u32_at(&strs, 12 + 4 * i)? as usize);
    }
    if lens.iter().sum::<usize>() * 2 != text_c.len || strs.len() < 12 + 4 * count {
        return Err(bad("STRS lengths do not add up to the TEXT length"));
    }
    let s = TEXT_OFFSET + text_c.len;
    let end = s + 2 * units.len();
    strs.splice(
        12 + 4 * count..12 + 4 * count,
        (units.len() as u32).to_le_bytes(),
    );
    // The first story's record is part of the chunk's fixed tail (spec 6.11).
    if count > 0 {
        strs.extend_from_slice(&4u32.to_le_bytes());
    }
    put_u32(&mut strs, 0, (count + 1) as u32);

    let mut text_bytes = data(q, text_c)?.to_vec();
    for u in &units {
        text_bytes.extend_from_slice(&u.to_le_bytes());
    }

    // Formatting runs.
    let (mut cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (mut pruns, _) = read_runs(q, &dir, b"FDPP")?;
    let empty_text = text_c.len == 0 && cruns.is_empty() && pruns.is_empty();
    if !empty_text
        && (cruns.last().map(|r| r.end) != Some(s) || pruns.last().map(|r| r.end) != Some(s))
    {
        return Err(bad("the formatting runs do not end at the end of TEXT"));
    }
    // Character runs: Publisher extends the previous run when its record is the same
    // (an empty box's paragraph mark joins the last mark run); paragraph runs never merge.
    let mut push_char = |end: usize, record: &[u8]| match cruns.last_mut() {
        Some(prev) if prev.record == record => prev.end = end,
        _ => cruns.push(Run {
            end,
            record: record.to_vec(),
        }),
    };
    if split_runs {
        // A table: one text run per non-empty cell paragraph, one mark run per stretch of
        // paragraph marks (spec 4E.1.3).
        let mut i = 0;
        while i < units.len() {
            if units[i] == 0x0D {
                push_char(s + 2 * (i + 1), &REC_MARK);
                i += 1;
            } else {
                while units[i] != 0x0D {
                    i += 1;
                }
                push_char(s + 2 * i, &REC_TEXT);
            }
        }
    } else {
        if units.len() > 1 {
            push_char(end - 2, &REC_TEXT);
        }
        push_char(end, &REC_MARK);
    }
    for (i, u) in units.iter().enumerate() {
        if *u == 0x0D {
            pruns.push(Run {
                end: s + 2 * (i + 1),
                record: REC_PARA.to_vec(),
            });
        }
    }

    // SYID: high-water mark, count, story numbers.
    let mut syid = data(q, syid_c)?.to_vec();
    let (high, n) = (u32_at(&syid, 0)?, u32_at(&syid, 4)? as usize);
    if syid.len() != 8 + 4 * n {
        return Err(bad("unexpected SYID layout"));
    }
    let mut top = high;
    for i in 0..n {
        top = top.max(u32_at(&syid, 8 + 4 * i)?);
    }
    // Publisher numbers a table's story two above the high-water mark in a file without a
    // story and three above otherwise (`table-2x2`, `table-two`, `table-after-textbox`); any
    // number above all existing ones keeps the order of the stories (spec 4E.1.3).
    let story = top + if !table { 1 } else if n == 0 { 2 } else { 3 };
    put_u32(&mut syid, 0, story);
    put_u32(&mut syid, 4, (n + 1) as u32);
    syid.extend_from_slice(&story.to_le_bytes());

    // MCLD: header n, n, 1..n; then one record per story. A stream whose stories were all
    // deleted keeps a stub (`01 00 00 00 00 00 00 00`, spec 4E.1.5) that counts as empty.
    let mcld = data(q, mcld_c)?;
    let (high, ids) = if count == 0 {
        (0, Vec::new())
    } else {
        mcld_header(mcld, count)?
    };
    let m = ids.len();
    let mcld: &[u8] = if m == 0 { &[0; 8] } else { mcld };
    let mut pos = 8 + 4 * m;
    let mut template: Option<&[u8]> = None;
    for _ in 0..m {
        let start = pos;
        pos += u32_at(mcld, pos)? as usize;
        let n_frames = u32_at(mcld, pos)? as usize;
        pos += 4;
        let mut ok = n_frames == 1;
        for _ in 0..n_frames {
            pos += u32_at(mcld, pos)? as usize;
        }
        ok &= pos <= mcld.len();
        if ok {
            template = Some(&mcld[start..pos]);
        }
    }
    // A table's frames are not text-box frames (spec 4E.1.3), and a file whose only
    // records are table records has no single-frame record: Publisher's default one serves.
    let fresh = default_mcld_record();
    let template = match template {
        Some(t) if !table => t,
        _ => &fresh[..],
    };
    // The record is: a header block, the frame count, then the frames.
    let head_len = u32_at(template, 0)? as usize;
    let frame_tpl = template
        .get(head_len + 4..)
        .ok_or_else(|| bad("MCLD record too short"))?;
    let set = |rec: &mut Vec<u8>, id: u16, v: u32| -> Result<()> {
        let slot = pub_format::items::locate_int(rec, 0, id)
            .ok()
            .flatten()
            .filter(|s| s.len == 4)
            .ok_or_else(|| bad(format!("MCLD frame lacks 4-byte item {id}")))?;
        put_u32(rec, slot.offset, v);
        Ok(())
    };
    let fit = |a: f64, b: f64, v: f64| (a * v + b).round().max(0.0) as u32;
    let mut rec = template[..head_len].to_vec();
    rec.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    for frame in frames {
        let mut f = frame_tpl.to_vec();
        let (l, t) = (frame.left_pt, frame.top_pt);
        let (r, b) = (l + frame.width_pt, t + frame.height_pt);
        let cache = frame.cache.unwrap_or([
            fit(0.5696, 4715.1, t),
            fit(0.5692, 4766.2, l),
            fit(0.5702, 4714.2, b),
            fit(0.5695, 4766.1, r),
        ]);
        for (k, v) in cache.into_iter().enumerate() {
            set(&mut f, 0x200 + k as u16, v)?;
        }
        set(&mut f, 0x204, frame.width_emu)?;
        set(&mut f, 0x205, frame.height_emu)?;
        if table {
            set(&mut f, 0x20A, 0)?;
            set(&mut f, 0x216, 0)?;
        }
        rec.extend_from_slice(&f);
    }
    // Header: record-id high-water mark, count, ids (a fresh file has n, n, 1..n; Publisher
    // burns an id when it edits a table's rows or columns, spec 4E.1.5).
    let mut new_mcld = Vec::new();
    new_mcld.extend_from_slice(&(high + 1).to_le_bytes());
    new_mcld.extend_from_slice(&((m + 1) as u32).to_le_bytes());
    for id in ids.iter().chain(&[high + 1]) {
        new_mcld.extend_from_slice(&id.to_le_bytes());
    }
    new_mcld.extend_from_slice(&mcld[8 + 4 * m..]);
    new_mcld.extend_from_slice(&rec);

    let out = rebuild_with(
        q,
        &dir,
        text_bytes,
        strs,
        &cruns,
        &pruns,
        None,
        None,
        &[(*b"SYID", syid), (*b"MCLD", new_mcld)],
        &DirEdit {
            tcd: match tcd_ends {
                Some(ends) => TcdEdit::Add {
                    instance: u16::try_from(count)
                        .map_err(|_| bad("too many stories for a TCD instance"))?,
                    ends,
                },
                None => TcdEdit::None,
            },
            drop: Vec::new(),
            fields: None,
        },
    )?;
    Ok((out, story))
}

const MCLD_FLAG_AT: usize = 113;

/// Append a copy of each story in `src` (story indexes, in the order given) to the end of
/// the Quill stream (spec 4P.5): the text, the character and paragraph runs (full
/// formatting) and the MCLD frame record are copied; SYID gets new story numbers
/// (`high-water mark + 1`, then +2 each, skipping one like Publisher). Returns the stream and the new story numbers.
/// Stories with more than one frame (linked text boxes) are refused.
pub(crate) fn duplicate_stories(q: &[u8], src: &[usize]) -> Result<(Vec<u8>, Vec<u32>)> {
    let dir = read_directory(q)?;
    let text_c = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let strs_c = find(&dir, b"STRS", 0).ok_or_else(|| bad("no STRS chunk"))?;
    let syid_c = find(&dir, b"SYID", 0).ok_or_else(|| bad("no SYID chunk"))?;
    let mcld_c = find(&dir, b"MCLD", 0).ok_or_else(|| bad("no MCLD chunk"))?;
    if text_c.offset != TEXT_OFFSET {
        return Err(bad("TEXT is not at 0x200"));
    }
    let mut strs = data(q, strs_c)?.to_vec();
    let count = u32_at(&strs, 0)? as usize;
    let mut lens = Vec::with_capacity(count);
    for i in 0..count {
        lens.push(u32_at(&strs, 12 + 4 * i)? as usize);
    }
    if lens.iter().sum::<usize>() * 2 != text_c.len || strs.len() < 12 + 4 * count {
        return Err(bad("STRS lengths do not add up to the TEXT length"));
    }
    if let Some(&i) = src.iter().find(|&&i| i >= count) {
        return Err(WriteError::NoSuchStory { index: i, count });
    }
    let old_end = TEXT_OFFSET + text_c.len;
    let (cruns0, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns0, _) = read_runs(q, &dir, b"FDPP")?;
    if cruns0.last().map(|r| r.end) != Some(old_end) || pruns0.last().map(|r| r.end) != Some(old_end)
    {
        return Err(bad("the formatting runs do not end at the end of TEXT"));
    }
    let (mut cruns, mut pruns) = (cruns0.clone(), pruns0.clone());
    let mut text_bytes = data(q, text_c)?.to_vec();

    // SYID.
    let mut syid = data(q, syid_c)?.to_vec();
    let (high, n) = (u32_at(&syid, 0)?, u32_at(&syid, 4)? as usize);
    if syid.len() != 8 + 4 * n {
        return Err(bad("unexpected SYID layout"));
    }
    let mut top = high;
    for i in 0..n {
        top = top.max(u32_at(&syid, 8 + 4 * i)?);
    }

    // MCLD record boundaries.
    let mcld = data(q, mcld_c)?;
    let m = u32_at(mcld, 0)? as usize;
    if m != count || u32_at(mcld, 4)? as usize != m {
        return Err(bad("MCLD does not hold one record per story"));
    }
    let mut recs: Vec<(usize, usize, u32)> = Vec::with_capacity(m);
    let mut pos = 8 + 4 * m;
    for _ in 0..m {
        let start = pos;
        pos += u32_at(mcld, pos)? as usize;
        let frames = u32_at(mcld, pos)?;
        pos += 4;
        for _ in 0..frames {
            pos += u32_at(mcld, pos)? as usize;
        }
        if pos > mcld.len() {
            return Err(bad("truncated MCLD"));
        }
        recs.push((start, pos, frames));
    }
    let mut new_mcld = mcld.to_vec();
    let mut mcld_tail = Vec::new();

    let mut numbers = Vec::with_capacity(src.len());
    let mut cur = old_end;
    for (k, &i) in src.iter().enumerate() {
        let s = TEXT_OFFSET + 2 * lens[..i].iter().sum::<usize>();
        let e = s + 2 * lens[i];
        let (start, end, frames) = recs[i];
        if frames != 1 {
            return Err(bad(format!("story {i} has {frames} frames (linked text boxes)")));
        }
        text_bytes.extend_from_slice(&q[s..e]);
        strs.splice(
            12 + 4 * (count + k)..12 + 4 * (count + k),
            (lens[i] as u32).to_le_bytes(),
        );
        strs.extend_from_slice(&4u32.to_le_bytes());
        for (src_runs, out, merge) in [(&cruns0, &mut cruns, true), (&pruns0, &mut pruns, false)] {
            let mut from = TEXT_OFFSET;
            for r in src_runs {
                let (a, b) = (from.max(s), r.end.min(e));
                from = r.end;
                if a < b {
                    let end_at = cur + (b - s);
                    match out.last_mut() {
                        Some(prev) if merge && prev.record == r.record && prev.end == cur + (a - s) => {
                            prev.end = end_at
                        }
                        _ => out.push(Run {
                            end: end_at,
                            record: r.record.clone(),
                        }),
                    }
                }
            }
        }
        cur += e - s;
        top += if k == 0 { 1 } else { 2 };
        numbers.push(top);
        syid.extend_from_slice(&top.to_le_bytes());
        new_mcld.splice(
            8 + 4 * (m + k)..8 + 4 * (m + k),
            ((m + k + 1) as u32).to_le_bytes(),
        );
        let at = mcld_tail.len();
        mcld_tail.extend_from_slice(&mcld[start..end]);
        // Byte 113 of a frame record is 10 on page 1's stories and 2 on page 2's; Publisher
        // writes 10 on every copy (spec 4P.5, checked on a page-2 duplicate: leaving the 2
        // makes Publisher 2021 hang on Open).
        if end - start > MCLD_FLAG_AT && mcld_tail[at + MCLD_FLAG_AT] == 2 {
            mcld_tail[at + MCLD_FLAG_AT] = 10;
        }
    }
    put_u32(&mut strs, 0, (count + src.len()) as u32);
    put_u32(&mut syid, 0, top);
    put_u32(&mut syid, 4, (n + src.len()) as u32);
    new_mcld.extend_from_slice(&mcld_tail);
    put_u32(&mut new_mcld, 0, (m + src.len()) as u32);
    put_u32(&mut new_mcld, 4, (m + src.len()) as u32);
    let out = rebuild_with(
        q,
        &dir,
        text_bytes,
        strs,
        &cruns,
        &pruns,
        None,
        None,
        &[(*b"SYID", syid), (*b"MCLD", new_mcld)],
        &DirEdit::default(),
    )?;
    Ok((out, numbers))
}

/// Remove the story with number `story_number` (spec 4E.1.5, "Delete table"): its text and
/// formatting runs, its `STRS` length, its `SYID` entry (the high-water mark stays), its
/// `MCLD` record and its `TCD ` chunk (later `TCD ` instances move down by one). When it was
/// the last story, the text engine's chunks go as in a blank publication (`TEXT`, `FDPP`,
/// `FDPC`, `BTEP`, `BTEC`, `STRS`), leaving an `MCLD` stub.
pub(crate) fn remove_story(q: &[u8], story_number: u32) -> Result<(Vec<u8>, usize)> {
    let dir = read_directory(q)?;
    let text_c = find(&dir, b"TEXT", 0).ok_or_else(|| bad("no TEXT chunk"))?;
    let strs_c = find(&dir, b"STRS", 0).ok_or_else(|| bad("no STRS chunk"))?;
    let syid_c = find(&dir, b"SYID", 0).ok_or_else(|| bad("no SYID chunk"))?;
    let mcld_c = find(&dir, b"MCLD", 0).ok_or_else(|| bad("no MCLD chunk"))?;
    if text_c.offset != TEXT_OFFSET {
        return Err(bad("TEXT is not at 0x200"));
    }
    let syid = data(q, syid_c)?;
    let (high, n) = (u32_at(syid, 0)?, u32_at(syid, 4)? as usize);
    if syid.len() != 8 + 4 * n {
        return Err(bad("unexpected SYID layout"));
    }
    let numbers: Vec<u32> = (0..n)
        .map(|i| u32_at(syid, 8 + 4 * i))
        .collect::<Result<_>>()?;
    let idx = numbers
        .iter()
        .position(|&x| x == story_number)
        .ok_or_else(|| bad(format!("no story number {story_number}")))?;

    let mut strs = data(q, strs_c)?.to_vec();
    let count = u32_at(&strs, 0)? as usize;
    if count != n {
        return Err(bad("SYID and STRS disagree on the number of stories"));
    }
    let lens: Vec<usize> = (0..count)
        .map(|i| u32_at(&strs, 12 + 4 * i).map(|v| v as usize))
        .collect::<Result<_>>()?;
    if lens.iter().sum::<usize>() * 2 != text_c.len || strs.len() < 12 + 4 * count {
        return Err(bad("STRS lengths do not add up to the TEXT length"));
    }
    let s = TEXT_OFFSET + 2 * lens[..idx].iter().sum::<usize>();
    let e = s + 2 * lens[idx];
    let mut text_bytes = data(q, text_c)?.to_vec();
    text_bytes.drain(s - TEXT_OFFSET..e - TEXT_OFFSET);
    strs.drain(12 + 4 * idx..16 + 4 * idx);
    // Every story after the first has a 4-byte record at the end of the chunk (spec 4L.3).
    if count >= 2 {
        strs.truncate(strs.len() - 4);
    }
    put_u32(&mut strs, 0, (count - 1) as u32);

    let (cruns, _) = read_runs(q, &dir, b"FDPC")?;
    let (pruns, _) = read_runs(q, &dir, b"FDPP")?;
    let delta = -((e - s) as isize);
    let cruns = splice_runs(&cruns, s, e, delta, Vec::new());
    let pruns = splice_runs(&pruns, s, e, delta, Vec::new());

    let mut new_syid = syid.to_vec();
    new_syid.drain(8 + 4 * idx..12 + 4 * idx);
    put_u32(&mut new_syid, 0, high);
    put_u32(&mut new_syid, 4, (n - 1) as u32);

    // MCLD: header m, m, 1..m, then one record per story.
    let mcld = data(q, mcld_c)?;
    let (high, mut ids) = mcld_header(mcld, count)?;
    let regular = high as usize == ids.len() && ids.iter().zip(1..).all(|(a, b)| *a == b);
    let m = ids.len();
    let mut pos = 8 + 4 * m;
    let mut records: Vec<&[u8]> = Vec::new();
    for _ in 0..m {
        let start = pos;
        pos += u32_at(mcld, pos)? as usize;
        let frames = u32_at(mcld, pos)? as usize;
        pos += 4;
        for _ in 0..frames {
            pos += u32_at(mcld, pos)? as usize;
        }
        records.push(
            mcld.get(start..pos)
                .ok_or_else(|| bad("MCLD record runs past the chunk"))?,
        );
    }
    records.remove(idx);
    ids.remove(idx);
    let mut new_mcld = Vec::new();
    if records.is_empty() {
        new_mcld.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);
    } else {
        // A regular header (n, n, 1..n) is renumbered; one Publisher has edited keeps its
        // high-water mark and the other records' ids.
        if regular {
            ids = (1..=records.len() as u32).collect();
        }
        let top = if regular { records.len() as u32 } else { high };
        new_mcld.extend_from_slice(&top.to_le_bytes());
        new_mcld.extend_from_slice(&(records.len() as u32).to_le_bytes());
        for k in &ids {
            new_mcld.extend_from_slice(&k.to_le_bytes());
        }
        for r in &records {
            new_mcld.extend_from_slice(r);
        }
    }

    let last = count == 1;
    let out = rebuild_with(
        q,
        &dir,
        text_bytes,
        strs,
        &cruns,
        &pruns,
        None,
        None,
        &[(*b"SYID", new_syid), (*b"MCLD", new_mcld)],
        &DirEdit {
            tcd: TcdEdit::Remove(u16::try_from(idx).map_err(|_| bad("story index too large"))?),
            drop: if last {
                ["TEXT", "FDPP", "FDPC", "BTEP", "BTEC", "STRS"]
                    .iter()
                    .map(|t| <[u8; 4]>::try_from(t.as_bytes()).expect("four letters"))
                    .collect()
            } else {
                Vec::new()
            },
            fields: None,
        },
    )?;
    Ok((out, count - 1))
}

#[cfg(test)]
mod item_id_tests {
    use super::*;

    #[test]
    fn header_bytes_carry_id_bits_8_to_10() {
        // Ids below 256 keep their bytes; id 0x12C class 4 is `2C 21`.
        assert_eq!(ih(0x0C, 0x22), [0x0C, 0x22 & 0xF8]);
        assert_eq!(ih(0x12C, 0x20), [0x2C, 0x21]);
        assert_eq!(ih(ITEM_FONT, 0x8A), [0x24, 0x8A]);
    }

    #[test]
    fn put_item_orders_by_full_id() {
        let raw = |id: u16| RawItem {
            id,
            bytes: ih(id, 0x08).to_vec(),
        };
        let mut items = vec![raw(0x05), raw(0x13C), raw(0x3C2)];
        put_item(&mut items, 0x3C, ih(0x3C, 0x08).to_vec());
        put_item(&mut items, 0x200, ih(0x200, 0x08).to_vec());
        let ids: Vec<u16> = items.iter().map(|i| i.id).collect();
        assert_eq!(ids, [0x05, 0x3C, 0x13C, 0x200, 0x3C2]);
        let joined = join_items(&items);
        let split = split_items(&joined).unwrap();
        assert_eq!(split.iter().map(|i| i.id).collect::<Vec<_>>(), ids);
    }
}

#[cfg(test)]
mod duplicate_tests {
    use super::*;

    /// Page-2 stories (flag byte 2) copied by Publisher carry 10; story numbers go up by
    /// 1 then 2; the high-water mark is the last number (spec 4P.5; a copy with the flag
    /// left at 2 makes Publisher 2021 hang on Open).
    #[test]
    fn copies_of_page_two_stories_follow_publishers_numbering_and_flag() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/gen/bulletin.pub");
        let streams = pub_format::Streams::from_cfb(&std::fs::read(path).unwrap()).unwrap();
        let q = &streams.quill;
        let dir = read_directory(q).unwrap();
        let mcld = data(q, find(&dir, b"MCLD", 0).unwrap()).unwrap().to_vec();
        let m = u32_at(&mcld, 0).unwrap() as usize;
        // Page 2's stories are the last two.
        let (out, nums) = duplicate_stories(q, &[m - 1, m - 2]).unwrap();
        let d2 = read_directory(&out).unwrap();
        let mcld2 = data(&out, find(&d2, b"MCLD", 0).unwrap()).unwrap().to_vec();
        let n2 = u32_at(&mcld2, 0).unwrap() as usize;
        assert_eq!(n2, m + 2);
        let syid = data(&out, find(&d2, b"SYID", 0).unwrap()).unwrap().to_vec();
        let high = u32_at(&syid, 0).unwrap();
        assert_eq!(nums[1], nums[0] + 2);
        assert_eq!(high, nums[1]);
        for i in m..m + 2 {
            let mut pos = 8 + 4 * n2;
            for _ in 0..i {
                pos += u32_at(&mcld2, pos).unwrap() as usize;
                let f = u32_at(&mcld2, pos).unwrap();
                pos += 4;
                for _ in 0..f {
                    pos += u32_at(&mcld2, pos).unwrap() as usize;
                }
            }
            assert_eq!(mcld2[pos + MCLD_FLAG_AT], 10, "copy {i}");
        }
    }
}
