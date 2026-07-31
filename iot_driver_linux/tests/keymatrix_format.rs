//! Integration tests for the data behind the `keymatrix` listing.
//!
//! A key counts as customised when its decoded action differs from the position's
//! factory default; `[0, 0, 0, 0]` (disabled / transparent) and the Fn key at its
//! own position are not customisations. See `keymap::is_user_remap`.
//!
//! Factory defaults are derived from the transport matrix key names
//! (e.g. position 12 = "F1" → HID 0x3A), NOT from device_matrices.json
//! which uses a different position numbering.

use iot_driver::key_action::KeyAction;
use iot_driver::keymap::{KeyRef, Layer, is_user_remap as shared_is_user_remap};
use iot_driver::protocol::hid;
use monsgeek_transport::protocol::{HidUsage, MatrixPos, matrix};

/// Helper: detect whether a 4-byte key config represents a user remap.
/// Mirrors the logic in `commands::keymap::list_remaps` and `tui::load_remaps`.
///
/// `default_hid_code`: the factory default HID keycode for this matrix position,
/// derived from `hid::key_code_from_name(matrix::key_name(i))`.
fn is_user_remap(k: &[u8; 4], default_hid_code: HidUsage) -> bool {
    // Disabled: never a remap
    if *k == [0, 0, 0, 0] {
        return false;
    }

    // Fn key at physical Fn position: factory default
    if matches!(KeyAction::from_config_bytes(*k), KeyAction::Fn) {
        return false;
    }

    // Non-zero config_type (mouse/macro/consumer/etc): always a remap
    if k[0] != 0 {
        return true;
    }

    // Byte 1 non-zero (user remap format or combo): always a remap
    if k[1] != 0 {
        return true;
    }

    // config_type=0, byte1=0, byte2!=0: compare against factory default
    k[2] != default_hid_code.get()
}

/// Resolve the default HID code for a matrix position, same as the production code.
fn default_for(pos: u8) -> HidUsage {
    hid::key_code_from_name(matrix::key_name(MatrixPos::new(pos))).unwrap_or(HidUsage::NONE)
}

// ── hid::key_code_from_name resolution (still useful for display) ──

#[test]
fn hid_key_code_standard_keys() {
    assert_eq!(hid::key_code_from_name("Esc"), Some(HidUsage::new(0x29)));
    assert_eq!(hid::key_code_from_name("Tab"), Some(HidUsage::new(0x2B)));
    assert_eq!(hid::key_code_from_name("Caps"), Some(HidUsage::new(0x39)));
    assert_eq!(hid::key_code_from_name("A"), Some(HidUsage::new(0x04)));
    assert_eq!(hid::key_code_from_name("Z"), Some(HidUsage::new(0x1D)));
    assert_eq!(hid::key_code_from_name("F1"), Some(HidUsage::new(0x3A)));
    assert_eq!(hid::key_code_from_name("F12"), Some(HidUsage::new(0x45)));
    assert_eq!(hid::key_code_from_name("Del"), Some(HidUsage::new(0x4C)));
    assert_eq!(hid::key_code_from_name("Home"), Some(HidUsage::new(0x4A)));
}

#[test]
fn hid_key_code_abbreviations() {
    assert_eq!(hid::key_code_from_name("Ent"), Some(HidUsage::new(0x28)));
    assert_eq!(hid::key_code_from_name("Bksp"), Some(HidUsage::new(0x2A)));
    assert_eq!(hid::key_code_from_name("LShf"), Some(HidUsage::new(0xE1)));
    assert_eq!(hid::key_code_from_name("RShf"), Some(HidUsage::new(0xE5)));
    assert_eq!(hid::key_code_from_name("LCtl"), Some(HidUsage::new(0xE0)));
    assert_eq!(hid::key_code_from_name("RCtl"), Some(HidUsage::new(0xE4)));
    assert_eq!(hid::key_code_from_name("Win"), Some(HidUsage::new(0xE3)));
    assert_eq!(hid::key_code_from_name("LAlt"), Some(HidUsage::new(0xE2)));
    assert_eq!(hid::key_code_from_name("RAlt"), Some(HidUsage::new(0xE6)));
}

