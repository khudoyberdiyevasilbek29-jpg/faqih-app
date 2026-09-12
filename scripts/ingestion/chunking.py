#!/usr/bin/env python3
"""
Chunk cleaned articles for embedding.

Strategy:
  1. One chunk per article (modda) by default.
  2. Oversized articles → ~300–500 token sliding windows with ~50 token overlap.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from paths_util import (
    CHUNKS_DIR,
    CLEANED_DIR,
    ensure_dirs,
    estimate_tokens,
    load_targets,
    slugify,
)

DEFAULT_MIN_TOKENS = 300
DEFAULT_MAX_TOKENS = 500
DEFAULT_OVERLAP_TOKENS = 50


def _word_windows(
    text: str,
    *,
    max_tokens: int,
    overlap_tokens: int,
) -> list[str]:
    words = text.split()
    if not words:
        return []

    # Convert token budgets to approximate word budgets (tokens ≈ words × 1.3).
    max_words = max(40, int(max_tokens / 1.3))
    overlap_words = max(10, int(overlap_tokens / 1.3))
    step = max(1, max_words - overlap_words)

    windows: list[str] = []
    start = 0
    while start < len(words):
        end = min(len(words), start + max_words)
        windows.append(" ".join(words[start:end]))
        if end >= len(words):
            break
        start += step
    return windows


def chunk_document(
    cleaned: dict[str, Any],
    *,
    max_tokens: int = DEFAULT_MAX_TOKENS,
    min_tokens: int = DEFAULT_MIN_TOKENS,
    overlap_tokens: int = DEFAULT_OVERLAP_TOKENS,
) -> list[dict[str, Any]]:
    """
    min_tokens is informational for the preferred band; we only split when
    estimate_tokens(article) > max_tokens.
    """
    _ = min_tokens
    law_name = cleaned["law_name"]
    source_id = cleaned["id"]
    chunks: list[dict[str, Any]] = []

    for article in cleaned.get("articles", []):
        article_number = str(article.get("article_number") or "").strip()
        chapter = str(article.get("chapter") or "").strip()
        section_title = str(article.get("section_title") or "").strip()
        text = str(article.get("text") or "").strip()
        if not text:
            continue

        token_count = estimate_tokens(text)
        if token_count <= max_tokens:
            pieces = [text]
        else:
            pieces = _word_windows(
                text,
                max_tokens=max_tokens,
                overlap_tokens=overlap_tokens,
            )

        for idx, piece in enumerate(pieces):
            suffix = "" if len(pieces) == 1 else f"__p{idx + 1}"
            art_key = article_number or slugify(article.get("article_title") or "article")
            chunk_id = f"{source_id}__art-{art_key}{suffix}"
            chunks.append(
                {
                    "id": chunk_id,
                    "text": piece,
                    "law_name": law_name,
                    "article_number": article_number,
                    "chapter": chapter,
                    "section_title": section_title,
                    "source_id": source_id,
                    "token_estimate": estimate_tokens(piece),
                    "part_index": idx,
                    "part_count": len(pieces),
                }
            )

    return chunks


def chunk_file(
    cleaned_path: Path,
    *,
    max_tokens: int,
    min_tokens: int,
    overlap_tokens: int,
) -> Path:
    cleaned = json.loads(cleaned_path.read_text(encoding="utf-8"))
    chunks = chunk_document(
        cleaned,
        max_tokens=max_tokens,
        min_tokens=min_tokens,
        overlap_tokens=overlap_tokens,
    )
    ensure_dirs()
    out = CHUNKS_DIR / f"{cleaned_path.stem}.chunks.json"
    payload = {
        "id": cleaned.get("id"),
        "law_name": cleaned.get("law_name"),
        "chunk_count": len(chunks),
        "chunks": chunks,
    }
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    oversized = sum(1 for c in chunks if c.get("part_count", 1) > 1)
    print(
        f"[chunk] {cleaned_path.stem}: {len(chunks)} chunks "
        f"({oversized} from split articles) → {out}"
    )
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Chunk cleaned articles for embedding")
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    parser.add_argument("--min-tokens", type=int, default=DEFAULT_MIN_TOKENS)
    parser.add_argument("--overlap-tokens", type=int, default=DEFAULT_OVERLAP_TOKENS)
    args = parser.parse_args(argv)

    ensure_dirs()
    from paths_util import TARGETS_PATH

    config = load_targets(TARGETS_PATH)
    targets = config["targets"]
    if args.only:
        wanted = set(args.only)
        targets = [t for t in targets if t["id"] in wanted]

    if not targets:
        print("[chunk] no targets", file=sys.stderr)
        return 2

    for target in targets:
        cleaned_path = CLEANED_DIR / f"{target['id']}.json"
        if not cleaned_path.exists():
            print(f"[chunk] missing cleaned file: {cleaned_path}", file=sys.stderr)
            return 1
        chunk_file(
            cleaned_path,
            max_tokens=args.max_tokens,
            min_tokens=args.min_tokens,
            overlap_tokens=args.overlap_tokens,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
