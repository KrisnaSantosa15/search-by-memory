//! The operating system adapter for volume identity, against the real file system.

use sbm_core::{SystemVolumes, Volumes};
use tempfile::TempDir;

#[test]
fn folders_on_the_same_drive_share_a_volume_whose_root_holds_them() {
    let folder = TempDir::new().unwrap();
    let inner = folder.path().join("inner");
    std::fs::create_dir(&inner).unwrap();

    let outer = SystemVolumes.volume_of(folder.path()).unwrap();
    let inner_volume = SystemVolumes.volume_of(&inner).unwrap();

    assert_eq!(outer, inner_volume);
    assert!(folder.path().starts_with(&outer.root), "{outer:?}");
}

#[test]
fn volume_is_found_again_at_its_mount_point() {
    let folder = TempDir::new().unwrap();

    let volume = SystemVolumes.volume_of(folder.path()).unwrap();

    assert_eq!(SystemVolumes.mount_point(&volume.id), Some(volume.root));
}

#[cfg(windows)]
#[test]
fn windows_volume_id_is_the_volume_guid_not_the_drive_letter() {
    let folder = TempDir::new().unwrap();

    let volume = SystemVolumes.volume_of(folder.path()).unwrap();

    assert!(volume.id.starts_with(r"\\?\Volume{"), "{volume:?}");
}

#[cfg(windows)]
#[test]
fn volume_that_is_not_connected_has_no_mount_point() {
    let missing = r"\\?\Volume{00000000-0000-0000-0000-000000000000}\";

    assert_eq!(SystemVolumes.mount_point(missing), None);
}
