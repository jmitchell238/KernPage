//! Writing bullets, numbering and indents (spec 4R): the paragraph property records written
//! onto `list-empty.pub` are compared byte for byte with the ones Publisher 2021 saved in
//! `samples/gen/list-*.pub`, and read back with `pub-format`.

use pub_format::{list_labels, ListInfo, ListType, ParaIndents};
use pub_write::{add_text_box, set_story_lists, story_records, ListSpan};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

const NAMES: [&str; 9] = [
    "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine",
];

fn text_of(n: usize) -> String {
    NAMES[..n].join("\r")
}

fn story_of(bytes: &[u8]) -> usize {
    pub_format::read_bytes(bytes)
        .unwrap()
        .stories
        .iter()
        .position(|t| t.contains("One"))
        .unwrap()
}

/// `list-empty.pub` plus a text box holding `text` (our own writer, not Publisher's).
fn built(text: &str) -> Vec<u8> {
    add_text_box(&gen("list-empty"), 0, 72.0, 72.0, 300.0, 300.0, text).unwrap()
}

/// What one paragraph should get.
#[derive(Clone)]
enum P {
    Plain,
    List(ListInfo),
    Indents(ParaIndents),
}

fn apply(bytes: &[u8], specs: &[P]) -> Vec<u8> {
    let story = story_of(bytes);
    let recs = story_records(bytes, story).unwrap();
    assert_eq!(recs.paragraphs.len(), specs.len());
    let spans: Vec<ListSpan> = recs
        .paragraphs
        .iter()
        .zip(specs)
        .map(|(p, s)| match s {
            P::Plain => ListSpan::new(p.start, p.end, None),
            P::List(l) => ListSpan::new(p.start, p.end, Some(l.clone())),
            P::Indents(i) => ListSpan {
                start: p.start,
                end: p.end,
                list: None,
                indents: *i,
            },
        })
        .collect();
    set_story_lists(bytes, story, &spans).unwrap()
}

/// The paragraph property records, one per paragraph, of the story containing "One".
fn para_records(bytes: &[u8]) -> Vec<Vec<u8>> {
    let r = story_records(bytes, story_of(bytes)).unwrap();
    r.paragraphs
        .iter()
        .map(|p| r.para_records[p.record].clone())
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

/// Write `specs` onto a fresh text box of `n` paragraphs and require the same property
/// records, font table and read-back labels as Publisher's `list-{sample}.pub`.
fn same_as_publisher(sample: &str, text: &str, specs: &[P]) -> Vec<u8> {
    let ours = apply(&built(text), specs);
    let theirs = gen(&format!("list-{sample}"));
    let (a, b) = (para_records(&ours), para_records(&theirs));
    assert_eq!(a.len(), b.len(), "{sample}: paragraph count");
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!(hex(x), hex(y), "{sample}: FDPP record of paragraph {i}");
    }
    let (da, db) = (
        pub_format::read_bytes(&ours).unwrap(),
        pub_format::read_bytes(&theirs).unwrap(),
    );
    // The Quill FONT table order is checked through the bullet font indexes inside the\n    // identical records and the font names of the read-back below. Publisher also lists\n    // the fonts in use in the Contents font table (kind 108); we do not (spec 4R.8), so\n    // `Document::fonts` differs.\n    let _ = (&da, &db);
    let labels = |d: &pub_format::Document| {
        let s = d
            .pages
            .iter()
            .flat_map(|p| p.shapes.iter())
            .find(|s| s.text.as_deref().is_some_and(|t| t.contains("One")))
            .unwrap();
        (list_labels(&s.paragraphs), s.paragraphs.iter().map(|p| (p.list.clone(), p.indents)).collect::<Vec<_>>())
    };
    assert_eq!(labels(&da), labels(&db), "{sample}: read back");
    ours
}

fn n(kind: ListType) -> P {
    P::List(ListInfo::numbered(kind))
}

fn bullet() -> P {
    P::List(ListInfo::bullet())
}

fn with(kind: ListType, f: impl FnOnce(&mut ListInfo)) -> P {
    let mut l = ListInfo::numbered(kind);
    f(&mut l);
    P::List(l)
}

