//! Drawing document shapes with egui (used by the page view and the thumbnails).

use crate::fontmgr::FontMgr;
use crate::metapic::MetaPic;
use eframe::egui::{
    self, text::LayoutJob, Align, Color32, Context, FontId, Painter, Pos2, Stroke, StrokeKind,
    TextFormat, TextureHandle, Vec2,
};
use kernpage_core::geometry::{emu_to_px, pt_to_px};
use kernpage_core::wrap::free_spans;
use kernpage_core::{
    Align as PAlign, LineSpacing, ParaSpacing, Paragraph, PictureFormat, Rect, Rgb, Shape,
    ShapeKind,
};
use pub_format::wrap::Poly;
use std::collections::HashMap;
use std::sync::Arc;

/// Fallback text inset in points, for a box whose margins are not known.
pub const INSET_PT: f64 = 3.6;
/// Extra space between a box's margin and its text (see `pub2pdf::TEXT_PAD_PT`).
pub const TEXT_PAD_PT: f64 = 0.72;

/// Where text starts inside a shape (left, top, right, bottom) in screen pixels at `zoom`:
/// the box's own text-frame margins plus the 0.72 pt Publisher adds (same as the PDF export).
pub fn shape_inset_px(shape: &kernpage_core::Shape, zoom: f64) -> [f32; 4] {
    shape
        .inset_emu
        .map(|e| (emu_to_px(e, zoom) + pt_to_px(TEXT_PAD_PT, zoom)) as f32)
}
/// Largest picture edge uploaded as a texture.
const MAX_TEXTURE_EDGE: u32 = 2048;
/// Pictures come from untrusted files: refuse more than this many pixels per side and more than
/// this much decoder memory (security audit PW-SEC-002), instead of trusting the header.
pub const MAX_PICTURE_EDGE: u32 = 16_384;
pub const MAX_PICTURE_ALLOC: u64 = 256 << 20;

/// Decode a PNG/JPEG with explicit dimension and memory limits. `None` for anything unreadable
/// or over the limits.
pub fn decode_picture(data: &[u8]) -> Option<image::DynamicImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_PICTURE_EDGE);
    limits.max_image_height = Some(MAX_PICTURE_EDGE);
    limits.max_alloc = Some(MAX_PICTURE_ALLOC);
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data)).with_guessed_format().ok()?;
    reader.limits(limits);
    reader.decode().ok()
}

#[cfg(test)]
mod picture_limit_tests {
    use super::*;

    fn png_with_size(w: u32, h: u32) -> Vec<u8> {
        // A valid PNG signature + IHDR claiming w x h, then no pixel data: a decoder that
        // trusted the header would try to allocate w*h*4 bytes.
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        v.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
        let mut chunk = b"IHDR".to_vec();
        chunk.extend_from_slice(&ihdr);
        v.extend_from_slice(&chunk);
        v.extend_from_slice(&crc32(&chunk).to_be_bytes());
        v
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }

    #[test]
    fn a_picture_claiming_huge_dimensions_is_refused_without_allocating() {
        assert!(decode_picture(&png_with_size(60_000, 60_000)).is_none());
        assert!(decode_picture(&png_with_size(u32::MAX >> 1, 4)).is_none());
        assert!(decode_picture(&png_with_size(16_384, 16_384)).is_none()); // in range, but no data
    }

    #[test]
    fn junk_and_empty_pictures_are_refused() {
        assert!(decode_picture(&[]).is_none());
        assert!(decode_picture(b"not an image").is_none());
    }

    #[test]
    fn a_small_real_png_still_decodes() {
        let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packaging/windows/icon/png-master/256.png")).unwrap();
        let img = decode_picture(&png).expect("icon decodes");
        assert_eq!((img.width(), img.height()), (256, 256));
    }
}

pub fn color(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

/// Decoded pictures, keyed by the address of their shared byte buffer. Clear it when the
/// document is replaced (addresses may be reused).
#[derive(Default)]
pub struct PictureCache {
    map: HashMap<usize, Option<TextureHandle>>,
    /// Metafile clip art, converted once per picture (zoom-independent).
    metas: HashMap<usize, Option<Arc<MetaPic>>>,
}

impl PictureCache {
    pub fn clear(&mut self) {
        self.map.clear();
        self.metas.clear();
    }

    fn get_meta(&mut self, ctx: &Context, data: &Arc<Vec<u8>>) -> Option<Arc<MetaPic>> {
        let key = Arc::as_ptr(data) as usize;
        self.metas
            .entry(key)
            .or_insert_with(|| {
                let mf = metafile::parse(data).ok()?;
                Some(Arc::new(MetaPic::build(ctx, &mf, key)))
            })
            .clone()
    }

    fn get(&mut self, ctx: &Context, data: &Arc<Vec<u8>>) -> Option<TextureHandle> {
        let key = Arc::as_ptr(data) as usize;
        self.map
            .entry(key)
            .or_insert_with(|| {
                let img = decode_picture(data)?;
                let img = if img.width().max(img.height()) > MAX_TEXTURE_EDGE {
                    img.thumbnail(MAX_TEXTURE_EDGE, MAX_TEXTURE_EDGE)
                } else {
                    img
                };
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                Some(ctx.load_texture(format!("pic{key}"), ci, egui::TextureOptions::LINEAR))
            })
            .clone()
    }
}

/// Everything needed to draw one page.
pub struct PageView<'a> {
    pub painter: &'a Painter,
    pub fonts: &'a FontMgr,
    pub pics: &'a mut PictureCache,
    /// Screen position of the page's top-left corner.
    pub origin: Pos2,
    pub zoom: f64,
    /// Draw a thin outline around text boxes (View > Boundaries).
    pub boundaries: bool,
    /// The page's shapes in z-order, for text wrap around objects (spec 4K).
    pub shapes: &'a [Shape],
    /// The table cell being edited in place (only that cell's text is hidden).
    pub editing_cell: Option<usize>,
}

