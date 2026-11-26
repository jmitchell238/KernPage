//! Saving pictures: inserted (into an opened file and into a publication made in the editor),
//! changed (Change Picture) and deleted.
//!
//! `cargo test -p kernpage-core --test save_picture -- --ignored write_publisher_check_files`
//! writes the saved files to open in Publisher to `PICSAVE_OUT_DIR` (default: the
//! `pw-picsave` folder under the system temp folder).

use kernpage_core::geometry::in_to_emu;
use kernpage_core::picture::{change_rect, image_info, sniff_format};
use kernpage_core::{
    from_pub_flowed, Command, Document, Editor, PictureFile, Rect, SaveSource, Shape, ShapeId,
    ShapeKind,
};
use std::sync::Arc;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn asset(name: &str) -> Vec<u8> {
    std::fs::read(root().join("tools/assets").join(name)).unwrap()
}
fn sample(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join("samples/gen").join(name))).unwrap()
}

fn open(bytes: &[u8]) -> (SaveSource, Editor) {
    let src = pub_format::read_bytes(bytes).unwrap();
    let breaks = pub2pdf::chain_breaks(&src, &pub2pdf::Options::default());
    let model = from_pub_flowed("t.pub", &src, &breaks);
    let source = SaveSource::new(bytes.to_vec(), &src, &model, &breaks);
    (source, Editor::new(model))
}

fn picture_shape(bytes: Vec<u8>, name: &str, rect: Rect) -> Shape {
    let format = sniff_format(&bytes);
    Shape::new(
        rect,
        ShapeKind::Picture {
            data: Arc::new(bytes),
            format,
            file: Some(PictureFile {
                name: name.to_owned(),
                modified: Some(132_000_000_000_000_000),
            }),
        },
    )
}

fn add(ed: &mut Editor, page: usize, s: Shape) -> ShapeId {
    ed.execute(Command::AddShape { page, shape: s }).unwrap()[0]
}

/// The pictures of a page of the saved file: (image bytes, frame in EMU from the top-left).
fn pictures(bytes: &[u8], page: usize) -> Vec<(Vec<u8>, Rect)> {
    let d = pub_format::read_bytes(bytes).unwrap();
    let (cx, cy) = (
        i64::from(d.page_width_emu) / 2,
        i64::from(d.page_height_emu) / 2,
    );
    d.pages[page]
        .shapes
        .iter()
        .filter_map(|s| {
            let p = s.picture.as_ref()?;
            Some((
                p.bytes.clone(),
                Rect::new(
                    cx + i64::from(s.left_emu),
                    cy + i64::from(s.top_emu),
                    i64::from(s.right_emu) - i64::from(s.left_emu),
                    i64::from(s.bottom_emu) - i64::from(s.top_emu),
                ),
            ))
        })
        .collect()
}

fn first_picture(ed: &Editor) -> (usize, ShapeId) {
    for (pi, p) in ed.doc().pages.iter().enumerate() {
        if let Some(s) = p
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Picture { .. }))
        {
            return (pi, s.id);
        }
    }
    panic!("no picture");
}

