---
status: accepted
---

# A ChangeSource reports paths to re-check, read from the USN Journal without administrator rights

Live Updates need to know what changed since the last run, including changes made while the app was not running. On Windows the NTFS USN Journal records every change, but the classic read (`FSCTL_READ_USN_JOURNAL`) needs administrator rights, and we do not want the app or a service to run elevated. Windows 11 offers `FSCTL_READ_UNPRIVILEGED_USN_JOURNAL` on a handle to the volume's root folder; its records carry file IDs, parent folder IDs, and reasons, but no names.

We therefore keep the `ChangeSource` contract minimal: for a volume and a saved cursor it returns the paths (files or folders, relative to the volume root) that may have changed, or says the history is lost and the volume needs a rescan. It does not classify events. The index re-checks each path with the same logic as a scan (size and mtime, then the content hash, ADR 0004), and a folder stands for everything beneath it. The USN adapter resolves each file ID to its current path; a deleted file, or the old name of a renamed one, cannot be resolved, so it reports the parent folder instead.

## Consequences

- No elevation, no service: the journal is read by the app itself as the signed-in user.
- Event types never leak out of the adapter; a filesystem-watcher adapter on Linux or macOS can report paths the same way.
- A delete or rename re-checks its whole parent folder recursively. This is a walk of file metadata only (no re-embedding), but a change near the top of a large tree costs a large walk.
- Volumes without a journal (FAT, exFAT, network shares) always report a lost history, so every run rescans them until a watcher-based adapter exists.
- Adding an Indexed Folder or an Exclusion forgets the affected volumes' cursors so the next run rescans them.
- The cursor is opaque to the index and is saved in the same transaction as the changes it covers, so a crash replays changes instead of losing them.