impl PageView<'_> {
    pub fn px(&self, emu: i64) -> f32 {
        emu_to_px(emu, self.zoom) as f32
    }

    pub fn screen_rect(&self, r: Rect) -> egui::Rect {
        let min = self.origin + Vec2::new(self.px(r.x), self.px(r.y));
        egui::Rect::from_min_size(min, Vec2::new(self.px(r.w), self.px(r.h)))
    }

    /// Draw `shape`, optionally at a different rectangle (during a drag). When
    /// `hide_text` is set the shape's text is not drawn (an editor is over it). A rotated
    /// shape (spec 4H) is drawn in its unrotated rectangle, then turned about its centre.
    /// Draw the master page's shapes (spec 4T.6): behind the page's own shapes, with a
    /// page-number field showing `number`. Text wrap around the master's shapes is not
    /// applied (~).
    pub fn draw_master(&mut self, ctx: &Context, master: &[Shape], number: &str) {
        let saved = std::mem::take(&mut self.shapes);
        for s in master {
            self.draw_shape_numbered(ctx, s, number);
        }
        self.shapes = saved;
    }

    /// [`Self::draw_shape`] with page-number fields showing `number`.
    pub fn draw_shape_numbered(&mut self, ctx: &Context, shape: &Shape, number: &str) {
        match shape.with_page_number(number) {
            Some(s) => self.draw_shape(ctx, &s, None, false),
            None => self.draw_shape(ctx, shape, None, false),
        }
    }

    pub fn draw_shape(&mut self, ctx: &Context, shape: &Shape, at: Option<Rect>, hide_text: bool) {
        let rect = self.screen_rect(at.unwrap_or(shape.rect));
        let turn = (shape.rotation_deg.rem_euclid(360.0) != 0.0).then(|| Turn {
            centre: rect.center(),
            angle: shape.rotation_deg.to_radians(),
        });
        let stroke = shape
            .line
            .map(|l| Stroke::new(self.px(l.width_emu).max(0.75), color(l.color)));
        let painter = self.painter;
        match &shape.kind {
            ShapeKind::TextBox { paragraphs } => {
                if let Some(f) = shape.fill {
                    self.fill_rect(turn, rect, color(f));
                }
                if let Some(s) = stroke {
                    self.stroke_rect(turn, rect, s);
                } else if self.boundaries {
                    let s = Stroke::new(1.0, Color32::from_gray(190));
                    let corners = [
                        rect.left_top(),
                        rect.right_top(),
                        rect.right_bottom(),
                        rect.left_bottom(),
                        rect.left_top(),
                    ]
                    .map(|p| turn.map_or(p, |t| t.pt(p)));
                    for seg in egui::Shape::dashed_line(&corners, s, 3.0, 3.0) {
                        painter.add(seg);
                    }
                }
                if !hide_text {
                    let cols = Columns {
                        count: shape.columns.max(1) as usize,
                        gap: self.px(shape.column_gap_emu),
                    };
                    // Text wrap (spec 4K): one-column, unrotated boxes that are not being
                    // dragged; other boxes ignore wrapping objects for now.
                    let obstacles = if turn.is_none() && cols.count <= 1 && at.is_none() {
                        kernpage_core::wrap::obstacles(self.shapes, shape.id)
                    } else {
                        Vec::new()
                    };
                    if obstacles.is_empty() {
                        self.draw_text(ctx, rect, paragraphs, turn, cols, shape_inset_px(shape, self.zoom));
                    } else {
                        self.draw_wrapped(rect, paragraphs, &obstacles, shape_inset_px(shape, self.zoom));
                    }
                }
            }
            ShapeKind::Picture {
                data,
                format: PictureFormat::Wmf | PictureFormat::Emf,
                ..
            } if self.pics.get_meta(ctx, data).is_some() => {
                if let Some(pic) = self.pics.get_meta(ctx, data) {
                    let xf = |p: Pos2| turn.map_or(p, |t| t.pt(p));
                    let angle = turn.map_or(0.0, |t| t.angle);
                    // Unturned pictures are clipped to their frame.
                    let clipped = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
                    let target = if turn.is_none() { &clipped } else { painter };
                    pic.draw(target, self.fonts, rect, &xf, angle);
                }
            }
            ShapeKind::Picture { data, .. } => match self.pics.get(ctx, data) {
                Some(tex) => self.image(turn, rect, tex.id()),
                None => {
                    self.fill_rect(turn, rect, Color32::from_gray(225));
                    let gray = Stroke::new(1.0, Color32::GRAY);
                    self.stroke_rect(turn, rect, gray);
                    let t = |p: Pos2| turn.map_or(p, |t| t.pt(p));
                    painter.line_segment([t(rect.left_top()), t(rect.right_bottom())], gray);
                    painter.line_segment([t(rect.right_top()), t(rect.left_bottom())], gray);
                }
            },
            // A group draws nothing; its members draw themselves.
            ShapeKind::Group => {}
            ShapeKind::Rectangle => {
                if let Some(f) = shape.fill {
                    self.fill_rect(turn, rect, color(f));
                }
                if let Some(s) = stroke {
                    self.stroke_rect(turn, rect, s);
                }
            }
            ShapeKind::Ellipse => {
                if turn.is_some() {
                    let pts = ellipse_points(rect, turn);
                    if let Some(f) = shape.fill {
                        painter.add(egui::Shape::convex_polygon(
                            pts.clone(),
                            color(f),
                            Stroke::NONE,
                        ));
                    }
                    if let Some(s) = stroke {
                        painter.add(egui::Shape::closed_line(pts, s));
                    }
                } else {
                    if let Some(f) = shape.fill {
                        painter.add(egui::Shape::ellipse_filled(
                            rect.center(),
                            rect.size() / 2.0,
                            color(f),
                        ));
                    }
                    if let Some(s) = stroke {
                        painter.add(egui::Shape::ellipse_stroke(
                            rect.center(),
                            rect.size() / 2.0,
                            s,
                        ));
                    }
                }
            }
            ShapeKind::Line => {
                let s = stroke.unwrap_or(Stroke::new(1.0, Color32::BLACK));
                let t = |p: Pos2| turn.map_or(p, |t| t.pt(p));
                let (a, b) = if shape.flip_h != shape.flip_v {
                    (rect.left_bottom(), rect.right_top())
                } else {
                    (rect.left_top(), rect.right_bottom())
                };
                painter.line_segment([t(a), t(b)], s);
            }
            ShapeKind::Table {
                rows,
                cols,
                col_widths_emu,
                row_heights_emu,
                cells,
                ..
            } => {
                let (rows, cols) = (
                    (*rows).clamp(1, 1000) as usize,
                    (*cols).clamp(1, 1000) as usize,
                );
                if let Some(f) = shape.fill {
                    self.fill_rect(turn, rect, color(f));
                }
                let grid = Stroke::new(
                    (pt_to_px(0.75, self.zoom) as f32).max(0.75),
                    stroke.map_or(Color32::BLACK, |s| s.color),
                );
                // Cumulative cell edges in pixels from the table's top-left: the stored
                // sizes when present, else equal cells filling the frame.
                let edges = |sizes: &[i64], n: usize, total: f32| -> Vec<f32> {
                    let mut e = vec![0.0f32];
                    for i in 0..n {
                        let step = if sizes.len() == n {
                            emu_to_px(sizes[i], self.zoom) as f32
                        } else {
                            total / n as f32
                        };
                        e.push(e[i] + step);
                    }
                    e
                };
                let xs = edges(col_widths_emu, cols, rect.width());
                let ys = edges(row_heights_emu, rows, rect.height());
                for r in 0..rows {
                    for c in 0..cols {
                        let cell_rect = egui::Rect::from_min_max(
                            rect.min + Vec2::new(xs[c], ys[r]),
                            rect.min + Vec2::new(xs[c + 1], ys[r + 1]),
                        );
                        self.stroke_rect(turn, cell_rect, grid);
                        if hide_text && self.editing_cell.is_none_or(|e| e == r * cols + c) {
                            continue;
                        }
                        if let Some(cell) = cells.get(r * cols + c) {
                            let inset = [emu_to_px(cell.margins_emu[0], self.zoom) as f32; 4];
                            self.draw_text_inset(
                                ctx,
                                cell_rect,
                                &cell.paragraphs,
                                turn,
                                Columns::default(),
                                inset,
                            );
                        }
                    }
                }
            }
        }
    }

    fn fill_rect(&self, turn: Option<Turn>, rect: egui::Rect, col: Color32) {
        match turn {
            None => {
                self.painter.rect_filled(rect, 0.0, col);
            }
            Some(t) => {
                self.painter.add(egui::Shape::convex_polygon(
                    t.corners(rect),
                    col,
                    Stroke::NONE,
                ));
            }
        }
    }

    fn stroke_rect(&self, turn: Option<Turn>, rect: egui::Rect, s: Stroke) {
        match turn {
            None => {
                self.painter.rect_stroke(rect, 0.0, s, StrokeKind::Middle);
            }
            Some(t) => {
                self.painter
                    .add(egui::Shape::closed_line(t.corners(rect), s));
            }
        }
    }

    fn image(&self, turn: Option<Turn>, rect: egui::Rect, tex: egui::TextureId) {
        let uv = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        match turn {
            None => {
                self.painter.image(tex, rect, uv, Color32::WHITE);
            }
            Some(t) => {
                let mut mesh = egui::Mesh::with_texture(tex);
                let uvs = [
                    uv.left_top(),
                    uv.right_top(),
                    uv.right_bottom(),
                    uv.left_bottom(),
                ];
                for (pos, uv) in t.corners(rect).into_iter().zip(uvs) {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos,
                        uv,
                        color: Color32::WHITE,
                    });
                }
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                self.painter.add(egui::Shape::mesh(mesh));
            }
        }
    }

    /// Lay out and draw paragraphs inside `rect`, in `cols` text columns (spec 4I). The
    /// text is clipped to the column; a turned shape is not clipped and keeps one column.
    fn draw_text(
        &self,
        ctx: &Context,
        rect: egui::Rect,
        paragraphs: &[Paragraph],
        turn: Option<Turn>,
        cols: Columns,
        inset: [f32; 4],
    ) {
        self.draw_text_inset(ctx, rect, paragraphs, turn, cols, inset);
    }

    /// Text of a one-column, unrotated box flowed around `obstacles` (spec 4K; regions in
    /// page points, see `kernpage_core::wrap`): each line's band is cut into free pieces,
    /// text fills the pieces left to right on the same baseline, then the next line.
    fn draw_wrapped(
        &self,
        rect: egui::Rect,
        paragraphs: &[Paragraph],
        obstacles: &[Poly],
        inset: [f32; 4],
    ) {
        let k = pt_to_px(1.0, self.zoom) as f32;
        let width = (rect.width() - inset[0] - inset[2]).max(1.0);
        let height = (rect.height() - inset[1] - inset[3]).max(0.0);
        let (left, top) = (rect.min.x + inset[0], rect.min.y + inset[1]);
        let base = self
            .painter
            .with_clip_rect(rect.intersect(self.painter.clip_rect()));
        let mut row_height = |f: &FontId| base.fonts_mut(|fv| fv.row_height(f));
        let to_pt = |p: Pos2| ((p.x - self.origin.x) / k, (p.y - self.origin.y) / k);
        let mut y = 0.0f32;
        let labels = kernpage_core::text::list_labels(paragraphs);
        for (pi, p) in paragraphs.iter().enumerate() {
            y += pt_to_px(f64::from(p.spacing.space_before_pt), self.zoom) as f32;
            let mut cur = p.clone();
            // spec: ? no rule for the narrowest usable piece; about half an em.
            let min_piece = 0.5 * p.base_run().size_pt;
            // Indents and the list label (spec 4R): the first line starts at its own
            // indent, the label hangs before it.
            let label_g = labels[pi].as_deref().map(|text| {
                let (text, font_id, col) = self.label_style(p, text);
                base.layout_no_wrap(text, font_id, col)
            });
            let geom = indent_layout(
                p.indents,
                width,
                label_g.as_ref().map(|g| g.size().x),
                self.zoom,
            );
            let right_pt = p.indents.right_pt.max(0.0);
            let mut first_line = true;
            let mut first = true;
            while first || !cur.text().is_empty() {
                first = false;
                if y > height {
                    return;
                }
                let (indent_px, line_w) = if first_line {
                    (geom.first_x, geom.first_w)
                } else {
                    (geom.rest_x, geom.rest_w)
                };
                let probe = base.layout_job(paragraph_job(
                    self.fonts,
                    &cur,
                    line_w,
                    self.zoom,
                    &mut row_height,
                ));
                let line_h = probe.rows.first().map_or(0.0, |r| r.height());
                let (x0, y0) = to_pt(Pos2::new(left, top + y));
                let y1 = y0 + line_h / k;
                let spans = free_spans(
                    obstacles,
                    y0,
                    y1,
                    x0 + indent_px / k,
                    (x0 + width / k - right_pt).max(x0 + indent_px / k),
                    min_piece,
                );
                if spans.is_empty() {
                    // Nothing free in this band: try the next one down.
                    y += line_h.max(1.0);
                    continue;
                }
                let chars: Vec<char> = cur.text().chars().collect();
                let mut used = 0usize;
                let mut placed: Vec<(f32, f32, Arc<egui::Galley>)> = Vec::new();
                for fallback in [false, true] {
                    if !placed.is_empty() {
                        break;
                    }
                    for &(a, b) in &spans {
                        let wpx = (b - a) * k;
                        let rest = skip_chars(&cur, used);
                        if rest.text().is_empty() {
                            break;
                        }
                        let g = base.layout_job(paragraph_job(
                            self.fonts,
                            &rest,
                            wpx,
                            self.zoom,
                            &mut row_height,
                        ));
                        let Some(n) = g.rows.first().map(|r| r.char_count_including_newline().0)
                        else {
                            continue;
                        };
                        let end = used + n;
                        // A first word that fits no piece waits for a wider one.
                        let mid_word = spans.len() > 1
                            && !fallback
                            && end < chars.len()
                            && end > 0
                            && chars[end - 1] != ' '
                            && chars[end] != ' '
                            && chars[used] != ' '
                            && !chars[used..end].contains(&' ');
                        if mid_word {
                            continue;
                        }
                        placed.push((a, b, g));
                        used = end;
                    }
                }
                for (a, b, g) in &placed {
                    let (sx0, sx1) = (self.origin.x + a * k, self.origin.x + b * k);
                    let anchor = match p.align {
                        PAlign::Center => (sx1 - sx0) / 2.0,
                        PAlign::Right => sx1 - sx0,
                        _ => 0.0,
                    };
                    let row_h = g.rows.first().map_or(0.0, |r| r.height());
                    let band = egui::Rect::from_min_max(
                        Pos2::new(sx0, top + y),
                        Pos2::new(sx1, top + y + row_h),
                    );
                    let painter = self
                        .painter
                        .with_clip_rect(band.intersect(base.clip_rect()));
                    let gp = Pos2::new(sx0 + anchor, top + y);
                    painter.galley(gp, g.clone(), Color32::BLACK);
                    paint_underlines(&painter, g, gp, None);
                }
                if let (true, Some(lg), Some((a, _, _))) = (first_line, &label_g, placed.first()) {
                    // The label hangs left of the first piece, level with its line.
                    let lx = self.origin.x + a * k - (geom.first_x - geom.label_x);
                    let dy = (line_h - lg.size().y).max(0.0);
                    base.galley(Pos2::new(lx, top + y + dy), lg.clone(), Color32::BLACK);
                }
                first_line = false;
                if used == 0 {
                    // Empty paragraph (or nothing fit): one empty line.
                    y += line_h;
                    break;
                }
                cur = skip_chars(&cur, used);
                y += line_h;
            }
            y += pt_to_px(f64::from(p.spacing.space_after_pt), self.zoom) as f32;
        }
    }

    /// The galleys of a paragraph with indents or a list (spec 4R), placed in a column of
    /// width `wrap` whose paragraph top is `y`, and the paragraph's height: the label at
    /// the first-line position, the first line, then the other lines laid out at the
    /// left indent (a hanging indent has the lines after the first start at the text).
    fn indented_galleys(
        &self,
        painter: &Painter,
        p: &Paragraph,
        label: Option<&str>,
        wrap: f32,
        y: f32,
    ) -> (Vec<(Pos2, Arc<egui::Galley>)>, f32) {
        let mut row_height = |f: &FontId| painter.fonts_mut(|fv| fv.row_height(f));
        let label_galley = label.map(|text| {
            let (text, font_id, col) = self.label_style(p, text);
            painter.layout_no_wrap(text, font_id, col)
        });
        let geom = indent_layout(
            p.indents,
            wrap,
            label_galley.as_ref().map(|g| g.size().x),
            self.zoom,
        );
        let mut parts = Vec::new();
        let anchor = |x: f32, w: f32| match p.align {
            PAlign::Center => x + w / 2.0,
            PAlign::Right => x + w,
            _ => x,
        };
        // The first line, laid out at its own width; a paragraph of more rows is cut after
        // its first row and the rest laid out again at the left indent.
        let full = painter.layout_job(paragraph_job(
            self.fonts,
            p,
            geom.first_w,
            self.zoom,
            &mut row_height,
        ));
        let (first, rest) = match full.rows.get(1) {
            None => (full, None),
            Some(_) => {
                let n = full.rows[0].char_count_including_newline().0;
                let mut head = take_chars(p, n);
                // The row's own line end is not part of the line.
                if let Some(r) = head.runs.last_mut() {
                    if r.text.ends_with('\n') {
                        r.text.pop();
                    }
                }
                let first = painter.layout_job(paragraph_job(
                    self.fonts,
                    &head,
                    geom.first_w,
                    self.zoom,
                    &mut row_height,
                ));
                let rest = painter.layout_job(paragraph_job(
                    self.fonts,
                    &skip_chars(p, n),
                    geom.rest_w,
                    self.zoom,
                    &mut row_height,
                ));
                (first, Some(rest))
            }
        };
        let first_h = first.size().y;
        if let Some(g) = label_galley {
            // Bottom-aligned with the first line (a bigger or smaller bullet shares its
            // baseline closely enough).
            let dy = (first_h - g.size().y).max(0.0);
            parts.push((Pos2::new(geom.label_x, y + dy), g));
        }
        parts.push((Pos2::new(anchor(geom.first_x, geom.first_w), y), first));
        let mut h = first_h;
        if let Some(rest) = rest {
            let rest_h = rest.size().y;
            parts.push((Pos2::new(anchor(geom.rest_x, geom.rest_w), y + first_h), rest));
            h += rest_h;
        }
        (parts, h)
    }

    /// The text, font and colour a list label of `p` is drawn in: a bullet in its own font
    /// and size (a symbol-encoded font's code becomes Unicode, drawn in the text font), a
    /// number in the text style of the paragraph's last run.
    fn label_style(&self, p: &Paragraph, text: &str) -> (String, FontId, Color32) {
        let run = p.runs.last().cloned().unwrap_or_else(|| p.base_run());
        let size_px = |pt: f32| pt_to_px(f64::from(pt), self.zoom) as f32;
        match &p.list {
            Some(l) if l.kind.is_bullet() => {
                let font = if l.bullet_font.is_empty() || pub_format::is_symbol_font(&l.bullet_font)
                {
                    run.font.as_str()
                } else {
                    l.bullet_font.as_str()
                };
                let ch = pub_format::bullet_display_char(l.bullet_char, &l.bullet_font);
                let pt = l.bullet_size_pt.filter(|s| *s > 0.0).unwrap_or(run.size_pt);
                (
                    ch.to_string(),
                    self.fonts.font_id(font, false, false, size_px(pt)),
                    color(run.color),
                )
            }
            _ => (
                text.to_owned(),
                self.fonts
                    .font_id(&run.font, run.bold, run.italic, size_px(run.size_pt)),
                color(run.color),
            ),
        }
    }

    /// [`Self::draw_text`] with an explicit text inset in screen pixels.
    fn draw_text_inset(
        &self,
        _ctx: &Context,
        rect: egui::Rect,
        paragraphs: &[Paragraph],
        turn: Option<Turn>,
        cols: Columns,
        inset: [f32; 4],
    ) {
        let n = if turn.is_some() { 1 } else { cols.count.max(1) };
        let inner_w = (rect.width() - inset[0] - inset[2]).max(1.0);
        let wrap = ((inner_w - (n - 1) as f32 * cols.gap) / n as f32).max(1.0);
        let height = (rect.height() - inset[1] - inset[3]).max(0.0);
        let base = self
            .painter
            .with_clip_rect(rect.intersect(self.painter.clip_rect()));
        let clipped = if turn.is_some() { self.painter } else { &base };

        // Lay the story out as one strip of width `wrap`; `rows` holds the strip y of
        // every row bottom so a column can end on a whole line.
        let mut placed: Vec<(Pos2, Arc<egui::Galley>)> = Vec::new();
        let mut y = 0.0f32;
        let labels = kernpage_core::text::list_labels(paragraphs);
        for (pi, p) in paragraphs.iter().enumerate() {
            // Spec 3.6: space before + lines + space after; line height is a multiple of
            // the font's row height or an exact size.
            y += pt_to_px(f64::from(p.spacing.space_before_pt), self.zoom) as f32;
            let mut row_height = |f: &FontId| clipped.fonts_mut(|fv| fv.row_height(f));
            let h = if is_indented(p) {
                let (parts, h) = self.indented_galleys(clipped, p, labels[pi].as_deref(), wrap, y);
                placed.extend(parts);
                h
            } else {
                let galley = clipped.layout_job(paragraph_job(
                    self.fonts,
                    p,
                    wrap,
                    self.zoom,
                    &mut row_height,
                ));
                // The alignment anchor of a galley is the point it is placed at.
                let x = match p.align {
                    PAlign::Center => wrap / 2.0,
                    PAlign::Right => wrap,
                    _ => 0.0,
                };
                let h = galley.size().y;
                placed.push((Pos2::new(x, y), galley));
                h
            };
            y += h + pt_to_px(f64::from(p.spacing.space_after_pt), self.zoom) as f32;
            if n == 1 && y > height {
                break;
            }
            if n > 1 && y > height * n as f32 {
                break;
            }
        }
        let mut row_bottoms: Vec<f32> = Vec::new();
        for (pos, g) in &placed {
            row_bottoms.extend(g.rows.iter().map(|r| pos.y + r.max_y()));
        }

        let mut start = 0.0f32;
        for c in 0..n {
            // This column ends at the last row bottom that fits; it may not fit at all
            // for the last column, which just clips.
            let limit = start + height;
            let end = if c + 1 < n {
                row_bottoms
                    .iter()
                    .copied()
                    .filter(|&b| b > start && b <= limit + 0.5)
                    .fold(start, f32::max)
            } else {
                limit
            };
            let col_left = rect.min.x + inset[0] + c as f32 * (wrap + cols.gap);
            let col_top = rect.min.y + inset[1];
            let col_clip = if turn.is_some() {
                None
            } else {
                Some(egui::Rect::from_min_max(
                    Pos2::new(col_left, col_top),
                    Pos2::new(col_left + wrap, col_top + height),
                ))
            };
            for (pos, g) in &placed {
                let (top, bottom) = (pos.y, pos.y + g.size().y);
                if bottom < start || top > end {
                    continue;
                }
                let at = Pos2::new(col_left + pos.x, col_top + pos.y - start);
                match (turn, col_clip) {
                    (Some(t), _) => {
                        self.painter.add(
                            egui::epaint::TextShape::new(t.pt(at), g.clone(), Color32::BLACK)
                                .with_angle(t.angle),
                        );
                        paint_underlines(self.painter, g, at, Some(t));
                    }
                    (None, Some(clip)) => {
                        // Clip to the whole line band of this column.
                        let band = egui::Rect::from_min_max(
                            clip.min,
                            Pos2::new(clip.max.x, col_top + (end - start)),
                        );
                        let p = self
                            .painter
                            .with_clip_rect(band.intersect(base.clip_rect()));
                        p.galley(at, g.clone(), Color32::BLACK);
                        paint_underlines(&p, g, at, None);
                    }
                    (None, None) => {}
                }
            }
            start = end;
        }
    }
}

