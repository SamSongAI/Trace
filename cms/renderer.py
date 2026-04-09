from __future__ import annotations

import math
import re
from html import unescape
from dataclasses import replace
from pathlib import Path
from typing import Any

import markdown
from jinja2 import Environment, FileSystemLoader, select_autoescape
from markdown.extensions.toc import slugify_unicode

try:
    import bleach
except ImportError:  # pragma: no cover - dependency declared in requirements
    bleach = None

from .config import Settings
from .content import ContentStore
from .models import ContentEntry, HomepageConfig, TocItem


ALLOWED_TAGS = [
    "a",
    "blockquote",
    "br",
    "code",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "hr",
    "li",
    "ol",
    "p",
    "pre",
    "strong",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "ul",
]
ALLOWED_ATTRIBUTES = {
    "a": ["href", "title", "target", "rel"],
    "h1": ["id"],
    "h2": ["id"],
    "h3": ["id"],
    "h4": ["id"],
    "h5": ["id"],
}


def compute_reading_time(text: str) -> int:
    latin_words = re.findall(r"[A-Za-z0-9_]+", text)
    cjk_chars = re.findall(r"[\u4e00-\u9fff]", text)
    units = len(latin_words) + len(cjk_chars)
    return max(1, math.ceil(units / 320))


def sanitize_html(html: str) -> str:
    if bleach is None:
        return html
    return bleach.clean(
        html,
        tags=ALLOWED_TAGS,
        attributes=ALLOWED_ATTRIBUTES,
        protocols=["http", "https", "mailto"],
        strip=True,
    )


def strip_leading_title(body: str) -> str:
    normalized = body.lstrip("\n")
    lines = normalized.splitlines()
    if not lines:
        return body
    first = lines[0].strip()
    if not first.startswith("# ") or first.startswith("##"):
        return body
    remainder = lines[1:]
    while remainder and not remainder[0].strip():
        remainder = remainder[1:]
    return "\n".join(remainder)


def render_markdown(body: str) -> tuple[str, list[TocItem], int]:
    prepared_body = strip_leading_title(body)
    md = markdown.Markdown(
        extensions=["extra", "tables", "toc", "sane_lists"],
        extension_configs={
            "toc": {
                "slugify": slugify_unicode,
                "permalink": False,
                "toc_depth": "2-3",
            }
        },
        output_format="html5",
    )
    rendered = md.convert(prepared_body)
    toc_tokens = getattr(md, "toc_tokens", []) or []
    toc_items: list[TocItem] = []

    def visit(nodes: list[dict[str, Any]]) -> None:
        for node in nodes:
            level = int(node.get("level", 0))
            if level in {2, 3}:
                toc_items.append(
                    TocItem(
                        level=level,
                        title=unescape(str(node.get("name", ""))),
                        anchor=str(node.get("id", "")),
                    )
                )
            children = node.get("children") or []
            if isinstance(children, list):
                visit(children)

    visit(toc_tokens)
    return sanitize_html(rendered), toc_items, compute_reading_time(prepared_body)


class SiteRenderer:
    def __init__(self, settings: Settings):
        self.settings = settings
        template_root = Path(__file__).resolve().parent / "templates"
        self.env = Environment(
            loader=FileSystemLoader(template_root),
            autoescape=select_autoescape(["html", "xml"]),
            trim_blocks=True,
            lstrip_blocks=True,
        )

    def decorate_entry(self, entry: ContentEntry) -> ContentEntry:
        body_html, toc, reading_time = render_markdown(entry.body_markdown)
        return replace(entry, body_html=body_html, toc=toc, reading_time_minutes=reading_time)

    def build_site_context(self, store: ContentStore) -> dict[str, Any]:
        blog_entries = [self.decorate_entry(entry) for entry in store.published_entries("blog")]
        resource_entries = [self.decorate_entry(entry) for entry in store.published_entries("resources")]
        blog_entries.sort(key=lambda entry: entry.published_sort_key, reverse=True)
        resource_entries.sort(key=lambda entry: entry.published_sort_key, reverse=True)

        homepage = store.load_homepage_config()
        featured_blogs = self._select_featured(homepage.featured_blog_slugs, blog_entries, limit=3)
        featured_resources = self._select_featured(homepage.featured_resource_slugs, resource_entries, limit=6)
        if not featured_blogs:
            featured_blogs = blog_entries[:3]
        if not featured_resources:
            featured_resources = resource_entries[:6]

        return {
            "blogs": blog_entries,
            "resources": resource_entries,
            "homepage": homepage,
            "featured_blogs": featured_blogs,
            "featured_resources": featured_resources,
            "site_base_url": self.settings.site_base_url,
        }

    def _select_featured(
        self,
        selected_slugs: list[str],
        entries: list[ContentEntry],
        limit: int,
    ) -> list[ContentEntry]:
        mapping = {entry.slug: entry for entry in entries}
        selected: list[ContentEntry] = []
        for slug in selected_slugs:
            entry = mapping.get(slug)
            if entry:
                selected.append(entry)
        return selected[:limit]

    def render_template(self, name: str, **context: Any) -> str:
        template = self.env.get_template(name)
        return template.render(**context)

    def render_homepage(self, context: dict[str, Any], is_preview: bool = False) -> str:
        return self.render_template(
            "public/home.html",
            active_nav="home",
            footer_copy="© 2026 SOTA SYNC. Frontier agent curation with a durable product edge.",
            is_preview=is_preview,
            **context,
        )

    def render_blog_index(self, context: dict[str, Any], is_preview: bool = False) -> str:
        return self.render_template(
            "public/blog_index.html",
            active_nav="blog",
            footer_copy="Long-form writing on frontier agent systems.",
            is_preview=is_preview,
            **context,
        )

    def render_blog_post(self, entry: ContentEntry, is_preview: bool = False) -> str:
        return self.render_template(
            "public/blog_post.html",
            active_nav="blog",
            footer_copy=f"{entry.title} at SOTA SYNC Blog.",
            entry=entry,
            is_preview=is_preview,
            site_base_url=self.settings.site_base_url,
        )

    def render_resources(
        self,
        context: dict[str, Any],
        preview_entry: ContentEntry | None = None,
        is_preview: bool = False,
    ) -> str:
        return self.render_template(
            "public/resources.html",
            active_nav="resources",
            footer_copy="Resources are the long-term memory layer of SOTA SYNC.",
            preview_entry=preview_entry,
            is_preview=is_preview,
            **context,
        )

    def write_public_site(self, context: dict[str, Any], output_root: Path) -> None:
        blog_root = output_root / "blog"
        if blog_root.exists():
            for path in blog_root.glob("*.html"):
                path.unlink()
        blog_root.mkdir(parents=True, exist_ok=True)

        (output_root / "index.html").write_text(self.render_homepage(context), encoding="utf-8")
        (output_root / "resources.html").write_text(self.render_resources(context), encoding="utf-8")
        (blog_root / "index.html").write_text(self.render_blog_index(context), encoding="utf-8")

        for entry in context["blogs"]:
            (blog_root / f"{entry.slug}.html").write_text(self.render_blog_post(entry), encoding="utf-8")
