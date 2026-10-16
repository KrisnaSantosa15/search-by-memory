//! How much a location would add to the index, before it is chosen.

use std::fs;
use std::path::Path;

use sbm_index::{Estimate, Exclusion, estimate};
use tempfile::TempDir;

fn write(root: &Path, relative: &str, bytes: usize) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![b'x'; bytes]).unwrap();
}

#[test]
fn estimate_counts_the_files_and_bytes_that_would_be_indexed() {
    let folder = TempDir::new().unwrap();
    write(folder.path(), "Documents/report.pdf", 1_000);
    write(folder.path(), "Pictures/beach.jpg", 3_000);
    write(folder.path(), "code/app/node_modules/lib.js", 50_000);
    write(folder.path(), "AppData/Local/cache.bin", 90_000);
    let exclusions = [
        Exclusion::defaults(),
        vec![Exclusion::Folder(folder.path().join("AppData"))],
    ]
    .concat();

    let found = estimate(folder.path(), &exclusions);

    assert_eq!(
        found,
        Estimate {
            files: 2,
            bytes: 4_000
        }
    );
}
