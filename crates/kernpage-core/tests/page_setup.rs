//! Page Design > Page Setup (size, orientation, margins): reading the margins, and saving
//! the changes into opened files and new publications.
//!
//! `cargo test -p kernpage-core --test page_setup -- --ignored write_publisher_check_files`
//! writes the saved files to open in Publisher to `PAGESAVE_OUT_DIR` (default: the
//! `pw-pagesave` folder under the system temp folder).

use kernpage_core::geometry::in_to_emu;
use kernpage_core::pagesetup::{MARGIN_PRESETS, PAGE_SIZES};
use kernpage_core::picture::sniff_format;
use kernpage_core::{
    from_pub_flowed, Command, Document, Editor, Margins, Paragraph, PictureFile, Rect, SaveSource,
    Shape, ShapeKind,
};
use std::sync::Arc;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn sample(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/gen").join(format!("{name}.pub")))).unwrap()
}
fn asset(name: &str) -> Vec<u8> {
    std::fs::read(root().join("tools/assets").join(name)).unwrap()
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    (
        SaveSource::new(bytes.to_vec(), &src, &model, &breaks),
        Editor::new(model),
    )
}

fn reopen(bytes: &[u8]) -> Document {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    from_pub_flowed("t.pub", &src, &breaks)
}

fn save(source: &mut SaveSource, doc: &Document) -> Vec<u8> {
    let plan = source.plan(doc);
    assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
    let (bytes, plan) = source.build(doc).unwrap();
    source.mark_saved(bytes.clone(), doc, &plan);
    assert!(
        source.plan(doc).is_empty(),
        "still unsaved: {:?}",
        source.plan(doc)
    );
    bytes
}

fn pt(v: f64) -> i64 {
    (v * 12_700.0).round() as i64
}

