//! Document save counter (spec 4M): item 60 of Contents object 256.

use pub_format::{contents::Contents, items, Streams};
use pub_write::bump_save_counter;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::PathBuf;

fn gen(name: &str) -> Vec<u8> {
    std::fs::read(kernpage_samples::resolve(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../samples/gen/{name}.pub")),))
    .unwrap()
}

/// The counter's value and its byte offset in Contents, if present.
fn counter(bytes: &[u8]) -> Option<(u32, usize)> {
    let s = Streams::from_cfb(bytes).unwrap();
    let c = Contents::parse(&s.contents).unwrap();
    let e = c.get(256)?;
    let slot = items::locate_int(&s.contents, e.offset, 60).unwrap()?;
    let b = &s.contents[slot.offset..slot.offset + 4];
    Some((u32::from_le_bytes([b[0], b[1], b[2], b[3]]), slot.offset))
}

fn set_contents_u32(bytes: &[u8], offset: usize, v: u32) -> Vec<u8> {
    let mut cf = cfb::CompoundFile::open(Cursor::new(bytes.to_vec())).unwrap();
    {
        let mut s = cf.open_stream("/Contents").unwrap();
        s.seek(SeekFrom::Start(offset as u64)).unwrap();
        s.write_all(&v.to_le_bytes()).unwrap();
    }
    cf.flush().unwrap();
    cf.into_inner().into_inner()
}

#[kernpage_samples::test]
fn bulletin_counter_goes_from_two_to_three_and_only_that_byte_changes() {
    let before = gen("bulletin");
    assert_eq!(counter(&before).unwrap().0, 2);
    let after = bump_save_counter(&before).unwrap();
    let (v, off) = counter(&after).unwrap();
    assert_eq!(v, 3);
    let (a, b) = (
        Streams::from_cfb(&before).unwrap(),
        Streams::from_cfb(&after).unwrap(),
    );
    assert_eq!(a.contents.len(), b.contents.len());
    let diff: Vec<usize> = (0..a.contents.len())
        .filter(|&i| a.contents[i] != b.contents[i])
        .collect();
    assert_eq!(diff, vec![off]); // 2 -> 3 changes the low byte only
    assert_eq!(a.escher, b.escher);
    assert_eq!(counter(&bump_save_counter(&after).unwrap()).unwrap().0, 4);
    pub_format::read_bytes(&after).unwrap();
}

#[kernpage_samples::test]
fn counter_saturates_at_u32_max() {
    let bytes = gen("bulletin");
    let (_, off) = counter(&bytes).unwrap();
    let max = set_contents_u32(&bytes, off, u32::MAX);
    assert_eq!(counter(&max).unwrap().0, u32::MAX);
    let out = bump_save_counter(&max).unwrap();
    assert_eq!(counter(&out).unwrap().0, u32::MAX);
}

#[kernpage_samples::test]
fn file_without_the_item_is_returned_unchanged() {
    // Rename item 60 to an id object 256 does not use: the item is then "absent".
    let bytes = gen("bulletin");
    let (_, off) = counter(&bytes).unwrap();
    let mut cf = cfb::CompoundFile::open(Cursor::new(bytes)).unwrap();
    {
        let mut s = cf.open_stream("/Contents").unwrap();
        s.seek(SeekFrom::Start(off as u64 - 2)).unwrap();
        s.write_all(&[0xEE]).unwrap();
    }
    cf.flush().unwrap();
    let missing = cf.into_inner().into_inner();
    assert!(counter(&missing).is_none());
    assert_eq!(bump_save_counter(&missing).unwrap(), missing);
}
