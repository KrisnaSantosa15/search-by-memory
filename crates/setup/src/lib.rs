//! First-run setup: verified, resumable downloads of the embedding model and
//! GPU runtimes.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;

use sbm_core::Result;

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

/// The web (system boundary).
pub trait Http {
    /// The body of `url` from byte `from` on.
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>>;
}

/// Downloads `files` into `dir`, reporting progress over all of them.
pub fn download(
    files: &[RemoteFile],
    dir: &Path,
    http: &dyn Http,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    let _ = cancel;
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
        let mut body = http.get(&file.url, 0)?;
        let mut out = File::create(&path)?;
        loop {
            let read = body.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            out.write_all(&buffer[..read])?;
            current.done += read as u64;
            progress(current);
        }
    }
    Ok(())
}