// ── Byte-pattern detection ──

#[test]
fn factory_default_not_detected() {
    // [0, 0, keycode, 0] where keycode matches factory default → NOT a remap
    assert!(!is_user_remap(&[0, 0, 0x29, 0], HidUsage::new(0x29))); // Escape at Escape position
    assert!(!is_user_remap(&[0, 0, 0x04, 0], HidUsage::new(0x04))); // A at A position
    assert!(!is_user_remap(&[0, 0, 0xE1, 0], HidUsage::new(0xE1))); // LShift at LShift position
}

#[test]
fn factory_default_changed_detected() {
    // [0, 0, new_code, 0] where new_code differs from factory default → IS a remap
    assert!(is_user_remap(&[0, 0, 0x04, 0], HidUsage::new(0x39))); // A at Caps position (default 0x39)
    assert!(is_user_remap(&[0, 0, 0x29, 0], HidUsage::new(0x35))); // Escape at ` position (default 0x35)
}

#[test]
fn identity_map_not_detected_any_layer() {
    // Both base layer and Fn layer use the same defaults: [0, 0, default_code, 0]
    // An identity map should NOT be detected on either layer.
    assert!(!is_user_remap(&[0, 0, 0x3A, 0], HidUsage::new(0x3A))); // F1 → F1 (identity)
    assert!(!is_user_remap(&[0, 0, 0x04, 0], HidUsage::new(0x04))); // A → A (identity)
}

#[test]
fn disabled_not_detected() {
    assert!(!is_user_remap(&[0, 0, 0, 0], HidUsage::new(0x29)));
    assert!(!is_user_remap(&[0, 0, 0, 0], HidUsage::new(0)));
}

#[test]
fn user_remap_detected() {
    // [0, keycode, 0, 0] — user remap format, byte1 non-zero
    assert!(is_user_remap(&[0, 0x04, 0, 0], HidUsage::new(0x39))); // remapped to A
    assert!(is_user_remap(&[0, 0x29, 0, 0], HidUsage::new(0x35))); // remapped to Escape
}

#[test]
fn combo_detected() {
    // [0, mods, key, 0] — both bytes 1 and 2 non-zero
    assert!(is_user_remap(&[0, 0x01, 0x06, 0], HidUsage::new(0x39))); // Ctrl+C
    assert!(is_user_remap(&[0, 0x04, 0x29, 0], HidUsage::new(0x35))); // Alt+Escape
}

#[test]
fn macro_detected() {
    // config_type 9 = macro
    assert!(is_user_remap(&[9, 0, 0, 0], HidUsage::new(0xE0))); // Macro(0)
    assert!(is_user_remap(&[9, 0, 2, 0], HidUsage::new(0xE0))); // Macro(2)
}

#[test]
fn mouse_detected() {
    // config_type 1 = mouse
    assert!(is_user_remap(&[1, 0, 1, 0], HidUsage::new(0xE0))); // Mouse1
}

// ── Default resolution from transport matrix ──

#[test]
fn default_for_matches_transport_matrix() {
    // Verify default_for() resolves key names from the transport matrix.
    // Col 0 (positions 0-5)
    assert_eq!(default_for(0), HidUsage::new(0x29)); // Esc
    assert_eq!(default_for(1), HidUsage::new(0x35)); // `
    assert_eq!(default_for(2), HidUsage::new(0x2B)); // Tab
    assert_eq!(default_for(3), HidUsage::new(0x39)); // Caps
    assert_eq!(default_for(4), HidUsage::new(0xE1)); // LShf
    assert_eq!(default_for(5), HidUsage::new(0xE0)); // LCtl
    // F-row and modifier positions (verified against firmware GET_KEYMATRIX)
    assert_eq!(default_for(6), HidUsage::new(0x3A)); // F1 at Col 1 Row 0
    assert_eq!(default_for(11), HidUsage::new(0xE3)); // Win at Col 1 Row 5
    assert_eq!(default_for(12), HidUsage::new(0x3B)); // F2 at Col 2 Row 0
    assert_eq!(default_for(17), HidUsage::new(0xE2)); // LAlt at Col 2 Row 5
    assert_eq!(default_for(41), HidUsage::new(0x2C)); // Space at Col 6 Row 5
    assert_eq!(default_for(59), HidUsage::new(0xE6)); // RAlt at Col 9 Row 5
    assert_eq!(default_for(71), HidUsage::new(0xE4)); // RCtl at Col 11 Row 5
    assert_eq!(default_for(78), HidUsage::new(0x4C)); // Del at Col 13 Row 0
}

