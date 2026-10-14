//! The desktop app: Tauri commands over the Engine and first-run setup.

use std::path::{Path, PathBuf};

use sbm_core::{AppDirs, Location};
use sbm_embed_onnx::Accelerator;
use sbm_index::{MatchReason, SearchResult};
use sbm_setup::{CUDA_RUNTIME_DIR, Gpu, cuda_runtime, speed_options};
use serde::{Deserialize, Serialize};

mod preview;
pub use preview::{TextPreview, text_preview};
use std::sync::atomic::Ordering;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

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

/// What onboarding decided; saved when it finishes, so the next start skips it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The picked Accelerator's name, e.g. `cuda`.
    pub accelerator: String,
    /// The global shortcut that brings up search, e.g. `Alt+Space`.
    pub hotkey: String,
}

const SETTINGS_FILE: &str = "settings.json";

impl Settings {
    /// The settings saved in the data directory `data`; `None` until onboarding
    /// has finished (or if the file cannot be read).
    pub fn load(data: &Path) -> Option<Settings> {
        let json = std::fs::read(data.join(SETTINGS_FILE)).ok()?;
        serde_json::from_slice(&json).ok()
    }

    pub fn save(&self, data: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(data)?;
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(data.join(SETTINGS_FILE), json)
    }
}

/// What a file is, so the launcher can group Results by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Document,
    Code,
    Image,
    Audio,
    Video,
    Other,
}

impl FileKind {
    /// The kind of file `path` is, by its extension.
    pub fn of(path: &Path) -> FileKind {
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match extension.as_str() {
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "markdown" | "ppt" | "pptx"
            | "odp" | "xls" | "xlsx" | "ods" | "csv" | "epub" => FileKind::Document,
            "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "c" | "h" | "cpp" | "hpp" | "cc"
            | "cs" | "java" | "kt" | "swift" | "rb" | "php" | "sh" | "ps1" | "lua" | "json"
            | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "sql" | "ipynb" => FileKind::Code,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "heic" | "svg" | "tif" | "tiff" => {
                FileKind::Image
            }
            "mp3" | "m4a" | "wav" | "flac" | "ogg" | "aac" | "opus" | "wma" => FileKind::Audio,
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "wmv" | "m4v" => FileKind::Video,
            _ => FileKind::Other,
        }
    }
}

/// One Result as the launcher shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRow {
    /// The file name, e.g. `Q3 Report.pdf`.
    pub name: String,
    pub path: PathBuf,
    /// The folder holding `path`.
    pub folder: PathBuf,
    pub kind: FileKind,
    /// Other paths with the same content.
    pub copies: Vec<PathBuf>,
    /// False while the file's drive is a Disconnected Drive.
    pub connected: bool,
    /// Bytes; `None` when the file cannot be read now.
    pub size: Option<u64>,
    /// Last change, in milliseconds since 1970; `None` when the file cannot be read now.
    pub modified: Option<u64>,
    /// Verbatim text of the Content Match, if any.
    pub snippet: Option<String>,
    /// Where the snippet is, e.g. `Page 14`; `None` for the whole file or a Name Match.
    pub location: Option<String>,
    /// The matched lines (1-based, inclusive), when the Location is lines.
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    /// `name`, `content`, or `both`.
    pub match_reason: String,
    /// A Weaker Match, folded away below the others.
    pub weaker: bool,
}

pub fn result_row(result: SearchResult) -> ResultRow {
    // A Disconnected Drive is not asked: a missing network share can take long to answer.
    let metadata = result
        .connected
        .then(|| std::fs::metadata(&result.path).ok())
        .flatten();
    ResultRow {
        kind: FileKind::of(&result.path),
        weaker: result.weaker,
        folder: result
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
        size: metadata.as_ref().map(|metadata| metadata.len()),
        modified: metadata
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_millis() as u64),
        name: result
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        location: result.location.as_ref().and_then(location_label),
        line_start: match result.location {
            Some(Location::Lines { start, .. }) => Some(start),
            _ => None,
        },
        line_end: match result.location {
            Some(Location::Lines { end, .. }) => Some(end),
            _ => None,
        },
        match_reason: match result.match_reason {
            MatchReason::Name => "name",
            MatchReason::Content => "content",
            MatchReason::Both => "both",
        }
        .into(),
        path: result.path,
        copies: result.copies,
        connected: result.connected,
        snippet: result.snippet,
    }
}

