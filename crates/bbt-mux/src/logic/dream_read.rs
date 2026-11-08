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
        &self,
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
        self.dream_runtime
            .read(&self.handle, authority, call, move |result| {
                let _ = reply.send(Ok(result));
            });
    }
}
