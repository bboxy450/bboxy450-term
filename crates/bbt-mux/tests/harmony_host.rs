// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "owned private accounts, files and child processes; no operator account"
)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::TestAccount,
    logic::{Logic, LogicClient, LogicConfig},
    mcp::{Call, Door, Origin},
    plugins::{HostConfig, ProviderConfig, RegistryConfig},
    registry::{Class, Registry, SeatRecord},
    store::sqlite::SqliteStore,
};
use bbt_plugin_fixture::Knobs;
use bbt_proto::gui::{KnowledgeRequest, KnowledgeStatus, PluginRequest, SwarmScope};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

fn origin() -> Origin {
    Origin {
        door: Door::Ctl,
        issued_by: None,
    }
}
async fn call(client: &LogicClient, seat: Ulid, tool: &str, args: Value) -> Value {
    let request = Call {
        op_id: Ulid::new(),
        tool: tool.into(),
        args,
    };
    invoke(client, seat, request).await
}
async fn invoke(client: &LogicClient, seat: Ulid, request: Call) -> Value {
    for _ in 0..100 {
        let result = client.control_call(seat, 1, request.clone()).await.unwrap();
        if result["structuredContent"]["error"]["code"] != "workspace_pending" {
            return result;
        }
        // This named condition admits no effect. Retain the exact operation,
        // as the production ctl workspace client does; never retry unknown effects.
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{}: workspace preparation did not finish", request.tool);
}
#[track_caller]
fn success(value: Value) -> Value {
    assert_eq!(value["isError"], false, "{value:#}");
    let text: Value = serde_json::from_str(value["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        text, value["structuredContent"],
        "MCP text and structured results must agree"
    );
    value["structuredContent"].clone()
}
async fn status(client: &LogicClient, scope: SwarmScope) -> KnowledgeStatus {
    for _ in 0..200 {
        match client
            .gui_knowledge_control_scoped(
                Ulid::new(),
                KnowledgeRequest::Status,
                Some(scope),
                origin(),
            )
            .await
        {
            Ok(status) => return status,
            Err(error) if error.to_string().contains("workspace_pending") => {
                tokio::time::sleep(Duration::from_millis(20)).await
            }
            Err(error) => panic!("{error}"),
        }
    }
    panic!("workspace mount did not finish")
}
fn fixture(path: &Path) {
    // The root fixture's Harmony variant: signed writer with commit, held and
    // slow calls keyed by marker files in its root, a signed sidecar, and a
    // distinct backing directory outside the fixture bootstrap that detects a
    // bundled fallback.
    Knobs {
        harmony: true,
        ..Knobs::root()
    }
    .install(path);
}
async fn spawn(client: &LogicClient, temp: &Path, id: Ulid, swarm: Ulid, oracle: bool) {
    let mut seat = SeatRecord::new(id, swarm, "private source fixture", "zsh");
    if oracle {
        seat.role.class = Class::Oracle;
        seat.role.name = "oracle".into();
    }
    let mut spec = LaunchSpec::new("/bin/cat", temp);
    spec.seat = id;
    spec.generation = 1;
    client.spawn(seat, spec).await.unwrap();
}

struct ReleaseFile(std::path::PathBuf);
impl Drop for ReleaseFile {
    fn drop(&mut self) {
        fs::write(&self.0, b"release").unwrap();
    }
}
async fn wait_file(path: &Path) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("held operation entered");
}
async fn assert_other_swarm_controls(
    client: &LogicClient,
    seat: Ulid,
    pending: Option<(bbt_proto::GuiAction, &Path, ReleaseFile)>,
) -> Option<Value> {
    use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatId};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let endpoint = client.gui_endpoint().await.unwrap();
    let (read, mut write) = tokio::net::UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut lines = BufReader::new(read).lines();
    let release = if let Some((action, entered, release)) = pending {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: endpoint.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        let marker = wait_file(entered);
        tokio::pin!(marker);
        loop {
            tokio::select! {
                _ = &mut marker => break,
                response = lines.next_line() => {
                    let pending = match serde_json::from_str::<GuiReply>(&response.unwrap().unwrap()).unwrap() {
                        GuiReply::Error { reason, .. } if reason.starts_with("workspace_pending:") => true,
                        GuiReply::Knowledge { response, .. } if response["Err"]["code"] == "workspace_pending" => true,
                        GuiReply::Error { reason, .. } => panic!("held operation refused before hook entry: {reason}"),
                        GuiReply::Knowledge { response, .. } => panic!("held operation completed before hook entry: {response}"),
                        _ => false,
                    };
                    if pending {
                        // A pending workspace admits no mutation. Retain the
                        // original operation and bytes, as the real CLI does.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        write.write_all(&bytes).await.unwrap();
                    }
                }
            }
        }
        Some(release)
    } else {
        None
    };
    let started = std::time::Instant::now();
    let input_id = Ulid::new();
    for action in [
        GuiAction::Input {
            seat: SeatId::from_u128(seat.0),
            generation: 1,
            id: input_id,
            bytes: format!("responsive-{input_id}\n").into_bytes(),
        },
        GuiAction::Resize {
            seat: SeatId::from_u128(seat.0),
            generation: 1,
            cols: 66,
            rows: 22,
        },
    ] {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: endpoint.capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
    }
    tokio::time::timeout(Duration::from_millis(500), async {
        let mut replies = 0;
        while replies < 2 {
            match serde_json::from_str::<GuiReply>(&lines.next_line().await.unwrap().unwrap())
                .unwrap()
            {
                GuiReply::Ok { .. } => {
                    replies += 1;
                    eprintln!(
                        "held effect control reply {replies} at {:?}",
                        started.elapsed()
                    );
                }
                GuiReply::Error { reason, .. } => panic!("control refused: {reason}"),
                _ => (),
            }
        }
        success(call(client, seat, "self_status", json!({})).await);
        eprintln!("held effect status at {:?}", started.elapsed());
    })
    .await
    .expect("another swarm Input/Resize/status remain live while held");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let snapshot = client.snapshot(seat, 1).await.unwrap();
            let text: String = snapshot
                .screen
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.c)
                .collect();
            if snapshot.screen.cols == 66
                && snapshot.screen.rows == 22
                && text.contains(&format!("responsive-{input_id}"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("actual PTY grid and output changed while held");
    eprintln!(
        "held effect: another swarm Input/Resize/status and actual PTY output completed in {:?}",
        started.elapsed()
    );
    if release.is_some() {
        drop(release);
        return Some(
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    match serde_json::from_str::<GuiReply>(
                        &lines.next_line().await.unwrap().unwrap(),
                    )
                    .unwrap()
                    {
                        GuiReply::Knowledge { response, .. } => {
                            return serde_json::from_value::<
                                bbt_mux::knowledge::data_source::Result<Value>,
                            >(response)
                            .unwrap()
                            .unwrap();
                        }
                        GuiReply::Error { reason, .. } => panic!("held commit refused: {reason}"),
                        _ => (),
                    }
                }
            })
            .await
            .unwrap(),
        );
    }
    None
}

