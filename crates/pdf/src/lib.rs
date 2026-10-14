//! PDF pages as pictures, with where the Query's words sit on them.

use std::path::Path;
use std::sync::OnceLock;

use pdfium_render::prelude::*;

/// PDFium, loaded once per process (its bindings are global), or why it failed.
static PDFIUM: OnceLock<Result<Pdfium, String>> = OnceLock::new();

/// Renders PDF pages with PDFium.
pub struct PdfPages(&'static Pdfium);

/// One page as a PNG.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPage {
    pub png: Vec<u8>,
    /// Where the words asked for sit on the page.
    pub marks: Vec<Area>,
}

/// Part of a page, in fractions of its width and height from the top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl PdfPages {
    /// PDFium from the folder `dir` holding its library.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let loaded = PDFIUM.get_or_init(|| {
            let library = Pdfium::pdfium_platform_library_name_at_path(dir);
            let bindings = Pdfium::bind_to_library(library).map_err(|error| error.to_string())?;
            Ok(Pdfium::new(bindings))
        });
        loaded.as_ref().map(Self).map_err(Clone::clone)
    }

    /// Page `number` (1-based) of the PDF `path`, `width` pixels wide, with the
    /// places of `words` on it; `None` when the file or page cannot be read.
    pub fn page(
        &self,
        path: &Path,
        number: u32,
        width: u32,
        words: &[&str],
    ) -> Option<RenderedPage> {
        let document = self.0.load_pdf_from_file(path, None).ok()?;
        let index = PdfPageIndex::try_from(number.checked_sub(1)?).ok()?;
        let page = document.pages().get(index).ok()?;
        let config = PdfRenderConfig::new().set_target_width(width as Pixels);
        let bitmap = page.render_with_config(&config).ok()?;
        let png = encode_png(
            bitmap.width() as u32,
            bitmap.height() as u32,
            &bitmap.as_rgba_bytes(),
        )?;
        Some(RenderedPage {
            png,
            marks: marks(&page, words),
        })
    }
}

/// Where each of `words` appears on `page`, as whole words in any case.
fn marks(page: &PdfPage, words: &[&str]) -> Vec<Area> {
    let (width, height) = (page.width().value, page.height().value);
    let Ok(text) = page.text() else {
        return Vec::new();
    };
    let options = PdfSearchOptions::new().match_whole_word(true);
    let mut marks = Vec::new();
    for word in words {
        let Ok(search) = text.search(word, &options) else {
            continue;
        };
        while let Some(found) = search.find_next() {
            for segment in found.iter() {
                let bounds = segment.bounds();
                marks.push(Area {
                    left: bounds.left().value / width,
                    top: (height - bounds.top().value) / height,
                    width: bounds.width().value / width,
                    height: bounds.height().value / height,
                });
            }
        }
    }
    marks
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(bytes)
}
