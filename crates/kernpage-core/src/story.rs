//! Linked stories (docs/design/LINKED-STORIES.md, phase 1): cutting the whole text of a
//! story into the share of one frame (`slice`) and putting an edited share back
//! (`splice`), including shares that start or end in the middle of a paragraph.
//!
//! Positions are [`StoryPos`]: (paragraph index, glyph offset). A glyph is a character
//! that is not `\r`, `\n` or a control character (a tab counts), the same definition as
//! `pub2pdf::chain_breaks`; a line break (`\n` in a run) is therefore never counted.

use crate::model::*;

/// The one empty paragraph a linked frame's copy starts with: the first paragraph of the
/// text box `kind` with its text cleared (the formatting of what is typed next).
pub fn empty_like(kind: &ShapeKind) -> Paragraph {
    match kind {
        ShapeKind::TextBox { paragraphs } if !paragraphs.is_empty() => {
            make(&paragraphs[0], vec![paragraphs[0].base_run().with_text("")])
        }
        _ => Paragraph::plain(""),
    }
}

pub(crate) fn counts_as_glyph(ch: char) -> bool {
    !matches!(ch, '\r' | '\n') && (ch == '\t' || !ch.is_control())
}

fn same_format(a: &Run, b: &Run) -> bool {
    a.font == b.font
        && a.size_pt == b.size_pt
        && a.bold == b.bold
        && a.italic == b.italic
        && a.underline == b.underline
        && a.script == b.script
        && a.color == b.color
}

/// A paragraph with runs `runs` and the attributes of `like`. Empty runs are dropped; a
/// paragraph with no text keeps one empty run carrying the first run's formatting.
fn make(like: &Paragraph, mut runs: Vec<Run>) -> Paragraph {
    if runs.iter().any(|r| !r.text.is_empty()) {
        runs.retain(|r| !r.text.is_empty());
    } else {
        runs.truncate(1);
        if runs.is_empty() {
            runs.push(like.base_run().with_text(""));
        }
    }
    Paragraph {
        runs,
        align: like.align,
        spacing: like.spacing,
        list: like.list.clone(),
        indents: like.indents,
    }
}

/// `p` cut after `at` glyphs: (the first `at` glyphs, the rest).
fn split_at(p: &Paragraph, at: usize) -> (Paragraph, Paragraph) {
    let mut index = 0;
    let (mut head, mut rest) = (Vec::new(), Vec::new());
    for r in &p.runs {
        let (mut h, mut t) = (String::new(), String::new());
        for ch in r.text.chars() {
            if index < at {
                h.push(ch);
            } else {
                t.push(ch);
            }
            if counts_as_glyph(ch) {
                index += 1;
            }
        }
        if !h.is_empty() {
            head.push(r.with_text(&h));
        }
        if !t.is_empty() {
            rest.push(r.with_text(&t));
        }
    }
    (make(p, head), make(p, rest))
}

/// The part of `p` between glyph offsets `from` (inclusive) and `to` (exclusive).
fn cut(p: &Paragraph, from: usize, to: usize) -> Paragraph {
    let rest = if from > 0 { split_at(p, from).1 } else { p.clone() };
    if to == usize::MAX {
        rest
    } else {
        split_at(&rest, to.saturating_sub(from)).0
    }
}

/// `a` followed by `b` as one paragraph (`a`'s alignment and spacing win); runs of equal
/// formatting that meet at the seam become one run.
fn join(a: Paragraph, b: &Paragraph) -> Paragraph {
    let mut runs = a.runs.clone();
    if runs.iter().all(|r| r.text.is_empty()) {
        runs.clear();
    }
    for r in b.runs.iter().filter(|r| !r.text.is_empty()) {
        match runs.last_mut() {
            Some(last) if same_format(last, r) => last.text.push_str(&r.text),
            _ => runs.push(r.clone()),
        }
    }
    make(&a, runs)
}

fn has_text(p: &Paragraph) -> bool {
    p.runs.iter().any(|r| !r.text.is_empty())
}

