// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private account/process fixtures, never UI IO"
)]
use super::*;
use crate::{Ulid, store::sqlite::SqliteStore};
use bbt_plugin::{contracts::Contract, registry::State};
use bbt_plugin_fixture::{Kind, Knobs};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    control: Control,
    journal: Journal,
    path: PathBuf,
    marker: PathBuf,
    knobs: Knobs,
}
impl Fixture {
    /// New bytes (a new digest) at the same path: the variant plus `nonce`.
    fn replace(&self, nonce: &str) {
        self.knobs.clone().nonce(nonce).install(&self.path);
    }

    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let plugins = root.path().join("repo/.bboxy/plugins");
        fs::create_dir_all(&plugins).unwrap();
        let path = plugins.join("bboxy450-term-repo_graph-stub");
        let marker = root.path().join("descriptor-count");
        let contract = Contract::RepoGraph;
        let shape = json!({"name":"private stub", "version":"1.0", "shape_version":"1.0", "health_command":"health",
            "systems":[{"contract":contract.name(),"contract_version":contract.version(),
            "commands":contract.methods().iter().map(|method| json!({"name":method,
                "args":contract.method_schema(method,false).unwrap(),"result":contract.method_schema(method,true).unwrap(),"timeout_ms":1000})).collect::<Vec<_>>()}]});
        // Prints the shape and marks `shape` once per descriptor run.
        let knobs = Knobs {
            descriptor: Some(shape),
            marker: Some(marker.to_string_lossy().into_owned()),
            ..Knobs::of(Kind::Inspect)
        };
        knobs.install(&path);
        let account = SqliteStore::open(&root.path().join("account")).unwrap();
        let registry = super::super::Registry::with_config(
            account.clone(),
            super::super::DiscoveryRoots::new(&root.path().join("app"), &root.path().join("repo")),
            super::super::RegistryConfig {
                descriptor_timeout: std::time::Duration::from_secs(30),
                conformance_timeout: std::time::Duration::from_secs(30),
            },
        )
        .unwrap();
        Self {
            control: Control::new(registry),
            journal: Journal::new(account),
            path,
            marker,
            knobs,
            _root: root,
        }
    }
    fn reference(fault: Option<&str>, gate: Option<&std::path::Path>) -> Self {
        let mut f = Self::new();
        let path = f.path.with_file_name("bboxy450-term-data_source-reference");
        fs::rename(&f.path, &path).unwrap();
        f.path = path;
        // Observes shape, initialize (asserting a disposable root apart from
        // its data dir and no host fields) and calls; `gate` holds initialize.
        let text = |path: &std::path::Path| path.to_string_lossy().into_owned();
        f.knobs = Knobs {
            fault: fault.map(str::to_owned),
            timeout_ms: Some(30_000),
            observe: Some(text(&f.marker)),
            initialize_gate: gate.map(text),
            ..Knobs::reference()
        };
        f.knobs.install(&f.path);
        f
    }

    fn observations(&self) -> Vec<serde_json::Value> {
        fs::read_to_string(&self.marker)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    async fn trust_reference(&self) -> Entry {
        self.ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: self.path.to_string_lossy().into_owned(),
                sha256: None,
                grants: None,
                tier: None,
            },
        )
        .await
        .unwrap()
        .remove(0)
    }

    async fn ask(&self, id: Ulid, request: PluginRequest) -> Result<Vec<Entry>> {
        self.control
            .request(
                Actor::Operator {
                    device_id: Ulid::from(91_u128),
                    door: mcp::Door::Ctl,
                    issued_by: None,
                },
                self.journal.clone(),
                Call {
                    op_id: id,
                    tool: "plugin_control".into(),
                    args: serde_json::to_value(&request).unwrap(),
                },
                request,
            )
            .await
    }
}

