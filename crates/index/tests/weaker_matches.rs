//! Weaker Matches: Results related to the Query only by meaning, and much less
//! so than the best ones, are flagged so the launcher can fold them away.

mod common;

use std::path::PathBuf;

use common::{BagOfWords, store, write};
use sbm_index::{Index, SearchResult, View};
use sbm_store_usearch::UsearchStore;
use tempfile::TempDir;

/// Indexes `files` (relative path, content) under a new folder.
fn indexed(files: &[(&str, &str)]) -> (TempDir, Index, UsearchStore, Vec<PathBuf>) {
    let folder = TempDir::new().unwrap();
    let paths: Vec<PathBuf> = files
        .iter()
        .map(|(relative, content)| write(folder.path(), relative, content))
        .collect();
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    for path in &paths {
        index
            .index_content(path, &BagOfWords::default(), &mut store)
            .unwrap();
    }
    (folder, index, store, paths)
}

fn search(index: &Index, store: &UsearchStore, query: &str) -> Vec<SearchResult> {
    index
        .search(query, &BagOfWords::default(), store, View::File, 20)
        .unwrap()
}

fn weaker(results: &[SearchResult], path: &PathBuf) -> bool {
    results
        .iter()
        .find(|result| &result.path == path)
        .unwrap_or_else(|| panic!("{} not found", path.display()))
        .weaker
}

#[test]
fn result_related_only_by_meaning_and_far_below_the_best_is_weaker() {
    let (_folder, index, store, paths) = indexed(&[
        (
            "kitchen/a.txt",
            "slow cooked beef in coconut milk and chili\n",
        ),
        (
            "garage/b.txt",
            "change the engine oil every five thousand miles\n",
        ),
    ]);

    let results = search(&index, &store, "beef coconut chili");

    assert!(!weaker(&results, &paths[0]));
    assert!(weaker(&results, &paths[1]));
}

#[test]
fn result_with_the_query_words_in_it_is_never_weaker() {
    let long_note = "beef coconut chili \
        plus a long shopping list of bread butter apples onions garlic rice flour sugar \
        salt pepper soap tissues batteries candles matches tape glue string nails";
    let (_folder, index, store, paths) = indexed(&[
        ("kitchen/a.txt", "beef coconut chili\n"),
        ("notes/list.txt", long_note),
    ]);

    let results = search(&index, &store, "beef coconut chili");

    assert!(!weaker(&results, &paths[1]));
}
