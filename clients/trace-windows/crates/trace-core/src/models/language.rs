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

impl Language {
    /// Returns the locale's self-referential name (endonym) rendered in its
    /// own script, matching Mac `AppLanguage.displayName`:
    ///
    /// * [`Language::Zh`] → `"中文"`
    /// * [`Language::En`] → `"English"`
    /// * [`Language::Ja`] → `"日本語"`
    ///
    /// [`Language::SystemDefault`] returns [`None`] because the sentinel has
    /// no natural endonym — the settings UI renders a separate "system
    /// default" label via [`crate::L10n::language_system_default`], so keeping
    /// the `None` case forces callers to handle the sentinel explicitly
    /// rather than silently falling back to an English label. The Mac
    /// reference skips the system-default variant entirely (no corresponding
    /// case exists in `AppLanguage`), so this method intentionally has no Mac
    /// peer.
    pub const fn native_display_name(self) -> Option<&'static str> {
        match self {
            Self::Zh => Some("中文"),
            Self::En => Some("English"),
            Self::Ja => Some("日本語"),
            Self::SystemDefault => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_system_default() {
        assert_eq!(Language::default(), Language::SystemDefault);
    }

    #[test]
    fn language_native_display_name_covers_three_native_locales() {
        assert_eq!(Language::Zh.native_display_name(), Some("中文"));
        assert_eq!(Language::En.native_display_name(), Some("English"));
        assert_eq!(Language::Ja.native_display_name(), Some("日本語"));
        assert_eq!(Language::SystemDefault.native_display_name(), None);
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
