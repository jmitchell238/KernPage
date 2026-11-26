//! Create a complete blank publication from nothing (spec section 6, "A complete minimal
//! publication"). Everything here is built from the tables of that section; no bytes are
//! taken from any existing file.

use crate::{Result, WriteError};
use cfb::CompoundFile;
use std::io::{Cursor, Write};
use uuid::Uuid;

/// EMU per inch.
const EMU_PER_INCH: u32 = 914_400;

/// What to create: a blank publication of `page_count` pages of the given size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewPublication {
    /// Page width in EMU (914,400 per inch; 12,700 per point).
    pub width_emu: u32,
    /// Page height in EMU.
    pub height_emu: u32,
    /// Number of pages, at least 1.
    pub page_count: u32,
}

impl NewPublication {
    pub fn new(width_emu: u32, height_emu: u32, page_count: u32) -> Self {
        Self { width_emu, height_emu, page_count }
    }
    /// A size given in typographic points (rounded to whole EMU).
    pub fn from_points(width_pt: f64, height_pt: f64, page_count: u32) -> Self {
        let e = |p: f64| (p * 12_700.0).round() as u32;
        Self::new(e(width_pt), e(height_pt), page_count)
    }
    /// A size given in inches.
    pub fn from_inches(width_in: f64, height_in: f64, page_count: u32) -> Self {
        let e = |p: f64| (p * f64::from(EMU_PER_INCH)).round() as u32;
        Self::new(e(width_in), e(height_in), page_count)
    }
    pub fn letter(page_count: u32) -> Self {
        Self::new(7_772_400, 10_058_400, page_count)
    }
}

/// Which reductions of spec 6.8 to apply. The default is the shipped form (H0).
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Include the thumbnail (property 17 of the summary information).
    pub thumbnail: bool,
    /// Include `\x05SummaryInformation` at all.
    pub summary_information: bool,
    /// Include `\x05DocumentSummaryInformation`.
    pub document_summary: bool,
    /// Use the 2021 (larger) Escher default property tables instead of the 2010 form.
    pub escher_2021: bool,
    /// Leave the body of object 292 (page setup) empty.
    pub empty_page_setup: bool,
    /// Add style property 0x258 to every character style (2021 only).
    pub style_0x258: bool,
    /// Omit the four helper pages (269, 272, 275, 279) and their children.
    pub omit_helper_pages: bool,
    /// Omit objects 288, 293, 294, 295.
    pub omit_misc_objects: bool,
    /// Omit the empty `Objects` and `VBA` storages.
    pub omit_empty_storages: bool,
    /// Omit these object numbers (references to them are dropped too).
    pub omit: &'static [u32],
    /// Use the Letter preset form of object 292 for plain Letter portrait.
    pub letter_preset: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            thumbnail: true,
            summary_information: true,
            document_summary: true,
            escher_2021: false,
            empty_page_setup: false,
            style_0x258: false,
            omit_helper_pages: false,
            omit_misc_objects: false,
            omit_empty_storages: false,
            omit: &[],
            letter_preset: true,
        }
    }
}

/// Create a blank publication (spec 6, hypothesis H0) and return the `.pub` file bytes.
pub fn new_publication(spec: &NewPublication) -> Result<Vec<u8>> {
    new_publication_with(spec, &Options::default())
}

/// [`new_publication`] with explicit reductions (for the experiments of spec 6.8/6.9).
#[doc(hidden)]
pub fn new_publication_with(spec: &NewPublication, opt: &Options) -> Result<Vec<u8>> {
    if spec.page_count == 0 {
        return Err(WriteError::Blank("a publication needs at least one page".into()));
    }
    // The Contents guide positions are `size - 0.5 in`; keep sizes where they make sense.
    if spec.width_emu < 2 * 457_200 || spec.height_emu < 2 * 457_200 || spec.width_emu > 100_000_000 || spec.height_emu > 100_000_000 {
        return Err(WriteError::Blank(format!(
            "page size {} x {} EMU is outside 1 in .. about 109 in",
            spec.width_emu, spec.height_emu
        )));
    }
    let contents = contents_stream(spec, opt);
    let quill = quill_stream(opt);
    let escher = escher_stream(opt);

    let mut cf = CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))?;
    let uuid = |s: &str| Uuid::parse_str(s).expect("constant");
    cf.set_storage_clsid("/", uuid("00021201-0000-0000-C000-000000000046"))?;
    let put = |cf: &mut CompoundFile<Cursor<Vec<u8>>>, path: &str, data: &[u8]| -> Result<()> {
        cf.create_stream(path)?.write_all(data)?;
        Ok(())
    };
    put(&mut cf, "/\u{1}CompObj", &compobj_root())?;
    put(&mut cf, "/\u{3}Internal", &[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
    if opt.summary_information {
        put(&mut cf, "/\u{5}SummaryInformation", &summary_information(spec, opt))?;
    }
    if opt.document_summary {
        put(&mut cf, "/\u{5}DocumentSummaryInformation", &document_summary())?;
    }
    put(&mut cf, "/Contents", &contents)?;
    put(&mut cf, "/Envelope", &[])?;
    if !opt.omit_empty_storages {
        cf.create_storage("/Objects")?;
        cf.create_storage("/VBA")?;
    }
    cf.create_storage("/Quill")?;
    cf.create_storage("/Quill/QuillSub")?;
    cf.set_storage_clsid("/Quill/QuillSub", uuid("08C8F6DA-969D-11D1-8E02-00C04FB6FECE"))?;
    put(&mut cf, "/Quill/QuillSub/\u{1}CompObj", &compobj_quill())?;
    put(&mut cf, "/Quill/QuillSub/CONTENTS", &quill)?;
    cf.create_storage("/Escher")?;
    put(&mut cf, "/Escher/EscherStm", &escher)?;
    put(&mut cf, "/Escher/EscherDelayStm", &[])?;
    cf.flush()?;
    let bytes = cf.into_inner().into_inner();

    Ok(bytes)
}

// ---------------------------------------------------------------------------------------
// Item encoding (spec 4.2)

/// A block under construction: items keyed by id, written in ascending id order.
#[derive(Default, Clone)]
pub(crate) struct Blk {
    items: Vec<(u16, u8, Vec<u8>)>,
}

pub(crate) fn hdr(id: u16, class: u8) -> [u8; 2] {
    [id as u8, ((id >> 8) as u8 & 7) | (class << 3)]
}

fn utf16z(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
    v.extend_from_slice(&[0, 0]);
    v
}

impl Blk {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn item(mut self, id: u16, class: u8, payload: &[u8]) -> Self {
        let mut b = hdr(id, class).to_vec();
        b.extend_from_slice(payload);
        self.items.push((id, class, b));
        self
    }
    /// A copy without the top-level object references to the given objects.
    pub(crate) fn without_refs(&self, omit: &dyn Fn(u32) -> bool) -> Blk {
        let items = self.items.iter().filter(|(_, c, b)| !((*c == 13 || *c == 14) && omit(u32::from_le_bytes([b[2], b[3], b[4], b[5]])))).cloned().collect();
        Blk { items }
    }
    pub(crate) fn flag(self, id: u16, v: bool) -> Self {
        self.item(id, if v { 1 } else { 0 }, &[])
    }
    pub(crate) fn u16(self, id: u16, v: u16) -> Self {
        self.item(id, 3, &v.to_le_bytes())
    }
    pub(crate) fn u32(self, id: u16, v: u32) -> Self {
        self.item(id, 4, &v.to_le_bytes())
    }
    pub(crate) fn u64(self, id: u16, v: u64) -> Self {
        self.item(id, 5, &v.to_le_bytes())
    }
    pub(crate) fn zero16(self, id: u16) -> Self {
        self.item(id, 7, &[0; 16])
    }
    /// A reference to another object (class 13).
    pub(crate) fn reference(self, id: u16, n: u32) -> Self {
        self.item(id, 13, &n.to_le_bytes())
    }
    /// A reference to an object this object owns (class 14; spec 6.2 "refs").
    pub(crate) fn own_ref(self, id: u16, n: u32) -> Self {
        self.item(id, 14, &n.to_le_bytes())
    }
    /// A 2-byte value of class 2.
    pub(crate) fn c2(self, id: u16, v: u16) -> Self {
        self.item(id, 2, &v.to_le_bytes())
    }
    /// A record of class 19.
    pub(crate) fn rec19(self, id: u16, b: Blk) -> Self {
        let e = b.encode();
        self.item(id, 19, &e)
    }
    /// A list of class 20.
    pub(crate) fn list20(self, id: u16, elements: Vec<Vec<u8>>) -> Self {
        let mut body = Vec::new();
        for e in &elements {
            body.extend_from_slice(e);
        }
        let mut p = (body.len() as u32 + 4).to_le_bytes().to_vec();
        p.extend_from_slice(&body);
        self.item(id, 20, &p)
    }
    pub(crate) fn offset(self, id: u16, n: u32) -> Self {
        self.item(id, 23, &n.to_le_bytes())
    }
    pub(crate) fn string(self, id: u16, s: &str) -> Self {
        let body = utf16z(s);
        let mut p = (body.len() as u32 + 4).to_le_bytes().to_vec();
        p.extend_from_slice(&body);
        self.item(id, 24, &p)
    }
    /// Length-prefixed raw bytes (class 16).
    pub(crate) fn raw(self, id: u16, data: &[u8]) -> Self {
        let mut p = (data.len() as u32 + 4).to_le_bytes().to_vec();
        p.extend_from_slice(data);
        self.item(id, 16, &p)
    }
    pub(crate) fn rec(self, id: u16, b: Blk) -> Self {
        let e = b.encode();
        self.item(id, 17, &e)
    }
    /// A list: elements are already encoded (id 0) items.
    pub(crate) fn list(self, id: u16, elements: Vec<Vec<u8>>) -> Self {
        let mut body = Vec::new();
        for e in &elements {
            body.extend_from_slice(e);
        }
        let mut p = (body.len() as u32 + 4).to_le_bytes().to_vec();
        p.extend_from_slice(&body);
        self.item(id, 18, &p)
    }
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut items = self.items.clone();
        items.sort_by_key(|(id, _, _)| *id);
        let mut body = Vec::new();
        for (_, _, b) in &items {
            body.extend_from_slice(b);
        }
        let mut out = (body.len() as u32 + 4).to_le_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }
}

