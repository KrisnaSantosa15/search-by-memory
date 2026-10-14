//! The Engine: keeps the indexes and the embedding model loaded, indexes in a
//! background thread, and answers Queries while it does.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use sbm_core::{
    ChangeSource, Embedding, EmbeddingProvider, Neighbor, Result, VectorId, VectorStore,
};
use sbm_index::{Exclusion, Index, MatchReason, SearchResult, View};
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
}

pub struct Engine {
    shared: Arc<Shared>,
    index: Mutex<Index>,
    worker: Option<JoinHandle<()>>,
}

struct Shared {
    /// Set once the model is loaded.
    model: OnceLock<Model>,
    work: Mutex<Work>,
    changed: Condvar,
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
    /// Why the last pass stopped early, if it did.
    error: Option<String>,
    /// Why the model could not be loaded, if it could not.
    model_error: Option<String>,
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
        let engine = Self::launch(data_dir, OnceLock::new(), changes)?;
        let (shared, database) = (engine.shared.clone(), data_dir.join(DATABASE));
        std::thread::spawn(move || {
            let loaded = load().and_then(|loaded| model_with_vectors(loaded, &database));
            let mut work = shared.work.lock().unwrap();
            match loaded {
                Ok(model) => {
                    let _ = shared.model.set(model);
                    work.requested += 1;
                }
                Err(error) => work.model_error = Some(error.to_string()),
            }
            shared.changed.notify_all();
        });
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
        });
        let worker = {
            let mut worker_index = Index::open(&database)?;
            let (shared, data_dir) = (shared.clone(), data_dir.to_path_buf());
            std::thread::spawn(move || run_worker(&mut worker_index, &shared, &*changes, &data_dir))
        };
        Ok(Self {
            shared,
            index: Mutex::new(index),
            worker: Some(worker),
        })
    }

    /// Remembers `folder` as an Indexed Folder and indexes it.
    pub fn add_indexed_folder(&self, folder: &Path) -> Result<()> {
        self.index.lock().unwrap().add_indexed_folder(folder)?;
        self.request_pass();
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

    /// Remembers `exclusion`; files it matches leave both indexes.
    pub fn add_exclusion(&self, exclusion: Exclusion) -> Result<()> {
        self.index.lock().unwrap().add_exclusion(exclusion)?;
        self.request_pass();
        Ok(())
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
        self.shared.work.lock().unwrap().paused = true;
    }

    pub fn resume(&self) {
        self.shared.work.lock().unwrap().paused = false;
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
        })
    }

    /// Files whose content could not be indexed, with the reason.
    pub fn failed_files(&self) -> Result<Vec<(PathBuf, String)>> {
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
        let index = self.index.lock().unwrap();
        let Some(model) = self.shared.model.get() else {
            if view == View::Passage {
                return Ok(Vec::new());
            }
            return Ok(index
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
                .collect());
        };
        index.search_where(
            query,
            &*model.embedder,
            &SharedStore(&model.store),
            view,
            limit,
            keep,
        )
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

/// Indexes until the Engine stops: a pass whenever one is asked for, and one
/// every `CHANGE_POLL` to pick up Live Updates.
fn run_worker(index: &mut Index, shared: &Shared, changes: &dyn ChangeSource, data_dir: &Path) {
    loop {
        let target = {
            let work = shared.work.lock().unwrap();
            if work.stop {
                return;
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
        match index.run_jobs(&*model.embedder, &mut store, 1) {
            Ok(0) => break Ok(()),
            Ok(_) => unsaved += 1,
            Err(error) => break Err(error),
        }
        if unsaved == FILES_PER_SAVE {
            model.store.read().unwrap().save(&data_dir.join(VECTORS))?;
            unsaved = 0;
        }
    };
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
