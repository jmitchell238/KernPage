//! New-document starting points.

use crate::geometry::{in_to_emu, Rect};
use crate::model::*;

pub const LETTER_W_EMU: i64 = 7_772_400;
pub const LETTER_H_EMU: i64 = 10_058_400;
/// Our own accent colour (a deep blue).
pub const ACCENT: Rgb = Rgb(0x1F, 0x3A, 0x6E);

pub fn blank_portrait() -> Document {
    Document::blank("Publication1", LETTER_W_EMU, LETTER_H_EMU)
}

pub fn blank_landscape() -> Document {
    Document::blank("Publication1", LETTER_H_EMU, LETTER_W_EMU)
}

fn run(text: &str, font: &str, size: f32, bold: bool, italic: bool, color: Rgb) -> Run {
    Run {
        text: text.to_owned(),
        font: font.to_owned(),
        size_pt: size,
        bold,
        italic,
        underline: false,
        script: crate::model::Script::Normal,
        color,
        field: None,
    }
}

fn para(text: &str, font: &str, size: f32, bold: bool, italic: bool, align: Align) -> Paragraph {
    Paragraph {
        runs: vec![run(text, font, size, bold, italic, Rgb::BLACK)],
        align,
        spacing: Default::default(),
        list: None,
        indents: Default::default(),
    }
}

fn heading(text: &str) -> Paragraph {
    Paragraph {
        runs: vec![run(text, "Georgia", 20.0, true, false, ACCENT)],
        align: Align::Left,
        spacing: Default::default(),
        list: None,
        indents: Default::default(),
    }
}

fn body(text: &str) -> Paragraph {
    para(text, "Calibri", 14.0, false, false, Align::Left)
}

/// A two-page Letter church bulletin: a title box and an order of service on page 1,
/// announcements on page 2. Every box is ordinary editable text.
pub fn church_bulletin() -> Document {
    let inch = in_to_emu;
    let title = Shape::text_box(
        Rect::new(inch(0.75), inch(0.6), inch(7.0), inch(1.3)),
        vec![
            Paragraph {
                runs: vec![run(
                    "Your Church Name",
                    "Georgia",
                    30.0,
                    true,
                    false,
                    ACCENT,
                )],
                align: Align::Center,
                spacing: Default::default(),
                list: None,
                indents: Default::default(),
            },
            para(
                "Sunday Morning Worship",
                "Georgia",
                16.0,
                false,
                true,
                Align::Center,
            ),
            para(
                "Sunday, 10:30 a.m.",
                "Calibri",
                12.0,
                false,
                false,
                Align::Center,
            ),
        ],
    );
    let mut rule = Shape::new(
        Rect::new(inch(0.75), inch(2.0), inch(7.0), 0),
        ShapeKind::Line,
    );
    rule.line = Some(Stroke {
        color: ACCENT,
        width_emu: 2 * EMU_PER_POINT,
    });
    let mut order = vec![heading("Order of Service")];
    order.extend(
        [
            "Welcome and Announcements",
            "Opening Hymn",
            "Prayer",
            "Scripture Reading",
            "Offering",
            "Message",
            "Closing Hymn",
            "Benediction",
        ]
        .map(body),
    );
    let order_box = Shape::text_box(
        Rect::new(inch(0.75), inch(2.3), inch(7.0), inch(7.4)),
        order,
    );
    let mut page1 = Page::blank(LETTER_W_EMU, LETTER_H_EMU);
    page1.shapes = vec![title, rule, order_box];

    let mut ann = vec![heading("Announcements")];
    ann.extend(
        [
            "Wednesday Bible Study at 6:30 p.m.",
            "Nursery is available during the morning service.",
            "Please see the welcome table for the prayer list.",
        ]
        .map(body),
    );
    let ann_box = Shape::text_box(Rect::new(inch(0.75), inch(0.75), inch(7.0), inch(8.5)), ann);
    let mut page2 = Page::blank(LETTER_W_EMU, LETTER_H_EMU);
    page2.shapes = vec![ann_box];

    Document::new("Church Bulletin", vec![page1, page2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_sizes() {
        let p = blank_portrait();
        assert_eq!(
            (p.pages[0].width_emu, p.pages[0].height_emu),
            (7_772_400, 10_058_400)
        );
        let l = blank_landscape();
        assert_eq!(
            (l.pages[0].width_emu, l.pages[0].height_emu),
            (10_058_400, 7_772_400)
        );
        assert!(p.pages[0].shapes.is_empty());
    }

    #[test]
    fn bulletin_has_two_pages_with_title_order_and_announcements() {
        let d = church_bulletin();
        assert_eq!(d.pages.len(), 2);
        let texts = |i: usize| -> Vec<String> {
            d.pages[i]
                .shapes
                .iter()
                .filter_map(|s| match &s.kind {
                    ShapeKind::TextBox { paragraphs } => Some(paragraphs[0].text()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(texts(0), ["Your Church Name", "Order of Service"]);
        assert_eq!(texts(1), ["Announcements"]);
        let mut ids: Vec<_> = d
            .pages
            .iter()
            .flat_map(|p| p.shapes.iter().map(|s| s.id))
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4);
        // Everything sits inside the page.
        for p in &d.pages {
            for s in &p.shapes {
                assert!(s.rect.right() <= p.width_emu && s.rect.bottom() <= p.height_emu);
            }
        }
    }
}
