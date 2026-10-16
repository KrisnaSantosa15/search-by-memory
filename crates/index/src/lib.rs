//! SQLite-backed Name Index and file catalog (ADR 0003).

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf, Prefix};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sbm_core::{
    ChangeCursor, ChangeSource, Changes, Embedding, EmbeddingProvider, FileAvailability, Location,
    Result, SystemFileAvailability, SystemVolumes, Unreadable, VectorStore, Volume, Volumes,
};
use walkdir::WalkDir;

/// A file whose filename or folder path is relevant to the Query.
#[derive(Debug, Clone, PartialEq)]
pub struct NameMatch {
    pub path: PathBuf,
    /// The Query is the whole filename, with or without its extension.
    pub exact: bool,
    /// False while the file's drive is a Disconnected Drive.
    pub connected: bool,
}

/// Folder names excluded anywhere on any drive (FR-03).
const DEFAULT_EXCLUDED_NAMES: [&str; 8] = [
    "node_modules",
    ".git",
    "target",
    "venv",
    ".venv",
    "__pycache__",
    ".cache",
    "$RECYCLE.BIN",
];

/// Folders excluded on the system drive only, relative to its root (FR-03);
/// each user's `AppData` is excluded too.
const SYSTEM_DRIVE_EXCLUDED_FOLDERS: [&str; 8] = [
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "System Volume Information",
    "Users/Default",
    "Users/Public",
];

/// A folder or name pattern the user does not want indexed.
#[derive(Debug, Clone, PartialEq)]
pub enum Exclusion {
    /// Any file or folder whose name matches this pattern, anywhere beneath an
    /// Indexed Folder; `*` stands for any run of characters.
    Name(String),
    /// One folder and everything beneath it.
    Folder(PathBuf),
}

impl Exclusion {
    /// The Default Exclusions that apply on every drive.
    pub fn defaults() -> Vec<Exclusion> {
        DEFAULT_EXCLUDED_NAMES
            .iter()
            .map(|name| Exclusion::Name(name.to_string()))
            .collect()
    }

    /// The Default Exclusions that apply only on the system drive whose root is
    /// `drive` (FR-03): Windows, installed programs, and every user's AppData.
    /// Personal folders such as Documents stay in.
    pub fn system_drive_defaults(drive: &Path) -> Vec<Exclusion> {
        let users = drive.join("Users");
        let app_data = std::fs::read_dir(&users)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|user| user.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|user| user.path().join("AppData"));
        SYSTEM_DRIVE_EXCLUDED_FOLDERS
            .iter()
            .map(|folder| drive.join(folder))
            .chain(app_data)
            .map(Exclusion::Folder)
            .collect()
    }

    /// How a turned-off Default Exclusion is remembered.
    fn key(&self) -> String {
        match self {
            Exclusion::Name(pattern) => pattern.clone(),
            Exclusion::Folder(folder) => folder.to_string_lossy().into_owned(),
        }
    }

    fn excludes(&self, path: &Path) -> bool {
        match self {
            Exclusion::Name(pattern) => path
                .file_name()
                .is_some_and(|name| wildcard_match(pattern, &name.to_string_lossy())),
            Exclusion::Folder(folder) => path == folder,
        }
    }
}

/// The root of the drive the operating system runs from: `C:\` on most Windows
/// machines; `None` where there is no such drive.
pub fn system_drive() -> Option<PathBuf> {
    // Windows names it like `C:`.
    std::env::var_os("SystemDrive")
        .map(|drive| PathBuf::from(drive).join(std::path::MAIN_SEPARATOR_STR))
}

/// How much a folder would add to the index: what a scan of it would find.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Estimate {
    pub files: u64,
    pub bytes: u64,
}

/// What a scan of `folder` with `exclusions` would find, without indexing it;
/// unreadable entries are skipped.
pub fn estimate(folder: &Path, exclusions: &[Exclusion]) -> Estimate {
    walk(folder, exclusions)
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .fold(Estimate::default(), |sum, metadata| Estimate {
            files: sum.files + 1,
            bytes: sum.bytes + metadata.len(),
        })
}

/// Everything beneath `folder` but what `exclusions` leave out; `folder` itself
/// is never left out.
fn walk<'a>(
    folder: &Path,
    exclusions: &'a [Exclusion],
) -> walkdir::FilterEntry<walkdir::IntoIter, impl FnMut(&walkdir::DirEntry) -> bool + 'a> {
    WalkDir::new(folder).into_iter().filter_entry(move |entry| {
        entry.depth() == 0 || !exclusions.iter().any(|e| e.excludes(entry.path()))
    })
}

/// A folder or whole drive the user has chosen to make searchable.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedFolder {
    /// Where the folder is now, or was when its drive was last connected.
    pub path: PathBuf,
    /// False while the folder's drive is a Disconnected Drive.
    pub connected: bool,
}

/// How far the Content Index has got (FR-08).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContentProgress {
    /// Files whose content is indexed.
    pub indexed: usize,
    /// Files whose content is still to be indexed.
    pub pending: usize,
    /// Every file modified after this time has its content indexed; `None` when
    /// nothing is pending.
    pub ready_since: Option<SystemTime>,
}

/// What a scan found different from the previous one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanChanges {
    /// Files new to the index.
    pub added: Vec<PathBuf>,
    /// Known files whose size or modification time differ, so their content needs a check.
    pub changed: Vec<PathBuf>,
    /// Files no longer beneath the folder, now removed from the index.
    pub removed: Vec<PathBuf>,
}