// ── KeyAction parsing from wire bytes ──

#[test]
fn parse_factory_default_format() {
    // [0, 0, code, 0] — keycode at byte 2
    assert_eq!(
        KeyAction::from_config_bytes([0, 0, 0x29, 0]),
        KeyAction::Key(HidUsage::new(0x29))
    );
}

#[test]
fn parse_user_remap_format() {
    // [0, code, 0, 0] — keycode at byte 1
    assert_eq!(
        KeyAction::from_config_bytes([0, 0x04, 0, 0]),
        KeyAction::Key(HidUsage::new(0x04))
    );
    assert_eq!(
        KeyAction::from_config_bytes([0, 0x29, 0, 0]),
        KeyAction::Key(HidUsage::new(0x29))
    );
}

#[test]
fn parse_combo_format() {
    // [0, skey, key, key2] — three independent HID usages, modifiers as 0xE0..=0xE7
    assert_eq!(
        KeyAction::from_config_bytes([0, 0xE0, 0x06, 0]),
        KeyAction::Combo {
            keys: [HidUsage::new(0xE0), HidUsage::new(0x06), HidUsage::new(0)]
        }
    );
}

#[test]
fn parse_disabled() {
    assert_eq!(
        KeyAction::from_config_bytes([0, 0, 0, 0]),
        KeyAction::Disabled
    );
}

// ── Simulated full matrix scan (byte-pattern detection) ──

#[test]
fn simulated_remap_scan() {
    // Build a fake keymatrix buffer using correct firmware byte formats.
    // Factory defaults derived from transport matrix::key_name → hid::key_code_from_name.
    let key_count = 6;
    let defaults: Vec<HidUsage> = (0..key_count as u8).map(default_for).collect();

    let mut data = vec![0u8; key_count * 4];

    // Position 0: Esc — factory default [0, 0, 0x29, 0] → NOT a remap
    data[0..4].copy_from_slice(&[0, 0, 0x29, 0]);
    // Position 1: ` — factory default [0, 0, 0x35, 0] → NOT a remap
    data[4..8].copy_from_slice(&[0, 0, 0x35, 0]);
    // Position 2: Tab — factory default [0, 0, 0x2B, 0] → NOT a remap
    data[8..12].copy_from_slice(&[0, 0, 0x2B, 0]);
    // Position 3: Caps — remapped to A [0, 0, 0x04, 0] → REMAP (0x04 != default 0x39)
    data[12..16].copy_from_slice(&[0, 0, 0x04, 0]);
    // Position 4: LShf — factory default [0, 0, 0xE1, 0] → NOT a remap
    data[16..20].copy_from_slice(&[0, 0, 0xE1, 0]);
    // Position 5: LCtl — macro assignment [9, 0, 0, 0] → REMAP
    data[20..24].copy_from_slice(&[9, 0, 0, 0]);

    let mut remaps = Vec::new();
    for i in 0..key_count {
        let name = matrix::key_name(MatrixPos::new(i as u8));
        if name == "?" {
            continue;
        }
        let k: [u8; 4] = [
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ];

        if is_user_remap(&k, defaults[i]) {
            let action = KeyAction::from_config_bytes(k);
            remaps.push((i, name, action));
        }
    }

    assert_eq!(remaps.len(), 2);
    assert_eq!(remaps[0], (3, "Caps", KeyAction::Key(HidUsage::new(0x04)))); // A
    assert_eq!(
        remaps[1],
        (5, "LCtl", KeyAction::Macro { index: 0, kind: 0 })
    );
}

