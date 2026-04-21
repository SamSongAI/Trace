//! Keyboard → Win32 translation for the Shortcuts recorder.
//!
//! The Shortcuts card recorder is fed by `iced::event::listen_with`, which
//! requires a plain `fn` pointer (no captures). Keeping the translation
//! routines here — as free functions with no state — makes them trivially
//! usable from that `fn` pointer, and lets the unit tests in this module
//! exercise every mapping branch without spinning up the iced runtime.
//!
//! Two responsibilities live side-by-side:
//!
//! * [`modifiers_to_win32`] projects an [`iced::keyboard::Modifiers`] bitset
//!   onto the Win32 `RegisterHotKey` `MOD_*` bits exposed by
//!   [`trace_core::shortcut_spec`]. The shadow [`trace_core::ShortcutSpec`]
//!   and the on-disk settings both persist the Win32 encoding, so the UI
//!   must normalize to the same bitset before writing the shadow.
//! * [`key_to_vk`] projects an [`iced::keyboard::Key`] onto a Win32
//!   Virtual-Key code. The match covers the set of keys any reasonable
//!   global shortcut uses: letters, digits, function keys, common editing
//!   keys, arrow keys, and the main OEM punctuation on a US layout. Keys
//!   outside the covered set return `None` so the recorder can drop them
//!   without tripping validation.
//!
//! The two functions pair up to produce a [`trace_core::ShortcutSpec`] for
//! [`crate::settings::SettingsMessage::RecordingCaptured`] — see
//! [`keyboard_event_to_message`] in the parent module.

use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use trace_core::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

/// Converts an iced [`Modifiers`] bitset into the Win32 `MOD_*` bitmask used
/// by [`trace_core::ShortcutSpec::modifiers`].
///
/// The mapping is direct:
///
/// | iced helper        | Win32 bit       |
/// |--------------------|-----------------|
/// | [`Modifiers::shift`]   | [`MOD_SHIFT`]   |
/// | [`Modifiers::control`] | [`MOD_CONTROL`] |
/// | [`Modifiers::alt`]     | [`MOD_ALT`]     |
/// | [`Modifiers::logo`]    | [`MOD_WIN`]     |
///
/// Mac users arriving here via `⌘` see their Command key mapped to
/// `MOD_CONTROL` because iced already normalizes the Mac `⌘` to
/// `Modifiers::command`, which surfaces as `control()` on every non-Mac
/// platform. Windows is the canonical target, so the direct bit mapping is
/// correct — the recorder runs on the same machine the hotkey is registered
/// on.
pub(super) fn modifiers_to_win32(mods: Modifiers) -> u32 {
    let mut bits = 0u32;
    if mods.shift() {
        bits |= MOD_SHIFT;
    }
    if mods.control() {
        bits |= MOD_CONTROL;
    }
    if mods.alt() {
        bits |= MOD_ALT;
    }
    if mods.logo() {
        bits |= MOD_WIN;
    }
    bits
}

/// Projects an iced [`Key`] onto a Win32 Virtual-Key code.
///
/// Returns `None` for keys outside the covered set so the event decoder can
/// drop them silently; the recorder only reacts to keys it can round-trip
/// back through `trace_core::ShortcutSpec::display_label` (and its internal
/// `vk_label` helper). The coverage intentionally matches that helper so
/// every captured key renders with a stable text label.
///
/// # Covered
///
/// * [`Key::Character`]: ASCII letters (case-insensitive), digits `0`..`9`,
///   and the main OEM punctuation on a US layout (`-`, `=`, `,`, `.`, `;`,
///   `/`, `` ` ``, `[`, `\`, `]`, `'`).
/// * [`Key::Named`]: `Enter`, `Tab`, `Space`, `Escape`, `Backspace`, `Delete`,
///   `Home`, `End`, `PageUp`, `PageDown`, `ArrowLeft`/`Up`/`Right`/`Down`,
///   and `F1`..`F12`.
///
/// # Not covered
///
/// Named keys outside this set (numpad, media, browser control, ...) return
/// `None`. A global shortcut that needs one of those can still land in the
/// shadow through the existing `AppSettings` defaults — the recorder
/// surface is deliberately narrow so "press anything" doesn't yield an
/// unregistrable chord.
pub(super) fn key_to_vk(key: &Key) -> Option<u32> {
    match key {
        Key::Character(chars) => character_to_vk(chars),
        Key::Named(named) => named_to_vk(*named),
        // iced's `Key` enum has a catch-all `Unidentified` variant on some
        // backends; treat anything that isn't `Character` or `Named` as
        // "no shortcut".
        _ => None,
    }
}

