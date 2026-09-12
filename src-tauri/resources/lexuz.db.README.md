# lexuz.db

LanceDB database directory produced by `scripts/ingestion/` (MVP: Labor Code + Civil Code only).

```bash
cd scripts/ingestion
pip install torch --index-url https://download.pytorch.org/whl/cpu
pip install -r requirements.txt
python run_pipeline.py
```

The runtime app opens this DB **read-only**. Never write to it from the desktop app.

Until the pipeline has been run, `store.py` has not created this path yet — the checked-in `lexuz.db` placeholder file (if present) should be replaced by the LanceDB directory output.
