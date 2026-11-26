//! Picture writing (spec 4C.1): insert, share, replace, delete. Read-back through
//! `pub-format`, plus a structural comparison with the files Publisher 2021 wrote
//! (`samples/gen/picture-*.pub`).
//!
//! `cargo test -p pub-write --test picture -- --ignored write_publisher_check_files` writes the
//! files to open in Publisher to `PICTURE_OUT_DIR` (default: a folder under the system
//! temp folder).

use pub_format::{contents::Contents, items, PictureFormat, Streams};
use pub_write::{
    add_picture, add_shape, add_text_box, delete_picture, new_publication, replace_picture,
    NewPicture, NewPublication, NewShapeKind, PictureKind, PictureSource, ShapeStyle,
};
use std::path::PathBuf;

const EMU: f64 = 12_700.0;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn asset(name: &str) -> Vec<u8> {
    std::fs::read(root().join("tools/assets").join(name)).unwrap()
}
fn blank_sample() -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/blank.pub"))).unwrap()
}
fn blank_new() -> Vec<u8> {
    new_publication(&NewPublication::letter(1)).unwrap()
}
fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/gen").join(name))).unwrap()
}

fn png<'a>(image: &'a [u8], name: &'a str, l: f64, t: f64, w: f64, h: f64) -> NewPicture<'a> {
    NewPicture {
        image,
        format: PictureKind::Png,
        file_name: name,
        modified: None,
        left_pt: l,
        top_pt: t,
        width_pt: w,
        height_pt: h,
    }
}
fn jpeg<'a>(image: &'a [u8], name: &'a str, l: f64, t: f64, w: f64, h: f64) -> NewPicture<'a> {
    NewPicture {
        format: PictureKind::Jpeg,
        ..png(image, name, l, t, w, h)
    }
}
fn src<'a>(p: &NewPicture<'a>) -> PictureSource<'a> {
    PictureSource {
        image: p.image,
        format: p.format,
        file_name: p.file_name,
        modified: p.modified,
    }
}

// ------------------------------------------------------------ raw stream helpers

#[derive(Debug, Clone)]
struct Raw {
    start: usize,
    vi: u16,
    ty: u16,
    end: usize,
    kids: Vec<Raw>,
}

fn walk(d: &[u8], mut pos: usize, end: usize) -> Vec<Raw> {
    let mut out = Vec::new();
    while pos + 8 <= end {
        let vi = u16::from_le_bytes([d[pos], d[pos + 1]]);
        let ty = u16::from_le_bytes([d[pos + 2], d[pos + 3]]);
        let len = u32::from_le_bytes(d[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let next = pos + 8 + len;
        let kids = if vi & 0xF == 0xF {
            walk(d, pos + 8, next)
        } else {
            Vec::new()
        };
        out.push(Raw {
            start: pos,
            vi,
            ty,
            end: next,
            kids,
        });
        pos = next;
    }
    out
}

/// The drawing-group record and the drawing records of an `EscherStm`.
fn escher(d: &[u8]) -> (Raw, Vec<Raw>) {
    let head = walk(d, 0, d.len())[0].clone();
    let mut pos = head.end;
    let mut drawings = Vec::new();
    while pos < d.len() {
        pos += 4;
        let r = walk(d, pos, d.len())[0].clone();
        pos = r.end;
        drawings.push(r);
    }
    (head, drawings)
}

fn find_all<'a>(r: &'a Raw, ty: u16, out: &mut Vec<&'a Raw>) {
    if r.ty == ty {
        out.push(r);
    }
    for k in &r.kids {
        find_all(k, ty, out);
    }
}

/// The store entries' (record bytes, payload) of the 0xF001, if present.
fn store(esc: &[u8]) -> Option<(u16, Vec<Vec<u8>>)> {
    let (head, _) = escher(esc);
    let s = head.kids.iter().find(|k| k.ty == 0xF001)?;
    Some((
        s.vi,
        s.kids
            .iter()
            .map(|e| esc[e.start..e.end].to_vec())
            .collect(),
    ))
}

/// Header bytes of every shape container, keyed by Contents object (client data item 1).
fn containers(esc: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let (_, drawings) = escher(esc);
    let mut out = Vec::new();
    for d in &drawings {
        let mut all = Vec::new();
        find_all(d, 0xF004, &mut all);
        for c in all {
            if let Some(cd) = c.kids.iter().find(|k| k.ty == 0xF011) {
                let b = &esc[cd.start + 8..cd.end];
                if b.len() >= 10 {
                    // Publisher 2021 re-stamps the instance of the anchor and client data
                    // records (0x18A -> 0x1AA) on every save; it carries no structure.
                    let mut bytes = esc[c.start..c.end].to_vec();
                    for k in c.kids.iter().filter(|k| k.ty == 0xF010 || k.ty == 0xF011) {
                        let at = k.start - c.start;
                        bytes[at..at + 2].copy_from_slice(&0x018Au16.to_le_bytes());
                    }
                    out.push((u32::from_le_bytes(b[6..10].try_into().unwrap()), bytes));
                }
            }
        }
    }
    out
}

fn container_of(esc: &[u8], obj: u32) -> Option<Vec<u8>> {
    containers(esc).into_iter().find(|c| c.0 == obj).map(|c| c.1)
}

fn block(contents: &[u8], obj: u32) -> Option<Vec<u8>> {
    let c = Contents::parse(contents).unwrap();
    let e = c.get(obj)?;
    let len = u32::from_le_bytes(contents[e.offset..e.offset + 4].try_into().unwrap()) as usize;
    Some(contents[e.offset..e.offset + len].to_vec())
}

fn slots(contents: &[u8]) -> Vec<(u32, u16, Option<u32>)> {
    let c = Contents::parse(contents).unwrap();
    c.objects
        .iter()
        .map(|o| (o.number, o.kind, o.parent))
        .collect()
}

fn slot_count(contents: &[u8]) -> u64 {
    let io = u32::from_le_bytes(contents[0x1A..0x1E].try_into().unwrap()) as usize;
    let idx = items::decode_block(contents, io).unwrap();
    items::find_int(&idx, 1).unwrap()
}

fn pictures(doc: &pub_format::Document, page: usize) -> Vec<&pub_format::Shape> {
    doc.pages[page]
        .shapes
        .iter()
        .filter(|s| s.picture.is_some())
        .collect()
}

fn frame(doc: &pub_format::Document, s: &pub_format::Shape) -> [f64; 4] {
    let cx = f64::from(doc.page_width_emu) / 2.0;
    let cy = f64::from(doc.page_height_emu) / 2.0;
    [
        (f64::from(s.left_emu) + cx) / EMU,
        (f64::from(s.top_emu) + cy) / EMU,
        (f64::from(s.right_emu - s.left_emu)) / EMU,
        (f64::from(s.bottom_emu - s.top_emu)) / EMU,
    ]
}

fn assert_frame(got: [f64; 4], want: [f64; 4]) {
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 0.01, "{got:?} vs {want:?}");
    }
}

