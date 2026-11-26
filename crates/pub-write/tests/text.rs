use pub_write::{set_story_text, WriteError};
use std::io::Read;
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

fn streams(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut cf = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).unwrap();
    let paths: Vec<_> = cf
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_path_buf())
        .collect();
    paths
        .into_iter()
        .map(|p| {
            let mut v = Vec::new();
            cf.open_stream(&p).unwrap().read_to_end(&mut v).unwrap();
            (p.to_string_lossy().replace('\\', "/"), v)
        })
        .collect()
}

const QUILL: &str = "/Quill/QuillSub/CONTENTS";

fn quill(bytes: &[u8]) -> Vec<u8> {
    streams(bytes).into_iter().find(|s| s.0 == QUILL).unwrap().1
}

fn u32at(d: &[u8], o: usize) -> usize {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize
}

/// (tag, instance, offset, len) for every chunk, from our own reading of spec 3.1/3.2.
fn chunks(d: &[u8]) -> Vec<(String, u16, usize, usize)> {
    let mut out = Vec::new();
    let mut off = 0x18;
    loop {
        let n = u16::from_le_bytes([d[off + 2], d[off + 3]]) as usize;
        for i in 0..n {
            let e = off + 8 + 24 * i;
            out.push((
                String::from_utf8_lossy(&d[e + 2..e + 6]).into_owned(),
                u16::from_le_bytes([d[e + 6], d[e + 7]]),
                u32at(d, e + 16),
                u32at(d, e + 20),
            ));
        }
        let next = u32at(d, off + 4);
        if next == 0xFFFF_FFFF {
            break;
        }
        off = next;
    }
    assert_eq!(out.len(), u16::from_le_bytes([d[0xC], d[0xD]]) as usize);
    out
}

fn chunk<'a>(d: &'a [u8], tag: &str, inst: u16) -> &'a [u8] {
    let c = chunks(d)
        .into_iter()
        .find(|c| c.0 == tag && c.1 == inst)
        .unwrap();
    &d[c.2..c.2 + c.3]
}

fn check_quill(d: &[u8]) {
    assert_eq!(d.len() % 512, 0);
    assert_eq!(u32at(d, 0x14), d.len());
    // Header u32 at 0x10 = 512 x directory blocks (Publisher rejects 0x200 with spill).
    let mut blocks = 1;
    let mut blk = 0x18;
    while u32at(d, blk + 4) != 0xFFFF_FFFF {
        blk = u32at(d, blk + 4);
        blocks += 1;
    }
    assert_eq!(u32at(d, 0x10), 512 * blocks);
    let cs = chunks(d);
    let text = cs.iter().find(|c| c.0 == "TEXT").unwrap();
    assert_eq!(text.2, 0x200);
    let strs = chunk(d, "STRS", 0);
    let n = u32at(strs, 0);
    let sum: usize = (0..n).map(|i| u32at(strs, 12 + 4 * i)).sum();
    assert_eq!(sum * 2, text.3, "STRS sums to TEXT");
    // chunks do not overlap and stay inside the stream
    let mut sorted = cs.clone();
    sorted.sort_by_key(|c| c.2);
    for w in sorted.windows(2) {
        assert!(w[0].2 + w[0].3 <= w[1].2, "{w:?}");
    }
    assert!(sorted.last().unwrap().2 + sorted.last().unwrap().3 <= d.len());
    for c in cs.iter().filter(|c| c.0 == "FDPP" || c.0 == "FDPC") {
        assert_eq!(c.2 % 512, 0);
        assert_eq!(c.3, 512);
        let p = &d[c.2..c.2 + 512];
        let n = u16::from_le_bytes([p[0], p[1]]) as usize;
        for i in 0..n {
            let off = u16::from_le_bytes([p[8 + 4 * n + 2 * i], p[9 + 4 * n + 2 * i]]) as usize;
            assert!(off >= 8 + 6 * n && off + u32at(p, off) <= 512);
        }
    }
    // runs cover exactly the TEXT range and the BTE tables end at the text end
    let text_end = text.2 + text.3;
    for (fdp, bte) in [("FDPP", "BTEP"), ("FDPC", "BTEC")] {
        let mut last = 0;
        let pages: Vec<_> = cs.iter().filter(|c| c.0 == fdp).collect();
        for c in &pages {
            let p = &d[c.2..c.2 + 512];
            let n = u16::from_le_bytes([p[0], p[1]]) as usize;
            let mut prev = 0x200;
            for i in 0..n {
                let e = u32at(p, 8 + 4 * i);
                assert!(e > prev, "{fdp} run ends strictly increase");
                prev = e;
            }
            last = prev;
        }
        assert_eq!(last, text_end, "{fdp} runs end at TEXT end");
        let b = chunk(d, bte, 0);
        let np = u32at(b, 0);
        assert_eq!(np, pages.len());
        assert_eq!(u32at(b, 12), 0);
        assert_eq!(u32at(b, 12 + 4 * np), text_end);
        for (i, c) in pages.iter().enumerate() {
            assert_eq!(u32at(b, 12 + 4 * (np + 1) + 4 * i), c.2);
        }
    }
}

