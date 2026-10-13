//! Online-only Files: in the Name Index, never opened for content until downloaded (FR-03).

mod common;

use common::{BagOfWords, FakeFileAvailability, store, write};
use sbm_index::{Index, MatchReason, View};
use tempfile::TempDir;

#[test]
fn online_only_file_is_in_name_index_but_not_content_index() {
    let folder = TempDir::new().unwrap();
    let report = write(
        folder.path(),
        "tax-report.txt",
        "annual income and deductions\n",
    );
    let availability = FakeFileAvailability::default();
    availability.move_to_cloud(&report);
    let mut index = Index::open_in_memory()
        .unwrap()
        .with_file_availability(availability);
    let (embedder, mut store) = (BagOfWords::default(), store());
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();

    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    let results = index
        .search("tax report", &embedder, &store, View::File, 20)
        .unwrap();
    assert_eq!(results[0].path, report);
    assert_eq!(results[0].match_reason, MatchReason::Name);
    assert_eq!(embedder.embedded(), 0);
}
