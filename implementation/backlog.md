# Implementation Backlog

## Done

### M1 — Embedding (`crates/core`, `crates/embed-onnx`)
- `Embedding` type and `EmbeddingProvider` trait.
- EmbeddingGemma 2 via ONNX Runtime in Rust, no Python at runtime (ADR 0001).
- Model-card prompts for Queries and Chunks.
- Matryoshka truncation to configured dimensions + unit normalization.
- Batched embedding: one padded, masked model call per batch.
- Parity gate: matches PyTorch reference at cosine ≥ 0.99 (`tools/make_parity_fixtures.py`).

## In progress
- Overlong input is truncated to the model limit without error.

## Next (M1)
- Cross-lingual retrieval test.
- Embedding throughput benchmark: CPU/GPU, Linux then Windows RTX 4060.
- USearch validation: insert, remove, mmap, `f16`/`i8` recall.
- Public Corpus and first Benchmark labels.
