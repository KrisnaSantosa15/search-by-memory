//! Measures Chunk embedding throughput on this machine.
//!
//! Usage: cargo run --release -p sbm-bench --bin embed_throughput [-- --accelerator cpu|directml|directml:<index>]
//! Model directory: `SBM_MODEL_DIR` (default `models/embeddinggemma-2-onnx` in the cache directory, see `AppDirs`).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sbm_bench::Throughput;
use sbm_core::EmbeddingProvider;
use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};

const SENTENCE: &str = "The indexing service splits each document into overlapping chunks and stores their embeddings. ";
const CHUNKS_PER_SETTING: usize = 64;
const BATCH_SIZES: [usize; 3] = [1, 8, 32];
/// Approximate words for ~256 and ~512 tokens of English prose.
const CHUNK_WORDS: [(usize, &str); 2] = [(190, "~256 tok"), (380, "~512 tok")];

fn main() -> sbm_core::Result<()> {
    let model_dir = std::env::var_os("SBM_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            sbm_core::AppDirs::current()
                .cache
                .join("models/embeddinggemma-2-onnx")
        });
    let args: Vec<String> = std::env::args().skip(1).collect();
    let accelerator = match args.as_slice() {
        [] => Accelerator::Cpu,
        [flag, name] if flag == "--accelerator" => name.parse()?,
        _ => return Err("usage: embed_throughput [--accelerator <name>]".into()),
    };
    let embedder = OnnxEmbedder::load_on(&model_dir, 256, DEFAULT_MAX_TOKENS, accelerator)?;
    println!(
        "Accelerator: {:?}
",
        embedder.accelerator()
    );

    println!("| Chunk | Batch | Chunks/s | p50 batch | p95 batch | 1M Chunks | 2M Chunks |");
    println!("|---|---|---|---|---|---|---|");
    for (words, label) in CHUNK_WORDS {
        let chunk = words_of_prose(words);
        for batch_size in BATCH_SIZES {
            let batch: Vec<&str> = vec![chunk.as_str(); batch_size];
            embedder.embed_chunks(&batch)?; // warm-up
            let timings: Vec<Duration> = (0..CHUNKS_PER_SETTING / batch_size)
                .map(|_| {
                    let start = Instant::now();
                    embedder.embed_chunks(&batch).map(|_| start.elapsed())
                })
                .collect::<sbm_core::Result<_>>()?;
            let throughput = Throughput::new(batch_size, &timings);
            let per_second = throughput.chunks_per_second();
            println!(
                "| {label} | {batch_size} | {per_second:.1} | {:.2}s | {:.2}s | {:.1}h | {:.1}h |",
                throughput.latency_percentile(0.50).as_secs_f64(),
                throughput.latency_percentile(0.95).as_secs_f64(),
                1_000_000.0 / per_second / 3600.0,
                2_000_000.0 / per_second / 3600.0,
            );
        }
    }
    Ok(())
}

fn words_of_prose(words: usize) -> String {
    SENTENCE
        .split_whitespace()
        .cycle()
        .take(words)
        .collect::<Vec<_>>()
        .join(" ")
}
