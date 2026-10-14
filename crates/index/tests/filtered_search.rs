//! Searching only some files, e.g. code: the filter applies before Results are
//! counted and before Weaker Matches are decided.

mod common;

use std::path::{Path, PathBuf};

use common::{BagOfWords, store, write};
use sbm_index::{Index, View};
use tempfile::TempDir;

fn is_code(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

#[test]
fn search_where_keeps_only_matching_files_and_still_fills_the_limit() {
    let folder = TempDir::new().unwrap();
    let mut paths: Vec<PathBuf> = Vec::new();
    for n in 0..3 {
        paths.push(write(
            folder.path(),
            &format!("notes/{n}.txt"),
            "retry with backoff after a failure\n",
        ));
    }
    for n in 0..2 {
        paths.push(write(
            folder.path(),
            &format!("src/{n}.rs"),
            &format!("fn retry_{n}() {{ backoff(); }} // retry later\n"),
        ));
    }
    let mut index = Index::open_in_memory().unwrap();
    let mut store = store();
    index.scan(folder.path(), &[]).unwrap();
    for path in &paths {
        index
            .index_content(path, &BagOfWords::default(), &mut store)
            .unwrap();
    }

    let results = index
        .search_where(
            "retry with backoff",
            &BagOfWords::default(),
            &store,
            View::File,
            2,
            &is_code,
        )
        .unwrap();

    let found: Vec<&Path> = results.iter().map(|result| result.path.as_path()).collect();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|path| is_code(path)));
    assert!(results.iter().all(|result| !result.weaker));
}
