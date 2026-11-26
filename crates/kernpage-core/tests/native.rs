//! The `.kern` format: round trips, limits, damaged input, and committed version-1 files that
//! must keep loading and keep their exact bytes.

use kernpage_core::native::{
    is_kern, read_kern, read_kern_with_id, write_file_checked, write_kern, write_kern_with_id, KernError,
};
use kernpage_core::{Document, Page, Rect, Shape, ShapeKind};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    root().join("crates/kernpage-core/tests/fixtures").join(name)
}

/// Same content (names, pages, stories) plus the parts `PartialEq` leaves out.
fn assert_same(a: &Document, b: &Document) {
    assert_eq!(a, b);
    assert_eq!(a.masters, b.masters);
    assert_eq!(a.sections, b.sections);
    assert_eq!(a.default_line, b.default_line);
    assert_eq!(
        a.pages.iter().map(|p| (p.master, p.ignore_master)).collect::<Vec<_>>(),
        b.pages.iter().map(|p| (p.master, p.ignore_master)).collect::<Vec<_>>()
    );
}

fn round_trip(doc: &Document) -> Document {
    let bytes = write_kern(doc).unwrap();
    assert!(is_kern(&bytes));
    let back = read_kern(&bytes).unwrap();
    assert_same(doc, &back);
    back
}

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0,
    0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89,
];

fn picture(data: &[u8], x: i64) -> Shape {
    Shape::new(
        Rect::new(x, 0, 1_000_000, 1_000_000),
        ShapeKind::Picture {
            data: Arc::new(data.to_vec()),
            format: kernpage_core::PictureFormat::Png,
            file: Some(kernpage_core::PictureFile { name: "logo.png".into(), modified: Some(7) }),
        },
    )
}

/// A fixed document: page ids are set by hand because the counter is global.
fn sample_doc() -> Document {
    let mut page = Page::blank(7_772_400, 10_058_400);
    page.id = 1;
    let mut text = Shape::new(
        Rect::new(500_000, 500_000, 3_000_000, 800_000),
        ShapeKind::TextBox { paragraphs: vec![kernpage_core::Paragraph::plain("Hello, KernPage")] },
    );
    text.fill = Some(kernpage_core::Rgb(1, 2, 3));
    page.shapes = vec![text, picture(PNG, 1_000_000)];
    Document::new("Sample", vec![page])
}

const SAMPLE_ID: &str = "00112233445566778899aabbccddeeff";

#[kernpage_samples::test]
fn templates_round_trip() {
    round_trip(&kernpage_core::template::blank_portrait());
    round_trip(&kernpage_core::template::blank_landscape());
    round_trip(&kernpage_core::template::church_bulletin());
}

#[kernpage_samples::test]
fn sample_publications_round_trip() {
    for name in ["bulletin", "linked", "group", "colors", "columns", "formatting", "list-bullet-all"] {
        let Some(path) = kernpage_samples::find(&format!("gen/{name}.pub")) else {
            continue;
        };
        let doc = kernpage_core::from_pub(name, &pub_format::read(&path).unwrap());
        round_trip(&doc);
    }
}

fn names(z: &zip::ZipArchive<Cursor<Vec<u8>>>) -> Vec<String> {
    z.file_names().map(|n| n.unwrap().into_owned()).collect()
}

fn entry(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    let mut out = Vec::new();
    std::io::copy(&mut z.by_name(name).unwrap(), &mut out).unwrap();
    out
}

