//! Duplicate a page (spec 4P.5): a new page right after the source plus a clone of every
//! child (text boxes, AutoShapes, lines, pictures). Every cloned text box gets a new Quill
//! story holding a copy of the source story with its full formatting.

use super::group::{
    drawing_counters, group_counters, install, items_of, put, rd, Body, Cts, EscherDoc, Rec,
};
use super::pages::{add_page, adjust_crefs, picture_indexes};
use super::quill;
use super::textbox::append_record;
use super::{install_quill, Result, WriteError};
use pub_format::Streams;
use std::collections::HashMap;

const KIND_SHAPE: u64 = 1;
const KIND_PICTURE_FILE: u64 = 102;
const KIND_STORY_MAP: u64 = 97;
const KIND_STORY_INFO: u64 = 101;
const ITEM_STORY: u16 = 39;
const ITEM_PICTURE_REF: u16 = 58;
/// Overlap bookkeeping (spec 4L.2) is page specific; Publisher drops it on clones.
const DROPPED: [u16; 3] = [45, 70, 71];

fn unsupported<T>(m: impl Into<String>) -> Result<T> {
    Err(WriteError::Unsupported(m.into()))
}

struct Child {
    obj: u32,
    story: Option<u32>,
    /// Contents children of the shape (a picture's kind-102 file name).
    subs: Vec<u32>,
}

