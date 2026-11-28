//! Writer for Microsoft Publisher 2003-2010 `.pub` files (milestone 1): compound-file
//! round-trip and in-place patching of a shape's frame. Written from
//! `docs/spec/PUB-FORMAT-SPEC.md` (sections 1, 2, 4.2, 4A, 4.5) and the public [MS-CFB] spec.

mod blank;
mod counter;
mod duplicate;
mod group;
mod masters;
mod md4;
mod pagesetup;
mod pages;
mod picture;
mod quill;
mod sections;
mod shape;
mod style;
mod table;
mod textbox;
mod wrapcols;

pub use blank::{new_publication, new_publication_with, NewPublication, Options as BlankOptions};
pub use counter::bump_save_counter;
pub use duplicate::duplicate_page;
pub use masters::{
    add_master, delete_master, rename_master, set_master_margins, set_page_masters, PageMaster,
};
pub use pages::{add_page, delete_page, move_page};
pub use sections::set_sections;
pub use pagesetup::{set_margins, set_page_size};
pub use group::{group_shapes, ungroup, ungroup_with};
pub use picture::{
    add_picture, delete_picture, replace_picture, NewPicture, PictureKind, PictureSource,
};
pub use shape::{add_shape, NewShapeKind, ShapeStyle};
pub use style::{set_shape_style, ShapeLook};
pub use table::{
    add_table, add_table_tracks_emu, delete_table, delete_table_columns, delete_table_rows, insert_table_columns,
    insert_table_rows, replace_table, NewTable, MAX_CELLS, MIN_COLUMN_EMU, MIN_ROW_EMU,
};
pub use textbox::add_text_box;
pub use wrapcols::{set_shape_wrap, set_text_columns};

use cfb::CompoundFile;
use pub_format::{IntSlot, PubError, Streams};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use thiserror::Error;

/// EMU per typographic point.
pub const EMU_PER_POINT: f64 = 12_700.0;