fn other_chunks_identical(a: &[u8], b: &[u8], changed: &[&str]) {
    for c in chunks(a) {
        if changed.contains(&c.0.as_str()) {
            continue;
        }
        assert_eq!(chunk(a, &c.0, c.1), chunk(b, &c.0, c.1), "{} {}", c.0, c.1);
    }
}

fn assert_non_quill_identical(a: &[u8], b: &[u8]) {
    let (sa, sb) = (streams(a), streams(b));
    assert_eq!(sa.len(), sb.len());
    for (x, y) in sa.iter().zip(&sb) {
        assert_eq!(x.0, y.0);
        if x.0 != QUILL {
            assert_eq!(x.1, y.1, "{}", x.0);
        }
    }
}

const CHANGED: [&str; 7] = ["TEXT", "STRS", "FDPP", "FDPC", "BTEP", "BTEC", "MCLD_NONE"];

#[kernpage_samples::test]
fn apple_banana_story_zero() {
    let x = gen("apple-banana");
    let y = set_story_text(&x, 0, "Apple pie\nwith cream").unwrap();
    let before = pub_format::read_bytes(&x).unwrap();
    let after = pub_format::read_bytes(&y).unwrap();
    assert_eq!(after.stories, vec!["Apple pie\rwith cream\r", "Banana\r"]);
    let texts = |d: &pub_format::Document| -> Vec<(i32, Option<String>)> {
        d.pages[0]
            .shapes
            .iter()
            .map(|s| (s.left_emu, s.text.clone()))
            .collect()
    };
    let (tb, ta) = (texts(&before), texts(&after));
    for (b, a) in tb.iter().zip(&ta) {
        assert_eq!(b.0, a.0);
    }
    let apple = after.pages[0]
        .shapes
        .iter()
        .find(|s| s.story.is_some() && s.text.as_deref().is_some_and(|t| t.starts_with("Apple")))
        .expect("apple box");
    assert_eq!(apple.paragraphs.len(), 2);
    let banana = after.pages[0]
        .shapes
        .iter()
        .find(|s| s.text.as_deref().is_some_and(|t| t.trim_end() == "Banana"))
        .expect("banana box unchanged");
    assert_eq!(banana.paragraphs.len(), 1);
    assert_non_quill_identical(&x, &y);
    let (qa, qb) = (quill(&x), quill(&y));
    check_quill(&qa);
    check_quill(&qb);
    other_chunks_identical(&qa, &qb, &CHANGED);
    let t = chunk(&qb, "TEXT", 0);
    let units: Vec<u16> = t
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(
        String::from_utf16(&units).unwrap(),
        "Apple pie\rwith cream\rBanana\r"
    );
}

#[kernpage_samples::test]
fn same_text_keeps_the_layout() {
    let x = gen("apple-banana");
    let y = set_story_text(&x, 1, "Banana").unwrap();
    let (qa, qb) = (quill(&x), quill(&y));
    assert_eq!(chunks(&qa), chunks(&qb));
    other_chunks_identical(&qa, &qb, &["FDPC"]); // runs merge: text and its paragraph mark share a record
    assert_eq!(chunk(&qa, "BTEP", 0), chunk(&qb, "BTEP", 0));
    assert_eq!(chunk(&qa, "STRS", 0), chunk(&qb, "STRS", 0));
}

