//! The desktop app: Tauri commands over the Engine and first-run setup.

use std::path::{Component, Path, PathBuf};

use jiff::civil::Date;
use jiff::{Timestamp, Zoned};
use sbm_core::{AppDirs, Location, ThrottleReason, Unreadable};
use sbm_embed_onnx::Accelerator;
use sbm_engine::{ModelStatus, Status};
use sbm_index::{Exclusion, FailedFile, MatchReason, NotRead, SearchResult};
use sbm_setup::{CUDA_RUNTIME_DIR, Gpu, cuda_runtime, speed_options};
use serde::{Deserialize, Serialize};

mod preview;
pub use preview::{TextPreview, text_preview};
use std::sync::atomic::Ordering;

use tauri::{Emitter, Manager, WindowEvent};
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

    /// Removes the saved settings, so the next start shows onboarding; fine
    /// when there are none.
    pub fn forget(data: &Path) -> std::io::Result<()> {
        match std::fs::remove_file(data.join(SETTINGS_FILE)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
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
    /// The matched page (1-based), when the Location is a page.
    pub page: Option<u32>,
    /// The matched lines (1-based, inclusive), when the Location is lines.
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    /// `name`, `content`, or `both`.
    pub match_reason: String,
    /// A Weaker Match, folded away below the others.
    pub weaker: bool,
    /// Why its content is not searched; `None` when it is, or when not asked.
    pub not_read: Option<NotReadRow>,
}

/// What indexing is doing, as the tray says it.
/// When a pause "until tomorrow" ends: the first 06:00 after `now`, in its time zone.
pub fn next_morning(now: &Zoned) -> Zoned {
    let morning = |date: Date| {
        date.at(6, 0, 0, 0)
            .to_zoned(now.time_zone().clone())
            .expect("a date far from the end of the calendar")
    };
    let today = morning(now.date());
    if today > *now {
        return today;
    }
    morning(
        now.date()
            .tomorrow()
            .expect("not the last day of the calendar"),
    )
}

pub fn indexing_state(status: &Status, now: &Zoned) -> String {
    if status.paused {
        let until = status
            .paused_until
            .and_then(|until| Timestamp::try_from(until).ok());
        let Some(until) = until.map(|until| until.to_zoned(now.time_zone().clone())) else {
            return "Indexing is paused".into();
        };
        let day = if until.date() == now.date() {
            ""
        } else {
            "tomorrow "
        };
        return format!("Indexing is paused until {day}{}", until.strftime("%H:%M"));
    }
    match status.throttled {
        Some(ThrottleReason::FullScreen) => "Indexing waits while a full-screen app is open".into(),
        Some(ThrottleReason::Battery) => "Indexing waits until the computer is plugged in".into(),
        Some(ThrottleReason::Busy) => "Indexing waits while the computer is busy".into(),
        None if status.pending == 0 => "All files are indexed".into(),
        None if status.pending == 1 => "Indexing: 1 file to go".into(),
        None => format!("Indexing: {} files to go", thousands(status.pending)),
    }
}

/// What the Status tab shows (FR-08).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    /// Files in the Name Index.
    pub files: usize,
    /// Files whose content is indexed.
    pub indexed: usize,
    /// Files whose content is still to be indexed.
    pub pending: usize,
    /// Every file modified after this time (Unix milliseconds) has its content indexed.
    pub ready_since: Option<u64>,
    /// What indexing is doing, and why it waits, as the tray says it.
    pub state: String,
    /// Stopped by the user, not held back by the computer.
    pub paused: bool,
    /// Bytes the indexes take on disk.
    pub size_on_disk: u64,
    /// The names in a new location are still being found.
    pub scanning: bool,
    /// The file whose content is being read right now.
    pub current: Option<PathBuf>,
    /// Files whose content could not be read.
    pub failed: usize,
    /// The model that reads what is inside files.
    pub model: ModelRow,
    /// The download content reading waits for, while it runs; set by the command.
    pub download: Option<DownloadRow>,
}

/// Whether the model that reads what is inside files can be used yet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ModelRow {
    /// Still loading, or waiting for its download: files are found by name only.
    Loading,
    /// Reading on `device`, the Accelerator's name, e.g. `cuda` or `cpu`.
    Ready { device: String },
    /// Could not be loaded: files are found by name only.
    Failed { reason: String },
}

/// How far a running download has got, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DownloadRow {
    pub done: u64,
    pub total: u64,
}

pub fn dashboard(status: &Status, now: &Zoned) -> Dashboard {
    Dashboard {
        state: indexing_state(status, now),
        paused: status.paused,
        size_on_disk: status.size_on_disk,
        scanning: status.scanning,
        current: status.current.clone(),
        failed: status.failed,
        model: match &status.model {
            ModelStatus::Loading => ModelRow::Loading,
            ModelStatus::Ready { device } => ModelRow::Ready {
                device: device.clone(),
            },
            ModelStatus::Failed(reason) => ModelRow::Failed {
                reason: reason.clone(),
            },
        },
        download: None,
        files: status.files,
        indexed: status.indexed,
        pending: status.pending,
        ready_since: status
            .ready_since
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_millis() as u64),
    }
}