fn manifest() -> serde_json::Value {
    let text = std::fs::read_to_string(root().join("samples/gen/manifest.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn set_size(ed: &mut Editor, w: i64, h: i64) {
    ed.execute(Command::SetPageSize {
        width_emu: w,
        height_emu: h,
    })
    .unwrap();
}

#[kernpage_samples::test]
fn margins_and_size_are_read_from_every_page_sample() {
    let m = manifest();
    let mut seen = 0;
    for (name, v) in m.as_object().unwrap() {
        if !name.starts_with("page-") || v.get("margins_pt").is_none() {
            continue;
        }
        let Some(bytes) = kernpage_samples::read_opt(&format!("gen/{name}.pub")) else {
            continue;
        };
        let doc = reopen(&bytes);
        let g = |k: &str| v["margins_pt"][k].as_f64().unwrap();
        let want = Margins {
            left: pt(g("left")),
            right: pt(g("right")),
            top: pt(g("top")),
            bottom: pt(g("bottom")),
        };
        let page = &doc.pages[0];
        assert_eq!(page.margins_emu, want, "{name}");
        let (w, h) = (
            v["page_width_pt"].as_f64().unwrap(),
            v["page_height_pt"].as_f64().unwrap(),
        );
        assert!((page.width_emu - pt(w)).abs() <= 1, "{name} width");
        assert!((page.height_emu - pt(h)).abs() <= 1, "{name} height");
        seen += 1;
    }
    let want = if kernpage_samples::extra_samples().is_some() { 20 } else { 1 };
    assert!(seen >= want, "only {seen} samples checked");
}

#[kernpage_samples::test]
fn presets_match_the_galleries() {
    assert_eq!(PAGE_SIZES.len(), 9);
    let texts: Vec<String> = PAGE_SIZES.iter().map(|p| p.size_text(false)).collect();
    assert_eq!(texts[0], "11.693 x 16.535\"");
    assert_eq!(texts[1], "8.268 x 11.693\"");
    assert_eq!(texts[2], "5.827 x 8.268\"");
    assert_eq!(texts[3], "9.843 x 13.898\"");
    assert_eq!(texts[4], "6.929 x 9.843\"");
    assert_eq!(texts[5], "7.25 x 10.5\"");
    assert_eq!(texts[6], "8.5 x 14\"");
    assert_eq!(texts[7], "8.5 x 11\"");
    assert_eq!(texts[8], "11 x 17\"");
    assert_eq!(PAGE_SIZES[6].size_text(true), "14 x 8.5\"");
    assert_eq!(PAGE_SIZES[1].label(true), "A4 (Landscape)");
    let names: Vec<_> = MARGIN_PRESETS.iter().map(|p| (p.name, p.text)).collect();
    assert_eq!(
        names,
        [
            ("Wide", "1\""),
            ("Moderate", "0.5\""),
            ("Narrow", "0.25\""),
            ("None", "0\"")
        ]
    );
    // Sizes read from Publisher files are recognised (A4 made with PageWidth = 595.3).
    let of = |name: &str| {
        let d = reopen(&sample(name));
        kernpage_core::pagesetup::preset_of(d.pages[0].width_emu, d.pages[0].height_emu)
    };
    assert_eq!(of("page-a4"), Some(1));
    assert_eq!(of("page-landscape"), Some(7));
    assert_eq!(of("page-custom-5x7"), None);
}

/// (left, top) of every shape of page `page`, EMU from the page's top-left.
fn positions(doc: &Document, page: usize) -> Vec<(i64, i64)> {
    doc.pages[page]
        .shapes
        .iter()
        .map(|s| (s.rect.x, s.rect.y))
        .collect()
}

#[kernpage_samples::test]
fn size_orientation_and_margins_round_trip_through_an_opened_file() {
    for (name, (w, h)) in [
        ("a4", (7_560_000, 10_692_000)),
        ("legal landscape", (12_801_600, 7_772_400)),
        ("tabloid", (10_058_400, 15_544_800)),
    ] {
        let bytes = sample("page-base");
        let (mut source, mut ed) = open(&bytes);
        let before = ed.doc().clone();
        set_size(&mut ed, w, h);
        ed.execute(Command::SetMargins {
            margins: MARGIN_PRESETS[0].margins,
        })
        .unwrap();
        assert!(source.plan(ed.doc()).page_setup.is_some());
        let saved = save(&mut source, ed.doc());
        let back = reopen(&saved);
        let p = &back.pages[0];
        assert_eq!((p.width_emu, p.height_emu), (w, h), "{name}");
        assert_eq!(p.margins_emu, Margins::all(914_400), "{name}");
        // The shapes are where the editor showed them (within 1 EMU of rounding).
        for (a, b) in positions(&back, 0).iter().zip(positions(ed.doc(), 0)) {
            assert!(
                (a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1,
                "{name}: {a:?} vs {b:?}"
            );
        }
        assert_eq!(
            back.pages[0].shapes.len(),
            before.pages[0].shapes.len(),
            "{name}"
        );
        // A second save with no further change has nothing to do, and a further edit
        // (here: another size) still saves against the new file.
        assert!(source.plan(ed.doc()).is_empty());
        set_size(&mut ed, 7_772_400, 10_058_400);
        let again = save(&mut source, ed.doc());
        let back2 = reopen(&again);
        assert_eq!(
            (back2.pages[0].width_emu, back2.pages[0].height_emu),
            (7_772_400, 10_058_400)
        );
        for (a, b) in positions(&back2, 0).iter().zip(positions(&before, 0)) {
            assert!(
                (a.0 - b.0).abs() <= 2 && (a.1 - b.1).abs() <= 2,
                "{name} back: {a:?} vs {b:?}"
            );
        }
    }
}

#[kernpage_samples::test]
fn a_shape_edit_and_a_size_change_save_together() {
    let (mut source, mut ed) = open(&sample("page-base"));
    let id = ed.doc().pages[0].shapes[0].id;
    set_size(&mut ed, 7_560_000, 10_692_000);
    let before = ed.doc().pages[0].shapes[0].rect;
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: vec![id],
        dx: 100_000,
        dy: 50_000,
    })
    .unwrap();
    let saved = save(&mut source, ed.doc());
    let back = reopen(&saved);
    let r = back.pages[0].shapes[0].rect;
    assert!(
        (r.x - (before.x + 100_000)).abs() <= 1,
        "{r:?} vs {before:?}"
    );
    assert!((r.y - (before.y + 50_000)).abs() <= 1);
    assert_eq!(back.pages[0].width_emu, 7_560_000);
}

#[kernpage_samples::test]
fn shapes_pushed_off_the_page_save_and_stay_saved() {
    let (mut source, mut ed) = open(&sample("page-base-edge"));
    let n = ed.doc().pages[0].shapes.len();
    set_size(&mut ed, 4_572_000, 6_400_800);
    let saved = save(&mut source, ed.doc());
    let back = reopen(&saved);
    assert_eq!(
        (back.pages[0].width_emu, back.pages[0].height_emu),
        (4_572_000, 6_400_800)
    );
    assert!(
        back.pages[0].shapes.len() < n,
        "some shapes left for the scratch area"
    );
    // The editor still has them, and a later save does not add or duplicate anything.
    assert_eq!(ed.doc().pages[0].shapes.len(), n);
    assert!(source.plan(ed.doc()).is_empty());
    let id = ed.doc().pages[0].shapes[0].id;
    ed.execute(Command::MoveShapes {
        page: 0,
        ids: vec![id],
        dx: 12_700,
        dy: 0,
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert!(
        plan.new_shapes.is_empty()
            && plan.new_text_boxes.is_empty()
            && plan.new_pictures.is_empty()
    );
}

#[kernpage_samples::test]
fn a_refused_size_change_is_reported_not_fatal() {
    // Facing-page masters are refused by the writer.
    let (source, mut ed) = open(&sample("page-margins-twopage-master"));
    set_size(&mut ed, 7_560_000, 10_692_000);
    let (bytes, plan) = source.build(ed.doc()).unwrap();
    assert!(plan.page_setup.is_none());
    assert!(
        plan.unsupported.iter().any(|u| u.contains("page size")),
        "{:?}",
        plan.unsupported
    );
    let back = reopen(&bytes);
    assert_eq!(back.pages[0].width_emu, 7_772_400);
}

fn new_doc(w: i64, h: i64) -> Document {
    Document::blank("new", w, h)
}

#[kernpage_samples::test]
fn new_publication_a4_landscape_narrow_saves_and_reopens() {
    let mut ed = Editor::new(new_doc(in_to_emu(8.5), in_to_emu(11.0)));
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::text_box(
            Rect::new(
                in_to_emu(1.0),
                in_to_emu(1.0),
                in_to_emu(3.0),
                in_to_emu(1.0),
            ),
            vec![Paragraph::plain("Hello")],
        ),
    })
    .unwrap();
    set_size(&mut ed, 10_692_000, 7_560_000);
    ed.execute(Command::SetMargins {
        margins: MARGIN_PRESETS[2].margins,
    })
    .unwrap();
    let mut source = SaveSource::for_new(ed.doc()).unwrap();
    let saved = save(&mut source, ed.doc());
    let back = reopen(&saved);
    assert_eq!(
        (back.pages[0].width_emu, back.pages[0].height_emu),
        (10_692_000, 7_560_000)
    );
    assert_eq!(back.pages[0].margins_emu, Margins::all(228_600));
    let r = back.pages[0].shapes[0].rect;
    let want = ed.doc().pages[0].shapes[0].rect;
    assert!(
        (r.x - want.x).abs() <= 1 && (r.y - want.y).abs() <= 1,
        "{r:?} vs {want:?}"
    );
}

fn picture() -> (Vec<u8>, Shape) {
    let png = asset("test-picture.png");
    let format = sniff_format(&png);
    let shape = Shape::new(
        Rect::new(
            in_to_emu(1.0),
            in_to_emu(2.0),
            in_to_emu(2.0),
            in_to_emu(1.5),
        ),
        ShapeKind::Picture {
            data: Arc::new(png.clone()),
            format,
            file: Some(PictureFile {
                name: "test-picture.png".into(),
                modified: Some(132_000_000_000_000_000),
            }),
        },
    );
    (png, shape)
}

#[kernpage_samples::test]
fn page_setup_and_a_picture_insert_save_in_one_go() {
    let (png, pic) = picture();
    // Into an opened file.
    let (mut source, mut ed) = open(&sample("page-base"));
    ed.execute(Command::AddShape {
        page: 0,
        shape: pic.clone(),
    })
    .unwrap();
    set_size(&mut ed, 7_560_000, 10_692_000);
    ed.execute(Command::SetMargins {
        margins: Margins::all(0),
    })
    .unwrap();
    let saved = save(&mut source, ed.doc());
    let back = pub_format::read_bytes(&saved).unwrap();
    assert_eq!(back.page_width_emu, 7_560_000);
    let p = back.pages[0]
        .shapes
        .iter()
        .find_map(|s| s.picture.as_ref())
        .expect("picture");
    assert_eq!(p.bytes, png);
    let m = reopen(&saved);
    let find = |d: &Document| {
        d.pages[0]
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Picture { .. }))
            .unwrap()
            .rect
    };
    // Added at (1 in, 2 in) on Letter, then the page shrank: it follows the page centre.
    let (r, want) = (find(&m), find(ed.doc()));
    assert_eq!(want.x, in_to_emu(1.0) + 7_560_000 / 2 - 7_772_400 / 2);
    assert!((r.x - want.x).abs() <= 1 && (r.y - want.y).abs() <= 1, "{r:?} vs {want:?}");
    assert_eq!(m.pages[0].margins_emu, Margins::all(0));
    // Into a new publication.
    let mut ed = Editor::new(new_doc(in_to_emu(8.5), in_to_emu(11.0)));
    ed.execute(Command::AddShape {
        page: 0,
        shape: pic,
    })
    .unwrap();
    set_size(&mut ed, 10_692_000, 7_560_000);
    let mut source = SaveSource::for_new(ed.doc()).unwrap();
    let saved = save(&mut source, ed.doc());
    let m = reopen(&saved);
    assert_eq!(m.pages[0].width_emu, 10_692_000);
    assert!(m.pages[0]
        .shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Picture { .. })));
}

