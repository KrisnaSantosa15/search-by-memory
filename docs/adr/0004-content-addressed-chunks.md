---
status: accepted
---

# Chunks and embeddings are keyed by file content, not by file path

Embedding is the most expensive step of indexing, and on a whole-disk index files are constantly moved, renamed, and duplicated. If Chunks belonged to paths, moving a folder of 2,000 PDFs would re-embed tens of thousands of Chunks for unchanged content. We therefore key Chunks and their embeddings by a full-content hash of the file; paths are a separate many-to-one mapping onto that content. A file appearing at a new path with a known hash reuses the existing Chunks instantly, and identical Copies share one set of Chunks and are grouped into a single Result in File view.

## Consequences

- The schema separates *content* (hash → Chunks → embeddings) from *files on disk* (path → hash, plus size, mtime, drive), which differs from the PRD's original `chunks.file_id` design.
- Content is garbage-collected only when no path references its hash any longer.
- Editing one Copy gives it a new hash and splits it from the group; the remaining Copies keep the old content.
- Change detection and deduplication share the same full-content hash (see FR-04).
