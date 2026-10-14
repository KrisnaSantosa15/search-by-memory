//! Test doubles at the system boundaries shared by the setup tests.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

use sbm_core::Result;
use sbm_setup::{Http, RemoteFile};
use sha2::{Digest, Sha256};

/// Stands in for the web: serves files from memory and records each request.
#[derive(Clone, Default)]
pub struct FakeHttp {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// (url, first byte asked for) of every request.
    pub requests: Arc<Mutex<Vec<(String, u64)>>>,
    /// The next response breaks off after this many bytes, as a dropped connection.
    cut: Arc<Mutex<Option<usize>>>,
}

impl FakeHttp {
    /// Serves `content` at `url` and describes it as a file saved at `path`.
    pub fn serve(&self, url: &str, path: &str, content: &[u8]) -> RemoteFile {
        self.files
            .lock()
            .unwrap()
            .insert(url.to_string(), content.to_vec());
        RemoteFile {
            url: url.to_string(),
            path: path.to_string(),
            size: content.len() as u64,
            sha256: sha256(content),
        }
    }
}

impl FakeHttp {
    /// Drops the connection of the next request after `bytes` bytes.
    pub fn drop_next_connection_after(&self, bytes: usize) {
        *self.cut.lock().unwrap() = Some(bytes);
    }
}

/// A body that fails after its bytes run out, as a dropped connection.
struct Dropped(std::io::Cursor<Vec<u8>>);

impl Read for Dropped {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self.0.read(buffer)? {
            0 => Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "connection dropped",
            )),
            read => Ok(read),
        }
    }
}

impl Http for FakeHttp {
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>> {
        self.requests.lock().unwrap().push((url.to_string(), from));
        let content = self.files.lock().unwrap().get(url).cloned();
        let content = content.ok_or_else(|| format!("404 {url}"))?;
        let rest = content[from as usize..].to_vec();
        Ok(match self.cut.lock().unwrap().take() {
            Some(bytes) => Box::new(Dropped(std::io::Cursor::new(rest[..bytes].to_vec()))),
            None => Box::new(std::io::Cursor::new(rest)),
        })
    }
}

pub fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A gzipped tar archive holding `files` as (path, content).
pub fn tgz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(gzip);
    for (path, content) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, path, *content).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}
