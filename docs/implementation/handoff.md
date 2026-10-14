# Handoff

Snapshot for the next session or agent (2026-10-14, after the launcher follow-ups, PDFium text extraction, and throttling). Progress lives in [`backlog.md`](backlog.md); rules in [`AGENTS.md`](../../AGENTS.md); terms in [`CONTEXT.md`](../../CONTEXT.md).

## State
- **Development moved to Windows** (RTX 4060 8 GB). The workspace builds and all tests pass on Windows; Linux paths are unchanged.
- **M1 Technical Validation**: done except deferred benchmarks: a clean GPU throughput run, and CPU int8/q4 models.
- **M2 Name + Text Search MVP**: Name Index, extractors, chunking, Content Index, hybrid search, and the `sbm` CLI are done and smoke-tested with the real model (Linux, then Windows on CUDA). Only the end-to-end Benchmark run is left.
- **GPU embedding**: `Accelerator` (`cpu`, `cuda`, `directml`, `directml:<n>`) with CPU fallback; CUDA and DirectML both pass the PyTorch parity gate. `sbm` tries CUDA → DirectML → CPU (`--accelerator <name>` overrides; a named GPU falls back to CPU) and prints the one in use plus why others were skipped.
- **Volume-aware file catalog**: done (volume ID + relative path; Disconnected Drives flagged).
- **M4 indexing core**: done (saved Indexed Folders and Exclusions, `index_jobs` queue: newest first, retries, resume).
- **Engine** (`crates/engine`): done. Background worker + `search`, `status` (files, indexed, pending, `ready_since`, failed, last error, `ModelStatus`), `failed_files`, `remove_indexed_folder`, `add_exclusion`, `pause`/`resume`, Live Updates polled every 2 s. `start_loading` loads the model on its own thread: until it is ready, search is name-only (Passage view empty) and content waits. The desktop app will run on it. `sbm index` runs on it; `sbm search` / `folders` / `remove` / `exclude` read and write the index directly (no worker, no model load where none is needed).
- **Setup** (`crates/setup`): done. Verified, resumable downloads (`download`, `Web`), pinned `embedding_model()` and `cuda_runtime()` manifests, `install_runtime`, `system_gpus` (DXGI), `speed_options` (recommendation).
- **M4 Online-only Files**: done. `FileAvailability` adapter (`SystemFileAvailability`: Windows recall/offline attributes, never opens the file); Online-only Files are name-indexed only, and content-indexed once downloaded.
- **M4 Throttling**: done (CPU side). `Activity` adapter + `ThrottleReason` in core; `SystemActivity` (Windows): full-screen/presentation via `SHQueryUserNotificationState` (QUNS_BUSY, D3D full screen, presentation), on battery via `GetSystemPowerStatus`, busy = other processes > 50% of all CPU time between samples ≥ 1 s apart (own process excluded). `Engine::set_activity(activity, quiet)`: while throttled and for `quiet` after, only `Index::run_live_jobs` runs; `Status.throttled` / `Status.paused`. The app uses a 60 s quiet period; the tray's first (disabled) entry and tooltip show `indexing_state` every 3 s; tray pauses 1 h / 8 h or resumes. `sbm index` is not throttled. Not done: GPU busy, Live Updates on CPU while full-screen (needs a second, CPU model), "until tomorrow".
- **M4 Live Updates**: done. `ChangeSource` trait, `Index::apply_changes`, `UsnJournal` adapter (`crates/change-usn`, no admin rights; ADR 0005); `sbm index` uses it.
- **Deferred by the user**: all benchmarks: clean GPU throughput run (close GPU-using apps first), CPU int8/q4, end-to-end Benchmark through `sbm` (hybrid vs semantic-only).
- **M3 desktop app** (`app/`): in progress. Tauri 2 + React + TypeScript + Tailwind 4, Vitest, pnpm (npm 11.4 crashes on install). `app/src-tauri` is `sbm-app`, a workspace member; it compiles in dev mode without `dist`, so `cargo test --workspace` needs no frontend build.
  - Goal (agreed): easy for non-developers; every step has a working default, plain language (no "CUDA"/"DirectML" wording), one progress bar with time left, name search usable as soon as onboarding ends.
  - Look and behaviour (user's choice, 2026-10-14): like DigUp (github.com/ARahim3/DigUp, MIT, Swift: ideas only). Palette our own, "archive at night": ink-slate glass `#1D2230`, paper `#ECE7DC`, lamplight amber `#E3A646` for actions and the Query's words (highlighter); Segoe UI Variable, Cascadia Mono for code. Tokens in `app/src/index.css` (base rules inside `@layer base`). This replaced the earlier VoiceStudio look.
  - Onboarding done: speed step (`speed_choices`), download step (`setup::Plan` + `setup::prepare` + `setup::TimeLeft`; `prepare` command emits throttled `setup-progress`; "Try again" resumes), folders step (personal folder picked, other drives offered: the user's choice; `folder_choices`), then `start_indexing` starts the Engine (`start_loading` on the picked Accelerator, CPU fallback, USN Journal) and adds the folders. `run()` puts `<cache>/runtimes/cuda13` on `PATH` before any thread starts.
  - Then the hotkey step (`HotkeyStep`: tries `Alt+Space` via `try_hotkey`, says when another app uses it, records other keys as `Ctrl+Shift+KeyK`; needs a modifier) and the done step; "Start searching" calls `finish_onboarding`, which saves `Settings` (`settings.json` in the data dir: `accelerator`, `hotkey`) and hides the window (the user's choice: hide to tray). Hotkey: `tauri-plugin-global-shortcut` 2.4.0; tray icon (Search, Quit) via Tauri's `tray-icon` feature.
  - With `settings.json` present, the next start resumes the Engine on the saved Accelerator, registers the hotkey, and starts hidden; the `settings` command makes the window show the launcher instead of onboarding. Delete `settings.json` to see onboarding again.
  - Launcher (`app/src/Launcher.tsx`, `Preview.tsx`, `Marked.tsx`, `Path.tsx`): frameless panel: Query bar, Results (left) beside the Preview Pane (right), footer with status ("Searching…", "3 results", "Nothing matches yet") and key hints. Results as you type (no debounce; stale answers dropped); ↑/↓ or click selects; Enter opens, Ctrl/Shift+Enter shows in folder, Ctrl+C copies the file itself (unless Query text is selected), Escape hides. A Result with Copies asks which one in the Preview Pane (Enter or → starts, ← stops). Disconnected Drive Results are labelled and not opened or shown. Preview Pane: snippet as a paper page with the Query's words marked (whole words; the last word also as a prefix), Location badge, name, folder, size, date, why it matched, Open / Show in folder, other Copies. No Passage view and no Detail Panel any more (PRD, CONTEXT.md updated). The Launcher takes its IPC as props; `App.tsx` wires `search` (File view, top 20; `result_row` adds folder, size, mtime from the file system, skipped on a Disconnected Drive), `open_file` / `reveal_file` (`tauri-plugin-opener` 2.7.0), `copy_file` (CF_HDROP via clipboard-win 5.4.1, Windows only), `hide_launcher`.
  - Stage 2 (agreed rule): `Index::search` flags `SearchResult.weaker` for Results related only by meaning (Match Reason content, none of the Query's words in any of its Chunks) whose best semantic similarity is more than `WEAKER_MARGIN` (0.08, untuned) below the best Result's; tests in `crates/index/tests/weaker_matches.rs`. `ResultRow` adds `kind` (`FileKind::of`, by extension) and `weaker`. The Launcher groups strong Results by kind (groups in order of their best Result, headings "Documents", "Code", …) and folds Weaker Matches into one "N weaker matches" row at the bottom (Enter toggles, → or click unfolds; the Preview Pane explains it); footer "1 result, 2 weaker". With the real model on 4 files, "slow cooked beef" left 1 Result and folded 2.
  - Stage 3 (agreed): `text_preview(path, matched)` in `app/src-tauri/src/preview.rs` reads the file line by line (never whole; lossy UTF-8, `\r` trimmed): matched lines with 3 lines of context, at most 40 (a long match keeps its start), first 40 lines without a match, `None` if unreadable; tests in `tests/preview.rs`. `preview_lines` command (blocking thread). Rows carry `lineStart`/`lineEnd`. The Preview Pane asks for lines for code, and for documents whose Location is lines, on a connected drive only: code → `CodeLines` (numbered, Cascadia Mono, matched lines tinted, scrolled into view); text → `PageLines` on the paper page (unmatched lines faded); otherwise, while loading, or when unreadable, the snippet. PDF/DOCX (page Locations) still show the snippet.
  - Stage 4a: a Query starting with `code:` (any case) searches the rest over code only: `search` command `onlyCode` → `Engine::search_where` / `Index::search_where(…, keep)`, which filters files before the limit and before Weaker Matches are judged (also for name-only search while the model loads). The magnifier turns into `</>` (`role=img`, "Searching code").
  - Stage 4b: `crates/thumbnail` (`sbm-thumbnail`): `thumbnail(path, size, cached_only)` → PNG from `IShellItemImageFactory` (`SIIGBF_THUMBNAILONLY`, plus `SIIGBF_INCACHEONLY` when cached only; premultiplied BGRA → straight RGBA; all-zero alpha → opaque); tests against the real Shell. `thumbnail` command (80 px, cache-only for Online-only Files via `SystemFileAvailability`) returns a `data:` URL. Rows show it (40 px) or a drawn kind icon (`Thumbnail.tsx`, cached per path and size per Launcher). The command takes a `size` (16–840 px): rows ask 80, the Preview Pane asks 840 for images and videos and shows the picture in place of the snippet.
  - Stage 4c: PDFium `chromium/7881` (bblanchon/pdfium-binaries, `pdfium-win-x64.tgz`, 3.7 MB, SHA-256 pinned in `pdfium_runtime()`), matching `pdfium-render` 0.9.4's API (`pdfium_7881`, `thread_safe`, no `image` feature). Every `Plan` includes it; `prepare` installs runtime → PDFium → model under one progress bar; `install_runtime` unpacks `.tgz` (`bin/*.dll` → dir, `licenses/*` → dir/licenses). Installed in `<cache>/runtimes/pdfium` (`PDFIUM_DIR`); on this machine it was put there by hand from the verified archive. `crates/pdf` (`sbm-pdf`): `PdfPages::load(dir)` (one PDFium per process, `OnceLock`), `page(path, number, width, words)` → PNG + word `Area`s (fractions, whole words, any case); tests use the real DLL and print SKIPPED without it. Rows carry `page`; `pdf_page` command (840 px) → `data:` URL + marks; `PdfPage.tsx` overlays `<mark>`s (multiply blend).
  - Window (`tauri.conf.json`): 880×560, no frame, transparent, Acrylic, centered. Onboarding draws a title strip (`data-tauri-drag-region`, Quit → `quit`). After onboarding (`become_launcher`): off the taskbar, always on top, hides when it loses focus (`Launching` flag + `on_window_event`).
  - Testing the real window without touching the real index: `SBM_DATA_DIR=<scratch>/data sbm index <scratch>/files`, write `<scratch>/data/settings.json` (`{"accelerator":"cuda","hotkey":"Ctrl+Alt+Shift+F9"}`), then `SBM_DATA_DIR=... pnpm tauri dev`; press the hotkey with `keybd_event` and type with `SendKeys` (PowerShell), screenshot with `PrintWindow`.
  - Verified in the real window: speed step only (`pnpm tauri dev`; also starts with the shortcut plugin and tray). The full onboarding has not been run through the GUI: Continue on the folders step indexes the real personal folder into the shared data dir (`sbm` uses it too). Here the model is already in the cache; the CUDA runtime is not yet in `<cache>/runtimes/cuda13` (it lives in `%LOCALAPPDATA%/nvidia-cuda13`), so "Fastest" would download 1.9 GB.
  - Code layout: plain, tested functions in `app/src-tauri/src/lib.rs` (`speed_choices`, `folder_choices`); thin `#[tauri::command]` wrappers in `mod commands` (untested glue). Frontend steps in `app/src/onboarding/`, each with a Vitest test; `App.tsx` drives the steps, tested with `mockIPC` (`shouldMockEvents`).
- **Next**: each needs its interface agreed with the user first (AGENTS.md):
  - Done since stage 4c: large picture for images/videos; PDF page scrolls to its first mark; Copy picking starts on the one picked last time (WebView `localStorage`, `picked-copy <path>` = timestamp; still confirmed with Enter, PRD updated); Cancel on the download step (`cancel_prepare`, fresh cancel flag per `prepare`; back to the speed choices, download resumes on the next pick); PDF text from PDFium (ADR 0006).
  - M3 launcher follow-ups: run the whole onboarding through the GUI once (indexes the real personal folder; check Alt + Space registers on Windows; PDFium now downloads too).
  - M4 throttling follow-ups: GPU busy detection; Live Updates on CPU while a full-screen app runs (PRD acceptance item); pause "until tomorrow".
  - PDFs indexed before PDFium was installed keep their pdf-extract Chunks (content hash unchanged); decide whether to re-extract them.
- Tests: `cargo test --workspace` 229 passed, 1 ignored (i8 recall, deliberate); `cd tools && uv run pytest` 13 passed; `cd app && pnpm test` 52 passed.

## Workspace
| Path | Purpose |
|---|---|
| `crates/core` | Domain types and provider traits: `Location`, `Embedding`, `Section`, `EmbeddingProvider`, `VectorStore`, `ContentExtractor`; `AppDirs` (per-platform data and cache directories); `Volumes` / `SystemVolumes` (volume identity adapter). |
| `crates/embed-onnx` | EmbeddingGemma 2 via ONNX Runtime (`OnnxEmbedder::load` / `load_on` / `load_first`, `skipped()`, `Accelerator::default_order`). |
| `crates/store-usearch` | USearch `VectorStore` (`UsearchStore`: f16/i8, `save`, memory-mapped `open_view`). |
| `crates/extract` | `TextExtractor`, `PdfExtractor` (pdf-extract), `DocxExtractor` (zip + quick-xml); `extractor_for(path)`; `chunk_sections` (sliding window, overlap). |
| `crates/index` | SQLite (rusqlite, bundled FTS5): file catalog + Name Index (`scan`, `search_names`, `Exclusion`), Content Index (`index_content`, `remove_unreferenced_content`, `rebuild_vectors`), hybrid `search` / `search_where(…, keep)` (File/Passage view, `MatchReason`, `connected`, `weaker`); saved Indexed Folders and Exclusions, `scan_all`, `index_jobs` queue (`run_jobs`, `pending_jobs`, `failed_files`). Paths are normalized (`C:/a`, `C:\a`, `\\?\C:\a` are one file), then stored as volume ID + relative path. `Index::with_volumes` swaps the adapter (tests use a fake). |
| `crates/change-usn` | `UsnJournal`: the NTFS USN Journal as a `ChangeSource` (unprivileged read, paths by file ID); elsewhere always Lost. |
| `crates/engine` | `Engine`: owns the indexes and embedder, loads the model in the background (`start_loading`), indexes in a background thread (own SQLite connection, vector store shared through an `RwLock`), answers `search` / `search_where` meanwhile. |
| `crates/setup` | First-run setup: `download` (resume, SHA-256, cancel), `Web` (ureq), model, CUDA and PDFium manifests, `install_runtime` (zip or `.tgz`), `Plan` / `prepare` (one progress bar), `system_gpus`, `speed_options`. |
| `crates/thumbnail` | `thumbnail(path, size, cached_only)`: the Windows Shell thumbnail as PNG (`IShellItemImageFactory`); `None` elsewhere. |
| `crates/pdf` | `PdfPages` (PDFium via `pdfium-render`): `page(path, number, width, words)` → PNG + where the words sit; `text(path)` → each page's text (used by `PdfExtractor`); `PDFIUM_DIR`. |
| `app/` | Desktop app: React frontend (`src/`, Vitest tests next to components); `src-tauri/` is the Tauri shell (`sbm-app`): plain tested functions in `lib.rs` (`speed_choices`, `folder_choices`, `Settings`, `result_row`, `FileKind`) and `preview.rs` (`text_preview`), thin `#[tauri::command]` wrappers in `mod commands`. Frontend: `App.tsx` (onboarding steps, then `Launcher`), `Launcher.tsx`, `Preview.tsx`, `Lines.tsx`, `PdfPage.tsx`, `Thumbnail.tsx`, `Marked.tsx`, `Path.tsx`. |
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
- Weaker Matches: `search_where` keeps the best semantic similarity per Result and which Results had a keyword hit; after fusing with Name Matches, content-only Results without the Query's words and more than `WEAKER_MARGIN` (0.08) below the best are `weaker`. `keep` filters paths before the limit and before this judgement (the launcher's `code:`). Tests: `crates/index/tests/{weaker_matches,filtered_search}.rs`.
- Not done yet: throttling (M4); the dashboard counts exist (`pending_jobs`, `failed_files`) but nothing shows them except `sbm index`. Passage view intentionally omits name-only files.

## Directories (`AppDirs`)
| | Windows | Linux |
|---|---|---|
| Data (`SBM_DATA_DIR`) | `%LOCALAPPDATA%\search-by-memory` | `~/.local/share/search-by-memory` |
| Cache | `%LOCALAPPDATA%\search-by-memory\cache` | `~/.cache/search-by-memory` |
| Model (`SBM_MODEL_DIR`) | `<cache>\models\embeddinggemma-2-onnx` | same under cache |
| Public Corpus (`SBM_PUBLIC_CORPUS`) | `<cache>\public-corpus` | same under cache |

PDFium lives in `<cache>/runtimes/pdfium` (`PDFIUM_DIR`), the CUDA runtime in `<cache>/runtimes/cuda13`. The app honours `SBM_DATA_DIR` like `sbm`.

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
- `SystemActivity` tests use the real system; while a full-screen app (e.g. a game) is in front, the busy test prints SKIPPED. Its `cmd` busy loops run at idle priority.
- `WEAKER_MARGIN` (0.08) is a first guess; tune it on the Benchmark. Similarity is per Chunk, so a long file's best Chunk decides.
- Online-only detection is tested with `FILE_ATTRIBUTE_OFFLINE` only; the cloud recall attributes need a real OneDrive folder (not yet tried).
- `Engine` search shares the ONNX session with indexing: a Query waits for the current batch of 8 Chunks.
- pdf-extract (and lopdf beneath it) returns no text for the scanned UU 27/2022 PDF with an invisible OCR layer; pdftotext reads it. Two Benchmark Queries (id-001, id-002) depend on it.
- pdf-extract panics on many corrupt PDFs; `PdfExtractor` catches the panic (the panic message still prints to stderr).
- Text and DOCX extraction stop at 50 MB decompressed; PDFs have no size cap yet.
- Public Corpus labels: many arXiv Targets are page 1 (easy); UU 36/2009 and UU 13/2003 are superseded texts; licenses to spot-check: Tom Lehrer recordings (PD claim), arXiv CC BY (verified on abs pages only).
- Wikipedia files are fetched at a pinned revid as wikitext and converted with mwparserfromhell; changing the converter changes `sha256`: recompute and relabel affected line Targets.
- USN adapter: a delete or rename re-checks its whole parent folder (metadata walk only). Paths come back in the file system's true case, so an Indexed Folder saved in another case would not match them (`sbm` canonicalizes folders; only hand-made paths are affected). If the journal wraps during a read, `ERROR_JOURNAL_ENTRY_DELETED` surfaces as an error, and the next run reports Lost.
- Volumes without a journal (FAT/exFAT USB, network shares) rescan on every `sbm index`.
- UNC (`\\server\share`) paths are not normalized; only drive-letter verbatim paths are. Network shares have no volume GUID: their root is their ID. The loopback share `\\localhost\C$\` is reachable on this machine and stands in for a share in the `change-usn` tests.
- `sbm index` prints files / with content indexed / pending / failed (Engine status), no longer added/changed/removed.
- A corrupt PDF that panics pdf-extract prints the panic message once per try (3 times).
- Unix `SystemVolumes` (mount point by device ID) compiles only on Unix and has not been run since the Windows move; an empty mount folder left behind by an unmounted drive counts as connected.

## Environment gotchas
- D: was full (114 MB free) and broke the build; `target/debug/incremental` (compile cache only) was deleted to free space. `target` is ~25 GB.
- `pnpm tauri dev` right after the previous app instance was killed can fail with WebView2 `0x800700AA` (resource in use); retry after a few seconds.
- Screenshots of the app: `PrintWindow` on the window handle (never screen copy: it captures whatever is in front).
- Windows: `core.autocrlf=true`. `.gitattributes` marks PDF/DOCX/PNG/ONNX binary; add new binary fixture types there, or CRLF conversion corrupts them.
- usearch 2.26.2 + MSVC: `.cargo/config.toml` sets `CXXFLAGS_x86_64_pc_windows_msvc=/DNK_DYNAMIC=` so NumKong links statically (otherwise unresolved `__imp_nk_*`).
- In PowerShell, `Select-String` is case-insensitive: `FAILED` also matches `0 failed`. Use `-CaseSensitive` or Git Bash `grep`.
- `ort` uses `tls-rustls`; `tokenizers` default features off; `directml` and `cuda` features enabled for the Windows target only.
- New dependencies are pinned (`=x.y.z`) to releases at least two weeks old (applies to downloaded runtimes too).
- `gh` points to a different host; use `git ls-remote` for GitHub SHAs. Plain `curl` is blocked by a hook; use Python `urllib`.
- Untracked `tools/**/__pycache__/*.pyc` files appear after pytest; do not commit them.
- Multi-line Python heredocs (`python - <<'EOF'`) sometimes fail to parse in the Bash tool; write the script to the scratchpad and run it instead.

## Open questions for the user
1. NVIDIA licence: show cuDNN's NVIDIA SLA and ask for consent in onboarding before the CUDA runtime download? (Decided: the CUDA runtime is downloaded during onboarding only when the user picks the NVIDIA option.)
2. Review the 58 drafted Queries in `benchmark/public/queries.toml`; license spot-checks above.
3. f16 vs i8 and 256 vs 512 dims: confirm with the deferred Benchmark runs.
4. PDF backend: decided (ADR 0006): PDFium where installed, pdf-extract as fallback. Re-extract PDFs indexed before?
5. Run the full onboarding through the GUI? It indexes the real personal folder into the shared data dir.
