//! Rendering for the `keymatrix` listing.
//!
//! Kept pure — `&[KeyRow]` in, `String` out — so the layout is testable without a
//! device. The command handler is only a load and a print.

use crate::key_action::KeyAction;
use crate::keymap::{KeyRow, Layer};
use monsgeek_keyboard::{DksBinding, DksPhase, KeyMode, ModeByte};

/// What to include in the listing.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Layers to show; empty means all of them.
    pub layers: Vec<Layer>,
    /// Matrix positions to show; empty means all of them.
    pub keys: Vec<u8>,
    /// Include keys and slots that are at their factory default.
    pub show_unset: bool,
    /// Append the raw 4-byte config to each row.
    pub raw: bool,
    /// Device travel precision (raw units per mm) for the DKS trigger point.
    pub travel_factor: f32,
}

impl ListOptions {
    fn wants(&self, layer: Layer) -> bool {
        self.layers.is_empty() || self.layers.contains(&layer)
    }
}

/// One printed line: which slot, what it emits, and whether it is set.
struct Slot {
    label: String,
    action: KeyAction,
    set: bool,
    raw: Option<[u8; 4]>,
    note: String,
}

/// Slots to print for one key, in display order.
///
/// In DKS mode the firmware reinterprets keymatrix layers 1–3 as the key's output
/// slots, so they are labelled `W dks#1` rather than `L1+W` — a DKS key has no
/// Layer1 at all.
fn slots_for(row: &KeyRow, opts: &ListOptions) -> Vec<Slot> {
    let mut slots = Vec::new();
    let dks = row.mode == KeyMode::DynamicKeystroke;

    if opts.wants(Layer::Base) {
        slots.push(Slot {
            label: row.position.clone(),
            action: row.outputs[0],
            set: row.output_remapped[0],
            raw: Some(row.raw[0]),
            note: String::new(),
        });
        if dks {
            for n in 1..4 {
                slots.push(Slot {
                    label: format!("{} dks#{n}", row.position),
                    action: row.outputs[n],
                    set: row.output_remapped[n],
                    raw: Some(row.raw[n]),
                    note: dks_phase_note(row.dks_modes[n], row.outputs[n]),
                });
            }
        }
    }
    if !dks && opts.wants(Layer::Layer1) {
        slots.push(Slot {
            label: format!("L1+{}", row.position),
            action: row.outputs[1],
            set: row.output_remapped[1],
            raw: Some(row.raw[1]),
            note: String::new(),
        });
    }
    if opts.wants(Layer::Fn) {
        slots.push(Slot {
            label: format!("Fn+{}", row.position),
            action: row.fn_action.unwrap_or(KeyAction::Disabled),
            set: row.fn_action.is_some(),
            raw: row.fn_raw,
            note: String::new(),
        });
    }
    slots
}

