// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Sudo tools run through the operator's own `ctl` paths: `gui_spawn`,
//! `gui_messaging` and the swarm metadata journal. `sudo_as` is a rewrite of the
//! call's seat, so the target's ordinary authority checks decide the rest.
use super::*;
use crate::mcp::sudo::{Request, request};
use serde_json::{Value, json};

pub(super) enum Sudo {
    Done(Value),
    Ctl(Ulid, Vec<String>),
    /// Continue the ordinary dispatch as this seat and generation.
    As(Ulid, u64, crate::mcp::Call),
    /// Reply `value` once these operator swarm actions commit.
    Swarm(Value, Vec<bbt_proto::GuiAction>),
}

fn origin() -> crate::mcp::Origin {
    crate::mcp::Origin {
        door: crate::mcp::Door::Ctl,
        issued_by: None,
    }
}

impl Worker {
    pub(super) fn start_sudo_ctl(
        &self,
        seat: Ulid,
        generation: u64,
        op: Ulid,
        args: Vec<String>,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        let Some(run) = self.ctl_runner else {
            let _ = reply.send(Ok(crate::mcp::error(
                op,
                "unavailable",
                "ctl dispatcher is not installed in this host",
            )));
            return;
        };
        let endpoint = self.gui_info.clone();
        let root = self.config.run_dir.clone();
        self.handle.spawn_blocking(move || {
            let result = match run(
                args,
                endpoint,
                root,
                op,
                format!("sudo:{seat}:{generation}"),
            ) {
                Ok(value) if value.is_object() => {
                    let failed = value["ok"] == false;
                    let mut result = crate::mcp::success(op, value);
                    result["isError"] = json!(failed);
                    result
                }
                Ok(_) => {
                    crate::mcp::error(op, "unknown", "ctl dispatcher returned a non-object reply")
                }
                Err(reason) => crate::mcp::error(op, "unknown", &reason),
            };
            let _ = reply.send(Ok(result));
        });
    }
    pub(super) fn sudo_call(&mut self, id: Ulid, generation: u64, call: crate::mcp::Call) -> Sudo {
        let op = call.op_id;
        let result = (|| -> std::result::Result<Sudo, String> {
            let (actor, _) = self
                .control_actor(id, generation)
                .map_err(|e| e.to_string())?;
            Ok(match request(&actor, &call)? {
                Request::Ctl(args) => Sudo::Ctl(op, args),
                Request::Swarms => Sudo::Done(crate::mcp::success(op, self.sudo_swarms()?)),
                Request::As {
                    seat,
                    generation,
                    call,
                } => {
                    let record = self
                        .registry
                        .get(seat)
                        .map_err(|_| "target seat unavailable")?;
                    if record.lifecycle != Lifecycle::Open || record.generation != generation {
                        return Err(
                            "target seat is closed or its process generation changed".into()
                        );
                    }
                    let Reply::Seats { seats } = self
                        .request_for_seat(seat, Action::ListOwnedSeats)
                        .map_err(|e| e.to_string())?
                    else {
                        return Err("target supervisor ownership unavailable".into());
                    };
                    let Some(owned) = seats.iter().find(|info| info.seat == seat) else {
                        return Ok(Sudo::Done(crate::mcp::error(
                            op,
                            "process_gone",
                            "target process is gone; its open registry record is stale",
                        )));
                    };
                    if owned.generation != generation {
                        return Err("target process generation changed at its supervisor".into());
                    }
                    self.control_actor(seat, generation)
                        .map_err(|e| format!("target control unavailable: {e}"))?;
                    Sudo::As(seat, generation, call)
                }
                Request::Spawn {
                    swarm,
                    swarm_name,
                    profile,
                    cwd,
                    role,
                    name,
                    resume,
                } => {
                    let created = swarm.is_none();
                    let swarm = swarm.unwrap_or_else(Ulid::new);
                    let cwd = match cwd {
                        Some(cwd) => cwd,
                        None => self
                            .registry
                            .swarm(swarm)
                            .ok()
                            .flatten()
                            .and_then(|r| r.workdir)
                            .map(std::path::PathBuf::from)
                            .ok_or("cwd is required: this swarm has no working folder")?,
                    };
                    if !cwd.is_dir() {
                        return Err("spawn cwd is not an available directory".into());
                    }
                    let seat = self
                        .gui_spawn(
                            op,
                            bbt_proto::SpawnSpec {
                                swarm: bbt_proto::SwarmId::from_u128(swarm.0),
                                harness_kind: profile,
                                name,
                                role,
                                cwd: cwd.to_string_lossy().into_owned(),
                                location: bbt_proto::Where::Local,
                                resume,
                                placement: Some(bbt_proto::seat_state::Placement::NewTab {
                                    tab: bbt_proto::TabId::from_u128(Ulid::new().0),
                                }),
                                via: None,
                                command: None,
                                args: vec![],
                                env: vec![],
                            },
                            crate::mcp::Door::Ctl,
                        )
                        .map_err(|e| e.to_string())?;
                    let seat = Ulid(seat.0.0);
                    let generation = self.registry.get(seat).map(|r| r.generation).ok();
                    let value = crate::mcp::success(
                        op,
                        json!({
                            "status":"applied","seat_id":seat,"swarm_id":swarm,"created_swarm":created,
                            "process_generation":generation.map(|g| g.to_string()),
                        }),
                    );
                    let mut actions = vec![];
                    if created {
                        actions.push(bbt_proto::GuiAction::SwarmWorkdir {
                            id: Ulid::new(),
                            swarm,
                            path: Some(cwd.to_string_lossy().into_owned()),
                        });
                    }
                    if let Some(name) = swarm_name {
                        actions.push(bbt_proto::GuiAction::SwarmRename {
                            id: Ulid::new(),
                            swarm,
                            name,
                        });
                    }
                    Sudo::Swarm(value, actions)
                }
                Request::Archive { swarm } => {
                    let principal = crate::mcp::Principal::Operator {
                        device_id: self
                            .operator_device_for_effect()
                            .map_err(|e| e.to_string())?,
                        door: crate::mcp::Door::Ctl,
                        issued_by: Some(format!(
                            "sudo:{}:{}",
                            actor.seat_id, actor.process_generation
                        )),
                    };
                    let registry = self.registry.clone();
                    let value = self
                        .operator_journal
                        .effect(principal, &call, || {
                            match registry.archive_empty_swarm(swarm) {
                                Ok(record) => crate::mcp::success(
                                    op,
                                    json!({"status":"applied","swarm":record}),
                                ),
                                Err(error) => crate::mcp::error(op, "rejected", &error.to_string()),
                            }
                        })
                        .map_err(|e| e.to_string())?;
                    Sudo::Done(value)
                }
                Request::Swarm {
                    swarm,
                    name,
                    workdir,
                } => {
                    let mut actions = vec![];
                    if let Some(name) = name {
                        actions.push(bbt_proto::GuiAction::SwarmRename {
                            id: Ulid::new(),
                            swarm,
                            name,
                        });
                    }
                    if let Some(path) = workdir {
                        actions.push(bbt_proto::GuiAction::SwarmWorkdir {
                            id: Ulid::new(),
                            swarm,
                            path: Some(path),
                        });
                    }
                    Sudo::Swarm(
                        crate::mcp::success(op, json!({"status":"applied","swarm_id":swarm})),
                        actions,
                    )
                }
                Request::Message {
                    swarm,
                    to,
                    text,
                    reply_to,
                } => {
                    let destination = if to.contains('/') {
                        to
                    } else {
                        let seats = self.registry.list().map_err(|e| e.to_string())?;
                        let seat = seats
                            .iter()
                            .filter(|r| r.swarm == swarm && r.lifecycle == Lifecycle::Open)
                            .find(|r| r.id.to_string() == to || r.name == to)
                            .ok_or("no open seat with that id or name in this swarm")?;
                        format!("seats/{}/inbox", seat.id)
                    };
                    let mut message = bbt_proto::messaging::SendRequest::text(destination, text);
                    message.id = op;
                    message.reply_to = reply_to;
                    let response = self
                        .gui_messaging(
                            op,
                            swarm,
                            crate::messaging::api::Request::Send { message },
                            origin(),
                        )
                        .map_err(|e| e.to_string())?;
                    if let bbt_proto::messaging::Response::Refused { error } = &response {
                        return Err(error.message.clone());
                    }
                    Sudo::Done(crate::mcp::success(
                        op,
                        json!({"message_id":op,"swarm_id":swarm,"response":response}),
                    ))
                }
                Request::Inbox {
                    swarm,
                    since,
                    limit,
                } => {
                    let swarms = match swarm {
                        Some(swarm) => vec![swarm],
                        None => self.sudo_swarm_ids()?,
                    };
                    let mut pages = vec![];
                    for swarm in swarms {
                        let request = crate::messaging::api::Request::List {
                            thread: None,
                            channel: None,
                            after: since,
                            limit,
                        };
                        match self.gui_messaging(Ulid::new(), swarm, request, origin()) {
                            Ok(response) => {
                                pages.push(json!({"swarm_id":swarm,"response":response}))
                            }
                            Err(error) => {
                                pages.push(json!({"swarm_id":swarm,"error":error.to_string()}))
                            }
                        }
                    }
                    Sudo::Done(crate::mcp::success(op, json!({"swarms":pages})))
                }
            })
        })();
        result.unwrap_or_else(|reason| Sudo::Done(crate::mcp::error(op, "rejected", &reason)))
    }

