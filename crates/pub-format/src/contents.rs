//! The `Contents` stream: header (spec 4.1) and object index (spec 4.4).

use crate::error::PubError;
use crate::items::{self, Item, Value};
use std::collections::HashMap;

const SIGNATURE: u16 = 0xACE8;
const HEADER_LEN: usize = 0x2C;
const SUPPORTED_GENERATION: u16 = 1000;
const INDEX_OFFSET_FIELD: usize = 0x1A;

/// One slot of the object index.
#[derive(Debug, Clone)]
pub struct ObjectEntry {
    /// The slot position in the index list.
    pub number: u32,
    pub kind: u16,
    pub parent: Option<u32>,
    /// Offset of the object's data block in the stream.
    pub offset: usize,
}

pub struct Contents<'a> {
    data: &'a [u8],
    /// Objects in index order.
    pub objects: Vec<ObjectEntry>,
    by_number: HashMap<u32, usize>,
}

impl<'a> Contents<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, PubError> {
        if data.len() < HEADER_LEN {
            return Err(PubError::format(
                0,
                "Contents stream shorter than its header",
            ));
        }
        let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]);
        if u16_at(0) != SIGNATURE {
            return Err(PubError::format(
                0,
                "bad Contents signature (expected 0xACE8)",
            ));
        }
        let generation = u16_at(4);
        if generation != SUPPORTED_GENERATION {
            return Err(PubError::UnsupportedGeneration(generation));
        }
        let index_offset = u32::from_le_bytes([
            data[INDEX_OFFSET_FIELD],
            data[INDEX_OFFSET_FIELD + 1],
            data[INDEX_OFFSET_FIELD + 2],
            data[INDEX_OFFSET_FIELD + 3],
        ]) as usize;
        let index = items::decode_block(data, index_offset)?;
        let list = match items::find(&index, 3).map(|i| &i.value) {
            Some(Value::List(l)) => l,
            _ => {
                return Err(PubError::format(
                    index_offset,
                    "object index has no item 3 list",
                ))
            }
        };
        let mut objects = Vec::new();
        let mut by_number = HashMap::new();
        for (slot, entry) in list.iter().enumerate() {
            let Value::Record(rec) = &entry.value else {
                continue; // empty slot
            };
            let kind = items::find_int(rec, 2)
                .ok_or_else(|| PubError::format(index_offset, format!("object {slot}: no kind")))?;
            let offset = items::find_int(rec, 4).ok_or_else(|| {
                PubError::format(index_offset, format!("object {slot}: no offset"))
            })?;
            by_number.insert(slot as u32, objects.len());
            objects.push(ObjectEntry {
                number: slot as u32,
                kind: kind as u16,
                parent: items::find_int(rec, 5).map(|p| p as u32),
                offset: offset as usize,
            });
        }
        Ok(Contents {
            data,
            objects,
            by_number,
        })
    }

    pub fn get(&self, number: u32) -> Option<&ObjectEntry> {
        self.by_number.get(&number).map(|&i| &self.objects[i])
    }

    /// Decode the data block of one object.
    pub fn decode(&self, entry: &ObjectEntry) -> Result<Vec<Item>, PubError> {
        items::decode_block(self.data, entry.offset)
    }
}
