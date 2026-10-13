use std::fs;
use std::path::Path;

use sbm_change_usn::UsnJournal;
use sbm_cli::{accelerator_option, exclude, folders, index, remove_folder, search};
use sbm_core::{Embedding, EmbeddingProvider, Result};
use sbm_embed_onnx::Accelerator;
use sbm_index::{Exclusion, View};
use tempfile::TempDir;

const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar Embeddings.
struct BagOfWords;

impl EmbeddingProvider for BagOfWords {
    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
    fn embed_query(&self, query: &str) -> Result<Embedding> {
        let mut values = vec![0.0; DIMENSIONS];
        for word in query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            let hash = word
                .to_lowercase()
                .bytes()
                .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
            values[(hash % DIMENSIONS as u64) as usize] += 1.0;
        }
        Ok(Embedding::new(values).truncated(DIMENSIONS))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        chunks.iter().map(|chunk| self.embed_query(chunk)).collect()
    }
}

fn write(root: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn indexed_folder_is_searchable_from_a_later_run() {
    let folder = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let recipe = write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk and chili\n",
    );
    write(
        folder.path(),
        "garage/b.txt",
        "change the engine oil every five thousand miles\n",
    );

    index(
        data.path(),
        &[folder.path().to_path_buf()],
        &BagOfWords,
        &UsnJournal,
    )
    .unwrap();
    let results = search(
        data.path(),
        "beef with coconut milk",
        &BagOfWords,
        View::File,
        20,
    )
    .unwrap();

    assert_eq!(results[0].path, recipe);
}

#[test]
fn search_before_anything_is_indexed_has_no_results() {
    let data = TempDir::new().unwrap();

    let results = search(data.path(), "anything", &BagOfWords, View::File, 20).unwrap();

    assert!(results.is_empty());
}

#[test]
fn unreadable_file_is_reported_and_the_rest_still_indexed() {
    let folder = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    let broken = write(folder.path(), "scans/broken.pdf", "%PDF-1.4 truncated");
    let recipe = write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk and chili\n",
    );

    let report = index(
        data.path(),
        &[folder.path().to_path_buf()],
        &BagOfWords,
        &UsnJournal,
    )
    .unwrap();
    let results = search(
        data.path(),
        "beef with coconut milk",
        &BagOfWords,
        View::File,
        20,
    )
    .unwrap();

    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, broken);
    assert_eq!(results[0].path, recipe);
}

#[test]
fn terminal_control_sequences_in_file_text_are_neutralised_for_display() {
    // File content could otherwise clear the screen or forge output in the terminal.
    let shown = sbm_cli::printable("total \u{1b}[2J\u{1b}]0;pwned\u{7} due\r\n");

    assert!(!shown.chars().any(|c| c.is_control()), "{shown:?}");
    assert!(shown.contains("total") && shown.contains("due"));
}

#[test]
fn bidirectional_and_invisible_formatting_characters_are_neutralised_for_display() {
    // Right-to-left overrides can make "invoice\u{202e}fdp.exe" display as "invoiceexe.pdf".
    let shown =
        sbm_cli::printable("invoice\u{202e}fdp.exe \u{2066}x\u{2069} zero\u{200b}width \u{feff}");

    for hidden in ['\u{202e}', '\u{2066}', '\u{2069}', '\u{200b}', '\u{feff}'] {
        assert!(!shown.contains(hidden), "{shown:?} still has {hidden:?}");
    }
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test]
fn named_accelerator_is_tried_before_falling_back_to_cpu() {
    let (accelerators, rest) =
        accelerator_option(&args(&["search", "--accelerator", "cuda", "tax", "letter"]))
            .expect("option parses");

    assert_eq!(accelerators, [Accelerator::Cuda, Accelerator::Cpu]);
    assert_eq!(rest, args(&["search", "tax", "letter"]));
}

#[test]
fn without_accelerator_option_the_default_order_is_tried() {
    let (accelerators, rest) =
        accelerator_option(&args(&["index", "Documents"])).expect("option parses");

    assert_eq!(accelerators, Accelerator::default_order());
    assert_eq!(rest, args(&["index", "Documents"]));
}

#[test]
fn cpu_named_as_accelerator_is_tried_once() {
    let (accelerators, _) =
        accelerator_option(&args(&["index", "--accelerator", "cpu", "Documents"]))
            .expect("option parses");

    assert_eq!(accelerators, [Accelerator::Cpu]);
}

#[test]
fn unknown_or_missing_accelerator_name_is_an_error() {
    assert!(accelerator_option(&args(&["index", "--accelerator", "tpu", "Documents"])).is_err());
    assert!(accelerator_option(&args(&["index", "--accelerator"])).is_err());
}

#[test]
fn indexed_folders_are_rescanned_by_later_runs_without_naming_them() {
    let (folder, data) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk\n",
    );
    index(
        data.path(),
        &[folder.path().to_path_buf()],
        &BagOfWords,
        &UsnJournal,
    )
    .unwrap();
    let later = write(folder.path(), "garage/b.txt", "change the engine oil\n");

    let report = index(data.path(), &[], &BagOfWords, &UsnJournal).unwrap();
    let results = search(data.path(), "engine oil", &BagOfWords, View::File, 20).unwrap();

    assert_eq!(report.changes.added, [later.clone()]);
    assert_eq!(results[0].path, later);
}

#[test]
fn removed_folder_is_no_longer_listed_or_searched() {
    let (folder, data) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk\n",
    );
    index(
        data.path(),
        &[folder.path().to_path_buf()],
        &BagOfWords,
        &UsnJournal,
    )
    .unwrap();
    assert_eq!(folders(data.path()).unwrap().len(), 1);

    remove_folder(data.path(), folder.path(), DIMENSIONS).unwrap();

    assert_eq!(folders(data.path()).unwrap(), []);
    let results = search(data.path(), "beef coconut", &BagOfWords, View::File, 20).unwrap();
    assert_eq!(results, []);
}

#[test]
fn excluded_files_leave_the_results_on_the_next_index_run() {
    let (folder, data) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    write(
        folder.path(),
        "private/diary.txt",
        "dear diary, coconut milk again\n",
    );
    index(
        data.path(),
        &[folder.path().to_path_buf()],
        &BagOfWords,
        &UsnJournal,
    )
    .unwrap();

    exclude(
        data.path(),
        Exclusion::Folder(folder.path().join("private")),
    )
    .unwrap();
    index(data.path(), &[], &BagOfWords, &UsnJournal).unwrap();

    let results = search(data.path(), "diary coconut", &BagOfWords, View::File, 20).unwrap();
    assert_eq!(results, []);
}
