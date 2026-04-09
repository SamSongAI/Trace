# FlashNote Website Launch Playbook (2026-02-28)

## Goal
Ship a production-ready product website that supports ongoing binary releases for macOS now and Windows later.

## What is considered done
- Multi-page website is live.
- macOS download artifact and checksum are publicly available.
- Windows slot is visible and can be switched to active without redesign.
- Legal pages (privacy/terms draft) are available.

## Hosting setup
1. Use static hosting provider (Cloudflare Pages / Vercel / Netlify / GitHub Pages).
2. Publish from repository folder: `website/`.
3. Set custom domain (example: `flashnote.app`).
4. Enable HTTPS and force redirect to HTTPS.

## Release operations
1. Build app bundle:
```bash
./scripts/flashnote.sh build-app
```
2. Create download archive:
```bash
./scripts/package-downloads.sh
```
3. Update website release metadata in `website/release-data.js`.
4. Commit new artifact in `website/downloads/` and metadata updates.
5. Deploy website changes.

## Windows activation procedure
When Windows package is ready:
1. Put file in `website/downloads/` (e.g. `FlashNote-win-x64.zip`).
2. Compute and fill SHA256 in `release-data.js`.
3. Change `current.platforms.windows.status` to `available`.
4. Set `current.platforms.windows.url` to the artifact path.
5. Deploy website.

## Risk controls
- Never publish package without checksum.
- Keep previous package in release history for rollback.
- Validate download links before each deployment.
