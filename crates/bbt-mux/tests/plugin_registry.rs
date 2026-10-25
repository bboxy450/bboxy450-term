#![allow(
    clippy::disallowed_methods,
    reason = "private plugin/account fixtures, never UI or operator identity IO"
)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use bbt_mux::{
    Ulid,
    mcp::{Actor, Door, SeatActor},
    messaging::wire::IssuerKind,
    plugins::{DiscoveryRoots, Registry},
    registry::{Class, Role},
    store::{Store, sqlite::SqliteStore},
};
use bbt_plugin::{
    contracts::Contract,
    policy::{Hook, Phase, Tier},
    registry::{Approval, Entry, Scope, State},
};
use serde_json::{Value, json};

fn shape(contracts: &[Contract]) -> Value {
    json!({"name":"private reference", "version":"0.1", "shape_version":"1.0", "health_command":"health",
        "systems":contracts.iter().map(|contract| json!({
            "contract":contract.name(), "contract_version":contract.version(),
            "commands":contract.methods().iter().map(|method| json!({
                "name":method, "args":contract.method_schema(method,false).unwrap(),
                "result":contract.method_schema(method,true).unwrap(), "timeout_ms":1000
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()})
}

fn operator() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: Door::Ctl,
        issued_by: None,
    }
}

struct Harness {
    root: tempfile::TempDir,
    account: SqliteStore,
    registry: Registry,
}

impl Harness {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("app/plugins")).unwrap();
        fs::create_dir_all(root.path().join("repo/.bboxy/plugins")).unwrap();
        fs::create_dir_all(root.path().join("markers")).unwrap();
        let account = SqliteStore::open(&root.path().join("account")).unwrap();
        let registry = Registry::new(account.clone(), Self::roots(root.path())).unwrap();
        Self {
            root,
            account,
            registry,
        }
    }

    fn roots(root: &Path) -> DiscoveryRoots {
        DiscoveryRoots::new(&root.join("app"), &root.join("repo"))
    }

    fn marker(&self, filename: &str) -> PathBuf {
        self.root.path().join("markers").join(filename)
    }

    fn plugin(&self, scope: Scope, filename: &str, shape: Value, extra: &str) -> PathBuf {
        let parent = match scope {
            Scope::Application => "app/plugins",
            Scope::Repository => "repo/.bboxy/plugins",
        };
        let path = self.root.path().join(parent).join(filename);
        let marker = serde_json::to_string(&self.marker(filename).to_string_lossy()).unwrap();
        let source = serde_json::to_string(&path.to_string_lossy()).unwrap();
        let db = serde_json::to_string(
            &self
                .root
                .path()
                .join("account/object.sqlite")
                .to_string_lossy(),
        )
        .unwrap();
        let descriptor = serde_json::to_string(&shape.to_string()).unwrap();
        let code = format!(
            r#"#!/usr/bin/env python3
import hashlib,json,os,sqlite3,sys
MARKER = {marker}
SOURCE = {source}
with open(MARKER, 'a') as output:
    output.write(json.dumps({{'phase':'entered','pid':os.getpid()}})+'\n')
assert sys.argv[1:] == ['--shape'], 'no business or setup method in P-1 cut 1'
copy = os.path.realpath(sys.argv[0])
with open(copy,'rb') as data:
    digest = hashlib.sha256(data.read()).hexdigest()
db = sqlite3.connect('file:' + {db} + '?mode=ro', uri=True)
row = db.execute('SELECT value FROM kv WHERE key=?', ('plugins/trust/'+digest,)).fetchone()
assert row is not None, 'account approval must be durable before --shape'
record = json.loads(row[0])
assert record['copy'] == copy and record['source'] == os.path.realpath(SOURCE)
assert os.stat(copy).st_mode & 0o222 == 0
with open(MARKER, 'a') as output:
    output.write(json.dumps({{'phase':'verified','pid':os.getpid(),'cwd':os.getcwd(), 'copy':copy, 'env':sorted(os.environ)}})+'\n')
{extra}
print(json.dumps(json.loads({descriptor})))
"#
        );
        fs::write(&path, code).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn entry(&self, filename: &str) -> Entry {
        self.registry
            .refresh()
            .unwrap()
            .into_iter()
            .find(|e| Path::new(&e.candidate.source).file_name().unwrap() == filename)
            .unwrap()
    }

    fn trust(&self, filename: &str) -> Entry {
        let entry = self.entry(filename);
        self.registry
            .trust(
                &operator(),
                &entry.candidate.id,
                entry.candidate.sha256.as_deref().unwrap(),
                None,
            )
            .unwrap()
    }

    async fn inspect(&self, id: &str) -> Entry {
        let job = self
            .registry
            .inspection(id)
            .unwrap()
            .expect("first descriptor job");
        assert!(
            self.registry.inspection(id).unwrap().is_none(),
            "one admitted inspection per generation"
        );
        let result = job.run().await;
        if let Some(pid) = result.pid() {
            reaped(pid);
        }
        self.registry.finish_inspection(result).unwrap()
    }

    fn runs(&self, filename: &str) -> Vec<Value> {
        fs::read_to_string(self.marker(filename))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

fn reaped(pid: u32) {
    assert_eq!(
        unsafe { libc::kill(pid as i32, 0) },
        -1,
        "owned child {pid} still exists"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

#[tokio::test]
async fn discovery_never_executes_or_consumes_repository_trust_and_seats_cannot_approve() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    h.plugin(Scope::Repository, name, shape(&[Contract::DataSource]), "");
    fs::write(
        h.root.path().join("repo/.bboxy/plugins/trust.json"),
        br#"{"trusted":true,"enabled":true}"#,
    )
    .unwrap();
    let entry = h.entry(name);
    assert_eq!(entry.state(), State::Untrusted);
    assert_eq!(
        h.registry
            .inspection(&entry.candidate.id)
            .err()
            .unwrap()
            .code,
        "untrusted_plugin"
    );
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap_err().code,
        "untrusted_plugin"
    );
    let seat = Actor::Seat(SeatActor {
        seat_id: Ulid::new(),
        process_generation: 1,
        swarm_id: Ulid::new(),
        role: Role {
            name: "Oracle".into(),
            class: Class::Oracle,
        },
        grant_epoch: 1,
        issuer_kind: IssuerKind::GpgRoot,
    });
    assert_eq!(
        h.registry
            .trust(
                &seat,
                &entry.candidate.id,
                entry.candidate.sha256.as_deref().unwrap(),
                None
            )
            .unwrap_err()
            .code,
        "operator_required"
    );
    assert_eq!(
        h.registry
            .set_enabled(&seat, &entry.candidate.id, true)
            .unwrap_err()
            .code,
        "operator_required"
    );
    assert!(h.account.list("plugins/trust/").unwrap().is_empty());
    assert!(!h.marker(name).exists());
    let source = PathBuf::from(&entry.candidate.source);
    let checked = bbt_mux::plugins::check(&source).await;
    assert_eq!(checked.refusal.unwrap().code, "untrusted_production_file");
    assert!(checked.owned_pids.is_empty());
    assert!(h.account.list("plugins/trust/").unwrap().is_empty());
    assert!(!h.marker(name).exists());
}

#[tokio::test]
async fn missing_sources_and_unfinished_inspections_remain_disabled_after_restart() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    let source = h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    let entry = h.trust(name);
    let job = h.registry.inspection(&entry.candidate.id).unwrap().unwrap();
    drop(job); // No native attempt: emulate a lost metadata job, not a retry.
    let reopened = Registry::new(
        SqliteStore::open(&h.root.path().join("account")).unwrap(),
        Harness::roots(h.root.path()),
    )
    .unwrap();
    assert!(reopened.inspection(&entry.candidate.id).unwrap().is_none());
    assert_eq!(
        reopened
            .set_enabled(&operator(), &entry.candidate.id, true)
            .unwrap_err()
            .code,
        "shape_not_cached"
    );
    assert!(h.entry(name).activation.unwrap().inspection_started);
    assert!(!h.marker(name).exists());
    // Explicit reapproval can retry the read; it is never replayed on restart.
    h.trust(name);
    h.inspect(&entry.candidate.id).await;
    h.registry
        .set_enabled(&operator(), &entry.candidate.id, true)
        .unwrap();
    let outside = h.root.path().join("temporarily-removed");
    fs::rename(&source, &outside).unwrap();
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap_err().code,
        "plugin_missing"
    );
    fs::rename(&outside, &source).unwrap();
    assert_eq!(h.entry(name).state(), State::NeedsRetrust);
}

#[tokio::test]
async fn account_trust_precedes_shape_and_enable_is_separate_persistent_and_pinned() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    let source = h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    let entry = h.trust(name);
    assert_eq!(entry.state(), State::Unchecked);
    let record = entry.trust.as_ref().unwrap();
    assert_eq!(record.approved_tier, Tier::Export);
    assert_eq!(record.grants, Default::default());
    assert_eq!(
        Path::new(&record.copy)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        record.sha256
    );
    assert_ne!(Path::new(&record.copy), source);
    assert!(!h.marker(name).exists());
    assert_eq!(
        h.registry
            .set_enabled(&operator(), &entry.candidate.id, true)
            .unwrap_err()
            .code,
        "shape_not_cached"
    );
    let entry = h.inspect(&entry.candidate.id).await;
    assert_eq!(entry.state(), State::Disabled);
    assert!(!entry.activation.as_ref().unwrap().desired_enabled);
    assert!(
        h.registry
            .inspection(&entry.candidate.id)
            .unwrap()
            .is_none()
    );
    let runs = h.runs(name);
    assert_eq!(runs.iter().filter(|r| r["phase"] == "entered").count(), 1);
    let verified = runs.iter().find(|r| r["phase"] == "verified").unwrap();
    assert_eq!(verified["cwd"], record.data_dir);
    for key in verified["env"].as_array().unwrap() {
        let key = key.as_str().unwrap();
        assert!(
            bbt_plugin::policy::allowed_env_name(key)
                || cfg!(target_os = "macos") && key == "__CF_USER_TEXT_ENCODING",
            "unexpected inherited env name {key}"
        );
    }
    let enabled = h
        .registry
        .set_enabled(&operator(), &entry.candidate.id, true)
        .unwrap();
    assert_eq!(enabled.state(), State::Enabled);
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap(),
        PathBuf::from(&record.copy)
    );
    let reopened = Registry::new(
        SqliteStore::open(&h.root.path().join("account")).unwrap(),
        Harness::roots(h.root.path()),
    )
    .unwrap();
    assert_eq!(
        reopened.executable(&entry.candidate.id).unwrap(),
        PathBuf::from(&record.copy)
    );
    assert!(reopened.inspection(&entry.candidate.id).unwrap().is_none());
    assert_eq!(
        reopened
            .set_enabled(&operator(), &entry.candidate.id, false)
            .unwrap()
            .state(),
        State::Disabled
    );
    assert_eq!(
        reopened.executable(&entry.candidate.id).unwrap_err().code,
        "plugin_disabled"
    );
}

#[test]
fn independent_plugins_can_pin_and_approve_concurrently_in_the_same_account() {
    let h = Harness::new();
    let entries: Vec<_> = (0..4)
        .map(|i| {
            let name = format!("bboxy450-term-data_source-concurrent-{i}");
            h.plugin(
                Scope::Application,
                &name,
                shape(&[Contract::DataSource]),
                "",
            );
            h.entry(&name)
        })
        .collect();
    let start = std::sync::Barrier::new(entries.len());
    std::thread::scope(|scope| {
        let jobs: Vec<_> = entries
            .iter()
            .map(|entry| {
                let start = &start;
                let registry = &h.registry;
                scope.spawn(move || {
                    start.wait();
                    registry.trust(
                        &operator(),
                        &entry.candidate.id,
                        entry.candidate.sha256.as_deref().unwrap(),
                        None,
                    )
                })
            })
            .collect();
        for job in jobs {
            assert_eq!(job.join().unwrap().unwrap().state(), State::Unchecked);
        }
    });
    assert_eq!(
        h.account.list("plugins/trust/").unwrap().len(),
        entries.len()
    );
    for entry in entries {
        assert!(
            !h.marker(
                Path::new(&entry.candidate.source)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
            )
            .exists()
        );
    }
}

#[tokio::test]
async fn changed_digest_and_older_hash_restoration_require_explicit_reapproval() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    let source = h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    let original = fs::read(&source).unwrap();
    let entry = h.trust(name);
    h.inspect(&entry.candidate.id).await;
    h.registry
        .set_enabled(&operator(), &entry.candidate.id, true)
        .unwrap();
    fs::write(&source, b"#!/bin/sh\nexit 99\n").unwrap();
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap_err().code,
        "plugin_reapproval_required"
    );
    fs::write(&source, original).unwrap();
    let restored = h.entry(name);
    assert_eq!(restored.state(), State::NeedsRetrust);
    assert!(!restored.activation.unwrap().desired_enabled);
    assert_eq!(
        h.registry
            .set_enabled(&operator(), &entry.candidate.id, true)
            .unwrap_err()
            .code,
        "plugin_reapproval_required"
    );
    let reapproved = h.trust(name);
    assert_eq!(reapproved.state(), State::Unchecked);
    assert!(!reapproved.activation.unwrap().desired_enabled);
    assert_eq!(
        h.runs(name)
            .iter()
            .filter(|r| r["phase"] == "entered")
            .count(),
        1
    );
}

