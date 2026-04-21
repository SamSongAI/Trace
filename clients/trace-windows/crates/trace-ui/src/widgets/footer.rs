//! Capture-panel footer — the region below the editor that changes with the
//! current [`trace_core::WriteMode`].
//!
//! | Mode | Content |
//! | --- | --- |
//! | [`WriteMode::Dimension`] | Grid of section chips sourced from [`AppSettings::sections`]. |
//! | [`WriteMode::Thread`] | Grid of thread chips sourced from [`AppSettings::thread_configs`] (sorted by `order`). |
//! | [`WriteMode::File`] | Single text input for the document title. |
//!
//! All three variants share a 1 px top separator, the chrome background, and
//! the 12 px horizontal / 8 px vertical padding from Mac
//! `CaptureView.swift:173-174`.
//!
//! # Grid layout
//!
//! The Mac reference uses SwiftUI's `LazyVGrid` with a min button width of
//! 92 pt and up to 3 rows. iced 0.14 does not ship a direct `LazyVGrid`
//! equivalent, so Phase 10 approximates the layout with a column of rows,
//! chunking chips into rows sized to [`SECTION_GRID_COLUMNS`]. This keeps the
//! visual rhythm close enough for the shell; Phase 11 can swap in a responsive
//! column count driven by the actual container width once we listen for
//! resize events.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Length, Pixels};
use trace_core::{CapturePalette, NoteSection, ThreadConfig, WriteMode};

use crate::app::{Message, SectionId, ThreadId, SEPARATOR_HEIGHT};
use crate::theme::{chip_button_style, chrome_container_style, document_title_input_style, separator_container_style};

/// Number of chip columns used by the Phase 10 static grid layout.
///
/// The Mac reference computes this from the live container width. Until
/// Phase 11 wires up iced's resize subscription, we pick a reasonable
/// default (3) that matches the shipped 440 px panel width.
pub const SECTION_GRID_COLUMNS: usize = 3;
/// Spacing between chips in either axis, matching Mac
/// `sectionGridSpacing = 6`.
pub const CHIP_SPACING: f32 = 6.0;
/// Horizontal inner padding of the chip grid area.
pub const FOOTER_HORIZONTAL_PADDING: u16 = 12;
/// Vertical inner padding of the chip grid area.
pub const FOOTER_VERTICAL_PADDING: u16 = 8;
/// Horizontal inner padding of the document-title row (wider than the grid
/// padding to mirror Mac `documentFooter` spacing).
pub const DOCUMENT_HORIZONTAL_PADDING: u16 = 12;
/// Vertical inner padding of the document-title row.
pub const DOCUMENT_VERTICAL_PADDING: u16 = 10;
/// Minimum chip button height, matching Mac `.frame(minHeight: 34)`.
pub const CHIP_MIN_HEIGHT: f32 = 34.0;
/// Document title field height, matching Mac `.frame(height: 38)`.
pub const DOCUMENT_TITLE_HEIGHT: f32 = 38.0;

/// Builds the footer element for the given write mode.
pub fn footer<'a>(
    palette: CapturePalette,
    write_mode: WriteMode,
    sections: &'a [NoteSection],
    threads: &'a [ThreadConfig],
    selected_section: Option<SectionId>,
    selected_thread: Option<ThreadId>,
    document_title: &'a str,
) -> iced::Element<'a, Message> {
    match write_mode {
        WriteMode::Dimension => footer_dimension(palette, sections, selected_section),
        WriteMode::Thread => footer_thread(palette, threads, selected_thread),
        WriteMode::File => footer_document(palette, document_title),
    }
}

/// Footer rendered when `WriteMode == Dimension` — the section chip grid.
pub fn footer_dimension<'a>(
    palette: CapturePalette,
    sections: &'a [NoteSection],
    selected: Option<SectionId>,
) -> iced::Element<'a, Message> {
    let chips: Vec<iced::Element<'_, Message>> = sections
        .iter()
        .map(|section| {
            let is_selected = selected == Some(section.index);
            section_chip(palette, section, is_selected)
        })
        .collect();

    let grid = chip_grid(chips);
    chrome_wrapper(palette, grid)
}

/// Footer rendered when `WriteMode == Thread` — the thread chip grid.
pub fn footer_thread<'a>(
    palette: CapturePalette,
    threads: &'a [ThreadConfig],
    selected: Option<ThreadId>,
) -> iced::Element<'a, Message> {
    let chips: Vec<iced::Element<'_, Message>> = threads
        .iter()
        .map(|thread| {
            let is_selected = selected == Some(thread.id);
            thread_chip(palette, thread, is_selected)
        })
        .collect();

    let grid = chip_grid(chips);
    chrome_wrapper(palette, grid)
}

/// Footer rendered when `WriteMode == File` — single-line title input.
pub fn footer_document<'a>(
    palette: CapturePalette,
    document_title: &'a str,
) -> iced::Element<'a, Message> {
    // Phase 10 placeholder is a neutral reminder. Localized strings are a
    // Phase 12 concern (L10n is already in trace-core but the UI wiring is
    // not ready here).
    // TODO(phase-11): wire the placeholder through `trace_core::L10n::document_placeholder`.
    let title_input = text_input("Document title", document_title)
        .on_input(Message::DocumentTitleChanged)
        .size(Pixels(13.0))
        .style(document_title_input_style(palette));

    let field = container(title_input)
        .padding([DOCUMENT_VERTICAL_PADDING, DOCUMENT_HORIZONTAL_PADDING])
        .width(Length::Fill)
        .height(Length::Fixed(
            DOCUMENT_TITLE_HEIGHT + f32::from(DOCUMENT_VERTICAL_PADDING) * 2.0,
        ));

    chrome_wrapper(palette, field.into())
}

