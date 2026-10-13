//! Test doubles at the system boundaries shared by the engine tests.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use sbm_core::{ChangeCursor, ChangeSource, Changes, Embedding, EmbeddingProvider, Result};

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

pub fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}
