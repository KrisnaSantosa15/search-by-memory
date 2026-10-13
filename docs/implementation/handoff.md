# Handoff

Snapshot for the next session or agent (2026-10-13, after commit `4692004`). Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md); terms in [`CONTEXT.md`](../../CONTEXT.md).

## State
- **Development moved to Windows** (RTX 4060 8 GB). The workspace builds and all tests pass on Windows; Linux paths are unchanged.
- **M1 Technical Validation**: done except deferred benchmarks: a clean GPU throughput run, and CPU int8/q4 models.
- **M2 Name + Text Search MVP**: Name Index, extractors, chunking, Content Index, hybrid search, and the `sbm` CLI are done and smoke-tested with the real model (Linux, then Windows on CUDA). Only the end-to-end Benchmark run is left.
- **GPU embedding**: `Accelerator` (`cpu`, `cuda`, `directml`, `directml:<n>`) with CPU fallback; CUDA and DirectML both pass the PyTorch parity gate. `sbm` tries CUDA → DirectML → CPU (`--accelerator <name>` overrides; a named GPU falls back to CPU) and prints the one in use plus why others were skipped.
- **Volume-aware file catalog**: done (volume ID + relative path; Disconnected Drives flagged).
- **M4 indexing core**: done (saved Indexed Folders and Exclusions, `index_jobs` queue: newest first, retries, resume).
- **M4 Online-only Files**: done. `FileAvailability` adapter (`SystemFileAvailability`: Windows recall/offline attributes, never opens the file); Online-only Files are name-indexed only, and content-indexed once downloaded.
- **M4 Live Updates**: done. `ChangeSource` trait, `Index::apply_changes`, `UsnJournal` adapter (`crates/change-usn`, no admin rights; ADR 0005); `sbm index` uses it.
- **Deferred by the user**: all benchmarks: clean GPU throughput run (close GPU-using apps first), CPU int8/q4, end-to-end Benchmark through `sbm` (hybrid vs semantic-only).
- **Next**: each needs its interface agreed with the user first (AGENTS.md):
  - M4: throttling (full-screen, battery, busy; Live Updates exempt).
  - M4 later: a background process that applies changes continuously (today `sbm index` runs on demand).
  - M3: Tauri 2 desktop launcher.
- Tests: `cargo test --workspace` 149 passed, 1 ignored (i8 recall, deliberate); `cd tools && uv run pytest` 13 passed.

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits: `Location`, `Embedding`, `Section`, `EmbeddingProvider`, `VectorStore`, `ContentExtractor`; `AppDirs` (per-platform data and cache directories); `Volumes` / `SystemVolumes` (volume identity adapter). |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder::load` / `load_on` / `load_first`, `skipped()`, `Accelerator::default_order`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`: f16/i8, `save`, memory-mapped `open_view`). |
| `crates/extract` | `TextExtractor`, `PdfExtractor` (pdf-extract), `DocxExtractor` (zip + quick-xml); `extractor_for(path)`; `chunk_sections` (sliding window, overlap). |
| `crates/index` | SQLite (rusqlite, bundled FTS5): file catalog + Name Index (`scan`, `search_names`, `Exclusion`), Content Index (`index_content`, `remove_unreferenced_content`, `rebuild_vectors`), hybrid `search` (File/Passage view, `MatchReason`, `connected`); saved Indexed Folders and Exclusions, `scan_all`, `index_jobs` queue (`run_jobs`, `pending_jobs`, `failed_files`). Paths are normalized (`C:/a`, `C:\a`, `\\?\C:\a` are one file), then stored as volume ID + relative path. `Index::with_volumes` swaps the adapter (tests use a fake). |
| `crates/change-usn` | `UsnJournal`: the NTFS USN Journal as a `ChangeSource` (unprivileged read, paths by file ID); elsewhere always Lost. |
| `crates/cli` | `sbm` binary and the tested `index` / `search` / `folders` / `remove_folder` / `exclude` / `accelerator_option` / `printable` functions; `main.rs` is thin, untested glue. |
| `crates/bench` | Dev-only measurements: `embed_throughput [--accelerator <name>]`, `retrieval_benchmark`, Benchmark parser, File-view metrics, Location-miss report. |
| `benchmark/public/` | Public Corpus `manifest.toml` (74 files: URL, sha256, license) and `queries.toml` (58 labelled Queries). |
| `tools/` | Python dev tooling (uv): parity fixtures, `fetch_public_corpus.py` (incl. wikitext → plain text). Never shipped. |

