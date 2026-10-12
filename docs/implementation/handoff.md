# Handoff

Snapshot for the next session or agent. Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md).

## State (2026-10-12)
- M1 technical validation in progress. Embedding (`crates/embed-onnx`) and vector store (`crates/store-usearch`) are done; all tests green, one deliberately ignored (i8 recall).
- Nothing in progress. Next: Public Corpus + first Benchmark labels (needs user input, see below).

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits (`Embedding`, `EmbeddingProvider`, `VectorStore`). |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`, f16/i8, save + mmap view). |
| `crates/bench` | Benchmark math + `embed_throughput` runner. |
| `tools/` | Python dev tooling (uv): PyTorch parity fixture generator. Never shipped. |

## Run
- Tests: `cargo test --workspace` (Rust via rustup at `~/.cargo/bin`).
- Ignored tests: `cargo test -p sbm-store-usearch -- --ignored`.
- Benchmark: `cargo run --release -p sbm-bench --bin embed_throughput` (do not run tests at the same time).
- Parity fixtures: `cd tools && uv run make_parity_fixtures.py`.

## Model
- `onnx-community/embeddinggemma-2-ONNX` (Apache-2.0, not gated), stored outside the repo at `~/.cache/search-by-memory/models/embeddinggemma-2-onnx` (override: `SBM_MODEL_DIR`).
- Download: `uvx --from huggingface_hub hf download onnx-community/embeddinggemma-2-ONNX tokenizer.json tokenizer_config.json config.json onnx/model.onnx onnx/model.onnx_data --local-dir <dir>`.
- Model-dependent tests print `SKIPPED` and pass when the model is missing.

## Environment gotchas
- Dev machine: Linux, i7-1255U, no GPU. GPU/Windows work is deferred (see backlog).
- No OpenSSL headers: `ort` uses `tls-rustls`; `tokenizers` has default features off (`fancy-regex`).
- `tools/`: `torch` and `torchvision` must both come from the PyTorch CPU index; `sentence-transformers[image]` is required even for text.
- New dependencies are pinned to releases at least two weeks old (`ort =2.0.0-rc.13`, `usearch =2.26.2`, `tempfile =3.27.0`).

## Findings to carry forward
- CPU fp32 embedding ≈ 2 Chunks/s (256 tok); batching barely helps on CPU. Benchmark unfinished: 512 tok, batch 32, int8/q4 models.
- USearch needs search depth 128 for f16 recall@10 ≥ 0.95; i8 ≈ 0.88 (quantization loss).
- ONNX Runtime silently accepts input beyond 8,192 tokens; the embedder truncates at `DEFAULT_MAX_TOKENS` (2,048).
- EmbeddingGemma 2 is multimodal (text, image, video, audio in one space); may simplify P1/P2.

## Open questions for the user
1. Public Corpus contents and who drafts the first 20–30 labelled Queries.
2. f16 vs i8 vector precision (decide on real Benchmark data).
3. When to resume the benchmark (CPU int8/q4 now, GPU on Windows later).