/// One Exclusion as the Folders tab lists it, and as it comes back to be removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExclusionRow {
    Name {
        pattern: String,
        /// A Default Exclusion, which can be turned off and restored.
        #[serde(default)]
        default: bool,
    },
    Folder {
        path: PathBuf,
        /// A Default Exclusion of the system drive, which can be turned off and restored.
        #[serde(default)]
        default: bool,
    },
}

impl From<ExclusionRow> for Exclusion {
    fn from(row: ExclusionRow) -> Self {
        match row {
            ExclusionRow::Name { pattern, .. } => Exclusion::Name(pattern),
            ExclusionRow::Folder { path, .. } => Exclusion::Folder(path),
        }
    }
}

/// The Exclusions the Folders tab shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExclusionList {
    pub rows: Vec<ExclusionRow>,
    /// Some Default Exclusion is off, so they can be restored.
    pub defaults_turned_off: bool,
}

/// `active` (the Default Exclusions still on, then the user's) as the Folders tab
/// lists them; `defaults` are every Default Exclusion, on or turned off.
pub fn exclusion_list(active: &[Exclusion], defaults: &[Exclusion]) -> ExclusionList {
    ExclusionList {
        defaults_turned_off: defaults.iter().any(|default| !active.contains(default)),
        rows: active
            .iter()
            .map(|exclusion| match exclusion {
                Exclusion::Name(pattern) => ExclusionRow::Name {
                    pattern: pattern.clone(),
                    default: defaults.contains(exclusion),
                },
                Exclusion::Folder(path) => ExclusionRow::Folder {
                    path: path.clone(),
                    default: defaults.contains(exclusion),
                },
            })
            .collect(),
    }
}

/// A file that could not be read, as the Status tab lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FailedRow {
    pub path: PathBuf,
    /// Which plain-language reason the window shows.
    pub reason: &'static str,
    /// The underlying error message, behind Details.
    pub detail: String,
}

pub fn failed_row(failed: FailedFile) -> FailedRow {
    FailedRow {
        path: failed.path,
        reason: reason_name(failed.reason),
        detail: failed.detail,
    }
}

/// The name the window words an `Unreadable` reason by.
fn reason_name(reason: Unreadable) -> &'static str {
    match reason {
        Unreadable::PasswordProtected => "passwordProtected",
        Unreadable::TooLarge => "tooLarge",
        Unreadable::Damaged => "damaged",
        Unreadable::InUse => "inUse",
        Unreadable::NoAccess => "noAccess",
        Unreadable::Gone => "gone",
        Unreadable::Other => "other",
    }
}

/// Why a Result's content is not searched, as the Preview Pane words it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotReadRow {
    OnlineOnly,
    Format,
    Pending,
    Failed { reason: &'static str },
}

pub fn not_read_row(not_read: NotRead) -> NotReadRow {
    match not_read {
        NotRead::OnlineOnly => NotReadRow::OnlineOnly,
        NotRead::Format => NotReadRow::Format,
        NotRead::Pending => NotReadRow::Pending,
        NotRead::Failed(reason) => NotReadRow::Failed {
            reason: reason_name(reason),
        },
    }
}

/// `12345` as `12,345`.
fn thousands(number: usize) -> String {
    let digits = number.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
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
        page: match result.location {
            Some(Location::Page(page)) => Some(page),
            _ => None,
        },
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
        not_read: None,
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

/// What was typed into the launcher: the Query, and the filter prefixes found in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Filters {
    query: String,
    extensions: Vec<String>,
    /// Parts of folder names, as typed; a file must be inside a folder matching each.
    folders: Vec<String>,
    /// From `date:`: when a file must be last modified.
    modified: Option<Period>,
    code_only: bool,
    names_only: bool,
}

impl Filters {
    /// Lifts the filter prefixes (e.g. `ext:pdf`) out of `typed`; the rest is the Query.
    pub fn parse(typed: &str, now: &Zoned) -> Filters {
        let mut words = Vec::new();
        let mut extensions = Vec::new();
        let mut folders = Vec::new();
        let mut modified = None;
        let mut code_only = false;
        let mut names_only = false;
        let typed = words_of(typed);
        for word in &typed {
            let word = word.as_str();
            match word
                .split_once(':')
                .map(|(prefix, value)| (prefix, value.trim_matches('"')))
            {
                // A prefix with nothing after it yet filters nothing.
                Some((prefix, ""))
                    if ["ext", "in", "date"]
                        .iter()
                        .any(|p| prefix.eq_ignore_ascii_case(p)) => {}
                Some((prefix, value)) if prefix.eq_ignore_ascii_case("ext") => {
                    extensions.push(value.trim_start_matches('.').to_lowercase())
                }
                Some((prefix, value)) if prefix.eq_ignore_ascii_case("in") => {
                    folders.push(value.to_string())
                }
                Some((prefix, value)) if prefix.eq_ignore_ascii_case("date") => {
                    match period(value, now) {
                        Some(found) => modified = Some(found),
                        None => words.push(word),
                    }
                }
                // `code:` and `name:` take no value: a word typed right after one is part of the Query.
                Some((prefix, value)) if prefix.eq_ignore_ascii_case("code") => {
                    code_only = true;
                    if !value.is_empty() {
                        words.push(value);
                    }
                }
                Some((prefix, value)) if prefix.eq_ignore_ascii_case("name") => {
                    names_only = true;
                    if !value.is_empty() {
                        words.push(value);
                    }
                }
                _ => words.push(word),
            }
        }
        Filters {
            query: words.join(" "),
            extensions,
            folders,
            modified,
            code_only,
            names_only,
        }
    }

