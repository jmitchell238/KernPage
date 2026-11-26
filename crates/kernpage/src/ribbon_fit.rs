//! Ribbon scaling: as the window narrows, groups shrink step by step instead of clipping.
//!
//! Each group has four size variants (levels):
//!
//! 0. full: the layout at wide widths,
//! 1. reduced: galleries show fewer columns and large buttons become small buttons stacked
//!    in columns of three,
//! 2. icons: small buttons drop their labels (the label becomes the tooltip),
//! 3. collapsed: one large drop-down button (group icon and name) whose popup holds the full
//!    group.
//!
//! The width of every variant is measured once per tab (a hidden layout pass per level) and
//! cached. Each frame [`pick_variants`] chooses the level of every group from the available
//! width, and `widgets::group` reads its level from the thread-local state kept here.

use std::cell::RefCell;
use std::collections::HashMap;

pub const LEVELS: usize = 4;
pub const COLLAPSED: usize = 3;

/// What the picker needs to know about one group.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupSpec {
    /// Width of the group at each level (index = level).
    pub widths: [f32; LEVELS],
    /// Higher = more important: shrinks later. Equal priorities shrink right to left.
    pub priority: u8,
}

/// Choose a level for every group so the row fits `available`, shrinking as little as
/// possible. Levels that do not make a group narrower are skipped. If even the fully
/// collapsed row does not fit, everything is collapsed (the ribbon then scrolls).
pub fn pick_variants(groups: &[GroupSpec], available: f32) -> Vec<usize> {
    // The usable levels of each group: strictly narrower than the previous usable level.
    let steps: Vec<Vec<usize>> = groups
        .iter()
        .map(|g| {
            let mut v = vec![0];
            let mut last = g.widths[0];
            for l in 1..LEVELS {
                if g.widths[l] < last - 0.5 {
                    v.push(l);
                    last = g.widths[l];
                }
            }
            v
        })
        .collect();
    let mut pos = vec![0usize; groups.len()];
    let total = |pos: &[usize]| -> f32 {
        groups
            .iter()
            .zip(&steps)
            .zip(pos)
            .map(|((g, s), &p)| g.widths[s[p]])
            .sum()
    };
    // One group, one level per step. Next group: lowest priority first, right-most among
    // equals. Collapsing a more important group (priority > 0) is the very last resort.
    while total(&pos) > available {
        let next = (0..groups.len())
            .filter(|&g| pos[g] + 1 < steps[g].len())
            .min_by_key(|&g| {
                let collapse = groups[g].priority > 0 && steps[g][pos[g] + 1] == COLLAPSED;
                (collapse, groups[g].priority, std::cmp::Reverse(g))
            });
        match next {
            Some(g) => pos[g] += 1,
            None => break,
        }
    }
    // Use leftover space: give size back (most important group first, then left to right)
    // while the row still fits.
    if total(&pos) <= available {
        let mut back: Vec<usize> = (0..groups.len()).collect();
        back.sort_by(|&a, &b| groups[b].priority.cmp(&groups[a].priority).then(a.cmp(&b)));
        for g in back {
            while pos[g] > 0 {
                pos[g] -= 1;
                if total(&pos) > available {
                    pos[g] += 1;
                    break;
                }
            }
        }
    }
    pos.iter().zip(&steps).map(|(&p, s)| s[p]).collect()
}

/// Groups that should keep their full size longer than the default right-to-left order.
pub fn priority_of(title: &str) -> u8 {
    match title {
        "Font" | "Paragraph" | "Clipboard" | "Size" => 1,
        _ => 0,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// Normal drawing: levels come from the plan.
    Render,
    /// Hidden pass that records group widths with every group at this level.
    Measure(usize),
}

struct State {
    mode: Mode,
    plan: Vec<usize>,
    plan_titles: Vec<String>,
    next: usize,
    /// Level of the group being drawn (read by buttons and galleries).
    level: usize,
    recorded: Vec<(String, f32)>,
    stale: bool,
    /// Debug: collapse this group and open its popup.
    debug_open: Option<(String, u32)>,
    cache: HashMap<String, Vec<(String, [f32; LEVELS])>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        mode: Mode::Render,
        plan: Vec::new(),
        plan_titles: Vec::new(),
        next: 0,
        level: 0,
        recorded: Vec::new(),
        stale: false,
        debug_open: None,
        cache: HashMap::new(),
    });
}

