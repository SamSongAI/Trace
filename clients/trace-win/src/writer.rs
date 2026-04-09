use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

use crate::{
    model::NoteSection,
    settings::{AppSettings, NoteWriteMode},
};

pub fn save_note(
    now: DateTime<Local>,
    text: &str,
    section: NoteSection,
    settings: &AppSettings,
) -> Result<PathBuf> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Cannot save empty note")
    }

    match settings.note_write_mode {
        NoteWriteMode::Dimension => save_note_to_daily(now, trimmed, section, settings),
        NoteWriteMode::File => save_note_to_inbox(now, trimmed, section, settings),
    }
}

fn save_note_to_daily(
    now: DateTime<Local>,
    text: &str,
    section: NoteSection,
    settings: &AppSettings,
) -> Result<PathBuf> {
    let file_path = daily_file_path(now, settings)?;
    let parent = file_path
        .parent()
        .context("Failed to locate parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory {}", parent.display()))?;

    let current = fs::read_to_string(&file_path).unwrap_or_default();
    let updated = insert_entry(&current, text, settings.title_for(section), now);

    fs::write(&file_path, updated)
        .with_context(|| format!("Failed to write file {}", file_path.display()))?;

    Ok(file_path)
}

fn save_note_to_inbox(
    now: DateTime<Local>,
    text: &str,
    section: NoteSection,
    settings: &AppSettings,
) -> Result<PathBuf> {
    let file_path = inbox_file_path(now, settings)?;
    let parent = file_path
        .parent()
        .context("Failed to locate parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory {}", parent.display()))?;

    let content = standalone_file_content(text, settings.title_for(section), now);
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write file {}", file_path.display()))?;

    Ok(file_path)
}

pub fn daily_file_path(now: DateTime<Local>, settings: &AppSettings) -> Result<PathBuf> {
    let vault = settings.vault_path.trim();
    if vault.is_empty() {
        anyhow::bail!("Vault path is empty")
    }

    let file_name = format!(
        "{}.md",
        format_daily_file_name(now, &settings.daily_file_date_format)
    );
    Ok(PathBuf::from(vault)
        .join(settings.daily_folder_name.trim())
        .join(file_name))
}

pub fn inbox_file_path(now: DateTime<Local>, settings: &AppSettings) -> Result<PathBuf> {
    let vault = settings.vault_path.trim();
    if vault.is_empty() {
        anyhow::bail!("Vault path is empty")
    }

    let inbox_folder = if settings.inbox_folder_name.trim().is_empty() {
        "inbox"
    } else {
        settings.inbox_folder_name.trim()
    };

    let directory = PathBuf::from(vault).join(inbox_folder);
    let base_name = format!("trace-{}", now.format("%Y%m%d-%H%M%S-%3f"));
    Ok(next_available_markdown_path(&directory, &base_name))
}

fn insert_entry(content: &str, text: &str, section_title: &str, now: DateTime<Local>) -> String {
    let header = format!("# {}", section_title);
    let entry = format!("```\n{}\n{}\n```\n\n", text, now.format("%Y-%m-%d %H:%M"));

    if let Some(start) = content.find(&header) {
        let after_header = start + header.len();
        let suffix = &content[after_header..];
        let insert_pos = if let Some(line_break_offset) = suffix.find('\n') {
            after_header + line_break_offset + 1
        } else {
            content.len()
        };

        let mut output = String::with_capacity(content.len() + entry.len());
        output.push_str(&content[..insert_pos]);
        output.push_str(&entry);
        output.push_str(&content[insert_pos..]);
        return output;
    }

    if content.trim().is_empty() {
        return format!("{}\n{}", header, entry);
    }

    let mut output = content.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push('\n');
    output.push_str(&header);
    output.push('\n');
    output.push_str(&entry);
    output
}

fn format_daily_file_name(now: DateTime<Local>, swift_style_format: &str) -> String {
    let fmt = normalize_swift_date_format(swift_style_format);
    now.format(&fmt).to_string()
}

fn normalize_swift_date_format(input: &str) -> String {
    // Supported subset used by existing mac app: yyyy M d MM dd EEEE HH mm
    let mut output = input.to_string();
    let replacements = [
        ("yyyy", "%Y"),
        ("MM", "%m"),
        ("dd", "%d"),
        ("EEEE", "%A"),
        ("HH", "%H"),
        ("mm", "%M"),
        ("M", "%-m"),
        ("d", "%-d"),
    ];

    for (source, target) in replacements {
        output = output.replace(source, target);
    }
    output
}

