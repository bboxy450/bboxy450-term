// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    index::Document,
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
    store::{Events, events::EventLog},
};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

struct Fixture {
    _temp: Option<tempfile::TempDir>,
    runtime: tokio::runtime::Runtime,
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    config: LogicConfig,
    one: Ulid,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_owned();
        Self::at(&path, Some(temp))
    }
    fn at(path: &Path, temp: Option<tempfile::TempDir>) -> Self {
        Self::construct(path, temp, None)
    }
    fn construct(
        path: &Path,
        temp: Option<tempfile::TempDir>,
        custody: Option<std::sync::Arc<dyn bbt_mux::identity::DeviceKeyStore + Send + Sync>>,
    ) -> Self {
        let repository = path.join("repository");
        std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
        let config = LogicConfig {
            repository,
            run_dir: path.join("run"),
            ring: Default::default(),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(common::capture_config(path)).unwrap();
        let connection = supervisor.connect().unwrap();
        let logic = match custody {
            Some(store) => Logic::start_custodied_with_store(
                connection,
                config.clone(),
                store,
                runtime.handle(),
            ),
            None => Logic::start(connection, config.clone(), key(), runtime.handle()),
        }
        .unwrap();
        let one = Ulid::new();
        let record = SeatRecord::new(one, Ulid::new(), "Provenance fixture", "zsh");
        let mut spec = LaunchSpec::new("/bin/zsh", &config.repository);
        spec.seat = one;
        spec.args = vec![
            "-f".into(),
            "-c".into(),
            "while IFS= read -r line; do :; done".into(),
        ];
        runtime.block_on(logic.client.spawn(record, spec)).unwrap();
        Self {
            _temp: temp,
            runtime,
            logic: Some(logic),
            supervisor: Some(supervisor),
            config,
            one,
        }
    }
    fn documents(&self) -> Vec<Document> {
        let path = self.config.repository.join(".bboxy/artifacts");
        std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .map(|p| {
                Document::parse(
                    p.file_name().unwrap().to_str().unwrap(),
                    &std::fs::read_to_string(&p).unwrap(),
                )
            })
            .collect()
    }
    fn wait(&self, reference: &str, predicate: impl Fn(&Document) -> bool) -> Document {
        let until = Instant::now() + Duration::from_secs(8);
        loop {
            if let Some(doc) = self
                .documents()
                .into_iter()
                .find(|d| d.metadata["ref"] == reference && predicate(d))
            {
                return doc;
            }
            assert!(
                Instant::now() < until,
                "missing {reference}: {:?}",
                self.documents()
                    .iter()
                    .map(|d| &d.metadata)
                    .collect::<Vec<_>>()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn rows(&self) -> Vec<bbt_mux::store::events::Envelope> {
        EventLog::open(&self.config.run_dir.join("index"))
            .unwrap()
            .after(0, 1024)
            .unwrap()
    }
    fn hook(&self, path: &Path, body: &str) {
        let event=bbt_mux::adapters::hook::prepare(json!({"hook_event_name":"PostToolUse","tool_name":"Write","tool_input":{"file_path":path,"content":body},"token":"not-to-publish"}),"fixture").unwrap();
        self.runtime
            .block_on(self.logic.as_ref().unwrap().client.hook(self.one, 1, event))
            .unwrap();
    }
    fn claim(&self, reference: &str, digest: &str) -> Value {
        let result = self
            .runtime
            .block_on(self.logic.as_ref().unwrap().client.control_call(
                self.one,
                1,
                bbt_mux::mcp::Call {
                    op_id: Ulid::new(),
                    tool: "artifact_claim".into(),
                    args: json!({"kind":"file","ref":reference,"digest":digest}),
                },
            ))
            .unwrap();
        assert_eq!(result["isError"], false, "{result}");
        result["structuredContent"].clone()
    }
    fn restart(&mut self) {
        if let Some(logic) = self.logic.take() {
            logic.shutdown().unwrap();
        }
        self.logic = Some(
            Logic::start(
                self.supervisor.as_ref().unwrap().connect().unwrap(),
                self.config.clone(),
                key(),
                self.runtime.handle(),
            )
            .unwrap(),
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(logic) = self.logic.take() {
            let _ = logic.shutdown();
        }
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.shutdown();
        }
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[81; 32])
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn exercise_claim(f: &Fixture) -> Document {
    std::fs::write(f.config.repository.join("out.txt"), b"alpha").unwrap();
    let doc = f.wait("out.txt", |d| d.metadata["digest"] == digest(b"alpha"));
    assert_eq!(doc.metadata["claimed"], false);
    assert_eq!(doc.metadata["authorship"]["kind"], "local-dev");
    assert!(
        doc.metadata["updated_by"]
            .as_str()
            .unwrap()
            .starts_with("observer:local-dev:")
    );
    verify_document(&doc);
    let claim = f.claim("out.txt", &digest(b"alpha"));
    assert_eq!(claim["artifact_id"], doc.id);
    let doc = f.wait("out.txt", |d| d.metadata["claimed"] == true);
    assert_eq!(
        f.documents()
            .into_iter()
            .filter(|d| d.metadata["ref"] == "out.txt")
            .count(),
        1
    );
    assert_eq!(doc.metadata["produced_by"], format!("seat:{}", f.one));
    verify_document(&doc);
    doc
}

// keeps: spec provenance
#[test]
fn live_detection_claim_hook_rename_delete_and_restart_preserve_evidence() {
    let mut f = Fixture::new();
    let claimed = exercise_claim(&f);
    let path = f.config.repository.join("out.txt");
    std::fs::write(&path, b"beta").unwrap();
    f.hook(&path, "fixture-private-body-never-an-event");
    let modified = f.wait("out.txt", |d| {
        d.metadata["digest"] == digest(b"beta")
            && d.metadata["history"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["grade"] == "reported" && entry["kind"] != "claimed")
    });
    assert_eq!(modified.id, claimed.id);
    assert_eq!(modified.metadata["produced_by"], format!("seat:{}", f.one));
    std::fs::rename(&path, f.config.repository.join("renamed.txt")).unwrap();
    let renamed = f.wait("renamed.txt", |d| d.metadata["digest"] == digest(b"beta"));
    if renamed.id == claimed.id {
        assert!(
            renamed.metadata["aliases"]
                .as_array()
                .unwrap()
                .contains(&json!("out.txt"))
        );
    } else {
        f.wait("out.txt", |d| d.metadata["deleted"] == true);
    }
    std::fs::remove_file(f.config.repository.join("renamed.txt")).unwrap();
    f.wait("renamed.txt", |d| d.metadata["deleted"] == true);
    let rows = f.rows();
    assert!(rows.iter().any(|row| {
        row.event
            .subject
            .starts_with(&format!("artifact/{}/", claimed.id))
    }));
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("fixture-private-body-never-an-event")
    );
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("not-to-publish")
    );
    for row in &rows {
        row.verify().unwrap();
    }
    let hashes: Vec<_> = rows.iter().map(|r| (r.seq, r.hash().unwrap())).collect();
    f.restart();
    std::thread::sleep(Duration::from_millis(400));
    for (sequence, hash) in hashes {
        assert_eq!(
            EventLog::open(&f.config.run_dir.join("index"))
                .unwrap()
                .at(sequence)
                .unwrap()
                .unwrap()
                .hash()
                .unwrap(),
            hash
        );
    }
    f.wait("renamed.txt", |d| d.metadata["deleted"] == true);
}

// keeps: spec provenance
#[test]
fn symlinks_outside_paths_and_oversized_files_never_leak_content_or_partial_hashes() {
    let f = Fixture::new();
    let root = &f.config.repository;
    let outside = root.parent().unwrap().join("outside.txt");
    std::fs::write(&outside, b"outside-private-fixture").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("evil.txt")).unwrap();
    f.hook(&outside, "outside-private-fixture");
    let large = std::fs::File::create(root.join("large.bin")).unwrap();
    large.set_len(64 * 1024 * 1024 + 1).unwrap();
    let doc = f.wait("large.bin", |d| {
        d.metadata["observation_reason"] == "artifact-digest-size-bound"
    });
    assert!(doc.metadata["digest"].is_null());
    assert_eq!(doc.metadata["bytes"], 64 * 1024 * 1024 + 1u64);
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !f.documents()
            .iter()
            .any(|d| d.metadata["ref"] == "evil.txt")
    );
    let rows = serde_json::to_string(&f.rows()).unwrap();
    assert!(!rows.contains("outside-private-fixture"));
    assert!(rows.contains("artifact-hook-path-outside-root-or-excluded"));
}