// keeps: plugins §1 trust-before-execution, disabled default, pinned digest and journal replay.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operator_controls_inspect_once_replay_and_refuse_a_changed_source() {
    let f = Fixture::new();
    let entry = f
        .ask(Ulid::new(), PluginRequest::List)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(entry.state(), State::Untrusted);
    assert!(!f.marker.exists());
    assert!(
        f.ask(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: entry.candidate.id.clone()
            }
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("untrusted")
    );
    let trust = PluginRequest::Trust {
        path: f.path.to_str().unwrap().into(),
        sha256: None,
        grants: None,
        tier: None,
    };
    let op = Ulid::new();
    let trusted = f.ask(op, trust.clone()).await.unwrap();
    assert_eq!(trusted[0].state(), State::Disabled);
    assert_eq!(f.ask(op, trust).await.unwrap(), trusted);
    assert_eq!(fs::read_to_string(&f.marker).unwrap(), "shape\n");
    let record = trusted[0].trust.as_ref().unwrap();
    assert_eq!(
        fs::metadata(&record.copy).unwrap().permissions().mode() & 0o222,
        0
    );
    let enabled = f
        .ask(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: entry.candidate.id.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(enabled[0].state(), State::Enabled);
    assert_eq!(
        f.ask(
            Ulid::new(),
            PluginRequest::Disable {
                candidate: entry.candidate.id.clone()
            }
        )
        .await
        .unwrap()[0]
            .state(),
        State::Disabled
    );
    fs::write(&f.path, "#!/bin/sh\nexit 41\n").unwrap();
    assert!(
        f.ask(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: entry.candidate.id
            }
        )
        .await
        .is_err()
    );
    assert_eq!(
        f.ask(Ulid::new(), PluginRequest::List).await.unwrap()[0].state(),
        State::NeedsRetrust
    );
    assert_eq!(fs::read_to_string(&f.marker).unwrap(), "shape\n");
}

// keeps: plugin trust is restricted to the discovered roots, including a refused symlink leaf.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn arbitrary_path_and_symlink_never_authorize_a_run() {
    let f = Fixture::new();
    let outside = f._root.path().join("outside");
    fs::copy(&f.path, &outside).unwrap();
    assert!(
        f.ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: outside.to_string_lossy().into_owned(),
                sha256: None,
                grants: None,
                tier: None,
            }
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("plugin_not_discovered")
    );
    let linked = f.path.with_file_name("bboxy450-term-repo_graph-linked");
    std::os::unix::fs::symlink(&outside, &linked).unwrap();
    assert!(
        f.ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: linked.to_string_lossy().into_owned(),
                sha256: None,
                grants: None,
                tier: None,
            }
        )
        .await
        .is_err()
    );
    assert!(!f.marker.exists());
}

// keeps: a Settings trust act approves the digest it displayed, never a replacement binary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_digest_change_is_refused_before_trust_or_inspection() {
    let f = Fixture::new();
    let entry = f
        .ask(Ulid::new(), PluginRequest::List)
        .await
        .unwrap()
        .remove(0);
    f.replace("replacement");
    let error = f
        .ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: f.path.to_string_lossy().into_owned(),
                sha256: entry.candidate.sha256,
                grants: None,
                tier: None,
            },
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("plugin_changed_since_observed"));
    assert!(!f.marker.exists());
    assert_eq!(
        f.ask(Ulid::new(), PluginRequest::List).await.unwrap()[0].state(),
        State::Untrusted
    );
}

// keeps: a blocked plugin descriptor never serializes inventory or another plugin's trust.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_descriptor_leaves_inventory_and_another_plugin_independent() {
    let f = std::sync::Arc::new(Fixture::new());
    let socket = f._root.path().join("gate.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let other = f.path.with_file_name("bboxy450-term-repo_graph-other");
    f.knobs.install(&other);
    // The held plugin waits on the socket before printing its shape.
    Knobs {
        initialize_gate: Some(socket.to_string_lossy().into_owned()),
        ..f.knobs.clone()
    }
    .install(&f.path);
    let task = tokio::spawn({
        let f = f.clone();
        async move {
            f.ask(
                Ulid::new(),
                PluginRequest::Trust {
                    path: f.path.to_string_lossy().into_owned(),
                    sha256: None,
                    grants: None,
                    tier: None,
                },
            )
            .await
        }
    });
    let (mut gate, _) = tokio::time::timeout(std::time::Duration::from_secs(30), listener.accept())
        .await
        .unwrap()
        .unwrap();
    // These complete before release; no machine-speed assertion or short sleep.
    let entries = f.ask(Ulid::new(), PluginRequest::List).await.unwrap();
    assert_eq!(entries.len(), 2);
    let other = f
        .ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: other.to_string_lossy().into_owned(),
                sha256: None,
                grants: None,
                tier: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(other[0].state(), State::Disabled);
    use tokio::io::AsyncWriteExt;
    gate.write_all(b"x").await.unwrap();
    assert_eq!(task.await.unwrap().unwrap()[0].state(), State::Disabled);
}

