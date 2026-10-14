//! The Status tab's files that could not be read, and why.

use std::path::PathBuf;

use sbm_app::{FailedRow, failed_row};
use sbm_core::Unreadable;
use sbm_index::FailedFile;

#[test]
fn failed_file_row_carries_the_reason_and_the_raw_message() {
    let row = failed_row(FailedFile {
        path: PathBuf::from("C:/Users/ana/Documents/salary.pdf"),
        reason: Unreadable::PasswordProtected,
        detail: "PDF is protected by a password".to_string(),
    });

    assert_eq!(
        row,
        FailedRow {
            path: PathBuf::from("C:/Users/ana/Documents/salary.pdf"),
            reason: "passwordProtected",
            detail: "PDF is protected by a password".to_string(),
        }
    );
}
