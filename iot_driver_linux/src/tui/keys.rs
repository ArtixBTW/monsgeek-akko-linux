//! Key vocabulary shared across TUI tabs: the bindable key and consumer-usage
//! lists, and the layer filter.

pub(in crate::tui) use crate::key_action::CONSUMER_KEYS;
use crate::protocol::hid::key_name;

/// Layer filter for the key-mapping views.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(in crate::tui) enum RemapLayerView {
    #[default]
    Both,
    L0,
    L1,
    Fn,
}

impl RemapLayerView {
    pub fn cycle(self) -> Self {
        match self {
            Self::Both => Self::L0,
            Self::L0 => Self::L1,
            Self::L1 => Self::Fn,
            Self::Fn => Self::Both,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "All",
            Self::L0 => "L0",
            Self::L1 => "L1",
            Self::Fn => "Fn",
        }
    }
}

/// Every bindable HID keyboard usage, including the modifiers `0xE0..=0xE7`
/// (the firmware treats those as ordinary usages), sorted by name.
pub(in crate::tui) fn all_hid_keys() -> Vec<(u8, &'static str)> {
    // `key_name` collapses 0x68..=0x73 to the single label "F13-F24", so those are
    // filtered out here and re-added individually.
    let mut keys: Vec<(u8, &'static str)> = (0x04..=0x73u8)
        .chain(0xE0..=0xE7)
        .filter_map(|code| {
            let name = key_name(code);
            (name != "?" && name != "F13-F24").then_some((code, name))
        })
        .collect();
    const F13_F24: [&str; 12] = [
        "F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
    ];
    keys.extend(
        F13_F24
            .iter()
            .enumerate()
            .map(|(i, &name)| (0x68 + i as u8, name)),
    );
    keys.sort_by_key(|&(_, name)| name);
    keys
}
