//! Test doubles at the system boundaries shared by the engine tests.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sbm_core::{
    Activity, ChangeCursor, ChangeSource, Changes, Embedding, EmbeddingProvider, Result,
    SystemVolumes, ThrottleReason, Volumes,
};

pub const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar Embeddings.
#[derive(Default)]
pub struct BagOfWords;

impl BagOfWords {
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
        Ok(chunks.iter().map(|chunk| Self::embed(chunk)).collect())
    }
}

/// Stands in for a file system without a change journal: every check rescans.
pub struct NoJournal;

impl ChangeSource for NoJournal {
    fn changes_since(&self, _volume_id: &str, _since: Option<&ChangeCursor>) -> Result<Changes> {
        Ok(Changes::Lost {
            next: ChangeCursor(String::new()),
        })
    }
}

/// Like [`NoJournal`], but each pass waits at the journal until [`GatedJournal::open`],
/// so a test can look at the Engine in the middle of a pass. Clones share one gate.
#[derive(Clone, Default)]
pub struct GatedJournal(Arc<(Mutex<bool>, std::sync::Condvar)>);

impl GatedJournal {
    pub fn open(&self) {
        let (open, changed) = &*self.0;
        *open.lock().unwrap() = true;
        changed.notify_all();
    }

    /// Makes the next pass wait at the journal again.
    pub fn close(&self) {
        *self.0.0.lock().unwrap() = false;
    }
}

impl ChangeSource for GatedJournal {
    fn changes_since(&self, volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes> {
        let (open, changed) = &*self.0;
        let _open = changed
            .wait_while(open.lock().unwrap(), |open| !*open)
            .unwrap();
        NoJournal.changes_since(volume_id, since)
    }
}

/// Like [`BagOfWords`], but reading a file's Chunks waits until [`GatedEmbedder::open`],
/// so a test can look at the Engine while a file is being read. Clones share one gate.
#[derive(Clone, Default)]
pub struct GatedEmbedder(Arc<(Mutex<bool>, std::sync::Condvar)>);

impl GatedEmbedder {
    pub fn open(&self) {
        let (open, changed) = &*self.0;
        *open.lock().unwrap() = true;
        changed.notify_all();
    }
}

impl EmbeddingProvider for GatedEmbedder {
    fn dimensions(&self) -> usize {
        BagOfWords.dimensions()
    }
    fn embed_query(&self, query: &str) -> Result<Embedding> {
        BagOfWords.embed_query(query)
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        let (open, changed) = &*self.0;
        let _open = changed
            .wait_while(open.lock().unwrap(), |open| !*open)
            .unwrap();
        BagOfWords.embed_chunks(chunks)
    }
}

pub fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

/// Stands in for a change journal that cannot be read.
pub struct BrokenJournal;

impl ChangeSource for BrokenJournal {
    fn changes_since(&self, _volume_id: &str, _since: Option<&ChangeCursor>) -> Result<Changes> {
        Err("the change journal cannot be read".into())
    }
}

/// Stands in for a change journal: reports the paths recorded since the cursor,
/// on the real volumes they sit on. Without a cursor, the history is lost.
/// Clones share one record.
#[derive(Clone, Default)]
pub struct RecordingJournal(Arc<Mutex<Vec<PathBuf>>>);

impl RecordingJournal {
    pub fn record(&self, path: &Path) {
        self.0.lock().unwrap().push(path.to_path_buf());
    }
}

impl ChangeSource for RecordingJournal {
    fn changes_since(&self, volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes> {
        let recorded = self.0.lock().unwrap();
        let next = ChangeCursor(recorded.len().to_string());
        let Some(read) = since.and_then(|since| since.0.parse::<usize>().ok()) else {
            return Ok(Changes::Lost { next });
        };
        let root = SystemVolumes.mount_point(volume_id).unwrap();
        let paths = recorded[read..]
            .iter()
            .filter_map(|path| path.strip_prefix(&root).ok())
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .collect();
        Ok(Changes::Paths { paths, next })
    }
}

/// Stands in for what the computer is doing: always the same answer.
pub struct SteadyActivity(pub Option<ThrottleReason>);

impl Activity for SteadyActivity {
    fn throttle_reason(&self) -> Option<ThrottleReason> {
        self.0
    }
}

/// Stands in for what the computer is doing, changed by the test as it goes.
#[derive(Clone, Default)]
pub struct ChangingActivity(Arc<Mutex<Option<ThrottleReason>>>);

impl ChangingActivity {
    pub fn set(&self, reason: Option<ThrottleReason>) {
        *self.0.lock().unwrap() = reason;
    }
}

impl Activity for ChangingActivity {
    fn throttle_reason(&self) -> Option<ThrottleReason> {
        *self.0.lock().unwrap()
    }
}
