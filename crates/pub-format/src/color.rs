//! Colour values (spec 3.8, 4D): four bytes R, G, B, flag.

use crate::items::{self, Item, Value};
use crate::quill;
use serde::Serialize;

/// Flag byte value meaning "colour-scheme entry" (spec 4D).
const FLAG_SCHEME: u8 = 0x08;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Color {
    /// A literal colour.
    Rgb(u8, u8, u8),
    /// An index into the document's colour scheme (spec 4F); see [`Color::to_rgb`].
    Scheme(u8),
}

impl Color {
    /// Decode a stored colour value. The little-endian bytes of `value` are R, G, B, flag.
    ///
    /// UNCONFIRMED: any flag other than 0x08 is treated like 0 (literal R, G, B); only
    /// flags 0 and 8 have been observed.
    pub fn from_u32(value: u32) -> Color {
        let [r, g, b, flag] = value.to_le_bytes();
        if flag == FLAG_SCHEME {
            Color::Scheme(r)
        } else {
            Color::Rgb(r, g, b)
        }
    }

    /// Resolve to RGB. A scheme index is looked up in `scheme`; a document without a
    /// kind-92 object (`None`), or an index above 7, falls back to the old guess:
    /// index 7 is white, everything else black.
    pub fn to_rgb(self, scheme: Option<&ColorScheme>) -> [u8; 3] {
        match self {
            Color::Rgb(r, g, b) => [r, g, b],
            Color::Scheme(i) => match scheme.and_then(|s| s.colors.get(usize::from(i))) {
                Some(&rgb) => rgb,
                None if i == 7 => [255, 255, 255],
                None => [0, 0, 0],
            },
        }
    }
}

/// The document's colour scheme: the `Contents` object of kind 92 (spec 4F).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ColorScheme {
    /// Scheme colours in index order (Publisher's slots 1-8 are indices 0-7).
    pub colors: [[u8; 3]; 8],
    /// Scheme name, e.g. "Black & Gray".
    pub name: String,
}

const ITEM_COUNT: u16 = 1;
const ITEM_COLORS: u16 = 2;
const ITEM_NAME: u16 = 6;

impl ColorScheme {
    /// Decode the items of a kind-92 object.
    pub(crate) fn from_items(items: &[Item]) -> ColorScheme {
        // spec §4F: item 1 is the colour count (8); the list is read for up to 8 entries.
        let count = items::find_int(items, ITEM_COUNT).map_or(8, |n| (n as usize).min(8));
        let mut colors = [[0u8; 3]; 8];
        if let Some(Value::List(list)) = items::find(items, ITEM_COLORS).map(|i| &i.value) {
            for (slot, entry) in list.iter().take(count).enumerate() {
                // Element 0 is a bare flag element = black (stays [0, 0, 0]); the others
                // are records whose item 1 is R, G, B, 0.
                if let Value::Record(rec) = &entry.value {
                    if let Some(v) = items::find_int(rec, 1) {
                        let [r, g, b, _] = (v as u32).to_le_bytes();
                        colors[slot] = [r, g, b];
                    }
                }
            }
        }
        let name = match items::find(items, ITEM_NAME).map(|i| &i.value) {
            Some(Value::Raw(raw)) => {
                // Terminated by a UTF-16 zero.
                let even = raw.len() - raw.len() % 2;
                let end = (0..even)
                    .step_by(2)
                    .find(|&i| raw[i] == 0 && raw[i + 1] == 0)
                    .unwrap_or(even);
                quill::utf16le(&raw[..end])
            }
            _ => String::new(),
        };
        ColorScheme { colors, name }
    }
}
