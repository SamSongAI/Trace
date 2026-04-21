//! Preset file-name date formats for Daily write mode.
//!
//! Mirrors `Sources/Trace/Services/AppSettings.swift`'s
//! `DailyFileDateFormat` enum. Each variant carries the raw ICU format
//! string stored on disk (`AppSettings.daily_file_date_format`) so the
//! Windows port reads and writes the same JSON value as Mac.
//!
//! The Mac reference renders each picker entry as `"{raw}  →  {example}"`
//! where `example` is the current date formatted through the raw ICU
//! pattern using `DateFormatter.locale = .current`. The Windows port keeps
//! the same visible shape but hard-codes each variant's example template
//! against `chrono::Local::now()` — this keeps the preview deterministic
//! without pulling in ICU or depending on `chrono::Locale`.
//!
//! Persistence uses the raw ICU string (see [`Self::raw_value`]) and
//! [`Self::resolved_from_raw`] mirrors Swift's `resolved(fromStored:)`:
//! unknown strings fall back to [`Self::ChineseFull`].

use chrono::{Datelike, Local};

/// The five preset formats shown in the Storage → File Name Format picker.
///
/// Ordered identically to `DailyFileDateFormat.allCases` in the Mac
/// reference so the persisted preset can round-trip through the UI
/// without reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DailyFileDateFormat {
    /// `yyyy M月d日 EEEE` — Chinese long form (default).
    ChineseFull,
    /// `yyyy M月d日` — Chinese short form.
    ChineseShort,
    /// `yyyy-MM-dd` — ISO 8601 date.
    IsoDate,
    /// `yyyy-MM-dd EEEE` — ISO date with Chinese weekday suffix.
    IsoDateTime,
    /// `yyyy/MM/dd` — slash-separated date.
    SlashDate,
}

impl DailyFileDateFormat {
    /// Every variant, in the order the picker renders them. Pinned so
    /// `pick_list(&DailyFileDateFormat::ALL, …)` is stable across builds.
    pub const ALL: [Self; 5] = [
        Self::ChineseFull,
        Self::ChineseShort,
        Self::IsoDate,
        Self::IsoDateTime,
        Self::SlashDate,
    ];

    /// Returns the raw ICU format string stored on disk.
    ///
    /// Must match the Mac reference byte-for-byte — this value is the
    /// on-disk representation of `AppSettings.daily_file_date_format`
    /// (dotted key `trace.dailyFileDateFormat`).
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::ChineseFull => "yyyy M月d日 EEEE",
            Self::ChineseShort => "yyyy M月d日",
            Self::IsoDate => "yyyy-MM-dd",
            Self::IsoDateTime => "yyyy-MM-dd EEEE",
            Self::SlashDate => "yyyy/MM/dd",
        }
    }

    /// Resolves a stored raw ICU string back into the preset, falling back
    /// to [`Self::ChineseFull`] when the string is unknown. Mirrors Mac's
    /// `DailyFileDateFormat.resolved(fromStored:)`.
    pub fn resolved_from_raw(raw: &str) -> Self {
        for preset in Self::ALL {
            if preset.raw_value() == raw {
                return preset;
            }
        }
        Self::ChineseFull
    }

    /// Human-readable picker label rendered as `"{raw}  →  {example}"`.
    ///
    /// The preview is built against the current local date using hand-written
    /// format helpers so the output stays consistent across platforms without
    /// depending on `chrono`'s optional `locale` feature. Weekday labels use
    /// the Chinese convention ("星期二") — localizing the preview per
    /// `Language` is deferred to a future sub-task (MVP matches Mac's shipped
    /// fallback, which also falls back to `.current` but displays Chinese on
    /// zh-CN hosts).
    pub fn title(self) -> String {
        let now = Local::now().date_naive();
        let year = now.year();
        let month = now.month();
        let day = now.day();
        let weekday_zh = chinese_weekday(now.weekday());

        let example = match self {
            Self::ChineseFull => format!("{year} {month}月{day}日 {weekday_zh}"),
            Self::ChineseShort => format!("{year} {month}月{day}日"),
            Self::IsoDate => format!("{year:04}-{month:02}-{day:02}"),
            Self::IsoDateTime => format!("{year:04}-{month:02}-{day:02} {weekday_zh}"),
            Self::SlashDate => format!("{year:04}/{month:02}/{day:02}"),
        };

        format!("{}  →  {}", self.raw_value(), example)
    }
}

