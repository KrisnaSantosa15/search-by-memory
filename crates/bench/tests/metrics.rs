use std::path::PathBuf;

use sbm_core::Location;

use sbm_bench::{Benchmark, FileRanking, FileResult, Query, Scores};

fn query_targeting(files: &[&str]) -> Query {
    let targets: String = files
        .iter()
        .map(|file| format!("[[query.target]]\nfile = \"{file}\"\n"))
        .collect();
    let source = format!("[[query]]\nid = \"q\"\ntext = \"t\"\nlanguage = \"en\"\n{targets}");
    Benchmark::parse(&source).unwrap().queries()[0].clone()
}

fn ranked(files: &[&str]) -> Vec<FileResult> {
    files
        .iter()
        .map(|file| FileResult {
            file: PathBuf::from(file),
            snippet: None,
        })
        .collect()
}

#[test]
fn recall_at_k_is_the_share_of_queries_whose_target_file_is_in_the_top_k() {
    let query = query_targeting(&["a.pdf"]);
    let scores = Scores::new(vec![
        FileRanking::new(&query, &ranked(&["a.pdf", "b.pdf"])),
        FileRanking::new(&query, &ranked(&["b.pdf", "c.pdf", "a.pdf"])),
        FileRanking::new(&query, &ranked(&["b.pdf"])),
        FileRanking::new(&query, &ranked(&[])),
    ]);

    assert_eq!(scores.recall_at(1), 0.25);
    assert_eq!(scores.recall_at(3), 0.5);
}

#[test]
fn mean_reciprocal_rank_averages_one_over_the_first_target_rank() {
    let query = query_targeting(&["a.pdf"]);
    let scores = Scores::new(vec![
        FileRanking::new(&query, &ranked(&["a.pdf"])),
        FileRanking::new(&query, &ranked(&["b.pdf", "a.pdf"])),
        FileRanking::new(&query, &ranked(&["b.pdf", "c.pdf", "d.pdf", "a.pdf"])),
        FileRanking::new(&query, &ranked(&["b.pdf"])),
    ]);

    assert_eq!(
        scores.mean_reciprocal_rank(),
        (1.0 + 0.5 + 0.25 + 0.0) / 4.0
    );
}

#[test]
fn ndcg_at_k_rewards_every_distinct_target_file_by_its_rank() {
    // Two Targets in a.pdf (e.g. different pages) count as one relevant file.
    let query = query_targeting(&["a.pdf", "a.pdf", "b.pdf"]);
    let scores = Scores::new(vec![
        FileRanking::new(&query, &ranked(&["a.pdf", "x.pdf", "b.pdf"])),
        FileRanking::new(&query, &ranked(&["x.pdf", "y.pdf"])),
    ]);

    let ideal = 1.0 + 1.0 / 3f64.log2();
    let found = 1.0 + 1.0 / 4f64.log2();
    assert!((scores.ndcg_at(10) - (found / ideal) / 2.0).abs() < 1e-12);
    assert!((scores.ndcg_at(2) - (1.0 / ideal) / 2.0).abs() < 1e-12);
}

#[test]
fn location_accuracy_is_the_share_of_found_targets_whose_snippet_overlaps_the_target() {
    let query = Benchmark::parse(
        r#"
        [[query]]
        id = "q"
        text = "t"
        language = "en"

        [[query.target]]
        file = "a.pdf"
        page = 3
        "#,
    )
    .unwrap()
    .queries()[0]
        .clone();
    let shown = |page: Option<u32>| {
        vec![FileResult {
            file: PathBuf::from("a.pdf"),
            snippet: page.map(Location::Page),
        }]
    };
    let scores = Scores::new(vec![
        FileRanking::new(&query, &shown(Some(3))),
        FileRanking::new(&query, &shown(Some(4))),
        FileRanking::new(&query, &shown(None)),
        // Not found: excluded, Location Accuracy only judges correctly found files.
        FileRanking::new(&query, &ranked(&["b.pdf"])),
    ]);

    assert_eq!(scores.location_accuracy(), 1.0 / 3.0);
}
