# Agent Instructions — Search by Memory

These rules apply to every agent and contributor working in this repository.

## Read first

- [`CONTEXT.md`](CONTEXT.md) — the domain glossary. Use its terms exactly (e.g. **Chunk**, **Content Match**, **Name Index**, **Copy**) in code, test names, and docs; avoid the words listed under `_Avoid_`.
- [`docs/adr/`](docs/adr/) — accepted architecture decisions. Do not contradict an ADR; if a change requires it, propose a new ADR that supersedes it.
- [`implementation/PRD.md`](implementation/PRD.md) — requirements, milestones, and acceptance criteria.

## Test-Driven Development is mandatory

All production code in this repository is written test-first. No exceptions for "small" changes, spikes, or prototypes: if it is code that will run, a failing test comes before it.

### The loop

Work in **vertical slices**, one behavior at a time:

1. **RED** — write ONE test for ONE observable behavior. Run it and confirm it fails for the expected reason (not a compile error in unrelated code, not a typo).
2. **GREEN** — write the minimum code that makes that test pass. Nothing speculative.
3. **REFACTOR** — only while all tests are green; run the full suite after each step.
4. Repeat with the next behavior.

**Never** write a batch of tests up front and then a batch of implementation ("horizontal slicing"). Each test should be informed by what the previous cycle taught you.

### What a good test looks like

- Tests verify **behavior through public interfaces** — crate APIs and the provider traits (`EmbeddingProvider`, `VectorStore`, `ContentExtractor`, `ChangeSource`) — not private functions or internal data layouts.
- A test must survive an internal refactor. If renaming an internal function breaks a test, the test was wrong.
- Test names read as specifications in domain language, e.g. `moved_file_reuses_existing_chunks_without_reembedding`, `online_only_file_is_in_name_index_but_not_content_index`.
- Mock or fake only at **system boundaries** (filesystem, OS adapters, the ONNX model, clock). Never mock internal collaborators. Prefer in-memory fakes of the provider traits over mocking libraries.
- Prefer real SQLite (in-memory or temp file) and real USearch indexes in tests over fakes; they are fast enough.

### Before starting a feature

1. Agree with the user on the public interface and the prioritized list of behaviors to test.
2. Write the first test (tracer bullet) that proves the path end-to-end.
3. Proceed one behavior at a time.

### Per-cycle checklist

- [ ] The test failed before the implementation existed, for the right reason
- [ ] The test describes behavior, not implementation
- [ ] The test uses only the public interface
- [ ] The implementation is the minimum needed to pass
- [ ] The full suite is green before moving on

### Measurements are not exempt

Benchmarks and validation scripts (embedding throughput, ONNX ↔ PyTorch parity, USearch recall, retrieval metrics) contain logic — metric calculations, corpus loading, label parsing. That logic is test-driven like any other code; only the final measurement run itself is not a unit test. Parity and acceptance thresholds from the ADRs (e.g. cosine ≥ ~0.99 in ADR 0001) are expressed as executable tests.

## Commits

Always commit in the smallest possible units.

- **One logical change per commit** — ideally a single file. Split unrelated edits (e.g. a doc tweak and a code change) into separate commits, even when made together.
- **Every commit must build and pass the test suite.** In TDD this means a failing test and the code that makes it pass go in the same commit; commit after each GREEN, and again after each REFACTOR step.
- Commit as you go, not in a batch at the end of a session.
- Use Conventional Commit messages: `feat(scope):`, `fix(scope):`, `test(scope):`, `refactor(scope):`, `docs(scope):`, `build:`, `chore:`.

## Running tests

- Rust: `cargo test --workspace`
- Python dev tooling (`tools/`): `uv run pytest`

A change is not done until the relevant suite passes. Report failing tests honestly, with their output.
