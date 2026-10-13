use std::path::Path;

use sbm_bench::chunk_text;
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
