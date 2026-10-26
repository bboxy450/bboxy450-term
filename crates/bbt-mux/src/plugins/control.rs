//! The operator port. Filesystem IO and the descriptor process never run on logic's turn.
use std::{path::Path, sync::Arc};

use bbt_plugin::registry::Entry;
use bbt_proto::gui::PluginRequest;
use serde_json::Value;
use tokio::sync::Semaphore;

use super::Registry;
use crate::{
    Result,
    mcp::{self, Actor, Call, journal::Journal},
    store::invalid,
};

#[derive(Clone)]
pub(crate) struct Control {
    registry: Registry,
    reads: Arc<Semaphore>,
    effects: Arc<Semaphore>,
}

impl Control {
    pub(crate) fn new(registry: Registry) -> Self {
        Self {
            registry,
            reads: Arc::new(Semaphore::new(2)),
            effects: Arc::new(Semaphore::new(8)),
        }
    }

    pub(crate) async fn request(
        &self,
        actor: Actor,
        journal: Journal,
        call: Call,
        request: PluginRequest,
    ) -> Result<Vec<Entry>> {
        let mutation = request.mutation();
        let _permit = if mutation { &self.effects } else { &self.reads }
            .clone()
            .try_acquire_owned()
            .map_err(|_| invalid("plugin_busy: retry with the same operation ID"))?;
        let registry = self.registry.clone();
        let preparation = blocking(move || {
            if mutation {
                let (retained, fresh) = journal.admit(&actor, &call)?;
                if !fresh {
                    return Ok(Prepared::Retained(retained));
                }
            }
            let result = prepare(&registry, &actor, &request);
            Ok(Prepared::Work {
                registry,
                actor,
                journal,
                call,
                mutation,
                result,
            })
        })
        .await?;
        let value = match preparation {
            Prepared::Retained(value) => value,
            Prepared::Work {
                registry,
                actor,
                journal,
                call,
                mutation,
                result,
            } => {
                // A separate job per plugin: one slow descriptor cannot serialize the host.
                let result = match result {
                    Ok((_entries, Some(job))) => {
                        let result = job.run().await;
                        blocking(move || {
                            registry
                                .finish_inspection(result)
                                .map(|entry| vec![entry])
                                .map_err(refusal)
                        })
                        .await
                    }
                    Ok((entries, None)) => Ok(entries),
                    Err(error) => Err(error),
                };
                blocking(move || {
                    let value = match result {
                        Ok(entries) => {
                            let value = serde_json::to_value(entries)?;
                            if serde_json::to_vec(&value)?.len() > 256 * 1024 {
                                mcp::error(
                                    call.op_id,
                                    "plugin_inventory_too_large",
                                    "registry exceeds the bounded control page",
                                )
                            } else {
                                mcp::success(call.op_id, serde_json::json!({"entries":value}))
                            }
                        }
                        Err(error) => mcp::error(call.op_id, "plugin_refused", &error.to_string()),
                    };
                    if mutation {
                        journal.complete(&actor, &call, value)
                    } else {
                        journal.checked(&actor, &call, value)
                    }
                })
                .await?
            }
        };
        if value["isError"] == true {
            let error = &value["structuredContent"]["error"];
            return Err(invalid(format!(
                "{}: {}",
                error["code"].as_str().unwrap_or("plugin_refused"),
                error["message"]
                    .as_str()
                    .unwrap_or("plugin request refused")
            )));
        }
        serde_json::from_value(value["structuredContent"]["entries"].clone()).map_err(Into::into)
    }
}

enum Prepared {
    Retained(Value),
    Work {
        registry: Registry,
        actor: Actor,
        journal: Journal,
        call: Call,
        mutation: bool,
        result: Result<(Vec<Entry>, Option<super::InspectionJob>)>,
    },
}

fn refusal(error: bbt_plugin::Refusal) -> crate::store::Error {
    invalid(format!("{}: {}", error.code, error.message))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| invalid("plugin worker interrupted; retain the original operation ID"))?
}

fn prepare(
    registry: &Registry,
    actor: &Actor,
    request: &PluginRequest,
) -> Result<(Vec<Entry>, Option<super::InspectionJob>)> {
    let entries = registry.refresh().map_err(refusal)?;
    if matches!(request, PluginRequest::List) {
        return Ok((entries, None));
    }
    let selector = match request {
        PluginRequest::Trust { path } => normalized_source(path)?,
        PluginRequest::Enable { candidate } | PluginRequest::Disable { candidate } => {
            candidate.clone()
        }
        PluginRequest::List => unreachable!(),
    };
    let matches: Vec<_> = entries
        .iter()
        .filter(|e| e.candidate.source == selector || e.candidate.id == selector)
        .collect();
    let [entry] = matches.as_slice() else {
        return Err(invalid(
            "plugin_not_discovered: use a candidate ID from plugin list or its exact discovered path",
        ));
    };
    let id = &entry.candidate.id;
    let entry = match request {
        PluginRequest::Trust { .. } => {
            let digest = entry
                .candidate
                .sha256
                .as_deref()
                .ok_or_else(|| invalid("plugin_refused: candidate has no verified digest"))?;
            registry.trust(actor, id, digest, None).map_err(refusal)?
        }
        PluginRequest::Enable { .. } => registry.set_enabled(actor, id, true).map_err(refusal)?,
        PluginRequest::Disable { .. } => registry.set_enabled(actor, id, false).map_err(refusal)?,
        PluginRequest::List => unreachable!(),
    };
    let job = if matches!(request, PluginRequest::Trust { .. }) {
        registry.inspection(id).map_err(refusal)?
    } else {
        None
    };
    Ok((vec![entry], job))
}

// Canonicalize the parent only. The registry must still see and refuse a symlink leaf.
fn normalized_source(value: &str) -> Result<String> {
    let path = Path::new(value);
    if !path.is_absolute() || value.len() > 4096 {
        return Err(invalid(
            "invalid_plugin_path: use an absolute discovered path",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("invalid_plugin_path"))?
        .canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid("invalid_plugin_path"))?;
    Ok(parent.join(name).to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
