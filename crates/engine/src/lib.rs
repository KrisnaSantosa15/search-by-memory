//! The Engine: keeps the indexes and the embedding model loaded, indexes in a
//! background thread, and answers Queries while it does.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use sbm_core::{
    Activity, ChangeSource, Embedding, EmbeddingProvider, Neighbor, Result, ThrottleReason,
    VectorId, VectorStore,
};
use sbm_index::{Exclusion, FailedFile, Index, IndexedFolder, MatchReason, SearchResult, View};
use sbm_store_usearch::{Precision, UsearchStore};

/// Name Matches looked at per Result asked for, when only some files are kept.
const CANDIDATES_PER_RESULT: usize = 10;

const DATABASE: &str = "index.sqlite";
const VECTORS: &str = "vectors.usearch";
/// Files indexed between saves of the vector index.
const FILES_PER_SAVE: usize = 100;
/// How often the ChangeSource is asked for Live Updates while idle.
const CHANGE_POLL: Duration = Duration::from_secs(2);

type Embedder = Arc<dyn EmbeddingProvider + Send + Sync>;

/// An embedding model ready to use, and the device it runs on.
pub struct LoadedModel {
    pub embedder: Embedder,
    pub device: String,
}

/// Whether the embedding model can be used yet (FR-08).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ModelStatus {
    /// Files are found by name only, and content waits, until the model is loaded.
    #[default]
    Loading,
    Ready {
        device: String,
    },
    Failed(String),
}

/// What the Engine has indexed so far (FR-08).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Files in the Name Index.
    pub files: usize,
    /// Files whose content is indexed.
    pub indexed: usize,
    /// Files whose content is still to be indexed.
    pub pending: usize,
    /// Every file modified after this time has its content indexed; `None` when
    /// nothing is pending.
    pub ready_since: Option<SystemTime>,
    /// Files whose content could not be indexed; see [`Engine::failed_files`].
    pub failed: usize,
    /// Why the last indexing pass stopped early, if it did.
    pub error: Option<String>,
    pub model: ModelStatus,
    /// Why Initial Indexing is holding back, if it is; Live Updates go on.
    pub throttled: Option<ThrottleReason>,
    /// The names in a newly added Indexed Folder are still being found; they
    /// are searchable once this turns false.
    pub scanning: bool,
    /// The file whose content is being read right now.
    pub current: Option<PathBuf>,
    /// Content indexing stopped by [`Engine::pause`].
    pub paused: bool,
    /// When a pause from [`Engine::pause_until`] ends by itself.
    pub paused_until: Option<SystemTime>,
    /// Bytes the indexes take on disk.
    pub size_on_disk: u64,
}

pub struct Engine {
    shared: Arc<Shared>,
    index: Mutex<Index>,
    data_dir: PathBuf,
    worker: Option<JoinHandle<()>>,
    /// Loads the model, from [`Engine::start_loading`].
    loader: Option<JoinHandle<()>>,
}

struct Shared {
    /// Set once the model is loaded.
    model: OnceLock<Model>,
    work: Mutex<Work>,
    changed: Condvar,
    throttle: Mutex<Throttle>,
}

/// Holds Initial Indexing back while the computer is in use, and for a quiet
/// period after (FR-04).
#[derive(Default)]
struct Throttle {
    activity: Option<Arc<dyn Activity + Send + Sync>>,
    quiet: Duration,
    /// The last reason seen, and when.
    last: Option<(ThrottleReason, Instant)>,
}

impl Throttle {
    /// Why indexing holds back now: what the computer is doing, or what it did
    /// less than the quiet period ago.
    fn reason(&mut self) -> Option<ThrottleReason> {
        let now = Instant::now();
        if let Some(reason) = self.activity.as_ref()?.throttle_reason() {
            self.last = Some((reason, now));
        }
        self.last
            .filter(|(_, seen)| now.duration_since(*seen) <= self.quiet)
            .map(|(reason, _)| reason)
    }
}

