use std::fs;
use std::path::Path;

use sbm_core::{Embedding, EmbeddingProvider, Result};
use sbm_index::{Index, MatchReason, View};
use sbm_store_usearch::{Precision, UsearchStore};
use tempfile::TempDir;

const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar Embeddings.
struct BagOfWords;

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

fn write(root: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

fn store() -> UsearchStore {
    UsearchStore::new(DIMENSIONS, Precision::F16).unwrap()
}

#[test]
fn file_content_is_found_by_meaning() {
    let folder = TempDir::new().unwrap();
    let recipe = write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk and chili\n",
    );
    let other = write(
        folder.path(),
        "garage/b.txt",
        "change the engine oil every five thousand miles\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&recipe, &BagOfWords, &mut store)
        .unwrap();
    index
        .index_content(&other, &BagOfWords, &mut store)
        .unwrap();

    let results = index
        .search(
            "beef with coconut milk",
            &BagOfWords,
            &store,
            View::File,
            20,
        )
        .unwrap();

    assert_eq!(results[0].path, recipe);
    assert_eq!(results[0].match_reason, MatchReason::Content);
}
