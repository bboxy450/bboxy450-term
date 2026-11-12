// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bulk task import owns bounded per-root IO capacity, never the logic loop.
use super::*;
use crate::mcp::{Actor, Call};
use crate::registry::Class;
use std::os::unix::fs::MetadataExt;

/// Captured only by the trusted logic owner, never from request fields.
#[derive(Clone)]
pub(super) struct Authority {
    actor: Actor,
    custodian: crate::mcp::SeatActor,
    issuer: crate::messaging::wire::Issuer,
    public: [u8; 32],
    operator_grant: Option<(String, u64)>,
}

impl Worker {
    pub(super) fn validate_task_import_authority(&mut self, authority: &Authority) -> Result<()> {
        let (custodian, _) = self.control_actor(
            authority.custodian.seat_id,
            authority.custodian.process_generation,
        )?;
        if custodian != authority.custodian {
            return Err(invalid("import custodian grant changed before publication"));
        }
        if self.identity.reopening_failed() {
            return Err(invalid("import signing custody held before publication"));
        }
        let keys = self.identity.keys();
        if let Some(reason) = keys.signing_hold() {
            return Err(invalid(format!("import signing custody held: {reason}")));
        }
        if keys.ready_issuer().as_ref() != Some(&authority.issuer) {
            return Err(invalid("import signing issuer changed before publication"));
        }
        match &authority.actor {
            Actor::Seat(seat) => {
                let current = if seat.seat_id == custodian.seat_id {
                    custodian
                } else {
                    self.control_actor(seat.seat_id, seat.process_generation)?.0
                };
                if current != *seat
                    || keys
                        .leaf(seat.seat_id, seat.process_generation)?
                        .verifying_key()
                        .to_bytes()
                        != authority.public
                {
                    return Err(invalid(
                        "import caller grant or signer changed before publication",
                    ));
                }
            }
            Actor::Operator { device_id, .. } => {
                let current = self.identity.operator_signing()?;
                if current.device_id != *device_id
                    || current.issuer != authority.issuer
                    || current.key.verifying_key().to_bytes() != authority.public
                    || authority.operator_grant.as_ref() != Some(&(current.grant, current.epoch))
                {
                    return Err(invalid("import operator grant changed before publication"));
                }
            }
            _ => return Err(invalid("import requires admitted signer")),
        }
        Ok(())
    }

    pub(super) fn start_seat_task_import(
        &mut self,
        id: Ulid,
        generation: u64,
        call: Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        let prepared = (|| -> Result<_> {
            let (actor, _) = self.control_actor(id, generation)?;
            let port = self.seat_workspace(&actor)?;
            let journal = self.control_journal(id)?;
            let journal = self.root_journal(&journal, &port);
            Ok((Actor::Seat(actor), port, journal))
        })();
        match prepared {
            Ok((actor, port, journal)) => {
                self.start_task_import(actor, port, call, journal, move |result| {
                    let _ = reply.send(result);
                })
            }
            Err(error) => {
                let _ = reply.send(Ok(crate::mcp::error(
                    call.op_id,
                    workspace_command_error(error.as_ref()).code.as_str(),
                    &error.to_string(),
                )));
            }
        }
    }

    pub(super) fn start_gui_task_import(
        &mut self,
        id: Ulid,
        request: bbt_proto::messaging::TaskRequest,
        scope: Option<bbt_proto::gui::SwarmScope>,
        origin: crate::mcp::Origin,
        reply: oneshot::Sender<Result<bbt_proto::messaging::TaskResponse>>,
    ) {
        let prepared = (|| -> Result<_> {
            let port = self.gui_workspace(scope)?;
            let origin = self.identity.bind_origin(origin);
            let actor = Actor::Operator {
                device_id: self.operator_device_for_effect()?,
                door: origin.door,
                issued_by: origin.issued_by,
            };
            let call = crate::mcp::work_items::call(id, request).map_err(invalid)?;
            let journal = self.root_journal(&self.operator_journal, &port);
            Ok((actor, port, call, journal))
        })();
        match prepared {
            Ok((actor, port, call, journal)) => {
                self.start_task_import(actor, port, call, journal, move |result| {
                    let result = result.and_then(|v| {
                        crate::mcp::work_items::response("task_import", v).map_err(invalid)
                    });
                    let _ = reply.send(result);
                })
            }
            Err(error) => {
                let _ = reply.send(Err(error));
            }
        }
    }