struct Model {
    embedder: Embedder,
    device: String,
    store: RwLock<UsearchStore>,
}

/// Indexing passes asked for and finished; a pass picks up everything asked before it.
#[derive(Default)]
struct Work {
    requested: u64,
    completed: u64,
    stop: bool,
    /// No content is indexed while paused.
    paused: bool,
    /// When a pause ends by itself, if it does.
    paused_until: Option<SystemTime>,
    /// Why the last pass stopped early, if it did.
    error: Option<String>,
    /// Why the model could not be loaded, if it could not.
    model_error: Option<String>,
    /// Why Initial Indexing held back when last checked.
    throttled: Option<ThrottleReason>,
    /// The pass that finds the names in the newest Indexed Folder.
    scan_pass: u64,
    /// The file whose content is being read, if any.
    current: Option<PathBuf>,
}

impl Engine {
    /// Opens the indexes in `data_dir` and starts indexing in the background.
    pub fn start(
        data_dir: &Path,
        embedder: Embedder,
        changes: Box<dyn ChangeSource + Send>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let model = OnceLock::new();
        let loaded = LoadedModel {
            embedder,
            device: String::new(),
        };
        let _ = model.set(model_with_vectors(loaded, &data_dir.join(DATABASE))?);
        Self::launch(data_dir, model, changes)
    }

    /// Opens the indexes in `data_dir` and starts indexing in the background while
    /// `load` loads the embedding model on a thread of its own.
    pub fn start_loading(
        data_dir: &Path,
        load: impl FnOnce() -> Result<LoadedModel> + Send + 'static,
        changes: Box<dyn ChangeSource + Send>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let mut engine = Self::launch(data_dir, OnceLock::new(), changes)?;
        let (shared, database) = (engine.shared.clone(), data_dir.join(DATABASE));
        engine.loader = Some(std::thread::spawn(move || {
            let loaded = load();
            if shared.work.lock().unwrap().stop {
                // Stopped, perhaps purged, meanwhile: leave the database alone.
                return;
            }
            let loaded = loaded.and_then(|loaded| model_with_vectors(loaded, &database));
            let mut work = shared.work.lock().unwrap();
            match loaded {
                Ok(model) => {
                    let _ = shared.model.set(model);
                    work.requested += 1;
                }
                Err(error) => work.model_error = Some(error.to_string()),
            }
            shared.changed.notify_all();
        }));
        Ok(engine)
    }

    fn launch(
        data_dir: &Path,
        model: OnceLock<Model>,
        changes: Box<dyn ChangeSource + Send>,
    ) -> Result<Self> {
        let database = data_dir.join(DATABASE);
        let index = Index::open(&database)?;
        let shared = Arc::new(Shared {
            model,
            work: Mutex::new(Work {
                requested: 1,
                ..Work::default()
            }),
            changed: Condvar::new(),
            throttle: Mutex::default(),
        });
        let worker = {
            let mut worker_index = Index::open(&database)?;
            let (shared, data_dir) = (shared.clone(), data_dir.to_path_buf());
            std::thread::spawn(move || run_worker(&mut worker_index, &shared, &*changes, &data_dir))
        };
        Ok(Self {
            shared,
            index: Mutex::new(index),
            data_dir: data_dir.to_path_buf(),
            worker: Some(worker),
            loader: None,
        })
    }

    /// Remembers `folder` as an Indexed Folder and indexes it.
    pub fn add_indexed_folder(&self, folder: &Path) -> Result<()> {
        self.index.lock().unwrap().add_indexed_folder(folder)?;
        let mut work = self.shared.work.lock().unwrap();
        work.requested += 1;
        work.scan_pass = work.requested;
        self.shared.changed.notify_all();
        Ok(())
    }

