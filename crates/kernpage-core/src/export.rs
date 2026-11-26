//! PDF export of the editor model.
//!
//! The editor model is converted to a [`pub_format::Document`] (the structure `pub2pdf`
//! draws from) and handed to `pub2pdf::render`, so the exported PDF has exactly the same
//! quality as `pub2pdf` output for a `.pub` file: embedded fonts, vector clip art, wrap,
//! columns, spacing, rotation and tables. This is the inverse of [`crate::convert`].

use crate::model::*;
use pub_format as pf;

fn color(c: Rgb) -> pf::Color {
    pf::Color::Rgb(c.0, c.1, c.2)
}

fn emu32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn ueemu(v: i64) -> u32 {
    v.clamp(0, i64::from(u32::MAX)) as u32
}

fn run(r: &Run, text: &str) -> pf::TextRun {
    pf::TextRun {
        text: text.to_owned(),
        bold: r.bold,
        italic: r.italic,
        size_pt: r.size_pt,
        font: r.font.clone(),
        underline: r.underline,
        script: match r.script {
            crate::model::Script::Normal => pf::Script::Normal,
            crate::model::Script::Superscript => pf::Script::Superscript,
            crate::model::Script::Subscript => pf::Script::Subscript,
        },
        color: Some(color(r.color)),
        field: r.field,
    }
}

/// One model paragraph becomes one or more `.pub` paragraphs: a line break (`\n`) inside a
/// paragraph starts a new paragraph without spacing between the pieces, because the PDF
/// layout draws paragraphs on separate lines but has no soft line break.
fn paragraphs(p: &Paragraph) -> Vec<pf::Paragraph> {
    let alignment = match p.align {
        Align::Left => pf::Alignment::Left,
        Align::Center => pf::Alignment::Center,
        Align::Right => pf::Alignment::Right,
        Align::Justify => pf::Alignment::Justify,
    };
    // Split the runs at every '\n'.
    let mut pieces: Vec<Vec<pf::TextRun>> = vec![Vec::new()];
    for r in &p.runs {
        for (i, part) in r.text.split('\n').enumerate() {
            if i > 0 {
                pieces.push(Vec::new());
            }
            pieces.last_mut().unwrap().push(run(r, part));
        }
    }
    let n = pieces.len();
    pieces
        .into_iter()
        .enumerate()
        .map(|(i, runs)| {
            let mut spacing = p.spacing;
            if i > 0 {
                spacing.space_before_pt = 0.0;
            }
            if i + 1 < n {
                spacing.space_after_pt = 0.0;
            }
            // A line break in a list paragraph starts a piece without a label whose text
            // lines up with the list text (the hanging indent has no first-line offset).
            let indents = if i == 0 {
                p.indents
            } else {
                pf::ParaIndents {
                    first_line_pt: 0.0,
                    ..p.indents
                }
            };
            pf::Paragraph {
                runs,
                alignment,
                spacing,
                list: p.list.clone(),
                indents,
                continued: i > 0 && p.list.is_some(),
            }
        })
        .collect()
}

/// Model paragraphs as reader paragraphs one for one, line breaks left in the text: the
/// layout counts glyphs without them, so story positions stay valid for the model.
pub(crate) fn flow_paragraphs(ps: &[Paragraph]) -> Vec<pf::Paragraph> {
    ps.iter()
        .map(|p| pf::Paragraph {
            runs: p.runs.iter().map(|r| run(r, &r.text)).collect(),
            alignment: match p.align {
                Align::Left => pf::Alignment::Left,
                Align::Center => pf::Alignment::Center,
                Align::Right => pf::Alignment::Right,
                Align::Justify => pf::Alignment::Justify,
            },
            spacing: p.spacing,
            list: p.list.clone(),
            indents: p.indents,
            continued: false,
        })
        .collect()
}

fn all_paragraphs(ps: &[Paragraph]) -> Vec<pf::Paragraph> {
    ps.iter().flat_map(paragraphs).collect()
}