    fn start_task_import(
        &mut self,
        actor: Actor,
        port: WorkspacePort,
        call: Call,
        journal: crate::mcp::journal::Journal,
        reply: impl FnOnce(Result<serde_json::Value>) + Send + 'static,
    ) {
        let prepared = (|| -> Result<_> {
            call.validate()?;
            if actor.class() != Class::Oracle {
                return Err(invalid("task control requires oracle-class authority"));
            }
            self.workspaces.write_ready(&self.registry, &port.binding)?;
            let provider_revision = port.host.revision()?;
            port.host
                .source_at(&port.index, &self.registry, provider_revision)
                .import_root()
                .map_err(|error| invalid(error.to_string()))?;
            let owner: Ulid = serde_json::from_value(call.args["owner_seat"].clone())?;
            let owner_record = self.registry.get(owner)?;
            if owner_record.role.class != Class::Oracle
                || owner_record.lifecycle != crate::registry::Lifecycle::Open
                || actor
                    .seat()
                    .is_some_and(|s| s.swarm_id != owner_record.swarm)
            {
                return Err(invalid("import custodian outside caller authority"));
            }
            let selected = self
                .workspaces
                .select(&self.registry, owner_record.swarm, None)?;
            if selected.binding.root_id != port.binding.root_id {
                return Err(invalid(
                    "import custodian belongs to a different knowledge root",
                ));
            }
            let owner_scope = self.registry.swarm(owner_record.swarm)?;
            let (custodian, _) = self.control_actor(owner, owner_record.generation)?;
            if self.identity.reopening_failed() {
                return Err(invalid("import signing custody held"));
            }
            let keys = self.identity.keys();
            if let Some(reason) = keys.signing_hold() {
                return Err(invalid(format!("work-item signing held: {reason}")));
            }
            let admitted_issuer = keys
                .ready_issuer()
                .ok_or_else(|| invalid("import issuer unavailable"))?;
            let (key, issuer, operator_grant) = match &actor {
                Actor::Seat(seat) if keys.ready_issuer().is_some() => (
                    keys.leaf(seat.seat_id, seat.process_generation)?,
                    seat.issuer_kind,
                    None,
                ),
                Actor::Operator { device_id, .. } => {
                    let authority = self.identity.operator_signing()?;
                    if authority.device_id != *device_id {
                        return Err(invalid("operator device changed"));
                    }
                    (
                        authority.key,
                        authority.issuer.kind,
                        Some((authority.grant, authority.epoch)),
                    )
                }
                _ => return Err(invalid("import requires admitted signing custody")),
            };
            let authority = Authority {
                actor: actor.clone(),
                custodian,
                issuer: admitted_issuer,
                public: key.verifying_key().to_bytes(),
                operator_grant,
            };
            let root = port.index.knowledge_root().to_owned();
            let inode = std::fs::metadata(&root)?;
            let permit = self
                .task_import_capacity
                .entry(port.binding.root_id.clone())
                .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1)))
                .clone()
                .try_acquire_owned()
                .map_err(|_| {
                    invalid(
                        "task_import_busy: capacity occupied for this root; other roots and messages remain available",
                    )
                })?;
            Ok((
                owner,
                owner_record,
                owner_scope,
                key,
                issuer,
                root,
                (inode.dev(), inode.ino()),
                permit,
                authority,
                provider_revision,
            ))
        })();
        let (owner, owner_record, owner_scope, key, issuer, root, inode, permit, authority, provider_revision) =
            match prepared {
                Ok(value) => value,
                Err(error) => {
                    let code = if error
                        .downcast_ref::<crate::knowledge::workspaces::Error>()
                        .is_some()
                    {
                        workspace_command_error(error.as_ref()).code
                    } else if error.to_string().starts_with("task_import_busy:") {
                        "task_import_busy".into()
                    } else {
                        "rejected".into()
                    };
                    reply(Ok(crate::mcp::error(call.op_id, &code, &error.to_string())));
                    return;
                }
            };
        let registry = self.registry.clone();
        let commands = self.client.tx.clone();
        #[cfg(test)]
        let hook = self.task_import_hook.clone();
        self.task_import_jobs.spawn_blocking_on(move || {
            let _permit=permit;
            let result=(|| -> Result<_> {
                let effect = crate::mcp::work_items::prepare(port.index.as_ref(),&registry,&actor,&call).map_err(invalid)?;
                let crate::mcp::work_items::Prepared::Effect(effect)=effect else {return Err(invalid("import is not a mutation"));};
                let crate::mcp::work_items::Effect::Import{bundle,owner:prepared_owner}=*effect else {return Err(invalid("not a task import"));};
                if prepared_owner!=owner {return Err(invalid("import custodian changed"));}
                let author=match &actor {
                    Actor::Seat(seat)=>crate::provenance::author::Author::seat(seat.seat_id,Some(seat.process_generation),&key),
                    Actor::Operator{device_id,..}=>crate::provenance::author::Author::operator(*device_id,issuer,&key),
                    _=>return Err(invalid("import requires admitted signer")),
                };
                journal.effect(&actor,&call,|| {
                    let result=port.index.import_staged(*bundle,owner,&author,|| {
                        #[cfg(test)]
                        if let Some(hook)=hook {hook();}
                        // Read current authority after all bulk work, off the root
                        // number lock. A full/absent owner fails closed; messages
                        // never wait for this background import or its IO.
                        let (tx,rx)=std::sync::mpsc::sync_channel(1);
                        commands.try_send(Command::TaskImportAuthority(Box::new(authority),tx))
                            .map_err(|_| invalid("import_authority_unavailable: owner busy or unavailable; no publication"))?;
                        rx.recv_timeout(Duration::from_secs(5))
                            .map_err(|_| invalid("import_authority_unavailable: recheck deadline exceeded; no publication"))?
                    },|| {
                        port.host.current(provider_revision).map_err(|error| invalid(format!("import_provider_changed: {error}")))?;
                        let current=std::fs::metadata(&root)?;
                        let current_owner=registry.get(owner)?;
                        if (current.dev(),current.ino())!=inode || registry.swarm(owner_record.swarm)?!=owner_scope
                            || current_owner.role!=owner_record.role || current_owner.generation!=owner_record.generation
                            || current_owner.swarm!=owner_record.swarm || current_owner.lifecycle!=crate::registry::Lifecycle::Open
                        {return Err(invalid("swarm_root_changed: import root or custody changed before publication"));}
                        if let Actor::Seat(seat)=&actor {
                            let current=registry.get(seat.seat_id)?;
                            if current.generation!=seat.process_generation || current.role!=seat.role || current.swarm!=seat.swarm_id || current.lifecycle!=crate::registry::Lifecycle::Open {
                                return Err(invalid("import caller authority changed before publication"));
                            }
                        }
                        Ok(())
                    });
                    match result {Ok(data)=>crate::mcp::success(call.op_id,data),Err(error)=>crate::mcp::error(call.op_id,if error.downcast_ref::<crate::index::IndexBusy>().is_some(){"index_busy"}else if error.downcast_ref::<crate::index::migration::AllocationStale>().is_some(){"import_allocation_stale"}else if error.to_string().starts_with("import_authority_unavailable:"){"import_authority_unavailable"}else{"rejected"},&error.to_string())}
                })
            })();
            reply(match result {Ok(v)=>Ok(v),Err(error)=>Ok(crate::mcp::error(call.op_id,"rejected",&error.to_string()))});
        },&self.handle);
    }
}
