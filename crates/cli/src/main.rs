//! `sbm`: index folders and search them from the command line.
//!
//! Usage:
//!   sbm index <folder>...
//!   sbm search [--passages] <query>...
//!
//! Data lives in $SBM_DATA_DIR (default ~/.local/share/search-by-memory); the
//! EmbeddingGemma ONNX model in $SBM_MODEL_DIR.

use std::path::PathBuf;
use std::process::ExitCode;

use sbm_core::{Location, Result};
use sbm_embed_onnx::{DEFAULT_MAX_TOKENS, OnnxEmbedder};
use sbm_index::{MatchReason, SearchResult, View};

/// Matryoshka dimensions: 256 scored as well as 512 and 768 on the Benchmark.
const DIMENSIONS: usize = 256;
const RESULTS: usize = 20;
const SNIPPET_CHARS: usize = 160;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sbm: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let data_dir = env_path("SBM_DATA_DIR", home.join(".local/share/search-by-memory"));
    let model_dir = env_path(
        "SBM_MODEL_DIR",
        home.join(".cache/search-by-memory/models/embeddinggemma-2-onnx"),
    );
    match args {
        [command, folders @ ..] if command == "index" && !folders.is_empty() => {
            let embedder = OnnxEmbedder::load(&model_dir, DIMENSIONS, DEFAULT_MAX_TOKENS)?;
            for folder in folders {
                let folder = std::fs::canonicalize(folder)?;
                let changes = sbm_cli::index_folder(&data_dir, &folder, &embedder)?;
                println!(
                    "{}: {} added, {} changed, {} removed",
                    folder.display(),
                    changes.added.len(),
                    changes.changed.len(),
                    changes.removed.len()
                );
            }
            Ok(())
        }
        [command, rest @ ..] if command == "search" => {
            let (view, words) = match rest {
                [flag, words @ ..] if flag == "--passages" => (View::Passage, words),
                words => (View::File, words),
            };
            if words.is_empty() {
                return Err("search needs a Query".into());
            }
            let embedder = OnnxEmbedder::load(&model_dir, DIMENSIONS, DEFAULT_MAX_TOKENS)?;
            let results = sbm_cli::search(&data_dir, &words.join(" "), &embedder, view, RESULTS)?;
            for (rank, result) in results.iter().enumerate() {
                print_result(rank + 1, result);
            }
            Ok(())
        }
        _ => Err("usage: sbm index <folder>... | sbm search [--passages] <query>...".into()),
    }
}

fn print_result(rank: usize, result: &SearchResult) {
    let reason = match result.match_reason {
        MatchReason::Name => "name",
        MatchReason::Content => "content",
        MatchReason::Both => "name + content",
    };
    let location = match &result.location {
        Some(Location::Page(page)) => format!(" p. {page}"),
        Some(Location::Lines { start, end }) => format!(" lines {start}-{end}"),
        _ => String::new(),
    };
    println!("{rank:>2}. {}{location}  [{reason}]", result.path.display());
    for copy in &result.copies {
        println!("    also at {}", copy.display());
    }
    if let Some(snippet) = &result.snippet {
        let line: String = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
        println!(
            "    {}",
            line.chars().take(SNIPPET_CHARS).collect::<String>()
        );
    }
}

fn env_path(variable: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(default)
}