/// Which travel phases fire this DKS slot, or a warning when none do.
fn dks_phase_note(packed: u8, action: KeyAction) -> String {
    if action == KeyAction::Disabled {
        return String::new();
    }
    if packed == 0 {
        // The slot has an output but no phase ever triggers it.
        return "(inactive)".to_string();
    }
    let actions = DksBinding::unpack_phase_actions(packed);
    DksPhase::ALL
        .iter()
        .filter(|p| actions[p.index()] != Default::default())
        .map(|p| format!("{}:{}", p.short_label(), actions[p.index()]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Trailing detail for a key's header: its mode, and the DKS trigger point.
fn header_note(row: &KeyRow, opts: &ListOptions) -> String {
    let mut parts = Vec::new();
    if row.mode != KeyMode::Normal || row.rapid_trigger {
        parts.push(ModeByte::new(row.mode, row.rapid_trigger).to_string());
    }
    if row.mode == KeyMode::DynamicKeystroke && opts.travel_factor > 0.0 {
        parts.push(format!(
            "↧{:.2}mm",
            row.dks_travel as f32 / opts.travel_factor
        ));
    }
    parts.join("  ")
}

/// Render the listing.
pub fn render(rows: &[KeyRow], opts: &ListOptions) -> String {
    let selected: Vec<(&KeyRow, Vec<Slot>)> = rows
        .iter()
        .filter(|r| opts.keys.is_empty() || opts.keys.contains(&r.index))
        .filter(|r| opts.show_unset || r.is_customized())
        .map(|r| {
            let mut slots = slots_for(r, opts);
            if !opts.show_unset {
                slots.retain(|s| s.set);
            }
            (r, slots)
        })
        .filter(|(_, slots)| !slots.is_empty())
        .collect();

    if selected.is_empty() {
        return "No customised keys found. Use --unset to show factory defaults.\n".to_string();
    }

    // One column width across the whole listing, from the labels actually printed.
    let width = selected
        .iter()
        .flat_map(|(_, slots)| slots.iter())
        .map(|s| s.label.chars().count())
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    for (row, slots) in &selected {
        let note = header_note(row, opts);
        out.push_str(&format!("{} ({})", row.position, row.index));
        if !note.is_empty() {
            out.push_str(&format!("   {note}"));
        }
        out.push('\n');

        for slot in slots {
            let mark = if slot.set { '*' } else { ' ' };
            let action = if slot.action == KeyAction::Disabled {
                "·".to_string()
            } else {
                slot.action.to_string()
            };
            out.push_str(&format!(
                "  {mark} {:width$}  {action}",
                slot.label,
                width = width
            ));
            if !slot.note.is_empty() {
                out.push_str(&format!("   {}", slot.note));
            }
            if opts.raw {
                if let Some(b) = slot.raw {
                    out.push_str(&format!(
                        "   [{:02x} {:02x} {:02x} {:02x}]",
                        b[0], b[1], b[2], b[3]
                    ));
                }
            }
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ListOptions {
        ListOptions {
            travel_factor: 100.0,
            ..Default::default()
        }
    }

    /// A key with nothing set, for building fixtures.
    fn row(index: u8, position: &str) -> KeyRow {
        KeyRow {
            index,
            position: position.to_string(),
            mode: KeyMode::Normal,
            rapid_trigger: false,
            actuation: 200,
            release: 200,
            rt_press: 30,
            rt_lift: 30,
            bottom_dz: 0,
            top_dz: 0,
            outputs: [KeyAction::Disabled; 4],
            output_remapped: [false; 4],
            raw: [[0; 4]; 4],
            fn_raw: None,
            fn_action: None,
            dks_travel: 0,
            dks_modes: [0; 4],
            modtap_ms: 0,
            snaptap_partner: None,
        }
    }

    fn caps_remapped() -> KeyRow {
        let mut r = row(3, "Caps");
        r.outputs[0] = KeyAction::Key(0x29);
        r.output_remapped[0] = true;
        r.raw[0] = [0, 0, 0x29, 0];
        r
    }

    #[test]
    fn untouched_keys_are_hidden_by_default() {
        let out = render(&[row(3, "Caps")], &opts());
        assert!(out.starts_with("No customised keys found"), "{out}");
    }

    #[test]
    fn a_remapped_key_shows_one_row() {
        let out = render(&[caps_remapped()], &opts());
        assert!(out.contains("Caps (3)"), "{out}");
        assert!(out.contains("* Caps"), "{out}");
        assert!(out.contains("Escape"), "{out}");
        assert!(!out.contains("L1+Caps"), "{out}");
    }

    /// Base then Fn, in that order, both marked as set.
    #[test]
    fn base_and_fn_bindings_each_get_a_row() {
        let mut r = caps_remapped();
        r.fn_action = Some(KeyAction::Consumer(0x00E9));
        r.fn_raw = Some([3, 0, 0xE9, 0]);
        let out = render(&[r], &opts());
        let base = out.find("* Caps").expect("base row");
        let fnr = out.find("* Fn+Caps").expect("fn row");
        assert!(base < fnr, "base must precede Fn:\n{out}");
        assert!(out.contains("Volume Up"), "{out}");
    }

    #[test]
    fn unset_reveals_the_empty_overlay_slot() {
        let mut o = opts();
        o.show_unset = true;
        let out = render(&[caps_remapped()], &o);
        assert!(out.contains("L1+Caps"), "{out}");
        // Empty overlay slots read as transparent, not as a binding.
        assert!(out.contains('·'), "{out}");
    }

    #[test]
    fn layer_filter_selects_which_rows_appear() {
        let mut r = caps_remapped();
        r.fn_action = Some(KeyAction::Fn);
        r.fn_raw = Some([10, 1, 0, 0]);
        let mut o = opts();
        o.layers = vec![Layer::Fn];
        let out = render(&[r], &o);
        assert!(out.contains("Fn+Caps"), "{out}");
        assert!(
            !out.contains("* Caps "),
            "base row should be filtered out:\n{out}"
        );
    }

    #[test]
    fn key_filter_selects_which_keys_appear() {
        let mut w = row(14, "W");
        w.outputs[0] = KeyAction::Key(0x1A);
        w.output_remapped[0] = true;
        let mut o = opts();
        o.keys = vec![3];
        let out = render(&[caps_remapped(), w], &o);
        assert!(out.contains("Caps (3)"), "{out}");
        assert!(!out.contains("W (14)"), "{out}");
    }

    /// DKS reinterprets layers 1-3 as output slots, so they are never `L1+`.
    #[test]
    fn dks_slots_are_labelled_by_slot_not_layer() {
        let mut r = row(14, "W");
        r.mode = KeyMode::DynamicKeystroke;
        r.dks_travel = 120;
        r.outputs[0] = KeyAction::Key(0x1A);
        r.output_remapped[0] = true;
        r.outputs[1] = KeyAction::Key(0x06);
        r.output_remapped[1] = true;
        r.dks_modes[1] = 1;
        let out = render(&[r], &opts());
        assert!(out.contains("W dks#1"), "{out}");
        assert!(!out.contains("L1+W"), "{out}");
        assert!(out.contains("↧1.20mm"), "{out}");
    }

    /// A slot with an output but no phase set never fires.
    #[test]
    fn dks_slot_without_a_phase_is_flagged_inactive() {
        let mut r = row(14, "W");
        r.mode = KeyMode::DynamicKeystroke;
        r.outputs[2] = KeyAction::Key(0x06);
        r.output_remapped[2] = true;
        r.dks_modes[2] = 0;
        let out = render(&[r], &opts());
        assert!(out.contains("(inactive)"), "{out}");
    }

    /// `--raw` must print what the device holds. Re-encoding would be lossy:
    /// `[0,0x29,0,0]` and `[0,0,0x29,0]` both decode to `Key(0x29)`.
    #[test]
    fn raw_prints_device_bytes_not_a_re_encoding() {
        let mut r = row(3, "Caps");
        r.outputs[0] = KeyAction::Key(0x29);
        r.output_remapped[0] = true;
        r.raw[0] = [0, 0x29, 0, 0];
        let mut o = opts();
        o.raw = true;
        let encoded = r.outputs[0].to_config_bytes();
        let out = render(&[r], &o);
        assert!(out.contains("[00 29 00 00]"), "{out}");
        assert_eq!(encoded, [0, 0, 0x29, 0], "re-encoding moves the usage slot");
    }

    #[test]
    fn golden_layout() {
        let mut caps = caps_remapped();
        caps.fn_action = Some(KeyAction::Consumer(0x00E9));
        caps.fn_raw = Some([3, 0, 0xE9, 0]);
        let mut w = row(14, "W");
        w.outputs[0] = KeyAction::Key(0x1A);
        w.output_remapped[0] = true;
        let out = render(&[caps, w], &opts());
        assert_eq!(
            out,
            concat!(
                "Caps (3)\n",
                "  * Caps     Escape\n",
                "  * Fn+Caps  Volume Up\n",
                "\n",
                "W (14)\n",
                "  * W        W\n",
                "\n",
            ),
            "\n--- actual ---\n{out}"
        );
    }
}