pub(crate) fn el_rec(b: Blk) -> Vec<u8> {
    let mut v = hdr(0, 17).to_vec();
    v.extend_from_slice(&b.encode());
    v
}
fn el_ref(n: u32) -> Vec<u8> {
    let mut v = hdr(0, 14).to_vec();
    v.extend_from_slice(&n.to_le_bytes());
    v
}
fn el_empty() -> Vec<u8> {
    hdr(0, 15).to_vec()
}

const EMPTY_BODY: [u8; 4] = [4, 0, 0, 0];

// ---------------------------------------------------------------------------------------
// Contents (spec 6.2)

/// One row of the kind table (spec 6.2.3): kinds `lo..=hi`.
struct KindRow {
    lo: u16,
    hi: u16,
    rev: u32,
    f6: bool,
    f7: bool,
    f8: bool,
    grp: u16,
    f10: bool,
    f11: bool,
    store: u32,
}

const fn kr(lo: u16, hi: u16, rev: u32, flags: &'static str, grp: u16, tail: u8, store: u32) -> KindRow {
    // `flags` holds the digits 6, 7, 8 that are present; `tail` is 0, 10 or 11.
    let b = flags.as_bytes();
    let mut f6 = false;
    let mut f7 = false;
    let mut f8 = false;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'6' => f6 = true,
            b'7' => f7 = true,
            b'8' => f8 = true,
            _ => {}
        }
        i += 1;
    }
    KindRow { lo, hi, rev, f6, f7, f8, grp, f10: tail == 10, f11: tail == 11, store }
}

const NONE: u32 = 0xFFFF_FFFF;

/// Rows of the kind table; kinds absent from the rows are the empty records `EMPTY_KINDS`.
const KIND_ROWS: &[KindRow] = &[
    kr(1, 15, 0, "68", 1, 0, 1),
    kr(16, 31, 0, "8", 1, 0, 2),
    kr(32, 32, 0, "8", 0, 0, 3),
    kr(33, 33, 0, "8", 4, 0, 3),
    kr(34, 47, 0, "8", 0, 0, 3),
    kr(48, 63, 0, "678", 1, 0, 4),
    kr(64, 64, 0, "8", 1, 11, 5),
    kr(67, 68, 0, "678", 1, 0, 5),
    kr(69, 69, 0, "8", 1, 0, 5),
    kr(70, 70, 0, "8", 3, 0, 5),
    kr(71, 71, 0, "", 0, 0, 5),
    kr(72, 72, 0, "", 0, 11, 5),
    kr(73, 73, 0, "8", 4, 0, 5),
    kr(74, 75, 0, "8", 3, 0, 5),
    kr(76, 76, 0, "8", 1, 0, 5),
    kr(77, 77, 0, "8", 4, 0, 5),
    kr(78, 78, 0, "8", 1, 0, 5),
    kr(79, 79, 0, "8", 2, 0, 5),
    kr(83, 83, 0, "68", 2, 10, 5),
    kr(84, 84, 0, "8", 0, 0, 5),
    kr(85, 85, 0, "", 4, 0, 5),
    kr(86, 86, 0, "8", 3, 0, 5),
    kr(87, 87, 0, "8", 4, 0, 5),
    kr(88, 88, 0, "8", 0, 0, 5),
    kr(89, 89, 0, "678", 1, 0, 5),
    kr(90, 90, 0, "68", 3, 11, NONE),
    kr(91, 91, 0, "8", 0, 0, 5),
    kr(92, 92, 0, "8", 1, 0, 5),
    kr(93, 94, 0, "8", 3, 0, 5),
    kr(95, 96, 0, "8", 4, 0, 5),
    kr(97, 97, 0, "68", 2, 0, 5),
    kr(99, 99, 0, "8", 1, 0, 5),
    kr(101, 101, 0, "8", 2, 0, 5),
    kr(102, 104, 0, "8", 4, 0, 5),
    kr(105, 105, 0, "8", 0, 0, 5),
    kr(106, 107, 0, "8", 4, 0, 5),
    kr(108, 109, 0, "8", 3, 0, 5),
    kr(112, 112, 0, "68", 2, 0, 5),
    kr(113, 115, 0, "8", 2, 10, 5),
    kr(116, 116, 0, "8", 4, 0, 5),
    kr(117, 117, 0, "68", 2, 0, 5),
    kr(118, 118, 0, "8", 0, 0, 5),
    kr(119, 125, 0, "8", 4, 0, 5),
    kr(126, 126, 0, "68", 1, 0, 5),
    kr(127, 127, 0, "678", 1, 0, 5),
    kr(128, 128, 0, "8", 2, 0, 5),
    kr(129, 129, 0, "678", 1, 0, 5),
    kr(130, 130, 0, "8", 4, 0, 5),
    kr(131, 131, 15, "8", 4, 0, 5),
    kr(132, 132, 17, "8", 1, 0, 5),
    kr(133, 134, 17, "68", 1, 0, NONE),
    kr(135, 136, 17, "8", 1, 0, NONE),
    kr(137, 137, 17, "68", 1, 0, NONE),
    kr(138, 138, 20, "8", 4, 0, 5),
    kr(139, 139, 20, "678", 1, 0, 5),
    kr(140, 140, 20, "8", 1, 0, 5),
    kr(141, 141, 1, "68", 1, 0, 5),
    kr(142, 142, 1, "8", 1, 0, 5),
    kr(143, 143, 20, "8", 4, 0, 5),
];

