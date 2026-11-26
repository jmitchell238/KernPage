//! Plain-text round trips for the in-place editor.

use crate::command::CharChange;
use crate::model::{Align, ListInfo, ParaIndents, Paragraph, Rgb, Run};
use std::ops::Range;

/// The paragraphs as editable plain text, one line per paragraph.
pub fn paragraphs_to_text(paragraphs: &[Paragraph]) -> String {
    paragraphs
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A run's formatting without its text.
fn style_of(r: &Run) -> Run {
    r.with_text("")
}

fn same_style(a: &Run, b: &Run) -> bool {
    a.font == b.font
        && a.size_pt == b.size_pt
        && a.bold == b.bold
        && a.italic == b.italic
        && a.underline == b.underline
        && a.script == b.script
        && a.color == b.color
        && a.field == b.field
}

/// Merge neighbouring runs with identical formatting and drop empty runs (an empty
/// paragraph keeps one empty run so it still has a style).
pub fn normalize_runs(runs: &[Run]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for r in runs.iter().filter(|r| !r.text.is_empty()) {
        match out.last_mut() {
            Some(prev) if same_style(prev, r) => prev.text.push_str(&r.text),
            _ => out.push(r.clone()),
        }
    }
    if out.is_empty() {
        out.push(runs.first().cloned().unwrap_or_else(|| Run::plain("")));
    }
    out
}

/// `paragraphs` with runs merged and empty runs dropped; two paragraph lists that look the
/// same compare equal after this.
pub fn canonical(paragraphs: &[Paragraph]) -> Vec<Paragraph> {
    paragraphs
        .iter()
        .map(|p| Paragraph {
            runs: normalize_runs(&p.runs),
            align: p.align,
            spacing: p.spacing,
            list: p.list.clone(),
            indents: p.indents,
        })
        .collect()
}

/// Number of characters in the editable text (paragraphs joined by one `\n`).
pub fn text_len(paragraphs: &[Paragraph]) -> usize {
    paragraphs
        .iter()
        .map(|p| p.text().chars().count())
        .sum::<usize>()
        + paragraphs.len().saturating_sub(1)
}

/// `[start, end]` character span of each paragraph in the editable text (`end` is the
/// position of the paragraph break after it, or the end of the text).
fn para_spans(paragraphs: &[Paragraph]) -> Vec<(usize, usize)> {
    let mut at = 0;
    paragraphs
        .iter()
        .map(|p| {
            let n = p.text().chars().count();
            let span = (at, at + n);
            at += n + 1;
            span
        })
        .collect()
}

pub fn apply_change(r: &mut Run, change: &CharChange) {
    match change {
        CharChange::FontName(n) => r.font = n.clone(),
        CharChange::Size(s) => r.size_pt = *s,
        CharChange::Bold(b) => r.bold = *b,
        CharChange::Italic(i) => r.italic = *i,
        CharChange::Underline(u) => r.underline = *u,
        CharChange::Script(s) => r.script = *s,
        CharChange::Color(c) => r.color = *c,
        CharChange::Clear => {
            r.font = crate::model::DEFAULT_FONT.to_owned();
            r.size_pt = crate::model::FILE_DEFAULT_SIZE_PT;
            r.bold = false;
            r.italic = false;
            r.underline = false;
            r.script = crate::model::Script::Normal;
            r.color = Rgb::BLACK;
        }
    }
}

fn byte_at(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(b, _)| b)
}

/// Apply `change` to the characters of `range` (character offsets in the editable text),
/// splitting runs at the range edges and merging identical neighbours afterwards.
/// Returns whether anything changed.
pub fn format_range(
    paragraphs: &mut [Paragraph],
    range: Range<usize>,
    change: &CharChange,
) -> bool {
    let spans = para_spans(paragraphs);
    let mut any = false;
    for (p, &(ps, pe)) in paragraphs.iter_mut().zip(&spans) {
        let (lo, hi) = (range.start.max(ps), range.end.min(pe));
        if lo >= hi {
            continue;
        }
        let (lo, hi) = (lo - ps, hi - ps);
        let mut out: Vec<Run> = Vec::new();
        let mut at = 0;
        for r in &p.runs {
            let n = r.text.chars().count();
            let (a, b) = (lo.clamp(at, at + n) - at, hi.clamp(at, at + n) - at);
            if a == b {
                out.push(r.clone());
            } else {
                let (x, y) = (byte_at(&r.text, a), byte_at(&r.text, b));
                if x > 0 {
                    out.push(r.with_text(&r.text[..x]));
                }
                let mut mid = r.with_text(&r.text[x..y]);
                apply_change(&mut mid, change);
                out.push(mid);
                if y < r.text.len() {
                    out.push(r.with_text(&r.text[y..]));
                }
            }
            at += n;
        }
        let merged = normalize_runs(&out);
        if merged != normalize_runs(&p.runs) {
            any = true;
        }
        p.runs = merged;
    }
    any
}

