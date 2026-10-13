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
