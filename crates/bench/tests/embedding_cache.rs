use std::cell::Cell;

use sbm_bench::EmbeddingCache;
use sbm_core::{Embedding, EmbeddingProvider, Result};

/// Counts how many Chunks reach the (expensive) model.
#[derive(Default)]
struct CountingEmbedder {
    embedded: Cell<usize>,
}

impl EmbeddingProvider for CountingEmbedder {
    fn dimensions(&self) -> usize {
        2
    }
    fn embed_query(&self, _query: &str) -> Result<Embedding> {
        unreachable!("the cache only embeds Chunks")
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        self.embedded.set(self.embedded.get() + chunks.len());
        Ok(chunks
            .iter()
            .map(|chunk| Embedding::new(vec![chunk.len() as f32, 1.0]))
            .collect())
    }
}

#[test]
fn chunks_of_an_already_embedded_file_are_not_embedded_again() {
    let dir = tempfile::tempdir().unwrap();
    let embedder = CountingEmbedder::default();
    let chunks = ["first chunk", "second"];

    let first = EmbeddingCache::new(dir.path())
        .embed("file-sha256", &chunks, &embedder)
        .unwrap();
    let second = EmbeddingCache::new(dir.path())
        .embed("file-sha256", &chunks, &embedder)
        .unwrap();

    assert_eq!(embedder.embedded.get(), 2);
    assert_eq!(first, second);
    assert_eq!(second[0], Embedding::new(vec![11.0, 1.0]));
}
