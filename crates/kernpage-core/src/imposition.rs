//! Print layouts ("imposition"): where each publication page goes on the sheets of paper.
//!
//! Pure geometry, no GUI and no printing. All sizes are in points (1/72 inch). Sheets are
//! described from the top-left corner, y growing downwards. The rules follow what the
//! reference program does with the same page sizes:
//!
//! * One page per sheet: the page is centred on the sheet (scaled down if it is bigger).
//! * Multiple copies per sheet: one page repeated over a grid, the next page on the next
//!   sheet.
//! * Multiple pages per sheet: the same grid, filled row by row; leftover cells stay empty.
//! * Booklet, side-fold: landscape sheets with two pages across; the page count is rounded up
//!   to an even number (blank pages at the end) and the pages are ordered for folding.
//!
//! The grid has one cell per page, spaced by the gap; the number of columns is
//! `floor((sheet width - 2 * margin + gap) / (page width + gap))`, rows likewise, and the
//! grid is centred on the sheet.

/// How the pages are arranged on the paper.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PrintLayout {
    OnePerSheet,
    CopiesPerSheet,
    PagesPerSheet,
    Booklet,
}

impl PrintLayout {
    pub const ALL: [PrintLayout; 4] = [
        PrintLayout::OnePerSheet,
        PrintLayout::CopiesPerSheet,
        PrintLayout::PagesPerSheet,
        PrintLayout::Booklet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PrintLayout::OnePerSheet => "One page per sheet",
            PrintLayout::CopiesPerSheet => "Multiple copies per sheet",
            PrintLayout::PagesPerSheet => "Multiple pages per sheet",
            PrintLayout::Booklet => "Booklet, side-fold",
        }
    }
}

/// Margin (left = right, top = bottom) and the gap between neighbouring pages, in points.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Spacing {
    pub margin_x: f64,
    pub margin_y: f64,
    pub gap_x: f64,
    pub gap_y: f64,
}

/// A rectangle on a sheet, in points from its top-left corner.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SheetRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// One page (or an empty cell) on a sheet side.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placement {
    /// 0-based publication page; `None` is an empty cell.
    pub page: Option<usize>,
    pub rect: SheetRect,
}

/// One side of one sheet of paper.
#[derive(Clone, PartialEq, Debug)]
pub struct SheetSide {
    /// The paper as used (width, height); landscape when the layout turns it.
    pub paper: (f64, f64),
    pub placements: Vec<Placement>,
    /// 0-based physical sheet. Two consecutive booklet sides share a sheet.
    pub sheet: usize,
    /// Booklet only: the second side of the sheet.
    pub back: bool,
}

/// Everything the layout depends on.
#[derive(Clone, Debug)]
pub struct Job<'a> {
    pub layout: PrintLayout,
    /// Paper, portrait (width, height).
    pub paper: (f64, f64),
    /// Publication page size (width, height).
    pub page: (f64, f64),
    pub spacing: Spacing,
    /// 0-based pages to print, in order.
    pub pages: &'a [usize],
    pub copies: u32,
    pub collate: bool,
}

/// US Letter, the paper the layouts assume when the printer's paper is not known.
pub const LETTER: (f64, f64) = (612.0, 792.0);

const EPS: f64 = 0.01;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.5
}

/// The default sheet margins and page gaps for a page size (the values the reference program
/// uses for its stock sizes; everything else prints edge to edge: 0).
pub fn default_spacing(page: (f64, f64), _paper: (f64, f64)) -> Spacing {
    let s = |mx: f64, my: f64, gx: f64, gy: f64| Spacing { margin_x: mx, margin_y: my, gap_x: gx, gap_y: gy };
    let (w, h) = page;
    let is = |pw: f64, ph: f64| near(w, pw) && near(h, ph);
    if is(252.0, 144.0) {
        s(36.0, 12.02394, 36.0, 12.02394)
    } else if is(144.0, 252.0) {
        s(51.12, 18.0, 38.88, 0.0)
    } else if is(240.912, 155.88) {
        s(37.8, 10.44, 37.8, 10.44)
    } else if is(155.88, 240.912) {
        s(31.896, 29.736, 31.896, 29.736)
    } else if is(257.976, 155.88) {
        s(26.424, 10.368, 26.424, 10.368)
    } else if is(189.0, 72.0) {
        s(13.536, 36.0, 9.0, 0.0)
    } else if is(72.0, 189.0) {
        s(36.0, 13.536, 0.0, 9.0)
    } else {
        Spacing::default()
    }
}