/// Whether a paragraph needs the indented layout: it is in a list or has any indent.
fn is_indented(p: &Paragraph) -> bool {
    p.list.is_some() || p.indents != kernpage_core::ParaIndents::default()
}

/// Where the parts of an indented paragraph go, in pixels from the column's left edge, and
/// how wide the first line and the other lines are (spec 4R; the same rule as the PDF,
/// [`pub_format::line_starts`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndentLayout {
    /// The list label.
    pub label_x: f32,
    /// The first line's text and the width it may fill.
    pub first_x: f32,
    pub first_w: f32,
    /// The lines after the first: they start at the left indent, not at the label.
    pub rest_x: f32,
    pub rest_w: f32,
}

/// The [`IndentLayout`] of a paragraph with `indents` in a column `wrap` pixels wide;
/// `label_w` is the width in pixels of its list label, if it has one.
pub fn indent_layout(
    indents: kernpage_core::ParaIndents,
    wrap: f32,
    label_w: Option<f32>,
    zoom: f64,
) -> IndentLayout {
    let k = pt_to_px(1.0, zoom) as f32;
    let s = pub_format::line_starts(indents, label_w.map(|w| w / k));
    let width = |x: f32| (wrap - (x + s.right) * k).max(1.0);
    IndentLayout {
        label_x: s.label_x * k,
        first_x: s.first_x * k,
        first_w: width(s.first_x),
        rest_x: s.rest_x * k,
        rest_w: width(s.rest_x),
    }
}

