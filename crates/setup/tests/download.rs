//! Downloads for first-run setup: resumable and verified.

mod common;

use std::sync::atomic::AtomicBool;

use common::FakeHttp;
use sbm_setup::{Progress, download};
use tempfile::TempDir;

#[test]
fn files_are_downloaded_into_the_folder_and_progress_reaches_the_total() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let files = [
        http.serve("https://example.test/config.json", "config.json", b"{}"),
        http.serve(
            "https://example.test/model.onnx",
            "onnx/model.onnx",
            &[7; 5000],
        ),
    ];
    let mut last = Progress::default();

    download(
        &files,
        dir.path(),
        &http,
        &AtomicBool::new(false),
        &mut |p| last = p,
    )
    .unwrap();

    assert_eq!(
        std::fs::read(dir.path().join("config.json")).unwrap(),
        b"{}"
    );
    assert_eq!(
        std::fs::read(dir.path().join("onnx/model.onnx")).unwrap(),
        [7; 5000]
    );
    assert_eq!(
        last,
        Progress {
            done: 5002,
            total: 5002
        }
    );
}
