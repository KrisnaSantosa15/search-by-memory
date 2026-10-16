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
- Known gap: pdf-extract/lopdf return no text for the OCR layer of UU 27/2022 (pdftotext reads it); closed by PDFium (ADR 0006).

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

### Setup for onboarding (`crates/setup`)
- `download`: progress over all files, resumes from `.part`, SHA-256 verified, skips finished files, cancel keeps progress, oversize rejected.
- `Web` adapter (ureq, rustls): HTTP ranges, also from servers that ignore them.
- `embedding_model()` pinned to HF revision `daa72c5` (1.1 GB); `cuda_runtime()`: 4 NVIDIA redist zips (1.9 GB, 1.4 GB unpacked).
- `install_runtime`: unpacks `bin` DLLs, deletes archives, not downloaded again.
- `system_gpus` (DXGI, skips software renderers); `speed_options`: NVIDIA → CUDA, other ≥ 2 GB GPU → DirectML, else CPU recommended.

### Engine (`crates/engine`)
- Long-running indexing + search for the desktop app: background worker, Live Updates polled every 2 s, `pause`/`resume`, `wait_until_idle`.
- `Index::open` waits instead of failing while another connection writes.
- `add_indexed_folder`, `remove_indexed_folder`, `add_exclusion`; `failed_files` with reasons.
- `status`: files, indexed, pending, `ready_since` (newest pending file's mtime; `Index::content_progress`), failed, last error, `ModelStatus`.
- `start_loading`: model loads on its own thread; name-only search and no content work until then; Loading / Ready { device } / Failed.
- `sbm index` runs on the Engine and prints files / indexed / pending / failed; smoke-tested on Windows (CUDA, USN pickup). `search`, `folders`, `remove`, `exclude` stay direct.

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

### M4 — Throttling (`crates/core`, `crates/index`, `crates/engine`, `app/`)
- `Activity` adapter + `ThrottleReason` (FullScreen, Battery, Busy); `Index::run_live_jobs`.
- `Engine::set_activity(activity, quiet)`: while throttled and for the quiet period after, only Live Updates run; `Status.throttled`, `Status.paused`.
- `SystemActivity` (Windows): full-screen/presentation (`SHQueryUserNotificationState`), on battery, other processes > 50% CPU (own process excluded; real-system tests).
- App: quiet period 60 s; tray shows what indexing is doing and why it waits; tray pauses for 1 h / 8 h or resumes.
- Timed pauses end in the Engine (`pause_until`, wall clock, survives sleep); `Status.paused_until`; tray "until tomorrow" = next 06:00 local (jiff); tray and Settings say "paused until 14:30" / "until tomorrow 06:00".
- Not yet: GPU busy; Live Updates still use the GPU while full-screen; CLI not throttled (by design).

### M4 — Settings and Status window (`crates/index`, `crates/engine`, `app/`)
- Tray "Settings…" opens a separate window (`?view=settings`); only the launcher hides on blur.
- Status tab (every 2 s): files found, content % and how far back it is ready, size on disk (`Status.size_on_disk`), why indexing waits, Pause/Resume, failed files with reasons.
- Folders tab: Indexed Folders (Disconnected Drives labelled), add with the folder picker (`tauri-plugin-dialog` 2.8.0), remove after confirming.
- Exclusions: defaults marked; remove the user's, turn off defaults (`turned_off_defaults` table), Restore defaults; exclude typed names or a picked folder. `Index`/`Engine::remove_exclusion`, `restore_default_exclusions`.
- Checked in the real window on a scratch index.
- Failed files say why in plain words (`Unreadable`: password, damaged, too large, in use, no access, gone, other; stored per job, migration 12); raw message under Details.

### MVP gaps
- PDF text beyond 50 MB refused as too large (`PdfExtractor::with_limit`; `default()` = 50 MB).
- Purge (FR-06): `Engine::purge` erases database, journal files, vectors (waits for a loading model); Settings "Delete index…" confirms, forgets `settings.json`, releases the hotkey, main window back to onboarding.
- `search_latency` (`crates/bench`): Public Corpus (71 files), File view, 58 Queries × 3: CUDA p50 24 ms / p95 33 ms; CPU p50 71 ms / p95 135 ms; target ≤ 1 s met.
- `fetch_public_corpus.py`: an unreachable site no longer stops the run; missing files listed.
- Installer: `pnpm bundle` → per-user NSIS setup (47 MB) with DirectML + ONNX Runtime CUDA provider DLLs; silent install, start (CUDA loaded), uninstall verified clean.
- Not yet: launcher appear time (≤ 300 ms) unmeasured; Purge not tried in the real window; installer unsigned.

## In progress
- M3 desktop app (`app/`: Tauri 2 + React + TypeScript + Tailwind, Vitest; pnpm).
  - Done: Tauri shell (`sbm-app`, workspace member) with `speed_choices` command; onboarding speed step (plain language, recommended picked, no CUDA/DirectML wording).
  - Done: dark plum theme with magenta accent (VoiceStudio look, ideas only).
  - Done: `setup::Plan` (model + CUDA runtime for NVIDIA), `setup::prepare` (one progress over both), `setup::TimeLeft` (rate since start, none for the first 3 s).
  - Done: download step: one progress bar, GB done, time left in plain words; `prepare` command emits throttled `setup-progress` events; stopped download offers "Try again" (resumes).
  - Done: folders step (personal folder picked, other drives offered; agreed with the user); follows a finished download; `start_indexing` starts the Engine (`start_loading` on the picked Accelerator, CPU fallback, USN Journal) and adds the folders. CUDA runtime folder put on `PATH` at startup.
  - Done: hotkey step (Alt + Space; says when another app uses it; records other keys), done step; `Settings` (`settings.json`: Accelerator, hotkey) saved on finish; next start resumes the Engine, registers the hotkey, starts hidden with a tray icon (Search, Quit); window shows a launcher placeholder.
  - Done: launcher slice 1: File-view Results as you type (name, Location, Match Reason, snippet, path); ↑/↓; Enter opens, Ctrl/Shift+Enter reveals, Ctrl+C copies path; Escape hides; Tab toggles Passage view; several Copies → Detail Panel to pick one (→ opens, ← closes); Disconnected Drive labelled, not opened; Query box focused on show. `search`, `open_file`, `reveal_file`, `hide_launcher` commands (`tauri-plugin-opener` 2.7.0); app honours `SBM_DATA_DIR`. Checked in the real window on a scratch index.
  - Done: launcher follows DigUp (user's choice), stage 1: frameless Acrylic panel (off taskbar, on top, hides on blur after onboarding); Results beside a Preview Pane (snippet as a paper page with the Query's words marked, folder, size, date, why it matched, Open / Show in folder, Copies picked there); footer status + key hints; click selects; Ctrl+C copies the file (`copy_file`, clipboard-win 5.4.1); Passage view and Detail Panel dropped from the launcher (PRD/CONTEXT updated). Palette "archive at night" (ink-slate glass, lamplight amber), onboarding restyled to match, with a title strip (drag, Quit).
  - Done: stage 2: `SearchResult.weaker` (content-only, no Query words, similarity > 0.08 below the best); rows carry `kind` (by extension) and `weaker`; Results grouped by kind in order of each group's best; Weaker Matches folded into one row (Enter toggles, → or click unfolds); footer "1 result, 2 weaker". Checked with the real model on a scratch index.
  - Done: stage 3: `text_preview` (matched lines ± 3, at most 40, first 40 for name-only; streamed, lossy UTF-8) via `preview_lines`; rows carry `lineStart`/`lineEnd`; code shown in a numbered mono panel, text documents on the paper page, matched lines lit and Query words marked; snippet shown while loading or unreadable.
  - Done: stage 4a/4b: `code:` searches code only (`Index::search_where` / `Engine::search_where` filter before counting and Weaker Matches; icon turns to `</>`); `sbm-thumbnail` crate (Windows Shell thumbnails as PNG, thumbnail-only, cache-only for Online-only Files); rows show thumbnails or a kind icon.
  - Done: stage 4c: PDFium 7881 (`pdfium-render` 0.9.4) planned for every speed choice and installed by `prepare` (tgz runtimes: DLLs + licenses); `sbm-pdf` renders a page as PNG and marks the Query's words; rows carry `page`; `pdf_page` command; Preview Pane shows the matched page with highlighter marks.
  - Done: pictures and videos shown as a large thumbnail (840 px) in the Preview Pane; `thumbnail` command takes a size.
  - Done: a PDF page scrolls to the first of the Query's words.
  - Done: asking which Copy starts on the one picked last time (WebView `localStorage`, still confirmed with Enter).
  - Done: Cancel on the download step (`cancel_prepare`): back to the speed choices, download kept for resuming.
  - Done: PDF text read with PDFium where installed, pdf-extract as fallback (ADR 0006); scans' invisible OCR layers now indexed.
  - Done: full onboarding run through the GUI (Fast/DirectML, scratch data dir and home); one run indexed no content for 11 min, not reproduced in 5 reruns.
  - Next: tune `WEAKER_MARGIN` on the Benchmark.
- App redesign (user, 2026-10-15): VoiceStudio Classic look app-wide; one-location onboarding with a visible indexing step; Settings + live dashboard; features beyond DigUp.
  - Done: stage 1: VoiceStudio Classic tokens (dark + light by OS theme), Inter bundled, lucide icons; `Button` (primary/outline/ghost/danger, never submits unless asked) used across onboarding, Settings, Preview; launcher restyled (chrome list, pill chips, key caps, tinted kind tiles); Query box without spell-check. Checked in the real window (dark).
  - Done: stage 2: Wizard rail (Back on Speed, Search keys, Done); Welcome; Download skippable (model loads once a download finished; tray shows %; Try again on the Indexing step); one Location (Change… via folder dialog, Reset to default, estimate with Default Exclusions); Indexing step (names found, files read of total, speed from when reading began, time left, current file, failures, why it waits); Ctrl+Shift+Space default with a live "They work." check. Checked end to end in the real window (DirectML, scratch data).
  - Done along the way: system drive Default Exclusions (Windows, Program Files, ProgramData, every user's AppData, …) now applied, listed as Default in Settings, can be turned off and restored; `Engine` status `scanning` and `current`; `Index::next_queued_file`, `default_exclusions`, `estimate`.
  - Done: stage 3: Settings in the VoiceStudio layout (section rail, glass panels of rows); frameless acrylic Settings window with its own title strip and Close; Status shows the same reading progress as onboarding (shared `IndexingProgress`); Try again for unreadable files (`Index::retry_failed`, `Engine::retry_failed`, `retry_failed` command). Checked in the real window.
  - Found: the Settings window was once found minimized between two UI Automation calls; cause unknown.
  - Done: stage 4a: Filter Prefixes anywhere in the Query, combinable (`sbm_app::Filters`): `ext:` (several = any), `in:` (folder name contains; quoted values), `date:` today / yesterday / week / month / year (last 7/30/365 days) / `2025` / `2025-03` by modified time, `code:`, `name:` (`Engine::search_names`, names only); empty prefixes ignored, unknown ones stay in the Query. `search` takes the typed text and answers `{ query, results, chips }`; chips shown in the Query bar.
  - Done: `in:` matches folder names only, not the drive; a Query of filters only (`date:today`) lists the newest files that pass them (`Engine::newest_files`), the Preview Pane says "It fits the filters".
  - Done: the Preview Pane says why only a Result's name is searched: online only (download it), format not read (pictures, videos, audio, other), not read yet, could not be read (with the reason). `Index`/`Engine::not_read`, `ResultRow.notRead`.
  - Done: stage 4: Speed, Download and Search keys steps in the VoiceStudio card style (icon tiles, shared progress bar, one key cap per key; the keys button names the keys for screen readers; a stopped download says "Stopped").
  - Done: real-window check of stage 4 (scratch data and home): filter-only listing, `in:` without the drive, `date:2023`, why only a name is searched (online only, picture, damaged), restyled steps, "They work.".
  - Fixed on the way: the reading bar fills once every file was read or given up on; no time left once nothing is pending.
  - Found: a name-only Result ranks below unrelated content-only Results ("holiday" puts `retry.rs` above `holiday beach.jpg` on 7 files); the Download step counts 0.0 of 3.0 GB although the model is already on disk (Speed says 1.9 GB); speed keeps falling after reading ends.
  - Found: Live Updates run before Initial Indexing (`live DESC`), so a folder that keeps changing can hold content reading back; check with the Benchmark of a busy Downloads folder.
  - Found: while Initial Indexing runs (debug build, DirectML), a Query stayed "Searching…" for over 10 s; check whether Query embedding waits behind indexing batches.

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
