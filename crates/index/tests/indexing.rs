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
