//! First-run setup: verified, resumable downloads of the embedding model and
//! GPU runtimes.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod system_gpus;

pub use system_gpus::system_gpus;

use sbm_core::Result;
use sbm_embed_onnx::Accelerator;
use sha2::{Digest, Sha256};

/// A file to download.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteFile {
    pub url: String,
    /// Where it is saved, relative to the download folder, with forward slashes.
    pub path: String,
    pub size: u64,
    /// Lowercase hex SHA-256 of the whole file.
    pub sha256: String,
}

/// Bytes of a download on disk so far, out of all of them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

/// Where the embedding model lives, relative to the cache directory (`AppDirs`).
pub const EMBEDDING_MODEL_DIR: &str = "models/embeddinggemma-2-onnx";

/// Hugging Face revision of `onnx-community/embeddinggemma-2-ONNX` (Apache-2.0)
/// that passed the parity gate (ADR 0001).
const EMBEDDING_MODEL_REVISION: &str = "daa72c51243991dfcaf9f9137d2c573d8f7790c0";

/// The EmbeddingGemma 2 ONNX model files the embedder loads (about 1.1 GB).
pub fn embedding_model() -> Vec<RemoteFile> {
    [
        ("config.json", 5_031, "8d011bfe08b5e345bbe0b81e5c6fd02c381920b345b986047bc2a33ce7b90d1d"),
        ("tokenizer_config.json", 1_599, "17bd5d6e9364ca49a534e1502076593317c298d4a663623091ed45388f004874"),
        ("tokenizer.json", 32_170_510, "4d777ef5bdc1aa36227abdfb77c3e49e7b9c892d16e1b6bda41c393504828be4"),
        ("onnx/model.onnx", 421_421, "bc47de15f81208a5c99e5ab10f746d5e33b51ea228b7dc0bef9c133a94f1c1c3"),
        ("onnx/model.onnx_data", 1_084_170_240, "9fd452bfc6916e5a92f080f8dada35598ca4512b21c65897d4c5974d5f9630a2"),
    ]
    .into_iter()
    .map(|(path, size, sha256)| RemoteFile {
        url: format!(
            "https://huggingface.co/onnx-community/embeddinggemma-2-ONNX/resolve/{EMBEDDING_MODEL_REVISION}/{path}"
        ),
        path: path.to_string(),
        size,
        sha256: sha256.to_string(),
    })
    .collect()
}

/// Who made a GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

/// A graphics card found on this machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    pub name: String,
    pub vendor: GpuVendor,
    /// Video memory of its own, in bytes; 0 for most integrated GPUs.
    pub dedicated_memory: u64,
}

/// One way to run the embedding model, offered during first-run setup.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedOption {
    pub accelerator: Accelerator,
    /// The GPU it uses; `None` for the CPU.
    pub gpu: Option<Gpu>,
    pub recommended: bool,
    /// Needs the NVIDIA CUDA runtime downloaded first.
    pub needs_cuda_runtime: bool,
}

/// Video memory a GPU needs before it is recommended over the CPU.
const MIN_GPU_MEMORY: u64 = 2 << 30;

/// The ways to run the embedding model on a machine with `gpus`, fastest first.
pub fn speed_options(gpus: &[Gpu]) -> Vec<SpeedOption> {
    let option = |accelerator, gpu: Option<&Gpu>, recommended| SpeedOption {
        accelerator,
        gpu: gpu.cloned(),
        recommended,
        needs_cuda_runtime: accelerator == Accelerator::Cuda,
    };
    let mut options = Vec::new();
    if let Some(nvidia) = gpus.iter().find(|gpu| gpu.vendor == GpuVendor::Nvidia) {
        options.push(option(Accelerator::Cuda, Some(nvidia), true));
    }
    if let Some(strongest) = gpus.iter().max_by_key(|gpu| gpu.dedicated_memory) {
        // Integrated graphics shares memory with the CPU and is rarely faster than it.
        let recommended = options.is_empty() && strongest.dedicated_memory >= MIN_GPU_MEMORY;
        options.push(option(Accelerator::DirectMl, Some(strongest), recommended));
    }
    let recommended = !options.iter().any(|option| option.recommended);
    options.push(option(Accelerator::Cpu, None, recommended));
    options
}

/// How a download ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finished {
    Complete,
    /// Stopped on request; the next download resumes where this one stopped.
    Cancelled,
}

/// The web (system boundary).
pub trait Http {
    /// The body of `url` from byte `from` on.
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>>;
}

/// The web over HTTPS.
pub struct Web {
    agent: ureq::Agent,
}

impl Web {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self { agent }
    }
}

