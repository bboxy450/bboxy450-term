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
        let deadline = Instant::now() + Duration::from_secs(2);
        let via = options.via.clone().expect("plugin transport");
        let cwd = options.cwd.to_string_lossy().into_owned();
        let env = options.env.clone();
        let command = harness.command.clone();
        let wrap_host = host.clone();
        self.plugin_jobs.spawn_on(
            async move {
                let result = bounded_wrap(deadline, move || {
                    let inner = bbt_proto::TransportSpec::inner_command(&cwd, &command, &env)
                        .map_err(|e| {
                            crate::knowledge::data_source::CommandError::new(
                                "invalid_transport_command",
                                e,
                            )
                        })?;
                    wrap_host.wrap_transport(
                        via.kind.strip_prefix("plugin:").unwrap(),
                        &via.fields,
                        vec!["/bin/sh".into(), "-lc".into(), inner],
                        deadline,
                    )
                })
                .await;
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
            done.host
                .transport_plan_current(&plan)
                .map_err(|e| invalid(format!("{}: {}", e.code, e.message)))?;
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

struct CancelQueuedWrap(tokio::task::AbortHandle);
impl Drop for CancelQueuedWrap {
    fn drop(&mut self) {
        self.0.abort();
    }
}

// Origin: bboxy450-term; credit reuse: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// A separate async timer includes time queued behind blocking work. Only its
// result reaches the launch owner; a late blocking completion has no effects.
async fn bounded_wrap<T: Send + 'static>(
    deadline: Instant,
    work: impl FnOnce() -> crate::knowledge::data_source::Result<T> + Send + 'static,
) -> crate::knowledge::data_source::Result<T> {
    bounded_job(deadline, tokio::task::spawn_blocking(work)).await
}

async fn bounded_job<T: Send + 'static>(
    deadline: Instant,
    mut job: tokio::task::JoinHandle<crate::knowledge::data_source::Result<T>>,
) -> crate::knowledge::data_source::Result<T> {
    let _cancel_queued = CancelQueuedWrap(job.abort_handle());
    match tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), &mut job).await {
        Ok(Ok(result)) if Instant::now() < deadline => result,
        Ok(Err(_)) => Err(crate::knowledge::data_source::CommandError::new(
            "transport_worker_lost",
            "transport worker stopped before replying",
        )),
        _ => {
            // The guard cancels queued work; a running result is retired.
            Err(crate::knowledge::data_source::CommandError::new(
                "transport_timeout",
                "transport wrap exceeded its two-second admission budget",
            ))
        }
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };

    struct Release(Option<mpsc::Sender<()>>);
    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    #[test]
    fn a_ready_plan_observed_after_the_deadline_is_refused() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let deadline = Instant::now();
        let job = runtime.spawn_blocking(|| Ok("completed plan"));
        let until = Instant::now() + Duration::from_secs(3);
        while !job.is_finished() && Instant::now() < until {
            std::thread::yield_now();
        }
        assert!(job.is_finished(), "fixture worker never completed");
        let result = runtime.block_on(bounded_job(deadline, job));
        runtime.shutdown_timeout(Duration::from_secs(3));
        assert_eq!(result.unwrap_err().code, "transport_timeout");
    }

    #[test]
    fn deadline_covers_the_worker_queue_and_cancels_undispatched_work() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release = Release(Some(release_tx));
        let held = runtime.spawn_blocking(move || {
            entered_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let result = runtime.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(3),
                bounded_wrap(Instant::now() + Duration::from_secs(2), move || {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }),
            )
            .await
        });
        drop(release);
        runtime.block_on(held).unwrap();
        runtime.shutdown_timeout(Duration::from_secs(3));
        let error = result
            .expect("blocking queue prevented the async deadline")
            .unwrap_err();
        assert_eq!(error.code, "transport_timeout");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "timed-out work ran after capacity returned"
        );
    }

    #[test]
    fn deadline_retires_a_running_workers_late_result() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let release = Release(Some(release_tx));
        let waiting = runtime.spawn(bounded_wrap(
            Instant::now() + Duration::from_secs(2),
            move || {
                entered_tx.send(()).unwrap();
                let _ = release_rx.recv();
                let _ = finished_tx.send(());
                Ok("late plan")
            },
        ));
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let result =
            runtime.block_on(async { tokio::time::timeout(Duration::from_secs(3), waiting).await });
        drop(release);
        finished_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        runtime.shutdown_timeout(Duration::from_secs(3));
        assert_eq!(
            result.unwrap().unwrap().unwrap_err().code,
            "transport_timeout"
        );
    }
}