/// The first `n` characters of the paragraph (runs keep their formatting).
fn take_chars(p: &Paragraph, mut n: usize) -> Paragraph {
    let mut out = p.clone();
    out.runs.clear();
    for r in &p.runs {
        if n == 0 {
            break;
        }
        let len = r.text.chars().count();
        out.runs
            .push(r.with_text(&r.text.chars().take(n).collect::<String>()));
        n = n.saturating_sub(len);
    }
    if out.runs.is_empty() {
        out.runs.extend(p.runs.first().map(|r| r.with_text("")));
    }
    out
}

/// The paragraph without its first `n` characters (runs keep their formatting).
fn skip_chars(p: &Paragraph, mut n: usize) -> Paragraph {
    let mut out = p.clone();
    out.runs.clear();
    for r in &p.runs {
        let len = r.text.chars().count();
        if n >= len {
            n -= len;
            continue;
        }
        out.runs
            .push(r.with_text(&r.text.chars().skip(n).collect::<String>()));
        n = 0;
    }
    out
}

/// A rotation of a drawn shape about its centre, in screen space (angle clockwise, radians).
#[derive(Clone, Copy)]
struct Turn {
    centre: Pos2,
    angle: f32,
}

impl Turn {
    fn pt(&self, p: Pos2) -> Pos2 {
        self.centre + egui::emath::Rot2::from_angle(self.angle) * (p - self.centre)
    }

