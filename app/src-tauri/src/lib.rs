//! The desktop app: Tauri commands over the Engine and first-run setup.

use std::path::{Path, PathBuf};

use sbm_core::AppDirs;
use sbm_embed_onnx::Accelerator;
use sbm_setup::{CUDA_RUNTIME_DIR, Gpu, cuda_runtime, speed_options};
use serde::Serialize;

/// One way to run the embedding model, as the onboarding speed step offers it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedChoice {
    /// The Accelerator's name, e.g. `cuda`; picked choices come back by it.
    pub id: String,
    /// The graphics card it uses; `None` for the processor.
    pub graphics_card: Option<String>,
    pub recommended: bool,
    /// Bytes to download before it can be used.
    pub extra_download: u64,
}

/// The speed step's choices on a machine with `gpus`, fastest first.
pub fn speed_choices(gpus: &[Gpu]) -> Vec<SpeedChoice> {
    speed_options(gpus)
        .into_iter()
        .map(|option| SpeedChoice {
            id: option.accelerator.to_string(),
            graphics_card: option.gpu.map(|gpu| gpu.name),
            recommended: option.recommended,
            extra_download: if option.accelerator == Accelerator::Cuda {
                cuda_runtime().iter().map(|file| file.size).sum()
            } else {
                0
            },
        })
        .collect()
}

/// What a folder offered in onboarding is, so the UI can name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FolderKind {
    /// The user's own folder: Documents, Desktop, Downloads, and the rest.
    Personal,
    /// A whole drive.
    Drive,
}

/// A folder the onboarding folders step offers to make searchable.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChoice {
    pub path: PathBuf,
    pub kind: FolderKind,
    /// Picked until the user unpicks it.
    pub picked: bool,
}

/// The personal folder `home`, picked, then every drive in `drives` that does
/// not hold it, offered but not picked.
pub fn folder_choices(home: &Path, drives: &[PathBuf]) -> Vec<FolderChoice> {
    let personal = FolderChoice {
        path: home.to_path_buf(),
        kind: FolderKind::Personal,
        picked: true,
    };
    let others = drives
        .iter()
        .filter(|drive| !home.starts_with(drive))
        .map(|drive| FolderChoice {
            path: drive.clone(),
            kind: FolderKind::Drive,
            picked: false,
        });
    std::iter::once(personal).chain(others).collect()
}

/// What the window can call; thin wrappers over the functions above.
mod commands {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use sbm_change_usn::UsnJournal;
    use sbm_core::AppDirs;
    use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};
    use sbm_engine::{Engine, LoadedModel};
    use sbm_setup::{
        EMBEDDING_MODEL_DIR, Finished, Plan, Progress, TimeLeft, Web, prepare as install,
    };
    use serde::Serialize;
    use tauri::{AppHandle, Emitter, State};

    use super::{FolderChoice, SpeedChoice};

    /// Matryoshka dimensions, as `sbm` uses: 256 scored as well as 512 and 768.
    const DIMENSIONS: usize = 256;
    /// Progress events per second at most; the bar cannot show more.
    const REPORT_EVERY: Duration = Duration::from_millis(200);

    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DownloadProgress {
        done: u64,
        total: u64,
        seconds_left: Option<u64>,
    }

    #[tauri::command]
    pub fn speed_choices() -> Vec<SpeedChoice> {
        super::speed_choices(&sbm_setup::system_gpus())
    }

    /// Downloads what `choice` needs, reporting `setup-progress` events.
    #[tauri::command]
    pub async fn prepare(app: AppHandle, choice: String) -> Result<bool, String> {
        let accelerator: Accelerator = choice.parse()?;
        tauri::async_runtime::spawn_blocking(move || {
            let plan = Plan::for_accelerator(accelerator);
            let started = Instant::now();
            let (mut time_left, mut last_report) = (None, None::<Instant>);
            let mut report = |progress: Progress| {
                let time_left = *time_left.get_or_insert(TimeLeft::starting_at(progress.done));
                let finished = progress.done == progress.total;
                if !finished && last_report.is_some_and(|last| last.elapsed() < REPORT_EVERY) {
                    return;
                }
                last_report = Some(Instant::now());
                let _ = app.emit(
                    "setup-progress",
                    DownloadProgress {
                        done: progress.done,
                        total: progress.total,
                        seconds_left: time_left
                            .estimate(progress, started.elapsed())
                            .map(|left| left.as_secs()),
                    },
                );
            };
            let finished = install(
                &plan,
                &AppDirs::current().cache,
                &Web::new(),
                &AtomicBool::new(false),
                &mut report,
            )
            .map_err(|error| error.to_string())?;
            Ok(finished == Finished::Complete)
        })
        .await
        .map_err(|error| error.to_string())?
    }

    #[tauri::command]
    pub fn folder_choices() -> Vec<FolderChoice> {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        super::folder_choices(&home, &drives())
    }

    /// Drive roots present on this machine (Windows only).
    fn drives() -> Vec<PathBuf> {
        if !cfg!(windows) {
            return Vec::new();
        }
        ('A'..='Z')
            .map(|letter| PathBuf::from(format!("{letter}:\\")))
            .filter(|root| root.exists())
            .collect()
    }

    /// The running Engine, once onboarding has started it.
    #[derive(Default)]
    pub struct Running(Mutex<Option<Engine>>);

    /// Starts the Engine on `choice`'s Accelerator and makes `folders` searchable:
    /// names at once, content once the model is loaded.
    #[tauri::command]
    pub fn start_indexing(
        running: State<'_, Running>,
        choice: String,
        folders: Vec<PathBuf>,
    ) -> Result<(), String> {
        let accelerator: Accelerator = choice.parse()?;
        let dirs = AppDirs::current();
        let model_dir = dirs.cache.join(EMBEDDING_MODEL_DIR);
        let load = move || {
            let mut accelerators = vec![accelerator];
            if accelerator != Accelerator::Cpu {
                accelerators.push(Accelerator::Cpu);
            }
            let embedder = OnnxEmbedder::load_first(
                &model_dir,
                DIMENSIONS,
                DEFAULT_MAX_TOKENS,
                &accelerators,
            )?;
            let device = embedder.accelerator().to_string();
            Ok(LoadedModel {
                embedder: Arc::new(embedder),
                device,
            })
        };
        let engine = Engine::start_loading(&dirs.data, load, Box::new(UsnJournal))
            .map_err(|error| error.to_string())?;
        for folder in &folders {
            engine
                .add_indexed_folder(folder)
                .map_err(|error| error.to_string())?;
        }
        *running.0.lock().unwrap() = Some(engine);
        Ok(())
    }
}

pub fn run() {
    put_cuda_runtime_on_path();
    tauri::Builder::default()
        .manage(commands::Running::default())
        .invoke_handler(tauri::generate_handler![
            commands::speed_choices,
            commands::prepare,
            commands::folder_choices,
            commands::start_indexing
        ])
        .run(tauri::generate_context!())
        .expect("the app window could not be opened");
}

/// ONNX Runtime's CUDA provider finds the CUDA and cuDNN DLLs on `PATH`.
fn put_cuda_runtime_on_path() {
    let runtime = AppDirs::current().cache.join(CUDA_RUNTIME_DIR);
    let mut paths = vec![runtime];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    if let Ok(path) = std::env::join_paths(paths) {
        // SAFETY: called first thing in `run`, before any other thread exists.
        unsafe { std::env::set_var("PATH", path) };
    }
}
