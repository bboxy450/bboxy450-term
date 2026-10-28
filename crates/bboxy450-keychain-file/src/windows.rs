// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "explicit private custody file IO"
)]
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path, PathBuf, Prefix},
};

use crate::{Acl, Error, MAX_SECRET_BYTES};
use bboxy450_keychain_windows::private_files;

pub(super) struct Store {
    root: File,
    path: PathBuf,
}
impl Store {
    pub(super) fn open(directory: &Path) -> Result<Self, Error> {
        if !directory.is_absolute()
            || directory
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            || !matches!(directory.components().next(), Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(Error::UnsafePath);
        }
        let root = private_files::directory(directory, true).map_err(io_error)?;
        let path = directory.canonicalize()?;
        Ok(Self { root, path })
    }
    fn check_root(&self) -> Result<(), Error> {
        private_files::verify(&self.root).map_err(io_error)?;
        let current = private_files::directory(&self.path, false).map_err(io_error)?;
        if !private_files::same_file(&self.root, &current)? {
            return Err(Error::UnsafePath);
        }
        Ok(())
    }
    fn slot(&self, label: &str) -> Result<Option<File>, Error> {
        self.check_root()?;
        let file = match private_files::file(&self.path.join(label), false) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error(error)),
        };
        if file.metadata()?.len() > MAX_SECRET_BYTES as u64 {
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
        self.check_root()?;
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| Error::Io(std::io::ErrorKind::Other))?;
        let temporary = self.path.join(format!(
            ".put-{}",
            nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        let mut file = private_files::file(&temporary, true).map_err(io_error)?;
        let result: Result<(), Error> = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            std::fs::hard_link(&temporary, self.path.join(label)).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::Conflict
                } else {
                    io_error(error)
                }
            })?;
            file.sync_all()?;
            Ok(())
        })();
        let cleanup = std::fs::remove_file(&temporary);
        result?;
        cleanup?;
        Ok(())
    }
    pub(super) fn remove(&self, label: &str) -> Result<(), Error> {
        if self.slot(label)?.is_none() {
            return Ok(());
        }
        match std::fs::remove_file(self.path.join(label)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }
    pub(super) fn acl(&self, label: &str) -> Result<Option<Acl>, Error> {
        Ok(self.slot(label)?.map(|_| Acl::OwnerOnly))
    }
}
fn io_error(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        Error::Permissions
    } else {
        error.into()
    }
}
