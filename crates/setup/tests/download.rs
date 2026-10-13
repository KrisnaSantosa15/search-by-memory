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

#[test]
fn interrupted_download_resumes_where_it_stopped() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let content: Vec<u8> = (0..10_000u32).map(|n| n as u8).collect();
    let files = [http.serve("https://example.test/model.onnx", "model.onnx", &content)];
    http.drop_next_connection_after(4000);
    let cancel = AtomicBool::new(false);
    assert!(download(&files, dir.path(), &http, &cancel, &mut |_| {}).is_err());

    download(&files, dir.path(), &http, &cancel, &mut |_| {}).unwrap();

    assert_eq!(
        std::fs::read(dir.path().join("model.onnx")).unwrap(),
        content
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.last().unwrap().1, 4000);
}