// keeps: plugin-host — checks use trusted bytes, isolated state and root-at-initialize; no implicit enable.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conformance_is_explicit_isolated_digest_bound_and_replayed_without_execution() {
    let f = Fixture::reference(None, None);
    let entry = f
        .ask(Ulid::new(), PluginRequest::List)
        .await
        .unwrap()
        .remove(0);
    let request = PluginRequest::Check {
        candidate: entry.candidate.id.clone(),
    };
    assert!(
        f.ask(Ulid::new(), request.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("untrusted_plugin")
    );
    let standalone = crate::plugins::check(&f.path).await;
    assert_eq!(
        standalone.refusal.unwrap().code,
        "untrusted_production_file"
    );
    assert!(!f.marker.exists());
    let before = f.trust_reference().await;
    let op = Ulid::new();
    let checked = f.ask(op, request.clone()).await.unwrap().remove(0);
    assert_eq!(checked.state(), State::Disabled);
    assert_eq!(checked.activation, before.activation);
    assert_eq!(checked.trust, before.trust);
    let result = checked.conformance.as_ref().unwrap();
    assert!(result.ok, "{result:?}");
    assert_eq!(result.sha256, before.activation.as_ref().unwrap().sha256);
    assert!(
        result
            .checks
            .iter()
            .any(|check| check.contains("eight data_source methods"))
    );
    let observations = f.observations();
    assert_eq!(
        observations
            .iter()
            .filter(|row| row["kind"] == "shape")
            .count(),
        1
    );
    let init = observations
        .iter()
        .find(|row| row["kind"] == "initialize")
        .unwrap();
    for field in ["root", "cwd"] {
        assert!(!std::path::Path::new(init[field].as_str().unwrap()).starts_with(f._root.path()));
    }
    let pid = init["pid"].as_u64().unwrap() as libc::pid_t;
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "checker child still exists"
    );
    assert_eq!(
        f.ask(op, request.clone()).await.unwrap(),
        vec![checked.clone()]
    );
    assert_eq!(f.ask(Ulid::new(), request).await.unwrap(), vec![checked]);
    assert_eq!(
        f.observations(),
        observations,
        "replay or cached success executed a child"
    );
}

// keeps: plugin-host — canonical enrichment is lossless; both write probes require disclosure of actual conversion loss.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conformance_allows_metadata_enrichment_and_requires_reported_conversion_loss() {
    for (fault, refused) in [
        ("write_enrichment", false),
        ("write_drop_title", true),
        ("write_drop_title_minor", true),
        ("write_change_body", true),
        ("write_reported_loss", false),
        ("write_identity_guard", false),
    ] {
        let f = Fixture::reference(Some(fault), None);
        let trusted = f.trust_reference().await;
        let result = f
            .ask(
                Ulid::new(),
                PluginRequest::Check {
                    candidate: trusted.candidate.id.clone(),
                },
            )
            .await;
        let checked = if refused {
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("unreported_conversion_loss"),
                "{fault}: {error}"
            );
            f.ask(Ulid::new(), PluginRequest::List)
                .await
                .unwrap()
                .remove(0)
        } else {
            result.unwrap().remove(0)
        };
        let report = checked.conformance.as_ref().unwrap();
        assert_eq!(report.ok, !refused, "{fault}: {report:?}");
        assert_eq!(
            report.refusal.as_ref().map(|failure| failure.code.as_str()),
            refused.then_some("unreported_conversion_loss"),
            "{fault}"
        );
        assert_eq!(checked.state(), State::Disabled);
        let writes: Vec<_> = f
            .observations()
            .into_iter()
            .filter(|row| row["kind"] == "call" && row["name"] == "data_source_write")
            .map(|row| row["arguments"].clone())
            .collect();
        assert_eq!(
            writes.len(),
            2,
            "{fault}: the original and additive write must both reach the private process"
        );
        let ids: Vec<_> = writes
            .iter()
            .map(|request| {
                request["frontmatter"]["id"]
                    .as_str()
                    .unwrap()
                    .parse::<Ulid>()
                    .unwrap()
            })
            .collect();
        assert_ne!(
            ids[0], ids[1],
            "{fault}: creates must not reuse native identity"
        );
        assert_ne!(writes[0]["path"], writes[1]["path"]);
        for request in &writes {
            assert_eq!(
                request["frontmatter"]["slug"].as_str(),
                request["path"]
                    .as_str()
                    .map(|path| path.trim_end_matches(".md"))
            );
        }
        eprintln!(
            "private conversion fixture {fault}: ok={}, refusal={:?}",
            report.ok, report.refusal
        );
    }
}