impl Default for Web {
    fn default() -> Self {
        Self::new()
    }
}

impl Http for Web {
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>> {
        let mut request = self.agent.get(url);
        if from > 0 {
            request = request.header("Range", format!("bytes={from}-"));
        }
        let response = request.call()?;
        let resumed = response.status() == 206;
        let mut body = response.into_body().into_reader();
        if from > 0 && !resumed {
            // The server sent the whole file: skip what is already on disk.
            std::io::copy(&mut (&mut body).take(from), &mut std::io::sink())?;
        }
        Ok(Box::new(body))
    }
}

/// Downloads `files` into `dir`, reporting progress over all of them.
pub fn download(
    files: &[RemoteFile],
    dir: &Path,
    http: &dyn Http,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Finished> {
    let mut current = Progress {
        done: 0,
        total: files.iter().map(|file| file.size).sum(),
    };
    let mut buffer = vec![0; 64 * 1024];
    for file in files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Only a verified download gets its final name.
        if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() == file.size) {
            current.done += file.size;
            progress(current);
            continue;
        }
        // Bytes already fetched wait in a `.part` file, so a new run resumes there.
        let part = dir.join(format!("{}.part", file.path));
        let from = std::fs::metadata(&part).map_or(0, |metadata| metadata.len());
        current.done += from;
        let mut out = File::options().create(true).append(true).open(&part)?;
        // One byte past the expected size is enough to tell the file is wrong.
        let mut body = http
            .get(&file.url, from)?
            .take(file.size.saturating_sub(from) + 1);
        let mut written = from;
        loop {
            let read = body.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            written += read as u64;
            if written > file.size {
                drop(out);
                std::fs::remove_file(&part)?;
                return Err(format!("{} is larger than expected", file.path).into());
            }
            out.write_all(&buffer[..read])?;
            current.done += read as u64;
            progress(current);
            if cancel.load(Ordering::SeqCst) {
                return Ok(Finished::Cancelled);
            }
        }
        drop(out);
        if sha256_of(&part)? != file.sha256 {
            std::fs::remove_file(&part)?;
            return Err(
                format!("{} arrived damaged and will be downloaded again", file.path).into(),
            );
        }
        std::fs::rename(&part, &path)?;
    }
    Ok(Finished::Complete)
}

