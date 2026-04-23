use serde::{Deserialize, Serialize};

use crate::l10n::L10n;
use crate::models::Language;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryTheme {
    #[default]
    CodeBlockClassic,
    PlainTextTimestamp,
    MarkdownQuote,
}

impl EntryTheme {
    /// Every variant, in the order the Storage → Entry Format picker renders
    /// them. Pinned so `pick_list(&EntryTheme::ALL, …)` stays stable across
    /// builds and so the settings UI can iterate without hand-writing the
    /// order at every call site.
    pub const ALL: [Self; 3] = [
        Self::CodeBlockClassic,
        Self::PlainTextTimestamp,
        Self::MarkdownQuote,
    ];

    /// Localized picker label. Mirrors Mac
    /// `DailyEntryThemePreset.title` which dispatches through `L10n`.
    pub fn title(self, lang: Language) -> &'static str {
        match self {
            Self::CodeBlockClassic => L10n::entry_code_block(lang),
            Self::PlainTextTimestamp => L10n::entry_plain_text(lang),
            Self::MarkdownQuote => L10n::entry_quote(lang),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(EntryTheme::default(), EntryTheme::CodeBlockClassic);
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&EntryTheme::CodeBlockClassic).unwrap(),
            "\"codeBlockClassic\""
        );
        assert_eq!(
            serde_json::to_string(&EntryTheme::PlainTextTimestamp).unwrap(),
            "\"plainTextTimestamp\""
        );
        assert_eq!(
            serde_json::to_string(&EntryTheme::MarkdownQuote).unwrap(),
            "\"markdownQuote\""
        );
    }

    #[test]
    fn round_trip_through_json() {
        for theme in [
            EntryTheme::CodeBlockClassic,
            EntryTheme::PlainTextTimestamp,
            EntryTheme::MarkdownQuote,
        ] {
            let json = serde_json::to_string(&theme).unwrap();
            let decoded: EntryTheme = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, theme);
        }
    }

    #[test]
    fn all_enumerates_three_variants_in_mac_order() {
        assert_eq!(EntryTheme::ALL.len(), 3);
        assert_eq!(EntryTheme::ALL[0], EntryTheme::CodeBlockClassic);
        assert_eq!(EntryTheme::ALL[1], EntryTheme::PlainTextTimestamp);
        assert_eq!(EntryTheme::ALL[2], EntryTheme::MarkdownQuote);
    }

    #[test]
    fn title_routes_through_l10n_for_every_language() {
        // The helper must match L10n directly for every (variant, lang)
        // pair; otherwise a rename in `L10n::entry_*` would silently leave
        // the picker label out of sync.
        let langs = [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ];
        for lang in langs {
            assert_eq!(
                EntryTheme::CodeBlockClassic.title(lang),
                L10n::entry_code_block(lang)
            );
            assert_eq!(
                EntryTheme::PlainTextTimestamp.title(lang),
                L10n::entry_plain_text(lang)
            );
            assert_eq!(
                EntryTheme::MarkdownQuote.title(lang),
                L10n::entry_quote(lang)
            );
        }
    }

    #[test]
    fn title_is_non_empty_and_distinct_across_variants() {
        for lang in [Language::Zh, Language::En, Language::Ja] {
            let titles: Vec<&'static str> = EntryTheme::ALL.iter().map(|t| t.title(lang)).collect();
            for (i, a) in titles.iter().enumerate() {
                assert!(
                    !a.is_empty(),
                    "variant {:?} title empty",
                    EntryTheme::ALL[i]
                );
                for b in &titles[i + 1..] {
                    assert_ne!(a, b, "duplicate title under {lang:?}: {a}");
                }
            }
        }
    }
}
