use std::path::PathBuf;
use std::time::Duration;

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

#[test]
fn time_range_target_parses_for_audio_and_video() {
    let target = only_target(
        r#"
        [[query]]
        id = "en-003"
        text = "clip where the rocket separates its first stage"
        language = "en"

        [[query.target]]
        file = "video/launch.webm"
        time = ["01:30", "1:02:10"]
        "#,
    );

    assert_eq!(
        target.location,
        Location::Time {
            start: Duration::from_secs(90),
            end: Duration::from_secs(3730),
        }
    );
}

#[test]
fn target_without_location_means_the_whole_file_for_images() {
    let target = only_target(
        r#"
        [[query]]
        id = "id-001"
        text = "foto candi dengan stupa berlubang saat matahari terbit"
        language = "id"

        [[query.target]]
        file = "images/borobudur.jpg"
        "#,
    );

    assert_eq!(target.location, Location::WholeFile);
}

fn with_target(target: &str) -> String {
    format!(
        r#"
        [[query]]
        id = "en-900"
        text = "a remembered passage"
        language = "en"

        [[query.target]]
        file = "papers/paper.pdf"
        {target}
        "#
    )
}

#[test]
fn page_zero_is_rejected_because_pages_start_at_one() {
    assert!(Benchmark::parse(&with_target("page = 0")).is_err());
}

#[test]
fn ranges_ending_before_they_start_are_rejected() {
    assert!(Benchmark::parse(&with_target("lines = [24, 10]")).is_err());
    assert!(Benchmark::parse(&with_target(r#"time = ["02:10", "01:30"]"#)).is_err());
}

#[test]
fn target_with_more_than_one_location_kind_is_rejected() {
    assert!(Benchmark::parse(&with_target("page = 2\nlines = [10, 24]")).is_err());
}

#[test]
fn query_with_empty_target_list_is_rejected() {
    let source = r#"
        [[query]]
        id = "en-901"
        text = "a remembered passage"
        language = "en"
        target = []
        "#;

    assert!(Benchmark::parse(source).is_err());
}

#[test]
fn duplicate_query_ids_are_rejected() {
    let query = r#"
        [[query]]
        id = "en-902"
        text = "a remembered passage"
        language = "en"

        [[query.target]]
        file = "papers/paper.pdf"
        "#;

    assert!(Benchmark::parse(&format!("{query}{query}")).is_err());
}
