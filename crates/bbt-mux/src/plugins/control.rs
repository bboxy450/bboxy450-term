// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The operator port. Filesystem IO and the descriptor process never run on logic's turn.
use std::{path::Path, sync::Arc};

use bbt_plugin::registry::Entry;
use bbt_proto::gui::PluginRequest;
use serde_json::Value;
use tokio::sync::Semaphore;

use super::Registry;
use crate::{
    mcp::{self, Actor, Call, journal::Journal},
    store::{Result, invalid},
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
        let permit = if mutation { &self.effects } else { &self.reads }
            .clone()
            .try_acquire_owned()
            .map_err(|_| invalid("plugin_busy: retry with the same operation ID"))?;
        let registry = self.registry.clone();
        let preparation = blocking(move || {
            if mutation {
                let (retained, fresh) = journal.admit(&actor, &call)?;
                if !fresh {
                    return Ok((permit, Prepared::Retained(retained)));
                }
            }
            let result = prepare(&registry, &actor, &request);
            Ok((
                permit,
                Prepared::Work(Box::new(Work {
                    registry,
                    actor,
                    journal,
                    call,
                    mutation,
                    result,
                })),
            ))
        })
        .await?;
        let (permit, preparation) = preparation;
        let value = match preparation {
            Prepared::Retained(value) => value,
            Prepared::Work(work) => {
                let Work {
                    registry,
                    actor,
                    journal,
                    call,
                    mutation,
                    result,
                } = *work;
                // A separate job per plugin: one slow descriptor cannot serialize the host.
                let result = match result {
                    Ok((_entries, Some(job))) => match job {
                        Job::Descriptor(job) => {
                            let result = (*job).run().await;
                            blocking(move || {
                                registry
                                    .finish_inspection(result)
                                    .map(|entry| vec![entry])
                                    .map_err(refusal)
                            })
                            .await
                        }
                        Job::Conformance(job) => {
                            let result = (*job).run().await;
                            blocking(move || {
                                let entry = registry.finish_conformance(result).map_err(refusal)?;
                                if let Some(result) = &entry.conformance
                                    && !result.ok
                                {
                                    return Err(invalid(result.refusal.as_ref().map_or_else(
                                        || "contract_failed: conformance refused".into(),
                                        |r| format!("contract_failed: {}: {}", r.code, r.message),
                                    )));
                                }
                                Ok(vec![entry])
                            })
                            .await
                        }
                        // Status and login block on their processes, never logic's turn.
                        Job::Setup(job) => {
                            blocking(move || {
                                let run = (*job).run();
                                registry
                                    .finish_setup(run)
                                    .map(|entry| vec![entry])
                                    .map_err(refusal)
                            })
                            .await
                        }
                    },
                    Ok((entries, None)) => Ok(entries),
                    Err(error) => Err(error),
                };
                blocking(move || {
                    let _permit = permit;
                    let value = match result {
                        Ok(mut entries) => {
                            // A successful inspection duplicates the exact trusted Shape.
                            // Keep its complete schemas once, plus inspection timings/refusal.
                            // Durable authority/inspection records are never modified here.
                            for entry in &mut entries {
                                if let Some(inspection) = entry
                                    .activation
                                    .as_mut()
                                    .and_then(|a| a.inspection.as_mut())
                                    && inspection.shape.as_ref().is_some_and(|shape| {
                                        entry.trust.as_ref().and_then(|t| t.shape.as_ref())
                                            == Some(shape)
                                    })
                                {
                                    inspection.shape = None;
                                }
                            }
                            let value = serde_json::to_value(entries)?;
                            if serde_json::to_vec(&value)?.len() > 256 * 1024 {
                                mcp::error(
                                    call.op_id,
                                    "plugin_inventory_too_large",
                                    "registry exceeds the bounded control page",
                                )
                            } else {
                                let value =
                                    mcp::success(call.op_id, serde_json::json!({"entries":value}));
                                if serde_json::to_vec(&value)?.len() > mcp::MAX_RESULT {
                                    mcp::error(
                                        call.op_id,
                                        "plugin_inventory_too_large",
                                        "encoded registry exceeds the journal response bound",
                                    )
                                } else {
                                    value
                                }
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
    Work(Box<Work>),
}
struct Work {
    registry: Registry,
    actor: Actor,
    journal: Journal,
    call: Call,
    mutation: bool,
    result: Result<(Vec<Entry>, Option<Job>)>,
}

enum Job {
    Descriptor(Box<super::InspectionJob>),
    Conformance(Box<super::ConformanceJob>),
    Setup(Box<super::SetupJob>),
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
) -> Result<(Vec<Entry>, Option<Job>)> {
    let entries = registry.refresh().map_err(refusal)?;
    if matches!(request, PluginRequest::List) {
        return Ok((registry.with_measurements(entries), None));
    }
    let selector = match request {
        PluginRequest::Trust { path, .. } => normalized_source(path)?,
        PluginRequest::Enable { candidate }
        | PluginRequest::Disable { candidate }
        | PluginRequest::Check { candidate }
        | PluginRequest::Setup { candidate } => candidate.clone(),
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
            if let PluginRequest::Trust {
                sha256: Some(expected),
                ..
            } = request
                && expected != digest
            {
                return Err(invalid(
                    "plugin_changed_since_observed: refresh the registry and explicitly trust the current digest",
                ));
            }
            let approval = match request {
                PluginRequest::Trust {
                    grants: None,
                    tier: None,
                    ..
                } => None,
                PluginRequest::Trust { grants, tier, .. } => {
                    let name =
                        entry.candidate.name.as_ref().ok_or_else(|| {
                            invalid("plugin_refused: candidate has no parsed name")
                        })?;
                    let mut approval = bbt_plugin::registry::Approval::implied(
                        name,
                        grants.clone().unwrap_or_default(),
                    );
                    if let Some(tier) = tier {
                        approval.tier = *tier;
                    }
                    Some(approval)
                }
                _ => None,
            };
            registry
                .trust(actor, id, digest, approval)
                .map_err(refusal)?
        }
        PluginRequest::Enable { .. } => registry.set_enabled(actor, id, true).map_err(refusal)?,
        PluginRequest::Disable { .. } => registry.set_enabled(actor, id, false).map_err(refusal)?,
        PluginRequest::Check { .. } | PluginRequest::Setup { .. } => (**entry).clone(),
        PluginRequest::List => unreachable!(),
    };
    let job = match request {
        PluginRequest::Trust { .. } => registry
            .inspection(id)
            .map_err(refusal)?
            .map(Box::new)
            .map(Job::Descriptor),
        PluginRequest::Check { .. } => registry
            .conformance(actor, id)
            .map_err(refusal)?
            .map(Box::new)
            .map(Job::Conformance),
        PluginRequest::Setup { .. } => {
            Some(Job::Setup(Box::new(registry.setup(id).map_err(refusal)?)))
        }
        _ => None,
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
