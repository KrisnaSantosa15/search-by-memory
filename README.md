<div align="center">

# 🧠 Search by Memory
### *Find what you remember, not what you named.*

[![Local-First](https://img.shields.io/badge/Privacy-100%25%20Local-success?style=for-the-badge&logo=shield)](implementation/PRD.md)
[![Architecture](https://img.shields.io/badge/Stack-Tauri%202%20%7C%20Rust%20%7C%20React-orange?style=for-the-badge&logo=rust)](implementation/PRD.md)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet?style=for-the-badge&logo=windows)](implementation/PRD.md)

<p align="center">
  <b>Instant semantic desktop search powered by local vector retrieval.</b><br>
  Search through your documents, code, notes, and media using vague recollections or natural descriptions.
</p>

```
Alt + Space  ➜  "That tutorial about attention mechanisms"  ➜  Found in 32ms ⚡
```

---

</div>

## 💡 What is it?

We collect thousands of files across deep folders, but our brains remember **concepts**, **context**, or **snippets**—not filenames like `Report_final_v2_final.docx` or `screenshot_20261012.png`.

**Search by Memory** is a lightning-fast desktop launcher that bridges this gap using local semantic embeddings (**Google EmbeddingGemma 2**) and vector similarity search (**FAISS**).

---

## ⚡ Core Highlights

<table>
<tr>
<td width="50%">

### 🔒 100% Local & Private
* Zero cloud telemetry.
* No documents or queries leave your machine.
* Completely functional in air-gapped / offline environments.

</td>
<td width="50%">

### 🚀 Instant Launcher (`Alt + Space`)
* Keyboard-first workflow (Arrow keys, `Enter`, `Ctrl+Enter`).
* Sub-second retrieval ($P95 < 1\text{s}$) across hundreds of thousands of vectors.
* Minimal background memory and CPU footprint.

</td>
</tr>
<tr>
<td width="50%">

### 🎯 Semantic-First Retrieval
* No generative LLM hallucinations or chat latency.
* Searches meaning, concepts, and synonyms—not just exact keywords.
* True source snippets and direct page/line references.

</td>
<td width="50%">

### 🔄 Resilient Background Indexing
* Incremental file change detection (watches modified/deleted files).
* Automatic pause, resume, and checkpoint recovery.
* Granular folder and file-extension exclusions.

</td>
</tr>
</table>

---

## 🖥️ Launcher Experience

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔍  Type what you remember about the file...                          [Esc] │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📄 Quarterly Financial Report Q3.docx                         (Page 14) 89% │
│    "...net profit increased by 18% following the launch..."                 │
│    D:\Work\Finances\2026\Q3_Report.docx                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🦀 auth_service.rs                                             (Line 142)84% │
│    "pub async fn refresh_jwt_token(token: &str) -> Result"                  │
│    D:\Projects\backend\src\auth_service.rs                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🖼️ database_architecture_diagram.png                                     78% │
│    [Thumbnail Preview] Primary-Replica failover cluster                     │
│    D:\Notes\Architecture\database_diagram.png                               │
├─────────────────────────────────────────────────────────────────────────────┤
│ [↵ Open File]       [Ctrl+↵ Reveal in Folder]       [Ctrl+C Copy Path] 28ms │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🏗️ Architecture at a Glance

Search by Memory is engineered around a clean **5-Layer Modular Architecture**:

```
 ┌─────────────────────────────────────────────────────────┐
 │  Presentation Layer : React + TypeScript (Tauri Shell)  │
 └────────────────────────────┬────────────────────────────┘
                              │ IPC
 ┌────────────────────────────▼────────────────────────────┐
 │  Application Core   : Rust (Orchestrator & Concurrency) │
 └──────────────┬───────────────────────────┬──────────────┘
                ▼                           ▼
   [ Platform Adapters ]          [ Provider Interfaces ]
   (Shortcuts, Watcher, OS)       (Embedding, VectorStore)
                │                           │
 ┌──────────────▼───────────────────────────▼──────────────┐
 │  Storage Layer      : SQLite (Metadata) + FAISS (Vectors)│
 └─────────────────────────────────────────────────────────┘
```

* **Frontend Shell**: Tauri 2 + React + TypeScript + Tailwind CSS.
* **Core Engine**: Rust for memory-safe indexing orchestration, job queuing, and ranking.
* **Embedding Backend**: Python + EmbeddingGemma 2 (local IPC, upgradeable to ONNX Runtime).
* **Storage & Index**: SQLite for transactional metadata & FAISS for high-throughput vector search.

---

## 🗺️ Roadmap & Supported Formats

| Phase | Modality | Supported Formats |
| :--- | :--- | :--- |
| **P0 (MVP)** | **Documents & Code** | `.pdf`, `.docx`, `.txt`, `.md`, source code (`.rs`, `.py`, `.ts`, `.go`, etc.) |
| **P1** | **Visual & Screenshots**| Image embeddings, OCR for screenshots, thumbnail preview cards |
| **P2** | **Audio & Video** | Audio transcription via local Whisper, FFmpeg soundtrack extraction, timestamp jumping |
| **P3** | **Advanced Retrieval** | Hybrid lexical (BM25) + semantic search, cross-modal blending, non-generative reranking |

---

## 📚 Documentation

For complete technical specifications, architectural diagrams, acceptance criteria, and benchmarks, check the full PRD:

👉 **[Read the Full Product Requirements Document (PRD.md)](implementation/PRD.md)**

---

<div align="center">
  <sub>Built with ❤️ for privacy-minded power users, researchers, and developers.</sub>
</div>

