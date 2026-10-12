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

**Search by Memory** is a desktop launcher that bridges this gap using local semantic embeddings (**Google EmbeddingGemma 2**) and vector similarity search (**FAISS**).

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
* Sub-second retrieval ($P95 < 1\text{s}$) across hundreds of thousands of vectors.
* Minimal background memory and CPU footprint.

</td>
</tr>
<tr>
<td width="50%">

### Semantic-First Retrieval
* No generative LLM hallucinations or chat latency.
* Searches meaning, concepts, and synonyms—not just exact keywords.
* True source snippets and direct page or line references.

</td>
<td width="50%">

### Resilient Background Indexing
* Incremental file change detection (watches modified and deleted files).
* Automatic pause, resume, and checkpoint recovery.
* Granular folder and file-extension exclusions.

</td>
</tr>
</table>

---

## How It Works

Search by Memory indexes files in the background so search is instantaneous when invoked.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Launcher as Tauri UI (React)
    participant Core as Rust Core
    participant Embedder as Embedding Engine
    participant Store as FAISS & SQLite

    Note over Core,Store: Background Indexing (Zero search latency)
    Core->>Store: Extract chunks & generate vectors
    
    Note over User,Store: Instant Query Flow
    User->>Launcher: Global shortcut (Alt + Space)
    Launcher->>User: Overlay search bar appears (<300ms)
    User->>Launcher: Types natural query ("auth refresh token")
    Launcher->>Core: Dispatches search request
    Core->>Embedder: Encode query vector
    Embedder-->>Core: Query embedding
    Core->>Store: Top-K Vector Search + Metadata Lookup
    Store-->>Core: Ranked candidates with text snippets
    Core-->>Launcher: Streams results
    Launcher-->>User: Displays matches with direct page/line references
    User->>Launcher: Press Enter to open file or Ctrl+Enter to reveal in folder
```

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
        Watcher["Filesystem Watcher<br/>(ReadDirectoryChangesW / inotify)"]
        OSOps["OS File Operations<br/>(Default App Launch & Reveal)"]
    end

    subgraph Providers["4. Provider Interfaces"]
        IEmbed["EmbeddingProvider<br/>(EmbeddingGemma 2)"]
        IVector["VectorStore<br/>(FAISS Abstraction)"]
        IExtract["ContentExtractor<br/>(PDF, DOCX, Code, Text)"]
    end

    subgraph Storage["5. Storage Layer"]
        DB[("SQLite Database<br/>Metadata, Chunks & Jobs")]
        VIndex[("FAISS Vector Index<br/>Serialized Dense Vectors")]
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
* **Embedding Engine**: Python with Google EmbeddingGemma 2 (local IPC, designed for future migration to native ONNX Runtime).
* **Storage and Index**: SQLite for ACID relational metadata & FAISS for sub-second vector similarity retrieval.

---

## Roadmap and Supported Formats

| Phase | Modality | Supported Formats |
| :--- | :--- | :--- |
| **P0 (MVP)** | **Documents and Code** | `.pdf`, `.docx`, `.txt`, `.md`, source code (`.rs`, `.py`, `.ts`, `.go`, etc.) |
| **P1** | **Visual and Screenshots**| Image embeddings, OCR for screenshots, thumbnail preview cards |
| **P2** | **Audio and Video** | Audio transcription via local Whisper, FFmpeg soundtrack extraction, timestamp jumping |
| **P3** | **Advanced Retrieval** | Hybrid lexical (BM25) + semantic search, cross-modal blending, non-generative reranking |

---

## Documentation

For complete technical specifications, architectural diagrams, acceptance criteria, and benchmarks, check the full PRD:

[Read the Full Product Requirements Document (PRD.md)](implementation/PRD.md)

---

## License

This project is licensed under the [MIT License](LICENSE).


