//! Syntactic parsing of a descriptor byte stream

use crate::{ItemType, LongItem, RawItem, ShortItem};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("short item at offset {offset} requires {required} payload bytes, only {remaining} remain")]
    TruncatedShort {
        offset: usize,
        required: usize,
        remaining: usize,
    },
    
    #[error("long item header is truncated at offset {offset}")]
    TruncatedLongHeader { 
        offset: usize
    },
    
    #[error("long item at offset {offset} declares {declared} bytes, only {remaining} remain")]
    TruncatedLong {
        offset: usize,
        declared: usize,
        remaining: usize,
    },
}

/// Lazily splits descriptor bytes into borrowed raw items
pub struct ItemIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ItemIter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn parse_long(&mut self, start: usize) -> Result<RawItem<'a>, ParseError> {
        if self.remaining() < 2 {
            self.finish();
            return Err(ParseError::TruncatedLongHeader { offset: start });
        }

        // A long item stores its payload length and tag after the 0xFE prefix.
        let size = self.bytes[self.offset] as usize;
        let tag = self.bytes[self.offset + 1];
        
        self.offset += 2;
        
        if self.remaining() < size {
            let remaining = self.remaining();
        
            self.finish();
        
            return Err(ParseError::TruncatedLong {
                offset: start,
                declared: size,
                remaining,
            });
        }

        let data = &self.bytes[self.offset..self.offset + size];
        self.offset += size;
        
        Ok(RawItem::Long(LongItem { tag, data }))
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    // A malformed item consumes the rest so one descriptor yields one error.
    fn finish(&mut self) {
        self.offset = self.bytes.len();
    }
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Result<RawItem<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.bytes.len() {
            return None;
        }

        let start = self.offset;
        let prefix = self.bytes[self.offset];

        self.offset += 1;

        if prefix == 0xFE {
            return Some(self.parse_long(start));
        }

        // In HID's size code, binary 11 means four bytes rather than three.
        let size = [0, 1, 2, 4][usize::from(prefix & 0b11)];

        if self.remaining() < size {
            let remaining = self.remaining();

            self.finish();
            
            return Some(Err(ParseError::TruncatedShort {
                offset: start,
                required: size,
                remaining,
            }));
        }

        let data = &self.bytes[self.offset..self.offset + size];
        
        self.offset += size;
        
        Some(Ok(RawItem::Short(ShortItem {
            prefix,
            item_type: ItemType::from_prefix(prefix),
            tag: prefix >> 4,
            data,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_code_three_means_four_bytes() {
        let bytes = [0x07, 1, 2, 3, 4];
        let RawItem::Short(item) = ItemIter::new(&bytes).next().unwrap().unwrap() else {
            panic!("expected a short item");
        };
        
        assert_eq!(item.data, &[1, 2, 3, 4]);
    }

    #[test]
    fn rejects_truncated_short_item() {
        let error = ItemIter::new(&[0x06, 1]).next().unwrap().unwrap_err();
        
        assert_eq!(
            error,
            ParseError::TruncatedShort {
                offset: 0,
                required: 2,
                remaining: 1
            }
        );
    }

    #[test]
    fn rejects_truncated_long_items() {
        assert_eq!(
            ItemIter::new(&[0xFE]).next().unwrap().unwrap_err(),
            ParseError::TruncatedLongHeader { offset: 0 }
        );
        
        assert_eq!(
            ItemIter::new(&[0xFE, 2, 0x42, 0xAA])
                .next()
                .unwrap()
                .unwrap_err(),
            ParseError::TruncatedLong {
                offset: 0,
                declared: 2,
                remaining: 1
            }
        );
    }

    #[test]
    fn parses_long_item_without_losing_sync() {
        let bytes = [0xFE, 2, 0x42, 0xAA, 0xBB, 0x05, 0x01];
        let mut items = ItemIter::new(&bytes);
        
        assert_eq!(
            items.next().unwrap().unwrap(),
            RawItem::Long(LongItem {
                tag: 0x42,
                data: &[0xAA, 0xBB],
            })
        );
        
        assert!(matches!(items.next().unwrap().unwrap(), RawItem::Short(_)));
        assert!(items.next().is_none());
    }
}