    /// Corners of `rect` (top-left first, clockwise), turned.
    fn corners(&self, rect: egui::Rect) -> Vec<Pos2> {
        [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .into_iter()
        .map(|p| self.pt(p))
        .collect()
    }
}

/// Text columns of a box: count and gap in pixels (spec 4I).
#[derive(Clone, Copy)]
struct Columns {
    count: usize,
    gap: f32,
}

impl Default for Columns {
    fn default() -> Self {
        Columns { count: 1, gap: 0.0 }
    }
}

/// Points around an ellipse inscribed in `rect`, turned.
fn ellipse_points(rect: egui::Rect, turn: Option<Turn>) -> Vec<Pos2> {
    const STEPS: usize = 64;
    let (c, r) = (rect.center(), rect.size() / 2.0);
    (0..STEPS)
        .map(|i| {
            let a = i as f32 / STEPS as f32 * std::f32::consts::TAU;
            let p = c + Vec2::new(r.x * a.cos(), r.y * a.sin());
            turn.map_or(p, |t| t.pt(p))
        })
        .collect()
}

/// Line height in pixels for a run whose font has natural row height `natural`.
fn line_height_px(sp: &ParaSpacing, natural: f32, zoom: f64) -> f32 {
    match sp.line {
        LineSpacing::Multiple(k) => k * natural,
        LineSpacing::Exactly(pt) => pt_to_px(f64::from(pt), zoom) as f32,
        LineSpacing::Unknown(_) => 1.19 * natural,
    }
}

/// Marks an underlined run in a layout job. A zero-width stroke is skipped by egui, which
/// would draw the line at the bottom of the row; [`underline_segments`] reads the mark and
/// draws the line near the baseline instead.
pub fn underline_mark(on: bool, col: Color32) -> Stroke {
    if on {
        Stroke::new(0.0, col)
    } else {
        Stroke::NONE
    }
}

/// One underline line segment in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnderlineSeg {
    pub from: Pos2,
    pub to: Pos2,
    pub width: f32,
    pub color: Color32,
}

