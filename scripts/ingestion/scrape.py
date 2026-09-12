#!/usr/bin/env python3
"""
Fetch Labor + Civil Code HTML from lex.uz (URLs from scrape_targets.yaml).

Manual batch job only — never invoked by the Faqih AI runtime app.

Politeness:
  - Honest User-Agent from config
  - Respect robots.txt (lex.uz Crawl-delay currently 20s — overrides shorter floors)
  - Delay between requests + retry with exponential backoff
"""

from __future__ import annotations

import argparse
import shutil
import sys
import time
import urllib.robotparser
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

import requests

from paths_util import (
    ARCHIVE_DIR,
    RAW_DIR,
    TARGETS_PATH,
    append_changelog,
    ensure_dirs,
    load_manifest,
    load_targets,
    save_manifest,
    sha256_bytes,
)

SESSION_TIMEOUT_DEFAULT = 120


def _parse_robots_body(body: str, url: str, user_agent: str) -> tuple[bool, float | None]:
    """Parse robots.txt; strip BOM (lex.uz serves one) which breaks urllib otherwise."""
    cleaned = body.lstrip("\ufeff").replace("\r\n", "\n").replace("\r", "\n")
    parser = urllib.robotparser.RobotFileParser()
    parser.parse(cleaned.splitlines())

    allowed = parser.can_fetch(user_agent, url)
    crawl_delay = parser.crawl_delay(user_agent)
    if crawl_delay is None:
        crawl_delay = parser.crawl_delay("*")

    # Fallback: some parsers miss Crawl-delay; read it directly from text.
    if crawl_delay is None:
        import re

        match = re.search(
            r"(?im)^\s*Crawl-delay\s*:\s*([0-9]+(?:\.[0-9]+)?)\s*$",
            cleaned,
        )
        if match:
            crawl_delay = float(match.group(1))

    return allowed, float(crawl_delay) if crawl_delay is not None else None


def _robots_allowed(
    session: requests.Session,
    url: str,
    user_agent: str,
    *,
    timeout: float,
) -> tuple[bool, float | None]:
    parsed = urlparse(url)
    robots_url = f"{parsed.scheme}://{parsed.netloc}/robots.txt"
    try:
        response = session.get(robots_url, timeout=timeout)
        response.raise_for_status()
        # Prefer decoded text; fall back if charset mishandles BOM.
        body = response.content.decode("utf-8-sig", errors="replace")
        return _parse_robots_body(body, url, user_agent)
    except Exception as exc:  # noqa: BLE001 — allow with warning; still apply config floor
        print(f"[warn] Could not read robots.txt at {robots_url}: {exc}", file=sys.stderr)
        return True, None


def _effective_delay(config_delay: float, robots_delay: float | None) -> float:
    floor = max(2.0, float(config_delay))
    if robots_delay is None:
        return floor
    return max(floor, float(robots_delay))


def _fetch_with_retries(
    session: requests.Session,
    url: str,
    *,
    timeout: float,
    max_retries: int,
    backoff: float,
) -> requests.Response:
    last_error: Exception | None = None
    for attempt in range(1, max_retries + 1):
        try:
            response = session.get(url, timeout=timeout)
            if response.status_code in {429, 500, 502, 503, 504}:
                raise requests.HTTPError(
                    f"Retryable HTTP {response.status_code}",
                    response=response,
                )
            response.raise_for_status()
            return response
        except (requests.RequestException, requests.HTTPError) as exc:
            last_error = exc
            if attempt >= max_retries:
                break
            sleep_for = backoff * (2 ** (attempt - 1))
            print(
                f"[retry] {url} attempt {attempt}/{max_retries} failed: {exc}; "
                f"sleeping {sleep_for:.1f}s",
                file=sys.stderr,
            )
            time.sleep(sleep_for)
    assert last_error is not None
    raise last_error


def _archive_existing(doc_id: str, raw_path: Path, stamp: str) -> Path | None:
    if not raw_path.exists():
        return None
    dest_dir = ARCHIVE_DIR / stamp / doc_id
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / raw_path.name
    shutil.move(str(raw_path), str(dest))
    meta = raw_path.with_suffix(raw_path.suffix + ".meta.json")
    if meta.exists():
        shutil.move(str(meta), str(dest_dir / meta.name))
    return dest