/// Duplicate document page `index` (0-based); the copy becomes page `index + 1`.
/// Groups, tables and linked (chained) text boxes are refused with
/// [`WriteError::Unsupported`]; nothing is half-copied.
pub fn duplicate_page(input: &[u8], index: usize) -> Result<Vec<u8>> {
    let doc = pub_format::read_bytes(input)?;
    let page = doc.pages.get(index).ok_or(WriteError::NoSuchPage(index))?;
    let streams = Streams::from_cfb(input)?;
    let cts0 = Cts::parse(&streams.contents)?;
    let kid_ids = cts0.child_ids(page.object_number as usize)?;
    let numbers = doc.story_numbers();

    // ---- Validate and collect the children (z-order).
    let mut kids: Vec<Child> = Vec::new();
    for &k in &kid_ids {
        let kind = cts0.kind(k as usize);
        if kind != Some(KIND_SHAPE) {
            return unsupported(format!(
                "the page holds a shape of kind {} (groups and tables are not supported)",
                kind.map_or_else(|| "?".to_string(), |v| v.to_string())
            ));
        }
        let Some(shape) = page.shapes.iter().find(|s| s.object_number == k) else {
            return unsupported(format!("shape {k} is not readable"));
        };
        if shape.group.is_some() || shape.table.is_some() {
            return unsupported("the page holds a group or a table");
        }
        let story = match (shape.chain_story, shape.story) {
            (Some(a), Some(b)) if a != b => return unsupported("inconsistent story numbers"),
            (a, b) => a.or(b),
        };
        if let Some(sn) = story {
            let users = doc
                .pages
                .iter()
                .flat_map(|p| p.shapes.iter())
                .filter(|s| s.chain_story.or(s.story) == Some(sn))
                .count();
            if users > 1 || shape.chain_pos != 0 {
                return unsupported("the page holds linked (chained) text boxes");
            }
        }
        let subs: Vec<u32> = (0..cts0.entries.len() as u32)
            .filter(|&i| cts0.kind(i as usize).is_some() && cts0.parent(i as usize).ok() == Some(k as usize))
            .collect();
        if let Some(&c) = subs
            .iter()
            .find(|&&c| cts0.kind(c as usize) != Some(KIND_PICTURE_FILE))
        {
            return unsupported(format!("shape {k} has an unsupported child object {c}"));
        }
        kids.push(Child {
            obj: k,
            story,
            subs,
        });
    }

    // ---- Quill: copy the stories of the text-bearing children.
    // Publisher processes the shapes last-to-first: the last text box gets the lowest new
    // story number and its text comes first (spec 4P.5).
    let mut src_idx = Vec::new();
    let mut story_order: Vec<u32> = Vec::new();
    for c in kids.iter().rev() {
        if let Some(sn) = c.story {
            let Some(i) = numbers.iter().position(|&n| n == sn) else {
                return unsupported(format!("story {sn} is not in the Quill stream"));
            };
            src_idx.push(i);
            story_order.push(sn);
        }
    }
    let new_quill = if src_idx.is_empty() {
        None
    } else {
        Some(quill::duplicate_stories(&streams.quill, &src_idx)?)
    };
    let mut new_numbers = new_quill
        .as_ref()
        .map(|q| q.1.clone())
        .unwrap_or_default()
        .into_iter();
    let story_map: HashMap<u32, u32> = story_order.into_iter().zip(new_numbers.by_ref()).collect();

    // ---- The new page (blank), then its children.
    let paged = add_page(input, Some(index))?;
    let new_page = pub_format::read_bytes(&paged)?
        .pages
        .get(index + 1)
        .map(|p| p.object_number)
        .ok_or_else(|| WriteError::Pages("the new page was not found".into()))?;
    let s1 = Streams::from_cfb(&paged)?;
    let mut cts = Cts::parse(&s1.contents)?;
    let mut esc = EscherDoc::parse(&s1.escher)?;
    let (font0, _) = cts.tail()?;
    let reg_parent = cts
        .slot_of_kind(91)
        .ok_or_else(|| WriteError::Pages("the story root object (kind 91) is missing".into()))?;
    let slot_of = |cts: &Cts, kind: u64| {
        (0..cts.entries.len())
            .find(|&i| cts.kind(i) == Some(kind) && cts.parent(i).ok() == Some(reg_parent))
    };
    let regs = (slot_of(&cts, KIND_STORY_MAP), slot_of(&cts, KIND_STORY_INFO));
    if !story_map.is_empty() && (regs.0.is_none() || regs.1.is_none()) {
        return Err(WriteError::Pages(
            "the story registries (kinds 97 and 101) are missing".into(),
        ));
    }

    // New object ids: each child, then its kind-102 children, from the font slot up.
    let mut next = font0 as u32;
    let mut plan: Vec<(u32, u32, Vec<(u32, u32)>)> = Vec::new();
    for c in &kids {
        let id = next;
        next += 1;
        let subs: Vec<(u32, u32)> = c
            .subs
            .iter()
            .map(|&s| {
                next += 1;
                (s, next - 1)
            })
            .collect();
        plan.push((c.obj, id, subs));
    }

    let mut pibs: Vec<u32> = Vec::new();
    for (c, (old, new_id, subs)) in kids.iter().zip(&plan) {
        // Contents body: story renumbered, picture ref repointed, overlap items dropped.
        let off = cts.offset(*old as usize)?;
        let mut body = vec![0u8; 4];
        for (id, h, p, e) in items_of(&cts.blocks, off)? {
            if DROPPED.contains(&id) {
                continue;
            }
            if id == ITEM_STORY {
                let Some(&ns) = c.story.and_then(|s| story_map.get(&s)) else {
                    return unsupported("a story item without a story");
                };
                body.extend_from_slice(&cts.blocks[h..p]);
                body.extend_from_slice(&ns.to_le_bytes());
            } else if id == ITEM_PICTURE_REF {
                let Some(&(_, nc)) = subs.first() else {
                    return unsupported("a picture reference without a child object");
                };
                body.extend_from_slice(&cts.blocks[h..p]);
                body.extend_from_slice(&nc.to_le_bytes());
            } else if cts.blocks[h + 1] >> 3 == 14 {
                return unsupported(format!("shape {old} has an unknown reference (item {id})"));
            } else {
                body.extend_from_slice(&cts.blocks[h..e]);
            }
        }
        let n = body.len() as u32;
        body[..4].copy_from_slice(&n.to_le_bytes());
        let mut inserts = vec![(*old as usize, *new_id, body, new_page)];
        for &(s, ns) in subs {
            let so = cts.offset(s as usize)?;
            let sl = rd(&cts.blocks, so)? as usize;
            inserts.push((s as usize, ns, cts.blocks[so..so + sl].to_vec(), *new_id));
        }
        for (src, id, body, parent) in inserts {
            let (font, at) = cts.tail()?;
            if font as u32 != id {
                return Err(WriteError::Pages("object id allocation drifted".into()));
            }
            cts.blocks.splice(at..at, body.iter().copied());
            cts.shift(at, true, body.len() as isize)?;
            let entry = cts.entries[src].clone();
            cts.insert_entries(font, vec![entry])?;
            cts.set_int(font, 4, at)?;
            cts.set_int(font, 5, parent as usize)?;
        }

        // Escher: clone the container with the new spid, object and story.
        let mut hit: Option<(usize, Rec)> = None;
        for (di, (_, d)) in esc.drawings.iter().enumerate() {
            if let Some(g) = d.kids().iter().find(|r| r.ty == 0xF003) {
                if let Some(k) = g
                    .kids()
                    .iter()
                    .find(|k| k.ty == 0xF004 && k.object() == Some(*old))
                {
                    hit = Some((di, k.clone()));
                }
            }
        }
        let Some((di, mut cont)) = hit else {
            return unsupported(format!("shape {old} has no Escher container"));
        };
        let drawing = &mut esc.drawings[di].1;
        let dgid = drawing.find(0xF008).map_or(0, |r| r.vi >> 4);
        let spid = match drawing.find(0xF008).map(|r| &r.body) {
            Some(Body::Atom(d)) => rd(d, 4)? + 1,
            _ => return Err(WriteError::Pages("the drawing has no 0xF008".into())),
        };
        let story = c.story.and_then(|s| story_map.get(&s)).copied();
        for k in cont.kids_mut()? {
            match k.ty {
                0xF00A => put(k.data_mut()?, 0, spid)?,
                0xF00B => {
                    if let Some(st) = story {
                        let cnt = usize::from(k.vi >> 4);
                        let d = k.data_mut()?;
                        if let Some(at) = (0..cnt).find(|i| {
                            d.get(6 * i..6 * i + 2)
                                .is_some_and(|b| b[0] == 0x80 && b[1] & 0x3F == 0)
                        }) {
                            put(d, 6 * at + 2, st)?;
                        }
                    }
                }
                0xF011 => {
                    let d = k.data_mut()?;
                    if d.len() != 10 {
                        return unsupported("unexpected client data in an Escher container");
                    }
                    put(d, 6, *new_id)?;
                }
                0xF00D => {
                    if let Some(st) = story {
                        let d = k.data_mut()?;
                        if d.len() != 10 {
                            return unsupported("unexpected text client data");
                        }
                        put(d, 6, st)?;
                    }
                }
                _ => {}
            }
        }
        picture_indexes(&cont, &mut pibs);
        drawing_counters(drawing, 1, Some(spid))?;
        drawing
            .kids_mut()?
            .iter_mut()
            .find(|r| r.ty == 0xF003)
            .ok_or_else(|| WriteError::Pages("the drawing has no page group".into()))?
            .kids_mut()?
            .push(cont);
        group_counters(&mut esc.head, dgid, 1, 1)?;
    }
    if !pibs.is_empty() {
        adjust_crefs(&mut esc.head, &mut 0, &pibs, 1);
    }
    let new_ids: Vec<u32> = plan.iter().map(|p| p.1).collect();
    if !new_ids.is_empty() {
        cts.set_child_ids(new_page as usize, &new_ids)?;
    }

    // Registries 283 / 284: one record per cloned story (ascending story number).
    if let (Some(map), Some(info)) = regs {
        let mut recs: Vec<(u32, u32)> = kids
            .iter()
            .zip(&plan)
            .filter_map(|(c, p)| c.story.and_then(|s| story_map.get(&s)).map(|&ns| (ns, p.1)))
            .collect();
        recs.sort_unstable();
        for &(story, obj) in &recs {
            append_record(&mut cts, map, &|_, _| {
                let mut b = vec![0x01, 0x20];
                b.extend_from_slice(&story.to_le_bytes());
                b.extend_from_slice(&[0x03, 0x68]);
                b.extend_from_slice(&obj.to_le_bytes());
                Ok(b)
            })?;
            append_record(&mut cts, info, &|last, ordinal| {
                let its = items_of(last, 2)?;
                let mut b = vec![0x01, 0x20];
                b.extend_from_slice(&story.to_le_bytes());
                b.extend_from_slice(&[0x07, 0x20]);
                b.extend_from_slice(&(ordinal as u32).to_le_bytes());
                for want in [8u16, 9] {
                    let &(_, h, _, e) = its.iter().find(|i| i.0 == want).ok_or_else(|| {
                        WriteError::Pages(format!("registry record lacks item {want}"))
                    })?;
                    b.extend_from_slice(&last[h..e]);
                }
                Ok(b)
            })?;
        }
    }
    let out = install(&paged, &cts.finish()?, &esc.to_bytes())?;
    match new_quill {
        Some((q, _)) => install_quill(&out, &q),
        None => Ok(out),
    }
}
