from __future__ import annotations

import re
from dataclasses import replace
from datetime import datetime
from pathlib import Path
from typing import Any

import yaml

from .models import ContentEntry, HomepageConfig, VALID_STATUS


SLUG_RE = re.compile(r"[^a-z0-9-]+")


def slugify(text: str) -> str:
    base = text.strip().lower()
    base = base.replace("_", "-")
    base = re.sub(r"\s+", "-", base)
    base = SLUG_RE.sub("-", base)
    base = re.sub(r"-{2,}", "-", base).strip("-")
    return base


def parse_frontmatter(raw: str) -> tuple[dict[str, Any], str]:
    if not raw.startswith("---\n"):
        return {}, raw
    parts = raw.split("\n---\n", 1)
    if len(parts) != 2:
        return {}, raw
    meta_raw = parts[0][4:]
    body = parts[1]
    data = yaml.safe_load(meta_raw) or {}
    if not isinstance(data, dict):
        raise ValueError("frontmatter must be a mapping")
    return data, body.lstrip("\n")


def dump_frontmatter(data: dict[str, Any], body: str) -> str:
    frontmatter = yaml.safe_dump(data, sort_keys=False, allow_unicode=True).strip()
    return f"---\n{frontmatter}\n---\n\n{body.rstrip()}\n"


