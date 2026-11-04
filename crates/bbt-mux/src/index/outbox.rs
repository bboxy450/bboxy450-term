// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Lightweight durable outbox handle; background publication retains no Index snapshot.
// Adapted from bboxy450-term src/index.rs pending_events/acknowledge, same project.
use crate::{
    Ulid,
    store::{Result, events::Draft, sqlite::SqliteStore},
};

#[derive(Clone)]
pub(crate) struct DocumentOutbox(pub(super) SqliteStore);
impl DocumentOutbox {
    pub(crate) fn page(&self, limit: usize) -> Result<Vec<Draft>> {
        let db = self.0.connection()?;
        let mut statement =
            db.prepare("SELECT payload FROM document_outbox ORDER BY seq LIMIT ?1")?;
        let pending = statement
            .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                row.get::<_, Vec<u8>>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        pending
            .into_iter()
            .map(|bytes| Ok(serde_json::from_slice(&bytes)?))
            .collect()
    }
    pub(crate) fn acknowledge(&self, event: Ulid) -> Result<()> {
        self.0.connection()?.execute(
            "DELETE FROM document_outbox WHERE id=?1",
            [event.to_string()],
        )?;
        Ok(())
    }
}
