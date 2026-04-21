//! Keyboard shortcut specification for the Windows client.
//!
//! Mirrors Mac `Sources/Trace/Utils/KeyboardShortcut.swift`, but swaps Carbon
//! `kVK_*` codes for Win32 Virtual-Key codes and Carbon modifier masks
//! (`cmdKey`, `shiftKey`, …) for Win32 `RegisterHotKey` modifier bits
//! (`MOD_ALT`, `MOD_CONTROL`, `MOD_SHIFT`, `MOD_WIN`).
//!
//! The display label uses plain-text Windows conventions (`Ctrl+Shift+N`)
//! rather than the SF Symbols glyphs (`⌃⇧N`) the Mac reference emits — iced
//! 0.14 has no SF Symbols equivalent, and Windows users expect the text
//! spelling in every OS-level shortcut UI.
//!
//! `ShortcutSpec` deliberately does **not** derive `Serialize` /
//! `Deserialize`: persistence stores the VK code and modifier mask as two
//! independent `u32` fields on `AppSettings`, not a struct payload.
//! `ShortcutSpec` is an in-memory UI shadow only.

/// `MOD_ALT` bit in the `modifiers: u32` on-disk storage. Matches Win32
/// `RegisterHotKey` `MOD_ALT = 0x0001`. Exposed so the UI layer can compose
/// bitmasks without hard-coding the hex literal.
pub const MOD_ALT: u32 = 0x0001;
/// `MOD_CONTROL` bit. Matches Win32 `MOD_CONTROL = 0x0002`.
pub const MOD_CONTROL: u32 = 0x0002;
/// `MOD_SHIFT` bit. Matches Win32 `MOD_SHIFT = 0x0004`.
pub const MOD_SHIFT: u32 = 0x0004;
/// `MOD_WIN` bit. Matches Win32 `MOD_WIN = 0x0008`. The logical Windows-logo
/// modifier; macOS `⌘` maps here only when a user coming from macOS has
/// remapped the Windows key.
pub const MOD_WIN: u32 = 0x0008;
/// Reserved modifier used to detect the `Ctrl+1`..`Ctrl+9` panel-section
/// switch shortcut. On Mac this is `⌘` (command); on Windows the command-key
/// conventions for section switching collapse to `Ctrl`. Exposed as a
/// symbolic constant so the UI layer does not re-state the mapping.
pub const RESERVED_SECTION_MOD: u32 = MOD_CONTROL;

const ALL_MODIFIERS: u32 = MOD_ALT | MOD_CONTROL | MOD_SHIFT | MOD_WIN;

/// A keyboard shortcut spec: a virtual-key code plus a Win32 modifier
/// bitmask. The `modifiers` field uses the `MOD_*` bit conventions defined
/// in this module so both on-disk persistence and runtime `RegisterHotKey`
/// calls can share one representation.
///
/// Mirrors Mac `KeyboardShortcut` (`Sources/Trace/Utils/KeyboardShortcut.swift`)
/// but re-encoded for Windows. The struct itself is `Copy` because it holds
/// only two `u32`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    /// Win32 Virtual-Key code. `VK_RETURN = 0x0D`, `VK_TAB = 0x09`, letter
    /// keys `A..Z` at `0x41..0x5A`, digits `0..9` at `0x30..0x39`, and so on.
    pub key_code: u32,
    /// Win32 `MOD_*` bit mask. Zero means "no modifier". Values are bitwise
    /// combinations of [`MOD_ALT`] / [`MOD_CONTROL`] / [`MOD_SHIFT`] /
    /// [`MOD_WIN`].
    pub modifiers: u32,
}

impl ShortcutSpec {
    /// Constructs a fresh [`ShortcutSpec`].
    ///
    /// The helper exists so callers don't need to write struct-literal syntax
    /// when composing a shortcut inline from an event callback.
    pub fn new(key_code: u32, modifiers: u32) -> Self {
        Self {
            key_code,
            modifiers,
        }
    }

    /// Renders the shortcut as a Windows-conventional plain-text label such as
    /// `"Ctrl+Shift+N"`.
    ///
    /// Modifier order is fixed (`Ctrl` → `Alt` → `Shift` → `Win`) regardless
    /// of the order the caller set the bits — matching the order used in the
    /// majority of Windows shortcut surfaces and in Microsoft's own
    /// documentation. The key label comes from [`vk_label`]; unknown VK codes
    /// produce a `"Key-<decimal>"` fallback rather than an empty string so
    /// debugging is possible even if the map grows stale.
    pub fn display_label(&self) -> String {
        let mut label = String::new();
        if self.modifiers & MOD_CONTROL != 0 {
            label.push_str("Ctrl+");
        }
        if self.modifiers & MOD_ALT != 0 {
            label.push_str("Alt+");
        }
        if self.modifiers & MOD_SHIFT != 0 {
            label.push_str("Shift+");
        }
        if self.modifiers & MOD_WIN != 0 {
            label.push_str("Win+");
        }
        label.push_str(&vk_label(self.key_code));
        label
    }

