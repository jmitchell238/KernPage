//! Insert, replace and delete embedded pictures (spec 4C.1): a picture-store entry in the
//! drawing group, a record in `EscherDelayStm`, a picture shape container in the page group,
//! and two `Contents` objects (the kind-1 frame and the kind-102 picture info). Quill and the
//! story registries are untouched.

use super::group::{
    anchor_atom, client_anchor, drawing_counters, find_path, group_counters, index_entry, install,
    items_of, put, rd, rec_at, Body, Cts, EscherDoc, Rec,
};
use super::md4::md4;
use super::pages::EMPTY_SLOT;
use super::shape::{page_drawing, props};
use super::textbox::{add_overlap_with, grow_block};
use super::{Result, WriteError, EMU_PER_POINT, KIND_TEXT_BOX};
use cfb::CompoundFile;
use pub_format::Streams;
use std::collections::HashSet;
use std::io::{Cursor, Seek, SeekFrom, Write};

const KIND_PAGE: u64 = 67;
const KIND_FRAME: u64 = 1;
const KIND_PICTURE_INFO: u16 = 102;
const FSP_FLAGS: u32 = 0x0A00;
const INSET: u32 = 36_576;
/// Size of a picture-store entry's payload (spec 4C.1.1).
const BSE_LEN: usize = 36;
const BSE_TAG: u16 = 0x00FF;
const DELAY_PATH: &str = "/Escher/EscherDelayStm";

/// Image formats that can be embedded as they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureKind {
    Png,
    Jpeg,
}

impl PictureKind {
    /// [MS-ODRAW] blip type of the store entry (6 PNG, 5 JPEG).
    fn blip_type(self) -> u8 {
        match self {
            PictureKind::Png => 6,
            PictureKind::Jpeg => 5,
        }
    }
    /// Instance and record type of the blip record in the delay stream (one uid).
    fn record(self) -> (u16, u16) {
        match self {
            PictureKind::Png => (0x6E0, 0xF01E),
            PictureKind::Jpeg => (0x46A, 0xF01D),
        }
    }
    fn matches(self, image: &[u8]) -> bool {
        match self {
            PictureKind::Png => image.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            PictureKind::Jpeg => image.starts_with(&[0xFF, 0xD8]),
        }
    }
}

/// An image to embed and what Publisher records about its file.
#[derive(Debug, Clone, Copy)]
pub struct PictureSource<'a> {
    /// The original file bytes, stored unchanged.
    pub image: &'a [u8],
    pub format: PictureKind,
    /// The file's name, with or without a path; the shape is named after its stem.
    pub file_name: &'a str,
    /// Last-write time of the file as a Windows FILETIME. `None` (or 0) writes the current
    /// time: Publisher 2021 refuses to open a file whose picture-info FILETIME is 0.
    pub modified: Option<u64>,
}

/// A picture to insert, with its frame in points from the page's top-left corner.
#[derive(Debug, Clone, Copy)]
pub struct NewPicture<'a> {
    pub image: &'a [u8],
    pub format: PictureKind,
    pub file_name: &'a str,
    pub modified: Option<u64>,
    pub left_pt: f64,
    pub top_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
}

impl<'a> NewPicture<'a> {
    fn source(&self) -> PictureSource<'a> {
        PictureSource {
            image: self.image,
            format: self.format,
            file_name: self.file_name,
            modified: self.modified,
        }
    }
}

fn fail<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Picture(m.into()))
}

fn check_source(s: &PictureSource) -> Result<()> {
    if s.image.is_empty() || !s.format.matches(s.image) {
        return fail("the image bytes are not a PNG/JPEG file of the stated format");
    }
    if u32::try_from(s.image.len() + 25).is_err() {
        return fail("the image is too large");
    }
    Ok(())
}

fn base_name(file_name: &str) -> &str {
    file_name.rsplit(['/', '\\']).next().unwrap_or(file_name)
}

fn stem(file_name: &str) -> &str {
    let n = base_name(file_name);
    match n.rfind('.') {
        Some(i) if i > 0 => &n[..i],
        _ => n,
    }
}

/// UTF-16LE with a terminating NUL.
fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

// ---------------------------------------------------------------- picture store

