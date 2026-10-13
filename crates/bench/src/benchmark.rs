//! Labelled Benchmark Queries and their Targets.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sbm_core::{Location, Result};
use serde::Deserialize;

/// A fixed set of labelled Queries.
pub struct Benchmark {
    queries: Vec<Query>,
}

/// A Benchmark Query and the Targets where its remembered content lives.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub id: String,
    pub text: String,
    pub language: String,
    pub targets: Vec<Target>,
}

/// A file, relative to the corpus root, plus the Location of the remembered content.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub file: PathBuf,
    pub location: Location,
}

impl Benchmark {
    /// Parses a Benchmark file in TOML.
    pub fn parse(source: &str) -> Result<Self> {
        let file: BenchmarkFile = toml::from_str(source)?;
        let queries = file
            .query
            .into_iter()
            .map(|query| {
                if query.target.is_empty() {
                    return Err(format!("Query {:?} has no Target", query.id).into());
                }
                let targets = query
                    .target
                    .into_iter()
                    .map(|target| {
                        Ok(Target {
                            location: target.location()?,
                            file: target.file,
                        })
                    })
                    .collect::<Result<_>>()?;
                Ok(Query {
                    id: query.id,
                    text: query.text,
                    language: query.language,
                    targets,
                })
            })
            .collect::<Result<Vec<Query>>>()?;
        let mut ids = HashSet::new();
        if let Some(duplicate) = queries.iter().find(|query| !ids.insert(&query.id)) {
            return Err(format!("Query id {:?} is used more than once", duplicate.id).into());
        }
        Ok(Self { queries })
    }

    pub fn queries(&self) -> &[Query] {
        &self.queries
    }

    /// Target files, relative to `corpus_root`, that do not exist there.
    pub fn missing_targets(&self, corpus_root: &Path) -> Vec<PathBuf> {
        self.queries
            .iter()
            .flat_map(|query| &query.targets)
            .filter(|target| !corpus_root.join(&target.file).is_file())
            .map(|target| target.file.clone())
            .collect()
    }
}

#[derive(Deserialize)]
struct BenchmarkFile {
    query: Vec<QueryEntry>,
}

#[derive(Deserialize)]
struct QueryEntry {
    id: String,
    text: String,
    language: String,
    target: Vec<TargetEntry>,
}

#[derive(Deserialize)]
struct TargetEntry {
    file: PathBuf,
    page: Option<u32>,
    lines: Option<[u32; 2]>,
    time: Option<[String; 2]>,
}

impl TargetEntry {
    fn location(&self) -> Result<Location> {
        let kinds = [self.page.is_some(), self.lines.is_some(), self.time.is_some()];
        if kinds.iter().filter(|&&given| given).count() > 1 {
            return Err(format!("{:?} has more than one Location", self.file).into());
        }
        Ok(match (self.page, self.lines, &self.time) {
            (_, _, Some([start, end])) => {
                let (start, end) = (parse_timestamp(start)?, parse_timestamp(end)?);
                ensure_ordered(start, end)?;
                Location::Time { start, end }
            }
            (_, Some([start, end]), _) => {
                ensure_ordered(start, end)?;
                Location::Lines { start, end }
            }
            (Some(0), _, _) => return Err("pages start at 1".into()),
            (Some(page), _, _) => Location::Page(page),
            (None, None, None) => Location::WholeFile,
        })
    }
}

fn ensure_ordered<T: PartialOrd + std::fmt::Debug>(start: T, end: T) -> Result<()> {
    if end < start {
        return Err(format!("range ends at {end:?} before it starts at {start:?}").into());
    }
    Ok(())
}

/// Parses `[h:]mm:ss` into the time from the start of the media.
fn parse_timestamp(timestamp: &str) -> Result<Duration> {
    let seconds = timestamp.split(':').try_fold(0u64, |total, part| {
        part.parse::<u64>()
            .map(|value| total * 60 + value)
            .map_err(|_| format!("invalid timestamp {timestamp:?}"))
    })?;
    Ok(Duration::from_secs(seconds))
}
