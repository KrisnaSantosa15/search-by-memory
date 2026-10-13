//! Test doubles at the system boundaries shared by the index tests.

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sbm_core::{
    ChangeCursor, ChangeSource, Changes, Embedding, EmbeddingProvider, FileAvailability, Result,
    Volume, Volumes,
};
use sbm_store_usearch::{Precision, UsearchStore};

pub const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar
/// Embeddings. Counts the Chunks it embeds.
#[derive(Default)]
pub struct BagOfWords {
    embedded: AtomicUsize,
}

impl BagOfWords {
    /// Chunks embedded so far.
    pub fn embedded(&self) -> usize {
        self.embedded.load(Ordering::SeqCst)
    }

    fn embed(text: &str) -> Embedding {
        let mut values = vec![0.0; DIMENSIONS];
        for word in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            let hash = word
                .to_lowercase()
                .bytes()
                .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
            values[(hash % DIMENSIONS as u64) as usize] += 1.0;
        }
        Embedding::new(values).truncated(DIMENSIONS)
    }
}

impl EmbeddingProvider for BagOfWords {
    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
    fn embed_query(&self, query: &str) -> Result<Embedding> {
        Ok(Self::embed(query))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        self.embedded.fetch_add(chunks.len(), Ordering::SeqCst);
        Ok(chunks.iter().map(|chunk| Self::embed(chunk)).collect())
    }
}

pub fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

/// Sets the modification time of `path` to `days` days ago.
pub fn modified_days_ago(path: &Path, days: u64) {
    let time = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

pub fn store() -> UsearchStore {
    UsearchStore::new(DIMENSIONS, Precision::F16).unwrap()
}

/// Stands in for the operating system's volume table: tests mount, move,
/// and disconnect volumes by ID.
#[derive(Clone, Default)]
pub struct FakeVolumes(Arc<Mutex<HashMap<String, PathBuf>>>);

impl FakeVolumes {
    pub fn mount(&self, id: &str, root: &Path) {
        self.0
            .lock()
            .unwrap()
            .insert(id.to_string(), root.to_path_buf());
    }

    pub fn disconnect(&self, id: &str) {
        self.0.lock().unwrap().remove(id);
    }
}

impl Volumes for FakeVolumes {
    fn volume_of(&self, path: &Path) -> Result<Volume> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, root)| path.starts_with(root))
            .max_by_key(|(_, root)| root.components().count())
            .map(|(id, root)| Volume {
                id: id.clone(),
                root: root.clone(),
            })
            .ok_or_else(|| format!("no volume holds {}", path.display()).into())
    }

    fn mount_point(&self, id: &str) -> Option<PathBuf> {
        self.0.lock().unwrap().get(id).cloned()
    }
}

/// Stands in for the operating system's change journal: tests record the
/// paths that changed on each volume. A cursor is the journal's generation and
/// the number of its changes read.
#[derive(Clone, Default)]
pub struct FakeChangeSource(Arc<Mutex<HashMap<String, Journal>>>);

#[derive(Default)]
pub struct Journal {
    generation: u32,
    changes: Vec<String>,
}

impl FakeChangeSource {
    /// Records that `path`, relative to the root of volume `volume_id`, changed.
    pub fn record(&self, volume_id: &str, path: &str) {
        self.0
            .lock()
            .unwrap()
            .entry(volume_id.to_string())
            .or_default()
            .changes
            .push(path.to_string());
    }

    /// Starts a new journal on volume `volume_id`, as when the operating system
    /// recreates it: changes before now are gone.
    pub fn lose_history(&self, volume_id: &str) {
        let mut journals = self.0.lock().unwrap();
        let journal = journals.entry(volume_id.to_string()).or_default();
        journal.generation += 1;
        journal.changes.clear();
    }
}

impl ChangeSource for FakeChangeSource {
    fn changes_since(&self, volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes> {
        let mut journals = self.0.lock().unwrap();
        let journal = journals.entry(volume_id.to_string()).or_default();
        let next = ChangeCursor(format!("{}:{}", journal.generation, journal.changes.len()));
        let read = since.and_then(|since| {
            let (generation, read) = since.0.split_once(':')?;
            (generation.parse() == Ok(journal.generation)).then(|| read.parse::<usize>().ok())?
        });
        Ok(match read {
            Some(read) => Changes::Paths {
                paths: journal.changes[read..].to_vec(),
                next,
            },
            None => Changes::Lost { next },
        })
    }
}

/// Stands in for the operating system's record of which files are Online-only
/// Files: tests move a file's content to the cloud and download it again.
#[derive(Clone, Default)]
pub struct FakeFileAvailability(Arc<Mutex<Vec<PathBuf>>>);

impl FakeFileAvailability {
    pub fn move_to_cloud(&self, path: &Path) {
        self.0.lock().unwrap().push(path.to_path_buf());
    }

    pub fn download(&self, path: &Path) {
        self.0
            .lock()
            .unwrap()
            .retain(|online_only| online_only != path);
    }
}

impl FileAvailability for FakeFileAvailability {
    fn is_online_only(&self, path: &Path) -> bool {
        self.0
            .lock()
            .unwrap()
            .iter()
            .any(|online_only| online_only == path)
    }
}
