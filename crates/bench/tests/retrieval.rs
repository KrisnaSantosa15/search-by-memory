//! End-to-end retrieval scoring with a fake embedder standing in for the ONNX model.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use sbm_bench::{Benchmark, ExactStore, RetrievalIndex, chunk_text, evaluate};
use sbm_core::{Embedding, EmbeddingProvider, Result};

const DIMENSIONS: usize = 64;

/// Bag-of-words embedder: texts sharing words are similar.
struct BagOfWords;

impl BagOfWords {
    fn embed(text: &str) -> Embedding {
        let mut values = vec![0.0f32; DIMENSIONS];
        for word in text.split_whitespace() {
            let mut hasher = DefaultHasher::new();
            word.to_lowercase().hash(&mut hasher);
            values[hasher.finish() as usize % DIMENSIONS] += 1.0;
        }
        let norm = values
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt()
            .max(f32::MIN_POSITIVE);
        Embedding::new(values.into_iter().map(|v| v / norm).collect())
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

#[test]
fn query_finds_the_file_whose_chunks_share_its_meaning() {
    let mut chunks = chunk_text(
        Path::new("notes/fruit.txt"),
        "ripe red apples and sweet pears\n",
        1000,
    );
    chunks.extend(chunk_text(
        Path::new("notes/cars.txt"),
        "fast engines and racing tires\n",
        1000,
    ));
    let texts: Vec<&str> = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
    let embeddings = BagOfWords.embed_chunks(&texts).unwrap();
    let index = RetrievalIndex::build(chunks, &embeddings, ExactStore::default()).unwrap();
    let benchmark = Benchmark::parse(
        r#"
        [[query]]
        id = "en-001"
        text = "racing engines"
        language = "en"

        [[query.target]]
        file = "notes/cars.txt"
        lines = [1, 1]
        "#,
    )
    .unwrap();

    let scores = evaluate(&benchmark, &index, &BagOfWords).unwrap();

    assert_eq!(scores.recall_at(1), 1.0);
}
