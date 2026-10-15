//! Measures how long Queries take through the Engine, as the launcher runs them
//! (PRD: Results within 1 second).
//!
//! Usage: SBM_DATA_DIR=<index> cargo run --release -p sbm-bench --bin search_latency [-- --accelerator <name>]
//! Build the index first, e.g. `SBM_DATA_DIR=<index> sbm index <public corpus>`. Queries come
//! from `benchmark/public/queries.toml`; each runs `ROUNDS` times, after one warm-up Query.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sbm_bench::{Benchmark, Throughput};
use sbm_change_usn::UsnJournal;
use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};
use sbm_engine::Engine;
use sbm_index::View;

const ROUNDS: usize = 3;
/// As `sbm` and the app index.
const DIMENSIONS: usize = 256;
/// Results the launcher asks for.
const LIMIT: usize = 20;
const BUDGET: Duration = Duration::from_secs(1);

fn main() -> sbm_core::Result<()> {
    let dirs = sbm_core::AppDirs::current();
    let data_dir = std::env::var_os("SBM_DATA_DIR")
        .map(PathBuf::from)
        .ok_or("set SBM_DATA_DIR to an index built by `sbm index`")?;
    let model_dir = std::env::var_os("SBM_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs.cache.join("models/embeddinggemma-2-onnx"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let accelerator = match args.as_slice() {
        [] => Accelerator::Cpu,
        [flag, name] if flag == "--accelerator" => name.parse()?,
        _ => return Err("usage: search_latency [--accelerator <name>]".into()),
    };

    let loading = Instant::now();
    let embedder = OnnxEmbedder::load_first(
        &model_dir,
        DIMENSIONS,
        DEFAULT_MAX_TOKENS,
        &[accelerator, Accelerator::Cpu],
    )?;
    println!(
        "Model on {} loaded in {:.1} s",
        embedder.accelerator(),
        loading.elapsed().as_secs_f64()
    );
    let opening = Instant::now();
    let engine = Engine::start(&data_dir, Arc::new(embedder), Box::new(UsnJournal))?;
    engine.wait_until_idle();
    let status = engine.status()?;
    println!(
        "Index opened in {:.1} s: {} files, {} with content, {} pending",
        opening.elapsed().as_secs_f64(),
        status.files,
        status.indexed,
        status.pending
    );

    let queries = Benchmark::parse(&std::fs::read_to_string(queries_file())?)?;
    let texts: Vec<&str> = queries.queries().iter().map(|q| q.text.as_str()).collect();
    let first = time(&engine, texts[0])?;
    println!("First Query: {} ms", first.as_millis());

    let mut timings = Vec::new();
    for _ in 0..ROUNDS {
        for text in &texts {
            timings.push(time(&engine, text)?);
        }
    }
    let latency = Throughput::new(1, &timings);
    let over = timings.iter().filter(|timing| **timing > BUDGET).count();
    println!(
        "{} Queries x {ROUNDS}: p50 {} ms, p95 {} ms, max {} ms; {over} over {} ms",
        texts.len(),
        latency.latency_percentile(0.50).as_millis(),
        latency.latency_percentile(0.95).as_millis(),
        latency.latency_percentile(1.0).as_millis(),
        BUDGET.as_millis()
    );
    Ok(())
}

/// How long `query` takes in the File view, as the launcher asks for it.
fn time(engine: &Engine, query: &str) -> sbm_core::Result<Duration> {
    let start = Instant::now();
    engine.search(query, View::File, LIMIT)?;
    Ok(start.elapsed())
}

fn queries_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmark/public/queries.toml")
}
