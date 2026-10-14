use std::path::Path;

use sbm_core::{ContentExtractor, Location};
use sbm_extract::PdfExtractor;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn pdf_is_extracted_page_by_page() {
    let sections = PdfExtractor.extract(&fixture("two-pages.pdf")).unwrap();

    let pages: Vec<(Location, String)> = sections
        .into_iter()
        .map(|section| (section.location, section.text.trim().to_string()))
        .collect();
    assert_eq!(
        pages,
        [
            (Location::Page(1), "Invoice for March rent".to_string()),
            (Location::Page(2), "Paid by bank transfer".to_string()),
        ]
    );
}

#[test]
fn corrupt_pdf_that_crashes_pdf_extract_is_read_by_pdfium_or_is_an_error() {
    // A one-byte corruption of two-pages.pdf that makes pdf-extract panic;
    // PDFium, where installed, reads past it.
    let result = PdfExtractor.extract(&fixture("corrupt-panics-parser.pdf"));

    if pdfium_installed() {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

/// PDFium from the cache, where setup installs it; `false` (and SKIPPED) without it.
fn pdfium_installed() -> bool {
    let dir = sbm_core::AppDirs::current().cache.join(sbm_pdf::PDFIUM_DIR);
    let loaded = sbm_pdf::PdfPages::load(&dir).is_ok();
    if !loaded {
        println!("SKIPPED: PDFium not installed in {}", dir.display());
    }
    loaded
}

#[test]
fn scan_with_an_invisible_text_layer_is_extracted_with_pdfium() {
    if !pdfium_installed() {
        return;
    }
    // Page 6 of UU 27/2022 (public domain), cut to its OCR layer: invisible
    // text in a non-embedded font, where pdf-extract finds nothing.
    let sections = PdfExtractor
        .extract(&fixture("invisible-text-layer.pdf"))
        .unwrap();

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].location, Location::Page(1));
    assert!(sections[0].text.contains("Subjek Data Pribadi berhak mendapatkan akses"));
}