const KIND_TEXT_BOX: u16 = 1;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot read the input: {0}")]
    Read(#[from] PubError),
    #[error("page {0} does not exist")]
    NoSuchPage(usize),
    #[error("shape {shape} does not exist on page {page}")]
    NoSuchShape { page: usize, shape: usize },
    #[error("the shape's group is rotated ({0} degrees); moving its children is not supported")]
    Rotated(f32),
    #[error("the shape has no editable `{0}` field of 4 bytes in the file")]
    MissingField(&'static str),
    #[error("value {0} does not fit in 32 bits")]
    OutOfRange(i64),
    #[error("story {index} does not exist (the document has {count} stories)")]
    NoSuchStory { index: usize, count: usize },
    #[error("story {0} is a table story (TCD cell boundaries); use set_table_text")]
    TableStory(usize),
    #[error("unsupported Quill stream: {0}")]
    Quill(String),
    #[error("cannot group/ungroup: {0}")]
    Group(String),
    #[error("cannot add a text box: {0}")]
    TextBox(String),
    #[error("cannot change the pages: {0}")]
    Pages(String),
    #[error("cannot add a shape: {0}")]
    Shape(String),
    #[error("cannot change the table: {0}")]
    Table(String),
    #[error("cannot change the picture: {0}")]
    Picture(String),
    #[error("cannot change the page setup: {0}")]
    PageSetup(String),
    #[error("cannot change the master pages: {0}")]
    Master(String),
    #[error("cannot create the publication: {0}")]
    Blank(String),
    #[error("cannot duplicate the page: {0}")]
    Unsupported(String),
}

pub type Result<T> = std::result::Result<T, WriteError>;

/// Rewrite the compound file into a new one holding every storage and stream with
/// identical bytes, the same storage CLSIDs and state bits, and the same tree.
pub fn rewrite(input: &[u8]) -> Result<Vec<u8>> {
    let mut src = CompoundFile::open(Cursor::new(input))?;
    let mut dst = CompoundFile::create_with_version(src.version(), Cursor::new(Vec::new()))?;
    let root = src.root_entry();
    dst.set_storage_clsid("/", *root.clsid())?;
    dst.set_state_bits("/", root.state_bits())?;
    // `walk` is pre-order over the directory tree, so parents precede children.
    let entries: Vec<_> = src
        .walk()
        .filter(|e| !e.is_root())
        .map(|e| {
            (
                e.path().to_path_buf(),
                e.is_stream(),
                *e.clsid(),
                e.state_bits(),
            )
        })
        .collect();
    for (path, is_stream, clsid, bits) in entries {
        if is_stream {
            let mut buf = Vec::new();
            src.open_stream(&path)?.read_to_end(&mut buf)?;
            dst.create_stream(&path)?.write_all(&buf)?;
        } else {
            dst.create_storage(&path)?;
            dst.set_storage_clsid(&path, clsid)?;
        }
        dst.set_state_bits(&path, bits)?;
    }
    dst.flush()?;
    Ok(dst.into_inner().into_inner())
}

/// Move and resize one shape without changing any stream length. Values are EMU; `left`
/// and `top` are measured from the **page centre** (spec 4A), as stored, and describe the
/// shape's visual (unrotated) box, the same box `pub_format::Shape` reports.
///
/// * Plain shapes: the anchor's right and bottom become left + width and top + height;
///   text boxes (kind 1) also get `Contents` items 170/171 = width/height (unrotated size,
///   spec 4H) and the story's `MCLD` frame record items 4/5 (spec 4G) when it can be
///   matched to this box.
/// * Rotated shapes: for rotations in [45, 135) and [225, 315) the anchor is written with
///   width and height swapped about the box centre (spec 4H).
/// * Children of a group (spec 4J): the child anchor (0xF00F) is written in the group's
///   coordinate box, mapped back from page space; afterwards the group's own anchor and
///   coordinate box follow the union of its children (see `refit_group`). Rotated groups are
///   refused.
pub fn set_shape_frame(
    input: &[u8],
    page_index: usize,
    shape_index: usize,
    left_emu: i32,
    top_emu: i32,
    width_emu: u32,
    height_emu: u32,
) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .surface(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let shape = page
        .shapes
        .get(shape_index)
        .ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?;
    let streams = Streams::from_cfb(input)?;
    let slots =
        pub_format::shape_slots(&streams, shape.object_number)?.ok_or(WriteError::NoSuchShape {
            page: page_index,
            shape: shape_index,
        })?;
    let right = i32::try_from(i64::from(left_emu) + i64::from(width_emu))
        .map_err(|_| WriteError::OutOfRange(i64::from(left_emu) + i64::from(width_emu)))?;
    let bottom = i32::try_from(i64::from(top_emu) + i64::from(height_emu))
        .map_err(|_| WriteError::OutOfRange(i64::from(top_emu) + i64::from(height_emu)))?;

    // The stored rectangle: the visual box, with width and height swapped about its centre
    // for rotations of about a quarter turn (spec 4H).
    let stored: [i64; 4] = if swapped_extent(slots.rotation_deg) {
        let cx2 = i64::from(left_emu) + i64::from(right);
        let cy2 = i64::from(top_emu) + i64::from(bottom);
        let (w, h) = (i64::from(width_emu), i64::from(height_emu));
        let (l, t) = ((cx2 - h).div_euclid(2), (cy2 - w).div_euclid(2));
        [l, t, l + h, t + w]
    } else {
        [
            i64::from(left_emu),
            i64::from(top_emu),
            i64::from(right),
            i64::from(bottom),
        ]
    };
    let to_i32 = |v: i64| i32::try_from(v).map_err(|_| WriteError::OutOfRange(v));

    let mut escher_patches = Vec::new();
    if let Some(child) = slots.child {
        if child.group_rotation != 0.0 {
            return Err(WriteError::Rotated(child.group_rotation));
        }
        let (iw, ih) = (
            child.inner[2] - child.inner[0],
            child.inner[3] - child.inner[1],
        );
        let (rw, rh) = (child.rect[2] - child.rect[0], child.rect[3] - child.rect[1]);
        if rw == 0.0 || rh == 0.0 {
            return Err(WriteError::MissingField("group rectangle"));
        }
        let (sx, sy) = (iw / rw, ih / rh);
        let back = |v: i64, origin: f64, inner0: f64, scale: f64| -> Result<u32> {
            let g = (inner0 + (v as f64 - origin) * scale).round();
            Ok(to_i32(g as i64)? as u32)
        };
        let g = [
            back(stored[0], child.rect[0], child.inner[0], sx)?,
            back(stored[1], child.rect[1], child.inner[1], sy)?,
            back(stored[2], child.rect[0], child.inner[0], sx)?,
            back(stored[3], child.rect[1], child.inner[1], sy)?,
        ];
        for (i, v) in g.iter().enumerate() {
            escher_patches.push((child.offset + 4 * i, *v));
        }
    } else {
        let names = ["anchor left", "anchor top", "anchor right", "anchor bottom"];
        for (i, slot) in slots.anchor.iter().enumerate() {
            escher_patches.push((slot4(*slot, names[i])?, to_i32(stored[i])? as u32));
        }
    }
    let mut contents_patches = Vec::new();
    let mut quill_patches = Vec::new();
    if slots.kind == KIND_TEXT_BOX {
        contents_patches.push((slot4(slots.width, "width (item 170)")?, width_emu));
        contents_patches.push((slot4(slots.height, "height (item 171)")?, height_emu));
        if let Some([w, h]) = mcld_slots(&doc, &streams, page_index, shape_index)? {
            quill_patches.push((w, width_emu));
            quill_patches.push((h, height_emu));
        }
    }

    let mut cf = CompoundFile::open(Cursor::new(input.to_vec()))?;
    patch(&mut cf, "/Escher/EscherStm", &escher_patches)?;
    patch(&mut cf, "/Contents", &contents_patches)?;
    patch(&mut cf, "/Quill/QuillSub/CONTENTS", &quill_patches)?;
    cf.flush()?;
    let out = cf.into_inner().into_inner();
    match shape.group {
        Some(group) if slots.child.is_some() => refit_group(out, group),
        _ => Ok(out),
    }
}

/// Visual bounding box (centre-relative EMU) of a shape: its unrotated frame turned about
/// its centre by the total rotation.
fn bounding_box(s: &pub_format::Shape) -> [f64; 4] {
    let f = [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu].map(f64::from);
    if s.rotation_deg == 0.0 {
        return f;
    }
    let (cx, cy) = ((f[0] + f[2]) / 2.0, (f[1] + f[3]) / 2.0);
    let (hw, hh) = ((f[2] - f[0]) / 2.0, (f[3] - f[1]) / 2.0);
    let (sin, cos) = f64::from(s.rotation_deg).to_radians().sin_cos();
    let (ex, ey) = (
        hw * cos.abs() + hh * sin.abs(),
        hw * sin.abs() + hh * cos.abs(),
    );
    [cx - ex, cy - ey, cx + ex, cy + ey]
}

/// After a child of group `group` moved: set the group's client anchor to the union of its
/// children's page rectangles and extend its coordinate box (FSPGR) by the same amount at
/// the group's own scale, so every child anchor keeps mapping to the same page position
/// (spec 4J; Publisher does not recompute group bounds on open). Nested and rotated groups
/// are left alone.
fn refit_group(bytes: Vec<u8>, group: u32) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(&bytes)?;
    let shapes: Vec<&pub_format::Shape> = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .chain(doc.masters.iter().flat_map(|m| &m.shapes))
        .collect();
    let Some(g) = shapes.iter().find(|s| s.object_number == group) else {
        return Ok(bytes);
    };
    if g.group.is_some() || g.rotation_deg != 0.0 {
        return Ok(bytes);
    }
    let mut union: Option<[f64; 4]> = None;
    for b in shapes
        .iter()
        .filter(|s| s.group == Some(group))
        .map(|s| bounding_box(s))
    {
        union = Some(match union {
            None => b,
            Some(u) => [
                u[0].min(b[0]),
                u[1].min(b[1]),
                u[2].max(b[2]),
                u[3].max(b[3]),
            ],
        });
    }
    let Some(new) = union.map(|u| u.map(|v| v.round() as i64)) else {
        return Ok(bytes);
    };
    let streams = Streams::from_cfb(&bytes)?;
    let Some(slots) = pub_format::shape_slots(&streams, group)? else {
        return Ok(bytes);
    };
    let Some(box_at) = slots.group_box else {
        return Ok(bytes);
    };
    let mut anchor_at = [0usize; 4];
    for (i, slot) in slots.anchor.iter().enumerate() {
        match slot {
            Some(s) if s.len == 4 => anchor_at[i] = s.offset,
            _ => return Ok(bytes),
        }
    }
    let read = |at: usize| -> Option<i64> {
        let b = streams.escher.get(at..at + 4)?;
        Some(i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])))
    };
    let (Some(old), Some(inner)) = (
        anchor_at.map(read).into_iter().collect::<Option<Vec<_>>>(),
        (0..4)
            .map(|i| read(box_at + 4 * i))
            .collect::<Option<Vec<_>>>(),
    ) else {
        return Ok(bytes);
    };
    if old.iter().zip(&new).all(|(o, n)| (o - n).abs() <= 1) {
        return Ok(bytes);
    }
    let (rw, rh) = ((old[2] - old[0]) as f64, (old[3] - old[1]) as f64);
    if rw == 0.0 || rh == 0.0 {
        return Ok(bytes);
    }
    let (kx, ky) = (
        ((inner[2] - inner[0]) as f64) / rw,
        ((inner[3] - inner[1]) as f64) / rh,
    );
    let k = [kx, ky, kx, ky];
    let to_u32 = |v: i64| -> Result<u32> {
        Ok(i32::try_from(v).map_err(|_| WriteError::OutOfRange(v))? as u32)
    };
    let mut patches = Vec::new();
    for i in 0..4 {
        patches.push((anchor_at[i], to_u32(new[i])?));
        let moved = inner[i] + (((new[i] - old[i]) as f64) * k[i]).round() as i64;
        patches.push((box_at + 4 * i, to_u32(moved)?));
    }
    let mut cf = CompoundFile::open(Cursor::new(bytes))?;
    patch(&mut cf, "/Escher/EscherStm", &patches)?;
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}

