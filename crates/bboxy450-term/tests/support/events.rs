// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! State-only compatibility for the lifecycle fixtures. The production IO and
//! dedicated lane tests prove publish-before-ACK; this adapter does not ACK or
//! mark anything read and leaves the originals pending in the private fixture.
use bboxy450_term::cockpit::Session;
use bbt_mux::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::{GuiAction, GuiReply, events::GuiEventPayload};

pub trait StateEvents {
    fn receive_state(&mut self) -> impl std::future::Future<Output = Result<GuiReply>> + Send;
}
impl StateEvents for Session {
    fn receive_state(&mut self) -> impl std::future::Future<Output = Result<GuiReply>> + Send {
        receive(self)
    }
}

pub async fn attach(session: &mut Session) -> Result<()> {
    let id = Ulid::new();
    session.send(GuiAction::AttachEvents { id }).await?;
    loop {
        match session.receive().await? {
            GuiReply::Ok { op } if op == id => return Ok(()),
            GuiReply::Error { op, reason } if op == id => {
                return if reason.starts_with("GuiEventsHeld:") {
                    Ok(())
                } else {
                    Err(invalid(reason))
                };
            }
            _ => (),
        }
    }
}

pub async fn receive(session: &mut Session) -> Result<GuiReply> {
    loop {
        match session.receive().await? {
            GuiReply::Event { envelope, .. } => match envelope.payload {
                GuiEventPayload::Seat { seat } => return Ok(GuiReply::Seat(*seat)),
                GuiEventPayload::Identity { .. } => (),
                GuiEventPayload::Host { host, .. } => return Ok(GuiReply::HostState { host }),
                _ => (),
            },
            GuiReply::Identity(_) | GuiReply::EventStatus { .. } | GuiReply::Gap { .. } => (),
            reply => return Ok(reply),
        }
    }
}