#[test]
fn simulated_remap_byte2_format() {
    // Tests the critical case: firmware stores remap as [0, 0, new_code, 0]
    // which is the same byte format as factory defaults.
    // Only detectable by comparing against the factory default keycode.
    let key_count = 6;
    let defaults: Vec<HidUsage> = (0..key_count as u8).map(default_for).collect();

    let mut data = vec![0u8; key_count * 4];

    // Position 0: Esc → remapped to Tab [0, 0, 0x2B, 0] — REMAP (0x2B != 0x29)
    data[0..4].copy_from_slice(&[0, 0, 0x2B, 0]);
    // Position 1: ` → factory default [0, 0, 0x35, 0] — NOT a remap
    data[4..8].copy_from_slice(&[0, 0, 0x35, 0]);
    // Position 2: Tab → remapped to Esc [0, 0, 0x29, 0] — REMAP (0x29 != 0x2B)
    data[8..12].copy_from_slice(&[0, 0, 0x29, 0]);
    // Position 3: Caps → remapped to A [0, 0, 0x04, 0] — REMAP (0x04 != 0x39)
    data[12..16].copy_from_slice(&[0, 0, 0x04, 0]);
    // Position 4: LShf → factory default [0, 0, 0xE1, 0] — NOT a remap
    data[16..20].copy_from_slice(&[0, 0, 0xE1, 0]);
    // Position 5: LCtl → factory default [0, 0, 0xE0, 0] — NOT a remap
    data[20..24].copy_from_slice(&[0, 0, 0xE0, 0]);

    let mut remaps = Vec::new();
    for i in 0..key_count {
        let name = matrix::key_name(MatrixPos::new(i as u8));
        if name == "?" {
            continue;
        }
        let k: [u8; 4] = [
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ];

        if is_user_remap(&k, defaults[i]) {
            let action = KeyAction::from_config_bytes(k);
            remaps.push((i, name, action));
        }
    }

    assert_eq!(remaps.len(), 3);
    assert_eq!(remaps[0], (0, "Esc", KeyAction::Key(HidUsage::new(0x2B)))); // Tab
    assert_eq!(remaps[1], (2, "Tab", KeyAction::Key(HidUsage::new(0x29)))); // Escape
    assert_eq!(remaps[2], (3, "Caps", KeyAction::Key(HidUsage::new(0x04)))); // A
}

#[test]
fn simulated_layer1_identity_maps_filtered() {
    // Layer 1 also stores identity maps [0, 0, default_code, 0] for each key.
    // These should be filtered out the same as layer 0.
    let key_count = 6;
    let defaults: Vec<HidUsage> = (0..key_count as u8).map(default_for).collect();

    let mut data = vec![0u8; key_count * 4];

    // All positions: identity maps (same code as default)
    for i in 0..key_count {
        data[i * 4..i * 4 + 4].copy_from_slice(&[0, 0, defaults[i].get(), 0]);
    }

    let mut remaps = Vec::new();
    for i in 0..key_count {
        let name = matrix::key_name(MatrixPos::new(i as u8));
        if name == "?" {
            continue;
        }
        let k: [u8; 4] = [
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ];

        if is_user_remap(&k, defaults[i]) {
            let action = KeyAction::from_config_bytes(k);
            remaps.push((i, name, action));
        }
    }

    assert_eq!(
        remaps.len(),
        0,
        "Identity maps should be filtered on both layers"
    );
}

