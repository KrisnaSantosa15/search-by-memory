//! SQLite-backed Name Index and file catalog (ADR 0003).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use sbm_core::Result;
use walkdir::{DirEntry, WalkDir};

/// A file whose filename or folder path is relevant to the Query.
#[derive(Debug, Clone, PartialEq)]
pub struct NameMatch {
    pub path: PathBuf,
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

/// The file catalog and Name Index, stored in SQLite.
pub struct Index {
    db: Connection,
}

impl Index {
    pub fn open_in_memory() -> Result<Self> {
        let db = Connection::open_in_memory()?;
        db.execute_batch(
            "CREATE TABLE files (
                 id INTEGER PRIMARY KEY,
                 path TEXT NOT NULL UNIQUE,
                 name_lower TEXT NOT NULL,
                 stem_lower TEXT NOT NULL,
                 seen_in_scan INTEGER NOT NULL
             );
             CREATE VIRTUAL TABLE names USING fts5(name, folder);",
        )?;
        Ok(Self { db })
    }

    /// Brings the Name Index in line with the files beneath the Indexed Folder `folder`,
    /// minus Exclusions: new files are added and files no longer there are removed.
    pub fn scan(&mut self, folder: &Path, exclusions: &[Exclusion]) -> Result<()> {
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
            let path = entry.path().to_string_lossy();
            let known = tx.execute(
                "UPDATE files SET seen_in_scan = ?1 WHERE path = ?2",
                params![scan, path],
            )?;
            if known > 0 {
                continue;
            }
            let name_lower = entry.file_name().to_string_lossy().to_lowercase();
            let stem_lower = entry
                .path()
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            tx.execute(
                "INSERT INTO files (path, name_lower, stem_lower, seen_in_scan)
                 VALUES (?1, ?2, ?3, ?4)",
                params![path, name_lower, stem_lower, scan],
            )?;
            let name = entry.file_name().to_string_lossy();
            let folder = entry.path().parent().unwrap_or(folder).to_string_lossy();
            tx.execute(
                "INSERT INTO names (rowid, name, folder) VALUES (last_insert_rowid(), ?1, ?2)",
                params![name, folder],
            )?;
        }
        let gone: Vec<(i64, String)> = tx
            .prepare("SELECT id, path FROM files WHERE seen_in_scan != ?1")?
            .query_map(params![scan], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, path) in gone {
            if Path::new(&path).starts_with(folder) {
                tx.execute("DELETE FROM names WHERE rowid = ?1", params![id])?;
                tx.execute("DELETE FROM files WHERE id = ?1", params![id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Name Matches for a Query, best first.
    pub fn search_names(&self, query: &str, limit: usize) -> Result<Vec<NameMatch>> {
        let exact = query.trim().to_lowercase();
        let Some(query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let mut statement = self.db.prepare(
            "SELECT files.path FROM names JOIN files ON files.id = names.rowid
             WHERE names MATCH ?1
             ORDER BY (files.name_lower = ?3 OR files.stem_lower = ?3) DESC, rank
             LIMIT ?2",
        )?;
        let matches = statement
            .query_map(params![query, limit as i64, exact], |row| {
                Ok(NameMatch {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(matches)
    }
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
