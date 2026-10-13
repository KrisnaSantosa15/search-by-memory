//! ContentExtractors for the Content Index: PDF, DOCX, plain text, Markdown, and code.

use std::path::Path;

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use sbm_core::{ContentExtractor, Location, Result, Section};

/// Plain text, Markdown, and source code: the whole file as one line range.
pub struct TextExtractor;

impl ContentExtractor for TextExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<Section>> {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut Limited {
                inner: std::fs::File::open(path)?,
                remaining: MAX_EXTRACTED_BYTES,
            },
            &mut bytes,
        )?;
        // Files in legacy encodings still yield their ASCII words.
        let text = String::from_utf8_lossy(&bytes).into_owned();
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
        // pdf-extract panics on some malformed PDFs; a bad file must not stop indexing.
        let pages = std::panic::catch_unwind(|| pdf_extract::extract_text_by_pages(path))
            .map_err(|_| format!("PDF parser crashed on {}", path.display()))??;
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

/// Largest content an extractor reads from one file (FR-03), guarding against
/// archives that decompress to far more than their size on disk.
const MAX_EXTRACTED_BYTES: u64 = 50 * 1024 * 1024;

/// A reader that fails once more than `remaining` bytes have been read.
struct Limited<R> {
    inner: R,
    remaining: u64,
}

impl<R: std::io::Read> std::io::Read for Limited<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.remaining = self
            .remaining
            .checked_sub(read as u64)
            .ok_or_else(|| std::io::Error::other("content too large to index"))?;
        Ok(read)
    }
}

/// Word documents: one line per paragraph of the main document body.
pub struct DocxExtractor;

impl ContentExtractor for DocxExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<Section>> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        let document = std::io::BufReader::new(Limited {
            inner: archive.by_name("word/document.xml")?,
            remaining: MAX_EXTRACTED_BYTES,
        });
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
                Event::GeneralRef(entity) if in_text => {
                    if let Some(character) = entity.resolve_char_ref()? {
                        text.push(character);
                    } else if let Some(resolved) = resolve_predefined_entity(&entity) {
                        text.push_str(resolved);
                    }
                }
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

/// Extensions of plain text, Markdown, and source code files (FR-03).
const TEXT_EXTENSIONS: [&str; 30] = [
    "txt", "md", "markdown", "rst", "csv", "log", "rs", "py", "js", "jsx", "ts", "tsx", "go", "c",
    "h", "cpp", "hpp", "cs", "java", "kt", "swift", "rb", "php", "sh", "sql", "html", "css",
    "json", "yaml", "toml",
];

/// The ContentExtractor for a file's format, or `None` when its content is not indexed.
pub fn extractor_for(path: &Path) -> Option<&'static dyn ContentExtractor> {
    let name = path.file_name()?.to_string_lossy();
    if name.starts_with("~$") {
        // Office lock files hold the editing user's name, not document content.
        return None;
    }
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    match extension.as_str() {
        "pdf" => Some(&PdfExtractor),
        "docx" => Some(&DocxExtractor),
        _ if TEXT_EXTENSIONS.contains(&extension.as_str()) => Some(&TextExtractor),
        _ => None,
    }
}