/// Maps a `chrono::Weekday` to the two-character Chinese weekday label used
/// by the Mac reference's `.current` locale on zh-CN.
fn chinese_weekday(weekday: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;
    match weekday {
        Mon => "星期一",
        Tue => "星期二",
        Wed => "星期三",
        Thu => "星期四",
        Fri => "星期五",
        Sat => "星期六",
        Sun => "星期日",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_enumerates_five_variants_in_mac_order() {
        // Mac `DailyFileDateFormat.allCases` order. A drift here would
        // reorder the picker and break a stored `lastUsedSectionIndex`-like
        // remembered choice on any future backed-by-index binding.
        assert_eq!(DailyFileDateFormat::ALL.len(), 5);
        assert_eq!(DailyFileDateFormat::ALL[0], DailyFileDateFormat::ChineseFull);
        assert_eq!(DailyFileDateFormat::ALL[1], DailyFileDateFormat::ChineseShort);
        assert_eq!(DailyFileDateFormat::ALL[2], DailyFileDateFormat::IsoDate);
        assert_eq!(DailyFileDateFormat::ALL[3], DailyFileDateFormat::IsoDateTime);
        assert_eq!(DailyFileDateFormat::ALL[4], DailyFileDateFormat::SlashDate);
    }

    #[test]
    fn raw_value_matches_mac_icu_strings() {
        // Byte-for-byte parity with the Mac `rawValue` — the persisted JSON
        // travels between Windows and Mac unmodified.
        assert_eq!(DailyFileDateFormat::ChineseFull.raw_value(), "yyyy M月d日 EEEE");
        assert_eq!(DailyFileDateFormat::ChineseShort.raw_value(), "yyyy M月d日");
        assert_eq!(DailyFileDateFormat::IsoDate.raw_value(), "yyyy-MM-dd");
        assert_eq!(DailyFileDateFormat::IsoDateTime.raw_value(), "yyyy-MM-dd EEEE");
        assert_eq!(DailyFileDateFormat::SlashDate.raw_value(), "yyyy/MM/dd");
    }

    #[test]
    fn resolved_from_raw_returns_matching_variant() {
        for preset in DailyFileDateFormat::ALL {
            assert_eq!(
                DailyFileDateFormat::resolved_from_raw(preset.raw_value()),
                preset
            );
        }
    }

    #[test]
    fn resolved_from_raw_unknown_falls_back_to_chinese_full() {
        assert_eq!(
            DailyFileDateFormat::resolved_from_raw(""),
            DailyFileDateFormat::ChineseFull
        );
        assert_eq!(
            DailyFileDateFormat::resolved_from_raw("dd/MM/yyyy"),
            DailyFileDateFormat::ChineseFull
        );
        assert_eq!(
            DailyFileDateFormat::resolved_from_raw("not a format"),
            DailyFileDateFormat::ChineseFull
        );
    }

    #[test]
    fn title_contains_raw_value_and_arrow_separator() {
        for preset in DailyFileDateFormat::ALL {
            let title = preset.title();
            assert!(
                !title.is_empty(),
                "{preset:?} title must not be empty"
            );
            assert!(
                title.contains("  →  "),
                "{preset:?} title missing the arrow separator: {title}"
            );
            assert!(
                title.starts_with(preset.raw_value()),
                "{preset:?} title must begin with the raw ICU format: {title}"
            );
        }
    }

    #[test]
    fn title_iso_date_preview_renders_zero_padded_date() {
        // The ISO preview must zero-pad month/day so a single-digit month
        // like April still reads `2026-04-...` rather than `2026-4-...`.
        let title = DailyFileDateFormat::IsoDate.title();
        let parts: Vec<&str> = title.split("  →  ").collect();
        assert_eq!(parts.len(), 2);
        let example = parts[1];
        // Example must match the shape `YYYY-MM-DD` (10 chars total).
        assert_eq!(example.len(), 10, "ISO preview wrong length: {example}");
        assert_eq!(&example[4..5], "-");
        assert_eq!(&example[7..8], "-");
    }

    #[test]
    fn title_chinese_full_contains_weekday_prefix() {
        // The weekday label uses "星期X" — we can assert the prefix rather
        // than the specific day, which depends on when the test runs.
        let title = DailyFileDateFormat::ChineseFull.title();
        assert!(
            title.contains("星期"),
            "ChineseFull title should contain a Chinese weekday: {title}"
        );
    }

    #[test]
    fn chinese_weekday_covers_all_seven_days() {
        use chrono::Weekday::*;
        // Round-trip every weekday to guarantee no arm is dead.
        let all = [Mon, Tue, Wed, Thu, Fri, Sat, Sun];
        let labels: Vec<&'static str> = all.iter().map(|w| chinese_weekday(*w)).collect();
        // Every label is non-empty and distinct.
        for (i, a) in labels.iter().enumerate() {
            assert!(!a.is_empty(), "{:?} weekday label empty", all[i]);
            for b in &labels[i + 1..] {
                assert_ne!(a, b, "duplicate weekday label: {a}");
            }
        }
    }
}
