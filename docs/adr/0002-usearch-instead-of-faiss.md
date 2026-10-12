---
status: accepted
---

# USearch instead of FAISS as the vector index

The target is a whole 1 TB disk, which can yield millions of Chunks, so an approximate nearest-neighbour index is justified over exhaustive search. We chose USearch over FAISS because its Rust crate builds its own C++ core without a separate FAISS/BLAS toolchain (FAISS is painful to build for Rust on Windows), it supports removing vectors by key (FAISS's `IndexHNSWFlat` does not), it can memory-map a saved index to keep idle RAM low, and it supports compact `f16`/`i8` storage (roughly 0.5–1 GB for 2M Chunks at 256 dimensions). SQLite remains the source of truth: the USearch file is a derived artefact that can be rebuilt from stored Chunk embeddings without re-reading user files.

## Considered Options

- **FAISS** — the original PRD choice; rejected for Windows build friction and lack of deletion in HNSW.
- **Exhaustive (exact) search** — simplest and exact, but latency grows linearly with Chunk count and becomes marginal at whole-disk scale.
- **arroy** — pure Rust, but requires LMDB as a second database alongside SQLite.
- **hnsw_rs** — pure Rust, but limited deletion support.
- **LanceDB** — capable, but far heavier than a launcher needs.

## Consequences

- All access goes through the `VectorStore` interface, so the backend can be swapped later.
- Milestone 1 must confirm the Rust removal API, measure recall against exact search on the Benchmark, and choose `f16` vs `i8` storage.
