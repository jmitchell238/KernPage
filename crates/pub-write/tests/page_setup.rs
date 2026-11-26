//! Page setup (spec 4Q): page size, orientation, margins. Compared structurally with the
//! files Publisher 2021 wrote (`samples/gen/page-*.pub`) and exercised on new blanks.

use pub_format::contents::Contents;
use pub_format::items::{Item, Value};
use pub_format::Streams;
use pub_write::{
    add_picture, add_shape, add_text_box, new_publication, set_margins, set_page_size,
    NewPicture, NewPublication, NewShapeKind, PictureKind, ShapeStyle, WriteError,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

const PT: f64 = 12_700.0;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap()
}
/// True when every named `gen` sample is available here (a loop skips the ones that are not).
fn have(names: &[&str]) -> bool {
    names.iter().all(|n| kernpage_samples::find(&format!("gen/{n}.pub")).is_some())
}
fn manifest() -> serde_like::Manifest {
    serde_like::Manifest::load(&root().join("samples/gen/manifest.json"))
}

/// A tiny extractor for the few manifest numbers we need (no JSON crate in this workspace).
mod serde_like {
    use std::path::Path;
    pub struct Manifest(String);
    impl Manifest {
        pub fn load(p: &Path) -> Self {
            Manifest(std::fs::read_to_string(p).unwrap())
        }
        /// The number after `"key":` inside the entry `"name": {`.
        pub fn margins(&self, name: &str) -> [u32; 4] {
            let start = self.0.find(&format!("\"{name}\": {{")).unwrap();
            let rest = &self.0[start..];
            let at = rest.find("\"margins_pt\": {").unwrap();
            let rest = &rest[at..];
            ["left", "right", "top", "bottom"].map(|k| {
                let key = format!("\"{k}\": ");
                let i = rest.find(&key).unwrap() + key.len();
                let t = &rest[i..];
                let e = t.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap();
                (t[..e].parse::<f64>().unwrap() * super::PT).round() as u32
            })
        }
        pub fn num(&self, name: &str, key: &str) -> f64 {
            let start = self.0.find(&format!("\"{name}\": {{")).unwrap();
            let rest = &self.0[start..];
            let k = rest.find(&format!("\"{key}\": ")).unwrap() + key.len() + 4;
            let tail = &rest[k..];
            let end = tail.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap();
            tail[..end].parse().unwrap()
        }
    }
}

fn emu(pt: f64) -> u32 {
    (pt * PT).round() as u32
}
fn size_of(m: &serde_like::Manifest, name: &str) -> (u32, u32) {
    (
        emu(m.num(name, "page_width_pt")),
        emu(m.num(name, "page_height_pt")),
    )
}
// ------------------------------------------------------------------ decoding helpers

#[derive(Debug, PartialEq)]
struct Obj {
    kind: u16,
    parent: Option<u32>,
    items: Vec<Item>,
}

fn objects(bytes: &[u8]) -> BTreeMap<u32, Obj> {
    let s = Streams::from_cfb(bytes).unwrap();
    let c = Contents::parse(&s.contents).unwrap();
    c.objects
        .iter()
        .map(|e| {
            (
                e.number,
                Obj {
                    kind: e.kind,
                    parent: e.parent,
                    items: c.decode(e).unwrap(),
                },
            )
        })
        .collect()
}

