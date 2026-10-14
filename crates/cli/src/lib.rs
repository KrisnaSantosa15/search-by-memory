//! Command-line indexing and search over the Name Index and Content Index (M2).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sbm_core::{ChangeSource, EmbeddingProvider, Result};
use sbm_embed_onnx::Accelerator;
use sbm_engine::{Engine, Status};
use sbm_index::{Exclusion, Index, IndexedFolder, SearchResult, View};
use sbm_store_usearch::{Precision, UsearchStore};

const DATABASE: &str = "index.sqlite";
const VECTORS: &str = "vectors.usearch";

/// What an index run left behind.
#[derive(Debug, Default)]
pub struct IndexReport {
    pub status: Status,
    /// Files whose content could not be indexed, with the reason; they stay in the Name Index.
    pub failed: Vec<(PathBuf, String)>,
}

/// Adds `new_folders` to the Indexed Folders in `data_dir`, applies what
/// `changes` reports since the last run (a rescan where its history is lost),
/// and indexes the content of added and changed files through the Engine: Live
/// Updates first, then newest first. Work left by an interrupted run is resumed.
/// Files that fail to index are reported, not fatal.
pub fn index(
    data_dir: &Path,
    new_folders: &[PathBuf],
    embedder: Arc<dyn EmbeddingProvider + Send + Sync>,
    changes: Box<dyn ChangeSource + Send>,
) -> Result<IndexReport> {
    let engine = Engine::start(data_dir, embedder, changes)?;
    for folder in new_folders {
        engine.add_indexed_folder(folder)?;
    }
    engine.wait_until_idle();
    let status = engine.status()?;
    if let Some(error) = &status.error {
        return Err(error.clone().into());
    }
    Ok(IndexReport {
        failed: engine.failed_files()?,
        status,
    })
}

/// The Indexed Folders in `data_dir`.
pub fn folders(data_dir: &Path) -> Result<Vec<IndexedFolder>> {
    Index::open(&data_dir.join(DATABASE))?.indexed_folders()
}

/// Forgets the Indexed Folder `folder` and removes its files from the indexes in
/// `data_dir`, whose vectors have `dimensions` dimensions.
pub fn remove_folder(data_dir: &Path, folder: &Path, dimensions: usize) -> Result<()> {
    let mut index = Index::open(&data_dir.join(DATABASE))?;
    let mut store = UsearchStore::new(dimensions, Precision::F16)?;
    index.rebuild_vectors(&mut store)?;
    index.remove_indexed_folder(folder, &mut store)?;
    store.save(&data_dir.join(VECTORS))
}

/// Saves an Exclusion in `data_dir`; the next index run drops the files it matches.
pub fn exclude(data_dir: &Path, exclusion: Exclusion) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    Index::open(&data_dir.join(DATABASE))?.add_exclusion(exclusion)
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

/// Takes `--accelerator <name>` out of `args`. Returns the Accelerators to try,
/// in order, and the remaining arguments. A named GPU falls back to CPU;
/// without the option, [`Accelerator::default_order`] is tried.
pub fn accelerator_option(args: &[String]) -> Result<(Vec<Accelerator>, Vec<String>)> {
    let mut rest = Vec::new();
    let mut accelerators = Accelerator::default_order().to_vec();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--accelerator" {
            let name = args.next().ok_or("--accelerator needs a name")?;
            accelerators = vec![name.parse()?];
            if accelerators[0] != Accelerator::Cpu {
                accelerators.push(Accelerator::Cpu);
            }
        } else {
            rest.push(arg.clone());
        }
    }
    Ok((accelerators, rest))
}

/// Text from files or paths made safe to print: control characters, which could
/// drive the terminal through escape sequences, and invisible formatting
/// characters, which could disguise text, become visible `\u{..}` escapes.
pub fn printable(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || is_hidden_format(c) {
                c.escape_unicode().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Invisible formatting characters (Unicode Cf) that reorder or hide text:
/// bidirectional marks, embeddings, overrides and isolates, zero-width
/// characters, and the byte order mark.
fn is_hidden_format(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}'
            | '\u{061c}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
    )
}
