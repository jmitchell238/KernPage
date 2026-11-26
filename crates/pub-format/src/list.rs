//! Bullets and numbering (spec 4R): the list descriptor of a paragraph, the paragraph
//! indents, and the label (bullet or number text) each paragraph of a story displays.
//! A list is only paragraph properties; the label is never stored in the text.

use crate::quill::Paragraph;
use serde::{Deserialize, Serialize};

/// Default list indent, points (spec 4R.1: `0x206` absent).
pub const DEFAULT_LIST_INDENT_PT: f32 = 18.0;
/// Default bullet size, points (`SetListType(23)` writes 127,000 EMU).
pub const DEFAULT_BULLET_SIZE_PT: f32 = 10.0;
/// The default bullet: U+00B7 in Symbol (spec 4R.4).
pub const DEFAULT_BULLET_CHAR: u32 = 0xB7;
pub const DEFAULT_BULLET_FONT: &str = "Symbol";
/// Separator index "plain" (`1`, no mark); `0x258` is absent for it (spec 4R.3).
pub const SEPARATOR_PLAIN: u8 = 3;
/// Separator index of `1.`, what applying a numbered list writes.
pub const SEPARATOR_DOT: u8 = 2;

/// The `ParagraphFormat.ListType` values Publisher accepts (spec 4R.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListType {
    Arabic,
    UpperRoman,
    LowerRoman,
    UpperLetter,
    LowerLetter,
    Ordinal,
    CardinalText,
    OrdinalText,
    LeadingZero,
    Bullet,
    /// A value outside the table (never written by Publisher here); numbered as Arabic.
    Other(u32),
}

impl ListType {
    pub fn from_code(c: u32) -> Self {
        match c {
            0 => Self::Arabic,
            1 => Self::UpperRoman,
            2 => Self::LowerRoman,
            3 => Self::UpperLetter,
            4 => Self::LowerLetter,
            5 => Self::Ordinal,
            6 => Self::CardinalText,
            7 => Self::OrdinalText,
            22 => Self::LeadingZero,
            23 => Self::Bullet,
            n => Self::Other(n),
        }
    }

    pub fn code(self) -> u32 {
        match self {
            Self::Arabic => 0,
            Self::UpperRoman => 1,
            Self::LowerRoman => 2,
            Self::UpperLetter => 3,
            Self::LowerLetter => 4,
            Self::Ordinal => 5,
            Self::CardinalText => 6,
            Self::OrdinalText => 7,
            Self::LeadingZero => 22,
            Self::Bullet => 23,
            Self::Other(n) => n,
        }
    }

    pub fn is_bullet(self) -> bool {
        self == Self::Bullet
    }
}

/// The list a paragraph is in (present exactly when the paragraph has item `0x257`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListInfo {
    pub kind: ListType,
    /// Bullet character (UTF-16 code unit; spec 4R.2), 0 for numbering.
    pub bullet_char: u32,
    /// Bullet font name from the FONT table; empty = not stored / unresolved.
    pub bullet_font: String,
    /// Bullet size in points; `None` = no item (the style's size).
    pub bullet_size_pt: Option<f32>,
    /// Number separator index 0-8 (spec 4R.3); 3 = plain, also what an absent `0x258` means.
    pub separator: u8,
    /// Start-at number (absent = 1).
    pub start: u32,
    /// `ListIndent` in points (absent = 18).
    pub list_indent_pt: f32,
}

impl ListInfo {
    /// A numbered list of `kind` as Publisher writes it after `SetListType`: separator `.`,
    /// start 1, list indent 18 pt.
    pub fn numbered(kind: ListType) -> Self {
        ListInfo {
            kind,
            bullet_char: 0,
            bullet_font: String::new(),
            bullet_size_pt: None,
            separator: SEPARATOR_DOT,
            start: 1,
            list_indent_pt: DEFAULT_LIST_INDENT_PT,
        }
    }

    /// A bullet list with the default bullet (U+00B7, Symbol, 10 pt).
    pub fn bullet() -> Self {
        Self::bullet_with(DEFAULT_BULLET_CHAR, DEFAULT_BULLET_FONT, DEFAULT_BULLET_SIZE_PT)
    }

    pub fn bullet_with(ch: u32, font: &str, size_pt: f32) -> Self {
        ListInfo {
            kind: ListType::Bullet,
            bullet_char: ch,
            bullet_font: font.to_owned(),
            bullet_size_pt: Some(size_pt),
            separator: SEPARATOR_PLAIN,
            start: 1,
            list_indent_pt: DEFAULT_LIST_INDENT_PT,
        }
    }
}

