//! Platform adapter: the NTFS USN Journal as a `ChangeSource` (FR-04).

use sbm_core::{ChangeCursor, ChangeSource, Changes, Result};

/// The change journal of the running operating system's volumes. Read without
/// administrator rights, so records carry no names: paths are looked up by file ID.
#[derive(Debug, Clone, Copy, Default)]
pub struct UsnJournal;

impl ChangeSource for UsnJournal {
    fn changes_since(&self, volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes> {
        platform::changes_since(volume_id, since)
    }
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use sbm_core::{ChangeCursor, Changes, Result};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_FUNCTION, ERROR_JOURNAL_DELETE_IN_PROGRESS,
        ERROR_JOURNAL_NOT_ACTIVE, ERROR_NOT_SUPPORTED, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdType,
        GetFinalPathNameByHandleW, OPEN_EXISTING, OpenFileById, VOLUME_NAME_NONE,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_UNPRIVILEGED_USN_JOURNAL, READ_USN_JOURNAL_DATA_V1,
        USN_JOURNAL_DATA_V0, USN_REASON_FILE_DELETE, USN_REASON_RENAME_OLD_NAME, USN_RECORD_V2,
    };

    /// Bytes of records per read.
    const READ_BUFFER: usize = 64 * 1024;
    /// Characters per path buffer.
    const PATH_BUFFER: usize = 32 * 1024;
    const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;

