use std::path::Path;

use sbm_bench::{chunk_pages, chunk_text};
use sbm_core::Location;

#[test]
fn text_is_split_into_chunks_of_whole_lines_within_the_size_limit() {
    let text = "alpha alpha\nbravo bravo\ncharlie charlie\ndelta\n";

    let chunks = chunk_text(Path::new("a.txt"), text, 24);

    let pieces: Vec<(&str, Location)> = chunks
        .iter()
        .map(|chunk| (chunk.text.as_str(), chunk.location.clone()))
        .collect();
    assert_eq!(
        pieces,
        vec![
            (
                "alpha alpha\nbravo bravo\n",
                Location::Lines { start: 1, end: 2 }
            ),
            (
                "charlie charlie\ndelta\n",
                Location::Lines { start: 3, end: 4 }
            ),
        ]
    );
}

#[test]
fn paged_text_is_chunked_within_each_page_and_located_by_page() {
    // pdftotext separates pages with form feeds.
    let text = "cover page\n\x0clong page line one\nlong page line two\n\x0c";

    let chunks = chunk_pages(Path::new("a.pdf"), text, 20);

    let pieces: Vec<(&str, Location)> = chunks
        .iter()
        .map(|chunk| (chunk.text.as_str(), chunk.location.clone()))
        .collect();
    assert_eq!(
        pieces,
        vec![
            ("cover page\n", Location::Page(1)),
            ("long page line one\n", Location::Page(2)),
            ("long page line two\n", Location::Page(2)),
        ]
    );
}
