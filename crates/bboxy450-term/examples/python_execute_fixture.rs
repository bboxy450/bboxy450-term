// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Unattended native bridge + ordinary harness Host fixture; never installed.
#![allow(
    clippy::disallowed_methods,
    reason = "private test process, never the UI"
)]
use bboxy450_harness::{
    engine::{Config, LaunchOptions},
    gateway::Host,
    mcp::ServerConfig,
    policy::Policy,
    protocol::*,
    provider::{ScriptedProvider, profile::Unattended},
    redact::Set,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

#[derive(Deserialize)]
struct Input {
    socket: PathBuf,
    seat: bbt_mux::Ulid,
    generation: u64,
    capability: [u8; 32],
    cwd: PathBuf,
    run_dir: PathBuf,
}

/// Hold one completed native mutation's acknowledgement on the real serial
/// Bridge::run stream. Its separate context lane must still serve get/cancel.
struct HeldReply<W> {
    inner: W,
    run: PathBuf,
}
impl<W: std::io::Write> std::io::Write for HeldReply<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.run.join("hold-native-write").is_file()
            && serde_json::from_slice::<serde_json::Value>(bytes).is_ok_and(|value| {
                value["result"]["structuredContent"]["record"]["path"] == "reports/python-held.md"
            })
        {
            std::fs::write(self.run.join("held-native-write"), b"acknowledgement held")?;
            while !self.run.join("release-native-write").exists() && !self.run.join("stop").exists()
            {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().ok_or("fixture mode required")?;
    let path = PathBuf::from(args.next().ok_or("private fixture input required")?);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("input must be a private owned regular file".into());
    }
    let input: Input = serde_json::from_slice(&std::fs::read(&path)?)?;
    if mode == "bridge" {
        if let Some(socket) = std::env::var_os("BBOXY_PYTHON_SOCKET") {
            std::fs::write(
                input.run_dir.join("runtime-socket"),
                socket.as_encoded_bytes(),
            )?;
        }
        bbt_mux::mcp::bridge::Bridge::new(bbt_mux::mcp::bridge::Config {
            socket: input.socket,
            seat: input.seat,
            generation: input.generation,
            capability: input.capability,
        })
        .run(
            &mut std::io::BufReader::new(std::io::stdin().lock()),
            &mut HeldReply {
                inner: std::io::stdout().lock(),
                run: input.run_dir,
            },
        )?;
    } else if mode == "host" {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?
            .block_on(async move {
                let session = SessionId::new();
                let meta = SessionMeta {
                    vcs: None,
                    dream: None,
                    source: None,
                    session,
                    cwd: input.cwd.clone(),
                    profile: None,
                    fork: None,
                    protocol: PROTOCOL,
                    harness: "private-python-fixture".into(),
                    created_ms: 0,
                };
                let host = Host::new(Config {
                    meta: meta.clone(),
                    sessions: input.run_dir.join("sessions"),
                    profiles: input.run_dir.join("profiles"),
                    provider: Arc::new(ScriptedProvider::new(vec![])),
                    model: "scripted".into(),
                    policy: Policy::allow_all(Unattended::Deny),
                    redactor: Set::default(),
                    max_rounds: 4,
                    profile_error: None,
                })
                .with_launch(LaunchOptions {
                    mcp_servers: [(
                        "native".into(),
                        ServerConfig {
                            command: std::env::current_exe()?.to_string_lossy().into_owned(),
                            args: vec!["bridge".into(), path.to_string_lossy().into_owned()],
                            env: BTreeMap::new(),
                            timeout_s: 5,
                            trust_hints: false,
                        },
                    )]
                    .into(),
                    ..LaunchOptions::default()
                });
                let _gateway = host
                    .create(OpEnvelope {
                        v: PROTOCOL,
                        session,
                        op_id: OpId::new(),
                        op: Op::StartSession {
                            cwd: meta.cwd,
                            profile: None,
                            fork: None,
                        },
                    })
                    .await
                    .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
                // This owned stop file gives the fixture a graceful close even when
                // its parent test unwinds; no model or cloud request ever runs.
                while !input.run_dir.join("stop").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                host.close_all().await;
                Ok::<_, Box<dyn std::error::Error>>(())
            })?;
    } else {
        return Err("unknown private fixture mode".into());
    }
    Ok(())
}
