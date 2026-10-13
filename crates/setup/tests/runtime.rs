//! Installing a GPU runtime: archives downloaded, their DLLs unpacked.

mod common;

use std::io::Write;
use std::sync::atomic::AtomicBool;

use common::FakeHttp;
use sbm_setup::{Finished, install_runtime};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

/// A zip archive holding `files` as (path, content).
fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (path, content) in files {
        zip.start_file(*path, SimpleFileOptions::default()).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn dlls_in_the_downloaded_archives_are_unpacked_into_the_runtime_folder() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let archives = [
        http.serve(
            "https://example.test/cudart.zip",
            "cudart.zip",
            &archive(&[
                ("cudart/bin/x64/cudart64_13.dll", b"cudart"),
                ("cudart/include/cuda.h", b"header"),
                ("cudart/LICENSE", b"license"),
            ]),
        ),
        http.serve(
            "https://example.test/cudnn.zip",
            "cudnn.zip",
            &archive(&[("cudnn/bin/x64/cudnn64_9.dll", b"cudnn")]),
        ),
    ];

    let finished = install_runtime(
        &archives,
        dir.path(),
        &http,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(finished, Finished::Complete);
    assert_eq!(names_in(dir.path()), ["cudart64_13.dll", "cudnn64_9.dll"]);
    assert_eq!(
        std::fs::read(dir.path().join("cudnn64_9.dll")).unwrap(),
        b"cudnn"
    );
}