#[kernpage_samples::test]
fn bullets_match_publisher() {
    let t = text_of(3);
    same_as_publisher("bullet-default", &t, &[P::Plain, bullet(), P::Plain]);
    same_as_publisher("bullet-all", &t, &[bullet(), bullet(), bullet()]);
    let wing = |c| P::List(ListInfo::bullet_with(c, "Wingdings", 10.0));
    same_as_publisher("bullet-wingdings", &t, &[wing(0xA7), wing(0xFC), P::Plain]);
    same_as_publisher(
        "bullet-chars",
        &t,
        &[
            P::List(ListInfo::bullet_with(0x2022, "Symbol", 10.0)),
            P::List(ListInfo::bullet_with(42, "Symbol", 10.0)),
            P::List(ListInfo::bullet_with(b'o'.into(), "Courier New", 10.0)),
        ],
    );
    same_as_publisher(
        "bullet-size",
        &t,
        &[
            P::List(ListInfo::bullet_with(0xB7, "Symbol", 20.0)),
            P::List(ListInfo::bullet_with(0xB7, "Symbol", 5.0)),
            P::Plain,
        ],
    );
    let ind = |pt| {
        let mut l = ListInfo::bullet();
        l.list_indent_pt = pt;
        P::List(l)
    };
    same_as_publisher("bullet-indent", &t, &[ind(36.0), ind(0.0), P::Plain]);
}

#[kernpage_samples::test]
fn numbering_matches_publisher() {
    let t = text_of(3);
    let all = |p: P| [p.clone(), p.clone(), p];
    same_as_publisher("num-1dot", &t, &all(n(ListType::Arabic)));
    same_as_publisher("num-1dot-each", &t, &all(n(ListType::Arabic)));
    same_as_publisher("num-idot", &t, &all(n(ListType::LowerRoman)));
    same_as_publisher(
        "num-1paren",
        &t,
        &all(with(ListType::Arabic, |l| l.separator = 0)),
    );
    same_as_publisher(
        "num-aparen",
        &t,
        &all(with(ListType::LowerLetter, |l| l.separator = 0)),
    );
    same_as_publisher("num-start5", &t, &all(with(ListType::Arabic, |l| l.start = 5)));
    same_as_publisher(
        "num-start-p2",
        &t,
        &[
            n(ListType::Arabic),
            with(ListType::Arabic, |l| l.start = 7),
            with(ListType::Arabic, |l| l.start = 7),
        ],
    );
    same_as_publisher(
        "num-indent36",
        &t,
        &all(with(ListType::Arabic, |l| l.list_indent_pt = 36.0)),
    );
    same_as_publisher(
        "num-bullet-mix",
        &t,
        &[n(ListType::Arabic), bullet(), n(ListType::Arabic)],
    );
}

#[kernpage_samples::test]
fn runs_types_and_separators_match_publisher() {
    let a = || n(ListType::Arabic);
    same_as_publisher(
        "num-mixed",
        &text_of(5),
        &[a(), a(), P::Plain, a(), a()],
    );
    same_as_publisher("num-empty", "One\r\rThree", &[a(), a(), a()]);
    let kinds = [
        ListType::Arabic,
        ListType::UpperRoman,
        ListType::LowerRoman,
        ListType::UpperLetter,
        ListType::LowerLetter,
        ListType::Ordinal,
        ListType::CardinalText,
        ListType::OrdinalText,
        ListType::LeadingZero,
    ];
    let specs: Vec<P> = kinds.iter().map(|&k| n(k)).collect();
    same_as_publisher("num-types", &text_of(9), &specs);
    let seps: Vec<P> = (0..9u8)
        .map(|i| with(ListType::Arabic, |l| l.separator = i))
        .collect();
    same_as_publisher("num-seps", &text_of(9), &seps);
}

#[kernpage_samples::test]
fn plain_indents_match_publisher() {
    let i = |l, f, r| {
        P::Indents(ParaIndents {
            left_pt: l,
            first_line_pt: f,
            right_pt: r,
        })
    };
    same_as_publisher(
        "indent-plain",
        &text_of(3),
        &[i(36.0, -18.0, 0.0), i(18.0, 18.0, 0.0), i(0.0, 0.0, 18.0)],
    );
}