/// Distance of an underline below the baseline, in em (the same as the PDF export).
pub const UNDERLINE_OFFSET_EM: f32 = 0.145;
/// Underline thickness, in em (the same as the PDF export).
pub const UNDERLINE_THICKNESS_EM: f32 = 0.066;

type OpenSeg = (usize, f32, f32, f32);

/// The underline segments of `galley` placed at `at`: one per underlined stretch of a row,
/// just below the baseline, scaled with the (script) font size of the run. Trailing spaces
/// of an underlined run are included.
pub fn underline_segments(galley: &egui::Galley, at: Pos2) -> Vec<UnderlineSeg> {
    let job = &galley.job;
    // Run of each char: the section whose byte range holds it.
    let sec_of: Vec<usize> = job
        .text
        .char_indices()
        .map(|(byte, _)| {
            job.sections
                .iter()
                .position(|sec| sec.byte_range.contains(&egui::text::ByteIndex(byte)))
                .unwrap_or(0)
        })
        .collect();
    let mut out = Vec::new();
    let mut char_ix = 0usize;
    for row in &galley.rows {
        let mut cur: Option<OpenSeg> = None; // section, x0, x1, baseline y
        let flush = |cur: &mut Option<OpenSeg>, out: &mut Vec<UnderlineSeg>| {
            if let Some((sec, x0, x1, by)) = cur.take() {
                let f = &job.sections[sec].format;
                let size = f.font_id.size;
                let y = at.y + row.pos.y + by + UNDERLINE_OFFSET_EM * size;
                out.push(UnderlineSeg {
                    from: Pos2::new(at.x + row.pos.x + x0, y),
                    to: Pos2::new(at.x + row.pos.x + x1, y),
                    width: (UNDERLINE_THICKNESS_EM * size).max(1.0),
                    color: f.underline.color,
                });
            }
        };
        for g in &row.glyphs {
            let sec = sec_of.get(char_ix).copied();
            char_ix += 1;
            let under = sec.filter(|&s| {
                let u = job.sections[s].format.underline;
                u != Stroke::NONE && u.width <= 0.0
            });
            match (under, &mut cur) {
                (Some(s), Some(c)) if c.0 == s && (c.3 - g.pos.y).abs() < 0.01 => {
                    c.2 = g.pos.x + g.advance_width;
                }
                (Some(s), _) => {
                    flush(&mut cur, &mut out);
                    cur = Some((s, g.pos.x, g.pos.x + g.advance_width, g.pos.y));
                }
                (None, _) => flush(&mut cur, &mut out),
            }
        }
        flush(&mut cur, &mut out);
        if row.ends_with_newline {
            char_ix += 1;
        }
    }
    out
}