/// Rotations whose stored anchor has width and height swapped (spec 4H).
fn swapped_extent(rotation_deg: f32) -> bool {
    (45.0..135.0).contains(&rotation_deg) || (225.0..315.0).contains(&rotation_deg)
}

/// Where the width/height of this text box live in its story's `MCLD` frame record
/// (spec 4G): the story group is the box's story number ranked among all story numbers
/// (as the reader does), the frame is its place in the chain. `None` when the box cannot be
/// matched to a frame (no story, a table, a frame count different from the chain length).
fn mcld_slots(
    doc: &pub_format::Document,
    streams: &Streams,
    page: usize,
    shape: usize,
) -> Result<Option<[usize; 2]>> {
    Ok(mcld_item_slots(doc, streams, page, shape, [0x204, 0x205])?
        .filter(|s| s.iter().all(|x| x.1 == 4))
        .map(|s| [s[0].0, s[1].0]))
}

/// Like [`mcld_slots`] for any integer items: (offset, byte length) of each.
fn mcld_item_slots<const N: usize>(
    doc: &pub_format::Document,
    streams: &Streams,
    page: usize,
    shape: usize,
    ids: [u16; N],
) -> Result<Option<[(usize, usize); N]>> {
    let Some(s) = doc.surface_shapes(page).and_then(|p| p.get(shape)) else {
        return Ok(None);
    };
    let Some(story) = s.story else {
        return Ok(None);
    };
    if s.table.is_some() {
        return Ok(None);
    }
    let numbers = doc.story_numbers();
    let Some(group) = numbers.iter().position(|&n| n == story) else {
        return Ok(None);
    };
    let Some(chain) = doc
        .surface_chains()
        .into_iter()
        .find(|c| c.iter().any(|r| r.page == page && r.shape == shape))
    else {
        return Ok(None);
    };
    let frame = chain
        .iter()
        .position(|r| r.page == page && r.shape == shape)
        .unwrap_or(0);
    quill::mcld_frame_slots(&streams.quill, group, frame, chain.len(), ids)
}

/// Same as [`set_shape_frame`] with values in points.
pub fn set_shape_frame_pt(
    input: &[u8],
    page_index: usize,
    shape_index: usize,
    left_pt: f64,
    top_pt: f64,
    width_pt: f64,
    height_pt: f64,
) -> Result<Vec<u8>> {
    let emu = |pt: f64| (pt * EMU_PER_POINT).round() as i64;
    let to_i32 = |v: i64| i32::try_from(v).map_err(|_| WriteError::OutOfRange(v));
    let to_u32 = |v: i64| u32::try_from(v).map_err(|_| WriteError::OutOfRange(v));
    set_shape_frame(
        input,
        page_index,
        shape_index,
        to_i32(emu(left_pt))?,
        to_i32(emu(top_pt))?,
        to_u32(emu(width_pt))?,
        to_u32(emu(height_pt))?,
    )
}