#[test]
fn simulated_fn_layer_scan() {
    // Fn layer: a mix of disabled (identity maps filtered), non-zero config types,
    // and user byte1 remaps.
    let key_count = 6;
    let defaults: Vec<HidUsage> = (0..key_count as u8).map(default_for).collect();

    let mut data = vec![0u8; key_count * 4];

    // Position 0: Disabled [0, 0, 0, 0] → NOT a remap
    data[0..4].copy_from_slice(&[0, 0, 0, 0]);
    // Position 1: Identity map [0, 0, 0x35, 0] → NOT a remap (same as default)
    data[4..8].copy_from_slice(&[0, 0, defaults[1].get(), 0]);
    // Position 2: Macro(1) [9, 0, 1, 0] → REMAP (config_type != 0)
    data[8..12].copy_from_slice(&[9, 0, 1, 0]);
    // Position 3: Mouse1 [1, 0, 1, 0] → REMAP (config_type != 0)
    data[12..16].copy_from_slice(&[1, 0, 1, 0]);
    // Position 4: Identity map [0, 0, 0xE1, 0] → NOT a remap (same as default)
    data[16..20].copy_from_slice(&[0, 0, defaults[4].get(), 0]);
    // Position 5: User remap to F13 [0, 0x68, 0, 0] → REMAP (byte1 != 0)
    data[20..24].copy_from_slice(&[0, 0x68, 0, 0]);

    let mut remaps = Vec::new();
    for i in 0..key_count {
        let name = matrix::key_name(MatrixPos::new(i as u8));
        if name == "?" {
            continue;
        }
        let k: [u8; 4] = [
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ];

        if is_user_remap(&k, defaults[i]) {
            let action = KeyAction::from_config_bytes(k);
            remaps.push((i, name, action));
        }
    }

    assert_eq!(remaps.len(), 3);
    assert_eq!(
        remaps[0],
        (2, "Tab", KeyAction::Macro { index: 1, kind: 0 })
    );
    assert_eq!(remaps[1], (3, "Caps", KeyAction::Mouse(1)));
    assert_eq!(remaps[2], (5, "LCtl", KeyAction::Key(HidUsage::new(0x68))));
}

// ── Display formatting ──

#[test]
fn remap_display_format() {
    assert_eq!(format!("{}", KeyAction::Key(HidUsage::new(0x29))), "Escape");
    assert_eq!(format!("{}", KeyAction::Key(HidUsage::new(0x35))), "`");
    assert_eq!(format!("{}", KeyAction::Mouse(1)), "Mouse1");
    assert_eq!(
        format!("{}", KeyAction::Macro { index: 0, kind: 0 }),
        "Macro(0)"
    );
    assert_eq!(
        format!(
            "{}",
            KeyAction::Combo {
                keys: [HidUsage::new(0xE0), HidUsage::new(0x06), HidUsage::new(0)]
            }
        ),
        "LCtrl+C"
    );
}

// ── Fn key false-positive fix ──

#[test]
fn fn_key_not_detected_as_remap() {
    // [10, 1, 0, 0] = Fn key — factory default at the physical Fn position
    assert!(!is_user_remap(&[10, 1, 0, 0], HidUsage::new(0xE4)));
    assert!(!is_user_remap(&[10, 1, 0, 0], HidUsage::new(0)));
}

// ── Consumer / LED control parsing (Fn layer entries) ──

#[test]
fn parse_consumer_volume_up() {
    let action = KeyAction::from_config_bytes([3, 0, 0xE9, 0]);
    assert_eq!(action, KeyAction::Consumer(0x00E9));
    assert_eq!(action.to_string(), "Volume Up");
}

#[test]
fn parse_consumer_calculator() {
    // 146 + 1*256 = 402 = 0x192
    let action = KeyAction::from_config_bytes([3, 0, 0x92, 0x01]);
    assert_eq!(action, KeyAction::Consumer(0x0192));
    assert_eq!(action.to_string(), "Calculator");
}

#[test]
fn parse_led_brightness() {
    let action = KeyAction::from_config_bytes([13, 2, 1, 0]);
    assert_eq!(action, KeyAction::LedControl { data: [2, 1, 0] });
    assert_eq!(action.to_string(), "LED Brightness Up");
}

