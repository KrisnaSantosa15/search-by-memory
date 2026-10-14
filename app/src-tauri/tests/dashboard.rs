//! The Status tab: what is indexed, and how far content search reaches (FR-08).

use std::time::{Duration, UNIX_EPOCH};

use sbm_app::{Dashboard, dashboard};
use sbm_engine::Status;

#[test]
fn dashboard_reports_files_found_and_how_far_content_search_reaches() {
    let status = Status {
        files: 12_345,
        indexed: 340,
        pending: 660,
        ready_since: Some(UNIX_EPOCH + Duration::from_millis(1_740_787_200_000)),
        ..Status::default()
    };

    assert_eq!(
        dashboard(&status),
        Dashboard {
            files: 12_345,
            indexed: 340,
            pending: 660,
            ready_since: Some(1_740_787_200_000),
        }
    );
}