    /// Forgets the Indexed Folder `folder`: its files leave both indexes.
    pub fn remove_indexed_folder(&self, folder: &Path) -> Result<()> {
        let mut index = self.index.lock().unwrap();
        match self.shared.model.get() {
            Some(model) => index.remove_indexed_folder(folder, &mut SharedStore(&model.store)),
            // Vectors are rebuilt from the Content Index once the model is loaded.
            None => index.remove_indexed_folder(folder, &mut NoVectors),
        }
    }

    /// The Indexed Folders, in the order they were added.
    pub fn indexed_folders(&self) -> Result<Vec<IndexedFolder>> {
        self.index.lock().unwrap().indexed_folders()
    }

    /// Remembers `exclusion`; files it matches leave both indexes.
    pub fn add_exclusion(&self, exclusion: Exclusion) -> Result<()> {
        self.index.lock().unwrap().add_exclusion(exclusion)?;
        self.request_pass();
        Ok(())
    }

    /// Forgets `exclusion`, or turns it off if it is a Default Exclusion; the files it
    /// kept out join both indexes.
    pub fn remove_exclusion(&self, exclusion: &Exclusion) -> Result<()> {
        self.index.lock().unwrap().remove_exclusion(exclusion)?;
        self.request_pass();
        Ok(())
    }

    /// Turns every Default Exclusion back on; the files they match leave both indexes.
    pub fn restore_default_exclusions(&self) -> Result<()> {
        self.index.lock().unwrap().restore_default_exclusions()?;
        self.request_pass();
        Ok(())
    }

    /// The Default Exclusions still on, followed by the user's.
    pub fn exclusions(&self) -> Result<Vec<Exclusion>> {
        self.index.lock().unwrap().exclusions()
    }

    /// Reads the files whose content could not be indexed once more, e.g. after
    /// the program that had one open has closed it.
    pub fn retry_failed(&self) -> Result<()> {
        self.index.lock().unwrap().retry_failed()?;
        self.request_pass();
        Ok(())
    }

    /// Every Default Exclusion, on or turned off.
    pub fn default_exclusions(&self) -> Vec<Exclusion> {
        self.index.lock().unwrap().default_exclusions()
    }

    /// Blocks until all work queued so far is done.
    pub fn wait_until_idle(&self) {
        let work = self.shared.work.lock().unwrap();
        let target = work.requested;
        let _done = self
            .shared
            .changed
            .wait_while(work, |work| work.completed < target && !work.stop)
            .unwrap();
    }

    /// Stops indexing content until [`Engine::resume`]; the Name Index stays current.
    pub fn pause(&self) {
        let mut work = self.shared.work.lock().unwrap();
        (work.paused, work.paused_until) = (true, None);
    }

    /// Like [`Engine::pause`], until `when` has passed; the wall clock decides,
    /// so a computer asleep meanwhile resumes on waking.
    pub fn pause_until(&self, when: SystemTime) {
        let mut work = self.shared.work.lock().unwrap();
        (work.paused, work.paused_until) = (true, Some(when));
    }

    pub fn resume(&self) {
        let mut work = self.shared.work.lock().unwrap();
        (work.paused, work.paused_until) = (false, None);
        drop(work);
        self.request_pass();
    }

    /// Holds Initial Indexing back whenever `activity` gives a reason, and until
    /// it has given none for `quiet`; Live Updates go on regardless.
    pub fn set_activity(&self, activity: Arc<dyn Activity + Send + Sync>, quiet: Duration) {
        *self.shared.throttle.lock().unwrap() = Throttle {
            activity: Some(activity),
            quiet,
            last: None,
        };
        self.request_pass();
    }