    /// Returns `true` if any of the four recognized Win32 modifier bits are
    /// set. Used by the recorder UI to enforce "shortcut must include at
    /// least one modifier key" before writing the shadow.
    pub fn has_modifier(&self) -> bool {
        self.modifiers & ALL_MODIFIERS != 0
    }

    /// Returns `true` when this shortcut collides with the built-in
    /// `Ctrl+1`..`Ctrl+9` panel-section switch shortcut. Panel shortcuts
    /// (send / append / mode-toggle) must reject these combinations so the
    /// section switch stays reachable no matter how the user configures
    /// their other shortcuts. Mirrors Mac's `⌘1-9` guard on panel shortcuts.
    ///
    /// The check is strict: only `Ctrl + 1..=9` counts. `Ctrl+0` does not
    /// conflict because Mac's reservation covers `⌘1`..`⌘9` only.
    pub fn is_reserved_section_switch(&self) -> bool {
        self.modifiers & RESERVED_SECTION_MOD != 0 && (0x31..=0x39).contains(&self.key_code)
    }
}

/// Maps a Win32 Virtual-Key code to its human-readable text label.
///
/// The map covers the set of keys any reasonable global shortcut uses: letter
/// keys, digits, function keys, common editing keys (Enter/Tab/Esc/etc.),
/// arrow keys, and the main OEM punctuation keys on a US layout. Keys
/// outside this coverage fall through to `"Key-<decimal>"` so a future
/// keyboard layout with atypical virtual-keys still produces a
/// human-readable label instead of an empty string.
///
/// The US-layout assumption matches the Mac reference's `keyCodeLabels`
/// map and is documented there as well. A future sub-task that wants to
/// localize the punctuation labels can swap this helper for a locale-aware
/// one without breaking callers because the signature is `&str`-free.
pub fn vk_label(key_code: u32) -> String {
    match key_code {
        // Letters: VK codes 0x41..=0x5A map 1:1 onto ASCII 'A'..='Z'.
        0x41..=0x5A => ((key_code as u8) as char).to_string(),
        // Digits: VK codes 0x30..=0x39 map 1:1 onto ASCII '0'..='9'.
        0x30..=0x39 => ((key_code as u8) as char).to_string(),
        // Control / editing keys
        0x0D => "Enter".to_string(),
        0x09 => "Tab".to_string(),
        0x20 => "Space".to_string(),
        0x1B => "Esc".to_string(),
        0x08 => "Backspace".to_string(),
        0x2E => "Delete".to_string(),
        0x24 => "Home".to_string(),
        0x23 => "End".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        // Function keys: `VK_F1..VK_F12 = 0x70..=0x7B`.
        0x70..=0x7B => format!("F{}", key_code - 0x70 + 1),
        // OEM punctuation (US layout)
        0xBD => "-".to_string(),
        0xBB => "=".to_string(),
        0xBC => ",".to_string(),
        0xBE => ".".to_string(),
        0xBA => ";".to_string(),
        0xBF => "/".to_string(),
        0xC0 => "`".to_string(),
        0xDB => "[".to_string(),
        0xDC => "\\".to_string(),
        0xDD => "]".to_string(),
        0xDE => "'".to_string(),
        other => format!("Key-{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- display_label --------------------------------------------------

    #[test]
    fn display_label_returns_key_only_when_no_modifier() {
        let spec = ShortcutSpec::new(0x4E, 0); // VK_N, no modifiers.
        assert_eq!(spec.display_label(), "N");
    }

    #[test]
    fn display_label_ctrl_plus_letter() {
        let spec = ShortcutSpec::new(0x4E, MOD_CONTROL);
        assert_eq!(spec.display_label(), "Ctrl+N");
    }

    #[test]
    fn display_label_ctrl_shift_plus_enter() {
        let spec = ShortcutSpec::new(0x0D, MOD_CONTROL | MOD_SHIFT);
        assert_eq!(spec.display_label(), "Ctrl+Shift+Enter");
    }

    #[test]
    fn display_label_orders_all_four_modifiers_before_key() {
        // Regardless of how the caller composes the bits, the rendered order
        // must be Ctrl → Alt → Shift → Win → key so a drift in the match
        // arms is caught at test time.
        let spec = ShortcutSpec::new(0x41, MOD_SHIFT | MOD_WIN | MOD_ALT | MOD_CONTROL);
        assert_eq!(spec.display_label(), "Ctrl+Alt+Shift+Win+A");
    }

    #[test]
    fn display_label_unknown_key_code_falls_back_to_numeric() {
        // 0x99 is a VK code with no named mapping; the fallback branch must
        // produce a human-readable label rather than an empty string.
        let spec = ShortcutSpec::new(0x99, 0);
        assert_eq!(spec.display_label(), "Key-153");
    }

    // --- has_modifier ---------------------------------------------------

    #[test]
    fn has_modifier_is_false_for_zero_modifiers() {
        assert!(!ShortcutSpec::new(0x41, 0).has_modifier());
    }

    #[test]
    fn has_modifier_detects_shift() {
        assert!(ShortcutSpec::new(0x41, MOD_SHIFT).has_modifier());
    }

    #[test]
    fn has_modifier_detects_every_single_modifier() {
        // Guard each arm of the bit-union check so no modifier is silently
        // dropped by a future refactor.
        for m in [MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN] {
            assert!(
                ShortcutSpec::new(0x41, m).has_modifier(),
                "modifier {m:#06x} must register as a modifier"
            );
        }
    }

    // --- is_reserved_section_switch ------------------------------------

    #[test]
    fn is_reserved_section_switch_true_for_ctrl_digit_1_through_9() {
        for digit in 0x31..=0x39 {
            assert!(
                ShortcutSpec::new(digit, MOD_CONTROL).is_reserved_section_switch(),
                "Ctrl+{digit:#04x} must be reserved"
            );
        }
    }

    #[test]
    fn is_reserved_section_switch_false_for_ctrl_digit_0() {
        // Mac reserves `⌘1-9` only; `⌘0` is not part of the reservation.
        assert!(!ShortcutSpec::new(0x30, MOD_CONTROL).is_reserved_section_switch());
    }

    #[test]
    fn is_reserved_section_switch_false_for_shift_digit_without_ctrl() {
        // Same digit but missing the reserved modifier bit; not a conflict.
        assert!(!ShortcutSpec::new(0x31, MOD_SHIFT).is_reserved_section_switch());
    }

    #[test]
    fn is_reserved_section_switch_false_for_ctrl_plus_non_digit() {
        assert!(!ShortcutSpec::new(0x41, MOD_CONTROL).is_reserved_section_switch());
    }

    // --- vk_label coverage ----------------------------------------------

    #[test]
    fn vk_label_covers_letter_digit_and_named_branches() {
        assert_eq!(vk_label(0x41), "A");
        assert_eq!(vk_label(0x5A), "Z");
        assert_eq!(vk_label(0x30), "0");
        assert_eq!(vk_label(0x39), "9");
        assert_eq!(vk_label(0x0D), "Enter");
        assert_eq!(vk_label(0x09), "Tab");
        assert_eq!(vk_label(0x20), "Space");
        assert_eq!(vk_label(0x1B), "Esc");
        assert_eq!(vk_label(0x08), "Backspace");
        assert_eq!(vk_label(0x2E), "Delete");
        assert_eq!(vk_label(0x24), "Home");
        assert_eq!(vk_label(0x23), "End");
        assert_eq!(vk_label(0x21), "PageUp");
        assert_eq!(vk_label(0x22), "PageDown");
        assert_eq!(vk_label(0x25), "Left");
        assert_eq!(vk_label(0x26), "Up");
        assert_eq!(vk_label(0x27), "Right");
        assert_eq!(vk_label(0x28), "Down");
    }

    #[test]
    fn vk_label_covers_every_function_key() {
        for (i, vk) in (0x70..=0x7B).enumerate() {
            assert_eq!(vk_label(vk), format!("F{}", i + 1));
        }
    }

    #[test]
    fn vk_label_covers_oem_punctuation() {
        assert_eq!(vk_label(0xBD), "-");
        assert_eq!(vk_label(0xBB), "=");
        assert_eq!(vk_label(0xBC), ",");
        assert_eq!(vk_label(0xBE), ".");
        assert_eq!(vk_label(0xBA), ";");
        assert_eq!(vk_label(0xBF), "/");
        assert_eq!(vk_label(0xC0), "`");
        assert_eq!(vk_label(0xDB), "[");
        assert_eq!(vk_label(0xDC), "\\");
        assert_eq!(vk_label(0xDD), "]");
        assert_eq!(vk_label(0xDE), "'");
    }

    #[test]
    fn vk_label_fallback_uses_decimal_suffix() {
        // Guard the fallback branch so unfamiliar VK codes still surface a
        // debuggable label instead of an empty string.
        assert_eq!(vk_label(0x99), "Key-153");
    }

    // --- modifier constants lock ----------------------------------------

    #[test]
    fn modifier_constants_match_win32_register_hotkey_values() {
        // Lock the `MOD_*` bit values to their Win32 RegisterHotKey
        // contract; a silent drift here would misalign the on-disk
        // `modifiers: u32` encoding with both the runtime hotkey registration
        // and the Mac-derived defaults in `trace-core::settings`.
        assert_eq!(MOD_ALT, 0x0001);
        assert_eq!(MOD_CONTROL, 0x0002);
        assert_eq!(MOD_SHIFT, 0x0004);
        assert_eq!(MOD_WIN, 0x0008);
        assert_eq!(RESERVED_SECTION_MOD, MOD_CONTROL);
    }
}
