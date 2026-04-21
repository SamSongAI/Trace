//! Transient toast pill overlay.
//!
//! Matches Mac `CaptureView.swift`: a rounded horizontal pill with an icon
//! glyph and a short message, anchored to the bottom of the capture panel.
//! Phase 11 uses a single-line layout; the Mac reference adds an animated
//! entry/exit curve which will land later once iced 0.14's `Animation` API
//! is wired from `CaptureApp`.
//!
//! The widget is pure: it takes a palette and a string and returns an
//! `Element`. Dismissal is orchestrated from [`crate::app`] via the
//! `ToastDismiss` message, which the toast does not dispatch — the timer
//! lives on [`crate::app::subscription`].

use iced::alignment::Horizontal;
use iced::widget::{container, row, text, Space};
use iced::{Length, Pixels};
use trace_core::CapturePalette;

use crate::app::Message;
use crate::fonts::LORA_FONT;
use crate::theme::toast_container_style;

/// Horizontal padding baked into the toast pill. 16 px matches the Mac's
/// `CaptureView` inset.
pub const TOAST_HORIZONTAL_PADDING: u16 = 16;
/// Vertical padding baked into the toast pill.
pub const TOAST_VERTICAL_PADDING: u16 = 10;
/// Bottom-anchor offset so the pill sits above the footer rather than
/// overlapping it.
pub const TOAST_BOTTOM_OFFSET: u16 = 72;
/// Font size for the toast body text. One point smaller than the editor so
/// the message reads as a secondary UI element.
pub const TOAST_FONT_SIZE: f32 = 13.0;

/// Builds the toast pill overlay that sits on top of the capture panel.
///
/// The outer `container` fills the available space so the pill can be
/// positioned near the bottom via `align_*`. The inner `container` carries
/// the styled pill itself.
pub fn toast<'a>(palette: CapturePalette, message: &'a str) -> iced::Element<'a, Message> {
    let label = text(message).font(LORA_FONT).size(Pixels(TOAST_FONT_SIZE));

    let pill = container(row![label].spacing(8))
        .padding([TOAST_VERTICAL_PADDING, TOAST_HORIZONTAL_PADDING])
        .style(toast_container_style(palette));

    // Anchor the pill near the bottom-center. Column layout inside the
    // outer container: a growing spacer pushes the pill down, then a fixed
    // bottom gap sits below it.
    let layout = iced::widget::column![
        Space::new().width(Length::Fill).height(Length::Fill),
        container(pill)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(TOAST_BOTTOM_OFFSET as f32)),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    #[test]
    fn toast_constructs_with_nonempty_message() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let _element: iced::Element<'_, Message> = toast(palette, "空内容未保存");
    }

    #[test]
    fn toast_constructs_with_empty_message() {
        // Defensive: the overlay is only rendered when `CaptureApp::toast`
        // is `Some(_)`, but if an upstream path ever passes an empty string
        // we should still produce a valid widget tree rather than panic.
        let palette = TraceTheme::for_preset(ThemePreset::Paper).capture;
        let _element: iced::Element<'_, Message> = toast(palette, "");
    }

    #[test]
    fn padding_constants_are_stable() {
        // Guard the Mac-reference values against accidental drift.
        assert_eq!(TOAST_HORIZONTAL_PADDING, 16);
        assert_eq!(TOAST_VERTICAL_PADDING, 10);
    }
}
