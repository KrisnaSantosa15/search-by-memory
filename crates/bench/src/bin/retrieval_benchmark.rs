//! Scores semantic-only retrieval on the Public Corpus Benchmark.
//!
//! Usage: cargo run --release -p sbm-bench --bin retrieval_benchmark
//! Needs the Public Corpus (`cd tools && uv run fetch_public_corpus.py`; `SBM_PUBLIC_CORPUS`),
//! the model (`SBM_MODEL_DIR`), and `pdftotext` (poppler) as a dev-only PDF stand-in until M2.
//! Chunk Embeddings are cached in `bench-embeddings` in the cache directory (see `AppDirs`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use sbm_bench::{
    Benchmark, Chunk, EmbeddingCache, ExactStore, RetrievalIndex, chunk_pages, chunk_text,
    evaluate, location_misses,
};
use sbm_core::{Embedding, EmbeddingProvider, Result, VectorStore};
use sbm_embed_onnx::{DEFAULT_MAX_TOKENS, OnnxEmbedder};
use sbm_store_usearch::{Precision, UsearchStore};
use serde::Deserialize;

/// About 256 tokens of prose.
const MAX_CHUNK_CHARS: usize = 1000;
const MODEL_DIMENSIONS: usize = 768;
const DIMENSIONS: [usize; 3] = [256, 512, 768];
const TEXT_EXTENSIONS: [&str; 6] = ["txt", "md", "py", "rs", "go", "js"];

#[derive(Deserialize)]
struct Manifest {
    file: Vec<ManifestFile>,
}

#[derive(Deserialize)]
struct ManifestFile {
    path: PathBuf,
    sha256: String,
}

fn main() -> Result<()> {
    let cache_root = sbm_core::AppDirs::current().cache;
    let model_dir = env_path(
        "SBM_MODEL_DIR",
        cache_root.join("models/embeddinggemma-2-onnx"),
    );
    let corpus = env_path("SBM_PUBLIC_CORPUS", cache_root.join("public-corpus"));
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let manifest: Manifest = toml::from_str(&std::fs::read_to_string(
        repo.join("benchmark/public/manifest.toml"),
    )?)?;
    let benchmark = Benchmark::parse(&std::fs::read_to_string(
        repo.join("benchmark/public/queries.toml"),
    )?)?;
    let embedder = OnnxEmbedder::load(&model_dir, MODEL_DIMENSIONS, DEFAULT_MAX_TOKENS)?;
    let cache = EmbeddingCache::new(&cache_root.join("bench-embeddings"));

    let mut chunks: Vec<Chunk> = Vec::new();
    let mut embeddings: Vec<Embedding> = Vec::new();
    for file in &manifest.file {
        let Some(file_chunks) = chunk_file(&corpus, &file.path)? else {
            eprintln!("skip   {} (format not indexed yet)", file.path.display());
            continue;
        };
        let started = Instant::now();
        let texts: Vec<&str> = file_chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect();
        let key = format!("{}-{MAX_CHUNK_CHARS}c-{MODEL_DIMENSIONS}d", file.sha256);
        embeddings.extend(cache.embed(&key, &texts, &embedder)?);
        eprintln!(
            "index  {} ({} Chunks, {:.0}s)",
            file.path.display(),
            file_chunks.len(),
            started.elapsed().as_secs_f64()
        );
        chunks.extend(file_chunks);
    }

    println!(
        "\nSemantic-only, File view, {} Chunks, {} Benchmark Queries\n",
        chunks.len(),
        benchmark.queries().len()
    );
    println!(
        "| Dims | Store | Queries scored | Recall@5 | Recall@10 | MRR | nDCG@10 | Location Accuracy |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for dimensions in DIMENSIONS {
        let embedder = Truncating {
            inner: &embedder,
            dimensions,
        };
        let truncated: Vec<Embedding> =
            embeddings.iter().map(|e| e.truncated(dimensions)).collect();
        report(
            dimensions,
            "exact f32",
            &chunks,
            &truncated,
            ExactStore::default(),
            &benchmark,
            &embedder,
        )?;
        report(
            dimensions,
            "USearch f16",
            &chunks,
            &truncated,
            UsearchStore::new(dimensions, Precision::F16)?,
            &benchmark,
            &embedder,
        )?;
        report(
            dimensions,
            "USearch i8",
            &chunks,
            &truncated,
            UsearchStore::new(dimensions, Precision::I8)?,
            &benchmark,
            &embedder,
        )?;
    }

    let baseline = Truncating {
        inner: &embedder,
        dimensions: DIMENSIONS[0],
    };
    let truncated: Vec<Embedding> = embeddings
        .iter()
        .map(|e| e.truncated(DIMENSIONS[0]))
        .collect();
    let index = RetrievalIndex::build(chunks.clone(), &truncated, ExactStore::default())?;
    println!("\nLocation misses ({} dims, exact):\n", DIMENSIONS[0]);
    for miss in location_misses(&benchmark, &index, &baseline)? {
        println!(
            "- {} {}: shown {:?}, Target {:?}",
            miss.query_id,
            miss.file.display(),
            miss.shown,
            miss.targets
        );
    }
    Ok(())
}

fn report<S: VectorStore>(
    dimensions: usize,
    store_name: &str,
    chunks: &[Chunk],
    embeddings: &[Embedding],
    store: S,
    benchmark: &Benchmark,
    embedder: &dyn EmbeddingProvider,
) -> Result<()> {
    let index = RetrievalIndex::build(chunks.to_vec(), embeddings, store)?;
    let scores = evaluate(benchmark, &index, embedder)?;
    println!(
        "| {dimensions} | {store_name} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |",
        scores.len(),
        scores.recall_at(5),
        scores.recall_at(10),
        scores.mean_reciprocal_rank(),
        scores.ndcg_at(10),
        scores.location_accuracy(),
    );
    Ok(())
}

/// Chunks of a corpus file, or `None` when its format is not indexed yet.
fn chunk_file(corpus: &Path, file: &Path) -> Result<Option<Vec<Chunk>>> {
    let path = corpus.join(file);
    let extension = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if extension == "pdf" {
        let output = Command::new("pdftotext").arg(&path).arg("-").output()?;
        if !output.status.success() {
            return Err(format!("pdftotext failed on {}", path.display()).into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        return Ok(Some(chunk_pages(file, &text, MAX_CHUNK_CHARS)));
    }
    if TEXT_EXTENSIONS.contains(&extension) {
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some(chunk_text(file, &text, MAX_CHUNK_CHARS)));
    }
    Ok(None)
}

fn env_path(variable: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(default)
}

/// Embeds Queries at full width, then applies Matryoshka truncation.
struct Truncating<'a> {
    inner: &'a OnnxEmbedder,
    dimensions: usize,
}

impl EmbeddingProvider for Truncating<'_> {
    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed_query(&self, query: &str) -> Result<Embedding> {
        Ok(self.inner.embed_query(query)?.truncated(self.dimensions))
    }

    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        let embeddings = self.inner.embed_chunks(chunks)?;
        Ok(embeddings
            .iter()
            .map(|e| e.truncated(self.dimensions))
            .collect())
    }
}
