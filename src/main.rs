// Global items persist until replaced or restored through push/pop.
#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    pub usage_page: Option<u32>,
    pub logical_minimum: Option<i32>,
    pub logical_maximum: Option<i32>,
    pub physical_minimum: Option<i32>,
    pub physical_maximum: Option<i32>,
    pub report_size: Option<u32>,
    pub report_count: Option<u32>,
    pub report_id: Option<u8>,
}

// Local usages accumulate until a main item consumes them
// After an Input, Output, Feature, Collection, or other relevant Main item, local state is cleared
#[derive(Debug, Clone, Default)]
pub struct LocalState {
    pub usages: Vec<u32>,
    pub usage_minimum: Option<u32>,
    pub usage_maximum: Option<u32>,
}

// Populate with the collection kinds
pub enum CollectionKind {}

// Collections group related controls and commmunicate semantic scope.
// The NSP2 controller contains nested collections
pub struct Collection {
    pub kind: CollectionKind,
    pub usage_page: Option<u32>,
    pub usage: Option<u32>,
}

// Controller descryptor item types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Main,
    Global,
    Local,
    Reserved,
}

impl ItemType {
    fn from_prefix(prefix: u8) -> Self {
        match (prefix >> 2) & 0b11 {
            0 => Self::Main,     // 00
            1 => Self::Global,   // 01
            2 => Self::Local,    // 10
            3 => Self::Reserved, // 11
            _ => unreachable!(),
        }
    }
}

// Short item counts with prefix, type, tag and data
// 0000(tag)x00(type)00(data size)
#[derive(Debug, Clone, Copy)]
pub struct ShortItem<'a> {
    pub prefix: u8,
    pub item_type: ItemType,
    pub tag: u8,
    pub data: &'a [u8],
}

// Uncommon long items handler
// Prevents mistaking long items with short items
#[derive(Debug, Clone, Copy)]
pub struct LongItem<'a> {
    pub tag: u8,
    pub data: &'a [u8],
}

// Raw item can be either long or short
#[derive(Debug, Clone, Copy)]
pub enum RawItem<'a> {
    Short(ShortItem<'a>),
    Long(LongItem<'a>),
}

// Multi-byte item values are stored least-significant byte first
fn unsigned_value(data: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];

    bytes[..data.len()].copy_from_slice(data);
    u32::from_le_bytes(bytes)
}

// Logical and physical minima can be negative.
// The payload width determines sign extension.
fn signed_value(data: &[u8]) -> i32 {
    match data {
        [] => 0,
        [a] => i8::from_be_bytes([*a]) as i32,
        [a, b] => i16::from_be_bytes([*a, *b]) as i32,
        [a, b, c, d] => i32::from_be_bytes([*a, *b, *c, *d]),
        _ => panic!("Invalid HID short-item payload length"),
    }
}

// For controller axes, we want to have Data, Variable and Absolute
// As these are the most common
#[derive(Debug, Clone, Copy)]
pub struct MainDataFlags(u16);

impl MainDataFlags {
    pub fn is_constant(self) -> bool {
        self.0 & 0b0000_0001 != 0
    }

    pub fn is_variable(self) -> bool {
        self.0 & 0b0000_0010 != 0
    }

    pub fn is_absolute(self) -> bool {
        self.0 & 0b0000_0100 != 0
    }
}

// Report Ids - a device can have several Report Ids (inputs, battery status)
pub enum ReportKind {
    Input,
    Output,
    Feature,
}

pub struct ReportKey {
    pub kind: ReportKind,
    pub id: Option<u8>,
}

pub struct Field {
    pub report_id: Option<u8>,
    pub bit_offeset: usize,
    pub bit_size: usize,
    pub usage_page: Option<u32>,
    pub usage: Option<u32>,
    pub logical_minimum: Option<i32>,
    pub logical_maximum: Option<i32>,
    pub flags: MainDataFlags,
}

// Usage Page + Usage are ambiguos, can mean completely different things depending on context
// take into consideration vendor usage pages like 0xFF00 through 0xFFFF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Usage {
    pub page: u32,
    pub id: u32,
}

// Parsing errors
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(
        "short item at offset {offset} requires {required} payload bytes, only {remaining} remain"
    )]
    TruncatedShort {
        offset: usize,
        required: usize,
        remaining: usize,
    },

    #[error("long item header is truncated at offset {offset}")]
    TruncatedLongHeader { offset: usize },

    #[error("long item at offset {offset} declares {declared} bytes, only {remaining} remain")]
    TruncatedLong {
        offset: usize,
        declared: usize,
        remaining: usize,
    },
}

// Iterators to make item parsing composable
pub struct ItemIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

// Utils for new and parse_long, necessary for Iterator
impl<'a> ItemIter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn parse_long(&mut self, start: usize) -> Result<RawItem<'a>, ParseError> {
        if self.bytes.len() - self.offset < 2 {
            self.offset = self.bytes.len();
            return Err(ParseError::TruncatedLongHeader { offset: start })
        }

        let size = self.bytes[self.offset] as usize;
        let tag = self.bytes[self.offset + 1];

        self.offset += 2;

        let remaining = self.bytes.len() - self.offset;

        if remaining < size {
            self.offset = self.bytes.len();
            return Err(ParseError::TruncatedLong { offset: start, declared: size, remaining })
        }

        let data = &self.bytes[self.offset..self.offset + size];
        self.offset += size;

        Ok(RawItem::Long(LongItem { tag, data }))
    }
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Result<RawItem<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.bytes.len() {
            return None;
        }

        let start = self.offset;
        let prefix = self.bytes[self.offset];

        self.offset += 1;

        if prefix == 0xFE {
            return Some(self.parse_long(start));
        }

        let size = match prefix & 0b11 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            _ => unreachable!()
        };

        let remaining = self.bytes.len() - self.offset;

        if remaining < size {
            self.offset = self.bytes.len();

            return Some(Err(ParseError::TruncatedShort { offset: start, required: size, remaining }))
        }

        let data = &self.bytes[self.offset..self.offset + size];
        self.offset += size;

        let item_type = match (prefix >> 2) & 0b11 {
            0 => ItemType::Main,
            1 => ItemType::Global,
            2 => ItemType::Local,
            _ => ItemType::Reserved
        };

        let tag = prefix >> 4;

        Some(Ok(RawItem::Short(ShortItem { prefix, item_type, tag, data })))
    }
}

fn main() {
    println!("Hello, world!");
}