fn kind_table() -> Vec<u8> {
    const WRITER_MINOR: u32 = 0x1A;
    let mut elements = Vec::new();
    for k in 1u16..=143 {
        let mut b = Blk::new().u16(1, k);
        if let Some(r) = KIND_ROWS.iter().find(|r| r.lo <= k && k <= r.hi) {
            b = b.u32(3, (1000 << 16) | r.rev);
            if r.f6 {
                b = b.flag(6, true);
            }
            if r.f7 {
                b = b.flag(7, true);
            }
            if r.f8 {
                b = b.flag(8, true);
            }
            if r.grp != 0 {
                b = b.c2(9, r.grp);
            }
            if r.f10 {
                b = b.flag(10, true);
            }
            if r.f11 {
                b = b.flag(11, true);
            }
            b = b.u32(12, r.store);
        } // else: an empty record (65, 66, 80-82, 98, 100, 110, 111)
        elements.push(el_rec(b));
    }
    Blk::new()
        .u32(1, 143)
        .u32(2, (1000 << 16) | WRITER_MINOR)
        .list(3, elements)
        .encode()
}

struct Obj {
    num: u32,
    kind: u16,
    parent: Option<u32>,
    body: Vec<u8>,
    idx8: bool,
    idx11: Option<u16>,
}

fn size_rec(w: u32, h: u32) -> Blk {
    Blk::new().u32(1, w).u32(2, h)
}

const WEB_FORM_FIELDS: [(u16, &str); 5] = [
    (14, "someone@example.com"),
    (15, "Web Site Form Response"),
    (16, "Your information was received"),
    (19, "FORMDATA.HTM"),
    (24, "http://example.com/~user/ispscript.cgi"),
];

/// The web-form defaults record every page carries (kind 119, spec 6.2.4).
fn web_form() -> Blk {
    let mut b = Blk::new().u32(12, 2).u32(23, 1);
    for (id, s) in WEB_FORM_FIELDS {
        b = b.string(id, s);
    }
    b
}

fn page_setup_record(w: u32, h: u32, preset_letter: bool) -> Blk {
    let margins = |b: Blk| b.u32(6, 457_200).u32(7, 457_200).u32(8, 457_200).u32(9, 457_200);
    if preset_letter {
        Blk::new()
            .u32(2, 104)
            .u32(3, 109)
            .u32(4, 0x8001_0000)
            .string(6, "Letter")
            .u32(7, 0x0102_7100)
            .u32(10, 0)
            .u32(17, 1)
            .list(18, vec![el_rec(margins(Blk::new().u32(3, w).u32(4, h)))])
            .u32(19, 1)
            .list(20, vec![el_rec(size_rec(w, h).rec(8, Blk::new().u32(2, 0).u32(3, 0)))])
    } else {
        // The same paper in portrait order (landscape pages swap).
        let (pw, ph) = if w > h { (h, w) } else { (w, h) };
        Blk::new()
            .u32(2, 161)
            .string(5, "Current")
            .u32(8, 2)
            .u32(10, 0)
            .u32(17, 1)
            .list(18, vec![el_rec(margins(Blk::new().u32(3, w).u32(4, h)))])
            .u32(19, 1)
            .list(
                20,
                vec![el_rec(size_rec(pw, ph).flag(5, true).rec(8, Blk::new().u32(2, 0).u32(3, 0)))],
            )
    }
}

