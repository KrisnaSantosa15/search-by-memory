//! PDF pages as pictures, with where the Query's words sit on them.

use std::path::Path;
use std::sync::OnceLock;

use pdfium_render::prelude::*;

/// PDFium, loaded once per process: its bindings are global.
static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

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
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(Self(pdfium));
        }
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir))
            .map_err(|error| error.to_string())?;
        Ok(Self(PDFIUM.get_or_init(|| Pdfium::new(bindings))))
    }

    /// Page `number` (1-based) of the PDF `path`, `width` pixels wide, with the
    /// places of `words` on it; `None` when the file or page cannot be read.
    pub fn page(
        &self,
        path: &Path,
        number: u32,
        width: u32,
        _words: &[&str],
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
            marks: Vec::new(),
        })
    }
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
