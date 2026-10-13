//! Domain types and provider interfaces for Search by Memory.

use std::time::Duration;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

/// A position inside a file the user can navigate to.
#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    /// 1-based page of a paged document.
    Page(u32),
    /// Inclusive, 1-based line range of text or code.
    Lines { start: u32, end: u32 },
    /// Time range of audio or video, from the start of the media.
    Time { start: Duration, end: Duration },
    /// The whole file, for content without inner positions such as images.
    WholeFile,
}

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

/// Identifier of a Chunk's vector inside a VectorStore.
pub type VectorId = u64;

/// A stored vector close to a Query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neighbor {
    pub id: VectorId,
    pub similarity: f32,
}

/// Nearest-neighbour index over Chunk Embeddings.
pub trait VectorStore {
    fn add(&mut self, id: VectorId, embedding: &Embedding) -> Result<()>;
    fn remove(&mut self, id: VectorId) -> Result<()>;
    /// Returns up to `limit` neighbours, most similar first.
    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<Neighbor>>;
    fn len(&self) -> usize;
}
