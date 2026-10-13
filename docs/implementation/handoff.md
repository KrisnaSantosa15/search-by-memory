# Handoff

Snapshot for the next session or agent (2026-10-13, after commit `039a479`). Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md); terms in [`CONTEXT.md`](../../CONTEXT.md).

## State
- **Development moved to Windows** (RTX 4060 8 GB). The workspace builds and all tests pass on Windows; Linux paths are unchanged.
- **M1 Technical Validation**: done except deferred benchmarks: a clean GPU throughput run, and CPU int8/q4 models.
- **M2 Name + Text Search MVP**: Name Index, extractors, chunking, Content Index, hybrid search, and the `sbm` CLI are done and smoke-tested with the real model (on Linux).
- **GPU embedding**: `Accelerator` (`cpu`, `cuda`, `directml`, `directml:<n>`) with CPU fallback; CUDA and DirectML both pass the PyTorch parity gate. `sbm` tries CUDA → DirectML → CPU (`--accelerator <name>` overrides; a named GPU falls back to CPU) and prints the one in use plus why others were skipped.
- **In progress**: clean GPU benchmark (user closes GPU-using apps first).
- **Volume-aware file catalog**: done (volume ID + relative path; Disconnected Drives flagged).
- **M4 indexing core**: done (saved Indexed Folders and Exclusions, `index_jobs` queue: newest first, retries, resume).
- **Next M2 item**: end-to-end Benchmark run through `sbm` (hybrid vs semantic-only); deferred by the user with the other benchmarks.
- **Next M4 items**: USN Journal `ChangeSource` and Live Updates (ahead of Initial Indexing jobs), throttling, Online-only Files.
- Tests: `cargo test --workspace` 127 passed, 1 ignored (i8 recall, deliberate); `cd tools && uv run pytest` 13 passed.

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits: `Location`, `Embedding`, `Section`, `EmbeddingProvider`, `VectorStore`, `ContentExtractor`; `AppDirs` (per-platform data and cache directories); `Volumes` / `SystemVolumes` (volume identity adapter). |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder::load` / `load_on` / `load_first`, `skipped()`, `Accelerator::default_order`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`: f16/i8, `save`, memory-mapped `open_view`). |
| `crates/extract` | `TextExtractor`, `PdfExtractor` (pdf-extract), `DocxExtractor` (zip + quick-xml); `extractor_for(path)`; `chunk_sections` (sliding window, overlap). |
| `crates/index` | SQLite (rusqlite, bundled FTS5): file catalog + Name Index (`scan`, `search_names`, `Exclusion`), Content Index (`index_content`, `remove_unreferenced_content`, `rebuild_vectors`), hybrid `search` (File/Passage view, `MatchReason`). Paths are normalized (`C:/a`, `C:\a`, `\\?\C:\a` are one file), then stored as volume ID + relative path. `Index::with_volumes` swaps the adapter (tests use a fake). |
| `crates/cli` | `sbm` binary and the tested `index_folder` / `search` / `printable` functions. |
| `crates/bench` | Dev-only measurements: `embed_throughput [--accelerator <name>]`, `retrieval_benchmark`, Benchmark parser, File-view metrics, Location-miss report. |
| `benchmark/public/` | Public Corpus `manifest.toml` (74 files: URL, sha256, license) and `queries.toml` (58 labelled Queries). |
| `tools/` | Python dev tooling (uv): parity fixtures, `fetch_public_corpus.py` (incl. wikitext → plain text). Never shipped. |

