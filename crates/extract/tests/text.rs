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