/// Find the picture's store entry by its MD4 uid and take a reference, or append a new entry
/// (creating the 0xF001 after the 0xF006 header when there is none) and its delay record.
/// Returns the 1-based store index.
fn acquire_blip(head: &mut Rec, delay: &mut Vec<u8>, s: &PictureSource) -> Result<u32> {
    let uid = md4(s.image);
    let kids = head.kids_mut()?;
    let at = match kids.iter().position(|r| r.ty == 0xF001) {
        Some(i) => i,
        None => {
            let h = kids
                .iter()
                .position(|r| r.ty == 0xF006)
                .ok_or_else(|| WriteError::Picture("the drawing group has no 0xF006".into()))?;
            kids.insert(
                h + 1,
                Rec {
                    vi: 0x000F,
                    ty: 0xF001,
                    body: Body::Cont(Vec::new()),
                },
            );
            h + 1
        }
    };
    let store = &mut kids[at];
    let entries = store.kids_mut()?;
    for (i, e) in entries.iter_mut().enumerate() {
        if let Body::Atom(d) = &mut e.body {
            if e.ty == 0xF007 && d.len() == BSE_LEN && d[0] != 0 && d[2..18] == uid {
                let c = rd(d, 24)? + 1;
                put(d, 24, c)?;
                return Ok(i as u32 + 1);
            }
        }
    }
    let ty = s.format.blip_type();
    let size = (8 + 17 + s.image.len()) as u32;
    let mut d = vec![ty, ty];
    d.extend_from_slice(&uid);
    d.extend_from_slice(&BSE_TAG.to_le_bytes());
    d.extend_from_slice(&size.to_le_bytes());
    d.extend_from_slice(&1u32.to_le_bytes());
    d.extend_from_slice(&(delay.len() as u32).to_le_bytes());
    // The last four bytes are save-to-save garbage in Publisher's files; zeros are written.
    d.extend_from_slice(&[0; 4]);
    entries.push(Rec::atom((u16::from(ty) << 4) | 2, 0xF007, d));
    let n = entries.len();
    store.vi = ((n as u16) << 4) | 0xF;
    let (inst, rty) = s.format.record();
    delay.extend_from_slice(&(inst << 4).to_le_bytes());
    delay.extend_from_slice(&rty.to_le_bytes());
    delay.extend_from_slice(&((17 + s.image.len()) as u32).to_le_bytes());
    delay.extend_from_slice(&uid);
    delay.push(0xFF);
    delay.extend_from_slice(s.image);
    Ok(n as u32)
}

/// Drop one reference of store entry `pib` (1-based); the last one turns the entry into an
/// empty placeholder (all zero but the tag, spec 4C.1.6/7).
fn release_blip(head: &mut Rec, pib: u32) -> Result<()> {
    let store = head
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF001)
        .ok_or_else(|| WriteError::Picture("the file has no picture store".into()))?;
    let e = store
        .kids_mut()?
        .get_mut((pib as usize).wrapping_sub(1))
        .ok_or_else(|| WriteError::Picture(format!("picture {pib} is not in the store")))?;
    let d = e.data_mut()?;
    if d.len() != BSE_LEN {
        return fail("unsupported picture-store entry");
    }
    let c = rd(d, 24)?;
    if c > 1 {
        put(d, 24, c - 1)?;
    } else {
        let mut z = vec![0u8; BSE_LEN];
        z[18..20].copy_from_slice(&BSE_TAG.to_le_bytes());
        *e = Rec::atom(0x0002, 0xF007, z);
    }
    Ok(())
}

/// Rewrite the delay stream from the live entries only, offsets recomputed from 0; drop the
/// whole 0xF001 and empty the stream when no live entry is left (spec 4C.1.2, 4C.1.7).
fn compact_store(head: &mut Rec, delay: &mut Vec<u8>) -> Result<()> {
    let kids = head.kids_mut()?;
    let Some(pos) = kids.iter().position(|r| r.ty == 0xF001) else {
        delay.clear();
        return Ok(());
    };
    let live = kids[pos].kids().iter().any(|e| match &e.body {
        Body::Atom(d) => d.len() == BSE_LEN && rd(d, 20).unwrap_or(0) != 0,
        Body::Cont(_) => true,
    });
    if !live {
        kids.remove(pos);
        delay.clear();
        return Ok(());
    }
    let mut out = Vec::new();
    for e in kids[pos].kids_mut()? {
        let d = e.data_mut()?;
        if d.len() != BSE_LEN {
            return fail("unsupported picture-store entry");
        }
        let size = rd(d, 20)? as usize;
        if size == 0 {
            continue;
        }
        let off = rd(d, 28)? as usize;
        let rec = delay
            .get(off..off.saturating_add(size))
            .ok_or_else(|| WriteError::Picture("a picture record is outside EscherDelayStm".into()))?;
        put(d, 28, out.len() as u32)?;
        out.extend_from_slice(rec);
    }
    *delay = out;
    Ok(())
}

