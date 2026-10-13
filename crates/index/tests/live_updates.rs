//! Live Updates from a ChangeSource (FR-04).

mod common;

use common::{BagOfWords, FakeChangeSource, FakeVolumes, store, write};
use sbm_index::{Index, View};
use tempfile::TempDir;

#[test]
fn file_created_after_the_cursor_becomes_content_searchable() {
    let (data, drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let volumes = FakeVolumes::default();
    volumes.mount("d-drive", drive.path());
    let journal = FakeChangeSource::default();
    let mut index = Index::open(&data.path().join("index.sqlite"))
        .unwrap()
        .with_volumes(volumes);
    index.add_indexed_folder(drive.path()).unwrap();
    index.apply_changes(&journal).unwrap();

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
