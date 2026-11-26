//! Hand-built WMF/EMF files, assembled byte by byte from [MS-WMF] / [MS-EMF].

use metafile::*;

// ---------- WMF builder ----------

struct Wmf {
    recs: Vec<u8>,
}

impl Wmf {
    fn new() -> Wmf {
        Wmf { recs: Vec::new() }
    }
    fn rec(&mut self, func: u16, params: &[u8]) -> &mut Self {
        let mut p = params.to_vec();
        if p.len() % 2 == 1 {
            p.push(0);
        }
        let words = (6 + p.len()) / 2;
        self.recs.extend((words as u32).to_le_bytes());
        self.recs.extend(func.to_le_bytes());
        self.recs.extend(p);
        self
    }
    fn words(&mut self, func: u16, w: &[i16]) -> &mut Self {
        let b: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.rec(func, &b)
    }
    fn finish(&mut self, placeable: Option<(i16, i16, i16, i16, u16)>) -> Vec<u8> {
        self.rec(0, &[]);
        let mut out = Vec::new();
        if let Some((l, t, r, b, inch)) = placeable {
            out.extend(0x9AC6_CDD7u32.to_le_bytes());
            out.extend(0u16.to_le_bytes());
            for v in [l, t, r, b] {
                out.extend(v.to_le_bytes());
            }
            out.extend(inch.to_le_bytes());
            out.extend(0u32.to_le_bytes());
            out.extend(0u16.to_le_bytes());
        }
        out.extend(1u16.to_le_bytes());
        out.extend(9u16.to_le_bytes());
        out.extend(0x0300u16.to_le_bytes());
        out.extend((((18 + self.recs.len()) / 2) as u32).to_le_bytes());
        out.extend(8u16.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(&self.recs);
        out
    }
    fn brush(&mut self, style: u16, rgb: [u8; 3]) -> &mut Self {
        let mut b = style.to_le_bytes().to_vec();
        b.extend([rgb[0], rgb[1], rgb[2], 0, 0, 0]);
        self.rec(0x02FC, &b)
    }
    fn pen(&mut self, style: u16, w: i16, rgb: [u8; 3]) -> &mut Self {
        let mut b = style.to_le_bytes().to_vec();
        b.extend(w.to_le_bytes());
        b.extend(0i16.to_le_bytes());
        b.extend([rgb[0], rgb[1], rgb[2], 0]);
        self.rec(0x02FA, &b)
    }
    fn select(&mut self, i: u16) -> &mut Self {
        self.rec(0x012D, &i.to_le_bytes())
    }
}

fn red() -> Color {
    Color { r: 255, g: 0, b: 0 }
}

#[test]
fn rectangle_with_solid_brush() {
    let mut w = Wmf::new();
    w.brush(0, [255, 0, 0])
        .select(0)
        .pen(5, 0, [0; 3])
        .select(1);
    // Rectangle params: bottom, right, top, left
    w.words(0x041B, &[80, 90, 10, 20]);
    let mf = parse(&w.finish(Some((0, 0, 100, 100, 1440)))).unwrap();
    assert_eq!(mf.inch, Some(1440));
    assert_eq!(mf.bounds.right, 100.0);
    assert!(!mf.truncated);
    assert_eq!(mf.skipped, 0);
    match &mf.items[..] {
        [Item::Rect {
            rect, fill, stroke, ..
        }] => {
            assert_eq!(
                (rect.left, rect.top, rect.right, rect.bottom),
                (20.0, 10.0, 90.0, 80.0)
            );
            assert_eq!(fill.unwrap().color, red());
            assert!(stroke.is_none());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn polygon_polyline_polypolygon() {
    let mut w = Wmf::new();
    w.words(0x0106, &[2]); // winding
    w.words(0x0324, &[3, 0, 0, 10, 0, 5, 8]);
    w.words(0x0325, &[2, 1, 1, 4, 4]);
    // PolyPolygon: 2 polygons of 3 and 3 points
    w.words(0x0538, &[2, 3, 3, 0, 0, 2, 0, 1, 2, 5, 5, 7, 5, 6, 7]);
    let mf = parse(&w.finish(None)).unwrap();
    assert_eq!(mf.items.len(), 3);
    match &mf.items[0] {
        Item::Polygon { points, rule, .. } => {
            assert_eq!(points.len(), 3);
            assert_eq!(*rule, FillRule::NonZero);
            assert_eq!(points[2], Point { x: 5.0, y: 8.0 });
        }
        o => panic!("{o:?}"),
    }
    assert!(matches!(&mf.items[1], Item::Polyline { points, .. } if points.len() == 2));
    assert!(matches!(&mf.items[2], Item::PolyPolygon { polygons, .. } if polygons.len() == 2));
    // No placeable header: bounds come from the items.
    assert_eq!(mf.bounds.right, 10.0);
}

#[test]
fn ellipse_and_hatched_brush_flagged() {
    let mut w = Wmf::new();
    w.brush(2, [0, 0, 255]).select(0);
    w.words(0x0418, &[50, 60, 0, 0]);
    let mf = parse(&w.finish(None)).unwrap();
    assert_eq!(mf.approximations, 1);
    match &mf.items[0] {
        Item::Ellipse { rect, fill, .. } => {
            assert_eq!(rect.right, 60.0);
            assert!(fill.unwrap().approximated);
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn text_with_font() {
    let mut w = Wmf::new();
    // CreateFontIndirect: height -20, w 0, esc 0, orient 0, weight 700, italic 1, ... face
    let mut f = Vec::new();
    for v in [-20i16, 0, 0, 0, 700] {
        f.extend(v.to_le_bytes());
    }
    f.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    let mut face = b"Arial".to_vec();
    face.resize(32, 0);
    f.extend(face);
    w.rec(0x02FB, &f).select(0);
    w.rec(0x0209, &[0, 128, 0, 0]); // text colour
    w.words(0x012E, &[6 | 24]); // centre, baseline
                                // TextOut: len=5, "Hello" padded, y, x
    let mut t = 5i16.to_le_bytes().to_vec();
    t.extend(b"Hello\0");
    t.extend(40i16.to_le_bytes());
    t.extend(30i16.to_le_bytes());
    w.rec(0x0521, &t);
    let mf = parse(&w.finish(Some((0, 0, 200, 100, 96)))).unwrap();
    match &mf.items[..] {
        [Item::Text(t)] => {
            assert_eq!(t.text, "Hello");
            assert_eq!(t.pos, Point { x: 30.0, y: 40.0 });
            assert_eq!(t.font.face, "Arial");
            assert_eq!(t.font.weight, 700);
            assert!(t.font.italic);
            assert_eq!(t.font.height, 20.0);
            assert!(!t.font.cell_height);
            assert_eq!(t.color, Color { r: 0, g: 128, b: 0 });
            assert_eq!((t.halign, t.valign), (HAlign::Center, VAlign::Baseline));
        }
        o => panic!("{o:?}"),
    }
}

fn dib24(w: i32, h: i32, px: &[[u8; 3]]) -> Vec<u8> {
    // Bottom-up 24-bit; `px` given top-down row-major.
    let stride = ((w * 3 + 3) / 4 * 4) as usize;
    let mut d = Vec::new();
    d.extend(40u32.to_le_bytes());
    d.extend(w.to_le_bytes());
    d.extend(h.to_le_bytes());
    d.extend(1u16.to_le_bytes());
    d.extend(24u16.to_le_bytes());
    d.extend([0u8; 24]);
    for y in (0..h as usize).rev() {
        let mut row = Vec::new();
        for x in 0..w as usize {
            let p = px[y * w as usize + x];
            row.extend([p[2], p[1], p[0]]);
        }
        row.resize(stride, 0);
        d.extend(row);
    }
    d
}

#[test]
fn stretchdib_24bit() {
    let px = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [10, 20, 30]];
    let dib = dib24(2, 2, &px);
    let mut w = Wmf::new();
    let mut p = Vec::new();
    p.extend(0x00CC_0020u32.to_le_bytes()); // SRCCOPY
    p.extend(0u16.to_le_bytes()); // DIB_RGB_COLORS
    for v in [2i16, 2, 0, 0, 100, 100, 10, 10] {
        // srcH srcW ySrc xSrc destH destW yDst xDst
        p.extend(v.to_le_bytes());
    }
    p.extend(&dib);
    w.rec(0x0F43, &p);
    let mf = parse(&w.finish(Some((0, 0, 200, 200, 96)))).unwrap();
    match &mf.items[..] {
        [Item::Bitmap {
            dest,
            width,
            height,
            rgba,
        }] => {
            assert_eq!((*width, *height), (2, 2));
            assert_eq!(
                (dest.left, dest.top, dest.right, dest.bottom),
                (10.0, 10.0, 110.0, 110.0)
            );
            assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
            assert_eq!(&rgba[4..8], &[0, 255, 0, 255]);
            assert_eq!(&rgba[8..12], &[0, 0, 255, 255]);
            assert_eq!(&rgba[12..16], &[10, 20, 30, 255]);
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn compressed_dib_is_placeholder() {
    let mut dib = dib24(2, 2, &[[0; 3]; 4]);
    dib[16] = 1; // BI_RLE8
    let mut w = Wmf::new();
    let mut p = Vec::new();
    p.extend(0u32.to_le_bytes());
    p.extend(0u16.to_le_bytes());
    for v in [2i16, 2, 0, 0, 5, 5, 0, 0] {
        p.extend(v.to_le_bytes());
    }
    p.extend(&dib);
    w.rec(0x0F43, &p);
    let mf = parse(&w.finish(None)).unwrap();
    assert!(matches!(&mf.items[..], [Item::BitmapPlaceholder { .. }]));
}

#[test]
fn window_viewport_mapping_is_normalised() {
    let mut w = Wmf::new();
    w.words(0x0103, &[8]); // MM_ANISOTROPIC
    w.words(0x020C, &[1000, 2000]); // window ext (cy, cx stored y first)
    w.words(0x020B, &[0, 0]);
    w.words(0x020E, &[100, 200]); // viewport ext
    w.words(0x041B, &[500, 1000, 0, 0]);
    // Window changes mid-stream: new window org/ext halves the scale. Output stays in the
    // base window space.
    w.words(0x020C, &[500, 1000]);
    w.words(0x041B, &[250, 500, 0, 0]);
    let mf = parse(&w.finish(None)).unwrap();
    assert_eq!(mf.bounds.right, 2000.0);
    assert_eq!(mf.bounds.bottom, 1000.0);
    let rects: Vec<_> = mf
        .items
        .iter()
        .map(|i| match i {
            Item::Rect { rect, .. } => *rect,
            _ => panic!(),
        })
        .collect();
    assert_eq!(rects[0].right, 1000.0);
    assert_eq!(rects[0].bottom, 500.0);
    // Halving the window extent doubles the scale: 500x250 logical units become 1000x500
    // in the base window space, i.e. the same rectangle as before.
    assert_eq!(rects[1].right, 1000.0);
    assert_eq!(rects[1].bottom, 500.0);
}

#[test]
fn save_restore_dc_and_delete_object_reuse() {
    let mut w = Wmf::new();
    w.brush(0, [255, 0, 0]).select(0);
    w.rec(0x001E, &[]);
    w.brush(0, [0, 255, 0]).select(1);
    w.words(0x0127, &[-1]);
    w.words(0x041B, &[1, 1, 0, 0]);
    // Delete object 0 then create: lowest free slot is reused.
    w.words(0x01F0, &[0]);
    w.brush(0, [0, 0, 255]).select(0);
    w.words(0x041B, &[1, 1, 0, 0]);
    let mf = parse(&w.finish(None)).unwrap();
    let fills: Vec<_> = mf
        .items
        .iter()
        .map(|i| match i {
            Item::Rect { fill, .. } => fill.unwrap().color,
            _ => panic!(),
        })
        .collect();
    assert_eq!(fills[0], red());
    assert_eq!(fills[1], Color { r: 0, g: 0, b: 255 });
}

#[test]
fn unknown_records_counted_not_fatal() {
    let mut w = Wmf::new();
    w.words(0x7777, &[1, 2, 3]);
    w.words(0x0817, &[1, 2, 3, 4, 5, 6, 7, 8]); // Arc: unsupported
    w.words(0x0104, &[13]); // SetROP2: ignored
    let mf = parse(&w.finish(None)).unwrap();
    assert_eq!(mf.skipped, 2);
    assert_eq!(mf.ignored, 1);
    assert_eq!(mf.skipped_types.get(&0x7777), Some(&1));
}

#[test]
fn truncated_and_garbage_never_panic() {
    let mut w = Wmf::new();
    w.brush(0, [1, 2, 3]).select(0);
    w.words(0x0324, &[3, 0, 0, 10, 0, 5, 8]);
    w.words(0x041B, &[80, 90, 10, 20]);
    let full = w.finish(Some((0, 0, 100, 100, 96)));
    for n in 0..full.len() {
        let _ = parse(&full[..n]);
    }
    let _ = parse(&full[..full.len() - 3]).map(|m| assert!(m.truncated));
    // xorshift fuzz, including mutation of a valid file.
    let mut s = 0x1234_5678_9abc_def0u64;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for _ in 0..3000 {
        let mut b = full.clone();
        for _ in 0..(next() % 8 + 1) {
            let i = (next() as usize) % b.len();
            b[i] = next() as u8;
        }
        let _ = parse(&b);
        let len = (next() % 300) as usize;
        let g: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        let _ = parse(&g);
        // Garbage after a valid header.
        let mut h = full[..40].to_vec();
        h.extend(&g);
        let _ = parse(&h);
    }
    assert!(parse(&[]).is_err());
    assert!(parse(&[0u8; 64]).is_err());
}

#[test]
fn zero_size_record_terminates() {
    let mut w = Wmf::new();
    w.recs.extend([0u8; 12]);
    let mf = parse(&w.finish(None)).unwrap();
    assert!(mf.truncated);
}

// ---------- EMF ----------

fn emr(ty: u32, body: &[u8]) -> Vec<u8> {
    let mut b = body.to_vec();
    while !b.len().is_multiple_of(4) {
        b.push(0);
    }
    let mut v = ty.to_le_bytes().to_vec();
    v.extend(((b.len() + 8) as u32).to_le_bytes());
    v.extend(b);
    v
}

fn i32s(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn emf_file(recs: &[Vec<u8>]) -> Vec<u8> {
    let mut hb = i32s(&[0, 0, 99, 99, 0, 0, 1000, 1000]);
    hb.extend(0x464D_4520u32.to_le_bytes());
    hb.extend(0x10000u32.to_le_bytes());
    hb.extend([0u8; 40]);
    let mut out = emr(1, &hb);
    // pad header to 88 bytes
    out.resize(88, 0);
    out[4..8].copy_from_slice(&88u32.to_le_bytes());
    for r in recs {
        out.extend(r);
    }
    out.extend(emr(14, &[0u8; 8]));
    out
}

#[test]
fn emf_rect_polygon16_and_path() {
    let mut brush = i32s(&[1, 0]);
    brush.extend([0, 0, 255, 0]);
    brush.extend(i32s(&[0]));
    let mut poly16 = i32s(&[0, 0, 0, 0, 3]);
    for v in [0i16, 0, 10, 0, 5, 9] {
        poly16.extend(v.to_le_bytes());
    }
    let recs = vec![
        emr(39, &brush),
        emr(37, &i32s(&[1])),
        emr(43, &i32s(&[10, 10, 50, 40])),
        emr(86, &poly16),
        emr(59, &[]),
        emr(27, &i32s(&[0, 0])),
        emr(54, &i32s(&[20, 0])),
        emr(54, &i32s(&[20, 20])),
        emr(61, &[]),
        emr(60, &[]),
        emr(62, &i32s(&[0, 0, 0, 0])),
        emr(9999, &[]),
    ];
    let mf = parse(&emf_file(&recs)).unwrap();
    assert_eq!(mf.format, Format::Emf);
    assert_eq!(mf.skipped, 1);
    assert!(!mf.truncated);
    assert_eq!(mf.bounds.right, 100.0);
    assert_eq!(mf.items.len(), 3);
    assert!(matches!(&mf.items[0], Item::Rect { fill: Some(f), .. } if f.color.b == 255));
    assert!(matches!(&mf.items[1], Item::Polygon { points, .. } if points.len() == 3));
    match &mf.items[2] {
        Item::Path { segments, fill, .. } => {
            assert_eq!(segments.len(), 4);
            assert!(fill.is_some());
        }
        o => panic!("{o:?}"),
    }
}

/// PW-SEC-003: creating objects must stay fast after the object table is full (it used to
/// rescan all 65536 slots for every record), and a flood of records must stop at the cap.
#[test]
fn object_flood_is_fast_and_bounded() {
    let mut w = Wmf::new();
    for _ in 0..400_000 {
        w.brush(0, [1, 2, 3]);
    }
    let file = w.finish(Some((0, 0, 100, 100, 96)));
    let t = std::time::Instant::now();
    let mf = parse(&file).expect("parses");
    assert!(t.elapsed() < std::time::Duration::from_secs(20), "took {:?}", t.elapsed());
    assert!(mf.records >= 400_000);
}

/// PW-SEC-003: a 1-bit DIB claiming 8192 x 8192 pixels is refused (it would expand to 256 MB).
#[test]
fn oversized_dib_is_refused() {
    let mut w = Wmf::new();
    // BITMAPINFOHEADER: size 40, 8192 x 8192, 1 plane, 1 bpp, no compression, no pixel data.
    let mut info = Vec::new();
    info.extend_from_slice(&40u32.to_le_bytes());
    info.extend_from_slice(&8192i32.to_le_bytes());
    info.extend_from_slice(&8192i32.to_le_bytes());
    info.extend_from_slice(&1u16.to_le_bytes());
    info.extend_from_slice(&1u16.to_le_bytes());
    info.extend_from_slice(&[0u8; 24]);
    w.rec(0x0F43, &info);
    let file = w.finish(Some((0, 0, 100, 100, 96)));
    let mf = parse(&file).expect("parses");
    assert!(mf.items.iter().all(|i| !matches!(i, Item::Bitmap { .. })), "no bitmap is made");
}