fn contents_stream(spec: &NewPublication, opt: &Options) -> Vec<u8> {
    let (w, h) = (spec.width_emu, spec.height_emu);
    // Big placeholder page extent of the master and the helper pages, in EMU (1800 pt and
    // 8676 pt), and the half-page formula of object 269.
    const BIG_A: u32 = 22_860_000;
    const BIG_B: u32 = 110_185_200;
    let half = |v: u32, centre: u32| centre - v / 2;
    let helper_a_w = half(w, BIG_A);
    let helper_a_h = half(h, BIG_A);
    let helper_b_w = half(w, BIG_B);
    let helper_b_h = half(h, BIG_B);

    let mut objs: Vec<Obj> = Vec::new();
    let omitted = |n: u32| opt.omit.contains(&n) || (opt.omit_misc_objects && [288, 293, 294, 295].contains(&n));
    let mut add = |num: u32, kind: u16, parent: Option<u32>, body: Blk, idx8: bool, idx11: Option<u16>| {
        if omitted(num) {
            return;
        }
        let body = body.without_refs(&omitted).encode();
        objs.push(Obj { num, kind, parent, body, idx8, idx11 });
    };

    let helpers: &[(u32, u32, u32, bool)] = &[
        // page object, kind-96 child, kind-119 child, is object 269 (own sizes)
        (269, 270, 271, true),
        (272, 273, 274, false),
        (275, 276, 277, false),
        (279, 280, 281, false),
    ];
    let helper_list: Vec<u32> = if opt.omit_helper_pages { vec![] } else { helpers.iter().map(|x| x.0).collect() };
    // Pages after the first take object numbers 296, 297, 298 (page, kind 96, kind 119),
    // 299.. and so on (spec 4P.2); they sit in the list after page 1 and before the helpers.
    let extra_pages: Vec<u32> = (1..spec.page_count).map(|k| 296 + 3 * (k - 1)).collect();
    let mut page_refs = vec![263u32, 266];
    page_refs.extend(extra_pages.iter().copied());
    page_refs.extend(helper_list.iter().copied());

    // 256: the document object.
    let mut doc = Blk::new()
        .u32(1, page_refs.len() as u32)
        .list20(2, page_refs.iter().map(|n| el_ref(*n)).collect())
        .own_ref(3, 287)
        .own_ref(4, 291)
        .flag(8, true)
        .flag(14, true)
        .rec(18, size_rec(w, h))
        .own_ref(24, 259)
        .own_ref(25, 261)
        .own_ref(26, 257)
        .own_ref(32, 282)
        .own_ref(33, 262)
        .own_ref(34, 285)
        .u32(35, spec.page_count)
        .zero16(42)
        .u16(44, 5)
        .u16(45, 1)
        .reference(49, 278)
        .flag(57, false)
        .u32(60, 1)
        .u32(65, 0)
        .own_ref(68, 292)
        .flag(77, true);
    if !omitted(293) {
        doc = doc.own_ref(20, 293);
    }
    add(256, 68, None, doc, true, Some(23));
    add(257, 114, Some(256), Blk::new(), true, None);
    add(259, 115, Some(256), Blk::new(), true, None);
    add(261, 70, Some(256), Blk::new(), true, None);
    add(262, 84, Some(256), Blk::new(), true, None);

    // 263: master page "A"; 264, 265 its children; 289 the layout guides.
    let master = Blk::new()
        .own_ref(3, 289)
        .rec(5, size_rec(BIG_A, BIG_A))
        .u64(6, 0)
        .own_ref(9, 264)
        .own_ref(11, 265)
        .string(14, "A")
        .string(15, "Master Page A")
        .u32(16, 3)
        .rec(17, size_rec(BIG_B, BIG_B));
    add(263, 67, Some(256), master, true, Some(1));
    add(264, 96, Some(263), Blk::new().u32(5, 1), false, Some(1));
    add(265, 119, Some(263), web_form(), false, Some(1));

    // 266: page 1; 267, 268 its children.
    let mut page1 = Blk::new()
        .rec(5, size_rec(BIG_A, BIG_A))
        .u64(6, 1)
        .own_ref(9, 267)
        .own_ref(11, 268)
        .reference(13, 263)
        .string(14, "")
        .string(15, "")
        .u32(16, 5)
        .rec(17, size_rec(BIG_B, BIG_B));
    if !omitted(295) {
        page1 = page1.own_ref(8, 295);
    }
    add(266, 67, Some(256), page1, true, Some(1));
    add(267, 96, Some(266), Blk::new(), false, None);
    add(268, 119, Some(266), web_form(), false, Some(1));

    // The four helper pages.
    if !opt.omit_helper_pages {
        for (p, c96, c119, own) in helpers.iter().copied() {
            let (a, b) = if own {
                ((helper_a_w, helper_a_h), (helper_b_w, helper_b_h))
            } else {
                ((BIG_A, BIG_A), (BIG_B, BIG_B))
            };
            let body = Blk::new()
                .rec(5, size_rec(a.0, a.1))
                .u64(6, 0)
                .own_ref(9, c96)
                .own_ref(11, c119)
                .reference(13, 263)
                .string(14, "")
                .string(15, "")
                .u32(16, 5)
                .rec(17, size_rec(b.0, b.1));
            add(p, 67, Some(256), body, true, Some(1));
            add(c96, 96, Some(p), Blk::new(), false, Some(1));
            add(c119, 119, Some(p), web_form(), false, Some(1));
        }
    }
    for (k, p) in extra_pages.iter().copied().enumerate() {
        // The serial of the page is the value of object 256 item 35 before it was added.
        let serial = (k as u64 + 1) << 32 | 2;
        let body = Blk::new()
            .rec(5, size_rec(BIG_A, BIG_A))
            .u64(6, serial)
            .own_ref(9, p + 1)
            .own_ref(11, p + 2)
            .reference(13, 263)
            .string(14, "")
            .string(15, "")
            .u32(16, 5)
            .rec(17, size_rec(BIG_B, BIG_B));
        add(p, 67, Some(256), body, true, Some(1));
        add(p + 1, 96, Some(p), Blk::new().u32(5, 1), false, Some(1));
        add(p + 2, 119, Some(p), web_form(), false, Some(1));
    }
    add(278, 127, Some(256), Blk::new(), true, None);
    add(
        282,
        91,
        Some(256),
        Blk::new().own_ref(1, 283).own_ref(2, 284).rec19(10, Blk::new().u32(1, 0x0800_0000)),
        true,
        Some(1),
    );
    add(283, 97, Some(282), Blk::new(), true, None);
    add(284, 101, Some(282), Blk::new(), true, None);

    // 285: colour scheme "Office" (spec 4F).
    let rgb = |r: u32, g: u32, b: u32| r | (g << 8) | (b << 16);
    let mut colours = vec![el_empty()];
    for c in [
        rgb(0x1F, 0x49, 0x7D),
        rgb(0xC0, 0x50, 0x4D),
        rgb(0x4F, 0x81, 0xBD),
        rgb(0xEE, 0xEC, 0xE1),
        rgb(0x00, 0x00, 0xFF),
        rgb(0x80, 0x00, 0x80),
        rgb(0xFF, 0xFF, 0xFF),
    ] {
        colours.push(el_rec(Blk::new().u32(1, c)));
    }
    add(
        285,
        92,
        Some(256),
        Blk::new().u32(1, 8).list20(2, colours).u32(3, 75).string(6, "Office"),
        true,
        Some(1),
    );

    // 287: printer and paper list (no printer names); 288 its child.
    let paper = || {
        el_rec(
            Blk::new()
                .u16(1, 1)
                .rec19(2, size_rec(7_772_400, 10_058_400))
                .u16(3, 7)
                .flag(6, true)
                .string(8, "")
                .string(9, "")
                .string(10, "")
                .raw(14, &[0, 0]),
        )
    };
    let mut printers = Blk::new()
        .u32(1, 2)
        .list(2, vec![paper(), paper()])
        .flag(10, true)
        .u32(11, 1)
        .u32(16, 228_600)
        .flag(23, true)
        .flag(24, true)
        .flag(26, true)
        .flag(28, true)
        .flag(29, true)
        .u16(34, 0)
        .u16(35, 0);
    if !omitted(288) {
        printers = printers.own_ref(3, 288);
    }
    add(287, 75, Some(256), printers, true, Some(20));
    if !omitted(288) {
        let b288 = Blk::new()
            .list20(2, vec![el_rec(Blk::new().rec19(2, Blk::new().u32(1, 0xFFFF_FFFF)))])
            .u32(3, 304_800)
            .u32(4, 243)
            .u32(12, 3_175)
            .u32(13, 3_175)
            .u32(14, 70)
            .flag(15, true)
            .flag(16, true)
            .flag(17, true);
        add(288, 79, Some(287), b288, true, Some(1));
    }

    // 289: layout guides of the master page (0.5 in margins).
    let guide = |v: u32, flags: [u16; 2]| el_rec(Blk::new().u32(1, v).flag(flags[0], true).flag(flags[1], true));
    let b289 = Blk::new()
        .u32(1, 4)
        .list20(
            2,
            vec![
                guide(457_200, [3, 4]),
                guide(w - 457_200, [2, 4]),
                guide(457_200, [3, 4]),
                guide(h - 457_200, [2, 4]),
            ],
        )
        .rec(3, size_rec(w, h))
        .u32(4, 45_720)
        .u32(5, 45_720)
        .u16(6, 2)
        .u16(7, 2)
        .u32(8, 165_100)
        .u32(9, 165_100)
        .u32(10, 165_100)
        .u32(11, 165_100);
    add(289, 76, Some(263), b289, true, Some(15));

    // 291: the six default bullets.
    let bullet = |c: u16, size: u32| el_rec(Blk::new().u16(1, c).u32(2, size).string(3, "Symbol"));
    let b291 = Blk::new().u16(1, 6).list20(
        2,
        vec![
            bullet(183, 127_000),
            bullet(183, 152_400),
            bullet(168, 127_000),
            bullet(222, 127_000),
            bullet(224, 127_000),
            bullet(42, 127_000),
        ],
    );
    add(291, 74, Some(256), b291, true, Some(1));

    // 292: page setup.
    let preset = opt.letter_preset && w == 7_772_400 && h == 10_058_400;
    let b292 = if opt.empty_page_setup {
        Blk::new()
    } else {
        Blk::new().rec(4, page_setup_record(w, h, preset))
    };
    add(292, 138, Some(256), b292, true, Some(20));
    let b77 = Blk::new()
        .u32(0, 1)
        .rec(1, Blk::new().u32(0, 1).u32(20, 32).u32(22, 1).u32(38, 1).u32(58, 15))
        .u32(9, 1);
    if !omitted(293) {
        let mut b293 = Blk::new().u32(1, 178).u32(11, 179);
        if !omitted(294) {
            b293 = b293.own_ref(4, 294);
        }
        add(293, 88, Some(256), b293, true, Some(20));
        if !omitted(294) {
            add(294, 77, Some(293), b77.clone(), false, Some(26));
        }
    }
    if !omitted(295) {
        add(295, 77, Some(266), b77, false, Some(26));
    }
    objs.sort_by_key(|o| o.num);

    // Block 0 (stream identity); built twice because it contains offsets into the stream.
    let block0 = |total: u32, index_off: u32, table_off: u32| {
        Blk::new()
            .u16(1, 0xACE8)
            .reference(2, 256)
            .u32(3, 0x03E8_001A)
            .u32(4, 0x03E8_001A)
            .u32(5, 0x03E8_0018)
            .u32(7, 0x03E8_0018)
            .u32(8, total)
            .string(9, "")
            .offset(10, index_off)
            .offset(11, table_off)
            .u16(12, 1)
            .u32(13, 14_334)
            .u32(14, 14_334)
            .u32(15, 4_750)
            .u32(16, 4_750)
            .encode()
    };
    let b0_len = block0(0, 0, 0).len();
    let table = kind_table();
    let table_off = 0x2C + b0_len;
    let mut offsets = Vec::new();
    let mut pos = table_off + table.len();
    for o in &objs {
        offsets.push(pos);
        pos += o.body.len();
    }
    let index_off = pos;

    let last = objs.last().expect("objects").num;
    let mut slots: Vec<Vec<u8>> = vec![el_empty(); (last + 1) as usize];
    for (o, off) in objs.iter().zip(&offsets) {
        let mut r = Blk::new().u16(2, o.kind).offset(4, *off as u32);
        if let Some(p) = o.parent {
            r = r.reference(5, p);
        }
        r = r.c2(6, if o.num == 256 { 0x1002 } else { 0x0102 });
        if o.idx8 {
            r = r.flag(8, true);
        }
        if let Some(v) = o.idx11 {
            r = r.u16(11, v);
        }
        slots[o.num as usize] = el_rec(r);
    }
    let index = Blk::new().u32(1, last + 1).u32(2, last).list(3, slots).encode();
    let total = index_off + index.len();

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0xACE8u16.to_le_bytes());
    out.extend_from_slice(&0x2Cu16.to_le_bytes());
    out.extend_from_slice(&1000u16.to_le_bytes());
    out.extend_from_slice(&0x000Au16.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&0x1Au16.to_le_bytes());
    out.extend_from_slice(&0x0100u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 10]);
    out.extend_from_slice(&(index_off as u32).to_le_bytes());
    out.extend_from_slice(&(table_off as u32).to_le_bytes());
    out.extend_from_slice(&14u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 6]);
    debug_assert_eq!(out.len(), 0x2C);
    out.extend_from_slice(&block0(total as u32, index_off as u32, table_off as u32));
    out.extend_from_slice(&table);
    for o in &objs {
        out.extend_from_slice(&o.body);
    }
    out.extend_from_slice(&index);
    debug_assert_eq!(out.len(), total);
    out
}