#[kernpage_samples::test]
fn page_setup_dialog_result_saves_and_reopens_new_publication() {
    let mut ed = Editor::new(new_doc(in_to_emu(8.5), in_to_emu(11.0)));
    let m = Margins {
        left: in_to_emu(0.75),
        right: in_to_emu(0.5),
        top: in_to_emu(1.0),
        bottom: in_to_emu(0.25),
    };
    ed.execute(Command::SetPageSetup {
        width_emu: in_to_emu(5.5),
        height_emu: in_to_emu(8.5),
        margins: m,
    })
    .unwrap();
    assert_eq!(ed.undo_label(), Some("Page Setup"));
    let mut source = SaveSource::for_new(ed.doc()).unwrap();
    let back = reopen(&save(&mut source, ed.doc()));
    assert_eq!(
        (back.pages[0].width_emu, back.pages[0].height_emu),
        (in_to_emu(5.5), in_to_emu(8.5))
    );
    assert_eq!(back.pages[0].margins_emu, m);
    // The Size gallery then shows "Current" for it: no preset matches.
    assert_eq!(
        kernpage_core::pagesetup::preset_of(back.pages[0].width_emu, back.pages[0].height_emu),
        None
    );
}

#[kernpage_samples::test]
fn page_setup_dialog_result_saves_into_an_opened_file() {
    let (mut source, mut ed) = open(&sample("page-base"));
    ed.execute(Command::SetPageSetup {
        width_emu: in_to_emu(5.5),
        height_emu: in_to_emu(8.5),
        margins: Margins::all(in_to_emu(0.25)),
    })
    .unwrap();
    let back = reopen(&save(&mut source, ed.doc()));
    assert_eq!(
        (back.pages[0].width_emu, back.pages[0].height_emu),
        (in_to_emu(5.5), in_to_emu(8.5))
    );
    assert_eq!(back.pages[0].margins_emu, Margins::all(in_to_emu(0.25)));
}

