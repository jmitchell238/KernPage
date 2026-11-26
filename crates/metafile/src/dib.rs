//! Small decoder for uncompressed device-independent bitmaps (1/4/8/16/24/32 bpp).

pub(crate) struct Dib {
    pub width: u32,
    pub height: u32,
    /// True when the file stored rows bottom-up (positive height).
    pub bottom_up: bool,
    /// Top-down RGBA.
    pub rgba: Vec<u8>,
}

pub(crate) enum DibError {
    Unsupported(String),
    Malformed,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Decode a DIB. `info` starts at the bitmap header and includes the colour table;
/// when `bits` is `None` the pixels follow the colour table inside `info` (WMF style),
/// otherwise they are in `bits` (EMF style).
pub(crate) fn decode(info: &[u8], bits: Option<&[u8]>) -> Result<Dib, DibError> {
    let hsize = u32_at(info, 0).ok_or(DibError::Malformed)? as usize;
    let (w, h, bpp, comp, clr_used, entry) = if hsize == 12 {
        (
            i64::from(u16_at(info, 4).ok_or(DibError::Malformed)?),
            i64::from(u16_at(info, 6).ok_or(DibError::Malformed)?),
            u16_at(info, 10).ok_or(DibError::Malformed)?,
            0u32,
            0u32,
            3usize,
        )
    } else if hsize >= 40 {
        (
            i64::from(u32_at(info, 4).ok_or(DibError::Malformed)? as i32),
            i64::from(u32_at(info, 8).ok_or(DibError::Malformed)? as i32),
            u16_at(info, 14).ok_or(DibError::Malformed)?,
            u32_at(info, 16).ok_or(DibError::Malformed)?,
            u32_at(info, 32).ok_or(DibError::Malformed)?,
            4usize,
        )
    } else {
        return Err(DibError::Malformed);
    };
    if comp != 0 {
        return Err(DibError::Unsupported(format!("DIB compression {comp}")));
    }
    if !matches!(bpp, 1 | 4 | 8 | 16 | 24 | 32) {
        return Err(DibError::Unsupported(format!("DIB {bpp} bits per pixel")));
    }
    let bottom_up = h > 0;
    let (w, h) = (w, h.abs());
    if w <= 0 || h <= 0 || w > 32768 || h > 32768 || w * h > (1 << 24) {
        // At most 16 Mpixel (64 MB of RGBA) per bitmap: a 1-bit bitmap expands 32 times, so
        // the old 64 Mpixel cap let a small file ask for 256 MB per record (PW-SEC-003).
        return Err(DibError::Malformed);
    }
    let (w, h) = (w as usize, h as usize);
    let pal_n = if bpp <= 8 {
        let max = 1usize << bpp;
        match clr_used as usize {
            0 => max,
            n => n.min(max),
        }
    } else {
        0
    };
    let pal_off = hsize;
    let palette: Vec<[u8; 3]> = (0..pal_n)
        .map(|i| {
            let o = pal_off + i * entry;
            info.get(o..o + 3).map_or([0, 0, 0], |c| [c[2], c[1], c[0]])
        })
        .collect();
    let pixels: &[u8] = match bits {
        Some(b) => b,
        None => info
            .get(pal_off + pal_n * entry..)
            .ok_or(DibError::Malformed)?,
    };
    let stride = (w * usize::from(bpp)).div_ceil(32) * 4;
    if pixels.len() < stride.checked_mul(h).ok_or(DibError::Malformed)? {
        return Err(DibError::Malformed);
    }
    let mut rgba = vec![255u8; w * h * 4];
    for y in 0..h {
        let src_row = if bottom_up { h - 1 - y } else { y };
        let row = &pixels[src_row * stride..src_row * stride + stride];
        for x in 0..w {
            let rgb: [u8; 3] = match bpp {
                1 => palette
                    .get(usize::from((row[x / 8] >> (7 - x % 8)) & 1))
                    .copied()
                    .unwrap_or([0; 3]),
                4 => {
                    let b = row[x / 2];
                    let i = if x % 2 == 0 { b >> 4 } else { b & 15 };
                    palette.get(usize::from(i)).copied().unwrap_or([0; 3])
                }
                8 => palette.get(usize::from(row[x])).copied().unwrap_or([0; 3]),
                16 => {
                    let v = u16::from_le_bytes([row[x * 2], row[x * 2 + 1]]);
                    let c = |s: u16| {
                        let c5 = ((v >> s) & 31) as u8;
                        (c5 << 3) | (c5 >> 2)
                    };
                    [c(10), c(5), c(0)]
                }
                24 => [row[x * 3 + 2], row[x * 3 + 1], row[x * 3]],
                _ => [row[x * 4 + 2], row[x * 4 + 1], row[x * 4]],
            };
            let o = (y * w + x) * 4;
            rgba[o..o + 3].copy_from_slice(&rgb);
        }
    }
    Ok(Dib {
        width: w as u32,
        height: h as u32,
        bottom_up,
        rgba,
    })
}

impl Dib {
    /// Crop to a source rectangle given in the DIB's own convention (rows counted from
    /// the bottom for bottom-up bitmaps). A rectangle that is empty or outside the bitmap
    /// leaves the bitmap whole.
    pub fn crop(self, x: i32, y: i32, w: i32, h: i32) -> Dib {
        let (bw, bh) = (self.width as i32, self.height as i32);
        let (w, h) = (w.abs(), h.abs());
        if w == 0 || h == 0 || (x == 0 && y == 0 && w >= bw && h >= bh) {
            return self;
        }
        let x0 = x.clamp(0, bw);
        let x1 = x.saturating_add(w).clamp(0, bw);
        let (y0, y1) = {
            let a = y.clamp(0, bh);
            let b = y.saturating_add(h).clamp(0, bh);
            if self.bottom_up {
                (bh - b, bh - a)
            } else {
                (a, b)
            }
        };
        if x1 <= x0 || y1 <= y0 {
            return self;
        }
        let (nw, nh) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut rgba = Vec::with_capacity(nw * nh * 4);
        for row in y0 as usize..y1 as usize {
            let s = (row * self.width as usize + x0 as usize) * 4;
            rgba.extend_from_slice(&self.rgba[s..s + nw * 4]);
        }
        Dib {
            width: nw as u32,
            height: nh as u32,
            bottom_up: self.bottom_up,
            rgba,
        }
    }

    pub fn flip(&mut self, fx: bool, fy: bool) {
        let (w, h) = (self.width as usize, self.height as usize);
        if fx {
            for y in 0..h {
                for x in 0..w / 2 {
                    for c in 0..4 {
                        self.rgba
                            .swap((y * w + x) * 4 + c, (y * w + w - 1 - x) * 4 + c);
                    }
                }
            }
        }
        if fy {
            for y in 0..h / 2 {
                for x in 0..w * 4 {
                    self.rgba.swap(y * w * 4 + x, (h - 1 - y) * w * 4 + x);
                }
            }
        }
    }
}
