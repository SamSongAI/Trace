//! Swift `DateFormatter` pattern → chrono pattern translation.
//!
//! Mac Trace persists date format strings using Unicode TR 35 tokens
//! (`yyyy`, `M`, `d`, `EEEE`, `MM`, `dd`). chrono uses strftime tokens
//! (`%Y`, `%-m`, `%-d`, `%A`, `%m`, `%d`). To achieve byte-level output
//! parity with the Mac client we must:
//!
//! 1. Translate the five presets Mac ships (see
//!    `AppSettings.DailyFileDateFormat`) into chrono patterns, preserving
//!    literal characters like `月`, `日`, `-`, `/`, and spaces.
//! 2. Format using the locale Mac uses when *writing files*, which is
//!    `zh_CN` (see `DailyNoteWriter.formattedFileName`). Weekdays must be
//!    rendered in Chinese for the default preset (`yyyy M月d日 EEEE` →
//!    `2026 4月20日 星期一`).
//!
//! Because chrono's built-in localized formatting is gated on the `unstable-
//! locales` feature (and its zh_CN weekday table uses slightly different
//! strings than Apple's), we use a narrow hand-rolled lookup for weekday and
//! month names. Trace only supports zh/en/ja, so a fixed table is exact and
//! keeps the crate dependency-free.

use chrono::{Datelike, NaiveDate};

use crate::error::TraceError;

/// Locales supported for date formatting. Mirrors the locales Mac Trace uses
/// directly: `zh_CN` for file names, `en_US_POSIX` for timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// `zh_CN`. Used for daily filename rendering (weekdays → 星期一 ...).
    ZhCn,
    /// `en_US_POSIX`. Used for ISO-style timestamps.
    EnUsPosix,
    /// `ja_JP`. Unused by the Mac client but wired up for language switching.
    JaJp,
}

/// The five filename presets Mac Trace ships, verbatim from
/// `DailyFileDateFormat` in `Sources/Trace/Services/AppSettings.swift:167`.
pub const MAC_DATE_FORMAT_PRESETS: [&str; 5] = [
    "yyyy M月d日 EEEE",
    "yyyy M月d日",
    "yyyy-MM-dd",
    "yyyy-MM-dd EEEE",
    "yyyy/MM/dd",
];

/// Translates a Unicode TR 35 pattern (as used by `DateFormatter`) into a
/// chrono strftime-style pattern. Only the tokens needed by the five presets
/// and a handful of obvious extras are supported; anything else returns
/// `TraceError::UnsupportedDatePatternToken`.
///
/// Supported tokens:
/// - Year: `yyyy`, `yy`
/// - Month: `MMMM` (full name), `MMM` (short name), `MM` (zero-padded),
///   `M` (no padding)
/// - Day: `dd` (zero-padded), `d` (no padding)
/// - Weekday: `EEEE` (full name), `EEE` (short name)
///
/// Characters that are not ASCII letters pass through as literals, so
/// `月`, `日`, `-`, `/`, and spaces are preserved byte-for-byte.
pub fn translate_swift_pattern(swift_pattern: &str) -> Result<String, TraceError> {
    let mut out = String::with_capacity(swift_pattern.len());
    let chars: Vec<char> = swift_pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if is_pattern_letter(c) {
            let run_start = i;
            while i < chars.len() && chars[i] == c {
                i += 1;
            }
            let run_len = i - run_start;
            let token: String = std::iter::repeat(c).take(run_len).collect();
            out.push_str(translate_token(&token)?);
        } else {
            // Literal. Escape strftime metacharacter so it round-trips.
            if c == '%' {
                out.push_str("%%");
            } else {
                out.push(c);
            }
            i += 1;
        }
    }

    Ok(out)
}

fn is_pattern_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn translate_token(token: &str) -> Result<&'static str, TraceError> {
    match token {
        "yyyy" => Ok("%Y"),
        "yy" => Ok("%y"),
        "MMMM" => Ok("%B"),
        "MMM" => Ok("%b"),
        "MM" => Ok("%m"),
        "M" => Ok("%-m"),
        "dd" => Ok("%d"),
        "d" => Ok("%-d"),
        "EEEE" => Ok("%A"),
        "EEE" => Ok("%a"),
        other => Err(TraceError::UnsupportedDatePatternToken(other.to_string())),
    }
}

