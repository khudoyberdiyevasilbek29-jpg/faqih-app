# Faqih AI by MOND

**100% offline** desktop legal assistant for Uzbekistan legislation (MVP: Labor Code + Civil Code).

Built with **Tauri v2**, **React 18 + TypeScript**, and **Rust**. All inference runs on-device — no cloud AI providers in the runtime app.

## Status

Fresh scaffold replacing the previous prototype. Current milestone:

- [x] Tauri v2 + React/TS + Tailwind shell
- [x] Typed `src/lib/api.ts` invoke wrapper
- [x] `hello_world` IPC end-to-end
- [x] Module stubs for LLM / embeddings / RAG / LanceDB
- [ ] Wire `llama-cpp-2` + GGUF first-run picker
- [ ] Wire ONNX e5 embeddings + real tokenizer
- [x] Offline ingestion pipeline (`scripts/ingestion/`) for Labor + Civil codes
- [ ] Open read-only `lexuz.db` and chat RAG
- [ ] Document contradiction analysis

## Stack

| Layer | Choice |
|-------|--------|
| Shell | Tauri v2 (`com.mond.faqihai`) |
| UI | React 18, Vite, Tailwind, Lucide, Framer Motion, Zustand |
| LLM | `llama-cpp-2` **0.1.156** (pinned at setup) + user GGUF |
| Embeddings | `ort` + multilingual-e5-small ONNX |
| Tokenizer | `tokenizers` + bundled `tokenizer.json` |
| Vectors | LanceDB (`resources/lexuz.db`, read-only) |

## Prerequisites

- Node.js 20+
- Rust stable + system deps for Tauri ([guide](https://v2.tauri.app/start/prerequisites/))
- `clang` (llama-cpp-2 bindgen) and `protoc` (LanceDB build). A portable protoc can live at `src-tauri/.tools/protoc/bin/protoc` with `PROTOC` set.
- Bundled resources: `tokenizer.json` + `multilingual-e5-small.onnx` under `src-tauri/resources/models/`
- User-provided GGUF (Qwen2.5-1.5B/3B-Instruct) selected at first run

## Develop

See [BUILD.md](./BUILD.md) for Linux packaging, release-profile tradeoffs, and **Windows/macOS GitHub Actions installers** (Actions → *Build installers* → Artifacts).

```bash
npm install
npm run tauri dev
```

Frontend-only:

```bash
npm run dev
```

Verify TypeScript + Vite build:

```bash
npm run build
```

Verify Rust (IPC + stubs):

```bash
cd src-tauri && cargo check
```

## Architecture

- Frontend talks to Rust **only** through `src/lib/api.ts`.
- AI lives under `src-tauri/src/ai/` (`llm`, `embeddings`, `rag`).
- Vector access: `src-tauri/src/db/vector_store.rs`.
- Ingestion pipeline: `scripts/ingestion/` (offline; not part of the running app).
- Constraints & known-bug preventions: see [`.cursorrules`](./.cursorrules).

## Resources

- `src-tauri/resources/models/` — bundle `multilingual-e5-small.onnx` + `tokenizer.json` (see README there).
- Generative **AI model** is downloaded once on first launch into the app data directory (~1 GB). Power users can still pick a custom file under Settings → Kengaytirilgan.
- `src-tauri/resources/lexuz.db` — LanceDB built by `scripts/ingestion/` (Labor + Civil only for MVP).

## Build the legal vector DB (offline, manual)

```bash
cd scripts/ingestion
python3 -m venv .venv && source .venv/bin/activate
pip install torch --index-url https://download.pytorch.org/whl/cpu   # CPU torch FIRST
pip install -r requirements.txt
# Re-verify URLs in scrape_targets.yaml, then:
python run_pipeline.py
```

See [`scripts/ingestion/README.md`](./scripts/ingestion/README.md) for politeness rules, change detection, and validation.

## Offline policy

The runtime app must never call OpenAI, Anthropic, Gemini, Groq, or any other hosted LLM API. Reject proposals to add them.
# Faqih-AI
# faqih.ai
