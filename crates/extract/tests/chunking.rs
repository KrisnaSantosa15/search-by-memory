use sbm_core::{Location, Section};
use sbm_extract::{Chunk, chunk_sections};

fn numbered_lines(count: u32) -> String {
    (1..=count).map(|n| format!("line {n:02} a\n")).collect()
}

#[test]
fn long_text_is_split_into_overlapping_chunks_with_line_ranges() {
    let section = Section {
        location: Location::Lines { start: 1, end: 10 },
        text: numbered_lines(10), // ten lines of 10 characters
    };

    let chunks = chunk_sections(&[section], 40, 10);

    let ranges: Vec<Location> = chunks
        .into_iter()
        .map(|chunk: Chunk| chunk.location)
        .collect();
    assert_eq!(
        ranges,
        [
            Location::Lines { start: 1, end: 4 },
            Location::Lines { start: 4, end: 7 },
            Location::Lines { start: 7, end: 10 },
        ]
    );
}

#[test]
fn page_chunks_keep_their_page_and_blank_pages_have_none() {
    let sections = [
        Section {
            location: Location::Page(1),
            text: "Invoice for March rent\n".into(),
        },
        Section {
            location: Location::Page(2),
            text: "\n  \n".into(),
        },
        Section {
            location: Location::Page(3),
            text: "Paid by bank transfer\n".into(),
        },
    ];

    let chunks = chunk_sections(&sections, 1000, 100);

    let pages: Vec<Location> = chunks.into_iter().map(|chunk| chunk.location).collect();
    assert_eq!(pages, [Location::Page(1), Location::Page(3)]);
}
