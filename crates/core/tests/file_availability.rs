//! The operating system adapter for Online-only Files, against the real file system.

use sbm_core::{FileAvailability, SystemFileAvailability};
use tempfile::TempDir;

#[test]
fn file_with_its_content_on_disk_is_not_online_only() {
    let folder = TempDir::new().unwrap();
    let file = folder.path().join("notes.txt");
    std::fs::write(&file, "on disk").unwrap();

    assert!(!SystemFileAvailability.is_online_only(&file));
}

#[cfg(windows)]
#[test]
fn windows_file_whose_content_is_offline_is_online_only() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_OFFLINE, SetFileAttributesW};

    let folder = TempDir::new().unwrap();
    let file = folder.path().join("notes.txt");
    std::fs::write(&file, "in the cloud").unwrap();
    let wide: Vec<u16> = file.as_os_str().encode_wide().chain([0]).collect();
    assert_ne!(
        unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_OFFLINE) },
        0
    );

    assert!(SystemFileAvailability.is_online_only(&file));
}
