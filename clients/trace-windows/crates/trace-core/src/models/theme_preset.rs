use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreset {
    Light,
    #[default]
    Dark,
    Paper,
    Dune,
}

impl ThemePreset {
    /// English preset name shown inside the theme picker tile. Mac
    /// `AppThemePreset.title` returns the same literals unconditionally, so
    /// the Windows port keeps them non-localized — the summary text below
    /// the title already carries the translated description via
    /// [`crate::L10n::theme_light_summary`] and friends.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Paper => "Paper",
            Self::Dune => "Dune",
        }
    }

    /// Single-character icon glyph rendered inside the 28×28 swatch next to
    /// the preset title.
    ///
    /// Mac uses SF Symbols (`sun.max`, `moon.stars`, `doc.text`, `sun.haze`),
    /// which are unavailable on Windows. The glyphs below are all covered by
    /// the default Segoe UI / Segoe UI Symbol fallback chain, so they render
    /// consistently across Win10 and Win11 without bundling an icon font.
    ///
    /// * [`Self::Light`] → `"☀"` (U+2600) — sun, mirrors `sun.max`.
    /// * [`Self::Dark`]  → `"☾"` (U+263E) — last-quarter moon, mirrors `moon.stars`.
    /// * [`Self::Paper`] → `"▤"` (U+25A4) — squared ruled paper, mirrors `doc.text`.
    /// * [`Self::Dune`]  → `"◐"` (U+25D0) — warm half-filled circle, mirrors `sun.haze`.
    pub const fn icon_glyph(self) -> &'static str {
        match self {
            Self::Light => "\u{2600}",
            Self::Dark => "\u{263E}",
            Self::Paper => "\u{25A4}",
            Self::Dune => "\u{25D0}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(ThemePreset::default(), ThemePreset::Dark);
    }

    #[test]
    fn theme_preset_title_is_stable_english() {
        assert_eq!(ThemePreset::Light.title(), "Light");
        assert_eq!(ThemePreset::Dark.title(), "Dark");
        assert_eq!(ThemePreset::Paper.title(), "Paper");
        assert_eq!(ThemePreset::Dune.title(), "Dune");
    }

    #[test]
    fn theme_preset_icon_glyph_is_distinct_across_presets() {
        let variants = [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ];
        for preset in variants {
            assert!(
                !preset.icon_glyph().is_empty(),
                "{preset:?} must carry a non-empty glyph"
            );
        }
        // Walk the upper triangle to confirm every pair of glyphs differs.
        for (i, a) in variants.iter().enumerate() {
            for b in &variants[i + 1..] {
                assert_ne!(
                    a.icon_glyph(),
                    b.icon_glyph(),
                    "presets {a:?} and {b:?} must have distinct glyphs"
                );
            }
        }
    }

    #[test]
    fn theme_preset_icon_glyph_values_are_locked() {
        // 锁死每个 preset 当前使用的 unicode 码位。`Paper` 保留 U+25A4(▤)
        // 作为纸张书页语义——与 `WriteMode::File` 的 U+1F4C4(📄)拉开视
        // 觉差异,详见模块文档。
        assert_eq!(ThemePreset::Light.icon_glyph(), "\u{2600}");
        assert_eq!(ThemePreset::Dark.icon_glyph(), "\u{263E}");
        assert_eq!(ThemePreset::Paper.icon_glyph(), "\u{25A4}");
        assert_eq!(ThemePreset::Dune.icon_glyph(), "\u{25D0}");
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&ThemePreset::Light).unwrap(),
            "\"light\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreset::Dark).unwrap(),
            "\"dark\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreset::Paper).unwrap(),
            "\"paper\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreset::Dune).unwrap(),
            "\"dune\""
        );
    }

    #[test]
    fn round_trip_through_json() {
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let json = serde_json::to_string(&preset).unwrap();
            let decoded: ThemePreset = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, preset);
        }
    }
}
