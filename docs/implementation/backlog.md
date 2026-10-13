# Implementation Backlog

## Done

### M1 — Embedding (`crates/core`, `crates/embed-onnx`)
- `Embedding` type and `EmbeddingProvider` trait.
- EmbeddingGemma 2 via ONNX Runtime in Rust, no Python at runtime (ADR 0001).
- Model-card prompts for Queries and Chunks.
- Matryoshka truncation to configured dimensions + unit normalization.
- Batched embedding: one padded, masked model call per batch.
- Parity gate: matches PyTorch reference at cosine ≥ 0.99 (`tools/make_parity_fixtures.py`).
- Text beyond a configurable token limit is ignored (default 2,048; model max 8,192).
- Multilingual guard: English ↔ Indonesian Queries find related Chunks.
- Throughput runner `embed_throughput` (`crates/bench`); partial CPU fp32 result: ~2 Chunks/s at 256 tok (1M Chunks ≈ 140 h).

### M1 — Vector store (`crates/store-usearch`)
- `VectorStore` trait; USearch backend: add, remove, search, save + mmap view (ADR 0002).
- Recall@10 on synthetic vectors (search depth 128): f16 ≥ 0.95; i8 ≈ 0.88 (test ignored until real Benchmark).

### M1 — Benchmark and Public Corpus (`crates/bench`, `tools/`, `benchmark/public/`)
- `Location` (page, lines, time, whole file) with overlap; Benchmark TOML parser rejects malformed labels.
- File-view metrics: Recall@k, MRR, nDCG@k, Location Accuracy.
- `fetch_public_corpus.py`: sha256-verified, skips present files, retries 429.
- Public Corpus: 49 files, 66 MB (EN/ID PDFs, text, DOCX, code, images, audio, video).
- First 40 labelled Queries (24 en, 16 id); guard test checks Targets exist.

## In progress
- —

## Next (M1)
- Finish CPU benchmark (512 tok, batch 32) and compare int8/q4 models.
- Benchmark runner: embed Public Corpus, score semantic-only baseline (needs extractors or a text-only subset).
- Grow Benchmark to 50–100 Queries; more non-abstract PDF Targets.

## Deferred until development moves to Windows (RTX 4060 8 GB)
- GPU throughput: CUDA and DirectML execution providers.
- Windows builds of `ort` and USearch.
- Full-disk Initial Indexing time estimate.
