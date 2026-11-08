// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read admission uses the existing public C1 authority, off the logic owner.
use super::*;
impl Worker {
    pub(super) fn start_dream_read(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        let authority = match self.dream_authority(id, generation) {
            Ok(authority) => authority,
            Err(_) => {
                let _ = reply.send(Ok(crate::mcp::dream_read::result(
                    &call,
                    bbt_proto::dream_read::Reply::failure(
                        bbt_proto::dream_read::Code::StaleContext,
                    ),
                )));
                return;
            }
        };
        let journal = if call.tool == "dream_feedback" {
            // spawn initialized the canonical own-seat object store. This
            // lazy handle does no IO on the logic owner; SQL runs below on the
            // existing isolated request owner and uses the existing journal.
            let path = self
                .config
                .run_dir
                .join(format!("seat-{id}"))
                .join("object.sqlite");
            let journal = self
                .control_journals
                .entry(id)
                .or_insert_with(|| {
                    crate::mcp::journal::Journal::new(crate::store::sqlite::SqliteStore { path })
                })
                .clone();
            Some(self.root_journal(&journal, &authority.port))
        } else {
            None
        };
        let original = call.clone();
        let final_authority = authority.clone();
        self.dream_runtime
            .read(&self.handle, authority, call, move |result| {
                let result = match journal {
                    Some(journal) => crate::registry::dream_read::journal_feedback(
                        &journal,
                        &final_authority,
                        &original,
                        result,
                    ),
                    None => result,
                };
                let _ = reply.send(Ok(result));
            });
    }
}
