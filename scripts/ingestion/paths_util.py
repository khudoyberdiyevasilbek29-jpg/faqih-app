"""Shared paths and helpers for the offline ingestion pipeline."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

import yaml

INGESTION_ROOT = Path(__file__).resolve().parent
REPO_ROOT = INGESTION_ROOT.parents[1]

RAW_DIR = INGESTION_ROOT / "raw_data"
ARCHIVE_DIR = INGESTION_ROOT / "archive"
CLEANED_DIR = INGESTION_ROOT / "cleaned"
CHUNKS_DIR = INGESTION_ROOT / "chunks"
MANIFEST_PATH = RAW_DIR / ".snapshot_manifest.json"
CHANGELOG_PATH = INGESTION_ROOT / "CHANGELOG.md"
TARGETS_PATH = INGESTION_ROOT / "scrape_targets.yaml"
DEFAULT_DB_PATH = REPO_ROOT / "src-tauri" / "resources" / "lexuz.db"

EMBEDDING_MODEL = "intfloat/multilingual-e5-small"
EMBEDDING_DIM = 384
PASSAGE_PREFIX = "passage: "
QUERY_PREFIX = "query: "


def ensure_dirs() -> None:
    for path in (RAW_DIR, ARCHIVE_DIR, CLEANED_DIR, CHUNKS_DIR):
        path.mkdir(parents=True, exist_ok=True)


def load_targets(path: Path = TARGETS_PATH) -> dict[str, Any]:
    with path.open(encoding="utf-8") as fh:
        data = yaml.safe_load(fh)
    if not data or "targets" not in data:
        raise ValueError(f"No targets found in {path}")
    return data


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_manifest(path: Path = MANIFEST_PATH) -> dict[str, Any]:
    if not path.exists():
        return {"documents": {}}
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def save_manifest(manifest: dict[str, Any], path: Path = MANIFEST_PATH) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        json.dump(manifest, fh, ensure_ascii=False, indent=2)
        fh.write("\n")


def append_changelog(lines: list[str], path: Path = CHANGELOG_PATH) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    header_needed = not path.exists()
    with path.open("a", encoding="utf-8") as fh:
        if header_needed:
            fh.write("# Ingestion CHANGELOG\n\n")
        for line in lines:
            fh.write(line.rstrip() + "\n")
        fh.write("\n")


def slugify(value: str) -> str:
    value = value.strip().lower()
    value = re.sub(r"[^\w\s-]", "", value, flags=re.UNICODE)
    value = re.sub(r"[\s_]+", "-", value)
    return value.strip("-") or "chunk"


def estimate_tokens(text: str) -> int:
    """Rough token estimate for chunk sizing (whitespace words × 1.3)."""
    words = [w for w in text.split() if w]
    return max(1, int(round(len(words) * 1.3))) if words else 0
