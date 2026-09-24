//! HID item representations and tag decoding.

/// The two type bits in a short-item prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Main,
    Global,
    Local,
    Reserved,
}

impl ItemType {
    pub(crate) fn from_prefix(prefix: u8) -> Self {
        match (prefix >> 2) & 0b11 {
            0 => Self::Main,
            1 => Self::Global,
            2 => Self::Local,
            3 => Self::Reserved,
            _ => unreachable!("the mask produces only two bits"),
        }
    }
}

/// A short item whose payload borrows from the original descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortItem<'a> {
    pub prefix: u8,
    pub item_type: ItemType,
    pub tag: u8,
    pub data: &'a [u8],
}

/// The uncommon long-item format, retained without interpreting its tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LongItem<'a> {
    pub tag: u8,
    pub data: &'a [u8],
}

/// An item split from the byte stream, before its type-specific tag is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawItem<'a> {
    Short(ShortItem<'a>),
    Long(LongItem<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTag {
    Input,
    Output,
    Collection,
    Feature,
    EndCollection,
    Unknown(u8),
}

impl From<u8> for MainTag {
    fn from(tag: u8) -> Self {
        match tag {
            0x8 => Self::Input,
            0x9 => Self::Output,
            0xA => Self::Collection,
            0xB => Self::Feature,
            0xC => Self::EndCollection,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalTag {
    UsagePage,
    LogicalMinimum,
    LogicalMaximum,
    PhysicalMinimum,
    PhysicalMaximum,
    UnitExponent,
    Unit,
    ReportSize,
    ReportId,
    ReportCount,
    Push,
    Pop,
    Unknown(u8),
}

impl From<u8> for GlobalTag {
    fn from(tag: u8) -> Self {
        match tag {
            0x0 => Self::UsagePage,
            0x1 => Self::LogicalMinimum,
            0x2 => Self::LogicalMaximum,
            0x3 => Self::PhysicalMinimum,
            0x4 => Self::PhysicalMaximum,
            0x5 => Self::UnitExponent,
            0x6 => Self::Unit,
            0x7 => Self::ReportSize,
            0x8 => Self::ReportId,
            0x9 => Self::ReportCount,
            0xA => Self::Push,
            0xB => Self::Pop,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTag {
    Usage,
    UsageMinimum,
    UsageMaximum,
    DesignatorIndex,
    DesignatorMinimum,
    DesignatorMaximum,
    StringIndex,
    StringMinimum,
    StringMaximum,
    Delimiter,
    Unknown(u8),
}

impl From<u8> for LocalTag {
    fn from(tag: u8) -> Self {
        match tag {
            0x0 => Self::Usage,
            0x1 => Self::UsageMinimum,
            0x2 => Self::UsageMaximum,
            0x3 => Self::DesignatorIndex,
            0x4 => Self::DesignatorMinimum,
            0x5 => Self::DesignatorMaximum,
            0x7 => Self::StringIndex,
            0x8 => Self::StringMinimum,
            0x9 => Self::StringMaximum,
            0xA => Self::Delimiter,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainItem<'a> {
    pub tag: MainTag,
    pub data: &'a [u8],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalItem<'a> {
    pub tag: GlobalTag,
    pub data: &'a [u8],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalItem<'a> {
    pub tag: LocalTag,
    pub data: &'a [u8],
}

/// A short item with a tag decoded in the context of its item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item<'a> {
    Main(MainItem<'a>),
    Global(GlobalItem<'a>),
    Local(LocalItem<'a>),
    Reserved(ShortItem<'a>),
    Long(LongItem<'a>),
}

impl<'a> From<RawItem<'a>> for Item<'a> {
    fn from(raw: RawItem<'a>) -> Self {
        match raw {
            RawItem::Long(item) => Self::Long(item),
            RawItem::Short(item) => match item.item_type {
                ItemType::Main => Self::Main(MainItem {
                    tag: item.tag.into(),
                    data: item.data,
                }),
                ItemType::Global => Self::Global(GlobalItem {
                    tag: item.tag.into(),
                    data: item.data,
                }),
                ItemType::Local => Self::Local(LocalItem {
                    tag: item.tag.into(),
                    data: item.data,
                }),
                ItemType::Reserved => Self::Reserved(item),
            },
        }
    }
}

/// Bit flags carried by Input, Output, and Feature main items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainDataFlags(u32);

impl MainDataFlags {
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
    pub fn bits(self) -> u32 {
        self.0
    }
    pub fn is_constant(self) -> bool {
        self.0 & (1 << 0) != 0
    }
    pub fn is_variable(self) -> bool {
        self.0 & (1 << 1) != 0
    }
    pub fn is_relative(self) -> bool {
        self.0 & (1 << 2) != 0
    }
    pub fn is_absolute(self) -> bool {
        !self.is_relative()
    }
}