fn counts(out: &[u8]) -> (usize, Vec<u8>) {
    let st = Streams::from_cfb(out).unwrap();
    (store(&st.escher).map_or(0, |s| s.1.len()), st.delay)
}

// --------------------------------------------------------------------- read back

fn check_one_picture(blank: Vec<u8>) {
    let image = asset("red4x3.png");
    let (out, id) = add_picture(&blank, 0, png(&image, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 1);
    assert_eq!(doc.pages[0].shapes.len(), 1);
    assert_eq!(pics[0].object_number, id);
    assert_frame(frame(&doc, pics[0]), [100.0, 150.0, 120.0, 90.0]);
    let p = pics[0].picture.as_ref().unwrap();
    assert_eq!(p.format, PictureFormat::Png);
    assert_eq!(p.bytes, image);
    assert_eq!(pics[0].width_emu, Some((120.0 * EMU) as u32));
    assert_eq!(pics[0].height_emu, Some((90.0 * EMU) as u32));
    // The file is still a clean compound file whose streams re-read.
    Streams::from_cfb(&out).unwrap();
}

#[kernpage_samples::test]
fn insert_png_into_a_new_blank() {
    check_one_picture(blank_new());
}

#[kernpage_samples::test]
fn insert_png_into_the_research_blank() {
    check_one_picture(blank_sample());
}

#[kernpage_samples::test]
fn insert_jpeg() {
    let image = asset("green8x6.jpg");
    let (out, _) = add_picture(&blank_sample(), 0, jpeg(&image, "green8x6.jpg", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let doc = pub_format::read_bytes(&out).unwrap();
    let p = pictures(&doc, 0)[0].picture.clone().unwrap();
    assert_eq!(p.format, PictureFormat::Jpeg);
    assert_eq!(p.bytes, image);
}

#[kernpage_samples::test]
fn two_different_pictures() {
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, id1) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, id2) = add_picture(&a, 0, png(&blue, "blue5x5.png", 300.0, 300.0, 100.0, 100.0)).unwrap();
    assert_eq!((id1, id2), (296, 298));
    let doc = pub_format::read_bytes(&b).unwrap();
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 2);
    assert_eq!(pics[0].picture.as_ref().unwrap().bytes, red);
    assert_eq!(pics[1].picture.as_ref().unwrap().bytes, blue);
    assert_frame(frame(&doc, pics[1]), [300.0, 300.0, 100.0, 100.0]);
    let (n, delay) = counts(&b);
    assert_eq!(n, 2);
    assert_eq!(delay.len(), (25 + red.len()) + (25 + blue.len()));
}

