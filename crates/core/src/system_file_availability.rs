use std::path::Path;

use crate::FileAvailability;

/// The operating system's view of which files are Online-only Files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemFileAvailability;

impl FileAvailability for SystemFileAvailability {
    fn is_online_only(&self, path: &Path) -> bool {
        platform::is_online_only(path)
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        FILE_ATTRIBUTE_RECALL_ON_OPEN, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
    };

    /// Reads the attributes from the directory entry: unlike opening the file,
    /// this never makes a sync client (OneDrive, Google Drive, Dropbox) download it.
    pub fn is_online_only(path: &Path) -> bool {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        attributes != INVALID_FILE_ATTRIBUTES
            && attributes
                & (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_OFFLINE)
                != 0
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    /// No sync client support outside Windows yet (M7).
    pub fn is_online_only(_path: &Path) -> bool {
        false
    }
}