// ---------------------------------------------------------------------------------------
// Quill text engine stream (spec 6.3)

/// One built-in style of the default style sheet (spec 6.3.2 data table).
struct Style {
    id: i32,
    size_pt: Option<f64>,
    heading_font: bool,
    space_after_pt: Option<f64>,
    /// Line spacing in packed form (spec 3.6: `(amount << 3) | rule 2`).
    lines: u32,
    flags230: u16,
    extras: Extras,
}

#[derive(Clone, Copy, PartialEq)]
enum Extras {
    None,
    /// Character property 0x21F = 1120.
    C1120,
    /// Italic-like character flags 0x203 and 0x238.
    Italic,
    /// List style: bullet character and bullet size.
    Bullet(u32, u32),
}

const L119: u32 = 1_450_850;
const L110: u32 = 1_341_122;
const L175: u32 = 2_133_602;

const fn st(id: i32, size: Option<f64>, heading: bool, after: Option<f64>, lines: u32, f230: u16, extras: Extras) -> Style {
    Style { id, size_pt: size, heading_font: heading, space_after_pt: after, lines, flags230: f230, extras }
}

const fn s(v: f64) -> Option<f64> {
    Some(v)
}

/// The 36 built-in styles in `STSH` 0 order.
const STYLES: [Style; 36] = [
    st(-1, s(10.0), false, s(6.0), L119, 10501, Extras::None),
    st(-3, s(16.0), true, s(6.0), L119, 10501, Extras::None),
    st(-6, s(14.0), true, s(6.0), L119, 10501, Extras::None),
    st(-55, s(9.0), true, s(12.0), L119, 21253, Extras::Bullet(183, 127_000)),
    st(-5, None, true, s(16.0), L119, 6405, Extras::None),
    st(-82, s(9.5), false, s(6.0), L119, 10501, Extras::None),
    st(-7, s(13.0), true, s(12.0), L119, 21253, Extras::None),
    st(-67, s(10.0), false, s(6.0), L110, 10501, Extras::None),
    st(-49, s(10.0), true, s(12.0), L119, 21253, Extras::Bullet(168, 114_300)),
    st(-102, s(36.0), true, None, L119, 5, Extras::None),
    st(-107, s(11.0), false, None, L175, 5, Extras::None),
    st(-108, s(8.0), false, None, L119, 5, Extras::C1120),
    st(-109, s(8.0), false, s(4.0), L119, 50693, Extras::C1120),
    st(-111, s(8.0), false, None, L119, 5, Extras::None),
    st(-112, s(9.0), false, None, L119, 5, Extras::None),
    st(-114, s(10.0), false, None, L119, 5, Extras::None),
    st(-115, s(10.0), false, None, L119, 5, Extras::None),
    st(-116, s(8.0), false, None, L119, 5, Extras::None),
    st(-117, None, false, None, L119, 5, Extras::None),
    st(-118, s(11.0), true, None, L119, 5, Extras::None),
    st(-119, s(11.0), true, None, L119, 5, Extras::None),
    st(-122, s(9.0), false, None, L119, 5, Extras::None),
    st(-123, None, false, None, L119, 5, Extras::None),
    st(-103, s(28.0), true, None, L119, 5, Extras::None),
    st(-104, s(18.0), true, None, L119, 5, Extras::None),
    st(-105, s(16.0), true, None, L119, 5, Extras::None),
    st(-4, s(14.0), true, None, L119, 5, Extras::None),
    st(-8, s(14.0), true, s(6.0), L119, 10501, Extras::None),
    st(-81, None, false, s(6.0), L110, 10501, Extras::None),
    st(-106, s(9.5), false, s(6.0), L119, 10501, Extras::Italic),
    st(-110, s(10.0), false, None, L119, 5, Extras::None),
    st(-113, None, false, None, L119, 5, Extras::None),
    st(-120, s(10.0), false, None, L119, 5, Extras::None),
    st(-121, s(9.0), false, None, L119, 5, Extras::None),
    st(-63, s(48.0), true, None, L119, 5, Extras::None),
    st(-2, s(18.0), true, None, L119, 5, Extras::None),
];

/// Font-map pattern A: (writing-system id, `FONT` index); sub-record 0 is absent.
const FONT_MAP_A: [(u16, u16); 33] = [
    (1, 0), (2, 0), (3, 0), (4, 0), (5, 1), (6, 2), (7, 2), (8, 3), (9, 4), (10, 5), (11, 6),
    (12, 7), (13, 8), (14, 9), (15, 10), (16, 11), (17, 12), (18, 13), (19, 14), (20, 1),
    (21, 15), (22, 16), (23, 17), (24, 18), (25, 16), (30, 19), (31, 20), (33, 21), (34, 22),
    (35, 23), (36, 24), (39, 25), (40, 26),
];

fn font_map(heading: bool) -> Blk {
    let mut entries: Vec<(u16, u16)> = FONT_MAP_A.to_vec();
    entries.push((42, 0));
    entries.push((44, 0));
    if heading {
        entries.push((0, 27));
        for e in entries.iter_mut() {
            match e.0 {
                1..=4 | 42 | 44 => e.1 = 27,
                6 | 7 => e.1 = 28,
                17 => e.1 = 29,
                21 => e.1 = 30,
                _ => {}
            }
        }
    }
    let mut b = Blk::new();
    for (ws, font) in entries {
        b = b.rec(ws, Blk::new().u16(0, font));
    }
    b
}

