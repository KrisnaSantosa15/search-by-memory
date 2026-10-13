use std::fs;
use std::path::Path;

use sbm_core::{Embedding, EmbeddingProvider, Result, VectorStore};
use sbm_index::{Index, MatchReason, View};
use sbm_store_usearch::{Precision, UsearchStore};
use tempfile::TempDir;

const DIMENSIONS: usize = 64;

/// Stands in for the embedding model: texts sharing words get similar Embeddings.
struct BagOfWords;

impl BagOfWords {
    fn embed(text: &str) -> Embedding {
        let mut values = vec![0.0; DIMENSIONS];
        for word in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            let hash = word
                .to_lowercase()
                .bytes()
                .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
            values[(hash % DIMENSIONS as u64) as usize] += 1.0;
        }
        Embedding::new(values).truncated(DIMENSIONS)
    }
}

impl EmbeddingProvider for BagOfWords {
    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
    fn embed_query(&self, query: &str) -> Result<Embedding> {
        Ok(Self::embed(query))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        Ok(chunks.iter().map(|chunk| Self::embed(chunk)).collect())
    }
}

fn write(root: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

fn store() -> UsearchStore {
    UsearchStore::new(DIMENSIONS, Precision::F16).unwrap()
}

#[test]
fn file_content_is_found_by_meaning() {
    let folder = TempDir::new().unwrap();
    let recipe = write(
        folder.path(),
        "kitchen/a.txt",
        "slow cooked beef in coconut milk and chili\n",
    );
    let other = write(
        folder.path(),
        "garage/b.txt",
        "change the engine oil every five thousand miles\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&recipe, &BagOfWords, &mut store)
        .unwrap();
    index
        .index_content(&other, &BagOfWords, &mut store)
        .unwrap();

    let results = index
        .search(
            "beef with coconut milk",
            &BagOfWords,
            &store,
            View::File,
            20,
        )
        .unwrap();

    assert_eq!(results[0].path, recipe);
    assert_eq!(results[0].match_reason, MatchReason::Content);
}

/// Knows no meaning at all: every text gets the same Embedding.
struct Blind;

impl EmbeddingProvider for Blind {
    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
    fn embed_query(&self, _query: &str) -> Result<Embedding> {
        Ok(Embedding::new(vec![1.0; DIMENSIONS]).truncated(DIMENSIONS))
    }
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        chunks.iter().map(|chunk| self.embed_query(chunk)).collect()
    }
}

#[test]
fn file_content_is_found_by_an_exact_keyword() {
    let folder = TempDir::new().unwrap();
    for n in 0..30 {
        let text = if n == 17 {
            "payment reference INV-2024-0371 received\n".to_string()
        } else {
            format!("unrelated note number {n}\n")
        };
        write(folder.path(), &format!("docs/{n}.txt"), &text);
    }
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    for n in 0..30 {
        let path = folder.path().join(format!("docs/{n}.txt"));
        index.index_content(&path, &Blind, &mut store).unwrap();
    }

    let results = index
        .search("INV-2024-0371", &Blind, &store, View::File, 5)
        .unwrap();

    assert_eq!(results[0].path, folder.path().join("docs/17.txt"));
}

#[test]
fn copy_of_indexed_content_reuses_its_chunks_without_embedding() {
    let folder = TempDir::new().unwrap();
    let text = "slow cooked beef in coconut milk and chili\n";
    let original = write(folder.path(), "recipes/rendang.txt", text);
    let copy = write(folder.path(), "backup/rendang (1).txt", text);
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&original, &BagOfWords, &mut store)
        .unwrap();
    let vectors = store.len();

    index.index_content(&copy, &Blind, &mut store).unwrap();

    assert_eq!(store.len(), vectors);
}

#[test]
fn copies_are_one_result_in_file_view() {
    let folder = TempDir::new().unwrap();
    let text = "slow cooked beef in coconut milk and chili\n";
    let original = write(folder.path(), "recipes/rendang.txt", text);
    let copy = write(folder.path(), "backup/rendang (1).txt", text);
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&original, &BagOfWords, &mut store)
        .unwrap();
    index.index_content(&copy, &BagOfWords, &mut store).unwrap();

    let results = index
        .search("beef in coconut milk", &BagOfWords, &store, View::File, 20)
        .unwrap();

    assert_eq!(results.len(), 1);
    let mut paths = vec![results[0].path.clone()];
    paths.extend(results[0].copies.clone());
    paths.sort();
    let mut expected = vec![original, copy];
    expected.sort();
    assert_eq!(paths, expected);
}

