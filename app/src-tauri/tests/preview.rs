//! Text previews: the lines around a Content Match, read from the file itself.

use std::path::{Path, PathBuf};

use sbm_app::{TextPreview, text_preview};

/// A file whose line `n` reads `line n`, for `count` lines.
fn numbered_file(folder: &Path, count: u32) -> PathBuf {
    let path = folder.join("retry.ts");
    let text: Vec<String> = (1..=count).map(|n| format!("line {n}")).collect();
    std::fs::write(&path, text.join("\n")).unwrap();
    path
}

fn lines(range: std::ops::RangeInclusive<u32>) -> Vec<String> {
    range.map(|n| format!("line {n}")).collect()
}

#[test]
fn preview_holds_the_matched_lines_with_three_lines_around_them() {
    let folder = tempfile::tempdir().unwrap();
    let path = numbered_file(folder.path(), 100);

    let preview = text_preview(&path, Some((20, 22)));

    assert_eq!(
        preview,
        Some(TextPreview {
            first_line: 17,
            lines: lines(17..=25),
        })
    );
}