fn pic_data(doc: &Document, page: usize, id: ShapeId) -> Vec<u8> {
    match &doc.shape(page, id).unwrap().kind {
        ShapeKind::Picture { data, .. } => data.as_ref().clone(),
        _ => panic!("not a picture"),
    }
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

fn new_doc() -> Document {
    let p = kernpage_core::Page::blank(in_to_emu(8.5), in_to_emu(11.0));
    Document::new("new", vec![p])
}

#[kernpage_samples::test]
fn picture_in_a_new_publication_saves_and_resaves() {
    let png = asset("test-picture.png");
    let jpg = asset("green8x6.jpg");
    let mut ed = Editor::new(new_doc());
    let r1 = Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(2.0));
    let r2 = Rect::new(in_to_emu(4.0), in_to_emu(5.0), in_to_emu(2.0), in_to_emu(1.5));
    let a = add(&mut ed, 0, picture_shape(png.clone(), "test-picture.png", r1));
    let rect = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(7.0), in_to_emu(2.0), in_to_emu(1.0)),
        ShapeKind::Rectangle,
    );
    add(&mut ed, 0, rect);
    let b = add(&mut ed, 0, picture_shape(jpg.clone(), "green8x6.jpg", r2));

    let mut source = SaveSource::for_new(ed.doc()).unwrap();
    let plan = source.plan(ed.doc());
    assert_eq!(plan.new_pictures.len(), 2);
    assert_eq!(plan.new_shapes.len(), 1);
    let first = save(&mut source, ed.doc());
    let pics = pictures(&first, 0);
    assert_eq!(pics.len(), 2);
    assert_eq!(pics[0].0, png);
    assert_eq!(pics[1].0, jpg);
    assert_eq!(pics[0].1, r1);
    assert_eq!(pics[1].1, r2);
    assert_eq!(
        pub_format::read_bytes(&first).unwrap().pages[0].shapes.len(),
        3
    );

    // Resave: move the first picture, replace the second one's image.
    let moved = Rect::new(in_to_emu(2.0), in_to_emu(2.0), r1.w, r1.h);
    ed.execute(Command::SetRect {
        page: 0,
        id: a,
        rect: moved,
    })
    .unwrap();
    let new_img = asset("red4x3.png");
    let rect2 = change_rect(r2, image_info(&new_img).unwrap());
    ed.execute(Command::ChangePicture {
        page: 0,
        id: b,
        data: Arc::new(new_img.clone()),
        format: sniff_format(&new_img),
        file: Some(PictureFile {
            name: "red4x3.png".into(),
            modified: None,
        }),
        rect: rect2,
    })
    .unwrap();
    let second = save(&mut source, ed.doc());
    let pics = pictures(&second, 0);
    assert_eq!(pics.len(), 2);
    assert_eq!((pics[0].0.clone(), pics[0].1), (png.clone(), moved));
    assert_eq!((pics[1].0.clone(), pics[1].1), (new_img.clone(), rect2));

    // And delete the first one.
    ed.execute(Command::DeleteShapes {
        page: 0,
        ids: vec![a],
    })
    .unwrap();
    let third = save(&mut source, ed.doc());
    let pics = pictures(&third, 0);
    assert_eq!(pics.len(), 1);
    assert_eq!((pics[0].0.clone(), pics[0].1), (new_img, rect2));
    assert_eq!(
        pub_format::read_bytes(&third).unwrap().pages[0].shapes.len(),
        2
    );
}

#[kernpage_samples::test]
fn picture_inserted_into_an_opened_file_saves() {
    let bytes = sample("bulletin.pub");
    let before = pub_format::read_bytes(&bytes).unwrap();
    let png = asset("grad40x30.png");
    for page in 0..2 {
        let (mut source, mut ed) = open(&bytes);
        let r = Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(2.0), in_to_emu(1.5));
        add(&mut ed, page, picture_shape(png.clone(), "grad40x30.png", r));
        let out = save(&mut source, ed.doc());
        let pics = pictures(&out, page);
        assert_eq!(pics.last().unwrap(), &(png.clone(), r));
        let d = pub_format::read_bytes(&out).unwrap();
        assert_eq!(
            d.pages[page].shapes.len(),
            before.pages[page].shapes.len() + 1
        );
        // The rest of the file is read back as before.
        let (_, ed2) = open(&out);
        assert_eq!(
            ed2.doc().pages[page].shapes.len(),
            ed.doc().pages[page].shapes.len()
        );
    }
}

#[kernpage_samples::test]
fn change_picture_swaps_the_image_with_the_aspect_rule() {
    let (mut source, mut ed) = open(&sample("picture.pub"));
    let (page, id) = first_picture(&ed);
    let old = ed.doc().shape(page, id).unwrap().rect;
    let img = asset("green8x6.jpg");
    let rect = change_rect(old, image_info(&img).unwrap());
    assert_eq!(rect.h, old.h);
    assert_eq!(rect.x + rect.w / 2, old.x + old.w / 2);
    ed.execute(Command::ChangePicture {
        page,
        id,
        data: Arc::new(img.clone()),
        format: sniff_format(&img),
        file: Some(PictureFile {
            name: "green8x6.jpg".into(),
            modified: None,
        }),
        rect,
    })
    .unwrap();
    let plan = source.plan(ed.doc());
    assert_eq!(plan.picture_changes.len(), 1);
    let out = save(&mut source, ed.doc());
    let pics = pictures(&out, page);
    assert!(pics.iter().any(|(b, r)| *b == img && *r == rect), "{pics:?}");
    // Reopened: same image and frame, and undo of the editor restores the old image.
    let (_, ed2) = open(&out);
    let (p2, id2) = first_picture(&ed2);
    assert_eq!(pic_data(ed2.doc(), p2, id2), img);
    assert!(ed.undo());
    assert_ne!(pic_data(ed.doc(), page, id), img);
    // Saving the undone state writes the old image back.
    let back = save(&mut source, ed.doc());
    assert!(!pictures(&back, page).iter().any(|(b, _)| *b == img));
}

