//! Picture helpers that need no GUI: reading an image's pixel size and resolution from its
//! header, and the frames Publisher gives a picture that is inserted or changed.

use crate::geometry::Rect;
use crate::model::PictureFormat;

/// What an image's header says about its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageInfo {
    pub width_px: u32,
    pub height_px: u32,
    /// Pixels per inch, horizontally and vertically, when the file records them.
    pub dpi: Option<(f64, f64)>,
}

/// The resolution assumed for an image that records none when the screen's is unknown.
/// Publisher 2021 uses the screen's resolution (120 on a 125 % display, as observed through
/// COM); see [`insert_rect`].
pub const DEFAULT_DPI: f64 = 96.0;

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// The PNG or JPEG format of `bytes`, from its signature.
pub fn sniff_format(bytes: &[u8]) -> PictureFormat {
    if bytes.starts_with(&PNG_SIGNATURE) {
        PictureFormat::Png
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        PictureFormat::Jpeg
    } else {
        PictureFormat::Unknown
    }
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be16(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_be_bytes(
        b.get(at..at + 2)?.try_into().ok()?,
    )))
}

/// Pixel size and recorded resolution (PNG `pHYs`, JPEG JFIF density) of a PNG or JPEG.
pub fn image_info(bytes: &[u8]) -> Option<ImageInfo> {
    match sniff_format(bytes) {
        PictureFormat::Png => png_info(bytes),
        PictureFormat::Jpeg => jpeg_info(bytes),
        _ => None,
    }
}

fn png_info(b: &[u8]) -> Option<ImageInfo> {
    let mut pos = 8;
    let (mut size, mut dpi) = (None, None);
    while pos + 8 <= b.len() {
        let len = be32(b, pos)? as usize;
        let ty = b.get(pos + 4..pos + 8)?;
        let data = pos + 8;
        match ty {
            b"IHDR" => size = Some((be32(b, data)?, be32(b, data + 4)?)),
            b"pHYs" if b.get(data + 8) == Some(&1) => {
                let (x, y) = (be32(b, data)?, be32(b, data + 4)?);
                if x > 0 && y > 0 {
                    dpi = Some((f64::from(x) * 0.0254, f64::from(y) * 0.0254));
                }
            }
            b"IDAT" | b"IEND" => break,
            _ => {}
        }
        pos = data.checked_add(len)?.checked_add(4)?;
    }
    let (width_px, height_px) = size?;
    Some(ImageInfo {
        width_px,
        height_px,
        dpi,
    })
}

fn jpeg_info(b: &[u8]) -> Option<ImageInfo> {
    let mut pos = 2;
    let mut dpi = None;
    while pos + 4 <= b.len() {
        if b[pos] != 0xFF {
            return None;
        }
        let marker = b[pos + 1];
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        let len = be16(b, pos + 2)? as usize;
        let data = pos + 4;
        if marker == 0xE0 && b.get(data..data + 5) == Some(b"JFIF\0".as_slice()) {
            let units = *b.get(data + 7)?;
            let (x, y) = (be16(b, data + 8)?, be16(b, data + 10)?);
            if x > 0 && y > 0 {
                dpi = match units {
                    1 => Some((f64::from(x), f64::from(y))),
                    2 => Some((f64::from(x) * 2.54, f64::from(y) * 2.54)),
                    _ => None,
                };
            }
        }
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            return Some(ImageInfo {
                width_px: be16(b, data + 3)?,
                height_px: be16(b, data + 1)?,
                dpi,
            });
        }
        pos = pos.checked_add(2 + len)?;
    }
    None
}

/// The frame of a picture inserted on a page of `page_w` x `page_h` EMU: its natural size
/// (pixels over its resolution; `screen_dpi` when the image records none), centred on the
/// page, and scaled down proportionally only when it does not fit inside the page.
pub fn insert_rect(info: ImageInfo, screen_dpi: f64, page_w: i64, page_h: i64) -> Rect {
    let valid = |d: f64| d.is_finite() && d >= 1.0;
    let fallback = if valid(screen_dpi) { screen_dpi } else { DEFAULT_DPI };
    let (dx, dy) = match info.dpi {
        Some((x, y)) if valid(x) && valid(y) => (x, y),
        _ => (fallback, fallback),
    };
    let (mut w, mut h) = (
        f64::from(info.width_px) / dx * 914_400.0,
        f64::from(info.height_px) / dy * 914_400.0,
    );
    let fit = (page_w as f64 / w).min(page_h as f64 / h);
    if fit < 1.0 {
        (w, h) = (w * fit, h * fit);
    }
    let (w, h) = ((w.round() as i64).max(1), (h.round() as i64).max(1));
    Rect::new((page_w - w) / 2, (page_h - h) / 2, w, h)
}

