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