#[test]
fn simulated_fn_layer_with_consumer_and_led() {
    // Simulated Fn layer data with consumer keys and LED controls
    let key_count = 6;
    let mut data = vec![0u8; key_count * 4];

    // Position 0: Disabled → skip
    data[0..4].copy_from_slice(&[0, 0, 0, 0]);
    // Position 1: Consumer Volume Up [3, 0, 0xE9, 0]
    data[4..8].copy_from_slice(&[3, 0, 0xE9, 0]);
    // Position 2: Consumer Play/Pause [3, 0, 0xCD, 0]
    data[8..12].copy_from_slice(&[3, 0, 0xCD, 0]);
    // Position 3: LED Brightness Up [13, 2, 1, 0]
    data[12..16].copy_from_slice(&[13, 2, 1, 0]);
    // Position 4: ConnectionMode (type 14) [14, 1, 0, 0]
    data[16..20].copy_from_slice(&[14, 1, 0, 0]);
    // Position 5: Fn key [10, 1, 0, 0] — should NOT be detected as remap
    data[20..24].copy_from_slice(&[10, 1, 0, 0]);

    let mut entries = Vec::new();
    for i in 0..key_count {
        let k: [u8; 4] = [
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ];
        if k == [0, 0, 0, 0] {
            continue;
        }
        let action = KeyAction::from_config_bytes(k);
        entries.push((i, action));
    }

    // 5 non-zero entries: Consumer x2, LedControl, ConnectionMode, Fn
    assert_eq!(entries.len(), 5);
    assert_eq!(entries[0].1, KeyAction::Consumer(0x00E9));
    assert_eq!(entries[0].1.to_string(), "Volume Up");
    assert_eq!(entries[1].1, KeyAction::Consumer(0x00CD));
    assert_eq!(entries[1].1.to_string(), "Play/Pause");
    assert!(matches!(entries[2].1, KeyAction::LedControl { .. }));
    assert_eq!(entries[2].1.to_string(), "LED Brightness Up");
    assert!(matches!(
        entries[3].1,
        KeyAction::ConnectionMode {
            b1: 1,
            b2: 0,
            b3: 0
        }
    ));
    assert_eq!(entries[4].1, KeyAction::Fn);
}

// ── Caps key (position 3) roundtrip tests for all mapping types ──
//
// Each test: build a KeyAction → to_config_bytes → from_config_bytes → verify
// roundtrip equality and is_user_remap detection.  Caps factory default is
// CapsLock (HID 0x39).

const CAPS_POS: u8 = 3;

fn caps_default() -> HidUsage {
    default_for(CAPS_POS) // 0x39
}

