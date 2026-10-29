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

impl Worker {
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
            let keys = self.identity.keys();
            if let Some(reason) = keys.signing_hold() {
                return Err(invalid(format!("work-item signing held: {reason}")));
            }
            let (key, issuer) = match &actor {
                Actor::Seat(seat) if keys.ready_issuer().is_some() => (
                    keys.leaf(seat.seat_id, seat.process_generation)?,
                    seat.issuer_kind,
                ),
                Actor::Operator { device_id, .. } => {
                    let authority = self.identity.operator_signing()?;
                    if authority.device_id != *device_id {
                        return Err(invalid("operator device changed"));
                    }
                    (authority.key, authority.issuer.kind)
                }
                _ => return Err(invalid("import requires admitted signing custody")),
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
            ))
        })();
        let (owner, owner_record, owner_scope, key, issuer, root, inode, permit) = match prepared {
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
        #[cfg(test)]
        let hook = self.task_import_hook.clone();
        self.task_import_jobs.spawn_blocking_on(move || {
            let _permit=permit;
            let result=(|| -> Result<_> {
                let effect = crate::mcp::work_items::prepare(&port.index,&registry,&actor,&call).map_err(invalid)?;
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
                        Ok(())
                    },|| {
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
                    match result {Ok(data)=>crate::mcp::success(call.op_id,data),Err(error)=>crate::mcp::error(call.op_id,if error.downcast_ref::<crate::index::IndexBusy>().is_some(){"index_busy"}else if error.downcast_ref::<crate::index::migration::AllocationStale>().is_some(){"import_allocation_stale"}else{"rejected"},&error.to_string())}
                })
            })();
            reply(match result {Ok(v)=>Ok(v),Err(error)=>Ok(crate::mcp::error(call.op_id,"rejected",&error.to_string()))});
        },&self.handle);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::sync::{Condvar, Mutex};

    #[derive(Default)]
    struct Gate {
        entered: tokio::sync::Notify,
        released: Mutex<bool>,
        wake: Condvar,
    }
    impl Gate {
        fn hold(&self) {
            self.entered.notify_one();
            let guard = self.released.lock().unwrap();
            drop(self.wake.wait_while(guard, |released| !*released).unwrap());
        }
        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.wake.notify_all();
        }
    }
    struct Release(Arc<Gate>);
    impl Drop for Release {
        fn drop(&mut self) {
            self.0.release();
        }
    }

    async fn call(client: &LogicClient, seat: Ulid, tool: &str, args: Value) -> Value {
        let op_id = Ulid::new();
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let reply = client
                    .control_call(
                        seat,
                        1,
                        Call {
                            op_id,
                            tool: tool.into(),
                            args: args.clone(),
                        },
                    )
                    .await
                    .unwrap();
                if reply["structuredContent"]["error"]["code"] != "workspace_pending" {
                    return reply;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("import must not hold unrelated control replies")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task_migration_held_import_keeps_messages_tasks_and_other_roots_live_and_refuses_stale_publication()
     {
        let temp = tempfile::tempdir().unwrap();
        let input = std::env::var_os("BBOXY_TASK_MIGRATION_INPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../scripts/test-task-migration.py");
                assert!(
                    std::process::Command::new("python3")
                        .arg(script)
                        .arg("prepare")
                        .arg(temp.path())
                        .status()
                        .unwrap()
                        .success()
                );
                temp.path().join("task-migration/input")
            });
        let repository = temp.path().join("repository");
        let root = repository.join(".bboxy");
        std::fs::create_dir_all(root.join("tasks")).unwrap();
        let root = root.canonicalize().unwrap();
        let owner = Ulid::new();
        let swarm = Ulid::new();
        let other = Ulid::new();
        let other_swarm = Ulid::new();
        let task = Ulid::new();
        let task_path = root.join("tasks/preserved.md");
        let original = format!(
            "---\nid: {task}\nnumber: 171\ntype: task\nstatus: running\nassignee: seat:{owner}\n---\nExisting live task.\n"
        );
        std::fs::write(&task_path, &original).unwrap();
        let control = Ulid::new();
        std::fs::write(root.join("tasks/control.md"),format!("---\nid: {control}\nnumber: 172\ntype: task\nstatus: running\nassignee: seat:{owner}\n---\nOrdinary task control.\n")).unwrap();
        let supervisor = crate::Supervisor::start(crate::Config {
            journal_directory: Some(temp.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let run = temp.path().join("run");
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: run.clone(),
                repository,
                ring: Default::default(),
            },
            SigningKey::from_bytes(&[37; 32]),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        for (id, swarm) in [(owner, swarm), (other, other_swarm)] {
            let mut record = SeatRecord::new(id, swarm, "private migration oracle", "zsh");
            record.role.name = "oracle".into();
            record.role.class = Class::Oracle;
            let mut spec = LaunchSpec::new("/bin/zsh", temp.path());
            spec.seat = id;
            spec.args = vec![
                "-f".into(),
                "-c".into(),
                "while IFS= read -r line; do :; done".into(),
            ];
            logic.client.spawn(record, spec).await.unwrap();
        }
        let registry = Registry::new(SqliteStore::open(&run.join("account")).unwrap());
        let other_project = temp.path().join("other");
        std::fs::create_dir(&other_project).unwrap();
        registry
            .update_swarm_workdir(other_swarm, Some(other_project.to_str().unwrap()))
            .unwrap();
        let gate = Arc::new(Gate::default());
        let release = Release(gate.clone());
        let (tx, rx) = oneshot::channel();
        let held = gate.clone();
        logic
            .client
            .tx
            .send(Command::TaskImportFixture(
                Arc::new(move || held.hold()),
                tx,
            ))
            .await
            .unwrap();
        rx.await.unwrap();
        let manifest = input.join("lossless-import-manifest.json");
        let audit = input.join("checked-audit.json");
        let sources = input.join("dependency-source");
        let digest = crate::index::migration::bundle_digest(&manifest, &audit, &sources).unwrap();
        let args = json!({"expected_root":root,"manifest_path":manifest,"audit_path":audit,"source_dir":sources,"bundle_sha256":digest,"owner_seat":owner});
        let client = logic.client.clone();
        let request = args.clone();
        let import =
            tokio::spawn(async move { call(&client, owner, "task_import", request).await });
        tokio::time::timeout(Duration::from_secs(15), gate.entered.notified())
            .await
            .unwrap();
        assert!(
            !root.join("imports").exists(),
            "staged subset became active"
        );
        assert_eq!(
            call(&logic.client, owner, "message_list", json!({})).await["isError"],
            false
        );
        let tasks = call(&logic.client, owner, "task_list", json!({})).await;
        assert_eq!(
            tasks["structuredContent"]["items"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            call(&logic.client, other, "task_list", json!({})).await["isError"],
            false
        );
        let own_context = call(&logic.client, owner, "task_import_context", json!({})).await;
        let other_context = call(&logic.client, other, "task_import_context", json!({})).await;
        assert_eq!(
            own_context["structuredContent"]["expected_root"],
            json!(root)
        );
        assert_eq!(
            other_context["structuredContent"]["expected_root"],
            json!(other_project.join(".bboxy").canonicalize().unwrap())
        );
        assert_ne!(
            own_context["structuredContent"]["expected_root"],
            other_context["structuredContent"]["expected_root"]
        );
        let busy = call(&logic.client, owner, "task_import", args).await;
        assert_eq!(
            busy["structuredContent"]["error"]["code"],
            "task_import_busy"
        );
        let ordinary=call(&logic.client,owner,"task_reconcile",json!({"task_path":"#172","disposition":"superseded","reason":"private contention control","evidence":["fixture"]})).await;
        assert_eq!(
            ordinary["isError"], false,
            "ordinary same-root task write must complete during bulk staging: {ordinary}"
        );
        assert_eq!(std::fs::read_to_string(&task_path).unwrap(), original);
        let next = temp.path().join("next");
        std::fs::create_dir(&next).unwrap();
        registry
            .update_swarm_workdir(swarm, Some(next.to_str().unwrap()))
            .unwrap();
        drop(release);
        let refused = import.await.unwrap();
        assert_eq!(refused["isError"], true);
        assert!(refused.to_string().contains("swarm_root_changed"));
        assert!(!root.join("imports").exists());
        assert_eq!(std::fs::read_to_string(task_path).unwrap(), original);
        logic.shutdown().unwrap();
        supervisor.shutdown().unwrap();
    }
}