#[kernpage_samples::test]
fn shrinking_and_growing_second_story() {
    let x = gen("apple-banana");
    for t in ["B", "A much longer second story\nwith three\nparagraphs"] {
        let y = set_story_text(&x, 1, t).unwrap();
        let d = pub_format::read_bytes(&y).unwrap();
        assert_eq!(d.stories[0], "Apple\r");
        assert_eq!(d.stories[1], cr(t));
        check_quill(&quill(&y));
        assert_non_quill_identical(&x, &y);
    }
}

#[kernpage_samples::test]
fn bulletin_order_of_service() {
    let x = gen("bulletin");
    let before = pub_format::read_bytes(&x).unwrap();
    let k = before
        .stories
        .iter()
        .position(|s| s.starts_with("Order of Service"))
        .expect("story");
    let new = "Order of Service\nWelcome and announcements\nHymn of praise\nPrayer\nScripture reading\nSermon\nInvitation\nBenediction and a much longer closing line";
    let y = set_single(&x, k, new);
    let after = pub_format::read_bytes(&y).unwrap();
    assert_eq!(after.stories[k], cr(new));
    for (i, s) in before.stories.iter().enumerate() {
        if i != k {
            assert_eq!(&after.stories[i], s);
        }
    }
    // The other boxes keep their text and first-paragraph formatting.
    let flat = |d: &pub_format::Document| -> Vec<_> {
        d.pages
            .iter()
            .flat_map(|p| &p.shapes)
            .map(|s| (s.story, s.text.clone(), s.runs.first().cloned()))
            .collect()
    };
    let (fb, fa) = (flat(&before), flat(&after));
    assert_eq!(fb.len(), fa.len());
    let mut edited = 0;
    for (b, a) in fb.iter().zip(&fa) {
        if a.1.as_deref() == Some(cr(new).as_str()) {
            edited += 1;
            let (br, ar) = (b.2.as_ref().unwrap(), a.2.as_ref().unwrap());
            assert_eq!(
                (&br.bold, &br.italic, br.size_pt),
                (&ar.bold, &ar.italic, ar.size_pt)
            );
            assert_eq!(br.font, ar.font);
        } else {
            assert_eq!(b, a);
        }
    }
    assert_eq!(edited, 1);
    assert_non_quill_identical(&x, &y);
    let (qa, qb) = (quill(&x), quill(&y));
    check_quill(&qb);
    other_chunks_identical(&qa, &qb, &CHANGED);
    // shrink it again
    let z = set_single(&y, k, "Order");
    assert_eq!(pub_format::read_bytes(&z).unwrap().stories[k], "Order\r");
    check_quill(&quill(&z));
}

fn cr(t: &str) -> String {
    format!("{}\r", t.replace('\n', "\r"))
}

fn set_single(x: &[u8], k: usize, t: &str) -> Vec<u8> {
    set_story_text(x, k, t).unwrap()
}

#[kernpage_samples::test]
fn table_story_is_refused_and_bad_index() {
    let x = gen("table");
    let d = pub_format::read_bytes(&x).unwrap();
    assert!(d.pages[0].shapes.iter().any(|s| s.table.is_some()));
    assert!(matches!(
        set_story_text(&x, 0, "x"),
        Err(WriteError::TableStory(0))
    ));
    let a = gen("apple-banana");
    assert!(matches!(
        set_story_text(&a, 7, "x"),
        Err(WriteError::NoSuchStory { .. })
    ));
}

#[kernpage_samples::test]
fn all_gen_samples_with_a_plain_story_edit() {
    for p in kernpage_samples::files("gen", "pub") {
        let x = std::fs::read(&p).unwrap();
        let Ok(doc) = pub_format::read_bytes(&x) else {
            continue;
        };
        if doc.stories.is_empty() {
            continue;
        }
        match set_story_text(&x, 0, "Edited\nstory") {
            Ok(y) => {
                check_quill(&quill(&y));
                let d = pub_format::read_bytes(&y).unwrap();
                assert_eq!(d.stories[0], "Edited\rstory\r", "{}", p.display());
            }
            Err(WriteError::TableStory(_) | WriteError::Quill(_)) => {}
            Err(e) => panic!("{}: {e}", p.display()),
        }
    }
}

