# Downloads Directory

Put signed release archives here.

Expected file names:
- `Trace-macOS-universal.zip`
- `Trace-win-x64.zip`

Windows note:
- CI artifact is produced by `.github/workflows/windows-build.yml`.
- Publish the first signed Windows package into this folder before switching website status to `available`.

Then update `website/release-data.js`:
- `current.version`
- `current.releasedAt`
- `current.platforms.macos.sha256`
- `current.platforms.windows.status` to `available` when win build is ready
