//! The Engine: indexes in the background and answers Queries meanwhile.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{BagOfWords, BrokenJournal, NoJournal, write};
use sbm_engine::Engine;
use sbm_index::{Exclusion, View};
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

#[test]
fn file_created_after_indexing_becomes_searchable_without_asking() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().files == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    engine.wait_until_idle();
    let results = engine.search("beef coconut", View::Passage, 20).unwrap();
    assert_eq!(results.first().map(|r| &r.path), Some(&recipe));
}

#[test]
fn file_that_cannot_be_indexed_is_listed_with_the_reason() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "good.txt", "notes about the garden fence\n");
    let broken = write(folder.path(), "broken.docx", "not really a zip archive");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let failed = engine.failed_files().unwrap();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].0, broken);
    assert!(!failed[0].1.is_empty());
    assert_eq!(engine.status().unwrap().failed, 1);
}

#[test]
fn removed_indexed_folder_leaves_search_and_status() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    engine.remove_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    assert_eq!(engine.search("beef coconut", View::Passage, 20).unwrap(), []);
    assert_eq!(engine.search("recipe", View::File, 20).unwrap(), []);
    assert_eq!(engine.status().unwrap().files, 0);
}

#[test]
fn files_beneath_a_new_exclusion_leave_the_indexes() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "notes.txt", "shopping list\n");
    let diary = write(folder.path(), "private/diary.txt", "secret garden plans\n");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    engine
        .add_exclusion(Exclusion::Folder(folder.path().join("private")))
        .unwrap();
    engine.wait_until_idle();

    let results = engine.search("secret garden diary", View::File, 20).unwrap();
    assert!(results.iter().all(|r| r.path != diary), "{results:?}");
    assert_eq!(engine.status().unwrap().files, 1);
}
