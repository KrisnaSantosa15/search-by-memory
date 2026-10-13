//! Command-line indexing and search over the Name Index and Content Index (M2).

use std::path::Path;

use sbm_core::{EmbeddingProvider, Result};
use sbm_index::{Exclusion, Index, ScanChanges, SearchResult, View};
use sbm_store_usearch::{Precision, UsearchStore};

const DATABASE: &str = "index.sqlite";
const VECTORS: &str = "vectors.usearch";

/// Brings the indexes in `data_dir` in line with `folder`: scans it, embeds
/// added and changed files, drops content no file uses, and saves the vectors.
pub fn index_folder(
    data_dir: &Path,
    folder: &Path,
    embedder: &dyn EmbeddingProvider,
) -> Result<ScanChanges> {
    std::fs::create_dir_all(data_dir)?;
    let mut index = Index::open(&data_dir.join(DATABASE))?;
    let mut store = UsearchStore::new(embedder.dimensions(), Precision::F16)?;
    index.rebuild_vectors(&mut store)?;
    let changes = index.scan(folder, &Exclusion::defaults())?;
    for path in changes.added.iter().chain(&changes.changed) {
        index.index_content(path, embedder, &mut store)?;
    }
    index.remove_unreferenced_content(&mut store)?;
    store.save(&data_dir.join(VECTORS))?;
    Ok(changes)
}

/// Results for a Query against the indexes in `data_dir`.
pub fn search(
    data_dir: &Path,
    query: &str,
    embedder: &dyn EmbeddingProvider,
    view: View,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let index = Index::open(&data_dir.join(DATABASE))?;
    let store = UsearchStore::open_view(&data_dir.join(VECTORS))?;
    index.search(query, embedder, &store, view, limit)
}