/// Maps a `Key::Character` payload to a Win32 VK code. The payload is
/// always a tiny `SmolStr`; we accept it as `&str` to keep the helper free
/// of the iced types in the test module.
fn character_to_vk(chars: &str) -> Option<u32> {
    // iced delivers `Character` payloads as locale-aware text; a single
    // Unicode scalar is the only shape that maps cleanly to a Win32 VK
    // code in the recorder's narrow domain.
    let mut iter = chars.chars();
    let first = iter.next()?;
    if iter.next().is_some() {
        return None;
    }

    // Lowercase the glyph so Shift+A and plain A alike resolve to VK_A. The
    // modifier bitset captured from [`Modifiers`] records the shift intent
    // separately.
    let lower = first.to_ascii_lowercase();

    if lower.is_ascii_alphabetic() {
        // VK codes for 'A'..='Z' sit at 0x41..=0x5A and match the uppercase
        // ASCII directly — lowercase back to uppercase for the shift.
        return Some(lower.to_ascii_uppercase() as u32);
    }
    if lower.is_ascii_digit() {
        // VK codes for '0'..='9' sit at 0x30..=0x39, identical to ASCII.
        return Some(lower as u32);
    }

    // OEM punctuation on a US layout. The match arms lock the VK codes to
    // the values in `trace-core`'s internal `vk_label` helper so
    // `ShortcutSpec::display_label` round-trips.
    match lower {
        '-' => Some(0xBD),
        '=' => Some(0xBB),
        ',' => Some(0xBC),
        '.' => Some(0xBE),
        ';' => Some(0xBA),
        '/' => Some(0xBF),
        '`' => Some(0xC0),
        '[' => Some(0xDB),
        '\\' => Some(0xDC),
        ']' => Some(0xDD),
        '\'' => Some(0xDE),
        _ => None,
    }
}

