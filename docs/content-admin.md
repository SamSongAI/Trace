# SOTA SYNC Content Admin

## What It Controls

- `Blog`: Markdown-first long-form essays
- `Resources`: external link cards plus your short commentary
- Homepage featured slots for Blog and Resources

`Product`、`Read Me`、下载/定价/路线图等页面仍然保持手工维护，不经过后台。

## Runtime

- Flask admin app on `127.0.0.1:4310`
- Static public site stays in [`website`](/Users/apple/Desktop/Sam%20Project/Project2026/FlashNote/website)
- Content source lives in [`content`](/Users/apple/Desktop/Sam%20Project/Project2026/FlashNote/content)
- Publish backups default to `.content-admin/backups/`

## Environment

Set these before starting the admin:

```bash
export ADMIN_PASSWORD_HASH='pbkdf2:sha256:...'
export SESSION_SECRET='a-long-random-secret'
```

Optional:

```bash
export ADMIN_HOST='127.0.0.1'
export ADMIN_PORT='4310'
export SITE_BASE_URL='https://sotasync.com'
export ADMIN_COOKIE_SECURE='1'
export PUBLIC_SITE_DIR='/var/www/flashnote'
export CONTENT_ROOT='/var/www/sotasync-content'
export BACKUP_ROOT='/var/backups/sotasync'
```

生成密码哈希：

```bash
python3 -m cms.app hash-password --password 'your-admin-password'
```

## Local Commands

Install Python dependencies:

```bash
python3 -m pip install -r cms/requirements.txt
```

Run the admin:

```bash
./scripts/run-admin.sh
```

Build the managed public pages into a temp preview directory:

```bash
./scripts/build-content-site.sh
```

Publish the managed pages into the live public root with backup:

```bash
./scripts/publish-content-site.sh
```

## Publish Behavior

- Save draft: writes Markdown/frontmatter to the content library only
- Preview: renders the public template behind auth
- Publish: validates content, creates a full-site backup, rebuilds managed pages in a staging directory, then swaps the site atomically
- Unpublish: flips the entry back to `draft` and republishes the site so the public entry disappears
