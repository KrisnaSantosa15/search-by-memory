//! Platform adapter: the running operating system's volumes.

use std::path::{Path, PathBuf};

use crate::{Result, Volume, Volumes};

/// The running operating system's volumes. On Windows a volume is identified by
/// its volume GUID, so it keeps its identity under another drive letter; elsewhere
/// by its mount point.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemVolumes;

impl Volumes for SystemVolumes {
    fn volume_of(&self, path: &Path) -> Result<Volume> {
        platform::volume_of(path)
    }

    fn mount_point(&self, id: &str) -> Option<PathBuf> {
        platform::mount_point(id)
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Storage::FileSystem::{
        GetVolumeNameForVolumeMountPointW, GetVolumePathNameW, GetVolumePathNamesForVolumeNameW,
    };

    use crate::{Result, Volume};

    /// Characters per path buffer; volume roots and GUID names are far shorter.
    const BUFFER: usize = 1024;
    const GUID_PREFIX: &str = r"\\?\Volume{";

    pub fn volume_of(path: &Path) -> Result<Volume> {
        let mut root = [0u16; BUFFER];
        // SAFETY: the input is NUL-terminated and the buffer length is passed.
        let found = unsafe {
            GetVolumePathNameW(
                wide(path.as_os_str()).as_ptr(),
                root.as_mut_ptr(),
                BUFFER as u32,
            )
        };
        if found == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let root = until_nul(&root);
        let mut name = [0u16; BUFFER];
        // SAFETY: as above.
        let named = unsafe {
            GetVolumeNameForVolumeMountPointW(
                wide(OsStr::new(&root)).as_ptr(),
                name.as_mut_ptr(),
                BUFFER as u32,
            )
        };
        // Network shares have no volume GUID; their root is their identity.
        let id = if named != 0 {
            until_nul(&name)
        } else {
            root.clone()
        };
        Ok(Volume {
            id,
            root: PathBuf::from(root),
        })
    }

    pub fn mount_point(id: &str) -> Option<PathBuf> {
        if !id.starts_with(GUID_PREFIX) {
            let root = PathBuf::from(id);
            return root.exists().then_some(root);
        }
        let mut roots = [0u16; BUFFER];
        let mut length = 0u32;
        // SAFETY: as above; `length` receives the characters written.
        let found = unsafe {
            GetVolumePathNamesForVolumeNameW(
                wide(OsStr::new(id)).as_ptr(),
                roots.as_mut_ptr(),
                BUFFER as u32,
                &mut length,
            )
        };
        // The roots are a NUL-separated list; the first is the drive letter, if any.
        let root = until_nul(&roots);
        (found != 0 && !root.is_empty()).then(|| PathBuf::from(root))
    }

    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn until_nul(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }
}

#[cfg(not(windows))]
mod platform {
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use crate::{Result, Volume};

    /// The volume is the file system mounted at the highest folder above `path`
    /// on the same device.
    pub fn volume_of(path: &Path) -> Result<Volume> {
        let device = std::fs::metadata(path)?.dev();
        let mut root = path;
        while let Some(parent) = root.parent() {
            if std::fs::metadata(parent)?.dev() != device {
                break;
            }
            root = parent;
        }
        Ok(Volume {
            id: root.to_string_lossy().into_owned(),
            root: root.to_path_buf(),
        })
    }

    pub fn mount_point(id: &str) -> Option<PathBuf> {
        let root = PathBuf::from(id);
        root.exists().then_some(root)
    }
}