/// Schema migrations in order; the database's `user_version` counts those applied.
/// Append new migrations, never edit released ones.
const MIGRATIONS: [&str; 12] = [
    "CREATE TABLE files (
         id INTEGER PRIMARY KEY,
         path TEXT NOT NULL UNIQUE,
         name_lower TEXT NOT NULL,
         stem_lower TEXT NOT NULL,
         size INTEGER NOT NULL,
         mtime_ns INTEGER NOT NULL,
         seen_in_scan INTEGER NOT NULL
     );
     CREATE VIRTUAL TABLE names USING fts5(name, folder);",
    // Content Index (ADR 0004): Chunks belong to file content, not to paths.
    "ALTER TABLE files ADD COLUMN content_hash TEXT;
     CREATE TABLE chunks (
         id INTEGER PRIMARY KEY,
         content_hash TEXT NOT NULL,
         page INTEGER,
         line_start INTEGER,
         line_end INTEGER,
         text TEXT NOT NULL,
         embedding BLOB NOT NULL
     );
     CREATE INDEX chunks_by_content ON chunks (content_hash);
     CREATE INDEX files_by_content ON files (content_hash);",
    // Keyword search over Chunk text; rowid is the Chunk id.
    "CREATE VIRTUAL TABLE chunk_words USING fts5(text);",
    // Files are identified by volume ID plus a forward-slash path relative to the
    // volume (FR-05). The catalog is cleared; a rescan rebuilds it and reuses Chunks
    // by content hash, without re-embedding.
    "DROP TABLE files;
     DELETE FROM names;
     CREATE TABLE files (
         id INTEGER PRIMARY KEY,
         volume_id TEXT NOT NULL,
         relative_path TEXT NOT NULL,
         name_lower TEXT NOT NULL,
         stem_lower TEXT NOT NULL,
         size INTEGER NOT NULL,
         mtime_ns INTEGER NOT NULL,
         seen_in_scan INTEGER NOT NULL,
         content_hash TEXT,
         UNIQUE (volume_id, relative_path)
     );
     CREATE INDEX files_by_content ON files (content_hash);",
    // Where each volume was last mounted, to show files on a Disconnected Drive.
    "CREATE TABLE volumes (id TEXT PRIMARY KEY, last_root TEXT NOT NULL);",
    // Indexed Folders the user chose, and the content work still to do (FR-04).
    // A job is deleted once its file's content is indexed.
    "CREATE TABLE indexed_folders (
         id INTEGER PRIMARY KEY,
         volume_id TEXT NOT NULL,
         relative_path TEXT NOT NULL,
         UNIQUE (volume_id, relative_path)
     );
     CREATE TABLE index_jobs (
         id INTEGER PRIMARY KEY,
         file_id INTEGER NOT NULL UNIQUE,
         priority INTEGER NOT NULL,
         status TEXT NOT NULL DEFAULT 'pending',
         retry_count INTEGER NOT NULL DEFAULT 0,
         error_message TEXT,
         created_at INTEGER NOT NULL
     );",
    // The user's Exclusions: a name pattern, or a folder by volume (FR-03).
    "CREATE TABLE exclusions (
         id INTEGER PRIMARY KEY,
         pattern TEXT,
         volume_id TEXT,
         relative_path TEXT
     );",
    // Where each volume's ChangeSource was last read (FR-04).
    "CREATE TABLE change_cursors (volume_id TEXT PRIMARY KEY, cursor TEXT NOT NULL);",
    // Live Updates go ahead of Initial Indexing work (FR-04).
    "ALTER TABLE index_jobs ADD COLUMN live INTEGER NOT NULL DEFAULT 0;",
    "ALTER TABLE files ADD COLUMN is_online_only INTEGER NOT NULL DEFAULT 0;",
    // Default Exclusions the user turned off; defaults added later still apply (FR-03).
    "CREATE TABLE turned_off_defaults (pattern TEXT PRIMARY KEY);",
    // Why a file failed, in terms the user can act on (FR-08); NULL is not known.
    "ALTER TABLE index_jobs ADD COLUMN failure_reason TEXT;",
];

/// A file whose content could not be indexed (FR-08).
#[derive(Debug, Clone, PartialEq)]
pub struct FailedFile {
    pub path: PathBuf,
    pub reason: Unreadable,
    /// The underlying error message, for details.
    pub detail: String,
}

/// Why a file in the Name Index is not in the Content Index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotRead {
    /// An Online-only File: its content is still in the cloud.
    OnlineOnly,
}

/// How each reason is stored in `index_jobs.failure_reason`.
const REASON_NAMES: [(Unreadable, &str); 7] = [
    (Unreadable::PasswordProtected, "password_protected"),
    (Unreadable::TooLarge, "too_large"),
    (Unreadable::Damaged, "damaged"),
    (Unreadable::InUse, "in_use"),
    (Unreadable::NoAccess, "no_access"),
    (Unreadable::Gone, "gone"),
    (Unreadable::Other, "other"),
];

fn reason_name(reason: Unreadable) -> &'static str {
    REASON_NAMES
        .iter()
        .find(|(known, _)| *known == reason)
        .map_or("other", |(_, name)| name)
}

/// The reason stored as `name`; names this version does not know are `Other`.
fn reason_named(name: &str) -> Unreadable {
    REASON_NAMES
        .iter()
        .find(|(_, known)| *known == name)
        .map_or(Unreadable::Other, |(reason, _)| *reason)
}

/// Chunk size in characters: about 256 tokens of prose.
const CHUNK_CHARS: usize = 1000;
/// Characters each Chunk repeats from the one before (15%).
const CHUNK_OVERLAP_CHARS: usize = 150;
/// Chunks per embedding call: bounds memory for files with many Chunks.
const EMBEDDING_BATCH: usize = 8;
/// Tries to index a file's content before it is reported as failed.
const ATTEMPTS_PER_FILE: i64 = 3;
/// Content Matches considered per Result requested.
const CANDIDATES_PER_RESULT: usize = 10;
/// How far a Result's similarity may fall below the best Result's before it is
/// a Weaker Match. A starting value; to be tuned on the Benchmark.
const WEAKER_MARGIN: f32 = 0.08;

/// How Results are grouped (FR-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// One Result per file.
    File,
    /// One Result per Content Match.
    Passage,
}

/// Why a Result was returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchReason {
    Name,
    Content,
    Both,
}

/// One row of the result list.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub path: PathBuf,
    /// Other paths with byte-for-byte identical content.
    pub copies: Vec<PathBuf>,
    /// False while the drive holding `path` is a Disconnected Drive.
    pub connected: bool,
    /// Verbatim text of the best Content Match, if any.
    pub snippet: Option<String>,
    pub location: Option<Location>,
    pub match_reason: MatchReason,
    /// A Weaker Match: related only by meaning, and much less than the best Results.
    pub weaker: bool,
}

/// The file catalog and Name Index, stored in SQLite.
pub struct Index {
    db: Connection,
    volumes: Box<dyn Volumes + Send>,
    availability: Box<dyn FileAvailability + Send>,
    /// The root of the drive the operating system runs from, if known.
    system_drive: Option<PathBuf>,
}