    /// The Query, without its filter prefixes.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Whether `name:` asks to search by name only, leaving content out.
    pub fn names_only(&self) -> bool {
        self.names_only
    }

    /// Whether only filters were typed, no Query words: then the newest files
    /// that pass them are listed.
    pub fn lists_newest(&self) -> bool {
        self.query.is_empty()
            && (self.code_only
                || !self.extensions.is_empty()
                || !self.folders.is_empty()
                || self.modified.is_some())
    }

    /// Whether the file at `path` passes every filter.
    pub fn keeps(&self, path: &Path) -> bool {
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let folder_names: Vec<String> = path
            .parent()
            .into_iter()
            .flat_map(Path::components)
            // Folder names only: the drive and root are not folders.
            .filter_map(|component| match component {
                Component::Normal(name) => Some(name.to_string_lossy().to_lowercase()),
                _ => None,
            })
            .collect();
        (!self.code_only || FileKind::of(path) == FileKind::Code)
            && (self.extensions.is_empty() || self.extensions.contains(&extension))
            && self.folders.iter().all(|part| {
                let part = part.to_lowercase();
                folder_names.iter().any(|name| name.contains(&part))
            })
            && self.modified.as_ref().is_none_or(|period| {
                // Asked only when filtering by date: it touches the file system.
                std::fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|time| Timestamp::try_from(time).ok())
                    .is_some_and(|time| period.start <= time && time < period.end)
            })
    }

    /// One chip per filter, in plain words, for the Query bar.
    pub fn chips(&self) -> Vec<FilterChip> {
        let chip = |kind, label: String| FilterChip { kind, label };
        let mut chips = Vec::new();
        if self.code_only {
            chips.push(chip(FilterKind::Code, "Code".into()));
        }
        if self.names_only {
            chips.push(chip(FilterKind::Name, "Names only".into()));
        }
        for extension in &self.extensions {
            chips.push(chip(FilterKind::Ext, extension.to_uppercase()));
        }
        for folder in &self.folders {
            chips.push(chip(FilterKind::In, format!("In “{folder}”")));
        }
        if let Some(period) = &self.modified {
            chips.push(chip(FilterKind::Date, period.label.clone()));
        }
        chips
    }
}

/// What a search found, with the Query it searched and the filters it kept to.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Searched {
    pub query: String,
    pub results: Vec<ResultRow>,
    pub chips: Vec<FilterChip>,
}

impl Searched {
    pub fn new(filters: &Filters, results: Vec<SearchResult>) -> Searched {
        Searched {
            query: filters.query().to_string(),
            results: results.into_iter().map(result_row).collect(),
            chips: filters.chips(),
        }
    }
}

/// Which filter prefix a chip stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterKind {
    Code,
    Name,
    Ext,
    In,
    Date,
}

/// A filter as the Query bar shows it, e.g. `PDF` or `Last 7 days`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterChip {
    pub kind: FilterKind,
    pub label: String,
}

/// The span of time a `date:` value names: `start..end`.
#[derive(Debug, Clone, PartialEq)]
struct Period {
    start: Timestamp,
    end: Timestamp,
    label: String,
}

/// The span of time a `date:` value names, as of `now`; `None` if it names none.
fn period(value: &str, now: &Zoned) -> Option<Period> {
    let today = now.start_of_day().ok()?;
    let tomorrow = today.tomorrow().ok()?.timestamp();
    let days_back = |days: i64| {
        let start = today.checked_sub(jiff::Span::new().days(days)).ok()?;
        Some(Period {
            start: start.timestamp(),
            end: tomorrow,
            label: format!("Last {days} days"),
        })
    };
    match value.to_lowercase().as_str() {
        "today" => Some(Period {
            start: today.timestamp(),
            end: tomorrow,
            label: "Today".into(),
        }),
        "yesterday" => Some(Period {
            start: today.yesterday().ok()?.timestamp(),
            end: today.timestamp(),
            label: "Yesterday".into(),
        }),
        "week" => days_back(7),
        "month" => days_back(30),
        "year" => days_back(365),
        value => calendar_span(value, now),
    }
}