fn style_char_props(i: usize, opt: &Options) -> Blk {
    let sty = &STYLES[i];
    let colour = if i == 0 { 0 } else { 1 };
    let mut b = Blk::new().rec(0x224, font_map(sty.heading_font)).rec(0x244, Blk::new().u32(0x200, colour)).rec(0x248, Blk::new().u32(0x200, 0));
    if let Some(pt) = sty.size_pt {
        let emu = (pt * 12_700.0).round() as u32;
        b = b.u32(0x20C, emu).u32(0x239, emu);
    }
    match sty.extras {
        Extras::C1120 => b = b.u32(0x21F, 1120),
        Extras::Italic => b = b.flag(0x203, true).flag(0x238, true),
        _ => {}
    }
    if opt.style_0x258 {
        b = b.rec(0x258, Blk::new().u16(0x200, 1).u32(0x201, colour).u32(0x202, 100_000));
    }
    b
}

fn style_para_props(i: usize) -> Blk {
    let sty = &STYLES[i];
    let mut b = Blk::new().u16(0x230, sty.flags230).u32(0x234, sty.lines);
    if let Some(pt) = sty.space_after_pt {
        b = b.u32(0x213, (pt * 12_700.0).round() as u32);
    }
    if let Extras::Bullet(ch, size) = sty.extras {
        b = b
            .u32(0x202, size)
            .u32(0x20C, (-137_160i32) as u32)
            .rec(0x257, Blk::new().u32(0x200, 23).u32(0x201, ch).u32(0x202, 0xFFFF));
    }
    b
}

