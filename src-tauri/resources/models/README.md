# Bundled embedding assets

Place these files here before enabling the embedding engine:

| File | Source |
|------|--------|
| `multilingual-e5-small.onnx` | Export of [`intfloat/multilingual-e5-small`](https://huggingface.co/intfloat/multilingual-e5-small) to ONNX |
| `tokenizer.json` | The model's Hugging Face `tokenizer.json` (real tokenizer — never invent one) |

## GGUF LLM (NOT bundled)

The generative model is **user-provided**. Supported targets:

- Qwen2.5-1.5B-Instruct (GGUF quantized)
- Qwen2.5-3B-Instruct (GGUF quantized)

On first launch the app opens a file picker; the chosen path is persisted under the Tauri app data directory as JSON. Do not commit GGUF weights to this repo.