/// An empty frame is one empty paragraph in the model; as a share it is "nothing".
fn normalized(share: &[Paragraph]) -> &[Paragraph] {
    match share {
        [only] if !has_text(only) => &[],
        _ => share,
    }
}

/// The paragraphs of a story from `start` up to `end` (exclusive). A share that starts or
/// ends in the middle of a paragraph begins or ends with that part of it.
pub fn slice_range(paras: &[Paragraph], start: StoryPos, end: StoryPos) -> Vec<Paragraph> {
    let last = if end.1 > 0 { end.0 + 1 } else { end.0 };
    (start.0..last.min(paras.len()))
        .map(|k| {
            let from = if k == start.0 { start.1 } else { 0 };
            let to = if k == end.0 { end.1 } else { usize::MAX };
            cut(&paras[k], from, to)
        })
        .collect()
}

/// `paras` with the range `start..end` replaced by `new`.
///
/// If `start` is in the middle of a paragraph, the part of it before `start` joins the
/// first new paragraph (the paragraph keeps its attributes); if `end` is in the middle of
/// one, the part after `end` joins the last new paragraph (the new paragraph's attributes
/// win). With no new paragraphs those two parts join each other. Splicing back
/// `slice_range(paras, start, end)` gives `paras` again.
pub fn splice_range(
    paras: &[Paragraph],
    start: StoryPos,
    end: StoryPos,
    new: &[Paragraph],
) -> Vec<Paragraph> {
    let n = paras.len();
    let s0 = start.0.min(n);
    let mut out = paras[..s0].to_vec();
    let head = (start.1 > 0 && s0 < n).then(|| split_at(&paras[s0], start.1).0);
    let (rest, after) = if end.1 > 0 && end.0 < n {
        (Some(split_at(&paras[end.0], end.1).1), end.0 + 1)
    } else {
        (None, end.0.min(n))
    };
    let mut mid = new.to_vec();
    if let Some(h) = head {
        match mid.first_mut() {
            Some(first) => *first = join(h, first),
            None => mid.push(h),
        }
    }
    if let Some(r) = rest.filter(has_text) {
        match mid.last_mut() {
            Some(last) => *last = join(last.clone(), &r),
            None => mid.push(r),
        }
    }
    out.extend(mid);
    out.extend_from_slice(&paras[after.max(s0)..]);
    out
}

fn glyph_count(p: &Paragraph) -> usize {
    p.text().chars().filter(|c| counts_as_glyph(*c)).count()
}

/// The story position of the caret at character `caret` of the text of a frame that starts
/// at `start` (the frame's paragraphs joined by `\n`, as the editor shows them).
pub fn caret_to_pos(start: StoryPos, text: &str, caret: usize) -> StoryPos {
    let (mut line, mut glyphs) = (0, 0);
    for ch in text.chars().take(caret) {
        if ch == '\n' {
            line += 1;
            glyphs = 0;
        } else if counts_as_glyph(ch) {
            glyphs += 1;
        }
    }
    (start.0 + line, glyphs + if line == 0 { start.1 } else { 0 })
}

/// The character offset in a frame's text (it starts at `start`) of story position `pos`;
/// clamped to the text.
pub fn pos_to_caret(start: StoryPos, text: &str, pos: StoryPos) -> usize {
    let line = pos.0.saturating_sub(start.0);
    let col = pos.1.saturating_sub(if line == 0 { start.1 } else { 0 });
    let mut at = 0;
    for (k, l) in text.split('\n').enumerate() {
        if k == line {
            let mut glyphs = 0;
            for ch in l.chars() {
                if glyphs >= col {
                    break;
                }
                if counts_as_glyph(ch) {
                    glyphs += 1;
                }
                at += 1;
            }
            return at;
        }
        at += l.chars().count() + 1;
    }
    text.chars().count()
}

