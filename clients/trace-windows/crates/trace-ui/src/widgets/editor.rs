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

use crate::app::{paste_key_binding, Message};
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
        // Ctrl+V (Cmd+V on macOS) is intercepted here so the app can route
        // paste through [`crate::clipboard::ClipboardProbe`] (image-first,
        // text-fallback). Every other key press delegates to iced's
        // default binding, so typing / selection / copy / cut / select-all
        // keep their built-in behaviour.
        .key_binding(paste_key_binding)
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
    use iced::widget::text_editor::{Binding, Content, KeyPress, Status};
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

    /// Duplicate of `app.rs::tests::key_press` — keeps this module's coverage
    /// self-contained so a reader inspecting `widgets/editor.rs` can verify
    /// the `.key_binding(paste_key_binding)` wire without jumping files.
    fn key_press(
        key: iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
        physical: iced::keyboard::key::Physical,
    ) -> KeyPress {
        KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: physical,
            modifiers,
            text: None,
            status: Status::Focused { is_hovered: false },
        }
    }

    #[test]
    fn paste_key_binding_intercepts_command_v() {
        // Mirrors `app.rs::tests::paste_key_binding_intercepts_command_v` so
        // the widget file documents its own paste wiring: the
        // `.key_binding(paste_key_binding)` call above routes Ctrl/Cmd+V into
        // `Message::PasteRequested`.
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        let press = key_press(
            Key::Character("v".into()),
            Modifiers::COMMAND,
            Physical::Code(Code::KeyV),
        );
        let binding = paste_key_binding(press);
        match binding {
            Some(Binding::Custom(Message::PasteRequested)) => {}
            other => panic!(
                "Ctrl/Cmd+V should map to Binding::Custom(PasteRequested), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn paste_key_binding_falls_through_for_other_keys() {
        // Mirrors `app.rs::tests::paste_key_binding_falls_through_for_other_keys`.
        // A plain 'A' with no modifiers must not produce our custom paste
        // message — it should delegate to iced's default binding.
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        let press = key_press(
            Key::Character("a".into()),
            Modifiers::empty(),
            Physical::Code(Code::KeyA),
        );
        let binding = paste_key_binding(press);
        assert!(
            !matches!(binding, Some(Binding::Custom(Message::PasteRequested))),
            "non-V key must not route to PasteRequested, got {:?}",
            binding
        );
    }
}
