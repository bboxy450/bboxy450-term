// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure document read jobs; the logic owner revalidates seat/root authority on completion.
use super::{data_source as d, roots::Host, workspaces::RootBinding};
use crate::{
    index::Index,
    mcp::{self, Call, SeatActor, journal::Journal, tools},
    registry::{Registry, SeatRecord},
    store::{Result, invalid},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, oneshot};

pub(crate) struct Completion {
    pub actor: SeatActor,
    pub binding: RootBinding,
    pub revision: u64,
    pub reply: oneshot::Sender<Result<Value>>,
    pub result: Result<Value>,
}
impl Completion {
    pub(crate) fn current_binding(&self, current: &RootBinding) -> Result<()> {
        if &self.binding != current || self.binding.swarm != self.actor.swarm_id {
            return Err(invalid(
                "swarm_root_changed: workspace changed during knowledge read",
            ));
        }
        Ok(())
    }
}

fn retained_context(value: &Value, binding: &RootBinding, provider_revision: u64) -> bool {
    let context = &value["structuredContent"];
    value["content"][0]["text"]
        .as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .as_ref()
        == Some(context)
        && context["knowledge_root_id"] == json!(binding.root_id)
        && context["swarm_workspace_id"] == json!(binding.swarm)
        && context["swarm_workspace_revision"] == json!(binding.revision)
        && context["knowledge_root_revision"] == json!(provider_revision)
}
fn scoped_success(op: crate::Ulid, mut data: Value, binding: &RootBinding, revision: u64) -> Value {
    data["knowledge_root_revision"] = json!(revision);
    data["knowledge_root_id"] = json!(binding.root_id);
    data["swarm_workspace_id"] = json!(binding.swarm);
    data["swarm_workspace_revision"] = json!(binding.revision);
    mcp::success(op, data)
}
pub(crate) fn handles(call: &Call) -> bool {
    mcp::skills::handles(&call.tool)
        || matches!(
            call.tool.as_str(),
            "knowledge_fetch"
                | "knowledge_status"
                | "knowledge_search"
                | "orient"
                | "task_status"
                | "task_list"
                | "task_import_context"
        )
        || call.tool == "graph_read"
            && matches!(call.args["graph"].as_str(), Some("task" | "markdown"))
}
pub(crate) struct Job {
    pub host: Host,
    pub index: Arc<Index>,
    pub registry: Registry,
    pub actor: SeatActor,
    pub binding: RootBinding,
    pub record: SeatRecord,
    pub call: Call,
    pub journal: Journal,
    pub orient: Option<Value>,
    pub revision: u64,
    pub reply: oneshot::Sender<Result<Value>>,
    pub permit: OwnedSemaphorePermit,
}
struct ReadFailure {
    code: String,
    message: String,
}
impl From<String> for ReadFailure {
    fn from(message: String) -> Self {
        Self {
            code: mcp::work_items::failure_code(&message).into(),
            message,
        }
    }
}
impl From<d::CommandError> for ReadFailure {
    fn from(error: d::CommandError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}
impl From<serde_json::Error> for ReadFailure {
    fn from(error: serde_json::Error) -> Self {
        error.to_string().into()
    }
}
impl Job {
    pub(crate) fn run(self) -> Completion {
        let Self {
            host,
            index,
            registry,
            actor,
            binding,
            record,
            call,
            journal,
            orient,
            revision,
            reply,
            permit,
        } = self;
        let _permit = permit;
        let result = if host.root_id() != binding.root_id.as_str()
            || journal.root_id() != Some(binding.root_id.as_str())
            || actor.swarm_id != binding.swarm
        {
            Err(invalid(
                "swarm_root_changed: knowledge read root does not match admission",
            ))
        } else if call.tool == "knowledge_status" || mcp::skills::handles(&call.tool) {
            // Status must remain readable when the selected provider is broken.
            let result = if call.tool == "knowledge_status" {
                tools::parse::<tools::Empty>(&call.args)
                    .map_err(|reason| d::CommandError::new("invalid_arguments", reason))
                    .and_then(|_| host.seat_status(&index))
            } else {
                mcp::skills::read(&host, &actor, &call)
            };
            let value = match result {
                Ok(value) => scoped_success(call.op_id, value, &binding, revision),
                Err(reason) => mcp::error(call.op_id, &reason.code, &reason.message),
            };
            journal.checked_authorized(
                &actor,
                &call,
                mcp::knowledge::bounded(call.op_id, value, false),
                |old| {
                    retained_context(old, &binding, revision)
                        && (call.tool == "knowledge_status"
                            || mcp::skills::retained(&host, &actor, &call, old))
                },
            )
        } else {
            host.with_source(&index, &registry, |source| {
                let result: std::result::Result<Value, ReadFailure> = (|| match call.tool.as_str() {
                    "task_status" | "task_list" | "task_import_context" => {
                        let authority = mcp::Actor::from(&actor);
                        let tasks = mcp::work_items::ProviderTasks(source, Some(actor.swarm_id));
                        match mcp::work_items::prepare(&tasks, &registry, &authority, &call)? {
                            mcp::work_items::Prepared::Read(mut value) => {
                                if call.tool == "task_list" {
                                    let empty =
                                        value["items"].as_array().is_some_and(Vec::is_empty);
                                    value = tools::scoped_list(
                                        &actor,
                                        value,
                                        empty.then_some("nothing_assigned"),
                                    );
                                }
                                Ok(value)
                            }
                            _ => Err("scheduled task read cannot mutate".to_owned().into()),
                        }
                    }
                    "knowledge_fetch" => {
                        let mcp::knowledge::Request::Fetch(locator) =
                            mcp::knowledge::prepare(&call)?
                        else {
                            unreachable!()
                        };
                        Ok(serde_json::to_value(mcp::knowledge::fetch(
                            source, locator,
                        )?)?)
                    }
                    "knowledge_search" => {
                        let mcp::knowledge::Request::Search(request) =
                            mcp::knowledge::prepare(&call)?
                        else {
                            unreachable!()
                        };
                        let page = source.search(request)?;
                        let empty = page.items.is_empty();
                        Ok(tools::scoped_list(
                            &actor,
                            serde_json::to_value(page).map_err(|e| e.to_string())?,
                            empty.then_some("nothing_yet"),
                        ))
                    }
                    "orient" => {
                        let tasks = tools::orient_documents(source, &record)?;
                        let empty = tasks["items"].as_array().is_some_and(Vec::is_empty);
                        let mut base = orient.expect("host orientation prepared before IO");
                        base["tasks"] =
                            tools::scoped_list(&actor, tasks, empty.then_some("nothing_assigned"));
                        Ok(base)
                    }
                    "graph_read" => Ok(tools::graph(
                        &index,
                        source,
                        &actor,
                        &record,
                        &tools::parse(&call.args)?,
                        false,
                    )?),
                    _ => unreachable!("only document reads are scheduled"),
                })();
                let value = match result {
                    Ok(value) => scoped_success(call.op_id, value, &binding, revision),
                    Err(reason) => mcp::error(call.op_id, &reason.code, &reason.message),
                };
                let value = mcp::knowledge::bounded(call.op_id, value, false);
                journal
                    .checked_authorized(&actor, &call, value, |old| {
                        retained_context(old, &binding, revision)
                            && if matches!(call.tool.as_str(), "task_status" | "task_list") {
                                mcp::work_items::retained_allowed(
                                    &mcp::work_items::ProviderTasks(source, Some(actor.swarm_id)),
                                    &registry,
                                    &mcp::Actor::from(&actor),
                                    &call,
                                    old,
                                )
                            } else {
                                tools::retained_documents(source, &actor, &record, &call, old)
                            }
                    })
                    .map_err(|e| d::CommandError::new("journal_unavailable", e.to_string()))
            })
            .map_err(|e| invalid(e.to_string()))
        };
        Completion {
            actor,
            binding,
            revision,
            reply,
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Ulid,
        messaging::wire::IssuerKind,
        registry::{Class, Role},
    };

    fn binding() -> RootBinding {
        RootBinding {
            swarm: Ulid::from(1_u128),
            revision: 7,
            root_id: "root-A".into(),
        }
    }
    fn completion(binding: RootBinding) -> Completion {
        let (reply, _) = oneshot::channel();
        Completion {
            actor: SeatActor {
                seat_id: Ulid::from(2_u128),
                process_generation: 1,
                swarm_id: Ulid::from(1_u128),
                role: Role {
                    name: "executor".into(),
                    class: Class::Worker,
                },
                grant_epoch: 1,
                issuer_kind: IssuerKind::LocalDev,
            },
            binding,
            revision: 0,
            reply,
            result: Ok(json!({})),
        }
    }
    #[test]
    fn equal_provider_revisions_do_not_authorize_another_root_or_stale_swarm_binding() {
        let captured = binding();
        let done = completion(captured.clone());
        assert!(done.current_binding(&captured).is_ok());
        for current in [
            RootBinding {
                root_id: "root-B".into(),
                ..captured.clone()
            },
            RootBinding {
                revision: captured.revision + 1,
                ..captured.clone()
            },
            RootBinding {
                swarm: Ulid::from(3_u128),
                ..captured.clone()
            },
        ] {
            assert!(
                done.current_binding(&current)
                    .unwrap_err()
                    .to_string()
                    .contains("swarm_root_changed")
            );
        }
        let wrong_actor = RootBinding {
            swarm: Ulid::from(3_u128),
            ..captured
        };
        assert!(
            completion(wrong_actor.clone())
                .current_binding(&wrong_actor)
                .is_err()
        );
    }
    #[test]
    fn retained_read_context_covers_root_swarm_binding_and_provider_revision() {
        let captured = binding();
        let mut value = scoped_success(
            Ulid::new(),
            json!({"body":"exact native content"}),
            &captured,
            0,
        );
        assert_eq!(
            serde_json::from_str::<Value>(value["content"][0]["text"].as_str().unwrap()).unwrap(),
            value["structuredContent"]
        );
        assert!(retained_context(&value, &captured, 0));
        let other = RootBinding {
            root_id: "root-B".into(),
            ..captured.clone()
        };
        assert!(!retained_context(&value, &other, 0));
        let stale = RootBinding {
            revision: captured.revision + 1,
            ..captured.clone()
        };
        assert!(!retained_context(&value, &stale, 0));
        assert!(!retained_context(&value, &captured, 1));
        // A journal produced by the old host can have correct structured
        // context but omit it from text. Refuse that retained representation.
        value["content"][0]["text"] = json!("{\"body\":\"exact native content\"}");
        assert!(!retained_context(&value, &captured, 0));
        value = scoped_success(Ulid::new(), json!({}), &captured, 0);
        value["structuredContent"]
            .as_object_mut()
            .unwrap()
            .remove("knowledge_root_id");
        assert!(!retained_context(&value, &captured, 0));
    }
}
