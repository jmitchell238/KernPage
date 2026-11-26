//! Tests driven by `samples/gen/manifest.json`: the exact values set through Publisher's
//! automation (positions in points, page top-left origin).

use pub_format::{read, Alignment, Color, Document, PictureFormat, Shape, ShapeGeometry};
use serde_json::Value;
use std::path::PathBuf;

const EMU_PER_POINT: f64 = 12_700.0;
const TOLERANCE_PT: f64 = 0.5;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest() -> Value {
    let text = std::fs::read_to_string(root().join("samples/gen/manifest.json")).unwrap();
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}

fn load(name: &str) -> Document {
    read(&kernpage_samples::resolve(root().join(format!("samples/gen/{name}.pub")))).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn num(v: &Value, key: &str) -> f64 {
    v[key]
        .as_f64()
        .unwrap_or_else(|| panic!("manifest: missing number `{key}` in {v}"))
}

fn all_shapes(doc: &Document) -> impl Iterator<Item = &Shape> {
    doc.pages.iter().flat_map(|p| p.shapes.iter())
}

/// Shape box in points from the page top-left: (left, top, width, height).
fn shape_pt(doc: &Document, s: &Shape) -> (f64, f64, f64, f64) {
    (
        f64::from(doc.page_width_emu) / 2.0 / EMU_PER_POINT + f64::from(s.left_emu) / EMU_PER_POINT,
        f64::from(doc.page_height_emu) / 2.0 / EMU_PER_POINT + f64::from(s.top_emu) / EMU_PER_POINT,
        f64::from(s.right_emu - s.left_emu) / EMU_PER_POINT,
        f64::from(s.bottom_emu - s.top_emu) / EMU_PER_POINT,
    )
}

fn geometry_matches(doc: &Document, s: &Shape, b: &Value) -> bool {
    let (l, t, w, h) = shape_pt(doc, s);
    (l - num(b, "left_pt")).abs() <= TOLERANCE_PT
        && (t - num(b, "top_pt")).abs() <= TOLERANCE_PT
        && (w - num(b, "width_pt")).abs() <= TOLERANCE_PT
        && (h - num(b, "height_pt")).abs() <= TOLERANCE_PT
}

fn alignment_of(automation: i64) -> Alignment {
    match automation {
        2 => Alignment::Center,
        3 => Alignment::Right,
        4 => Alignment::Justify,
        _ => Alignment::Left,
    }
}

/// Manifest colours are BGR integers (255 = red); 0 means the default black.
fn bgr(v: i64) -> Color {
    Color::Rgb(
        (v & 0xFF) as u8,
        (v >> 8 & 0xFF) as u8,
        (v >> 16 & 0xFF) as u8,
    )
}

fn color_matches(got: Option<Color>, manifest: i64) -> bool {
    if manifest == 0 {
        // Default black: no colour, scheme colour 0 or a literal black.
        matches!(
            got,
            None | Some(Color::Scheme(0)) | Some(Color::Rgb(0, 0, 0))
        )
    } else {
        got == Some(bgr(manifest))
    }
}

/// `Ok` when every manifest paragraph matches the shape's paragraphs, else why not.
fn paragraphs_match(s: &Shape, b: &Value) -> Result<(), String> {
    let want = b["paragraphs"].as_array().unwrap();
    if s.paragraphs.len() != want.len() {
        return Err(format!(
            "{} paragraphs, expected {}",
            s.paragraphs.len(),
            want.len()
        ));
    }
    for (i, (got, w)) in s.paragraphs.iter().zip(want).enumerate() {
        let text: String = got.runs.iter().map(|r| r.text.as_str()).collect();
        let expect = w["text"].as_str().unwrap();
        if text != expect {
            return Err(format!("paragraph {i}: text {text:?} != {expect:?}"));
        }
        let alignment = alignment_of(w["alignment"].as_i64().unwrap());
        if got.alignment != alignment {
            return Err(format!(
                "paragraph {i}: alignment {:?} != {alignment:?}",
                got.alignment
            ));
        }
        for r in &got.runs {
            let flag = |k: &str| w[k].as_i64().unwrap() != 0;
            let size = w["size"].as_f64().unwrap();
            let ok = r.font == w["font"].as_str().unwrap()
                && (f64::from(r.size_pt) - size).abs() < 0.01
                && r.bold == flag("bold")
                && r.italic == flag("italic")
                && r.underline == flag("underline")
                && color_matches(r.color, w["color"].as_i64().unwrap());
            if !ok {
                return Err(format!("paragraph {i}: run {r:?} does not match {w}"));
            }
        }
    }
    Ok(())
}

fn check_boxes(name: &str) -> Document {
    let doc = load(name);
    let m = manifest();
    for b in m[name]["boxes"].as_array().unwrap() {
        let candidates: Vec<&Shape> = all_shapes(&doc)
            .filter(|s| geometry_matches(&doc, s, b))
            .collect();
        assert!(
            !candidates.is_empty(),
            "{name}/{}: no shape at the manifest geometry; shapes: {:?}",
            b["name"],
            all_shapes(&doc)
                .map(|s| shape_pt(&doc, s))
                .collect::<Vec<_>>()
        );
        let results: Vec<Result<(), String>> =
            candidates.iter().map(|s| paragraphs_match(s, b)).collect();
        assert!(
            results.iter().any(Result::is_ok),
            "{name}/{}: {results:?}",
            b["name"]
        );
    }
    doc
}

#[kernpage_samples::test]
fn apple_banana_matches_manifest() {
    let doc = check_boxes("apple-banana");
    assert_eq!(all_shapes(&doc).count(), 2);
}

#[kernpage_samples::test]
fn formatting_matches_manifest() {
    let doc = check_boxes("formatting");
    assert_eq!(all_shapes(&doc).count(), 1);
}

#[kernpage_samples::test]
fn legal_matches_manifest() {
    let doc = check_boxes("legal");
    let m = manifest();
    assert!(
        (f64::from(doc.page_width_emu) / EMU_PER_POINT - num(&m["legal"], "page_width_pt")).abs()
            < 0.01
    );
    assert!(
        (f64::from(doc.page_height_emu) / EMU_PER_POINT - num(&m["legal"], "page_height_pt")).abs()
            < 0.01
    );
}

fn picture_shape<'a>(doc: &'a Document, m: &Value) -> &'a Shape {
    let want = std::fs::read(root().join("tools/assets/test-picture.png")).unwrap();
    let s = all_shapes(doc)
        .find(|s| s.picture.is_some())
        .expect("a shape with a picture");
    assert!(
        geometry_matches(doc, s, &m["picture"]),
        "{:?}",
        shape_pt(doc, s)
    );
    let pic = s.picture.as_ref().unwrap();
    assert_eq!(pic.format, PictureFormat::Png);
    assert_eq!(pic.bytes, want);
    s
}

