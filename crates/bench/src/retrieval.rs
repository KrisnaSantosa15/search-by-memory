//! Semantic-only retrieval over Chunks, scored against a Benchmark (dev-only stand-in for M2 search).

use std::path::{Path, PathBuf};

use sbm_core::{Embedding, EmbeddingProvider, Location, Neighbor, Result, VectorId, VectorStore};

use crate::{Benchmark, FileRanking, FileResult, Scores};

/// Number of File-view Results scored per Query.
const RESULTS_PER_QUERY: usize = 100;

/// A slice of a file's extracted content, with its Location.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub file: PathBuf,
    pub location: Location,
    pub text: String,
}

/// Splits text or code into Chunks of whole lines, each at most `max_chars`
/// long unless a single line is longer.
pub fn chunk_text(file: &Path, text: &str, max_chars: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut start = 1;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let number = index as u32 + 1;
        if !current.is_empty() && current.len() + line.len() > max_chars {
            chunks.push(Chunk {
                file: file.to_path_buf(),
                location: Location::Lines {
                    start,
                    end: number - 1,
                },
                text: std::mem::take(&mut current),
            });
            start = number;
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        let end = start + current.split_inclusive('\n').count() as u32 - 1;
        chunks.push(Chunk {
            file: file.to_path_buf(),
            location: Location::Lines { start, end },
            text: current,
        });
    }
    chunks
}

/// Splits paged text (pages separated by form feeds, as pdftotext writes them)
/// into Chunks of whole lines within each page, located by 1-based page.
pub fn chunk_pages(file: &Path, text: &str, max_chars: usize) -> Vec<Chunk> {
    text.split('\x0c')
        .enumerate()
        .flat_map(|(index, page)| {
            chunk_text(file, page, max_chars)
                .into_iter()
                .map(move |chunk| Chunk {
                    location: Location::Page(index as u32 + 1),
                    ..chunk
                })
        })
        .filter(|chunk| !chunk.text.trim().is_empty())
        .collect()
}

/// Exhaustive nearest-neighbour search: the exact baseline for approximate stores.
#[derive(Default)]
pub struct ExactStore {
    vectors: Vec<(VectorId, Embedding)>,
}

impl VectorStore for ExactStore {
    fn add(&mut self, id: VectorId, embedding: &Embedding) -> Result<()> {
        self.vectors.push((id, embedding.clone()));
        Ok(())
    }

    fn remove(&mut self, id: VectorId) -> Result<()> {
        self.vectors.retain(|(stored, _)| *stored != id);
        Ok(())
    }

    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<Neighbor>> {
        let mut neighbors: Vec<Neighbor> = self
            .vectors
            .iter()
            .map(|(id, embedding)| Neighbor {
                id: *id,
                similarity: query.similarity(embedding),
            })
            .collect();
        neighbors.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
        neighbors.truncate(limit);
        Ok(neighbors)
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

/// Chunks and their Embeddings in a VectorStore, searchable in File view.
pub struct RetrievalIndex<S> {
    chunks: Vec<Chunk>,
    store: S,
}

impl<S: VectorStore> RetrievalIndex<S> {
    /// `embeddings[i]` is the Embedding of `chunks[i]`.
    pub fn build(chunks: Vec<Chunk>, embeddings: &[Embedding], mut store: S) -> Result<Self> {
        for (id, embedding) in embeddings.iter().enumerate() {
            store.add(id as VectorId, embedding)?;
        }
        Ok(Self { chunks, store })
    }

    /// Whether any Chunk of `file` is in the index.
    pub fn contains(&self, file: &Path) -> bool {
        self.chunks.iter().any(|chunk| chunk.file == file)
    }

    /// File-view Results for a Query Embedding: one per file, ranked by its best Chunk.
    pub fn search(&self, query: &Embedding, files: usize) -> Result<Vec<FileResult>> {
        let mut results: Vec<FileResult> = Vec::new();
        for neighbor in self.store.search(query, self.chunks.len())? {
            let chunk = &self.chunks[neighbor.id as usize];
            if !results.iter().any(|result| result.file == chunk.file) {
                results.push(FileResult {
                    file: chunk.file.clone(),
                    snippet: Some(chunk.location.clone()),
                });
            }
        }
        results.truncate(files);
        Ok(results)
    }
}

/// Scores semantic-only retrieval of `index` over the Benchmark Queries with a Target file in it.
pub fn evaluate<S: VectorStore>(
    benchmark: &Benchmark,
    index: &RetrievalIndex<S>,
    embedder: &dyn EmbeddingProvider,
) -> Result<Scores> {
    let rankings = benchmark
        .queries()
        .iter()
        .filter(|query| {
            query
                .targets
                .iter()
                .any(|target| index.contains(&target.file))
        })
        .map(|query| {
            let results = index.search(&embedder.embed_query(&query.text)?, RESULTS_PER_QUERY)?;
            Ok(FileRanking::new(query, &results))
        })
        .collect::<Result<_>>()?;
    Ok(Scores::new(rankings))
}
