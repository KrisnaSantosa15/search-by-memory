//! What first-run setup downloads.
//!
//! The manifest check needs the EmbeddingGemma 2 ONNX model on disk (see
//! `SBM_MODEL_DIR`); without it, that test is skipped with a message.

mod common;

use std::path::PathBuf;

use sbm_core::AppDirs;
use sbm_setup::{EMBEDDING_MODEL_DIR, embedding_model};

fn model_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("SBM_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| AppDirs::current().cache.join(EMBEDDING_MODEL_DIR));
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
fn embedding_model_is_downloaded_from_a_fixed_revision() {
    let files = embedding_model();

    assert!(!files.is_empty());
    for file in files {
        let revision = file
            .url
            .split("/resolve/")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or_default();
        assert!(
            revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
            "{}",
            file.url
        );
    }
}

#[test]
fn embedding_model_manifest_matches_the_model_the_embedder_loads() {
    let Some(dir) = model_dir() else { return };

    let files = embedding_model();

    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    for needed in ["tokenizer.json", "onnx/model.onnx", "onnx/model.onnx_data"] {
        assert!(paths.contains(&needed), "{needed} missing from {paths:?}");
    }
    for file in files {
        let path = dir.join(&file.path);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            file.size,
            "{}",
            file.path
        );
        // Hashing the 1 GB weights in a debug build is slow; their size is checked above.
        if file.size < 50 * 1024 * 1024 {
            let content = std::fs::read(&path).unwrap();
            assert_eq!(common::sha256(&content), file.sha256, "{}", file.path);
        }
    }
}
