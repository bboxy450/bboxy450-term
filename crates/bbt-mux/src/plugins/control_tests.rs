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
                conformance_timeout: std::time::Duration::from_secs(30),
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
    fn reference(fault: Option<&str>, gate: Option<&std::path::Path>) -> Self {
        let mut f = Self::new();
        let path = f.path.with_file_name("bboxy450-term-data_source-reference");
        fs::rename(&f.path, &path).unwrap();
        f.path = path;
        let mut source =
            include_str!("../../tests/fixtures/plugins/bboxy450-term-data_source-reference")
                .to_owned();
        if let Some(fault) = fault {
            source = source.replace(
                "FAULT = None",
                &format!("FAULT = {}", serde_json::to_string(fault).unwrap()),
            );
        }
        source = source.replace("else 2000,", "else 30000,");
        let marker = serde_json::to_string(&f.marker.to_string_lossy()).unwrap();
        source = source.replace("if sys.argv[1:]==[\"--shape\"]:\n", &format!("if sys.argv[1:]==[\"--shape\"]:\n    with open({marker},'a') as f: f.write(json.dumps({{'kind':'shape'}})+'\\n')\n"));
        let barrier = gate.map_or_else(String::new, |path| format!("        import socket\n        with socket.socket(socket.AF_UNIX) as gate:\n            gate.connect({})\n            gate.recv(1)\n", serde_json::to_string(&path.to_string_lossy()).unwrap()));
        let instrumentation = format!(
            r#"    method=req.get("method")
    if method == 'initialize':
        root = req['params']['root']
        assert os.path.isfile(os.path.join(root,'p0-fixture.md'))
        assert os.path.realpath(root) != os.path.realpath(os.getcwd())
        assert not any(key in req['params'] for key in ['author','signer','device','account'])
        with open({marker},'a') as f: f.write(json.dumps({{'kind':'initialize','pid':os.getpid(),'root':root,'cwd':os.getcwd()}})+'\n')
{barrier}    elif method == 'tools/call':
        assert not any(key in req['params'].get('arguments',{{}}) for key in ['root','author','signer','device','account'])
        with open({marker},'a') as f: f.write(json.dumps({{'kind':'call','name':req['params']['name']}})+'\n')"#
        );
        source = source.replace("    method=req.get(\"method\")", &instrumentation);
        fs::write(&f.path, source).unwrap();
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
    let mut source = fs::read_to_string(&f.path).unwrap();
    source.push_str("\n# replaced bytes\n");
    fs::write(&f.path, source).unwrap();
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
    let other = f.path.with_file_name("bboxy450-term-data_source-other");
    fs::copy(&f.path, &other).unwrap();
    let mut other_bytes = fs::read_to_string(&other).unwrap();
    other_bytes.push_str("\n# distinct second plugin binding\n");
    fs::write(&other, other_bytes).unwrap();
    let task = tokio::spawn({
        let f = f.clone();
        let candidate = trusted.candidate.id;
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
    let mut source = fs::read_to_string(&f.path).unwrap();
    source.push_str("\n# changed while checking\n");
    fs::write(&f.path, source).unwrap();
    use tokio::io::AsyncWriteExt;
    gate.write_all(b"x").await.unwrap();
    let error = task.await.unwrap().unwrap_err().to_string();
    assert!(error.contains("plugin_reapproval_required"), "{error}");
    let entries = f.ask(Ulid::new(), PluginRequest::List).await.unwrap();
    let changed = entries
        .iter()
        .find(|entry| entry.candidate.source == f.path.to_string_lossy())
        .unwrap();
    assert_eq!(changed.state(), State::NeedsRetrust);
    assert!(changed.conformance.is_none());
}
