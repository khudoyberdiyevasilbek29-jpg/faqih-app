#!/usr/bin/env python3
"""
Embed chunks with multilingual-e5-small (sentence-transformers).

ALWAYS apply the e5 passage prefix: "passage: "

Install note (CPU-only — do this BEFORE requirements.txt):
  pip install torch --index-url https://download.pytorch.org/whl/cpu
  pip install -r requirements.txt
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from paths_util import (
    CHUNKS_DIR,
    EMBEDDING_MODEL,
    PASSAGE_PREFIX,
    ensure_dirs,
    load_targets,
)


def _load_model(model_name: str = EMBEDDING_MODEL):
    try:
        from sentence_transformers import SentenceTransformer
    except ImportError as exc:
        raise SystemExit(
            "sentence-transformers is not installed.\n"
            "CPU-only install order:\n"
            "  pip install torch --index-url https://download.pytorch.org/whl/cpu\n"
            "  pip install -r requirements.txt\n"
        ) from exc

    print(f"[embed] loading {model_name} (CPU)…")
    return SentenceTransformer(model_name, device="cpu")


def embed_chunks(
    chunks: list[dict[str, Any]],
    model,
    *,
    batch_size: int = 32,
) -> list[dict[str, Any]]:
    texts = [f"{PASSAGE_PREFIX}{c['text']}" for c in chunks]
    vectors = model.encode(
        texts,
        batch_size=batch_size,
        show_progress_bar=True,
        normalize_embeddings=True,
        convert_to_numpy=True,
    )
    embedded: list[dict[str, Any]] = []
    for chunk, vector in zip(chunks, vectors, strict=True):
        row = dict(chunk)
        row["embedding_vector"] = vector.astype(float).tolist()
        embedded.append(row)
    return embedded


def embed_file(chunks_path: Path, model, *, batch_size: int) -> Path:
    payload = json.loads(chunks_path.read_text(encoding="utf-8"))
    chunks = payload.get("chunks", [])
    if not chunks:
        raise ValueError(f"No chunks in {chunks_path}")

    embedded = embed_chunks(chunks, model, batch_size=batch_size)
    ensure_dirs()
    stem = chunks_path.name.replace(".chunks.json", "")
    out = CHUNKS_DIR / f"{stem}.embedded.json"
    out.write_text(
        json.dumps(
            {
                "id": payload.get("id"),
                "law_name": payload.get("law_name"),
                "model": EMBEDDING_MODEL,
                "prefix": PASSAGE_PREFIX,
                "chunk_count": len(embedded),
                "chunks": embedded,
            },
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"[embed] {stem}: {len(embedded)} vectors → {out}")
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Embed chunk JSON with e5-small")
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument("--model", default=EMBEDDING_MODEL)
    args = parser.parse_args(argv)

    ensure_dirs()
    from paths_util import TARGETS_PATH

    config = load_targets(TARGETS_PATH)
    targets = config["targets"]
    if args.only:
        wanted = set(args.only)
        targets = [t for t in targets if t["id"] in wanted]

    model = _load_model(args.model)
    for target in targets:
        chunks_path = CHUNKS_DIR / f"{target['id']}.chunks.json"
        if not chunks_path.exists():
            print(f"[embed] missing chunks: {chunks_path}", file=sys.stderr)
            return 1
        embed_file(chunks_path, model, batch_size=args.batch_size)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