#[kernpage_samples::test]
fn clearing_a_bullet_leaves_publishers_residue() {
    // Publisher: bullet on p2, then SetListType(255).
    let with_bullet = apply(&built(&text_of(3)), &[P::Plain, bullet(), P::Plain]);
    let out = apply(&with_bullet, &[P::Plain, P::Plain, P::Plain]);
    let theirs = para_records(&gen("list-bullet-then-none"));
    let ours = para_records(&out);
    assert_eq!(
        ours.iter().map(|r| hex(r)).collect::<Vec<_>>(),
        theirs.iter().map(|r| hex(r)).collect::<Vec<_>>()
    );
    // The reader ignores the residue.
    let doc = pub_format::read_bytes(&out).unwrap();
    let s = doc.pages[0].shapes.iter().find(|s| s.text.as_deref().is_some_and(|t| t.contains("One"))).unwrap();
    assert!(s.paragraphs.iter().all(|p| p.list.is_none() && p.indents == ParaIndents::default()));
}

#[kernpage_samples::test]
fn bullet_turned_into_numbering_keeps_the_old_size() {
    let t = text_of(3);
    let b = apply(
        &built(&t),
        &[P::Plain, P::List(ListInfo::bullet_with(0xB7, "Arial", 10.0)), P::Plain],
    );
    let out = apply(&b, &[P::Plain, n(ListType::Arabic), P::Plain]);
    let theirs = para_records(&gen("list-bullet-font-then-num"));
    assert_eq!(
        para_records(&out).iter().map(|r| hex(r)).collect::<Vec<_>>(),
        theirs.iter().map(|r| hex(r)).collect::<Vec<_>>()
    );
}

#[kernpage_samples::test]
fn lists_survive_text_and_character_formatting_untouched() {
    // Text, character runs and the other paragraphs are unchanged by a list edit.
    let src = built(&text_of(3));
    let out = apply(&src, &[bullet(), P::Plain, bullet()]);
    let (a, b) = (
        pub_format::read_bytes(&src).unwrap(),
        pub_format::read_bytes(&out).unwrap(),
    );
    assert_eq!(a.text, b.text);
    assert_eq!(a.stories, b.stories);
    let runs = |bytes: &[u8]| story_records(bytes, story_of(bytes)).unwrap().runs;
    assert_eq!(runs(&src), runs(&out));
    // Every paragraph keeps its own run.
    assert_eq!(para_records(&out).len(), 3);
    // Idempotent: applying the same formatting again changes nothing.
    let again = apply(&out, &[bullet(), P::Plain, bullet()]);
    assert_eq!(para_records(&out), para_records(&again));
}

#[kernpage_samples::test]
fn works_on_a_new_publication_and_reads_back() {
    let blank = pub_write::new_publication(&pub_write::NewPublication::letter(1)).unwrap();
    let doc = add_text_box(&blank, 0, 72.0, 72.0, 300.0, 200.0, &text_of(4)).unwrap();
    let specs = [
        P::List(ListInfo::bullet_with(0xA7, "Wingdings", 12.0)),
        n(ListType::UpperRoman),
        with(ListType::LowerLetter, |l| {
            l.separator = 0;
            l.start = 3;
        }),
        P::Plain,
    ];
    let out = apply(&doc, &specs);
    let d = pub_format::read_bytes(&out).unwrap();
    let s = d.pages[0].shapes.iter().find(|s| s.text.as_deref().is_some_and(|t| t.contains("One"))).unwrap();
    let labels: Vec<_> = list_labels(&s.paragraphs).into_iter().map(|l| l.unwrap_or_default()).collect();
    assert_eq!(labels, ["\u{a7}", "I.", "c)", ""]);
    let l0 = s.paragraphs[0].list.as_ref().unwrap();
    assert_eq!((l0.bullet_font.as_str(), l0.bullet_size_pt), ("Wingdings", Some(12.0)));
}