#[kernpage_samples::test]
fn the_same_picture_twice_shares_one_entry() {
    let red = asset("red4x3.png");
    let (a, _) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, _) = add_picture(&a, 0, png(&red, "red4x3.png", 300.0, 300.0, 120.0, 90.0)).unwrap();
    let st = Streams::from_cfb(&b).unwrap();
    let (inst, entries) = store(&st.escher).unwrap();
    assert_eq!(inst >> 4, 1);
    assert_eq!(entries.len(), 1);
    // header(8) + type,type,uid,tag,size | ref_count at payload 24
    assert_eq!(u32::from_le_bytes(entries[0][8 + 24..8 + 28].try_into().unwrap()), 2);
    assert_eq!(st.delay.len(), 25 + red.len());
    let doc = pub_format::read_bytes(&b).unwrap();
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 2);
    assert!(pics.iter().all(|p| p.picture.as_ref().unwrap().bytes == red));
}

#[kernpage_samples::test]
fn bad_input_is_an_error() {
    let red = asset("red4x3.png");
    let blank = blank_sample();
    let p = |l, t, w, h| png(&red, "red4x3.png", l, t, w, h);
    assert!(add_picture(&blank, 5, p(0.0, 0.0, 10.0, 10.0)).is_err());
    assert!(add_picture(&blank, 0, p(0.0, 0.0, 0.0, 10.0)).is_err());
    assert!(add_picture(&blank, 0, p(f64::NAN, 0.0, 10.0, 10.0)).is_err());
    assert!(add_picture(&blank, 0, jpeg(&red, "x.jpg", 0.0, 0.0, 10.0, 10.0)).is_err());
    assert!(add_picture(&blank, 0, png(&[], "x.png", 0.0, 0.0, 10.0, 10.0)).is_err());
    assert!(add_picture(b"not a pub file", 0, p(0.0, 0.0, 10.0, 10.0)).is_err());
    assert!(delete_picture(&blank, 5).is_err());
    assert!(replace_picture(&blank, 5, src(&p(0.0, 0.0, 1.0, 1.0))).is_err());
}

// ------------------------------------------------- comparison with Publisher's files

/// `theirs` is a Publisher-written file, `ours` our result for the same edits. Everything
/// but the store entries' last four bytes, the info objects' FILETIME and `mask_frame`'s
/// 170/171 must agree.
fn same_as_publisher(ours: &[u8], theirs: &[u8], objs: &[u32], mask_frame: bool) {
    let (o, t) = (Streams::from_cfb(ours).unwrap(), Streams::from_cfb(theirs).unwrap());
    assert_eq!(o.delay, t.delay, "EscherDelayStm");
    match (store(&o.escher), store(&t.escher)) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            assert_eq!(a.0, b.0, "0xF001 instance");
            assert_eq!(a.1.len(), b.1.len());
            for (i, (x, y)) in a.1.iter().zip(&b.1).enumerate() {
                assert_eq!(x[..x.len() - 4], y[..y.len() - 4], "store entry {}", i + 1);
            }
        }
        (a, b) => panic!("picture store present: ours {} theirs {}", a.is_some(), b.is_some()),
    }
    for &obj in objs {
        assert_eq!(
            container_of(&o.escher, obj),
            container_of(&t.escher, obj),
            "shape container of object {obj}"
        );
        let (bo, bt) = (block(&o.contents, obj), block(&t.contents, obj));
        let (mut bo, mut bt) = (bo.unwrap_or_default(), bt.unwrap_or_default());
        let kind = |c: &[u8]| Contents::parse(c).unwrap().get(obj).map(|e| e.kind);
        assert_eq!(kind(&o.contents), kind(&t.contents), "kind of {obj}");
        if kind(&t.contents) == Some(102) && !bo.is_empty() {
            // FILETIME low/high (spec 4C.1.5).
            for r in [24..28, 30..34] {
                bo[r.clone()].fill(0);
                bt[r].fill(0);
            }
        }
        if mask_frame && kind(&t.contents) == Some(1) && bo.len() >= 50 {
            // Items 170/171 sit at the end (item 3 makes the replaced frame 52 bytes).
            let n = bo.len();
            for r in [n - 10..n - 6, n - 4..n] {
                bo[r.clone()].fill(0);
                bt[r].fill(0);
            }
        }
        assert_eq!(bo, bt, "Contents object {obj}");
    }
    assert_eq!(slots(&o.contents), slots(&t.contents), "object index");
    assert_eq!(slot_count(&o.contents), slot_count(&t.contents), "slot count");
    let doc_o = pub_format::read_bytes(ours).unwrap();
    let doc_t = pub_format::read_bytes(theirs).unwrap();
    let ids = |d: &pub_format::Document| -> Vec<u32> {
        d.pages[0].shapes.iter().map(|s| s.object_number).collect()
    };
    assert_eq!(ids(&doc_o), ids(&doc_t), "page child list");
    let page_obj = doc_o.pages[0].object_number;
    assert_eq!(
        block(&o.contents, page_obj),
        block(&t.contents, page_obj),
        "page object"
    );
}