/// With `layout_only` the shape keeps what the text layout reads (frame, wrap, columns,
/// whether it has text) but not the text itself or picture bytes.
fn shape(s: &Shape, page_w: i64, page_h: i64, layout_only: bool) -> pf::Shape {
    let mut out = pf::Shape {
        object_number: s.id as u32,
        kind: 0,
        left_emu: emu32(s.rect.x - page_w / 2),
        top_emu: emu32(s.rect.y - page_h / 2),
        right_emu: emu32(s.rect.x + s.rect.w - page_w / 2),
        bottom_emu: emu32(s.rect.y + s.rect.h - page_h / 2),
        width_emu: None,
        height_emu: None,
        rotation_deg: s.rotation_deg.rem_euclid(360.0),
        group: None,
        wrap: match s.wrap {
            WrapType::None => pf::WrapType::None,
            WrapType::Square => pf::WrapType::Square,
            WrapType::Tight => pf::WrapType::Tight,
            WrapType::TopAndBottom => pf::WrapType::TopAndBottom,
            WrapType::Through => pf::WrapType::Through,
            WrapType::Other(n) => pf::WrapType::Other(n),
        },
        wrap_dist_emu: s.wrap_dist_emu.map(ueemu),
        inset_emu: s.inset_emu.map(ueemu),
        columns: s.columns.max(1),
        column_gap_emu: ueemu(s.column_gap_emu),
        story: None,
        chain_story: None,
        chain_pos: 0,
        text: None,
        runs: Vec::new(),
        paragraphs: Vec::new(),
        picture: None,
        geometry: None,
        flip_h: s.flip_h,
        flip_v: s.flip_v,
        fill: s.fill.map(color),
        line: s.line.map(|l| (color(l.color), ueemu(l.width_emu))),
        table: None,
    };
    match &s.kind {
        ShapeKind::TextBox { paragraphs } => {
            out.paragraphs = if layout_only {
                // Only "has text" matters (a text box is never a wrap obstacle).
                paragraphs
                    .first()
                    .map(|_| pf::Paragraph {
                        runs: Vec::new(),
                        alignment: pf::Alignment::Left,
                        spacing: pf::ParaSpacing::default(),
                        list: None,
                        indents: Default::default(),
                        continued: false,
                    })
                    .into_iter()
                    .collect()
            } else {
                all_paragraphs(paragraphs)
            };
            if s.fill.is_some() || s.line.is_some() {
                out.geometry = Some(pf::ShapeGeometry::Rectangle);
            }
        }
        ShapeKind::Rectangle => out.geometry = Some(pf::ShapeGeometry::Rectangle),
        ShapeKind::Ellipse => out.geometry = Some(pf::ShapeGeometry::Ellipse),
        ShapeKind::Line => out.geometry = Some(pf::ShapeGeometry::Line),
        ShapeKind::Group => {}
        ShapeKind::Picture { data, format, .. } => {
            out.fill = None;
            out.line = None;
            out.picture = Some(pf::Picture {
                format: match format {
                    PictureFormat::Png => pf::PictureFormat::Png,
                    PictureFormat::Jpeg => pf::PictureFormat::Jpeg,
                    PictureFormat::Wmf => pf::PictureFormat::Wmf,
                    PictureFormat::Emf => pf::PictureFormat::Emf,
                    PictureFormat::Unknown => pf::PictureFormat::Other(0),
                },
                bytes: if layout_only {
                    Vec::new()
                } else {
                    data.as_ref().clone()
                },
            });
        }
        ShapeKind::Table {
            rows,
            cols,
            col_widths_emu,
            row_heights_emu,
            cells,
            ..
        } => {
            let (rows, cols) = ((*rows).max(1), (*cols).max(1));
            let n = (rows as usize * cols as usize).min(1 << 16);
            let cell = |i: usize| cells.get(i);
            out.table = Some(pf::Table {
                rows,
                columns: cols,
                cells: (0..n)
                    .map(|i| {
                        cell(i)
                            .map(|c| {
                                c.paragraphs
                                    .iter()
                                    .map(Paragraph::text)
                                    .collect::<Vec<_>>()
                                    .join("\r")
                            })
                            .unwrap_or_default()
                    })
                    .collect(),
                column_widths_emu: col_widths_emu.iter().map(|&v| ueemu(v)).collect(),
                row_heights_emu: row_heights_emu.iter().map(|&v| ueemu(v)).collect(),
                cell_paragraphs: (0..n)
                    .map(|i| {
                        cell(i)
                            .map(|c| all_paragraphs(&c.paragraphs))
                            .unwrap_or_default()
                    })
                    .collect(),
                cell_props: (0..n)
                    .map(|i| {
                        let (r, c) = ((i / cols as usize) as u32, (i % cols as usize) as u32);
                        let m =
                            cell(i).map_or([TableCell::DEFAULT_MARGIN_EMU; 4], |c| c.margins_emu);
                        pf::CellProps {
                            first_row: r,
                            last_row: r,
                            first_column: c,
                            last_column: c,
                            margin_left_emu: ueemu(m[0]),
                            margin_top_emu: ueemu(m[1]),
                            margin_right_emu: ueemu(m[2]),
                            margin_bottom_emu: ueemu(m[3]),
                        }
                    })
                    .collect(),
            });
        }
    }
    out
}