/// `paras` without the glyph at `pos` (or with paragraph `pos.0` and the next one joined,
/// when `pos` is at the end of its paragraph: the paragraph break is deleted). `None` at
/// the very end of the story.
pub fn delete_at(paras: &[Paragraph], pos: StoryPos) -> Option<Vec<Paragraph>> {
    let p = paras.get(pos.0)?;
    let mut out = paras.to_vec();
    if pos.1 < glyph_count(p) {
        let (head, rest) = split_at(p, pos.1);
        let (_, tail) = split_at(&rest, 1);
        out[pos.0] = join(head, &tail);
    } else if pos.0 + 1 < paras.len() {
        out[pos.0] = join(p.clone(), &paras[pos.0 + 1]);
        out.remove(pos.0 + 1);
    } else {
        return None;
    }
    Some(out)
}

/// `paras` without the glyph before `pos` (or the paragraph break before it when `pos` is
/// at the start of a paragraph), and where `pos` is afterwards. `None` at the very start.
pub fn delete_before(paras: &[Paragraph], pos: StoryPos) -> Option<(Vec<Paragraph>, StoryPos)> {
    if pos.1 > 0 {
        let at = (pos.0, pos.1 - 1);
        Some((delete_at(paras, at)?, at))
    } else if pos.0 > 0 && pos.0 <= paras.len() {
        let at = (pos.0 - 1, glyph_count(&paras[pos.0 - 1]));
        Some((delete_at(paras, at)?, at))
    } else {
        None
    }
}

impl Story {
    /// The frame (searching from frame `from` on) the caret at story position `pos` is in.
    /// A frame owns the positions up to its end; the end itself belongs to the next frame
    /// when it is the start of a paragraph (a frame that ends at a paragraph boundary shows
    /// the end of the last paragraph, not the start of the next). A position past the last
    /// frame (in the overflow) belongs to the last frame.
    pub fn frame_for(&self, pos: StoryPos, from: usize) -> usize {
        let last = self.frames.len().saturating_sub(1);
        (from..self.frames.len())
            .find(|&j| {
                let end = self.end_of(j);
                pos < end || (pos == end && pos.1 > 0)
            })
            .unwrap_or(last)
    }

    /// Where frame `index` starts.
    pub fn start_of(&self, index: usize) -> StoryPos {
        if index == 0 {
            (0, 0)
        } else {
            self.end_of(index - 1)
        }
    }

    /// Where frame `index` ends (the end of the story for a frame with no recorded end).
    pub fn end_of(&self, index: usize) -> StoryPos {
        self.ends
            .get(index)
            .copied()
            .unwrap_or((self.paragraphs.len(), 0))
    }

    /// The position of `frame` in the chain.
    pub fn frame_index(&self, frame: ShapeId) -> Option<usize> {
        self.frames.iter().position(|&f| f == frame)
    }

    /// The paragraphs frame `index` shows (what its `TextBox` caches).
    pub fn slice(&self, index: usize) -> Vec<Paragraph> {
        slice_range(&self.paragraphs, self.start_of(index), self.end_of(index))
    }

    /// The story's paragraphs with frame `index`'s share replaced by `new` (the story
    /// itself is not changed: ends are only re-computed by a layout).
    pub fn splice(&self, index: usize, new: &[Paragraph]) -> Vec<Paragraph> {
        splice_range(
            &self.paragraphs,
            self.start_of(index),
            self.end_of(index),
            new,
        )
    }

    /// What frame `index`'s text box caches: its share, or one empty paragraph when the
    /// share is empty (a text box always has a paragraph).
    pub fn cache(&self, index: usize) -> Vec<Paragraph> {
        let share = self.slice(index);
        if share.is_empty() {
            vec![Paragraph::plain("")]
        } else {
            share
        }
    }

    /// The overflow: the story after the last frame.
    pub fn overflow(&self) -> Vec<Paragraph> {
        slice_range(
            &self.paragraphs,
            self.end_of(self.frames.len().saturating_sub(1)),
            (self.paragraphs.len(), 0),
        )
    }

