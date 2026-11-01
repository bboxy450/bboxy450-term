// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! SDK transport through the real owner/supervisor boundary, without network.
#![allow(clippy::disallowed_methods)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::TestAccount,
    logic::{Logic, LogicClient, LogicConfig},
    mcp::{Door, Origin},
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::SeatRecord,
};
use bbt_proto::gui::PluginRequest;
use serde_json::json;
use std::{
    fs,
    time::{Duration, Instant},
};
async fn plugin(client: &LogicClient, request: PluginRequest) -> Vec<bbt_plugin::registry::Entry> {
    client
        .gui_plugin(
            Ulid::new(),
            request,
            Origin {
                door: Door::Ctl,
                issued_by: None,
            },
        )
        .await
        .unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sdk_transport_fields_reach_real_argv_and_stale_wraps_never_launch() {
    if std::env::var_os("BBOXY_TRANSPORT_CHILD").is_none() {
        let example = tokio::task::spawn_blocking(|| {
            bbt_plugin_fixture::sdk_example("bboxy450-term-transport-example")
        })
        .await
        .unwrap();
        let shape = std::process::Command::new(&example)
            .arg("--shape")
            .output()
            .unwrap();
        let mut shape: serde_json::Value = serde_json::from_slice(&shape.stdout).unwrap();
        shape["spawn_fields"][1]["name"] = json!("prefix");
        let malformed = tokio::task::spawn_blocking(move || {
            bbt_plugin_fixture::Knobs {
                descriptor: Some(shape),
                marker: Some("malformed-inspected".into()),
                ..bbt_plugin_fixture::Knobs::of(bbt_plugin_fixture::Kind::Inspect)
            }
            .build()
        })
        .await
        .unwrap();
        let private = tempfile::tempdir().unwrap();
        let home = private.path().join("home");
        fs::create_dir(&home).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sdk_transport_fields_reach_real_argv_and_stale_wraps_never_launch",
                "--nocapture",
            ])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("ZDOTDIR", &home)
            .env("BBOXY_TRANSPORT_CHILD", "1")
            .env("BBOXY_TRANSPORT_EXAMPLE", example)
            .env("BBOXY_TRANSPORT_MALFORMED", malformed)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    bbt_mux::plugins::set_test_budget(Duration::from_secs(30));
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let root = repo.join(".bboxy");
    fs::create_dir_all(root.join("plugins")).unwrap();
    let example = std::path::PathBuf::from(std::env::var_os("BBOXY_TRANSPORT_EXAMPLE").unwrap());
    let source = root.join("plugins/bboxy450-term-transport-example");
    fs::copy(&example, &source).unwrap();
    let account = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run,
            repository: repo.clone(),
            ring: Default::default(),
        },
        account.key_store(),
        HostConfig {
            registry: RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
            provider: ProviderConfig {
                timeout: Duration::from_secs(30),
            },
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let swarm = Ulid::new();
    let initial = Ulid::new();
    let record = SeatRecord::new(initial, swarm, "transport anchor", "zsh");
    let mut launch = LaunchSpec::new("/bin/cat", &repo);
    launch.seat = initial;
    launch.generation = 1;
    logic.client.spawn(record, launch).await.unwrap();
    let trusted = plugin(
        &logic.client,
        PluginRequest::Trust {
            path: source.to_string_lossy().into_owned(),
            sha256: None,
            grants: None,
            tier: None,
        },
    )
    .await;
    let candidate = trusted[0].candidate.id.clone();
    let checked = plugin(
        &logic.client,
        PluginRequest::Check {
            candidate: candidate.clone(),
        },
    )
    .await;
    assert!(checked[0].conformance.as_ref().unwrap().ok, "{checked:?}");
    plugin(
        &logic.client,
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    )
    .await;
    let output = repo.join("transport-output");
    let sentinel = repo.join("must-not-expand");
    let literal = format!("two words ; $(touch {}) 'quote'", sentinel.display());
    let spec = |delay: &str| bbt_proto::SpawnSpec {
        swarm: bbt_proto::SwarmId::from_u128(swarm.0),
        harness_kind: "zsh".into(),
        name: Some("wrapped".into()),
        role: None,
        cwd: repo.to_string_lossy().into_owned(),
        via: Some(bbt_proto::TransportSpec {
            kind: "plugin:example".into(),
            route: String::new(),
            port: None,
            key: None,
            fields: std::collections::BTreeMap::from([
                ("prefix".into(), literal.clone()),
                ("delay_ms".into(), delay.into()),
            ]),
        }),
        location: bbt_proto::Where::Local,
        resume: None,
        placement: None,
        args: vec![
            "-c".into(),
            format!(
                "printf '%s' \"$FIXTURE_PREFIX\" > '{}'; sleep 0.2",
                output.display()
            ),
        ],
        command: Some("/bin/sh".into()),
        env: vec![],
    };
    let slow = spec("1000");
    let client = logic.client.clone();
    let operation = Ulid::new();
    let waiting = tokio::spawn(async move { client.gui_spawn(operation, slow, Door::Ctl).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    if waiting.is_finished() {
        panic!(
            "fixture did not enter held wrap: {:?}",
            waiting.await.unwrap()
        );
    }
    let start = Instant::now();
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(300), logic.client.gui_seats())
            .await
            .expect("wrap stalled logic")
            .unwrap()
            .len(),
        1
    );
    assert!(
        start.elapsed() < Duration::from_millis(300),
        "wrap stalled the logic owner"
    );
    plugin(
        &logic.client,
        PluginRequest::Disable {
            candidate: candidate.clone(),
        },
    )
    .await;
    assert!(
        waiting.await.unwrap().is_err(),
        "a disabled activation launched a stale wrap result"
    );
    assert!(!output.exists());
    plugin(
        &logic.client,
        PluginRequest::Enable {
            candidate: candidate.clone(),
        },
    )
    .await;
    let operation = Ulid::new();
    let good = spec("0");
    let seat = logic
        .client
        .gui_spawn(operation, good.clone(), Door::Ctl)
        .await
        .unwrap();
    assert_eq!(
        logic
            .client
            .gui_spawn(operation, good, Door::Ctl)
            .await
            .unwrap(),
        seat,
        "spawn retry ran again"
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !output.exists() && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(fs::read_to_string(&output).unwrap(), literal);
    assert!(!sentinel.exists(), "a field was shell-expanded");
    assert_eq!(
        logic.client.gui_seats().await.unwrap().len(),
        2,
        "duplicate launch"
    );
    // Explicit restart replays the captured, already-authorized argv, just as
    // built-in transports do; it never depends on a fresh plugin call.
    tokio::time::sleep(Duration::from_millis(300)).await;
    fs::remove_file(&output).unwrap();
    logic
        .client
        .ctl_restart(
            Ulid::from(seat.0.0),
            1,
            Ulid::new(),
            Origin {
                door: Door::Ctl,
                issued_by: None,
            },
        )
        .await
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !output.exists() && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        literal,
        "restart lost wrapped argv"
    );
    // Trust inspects and refuses malformed declared fields before enable/launch.
    let bad = root.join("plugins/bboxy450-term-transport-bad");
    fs::copy(std::env::var_os("BBOXY_TRANSPORT_MALFORMED").unwrap(), &bad).unwrap();
    let entries = plugin(
        &logic.client,
        PluginRequest::Trust {
            path: bad.to_string_lossy().into_owned(),
            sha256: None,
            grants: None,
            tier: None,
        },
    )
    .await;
    let bad = entries
        .iter()
        .find(|e| std::path::Path::new(&e.candidate.source) == bad.canonicalize().unwrap())
        .unwrap();
    assert_eq!(bad.state(), bbt_plugin::registry::State::Refused);
    assert_eq!(
        bad.activation
            .as_ref()
            .unwrap()
            .inspection
            .as_ref()
            .unwrap()
            .refusal
            .as_ref()
            .unwrap()
            .code,
        "invalid_transport_fields"
    );
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
