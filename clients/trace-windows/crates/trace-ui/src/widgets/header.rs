//! Capture-panel header: brand wordmark on the left, Pin + Settings buttons
//! on the right.
//!
//! Matches Mac `CaptureView.swift:90-123` — 36 px tall, 16 px horizontal
//! padding, chrome background, 8 px spacing between the wordmark and the
//! button cluster.
//!
//! Icons are rendered as unicode glyphs: pin uses U+1F4CC ("pushpin") to match
//! the Mac SF Symbol `pin` / `pin.fill`, and settings uses U+2699 ("gear").
//! Phase 10 keeps the icons purely decorative; Phase 11 will swap in bitmap
//! icons if the glyph coverage is insufficient on Windows defaults.

use iced::widget::{button, container, row, text, Space};
use iced::{Length, Pixels};
use trace_core::CapturePalette;

use crate::app::{Message, HEADER_HEIGHT};
use crate::fonts::LORA_FONT;
use crate::theme::{chrome_container_style, header_icon_button_style};

/// Brand wordmark displayed on the left side of the header.
pub const BRAND_TEXT: &str = "Trace";
/// Glyph used for the Pin button when the panel is not pinned. Maps to the
/// Mac SF Symbol `pin`.
pub const PIN_GLYPH: &str = "\u{1F4CC}";
/// Glyph used when the panel is pinned. Mac uses `pin.fill`; we reuse the
/// same codepoint and rely on weight/color to signal the state.
pub const PIN_FILL_GLYPH: &str = "\u{1F4CC}";
/// Gear glyph used for the Settings button.
pub const SETTINGS_GLYPH: &str = "\u{2699}";

/// Builds the 36 px header row.
///
/// `pinned` toggles the pin button's icon color and active state so it can
/// look distinct from the idle icon-muted shade.
pub fn header<'a>(palette: CapturePalette, pinned: bool) -> iced::Element<'a, Message> {
    let pin_icon = if pinned { PIN_FILL_GLYPH } else { PIN_GLYPH };

    let brand = text(BRAND_TEXT).font(LORA_FONT).size(Pixels(13.0));

    let pin_button = button(text(pin_icon).size(Pixels(11.0)))
        .on_press(Message::PinToggled)
        .style(header_icon_button_style(palette, pinned));

    let settings_button = button(text(SETTINGS_GLYPH).size(Pixels(11.0)))
        .on_press(Message::SettingsRequested)
        .style(header_icon_button_style(palette, false));

    let layout = row![
        brand,
        Space::new().width(Length::Fill),
        pin_button,
        settings_button,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    container(layout)
        .padding([0, 16])
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT))
        .align_y(iced::alignment::Vertical::Center)
        .style(chrome_container_style(palette))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    #[test]
    fn header_constructs_for_both_pin_states() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        // Construct both states — this proves the widget tree type-checks for
        // both branches. Without running the iced event loop we can't assert
        // on rendered pixels; the `view()` smoke test in `app.rs` exercises
        // the integration.
        let _idle: iced::Element<'_, Message> = header(palette, false);
        let _pinned: iced::Element<'_, Message> = header(palette, true);
    }

    #[test]
    fn brand_text_is_stable() {
        // The wordmark is user-visible and referenced from the Mac reference,
        // so lock it in place.
        assert_eq!(BRAND_TEXT, "Trace");
    }

    #[test]
    fn pin_and_settings_glyphs_are_distinct() {
        assert_ne!(PIN_GLYPH, SETTINGS_GLYPH);
    }
}
