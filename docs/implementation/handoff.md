# Handoff

Snapshot for the next session or agent (2026-10-13, after commit `78b6c8d`). Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md); terms in [`CONTEXT.md`](../../CONTEXT.md).

## State
- **M1 Technical Validation**: done except benchmarks the user deferred ("we will benchmark later"): CPU throughput (512 tok, batch 32, int8/q4) and GPU on Windows.
- **M2 Name + Text Search MVP**: Name Index, extractors, chunking, Content Index, hybrid search, and the `sbm` CLI are done and smoke-tested with the real model.
- **Next M2 items**: volume-aware file catalog (volume ID per file); end-to-end Benchmark run through `sbm` (hybrid vs semantic-only), which waits for the benchmark session.
- All tests green (`cargo test --workspace`: 85; `cd tools && uv run pytest`: 12). One test is deliberately ignored (i8 recall). Working tree clean; local `main` is ahead of `origin/main` (not pushed).

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits: `Location`, `Embedding`, `Section`, `EmbeddingProvider`, `VectorStore`, `ContentExtractor`. |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`: f16/i8, `save`, memory-mapped `open_view`). |
| `crates/extract` | `TextExtractor`, `PdfExtractor` (pdf-extract), `DocxExtractor` (zip + quick-xml); `extractor_for(path)`; `chunk_sections` (sliding window, overlap). |
| `crates/index` | SQLite (rusqlite, bundled FTS5): file catalog + Name Index (`scan`, `search_names`, `Exclusion`), Content Index (`index_content`, `remove_unreferenced_content`, `rebuild_vectors`), hybrid `search` (File/Passage view, `MatchReason`). |
| `crates/cli` | `sbm` binary and the tested `index_folder` / `search` / `printable` functions. |
| `crates/bench` | Dev-only measurements: `embed_throughput`, `retrieval_benchmark`, Benchmark parser, File-view metrics, Location-miss report. |
| `benchmark/public/` | Public Corpus `manifest.toml` (74 files: URL, sha256, license) and `queries.toml` (58 labelled Queries). |
| `tools/` | Python dev tooling (uv): parity fixtures, `fetch_public_corpus.py` (incl. wikitext → plain text). Never shipped. |

## How search works (M2)
- `scan` walks an Indexed Folder (Default Exclusions, user folder/`*` patterns), mark-and-sweep against SQLite, and reports added / changed (size or mtime) / removed files.
- `index_content` hashes a file (BLAKE3, streamed); known content is reused (Copies, moves; ADR 0004). New content is extracted, chunked (1,000 chars, 150 overlap), embedded in batches of 8, stored in `chunks` (text + embedding BLOB) and `chunk_words` (FTS5), and added to USearch with the Chunk id as vector id.
- `search` fuses semantic (USearch) and keyword (FTS5) Chunk lists by Reciprocal Rank Fusion (k = 60); File view groups by content hash (Copies) and fuses with Name Matches; an exact filename match goes on top.
- `remove_unreferenced_content` runs after indexing a scan's added/changed files, so moved files reuse content before it is collected.
- Schema: `MIGRATIONS` in `crates/index/src/lib.rs`, tracked by `PRAGMA user_version`; append only, never edit. A newer schema is refused.
- Not done yet: index_jobs / resume, Live Updates, throttling (M4); volume IDs. Passage view intentionally omits name-only files.

## Run
- Rust: `export PATH="$HOME/.cargo/bin:$PATH"` first (cargo is not on the default PATH).
- Tests: `cargo test --workspace`; Python: `cd tools && uv run pytest`.
- CLI: `cargo build --release -p sbm-cli`, then `target/release/sbm index <folder>...` and `target/release/sbm search [--passages] <query>...`. Data: `$SBM_DATA_DIR` (default `~/.local/share/search-by-memory`). 256 dims, f16 vectors.
- Retrieval Benchmark: `cargo run --release -p sbm-bench --bin retrieval_benchmark` (still uses pdftotext; embeddings cached in `~/.cache/search-by-memory/bench-embeddings`; a fresh run takes 40+ min on CPU).
- Throughput: `cargo run --release -p sbm-bench --bin embed_throughput` (not while tests run).
- Public Corpus: `cd tools && uv run fetch_public_corpus.py` → `~/.cache/search-by-memory/public-corpus` (override `SBM_PUBLIC_CORPUS`).
- Parity fixtures: `cd tools && uv run make_parity_fixtures.py`.

## Model
- `onnx-community/embeddinggemma-2-ONNX` (Apache-2.0) at `~/.cache/search-by-memory/models/embeddinggemma-2-onnx` (override `SBM_MODEL_DIR`).
- Download: `uvx --from huggingface_hub hf download onnx-community/embeddinggemma-2-ONNX tokenizer.json tokenizer_config.json config.json onnx/model.onnx onnx/model.onnx_data --local-dir <dir>`.
- Model-dependent tests print `SKIPPED` and pass when the model is missing.

## Measurements so far
- CPU embedding on the real corpus ≈ 1 Chunk/s (i7-1255U, fp32).
- `sbm index` on 15 code files: 194 s, 2.0 GB peak (6.1 GB before batching). `sbm search`: 2.7 s cold, mostly model load; the app must keep the model loaded.
- Retrieval Benchmark (semantic only, 43 of 58 Queries scorable): Recall@5 = 1.0 even with distractors; MRR 0.93–0.95; Location Accuracy 0.67 @256, 0.56 @512, 0.60 @768. f16 matches exact; i8 loses 0.02–0.07. 256 dims looks sufficient (open decision, PRD 11.2).
- Location misses are mostly right file, wrong place (similar articles of a law, neighbouring code, README headers).

## Known gaps and caveats
- pdf-extract (and lopdf beneath it) returns no text for the scanned UU 27/2022 PDF with an invisible OCR layer; pdftotext reads it. Two Benchmark Queries (id-001, id-002) depend on it. Revisit the PDF backend when benchmarking, or OCR in M5.
- pdf-extract panics on many corrupt PDFs; `PdfExtractor` catches the panic (the panic message still prints to stderr).
- Text and DOCX extraction stop at 50 MB decompressed; PDFs have no size cap yet.
- Public Corpus labels: many arXiv Targets are page 1 (easy); UU 36/2009 and UU 13/2003 are superseded texts; licenses to spot-check: Tom Lehrer recordings (PD claim), arXiv CC BY (verified on abs pages only).
- Wikipedia files are fetched at a pinned revid as wikitext and converted with mwparserfromhell; manifest `source_sha256` checks the download, `sha256` the converted text. Changing the converter changes `sha256`: recompute and relabel affected line Targets.

## Environment gotchas
- Dev machine: Linux, i7-1255U, no GPU; GPU/Windows work is deferred.
- `ort` uses `tls-rustls` (no OpenSSL headers); `tokenizers` default features off.
- `tools/`: `torch`/`torchvision` from the PyTorch CPU index; `sentence-transformers[image]` required.
- New dependencies are pinned (`=x.y.z`) to releases at least two weeks old. Current pins include rusqlite 0.40.2, walkdir 2.5.0, blake3 1.8.7, pdf-extract 0.12.1, quick-xml 0.42.0, zip 8.6.0, mwparserfromhell 0.7.2.
- `gh` here points to a different host; use `git ls-remote` for GitHub commit SHAs. Plain `curl` is blocked by a hook; use the context-mode execute tool or Python `urllib` with the fetcher's User-Agent.
- A GateGuard hook asks for facts (request, purpose, importers, rollback) before first edits and destructive commands. Chained `&&` commands keep running when an earlier tool call was blocked, so check test results before committing.

## Open questions for the user
1. Review the 58 drafted Queries in `benchmark/public/queries.toml`.
2. License spot-checks above.
3. f16 vs i8 and 256 vs 512 dims: confirm with the deferred Benchmark runs.
4. PDF backend: keep pdf-extract, or move to pdfium/poppler for scanned PDFs.
5. When to resume benchmarking (CPU int8/q4 now, GPU on Windows later).
