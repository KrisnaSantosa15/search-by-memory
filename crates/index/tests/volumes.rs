//! The file catalog identifies files by volume ID plus a path relative to the
//! volume, so a drive that returns under another letter keeps its files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sbm_core::{Embedding, EmbeddingProvider, Result, Volume, Volumes};
use sbm_index::{Index, ScanChanges, View};
use sbm_store_usearch::{Precision, UsearchStore};
use tempfile::TempDir;

/// Stands in for the operating system's volume table: tests mount, move,
/// and disconnect volumes by ID.
#[derive(Clone, Default)]
struct FakeVolumes(Arc<Mutex<HashMap<String, PathBuf>>>);

impl FakeVolumes {
    fn mount(&self, id: &str, root: &Path) {
        self.0
            .lock()
            .unwrap()
            .insert(id.to_string(), root.to_path_buf());
    }
}

impl Volumes for FakeVolumes {
    fn volume_of(&self, path: &Path) -> Result<Volume> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, root)| path.starts_with(root))
            .max_by_key(|(_, root)| root.components().count())
            .map(|(id, root)| Volume {
                id: id.clone(),
                root: root.clone(),
            })
            .ok_or_else(|| format!("no volume holds {}", path.display()).into())
    }

    fn mount_point(&self, id: &str) -> Option<PathBuf> {
        self.0.lock().unwrap().get(id).cloned()
    }
}

/// Stands in for the embedding model; every text gets the same Embedding.
struct SameEmbedding;

impl EmbeddingProvider for SameEmbedding {
    fn dimensions(&self) -> usize {
        8
    }
    fn embed_query(&self, _query: &str) -> Result<Embedding> {
        Ok(Embedding::new(vec![1.0; 8]).truncated(8))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        chunks.iter().map(|chunk| self.embed_query(chunk)).collect()
    }
}

fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"content").unwrap();
}

/// Moves everything on a volume to `to`, keeping modification times, as when
/// a drive comes back under another drive letter.
fn remount(volumes: &FakeVolumes, id: &str, from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), to.join(entry.file_name())).unwrap();
    }
    volumes.mount(id, to);
}

#[test]
fn drive_returning_under_another_letter_keeps_its_files() {
    let (e_drive, f_drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    touch(e_drive.path(), "reports/budget.xlsx");
    let volumes = FakeVolumes::default();
    volumes.mount("usb-stick", e_drive.path());
    let mut index = Index::open_in_memory()
        .unwrap()
        .with_volumes(volumes.clone());
    index.scan(e_drive.path(), &[]).unwrap();

    remount(&volumes, "usb-stick", e_drive.path(), f_drive.path());
    let changes = index.scan(f_drive.path(), &[]).unwrap();

    assert_eq!(changes, ScanChanges::default());
    let matches = index.search_names("budget", 20).unwrap();
    let paths: Vec<_> = matches.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(paths, [f_drive.path().join("reports/budget.xlsx")]);
}

#[test]
fn content_match_on_a_remounted_drive_points_at_its_new_location() {
    let (e_drive, f_drive) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    touch(e_drive.path(), "letters/tax.txt");
    let volumes = FakeVolumes::default();
    volumes.mount("usb-stick", e_drive.path());
    let mut index = Index::open_in_memory()
        .unwrap()
        .with_volumes(volumes.clone());
    let mut store = UsearchStore::new(8, Precision::F16).unwrap();
    index.scan(e_drive.path(), &[]).unwrap();
    index
        .index_content(
            &e_drive.path().join("letters/tax.txt"),
            &SameEmbedding,
            &mut store,
        )
        .unwrap();

    remount(&volumes, "usb-stick", e_drive.path(), f_drive.path());
    index.scan(f_drive.path(), &[]).unwrap();
    let results = index
        .search("content", &SameEmbedding, &store, View::Passage, 20)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, f_drive.path().join("letters/tax.txt"));
}
