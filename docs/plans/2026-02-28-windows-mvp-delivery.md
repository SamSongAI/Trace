# FlashNote Windows MVP Delivery (2026-02-28)

## Scope delivered
- New Windows client workspace: `clients/flashnote-win`.
- Desktop UI built with Rust + egui.
- Daily Markdown writer aligned with macOS behavior:
  - section-based `# Header` insertion
  - timestamp code block entry
  - folder auto-create
- Section model aligned to product defaults:
  - `Note / Clip / Link / Task / Project`
- Settings persistence:
  - vault path
  - daily folder
  - daily filename format
  - section titles
- Shortcut support inside app:
  - `Ctrl+1..5` section switch
  - `Ctrl+Enter` save

## Engineering notes
- Date format compatibility includes Swift-style patterns currently used by mac app (`yyyy M月d日 EEEE`).
- Legacy `TODO` section title is normalized to `Project`.

## Testing status
- Local unit tests: `cargo test` (4/4 passing).
- Existing mac app tests: `swift test` (3/3 passing).
- Windows CI pipeline added: `.github/workflows/windows-build.yml`.
  - Runs tests on `windows-latest`
  - Builds release binary
  - Uploads `FlashNote-win-x64.zip` artifact

## Remaining before dual-end public release
- Add global system hotkey in Windows app.
- Add tray behavior parity.
- Produce signed Windows installer.
- Publish first Windows artifact to `website/downloads` and flip website status to `available`.