    /// Swarms with metadata or an open seat, in id order.
    fn sudo_swarm_ids(&self) -> std::result::Result<Vec<Ulid>, String> {
        let mut ids: std::collections::BTreeSet<Ulid> = self
            .registry
            .swarms()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|r| r.id)
            .collect();
        ids.extend(
            self.registry
                .list()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|r| r.lifecycle == Lifecycle::Open)
                .map(|r| r.swarm),
        );
        Ok(ids.into_iter().collect())
    }

    fn sudo_swarms(&mut self) -> std::result::Result<Value, String> {
        let records = self.registry.swarms().map_err(|e| e.to_string())?;
        let seats = self.registry.list().map_err(|e| e.to_string())?;
        let owned = self.connections.owned_seats().map(|rows| {
            rows.into_iter()
                .map(|(_, info)| (info.seat, info.generation))
                .collect::<std::collections::BTreeSet<_>>()
        });
        let process_error = owned.as_ref().err().map(ToString::to_string);
        let swarms: Vec<_> =
            self.sudo_swarm_ids()?
                .into_iter()
                .map(|swarm| {
                    let record = records.iter().find(|r| r.id == swarm);
                    let members: Vec<_> = seats
                    .iter()
                    .filter(|r| r.swarm == swarm && r.lifecycle == Lifecycle::Open)
                    .map(|r| {
                        let projection = self.control_seat(r);
                        let task_paths: Vec<_> = self.workspaces.cache.current(r.swarm).ok()
                            .and_then(|binding| self.workspaces.cache.get(binding).ok())
                            .map(|root| root.index.documents.values().filter(|doc| doc.id.parse::<Ulid>().is_ok_and(|id| r.tasks.contains(&id))).map(|doc| doc.path.clone()).collect())
                            .unwrap_or_default();
                        json!({
                            "seat_id":r.id,"name":r.name,"role":r.role.name,"class":r.role.class,
                            "harness":r.harness_kind,"process_generation":r.generation.to_string(),
                            "process_state": match &owned { Ok(seats) if seats.contains(&(r.id,r.generation)) => "running", Ok(_) => "gone", Err(_) => "unknown" },
                            "adapter":projection["adapter"],"tasks":r.tasks,"task_paths":task_paths,
                        })
                    })
                    .collect();
                    json!({
                        "swarm_id":swarm,
                        "name":record.and_then(|r| r.name.clone()),
                        "workdir":record.and_then(|r| r.workdir.clone()),
                        "seats":members,
                    })
                })
                .collect();
        Ok(json!({"swarms":swarms,"process_error":process_error}))
    }

    /// Commits the swarm actions in order on the metadata journal, then replies.
    pub(super) fn sudo_swarm_actions(
        &self,
        value: Value,
        actions: Vec<bbt_proto::GuiAction>,
        reply: oneshot::Sender<Result<Value>>,
    ) {
        let op: Ulid =
            serde_json::from_value(value["structuredContent"]["op_id"].clone()).unwrap_or_default();
        let receivers: Vec<_> = actions
            .into_iter()
            .map(|action| {
                let (tx, rx) = oneshot::channel();
                self.start_gui_swarm(action, origin(), tx);
                rx
            })
            .collect();
        self.handle.spawn(async move {
            let mut value = value;
            for rx in receivers {
                let outcome = rx
                    .await
                    .map_err(|_| "swarm metadata owner stopped".to_string())
                    .and_then(|r| r.map_err(|e| e.to_string()));
                match outcome {
                    Ok(bbt_proto::GuiReply::Swarm {
                        record: Some(record),
                        ..
                    }) => {
                        value["structuredContent"]["swarm"] = json!(record);
                    }
                    Ok(_) => {}
                    Err(reason) => {
                        value = crate::mcp::error(
                            op,
                            "partial",
                            &format!(
                                "swarm update failed after any earlier step applied: {reason}"
                            ),
                        );
                        break;
                    }
                }
            }
            if value["isError"] != true {
                value["content"][0]["text"] = value["structuredContent"].to_string().into();
            }
            let _ = reply.send(Ok(value));
        });
    }
}