#[kernpage_samples::test]
fn insert_matches_publisher_picture_new() {
    let red = asset("red4x3.png");
    let (out, _) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    same_as_publisher(&out, &gen("picture-new.pub"), &[296, 297], false);
}

#[kernpage_samples::test]
fn insert_matches_publisher_picture_jpeg() {
    let image = asset("green8x6.jpg");
    let (out, _) = add_picture(&blank_sample(), 0, jpeg(&image, "green8x6.jpg", 100.0, 150.0, 120.0, 90.0)).unwrap();
    same_as_publisher(&out, &gen("picture-jpeg.pub"), &[296, 297], false);
}

#[kernpage_samples::test]
fn insert_matches_publisher_picture_big() {
    let image = asset("grad40x30.png");
    let (out, _) = add_picture(&blank_sample(), 0, png(&image, "grad40x30.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    same_as_publisher(&out, &gen("picture-big.pub"), &[296, 297], false);
}

#[kernpage_samples::test]
fn two_pictures_match_publisher_picture_two() {
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, _) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, _) = add_picture(&a, 0, png(&blue, "blue5x5.png", 300.0, 300.0, 100.0, 100.0)).unwrap();
    same_as_publisher(&b, &gen("picture-two.pub"), &[296, 297, 298, 299], false);
}

#[kernpage_samples::test]
fn same_picture_twice_matches_publisher() {
    let red = asset("red4x3.png");
    let (a, _) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, _) = add_picture(&a, 0, png(&red, "red4x3.png", 300.0, 300.0, 120.0, 90.0)).unwrap();
    same_as_publisher(&b, &gen("picture-same-twice.pub"), &[296, 297, 298, 299], false);
}

#[kernpage_samples::test]
fn picture_over_a_text_box_matches_publisher() {
    // formatting.pub + a picture over its text box (manifest picture-over-text).
    let manifest = std::fs::read_to_string(root().join("samples/gen/manifest.json")).unwrap();
    let at = manifest.find("\"picture-over-text\"").unwrap();
    let m: serde_like::Pic = serde_like::pic_over_text(&manifest[at..]);
    let base = std::fs::read(kernpage_samples::resolve(root().join("samples/gen/formatting.pub"))).unwrap();
    let image = asset("red4x3.png");
    let (out, _) = add_picture(
        &base,
        0,
        png(&image, "red4x3.png", m.left, m.top, m.width, m.height),
    )
    .unwrap();
    let theirs = gen("picture-over-text.pub");
    let doc = pub_format::read_bytes(&out).unwrap();
    let their_doc = pub_format::read_bytes(&theirs).unwrap();
    let new_id = pictures(&doc, 0)[0].object_number;
    assert_eq!(new_id, pictures(&their_doc, 0)[0].object_number);
    // The older text box gets items 70/71 and no new item 45 (spec 4C.1.5).
    let tb = doc.pages[0].shapes.iter().find(|s| s.chain_story.is_some()).unwrap();
    let (a, b) = (
        block(&Streams::from_cfb(&out).unwrap().contents, tb.object_number).unwrap(),
        block(&Streams::from_cfb(&theirs).unwrap().contents, tb.object_number).unwrap(),
    );
    // (Byte 9 is the high byte of item 4, a stray byte in Publisher's file.)
    let (mut a, mut b) = (a, b);
    a[9] = 0;
    b[9] = 0;
    assert_eq!(a, b, "text box object");
    // The font table moved two slots (297, 298 taken, table at 299).
    let o = Streams::from_cfb(&out).unwrap();
    let t = Streams::from_cfb(&theirs).unwrap();
    assert_eq!(slots(&o.contents), slots(&t.contents));
    assert_eq!(
        block(&o.contents, new_id + 1).map(|mut b| {
            b[24..28].fill(0);
            b[30..34].fill(0);
            b
        }),
        block(&t.contents, new_id + 1).map(|mut b| {
            b[24..28].fill(0);
            b[30..34].fill(0);
            b
        })
    );
    assert_eq!(container_of(&o.escher, new_id), container_of(&t.escher, new_id));
}