/// A style-sheet chunk: 20-byte header, offset table, entries.
fn chunk_with_table(count_x_y: (u32, u32), entries: &[Vec<u8>]) -> Vec<u8> {
    let table_len = entries.len() * 4;
    let mut offs = Vec::new();
    let mut pos = table_len as u32;
    for e in entries {
        offs.push(pos);
        pos += e.len() as u32;
    }
    let payload_len = pos;
    let mut out = Vec::new();
    out.extend_from_slice(&payload_len.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    out.extend_from_slice(&count_x_y.0.to_le_bytes());
    out.extend_from_slice(&count_x_y.1.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // stamp (volatile; zero)
    for o in offs {
        out.extend_from_slice(&o.to_le_bytes());
    }
    for e in entries {
        out.extend_from_slice(e);
    }
    out
}

const FONTS: [(&str, u32); 32] = [
    ("Calibri", 3),
    ("Sylfaen", 4),
    ("Tahoma", 0x362),
    ("Mangal", 9),
    ("Vrinda", 10),
    ("Raavi", 20),
    ("Shruti", 21),
    ("Kalinga", 22),
    ("Latha", 23),
    ("Gautami", 24),
    ("Tunga", 25),
    ("Kartika", 11),
    ("BrowalliaUPC", 0x0202_016B),
    ("DokChampa", 14),
    ("Microsoft Himalaya", 13),
    ("Malgun Gothic", 6),
    ("MS Mincho", 0x0103_0005),
    ("PMingLiU", 7),
    ("SimSun", 8),
    ("Estrangelo Edessa", 27),
    ("MV Boli", 28),
    ("Iskoola Pota", 15),
    ("Nyala", 18),
    ("Plantagenet Cherokee", 16),
    ("Euphemia Regular CAS", 17),
    ("DaunPenh", 29),
    ("Mongolian Baiti", 12),
    ("Cambria", 0x0201_0041),
    ("Times New Roman", 0),
    ("EucrosiaUPC", 0x0201_0187),
    ("Batang", 0x0201_0163),
    ("Symbol", 0x0201_0097),
];

fn quill_stream(opt: &Options) -> Vec<u8> {
    // STSH 0: style identifiers.
    let stsh0: Vec<Vec<u8>> = STYLES
        .iter()
        .map(|s| {
            let mut e = 0u16.to_le_bytes().to_vec();
            e.extend_from_slice(&s.id.to_le_bytes());
            e
        })
        .collect();
    let stsh0 = chunk_with_table((0, 4), &stsh0);

    // STSH 1: character and paragraph properties of each style.
    let entry = |b: Blk| {
        let block = b.encode();
        let words = (block.len() / 2 + 1) as u16;
        let mut e = words.to_le_bytes().to_vec();
        e.extend_from_slice(&block);
        e.extend_from_slice(&[0, 0]); // two trailing bytes (values unknown; zero)
        e
    };
    let mut stsh1 = Vec::new();
    for i in 0..STYLES.len() {
        stsh1.push(entry(style_char_props(i, opt)));
        stsh1.push(entry(style_para_props(i)));
    }
    let stsh1 = chunk_with_table((0, 0), &stsh1);

    // STSH 2: two fixed entries per style.
    let mut stsh2 = Vec::new();
    for k in 0..STYLES.len() as u32 {
        for _ in 0..2 {
            let mut e = 7u16.to_le_bytes().to_vec();
            e.extend_from_slice(&(-1i32).to_le_bytes());
            e.extend_from_slice(&k.to_le_bytes());
            e.extend_from_slice(&4u16.to_le_bytes());
            e.extend_from_slice(&0u32.to_le_bytes());
            stsh2.push(e);
        }
    }
    let stsh2 = chunk_with_table((0, 0), &stsh2);

    let fonts: Vec<Vec<u8>> = FONTS
        .iter()
        .map(|(name, tag)| {
            let mut e = (name.encode_utf16().count() as u16).to_le_bytes().to_vec();
            e.extend(name.encode_utf16().flat_map(u16::to_le_bytes));
            e.extend_from_slice(&tag.to_le_bytes());
            e
        })
        .collect();
    let font = chunk_with_table((0xB811_9816, 4), &fonts);

    // PL: the colour list.
    let mut pl = Vec::new();
    pl.extend_from_slice(&2u32.to_le_bytes());
    pl.extend_from_slice(&0x28u32.to_le_bytes());
    pl.extend_from_slice(&0u32.to_le_bytes());
    pl.extend_from_slice(
        &Blk::new()
            .u32(0x201, 0x0800_0000)
            .u32(0x202, 0xFFFF_FFFF)
            .u32(0x203, 0x2000_0000)
            .u32(0x204, 0xFFFF_FFFF)
            .u32(0x205, 0xFFFF_FFFF)
            .raw(0x206, &[0, 0])
            .encode(),
    );
    pl.extend_from_slice(&Blk::new().u32(0x201, 0).raw(0x206, &[0, 0]).encode());

    let mut syid = 1u32.to_le_bytes().to_vec();
    syid.extend_from_slice(&0u32.to_le_bytes());
    let sgp = EMPTY_BODY.to_vec();
    let ink = vec![0x74, 0, 0x74, 0];

    let chunks: [(&[u8; 4], u16, &Vec<u8>); 8] = [
        (b"STSH", 0, &stsh0),
        (b"STSH", 1, &stsh1),
        (b"STSH", 2, &stsh2),
        (b"SYID", 0, &syid),
        (b"SGP ", 0, &sgp),
        (b"INK ", 0, &ink),
        (b"FONT", 0, &font),
        (b"PL  ", 0, &pl),
    ];
    let mut body = Vec::new();
    let mut dir = Vec::new();
    for (tag, inst, data) in chunks {
        let off = 0x200 + body.len();
        dir.extend_from_slice(&0x18u16.to_le_bytes());
        dir.extend_from_slice(tag);
        dir.extend_from_slice(&inst.to_le_bytes());
        dir.extend_from_slice(&1u32.to_le_bytes());
        dir.extend_from_slice(tag);
        dir.extend_from_slice(&(off as u32).to_le_bytes());
        dir.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
    }
    let total = (0x200 + body.len()).div_ceil(512) * 512;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"CHNKINK ");
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&7u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&0x0300u16.to_le_bytes());
    out.extend_from_slice(&0x200u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&504u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    out.extend_from_slice(&dir);
    out.resize(0x200, 0);
    out.extend_from_slice(&body);
    out.resize(total, 0);
    out
}

// ---------------------------------------------------------------------------------------
// Escher drawing stream (spec 6.4; public [MS-ODRAW] record format)

fn rec(ver: u16, inst: u16, ty: u16, data: &[u8]) -> Vec<u8> {
    let mut v = (ver | (inst << 4)).to_le_bytes().to_vec();
    v.extend_from_slice(&ty.to_le_bytes());
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(data);
    v
}

fn props(entries: &[(u16, u32)]) -> Vec<u8> {
    let mut v = Vec::new();
    for (id, val) in entries {
        v.extend_from_slice(&id.to_le_bytes());
        v.extend_from_slice(&val.to_le_bytes());
    }
    v
}

fn prop_rec(ty: u16, entries: &[(u16, u32)]) -> Vec<u8> {
    rec(3, entries.len() as u16, ty, &props(entries))
}

fn container(ty: u16, children: &[Vec<u8>]) -> Vec<u8> {
    rec(15, 0, ty, &children.concat())
}

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

const INSET: u32 = 36_576;
/// Pairs of colour-scheme references in the tertiary tables.
fn scheme_pairs() -> Vec<(u16, u32)> {
    let mut v = Vec::new();
    for base in [0x540u16, 0x580, 0x5C0, 0x600, 0x640] {
        v.push((base, 0x0800_0000));
        v.push((base + 2, 0x0800_0007));
    }
    v
}

fn escher_stream(opt: &Options) -> Vec<u8> {
    // Document default shape properties.
    let mut defaults: Vec<(u16, u32)> = vec![(0x081, INSET), (0x082, INSET), (0x083, INSET), (0x084, INSET)];
    if opt.escher_2021 {
        defaults.push((0x180, 0));
    }
    defaults.extend([(0x181, 0x0800_0007), (0x183, 0x0800_0007), (0x1BF, 0x0010_0000), (0x1C0, 0x0800_0000), (0x1C2, 0x0800_0007)]);
    if opt.escher_2021 {
        defaults.push((0x1CB, 25_400));
    }
    defaults.extend([(0x1D7, 2), (0x1FF, 0x0008_0008), (0x201, 0x0800_0004)]);
    if opt.escher_2021 {
        defaults.extend([(0x285, 0), (0x2CB, 0), (0x2CC, 0), (0x2CE, 0), (0x2CF, 0)]);
    }
    defaults.extend([(0x384, INSET), (0x385, INSET), (0x386, INSET), (0x387, INSET)]);

    let mut tertiary: Vec<(u16, u32)> = Vec::new();
    if opt.escher_2021 {
        tertiary.push((0x1FF, 0x0040_0000));
        for id in 0x295..=0x29A {
            tertiary.push((id, 0));
        }
    }
    tertiary.extend(scheme_pairs());

    let dgg_header = u32s(&[0x0C03, 4, 5, 3, 1, 1, 2, 2, 3, 3]);
    let dgg = container(
        0xF000,
        &[
            rec(0, 0, 0xF006, &dgg_header),
            prop_rec(0xF00B, &defaults),
            prop_rec(0xF122, &tertiary),
            rec(0, 4, 0xF11E, &u32s(&[0x0800_0001, 0x0800_0000, 0x0800_0004, 0x1000_00F7])),
        ],
    );

    let fsp = |inst: u16, spid: u32, flags: u32| rec(2, inst, 0xF00A, &u32s(&[spid, flags]));
    let patriarch = |spid: u32| container(0xF004, &[rec(1, 0, 0xF009, &[0u8; 16]), fsp(0, spid, 5)]);
    let anchor_inst = if opt.escher_2021 { 0x1A } else { 0x18 };
    let client_anchor = |data: &[u8]| rec(10, anchor_inst, 0xF010, data);

    let mut pairs3: Vec<(u16, u32)> = Vec::new();
    pairs3.extend(scheme_pairs());
    let mut rect_t = vec![(0x1BF, 0x0060_0000u32), (0x1FF, 0x0040_0040)];
    rect_t.extend(pairs3.iter().copied());
    let rect = container(
        0xF004,
        &[
            fsp(1, 0x0C01, 0x0A00),
            prop_rec(
                0xF00B,
                &[
                    (0x081, INSET), (0x082, INSET), (0x083, INSET), (0x084, INSET), (0x181, 0xFFFF_FFFF),
                    (0x1BF, 0x001F_000C), (0x1C0, 0x0800_0000), (0x1C2, 0x0800_0007), (0x1FF, 0x0008_0008),
                    (0x201, 0x0800_0004), (0x384, INSET), (0x385, INSET), (0x386, INSET), (0x387, INSET),
                ],
            ),
            prop_rec(0xF122, &rect_t),
            client_anchor(&EMPTY_BODY),
        ],
    );
    let line = container(
        0xF004,
        &[
            fsp(20, 0x0C02, 0x0A00),
            prop_rec(
                0xF00B,
                &[
                    (0x081, INSET), (0x082, INSET), (0x083, INSET), (0x084, INSET), (0x1BF, 0x0010_0000),
                    (0x1C0, 0xFFFF_FFFF), (0x1C2, 0x0800_0007), (0x1CB, 0), (0x1FF, 0x0008_0000),
                    (0x201, 0x0800_0004), (0x384, INSET), (0x385, INSET), (0x386, INSET), (0x387, INSET),
                ],
            ),
            prop_rec(0xF122, &scheme_pairs()),
            client_anchor(&Blk::new().u32(1, 1).encode()),
        ],
    );
    let drawing3 = container(
        0xF002,
        &[rec(0, 3, 0xF008, &u32s(&[3, 0x0C02])), container(0xF003, &[patriarch(0x0C00), rect, line])],
    );
    let page_bg = container(
        0xF004,
        &[
            fsp(1, 0x0801, 0x0C00),
            prop_rec(0xF00B, &[(0x304, 9), (0x33F, 0x0001_0001)]),
            prop_rec(0xF122, &[(0x1FF, 0x0040_0040)]),
        ],
    );
    let drawing2 = container(
        0xF002,
        &[rec(0, 2, 0xF008, &u32s(&[1, 0x0801])), container(0xF003, &[patriarch(0x0800)]), page_bg],
    );
    let drawing1 = container(
        0xF002,
        &[rec(0, 1, 0xF008, &u32s(&[1, 0x0400])), container(0xF003, &[patriarch(0x0400)])],
    );

    let mut out = dgg;
    for (prefix, d) in [(2u32, drawing3), (1, drawing2), (0, drawing1)] {
        out.extend_from_slice(&prefix.to_le_bytes());
        out.extend_from_slice(&d);
    }
    out
}

// ---------------------------------------------------------------------------------------
// Small streams and property sets (spec 6.5)

fn compobj_root() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0xFFFEu16.to_le_bytes());
    v.extend_from_slice(&0x0000_0A03u32.to_le_bytes());
    v.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    // CLSID {00021201-0000-0000-C000-000000000046}
    v.extend_from_slice(&[0x01, 0x12, 0x02, 0x00, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
    for text in ["Microsoft Publisher 3.0", "MSPublisher.3"] {
        v.extend_from_slice(&(text.len() as u32 + 1).to_le_bytes());
        v.extend_from_slice(text.as_bytes());
        v.push(0);
    }
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0x71B2_39F4u32.to_le_bytes());
    v.extend_from_slice(&[0u8; 12]);
    v
}