#[kernpage_samples::test]
fn delete_picture_removes_it_and_keeps_the_rest() {
    let bytes = sample("picture-over-text.pub");
    let before = pub_format::read_bytes(&bytes).unwrap();
    let (mut source, mut ed) = open(&bytes);
    let (page, id) = first_picture(&ed);
    let others = ed.doc().pages[page].shapes.len() - 1;
    ed.execute(Command::DeleteShapes {
        page,
        ids: vec![id],
    })
    .unwrap();
    // A frame move of a surviving shape in the same save keeps working.
    let other = ed.doc().pages[page].shapes.last().unwrap().id;
    ed.execute(Command::MoveShapes {
        page,
        ids: vec![other],
        dx: 12_700,
        dy: 0,
    })
    .unwrap();
    let out = save(&mut source, ed.doc());
    assert!(pictures(&out, page).is_empty());
    let d = pub_format::read_bytes(&out).unwrap();
    assert_eq!(d.pages[page].shapes.len(), before.pages[page].shapes.len() - 1);
    assert_eq!(others, d.pages[page].shapes.len());
    let (_, ed2) = open(&out);
    assert_eq!(ed2.doc().pages[page].shapes.len(), others);
}

#[kernpage_samples::test]
fn add_change_and_delete_in_one_save_then_resave() {
    let (mut source, mut ed) = open(&sample("picture-two.pub"));
    let pics_before: Vec<_> = ed
        .doc()
        .pages
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| {
            p.shapes
                .iter()
                .filter(|s| matches!(s.kind, ShapeKind::Picture { .. }))
                .map(move |s| (pi, s.id))
        })
        .collect();
    assert!(pics_before.len() >= 2);
    let (p0, a) = pics_before[0];
    let (p1, b) = pics_before[1];
    assert_eq!(p0, p1, "the sample has both pictures on one page");
    let img = asset("red4x3.png");
    let old = ed.doc().shape(p0, a).unwrap().rect;
    let rect = change_rect(old, image_info(&img).unwrap());
    ed.execute(Command::ChangePicture {
        page: p0,
        id: a,
        data: Arc::new(img.clone()),
        format: sniff_format(&img),
        file: None,
        rect,
    })
    .unwrap();
    ed.execute(Command::DeleteShapes {
        page: p0,
        ids: vec![b],
    })
    .unwrap();
    let new_img = asset("blue5x5.png");
    let r = Rect::new(in_to_emu(1.0), in_to_emu(8.0), in_to_emu(1.0), in_to_emu(1.0));
    let n = add(&mut ed, p0, picture_shape(new_img.clone(), "blue5x5.png", r));
    let out = save(&mut source, ed.doc());
    let pics = pictures(&out, p0);
    assert!(pics.iter().any(|(b, r)| *b == img && *r == rect));
    assert!(pics.iter().any(|(b, rr)| *b == new_img && *rr == r));
    assert_eq!(pics.len(), pics_before.len());
    // Resave after moving the new picture and the changed one.
    let moved = Rect::new(in_to_emu(2.0), in_to_emu(8.0), r.w, r.h);
    ed.execute(Command::SetRect {
        page: p0,
        id: n,
        rect: moved,
    })
    .unwrap();
    ed.execute(Command::MoveShapes {
        page: p0,
        ids: vec![a],
        dx: 12_700,
        dy: 12_700,
    })
    .unwrap();
    let out2 = save(&mut source, ed.doc());
    let pics = pictures(&out2, p0);
    assert!(pics.iter().any(|(b, rr)| *b == new_img && *rr == moved));
    assert!(pics
        .iter()
        .any(|(b, rr)| *b == img && *rr == Rect::new(rect.x + 12_700, rect.y + 12_700, rect.w, rect.h)));
}

#[kernpage_samples::test]
fn unsupported_pictures_are_reported() {
    let png = asset("red4x3.png");
    let mut ed = Editor::new(new_doc());
    let mut s = picture_shape(png.clone(), "red4x3.png", Rect::new(100, 100, 1000, 1000));
    s.rotation_deg = 30.0;
    add(&mut ed, 0, s);
    let source = SaveSource::for_new(ed.doc()).unwrap();
    let plan = source.plan(ed.doc());
    assert_eq!(
        plan.unsupported,
        vec!["new pictures that are rotated or grouped".to_owned()]
    );
    assert!(plan.new_pictures.is_empty());
}