/// Just enough JSON scraping to read the manifest's frame of one entry.
mod serde_like {
    pub struct Pic {
        pub left: f64,
        pub top: f64,
        pub width: f64,
        pub height: f64,
    }
    fn num(s: &str, key: &str) -> f64 {
        let at = s.find(key).unwrap() + key.len();
        let rest = s[at..].trim_start_matches([':', ' ']);
        let end = rest.find([',', '\n', '}']).unwrap();
        rest[..end].trim().parse().unwrap()
    }
    pub fn pic_over_text(s: &str) -> Pic {
        // The picture is the entry after the "pictures" key.
        let p = &s[s.find("\"pictures\"").unwrap()..];
        Pic {
            left: num(p, "\"left_pt\""),
            top: num(p, "\"top_pt\""),
            width: num(p, "\"width_pt\""),
            height: num(p, "\"height_pt\""),
        }
    }
}

// ---------------------------------------------------------------------- replace

#[kernpage_samples::test]
fn replace_matches_publisher_when_the_frame_agrees() {
    // Publisher resized the frame to 115,150 90x90; we keep it, so start from that frame.
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, id) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 115.0, 150.0, 90.0, 90.0)).unwrap();
    let b = replace_picture(&a, id, src(&png(&blue, "blue5x5.png", 0.0, 0.0, 1.0, 1.0))).unwrap();
    // Publisher left 170/171 at the old 120x90; ours follow the anchor.
    same_as_publisher(&b, &gen("picture-replaced.pub"), &[296, 297, 298], true);
    let doc = pub_format::read_bytes(&b).unwrap();
    let s = pictures(&doc, 0)[0];
    assert_eq!(s.picture.as_ref().unwrap().bytes, blue);
    assert_eq!(s.width_emu, Some((90.0 * EMU) as u32));
}

#[kernpage_samples::test]
fn replace_keeps_the_frame_and_empties_the_old_entry() {
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, id) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let b = replace_picture(&a, id, src(&png(&blue, "blue5x5.png", 0.0, 0.0, 1.0, 1.0))).unwrap();
    let doc = pub_format::read_bytes(&b).unwrap();
    let s = pictures(&doc, 0)[0];
    assert_eq!(s.object_number, id);
    assert_frame(frame(&doc, s), [100.0, 150.0, 120.0, 90.0]);
    assert_eq!(s.width_emu, Some((120.0 * EMU) as u32));
    assert_eq!(s.picture.as_ref().unwrap().bytes, blue);
    let st = Streams::from_cfb(&b).unwrap();
    let (inst, entries) = store(&st.escher).unwrap();
    assert_eq!(inst >> 4, 2, "the placeholder counts");
    // Entry 1 is the placeholder: type 0, uid 0, tag 0x00FF only.
    let mut want = [0u8; 36];
    want[18] = 0xFF;
    assert_eq!(entries[0][8..], want[..]);
    assert_eq!(entries[0][..2], [0x02, 0x00]);
    assert_eq!(st.delay.len(), 25 + blue.len());
    // Replacing again with a JPEG changes the entry type.
    let green = asset("green8x6.jpg");
    let c = replace_picture(
        &b,
        id,
        PictureSource {
            image: &green,
            format: PictureKind::Jpeg,
            file_name: "green8x6.jpg",
            modified: None,
        },
    )
    .unwrap();
    let doc = pub_format::read_bytes(&c).unwrap();
    let p = pictures(&doc, 0)[0].picture.clone().unwrap();
    assert_eq!((p.format, p.bytes), (PictureFormat::Jpeg, green));
}

#[kernpage_samples::test]
fn replace_a_shared_picture_keeps_the_old_entry() {
    // picture-same-replaced-second: the second picture (shared entry) gets blue.
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, _) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, id2) = add_picture(&a, 0, png(&red, "red4x3.png", 315.0, 300.0, 90.0, 90.0)).unwrap();
    let c = replace_picture(&b, id2, src(&png(&blue, "blue5x5.png", 0.0, 0.0, 1.0, 1.0))).unwrap();
    same_as_publisher(&c, &gen("picture-same-replaced-second.pub"), &[296, 297, 298, 299, 300], true);
    let doc = pub_format::read_bytes(&c).unwrap();
    let pics = pictures(&doc, 0);
    assert_eq!(pics[0].picture.as_ref().unwrap().bytes, red);
    assert_eq!(pics[1].picture.as_ref().unwrap().bytes, blue);
}