#[tokio::test]
async fn writable_or_modified_pinned_copies_never_execute() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    let entry = h.trust(name);
    h.inspect(&entry.candidate.id).await;
    h.registry
        .set_enabled(&operator(), &entry.candidate.id, true)
        .unwrap();
    let copy = &entry.trust.as_ref().unwrap().copy;
    fs::set_permissions(copy, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap_err().code,
        "plugin_copy_writable"
    );
    fs::write(copy, b"#!/bin/sh\nexit 22\n").unwrap();
    fs::set_permissions(copy, fs::Permissions::from_mode(0o500)).unwrap();
    assert_eq!(
        h.registry.executable(&entry.candidate.id).unwrap_err().code,
        "plugin_digest_changed"
    );
    assert_eq!(
        h.runs(name)
            .iter()
            .filter(|r| r["phase"] == "entered")
            .count(),
        1
    );
}

#[tokio::test]
async fn stale_descriptor_completion_cannot_undo_disable_or_a_source_change() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-reference";
    let source = h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    let entry = h.trust(name);
    let first = h
        .registry
        .inspection(&entry.candidate.id)
        .unwrap()
        .unwrap()
        .run()
        .await;
    h.registry
        .set_enabled(&operator(), &entry.candidate.id, false)
        .unwrap();
    assert_eq!(
        h.registry.finish_inspection(first).unwrap_err().code,
        "stale_plugin_completion"
    );
    assert_eq!(h.entry(name).state(), State::Unchecked);
    h.trust(name);
    let second = h
        .registry
        .inspection(&entry.candidate.id)
        .unwrap()
        .unwrap()
        .run()
        .await;
    fs::write(&source, b"#!/bin/sh\nexit 33\n").unwrap();
    assert_eq!(
        h.registry.finish_inspection(second).unwrap_err().code,
        "plugin_reapproval_required"
    );
    assert_eq!(h.entry(name).state(), State::NeedsRetrust);
}