/// Whether paragraph `(ps, pe)` is touched by the selection `range`: a caret touches the
/// paragraph it is in; a selection touches every paragraph it covers at least one
/// character or break of.
fn touches(range: &Range<usize>, ps: usize, pe: usize) -> bool {
    if range.start >= range.end {
        ps <= range.start && range.start <= pe
    } else {
        range.start <= pe && range.end > ps
    }
}

/// Set the alignment of every paragraph touched by `range`. Returns whether anything changed.
pub fn align_range(paragraphs: &mut [Paragraph], range: Range<usize>, align: Align) -> bool {
    let spans = para_spans(paragraphs);
    let mut any = false;
    for (p, &(ps, pe)) in paragraphs.iter_mut().zip(&spans) {
        if touches(&range, ps, pe) && p.align != align {
            p.align = align;
            any = true;
        }
    }
    any
}

/// A change to paragraph spacing; `None` fields are left alone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpacingChange {
    pub before_pt: Option<f32>,
    pub after_pt: Option<f32>,
    pub line: Option<crate::model::LineSpacing>,
}

/// Apply `change` to every paragraph touched by `range`. Returns whether anything changed.
pub fn spacing_range(
    paragraphs: &mut [Paragraph],
    range: Range<usize>,
    change: SpacingChange,
) -> bool {
    let spans = para_spans(paragraphs);
    let mut any = false;
    for (p, &(ps, pe)) in paragraphs.iter_mut().zip(&spans) {
        if !touches(&range, ps, pe) {
            continue;
        }
        let old = p.spacing;
        if let Some(b) = change.before_pt {
            p.spacing.space_before_pt = b;
        }
        if let Some(a) = change.after_pt {
            p.spacing.space_after_pt = a;
        }
        if let Some(l) = change.line {
            p.spacing.line = l;
        }
        any |= p.spacing != old;
    }
    any
}

/// The spacing shared by every paragraph `range` touches; `None` where they differ.
pub fn selection_spacing(
    paragraphs: &[Paragraph],
    range: Range<usize>,
) -> Option<crate::model::ParaSpacing> {
    let spans = para_spans(paragraphs);
    let mut found: Option<crate::model::ParaSpacing> = None;
    for (p, &(ps, pe)) in paragraphs.iter().zip(&spans) {
        if !touches(&range, ps, pe) {
            continue;
        }
        match found {
            None => found = Some(p.spacing),
            Some(f) if f != p.spacing => return None,
            _ => {}
        }
    }
    found
}

/// A change to the bullets, numbering or indents of paragraphs (spec 4R).
#[derive(Debug, Clone, PartialEq)]
pub enum ListChange {
    /// Put the paragraphs in this list (replacing any list they are in). As Publisher does,
    /// the indents become left = list indent, first line = -list indent (right is kept).
    Set(ListInfo),
    /// Take the paragraphs out of their list: the list and its indents go (the right indent
    /// stays). Paragraphs not in a list are left alone.
    Clear,
    /// Number separator index 0-8 (spec 4R.3) of numbered paragraphs.
    Separator(u8),
    /// Start-at number of numbered paragraphs.
    Start(u32),
    /// List indent in points of paragraphs in a list; left = list indent and first line =
    /// -list indent follow (spec 4R.1; 0 gives no indents).
    ListIndent(f32),
    /// Increase (positive) or decrease (negative) the indent of each paragraph by this many
    /// points, never below 0: a list paragraph's list indent (left and first line follow, as
    /// `ListIndent`), a plain paragraph's left indent. Each paragraph steps from its own value,
    /// so a mixed selection is one command (the Home > Decrease / Increase Indent buttons).
    StepIndent(f32),
    /// Left, first-line and right indents in points; `None` leaves one alone. The list
    /// indent of a list paragraph does not change (spec 4R.1).
    Indents {
        left_pt: Option<f32>,
        first_line_pt: Option<f32>,
        right_pt: Option<f32>,
    },
}

