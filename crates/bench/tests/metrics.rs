use std::path::PathBuf;

use sbm_bench::{Benchmark, FileRanking, Query, Scores};

fn query_targeting(files: &[&str]) -> Query {
    let targets: String = files
        .iter()
        .map(|file| format!("[[query.target]]\nfile = \"{file}\"\n"))
        .collect();
    let source = format!("[[query]]\nid = \"q\"\ntext = \"t\"\nlanguage = \"en\"\n{targets}");
    Benchmark::parse(&source).unwrap().queries()[0].clone()
}

fn ranked(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
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

    assert_eq!(scores.mean_reciprocal_rank(), (1.0 + 0.5 + 0.25 + 0.0) / 4.0);
}
