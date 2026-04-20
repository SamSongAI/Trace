use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeparatorStyle {
    None,
    #[default]
    HorizontalRule,
    AsteriskRule,
}

impl SeparatorStyle {
    pub fn markdown(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::HorizontalRule => Some("---"),
            Self::AsteriskRule => Some("***"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(SeparatorStyle::default(), SeparatorStyle::HorizontalRule);
    }

    #[test]
    fn serializes_as_camel_case_raw_values() {
        assert_eq!(
            serde_json::to_string(&SeparatorStyle::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&SeparatorStyle::HorizontalRule).unwrap(),
            "\"horizontalRule\""
        );
        assert_eq!(
            serde_json::to_string(&SeparatorStyle::AsteriskRule).unwrap(),
            "\"asteriskRule\""
        );
    }

    #[test]
    fn round_trip_through_json() {
        for style in [
            SeparatorStyle::None,
            SeparatorStyle::HorizontalRule,
            SeparatorStyle::AsteriskRule,
        ] {
            let json = serde_json::to_string(&style).unwrap();
            let decoded: SeparatorStyle = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, style);
        }
    }

    #[test]
    fn markdown_renders_expected_rule() {
        assert_eq!(SeparatorStyle::None.markdown(), None);
        assert_eq!(SeparatorStyle::HorizontalRule.markdown(), Some("---"));
        assert_eq!(SeparatorStyle::AsteriskRule.markdown(), Some("***"));
    }
}