    /// The whole story with the current text of every frame put back. `current` gives a
    /// frame's paragraphs (`None` for a frame that no longer exists: it contributes
    /// nothing). Frames whose text equals their share are left alone, so an unedited story
    /// comes back identical.
    pub fn compose<'a>(&self, current: impl Fn(ShapeId) -> Option<&'a [Paragraph]>) -> Vec<Paragraph> {
        let mut out = self.paragraphs.clone();
        // Last frame first: a change only moves what comes after it, so the positions of
        // the earlier frames (which are in story coordinates) stay valid.
        for (i, &id) in self.frames.iter().enumerate().rev() {
            let now = normalized(current(id).unwrap_or(&[]));
            let share = self.slice(i);
            if now == normalized(&share) {
                continue;
            }
            let start = self.start_of(i);
            out = splice_range(&out, start, self.end_of(i), now);
            // A frame that starts in the middle of a paragraph shows that paragraph's
            // attributes on its first paragraph: changing them changes the paragraph.
            if let (true, Some(n), Some(old)) = (start.1 > 0, now.first(), normalized(&share).first()) {
                if (n.align, n.spacing, &n.list, n.indents)
                    != (old.align, old.spacing, &old.list, old.indents)
                {
                    if let Some(p) = out.get_mut(start.0) {
                        p.align = n.align;
                        p.spacing = n.spacing;
                        p.list = n.list.clone();
                        p.indents = n.indents;
                    }
                }
            }
        }
        out
    }
}

impl Document {
    /// The whole text of `story`. When every frame shows exactly its share (what every
    /// command leaves, see `Editor::execute`) the story is the truth and is returned as it
    /// is; otherwise the frames' current paragraphs are spliced into it (documents edited
    /// without re-flow, or a story with a deleted frame).
    pub fn story_text(&self, story: &Story) -> Vec<Paragraph> {
        if self.frames_match(story) == Some(true) {
            return story.paragraphs.clone();
        }
        story.compose(|id| {
            self.pages
                .iter()
                .flat_map(|p| p.shapes.iter())
                .find(|s| s.id == id)
                .and_then(|s| match &s.kind {
                    ShapeKind::TextBox { paragraphs } => Some(paragraphs.as_slice()),
                    _ => None,
                })
        })
    }

    /// Whether every frame of `story` shows exactly its share (an empty share and one
    /// empty paragraph count as equal); `None` if a frame no longer exists.
    fn frames_match(&self, story: &Story) -> Option<bool> {
        let mut all = true;
        for (i, &id) in story.frames.iter().enumerate() {
            let (page, ix) = locate(self, id)?;
            all &= match &self.pages[page].shapes[ix].kind {
                ShapeKind::TextBox { paragraphs } => {
                    normalized(paragraphs) == normalized(&story.slice(i))
                }
                _ => false,
            };
        }
        Some(all)
    }
}

/// Where frame `id` is: (page index, index in the page's shape list).
pub(crate) fn locate(doc: &Document, id: ShapeId) -> Option<(usize, usize)> {
    doc.pages
        .iter()
        .enumerate()
        .find_map(|(p, page)| page.shapes.iter().position(|s| s.id == id).map(|i| (p, i)))
}

/// One shared font/metrics state for every re-flow (loading fonts takes longer than the
/// layout itself). `None` until first used.
pub(crate) static FLOW: std::sync::Mutex<Option<pub2pdf::FlowContext>> = std::sync::Mutex::new(None);

/// The frame ends of `paragraphs` flowed through `frames`, with the same layout as PDF
/// export.
fn flow_ends(frames: &[pub2pdf::FrameGeom], paragraphs: &[Paragraph]) -> Vec<StoryPos> {
    let pf_paras = crate::export::flow_paragraphs(paragraphs);
    let mut guard = FLOW.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = guard.get_or_insert_with(|| pub2pdf::FlowContext::new(&pub2pdf::Options::default()));
    ctx.flow(frames, &pf_paras)
}

