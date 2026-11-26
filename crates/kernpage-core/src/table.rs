//! Table rules shared by the commands and the save planner: Publisher 2021's default
//! table, track (row and column) sizes, and inserting and deleting rows and columns
//! (docs/ui/TABLES.md sections 1, 4 and 5).

use crate::command::CommandError;
use crate::geometry::Rect;
use crate::model::*;

/// Rows or columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Rows,
    Columns,
}

impl Axis {
    pub fn name(self) -> &'static str {
        match self {
            Axis::Rows => "row",
            Axis::Columns => "column",
        }
    }
}

/// Width of one column of a table inserted from the Insert > Table grid: 72 pt (1 inch).
pub const DEFAULT_COLUMN_EMU: i64 = 914_400;
/// Height of one row of such a table: 27.10543 pt (TABLES.md section 1.1 measured 27.105 pt
/// rows, spec 4E.1.1 gives 27.10543 pt); 344,238 EMU.
pub const DEFAULT_ROW_EMU: i64 = 344_238;
/// Publisher puts a new table's left edge this far left of exact centring (measured: a 3
/// column table on a 612 pt page starts at 196.875 pt, exact centring is 198 pt; 6 columns
/// 88.875 vs 90 pt): 1.125 pt.
const LEFT_BIAS_EMU: i64 = 14_288;
/// Publisher's top edge of a new table on a 792 pt page: 369.79 pt, i.e. 26.21 pt above the
/// page centre, whatever the table's height. 26.21 pt = 332,867 EMU.
const TOP_ABOVE_CENTRE_EMU: i64 = 332_867;

/// The frame Publisher gives a table of `rows` x `columns` picked in the Insert > Table
/// grid (or the Create Table dialog) on a page of the given size, in EMU from the page's
/// top left corner. Derived from TABLES.md section 1.1 (one place, so it can be corrected):
/// width = columns x 72 pt, height = rows x 27.10543 pt; horizontally centred, then 1.125 pt
/// left (both measured cases agree); top = page centre - 26.21 pt, independent of the
/// height (measured for 2 and 4 rows on a 792 pt page; how it scales with page height is
/// not known, so the same offset from the centre is used on every page).
pub fn default_table_rect(page_w: i64, page_h: i64, rows: u32, columns: u32) -> Rect {
    let w = DEFAULT_COLUMN_EMU * i64::from(columns);
    let h = DEFAULT_ROW_EMU * i64::from(rows);
    Rect::new(
        (page_w - w) / 2 - LEFT_BIAS_EMU,
        page_h / 2 - TOP_ABOVE_CENTRE_EMU,
        w,
        h,
    )
}

/// An empty cell as Publisher makes it: one empty Calibri 10 paragraph, default margins.
pub fn empty_cell() -> TableCell {
    let mut cell = TableCell::plain("");
    for p in &mut cell.paragraphs {
        for r in &mut p.runs {
            r.size_pt = FILE_DEFAULT_SIZE_PT;
        }
    }
    cell
}

/// An empty cell that sits next to `src`: its text is empty and it types in the formatting of
/// the first run of `src`'s first paragraph (with that paragraph's alignment and spacing).
/// Publisher's rule for new cells is not in the spec; this is the neighbour the new track's
/// size is copied from.
pub fn empty_cell_like(src: &TableCell) -> TableCell {
    let Some(p) = src.paragraphs.first() else {
        return empty_cell();
    };
    let mut run = p.base_run();
    run.text.clear();
    TableCell {
        paragraphs: vec![Paragraph {
            runs: vec![run],
            align: p.align,
            spacing: p.spacing,
            list: None,
            indents: ParaIndents::default(),
        }],
        margins_emu: src.margins_emu,
        merged: false,
    }
}

/// `n` track sizes: the stored ones when they match, else equal tracks filling `total`
/// (rounded so that they add up to `total`, as `Shapes.AddTable` splits).
pub fn tracks_of(sizes: &[i64], n: usize, total: i64) -> Vec<i64> {
    if sizes.len() == n {
        return sizes.to_vec();
    }
    even_tracks(total, n)
}

/// `n` equal tracks across `total` EMU, rounded to whole EMU, summing to `total`.
pub fn even_tracks(total: i64, n: usize) -> Vec<i64> {
    let n64 = n.max(1) as i64;
    let mut prev = 0;
    (1..=n64)
        .map(|i| {
            let edge = (total * i + n64 / 2) / n64;
            let size = edge - prev;
            prev = edge;
            size
        })
        .collect()
}

