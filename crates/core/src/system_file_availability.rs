use std::path::Path;

use crate::FileAvailability;

/// The operating system's view of which files are Online-only Files.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemFileAvailability;

impl FileAvailability for SystemFileAvailability {
    fn is_online_only(&self, _path: &Path) -> bool {
        false
    }
}