#[kernpage_samples::test]
fn replace_with_the_same_image_changes_nothing_but_ids() {
    let red = asset("red4x3.png");
    let (a, id) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let b = replace_picture(&a, id, src(&png(&red, "red4x3.png", 0.0, 0.0, 1.0, 1.0))).unwrap();
    let st = Streams::from_cfb(&b).unwrap();
    let (inst, entries) = store(&st.escher).unwrap();
    assert_eq!((inst >> 4, entries.len()), (1, 1));
    assert_eq!(st.delay.len(), 25 + red.len());
    let doc = pub_format::read_bytes(&b).unwrap();
    assert_eq!(pictures(&doc, 0)[0].picture.as_ref().unwrap().bytes, red);
}

// ----------------------------------------------------------------------- delete

#[kernpage_samples::test]
fn delete_the_only_picture_gives_the_blank_back() {
    let red = asset("red4x3.png");
    let blank = blank_sample();
    let (a, id) = add_picture(&blank, 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let b = delete_picture(&a, id).unwrap();
    let (st, bl) = (Streams::from_cfb(&b).unwrap(), Streams::from_cfb(&blank).unwrap());
    assert!(store(&st.escher).is_none(), "0xF001 is gone");
    assert!(st.delay.is_empty());
    assert_eq!(st.contents.len(), bl.contents.len());
    assert_eq!(slot_count(&st.contents), slot_count(&bl.contents));
    // Contents is the blank's except the offsets/length that follow the same sizes: the page
    // object and the index agree with the blank's.
    assert_eq!(slots(&st.contents), slots(&bl.contents));
    let page = pub_format::read_bytes(&blank).unwrap().pages[0].object_number;
    assert_eq!(block(&st.contents, page), block(&bl.contents, page));
    assert_eq!(st.contents, bl.contents);
    // Escher: the blank's except the two bytes of spec 4C.1.3 (lastspid and FIDCL count).
    assert_eq!(st.escher.len(), bl.escher.len());
    let diff: Vec<usize> = (0..st.escher.len()).filter(|&i| st.escher[i] != bl.escher[i]).collect();
    assert_eq!(diff.len(), 2, "{diff:?}");
    assert!(pub_format::read_bytes(&b).unwrap().pages[0].shapes.is_empty());
    same_as_publisher(&b, &gen("picture-deleted.pub"), &[], false);
}

#[kernpage_samples::test]
fn delete_the_first_of_two_matches_publisher() {
    let (red, blue) = (asset("red4x3.png"), asset("blue5x5.png"));
    let (a, id1) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, _) = add_picture(&a, 0, png(&blue, "blue5x5.png", 300.0, 300.0, 100.0, 100.0)).unwrap();
    let c = delete_picture(&b, id1).unwrap();
    same_as_publisher(&c, &gen("picture-two-deleted-first.pub"), &[296, 297, 298, 299], false);
    let doc = pub_format::read_bytes(&c).unwrap();
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0].picture.as_ref().unwrap().bytes, blue);
    // Now delete the last one: no store, empty delay stream.
    let d = delete_picture(&c, pics[0].object_number).unwrap();
    let st = Streams::from_cfb(&d).unwrap();
    assert!(store(&st.escher).is_none() && st.delay.is_empty());
    assert!(pub_format::read_bytes(&d).unwrap().pages[0].shapes.is_empty());
}

#[kernpage_samples::test]
fn delete_one_of_two_sharing_an_entry() {
    let red = asset("red4x3.png");
    let (a, id1) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let (b, _) = add_picture(&a, 0, png(&red, "red4x3.png", 300.0, 300.0, 120.0, 90.0)).unwrap();
    let c = delete_picture(&b, id1).unwrap();
    let st = Streams::from_cfb(&c).unwrap();
    let (_, entries) = store(&st.escher).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(u32::from_le_bytes(entries[0][8 + 24..8 + 28].try_into().unwrap()), 1);
    assert_eq!(st.delay.len(), 25 + red.len());
    let doc = pub_format::read_bytes(&c).unwrap();
    assert_eq!(pictures(&doc, 0)[0].picture.as_ref().unwrap().bytes, red);
}

#[kernpage_samples::test]
fn delete_rejects_a_shape_that_is_not_a_picture() {
    let blank = blank_sample();
    let a = add_shape(&blank, 0, NewShapeKind::Rectangle, 72.0, 72.0, 100.0, 50.0, ShapeStyle::default()).unwrap();
    let id = pub_format::read_bytes(&a).unwrap().pages[0].shapes[0].object_number;
    assert!(delete_picture(&a, id).is_err());
    assert!(replace_picture(&a, id, src(&png(&asset("red4x3.png"), "r.png", 0.0, 0.0, 1.0, 1.0))).is_err());
}

