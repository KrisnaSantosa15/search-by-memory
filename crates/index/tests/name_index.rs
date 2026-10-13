use std::fs;
use std::path::Path;

use sbm_index::{Exclusion, Index};
use tempfile::TempDir;

fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"content").unwrap();
}

#[test]
fn file_under_indexed_folder_is_found_by_its_filename() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "invoices/INV-2024-0371.pdf");
    touch(folder.path(), "notes/shopping.txt");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &[]).unwrap();
    let matches = index.search_names("shopping", 20).unwrap();

    let paths: Vec<_> = matches.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(paths, [folder.path().join("notes/shopping.txt")]);
}

#[test]
fn file_is_found_by_a_folder_name_in_its_path() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "invoices/2024/march.pdf");
    touch(folder.path(), "notes/shopping.txt");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &[]).unwrap();
    let matches = index.search_names("invoices", 20).unwrap();

    let paths: Vec<_> = matches.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(paths, [folder.path().join("invoices/2024/march.pdf")]);
}

#[test]
fn partially_typed_filename_finds_the_file() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "app/docker-compose.yml");
    touch(folder.path(), "app/Dockerfile");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &[]).unwrap();
    let matches = index.search_names("docker-comp", 20).unwrap();

    let paths: Vec<_> = matches.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(paths, [folder.path().join("app/docker-compose.yml")]);
}

#[test]
fn query_without_words_has_no_name_matches() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "notes/shopping.txt");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &[]).unwrap();

    assert!(index.search_names("  -_. ", 20).unwrap().is_empty());
}

#[test]
fn exact_filename_match_ranks_first() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "report/report-draft-report-notes.txt");
    touch(folder.path(), "report/old/report-report.md");
    touch(folder.path(), "archive/Report.pdf");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &[]).unwrap();
    let matches = index.search_names("report", 20).unwrap();

    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].path, folder.path().join("archive/Report.pdf"));
}

#[test]
fn files_beneath_default_exclusions_are_not_in_the_name_index() {
    let folder = TempDir::new().unwrap();
    touch(folder.path(), "app/src/index.js");
    touch(folder.path(), "app/node_modules/lodash/index.js");
    touch(folder.path(), "app/.git/index.js");
    touch(folder.path(), "tool/.venv/lib/index.js");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(folder.path(), &Exclusion::defaults()).unwrap();
    let matches = index.search_names("index.js", 20).unwrap();

    let paths: Vec<_> = matches.iter().map(|m| m.path.as_path()).collect();
    assert_eq!(paths, [folder.path().join("app/src/index.js")]);
}

#[test]
fn indexed_folder_is_scanned_even_when_its_own_name_is_excluded() {
    let parent = TempDir::new().unwrap();
    let folder = parent.path().join("target");
    touch(&folder, "release-notes.txt");
    let mut index = Index::open_in_memory().unwrap();

    index.scan(&folder, &Exclusion::defaults()).unwrap();

    assert_eq!(index.search_names("release-notes", 20).unwrap().len(), 1);
}