fn location_label(location: &Location) -> Option<String> {
    match location {
        Location::Page(page) => Some(format!("Page {page}")),
        Location::Lines { start, end } if start == end => Some(format!("Line {start}")),
        Location::Lines { start, end } => Some(format!("Lines {start}–{end}")),
        Location::Time { start, .. } => {
            let seconds = start.as_secs();
            Some(format!("{}:{:02}", seconds / 60, seconds % 60))
        }
        Location::WholeFile => None,
    }
}

/// What the window can call; thin wrappers over the functions above.
mod commands {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use sbm_change_usn::UsnJournal;
    use sbm_core::AppDirs;
    use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};
    use sbm_engine::{Engine, LoadedModel};
    use sbm_index::View;
    use sbm_setup::{
        EMBEDDING_MODEL_DIR, Finished, Plan, Progress, TimeLeft, Web, prepare as install,
    };
    use serde::Serialize;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    use tauri_plugin_opener::OpenerExt;

    use super::{FolderChoice, ResultRow, Settings, SpeedChoice, TextPreview, data_dir};

    /// Matryoshka dimensions, as `sbm` uses: 256 scored as well as 512 and 768.
    const DIMENSIONS: usize = 256;
    /// Results the launcher shows, as the PRD's default top-K.
    const RESULTS: usize = 20;
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
        let engine = start_engine(&choice)?;
        for folder in &folders {
            engine
                .add_indexed_folder(folder)
                .map_err(|error| error.to_string())?;
        }
        *running.0.lock().unwrap() = Some(engine);
        Ok(())
    }

    /// Registers `hotkey` for search in place of the one before; `false` when
    /// another app already uses it (or it is not a shortcut).
    #[tauri::command]
    pub fn try_hotkey(app: AppHandle, hotkey: String) -> bool {
        let shortcuts = app.global_shortcut();
        let _ = shortcuts.unregister_all();
        shortcuts.register(hotkey.as_str()).is_ok()
    }

    /// What onboarding decided; `None` until it has finished.
    #[tauri::command]
    pub fn settings() -> Option<Settings> {
        Settings::load(&data_dir())
    }

    /// Saves what onboarding decided and hides the window until the hotkey.
    #[tauri::command]
    pub fn finish_onboarding(
        window: WebviewWindow,
        choice: String,
        hotkey: String,
    ) -> Result<(), String> {
        let settings = Settings {
            accelerator: choice,
            hotkey,
        };
        settings
            .save(&data_dir())
            .map_err(|error| error.to_string())?;
        become_launcher(&window).map_err(|error| error.to_string())?;
        window.hide().map_err(|error| error.to_string())
    }

    /// Ends the app, indexing included.
    #[tauri::command]
    pub fn quit(app: AppHandle) {
        app.exit(0);
    }

    /// From now on the window is the launcher: out of the taskbar, above other
    /// windows, and gone when the user clicks elsewhere.
    pub fn become_launcher(window: &WebviewWindow) -> tauri::Result<()> {
        window
            .app_handle()
            .state::<Launching>()
            .0
            .store(true, Ordering::Relaxed);
        window.set_skip_taskbar(true)?;
        window.set_always_on_top(true)
    }

    /// Whether onboarding is over and the window acts as the launcher.
    #[derive(Default)]
    pub struct Launching(pub AtomicBool);

    /// Starts again from finished onboarding: the Engine on the saved
    /// Accelerator, and the saved hotkey; the window stays hidden.
    pub fn resume(app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let engine = start_engine(&settings.accelerator)?;
        *app.state::<Running>().0.lock().unwrap() = Some(engine);
        app.global_shortcut()
            .register(settings.hotkey.as_str())
            .map_err(|error| error.to_string())
    }

    /// The Engine on `choice`'s Accelerator, falling back to the processor:
    /// names searchable at once, content once the model is loaded.
    fn start_engine(choice: &str) -> Result<Engine, String> {
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
        Engine::start_loading(&data_dir(), load, Box::new(UsnJournal))
            .map_err(|error| error.to_string())
    }

    /// Results for `query` in `view` (`file` or `passage`), best first; none
    /// before the Engine has started.
    #[tauri::command]
    pub async fn search(
        running: State<'_, Running>,
        query: String,
        view: String,
    ) -> Result<Vec<ResultRow>, String> {
        let view = if view == "passage" {
            View::Passage
        } else {
            View::File
        };
        let running = running.0.lock().unwrap();
        let Some(engine) = running.as_ref() else {
            return Ok(Vec::new());
        };
        let results = engine
            .search(&query, view, RESULTS)
            .map_err(|error| error.to_string())?;
        Ok(results.into_iter().map(super::result_row).collect())
    }

    /// The lines of `path` around `lineStart`–`lineEnd`, or its first lines.
    #[tauri::command]
    pub async fn preview_lines(
        path: PathBuf,
        line_start: Option<u32>,
        line_end: Option<u32>,
    ) -> Option<TextPreview> {
        let matched = line_start.zip(line_end);
        tauri::async_runtime::spawn_blocking(move || super::text_preview(&path, matched))
            .await
            .ok()
            .flatten()
    }

    /// Opens `path` with its default app.
    #[tauri::command]
    pub fn open_file(app: AppHandle, path: PathBuf) -> Result<(), String> {
        app.opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|error| error.to_string())
    }

    /// Shows `path` selected in its folder, without opening it.
    #[tauri::command]
    pub fn reveal_file(app: AppHandle, path: PathBuf) -> Result<(), String> {
        app.opener()
            .reveal_item_in_dir(path)
            .map_err(|error| error.to_string())
    }

    /// Puts the file itself on the clipboard, to paste in a folder or a chat.
    #[tauri::command]
    pub fn copy_file(path: PathBuf) -> Result<(), String> {
        #[cfg(windows)]
        {
            use clipboard_win::{Clipboard, Setter, formats::FileList, raw};
            let path = path.to_string_lossy();
            let _open = Clipboard::new_attempts(10).map_err(|error| error.to_string())?;
            raw::empty().map_err(|error| error.to_string())?;
            FileList
                .write_clipboard(&[path.as_ref()])
                .map_err(|error| error.to_string())
        }
        #[cfg(not(windows))]
        {
            Err(format!(
                "copying files is not supported here: {}",
                path.display()
            ))
        }
    }

    /// Hides the launcher; indexing goes on.
    #[tauri::command]
    pub fn hide_launcher(window: WebviewWindow) -> Result<(), String> {
        window.hide().map_err(|error| error.to_string())
    }

    /// Brings the window to the front, as the hotkey and the tray do.
    pub fn show(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }

    /// The tray icon: the way back to the hidden window, and out of the app.
    pub fn tray(app: &AppHandle) -> tauri::Result<()> {
        let menu = Menu::with_items(
            app,
            &[
                &MenuItem::with_id(app, "show", "Search", true, None::<&str>)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ],
        )?;
        let mut tray = TrayIconBuilder::new()
            .tooltip("Search by Memory")
            .menu(&menu)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show(app),
                "quit" => app.exit(0),
                _ => {}
            });
        if let Some(icon) = app.default_window_icon() {
            tray = tray.icon(icon.clone());
        }
        tray.build(app)?;
        Ok(())
    }
}