#[kernpage_samples::test]
fn a_document_with_pictures_round_trips_and_pictures_are_written_once() {
    let mut doc = sample_doc();
    doc.pages[0].shapes.push(picture(PNG, 2_000_000));
    doc.pages[0].shapes.push(picture(b"\xFF\xD8other picture", 3_000_000));
    let doc = Document::new("Sample", doc.pages);
    let back = round_trip(&doc);
    assert!(matches!(&back.pages[0].shapes[1].kind, ShapeKind::Picture { data, .. } if data.as_slice() == PNG));

    let bytes = write_kern(&doc).unwrap();
    let zip = zip::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let media: Vec<String> = names(&zip).into_iter().filter(|n| n.starts_with("assets/images/")).collect();
    assert_eq!(media.len(), 2, "{media:?}");
    assert!(media.iter().any(|n| n.ends_with(".png")) && media.iter().any(|n| n.ends_with(".jpg")));
    // Pictures are stored without compression, the JSON parts are deflated.
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    for i in 0..z.len() {
        let f = z.by_index_raw(i).unwrap();
        let n = f.name().unwrap().into_owned();
        let want = if n.starts_with("assets/") || n == "mimetype" {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        assert_eq!(f.compression(), want, "{n}");
    }
}

#[kernpage_samples::test]
fn written_files_are_the_same_every_time() {
    let doc = sample_doc();
    assert_eq!(
        write_kern_with_id(&doc, SAMPLE_ID).unwrap(),
        write_kern_with_id(&doc, SAMPLE_ID).unwrap()
    );
}

#[kernpage_samples::test]
fn the_document_id_is_kept_and_the_manifest_has_the_agreed_shape() {
    let bytes = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    assert_eq!(read_kern_with_id(&bytes).unwrap().1, SAMPLE_ID);
    let m: serde_json::Value = serde_json::from_slice(&entry(&bytes, "manifest.json")).unwrap();
    assert_eq!(m["format"], "KERN");
    assert_eq!(m["format_version"], "1.0");
    assert_eq!(m["minimum_reader_version"], "1.0");
    assert_eq!(m["document"], "document.json");
    assert_eq!(m["generator"]["application"], "KernPage");
    assert_eq!(m["required_features"], serde_json::json!([]));
    assert!(m.get("preview").is_none(), "no thumbnail is written yet");
    assert_ne!(kernpage_core::native::new_document_id(), kernpage_core::native::new_document_id());
}

#[kernpage_samples::test]
fn the_mimetype_entry_is_first_and_stored() {
    let bytes = write_kern(&sample_doc()).unwrap();
    let mt = b"application/vnd.kernpage.document+zip";
    assert!(bytes.starts_with(b"PK\x03\x04"));
    assert_eq!(&bytes[30..38], b"mimetype");
    assert_eq!(&bytes[38..38 + mt.len()], mt);
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (n, d) in entries {
        z.start_file(*n, opts).unwrap();
        z.write_all(d).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn doc_json() -> Vec<u8> {
    entry(&write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap(), "document.json")
}

fn manifest_with(format_version: &str, min: &str, required: &str, optional: &str) -> Vec<u8> {
    format!(
        r#"{{"format":"KERN","format_version":"{format_version}","minimum_reader_version":"{min}","document_id":"abc","generator":{{"application":"Other","version":"9"}},"document":"document.json","required_features":{required},"optional_features":{optional},"future":true}}"#
    )
    .into_bytes()
}

fn manifest() -> Vec<u8> {
    manifest_with("1.0", "1.0", "[]", "[]")
}

const MT: &[u8] = b"application/vnd.kernpage.document+zip";

/// A valid file around `manifest`, with the sample's picture.
fn file_with_manifest(manifest: &[u8]) -> Vec<u8> {
    let good = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    let z = zip::ZipArchive::new(Cursor::new(good.clone())).unwrap();
    let pic = names(&z).into_iter().find(|n| n.starts_with("assets/images/")).unwrap();
    zip_with(&[
        ("mimetype", MT),
        ("manifest.json", manifest),
        ("document.json", &entry(&good, "document.json")),
        (&pic, &entry(&good, &pic)),
    ])
}

#[kernpage_samples::test]
fn version_rules() {
    assert!(read_kern(&file_with_manifest(&manifest())).is_ok());
    // A later minor version, unknown optional features and a different generator are fine.
    assert!(read_kern(&file_with_manifest(&manifest_with("1.7", "1.0", "[]", r#"["future-thing"]"#))).is_ok());
    // A later major version, or a file that needs a newer reader, is refused clearly.
    for (v, min) in [("2.0", "2.0"), ("1.9", "1.5")] {
        let err = read_kern(&file_with_manifest(&manifest_with(v, min, "[]", "[]"))).unwrap_err();
        assert!(matches!(err, KernError::TooNew(_)), "{v}/{min}");
        assert!(err.to_string().contains("newer KernPage"));
    }
    // An unknown required feature is refused.
    let err = read_kern(&file_with_manifest(&manifest_with("1.0", "1.0", r#"["warp-drive"]"#, "[]"))).unwrap_err();
    assert!(matches!(err, KernError::Unsupported(ref f) if f == "warp-drive"));
    assert!(err.to_string().contains("warp-drive"));
    // An older, unknown major version is not read either.
    assert!(read_kern(&file_with_manifest(&manifest_with("0.9", "0.9", "[]", "[]"))).is_err());
    assert!(read_kern(&file_with_manifest(&manifest_with("one", "1.0", "[]", "[]"))).is_err());
}

#[kernpage_samples::test]
fn unknown_fields_are_ignored_and_missing_ones_defaulted() {
    let mut v: serde_json::Value = serde_json::from_slice(&doc_json()).unwrap();
    v["something_new"] = serde_json::json!({"a": 1});
    v.as_object_mut().unwrap().remove("name");
    v.as_object_mut().unwrap().remove("next_id");
    v.as_object_mut().unwrap().remove("default_line");
    let good = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    let z = zip::ZipArchive::new(Cursor::new(good.clone())).unwrap();
    let pic = names(&z).into_iter().find(|n| n.starts_with("assets/images/")).unwrap();
    let bytes = zip_with(&[
        ("mimetype", MT),
        ("manifest.json", &manifest()),
        ("document.json", v.to_string().as_bytes()),
        (&pic, &entry(&good, &pic)),
        ("previews/thumbnail.png", b"ignored"),
    ]);
    let mut doc = read_kern(&bytes).unwrap();
    assert_eq!(doc.name, "");
    assert_eq!(doc.default_line, kernpage_core::Rgb::BLACK);
    let id = doc.alloc_id();
    assert!(doc.pages[0].shapes.iter().all(|s| s.id != id), "new ids must not clash");
}

#[kernpage_samples::test]
fn bad_input_is_an_error() {
    assert!(matches!(read_kern(b""), Err(KernError::NotKern)));
    assert!(matches!(read_kern(b"hello"), Err(KernError::NotKern)));
    let m = manifest();
    let empty_doc: &[u8] = b"{\"pages\":[]}";
    let wrong = br#"{"format":"x","format_version":"1.0"}"#;
    for entries in [
        vec![("mimetype", MT), ("manifest.json", &wrong[..]), ("document.json", b"{}")],
        vec![("mimetype", MT), ("document.json", b"{}")],
        vec![("mimetype", MT), ("manifest.json", &m[..])],
        vec![("mimetype", MT), ("manifest.json", &m[..]), ("document.json", empty_doc)],
        vec![("mimetype", MT), ("manifest.json", &m[..]), ("document.json", b"not json")],
    ] {
        assert!(read_kern(&zip_with(&entries)).is_err());
    }
}

#[kernpage_samples::test]
fn pictures_are_not_inlined_and_a_missing_one_is_an_error() {
    let json = String::from_utf8(doc_json()).unwrap();
    assert!(json.contains("\"assets/images/"), "the document refers to its picture by name");
    assert!(!json.contains("89504e47"), "no inline picture bytes");
    let bytes = zip_with(&[("mimetype", MT), ("manifest.json", &manifest()), ("document.json", json.as_bytes())]);
    assert!(read_kern(&bytes).is_err());
}

#[kernpage_samples::test]
fn a_picture_whose_content_does_not_match_its_name_is_refused() {
    let good = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    let z = zip::ZipArchive::new(Cursor::new(good.clone())).unwrap();
    let pic = names(&z).into_iter().find(|n| n.starts_with("assets/images/")).unwrap();
    let bytes = zip_with(&[
        ("mimetype", MT),
        ("manifest.json", &manifest()),
        ("document.json", &entry(&good, "document.json")),
        (&pic, b"some other bytes"),
    ]);
    let err = read_kern(&bytes).unwrap_err();
    assert!(err.to_string().contains("does not match"), "{err}");
}

#[kernpage_samples::test]
fn duplicate_entry_names_are_refused_even_when_only_the_case_differs() {
    let bytes = zip_with(&[
        ("mimetype", MT),
        ("manifest.json", &manifest()),
        ("document.json", &doc_json()),
        ("Document.json", &doc_json()),
    ]);
    assert!(read_kern(&bytes).unwrap_err().to_string().contains("twice"));
}

#[kernpage_samples::test]
fn path_traversal_names_are_refused() {
    for bad in ["../evil.txt", "assets/../../evil", "/abs", "C:evil", "a\\b", "assets/./x"] {
        let bytes = zip_with(&[
            ("mimetype", MT),
            ("manifest.json", &manifest()),
            ("document.json", &doc_json()),
            (bad, b"x"),
        ]);
        assert!(read_kern(&bytes).is_err(), "{bad}");
    }
}

#[kernpage_samples::test]
fn too_many_entries_are_refused() {
    let names: Vec<String> = (0..4100).map(|i| format!("assets/images/{i}.bin")).collect();
    let mut entries: Vec<(&str, &[u8])> = vec![("mimetype", MT), ("manifest.json", b"{}")];
    entries.extend(names.iter().map(|n| (n.as_str(), &b"x"[..])));
    assert!(matches!(read_kern(&zip_with(&entries)), Err(KernError::TooLarge)));
}

#[kernpage_samples::test]
fn an_oversized_unpack_is_refused() {
    // Deflated zeros: a few KiB on disk that unpack past the cap.
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    z.start_file("mimetype", stored).unwrap();
    z.write_all(MT).unwrap();
    let zeros = vec![0u8; 1 << 20];
    for i in 0..513 {
        z.start_file(format!("assets/images/{i}.bin"), deflated).unwrap();
        z.write_all(&zeros).unwrap();
    }
    let bytes = z.finish().unwrap().into_inner();
    assert!(bytes.len() < 4 << 20);
    assert!(matches!(read_kern(&bytes), Err(KernError::TooLarge)));
}

#[kernpage_samples::test]
fn truncated_and_corrupted_files_never_panic() {
    let good = write_kern(&sample_doc()).unwrap();
    for cut in (0..good.len()).step_by(3) {
        assert!(read_kern(&good[..cut]).is_err(), "cut at {cut}");
    }
    // Flip every byte in turn (an Ok result is fine, a panic is not).
    for i in 0..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0xFF;
        let _ = read_kern(&bad);
    }
    // A simple pseudo-random mangling.
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..300 {
        let mut bad = good.clone();
        for _ in 0..4 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let at = (seed % bad.len() as u64) as usize;
            bad[at] = (seed >> 24) as u8;
        }
        let _ = read_kern(&bad);
    }
}

#[kernpage_samples::test]
fn a_checked_write_replaces_the_file_and_a_failed_one_leaves_it_alone() {
    let dir = std::env::temp_dir().join(format!("kern-write-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.kern");
    let first = write_kern(&sample_doc()).unwrap();
    write_file_checked(&path, &first, true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), first);
    // Not a valid file: refused, the old file stays and no temp file is left.
    let err = write_file_checked(&path, b"garbage", true).unwrap_err();
    assert!(err.contains("did not check out"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), first);
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")));
    // A valid overwrite keeps a backup of the previous file.
    let second = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    write_file_checked(&path, &second, true).unwrap();
    assert_eq!(std::fs::read(dir.join("a.kern.bak")).unwrap(), first);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The committed version-1 file keeps loading, whatever changes in later versions.
#[kernpage_samples::test]
fn the_v1_sample_still_loads() {
    let bytes = std::fs::read(fixture("v1-sample.kern")).unwrap();
    assert!(is_kern(&bytes));
    let (doc, id) = read_kern_with_id(&bytes).unwrap();
    assert_eq!(id, SAMPLE_ID);
    assert_eq!(doc.name, "Sample");
    assert_eq!(doc.pages.len(), 1);
    assert_eq!(doc.pages[0].shapes.len(), 2);
    match &doc.pages[0].shapes[0].kind {
        ShapeKind::TextBox { paragraphs } => assert_eq!(paragraphs[0].text(), "Hello, KernPage"),
        other => panic!("{other:?}"),
    }
    assert_eq!(doc.pages[0].shapes[0].fill, Some(kernpage_core::Rgb(1, 2, 3)));
    assert!(matches!(&doc.pages[0].shapes[1].kind, ShapeKind::Picture { data, .. } if data.as_slice() == PNG));
}

/// Fails when `document.json` for the fixed sample changes by even one byte. A change to the
/// document model that alters the format must be deliberate: bump `format_version`, update
/// docs/spec/KERN-FORMAT.md, then regenerate the file with `regenerate_golden_files`.
#[kernpage_samples::test]
fn document_json_for_the_fixed_sample_is_byte_for_byte_stable() {
    let bytes = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    let golden = std::fs::read(fixture("v1-sample.document.json")).unwrap();
    assert_eq!(
        String::from_utf8(entry(&bytes, "document.json")).unwrap(),
        String::from_utf8(golden).unwrap().trim_end(),
        "the .kern document format changed"
    );
}

/// Writes the committed fixtures. Run on purpose, once per format version:
/// `cargo test -p kernpage-core --test native -- --ignored regenerate_golden_files`
#[kernpage_samples::test]
#[ignore]
fn regenerate_golden_files() {
    std::fs::create_dir_all(fixture("")).unwrap();
    let bytes = write_kern_with_id(&sample_doc(), SAMPLE_ID).unwrap();
    std::fs::write(fixture("v1-sample.kern"), &bytes).unwrap();
    std::fs::write(fixture("v1-sample.document.json"), entry(&bytes, "document.json")).unwrap();
}
