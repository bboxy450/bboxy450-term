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
    pub(super) fn sudo_call(&mut self, id: Ulid, generation: u64, call: crate::mcp::Call) -> Sudo {
        let op = call.op_id;
        let result = (|| -> std::result::Result<Sudo, String> {
            let (actor, _) = self
                .control_actor(id, generation)
                .map_err(|e| e.to_string())?;
            Ok(match request(&actor, &call)? {
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

    fn sudo_swarms(&self) -> std::result::Result<Value, String> {
        let records = self.registry.swarms().map_err(|e| e.to_string())?;
        let seats = self.registry.list().map_err(|e| e.to_string())?;
        let swarms: Vec<_> =
            self.sudo_swarm_ids()?
                .into_iter()
                .map(|swarm| {
                    let record = records.iter().find(|r| r.id == swarm);
                    let members: Vec<_> = seats
                    .iter()
                    .filter(|r| r.swarm == swarm && r.lifecycle == Lifecycle::Open)
                    .map(|r| json!({
                        "seat_id":r.id,"name":r.name,"role":r.role.name,"class":r.role.class,
                        "harness":r.harness_kind,"process_generation":r.generation.to_string(),
                    }))
                    .collect();
                    json!({
                        "swarm_id":swarm,
                        "name":record.and_then(|r| r.name.clone()),
                        "workdir":record.and_then(|r| r.workdir.clone()),
                        "seats":members,
                    })
                })
                .collect();
        Ok(json!({"swarms":swarms}))
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
