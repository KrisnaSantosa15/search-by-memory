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

#[test]
fn long_match_is_cut_to_forty_lines_from_its_start() {
    let folder = tempfile::tempdir().unwrap();
    let path = numbered_file(folder.path(), 500);

    let preview = text_preview(&path, Some((100, 300))).unwrap();

    assert_eq!(preview.first_line, 97);
    assert_eq!(preview.lines, lines(97..=136));
}

#[test]
fn file_found_by_name_previews_its_first_lines() {
    let folder = tempfile::tempdir().unwrap();
    let path = numbered_file(folder.path(), 100);

    let preview = text_preview(&path, None).unwrap();

    assert_eq!(preview.first_line, 1);
    assert_eq!(preview.lines, lines(1..=40));
}

#[test]
fn missing_file_has_no_preview() {
    let folder = tempfile::tempdir().unwrap();

    assert_eq!(text_preview(&folder.path().join("gone.ts"), None), None);
}
