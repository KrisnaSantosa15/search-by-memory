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

### M2 — Extractors and chunking (`crates/extract`)
- `ContentExtractor` + `Section` in core; text/Markdown/code (lossy UTF-8), PDF per page (pdf-extract), DOCX paragraphs as lines (zip + quick-xml, entities resolved).
- Bad files never crash indexing: pdf-extract panics caught; text and DOCX capped at 50 MB decompressed (zip bomb).
- `extractor_for`: picks by extension; Office `~$` lock files and unsupported formats skipped.
- Sliding-window chunker: whole lines, overlap, line/page Locations kept, blank Chunks dropped.
- Known gap: pdf-extract/lopdf return no text for the OCR layer of UU 27/2022 (pdftotext reads it).

### M2 — Content Index, hybrid search, CLI (`crates/index`, `crates/cli`)
- Chunks keyed by BLAKE3 content hash (ADR 0004); Copies and moved files reuse Chunks; edited files re-chunked; unreferenced content removed with its vectors.
- Embeddings stored in SQLite; vector index rebuilt from them without re-embedding.
- Hybrid search: semantic (USearch) + keyword (FTS5 over Chunks) + Name Index, merged by Reciprocal Rank Fusion (k = 60); exact filename on top; Match Reason name/content/both.
- File view (one Result per file, Copies grouped) and Passage view (one per Content Match).
- Embedding in batches of 8; hashing streamed.
- `sbm index <folder>` / `sbm search [--passages]`: bad files reported and skipped; terminal control characters escaped.
- Smoke test (Public Corpus `code/`, 15 files, real model, CPU): index 194 s, 2.0 GB peak (6.1 GB before batching); search 2.7 s cold incl. model load.

### M2 — Volume-aware file catalog (`crates/core`, `crates/index`)
- `Volumes` adapter trait; `SystemVolumes`: Windows volume GUID (network shares: their root), Unix mount point.
- Files keyed by volume ID + forward-slash relative path (schema 4 clears the catalog; rescan reuses Chunks by content hash).
- Drive back under another letter: no changes reported, paths follow the new mount point.
- Disconnected Drive: files stay findable at their last-known path, `connected: false`; `sbm` prints "drive not connected".
- Scanning one drive never removes another drive's files.

### M4 — Indexing core (`crates/index`, `crates/cli`)
- Saved Indexed Folders (by volume) and Exclusions; `scan_all` skips folders on a Disconnected Drive.
- `index_jobs` queue: newest mtime first, one commit per file (resume), 3 tries then failed with error.
- Jobs on a Disconnected Drive wait; content cleanup only once the queue is empty (moves reuse Chunks).
- Removing an Indexed Folder drops its files unless another Indexed Folder covers them.
- `sbm index [<folder>...]` (vectors saved every 100 files), `sbm folders`, `sbm remove`, `sbm exclude <folder> | --name <pattern>`.
- Smoke-tested on Windows: kill -9 mid-run, rerun finishes the 16 pending files.
- Search skips vectors whose Chunk is gone (vector file saved before a crash).

### M4 — Live Updates (`crates/core`, `crates/index`, `crates/change-usn`, `crates/cli`)
- `ChangeSource` trait: changed paths since a cursor, or Lost (rescan); a folder stands for everything beneath it.
- `Index::apply_changes`: re-checks only reported paths in Indexed Folders, minus Exclusions; cursor per volume saved with the changes.
- Live Updates queued ahead of Initial Indexing work; deletes leave both indexes; renamed folders reuse Chunks.
- Lost history, new Indexed Folder, or new Exclusion rescans the volume; Disconnected Drives keep their cursor.
- `UsnJournal` adapter: unprivileged read (no admin), paths by file ID; deletes and old names report their folder; no journal (e.g. network share) → Lost.
- `sbm index` uses the USN Journal instead of rescanning.
- Measured: 88k journal records read in 0.43 s; 2,000 changed paths on a 50k-file catalog applied in 0.13 s (was 68 s before batching).
- Smoke-tested `sbm` on Windows: add, folder rename, and delete picked up between runs.

