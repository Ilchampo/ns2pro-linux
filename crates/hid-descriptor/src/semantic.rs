//! Stateful interpretation of typed HID items.

use crate::{GlobalItem, GlobalTag, Item, LocalItem, LocalTag, Usage};

/// Global values persist across main items and can be saved with Push.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalState {
    pub usage_page: Option<u32>,
    pub logical_minimum: Option<i32>,
    pub logical_maximum: Option<i32>,
    pub physical_minimum: Option<i32>,
    pub physical_maximum: Option<i32>,
    pub unit_exponent: Option<i32>,
    pub unit: Option<u32>,
    pub report_size: Option<u32>,
    pub report_id: Option<u8>,
    pub report_count: Option<u32>,
}

/// Local values describe only the next main item and are then cleared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalState {
    pub usages: Vec<Usage>,
    pub usage_minimum: Option<Usage>,
    pub usage_maximum: Option<Usage>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SemanticError {
    #[error("global Pop has no matching Push")]
    GlobalStackUnderflow,
    #[error("Report ID must be in the range 1..=255")]
    InvalidReportId,
}

/// Applies HID's global and local scoping rules.
///
/// Field emission and collection construction belong to the following parser
/// sections, so this builder deliberately stops at state application.
#[derive(Debug, Default)]
pub struct DescriptorBuilder {
    global: GlobalState,
    global_stack: Vec<GlobalState>,
    local: LocalState,
}

impl DescriptorBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn global(&self) -> &GlobalState {
        &self.global
    }
    pub fn local(&self) -> &LocalState {
        &self.local
    }

    /// Apply one typed item. Unknown and long items are retained by the syntax
    /// layer but do not mutate the state understood at this milestone.
    pub fn apply(&mut self, item: Item<'_>) -> Result<(), SemanticError> {
        match item {
            Item::Global(item) => self.apply_global(item),
            Item::Local(item) => {
                self.apply_local(item);
                Ok(())
            }
            Item::Main(_) => {
                // Every main item consumes the local state, including an
                // unfamiliar main tag that this parser cannot interpret yet.
                self.local = LocalState::default();
                Ok(())
            }
            Item::Reserved(_) | Item::Long(_) => Ok(()),
        }
    }

    fn apply_global(&mut self, item: GlobalItem<'_>) -> Result<(), SemanticError> {
        match item.tag {
            GlobalTag::UsagePage => self.global.usage_page = Some(unsigned_value(item.data)),
            GlobalTag::LogicalMinimum => {
                self.global.logical_minimum = Some(signed_value(item.data))
            }
            GlobalTag::LogicalMaximum => {
                // HID maxima are signed only when their corresponding minimum is negative.
                self.global.logical_maximum =
                    Some(if self.global.logical_minimum.unwrap_or(0) < 0 {
                        signed_value(item.data)
                    } else {
                        unsigned_value(item.data) as i32
                    });
            }
            GlobalTag::PhysicalMinimum => {
                self.global.physical_minimum = Some(signed_value(item.data))
            }
            GlobalTag::PhysicalMaximum => {
                self.global.physical_maximum =
                    Some(if self.global.physical_minimum.unwrap_or(0) < 0 {
                        signed_value(item.data)
                    } else {
                        unsigned_value(item.data) as i32
                    });
            }
            GlobalTag::UnitExponent => self.global.unit_exponent = Some(signed_value(item.data)),
            GlobalTag::Unit => self.global.unit = Some(unsigned_value(item.data)),
            GlobalTag::ReportSize => self.global.report_size = Some(unsigned_value(item.data)),
            GlobalTag::ReportId => {
                let id = unsigned_value(item.data);
                self.global.report_id = Some(
                    u8::try_from(id)
                        .ok()
                        .filter(|id| *id != 0)
                        .ok_or(SemanticError::InvalidReportId)?,
                );
            }
            GlobalTag::ReportCount => self.global.report_count = Some(unsigned_value(item.data)),
            GlobalTag::Push => self.global_stack.push(self.global.clone()),
            GlobalTag::Pop => {
                self.global = self
                    .global_stack
                    .pop()
                    .ok_or(SemanticError::GlobalStackUnderflow)?;
            }
            GlobalTag::Unknown(_) => {}
        }
        Ok(())
    }

    fn apply_local(&mut self, item: LocalItem<'_>) {
        let usage = || Usage {
            page: self.global.usage_page.unwrap_or(0),
            id: unsigned_value(item.data),
        };
        match item.tag {
            LocalTag::Usage => self.local.usages.push(usage()),
            LocalTag::UsageMinimum => self.local.usage_minimum = Some(usage()),
            LocalTag::UsageMaximum => self.local.usage_maximum = Some(usage()),
            _ => {}
        }
    }
}

/// HID scalar payloads are little-endian and at most four bytes wide.
fn unsigned_value(data: &[u8]) -> u32 {
    let mut bytes = [0; 4];
    bytes[..data.len()].copy_from_slice(data);
    u32::from_le_bytes(bytes)
}

/// Interpret the payload at its encoded width so the sign bit is extended.
fn signed_value(data: &[u8]) -> i32 {
    match data {
        [] => 0,
        [value] => i32::from(i8::from_le_bytes([*value])),
        [low, high] => i32::from(i16::from_le_bytes([*low, *high])),
        [a, b, c, d] => i32::from_le_bytes([*a, *b, *c, *d]),
        _ => unreachable!("short-item payloads are at most four bytes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ItemIter, RawItem};

    fn apply_descriptor(builder: &mut DescriptorBuilder, bytes: &[u8]) {
        for raw in ItemIter::new(bytes) {
            builder.apply(Item::from(raw.unwrap())).unwrap();
        }
    }

    #[test]
    fn signed_values_use_little_endian_and_sign_extension() {
        let mut builder = DescriptorBuilder::new();
        // Logical Minimum (-256), encoded as a two-byte global item.
        apply_descriptor(&mut builder, &[0x16, 0x00, 0xFF]);
        assert_eq!(builder.global().logical_minimum, Some(-256));
    }

    #[test]
    fn local_usages_reset_after_main_item() {
        let mut builder = DescriptorBuilder::new();
        // Usage Page (Button), Usage (1), then Input.
        apply_descriptor(&mut builder, &[0x05, 0x09, 0x09, 0x01, 0x81, 0x02]);
        assert!(builder.local().usages.is_empty());
        assert_eq!(builder.global().usage_page, Some(0x09));
    }

    #[test]
    fn push_and_pop_restore_global_state() {
        let mut builder = DescriptorBuilder::new();
        // Page 1, Push, Page 9, Pop.
        apply_descriptor(&mut builder, &[0x05, 0x01, 0xA4, 0x05, 0x09, 0xB4]);
        assert_eq!(builder.global().usage_page, Some(0x01));
    }

    #[test]
    fn tag_number_is_decoded_in_its_type_context() {
        let tags: Vec<_> = ItemIter::new(&[0x81, 0, 0x85, 1, 0x89, 2])
            .map(|raw| Item::from(raw.unwrap()))
            .collect();
        assert!(matches!(tags[0], Item::Main(_)));
        assert!(matches!(tags[1], Item::Global(_)));
        assert!(matches!(tags[2], Item::Local(_)));
    }

    #[test]
    fn raw_items_retain_borrowed_payloads() {
        let bytes = [0x05, 0x01];
        let RawItem::Short(item) = ItemIter::new(&bytes).next().unwrap().unwrap() else {
            panic!("expected short item");
        };
        assert_eq!(item.data.as_ptr(), bytes[1..].as_ptr());
    }
}
