from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path


VALID_STATUS = {"draft", "published"}


@dataclass(slots=True)
class TocItem:
    level: int
    title: str
    anchor: str


@dataclass(slots=True)
class ContentEntry:
    kind: str
    title: str = ""
    slug: str = ""
    status: str = "draft"
    published_at: str = ""
    tags: list[str] = field(default_factory=list)
    body_markdown: str = ""
    excerpt: str = ""
    seo_description: str = ""
    external_url: str = ""
    source_name: str = ""
    summary: str = ""
    source_path: Path | None = None
    updated_at: datetime | None = None
    body_html: str = ""
    toc: list[TocItem] = field(default_factory=list)
    reading_time_minutes: int = 1

    @property
    def is_published(self) -> bool:
        return self.status == "published"

    @property
    def public_path(self) -> str:
        if self.kind == "blog":
            return f"/blog/{self.slug}.html"
        return self.external_url

    @property
    def published_sort_key(self) -> tuple[int, str, str]:
        if self.published_at:
            return (0, self.published_at, self.slug)
        return (1, "", self.slug)

    @property
    def updated_label(self) -> str:
        if not self.updated_at:
            return ""
        return self.updated_at.strftime("%Y-%m-%d %H:%M")

    @property
    def tag_string(self) -> str:
        return ", ".join(self.tags)


@dataclass(slots=True)
class HomepageConfig:
    featured_blog_slugs: list[str] = field(default_factory=list)
    featured_resource_slugs: list[str] = field(default_factory=list)
