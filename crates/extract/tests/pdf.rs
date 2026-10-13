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
