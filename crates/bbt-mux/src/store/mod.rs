// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Per-object durable storage. Call these blocking APIs on a logic/IO worker.

use std::path::{Path, PathBuf};

pub mod events;
pub mod sqlite;

pub use bbt_supervisor::store::{Blobs, Error, Result, blobs, digest, invalid, now_ms};

pub fn object_dir(root: &Path, name: &str) -> Result<PathBuf> {
    bbt_rt::assert_not_ui_thread();
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err(invalid("invalid object name"));
    }
    let path = root.join(name);
    std::fs::create_dir_all(&path)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    Ok(path)
}

pub trait Store {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn put(&self, key: &str, value: &[u8]) -> Result<()>;
    fn list(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>>;
    fn sql(
        &self,
        query: &str,
        args: &[rusqlite::types::Value],
    ) -> Result<Vec<Vec<rusqlite::types::Value>>>;
    fn alarm(&self, key: &str, deadline_ms: u64) -> Result<()>;
    fn due_alarms(&self, now_ms: u64) -> Result<Vec<String>>;
}

pub trait Events {
    fn append(
        &self,
        event: events::Draft,
        key: &ed25519_dalek::SigningKey,
    ) -> Result<events::Envelope>;
    fn after(&self, sequence: u64, limit: usize) -> Result<Vec<events::Envelope>>;
    fn verify_chain(&self) -> Result<u64>;
}

pub(crate) fn sqlint(value: u64) -> Result<i64> {
    Ok(value.try_into()?)
}