def normalize_tags(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        items = value
    else:
        items = str(value).split(",")
    normalized = []
    for item in items:
        text = str(item).strip()
        if text:
            normalized.append(text)
    return normalized


def validate_iso_date(raw: str) -> bool:
    if not raw:
        return False
    try:
        datetime.strptime(raw, "%Y-%m-%d")
    except ValueError:
        return False
    return True


class ContentStore:
    def __init__(self, root: Path):
        self.root = root
        self.blog_root = root / "blog"
        self.resource_root = root / "resources"
        self.config_root = root / "config"
        self.homepage_path = self.config_root / "homepage.yaml"
        for path in (self.blog_root, self.resource_root, self.config_root):
            path.mkdir(parents=True, exist_ok=True)

    def _kind_root(self, kind: str) -> Path:
        if kind == "blog":
            return self.blog_root
        if kind == "resources":
            return self.resource_root
        raise ValueError(f"unsupported content kind: {kind}")

    def _entry_path(self, kind: str, slug: str) -> Path:
        return self._kind_root(kind) / f"{slug}.md"

    def list_entries(self, kind: str) -> list[ContentEntry]:
        root = self._kind_root(kind)
        entries = [self.load_path(kind, path) for path in sorted(root.glob("*.md"))]
        entries.sort(key=lambda entry: entry.published_sort_key, reverse=True)
        return entries

    def published_entries(self, kind: str) -> list[ContentEntry]:
        return [entry for entry in self.list_entries(kind) if entry.is_published]

    def load_path(self, kind: str, path: Path) -> ContentEntry:
        meta, body = parse_frontmatter(path.read_text(encoding="utf-8"))
        stat = path.stat()
        return ContentEntry(
            kind=kind,
            title=str(meta.get("title", "")),
            slug=str(meta.get("slug", path.stem)),
            excerpt=str(meta.get("excerpt", "")),
            seo_description=str(meta.get("seo_description", "")),
            published_at=str(meta.get("published_at", "")),
            status=str(meta.get("status", "draft")),
            tags=normalize_tags(meta.get("tags")),
            external_url=str(meta.get("external_url", "")),
            source_name=str(meta.get("source_name", "")),
            summary=str(meta.get("summary", "")),
            body_markdown=body,
            source_path=path,
            updated_at=datetime.fromtimestamp(stat.st_mtime),
        )

    def load_entry(self, kind: str, slug: str) -> ContentEntry | None:
        path = self._entry_path(kind, slug)
        if not path.exists():
            return None
        return self.load_path(kind, path)

    def save_entry(self, kind: str, payload: dict[str, Any], original_slug: str | None = None) -> ContentEntry:
        title = str(payload.get("title", "")).strip()
        raw_slug = str(payload.get("slug", "")).strip()
        slug = slugify(raw_slug or title)
        if not slug:
            raise ValueError("slug is required")

        entry = ContentEntry(
            kind=kind,
            title=title,
            slug=slug,
            excerpt=str(payload.get("excerpt", "")).strip(),
            seo_description=str(payload.get("seo_description", "")).strip(),
            published_at=str(payload.get("published_at", "")).strip(),
            status=str(payload.get("status", "draft")).strip() or "draft",
            tags=normalize_tags(payload.get("tags")),
            external_url=str(payload.get("external_url", "")).strip(),
            source_name=str(payload.get("source_name", "")).strip(),
            summary=str(payload.get("summary", "")).strip(),
            body_markdown=str(payload.get("body_markdown", "")).strip(),
        )

        conflict = self.load_entry(kind, slug)
        if conflict and slug != (original_slug or ""):
            raise ValueError(f"slug '{slug}' already exists")

        errors = self.validate_entry(entry, for_publish=False)
        if errors:
            raise ValueError(errors[0])

        path = self._entry_path(kind, slug)
        data = self.to_frontmatter(entry)
        path.write_text(dump_frontmatter(data, entry.body_markdown), encoding="utf-8")

        if original_slug and original_slug != slug:
            old_path = self._entry_path(kind, original_slug)
            if old_path.exists():
                old_path.unlink()

        return self.load_entry(kind, slug) or entry

    def validate_entry(self, entry: ContentEntry, for_publish: bool) -> list[str]:
        errors: list[str] = []
        if not entry.title:
            errors.append("title is required")
        if not entry.slug:
            errors.append("slug is required")
        if entry.status not in VALID_STATUS:
            errors.append("status must be draft or published")
        if for_publish or entry.status == "published":
            if not validate_iso_date(entry.published_at):
                errors.append("published_at must use YYYY-MM-DD")

        if entry.kind == "blog":
            if for_publish or entry.status == "published":
                if not entry.excerpt:
                    errors.append("excerpt is required for blog publish")
                if not entry.seo_description:
                    errors.append("seo_description is required for blog publish")
            if not entry.body_markdown:
                errors.append("body markdown is required")
        elif entry.kind == "resources":
            if for_publish or entry.status == "published":
                if not entry.external_url:
                    errors.append("external_url is required for resource publish")
                if not entry.source_name:
                    errors.append("source_name is required for resource publish")
                if not entry.summary:
                    errors.append("summary is required for resource publish")
            if not entry.body_markdown:
                errors.append("resource short comment is required")
        else:
            errors.append(f"unsupported kind: {entry.kind}")
        return errors

    def to_frontmatter(self, entry: ContentEntry) -> dict[str, Any]:
        data: dict[str, Any] = {
            "title": entry.title,
            "slug": entry.slug,
            "published_at": entry.published_at,
            "status": entry.status,
            "tags": entry.tags,
        }
        if entry.kind == "blog":
            data["excerpt"] = entry.excerpt
            data["seo_description"] = entry.seo_description
        else:
            data["external_url"] = entry.external_url
            data["source_name"] = entry.source_name
            data["summary"] = entry.summary
        return data

    def load_homepage_config(self) -> HomepageConfig:
        if not self.homepage_path.exists():
            return HomepageConfig()
        data = yaml.safe_load(self.homepage_path.read_text(encoding="utf-8")) or {}
        return HomepageConfig(
            featured_blog_slugs=[str(item).strip() for item in data.get("featured_blog_slugs", []) if str(item).strip()],
            featured_resource_slugs=[str(item).strip() for item in data.get("featured_resource_slugs", []) if str(item).strip()],
        )

    def save_homepage_config(self, featured_blog_slugs: list[str], featured_resource_slugs: list[str]) -> HomepageConfig:
        config = HomepageConfig(
            featured_blog_slugs=[slugify(item) for item in featured_blog_slugs if slugify(item)],
            featured_resource_slugs=[slugify(item) for item in featured_resource_slugs if slugify(item)],
        )
        payload = {
            "featured_blog_slugs": config.featured_blog_slugs,
            "featured_resource_slugs": config.featured_resource_slugs,
        }
        self.homepage_path.write_text(yaml.safe_dump(payload, sort_keys=False, allow_unicode=True), encoding="utf-8")
        return config

    def copy_entry(self, entry: ContentEntry) -> ContentEntry:
        return replace(entry, tags=list(entry.tags), toc=list(entry.toc))
