from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from werkzeug.security import generate_password_hash

from cms.app import create_app
from cms.config import Settings
from cms.content import ContentStore


def seed_public_site(root: Path) -> None:
    (root / "blog").mkdir(parents=True, exist_ok=True)
    (root / "site.css").write_text("body { font-family: sans-serif; }", encoding="utf-8")
    (root / "blog.html").write_text("<html><body>redirect shim</body></html>", encoding="utf-8")
    (root / "product.html").write_text("<html><body>product</body></html>", encoding="utf-8")
    (root / "readme.html").write_text("<html><body>readme</body></html>", encoding="utf-8")


class AdminAppTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.content_root = self.root / "content"
        self.public_root = self.root / "website"
        self.backup_root = self.root / "backups"
        self.state_root = self.root / "state"
        seed_public_site(self.public_root)
        self.settings = Settings(
            repo_root=self.root,
            content_root=self.content_root,
            public_site_dir=self.public_root,
            backup_root=self.backup_root,
            state_root=self.state_root,
            site_base_url="https://sotasync.test",
            admin_password_hash=generate_password_hash("secret-pass"),
            session_secret="test-session-secret",
            session_cookie_secure=False,
            admin_host="127.0.0.1",
            admin_port=4310,
        )
        self.app = create_app(self.settings)
        self.app.config.update(TESTING=True)
        self.client = self.app.test_client()
        self.store = ContentStore(self.content_root)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def login(self) -> None:
        response = self.client.post(
            "/admin/login",
            data={"password": "secret-pass"},
            follow_redirects=True,
        )
        self.assertEqual(response.status_code, 200)
        self.assertIn("内容后台".encode("utf-8"), response.data)

    def test_auth_flow(self) -> None:
        redirect_response = self.client.get("/admin")
        self.assertEqual(redirect_response.status_code, 302)
        self.assertIn("/admin/login", redirect_response.headers["Location"])

        failure = self.client.post("/admin/login", data={"password": "wrong"}, follow_redirects=True)
        self.assertEqual(failure.status_code, 200)
        self.assertIn(b"invalid password", failure.data)

        self.login()

        logout = self.client.post("/admin/logout", follow_redirects=True)
        self.assertEqual(logout.status_code, 200)
        self.assertIn("登录 SOTA SYNC 内容后台".encode("utf-8"), logout.data)

    def test_save_draft_and_block_invalid_publish(self) -> None:
        self.login()
        response = self.client.post(
            "/admin/content/blog/new",
            data={
                "title": "Draft Essay",
                "slug": "",
                "published_at": "",
                "status": "draft",
                "tags": "Harness, Testing",
                "excerpt": "",
                "seo_description": "",
                "body_markdown": "## Heading\n\nBody copy.",
            },
            follow_redirects=False,
        )
        self.assertEqual(response.status_code, 302)
        self.assertIn("/admin/content/blog/draft-essay", response.headers["Location"])

        publish = self.client.post(
            "/admin/content/blog/draft-essay/publish",
            data={"action": "publish"},
            follow_redirects=True,
        )
        self.assertEqual(publish.status_code, 200)
        self.assertIn(b"excerpt is required for blog publish", publish.data)
        self.assertFalse((self.public_root / "blog" / "draft-essay.html").exists())

    def test_publish_blog_generates_public_pages(self) -> None:
        self.login()
        self.store.save_entry(
            "blog",
            {
                "title": "Harness Essay",
                "slug": "harness-essay",
                "published_at": "2026-03-17",
                "status": "draft",
                "tags": "Harness, Agent Infra",
                "excerpt": "Why Harness matters.",
                "seo_description": "Harness is the runtime moat.",
                "body_markdown": "## What changed\n\nHarness moved from optional to required.",
            },
        )

        response = self.client.post(
            "/admin/content/blog/harness-essay/publish",
            data={"action": "publish"},
            follow_redirects=True,
        )
        self.assertEqual(response.status_code, 200)
        self.assertTrue((self.public_root / "blog" / "harness-essay.html").exists())
        self.assertIn("Harness Essay", (self.public_root / "blog" / "index.html").read_text(encoding="utf-8"))
        self.assertIn("Harness Essay", (self.public_root / "index.html").read_text(encoding="utf-8"))
        self.assertTrue(any(self.backup_root.iterdir()))

    def test_publish_resource_updates_resources_page(self) -> None:
        self.login()
        self.store.save_entry(
            "resources",
            {
                "title": "Codex Harness Post",
                "slug": "codex-harness-post",
                "published_at": "2026-03-17",
                "status": "draft",
                "tags": "OpenAI, Harness",
                "external_url": "https://example.com/harness",
                "source_name": "OpenAI",
                "summary": "A reference on harness engineering.",
                "body_markdown": "This resource matters because it makes harness legible.",
            },
        )

        response = self.client.post(
            "/admin/content/resources/codex-harness-post/publish",
            data={"action": "publish"},
            follow_redirects=True,
        )
        self.assertEqual(response.status_code, 200)
        resources_html = (self.public_root / "resources.html").read_text(encoding="utf-8")
        self.assertIn("Codex Harness Post", resources_html)
        self.assertIn("https://example.com/harness", resources_html)

    def test_homepage_featured_uses_manual_order(self) -> None:
        self.login()
        self.store.save_entry(
            "blog",
            {
                "title": "First Essay",
                "slug": "first-essay",
                "published_at": "2026-03-16",
                "status": "published",
                "tags": "One",
                "excerpt": "First excerpt.",
                "seo_description": "First seo.",
                "body_markdown": "## First\n\nBody",
            },
        )
        self.store.save_entry(
            "blog",
            {
                "title": "Second Essay",
                "slug": "second-essay",
                "published_at": "2026-03-17",
                "status": "published",
                "tags": "Two",
                "excerpt": "Second excerpt.",
                "seo_description": "Second seo.",
                "body_markdown": "## Second\n\nBody",
            },
        )
        self.client.post("/admin/publish", follow_redirects=True)

        response = self.client.post(
            "/admin/homepage/featured",
            data={
                "featured_blog_slugs": ["first-essay", "second-essay"],
                "blog_order_first-essay": "2",
                "blog_order_second-essay": "1",
                "publish_now": "1",
            },
            follow_redirects=True,
        )
        self.assertEqual(response.status_code, 200)
        homepage = (self.public_root / "index.html").read_text(encoding="utf-8")
        self.assertLess(homepage.index("Second Essay"), homepage.index("First Essay"))

    def test_preview_requires_auth_and_renders(self) -> None:
        self.store.save_entry(
            "blog",
            {
                "title": "Preview Essay",
                "slug": "preview-essay",
                "published_at": "2026-03-17",
                "status": "draft",
                "tags": "Preview",
                "excerpt": "Preview excerpt.",
                "seo_description": "Preview seo.",
                "body_markdown": "# Preview Essay\n\n## Preview section\n\nPreview body.",
            },
        )
        redirect_response = self.client.get("/admin/content/blog/preview-essay/preview")
        self.assertEqual(redirect_response.status_code, 302)
        self.login()
        preview = self.client.get("/admin/content/blog/preview-essay/preview")
        self.assertEqual(preview.status_code, 200)
        self.assertIn("Preview Mode".encode("utf-8"), preview.data)
        self.assertIn("Preview Essay".encode("utf-8"), preview.data)
        self.assertEqual(preview.data.count(b"<h1>"), 1)


if __name__ == "__main__":
    unittest.main()