/// Maps a `Named` key to a Win32 VK code.
fn named_to_vk(named: Named) -> Option<u32> {
    match named {
        Named::Enter => Some(0x0D),
        Named::Tab => Some(0x09),
        Named::Space => Some(0x20),
        Named::Escape => Some(0x1B),
        Named::Backspace => Some(0x08),
        Named::Delete => Some(0x2E),
        Named::Home => Some(0x24),
        Named::End => Some(0x23),
        Named::PageUp => Some(0x21),
        Named::PageDown => Some(0x22),
        Named::ArrowLeft => Some(0x25),
        Named::ArrowUp => Some(0x26),
        Named::ArrowRight => Some(0x27),
        Named::ArrowDown => Some(0x28),
        Named::F1 => Some(0x70),
        Named::F2 => Some(0x71),
        Named::F3 => Some(0x72),
        Named::F4 => Some(0x73),
        Named::F5 => Some(0x74),
        Named::F6 => Some(0x75),
        Named::F7 => Some(0x76),
        Named::F8 => Some(0x77),
        Named::F9 => Some(0x78),
        Named::F10 => Some(0x79),
        Named::F11 => Some(0x7A),
        Named::F12 => Some(0x7B),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- modifiers_to_win32 --------------------------------------------

    #[test]
    fn modifiers_to_win32_empty_produces_zero() {
        assert_eq!(modifiers_to_win32(Modifiers::empty()), 0);
    }

    #[test]
    fn modifiers_to_win32_maps_each_bit_individually() {
        // Each iced modifier must land on exactly one Win32 bit so the
        // shadow's encoding round-trips back through `vk_label` without
        // drift.
        assert_eq!(modifiers_to_win32(Modifiers::SHIFT), MOD_SHIFT);
        assert_eq!(modifiers_to_win32(Modifiers::CTRL), MOD_CONTROL);
        assert_eq!(modifiers_to_win32(Modifiers::ALT), MOD_ALT);
        assert_eq!(modifiers_to_win32(Modifiers::LOGO), MOD_WIN);
    }

    #[test]
    fn modifiers_to_win32_combines_all_four_bits() {
        let all = Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT | Modifiers::LOGO;
        assert_eq!(
            modifiers_to_win32(all),
            MOD_SHIFT | MOD_CONTROL | MOD_ALT | MOD_WIN
        );
    }

    // --- key_to_vk: Character branch -----------------------------------

    #[test]
    fn key_to_vk_letters_case_insensitive() {
        // Lowercase and uppercase glyphs resolve to the same VK code; the
        // shift intent rides on `Modifiers`, not on the letter case.
        assert_eq!(key_to_vk(&Key::Character("a".into())), Some(0x41));
        assert_eq!(key_to_vk(&Key::Character("A".into())), Some(0x41));
        assert_eq!(key_to_vk(&Key::Character("z".into())), Some(0x5A));
        assert_eq!(key_to_vk(&Key::Character("Z".into())), Some(0x5A));
    }

    #[test]
    fn key_to_vk_digits() {
        for d in '0'..='9' {
            assert_eq!(
                key_to_vk(&Key::Character(d.to_string().into())),
                Some(d as u32),
                "digit {d}"
            );
        }
    }

    #[test]
    fn key_to_vk_oem_punctuation_on_us_layout() {
        // Lock every punctuation arm so a drift between this map and
        // `vk_label` gets caught at test time.
        let cases = [
            ("-", 0xBD),
            ("=", 0xBB),
            (",", 0xBC),
            (".", 0xBE),
            (";", 0xBA),
            ("/", 0xBF),
            ("`", 0xC0),
            ("[", 0xDB),
            ("\\", 0xDC),
            ("]", 0xDD),
            ("'", 0xDE),
        ];
        for (ch, vk) in cases {
            assert_eq!(key_to_vk(&Key::Character(ch.into())), Some(vk), "char {ch}");
        }
    }

    #[test]
    fn key_to_vk_rejects_multi_char_payload() {
        // A dead-key or IME commit can deliver a multi-char payload; the
        // recorder must ignore it rather than coercing the first char into
        // a shortcut.
        assert_eq!(key_to_vk(&Key::Character("ab".into())), None);
    }

    #[test]
    fn key_to_vk_rejects_non_ascii_character() {
        // Non-ASCII glyphs (e.g. CJK, accented letters) have no stable VK
        // mapping in the recorder's US-layout surface.
        assert_eq!(key_to_vk(&Key::Character("中".into())), None);
    }

    // --- key_to_vk: Named branch ---------------------------------------

    #[test]
    fn key_to_vk_named_control_keys() {
        assert_eq!(key_to_vk(&Key::Named(Named::Enter)), Some(0x0D));
        assert_eq!(key_to_vk(&Key::Named(Named::Tab)), Some(0x09));
        assert_eq!(key_to_vk(&Key::Named(Named::Space)), Some(0x20));
        assert_eq!(key_to_vk(&Key::Named(Named::Escape)), Some(0x1B));
        assert_eq!(key_to_vk(&Key::Named(Named::Backspace)), Some(0x08));
        assert_eq!(key_to_vk(&Key::Named(Named::Delete)), Some(0x2E));
        assert_eq!(key_to_vk(&Key::Named(Named::Home)), Some(0x24));
        assert_eq!(key_to_vk(&Key::Named(Named::End)), Some(0x23));
        assert_eq!(key_to_vk(&Key::Named(Named::PageUp)), Some(0x21));
        assert_eq!(key_to_vk(&Key::Named(Named::PageDown)), Some(0x22));
    }

    #[test]
    fn key_to_vk_named_arrow_keys() {
        assert_eq!(key_to_vk(&Key::Named(Named::ArrowLeft)), Some(0x25));
        assert_eq!(key_to_vk(&Key::Named(Named::ArrowUp)), Some(0x26));
        assert_eq!(key_to_vk(&Key::Named(Named::ArrowRight)), Some(0x27));
        assert_eq!(key_to_vk(&Key::Named(Named::ArrowDown)), Some(0x28));
    }

    #[test]
    fn key_to_vk_named_function_keys_f1_through_f12() {
        let expected = [
            (Named::F1, 0x70),
            (Named::F2, 0x71),
            (Named::F3, 0x72),
            (Named::F4, 0x73),
            (Named::F5, 0x74),
            (Named::F6, 0x75),
            (Named::F7, 0x76),
            (Named::F8, 0x77),
            (Named::F9, 0x78),
            (Named::F10, 0x79),
            (Named::F11, 0x7A),
            (Named::F12, 0x7B),
        ];
        for (named, vk) in expected {
            assert_eq!(key_to_vk(&Key::Named(named)), Some(vk), "{named:?}");
        }
    }

    #[test]
    fn key_to_vk_named_out_of_scope_returns_none() {
        // Caps-lock / shift / alt / logo etc. are modifier-style keys; the
        // modifier bits capture them, so the recorder ignores the keyup
        // that carries them as the "key".
        assert_eq!(key_to_vk(&Key::Named(Named::CapsLock)), None);
        assert_eq!(key_to_vk(&Key::Named(Named::Shift)), None);
        assert_eq!(key_to_vk(&Key::Named(Named::Control)), None);
        assert_eq!(key_to_vk(&Key::Named(Named::Alt)), None);
        assert_eq!(key_to_vk(&Key::Named(Named::Super)), None);
        // Numpad-specific names are not part of the recorder's surface.
        assert_eq!(key_to_vk(&Key::Named(Named::NumLock)), None);
    }
}
