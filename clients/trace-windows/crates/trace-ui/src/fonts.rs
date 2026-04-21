//! Font assets bundled with the capture panel.
//!
//! Mac reference uses the Lora typeface for the brand wordmark and the editor
//! body text (`CaptureView.swift:93`, `CaptureTextEditor.swift`). Windows has
//! no guarantee Lora is installed system-wide, so the long-term plan is to
//! `include_bytes!` a bundled TTF.
//!
//! # Phase 10 status
//!
//! At the time of this crate's scaffolding the Mac project did not ship a
//! standalone Lora font file — the Mac build resolves `Font.custom("Lora", …)`
//! against user- or system-installed copies. There is therefore nothing to
//! physically copy into `clients/trace-windows/assets/fonts/` yet.
//!
//! [`LORA_FONT_BYTES`] is consequently [`None`]. The iced application loads
//! [`LORA_FONT`] by family name so Windows hosts with Lora installed pick it
//! up automatically; hosts without it fall back to the iced default font.
//!
//! TODO(phase-10-fonts): download Lora Regular and Bold from Google Fonts,
//! drop the files under `clients/trace-windows/assets/fonts/`, and switch
//! `LORA_FONT_BYTES` to `Some(include_bytes!("../../../assets/fonts/Lora-Regular.ttf"))`.
//! Also add a Bold weight constant once the asset lands.

use iced::Font;

/// Canonical font handle for the Trace wordmark and editor body text.
///
/// Resolved by family name. If no Lora face is installed on the host and no
/// bytes have been bundled yet (see [`LORA_FONT_BYTES`]), iced falls back to
/// its default sans-serif font.
pub const LORA_FONT: Font = Font::with_name("Lora");

/// Bundled Lora TTF bytes, loaded at app startup via
/// `iced::application(...).font(...)`.
///
/// [`None`] until the Phase 10 font asset is checked in. See module docs.
pub const LORA_FONT_BYTES: Option<&[u8]> = None;

/// Returns the list of font byte blobs that `app::run()` should register with
/// iced at startup. Callers splat this into the `application(...)` builder:
///
/// ```ignore
/// let mut app = iced::application(...);
/// for bytes in trace_ui::fonts::startup_font_bytes() {
///     app = app.font(bytes);
/// }
/// ```
pub fn startup_font_bytes() -> Vec<&'static [u8]> {
    match LORA_FONT_BYTES {
        Some(bytes) => vec![bytes],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::font::Family;

    #[test]
    fn lora_font_uses_named_family() {
        match LORA_FONT.family {
            Family::Name(name) => assert_eq!(name, "Lora"),
            other => panic!("expected named Lora family, got {:?}", other),
        }
    }

    #[test]
    fn startup_font_bytes_is_empty_when_unbundled() {
        // Guard test: once Phase 10 actually bundles Lora, update this test
        // to assert the bytes are loaded.
        if LORA_FONT_BYTES.is_none() {
            assert!(startup_font_bytes().is_empty());
        } else {
            assert_eq!(startup_font_bytes().len(), 1);
        }
    }
}