/// Writes the files for the manual Publisher 2021 check (git-ignored `out/lists-w/`) and
/// the expected COM read-back of every paragraph: `cargo test -p pub-write --test lists
/// -- --ignored write_publisher_verification_files`.
#[kernpage_samples::test]
#[ignore]
fn write_publisher_verification_files() {
    let wing = |c, pt| P::List(ListInfo::bullet_with(c, "Wingdings", pt));
    let a = || n(ListType::Arabic);
    let kinds9: Vec<P> = [0, 1, 2, 3, 4, 5, 6, 7, 22]
        .iter()
        .map(|&c| n(ListType::from_code(c)))
        .collect();
    let seps9: Vec<P> = (0..9u8)
        .map(|i| with(ListType::Arabic, |l| l.separator = i))
        .collect();
    let mut indented = ListInfo::bullet();
    indented.list_indent_pt = 36.0;
    let mut zero = ListInfo::bullet();
    zero.list_indent_pt = 0.0;
    let cases: Vec<(&str, usize, Vec<P>)> = vec![
        ("w-bullet-default", 3, vec![P::Plain, bullet(), P::Plain]),
        ("w-bullet-wingdings", 3, vec![wing(0xA7, 14.0), wing(0xFC, 10.0), P::Plain]),
        ("w-num-1dot", 3, vec![a(), a(), a()]),
        ("w-num-aparen", 3, (0..3).map(|_| with(ListType::LowerLetter, |l| l.separator = 0)).collect()),
        ("w-num-idot", 3, (0..3).map(|_| n(ListType::LowerRoman)).collect()),
        ("w-num-start5", 3, (0..3).map(|_| with(ListType::Arabic, |l| l.start = 5)).collect()),
        ("w-mixed", 5, vec![a(), a(), P::Plain, a(), bullet()]),
        ("w-indent", 3, vec![
            P::List(indented),
            P::Indents(ParaIndents { left_pt: 54.0, first_line_pt: -10.0, right_pt: 12.0 }),
            P::List(zero),
        ]),
        ("w-num-types", 9, kinds9),
        ("w-num-seps", 9, seps9),
    ];
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out/lists-w");
    std::fs::create_dir_all(&dir).unwrap();
    let mut expect = String::new();
    let mut list_file = String::new();
    let mut emit = |name: &str, bytes: &[u8], specs: &[P], cleared: Option<usize>| {
        let path = dir.canonicalize().unwrap().join(format!("{name}.pub"));
        std::fs::write(&path, bytes).unwrap();
        list_file.push_str(&format!("{}\n", path.display().to_string().trim_start_matches(r"\?\")));
        for (i, s) in specs.iter().enumerate() {
            let f = |v: f32| format!("{}", v.round() as i64);
            let line = match s {
                P::List(l) if l.kind.is_bullet() => format!(
                    "23;{};{};{};-;-;{};{};0;{}",
                    l.bullet_char,
                    if l.bullet_font.is_empty() { "Symbol" } else { &l.bullet_font },
                    f(l.bullet_size_pt.unwrap_or(10.0)),
                    f(ParaIndents::for_list(l.list_indent_pt).left_pt),
                    f(ParaIndents::for_list(l.list_indent_pt).first_line_pt),
                    f(l.list_indent_pt),
                ),
                P::List(l) => format!(
                    "{};-;-;-;{};{};{};{};0;{}",
                    l.kind.code(),
                    l.start,
                    u32::from(l.separator) << 16,
                    f(l.list_indent_pt),
                    f(-l.list_indent_pt),
                    f(l.list_indent_pt),
                ),
                P::Plain => format!("255;-;-;-;-;-;0;0;0;{}", if cleared == Some(i) { 0 } else { 18 }),
                P::Indents(x) => format!(
                    "255;-;-;-;-;-;{};{};{};18",
                    f(x.left_pt),
                    f(x.first_line_pt),
                    f(x.right_pt)
                ),
            };
            expect.push_str(&format!("{name}|p{}|{line}\n", i + 1));
        }
    };
    for (name, count, specs) in &cases {
        let out = apply(&built(&text_of(*count)), specs);
        emit(name, &out, specs, None);
    }
    // A list that was cleared again (Publisher's residue stays in the records).
    let all = [bullet(), bullet(), bullet()];
    let on = apply(&built(&text_of(3)), &all);
    let off_specs = [bullet(), P::Plain, bullet()];
    let off = apply(&on, &off_specs);
    emit("w-cleared", &off, &off_specs, Some(1));
    std::fs::write(dir.join("files.txt"), list_file).unwrap();
    std::fs::write(dir.join("expect.txt"), expect).unwrap();
}
