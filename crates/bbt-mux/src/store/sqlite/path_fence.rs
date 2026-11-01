// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Synchronous vnode invalidation for canonical paths; every uncertainty falls
//! back to the original path stat. No FSEvents callback or background waiter.
use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
};

pub(super) struct PathFence {
    queue: File,
    _directories: Vec<File>,
}
impl PathFence {
    pub fn new(path: &Path, expected: (u64, u64)) -> Option<Self> {
        // An alias can change through a symlink target's ancestors. Only the
        // canonical, absolute no-symlink case uses this optimization.
        if path.canonicalize().ok()?.as_path() != path {
            return None;
        }
        // SAFETY: kqueue creates a new descriptor; File owns it immediately.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return None;
        }
        // SAFETY: fd was just created and has no other owner.
        let queue = unsafe { File::from_raw_fd(fd) };
        // SAFETY: only this newly owned descriptor's inheritance flags change.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
            return None;
        }
        let ancestors: Vec<_> = path.ancestors().skip(1).collect();
        let directories: Vec<File> = ancestors
            .iter()
            .map(File::open)
            .collect::<std::io::Result<_>>()
            .ok()?;
        // Remote namespace edits need not produce local vnode notifications.
        // Keep those filesystems on the original per-read path stat.
        let mut filesystem = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: fstatfs initializes this output on success; the fd is live.
        if unsafe { libc::fstatfs(directories.first()?.as_raw_fd(), filesystem.as_mut_ptr()) } != 0
        {
            return None;
        }
        // SAFETY: fstatfs above succeeded and initialized every field.
        if unsafe { filesystem.assume_init() }.f_flags & u32::try_from(libc::MNT_LOCAL).ok()? == 0 {
            return None;
        }
        let changes: Vec<_> = directories
            .iter()
            .map(|dir| libc::kevent {
                ident: dir.as_raw_fd() as _,
                filter: libc::EVFILT_VNODE,
                flags: libc::EV_ADD | libc::EV_CLEAR,
                // Parent writes cover replacement of any component, including a
                // new symlink. Rename/delete/revoke cover the watched directories.
                fflags: libc::NOTE_WRITE
                    | libc::NOTE_RENAME
                    | libc::NOTE_DELETE
                    | libc::NOTE_REVOKE,
                data: 0,
                udata: std::ptr::null_mut(),
            })
            .collect();
        // SAFETY: all registrations borrow the live directory Files retained
        // below. This submits changes without waiting for any event.
        if unsafe {
            libc::kevent(
                fd,
                changes.as_ptr(),
                changes.len() as _,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        } != 0
        {
            return None;
        }
        // Check the opened directory chain too: a parent could have been
        // replaced during setup while a hard link kept the database inode equal.
        for (path, directory) in ancestors.into_iter().zip(&directories) {
            use std::os::unix::fs::MetadataExt;
            let opened = directory.metadata().ok()?;
            let current = std::fs::metadata(path).ok()?;
            if (opened.dev(), opened.ino()) != (current.dev(), current.ino()) {
                return None;
            }
        }
        if path.canonicalize().ok()?.as_path() != path {
            return None;
        }
        let fence = Self {
            queue,
            _directories: directories,
        };
        // Registration precedes the final identity check. A change during setup
        // cannot certify a different inode; any queued uncertainty disables reuse.
        if super::identity(path) != Some(expected) {
            return None;
        }
        Some(fence)
    }
    pub fn unchanged(&self) -> bool {
        let mut event = libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: one initialized output slot, a live owned queue and a zero
        // timeout. Errors, overflow/revoke and all events demand a path recheck.
        unsafe {
            libc::kevent(
                self.queue.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            ) == 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Store, sqlite::SqliteStore};
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap().join("parent/object");
        std::fs::create_dir_all(&path).unwrap();
        (root, path)
    }
    #[test]
    fn inode_fence_detects_leaf_and_ancestor_replacement_without_waiting() {
        for ancestor in [false, true] {
            let (_root, directory) = fixture();
            let store = SqliteStore::open(&directory).unwrap();
            store.put("which", b"old").unwrap();
            let expected = super::super::identity(&store.path).unwrap();
            let fence = PathFence::new(&store.path, expected).unwrap();
            // Ancestor notifications may conservatively include unrelated peers.
            // No current-path replacement is required for a dirty signal.
            if ancestor {
                let parent = directory.parent().unwrap();
                std::fs::rename(parent, parent.with_file_name("aside")).unwrap();
                std::fs::create_dir_all(&directory).unwrap();
            } else {
                // Move the whole database directory so WAL/SHM follow it.
                std::fs::rename(&directory, directory.with_file_name("aside")).unwrap();
                std::fs::create_dir_all(&directory).unwrap();
            }
            assert!(!fence.unchanged());
            let replacement = SqliteStore::open(&directory).unwrap();
            replacement.put("which", b"new").unwrap();
            assert_eq!(store.get("which").unwrap(), Some(b"new".to_vec()));
            assert_ne!(super::super::identity(&store.path), Some(expected));
        }
    }
    #[test]
    fn inode_fence_detects_a_replaced_leaf_with_the_original_still_linked() {
        let (_root, directory) = fixture();
        let path = directory.join("leaf");
        std::fs::write(&path, b"old").unwrap();
        let expected = super::super::identity(&path).unwrap();
        let fence = PathFence::new(&path, expected).unwrap();
        std::fs::rename(&path, path.with_extension("aside")).unwrap();
        std::fs::write(&path, b"new").unwrap();
        assert!(!fence.unchanged());
        assert_ne!(super::super::identity(&path), Some(expected));
    }
    #[test]
    fn inode_fence_keeps_data_writes_fresh_and_refuses_symlink_aliases() {
        let (_root, directory) = fixture();
        let store = SqliteStore::open(&directory).unwrap();
        store.put("which", b"first").unwrap();
        let expected = super::super::identity(&store.path).unwrap();
        let fence = PathFence::new(&store.path, expected).unwrap();
        store.put("which", b"second").unwrap();
        assert_eq!(super::super::identity(&store.path), Some(expected));
        drop(fence);
        assert_eq!(store.get("which").unwrap(), Some(b"second".to_vec()));
        let alias = directory.with_file_name("alias");
        std::os::unix::fs::symlink(&directory, &alias).unwrap();
        assert!(PathFence::new(&alias.join("object.sqlite"), expected).is_none());
    }
}