fn compobj_quill() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0xFFFEu16.to_le_bytes());
    v.extend_from_slice(&0x0000_0A03u32.to_le_bytes());
    v.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    // CLSID {08C8F6DA-969D-11D1-8E02-00C04FB6FECE}
    v.extend_from_slice(&[0xDA, 0xF6, 0xC8, 0x08, 0x9D, 0x96, 0xD1, 0x11, 0x8E, 0x02, 0x00, 0xC0, 0x4F, 0xB6, 0xFE, 0xCE]);
    let text = "Quill96 Story Group Class";
    v.extend_from_slice(&(text.len() as u32 + 1).to_le_bytes());
    v.extend_from_slice(text.as_bytes());
    v.push(0);
    v.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0x71B2_39F4u32.to_le_bytes());
    v.extend_from_slice(&[0u8; 12]);
    v
}

/// The property-set header ([MS-OLEPS]): byte order, version, system id, CLSID, one section.
fn propset_header(fmtid: [u8; 16]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&0xFFFEu16.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&0x0002_000Au32.to_le_bytes());
    v.extend_from_slice(&[0u8; 16]);
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&fmtid);
    v.extend_from_slice(&0x30u32.to_le_bytes());
    v
}

/// A section: size, property count, (id, offset) table, then the values.
fn section(props: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let head = 8 + 8 * props.len();
    let mut table = Vec::new();
    let mut vals = Vec::new();
    for (id, v) in props {
        table.extend_from_slice(&id.to_le_bytes());
        table.extend_from_slice(&((head + vals.len()) as u32).to_le_bytes());
        vals.extend_from_slice(v);
    }
    let mut out = ((head + vals.len()) as u32).to_le_bytes().to_vec();
    out.extend_from_slice(&(props.len() as u32).to_le_bytes());
    out.extend_from_slice(&table);
    out.extend_from_slice(&vals);
    out
}

fn vt_i2(v: i16) -> Vec<u8> {
    let mut b = 2u32.to_le_bytes().to_vec();
    b.extend_from_slice(&v.to_le_bytes());
    b.extend_from_slice(&[0, 0]);
    b
}

fn document_summary() -> Vec<u8> {
    // FMTID {D5CDD502-2E9C-101B-9397-08002B2CF9AE}
    let mut v = propset_header([0x02, 0xD5, 0xCD, 0xD5, 0x9C, 0x2E, 0x1B, 0x10, 0x93, 0x97, 0x08, 0x00, 0x2B, 0x2C, 0xF9, 0xAE]);
    let mut i4 = 3u32.to_le_bytes().to_vec();
    i4.extend_from_slice(&0x0010_0000u32.to_le_bytes());
    let vt_bool_false = || {
        let mut b = 11u32.to_le_bytes().to_vec();
        b.extend_from_slice(&[0, 0, 0, 0]);
        b
    };
    v.extend_from_slice(&section(&[
        (1, vt_i2(1252)),
        (0x17, i4),
        (0x0B, vt_bool_false()),
        (0x10, vt_bool_false()),
        (0x13, vt_bool_false()),
        (0x16, vt_bool_false()),
    ]));
    v
}

/// A white 24-bit device-independent bitmap of `w` x `h` pixels (header and pixels).
fn white_dib(w: u32, h: u32) -> Vec<u8> {
    let stride = (w * 3).div_ceil(4) * 4;
    let mut v = Vec::new();
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(w as i32).to_le_bytes());
    v.extend_from_slice(&(h as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(stride * h).to_le_bytes());
    v.extend_from_slice(&[0u8; 16]);
    v.resize(v.len() + (stride * h) as usize, 0xFF);
    v
}

/// The thumbnail: a placeable-less memory metafile that blits one white bitmap, wrapped in
/// the 8-byte mapping header of spec 6.5.
fn thumbnail(spec: &NewPublication) -> Vec<u8> {
    let (w, h) = (f64::from(spec.width_emu), f64::from(spec.height_emu));
    let scale = 147.0 / w.max(h);
    let (pw, ph) = (((w * scale).round() as u32).max(1), ((h * scale).round() as u32).max(1));
    let dib = white_dib(pw, ph);
    let record = |func: u16, params: &[u8]| {
        let mut r = (((params.len() + 6) / 2) as u32).to_le_bytes().to_vec();
        r.extend_from_slice(&func.to_le_bytes());
        r.extend_from_slice(params);
        r
    };
    let i16s = |v: &[i16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let mut stretch = 0x00CC_0020u32.to_le_bytes().to_vec(); // SRCCOPY
    stretch.extend_from_slice(&0u16.to_le_bytes()); // DIB_RGB_COLORS
    stretch.extend_from_slice(&i16s(&[ph as i16, pw as i16, 0, 0, ph as i16, pw as i16, 0, 0]));
    stretch.extend_from_slice(&dib);
    let records = [
        record(0x020B, &i16s(&[0, 0])),
        record(0x020C, &i16s(&[ph as i16, pw as i16])),
        record(0x0F43, &stretch),
        record(0x0000, &[]),
    ];
    let body: Vec<u8> = records.concat();
    let max_rec = records.iter().map(|r| r.len() / 2).max().unwrap_or(0) as u32;
    let mut wmf = Vec::new();
    wmf.extend_from_slice(&1u16.to_le_bytes());
    wmf.extend_from_slice(&9u16.to_le_bytes());
    wmf.extend_from_slice(&0x0300u16.to_le_bytes());
    wmf.extend_from_slice(&(((18 + body.len()) / 2) as u32).to_le_bytes());
    wmf.extend_from_slice(&0u16.to_le_bytes());
    wmf.extend_from_slice(&max_rec.to_le_bytes());
    wmf.extend_from_slice(&0u16.to_le_bytes());
    wmf.extend_from_slice(&body);

    let ext = |p: u32| (f64::from(p) * 21.17).round() as u16;
    let mut mf = 8u16.to_le_bytes().to_vec();
    mf.extend_from_slice(&ext(pw).to_le_bytes());
    mf.extend_from_slice(&ext(ph).to_le_bytes());
    mf.extend_from_slice(&0u16.to_le_bytes());
    mf.extend_from_slice(&wmf);
    mf
}

fn summary_information(spec: &NewPublication, opt: &Options) -> Vec<u8> {
    // FMTID {F29F85E0-4FF9-1068-AB91-08002B27B3D9}
    let mut v = propset_header([0xE0, 0x85, 0x9F, 0xF2, 0xF9, 0x4F, 0x68, 0x10, 0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9]);
    let mut author = 30u32.to_le_bytes().to_vec();
    author.extend_from_slice(&1u32.to_le_bytes());
    author.extend_from_slice(&[0, 0, 0, 0]);
    let mut props = vec![(1u32, vt_i2(1252)), (4, author)];
    if opt.thumbnail {
        let mf = thumbnail(spec);
        let mut cf = 0x47u32.to_le_bytes().to_vec();
        cf.extend_from_slice(&((mf.len() + 8) as u32).to_le_bytes());
        cf.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        cf.extend_from_slice(&3u32.to_le_bytes());
        cf.extend_from_slice(&mf);
        while cf.len() % 4 != 0 {
            cf.push(0);
        }
        props.push((17, cf));
    }
    v.extend_from_slice(&section(&props));
    v
}