/// Paragraph indents in points (spec 4R.1). `first_line_pt` is signed: a hanging indent
/// is negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ParaIndents {
    pub left_pt: f32,
    pub first_line_pt: f32,
    pub right_pt: f32,
}

impl ParaIndents {
    /// The indents applying a list gives: left = list indent, first line = -list indent.
    pub fn for_list(list_indent_pt: f32) -> Self {
        ParaIndents {
            left_pt: list_indent_pt,
            first_line_pt: -list_indent_pt,
            right_pt: 0.0,
        }
    }
}

/// Whether `font` is a symbol-encoded face whose character codes are not Unicode text
/// (Symbol, Wingdings, Webdings): a bullet stored in one of them must be mapped to a
/// Unicode character to be drawn with another font.
pub fn is_symbol_font(font: &str) -> bool {
    let f = font.trim().to_ascii_lowercase();
    f == "symbol" || f.starts_with("wingdings") || f.starts_with("webdings")
}

/// The Unicode character of code `code` (32..=254) in Microsoft's Symbol font (Adobe's
/// Symbol encoding): Greek letters, mathematical signs, arrows, card suits. `None` for the
/// codes with no single character (the bracket pieces) and outside the table.
pub fn symbol_unicode(code: u32) -> Option<char> {
    const GREEK_UP: &str = "ΑΒΧΔΕΦΓΗΙϑΚΛΜΝΟΠΘΡΣΤΥςΩΞΨΖ";
    const GREEK_LOW: &str = "αβχδεφγηιϕκλμνοπθρστυϖωξψζ";
    const HIGH_A0: &str = "€ϒ′≤⁄∞ƒ♣♦♥♠↔←↑→↓°±″≥×∝∂•÷≠≡≈…⏐⎯↵ℵℑℜ℘⊗⊕∅∩∪⊃⊇⊄⊂⊆∈∉∠∇®©™∏√⋅¬∧∨⇔⇐⇑⇒⇓◊〈";
    let c = code as u8 as char;
    if code > 254 {
        return None;
    }
    Some(match code {
        0x41..=0x5A => GREEK_UP.chars().nth((code - 0x41) as usize)?,
        0x61..=0x7A => GREEK_LOW.chars().nth((code - 0x61) as usize)?,
        0x22 => '∀',
        0x24 => '∃',
        0x27 => '∋',
        0x2A => '∗',
        0x2D => '−',
        0x40 => '≅',
        0x5C => '∴',
        0x5E => '⊥',
        0x7E => '∼',
        0x20..=0x7E => c,
        0xA0..=0xE1 => HIGH_A0.chars().nth((code - 0xA0) as usize)?,
        0xF1 => '〉',
        0xF2 => '∫',
        _ => return None,
    })
}

/// The Unicode character to draw for bullet code `code` of font `font`. Symbol (U+00B7 is
/// its round bullet) becomes U+2022; the common Wingdings codes map to characters Arial and
/// Calibri have; any other code of a symbol font becomes U+2022. Codes of ordinary fonts are
/// drawn as themselves.
pub fn bullet_display_char(code: u32, font: &str) -> char {
    const DOT: char = '\u{2022}';
    let f = font.trim().to_ascii_lowercase();
    if f == "symbol" {
        // The asterisk bullet (42) is drawn as a plain `*`, a glyph every font has.
        return if code == 0x2A { '*' } else { symbol_unicode(code).unwrap_or(DOT) };
    }
    if f.starts_with("wingdings") || f.starts_with("webdings") {
        return match (f.as_str(), code) {
            ("wingdings", 0xA7) => '\u{25AA}',
            ("wingdings", 0x6C) => '\u{25CF}',
            ("wingdings", 0x6E) => '\u{25A0}',
            ("wingdings", 0xFC) => '\u{221A}',
            ("wingdings", 0xD8) => '\u{25BA}',
            ("wingdings", 0x76) => '\u{25C6}',
            _ => DOT,
        };
    }
    match char::from_u32(code) {
        Some(c) if !c.is_control() => c,
        _ => DOT,
    }
}

/// Default tab stop distance for a label wider than the hanging indent (0.5 inch ~).
pub const TAB_STOP_PT: f32 = 36.0;

/// Where the lines of a paragraph start, points from the left edge of its column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStarts {
    /// The label (list paragraphs): left indent + first-line indent.
    pub label_x: f32,
    /// The text of the first line, and of the lines after it.
    pub first_x: f32,
    pub rest_x: f32,
    /// The right indent.
    pub right: f32,
}

