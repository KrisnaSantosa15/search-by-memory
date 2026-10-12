//! EmbeddingProvider backed by EmbeddingGemma 2 running in ONNX Runtime (ADR 0001).

use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use sbm_core::{Embedding, EmbeddingProvider, Result};
use tokenizers::Tokenizer;

/// Prompt prefixes from the EmbeddingGemma 2 model card.
const QUERY_PREFIX: &str = "task: search result | query: ";
const CHUNK_PREFIX: &str = "title: none | text: ";

/// Width of the (unused, text-only) image/video/audio feature inputs.
const MEDIA_FEATURE_WIDTH: usize = 512;

pub struct OnnxEmbedder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    dimensions: usize,
}

impl OnnxEmbedder {
    /// Loads `tokenizer.json` and `onnx/model.onnx` from `model_dir`.
    pub fn load(model_dir: &Path, dimensions: usize) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))?;
        let session = Session::builder()?.commit_from_file(model_dir.join("onnx/model.onnx"))?;
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimensions,
        })
    }

    fn embed_text(&self, text: &str) -> Result<Embedding> {
        let encoding = self.tokenizer.encode(text, true)?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| i64::from(id)).collect();
        let mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| i64::from(m)).collect();
        let len = ids.len();

        let no_media = || Tensor::from_array(([0usize, MEDIA_FEATURE_WIDTH], Vec::<f32>::new()));
        let mut session = self.session.lock().map_err(|_| "embedding session poisoned")?;
        let outputs = session.run(ort::inputs![
            "input_ids" => Tensor::from_array(([1usize, len], ids))?,
            "attention_mask" => Tensor::from_array(([1usize, len], mask))?,
            "image_features" => no_media()?,
            "video_features" => no_media()?,
            "audio_features" => no_media()?,
        ])?;
        let (_shape, values) = outputs["sentence_embedding"].try_extract_tensor::<f32>()?;
        Ok(Embedding::new(values.to_vec()))
    }
}

impl EmbeddingProvider for OnnxEmbedder {
    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed_query(&self, query: &str) -> Result<Embedding> {
        self.embed_text(&format!("{QUERY_PREFIX}{query}"))
    }

    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        chunks
            .iter()
            .map(|chunk| self.embed_text(&format!("{CHUNK_PREFIX}{chunk}")))
            .collect()
    }
}