fn slot4(slot: Option<IntSlot>, name: &'static str) -> Result<usize> {
    match slot {
        Some(s) if s.len == 4 => Ok(s.offset),
        _ => Err(WriteError::MissingField(name)),
    }
}

fn patch<F: Read + Write + Seek>(
    cf: &mut CompoundFile<F>,
    path: &str,
    patches: &[(usize, u32)],
) -> Result<()> {
    if patches.is_empty() {
        return Ok(());
    }
    let mut s = cf.open_stream(path)?;
    for &(offset, value) in patches {
        s.seek(SeekFrom::Start(offset as u64))?;
        s.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

/// Reference to a character property record (FDPC) of a story, by index into
/// [`StoryRecords::char_records`] or "the same as the original run covering story offset k".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharRecordRef {
    Original(usize),
    /// The record of the original run that covers this UTF-16 offset of the old story.
    SameAsRunAt(usize),
}

/// Reference to a paragraph property record (FDPP) of a story.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaRecordRef {
    Original(usize),
    /// The record of the original paragraph run covering this UTF-16 offset of the old story.
    SameAsParagraphAt(usize),
}

/// A stretch of new text with one character record. A `\r` is only allowed as the last
/// character of a paragraph's last run; if missing it is appended there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRun {
    pub text: String,
    pub char_record: CharRecordRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewParagraph {
    pub runs: Vec<NewRun>,
    pub para_record: ParaRecordRef,
}

/// A run of an existing story: UTF-16 offsets within the story and a record index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryRun {
    pub start: usize,
    pub end: usize,
    pub record: usize,
}

/// The story's text (UTF-16) with its original property records and run ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryRecords {
    pub text: Vec<u16>,
    /// Distinct raw FDPC records (including their 4-byte length prefix) used by the story.
    pub char_records: Vec<Vec<u8>>,
    /// Distinct raw FDPP records used by the story.
    pub para_records: Vec<Vec<u8>>,
    pub runs: Vec<StoryRun>,
    pub paragraphs: Vec<StoryRun>,
}

fn check_story(input: &[u8], story_index: usize) -> Result<()> {
    if table_of_story(input, story_index)?.is_some() {
        return Err(WriteError::TableStory(story_index));
    }
    Ok(())
}

/// For a table story: `(cell count, TCD instance)`; `None` for other stories. The instance
/// of a story's `TCD ` chunk is the story's 0-based ordinal among all the stories (spec
/// 4E.1.3), i.e. its index here.
fn table_of_story(input: &[u8], story_index: usize) -> Result<Option<(usize, usize)>> {
    let doc = pub_format::read_bytes(input)?;
    if story_index >= doc.stories.len() {
        return Err(WriteError::NoSuchStory {
            index: story_index,
            count: doc.stories.len(),
        });
    }
    let numbers = doc.story_numbers();
    let table_of = |n: u32| {
        doc.pages
            .iter()
            .flat_map(|p| &p.shapes)
            .chain(doc.masters.iter().flat_map(|m| &m.shapes))
            .find(|s| s.story == Some(n) && s.table.is_some())
            .and_then(|s| s.table.as_ref())
    };
    let Some(&n) = numbers.get(story_index) else {
        return Ok(None);
    };
    let Some(t) = table_of(n) else {
        return Ok(None);
    };
    Ok(Some((t.rows as usize * t.columns as usize, story_index)))
}

/// List the original text, runs, paragraphs and distinct property records of a story.
pub fn story_records(input: &[u8], story_index: usize) -> Result<StoryRecords> {
    let streams = Streams::from_cfb(input)?;
    let r = quill::story_records(&streams.quill, story_index)?;
    let conv = |v: &[(usize, usize, usize)]| {
        v.iter()
            .map(|&(start, end, record)| StoryRun { start, end, record })
            .collect()
    };
    Ok(StoryRecords {
        runs: conv(&r.runs),
        paragraphs: conv(&r.paras),
        text: r.text,
        char_records: r.char_records,
        para_records: r.para_records,
    })
}

pub(crate) fn install_quill(input: &[u8], quill: &[u8]) -> Result<Vec<u8>> {
    let mut cf = CompoundFile::open(Cursor::new(input.to_vec()))?;
    {
        let mut s = cf.open_stream("/Quill/QuillSub/CONTENTS")?;
        s.set_len(quill.len() as u64)?;
        s.seek(SeekFrom::Start(0))?;
        s.write_all(quill)?;
    }
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}

fn record_at(runs: &[StoryRun], offset: usize) -> Option<usize> {
    runs.iter()
        .find(|r| offset >= r.start && offset < r.end)
        .map(|r| r.record)
}

/// Character formatting that [`set_story_format`] makes a stretch of text carry.
#[derive(Debug, Clone, PartialEq)]
pub struct CharFormat {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Superscript / subscript (item 0x20F, spec 4S).
    pub script: pub_format::Script,
    pub size_pt: f32,
    /// Font family name; added to the file's font table when it is not there yet.
    pub font: String,
    /// Literal text colour; `None` leaves the colour alone, black removes the colour item.
    pub color: Option<[u8; 3]>,
    /// Colour-scheme text colour, index 0-7 (spec 4D: a colour-list entry with flag 0x08);
    /// wins over `color` when set, so the text follows the document's scheme.
    pub scheme_color: Option<u8>,
}