// ---- keep formatting (pw-v32.5) ----

use pub_write::{set_story_runs, set_story_text_keep_formatting, story_records};

type Ch = (u16, Vec<u8>, Vec<u8>);

fn fmt_runs(d: &pub_format::Document, story: &str) -> Vec<(String, bool, bool, f32, String)> {
    // runs of the first shape whose text starts with `story`
    let s = d
        .pages
        .iter()
        .flat_map(|p| &p.shapes)
        .find(|s| s.text.as_deref().is_some_and(|t| t.starts_with(story)))
        .expect("shape");
    s.runs
        .iter()
        .map(|r| (r.text.clone(), r.bold, r.italic, r.size_pt, r.font.clone()))
        .collect()
}

#[kernpage_samples::test]
fn bulletin_keeps_heading_and_body_formatting() {
    let x = gen("bulletin");
    let before = pub_format::read_bytes(&x).unwrap();
    let k = before
        .stories
        .iter()
        .position(|s| s.starts_with("Order of Service"))
        .unwrap();
    let old = fmt_runs(&before, "Order of Service");
    assert!(old.len() >= 2, "{old:?}");
    assert!(old[0].1 && (old[0].3 - 16.0).abs() < 0.01);
    assert!(!old[1].1 && (old[1].3 - 10.0).abs() < 0.01);
    let lines: Vec<&str> = before.stories[k]
        .trim_end_matches('\r')
        .split('\r')
        .collect();
    let mut edited: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    edited[2] = "Hymn of praise and thanksgiving".into();
    edited.push("Closing hymn".into());
    let new = edited.join("\n");
    let y = set_story_text_keep_formatting(&x, k, &new).unwrap();
    let after = pub_format::read_bytes(&y).unwrap();
    assert_eq!(after.stories[k], cr(&new));
    let runs = fmt_runs(&after, "Order of Service");
    assert_eq!(runs[0].0, format!("{}\r", lines[0]));
    assert!(runs[0].1 && (runs[0].3 - 16.0).abs() < 0.01, "{runs:?}");
    for r in &runs[1..] {
        assert!(!r.1 && (r.3 - 10.0).abs() < 0.01, "{runs:?}");
    }
    let body: String = runs[1..].iter().map(|r| r.0.as_str()).collect();
    assert!(body.contains("Hymn of praise and thanksgiving\r"));
    assert!(body.ends_with("Closing hymn\r"));
    assert_non_quill_identical(&x, &y);
    check_quill(&quill(&y));
}

#[kernpage_samples::test]
fn story_records_lists_original_runs() {
    let x = gen("bulletin");
    let before = pub_format::read_bytes(&x).unwrap();
    let k = before
        .stories
        .iter()
        .position(|s| s.starts_with("Order of Service"))
        .unwrap();
    let r = story_records(&x, k).unwrap();
    assert_eq!(r.text.len(), before.stories[k].encode_utf16().count());
    assert!(r.char_records.len() >= 2);
    assert_eq!(r.runs.last().unwrap().end, r.text.len());
    // identical re-set via explicit runs keeps the text
    let paras: Vec<pub_write::NewParagraph> = vec![pub_write::NewParagraph {
        runs: vec![pub_write::NewRun {
            text: "Only\r".into(),
            char_record: pub_write::CharRecordRef::Original(0),
        }],
        para_record: pub_write::ParaRecordRef::Original(0),
    }];
    let y = set_story_runs(&x, k, &paras).unwrap();
    assert_eq!(pub_format::read_bytes(&y).unwrap().stories[k], "Only\r");
}

