use std::path::Path;

use sbm_extract::extractor_for;

#[test]
fn supported_formats_have_an_extractor() {
    for file in [
        "report.pdf",
        "minutes.docx",
        "notes.txt",
        "README.md",
        "main.rs",
        "app.py",
        "config.yaml",
    ] {
        assert!(extractor_for(Path::new(file)).is_some(), "{file}");
    }
}

#[test]
fn unsupported_formats_have_no_extractor() {
    for file in ["photo.jpg", "archive.zip", "setup.exe", "Makefile"] {
        assert!(extractor_for(Path::new(file)).is_none(), "{file}");
    }
}

#[test]
fn office_lock_files_have_no_extractor() {
    assert!(extractor_for(Path::new("~$minutes.docx")).is_none());
}