/// Text `[start, end)` (UTF-16 units of the story) that must carry `format`.
#[derive(Debug, Clone, PartialEq)]
pub struct CharSpan {
    pub start: usize,
    pub end: usize,
    pub format: CharFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Line spacing rule (spec 3.6 item 0x34).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineRule {
    /// Multiple of a line (rule 2, 152,400 EMU per line).
    Multiple(f32),
    /// Exactly this many points (rule 1).
    Exactly(f32),
}

/// Paragraph spacing to write (spec 3.6 items 0x12, 0x13, 0x34).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParaSpacing {
    pub space_before_pt: f32,
    pub space_after_pt: f32,
    /// `None` leaves line spacing alone.
    pub line: Option<LineRule>,
}

/// A paragraph `[start, end)` (including its `\r`) that must have this alignment, and
/// (when given) this spacing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParaSpan {
    pub start: usize,
    pub end: usize,
    pub align: ParaAlign,
    pub spacing: Option<ParaSpacing>,
}

/// Make the formatting of story `story_index` (also a table story) match `chars` and
/// `paras` without changing its text. Bold, italic, underline, size, font and alignment are
/// set absolutely for the spans; text outside every span and all other properties of a
/// record (colour, language, spacing, ...) stay as they are.
pub fn set_story_format(
    input: &[u8],
    story_index: usize,
    chars: &[CharSpan],
    paras: &[ParaSpan],
) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    if story_index >= doc.stories.len() {
        return Err(WriteError::NoSuchStory {
            index: story_index,
            count: doc.stories.len(),
        });
    }
    let streams = Streams::from_cfb(input)?;
    let quill = quill::set_story_format(&streams.quill, story_index, chars, paras)?;
    install_quill(input, &quill)
}

/// A paragraph `[start, end)` (UTF-16 units of the story, including its `\r`) that must be
/// in this list (or in none) with these indents (spec 4R).
#[derive(Debug, Clone, PartialEq)]
pub struct ListSpan {
    pub start: usize,
    pub end: usize,
    /// `None` makes the paragraph plain (a list it had is removed, leaving Publisher's
    /// residue, spec 4R.1).
    pub list: Option<pub_format::ListInfo>,
    pub indents: pub_format::ParaIndents,
}

impl ListSpan {
    /// The paragraph in `list` with the indents applying it gives (left = list indent,
    /// first line = -list indent), or plain with no indents.
    pub fn new(start: usize, end: usize, list: Option<pub_format::ListInfo>) -> Self {
        let indents = list.as_ref().map_or_else(Default::default, |l| {
            pub_format::ParaIndents::for_list(l.list_indent_pt)
        });
        ListSpan {
            start,
            end,
            list,
            indents,
        }
    }
}

/// Make the list formatting and indents of story `story_index` match `spans` (spec 4R)
/// without changing its text or character formatting. Only the list and indent items of the
/// paragraphs in the spans change; new bullet fonts are appended to the font table; every
/// paragraph keeps its own run.
pub fn set_story_lists(input: &[u8], story_index: usize, spans: &[ListSpan]) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    if story_index >= doc.stories.len() {
        return Err(WriteError::NoSuchStory {
            index: story_index,
            count: doc.stories.len(),
        });
    }
    let streams = Streams::from_cfb(input)?;
    let quill = quill::set_story_lists(&streams.quill, story_index, spans)?;
    install_quill(input, &quill)
}

/// Replace the text of story `story_index` by explicit paragraphs, each made of runs that
/// reference the story's original property records (see [`story_records`]). The page count
/// of the character/paragraph formatting tables must not change.
pub fn set_story_runs(
    input: &[u8],
    story_index: usize,
    paragraphs: &[NewParagraph],
) -> Result<Vec<u8>> {
    check_story(input, story_index)?;
    set_story_runs_unchecked(input, story_index, paragraphs, None)
}

fn set_story_runs_unchecked(
    input: &[u8],
    story_index: usize,
    paragraphs: &[NewParagraph],
    fields: Option<&[usize]>,
) -> Result<Vec<u8>> {
    let streams = Streams::from_cfb(input)?;
    let recs = quill::story_records(&streams.quill, story_index)?;
    let (cruns, pruns): (Vec<StoryRun>, Vec<StoryRun>) = (
        recs.runs
            .iter()
            .map(|&(start, end, record)| StoryRun { start, end, record })
            .collect(),
        recs.paras
            .iter()
            .map(|&(start, end, record)| StoryRun { start, end, record })
            .collect(),
    );
    let mut q = Vec::new();
    for p in paragraphs {
        let para = match p.para_record {
            ParaRecordRef::Original(i) => i,
            ParaRecordRef::SameAsParagraphAt(o) => record_at(&pruns, o)
                .ok_or_else(|| WriteError::Quill(format!("no paragraph run at offset {o}")))?,
        };
        let mut runs = Vec::new();
        for r in &p.runs {
            let c = match r.char_record {
                CharRecordRef::Original(i) => i,
                CharRecordRef::SameAsRunAt(o) => record_at(&cruns, o)
                    .ok_or_else(|| WriteError::Quill(format!("no character run at offset {o}")))?,
            };
            runs.push((r.text.encode_utf16().collect(), c));
        }
        q.push(quill::QPara { runs, para });
    }
    let quill = quill::set_story_runs(&streams.quill, story_index, &q, fields)?;
    install_quill(input, &quill)
}