/// Apply `change` to every paragraph touched by `range`. Returns whether anything changed.
pub fn list_range(paragraphs: &mut [Paragraph], range: Range<usize>, change: &ListChange) -> bool {
    let spans = para_spans(paragraphs);
    let mut any = false;
    for (p, &(ps, pe)) in paragraphs.iter_mut().zip(&spans) {
        if !touches(&range, ps, pe) {
            continue;
        }
        let (old_list, old_indents) = (p.list.clone(), p.indents);
        match change {
            ListChange::Set(info) => {
                p.indents = ParaIndents {
                    right_pt: p.indents.right_pt,
                    ..ParaIndents::for_list(info.list_indent_pt)
                };
                p.list = Some(info.clone());
            }
            ListChange::Clear => {
                if p.list.is_some() {
                    p.list = None;
                    p.indents.left_pt = 0.0;
                    p.indents.first_line_pt = 0.0;
                }
            }
            ListChange::Separator(s) => {
                if let Some(l) = p.list.as_mut().filter(|l| !l.kind.is_bullet()) {
                    l.separator = *s;
                }
            }
            ListChange::Start(n) => {
                if let Some(l) = p.list.as_mut().filter(|l| !l.kind.is_bullet()) {
                    l.start = *n;
                }
            }
            ListChange::ListIndent(pt) => {
                if let Some(l) = p.list.as_mut() {
                    l.list_indent_pt = *pt;
                    let right = p.indents.right_pt;
                    p.indents = ParaIndents::for_list(*pt);
                    p.indents.right_pt = right;
                    if *pt == 0.0 {
                        p.indents.first_line_pt = 0.0;
                    }
                }
            }
            ListChange::StepIndent(delta) => {
                if let Some(l) = p.list.as_mut() {
                    let pt = (l.list_indent_pt + delta).max(0.0);
                    l.list_indent_pt = pt;
                    let right = p.indents.right_pt;
                    p.indents = ParaIndents::for_list(pt);
                    p.indents.right_pt = right;
                    if pt == 0.0 {
                        p.indents.first_line_pt = 0.0;
                    }
                } else {
                    p.indents.left_pt = (p.indents.left_pt + delta).max(0.0);
                }
            }
            ListChange::Indents {
                left_pt,
                first_line_pt,
                right_pt,
            } => {
                if let Some(v) = left_pt {
                    p.indents.left_pt = *v;
                }
                if let Some(v) = first_line_pt {
                    p.indents.first_line_pt = *v;
                }
                if let Some(v) = right_pt {
                    p.indents.right_pt = *v;
                }
            }
        }
        any |= p.list != old_list || p.indents != old_indents;
    }
    any
}

/// The label (bullet character or number text, spec 4R.5) each paragraph displays; `None`
/// for paragraphs not in a list.
pub fn list_labels(paragraphs: &[Paragraph]) -> Vec<Option<String>> {
    pub_format::list_labels_for(paragraphs.iter().map(|p| (p.list.as_ref(), false)))
}

/// The list and indents shared by every paragraph `range` touches; `None` where they differ.
pub fn selection_list(
    paragraphs: &[Paragraph],
    range: Range<usize>,
) -> Option<(Option<ListInfo>, ParaIndents)> {
    let spans = para_spans(paragraphs);
    let mut found: Option<(Option<ListInfo>, ParaIndents)> = None;
    for (p, &(ps, pe)) in paragraphs.iter().zip(&spans) {
        if !touches(&range, ps, pe) {
            continue;
        }
        let this = (p.list.clone(), p.indents);
        match &found {
            None => found = Some(this),
            Some(f) if *f != this => return None,
            _ => {}
        }
    }
    found
}

/// The formatting shared by a selection; `None` where the selected text differs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionFormat {
    pub font: Option<String>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub script: Option<crate::model::Script>,
    pub align: Option<Align>,
}

