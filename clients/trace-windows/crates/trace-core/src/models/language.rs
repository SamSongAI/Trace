use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    SystemDefault,
    Zh,
    En,
    Ja,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_system_default() {
        assert_eq!(Language::default(), Language::SystemDefault);
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&Language::SystemDefault).unwrap(),
            "\"systemDefault\""
        );
        assert_eq!(serde_json::to_string(&Language::Zh).unwrap(), "\"zh\"");
        assert_eq!(serde_json::to_string(&Language::En).unwrap(), "\"en\"");
        assert_eq!(serde_json::to_string(&Language::Ja).unwrap(), "\"ja\"");
    }

    #[test]
    fn round_trip_through_json() {
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            let json = serde_json::to_string(&lang).unwrap();
            let decoded: Language = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, lang);
        }
    }
}