// ------------------------------------------------------------------ Escher shape

type PropTable = (Vec<(u16, u32)>, Vec<u8>);

fn prop_table(r: &Rec) -> Result<PropTable> {
    let Body::Atom(d) = &r.body else {
        return fail("unexpected property table");
    };
    let n = (r.vi >> 4) as usize;
    if d.len() < 6 * n {
        return fail("truncated property table");
    }
    let list = (0..n)
        .map(|i| {
            (
                u16::from_le_bytes([d[6 * i], d[6 * i + 1]]),
                u32::from_le_bytes([d[6 * i + 2], d[6 * i + 3], d[6 * i + 4], d[6 * i + 5]]),
            )
        })
        .collect::<Vec<_>>();
    let data = d[6 * n..].to_vec();
    let complex: usize = list
        .iter()
        .filter(|p| p.0 & 0x8000 != 0)
        .map(|p| p.1 as usize)
        .sum();
    if complex != data.len() {
        return fail("property table complex data does not match its properties");
    }
    Ok((list, data))
}

fn encode_props(vi_ver: u16, ty: u16, list: &[(u16, u32)], data: &[u8]) -> Rec {
    let mut r = props(vi_ver, ty, list);
    if let Body::Atom(d) = &mut r.body {
        d.extend_from_slice(data);
    }
    r
}

/// Offset in the complex data where the complex data of list position `idx` starts.
fn complex_start(list: &[(u16, u32)], idx: usize) -> usize {
    list[..idx]
        .iter()
        .filter(|p| p.0 & 0x8000 != 0)
        .map(|p| p.1 as usize)
        .sum()
}

/// Remove property `id` (any flags) with its complex data.
fn drop_prop(list: &mut Vec<(u16, u32)>, data: &mut Vec<u8>, id: u16) {
    if let Some(i) = list.iter().position(|p| p.0 & 0x3FFF == id) {
        if list[i].0 & 0x8000 != 0 {
            let s = complex_start(list, i);
            data.drain(s..s + list[i].1 as usize);
        }
        list.remove(i);
    }
}

/// Set the picture name (property 0x0105, complex) keeping the list in id order.
fn set_name(list: &mut Vec<(u16, u32)>, data: &mut Vec<u8>, name: &str) {
    drop_prop(list, data, 0x105);
    let bytes = utf16z(name);
    let i = list
        .iter()
        .position(|p| p.0 & 0x3FFF > 0x105)
        .unwrap_or(list.len());
    let s = complex_start(list, i);
    data.splice(s..s, bytes.iter().copied());
    list.insert(i, (0xC105, bytes.len() as u32));
}

/// The picture shape container (spec 4C.1.4).
fn picture_container(spid: u32, pib: u32, name: &str, anchor: [i32; 4], obj: u32) -> Rec {
    let mut fsp = spid.to_le_bytes().to_vec();
    fsp.extend_from_slice(&FSP_FLAGS.to_le_bytes());
    let mut list: Vec<(u16, u32)> = vec![
        (0x7F, 0x0080_0080),
        (0x81, INSET),
        (0x82, INSET),
        (0x83, INSET),
        (0x84, INSET),
        (0x4104, pib),
        (0x106, 1),
        (0x181, 0x0800_0007),
        (0x183, 0x0800_0007),
        (0x1BF, 0x0010_0000),
        (0x1C0, 0x0800_0000),
        (0x1C2, 0x0800_0007),
        (0x1CB, 25_400),
        (0x1FF, 0x0008_0000),
        (0x201, 0x0800_0004),
        (0x285, 0),
        (0x2CB, 0),
        (0x2CC, 0),
        (0x2CE, 0),
        (0x2CF, 0),
        (0x33F, 0x0010_0010),
        (0x384, INSET),
        (0x385, INSET),
        (0x386, INSET),
        (0x387, INSET),
    ];
    let mut data = Vec::new();
    set_name(&mut list, &mut data, name);
    let t: Vec<(u16, u32)> = vec![
        (0x1FF, 0x0040_0000),
        (0x295, 0),
        (0x296, 0),
        (0x297, 0),
        (0x298, 0),
        (0x299, 0),
        (0x29A, 0),
        (0x540, 0x0800_0000),
        (0x542, 0x0800_0007),
        (0x580, 0x0800_0000),
        (0x582, 0x0800_0007),
        (0x5C0, 0x0800_0000),
        (0x5C2, 0x0800_0007),
        (0x600, 0x0800_0000),
        (0x602, 0x0800_0007),
        (0x640, 0x0800_0000),
        (0x642, 0x0800_0007),
        (0x6FF, 0x0002_0002),
    ];
    let mut cd = vec![0x0a, 0, 0, 0, 0x01, 0x68];
    cd.extend_from_slice(&obj.to_le_bytes());
    Rec {
        vi: 0x000F,
        ty: 0xF004,
        body: Body::Cont(vec![
            Rec::atom((1 << 4) | 2, 0xF00A, fsp),
            encode_props(3, 0xF00B, &list, &data),
            props(3, 0xF122, &t),
            anchor_atom(anchor),
            Rec::atom(0x018a, 0xF011, cd),
        ]),
    }
}

