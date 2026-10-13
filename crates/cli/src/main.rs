//! `sbm`: index folders and search them from the command line.
//!
//! Usage:
//!   sbm index [--accelerator <name>] <folder>...
//!   sbm search [--accelerator <name>] [--passages] <query>...
//!
//! `--accelerator` is cpu, cuda, directml, or directml:<index>; a GPU falls back
//! to CPU. Without it, CUDA, then DirectML, then CPU are tried (GPUs on Windows only).
//!
//! Data lives in $SBM_DATA_DIR (default: see `AppDirs`); the
//! EmbeddingGemma ONNX model in $SBM_MODEL_DIR.

use std::path::PathBuf;
use std::process::ExitCode;

use sbm_core::{AppDirs, Location, Result};
use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};
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
    let (accelerators, args) = sbm_cli::accelerator_option(args)?;
    let dirs = AppDirs::current();
    let data_dir = env_path("SBM_DATA_DIR", dirs.data);
    let model_dir = env_path(
        "SBM_MODEL_DIR",
        dirs.cache.join("models/embeddinggemma-2-onnx"),
    );
    match args.as_slice() {
        [command, folders @ ..] if command == "index" && !folders.is_empty() => {
            let embedder = load_embedder(&model_dir, &accelerators)?;
            for folder in folders {
                let folder = std::fs::canonicalize(folder)?;
                let report = sbm_cli::index_folder(&data_dir, &folder, &embedder)?;
                let changes = &report.changes;
                println!(
                    "{}: {} added, {} changed, {} removed",
                    folder.display(),
                    changes.added.len(),
                    changes.changed.len(),
                    changes.removed.len()
                );
                for (path, error) in &report.failed {
                    eprintln!(
                        "    skipped {}: {}",
                        sbm_cli::printable(&path.to_string_lossy()),
                        sbm_cli::printable(error)
                    );
                }
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
            let embedder = load_embedder(&model_dir, &accelerators)?;
            let results = sbm_cli::search(&data_dir, &words.join(" "), &embedder, view, RESULTS)?;
            for (rank, result) in results.iter().enumerate() {
                print_result(rank + 1, result);
            }
            Ok(())
        }
        _ => Err("usage: sbm index [--accelerator <name>] <folder>... |                   sbm search [--accelerator <name>] [--passages] <query>..."
            .into()),
    }
}

/// Loads the model on the first Accelerator that works and says which one, and why
/// any before it were skipped, on stderr.
fn load_embedder(
    model_dir: &std::path::Path,
    accelerators: &[Accelerator],
) -> Result<OnnxEmbedder> {
    let embedder =
        OnnxEmbedder::load_first(model_dir, DIMENSIONS, DEFAULT_MAX_TOKENS, accelerators)?;
    for (accelerator, reason) in embedder.skipped() {
        eprintln!(
            "sbm: skipped {accelerator}: {}",
            sbm_cli::printable(reason.trim())
        );
    }
    eprintln!("sbm: embedding on {}", embedder.accelerator());
    Ok(embedder)
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
    let path = sbm_cli::printable(&result.path.to_string_lossy());
    println!("{rank:>2}. {path}{location}  [{reason}]");
    for copy in &result.copies {
        println!(
            "    also at {}",
            sbm_cli::printable(&copy.to_string_lossy())
        );
    }
    if let Some(snippet) = &result.snippet {
        let line: String = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
        println!(
            "    {}",
            sbm_cli::printable(&line.chars().take(SNIPPET_CHARS).collect::<String>())
        );
    }
}

fn env_path(variable: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(default)
}
