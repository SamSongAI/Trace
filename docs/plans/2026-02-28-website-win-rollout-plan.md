# FlashNote Website + Windows Rollout Plan (2026-02-28)

## Goal
- Launch a public website with download entries for macOS and Windows.
- Keep the current product promise: low-friction Markdown capture first.
- Start Windows delivery without blocking current macOS momentum.

## Why this sequencing
- Website can ship immediately and become the stable distribution surface.
- Windows app cannot be delivered by simply reusing current macOS Swift code.
- A phased route reduces rewrite risk and keeps weekly release cadence.

## Release Surface (Now)
- `website/` serves as the official landing + download page.
- `website/release-data.js` is the single release manifest to update version/date/artifacts.
- `scripts/package-downloads.sh` packages `dist/FlashNote.app` into a downloadable macOS zip.

## Windows Approach Options

### Option A: Full native Windows app (.NET/WPF/WinUI)
- Pros: deeply native feel, strong OS integration.
- Cons: separate codebase and duplicated logic; slower to first public build.

### Option B: Electron/Tauri desktop app with shared writer core
- Pros: faster cross-platform delivery, reuse roadmap for future Linux if needed.
- Cons: UI parity work and runtime packaging complexity.

### Option C: Keep macOS only for now, add waitlist
- Pros: no engineering risk.
- Cons: misses near-term market signal from Windows demand.

## Recommendation
- Choose **Option B** for speed-to-market and maintainability.
- Build Windows MVP with Tauri + lightweight frontend shell.
- Keep Markdown write rules identical to macOS behavior.

## Windows MVP Scope
- Global hotkey to open capture window.
- Section switching (same 5 sections as macOS).
- Save to Daily Markdown with timestamp.
- Settings: vault path, daily folder, date format, hotkey.
- No embedded AI in app.

## Milestones
1. **Week 1**: project skeleton + settings persistence + file writer.
2. **Week 2**: capture UI + hotkey + tray integration.
3. **Week 3**: packaging/signing + private beta + telemetry-free crash logs.
4. **Week 4**: public Windows download switch from `planned` to `available` in `website/release-data.js`.

## Definition of Done (Windows launch)
- Stable install package (`FlashNote-win-x64.zip` or installer).
- Daily write success >= 99% in beta cohort.
- No data loss reports in 7-day beta run.
- Website download entry live with checksum.
