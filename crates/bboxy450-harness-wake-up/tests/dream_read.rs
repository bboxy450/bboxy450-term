// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Consumer conformance against the actual pinned producer corpus, without local goldens.
//! Adapted from bboxy450-harness src/dream/read_wire.rs conformance tests and
//! bboxy450-term crates/bboxy450-harness-wake-up/tests/dream_c1.rs corpus checks.
//! Producer source: https://github.com/bboxy450/bboxy450-harness/blob/f848c03ff0462e55dfa0b17fcfd550626a52db2b/src/dream/read_wire.rs
#![allow(
    clippy::disallowed_methods,
    reason = "private producer-corpus codec fixtures"
)]
use bboxy450_harness_wake_up::{
    dream_bootstrap as bootstrap,
    dream_c1::{PreparedStart, Worker, wire::Code as C1Code},
    dream_read::{self as read, wire as w},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn fixtures() -> PathBuf {
    std::env::var_os("DREAM_READ_FIXTURES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../bboxy450-harness/tests/fixtures/dream-read")
        })
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name))
        .unwrap_or_else(|error| panic!("missing producer read fixture {name}: {error}"))
}

#[test]
fn dream_read_prepare_uses_the_owned_spawner_and_rejects_cross_purpose_start() {
    let read_hello = String::from_utf8(bytes("valid/hello.json")).unwrap();
    let inspect_hello = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../bboxy450-harness/tests/fixtures/dream-c1/valid/hello.json"),
    )
    .unwrap();
    let start_bytes = bytes("valid/start.json");
    let w::WorkerIn::Start(start) = read::decode(&start_bytes).unwrap() else {
        panic!("producer fixture must be Start");
    };
    let read_start = PreparedStart::read(&start).unwrap();
    assert_eq!(
        read_start.as_bytes().strip_suffix(b"\n").unwrap(),
        start_bytes
    );
    let inspect_start = PreparedStart::from_parts(
        &start.run_id.0,
        &start.nonce.0,
        &"0".repeat(64),
        start.timeout_ms,
        start.activity_revision.0,
        start.foreground,
    )
    .unwrap();

    for (hello, frame, probe, may_start) in [
        (&read_hello, &read_start, false, true),
        (&read_hello, &inspect_start, false, false),
        (&read_hello, &read_start, true, false),
        (&inspect_hello, &read_start, false, false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let job = root.path().canonicalize().unwrap().join(&start.run_id.0);
        std::fs::create_dir(&job).unwrap();
        std::fs::set_permissions(&job, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(job.join("tmp")).unwrap();
        let binary = root.path().canonicalize().unwrap().join("worker.py");
        // Adapted from this crate's tests/dream_c1.rs private transport fixture.
        std::fs::write(
            &binary,
            format!(
                r#"#!/usr/bin/python3
import os, pathlib, sys
gate = int(os.environ['BBOXY_DREAM_BOOTSTRAP'].split(':')[1])
permit = os.read(gate, 128)
os.close(gate)
assert len(permit) == 128 and permit[8] == 2
assert sys.argv[1:4] == ['dream', 'prepare-read', '--job-dir']
assert os.environ['LANG'] == 'C.UTF-8'
assert os.environ['TMPDIR'] == str(pathlib.Path(sys.argv[4]) / 'tmp')
assert not any(k in ('HOME', 'PATH') or (k != 'BBOXY_DREAM_BOOTSTRAP' and k.startswith(('BBOXY_', 'OPENAI_', 'ANTHROPIC_', 'CODEX_', 'CLAUDE_'))) for k in os.environ)
print({hello:?}, flush=True)
pathlib.Path('captured-input').write_bytes(sys.stdin.buffer.read())
"#
            ),
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        let frozen = bootstrap::Frozen {
            purpose: bootstrap::Purpose::ReadPrepare,
            ..bootstrap::Frozen::inspect(
                probe,
                ulid::Ulid::new().to_bytes(),
                bootstrap::ulid(start.run_id.0.as_bytes()).unwrap(),
                bootstrap::ulid(start.nonce.0.as_bytes()).unwrap(),
                [0; 32],
                1,
            )
        };
        let mut command = Worker::prepare_command(
            &binary,
            &job,
            Worker::prepare_buffers().unwrap(),
            Worker::prepare_pipe().unwrap(),
            Worker::prepare_pipe().unwrap(),
            Worker::prepare_pipe().unwrap(),
            bootstrap::Gate::prepare(frozen).unwrap(),
        )
        .unwrap();
        command.configure_input().unwrap();
        command.configure_output().unwrap();
        command.configure_stderr().unwrap();
        let child = command.take_command().unwrap().spawn().unwrap();
        let returned = Instant::now();
        let (mut worker, unused) = command.attach_child(child);
        drop(unused);
        worker
            .bootstrap_spawn_returned(returned, returned + Duration::from_secs(30))
            .unwrap();
        worker.bootstrap_grant().unwrap();
        let until = worker.bootstrap_deadline().unwrap();
        let qualified = loop {
            assert!(Instant::now() < until);
            match worker.read_hello_step() {
                Ok(false) => worker.wait_io(Duration::from_millis(1)),
                Ok(true) => break true,
                Err(C1Code::ProtocolError | C1Code::VersionMismatch) => break false,
                Err(code) => panic!("private read worker Hello: {code:?}"),
            }
        };
        assert_eq!(qualified, hello == &read_hello);
        assert_eq!(worker.bootstrap_observation().qualified_hello, qualified);
        if may_start {
            worker.send_prepared_start(frame).unwrap();
        } else {
            assert_eq!(
                worker.send_prepared_start(frame),
                Err(C1Code::ProtocolError)
            );
        }
        assert_eq!(worker.start_write_observation().attempted, may_start);
        worker.close_input();
        let pid = worker.pid();
        let reap_by = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(status) = worker.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < reap_by);
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(status.success());
        let mut status = 0;
        // SAFETY: queries only this fixture's exact child PID, without signaling.
        assert_eq!(
            unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        assert_eq!(
            std::fs::read(job.join("captured-input")).unwrap(),
            if may_start {
                read_start.as_bytes()
            } else {
                &[]
            },
        );
    }
}

fn typed<T: Serialize + DeserializeOwned>(bytes: &[u8]) -> Result<Vec<u8>, w::Code> {
    read::canonical(&read::decode::<T>(bytes)?).map_err(|_| w::Code::ProtocolError)
}

fn dto(name: &str, bytes: &[u8]) -> Result<Vec<u8>, w::Code> {
    match name {
        "input" => typed::<w::Input>(bytes),
        "snapshot" => typed::<w::Snapshot>(bytes),
        "page" => typed::<w::Page>(bytes),
        "start" | "activity" | "cancel" => typed::<w::WorkerIn>(bytes),
        "hello" | "progress" | "done" | "unproven" => typed::<w::WorkerOut>(bytes),
        _ => panic!("unknown producer read fixture {name}"),
    }
}

#[test]
fn dream_read_canonical_producer_corpus_and_negative_controls_conform() {
    let lock: Value = serde_json::from_slice(&bytes("lock.json")).unwrap();
    let mut valid = 0;
    for name in lock["files"].as_object().unwrap().keys() {
        if name.starts_with("valid/") {
            let stem = name
                .strip_prefix("valid/")
                .unwrap()
                .strip_suffix(".json")
                .unwrap();
            let raw = bytes(name);
            assert_eq!(dto(stem, &raw).unwrap(), raw, "producer roundtrip: {name}");
            valid += 1;
        }
    }
    assert_eq!(
        valid, 10,
        "all pinned producer DTO fixtures must be exercised"
    );
    let invalid: Vec<Value> = serde_json::from_slice(&bytes("invalid/cases.json")).unwrap();
    assert!(!invalid.is_empty());
    for case in invalid {
        assert!(
            dto(
                case["dto"].as_str().unwrap(),
                case["bytes"].as_str().unwrap().as_bytes()
            )
            .is_err(),
            "consumer accepted producer negative control: {}",
            case["name"]
        );
    }
}

#[test]
fn dream_read_independent_hashes_pin_schema_lock_every_fixture_and_vectors() {
    let output = Command::new("/usr/bin/python3").env_clear().args(["-I", "-c", r#"
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1])
digest=lambda b:hashlib.sha256(b).hexdigest()
raw=(root/'lock.json').read_bytes()
assert digest(raw)==sys.argv[3], 'producer fixture lock changed'
lock=json.loads(raw)
schema=(root.parents[2]/'schema/dream-read-v1.schema.json').read_bytes()
assert digest(schema)==sys.argv[2]==lock['schema_sha256'], 'producer schema changed'
for name,wanted in lock['files'].items():
    assert digest((root/name).read_bytes())==wanted, 'fixture bytes changed: '+name
for v in json.loads((root/'vectors/canonical.json').read_bytes()):
    b=json.dumps(v['value'],ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
    assert b==v['canonical'].encode() and digest(b)==v['sha256'], 'canonical vector disagreement'
v=json.loads((root/'source-revision.json').read_bytes())
assert digest(v['canonical'].encode())==v['sha256'], 'source revision vector disagreement'
"#]).arg(fixtures()).arg(read::SCHEMA_SHA256).arg(read::CORPUS_LOCK_SHA256).output().unwrap();
    assert!(
        output.status.success(),
        "independent producer pin validation: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sha256(bytes: &[u8]) -> String {
    let mut child = Command::new("/usr/bin/shasum")
        .env_clear()
        .args(["-a", "256"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn dream_read_source_revision_matches_producer_and_binds_every_component() {
    let input: w::Input = read::decode(&bytes("valid/input.json")).unwrap();
    let vector: Value = serde_json::from_slice(&bytes("source-revision.json")).unwrap();
    let original = read::source_revision_bytes(&input).unwrap();
    assert_eq!(original, vector["canonical"].as_str().unwrap().as_bytes());
    assert_eq!(
        read::source_revision(&input, sha256).unwrap().0,
        vector["sha256"]
    );
    for mutate in [
        |v: &mut w::Input| v.context.generation.0 += 1,
        |v: &mut w::Input| v.provider_revision.0.push('b'),
        |v: &mut w::Input| v.closed_sources[0].source_generation.0 += 1,
        |v: &mut w::Input| v.evidence.selected_at_ms.0 += 1,
        |v: &mut w::Input| v.trust_policy.0 = "f".repeat(64),
    ] {
        let mut changed = input.clone();
        mutate(&mut changed);
        assert_ne!(read::source_revision_bytes(&changed).unwrap(), original);
    }
    let vectors: Vec<Value> = serde_json::from_slice(&bytes("vectors/canonical.json")).unwrap();
    for vector in vectors {
        let raw = read::canonical(&vector["value"]).unwrap();
        assert_eq!(raw, vector["canonical"].as_str().unwrap().as_bytes());
        assert_eq!(
            read::canonical_limited(&vector["value"], raw.len()).unwrap(),
            raw
        );
        assert_eq!(
            read::canonical_limited(&vector["value"], raw.len() - 1),
            Err(w::Code::ResourceLimit)
        );
    }
}

#[test]
fn dream_read_hello_and_entry_shape_controls_do_not_claim_settlement_or_proof() {
    let w::WorkerOut::Hello(hello) = read::decode(&bytes("valid/hello.json")).unwrap() else {
        panic!("hello fixture")
    };
    read::verify_hello(&hello).unwrap();
    for mutate in [
        |v: &mut w::Hello| v.schema_sha256.0 = "f".repeat(64),
        |v: &mut w::Hello| v.ir_major = 2,
        |v: &mut w::Hello| v.ir_minor_max = 4,
    ] {
        let mut changed = hello.clone();
        mutate(&mut changed);
        assert_eq!(read::verify_hello(&changed), Err(w::Code::ProtocolError));
    }
    let page: w::Page = read::decode(&bytes("valid/page.json")).unwrap();
    let entry = page.entries[0].clone();
    read::validate_entry(&entry).unwrap();
    let mut changed = entry.clone();
    changed.why = "x".repeat(2049);
    assert_eq!(read::validate_entry(&changed), Err(w::Code::ResourceLimit));
    let mut changed = entry;
    changed.hit.id.0 = "foreign".into();
    assert_eq!(
        read::validate_entry(&changed),
        Err(w::Code::VerificationFailed)
    );
    assert!(read::decode::<w::Page>(&[]).is_err());
    assert!(read::decode::<w::Page>(&vec![b' '; w::MAX_FRAME + 1]).is_err());
}
