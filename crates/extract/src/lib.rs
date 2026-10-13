//! ContentExtractors for the Content Index: PDF, DOCX, plain text, Markdown, and code.

use std::path::Path;

use quick_xml::events::Event;
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

/// Word documents: one line per paragraph of the main document body.
pub struct DocxExtractor;

impl ContentExtractor for DocxExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<Section>> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        let document = std::io::BufReader::new(archive.by_name("word/document.xml")?);
        let mut reader = quick_xml::Reader::from_reader(document);
        let mut buffer = Vec::new();
        let mut text = String::new();
        let mut lines = 0;
        let mut in_text = false;
        loop {
            match reader.read_event_into(&mut buffer)? {
                Event::Start(tag) if tag.name().as_ref() == "w:t" => in_text = true,
                Event::End(tag) if tag.name().as_ref() == "w:t" => in_text = false,
                Event::Text(content) if in_text => text.push_str(&content),
                Event::End(tag) if tag.name().as_ref() == "w:p" => {
                    text.push('\n');
                    lines += 1;
                }
                Event::Eof => break,
                _ => {}
            }
            buffer.clear();
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