/// The formatting of the text in `range`. A caret reports the formatting of the character
/// before it (or after it at the start of a paragraph); an empty paragraph reports its own
/// style.
pub fn selection_format(paragraphs: &[Paragraph], range: Range<usize>) -> SelectionFormat {
    let spans = para_spans(paragraphs);
    let mut styles: Vec<Run> = Vec::new();
    let mut aligns: Vec<Align> = Vec::new();
    let caret = range.start >= range.end;
    for (p, &(ps, pe)) in paragraphs.iter().zip(&spans) {
        if !touches(&range, ps, pe) {
            continue;
        }
        aligns.push(p.align);
        let (lo, hi) = if caret {
            let c = range.start.saturating_sub(ps).min(pe - ps);
            (c.saturating_sub(1), (c.max(1)).min(pe - ps))
        } else {
            (range.start.max(ps) - ps, range.end.min(pe) - ps)
        };
        let mut at = 0;
        let mut found = false;
        for r in &p.runs {
            let n = r.text.chars().count();
            if n > 0 && at < hi && at + n > lo {
                styles.push(style_of(r));
                found = true;
            }
            at += n;
        }
        if !found {
            styles.push(style_of(&p.base_run()));
        }
    }
    let uniform = |f: &dyn Fn(&Run) -> bool| {
        let first = styles.first().map(f)?;
        styles.iter().all(|s| f(s) == first).then_some(first)
    };
    SelectionFormat {
        font: styles
            .first()
            .map(|s| s.font.clone())
            .filter(|f| styles.iter().all(|s| s.font == *f)),
        size: styles
            .first()
            .map(|s| s.size_pt)
            .filter(|f| styles.iter().all(|s| s.size_pt == *f)),
        bold: uniform(&|s| s.bold),
        italic: uniform(&|s| s.italic),
        underline: uniform(&|s| s.underline),
        script: styles
            .first()
            .map(|s| s.script)
            .filter(|f| styles.iter().all(|s| s.script == *f)),
        align: aligns
            .first()
            .copied()
            .filter(|a| aligns.iter().all(|b| b == a)),
    }
}