/// Writes the saved files for a check in Publisher 2021.
#[kernpage_samples::test]
#[ignore]
fn write_publisher_check_files() {
    let dir = std::env::var("PICSAVE_OUT_DIR").unwrap_or_else(|_| {
        std::env::temp_dir().join("pw-picsave").to_string_lossy().into_owned().into()
    });
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let png = asset("test-picture.png");
    let jpg = asset("green8x6.jpg");

    // 1. Two pictures and a rectangle in a publication made in the editor.
    let mut ed = Editor::new(new_doc());
    let r1 = Rect::new(in_to_emu(1.0), in_to_emu(1.0), in_to_emu(3.0), in_to_emu(2.0));
    let r2 = Rect::new(in_to_emu(4.0), in_to_emu(5.0), in_to_emu(2.0), in_to_emu(1.5));
    add(&mut ed, 0, picture_shape(png.clone(), "test-picture.png", r1));
    let rect = Shape::new(
        Rect::new(in_to_emu(1.0), in_to_emu(7.0), in_to_emu(2.0), in_to_emu(1.0)),
        ShapeKind::Rectangle,
    );
    add(&mut ed, 0, rect);
    let b = add(&mut ed, 0, picture_shape(jpg, "green8x6.jpg", r2));
    let mut source = SaveSource::for_new(ed.doc()).unwrap();
    let bytes = save(&mut source, ed.doc());
    std::fs::write(dir.join("1-new-publication-two-pictures.pub"), &bytes).unwrap();

    // 2. The same publication after Change Picture on the second picture (aspect rule).
    let img = asset("red4x3.png");
    let rect2 = change_rect(r2, image_info(&img).unwrap());
    ed.execute(Command::ChangePicture {
        page: 0,
        id: b,
        data: Arc::new(img.clone()),
        format: sniff_format(&img),
        file: Some(PictureFile {
            name: "red4x3.png".into(),
            modified: None,
        }),
        rect: rect2,
    })
    .unwrap();
    let bytes = save(&mut source, ed.doc());
    std::fs::write(dir.join("2-after-change-picture.pub"), &bytes).unwrap();

    // 3. Opened sample (picture.pub): picture changed; picture-two.pub: one deleted.
    let (mut source, mut ed) = open(&sample("picture.pub"));
    let (page, id) = first_picture(&ed);
    let old = ed.doc().shape(page, id).unwrap().rect;
    let jpg = asset("green8x6.jpg");
    let rect = change_rect(old, image_info(&jpg).unwrap());
    ed.execute(Command::ChangePicture {
        page,
        id,
        data: Arc::new(jpg.clone()),
        format: sniff_format(&jpg),
        file: Some(PictureFile {
            name: "green8x6.jpg".into(),
            modified: None,
        }),
        rect,
    })
    .unwrap();
    let bytes = save(&mut source, ed.doc());
    std::fs::write(dir.join("3-opened-picture-changed.pub"), &bytes).unwrap();

    let (mut source, mut ed) = open(&sample("picture-two.pub"));
    let (page, id) = first_picture(&ed);
    ed.execute(Command::DeleteShapes {
        page,
        ids: vec![id],
    })
    .unwrap();
    let new_img = asset("blue5x5.png");
    add(
        &mut ed,
        page,
        picture_shape(
            new_img,
            "blue5x5.png",
            Rect::new(in_to_emu(1.0), in_to_emu(8.0), in_to_emu(1.0), in_to_emu(1.0)),
        ),
    );
    let bytes = save(&mut source, ed.doc());
    std::fs::write(dir.join("4-opened-one-deleted-one-added.pub"), &bytes).unwrap();
    println!("wrote files to {}", dir.display());
}

/// Publisher 2021 (COM `Shapes.AddPicture(file, 0, -1, L, T)` with no size, on a 120 dpi
/// screen) gave these natural sizes in points for the test assets, which record no
/// resolution.
#[kernpage_samples::test]
fn natural_size_matches_publisher_for_the_test_assets() {
    use kernpage_core::picture::insert_rect;
    let page = (in_to_emu(8.5), in_to_emu(11.0));
    for (name, w_pt, h_pt) in [
        ("blue5x5.png", 3.0, 3.0),
        ("grad40x30.png", 24.0, 18.0),
        ("green8x6.jpg", 4.8, 3.6),
        ("red4x3.png", 2.4, 1.8),
        ("test-picture.png", 120.0, 72.0),
    ] {
        let info = image_info(&asset(name)).unwrap();
        let r = insert_rect(info, 120.0, page.0, page.1);
        assert!(
            (r.w as f64 / 12_700.0 - w_pt).abs() < 0.01 && (r.h as f64 / 12_700.0 - h_pt).abs() < 0.01,
            "{name}: {} x {} pt",
            r.w as f64 / 12_700.0,
            r.h as f64 / 12_700.0
        );
    }
}