    /// A handle closed when dropped.
    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle is open and owned by this value.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub fn changes_since(volume_id: &str, since: Option<&ChangeCursor>) -> Result<Changes> {
        // Opening the root folder, not the volume, needs no administrator rights.
        let root = open_root(volume_id)?;
        let Some(journal) = query_journal(&root)? else {
            return Ok(Changes::Lost {
                next: ChangeCursor(String::new()),
            });
        };
        let cursor = |usn: i64| ChangeCursor(format!("{:x}:{usn}", journal.UsnJournalID));
        let Some(start) = since.and_then(|since| {
            let (id, usn) = since.0.split_once(':')?;
            let usn: i64 = usn.parse().ok()?;
            (u64::from_str_radix(id, 16).ok()? == journal.UsnJournalID && usn >= journal.FirstUsn)
                .then_some(usn)
        }) else {
            return Ok(Changes::Lost {
                next: cursor(journal.NextUsn),
            });
        };
        let mut paths = Vec::new();
        // A file usually has several records per read; look each one up once.
        let mut known: HashMap<u64, Option<String>> = HashMap::new();
        let mut lookup = |id: u64| {
            known
                .entry(id)
                .or_insert_with(|| path_of(&root, id))
                .clone()
        };
        let mut usn = start;
        let mut buffer = vec![0u8; READ_BUFFER];
        while usn < journal.NextUsn {
            let request = READ_USN_JOURNAL_DATA_V1 {
                StartUsn: usn,
                ReasonMask: u32::MAX,
                ReturnOnlyOnClose: 0,
                Timeout: 0,
                BytesToWaitFor: 0,
                UsnJournalID: journal.UsnJournalID,
                MinMajorVersion: 2,
                MaxMajorVersion: 2,
            };
            let mut returned = 0u32;
            // SAFETY: the request and buffer outlive the call and their sizes are passed.
            let read = unsafe {
                DeviceIoControl(
                    root.0,
                    FSCTL_READ_UNPRIVILEGED_USN_JOURNAL,
                    &request as *const _ as *const _,
                    size_of::<READ_USN_JOURNAL_DATA_V1>() as u32,
                    buffer.as_mut_ptr() as *mut _,
                    READ_BUFFER as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if read == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let returned = returned as usize;
            // The buffer starts with the USN to continue from, then the records.
            usn = i64::from_le_bytes(buffer[..8].try_into()?);
            let mut offset = 8;
            while offset + size_of::<USN_RECORD_V2>() <= returned {
                // SAFETY: a whole record header lies within the returned bytes.
                let record: USN_RECORD_V2 =
                    unsafe { std::ptr::read_unaligned(buffer[offset..].as_ptr() as *const _) };
                if record.RecordLength == 0 {
                    break;
                }
                // A file deleted or renamed away is gone from where it was, so its
                // folder is re-checked instead; so is a file that no longer resolves.
                let gone =
                    record.Reason & (USN_REASON_FILE_DELETE | USN_REASON_RENAME_OLD_NAME) != 0;
                let path = if gone {
                    None
                } else {
                    lookup(record.FileReferenceNumber)
                };
                if let Some(path) = path.or_else(|| lookup(record.ParentFileReferenceNumber)) {
                    paths.push(path);
                }
                offset += record.RecordLength as usize;
            }
            if returned <= 8 {
                break;
            }
        }
        paths.sort();
        paths.dedup();
        Ok(Changes::Paths {
            paths,
            next: cursor(usn),
        })
    }

    fn open_root(volume_id: &str) -> Result<Handle> {
        let name: Vec<u16> = OsStr::new(volume_id)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: the name is NUL-terminated; no security attributes or template.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                0,
                SHARE_ALL,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                0,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(Handle(handle))
    }

    /// The volume's journal, or `None` if it keeps none (e.g. FAT, exFAT, network shares).
    fn query_journal(root: &Handle) -> Result<Option<USN_JOURNAL_DATA_V0>> {
        // SAFETY: plain old data; zero is a valid value.
        let mut journal: USN_JOURNAL_DATA_V0 = unsafe { std::mem::zeroed() };
        let mut returned = 0u32;
        // SAFETY: the output buffer outlives the call and its size is passed.
        let queried = unsafe {
            DeviceIoControl(
                root.0,
                FSCTL_QUERY_USN_JOURNAL,
                std::ptr::null(),
                0,
                &mut journal as *mut _ as *mut _,
                size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if queried == 0 {
            let error = std::io::Error::last_os_error();
            let no_journal = [
                ERROR_INVALID_FUNCTION,
                ERROR_NOT_SUPPORTED,
                ERROR_JOURNAL_NOT_ACTIVE,
                ERROR_JOURNAL_DELETE_IN_PROGRESS,
            ];
            return match error.raw_os_error() {
                Some(code) if no_journal.contains(&(code as u32)) => Ok(None),
                _ => Err(error.into()),
            };
        }
        Ok(Some(journal))
    }

    /// The forward-slash path, relative to the volume root, of the existing file
    /// or folder with file ID `id`; `None` once it is gone.
    fn path_of(root: &Handle, id: u64) -> Option<String> {
        let descriptor = FILE_ID_DESCRIPTOR {
            dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: FileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: id as i64 },
        };
        // SAFETY: the descriptor outlives the call; no security attributes.
        let handle = unsafe {
            OpenFileById(
                root.0,
                &descriptor,
                0,
                SHARE_ALL,
                std::ptr::null(),
                FILE_FLAG_BACKUP_SEMANTICS,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return None;
        }
        let file = Handle(handle);
        let mut path = vec![0u16; PATH_BUFFER];
        // SAFETY: the buffer length is passed.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.0,
                path.as_mut_ptr(),
                PATH_BUFFER as u32,
                VOLUME_NAME_NONE,
            )
        } as usize;
        if length == 0 || length > PATH_BUFFER {
            return None;
        }
        // Without a volume name the path starts at the root: `\Users\me\notes.txt`.
        let path = String::from_utf16_lossy(&path[..length]);
        let path = path.trim_start_matches('\\').replace('\\', "/");
        // NTFS metadata, e.g. `$Extend/$Deleted` holding files deleted while open.
        (!path.starts_with("$Extend/")).then_some(path)
    }
}

/// Elsewhere there is no USN Journal: every read asks for a rescan.
#[cfg(not(windows))]
mod platform {
    use sbm_core::{ChangeCursor, Changes, Result};

    pub fn changes_since(_volume_id: &str, _since: Option<&ChangeCursor>) -> Result<Changes> {
        Ok(Changes::Lost {
            next: ChangeCursor(String::new()),
        })
    }
}
