//! System font lookup and TrueType embedding support.
//!
//! [`FontDb`] scans font directories by reading each `.ttf`/`.ttc`/`.otf` file's `name`
//! and `OS/2` tables (never by file name). [`EmbFont`] holds one chosen face while a
//! document is drawn: it hands out subset glyph ids (CIDs) and real advance widths, and
//! at the end yields the subsetted font program.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use subsetter::GlyphRemapper;
use ttf_parser::{name_id, GlyphId, Permissions};

/// Upper bound on a `name` or `OS/2` table we are willing to read while scanning.
const MAX_TABLE_LEN: u32 = 4 << 20;
/// Upper bound on faces per collection and tables per face (corrupt-file guards).
const MAX_FACES: u32 = 256;
const MAX_TABLES: usize = 512;

/// One face found on disk.
#[derive(Debug, Clone)]
pub struct FontFace {
    pub path: PathBuf,
    /// Face index inside a `.ttc` (0 for plain files).
    pub index: u32,
    /// Family name (name id 1) as written in the font, for menus.
    pub display_family: String,
    /// Lower-cased family names from name id 1 (the legacy, style-linked family).
    families: Vec<String>,
    /// Lower-cased family names from name id 16 (typographic family).
    typographic: Vec<String>,
    pub bold: bool,
    pub italic: bool,
    /// Weight class (400 regular, 700 bold, ...).
    weight: u16,
    /// `true` when the face has TrueType outlines and OS/2 `fsType` allows embedding.
    pub embeddable: bool,
}

/// All faces found in a list of directories.
#[derive(Debug, Clone, Default)]
pub struct FontDb {
    faces: Vec<FontFace>,
}

impl FontDb {
    /// Scan `dirs` (not recursively). Unreadable files and directories are skipped.
    pub fn scan(dirs: &[PathBuf]) -> FontDb {
        let mut faces = Vec::new();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        matches!(e.to_ascii_lowercase().as_str(), "ttf" | "ttc" | "otf")
                    })
                })
                .collect();
            files.sort();
            for file in files {
                faces.extend(read_faces(&file));
            }
        }
        FontDb { faces }
    }

    /// The best face for `family` with the requested style. The family is matched
    /// case-insensitively against name id 1 first (the regular/bold/italic/bold-italic
    /// group), then against the typographic family (id 16) restricted to regular and bold
    /// weights. Among matches a bold match counts more than an italic match; the first
    /// face in directory order wins ties.
    pub fn find(&self, family: &str, bold: bool, italic: bool) -> Option<&FontFace> {
        let want = family.trim().to_lowercase();
        if want.is_empty() {
            return None;
        }
        let first = self.faces.iter().filter(|f| f.families.contains(&want));
        best_match(first, bold, italic).or_else(|| {
            let second = self
                .faces
                .iter()
                .filter(|f| f.typographic.contains(&want) && matches!(f.weight, 400 | 700));
            best_match(second, bold, italic)
        })
    }

    /// Sorted, de-duplicated family names (as written in the fonts) for font menus.
    pub fn family_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .faces
            .iter()
            .map(|f| f.display_family.clone())
            .filter(|n| !n.trim().is_empty() && !n.starts_with('@'))
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by_key(|n| n.to_lowercase());
        names
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

