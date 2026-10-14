use std::fs;

use sbm_core::{ContentExtractor, Location, Section, Unreadable};
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
fn text_file_beyond_the_size_limit_is_refused_as_too_large() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("huge.log");
    fs::File::create(&path)
        .unwrap()
        .set_len(51 * 1024 * 1024)
        .unwrap();

    let error = TextExtractor.extract(&path).unwrap_err();

    assert!(error.to_string().contains("too large"), "{error}");
    assert_eq!(Unreadable::of(error.as_ref()), Unreadable::TooLarge);
}

#[test]
fn text_file_that_is_not_utf8_is_still_extracted() {
    // Latin-1 "café au lait", as saved by older Windows editors.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("menu.txt");
    fs::write(&path, b"caf\xe9 au lait\n").unwrap();

    let sections = TextExtractor.extract(&path).unwrap();

    assert!(
        sections[0].text.contains("au lait"),
        "{:?}",
        sections[0].text
    );
}

#[cfg(windows)]
#[test]
fn text_file_another_program_has_open_is_reported_as_in_use() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("open-elsewhere.txt");
    fs::write(&path, "draft budget\n").unwrap();
    // Opened with no sharing, as some programs hold the files they edit.
    let _held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    let error = TextExtractor.extract(&path).unwrap_err();

    assert_eq!(Unreadable::of(error.as_ref()), Unreadable::InUse);
}
