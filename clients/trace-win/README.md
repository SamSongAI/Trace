# Trace for Windows (MVP)

Windows desktop client for Trace, focused on Markdown-native capture.

## Implemented
- 5 sections: `Note / Clip / Link / Task / Project`
- Section switching with `Ctrl+1..Ctrl+5`
- Save with `Ctrl+Enter`
- Daily Markdown writing with section headers and timestamp code blocks
- Settings persistence (`vault_path`, `daily_folder_name`, `daily_file_date_format`, section titles)
- TODO title migration to Project

## Not yet in this MVP
- Global system hotkey (outside app focus)
- Tray icon behavior
- Signed installer

## Local run
```bash
cd clients/trace-win
cargo run
```

## Local tests
```bash
cd clients/trace-win
cargo test
```

## Windows build in CI
- Workflow: `.github/workflows/windows-build.yml`
- Artifact output: `Trace-win-x64.zip`
