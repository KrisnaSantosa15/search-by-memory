//! Live Updates from a ChangeSource (FR-04).

mod common;

use common::{BagOfWords, FakeChangeSource, FakeVolumes, modified_days_ago, store, write};
use sbm_core::VectorStore;
use sbm_index::{Exclusion, Index, View};
use tempfile::TempDir;

/// An index whose Indexed Folder is the whole of volume "d-drive", with its
/// change cursor saved.
fn indexed_drive(data: &TempDir, drive: &TempDir, journal: &FakeChangeSource) -> Index {
    let volumes = FakeVolumes::default();
    volumes.mount("d-drive", drive.path());
    let mut index = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes);
    index.add_indexed_folder(drive.path()).unwrap();
    index.apply_changes(journal).unwrap();
    index
}

#[test]
fn file_created_after_the_cursor_becomes_content_searchable() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);

    let recipe = write(
        drive.path(),
        "kitchen/recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    journal.record("d-drive", "kitchen/recipe.txt");
    let mut store = store();
    index.apply_changes(&journal).unwrap();
    index
        .run_jobs(&BagOfWords::default(), &mut store, usize::MAX)
        .unwrap();

    let results = index
        .search(
            "beef coconut",
            &BagOfWords::default(),
            &store,
            View::Passage,
            20,
        )
        .unwrap();
    assert_eq!(results[0].path, recipe);
}

#[test]
fn intact_change_history_rechecks_only_the_reported_paths() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);

    let reported = write(drive.path(), "notes/reported.txt", "reported\n");
    write(drive.path(), "notes/unreported.txt", "unreported\n");
    journal.record("d-drive", "notes/reported.txt");
    let changes = index.apply_changes(&journal).unwrap();

    assert_eq!(changes.added, vec![reported]);
}

#[test]
fn deleted_file_leaves_the_name_index_and_the_content_index() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        drive.path(),
        "kitchen/rendang.txt",
        "slow cooked beef in coconut milk
",
    );
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);
    let mut store = store();
    index
        .run_jobs(&BagOfWords::default(), &mut store, usize::MAX)
        .unwrap();

    std::fs::remove_file(&recipe).unwrap();
    journal.record("d-drive", "kitchen/rendang.txt");
    let changes = index.apply_changes(&journal).unwrap();
    index
        .run_jobs(&BagOfWords::default(), &mut store, usize::MAX)
        .unwrap();

    assert_eq!(changes.removed, vec![recipe]);
    assert!(index.search_names("rendang", 20).unwrap().is_empty());
    let results = index
        .search(
            "beef coconut",
            &BagOfWords::default(),
            &store,
            View::Passage,
            20,
        )
        .unwrap();
    assert!(results.is_empty(), "{results:?}");
    assert_eq!(store.len(), 0);
}

#[test]
fn renamed_folder_moves_its_files_without_reembedding() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        drive.path(),
        "photos/2024/beach-notes.txt",
        "sunset over the beach\n",
    );
    write(
        drive.path(),
        "photos/mountain-notes.txt",
        "snow on the mountain\n",
    );
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);
    let (embedder, mut store) = (BagOfWords::default(), store());
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();
    let embedded = embedder.embedded();

    std::fs::rename(drive.path().join("photos"), drive.path().join("pictures")).unwrap();
    journal.record("d-drive", "photos");
    journal.record("d-drive", "pictures");
    index.apply_changes(&journal).unwrap();
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    assert_eq!(embedder.embedded(), embedded);
    let results = index
        .search("sunset beach", &embedder, &store, View::Passage, 20)
        .unwrap();
    assert_eq!(
        results[0].path,
        drive.path().join("pictures/2024/beach-notes.txt")
    );
    let names = index.search_names("notes", 20).unwrap();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        names
            .iter()
            .all(|name| name.path.starts_with(drive.path().join("pictures")))
    );
}

#[test]
fn live_updates_are_indexed_before_remaining_initial_indexing_work() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        drive.path(),
        "inbox/today.txt",
        "agenda for the team meeting
",
    );
    write(
        drive.path(),
        "inbox/yesterday.txt",
        "notes from the team meeting
",
    );
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);

    // Restored from a backup: changed now, but modified long ago.
    let restored = write(
        drive.path(),
        "restored/minutes.txt",
        "minutes of the team meeting
",
    );
    modified_days_ago(&restored, 400);
    journal.record("d-drive", "restored");
    index.apply_changes(&journal).unwrap();
    let mut store = store();
    index
        .run_jobs(&BagOfWords::default(), &mut store, 1)
        .unwrap();

    let results = index
        .search(
            "team meeting",
            &BagOfWords::default(),
            &store,
            View::Passage,
            20,
        )
        .unwrap();
    let paths: Vec<_> = results.iter().map(|r| r.path.clone()).collect();
    assert_eq!(paths, [restored]);
}

