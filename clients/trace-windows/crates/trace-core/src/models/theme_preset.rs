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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_mac() {
        assert_eq!(ThemePreset::default(), ThemePreset::Dark);
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