// keeps: complete command schemas fit the existing registry/journal bounds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reference_provider_schemas_fit_a_multi_provider_inventory() {
    bbt_mux::plugins::set_test_budget(Duration::from_secs(10));
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let plugins = repo.join(".bboxy/plugins");
    fs::create_dir_all(&plugins).unwrap();
    let mut sources = Vec::new();
    for index in 0..3 {
        let source = plugins.join(format!("bboxy450-term-data_source-reference-{index}"));
        // Independent pinned candidates: each nonce is its own variant and digest.
        Knobs::reference().nonce(index).install(&source);
        sources.push(source.canonicalize().unwrap());
    }
    let account = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run,
            repository: repo,
            ring: Default::default(),
        },
        account.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    for source in &sources {
        let entries = logic
            .client
            .gui_plugin(
                Ulid::new(),
                PluginRequest::Trust {
                    path: source.to_str().unwrap().into(),
                    sha256: None,
                    grants: None,
                    tier: None,
                },
                origin(),
            )
            .await
            .unwrap();
        let entry = entries
            .iter()
            .find(|entry| {
                entry
                    .trust
                    .as_ref()
                    .is_some_and(|trust| trust.source == source.to_str().unwrap())
            })
            .unwrap();
        let checked = logic
            .client
            .gui_plugin(
                Ulid::new(),
                PluginRequest::Check {
                    candidate: entry.candidate.id.clone(),
                },
                origin(),
            )
            .await
            .unwrap();
        assert!(
            checked
                .iter()
                .any(|entry| { entry.conformance.as_ref().is_some_and(|report| report.ok) })
        );
    }
    let entries = logic
        .client
        .gui_plugin(Ulid::new(), PluginRequest::List, origin())
        .await
        .unwrap();
    assert_eq!(entries.len(), 3);
    let bytes = serde_json::to_vec(&entries).unwrap().len();
    eprintln!(
        "reference provider registry: {} entries, {bytes} bytes",
        entries.len()
    );
    // Successful trust/check/list also traversed the unchanged encoded journal bound.
    assert!(bytes <= 256 * 1024);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

