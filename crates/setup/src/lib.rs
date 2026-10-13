//! First-run setup: verified, resumable downloads of the embedding model and
//! GPU runtimes.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sbm_core::Result;
use sha2::{Digest, Sha256};

/// A file to download.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteFile {
    pub url: String,
    /// Where it is saved, relative to the download folder, with forward slashes.
    pub path: String,
    pub size: u64,
    /// Lowercase hex SHA-256 of the whole file.
    pub sha256: String,
}

/// Bytes of a download on disk so far, out of all of them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

/// How a download ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finished {
    Complete,
    /// Stopped on request; the next download resumes where this one stopped.
    Cancelled,
}

/// The web (system boundary).
pub trait Http {
    /// The body of `url` from byte `from` on.
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>>;
}

/// The web over HTTPS.
pub struct Web {
    agent: ureq::Agent,
}

impl Web {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self { agent }
    }
}

impl Default for Web {
    fn default() -> Self {
        Self::new()
    }
}

impl Http for Web {
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>> {
        let mut request = self.agent.get(url);
        if from > 0 {
            request = request.header("Range", format!("bytes={from}-"));
        }
        let response = request.call()?;
        let resumed = response.status() == 206;
        let mut body = response.into_body().into_reader();
        if from > 0 && !resumed {
            // The server sent the whole file: skip what is already on disk.
            std::io::copy(&mut (&mut body).take(from), &mut std::io::sink())?;
        }
        Ok(Box::new(body))
    }
}

/// Downloads `files` into `dir`, reporting progress over all of them.
pub fn download(
    files: &[RemoteFile],
    dir: &Path,
    http: &dyn Http,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Finished> {
    let mut current = Progress {
        done: 0,
        total: files.iter().map(|file| file.size).sum(),
    };
    let mut buffer = vec![0; 64 * 1024];
    for file in files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Only a verified download gets its final name.
        if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() == file.size) {
            current.done += file.size;
            progress(current);
            continue;
        }
        // Bytes already fetched wait in a `.part` file, so a new run resumes there.
        let part = dir.join(format!("{}.part", file.path));
        let from = std::fs::metadata(&part).map_or(0, |metadata| metadata.len());
        current.done += from;
        let mut out = File::options().create(true).append(true).open(&part)?;
        // One byte past the expected size is enough to tell the file is wrong.
        let mut body = http
            .get(&file.url, from)?
            .take(file.size.saturating_sub(from) + 1);
        let mut written = from;
        loop {
            let read = body.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            written += read as u64;
            if written > file.size {
                drop(out);
                std::fs::remove_file(&part)?;
                return Err(format!("{} is larger than expected", file.path).into());
            }
            out.write_all(&buffer[..read])?;
            current.done += read as u64;
            progress(current);
            if cancel.load(Ordering::SeqCst) {
                return Ok(Finished::Cancelled);
            }
        }
        drop(out);
        if sha256_of(&part)? != file.sha256 {
            std::fs::remove_file(&part)?;
            return Err(
                format!("{} arrived damaged and will be downloaded again", file.path).into(),
            );
        }
        std::fs::rename(&part, &path)?;
    }
    Ok(Finished::Complete)
}

/// Lowercase hex SHA-256 of the file at `path`, read in pieces.
fn sha256_of(path: &Path) -> Result<String> {
    let (mut hasher, mut file) = (Sha256::new(), File::open(path)?);
    let mut buffer = vec![0; 64 * 1024];
    loop {
        match file.read(&mut buffer)? {
            0 => break,
            read => hasher.update(&buffer[..read]),
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
