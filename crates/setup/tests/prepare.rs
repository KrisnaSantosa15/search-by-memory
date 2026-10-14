//! Preparing the machine for the speed the user picked: one download for
//! everything it needs, with one progress bar.

mod common;

use std::io::Write;
use std::sync::atomic::AtomicBool;

use common::FakeHttp;
use sbm_embed_onnx::Accelerator;
use sbm_setup::{
    CUDA_RUNTIME_DIR, EMBEDDING_MODEL_DIR, Finished, Plan, Progress, cuda_runtime, embedding_model,
    prepare,
};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

fn archive(path: &str, content: &[u8]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(path, SimpleFileOptions::default()).unwrap();
    zip.write_all(content).unwrap();
    zip.finish().unwrap().into_inner()
}

#[test]
fn runtime_and_model_are_installed_under_one_progress_bar() {
    let cache = TempDir::new().unwrap();
    let http = FakeHttp::default();
    let plan = Plan {
        runtime: vec![http.serve(
            "https://example.test/cudnn.zip",
            "cudnn.zip",
            &archive("cudnn/bin/cudnn64_9.dll", b"cudnn"),
        )],
        model: vec![http.serve(
            "https://example.test/model.onnx",
            "onnx/model.onnx",
            b"model weights",
        )],
    };
    let mut reports = Vec::new();

    let finished = prepare(
        &plan,
        cache.path(),
        &http,
        &AtomicBool::new(false),
        &mut |progress| reports.push(progress),
    )
    .unwrap();

    assert_eq!(finished, Finished::Complete);
    let total = plan.runtime[0].size + plan.model[0].size;
    assert!(reports.iter().all(|report| report.total == total));
    assert!(reports.windows(2).all(|pair| pair[0].done <= pair[1].done));
    assert_eq!(reports.last(), Some(&Progress { done: total, total }));
    let model = cache
        .path()
        .join(EMBEDDING_MODEL_DIR)
        .join("onnx/model.onnx");
    assert_eq!(std::fs::read(model).unwrap(), b"model weights");
    let dll = cache.path().join(CUDA_RUNTIME_DIR).join("cudnn64_9.dll");
    assert_eq!(std::fs::read(dll).unwrap(), b"cudnn");
}

#[test]
fn only_the_nvidia_choice_downloads_the_cuda_runtime() {
    let cuda = Plan::for_accelerator(Accelerator::Cuda);
    let directml = Plan::for_accelerator(Accelerator::DirectMl);
    let cpu = Plan::for_accelerator(Accelerator::Cpu);

    assert_eq!(cuda.runtime, cuda_runtime());
    assert_eq!((directml.runtime, cpu.runtime.clone()), (vec![], vec![]));
    for plan in [&cuda, &cpu] {
        assert_eq!(plan.model, embedding_model());
    }
}
