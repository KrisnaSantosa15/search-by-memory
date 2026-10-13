//! Labelled Benchmark Queries and their Targets.

use std::path::PathBuf;

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
            .map(|query| Query {
                id: query.id,
                text: query.text,
                language: query.language,
                targets: query
                    .target
                    .into_iter()
                    .map(|target| Target {
                        file: target.file,
                        location: Location::Page(target.page),
                    })
                    .collect(),
            })
            .collect();
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
    page: u32,
}
