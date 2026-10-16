//! The Status tab: what is indexed, and how far content search reaches (FR-08).

use std::time::{Duration, UNIX_EPOCH};

use jiff::Zoned;
use sbm_app::{ModelRow, dashboard};
use sbm_core::ThrottleReason;
use sbm_engine::{ModelStatus, Status};

#[test]
fn dashboard_reports_files_found_and_how_far_content_search_reaches() {
    let status = Status {
        files: 12_345,
        indexed: 340,
        pending: 660,
        ready_since: Some(UNIX_EPOCH + Duration::from_millis(1_740_787_200_000)),
        ..Status::default()
    };

    let dashboard = dashboard(&status, &Zoned::now());
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
        dashboard(&held, &Zoned::now()).state,
        "Indexing waits until the computer is plugged in"
    );
    assert!(!dashboard(&held, &Zoned::now()).paused);
    assert_eq!(
        dashboard(&paused, &Zoned::now()).state,
        "Indexing is paused"
    );
    assert!(dashboard(&paused, &Zoned::now()).paused);
}

#[test]
fn dashboard_reports_how_much_disk_the_indexes_take() {
    let status = Status {
        size_on_disk: 1_234_567_890,
        ..Status::default()
    };

    assert_eq!(
        dashboard(&status, &Zoned::now()).size_on_disk,
        1_234_567_890
    );
}

#[test]
fn dashboard_reports_whether_names_are_still_being_found_and_what_is_being_read() {
    let status = Status {
        scanning: true,
        current: Some(std::path::PathBuf::from(
            "C:/Users/ana/Documents/report.pdf",
        )),
        failed: 2,
        ..Status::default()
    };

    let dashboard = dashboard(&status, &Zoned::now());

    assert!(dashboard.scanning);
    assert_eq!(
        dashboard.current.as_deref(),
        Some(std::path::Path::new("C:/Users/ana/Documents/report.pdf"))
    );
    assert_eq!(dashboard.failed, 2);
}

#[test]
fn dashboard_says_whether_the_model_that_reads_files_is_loading_ready_or_failed() {
    let with_model = |model| Status {
        model,
        ..Status::default()
    };

    let loading = dashboard(&with_model(ModelStatus::Loading), &Zoned::now());
    let ready = dashboard(
        &with_model(ModelStatus::Ready {
            device: "cuda".into(),
        }),
        &Zoned::now(),
    );
    let failed = dashboard(
        &with_model(ModelStatus::Failed("model.onnx not found".into())),
        &Zoned::now(),
    );

    assert_eq!(loading.model, ModelRow::Loading);
    assert_eq!(
        ready.model,
        ModelRow::Ready {
            device: "cuda".into()
        }
    );
    assert_eq!(
        failed.model,
        ModelRow::Failed {
            reason: "model.onnx not found".into()
        }
    );
}
