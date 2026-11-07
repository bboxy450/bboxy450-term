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
use crate::common;
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

#[test]
fn twenty_commit_rebase_keeps_control_requests_responsive() {
    use std::process::{Command, Stdio};
    fn git(root: &Path, args: &[&str]) -> String {
        // Exercise native Git, without the operator's PATH wrappers or inherited
        // repository/object-directory settings affecting this private fixture.
        let output = Command::new("/usr/bin/git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("LC_ALL", "C")
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Watcher fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
    let f = Fixture::new();
    let root = &f.config.repository;
    git(root, &["init", "--quiet", "--initial-branch=main"]);
    std::fs::write(root.join(".gitignore"), ".bboxy/\n").unwrap();
    std::fs::create_dir(root.join("src")).unwrap();
    for n in 0..64 {
        std::fs::write(root.join(format!("src/{n}.txt")), "base\n").unwrap();
    }
    git(root, &["add", ".gitignore", "src"]);
    git(root, &["commit", "--quiet", "-m", "base"]);
    let base = git(root, &["rev-parse", "HEAD"]);
    git(root, &["checkout", "--quiet", "-b", "topic"]);
    for revision in 0..20 {
        for n in 0..64 {
            std::fs::write(
                root.join(format!("src/{n}.txt")),
                format!("revision {revision} file {n}\n"),
            )
            .unwrap();
        }
        git(
            root,
            &["commit", "--quiet", "-am", &format!("change {revision}")],
        );
    }
    git(root, &["checkout", "--quiet", "main"]);
    std::fs::write(root.join("README.md"), "upstream change\n").unwrap();
    git(root, &["add", "README.md"]);
    git(root, &["commit", "--quiet", "-m", "upstream"]);
    git(root, &["checkout", "--quiet", "topic"]);
    let path = root.to_owned();
    let rebasing =
        std::thread::spawn(move || git(&path, &["rebase", "--onto", "main", &base, "topic"]));
    let started = Instant::now();
    let mut samples = 0;
    let mut maximum = Duration::ZERO;
    loop {
        let at = Instant::now();
        f.runtime.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(2),
                f.logic.as_ref().unwrap().client.snapshot(f.one, 1),
            )
            .await
            .expect("filesystem burst blocked control requests")
            .unwrap();
        });
        maximum = maximum.max(at.elapsed());
        samples += 1;
        if rebasing.is_finished() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "private rebase hung"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    rebasing.join().unwrap();
    assert!(samples > 1);
    assert_eq!(git(root, &["rev-list", "--count", "main..topic"]), "20");
    assert_eq!(
        std::fs::read_to_string(root.join("src/0.txt")).unwrap(),
        "revision 19 file 0\n"
    );
    eprintln!(
        "watcher_rebase commits=20 files=64 control_reads={samples} max_control_us={} elapsed_ms={}",
        maximum.as_micros(),
        started.elapsed().as_millis()
    );
}

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
        let call = bbt_mux::mcp::Call {
            op_id: Ulid::new(),
            tool: "artifact_claim".into(),
            args: json!({"kind":"file","ref":reference,"digest":digest}),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        let result = loop {
            let result = self
                .runtime
                .block_on(self.logic.as_ref().unwrap().client.control_call(
                    self.one,
                    1,
                    call.clone(),
                ))
                .unwrap();
            if result["structuredContent"]["error"]["code"] != "workspace_pending" {
                break result;
            }
            assert!(
                Instant::now() < deadline,
                "workspace never settled before artifact claim: {result}"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
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

#[test]
fn native_knowledge_mcp_exposes_draft_commit_and_edited_draft_states() {
    let f = Fixture::new();
    let call =
        |tool: &str, args: Value| {
            let call = bbt_mux::mcp::Call {
                op_id: Ulid::new(),
                tool: tool.into(),
                args,
            };
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                let result =
                    f.runtime
                        .block_on(f.logic.as_ref().unwrap().client.control_call(
                            f.one,
                            1,
                            call.clone(),
                        ))
                        .unwrap();
                let code = result["structuredContent"]["error"]["code"]
                    .as_str()
                    .unwrap_or_default();
                if !matches!(
                    code,
                    "workspace_pending" | "provider_busy" | "workspace_refresh_pending"
                ) {
                    return result;
                }
                assert!(Instant::now() < until, "workspace did not settle: {result}");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
    let created = call(
        "knowledge_write",
        json!({"path":"notes/checkpoint.md","frontmatter":{"type":"note","title":"Checkpoint"},"body":"draft body"}),
    );
    assert_eq!(created["isError"], false, "{created}");
    assert_eq!(created["structuredContent"]["publication_state"], "draft");
    let root = f.config.repository.join(".bboxy");
    let signing = f.config.run_dir.join("fixture-signing");
    assert!(
        std::process::Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&signing)
            .status()
            .unwrap()
            .success()
    );
    let git = |args: &[&str]| {
        let output = std::process::Command::new("/usr/bin/git")
            .current_dir(&root)
            .args(["-c", "core.hooksPath=/dev/null"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet", "--initial-branch=main", "--template="]);
    git(&["config", "user.name", "Fixture"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    git(&["config", "gpg.format", "ssh"]);
    git(&["config", "user.signingkey", signing.to_str().unwrap()]);
    let committed = call(
        "knowledge_commit",
        json!({"locators":["notes/checkpoint.md"],"message":"Substantial checkpoint"}),
    );
    assert_eq!(committed["isError"], false, "{committed}");
    assert_eq!(
        committed["structuredContent"]["publication_state"],
        "committed"
    );
    let fetched = call("knowledge_fetch", json!({"locator":"notes/checkpoint.md"}));
    assert_eq!(fetched["isError"], false, "{fetched}");
    assert_eq!(
        fetched["structuredContent"]["publication_state"],
        "committed"
    );
    let old = &fetched["structuredContent"];
    let reference: bbt_mux::knowledge::data_source::RecordRef =
        serde_json::from_value(old.clone()).unwrap();
    let edited = call(
        "knowledge_write",
        json!({"ref":reference,"etag":old["etag"],"frontmatter":old["frontmatter"],"body":"next draft"}),
    );
    assert_eq!(edited["isError"], false, "{edited}");
    let fetched = call("knowledge_fetch", json!({"locator":"notes/checkpoint.md"}));
    assert_eq!(
        fetched["structuredContent"]["publication_state"], "draft",
        "{fetched}"
    );
    assert_eq!(
        f.rows()
            .iter()
            .filter(|row| row.event.source == "knowledge/commit"
                && row.event.subject.starts_with("artifact/"))
            .count(),
        1
    );
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
fn draft_edits_hooks_renames_and_restart_do_not_publish_artifacts() {
    let mut f = Fixture::new();
    let path = f.config.repository.join("out.txt");
    std::fs::write(&path, b"alpha").unwrap();
    f.hook(&path, "private draft body");
    std::fs::write(&path, b"beta").unwrap();
    f.hook(&path, "private draft body");
    std::fs::rename(&path, f.config.repository.join("renamed.txt")).unwrap();
    std::fs::remove_file(f.config.repository.join("renamed.txt")).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(f.documents().is_empty());
    assert!(
        f.rows()
            .iter()
            .all(|row| !row.event.subject.starts_with("artifact/"))
    );
    let rows = f.rows();
    for row in &rows {
        row.verify().unwrap();
    }
    f.restart();
    std::thread::sleep(Duration::from_millis(300));
    assert!(f.documents().is_empty());
    for row in rows {
        assert_eq!(
            EventLog::open(&f.config.run_dir.join("index"))
                .unwrap()
                .at(row.seq)
                .unwrap()
                .unwrap()
                .hash()
                .unwrap(),
            row.hash().unwrap()
        );
    }
}

// keeps: spec provenance
#[test]
fn import_staging_and_ordinary_drafts_stay_outside_artifact_publication() {
    let f = Fixture::new();
    for reference in [
        "out.txt",
        ".bboxy-import-notes/out.txt",
        ".bboxy-import-c025b21f11d2-01M64FDAXVG0N8AN7DX3MNZT3H/imports/raw.txt",
    ] {
        let path = f.config.repository.join(reference);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"draft").unwrap();
        f.hook(&path, "draft");
    }
    std::thread::sleep(Duration::from_millis(300));
    assert!(f.documents().is_empty());
    assert!(
        f.rows()
            .iter()
            .all(|row| !row.event.subject.starts_with("artifact/"))
    );
}

// keeps: spec provenance
#[test]
fn draft_symlinks_and_large_build_files_are_never_hashed_or_published() {
    let f = Fixture::new();
    let root = &f.config.repository;
    let outside = root.parent().unwrap().join("outside.txt");
    std::fs::write(&outside, b"outside-private-fixture").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("evil.txt")).unwrap();
    f.hook(&outside, "outside-private-fixture");
    let large = std::fs::File::create(root.join("large.bin")).unwrap();
    large.set_len(64 * 1024 * 1024 + 1).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(f.documents().is_empty());
    assert!(
        f.rows()
            .iter()
            .all(|row| !row.event.subject.starts_with("artifact/"))
    );
    assert!(
        !serde_json::to_string(&f.rows())
            .unwrap()
            .contains("outside-private-fixture")
    );
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
fn claiming_work_in_progress_does_not_promote_it_to_an_artifact() {
    let f = Fixture::new();
    let response = f
        .runtime
        .block_on(f.logic.as_ref().unwrap().client.control_call(
            f.one,
            1,
            bbt_mux::mcp::Call {
                op_id: Ulid::new(),
                tool: "artifact_claim".into(),
                args: json!({"kind":"file","ref":"uncommitted.txt","digest":digest(b"draft")}),
            },
        ))
        .unwrap();
    assert_eq!(response["isError"], true, "{response}");
    assert!(f.documents().is_empty());
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

#[test]
fn native_concurrent_knowledge_writes_wait_instead_of_refusing_provider_busy() {
    let f = Fixture::new();
    let client = f.logic.as_ref().unwrap().client.clone();
    let seat = f.one;
    f.runtime.block_on(async {
        let mut jobs = tokio::task::JoinSet::new();
        for n in 0..16 {
            let client = client.clone();
            jobs.spawn(async move {
                let call = bbt_mux::mcp::Call {
                    op_id: Ulid::new(),
                    tool: "knowledge_write".into(),
                    args: json!({"path":format!("notes/concurrent-{n}.md"),
                        "frontmatter":{"type":"note","title":format!("Concurrent {n}")},
                        "body":format!("private writer {n}")}),
                };
                client.control_call(seat, 1, call).await.unwrap()
            });
        }
        tokio::time::timeout(Duration::from_secs(60), async {
            while let Some(result) = jobs.join_next().await {
                let value = result.unwrap();
                assert_ne!(
                    value["isError"], true,
                    "a concurrent write was refused: {value}"
                );
            }
        })
        .await
        .expect("queued writes did not make progress");
    });
    for n in 0..16 {
        let path = f
            .config
            .repository
            .join(format!(".bboxy/notes/concurrent-{n}.md"));
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains(&format!("private writer {n}"))
        );
    }
}

// Host MCP route, own-record visibility, immutable evidence reads and replay.
#[test]
fn provenance_mcp_queries_are_catalogued_scoped_bounded_and_read_only() {
    let f=Fixture::new();
    let call=|tool:&str,args:Value| {
        let end=Instant::now()+Duration::from_secs(12);
        loop {
            let value=f.runtime.block_on(f.logic.as_ref().unwrap().client.control_call(f.one,1,
                bbt_mux::mcp::Call{op_id:Ulid::new(),tool:tool.into(),args:args.clone()})).unwrap();
            if ["workspace_pending","provider_busy","refresh_pending"].iter().any(|code|value["structuredContent"]["error"]["code"]==*code) {
                assert!(Instant::now()<end,"{value}"); std::thread::sleep(Duration::from_millis(10)); continue;
            }
            assert_eq!(value["isError"],false,"{value}");
            return value["structuredContent"].clone();
        }
    };
    let written=call("knowledge_write",json!({"path":"notes/provenance-query.md","frontmatter":{"type":"note","title":"provenance query fixture"},"body":"Private signed fixture."}));
    let id=written["record"]["id"].as_str().unwrap().to_owned();
    let end=Instant::now()+Duration::from_secs(8);
    loop {
        let found=call("provenance_search",json!({"query":"provenance query fixture","limit":1}));
        if found["items"].as_array().is_some_and(|items|items.iter().any(|v|v["id"]==id)) {break;}
        assert!(Instant::now()<end,"cached native index never reflected the write: {found}");
        std::thread::sleep(Duration::from_millis(20));
    }
    let entity=call("provenance_get",json!({"locator":id}));
    assert_eq!(entity["entity"]["publication"]["state"],"draft");
    assert_eq!(entity["entity"]["document_signer"]["principal"],format!("seat:{}",f.one));
    assert_eq!(entity["knowledge_root_id"].as_str().unwrap().len(),64);
    assert_eq!(entity["freshness"]["repair_triggered"],false);
    let history=call("provenance_history",json!({"locator":id,"limit":1}));
    assert_eq!(history["items"].as_array().unwrap().len(),1);
    assert!(history["next"].is_string());
    let next=call("provenance_history",json!({"locator":id,"limit":1,"cursor":history["next"]}));
    assert_eq!(next["items"].as_array().unwrap().len(),1);
    assert_ne!(next["items"][0]["sequence"],history["items"][0]["sequence"]);
    let verified=call("provenance_verify",json!({"locator":id}));
    assert_eq!(verified["document_signature"]["status"],"valid");
    assert_eq!(verified["event"]["status"],"valid");
    assert_eq!(verified["limits"]["account_key_trust"],"not_checked");
    let exact=call("provenance_verify",json!({"locator":id,"sequence":history["items"][0]["sequence"]}));
    assert_eq!(exact["event"]["status"],"valid");
    assert_eq!(exact["event"]["recorded_state_matches"],true);

    let lineage=call("provenance_lineage",json!({"locator":id,"depth":1}));
    assert_eq!(lineage["nodes"].as_array().unwrap().len(),1);
    assert!(lineage["edges"].as_array().unwrap().is_empty());
    let op=Ulid::new();
    let request=bbt_mux::mcp::Call{op_id:op,tool:"provenance_get".into(),args:json!({"locator":id})};
    let first=f.runtime.block_on(f.logic.as_ref().unwrap().client.control_call(f.one,1,request.clone())).unwrap();
    let retry=f.runtime.block_on(f.logic.as_ref().unwrap().client.control_call(f.one,1,request)).unwrap();
    assert_eq!(first,retry,"same operation must retain its scoped snapshot");
}