#[test]
fn refused_entries_and_cross_root_collisions_are_listed_without_precedence_or_execution() {
    let h = Harness::new();
    let name = "bboxy450-term-data_source-duplicate";
    let path = h.plugin(Scope::Application, name, shape(&[Contract::DataSource]), "");
    h.plugin(Scope::Repository, name, shape(&[Contract::DataSource]), "");
    fs::create_dir(
        h.root
            .path()
            .join("app/plugins/bboxy450-term-data_source-directory"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        &path,
        h.root
            .path()
            .join("app/plugins/bboxy450-term-data_source-symlink"),
    )
    .unwrap();
    fs::write(
        h.root
            .path()
            .join("app/plugins/other-app-data_source-invalid"),
        b"not executable",
    )
    .unwrap();
    let entries = h.registry.refresh().unwrap();
    assert_eq!(entries.len(), 5);
    let duplicates: Vec<_> = entries
        .iter()
        .filter(|e| e.candidate.source.ends_with(name))
        .collect();
    assert_eq!(duplicates.len(), 2);
    for entry in duplicates {
        assert_eq!(
            entry.candidate.refusal.as_ref().unwrap().code,
            "plugin_name_collision"
        );
        assert_eq!(
            h.registry
                .trust(
                    &operator(),
                    &entry.candidate.id,
                    entry.candidate.sha256.as_deref().unwrap(),
                    None
                )
                .unwrap_err()
                .code,
            "plugin_name_collision"
        );
    }
    assert!(
        entries
            .iter()
            .any(|e| e.candidate.refusal.as_ref().unwrap().code == "directory_plugin_refused")
    );
    assert!(
        entries
            .iter()
            .any(|e| e.candidate.refusal.as_ref().unwrap().code == "plugin_symlink_refused")
    );
    assert!(!h.marker(name).exists());
    assert!(h.account.list("plugins/trust/").unwrap().is_empty());
}

#[tokio::test]
async fn higher_tier_and_payload_grants_need_explicit_reapproval_and_reuse_observed_shape() {
    for payload in [false, true] {
        let h = Harness::new();
        let name = "bboxy450-term-data_source-grant";
        let mut descriptor = if payload {
            shape(&[Contract::DataSource])
        } else {
            shape(&[Contract::DataSource, Contract::Session])
        };
        if payload {
            descriptor["hooks"] = json!([Hook {
                event: "prompt_submit".into(),
                phase: Phase::After,
                budget_ms: 1000
            }]);
        }
        h.plugin(Scope::Application, name, descriptor, "");
        let entry = h.trust(name);
        let checked = h.inspect(&entry.candidate.id).await;
        let refusal = if payload {
            "payload_grant_required"
        } else {
            "tier_not_approved"
        };
        assert_eq!(
            checked
                .activation
                .as_ref()
                .unwrap()
                .inspection
                .as_ref()
                .unwrap()
                .refusal
                .as_ref()
                .unwrap()
                .code,
            refusal
        );
        assert!(checked.trust.unwrap().shape.is_none());
        assert_eq!(
            h.registry
                .set_enabled(&operator(), &entry.candidate.id, true)
                .unwrap_err()
                .code,
            refusal
        );
        let mut approval = Approval::minimum(entry.candidate.name.as_ref().unwrap());
        if payload {
            approval
                .grants
                .payload_events
                .insert("prompt_submit".into());
        } else {
            approval.tier = Tier::Transport;
        }
        let approved = h
            .registry
            .trust(
                &operator(),
                &entry.candidate.id,
                entry.candidate.sha256.as_deref().unwrap(),
                Some(approval),
            )
            .unwrap();
        assert_eq!(approved.state(), State::Disabled);
        assert!(
            h.registry
                .inspection(&entry.candidate.id)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            h.runs(name)
                .iter()
                .filter(|r| r["phase"] == "entered")
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn descriptor_added_contract_collisions_revoke_both_activations() {
    let h = Harness::new();
    let first = "bboxy450-term-data_source-shared";
    let second = "bboxy450-term-repo_graph-shared";
    h.plugin(
        Scope::Application,
        first,
        shape(&[Contract::DataSource]),
        "",
    );
    h.plugin(
        Scope::Repository,
        second,
        shape(&[Contract::RepoGraph, Contract::DataSource]),
        "",
    );
    let a = h.trust(first);
    h.inspect(&a.candidate.id).await;
    h.registry
        .set_enabled(&operator(), &a.candidate.id, true)
        .unwrap();
    let b = h.trust(second);
    // Additional data_source authority needs explicit approval before its shape
    // can be used; it then collides with the existing provider's public tools.
    let b = h.inspect(&b.candidate.id).await;
    assert_eq!(b.state(), State::Refused);
    h.registry
        .trust(
            &operator(),
            &b.candidate.id,
            b.candidate.sha256.as_deref().unwrap(),
            Some(Approval {
                tier: Tier::Export,
                grants: Default::default(),
            }),
        )
        .unwrap();
    for entry in h.registry.refresh().unwrap() {
        assert_eq!(
            entry.candidate.refusal.as_ref().unwrap().code,
            "plugin_name_collision"
        );
        assert!(!entry.activation.as_ref().unwrap().desired_enabled);
    }
    assert_eq!(
        h.registry.executable(&a.candidate.id).unwrap_err().code,
        "plugin_name_collision"
    );
}

#[tokio::test]
async fn deadline_failure_is_named_and_another_plugin_job_remains_independent() {
    let h = Harness::new();
    let slow = "bboxy450-term-data_source-slow";
    let good = "bboxy450-term-data_source-good";
    h.plugin(Scope::Application, slow, shape(&[Contract::DataSource]), "import signal,time\nsignal.signal(signal.SIGTERM, signal.SIG_IGN)\nwhile True: time.sleep(1)");
    h.plugin(Scope::Repository, good, shape(&[Contract::DataSource]), "");
    let a = h.trust(slow);
    let b = h.trust(good);
    let slow_job = h.registry.inspection(&a.candidate.id).unwrap().unwrap();
    let good_job = h.registry.inspection(&b.candidate.id).unwrap().unwrap();
    let (slow_result, good_result) = tokio::join!(slow_job.run(), good_job.run());
    let slow_pid = slow_result.pid().unwrap();
    let good_pid = good_result.pid().unwrap();
    reaped(slow_pid);
    reaped(good_pid);
    assert_eq!(
        slow_result.observation().refusal.as_ref().unwrap().code,
        "deadline_exceeded"
    );
    assert_eq!(
        h.registry.finish_inspection(slow_result).unwrap().state(),
        State::Refused
    );
    assert_eq!(
        h.registry.finish_inspection(good_result).unwrap().state(),
        State::Disabled
    );
    assert_eq!(
        h.registry
            .set_enabled(&operator(), &a.candidate.id, true)
            .unwrap_err()
            .code,
        "deadline_exceeded"
    );
}
