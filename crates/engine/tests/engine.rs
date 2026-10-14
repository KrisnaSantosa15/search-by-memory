//! The Engine: indexes in the background and answers Queries meanwhile.

mod common;

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use common::{
    BagOfWords, BrokenJournal, ChangingActivity, NoJournal, RecordingJournal, SteadyActivity, write,
};
use sbm_core::ThrottleReason;
use sbm_engine::{Engine, LoadedModel, ModelStatus};
use sbm_index::{Exclusion, MatchReason, View};
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

    assert_eq!(
        engine.search("beef coconut", View::Passage, 20).unwrap(),
        []
    );
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

    let results = engine
        .search("secret garden diary", View::File, 20)
        .unwrap();
    assert!(results.iter().all(|r| r.path != diary), "{results:?}");
    assert_eq!(engine.status().unwrap().files, 1);
}

#[test]
fn status_states_how_far_back_content_is_ready() {
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
    let resumed = engine.status().unwrap();

    let modified = std::fs::metadata(&recipe).unwrap().modified().unwrap();
    assert_eq!((paused.indexed, paused.ready_since), (0, Some(modified)));
    assert_eq!((resumed.indexed, resumed.ready_since), (1, None));
}

#[test]
fn files_are_found_by_name_while_the_model_loads() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let (release, gate) = mpsc::channel::<()>();
    let engine = Engine::start_loading(
        data.path(),
        move || {
            let _ = gate.recv();
            Ok(LoadedModel {
                embedder: Arc::new(BagOfWords),
                device: "test".into(),
            })
        },
        Box::new(NoJournal),
    )
    .unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let status = engine.status().unwrap();
    assert_eq!((status.model, status.pending), (ModelStatus::Loading, 1));
    let results = engine.search("recipe", View::File, 20).unwrap();
    assert_eq!(results.first().map(|r| &r.path), Some(&recipe));
    assert_eq!(results[0].match_reason, MatchReason::Name);
    drop(release);
}

#[test]
fn content_is_indexed_once_the_model_is_loaded() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let (release, gate) = mpsc::channel::<()>();
    let engine = Engine::start_loading(
        data.path(),
        move || {
            let _ = gate.recv();
            Ok(LoadedModel {
                embedder: Arc::new(BagOfWords),
                device: "graphics card".into(),
            })
        },
        Box::new(NoJournal),
    )
    .unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().model == ModelStatus::Loading && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    engine.wait_until_idle();

    let status = engine.status().unwrap();
    let ready = ModelStatus::Ready {
        device: "graphics card".into(),
    };
    assert_eq!((status.model, status.indexed), (ready, 1));
    let results = engine.search("beef coconut", View::Passage, 20).unwrap();
    assert_eq!(results.first().map(|r| &r.path), Some(&recipe));
}

#[test]
fn model_that_cannot_be_loaded_is_reported_and_names_still_work() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(folder.path(), "recipe.txt", "beef\n");
    let engine = Engine::start_loading(
        data.path(),
        || Err("model file is missing".into()),
        Box::new(NoJournal),
    )
    .unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().model == ModelStatus::Loading && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    engine.wait_until_idle();

    let status = engine.status().unwrap();
    assert_eq!(
        status.model,
        ModelStatus::Failed("model file is missing".into())
    );
    let results = engine.search("recipe", View::File, 20).unwrap();
    assert_eq!(results.first().map(|r| &r.path), Some(&recipe));
}

#[test]
fn search_where_keeps_only_the_files_asked_for_while_the_model_loads() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "recipe.txt", "slow cooked beef\n");
    let code = write(folder.path(), "recipe.rs", "fn cook() {}\n");
    let (release, gate) = mpsc::channel::<()>();
    let engine = Engine::start_loading(
        data.path(),
        move || {
            let _ = gate.recv();
            Ok(LoadedModel {
                embedder: Arc::new(BagOfWords),
                device: "test".into(),
            })
        },
        Box::new(NoJournal),
    )
    .unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let is_code = |path: &std::path::Path| path.extension().is_some_and(|e| e == "rs");
    let results = engine
        .search_where("recipe", View::File, 20, &is_code)
        .unwrap();

    let found: Vec<_> = results.iter().map(|result| &result.path).collect();
    assert_eq!(found, [&code]);
    drop(release);
}

#[test]
fn throttled_engine_indexes_live_updates_but_holds_back_initial_indexing() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "old.txt", "notes from the team meeting\n");
    let journal = RecordingJournal::default();
    let engine =
        Engine::start(data.path(), Arc::new(BagOfWords), Box::new(journal.clone())).unwrap();
    engine.set_activity(
        Arc::new(SteadyActivity(Some(ThrottleReason::Battery))),
        Duration::ZERO,
    );

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();
    let held = engine.status().unwrap();
    assert_eq!(
        (held.pending, held.throttled),
        (1, Some(ThrottleReason::Battery))
    );

    let new = write(folder.path(), "new.txt", "agenda for the team meeting\n");
    journal.record(&new);
    let deadline = Instant::now() + Duration::from_secs(10);
    let found = loop {
        let results = engine.search("team meeting", View::Passage, 20).unwrap();
        if !results.is_empty() || Instant::now() > deadline {
            break results;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let paths: Vec<_> = found.iter().map(|result| result.path.clone()).collect();
    assert_eq!(paths, [new]);
    assert_eq!(engine.status().unwrap().pending, 1);
}

#[test]
fn initial_indexing_goes_on_once_the_computer_has_been_quiet_for_the_quiet_period() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "notes.txt", "notes from the team meeting");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    let activity = ChangingActivity::default();
    activity.set(Some(ThrottleReason::FullScreen));
    let quiet = Duration::from_secs(1);
    engine.set_activity(Arc::new(activity.clone()), quiet);
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    activity.set(None);
    let quiet_since = Instant::now();
    engine.resume();
    engine.wait_until_idle();
    let still = engine.status().unwrap();
    assert_eq!(
        (still.pending, still.throttled),
        (1, Some(ThrottleReason::FullScreen))
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().pending > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let after = engine.status().unwrap();
    assert_eq!((after.pending, after.throttled), (0, None));
    assert!(quiet_since.elapsed() >= quiet);
}

#[test]
fn status_says_when_indexing_is_paused() {
    let data = TempDir::new().unwrap();
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();

    engine.pause();
    assert!(engine.status().unwrap().paused);
    engine.resume();
    assert!(!engine.status().unwrap().paused);
}

#[test]
fn status_reports_how_much_disk_the_indexes_take() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();
    let on_disk: u64 = std::fs::read_dir(data.path())
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    // The app keeps its settings beside the indexes; they are not part of them.
    write(data.path(), "settings.json", "{\"hotkey\":\"Alt+Space\"}");

    assert!(on_disk > 0);
    assert_eq!(engine.status().unwrap().size_on_disk, on_disk);
}