#[kernpage_samples::test]
#[ignore = "writes files for Publisher to open"]
fn write_publisher_check_files() {
    let out = std::env::var("PAGESAVE_OUT_DIR").unwrap_or_else(|_| {
        std::env::temp_dir().join("pw-pagesave").to_string_lossy().into_owned().into()
    });
    std::fs::create_dir_all(&out).unwrap();
    let put = |name: &str, bytes: &[u8]| {
        std::fs::write(std::path::Path::new(&out).join(name), bytes).unwrap()
    };
    let opened = |sample_name: &str, f: &dyn Fn(&mut Editor)| {
        let (source, mut ed) = open(&sample(sample_name));
        f(&mut ed);
        let plan = source.plan(ed.doc());
        assert!(plan.unsupported.is_empty(), "{:?}", plan.unsupported);
        source.build(ed.doc()).unwrap().0
    };
    put(
        "opened-a4.pub",
        &opened("page-base", &|e| set_size(e, 7_560_000, 10_692_000)),
    );
    put(
        "opened-legal-landscape.pub",
        &opened("page-base", &|e| set_size(e, 12_801_600, 7_772_400)),
    );
    put(
        "opened-margins-wide.pub",
        &opened("page-base", &|e| {
            e.execute(Command::SetMargins {
                margins: MARGIN_PRESETS[0].margins,
            })
            .unwrap();
        }),
    );
    put(
        "opened-shrunk-shape-off-page.pub",
        &opened("page-base-edge", &|e| set_size(e, 4_572_000, 6_400_800)),
    );
    let mut ed = Editor::new(new_doc(in_to_emu(8.5), in_to_emu(11.0)));
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::text_box(
            Rect::new(
                in_to_emu(0.75),
                in_to_emu(0.75),
                in_to_emu(3.0),
                in_to_emu(1.0),
            ),
            vec![Paragraph::plain("A5 with custom margins")],
        ),
    })
    .unwrap();
    set_size(&mut ed, 5_328_000, 7_560_000);
    ed.execute(Command::SetMargins {
        margins: Margins {
            left: 685_800,
            right: 457_200,
            top: 914_400,
            bottom: 228_600,
        },
    })
    .unwrap();
    let source = SaveSource::for_new(ed.doc()).unwrap();
    put("new-a5-custom-margins.pub", &source.build(ed.doc()).unwrap().0);
    // A folded bulletin typed into the Page Setup dialog: 5.5 x 8.5 in with custom margins.
    let bulletin = Command::SetPageSetup {
        width_emu: in_to_emu(5.5),
        height_emu: in_to_emu(8.5),
        margins: Margins {
            left: in_to_emu(0.75),
            right: in_to_emu(0.5),
            top: in_to_emu(1.0),
            bottom: in_to_emu(0.25),
        },
    };
    let mut ed = Editor::new(new_doc(in_to_emu(8.5), in_to_emu(11.0)));
    ed.execute(Command::AddShape {
        page: 0,
        shape: Shape::text_box(
            Rect::new(in_to_emu(0.75), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(1.0)),
            vec![Paragraph::plain("5.5 x 8.5 bulletin")],
        ),
    })
    .unwrap();
    ed.execute(bulletin.clone()).unwrap();
    let source = SaveSource::for_new(ed.doc()).unwrap();
    put("new-5.5x8.5-custom.pub", &source.build(ed.doc()).unwrap().0);
    put(
        "opened-5.5x8.5.pub",
        &opened("page-base", &|e| {
            e.execute(bulletin.clone()).unwrap();
        }),
    );
    println!("written to {out}");
}