/// A new table of `rows` x `columns` filling `rect` evenly, empty cells. Refuses sizes
/// outside what Publisher accepts (at least 1 x 1, at most 65,535 cells, columns of at least
/// 16.94 pt and rows of at least one 10 pt line).
pub fn new_table(rect: Rect, rows: u32, columns: u32) -> Result<ShapeKind, CommandError> {
    let cells = u64::from(rows) * u64::from(columns);
    if rows == 0 || columns == 0 || cells > u64::from(pub_write::MAX_CELLS) {
        return Err(CommandError::BadTableSize);
    }
    let widths = even_tracks(rect.w, columns as usize);
    let heights = even_tracks(rect.h, rows as usize);
    if widths.iter().any(|&w| w < i64::from(pub_write::MIN_COLUMN_EMU))
        || heights.iter().any(|&h| h < i64::from(pub_write::MIN_ROW_EMU))
    {
        return Err(CommandError::TableTooSmall);
    }
    Ok(ShapeKind::Table {
        rows,
        cols: columns,
        col_widths_emu: widths,
        row_heights_emu: heights,
        row_set_emu: Vec::new(),
        cells: (0..cells).map(|_| empty_cell()).collect(),
    })
}

/// The grid of a table shape, refusing other shapes and tables with merged cells.
struct Grid<'a> {
    rect: &'a mut Rect,
    rows: &'a mut u32,
    cols: &'a mut u32,
    widths: &'a mut Vec<i64>,
    heights: &'a mut Vec<i64>,
    set: &'a mut Vec<i64>,
    cells: &'a mut Vec<TableCell>,
}

fn grid(shape: &mut Shape) -> Result<Grid<'_>, CommandError> {
    let id = shape.id;
    match &mut shape.kind {
        ShapeKind::Table {
            rows,
            cols,
            col_widths_emu,
            row_heights_emu,
            row_set_emu,
            cells,
        } => {
            if cells.iter().any(|c| c.merged) {
                return Err(CommandError::MergedTable(id));
            }
            Ok(Grid {
                rect: &mut shape.rect,
                rows,
                cols,
                widths: col_widths_emu,
                heights: row_heights_emu,
                set: row_set_emu,
                cells,
            })
        }
        _ => Err(CommandError::NotTable(id)),
    }
}

/// Insert `count` empty rows/columns before track `before` (0-based; the track count
/// appends), like `Rows.Add` / `Columns.Add`: each new track copies the size of the one it
/// goes before (the last when appending), the table grows by that and keeps its left and
/// top, and the text of the other cells stays in its cells.
pub fn insert_tracks(
    shape: &mut Shape,
    axis: Axis,
    before: usize,
    count: usize,
) -> Result<(), CommandError> {
    let g = grid(shape)?;
    let (r, c) = ((*g.rows).max(1) as usize, (*g.cols).max(1) as usize);
    let n = if axis == Axis::Rows { r } else { c };
    let other = if axis == Axis::Rows { c } else { r };
    let new_cells = other as u64 * (n as u64 + count as u64);
    if count == 0 || before > n || new_cells > u64::from(pub_write::MAX_CELLS) {
        return Err(CommandError::BadTrackRange(axis));
    }
    let mut w = tracks_of(g.widths, c, g.rect.w);
    let mut h = tracks_of(g.heights, r, g.rect.h);
    let mut cells: Vec<TableCell> = std::mem::take(g.cells);
    cells.resize_with(r * c, empty_cell);
    match axis {
        Axis::Rows => {
            let size = h[before.min(r - 1)];
            h.splice(before..before, std::iter::repeat(size).take(count));
            if g.set.len() == r {
                let set = g.set[before.min(r - 1)];
                g.set.splice(before..before, std::iter::repeat(set).take(count));
            } else {
                g.set.clear();
            }
            let at = before * c;
            let src = before.min(r - 1) * c;
            let blank: Vec<TableCell> = (0..c).map(|i| empty_cell_like(&cells[src + i])).collect();
            cells.splice(at..at, blank.iter().cloned().cycle().take(count * c));
            g.rect.h += size * count as i64;
            *g.rows = (r + count) as u32;
        }
        Axis::Columns => {
            let size = w[before.min(c - 1)];
            w.splice(before..before, std::iter::repeat(size).take(count));
            let mut out = Vec::with_capacity(r * (c + count));
            let mut it = cells.into_iter();
            for _ in 0..r {
                let row: Vec<TableCell> = it.by_ref().take(c).collect();
                out.extend(row[..before].iter().cloned());
                let blank = empty_cell_like(&row[before.min(c - 1)]);
                out.extend(std::iter::repeat(blank).take(count));
                out.extend(row[before..].iter().cloned());
            }
            cells = out;
            g.rect.w += size * count as i64;
            *g.cols = (c + count) as u32;
        }
    }
    *g.widths = w;
    *g.heights = h;
    *g.cells = cells;
    Ok(())
}

