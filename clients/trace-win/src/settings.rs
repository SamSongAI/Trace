use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::model::NoteSection;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoteWriteMode {
    #[default]
    Dimension,
    File,
}

impl NoteWriteMode {
    pub fn title(self) -> &'static str {
        match self {
            NoteWriteMode::Dimension => "Dimension (Daily)",
            NoteWriteMode::File => "File (Inbox)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub vault_path: String,
    pub note_write_mode: NoteWriteMode,
    pub daily_folder_name: String,
    pub inbox_folder_name: String,
    pub daily_file_date_format: String,
    pub section_titles: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            vault_path: String::new(),
            note_write_mode: NoteWriteMode::Dimension,
            daily_folder_name: "Daily".to_string(),
            inbox_folder_name: "inbox".to_string(),
            daily_file_date_format: "yyyy M月d日 EEEE".to_string(),
            section_titles: default_section_titles(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Result<Self> {
        let path = settings_file_path()?;
        migrate_legacy_settings_if_needed(&path)?;
        match fs::read_to_string(&path) {
            Ok(raw) => {
                let mut parsed: AppSettings = serde_json::from_str(&raw)
                    .with_context(|| format!("Failed to parse settings at {}", path.display()))?;
                parsed.normalize();
                Ok(parsed)
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => {
                Err(err).with_context(|| format!("Failed to read settings at {}", path.display()))
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let mut normalized = self.clone();
        normalized.normalize();

        let path = settings_file_path()?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create settings dir {}", dir.display()))?;
        }

        let json = serde_json::to_string_pretty(&normalized)?;
        let mut file = fs::File::create(&path)
            .with_context(|| format!("Failed to create settings file {}", path.display()))?;
        file.write_all(json.as_bytes())
            .with_context(|| format!("Failed to write settings file {}", path.display()))?;
        Ok(())
    }

    pub fn title_for(&self, section: NoteSection) -> &str {
        self.section_titles
            .get(section.index())
            .map(String::as_str)
            .unwrap_or_else(|| section.default_title())
    }

    pub fn set_title_for(&mut self, section: NoteSection, value: String) {
        let idx = section.index();
        if self.section_titles.len() <= idx {
            self.section_titles = default_section_titles();
        }
        self.section_titles[idx] = normalize_title(&value, section.default_title());
    }

    pub fn normalize(&mut self) {
        if self.daily_folder_name.trim().is_empty() {
            self.daily_folder_name = "Daily".to_string();
        }

        if self.inbox_folder_name.trim().is_empty() {
            self.inbox_folder_name = "inbox".to_string();
        }

        if self.daily_file_date_format.trim().is_empty() {
            self.daily_file_date_format = "yyyy M月d日 EEEE".to_string();
        }

        let defaults = default_section_titles();
        let mut normalized = Vec::with_capacity(defaults.len());
        for (idx, fallback) in defaults.iter().enumerate() {
            let title = self
                .section_titles
                .get(idx)
                .map(|it| normalize_title(it, fallback))
                .unwrap_or_else(|| fallback.clone());
            normalized.push(title);
        }
        self.section_titles = normalized;
    }
}

fn default_section_titles() -> Vec<String> {
    NoteSection::ALL
        .iter()
        .map(|it| it.default_title().to_string())
        .collect()
}

fn normalize_title(input: &str, fallback: &str) -> String {
    let trimmed = input.trim().trim_start_matches('#').trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else if trimmed.eq_ignore_ascii_case("todo") {
        "Project".to_string()
    } else {
        trimmed.to_string()
    }
}

fn settings_file_path() -> Result<PathBuf> {
    project_settings_file_path("Trace", "Trace")
}

fn legacy_settings_file_path() -> Result<PathBuf> {
    project_settings_file_path("FlashNote", "FlashNote")
}

fn project_settings_file_path(organization: &str, application: &str) -> Result<PathBuf> {
    let dirs = ProjectDirs::from("app", organization, application)
        .context("Cannot resolve config directory")?;
    Ok(dirs.config_dir().join("settings.json"))
}

fn migrate_legacy_settings_if_needed(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let legacy_path = match legacy_settings_file_path() {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };

    migrate_settings_file(path, &legacy_path)
}

fn migrate_settings_file(path: &Path, legacy_path: &Path) -> Result<()> {
    if path.exists() || !legacy_path.exists() {
        return Ok(());
    }

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create settings dir {}", dir.display()))?;
    }

    fs::copy(legacy_path, path).with_context(|| {
        format!(
            "Failed to migrate legacy settings from {} to {}",
            legacy_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrate_settings_file_copies_legacy_config_once() {
        let temp = tempdir().expect("temp dir");
        let trace_path = temp.path().join("trace/settings.json");
        let legacy_path = temp.path().join("flashnote/settings.json");

        fs::create_dir_all(legacy_path.parent().expect("legacy dir")).expect("legacy dir");
        fs::write(&legacy_path, "{\"vault_path\":\"/tmp/vault\"}").expect("legacy config");

        migrate_settings_file(&trace_path, &legacy_path).expect("migrate legacy config");

        assert_eq!(
            fs::read_to_string(&trace_path).expect("trace config"),
            "{\"vault_path\":\"/tmp/vault\"}"
        );

        fs::write(&trace_path, "{\"vault_path\":\"/tmp/new\"}").expect("new config");
        migrate_settings_file(&trace_path, &legacy_path).expect("skip overwrite");
        assert_eq!(
            fs::read_to_string(&trace_path).expect("trace config"),
            "{\"vault_path\":\"/tmp/new\"}"
        );
    }
}
