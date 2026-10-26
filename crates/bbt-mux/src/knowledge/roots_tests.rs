#![allow(
    clippy::disallowed_methods,
    reason = "owned private root and plugin fixtures on IO, never operator identity"
)]
use super::*;
use crate::{
    Ulid,
    plugins::{DiscoveryRoots, RegistryConfig},
    registry,
    store::{Store, sqlite::SqliteStore},
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

struct Fixture {
    temp: tempfile::TempDir,
    index: Arc<Index>,
    seats: registry::Registry,
    plugins: Registry,
    host: Host,
    journal: Journal,
    trace: PathBuf,
    path: PathBuf,
    original: BTreeMap<String, Vec<u8>>,
}
fn actor() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: mcp::Door::Ctl,
        issued_by: Some("test-account:root-fixture".into()),
    }
}
impl Fixture {
    fn new(count: usize, gate: Option<&Path>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo/.bboxy");
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::create_dir_all(root.join("plugins")).unwrap();
        let mut original = BTreeMap::new();
        for n in 1..=count {
            let path = format!("notes/{n:04}.md");
            let metadata = json!({"id":Ulid::from(n as u128).to_string(),"number":n,"type":"note","title":format!("Seed {n}"),"unknown":{"keep":true}});
            let text = format!("---\n{metadata}\n---\n# Seed {n}\n\nRetained body.\n").into_bytes();
            fs::write(root.join(&path), &text).unwrap();
            original.insert(path, text);
        }
        let account = SqliteStore::open(&temp.path().join("account")).unwrap();
        let mut index = Index::open(
            &root,
            SqliteStore::open(&temp.path().join("index")).unwrap(),
        )
        .unwrap();
        index.scan().unwrap();
        let plugins = Registry::with_config(
            account.clone(),
            DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
            RegistryConfig {
                descriptor_timeout: std::time::Duration::from_secs(30),
                conformance_timeout: std::time::Duration::from_secs(30),
            },
        )
        .unwrap();
        let trace = temp.path().join("protocol.jsonl");
        let path = root.join("plugins/bboxy450-term-data_source-root-fixture");
        let mut source = format!(
            "#!/usr/bin/env python3\nimport json\nSCHEMA=json.loads({})\n{}",
            serde_json::to_string(bbt_plugin::data_source::SCHEMA).unwrap(),
            include_str!("../../tests/fixtures/plugins/root_provider.py")
        );
        source = source.replace(
            "TRACE = None",
            &format!("TRACE = {}", json!(trace.to_str().unwrap())),
        );
        if let Some(gate) = gate {
            source = source
                .replace(
                    "GATE = None",
                    &format!("GATE = {}", json!(gate.to_str().unwrap())),
                )
                .replace(
                    "GATE_ROOT = None",
                    &format!(
                        "GATE_ROOT = {}",
                        json!(root.canonicalize().unwrap().to_str().unwrap())
                    ),
                );
        }
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let host = Host::with_config(
            &root,
            plugins.clone(),
            Handle::current(),
            ProviderConfig {
                timeout: std::time::Duration::from_secs(30),
            },
        )
        .unwrap();
        Self {
            index: Arc::new(index),
            seats: registry::Registry::new(account.clone()),
            plugins,
            host,
            journal: Journal::new(account),
            trace,
            path,
            original,
            temp,
        }
    }
    async fn ask(&self, id: Ulid, request: KnowledgeRequest) -> Result<KnowledgeStatus> {
        self.host
            .request(
                self.index.clone(),
                self.seats.clone(),
                actor(),
                self.journal.clone(),
                Call {
                    op_id: id,
                    tool: "knowledge_control".into(),
                    args: serde_json::to_value(&request).unwrap(),
                },
                request,
            )
            .await
    }
    async fn status(&self) -> KnowledgeStatus {
        self.ask(Ulid::new(), KnowledgeRequest::Status)
            .await
            .unwrap()
    }
    async fn checked(&self) -> bbt_plugin::registry::Entry {
        let entry = self.plugins.refresh().unwrap().remove(0);
        self.plugins
            .trust(
                &actor(),
                &entry.candidate.id,
                entry.candidate.sha256.as_deref().unwrap(),
                None,
            )
            .unwrap();
        let inspected = self
            .plugins
            .inspection(&entry.candidate.id)
            .unwrap()
            .unwrap()
            .run()
            .await;
        self.plugins.finish_inspection(inspected).unwrap();
        let checked = self
            .plugins
            .conformance(&actor(), &entry.candidate.id)
            .unwrap()
            .unwrap()
            .run()
            .await;
        for pid in checked.owned_pids() {
            assert_gone(*pid);
        }
        let checked = self.plugins.finish_conformance(checked).unwrap();
        assert!(checked.conformance.as_ref().unwrap().ok, "{checked:?}");
        self.plugins
            .set_enabled(&actor(), &entry.candidate.id, true)
            .unwrap()
    }
    async fn select(&self, entry: &bbt_plugin::registry::Entry) -> KnowledgeStatus {
        let status = self.status().await;
        let root = &status.roots[0];
        self.ask(
            Ulid::new(),
            KnowledgeRequest::Select {
                root: root.id.clone(),
                provider: entry.candidate.id.clone(),
                revision: root.revision,
                sha256: entry.candidate.sha256.clone(),
            },
        )
        .await
        .unwrap()
    }
    async fn read(&self, method: &str, args: Value) -> d::Result<Value> {
        let host = self.host.clone();
        let index = self.index.clone();
        let registry = self.seats.clone();
        let method = method.to_owned();
        tokio::task::spawn_blocking(move || {
            host.with_source(&index, &registry, |source| {
                super::super::read(source, &method, args)
            })
        })
        .await
        .unwrap()
    }
    fn observed(&self) -> Vec<Value> {
        fs::read_to_string(&self.trace)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn unchanged(&self) {
        for (path, bytes) in &self.original {
            assert_eq!(
                &fs::read(self.index.knowledge_root().join(path)).unwrap(),
                bytes,
                "record {path} changed or moved"
            );
        }
    }
}
fn assert_gone(pid: u32) {
    // Only recorded fixture children. Signal zero observes, never kills a process.
    assert_eq!(
        unsafe { libc::kill(pid as i32, 0) },
        -1,
        "owned child {pid} remains"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
impl Drop for Fixture {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| self.host.close());
    }
}

// keeps: knowledge-base §6(3), plugins §1 trust-before-execution and pin-bound checks; #350 rootless authority.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_root_swap_and_back_preserves_418_records_and_persists_one_root_instance() {
    let f = Fixture::new(418, None);
    let before = f.status().await;
    assert_eq!(before.roots[0].records, Some(418));
    assert_eq!(before.offered.len(), 1);
    assert!(!f.trace.exists());
    let untrusted = f.plugins.refresh().unwrap().remove(0);
    let request = KnowledgeRequest::Select {
        root: before.roots[0].id.clone(),
        provider: untrusted.candidate.id.clone(),
        revision: 0,
        sha256: untrusted.candidate.sha256.clone(),
    };
    assert!(f.ask(Ulid::new(), request).await.is_err());
    assert!(!f.trace.exists());
    let entry = f.checked().await;
    let swapped = f.select(&entry).await;
    assert_eq!(swapped.roots[0].digest, before.roots[0].digest);
    assert_eq!(swapped.roots[0].records, Some(418));
    let pid = swapped.roots[0].process.expect("actual stdio responder");
    assert_ne!(pid, std::process::id());
    assert_eq!(swapped.roots[0].provider.id, entry.candidate.id);
    let reference = f.read("resolve", json!({"locator":"#71"})).await.unwrap();
    let record = f.read("fetch", json!({"ref":reference})).await.unwrap();
    assert_eq!(record["frontmatter"]["unknown"], json!({"keep":true}));
    assert!(record["body"].as_str().unwrap().contains("Seed 71"));
    let observations = f.observed();
    let runtime: Vec<_> = observations
        .iter()
        .filter(|row| {
            row["kind"] == "initialize"
                && row["root"]
                    == f.index
                        .knowledge_root()
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
        })
        .collect();
    assert_eq!(runtime.len(), 1);
    assert_eq!(runtime[0]["pid"], pid);
    assert_ne!(runtime[0]["cwd"], runtime[0]["root"]);
    assert!(
        f.plugins
            .set_enabled(&actor(), &entry.candidate.id, false)
            .unwrap_err()
            .code
            == "provider_of_root"
    );
    let host = f.host.clone();
    let index = f.index.clone();
    let seats = f.seats.clone();
    let refusal = tokio::task::spawn_blocking(move || {
        let key = ed25519_dalek::SigningKey::from_bytes(&[37; 32]);
        let context = super::super::WriteContext::seat(Ulid::from(37_u128), Some(1), &key);
        host.with_source(&index, &seats, |source| {
            source.write(
                d::WriteRequest {
                    reference: None,
                    path: Some("must-not-write.md".into()),
                    frontmatter: json!({"type":"note"}),
                    body: "denied".into(),
                    etag: None,
                },
                &context,
            )
        })
    })
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(refusal.code, "signed_writer_unavailable");
    assert!(refusal.message.contains(BUNDLED));
    assert!(!f.index.knowledge_root().join("must-not-write.md").exists());
    f.unchanged();
    let host = f.host.clone();
    tokio::task::spawn_blocking(move || host.close())
        .await
        .unwrap();
    assert_gone(pid);
    let reopened = Host::with_config(
        f.index.knowledge_root(),
        f.plugins.clone(),
        Handle::current(),
        ProviderConfig {
            timeout: std::time::Duration::from_secs(30),
        },
    )
    .unwrap();
    let status = reopened
        .request(
            f.index.clone(),
            f.seats.clone(),
            actor(),
            f.journal.clone(),
            Call {
                op_id: Ulid::new(),
                tool: "knowledge_control".into(),
                args: json!({"kind":"status"}),
            },
            KnowledgeRequest::Status,
        )
        .await
        .unwrap();
    assert_eq!(status.roots[0].provider.id, entry.candidate.id);
    assert_eq!(status.roots[0].digest, before.roots[0].digest);
    let new_pid = status.roots[0].process.unwrap();
    assert_ne!(pid, new_pid);
    let host = reopened.clone();
    tokio::task::spawn_blocking(move || host.close())
        .await
        .unwrap();
    assert_gone(new_pid);
    let reset_id = Ulid::new();
    let reset = KnowledgeRequest::Reset {
        root: swapped.roots[0].id.clone(),
    };
    let back = f.ask(reset_id, reset.clone()).await.unwrap();
    assert_eq!(back.roots[0].provider.id, BUNDLED);
    assert_eq!(back.roots[0].digest, before.roots[0].digest);
    assert_eq!(back.roots[0].records, Some(418));
    assert_eq!(
        f.ask(reset_id, reset).await.unwrap(),
        back,
        "same-op reset returns its durable receipt"
    );
    f.plugins
        .set_enabled(&actor(), &entry.candidate.id, false)
        .unwrap();
    f.unchanged();
    assert_eq!(
        f.plugins.account().list("knowledge/roots/").unwrap().len(),
        1
    );
    assert!(f.path.exists());
    assert!(f.temp.path().exists());
}

