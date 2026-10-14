//! The lines around a Content Match, read from the file itself for the Preview Pane.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Serialize;

/// Lines shown before and after the matched ones.
const CONTEXT: u32 = 3;
/// Lines shown at most; a longer match is cut, keeping its start.
const MOST_LINES: u32 = 40;

/// Some lines of a text file, as the Preview Pane shows them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPreview {
    /// The 1-based number of `lines[0]`.
    pub first_line: u32,
    pub lines: Vec<String>,
}

/// The lines `matched` (inclusive, 1-based) of the text file `path`, with a
/// few lines around them, or its first lines without a match; `None` when the
/// file cannot be read.
pub fn text_preview(path: &Path, matched: Option<(u32, u32)>) -> Option<TextPreview> {
    let (start, end) = matched.unwrap_or((1, MOST_LINES));
    let first_line = start.saturating_sub(CONTEXT).max(1);
    let last_line = (end + CONTEXT).min(first_line + MOST_LINES - 1);
    let reader = BufReader::new(File::open(path).ok()?);
    let mut lines = Vec::new();
    for (number, line) in (1..).zip(reader.split(b'\n')) {
        if number > last_line {
            break;
        }
        if number >= first_line {
            let line = line.ok()?;
            lines.push(
                String::from_utf8_lossy(&line)
                    .trim_end_matches('\r')
                    .to_string(),
            );
        }
    }
    Some(TextPreview { first_line, lines })
}
