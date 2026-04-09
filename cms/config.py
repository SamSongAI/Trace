from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path


def _env_flag(name: str, default: bool) -> bool:
    raw = os.getenv(name)
    if raw is None:
        return default
    return raw.strip().lower() in {"1", "true", "yes", "on"}


@dataclass(slots=True)
class Settings:
    repo_root: Path
    content_root: Path
    public_site_dir: Path
    backup_root: Path
    state_root: Path
    site_base_url: str
    admin_password_hash: str | None
    session_secret: str | None
    session_cookie_secure: bool
    admin_host: str
    admin_port: int

    @property
    def lock_file(self) -> Path:
        return self.state_root / "publish.lock"

    @classmethod
    def load(cls, repo_root: Path | None = None) -> "Settings":
        root = (repo_root or Path(__file__).resolve().parent.parent).resolve()
        state_root = Path(os.getenv("CONTENT_ADMIN_STATE_ROOT", root / ".content-admin")).resolve()
        return cls(
            repo_root=root,
            content_root=Path(os.getenv("CONTENT_ROOT", root / "content")).resolve(),
            public_site_dir=Path(os.getenv("PUBLIC_SITE_DIR", root / "website")).resolve(),
            backup_root=Path(os.getenv("BACKUP_ROOT", state_root / "backups")).resolve(),
            state_root=state_root,
            site_base_url=os.getenv("SITE_BASE_URL", "https://sotasync.com").rstrip("/"),
            admin_password_hash=os.getenv("ADMIN_PASSWORD_HASH"),
            session_secret=os.getenv("SESSION_SECRET"),
            session_cookie_secure=_env_flag("ADMIN_COOKIE_SECURE", True),
            admin_host=os.getenv("ADMIN_HOST", "127.0.0.1"),
            admin_port=int(os.getenv("ADMIN_PORT", "4310")),
        )