// keeps: ui-never-waits, root epoch fence, no fallback to bundled data after external failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_is_independent_of_a_held_provider_read_and_refuses_its_old_completion() {
    let gate = tempfile::tempdir().unwrap();
    let path = gate.path().join("gate.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let f = Fixture::new(2, Some(&path));
    let entry = f.checked().await;
    let swapped = f.select(&entry).await;
    let pid = swapped.roots[0].process.unwrap();
    let reference = f.read("resolve", json!({"locator":"#1"})).await.unwrap();
    let host = f.host.clone();
    let index = f.index.clone();
    let registry = f.seats.clone();
    let read = tokio::task::spawn_blocking(move || {
        host.with_source(&index, &registry, |source| {
            source.fetch(d::FetchRequest {
                reference: serde_json::from_value(reference).unwrap(),
            })
        })
    });
    let (mut connection, _) =
        tokio::time::timeout(std::time::Duration::from_secs(30), listener.accept())
            .await
            .expect("fixture did not reach its explicit read barrier")
            .unwrap();
    let reset = f
        .ask(
            Ulid::new(),
            KnowledgeRequest::Reset {
                root: swapped.roots[0].id.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(reset.roots[0].provider.id, BUNDLED);
    use tokio::io::AsyncWriteExt;
    connection.write_all(b"x").await.unwrap();
    assert_eq!(read.await.unwrap().unwrap_err().code, "root_changed");
    assert_gone(pid);
    f.unchanged();
}
