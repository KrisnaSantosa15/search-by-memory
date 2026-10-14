//! Domain types and provider interfaces for Search by Memory.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod system_activity;
mod system_file_availability;
mod system_volumes;

pub use system_activity::SystemActivity;
pub use system_file_availability::SystemFileAvailability;
pub use system_volumes::SystemVolumes;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

/// A position inside a file the user can navigate to.
#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    /// 1-based page of a paged document.
    Page(u32),
    /// Inclusive, 1-based line range of text or code.
    Lines { start: u32, end: u32 },
    /// Time range of audio or video, from the start of the media.
    Time { start: Duration, end: Duration },
    /// The whole file, for content without inner positions such as images.
    WholeFile,
}

impl Location {
    /// Whether both Locations point at some shared part of the same file.
    pub fn overlaps(&self, other: &Location) -> bool {
        use Location::*;
        match (self, other) {
            (WholeFile, _) | (_, WholeFile) => true,
            (Page(a), Page(b)) => a == b,
            (Lines { start: a1, end: a2 }, Lines { start: b1, end: b2 }) => a1 <= b2 && b1 <= a2,
            (Time { start: a1, end: a2 }, Time { start: b1, end: b2 }) => a1 <= b2 && b1 <= a2,
            _ => false,
        }
    }
}

/// Extracted text of one part of a file, such as a page, with its Location.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub location: Location,
    pub text: String,
}

/// Why the content of a file could not be read, in terms a user can act on (FR-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// Encrypted; it opens only with a password.
    PasswordProtected,
    /// More content than the extractors read from one file.
    TooLarge,
    /// Not a valid file of its format, or cut short.
    Damaged,
    /// Another program had it open and would not share it.
    InUse,
    /// The system did not allow reading it.
    NoAccess,
    /// Moved or deleted before it was read.
    Gone,
    /// None of the above, or not known.
    Other,
}

impl Unreadable {
    /// The reason behind `error`: the first [`ExtractError`] in its chain, or
    /// what the operating system said about the file.
    pub fn of(error: &(dyn std::error::Error + 'static)) -> Self {
        let mut next = Some(error);
        while let Some(error) = next {
            if let Some(extract) = error.downcast_ref::<ExtractError>() {
                return extract.reason;
            }
            next = match error.downcast_ref::<std::io::Error>() {
                // An I/O error's `source` skips the error it wraps.
                Some(io) => match io.get_ref() {
                    Some(inner) => Some(inner as _),
                    None => return Self::of_io(io),
                },
                None => error.source(),
            };
        }
        Unreadable::Other
    }

    fn of_io(error: &std::io::Error) -> Self {
        if is_sharing_violation(error) {
            return Unreadable::InUse;
        }
        match error.kind() {
            std::io::ErrorKind::NotFound => Unreadable::Gone,
            std::io::ErrorKind::PermissionDenied => Unreadable::NoAccess,
            _ => Unreadable::Other,
        }
    }
}

/// Whether another process holds the file without sharing it.
#[cfg(windows)]
fn is_sharing_violation(error: &std::io::Error) -> bool {
    use windows_sys::Win32::Foundation::{ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION};
    matches!(
        error.raw_os_error(),
        Some(code) if code == ERROR_SHARING_VIOLATION as i32 || code == ERROR_LOCK_VIOLATION as i32
    )
}

#[cfg(not(windows))]
fn is_sharing_violation(_: &std::io::Error) -> bool {
    false
}

/// A file a [`ContentExtractor`] could not read, with the reason and the
/// underlying message.
#[derive(Debug)]
pub struct ExtractError {
    pub reason: Unreadable,
    pub detail: String,
}

impl ExtractError {
    pub fn new(reason: Unreadable, detail: impl std::fmt::Display) -> Self {
        Self {
            reason,
            detail: detail.to_string(),
        }
    }
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for ExtractError {}

/// Reads the text content of files in a supported format, never modifying them.
pub trait ContentExtractor {
    fn extract(&self, path: &std::path::Path) -> Result<Vec<Section>>;
}

/// A dense vector representing the meaning of a Query or a Chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self(values)
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Matryoshka truncation: keeps the leading `dimensions` values, rescaled to unit length.
    pub fn truncated(&self, dimensions: usize) -> Embedding {
        let mut values = self.0[..dimensions.min(self.0.len())].to_vec();
        let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            values.iter_mut().for_each(|v| *v /= norm);
        }
        Embedding(values)
    }

