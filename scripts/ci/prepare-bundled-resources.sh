#!/usr/bin/env bash
# Fetch/install bundled runtime resources for packaging.
# Does NOT download or include any GGUF — those are user-downloaded at first run.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RES="$ROOT/src-tauri/resources"
MODELS="$RES/models"
DB="$RES/lexuz.db"

mkdir -p "$MODELS" "$DB"

need_pack=0
[[ -f "$MODELS/tokenizer.json" ]] || need_pack=1
[[ -f "$MODELS/multilingual-e5-small.onnx" ]] || need_pack=1
# LanceDB is a directory tree; require at least one .lance child or data file
if [[ ! -d "$DB" ]] || [[ -z "$(find "$DB" -mindepth 1 ! -name '.gitkeep' 2>/dev/null | head -1)" ]]; then
  need_pack=1
fi

if [[ "$need_pack" -eq 0 ]]; then
  echo "Bundled resources already present under src-tauri/resources/"
  ls -lh "$MODELS/tokenizer.json" "$MODELS/multilingual-e5-small.onnx" || true
  du -sh "$DB" || true
  exit 0
fi

if [[ -z "${FAQIH_BUNDLE_RESOURCES_URL:-}" ]]; then
  cat >&2 <<'EOF'
Missing bundled resources for CI packaging.

These files are gitignored (large / regenerated) and are NOT GGUF:
  - src-tauri/resources/models/tokenizer.json
  - src-tauri/resources/models/multilingual-e5-small.onnx
  - src-tauri/resources/lexuz.db/   (LanceDB directory)

Fix: create an archive locally and host it, then set the repo secret
FAQIH_BUNDLE_RESOURCES_URL to that .tar.gz URL:

  bash scripts/ci/pack-bundled-resources.sh
  # upload faqih-bundled-resources.tar.gz somewhere private/public
  # GitHub → Settings → Secrets → Actions → FAQIH_BUNDLE_RESOURCES_URL

Archive layout (paths relative to src-tauri/resources/):
  models/tokenizer.json
  models/multilingual-e5-small.onnx
  lexuz.db/...
EOF
  exit 1
fi

echo "Downloading bundled resources from FAQIH_BUNDLE_RESOURCES_URL…"
tmp="$(mktemp -d)"
archive="$tmp/resources.tar.gz"
curl -fsSL "$FAQIH_BUNDLE_RESOURCES_URL" -o "$archive"
tar -xzf "$archive" -C "$RES"

if [[ ! -f "$MODELS/tokenizer.json" || ! -f "$MODELS/multilingual-e5-small.onnx" ]]; then
  echo "Archive extracted but models/ files are missing. Check archive layout." >&2
  exit 1
fi

echo "Resources ready:"
ls -lh "$MODELS/tokenizer.json" "$MODELS/multilingual-e5-small.onnx"
du -sh "$DB"