#[kernpage_samples::test]
fn deleting_a_picture_over_a_text_box_removes_the_overlap_reference() {
    let base = std::fs::read(kernpage_samples::resolve(root().join("samples/gen/formatting.pub"))).unwrap();
    let doc = pub_format::read_bytes(&base).unwrap();
    let tb = doc.pages[0].shapes.iter().find(|s| s.chain_story.is_some()).unwrap().clone();
    let cx = f64::from(doc.page_width_emu) / 2.0;
    let cy = f64::from(doc.page_height_emu) / 2.0;
    let (l, t) = ((f64::from(tb.left_emu) + cx) / EMU, (f64::from(tb.top_emu) + cy) / EMU);
    let red = asset("red4x3.png");
    let (a, id) = add_picture(&base, 0, png(&red, "red4x3.png", l + 5.0, t + 5.0, 50.0, 40.0)).unwrap();
    let with_refs = block(&Streams::from_cfb(&a).unwrap().contents, tb.object_number).unwrap();
    let (b, id2) = add_picture(&a, 0, png(&red, "red4x3.png", l + 10.0, t + 10.0, 50.0, 40.0)).unwrap();
    let both = block(&Streams::from_cfb(&b).unwrap().contents, tb.object_number).unwrap();
    assert_eq!(both.len(), with_refs.len() + 6);
    let c = delete_picture(&b, id2).unwrap();
    assert_eq!(block(&Streams::from_cfb(&c).unwrap().contents, tb.object_number).unwrap(), with_refs);
    let d = delete_picture(&c, id).unwrap();
    let after = block(&Streams::from_cfb(&d).unwrap().contents, tb.object_number).unwrap();
    assert!(after.len() < with_refs.len());
    let base_block = block(&Streams::from_cfb(&base).unwrap().contents, tb.object_number).unwrap();
    assert_eq!(after, base_block);
    let doc = pub_format::read_bytes(&d).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), pub_format::read_bytes(&base).unwrap().pages[0].shapes.len());
}

// ------------------------------------------------ other editing on files with pictures

#[kernpage_samples::test]
fn a_rectangle_and_a_text_box_after_a_picture() {
    let red = asset("red4x3.png");
    let (a, id) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    let b = add_shape(&a, 0, NewShapeKind::Rectangle, 300.0, 300.0, 100.0, 50.0, ShapeStyle::default()).unwrap();
    let c = add_text_box(&b, 0, 72.0, 500.0, 200.0, 30.0, "Hello").unwrap();
    let doc = pub_format::read_bytes(&c).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 3);
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0].object_number, id);
    assert_eq!(pics[0].picture.as_ref().unwrap().bytes, red);
    assert_frame(frame(&doc, pics[0]), [100.0, 150.0, 120.0, 90.0]);
    // Another picture after the shapes, then delete the first and edit again.
    let blue = asset("blue5x5.png");
    let (d, _) = add_picture(&c, 0, png(&blue, "blue5x5.png", 10.0, 10.0, 40.0, 40.0)).unwrap();
    let e = delete_picture(&d, id).unwrap();
    let f = add_shape(&e, 0, NewShapeKind::Oval, 200.0, 200.0, 50.0, 50.0, ShapeStyle::default()).unwrap();
    let doc = pub_format::read_bytes(&f).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 4);
    let pics = pictures(&doc, 0);
    assert_eq!(pics.len(), 1);
    assert_eq!(pics[0].picture.as_ref().unwrap().bytes, blue);
    assert_eq!(doc.stories.iter().filter(|s| s.contains("Hello")).count(), 1);
}

#[kernpage_samples::test]
fn a_picture_after_text_keeps_the_text_and_moves_the_font_table() {
    let base = std::fs::read(kernpage_samples::resolve(root().join("samples/gen/formatting.pub"))).unwrap();
    let before = pub_format::read_bytes(&base).unwrap();
    let red = asset("red4x3.png");
    let (a, _) = add_picture(&base, 0, png(&red, "red4x3.png", 400.0, 600.0, 60.0, 40.0)).unwrap();
    let after = pub_format::read_bytes(&a).unwrap();
    assert_eq!(after.stories, before.stories);
    assert_eq!(after.fonts, before.fonts);
    assert_eq!(
        Streams::from_cfb(&a).unwrap().quill,
        Streams::from_cfb(&base).unwrap().quill
    );
    // Delete it again: the text document reads as before.
    let id = pictures(&after, 0)[0].object_number;
    let b = delete_picture(&a, id).unwrap();
    let again = pub_format::read_bytes(&b).unwrap();
    assert_eq!(again.stories, before.stories);
    assert_eq!(again.pages[0].shapes.len(), before.pages[0].shapes.len());
}

