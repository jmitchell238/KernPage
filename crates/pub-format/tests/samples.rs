use pub_format::{assemble, read, read_bytes, PictureFormat, Streams, TextRun};
use std::path::PathBuf;

fn sample(name: &str) -> PathBuf {
    kernpage_samples::path(name)
}

const GEN: [&str; 8] = [
    "gen/apple-banana.pub",
    "gen/formatting.pub",
    "gen/picture.pub",
    "gen/legal.pub",
    "gen/bulletin.pub",
    "gen/colors.pub",
    "gen/shapes.pub",
    "gen/table.pub",
];

const MODERN: [&str; 7] = [
    "blank.pub",
    "hello.pub",
    "hello-moved.pub",
    "two-pages.pub",
    "bold.pub",
    "two-boxes.pub",
    "tika-testPUBLISHER.pub",
];

#[kernpage_samples::test]
fn all_modern_samples_parse() {
    for name in MODERN {
        let Some(p) = kernpage_samples::find(name) else { continue };
        read(&p).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[kernpage_samples::test]
fn old_generation_is_rejected_not_panicked() {
    let err = read(&sample("lo-fdo59355-1.pub")).unwrap_err();
    assert!(
        err.to_string().contains("unsupported format generation"),
        "{err}"
    );
}

#[kernpage_samples::test]
fn garbage_is_an_error() {
    assert!(read_bytes(b"not a compound file").is_err());
    assert!(read_bytes(&[]).is_err());
}

#[kernpage_samples::test]
fn blank_has_one_page_and_no_shapes() {
    let doc = read(&sample("blank.pub")).unwrap();
    assert_eq!(doc.pages.len(), 1);
    assert!(doc.pages[0].shapes.is_empty());
}

#[kernpage_samples::test]
fn two_pages_has_two_pages() {
    assert_eq!(read(&sample("two-pages.pub")).unwrap().pages.len(), 2);
}

#[kernpage_samples::test]
fn hello_shape() {
    let doc = read(&sample("hello.pub")).unwrap();
    assert_eq!(doc.pages.len(), 1);
    assert_eq!(doc.pages[0].shapes.len(), 1);
    let s = &doc.pages[0].shapes[0];
    assert_eq!((s.width_emu, s.height_emu), (Some(3348111), Some(1392701)));
    assert_eq!(
        (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu),
        (-3341077, -4508695, 7034, -3115994)
    );
    assert_eq!(doc.text, "Hello\r");
    assert_eq!(doc.stories, vec!["Hello\r".to_string()]);
    assert_eq!(s.text.as_deref(), Some("Hello\r"));
    assert!(doc.fonts.iter().any(|f| f == "Calibri"), "{:?}", doc.fonts);
}

#[kernpage_samples::test]
fn hello_moved_shape() {
    let doc = read(&sample("hello-moved.pub")).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 1);
    let s = &doc.pages[0].shapes[0];
    assert_eq!((s.width_emu, s.height_emu), (Some(618979), Some(323556)));
    assert_eq!(
        (s.left_emu, s.top_emu, s.right_emu, s.bottom_emu),
        (-274320, -1287193, 344659, -963637)
    );
}

#[kernpage_samples::test]
fn page_sizes() {
    for name in MODERN {
        let Some(p) = kernpage_samples::find(name) else { continue };
        let doc = read(&p).unwrap();
        let expected = if name == "tika-testPUBLISHER.pub" {
            // A4 (8.27 x 11.69 in) per the kind-68 object, rounded to whole EMU.
            (doc.page_width_emu, doc.page_height_emu)
        } else {
            (7772400, 10058400)
        };
        assert_eq!(
            (doc.page_width_emu, doc.page_height_emu),
            expected,
            "{name}"
        );
    }
    let tika = read(&sample("tika-testPUBLISHER.pub")).unwrap();
    let (w, h) = (
        tika.page_width_emu as f64 / 914_400.0,
        tika.page_height_emu as f64 / 914_400.0,
    );
    assert!(
        (w - 8.27).abs() < 0.01 && (h - 11.69).abs() < 0.01,
        "{w} x {h}"
    );
    if let Some(p) = kernpage_samples::find("gen/legal.pub") {
        let legal = read(&p).unwrap();
        assert_eq!(
            (legal.page_width_emu, legal.page_height_emu),
            (7772400, 12801600)
        );
    }
}

#[kernpage_samples::test]
fn blank_has_no_stories() {
    let doc = read(&sample("blank.pub")).unwrap();
    assert!(doc.stories.is_empty());
    assert_eq!(doc.text, "");
}

fn bold_chars(runs: &[TextRun]) -> String {
    runs.iter()
        .filter(|r| r.bold)
        .map(|r| r.text.as_str())
        .collect()
}

#[kernpage_samples::test]
fn bold_runs() {
    let doc = read(&sample("bold.pub")).unwrap();
    assert_eq!(doc.pages[0].shapes.len(), 1);
    let s = &doc.pages[0].shapes[0];
    assert_eq!(s.text.as_deref(), Some("Hello\rHELLO BOLD\r"));
    let all: String = s.runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(all, "Hello\rHELLO BOLD\r");
    // The paragraph mark after the bold text is also bold (spec 3.5).
    assert_eq!(bold_chars(&s.runs).trim_end_matches('\r'), "HELLO BOLD");
    assert!(!s.runs[0].bold && s.runs[0].text == "Hello\r");
}

#[kernpage_samples::test]
fn two_boxes_have_their_own_stories() {
    let doc = read(&sample("two-boxes.pub")).unwrap();
    assert_eq!(doc.stories.len(), 2);
    let shapes = &doc.pages[0].shapes;
    assert_eq!(shapes.len(), 2);
    for s in shapes {
        assert_eq!(s.text.as_deref(), Some("Hello\rHELLO BOLD\r"));
        assert_eq!(bold_chars(&s.runs).trim_end_matches('\r'), "HELLO BOLD");
    }
}

#[kernpage_samples::test]
fn tika_stories_map_by_rank() {
    let doc = read(&sample("tika-testPUBLISHER.pub")).unwrap();
    assert_eq!(doc.stories.len(), 2);
    let shapes = &doc.pages[0].shapes;
    let texts: Vec<_> = shapes.iter().map(|s| s.text.clone().unwrap()).collect();
    assert_eq!(texts[0], doc.stories[0]);
    assert_eq!(texts[1], doc.stories[1]);
    assert_eq!(doc.stories.concat(), doc.text);
}

/// Quill positions to corrupt: the header and directory, plus every byte of the STRS, FDPC,
/// FDPP, FONT and PL chunks (which sit far past the first 2000 bytes).
fn quill_fuzz_positions(quill: &[u8]) -> Vec<usize> {
    let mut out: Vec<usize> = (0..quill.len().min(600)).collect();
    let mut pos = 0x20;
    while let Some(e) = quill.get(pos..pos + 24) {
        if e[0] != 24 {
            break;
        }
        if matches!(&e[2..6], b"STRS" | b"FDPC" | b"FDPP" | b"FONT" | b"PL  ") {
            let off = u32::from_le_bytes([e[16], e[17], e[18], e[19]]) as usize;
            let len = u32::from_le_bytes([e[20], e[21], e[22], e[23]]) as usize;
            out.extend(off..(off + len).min(quill.len()));
        }
        pos += 24;
    }
    out
}

/// Corrupt the Contents stream (and separately the Escher, Quill and delay streams) at
/// each of the first 2000 positions (Quill: see `quill_fuzz_positions`); the decoders may
/// return errors but must not panic. Covers the FDPP, FONT and picture-store parsers.
#[kernpage_samples::test]
fn corrupted_streams_never_panic() {
    for name in MODERN.iter().chain(GEN.iter()) {
        let Some(p) = kernpage_samples::find(name) else { continue };
        let bytes = std::fs::read(p).unwrap();
        let base = Streams::from_cfb(&bytes).unwrap();
        for which in 0..4 {
            let original = match which {
                0 => &base.contents,
                1 => &base.escher,
                2 => &base.quill,
                _ => &base.delay,
            };
            let positions = if which == 2 {
                quill_fuzz_positions(original)
            } else if which == 1 {
                // The whole Escher stream: includes the shape property tables.
                (0..original.len()).collect()
            } else {
                (0..original.len().min(2000)).collect()
            };
            for pos in positions {
                let mut variants: Vec<Vec<u8>> = Vec::new();
                let mutations: [fn(u8) -> u8; 4] =
                    [|b| !b, |_| 0x00, |_| 0xFF, |b| b.wrapping_add(1)];
                for mutate in mutations {
                    let mut v = original.clone();
                    v[pos] = mutate(v[pos]);
                    variants.push(v);
                }
                variants.push(original[..pos].to_vec());
                for v in variants {
                    let mut s = Streams {
                        contents: base.contents.clone(),
                        escher: base.escher.clone(),
                        quill: base.quill.clone(),
                        delay: base.delay.clone(),
                    };
                    match which {
                        0 => s.contents = v,
                        1 => s.escher = v,
                        2 => s.quill = v,
                        _ => s.delay = v,
                    }
                    let _ = assemble(&s);
                }
            }
        }
    }
}

/// Report (not a gate; the sample is untracked and untrusted): how well the MCLD
/// group-to-story rule of spec 4I holds on the POI newsletter.
#[kernpage_samples::test]
fn newsletter_mcld_mapping_report() {
    let Ok(doc) = read(&sample("external/poi/SampleNewsletter.pub")) else {
        eprintln!("SampleNewsletter.pub not present; skipped");
        return;
    };
    let multi = doc
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .filter(|s| s.columns > 1)
        .count();
    eprintln!(
        "newsletter MCLD: {} chains mapped, {} unmapped, {} boxes with >1 column",
        doc.columns_mapped, doc.columns_unmapped, multi
    );
}

/// The untrusted POI samples are git-ignored; skip when absent.
fn poi_metafiles(name: &str) -> Option<Vec<(PictureFormat, Vec<u8>)>> {
    let bytes = std::fs::read(sample(&format!("external/poi/{name}"))).ok()?;
    let doc = read_bytes(&bytes).unwrap();
    Some(
        doc.pages
            .iter()
            .flat_map(|p| p.shapes.iter())
            .filter_map(|s| s.picture.as_ref())
            .filter(|p| matches!(p.format, PictureFormat::Wmf | PictureFormat::Emf))
            .map(|p| (p.format, p.bytes.clone()))
            .collect(),
    )
}

#[kernpage_samples::test]
fn poi_brochure_metafile_blips_decompress() {
    let Some(pics) = poi_metafiles("SampleBrochure.pub") else {
        eprintln!("SampleBrochure.pub absent, skipping");
        return;
    };
    assert!(!pics.is_empty(), "the brochure has WMF/EMF pictures");
    for (format, bytes) in pics {
        match format {
            // Placeable WMF key 0x9AC6CDD7 or a bare METAHEADER (header size 9 words).
            PictureFormat::Wmf => assert!(
                bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A])
                    || bytes.get(2..4) == Some(&[9, 0][..]),
                "{:02x?}",
                &bytes[..16.min(bytes.len())]
            ),
            _ => assert_eq!(&bytes[40..44], b" EMF"),
        }
    }
}
