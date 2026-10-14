//! The Status tab: what is indexed, and how far content search reaches (FR-08).

use std::time::{Duration, UNIX_EPOCH};

use sbm_app::dashboard;
use sbm_core::ThrottleReason;
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

    let dashboard = dashboard(&status);
    assert_eq!(
        (
            dashboard.files,
            dashboard.indexed,
            dashboard.pending,
            dashboard.ready_since
        ),
        (12_345, 340, 660, Some(1_740_787_200_000))
    );
}

#[test]
fn dashboard_says_why_indexing_waits_and_whether_it_is_paused() {
    let held = Status {
        pending: 10,
        throttled: Some(ThrottleReason::Battery),
        ..Status::default()
    };
    let paused = Status {
        pending: 10,
        paused: true,
        ..Status::default()
    };

    assert_eq!(
        dashboard(&held).state,
        "Indexing waits until the computer is plugged in"
    );
    assert!(!dashboard(&held).paused);
    assert_eq!(dashboard(&paused).state, "Indexing is paused");
    assert!(dashboard(&paused).paused);
}

#[test]
fn dashboard_reports_how_much_disk_the_indexes_take() {
    let status = Status {
        size_on_disk: 1_234_567_890,
        ..Status::default()
    };

    assert_eq!(dashboard(&status).size_on_disk, 1_234_567_890);
}