    pub fn status(&self) -> Result<Status> {
        let index = self.index.lock().unwrap();
        let progress = index.content_progress()?;
        let work = self.shared.work.lock().unwrap();
        Ok(Status {
            files: index.file_count()?,
            indexed: progress.indexed,
            pending: progress.pending,
            ready_since: progress.ready_since,
            failed: index.failed_files()?.len(),
            error: work.error.clone(),
            model: match (self.shared.model.get(), &work.model_error) {
                (Some(model), _) => ModelStatus::Ready {
                    device: model.device.clone(),
                },
                (None, Some(error)) => ModelStatus::Failed(error.clone()),
                (None, None) => ModelStatus::Loading,
            },
            throttled: work.throttled,
            scanning: work.completed < work.scan_pass,
            current: work.current.clone(),
            paused: work.paused,
            paused_until: work.paused_until,
            size_on_disk: size_on_disk(&self.data_dir),
        })
    }

    /// Stops indexing and erases the indexes from disk (FR-06): the catalog,
    /// Chunks, embeddings, Indexed Folders, Exclusions, and vectors. Other
    /// files in the data directory stay.
    pub fn purge(mut self) -> Result<()> {
        let (data_dir, loader) = (self.data_dir.clone(), self.loader.take());
        drop(self);
        if let Some(loader) = loader {
            let _ = loader.join();
        }
        for file in index_files(&data_dir) {
            std::fs::remove_file(file.path())?;
        }
        Ok(())
    }

    /// Files whose content could not be indexed, with the reason.
    pub fn failed_files(&self) -> Result<Vec<FailedFile>> {
        self.index.lock().unwrap().failed_files()
    }

    /// Finds files by name and content; by name only until the model is loaded.
    pub fn search(&self, query: &str, view: View, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_where(query, view, limit, &|_| true)
    }

