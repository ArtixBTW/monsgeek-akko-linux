// HAL Constants - Single source of truth for device identification
//
// All HID device constants live here. Other modules import from hal::constants.

/// Vendor ID for MonsGeek/Akko devices
pub const VENDOR_ID: u16 = 0x3151;

/// All known Bluetooth PIDs (BLE HID devices)
pub const BLUETOOTH_PIDS: &[u16] = &[
    0x5027, // M1 V5 HE Bluetooth
];

/// Check if PID represents a Bluetooth device
#[inline]
pub fn is_bluetooth_pid(pid: u16) -> bool {
    BLUETOOTH_PIDS.contains(&pid)
}

/// All known dongle PIDs (2.4GHz wireless receivers)
pub const DONGLE_PIDS: &[u16] = &[
    0x5038, // M1 V5 HE dongle
    0x503A, // Legacy dongle variant
    0x503D, // Legacy dongle variant
    0x502B, // M1 V5 TMR dongle
    0x5006, // AttackShark K86 dongle
];

/// Check if PID represents a 2.4GHz dongle
#[inline]
pub fn is_dongle_pid(pid: u16) -> bool {
    DONGLE_PIDS.contains(&pid)
}

/// Vendor-specific HID usage page (0xFFFF)
pub const USAGE_PAGE: u16 = 0xFFFF;

/// Alternative vendor usage page seen on some models
pub const USAGE_PAGE_ALT: u16 = 0xFF00;

/// Check if a usage page is a vendor usage page (0xFFFF or 0xFF00)
#[inline]
pub fn is_vendor_usage_page(page: u16) -> bool {
    page == USAGE_PAGE || page == USAGE_PAGE_ALT
}

/// HID Usage for FEATURE interface (interface 2) - for sending commands
pub const USAGE_FEATURE: u16 = 0x02;

/// HID Usage for INPUT interface (interface 1) - for receiving key depth, events
pub const USAGE_INPUT: u16 = 0x01;

/// Interface number for FEATURE interface
pub const INTERFACE_FEATURE: i32 = 2;

/// Interface number for INPUT interface
pub const INTERFACE_INPUT: i32 = 1;

/// Physical inputs on the M1 V5 HE: 81 magnetic switches plus the volume encoder.
///
/// Distinct from the two matrix figures below. The matrix DB counts 85 *named*
/// positions for this board — the 81 switches, both encoder directions, and two
/// pseudo-entries at 96/97 whose HID codes (1, 2) are not valid usages.
pub const KEY_COUNT_M1_V5: u8 = 82;

/// One past the highest matrix position the M1 V5 HE uses, i.e. how far a scan runs.
pub const MATRIX_EXTENT_M1_V5: u8 = 98;

/// Full matrix array dimension for M1 V5 HE (21 columns x 6 rows, mostly gaps).
pub const MATRIX_SIZE_M1_V5: usize = 126;
