//! Behavior tests for the ONNX-backed EmbeddingProvider.
//!
//! These tests need the EmbeddingGemma 2 ONNX model on disk. Point
//! `SBM_MODEL_DIR` at a directory containing `tokenizer.json` and
//! `onnx/model.onnx` (default: ~/.cache/search-by-memory/models/embeddinggemma-2-onnx).
//! When the model is absent the tests are skipped with a message.

use std::path::PathBuf;

use sbm_core::EmbeddingProvider;
use sbm_embed_onnx::OnnxEmbedder;

fn model_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("SBM_MODEL_DIR").map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".cache/search-by-memory/models/embeddinggemma-2-onnx"))
    })?;
    if dir.join("onnx/model.onnx").exists() {
        Some(dir)
    } else {
        eprintln!("SKIPPED: EmbeddingGemma 2 ONNX model not found at {}", dir.display());
        None
    }
}

#[test]
fn related_chunk_scores_higher_than_unrelated_chunk_for_a_query() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256).expect("model loads");

    let query = embedder
        .embed_query("tutorial explaining attention mechanisms: query, key and value")
        .expect("query embeds");
    let chunks = embedder
        .embed_chunks(&[
            "Transformers compute attention by comparing query vectors against key vectors, then use the scores to weight the value vectors.",
            "Preheat the oven to 180 degrees, whisk the eggs with sugar, and fold in the flour before baking for 25 minutes.",
        ])
        .expect("chunks embed");

    let related = query.similarity(&chunks[0]);
    let unrelated = query.similarity(&chunks[1]);
    assert!(
        related > unrelated,
        "related chunk ({related}) should score higher than unrelated chunk ({unrelated})"
    );
}

#[test]
fn embeddings_have_the_configured_dimensions_and_unit_length() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256).expect("model loads");

    let query = embedder.embed_query("meeting notes from yesterday").expect("query embeds");
    let chunks = embedder.embed_chunks(&["Agenda: quarterly planning."]).expect("chunk embeds");

    for embedding in [&query, &chunks[0]] {
        assert_eq!(embedding.as_slice().len(), embedder.dimensions());
        let norm = embedding.as_slice().iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "expected unit length, got norm {norm}");
    }
}
