# Offline legislation ingestion pipeline

Builds `src-tauri/resources/lexuz.db` (LanceDB) from **Labor Code** + **Civil Code** only.

This pipeline is a **manually triggered batch job**. It is **not** part of the Faqih AI desktop runtime and must never be called from the app.

## MVP scope

| Code | lex.uz targets (config) |
|------|-------------------------|
| Mehnat kodeksi (Labor Code) | `labor_code` |
| Fuqarolik kodeksi (Civil Code) | `civil_code_part1` + `civil_code_part2` |

Civil Code is one statute published as **two** lex.uz documents. Do **not** add Tax Code or others yet — edit `scrape_targets.yaml` only when expanding scope.

**Re-verify URLs** in `scrape_targets.yaml` before each scrape; lex.uz document IDs are not stable long-term.

## CPU-only install (required order)

```bash
cd scripts/ingestion
python3 -m venv .venv
source .venv/bin/activate

# 1) CPU torch FIRST — avoids multi-GB CUDA pulls
pip install torch --index-url https://download.pytorch.org/whl/cpu

# 2) then the rest
pip install -r requirements.txt
```

## Pipeline stages

| Step | Script | Output |
|------|--------|--------|
| 1. Scrape + change detection | `scrape.py` | `raw_data/*.html`, `.snapshot_manifest.json`, `archive/`, `CHANGELOG.md` |
| 2. Clean | `clean.py` | `cleaned/*.json` (articles / modda) |
| 3. Chunk | `chunking.py` | `chunks/*.chunks.json` |
| 4. Embed | `embed.py` | `chunks/*.embedded.json` (`passage: ` prefix) |
| 5. Store | `store.py` | `../../src-tauri/resources/lexuz.db` |
| 6. Validate | `validate.py` | top-5 sample query hits (manual review) |

Or run everything:

```bash
python run_pipeline.py
```

Useful flags:

```bash
python scrape.py --only labor_code
python run_pipeline.py --skip-scrape          # reuse raw_data/
python run_pipeline.py --force-scrape
python validate.py --query "mehnat ta'tili"
```

## Politeness (scrape.py)

- Honest User-Agent from `scrape_targets.yaml`
- Respects `robots.txt` (`lex.uz` currently sets **Crawl-delay: 20** — that wins over the 2–3s floor)
- Retry with exponential backoff (few attempts)
- Delay between every request

Expect a full scrape of three pages to take roughly a minute+ of polite waiting, not seconds.

## Change detection

For each target, `scrape.py` SHA-256-hashes the response body and compares against `raw_data/.snapshot_manifest.json`.

- **Unchanged** → skip rewrite
- **Changed** → move previous HTML into `archive/<timestamp>/<id>/`, append `CHANGELOG.md`, then save the new snapshot  
  Never silently overwrite without archiving.

## Chunking rules

1. Prefer one chunk per **modda** (article)
2. Preserve `article_number`, `chapter`, `section_title`, `law_name`
3. Oversized articles only: sliding window ~**300–500** tokens, ~**50** token overlap

## Embeddings

- Model: `intfloat/multilingual-e5-small`
- Documents: prefix `passage: `
- Validation queries: prefix `query: `
- Same conventions the Rust runtime must use with ONNX + `tokenizers`

## LanceDB schema

```
id                utf8
text              utf8
law_name          utf8
article_number    utf8
chapter           utf8
section_title     utf8
embedding_vector  fixed-size list<float32>[384]
```

Table name: `lexuz_articles`

## LanceDB version coupling (known gotcha)

**The Python and Rust `lancedb` versions must be kept in sync**, or the built
`lexuz.db` will not be readable at runtime.

LanceDB embeds Lance index formats that are **not** always
forward/backward compatible. A typical failure looks like:

```text
Search failed: lance error: LanceError(IO): Execution error:
LanceError(Index): unsupported index version (maybe need to upgrade your
lance version)
```

That means the DB (written by `store.py`) used a newer index format than the
Rust `lancedb` crate in `src-tauri/Cargo.toml` can read.

| Side | Where pinned | Current (this repo) |
|------|----------------|---------------------|
| Python ingestion | `scripts/ingestion/requirements.txt` | **`lancedb==0.38.0`** (installed in `.venv`) |
| Rust runtime | `src-tauri/Cargo.toml` → `Cargo.lock` | **`lancedb = "0.38"` → 0.38.0** (pulls `lance` **11.0.0**; enable `remote` feature — upstream `Error::Http` compile gate) |

**Resolved:** Rust and Python both on the 0.38 line. Rebuild `resources/lexuz.db` after any LanceDB bump so the on-disk index matches.

## After a successful validate

1. Manually read `validate.py` top-5 rankings for the sample queries
2. Confirm `src-tauri/resources/lexuz.db` is the LanceDB directory the app will open **read-only**
3. Confirm Python `lancedb` version == Rust `lancedb` crate version (see table above)
4. Only then treat RAG as ready in the desktop runtime
