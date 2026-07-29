//! Key vocabulary shared across TUI tabs: the bindable key and consumer-usage
//! lists, and the layer filter.

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

/// Consumer-page usages the firmware accepts in a keymatrix slot (config_type 3).
pub(in crate::tui) const CONSUMER_KEYS: &[(u16, &str)] = &[
    (0x00B5, "Next Track"),
    (0x00B6, "Previous Track"),
    (0x00B7, "Stop"),
    (0x00CD, "Play/Pause"),
    (0x00E2, "Mute"),
    (0x00E9, "Volume Up"),
    (0x00EA, "Volume Down"),
    (0x006F, "Brightness Up"),
    (0x0070, "Brightness Down"),
    (0x018A, "Mail"),
    (0x0192, "Calculator"),
    (0x0194, "My Computer"),
    (0x0221, "Search"),
    (0x0223, "Browser Home"),
];

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
