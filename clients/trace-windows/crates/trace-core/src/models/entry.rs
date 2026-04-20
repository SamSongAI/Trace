use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::write_mode::WriteMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub source_mode: WriteMode,
}

impl Entry {
    pub fn new(
        content: impl Into<String>,
        timestamp: DateTime<Utc>,
        source_mode: WriteMode,
    ) -> Self {
        Self {
            content: content.into(),
            timestamp,
            source_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_timestamp() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 20, 12, 34, 56).unwrap()
    }

    #[test]
    fn construction_sets_fields() {
        let entry = Entry::new("hello", fixed_timestamp(), WriteMode::Dimension);
        assert_eq!(entry.content, "hello");
        assert_eq!(entry.timestamp, fixed_timestamp());
        assert_eq!(entry.source_mode, WriteMode::Dimension);
    }

    #[test]
    fn serialize_uses_camel_case_keys() {
        let entry = Entry::new("hi", fixed_timestamp(), WriteMode::Thread);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"content\":\"hi\""));
        assert!(json.contains("\"sourceMode\":\"thread\""));
        assert!(json.contains("\"timestamp\":\""));
    }

    #[test]
    fn round_trip_through_json_preserves_all_fields() {
        let entry = Entry::new("content body", fixed_timestamp(), WriteMode::File);
        let json = serde_json::to_string(&entry).unwrap();
        let decoded: Entry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, entry);
    }
}
