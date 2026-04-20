use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryTheme {
    #[default]
    CodeBlockClassic,
    PlainTextTimestamp,
    MarkdownQuote,
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
}
