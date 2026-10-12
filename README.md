<div align="center">

<img src="assets/logo.png" alt="Search by Memory Logo" width="180" style="border-radius: 20px; box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);" />

# Search by Memory
### *Find what you remember, not what you named.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Local-First](https://img.shields.io/badge/Privacy-100%25%20Local-success?style=for-the-badge&logo=shield)](implementation/PRD.md)
[![Architecture](https://img.shields.io/badge/Stack-Tauri%202%20%7C%20Rust%20%7C%20React-orange?style=for-the-badge&logo=rust)](implementation/PRD.md)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet?style=for-the-badge&logo=windows)](implementation/PRD.md)

<p align="center">
  <b>Instant semantic desktop search powered by local vector retrieval.</b><br>
  Search through your documents, code, notes, and media using vague recollections or natural descriptions.
</p>

```text
Alt + Space  ->  "That tutorial about attention mechanisms"  ->  Found in 32ms
```

---

</div>

## What is it?

We collect thousands of files across deep folders, but our brains remember **concepts**, **context**, or **snippets**—not filenames like `Report_final_v2_final.docx` or `screenshot_20261012.png`.

**Search by Memory** is a desktop launcher that bridges this gap using local semantic embeddings (**Google EmbeddingGemma 2** via ONNX Runtime) and vector similarity search (**USearch**), alongside instant filename and keyword search across your whole disk.

---

## Core Highlights

<table>
<tr>
<td width="50%">

### 100% Local and Private
* Zero cloud telemetry.
* No documents or queries leave your machine.
* Completely functional in air-gapped and offline environments.

</td>
<td width="50%">

### Instant Launcher (`Alt + Space`)
* Keyboard-first workflow (Arrow keys, `Enter`, `Ctrl+Enter`).
* Sub-second retrieval ($P95 < 1\text{s}$) across millions of chunks on a 1 TB+ disk.
* Minimal background memory and CPU footprint.

</td>
</tr>
<tr>
<td width="50%">

### Semantic-First Retrieval
* No generative LLM hallucinations or chat latency.
* Searches meaning, concepts, and synonyms—plus filenames and exact keywords, merged into one list.
* True source snippets and direct page or line references.

</td>
<td width="50%">

### Resilient Background Indexing
* Name search ready in minutes; content indexed newest-first in the background.
* Incremental change detection; moved and copied files are never re-embedded.
* Pauses automatically for full-screen apps, battery, and busy hardware.
* Choose any folder or drive, with sensible default exclusions pre-filled.

</td>
</tr>
</table>

---

## System Architecture

The project is structured around a clean 5-layer modular architecture to keep domain logic strictly decoupled from OS-specific integrations and UI rendering:

```mermaid
graph TD
    subgraph Presentation["1. Presentation Layer"]
        UI["React + TypeScript UI<br/>(Tailwind CSS, Virtualized Results)"]
        Tauri["Tauri 2 Shell<br/>(Window Lifecycle, Tray, IPC)"]
        UI <--> Tauri
    end

    subgraph Core["2. Application Core (Rust)"]
        Orch["Search Orchestrator"]
        Sched["Index Scheduler & Queue"]
        Rank["Result Ranker & Deduplicator"]
    end

    subgraph Adapters["3. Platform Adapters"]
        Hotkeys["Global Shortcuts<br/>(Win32 / X11 / Cocoa)"]
        Watcher["Change Source<br/>(NTFS USN Journal / inotify / FSEvents)"]
        OSOps["OS File Operations<br/>(Default App Launch & Reveal)"]
    end

    subgraph Providers["4. Provider Interfaces"]
        IEmbed["EmbeddingProvider<br/>(EmbeddingGemma 2)"]
        IVector["VectorStore<br/>(USearch)"]
        IExtract["ContentExtractor<br/>(PDF, DOCX, Code, Text)"]
    end

    subgraph Storage["5. Storage Layer"]
        DB[("SQLite Database<br/>Files, Chunks, Jobs & Full-text")]
        VIndex[("USearch Vector Index<br/>Derived from SQLite")]
        Thumb["Thumbnail Cache"]
    end

    Presentation <==>|Tauri IPC Commands & Events| Core
    Core <--> Adapters
    Core <--> Providers
    Providers <--> Storage
    Core <--> Storage
```

### Technology Breakdown

* **Desktop Shell**: Tauri 2 + React + TypeScript + Tailwind CSS for a featherlight desktop footprint.
* **Application Core**: Rust for deterministic concurrency, memory safety, and high-throughput job queueing.
* **Embedding Engine**: Google EmbeddingGemma 2 running in-process via ONNX Runtime in Rust (no Python shipped; Python is used only for offline dev tooling).
* **Storage and Index**: SQLite (with FTS5) for metadata, filename and keyword search & USearch for sub-second vector similarity retrieval.

---

## Roadmap and Supported Formats

| Phase | Modality | Supported Formats |
| :--- | :--- | :--- |
| **P0 (MVP)** | **Names, Documents and Code** | Filename search for every file; content search (semantic + keyword) for `.pdf`, `.docx`, `.txt`, `.md`, source code (`.rs`, `.py`, `.ts`, `.go`, etc.) |
| **P1** | **Visual and Screenshots**| Image embeddings, OCR for screenshots, thumbnail preview cards |
| **P2** | **Audio and Video** | Audio transcription via local Whisper, FFmpeg soundtrack extraction, timestamp jumping |
| **P3** | **Advanced Retrieval** | Cross-modal blending, non-generative reranking, local query history |

---

## Documentation

For complete technical specifications, architectural diagrams, acceptance criteria, and benchmarks, check the full PRD:

[Read the Full Product Requirements Document (PRD.md)](implementation/PRD.md)

* [Domain glossary (CONTEXT.md)](CONTEXT.md) — the shared vocabulary used across docs and code.
* [Architecture Decision Records (docs/adr/)](docs/adr/) — why we chose ONNX Runtime, USearch, two indexes, and content-keyed chunks.

---

## License

This project is licensed under the [MIT License](LICENSE).


