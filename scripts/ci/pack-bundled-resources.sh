#!/usr/bin/env bash
# Pack gitignored bundled resources for CI (no GGUF).
# Output: faqih-bundled-resources.tar.gz in the repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RES="$ROOT/src-tauri/resources"
OUT="$ROOT/faqih-bundled-resources.tar.gz"

for f in \
  "$RES/models/tokenizer.json" \
  "$RES/models/multilingual-e5-small.onnx"
do
  if [[ ! -f "$f" ]]; then
    echo "Missing required file: $f" >&2
    exit 1
  fi
done

if [[ ! -d "$RES/lexuz.db" ]] || [[ -z "$(find "$RES/lexuz.db" -mindepth 1 ! -name '.gitkeep' | head -1)" ]]; then
  echo "Missing populated lexuz.db — run scripts/ingestion/run_pipeline.py first." >&2
  exit 1
fi

tar -C "$RES" -czf "$OUT" \
  models/tokenizer.json \
  models/multilingual-e5-small.onnx \
  lexuz.db

echo "Wrote $OUT ($(du -h "$OUT" | awk '{print $1}'))"
echo "Host this file and set GitHub Actions secret FAQIH_BUNDLE_RESOURCES_URL to its URL."