#[kernpage_samples::test]
fn a_picture_on_page_two() {
    let two = std::fs::read(kernpage_samples::resolve(root().join("samples/two-pages.pub"))).unwrap();
    let red = asset("red4x3.png");
    let (a, _) = add_picture(&two, 1, png(&red, "red4x3.png", 50.0, 60.0, 70.0, 80.0)).unwrap();
    let doc = pub_format::read_bytes(&a).unwrap();
    assert_eq!(pictures(&doc, 1).len(), 1);
    assert!(pictures(&doc, 0).is_empty());
}

// --------------------------------------------------------- files for Publisher 2021

/// Writes the files to open in Publisher 2021 (and save once) to check they load.
/// Regenerate: `cargo test -p pub-write --test picture -- --ignored write_publisher_check_files`
#[kernpage_samples::test]
#[ignore]
fn write_publisher_check_files() {
    let dir = std::env::var("PICTURE_OUT_DIR").unwrap_or_else(|_| {
        std::env::temp_dir().join("pw-picwrite").to_string_lossy().into_owned().into()
    });
    std::fs::create_dir_all(&dir).unwrap();
    let put = |name: &str, bytes: &[u8]| std::fs::write(PathBuf::from(&dir).join(name), bytes).unwrap();
    let (red, blue, green) = (asset("red4x3.png"), asset("blue5x5.png"), asset("green8x6.jpg"));
    let blank = blank_sample();
    let (new, id) = add_picture(&blank, 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    put("ours-new.pub", &new);
    let (two, id2) = add_picture(&new, 0, png(&blue, "blue5x5.png", 300.0, 300.0, 100.0, 100.0)).unwrap();
    put("ours-two.pub", &two);
    let (same, _) = add_picture(&new, 0, png(&red, "red4x3.png", 300.0, 300.0, 120.0, 90.0)).unwrap();
    put("ours-same-twice.pub", &same);
    let (jp, _) = add_picture(&blank, 0, jpeg(&green, "green8x6.jpg", 100.0, 150.0, 120.0, 90.0)).unwrap();
    put("ours-jpeg.pub", &jp);
    put(
        "ours-replaced.pub",
        &replace_picture(&new, id, src(&png(&blue, "blue5x5.png", 0.0, 0.0, 1.0, 1.0))).unwrap(),
    );
    put("ours-deleted.pub", &delete_picture(&new, id).unwrap());
    put("ours-two-deleted-first.pub", &delete_picture(&two, id).unwrap());
    let _ = id2;
    let rect = add_shape(&new, 0, NewShapeKind::Rectangle, 300.0, 300.0, 100.0, 50.0, ShapeStyle::default()).unwrap();
    put("ours-pic-then-rect.pub", &rect);
    let text = std::fs::read(kernpage_samples::resolve(root().join("samples/gen/formatting.pub"))).unwrap();
    let (over, _) = add_picture(&text, 0, png(&red, "red4x3.png", 100.0, 100.0, 120.0, 90.0)).unwrap();
    put("ours-over-text.pub", &over);
    let fresh = blank_new();
    let (nb, _) = add_picture(&fresh, 0, png(&red, "red4x3.png", 100.0, 150.0, 120.0, 90.0)).unwrap();
    put("ours-new-from-new-publication.pub", &nb);
}

#[kernpage_samples::test]
fn the_info_object_never_carries_a_zero_filetime() {
    // Publisher 2021 refuses to open a file whose picture-info FILETIME is 0 (found by
    // bisecting our files in Publisher; spec 4C.1.5 only said "nothing depends on it").
    let red = asset("red4x3.png");
    let filetime = |out: &[u8], info: u32| {
        let c = Streams::from_cfb(out).unwrap().contents;
        let b = block(&c, info).unwrap();
        u64::from(u32::from_le_bytes(b[24..28].try_into().unwrap()))
            | u64::from(u32::from_le_bytes(b[30..34].try_into().unwrap())) << 32
    };
    let (out, id) = add_picture(&blank_sample(), 0, png(&red, "red4x3.png", 1.0, 1.0, 10.0, 10.0)).unwrap();
    // After 2020 (131_800_000_000_000_000 ticks) and before 2200.
    let t = filetime(&out, id + 1);
    assert!((131_800_000_000_000_000..160_000_000_000_000_000).contains(&t), "{t}");
    let mut p = png(&red, "red4x3.png", 1.0, 1.0, 10.0, 10.0);
    p.modified = Some(0x01D6_0000_1234_5678);
    let (out, id) = add_picture(&blank_sample(), 0, p).unwrap();
    assert_eq!(filetime(&out, id + 1), 0x01D6_0000_1234_5678);
    let b = replace_picture(&out, id, src(&png(&red, "r.png", 0.0, 0.0, 1.0, 1.0))).unwrap();
    // The new info object takes the next free id.
    assert!(filetime(&b, id + 2) >= 131_800_000_000_000_000);
}
