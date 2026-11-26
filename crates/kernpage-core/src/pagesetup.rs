//! Page Design > Page Setup: the preset page sizes and margin presets of Publisher 2021's
//! Size and Margins galleries (docs/ui/PAGE-SETUP.md) and the text they show.

use crate::model::{Margins, EMU_PER_INCH};

/// One row of the Size gallery's "Built-In" list: the portrait size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSizePreset {
    pub name: &'static str,
    pub width_emu: i64,
    pub height_emu: i64,
}

const fn mm(v: i64) -> i64 {
    v * 36_000
}

const fn inch_q(quarters: i64) -> i64 {
    quarters * EMU_PER_INCH / 4
}

/// The nine built-in sizes, alphabetical, in portrait (exact: A4 is 21 x 29.7 cm).
pub const PAGE_SIZES: [PageSizePreset; 9] = [
    PageSizePreset { name: "A3", width_emu: mm(297), height_emu: mm(420) },
    PageSizePreset { name: "A4", width_emu: mm(210), height_emu: mm(297) },
    PageSizePreset { name: "A5", width_emu: mm(148), height_emu: mm(210) },
    PageSizePreset { name: "B4", width_emu: mm(250), height_emu: mm(353) },
    PageSizePreset { name: "B5", width_emu: mm(176), height_emu: mm(250) },
    PageSizePreset { name: "Executive", width_emu: inch_q(29), height_emu: inch_q(42) },
    PageSizePreset { name: "Legal", width_emu: inch_q(34), height_emu: inch_q(56) },
    PageSizePreset { name: "Letter", width_emu: inch_q(34), height_emu: inch_q(44) },
    PageSizePreset { name: "Tabloid", width_emu: inch_q(44), height_emu: inch_q(68) },
];

/// Two sizes are the same when they differ by less than this (a Publisher file made with
/// `PageWidth = 595.3` stores 7,560,310, not 7,560,000).
const SAME_SIZE_EMU: i64 = 2_000;

pub fn is_landscape(width_emu: i64, height_emu: i64) -> bool {
    width_emu > height_emu
}

impl PageSizePreset {
    /// The preset's size in the given orientation.
    pub fn size(&self, landscape: bool) -> (i64, i64) {
        if landscape {
            (self.height_emu, self.width_emu)
        } else {
            (self.width_emu, self.height_emu)
        }
    }

    /// "A4 (Portrait)" / "A4 (Landscape)".
    pub fn label(&self, landscape: bool) -> String {
        format!(
            "{} ({})",
            self.name,
            if landscape { "Landscape" } else { "Portrait" }
        )
    }

    /// "8.268 x 11.693\"" as the gallery shows it.
    pub fn size_text(&self, landscape: bool) -> String {
        let (w, h) = self.size(landscape);
        size_text(w, h)
    }
}

/// Inches with up to three decimals, trailing zeros trimmed: `8.5`, `11`, `8.268`.
pub fn inches_text(emu: i64) -> String {
    let s = format!("{:.3}", emu as f64 / EMU_PER_INCH as f64);
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// "8.5 x 11\"".
pub fn size_text(w: i64, h: i64) -> String {
    format!("{} x {}\"", inches_text(w), inches_text(h))
}

/// Index into [`PAGE_SIZES`] of the preset the page size equals, in either orientation
/// (the gallery lists the orientation the page already has).
pub fn preset_of(width_emu: i64, height_emu: i64) -> Option<usize> {
    let landscape = is_landscape(width_emu, height_emu);
    PAGE_SIZES.iter().position(|p| {
        let (w, h) = p.size(landscape);
        (w - width_emu).abs() < SAME_SIZE_EMU && (h - height_emu).abs() < SAME_SIZE_EMU
    })
}

/// A row of the Margins gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginPreset {
    pub name: &'static str,
    /// "1\"", "0.5\"", "0.25\"", "0\"".
    pub text: &'static str,
    pub margins: Margins,
}

/// Wide, Moderate, Narrow, None (in the gallery's order).
pub const MARGIN_PRESETS: [MarginPreset; 4] = [
    MarginPreset { name: "Wide", text: "1\"", margins: Margins::all(EMU_PER_INCH) },
    MarginPreset { name: "Moderate", text: "0.5\"", margins: Margins::all(EMU_PER_INCH / 2) },
    MarginPreset { name: "Narrow", text: "0.25\"", margins: Margins::all(EMU_PER_INCH / 4) },
    MarginPreset { name: "None", text: "0\"", margins: Margins::all(0) },
];

/// Index into [`MARGIN_PRESETS`] of the preset the margins equal (to the EMU, since the
/// file stores exact values), or `None` for "Custom Margins".
pub fn margin_preset_of(m: Margins) -> Option<usize> {
    MARGIN_PRESETS.iter().position(|p| p.margins == m)
}

/// A length typed into a Page Setup field, in EMU: a number with an optional unit.
/// Units: `"` / `in` (inches, the default), `cm`, `mm`, `pt`, `pc` (picas). Spaces are free,
/// the decimal separator is `.`.
pub fn parse_length(text: &str) -> Result<i64, String> {
    let t = text.trim().to_lowercase();
    let units: [(&str, f64); 8] = [
        ("inches", EMU_PER_INCH as f64),
        ("inch", EMU_PER_INCH as f64),
        ("in", EMU_PER_INCH as f64),
        ("\"", EMU_PER_INCH as f64),
        ("cm", 360_000.0),
        ("mm", 36_000.0),
        ("pt", 12_700.0),
        ("pc", 152_400.0),
    ];
    let (num, per) = units
        .iter()
        .find_map(|(u, per)| t.strip_suffix(u).map(|n| (n.trim(), *per)))
        .unwrap_or((t.as_str(), EMU_PER_INCH as f64));
    let v: f64 = num
        .parse()
        .map_err(|_| format!("\"{}\" is not a length. Enter a number such as 8.5\", 21 cm or 36 pt.", text.trim()))?;
    if !v.is_finite() {
        return Err(format!("\"{}\" is not a length.", text.trim()));
    }
    Ok((v * per).round() as i64)
}

#[cfg(test)]
mod length_tests {
    use super::*;

    #[test]
    fn units_convert_to_emu() {
        let inch = EMU_PER_INCH;
        assert_eq!(parse_length("5"), Ok(5 * inch));
        assert_eq!(parse_length("5.5"), Ok(inch * 11 / 2));
        assert_eq!(parse_length("5.5\""), Ok(inch * 11 / 2));
        assert_eq!(parse_length(" 5.5 in "), Ok(inch * 11 / 2));
        assert_eq!(parse_length("14 cm"), Ok(5_040_000));
        assert_eq!(parse_length("140mm"), Ok(5_040_000));
        assert_eq!(parse_length("396 pt"), Ok(5_029_200));
        assert_eq!(parse_length("6pc"), Ok(914_400));
        assert_eq!(parse_length("0\""), Ok(0));
        assert_eq!(parse_length("-1"), Ok(-inch));
    }

    #[test]
    fn bad_input_is_an_error_message() {
        for bad in ["", "abc", "5 furlongs", "\"", "1,5", "NaN", "inf", "1..2 in"] {
            let e = parse_length(bad).unwrap_err();
            assert!(e.contains("not a length"), "{bad}: {e}");
        }
    }
}