#[kernpage_samples::test]
fn picture_matches_manifest() {
    let doc = load("picture");
    picture_shape(&doc, &manifest()["picture"]);
    assert_eq!(all_shapes(&doc).count(), 1);
}

fn page_texts(doc: &Document, page: usize) -> Vec<String> {
    doc.pages[page]
        .shapes
        .iter()
        .filter_map(|s| s.text.clone())
        .collect()
}

#[kernpage_samples::test]
fn bulletin_matches_manifest() {
    let doc = check_boxes("bulletin");
    let m = manifest();
    assert_eq!(
        doc.pages.len(),
        m["bulletin"]["pages"].as_u64().unwrap() as usize
    );
    picture_shape(&doc, &m["bulletin"]);
    // Page 1: title, order of service and the picture; page 2: announcements, prayer list.
    let starts =
        |page: usize, prefix: &str| page_texts(&doc, page).iter().any(|t| t.starts_with(prefix));
    assert!(starts(0, "Grace Community Church"));
    assert!(starts(0, "Order of Service"));
    assert!(doc.pages[0].shapes.iter().any(|s| s.picture.is_some()));
    assert!(starts(1, "Announcements"));
    assert!(starts(1, "Prayer List"));
    assert!(doc.pages[1].shapes.iter().all(|s| s.picture.is_none()));
}

#[kernpage_samples::test]
fn colors_match_manifest() {
    let doc = check_boxes("colors");
    assert_eq!(all_shapes(&doc).count(), 1);
    // The default paragraph has no colour at all; the others resolve to literal RGB.
    let s = all_shapes(&doc).next().unwrap();
    assert_eq!(s.runs[0].color, None);
    assert!(s
        .runs
        .iter()
        .any(|r| r.color == Some(Color::Rgb(255, 0, 0))));
}

