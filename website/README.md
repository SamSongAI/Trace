# SOTA SYNC Website

## Structure
- `index.html`: Generated SOTA SYNC homepage
- `resources.html`: Generated resource hub for curated links and short commentary
- `blog/index.html`: Generated blog landing page and article index
- `blog.html`: Redirect shim to `/blog/`
- `blog/*.html`: Generated blog detail pages
- `readme.html`: One-person company / site operating note
- `product.html`: Trace product overview
- `download.html`: Trace download center for macOS / Windows
- `changelog.html`: Trace release history
- `roadmap.html`: Trace public roadmap
- `pricing.html`: Trace commercial plans
- `privacy.html`: Trace privacy policy draft
- `terms.html`: Trace terms draft
- `release-data.js`: Website release source of truth
- `site.css`: Shared styles
- `site.js`: Shared interactive logic

## Content admin
- 内容源目录：`content/`
- 后台应用：`cms/`
- 后台运行说明：`docs/content-admin.md`
- 受后台管理的公开页面：`index.html`、`resources.html`、`blog/index.html`、`blog/*.html`
- 仍保持手工维护的页面：`product.html`、`readme.html`、`download.html`、`pricing.html`、`changelog.html`、`roadmap.html`

## Local preview
```bash
./scripts/serve-website.sh
# or custom port
./scripts/serve-website.sh 9090
```

启动内容后台：

```bash
python3 -m pip install -r cms/requirements.txt
export ADMIN_PASSWORD_HASH='pbkdf2:sha256:...'
export SESSION_SECRET='your-long-random-secret'
./scripts/run-admin.sh
```

构建后台管理页面到临时预览目录：

```bash
./scripts/build-content-site.sh
```

## Release update checklist
1. Build macOS release DMG:
```bash
./scripts/trace.sh build-dmg
```
2. Package download artifact:
```bash
./scripts/package-downloads.sh
```
3. Update `website/release-data.js`:
- `current.version`
- `current.releasedAt`
- `current.releaseTitle`
- `current.notes`
- `current.platforms.macos.sha256`
- `current.platforms.windows.status/url/sha256`
4. Append release item to `history`.
5. 如果改的是 Blog / Resources / 首页精选，优先通过后台发布或执行 `./scripts/publish-content-site.sh`。
6. Deploy `website/` to static hosting.

## Deploy options
- Cloudflare Pages
- Vercel
- Netlify
- GitHub Pages

Any static host that serves plain HTML/CSS/JS works.
