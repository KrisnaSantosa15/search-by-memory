use std::fs;
use std::path::Path;

use sbm_cli::{index_folder, search};
use sbm_core::{Embedding, EmbeddingProvider, Result};
use sbm_index::View;
use tempfile::TempDir;

const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar Embeddings.
struct BagOfWords;

impl EmbeddingProvider for BagOfWords {
    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
    fn embed_query(&self, query: &str) -> Result<Embedding> {
        let mut values = vec![0.0; DIMENSIONS];
        for word in query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            let hash = word
                .to_lowercase()
                .bytes()
                .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
            values[(hash % DIMENSIONS as u64) as usize] += 1.0;
        }
        Ok(Embedding::new(values).truncated(DIMENSIONS))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        chunks.iter().map(|chunk| self.embed_query(chunk)).collect()
    }
}

fn write(root: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn indexed_folder_is_searchable_from_a_later_run() {
    let folder = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let recipe = write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk and chili\n",
    );
    write(
        folder.path(),
        "garage/b.txt",
        "change the engine oil every five thousand miles\n",
    );

    index_folder(data.path(), folder.path(), &BagOfWords).unwrap();
    let results = search(
        data.path(),
        "beef with coconut milk",
        &BagOfWords,
        View::File,
        20,
    )
    .unwrap();

    assert_eq!(results[0].path, recipe);
}