## How search works (M2)
- `scan` walks an Indexed Folder (Default Exclusions, user folder/`*` patterns), mark-and-sweep against SQLite, and reports added / changed (size or mtime) / removed files.
- `index_content` hashes a file (BLAKE3, streamed); known content is reused (Copies, moves; ADR 0004). New content is extracted, chunked (1,000 chars, 150 overlap), embedded in batches of 8, stored in `chunks` (text + embedding BLOB) and `chunk_words` (FTS5), and added to USearch with the Chunk id as vector id.
- `search` fuses semantic (USearch) and keyword (FTS5) Chunk lists by Reciprocal Rank Fusion (k = 60); File view groups by content hash (Copies) and fuses with Name Matches; an exact filename match goes on top.
- Indexed Folders and Exclusions are saved (`add_indexed_folder`, `add_exclusion`). `scan_all` scans connected Indexed Folders and queues an `index_jobs` row per added/changed file (priority = mtime).
- `run_jobs(embedder, store, limit)` takes the newest pending job on a connected drive, commits each file on its own, retries a failing file 3 times then marks it failed (`failed_files`). Once no runnable job is left it runs `remove_unreferenced_content`, so moved files reuse content before it is collected.
- Schema: `MIGRATIONS` in `crates/index/src/lib.rs`, tracked by `PRAGMA user_version`; append only, never edit. A newer schema is refused.
- Files are keyed by `(volume_id, relative_path)`; the `volumes` table keeps each volume's last root, so files on a Disconnected Drive show their last-known path with `connected: false`.
- Not done yet: index_jobs / resume, Live Updates, throttling (M4); persisted Indexed Folders (the CLI passes them each run). Passage view intentionally omits name-only files.

## Directories (`AppDirs`)
| | Windows | Linux |
|---|---|---|
| Data (`SBM_DATA_DIR`) | `%LOCALAPPDATA%\search-by-memory` | `~/.local/share/search-by-memory` |
| Cache | `%LOCALAPPDATA%\search-by-memory\cache` | `~/.cache/search-by-memory` |
| Model (`SBM_MODEL_DIR`) | `<cache>\models\embeddinggemma-2-onnx` | same under cache |
| Public Corpus (`SBM_PUBLIC_CORPUS`) | `<cache>\public-corpus` | same under cache |

## Run (Windows, Git Bash)
- Rust 1.99 (rustup, MSVC) with VS Build Tools: `export PATH="$HOME/.cargo/bin:$PATH"`.
- CUDA runtime DLLs: `export PATH="$(cygpath -u "$LOCALAPPDATA")/nvidia-cuda13/bin:$PATH"`. Use `cygpath`: a raw `C:\...` entry breaks at the colon. Without it, CUDA silently falls back to CPU and the CUDA tests fail.
- Tests: `cargo test --workspace`; Python: `cd tools && uv run pytest`.
- CLI: `cargo build --release -p sbm-cli`, then `target/release/sbm index [<folder>...]` (remembers folders, rescans all, resumes) and `target/release/sbm search [--passages] <query>...`, each with optional `--accelerator <name>`; also `sbm folders`, `sbm remove <folder>`, `sbm exclude <folder> | --name <pattern>`. 256 dims, f16 vectors. Smoke-tested on Windows (CUDA); indexed folders print as `\?\D:\…` (canonicalize), cosmetic.
- Throughput: `target/release/embed_throughput --accelerator cpu|cuda|directml` (not while tests run; close GPU-using apps).
- Retrieval Benchmark: `cargo run --release -p sbm-bench --bin retrieval_benchmark` (still uses pdftotext, which is not installed on Windows yet; a fresh run takes 40+ min on CPU).
- Public Corpus: `cd tools && uv run fetch_public_corpus.py` (not yet fetched on Windows).
- Parity fixtures: `cd tools && uv run make_parity_fixtures.py`.

## Model
- `onnx-community/embeddinggemma-2-ONNX` (Apache-2.0), downloaded on Windows to the model directory above.
- Download: `uvx --from huggingface_hub hf download onnx-community/embeddinggemma-2-ONNX tokenizer.json tokenizer_config.json config.json onnx/model.onnx onnx/model.onnx_data --local-dir <dir>`.
- Model-dependent tests print `SKIPPED` and pass when the model is missing.

## GPU
- `ort` 2.0.0-rc.13 bundles ONNX Runtime 1.28. Its prebuilt Windows binaries support **CUDA 13 only** (the installed CUDA 12.4 toolkit is unused). Driver 616.92 supports CUDA 13.
- CUDA runtime in `%LOCALAPPDATA%\nvidia-cuda13\bin` (1.4 GB): cudart + cuBLAS (CUDA 13.4.2 redist), cuFFT, cuDNN 9.27.0 for CUDA 13; NVIDIA redist zips, sha256-verified; zips and extracted folders kept alongside. End users will need these DLLs too (packaging decision open).
- `ort`'s `copy-dylibs` symlinks DLLs next to the test binaries; without Windows Developer Mode, symlinks fail and it copies to `target/<profile>` only. Provider DLLs were copied by hand into `target/{release,debug}/deps`. Repeat after `cargo clean` or an `ort` update, or enable Developer Mode.
- DirectML: all 48 opset-20 `Gelu` nodes fall back to CPU; GPU↔CPU copies take 83% of its time. Rewriting `Gelu` into supported ops gave 1.6–2.4× (Python prototype only). The user chose CUDA over fixing this.

