use std::fs;
use std::path::Path;

use sbm_index::Index;
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
