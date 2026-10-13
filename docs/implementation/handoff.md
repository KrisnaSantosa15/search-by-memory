# Handoff

Snapshot for the next session or agent. Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md).

## State (2026-10-13)
- M1 technical validation in progress. Embedding, vector store, Benchmark tooling and Public Corpus are done; all tests green, one deliberately ignored (i8 recall).
- Nothing in progress. Next: the unfinished CPU throughput benchmark, or a much larger corpus so file-level Recall stops saturating.

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits (`Embedding`, `EmbeddingProvider`, `VectorStore`). |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`, f16/i8, save + mmap view). |
| `crates/bench` | Throughput math + `embed_throughput` runner; Benchmark parser (`Benchmark`, `Target`) and File-view metrics (`Scores`). |
| `benchmark/public/` | Public Corpus `manifest.toml` (URL, sha256, license) and labelled `queries.toml`. |
| `tools/` | Python dev tooling (uv): parity fixtures, `fetch_public_corpus.py`. Never shipped. |

## Run
- Tests: `cargo test --workspace` (Rust via rustup at `~/.cargo/bin`).
- Ignored tests: `cargo test -p sbm-store-usearch -- --ignored`.
- Benchmark: `cargo run --release -p sbm-bench --bin embed_throughput` (do not run tests at the same time).
- Parity fixtures: `cd tools && uv run make_parity_fixtures.py`.
- Retrieval Benchmark: `cargo run --release -p sbm-bench --bin retrieval_benchmark` (first run ≈ 40 min CPU; cached in `~/.cache/search-by-memory/bench-embeddings`).
- Public Corpus: `cd tools && uv run fetch_public_corpus.py` → `~/.cache/search-by-memory/public-corpus` (override: `SBM_PUBLIC_CORPUS`). Corpus-dependent test prints `SKIPPED` when absent.

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
- Benchmark is for development only; the user's own files come in as a Personal Corpus later (labels outside git).
- Public Corpus label caveats: most arXiv Targets are page 1 (abstract) — too easy; media time ranges verified from subtitles/frames; UU 36/2009 and UU 13/2003 are superseded/amended texts.
- Real-corpus CPU embedding ≈ 1 Chunk/s (1,000-char Chunks, batch 8), half the synthetic estimate.
- Retrieval runs: file-level Recall@5 = 1.0 even with 25 near-neighbour distractors; Location Accuracy (0.56–0.67) is the weak spot, mostly right file but wrong place. 256 dims ≥ 512/768 on this corpus; i8 loses 0.02–0.07 Location Accuracy, f16 matches exact.
- Wikipedia files are fetched as raw wikitext (pinned revid) and converted to plain text; manifest `source_sha256` checks the download, `sha256` the converted file.
- Wikimedia rate-limits bursts (429); the fetcher honours Retry-After.

## Open questions for the user
1. Review the 58 drafted Queries in `benchmark/public/queries.toml`.
2. License spot-checks flagged by gathering: Tom Lehrer recordings (PD claim), arXiv CC BY verified on abs pages only.
3. f16 vs i8 vector precision (decide on real Benchmark data).
4. When to resume the benchmark (CPU int8/q4 now, GPU on Windows later).
