//! WMF interpreter, from [MS-WMF].

use crate::dib;
use crate::player::*;
use crate::*;

const PLACEABLE_KEY: u32 = 0x9AC6_CDD7;

enum Done {
    Handled,
    Ignored,
    Skipped,
    Eof,
}

/// Bounds-checked little-endian reader over one record (offsets from record start).
struct Rec<'a>(&'a [u8]);

impl Rec<'_> {
    fn i16(&self, o: usize) -> Option<i16> {
        Some(i16::from_le_bytes(self.0.get(o..o + 2)?.try_into().ok()?))
    }
    fn u16(&self, o: usize) -> Option<u16> {
        Some(u16::from_le_bytes(self.0.get(o..o + 2)?.try_into().ok()?))
    }
    fn u32(&self, o: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.0.get(o..o + 4)?.try_into().ok()?))
    }
    /// Parameter word `i` (parameters start after the 6-byte record header).
    fn w(&self, i: usize) -> Option<f64> {
        self.i16(6 + 2 * i).map(f64::from)
    }
    fn color(&self, o: usize) -> Option<Color> {
        Some(colorref(self.0.get(o..o + 4)?))
    }
    fn points(&self, o: usize, n: usize) -> Option<Vec<(f64, f64)>> {
        let end = o.checked_add(n.checked_mul(4)?)?;
        let b = self.0.get(o..end)?;
        Some(
            b.chunks_exact(4)
                .map(|c| {
                    (
                        f64::from(i16::from_le_bytes([c[0], c[1]])),
                        f64::from(i16::from_le_bytes([c[2], c[3]])),
                    )
                })
                .collect(),
        )
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Metafile, MetafileError> {
    let mut pos = 0usize;
    let mut inch = None;
    let mut placeable: Option<Rect> = None;
    let u32at = |o: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?))
    };
    let u16at = |o: usize| -> Option<u16> {
        Some(u16::from_le_bytes(bytes.get(o..o + 2)?.try_into().ok()?))
    };
    if u32at(0) == Some(PLACEABLE_KEY) {
        if bytes.len() < 22 + 18 {
            return Err(MetafileError::TooShort);
        }
        let g = |o: usize| f64::from(u16at(o).unwrap_or(0) as i16);
        let (l, t, r, b) = (g(6), g(8), g(10), g(12));
        placeable = Some(Rect {
            left: l.min(r),
            top: t.min(b),
            right: l.max(r),
            bottom: t.max(b),
        });
        inch = u16at(14).filter(|&i| i != 0);
        pos = 22;
    }
    // META_HEADER: Type, HeaderSize (words), Version, Size, NumberOfObjects, MaxRecord, ...
    let ty = u16at(pos).ok_or(MetafileError::TooShort)?;
    let hsize = u16at(pos + 2).ok_or(MetafileError::TooShort)?;
    if ty != 1 && ty != 2 {
        return Err(MetafileError::BadHeader("not a WMF header type"));
    }
    if hsize != 9 {
        return Err(MetafileError::BadHeader("unexpected WMF header size"));
    }
    pos += usize::from(hsize) * 2;

    let mut p = Player::new();
    let mut truncated = true;
    while p.records < crate::player::MAX_RECORDS && pos + 6 <= bytes.len() {
        let words = u32at(pos).unwrap_or(0) as usize;
        if words < 3 {
            break;
        }
        let end = match words.checked_mul(2).and_then(|n| pos.checked_add(n)) {
            Some(e) if e <= bytes.len() => e,
            _ => break,
        };
        let func = u16at(pos + 4).unwrap_or(0);
        let rec = Rec(&bytes[pos..end]);
        p.records += 1;
        match record(&mut p, func, &rec, words) {
            Some(Done::Handled) => {}
            Some(Done::Ignored) => p.ignored += 1,
            Some(Done::Skipped) | None => p.skip(u32::from(func)),
            Some(Done::Eof) => {
                truncated = false;
                break;
            }
        }
        pos = end;
    }
    let bounds = p.finish_bounds(placeable);
    Ok(Metafile {
        format: Format::Wmf,
        bounds,
        inch,
        items: p.items,
        records: p.records,
        skipped: p.skipped,
        skipped_types: p.skipped_types,
        ignored: p.ignored,
        approximations: p.approximations,
        truncated,
    })
}

