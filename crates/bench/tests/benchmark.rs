use std::path::PathBuf;

use sbm_bench::{Benchmark, Target};
use sbm_core::Location;

#[test]
fn query_with_page_target_parses_from_benchmark_file() {
    let benchmark = Benchmark::parse(
        r#"
        [[query]]
        id = "en-001"
        text = "paper arguing attention alone replaces recurrence"
        language = "en"

        [[query.target]]
        file = "papers/attention.pdf"
        page = 2
        "#,
    )
    .unwrap();

    let query = &benchmark.queries()[0];
    assert_eq!(query.id, "en-001");
    assert_eq!(query.text, "paper arguing attention alone replaces recurrence");
    assert_eq!(query.language, "en");
    assert_eq!(
        query.targets,
        vec![Target {
            file: PathBuf::from("papers/attention.pdf"),
            location: Location::Page(2),
        }]
    );
}

fn only_target(source: &str) -> Target {
    let benchmark = Benchmark::parse(source).unwrap();
    benchmark.queries()[0].targets[0].clone()
}

#[test]
fn line_range_target_parses_for_text_and_code() {
    let target = only_target(
        r#"
        [[query]]
        id = "en-002"
        text = "function that retries failed downloads"
        language = "en"

        [[query.target]]
        file = "repo/src/download.rs"
        lines = [10, 24]
        "#,
    );

    assert_eq!(target.location, Location::Lines { start: 10, end: 24 });
}
