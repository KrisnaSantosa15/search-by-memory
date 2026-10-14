//! Installing a GPU runtime: archives downloaded, their DLLs unpacked.

mod common;

use std::io::Write;
use std::sync::atomic::AtomicBool;

use common::{FakeHttp, tgz};
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

/// Names of the files in `dir`, without the hidden ones setup keeps for itself.
fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
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

#[test]
fn installed_runtime_is_not_downloaded_again() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let archives = [http.serve(
        "https://example.test/cudart.zip",
        "cudart.zip",
        &archive(&[("cudart/bin/x64/cudart64_13.dll", b"cudart")]),
    )];
    let cancel = AtomicBool::new(false);
    install_runtime(&archives, dir.path(), &http, &cancel, &mut |_| {}).unwrap();
    http.requests.lock().unwrap().clear();

    let finished = install_runtime(&archives, dir.path(), &http, &cancel, &mut |_| {}).unwrap();

    assert_eq!(finished, Finished::Complete);
    assert!(http.requests.lock().unwrap().is_empty());
}

#[test]
fn cuda_runtime_is_downloaded_from_nvidia_over_https() {
    let archives = sbm_setup::cuda_runtime();

    assert!(!archives.is_empty());
    for archive in archives {
        assert!(
            archive
                .url
                .starts_with("https://developer.download.nvidia.com/compute/"),
            "{}",
            archive.url
        );
        assert!(archive.url.ends_with(&archive.path), "{}", archive.url);
    }
}

/// The archives the CUDA runtime on this development machine was installed
/// from, kept in `%LOCALAPPDATA%\nvidia-cuda13\zips`; the test is skipped without them.
#[test]
fn cuda_runtime_manifest_matches_the_archives_that_passed_the_parity_gate() {
    let Some(zips) = std::env::var_os("LOCALAPPDATA")
        .map(|local| std::path::PathBuf::from(local).join("nvidia-cuda13/zips"))
        .filter(|zips| zips.exists())
    else {
        eprintln!("SKIPPED: CUDA runtime archives not found");
        return;
    };

    for archive in sbm_setup::cuda_runtime() {
        let path = zips.join(&archive.path);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            archive.size,
            "{}",
            archive.path
        );
        if archive.size < 50 * 1024 * 1024 {
            let content = std::fs::read(&path).unwrap();
            assert_eq!(common::sha256(&content), archive.sha256, "{}", archive.path);
        }
    }
}

#[test]
fn tgz_runtime_unpacks_its_dlls_and_keeps_its_licenses() {
    let dir = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let archives = [http.serve(
        "https://example.test/pdfium-win-x64.tgz",
        "pdfium-win-x64.tgz",
        &tgz(&[
            ("bin/pdfium.dll", b"pdfium"),
            ("include/fpdfview.h", b"header"),
            ("licenses/pdfium.txt", b"BSD"),
        ]),
    )];

    let finished = install_runtime(
        &archives,
        dir.path(),
        &http,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(finished, Finished::Complete);
    assert_eq!(names_in(dir.path()), ["licenses", "pdfium.dll"]);
    assert_eq!(
        std::fs::read(dir.path().join("licenses/pdfium.txt")).unwrap(),
        b"BSD"
    );
}
