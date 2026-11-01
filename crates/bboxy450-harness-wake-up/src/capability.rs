// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Admission files owned by one private launch overlay, never an exec environment.
use std::{
    fs::{Metadata, OpenOptions, Permissions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

/// Fixed, non-secret refusal codes shared by the MCP bridge and hook consumer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No admission-file path was provided.
    MissingFile,
    /// The final component is a symbolic link.
    Symlink,
    /// Admission must come from a regular file, not a directory/device/socket.
    NotRegularFile,
    /// Only a file owned by this process's effective uid is admitted.
    WrongOwner,
    /// The permission bits must be exactly 0600, including no special bits.
    WrongMode,
    /// The wire format is exactly 64 hexadecimal bytes followed by a newline.
    Malformed,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingFile => "seat_admission_file_missing",
            Self::Symlink => "seat_admission_file_symlink",
            Self::NotRegularFile => "seat_admission_file_not_regular",
            Self::WrongOwner => "seat_admission_file_wrong_owner",
            Self::WrongMode => "seat_admission_file_wrong_mode",
            Self::Malformed => "seat_admission_file_malformed",
        })
    }
}
impl std::error::Error for Refusal {}
impl std::fmt::Debug for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
fn refused(reason: Refusal) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn validate(meta: &Metadata) -> io::Result<()> {
    if meta.file_type().is_symlink() {
        return Err(refused(Refusal::Symlink));
    }
    if !meta.is_file() {
        return Err(refused(Refusal::NotRegularFile));
    }
    // SAFETY: geteuid has no arguments or borrowed memory.
    if meta.uid() != unsafe { libc::geteuid() } {
        return Err(refused(Refusal::WrongOwner));
    }
    if meta.mode() & 0o7777 != 0o600 {
        return Err(refused(Refusal::WrongMode));
    }
    Ok(())
}

/// Create the overlay's new 0600 capability without following or replacing a file.
///
/// # Errors
/// Returns filesystem failures; an existing final component always refuses.
pub fn create(overlay: &Path, capability: &[u8; 32]) -> io::Result<PathBuf> {
    let path = overlay.join("capability");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)?;
    // Set on the owned descriptor so even a restrictive umask yields exact 0600.
    file.set_permissions(Permissions::from_mode(0o600))?;
    let mut bytes = String::with_capacity(65);
    for byte in capability {
        use std::fmt::Write as _;
        write!(&mut bytes, "{byte:02x}").expect("String formatting");
    }
    bytes.push('\n');
    file.write_all(bytes.as_bytes())?;
    Ok(path)
}

/// Reopen and validate admission on every consumer start; reading never unlinks it.
///
/// # Errors
/// Returns a typed [`Refusal`] within `io::Error` for unsafe shape/content, or IO
/// errors without the file's path or contents. Symlinks are refused at open too.
pub fn read(path: &Path) -> io::Result<[u8; 32]> {
    validate(&std::fs::symlink_metadata(path)?)?;
    let file = OpenOptions::new()
        .read(true)
        // Nonblocking prevents a replacement FIFO from hanging before fstat.
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                refused(Refusal::Symlink)
            } else {
                error
            }
        })?;
    validate(&file.metadata()?)?;
    let mut bytes = Vec::with_capacity(66);
    file.take(66).read_to_end(&mut bytes)?;
    if bytes.len() != 65 || bytes[64] != b'\n' || !bytes[..64].iter().all(u8::is_ascii_hexdigit) {
        return Err(refused(Refusal::Malformed));
    }
    let mut capability = [0; 32];
    for (out, pair) in capability.iter_mut().zip(bytes[..64].as_chunks::<2>().0) {
        let hex = |byte: u8| {
            if byte <= b'9' {
                byte - b'0'
            } else {
                byte.to_ascii_lowercase() - b'a' + 10
            }
        };
        *out = hex(pair[0]) * 16 + hex(pair[1]);
    }
    Ok(capability)
}

/// Read the path-only launch environment. Legacy value variables grant nothing.
///
/// # Errors
/// Returns [`Refusal::MissingFile`] if the path is absent, or [`read`]'s errors.
pub fn from_env() -> io::Result<[u8; 32]> {
    let path = std::env::var_os("BBOXY_CAPABILITY_FILE")
        .filter(|p| !p.is_empty())
        .ok_or_else(|| refused(Refusal::MissingFile))?;
    read(Path::new(&path))
}
