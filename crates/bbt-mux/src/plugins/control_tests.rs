#![allow(
    clippy::disallowed_methods,
    reason = "private account/process fixtures, never UI IO"
)]
use super::*;
use crate::{Ulid, store::sqlite::SqliteStore};
use bbt_plugin::{contracts::Contract, registry::State};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    control: Control,
    journal: Journal,
    path: PathBuf,
    marker: PathBuf,
}
impl Fixture {
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
        fs::write(&path, format!("#!/usr/bin/env python3\nimport json,sys\nassert sys.argv[1:]==['--shape']\nwith open({},'a') as f: f.write('shape\\n')\nprint({})\n",
            serde_json::to_string(&marker.to_string_lossy()).unwrap(),serde_json::to_string(&shape.to_string()).unwrap())).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let account = SqliteStore::open(&root.path().join("account")).unwrap();
        let registry = super::super::Registry::with_config(
            account.clone(),
            super::super::DiscoveryRoots::new(&root.path().join("app"), &root.path().join("repo")),
            super::super::RegistryConfig {
                descriptor_timeout: std::time::Duration::from_secs(30),
            },
        )
        .unwrap();
        Self {
            control: Control::new(registry),
            journal: Journal::new(account),
            path,
            marker,
            _root: root,
        }
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
                sha256: None
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
                sha256: None
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
    let source = fs::read_to_string(&f.path).unwrap();
    fs::write(&f.path, format!("{source}\n# replacement\n")).unwrap();
    let error = f
        .ask(
            Ulid::new(),
            PluginRequest::Trust {
                path: f.path.to_string_lossy().into_owned(),
                sha256: entry.candidate.sha256,
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
    let original = fs::read_to_string(&f.path).unwrap();
    let other = f.path.with_file_name("bboxy450-term-repo_graph-other");
    fs::write(&other, &original).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&f.path,original.replace("\nprint(",&format!("\nimport socket\ngate=socket.socket(socket.AF_UNIX)\ngate.connect({})\ngate.recv(1)\ngate.close()\nprint(",serde_json::to_string(&socket.to_string_lossy()).unwrap()))).unwrap();
    let task = tokio::spawn({
        let f = f.clone();
        async move {
            f.ask(
                Ulid::new(),
                PluginRequest::Trust {
                    path: f.path.to_string_lossy().into_owned(),
                    sha256: None,
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
            },
        )
        .await
        .unwrap();
    assert_eq!(other[0].state(), State::Disabled);
    use tokio::io::AsyncWriteExt;
    gate.write_all(b"x").await.unwrap();
    assert_eq!(task.await.unwrap().unwrap()[0].state(), State::Disabled);
}
