//! Rendering PDF pages with PDFium (the real library; skipped when it is missing).

use std::path::{Path, PathBuf};

use sbm_core::AppDirs;
use sbm_pdf::PdfPages;
use sbm_setup::PDFIUM_DIR;

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
