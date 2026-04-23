//! Capture-panel header: brand wordmark on the left, Pin + Settings buttons
//! on the right.
//!
//! Matches Mac `CaptureView.swift:90-123` — 36 px tall, 16 px horizontal
//! padding, chrome background, 8 px spacing between the wordmark and the
//! button cluster.
//!
//! Icons are rendered as unicode glyphs: pin uses U+1F4CC ("pushpin") to
//! approximate the Mac SF Symbol `pin`, and settings uses U+2699 ("gear").
//! Unicode has no filled-pushpin counterpart to SF Symbol `pin.fill`, so the
//! Mac's weight swap is replaced by an accent-color swap driven by the
//! `pinned` flag (see [`crate::theme::header_icon_button_style`]). Phase 11
//! will swap in bitmap icons if the glyph coverage is insufficient on
//! Windows defaults.

use iced::widget::{button, container, mouse_area, row, text, Space};
use iced::{Length, Pixels};
use trace_core::CapturePalette;

use crate::app::{Message, HEADER_HEIGHT};
use crate::fonts::LORA_FONT;
use crate::theme::{chrome_container_style, header_icon_button_style};

/// Brand wordmark displayed on the left side of the header.
pub const BRAND_TEXT: &str = "Trace";
/// Glyph used for the Pin button regardless of state. The pinned vs. idle
/// distinction is carried by the button's text color, not by swapping glyphs
/// (Unicode has no filled-pushpin sibling to U+1F4CC).
pub const PIN_GLYPH: &str = "\u{1F4CC}";
/// Gear glyph used for the Settings button.
pub const SETTINGS_GLYPH: &str = "\u{2699}";

/// Builds the 36 px header row.
///
/// `pinned` controls the pin button's text color (accent vs. icon-muted) so
/// it can look distinct from the idle state without changing the glyph.
pub fn header<'a>(palette: CapturePalette, pinned: bool) -> iced::Element<'a, Message> {
    let brand = text(BRAND_TEXT).font(LORA_FONT).size(Pixels(13.0));

    let pin_button = button(text(PIN_GLYPH).size(Pixels(11.0)))
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

    let chrome = container(layout)
        .padding([0, 16])
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT))
        .align_y(iced::alignment::Vertical::Center)
        .style(chrome_container_style(palette));

    // Wrap the whole header in a `mouse_area` so a left-press on empty
    // chrome initiates the OS-level window drag loop. The Pin and
    // Settings buttons inside `layout` capture their own press events
    // first (iced propagates events child → parent and `mouse_area`
    // short-circuits via `shell.is_event_captured()`), so clicking a
    // button still fires its own `on_press` instead of starting a drag.
    //
    // Why this is needed: the capture panel is created with
    // `decorations: false` — Windows gives it no native title bar for
    // the user to grab, so without this wrapper there is literally no
    // way to move the window (reported in v0.2.2 as
    // "产品界面也无法拖动"). The 36 px header is the one conventional
    // spot to put a drag handle, matching the Mac port's use of
    // `NSWindow.isMovableByWindowBackground = true` over the header
    // area.
    mouse_area(chrome)
        .on_press(Message::WindowDragRequested)
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