// keeps: spec provenance
#[test]
#[ignore = "explicit corpus driver; caller supplies a disposable output root"]
fn provenance_corpus_driver() {
    let root = PathBuf::from(
        std::env::var_os("BBOXY_PROVENANCE_CORPUS_OUT").expect("explicit fixture root"),
    );
    let f = Fixture::at(&root, None);
    let _ = exercise_claim(&f);
    println!("CORPUS_ROOT {}", f.config.repository.display());
}

fn verify_document(doc: &Document) {
    use base64::Engine;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let public = base64::engine::general_purpose::STANDARD
        .decode(doc.metadata["authorship"]["public_key"].as_str().unwrap())
        .unwrap();
    let key = VerifyingKey::from_bytes(&public.try_into().unwrap()).unwrap();
    let signature = Signature::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(doc.metadata["provenance"]["sig"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let mut metadata = doc.metadata.clone();
    metadata.as_object_mut().unwrap().remove("provenance");
    let front = serde_yaml::to_string(&metadata).unwrap();
    let front_hash = format!("{:x}", Sha256::digest(front.as_bytes()));
    let body_hash = format!("{:x}", Sha256::digest(doc.body.as_bytes()));
    assert_eq!(
        doc.metadata["provenance"]["frontmatter_digest"],
        format!("sha256:{front_hash}")
    );
    assert_eq!(
        doc.metadata["provenance"]["digest"],
        format!("sha256:{body_hash}")
    );
    let payload = format!("{front_hash}{body_hash}");
    let domain = "bboxy/item/v1";
    let pae = format!(
        "DSSEv1 {} {domain} {} {payload}",
        domain.len(),
        payload.len()
    );
    key.verify(pae.as_bytes(), &signature).unwrap();
    let changed = pae.replace(&body_hash, &format!("{:x}", Sha256::digest(b"tampered")));
    assert!(key.verify(changed.as_bytes(), &signature).is_err());
}

#[derive(Default)]
struct MemoryCustody {
    blocked: std::sync::atomic::AtomicBool,
    keys: std::sync::Mutex<std::collections::BTreeMap<Ulid, SigningKey>>,
}
impl bbt_mux::identity::DeviceKeyStore for MemoryCustody {
    fn custody(&self) -> bbt_mux::identity::Custody {
        bbt_mux::identity::Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        if self.blocked.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(bbt_mux::identity::PendingReason::KeychainInteractionRequired);
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        let mut keys = self.keys.lock().unwrap();
        if keys.contains_key(&id) {
            return Err(bbt_mux::identity::PendingReason::JournalConflict);
        }
        keys.insert(id, key.clone());
        Ok(())
    }
}
// keeps: spec provenance §W2.6 and id1-seat-chain §4: immutable observer signer during custody hold.
#[test]
#[ignore = "#327 B2: real actor custody/projection boundary; scripts/night-test-boundaries.sh"]
fn custody_hold_keeps_observations_ephemeral_until_legitimate_projection_recovers() {
    use std::sync::{Arc, atomic::Ordering};
    let memory = Arc::new(MemoryCustody::default());
    memory.blocked.store(true, Ordering::SeqCst);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_owned();
    let f = Fixture::construct(&root, Some(temp), Some(memory.clone()));
    std::fs::write(f.config.repository.join("held.txt"), b"held-content").unwrap();
    let until = Instant::now() + Duration::from_secs(8);
    let observed = loop {
        if let Some(row) = f
            .rows()
            .into_iter()
            .find(|row| row.event.payload["observation"]["reference"] == "held.txt")
        {
            break row;
        }
        assert!(Instant::now() < until, "missing held observation");
        std::thread::sleep(Duration::from_millis(20));
    };
    observed.verify().unwrap();
    assert!(
        observed.event.source.starts_with("ephemeral-hold/"),
        "{}",
        observed.event.source
    );
    assert!(f.documents().is_empty());
    assert!(
        f.rows()
            .iter()
            .any(|row| row.event.payload["reason"] == "artifact-projection-signing-held")
    );
    let original = serde_json::to_vec(&observed).unwrap();
    memory.blocked.store(false, Ordering::SeqCst);
    let until = Instant::now() + Duration::from_secs(36);
    while f.documents().is_empty() {
        assert!(
            Instant::now() < until,
            "30s custody retry never projected the artifact"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let doc = f.wait("held.txt", |doc| {
        doc.metadata["digest"] == digest(b"held-content")
    });
    assert_eq!(doc.metadata["authorship"]["kind"], "local-dev");
    verify_document(&doc);
    assert!(
        doc.metadata["history"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["source"] == observed.event.source)
    );
    let restored = EventLog::open(&f.config.run_dir.join("index"))
        .unwrap()
        .at(observed.seq)
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), original);
    assert_ne!(restored.provenance.signer, base64_public(&doc));
    let mut tampered = restored;
    tampered.event.payload["observation"]["digest"] = json!("sha256:forged");
    assert!(tampered.verify().is_err());
}
fn base64_public(doc: &Document) -> [u8; 32] {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(doc.metadata["authorship"]["public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap()
}

// keeps: spec provenance
#[test]
fn claim_before_detection_survives_missing_rebuildable_mirror() {
    let mut f = Fixture::new();
    let claimed = f.claim("claimed-first.txt", &digest(b"first"));
    let id = claimed["artifact_id"].as_str().unwrap().to_owned();
    f.logic.take().unwrap().shutdown().unwrap();
    let mirror =
        bbt_mux::store::sqlite::SqliteStore::open(&f.config.run_dir.join("index")).unwrap();
    mirror
        .connection()
        .unwrap()
        .execute(
            "DELETE FROM kv WHERE key=?1",
            [format!("doc/artifacts/{id}.md")],
        )
        .unwrap();
    f.restart();
    std::fs::write(f.config.repository.join("claimed-first.txt"), b"first").unwrap();
    let detected = f.wait("claimed-first.txt", |doc| doc.metadata["observed"] == true);
    assert_eq!(detected.id, id);
    assert!(detected.metadata["claimed"] == true);
    assert_eq!(
        f.documents()
            .into_iter()
            .filter(|doc| doc.metadata["ref"] == "claimed-first.txt")
            .count(),
        1
    );
}

// keeps: regression unchanged clipped Git refs; completed-snapshot proof is in detector tests.
#[test]
#[ignore = "#327 B2: real Git observer cadence; scripts/night-test-boundaries.sh"]
fn unchanged_clipped_git_refs_produce_no_activity_for_ten_seconds() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    fn git(repository: &Path, args: &[&str], input: Option<&[u8]>) -> Vec<u8> {
        let mut child = Command::new("git")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .args([
                "--no-optional-locks",
                "-c",
                "core.hooksPath=/dev/null",
                "-C",
            ])
            .arg(repository)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child.stdin.take().unwrap().write_all(input).unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_owned();
    let repository = root.join("repository");
    std::fs::create_dir(&repository).unwrap();
    git(
        &repository,
        &["init", "--quiet", "--initial-branch=main"],
        None,
    );
    // Copy public signed objects; the fixture never opens a signing agent.
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let objects: Vec<String> = ["HEAD", "HEAD^"]
        .iter()
        .map(|revision| {
            let commit = git(&source, &["cat-file", "commit", revision], None);
            String::from_utf8(git(
                &repository,
                &["hash-object", "-t", "commit", "-w", "--stdin"],
                Some(&commit),
            ))
            .unwrap()
            .trim()
            .into()
        })
        .collect();
    let mut updates = format!("update refs/heads/main {}\n", objects[0]);
    for number in 0..129 {
        updates.push_str(&format!(
            "update refs/heads/ref-{number:03} {}\n",
            objects[0]
        ));
    }
    git(
        &repository,
        &["update-ref", "--stdin"],
        Some(updates.as_bytes()),
    );
    let f = Fixture::at(&root, Some(temp));
    let gaps = |rows: &[bbt_mux::store::events::Envelope]| {
        rows.iter()
            .filter(|row| {
                row.event.payload["reason"] == "git-ref-scan-bound: retained first128 refs"
            })
            .count()
    };
    let until = Instant::now() + Duration::from_secs(15);
    let initial = loop {
        let rows = f.rows();
        if gaps(&rows) == 1 {
            break rows;
        }
        assert!(
            Instant::now() < until,
            "first clipped-ref warning missing: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    for row in &initial {
        row.verify().unwrap();
    }
    let cursor = initial.last().unwrap().seq;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        let added: Vec<_> = f
            .rows()
            .into_iter()
            .filter(|row| row.seq > cursor)
            .collect();
        assert!(
            added.is_empty(),
            "unchanged refs produced activity: {added:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    // A real retained-ref change starts a new incomplete-snapshot state.
    git(
        &repository,
        &["update-ref", "refs/heads/ref-000", &objects[1]],
        None,
    );
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let rows = f.rows();
        if gaps(&rows) == 2 {
            break;
        }
        assert!(
            Instant::now() < until,
            "changed refs did not produce a new warning: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
