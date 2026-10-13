//! One index database opened by several connections, as the Engine does.

mod common;

use common::write;
use sbm_index::Index;
use tempfile::TempDir;

#[test]
fn index_opens_while_another_connection_changes_it() {
    let (data, folder) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    for n in 0..50 {
        write(folder.path(), &format!("note-{n}.txt"), "a note\n");
    }
    let database = data.path().join("index.sqlite");
    let mut scanner = Index::open(&database).unwrap();
    scanner.add_indexed_folder(folder.path()).unwrap();

    std::thread::scope(|scope| {
        let scanning = scope.spawn(|| {
            for _ in 0..50 {
                scanner.scan_all().unwrap();
            }
        });
        for _ in 0..50 {
            Index::open(&database).unwrap();
        }
        scanning.join().unwrap();
    });
}
