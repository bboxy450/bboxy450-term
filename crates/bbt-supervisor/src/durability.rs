// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Ordering barriers and explicit drive flushes. A writer round groups all
//! ordered files on a device before asking that device to flush its cache.
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

pub(crate) fn sync_file(file: &File, strong: bool) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let operation = if strong {
            libc::F_FULLFSYNC
        } else {
            libc::F_BARRIERFSYNC
        };
        retry(|| unsafe { libc::fcntl(file.as_raw_fd(), operation) })
    }
    #[cfg(not(target_os = "macos"))]
    if strong {
        file.sync_all()
    } else {
        file.sync_data()
    }
}

pub(crate) fn sync_directory(directory: &File) -> io::Result<()> {
    // SAFETY: the closure uses an open owned directory descriptor; no pointer
    // arguments or ownership transfer. Drive flush belongs to the whole round.
    retry(|| unsafe { libc::fsync(directory.as_raw_fd()) })
}

fn retry(mut operation: impl FnMut() -> i32) -> io::Result<()> {
    loop {
        if operation() == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}
