use pub2pdf::{default_font_dirs, render, Fonts, Options};
use std::path::PathBuf;

fn sample(name: &str) -> PathBuf {
    kernpage_samples::path(name)
}

const MODERN: [&str; 7] = [
    "blank.pub",
    "hello.pub",
    "hello-moved.pub",
    "two-pages.pub",
    "bold.pub",
    "two-boxes.pub",
    "tika-testPUBLISHER.pub",
];

const GEN: [&str; 9] = [
    "gen/apple-banana.pub",
    "gen/formatting.pub",
    "gen/picture.pub",
    "gen/legal.pub",
    "gen/bulletin.pub",
    "gen/colors.pub",
    "gen/shapes.pub",
    "gen/table.pub",
    "gen/table2.pub",
];

/// Standard-14 fonts only, so the tests can look for plain strings and font names.
fn std_opts() -> Options {
    Options {
        font_dirs: Vec::new(),
        ..Options::default()
    }
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

#[kernpage_samples::test]
fn every_sample_renders_with_matching_page_count() {
    for name in MODERN.iter().chain(GEN.iter()) {
        let Some(p) = kernpage_samples::find(name) else {
            continue;
        };
        let doc = pub_format::read(&p).unwrap();
        for outlines in [true, false] {
            let pdf = render(
                &doc,
                &Options {
                    outlines,
                    ..Options::default()
                },
            );
            assert!(pdf.starts_with(b"%PDF"), "{name}");
            // `/Type /Page` also prefixes `/Type /Pages`; exclude the page tree.
            let pages = count(&pdf, b"/Type /Page") - count(&pdf, b"/Type /Pages");
            assert_eq!(pages, doc.pages.len(), "{name}");
        }
    }
}

#[kernpage_samples::test]
fn hello_text_is_in_the_content_stream() {
    let doc = pub_format::read(&sample("hello.pub")).unwrap();
    let pdf = render(&doc, &std_opts());
    assert!(count(&pdf, b"(Hello) Tj") >= 1);
}

#[kernpage_samples::test]
fn bold_text_uses_the_bold_font() {
    let doc = pub_format::read(&sample("bold.pub")).unwrap();
    let pdf = render(&doc, &std_opts());
    assert!(count(&pdf, b"/BaseFont /Helvetica-Bold") >= 1);
    assert!(count(&pdf, b"/BaseFont /Helvetica\n") >= 1);
    assert!(count(&pdf, b" Tf\n") >= 2);
    assert!(count(&pdf, b"(HELLO BOLD) Tj") >= 1);
}

#[kernpage_samples::test]
fn outlines_flag_controls_strokes() {
    let doc = pub_format::read(&sample("hello.pub")).unwrap();
    let with = |outlines| {
        render(
            &doc,
            &Options {
                outlines,
                ..std_opts()
            },
        )
    };
    assert!(count(&with(true), b" re\n") >= 1);
    assert_eq!(count(&with(false), b" re\n"), 0);
}

/// Render a gen sample with standard fonts (for text assertions).
fn render_gen(name: &str) -> (pub_format::Document, Vec<u8>) {
    let doc = pub_format::read(&sample(&format!("gen/{name}.pub"))).unwrap();
    let pdf = render(&doc, &std_opts());
    (doc, pdf)
}

/// Render a gen sample with the default font directories and leave the result in `out/`
/// for eyeballing (the folder is gitignored).
fn render_gen_fonts(name: &str) -> (pub_format::Document, Vec<u8>) {
    let doc = pub_format::read(&sample(&format!("gen/{name}.pub"))).unwrap();
    let pdf = render(&doc, &Options::default());
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out");
    let _ = std::fs::create_dir_all(&out);
    let _ = std::fs::write(out.join(format!("{name}.pdf")), &pdf);
    (doc, pdf)
}

#[kernpage_samples::test]
fn bulletin_renders_two_pages_with_an_image() {
    let (doc, pdf) = render_gen("bulletin");
    assert_eq!(doc.pages.len(), 2);
    assert_eq!(
        count(&pdf, b"/Type /Page") - count(&pdf, b"/Type /Pages"),
        2
    );
    assert!(count(&pdf, b"/Subtype /Image") >= 1);
}

#[kernpage_samples::test]
fn picture_has_an_image_xobject() {
    let (_, pdf) = render_gen("picture");
    assert!(count(&pdf, b"/Subtype /Image") >= 1);
    assert!(count(&pdf, b"/XObject") >= 1);
    assert!(count(&pdf, b" Do\n") >= 1);
}

#[kernpage_samples::test]
fn formatting_uses_times_and_courier() {
    let (_, pdf) = render_gen("formatting");
    assert!(count(&pdf, b"/BaseFont /Times-Italic") >= 1);
    assert!(count(&pdf, b"/BaseFont /Courier") >= 1);
    assert!(count(&pdf, b"/BaseFont /Helvetica-BoldOblique") >= 1);
    // The underlined Courier paragraph draws a line.
    assert!(count(&pdf, b" l\n") >= 1);
}

#[kernpage_samples::test]
fn alignment_moves_text() {
    let (doc, pdf) = render_gen("formatting");
    assert_eq!(doc.pages[0].shapes[0].paragraphs.len(), 8);
    // Left-aligned text starts at box left + inset; centred and right text must not.
    let text = String::from_utf8_lossy(&pdf);
    let x_of = |s: &str| -> f32 {
        let at = text.find(s).unwrap();
        let head = &text[..at];
        let line = head.lines().rev().find(|l| l.ends_with(" Td")).unwrap();
        line.split_whitespace().next().unwrap().parse().unwrap()
    };
    let left = x_of("(Arial 12)");
    assert!(x_of("(Centered)") > left + 20.0);
    assert!(x_of("(Right aligned)") > x_of("(Centered)"));
}

#[kernpage_samples::test]
fn every_gen_sample_is_written_to_out() {
    for name in GEN {
        if kernpage_samples::find(name).is_none() {
            continue;
        }
        render_gen_fonts(name.trim_start_matches("gen/").trim_end_matches(".pub"));
    }
}

#[kernpage_samples::test]
fn colors_draws_the_red_run_in_red() {
    let (_, pdf) = render_gen("colors");
    assert!(count(&pdf, b"1 0 0 rg\n") >= 1, "red fill colour operator");
    assert!(count(&pdf, b"0 1 0 rg\n") >= 1, "green");
    assert!(count(&pdf, b"0 0 1 rg\n") >= 1, "blue");
    // The default run stays black.
    assert!(count(&pdf, b"0 g\n") >= 1);
}

#[kernpage_samples::test]
fn shapes_have_fills_and_strokes() {
    let (_, pdf) = render_gen("shapes");
    // Light blue fill + dark blue outline on the rectangle, yellow oval, red 3 pt line.
    assert!(count(&pdf, b"0.6784314 0.84705883 0.9019608 rg\n") >= 1);
    assert!(count(&pdf, b"0 0 0.54509807 RG\n") >= 1);
    assert!(count(&pdf, b"1 1 0 rg\n") >= 1);
    assert!(count(&pdf, b"1 0 0 RG\n") >= 1);
    assert!(count(&pdf, b"3 w\n") >= 1, "3 pt line width");
    assert!(count(&pdf, b"\nB\n") >= 1, "fill and stroke");
    assert!(count(&pdf, b"\nf\n") >= 1, "fill only (the oval)");
    assert!(count(&pdf, b"\nS\n") >= 1, "stroke only");
    assert!(count(&pdf, b" c\n") >= 4, "oval curves");
}

#[kernpage_samples::test]
fn table_cells_are_drawn() {
    let (_, pdf) = render_gen("table");
    for cell in ["A1", "B1", "A2", "B2"] {
        assert!(
            count(&pdf, format!("({cell}) Tj").as_bytes()) >= 1,
            "{cell}"
        );
    }
    // Four cell rectangles in the grid plus the debug outline of the whole table.
    assert!(count(&pdf, b" re\n") >= 4);
    assert!(count(&pdf, b"0.75 w\n") >= 1);
}

#[kernpage_samples::test]
fn table2_uses_real_sizes_and_multi_paragraph_cells() {
    let (doc, pdf) = render_gen("table2");
    let text = String::from_utf8_lossy(&pdf);
    for cell in [
        "First paragraph in B2",
        "Second paragraph in B2",
        "Third",
        "Two",
        "Lines",
    ] {
        assert!(text.contains(&format!("({cell}) Tj")), "{cell}");
    }
    let s = doc.pages[0]
        .shapes
        .iter()
        .find(|s| s.table.is_some())
        .unwrap();
    let t = s.table.as_ref().unwrap();
    assert_eq!(t.column_widths_emu, vec![914_400, 2_743_200, 1_828_800]);
    // Cell rectangles are 72, 216 and 144 pt wide, 36, 108 and 72 pt tall.
    for w in ["72 36 re", "216 108 re", "144 72 re"] {
        assert!(text.contains(w), "{w}");
    }
}

fn has_font(name: &str) -> bool {
    Fonts::scan(&default_font_dirs())
        .find(name, false, false)
        .is_some_and(|f| f.embeddable)
}

#[kernpage_samples::test]
fn bulletin_embeds_georgia_and_calibri() {
    if !(has_font("Georgia") && has_font("Calibri")) {
        eprintln!("skipped: Georgia/Calibri not found in the font directories");
        return;
    }
    let (_, pdf) = render_gen_fonts("bulletin");
    assert!(count(&pdf, b"/FontFile2") >= 2);
    assert!(count(&pdf, b"+Georgia") >= 1);
    assert!(count(&pdf, b"+Calibri") >= 1);
    assert!(count(&pdf, b"/Identity-H") >= 2);
    assert!(count(&pdf, b"/ToUnicode") >= 2);
    assert!(count(&pdf, b"/CIDFontType2") >= 2);
    // Embedded fonts replace the standard stand-ins.
    assert_eq!(count(&pdf, b"/BaseFont /Times"), 0);
}

#[kernpage_samples::test]
fn missing_font_dirs_fall_back_to_standard_fonts() {
    let doc = pub_format::read(&sample("gen/bulletin.pub")).unwrap();
    for dirs in [vec![], vec![PathBuf::from("/nonexistent/fonts")]] {
        let pdf = render(
            &doc,
            &Options {
                font_dirs: dirs,
                ..Options::default()
            },
        );
        assert_eq!(count(&pdf, b"/FontFile2"), 0);
        assert!(count(&pdf, b"/BaseFont /Times") >= 1);
    }
}

/// Max x of the text positioned on the first baseline of the page.
fn first_line_right(pdf: &[u8]) -> (f32, usize) {
    let text = String::from_utf8_lossy(pdf);
    let tds: Vec<(f32, f32)> = text
        .lines()
        .filter(|l| l.ends_with(" Td"))
        .map(|l| {
            let mut it = l.split_whitespace();
            (
                it.next().unwrap().parse().unwrap(),
                it.next().unwrap().parse().unwrap(),
            )
        })
        .collect();
    let y = tds[0].1;
    let on_line: Vec<_> = tds.iter().filter(|t| (t.1 - y).abs() < 0.01).collect();
    (
        on_line.iter().map(|t| t.0).fold(0.0, f32::max),
        on_line.len(),
    )
}

#[kernpage_samples::test]
fn justify_spreads_words_but_not_the_last_line() {
    use pub_format::{Alignment, Document, Page, Paragraph, Shape, TextRun};
    let run = |text: &str| TextRun {
        text: text.into(),
        bold: false,
        italic: false,
        size_pt: 10.0,
        font: "Arial".into(),
        underline: false,
        script: pub_format::Script::Normal,
        color: None,
        field: None,
    };
    let doc = |alignment| Document {
        story_ids: Vec::new(),
        pages: vec![Page {
            object_number: 1,
            master: None,
            ignore_master: false,
            shapes: vec![Shape {
                object_number: 2,
                kind: 1,
                left_emu: -1_270_000,
                top_emu: -1_270_000,
                right_emu: 1_270_000, // a 200 pt wide box
                bottom_emu: 1_270_000,
                width_emu: None,
                height_emu: None,
                rotation_deg: 0.0,
                flip_h: false,
                flip_v: false,
                group: None,
                wrap: pub_format::WrapType::None,
                wrap_dist_emu: [36_576; 4],
                inset_emu: [36_576; 4],
                columns: 1,
                column_gap_emu: 0,
                story: None,
                chain_story: None,
                chain_pos: 0,
                text: None,
                runs: Vec::new(),
                paragraphs: vec![Paragraph {
                    runs: vec![run(
                        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu\r",
                    )],
                    alignment,
                    spacing: Default::default(),
                    list: None,
                    indents: Default::default(),
                    continued: false,
                }],
                picture: None,
                geometry: None,
                fill: None,
                line: None,
                table: None,
            }],
        }],
        text: String::new(),
        stories: Vec::new(),
        fonts: Vec::new(),
        page_width_emu: 7_772_400,
        page_height_emu: 10_058_400,
        margins_emu: None,
        color_scheme: None,
        columns_mapped: 0,
        columns_unmapped: 0,
        shapes_without_anchor: 0,
        masters: Vec::new(),
        sections: Vec::new(),
    };
    let left = render(&doc(Alignment::Left), &std_opts());
    let justified = render(&doc(Alignment::Justify), &std_opts());
    let (left_x, left_words) = first_line_right(&left);
    let (just_x, just_words) = first_line_right(&justified);
    // Left-aligned lines are one text object each; justified lines place every word.
    assert_eq!(left_words, 1);
    assert!(just_words > 2, "{just_words}");
    // The last word of a justified line ends at the box's right edge (box text area is
    // 200 - 2 * 3.6 pt wide, starting at x = 306 - 100 + 3.6).
    assert!(just_x > left_x + 20.0, "{just_x} vs {left_x}");
    // The paragraph's last line is left-aligned: it starts where left-aligned text does.
    let starts = |pdf: &[u8]| {
        String::from_utf8_lossy(pdf)
            .lines()
            .rfind(|l| l.ends_with(" Td"))
            .map(|l| l.split_whitespace().next().unwrap().to_string())
    };
    assert_eq!(starts(&left), starts(&justified));
}

/// Frame breaks of the single 3-frame chain in `gen/linked.pub`.
fn linked_breaks(opts: &Options) -> (Vec<(usize, usize)>, usize) {
    let doc = pub_format::read(&sample("gen/linked.pub")).unwrap();
    let breaks = pub2pdf::chain_breaks(&doc, opts);
    // One chain of three frames; the standalone box is not a chain.
    assert_eq!(breaks.len(), 1);
    assert_eq!(breaks[0].len(), 3);
    (breaks[0].clone(), doc.pages[0].shapes[0].paragraphs.len())
}

#[kernpage_samples::test]
fn linked_story_flows_through_the_frames() {
    let (ends, paragraphs) = linked_breaks(&Options::default());
    // Frames fill in order and the break moves forward each time.
    assert!(ends[0] < ends[1] && ends[1] < ends[2], "{ends:?}");
    // Each 3 x 2 in frame holds several 2-line paragraphs, not the whole story.
    assert!((4..=6).contains(&ends[0].0), "{ends:?}");
    assert!((8..=11).contains(&ends[1].0), "{ends:?}");
    // The story overflows the last frame; the rest is not drawn.
    assert!(ends[2].0 < paragraphs, "{ends:?}");
}

/// Spec 4G: Publisher breaks after paragraph 4 and after paragraph 8.
#[kernpage_samples::test]
fn linked_breaks_match_publisher() {
    let (ends, _) = linked_breaks(&Options::default());
    assert_eq!(&ends[..2], &[(4, 0), (8, 0)], "{ends:?}");
}

#[kernpage_samples::test]
fn linked_frames_draw_only_their_share() {
    let doc = pub_format::read(&sample("gen/linked.pub")).unwrap();
    let pdf = render(&doc, &std_opts());
    // Paragraph 30 lies far beyond the last frame and must not be drawn anywhere.
    assert_eq!(count(&pdf, b"Paragraph 30"), 0);
    // Paragraph 1 is drawn once (in frame 1), not once per frame.
    assert_eq!(count(&pdf, b"Paragraph 1 of"), 1);
}

/// The frames of the chain of `gen/linked.pub` as `flow_story` takes them.
fn linked_frames(doc: &pub_format::Document) -> Vec<pub2pdf::FrameGeom> {
    doc.chains()
        .into_iter()
        .find(|c| c.len() > 1)
        .unwrap()
        .iter()
        .map(|r| pub2pdf::FrameGeom::of_shape(doc, r.page, r.shape).unwrap())
        .collect()
}

#[kernpage_samples::test]
fn flow_story_ends_equal_chain_breaks_and_caching_changes_nothing() {
    let opts = Options::default();
    let doc = pub_format::read(&sample("gen/linked.pub")).unwrap();
    let frames = linked_frames(&doc);
    let chain = doc.chains().into_iter().find(|c| c.len() > 1).unwrap();
    let story = doc.pages[chain[0].page].shapes[chain[0].shape].paragraphs.clone();
    let want = pub2pdf::chain_breaks(&doc, &opts).remove(0);
    assert_eq!(pub2pdf::flow_story(&frames, &story, &opts), want);
    // A reused context gives the same answer every time, for other stories too.
    let mut ctx = pub2pdf::FlowContext::new(&opts);
    for _ in 0..3 {
        assert_eq!(ctx.flow(&frames, &story), want);
    }
    let shorter = &story[..10];
    let a = ctx.flow(&frames, shorter);
    assert_eq!(a, pub2pdf::flow_story(&frames, shorter, &opts));
    assert_eq!(ctx.flow(&frames, &story), want);
    // A shorter story ends every frame at its end once it is all laid out.
    assert_eq!(*a.last().unwrap(), (10, 0));
}

/// Time of one re-flow: `cargo test -p pub2pdf --test pdf '--' '--ignored' --nocapture flow_timing`
#[kernpage_samples::test]
#[ignore = "timing report"]
fn flow_timing() {
    let opts = Options::default();
    let doc = pub_format::read(&sample("gen/linked.pub")).unwrap();
    let frames = linked_frames(&doc);
    let chain = doc.chains().into_iter().find(|c| c.len() > 1).unwrap();
    let story = doc.pages[chain[0].page].shapes[chain[0].shape].paragraphs.clone();
    let t = std::time::Instant::now();
    pub2pdf::flow_story(&frames, &story, &opts);
    println!("first one-shot flow_story: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    pub2pdf::flow_story(&frames, &story, &opts);
    println!("second one-shot flow_story: {:?}", t.elapsed());
    let mut ctx = pub2pdf::FlowContext::new(&opts);
    let t = std::time::Instant::now();
    ctx.flow(&frames, &story);
    println!("first FlowContext::flow: {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..20 {
        ctx.flow(&frames, &story);
    }
    println!("cached FlowContext::flow: {:?} each", t.elapsed() / 20);
    let t = std::time::Instant::now();
    pub2pdf::chain_breaks(&doc, &opts);
    println!("chain_breaks (no cache): {:?}", t.elapsed());
}

/// `gen/spacing.pub`: each paragraph starts where Publisher put it (manifest `top_pt`).
#[kernpage_samples::test]
fn paragraph_tops_match_publisher() {
    let doc = pub_format::read(&sample("gen/spacing.pub")).unwrap();
    // `top_pt` of the `spacing` entry of samples/gen/manifest.json.
    let want = [75.6f32, 95.4, 117.0, 154.8, 194.4, 248.4, 315.0];
    let shape = doc.pages[0]
        .shapes
        .iter()
        .position(|s| !s.paragraphs.is_empty())
        .unwrap();
    let tops = pub2pdf::paragraph_tops(&doc, 0, shape, &Options::default());
    assert_eq!(tops.len(), want.len(), "{tops:?}");
    for (i, (got, w)) in tops.iter().zip(want).enumerate() {
        assert!((got - w).abs() <= 2.0, "paragraph {i}: {got} vs {w}");
    }
}

/// Text-position operators `x y Td` of a PDF content stream.
fn text_positions(pdf: &[u8]) -> Vec<(f32, f32)> {
    String::from_utf8_lossy(pdf)
        .lines()
        .filter(|l| l.ends_with(" Td"))
        .map(|l| {
            let mut it = l.split_whitespace();
            (
                it.next().unwrap().parse().unwrap(),
                it.next().unwrap().parse().unwrap(),
            )
        })
        .collect()
}

/// `gen/columns.pub`: the 3-column box (72..540 pt wide, 18 pt gaps) puts text at three
/// distinct left edges, 141.6 + 18 pt apart; the 2-column box at two.
#[kernpage_samples::test]
fn columns_box_text_occupies_distinct_x_ranges() {
    let doc = pub_format::read(&sample("gen/columns.pub")).unwrap();
    let pdf = render(&doc, &std_opts());
    let page_h = doc.page_height_emu as f32 / 12_700.0;
    let xs = |top_pt: f32, bottom_pt: f32| {
        let mut v: Vec<f32> = text_positions(&pdf)
            .into_iter()
            .filter(|&(_, y)| y < page_h - top_pt && y > page_h - bottom_pt)
            .map(|(x, _)| (x * 10.0).round() / 10.0)
            .collect();
        v.sort_by(f32::total_cmp);
        v.dedup();
        v
    };
    let three = xs(72.0, 360.0);
    assert_eq!(three.len(), 3, "{three:?}");
    assert!((three[0] - 75.6).abs() < 0.2, "{three:?}");
    assert!((three[1] - three[0] - 159.6).abs() < 0.2, "{three:?}");
    assert!((three[2] - three[1] - 159.6).abs() < 0.2, "{three:?}");
    assert_eq!(xs(432.0, 648.0).len(), 2);
}

/// A rotated box is drawn under a `cm` that turns it about its centre; an unrotated
/// sample has none beyond the picture placements.
#[kernpage_samples::test]
fn rotated_boxes_use_a_rotation_matrix() {
    let doc = pub_format::read(&sample("gen/rotate.pub")).unwrap();
    let pdf = String::from_utf8_lossy(&render(&doc, &std_opts())).into_owned();
    let cms = pdf.lines().filter(|l| l.ends_with(" cm")).count();
    assert_eq!(cms, 4, "one cm per rotated shape");
    // 90 degrees clockwise: cos = 0, sin(-90) = -1 -> matrix 0 -1 1 0 (to rounding).
    let turned = pdf.lines().filter(|l| l.ends_with(" cm")).any(|l| {
        let m: Vec<f32> = l
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        m.len() == 6
            && m[0].abs() < 1e-4
            && (m[1] + 1.0).abs() < 1e-4
            && (m[2] - 1.0).abs() < 1e-4
            && m[3].abs() < 1e-4
    });
    assert!(turned);
}

/// A placeable WMF (bounds 0,0 - 100,100) holding a red filled triangle and an ellipse.
fn small_wmf() -> Vec<u8> {
    fn rec(out: &mut Vec<u8>, func: u16, words: &[i16]) {
        out.extend(((3 + words.len()) as u32).to_le_bytes());
        out.extend(func.to_le_bytes());
        for w in words {
            out.extend(w.to_le_bytes());
        }
    }
    let mut recs = Vec::new();
    // CreateBrushIndirect: solid, red; SelectObject 0.
    rec(&mut recs, 0x02FC, &[0, 255, 0, 0]);
    rec(&mut recs, 0x012D, &[0]);
    rec(&mut recs, 0x0324, &[3, 0, 0, 100, 0, 50, 80]); // Polygon
    rec(&mut recs, 0x0418, &[90, 90, 10, 10]); // Ellipse: bottom, right, top, left
    rec(&mut recs, 0, &[]);
    let mut out = Vec::new();
    out.extend(0x9AC6_CDD7u32.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    for v in [0i16, 0, 100, 100] {
        out.extend(v.to_le_bytes());
    }
    out.extend(1440u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(9u16.to_le_bytes());
    out.extend(0x0300u16.to_le_bytes());
    out.extend((((18 + recs.len()) / 2) as u32).to_le_bytes());
    out.extend(8u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(recs);
    out
}

#[kernpage_samples::test]
fn wmf_picture_is_drawn_as_vector_paths() {
    use pub_format::{Document, Page, Picture, PictureFormat, Shape};
    let shape = Shape {
        object_number: 2,
        kind: 1,
        left_emu: -635_000,
        top_emu: -635_000,
        right_emu: 635_000, // a 100 pt square
        bottom_emu: 635_000,
        width_emu: None,
        height_emu: None,
        rotation_deg: 0.0,
        flip_h: false,
        flip_v: false,
        group: None,
        wrap: pub_format::WrapType::None,
        wrap_dist_emu: [36_576; 4],
        inset_emu: [36_576; 4],
        columns: 1,
        column_gap_emu: 0,
        story: None,
        chain_story: None,
        chain_pos: 0,
        text: None,
        runs: Vec::new(),
        paragraphs: Vec::new(),
        picture: Some(Picture {
            format: PictureFormat::Wmf,
            bytes: small_wmf(),
        }),
        geometry: None,
        fill: None,
        line: None,
        table: None,
    };
    let doc = Document {
        story_ids: Vec::new(),
        pages: vec![Page {
            object_number: 1,
            master: None,
            ignore_master: false,
            shapes: vec![shape],
        }],
        text: String::new(),
        stories: Vec::new(),
        fonts: Vec::new(),
        page_width_emu: 7_772_400,
        page_height_emu: 10_058_400,
        margins_emu: None,
        color_scheme: None,
        columns_mapped: 0,
        columns_unmapped: 0,
        shapes_without_anchor: 0,
        masters: Vec::new(),
        sections: Vec::new(),
    };
    let pdf = render(&doc, &std_opts());
    let text = String::from_utf8_lossy(&pdf);
    // Red fill, a closed polygon path and four curve segments for the ellipse.
    assert!(text.contains("1 0 0 rg"), "no red fill");
    assert!(count(&pdf, b" l\n") >= 2, "polygon lines");
    assert!(count(&pdf, b" c\n") >= 4, "ellipse curves");
    assert!(
        count(&pdf, b"\nB\n") + count(&pdf, b"\nB*\n") >= 2,
        "fill and stroke"
    );
    // Not the grey placeholder.
    assert_eq!(count(&pdf, b"0.85 g"), 0);
}

/// `gen/wrap.pub` (spec 4K): the first 27 lines Publisher laid out, as (top, text width)
/// in points (its own `BoundTop` / `BoundWidth` of each line; 450 = a row split into two
/// pieces around the square, 333.9 / 338.4 = a row that ends before the shape).
const WRAP_LINES: [(f32, f32); 27] = [
    (75.6, 333.9),
    (95.4, 333.9),
    (117.0, 333.9),
    (138.0, 450.0),
    (151.2, 39.6),
    (172.8, 450.0),
    (187.2, 39.6),
    (208.5, 450.0),
    (221.4, 39.6),
    (243.0, 450.0),
    (257.4, 39.6),
    (279.0, 333.9),
    (297.0, 333.9),
    (318.6, 338.4),
    (340.2, 338.4),
    (358.5, 428.4),
    (379.8, 459.0),
    (394.2, 23.4),
    (415.8, 450.0),
    (429.0, 39.6),
    (450.0, 448.2),
    (464.4, 23.4),
    (486.0, 338.4),
    (505.8, 338.4),
    (525.6, 338.4),
    (547.2, 338.4),
    (568.5, 338.4),
];

#[kernpage_samples::test]
fn wrap_splits_rows_around_the_square_and_narrows_beside_the_oval() {
    let doc = pub_format::read(&sample("gen/wrap.pub")).unwrap();
    let shape = doc.pages[0]
        .shapes
        .iter()
        .position(|s| !s.paragraphs.is_empty())
        .unwrap();
    let rows = pub2pdf::row_layout(&doc, 0, shape, &Options::default());
    let mut worst = 0.0f32;
    for (i, (top, _)) in WRAP_LINES.iter().enumerate() {
        worst = worst.max((rows[i].top - top).abs());
    }
    // Rows beside the square (frame 144-252 pt grown by 2.88 pt) keep out of x 213.1-362.9
    // pt; at least one row uses a piece on each side.
    let square: Vec<&pub2pdf::RowInfo> = rows
        .iter()
        .filter(|r| r.top + 12.0 > 141.1 && r.top < 254.9)
        .collect();
    assert!(!square.is_empty());
    for r in &square {
        for &(x, w) in &r.pieces {
            assert!(x + w <= 213.2 || x >= 362.8, "{r:?}");
        }
    }
    assert!(square.iter().any(|r| r.pieces.len() == 2));
    // Where Publisher's row spans both pieces (450 pt), ours does too.
    for (i, (_, w)) in WRAP_LINES.iter().enumerate() {
        if *w >= 428.0 {
            assert_eq!(rows[i].pieces.len(), 2, "row {i}: {:?}", rows[i]);
        }
        let ours = rows[i].text_right - rows[i].left;
        // Publisher's width includes the paragraph mark / trailing space (about 6 pt).
        assert!((ours - w).abs() <= 12.0, "row {i}: width {ours} vs {w}");
    }
    // Beside the oval (frame 360-468 pt) pieces stay narrower than the box.
    let oval: Vec<&pub2pdf::RowInfo> = rows
        .iter()
        .filter(|r| r.top > 362.0 && r.top + 12.0 < 466.0)
        .collect();
    assert!(oval.iter().any(|r| r.pieces.len() == 2), "{oval:?}");
    // The rectangle without wrap (y 576-648 pt) has no effect.
    for r in rows.iter().filter(|r| r.top > 580.0 && r.top < 640.0) {
        assert_eq!(r.pieces.len(), 1, "{r:?}");
    }
    // Measured: the worst difference is 1.6 pt (row 11).
    assert!(worst <= 2.0, "worst line-top difference {worst}");
}

/// The text of a box starts at the box's own left margin, not at a fixed inset.
#[kernpage_samples::test]
fn text_starts_at_the_boxs_own_margin() {
    let (mut doc, pdf) = render_gen("formatting");
    let first_x = |pdf: &[u8]| -> f32 {
        let text = String::from_utf8_lossy(pdf);
        let at = text.find("(Arial 12)").unwrap();
        let line = text[..at].lines().rev().find(|l| l.ends_with(" Td")).unwrap();
        line.split_whitespace().next().unwrap().parse().unwrap()
    };
    let before = first_x(&pdf);
    // The generated box has Publisher's own 0.04 in margin.
    assert_eq!(doc.pages[0].shapes[0].inset_emu, [36_576; 4]);
    doc.pages[0].shapes[0].inset_emu[0] = 12_700 * 20; // 20 pt
    let after = first_x(&render(&doc, &std_opts()));
    assert!((after - before - (20.0 - 2.88)).abs() < 0.05, "{before} -> {after}");
}

/// The column width (and so the wrapping width) comes from the box's own left and right
/// margins: a wider right margin narrows the single column by exactly that much.
#[kernpage_samples::test]
fn layout_width_uses_the_boxs_margins() {
    let mut doc = pub_format::read(&sample("gen/formatting.pub")).unwrap();
    let wide = |doc: &pub_format::Document| {
        let g = pub2pdf::FrameGeom::of_shape(doc, 0, 0).unwrap();
        g.size.0 - g.inset_pt[0] - g.inset_pt[2]
    };
    let before = wide(&doc);
    doc.pages[0].shapes[0].inset_emu[2] += 12_700 * 10; // 10 pt more on the right
    assert!((before - wide(&doc) - 10.0).abs() < 1e-3);
}

/// Pages of a PDF in order, each with the text it shows (the `( ... ) Tj` strings joined).
fn page_texts(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf).into_owned();
    // Each page's content is one stream; streams hold the `Tj` operators.
    text.split("stream
")
        .filter(|chunk| chunk.contains(" Tj"))
        .map(|chunk| {
            chunk
                .lines()
                .filter_map(|l| l.strip_suffix(") Tj").and_then(|l| l.split_once('(')))
                .map(|(_, t)| t)
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

#[kernpage_samples::test]
fn master_shapes_are_drawn_on_every_page() {
    let (doc, pdf) = render_gen("master-box-2p");
    assert_eq!(doc.pages.len(), 2);
    assert_eq!(count(&pdf, b"(MASTERTEXT) Tj"), 2, "once per page");
}

#[kernpage_samples::test]
fn ignore_master_page_does_not_draw_the_master() {
    let (_, pdf) = render_gen("master-box-2p-ignore");
    assert_eq!(count(&pdf, b"(MASTERTEXT) Tj"), 1, "only page 1");
}

#[kernpage_samples::test]
fn page_number_field_shows_each_pages_own_number() {
    let (_, pdf) = render_gen("master-pn-text-2p");
    let texts = page_texts(&pdf);
    assert_eq!(texts.len(), 2, "{texts:?}");
    assert!(texts[0].contains('1') && !texts[0].contains('#'), "{texts:?}");
    assert!(texts[1].contains('2') && !texts[1].contains('#'), "{texts:?}");
}

#[kernpage_samples::test]
fn page_number_field_uses_section_number_and_format() {
    let mut doc = pub_format::read(&sample("gen/master-pn-2p.pub")).unwrap();
    doc.sections = vec![pub_format::Section {
        start_page: 1,
        first_number: Some(4),
        format: pub_format::NumberFormat::LowerRoman,
        hide_first_page_hf: false,
    }];
    let texts = page_texts(&render(&doc, &std_opts()));
    assert_eq!(texts, ["iv", "v"]);
}