/// Formats `date` according to the Swift pattern (translated internally to a
/// chrono pattern). Locale-sensitive tokens (`%A`, `%a`, `%B`, `%b`) are
/// substituted using the manual name tables below so the output matches
/// Apple's `DateFormatter` byte-for-byte for zh/en/ja.
pub fn format_date(
    swift_pattern: &str,
    date: &NaiveDate,
    locale: Locale,
) -> Result<String, TraceError> {
    // We translate then render ourselves rather than delegating to
    // `chrono::format::strftime`, because chrono's default locale is always
    // English. Doing it here keeps the crate free of extra features.
    let chrono_pattern = translate_swift_pattern(swift_pattern)?;
    Ok(render(&chrono_pattern, date, locale))
}

fn render(chrono_pattern: &str, date: &NaiveDate, locale: Locale) -> String {
    let mut out = String::with_capacity(chrono_pattern.len() + 8);
    let chars: Vec<char> = chrono_pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c != '%' {
            out.push(c);
            i += 1;
            continue;
        }

        i += 1;
        if i >= chars.len() {
            out.push('%');
            break;
        }

        let (dash, token_char) = if chars[i] == '-' && i + 1 < chars.len() {
            (true, chars[i + 1])
        } else {
            (false, chars[i])
        };
        i += if dash { 2 } else { 1 };

        match token_char {
            'Y' => out.push_str(&format!("{:04}", date.year())),
            'y' => out.push_str(&format!("{:02}", date.year() % 100)),
            'm' => {
                if dash {
                    out.push_str(&date.month().to_string());
                } else {
                    out.push_str(&format!("{:02}", date.month()));
                }
            }
            'd' => {
                if dash {
                    out.push_str(&date.day().to_string());
                } else {
                    out.push_str(&format!("{:02}", date.day()));
                }
            }
            'A' => out.push_str(weekday_full(date, locale)),
            'a' => out.push_str(weekday_short(date, locale)),
            'B' => out.push_str(month_full(date, locale)),
            'b' => out.push_str(month_short(date, locale)),
            '%' => out.push('%'),
            other => {
                out.push('%');
                if dash {
                    out.push('-');
                }
                out.push(other);
            }
        }
    }

    out
}

fn weekday_index(date: &NaiveDate) -> usize {
    // `num_days_from_monday` returns 0..=6 starting Monday.
    date.weekday().num_days_from_monday() as usize
}

fn month_index(date: &NaiveDate) -> usize {
    (date.month() as usize).saturating_sub(1)
}

fn weekday_full(date: &NaiveDate, locale: Locale) -> &'static str {
    const ZH: [&str; 7] = [
        "星期一",
        "星期二",
        "星期三",
        "星期四",
        "星期五",
        "星期六",
        "星期日",
    ];
    const EN: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    const JA: [&str; 7] = [
        "月曜日",
        "火曜日",
        "水曜日",
        "木曜日",
        "金曜日",
        "土曜日",
        "日曜日",
    ];
    match locale {
        Locale::ZhCn => ZH[weekday_index(date)],
        Locale::EnUsPosix => EN[weekday_index(date)],
        Locale::JaJp => JA[weekday_index(date)],
    }
}

fn weekday_short(date: &NaiveDate, locale: Locale) -> &'static str {
    const ZH: [&str; 7] = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
    const EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const JA: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
    match locale {
        Locale::ZhCn => ZH[weekday_index(date)],
        Locale::EnUsPosix => EN[weekday_index(date)],
        Locale::JaJp => JA[weekday_index(date)],
    }
}

fn month_full(date: &NaiveDate, locale: Locale) -> &'static str {
    const ZH: [&str; 12] = [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ];
    const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const JA: [&str; 12] = [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ];
    match locale {
        Locale::ZhCn => ZH[month_index(date)],
        Locale::EnUsPosix => EN[month_index(date)],
        Locale::JaJp => JA[month_index(date)],
    }
}