#[kernpage_samples::test]
fn formatting_sample_one_paragraph_edit_keeps_the_rest() {
    let x = gen("formatting");
    let before = pub_format::read_bytes(&x).unwrap();
    // pick the story with the most runs
    let (k, _) = before
        .stories
        .iter()
        .enumerate()
        .max_by_key(|(i, _)| story_records(&x, *i).map(|r| r.runs.len()).unwrap_or(0))
        .unwrap();
    let old = before.stories[k].clone();
    let mut paras: Vec<String> = old
        .trim_end_matches('\r')
        .split('\r')
        .map(String::from)
        .collect();
    assert!(paras.len() > 2, "{paras:?}");
    let mid = paras.len() / 2;
    let orig_mid = paras[mid].clone();
    paras[mid] = format!("{orig_mid} EDITED");
    let y = set_story_text_keep_formatting(&x, k, &paras.join("\n")).unwrap();
    let a = story_records(&x, k).unwrap();
    let b = story_records(&y, k).unwrap();
    // Per-character resolved record bytes and paragraph record bytes.
    let per_char = |r: &pub_write::StoryRecords| -> Vec<Ch> {
        (0..r.text.len())
            .map(|i| {
                let c = r.runs.iter().find(|x| i >= x.start && i < x.end).unwrap();
                let p = r
                    .paragraphs
                    .iter()
                    .find(|x| i >= x.start && i < x.end)
                    .unwrap();
                (
                    r.text[i],
                    r.char_records[c.record].clone(),
                    r.para_records[p.record].clone(),
                )
            })
            .collect()
    };
    let (ca, cb) = (per_char(&a), per_char(&b));
    let split = |c: &[Ch]| -> Vec<Vec<Ch>> {
        let mut out = vec![];
        let mut cur = vec![];
        for x in c {
            cur.push(x.clone());
            if x.0 == 0x0D {
                out.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    };
    let (pa, pb) = (split(&ca), split(&cb));
    assert_eq!(pa.len(), pb.len());
    for i in 0..pa.len() {
        if i != mid {
            assert_eq!(pa[i], pb[i], "paragraph {i} changed");
        } else {
            // same paragraph record, and the old char styles of the shared prefix
            assert_eq!(pa[i][0].2, pb[i][0].2);
            assert_eq!(pa[i][0].1, pb[i][0].1);
        }
    }
    // The reader agrees on the other stories and the resulting file is well formed.
    let after = pub_format::read_bytes(&y).unwrap();
    assert_eq!(after.stories[k], cr(&paras.join("\n")));
    check_quill(&quill(&y));
}

fn fdp_pages(d: &[u8]) -> (usize, usize) {
    let cs = chunks(d);
    (
        cs.iter().filter(|c| c.0 == "FDPP").count(),
        cs.iter().filter(|c| c.0 == "FDPC").count(),
    )
}

/// A story of ~200 paragraphs with alternating character and paragraph records needs several
/// FDPC and FDPP pages (spec 3.9); text and per-run/per-paragraph formatting survive.
#[kernpage_samples::test]
fn long_story_grows_fdp_pages_and_directory() {
    for name in ["bulletin", "formatting"] {
        let x = gen(name);
        let before = pub_format::read_bytes(&x).unwrap();
        let k = before
            .stories
            .iter()
            .position(|s| s.starts_with("Order of Service") || name == "formatting")
            .unwrap();
        let recs = story_records(&x, k).unwrap();
        let (nc, np) = (recs.char_records.len(), recs.para_records.len());
        assert!(nc >= 2, "{name}: need two character records");
        // Original per-record properties as seen through pub-format.
        let shape = before
            .pages
            .iter()
            .flat_map(|p| &p.shapes)
            .find(|s| s.story.is_some() && s.text.as_deref() == Some(before.stories[k].as_str()))
            .expect("shape of story");
        let run_props = |r: &pub_format::TextRun| (r.bold, r.italic, r.size_pt.to_bits());
        let mut char_props = vec![None; nc];
        for cr in &recs.runs {
            let mut at = 0;
            for r in shape.runs.iter() {
                let len = r.text.encode_utf16().count();
                if at <= cr.start && cr.start < at + len {
                    char_props[cr.record].get_or_insert(run_props(r));
                }
                at += len;
            }
        }
        let mut para_props = vec![None; np];
        for (pi, pr) in recs.paragraphs.iter().enumerate() {
            let p = &shape.paragraphs[pi];
            para_props[pr.record].get_or_insert((p.alignment, p.spacing.space_after_pt.to_bits()));
        }

        const N: usize = 200;
        let paras: Vec<pub_write::NewParagraph> = (0..N)
            .map(|i| pub_write::NewParagraph {
                runs: vec![
                    pub_write::NewRun {
                        text: format!("Item {i} "),
                        char_record: pub_write::CharRecordRef::Original(i % nc),
                    },
                    pub_write::NewRun {
                        text: format!("detail {i}"),
                        char_record: pub_write::CharRecordRef::Original((i + 1) % nc),
                    },
                ],
                para_record: pub_write::ParaRecordRef::Original(i % np),
            })
            .collect();
        let y = set_story_runs(&x, k, &paras).unwrap();
        let (fp, fc) = fdp_pages(&quill(&y));
        assert!(fp >= 3 && fc >= 3, "{name}: pages FDPP {fp} FDPC {fc}");
        check_quill(&quill(&y));
        assert_non_quill_identical(&x, &y);

        let after = pub_format::read_bytes(&y).unwrap();
        let expect: String = (0..N).map(|i| format!("Item {i} detail {i}\r")).collect();
        assert_eq!(after.stories[k], expect, "{name}");
        let shape = after
            .pages
            .iter()
            .flat_map(|p| &p.shapes)
            .find(|s| s.story.is_some() && s.text.as_deref() == Some(expect.as_str()))
            .expect("shape of story after");
        assert_eq!(shape.paragraphs.len(), N);
        for (i, p) in shape.paragraphs.iter().enumerate() {
            let want_p = para_props[i % np].unwrap();
            assert_eq!(
                (p.alignment, p.spacing.space_after_pt.to_bits()),
                want_p,
                "{name} paragraph {i}"
            );
            let mut at = 0usize;
            let mut seen = 0;
            for r in &p.runs {
                let len = r.text.encode_utf16().count();
                // runs may merge or split at the mark; check each character's properties
                for (ci, ch) in r.text.chars().enumerate() {
                    let _ = ch;
                    let off = at + ci;
                    let first_len = format!("Item {i} ").chars().count();
                    let rec = if off < first_len {
                        i % nc
                    } else {
                        (i + 1) % nc
                    };
                    if ch == '\r' {
                        continue; // the mark's own record is not asserted
                    }
                    assert_eq!(
                        Some(run_props(r)),
                        char_props[rec],
                        "{name} para {i} off {off}"
                    );
                    seen += 1;
                }
                at += len;
            }
            assert!(seen > 0);
        }
        // Idempotent: editing the grown file again keeps it valid.
        let z = set_story_text(&y, k, "short\nagain").unwrap();
        check_quill(&quill(&z));
        assert_eq!(
            pub_format::read_bytes(&z).unwrap().stories[k],
            "short\ragain\r"
        );
        let (zp, zc) = fdp_pages(&quill(&z));
        assert!(zp < fp && zc < fc, "pages shrink back");
    }
}

/// Enough pages to need two and three directory blocks (spec 3.1): the chain stays valid.
#[kernpage_samples::test]
fn huge_story_spills_into_more_directory_blocks() {
    let x = gen("bulletin");
    let before = pub_format::read_bytes(&x).unwrap();
    let k = before
        .stories
        .iter()
        .position(|s| s.starts_with("Order of Service"))
        .unwrap();
    for (lines, min_blocks) in [(700usize, 2usize), (2500, 3)] {
        let text: Vec<String> = (0..lines).map(|i| format!("Line {i}")).collect();
        let y = set_story_text(&x, k, &text.join("\n")).unwrap();
        let q = quill(&y);
        check_quill(&q);
        assert_eq!(q[0x10..0x14], ((512 * min_blocks) as u32).to_le_bytes());
        let after = pub_format::read_bytes(&y).unwrap();
        assert_eq!(after.stories[k], cr(&text.join("\n")));
        assert_eq!(after.stories.len(), before.stories.len());
        assert_non_quill_identical(&x, &y);
    }
}
