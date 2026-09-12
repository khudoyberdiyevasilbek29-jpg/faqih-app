# Building Faqih AI

**Faqih AI by MOND** is packaged with **Tauri v2**. The MVP packaging target is **Linux** (current primary dev/test environment). GGUF weights are **not** bundled — users pick a local Qwen2.5 Instruct GGUF on first run.

## Bundled resources (not GGUF)

Configured in `src-tauri/tauri.conf.json` → `bundle.resources`:

| Source | Bundled as |
|--------|------------|
| `resources/models/tokenizer.json` | `models/tokenizer.json` |
| `resources/models/multilingual-e5-small.onnx` | `models/multilingual-e5-small.onnx` |
| `resources/lexuz.db/` (LanceDB directory) | `lexuz.db/` |

At startup the Rust side resolves `$RESOURCE` (then falls back to `CARGO_MANIFEST_DIR/resources` for `tauri dev`).

## Release profile tradeoff

`src-tauri/Cargo.toml` `[profile.release]`:

- `opt-level = 3`
- `lto = true`
- `codegen-units = 1`
- `strip = true`
- `panic = "abort"`

**Runtime:** faster inference binary, smaller artifact.  
**Compile time:** full release links are much slower (often many minutes) because of LTO + a single codegen unit. Use `npm run tauri dev` / `cargo check` for daily work; use release only for packaging or perf checks.

## Linux (primary — do this now)

### Prerequisites

- Node.js 20+
- Rust stable
- Tauri Linux system deps: https://v2.tauri.app/start/prerequisites/#linux
- `clang` / `libclang` (llama-cpp-2 bindgen)
- `protoc` on `PATH` **without whitespace in the path**. The folder name
  `Faqih AI` breaks prost-build if `PROTOC` points into the repo directly.
  Use either:
  - `npm run tauri:dev` (runs `scripts/with-protoc.sh`, symlinks to
    `~/.local/bin/protoc`), or
  - `sudo dnf install -y protobuf-compiler` (Fedora system package).

```bash
# optional system install
sudo dnf install -y protobuf-compiler
```
- Built `lexuz.db` via `scripts/ingestion/` and ONNX + `tokenizer.json` under `src-tauri/resources/models/`

### Dev

```bash
cd "/path/to/Faqih AI"
npm install
npm run tauri:dev
```

(`PROTOC` is set automatically by `src-tauri/.cargo/config.toml`.)

### Package (deb + AppImage)

`tauri.conf.json` sets `"targets": ["deb", "appimage"]`.

```bash
cd "/path/to/Faqih AI"
npm run tauri:build
```

Artifacts land under `src-tauri/target/release/bundle/` (e.g. `deb/`, `appimage/`).

### Smoke checklist after a Linux build

1. Very first launch: 3-screen Uzbek onboarding (Davom etish / O'tkazib yuborish), then model download if needed.
2. Relaunch: onboarding must not appear again (`hasSeenOnboarding`).
3. After download, confirm chat feels calm (cream UI, terracotta primary button only).
4. Ask a Labor/Civil question; confirm streaming + Manbalar.
5. Leave idle **10+ minutes**; next message shows **Uyg'onmoqda...**.

---

## TODO: Windows / macOS

> **Not executed for MVP.** Cross-compile from Linux is unreliable for this stack (`llama-cpp-2`, `ort`, LanceDB native bits). Prefer **native runners** (GitHub Actions `windows-latest` / `macos-latest`, or local Mac/Windows boxes) when packaging those platforms.

### Likely next steps (when prioritized)

1. Extend `bundle.targets` (e.g. `nsis` / `msi` on Windows, `dmg` / `app` on macOS) — or temporarily set `"all"` on a native CI matrix.
2. Verify resource map copies `lexuz.db/` recursively on each OS (LanceDB is a directory tree).
3. Confirm `sysinfo` process memory units and path separators for the GGUF picker.
4. Document MSVC + CUDA/CPU ORT notes for Windows; Xcode + notarization for macOS (if distributing outside direct builds).
5. Re-run the idle-unload + wake (“Uyg'onmoqda...”) checklist on each OS.

Until then, treat Linux `deb` / `AppImage` as the only supported ship path.