/// The picture's store index when `r` is a picture shape container.
fn picture_index(r: &Rec) -> Option<u32> {
    let (list, _) = prop_table(r.find(0xF00B)?).ok()?;
    list.iter()
        .find(|p| p.0 & 0x3FFF == 0x104)
        .map(|p| p.1)
        .filter(|&v| v > 0)
}

// --------------------------------------------------------------------- Contents

/// The kind-1 frame object of a picture (spec 4C.1.5, 50 bytes).
fn frame_body(info: u32, width_emu: u32, height_emu: u32) -> Vec<u8> {
    let mut b = vec![0u8; 4];
    b.extend_from_slice(&[0x02, 0x08]);
    b.extend_from_slice(&[0x0c, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x0d, 0x28, 0, 0, 0, 0, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x34, 0x20, 0, 0, 0, 0]);
    b.extend_from_slice(&[0x3a, 0x70]);
    b.extend_from_slice(&info.to_le_bytes());
    b.extend_from_slice(&[0xaa, 0x20]);
    b.extend_from_slice(&width_emu.to_le_bytes());
    b.extend_from_slice(&[0xab, 0x20]);
    b.extend_from_slice(&height_emu.to_le_bytes());
    let n = b.len() as u32;
    b[..4].copy_from_slice(&n.to_le_bytes());
    b
}

/// The current time as a Windows FILETIME (100 ns ticks since 1601).
fn now_filetime() -> u64 {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (unix + 11_644_473_600) * 10_000_000
}

/// The kind-102 picture-info object of an embedded picture (spec 4C.1.5).
fn info_body(s: &PictureSource) -> Vec<u8> {
    let ft = s.modified.filter(|&t| t != 0).unwrap_or_else(now_filetime);
    let mut time = vec![16, 0, 0, 0];
    time.extend_from_slice(&[0x01, 0x20]);
    time.extend_from_slice(&(ft as u32).to_le_bytes());
    time.extend_from_slice(&[0x02, 0x20]);
    time.extend_from_slice(&((ft >> 32) as u32).to_le_bytes());
    let mut file = vec![0u8; 4];
    file.extend_from_slice(&[0x02, 0x20]);
    file.extend_from_slice(&(s.image.len() as u32).to_le_bytes());
    file.extend_from_slice(&[0x03, 0x88]);
    file.extend_from_slice(&time);
    let n = file.len() as u32;
    file[..4].copy_from_slice(&n.to_le_bytes());
    let name = utf16z(base_name(s.file_name));
    let mut b = vec![0u8; 4];
    b.extend_from_slice(&[0x02, 0x88]);
    b.extend_from_slice(&file);
    b.extend_from_slice(&[0x03, 0xc0]);
    b.extend_from_slice(&((4 + name.len()) as u32).to_le_bytes());
    b.extend_from_slice(&name);
    let n = b.len() as u32;
    b[..4].copy_from_slice(&n.to_le_bytes());
    b
}

/// (header position, payload position, end) of top-level item `id` of object `slot`.
fn item_span(cts: &Cts, slot: usize, id: u16) -> Result<Option<(usize, usize, usize)>> {
    let off = cts.offset(slot)?;
    Ok(items_of(&cts.blocks, off)?
        .into_iter()
        .find(|i| i.0 == id)
        .map(|i| (i.1, i.2, i.3)))
}