fn record(p: &mut Player, func: u16, r: &Rec, words: usize) -> Option<Done> {
    use Done::*;
    match func {
        0x0000 => return Some(Eof),
        0x001E => p.save(),
        0x0127 => {
            if !p.restore(i32::from(r.i16(6)?)) {
                return Some(Skipped);
            }
        }
        0x0102 => p.dc.bk_opaque = r.u16(6)? == 2,
        0x0103 => p.dc.map_mode = u32::from(r.u16(6)?),
        0x0106 => {
            p.dc.rule = if r.u16(6)? == 2 {
                FillRule::NonZero
            } else {
                FillRule::EvenOdd
            }
        }
        0x012E => p.dc.text_align = u32::from(r.u16(6)?),
        0x0201 => p.dc.bk_color = r.color(6)?,
        0x0209 => p.dc.text_color = r.color(6)?,
        // Window / viewport. Parameters are stored last-argument-first (y before x).
        0x020B => p.dc.win_org = (r.w(1)?, r.w(0)?),
        0x020C => {
            p.dc.win_ext = (r.w(1)?, r.w(0)?);
            p.dc.win_ext_set = true;
        }
        0x020D => p.dc.vp_org = (r.w(1)?, r.w(0)?),
        0x020E => p.dc.vp_ext = (r.w(1)?, r.w(0)?),
        0x020F => {
            p.dc.win_org.0 += r.w(1)?;
            p.dc.win_org.1 += r.w(0)?;
        }
        0x0211 => {
            p.dc.vp_org.0 += r.w(1)?;
            p.dc.vp_org.1 += r.w(0)?;
        }
        0x0410 | 0x0412 => {
            let (yd, yn, xd, xn) = (r.w(0)?, r.w(1)?, r.w(2)?, r.w(3)?);
            let ext = if func == 0x0410 {
                &mut p.dc.win_ext
            } else {
                &mut p.dc.vp_ext
            };
            if xd != 0.0 && yd != 0.0 {
                ext.0 = ext.0 * xn / xd;
                ext.1 = ext.1 * yn / yd;
            }
        }
        0x0214 => p.dc.pos = (r.w(1)?, r.w(0)?),
        0x0213 => {
            let (x, y) = (r.w(1)?, r.w(0)?);
            let from = p.dc.pos;
            p.emit_polyline(&[from, (x, y)]);
            p.dc.pos = (x, y);
        }
        0x041B => p.emit_rect(r.w(3)?, r.w(2)?, r.w(1)?, r.w(0)?, None),
        0x061C => p.emit_rect(r.w(5)?, r.w(4)?, r.w(3)?, r.w(2)?, Some((r.w(1)?, r.w(0)?))),
        0x0418 => p.emit_ellipse(r.w(3)?, r.w(2)?, r.w(1)?, r.w(0)?),
        0x0324 | 0x0325 => {
            let n = usize::from(r.u16(6)?);
            let pts = r.points(8, n)?;
            if func == 0x0324 {
                p.emit_polygon(&pts);
            } else {
                p.emit_polyline(&pts);
            }
        }
        0x0538 => {
            let n = usize::from(r.u16(6)?);
            let mut off = 8 + 2 * n;
            let mut polys = Vec::with_capacity(n);
            for i in 0..n {
                let c = usize::from(r.u16(8 + 2 * i)?);
                polys.push(r.points(off, c)?);
                off += c * 4;
            }
            p.emit_polypolygon(&polys);
        }
        0x0521 => {
            let len = usize::from(r.u16(6)?);
            let s = r.0.get(8..8 + len)?;
            let o = 8 + ((len + 1) & !1);
            let (y, x) = (f64::from(r.i16(o)?), f64::from(r.i16(o + 2)?));
            p.emit_text(ansi(s), x, y);
        }
        0x0A32 => {
            let (y, x) = (r.w(0)?, r.w(1)?);
            let len = usize::from(r.u16(10)?);
            let opts = r.u16(12)?;
            let o = if opts & 0x6 != 0 { 22 } else { 14 };
            let s = r.0.get(o..o + len)?;
            p.emit_text(ansi(s), x, y);
        }
        0x061D => {
            let rop = r.u32(6)?;
            let (h, w, y, x) = (r.w(2)?, r.w(3)?, r.w(4)?, r.w(5)?);
            match rop {
                0x00F0_0021 => p.emit_rect_filled(x, y, x + w, y + h, p.fill()),
                0x00FF_0062 => p.emit_rect_filled(x, y, x + w, y + h, Some(solid(WHITE))),
                0x0000_0042 => p.emit_rect_filled(x, y, x + w, y + h, Some(solid(BLACK))),
                _ => return Some(Skipped),
            }
        }
        0x0F43 | 0x0B41 | 0x0940 => return bitmap(p, func, r, words),
        // Objects.
        0x02FA => {
            let style = u32::from(r.u16(6)?);
            let width = f64::from(r.i16(8)?);
            let color = r.color(12)?;
            let o = p.make_pen(style, width, color);
            p.create_lowest(o);
        }
        0x02FC => {
            let style = u32::from(r.u16(6)?);
            let color = r.color(8)?;
            let o = p.make_brush(style, color);
            p.create_lowest(o);
        }
        0x02FB => {
            let f = r.0.get(6..)?;
            let font = logfont(f)?;
            p.create_lowest(Obj::Font(font));
        }
        0x01F9 | 0x0142 | 0x00F7 | 0x06FF => {
            let o = if func == 0x01F9 || func == 0x0142 {
                p.make_brush(3, BLACK)
            } else {
                Obj::Other
            };
            p.create_lowest(o);
        }
        0x012D => {
            if !p.select_index(usize::from(r.u16(6)?)) {
                return Some(Skipped);
            }
        }
        0x01F0 => p.delete(usize::from(r.u16(6)?)),
        // Known, no effect on the display list.
        0x0104 | 0x0107 | 0x0231 | 0x0035 | 0x0037 | 0x0234 | 0x0139 | 0x0436 | 0x0108 | 0x020A
        | 0x0626 | 0x0105 | 0x0149 => return Some(Ignored),
        _ => return Some(Skipped),
    }
    Some(Handled)
}

