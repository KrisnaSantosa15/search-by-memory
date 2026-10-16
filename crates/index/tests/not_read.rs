//! Why a file's content is not searched, for files in the Name Index.

mod common;

use common::{BagOfWords, FakeFileAvailability, store, write};
use sbm_core::Unreadable;
use sbm_index::{Index, NotRead};
use tempfile::TempDir;

#[test]
fn online_only_file_is_not_read_because_it_is_online_only() {
    let folder = TempDir::new().unwrap();
    let report = write(folder.path(), "tax-report.txt", "annual income\n");
    let notes = write(folder.path(), "notes.txt", "team meeting\n");
    let availability = FakeFileAvailability::default();
    availability.move_to_cloud(&report);
    let mut index = Index::open_in_memory()
        .unwrap()
        .with_file_availability(availability);
    let (embedder, mut store) = (BagOfWords::default(), store());
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    assert_eq!(index.not_read(&report).unwrap(), Some(NotRead::OnlineOnly));
    assert_eq!(index.not_read(&notes).unwrap(), None);
}

#[test]
fn file_of_a_format_that_is_not_read_says_so() {
    let folder = TempDir::new().unwrap();
    let photo = write(folder.path(), "holiday.jpg", "not text\n");
    let mut index = Index::open_in_memory().unwrap();
    let (embedder, mut store) = (BagOfWords::default(), store());
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    assert_eq!(index.not_read(&photo).unwrap(), Some(NotRead::Format));
}

#[test]
fn file_still_queued_for_content_is_not_read_yet() {
    let folder = TempDir::new().unwrap();
    let notes = write(folder.path(), "notes.txt", "team meeting\n");
    let mut index = Index::open_in_memory().unwrap();
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();

    assert_eq!(index.not_read(&notes).unwrap(), Some(NotRead::Pending));
}

#[test]
fn file_that_could_not_be_read_says_why() {
    let folder = TempDir::new().unwrap();
    let broken = write(folder.path(), "broken.docx", "not really a zip archive");
    let mut index = Index::open_in_memory().unwrap();
    let (embedder, mut store) = (BagOfWords::default(), store());
    index.add_indexed_folder(folder.path()).unwrap();
    index.scan_all().unwrap();
    index.run_jobs(&embedder, &mut store, usize::MAX).unwrap();

    assert_eq!(
        index.not_read(&broken).unwrap(),
        Some(NotRead::Failed(Unreadable::Damaged))
    );
}