// keeps: explicit document commits cannot stop another swarm's process controls.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_private_git_hook_keeps_other_swarm_input_resize_and_status_live() {
    use std::process::Command;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    let other_repo = temp.path().join("other");
    fs::create_dir_all(repo.join(".bboxy")).unwrap();
    fs::create_dir_all(other_repo.join(".bboxy")).unwrap();
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .current_dir(&repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Private held hook"]);
    git(&["config", "user.email", "fixture@invalid.example"]);
    let key = temp.path().join("signing-key");
    assert!(
        Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f", key.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    git(&["config", "gpg.format", "ssh"]);
    git(&["config", "user.signingkey", key.to_str().unwrap()]);
    git(&["config", "commit.gpgsign", "true"]);
    fs::write(repo.join("baseline"), b"private").unwrap();
    git(&["add", "--", "baseline"]);
    git(&["commit", "-q", "-S", "-m", "private baseline"]);
    let run = temp.path().join("run");
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let registry = Registry::new(SqliteStore::open(&run.join("account")).unwrap());
    let swarm = Ulid::new();
    let other_swarm = Ulid::new();
    registry
        .update_swarm_workdir(other_swarm, Some(other_repo.to_str().unwrap()))
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run,
            repository: repo.clone(),
            ring: Default::default(),
        },
        custody.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let oracle = Ulid::new();
    let other = Ulid::new();
    spawn(&logic.client, temp.path(), oracle, swarm, true).await;
    spawn(&logic.client, temp.path(), other, other_swarm, true).await;
    success(call(&logic.client, oracle, "knowledge_write", json!({"path":"notes/held.md","frontmatter":{"type":"note","title":"hook"},"body":"private commit"})).await);
    let hook = repo.join(".git/hooks/pre-commit");
    fs::write(
        &hook,
        b"#!/bin/sh\ntouch hook-entered\nwhile [ ! -f hook-release ]; do sleep 0.01; done\n",
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    success(
        call(
            &logic.client,
            oracle,
            "knowledge_fetch",
            json!({"locator":"notes/held.md"}),
        )
        .await,
    );
    status(&logic.client, SwarmScope { swarm, revision: 0 }).await;
    let release = ReleaseFile(repo.join("hook-release"));
    let result = assert_other_swarm_controls(
        &logic.client,
        other,
        Some((
            bbt_proto::GuiAction::KnowledgeCommit {
                id: Ulid::new(),
                scope: Some(SwarmScope { swarm, revision: 0 }),
                locators: vec!["notes/held.md".into()],
                message: "explicit held commit".into(),
            },
            &repo.join("hook-entered"),
            release,
        )),
    )
    .await
    .unwrap();
    assert!(!result["commit"].as_str().unwrap().is_empty());
    let output = Command::new("git")
        .current_dir(&repo)
        .args(["cat-file", "commit", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("BEGIN SSH SIGNATURE"));
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

// keeps: #350 signed host authority, slug task lifecycle, distinct root/provider views and restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_source_owns_signed_documents_and_slug_task_lifecycle_without_bundled_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    for repo in [&a, &b] {
        fs::create_dir_all(repo.join(".bboxy/plugins")).unwrap();
        fixture(&repo.join(".bboxy/plugins/bboxy450-term-data_source-harmony"));
    }
    let swarm_a = Ulid::new();
    let swarm_b = Ulid::new();
    let oracle = Ulid::new();
    let worker = Ulid::new();
    let other = Ulid::new();
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let registry = Registry::new(SqliteStore::open(&run.join("account")).unwrap());
    let b_record = registry
        .update_swarm_workdir(swarm_b, Some(b.to_str().unwrap()))
        .unwrap();
    let scope_a = SwarmScope {
        swarm: swarm_a,
        revision: 0,
    };
    let scope_b = SwarmScope {
        swarm: swarm_b,
        revision: b_record.revision,
    };
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let config = LogicConfig {
        run_dir: run,
        repository: a.clone(),
        ring: Default::default(),
    };
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        config.clone(),
        custody.key_store(),
        HostConfig {
            registry: RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
            provider: ProviderConfig::default(),
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    spawn(&logic.client, temp.path(), oracle, swarm_a, true).await;
    spawn(&logic.client, temp.path(), worker, swarm_a, false).await;
    spawn(&logic.client, temp.path(), other, swarm_b, true).await;
    let seat_a = success(call(&logic.client, worker, "knowledge_status", json!({})).await);
    let seat_b = success(call(&logic.client, other, "knowledge_status", json!({})).await);
    assert_ne!(seat_a["knowledge_root_id"], seat_b["knowledge_root_id"]);
    assert_eq!(seat_a["swarm_workspace_id"], json!(swarm_a));
    assert_eq!(seat_b["swarm_workspace_id"], json!(swarm_b));
    let before = status(&logic.client, scope_a).await;
    let b_before = status(&logic.client, scope_b).await;
    assert_eq!(before.roots.len(), 1);
    assert_eq!(b_before.roots.len(), 1);
    assert_ne!(before.roots[0].id, b_before.roots[0].id);
    let entries = logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Trust {
                path: a
                    .join(".bboxy/plugins/bboxy450-term-data_source-harmony")
                    .to_str()
                    .unwrap()
                    .into(),
                sha256: None,
                grants: None,
                tier: None,
            },
            Some(scope_a),
            origin(),
        )
        .await
        .unwrap();
    let candidate = entries[0].candidate.id.clone();
    logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Check {
                candidate: candidate.clone(),
            },
            Some(scope_a),
            origin(),
        )
        .await
        .unwrap();
    logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: candidate.clone(),
            },
            Some(scope_a),
            origin(),
        )
        .await
        .unwrap();
    let offered = status(&logic.client, scope_a).await;
    let choice = offered.offered.iter().find(|p| p.id == candidate).unwrap();
    logic
        .client
        .gui_knowledge_control_scoped(
            Ulid::new(),
            KnowledgeRequest::Select {
                root: offered.roots[0].id.clone(),
                provider: candidate.clone(),
                revision: offered.roots[0].revision,
                sha256: choice.sha256.clone(),
            },
            Some(scope_a),
            origin(),
        )
        .await
        .unwrap();
    let b_plugins = logic
        .client
        .gui_plugin_scoped(Ulid::new(), PluginRequest::List, Some(scope_b), origin())
        .await
        .unwrap();
    for (tool, args) in [
        ("task_import_context", json!({})),
        (
            "task_import",
            json!({"expected_root":a.join(".bboxy"),"manifest_path":a.join("missing-manifest.json"),"audit_path":a.join("missing-audit.json"),"source_dir":a,"bundle_sha256":"0".repeat(64),"owner_seat":oracle}),
        ),
    ] {
        let refused = call(&logic.client, oracle, tool, args).await;
        assert_eq!(refused["isError"], true, "{refused}");
        assert!(
            refused["structuredContent"]["error"]["message"]
                .as_str()
                .unwrap()
                .contains("unsupported_operation"),
            "{refused}"
        );
    }
    let unsupported = call(
        &logic.client,
        worker,
        "artifact_claim",
        json!({"kind":"file","ref":"not-a-produced-artifact","task":"not-a-task"}),
    )
    .await;
    assert_eq!(
        unsupported["structuredContent"]["error"]["code"], "unsupported_operation",
        "unsupported external claims must be refused before looking up a task"
    );
    assert!(
        b_plugins
            .iter()
            .all(|e| e.candidate.id != candidate && e.activation.is_none())
    );
    assert_eq!(
        status(&logic.client, scope_b).await.roots[0].provider.id,
        "bboxy450-term-data_source-bboxy"
    );
    let made=success(call(&logic.client,oracle,"knowledge_write",json!({"path":"notes/same.md","frontmatter":{"type":"note","title":"source A"},"body":"A body"})).await);
    let first = made["record"].clone();
    let reference = json!({"id":first["id"],"number":first["number"],"slug":first["slug"],"path":first["path"],"digest":first["digest"]});
    let changed=success(call(&logic.client,oracle,"knowledge_write",json!({"ref":reference,"etag":first["etag"],"frontmatter":first["frontmatter"],"body":"A changed"})).await);
    assert_eq!(changed["record"]["body"], "A changed");
    // Hold the real stdio provider after effect dispatch. The other workspace's
    // native input/resize/status must finish before releasing the provider.
    fs::write(a.join("external/hold-write"), b"held").unwrap();
    let release = ReleaseFile(a.join("external/release-write"));
    let write_call = Call {
        op_id: Ulid::new(),
        tool: "knowledge_write".into(),
        args: json!({"path":"notes/held.md","frontmatter":{"type":"note","title":"held provider"},"body":"released"}),
    };
    let pending_call = write_call.clone();
    let client = logic.client.clone();
    let pending = tokio::spawn(async move { invoke(&client, oracle, pending_call).await });
    wait_file(&a.join("external/write-entered")).await;
    let replay = logic
        .client
        .control_call(oracle, 1, write_call.clone())
        .await
        .unwrap();
    assert_eq!(
        replay["structuredContent"]["error"]["code"], "unknown",
        "in-flight operation never redispatched"
    );
    let refused = call(&logic.client, oracle, "knowledge_write", json!({"path":"notes/not-admitted.md","frontmatter":{"type":"note","title":"occupied"},"body":""})).await;
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "provider_busy"
    );
    assert_other_swarm_controls(&logic.client, other, None).await;
    drop(release);
    success(pending.await.unwrap());
    assert!(!a.join("external/notes/not-admitted.md").exists());
    fs::remove_file(a.join("external/hold-write")).unwrap();

    // The optional commit gets 15 s; ordinary calls keep the 2 s production cap.
    fs::write(a.join("external/hold-commit"), b"held").unwrap();
    let release = ReleaseFile(a.join("external/release-commit"));
    let commit_call = Call {
        op_id: Ulid::new(),
        tool: "knowledge_commit".into(),
        args: json!({"locators":["same"],"message":"private method budget"}),
    };
    let client = logic.client.clone();
    let pending_call = commit_call.clone();
    let pending = tokio::spawn(async move { invoke(&client, oracle, pending_call).await });
    wait_file(&a.join("external/commit-entered")).await;
    let held = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(2300)).await;
    assert!(
        !pending.is_finished(),
        "commit must remain admitted beyond the ordinary 2 s budget"
    );
    let replay = invoke(&logic.client, oracle, commit_call).await;
    assert_eq!(replay["structuredContent"]["error"]["code"], "unknown");
    assert_other_swarm_controls(&logic.client, other, None).await;
    drop(release);
    assert_eq!(
        success(pending.await.unwrap())["commit"],
        "private-budget-fixture"
    );
    assert_eq!(
        fs::read_to_string(a.join("external/commit-attempts")).unwrap(),
        "commit\n"
    );
    eprintln!(
        "optional external commit completed after {:?}, once, while other swarm stayed live",
        held.elapsed()
    );
    fs::remove_file(a.join("external/hold-commit")).unwrap();
    let reference = logic
        .client
        .gui_knowledge_scoped("resolve".into(), json!({"locator":"same"}), Some(scope_a))
        .await
        .unwrap()
        .unwrap();
    fs::write(a.join("external/slow-fetch"), b"held").unwrap();
    let fetched = logic
        .client
        .gui_knowledge_scoped("fetch".into(), json!({"ref":reference}), Some(scope_a))
        .await
        .unwrap();
    assert_eq!(
        fetched.unwrap_err().code,
        "deadline_exceeded",
        "ordinary fetch must retain 2 s cap"
    );
    assert!(a.join("external/fetch-entered").exists());
    fs::remove_file(a.join("external/slow-fetch")).unwrap();

    let mut locators = Vec::new();
    for i in 0..10 {
        let path = format!("notes/preflight-{i}.md");
        success(call(&logic.client, oracle, "knowledge_write", json!({"path":path,"frontmatter":{"type":"note","title":"preflight budget"},"body":"private"})).await);
        locators.push(path);
    }
    fs::write(a.join("external/slow-preflight"), b"slow").unwrap();
    let started = std::time::Instant::now();
    let expired = call(
        &logic.client,
        oracle,
        "knowledge_commit",
        json!({"locators":locators,"message":"preflight exhausts whole budget"}),
    )
    .await;
    assert_eq!(
        expired["structuredContent"]["error"]["code"], "deadline_exceeded",
        "{expired}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(17),
        "{:?}",
        started.elapsed()
    );
    assert!(a.join("external/preflight-entered").exists());
    assert_eq!(
        fs::read_to_string(a.join("external/commit-attempts")).unwrap(),
        "commit\n",
        "exhausted preflight must not dispatch a second commit"
    );
    fs::remove_file(a.join("external/slow-preflight")).unwrap();
    eprintln!(
        "external commit preflight shared deadline: {:?}; no commit dispatched",
        started.elapsed()
    );

    assert!(!a.join(".bboxy/notes/same.md").exists());
    success(call(&logic.client,other,"knowledge_write",json!({"path":"notes/same.md","frontmatter":{"type":"note","title":"source B"},"body":"B body"})).await);
    assert_eq!(
        success(
            call(
                &logic.client,
                oracle,
                "knowledge_fetch",
                json!({"locator":"same"})
            )
            .await
        )["body"],
        "A changed"
    );
    assert_eq!(
        success(
            call(
                &logic.client,
                other,
                "knowledge_fetch",
                json!({"locator":"same"})
            )
            .await
        )["body"],
        "B body"
    );
    let task=success(call(&logic.client,oracle,"knowledge_write",json!({"path":"tasks/natural-slug.md","frontmatter":{"type":"task","title":"native task","verify":"fixture check","acceptance":["persisted"]},"body":"Intent: retain one backing source"})).await);
    assert!(
        task["record"]["number"].is_null(),
        "external global numbers may be absent"
    );
    fs::write(
        a.join("external/forbid-graph"),
        b"task reads must not scan the Markdown graph",
    )
    .unwrap();
    success(
        call(
            &logic.client,
            oracle,
            "task_delegate",
            json!({"task_path":"natural-slug","seat_id":worker}),
        )
        .await,
    );
    assert_eq!(
        call(
            &logic.client,
            other,
            "task_status",
            json!({"task_path":"natural-slug"})
        )
        .await["isError"],
        true
    );
    assert_eq!(
        call(
            &logic.client,
            worker,
            "task_accept",
            json!({"task_path":"natural-slug"})
        )
        .await["isError"],
        true
    );
    success(call(&logic.client,worker,"task_report",json!({"task_path":"natural-slug","summary":"failed","verify_result":{"command":"fixture check","exit_code":1,"evidence":[{"criterion":"persisted","reference":"native source"}]}})).await);
    assert_eq!(
        call(
            &logic.client,
            oracle,
            "task_accept",
            json!({"task_path":"natural-slug"})
        )
        .await["isError"],
        true
    );
    success(call(&logic.client,worker,"task_report",json!({"task_path":"natural-slug","summary":"stored","verify_result":{"command":"fixture check","exit_code":0,"evidence":[{"criterion":"persisted","reference":"native source"}]}})).await);
    success(
        call(
            &logic.client,
            oracle,
            "task_accept",
            json!({"task_path":"natural-slug"}),
        )
        .await,
    );
    let accepted = success(
        call(
            &logic.client,
            oracle,
            "knowledge_fetch",
            json!({"locator":"natural-slug"}),
        )
        .await,
    );
    assert_eq!(accepted["frontmatter"]["status"], "accepted");
    let proof: Value = serde_json::from_str(
        accepted["frontmatter"]["bboxy_signed_record"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        proof["payload"]["request"]["frontmatter"]["status"],
        "accepted"
    );
    assert_eq!(
        proof["payload"]["authorship"]["signer"],
        format!("seat:{oracle}")
    );
    assert!(
        proof["payload"]["request"]["frontmatter"]
            .get("bboxy_signed_record")
            .is_none()
    );
    assert!(!a.join(".bboxy/tasks/natural-slug.md").exists());
    assert_eq!(
        success(call(&logic.client, oracle, "task_list", json!({})).await)["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        success(call(&logic.client, other, "task_list", json!({})).await)["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        success(
            call(
                &logic.client,
                oracle,
                "knowledge_search",
                json!({"query":"A changed"})
            )
            .await
        )["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::remove_file(a.join("external/forbid-graph")).unwrap();
    assert!(
        !success(call(&logic.client, oracle, "graph_read", json!({"graph":"task"})).await)["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    logic.shutdown().unwrap();
    let restarted = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        config,
        custody.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert_eq!(
        status(&restarted.client, scope_a).await.roots[0]
            .provider
            .id,
        candidate
    );
    assert_eq!(
        status(&restarted.client, scope_b).await.roots[0]
            .provider
            .id,
        "bboxy450-term-data_source-bboxy"
    );
    restarted.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

// keeps: explicit knowledge commit — new paths only, signed commit, unrelated index preserved.
#[test]
fn local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks() {
    use bbt_mux::{
        index::Index,
        knowledge::{BboxySource, DataSource, WriteContext, data_source as d},
    };
    use std::process::Command;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| -> String {
        let result = Command::new("git")
            .current_dir(&repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Private fixture"]);
    git(&["config", "user.email", "fixture@invalid.example"]);
    let key = temp.path().join("git-signing-key");
    let result = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f", key.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(result.status.success());
    git(&["config", "gpg.format", "ssh"]);
    git(&["config", "user.signingkey", key.to_str().unwrap()]);
    git(&["config", "commit.gpgsign", "true"]);
    fs::write(repo.join("unrelated.txt"), "baseline").unwrap();
    git(&["add", "--", "unrelated.txt"]);
    git(&["commit", "-q", "-S", "-m", "fixture baseline"]);
    fs::write(
        repo.join("unrelated.txt"),
        "already staged by another operation",
    )
    .unwrap();
    git(&["add", "--", "unrelated.txt"]);
    let index = Index::open(
        &repo.join(".bboxy"),
        SqliteStore::open(&temp.path().join("index")).unwrap(),
    )
    .unwrap();
    let source = BboxySource::new(&index);
    let signing = ed25519_dalek::SigningKey::from_bytes(&[19; 32]);
    let context = WriteContext::seat(Ulid::new(), Some(1), &signing);
    let create = |name: &str| {
        source
            .write(
                d::WriteRequest {
                    reference: None,
                    path: Some(format!("notes/{name}.md")),
                    frontmatter: json!({"id":Ulid::new(),"type":"note","title":name}),
                    body: format!("{name} body"),
                    etag: None,
                },
                &context,
            )
            .unwrap()
            .record
            .reference
    };
    let one = create("one[ab]");
    let _decoy = create("onea");
    let two = create("two");
    let request = d::CommitRequest {
        refs: vec![one.clone()],
        message: "Commit one selected document".into(),
    };
    let committed = source.commit(request.clone(), &context).unwrap();
    assert_eq!(committed.committed, vec![one.clone()]);
    assert_eq!(
        git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]).trim(),
        ".bboxy/notes/one[ab].md"
    );
    assert_eq!(
        git(&["diff", "--cached", "--name-only"]).trim(),
        "unrelated.txt"
    );
    assert!(
        git(&[
            "status",
            "--porcelain",
            "--",
            &format!(".bboxy/{}", two.path)
        ])
        .starts_with("??")
    );
    assert!(git(&["cat-file", "-p", "HEAD"]).contains("gpgsig -----BEGIN SSH SIGNATURE-----"));
    assert_eq!(
        source.commit(request, &context).unwrap_err().code,
        "nothing_to_commit"
    );
    assert_eq!(
        git(&["diff", "--cached", "--name-only"]).trim(),
        "unrelated.txt"
    );
    let before = git(&["rev-parse", "HEAD"]);
    std::os::unix::fs::symlink(
        repo.join("unrelated.txt"),
        repo.join(".bboxy/notes/escape.md"),
    )
    .unwrap();
    let mut forged = one;
    forged.path = "notes/escape.md".into();
    assert!(
        source
            .commit(
                d::CommitRequest {
                    refs: vec![forged],
                    message: "refused".into()
                },
                &context
            )
            .is_err()
    );
    assert_eq!(git(&["rev-parse", "HEAD"]), before);
    let hook = repo.join(".git/hooks/pre-commit");
    fs::write(
        &hook,
        b"#!/bin/sh\nprintf 'called\\n' >> hook-calls\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let error = source
        .commit(
            d::CommitRequest {
                refs: vec![two],
                message: "Rejected by private hook".into(),
            },
            &context,
        )
        .unwrap_err();
    assert_eq!(error.code, "commit_not_confirmed");
    assert_eq!(git(&["rev-parse", "HEAD"]), before);
    assert_eq!(
        fs::read_to_string(repo.join("hook-calls")).unwrap(),
        "called\n"
    );
    let staged = git(&["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged.lines().collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([".bboxy/notes/two.md", "unrelated.txt"])
    );

    // A 6 s clean filter plus 12 s hook must share one 15 s operation budget.
    git(&["config", "core.hooksPath", ".git/hooks"]);
    git(&["config", "filter.slow.clean", "sh .git/slow-filter"]);
    fs::write(
        repo.join(".gitattributes"),
        ".bboxy/notes/cumulative.md filter=slow\n",
    )
    .unwrap();
    fs::write(
        repo.join(".git/slow-filter"),
        b"#!/bin/sh\nif [ ! -f filter-once ]; then touch filter-once; sleep 6; fi\ncat\n",
    )
    .unwrap();
    fs::write(&hook, b"#!/bin/sh\nprintf '%s\\n' $$ > deadline-hook-pid\ntouch deadline-hook-entered\nsleep 12\ntouch deadline-hook-finished\n").unwrap();
    let cumulative = create("cumulative");
    let started = std::time::Instant::now();
    let error = source
        .commit(
            d::CommitRequest {
                refs: vec![cumulative],
                message: "shared deadline".into(),
            },
            &context,
        )
        .unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(error.code, "commit_not_confirmed");
    assert!(error.message.contains("deadline"), "{error:?}");
    assert!(
        elapsed < Duration::from_secs(17),
        "subprocesses reset the operation budget: {elapsed:?}"
    );
    assert!(repo.join("filter-once").exists());
    assert!(repo.join("deadline-hook-entered").exists());
    assert!(!repo.join("deadline-hook-finished").exists());
    let hook_pid: i32 = fs::read_to_string(repo.join("deadline-hook-pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    for _ in 0..100 {
        if unsafe { libc::kill(hook_pid, 0) } == -1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        unsafe { libc::kill(hook_pid, 0) },
        -1,
        "owned Git hook survived deadline cleanup"
    );
    assert_eq!(git(&["rev-parse", "HEAD"]), before);
    assert!(
        git(&["diff", "--cached", "--name-only"])
            .lines()
            .any(|path| path == "unrelated.txt")
    );
    eprintln!(
        "shared local commit deadline: {elapsed:?}; {error}; hook group gone, HEAD/index preserved"
    );
}

// keeps: real external source — optional installed GitKB backend, private account and canonical CLI only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "explicit Make test-harmony-host-gitkb supplies a private launch HOME, artifact and backend"]
async fn real_gitkb_host_lifecycle() {
    use bbt_mux::knowledge::data_source as d;
    use std::{path::PathBuf, process::Command};
    // The launcher owns this fresh root. Align the TestAccount's existing child
    // HOME with the libtest HOME before any Git or native backend can run.
    let private_root = PathBuf::from(
        std::env::var_os("BBOXY_GITKB_TEST_ROOT")
            .expect("Make must supply BBOXY_GITKB_TEST_ROOT and a private launch HOME"),
    )
    .canonicalize()
    .unwrap();
    let temp_parent = std::env::temp_dir().canonicalize().unwrap();
    assert!(private_root != temp_parent && private_root.starts_with(&temp_parent));
    let run = private_root.join("run");
    let account_dir = run.join("account");
    let home = PathBuf::from(std::env::var_os("HOME").expect("private launch HOME"))
        .canonicalize()
        .unwrap();
    assert!(
        home == account_dir.join("plugin-home"),
        "private launch HOME must match the TestAccount provider HOME"
    );
    for directory in [&private_root, &run, &account_dir, &home] {
        let metadata = fs::symlink_metadata(directory).unwrap();
        assert!(metadata.is_dir() && metadata.permissions().mode() & 0o077 == 0);
    }
    assert!(
        !account_dir.join("object.sqlite").exists(),
        "native host integration requires a fresh launch account"
    );
    fs::write(
        home.join(".gitconfig"),
        "[user]\n\tname = Private GitKB fixture\n\temail = fixture@invalid.example\n",
    )
    .unwrap();
    let artifact =
        std::env::var_os("BBOXY_GITKB_ADAPTER").expect("Make must supply BBOXY_GITKB_ADAPTER");
    let backend = std::env::var("BBOXY_GITKB_BIN")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "git-kb".into());
    let temp = tempfile::tempdir_in(&private_root).unwrap();
    let repo = temp.path().join("project");
    let native_temp = home.join("tmp");
    for path in [&repo, &native_temp] {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    eprintln!(
        "private native and provider HOME aligned: {}",
        home.display()
    );
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&repo)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("TMPDIR", &native_temp)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Private GitKB fixture"]);
    git(&["config", "user.email", "fixture@invalid.example"]);
    let output = Command::new(&backend)
        .current_dir(&repo)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &home)
        .env("TMPDIR", &native_temp)
        .env("GITKB_ROOT", &repo)
        .env("GITKB_WORKSPACE", "main")
        .args(["init", "--name", "Private bboxy host integration"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native init: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (key, value) in [
        ("user.name", "Private GitKB fixture"),
        ("user.email", "fixture@invalid.example"),
    ] {
        let output = Command::new(&backend)
            .current_dir(&repo)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("TMPDIR", &native_temp)
            .env("GITKB_ROOT", &repo)
            .env("GITKB_WORKSPACE", "main")
            .args(["config", "set", key, value])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "native config: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::create_dir_all(repo.join(".bboxy/plugins")).unwrap();
    let source = repo.join(".bboxy/plugins/bboxy450-term-data_source-gitkb");
    fs::copy(&artifact, &source).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    use sha2::{Digest, Sha256};
    let artifact_sha256 = format!("{:x}", Sha256::digest(fs::read(&source).unwrap()));
    eprintln!("real adapter sha256={artifact_sha256}");
    let descriptor = Command::new(&source)
        .arg("--shape")
        .env_clear()
        .current_dir(&private_root)
        .output()
        .unwrap();
    assert!(descriptor.status.success());
    let descriptor: Value = serde_json::from_slice(&descriptor.stdout).unwrap();
    let expected_skill = &descriptor["skills"][0];
    assert_eq!(expected_skill["name"], "gitkb");
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let config = LogicConfig {
        run_dir: run,
        repository: repo.clone(),
        ring: Default::default(),
    };
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        config.clone(),
        custody.key_store(),
        HostConfig {
            registry: RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(2),
            },
            provider: ProviderConfig::default(),
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let swarm = Ulid::new();
    let oracle = Ulid::new();
    let worker = Ulid::new();
    let scope = SwarmScope { swarm, revision: 0 };
    spawn(&logic.client, temp.path(), oracle, swarm, true).await;
    spawn(&logic.client, temp.path(), worker, swarm, false).await;
    let status_request = Call {
        op_id: Ulid::new(),
        tool: "knowledge_status".into(),
        args: json!({}),
    };
    let initial = success(invoke(&logic.client, worker, status_request.clone()).await);
    assert_eq!(
        initial["status"]["roots"][0]["provider"]["id"],
        "bboxy450-term-data_source-bboxy"
    );
    assert_eq!(initial["plugins"][0]["state"], "untrusted");
    assert_eq!(initial["plugins"][0]["checked"], false);
    let entries = logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Trust {
                path: source.to_str().unwrap().into(),
                sha256: None,
                grants: None,
                tier: None,
            },
            Some(scope),
            origin(),
        )
        .await
        .unwrap();
    let candidate = entries[0].candidate.id.clone();
    let checked = logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Check {
                candidate: candidate.clone(),
            },
            Some(scope),
            origin(),
        )
        .await
        .unwrap();
    assert!(checked[0].conformance.as_ref().unwrap().ok);
    logic
        .client
        .gui_plugin_scoped(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: candidate.clone(),
            },
            Some(scope),
            origin(),
        )
        .await
        .unwrap();
    let before = status(&logic.client, scope).await;
    let choice = before.offered.iter().find(|p| p.id == candidate).unwrap();
    assert_eq!(
        success(invoke(&logic.client, worker, status_request).await),
        initial,
        "an explicit operation replay keeps its original snapshot"
    );
    let enabled = success(call(&logic.client, worker, "knowledge_status", json!({})).await);
    assert_eq!(
        enabled["status"]["roots"][0]["provider"]["id"],
        "bboxy450-term-data_source-bboxy"
    );
    assert_eq!(enabled["plugins"][0]["state"], "enabled");
    assert_eq!(enabled["plugins"][0]["checked"], true);
    let unselected = success(call(&logic.client, worker, "skill_list", json!({})).await);
    assert!(unselected["provider_error"].is_null());
    let unselected = unselected["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|skill| skill["name"] == "gitkb")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        unselected.len(),
        1,
        "an enabled, checked plugin supplies its instructions without selection"
    );
    assert_eq!(unselected[0]["origin"], candidate);
    assert_eq!(unselected[0]["plugin_sha256"], artifact_sha256);
    logic
        .client
        .gui_knowledge_control_scoped(
            Ulid::new(),
            KnowledgeRequest::Select {
                root: before.roots[0].id.clone(),
                provider: candidate.clone(),
                revision: before.roots[0].revision,
                sha256: choice.sha256.clone(),
            },
            Some(scope),
            origin(),
        )
        .await
        .unwrap();
    let created=success(call(&logic.client,oracle,"knowledge_write",json!({"path":"notes/private-custom.md","frontmatter":{"type":"project-brief","title":"Private custom type"},"body":"Native custom document"})).await)["record"].clone();
    let active = success(call(&logic.client, worker, "knowledge_status", json!({})).await);
    assert_eq!(active["status"]["roots"][0]["provider"]["id"], candidate);
    assert_eq!(
        active["status"]["roots"][0]["provider"]["sha256"],
        artifact_sha256
    );
    assert_eq!(active["knowledge_root_id"], before.roots[0].id);
    assert_eq!(active["swarm_workspace_id"], json!(swarm));
    let skills = success(call(&logic.client, worker, "skill_list", json!({})).await);
    let gitkb_skills = skills["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|skill| skill["name"] == "gitkb")
        .collect::<Vec<_>>();
    assert_eq!(gitkb_skills.len(), 1);
    let skill = gitkb_skills[0];
    assert_eq!(skill["origin"], candidate);
    assert_eq!(skill["root_id"], before.roots[0].id);
    assert_eq!(skill["plugin_sha256"], artifact_sha256);
    assert_eq!(
        skill, &unselected[0],
        "selection does not change a plugin skill's identity"
    );
    let skill_uri = skill["uri"].clone();
    let instruction = success(
        call(
            &logic.client,
            worker,
            "skill_read",
            json!({"uri":skill_uri}),
        )
        .await,
    );
    assert_eq!(instruction["skill"]["content"], expected_skill["content"]);
    let reference: d::RecordRef = serde_json::from_value(created.clone()).unwrap();
    assert!(reference.external_id.is_some());
    assert_ne!(reference.id, reference.external_id.clone().unwrap());
    let mut front = created["frontmatter"].clone();
    front["title"] = json!("Updated native custom type");
    success(call(&logic.client,oracle,"knowledge_write",json!({"ref":reference,"etag":created["etag"],"frontmatter":front,"body":"Updated native document"})).await);
    let doc = success(
        call(
            &logic.client,
            oracle,
            "knowledge_fetch",
            json!({"locator":reference.slug}),
        )
        .await,
    );
    assert_eq!(doc["body"], "Updated native document");
    assert_eq!(doc["frontmatter"]["type"], "project-brief");
    assert_eq!(doc["id"], reference.id);
    assert_eq!(
        success(
            call(
                &logic.client,
                oracle,
                "knowledge_search",
                json!({"query":"Updated native"})
            )
            .await
        )["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let task=success(call(&logic.client,oracle,"knowledge_write",json!({"path":"tasks/private-lifecycle.md","frontmatter":{"type":"task","title":"Native lifecycle","verify":"private check","acceptance":["persisted"]},"body":"Explicit private task intent"})).await)["record"].clone();
    let slug = task["slug"].as_str().unwrap().to_owned();
    assert!(!slug.is_empty());
    success(
        call(
            &logic.client,
            oracle,
            "task_delegate",
            json!({"task_path":slug,"seat_id":worker}),
        )
        .await,
    );
    success(call(&logic.client,worker,"task_report",json!({"task_path":slug,"summary":"Native persisted","verify_result":{"command":"private check","exit_code":0,"evidence":[{"criterion":"persisted","reference":"canonical native workspace"}]}})).await);
    success(
        call(
            &logic.client,
            oracle,
            "task_accept",
            json!({"task_path":slug}),
        )
        .await,
    );
    let accepted = success(
        call(
            &logic.client,
            oracle,
            "task_status",
            json!({"task_path":slug}),
        )
        .await,
    );
    assert_eq!(accepted["metadata"]["status"], "accepted");
    let list_started = std::time::Instant::now();
    let assigned = success(call(&logic.client, worker, "task_list", json!({})).await);
    assert_eq!(assigned["items"].as_array().unwrap().len(), 1);
    eprintln!(
        "real GitKB scoped task_list elapsed={:?}",
        list_started.elapsed()
    );
    let commit = success(
        call(
            &logic.client,
            oracle,
            "knowledge_commit",
            json!({"locators":[reference.slug,slug],"message":"Private native lifecycle evidence"}),
        )
        .await,
    );
    assert!(
        commit["commit"]
            .as_str()
            .is_some_and(|s| s.len() == 36 && s.contains('-'))
    );
    assert_eq!(commit["committed"].as_array().unwrap().len(), 2);
    let committed_doc = success(
        call(
            &logic.client,
            oracle,
            "knowledge_fetch",
            json!({"locator":reference.slug}),
        )
        .await,
    );
    let committed_reference: d::RecordRef = serde_json::from_value(committed_doc.clone()).unwrap();
    assert_eq!(committed_reference.id, reference.id);
    assert_eq!(committed_reference.external_id, reference.external_id);
    assert_ne!(
        committed_doc["digest"], doc["digest"],
        "native commit changes the selected source generation"
    );
    let materialized = success(call(&logic.client, oracle, "knowledge_write", json!({"ref":committed_reference,"etag":committed_doc["etag"],"frontmatter":committed_doc["frontmatter"],"body":"Signed edit after native commit"})).await)["record"].clone();
    assert_eq!(materialized["id"], created["id"]);
    assert_eq!(materialized["external_id"], created["external_id"]);
    assert_eq!(materialized["frontmatter"]["type"], "project-brief");
    assert_eq!(materialized["body"], "Signed edit after native commit");
    assert!(!repo.join(".bboxy/notes/private-custom.md").exists());
    assert!(!repo.join(".bboxy/tasks/private-lifecycle.md").exists());
    assert!(
        !success(call(&logic.client, oracle, "graph_read", json!({"graph":"task"})).await)["nodes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    logic.shutdown().unwrap();
    let restarted = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        config,
        custody.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert_eq!(
        status(&restarted.client, scope).await.roots[0].provider.id,
        candidate
    );
    let resolved = restarted
        .client
        .gui_knowledge_scoped("resolve".into(), json!({"locator":slug}), Some(scope))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved["id"], task["id"]);
    let after_restart = restarted
        .client
        .gui_knowledge_scoped(
            "fetch".into(),
            json!({"ref":serde_json::from_value::<d::RecordRef>(materialized).unwrap()}),
            Some(scope),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_restart["body"], "Signed edit after native commit");
    let instruction = success(
        call(
            &restarted.client,
            worker,
            "skill_read",
            json!({"uri":skill_uri}),
        )
        .await,
    );
    assert_eq!(instruction["skill"]["content"], expected_skill["content"]);
    let mut changed = fs::read(&source).unwrap();
    changed.extend_from_slice(b"\n# private changed-source diagnostic\n");
    fs::write(&source, changed).unwrap();
    let degraded = success(call(&restarted.client, worker, "knowledge_status", json!({})).await);
    assert_eq!(degraded["status"]["roots"][0]["provider"]["id"], candidate);
    assert!(degraded["status"]["roots"][0]["refusal"].is_string());
    assert_eq!(degraded["plugins"][0]["state"], "needs_retrust");
    restarted.shutdown().unwrap();
    supervisor.shutdown().unwrap();
    eprintln!(
        "real GitKB host: trust/check/enable/select, pinned root-scoped skill reads, typed create/update, slug delegate/report/accept, native commit, signed post-commit edit, restart all completed"
    );
}