    /// Like `search`, over only the files `keep` accepts (e.g. code).
    pub fn search_where(
        &self,
        query: &str,
        view: View,
        limit: usize,
        keep: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<SearchResult>> {
        let Some(model) = self.shared.model.get() else {
            if view == View::Passage {
                return Ok(Vec::new());
            }
            return self.search_names(query, limit, keep);
        };
        self.index.lock().unwrap().search_where(
            query,
            &*model.embedder,
            &SharedStore(&model.store),
            view,
            limit,
            keep,
        )
    }

    /// Finds files by name only, over only the files `keep` accepts; one Result per file.
    pub fn search_names(
        &self,
        query: &str,
        limit: usize,
        keep: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<SearchResult>> {
        Ok(self
            .index
            .lock()
            .unwrap()
            .search_names(query, limit * CANDIDATES_PER_RESULT)?
            .into_iter()
            .filter(|name| keep(&name.path))
            .take(limit)
            .map(|name| SearchResult {
                path: name.path,
                copies: Vec::new(),
                connected: name.connected,
                snippet: None,
                location: None,
                match_reason: MatchReason::Name,
                weaker: false,
            })
            .collect())
    }

    /// Up to `limit` files that `keep` accepts, newest modified first, as name Results.
    pub fn newest_files(
        &self,
        limit: usize,
        keep: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<SearchResult>> {
        Ok(self
            .index
            .lock()
            .unwrap()
            .newest_files(limit, keep)?
            .into_iter()
            .map(|file| SearchResult {
                path: file.path,
                copies: Vec::new(),
                connected: file.connected,
                snippet: None,
                location: None,
                match_reason: MatchReason::Name,
                weaker: false,
            })
            .collect())
    }

    fn request_pass(&self) {
        self.shared.work.lock().unwrap().requested += 1;
        self.shared.changed.notify_all();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shared.work.lock().unwrap().stop = true;
        self.shared.changed.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The loaded model with a vector index rebuilt from the Content Index.
fn model_with_vectors(loaded: LoadedModel, database: &Path) -> Result<Model> {
    let mut store = UsearchStore::new(loaded.embedder.dimensions(), Precision::F16)?;
    Index::open(database)?.rebuild_vectors(&mut store)?;
    Ok(Model {
        embedder: loaded.embedder,
        device: loaded.device,
        store: RwLock::new(store),
    })
}

/// The indexes' files in `dir`: the database with its journal files, and the vectors.
fn index_files(dir: &Path) -> impl Iterator<Item = std::fs::DirEntry> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(DATABASE) || name == VECTORS
        })
}

/// Bytes taken by the indexes in `dir`. A file that vanishes meanwhile counts as none.
fn size_on_disk(dir: &Path) -> u64 {
    index_files(dir)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

/// Indexes until the Engine stops: a pass whenever one is asked for, and one
/// every `CHANGE_POLL` to pick up Live Updates.
fn run_worker(index: &mut Index, shared: &Shared, changes: &dyn ChangeSource, data_dir: &Path) {
    loop {
        let target = {
            let mut work = shared.work.lock().unwrap();
            if work.stop {
                return;
            }
            if work
                .paused_until
                .is_some_and(|until| until <= SystemTime::now())
            {
                (work.paused, work.paused_until) = (false, None);
            }
            // A poll pass counts as asked for, so `wait_until_idle` waits for it too.
            if work.requested == work.completed {
                work.requested += 1;
            }
            work.requested
        };
        let outcome = index_pass(index, shared, changes, data_dir);
        let mut work = shared.work.lock().unwrap();
        work.error = outcome.err().map(|error| error.to_string());
        work.completed = target;
        shared.changed.notify_all();
        let (_work, _timeout) = shared
            .changed
            .wait_timeout_while(work, CHANGE_POLL, |work| {
                work.requested == target && !work.stop
            })
            .unwrap();
    }
}

/// Applies changes since the last pass and indexes queued content until the
/// queue is empty, the Engine is paused, or while the model is not loaded.
fn index_pass(
    index: &mut Index,
    shared: &Shared,
    changes: &dyn ChangeSource,
    data_dir: &Path,
) -> Result<()> {
    index.apply_changes(changes)?;
    let Some(model) = shared.model.get() else {
        return Ok(());
    };
    let mut store = SharedStore(&model.store);
    let mut unsaved = 0;
    let result = loop {
        if shared.work.lock().unwrap().paused {
            break Ok(());
        }
        let throttled = shared.throttle.lock().unwrap().reason();
        let next = match index.next_queued_file(throttled.is_some()) {
            Ok(next) => next,
            Err(error) => break Err(error),
        };
        {
            let mut work = shared.work.lock().unwrap();
            work.throttled = throttled;
            work.current = next;
        }
        let ran = match throttled {
            Some(_) => index.run_live_jobs(&*model.embedder, &mut store, 1),
            None => index.run_jobs(&*model.embedder, &mut store, 1),
        };
        match ran {
            Ok(0) => break Ok(()),
            Ok(_) => unsaved += 1,
            Err(error) => break Err(error),
        }
        if unsaved == FILES_PER_SAVE {
            model.store.read().unwrap().save(&data_dir.join(VECTORS))?;
            unsaved = 0;
        }
    };
    shared.work.lock().unwrap().current = None;
    model.store.read().unwrap().save(&data_dir.join(VECTORS))?;
    result
}

/// The vector store shared by indexing and search, locked per operation.
struct SharedStore<'a>(&'a RwLock<UsearchStore>);

impl VectorStore for SharedStore<'_> {
    fn add(&mut self, id: VectorId, embedding: &Embedding) -> Result<()> {
        self.0.write().unwrap().add(id, embedding)
    }
    fn remove(&mut self, id: VectorId) -> Result<()> {
        self.0.write().unwrap().remove(id)
    }
    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<Neighbor>> {
        self.0.read().unwrap().search(query, limit)
    }
    fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }
}

/// Stands in for the vector store before the model is loaded.
struct NoVectors;

impl VectorStore for NoVectors {
    fn add(&mut self, _id: VectorId, _embedding: &Embedding) -> Result<()> {
        Ok(())
    }
    fn remove(&mut self, _id: VectorId) -> Result<()> {
        Ok(())
    }
    fn search(&self, _query: &Embedding, _limit: usize) -> Result<Vec<Neighbor>> {
        Ok(Vec::new())
    }
    fn len(&self) -> usize {
        0
    }
}