/// The calendar year (`2025`) or month (`2025-03`) that `value` names, in `now`'s time zone.
fn calendar_span(value: &str, now: &Zoned) -> Option<Period> {
    let number = |digits: &str, length: usize| {
        (digits.len() == length && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| digits.parse::<i16>().ok())
            .flatten()
    };
    let (first, length, label) = match value.split_once('-') {
        None => {
            let first = Date::new(number(value, 4)?, 1, 1).ok()?;
            (first, jiff::Span::new().years(1), first.strftime("%Y"))
        }
        Some((year, month)) => {
            let first = Date::new(number(year, 4)?, number(month, 2)? as i8, 1).ok()?;
            (first, jiff::Span::new().months(1), first.strftime("%B %Y"))
        }
    };
    let start = first.to_zoned(now.time_zone().clone()).ok()?;
    let end = start.checked_add(length).ok()?;
    Some(Period {
        start: start.timestamp(),
        end: end.timestamp(),
        label: label.to_string(),
    })
}

/// The words of `typed`, split at spaces outside double quotes; quotes are kept.
fn words_of(typed: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for character in typed.chars() {
        if character == '"' {
            quoted = !quoted;
        }
        if character.is_whitespace() && !quoted {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// What the window can call; thin wrappers over the functions above.
mod commands {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant, SystemTime};

    use sbm_change_usn::UsnJournal;
    use sbm_core::{AppDirs, SystemActivity};
    use sbm_embed_onnx::{Accelerator, DEFAULT_MAX_TOKENS, OnnxEmbedder};
    use sbm_engine::{Engine, LoadedModel};
    use sbm_index::{Exclusion, View};
    use sbm_pdf::PdfPages;
    use sbm_setup::{
        EMBEDDING_MODEL_DIR, Finished, PDFIUM_DIR, Plan, Progress, TimeLeft, Web,
        prepare as install,
    };
    use serde::Serialize;
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::{
        AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    };
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    use tauri_plugin_opener::OpenerExt;

    use super::{
        Dashboard, DownloadRow, ExclusionList, ExclusionRow, FailedRow, Filters, Searched,
        Settings, SpeedChoice, TextPreview, data_dir, failed_row,
    };

    /// Matryoshka dimensions, as `sbm` uses: 256 scored as well as 512 and 768.
    const DIMENSIONS: usize = 256;
    /// Pixels across a rendered PDF page: the Preview Pane is about 420 px, sharp at 2×.
    const PDF_PAGE_WIDTH: u32 = 840;
    /// Thumbnail pixels on the longer side the frontend may ask for: from a row's
    /// icon to a picture across the Preview Pane at 2× scaling.
    const THUMBNAIL_SIZES: (u32, u32) = (16, 840);
    /// Results the launcher shows, as the PRD's default top-K.
    const RESULTS: usize = 20;
    /// How long the computer must be free (no full-screen app, mains power, not
    /// busy) before Initial Indexing goes on (FR-04).
    const QUIET_PERIOD: Duration = Duration::from_secs(60);
    /// How often the tray's indexing state is brought up to date.
    const TRAY_EVERY: Duration = Duration::from_secs(3);
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
    pub async fn prepare(
        app: AppHandle,
        preparing: State<'_, Preparing>,
        downloading: State<'_, Downloading>,
        choice: String,
    ) -> Result<bool, String> {
        let accelerator: Accelerator = choice.parse()?;
        let downloading = downloading.inner().clone();
        downloading.begin();
        // A fresh flag each time, so an earlier Cancel cannot stop this one.
        let cancel = Arc::new(AtomicBool::new(false));
        *preparing
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = cancel.clone();
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
                downloading.report(progress.done, progress.total);
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
                &cancel,
                &mut report,
            );
            let complete = matches!(finished, Ok(Finished::Complete));
            downloading.end(complete);
            Ok(finished.map_err(|error| error.to_string())? == Finished::Complete)
        })
        .await
        .map_err(|error| error.to_string())?
    }

    /// Whether `prepare` is downloading, and how far it has got; the model loads
    /// only once a download has finished, as onboarding may go on meanwhile and a
    /// stopped download may be tried again.
    #[derive(Clone, Default)]
    pub struct Downloading(Arc<(Mutex<Download>, Condvar)>);

    #[derive(Default)]
    struct Download {
        /// How far the running download has got; `None` when none runs.
        progress: Option<DownloadRow>,
        /// A download has finished: everything search needs is on disk.
        finished: bool,
    }

    impl Downloading {
        fn begin(&self) {
            self.0.0.lock().unwrap().progress = Some(DownloadRow { done: 0, total: 0 });
        }

        fn report(&self, done: u64, total: u64) {
            self.0.0.lock().unwrap().progress = Some(DownloadRow { done, total });
        }

        fn end(&self, finished: bool) {
            let mut download = self.0.0.lock().unwrap();
            download.progress = None;
            download.finished |= finished;
            self.0.1.notify_all();
        }

        /// How far the running download has got, in percent; `None` when none runs.
        pub fn percent(&self) -> Option<u8> {
            self.progress().map(|DownloadRow { done, total }| {
                (done * 100).checked_div(total).unwrap_or(0).min(100) as u8
            })
        }

        /// How far the running download has got; `None` when none runs.
        pub fn progress(&self) -> Option<DownloadRow> {
            self.0.0.lock().unwrap().progress
        }

        /// Blocks until a download has finished.
        fn wait(&self) {
            let _finished = self
                .0
                .1
                .wait_while(self.0.0.lock().unwrap(), |download| !download.finished)
                .unwrap();
        }
    }

    /// The cancel flag of the download `prepare` is running.
    #[derive(Default)]
    pub struct Preparing(Mutex<Arc<AtomicBool>>);

    /// Stops the running download; `prepare` then returns `false`, keeping
    /// what is on disk so the next `prepare` resumes it.
    #[tauri::command]
    pub fn cancel_prepare(preparing: State<'_, Preparing>) {
        let cancel = preparing
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cancel.store(true, Ordering::SeqCst);
    }

    /// What `location` would add to the index, with the Default Exclusions it
    /// would get; counted on a thread of its own, as a big folder takes a while.
    #[tauri::command]
    pub async fn estimate_location(location: PathBuf) -> Result<LocationEstimate, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let mut exclusions = Exclusion::defaults();
            if let Some(drive) = sbm_index::system_drive() {
                exclusions.extend(Exclusion::system_drive_defaults(&drive));
            }
            let found = sbm_index::estimate(&location, &exclusions);
            LocationEstimate {
                files: found.files,
                bytes: found.bytes,
            }
        })
        .await
        .map_err(|error| error.to_string())
    }

    #[derive(Serialize)]
    pub struct LocationEstimate {
        files: u64,
        bytes: u64,
    }

    /// Where onboarding offers to search first: the user's personal folder.
    #[tauri::command]
    pub fn default_location() -> PathBuf {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default()
    }

    /// Asks the user where to search instead; `None` when they picked no folder.
    #[tauri::command]
    pub async fn pick_location(window: WebviewWindow) -> Result<Option<PathBuf>, String> {
        pick_folder(&window, "Choose where to search")
    }

    /// The running Engine, once onboarding has started it.
    #[derive(Default)]
    pub struct Running(Mutex<Option<Engine>>);

    /// Starts the Engine on `choice`'s Accelerator and makes `location` searchable:
    /// names at once, content once the model is loaded.
    #[tauri::command]
    pub fn start_indexing(
        running: State<'_, Running>,
        downloading: State<'_, Downloading>,
        choice: String,
        location: PathBuf,
    ) -> Result<(), String> {
        let engine = start_engine(&choice, Some(downloading.inner().clone()))?;
        engine
            .add_indexed_folder(&location)
            .map_err(|error| error.to_string())?;
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

    /// Makes `hotkey` bring up search from now on, saving it; `false` and the
    /// keys before kept when another app already uses it.
    #[tauri::command]
    pub fn change_hotkey(app: AppHandle, hotkey: String) -> Result<bool, String> {
        let mut settings = Settings::load(&data_dir()).ok_or("onboarding has not finished")?;
        let shortcuts = app.global_shortcut();
        let _ = shortcuts.unregister_all();
        if shortcuts.register(hotkey.as_str()).is_err() {
            let _ = shortcuts.register(settings.hotkey.as_str());
            return Ok(false);
        }
        settings.hotkey = hotkey;
        settings
            .save(&data_dir())
            .map_err(|error| error.to_string())?;
        Ok(true)
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

    /// Erases the index (FR-06) and the saved settings, and starts onboarding
    /// again in the main window. Async: the Engine may finish a batch first.
    #[tauri::command]
    pub async fn purge_index(app: AppHandle) -> Result<(), String> {
        let engine = app.state::<Running>().0.lock().unwrap().take();
        if let Some(engine) = engine {
            engine.purge().map_err(|error| error.to_string())?;
        }
        Settings::forget(&data_dir()).map_err(|error| error.to_string())?;
        let _ = app.global_shortcut().unregister_all();
        app.state::<Launching>().0.store(false, Ordering::Relaxed);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_skip_taskbar(false);
            let _ = window.set_always_on_top(false);
            // Starts the page again, which finds no settings and shows onboarding.
            let _ = window.eval("location.reload()");
            show(&app);
        }
        if let Some(settings) = app.get_webview_window("settings") {
            let _ = settings.close();
        }
        Ok(())
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
        let engine = start_engine(&settings.accelerator, None)?;
        *app.state::<Running>().0.lock().unwrap() = Some(engine);
        app.global_shortcut()
            .register(settings.hotkey.as_str())
            .map_err(|error| error.to_string())
    }

    /// The Engine on `choice`'s Accelerator, falling back to the processor:
    /// names searchable at once, content once the model is loaded.
    /// With `downloading`, the model loads once that download is done.
    fn start_engine(choice: &str, downloading: Option<Downloading>) -> Result<Engine, String> {
        let accelerator: Accelerator = choice.parse()?;
        let dirs = AppDirs::current();
        let model_dir = dirs.cache.join(EMBEDDING_MODEL_DIR);
        let load = move || {
            if let Some(downloading) = downloading {
                downloading.wait();
            }
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
        let engine = Engine::start_loading(&data_dir(), load, Box::new(UsnJournal))
            .map_err(|error| error.to_string())?;
        engine.set_activity(Arc::new(SystemActivity::default()), QUIET_PERIOD);
        Ok(engine)
    }

    /// Results for what was typed in File view, best first, within its filter
    /// prefixes (only filters: the newest files that pass them); none before
    /// the Engine has started.
    #[tauri::command]
    pub async fn search(running: State<'_, Running>, typed: String) -> Result<Searched, String> {
        let filters = Filters::parse(&typed, &jiff::Zoned::now());
        let running = running.0.lock().unwrap();
        let Some(engine) = running.as_ref() else {
            return Ok(Searched::new(&filters, Vec::new()));
        };
        let keep = |path: &Path| filters.keeps(path);
        let results = if filters.lists_newest() {
            engine.newest_files(RESULTS, &keep)
        } else if filters.names_only() {
            engine.search_names(filters.query(), RESULTS, &keep)
        } else {
            engine.search_where(filters.query(), View::File, RESULTS, &keep)
        }
        .map_err(|error| error.to_string())?;
        let mut searched = Searched::new(&filters, results);
        // A Content Match was read; a Disconnected Drive's volume cannot be asked.
        for row in &mut searched.results {
            if row.connected && row.match_reason == "name" {
                row.not_read = engine
                    .not_read(&row.path)
                    .ok()
                    .flatten()
                    .map(super::not_read_row);
            }
        }
        Ok(searched)
    }

    /// What the Status tab shows; an error before the Engine has started.
    #[tauri::command]
    pub async fn dashboard(
        running: State<'_, Running>,
        downloading: State<'_, Downloading>,
    ) -> Result<Dashboard, String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        let status = engine.status().map_err(|error| error.to_string())?;
        Ok(Dashboard {
            download: downloading.progress(),
            ..super::dashboard(&status, &jiff::Zoned::now())
        })
    }

    #[tauri::command]
    pub async fn failed_files(running: State<'_, Running>) -> Result<Vec<FailedRow>, String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        let failed = engine.failed_files().map_err(|error| error.to_string())?;
        Ok(failed.into_iter().map(failed_row).collect())
    }

    /// Reads the files that could not be read once more.
    #[tauri::command]
    pub async fn retry_failed(running: State<'_, Running>) -> Result<(), String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine.retry_failed().map_err(|error| error.to_string())
    }

    /// A folder whose files are indexed, as the Folders tab lists it.
    #[derive(Serialize)]
    pub struct IndexedFolder {
        path: PathBuf,
        connected: bool,
    }

    #[tauri::command]
    pub async fn indexed_folders(
        running: State<'_, Running>,
    ) -> Result<Vec<IndexedFolder>, String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        let folders = engine
            .indexed_folders()
            .map_err(|error| error.to_string())?;
        Ok(folders
            .into_iter()
            .map(|folder| IndexedFolder {
                path: folder.path,
                connected: folder.connected,
            })
            .collect())
    }

    /// Asks the user for a folder and indexes it; false when they picked none.
    #[tauri::command]
    pub async fn pick_and_add_folder(
        window: WebviewWindow,
        running: State<'_, Running>,
    ) -> Result<bool, String> {
        let Some(folder) = pick_folder(&window, "Choose a folder to search")? else {
            return Ok(false);
        };
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .add_indexed_folder(&folder)
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    /// Asks the user for a folder and excludes it; false when they picked none.
    #[tauri::command]
    pub async fn pick_and_exclude_folder(
        window: WebviewWindow,
        running: State<'_, Running>,
    ) -> Result<bool, String> {
        let Some(folder) = pick_folder(&window, "Choose a folder to leave out")? else {
            return Ok(false);
        };
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .add_exclusion(Exclusion::Folder(folder))
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    /// The folder the user picks in the system's folder picker over `window`.
    fn pick_folder(window: &WebviewWindow, title: &str) -> Result<Option<PathBuf>, String> {
        let picked = window
            .dialog()
            .file()
            .set_parent(window)
            .set_title(title)
            .blocking_pick_folder();
        picked
            .map(|folder| folder.into_path().map_err(|error| error.to_string()))
            .transpose()
    }

    #[tauri::command]
    pub async fn exclusions(running: State<'_, Running>) -> Result<ExclusionList, String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        let active = engine.exclusions().map_err(|error| error.to_string())?;
        Ok(super::exclusion_list(&active, &engine.default_exclusions()))
    }

    /// Forgets one of the user's Exclusions, or turns off a default one.
    #[tauri::command]
    pub async fn remove_exclusion(
        running: State<'_, Running>,
        row: ExclusionRow,
    ) -> Result<(), String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .remove_exclusion(&row.into())
            .map_err(|error| error.to_string())
    }

    /// Excludes files and folders whose name matches `pattern`.
    #[tauri::command]
    pub async fn exclude_names(running: State<'_, Running>, pattern: String) -> Result<(), String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .add_exclusion(Exclusion::Name(pattern))
            .map_err(|error| error.to_string())
    }

    /// Turns every Default Exclusion back on.
    #[tauri::command]
    pub async fn restore_default_exclusions(running: State<'_, Running>) -> Result<(), String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .restore_default_exclusions()
            .map_err(|error| error.to_string())
    }

    /// Stops indexing `path`; its files leave both indexes.
    #[tauri::command]
    pub async fn remove_indexed_folder(
        running: State<'_, Running>,
        path: PathBuf,
    ) -> Result<(), String> {
        let running = running.0.lock().unwrap();
        let engine = running.as_ref().ok_or("search has not started yet")?;
        engine
            .remove_indexed_folder(&path)
            .map_err(|error| error.to_string())
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

    /// The system's thumbnail of `path`, `size` pixels at most on its longer
    /// side, as a `data:` URL, if it has one. For an Online-only File only an
    /// existing thumbnail is used, so nothing downloads.
    #[tauri::command]
    pub async fn thumbnail(path: PathBuf, size: u32) -> Option<String> {
        let size = size.clamp(THUMBNAIL_SIZES.0, THUMBNAIL_SIZES.1);
        tauri::async_runtime::spawn_blocking(move || {
            use base64::Engine as _;
            use sbm_core::FileAvailability;
            let cached_only = sbm_core::SystemFileAvailability.is_online_only(&path);
            let png = sbm_thumbnail::thumbnail(&path, size, cached_only)?;
            let base64 = base64::engine::general_purpose::STANDARD.encode(png);
            Some(format!("data:image/png;base64,{base64}"))
        })
        .await
        .ok()
        .flatten()
    }

    /// A page of a PDF as the Preview Pane shows it.
    #[derive(Serialize)]
    pub struct PdfPagePreview {
        /// The page as a PNG `data:` URL.
        image: String,
        /// Where the Query's words sit, in fractions of the page from its top left.
        marks: Vec<Mark>,
    }

    #[derive(Serialize)]
    pub struct Mark {
        left: f32,
        top: f32,
        width: f32,
        height: f32,
    }

    /// Page `page` of the PDF `path`, with the words of `query` marked on it;
    /// `None` when PDFium is missing or the page cannot be read.
    #[tauri::command]
    pub async fn pdf_page(path: PathBuf, page: u32, query: String) -> Option<PdfPagePreview> {
        tauri::async_runtime::spawn_blocking(move || {
            use base64::Engine as _;
            let pdfium = PdfPages::load(&AppDirs::current().cache.join(PDFIUM_DIR)).ok()?;
            let words: Vec<&str> = query
                .split(|c: char| !c.is_alphanumeric())
                .filter(|word| !word.is_empty())
                .collect();
            let rendered = pdfium.page(&path, page, PDF_PAGE_WIDTH, &words)?;
            let base64 = base64::engine::general_purpose::STANDARD.encode(rendered.png);
            Some(PdfPagePreview {
                image: format!("data:image/png;base64,{base64}"),
                marks: rendered
                    .marks
                    .into_iter()
                    .map(|area| Mark {
                        left: area.left,
                        top: area.top,
                        width: area.width,
                        height: area.height,
                    })
                    .collect(),
            })
        })
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

    /// Brings up the launcher with `typed` in its Query box, as Home's search does.
    #[tauri::command]
    pub fn open_search(app: AppHandle, typed: String) {
        show(&app);
        let _ = app.emit_to("main", "search-for", typed);
    }

    /// Brings up the Home window, opening it the first time.
    pub fn open_settings(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("settings") {
            let _ = window.unminimize();
            let _ = window.set_focus();
            return;
        }
        let window = WebviewWindowBuilder::new(
            app,
            "settings",
            WebviewUrl::App("index.html?view=settings".into()),
        )
        .title("Search by Memory")
        .inner_size(1080.0, 720.0)
        .min_inner_size(820.0, 560.0)
        // Glass like the launcher: no frame, the desktop blurred behind it.
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .effects(
            tauri::window::EffectsBuilder::new()
                .effect(tauri::window::Effect::Acrylic)
                .build(),
        )
        .center()
        .build();
        if let Err(error) = window {
            eprintln!("the Settings window could not be opened: {error}");
        }
    }

    /// Pauses indexing until `when` or until the user resumes it; the Engine
    /// resumes by itself.
    fn pause_until(app: &AppHandle, when: SystemTime) {
        with_engine(app, |engine| engine.pause_until(when));
    }

    /// Pauses indexing until the user resumes it, as the Settings window does.
    #[tauri::command]
    pub fn pause_indexing(app: AppHandle) {
        with_engine(&app, Engine::pause);
    }

    #[tauri::command]
    pub fn resume_indexing(app: AppHandle) {
        with_engine(&app, Engine::resume);
    }

    /// Runs `action` on the Engine, once onboarding has started it.
    fn with_engine<T>(app: &AppHandle, action: impl FnOnce(&Engine) -> T) -> Option<T> {
        app.state::<Running>()
            .0
            .lock()
            .unwrap()
            .as_ref()
            .map(action)
    }

    /// The tray icon: the way back to the hidden window, and out of the app.
    pub fn tray(app: &AppHandle) -> tauri::Result<()> {
        // Not clickable: says what indexing is doing, and why it waits.
        let state = MenuItem::with_id(app, "state", "Search by Memory", false, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[
                &state,
                &PredefinedMenuItem::separator(app)?,
                &MenuItem::with_id(app, "show", "Search", true, None::<&str>)?,
                &MenuItem::with_id(app, "settings", "Open Home", true, None::<&str>)?,
                &MenuItem::with_id(
                    app,
                    "pause-1h",
                    "Pause indexing for 1 hour",
                    true,
                    None::<&str>,
                )?,
                &MenuItem::with_id(
                    app,
                    "pause-8h",
                    "Pause indexing for 8 hours",
                    true,
                    None::<&str>,
                )?,
                &MenuItem::with_id(
                    app,
                    "pause-tomorrow",
                    "Pause indexing until tomorrow",
                    true,
                    None::<&str>,
                )?,
                &MenuItem::with_id(app, "resume", "Resume indexing", true, None::<&str>)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ],
        )?;
        let mut tray = TrayIconBuilder::new()
            .tooltip("Search by Memory")
            .menu(&menu)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show(app),
                "settings" => open_settings(app),
                "pause-1h" => pause_until(app, SystemTime::now() + Duration::from_secs(60 * 60)),
                "pause-8h" => {
                    pause_until(app, SystemTime::now() + Duration::from_secs(8 * 60 * 60))
                }
                "pause-tomorrow" => pause_until(
                    app,
                    super::next_morning(&jiff::Zoned::now()).timestamp().into(),
                ),
                "resume" => resume_indexing(app.clone()),
                "quit" => app.exit(0),
                _ => {}
            });
        if let Some(icon) = app.default_window_icon() {
            tray = tray.icon(icon.clone());
        }
        let tray = tray.build(app)?;
        let app = app.clone();
        std::thread::spawn(move || {
            loop {
                // A download onboarding went on from comes first: content waits for it.
                let text = match app.state::<Downloading>().percent() {
                    Some(percent) => Some(format!("Downloading what search needs: {percent}%")),
                    None => match with_engine(&app, Engine::status) {
                        Some(Ok(status)) => {
                            Some(super::indexing_state(&status, &jiff::Zoned::now()))
                        }
                        _ => None,
                    },
                };
                if let Some(text) = text {
                    let _ = state.set_text(&text);
                    let _ = tray.set_tooltip(Some(format!("Search by Memory: {text}")));
                }
                std::thread::sleep(TRAY_EVERY);
            }
        });
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
                        // Onboarding's keys step listens, to say the keys work.
                        let _ = app.emit("search-keys-pressed", ());
                        commands::show(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Running::default())
        .manage(commands::Launching::default())
        .manage(commands::Preparing::default())
        .manage(commands::Downloading::default())
        .on_window_event(|window, event| {
            let launching = window
                .state::<commands::Launching>()
                .0
                .load(Ordering::Relaxed);
            // Only the launcher hides on blur; the Settings window stays open.
            if launching && window.label() == "main" && matches!(event, WindowEvent::Focused(false))
            {
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
            commands::cancel_prepare,
            commands::default_location,
            commands::pick_location,
            commands::estimate_location,
            commands::retry_failed,
            commands::start_indexing,
            commands::settings,
            commands::try_hotkey,
            commands::finish_onboarding,
            commands::search,
            commands::dashboard,
            commands::failed_files,
            commands::indexed_folders,
            commands::pick_and_add_folder,
            commands::remove_indexed_folder,
            commands::exclusions,
            commands::remove_exclusion,
            commands::restore_default_exclusions,
            commands::exclude_names,
            commands::pick_and_exclude_folder,
            commands::pause_indexing,
            commands::resume_indexing,
            commands::open_file,
            commands::reveal_file,
            commands::copy_file,
            commands::preview_lines,
            commands::thumbnail,
            commands::pdf_page,
            commands::purge_index,
            commands::quit,
            commands::hide_launcher,
            commands::open_search,
            commands::change_hotkey
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