/// Columns and rows of pages that fit a sheet of this size.
fn grid_size(sheet: (f64, f64), page: (f64, f64), sp: Spacing) -> (usize, usize) {
    let fit = |avail: f64, margin: f64, gap: f64, size: f64| -> usize {
        if size <= 0.0 {
            return 0;
        }
        let n = ((avail - 2.0 * margin + gap) / (size + gap) + EPS).floor();
        if n < 1.0 { 0 } else { n as usize }
    };
    (
        fit(sheet.0, sp.margin_x, sp.gap_x, page.0),
        fit(sheet.1, sp.margin_y, sp.gap_y, page.1),
    )
}

/// The sheet orientation that holds more pages (portrait on a tie), with its columns and rows.
fn best_grid(paper: (f64, f64), page: (f64, f64), sp: Spacing) -> ((f64, f64), usize, usize) {
    let (pw, ph) = (paper.0.min(paper.1), paper.0.max(paper.1));
    let (pc, pr) = grid_size((pw, ph), page, sp);
    let (lc, lr) = grid_size((ph, pw), page, sp);
    if lc * lr > pc * pr {
        ((ph, pw), lc, lr)
    } else {
        ((pw, ph), pc, pr)
    }
}

/// How many pages fit on one sheet in the multiple-copies / multiple-pages layouts.
pub fn pages_per_sheet(page: (f64, f64), paper: (f64, f64), sp: Spacing) -> usize {
    let (_, c, r) = best_grid(paper, page, sp);
    c * r
}

fn booklet_fits(page: (f64, f64), paper: (f64, f64)) -> bool {
    let (w, h) = (paper.0.max(paper.1), paper.0.min(paper.1));
    page.0 * 2.0 <= w + EPS && page.1 <= h + EPS
}

/// The layouts that make sense for this page on this paper, in menu order.
pub fn available_layouts(page: (f64, f64), paper: (f64, f64)) -> Vec<PrintLayout> {
    let sp = default_spacing(page, paper);
    let mut v = vec![PrintLayout::OnePerSheet];
    if pages_per_sheet(page, paper, sp) >= 2 {
        v.push(PrintLayout::CopiesPerSheet);
        v.push(PrintLayout::PagesPerSheet);
    }
    if booklet_fits(page, paper) {
        v.push(PrintLayout::Booklet);
    }
    v
}

/// The layout a new Print page starts with: small pages that tile four or more to a sheet
/// print as copies (business cards, quarter pages); everything else one page per sheet.
pub fn default_layout(page: (f64, f64), paper: (f64, f64)) -> PrintLayout {
    let sp = default_spacing(page, paper);
    if pages_per_sheet(page, paper, sp) >= 4 {
        PrintLayout::CopiesPerSheet
    } else {
        PrintLayout::OnePerSheet
    }
}

/// `page` scaled down (never up) to fit `cell`, centred in it.
fn fit_in(cell: SheetRect, page: (f64, f64)) -> SheetRect {
    let scale = (cell.w / page.0).min(cell.h / page.1).min(1.0);
    let (w, h) = (page.0 * scale, page.1 * scale);
    SheetRect { x: cell.x + (cell.w - w) / 2.0, y: cell.y + (cell.h - h) / 2.0, w, h }
}

/// Booklet order: the pairs (left, right) of 1-based page numbers, one per sheet side, for
/// `n` pages (rounded up to even). Numbers above the real page count are blanks.
pub fn booklet_order(n: usize) -> Vec<(usize, usize)> {
    let n = n + n % 2;
    (0..n / 2)
        .map(|k| if k % 2 == 0 { (n - k, k + 1) } else { (k + 1, n - k) })
        .collect()
}

/// Lay out the job: every sheet side that will be printed, in print order.
pub fn impose(job: &Job) -> Vec<SheetSide> {
    if job.pages.is_empty() {
        return Vec::new();
    }
    let copies = job.copies.max(1) as usize;
    let mut sides = match job.layout {
        PrintLayout::OnePerSheet => one_per_sheet(job),
        PrintLayout::CopiesPerSheet | PrintLayout::PagesPerSheet => grid_sheets(job),
        PrintLayout::Booklet => booklet(job),
    };
    let collates = matches!(job.layout, PrintLayout::OnePerSheet | PrintLayout::Booklet);
    let base = std::mem::take(&mut sides);
    let repeated: Vec<SheetSide> = if collates && !job.collate {
        base.into_iter().flat_map(|s| std::iter::repeat_n(s, copies)).collect()
    } else {
        (0..copies).flat_map(|_| base.iter().cloned()).collect()
    };
    renumber(repeated, job.layout == PrintLayout::Booklet)
}

