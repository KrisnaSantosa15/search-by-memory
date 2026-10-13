//! Guards the committed Public Corpus Benchmark (`benchmark/public/queries.toml`).
//!
//! The Target-file check needs the Public Corpus on disk: fetch it with
//! `cd tools && uv run fetch_public_corpus.py`, or point `SBM_PUBLIC_CORPUS` at it
//! (default: `public-corpus` in the cache directory, see `AppDirs`). When absent, that check is skipped.

use std::path::{Path, PathBuf};

use sbm_bench::Benchmark;
use sbm_core::AppDirs;

fn public_benchmark() -> Benchmark {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmark/public/queries.toml");
    Benchmark::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn public_corpus() -> Option<PathBuf> {
    let dir = std::env::var_os("SBM_PUBLIC_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| AppDirs::current().cache.join("public-corpus"));
    if dir.is_dir() {
        Some(dir)
    } else {
        eprintln!("SKIPPED: Public Corpus not found at {}", dir.display());
        None
    }
}

#[test]
fn public_benchmark_has_english_and_indonesian_queries() {
    let benchmark = public_benchmark();

    for language in ["en", "id"] {
        assert!(
            benchmark
                .queries()
                .iter()
                .any(|query| query.language == language)
        );
    }
}

#[test]
fn every_public_target_file_is_in_the_public_corpus() {
    let Some(corpus) = public_corpus() else {
        return;
    };

    assert_eq!(
        public_benchmark().missing_targets(&corpus),
        Vec::<PathBuf>::new()
    );
}
