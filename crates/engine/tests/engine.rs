//! The Engine: indexes in the background and answers Queries meanwhile.

mod common;

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant, SystemTime};

use common::{
    BagOfWords, BrokenJournal, ChangingActivity, GatedEmbedder, GatedJournal, NoJournal,
    RecordingJournal, SteadyActivity, write,
};
use sbm_core::ThrottleReason;
use sbm_engine::{Engine, LoadedModel, ModelStatus};
use sbm_index::{Exclusion, MatchReason, NotRead, View};
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
    assert_eq!(failed[0].path, broken);
    assert!(!failed[0].detail.is_empty());
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
fn search_names_finds_files_by_name_only_once_the_model_is_loaded() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let named = write(folder.path(), "invoice march.txt", "beef\n");
    write(folder.path(), "notes.txt", "the invoice for march\n");
    write(folder.path(), "invoice.rs", "fn total() {}\n");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let is_text = |path: &std::path::Path| path.extension().is_some_and(|e| e == "txt");
    let results = engine.search_names("invoice", 20, &is_text).unwrap();

    let found: Vec<_> = results.iter().map(|result| &result.path).collect();
    assert_eq!(found, [&named]);
    assert_eq!(results[0].match_reason, MatchReason::Name);
}

#[test]
fn newest_files_lists_the_files_keep_accepts_newest_modified_first() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let modified = |relative: &str, days_ago: u64| {
        let path = write(folder.path(), relative, "anything\n");
        let time = SystemTime::now() - Duration::from_secs(days_ago * 24 * 60 * 60);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(time)
            .unwrap();
        path
    };
    let older = modified("older.txt", 3);
    let newest = modified("newest.txt", 1);
    modified("photo.jpg", 0);
    modified("oldest.txt", 9);
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    let is_text = |path: &std::path::Path| path.extension().is_some_and(|e| e == "txt");
    let results = engine.newest_files(2, &is_text).unwrap();

    let found: Vec<_> = results.iter().map(|result| &result.path).collect();
    assert_eq!(found, [&newest, &older]);
}

#[test]
fn not_read_says_why_a_files_content_is_not_searched() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let photo = write(folder.path(), "holiday.jpg", "not text\n");
    let notes = write(folder.path(), "notes.txt", "team meeting\n");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    assert_eq!(engine.not_read(&photo).unwrap(), Some(NotRead::Format));
    assert_eq!(engine.not_read(&notes).unwrap(), None);
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
fn indexing_paused_until_a_time_resumes_by_itself_once_it_passes() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let recipe = write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.pause_until(SystemTime::now() + Duration::from_millis(300));

    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();
    let paused = engine.status().unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().pending > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!((paused.pending, paused.paused), (1, true));
    let after = engine.status().unwrap();
    assert_eq!((after.pending, after.paused), (0, false));
    let results = engine.search("beef coconut", View::Passage, 20).unwrap();
    assert_eq!(results[0].path, recipe);
}

#[test]
fn status_says_until_when_indexing_is_paused() {
    let data = TempDir::new().unwrap();
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    let tomorrow = SystemTime::now() + Duration::from_secs(24 * 60 * 60);

    engine.pause_until(tomorrow);
    let timed = engine.status().unwrap();
    engine.pause();
    let indefinite = engine.status().unwrap();
    engine.pause_until(tomorrow);
    engine.resume();
    let resumed = engine.status().unwrap();

    assert_eq!((timed.paused, timed.paused_until), (true, Some(tomorrow)));
    assert_eq!((indefinite.paused, indefinite.paused_until), (true, None));
    assert_eq!((resumed.paused, resumed.paused_until), (false, None));
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

#[test]
fn indexed_folders_are_listed_in_the_order_they_were_added() {
    let (data, documents, photos) = (
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(documents.path()).unwrap();
    engine.add_indexed_folder(photos.path()).unwrap();
    engine.remove_indexed_folder(documents.path()).unwrap();

    let folders = engine.indexed_folders().unwrap();

    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].path, photos.path());
    assert!(folders[0].connected);
}

