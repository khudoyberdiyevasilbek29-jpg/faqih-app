#!/usr/bin/env python3
"""
Run the full offline ingestion pipeline (manual batch only).

  scrape → clean → chunk → embed → store → validate

Never called by the Faqih AI desktop runtime.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def _run(script: str, extra: list[str] | None = None) -> None:
    cmd = [sys.executable, str(ROOT / script), *(extra or [])]
    print(f"\n>>> {' '.join(cmd)}\n")
    subprocess.run(cmd, check=True, cwd=str(ROOT))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Faqih AI offline ingestion pipeline")
    parser.add_argument("--skip-scrape", action="store_true")
    parser.add_argument("--skip-validate", action="store_true")
    parser.add_argument("--force-scrape", action="store_true")
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--overwrite-db", action="store_true", default=True)
    args = parser.parse_args(argv)

    only = ["--only", *args.only] if args.only else []

    if not args.skip_scrape:
        scrape_extra = list(only)
        if args.force_scrape:
            scrape_extra.append("--force")
        _run("scrape.py", scrape_extra)

    _run("clean.py", only)
    _run("chunking.py", only)
    _run("embed.py", only)

    store_extra = list(only)
    if args.overwrite_db:
        store_extra.append("--overwrite")
    _run("store.py", store_extra)

    if not args.skip_validate:
        _run("validate.py")

    print("\n[pipeline] complete.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