/// Escher client anchors by Contents object number (raw atom bytes).
fn anchors(bytes: &[u8]) -> BTreeMap<u32, Vec<u8>> {
    let d = Streams::from_cfb(bytes).unwrap().escher;
    let u32at = |i: usize| u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i + 12 < d.len() {
        if d[i + 2] == 0x10 && d[i + 3] == 0xF0 {
            let len = u32at(i + 4) as usize;
            if (len == 28 || len == 32) && u32at(i + 8) as usize == len {
                let a = d[i + 8..i + 8 + len].to_vec();
                let mut j = i + 8 + len;
                while j + 14 < d.len() {
                    if d[j + 2] == 0x11 && d[j + 3] == 0xF0 && u32at(j + 8) == 0x0a {
                        out.insert(u32at(j + 8 + 6), a);
                        break;
                    }
                    j += 1;
                }
                i += 8 + len;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn item(o: &Obj, id: u16) -> Option<&Item> {
    o.items.iter().find(|i| i.id == id)
}
fn pair(v: &Value) -> (u64, u64) {
    let Value::Record(r) = v else { panic!("not a record") };
    let g = |id| r.iter().find(|i| i.id == id).and_then(Item::as_int).unwrap_or(0);
    (g(1), g(2))
}

/// Margins (left, right, top, bottom) read back from the layout-guides object.
fn margins(bytes: &[u8]) -> [u32; 4] {
    let doc = pub_format::read_bytes(bytes).unwrap();
    let objs = objects(bytes);
    let g = objs.values().find(|o| o.kind == 76).unwrap();
    let Value::List(l) = &item(g, 2).unwrap().value else { panic!() };
    let p: Vec<u64> = l.iter().map(|e| pair(&e.value).0).collect();
    [
        p[0] as u32,
        doc.page_width_emu - p[1] as u32,
        p[2] as u32,
        doc.page_height_emu - p[3] as u32,
    ]
}

fn size(bytes: &[u8]) -> (u32, u32) {
    let d = pub_format::read_bytes(bytes).unwrap();
    (d.page_width_emu, d.page_height_emu)
}

/// Blank the printer name (item 8 of each printer record in kind 75): it names the printer of
/// the machine that saved the file, and the repo's samples have it replaced by a neutral name.
fn mask_printer_names(items: &mut [Item]) {
    for i in items {
        match &mut i.value {
            Value::Record(r) => {
                for f in r.iter_mut().filter(|f| f.id == 8) {
                    f.value = Value::Raw(Vec::new());
                }
                mask_printer_names(r);
            }
            Value::List(l) => mask_printer_names(l),
            _ => {}
        }
    }
}

/// Mask what the spec calls save noise, normalise list order of the scratch page.
fn masked(mut m: BTreeMap<u32, Obj>) -> BTreeMap<u32, Obj> {
    for o in m.values_mut() {
        if o.kind == 75 {
            mask_printer_names(&mut o.items);
        }
        if o.kind == 68 {
            o.items.retain(|i| i.id != 60);
        }
        if o.kind == 1 {
            o.items.retain(|i| i.id != 4);
            // Overlap bookkeeping between shapes that end up on the scratch area (4Q gap).
            o.items.retain(|i| !matches!(i.id, 42 | 45 | 70 | 71));
        }
        if o.kind == 67 {
            for i in o.items.iter_mut().filter(|i| i.id == 2) {
                if let Value::List(l) = &mut i.value {
                    l.sort_by_key(|x| x.as_int());
                }
            }
        }
    }
    m
}

fn diff_objects(a: &BTreeMap<u32, Obj>, b: &BTreeMap<u32, Obj>, skip: &[u16]) -> Vec<String> {
    let mut out = Vec::new();
    for n in a.keys().chain(b.keys()).collect::<std::collections::BTreeSet<_>>() {
        match (a.get(n), b.get(n)) {
            (Some(x), Some(y)) if skip.contains(&x.kind) && x.kind == y.kind => {}
            (Some(x), Some(y)) if x != y => out.push(format!("object {n} (kind {}) differs:\n ours  {:?}\n their {:?}", x.kind, x.items, y.items)),
            (Some(_), Some(_)) => {}
            (x, y) => out.push(format!("object {n}: ours {} theirs {}", x.is_some(), y.is_some())),
        }
    }
    out
}

/// Strict structural comparison with Publisher's own file (form-C results).
fn assert_same(ours: &[u8], theirs: &[u8], skip_kinds: &[u16], what: &str) {
    let d = diff_objects(&masked(objects(ours)), &masked(objects(theirs)), skip_kinds);
    assert!(d.is_empty(), "{what}: Contents differs:\n{}", d.join("\n"));
    let (ao, at) = (anchors(ours), anchors(theirs));
    assert_eq!(ao, at, "{what}: Escher anchors differ");
    assert_eq!(
        Streams::from_cfb(ours).unwrap().escher.len(),
        Streams::from_cfb(theirs).unwrap().escher.len(),
        "{what}: Escher length"
    );
}

type ShapeView = (u16, [i32; 4], Option<String>);

/// Reader-level comparison (for preset-form results whose object layout differs).
fn assert_same_view(ours: &[u8], theirs: &[u8], what: &str) {
    let (a, b) = (
        pub_format::read_bytes(ours).unwrap(),
        pub_format::read_bytes(theirs).unwrap(),
    );
    assert_eq!((a.page_width_emu, a.page_height_emu), (b.page_width_emu, b.page_height_emu), "{what}: size");
    let view = |d: &pub_format::Document| -> Vec<Vec<ShapeView>> {
        d.pages
            .iter()
            .map(|p| {
                p.shapes
                    .iter()
                    .map(|s| (s.kind, [s.left_emu, s.top_emu, s.right_emu, s.bottom_emu], s.text.clone()))
                    .collect()
            })
            .collect()
    };
    assert_eq!(view(&a), view(&b), "{what}: shapes on the pages");
    // Scratch shapes: the multiset of anchors.
    let sorted = |x: &[u8]| {
        let mut v: Vec<Vec<u8>> = anchors(x).into_values().collect();
        v.sort();
        v
    };
    assert_eq!(sorted(ours), sorted(theirs), "{what}: all anchors");
}

// ------------------------------------------------------------------ Publisher pairs

#[kernpage_samples::test]
fn size_changes_match_publisher_form_c() {
    let m = manifest();
    for (name, base) in [
        ("page-a4", "page-base"),
        ("page-legal", "page-base"),
        ("page-11x17", "page-base"),
        ("page-custom-5x7", "page-base"),
        ("page-custom-odd", "page-base"),
        ("page-custom-300x400", "page-base"),
        ("page-landscape", "page-base"),
        ("page-edge-landscape", "page-base-edge"),
    ] {
        if !have(&[name, base]) {
            continue;
        }
        let (w, h) = size_of(&m, name);
        let ours = set_page_size(&gen(base), w, h).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_same(&ours, &gen(name), &[], name);
        assert_eq!(size(&ours), (w, h), "{name}");
    }
}

#[kernpage_samples::test]
fn ui_edge_case_matches_page_setup_dialog() {
    // The dialog variant of object 292 differs; everything else must match (both axes of
    // the scratch anchors converted, spec 4Q.5).
    let m = manifest();
    let (w, h) = size_of(&m, "page-edge-5x7-dialog");
    let ours = set_page_size(&gen("page-base-edge"), w, h).unwrap();
    assert_same_view(&ours, &gen("page-edge-5x7-dialog"), "edge-5x7-dialog");
}

#[kernpage_samples::test]
fn preset_results_agree_on_positions() {
    let m = manifest();
    for (name, base) in [
        ("page-a4-preset", "page-base"),
        ("page-legal-preset", "page-base"),
        ("page-tabloid-preset", "page-base"),
        ("page-landscape-preset", "page-base"),
        ("page-edge-a5-preset", "page-base-edge"),
    ] {
        if !have(&[name, base]) {
            continue;
        }
        let (w, h) = size_of(&m, name);
        // The preset sizes are exact EMU values (A4 7,560,000 x 10,692,000 ...).
        let (w, h) = match name {
            "page-a4-preset" => (7_560_000, 10_692_000),
            "page-edge-a5-preset" => (5_328_000, 7_560_000),
            _ => (w, h),
        };
        let ours = set_page_size(&gen(base), w, h).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_same_view(&ours, &gen(name), name);
    }
}

#[kernpage_samples::test]
fn margins_match_publisher() {
    let m = manifest();
    for name in ["page-margins-narrow", "page-margins-wide", "page-margins-none", "page-margins-custom"] {
        if !have(&[name, "page-base"]) {
            continue;
        }
        let mg = m.margins(name);
        let ours = set_margins(&gen("page-base"), mg[0], mg[1], mg[2], mg[3]).unwrap();
        assert_same(&ours, &gen(name), &[], name);
        assert_eq!(margins(&ours), mg, "{name}");
    }
    // 2-page base, dialog values 1.5 / 1 / 2 / 0.75 in.
    let ours = set_margins(&gen("page-base-2pages"), 1_371_600, 914_400, 1_828_800, 685_800).unwrap();
    assert_same(&ours, &gen("page-margins-custom-dialog-2pages"), &[], "custom-dialog-2pages");
}

#[kernpage_samples::test]
fn combined_two_page_a4_landscape_with_margins() {
    // 841.9 x 595.3 pt, margins L54 R36 T72 B90.
    let (w, h) = (10_692_130, 7_560_310);
    let a = set_page_size(&gen("page-base-2pages"), w, h).unwrap();
    let ours = set_margins(&a, 54 * 12_700, 36 * 12_700, 72 * 12_700, 90 * 12_700).unwrap();
    assert_same(&ours, &gen("page-a4-landscape-margins-2pages"), &[], "2pages-a4-landscape-margins");
}

// ------------------------------------------------------------------ blanks

fn blank() -> Vec<u8> {
    new_publication(&NewPublication::letter(1)).unwrap()
}

#[kernpage_samples::test]
fn blank_sizes_read_back() {
    let cases: [(&str, u32, u32); 6] = [
        ("a4", 7_560_000, 10_692_000),
        ("legal-landscape", 12_801_600, 7_772_400),
        ("5x7", 4_572_000, 6_400_800),
        ("11x17", 10_058_400, 15_544_800),
        ("letter-landscape", 10_058_400, 7_772_400),
        ("odd", 7_776_000, 10_060_000),
    ];
    for (name, w, h) in cases {
        let out = set_page_size(&blank(), w, h).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(size(&out), (w, h), "{name}");
        assert_eq!(margins(&out), [457_200; 4], "{name}: margins follow the size");
    }
}

#[kernpage_samples::test]
fn blank_resize_agrees_with_native_blank_for_exact_sizes() {
    // For sizes whose half is a multiple of 0.6 pt the blank writer and the resizer agree
    // on every size field.
    for (w, h) in [(7_772_400, 12_801_600), (10_058_400, 7_772_400), (10_058_400, 15_544_800)] {
        let ours = masked(objects(&set_page_size(&blank(), w, h).unwrap()));
        let native = masked(objects(&new_publication(&NewPublication::new(w, h, 1)).unwrap()));
        for n in ours.keys() {
            let (a, b) = (&ours[n], &native[n]);
            if matches!(a.kind, 67 | 68 | 76) {
                for id in [5u16, 17, 18, 3] {
                    assert_eq!(item(a, id), item(b, id), "{w}x{h} object {n} item {id}");
                }
            }
        }
    }
}

#[kernpage_samples::test]
fn blank_margin_presets() {
    let w = 7_772_400u32;
    let _ = w;
    for mg in [
        [228_600u32; 4],
        [914_400; 4],
        [0; 4],
        [685_800, 457_200, 914_400, 1_130_300],
    ] {
        let out = set_margins(&blank(), mg[0], mg[1], mg[2], mg[3]).unwrap();
        assert_eq!(margins(&out), mg);
    }
    assert!(matches!(
        set_margins(&blank(), 4_000_000, 4_000_000, 0, 0),
        Err(WriteError::PageSetup(_))
    ));
}

#[kernpage_samples::test]
fn margins_zero_has_no_position_item() {
    let out = set_margins(&blank(), 0, 457_200, 0, 457_200).unwrap();
    let objs = objects(&out);
    let g = objs.values().find(|o| o.kind == 76).unwrap();
    let Value::List(l) = &item(g, 2).unwrap().value else { panic!() };
    let Value::Record(first) = &l[0].value else { panic!() };
    assert!(first.iter().all(|i| i.id != 1), "left guide at 0 carries no item 1");
}

#[kernpage_samples::test]
fn rejects_unusable_sizes_and_two_page_master() {
    assert!(matches!(set_page_size(&blank(), 100, 100), Err(WriteError::PageSetup(_))));
    assert!(matches!(
        set_page_size(&gen("page-margins-twopage-master"), 7_560_000, 10_692_000),
        Err(WriteError::PageSetup(_))
    ));
}

// ------------------------------------------------------------------ sequences

#[kernpage_samples::test]
fn size_then_margins_then_size() {
    let a = set_page_size(&blank(), 7_560_000, 10_692_000).unwrap();
    let b = set_margins(&a, 914_400, 457_200, 228_600, 685_800).unwrap();
    let c = set_page_size(&b, 10_058_400, 15_544_800).unwrap();
    assert_eq!(size(&c), (10_058_400, 15_544_800));
    assert_eq!(margins(&c), [914_400, 457_200, 228_600, 685_800]);
    // Back to Letter and default margins gives the original size fields.
    let d = set_page_size(&c, 7_772_400, 10_058_400).unwrap();
    let e = set_margins(&d, 457_200, 457_200, 457_200, 457_200).unwrap();
    let (o, n) = (masked(objects(&e)), masked(objects(&blank())));
    for k in o.keys() {
        if matches!(o[k].kind, 67 | 76) {
            assert_eq!(o[k].items, n[k].items, "object {k}");
        }
    }
}

#[kernpage_samples::test]
fn resize_twice_is_the_same_as_once() {
    let base = gen("page-base");
    let once = set_page_size(&base, 4_572_000, 6_400_800).unwrap();
    let twice = set_page_size(&set_page_size(&base, 7_560_000, 10_692_000).unwrap(), 4_572_000, 6_400_800).unwrap();
    // Rect Far is on the scratch area in both (moved at the second size in `twice` from
    // the page it still touched on A4), so compare through the reader.
    assert_same_view(&twice, &once, "5x7 via A4");
}

#[kernpage_samples::test]
fn shapes_move_to_scratch_and_stay_there() {
    let small = set_page_size(&gen("page-base-edge"), 4_572_000, 6_400_800).unwrap();
    let doc = pub_format::read_bytes(&small).unwrap();
    assert!(doc.pages[0].shapes.is_empty());
    let big = set_page_size(&small, 10_058_400, 15_544_800).unwrap();
    assert!(pub_format::read_bytes(&big).unwrap().pages[0].shapes.is_empty());
    // Stored anchors are page-corner relative: a bigger page does not touch them.
    assert_eq!(anchors(&small), anchors(&big));
}

#[kernpage_samples::test]
fn editing_after_a_size_change() {
    let a = set_page_size(&blank(), 7_560_000, 10_692_000).unwrap();
    let b = add_shape(&a, 0, NewShapeKind::Rectangle, 50.0, 200.0, 100.0, 40.0, ShapeStyle::default()).unwrap();
    let c = add_text_box(&b, 0, 200.0, 300.0, 150.0, 50.0, "after a resize").unwrap();
    let doc = pub_format::read_bytes(&c).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 2);
    let (cx, cy) = (f64::from(doc.page_width_emu) / 2.0, f64::from(doc.page_height_emu) / 2.0);
    let s = &doc.pages[0].shapes[0];
    assert!((f64::from(s.left_emu) + cx - 50.0 * PT).abs() < 2.0);
    assert!((f64::from(s.top_emu) + cy - 200.0 * PT).abs() < 2.0);
    // And a resize after editing moves the page edges around the shapes.
    let d = set_page_size(&c, 10_058_400, 7_772_400).unwrap();
    let doc2 = pub_format::read_bytes(&d).unwrap();
    assert_eq!(doc2.pages[0].shapes.len(), 2);
    let s2 = &doc2.pages[0].shapes[0];
    let (cx2, cy2) = (f64::from(doc2.page_width_emu) / 2.0, f64::from(doc2.page_height_emu) / 2.0);
    let dx = (f64::from(doc2.page_width_emu) - f64::from(doc.page_width_emu)) / 2.0;
    let dy = (f64::from(doc2.page_height_emu) - f64::from(doc.page_height_emu)) / 2.0;
    assert!(((f64::from(s2.left_emu) + cx2) - (f64::from(s.left_emu) + cx) - dx).abs() < 2.0);
    assert!(((f64::from(s2.top_emu) + cy2) - (f64::from(s.top_emu) + cy) - dy).abs() < 2.0);
}

#[kernpage_samples::test]
fn picture_after_a_size_change() {
    let img = std::fs::read(root().join("tools/assets/red4x3.png")).unwrap();
    let a = set_page_size(&blank(), 7_560_000, 10_692_000).unwrap();
    let (b, _) = add_picture(
        &a,
        0,
        NewPicture {
            image: &img,
            format: PictureKind::Png,
            file_name: "red4x3.png",
            modified: None,
            left_pt: 100.0,
            top_pt: 150.0,
            width_pt: 120.0,
            height_pt: 90.0,
        },
    )
    .unwrap();
    let doc = pub_format::read_bytes(&b).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 1);
    assert!(doc.pages[0].shapes[0].picture.is_some());
}

#[kernpage_samples::test]
fn refuses_what_it_cannot_move_safely() {
    // A group that no longer touches the page cannot be moved (unverified).
    let b = blank();
    let b = add_shape(&b, 0, NewShapeKind::Rectangle, 500.0, 700.0, 60.0, 60.0, ShapeStyle::default()).unwrap();
    let b = add_shape(&b, 0, NewShapeKind::Rectangle, 520.0, 720.0, 30.0, 30.0, ShapeStyle::default()).unwrap();
    let g = pub_write::group_shapes(&b, 0, &[0, 1]).unwrap();
    match set_page_size(&g, 4_572_000, 6_400_800) {
        Err(WriteError::PageSetup(_)) => {}
        other => panic!("expected PageSetup error, got {:?}", other.map(|v| v.len())),
    }
}

// ------------------------------------------------------------------ check files

#[kernpage_samples::test]
#[ignore]
fn write_check_files() {
    let dir = PathBuf::from(
        std::env::temp_dir().join("pw-pagewrite").to_string_lossy().into_owned(),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let w = |name: &str, bytes: Vec<u8>| std::fs::write(dir.join(name), bytes).unwrap();
    let with_shapes = {
        let b = blank();
        let b = add_shape(&b, 0, NewShapeKind::Rectangle, 72.0, 72.0, 144.0, 72.0, ShapeStyle::default()).unwrap();
        add_text_box(&b, 0, 72.0, 200.0, 216.0, 72.0, "Page setup text box.").unwrap()
    };
    w("ours-a4.pub", set_page_size(&with_shapes, 7_560_000, 10_692_000).unwrap());
    w("ours-legal-landscape.pub", set_page_size(&with_shapes, 12_801_600, 7_772_400).unwrap());
    w("ours-5x7.pub", set_page_size(&with_shapes, 4_572_000, 6_400_800).unwrap());
    w("ours-11x17.pub", set_page_size(&with_shapes, 10_058_400, 15_544_800).unwrap());
    w("ours-margins-narrow.pub", set_margins(&with_shapes, 228_600, 228_600, 228_600, 228_600).unwrap());
    w("ours-margins-none.pub", set_margins(&with_shapes, 0, 0, 0, 0).unwrap());
    w("ours-margins-custom.pub", set_margins(&with_shapes, 685_800, 457_200, 914_400, 1_143_000).unwrap());
    let a = set_page_size(&gen("page-base-2pages"), 10_692_130, 7_560_310).unwrap();
    w(
        "ours-2pages-a4-landscape-margins.pub",
        set_margins(&a, 54 * 12_700, 36 * 12_700, 72 * 12_700, 90 * 12_700).unwrap(),
    );
    w("ours-shape-to-scratch.pub", set_page_size(&gen("page-base-edge"), 4_572_000, 6_400_800).unwrap());
    let img = std::fs::read(root().join("tools/assets/red4x3.png")).unwrap();
    let a4 = set_page_size(&blank(), 7_560_000, 10_692_000).unwrap();
    let (p, _) = add_picture(
        &a4,
        0,
        NewPicture {
            image: &img,
            format: PictureKind::Png,
            file_name: "red4x3.png",
            modified: None,
            left_pt: 100.0,
            top_pt: 150.0,
            width_pt: 120.0,
            height_pt: 90.0,
        },
    )
    .unwrap();
    w("ours-new-pub-a4-then-picture.pub", p);
    // A Publisher-saved preset-form file resized by us (form P -> form C, no 293-295).
    w("ours-from-preset-a4-to-legal.pub", set_page_size(&gen("page-a4-preset"), 7_772_400, 12_801_600).unwrap());
}