#[kernpage_samples::test]
fn shapes_match_manifest() {
    let doc = load("shapes");
    let m = manifest();
    let want = m["shapes"]["shapes"].as_array().unwrap();
    assert_eq!(all_shapes(&doc).count(), want.len());
    for b in want {
        let name = b["name"].as_str().unwrap();
        let s = all_shapes(&doc)
            .find(|s| geometry_matches(&doc, s, b))
            .unwrap_or_else(|| panic!("{name}: no shape at the manifest position"));
        let geometry = match b["autoshape"].as_i64().unwrap() {
            1 => ShapeGeometry::Rectangle,
            9 => ShapeGeometry::Ellipse,
            -2 => ShapeGeometry::Line, // line shapes report type 9, autoshape "mixed"
            other => panic!("{name}: unexpected autoshape {other}"),
        };
        assert_eq!(s.geometry, Some(geometry), "{name}");
        match b["fill_visible"].as_i64().unwrap() {
            0 => assert_eq!(s.fill, None, "{name}: fill should be off"),
            _ => assert_eq!(s.fill, Some(bgr(b["fill_rgb"].as_i64().unwrap())), "{name}"),
        }
        match b["line_visible"].as_i64().unwrap() {
            0 => assert_eq!(s.line, None, "{name}: line should be off"),
            _ => {
                let (c, w) = s
                    .line
                    .unwrap_or_else(|| panic!("{name}: line should be on"));
                let rgb = b["line_rgb"].as_i64().unwrap();
                assert!(
                    color_matches(Some(c), rgb) || c == bgr(rgb),
                    "{name}: {c:?}"
                );
                let pt = f64::from(w) / EMU_PER_POINT;
                assert!(
                    (pt - num(b, "line_weight_pt")).abs() < 0.01,
                    "{name}: {pt} pt"
                );
            }
        }
    }
}

#[kernpage_samples::test]
fn table_matches_manifest() {
    let doc = load("table");
    let m = &manifest()["table"];
    assert_eq!(all_shapes(&doc).count(), 1);
    let s = all_shapes(&doc).next().unwrap();
    assert!(geometry_matches(&doc, s, m), "{:?}", shape_pt(&doc, s));
    let t = s.table.as_ref().expect("a table");
    assert_eq!(
        (f64::from(t.rows), f64::from(t.columns)),
        (num(m, "rows"), num(m, "columns"))
    );
    let cell = |k: &str| m["cells"][k].as_str().unwrap().to_string();
    assert_eq!(
        t.cells,
        vec![cell("r1c1"), cell("r1c2"), cell("r2c1"), cell("r2c2")]
    );
    assert!(all_shapes(&doc).all(|s| s.picture.is_none()));
}

fn scheme_of(doc: &Document) -> &pub_format::ColorScheme {
    doc.color_scheme
        .as_ref()
        .expect("document has a kind-92 colour scheme")
}

#[kernpage_samples::test]
fn schemes_match_manifest() {
    let m = manifest();
    let doc = load("schemes");
    let scheme = scheme_of(&doc);
    assert_eq!(scheme.name, m["schemes"]["scheme_name"].as_str().unwrap());
    let slots = m["schemes"]["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 8);
    for slot in slots {
        let n = slot["slot"].as_u64().unwrap() as usize;
        let Color::Rgb(r, g, b) = bgr(slot["rgb"].as_i64().unwrap()) else {
            unreachable!()
        };
        assert_eq!(scheme.colors[n - 1], [r, g, b], "slot {n}");
        // Scheme index n-1 resolves to the same colour.
        assert_eq!(Color::Scheme((n - 1) as u8).to_rgb(Some(scheme)), [r, g, b]);
    }
}

#[kernpage_samples::test]
fn scheme_shapes_resolve_to_manifest_colors() {
    let m = manifest();
    let doc = load("schemes");
    let scheme = scheme_of(&doc);
    // Every rectangle's fill, once resolved, must be one of the manifest slot colours.
    let wanted: Vec<[u8; 3]> = m["schemes"]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| match bgr(s["rgb"].as_i64().unwrap()) {
            Color::Rgb(r, g, b) => [r, g, b],
            Color::Scheme(_) => unreachable!(),
        })
        .collect();
    let mut seen = 0;
    for fill in all_shapes(&doc).filter_map(|s| s.fill) {
        assert!(wanted.contains(&fill.to_rgb(Some(scheme))), "{fill:?}");
        seen += 1;
    }
    assert!(seen >= 8, "expected the 8 slot rectangles, saw {seen}");
}

