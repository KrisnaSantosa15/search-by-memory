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

## In progress
- Embedding throughput benchmark on Linux CPU (i7-1255U, no GPU).

## Next (M1)
- USearch validation: insert, remove, mmap, `f16`/`i8` recall.
- Public Corpus and first Benchmark labels.

## Deferred until development moves to Windows (RTX 4060 8 GB)
- GPU throughput: CUDA and DirectML execution providers.
- Windows builds of `ort` and USearch.
- Full-disk Initial Indexing time estimate.
