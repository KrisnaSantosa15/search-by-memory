//! Indexing through saved Indexed Folders and the job queue (FR-04).

mod common;

use common::{BagOfWords, store, write};
use sbm_index::{Index, View};
use tempfile::TempDir;

#[test]
fn indexed_folder_is_remembered_and_its_files_become_content_searchable() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "kitchen/recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let database = data.path().join("index.sqlite");
    Index::open(&database)
        .unwrap()
        .add_indexed_folder(folder.path())
        .unwrap();

    let mut index = Index::open(&database).unwrap();
    let mut store = store();
    index.scan_all().unwrap();
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

/// Sets the modification time of `path` to `days` days ago.
fn modified_days_ago(path: &std::path::Path, days: u64) {
    let time = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

#[test]
fn recently_modified_files_are_indexed_before_older_ones() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let old = write(
        folder.path(),
        "a-old.txt",
        "minutes of the annual meeting\n",
    );
    let new = write(folder.path(), "b-new.txt", "draft of the annual meeting\n");
    modified_days_ago(&old, 400);
    modified_days_ago(&new, 1);
    let mut index = Index::open(&data.path().join("index.sqlite")).unwrap();
    let mut store = store();
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();

    index
        .run_jobs(&BagOfWords::default(), &mut store, 1)
        .unwrap();

    let results = index
        .search(
            "annual meeting",
            &BagOfWords::default(),
            &store,
            View::Passage,
            20,
        )
        .unwrap();
    let paths: Vec<_> = results.iter().map(|r| r.path.clone()).collect();
    assert_eq!(paths, [new]);
}

#[test]
fn interrupted_indexing_resumes_without_embedding_finished_files_again() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "a.txt", "first letter to the landlord\n");
    write(folder.path(), "b.txt", "second letter to the landlord\n");
    let database = data.path().join("index.sqlite");
    let embedder = BagOfWords::default();
    let mut store = store();
    let mut index = Index::open(&database).unwrap();
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();
    index.run_jobs(&embedder, &mut store, 1).unwrap();
    drop(index);

    let mut index = Index::open(&database).unwrap();
    index.scan_all().unwrap();
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    assert_eq!(
        embedder.embedded(),
        2,
        "each file's single Chunk is embedded once"
    );
    let results = index
        .search("letter landlord", &embedder, &store, View::Passage, 20)
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn rescan_queues_only_files_that_changed() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "kept.txt", "unchanged notes\n");
    let edited = write(folder.path(), "edited.txt", "first draft\n");
    let mut index = Index::open(&data.path().join("index.sqlite")).unwrap();
    let mut store = store();
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();
    assert_eq!(index.pending_jobs().unwrap(), 2);
    index
        .run_jobs(&BagOfWords::default(), &mut store, usize::MAX)
        .unwrap();

    std::fs::write(&edited, "second draft, much longer than the first\n").unwrap();
    index.scan_all().unwrap();

    assert_eq!(index.pending_jobs().unwrap(), 1);
}

#[test]
fn file_that_cannot_be_indexed_is_reported_and_does_not_block_others() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let good = write(folder.path(), "good.txt", "notes about the garden fence\n");
    let broken = write(folder.path(), "broken.docx", "not really a zip archive");
    modified_days_ago(&good, 10);
    let mut index = Index::open(&data.path().join("index.sqlite")).unwrap();
    let mut store = store();
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();

    index
        .run_jobs(&BagOfWords::default(), &mut store, usize::MAX)
        .unwrap();

    assert_eq!(index.pending_jobs().unwrap(), 0);
    let failed = index.failed_files().unwrap();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].0, broken);
    assert!(!failed[0].1.is_empty());
    let results = index
        .search(
            "garden fence",
            &BagOfWords::default(),
            &store,
            View::Passage,
            20,
        )
        .unwrap();
    assert_eq!(results[0].path, good);
}