/// Replace the text of story `story_index` (the k-th range of the Quill `STRS` table, i.e.
/// `Document::stories[k]`). Paragraphs in `new_text` are separated by `\n` (a trailing `\n`
/// is just the last paragraph's terminator); each becomes a `\r`-terminated paragraph in
/// `TEXT`. The story keeps the character formatting of its first original character and
/// the paragraph formatting of its first original paragraph (flattening). Table stories are
/// refused. See [`set_story_text_keep_formatting`] to keep mixed formatting.
pub fn set_story_text(input: &[u8], story_index: usize, new_text: &str) -> Result<Vec<u8>> {
    check_story(input, story_index)?;
    set_text_unchecked(input, story_index, new_text)
}

fn set_text_unchecked(input: &[u8], story_index: usize, new_text: &str) -> Result<Vec<u8>> {
    let paragraphs: Vec<NewParagraph> = quill::split_paragraphs(new_text)
        .into_iter()
        .map(|t| NewParagraph {
            runs: vec![NewRun {
                text: t,
                char_record: CharRecordRef::SameAsRunAt(0),
            }],
            para_record: ParaRecordRef::SameAsParagraphAt(0),
        })
        .collect();
    set_story_runs_unchecked(input, story_index, &paragraphs, None)
}

fn utf16_string(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// Like [`set_story_text`] but keeps formatting. Unchanged paragraphs (matched from both
/// ends by text) keep their exact character runs and paragraph record; an edited paragraph
/// keeps the runs of the old paragraph at the same position, mapping unchanged leading and
/// trailing characters to their old runs and giving typed text the style at the edit point;
/// extra new paragraphs inherit from the previous paragraph's last run and paragraph record.
pub fn set_story_text_keep_formatting(
    input: &[u8],
    story_index: usize,
    new_text: &str,
) -> Result<Vec<u8>> {
    check_story(input, story_index)?;
    keep_formatting(input, story_index, new_text, None)
}

/// [`set_story_text_keep_formatting`] for a story that holds page-number fields (spec 4T.3):
/// `fields` are the text offsets (UTF-16 units, paragraph breaks count one) of the fields
/// the story has after the edit, so the file's `TOKN` table follows exactly: cps of other
/// stories move with the length change, the story's own become `fields`, and a story left
/// with no field at all loses its `TOKN` entry. The number of fields in the file may only
/// change to none.
pub fn set_story_text_with_fields(
    input: &[u8],
    story_index: usize,
    new_text: &str,
    fields: &[usize],
) -> Result<Vec<u8>> {
    check_story(input, story_index)?;
    keep_formatting(input, story_index, new_text, Some(fields))
}

/// Replace the text of every cell of a table story (spec 4E). `cells` holds one string per
/// cell in row-major order (exactly rows x columns of them); `\n` separates the paragraphs
/// of a cell, an empty string is an empty cell. The story text is rebuilt with the same
/// formatting rules as [`set_story_text_keep_formatting`] and the `TCD ` cell ends are
/// rewritten to the new cell boundaries.
pub fn set_table_text(input: &[u8], story_index: usize, cells: &[String]) -> Result<Vec<u8>> {
    let Some((count, rank)) = table_of_story(input, story_index)? else {
        return Err(WriteError::Quill(format!(
            "story {story_index} is not a table story"
        )));
    };
    if cells.len() != count {
        return Err(WriteError::Quill(format!(
            "the table has {count} cells, {} given",
            cells.len()
        )));
    }
    let streams = Streams::from_cfb(input)?;
    let tcd = quill::find_tcd(&streams.quill, story_index, count, rank)?;
    let mut paragraphs: Vec<String> = Vec::new();
    let mut ends: Vec<u32> = Vec::with_capacity(count);
    let mut pos = 0usize;
    for cell in cells {
        let norm = cell.replace("\r\n", "\n").replace('\r', "\n");
        for p in norm.split('\n') {
            pos += p.encode_utf16().count() + 1;
            paragraphs.push(p.to_owned());
        }
        ends.push(u32::try_from(pos - 1).map_err(|_| WriteError::OutOfRange(pos as i64))?);
    }
    // The last cell's end is the story length (one past its final `\r`).
    if let Some(last) = ends.last_mut() {
        *last += 1;
    }
    let text = paragraphs.join("\n") + "\n";
    let edited = keep_formatting(input, story_index, &text, None)?;
    let streams = Streams::from_cfb(&edited)?;
    let quill = quill::set_tcd(&streams.quill, tcd, &ends)?;
    install_quill(&edited, &quill)
}

fn keep_formatting(
    input: &[u8],
    story_index: usize,
    new_text: &str,
    fields: Option<&[usize]>,
) -> Result<Vec<u8>> {
    let recs = story_records(input, story_index)?;
    if recs.runs.is_empty() || recs.paragraphs.is_empty() {
        return set_text_unchecked(input, story_index, new_text);
    }
    // Old paragraphs: [start, end) up to and including each CR.
    let mut old: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    for (i, &u) in recs.text.iter().enumerate() {
        if u == 0x0D {
            old.push((start, i + 1));
            start = i + 1;
        }
    }
    if start < recs.text.len() {
        old.push((start, recs.text.len()));
    }
    let old_text: Vec<String> = old
        .iter()
        .map(|&(a, b)| {
            let mut t = &recs.text[a..b];
            if t.last() == Some(&0x0D) {
                t = &t[..t.len() - 1];
            }
            utf16_string(t)
        })
        .collect();
    let new_texts = quill::split_paragraphs(new_text);
    if old.is_empty() {
        return set_text_unchecked(input, story_index, new_text);
    }

    let n_common = old_text.len().min(new_texts.len());
    let mut pre = 0;
    while pre < n_common && old_text[pre] == new_texts[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < n_common - pre
        && old_text[old_text.len() - 1 - suf] == new_texts[new_texts.len() - 1 - suf]
    {
        suf += 1;
    }
    let old_mid = old.len() - pre - suf;

    // The old runs inside [a, b) as (text, record) pieces.
    let pieces = |a: usize, b: usize| -> Vec<(String, usize)> {
        recs.runs
            .iter()
            .filter(|r| r.start < b && r.end > a)
            .map(|r| {
                let (x, y) = (r.start.max(a), r.end.min(b));
                (utf16_string(&recs.text[x..y]), r.record)
            })
            .collect()
    };
    let para_of = |i: usize| ParaRecordRef::SameAsParagraphAt(old[i].0);
    let char_ref = |r: usize| CharRecordRef::Original(r);
    // Style of the old character at `off`, clamped into the paragraph `[a, b)`.
    let style_at = |off: usize, a: usize, b: usize| -> usize {
        let mut o = off.clamp(a, b.saturating_sub(1));
        // Prefer a non-CR character as the style source.
        if recs.text.get(o) == Some(&0x0D) && o > a {
            o -= 1;
        }
        record_at(
            &recs
                .runs
                .iter()
                .map(|r| StoryRun {
                    start: r.start,
                    end: r.end,
                    record: r.record,
                })
                .collect::<Vec<_>>(),
            o,
        )
        .unwrap_or(recs.runs[0].record)
    };

    let mut out: Vec<NewParagraph> = Vec::new();
    let mut last_style: Option<(usize, ParaRecordRef)> = None;
    for (i, text) in new_texts.iter().enumerate() {
        let from_end = new_texts.len() - 1 - i;
        let unchanged_old = if i < pre {
            Some(i)
        } else if from_end < suf {
            Some(old.len() - 1 - from_end)
        } else {
            None
        };
        if let Some(j) = unchanged_old {
            let (a, b) = old[j];
            let runs: Vec<NewRun> = pieces(a, b)
                .into_iter()
                .map(|(t, r)| NewRun {
                    text: t,
                    char_record: char_ref(r),
                })
                .collect();
            out.push(NewParagraph {
                runs,
                para_record: para_of(j),
            });
            // Inherit source for following new paragraphs: the last non-CR style.
            last_style = Some((style_at(b.saturating_sub(2), a, b), para_of(j)));
            continue;
        }
        let k = i - pre;
        if k < old_mid {
            // Edited paragraph aligned with old paragraph `pre + k`.
            let j = pre + k;
            let (a, b) = old[j];
            let body_end = if recs.text[b - 1] == 0x0D { b - 1 } else { b };
            let o: Vec<u16> = recs.text[a..body_end].to_vec();
            let n: Vec<u16> = text.encode_utf16().collect();
            let mut p = 0;
            while p < o.len().min(n.len()) && o[p] == n[p] {
                p += 1;
            }
            let mut q = 0;
            while q < o.len().min(n.len()) - p && o[o.len() - 1 - q] == n[n.len() - 1 - q] {
                q += 1;
            }
            let mut per_char: Vec<usize> = Vec::with_capacity(n.len());
            for c in 0..n.len() {
                let src = if c < p {
                    a + c
                } else if c >= n.len() - q {
                    body_end - (n.len() - c)
                } else {
                    // typed text: style at the edit point (character before it if any)
                    a + p
                        .saturating_sub(usize::from(p > 0))
                        .min(o.len().saturating_sub(1))
                };
                per_char.push(style_at(src, a, b));
            }
            let fallback = style_at(a, a, b);
            let mut runs: Vec<NewRun> = Vec::new();
            let mut cur: Option<(usize, Vec<u16>)> = None;
            for (c, &r) in per_char.iter().enumerate() {
                match &mut cur {
                    Some((cr, buf)) if *cr == r => buf.push(n[c]),
                    _ => {
                        if let Some((cr, buf)) = cur.take() {
                            runs.push(NewRun {
                                text: utf16_string(&buf),
                                char_record: char_ref(cr),
                            });
                        }
                        cur = Some((r, vec![n[c]]));
                    }
                }
            }
            if let Some((cr, buf)) = cur {
                runs.push(NewRun {
                    text: utf16_string(&buf),
                    char_record: char_ref(cr),
                });
            }
            if runs.is_empty() {
                runs.push(NewRun {
                    text: String::new(),
                    char_record: char_ref(fallback),
                });
            }
            let tail_style = per_char.last().copied().unwrap_or(fallback);
            out.push(NewParagraph {
                runs,
                para_record: para_of(j),
            });
            last_style = Some((tail_style, para_of(j)));
        } else {
            // New paragraph: inherit from the previous paragraph, else from the first one.
            let (c, p) = last_style.unwrap_or((recs.runs[0].record, para_of(0)));
            out.push(NewParagraph {
                runs: vec![NewRun {
                    text: text.clone(),
                    char_record: char_ref(c),
                }],
                para_record: p,
            });
        }
    }
    set_story_runs_unchecked(input, story_index, &out, fields)
}

/// Set the stacking order (z-order) of a page's shapes. `order` lists indexes into
/// `Document::pages[page].shapes` from the back-most shape to the front-most; it must be a
/// permutation of all of them.
///
/// Stacking order lives only in `Escher/EscherStm` ([MS-ODRAW]): shapes draw in the order of
/// their shape containers (0xF004) inside their group container (0xF003), first = back.
/// Publisher 2010 changes nothing in `Contents` but a save counter when `ZOrder` is used
/// (verified: Bring to Front / Send to Back only permute the containers; shape ids are kept).
/// So this permutes containers in place without changing any length. In every group
/// container the first child (the group's own shape, or the page's patriarch) stays first,
/// and children are reordered only among their siblings, so a group's members stay inside it
/// and a nested group travels as one unit, ranked by its own shape.
pub fn set_shape_order(input: &[u8], page_index: usize, order: &[usize]) -> Result<Vec<u8>> {
    use std::collections::HashMap;
    let doc = pub_format::read_bytes(input)?;
    let page = doc
        .surface(page_index)
        .ok_or(WriteError::NoSuchPage(page_index))?;
    let n = page.shapes.len();
    let mut seen = vec![false; n];
    for &ix in order {
        if ix >= n || std::mem::replace(&mut seen[ix], true) {
            return Err(WriteError::NoSuchShape {
                page: page_index,
                shape: ix,
            });
        }
    }
    if order.len() != n {
        return Err(WriteError::NoSuchShape {
            page: page_index,
            shape: order.len(),
        });
    }
    let rank: HashMap<u32, usize> = order
        .iter()
        .enumerate()
        .map(|(pos, &ix)| (page.shapes[ix].object_number, pos))
        .collect();

    let streams = Streams::from_cfb(input)?;
    let mut data = streams.escher;
    let bad = |m: &str| WriteError::Quill(format!("EscherStm: {m}"));
    let mut pos = escher_header(&data, 0)
        .map(|h| 8 + h.len)
        .ok_or_else(|| bad("no drawing group"))?;
    while pos < data.len() {
        pos += 4;
        let h = escher_header(&data, pos).ok_or_else(|| bad("truncated drawing"))?;
        let end = pos + 8 + h.len;
        if end > data.len() {
            return Err(bad("drawing extends past the stream"));
        }
        reorder_children(&mut data, pos + 8, end, &rank, 0)?;
        pos = end;
    }
    let mut cf = CompoundFile::open(Cursor::new(input.to_vec()))?;
    {
        let mut s = cf.open_stream("/Escher/EscherStm")?;
        s.seek(SeekFrom::Start(0))?;
        s.write_all(&data)?;
    }
    cf.flush()?;
    Ok(cf.into_inner().into_inner())
}

struct EscherHeader {
    ty: u16,
    container: bool,
    len: usize,
}

fn escher_header(d: &[u8], at: usize) -> Option<EscherHeader> {
    let h = d.get(at..at.checked_add(8)?)?;
    Some(EscherHeader {
        ty: u16::from_le_bytes([h[2], h[3]]),
        container: h[0] & 0x0F == 0x0F,
        len: u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize,
    })
}

/// The `Contents` object of a shape container (0xF004) or, for a group container (0xF003),
/// of its first shape.
fn container_object(d: &[u8], start: usize, end: usize) -> Option<u32> {
    let mut pos = start;
    while pos < end {
        let h = escher_header(d, pos)?;
        let (body, next) = (pos + 8, pos + 8 + h.len);
        if next > end {
            return None;
        }
        if h.ty == 0xF011 && !h.container {
            let block = pub_format::items::decode_block(&d[..next], body).ok()?;
            return pub_format::items::find_int(&block, 1).map(|v| v as u32);
        }
        if h.ty == 0xF004 && h.container {
            return container_object(d, body, next);
        }
        pos = next;
    }
    None
}

fn reorder_children(
    d: &mut [u8],
    start: usize,
    end: usize,
    rank: &std::collections::HashMap<u32, usize>,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Err(WriteError::Quill("EscherStm: nesting too deep".into()));
    }
    // (start, end, type) of each child.
    let mut kids = Vec::new();
    let mut pos = start;
    while pos < end {
        let Some(h) = escher_header(d, pos) else {
            break;
        };
        let next = pos + 8 + h.len;
        if next > end {
            return Err(WriteError::Quill(
                "EscherStm: record extends past its container".into(),
            ));
        }
        kids.push((pos, next, h.ty, h.container));
        pos = next;
    }
    for &(s, e, _, container) in &kids {
        if container {
            reorder_children(d, s + 8, e, rank, depth + 1)?;
        }
    }
    // Only group containers (0xF003) order their children; the leader stays first.
    let is_group = depth > 0 && escher_header(d, start - 8).is_some_and(|h| h.ty == 0xF003);
    if !is_group || kids.len() < 2 {
        return Ok(());
    }
    // Children of other pages (one drawing can hold several pages) keep their slots; this
    // page's children are permuted among theirs.
    let rest = &kids[1..];
    let ranks: Vec<Option<usize>> = rest
        .iter()
        .map(|&(s, e, ty, container)| {
            (container && (ty == 0xF004 || ty == 0xF003))
                .then(|| container_object(d, s + 8, e))
                .flatten()
                .and_then(|o| rank.get(&o).copied())
        })
        .collect();
    let mut mine: Vec<(usize, usize, usize)> = rest
        .iter()
        .zip(&ranks)
        .filter_map(|(&(s, e, ..), r)| r.map(|r| (r, s, e)))
        .collect();
    let sorted_ok = mine.windows(2).all(|w| w[0].0 <= w[1].0);
    if sorted_ok {
        return Ok(());
    }
    mine.sort_by_key(|k| k.0);
    let mut next_mine = mine.iter();
    let mut out = Vec::with_capacity(end - rest[0].0);
    for (&(s, e, ..), r) in rest.iter().zip(&ranks) {
        let &(_, ms, me) = if r.is_some() {
            next_mine.next().expect("one sorted entry per keyed child")
        } else {
            &(0, s, e)
        };
        out.extend_from_slice(&d[ms..me]);
    }
    d[rest[0].0..end].copy_from_slice(&out);
    Ok(())
}
