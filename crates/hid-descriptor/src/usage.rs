//! Semantic HID usage identifiers

/// A usage needs both components because the same ID means different things on
/// different pages (and vendor pages occupy `0xFF00..=0xFFFF`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Usage {
    pub page: u32,
    pub id: u32,
}
