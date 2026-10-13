//! SQLite-backed Name Index and file catalog (ADR 0003).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use sbm_core::Result;
use walkdir::WalkDir;

/// A file whose filename or folder path is relevant to the Query.
#[derive(Debug, Clone, PartialEq)]
pub struct NameMatch {
    pub path: PathBuf,
}

/// The file catalog and Name Index, stored in SQLite.
pub struct Index {
    db: Connection,
}

impl Index {
    pub fn open_in_memory() -> Result<Self> {
        let db = Connection::open_in_memory()?;
        db.execute_batch(
            "CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE);
             CREATE VIRTUAL TABLE names USING fts5(name, folder);",
        )?;
        Ok(Self { db })
    }

    /// Records every file beneath the Indexed Folder `folder` in the Name Index.
    pub fn scan(&mut self, folder: &Path, _exclusions: &[String]) -> Result<()> {
        let tx = self.db.transaction()?;
        for entry in WalkDir::new(folder) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy();
            tx.execute("INSERT INTO files (path) VALUES (?1)", params![path])?;
            let name = entry.file_name().to_string_lossy();
            let folder = entry.path().parent().unwrap_or(folder).to_string_lossy();
            tx.execute(
                "INSERT INTO names (rowid, name, folder) VALUES (last_insert_rowid(), ?1, ?2)",
                params![name, folder],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Name Matches for a Query, best first.
    pub fn search_names(&self, query: &str, limit: usize) -> Result<Vec<NameMatch>> {
        let mut statement = self.db.prepare(
            "SELECT files.path FROM names JOIN files ON files.id = names.rowid
             WHERE names MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let matches = statement
            .query_map(params![fts_query(query), limit as i64], |row| {
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
fn fts_query(query: &str) -> String {
    let words: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let mut terms: Vec<String> = words.iter().map(|word| format!("\"{word}\"")).collect();
    if let Some(last) = terms.last_mut() {
        last.push('*');
    }
    terms.join(" ")
}