### M4 — Online-only Files (`crates/core`, `crates/index`)
- `FileAvailability` adapter; `SystemFileAvailability` reads Windows recall/offline attributes without opening the file (other platforms: never Online-only).
- Online-only Files enter the Name Index only; queued content work is dropped when a file moves to the cloud.
- A download (size and mtime unchanged) is noticed on the next scan or Live Update and content-indexed.

### Windows port
- Builds with MSVC: usearch's NumKong linked statically (`.cargo/config.toml`); binary fixtures exempt from CRLF (`.gitattributes`).
- `AppDirs`: data in `%LOCALAPPDATA%\search-by-memory`, cache (models, Public Corpus) in its `cache` folder; Unix unchanged. CLI, benches, tests, and `tools/` use it.
- Index paths normalized: `C:/…` and `\\?\C:\…` spellings are the same file.
- Rust suite green on Windows (89 passed, 1 ignored); Python 13 passed.

### GPU embedding (`crates/embed-onnx`, `crates/bench`)
- `Accelerator` (`cpu`, `directml`, `directml:<n>`) and `OnnxEmbedder::load_on`; CPU fallback when the GPU cannot be used.
- DirectML passes the PyTorch parity gate (cosine ≥ 0.99) and batch-vs-single check.
- `embed_throughput --accelerator`. RTX 4060 + DirectML, fp32: 13–17 Chunks/s @256 tok, 2.5–6 @512 (CPU on this machine: 4.5–5.3 and 2.1–2.3).
- Diagnosis: all 48 opset-20 `Gelu` nodes fall back to CPU; GPU↔CPU copies take 83% of DirectML time. Rewriting `Gelu` (tanh form) into DirectML-supported ops: 1.6–2.4× faster (Python ORT 1.23 prototype, not in repo).
- GPU was 58–62% busy with other apps (e.g. NVIDIA Broadcast) during all measurements.
- CUDA (`Accelerator::Cuda`): ort ships Windows binaries for CUDA 13 only; needs CUDA 13 runtime + cuDNN 9 DLLs on PATH. Passes parity and batch checks.
- CUDA, preliminary (GPU ~65% busy with other apps): ~48 Chunks/s at both 256 and 512 tok (1M Chunks ≈ 6 h).
- `OnnxEmbedder::load_first` tries Accelerators in order; `skipped()` gives each one's failure reason.
- `sbm --accelerator <name>` (GPU falls back to CPU); default CUDA → DirectML → CPU on Windows, CPU elsewhere; prints the one in use and why others were skipped.
- `sbm` smoke-tested on Windows (9 files, CUDA): index 4.8 s; fallback without CUDA DLLs lands on DirectML.

## In progress
- Engine (`crates/engine`): long-running indexing + search for the desktop app.
  - Done: background worker, `add_indexed_folder`, `search`, `status` (files, pending, last error), `pause`/`resume`, Live Updates polled every 2 s, `wait_until_idle`.
  - Done: `Index::open` waits instead of failing while another connection writes.
  - Next: content progress ("files since … are ready"), failed files, remove folder/Exclusions, accelerator/model status; CLI on the Engine.

## Next
- Clean GPU benchmark (background apps closed); deferred by the user.
- M2: end-to-end Benchmark run through `sbm` (hybrid vs semantic-only), with the M1 benchmark work.

## Next (M1)
- Finish CPU benchmark (512 tok, batch 32) and compare int8/q4 models.
- File-level Recall still saturated: needs vaguer Queries or a much larger corpus (thousands of files).
- More non-abstract PDF Targets.
- Switch `retrieval_benchmark` from pdftotext to `PdfExtractor` and compare.

## Deferred until development moves to Windows (RTX 4060 8 GB)
- Full-disk Initial Indexing time estimate.
