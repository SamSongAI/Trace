from __future__ import annotations

import argparse
import os
from functools import wraps
from pathlib import Path
from typing import Any, Callable

from flask import Flask, abort, flash, redirect, render_template, request, send_from_directory, session, url_for
from werkzeug.security import check_password_hash, generate_password_hash

from .config import Settings
from .content import ContentStore, slugify
from .models import ContentEntry
from .publisher import PublishError, Publisher
from .renderer import SiteRenderer


def create_app(settings: Settings | None = None) -> Flask:
    app = Flask(
        __name__,
        template_folder=str(Path(__file__).resolve().parent / "templates"),
        static_folder=str(Path(__file__).resolve().parent / "static"),
        static_url_path="/admin/static",
    )
    settings = settings or Settings.load()
    app.config.update(
        SECRET_KEY=settings.session_secret or "dev-only-session-secret",
        SESSION_COOKIE_NAME="sotasync_admin_session",
        SESSION_COOKIE_HTTPONLY=True,
        SESSION_COOKIE_SAMESITE="Strict",
        SESSION_COOKIE_SECURE=settings.session_cookie_secure,
    )

    store = ContentStore(settings.content_root)
    renderer = SiteRenderer(settings)
    publisher = Publisher(settings, store, renderer)

    app.extensions["content_store"] = store
    app.extensions["site_renderer"] = renderer
    app.extensions["site_publisher"] = publisher
    app.extensions["settings"] = settings

    def require_auth_config() -> None:
        if not settings.admin_password_hash or not settings.session_secret:
            raise RuntimeError("ADMIN_PASSWORD_HASH and SESSION_SECRET must be set")

    def login_required(view: Callable[..., Any]) -> Callable[..., Any]:
        @wraps(view)
        def wrapped(*args: Any, **kwargs: Any) -> Any:
            if not session.get("is_admin"):
                return redirect(url_for("login", next=request.path))
            return view(*args, **kwargs)

        return wrapped

    def get_kind_or_404(kind: str) -> str:
        if kind not in {"blog", "resources"}:
            abort(404)
        return kind

    def content_defaults(kind: str) -> dict[str, Any]:
        base = {
            "title": "",
            "slug": "",
            "published_at": "",
            "status": "draft",
            "tags": "",
            "body_markdown": "",
        }
        if kind == "blog":
            base.update({"excerpt": "", "seo_description": ""})
        else:
            base.update({"external_url": "", "source_name": "", "summary": ""})
        return base

    def entry_to_form(entry: ContentEntry) -> dict[str, Any]:
        data = {
            "title": entry.title,
            "slug": entry.slug,
            "published_at": entry.published_at,
            "status": entry.status,
            "tags": ", ".join(entry.tags),
            "body_markdown": entry.body_markdown,
        }
        if entry.kind == "blog":
            data["excerpt"] = entry.excerpt
            data["seo_description"] = entry.seo_description
        else:
            data["external_url"] = entry.external_url
            data["source_name"] = entry.source_name
            data["summary"] = entry.summary
        return data

    def get_entry_or_redirect(kind: str, slug: str) -> ContentEntry | Any:
        entry = store.load_entry(kind, slug)
        if not entry:
            flash("content entry not found", "error")
            return redirect(url_for("content_list", kind=kind))
        return entry

    def sorted_selection(prefix: str, selected: list[str]) -> list[str]:
        def order_for(slug: str) -> tuple[int, str]:
            raw = request.form.get(f"{prefix}_{slug}", "").strip()
            if not raw:
                return (10_000, slug)
            try:
                return (int(raw), slug)
            except ValueError:
                return (10_000, slug)

        return [slug for slug in sorted(selected, key=order_for)]

    @app.context_processor
    def inject_globals() -> dict[str, Any]:
        return {"settings": settings}

    @app.route("/site.css")
    def preview_site_css() -> Any:
        return send_from_directory(settings.public_site_dir, "site.css")

    @app.route("/admin/login", methods=["GET", "POST"])
    def login() -> Any:
        error = None
        if request.method == "POST":
            try:
                require_auth_config()
            except RuntimeError as exc:
                error = str(exc)
            else:
                password = request.form.get("password", "")
                if settings.admin_password_hash and check_password_hash(settings.admin_password_hash, password):
                    session.clear()
                    session["is_admin"] = True
                    flash("logged in", "success")
                    target = request.args.get("next") or url_for("dashboard")
                    return redirect(target)
                error = "invalid password"
        return render_template("admin/login.html", error=error)

    @app.route("/admin/logout", methods=["POST"])
    @login_required
    def logout() -> Any:
        session.clear()
        flash("logged out", "success")
        return redirect(url_for("login"))

    @app.route("/admin")
    @app.route("/admin/")
    @login_required
    def dashboard() -> Any:
        blogs = store.list_entries("blog")
        resources = store.list_entries("resources")
        homepage = store.load_homepage_config()
        return render_template(
            "admin/dashboard.html",
            blog_count=len(blogs),
            blog_published=sum(1 for entry in blogs if entry.is_published),
            resource_count=len(resources),
            resource_published=sum(1 for entry in resources if entry.is_published),
            homepage=homepage,
            public_site_dir=str(settings.public_site_dir),
            backup_root=str(settings.backup_root),
        )

    @app.route("/admin/publish", methods=["POST"])
    @login_required
    def publish_site() -> Any:
        try:
            backup_path = publisher.publish()
        except PublishError as exc:
            flash(str(exc), "error")
        else:
            flash(f"site published, backup created at {backup_path}", "success")
        return redirect(request.referrer or url_for("dashboard"))

    @app.route("/admin/content/<kind>")
    @login_required
    def content_list(kind: str) -> Any:
        kind = get_kind_or_404(kind)
        status_filter = request.args.get("status", "all")
        entries = store.list_entries(kind)
        if status_filter in {"draft", "published"}:
            entries = [entry for entry in entries if entry.status == status_filter]
        return render_template(
            "admin/content_list.html",
            kind=kind,
            entries=entries,
            status_filter=status_filter,
        )

    @app.route("/admin/content/<kind>/new", methods=["GET", "POST"])
    @login_required
    def content_new(kind: str) -> Any:
        kind = get_kind_or_404(kind)
        form_data = content_defaults(kind)
        errors: list[str] = []
        if request.method == "POST":
            form_data = request.form.to_dict()
            try:
                entry = store.save_entry(kind, form_data)
            except ValueError as exc:
                errors.append(str(exc))
            else:
                flash("draft saved", "success")
                return redirect(url_for("content_edit", kind=kind, slug=entry.slug))
        return render_template(
            "admin/content_form.html",
            kind=kind,
            form_data=form_data,
            errors=errors,
            mode="new",
            entry=None,
        )

    @app.route("/admin/content/<kind>/<slug>", methods=["GET", "POST"])
    @login_required
    def content_edit(kind: str, slug: str) -> Any:
        kind = get_kind_or_404(kind)
        entry = store.load_entry(kind, slug)
        if not entry:
            flash("content entry not found", "error")
            return redirect(url_for("content_list", kind=kind))

        errors: list[str] = []
        form_data = entry_to_form(entry)
        if request.method == "POST":
            form_data = request.form.to_dict()
            try:
                entry = store.save_entry(kind, form_data, original_slug=slug)
            except ValueError as exc:
                errors.append(str(exc))
            else:
                flash("changes saved", "success")
                return redirect(url_for("content_edit", kind=kind, slug=entry.slug))

        return render_template(
            "admin/content_form.html",
            kind=kind,
            form_data=form_data,
            errors=errors,
            mode="edit",
            entry=entry,
        )

    @app.route("/admin/content/<kind>/<slug>/preview")
    @login_required
    def content_preview(kind: str, slug: str) -> Any:
        kind = get_kind_or_404(kind)
        entry = store.load_entry(kind, slug)
        if not entry:
            flash("content entry not found", "error")
            return redirect(url_for("content_list", kind=kind))
        decorated = renderer.decorate_entry(entry)
        context = renderer.build_site_context(store)
        if kind == "blog":
            return renderer.render_blog_post(decorated, is_preview=True)
        preview_resources = [item for item in context["resources"] if item.slug != decorated.slug]
        decorated_context = dict(context)
        decorated_context["resources"] = [decorated, *preview_resources]
        return renderer.render_resources(decorated_context, preview_entry=decorated, is_preview=True)

    @app.route("/admin/content/<kind>/<slug>/publish", methods=["POST"])
    @login_required
    def content_publish(kind: str, slug: str) -> Any:
        kind = get_kind_or_404(kind)
        entry = store.load_entry(kind, slug)
        if not entry:
            flash("content entry not found", "error")
            return redirect(url_for("content_list", kind=kind))

        action = request.form.get("action", "publish")
        form_payload = entry_to_form(entry)
        form_payload["status"] = "published" if action == "publish" else "draft"

        candidate = store.copy_entry(entry)
        candidate.status = form_payload["status"]
        errors = store.validate_entry(candidate, for_publish=action == "publish")
        if errors:
            for error in errors:
                flash(error, "error")
            return redirect(url_for("content_edit", kind=kind, slug=slug))

        try:
            updated = store.save_entry(kind, form_payload, original_slug=slug)
            backup_path = publisher.publish()
        except (ValueError, PublishError) as exc:
            flash(str(exc), "error")
            return redirect(url_for("content_edit", kind=kind, slug=slug))

        flash(
            f"content published and site rebuilt, backup created at {backup_path}"
            if action == "publish"
            else f"content unpublished, backup created at {backup_path}",
            "success",
        )
        return redirect(url_for("content_edit", kind=kind, slug=updated.slug))

    @app.route("/admin/homepage/featured", methods=["GET", "POST"])
    @login_required
    def homepage_featured() -> Any:
        published_blogs = store.published_entries("blog")
        published_resources = store.published_entries("resources")
        config = store.load_homepage_config()
        if request.method == "POST":
            blog_slugs = sorted_selection("blog_order", [slug for slug in request.form.getlist("featured_blog_slugs") if slug])
            resource_slugs = sorted_selection(
                "resource_order",
                [slug for slug in request.form.getlist("featured_resource_slugs") if slug],
            )
            config = store.save_homepage_config(blog_slugs, resource_slugs)
            flash("homepage featured slots saved", "success")
            if request.form.get("publish_now") == "1":
                try:
                    backup_path = publisher.publish()
                except PublishError as exc:
                    flash(str(exc), "error")
                else:
                    flash(f"homepage published, backup created at {backup_path}", "success")
            return redirect(url_for("homepage_featured"))
        return render_template(
            "admin/featured.html",
            published_blogs=published_blogs,
            published_resources=published_resources,
            config=config,
        )

    return app


def main() -> None:
    parser = argparse.ArgumentParser(description="SOTA SYNC content admin")
    parser.add_argument("command", choices=["serve", "build", "publish", "hash-password"])
    parser.add_argument("--host", default=None)
    parser.add_argument("--port", type=int, default=None)
    parser.add_argument("--password", default=None)
    args = parser.parse_args()

    settings = Settings.load()
    store = ContentStore(settings.content_root)
    renderer = SiteRenderer(settings)
    publisher = Publisher(settings, store, renderer)

    if args.command == "hash-password":
        password = args.password or os.getenv("ADMIN_PASSWORD")
        if not password:
            raise SystemExit("provide --password or ADMIN_PASSWORD")
        print(generate_password_hash(password))
        return

    if args.command == "build":
        output = settings.state_root / "preview-build"
        if output.exists():
            import shutil

            shutil.rmtree(output)
        publisher.build(output)
        print(output)
        return

    if args.command == "publish":
        print(publisher.publish())
        return

    app = create_app(settings)
    app.run(
        host=args.host or settings.admin_host,
        port=args.port or settings.admin_port,
        debug=False,
    )


if __name__ == "__main__":
    main()
