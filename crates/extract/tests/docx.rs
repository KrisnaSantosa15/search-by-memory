use std::fs::File;
use std::io::Write;
use std::path::Path;

use sbm_core::{ContentExtractor, Location, Section, Unreadable};
use sbm_extract::DocxExtractor;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

/// Writes a .docx whose body is the given `<w:p>` paragraph XML.
fn write_docx(path: &Path, paragraphs: &str) {
    let mut docx = zip::ZipWriter::new(File::create(path).unwrap());
    docx.start_file("word/document.xml", SimpleFileOptions::default())
        .unwrap();
    write!(
        docx,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{paragraphs}</w:body></w:document>"#
    )
    .unwrap();
    docx.finish().unwrap();
}

#[test]
fn docx_paragraphs_are_extracted_one_per_line() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("minutes.docx");
    write_docx(
        &path,
        "<w:p><w:r><w:t>Meeting notes</w:t></w:r></w:p>\
         <w:p><w:r><w:t xml:space=\"preserve\">Budget approved </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>for Q3</w:t></w:r></w:p>",
    );

    let sections = DocxExtractor.extract(&path).unwrap();

    assert_eq!(
        sections,
        [Section {
            location: Location::Lines { start: 1, end: 2 },
            text: "Meeting notes\nBudget approved for Q3\n".into(),
        }]
    );
}

#[test]
fn docx_text_keeps_escaped_characters() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("contract.docx");
    write_docx(
        &path,
        "<w:p><w:r><w:t>Smith &amp; Sons &lt;draft&gt; &#169; 2024</w:t></w:r></w:p>",
    );

    let sections = DocxExtractor.extract(&path).unwrap();

    assert_eq!(sections[0].text, "Smith & Sons <draft> \u{a9} 2024\n");
}

#[test]
fn docx_that_decompresses_beyond_the_size_limit_is_refused_as_too_large() {
    // A zip bomb: a few hundred kilobytes on disk, 60 MB of XML once decompressed.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bomb.docx");
    let mut docx = zip::ZipWriter::new(File::create(&path).unwrap());
    docx.start_file("word/document.xml", SimpleFileOptions::default())
        .unwrap();
    docx.write_all(b"<w:document><w:body><w:p><w:r><w:t>")
        .unwrap();
    let padding = vec![b'a'; 1 << 20];
    for _ in 0..60 {
        docx.write_all(&padding).unwrap();
    }
    docx.write_all(b"</w:t></w:r></w:p></w:body></w:document>")
        .unwrap();
    docx.finish().unwrap();

    let error = DocxExtractor.extract(&path).unwrap_err();

    assert!(error.to_string().contains("too large"), "{error}");
    assert_eq!(Unreadable::of(error.as_ref()), Unreadable::TooLarge);
}