fn solid(color: Color) -> Fill {
    Fill {
        color,
        approximated: false,
    }
}

impl Player {
    fn emit_rect_filled(&mut self, l: f64, t: f64, r: f64, b: f64, fill: Option<Fill>) {
        let rect = self.rect(l, t, r, b);
        self.push(Item::Rect {
            rect,
            corner: None,
            fill,
            stroke: None,
        });
    }
}

/// LOGFONT: height, width, escapement, orientation, weight (i16 each), italic,
/// underline, strikeout, charset, then 4 more bytes and a 32-byte face name.
fn logfont(f: &[u8]) -> Option<Font> {
    let r = Rec(f);
    let h = f64::from(r.i16(0)?);
    let face_raw = f.get(18..50)?;
    let end = face_raw.iter().position(|&b| b == 0).unwrap_or(32);
    Some(Font {
        face: ansi(&face_raw[..end]),
        height: h.abs(),
        cell_height: h > 0.0,
        weight: r.i16(8)?.max(0) as u16,
        italic: *f.get(10)? != 0,
        underline: *f.get(11)? != 0,
        strikeout: *f.get(12)? != 0,
        charset: *f.get(13)?,
        escapement_deg: f64::from(r.i16(4)?) / 10.0,
    })
}

fn bitmap(p: &mut Player, func: u16, r: &Rec, words: usize) -> Option<Done> {
    // (dib offset, dest x/y/w/h offsets, optional source rect offsets)
    match func {
        0x0F43 => {
            let usage = r.u16(10)?;
            let (sh, sw, sy, sx) = (r.i16(12)?, r.i16(14)?, r.i16(16)?, r.i16(18)?);
            let (dh, dw, dy, dx) = (r.i16(20)?, r.i16(22)?, r.i16(24)?, r.i16(26)?);
            let data = r.0.get(28..)?;
            emit(
                p,
                usage,
                data,
                (dx, dy, dw, dh),
                Some((sx.into(), sy.into(), sw.into(), sh.into())),
            )
        }
        0x0B41 => {
            if words <= 11 + 3 {
                return Some(Done::Skipped);
            }
            let (sh, sw, sy, sx) = (r.i16(10)?, r.i16(12)?, r.i16(14)?, r.i16(16)?);
            let (dh, dw, dy, dx) = (r.i16(18)?, r.i16(20)?, r.i16(22)?, r.i16(24)?);
            let data = r.0.get(26..)?;
            emit(
                p,
                0,
                data,
                (dx, dy, dw, dh),
                Some((sx.into(), sy.into(), sw.into(), sh.into())),
            )
        }
        _ => {
            // DIBBITBLT: without a bitmap the record is 3 words longer than its parameter count.
            if words <= usize::from(func >> 8) + 3 {
                return Some(Done::Skipped);
            }
            let (sy, sx, h, w, dy, dx) = (
                r.i16(10)?,
                r.i16(12)?,
                r.i16(14)?,
                r.i16(16)?,
                r.i16(18)?,
                r.i16(20)?,
            );
            let data = r.0.get(22..)?;
            emit(
                p,
                0,
                data,
                (dx, dy, w, h),
                Some((sx.into(), sy.into(), w.into(), h.into())),
            )
        }
    }
}

fn emit(
    p: &mut Player,
    usage: u16,
    data: &[u8],
    d: (i16, i16, i16, i16),
    src: Option<(i32, i32, i32, i32)>,
) -> Option<Done> {
    let dest = (
        f64::from(d.0),
        f64::from(d.1),
        f64::from(d.2),
        f64::from(d.3),
    );
    let dib = if usage != 0 {
        Err(dib::DibError::Unsupported("DIB_PAL_COLORS".into()))
    } else {
        dib::decode(data, None)
    };
    p.emit_bitmap(dib, dest, src);
    Some(Done::Handled)
}