fn month_short(date: &NaiveDate, locale: Locale) -> &'static str {
    const EN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    match locale {
        Locale::ZhCn => month_full(date, locale),
        Locale::JaJp => month_full(date, locale),
        Locale::EnUsPosix => EN[month_index(date)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monday() -> NaiveDate {
        // 2026-04-20 is a Monday. Chosen to match the project's canonical
        // fixed-date convention used elsewhere in the suite.
        NaiveDate::from_ymd_opt(2026, 4, 20).unwrap()
    }

    #[test]
    fn translate_handles_chinese_full_preset() {
        let pattern = translate_swift_pattern("yyyy M月d日 EEEE").unwrap();
        assert_eq!(pattern, "%Y %-m月%-d日 %A");
    }

    #[test]
    fn translate_handles_chinese_short_preset() {
        let pattern = translate_swift_pattern("yyyy M月d日").unwrap();
        assert_eq!(pattern, "%Y %-m月%-d日");
    }

    #[test]
    fn translate_handles_iso_date_preset() {
        let pattern = translate_swift_pattern("yyyy-MM-dd").unwrap();
        assert_eq!(pattern, "%Y-%m-%d");
    }

    #[test]
    fn translate_handles_iso_date_with_weekday_preset() {
        let pattern = translate_swift_pattern("yyyy-MM-dd EEEE").unwrap();
        assert_eq!(pattern, "%Y-%m-%d %A");
    }

    #[test]
    fn translate_handles_slash_date_preset() {
        let pattern = translate_swift_pattern("yyyy/MM/dd").unwrap();
        assert_eq!(pattern, "%Y/%m/%d");
    }

    #[test]
    fn translate_rejects_unsupported_token() {
        let err = translate_swift_pattern("yyyy Q").unwrap_err();
        assert!(matches!(err, TraceError::UnsupportedDatePatternToken(_)));
    }

    #[test]
    fn translate_escapes_percent_literals() {
        // `%` is legal in a Swift pattern (it's not a pattern letter), but in
        // chrono strftime it would be interpreted as a specifier. Escape it.
        let pattern = translate_swift_pattern("yyyy %").unwrap();
        assert_eq!(pattern, "%Y %%");
    }

    #[test]
    fn format_matches_mac_chinese_full_default() {
        let out = format_date("yyyy M月d日 EEEE", &monday(), Locale::ZhCn).unwrap();
        assert_eq!(out, "2026 4月20日 星期一");
    }

    #[test]
    fn format_matches_mac_chinese_short() {
        let out = format_date("yyyy M月d日", &monday(), Locale::ZhCn).unwrap();
        assert_eq!(out, "2026 4月20日");
    }

    #[test]
    fn format_matches_mac_iso_date() {
        let out = format_date("yyyy-MM-dd", &monday(), Locale::EnUsPosix).unwrap();
        assert_eq!(out, "2026-04-20");
    }

    #[test]
    fn format_matches_mac_iso_date_with_weekday_in_chinese() {
        let out = format_date("yyyy-MM-dd EEEE", &monday(), Locale::ZhCn).unwrap();
        assert_eq!(out, "2026-04-20 星期一");
    }

    #[test]
    fn format_matches_mac_iso_date_with_weekday_in_english() {
        let out = format_date("yyyy-MM-dd EEEE", &monday(), Locale::EnUsPosix).unwrap();
        assert_eq!(out, "2026-04-20 Monday");
    }

    #[test]
    fn format_matches_mac_slash_date() {
        let out = format_date("yyyy/MM/dd", &monday(), Locale::EnUsPosix).unwrap();
        assert_eq!(out, "2026/04/20");
    }

    #[test]
    fn format_weekday_japanese() {
        let out = format_date("yyyy-MM-dd EEEE", &monday(), Locale::JaJp).unwrap();
        assert_eq!(out, "2026-04-20 月曜日");
    }

    #[test]
    fn format_month_full_chinese() {
        let out = format_date("yyyy MMMM d", &monday(), Locale::ZhCn).unwrap();
        assert_eq!(out, "2026 四月 20");
    }

    #[test]
    fn format_month_short_english() {
        let out = format_date("MMM d, yyyy", &monday(), Locale::EnUsPosix).unwrap();
        assert_eq!(out, "Apr 20, 2026");
    }

    #[test]
    fn format_two_digit_year() {
        let out = format_date("yy", &monday(), Locale::EnUsPosix).unwrap();
        assert_eq!(out, "26");
    }

    #[test]
    fn mac_presets_constant_matches_source_of_truth() {
        // Guards against accidental drift from the Swift source file.
        assert_eq!(
            MAC_DATE_FORMAT_PRESETS,
            [
                "yyyy M月d日 EEEE",
                "yyyy M月d日",
                "yyyy-MM-dd",
                "yyyy-MM-dd EEEE",
                "yyyy/MM/dd",
            ]
        );
    }

    #[test]
    fn all_mac_presets_translate_successfully() {
        for preset in MAC_DATE_FORMAT_PRESETS {
            let pattern = translate_swift_pattern(preset).unwrap();
            assert!(
                !pattern.is_empty(),
                "preset {preset} produced empty pattern"
            );
        }
    }
}