/// The size level of the group being drawn (0 outside the ribbon).
pub fn level() -> usize {
    STATE.with(|s| s.borrow().level)
}

/// Run `f` with the current level forced to `level`.
pub fn with_level<R>(level: usize, f: impl FnOnce() -> R) -> R {
    let old = STATE.with(|s| std::mem::replace(&mut s.borrow_mut().level, level));
    let r = f();
    STATE.with(|s| s.borrow_mut().level = old);
    r
}

/// Called at the start of each `group`: the level it must draw at.
pub(crate) fn enter_group(title: &str) -> usize {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let lvl = match s.mode {
            Mode::Measure(l) => l,
            Mode::Render if s.debug_open.as_ref().is_some_and(|d| d.0 == title) => COLLAPSED,
            Mode::Render => {
                let i = s.next;
                if s.plan_titles.get(i).map(String::as_str) == Some(title) {
                    s.plan[i]
                } else {
                    s.stale = true;
                    0
                }
            }
        };
        s.next += 1;
        s.level = lvl;
        lvl
    })
}

/// Called at the end of each `group` with the width it took.
pub(crate) fn leave_group(title: &str, width: f32) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.level = 0;
        if matches!(s.mode, Mode::Measure(_)) {
            s.recorded.push((title.to_owned(), width));
        }
    })
}

/// Debug option: draw this group collapsed with its popup open.
pub fn set_debug_open(title: &str) {
    STATE.with(|s| s.borrow_mut().debug_open = Some((title.to_owned(), 0)));
}

/// True once, a few frames after start, for the group named by `set_debug_open`.
pub(crate) fn debug_open_now(title: &str) -> bool {
    STATE.with(|s| match &mut s.borrow_mut().debug_open {
        Some((t, n)) if t == title => {
            *n += 1;
            *n == 3
        }
        _ => false,
    })
}

pub fn cached(key: &str) -> Option<Vec<GroupSpec>> {
    STATE.with(|s| {
        s.borrow().cache.get(key).map(|v| {
            v.iter()
                .map(|(t, w)| GroupSpec {
                    widths: *w,
                    priority: priority_of(t),
                })
                .collect()
        })
    })
}

pub fn cached_titles(key: &str) -> Vec<String> {
    STATE.with(|s| {
        s.borrow()
            .cache
            .get(key)
            .map(|v| v.iter().map(|(t, _)| t.clone()).collect())
            .unwrap_or_default()
    })
}

pub fn begin_measure(level: usize) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.mode = Mode::Measure(level);
        s.next = 0;
        s.recorded.clear();
    })
}

pub fn end_measure() -> Vec<(String, f32)> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.mode = Mode::Render;
        s.level = 0;
        std::mem::take(&mut s.recorded)
    })
}

pub fn store(key: &str, per_level: &[Vec<(String, f32)>]) {
    let Some(first) = per_level.first() else { return };
    let rows: Vec<(String, [f32; LEVELS])> = first
        .iter()
        .enumerate()
        .map(|(i, (t, _))| {
            let mut w = [0.0; LEVELS];
            for (l, rec) in per_level.iter().enumerate().take(LEVELS) {
                w[l] = rec.get(i).map_or(f32::MAX, |r| r.1);
            }
            (t.clone(), w)
        })
        .collect();
    STATE.with(|s| s.borrow_mut().cache.insert(key.to_owned(), rows));
}

pub fn begin_render(plan: Vec<usize>, titles: Vec<String>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.mode = Mode::Render;
        s.plan = plan;
        s.plan_titles = titles;
        s.next = 0;
        s.level = 0;
        s.stale = false;
    })
}

/// End of the tab. If the groups drawn differ from the cache, drop the cache so the next
/// frame measures again; returns true when that happened.
pub fn end_render(key: &str) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let stale = s.stale || s.next != s.plan_titles.len();
        if stale {
            s.cache.remove(key);
        }
        stale
    })
}