/// Turn edited plain text back into paragraphs, keeping all formatting that can be kept.
/// Unchanged leading and trailing paragraphs are kept as they were. In the changed middle
/// part every character that survives the edit (the common start and end of the old and
/// new text) keeps its run formatting; inserted characters take the formatting of the
/// character before them (of the one after them at the start of a paragraph), so typing
/// inherits the style at the caret. Paragraph alignment and spacing follow the old
/// paragraph the line came from; new lines inherit from the line they were split from.
pub fn rebuild_paragraphs(old: &[Paragraph], new_text: &str) -> Vec<Paragraph> {
    let lines: Vec<&str> = new_text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let old_texts: Vec<String> = old.iter().map(Paragraph::text).collect();

    let mut prefix = 0;
    while prefix < old.len() && prefix < lines.len() && old_texts[prefix] == lines[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len() - prefix
        && suffix < lines.len() - prefix
        && old_texts[old.len() - 1 - suffix] == lines[lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_mid = &old[prefix..old.len() - suffix];
    let new_mid = &lines[prefix..lines.len() - suffix];
    let mut out: Vec<Paragraph> = old[..prefix].to_vec();
    if !new_mid.is_empty() {
        // With no old paragraph in the middle, borrow the style of a neighbour.
        let neighbour;
        let src: &[Paragraph] = if old_mid.is_empty() {
            neighbour = [prefix
                .checked_sub(1)
                .and_then(|i| old.get(i))
                .or_else(|| old.get(old.len() - suffix))
                .or_else(|| old.last())
                .map_or_else(
                    || Paragraph::plain(""),
                    |p| Paragraph {
                        runs: vec![style_of(&p.base_run())],
                        align: p.align,
                        spacing: p.spacing,
                        list: p.list.clone(),
                        indents: p.indents,
                    },
                )];
            &neighbour
        } else {
            old_mid
        };
        out.extend(rebuild_middle(src, new_mid));
    }
    out.extend_from_slice(&old[old.len() - suffix..]);
    if out.is_empty() {
        out.push(Paragraph::plain(""));
    }
    out
}

/// The character-level part of [`rebuild_paragraphs`].
fn rebuild_middle(old: &[Paragraph], new_lines: &[&str]) -> Vec<Paragraph> {
    // Old text as chars with a style each; a break takes the style of the char before it.
    let mut old_chars: Vec<char> = Vec::new();
    let mut old_styles: Vec<Run> = Vec::new();
    for (i, p) in old.iter().enumerate() {
        if i > 0 {
            old_chars.push('\n');
            old_styles.push(
                old_styles
                    .last()
                    .cloned()
                    .unwrap_or_else(|| style_of(&p.base_run())),
            );
        }
        for r in &p.runs {
            let st = style_of(r);
            for ch in r.text.chars() {
                old_chars.push(ch);
                old_styles.push(st.clone());
            }
        }
    }
    let new_chars: Vec<char> = new_lines.join("\n").chars().collect();

    let mut p = 0;
    while p < old_chars.len() && p < new_chars.len() && old_chars[p] == new_chars[p] {
        p += 1;
    }
    let mut q = 0;
    while q < old_chars.len() - p
        && q < new_chars.len() - p
        && old_chars[old_chars.len() - 1 - q] == new_chars[new_chars.len() - 1 - q]
    {
        q += 1;
    }
    let fallback = old
        .first()
        .map_or_else(|| Run::plain(""), |p| style_of(&p.base_run()));
    let inserted = if p > 0 && old_chars[p - 1] != '\n' {
        old_styles[p - 1].clone()
    } else if p < old_chars.len() {
        old_styles[p].clone()
    } else {
        old_styles.last().cloned().unwrap_or(fallback)
    };
    // Typed text is never part of a field.
    let inserted = Run {
        field: None,
        ..inserted
    };
    let style_at = |i: usize| -> Run {
        if i < p {
            old_styles[i].clone()
        } else if i >= new_chars.len() - q {
            old_styles[old_chars.len() - (new_chars.len() - i)].clone()
        } else {
            inserted.clone()
        }
    };

    // Paragraph attributes of each new line.
    let count = |chars: &[char]| chars.iter().filter(|&&c| c == '\n').count();
    let l_start = count(&new_chars[..p]);
    let l_end_new = count(&new_chars[..new_chars.len() - q]);
    let l_end_old = count(&old_chars[..old_chars.len() - q]);
    let attrs = |i: usize| -> &Paragraph {
        let ix = if i <= l_start {
            i
        } else if i > l_end_new {
            i - l_end_new + l_end_old
        } else {
            l_start
        };
        &old[ix.min(old.len() - 1)]
    };

    let mut out = Vec::new();
    let mut at = 0;
    for (li, line) in new_lines.iter().enumerate() {
        let n = line.chars().count();
        let src = attrs(li);
        let mut runs: Vec<Run> = Vec::new();
        for (k, &ch) in new_chars.iter().enumerate().skip(at).take(n) {
            let st = style_at(k);
            match runs.last_mut() {
                Some(r) if same_style(r, &st) => r.text.push(ch),
                _ => runs.push(st.with_text(&ch.to_string())),
            }
        }
        if runs.is_empty() {
            runs.push(style_of(&src.base_run()));
        }
        out.push(Paragraph {
            runs,
            align: src.align,
            spacing: src.spacing,
            list: src.list.clone(),
            indents: src.indents,
        });
        at += n + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn para(text: &str, size: f32, bold: bool) -> Paragraph {
        let mut r = Run::plain(text);
        r.size_pt = size;
        r.bold = bold;
        Paragraph {
            runs: vec![r],
            align: Align::Center,
            spacing: Default::default(),
            list: None,
            indents: Default::default(),
        }
    }

    #[test]
    fn to_text_joins_lines() {
        let ps = vec![para("a", 10.0, false), para("b", 10.0, false)];
        assert_eq!(paragraphs_to_text(&ps), "a\nb");
    }

    #[test]
    fn untouched_paragraphs_keep_formatting() {
        let old = vec![para("Title", 24.0, true), para("Body", 10.0, false)];
        let same = rebuild_paragraphs(&old, "Title\nBody");
        assert_eq!(same, old);

        let edited = rebuild_paragraphs(&old, "Title\nBody text");
        assert_eq!(edited[0], old[0]);
        assert_eq!(edited[1].text(), "Body text");
        assert_eq!(edited[1].runs[0].size_pt, 10.0);
        assert_eq!(edited[1].align, Align::Center);
    }

    #[test]
    fn added_and_removed_lines() {
        let old = vec![para("Title", 24.0, true), para("Body", 10.0, false)];
        let more = rebuild_paragraphs(&old, "Title\nBody\nMore");
        assert_eq!(more.len(), 3);
        assert_eq!(more[2].runs[0].size_pt, 10.0);
        let fewer = rebuild_paragraphs(&old, "Title");
        assert_eq!(fewer, vec![old[0].clone()]);
        let inserted = rebuild_paragraphs(&old, "Title\nNew\nBody");
        assert_eq!(inserted[1].text(), "New");
        assert_eq!(inserted[2], old[1]);
        let empty = rebuild_paragraphs(&old, "");
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].text(), "");
        let from_nothing = rebuild_paragraphs(&[], "hi");
        assert_eq!(from_nothing[0].text(), "hi");
    }

    fn mixed() -> Vec<Paragraph> {
        let mut bold = Run::plain("bold");
        bold.bold = true;
        bold.size_pt = 20.0;
        vec![
            Paragraph {
                runs: vec![Run::plain("plain "), bold, Run::plain(" tail")],
                align: Align::Center,
                spacing: Default::default(),
                list: None,
                indents: Default::default(),
            },
            Paragraph::plain("Second"),
        ]
    }

    #[test]
    fn selection_format_reports_uniform_and_mixed() {
        let ps = mixed();
        // "bold" is chars 6..10 of the first paragraph.
        let f = selection_format(&ps, 6..10);
        assert_eq!(f.bold, Some(true));
        assert_eq!(f.size, Some(20.0));
        assert_eq!(f.align, Some(Align::Center));
        // Part plain, part bold: mixed.
        let f = selection_format(&ps, 3..8);
        assert_eq!(f.bold, None);
        assert_eq!(f.size, None);
        assert_eq!(f.font.as_deref(), Some("Calibri"));
        assert_eq!(f.italic, Some(false));
        // Across both paragraphs: bold and alignment are mixed.
        let f = selection_format(&ps, 12..20);
        assert_eq!(f.align, None);
        // A caret reports the character before it.
        assert_eq!(selection_format(&ps, 8..8).bold, Some(true));
        assert_eq!(selection_format(&ps, 11..11).bold, Some(false));
        // Start of the text, and an empty paragraph.
        assert_eq!(selection_format(&ps, 0..0).bold, Some(false));
        let empty = vec![Paragraph::plain("")];
        assert_eq!(selection_format(&empty, 0..0).size, Some(12.0));
    }

    #[test]
    fn typing_inherits_formatting_and_keeps_runs() {
        let old = mixed();
        // Insert into the middle of the bold run.
        let new = rebuild_paragraphs(&old, "plain boXld tail\nSecond");
        assert_eq!(new[0].runs.len(), 3);
        assert_eq!(new[0].runs[1].text, "boXld");
        assert!(new[0].runs[1].bold);
        assert_eq!(new[0].runs[0].text, "plain ");
        assert_eq!(new[0].align, Align::Center);
        assert_eq!(new[1], old[1]);
        // Append at the end of the first paragraph: inherits the last (plain) run.
        let new = rebuild_paragraphs(&old, "plain bold tail!\nSecond");
        assert_eq!(new[0].runs.last().unwrap().text, " tail!");
        // Delete the bold word: its neighbours merge into one plain run.
        let new = rebuild_paragraphs(&old, "plain  tail\nSecond");
        assert_eq!(new[0].runs.len(), 1);
        assert_eq!(new[0].runs[0].text, "plain  tail");
        // Type at the start of the text, and at the start of a paragraph.
        let new = rebuild_paragraphs(&old, "<plain bold tail\nSecond");
        assert_eq!(new[0].runs[0].text, "<plain ");
        // Split a paragraph with Enter inside the bold run: both halves stay bold.
        let new = rebuild_paragraphs(&old, "plain bo\nld tail\nSecond");
        assert_eq!(new.len(), 3);
        assert!(new[0].runs.last().unwrap().bold && new[1].runs[0].bold);
        assert_eq!(new[0].align, Align::Center);
        assert_eq!(new[1].align, Align::Center);
        assert_eq!(new[2], old[1]);
        // Join the two paragraphs by deleting the break.
        let new = rebuild_paragraphs(&old, "plain bold tailSecond");
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].text(), "plain bold tailSecond");
        assert_eq!(new[0].runs[1].text, "bold");
    }

    #[test]
    fn normalize_merges_and_canonical_ignores_run_splits() {
        let a = Paragraph {
            runs: vec![Run::plain("ab"), Run::plain(""), Run::plain("cd")],
            align: Align::Left,
            spacing: Default::default(),
            list: None,
            indents: Default::default(),
        };
        let b = Paragraph::plain("abcd");
        assert_ne!(a, b);
        assert_eq!(canonical(&[a]), canonical(&[b]));
    }
}
