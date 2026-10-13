//! Command-line indexing and search over the Name Index and Content Index (M2).

use std::path::{Path, PathBuf};

use sbm_core::{EmbeddingProvider, Result};
use sbm_index::{Exclusion, Index, ScanChanges, SearchResult, View};
use sbm_store_usearch::{Precision, UsearchStore};

const DATABASE: &str = "index.sqlite";
const VECTORS: &str = "vectors.usearch";

/// What indexing a folder did.
#[derive(Debug, Default)]
pub struct IndexReport {
    pub changes: ScanChanges,
    /// Files whose content could not be indexed, with the reason; they stay in the Name Index.
    pub failed: Vec<(PathBuf, String)>,
}

/// Brings the indexes in `data_dir` in line with `folder`: scans it, embeds
/// added and changed files, drops content no file uses, and saves the vectors.
/// Files that fail to index are reported, not fatal.
pub fn index_folder(
    data_dir: &Path,
    folder: &Path,
    embedder: &dyn EmbeddingProvider,
) -> Result<IndexReport> {
    std::fs::create_dir_all(data_dir)?;
    let mut index = Index::open(&data_dir.join(DATABASE))?;
    let mut store = UsearchStore::new(embedder.dimensions(), Precision::F16)?;
    index.rebuild_vectors(&mut store)?;
    let changes = index.scan(folder, &Exclusion::defaults())?;
    let mut failed = Vec::new();
    for path in changes.added.iter().chain(&changes.changed) {
        // A bad file must not stop indexing (FR-03).
        if let Err(error) = index.index_content(path, embedder, &mut store) {
            failed.push((path.clone(), error.to_string()));
        }
    }
    index.remove_unreferenced_content(&mut store)?;
    store.save(&data_dir.join(VECTORS))?;
    Ok(IndexReport { changes, failed })
}

/// Results for a Query against the indexes in `data_dir`.
pub fn search(
    data_dir: &Path,
    query: &str,
    embedder: &dyn EmbeddingProvider,
    view: View,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let vectors = data_dir.join(VECTORS);
    if !vectors.exists() {
        return Ok(Vec::new());
    }
    let index = Index::open(&data_dir.join(DATABASE))?;
    let store = UsearchStore::open_view(&vectors)?;
    index.search(query, embedder, &store, view, limit)
}