/// Delete `count` rows/columns from `first` with their text, like `Rows(n).Delete`: the
/// table shrinks by their size. Deleting every row or every column is refused.
pub fn delete_tracks(
    shape: &mut Shape,
    axis: Axis,
    first: usize,
    count: usize,
) -> Result<(), CommandError> {
    let g = grid(shape)?;
    let (r, c) = ((*g.rows).max(1) as usize, (*g.cols).max(1) as usize);
    let n = if axis == Axis::Rows { r } else { c };
    if count == 0 || first >= n || first + count > n {
        return Err(CommandError::BadTrackRange(axis));
    }
    if count >= n {
        return Err(CommandError::LastTrack(axis));
    }
    let mut w = tracks_of(g.widths, c, g.rect.w);
    let mut h = tracks_of(g.heights, r, g.rect.h);
    let mut cells: Vec<TableCell> = std::mem::take(g.cells);
    cells.resize_with(r * c, empty_cell);
    match axis {
        Axis::Rows => {
            g.rect.h -= h.drain(first..first + count).sum::<i64>();
            if g.set.len() == r {
                g.set.drain(first..first + count);
            } else {
                g.set.clear();
            }
            cells.drain(first * c..(first + count) * c);
            *g.rows = (r - count) as u32;
        }
        Axis::Columns => {
            g.rect.w -= w.drain(first..first + count).sum::<i64>();
            let mut out = Vec::with_capacity(r * (c - count));
            let mut it = cells.into_iter();
            for _ in 0..r {
                out.extend(it.by_ref().take(first));
                it.by_ref().take(count).for_each(drop);
                out.extend(it.by_ref().take(c - first - count));
            }
            cells = out;
            *g.cols = (c - count) as u32;
        }
    }
    *g.widths = w;
    *g.heights = h;
    *g.cells = cells;
    Ok(())
}

/// Publisher grows a table row to fit its text (`GrowToFitText`, spec 4E.1): a row is as
/// high as its tallest cell's text plus that cell's top and bottom margins, and never lower
/// than the height it was set to (`row_set_emu`, else the stored height). Call it after
/// anything that changes cell text, formatting, row heights or column widths: rows grow or
/// shrink back, and the frame height follows the sum of the rows. Does nothing for other
/// shapes, for tables with merged cells, or while no row is taller than its set height.
pub fn fit_rows(shape: &mut Shape) {
    let ShapeKind::Table {
        rows,
        cols,
        col_widths_emu,
        row_heights_emu,
        row_set_emu,
        cells,
    } = &mut shape.kind
    else {
        return;
    };
    if cells.iter().any(|c| c.merged) {
        return;
    }
    let (r, c) = ((*rows).clamp(1, 1000) as usize, (*cols).clamp(1, 1000) as usize);
    let stored = row_heights_emu.len() == r;
    let widths = tracks_of(col_widths_emu, c, shape.rect.w);
    let heights = tracks_of(row_heights_emu, r, shape.rect.h);
    let set = if row_set_emu.len() == r { row_set_emu.clone() } else { heights.clone() };
    let mut need = vec![0i64; r];
    {
        let mut guard = crate::story::FLOW.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = guard
            .get_or_insert_with(|| pub2pdf::FlowContext::new(&pub2pdf::Options::default()));
        for (i, cell) in cells.iter().enumerate().take(r * c) {
            let m = cell.margins_emu;
            let width_pt = (widths[i % c] - m[0] - m[2]) as f32 / 12_700.0;
            let paras = crate::export::flow_paragraphs(&cell.paragraphs);
            let text_pt = ctx.text_height(&paras, width_pt);
            let emu = (f64::from(text_pt) * 12_700.0).ceil() as i64 + m[1] + m[3];
            need[i / c] = need[i / c].max(emu);
        }
    }
    let grown: Vec<i64> = set.iter().zip(&need).map(|(&s, &n)| s.max(n)).collect();
    if grown == set {
        // Nothing is taller than its set height: rows are at their set heights.
        if stored && heights != set {
            shape.rect.h += set.iter().sum::<i64>() - heights.iter().sum::<i64>();
            *row_heights_emu = set;
        }
        row_set_emu.clear();
    } else {
        shape.rect.h += grown.iter().sum::<i64>() - heights.iter().sum::<i64>();
        *row_heights_emu = grown;
        *row_set_emu = set;
    }
}