#[test]
fn moved_file_reuses_existing_chunks_without_reembedding() {
    let folder = TempDir::new().unwrap();
    let before = write(
        folder.path(),
        "inbox/rendang.txt",
        "slow cooked beef in coconut milk and chili\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&before, &BagOfWords, &mut store)
        .unwrap();
    let vectors = store.len();

    let after = folder.path().join("recipes/rendang.txt");
    fs::create_dir_all(after.parent().unwrap()).unwrap();
    fs::rename(&before, &after).unwrap();
    let changes = index.scan(folder.path(), &[]).unwrap();
    for added in &changes.added {
        index.index_content(added, &Blind, &mut store).unwrap();
    }
    let results = index
        .search("beef in coconut milk", &BagOfWords, &store, View::File, 20)
        .unwrap();

    assert_eq!(store.len(), vectors);
    assert_eq!(results[0].path, after);
    assert!(results[0].copies.is_empty());
}

#[test]
fn edited_file_is_found_by_its_new_content_and_old_content_is_removed() {
    let folder = TempDir::new().unwrap();
    let notes = write(
        folder.path(),
        "notes.txt",
        "slow cooked beef in coconut milk\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&notes, &BagOfWords, &mut store)
        .unwrap();

    fs::write(&notes, "change the engine oil every five thousand miles\n").unwrap();
    let changes = index.scan(folder.path(), &[]).unwrap();
    for changed in &changes.changed {
        index
            .index_content(changed, &BagOfWords, &mut store)
            .unwrap();
    }
    index.remove_unreferenced_content(&mut store).unwrap();
    let results = index
        .search("engine oil", &BagOfWords, &store, View::File, 20)
        .unwrap();

    assert_eq!(store.len(), 1);
    assert_eq!(results.len(), 1);
    assert!(
        results[0]
            .snippet
            .as_deref()
            .unwrap()
            .contains("engine oil")
    );
}

#[test]
fn name_and_content_results_are_merged_with_their_match_reason() {
    let folder = TempDir::new().unwrap();
    let photo = write(
        folder.path(),
        "photos/rendang-dinner.jpg",
        "binary image data",
    );
    let recipe = write(
        folder.path(),
        "recipes/rendang.txt",
        "rendang is beef slow cooked in coconut milk\n",
    );
    let other = write(
        folder.path(),
        "recipes/soup.txt",
        "a clear soup, unlike rendang, uses no coconut milk\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    for path in [&photo, &recipe, &other] {
        index.index_content(path, &BagOfWords, &mut store).unwrap();
    }

    let results = index
        .search("rendang", &BagOfWords, &store, View::File, 20)
        .unwrap();

    let reasons: Vec<(&Path, MatchReason)> = results
        .iter()
        .map(|result| (result.path.as_path(), result.match_reason))
        .collect();
    assert_eq!(reasons[0], (recipe.as_path(), MatchReason::Both));
    assert!(reasons.contains(&(photo.as_path(), MatchReason::Name)));
    assert!(reasons.contains(&(other.as_path(), MatchReason::Content)));
}

#[test]
fn exact_filename_match_is_placed_at_the_top() {
    let folder = TempDir::new().unwrap();
    for n in 0..5 {
        write(
            folder.path(),
            &format!("notes/{n}.txt"),
            "budget budget budget for the trip\n",
        );
    }
    let sheet = write(folder.path(), "finance/Budget.xlsx", "binary spreadsheet");
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    for n in 0..5 {
        let path = folder.path().join(format!("notes/{n}.txt"));
        index.index_content(&path, &BagOfWords, &mut store).unwrap();
    }

    let results = index
        .search("budget.xlsx", &BagOfWords, &store, View::File, 20)
        .unwrap();

    assert_eq!(results[0].path, sheet);
}

/// Twelve paragraphs about one topic, long enough for several Chunks.
fn long_recipe() -> String {
    (0..12)
        .map(|n| {
            format!(
                "step {n}: stir the rendang slowly so the coconut milk does not burn, {}\n",
                "and keep the heat low ".repeat(5)
            )
        })
        .collect()
}

#[test]
fn file_view_shows_each_file_once_however_many_chunks_match() {
    let folder = TempDir::new().unwrap();
    let recipe = write(folder.path(), "recipes/rendang.txt", &long_recipe());
    let other = write(
        folder.path(),
        "recipes/gulai.txt",
        "gulai also uses coconut milk\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&recipe, &BagOfWords, &mut store)
        .unwrap();
    index
        .index_content(&other, &BagOfWords, &mut store)
        .unwrap();
    assert!(store.len() > 3, "the recipe should span several Chunks");

    let results = index
        .search("coconut milk", &BagOfWords, &store, View::File, 20)
        .unwrap();

    let paths: Vec<&Path> = results.iter().map(|result| result.path.as_path()).collect();
    assert_eq!(paths.len(), 2);
    assert!(paths.contains(&recipe.as_path()) && paths.contains(&other.as_path()));
}

#[test]
fn passage_view_shows_each_content_match_and_no_name_only_files() {
    let folder = TempDir::new().unwrap();
    let recipe = write(folder.path(), "recipes/rendang.txt", &long_recipe());
    write(
        folder.path(),
        "photos/coconut-milk.jpg",
        "binary image data",
    );
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    index
        .index_content(&recipe, &BagOfWords, &mut store)
        .unwrap();

    let results = index
        .search("coconut milk", &BagOfWords, &store, View::Passage, 20)
        .unwrap();

    assert_eq!(results.len(), store.len());
    assert!(results.iter().all(|result| result.path == recipe));
    assert!(
        results
            .iter()
            .all(|result| result.match_reason == MatchReason::Content)
    );
}