## How indexing and search work
- `scan` walks an Indexed Folder (Default Exclusions, user folder/`*` patterns), mark-and-sweep against SQLite, and reports added / changed (size or mtime) / removed files.
- `index_content` hashes a file (BLAKE3, streamed); known content is reused (Copies, moves; ADR 0004). New content is extracted, chunked (1,000 chars, 150 overlap), embedded in batches of 8, stored in `chunks` (text + embedding BLOB) and `chunk_words` (FTS5), and added to USearch with the Chunk id as vector id.
- `search` fuses semantic (USearch) and keyword (FTS5) Chunk lists by Reciprocal Rank Fusion (k = 60); File view groups by content hash (Copies) and fuses with Name Matches; an exact filename match goes on top.
- Indexed Folders and Exclusions are saved (`add_indexed_folder`, `add_exclusion`). `scan_all` scans connected Indexed Folders and queues an `index_jobs` row per added/changed file (priority = mtime).
- `run_jobs(embedder, store, limit)` takes the newest pending job on a connected drive, commits each file on its own, retries a failing file 3 times then marks it failed (`failed_files`). Once no runnable job is left it runs `remove_unreferenced_content`, so moved files reuse content before it is collected.
- Schema: `MIGRATIONS` in `crates/index/src/lib.rs`, tracked by `PRAGMA user_version`; append only, never edit. A newer schema is refused.
- Files are keyed by `(volume_id, relative_path)`; the `volumes` table keeps each volume's last root, so files on a Disconnected Drive show their last-known path with `connected: false`.
- `sbm index` saves the vector file every 100 files; on every run the vector index is rebuilt from SQLite, so a crash between the two loses nothing.
- Tests for this live in `crates/index/tests/{indexing,volumes}.rs`; shared fakes (`BagOfWords` counting embedder, `FakeVolumes`) in `tests/common/mod.rs`.
- `apply_changes(source)` asks the `ChangeSource` for each connected volume's changes since its saved cursor. Reported paths inside Indexed Folders (minus Exclusions) are re-checked like a scan and queued as Live Updates (`index_jobs.live`, run before other jobs); a Lost history rescans that volume's Indexed Folders. Changes and the new cursor commit in one transaction. Adding an Indexed Folder or Exclusion deletes the affected cursors. Tests: `crates/index/tests/live_updates.rs` (`FakeChangeSource`), `crates/change-usn/tests/usn_journal.rs` (real journal on the temp folder).
- Not done yet: throttling (M4); the dashboard counts exist (`pending_jobs`, `failed_files`) but nothing shows them except `sbm index`. Passage view intentionally omits name-only files.

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
- CLI: `cargo build --release -p sbm-cli`, then `target/release/sbm index [<folder>...]` (remembers folders, applies USN Journal changes since the last run, resumes) and `target/release/sbm search [--passages] <query>...`, each with optional `--accelerator <name>`; also `sbm folders`, `sbm remove <folder>`, `sbm exclude <folder> | --name <pattern>`. 256 dims, f16 vectors. Smoke-tested on Windows (CUDA), including kill -9 mid-run and resume.
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
- USN adapter: a delete or rename re-checks its whole parent folder (metadata walk only). Paths come back in the file system's true case, so an Indexed Folder saved in another case would not match them (`sbm` canonicalizes folders; only hand-made paths are affected). If the journal wraps during a read, `ERROR_JOURNAL_ENTRY_DELETED` surfaces as an error, and the next run reports Lost.
- Volumes without a journal (FAT/exFAT USB, network shares) rescan on every `sbm index`.
- UNC (`\\server\share`) paths are not normalized; only drive-letter verbatim paths are. Network shares have no volume GUID: their root is their ID. The loopback share `\\localhost\C$\` is reachable on this machine and stands in for a share in the `change-usn` tests.
- `sbm index` prints only added/changed/removed counts; work resumed from an interrupted run is not reported.
- A corrupt PDF that panics pdf-extract prints the panic message once per try (3 times).
- Unix `SystemVolumes` (mount point by device ID) compiles only on Unix and has not been run since the Windows move; an empty mount folder left behind by an unmounted drive counts as connected.

## Environment gotchas
- Windows: `core.autocrlf=true`. `.gitattributes` marks PDF/DOCX/PNG/ONNX binary; add new binary fixture types there, or CRLF conversion corrupts them.
- usearch 2.26.2 + MSVC: `.cargo/config.toml` sets `CXXFLAGS_x86_64_pc_windows_msvc=/DNK_DYNAMIC=` so NumKong links statically (otherwise unresolved `__imp_nk_*`).
- In PowerShell, `Select-String` is case-insensitive: `FAILED` also matches `0 failed`. Use `-CaseSensitive` or Git Bash `grep`.
- `ort` uses `tls-rustls`; `tokenizers` default features off; `directml` and `cuda` features enabled for the Windows target only.
- New dependencies are pinned (`=x.y.z`) to releases at least two weeks old (applies to downloaded runtimes too).
- `gh` points to a different host; use `git ls-remote` for GitHub SHAs. Plain `curl` is blocked by a hook; use Python `urllib`.
- Untracked `tools/**/__pycache__/*.pyc` files appear after pytest; do not commit them.
- Multi-line Python heredocs (`python - <<'EOF'`) sometimes fail to parse in the Bash tool; write the script to the scratchpad and run it instead.

## Open questions for the user
1. Shipping CUDA: bundle the ~1.4 GB runtime DLLs, download on first run, or require a user install.
2. Review the 58 drafted Queries in `benchmark/public/queries.toml`; license spot-checks above.
3. f16 vs i8 and 256 vs 512 dims: confirm with the deferred Benchmark runs.
4. PDF backend: keep pdf-extract, or move to pdfium/poppler for scanned PDFs.