/// Line starts from the indents (spec 4R.1) and, for a list paragraph, the width of its
/// label. The label sits at left + first-line indent; the text starts at the left indent
/// (the list indent by default), or, when the label is wider than the hanging indent, at
/// the next 36 pt tab stop. UNCONFIRMED: Publisher's rule for that case, and for a list
/// with no hanging indent (taken here as text right after the label). Without a label the
/// first line simply starts at left + first-line indent.
pub fn line_starts(indents: ParaIndents, label_width: Option<f32>) -> LineStarts {
    let rest_x = indents.left_pt.max(0.0);
    let label_x = (indents.left_pt + indents.first_line_pt).max(0.0);
    let first_x = match label_width {
        None => label_x,
        Some(w) if rest_x > label_x => {
            if label_x + w <= rest_x {
                rest_x
            } else {
                ((label_x + w) / TAB_STOP_PT).floor() * TAB_STOP_PT + TAB_STOP_PT
            }
        }
        Some(w) => label_x + w,
    };
    LineStarts {
        label_x,
        first_x,
        rest_x,
        right: indents.right_pt.max(0.0),
    }
}

/// The separator pattern around the number text (spec 4R.3). Index 8 shows a character
/// the PDF text did not reveal; U+3001 is a guess ~.
fn with_separator(index: u8, n: &str) -> String {
    match index {
        0 => format!("{n})"),
        1 => format!("({n})"),
        2 => format!("{n}."),
        4 => format!("{n}]"),
        5 => format!("{n}:"),
        6 => format!("[{n}]"),
        7 => format!("-{n}-"),
        8 => format!("{n}\u{3001}"),
        _ => n.to_owned(),
    }
}

fn roman(mut n: u32) -> Option<String> {
    if n == 0 || n >= 4000 {
        return None;
    }
    const T: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, r) in T {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    Some(s)
}

/// A, B .. Z, AA, BB .. (the letter repeated; Word's rule ~).
fn letters(n: u32, upper: bool) -> Option<String> {
    if n == 0 {
        return None;
    }
    let base = if upper { b'A' } else { b'a' };
    let ch = (base + ((n - 1) % 26) as u8) as char;
    Some(std::iter::repeat_n(ch, ((n - 1) / 26 + 1) as usize).collect())
}

fn ordinal(n: u32) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

const ONES: [&str; 20] = [
    "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
    "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen", "Seventeen", "Eighteen",
    "Nineteen",
];
const TENS: [&str; 10] = [
    "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
];
const ORD_ONES: [&str; 20] = [
    "Zeroth", "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth",
    "Ninth", "Tenth", "Eleventh", "Twelfth", "Thirteenth", "Fourteenth", "Fifteenth",
    "Sixteenth", "Seventeenth", "Eighteenth", "Nineteenth",
];

/// "Twenty-One" (cardinal) / "Twenty-First" (ordinal) for 1..=99 (Word's capitalisation ~).
fn words(n: u32, ordinal: bool) -> Option<String> {
    match n {
        1..=19 => Some((if ordinal { ORD_ONES[n as usize] } else { ONES[n as usize] }).to_owned()),
        20..=99 => {
            let (t, o) = (n / 10, n % 10);
            if o == 0 {
                let w = TENS[t as usize];
                Some(if ordinal {
                    format!("{}ieth", &w[..w.len() - 1])
                } else {
                    w.to_owned()
                })
            } else {
                let tail = if ordinal { ORD_ONES[o as usize] } else { ONES[o as usize] };
                Some(format!("{}-{tail}", TENS[t as usize]))
            }
        }
        _ => None,
    }
}

/// The number text of value `n` in numbering `kind`, without separator. Values a style
/// cannot express (Roman 4000+, text 100+, letters 0) fall back to Arabic.
pub fn number_text(kind: ListType, n: u32) -> String {
    let t = match kind {
        ListType::UpperRoman => roman(n),
        ListType::LowerRoman => roman(n).map(|s| s.to_lowercase()),
        ListType::UpperLetter => letters(n, true),
        ListType::LowerLetter => letters(n, false),
        ListType::Ordinal => Some(ordinal(n)),
        ListType::CardinalText => words(n, false),
        ListType::OrdinalText => words(n, true),
        ListType::LeadingZero => Some(format!("{n:02}")),
        _ => None,
    };
    t.unwrap_or_else(|| n.to_string())
}