fn add_delta(len: u32, delta: isize) -> u32 {
    (len as isize + delta) as u32
}

/// Remove the blocks of `slots` and empty their index slots; later offsets follow. With a
/// font table, empty slots directly below it close up, and trailing empty slots are trimmed
/// (spec 4C.1.7, as `delete_page` does).
pub(crate) fn remove_objects(cts: &mut Cts, slots: &[usize]) -> Result<()> {
    let mut blocks = Vec::new();
    for &g in slots {
        let off = cts.offset(g)?;
        let len = rd(&cts.blocks, off)? as usize;
        if off + len > cts.blocks.len() {
            return fail("object block past the stream");
        }
        blocks.push((off, len));
    }
    for &g in slots {
        cts.entries[g] = EMPTY_SLOT.to_vec();
    }
    blocks.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
    for (off, len) in blocks {
        cts.splice(off, len, &[])?;
    }
    if let Some(mut font) = cts.font_slot_any().filter(|&f| f + 1 == cts.entries.len()) {
        while font > 0 && cts.entries[font - 1] == EMPTY_SLOT {
            let fp = cts.parent(font)?;
            cts.entries.remove(font - 1);
            cts.repoint(fp, font, font - 1)?;
            font -= 1;
        }
    }
    while cts.entries.last().is_some_and(|e| *e == EMPTY_SLOT) {
        cts.entries.pop();
    }
    Ok(())
}

/// The ids in an overlap list (item 71): entries are class-13 ints (`00 68 <u32>`), unlike
/// the class-14 entries of a page's child list.
fn overlap_ids(d: &[u8], payload: usize, end: usize) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    let mut pos = payload + 4;
    while pos < end {
        if end - pos < 6 || d[pos + 1] >> 3 != 13 {
            return fail("unexpected overlap list entry");
        }
        out.push(rd(d, pos + 2)?);
        pos += 6;
    }
    Ok(out)
}

fn overlap_list(ids: &[u32]) -> Vec<u8> {
    let mut d = ((4 + 6 * ids.len()) as u32).to_le_bytes().to_vec();
    for id in ids {
        d.extend_from_slice(&[0x00, 0x68]);
        d.extend_from_slice(&id.to_le_bytes());
    }
    d
}

/// Remove `id` from the overlap items (70 count, 71 list) of every other object. Spec 4C.1
/// does not cover deletion; a reference to a vanished object is the thing to avoid.
pub(crate) fn drop_overlap_refs(cts: &mut Cts, id: u32) -> Result<()> {
    for slot in 0..cts.entries.len() {
        if cts.kind(slot).is_none() || slot as u32 == id {
            continue;
        }
        let off = cts.offset(slot)?;
        let items = items_of(&cts.blocks, off)?;
        let find = |n: u16| items.iter().find(|i| i.0 == n).copied();
        let (Some(c), Some(l)) = (find(70), find(71)) else {
            continue;
        };
        let ids = overlap_ids(&cts.blocks, l.2, l.3)?;
        if !ids.contains(&id) {
            continue;
        }
        let kept: Vec<u32> = ids.into_iter().filter(|&x| x != id).collect();
        let before = rd(&cts.blocks, off)?;
        if kept.is_empty() {
            let (from, to) = (c.1, l.3);
            cts.splice(from, to - from, &[])?;
            put(&mut cts.blocks, off, add_delta(before, -((to - from) as isize)))?;
        } else {
            put(&mut cts.blocks, c.2, kept.len() as u32)?;
            let new = overlap_list(&kept);
            let delta = new.len() as isize - (l.3 - l.2) as isize;
            cts.splice(l.2, l.3 - l.2, &new)?;
            put(&mut cts.blocks, off, add_delta(before, delta))?;
        }
    }
    Ok(())
}

/// Install `contents`, `escher` and `delay` (creating the delay stream if the file lacks it).
fn install_all(input: &[u8], contents: &[u8], escher: &[u8], delay: &[u8]) -> Result<Vec<u8>> {
    let out = install(input, contents, escher)?;
    let mut cf = CompoundFile::open(Cursor::new(out))?;
    if !cf.is_stream(DELAY_PATH) {
        cf.create_stream(DELAY_PATH)?;
    }
    let mut s = cf.open_stream(DELAY_PATH)?;
    s.set_len(delay.len() as u64)?;
    s.seek(SeekFrom::Start(0))?;
    s.write_all(delay)?;
    drop(s);
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}

