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
- `retrieval_benchmark`: semantic-only File view over text, code, PDFs (pdftotext stand-in); embedding cache by file sha256.
- First run (2,442 Chunks, 25 Queries scored): Recall@5 = 1.0 at every dim/precision, MRR 0.96–0.98; Location Accuracy only 0.28–0.44. Corpus too small to separate f16/i8 or 256/512/768.
- Location-miss report: 14/25 misses, mostly right file wrong place (sibling code, similar Pasal, same deadline in another article), plus wikitext markup Chunks and generic file headers; 1 label fixed (en-004).
- Wikipedia files fetched as plain text (`convert = "wikitext"`, mwparserfromhell): Location Accuracy @256 0.44 → 0.52; en-102 label fixed.
- 25 near-neighbour distractors (wiki, code, laws, Gothic tales) and 18 harder Queries (58 total).
- Run with distractors (3,992 Chunks, 43 Queries scored): Recall@5 still 1.0, MRR 0.93–0.95; Location Accuracy 0.67 @256, 0.56 @512, 0.60 @768; i8 costs 0.02–0.07.

### M2 — Name Index (`crates/index`)
- SQLite FTS5 Name Index over filename and folder path; last Query word matches as a prefix.
- Exact filename (or stem) match ranks first; blank Query has no Name Matches.
- Default Exclusions (tooling folders), user folder Exclusions, `*` name patterns; Indexed Folder root never excluded.
- Rescan: mark-and-sweep; reports added, changed (size/mtime), removed files.
- On-disk database with `user_version` migrations; newer schema refused.

## In progress
- M2 extractors (`crates/extract`): text, PDF (pdf-extract), DOCX, failures, lock files; sliding-window chunker.

## Next (M1)
- Finish CPU benchmark (512 tok, batch 32) and compare int8/q4 models.
- File-level Recall still saturated: needs vaguer Queries or a much larger corpus (thousands of files).
- More non-abstract PDF Targets.

## Deferred until development moves to Windows (RTX 4060 8 GB)
- GPU throughput: CUDA and DirectML execution providers.
- Windows builds of `ort` and USearch.
- Full-disk Initial Indexing time estimate.
