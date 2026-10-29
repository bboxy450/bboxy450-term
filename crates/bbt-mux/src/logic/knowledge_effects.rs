// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two bounded IO phases, separated by owner-side authority/signing preparation.
// Adapted from bboxy450-term, crates/bbt-mux/src/logic.rs:
// https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/src/logic.rs
// Changes: capture source/actor state and move provider effects off the owner loop.
use super::*;
use crate::{
    knowledge::{
        data_source as d,
        effects::{self, Prepared},
    },
    mcp::{self, Actor, Call, journal::Journal},
};
use serde_json::Value;

pub(super) enum EffectReply {
    SeatControl {
        seat: Ulid,
        generation: u64,
        reply: oneshot::Sender<Result<()>>,
    },
    Seat(oneshot::Sender<Result<Value>>),
    Knowledge(oneshot::Sender<d::Result<Value>>),
    Task(oneshot::Sender<Result<bbt_proto::messaging::TaskResponse>>),
}
impl EffectReply {
    fn failure(self, call: &Call, error: crate::store::Error) {
        if error
            .downcast_ref::<crate::knowledge::workspaces::Error>()
            .is_some()
        {
            let code = workspace_command_error(error.as_ref()).code;
            self.send(call, Ok(mcp::error(call.op_id, &code, &error.to_string())));
        } else {
            self.send(call, Err(error));
        }
    }
    fn send(self, call: &Call, result: Result<Value>) {
        match self {
            Self::SeatControl { reply, .. } => {
                let _ = reply.send(result.and_then(|value| {
                    match mcp::work_items::response(&call.tool, value).map_err(invalid)? {
                        bbt_proto::messaging::TaskResponse::Applied => Ok(()),
                        _ => Err(invalid("unexpected task assignment response")),
                    }
                }));
            }
            Self::Seat(reply) => {
                let _ = reply.send(result);
            }
            Self::Knowledge(reply) => {
                let value = result
                    .map_err(|e| d::CommandError::new("unknown", e.to_string()))
                    .and_then(|value| {
                        if value["isError"] == true {
                            let error = &value["structuredContent"]["error"];
                            Err(d::CommandError::new(
                                error["code"].as_str().unwrap_or("unknown"),
                                error["message"]
                                    .as_str()
                                    .unwrap_or("document effect unavailable"),
                            ))
                        } else {
                            Ok(value["structuredContent"].clone())
                        }
                    });
                let _ = reply.send(value);
            }
            Self::Task(reply) => {
                let _ = reply.send(result.and_then(|value| {
                    mcp::work_items::response(&call.tool, value).map_err(invalid)
                }));
            }
        }
    }
}
pub(super) struct Context {
    port: WorkspacePort,
    revision: u64,
    actor: Actor,
    call: Call,
    journal: Journal,
    reply: EffectReply,
}
pub(super) enum Completion {
    Prepared(Context, d::Result<Prepared>),
    Applied(
        Context,
        Box<Prepared>,
        Option<Box<crate::messaging::keys::OperatorSigning>>,
        Vec<Ulid>,
        effects::Applied,
    ),
}
fn io<T>(job: impl FnOnce() -> d::Result<T>) -> d::Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).unwrap_or_else(|_| {
        Err(d::CommandError::new(
            "unknown",
            "document IO interrupted; inspect the original operation",
        ))
    })
}
impl Worker {
    pub(super) fn start_seat_effect(
        &mut self,
        id: Ulid,
        generation: u64,
        call: Call,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        let prepared = (|| -> Result<_> {
            let (actor, _) = self.control_actor(id, generation)?;
            let port = self.seat_workspace(&actor)?;
            let journal = self.control_journal(id)?;
            let journal = self.root_journal(&journal, &port);
            Ok((Actor::from(&actor), port, journal))
        })();
        self.start_document_effect(prepared, call, EffectReply::Seat(reply));
    }
    pub(super) fn start_operator_effect(
        &mut self,
        scope: Option<bbt_proto::gui::SwarmScope>,
        origin: mcp::Origin,
        call: Call,
        reply: EffectReply,
    ) {
        let prepared = (|| -> Result<_> {
            let port = self.gui_workspace(scope)?;
            let origin = self.identity.bind_origin(origin);
            let device_id = self.operator_device_for_effect()?;
            let actor = Actor::Operator {
                device_id,
                door: origin.door,
                issued_by: origin.issued_by,
            };
            let journal = self.root_journal(&self.operator_journal, &port);
            Ok((actor, port, journal))
        })();
        self.start_document_effect(prepared, call, reply);
    }
    fn start_document_effect(
        &mut self,
        prepared: Result<(Actor, WorkspacePort, Journal)>,
        call: Call,
        reply: EffectReply,
    ) {
        let (actor, port, journal) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                reply.failure(&call, error);
                return;
            }
        };
        let admitted = (|| -> Result<_> {
            call.validate()?;
            // A replay must not be redispatched or hidden behind a capacity refusal.
            if let Some(value) = journal.retained(&actor, &call)? {
                return Ok(Err(value));
            }
            self.workspaces.write_ready(&self.registry, &port.binding)?;
            let port = self.scan_port(&port.binding)?;
            if self.effect_roots.contains(&port.binding.root_id) || self.effect_roots.len() >= 8 {
                return Ok(Err(mcp::error(
                    call.op_id,
                    "provider_busy",
                    "A document effect is already admitted for this root, or effect capacity is occupied; no effect admitted.",
                )));
            }
            let revision = port.host.revision()?;
            let (value, fresh) = journal.admit(&actor, &call)?;
            if !fresh {
                return Ok(Err(value));
            }
            Ok(Ok((port, revision)))
        })();
        let (port, revision) = match admitted {
            Ok(Ok(admitted)) => admitted,
            Ok(Err(value)) => {
                reply.send(&call, Ok(value));
                return;
            }
            Err(error) => {
                reply.failure(&call, error);
                return;
            }
        };
        self.effect_roots.insert(port.binding.root_id.clone());
        let context = Context {
            port,
            revision,
            actor,
            call,
            journal,
            reply,
        };
        let registry = self.registry.clone();
        self.effect_jobs.spawn_blocking_on(
            move || {
                let result = io(|| {
                    let source = context.port.host.source_at(
                        &context.port.index,
                        &registry,
                        context.revision,
                    );
                    effects::prepare(
                        &source,
                        &registry,
                        context.port.binding.swarm,
                        &context.actor,
                        &context.call,
                    )
                });
                Completion::Prepared(context, result)
            },
            &self.handle,
        );
    }
    fn effect_current(&mut self, context: &Context) -> Result<()> {
        if let EffectReply::SeatControl {
            seat, generation, ..
        } = &context.reply
        {
            let record = self.controlled_seat(*seat, *generation)?;
            if record.swarm != context.port.binding.swarm {
                return Err(invalid("task target moved to another swarm"));
            }
        }
        self.workspaces
            .current(&self.registry, &context.port.binding)?;
        context
            .port
            .host
            .current(context.revision)
            .map_err(|e| invalid(e.to_string()))?;
        match &context.actor {
            Actor::Seat(old) => {
                let (current, _) = self.control_actor(old.seat_id, old.process_generation)?;
                if current != *old {
                    return Err(invalid("seat authority changed during document effect"));
                }
            }
            Actor::Operator { device_id, .. } => {
                if self.operator_device_for_effect()? != *device_id {
                    return Err(invalid("operator device changed during document effect"));
                }
            }
            Actor::OperatorLocal { .. } => {
                return Err(invalid("document signing requires custody"));
            }
        }
        Ok(())
    }
    fn finish_document_effect(&mut self, context: Context, value: Value) {
        self.effect_roots.remove(&context.port.binding.root_id);
        let value = context
            .journal
            .complete(&context.actor, &context.call, value);
        context.reply.send(&context.call, value);
    }
    pub(super) fn poll_document_effects(&mut self) {
        while let Some(joined) = self.effect_jobs.try_join_next() {
            let completion = match joined {
                Ok(done) => done,
                Err(error) => {
                    eprintln!(
                        "document effect task interrupted: {error}; admitted operation remains unknown"
                    );
                    continue;
                }
            };
            match completion {
                Completion::Prepared(context, prepared) => {
                    let prepared = prepared.and_then(|prepared| {
                        self.effect_current(&context).map_err(|e| {
                            d::CommandError::new("swarm_root_changed", e.to_string())
                        })?;
                        Ok(prepared)
                    });
                    let mut prepared = match prepared {
                        Ok(prepared) => prepared,
                        Err(error) => {
                            let value = mcp::error(context.call.op_id, &error.code, &error.message);
                            self.finish_document_effect(context, value);
                            continue;
                        }
                    };
                    let ready = self.prepare_document_authority(&context, &mut prepared);
                    let (key, operator, wake_swarms) = match ready {
                        Ok(ready) => ready,
                        Err(error) => {
                            let value = mcp::error(context.call.op_id, "held", &error.to_string());
                            self.finish_document_effect(context, value);
                            continue;
                        }
                    };
                    let registry = self.registry.clone();
                    self.effect_jobs.spawn_blocking_on(
                        move || {
                            let result = io(|| {
                                let author = match &context.actor {
                                    Actor::Seat(actor) => crate::provenance::author::Author::seat(
                                        actor.seat_id,
                                        Some(actor.process_generation),
                                        &key,
                                    ),
                                    Actor::Operator { device_id, .. } => {
                                        crate::provenance::author::Author::operator(
                                            *device_id,
                                            operator
                                                .as_ref()
                                                .expect("operator authority prepared")
                                                .issuer
                                                .kind,
                                            &key,
                                        )
                                    }
                                    Actor::OperatorLocal { .. } => unreachable!(),
                                };
                                let source = context.port.host.source_at(
                                    &context.port.index,
                                    &registry,
                                    context.revision,
                                );
                                Ok(effects::apply(
                                    &source,
                                    &mut prepared,
                                    &context.call,
                                    &crate::knowledge::WriteContext { author },
                                ))
                            })
                            .unwrap_or_else(|error| {
                                effects::Applied {
                                    value: mcp::error(
                                        context.call.op_id,
                                        "unknown",
                                        &error.message,
                                    ),
                                    statuses: Vec::new(),
                                    applied: true,
                                }
                            });
                            Completion::Applied(
                                context,
                                Box::new(prepared),
                                operator.map(Box::new),
                                wake_swarms,
                                result,
                            )
                        },
                        &self.handle,
                    );
                }
                Completion::Applied(context, mut prepared, operator, wake_swarms, mut result) => {
                    if let Err(error) = self.effect_current(&context) {
                        result.value = mcp::error(
                            context.call.op_id,
                            if result.applied {
                                "unknown"
                            } else {
                                "swarm_root_changed"
                            },
                            &format!(
                                "authority or workspace changed during document effect: {error}; inspect the original operation"
                            ),
                        );
                    } else if result.value["isError"] != true {
                        let finish = (|| -> Result<()> {
                            let swarms = self
                                .workspaces
                                .bound_swarms(&self.registry, &context.port.binding.root_id)?;
                            for (task, status) in result.statuses {
                                self.messaging.task_state_root(
                                    &context.port.binding.root_id,
                                    &swarms,
                                    task,
                                    &status,
                                    now_ms(),
                                )?;
                            }
                            if let Prepared::Task { plan, .. } = prepared.as_mut() {
                                self.finish_effect_wakes(
                                    &context,
                                    std::mem::take(&mut plan.wakes),
                                    operator.as_deref(),
                                    wake_swarms,
                                )?;
                            }
                            self.workspaces.reconcile(&context.port.binding.root_id)?;
                            Ok(())
                        })();
                        if let Err(error) = finish {
                            let effect = if matches!(prepared.as_ref(), Prepared::Task { .. }) {
                                "work-item frontmatter written"
                            } else {
                                "document effect applied"
                            };
                            result.value = mcp::error(
                                context.call.op_id,
                                "unknown",
                                &format!(
                                    "{effect}; host publication unavailable: {error}; inspect the original operation"
                                ),
                            );
                        }
                    }
                    self.finish_document_effect(context, result.value);
                }
            }
        }
    }
    fn prepare_document_authority(
        &mut self,
        context: &Context,
        prepared: &mut Prepared,
    ) -> Result<(
        SigningKey,
        Option<crate::messaging::keys::OperatorSigning>,
        Vec<Ulid>,
    )> {
        let keys = self.identity.keys();
        if let Some(reason) = keys.signing_hold() {
            return Err(invalid(format!("document signing held: {reason}")));
        }
        let operator = match &context.actor {
            Actor::Operator {
                device_id,
                door,
                issued_by,
            } => {
                let mut authority = self.identity.operator_signing()?;
                if authority.device_id != *device_id {
                    return Err(invalid("operator device changed"));
                }
                authority.origin = mcp::Origin {
                    door: *door,
                    issued_by: issued_by.clone(),
                };
                Some(authority)
            }
            _ => None,
        };
        let key = match context.actor.seat() {
            Some(actor) if keys.ready_issuer().is_some() => {
                keys.leaf(actor.seat_id, actor.process_generation)?
            }
            Some(_) => return Err(invalid("document identity pending")),
            None => operator
                .as_ref()
                .ok_or_else(|| invalid("operator signing requires custody"))?
                .key
                .clone(),
        };
        let mut wake_swarms = Vec::new();
        if let Prepared::Task { plan, .. } = prepared {
            let mut task_swarm = None;
            for update in &mut plan.writes {
                if matches!(
                    update.metadata["type"].as_str(),
                    Some("task" | "bug" | "epic")
                ) {
                    let swarm = update.metadata["assignee"]
                        .as_str()
                        .and_then(|id| id.strip_prefix("seat:").unwrap_or(id).parse::<Ulid>().ok())
                        .and_then(|id| self.registry.get(id).ok())
                        .map(|seat| seat.swarm)
                        .unwrap_or(context.port.binding.swarm);
                    if !swarm.is_nil() {
                        let target = self.workspaces.select(&self.registry, swarm, None)?;
                        if target.binding.root_id != context.port.binding.root_id {
                            return Err(invalid(
                                "task_workspace_mismatch: task assignee must use the captured workspace",
                            ));
                        }
                        update.metadata["swarm"] = serde_json::json!(swarm);
                        task_swarm = Some(swarm);
                    }
                }
            }
            for wake in &plan.wakes {
                let swarm = match wake.to {
                    bbt_proto::messaging::Recipient::Seat(seat) => self.registry.get(seat)?.swarm,
                    bbt_proto::messaging::Recipient::Operator(device) => {
                        if self.identity.status()?.device_id != Some(device) {
                            return Err(invalid("task delegator operator unavailable"));
                        }
                        self.messaging.set_operator_device(device)?;
                        task_swarm.ok_or_else(|| invalid("task swarm requires an assignee"))?
                    }
                    bbt_proto::messaging::Recipient::OperatorLocal(_) => {
                        task_swarm.ok_or_else(|| invalid("task swarm requires an assignee"))?
                    }
                };
                wake_swarms.push(swarm);
            }
        }
        Ok((key, operator, wake_swarms))
    }
    fn finish_effect_wakes(
        &mut self,
        context: &Context,
        wakes: Vec<crate::mcp::work_items::Wake>,
        operator: Option<&crate::messaging::keys::OperatorSigning>,
        wake_swarms: Vec<Ulid>,
    ) -> Result<()> {
        for (wake, swarm) in wakes.into_iter().zip(wake_swarms) {
            let summary = format!(
                "{}\nTask workspace: {} (knowledge root {}).",
                wake.summary,
                context.port.index.knowledge_root().display(),
                context.port.binding.root_id
            );
            let destination = match wake.to {
                bbt_proto::messaging::Recipient::Seat(seat) => format!("seats/{seat}/inbox"),
                _ => "operator/inbox".into(),
            };
            let mut request =
                crate::messaging::wire::SendRequest::text(destination, summary.clone());
            request.id = wake.id;
            request.kind = crate::messaging::wire::MessageKind::Task;
            request.attachments = vec![format!("[[{}]]", wake.task_id)];
            let body = crate::messaging::wire::Body::Task {
                task_id: wake.task_id,
                event: wake.event,
                path: wake.path,
                summary,
            };
            self.messaging
                .task_root(swarm, request.id, &context.port.binding.root_id)?;
            match &context.actor {
                Actor::Seat(actor) => {
                    self.messaging.task_event(
                        actor.seat_id,
                        actor.process_generation,
                        request,
                        body,
                        now_ms(),
                    )?;
                }
                Actor::Operator { .. } => {
                    self.messaging.task_event_operator(
                        operator.ok_or_else(|| invalid("operator authority unavailable"))?,
                        swarm,
                        request,
                        body,
                        now_ms(),
                    )?;
                }
                Actor::OperatorLocal { .. } => {
                    return Err(invalid("task signing requires custody"));
                }
            }
        }
        Ok(())
    }
}