/// The outcome of re-flowing a story: the new story and the new cached paragraphs of its
/// frames as (page, frame id, paragraphs).
pub(crate) type Reflowed = (Story, Vec<(usize, ShapeId, Vec<Paragraph>)>);

impl Document {
    /// Re-flow story `index`: take the current text of its frames into the story (the
    /// frame caches are where edits land), lay it out through the frames again and cut it
    /// into new shares. `None` if the story does not exist or one of its frames is gone
    /// (or is not a text box), in which case nothing can be laid out.
    pub(crate) fn reflow_story(&self, index: usize) -> Option<Reflowed> {
        let story = self.stories.get(index)?;
        // Check the frames first: `story_text` of a story with a missing frame is not the
        // story.
        for &id in &story.frames {
            locate(self, id)?;
        }
        let paragraphs = self.story_text(story);
        self.reflow_value(story, paragraphs)
    }

    /// Lay `paragraphs` out through the frames of `story` (all of which must exist and be
    /// text boxes) and cut them into shares: the story as it would be after a re-flow,
    /// and its frames' new caches as (page, frame id, paragraphs).
    pub(crate) fn reflow_value(&self, story: &Story, paragraphs: Vec<Paragraph>) -> Option<Reflowed> {
        let mut at = Vec::with_capacity(story.frames.len());
        for &id in &story.frames {
            let (page, ix) = locate(self, id)?;
            if !self.pages[page].shapes[ix].is_text_box() {
                return None;
            }
            at.push((page, ix));
        }
        let frames = crate::export::flow_geometry(self, &at)?;
        let ends = flow_ends(&frames, &paragraphs);
        let mut new = Story {
            id: story.id,
            frames: story.frames.clone(),
            paragraphs,
            ends,
        };
        // Frames whose layout reports fewer ends than frames cannot happen; guard anyway.
        new.ends.resize(new.frames.len(), (new.paragraphs.len(), 0));
        let caches = (0..new.frames.len())
            .map(|i| (at[i].0, new.frames[i], new.cache(i)))
            .collect();
        Some((new, caches))
    }

    /// Whether `paragraphs` do not fit the text box `id` on `page` when it is on its own
    /// (a text box that is not part of a chain): the text after its end has any glyph.
    /// `false` if the shape is not a text box.
    pub fn text_overflows(&self, page: usize, id: ShapeId, paragraphs: &[Paragraph]) -> bool {
        let Some(ix) = self.pages.get(page).and_then(|p| p.shapes.iter().position(|s| s.id == id))
        else {
            return false;
        };
        if !self.pages[page].shapes[ix].is_text_box() {
            return false;
        }
        let Some(frames) = crate::export::flow_geometry(self, &[(page, ix)]) else {
            return false;
        };
        let Some(&end) = flow_ends(&frames, paragraphs).first() else {
            return false;
        };
        slice_range(paragraphs, end, (paragraphs.len(), 0)).iter().any(has_text)
    }

    /// Whether the text box `id` on `page` shows an overflow indicator: the last frame of
    /// a chain whose story has text left after it, or a standalone box whose text does
    /// not fit (`paragraphs` is its text, which may be newer than the model's).
    pub fn frame_overflows(&self, page: usize, id: ShapeId, paragraphs: &[Paragraph]) -> bool {
        match self.story_of(id) {
            Some(story) => {
                story.frames.last() == Some(&id) && story.overflow().iter().any(has_text)
            }
            None => self.text_overflows(page, id, paragraphs),
        }
    }

    /// (has a previous link, has a next link) of a frame; both false for a text box that
    /// is not linked to another one.
    pub fn frame_links(&self, id: ShapeId) -> (bool, bool) {
        match self.story_of(id) {
            Some(s) => match s.frame_index(id) {
                Some(i) if s.frames.len() > 1 => (i > 0, i + 1 < s.frames.len()),
                _ => (false, false),
            },
            None => (false, false),
        }
    }

