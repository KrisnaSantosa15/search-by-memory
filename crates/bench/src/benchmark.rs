//! Labelled Benchmark Queries and their Targets.

use std::path::PathBuf;
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
            .collect::<Result<_>>()?;
        Ok(Self { queries })
    }

    pub fn queries(&self) -> &[Query] {
        &self.queries
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
        Ok(match (self.page, self.lines, &self.time) {
            (_, _, Some([start, end])) => Location::Time {
                start: parse_timestamp(start)?,
                end: parse_timestamp(end)?,
            },
            (_, Some([start, end]), _) => Location::Lines { start, end },
            (Some(page), _, _) => Location::Page(page),
            (None, None, None) => Location::WholeFile,
        })
    }
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