fn chrome_wrapper<'a>(
    palette: CapturePalette,
    body: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let separator = container(
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(SEPARATOR_HEIGHT)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(SEPARATOR_HEIGHT))
    .style(separator_container_style(palette));

    let wrapper = column![separator, body].spacing(0).width(Length::Fill);

    container(wrapper)
        .width(Length::Fill)
        .style(chrome_container_style(palette))
        .into()
}

fn chip_grid<'a>(
    chips: Vec<iced::Element<'a, Message>>,
) -> iced::Element<'a, Message> {
    let columns = SECTION_GRID_COLUMNS.max(1);
    let mut rows: Vec<iced::Element<'a, Message>> = Vec::new();
    let mut current_row: Vec<iced::Element<'a, Message>> = Vec::with_capacity(columns);
    for chip in chips {
        current_row.push(chip);
        if current_row.len() == columns {
            rows.push(finalize_row(std::mem::take(&mut current_row)));
        }
    }
    if !current_row.is_empty() {
        // Pad trailing row with blank spacers so columns stay aligned.
        while current_row.len() < columns {
            current_row.push(
                Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(CHIP_MIN_HEIGHT))
                    .into(),
            );
        }
        rows.push(finalize_row(current_row));
    }

    let grid = column(rows)
        .spacing(CHIP_SPACING)
        .width(Length::Fill);

    container(grid)
        .padding([FOOTER_VERTICAL_PADDING, FOOTER_HORIZONTAL_PADDING])
        .width(Length::Fill)
        .into()
}

fn finalize_row<'a>(
    items: Vec<iced::Element<'a, Message>>,
) -> iced::Element<'a, Message> {
    row(items)
        .spacing(CHIP_SPACING)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn section_chip<'a>(
    palette: CapturePalette,
    section: &'a NoteSection,
    selected: bool,
) -> iced::Element<'a, Message> {
    let label = text(section.title.as_str()).size(Pixels(12.0));
    button(label)
        .width(Length::Fill)
        .height(Length::Fixed(CHIP_MIN_HEIGHT))
        .padding([8, 8])
        .on_press(Message::SectionSelected(section.index))
        .style(chip_button_style(palette, selected))
        .into()
}

fn thread_chip<'a>(
    palette: CapturePalette,
    thread: &'a ThreadConfig,
    selected: bool,
) -> iced::Element<'a, Message> {
    let label = text(thread.name.as_str()).size(Pixels(12.0));
    button(label)
        .width(Length::Fill)
        .height(Length::Fixed(CHIP_MIN_HEIGHT))
        .padding([8, 8])
        .on_press(Message::ThreadSelected(thread.id))
        .style(chip_button_style(palette, selected))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::{ThemePreset, TraceTheme};
    use uuid::Uuid;

    fn sample_sections() -> Vec<NoteSection> {
        NoteSection::DEFAULT_TITLES
            .iter()
            .enumerate()
            .map(|(i, title)| NoteSection::new(i, *title))
            .collect()
    }

    fn sample_threads() -> Vec<ThreadConfig> {
        vec![
            ThreadConfig::new("A", "a.md", None, 0),
            ThreadConfig::new("B", "b.md", None, 1),
        ]
    }

    #[test]
    fn dimension_footer_constructs_with_and_without_selection() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let sections = sample_sections();
        let _idle = footer_dimension(palette, &sections, None);
        let _selected = footer_dimension(palette, &sections, Some(0));
    }

    #[test]
    fn thread_footer_constructs_with_and_without_selection() {
        let palette = TraceTheme::for_preset(ThemePreset::Dark).capture;
        let threads = sample_threads();
        let _idle = footer_thread(palette, &threads, None);
        let _selected = footer_thread(palette, &threads, Some(threads[0].id));
    }

    #[test]
    fn document_footer_constructs_with_empty_and_populated_title() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let _empty: iced::Element<'_, Message> = footer_document(palette, "");
        let _populated: iced::Element<'_, Message> = footer_document(palette, "draft");
    }

    #[test]
    fn footer_branch_dispatches_by_write_mode() {
        let palette = TraceTheme::for_preset(ThemePreset::Light).capture;
        let sections = sample_sections();
        let threads = sample_threads();
        // Each branch returns an Element — exercising all three proves the
        // dispatcher compiles and picks the intended variant.
        let _dim = footer(
            palette,
            WriteMode::Dimension,
            &sections,
            &threads,
            None,
            None,
            "",
        );
        let _thr = footer(
            palette,
            WriteMode::Thread,
            &sections,
            &threads,
            None,
            Some(threads[0].id),
            "",
        );
        let _doc = footer(
            palette,
            WriteMode::File,
            &sections,
            &threads,
            None,
            None,
            "hello",
        );
    }

    #[test]
    fn chip_grid_handles_empty_chips() {
        // Phase 10 guards against an empty section/thread list. The footer
        // should still build — no panics expected.
        let palette = TraceTheme::for_preset(ThemePreset::Dune).capture;
        let _empty_sections = footer_dimension(palette, &[], None);
        let _empty_threads = footer_thread(palette, &[], None);
    }

    #[test]
    fn unmatched_thread_selection_does_not_crash() {
        let palette = TraceTheme::for_preset(ThemePreset::Dune).capture;
        let threads = sample_threads();
        // Simulate a stale selection carried over from a previous session
        // that no longer maps to any configured thread — the footer must
        // still render.
        let _element = footer_thread(palette, &threads, Some(Uuid::new_v4()));
    }

    #[test]
    fn chip_metrics_match_mac_reference() {
        assert_eq!(CHIP_MIN_HEIGHT, 34.0);
        assert_eq!(CHIP_SPACING, 6.0);
        assert_eq!(DOCUMENT_TITLE_HEIGHT, 38.0);
    }
}
