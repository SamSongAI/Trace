//! System card: renders the "Launch at Login" toggle plus a right-aligned
//! `Trace v<version>` footer. Mirrors Mac `SettingsView.swift:527-547`.
//!
//! # Shadow-only contract
//!
//! This module **only reads** a `launch_at_login: bool` flag from
//! [`crate::settings::SettingsApp`] and emits a
//! [`crate::settings::SettingsMessage::LaunchAtLoginToggled`] message on
//! every flip. The toggle **does not** write to the filesystem or registry —
//! that wiring is deferred to sub-tasks 8b (persistence architecture) and
//! 8c (autostart integration). The rest of sub-task 8a follows the same
//! shadow-only discipline every other sub-task 1-7 card already does.
//!
//! # Unconditional render
//!
//! Unlike the Quick Sections card (Dimension-only) or the Threads card
//! (Thread-only), the System card is **not** gated on
//! [`trace_core::WriteMode`]. The setting it carries — "Launch at Login" —
//! applies equally under every write mode, so
//! [`crate::settings::build_cards`] pushes it after the
//! write-mode-specific branches and after the Shortcuts card
//! unconditionally.

use iced::font::Weight;
use iced::widget::{column, row, text, toggler, Space};
use iced::{Element, Font, Length, Pixels};
use trace_core::{L10n, Language, SettingsPalette};

use crate::theme::trace_color_to_iced;

use super::widgets::section_card;
use super::SettingsMessage;

/// Font size of the row label. Matches Mac
/// `.font(.system(size: 13, weight: .medium))` on the Launch-at-Login `Text`.
const TOGGLE_LABEL_FONT_SIZE: f32 = 13.0;
/// Font size of the trailing "Trace v<version>" caption. Matches Mac
/// `.font(.system(size: 11, weight: .medium))` on the version `Text`.
const VERSION_LABEL_FONT_SIZE: f32 = 11.0;
/// Vertical spacing between the toggle row and the version row inside the
/// card body. Matches Mac `VStack(spacing: 12)` in the System card.
const SYSTEM_CARD_ROW_SPACING: f32 = 12.0;

/// Medium-weight font shared by the toggle label and the version caption.
/// The Mac reference uses `weight: .medium` on both rows; iced 0.14 has no
/// built-in "medium" alias, so the helper assembles a `Font::DEFAULT` whose
/// weight is lifted to [`Weight::Medium`]. Mirrors the same pattern
/// `shortcuts.rs::label_font` already uses inside this module.
fn medium_font() -> Font {
    Font {
        weight: Weight::Medium,
        ..Font::DEFAULT
    }
}

/// Assembles the `"Trace v<crate-version>"` caption shown on the version
/// row. Extracted as a free function so the renderer **and** the unit test
/// both call the same code path — otherwise a test that re-runs
/// `format!("Trace v{}", trace_core::VERSION)` inline would be a tautology
/// against itself and miss a refactor that drops the brand prefix.
fn version_caption_text() -> String {
    format!("Trace v{}", trace_core::VERSION)
}

/// Builds the System card element.
///
/// The card contains two rows stacked vertically:
///
/// 1. Toggle row: a medium-weight label on the left and an iced `toggler`
///    on the right (space-filled between them so the toggle hugs the
///    trailing edge). The toggler emits
///    [`SettingsMessage::LaunchAtLoginToggled`] on every flip.
/// 2. Version row: a right-aligned `Trace v<version>` caption pulled from
///    [`trace_core::VERSION`] at compile time via `env!("CARGO_PKG_VERSION")`.
///
/// Both rows paint in the muted-text and section-title colors from the
/// shared [`SettingsPalette`], matching the Mac reference's
/// `palette.mutedText` / `palette.sectionTitle` choices.
pub(super) fn system_card<'a>(
    palette: SettingsPalette,
    lang: Language,
    launch_at_login: bool,
) -> Element<'a, SettingsMessage> {
    let label = text(L10n::launch_at_login(lang))
        .size(Pixels(TOGGLE_LABEL_FONT_SIZE))
        .color(trace_color_to_iced(palette.section_title))
        .font(medium_font());

    let toggle = toggler(launch_at_login).on_toggle(SettingsMessage::LaunchAtLoginToggled);

    let toggle_row = row![label, Space::new().width(Length::Fill), toggle]
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill);

    let version_caption = text(version_caption_text())
        .size(Pixels(VERSION_LABEL_FONT_SIZE))
        .color(trace_color_to_iced(palette.muted_text))
        .font(medium_font());

    let version_row = row![Space::new().width(Length::Fill), version_caption].width(Length::Fill);

    let body = column![toggle_row, version_row]
        .spacing(SYSTEM_CARD_ROW_SPACING)
        .width(Length::Fill);

    section_card(palette, L10n::system(lang), body.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};

    fn sample_palette() -> SettingsPalette {
        TraceTheme::for_preset(ThemePreset::Light).settings
    }

    #[test]
    fn card_renders_for_every_language_and_state() {
        // The System card must build for every (language, toggle-state)
        // combination so a drift in the L10n helpers or a stale match arm in
        // the renderer surfaces at test time rather than first paint.
        let palette = sample_palette();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for launch_at_login in [false, true] {
                let _element: Element<'_, SettingsMessage> =
                    system_card(palette, lang, launch_at_login);
            }
        }
    }

    #[test]
    fn card_renders_across_every_preset() {
        // Palette swaps must not break the card — mirror
        // `quick_sections::card_renders_across_every_preset` so a regression
        // in `section_title` / `muted_text` / `accent` slots is caught here.
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let palette = TraceTheme::for_preset(preset).settings;
            let _element: Element<'_, SettingsMessage> = system_card(palette, Language::En, false);
        }
    }

    #[test]
    fn version_caption_text_pins_brand_prefix_and_crate_version() {
        // `version_caption_text` is the single source of truth the renderer
        // hands to `text(...)`. A refactor that drops the `"Trace "` prefix,
        // replaces the `v` with `V`, or skips `trace_core::VERSION` would
        // mutate the helper and immediately fail this assert.
        let version = trace_core::VERSION;
        assert!(
            !version.is_empty(),
            "VERSION must be populated at build time"
        );
        assert_eq!(version_caption_text(), format!("Trace v{version}"));
    }

    #[test]
    fn layout_constants_match_mac_reference() {
        // Pin the three layout numbers to the Mac reference so an accidental
        // edit surfaces at test time instead of at first paint. Values come
        // from `SettingsView.swift:527-547`:
        //   - 13pt medium toggle label
        //   - 11pt medium version caption
        //   - VStack(spacing: 12) between the two body rows
        // Mirrors the same pinning pattern already in `shortcuts.rs`,
        // `threads.rs`, and `widgets.rs`.
        assert_eq!(TOGGLE_LABEL_FONT_SIZE, 13.0);
        assert_eq!(VERSION_LABEL_FONT_SIZE, 11.0);
        assert_eq!(SYSTEM_CARD_ROW_SPACING, 12.0);
    }
}
