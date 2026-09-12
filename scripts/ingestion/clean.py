#!/usr/bin/env python3
"""
Clean scraped lex.uz HTML into structured article JSON.

Parses modda (article) boundaries and preserves chapter / section titles.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from bs4 import BeautifulSoup, NavigableString, Tag

from paths_util import CLEANED_DIR, RAW_DIR, ensure_dirs, load_targets

NOISE_CLASS_FRAGMENTS = (
    "INDEXES_ON_REF",
    "lx_no_select",
    "lx_revive",
    "docContentHeader",
)

CLAUSE_RE = re.compile(
    r"^\s*(\d+)\s*-\s*modda\.?\s*(.*)$",
    re.IGNORECASE | re.UNICODE,
)
CHAPTER_RE = re.compile(
    r"^\s*(\d+)\s*-\s*bob\.?\s*(.*)$",
    re.IGNORECASE | re.UNICODE,
)
SECTION_RE = re.compile(
    r"^\s*(\d+)\s*-\s*§\.?\s*(.*)$",
    re.IGNORECASE | re.UNICODE,
)
BOB_HEADER_RE = re.compile(
    r"^\s*((?:[IVXLC]+|\d+)\s*BO[ʻ']?LIM[^\n]*)$",
    re.IGNORECASE | re.UNICODE,
)

UI_NOISE_RE = re.compile(
    r"(Hujjatga taklif yuborish|Audioni tinglash|Hujjat elementidan havola olish|"
    r"LexUZ sharhi|OKOZ:|TSZ:)",
    re.IGNORECASE,
)


def _class_list(tag: Tag) -> list[str]:
    raw = tag.get("class") or []
    return [str(c) for c in raw]


def _is_noise(tag: Tag) -> bool:
    classes = " ".join(_class_list(tag))
    return any(frag in classes for frag in NOISE_CLASS_FRAGMENTS)


def _visible_text(tag: Tag) -> str:
    parts: list[str] = []
    for child in tag.descendants:
        if isinstance(child, NavigableString):
            parent = child.parent
            if isinstance(parent, Tag) and parent.name in {"script", "style"}:
                continue
            if isinstance(parent, Tag) and _is_noise(parent):
                continue
            text = str(child).strip()
            if text:
                parts.append(text)
    text = " ".join(parts)
    text = UI_NOISE_RE.sub(" ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text


def parse_lex_html(html: str, *, law_name: str, source_id: str, source_url: str) -> dict[str, Any]:
    soup = BeautifulSoup(html, "html.parser")
    root = soup.select_one("#divCont") or soup.select_one("#doc_main") or soup.body
    if root is None:
        raise ValueError("Could not locate document content container")

    # Drop obvious noise nodes before walking.
    for bad in root.select(".INDEXES_ON_REF, .lx_no_select, script, style, noscript"):
        bad.decompose()

    current_chapter = ""
    current_section = ""
    current_part = ""
    articles: list[dict[str, Any]] = []
    pending_clause: dict[str, Any] | None = None
    preamble_bits: list[str] = []

    def flush_pending() -> None:
        nonlocal pending_clause
        if pending_clause is None:
            return
        body = pending_clause.get("text", "").strip()
        title = pending_clause.get("article_title", "").strip()
        if body or title:
            pending_clause["text"] = (f"{title}. {body}" if title and not body.startswith(title) else body or title).strip()
            articles.append(pending_clause)
        pending_clause = None

    elements = root.select(".lx_elem, .ACT_TITLE, .TEXT_HEADER_DEFAULT, .CLAUSE_DEFAULT, .ACT_TEXT, .ACT_FORM")
    if not elements:
        # Fallback: treat whole text and split by modda regex later.
        full = _visible_text(root)
        return _fallback_split(full, law_name=law_name, source_id=source_id, source_url=source_url)

    for el in elements:
        if _is_noise(el):
            continue
        classes = _class_list(el)
        text = _visible_text(el)
        if not text:
            continue

        joined = " ".join(classes)

        if "ACT_TITLE" in joined or "ACT_FORM" in joined:
            continue

        if "TEXT_HEADER_DEFAULT" in joined or "TEXT_HEADER" in joined:
            flush_pending()
            chapter_match = CHAPTER_RE.match(text)
            section_match = SECTION_RE.match(text)
            if chapter_match:
                current_chapter = text
                current_section = ""
            elif section_match:
                current_section = text
            elif BOB_HEADER_RE.match(text) or "BOʻLIM" in text.upper() or "BO'LIM" in text.upper():
                current_part = text
                current_chapter = text
            else:
                # Generic header — treat as section title under current chapter.
                if current_chapter:
                    current_section = text
                else:
                    current_chapter = text
            continue

        if "CLAUSE_DEFAULT" in joined or "CLAUSE_" in joined:
            flush_pending()
            clause_match = CLAUSE_RE.match(text)
            article_number = clause_match.group(1) if clause_match else ""
            article_title = clause_match.group(2).strip() if clause_match else text
            pending_clause = {
                "article_number": article_number,
                "article_title": article_title,
                "chapter": current_chapter or current_part,
                "section_title": current_section,
                "text": "",
            }
            continue

        if "ACT_TEXT" in joined:
            if pending_clause is not None:
                existing = pending_clause["text"]
                pending_clause["text"] = f"{existing} {text}".strip() if existing else text
            else:
                preamble_bits.append(text)
            continue

    flush_pending()

    if not articles:
        full = _visible_text(root)
        return _fallback_split(full, law_name=law_name, source_id=source_id, source_url=source_url)

    return {
        "id": source_id,
        "law_name": law_name,
        "source_url": source_url,
        "article_count": len(articles),
        "preamble": " ".join(preamble_bits).strip(),
        "articles": articles,
    }


def _fallback_split(
    full_text: str,
    *,
    law_name: str,
    source_id: str,
    source_url: str,
) -> dict[str, Any]:
    """Regex fallback when structured lx_elem markup is missing."""
    pattern = re.compile(r"(?=(?:^|\n)\s*\d+\s*-\s*modda\.)", re.IGNORECASE)
    parts = [p.strip() for p in pattern.split(full_text) if p.strip()]
    articles: list[dict[str, Any]] = []
    chapter = ""
    section = ""

    for part in parts:
        # Capture preceding chapter headers inside the chunk head.
        lines = [ln.strip() for ln in part.splitlines() if ln.strip()]
        body_lines: list[str] = []
        article_number = ""
        article_title = ""
        for ln in lines:
            if CHAPTER_RE.match(ln):
                chapter = ln
                continue
            if SECTION_RE.match(ln):
                section = ln
                continue
            cm = CLAUSE_RE.match(ln)
            if cm and not article_number:
                article_number = cm.group(1)
                article_title = cm.group(2).strip()
                continue
            body_lines.append(ln)
        text = " ".join(body_lines).strip()
        if article_title and text:
            text = f"{article_title}. {text}"
        elif article_title:
            text = article_title
        if not text:
            continue
        articles.append(
            {
                "article_number": article_number,
                "article_title": article_title,
                "chapter": chapter,
                "section_title": section,
                "text": text,
            }
        )

    return {
        "id": source_id,
        "law_name": law_name,
        "source_url": source_url,
        "article_count": len(articles),
        "preamble": "",
        "articles": articles,
        "parse_mode": "fallback_regex",
    }


def clean_file(raw_path: Path, *, law_name: str, source_id: str, source_url: str) -> Path:
    html = raw_path.read_text(encoding="utf-8", errors="replace")
    cleaned = parse_lex_html(html, law_name=law_name, source_id=source_id, source_url=source_url)
    ensure_dirs()
    out = CLEANED_DIR / f"{source_id}.json"
    out.write_text(json.dumps(cleaned, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[clean] {source_id}: {cleaned['article_count']} articles → {out}"
    )
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Clean raw lex.uz HTML into article JSON")
    parser.add_argument("--config", type=Path, default=None)
    parser.add_argument("--only", nargs="*")
    args = parser.parse_args(argv)

    ensure_dirs()
    from paths_util import TARGETS_PATH

    config = load_targets(args.config or TARGETS_PATH)
    targets = config["targets"]
    if args.only:
        wanted = set(args.only)
        targets = [t for t in targets if t["id"] in wanted]

    if not targets:
        print("[clean] no targets", file=sys.stderr)
        return 2

    for target in targets:
        raw_path = RAW_DIR / f"{target['id']}.html"
        if not raw_path.exists():
            print(f"[clean] missing raw file: {raw_path}", file=sys.stderr)
            return 1
        clean_file(
            raw_path,
            law_name=target["law_name"],
            source_id=target["id"],
            source_url=target["url"],
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
