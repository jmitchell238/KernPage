//! EMF interpreter (practical subset), from [MS-EMF].

use crate::dib;
use crate::player::*;
use crate::*;

enum Done {
    Handled,
    Ignored,
    Skipped,
    Eof,
}

struct Rec<'a>(&'a [u8]);

impl Rec<'_> {
    fn u32(&self, o: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.0.get(o..o + 4)?.try_into().ok()?))
    }
    fn i32(&self, o: usize) -> Option<i32> {
        self.u32(o).map(|v| v as i32)
    }
    fn f(&self, o: usize) -> Option<f64> {
        Some(f64::from(f32::from_le_bytes(
            self.0.get(o..o + 4)?.try_into().ok()?,
        )))
    }
    fn d(&self, o: usize) -> Option<f64> {
        self.i32(o).map(f64::from)
    }
    fn color(&self, o: usize) -> Option<Color> {
        Some(colorref(self.0.get(o..o + 4)?))
    }
    /// `n` points of 32-bit (`wide`) or 16-bit coordinates starting at `o`.
    fn points(&self, o: usize, n: usize, wide: bool) -> Option<Vec<(f64, f64)>> {
        let sz = if wide { 8 } else { 4 };
        let end = o.checked_add(n.checked_mul(sz)?)?;
        let b = self.0.get(o..end)?;
        Some(
            b.chunks_exact(sz)
                .map(|c| {
                    if wide {
                        (
                            f64::from(i32::from_le_bytes([c[0], c[1], c[2], c[3]])),
                            f64::from(i32::from_le_bytes([c[4], c[5], c[6], c[7]])),
                        )
                    } else {
                        (
                            f64::from(i16::from_le_bytes([c[0], c[1]])),
                            f64::from(i16::from_le_bytes([c[2], c[3]])),
                        )
                    }
                })
                .collect(),
        )
    }
    /// Sub-slice given an offset and size relative to the record start.
    fn span(&self, off: u32, size: u32) -> Option<&[u8]> {
        let (o, s) = (off as usize, size as usize);
        self.0.get(o..o.checked_add(s)?)
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Metafile, MetafileError> {
    let hdr = Rec(bytes);
    let hsize = hdr.u32(4).ok_or(MetafileError::TooShort)? as usize;
    if hsize < 88 || hsize > bytes.len() {
        return Err(MetafileError::BadHeader("EMR_HEADER size"));
    }
    let hb = (
        hdr.d(8).unwrap_or(0.0),
        hdr.d(12).unwrap_or(0.0),
        hdr.d(16).unwrap_or(0.0),
        hdr.d(20).unwrap_or(0.0),
    );
    let header_bounds =
        (hb.2 >= hb.0 && hb.3 >= hb.1 && (hb.2 > hb.0 || hb.3 > hb.1)).then_some(Rect {
            left: hb.0,
            top: hb.1,
            right: hb.2 + 1.0,
            bottom: hb.3 + 1.0,
        });
    let mut p = Player::new();
    let mut pos = hsize;
    let mut truncated = true;
    while p.records < crate::player::MAX_RECORDS && pos + 8 <= bytes.len() {
        let ty = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap_or([0; 4]));
        let size =
            u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap_or([0; 4])) as usize;
        if size < 8 || !size.is_multiple_of(4) {
            break;
        }
        let end = match pos.checked_add(size) {
            Some(e) if e <= bytes.len() => e,
            _ => break,
        };
        let rec = Rec(&bytes[pos..end]);
        p.records += 1;
        match record(&mut p, ty, &rec) {
            Some(Done::Handled) => {}
            Some(Done::Ignored) => p.ignored += 1,
            Some(Done::Skipped) | None => p.skip(ty),
            Some(Done::Eof) => {
                truncated = false;
                break;
            }
        }
        pos = end;
    }
    let fallback = p.base_window().or(header_bounds);
    let bounds = p.finish_bounds(fallback);
    Ok(Metafile {
        format: Format::Emf,
        bounds,
        inch: None,
        items: p.items,
        records: p.records,
        skipped: p.skipped,
        skipped_types: p.skipped_types,
        ignored: p.ignored,
        approximations: p.approximations,
        truncated,
    })
}

fn stock(p: &mut Player, n: u32) -> Option<Obj> {
    let gray = |v: u8| Color { r: v, g: v, b: v };
    let brush = |c: Color| {
        Obj::Brush(Some(Fill {
            color: c,
            approximated: false,
        }))
    };
    Some(match n {
        0 => brush(WHITE),
        1 => brush(gray(0xC0)),
        2 => brush(gray(0x80)),
        3 => brush(gray(0x40)),
        4 => brush(BLACK),
        5 => Obj::Brush(None),
        6 => p.make_pen(0, 0.0, WHITE),
        7 => p.make_pen(0, 0.0, BLACK),
        8 => p.make_pen(5, 0.0, BLACK),
        10..=14 | 16 | 17 => Obj::Font(default_font()),
        15 => Obj::Other,
        18 => brush(WHITE),
        19 => p.make_pen(0, 0.0, BLACK),
        _ => return None,
    })
}