/// Draw the underlines of `galley` placed at `at`; `turn` maps the points of a turned shape.
fn paint_underlines(painter: &Painter, galley: &egui::Galley, at: Pos2, turn: Option<Turn>) {
    for u in underline_segments(galley, at) {
        let map = |p: Pos2| turn.map_or(p, |t| t.pt(p));
        painter.line_segment([map(u.from), map(u.to)], Stroke::new(u.width, u.color));
    }
}

/// [`paint_underlines`] for the text editor's galley.
pub fn paint_edit_underlines(painter: &Painter, galley: &egui::Galley, at: Pos2) {
    paint_underlines(painter, galley, at, None);
}

/// The font a run is drawn in: superscript and subscript use 0.65 x the nominal size.
pub fn script_font(fonts: &FontMgr, r: &kernpage_core::Run, zoom: f64, nominal: FontId) -> FontId {
    if r.script == kernpage_core::Script::Normal {
        return nominal;
    }
    let px = pt_to_px(f64::from(r.size_pt * r.script.size_factor()), zoom) as f32;
    fonts.font_id(&r.font, r.bold, r.italic, px)
}

/// Superscript sits at the top of the line, subscript and normal text at the bottom.
pub fn script_valign(s: kernpage_core::Script) -> Align {
    match s {
        kernpage_core::Script::Superscript => Align::TOP,
        _ => Align::BOTTOM,
    }
}

