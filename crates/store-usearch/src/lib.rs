//! VectorStore backed by USearch (ADR 0002).

use std::path::Path;

use sbm_core::{Embedding, Neighbor, Result, VectorId, VectorStore};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind, new_index};

/// Storage precision of vectors inside the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    F16,
    I8,
}

pub struct UsearchStore {
    index: Index,
}

impl UsearchStore {
    pub fn new(dimensions: usize, precision: Precision) -> Result<Self> {
        let options = IndexOptions {
            dimensions,
            metric: MetricKind::IP,
            quantization: match precision {
                Precision::F16 => ScalarKind::F16,
                Precision::I8 => ScalarKind::I8,
            },
            connectivity: 0,
            expansion_add: 0,
            expansion_search: 0,
            multi: false,
        };
        Ok(Self {
            index: new_index(&options)?,
        })
    }

    /// Writes the index to `path`.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.index.save(utf8(path)?)?;
        Ok(())
    }

    /// Opens a saved index as a read-only, memory-mapped view (low idle RAM).
    pub fn open_view(path: &Path) -> Result<Self> {
        Ok(Self {
            index: Index::restore_view(utf8(path)?)?,
        })
    }
}

fn utf8(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| format!("index path is not UTF-8: {}", path.display()).into())
}

impl VectorStore for UsearchStore {
    fn add(&mut self, id: VectorId, embedding: &Embedding) -> Result<()> {
        if self.index.size() >= self.index.capacity() {
            self.index.reserve((self.index.capacity() * 2).max(64))?;
        }
        self.index.add(id, embedding.as_slice())?;
        Ok(())
    }

    fn remove(&mut self, id: VectorId) -> Result<()> {
        self.index.remove(id)?;
        Ok(())
    }

    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<Neighbor>> {
        let matches = self.index.search(query.as_slice(), limit)?;
        Ok(matches
            .keys
            .iter()
            .zip(&matches.distances)
            .map(|(&id, &distance)| Neighbor {
                id,
                similarity: 1.0 - distance,
            })
            .collect())
    }

    fn len(&self) -> usize {
        self.index.size()
    }
}
