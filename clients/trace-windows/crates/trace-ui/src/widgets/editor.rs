//! Capture-panel multi-line text editor.
//!
//! Wraps `iced::widget::text_editor` with the Trace visual treatment:
//! Lora 15 pt body text, panel-background fill, 16 px horizontal +
//! 12 px vertical padding to mimic the Mac `textContainerInset` of
//! `(w: 16, h: 12)` used in `CaptureTextEditor.swift`.
//!
//! # Phase 10 scope
//!
//! The editor intentionally does not consume placeholder text, per-mode hints,
//! paste events, or submit keybindings — those land in Phase 11 when the
//! interaction layer is wired up. What we commit to here is the visual shell
//! and the `EditorAction` → state plumbing.

use iced::widget::{container, text_editor};
use iced::{Length, Pixels};
use trace_core::CapturePalette;

use crate::app::Message;
use crate::fonts::LORA_FONT;
use crate::theme::{capture_editor_style, panel_container_style};

/// Horizontal padding applied to the editor container. Matches Mac
/// `CaptureTextEditor` `textContainerInset.w = 16`.
pub const EDITOR_HORIZONTAL_PADDING: u16 = 16;
/// Vertical padding applied to the editor container. Matches Mac
/// `CaptureTextEditor` `textContainerInset.h = 12`.
pub const EDITOR_VERTICAL_PADDING: u16 = 12;
/// Font size used by the editor body, matching Mac `Lora` 15 pt.
pub const EDITOR_FONT_SIZE: f32 = 15.0;

/// Builds the capture text editor element. The editor fills any remaining
/// vertical space between the header and the footer.
pub fn editor<'a>(
    palette: CapturePalette,
    content: &'a text_editor::Content,
) -> iced::Element<'a, Message> {
    let editor_widget = text_editor(content)
        .font(LORA_FONT)
        .size(Pixels(EDITOR_FONT_SIZE))
        .on_action(Message::EditorAction)
        .height(Length::Fill)
        .style(capture_editor_style(palette));

    container(editor_widget)
        .padding([EDITOR_VERTICAL_PADDING, EDITOR_HORIZONTAL_PADDING])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_container_style(palette))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text_editor::Content;
    use trace_core::{ThemePreset, TraceTheme};

    #[test]
    fn editor_builds_with_empty_content() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let content: Content = Content::new();
        let _element: iced::Element<'_, Message> = editor(palette, &content);
    }

    #[test]
    fn editor_builds_with_seeded_content() {
        let palette = TraceTheme::for_preset(ThemePreset::Paper).capture;
        let content: Content = Content::with_text("hello\nworld");
        let _element: iced::Element<'_, Message> = editor(palette, &content);
    }

    #[test]
    fn padding_matches_mac_reference() {
        // Guard test: the Mac `textContainerInset` values are load-bearing
        // for visual parity. If they ever diverge from the Swift source we
        // want the test suite to shout.
        assert_eq!(EDITOR_HORIZONTAL_PADDING, 16);
        assert_eq!(EDITOR_VERTICAL_PADDING, 12);
    }
}
