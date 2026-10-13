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
