use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadConfig {
    pub id: Uuid,
    pub name: String,
    pub target_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub order: i32,
}

impl ThreadConfig {
    pub fn new(
        name: impl Into<String>,
        target_file: impl Into<String>,
        icon: Option<String>,
        order: i32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            target_file: target_file.into(),
            icon,
            order,
        }
    }

    pub fn with_id(
        id: Uuid,
        name: impl Into<String>,
        target_file: impl Into<String>,
        icon: Option<String>,
        order: i32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            target_file: target_file.into(),
            icon,
            order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_sets_fields_and_generates_id() {
        let config = ThreadConfig::new("想法", "想法.md", Some("lightbulb".into()), 0);
        assert_eq!(config.name, "想法");
        assert_eq!(config.target_file, "想法.md");
        assert_eq!(config.icon.as_deref(), Some("lightbulb"));
        assert_eq!(config.order, 0);
        assert!(!config.id.is_nil());
    }

    #[test]
    fn fresh_configs_have_distinct_ids() {
        let a = ThreadConfig::new("A", "A.md", None, 0);
        let b = ThreadConfig::new("B", "B.md", None, 1);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn serialize_uses_camel_case_keys() {
        let id = Uuid::parse_str("3f2504e0-4f89-41d3-9a0c-0305e82c3301").unwrap();
        let config = ThreadConfig::with_id(id, "想法", "想法.md", Some("lightbulb".into()), 0);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"targetFile\":\"想法.md\""));
        assert!(json.contains("\"icon\":\"lightbulb\""));
        assert!(json.contains("\"order\":0"));
        assert!(json.contains("\"id\":\"3f2504e0-4f89-41d3-9a0c-0305e82c3301\""));
    }

    #[test]
    fn round_trip_through_json_preserves_all_fields() {
        let id = Uuid::new_v4();
        let config = ThreadConfig::with_id(id, "读书笔记", "读书笔记.md", Some("book".into()), 2);
        let json = serde_json::to_string(&config).unwrap();
        let decoded: ThreadConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn icon_is_optional_and_omitted_when_none() {
        let id = Uuid::new_v4();
        let config = ThreadConfig::with_id(id, "t", "t.md", None, 0);
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("\"icon\""));
        let decoded: ThreadConfig = serde_json::from_str(&json).unwrap();
        assert!(decoded.icon.is_none());
    }
}