    /// Relevance between two embeddings; higher means more similar.
    pub fn similarity(&self, other: &Embedding) -> f32 {
        self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum()
    }
}

/// Turns Queries and Chunk text into Embeddings in a shared space.
pub trait EmbeddingProvider {
    fn dimensions(&self) -> usize;
    fn embed_query(&self, query: &str) -> Result<Embedding>;
    fn embed_chunks(&self, chunks: &[&str]) -> Result<Vec<Embedding>>;
}

/// Identifier of a Chunk's vector inside a VectorStore.
pub type VectorId = u64;

/// A stored vector close to a Query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neighbor {
    pub id: VectorId,
    pub similarity: f32,
}

/// Nearest-neighbour index over Chunk Embeddings.
pub trait VectorStore {
    fn add(&mut self, id: VectorId, embedding: &Embedding) -> Result<()>;
    fn remove(&mut self, id: VectorId) -> Result<()>;
    /// Returns up to `limit` neighbours, most similar first.
    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<Neighbor>>;
    fn len(&self) -> usize;
}

/// A drive or other file system, identified independently of where it is mounted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Stays the same when the volume is mounted elsewhere, e.g. under another drive letter.
    pub id: String,
    /// Where the volume is mounted now.
    pub root: PathBuf,
}

/// The operating system's volume table (platform adapter).
pub trait Volumes {
    /// The volume holding the existing file or folder `path`.
    fn volume_of(&self, path: &Path) -> Result<Volume>;
    /// Where the volume `id` is mounted now, or `None` while it is not connected.
    fn mount_point(&self, id: &str) -> Option<PathBuf>;
}

/// Whether a file's content is on disk (platform adapter).
pub trait FileAvailability {
    /// Whether `path` is an Online-only File. Must not open the file, which
    /// could download it.
    fn is_online_only(&self, path: &Path) -> bool;
}

/// Where a ChangeSource stopped reading one volume's changes. The index stores
/// it without looking inside (for the USN Journal: journal ID and next USN).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeCursor(pub String);

/// What changed on a volume since a ChangeCursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Changes {
    /// Files or folders, relative to the volume root with forward slashes, that
    /// may have been created, edited, renamed, or deleted since the cursor. A
    /// folder stands for everything beneath it.
    Paths {
        paths: Vec<String>,
        next: ChangeCursor,
    },
    /// The change history since the cursor is gone (journal wrapped or recreated,
    /// no cursor yet, or a volume without a journal): the volume needs a rescan.
    Lost { next: ChangeCursor },
}

/// The operating system's record of file changes (platform adapter).
pub trait ChangeSource {
    /// Changes on volume `volume_id` since `since`; `None` means no cursor yet.
    fn changes_since(&self, volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes>;
}

/// Why Initial Indexing holds back for now (FR-04); Live Updates go on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleReason {
    /// A game, video, or presentation fills the screen.
    FullScreen,
    /// The computer runs on battery power.
    Battery,
    /// Other work keeps the computer busy.
    Busy,
}

/// What the computer is doing that indexing must not disturb (platform adapter).
pub trait Activity {
    /// Why indexing should hold back right now, or `None` when it need not.
    fn throttle_reason(&self) -> Option<ThrottleReason>;
}

/// The operating system convention that decides where the app keeps its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Unix,
}

/// Where the app keeps its index (`data`) and re-creatable files such as
/// models (`cache`).
#[derive(Debug, Clone, PartialEq)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    /// The directories for the running platform and environment.
    pub fn current() -> Self {
        let platform = if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        };
        Self::from_env(platform, |name| std::env::var_os(name))
    }

    /// Resolves the directories from environment variables, looked up by `var`.
    pub fn from_env(platform: Platform, var: impl Fn(&str) -> Option<OsString>) -> Self {
        let dir = |name| PathBuf::from(var(name).unwrap_or_default());
        match platform {
            Platform::Windows => {
                let root = dir("LOCALAPPDATA").join(APP_DIR);
                AppDirs {
                    cache: root.join("cache"),
                    data: root,
                }
            }
            Platform::Unix => {
                let home = dir("HOME");
                AppDirs {
                    data: home.join(".local/share").join(APP_DIR),
                    cache: home.join(".cache").join(APP_DIR),
                }
            }
        }
    }
}

const APP_DIR: &str = "search-by-memory";
