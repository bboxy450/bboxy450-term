#![allow(
    clippy::disallowed_methods,
    reason = "private custody file IO on the IO owner"
)]
use std::{
    ffi::CString,
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path},
};

use crate::{Acl, Error, MAX_SECRET_BYTES};

pub(super) struct Store(File);
impl Store {
    pub(super) fn open(directory: &Path) -> Result<Self, Error> {
        if !directory.is_absolute()
            || directory
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(Error::UnsafePath);
        }
        // Parent creation is the harness's job; we do not repair or chmod it.
        match std::fs::DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(directory)
            .map_err(path_error)?;
        let meta = file.metadata()?;
        if !meta.is_dir() {
            return Err(Error::UnsafePath);
        }
        private(&meta, 0o700)?;
        Ok(Self(file))
    }
    fn slot(&self, label: &str) -> Result<Option<File>, Error> {
        private(&self.0.metadata()?, 0o700)?;
        let label = CString::new(label).map_err(|_| Error::InvalidLabel)?;
        // SAFETY: the directory FD and NUL-terminated relative label stay live;
        // NOFOLLOW and NONBLOCK prevent a symlink or FIFO from being followed.
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                label.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(path_error(error))
            };
        }
        // SAFETY: openat returned a new owned descriptor, adopted exactly once.
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        if !meta.is_file() || meta.nlink() != 1 {
            return Err(Error::UnsafePath);
        }
        private(&meta, 0o600)?;
        if meta.len() > MAX_SECRET_BYTES as u64 {
            return Err(Error::TooLarge);
        }
        Ok(Some(file))
    }
    pub(super) fn get(&self, label: &str) -> Result<Option<Vec<u8>>, Error> {
        let Some(file) = self.slot(label)? else {
            return Ok(None);
        };
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        file.take(MAX_SECRET_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.is_empty() {
            return Err(Error::EmptySecret);
        }
        if bytes.len() > MAX_SECRET_BYTES {
            return Err(Error::TooLarge);
        }
        Ok(Some(std::mem::take(&mut *bytes)))
    }
    pub(super) fn put(&self, label: &str, bytes: &[u8]) -> Result<(), Error> {
        private(&self.0.metadata()?, 0o700)?;
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| Error::Io(std::io::ErrorKind::Other))?;
        let temp = CString::new(format!(
            ".put-{}",
            nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
        .map_err(|_| Error::InvalidLabel)?;
        let label = CString::new(label).map_err(|_| Error::InvalidLabel)?;
        // SAFETY: valid directory FD and relative name; create-new publishes no
        // partially written destination and no inherited user-wide access.
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if fd < 0 {
            return Err(path_error(std::io::Error::last_os_error()));
        }
        // SAFETY: fd was just created and is adopted once.
        let mut file = unsafe { File::from_raw_fd(fd) };
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            // SAFETY: both names are relative to the same live directory FD.
            // linkat refuses an existing destination instead of replacing it.
            if unsafe {
                libc::linkat(
                    self.0.as_raw_fd(),
                    temp.as_ptr(),
                    self.0.as_raw_fd(),
                    label.as_ptr(),
                    0,
                )
            } < 0
            {
                let error = std::io::Error::last_os_error();
                return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::Conflict
                } else {
                    path_error(error)
                });
            }
            Ok(())
        })();
        // SAFETY: this nonce name was created by this operation, never a peer.
        let removed = unsafe { libc::unlinkat(self.0.as_raw_fd(), temp.as_ptr(), 0) };
        if result.is_ok() && removed < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        result?;
        self.0.sync_all()?;
        Ok(())
    }
    pub(super) fn remove(&self, label: &str) -> Result<(), Error> {
        if self.slot(label)?.is_none() {
            return Ok(());
        }
        let label = CString::new(label).map_err(|_| Error::InvalidLabel)?;
        // SAFETY: validated relative slot within the owned directory handle.
        if unsafe { libc::unlinkat(self.0.as_raw_fd(), label.as_ptr(), 0) } < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(error.into());
            }
        }
        self.0.sync_all()?;
        Ok(())
    }
    pub(super) fn acl(&self, label: &str) -> Result<Option<Acl>, Error> {
        Ok(self.slot(label)?.map(|_| Acl::OwnerOnly))
    }
}
fn private(meta: &std::fs::Metadata, mode: u32) -> Result<(), Error> {
    // SAFETY: geteuid has no precondition and discloses no credential material.
    if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o777 != mode {
        return Err(Error::Permissions);
    }
    Ok(())
}
fn path_error(error: std::io::Error) -> Error {
    match error.raw_os_error() {
        Some(libc::ELOOP | libc::ENOTDIR) => Error::UnsafePath,
        _ => error.into(),
    }
}
