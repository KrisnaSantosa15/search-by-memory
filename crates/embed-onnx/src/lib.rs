//! EmbeddingProvider backed by EmbeddingGemma 2 running in ONNX Runtime (ADR 0001).

use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use sbm_core::{Embedding, EmbeddingProvider, Result};
use tokenizers::{Tokenizer, TruncationParams};

/// Prompt prefixes from the EmbeddingGemma 2 model card.
const QUERY_PREFIX: &str = "task: search result | query: ";
const CHUNK_PREFIX: &str = "title: none | text: ";

/// Token limit per text used by the app. Chunks are 256–512 tokens (PRD FR-03), so this
/// leaves headroom while capping the cost of oversized input. The model itself accepts 8,192.
pub const DEFAULT_MAX_TOKENS: usize = 2048;

/// Width of the (unused, text-only) image/video/audio feature inputs.
const MEDIA_FEATURE_WIDTH: usize = 512;

pub struct OnnxEmbedder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    pad_id: i64,
    dimensions: usize,
}

impl OnnxEmbedder {
    /// Loads `tokenizer.json` and `onnx/model.onnx` from `model_dir`.
    /// Text beyond `max_tokens` tokens is ignored.
    pub fn load(model_dir: &Path, dimensions: usize, max_tokens: usize) -> Result<Self> {
        let mut tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))?;
        tokenizer.with_truncation(Some(TruncationParams {
            max_length: max_tokens,
            ..TruncationParams::default()
        }))?;
        let pad_id = tokenizer.token_to_id("<pad>").map_or(0, i64::from);
        let session = Session::builder()?.commit_from_file(model_dir.join("onnx/model.onnx"))?;
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            pad_id,
            dimensions,
        })
    }

    /// Embeds all texts in one model call, right-padding to the longest input.
    fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let encodings = self.tokenizer.encode_batch(texts, true)?;
        let batch = encodings.len();
        let len = encodings.iter().map(|e| e.len()).max().unwrap_or(0);

        let mut ids = Vec::with_capacity(batch * len);
        let mut mask = Vec::with_capacity(batch * len);
        for encoding in &encodings {
            let padding = len - encoding.len();
            ids.extend(encoding.get_ids().iter().map(|&id| i64::from(id)));
            ids.extend(std::iter::repeat_n(self.pad_id, padding));
            mask.extend(encoding.get_attention_mask().iter().map(|&m| i64::from(m)));
            mask.extend(std::iter::repeat_n(0, padding));
        }

        let no_media = || Tensor::from_array(([0usize, MEDIA_FEATURE_WIDTH], Vec::<f32>::new()));
        let mut session = self
            .session
            .lock()
            .map_err(|_| "embedding session poisoned")?;
        let outputs = session.run(ort::inputs![
            "input_ids" => Tensor::from_array(([batch, len], ids))?,
            "attention_mask" => Tensor::from_array(([batch, len], mask))?,
            "image_features" => no_media()?,
            "video_features" => no_media()?,
            "audio_features" => no_media()?,
        ])?;
        let (shape, values) = outputs["sentence_embedding"].try_extract_tensor::<f32>()?;
        let width = usize::try_from(shape[1])?;
        Ok(values
            .chunks(width)
            .map(|row| Embedding::new(row.to_vec()).truncated(self.dimensions))
            .collect())
    }
}

impl EmbeddingProvider for OnnxEmbedder {
    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed_query(&self, query: &str) -> Result<Embedding> {
        let mut embeddings = self.embed_texts(vec![format!("{QUERY_PREFIX}{query}")])?;
        Ok(embeddings.remove(0))
    }

    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>> {
        self.embed_texts(
            chunks
                .iter()
                .map(|chunk| format!("{CHUNK_PREFIX}{chunk}"))
                .collect(),
        )
    }
}
