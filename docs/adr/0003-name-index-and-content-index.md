---
status: accepted
---

# Two indexes: a Name Index for every file and a Content Index for supported files

The product must search a whole 1 TB disk and feel like Windows Search, but semantic embeddings are weak at exact names and identifiers (`INV-2024-0371.pdf`, `docker-compose.yml`), and embedding an entire disk takes hours. We therefore split indexing in two: a **Name Index** covering every in-scope file by filename and path, built in minutes using SQLite full-text search; and a **Content Index** covering Chunks of supported formats, searchable by meaning (EmbeddingGemma + USearch) and by keyword (SQLite full-text search over Chunk text). Every Query runs against both and the results are merged into one list. This pulls keyword/hybrid retrieval, originally deferred to P3 in the PRD, into the MVP because it is cheap with SQLite and essential at whole-disk scale.

## Consequences

- Name search is usable shortly after first launch, while the Content Index fills in over hours in the background.
- Files of unsupported formats (images, archives, executables) are still findable by name before P1/P2 modalities exist.
- A merge/ranking strategy across Name and Content results is required (to be decided).
