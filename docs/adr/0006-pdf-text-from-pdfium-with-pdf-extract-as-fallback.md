---
status: accepted
---

# PDF text is read with PDFium, with pdf-extract as the fallback

PDFs were read with pdf-extract (pure Rust, on lopdf). It returns no text for scans whose OCR layer is invisible text in a non-embedded font: UU 27/2022 gives 619 characters over 50 pages, where PDFium reads about 1,300 per page. It also panics on many malformed files. The desktop app already ships PDFium (`pdfium-render`, a pinned `chromium/7881` build installed by setup) to draw PDF pages in the Preview Pane.

We therefore read PDF text with PDFium wherever setup installed it, one Section per page as before, and fall back to pdf-extract when PDFium is missing (the CLI on a machine without the app's setup, Linux, the benches) or cannot open the file.

## Consequences

- Scans with an OCR layer become searchable by content; Benchmark Queries id-001 and id-002 depend on this.
- The same PDF can give different text, so different Chunks, with or without PDFium. Chunks are keyed by content hash (ADR 0004), so a PDF indexed before PDFium was installed keeps its pdf-extract Chunks until its content changes.
- PDFium is native code loaded once per process; the indexing thread and the Preview Pane share it (`thread_safe`, calls serialized).
- pdf-extract stays a dependency for the fallback, with its panics caught as before.
