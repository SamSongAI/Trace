from __future__ import annotations

import shutil
from contextlib import contextmanager
from datetime import datetime
from pathlib import Path

from .config import Settings
from .content import ContentStore
from .renderer import SiteRenderer

try:
    import fcntl
except ImportError:  # pragma: no cover
    fcntl = None


class PublishError(RuntimeError):
    pass


@contextmanager
def publish_lock(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        if fcntl is not None:
            try:
                fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as exc:
                raise PublishError("another publish is already in progress") from exc
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


class Publisher:
    def __init__(self, settings: Settings, store: ContentStore, renderer: SiteRenderer):
        self.settings = settings
        self.store = store
        self.renderer = renderer
        self.settings.backup_root.mkdir(parents=True, exist_ok=True)
        self.settings.state_root.mkdir(parents=True, exist_ok=True)

    def build(self, output_root: Path) -> dict[str, str]:
        output_root.mkdir(parents=True, exist_ok=True)
        context = self.renderer.build_site_context(self.store)
        self.renderer.write_public_site(context, output_root)
        self._validate_output(output_root)
        return {
            "index": str(output_root / "index.html"),
            "resources": str(output_root / "resources.html"),
            "blog_index": str(output_root / "blog" / "index.html"),
        }

    def publish(self) -> str:
        live_root = self.settings.public_site_dir
        parent = live_root.parent
        parent.mkdir(parents=True, exist_ok=True)
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        staging_root = parent / f".{live_root.name}.staging-{timestamp}"
        temp_old_root = parent / f".{live_root.name}.live-old-{timestamp}"
        backup_root = self.settings.backup_root / f"{live_root.name}-{timestamp}"

        with publish_lock(self.settings.lock_file):
            try:
                if live_root.exists():
                    shutil.copytree(live_root, staging_root, dirs_exist_ok=False)
                    shutil.copytree(live_root, backup_root, dirs_exist_ok=False)
                else:
                    staging_root.mkdir(parents=True, exist_ok=True)

                self.build(staging_root)

                if live_root.exists():
                    live_root.rename(temp_old_root)
                staging_root.rename(live_root)
                if temp_old_root.exists():
                    shutil.rmtree(temp_old_root)
            except Exception as exc:  # noqa: BLE001
                if staging_root.exists():
                    shutil.rmtree(staging_root, ignore_errors=True)
                if temp_old_root.exists() and not live_root.exists():
                    temp_old_root.rename(live_root)
                raise PublishError(str(exc)) from exc

        return str(backup_root)

    def _validate_output(self, output_root: Path) -> None:
        required = [
            output_root / "index.html",
            output_root / "resources.html",
            output_root / "blog" / "index.html",
        ]
        for path in required:
            if not path.exists():
                raise PublishError(f"missing generated file: {path}")