/// The label each paragraph of a story displays: the bullet character, or the number text
/// with its separator; `None` for paragraphs not in a list (spec 4R.5).
///
/// A numbering run is a sequence of adjacent paragraphs with the same list type, separator
/// and start; a plain paragraph or a bullet ends it, and the next numbered paragraph
/// restarts at its start value. Empty list paragraphs count. A different type or separator
/// also starts a new run (live PDF export of `w-num-types` and `w-num-seps`: every paragraph
/// showed its first number).
pub fn list_labels(paragraphs: &[Paragraph]) -> Vec<Option<String>> {
    list_labels_for(paragraphs.iter().map(|p| (p.list.as_ref(), p.continued)))
}

/// [`list_labels`] for any paragraph type: the list of each paragraph and whether it only
/// continues the previous one (a piece after a line break: no label, no effect on a run).
pub fn list_labels_for<'a>(
    items: impl IntoIterator<Item = (Option<&'a ListInfo>, bool)>,
) -> Vec<Option<String>> {
    let items = items.into_iter();
    let mut out = Vec::with_capacity(items.size_hint().0);
    let mut run: Option<(ListType, u8, u32, u32)> = None; // type, separator, start, next
    for (list, continued) in items {
        if continued {
            out.push(None);
            continue;
        }
        let Some(l) = list else {
            run = None;
            out.push(None);
            continue;
        };
        if l.kind.is_bullet() {
            run = None;
            out.push(Some(
                char::from_u32(l.bullet_char).map_or_else(String::new, |c| c.to_string()),
            ));
            continue;
        }
        let n = match run {
            Some((k, sep, s, next)) if k == l.kind && sep == l.separator && s == l.start => next,
            _ => l.start,
        };
        run = Some((l.kind, l.separator, l.start, n + 1));
        out.push(Some(with_separator(l.separator, &number_text(l.kind, n))));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_styles() {
        assert_eq!(number_text(ListType::UpperRoman, 4), "IV");
        assert_eq!(number_text(ListType::LowerRoman, 1994), "mcmxciv");
        assert_eq!(number_text(ListType::UpperLetter, 27), "AA");
        assert_eq!(number_text(ListType::LowerLetter, 3), "c");
        assert_eq!(number_text(ListType::Ordinal, 11), "11th");
        assert_eq!(number_text(ListType::Ordinal, 22), "22nd");
        assert_eq!(number_text(ListType::CardinalText, 21), "Twenty-One");
        assert_eq!(number_text(ListType::OrdinalText, 20), "Twentieth");
        assert_eq!(number_text(ListType::OrdinalText, 3), "Third");
        assert_eq!(number_text(ListType::LeadingZero, 7), "07");
        assert_eq!(number_text(ListType::LeadingZero, 12), "12");
        assert_eq!(number_text(ListType::Arabic, 5), "5");
        assert_eq!(number_text(ListType::UpperRoman, 4000), "4000");
    }

    #[test]
    fn separators() {
        let t = |i| with_separator(i, "1");
        assert_eq!(
            (0..8).map(t).collect::<Vec<_>>(),
            ["1)", "(1)", "1.", "1", "1]", "1:", "[1]", "-1-"]
        );
    }
}

#[cfg(test)]
mod symbol_tests {
    use super::*;

    #[test]
    fn symbol_font_codes_map_to_their_characters() {
        assert_eq!(symbol_unicode(0xB7), Some('\u{2022}'));
        assert_eq!(symbol_unicode(0xA8), Some('\u{2666}'));
        assert_eq!(symbol_unicode(0xDE), Some('\u{21D2}'));
        assert_eq!(symbol_unicode(0xE0), Some('\u{25CA}'));
        assert_eq!(symbol_unicode(0x61), Some('\u{3B1}'));
        assert_eq!(symbol_unicode(0x7A), Some('\u{3B6}'));
        assert_eq!(symbol_unicode(0x41), Some('\u{391}'));
        assert_eq!(symbol_unicode(0x5A), Some('\u{396}'));
        assert_eq!(symbol_unicode(0xD6), Some('\u{221A}'));
        assert_eq!(symbol_unicode(0xF2), Some('\u{222B}'));
        assert_eq!(symbol_unicode(0xE5), None);
        assert_eq!(bullet_display_char(183, "Symbol"), '\u{2022}');
        assert_eq!(bullet_display_char(168, "Symbol"), '\u{2666}');
        assert_eq!(bullet_display_char(222, "Symbol"), '\u{21D2}');
        assert_eq!(bullet_display_char(224, "Symbol"), '\u{25CA}');
        assert_eq!(bullet_display_char(42, "Symbol"), '*');
        assert_eq!(bullet_display_char(5, "Symbol"), '\u{2022}');
    }
}
