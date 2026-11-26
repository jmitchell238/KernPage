//! Conversion from the `.pub` reader's [`pub_format::Document`] to the editable model.

use crate::geometry::Rect;
use crate::model::*;
use pub_format as pf;
use std::sync::Arc;

fn rgb(c: pf::Color, scheme: Option<&pf::ColorScheme>) -> Rgb {
    let [r, g, b] = c.to_rgb(scheme);
    Rgb(r, g, b)
}

fn run(r: &pf::TextRun, sch: Option<&pf::ColorScheme>) -> Run {
    Run {
        text: r.text.replace(['\r', '\u{7}'], "").replace('\u{b}', "\n"),
        font: if r.font.is_empty() {
            DEFAULT_FONT.to_owned()
        } else {
            r.font.clone()
        },
        size_pt: if r.size_pt > 0.0 { r.size_pt } else { 10.0 },
        bold: r.bold,
        italic: r.italic,
        underline: r.underline,
        script: match r.script {
            pf::Script::Normal => crate::model::Script::Normal,
            pf::Script::Superscript => crate::model::Script::Superscript,
            pf::Script::Subscript => crate::model::Script::Subscript,
        },
        color: r.color.map_or(Rgb::BLACK, |c| rgb(c, sch)),
        field: r.field,
    }
}

pub(crate) fn paragraph(p: &pf::Paragraph, sch: Option<&pf::ColorScheme>) -> Paragraph {
    let mut runs: Vec<Run> = p.runs.iter().map(|r| run(r, sch)).collect();
    // Keep an empty run only when it is the sole carrier of the paragraph's formatting.
    if runs.iter().any(|r| !r.text.is_empty()) {
        runs.retain(|r| !r.text.is_empty());
    } else {
        runs.truncate(1);
    }
    if runs.is_empty() {
        runs.push(Run::plain(""));
    }
    Paragraph {
        runs,
        align: match p.alignment {
            pf::Alignment::Left => Align::Left,
            pf::Alignment::Center => Align::Center,
            pf::Alignment::Right => Align::Right,
            pf::Alignment::Justify => Align::Justify,
        },
        spacing: p.spacing,
        list: p.list.clone(),
        indents: p.indents,
    }
}

use crate::story::counts_as_glyph;

/// The part of `p` between glyph offsets `from` (inclusive) and `to` (exclusive).
fn slice_paragraph(p: &pf::Paragraph, from: usize, to: usize) -> pf::Paragraph {
    let mut index = 0;
    let mut runs = Vec::new();
    for r in &p.runs {
        let text: String = r
            .text
            .chars()
            .filter(|&ch| {
                let keep = index >= from && index < to;
                if counts_as_glyph(ch) {
                    index += 1;
                }
                keep
            })
            .collect();
        if !text.is_empty() {
            runs.push(pf::TextRun { text, ..r.clone() });
        }
    }
    if runs.is_empty() {
        runs.extend(p.runs.first().map(|r| pf::TextRun {
            text: String::new(),
            ..r.clone()
        }));
    }
    pf::Paragraph {
        runs,
        alignment: p.alignment,
        spacing: p.spacing,
        list: p.list.clone(),
        indents: p.indents,
        continued: false,
    }
}

/// The paragraphs of a story from position `start` up to `end` (exclusive).
pub(crate) fn story_slice(paras: &[pf::Paragraph], start: StoryPos, end: StoryPos) -> Vec<pf::Paragraph> {
    let last = if end.1 > 0 { end.0 + 1 } else { end.0 };
    (start.0..last.min(paras.len()))
        .map(|k| {
            let from = if k == start.0 { start.1 } else { 0 };
            let to = if k == end.0 { end.1 } else { usize::MAX };
            slice_paragraph(&paras[k], from, to)
        })
        .collect()
}

fn shape(s: &pf::Shape, page_w: i64, page_h: i64, sch: Option<&pf::ColorScheme>) -> Shape {
    let (l, r) = (s.left_emu.min(s.right_emu), s.left_emu.max(s.right_emu));
    let (t, b) = (s.top_emu.min(s.bottom_emu), s.top_emu.max(s.bottom_emu));
    let rect = Rect::new(
        page_w / 2 + i64::from(l),
        page_h / 2 + i64::from(t),
        i64::from(r) - i64::from(l),
        i64::from(b) - i64::from(t),
    );
    shape_with(s, &s.paragraphs, rect, sch)
}

