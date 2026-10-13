use std::fs;

use sbm_core::{ContentExtractor, Location, Section};
use sbm_extract::TextExtractor;
use tempfile::TempDir;

#[test]
fn text_file_is_extracted_with_its_line_range() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("notes.md");
    fs::write(&path, "# Groceries\n\n- rice\n- eggs\n").unwrap();

    let sections = TextExtractor.extract(&path).unwrap();

    assert_eq!(
        sections,
        [Section {
            location: Location::Lines { start: 1, end: 4 },
            text: "# Groceries\n\n- rice\n- eggs\n".into(),
        }]
    );
}

#[test]
fn empty_text_file_has_no_sections() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("empty.txt");
    fs::write(&path, "").unwrap();

    assert!(TextExtractor.extract(&path).unwrap().is_empty());
}

#[test]
fn text_file_beyond_the_size_limit_is_refused() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("huge.log");
    fs::File::create(&path)
        .unwrap()
        .set_len(51 * 1024 * 1024)
        .unwrap();

    let error = TextExtractor.extract(&path).unwrap_err();

    assert!(error.to_string().contains("too large"), "{error}");
}