def scrape_one(
    target: dict[str, Any],
    *,
    session: requests.Session,
    user_agent: str,
    delay: float,
    timeout: float,
    max_retries: int,
    backoff: float,
    force: bool,
) -> dict[str, Any]:
    doc_id = target["id"]
    url = target["url"]
    raw_path = RAW_DIR / f"{doc_id}.html"

    allowed, robots_delay = _robots_allowed(session, url, user_agent, timeout=timeout)
    if not allowed:
        raise PermissionError(f"robots.txt disallows fetch for {url} with UA={user_agent!r}")

    effective_delay = _effective_delay(delay, robots_delay)
    print(
        f"[scrape] {doc_id}: delay={effective_delay:.0f}s "
        f"(config={delay}, robots={robots_delay})"
    )

    response = _fetch_with_retries(
        session,
        url,
        timeout=timeout,
        max_retries=max_retries,
        backoff=backoff,
    )
    content = response.content
    content_hash = sha256_bytes(content)

    manifest = load_manifest()
    docs = manifest.setdefault("documents", {})
    previous = docs.get(doc_id)
    previous_hash = previous.get("content_hash") if previous else None

    result: dict[str, Any] = {
        "id": doc_id,
        "url": url,
        "law_name": target["law_name"],
        "title": target.get("title", target["law_name"]),
        "content_hash": content_hash,
        "changed": previous_hash != content_hash,
        "bytes": len(content),
        "path": str(raw_path),
    }

    if previous_hash == content_hash and raw_path.exists() and not force:
        print(f"[scrape] {doc_id}: unchanged ({content_hash[:12]}…); skipping write")
        result["skipped"] = True
        time.sleep(effective_delay)
        return result

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    archived = None
    if raw_path.exists() and previous_hash and previous_hash != content_hash:
        archived = _archive_existing(doc_id, raw_path, stamp)
        append_changelog(
            [
                f"## {stamp} — {doc_id}",
                f"- Law: {target['law_name']}",
                f"- URL: {url}",
                f"- Old hash: `{previous_hash}`",
                f"- New hash: `{content_hash}`",
                f"- Archived to: `{archived}`" if archived else "- Archived: (none)",
            ]
        )
        print(f"[scrape] {doc_id}: changed; archived previous → {archived}")
    elif not previous_hash:
        append_changelog(
            [
                f"## {stamp} — {doc_id} (initial snapshot)",
                f"- Law: {target['law_name']}",
                f"- URL: {url}",
                f"- Hash: `{content_hash}`",
            ]
        )

    ensure_dirs()
    raw_path.write_bytes(content)
    meta = {
        "id": doc_id,
        "law_name": target["law_name"],
        "title": target.get("title"),
        "url": url,
        "fetched_at": datetime.now(timezone.utc).isoformat(),
        "content_hash": content_hash,
        "http_status": response.status_code,
        "final_url": str(response.url),
        "user_agent": user_agent,
    }
    raw_path.with_suffix(".html.meta.json").write_text(
        __import__("json").dumps(meta, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    docs[doc_id] = {
        "law_name": target["law_name"],
        "title": target.get("title"),
        "url": url,
        "content_hash": content_hash,
        "fetched_at": meta["fetched_at"],
        "raw_file": raw_path.name,
        "bytes": len(content),
    }
    manifest["updated_at"] = meta["fetched_at"]
    save_manifest(manifest)

    print(f"[scrape] {doc_id}: saved {raw_path} ({len(content)} bytes, {content_hash[:12]}…)")
    time.sleep(effective_delay)
    result["skipped"] = False
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Scrape lex.uz targets into raw_data/")
    parser.add_argument("--config", type=Path, default=TARGETS_PATH)
    parser.add_argument("--force", action="store_true", help="Rewrite even if hash matches")
    parser.add_argument("--only", nargs="*", help="Limit to target id(s)")
    args = parser.parse_args(argv)

    ensure_dirs()
    config = load_targets(args.config)
    user_agent = config.get(
        "user_agent",
        "FaqihAI-Ingestion/0.1 (+offline Uzbekistan legal corpus; manual batch)",
    )
    delay = float(config.get("request_delay_seconds", 3))
    max_retries = int(config.get("max_retries", 3))
    backoff = float(config.get("retry_backoff_seconds", 5))
    timeout = float(config.get("timeout_seconds", SESSION_TIMEOUT_DEFAULT))

    targets = config["targets"]
    if args.only:
        wanted = set(args.only)
        targets = [t for t in targets if t["id"] in wanted]
        missing = wanted - {t["id"] for t in targets}
        if missing:
            print(f"[error] Unknown target id(s): {sorted(missing)}", file=sys.stderr)
            return 2

    session = requests.Session()
    session.headers.update(
        {
            "User-Agent": user_agent,
            "Accept": "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8",
            "Accept-Language": "uz,ru;q=0.8,en;q=0.5",
        }
    )

    print(f"[scrape] {len(targets)} target(s) from {args.config}")
    print(
        "[scrape] Note: lex.uz robots.txt currently sets Crawl-delay: 20 — "
        "expect ~20s between requests."
    )

    results = []
    for target in targets:
        results.append(
            scrape_one(
                target,
                session=session,
                user_agent=user_agent,
                delay=delay,
                timeout=timeout,
                max_retries=max_retries,
                backoff=backoff,
                force=args.force,
            )
        )

    changed = sum(1 for r in results if r.get("changed") and not r.get("skipped"))
    print(f"[scrape] done. {changed} document(s) updated.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