/// The face matching the requested style best; the first one wins ties.
fn best_match<'a>(
    faces: impl Iterator<Item = &'a FontFace>,
    bold: bool,
    italic: bool,
) -> Option<&'a FontFace> {
    let score = |f: &FontFace| 2 * u8::from(f.bold == bold) + u8::from(f.italic == italic);
    let mut best: Option<&FontFace> = None;
    for f in faces {
        if best.is_none_or(|b| score(f) > score(b)) {
            best = Some(f);
        }
    }
    best
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    let s = b.get(o..o.checked_add(2)?)?;
    Some(u16::from_be_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    let s = b.get(o..o.checked_add(4)?)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn read_at(f: &mut File, offset: u64, len: usize) -> Option<Vec<u8>> {
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// Read the face descriptions of one font file, touching only its table directories and
/// its `name` and `OS/2` tables.
fn read_faces(path: &Path) -> Vec<FontFace> {
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let Some(head) = read_at(&mut f, 0, 12) else {
        return Vec::new();
    };
    let offsets: Vec<u32> = if &head[..4] == b"ttcf" {
        let n = u32_at(&head, 8).unwrap_or(0).min(MAX_FACES) as usize;
        let Some(list) = read_at(&mut f, 12, 4 * n) else {
            return Vec::new();
        };
        (0..n).filter_map(|i| u32_at(&list, 4 * i)).collect()
    } else {
        vec![0]
    };
    offsets
        .iter()
        .enumerate()
        .filter_map(|(i, &off)| read_face(&mut f, path, i as u32, u64::from(off)))
        .collect()
}

fn read_face(f: &mut File, path: &Path, index: u32, off: u64) -> Option<FontFace> {
    let head = read_at(f, off, 12)?;
    let n = usize::from(u16_at(&head, 4)?).min(MAX_TABLES);
    let dir = read_at(f, off + 12, 16 * n)?;
    let mut name_tab = None;
    let mut os2_tab = None;
    let mut has_glyf = false;
    for e in dir.as_chunks::<16>().0 {
        let (offset, len) = (u32_at(e, 8)?, u32_at(e, 12)?);
        match &e[..4] {
            b"name" => name_tab = Some((offset, len)),
            b"OS/2" => os2_tab = Some((offset, len)),
            b"glyf" => has_glyf = true,
            _ => {}
        }
    }
    let table = |f: &mut File, t: Option<(u32, u32)>| {
        let (o, l) = t?;
        (l <= MAX_TABLE_LEN)
            .then(|| read_at(f, u64::from(o), l as usize))
            .flatten()
    };
    let name_bytes = table(f, name_tab)?;
    let os2_bytes = table(f, os2_tab);
    let names = ttf_parser::name::Table::parse(&name_bytes)?.names;
    let (mut families, mut typographic) = (Vec::new(), Vec::new());
    let mut display_family = String::new();
    for i in 0..names.len() {
        let Some(n) = names.get(i) else { continue };
        let Some(s) = n.to_string() else { continue };
        if n.name_id == name_id::FAMILY && display_family.is_empty() {
            display_family = s.clone();
        }
        let s = s.to_lowercase();
        // Any language: localized names are extra aliases.
        match n.name_id {
            name_id::FAMILY => families.push(s),
            name_id::TYPOGRAPHIC_FAMILY => typographic.push(s),
            _ => {}
        }
    }
    if families.is_empty() && typographic.is_empty() {
        return None;
    }
    let os2 = os2_bytes.as_deref().and_then(ttf_parser::os2::Table::parse);
    let (bold, italic, weight) = match &os2 {
        Some(t) => (
            t.is_bold(),
            matches!(
                t.style(),
                ttf_parser::Style::Italic | ttf_parser::Style::Oblique
            ),
            t.weight().to_number(),
        ),
        None => (false, false, 400),
    };
    // UNCONFIRMED: a font with no OS/2 table has no embedding restriction.
    let allowed = os2.as_ref().is_none_or(|t| {
        matches!(
            t.permissions(),
            Some(Permissions::Installable | Permissions::Editable | Permissions::PreviewAndPrint)
        ) && t.is_subsetting_allowed()
            && t.is_outline_embedding_allowed()
    });
    Some(FontFace {
        path: path.to_path_buf(),
        index,
        display_family,
        families,
        typographic,
        bold,
        italic,
        weight,
        embeddable: allowed && has_glyf,
    })
}

/// A glyph looked up in an embedded font.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// Glyph id in the subset font, used as the CID in the content stream.
    pub cid: u16,
    /// Advance in em units (1.0 = the font size).
    pub advance_em: f32,
}

/// A TrueType face being embedded as a subset.
pub struct EmbFont {
    data: Vec<u8>,
    index: u32,
    upem: f32,
    remapper: GlyphRemapper,
    glyphs: HashMap<char, GlyphInfo>,
    /// Used CIDs with the character they were first looked up for.
    used: BTreeMap<u16, (char, f32)>,
    pub ps_name: String,
    pub bold: bool,
    pub italic: bool,
    pub ascent: f32,
    pub descent: f32,
    /// Natural line height (ascent + descent + line gap, hhea) in 1/1000 em.
    pub line_height: f32,
    pub cap_height: f32,
    pub italic_angle: f32,
    pub bbox: [f32; 4],
    pub fixed_pitch: bool,
}

/// Everything needed to write the font objects.
pub struct FontProgram {
    /// The subsetted font file, or the full font if subsetting failed.
    pub bytes: Vec<u8>,
    /// `(cid, advance in 1/1000 em)` ascending.
    pub widths: Vec<(u16, f32)>,
    /// `(cid, character)` for the ToUnicode map.
    pub unicode: Vec<(u16, char)>,
    /// Big-endian CID -> GID table, only when subsetting failed and the whole font is
    /// embedded (the CIDs are then not the font's own glyph ids).
    pub cid_to_gid: Option<Vec<u8>>,
}

impl EmbFont {
    /// Read and parse the face. `None` when the file cannot be read or parsed.
    pub fn load(face: &FontFace) -> Option<EmbFont> {
        let data = std::fs::read(&face.path).ok()?;
        let f = ttf_parser::Face::parse(&data, face.index).ok()?;
        let upem = f32::from(f.units_per_em()).max(1.0);
        let scale = 1000.0 / upem;
        let bb = f.global_bounding_box();
        let ps_name = f
            .names()
            .into_iter()
            .find(|n| n.name_id == name_id::POST_SCRIPT_NAME && n.is_unicode())
            .and_then(|n| n.to_string())
            .unwrap_or_default();
        let ps_name: String = ps_name.chars().filter(|c| c.is_ascii_graphic()).collect();
        let ps_name = if ps_name.is_empty() {
            "Font".to_string()
        } else {
            ps_name
        };
        let out = EmbFont {
            index: face.index,
            upem,
            remapper: GlyphRemapper::new(),
            glyphs: HashMap::new(),
            used: BTreeMap::new(),
            ps_name,
            bold: face.bold,
            italic: face.italic,
            ascent: f32::from(f.ascender()) * scale,
            descent: f32::from(f.descender()) * scale,
            line_height: f32::from(f.ascender() - f.descender() + f.line_gap()) * scale,
            cap_height: f
                .capital_height()
                .map_or(f32::from(f.ascender()), f32::from)
                * scale,
            italic_angle: f.italic_angle(),
            bbox: [
                f32::from(bb.x_min) * scale,
                f32::from(bb.y_min) * scale,
                f32::from(bb.x_max) * scale,
                f32::from(bb.y_max) * scale,
            ],
            fixed_pitch: f.is_monospaced(),
            data: Vec::new(),
        };
        Some(EmbFont { data, ..out })
    }

    /// The glyph for `ch` (the font's `.notdef` when it has none). New glyphs are added
    /// to the subset.
    pub fn glyph(&mut self, ch: char) -> GlyphInfo {
        if let Some(g) = self.glyphs.get(&ch) {
            return *g;
        }
        let (gid, advance_em) = match ttf_parser::Face::parse(&self.data, self.index) {
            Ok(f) => {
                let gid = f.glyph_index(ch).map_or(0, |g| g.0);
                let adv = f.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);
                (gid, f32::from(adv) / self.upem)
            }
            Err(_) => (0, 0.5),
        };
        let cid = self.remapper.remap(gid);
        let info = GlyphInfo { cid, advance_em };
        self.glyphs.insert(ch, info);
        self.used.entry(cid).or_insert((ch, advance_em * 1000.0));
        info
    }

    /// Subset the font to the glyphs handed out so far.
    pub fn program(&self) -> FontProgram {
        let (bytes, cid_to_gid) = match subsetter::subset(&self.data, self.index, &self.remapper) {
            Ok(b) => (b, None),
            Err(_) => (
                self.data.clone(),
                Some(
                    self.remapper
                        .remapped_gids()
                        .flat_map(u16::to_be_bytes)
                        .collect(),
                ),
            ),
        };
        FontProgram {
            bytes,
            cid_to_gid,
            widths: self.used.iter().map(|(&c, &(_, w))| (c, w)).collect(),
            unicode: self.used.iter().map(|(&c, &(ch, _))| (c, ch)).collect(),
        }
    }
}
