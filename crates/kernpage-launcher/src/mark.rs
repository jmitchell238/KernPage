//! The KernPage mark for the launcher window: the real app icon (packaging/windows/icon/png-master),
//! decoded once and cached as an egui texture. The tray icon uses the 32 px icon image (`rgba`).

use eframe::egui::{pos2, Color32, ColorImage, Painter, Rect, TextureHandle, TextureOptions};

/// Tile colour of the icon.
pub const TEAL: Color32 = Color32::from_rgb(0x1E, 0x8C, 0x99);

/// Paint the mark inside `rect`: the real app icon (png-master art) as a cached texture.
/// `tile` is kept for callers; the picture carries its own colours.
pub fn paint(painter: &Painter, rect: Rect, _tile: Color32) {
    let ctx = painter.ctx();
    let wanted = rect.width() * ctx.pixels_per_point();
    let (id, bytes) = if wanted > 64.0 { ("pw-mark-256", PNG256) } else { ("pw-mark-64", PNG64) };
    let key = eframe::egui::Id::new(id);
    let tex = match ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        Some(t) => t,
        None => {
            let (px, w, h) = decode(bytes);
            let img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
            let t = ctx.load_texture(id, img, TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(key, t.clone()));
            t
        }
    };
    painter.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

const PNG64: &[u8] = include_bytes!("../../../packaging/windows/icon/png-master/64.png");
const PNG256: &[u8] = include_bytes!("../../../packaging/windows/icon/png-master/256.png");

/// Decode a PNG to RGBA8 pixels and its width and height.
fn decode(bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().expect("icon png");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("icon frame");
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => panic!("unsupported icon png colour type {other:?}"),
    };
    (rgba, info.width, info.height)
}

/// RGBA pixels (size x size) of the app icon, for the tray icon: the 32 px icon image
/// (packaging/windows/icon/png/32.png, a downscale of the artwork) decoded at run time.
#[allow(dead_code)]
pub fn rgba(_size: u32) -> (Vec<u8>, u32) {
    const PNG32: &[u8] = include_bytes!("../../../packaging/windows/icon/png/32.png");
    let decoder = png::Decoder::new(std::io::Cursor::new(PNG32));
    let mut reader = decoder.read_info().expect("32 px icon png");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("32 px icon frame");
    buf.truncate(info.buffer_size());
    (buf, info.width)
}