#[kernpage_samples::test]
fn colors_sample_has_office_or_named_scheme() {
    let doc = load("colors");
    let scheme = scheme_of(&doc);
    assert!(!scheme.name.is_empty());
    assert_eq!(scheme.colors[0], [0, 0, 0]);
    assert_eq!(scheme.colors[7], [255, 255, 255]);
}

#[kernpage_samples::test]
fn missing_scheme_falls_back_to_guess() {
    assert_eq!(Color::Scheme(7).to_rgb(None), [255, 255, 255]);
    assert_eq!(Color::Scheme(3).to_rgb(None), [0, 0, 0]);
    assert_eq!(Color::Rgb(1, 2, 3).to_rgb(None), [1, 2, 3]);
}

#[kernpage_samples::test]
fn linked_sample_chains_match_manifest() {
    let doc = load("linked");
    let m = &manifest()["linked"];
    let frames = m["frames"].as_array().unwrap();
    let chains = doc.chains();
    assert_eq!(chains.len(), 2, "one 3-frame chain plus the standalone box");
    let long = chains.iter().find(|c| c.len() == 3).expect("3-frame chain");
    for (r, want) in long.iter().zip(frames) {
        let s = &doc.pages[r.page].shapes[r.shape];
        assert!(geometry_matches(&doc, s, want), "frame {r:?}");
        assert_eq!(s.chain_story, Some(2));
    }
    let positions: Vec<u32> = long
        .iter()
        .map(|r| doc.pages[r.page].shapes[r.shape].chain_pos)
        .collect();
    assert_eq!(positions, [0, 1, 2]);
    let alone = chains.iter().find(|c| c.len() == 1).expect("standalone");
    let s = &doc.pages[alone[0].page].shapes[alone[0].shape];
    assert!(geometry_matches(&doc, s, &m["standalone"]));
    assert_ne!(s.chain_story, Some(2));
    assert_eq!(s.chain_pos, 0);
}

#[kernpage_samples::test]
fn spacing_sample_matches_manifest() {
    use pub_format::LineSpacing;
    let doc = load("spacing");
    let want = manifest()["spacing"]["paragraphs"]
        .as_array()
        .unwrap()
        .clone();
    let story = &all_shapes(&doc)
        .find(|s| !s.paragraphs.is_empty())
        .expect("text box")
        .paragraphs;
    assert_eq!(story.len(), want.len());
    for (i, (p, w)) in story.iter().zip(&want).enumerate() {
        let sp = p.spacing;
        assert!(
            (f64::from(sp.space_before_pt) - num(w, "space_before")).abs() < 0.01,
            "{i}"
        );
        assert!(
            (f64::from(sp.space_after_pt) - num(w, "space_after")).abs() < 0.01,
            "{i}"
        );
        let amount = num(w, "line_spacing");
        match w["line_spacing_rule"].as_i64().unwrap() {
            // 5 = multiple of lines, 2 = double (2 lines), 4 = exactly (points).
            5 | 2 => assert!(
                matches!(sp.line, LineSpacing::Multiple(l) if (f64::from(l) - amount).abs() < 0.001),
                "{i}: {:?}",
                sp.line
            ),
            4 => assert!(
                matches!(sp.line, LineSpacing::Exactly(pt) if (f64::from(pt) - amount).abs() < 0.01),
                "{i}: {:?}",
                sp.line
            ),
            r => panic!("unexpected rule {r}"),
        }
    }
}

#[kernpage_samples::test]
fn rotate_sample_matches_manifest() {
    let doc = load("rotate");
    let want = manifest()["rotate"]["shapes"].as_array().unwrap().clone();
    assert_eq!(want.len(), 4);
    for w in &want {
        let angle = num(w, "rotation") as f32;
        let s = all_shapes(&doc)
            .find(|s| {
                (s.rotation_deg - angle).abs() < 0.01 && {
                    let (l, t, _, _) = shape_pt(&doc, s);
                    (l - num(w, "left_pt")).abs() <= TOLERANCE_PT
                        && (t - num(w, "top_pt")).abs() <= TOLERANCE_PT
                }
            })
            .unwrap_or_else(|| panic!("no shape rotated {angle} at the manifest position"));
        assert!(geometry_matches(&doc, s, w), "unrotated frame of {angle}");
    }
}