#[test]
fn files_beneath_a_removed_exclusion_become_searchable_again() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let diary = write(folder.path(), "private/diary.txt", "secret garden plans\n");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    let private = Exclusion::Folder(folder.path().join("private"));
    engine.add_exclusion(private.clone()).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    engine.remove_exclusion(&private).unwrap();
    engine.wait_until_idle();

    assert!(!engine.exclusions().unwrap().contains(&private));
    let results = engine.search("secret garden", View::File, 20).unwrap();
    assert!(results.iter().any(|r| r.path == diary), "{results:?}");
}

#[test]
fn restored_default_exclusions_take_their_files_out_of_search() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "site/node_modules/lodash/readme.md",
        "lodash\n",
    );
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    let defaults = engine.exclusions().unwrap();
    engine
        .remove_exclusion(&Exclusion::Name("node_modules".to_string()))
        .unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    engine.restore_default_exclusions().unwrap();
    engine.wait_until_idle();

    assert_eq!(engine.exclusions().unwrap(), defaults);
    assert_eq!(engine.search("lodash", View::File, 20).unwrap(), []);
}

#[test]
fn a_turned_off_default_exclusion_is_still_listed_among_the_defaults() {
    let data = TempDir::new().unwrap();
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    let node_modules = Exclusion::Name("node_modules".to_string());

    engine.remove_exclusion(&node_modules).unwrap();

    assert!(!engine.exclusions().unwrap().contains(&node_modules));
    assert!(engine.default_exclusions().contains(&node_modules));
}

#[test]
fn names_are_being_found_until_the_first_pass_over_a_new_folder_ends() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "notes.txt", "groceries\n");
    let journal = GatedJournal::default();
    let engine =
        Engine::start(data.path(), Arc::new(BagOfWords), Box::new(journal.clone())).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    assert!(engine.status().unwrap().scanning);

    journal.open();
    engine.wait_until_idle();
    assert!(!engine.status().unwrap().scanning);
    assert_eq!(engine.status().unwrap().files, 1);
}

#[test]
fn the_file_being_read_is_named_until_it_is_read() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let notes = write(folder.path(), "notes.txt", "groceries");
    let embedder = GatedEmbedder::default();
    let engine =
        Engine::start(data.path(), Arc::new(embedder.clone()), Box::new(NoJournal)).unwrap();

    engine.add_indexed_folder(folder.path()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while engine.status().unwrap().current.is_none() {
        assert!(Instant::now() < deadline, "no file was named as being read");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(engine.status().unwrap().current, Some(notes));

    embedder.open();
    engine.wait_until_idle();
    assert_eq!(engine.status().unwrap().current, None);
}

#[test]
fn trying_again_puts_the_files_that_could_not_be_read_back_in_the_queue() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(folder.path(), "broken.docx", "not really a zip archive");
    let journal = GatedJournal::default();
    journal.open();
    let engine =
        Engine::start(data.path(), Arc::new(BagOfWords), Box::new(journal.clone())).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();
    assert_eq!(engine.status().unwrap().failed, 1);

    journal.close();
    engine.retry_failed().unwrap();

    let status = engine.status().unwrap();
    assert_eq!((status.failed, status.pending), (0, 1));
    journal.open();
}

#[test]
fn purge_erases_the_indexes_from_disk_and_leaves_other_files() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "recipe.txt",
        "slow cooked beef in coconut milk\n",
    );
    let settings = write(data.path(), "settings.json", "{}");
    let engine = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    engine.add_indexed_folder(folder.path()).unwrap();
    engine.wait_until_idle();

    engine.purge().unwrap();

    let left: Vec<_> = std::fs::read_dir(data.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(left, vec![settings]);
    let reopened = Engine::start(data.path(), Arc::new(BagOfWords), Box::new(NoJournal)).unwrap();
    reopened.wait_until_idle();
    assert!(reopened.indexed_folders().unwrap().is_empty());
    assert_eq!(reopened.status().unwrap().files, 0);
}

#[test]
fn purge_while_the_model_loads_leaves_no_index_behind() {
    let data = TempDir::new().unwrap();
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
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        let _ = release.send(());
    });

    engine.purge().unwrap();
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
}