#[test]
fn caps_roundtrip_simple_key() {
    let action = KeyAction::Key(HidUsage::new(0x05)); // B
    let wire = action.to_config_bytes();
    assert_eq!(wire, [0, 0, 0x05, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_identity_not_detected() {
    // Caps mapped to CapsLock = factory default → NOT a remap
    let action = KeyAction::Key(HidUsage::new(0x39));
    let wire = action.to_config_bytes();
    assert_eq!(wire, [0, 0, 0x39, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(!is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_combo() {
    let action = KeyAction::Combo {
        keys: [HidUsage::new(0xE0), HidUsage::new(0x06), HidUsage::new(0)], // LCtrl + C
    };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [0, 0xE0, 0x06, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_combo_shift_alt() {
    let action = KeyAction::Combo {
        keys: [
            HidUsage::new(0xE1),
            HidUsage::new(0xE2),
            HidUsage::new(0x3C),
        ], // LShift + LAlt + F3
    };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [0, 0xE1, 0xE2, 0x3C]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_macro_repeat() {
    let action = KeyAction::Macro { index: 0, kind: 0 };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [9, 0, 0, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_macro_toggle() {
    let action = KeyAction::Macro { index: 3, kind: 1 };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [9, 1, 3, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_macro_hold() {
    let action = KeyAction::Macro { index: 7, kind: 2 };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [9, 2, 7, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_mouse() {
    let action = KeyAction::Mouse(1);
    let wire = action.to_config_bytes();
    assert_eq!(wire, [1, 0, 1, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_gamepad() {
    let action = KeyAction::Gamepad(5);
    let wire = action.to_config_bytes();
    assert_eq!(wire, [21, 0, 5, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_fn() {
    let action = KeyAction::Fn;
    let wire = action.to_config_bytes();
    assert_eq!(wire, [10, 1, 0, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    // Fn is factory default at Fn position — detection filter excludes it
    assert!(!is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_disabled() {
    let action = KeyAction::Disabled;
    let wire = action.to_config_bytes();
    assert_eq!(wire, [0, 0, 0, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(!is_user_remap(&wire, caps_default()));
}

#[test]
fn caps_roundtrip_consumer_volume_up() {
    let action = KeyAction::Consumer(0x00E9);
    let wire = action.to_config_bytes();
    assert_eq!(wire, [3, 0, 0xE9, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
    assert_eq!(action.to_string(), "Volume Up");
}

#[test]
fn caps_roundtrip_consumer_calculator() {
    let action = KeyAction::Consumer(0x0192);
    let wire = action.to_config_bytes();
    assert_eq!(wire, [3, 0, 0x92, 0x01]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
    assert_eq!(action.to_string(), "Calculator");
}

#[test]
fn caps_roundtrip_led_control() {
    let action = KeyAction::LedControl { data: [2, 1, 0] };
    let wire = action.to_config_bytes();
    assert_eq!(wire, [13, 2, 1, 0]);
    assert_eq!(KeyAction::from_config_bytes(wire), action);
    assert!(is_user_remap(&wire, caps_default()));
    assert_eq!(action.to_string(), "LED Brightness Up");
}

/// Full simulated matrix scan: assign several different action types to
/// Caps (position 3) and verify remap-list would show each one correctly.
#[test]
fn caps_all_remap_types_in_matrix_scan() {
    let caps_def = caps_default();

    let actions: &[KeyAction] = &[
        KeyAction::Key(HidUsage::new(0x05)), // B
        KeyAction::Combo {
            keys: [HidUsage::new(0xE0), HidUsage::new(0x06), HidUsage::new(0)],
        }, // Ctrl+C
        KeyAction::Macro { index: 0, kind: 0 }, // Macro(0)
        KeyAction::Macro { index: 2, kind: 2 }, // Macro(2,hold)
        KeyAction::Mouse(1),
        KeyAction::Gamepad(3),
        KeyAction::Consumer(0x00E9), // Volume Up
        KeyAction::LedControl { data: [2, 1, 0] },
    ];

    for action in actions {
        let wire = action.to_config_bytes();
        let parsed = KeyAction::from_config_bytes(wire);
        assert_eq!(&parsed, action, "roundtrip failed for {action}");
        assert!(
            is_user_remap(&wire, caps_def),
            "{action} should be detected as remap on Caps"
        );
    }
}

/// Actions that should NOT be detected as remaps on Caps.
#[test]
fn caps_non_remap_types_not_detected() {
    let caps_def = caps_default();

    // Disabled
    assert!(!is_user_remap(
        &KeyAction::Disabled.to_config_bytes(),
        caps_def
    ));
    // Identity (CapsLock → CapsLock)
    assert!(!is_user_remap(
        &KeyAction::Key(HidUsage::new(0x39)).to_config_bytes(),
        caps_def
    ));
    // Fn (filtered as factory default regardless of position)
    assert!(!is_user_remap(&KeyAction::Fn.to_config_bytes(), caps_def));
}

// ════════════════════════════════════════════════════════════════════════════
// Tests for the shared keymap module (Layer, KeyRef, KeyMap)
// ════════════════════════════════════════════════════════════════════════════

// -- shared is_user_remap matches local --

#[test]
fn shared_remap_detection_matches_local() {
    // Verify the shared module's is_user_remap matches our local version
    let cases: &[([u8; 4], u8, bool)] = &[
        ([0, 0, 0, 0], 0x29, false),      // disabled
        ([0, 0, 0x29, 0], 0x29, false),   // identity
        ([0, 0, 0x04, 0], 0x39, true),    // changed
        ([9, 0, 0, 0], 0xE0, true),       // macro
        ([10, 1, 0, 0], 0xE4, false),     // Fn key
        ([0, 0x01, 0x06, 0], 0x39, true), // combo
        ([1, 0, 1, 0], 0xE0, true),       // mouse
    ];
    for &(k, def, expected) in cases {
        assert_eq!(
            shared_is_user_remap(&k, HidUsage::new(def)),
            expected,
            "shared_is_user_remap({k:?}, 0x{def:02x}) should be {expected}"
        );
        assert_eq!(
            is_user_remap(&k, HidUsage::new(def)),
            expected,
            "local is_user_remap({k:?}, 0x{def:02x}) should be {expected}"
        );
    }
}

// -- Layer --

#[test]
fn layer_parse_all_forms() {
    assert_eq!("0".parse::<Layer>(), Ok(Layer::Base));
    assert_eq!("L0".parse::<Layer>(), Ok(Layer::Base));
    assert_eq!("base".parse::<Layer>(), Ok(Layer::Base));
    assert_eq!("1".parse::<Layer>(), Ok(Layer::Layer1));
    assert_eq!("l1".parse::<Layer>(), Ok(Layer::Layer1));
    assert_eq!("L1".parse::<Layer>(), Ok(Layer::Layer1));
    assert_eq!("2".parse::<Layer>(), Ok(Layer::Fn));
    assert_eq!("fn".parse::<Layer>(), Ok(Layer::Fn));
    assert_eq!("FN".parse::<Layer>(), Ok(Layer::Fn));
    assert!("3".parse::<Layer>().is_err());
    assert!("bad".parse::<Layer>().is_err());
}

#[test]
fn layer_display_short() {
    assert_eq!(Layer::Base.to_string(), "L0");
    assert_eq!(Layer::Layer1.to_string(), "L1");
    assert_eq!(Layer::Fn.to_string(), "Fn");
}

// -- KeyRef --

#[test]
fn keyref_parse_bare_key() {
    let kr: KeyRef = "Caps".parse().unwrap();
    assert_eq!(kr.index, MatrixPos::new(3));
    assert_eq!(kr.position, "Caps");
    assert_eq!(kr.layer, Layer::Base);
}

#[test]
fn keyref_parse_fn_prefix() {
    let kr: KeyRef = "Fn+Caps".parse().unwrap();
    assert_eq!(kr.index, MatrixPos::new(3));
    assert_eq!(kr.layer, Layer::Fn);
}

#[test]
fn keyref_parse_l1_prefix() {
    let kr: KeyRef = "L1+A".parse().unwrap();
    assert_eq!(kr.layer, Layer::Layer1);
}

#[test]
fn keyref_parse_numeric() {
    let kr: KeyRef = "42".parse().unwrap();
    assert_eq!(kr.index, MatrixPos::new(42));
    assert_eq!(kr.layer, Layer::Base);
}

#[test]
fn keyref_parse_fn_numeric() {
    let kr: KeyRef = "Fn+42".parse().unwrap();
    assert_eq!(kr.index, MatrixPos::new(42));
    assert_eq!(kr.layer, Layer::Fn);
}

#[test]
fn keyref_display_base() {
    let kr = KeyRef::new(MatrixPos::new(3), Layer::Base);
    assert_eq!(kr.to_string(), "Caps");
}

#[test]
fn keyref_display_fn() {
    let kr = KeyRef::new(MatrixPos::new(3), Layer::Fn);
    assert_eq!(kr.to_string(), "Fn+Caps");
}

#[test]
fn keyref_display_l1() {
    let kr = KeyRef::new(MatrixPos::new(3), Layer::Layer1);
    assert_eq!(kr.to_string(), "L1+Caps");
}

#[test]
fn keyref_roundtrip() {
    // Parse a KeyRef, display it, parse the display back → same
    for s in &["Caps", "Fn+Caps", "L1+Caps", "42", "Fn+42"] {
        let kr: KeyRef = s.parse().unwrap();
        let display = kr.to_string();
        let kr2: KeyRef = display.parse().unwrap();
        assert_eq!(kr, kr2, "roundtrip failed for {s}");
    }
}
