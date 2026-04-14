# Changelog

This file tracks shipped changes for public Trace releases.

本文件记录 Trace 对外发布版本的主要更新，不逐条罗列所有 commit，只保留用户真正会感知到的变化。

## [1.0.3] - 2026-04-14

Release: [Trace v1.0.3](https://github.com/SamSongAI/Trace/releases/tag/v1.0.3)

### Added

- Thread mode is included in the current macOS package. You can now append notes by topic into a chosen thread file for continuous tracking.

### Fixed

- Fixed the issue where the capture panel could fail to appear in the current full-screen or non-desktop Space after invoking the hotkey.
- Brightened the dark theme placeholder text to improve readability and restore test coverage.
- Updated the packaging flow so `build-dmg` always rebuilds the latest app bundle before creating a new DMG.

## [1.0.2] - 2026-04-07

Release: [Trace v1.0.2](https://github.com/SamSongAI/Trace/releases/tag/v1.0.2)

### Changed

- Tuned the macOS DMG window layout.
- Enlarged the DMG window.
- Reduced icon size and adjusted the positions of `Trace.app` and `Applications` for a cleaner install flow.

## [1.0.1] - 2026-04-07

Release: [Trace v1.0.1](https://github.com/SamSongAI/Trace/releases/tag/v1.0.1)

### Added

- Added drag-to-Applications installation flow in the DMG.

### Changed

- The DMG now includes an `Applications` shortcut.
- Opening the DMG shows the direct install path for dragging `Trace.app` into `Applications`.

## [1.0.0] - 2026-04-03

Release: [Trace v1.0.0](https://github.com/SamSongAI/Trace/releases/tag/v1.0.0)

### Added

- First public macOS release of Trace.
- Distributed as a local-first `Trace.dmg` package for macOS 13+.
- Global hotkey capture panel and direct local Markdown writing workflow.
