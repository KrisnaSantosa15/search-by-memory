//! Behavior tests for the ONNX-backed EmbeddingProvider.
//!
//! These tests need the EmbeddingGemma 2 ONNX model on disk. Point
//! `SBM_MODEL_DIR` at a directory containing `tokenizer.json` and
//! `onnx/model.onnx` (default: ~/.cache/search-by-memory/models/embeddinggemma-2-onnx).
//! When the model is absent the tests are skipped with a message.

use std::path::PathBuf;

use sbm_core::EmbeddingProvider;
use sbm_embed_onnx::{DEFAULT_MAX_TOKENS, OnnxEmbedder};

fn model_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("SBM_MODEL_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home).join(".cache/search-by-memory/models/embeddinggemma-2-onnx")
            })
        })?;
    if dir.join("onnx/model.onnx").exists() {
        Some(dir)
    } else {
        eprintln!(
            "SKIPPED: EmbeddingGemma 2 ONNX model not found at {}",
            dir.display()
        );
        None
    }
}

#[test]
fn related_chunk_scores_higher_than_unrelated_chunk_for_a_query() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256, DEFAULT_MAX_TOKENS).expect("model loads");

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
    let embedder = OnnxEmbedder::load(&dir, 256, DEFAULT_MAX_TOKENS).expect("model loads");

    let query = embedder
        .embed_query("meeting notes from yesterday")
        .expect("query embeds");
    let chunks = embedder
        .embed_chunks(&["Agenda: quarterly planning."])
        .expect("chunk embeds");

    for embedding in [&query, &chunks[0]] {
        assert_eq!(embedding.as_slice().len(), embedder.dimensions());
        let norm = embedding
            .as_slice()
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "expected unit length, got norm {norm}"
        );
    }
}

/// ADR 0001 go/no-go gate: the ONNX embedder must reproduce the official
/// PyTorch / sentence-transformers embeddings (tools/make_parity_fixtures.py).
#[test]
fn onnx_embeddings_match_the_pytorch_reference() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 768, DEFAULT_MAX_TOKENS).expect("model loads");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/parity.json")).expect("fixture parses");

    for item in fixture["items"].as_array().expect("items") {
        let text = item["text"].as_str().expect("text");
        let reference: Vec<f32> = item["embedding"]
            .as_array()
            .expect("embedding")
            .iter()
            .map(|v| v.as_f64().expect("number") as f32)
            .collect();
        let actual = match item["kind"].as_str() {
            Some("query") => embedder.embed_query(text),
            Some("chunk") => embedder.embed_chunks(&[text]).map(|mut v| v.remove(0)),
            other => panic!("unknown fixture kind {other:?}"),
        }
        .expect("embeds");

        let reference_norm = reference.iter().map(|v| v * v).sum::<f32>().sqrt();
        let cosine = actual
            .as_slice()
            .iter()
            .zip(&reference)
            .map(|(a, r)| a * r)
            .sum::<f32>()
            / reference_norm;
        assert!(cosine >= 0.99, "cosine {cosine} < 0.99 for {:?}", text);
    }
}

#[test]
fn embedding_chunks_in_a_batch_matches_embedding_them_one_by_one() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256, DEFAULT_MAX_TOKENS).expect("model loads");
    let chunks = [
        "Short note.",
        "A much longer chunk of text that spans many more tokens than the first one, so that batching \
         must pad the shorter inputs and mask the padding correctly for the embeddings to stay identical.",
        "This code refreshes the JWT token once it has expired.",
    ];

    let batched = embedder.embed_chunks(&chunks).expect("batch embeds");
    assert_eq!(batched.len(), chunks.len());
    for (chunk, from_batch) in chunks.iter().zip(&batched) {
        let alone = &embedder.embed_chunks(&[chunk]).expect("single embeds")[0];
        let cosine = from_batch.similarity(alone);
        assert!(
            cosine > 0.9999,
            "batched vs single cosine {cosine} for {chunk:?}"
        );
    }
}

#[test]
fn text_beyond_the_token_limit_is_ignored() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256, 64).expect("model loads");
    let beyond_limit = "The quarterly budget meeting covered hiring plans. ".repeat(20);
    let with_unrelated_tail =
        format!("{beyond_limit} Preheat the oven and bake the bread for twenty minutes.");

    let embeddings = embedder
        .embed_chunks(&[&beyond_limit, &with_unrelated_tail])
        .expect("chunks embed");

    let cosine = embeddings[0].similarity(&embeddings[1]);
    assert!(
        cosine > 0.9999,
        "text past the token limit changed the embedding (cosine {cosine})"
    );
}

/// Multilingual capability (PRD FR-02: English and Indonesian Queries).
#[test]
fn queries_find_related_chunks_across_english_and_indonesian() {
    let Some(dir) = model_dir() else { return };
    let embedder = OnnxEmbedder::load(&dir, 256, DEFAULT_MAX_TOKENS).expect("model loads");
    let chunks = embedder
        .embed_chunks(&[
            "Rapat membahas anggaran kuartal ketiga dan target penjualan tahun depan.",
            "The meeting discussed the third-quarter budget and next year's sales targets.",
            "Preheat the oven to 180 degrees and bake for 25 minutes.",
        ])
        .expect("chunks embed");
    let (indonesian_budget, english_budget, unrelated) = (&chunks[0], &chunks[1], &chunks[2]);

    let english_query = embedder
        .embed_query("meeting notes about the quarterly budget")
        .expect("query embeds");
    assert!(
        english_query.similarity(indonesian_budget) > english_query.similarity(unrelated),
        "English Query should find the Indonesian Chunk"
    );

    let indonesian_query = embedder
        .embed_query("catatan rapat tentang anggaran kuartal")
        .expect("query embeds");
    assert!(
        indonesian_query.similarity(english_budget) > indonesian_query.similarity(unrelated),
        "Indonesian Query should find the English Chunk"
    );
}
