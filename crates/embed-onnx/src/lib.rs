//! EmbeddingProvider backed by EmbeddingGemma 2 running in ONNX Runtime (ADR 0001).

use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::session::builder::SessionBuilder;
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

/// The hardware that runs the embedding model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accelerator {
    Cpu,
    /// Any DirectX 12 GPU on Windows, preferring the most powerful one.
    DirectMl,
    /// The DirectX 12 GPU with this adapter index on Windows.
    DirectMlDevice(u32),
    /// The first NVIDIA GPU, through CUDA 13 and cuDNN 9.
    Cuda,
}

impl Accelerator {
    /// The Accelerators to try, fastest first, when the user names none.
    /// GPUs are only enabled in Windows builds.
    pub fn default_order() -> &'static [Accelerator] {
        if cfg!(windows) {
            &[Accelerator::Cuda, Accelerator::DirectMl, Accelerator::Cpu]
        } else {
            &[Accelerator::Cpu]
        }
    }
}

impl std::fmt::Display for Accelerator {
    /// Writes the command-line name that [`FromStr`](std::str::FromStr) parses.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Accelerator::Cpu => write!(f, "cpu"),
            Accelerator::Cuda => write!(f, "cuda"),
            Accelerator::DirectMl => write!(f, "directml"),
            Accelerator::DirectMlDevice(device) => write!(f, "directml:{device}"),
        }
    }
}

impl std::str::FromStr for Accelerator {
    type Err = String;

    /// Parses `cpu`, `directml`, or `directml:<adapter index>`.
    fn from_str(name: &str) -> std::result::Result<Self, String> {
        match name.split_once(':') {
            None if name == "cpu" => Ok(Accelerator::Cpu),
            None if name == "directml" => Ok(Accelerator::DirectMl),
            None if name == "cuda" => Ok(Accelerator::Cuda),
            Some(("directml", device)) => device
                .parse()
                .map(Accelerator::DirectMlDevice)
                .map_err(|_| format!("invalid DirectML adapter index {device:?}")),
            _ => Err(format!(
                "unknown accelerator {name:?}; expected cpu, cuda, directml, or directml:<index>"
            )),
        }
    }
}

pub struct OnnxEmbedder {
    session: Mutex<Session>,
    accelerator: Accelerator,
    skipped: Vec<(Accelerator, String)>,
    tokenizer: Tokenizer,
    pad_id: i64,
    dimensions: usize,
}

impl OnnxEmbedder {
    /// Loads `tokenizer.json` and `onnx/model.onnx` from `model_dir`.
    /// Text beyond `max_tokens` tokens is ignored.
    pub fn load(model_dir: &Path, dimensions: usize, max_tokens: usize) -> Result<Self> {
        Self::load_on(model_dir, dimensions, max_tokens, Accelerator::Cpu)
    }

    /// Like [`OnnxEmbedder::load`], running the model on `accelerator`.
    /// When the GPU is missing or unusable, falls back to CPU: slower but always works.
    pub fn load_on(
        model_dir: &Path,
        dimensions: usize,
        max_tokens: usize,
        accelerator: Accelerator,
    ) -> Result<Self> {
        Self::load_first(
            model_dir,
            dimensions,
            max_tokens,
            &[accelerator, Accelerator::Cpu],
        )
    }

    /// Like [`OnnxEmbedder::load`], running the model on the first of
    /// `accelerators` that works. See [`OnnxEmbedder::skipped`] for the others.
    pub fn load_first(
        model_dir: &Path,
        dimensions: usize,
        max_tokens: usize,
        accelerators: &[Accelerator],
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))?;
        tokenizer.with_truncation(Some(TruncationParams {
            max_length: max_tokens,
            ..TruncationParams::default()
        }))?;
        let pad_id = tokenizer.token_to_id("<pad>").map_or(0, i64::from);
        let model = model_dir.join("onnx/model.onnx");
        let mut skipped = Vec::new();
        for &accelerator in accelerators {
            match session_builder(accelerator)
                .and_then(|mut builder| builder.commit_from_file(&model))
            {
                Ok(session) => {
                    return Ok(Self {
                        session: Mutex::new(session),
                        accelerator,
                        skipped,
                        tokenizer,
                        pad_id,
                        dimensions,
                    });
                }
                Err(error) => skipped.push((accelerator, error.to_string())),
            }
        }
        let reasons: Vec<String> = skipped
            .iter()
            .map(|(accelerator, reason)| format!("{accelerator}: {reason}"))
            .collect();
        Err(format!(
            "no accelerator could run the model ({})",
            reasons.join("; ")
        )
        .into())
    }

    /// The hardware the model runs on.
    pub fn accelerator(&self) -> Accelerator {
        self.accelerator
    }

    /// Accelerators tried before the one in use, each with why it could not run the model.
    pub fn skipped(&self) -> &[(Accelerator, String)] {
        &self.skipped
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

fn session_builder(accelerator: Accelerator) -> ort::Result<SessionBuilder> {
    let builder = Session::builder()?;
    match accelerator {
        Accelerator::Cpu => Ok(builder),
        #[cfg(windows)]
        Accelerator::DirectMl | Accelerator::DirectMlDevice(_) => {
            let directml = ort::ep::DirectML::default();
            let directml = match accelerator {
                Accelerator::DirectMlDevice(device) => directml.with_device_id(device as i32),
                _ => directml.with_performance_preference(
                    ort::ep::directml::PerformancePreference::HighPerformance,
                ),
            };
            Ok(builder
                // DirectML supports neither memory patterns nor parallel execution.
                .with_memory_pattern(false)?
                .with_parallel_execution(false)?
                .with_execution_providers([directml.build().error_on_failure()])?)
        }
        #[cfg(windows)]
        Accelerator::Cuda => Ok(builder
            .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])?),
        #[cfg(not(windows))]
        Accelerator::Cuda => Err(ort::Error::new("CUDA is only enabled in Windows builds")),
        #[cfg(not(windows))]
        Accelerator::DirectMl | Accelerator::DirectMlDevice(_) => {
            Err(ort::Error::new("DirectML is only available on Windows"))
        }
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