fn renumber(mut sides: Vec<SheetSide>, booklet: bool) -> Vec<SheetSide> {
    for (i, s) in sides.iter_mut().enumerate() {
        if booklet {
            s.sheet = i / 2;
            s.back = i % 2 == 1;
        } else {
            s.sheet = i;
            s.back = false;
        }
    }
    sides
}

fn one_per_sheet(job: &Job) -> Vec<SheetSide> {
    let (a, b) = (job.paper.0.min(job.paper.1), job.paper.0.max(job.paper.1));
    let paper = if job.page.0 > job.page.1 { (b, a) } else { (a, b) };
    let cell = SheetRect { x: 0.0, y: 0.0, w: paper.0, h: paper.1 };
    let rect = fit_in(cell, job.page);
    job.pages
        .iter()
        .map(|&p| SheetSide { paper, placements: vec![Placement { page: Some(p), rect }], sheet: 0, back: false })
        .collect()
}

fn grid_cells(paper: (f64, f64), page: (f64, f64), sp: Spacing, cols: usize, rows: usize) -> Vec<SheetRect> {
    let total_w = cols as f64 * page.0 + (cols.saturating_sub(1)) as f64 * sp.gap_x;
    let total_h = rows as f64 * page.1 + (rows.saturating_sub(1)) as f64 * sp.gap_y;
    let (x0, y0) = ((paper.0 - total_w) / 2.0, (paper.1 - total_h) / 2.0);
    let mut cells = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        for c in 0..cols {
            cells.push(SheetRect {
                x: x0 + c as f64 * (page.0 + sp.gap_x),
                y: y0 + r as f64 * (page.1 + sp.gap_y),
                w: page.0,
                h: page.1,
            });
        }
    }
    cells
}

fn grid_sheets(job: &Job) -> Vec<SheetSide> {
    let (paper, cols, rows) = best_grid(job.paper, job.page, job.spacing);
    if cols * rows == 0 {
        // Nothing fits: one page per sheet, so the job still prints.
        return one_per_sheet(job);
    }
    let cells = grid_cells(paper, job.page, job.spacing, cols, rows);
    let side = |pages: Vec<Option<usize>>| SheetSide {
        paper,
        placements: cells
            .iter()
            .zip(pages)
            .map(|(&rect, page)| Placement { page, rect })
            .collect(),
        sheet: 0,
        back: false,
    };
    if job.layout == PrintLayout::CopiesPerSheet {
        job.pages.iter().map(|&p| side(vec![Some(p); cells.len()])).collect()
    } else {
        job.pages
            .chunks(cells.len())
            .map(|chunk| {
                let mut v: Vec<Option<usize>> = chunk.iter().map(|&p| Some(p)).collect();
                v.resize(cells.len(), None);
                side(v)
            })
            .collect()
    }
}