fn shape_with(
    s: &pf::Shape,
    source: &[pf::Paragraph],
    rect: Rect,
    sch: Option<&pf::ColorScheme>,
) -> Shape {
    let paragraphs: Vec<Paragraph> = source.iter().map(|p| paragraph(p, sch)).collect();
    let mut fill = s.fill.map(|c| rgb(c, sch));
    let mut line = s.line.map(|(c, w)| Stroke {
        color: rgb(c, sch),
        width_emu: i64::from(w),
    });

    let kind = if let Some(pic) = &s.picture {
        // A picture frame draws its picture only.
        fill = None;
        line = None;
        ShapeKind::Picture {
            data: Arc::new(pic.bytes.clone()),
            format: match pic.format {
                pf::PictureFormat::Png => PictureFormat::Png,
                pf::PictureFormat::Jpeg => PictureFormat::Jpeg,
                pf::PictureFormat::Wmf => PictureFormat::Wmf,
                pf::PictureFormat::Emf => PictureFormat::Emf,
                pf::PictureFormat::Other(_) => PictureFormat::Unknown,
            },
            file: None,
        }
    } else if let Some(t) = &s.table {
        let wanted = (t.rows as usize)
            .saturating_mul(t.columns as usize)
            .min(1 << 16);
        let mut cells: Vec<TableCell> = (0..wanted)
            .map(|i| {
                let source = t.cell_paragraphs.get(i);
                let mut cell = TableCell::plain("");
                if let Some(src) = source.filter(|p| !p.is_empty()) {
                    cell.paragraphs = src.iter().map(|p| paragraph(p, sch)).collect();
                }
                if let Some(p) = t.cell_props.get(i) {
                    cell.margins_emu = [
                        i64::from(p.margin_left_emu),
                        i64::from(p.margin_top_emu),
                        i64::from(p.margin_right_emu),
                        i64::from(p.margin_bottom_emu),
                    ];
                    cell.merged = p.first_row != p.last_row || p.first_column != p.last_column;
                }
                cell
            })
            .collect();
        // Without per-cell text (no cell data at all) keep the story paragraphs, one per cell.
        if t.cell_paragraphs.is_empty() {
            for (cell, p) in cells.iter_mut().zip(&paragraphs) {
                cell.paragraphs = vec![p.clone()];
            }
        }
        ShapeKind::Table {
            rows: t.rows,
            cols: t.columns,
            col_widths_emu: t.column_widths_emu.iter().map(|&v| i64::from(v)).collect(),
            row_heights_emu: t.row_heights_emu.iter().map(|&v| i64::from(v)).collect(),
            row_set_emu: Vec::new(),
            cells,
        }
    } else {
        match s.geometry {
            Some(pf::ShapeGeometry::Line) => ShapeKind::Line,
            // UNCONFIRMED: text inside an ellipse is dropped.
            Some(pf::ShapeGeometry::Ellipse) => ShapeKind::Ellipse,
            _ if !paragraphs.is_empty() => ShapeKind::TextBox { paragraphs },
            _ if fill.is_some() || line.is_some() => ShapeKind::Rectangle,
            _ => ShapeKind::TextBox {
                paragraphs: vec![Paragraph::plain("")],
            },
        }
    };
    Shape {
        id: 0,
        rect,
        kind,
        fill,
        line,
        rotation_deg: s.rotation_deg,
        flip_h: s.flip_h,
        flip_v: s.flip_v,
        columns: s.columns.max(1),
        column_gap_emu: i64::from(s.column_gap_emu),
        wrap: s.wrap,
        wrap_dist_emu: s.wrap_dist_emu.map(i64::from),
        inset_emu: s.inset_emu.map(i64::from),
        group: None,
    }
}

/// Build an editable document from a parsed `.pub` file.
///
/// Linked text boxes (spec 4G) get no split here: the first box of a chain holds the whole
/// story and the later boxes are empty. Use [`from_pub_flowed`] to split the story across
/// the frames.
pub fn from_pub(name: &str, doc: &pf::Document) -> Document {
    from_pub_flowed(name, doc, &[])
}

