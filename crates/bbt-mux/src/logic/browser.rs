// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Seat admission and process ownership for offscreen Chromium.
#[cfg(all(feature = "browser", target_os = "macos"))]
mod session;
#[cfg(all(feature = "browser", target_os = "macos"))]
pub(super) use session::{Capture, Sessions};

use super::Worker;
use crate::{
    mcp::{self, Call},
    store::Result,
};
use serde_json::Value;
use tokio::sync::oneshot;
use ulid::Ulid;

impl Worker {
    pub(super) fn start_browser(
        &mut self,
        seat: Ulid,
        generation: u64,
        call: Call,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        let result = self.control_actor(seat, generation);
        match result {
            Ok((actor, record)) if record.lifecycle == crate::registry::Lifecycle::Open => {
                #[cfg(all(feature = "browser", target_os = "macos"))]
                self.browser_call(actor, call, reply);
                #[cfg(not(all(feature = "browser", target_os = "macos")))]
                {
                    let _ = actor;
                    let _ = reply.send(Ok(mcp::error(
                        call.op_id, "browser_unavailable",
                        "Chromium requires a macOS bundle built with BROWSER=1 (Cargo feature browser)",
                    )));
                }
            }
            _ => {
                let _ = reply.send(Ok(mcp::error(
                    call.op_id,
                    "browser_forbidden",
                    "seat is closed or its process generation changed",
                )));
            }
        }
    }

    pub(super) fn retire_browser(&mut self, seat: Ulid, generation: u64) {
        #[cfg(all(feature = "browser", target_os = "macos"))]
        self.browsers.retire(seat, generation);
        #[cfg(not(all(feature = "browser", target_os = "macos")))]
        let _ = (seat, generation);
    }
}

#[cfg(all(feature = "browser", target_os = "macos"))]
impl Worker {
    fn browser_call(
        &mut self,
        actor: mcp::SeatActor,
        call: Call,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        let action = match session::Action::parse(&call) {
            Ok(action) => action,
            Err(error) => {
                let _ = reply.send(Ok(mcp::error(call.op_id, "invalid_arguments", &error)));
                return;
            }
        };
        let binding = if matches!(action, session::Action::Screenshot(_)) {
            match self.seat_workspace(&actor) {
                Ok(port) => {
                    let retained = self.control_journal(actor.seat_id).and_then(|control| {
                        self.root_journal(&control, &port)
                            .retained(&mcp::Actor::from(&actor), &call)
                    });
                    match retained {
                        Ok(Some(value)) if value["structuredContent"]["status"] != "unknown" => {
                            let _ = reply.send(Ok(value));
                            return;
                        }
                        Err(error) => {
                            let _ =
                                reply.send(Ok(mcp::artifacts::failure(call.op_id, error.as_ref())));
                            return;
                        }
                        _ => Some(port.binding),
                    }
                }
                Err(error) => {
                    let _ = reply.send(Ok(mcp::error(
                        call.op_id,
                        "browser_artifact_unavailable",
                        &error.to_string(),
                    )));
                    return;
                }
            }
        } else {
            None
        };
        self.browsers.call(
            &self.handle,
            &self.client,
            &self.config.run_dir,
            session::Submission {
                actor,
                call,
                action,
                binding,
                reply,
            },
        );
    }
}
