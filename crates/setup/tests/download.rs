//! Downloads for first-run setup: resumable and verified.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};

use common::FakeHttp;
use sbm_setup::{Finished, Progress, download};
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

#[test]
fn corrupt_download_is_rejected_and_started_over_next_time() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let mut file = http.serve("https://example.test/model.onnx", "model.onnx", b"tampered");
    file.sha256 = common::sha256(b"original");

    let error = download(
        &[file],
        dir.path(),
        &http,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap_err();

    assert!(error.to_string().contains("model.onnx"), "{error}");
    let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn files_already_downloaded_are_not_fetched_again() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let files = [http.serve("https://example.test/model.onnx", "model.onnx", &[7; 5000])];
    let cancel = AtomicBool::new(false);
    download(&files, dir.path(), &http, &cancel, &mut |_| {}).unwrap();
    http.requests.lock().unwrap().clear();
    let mut last = Progress::default();

    download(&files, dir.path(), &http, &cancel, &mut |p| last = p).unwrap();

    assert!(http.requests.lock().unwrap().is_empty());
    assert_eq!(
        last,
        Progress {
            done: 5000,
            total: 5000
        }
    );
}

#[test]
fn cancelled_download_stops_and_resumes_later() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let content = vec![7; 300_000];
    let files = [http.serve("https://example.test/model.onnx", "model.onnx", &content)];
    let cancel = AtomicBool::new(false);

    let first = download(&files, dir.path(), &http, &cancel, &mut |_| {
        cancel.store(true, Ordering::SeqCst)
    })
    .unwrap();
    cancel.store(false, Ordering::SeqCst);
    let second = download(&files, dir.path(), &http, &cancel, &mut |_| {}).unwrap();

    assert_eq!((first, second), (Finished::Cancelled, Finished::Complete));
    assert_eq!(
        std::fs::read(dir.path().join("model.onnx")).unwrap(),
        content
    );
    assert!(http.requests.lock().unwrap().last().unwrap().1 > 0);
}