/// Number of columns a gallery shows at the current level.
pub fn gallery_cols(full: usize) -> usize {
    match level() {
        0 => full,
        1 => (full * 2).div_ceil(3).max(1),
        _ => full.div_ceil(3).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(w: [f32; 4]) -> GroupSpec {
        GroupSpec { widths: w, priority: 0 }
    }

    #[test]
    fn fits_at_full_width_changes_nothing() {
        let gs = [g([100.0, 80.0, 60.0, 50.0]), g([200.0, 150.0, 90.0, 50.0])];
        assert_eq!(pick_variants(&gs, 300.0), vec![0, 0]);
        assert_eq!(pick_variants(&gs, 5000.0), vec![0, 0]);
    }

    #[test]
    fn shrinks_rightmost_group_first() {
        let gs = [g([100.0, 80.0, 60.0, 50.0]), g([200.0, 150.0, 90.0, 50.0])];
        assert_eq!(pick_variants(&gs, 250.0), vec![0, 1]);
    }

    #[test]
    fn one_group_goes_all_the_way_before_the_next() {
        let gs = [g([100.0, 80.0, 60.0, 50.0]), g([200.0, 150.0, 90.0, 50.0])];
        assert_eq!(pick_variants(&gs, 195.0), vec![0, 2]);
        assert_eq!(pick_variants(&gs, 150.0), vec![0, 3]);
        assert_eq!(pick_variants(&gs, 140.0), vec![1, 3]);
        assert_eq!(pick_variants(&gs, 120.0), vec![2, 3]);
        assert_eq!(pick_variants(&gs, 100.0), vec![3, 3]);
        assert_eq!(pick_variants(&gs, 10.0), vec![3, 3]);
    }

    #[test]
    fn useless_levels_are_skipped() {
        // Level 1 and 2 are not narrower: go straight to the collapsed variant.
        let gs = [g([120.0, 120.0, 130.0, 50.0])];
        assert_eq!(pick_variants(&gs, 100.0), vec![3]);
        // Level 2 is narrower than level 1: skip level 1 only.
        let gs = [g([120.0, 125.0, 90.0, 50.0])];
        assert_eq!(pick_variants(&gs, 100.0), vec![2]);
    }

    #[test]
    fn important_groups_shrink_last_and_collapse_last_of_all() {
        let mut a = g([100.0, 80.0, 60.0, 50.0]);
        a.priority = 1;
        let b = g([100.0, 80.0, 60.0, 50.0]);
        let c = g([100.0, 80.0, 60.0, 50.0]);
        let gs = [a, b, c];
        // c, then b (right to left) go all the way down first.
        assert_eq!(pick_variants(&gs, 280.0), vec![0, 0, 1]);
        assert_eq!(pick_variants(&gs, 255.0), vec![0, 0, 3]);
        assert_eq!(pick_variants(&gs, 200.0), vec![0, 3, 3]);
        // Only then does the important group shrink, and it collapses last.
        assert_eq!(pick_variants(&gs, 190.0), vec![1, 2, 3]);
        assert_eq!(pick_variants(&gs, 170.0), vec![2, 2, 3]);
        assert_eq!(pick_variants(&gs, 150.0), vec![3, 3, 3]);
    }

    #[test]
    fn leftover_space_is_given_back() {
        // The right group collapses first, then the left one shrinks; that frees enough room
        // to bring the right group back to a middle level.
        let gs = [g([300.0, 160.0, 150.0, 60.0]), g([200.0, 150.0, 100.0, 50.0])];
        assert_eq!(pick_variants(&gs, 300.0), vec![1, 2]);
    }

    #[test]
    fn does_not_shrink_more_than_needed() {
        let gs = [g([100.0, 80.0, 60.0, 50.0]), g([100.0, 80.0, 60.0, 50.0])];
        assert_eq!(pick_variants(&gs, 200.0), vec![0, 0]);
        assert_eq!(pick_variants(&gs, 199.0), vec![0, 1]);
        assert_eq!(pick_variants(&gs, 180.0), vec![0, 1]);
    }

    #[test]
    fn gallery_columns_shrink() {
        assert_eq!(with_level(0, || gallery_cols(6)), 6);
        assert_eq!(with_level(1, || gallery_cols(6)), 4);
        assert_eq!(with_level(2, || gallery_cols(6)), 2);
        assert_eq!(with_level(1, || gallery_cols(7)), 5);
        assert_eq!(with_level(2, || gallery_cols(7)), 3);
    }
}
