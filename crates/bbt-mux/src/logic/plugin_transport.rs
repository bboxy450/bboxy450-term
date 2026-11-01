// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Asynchronous host-only wrap, followed by root/activation-fenced launch.
use super::*;

pub(super) struct Completion {
    principal: crate::mcp::Principal,
    call: crate::mcp::Call,
    record: SeatRecord,
    harness: HarnessRecord,
    options: crate::adapters::launch::LaunchOptions,
    binding: RootBinding,
    host: crate::plugins::PluginHost,
    result: crate::knowledge::data_source::Result<crate::plugins::TransportPlan>,
    reply: oneshot::Sender<Result<bbt_proto::SeatId>>,
}

fn seat_result(result: serde_json::Value) -> Result<bbt_proto::SeatId> {
    if result["isError"] == true {
        let error = &result["structuredContent"]["error"];
        return Err(invalid(format!(
            "{}: {}",
            error["code"].as_str().unwrap_or("unknown"),
            error["message"]
                .as_str()
                .unwrap_or("transport launch unavailable")
        )));
    }
    let seat: Ulid = serde_json::from_value(result["structuredContent"]["seat"].clone())?;
    Ok(bbt_proto::SeatId::from_u128(seat.0))
}

impl Worker {
    pub(super) fn start_plugin_transport(
        &mut self,
        id: Ulid,
        spec: bbt_proto::SpawnSpec,
        door: crate::mcp::Door,
        reply: oneshot::Sender<Result<bbt_proto::SeatId>>,
    ) {
        let context = (|| -> Result<_> {
            self.roles_idle()?;
            let call = crate::mcp::Call {
                op_id: id,
                tool: "gui_spawn".into(),
                args: serde_json::to_value(&spec)?,
            };
            call.validate()?;
            let principal = crate::mcp::Principal::Operator {
                device_id: self.operator_device_for_effect()?,
                door,
                issued_by: self.identity.issued_by().map(str::to_owned),
            };
            Ok((principal, call))
        })();
        let (principal, call) = match context {
            Ok(v) => v,
            Err(e) => {
                let _ = reply.send(Err(e));
                return;
            }
        };
        match self.operator_journal.admit(&principal, &call) {
            Ok((old, false)) => {
                let _ = reply.send(seat_result(old));
                return;
            }
            Ok((_, true)) => (),
            Err(e) => {
                let _ = reply.send(Err(e));
                return;
            }
        };
        let prepared = (|| -> Result<_> {
            let (record, harness, options) = self.prepare_gui_spawn(spec)?;
            let port = self.workspaces.select(&self.registry, record.swarm, None)?;
            let host = port.host.plugin_host().clone();
            Ok((record, harness, options, port.binding, host))
        })();
        let (record, harness, options, binding, host) = match prepared {
            Ok(v) => v,
            Err(e) => {
                let result = self
                    .operator_journal
                    .complete(
                        &principal,
                        &call,
                        crate::mcp::error(id, "rejected", &e.to_string()),
                    )
                    .and_then(seat_result);
                let _ = reply.send(result);
                return;
            }
        };
        let tx = self.client.tx.clone();
        self.plugin_jobs.spawn_blocking_on(
            move || {
                let via = options.via.as_ref().expect("plugin transport");
                let result = (|| {
                    let inner = bbt_proto::TransportSpec::inner_command(
                        &options.cwd.to_string_lossy(),
                        &harness.command,
                        &options.env,
                    )
                    .map_err(|e| {
                        crate::knowledge::data_source::CommandError::new(
                            "invalid_transport_command",
                            e,
                        )
                    })?;
                    host.wrap_transport(
                        via.kind.strip_prefix("plugin:").unwrap(),
                        &via.fields,
                        vec!["/bin/sh".into(), "-lc".into(), inner],
                        Instant::now() + Duration::from_secs(2),
                    )
                })();
                let done = Completion {
                    principal,
                    call,
                    record,
                    harness,
                    options,
                    binding,
                    host,
                    result,
                    reply,
                };
                if let Err(error) = tx.try_send(Command::PluginTransportReady(Box::new(done)))
                    && let Command::PluginTransportReady(done) = error.into_inner()
                {
                    let _ = done.reply.send(Err(invalid(
                        "transport completion unavailable; query the original spawn operation",
                    )));
                }
            },
            &self.handle,
        );
    }
    pub(super) fn finish_plugin_transport(&mut self, mut done: Completion) {
        let outcome = (|| -> Result<_> {
            let plan = done
                .result
                .map_err(|e| invalid(format!("{}: {}", e.code, e.message)))?;
            self.workspaces.current(&self.registry, &done.binding)?;
            if !done.host.transport_plan_current(&plan) {
                return Err(invalid(
                    "transport_changed: activation changed before launch",
                ));
            }
            if let crate::mcp::Principal::Operator { device_id, .. } = &done.principal
                && self.operator_device_for_effect()? != *device_id
            {
                return Err(invalid("operator identity changed during transport wrap"));
            }
            self.roles_idle()?;
            done.options.transport_argv = Some(plan.argv);
            Ok((done.record, done.harness, done.options))
        })();
        let value = match outcome {
            Ok((record, harness, options)) => match self.launch(record, harness, options) {
                Ok(info) => {
                    crate::mcp::success(done.call.op_id, serde_json::json!({"seat":info.seat}))
                }
                Err(error) => crate::mcp::error(done.call.op_id, "unknown", &error.to_string()),
            },
            Err(error) => crate::mcp::error(done.call.op_id, "rejected", &error.to_string()),
        };
        let result = self
            .operator_journal
            .complete(&done.principal, &done.call, value)
            .and_then(seat_result);
        let _ = done.reply.send(result);
    }
}