#[kernpage_samples::test]
fn columns_sample_matches_manifest() {
    let doc = load("columns");
    for w in manifest()["columns"]["boxes"].as_array().unwrap() {
        let s = all_shapes(&doc)
            .find(|s| geometry_matches(&doc, s, w))
            .unwrap_or_else(|| panic!("no box at {w}"));
        assert_eq!(f64::from(s.columns), num(w, "columns"), "{w}");
        let gap_pt = f64::from(s.column_gap_emu) / EMU_PER_POINT;
        assert!(
            (gap_pt - num(w, "column_spacing_pt")).abs() <= 0.1,
            "{gap_pt} {w}"
        );
    }
}

#[kernpage_samples::test]
fn linked_sample_frames_have_one_column() {
    let doc = load("linked");
    assert_eq!(doc.columns_unmapped, 0);
    assert_eq!(doc.columns_mapped, 2);
    assert!(all_shapes(&doc).all(|s| s.columns == 1));
}

#[kernpage_samples::test]
fn table2_sizes_and_cells_match_manifest() {
    let doc = load("table2");
    let m = &manifest()["table2"];
    let s = all_shapes(&doc).next().unwrap();
    let t = s.table.as_ref().expect("a table");
    let pts = |v: &[u32]| {
        v.iter()
            .map(|&e| f64::from(e) / EMU_PER_POINT)
            .collect::<Vec<_>>()
    };
    let want = |k: &str| {
        m[k].as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    for (got, exp) in [
        (pts(&t.column_widths_emu), want("column_widths_pt")),
        (pts(&t.row_heights_emu), want("row_heights_pt")),
    ] {
        assert_eq!(got.len(), exp.len());
        assert!(got
            .iter()
            .zip(&exp)
            .all(|(a, b)| (a - b).abs() <= TOLERANCE_PT));
    }
    assert_eq!(t.cells.len(), 9);
    for (k, v) in m["cells"].as_object().unwrap() {
        let r: usize = k[1..2].parse().unwrap();
        let c: usize = k[3..4].parse().unwrap();
        assert_eq!(t.cells[(r - 1) * 3 + c - 1], v.as_str().unwrap(), "{k}");
    }
    let counts: Vec<usize> = t.cell_paragraphs.iter().map(Vec::len).collect();
    assert_eq!(counts, vec![1, 1, 1, 1, 3, 1, 2, 1, 1]);
    assert_eq!(t.cell_props.len(), 9);
    assert!(t.cell_props.iter().all(|p| p.margin_left_emu == 36_576
        && p.first_row == p.last_row
        && p.first_column == p.last_column));
}

#[kernpage_samples::test]
fn table_without_grid_keeps_single_paragraph_cells() {
    let doc = load("table");
    let t = all_shapes(&doc).next().unwrap().table.as_ref().unwrap();
    assert_eq!(t.cell_paragraphs.len(), 4);
    assert!(t.cell_paragraphs.iter().all(|c| c.len() == 1));
}

#[kernpage_samples::test]
fn group_children_map_onto_the_page() {
    let doc = load("group");
    let m = manifest();
    let g = &m["group"];
    let group = all_shapes(&doc)
        .find(|s| s.kind == 48)
        .expect("the kind-48 group shape");
    assert!(geometry_matches(&doc, group, &g["group"]));
    assert_eq!(group.group, None);
    for child in g["group"]["children"].as_array().unwrap() {
        let s = all_shapes(&doc)
            .find(|s| geometry_matches(&doc, s, child))
            .unwrap_or_else(|| panic!("no shape at the geometry of {}", child["name"]));
        assert_eq!(s.group, Some(group.object_number), "{}", child["name"]);
    }
    let standalone = all_shapes(&doc)
        .find(|s| geometry_matches(&doc, s, &g["standalone"]))
        .expect("standalone box");
    assert_eq!(standalone.group, None);
    assert_eq!(doc.shapes_without_anchor, 0);
}
