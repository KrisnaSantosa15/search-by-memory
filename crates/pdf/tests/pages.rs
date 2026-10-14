//! Rendering PDF pages with PDFium (the real library; skipped when it is missing).

use std::path::{Path, PathBuf};

use sbm_core::AppDirs;
use sbm_pdf::{PDFIUM_DIR, PdfPages};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../extract/tests/fixtures")
        .join(name)
}

/// PDFium from the cache, where setup installs it; `None` (and SKIPPED) without it.
fn pdfium() -> Option<PdfPages> {
    let dir = AppDirs::current().cache.join(PDFIUM_DIR);
    match PdfPages::load(&dir) {
        Ok(pages) => Some(pages),
        Err(error) => {
            println!("SKIPPED: PDFium not loaded from {}: {error}", dir.display());
            None
        }
    }
}

/// Width and height of a PNG.
fn size_of_png(bytes: &[u8]) -> (u32, u32) {
    let reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    (reader.info().width, reader.info().height)
}

#[test]
fn page_is_rendered_as_a_picture_of_the_width_asked_for() {
    let Some(pdfium) = pdfium() else { return };

    let page = pdfium
        .page(&fixture("two-pages.pdf"), 2, 400, &[])
        .expect("page 2");

    let (width, height) = size_of_png(&page.png);
    assert_eq!(width, 400);
    assert!(height > 0);
}

#[test]
fn words_found_on_the_page_are_marked_where_they_sit() {
    let Some(pdfium) = pdfium() else { return };

    let page = pdfium
        .page(&fixture("two-pages.pdf"), 2, 400, &["Bank", "rent"])
        .expect("page 2");

    // "bank" is on page 2, in any case; "rent" is only on page 1.
    assert_eq!(page.marks.len(), 1, "{:?}", page.marks);
    let mark = page.marks[0];
    let inside = |start: f32, length: f32| start >= 0.0 && length > 0.0 && start + length <= 1.0;
    assert!(
        inside(mark.left, mark.width) && inside(mark.top, mark.height),
        "{mark:?}"
    );
}

#[test]
fn page_that_is_not_there_has_no_picture() {
    let Some(pdfium) = pdfium() else { return };

    assert_eq!(pdfium.page(&fixture("two-pages.pdf"), 3, 400, &[]), None);
    assert_eq!(pdfium.page(&fixture("gone.pdf"), 1, 400, &[]), None);
}