    /// Do all frames of every story show exactly their share of it? A story with a frame
    /// that no longer exists is not checked (deleting frames is not handled yet: its text
    /// lives on in the others).
    pub fn story_caches_consistent(&self) -> bool {
        self.stories.iter().all(|s| self.frames_match(s).unwrap_or(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, bold: bool) -> Run {
        Run {
            bold,
            ..Run::plain(text)
        }
    }

    fn para(runs: &[(&str, bool)]) -> Paragraph {
        Paragraph {
            runs: runs.iter().map(|(t, b)| run(t, *b)).collect(),
            align: Align::Left,
            spacing: ParaSpacing::default(),
            list: None,
            indents: Default::default(),
        }
    }

    fn texts(p: &[Paragraph]) -> Vec<String> {
        p.iter().map(Paragraph::text).collect()
    }

    /// "alpha beta", "gamma" (bold "gam", plain "ma"), "", "delta\nline two"
    fn story() -> Vec<Paragraph> {
        vec![
            para(&[("alpha beta", false)]),
            para(&[("gam", true), ("ma", false)]),
            para(&[("", false)]),
            para(&[("delta\nline two", false)]),
        ]
    }

    fn all_positions(p: &[Paragraph]) -> Vec<StoryPos> {
        let mut v = Vec::new();
        for (k, para) in p.iter().enumerate() {
            let glyphs = para.text().chars().filter(|c| counts_as_glyph(*c)).count();
            for g in 0..=glyphs {
                v.push((k, g));
            }
        }
        v.push((p.len(), 0));
        v
    }

    #[test]
    fn slice_of_whole_ranges_and_paragraph_boundaries() {
        let s = story();
        assert_eq!(slice_range(&s, (0, 0), (s.len(), 0)), s);
        assert_eq!(texts(&slice_range(&s, (1, 0), (3, 0))), ["gamma", ""]);
        assert!(slice_range(&s, (2, 0), (2, 0)).is_empty());
    }

    #[test]
    fn slice_starting_and_ending_mid_paragraph_keeps_formatting() {
        let s = story();
        let part = slice_range(&s, (0, 6), (1, 4));
        assert_eq!(texts(&part), ["beta", "gamm"]);
        assert_eq!(part[1].runs.len(), 2);
        assert!(part[1].runs[0].bold && part[1].runs[0].text == "gam");
        assert!(!part[1].runs[1].bold && part[1].runs[1].text == "m");
        // Inside one paragraph.
        assert_eq!(texts(&slice_range(&s, (0, 2), (0, 7))), ["pha b"]);
        // A line break is not a glyph: offset 5 of "delta\nline two" is before the break.
        assert_eq!(texts(&slice_range(&s, (3, 0), (3, 5))), ["delta"]);
        assert_eq!(texts(&slice_range(&s, (3, 5), (3, 9))), ["\nline"]);
    }

    #[test]
    fn splicing_the_unchanged_slice_returns_the_identical_story() {
        let s = story();
        let pos = all_positions(&s);
        for &a in &pos {
            for &b in &pos {
                if b < a {
                    continue;
                }
                let part = slice_range(&s, a, b);
                assert_eq!(splice_range(&s, a, b, &part), s, "range {a:?}..{b:?}");
            }
        }
    }

    #[test]
    fn edit_at_the_start_of_a_slice_that_starts_mid_paragraph() {
        let s = story();
        let (a, b) = ((0, 6), (1, 4));
        let mut part = slice_range(&s, a, b);
        part[0].runs[0].text = format!("NEW {}", part[0].runs[0].text);
        let out = splice_range(&s, a, b, &part);
        assert_eq!(texts(&out), ["alpha NEW beta", "gamma", "", "delta
line two"]);
    }

    #[test]
    fn typing_into_a_slice_in_the_middle_end_and_with_new_paragraphs() {
        let s = story();
        let (a, b) = ((0, 6), (1, 4));
        // Middle: "be|ta" -> "beXta".
        let mut part = slice_range(&s, a, b);
        part[0].runs[0].text = "beXta".into();
        let out = splice_range(&s, a, b, &part);
        assert_eq!(texts(&out), ["alpha beXta", "gamma", "", "delta
line two"]);
        // End: append to the last paragraph of the slice; the rest "a" still follows.
        let mut part = slice_range(&s, a, b);
        part[1].runs.last_mut().unwrap().text.push('!');
        let out = splice_range(&s, a, b, &part);
        assert_eq!(texts(&out)[1], "gamm!a");
        // A new paragraph typed between the two share paragraphs.
        let mut part = slice_range(&s, a, b);
        part.insert(1, para(&[("one more", false)]));
        let out = splice_range(&s, a, b, &part);
        assert_eq!(texts(&out), ["alpha beta", "one more", "gamma", "", "delta
line two"]);
    }

    #[test]
    fn deleting_the_whole_slice_joins_the_two_halves() {
        let s = story();
        let out = splice_range(&s, (0, 6), (1, 4), &[]);
        assert_eq!(texts(&out), ["alpha a", "", "delta\nline two"]);
        // Boundaries on paragraph ends: the paragraphs just go away.
        let out = splice_range(&s, (1, 0), (3, 0), &[]);
        assert_eq!(texts(&out), ["alpha beta", "delta\nline two"]);
        // Empty range with new text inserts it.
        let out = splice_range(&s, (2, 0), (2, 0), &[para(&[("ins", false)])]);
        assert_eq!(texts(&out), ["alpha beta", "gamma", "ins", "", "delta\nline two"]);
    }

    #[test]
    fn inserting_into_an_empty_range_in_the_middle_of_a_paragraph() {
        let s = story();
        let out = splice_range(&s, (0, 5), (0, 5), &[para(&[("X", false)])]);
        assert_eq!(texts(&out)[0], "alphaX beta");
        let out = splice_range(&s, (0, 5), (0, 5), &[]);
        assert_eq!(out, s);
    }

    #[test]
    fn formatting_runs_of_the_edited_share_are_kept() {
        let s = story();
        let (a, b) = ((1, 1), (3, 0));
        let mut part = slice_range(&s, a, b);
        assert_eq!(texts(&part), ["amma", ""]);
        part[0].runs = vec![run("MM", true)];
        let out = splice_range(&s, a, b, &part);
        // "g" (bold, before the cut) merges with the bold "MM" into one run.
        assert_eq!(texts(&out), ["alpha beta", "gMM", "", "delta\nline two"]);
        assert_eq!(out[1].runs.len(), 1);
        assert!(out[1].runs[0].bold);
    }

    fn three_frames() -> Story {
        Story {
            id: 1,
            frames: vec![10, 11, 12],
            paragraphs: story(),
            // frame 0: "alpha beta" + "gam"; frame 1: "ma" + "" ; frame 2: "delta", overflow rest.
            ends: vec![(1, 3), (3, 0), (3, 5)],
        }
    }

    #[test]
    fn story_slices_and_overflow() {
        let st = three_frames();
        assert_eq!(texts(&st.slice(0)), ["alpha beta", "gam"]);
        assert_eq!(texts(&st.slice(1)), ["ma", ""]);
        assert_eq!(texts(&st.slice(2)), ["delta"]);
        assert_eq!(texts(&st.overflow()), ["\nline two"]);
        assert_eq!(st.frame_index(11), Some(1));
        assert_eq!(st.frame_index(99), None);
    }

    #[test]
    fn compose_puts_every_edited_frame_back_and_leaves_the_rest() {
        let st = three_frames();
        let mut shares: std::collections::HashMap<ShapeId, Vec<Paragraph>> =
            (0..3).map(|i| (10 + i as ShapeId, st.slice(i))).collect();
        assert_eq!(st.compose(|id| shares.get(&id).map(|v| v.as_slice())), st.paragraphs);
        // Edit frames 0 and 2 at once (different lengths): both land, the tail follows.
        shares.get_mut(&10).unwrap()[0].runs[0].text = "ALPHA beta".into();
        shares.get_mut(&10).unwrap()[1].runs[0].text.push('+');
        shares.get_mut(&12).unwrap()[0].runs[0].text = "delta, much longer".into();
        let out = st.compose(|id| shares.get(&id).map(|v| v.as_slice()));
        assert_eq!(
            texts(&out),
            ["ALPHA beta", "gam+ma", "", "delta, much longer
line two"]
        );
        // A missing frame contributes nothing.
        let out = st.compose(|id| if id == 11 { None } else { shares.get(&id).map(|v| v.as_slice()) });
        assert_eq!(texts(&out), ["ALPHA beta", "gam+", "delta, much longer
line two"]);
    }

    #[test]
    fn caret_and_story_position_convert_both_ways() {
        // A frame starting at (1, 3) showing "ma" and "" and "xyz".
        let text = "ma\n\nxyz";
        let start = (1, 3);
        let cases = [(0, (1, 3)), (2, (1, 5)), (3, (2, 0)), (4, (3, 0)), (7, (3, 3))];
        for (caret, pos) in cases {
            assert_eq!(caret_to_pos(start, text, caret), pos, "caret {caret}");
            assert_eq!(pos_to_caret(start, text, pos), caret, "pos {pos:?}");
        }
        // Past the end clamps.
        assert_eq!(pos_to_caret(start, text, (9, 0)), 7);
        assert_eq!(pos_to_caret(start, text, (1, 99)), 2);
    }

    #[test]
    fn deleting_a_glyph_or_a_paragraph_break_around_a_position() {
        let s = story();
        let (out, at) = delete_before(&s, (0, 6)).unwrap();
        assert_eq!((texts(&out)[0].as_str(), at), ("alphabeta", (0, 5)));
        // Start of a paragraph: the break goes, the two paragraphs join.
        let (out, at) = delete_before(&s, (1, 0)).unwrap();
        assert_eq!((texts(&out)[..2].to_vec(), at), (vec!["alpha betagamma".to_owned(), String::new()], (0, 10)));
        assert!(delete_before(&s, (0, 0)).is_none());
        // Forward: a glyph, then at a paragraph end the break.
        assert_eq!(texts(&delete_at(&s, (0, 0)).unwrap())[0], "lpha beta");
        assert_eq!(texts(&delete_at(&s, (0, 10)).unwrap())[0], "alpha betagamma");
        // The bold run survives joining.
        assert!(delete_at(&s, (0, 10)).unwrap()[0].runs.iter().any(|r| r.bold));
        assert!(delete_at(&s, (3, 14)).is_none());
    }

    #[test]
    fn frame_for_a_caret_position() {
        let st = three_frames();
        // Frame 0 ends mid paragraph 1: its end belongs to it, the next position to frame 1.
        assert_eq!(st.frame_for((1, 3), 0), 0);
        assert_eq!(st.frame_for((1, 4), 0), 1);
        // Frame 1 ends at a paragraph start: that position is the start of frame 2's text.
        assert_eq!(st.frame_for((3, 0), 1), 2);
        assert_eq!(st.frame_for((2, 0), 1), 1);
        // In the overflow: the last frame.
        assert_eq!(st.frame_for((3, 9), 0), 2);
    }

    #[test]
    fn an_empty_frame_is_one_empty_paragraph_in_the_model() {
        let st = Story {
            id: 1,
            frames: vec![1, 2],
            paragraphs: story(),
            ends: vec![(4, 0), (4, 0)],
        };
        let blank = [Paragraph::plain("")];
        let first = st.slice(0);
        let out = st.compose(|id| Some(if id == 1 { first.as_slice() } else { &blank[..] }));
        assert_eq!(out, st.paragraphs);
        // Typing into the empty frame appends to the story.
        let typed = [Paragraph::plain("typed")];
        let out = st.compose(|id| Some(if id == 1 { first.as_slice() } else { &typed[..] }));
        assert_eq!(texts(&out).last().unwrap(), "typed");
    }
}
