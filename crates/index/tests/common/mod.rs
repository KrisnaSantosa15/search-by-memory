//! Test doubles at the system boundaries shared by the index tests.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sbm_core::{Embedding, EmbeddingProvider, Result};
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

pub fn store() -> UsearchStore {
    UsearchStore::new(DIMENSIONS, Precision::F16).unwrap()
}