fn xform(r: &Rec, o: usize) -> Option<[f64; 6]> {
    Some([
        r.f(o)?,
        r.f(o + 4)?,
        r.f(o + 8)?,
        r.f(o + 12)?,
        r.f(o + 16)?,
        r.f(o + 20)?,
    ])
}

/// `a` applied first, then `b` (row-vector convention).
fn then(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn bezier_item(p: &mut Player, pts: &[(f64, f64)]) {
    let saved = p.path.replace(Vec::new());
    p.path_bezier(pts, true);
    let segments = p.path.take().unwrap_or_default();
    p.path = saved;
    let stroke = p.stroke();
    let rule = p.dc.rule;
    p.push(Item::Path {
        segments,
        fill: None,
        stroke,
        rule,
    });
}

fn record(p: &mut Player, ty: u32, r: &Rec) -> Option<Done> {
    use Done::*;
    match ty {
        14 => return Some(Eof),
        // Polylines, polygons, Beziers (32-bit and 16-bit forms).
        2..=6 | 85..=89 => {
            let wide = ty < 10;
            let kind = if wide { ty } else { ty - 83 };
            let n = r.u32(24)? as usize;
            let pts = r.points(28, n, wide)?;
            let last = pts.last().copied();
            match kind {
                2 => {
                    if p.in_path() {
                        p.path_bezier(&pts, true);
                    } else {
                        bezier_item(p, &pts);
                    }
                }
                3 => p.emit_polygon(&pts),
                4 => p.emit_polyline(&pts),
                5 => {
                    if p.in_path() {
                        p.path_bezier(&pts, false);
                    } else {
                        let mut all = vec![p.dc.pos];
                        all.extend(&pts);
                        bezier_item(p, &all);
                    }
                }
                _ => {
                    if p.in_path() {
                        for &(x, y) in &pts {
                            p.path_line(x, y);
                        }
                    } else {
                        let mut all = vec![p.dc.pos];
                        all.extend(&pts);
                        p.emit_polyline(&all);
                    }
                }
            }
            if let Some(l) = last {
                p.dc.pos = l;
            }
        }
        7 | 8 | 90 | 91 => {
            let wide = ty < 10;
            let np = r.u32(24)? as usize;
            let total = r.u32(28)? as usize;
            let mut counts = Vec::new();
            for i in 0..np {
                counts.push(r.u32(32 + 4 * i)? as usize);
            }
            let mut off = 32 + 4 * np;
            if counts.iter().try_fold(0usize, |a, &c| a.checked_add(c)) != Some(total) {
                return Some(Skipped);
            }
            let sz = if wide { 8 } else { 4 };
            let mut polys = Vec::new();
            for c in counts {
                polys.push(r.points(off, c, wide)?);
                off += c * sz;
            }
            if ty == 8 || ty == 91 {
                p.emit_polypolygon(&polys);
            } else {
                for poly in &polys {
                    p.emit_polyline(poly);
                }
            }
        }
        9 => {
            p.dc.win_ext = (r.d(8)?, r.d(12)?);
            p.dc.win_ext_set = true;
        }
        10 => p.dc.win_org = (r.d(8)?, r.d(12)?),
        11 => p.dc.vp_ext = (r.d(8)?, r.d(12)?),
        12 => p.dc.vp_org = (r.d(8)?, r.d(12)?),
        17 => p.dc.map_mode = r.u32(8)?,
        18 => p.dc.bk_opaque = r.u32(8)? == 2,
        19 => {
            p.dc.rule = if r.u32(8)? == 2 {
                FillRule::NonZero
            } else {
                FillRule::EvenOdd
            }
        }
        22 => p.dc.text_align = r.u32(8)?,
        24 => p.dc.text_color = r.color(8)?,
        25 => p.dc.bk_color = r.color(8)?,
        27 => {
            let (x, y) = (r.d(8)?, r.d(12)?);
            p.dc.pos = (x, y);
            if p.in_path() {
                p.path_move(x, y);
            }
        }
        54 => {
            let (x, y) = (r.d(8)?, r.d(12)?);
            if p.in_path() {
                p.path_line(x, y);
            } else {
                let from = p.dc.pos;
                p.emit_polyline(&[from, (x, y)]);
            }
            p.dc.pos = (x, y);
        }
        33 => p.save(),
        34 => {
            if !p.restore(r.i32(8)?) {
                return Some(Skipped);
            }
        }
        35 => p.dc.world = xform(r, 8)?,
        36 => {
            let x = xform(r, 8)?;
            let w = p.dc.world;
            p.dc.world = match r.u32(32)? {
                1 => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                2 => then(x, w),
                3 => then(w, x),
                4 => x,
                _ => return Some(Skipped),
            };
        }
        37 => {
            let h = r.u32(8)?;
            let ok = if h & 0x8000_0000 != 0 {
                match stock(p, h & 0x7FFF_FFFF) {
                    Some(o) => {
                        p.select(o);
                        true
                    }
                    None => false,
                }
            } else {
                p.select_index(h as usize)
            };
            if !ok {
                return Some(Skipped);
            }
        }
        40 => p.delete(r.u32(8)? as usize),
        38 => {
            let h = r.u32(8)? as usize;
            let o = p.make_pen(r.u32(12)?, r.d(16)?, r.color(24)?);
            p.create_at(h, o);
        }
        95 => {
            let h = r.u32(8)? as usize;
            let style = r.u32(28)?;
            let width = if style & 0x30000 == 0x10000 {
                f64::from(r.u32(32)?)
            } else {
                0.0
            };
            let o = p.make_pen(style, width, r.color(40)?);
            p.create_at(h, o);
        }
        39 => {
            let h = r.u32(8)? as usize;
            let o = p.make_brush(r.u32(12)?, r.color(16)?);
            p.create_at(h, o);
        }
        93 | 94 => {
            let h = r.u32(8)? as usize;
            let o = p.make_brush(3, BLACK);
            p.create_at(h, o);
        }
        49 => {
            let h = r.u32(8)? as usize;
            p.create_at(h, Obj::Other);
        }
        82 => {
            let h = r.u32(8)? as usize;
            let height = r.d(12)?;
            let face_raw = r.0.get(40..104)?;
            let units: Vec<u16> = face_raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&u| u != 0)
                .collect();
            let font = Font {
                face: String::from_utf16_lossy(&units),
                height: height.abs(),
                cell_height: height > 0.0,
                weight: r.i32(28)?.clamp(0, 1000) as u16,
                italic: *r.0.get(32)? != 0,
                underline: *r.0.get(33)? != 0,
                strikeout: *r.0.get(34)? != 0,
                charset: *r.0.get(35)?,
                escapement_deg: r.d(20)? / 10.0,
            };
            p.create_at(h, Obj::Font(font));
        }
        42 => p.emit_ellipse(r.d(8)?, r.d(12)?, r.d(16)?, r.d(20)?),
        43 => p.emit_rect(r.d(8)?, r.d(12)?, r.d(16)?, r.d(20)?, None),
        44 => p.emit_rect(
            r.d(8)?,
            r.d(12)?,
            r.d(16)?,
            r.d(20)?,
            Some((r.d(24)?, r.d(28)?)),
        ),
        59 => p.path = Some(Vec::new()),
        60 => {
            if let Some(path) = p.path.take() {
                p.last_path = path;
            }
        }
        61 => p.path_close(),
        68 => {
            p.path = None;
            p.last_path.clear();
        }
        62..=64 => {
            let segments = std::mem::take(&mut p.last_path);
            if segments.is_empty() {
                return Some(Skipped);
            }
            let fill = if ty != 64 { p.fill() } else { None };
            let stroke = if ty != 62 { p.stroke() } else { None };
            let rule = p.dc.rule;
            p.push(Item::Path {
                segments,
                fill,
                stroke,
                rule,
            });
        }
        83 | 84 => {
            let (x, y) = (r.d(36)?, r.d(40)?);
            let n = r.u32(44)?;
            let off = r.u32(48)?;
            let text = if ty == 84 {
                let b = r.span(off, n.checked_mul(2)?)?;
                let u: Vec<u16> = b
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&u)
            } else {
                ansi(r.span(off, n)?)
            };
            p.emit_text(text, x, y);
        }
        81 => {
            let (xd, yd) = (r.d(24)?, r.d(28)?);
            let (xs, ys, cxs, cys) = (r.i32(32)?, r.i32(36)?, r.i32(40)?, r.i32(44)?);
            let info = r.span(r.u32(48)?, r.u32(52)?)?;
            let bits = r.span(r.u32(56)?, r.u32(60)?)?;
            let usage = r.u32(64)?;
            let (cxd, cyd) = (r.d(72)?, r.d(76)?);
            let d = if usage != 0 {
                Err(dib::DibError::Unsupported("DIB_PAL_COLORS".into()))
            } else {
                dib::decode(info, Some(bits))
            };
            p.emit_bitmap(d, (xd, yd, cxd, cyd), Some((xs, ys, cxs, cys)));
        }
        76 => {
            let (xd, yd, cxd, cyd) = (r.d(24)?, r.d(28)?, r.d(32)?, r.d(36)?);
            let (xs, ys) = (r.i32(44)?, r.i32(48)?);
            let (offi, szi, offb, szb) = (r.u32(84)?, r.u32(88)?, r.u32(92)?, r.u32(96)?);
            if szi == 0 {
                return Some(Skipped);
            }
            let d = dib::decode(r.span(offi, szi)?, Some(r.span(offb, szb)?));
            p.emit_bitmap(
                d,
                (xd, yd, cxd, cyd),
                Some((xs, ys, cxd as i32, cyd as i32)),
            );
        }
        // Known, no effect on the display list.
        13 | 16 | 20 | 21 | 23 | 48..=52 | 58 | 70 | 98 | 115 => return Some(Ignored),
        _ => return Some(Skipped),
    }
    Some(Handled)
}
