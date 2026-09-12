#!/usr/bin/env python3
"""
Sanity-check the built LanceDB with a handful of legal sample queries.

Prints top-5 hits with metadata + similarity scores for manual review.
Uses e5 "query: " prefix (must match runtime RAG).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from paths_util import (
    DEFAULT_DB_PATH,
    EMBEDDING_MODEL,
    QUERY_PREFIX,
)

TABLE_NAME = "lexuz_articles"

SAMPLE_QUERIES = [
    "Mehnat shartnomasini bekor qilish asoslari",
    "Ish vaqti va dam olish vaqti",
    "Xodimning mehnat tatili huquqi",
    "Mulk huquqi va uni himoya qilish",
    "Oldi-sotdi shartnomasi majburiyatlari",
    "Zararni qoplash asoslari",
]


def _table_names(db) -> list[str]:
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


def _load_model(model_name: str = EMBEDDING_MODEL):
    from sentence_transformers import SentenceTransformer

    return SentenceTransformer(model_name, device="cpu")


def validate(db_path: Path, queries: list[str], *, top_k: int = 5) -> int:
    import lancedb

    if not db_path.exists():
        print(f"[validate] DB not found: {db_path}", file=sys.stderr)
        return 1

    db = lancedb.connect(str(db_path))
    if TABLE_NAME not in _table_names(db):
        print(f"[validate] table {TABLE_NAME!r} missing in {db_path}", file=sys.stderr)
        return 1

    table = db.open_table(TABLE_NAME)
    model = _load_model()

    print(f"[validate] db={db_path}")
    print(f"[validate] rows≈{table.count_rows()}  model={EMBEDDING_MODEL}\n")

    for query in queries:
        qvec = model.encode(
            f"{QUERY_PREFIX}{query}",
            normalize_embeddings=True,
            convert_to_numpy=True,
        )
        results = (
            table.search(qvec.tolist(), vector_column_name="embedding_vector")
            .metric("cosine")
            .limit(top_k)
            .to_list()
        )

        print("=" * 72)
        print(f"QUERY: {query}")
        if not results:
            print("  (no results)")
            continue

        for rank, row in enumerate(results, start=1):
            distance = row.get("_distance", row.get("distance"))
            similarity = None
            if isinstance(distance, (int, float)):
                similarity = 1.0 - float(distance)
            print(f"  #{rank}")
            print(f"    law:      {row.get('law_name')}")
            print(f"    article:  {row.get('article_number')}")
            print(f"    chapter:  {row.get('chapter')}")
            print(f"    section:  {row.get('section_title')}")
            if similarity is not None:
                print(f"    score:    sim={similarity:.4f}  dist={float(distance):.4f}")
            text = (row.get("text") or "").replace("\n", " ")
            print(f"    text:     {text[:240]}{'…' if len(text) > 240 else ''}")
        print()

    print("[validate] Done — review rankings manually before shipping the DB in-app.")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate lexuz.db with sample queries")
    parser.add_argument("--db", type=Path, default=DEFAULT_DB_PATH)
    parser.add_argument("--top-k", type=int, default=5)
    parser.add_argument("--query", action="append", help="Extra / override queries")
    args = parser.parse_args(argv)

    queries = args.query if args.query else SAMPLE_QUERIES
    return validate(args.db, queries, top_k=args.top_k)


if __name__ == "__main__":
    raise SystemExit(main())