/// Like [`from_pub`], but gives every frame of each linked chain only its share of the
/// story. `breaks` is `pub2pdf::chain_breaks(doc, ..)`: one entry per chain of two or more
/// boxes (in `Document::chains` order) listing the story position where each frame ends.
/// Chains without an entry fall back to "whole story in the first box".
///
/// Each chain also becomes a [`Story`] of `Document::stories` holding the whole story (with
/// the overflow after the last frame); the frames' paragraphs are the cached shares. Editing
/// a frame does not yet re-flow text into the next one.
pub fn from_pub_flowed(name: &str, doc: &pf::Document, breaks: &[Vec<StoryPos>]) -> Document {
    let (w, h) = (
        i64::from(doc.page_width_emu),
        i64::from(doc.page_height_emu),
    );
    let margins = doc.margins_emu.map_or(Margins::all(DEFAULT_MARGIN_EMU), |m| {
        Margins::from_array(m.map(i64::from))
    });
    let sch = doc.color_scheme.as_ref();
    let mut pages = doc
        .pages
        .iter()
        .enumerate()
        // Opened pages are numbered by position (two opens of one file agree); pages made
        // in the editor take ids from a counter that starts far above.
        .map(|(i, p)| Page {
            id: i as PageId + 1,
            width_emu: w,
            height_emu: h,
            margins_emu: margins,
            shapes: p.shapes.iter().map(|s| shape(s, w, h, sch)).collect(),
            duplicate_of: None,
            master: p.master,
            ignore_master: p.ignore_master,
        })
        .collect::<Vec<_>>();
    let mut flowed = breaks.iter();
    // Every linked chain: its frames as (page, shape), the whole story and the frame ends.
    type Found = (Vec<(usize, usize)>, Vec<Paragraph>, Vec<StoryPos>);
    let mut stories: Vec<Found> = Vec::new();
    for chain in doc.chains().into_iter().filter(|c| c.len() > 1) {
        let shape_of = |r: &pf::ShapeRef| &doc.pages[r.page].shapes[r.shape];
        let Some(story) = chain
            .iter()
            .map(shape_of)
            .find(|s| !s.paragraphs.is_empty())
            .map(|s| &s.paragraphs)
        else {
            continue;
        };
        let ends = flowed.next();
        // Without break info the whole story sits in the first frame.
        let whole = (story.len(), 0);
        stories.push((
            chain.iter().map(|r| (r.page, r.shape)).collect(),
            story.iter().map(|p| paragraph(p, sch)).collect(),
            (0..chain.len())
                .map(|i| ends.and_then(|e| e.get(i)).copied().unwrap_or(whole))
                .collect(),
        ));
        let mut start = (0, 0);
        for (i, r) in chain.iter().enumerate() {
            let src = shape_of(r);
            let share = match ends {
                Some(ends) => {
                    let end = ends.get(i).copied().unwrap_or((story.len(), 0));
                    let share = story_slice(story, start, end);
                    start = end;
                    share
                }
                // No split known: everything in the first box.
                None if i == 0 => story.clone(),
                None => Vec::new(),
            };
            let rect = pages[r.page].shapes[r.shape].rect;
            let mut linked = shape_with(src, &share, rect, sch);
            if share.is_empty() {
                linked.kind = ShapeKind::TextBox {
                    paragraphs: vec![Paragraph::plain("")],
                };
            }
            pages[r.page].shapes[r.shape] = linked;
        }
    }
    let pages = if pages.is_empty() {
        vec![Page::blank(w, h)]
    } else {
        pages
    };
    let mut out = Document::new(name, pages);
    for (n, (frames, paragraphs, ends)) in stories.into_iter().enumerate() {
        out.stories.push(Story {
            id: n as StoryId + 1,
            frames: frames.iter().map(|&(p, s)| out.pages[p].shapes[s].id).collect(),
            paragraphs,
            ends,
        });
    }
    if let Some(c) = sch {
        let [r, g, b] = c.colors[0];
        out.default_line = Rgb(r, g, b);
    }
    link_groups(&mut out, doc);
    out.sections = doc.sections.clone();
    // Master shapes (spec 4T): converted like page shapes, with ids of their own.
    for m in &doc.masters {
        let shapes = m
            .shapes
            .iter()
            .map(|s| {
                let mut shape = shape(s, w, h, sch);
                shape.id = out.alloc_id();
                shape
            })
            .collect();
        out.masters.push(MasterPage {
            name: m.name.clone(),
            shapes,
        });
    }
    out
}

/// Turn the reader's group shapes (kind 48, spec 4J) into [`ShapeKind::Group`] and point
/// their children at them. Both lists have the same page and shape order.
fn link_groups(out: &mut Document, src: &pf::Document) {
    for (mp, sp) in out.pages.iter_mut().zip(&src.pages) {
        let id_of: std::collections::HashMap<u32, ShapeId> = sp
            .shapes
            .iter()
            .zip(&mp.shapes)
            .map(|(s, m)| (s.object_number, m.id))
            .collect();
        for (s, m) in sp.shapes.iter().zip(mp.shapes.iter_mut()) {
            m.group = s.group.and_then(|g| id_of.get(&g).copied());
            if sp.shapes.iter().any(|c| c.group == Some(s.object_number)) {
                m.kind = ShapeKind::Group;
                m.fill = None;
                m.line = None;
            }
        }
    }
}