// ------------------------------------------------------------------- public API

/// Embed a PNG or JPEG as a new picture on page `page_index` (0-based). The frame is in
/// points from the page's top-left corner. Returns the new file and the picture's `Contents`
/// object number (`Shape::object_number`). The same image bytes already in the file share
/// one store entry and delay record (its reference count goes up).
pub fn add_picture(
    input: &[u8],
    page_index: usize,
    pic: NewPicture,
) -> Result<(Vec<u8>, u32)> {
    let src = pic.source();
    check_source(&src)?;
    let (left_pt, top_pt, width_pt, height_pt) = (pic.left_pt, pic.top_pt, pic.width_pt, pic.height_pt);
    if ![left_pt, top_pt, width_pt, height_pt].iter().all(|v| v.is_finite())
        || width_pt <= 0.0
        || height_pt <= 0.0
    {
        return fail("the frame must be finite with a positive size");
    }
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .pages
        .get(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let emu = |v: f64| (v * EMU_PER_POINT).round();
    let (cx, cy) = (
        f64::from(doc.page_width_emu) / 2.0,
        f64::from(doc.page_height_emu) / 2.0,
    );
    let side = |v: f64| {
        let r = v.round();
        i32::try_from(r as i64).map_err(|_| WriteError::OutOfRange(r as i64))
    };
    let anchor = [
        side(emu(left_pt) - cx)?,
        side(emu(top_pt) - cy)?,
        side(emu(left_pt + width_pt) - cx)?,
        side(emu(top_pt + height_pt) - cy)?,
    ];
    let to_u32 = |v: f64| u32::try_from(v as i64).map_err(|_| WriteError::OutOfRange(v as i64));
    let (width_emu, height_emu) = (to_u32(emu(width_pt))?, to_u32(emu(height_pt))?);

    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let mut delay = streams.delay.clone();
    let page_obj = page.object_number as usize;
    if cts.kind(page_obj) != Some(KIND_PAGE) {
        return fail("the page has no page object in Contents");
    }
    let (slot, _) = cts.tail()?;
    let (pic_id, info_id) = (slot as u32, slot as u32 + 1);

    // ---- Escher: store entry and delay record, then the shape container.
    let pib = acquire_blip(&mut esc.head, &mut delay, &src)?;
    let page_objs: HashSet<u32> = page.shapes.iter().map(|s| s.object_number).collect();
    let di = page_drawing(&esc, &page_objs)?;
    let drawing = &mut esc.drawings[di].1;
    let dgid = drawing.find(0xF008).map_or(0, |r| r.vi >> 4);
    let spid = match drawing.find(0xF008).map(|r| &r.body) {
        Some(Body::Atom(d)) => rd(d, 4)? + 1,
        _ => return fail("the drawing has no 0xF008"),
    };
    let container = picture_container(spid, pib, stem(pic.file_name), anchor, pic_id);
    drawing_counters(drawing, 1, Some(spid))?;
    let group = drawing
        .kids_mut()?
        .iter_mut()
        .find(|r| r.ty == 0xF003)
        .ok_or_else(|| WriteError::Picture("the drawing has no page group".into()))?;
    group.kids_mut()?.push(container);
    group_counters(&mut esc.head, dgid, 1, 1)?;

    // ---- Contents: items 70/71 on older text boxes it overlaps (one observed case, no
    // item 45; spec 4C.1.5), then the two objects in front of the font table.
    let [l, tp, r, b] = anchor;
    for s in &page.shapes {
        if s.kind == KIND_TEXT_BOX
            && s.chain_story.is_some()
            && s.group.is_none()
            && s.left_emu < r
            && s.right_emu > l
            && s.top_emu < b
            && s.bottom_emu > tp
        {
            add_overlap_with(&mut cts, s.object_number as usize, pic_id, false)?;
        }
    }
    let (font_or_end, at) = cts.tail()?;
    let frame = frame_body(info_id, width_emu, height_emu);
    let info = info_body(&src);
    let mut both = frame.clone();
    both.extend_from_slice(&info);
    cts.blocks.splice(at..at, both.iter().copied());
    cts.shift(at, true, both.len() as isize)?;
    cts.insert_entries(
        font_or_end,
        vec![
            index_entry(KIND_FRAME as u16, page_obj as u32, 258, false, 1),
            index_entry(KIND_PICTURE_INFO, pic_id, 258, false, 1),
        ],
    )?;
    cts.set_int(slot, 4, at)?;
    cts.set_int(slot, 5, page_obj)?;
    cts.set_int(slot + 1, 4, at + frame.len())?;
    cts.set_int(slot + 1, 5, slot)?;
    let mut kids = cts.child_ids(page_obj)?;
    kids.push(pic_id);
    cts.set_child_ids(page_obj, &kids)?;

    let out = install_all(input, &cts.finish()?, &esc.to_bytes(), &delay)?;
    Ok((out, pic_id))
}

/// "Change Picture": give picture `object_id` (a `Shape::object_number`) another image,
/// keeping its frame (spec 4C.1.6). The old store entry loses a reference (an empty
/// placeholder when it was the last), the new image gets a store entry, the old picture-info
/// object is removed and a new one takes the next free id; the frame object gains item 3.
/// `Contents` items 170/171 are made to match the Escher anchor.
pub fn replace_picture(input: &[u8], object_id: u32, new: PictureSource) -> Result<Vec<u8>> {
    check_source(&new)?;
    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let mut delay = streams.delay.clone();
    let pic = object_id as usize;
    let old_info = frame_info(&cts, pic)?
        .ok_or_else(|| WriteError::Picture("the picture has no picture-info object".into()))?;
    if cts.kind(old_info) != Some(u64::from(KIND_PICTURE_INFO)) {
        return fail("the picture's info object is not a kind-102 object");
    }

    // ---- Escher: new store entry, release the old, edit the shape's properties.
    let (di, path) = find_picture_shape(&esc, object_id)?;
    let old_pib = picture_index(rec_at(&mut esc.drawings[di].1, &path)?)
        .ok_or_else(|| WriteError::Picture("the shape is not a picture".into()))?;
    let new_pib = acquire_blip(&mut esc.head, &mut delay, &new)?;
    release_blip(&mut esc.head, old_pib)?;
    compact_store(&mut esc.head, &mut delay)?;
    let shape = rec_at(&mut esc.drawings[di].1, &path)?;
    let anchor = match shape.find(0xF010).map(|r| &r.body) {
        Some(Body::Atom(d)) => client_anchor(d)?,
        _ => return fail("the picture has no client anchor"),
    };
    {
        let table = shape
            .kids_mut()?
            .iter_mut()
            .find(|r| r.ty == 0xF00B)
            .ok_or_else(|| WriteError::Picture("the picture has no property table".into()))?;
        let (mut list, mut data) = prop_table(table)?;
        if let Some(p) = list.iter_mut().find(|p| p.0 & 0x3FFF == 0x104) {
            p.1 = new_pib;
        }
        drop_prop(&mut list, &mut data, 0x106);
        set_name(&mut list, &mut data, stem(new.file_name));
        *table = encode_props(table.vi & 0xF, 0xF00B, &list, &data);
    }

    // ---- Contents: old info out, new info in the next free slot, item 58, item 3, 170/171.
    let off = cts.offset(old_info)?;
    let len = rd(&cts.blocks, off)? as usize;
    cts.entries[old_info] = EMPTY_SLOT.to_vec();
    cts.splice(off, len, &[])?;
    let (slot, at) = cts.tail()?;
    let body = info_body(&new);
    cts.blocks.splice(at..at, body.iter().copied());
    cts.shift(at, true, body.len() as isize)?;
    cts.insert_entries(slot, vec![index_entry(KIND_PICTURE_INFO, pic as u32, 258, false, 1)])?;
    cts.set_int(slot, 4, at)?;
    cts.set_int(slot, 5, pic)?;
    let (_, p58, _) = item_span(&cts, pic, 58)?
        .ok_or_else(|| WriteError::Picture("the picture lost item 58".into()))?;
    put(&mut cts.blocks, p58, slot as u32)?;
    // Item 3 ("replaced", observed once per replace): a flag between items 2 and 12.
    if item_span(&cts, pic, 3)?.is_none() {
        let off = cts.offset(pic)?;
        let at = items_of(&cts.blocks, off)?
            .iter()
            .find(|i| i.0 > 3)
            .map(|i| i.1)
            .ok_or_else(|| WriteError::Picture("the picture has no item after 3".into()))?;
        let ins = pub_format::items::header(3, 1);
        cts.splice(at, 0, &ins)?;
        grow_block(&mut cts, off, ins.len())?;
    }
    // Publisher leaves 170/171 stale after a replace (spec 4C.1.5); keep them equal to the anchor.
    let (w, h) = (
        anchor[2].wrapping_sub(anchor[0]) as u32,
        anchor[3].wrapping_sub(anchor[1]) as u32,
    );
    for (id, v) in [(170u16, w), (171, h)] {
        if let Some((_, p, e)) = item_span(&cts, pic, id)? {
            if e - p == 4 {
                put(&mut cts.blocks, p, v)?;
            }
        }
    }

    install_all(input, &cts.finish()?, &esc.to_bytes(), &delay)
}

/// Delete picture `object_id` (a `Shape::object_number`) from its page (spec 4C.1.7).
pub fn delete_picture(input: &[u8], object_id: u32) -> Result<Vec<u8>> {
    let streams = Streams::from_cfb(input)?;
    let mut esc = EscherDoc::parse(&streams.escher)?;
    let mut cts = Cts::parse(&streams.contents)?;
    let mut delay = streams.delay.clone();
    let pic = object_id as usize;
    let info = frame_info(&cts, pic)?;
    let page = cts.parent(pic)?;
    if cts.kind(page) != Some(KIND_PAGE) {
        return fail("only a picture directly on a page can be deleted");
    }

    // ---- Escher: the container goes, counters drop (lastspid and FIDCL stay), store follows.
    let (di, path) = find_picture_shape(&esc, object_id)?;
    let pib = picture_index(rec_at(&mut esc.drawings[di].1, &path)?)
        .ok_or_else(|| WriteError::Picture("the shape is not a picture".into()))?;
    let (&last, parent_path) = path.split_last().expect("path is never empty");
    rec_at(&mut esc.drawings[di].1, parent_path)?
        .kids_mut()?
        .remove(last);
    let dgid = esc.drawings[di].1.find(0xF008).map_or(0, |r| r.vi >> 4);
    drawing_counters(&mut esc.drawings[di].1, -1, None)?;
    group_counters(&mut esc.head, dgid, -1, 0)?;
    release_blip(&mut esc.head, pib)?;
    compact_store(&mut esc.head, &mut delay)?;

    // ---- Contents: page list, overlap references, both objects.
    drop_overlap_refs(&mut cts, object_id)?;
    let mut ids = cts.child_ids(page)?;
    ids.retain(|&x| x != object_id);
    let both_items = {
        let off = cts.offset(page)?;
        let items = items_of(&cts.blocks, off)?;
        (items.iter().find(|i| i.0 == 1).copied(), items.iter().find(|i| i.0 == 2).copied())
    };
    match both_items {
        // An empty list removes items 1 and 2 altogether (the page is then the blank's).
        (Some(a), Some(b)) if ids.is_empty() && a.3 == b.1 => {
            let off = cts.offset(page)?;
            let before = rd(&cts.blocks, off)?;
            cts.splice(a.1, b.3 - a.1, &[])?;
            put(&mut cts.blocks, off, add_delta(before, -((b.3 - a.1) as isize)))?;
        }
        _ => cts.set_child_ids(page, &ids)?,
    }
    let mut gone = vec![pic];
    gone.extend(info);
    remove_objects(&mut cts, &gone)?;

    install_all(input, &cts.finish()?, &esc.to_bytes(), &delay)
}

/// The info object id (item 58) of a picture's frame object.
fn frame_info(cts: &Cts, pic: usize) -> Result<Option<usize>> {
    if cts.kind(pic) != Some(KIND_FRAME) {
        return fail(format!("object {pic} is not a picture frame"));
    }
    item_span(cts, pic, 58)?
        .map(|(_, p, _)| rd(&cts.blocks, p).map(|v| v as usize))
        .transpose()
}

/// Drawing index and path of the picture shape container of Contents object `id`.
fn find_picture_shape(esc: &EscherDoc, id: u32) -> Result<(usize, Vec<usize>)> {
    for (di, (_, d)) in esc.drawings.iter().enumerate() {
        let pred = |r: &Rec| r.ty == 0xF004 && r.object() == Some(id);
        if let Some(p) = find_path(d.kids(), &pred) {
            // `find_path` is relative to the drawing's children; `rec_at` wants the same.
            return Ok((di, p));
        }
    }
    fail(format!("no shape for object {id}"))
}