fn standalone_file_content(text: &str, section_title: &str, now: DateTime<Local>) -> String {
    let escaped_section = section_title.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "---\nsection: \"{}\"\ncreated: \"{}\"\n---\n\n{}\n",
        escaped_section,
        now.format("%Y-%m-%d %H:%M"),
        text
    )
}

fn next_available_markdown_path(directory: &Path, base_name: &str) -> PathBuf {
    let mut candidate = directory.join(format!("{}.md", base_name));
    let mut sequence = 2;

    while candidate.exists() {
        candidate = directory.join(format!("{}-{}.md", base_name, sequence));
        sequence += 1;
    }

    candidate
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::{Local, TimeZone};
    use tempfile::tempdir;

    use crate::settings::AppSettings;

    use super::{daily_file_path, save_note};
    use crate::model::NoteSection;
    use crate::settings::NoteWriteMode;

    #[test]
    fn save_note_creates_target_file_and_header() {
        let dir = tempdir().expect("tempdir");
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().display().to_string();
        settings.daily_folder_name = "Daily".to_string();
        settings.daily_file_date_format = "yyyy-MM-dd".to_string();

        let now = Local.with_ymd_and_hms(2026, 2, 28, 18, 35, 0).unwrap();

        let file_path =
            save_note(now, "hello world", NoteSection::Clip, &settings).expect("save success");

        let content = std::fs::read_to_string(&file_path).expect("read file");
        assert!(content.contains("# Clip"));
        assert!(content.contains("hello world"));
        assert!(Path::new(&file_path).exists());
    }

    #[test]
    fn save_note_appends_under_existing_header() {
        let dir = tempdir().expect("tempdir");
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().display().to_string();
        settings.daily_folder_name = "Daily".to_string();
        settings.daily_file_date_format = "yyyy-MM-dd".to_string();

        let now = Local.with_ymd_and_hms(2026, 2, 28, 18, 35, 0).unwrap();
        let file = daily_file_path(now, &settings).expect("path");
        std::fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
        std::fs::write(&file, "# Note\n\n# Clip\n").expect("seed");

        save_note(now, "second clip", NoteSection::Clip, &settings).expect("save success");

        let content = std::fs::read_to_string(&file).expect("read");
        assert!(content.contains("# Clip\n```\nsecond clip"));
    }

    #[test]
    fn todo_title_migrates_to_project() {
        let mut settings = AppSettings::default();
        settings.section_titles[4] = "TODO".to_string();
        settings.normalize();
        assert_eq!(settings.section_titles[4], "Project");
    }

    #[test]
    fn custom_date_format_translation() {
        let dir = tempdir().expect("tempdir");
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().display().to_string();
        settings.daily_folder_name = "Daily".to_string();
        settings.daily_file_date_format = "yyyy M月d日 EEEE".to_string();

        let now = Local.with_ymd_and_hms(2026, 2, 28, 18, 35, 0).unwrap();
        let path = daily_file_path(now, &settings).expect("path");

        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("2026 2月28日"));
    }

    #[test]
    fn file_mode_creates_standalone_markdown_in_inbox() {
        let dir = tempdir().expect("tempdir");
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().display().to_string();
        settings.note_write_mode = NoteWriteMode::File;
        settings.inbox_folder_name = "inbox".to_string();

        let now = Local.with_ymd_and_hms(2026, 3, 5, 10, 34, 0).unwrap();
        let path = save_note(now, "quick note", NoteSection::Project, &settings).expect("save");

        assert!(path.starts_with(dir.path().join("inbox")));
        assert_eq!(path.extension().and_then(|v| v.to_str()), Some("md"));
        let content = std::fs::read_to_string(path).expect("read file");
        assert!(content.contains("section: \"Project\""));
        assert!(content.contains("created: \"2026-03-05 10:34\""));
        assert!(content.contains("quick note"));
    }

    #[test]
    fn file_mode_uses_unique_name_when_collision_happens() {
        let dir = tempdir().expect("tempdir");
        let mut settings = AppSettings::default();
        settings.vault_path = dir.path().display().to_string();
        settings.note_write_mode = NoteWriteMode::File;
        settings.inbox_folder_name = "inbox".to_string();

        let now = Local.with_ymd_and_hms(2026, 3, 5, 10, 34, 0).unwrap();
        let first = save_note(now, "doc A", NoteSection::Note, &settings).expect("save first");
        let second = save_note(now, "doc B", NoteSection::Note, &settings).expect("save second");

        assert_ne!(first, second);
        let inbox = dir.path().join("inbox");
        let files: Vec<_> = std::fs::read_dir(inbox).expect("read dir").collect();
        assert_eq!(files.len(), 2);
    }
}