/// The frame after Change Picture: the old frame's height and centre, with the width taken
/// from the new image's proportions.
pub fn change_rect(old: Rect, info: ImageInfo) -> Rect {
    let aspect = f64::from(info.width_px) / f64::from(info.height_px.max(1));
    let w = ((old.h as f64 * aspect).round() as i64).max(1);
    let cx = old.x + old.w / 2;
    Rect::new(cx - w / 2, old.y, w, old.h)
}

/// Seconds-and-nanoseconds since the Unix epoch as a Windows FILETIME.
pub fn filetime_from_unix(secs: u64, nanos: u32) -> u64 {
    (secs + 11_644_473_600) * 10_000_000 + u64::from(nanos) / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, ppm: Option<u32>) -> Vec<u8> {
        let mut v = PNG_SIGNATURE.to_vec();
        v.extend(13u32.to_be_bytes());
        v.extend(b"IHDR");
        v.extend(w.to_be_bytes());
        v.extend(h.to_be_bytes());
        v.extend([8, 2, 0, 0, 0, 0, 0, 0, 0]);
        if let Some(p) = ppm {
            v.extend(9u32.to_be_bytes());
            v.extend(b"pHYs");
            v.extend(p.to_be_bytes());
            v.extend(p.to_be_bytes());
            v.push(1);
            v.extend([0; 4]);
        }
        v.extend(0u32.to_be_bytes());
        v.extend(b"IEND");
        v.extend([0; 4]);
        v
    }

    #[test]
    fn png_header_is_read() {
        let i = image_info(&png(600, 400, Some(11_811))).unwrap();
        assert_eq!((i.width_px, i.height_px), (600, 400));
        let (x, _) = i.dpi.unwrap();
        assert!((x - 300.0).abs() < 0.1);
        assert_eq!(image_info(&png(5, 6, None)).unwrap().dpi, None);
    }

    #[test]
    fn jpeg_header_is_read() {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        v.extend(b"JFIF\0");
        v.extend([1, 1, 1, 0, 72, 0, 72, 0, 0]);
        v.extend([0xFF, 0xC0, 0, 17, 8, 0x01, 0x90, 0x02, 0x58, 3]);
        v.extend([0; 9]);
        let i = image_info(&v).unwrap();
        assert_eq!((i.width_px, i.height_px), (600, 400));
        assert_eq!(i.dpi, Some((72.0, 72.0)));
    }

    #[test]
    fn natural_size_is_centred_and_only_shrinks_to_fit() {
        let page = (7_772_400, 10_058_400); // letter
        let i = ImageInfo {
            width_px: 300,
            height_px: 200,
            dpi: None,
        };
        let r = insert_rect(i, 96.0, page.0, page.1);
        assert_eq!((r.w, r.h), (2_857_500, 1_905_000));
        assert_eq!((r.x, r.y), ((page.0 - r.w) / 2, (page.1 - r.h) / 2));
        let i72 = ImageInfo {
            dpi: Some((72.0, 72.0)),
            ..i
        };
        assert_eq!(insert_rect(i72, 96.0, page.0, page.1).w, 3_810_000);
        let big = ImageInfo {
            width_px: 4000,
            height_px: 1000,
            dpi: None,
        };
        let r = insert_rect(big, 96.0, page.0, page.1);
        assert_eq!(r.w, page.0);
        assert_eq!(r.h, page.0 / 4);
    }

    #[test]
    fn change_keeps_height_and_centre() {
        let old = Rect::new(1_000_000, 2_000_000, 2_000_000, 1_000_000);
        let info = ImageInfo {
            width_px: 100,
            height_px: 400,
            dpi: None,
        };
        let r = change_rect(old, info);
        assert_eq!((r.h, r.y), (1_000_000, 2_000_000));
        assert_eq!(r.w, 250_000);
        assert_eq!(r.x + r.w / 2, old.x + old.w / 2);
    }
}
