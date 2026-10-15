use std::path::Path;

use sbm_core::{ContentExtractor, Location, Unreadable};
use sbm_extract::PdfExtractor;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn pdf_is_extracted_page_by_page() {
    let sections = PdfExtractor::default()
        .extract(&fixture("two-pages.pdf"))
        .unwrap();

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
fn password_protected_pdf_is_reported_as_protected() {
    // two-pages.pdf encrypted by PDFtk with the user password "secret".
    let error = PdfExtractor::default()
        .extract(&fixture("password-protected.pdf"))
        .unwrap_err();

    assert_eq!(
        Unreadable::of(error.as_ref()),
        Unreadable::PasswordProtected
    );
}

#[test]
fn corrupt_pdf_that_crashes_pdf_extract_is_read_by_pdfium_or_is_an_error() {
    // A one-byte corruption of two-pages.pdf that makes pdf-extract panic;
    // PDFium, where installed, reads past it.
    let result = PdfExtractor::default().extract(&fixture("corrupt-panics-parser.pdf"));

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
    let sections = PdfExtractor::default()
        .extract(&fixture("invisible-text-layer.pdf"))
        .unwrap();

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].location, Location::Page(1));
    assert!(
        sections[0]
            .text
            .contains("Subjek Data Pribadi berhak mendapatkan akses")
    );
}

#[test]
fn pdf_whose_text_is_beyond_the_size_limit_is_refused_as_too_large() {
    // The default limit is 50 MB; reading that much takes PDFium ~20 s.
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("huge.pdf");
    std::fs::write(&path, pdf_with_text(20_000)).unwrap();

    let error = PdfExtractor::with_limit(10_000).extract(&path).unwrap_err();

    assert!(error.to_string().contains("too large"), "{error}");
    assert_eq!(Unreadable::of(error.as_ref()), Unreadable::TooLarge);
}

/// A one-page PDF showing at least `bytes` (up to ~70 KB) of text, in lines of
/// 100 characters laid out in columns, so none of it overlaps or leaves the page.
fn pdf_with_text(bytes: usize) -> Vec<u8> {
    let lines: String = (0..bytes.div_ceil(100))
        .map(|line| {
            let (column, row) = (line / 760, line % 760);
            format!(
                "1 0 0 1 {} {} Tm ({}) Tj\n",
                10 + column * 60,
                780 - row,
                "word ".repeat(20)
            )
        })
        .collect();
    let content = format!("BT /F1 1 Tf\n{lines}ET\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n{object}\nendobj\n", number + 1).bytes());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    pdf.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    pdf
}
