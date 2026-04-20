use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSection {
    pub index: usize,
    pub title: String,
}

impl NoteSection {
    pub const MINIMUM_COUNT: usize = 1;
    pub const MAXIMUM_COUNT: usize = 9;
    pub const DEFAULT_TITLES: [&'static str; 4] = ["Note", "Memo", "Link", "Task"];

    pub fn new(index: usize, title: impl Into<String>) -> Self {
        Self {
            index,
            title: title.into(),
        }
    }

    pub fn display_index(&self) -> usize {
        self.index + 1
    }

    pub fn header(&self) -> String {
        format!("# {}", self.title)
    }

    pub fn default_title_for(index: usize) -> String {
        format!("Section {}", index + 1)
    }
}

// Sections are identified by their slot index, not their display title.
// Two sections with the same index are equal even if titles differ, so
// renames (via settings) do not break collections keyed on section identity.
impl PartialEq for NoteSection {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for NoteSection {}

impl std::hash::Hash for NoteSection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_sets_fields() {
        let section = NoteSection::new(0, "Note");
        assert_eq!(section.index, 0);
        assert_eq!(section.title, "Note");
    }

    #[test]
    fn display_index_is_one_based() {
        let section = NoteSection::new(2, "Link");
        assert_eq!(section.display_index(), 3);
    }

    #[test]
    fn header_prefixes_hash() {
        let section = NoteSection::new(0, "Memo");
        assert_eq!(section.header(), "# Memo");
    }

    #[test]
    fn default_title_is_human_readable() {
        assert_eq!(NoteSection::default_title_for(4), "Section 5");
    }

    #[test]
    fn equality_matches_index_only() {
        let a = NoteSection::new(0, "Note");
        let b = NoteSection::new(0, "Memo");
        let c = NoteSection::new(1, "Note");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn serde_round_trip_preserves_fields() {
        let section = NoteSection::new(1, "Memo");
        let json = serde_json::to_string(&section).unwrap();
        assert_eq!(json, r#"{"index":1,"title":"Memo"}"#);
        let decoded: NoteSection = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.index, 1);
        assert_eq!(decoded.title, "Memo");
    }

    #[test]
    fn count_constants_match_mac() {
        assert_eq!(NoteSection::MINIMUM_COUNT, 1);
        assert_eq!(NoteSection::MAXIMUM_COUNT, 9);
        assert_eq!(
            NoteSection::DEFAULT_TITLES,
            ["Note", "Memo", "Link", "Task"]
        );
    }
}
