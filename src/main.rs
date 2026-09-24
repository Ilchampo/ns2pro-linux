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

// Uncommon long items handler
// Prevents mistaking long items with short items
pub struct LongItem<'a> {
    pub tag: u8,
    pub data: &'a [u8],
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

fn main() {
    println!("Hello, world!");
}
