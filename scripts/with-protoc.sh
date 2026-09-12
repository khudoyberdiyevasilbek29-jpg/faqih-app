#!/usr/bin/env bash
# Ensure protoc is visible to LanceDB / prost-build.
# Prefer a PROTOC path *without spaces* — prost-build can fail to spawn
# binaries when PROTOC contains whitespace (project dir is "Faqih AI").
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORTABLE="$ROOT/src-tauri/.tools/protoc/bin/protoc"
NOSPACE="${HOME}/.local/bin/protoc"

if [[ ! -x "$PORTABLE" ]]; then
  echo "error: missing portable protoc at $PORTABLE" >&2
  exit 1
fi

mkdir -p "${HOME}/.local/bin"
if [[ ! -e "$NOSPACE" ]]; then
  ln -sfn "$PORTABLE" "$NOSPACE"
fi

export PROTOC="$NOSPACE"
export PATH="${HOME}/.local/bin:$(dirname "$PORTABLE"):${PATH}"
exec "$@"
