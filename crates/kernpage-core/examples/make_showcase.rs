//! Builds `the showcase newsletter (the repo keeps it as `samples/kern/showcase-newsletter.kern`; open the built .pub in KernPage and Save As .kern)`, an original made-up newsletter used for the
//! pictures in the README. Run: `cargo run -p kernpage-core --example make_showcase -- <out.pub>`.

use kernpage_core::geometry::in_to_emu as inch;
use kernpage_core::template::blank_portrait;
use kernpage_core::{
    Align, Command, Editor, Paragraph, Rect, Rgb, Run, SaveSource, Script, Shape, ShapeKind, Stroke,
};

const TEAL: Rgb = Rgb(0x0F, 0x76, 0x6E);
const DARK: Rgb = Rgb(0x1F, 0x2A, 0x37);
const SAND: Rgb = Rgb(0xF4, 0xEC, 0xD8);
const WHITE: Rgb = Rgb(0xFF, 0xFF, 0xFF);

fn p(text: &str, font: &str, size: f32, bold: bool, italic: bool, color: Rgb, align: Align) -> Paragraph {
    Paragraph {
        runs: vec![Run {
            text: text.to_owned(),
            font: font.to_owned(),
            size_pt: size,
            bold,
            italic,
            underline: false,
            script: Script::Normal,
            color,
            field: None,
        }],
        align,
        spacing: Default::default(),
        list: None,
        indents: Default::default(),
    }
}

fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect::new(inch(x), inch(y), inch(w), inch(h))
}

fn main() {
    let out = std::env::args().nth(1).expect("usage: make_showcase <out.pub>");
    let mut ed = Editor::new(blank_portrait());
    let add = |ed: &mut Editor, s: Shape| {
        ed.execute(Command::AddShape { page: 0, shape: s }).unwrap();
    };

    // Banner.
    let mut banner = Shape::new(r(0.0, 0.0, 8.5, 2.3), ShapeKind::Rectangle);
    banner.fill = Some(TEAL);
    add(&mut ed, banner);
    let mut sun = Shape::new(r(6.6, 0.35, 1.5, 1.5), ShapeKind::Ellipse);
    sun.fill = Some(Rgb(0xF2, 0xB7, 0x05));
    add(&mut ed, sun);
    add(
        &mut ed,
        Shape::text_box(
            r(0.6, 0.45, 6.0, 0.9),
            vec![p("The Riverbend Review", "Georgia", 34.0, true, false, WHITE, Align::Left)],
        ),
    );
    add(
        &mut ed,
        Shape::text_box(
            r(0.6, 1.4, 6.0, 0.6),
            vec![p(
                "News and notes for the neighbourhood  |  Autumn issue",
                "Calibri",
                16.0,
                false,
                true,
                WHITE,
                Align::Left,
            )],
        ),
    );

    // Lead article in two columns.
    let lead = Shape::text_box(
        r(0.6, 2.6, 5.2, 3.1),
        vec![
            p("Harvest Supper returns to the old mill", "Georgia", 22.0, true, false, TEAL, Align::Left),
            p(
                "After two quiet years the long tables are coming back. Bring a dish to share, a friend \
                 to sit beside, and an appetite. Soup, bread and pies will be on the counter from five \
                 o'clock, and the band begins as the light fades.",
                "Calibri",
                13.0,
                false,
                false,
                DARK,
                Align::Left,
            ),
            p(
                "Volunteers are needed to set up chairs on Friday afternoon and to wash dishes on \
                 Saturday night. Sign the sheet by the door, or simply turn up with your sleeves \
                 already rolled.",
                "Calibri",
                13.0,
                false,
                false,
                DARK,
                Align::Left,
            ),
        ],
    );
    add(&mut ed, lead);

    // Sidebar.
    let mut panel = Shape::new(r(6.0, 2.55, 2.1, 5.5), ShapeKind::Rectangle);
    panel.fill = Some(SAND);
    panel.line = Some(Stroke { color: TEAL, width_emu: 19_050 });
    add(&mut ed, panel);
    let side = Shape::text_box(
        r(6.1, 2.65, 1.9, 5.3),
        vec![
            p("Dates to keep", "Georgia", 18.0, true, false, TEAL, Align::Left),
            p("Oct 12", "Calibri", 13.0, true, false, DARK, Align::Left),
            p("Harvest Supper, 5 p.m.", "Calibri", 12.0, false, false, DARK, Align::Left),
            p("Oct 19", "Calibri", 13.0, true, false, DARK, Align::Left),
            p("Choir practice, 7 p.m.", "Calibri", 12.0, false, false, DARK, Align::Left),
            p("Oct 26", "Calibri", 13.0, true, false, DARK, Align::Left),
            p("Book swap in the hall", "Calibri", 12.0, false, false, DARK, Align::Left),
            p("Nov 2", "Calibri", 13.0, true, false, DARK, Align::Left),
            p("Clocks go back", "Calibri", 12.0, false, false, DARK, Align::Left),
        ],
    );
    add(&mut ed, side);

    add(
        &mut ed,
        Shape::text_box(
            r(0.6, 5.85, 5.2, 2.3),
            vec![
                p("From the garden", "Georgia", 18.0, true, false, TEAL, Align::Left),
                p(
                    "The pumpkins are enormous this year and nobody can say why. The allotment group will be giving them away at the supper, along with cuttings, seed packets and advice nobody asked for. Bring a bag, and perhaps a wheelbarrow.",
                    "Calibri",
                    13.0,
                    false,
                    false,
                    DARK,
                    Align::Left,
                ),
            ],
        ),
    );

    // Footer rule and note.
    let mut rule = Shape::new(r(0.6, 8.5, 7.4, 0.0), ShapeKind::Line);
    rule.line = Some(Stroke { color: TEAL, width_emu: 25_400 });
    add(&mut ed, rule);
    add(
        &mut ed,
        Shape::text_box(
            r(0.6, 8.65, 7.4, 1.0),
            vec![
                p(
                    "Got news? Leave a note in the box by the front door. This is a made-up sample page \
                     for KernPage; every name and date in it is invented.",
                    "Calibri",
                    11.0,
                    false,
                    true,
                    DARK,
                    Align::Center,
                ),
            ],
        ),
    );

    let src = SaveSource::for_new(ed.doc()).unwrap();
    let plan = src.plan(ed.doc());
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (bytes, _) = src.build(ed.doc()).unwrap();
    std::fs::write(&out, bytes).unwrap();
    println!("wrote {out}");
}
