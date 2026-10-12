//! Domain types and provider interfaces for Search by Memory.

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

/// A dense vector representing the meaning of a Query or a Chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self(values)
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Relevance between two embeddings; higher means more similar.
    pub fn similarity(&self, other: &Embedding) -> f32 {
        self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum()
    }
}

/// Turns Queries and Chunk text into Embeddings in a shared space.
pub trait EmbeddingProvider {
    fn dimensions(&self) -> usize;
    fn embed_query(&self, query: &str) -> Result<Embedding>;
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>>;
}
