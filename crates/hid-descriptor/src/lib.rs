//! Parsing primitives for USB HID report descriptors.
//!
//! Parsing happens in two layers: [`ItemIter`] splits bytes into raw HID items,
//! then [`DescriptorBuilder`] applies the stateful HID rules to typed items.

mod item;
mod parser;
mod semantic;
mod usage;

pub use item::{
    GlobalItem, GlobalTag, Item, ItemType, LocalItem, LocalTag, LongItem, MainDataFlags, MainItem,
    MainTag, RawItem, ShortItem,
};
pub use parser::{ItemIter, ParseError};
pub use semantic::{DescriptorBuilder, GlobalState, LocalState, SemanticError};
pub use usage::Usage;
