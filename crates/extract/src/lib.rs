//! ContentExtractors for the Content Index: PDF, DOCX, plain text, Markdown, and code.

use std::path::Path;

use sbm_core::{ContentExtractor, Location, Result, Section};

/// Plain text, Markdown, and source code: the whole file as one line range.
pub struct TextExtractor;

impl ContentExtractor for TextExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<Section>> {
        let text = std::fs::read_to_string(path)?;
        let lines = text.lines().count() as u32;
        if lines == 0 {
            return Ok(Vec::new());
        }
        Ok(vec![Section {
            location: Location::Lines {
                start: 1,
                end: lines,
            },
            text,
        }])
    }
}

/// PDF documents: one Section per page.
pub struct PdfExtractor;

impl ContentExtractor for PdfExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<Section>> {
        let pages = pdf_extract::extract_text_by_pages(path)?;
        Ok(pages
            .into_iter()
            .enumerate()
            .map(|(index, text)| Section {
                location: Location::Page(index as u32 + 1),
                text,
            })
            .collect())
    }
}