## Measurements so far
| Chunks/s (fp32, 256 dims) | 256 tok | 512 tok |
|---|---|---|
| CPU (Windows desktop) | 4.5–5.3 | 2.1–2.3 |
| DirectML, RTX 4060 | 13–17 | 2.5–6 |
| CUDA, RTX 4060 (preliminary) | 48–52 | 47–48 |
- GPU was 58–65% busy with other apps (NVIDIA Broadcast, Brave, Steam) during these runs. CUDA is flat across Chunk lengths, likely from that contention. Re-measure clean.
- CUDA ≈ 6 h per 1M Chunks (CPU ≈ 120 h).
- Linux laptop (i7-1255U): ~1 Chunk/s on the real corpus; `sbm index` on 15 code files 194 s, 2.0 GB peak; `sbm search` 2.7 s cold (model load).
- Retrieval Benchmark (semantic only, 43 of 58 Queries scorable): Recall@5 = 1.0; MRR 0.93–0.95; Location Accuracy 0.67 @256, 0.56 @512, 0.60 @768. f16 matches exact; i8 loses 0.02–0.07.

## Known gaps and caveats
- pdf-extract (and lopdf beneath it) returns no text for the scanned UU 27/2022 PDF with an invisible OCR layer; pdftotext reads it. Two Benchmark Queries (id-001, id-002) depend on it.
- pdf-extract panics on many corrupt PDFs; `PdfExtractor` catches the panic (the panic message still prints to stderr).
- Text and DOCX extraction stop at 50 MB decompressed; PDFs have no size cap yet.
- Public Corpus labels: many arXiv Targets are page 1 (easy); UU 36/2009 and UU 13/2003 are superseded texts; licenses to spot-check: Tom Lehrer recordings (PD claim), arXiv CC BY (verified on abs pages only).
- Wikipedia files are fetched at a pinned revid as wikitext and converted with mwparserfromhell; changing the converter changes `sha256`: recompute and relabel affected line Targets.
- UNC (`\\server\share`) paths are not normalized; only drive-letter verbatim paths are. Network shares have no volume GUID: their root is their ID (untested; no share on this machine).
- Unix `SystemVolumes` (mount point by device ID) compiles only on Unix and has not been run since the Windows move; an empty mount folder left behind by an unmounted drive counts as connected.

## Environment gotchas
- Windows: `core.autocrlf=true`. `.gitattributes` marks PDF/DOCX/PNG/ONNX binary; add new binary fixture types there, or CRLF conversion corrupts them.
- usearch 2.26.2 + MSVC: `.cargo/config.toml` sets `CXXFLAGS_x86_64_pc_windows_msvc=/DNK_DYNAMIC=` so NumKong links statically (otherwise unresolved `__imp_nk_*`).
- In PowerShell, `Select-String` is case-insensitive: `FAILED` also matches `0 failed`. Use `-CaseSensitive` or Git Bash `grep`.
- `ort` uses `tls-rustls`; `tokenizers` default features off; `directml` and `cuda` features enabled for the Windows target only.
- New dependencies are pinned (`=x.y.z`) to releases at least two weeks old (applies to downloaded runtimes too).
- `gh` points to a different host; use `git ls-remote` for GitHub SHAs. Plain `curl` is blocked by a hook; use Python `urllib`.
- Untracked `tools/**/__pycache__/*.pyc` files appear after pytest; do not commit them.

## Open questions for the user
1. Shipping CUDA: bundle the ~1.4 GB runtime DLLs, download on first run, or require a user install.
2. Review the 58 drafted Queries in `benchmark/public/queries.toml`; license spot-checks above.
3. f16 vs i8 and 256 vs 512 dims: confirm with the deferred Benchmark runs.
4. PDF backend: keep pdf-extract, or move to pdfium/poppler for scanned PDFs.
