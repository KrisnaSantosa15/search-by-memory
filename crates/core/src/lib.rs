//! Domain types and provider interfaces for Search by Memory.

use std::ffi::OsString;
use std::path::PathBuf;
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

impl Location {
    /// Whether both Locations point at some shared part of the same file.
    pub fn overlaps(&self, other: &Location) -> bool {
        use Location::*;
        match (self, other) {
            (WholeFile, _) | (_, WholeFile) => true,
            (Page(a), Page(b)) => a == b,
            (Lines { start: a1, end: a2 }, Lines { start: b1, end: b2 }) => a1 <= b2 && b1 <= a2,
            (Time { start: a1, end: a2 }, Time { start: b1, end: b2 }) => a1 <= b2 && b1 <= a2,
            _ => false,
        }
    }
}

/// Extracted text of one part of a file, such as a page, with its Location.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub location: Location,
    pub text: String,
}

/// Reads the text content of files in a supported format, never modifying them.
pub trait ContentExtractor {
    fn extract(&self, path: &std::path::Path) -> Result<Vec<Section>>;
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

    /// Matryoshka truncation: keeps the leading `dimensions` values, rescaled to unit length.
    pub fn truncated(&self, dimensions: usize) -> Embedding {
        let mut values = self.0[..dimensions.min(self.0.len())].to_vec();
        let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            values.iter_mut().for_each(|v| *v /= norm);
        }
        Embedding(values)
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

/// The operating system convention that decides where the app keeps its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Unix,
}

/// Where the app keeps its index (`data`) and re-creatable files such as
/// models (`cache`).
#[derive(Debug, Clone, PartialEq)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    /// The directories for the running platform and environment.
    pub fn current() -> Self {
        let platform = if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        };
        Self::from_env(platform, |name| std::env::var_os(name))
    }

    /// Resolves the directories from environment variables, looked up by `var`.
    pub fn from_env(platform: Platform, var: impl Fn(&str) -> Option<OsString>) -> Self {
        let dir = |name| PathBuf::from(var(name).unwrap_or_default());
        match platform {
            Platform::Windows => {
                let root = dir("LOCALAPPDATA").join(APP_DIR);
                AppDirs {
                    cache: root.join("cache"),
                    data: root,
                }
            }
            Platform::Unix => {
                let home = dir("HOME");
                AppDirs {
                    data: home.join(".local/share").join(APP_DIR),
                    cache: home.join(".cache").join(APP_DIR),
                }
            }
        }
    }
}

const APP_DIR: &str = "search-by-memory";