fn booklet(job: &Job) -> Vec<SheetSide> {
    let (w, h) = (job.paper.0.max(job.paper.1), job.paper.0.min(job.paper.1));
    let half = w / 2.0;
    let left = SheetRect { x: 0.0, y: 0.0, w: half, h };
    let right = SheetRect { x: half, y: 0.0, w: half, h };
    let (lr, rr) = (fit_in(left, job.page), fit_in(right, job.page));
    let page_at = |number: usize| job.pages.get(number - 1).copied();
    booklet_order(job.pages.len())
        .into_iter()
        .map(|(l, r)| SheetSide {
            paper: (w, h),
            placements: vec![
                Placement { page: page_at(l), rect: lr },
                Placement { page: page_at(r), rect: rr },
            ],
            sheet: 0,
            back: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIZ: (f64, f64) = (252.0, 144.0);
    const HALF: (f64, f64) = (396.0, 612.0);
    const QUARTER: (f64, f64) = (306.0, 396.0);

    fn job<'a>(layout: PrintLayout, page: (f64, f64), pages: &'a [usize]) -> Job<'a> {
        Job {
            layout,
            paper: LETTER,
            page,
            spacing: default_spacing(page, LETTER),
            pages,
            copies: 1,
            collate: true,
        }
    }

    fn pages_of(sides: &[SheetSide]) -> Vec<Vec<Option<usize>>> {
        sides.iter().map(|s| s.placements.iter().map(|p| p.page.map(|n| n + 1)).collect()).collect()
    }

    fn booklet_pages(n: usize) -> Vec<Vec<Option<usize>>> {
        let pages: Vec<usize> = (0..n).collect();
        pages_of(&impose(&job(PrintLayout::Booklet, HALF, &pages)))
    }

    fn pair(l: Option<usize>, r: Option<usize>) -> Vec<Option<usize>> {
        vec![l, r]
    }

    #[test]
    fn booklet_order_matches_the_reference_for_even_counts() {
        assert_eq!(booklet_order(2), vec![(2, 1)]);
        assert_eq!(booklet_order(4), vec![(4, 1), (2, 3)]);
        assert_eq!(booklet_order(8), vec![(8, 1), (2, 7), (6, 3), (4, 5)]);
        assert_eq!(booklet_order(12), vec![(12, 1), (2, 11), (10, 3), (4, 9), (8, 5), (6, 7)]);
    }

    #[test]
    fn booklet_pads_odd_counts_with_blank_pages_at_the_end() {
        assert_eq!(booklet_pages(3), vec![pair(None, Some(1)), pair(Some(2), Some(3))]);
        assert_eq!(
            booklet_pages(5),
            vec![pair(None, Some(1)), pair(Some(2), Some(5)), pair(Some(4), Some(3))]
        );
        assert_eq!(booklet_pages(7).len(), 4);
        assert_eq!(booklet_pages(7)[0], pair(None, Some(1)));
        assert_eq!(booklet_pages(13)[6], pair(Some(8), Some(7)));
    }

    #[test]
    fn booklet_of_eight_and_twelve_pages_is_in_folding_order() {
        let eight = booklet_pages(8);
        assert_eq!(eight[0], pair(Some(8), Some(1)));
        assert_eq!(eight[1], pair(Some(2), Some(7)));
        assert_eq!(eight[2], pair(Some(6), Some(3)));
        assert_eq!(eight[3], pair(Some(4), Some(5)));
        assert_eq!(booklet_pages(12)[5], pair(Some(6), Some(7)));
    }

    #[test]
    fn booklet_uses_landscape_sheets_with_a_page_in_each_half_and_front_back_pairs() {
        let pages: Vec<usize> = (0..8).collect();
        let sides = impose(&job(PrintLayout::Booklet, HALF, &pages));
        assert_eq!(sides.len(), 4);
        for s in &sides {
            assert_eq!(s.paper, (792.0, 612.0));
            assert_eq!(s.placements[0].rect, SheetRect { x: 0.0, y: 0.0, w: 396.0, h: 612.0 });
            assert_eq!(s.placements[1].rect, SheetRect { x: 396.0, y: 0.0, w: 396.0, h: 612.0 });
        }
        let marks: Vec<(usize, bool)> = sides.iter().map(|s| (s.sheet, s.back)).collect();
        assert_eq!(marks, vec![(0, false), (0, true), (1, false), (1, true)]);
    }

    #[test]
    fn booklet_follows_the_selected_pages_not_the_page_numbers() {
        let sides = impose(&job(PrintLayout::Booklet, HALF, &[4, 6, 7, 9]));
        assert_eq!(pages_of(&sides), vec![pair(Some(10), Some(5)), pair(Some(7), Some(8))]);
    }

    #[test]
    fn booklet_pages_smaller_than_their_slot_are_centred_not_scaled_up() {
        let pages = [0usize, 1];
        let sides = impose(&job(PrintLayout::Booklet, QUARTER, &pages));
        let r = sides[0].placements[0].rect;
        assert_eq!((r.w, r.h), (306.0, 396.0));
        assert_eq!((r.x, r.y), (45.0, 108.0));
        assert_eq!(sides[0].placements[1].rect.x, 396.0 + 45.0);
    }

    #[test]
    fn business_card_grid_is_two_by_five_centred_with_the_default_spacing() {
        let sp = default_spacing(BIZ, LETTER);
        assert_eq!(sp, Spacing { margin_x: 36.0, margin_y: 12.02394, gap_x: 36.0, gap_y: 12.02394 });
        assert_eq!(pages_per_sheet(BIZ, LETTER, sp), 10);
        let sides = impose(&job(PrintLayout::CopiesPerSheet, BIZ, &[0]));
        assert_eq!(sides.len(), 1);
        let s = &sides[0];
        assert_eq!(s.paper, (612.0, 792.0));
        assert_eq!(s.placements.len(), 10);
        assert!(s.placements.iter().all(|p| p.page == Some(0)));
        let first = s.placements[0].rect;
        assert!((first.x - 36.0).abs() < 1e-9);
        let total_h = 5.0 * 144.0 + 4.0 * 12.02394;
        assert!((first.y - (792.0 - total_h) / 2.0).abs() < 1e-9);
        assert!((first.y - 12.02394).abs() < 0.1, "close to the stated margin");
        // Pitch is page + gap.
        assert!((s.placements[1].rect.x - first.x - 288.0).abs() < 1e-9);
        assert!((s.placements[2].rect.y - first.y - (144.0 + 12.02394)).abs() < 1e-9);
        // Centred: equal left and right margins.
        let last = s.placements[9].rect;
        assert!((first.x - (612.0 - last.x - last.w)).abs() < 1e-9);
    }

    #[test]
    fn copies_per_sheet_puts_each_page_on_its_own_sheet() {
        let sides = impose(&job(PrintLayout::CopiesPerSheet, BIZ, &[0, 1, 2]));
        assert_eq!(sides.len(), 3);
        for (i, s) in sides.iter().enumerate() {
            assert!(s.placements.iter().all(|p| p.page == Some(i)));
        }
    }

    #[test]
    fn copies_repeat_the_whole_sheet_sequence_and_ignore_collate() {
        for collate in [true, false] {
            let mut j = job(PrintLayout::CopiesPerSheet, BIZ, &[0, 1, 2]);
            j.copies = 7;
            j.collate = collate;
            let sides = impose(&j);
            assert_eq!(sides.len(), 21);
            let order: Vec<usize> = sides.iter().map(|s| s.placements[0].page.unwrap()).collect();
            assert_eq!(&order[..6], &[0, 1, 2, 0, 1, 2]);
        }
    }

    #[test]
    fn quarter_page_grid_is_two_by_two_filled_row_by_row_with_empty_leftovers() {
        let pages: Vec<usize> = (0..6).collect();
        let sides = impose(&job(PrintLayout::PagesPerSheet, QUARTER, &pages));
        assert_eq!(sides.len(), 2);
        assert_eq!(
            pages_of(&sides),
            vec![
                vec![Some(1), Some(2), Some(3), Some(4)],
                vec![Some(5), Some(6), None, None]
            ]
        );
        let c = &sides[0].placements;
        assert_eq!((c[0].rect.x, c[0].rect.y), (0.0, 0.0));
        assert_eq!((c[1].rect.x, c[1].rect.y), (306.0, 0.0));
        assert_eq!((c[2].rect.x, c[2].rect.y), (0.0, 396.0));
        assert_eq!((c[3].rect.x, c[3].rect.y), (306.0, 396.0));
        assert_eq!(sides[0].paper, (612.0, 792.0));
    }

    #[test]
    fn business_card_pages_per_sheet_fill_row_major() {
        let sides = impose(&job(PrintLayout::PagesPerSheet, BIZ, &[0, 1, 2]));
        assert_eq!(sides.len(), 1);
        let p = &sides[0].placements;
        assert_eq!(p.iter().filter(|c| c.page.is_some()).count(), 3);
        assert_eq!(p[0].page, Some(0));
        assert_eq!(p[1].page, Some(1));
        assert_eq!(p[2].page, Some(2));
        assert!(p[1].rect.x > p[0].rect.x && p[1].rect.y == p[0].rect.y);
        assert!(p[2].rect.y > p[0].rect.y && p[2].rect.x == p[0].rect.x);
    }

    #[test]
    fn half_letter_goes_two_up_on_a_landscape_sheet() {
        let sides = impose(&job(PrintLayout::PagesPerSheet, HALF, &[0, 1, 2, 3]));
        assert_eq!(sides.len(), 2);
        assert_eq!(sides[0].paper, (792.0, 612.0));
        assert_eq!(pages_of(&sides), vec![vec![Some(1), Some(2)], vec![Some(3), Some(4)]]);
        assert_eq!(sides[0].placements[1].rect.x, 396.0);
    }

    #[test]
    fn one_per_sheet_centres_small_pages_and_scales_big_ones_down() {
        let small = impose(&job(PrintLayout::OnePerSheet, HALF, &[0, 1]));
        assert_eq!(small.len(), 2);
        let r = small[0].placements[0].rect;
        assert_eq!((r.w, r.h, r.x, r.y), (396.0, 612.0, 108.0, 90.0));
        // Landscape publications turn the paper.
        let wide = impose(&job(PrintLayout::OnePerSheet, (792.0, 612.0), &[0]));
        assert_eq!(wide[0].paper, (792.0, 612.0));
        assert_eq!(wide[0].placements[0].rect, SheetRect { x: 0.0, y: 0.0, w: 792.0, h: 612.0 });
        // A page bigger than the paper shrinks, keeping its shape, and is centred.
        let big = impose(&job(PrintLayout::OnePerSheet, (1224.0, 792.0), &[0]));
        let r = big[0].placements[0].rect;
        assert!((r.w - 792.0).abs() < 1e-9 && (r.h - 792.0 * 792.0 / 1224.0).abs() < 1e-9);
        assert!((r.x).abs() < 1e-9 && (r.y - (612.0 - r.h) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn one_per_sheet_copies_follow_the_collate_rule() {
        let mut j = job(PrintLayout::OnePerSheet, (612.0, 792.0), &[0, 1, 2]);
        j.copies = 2;
        let order = |sides: Vec<SheetSide>| -> Vec<usize> {
            sides.iter().map(|s| s.placements[0].page.unwrap()).collect()
        };
        assert_eq!(order(impose(&j)), vec![0, 1, 2, 0, 1, 2]);
        j.collate = false;
        assert_eq!(order(impose(&j)), vec![0, 0, 1, 1, 2, 2]);
    }

    #[test]
    fn booklet_copies_repeat_the_sides() {
        let pages = [0usize, 1, 2, 3];
        let mut j = job(PrintLayout::Booklet, HALF, &pages);
        j.copies = 2;
        let sides = impose(&j);
        assert_eq!(sides.len(), 4);
        assert_eq!(pages_of(&sides)[2], pair(Some(4), Some(1)));
        j.collate = false;
        let sides = impose(&j);
        assert_eq!(pages_of(&sides)[1], pair(Some(4), Some(1)));
        assert_eq!(pages_of(&sides)[2], pair(Some(2), Some(3)));
    }

    #[test]
    fn nothing_selected_means_no_sheets() {
        assert!(impose(&job(PrintLayout::Booklet, HALF, &[])).is_empty());
    }

    #[test]
    fn layouts_offered_depend_on_the_page_size() {
        use PrintLayout::*;
        assert_eq!(available_layouts((612.0, 792.0), LETTER), vec![OnePerSheet]);
        assert_eq!(available_layouts((792.0, 612.0), LETTER), vec![OnePerSheet]);
        assert_eq!(available_layouts(BIZ, LETTER), vec![OnePerSheet, CopiesPerSheet, PagesPerSheet, Booklet]);
        assert_eq!(available_layouts(HALF, LETTER), vec![OnePerSheet, CopiesPerSheet, PagesPerSheet, Booklet]);
        assert_eq!(available_layouts(QUARTER, LETTER), vec![OnePerSheet, CopiesPerSheet, PagesPerSheet, Booklet]);
        // A page just over half a sheet wide does not fit twice.
        assert_eq!(available_layouts((400.0, 612.0), LETTER), vec![OnePerSheet]);
    }

    #[test]
    fn default_layout_matches_the_page_size() {
        assert_eq!(default_layout((612.0, 792.0), LETTER), PrintLayout::OnePerSheet);
        assert_eq!(default_layout(HALF, LETTER), PrintLayout::OnePerSheet);
        assert_eq!(default_layout(BIZ, LETTER), PrintLayout::CopiesPerSheet);
        assert_eq!(default_layout(QUARTER, LETTER), PrintLayout::CopiesPerSheet);
    }

    #[test]
    fn labels_are_the_menu_texts() {
        let l: Vec<&str> = PrintLayout::ALL.iter().map(|l| l.label()).collect();
        assert_eq!(
            l,
            ["One page per sheet", "Multiple copies per sheet", "Multiple pages per sheet", "Booklet, side-fold"]
        );
    }
}
