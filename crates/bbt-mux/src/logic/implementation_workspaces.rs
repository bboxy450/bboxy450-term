// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Provider jobs never run Git or wait for a process on the logic owner loop.
use super::*;
use crate::mcp::{
    self,
    workspaces::{self as managed, Begun, Record, Service, Work},
};
use serde_json::Value;

pub(super) struct Context {
    actor: mcp::SeatActor,
    port: WorkspacePort,
    revision: u64,
    journal: mcp::journal::Journal,
    call: mcp::Call,
    reply: Option<oneshot::Sender<Result<Value>>>,
}
pub(super) enum Completion {
    Called(Box<Context>, Result<Box<Begun>>),
    Provider(Box<Context>, Box<Work>, Result<Box<Record>>),
    Spawn(Box<Context>, Result<()>),
}
impl Worker {
    fn implementation_context(
        &mut self,
        id: Ulid,
        generation: u64,
        call: &mcp::Call,
    ) -> Result<(mcp::SeatActor, WorkspacePort, u64, mcp::journal::Journal)> {
        call.validate()?;
        let (actor, _) = self.control_actor(id, generation)?;
        let port = self.seat_workspace(&actor)?;
        let revision = port.host.revision()?;
        let journal = self.control_journal(id)?;
        let journal = self.root_journal(&journal, &port);
        Ok((actor, port, revision, journal))
    }
    fn implementation_current(&mut self, c: &Context) -> bool {
        self.control_actor(c.actor.seat_id, c.actor.process_generation)
            .is_ok_and(|(a, _)| a == c.actor)
            && self
                .workspaces
                .current(&self.registry, &c.port.binding)
                .is_ok()
            && c.port.host.current(c.revision).is_ok()
    }
    pub(super) fn start_workspace_call(
        &mut self,
        id: Ulid,
        generation: u64,
        call: mcp::Call,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        // Queue admission is bounded before any journal/provider reservation.
        if self.implementation_jobs.len() >= 8 {
            let _ = reply.send(Ok(mcp::error_retry(
                call.op_id,
                "provider_busy",
                "workspace IO capacity busy; request was not admitted",
                Some(100),
            )));
            return;
        }
        let prepared = self.implementation_context(id, generation, &call);
        let (actor, port, revision, journal) = match prepared {
            Ok(v) => v,
            Err(e) => {
                let _ = reply.send(Ok(managed::failure(call.op_id, e.as_ref())));
                return;
            }
        };
        if managed::mutation(&call.tool) {
            match journal.admit(&actor, &call) {
                Ok((value, false)) => {
                    let _ = reply.send(Ok(value));
                    return;
                }
                Ok((_, true)) => {}
                Err(e) => {
                    let _ = reply.send(Err(e));
                    return;
                }
            }
        }
        let context = Context {
            actor,
            port,
            revision,
            journal,
            call,
            reply: Some(reply),
        };
        let service = self.implementation_workspaces.clone();
        let registry = self.registry.clone();
        self.implementation_jobs.spawn_blocking_on(
            move || {
                let repository = context
                    .port
                    .index
                    .knowledge_root()
                    .parent()
                    .ok_or_else(|| invalid("workspace_repository_unavailable"));
                let result = repository.and_then(|repository| {
                    if context.call.tool == "workspace_prepare" {
                        context
                            .port
                            .host
                            .with_source(&context.port.index, &registry, |source| {
                                service
                                    .begin(
                                        &context.actor,
                                        &context.call,
                                        repository,
                                        &context.port.binding,
                                        context.revision,
                                        source,
                                        &registry,
                                    )
                                    .map_err(|e| {
                                        crate::knowledge::data_source::CommandError::new(
                                            "workspace_unavailable",
                                            e.to_string(),
                                        )
                                    })
                            })
                            .map_err(|e| invalid(e.to_string()))
                    } else {
                        let source = BboxySource::new(&context.port.index);
                        service.begin(
                            &context.actor,
                            &context.call,
                            repository,
                            &context.port.binding,
                            context.revision,
                            &source,
                            &registry,
                        )
                    }
                });
                Completion::Called(Box::new(context), result.map(Box::new))
            },
            &self.handle,
        );
    }
    pub(super) fn start_workspace_spawn(
        &mut self,
        id: Ulid,
        generation: u64,
        call: mcp::Call,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        if self.implementation_jobs.len() >= 8 {
            let _ = reply.send(Ok(mcp::error_retry(
                call.op_id,
                "provider_busy",
                "workspace IO capacity busy; launch was not admitted",
                Some(100),
            )));
            return;
        }
        let (actor, port, revision, journal) =
            match self.implementation_context(id, generation, &call) {
                Ok(v) => v,
                Err(e) => {
                    let _ = reply.send(Ok(managed::failure(call.op_id, e.as_ref())));
                    return;
                }
            };
        // Replay precedes current readiness/revision checks: a successful launch
        // consumed that ready revision and must still return its original seat.
        match journal.admit(&actor, &call) {
            Ok((value, false)) => {
                let _ = reply.send(Ok(value));
                return;
            }
            Ok((_, true)) => {}
            Err(e) => {
                let _ = reply.send(Err(e));
                return;
            }
        }
        let prepared = (|| -> Result<Record> {
            let (_, caller) = self.control_actor(id, generation)?;
            self.prepare_seat_control(&actor, &caller, &call)
                .map_err(invalid)?;
            let reference: managed::Reference =
                serde_json::from_value(call.args["workspace"].clone())?;
            self.implementation_workspaces
                .resolve(&reference, &actor, &port.binding, revision)
        })();
        let workspace = match prepared {
            Ok(r) => r,
            Err(e) => {
                let value = managed::failure(call.op_id, e.as_ref());
                let _ = reply.send(journal.complete(&actor, &call, value));
                return;
            }
        };
        let context = Context {
            actor,
            port,
            revision,
            journal,
            call,
            reply: Some(reply),
        };
        let registry = self.registry.clone();
        self.implementation_jobs.spawn_blocking_on(
            move || {
                let result = context
                    .port
                    .host
                    .with_source(&context.port.index, &registry, |source| {
                        Service::verify_ready(&workspace, source).map_err(|e| {
                            crate::knowledge::data_source::CommandError::new(
                                "workspace_unavailable",
                                e.to_string(),
                            )
                        })
                    })
                    .map_err(|e| invalid(e.to_string()));
                Completion::Spawn(Box::new(context), result)
            },
            &self.handle,
        );
    }
    pub(super) fn poll_implementation_workspaces(&mut self) {
        while let Some(joined) = self.implementation_jobs.try_join_next() {
            let Ok(completion) = joined else {
                eprintln!("workspace IO job stopped; retained operation remains unknown");
                continue;
            };
            match completion {
                Completion::Called(mut context, result) => {
                    let current = self.implementation_current(&context);
                    let (value, work) = match result {
                        Ok(begun) if current => {
                            (mcp::success(context.call.op_id, begun.value), begun.work)
                        }
                        Ok(begun) => {
                            if let Some(work) = begun.work {
                                let _ = self.implementation_workspaces.finish(
                                    work.record(),
                                    Err(invalid(
                                        "workspace_binding_changed: provider effect not started",
                                    )),
                                    false,
                                );
                            }
                            (
                                mcp::error(
                                    context.call.op_id,
                                    "workspace_binding_changed",
                                    "authority or canonical KB/provider changed before workspace admission",
                                ),
                                None,
                            )
                        }
                        Err(e) => (managed::failure(context.call.op_id, e.as_ref()), None),
                    };
                    let result = if managed::mutation(&context.call.tool) {
                        context
                            .journal
                            .complete(&context.actor, &context.call, value)
                    } else {
                        context
                            .journal
                            .checked(&context.actor, &context.call, value)
                    };
                    if let Some(reply) = context.reply.take() {
                        let _ = reply.send(result);
                    }
                    if let Some(work) = work {
                        // The response above confirms the durable preparation,
                        // not readiness. The same operation receives its final
                        // outcome; workspace_get is the live state projection.
                        let record = work.record();
                        if context.call.tool == "workspace_get" {
                            if let Ok(journal) = self.control_journal(record.actor.seat_id) {
                                context.journal = self.root_journal(&journal, &context.port);
                            }
                            context.call = record.request.clone();
                        }
                        let service = self.implementation_workspaces.clone();
                        self.implementation_jobs.spawn_blocking_on(
                            move || {
                                let result = service.run(&work);
                                Completion::Provider(context, Box::new(work), result.map(Box::new))
                            },
                            &self.handle,
                        );
                    }
                }
                Completion::Provider(context, work, result) => {
                    let current = self.implementation_current(&context);
                    let result = self.implementation_workspaces.finish(
                        work.record(),
                        result.map(|record| *record),
                        current,
                    );
                    let value = match result {
                        Ok(r) => mcp::success(
                            context.call.op_id,
                            serde_json::json!({"workspace":r.view()}),
                        ),
                        Err(e) => managed::failure(context.call.op_id, e.as_ref()),
                    };
                    // A recovered GET may finish an older generation's original
                    // operation, but cannot start another external effect.
                    if let Err(e) =
                        context
                            .journal
                            .complete(&work.record().actor, &context.call, value)
                    {
                        eprintln!("workspace outcome retained but journal completion failed: {e}");
                    }
                }
                Completion::Spawn(mut context, result) => {
                    let value = match result {
                        Ok(()) if self.implementation_current(&context) => {
                            match self
                                .control_actor(
                                    context.actor.seat_id,
                                    context.actor.process_generation,
                                )
                                .and_then(|(_, caller)| {
                                    self.prepare_seat_control(
                                        &context.actor,
                                        &caller,
                                        &context.call,
                                    )
                                    .map_err(|e| invalid(e.to_string()))
                                }) {
                                Ok(request) => {
                                    self.effect_seat_control(&context.actor, &context.call, request)
                                }
                                Err(e) => managed::failure(context.call.op_id, e.as_ref()),
                            }
                        }
                        Ok(()) => mcp::error(
                            context.call.op_id,
                            "workspace_binding_changed",
                            "authority or KB binding changed during launch checks",
                        ),
                        Err(e) => managed::failure(context.call.op_id, e.as_ref()),
                    };
                    let result = context
                        .journal
                        .complete(&context.actor, &context.call, value);
                    if let Some(reply) = context.reply.take() {
                        let _ = reply.send(result);
                    }
                }
            }
        }
    }
}