pub fn run() {
    put_cuda_runtime_on_path();
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        commands::show(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .manage(commands::Running::default())
        .manage(commands::Launching::default())
        .on_window_event(|window, event| {
            let launching = window
                .state::<commands::Launching>()
                .0
                .load(Ordering::Relaxed);
            if launching && matches!(event, WindowEvent::Focused(false)) {
                let _ = window.hide();
            }
        })
        .setup(|app| {
            commands::tray(app.handle())?;
            if let Some(settings) = Settings::load(&data_dir()) {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                    commands::become_launcher(&window)?;
                }
                if let Err(error) = commands::resume(app.handle(), &settings) {
                    eprintln!("search could not start: {error}");
                    commands::show(app.handle());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::speed_choices,
            commands::prepare,
            commands::folder_choices,
            commands::start_indexing,
            commands::settings,
            commands::try_hotkey,
            commands::finish_onboarding,
            commands::search,
            commands::open_file,
            commands::reveal_file,
            commands::copy_file,
            commands::preview_lines,
            commands::quit,
            commands::hide_launcher
        ])
        .run(tauri::generate_context!())
        .expect("the app window could not be opened");
}

/// Where the index and settings live: `$SBM_DATA_DIR`, as for `sbm`, or the
/// platform's data directory.
fn data_dir() -> PathBuf {
    std::env::var_os("SBM_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| AppDirs::current().data)
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