/// The editor document as a `.pub` reader document. The reader format has one page size
/// for the whole publication, so the first page's size is used for every page.
pub fn to_pub(doc: &Document) -> pf::Document {
    pub_document(doc, |_| true, false)
}

/// [`to_pub`] with only the shapes of the pages `keep` accepts (the others are empty pages),
/// and with `layout_only` shapes (no text, no picture bytes).
fn pub_document(doc: &Document, keep: impl Fn(usize) -> bool, layout_only: bool) -> pf::Document {
    let (w, h) = doc
        .pages
        .first()
        .map_or((7_772_400, 10_058_400), |p| (p.width_emu, p.height_emu));
    pf::Document {
        pages: doc
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| pf::Page {
                object_number: i as u32 + 1,
                master: p.master.filter(|&m| m < doc.masters.len()),
                ignore_master: p.ignore_master,
                shapes: if keep(i) {
                    p.shapes
                        .iter()
                        .filter(|s| !matches!(s.kind, ShapeKind::Group))
                        .map(|s| shape(s, w, h, layout_only))
                        .collect()
                } else {
                    Vec::new()
                },
            })
            .collect(),
        text: String::new(),
        stories: Vec::new(),
        story_ids: Vec::new(),
        fonts: if layout_only {
            Vec::new()
        } else {
            doc.fonts_used().into_iter().map(|f| f.0).collect()
        },
        page_width_emu: ueemu(w),
        page_height_emu: ueemu(h),
        margins_emu: None,
        color_scheme: None,
        columns_mapped: 0,
        columns_unmapped: 0,
        shapes_without_anchor: 0,
        masters: doc
            .masters
            .iter()
            .enumerate()
            .map(|(i, m)| pf::MasterPage {
                object_number: 100_000 + i as u32,
                name: m.name.clone(),
                letter: String::new(),
                shapes: if !layout_only {
                    m.shapes
                        .iter()
                        .filter(|s| !matches!(s.kind, ShapeKind::Group))
                        .map(|s| shape(s, w, h, layout_only))
                        .collect()
                } else {
                    Vec::new()
                },
            })
            .collect(),
        sections: doc.sections.clone(),
    }
}

/// The layout geometry (frame, columns, wrap obstacles: the numbers the PDF export uses)
/// of the text boxes at `frames` = (page, index in that page's shape list), in order.
/// `None` if one of them does not exist.
pub(crate) fn flow_geometry(
    doc: &Document,
    frames: &[(usize, usize)],
) -> Option<Vec<pub2pdf::FrameGeom>> {
    let pf_doc = pub_document(doc, |p| frames.iter().any(|f| f.0 == p), true);
    frames
        .iter()
        .map(|&(page, ix)| {
            // Groups are not exported, so the index shifts by the groups before the shape.
            let before = doc
                .pages
                .get(page)?
                .shapes
                .get(..ix)?
                .iter()
                .filter(|s| !matches!(s.kind, ShapeKind::Group))
                .count();
            pub2pdf::FrameGeom::of_shape(&pf_doc, page, before)
        })
        .collect()
}

/// Render the document (including unsaved edits) to PDF bytes. Text box outlines are not
/// drawn.
pub fn export_pdf(doc: &Document, font_dirs: Vec<std::path::PathBuf>) -> Vec<u8> {
    pub2pdf::render(
        &to_pub(doc),
        &pub2pdf::Options {
            outlines: false,
            font_dirs,
        },
    )
}
