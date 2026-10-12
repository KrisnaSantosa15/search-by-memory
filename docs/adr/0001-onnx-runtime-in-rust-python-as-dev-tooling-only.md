---
status: accepted
---

# ONNX Runtime in Rust for the shipped app; Python only as offline dev tooling

The PRD originally planned a Python + PyTorch embedding sidecar for the MVP with a later migration to ONNX Runtime. We decided to skip that intermediate step: the shipped application embeds Queries and Chunks in-process via ONNX Runtime (`ort` crate) inside the Rust core, and Python is used only for offline developer tooling (Benchmark runner, embedding-dimension experiments, and parity checks of ONNX output against the PyTorch reference). Shipping Python would mean bundling an interpreter plus a multi-gigabyte PyTorch/CUDA stack, managing a sidecar process lifecycle, and designing an IPC protocol we already intended to discard — all at odds with a lightweight, offline, single-installer desktop app.

## Consequences

- PRD open decision #1 (Rust ↔ Python IPC) is eliminated; #5 (runtime migration) is resolved.
- Tokenization and the model's query/document prompt formatting must be reproduced in Rust and verified against the reference implementation.
- GPU acceleration comes from ONNX Runtime execution providers (CUDA / DirectML) rather than PyTorch.
- Milestone 1 includes a go/no-go gate: ONNX embeddings must match PyTorch embeddings (cosine ≥ ~0.99 on Benchmark inputs) and meet latency budgets on CPU and GPU. If the gate fails, this decision is revisited in favour of a Python sidecar.