#[test]
fn changes_outside_indexed_folders_or_inside_exclusions_are_ignored() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let volumes = FakeVolumes::default();
    volumes.mount("d-drive", drive.path());
    let journal = FakeChangeSource::default();
    let documents = drive.path().join("documents");
    std::fs::create_dir(&documents).unwrap();
    let mut index = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes);
    index.add_indexed_folder(&documents).unwrap();
    index
        .add_exclusion(Exclusion::Name("*.tmp".to_string()))
        .unwrap();
    index.apply_changes(&journal).unwrap();

    let report = write(drive.path(), "documents/report.txt", "quarterly report\n");
    write(drive.path(), "downloads/setup.txt", "installer notes\n");
    write(
        drive.path(),
        "documents/site/node_modules/left-pad/readme.txt",
        "pads strings\n",
    );
    write(drive.path(), "documents/draft.tmp", "scratch\n");
    for path in [
        "documents/report.txt",
        "downloads/setup.txt",
        "documents/site/node_modules/left-pad/readme.txt",
        "documents/draft.tmp",
    ] {
        journal.record("d-drive", path);
    }
    let changes = index.apply_changes(&journal).unwrap();

    assert_eq!(changes.added, vec![report]);
}

#[test]
fn lost_change_history_rescans_the_indexed_folders() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);

    let missed = write(
        drive.path(),
        "notes/missed.txt",
        "written while the journal wrapped
",
    );
    journal.lose_history("d-drive");
    let changes = index.apply_changes(&journal).unwrap();

    assert_eq!(changes.added, vec![missed]);
}

#[test]
fn saved_cursor_survives_reopening_the_index() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let journal = FakeChangeSource::default();
    drop(indexed_drive(&data, &drive, &journal));

    let reported = write(
        drive.path(),
        "notes/reported.txt",
        "reported
",
    );
    write(
        drive.path(),
        "notes/unreported.txt",
        "unreported
",
    );
    journal.record("d-drive", "notes/reported.txt");
    let volumes = FakeVolumes::default();
    volumes.mount("d-drive", drive.path());
    let mut reopened = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes);
    let changes = reopened.apply_changes(&journal).unwrap();

    assert_eq!(changes.added, vec![reported]);
}

#[test]
fn changes_on_a_disconnected_drive_are_applied_when_it_returns() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let kept = write(
        drive.path(),
        "notes/kept.txt",
        "kept while away
",
    );
    let volumes = FakeVolumes::default();
    volumes.mount("usb-stick", drive.path());
    let journal = FakeChangeSource::default();
    let mut index = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes.clone());
    index.add_indexed_folder(drive.path()).unwrap();
    index.apply_changes(&journal).unwrap();

    volumes.disconnect("usb-stick");
    // Changed on another computer, which records it in the drive's own journal.
    let added = write(
        drive.path(),
        "notes/added.txt",
        "added elsewhere
",
    );
    journal.record("usb-stick", "notes/added.txt");
    let while_away = index.apply_changes(&journal).unwrap();
    volumes.mount("usb-stick", drive.path());
    let on_return = index.apply_changes(&journal).unwrap();

    assert!(while_away.added.is_empty() && while_away.removed.is_empty());
    assert_eq!(on_return.added, vec![added]);
    assert!(on_return.removed.is_empty());
    assert_eq!(index.search_names("kept", 20).unwrap()[0].path, kept);
}

#[test]
fn indexed_folder_added_after_the_cursor_is_scanned() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(drive.path(), "documents/letter.txt", "dear sir\n");
    let photo_notes = write(drive.path(), "photos/notes.txt", "beach trip\n");
    let volumes = FakeVolumes::default();
    volumes.mount("d-drive", drive.path());
    let journal = FakeChangeSource::default();
    let mut index = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes);
    index
        .add_indexed_folder(&drive.path().join("documents"))
        .unwrap();
    index.apply_changes(&journal).unwrap();

    index
        .add_indexed_folder(&drive.path().join("photos"))
        .unwrap();
    let changes = index.apply_changes(&journal).unwrap();

    assert_eq!(changes.added, vec![photo_notes]);
}

#[test]
fn exclusion_added_after_the_cursor_drops_matching_files() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let draft = write(drive.path(), "documents/draft.tmp", "scratch\n");
    let journal = FakeChangeSource::default();
    let mut index = indexed_drive(&data, &drive, &journal);

    index
        .add_exclusion(Exclusion::Name("*.tmp".to_string()))
        .unwrap();
    let changes = index.apply_changes(&journal).unwrap();

    assert_eq!(changes.removed, vec![draft]);
}