/// Lowercase hex SHA-256 of the file at `path`, read in pieces.
fn sha256_of(path: &Path) -> Result<String> {
    let (mut hasher, mut file) = (Sha256::new(), File::open(path)?);
    let mut buffer = vec![0; 64 * 1024];
    loop {
        match file.read(&mut buffer)? {
            0 => break,
            read => hasher.update(&buffer[..read]),
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Where the CUDA runtime lives, relative to the cache directory (`AppDirs`).
pub const CUDA_RUNTIME_DIR: &str = "runtimes/cuda13";

/// The NVIDIA archives holding the CUDA 13 and cuDNN 9 DLLs that ONNX Runtime's
/// CUDA provider loads (about 1.9 GB to download, 1.4 GB unpacked).
pub fn cuda_runtime() -> Vec<RemoteFile> {
    // From NVIDIA's redistrib_13.4.3.json (CUDA) and redistrib_9.27.0.json (cuDNN).
    [
        (
            "cuda/redist/cuda_cudart/windows-x86_64",
            "cuda_cudart-windows-x86_64-13.4.92-archive.zip",
            2_737_397,
            "621a70c4778287b1abf8c27c61b841d797fda2aeff01033374dd19624bcfa249",
        ),
        (
            "cuda/redist/libcublas/windows-x86_64",
            "libcublas-windows-x86_64-13.8.0.4-archive.zip",
            422_415_931,
            "0974318e9861a61cb9091cf7de8d4c2880e9787fe311cb4bcbd08865663c5f43",
        ),
        (
            "cuda/redist/libcufft/windows-x86_64",
            "libcufft-windows-x86_64-12.4.0.43-archive.zip",
            159_602_693,
            "69d0ad8dc3a1be66f01a748a8206d0ceaafa56939474663fbe790dc3e91d2009",
        ),
        (
            "cudnn/redist/cudnn/windows-x86_64",
            "cudnn-windows-x86_64-9.27.0.42_cuda13-archive.zip",
            1_330_803_635,
            "15595c7b52896262a0e590f02b3b3851c6a2c9046a72047526deaf59a9cbf7d5",
        ),
    ]
    .into_iter()
    .map(|(folder, name, size, sha256)| RemoteFile {
        url: format!("https://developer.download.nvidia.com/compute/{folder}/{name}"),
        path: name.to_string(),
        size,
        sha256: sha256.to_string(),
    })
    .collect()
}

/// Installs a GPU runtime into `dir`: downloads the zip `archives` and unpacks
/// the DLLs in their `bin` folders.
pub fn install_runtime(
    archives: &[RemoteFile],
    dir: &Path,
    http: &dyn Http,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Finished> {
    // The archives are gone once unpacked; this records which ones were.
    let marker = dir.join(INSTALLED);
    let installed: String = archives
        .iter()
        .map(|archive| {
            format!(
                "{}
",
                archive.sha256
            )
        })
        .collect();
    if std::fs::read_to_string(&marker).is_ok_and(|done| done == installed) {
        let total = archives.iter().map(|archive| archive.size).sum();
        progress(Progress { done: total, total });
        return Ok(Finished::Complete);
    }
    let downloads = dir.join(DOWNLOADS);
    if download(archives, &downloads, http, cancel, progress)? == Finished::Cancelled {
        return Ok(Finished::Cancelled);
    }
    for archive in archives {
        unpack_dlls(&downloads.join(&archive.path), dir)?;
    }
    std::fs::remove_dir_all(&downloads)?;
    std::fs::write(&marker, installed)?;
    Ok(Finished::Complete)
}

/// Folder inside a runtime folder that holds its archives while they download.
const DOWNLOADS: &str = ".downloads";
/// File inside a runtime folder listing the SHA-256 of each archive installed.
const INSTALLED: &str = ".installed";

/// Copies the DLLs in the `bin` folders of the zip archive `archive` into `dir`.
fn unpack_dlls(archive: &Path, dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let in_bin = path.components().any(|part| part.as_os_str() == "bin");
        let is_dll = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
        // Only the file name is kept, so nothing lands outside `dir`.
        if let (true, true, Some(name)) = (in_bin, is_dll, path.file_name()) {
            std::io::copy(&mut entry, &mut File::create(dir.join(name))?)?;
        }
    }
    Ok(())
}

/// What to download for the speed the user picked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// GPU runtime archives, installed into [`CUDA_RUNTIME_DIR`]; empty when none is needed.
    pub runtime: Vec<RemoteFile>,
    /// Embedding model files, saved into [`EMBEDDING_MODEL_DIR`].
    pub model: Vec<RemoteFile>,
}

impl Plan {
    /// The embedding model, plus the CUDA runtime when `accelerator` needs it.
    pub fn for_accelerator(accelerator: Accelerator) -> Self {
        Self {
            runtime: if accelerator == Accelerator::Cuda {
                cuda_runtime()
            } else {
                Vec::new()
            },
            model: embedding_model(),
        }
    }
}

/// Installs everything in `plan` beneath the cache directory `cache`, reporting
/// progress over all of it as one.
pub fn prepare(
    plan: &Plan,
    cache: &Path,
    http: &dyn Http,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Finished> {
    let runtime_size: u64 = plan.runtime.iter().map(|file| file.size).sum();
    let total = runtime_size + plan.model.iter().map(|file| file.size).sum::<u64>();
    if !plan.runtime.is_empty() {
        let finished = install_runtime(
            &plan.runtime,
            &cache.join(CUDA_RUNTIME_DIR),
            http,
            cancel,
            &mut |runtime| {
                progress(Progress {
                    done: runtime.done,
                    total,
                })
            },
        )?;
        if finished == Finished::Cancelled {
            return Ok(Finished::Cancelled);
        }
    }
    download(
        &plan.model,
        &cache.join(EMBEDDING_MODEL_DIR),
        http,
        cancel,
        &mut |model| {
            progress(Progress {
                done: runtime_size + model.done,
                total,
            })
        },
    )
}

/// How long a download runs before its speed is trusted for an estimate.
const MEASURE_BEFORE_ESTIMATING: Duration = Duration::from_secs(3);

/// Estimates how long a download still takes from its speed so far.
#[derive(Debug, Clone, Copy)]
pub struct TimeLeft {
    /// Bytes already on disk when the download started.
    start: u64,
}

impl TimeLeft {
    pub fn starting_at(done: u64) -> Self {
        Self { start: done }
    }

    /// Time left at `progress`, `elapsed` after the download started.
    pub fn estimate(&self, progress: Progress, elapsed: Duration) -> Option<Duration> {
        let fetched = progress.done.saturating_sub(self.start);
        // The first seconds of a download are too noisy to extrapolate from.
        if fetched == 0 || elapsed < MEASURE_BEFORE_ESTIMATING {
            return None;
        }
        let rate = fetched as f64 / elapsed.as_secs_f64();
        let left = progress.total.saturating_sub(progress.done);
        Some(Duration::from_secs_f64(left as f64 / rate))
    }
}
