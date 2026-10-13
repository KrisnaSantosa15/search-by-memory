//! The USN Journal of the volume holding the temp directory (NTFS on Windows).
#![cfg(windows)]

use std::fs;

use sbm_change_usn::UsnJournal;
use sbm_core::{ChangeCursor, ChangeSource, Changes, SystemVolumes, Volume, Volumes};
use tempfile::TempDir;

/// The temp directory's volume, and a cursor at the end of its journal.
fn volume_and_cursor(folder: &TempDir) -> (Volume, ChangeCursor) {
    let volume = SystemVolumes.volume_of(folder.path()).unwrap();
    match UsnJournal.changes_since(&volume.id, None).unwrap() {
        Changes::Lost { next } => (volume, next),
        other => panic!("expected Lost without a cursor, got {other:?}"),
    }
}

/// `path` relative to the root of `volume`, with forward slashes.
fn relative(volume: &Volume, path: &std::path::Path) -> String {
    let path = fs::canonicalize(path).unwrap();
    let root = fs::canonicalize(&volume.root).unwrap();
    let parts: Vec<_> = path
        .strip_prefix(root)
        .unwrap()
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.join("/")
}

fn changed_paths(volume: &Volume, since: &ChangeCursor) -> Vec<String> {
    match UsnJournal.changes_since(&volume.id, Some(since)).unwrap() {
        Changes::Paths { paths, .. } => paths,
        other => panic!("expected Paths, got {other:?}"),
    }
}

#[test]
fn file_created_after_the_cursor_is_reported_by_its_path() {
    let folder = TempDir::new().unwrap();
    let (volume, cursor) = volume_and_cursor(&folder);

    let created = folder.path().join("created.txt");
    fs::write(&created, "new file").unwrap();

    assert!(changed_paths(&volume, &cursor).contains(&relative(&volume, &created)));
}

#[test]
fn deleted_file_is_reported_by_its_folder() {
    let folder = TempDir::new().unwrap();
    let notes = folder.path().join("notes");
    fs::create_dir(&notes).unwrap();
    let doomed = notes.join("doomed.txt");
    fs::write(&doomed, "short-lived").unwrap();
    let (volume, cursor) = volume_and_cursor(&folder);

    fs::remove_file(&doomed).unwrap();

    let paths = changed_paths(&volume, &cursor);
    assert!(
        paths.contains(&relative(&volume, &notes)),
        "{paths:?} {}",
        relative(&volume, &notes)
    );
}
