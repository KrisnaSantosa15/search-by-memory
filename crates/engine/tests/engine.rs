//! The Engine: indexes in the background and answers Queries meanwhile.

mod common;

use std::sync::Arc;

use common::{BagOfWords, BrokenJournal, NoJournal, write};
use sbm_engine::Engine;
use sbm_index::View;
use tempfile::TempDir;

#[test]
fn file_in_an_added_indexed_folder_becomes_searchable() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let results = engine.search("beef coconut", View::Passage, 20).unwrap();
    assert_eq!(results[0].path, recipe);
}

#[test]
fn status_counts_files_in_the_name_index_and_pending_content_work() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "a.txt", "first\n");
    write(folder.path(), "b.txt", "second\n");
    write(folder.path(), "photo.jpg", "not text\n");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let status = engine.status().unwrap();
    assert_eq!(status.files, 3);
    assert_eq!(status.pending, 0);
}

#[test]
fn indexing_error_is_reported_in_the_status() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(BrokenJournal)).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let error = engine.status().unwrap().error;
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("change journal cannot be read")),
        "{error:?}"
    );
}

#[test]
fn paused_engine_keeps_names_current_and_indexes_content_once_resumed() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.pause();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();
    let paused = engine.status().unwrap();
    engine.resume();
    engine.wait_until_idle();

    assert_eq!((paused.files, paused.pending), (1, 1));
    let results = engine.search("beef coconut", View::Passage, 20).unwrap();
    assert_eq!(results[0].path, recipe);
}
