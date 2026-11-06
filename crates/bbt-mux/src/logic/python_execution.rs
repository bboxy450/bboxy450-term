// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Runtime preflight is advice; guarded dispatch compares on the IO owner turn.
use super::*;
use serde_json::{Value, json};
use std::cell::RefCell;

thread_local! {static EXPECTED:RefCell<Option<Value>>=const {RefCell::new(None)};}
pub(super) struct ContextScope(Option<Value>);
impl Drop for ContextScope {
    fn drop(&mut self) {
        EXPECTED.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}
pub(super) fn scope(expected: Option<Value>) -> ContextScope {
    // Worker admission/port capture is synchronous. Never move an await or
    // provider job into this scope: async work retains its explicit port/actor.
    ContextScope(EXPECTED.with(|slot| slot.replace(expected)))
}

impl Worker {
    pub(super) fn python_execution_context(&mut self, id: Ulid, generation: u64) -> Result<Value> {
        let (actor, _) = self.control_actor(id, generation)?;
        let record = self.registry.get(id)?;
        let port = self.seat_workspace(&actor)?;
        self.python_context_for_port(&actor, &record, &port)
    }
    fn python_context_for_port(
        &self,
        actor: &crate::mcp::SeatActor,
        record: &SeatRecord,
        port: &WorkspacePort,
    ) -> Result<Value> {
        // Ordinary status/cleanup authority can outlive window close grace.
        // Python admits new work only while open, and retains the monotonic
        // revision so close -> undo cannot revive an old execution context.
        if record.lifecycle != Lifecycle::Open {
            return Err(invalid(
                "execution_context_changed: closed seat admits no new Python work",
            ));
        }
        let revision = port.host.revision()?;
        let workspace = self.implementation_workspaces.execution_context(
            actor,
            &port.binding,
            revision,
            None,
        )?;
        let target = match &record.location {
            crate::registry::Location::Local => json!({"kind":"local"}),
            crate::registry::Location::Ssh { host } => json!({"kind":"ssh","host":host}),
            crate::registry::Location::Cloud { provider, host } => {
                json!({"kind":"cloud","provider":provider,"host":host})
            }
        };
        Ok(
            json!({"guard_version":1,"seat_id":actor.seat_id,"process_generation":actor.process_generation,
            "swarm_id":actor.swarm_id,"role":actor.role,"grant_epoch":actor.grant_epoch,"issuer_kind":actor.issuer_kind,"task_ids":record.tasks,"lifecycle_revision":record.lifecycle_revision,
            "knowledge_binding":port.binding,"provider_revision":revision,
            "workspace":workspace.as_ref().map(|w|w.view()),"cwd":workspace.as_ref().map(|w|&w.cwd),"target":target}),
        )
    }
    pub(super) fn guard_python_port(
        &self,
        actor: &crate::mcp::SeatActor,
        port: &WorkspacePort,
    ) -> Result<()> {
        let Some(expected) = EXPECTED.with(|slot| slot.borrow().clone()) else {
            return Ok(());
        };
        let record = self.registry.get(actor.seat_id)?;
        let current = self.python_context_for_port(actor, &record, port)?;
        if !crate::mcp::python_context_matches(&expected, &current) {
            return Err(invalid(
                "execution_context_changed: actual provider/source capture differs from this runtime's admitted context",
            ));
        }
        Ok(())
    }
    pub(super) fn validate_python_context(
        &mut self,
        id: Ulid,
        generation: u64,
        expected: &Value,
    ) -> Result<()> {
        if serde_json::to_vec(expected)?.len() > 64 * 1024 {
            return Err(invalid(
                "execution_context_changed: oversized context expectation",
            ));
        }
        let current = self.python_execution_context(id, generation)?;
        if !crate::mcp::python_context_matches(expected, &current) {
            return Err(invalid(
                "execution_context_changed: captured seat/grant/workspace/KB/provider no longer matches; no new effect admitted",
            ));
        }
        Ok(())
    }
}