impl Index {
    /// Opens the index database at `path`, creating it or applying pending migrations.
    pub fn open(path: &Path) -> Result<Self> {
        Self::migrated(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::migrated(Connection::open_in_memory()?)
    }

    /// Every Default Exclusion, on or turned off.
    pub fn default_exclusions(&self) -> Vec<Exclusion> {
        [Exclusion::defaults(), self.system_drive_defaults()].concat()
    }

    /// The system drive's own Default Exclusions, turned off or not.
    fn system_drive_defaults(&self) -> Vec<Exclusion> {
        self.system_drive
            .as_deref()
            .map(Exclusion::system_drive_defaults)
            .unwrap_or_default()
    }

    /// Treats `drive` as the system drive, whose own Default Exclusions apply.
    pub fn with_system_drive(self, drive: &Path) -> Self {
        Self {
            system_drive: Some(drive.to_path_buf()),
            ..self
        }
    }

    /// Uses `volumes` instead of the operating system's to identify the drive of each file.
    pub fn with_volumes(self, volumes: impl Volumes + Send + 'static) -> Self {
        Self {
            volumes: Box::new(volumes),
            ..self
        }
    }

    /// Uses `availability` instead of the operating system's to tell Online-only Files apart.
    pub fn with_file_availability(
        self,
        availability: impl FileAvailability + Send + 'static,
    ) -> Self {
        Self {
            availability: Box::new(availability),
            ..self
        }
    }

    fn migrated(mut db: Connection) -> Result<Self> {
        // Immediate: it reads, then writes, the schema version; a deferred transaction
        // would fail at once instead of waiting while another connection writes.
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version as usize > MIGRATIONS.len() {
            return Err(format!(
                "index database schema {version} is newer than this app supports ({})",
                MIGRATIONS.len()
            )
            .into());
        }
        for migration in MIGRATIONS.iter().skip(version as usize) {
            tx.execute_batch(migration)?;
        }
        tx.pragma_update(None, "user_version", MIGRATIONS.len() as i64)?;
        tx.commit()?;
        Ok(Self {
            db,
            volumes: Box::new(SystemVolumes),
            availability: Box::new(SystemFileAvailability),
            system_drive: system_drive(),
        })
    }

    /// Brings the Name Index in line with the files beneath the Indexed Folder `folder`,
    /// minus Exclusions: new files are added, files no longer there are removed, and
    /// changed files are reported.
    pub fn scan(&mut self, folder: &Path, exclusions: &[Exclusion]) -> Result<ScanChanges> {
        let folder = normalized(folder);
        let volume = self.volumes.volume_of(&folder)?;
        let folder_path = relative_path(&folder, &volume)?;
        let tx = self.db.transaction()?;
        remember_volume(&tx, &volume)?;
        let changes = rescan(
            &tx,
            &volume,
            &folder_path,
            exclusions,
            false,
            next_scan(&tx)?,
            self.availability.as_ref(),
        )?;
        tx.commit()?;
        Ok(changes)
    }

    /// Remembers `folder` as an Indexed Folder, by volume, so it survives drive-letter changes.
    pub fn add_indexed_folder(&mut self, folder: &Path) -> Result<()> {
        let folder = &normalized(folder);
        let volume = self.volumes.volume_of(folder)?;
        let path = relative_path(folder, &volume)?;
        let tx = self.db.transaction()?;
        remember_volume(&tx, &volume)?;
        let added = tx.execute(
            "INSERT OR IGNORE INTO indexed_folders (volume_id, relative_path) VALUES (?1, ?2)",
            params![volume.id, path],
        )?;
        if added > 0 {
            // A new folder needs a first scan: the next apply_changes rescans its volume.
            tx.execute(
                "DELETE FROM change_cursors WHERE volume_id = ?1",
                params![volume.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Forgets the Indexed Folder `folder`, even on a Disconnected Drive: its files
    /// leave the Name Index, and their content, unless a Copy elsewhere uses it, the
    /// Content Index and `store`.
    pub fn remove_indexed_folder(
        &mut self,
        folder: &Path,
        store: &mut impl VectorStore,
    ) -> Result<()> {
        let folder = normalized(folder);
        let folders: Vec<(i64, String, String)> = self
            .db
            .prepare("SELECT id, volume_id, relative_path FROM indexed_folders")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut found = None;
        for (id, volume_id, path) in &folders {
            if self.path_of(volume_id, path)?.0 == folder {
                found = Some((*id, volume_id.clone(), path.clone()));
            }
        }
        let Some((id, volume_id, folder_path)) = found else {
            return Err(format!("{} is not an Indexed Folder", folder.display()).into());
        };
        // Files another Indexed Folder covers are still wanted.
        let others: Vec<&String> = folders
            .iter()
            .filter(|(other, volume, _)| *other != id && *volume == volume_id)
            .map(|(_, _, path)| path)
            .collect();
        let tx = self.db.transaction()?;
        let files: Vec<(i64, String)> = tx
            .prepare("SELECT id, relative_path FROM files WHERE volume_id = ?1")?
            .query_map(params![volume_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (file, path) in files {
            if is_beneath(&path, &folder_path)
                && !others.iter().any(|other| is_beneath(&path, other))
            {
                tx.execute("DELETE FROM names WHERE rowid = ?1", params![file])?;
                tx.execute("DELETE FROM index_jobs WHERE file_id = ?1", params![file])?;
                tx.execute("DELETE FROM files WHERE id = ?1", params![file])?;
            }
        }
        tx.execute("DELETE FROM indexed_folders WHERE id = ?1", params![id])?;
        tx.commit()?;
        self.remove_unreferenced_content(store)
    }

    /// Remembers an Exclusion; the next scan, or apply_changes, drops the files it
    /// matches by rescanning the volumes it applies to.
    pub fn add_exclusion(&mut self, exclusion: Exclusion) -> Result<()> {
        match exclusion {
            Exclusion::Name(pattern) => {
                let tx = self.db.transaction()?;
                tx.execute(
                    "INSERT INTO exclusions (pattern) VALUES (?1)",
                    params![pattern],
                )?;
                tx.execute("DELETE FROM change_cursors", [])?;
                tx.commit()?;
            }
            Exclusion::Folder(folder) => {
                let folder = normalized(&folder);
                let volume = self.volumes.volume_of(&folder)?;
                let path = relative_path(&folder, &volume)?;
                let tx = self.db.transaction()?;
                remember_volume(&tx, &volume)?;
                tx.execute(
                    "INSERT INTO exclusions (volume_id, relative_path) VALUES (?1, ?2)",
                    params![volume.id, path],
                )?;
                tx.execute(
                    "DELETE FROM change_cursors WHERE volume_id = ?1",
                    params![volume.id],
                )?;
                tx.commit()?;
            }
        }
        Ok(())
    }

    /// Forgets an Exclusion; the next scan, or apply_changes, brings back the files it
    /// kept out by rescanning the volumes it applied to.
    pub fn remove_exclusion(&mut self, exclusion: &Exclusion) -> Result<()> {
        match exclusion {
            Exclusion::Name(pattern) => {
                let tx = self.db.transaction()?;
                tx.execute(
                    "DELETE FROM exclusions WHERE pattern = ?1",
                    params![pattern],
                )?;
                if DEFAULT_EXCLUDED_NAMES.contains(&pattern.as_str()) {
                    tx.execute(
                        "INSERT OR IGNORE INTO turned_off_defaults (pattern) VALUES (?1)",
                        params![pattern],
                    )?;
                }
                tx.execute("DELETE FROM change_cursors", [])?;
                tx.commit()?;
            }
            Exclusion::Folder(folder) => {
                let default = self
                    .system_drive_defaults()
                    .into_iter()
                    .find(|default| default == exclusion);
                let folder = normalized(folder);
                let volume = self.volumes.volume_of(&folder)?;
                let path = relative_path(&folder, &volume)?;
                let tx = self.db.transaction()?;
                tx.execute(
                    "DELETE FROM exclusions WHERE volume_id = ?1 AND relative_path = ?2",
                    params![volume.id, path],
                )?;
                if let Some(default) = default {
                    tx.execute(
                        "INSERT OR IGNORE INTO turned_off_defaults (pattern) VALUES (?1)",
                        params![default.key()],
                    )?;
                }
                tx.execute(
                    "DELETE FROM change_cursors WHERE volume_id = ?1",
                    params![volume.id],
                )?;
                tx.commit()?;
            }
        }
        Ok(())
    }

    /// Turns every Default Exclusion back on; the next scan, or apply_changes, drops
    /// the files they match.
    pub fn restore_default_exclusions(&mut self) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM turned_off_defaults", [])?;
        tx.execute("DELETE FROM change_cursors", [])?;
        tx.commit()?;
        Ok(())
    }

    /// The Default Exclusions the user has not turned off, followed by the user's.
    pub fn exclusions(&self) -> Result<Vec<Exclusion>> {
        let saved: Vec<(Option<String>, Option<String>, Option<String>)> = self
            .db
            .prepare("SELECT pattern, volume_id, relative_path FROM exclusions ORDER BY id")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let turned_off: Vec<String> = self
            .db
            .prepare("SELECT pattern FROM turned_off_defaults")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut exclusions: Vec<Exclusion> = Exclusion::defaults()
            .into_iter()
            .filter(|default| !matches!(default, Exclusion::Name(pattern) if turned_off.contains(pattern)))
            .collect();
        exclusions.extend(
            self.system_drive_defaults()
                .into_iter()
                .filter(|default| !turned_off.contains(&default.key())),
        );
        for row in saved {
            exclusions.push(match row {
                (Some(pattern), _, _) => Exclusion::Name(pattern),
                (None, Some(volume_id), Some(path)) => {
                    Exclusion::Folder(self.path_of(&volume_id, &path)?.0)
                }
                _ => return Err("malformed Exclusion in the index database".into()),
            });
        }
        Ok(exclusions)
    }

    /// The Indexed Folders, in the order they were added.
    pub fn indexed_folders(&self) -> Result<Vec<IndexedFolder>> {
        let folders: Vec<(String, String)> = self
            .db
            .prepare("SELECT volume_id, relative_path FROM indexed_folders ORDER BY id")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        folders
            .into_iter()
            .map(|(volume_id, path)| {
                let (path, connected) = self.path_of(&volume_id, &path)?;
                Ok(IndexedFolder { path, connected })
            })
            .collect()
    }

    /// Scans every connected Indexed Folder, queueing a content job for each added
    /// or changed file. Folders on a Disconnected Drive keep their files.
    pub fn scan_all(&mut self) -> Result<ScanChanges> {
        let exclusions = self.exclusions()?;
        let mut changes = ScanChanges::default();
        for folder in self.indexed_folders()? {
            if !folder.connected {
                continue;
            }
            let found = self.scan(&folder.path, &exclusions)?;
            changes.added.extend(found.added);
            changes.changed.extend(found.changed);
            changes.removed.extend(found.removed);
        }
        Ok(changes)
    }

    /// Applies the changes `source` reports for each connected Indexed Folder's
    /// volume: changed files are queued as Live Updates, deleted ones leave both
    /// indexes, and a lost change history rescans that volume's Indexed Folders.
    pub fn apply_changes(&mut self, source: &dyn ChangeSource) -> Result<ScanChanges> {
        let exclusions = self.exclusions()?;
        let volume_ids: Vec<String> = self
            .db
            .prepare("SELECT DISTINCT volume_id FROM indexed_folders ORDER BY volume_id")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut changes = ScanChanges::default();
        for volume_id in volume_ids {
            // Folders on a Disconnected Drive keep their files and cursor.
            let Some(root) = self.volumes.mount_point(&volume_id) else {
                continue;
            };
            let volume = Volume {
                id: volume_id,
                root,
            };
            let cursor: Option<String> = self
                .db
                .query_row(
                    "SELECT cursor FROM change_cursors WHERE volume_id = ?1",
                    params![volume.id],
                    |row| row.get(0),
                )
                .optional()?;
            let (paths, next, live) = match source
                .changes_since(&volume.id, cursor.map(ChangeCursor).as_ref())?
            {
                Changes::Paths { paths, next } => {
                    (self.indexed_paths(&volume, paths, &exclusions)?, next, true)
                }
                Changes::Lost { next } => (
                    self.db
                        .prepare("SELECT relative_path FROM indexed_folders WHERE volume_id = ?1")?
                        .query_map(params![volume.id], |row| row.get(0))?
                        .collect::<rusqlite::Result<_>>()?,
                    next,
                    false,
                ),
            };
            // The changes and the cursor after them are saved together.
            let tx = self.db.transaction()?;
            remember_volume(&tx, &volume)?;
            let scan = next_scan(&tx)?;
            for path in paths {
                let found = rescan(
                    &tx,
                    &volume,
                    &path,
                    &exclusions,
                    live,
                    scan,
                    self.availability.as_ref(),
                )?;
                changes.added.extend(found.added);
                changes.changed.extend(found.changed);
                changes.removed.extend(found.removed);
            }
            tx.execute(
                "INSERT INTO change_cursors (volume_id, cursor) VALUES (?1, ?2)
                 ON CONFLICT (volume_id) DO UPDATE SET cursor = excluded.cursor",
                params![volume.id, next.0],
            )?;
            tx.commit()?;
        }
        Ok(changes)
    }

    /// Of the changed relative `paths` on `volume`, those in an Indexed Folder and
    /// not excluded; an Indexed Folder inside a changed folder stands for it.
    fn indexed_paths(
        &self,
        volume: &Volume,
        paths: Vec<String>,
        exclusions: &[Exclusion],
    ) -> Result<Vec<String>> {
        let folders: Vec<String> = self
            .db
            .prepare("SELECT relative_path FROM indexed_folders WHERE volume_id = ?1")?
            .query_map(params![volume.id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut indexed = Vec::new();
        for path in paths {
            for folder in &folders {
                if is_beneath(&path, folder) {
                    // The Indexed Folder itself is never excluded.
                    let folder = beneath(&volume.root, folder);
                    let excluded = beneath(&volume.root, &path)
                        .ancestors()
                        .take_while(|ancestor| *ancestor != folder)
                        .any(|ancestor| exclusions.iter().any(|e| e.excludes(ancestor)));
                    if !excluded {
                        indexed.push(path.clone());
                    }
                } else if is_beneath(folder, &path) {
                    indexed.push(folder.clone());
                }
            }
        }
        indexed.sort();
        indexed.dedup();
        Ok(indexed)
    }

    /// Indexes the content of up to `limit` queued files. Each file is committed on
    /// its own, so interrupted indexing resumes with the next file. Once the queue
    /// is empty, content no file uses any longer is removed.
    pub fn run_jobs(
        &mut self,
        embedder: &dyn EmbeddingProvider,
        store: &mut impl VectorStore,
        limit: usize,
    ) -> Result<usize> {
        self.run_queued(embedder, store, limit, false)
    }

    /// Like `run_jobs`, but only Live Updates: Initial Indexing work waits, and
    /// so does removing unused content (moves may still reuse it).
    pub fn run_live_jobs(
        &mut self,
        embedder: &dyn EmbeddingProvider,
        store: &mut impl VectorStore,
        limit: usize,
    ) -> Result<usize> {
        self.run_queued(embedder, store, limit, true)
    }

    /// The file `run_jobs` (or, with `only_live`, `run_live_jobs`) reads next.
    pub fn next_queued_file(&self, only_live: bool) -> Result<Option<PathBuf>> {
        Ok(self.next_job(only_live)?.map(|(_job, path)| path))
    }

    /// The pending job to run next and its file: Live Updates first, then the
    /// newest files; files on a Disconnected Drive wait until it returns.
    fn next_job(&self, only_live: bool) -> Result<Option<(i64, PathBuf)>> {
        let disconnected: Vec<String> = self
            .db
            .prepare("SELECT id FROM volumes")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?
            .into_iter()
            .filter(|id| self.volumes.mount_point(id).is_none())
            .collect();
        let next_job = format!(
            "SELECT index_jobs.id, files.volume_id, files.relative_path
             FROM index_jobs JOIN files ON files.id = index_jobs.file_id
             WHERE index_jobs.status = 'pending' AND files.volume_id NOT IN ({}) {}
             ORDER BY index_jobs.live DESC, index_jobs.priority DESC LIMIT 1",
            vec!["?"; disconnected.len()].join(", "),
            if only_live { "AND index_jobs.live" } else { "" },
        );
        let next: Option<(i64, String, String)> = self
            .db
            .query_row(
                &next_job,
                rusqlite::params_from_iter(&disconnected),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        next.map(|(job, volume_id, path)| Ok((job, self.path_of(&volume_id, &path)?.0)))
            .transpose()
    }

    fn run_queued(
        &mut self,
        embedder: &dyn EmbeddingProvider,
        store: &mut impl VectorStore,
        limit: usize,
        only_live: bool,
    ) -> Result<usize> {
        let mut done = 0;
        while done < limit {
            let Some((job, path)) = self.next_job(only_live)? else {
                if !only_live {
                    self.remove_unreferenced_content(store)?;
                }
                break;
            };
            if let Err(error) = self.index_content(&path, embedder, store) {
                // A bad file must not stop indexing (FR-03); it stays in the Name Index.
                self.db.execute(
                    "UPDATE index_jobs SET
                         retry_count = retry_count + 1,
                         error_message = ?2,
                         failure_reason = ?4,
                         status = CASE WHEN retry_count + 1 >= ?3 THEN 'failed' ELSE status END
                     WHERE id = ?1",
                    params![
                        job,
                        error.to_string(),
                        ATTEMPTS_PER_FILE,
                        reason_name(Unreadable::of(error.as_ref())),
                    ],
                )?;
            }
            done += 1;
        }
        Ok(done)
    }

    /// Puts the files whose content could not be indexed back in the queue, with
    /// their attempts counted from zero.
    pub fn retry_failed(&mut self) -> Result<()> {
        self.db.execute(
            "UPDATE index_jobs SET status = 'pending', retry_count = 0,
                 error_message = NULL, failure_reason = NULL
             WHERE status = 'failed'",
            [],
        )?;
        Ok(())
    }

    /// Files whose content could not be indexed, with why and the last error.
    pub fn failed_files(&self) -> Result<Vec<FailedFile>> {
        let failed: Vec<(String, String, String, Option<String>)> = self
            .db
            .prepare(
                "SELECT files.volume_id, files.relative_path, index_jobs.error_message,
                        index_jobs.failure_reason
                 FROM index_jobs JOIN files ON files.id = index_jobs.file_id
                 WHERE index_jobs.status = 'failed'
                 ORDER BY files.relative_path",
            )?
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        failed
            .into_iter()
            .map(|(volume_id, path, detail, reason)| {
                Ok(FailedFile {
                    path: self.path_of(&volume_id, &path)?.0,
                    reason: reason.as_deref().map_or(Unreadable::Other, reason_named),
                    detail,
                })
            })
            .collect()
    }

    /// Why the content of the file at `path` is not searched; `None` when it is,
    /// or when the file is not in the Name Index.
    pub fn not_read(&self, path: &Path) -> Result<Option<NotRead>> {
        let path = &normalized(path);
        let volume = self.volumes.volume_of(path)?;
        let relative = relative_path(path, &volume)?;
        let online_only: Option<bool> = self
            .db
            .query_row(
                "SELECT is_online_only FROM files WHERE volume_id = ?1 AND relative_path = ?2",
                params![volume.id, relative],
                |row| row.get(0),
            )
            .optional()?;
        Ok((online_only == Some(true)).then_some(NotRead::OnlineOnly))
    }

    /// Files in the Name Index.
    pub fn file_count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT count(*) FROM files", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Files whose content is still to be indexed.
    pub fn pending_jobs(&self) -> Result<usize> {
        let count: i64 = self.db.query_row(
            "SELECT count(*) FROM index_jobs WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// How far the Content Index has got: jobs run newest file first, so every file
    /// newer than the newest pending one is ready.
    pub fn content_progress(&self) -> Result<ContentProgress> {
        let (pending, newest_pending): (i64, Option<i64>) = self.db.query_row(
            "SELECT count(*), max(priority) FROM index_jobs WHERE status = 'pending'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let indexed: i64 = self.db.query_row(
            "SELECT count(*) FROM files WHERE content_hash IS NOT NULL
             AND id NOT IN (SELECT file_id FROM index_jobs WHERE status = 'pending')",
            [],
            |row| row.get(0),
        )?;
        Ok(ContentProgress {
            indexed: indexed as usize,
            pending: pending as usize,
            ready_since: newest_pending.map(|mtime_ns| {
                let since_epoch = Duration::from_nanos(mtime_ns.unsigned_abs());
                if mtime_ns < 0 {
                    UNIX_EPOCH - since_epoch
                } else {
                    UNIX_EPOCH + since_epoch
                }
            }),
        })
    }

    /// Adds the content of a file in the Name Index to the Content Index:
    /// its Chunks, their Embeddings in SQLite, and their vectors in `store`.
    pub fn index_content(
        &mut self,
        path: &Path,
        embedder: &dyn EmbeddingProvider,
        store: &mut impl VectorStore,
    ) -> Result<()> {
        let path = &normalized(path);
        let volume = self.volumes.volume_of(path)?;
        let relative = relative_path(path, &volume)?;
        let Some(extractor) = sbm_extract::extractor_for(path) else {
            finish_job(&self.db, &volume.id, &relative)?;
            return Ok(());
        };
        // Streamed, so hashing never holds a large file in memory.
        let hash = blake3::Hasher::new()
            .update_reader(std::fs::File::open(path)?)?
            .finalize()
            .to_hex()
            .to_string();
        let known: bool = self.db.query_row(
            "SELECT EXISTS (SELECT 1 FROM chunks WHERE content_hash = ?1)",
            params![hash],
            |row| row.get(0),
        )?;
        if known {
            // A Copy, or a moved file: its content is already indexed (ADR 0004).
            self.db.execute(
                "UPDATE files SET content_hash = ?1 WHERE volume_id = ?2 AND relative_path = ?3",
                params![hash, volume.id, relative],
            )?;
            finish_job(&self.db, &volume.id, &relative)?;
            return Ok(());
        }
        let chunks = sbm_extract::chunk_sections(
            &extractor.extract(path)?,
            CHUNK_CHARS,
            CHUNK_OVERLAP_CHARS,
        );
        let texts: Vec<&str> = chunks.iter().map(|chunk| chunk.text.as_str()).collect();
        let mut embeddings = Vec::with_capacity(texts.len());
        for batch in texts.chunks(EMBEDDING_BATCH) {
            embeddings.extend(embedder.embed_chunks(batch)?);
        }
        let tx = self.db.transaction()?;
        for (chunk, embedding) in chunks.iter().zip(&embeddings) {
            let (page, line_start, line_end) = location_columns(&chunk.location);
            tx.execute(
                "INSERT INTO chunks (content_hash, page, line_start, line_end, text, embedding)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    hash,
                    page,
                    line_start,
                    line_end,
                    chunk.text,
                    embedding_bytes(embedding)
                ],
            )?;
            let id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO chunk_words (rowid, text) VALUES (?1, ?2)",
                params![id, chunk.text],
            )?;
            store.add(id as u64, embedding)?;
        }
        tx.execute(
            "UPDATE files SET content_hash = ?1 WHERE volume_id = ?2 AND relative_path = ?3",
            params![hash, volume.id, relative],
        )?;
        finish_job(&tx, &volume.id, &relative)?;
        tx.commit()?;
        Ok(())
    }

    /// Fills an empty `store` with every Chunk's stored Embedding, so a lost or
    /// corrupt vector index is rebuilt without re-reading or re-embedding files.
    pub fn rebuild_vectors(&self, store: &mut impl VectorStore) -> Result<()> {
        let mut statement = self.db.prepare("SELECT id, embedding FROM chunks")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let bytes: Vec<u8> = row.get(1)?;
            store.add(id as u64, &embedding_from_bytes(&bytes))?;
        }
        Ok(())
    }

    /// Removes Chunks, and their vectors in `store`, whose content no file
    /// references any longer. Run it after indexing a scan's added and changed
    /// files, so that moved files reuse their content first (ADR 0004).
    pub fn remove_unreferenced_content(&mut self, store: &mut impl VectorStore) -> Result<()> {
        let tx = self.db.transaction()?;
        let unreferenced: Vec<i64> = tx
            .prepare(
                "SELECT id FROM chunks WHERE content_hash NOT IN
                 (SELECT content_hash FROM files WHERE content_hash IS NOT NULL)",
            )?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for id in unreferenced {
            store.remove(id as u64)?;
            tx.execute("DELETE FROM chunk_words WHERE rowid = ?1", params![id])?;
            tx.execute("DELETE FROM chunks WHERE id = ?1", params![id])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Results for a Query, best first: in File view one per file from the Name
    /// Index and Content Index, in Passage view one per Content Match.
    pub fn search(
        &self,
        query: &str,
        embedder: &dyn EmbeddingProvider,
        store: &impl VectorStore,
        view: View,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search_where(query, embedder, store, view, limit, &|_| true)
    }

    /// Like `search`, over only the files `keep` accepts (e.g. code): up to
    /// `limit` of them, with Weaker Matches judged among them alone.
    pub fn search_where(
        &self,
        query: &str,
        embedder: &dyn EmbeddingProvider,
        store: &impl VectorStore,
        view: View,
        limit: usize,
        keep: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<SearchResult>> {
        let candidates = limit * CANDIDATES_PER_RESULT;
        let neighbors = store.search(&embedder.embed_query(query)?, candidates)?;
        let semantic: Vec<i64> = neighbors
            .iter()
            .map(|neighbor| neighbor.id as i64)
            .collect();
        let similarity_of = |chunk_id: i64| {
            neighbors
                .iter()
                .find(|neighbor| neighbor.id as i64 == chunk_id)
                .map(|neighbor| neighbor.similarity)
        };
        let keyword: Vec<i64> = match fts_query(query) {
            Some(words) => self
                .db
                .prepare("SELECT rowid FROM chunk_words WHERE chunk_words MATCH ?1 ORDER BY rank LIMIT ?2")?
                .query_map(params![words, candidates as i64], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?,
            None => Vec::new(),
        };
        let mut chunk = self.db.prepare(
            "SELECT content_hash, page, line_start, line_end, text FROM chunks WHERE id = ?1",
        )?;
        let mut files = self
            .db
            .prepare("SELECT volume_id, relative_path FROM files WHERE content_hash = ?1")?;
        let mut results: Vec<SearchResult> = Vec::new();
        let mut seen_contents: Vec<String> = Vec::new();
        // Each Result's best semantic similarity, by its path.
        let mut similarities: HashMap<PathBuf, f32> = HashMap::new();
        // Results with the Query's words in their content.
        let mut by_keyword: HashSet<PathBuf> = HashSet::new();
        for chunk_id in reciprocal_rank_fusion(&[&semantic, &keyword]) {
            // A vector saved before a crash can outlive its Chunk.
            let Some((hash, page, line_start, line_end, text)) = chunk
                .query_row(params![chunk_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<u32>>(1)?,
                        row.get::<_, Option<u32>>(2)?,
                        row.get::<_, Option<u32>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .optional()?
            else {
                continue;
            };
            if view == View::File {
                if let Some(index) = seen_contents.iter().position(|seen| seen == &hash) {
                    let path = results[index].path.clone();
                    if keyword.contains(&chunk_id) {
                        by_keyword.insert(path.clone());
                    }
                    if let Some(similarity) = similarity_of(chunk_id) {
                        let best = similarities.entry(path).or_insert(similarity);
                        *best = best.max(similarity);
                    }
                    continue;
                }
            }
            let mut paths: Vec<(PathBuf, bool)> = files
                .query_map(params![hash], |row| {
                    self.path_of(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?)
                })?
                .collect::<rusqlite::Result<_>>()?;
            paths.retain(|(path, _)| keep(path));
            paths.sort();
            if paths.is_empty() {
                continue;
            }
            let (path, connected) = paths.remove(0);
            seen_contents.push(hash);
            if let Some(similarity) = similarity_of(chunk_id) {
                similarities.insert(path.clone(), similarity);
            }
            if keyword.contains(&chunk_id) {
                by_keyword.insert(path.clone());
            }
            results.push(SearchResult {
                path,
                copies: paths.into_iter().map(|(path, _)| path).collect(),
                connected,
                snippet: Some(text),
                location: Some(location_from_columns(page, line_start, line_end)),
                match_reason: MatchReason::Content,
                weaker: false,
            });
        }
        if view == View::File {
            let mut names = self.search_names(query, candidates)?;
            names.retain(|name| keep(&name.path));
            results = fuse_name_matches(results, &names);
        }
        mark_weaker_matches(&mut results, &similarities, &by_keyword);
        results.truncate(limit);
        Ok(results)
    }

    /// Where the file at the relative `path` on volume `volume_id` is now.
    /// For a Disconnected Drive, where the file was when the drive was last scanned;
    /// the flag says whether the drive is connected.
    fn path_of(&self, volume_id: &str, path: &str) -> rusqlite::Result<(PathBuf, bool)> {
        if let Some(root) = self.volumes.mount_point(volume_id) {
            return Ok((beneath(&root, path), true));
        }
        let last_root: String = self.db.query_row(
            "SELECT last_root FROM volumes WHERE id = ?1",
            params![volume_id],
            |row| row.get(0),
        )?;
        Ok((beneath(Path::new(&last_root), path), false))
    }

    /// Name Matches for a Query, best first.
    pub fn search_names(&self, query: &str, limit: usize) -> Result<Vec<NameMatch>> {
        let exact = query.trim().to_lowercase();
        let Some(query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let mut statement = self.db.prepare(
            "SELECT files.volume_id, files.relative_path, (files.name_lower = ?3 OR files.stem_lower = ?3)
             FROM names JOIN files ON files.id = names.rowid
             WHERE names MATCH ?1
             ORDER BY (files.name_lower = ?3 OR files.stem_lower = ?3) DESC, rank
             LIMIT ?2",
        )?;
        let matches = statement
            .query_map(params![query, limit as i64, exact], |row| {
                let (path, connected) =
                    self.path_of(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?)?;
                Ok(NameMatch {
                    path,
                    exact: row.get(2)?,
                    connected,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(matches)
    }

    /// Up to `limit` files that `keep` accepts, newest modified first.
    pub fn newest_files(
        &self,
        limit: usize,
        keep: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<NameMatch>> {
        let mut statement = self
            .db
            .prepare("SELECT volume_id, relative_path FROM files ORDER BY mtime_ns DESC")?;
        let mut rows = statement.query([])?;
        let mut found = Vec::new();
        while found.len() < limit
            && let Some(row) = rows.next()?
        {
            let (path, connected) =
                self.path_of(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?)?;
            if keep(&path) {
                found.push(NameMatch {
                    path,
                    exact: false,
                    connected,
                });
            }
        }
        Ok(found)
    }
}

/// Records where `volume` is mounted, to show its files while it is disconnected.
/// The number that marks files seen by a new scan.
fn next_scan(db: &Connection) -> rusqlite::Result<i64> {
    db.query_row(
        "SELECT coalesce(max(seen_in_scan), 0) + 1 FROM files",
        [],
        |row| row.get(0),
    )
}

/// Brings the Name Index in line with the file or folder at the relative
/// `folder_path` on `volume`, which may no longer exist, marking what it finds
/// with `scan`. With `live`, content work is queued as Live Updates. Content
/// work is never queued for Online-only Files.
fn rescan(
    tx: &Connection,
    volume: &Volume,
    folder_path: &str,
    exclusions: &[Exclusion],
    live: bool,
    scan: i64,
    availability: &dyn FileAvailability,
) -> Result<ScanChanges> {
    let folder = &beneath(&volume.root, folder_path);
    let mut changes = ScanChanges::default();
    let entries = walk(folder, exclusions)
        // A deleted file or folder: whatever the index knows beneath it is swept below.
        .filter(|entry| folder.exists() || entry.is_ok());
    for entry in entries {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = relative_path(entry.path(), volume)?;
        let metadata = entry.metadata()?;
        let size = metadata.len() as i64;
        let mtime_ns = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as i64);
        let online_only = availability.is_online_only(entry.path());
        let known: Option<(i64, i64, i64, bool)> = tx
            .query_row(
                "SELECT id, size, mtime_ns, is_online_only FROM files
                 WHERE volume_id = ?1 AND relative_path = ?2",
                params![volume.id, path],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        if let Some((id, previous_size, previous_mtime, was_online_only)) = known {
            // A download leaves size and mtime alone, but the content is new to the index.
            let downloaded = was_online_only && !online_only;
            let changed = (previous_size, previous_mtime) != (size, mtime_ns) || downloaded;
            if changed {
                changes.changed.push(entry.path().to_path_buf());
            }
            if online_only {
                // Opening it for queued content work would download it.
                tx.execute("DELETE FROM index_jobs WHERE file_id = ?1", params![id])?;
            } else if changed {
                queue_job(tx, id, mtime_ns, live)?;
            }
            tx.execute(
                "UPDATE files SET seen_in_scan = ?1, size = ?2, mtime_ns = ?3, is_online_only = ?4
                 WHERE volume_id = ?5 AND relative_path = ?6",
                params![scan, size, mtime_ns, online_only, volume.id, path],
            )?;
            continue;
        }
        let name_lower = entry.file_name().to_string_lossy().to_lowercase();
        let stem_lower = entry
            .path()
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        tx.execute(
            "INSERT INTO files (volume_id, relative_path, name_lower, stem_lower, size, mtime_ns,
                                seen_in_scan, is_online_only)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                volume.id,
                path,
                name_lower,
                stem_lower,
                size,
                mtime_ns,
                scan,
                online_only
            ],
        )?;
        let id = tx.last_insert_rowid();
        changes.added.push(entry.path().to_path_buf());
        let name = entry.file_name().to_string_lossy();
        let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
        tx.execute(
            "INSERT INTO names (rowid, name, folder) VALUES (?1, ?2, ?3)",
            params![id, name, folder],
        )?;
        if !online_only {
            queue_job(tx, id, mtime_ns, live)?;
        }
    }
    // Only rows that may lie beneath `folder_path`: `folder_path` and
    // `folder_path/...` sort before `folder_path0`, as '/' comes right before '0'.
    let (from, to) = if folder_path.is_empty() {
        (String::new(), String::from(char::MAX))
    } else {
        (folder_path.to_string(), format!("{folder_path}0"))
    };
    let gone: Vec<(i64, String)> = tx
        .prepare(
            "SELECT id, relative_path FROM files
             WHERE volume_id = ?1 AND relative_path >= ?2 AND relative_path < ?3
                 AND seen_in_scan != ?4",
        )?
        .query_map(params![volume.id, from, to, scan], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    for (id, path) in gone {
        if is_beneath(&path, folder_path) {
            tx.execute("DELETE FROM names WHERE rowid = ?1", params![id])?;
            tx.execute("DELETE FROM index_jobs WHERE file_id = ?1", params![id])?;
            tx.execute("DELETE FROM files WHERE id = ?1", params![id])?;
            changes.removed.push(beneath(&volume.root, &path));
        }
    }
    Ok(changes)
}

fn remember_volume(db: &Connection, volume: &Volume) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO volumes (id, last_root) VALUES (?1, ?2)
         ON CONFLICT (id) DO UPDATE SET last_root = excluded.last_root",
        params![volume.id, volume.root.to_string_lossy()],
    )?;
    Ok(())
}

/// Queues (or re-queues) indexing the content of file `file_id`, modified at
/// `mtime_ns`; a Live Update stays one until its job is done.
fn queue_job(db: &Connection, file_id: i64, mtime_ns: i64, live: bool) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    db.execute(
        "INSERT INTO index_jobs (file_id, priority, created_at, live) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (file_id) DO UPDATE SET
             priority = excluded.priority, status = 'pending', retry_count = 0, error_message = NULL, failure_reason = NULL,
             live = max(live, excluded.live)",
        params![file_id, mtime_ns, now, live],
    )?;
    Ok(())
}

/// Removes the job of the file at `path` on volume `volume_id`: its content is indexed.
fn finish_job(db: &Connection, volume_id: &str, path: &str) -> rusqlite::Result<()> {
    db.execute(
        "DELETE FROM index_jobs WHERE file_id =
         (SELECT id FROM files WHERE volume_id = ?1 AND relative_path = ?2)",
        params![volume_id, path],
    )?;
    Ok(())
}

/// `path` relative to the root of `volume`, with forward slashes (FR-05).
fn relative_path(path: &Path, volume: &Volume) -> Result<String> {
    let relative = path.strip_prefix(&volume.root).map_err(|_| {
        format!(
            "{} is not on the volume mounted at {}",
            path.display(),
            volume.root.display()
        )
    })?;
    let parts: Vec<_> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect();
    Ok(parts.join("/"))
}

/// The relative `path` under `root`, spelled with the platform's separator.
fn beneath(root: &Path, path: &str) -> PathBuf {
    let mut absolute = root.to_path_buf();
    absolute.extend(path.split('/').filter(|part| !part.is_empty()));
    absolute
}

/// Whether the relative path `path` is `folder` or beneath it.
fn is_beneath(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// One spelling per path, so the catalog has one row per file: on Windows,
/// `C:/a/b.txt` and `\\?\C:\a\b.txt` both become `C:\a\b.txt`.
fn normalized(path: &Path) -> PathBuf {
    path.components()
        .map(|component| match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", drive as char)),
                _ => PathBuf::from(component.as_os_str()),
            },
            _ => PathBuf::from(component.as_os_str()),
        })
        .collect()
}

/// The Query as FTS5 syntax: each word quoted, so user text is never parsed as
/// operators, and the last word matched as a prefix because the user may still be typing it.
fn fts_query(query: &str) -> Option<String> {
    let words: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let mut terms: Vec<String> = words.iter().map(|word| format!("\"{word}\"")).collect();
    terms.last_mut()?.push('*');
    Some(terms.join(" "))
}

/// Whether `name` matches `pattern`, where `*` stands for any run of characters.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

/// A Location as the nullable `page`, `line_start`, `line_end` columns of `chunks`.
fn location_columns(location: &Location) -> (Option<u32>, Option<u32>, Option<u32>) {
    match location {
        Location::Page(page) => (Some(*page), None, None),
        Location::Lines { start, end } => (None, Some(*start), Some(*end)),
        Location::Time { .. } | Location::WholeFile => (None, None, None),
    }
}

fn location_from_columns(
    page: Option<u32>,
    line_start: Option<u32>,
    line_end: Option<u32>,
) -> Location {
    match (page, line_start, line_end) {
        (Some(page), _, _) => Location::Page(page),
        (None, Some(start), Some(end)) => Location::Lines { start, end },
        _ => Location::WholeFile,
    }
}

/// Little-endian f32 values, as stored in `chunks.embedding`.
fn embedding_bytes(embedding: &Embedding) -> Vec<u8> {
    embedding
        .as_slice()
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// Rank constant of Reciprocal Rank Fusion: damps the weight of top ranks.
const FUSION_K: f64 = 60.0;

/// Merges ranked lists of ids: an id ranked well in several lists rises above
/// ids ranked well in only one.
fn reciprocal_rank_fusion(lists: &[&[i64]]) -> Vec<i64> {
    let mut scores: Vec<(i64, f64)> = Vec::new();
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            let score = fusion_score(rank);
            match scores.iter_mut().find(|(seen, _)| seen == id) {
                Some((_, total)) => *total += score,
                None => scores.push((*id, score)),
            }
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    scores.into_iter().map(|(id, _)| id).collect()
}

/// Flags the Results related only by meaning (no Name Match, none of the
/// Query's words) whose similarity falls more than `WEAKER_MARGIN` below the
/// best Result's.
fn mark_weaker_matches(
    results: &mut [SearchResult],
    similarities: &HashMap<PathBuf, f32>,
    by_keyword: &HashSet<PathBuf>,
) {
    let Some(best) = similarities.values().copied().reduce(f32::max) else {
        return;
    };
    for result in results {
        let similarity = similarities.get(&result.path).copied().unwrap_or(f32::MIN);
        result.weaker = result.match_reason == MatchReason::Content
            && !by_keyword.contains(&result.path)
            && similarity < best - WEAKER_MARGIN;
    }
}

/// Merges File-view content Results with Name Matches by Reciprocal Rank
/// Fusion; a file found both ways is one Result with Match Reason `Both`.
fn fuse_name_matches(content: Vec<SearchResult>, names: &[NameMatch]) -> Vec<SearchResult> {
    let mut scored: Vec<(SearchResult, f64)> = content
        .into_iter()
        .enumerate()
        .map(|(rank, result)| (result, fusion_score(rank)))
        .collect();
    for (rank, name) in names.iter().enumerate() {
        let found = scored
            .iter_mut()
            .find(|(result, _)| result.path == name.path || result.copies.contains(&name.path));
        match found {
            Some((result, score)) => {
                result.match_reason = MatchReason::Both;
                *score += fusion_score(rank);
            }
            None => scored.push((
                SearchResult {
                    path: name.path.clone(),
                    copies: Vec::new(),
                    connected: name.connected,
                    snippet: None,
                    location: None,
                    match_reason: MatchReason::Name,
                    weaker: false,
                },
                fusion_score(rank),
            )),
        }
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    // An exact filename match is always placed at the top (FR-02).
    let exact: Vec<&PathBuf> = names
        .iter()
        .filter(|name| name.exact)
        .map(|name| &name.path)
        .collect();
    scored.sort_by_key(|(result, _)| !exact.contains(&&result.path));
    scored.into_iter().map(|(result, _)| result).collect()
}

fn fusion_score(rank: usize) -> f64 {
    1.0 / (FUSION_K + rank as f64 + 1.0)
}

fn embedding_from_bytes(bytes: &[u8]) -> Embedding {
    Embedding::new(
        bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
    )
}
