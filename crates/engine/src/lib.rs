//! The Engine: keeps the indexes and the embedding model loaded, indexes in a
//! background thread, and answers Queries while it does.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use sbm_core::{
    ChangeSource, Embedding, EmbeddingProvider, Neighbor, Result, VectorId, VectorStore,
};
use sbm_index::{Exclusion, Index, SearchResult, View};
use sbm_store_usearch::{Precision, UsearchStore};

const DATABASE: &str = "index.sqlite";
const VECTORS: &str = "vectors.usearch";
/// Files indexed between saves of the vector index.
const FILES_PER_SAVE: usize = 100;
/// How often the ChangeSource is asked for Live Updates while idle.
const CHANGE_POLL: Duration = Duration::from_secs(2);

type Embedder = Arc<dyn EmbeddingProvider + Send + Sync>;

/// What the Engine has indexed so far (FR-08).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Files in the Name Index.
    pub files: usize,
    /// Files whose content is still to be indexed.
    pub pending: usize,
    /// Files whose content could not be indexed; see [`Engine::failed_files`].
    pub failed: usize,
    /// Why the last indexing pass stopped early, if it did.
    pub error: Option<String>,
}

pub struct Engine {
    shared: Arc<Shared>,
    embedder: Embedder,
    index: Mutex<Index>,
    worker: Option<JoinHandle<()>>,
}

struct Shared {
    store: RwLock<UsearchStore>,
    work: Mutex<Work>,
    changed: Condvar,
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
}

impl Engine {
    /// Opens the indexes in `data_dir` and starts indexing in the background.
    pub fn start(
        data_dir: &Path,
        embedder: Embedder,
        changes: Box<dyn ChangeSource + Send>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let index = Index::open(&data_dir.join(DATABASE))?;
        let mut store = UsearchStore::new(embedder.dimensions(), Precision::F16)?;
        index.rebuild_vectors(&mut store)?;
        let shared = Arc::new(Shared {
            store: RwLock::new(store),
            work: Mutex::new(Work {
                requested: 1,
                ..Work::default()
            }),
            changed: Condvar::new(),
        });
        let worker = {
            let mut worker_index = Index::open(&data_dir.join(DATABASE))?;
            let (shared, embedder, data_dir) =
                (shared.clone(), embedder.clone(), data_dir.to_path_buf());
            std::thread::spawn(move || {
                run_worker(&mut worker_index, &shared, &*embedder, &*changes, &data_dir)
            })
        };
        Ok(Self {
            shared,
            embedder,
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
        self.index
            .lock()
            .unwrap()
            .remove_indexed_folder(folder, &mut SharedStore(&self.shared.store))
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
        Ok(Status {
            files: index.file_count()?,
            pending: index.pending_jobs()?,
            failed: index.failed_files()?.len(),
            error: self.shared.work.lock().unwrap().error.clone(),
        })
    }

    /// Files whose content could not be indexed, with the reason.
    pub fn failed_files(&self) -> Result<Vec<(PathBuf, String)>> {
        self.index.lock().unwrap().failed_files()
    }

    pub fn search(&self, query: &str, view: View, limit: usize) -> Result<Vec<SearchResult>> {
        let index = self.index.lock().unwrap();
        index.search(
            query,
            &*self.embedder,
            &SharedStore(&self.shared.store),
            view,
            limit,
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

/// Indexes until the Engine stops: a pass whenever one is asked for, and one
/// every `CHANGE_POLL` to pick up Live Updates.
fn run_worker(
    index: &mut Index,
    shared: &Shared,
    embedder: &dyn EmbeddingProvider,
    changes: &dyn ChangeSource,
    data_dir: &Path,
) {
    loop {
        let target = {
            let work = shared.work.lock().unwrap();
            if work.stop {
                return;
            }
            work.requested
        };
        let outcome = index_pass(index, shared, embedder, changes, data_dir);
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
/// queue is empty or the Engine is paused.
fn index_pass(
    index: &mut Index,
    shared: &Shared,
    embedder: &dyn EmbeddingProvider,
    changes: &dyn ChangeSource,
    data_dir: &Path,
) -> Result<()> {
    index.apply_changes(changes)?;
    let mut store = SharedStore(&shared.store);
    let mut unsaved = 0;
    let result = loop {
        if shared.work.lock().unwrap().paused {
            break Ok(());
        }
        match index.run_jobs(embedder, &mut store, 1) {
            Ok(0) => break Ok(()),
            Ok(_) => unsaved += 1,
            Err(error) => break Err(error),
        }
        if unsaved == FILES_PER_SAVE {
            shared.store.read().unwrap().save(&data_dir.join(VECTORS))?;
            unsaved = 0;
        }
    };
    shared.store.read().unwrap().save(&data_dir.join(VECTORS))?;
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
