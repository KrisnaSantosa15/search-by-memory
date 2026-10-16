//! Launcher: Results as rows the window shows to people.

use std::path::PathBuf;

use sbm_app::{FileKind, ResultRow, not_read_row, result_row};
use sbm_core::{Location, Unreadable};
use sbm_index::{MatchReason, NotRead, SearchResult};
use serde_json::json;

fn result(location: Option<Location>, match_reason: MatchReason) -> SearchResult {
    SearchResult {
        path: PathBuf::from(r"D:\Work\Q3 Report.pdf"),
        copies: vec![PathBuf::from(r"E:\Backup\Q3 Report.pdf")],
        connected: true,
        snippet: location
            .as_ref()
            .map(|_| "net profit increased by 18%".to_string()),
        location,
        match_reason,
        weaker: false,
    }
}

#[test]
fn why_content_is_not_searched_is_sent_as_a_kind_and_for_failures_a_reason() {
    let sent = |not_read| serde_json::to_value(not_read_row(not_read)).unwrap();

    assert_eq!(sent(NotRead::OnlineOnly), json!({ "kind": "onlineOnly" }));
    assert_eq!(sent(NotRead::Format), json!({ "kind": "format" }));
    assert_eq!(sent(NotRead::Pending), json!({ "kind": "pending" }));
    assert_eq!(
        sent(NotRead::Failed(Unreadable::PasswordProtected)),
        json!({ "kind": "failed", "reason": "passwordProtected" })
    );
}

#[test]
fn content_match_row_names_the_file_its_page_and_why_it_matched() {
    let row = result_row(result(Some(Location::Page(14)), MatchReason::Both));

    assert_eq!(
        row,
        ResultRow {
            name: "Q3 Report.pdf".into(),
            path: PathBuf::from(r"D:\Work\Q3 Report.pdf"),
            folder: PathBuf::from(r"D:\Work"),
            kind: FileKind::Document,
            copies: vec![PathBuf::from(r"E:\Backup\Q3 Report.pdf")],
            connected: true,
            size: None,
            modified: None,
            snippet: Some("net profit increased by 18%".into()),
            location: Some("Page 14".into()),
            page: Some(14),
            line_start: None,
            line_end: None,
            match_reason: "both".into(),
            weaker: false,
        }
    );
}

#[test]
fn locations_read_as_lines_and_times() {
    let label = |location| result_row(result(Some(location), MatchReason::Content)).location;

    assert_eq!(
        label(Location::Lines { start: 3, end: 8 }).as_deref(),
        Some("Lines 3–8")
    );
    assert_eq!(
        label(Location::Lines { start: 5, end: 5 }).as_deref(),
        Some("Line 5")
    );
    let time = Location::Time {
        start: std::time::Duration::from_secs(62),
        end: std::time::Duration::from_secs(70),
    };
    assert_eq!(label(time).as_deref(), Some("1:02"));
    assert_eq!(label(Location::WholeFile), None);
}

#[test]
fn row_says_where_the_file_is_how_big_and_when_it_changed() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("notes.md");
    std::fs::write(&path, "twelve bytes").unwrap();
    let changed = std::fs::metadata(&path).unwrap().modified().unwrap();
    let found = SearchResult {
        path: path.clone(),
        ..result(None, MatchReason::Name)
    };

    let row = result_row(found);

    assert_eq!(row.folder, folder.path());
    assert_eq!(row.size, Some(12));
    let millis = changed
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert_eq!(row.modified, Some(millis));
}

#[test]
fn row_for_a_file_that_cannot_be_read_has_no_size_or_date() {
    let row = result_row(result(None, MatchReason::Name));

    assert_eq!(row.folder, PathBuf::from(r"D:\Work"));
    assert_eq!((row.size, row.modified), (None, None));
}

#[test]
fn name_match_row_has_no_location() {
    let row = result_row(result(None, MatchReason::Name));

    assert_eq!((row.location, row.match_reason.as_str()), (None, "name"));
}

#[test]
fn row_says_what_kind_of_file_it_is() {
    let kind = |name: &str| {
        result_row(SearchResult {
            path: PathBuf::from(format!(r"D:\Files\{name}")),
            ..result(None, MatchReason::Name)
        })
        .kind
    };

    assert_eq!(kind("Lease.PDF"), FileKind::Document);
    assert_eq!(kind("notes.md"), FileKind::Document);
    assert_eq!(kind("retry.ts"), FileKind::Code);
    assert_eq!(kind("holiday.jpg"), FileKind::Image);
    assert_eq!(kind("barking.m4a"), FileKind::Audio);
    assert_eq!(kind("wedding.mp4"), FileKind::Video);
    assert_eq!(kind("backup.zip"), FileKind::Other);
    assert_eq!(kind("Makefile"), FileKind::Other);
}

#[test]
fn row_carries_whether_it_is_a_weaker_match() {
    let row = result_row(SearchResult {
        weaker: true,
        ..result(None, MatchReason::Content)
    });

    assert!(row.weaker);
}

#[test]
fn row_carries_the_matched_line_numbers() {
    let lines = result_row(result(
        Some(Location::Lines { start: 8, end: 47 }),
        MatchReason::Content,
    ));
    let page = result_row(result(Some(Location::Page(4)), MatchReason::Content));

    assert_eq!((lines.line_start, lines.line_end), (Some(8), Some(47)));
    assert_eq!((page.line_start, page.line_end), (None, None));
}

#[test]
fn row_carries_the_matched_page_number() {
    let page = result_row(result(Some(Location::Page(4)), MatchReason::Content));
    let lines = result_row(result(
        Some(Location::Lines { start: 1, end: 2 }),
        MatchReason::Content,
    ));

    assert_eq!((page.page, lines.page), (Some(4), None));
}
