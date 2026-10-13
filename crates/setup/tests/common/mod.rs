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

impl Http for FakeHttp {
    fn get(&self, url: &str, from: u64) -> Result<Box<dyn Read + Send>> {
        self.requests.lock().unwrap().push((url.to_string(), from));
        let content = self.files.lock().unwrap().get(url).cloned();
        let content = content.ok_or_else(|| format!("404 {url}"))?;
        Ok(Box::new(std::io::Cursor::new(
            content[from as usize..].to_vec(),
        )))
    }
}

pub fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