/// One paragraph as an egui layout job at `zoom`; `wrap` is the width in pixels.
/// `row_height` gives a font's natural row height in pixels.
pub fn paragraph_job(
    fonts: &FontMgr,
    p: &Paragraph,
    wrap: f32,
    zoom: f64,
    row_height: &mut dyn FnMut(&FontId) -> f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    job.halign = match p.align {
        PAlign::Left | PAlign::Justify => Align::Min,
        PAlign::Center => Align::Center,
        PAlign::Right => Align::Max,
    };
    job.justify = p.align == PAlign::Justify;
    for r in &p.runs {
        let col = color(r.color);
        let font_id = fonts.font_id(
            &r.font,
            r.bold,
            r.italic,
            pt_to_px(f64::from(r.size_pt), zoom) as f32,
        );
        // The line is as tall as the nominal size; the glyphs are drawn smaller (spec 4S.3).
        let line_height = Some(line_height_px(&p.spacing, row_height(&font_id), zoom));
        let font_id = script_font(fonts, r, zoom, font_id);
        job.append(
            &r.text,
            0.0,
            TextFormat {
                font_id,
                line_height,
                valign: script_valign(r.script),
                color: col,
                italics: r.italic,
                underline: underline_mark(r.underline, col),
                ..Default::default()
            },
        );
    }
    if job.text.is_empty() {
        // An empty paragraph still takes one line of its own height.
        let r = p.base_run();
        let font_id = fonts.font_id(
            &r.font,
            r.bold,
            r.italic,
            pt_to_px(f64::from(r.size_pt), zoom) as f32,
        );
        let line_height = Some(line_height_px(&p.spacing, row_height(&font_id), zoom));
        job.append(
            " ",
            0.0,
            TextFormat {
                font_id,
                line_height,
                color: Color32::TRANSPARENT,
                ..Default::default()
            },
        );
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernpage_core::{ListInfo, ListType, ParaIndents};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    /// A list paragraph's label sits at left + first-line indent (the column's left edge
    /// by default), the first line's text and the wrapped lines start at the list indent.
    #[test]
    fn list_label_hangs_and_text_lines_up_at_the_list_indent() {
        let zoom = 1.0;
        let k = pt_to_px(1.0, zoom) as f32;
        let wrap = 300.0 * k;
        let l = indent_layout(ParaIndents::for_list(18.0), wrap, Some(6.0 * k), zoom);
        assert!(close(l.label_x, 0.0));
        assert!(close(l.first_x, 18.0 * k));
        assert!(close(l.rest_x, 18.0 * k));
        assert!(close(l.first_w, 282.0 * k));
        assert!(close(l.rest_w, 282.0 * k));
        // A bigger list indent moves the text, not the label.
        let l = indent_layout(ParaIndents::for_list(36.0), wrap, Some(6.0 * k), zoom);
        assert!(close(l.label_x, 0.0) && close(l.first_x, 36.0 * k) && close(l.rest_x, 36.0 * k));
        // A label wider than the hanging indent pushes the text to the next 36 pt stop.
        let l = indent_layout(ParaIndents::for_list(18.0), wrap, Some(20.0 * k), zoom);
        assert!(close(l.first_x, 36.0 * k) && close(l.rest_x, 18.0 * k), "{l:?}");
        // The right indent narrows both widths.
        let ind = ParaIndents {
            right_pt: 10.0,
            ..ParaIndents::for_list(18.0)
        };
        let l = indent_layout(ind, wrap, Some(6.0 * k), zoom);
        assert!(close(l.first_w, 272.0 * k) && close(l.rest_w, 272.0 * k));
    }

    /// Plain indents: the first line is offset by the first-line indent, the rest by left.
    #[test]
    fn plain_indents_offset_the_first_line_and_the_rest() {
        let k = pt_to_px(1.0, 1.0) as f32;
        let ind = ParaIndents {
            left_pt: 54.0,
            first_line_pt: -10.0,
            right_pt: 12.0,
        };
        let l = indent_layout(ind, 400.0 * k, None, 1.0);
        assert!(close(l.first_x, 44.0 * k) && close(l.rest_x, 54.0 * k));
        assert!(close(l.first_w, 344.0 * k) && close(l.rest_w, 334.0 * k));
    }

    #[test]
    fn only_lists_and_indents_take_the_indented_layout() {
        let mut p = Paragraph::plain("x");
        assert!(!is_indented(&p));
        p.list = Some(ListInfo::numbered(ListType::Arabic));
        assert!(is_indented(&p));
        p.list = None;
        p.indents.left_pt = 4.0;
        assert!(is_indented(&p));
    }

    #[test]
    fn take_and_skip_split_a_paragraph() {
        let mut p = Paragraph::plain("hello world");
        p.runs.push(kernpage_core::Run::plain("!!"));
        assert_eq!(take_chars(&p, 7).text(), "hello w");
        assert_eq!(skip_chars(&p, 7).text(), "orld!!");
        assert_eq!(take_chars(&p, 12).text(), "hello world!");
    }

    /// Lay out `job` with egui's bundled fonts and return the underline segments, the
    /// baseline of the first row and the bottom of the first row (galley placed at 0, 0).
    fn lay_out(job: egui::text::LayoutJob) -> (Vec<UnderlineSeg>, f32, f32) {
        let ctx = egui::Context::default();
        let mut got = None;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let g = ui.fonts_mut(|f| f.layout_job(job.clone()));
            let row = &g.rows[0];
            let baseline = row.pos.y + row.glyphs[0].pos.y;
            got = Some((underline_segments(&g, Pos2::ZERO), baseline, row.max_y()));
        });
        out.drop_without_applying_deltas();
        got.expect("laid out")
    }

    fn fmt(size: f32, line_height: Option<f32>, under: bool) -> egui::text::TextFormat {
        egui::text::TextFormat {
            font_id: FontId::proportional(size),
            line_height,
            color: Color32::BLACK,
            underline: underline_mark(under, Color32::BLACK),
            ..Default::default()
        }
    }

    /// The underline sits just below the baseline (0.145 em, measured from a Publisher 2021 PDF export), not at the bottom of a tall
    /// row (a row with extra line spacing); thickness follows the font size; the trailing
    /// space of the underlined run is covered, the plain tail is not.
    #[test]
    fn underline_is_just_below_the_baseline_not_at_the_row_bottom() {
        for size in [13.0f32, 26.0] {
            let mut job = egui::text::LayoutJob::default();
            job.append("under ", 0.0, fmt(size, Some(size * 2.5), true));
            job.append("plain", 0.0, fmt(size, Some(size * 2.5), false));
            let (segs, baseline, bottom) = lay_out(job);
            assert_eq!(segs.len(), 1, "{segs:?}");
            let u = segs[0];
            let d = u.from.y - baseline;
            assert!((d - UNDERLINE_OFFSET_EM * size).abs() < 0.01, "offset {d} for size {size}");
            assert!(u.from.y < bottom - size * 0.5, "not at the row bottom: {u:?} {bottom}");
            assert!((u.width - (UNDERLINE_THICKNESS_EM * size).max(1.0)).abs() < 0.01);
            assert_eq!(u.from.y, u.to.y);
            assert!(u.to.x > u.from.x);
        }
    }

    /// A superscript run is underlined at its own (smaller) size and baseline.
    #[test]
    fn underline_follows_a_smaller_script_run() {
        let mut job = egui::text::LayoutJob::default();
        job.append("big", 0.0, fmt(20.0, None, true));
        job.append("up", 0.0, egui::text::TextFormat { valign: Align::TOP, ..fmt(13.0, None, true) });
        let (segs, _, _) = lay_out(job);
        assert_eq!(segs.len(), 2, "{segs:?}");
        assert!(segs[1].from.y < segs[0].from.y, "the raised run is underlined higher");
        assert!(segs[1].width <= segs[0].width);
    }

    /// Text without an underline mark gets no segments; egui paints none for the mark.
    #[test]
    fn no_underline_no_segments() {
        let mut job = egui::text::LayoutJob::default();
        job.append("plain", 0.0, fmt(16.0, None, false));
        assert!(lay_out(job).0.is_empty());
    }

    /// A new text box carries Publisher's 0.04 in margin on each side, and the canvas insets
    /// the text by the box's own margins (here at 54 % zoom), not by a fixed constant.
    #[test]
    fn text_inset_comes_from_the_boxs_own_margins() {
        use kernpage_core::geometry::in_to_emu;
        let mut s = Shape::text_box(
            Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(1.0)),
            vec![Paragraph::plain("Hello")],
        );
        let one = ((0.04 * 96.0 + 0.72 / 72.0 * 96.0) * 0.54) as f32;
        for v in shape_inset_px(&s, 0.54) {
            assert!((v - one).abs() < 1e-3, "{v}");
        }
        s.inset_emu = [in_to_emu(0.1), in_to_emu(0.2), in_to_emu(0.3), in_to_emu(0.4)];
        let px = shape_inset_px(&s, 1.0);
        for (v, inches) in px.iter().zip([0.1, 0.2, 0.3, 0.4]) {
            assert!((v - (inches * 96.0 + 0.96)).abs() < 1e-3, "{px:?}");
        }
        assert!(INSET_PT > 0.0 && pt_to_px(INSET_PT, 1.0) > 0.0);
    }
}
