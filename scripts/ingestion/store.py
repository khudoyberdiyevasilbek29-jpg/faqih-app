#!/usr/bin/env python3
"""
Write embedded chunks into LanceDB.

Schema:
  id, text, law_name, article_number, chapter, section_title, embedding_vector

Default path: src-tauri/resources/lexuz.db (LanceDB directory URI).
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any

from paths_util import CHUNKS_DIR, DEFAULT_DB_PATH, EMBEDDING_DIM, ensure_dirs, load_targets

TABLE_NAME = "lexuz_articles"


def _load_embedded(paths: list[Path]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for path in paths:
        payload = json.loads(path.read_text(encoding="utf-8"))
        for chunk in payload.get("chunks", []):
            vector = chunk.get("embedding_vector")
            if not vector or len(vector) != EMBEDDING_DIM:
                raise ValueError(
                    f"Bad embedding_vector on {chunk.get('id')} in {path} "
                    f"(expected dim {EMBEDDING_DIM})"
                )
            rows.append(
                {
                    "id": chunk["id"],
                    "text": chunk["text"],
                    "law_name": chunk["law_name"],
                    "article_number": str(chunk.get("article_number") or ""),
                    "chapter": str(chunk.get("chapter") or ""),
                    "section_title": str(chunk.get("section_title") or ""),
                    "embedding_vector": vector,
                }
            )
    return rows


def _table_names(db) -> list[str]:
    """LanceDB API compatibility: list_tables() may return a list or a response object."""
    try:
        names = db.list_tables()
    except Exception:  # noqa: BLE001
        names = db.table_names()
    if isinstance(names, list):
        return [str(n) for n in names]
    tables = getattr(names, "tables", None)
    if tables is not None:
        return [str(n) for n in tables]
    return [str(n) for n in list(names)]


def write_lancedb(rows: list[dict[str, Any]], db_path: Path, *, overwrite: bool) -> None:
    try:
        import lancedb
        import pyarrow as pa
    except ImportError as exc:
        raise SystemExit(
            "lancedb/pyarrow missing. Install with:\n"
            "  pip install torch --index-url https://download.pytorch.org/whl/cpu\n"
            "  pip install -r requirements.txt\n"
        ) from exc

    if not rows:
        raise ValueError("No rows to write")

    if db_path.exists() and overwrite:
        if db_path.is_file():
            db_path.unlink()
        else:
            shutil.rmtree(db_path)

    db_path.parent.mkdir(parents=True, exist_ok=True)
    db = lancedb.connect(str(db_path))

    schema = pa.schema(
        [
            pa.field("id", pa.utf8()),
            pa.field("text", pa.utf8()),
            pa.field("law_name", pa.utf8()),
            pa.field("article_number", pa.utf8()),
            pa.field("chapter", pa.utf8()),
            pa.field("section_title", pa.utf8()),
            pa.field("embedding_vector", pa.list_(pa.float32(), EMBEDDING_DIM)),
        ]
    )

    if TABLE_NAME in _table_names(db):
        if not overwrite:
            raise FileExistsError(
                f"Table {TABLE_NAME!r} already exists in {db_path}. Pass --overwrite."
            )
        db.drop_table(TABLE_NAME)

    table = db.create_table(TABLE_NAME, data=rows, schema=schema)
    # Vector index — best-effort; small tables / API drift should not fail the build.
    try:
        from lancedb.index import IvfPq

        table.create_index(
            "embedding_vector",
            config=IvfPq(distance_type="cosine"),
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[store] vector index skipped: {exc}", file=sys.stderr)

    print(f"[store] wrote {len(rows)} rows → {db_path} (table={TABLE_NAME})")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Store embeddings in LanceDB")
    parser.add_argument("--db", type=Path, default=DEFAULT_DB_PATH)
    parser.add_argument("--only", nargs="*")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace existing lexuz.db / table",
    )
    args = parser.parse_args(argv)

    ensure_dirs()
    from paths_util import TARGETS_PATH

    config = load_targets(TARGETS_PATH)
    targets = config["targets"]
    if args.only:
        wanted = set(args.only)
        targets = [t for t in targets if t["id"] in wanted]

    embedded_paths: list[Path] = []
    for target in targets:
        path = CHUNKS_DIR / f"{target['id']}.embedded.json"
        if not path.exists():
            print(f"[store] missing embedded file: {path}", file=sys.stderr)
            return 1
        embedded_paths.append(path)

    rows = _load_embedded(embedded_paths)
    write_lancedb(rows, args.db, overwrite=args.overwrite)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
