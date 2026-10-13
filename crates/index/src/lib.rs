//! SQLite-backed Name Index and file catalog (ADR 0003).

use std::path::{Component, Path, PathBuf, Prefix};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OptionalExtension, params};
use sbm_core::{
    Embedding, EmbeddingProvider, Location, Result, SystemVolumes, VectorStore, Volume, Volumes,
};
use walkdir::{DirEntry, WalkDir};

/// A file whose filename or folder path is relevant to the Query.
#[derive(Debug, Clone, PartialEq)]
pub struct NameMatch {
    pub path: PathBuf,
    /// The Query is the whole filename, with or without its extension.
    pub exact: bool,
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

    fn excludes(&self, entry: &DirEntry) -> bool {
        match self {
            Exclusion::Name(pattern) => {
                wildcard_match(pattern, &entry.file_name().to_string_lossy())
            }
            Exclusion::Folder(folder) => entry.path() == folder,
        }
    }
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
const MIGRATIONS: [&str; 4] = [
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
];

/// Chunk size in characters: about 256 tokens of prose.
const CHUNK_CHARS: usize = 1000;
/// Characters each Chunk repeats from the one before (15%).
const CHUNK_OVERLAP_CHARS: usize = 150;
/// Chunks per embedding call: bounds memory for files with many Chunks.
const EMBEDDING_BATCH: usize = 8;
/// Content Matches considered per Result requested.
const CANDIDATES_PER_RESULT: usize = 10;

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
    /// Verbatim text of the best Content Match, if any.
    pub snippet: Option<String>,
    pub location: Option<Location>,
    pub match_reason: MatchReason,
}

/// The file catalog and Name Index, stored in SQLite.
pub struct Index {
    db: Connection,
    volumes: Box<dyn Volumes>,
}

impl Index {
    /// Opens the index database at `path`, creating it or applying pending migrations.
    pub fn open(path: &Path) -> Result<Self> {
        Self::migrated(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::migrated(Connection::open_in_memory()?)
    }

    /// Uses `volumes` instead of the operating system's to identify the drive of each file.
    pub fn with_volumes(self, volumes: impl Volumes + 'static) -> Self {
        Self {
            volumes: Box::new(volumes),
            ..self
        }
    }

    fn migrated(mut db: Connection) -> Result<Self> {
        let tx = db.transaction()?;
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
        })
    }

    /// Brings the Name Index in line with the files beneath the Indexed Folder `folder`,
    /// minus Exclusions: new files are added, files no longer there are removed, and
    /// changed files are reported.
    pub fn scan(&mut self, folder: &Path, exclusions: &[Exclusion]) -> Result<ScanChanges> {
        let folder = &normalized(folder);
        let volume = self.volumes.volume_of(folder)?;
        let folder_path = relative_path(folder, &volume)?;
        let mut changes = ScanChanges::default();
        let tx = self.db.transaction()?;
        let scan: i64 = tx.query_row(
            "SELECT coalesce(max(seen_in_scan), 0) + 1 FROM files",
            [],
            |row| row.get(0),
        )?;
        let entries = WalkDir::new(folder).into_iter().filter_entry(|entry| {
            entry.depth() == 0 || !exclusions.iter().any(|e| e.excludes(entry))
        });
        for entry in entries {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = relative_path(entry.path(), &volume)?;
            let metadata = entry.metadata()?;
            let size = metadata.len() as i64;
            let mtime_ns = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos() as i64);
            let known: Option<(i64, i64)> = tx
                .query_row(
                    "SELECT size, mtime_ns FROM files WHERE volume_id = ?1 AND relative_path = ?2",
                    params![volume.id, path],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some(previous) = known {
                if previous != (size, mtime_ns) {
                    changes.changed.push(entry.path().to_path_buf());
                }
                tx.execute(
                    "UPDATE files SET seen_in_scan = ?1, size = ?2, mtime_ns = ?3
                     WHERE volume_id = ?4 AND relative_path = ?5",
                    params![scan, size, mtime_ns, volume.id, path],
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
                "INSERT INTO files
                     (volume_id, relative_path, name_lower, stem_lower, size, mtime_ns, seen_in_scan)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![volume.id, path, name_lower, stem_lower, size, mtime_ns, scan],
            )?;
            changes.added.push(entry.path().to_path_buf());
            let name = entry.file_name().to_string_lossy();
            let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
            tx.execute(
                "INSERT INTO names (rowid, name, folder) VALUES (last_insert_rowid(), ?1, ?2)",
                params![name, folder],
            )?;
        }
        let gone: Vec<(i64, String)> = tx
            .prepare("SELECT id, relative_path FROM files WHERE seen_in_scan != ?1")?
            .query_map(params![scan], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, path) in gone {
            if is_beneath(&path, &folder_path) {
                tx.execute("DELETE FROM names WHERE rowid = ?1", params![id])?;
                tx.execute("DELETE FROM files WHERE id = ?1", params![id])?;
                changes.removed.push(volume.root.join(&path));
            }
        }
        tx.commit()?;
        Ok(changes)
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
        let Some(extractor) = sbm_extract::extractor_for(path) else {
            return Ok(());
        };
        let volume = self.volumes.volume_of(path)?;
        let relative = relative_path(path, &volume)?;
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
        let candidates = limit * CANDIDATES_PER_RESULT;
        let semantic: Vec<i64> = store
            .search(&embedder.embed_query(query)?, candidates)?
            .into_iter()
            .map(|neighbor| neighbor.id as i64)
            .collect();
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
        for chunk_id in reciprocal_rank_fusion(&[&semantic, &keyword]) {
            let (hash, page, line_start, line_end, text): (
                String,
                Option<u32>,
                Option<u32>,
                Option<u32>,
                String,
            ) = chunk.query_row(params![chunk_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?;
            if view == View::File && seen_contents.contains(&hash) {
                continue;
            }
            let mut paths: Vec<PathBuf> = files
                .query_map(params![hash], |row| {
                    Ok(self.path_of(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<_>>()?;
            paths.sort();
            if paths.is_empty() {
                continue;
            }
            let path = paths.remove(0);
            seen_contents.push(hash);
            results.push(SearchResult {
                path,
                copies: paths,
                snippet: Some(text),
                location: Some(location_from_columns(page, line_start, line_end)),
                match_reason: MatchReason::Content,
            });
        }
        if view == View::File {
            let names = self.search_names(query, candidates)?;
            results = fuse_name_matches(results, &names);
        }
        results.truncate(limit);
        Ok(results)
    }

    /// Where the file at the relative `path` on volume `volume_id` is now.
    fn path_of(&self, volume_id: &str, path: &str) -> PathBuf {
        self.volumes
            .mount_point(volume_id)
            .unwrap_or_default()
            .join(path)
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
                Ok(NameMatch {
                    path: self.path_of(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?),
                    exact: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(matches)
    }
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

/// Whether the relative path `path` is `folder` or beneath it.
fn is_beneath(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The Query as FTS5 syntax: each word quoted, so user text is never parsed as
/// operators, and the last word matched as a prefix because the user may still be typing it.
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
                    snippet: None,
                    location: None,
                    match_reason: MatchReason::Name,
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
