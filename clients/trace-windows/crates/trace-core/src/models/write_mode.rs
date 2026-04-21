use serde::{Deserialize, Serialize};

use crate::l10n::L10n;
use crate::models::Language;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteMode {
    #[default]
    Dimension,
    Thread,
    File,
}

impl WriteMode {
    pub fn next(self) -> Self {
        match self {
            Self::Dimension => Self::Thread,
            Self::Thread => Self::File,
            Self::File => Self::Dimension,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Dimension => Self::File,
            Self::Thread => Self::Dimension,
            Self::File => Self::Thread,
        }
    }

    /// Single-character icon glyph rendered inside the 28×28 swatch on each
    /// write-mode tile.
    ///
    /// Mac uses SF Symbols (`square.grid.2x2`, `text.bubble`, `doc.text`);
    /// the Windows port picks unicode glyphs from the Segoe UI / Segoe UI
    /// Symbol fallback chain so the tiles render without bundling an icon
    /// font:
    ///
    /// * [`Self::Dimension`] → `"⊞"` (U+229E) — squared plus, mirrors the
    ///   "sections" grid metaphor from `square.grid.2x2`.
    /// * [`Self::Thread`]    → `"≡"` (U+2261) — identical-to, evokes a
    ///   chat-bubble stack like `text.bubble`.
    /// * [`Self::File`]      → `"📄"` (U+1F4C4) — page-facing-up emoji from
    ///   the Segoe UI Emoji family, mirrors `doc.text` and避免与
    ///   [`crate::ThemePreset::Paper`] 的 `"▤"` 在设置面板里同时出现时产生视
    ///   觉混淆。
    pub const fn icon_glyph(self) -> &'static str {
        match self {
            Self::Dimension => "\u{229E}",
            Self::Thread => "\u{2261}",
            Self::File => "\u{1F4C4}",
        }
    }

    /// Localized short label rendered on each write-mode tile.
    ///
    /// Wraps [`L10n::write_mode_daily_compact`] / `write_mode_document_compact`
    /// / `write_mode_thread_compact` so callers never need to switch on the
    /// variant at the view layer. Mirrors Mac `NoteWriteMode.compactTitle`.
    pub fn compact_title(self, lang: Language) -> &'static str {
        match self {
            Self::Dimension => L10n::write_mode_daily_compact(lang),
            Self::Thread => L10n::write_mode_thread_compact(lang),
            Self::File => L10n::write_mode_document_compact(lang),
        }
    }

    /// Localized destination caption rendered under [`Self::compact_title`].
    ///
    /// Wraps the three `write_mode_*_destination` L10n entries so the view
    /// layer can pull a ready-to-render string for each variant. Mirrors Mac
    /// `NoteWriteMode.destinationTitle`.
    pub fn destination_title(self, lang: Language) -> &'static str {
        match self {
            Self::Dimension => L10n::write_mode_daily_destination(lang),
            Self::Thread => L10n::write_mode_thread_destination(lang),
            Self::File => L10n::write_mode_document_destination(lang),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(WriteMode::default(), WriteMode::Dimension);
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&WriteMode::Dimension).unwrap(),
            "\"dimension\""
        );
        assert_eq!(
            serde_json::to_string(&WriteMode::Thread).unwrap(),
            "\"thread\""
        );
        assert_eq!(serde_json::to_string(&WriteMode::File).unwrap(), "\"file\"");
    }

    #[test]
    fn round_trip_through_json() {
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            let json = serde_json::to_string(&mode).unwrap();
            let decoded: WriteMode = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, mode);
        }
    }

    #[test]
    fn next_cycles_dimension_thread_file() {
        assert_eq!(WriteMode::Dimension.next(), WriteMode::Thread);
        assert_eq!(WriteMode::Thread.next(), WriteMode::File);
        assert_eq!(WriteMode::File.next(), WriteMode::Dimension);
    }

    #[test]
    fn previous_is_inverse_of_next() {
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            assert_eq!(mode.next().previous(), mode);
        }
    }

    #[test]
    fn write_mode_icon_glyph_is_distinct_across_modes() {
        let variants = [WriteMode::Dimension, WriteMode::Thread, WriteMode::File];
        for mode in variants {
            assert!(
                !mode.icon_glyph().is_empty(),
                "{mode:?} must carry a non-empty glyph"
            );
        }
        for (i, a) in variants.iter().enumerate() {
            for b in &variants[i + 1..] {
                assert_ne!(
                    a.icon_glyph(),
                    b.icon_glyph(),
                    "modes {a:?} and {b:?} must have distinct glyphs"
                );
            }
        }
    }

    #[test]
    fn write_mode_icon_glyph_values_are_locked() {
        // 锁死每个变体当前使用的 unicode 码位,方便后续与 Mac SF Symbols
        // 对照或回归检查。`File` 使用 U+1F4C4(📄)而非 U+25A4(▤),是为
        // 了和 `ThemePreset::Paper` 拉开视觉差异——详见模块文档。
        assert_eq!(WriteMode::Dimension.icon_glyph(), "\u{229E}");
        assert_eq!(WriteMode::Thread.icon_glyph(), "\u{2261}");
        assert_eq!(WriteMode::File.icon_glyph(), "\u{1F4C4}");
    }

    #[test]
    fn write_mode_compact_title_and_destination_title_route_through_l10n() {
        // Every variant's helper must equal the direct L10n call for every
        // language. This catches drift if someone renames or re-routes an
        // L10n entry without updating the `WriteMode` wrapper.
        let langs = [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ];
        for lang in langs {
            assert_eq!(
                WriteMode::Dimension.compact_title(lang),
                L10n::write_mode_daily_compact(lang)
            );
            assert_eq!(
                WriteMode::Thread.compact_title(lang),
                L10n::write_mode_thread_compact(lang)
            );
            assert_eq!(
                WriteMode::File.compact_title(lang),
                L10n::write_mode_document_compact(lang)
            );

            assert_eq!(
                WriteMode::Dimension.destination_title(lang),
                L10n::write_mode_daily_destination(lang)
            );
            assert_eq!(
                WriteMode::Thread.destination_title(lang),
                L10n::write_mode_thread_destination(lang)
            );
            assert_eq!(
                WriteMode::File.destination_title(lang),
                L10n::write_mode_document_destination(lang)
            );
        }
    }
}
