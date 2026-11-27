// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{Context, Worker, WorkspacePort, effects};
use crate::{
    identity::IdentityStatus,
    knowledge::data_source as d,
    mcp::{self, Call, SeatActor, artifacts},
    provenance::{
        author::Author,
        stored::{Attempt, ExecutionSource, Store, Write},
    },
    store::{Result, invalid},
};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::runtime::Handle;

pub(super) struct Guard {
    pub(super) expected: Value,
    pub(super) offer: Option<Value>,
}

pub(super) struct Admission {
    runtime: PathBuf,
    stage_root: PathBuf,
    pub(super) guard: Option<Guard>,
    harness: Option<crate::adapters::Binding>,
    pub(super) fresh: bool,
    #[cfg(all(feature = "browser", target_os = "macos"))]
    pub(super) browser_png: Option<Vec<u8>>,
}

pub(super) fn stage_root(port: &WorkspacePort) -> Result<PathBuf> {
    port.index
        .knowledge_root()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| invalid("artifact_source_forbidden: workspace has no repository directory"))
}

impl Worker {
    pub(super) fn artifact_admission(
        &mut self,
        actor: &SeatActor,
        port: &WorkspacePort,
        guard: Option<Guard>,
    ) -> Result<Admission> {
        let record = self.registry.get(actor.seat_id)?;
        if record.location != crate::registry::Location::Local {
            return Err(invalid(
                "artifact_source_forbidden: native artifact custody requires a local seat",
            ));
        }
        let managed = self.implementation_workspaces.execution_context(
            actor,
            &port.binding,
            port.host.try_revision()?,
            None,
        )?;
        let stage_root = managed
            .map(|record| record.cwd)
            .map_or_else(|| stage_root(port), Ok)?;
        let harness = self
            .views
            .get(&actor.seat_id)
            .filter(|view| {
                view.generation == actor.process_generation
                    && view.adapter.record.kind == "bboxy450-harness"
            })
            .map(|view| view.adapter.binding.clone());
        Ok(Admission {
            runtime: self.config.run_dir.clone(),
            stage_root,
            guard,
            harness,
            fresh: false,
            #[cfg(all(feature = "browser", target_os = "macos"))]
            browser_png: None,
        })
    }
}

impl Admission {
    pub(super) fn prepare(
        &self,
        context: &Context,
        handle: &Handle,
    ) -> d::Result<effects::Prepared> {
        let call = &context.call;
        let result = self
            .plan(context, handle)
            .map(|plan| effects::Prepared::Artifact(Box::new(plan)));
        result.map_err(|error| {
            let response = artifacts::failure(call.op_id, error.as_ref());
            let detail = &response["structuredContent"]["error"];
            d::CommandError::new(
                detail["code"].as_str().unwrap_or("rejected"),
                detail["message"]
                    .as_str()
                    .unwrap_or("artifact prepare refused"),
            )
        })
    }

    fn plan(&self, context: &Context, handle: &Handle) -> Result<artifacts::Prepared> {
        let actor = context
            .actor
            .seat()
            .ok_or_else(|| invalid("artifact write requires an admitted seat"))?;
        #[cfg(all(feature = "browser", target_os = "macos"))]
        let write = if self.browser_png.is_some() {
            artifacts::Write::browser(&context.call)?
        } else {
            artifacts::Write::parse(&context.call)?
        };
        #[cfg(not(all(feature = "browser", target_os = "macos")))]
        let write = artifacts::Write::parse(&context.call)?;
        let store = Store::new(
            &context.port.index,
            &context.port.binding.root_id,
            &self.runtime,
        );
        if !matches!(
            store.inspect(actor.seat_id, context.call.op_id, &write.fingerprint)?,
            Attempt::Start
        ) {
            return Ok(artifacts::Prepared {
                write,
                bytes: None,
                source: None,
            });
        }
        if !self.fresh {
            return Err(invalid(
                "unknown: original operation has no durable capture",
            ));
        }
        if let Some(expected) = write.expected {
            store.check_expected(actor.seat_id, expected)?;
        }
        let source = self.guard.as_ref().map(|guard| {
            let offer = guard.offer.as_ref().ok_or_else(|| invalid("execution_source_unavailable: guarded artifact write has no broker dispatch"))?;
            let offer: crate::mcp::artifact_source::Offer = serde_json::from_value(offer.clone())
                .map_err(|_| invalid("execution_source_unavailable: invalid broker offer"))?;
            let binding = self.harness.as_ref().ok_or_else(|| invalid("execution_source_unavailable: process is not an admitted local Python host"))?;
            handle.block_on(offer.resolve(binding, &context.call, &guard.expected))
        }).transpose()?;
        let bytes = match &write.payload {
            #[cfg(all(feature = "browser", target_os = "macos"))]
            artifacts::Payload::Browser => self
                .browser_png
                .clone()
                .ok_or_else(|| invalid("browser capture unavailable"))?,
            artifacts::Payload::Inline(value) => {
                artifacts::inline_bytes(&write.media_type, value.clone())?
            }
            artifacts::Payload::Staged(path) => artifacts::staging::read(&self.stage_root, path)?,
        };
        artifacts::staging::validate_image(&write.media_type, &bytes)?;
        Ok(artifacts::Prepared {
            write,
            bytes: Some(bytes),
            source,
        })
    }

    pub(super) fn apply(
        &self,
        port: &WorkspacePort,
        actor: &SeatActor,
        call: &Call,
        plan: &artifacts::Prepared,
        author: &Author<'_>,
        observer: Option<(IdentityStatus, SigningKey)>,
    ) -> effects::Applied {
        let result = (|| -> Result<_> {
            let signer = observer
                .as_ref()
                .map(|(status, key)| Author::observer(status, key));
            let execution = plan
                .source
                .as_ref()
                .map(|source| {
                    let observer = signer.as_ref().ok_or_else(|| {
                        invalid("execution_source_unavailable: host signing unavailable")
                    })?;
                    Ok::<_, crate::store::Error>(ExecutionSource {
                        id: source.execution,
                        bytes: source.source.as_bytes(),
                        observer,
                    })
                })
                .transpose()?;
            let store = Store::new(&port.index, &port.binding.root_id, &self.runtime);
            store.write(
                &Write {
                    operation: call.op_id,
                    request_digest: plan.write.fingerprint.clone(),
                    expected: plan.write.expected,
                    media_type: &plan.write.media_type,
                    inputs: &plan.write.inputs,
                    execution,
                    reader: (actor.role.class != crate::registry::Class::Oracle)
                        .then_some(actor.seat_id),
                },
                author,
                plan.bytes.as_deref(),
            )
        })();
        let value = result
            .map(|revision| mcp::success(call.op_id, revision.value(true)))
            .unwrap_or_else(|error| artifacts::failure(call.op_id, error.as_ref()));
        let applied =
            value["isError"] != true || value["structuredContent"]["status"] == json!("unknown");
        effects::Applied {
            value,
            statuses: Vec::new(),
            applied,
        }
    }
}