// keeps: plugin-host — additive writes retain actual native conflict/deadline diagnostics without inferring a cause.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn additive_write_probe_preserves_backend_code_and_message() {
    for (fault, code, message) in [
        (
            "write_backend_conflict",
            "source_generation_mismatch",
            "private native source changed before additive write",
        ),
        (
            "write_backend_deadline",
            "deadline_exceeded",
            "private native fixture exceeded its write deadline",
        ),
    ] {
        let f = Fixture::reference(Some(fault), None);
        let trusted = f.trust_reference().await;
        let error = f
            .ask(
                Ulid::new(),
                PluginRequest::Check {
                    candidate: trusted.candidate.id.clone(),
                },
            )
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("command_error") && error.contains(code) && error.contains(message),
            "{fault}: {error}"
        );
        let entry = f
            .ask(Ulid::new(), PluginRequest::List)
            .await
            .unwrap()
            .remove(0);
        let refusal = entry
            .conformance
            .as_ref()
            .unwrap()
            .refusal
            .as_ref()
            .unwrap();
        assert_eq!(refusal.code, "command_error");
        assert!(
            refusal.message.contains("additive probe refused")
                && refusal.message.contains(code)
                && refusal.message.contains(message),
            "{fault}: {refusal:?}"
        );
        assert_eq!(entry.state(), State::Disabled);
        eprintln!("private additive backend refusal: {refusal:?}");
    }
}

// keeps: plugin-host — a conformance refusal is durable and cannot survive a binary replacement as a current result.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conformance_refusal_is_named_and_a_changed_digest_requires_explicit_trust() {
    let f = Fixture::reference(Some("bad_result"), None);
    let trusted = f.trust_reference().await;
    let request = PluginRequest::Check {
        candidate: trusted.candidate.id.clone(),
    };
    let op = Ulid::new();
    let error = f.ask(op, request.clone()).await.unwrap_err().to_string();
    assert!(
        error.contains("contract_failed") && error.contains("result_schema_mismatch"),
        "{error}"
    );
    let entry = f
        .ask(Ulid::new(), PluginRequest::List)
        .await
        .unwrap()
        .remove(0);
    let result = entry.conformance.as_ref().unwrap();
    assert!(!result.ok);
    assert_eq!(
        result.refusal.as_ref().unwrap().code,
        "result_schema_mismatch"
    );
    assert_eq!(entry.state(), State::Disabled);
    let observations = f.observations();
    assert_eq!(
        f.ask(op, request.clone()).await.unwrap_err().to_string(),
        error
    );
    assert_eq!(f.observations(), observations);
    f.replace("replaced bytes");
    let entry = f
        .ask(Ulid::new(), PluginRequest::List)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(entry.state(), State::NeedsRetrust);
    assert!(entry.conformance.is_none());
    assert!(
        f.ask(Ulid::new(), request)
            .await
            .unwrap_err()
            .to_string()
            .contains("plugin_reapproval_required")
    );
    assert_eq!(f.observations(), observations);
}

// keeps: plugin-host — a held check never gates other plugins, and a changed binding rejects its completion.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_conformance_keeps_other_controls_independent_and_fences_stale_completion() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("gate.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let f = std::sync::Arc::new(Fixture::reference(None, Some(&socket)));
    let trusted = f.trust_reference().await;
    // Keep the independent control inside the existing bounded inventory budget.
    let other_fixture = Fixture::new();
    let other = f.path.with_file_name("bboxy450-term-repo_graph-other");
    fs::copy(&other_fixture.path, &other).unwrap();
    let task = tokio::spawn({
        let f = f.clone();
        let candidate = trusted.candidate.id.clone();
        async move { f.ask(Ulid::new(), PluginRequest::Check { candidate }).await }
    });
    let (mut gate, _) = tokio::time::timeout(std::time::Duration::from_secs(30), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let inventory = f.ask(Ulid::new(), PluginRequest::List).await.unwrap();
    assert_eq!(inventory.len(), 2);
    let other = f
        .ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: other.to_string_lossy().into_owned(),
                sha256: None,
                grants: None,
                tier: None,
            },
        )
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        f.ask(
            Ulid::new(),
            PluginRequest::Enable {
                candidate: other.candidate.id
            }
        )
        .await
        .unwrap()[0]
            .state(),
        State::Enabled
    );
    f.replace("changed while checking");
    use tokio::io::AsyncWriteExt;
    gate.write_all(b"x").await.unwrap();
    let error = task.await.unwrap().unwrap_err().to_string();
    assert!(error.contains("plugin_reapproval_required"), "{error}");
    let entries = f.ask(Ulid::new(), PluginRequest::List).await.unwrap();
    let changed = entries
        .iter()
        .find(|entry| entry.candidate.id == trusted.candidate.id)
        .unwrap();
    assert_eq!(changed.state(), State::NeedsRetrust);
    assert!(changed.conformance.is_none());
}
