// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Producer-corpus conformance and private real-process transport witnesses.
//! Scripted workers below test transport failures only; they are not producer proof.
#![allow(
    clippy::disallowed_methods,
    reason = "private process fixtures, never a hot owner"
)]
use bboxy450_harness_wake_up::dream_bootstrap as bootstrap;
use bboxy450_harness_wake_up::dream_c1::{self as c1, PreparedStart, Worker, wire as w};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

const LOCK_SHA256: &str = "31113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31";

fn fixtures() -> PathBuf {
    let root = PathBuf::from(std::env::var_os("DREAM_C1_FIXTURES").expect(
        "DREAM_C1_FIXTURES must name the approved producer tests/fixtures/dream-c1 corpus",
    ));
    for required in ["lock.json", "valid", "invalid", "vectors/canonical.json"] {
        assert!(
            root.join(required).exists(),
            "missing producer corpus: {required}"
        );
    }
    root
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name))
        .unwrap_or_else(|error| panic!("missing canonical producer fixture {name}: {error}"))
}

fn roundtrip<T: DeserializeOwned + Serialize>(name: &str) {
    let bytes = bytes(&format!("valid/{name}.json"));
    c1::preflight_json(&bytes, c1::JsonShape::Canonical)
        .unwrap_or_else(|error| panic!("canonical wire lexical refusal {name}: {error:?}"));
    let value: T = c1::decode(&bytes)
        .unwrap_or_else(|error| panic!("consumer rejects canonical producer {name}: {error:?}"));
    assert_eq!(
        c1::canonical(&value).unwrap(),
        bytes,
        "canonical disagreement: {name}"
    );
}

#[test]
fn canonical_worker_manifest_result_and_pages_roundtrip_without_second_schema() {
    for name in ["start", "activity", "worker-cancel"] {
        roundtrip::<w::WorkerIn>(name);
    }
    for name in ["hello", "progress", "done"] {
        roundtrip::<w::WorkerOut>(name);
    }
    roundtrip::<w::Manifest>("manifest");
    roundtrip::<w::ResultDescriptor>("result");
    roundtrip::<w::Page>("page");
}

#[test]
fn prepared_start_matches_signed_producer_fixture_without_second_golden() {
    let raw = bytes("valid/start.json");
    let w::WorkerIn::Start(start) = c1::decode(&raw).unwrap() else {
        panic!("producer start fixture has the wrong variant")
    };
    let prepared = PreparedStart::new(&start).unwrap();
    assert!(prepared.as_bytes().len() <= 512);
    assert_eq!(prepared.as_bytes().last(), Some(&b'\n'));
    assert_eq!(
        prepared
            .as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count(),
        1,
        "the atomic write is exactly one complete JSON line"
    );
    let decoded: w::WorkerIn = c1::decode(prepared.as_bytes()).unwrap();
    assert_eq!(decoded, w::WorkerIn::Start(start));
    // JSON member order is not a wire constraint. Canonicalizing the prepared
    // bytes must reproduce the producer's exact signed fixture, not a local golden.
    assert_eq!(c1::canonical(&decoded).unwrap(), raw);
}

#[test]
fn prepared_start_rejects_invalid_bounded_scalars_before_any_process_effect() {
    let w::WorkerIn::Start(start) = c1::decode(&bytes("valid/start.json")).unwrap() else {
        unreachable!()
    };
    for timeout in [0, 30_001] {
        let mut bad = start.clone();
        bad.timeout_ms = timeout;
        assert!(PreparedStart::new(&bad).is_err());
    }
    for hash in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        let mut bad = start.clone();
        bad.manifest_sha256 = w::Hex(hash);
        assert!(PreparedStart::new(&bad).is_err());
    }
    let mut bad = start.clone();
    bad.activity_revision = w::Number(w::MAX_SAFE + 1);
    assert!(PreparedStart::new(&bad).is_err());
    let mut bad = start.clone();
    bad.run_id = w::Ulid("not-a-canonical-ulid".to_owned());
    assert!(PreparedStart::new(&bad).is_err());
    let mut bad = start;
    bad.nonce = w::Ulid("Z".repeat(26));
    assert!(PreparedStart::new(&bad).is_err());
}

#[test]
fn every_canonical_worker_negative_is_rejected_before_effect() {
    let root = fixtures();
    let lock: serde_json::Value = serde_json::from_slice(&bytes("lock.json")).unwrap();
    let mut count = 0;
    for (name, _) in lock["files"].as_object().expect("producer fixture lock") {
        if !name.starts_with("invalid/") {
            continue;
        }
        let raw = std::fs::read(root.join(name)).unwrap();
        let error = c1::decode::<w::WorkerIn>(&raw).expect_err(name);
        assert_eq!(
            error,
            if raw.len() > w::MAX_FRAME {
                w::Code::ResourceLimit
            } else {
                w::Code::ProtocolError
            },
            "wrong refusal: {name}"
        );
        count += 1;
    }
    assert_eq!(
        count, 10,
        "approved boundary negative corpus changed; review its pin"
    );
}

#[test]
fn independent_stdlib_hash_checks_pin_schema_every_fixture_and_vectors() {
    let result = Command::new("/usr/bin/python3").env_clear()
        .args(["-I", "-c", r#"
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
digest = lambda b: hashlib.sha256(b).hexdigest()
lock_bytes = (root / 'lock.json').read_bytes()
assert digest(lock_bytes) == sys.argv[3], 'producer fixture lock changed'
lock = json.loads(lock_bytes)
schema = (root.parents[2] / 'schema/dream-c1-v1.schema.json').read_bytes()
assert digest(schema) == sys.argv[2] == lock['schema_sha256'], 'schema pin mismatch'
for name, wanted in lock['files'].items():
    assert digest((root / name).read_bytes()) == wanted, 'fixture hash: ' + name
vectors = json.loads((root / 'vectors/canonical.json').read_bytes())
for vector in vectors:
    actual = json.dumps(vector['value'], ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    assert actual.decode() == vector['canonical'], 'canonical vector disagreement'
    assert digest(actual) == vector['sha256'], 'hash vector disagreement'
"#])
        .arg(fixtures()).arg(c1::SCHEMA_SHA256).arg(LOCK_SHA256).output().unwrap();
    assert!(
        result.status.success(),
        "independent corpus check failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Owns every script, process working directory and diagnostic touched by a witness.
struct Script {
    _root: tempfile::TempDir,
    binary: PathBuf,
    job: PathBuf,
}
impl Script {
    fn new(body: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("worker.py");
        let start: w::WorkerIn = c1::decode(&bytes("valid/start.json")).unwrap();
        let w::WorkerIn::Start(start) = start else {
            unreachable!()
        };
        let job = root.path().join(&start.run_id.0);
        std::fs::create_dir(&job).unwrap();
        std::fs::create_dir(job.join("tmp")).unwrap();
        std::fs::set_permissions(&job, std::fs::Permissions::from_mode(0o700)).unwrap();
        let job = job.canonicalize().unwrap();
        let hello = String::from_utf8(bytes("valid/hello.json")).unwrap();
        let script = format!(
            "#!/usr/bin/python3\nimport json, os, pathlib, sys, time\n_gate=int(os.environ['BBOXY_DREAM_BOOTSTRAP'].split(':')[1])\n_permit=os.read(_gate,128)\nassert len(_permit)==128\nos.close(_gate)\npathlib.Path(sys.argv[4]).joinpath('worker-started').write_text('ready')\nHELLO = {hello:?}\n{body}\n"
        );
        std::fs::write(&binary, script).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            _root: root,
            binary,
            job,
        }
    }
    fn spawn(&self) -> TestWorker {
        let start: w::WorkerIn = c1::decode(&bytes("valid/start.json")).unwrap();
        let w::WorkerIn::Start(start) = start else {
            unreachable!()
        };
        // Private transport-fault fixture only: this zero digest grants no
        // managed runner/profile/source authority and is not a binding vector.
        let frozen = bootstrap::Frozen::inspect(
            false,
            ulid::Ulid::new().to_bytes(),
            bootstrap::ulid(start.run_id.0.as_bytes()).unwrap(),
            bootstrap::ulid(start.nonce.0.as_bytes()).unwrap(),
            [0; 32],
            1,
        );
        let gate = ordinary(|| bootstrap::Gate::prepare(frozen).unwrap());
        let buffers = ordinary(|| Worker::prepare_buffers().unwrap());
        let input = ordinary(|| Worker::prepare_pipe().unwrap());
        let output = ordinary(|| Worker::prepare_pipe().unwrap());
        let stderr = ordinary(|| Worker::prepare_pipe().unwrap());
        let mut prepared = ordinary(|| {
            Worker::prepare_command(
                &self.binary,
                &self.job,
                buffers,
                input,
                output,
                stderr,
                gate,
            )
            .unwrap()
        });
        ordinary(|| prepared.configure_input().unwrap());
        ordinary(|| prepared.configure_output().unwrap());
        ordinary(|| prepared.configure_stderr().unwrap());
        let mut command = ordinary(|| prepared.take_command().unwrap());
        let cpu = thread_cpu();
        let wall = Instant::now();
        let child = command.spawn().unwrap();
        let returned = Instant::now();
        let end_cpu = thread_cpu_sample();
        let native_wall = returned.checked_duration_since(wall);
        let (worker, unused) = prepared.attach_child(child);
        let mut worker = TestWorker {
            worker,
            command: Some(command),
            unused,
        };
        let used = end_cpu.and_then(|end| end.checked_sub(cpu)).unwrap();
        assert!(
            used <= 2_000_000 && native_wall.is_some_and(|wall| wall <= Duration::from_millis(25)),
            "private native process span exceeded its observed contract"
        );
        worker
            .bootstrap_spawn_returned(returned, returned + Duration::from_secs(30))
            .unwrap();
        std::thread::sleep(Duration::from_nanos(9 * used.max(1_000_000)));
        worker.bootstrap_grant().unwrap();
        assert_eq!(
            worker.protected_capacity_bytes(),
            bboxy450_harness_wake_up::dream_c1::PROTECTED_BYTES,
            "all codec backing is provisioned before child use"
        );
        let until = worker.bootstrap_deadline().unwrap();
        while !self.job.join("worker-started").is_file() {
            assert!(
                Instant::now() < until,
                "owned Python fixture did not reach framing readiness"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        worker
    }
}

fn thread_cpu() -> u64 {
    thread_cpu_sample().expect("available finite calling-thread clock")
}
fn thread_cpu_sample() -> Option<u64> {
    let mut t = std::mem::MaybeUninit::<libc::timespec>::uninit();
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, t.as_mut_ptr()) } != 0 {
        return None;
    }
    let t = unsafe { t.assume_init() };
    let ns = u64::try_from(t.tv_nsec)
        .ok()
        .filter(|n| *n < 1_000_000_000)?;
    u64::try_from(t.tv_sec)
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(ns)
}
fn ordinary<T>(work: impl FnOnce() -> T) -> T {
    let cpu = thread_cpu();
    let value = work();
    let used = thread_cpu().checked_sub(cpu).unwrap();
    assert!(used <= 1_000_000, "private ordinary setup exceeded1ms");
    std::thread::sleep(Duration::from_nanos(9 * used.max(1_000_000)));
    value
}
struct TestWorker {
    worker: Worker,
    command: Option<Command>,
    unused: Option<Command>,
}
impl std::ops::Deref for TestWorker {
    type Target = Worker;
    fn deref(&self) -> &Worker {
        &self.worker
    }
}
impl std::ops::DerefMut for TestWorker {
    fn deref_mut(&mut self) -> &mut Worker {
        &mut self.worker
    }
}
impl TestWorker {
    fn hello(&mut self) -> Result<w::Hello, w::Code> {
        let result = self.worker.hello();
        ordinary(|| {
            self.command.take();
            self.unused.take();
        });
        result
    }
}

fn next(worker: &mut Worker) -> Result<w::WorkerOut, w::Code> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(frame) = worker.receive()? {
            return Ok(frame);
        }
        assert!(
            Instant::now() < until,
            "private worker did not produce its bounded frame"
        );
        worker.wait_io(Duration::from_millis(9));
    }
}

fn reaped(worker: &mut Worker) -> std::process::ExitStatus {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = worker.try_wait().unwrap() {
            return status;
        }
        assert!(
            Instant::now() < until,
            "private child did not exit/reap after EOF"
        );
        std::thread::sleep(Duration::from_millis(9));
    }
}

fn assert_reaped(pid: u32) {
    let mut status = 0;
    // SAFETY: queries only the exact child PID started by this witness; no signal sent.
    let waited = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
    assert_eq!(
        waited, -1,
        "Child must already have been reaped by its owner"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn actual_private_process_clears_authority_environment_and_uses_fixed_argv() {
    let script = Script::new(
        r#"
assert sys.argv[1:4] == ['dream', 'inspect-c1', '--job-dir']
job = pathlib.Path(sys.argv[4])
assert job.is_absolute() and pathlib.Path.cwd() == job
assert os.environ['LANG'] == 'C.UTF-8'
assert os.environ['TMPDIR'] == str(job / 'tmp')
assert not any(k == 'HOME' or k == 'PATH' or (k != 'BBOXY_DREAM_BOOTSTRAP' and k.startswith(('BBOXY_', 'OPENAI_', 'ANTHROPIC_', 'CODEX_', 'CLAUDE_'))) for k in os.environ)
assert os.getpriority(os.PRIO_PROCESS, 0) >= 10
(job / 'isolated').write_text('verified')
print(HELLO, flush=True)
start = json.loads(sys.stdin.readline())
assert start['type'] == 'start'
base = {'protocol':'bboxy.dream.c1/1', 'run_id':start['run_id'], 'nonce':start['nonce']}
print(json.dumps(dict(base, type='progress', seq=1, phase='paused', bytes=0, envelopes=0, cpu_ns=0)), flush=True)
print(json.dumps(dict(base, type='done', seq=2, outcome='completed', result_sha256='0'*64, pages=1, error=None)), flush=True)
"#,
    );
    let mut worker = script.spawn();
    let pid = worker.pid();
    worker.hello().unwrap();
    assert_eq!(
        std::fs::read_to_string(script.job.join("isolated")).unwrap(),
        "verified"
    );
    let w::WorkerIn::Start(start) = c1::decode(&bytes("valid/start.json")).unwrap() else {
        unreachable!()
    };
    let prepared = ordinary(|| PreparedStart::new(&start).unwrap());
    worker.send_prepared_start(&prepared).unwrap();
    let observed = worker.start_write_observation();
    assert!(observed.attempted);
    assert_eq!(observed.bytes, prepared.as_bytes().len());
    assert_eq!(observed.errno, 0);
    let w::WorkerOut::Progress(progress) = next(&mut worker).unwrap() else {
        panic!("missing progress")
    };
    assert_eq!(progress.seq.0, 1);
    assert_eq!(progress.run_id, start.run_id);
    assert_eq!(progress.nonce, start.nonce);
    let w::WorkerOut::Done(done) = next(&mut worker).unwrap() else {
        panic!("missing Done")
    };
    assert_eq!(done.seq.0, progress.seq.0 + 1);
    assert_eq!(done.run_id, start.run_id);
    assert_eq!(done.nonce, start.nonce);
    assert!(reaped(&mut worker).success());
    assert_reaped(pid);
}

#[test]
fn actual_private_prepared_start_is_one_write_and_duplicate_closes_without_replay() {
    // Protocol-fault process fixture only. The producer corpus supplies the
    // actual Start/Hello shapes; this echo process is not runtime conformance.
    let script = Script::new(
        "print(HELLO, flush=True)\nraw=sys.stdin.buffer.read()\npathlib.Path(sys.argv[4]).joinpath('captured-input').write_bytes(raw)\n",
    );
    let mut worker = script.spawn();
    let pid = worker.pid();
    worker.hello().unwrap();
    let w::WorkerIn::Start(start) = c1::decode(&bytes("valid/start.json")).unwrap() else {
        unreachable!()
    };
    let prepared = ordinary(|| PreparedStart::new(&start).unwrap());
    worker.send_prepared_start(&prepared).unwrap();
    let first = worker.start_write_observation();
    assert!(first.attempted);
    assert_eq!(first.bytes, prepared.as_bytes().len());
    assert_eq!(first.errno, 0);
    assert_eq!(
        worker.send_prepared_start(&prepared),
        Err(w::Code::ProtocolError)
    );
    assert_eq!(worker.start_write_observation(), first);
    assert_eq!(worker.flush(), Err(w::Code::WorkerLost));
    assert!(
        worker
            .send(&w::WorkerIn::Activity(w::Activity {
                protocol: w::Protocol::V1,
                run_id: start.run_id.clone(),
                nonce: start.nonce.clone(),
                revision: w::Number(start.activity_revision.0 + 1),
                foreground: true,
            }))
            .is_err()
    );
    assert!(
        worker
            .send(&w::WorkerIn::Cancel(w::WorkerCancel {
                protocol: w::Protocol::V1,
                run_id: start.run_id,
                nonce: start.nonce,
            }))
            .is_err()
    );
    worker.bootstrap_revoke();
    worker.close_input();
    assert!(reaped(&mut worker).success());
    assert_reaped(pid);
    assert_eq!(
        std::fs::read(script.job.join("captured-input")).unwrap(),
        prepared.as_bytes(),
        "the private process received the original Start exactly once"
    );
}

#[test]
fn actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames() {
    for (body, expected) in [
        (
            "sys.stdout.buffer.write(b'\\n'); sys.stdout.flush(); time.sleep(5)",
            w::Code::ProtocolError,
        ),
        (
            "sys.stdout.buffer.write(b'\\xff\\n'); sys.stdout.flush(); time.sleep(5)",
            w::Code::ProtocolError,
        ),
        (
            "print(HELLO[:-1] + ',\"extra\":true}', flush=True); time.sleep(5)",
            w::Code::ProtocolError,
        ),
        (
            "print(HELLO[:-1] + ',\"ir_major\":1}', flush=True); time.sleep(5)",
            w::Code::ProtocolError,
        ),
        (
            "sys.stdout.buffer.write(b'x'*(256*1024+1)); sys.stdout.flush(); time.sleep(5)",
            w::Code::ResourceLimit,
        ),
        (
            "v=json.loads(HELLO); v['schema_sha256']='f'*64; print(json.dumps(v), flush=True); time.sleep(5)",
            w::Code::VersionMismatch,
        ),
    ] {
        let script = Script::new(body);
        let mut worker = script.spawn();
        let pid = worker.pid();
        assert_eq!(
            worker.hello().unwrap_err(),
            expected,
            "framing control: {body}"
        );
        drop(worker);
        assert_reaped(pid);
    }
}

#[test]
fn eof_and_drop_retain_exact_child_custody_through_actual_reap() {
    let script = Script::new("print(HELLO, flush=True)\nfor line in sys.stdin: pass\n");
    let mut worker = script.spawn();
    let pid = worker.pid();
    worker.hello().unwrap();
    assert!(
        worker.try_wait().unwrap().is_none(),
        "child must be alive before EOF"
    );
    worker.close_input();
    assert!(reaped(&mut worker).success());
    assert_reaped(pid);

    let script = Script::new("print(HELLO, flush=True)\ntime.sleep(30)");
    let mut worker = script.spawn();
    let pid = worker.pid();
    worker.hello().unwrap();
    assert!(worker.try_wait().unwrap().is_none());
    drop(worker);
    assert_reaped(pid);
}

/// This module exercises the real Rich connection against kernel-identified
/// private child processes. Scripted IR is a causal transport fixture, not the
/// producer's canonical C1 corpus or proof of UI/GPU/source-closure behavior.
// Origin: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Reuse credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
mod rich_activity_process {
    use bboxy450_harness_wake_up::{
        Evidence,
        rich::{Connection, State},
    };
    use serde_json::{Value, json};
    use std::{
        fs::{self, File, OpenOptions},
        io::Write,
        os::unix::fs::OpenOptionsExt,
        path::{Path, PathBuf},
        process::{Child, ChildStdin, Command, Stdio},
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };
    use ulid::Ulid;

    const MAX_CALLBACKS: usize = 4096;
    const BOUND: Duration = Duration::from_millis(100);
    // Adapted envelope shapes from bboxy450-harness protocol/lagged.jsonl:
    // https://github.com/bboxy450/bboxy450-harness/blob/main/tests/fixtures/protocol/lagged.jsonl
    // Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
    // This process fixture never substitutes for the canonical C1 schema/corpus.
    const SERVER: &str = r#"
import json,os,pathlib,select,socket,sys
path,root,session,epoch,head=sys.argv[1:]
root=pathlib.Path(root);epoch=int(epoch);head=int(head);seq=0
listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
listener.bind(path);os.chmod(path,0o600);listener.listen(4)
control=sys.stdin.buffer.raw
(root/'server-ready').write_text(str(os.getpid()))
conn=None
while conn is None:
    candidate,_=listener.accept();reader=candidate.makefile('rb')
    line=reader.readline()
    if not line:
        reader.close();candidate.close();continue
    subscribe=json.loads(line)
    assert subscribe['subscribe']=={'after':None,'ephemeral':False}
    conn=candidate

def wire(value):
    conn.sendall(json.dumps(value,sort_keys=True,separators=(',',':')).encode()+b'\n')
wire({'v':{'major':1,'minor':3},'meta':{'session':session},'epoch':epoch,'head':head})
held=False
while True:
    ready,_,_=select.select([control,conn],[],[],5)
    if control in ready:
        line=control.readline()
        if not line:break
        cmd=json.loads(line)
        if cmd['type'] in ('quit','eof'):break
        assert cmd['type']=='event'
        if cmd.get('advance',True):seq+=1
        number=cmd.get('seq',seq)
        event={'type':cmd['kind']}
        if cmd['kind']=='session_started':
            event['meta']={'session':session,'harness':'fixture','profile':'private-rich',
                'cwd':str(root),'created_ms':0,'protocol':{'major':1,'minor':3},
                'fork':cmd.get('fork'),'source':cmd.get('source')}
        if cmd.get('turn') is not None:event['turn']=cmd['turn']
        if cmd['kind']=='turn_ended':
            event['reason']={'reason':'end_turn'}
            event['usage']={'input_tokens':0,'output_tokens':0,'cached_input_tokens':None,
                'reasoning_tokens':None,'cost_micros_usd':None}
        envelope={'v':{'major':1,'minor':3},'session':cmd.get('session',session),
            'epoch':cmd.get('epoch',epoch),'seq':number,'at':number,'ts_ms':number,'cause':None,'event':event}
        raw=json.dumps(envelope,sort_keys=True,separators=(',',':')).encode()+b'\n'
        with (root/'durable-ir.jsonl').open('ab',buffering=0) as log:
            log.write(raw);os.fsync(log.fileno())
        conn.sendall(raw)
    if conn in ready:
        line=reader.readline()
        if not line:break
        request=json.loads(line)
        assert request['session']==session
        assert request['op']['type']=='user_turn'
        if not held:
            held=True
            (root/'request-entered-legacy-read').write_text('held')
            gate=os.open(str(root/'request-gate'),os.O_RDONLY)
            assert os.read(gate,1)==b'x'
            os.close(gate)
            (root/'request-released-legacy-read').write_text('released')
        wire({'op_id':request['op_id'],'ack':{'op_id':request['op_id'],'duplicate':False}})
reader.close();conn.close();listener.close()
"#;
    const HELD_READ: &str = r#"
import os,pathlib,sys
root=pathlib.Path(sys.argv[1]);fd=os.open(str(root/'held-read'),os.O_RDONLY)
(root/'held-ready').write_text(str(os.getpid()))
assert os.read(fd,1)==b'x'
(root/'held-released').write_text('released')
os.close(fd)
"#;

    fn fifo(path: &Path) -> File {
        let path_c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: this creates only a new FIFO under the fixture's owned directory.
        assert_eq!(unsafe { libc::mkfifo(path_c.as_ptr(), 0o600) }, 0);
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)
            .unwrap()
    }
    fn reap(child: &mut Child, what: &str) {
        let pid = child.id();
        let deadline = Instant::now() + Duration::from_secs(2);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                assert!(
                    std::thread::panicking(),
                    "exact owned {what} child {pid} did not settle"
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let status = child.wait().unwrap();
        if !std::thread::panicking() {
            assert!(status.success(), "private {what} child failed: {status}");
        }
    }
    async fn file_ready(path: &Path, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if fs::read_to_string(path).ok().as_deref() == Some(expected) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "private child marker missing: {}",
                path.display()
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    struct Server {
        root: tempfile::TempDir,
        path: PathBuf,
        session: Ulid,
        child: Option<Child>,
        stdin: Option<ChildStdin>,
        gate: File,
    }
    impl Server {
        async fn new(head: u64) -> Self {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("rich.sock");
            let gate = fifo(&root.path().join("request-gate"));
            let session = Ulid::new();
            let script = root.path().join("rich-server.py");
            fs::write(&script, SERVER).unwrap();
            let stdout = File::create(root.path().join("server.stdout")).unwrap();
            let stderr = File::create(root.path().join("server.stderr")).unwrap();
            let mut child = Command::new("/usr/bin/python3")
                .env_clear()
                .args(["-I", "-u"])
                .arg(script)
                .arg(&path)
                .arg(root.path())
                .arg(session.to_string())
                .arg("1")
                .arg(head.to_string())
                .current_dir(root.path())
                .stdin(Stdio::piped())
                .stdout(stdout)
                .stderr(stderr)
                .spawn()
                .unwrap();
            let stdin = child.stdin.take().unwrap();
            file_ready(&root.path().join("server-ready"), &child.id().to_string()).await;
            Self {
                root,
                path,
                session,
                child: Some(child),
                stdin: Some(stdin),
                gate,
            }
        }
        fn pid(&self) -> u32 {
            self.child.as_ref().unwrap().id()
        }
        fn command(&mut self, value: Value) {
            let mut raw = serde_json::to_vec(&value).unwrap();
            raw.push(b'\n');
            self.stdin.as_mut().unwrap().write_all(&raw).unwrap();
        }
        fn emit(&mut self, kind: &str, turn: Option<Ulid>) {
            self.command(json!({"type":"event","kind":kind,"turn":turn}));
        }
        fn finish(&mut self) {
            // The socket owner may already have closed its end. Shutdown is
            // best effort; the exact child exit/reap below remains mandatory.
            if let Some(stdin) = &mut self.stdin {
                let _ = stdin.write_all(b"{\"type\":\"quit\"}\n");
            }
            self.stdin.take();
            if let Some(mut child) = self.child.take() {
                reap(&mut child, "Rich server");
                super::assert_reaped(child.id());
            }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.gate.write_all(b"x");
            if let Some(stdin) = &mut self.stdin {
                let _ = stdin.write_all(b"{\"type\":\"quit\"}\n");
            }
            self.stdin.take();
            if let Some(mut child) = self.child.take() {
                // An identity-refused server may still be waiting in accept.
                // Terminate only this fixture's exact recorded child PID.
                if child.try_wait().unwrap().is_none() {
                    let _ = child.kill();
                }
                let _ = child.wait();
            }
            if std::thread::panicking() {
                let root = std::mem::replace(&mut self.root, tempfile::tempdir().unwrap());
                eprintln!(
                    "private Rich activity server retained: {}",
                    root.keep().display()
                );
            }
        }
    }
    struct HeldRead {
        root: tempfile::TempDir,
        writer: File,
        child: Option<Child>,
    }
    impl HeldRead {
        async fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let writer = fifo(&root.path().join("held-read"));
            let log = File::create(root.path().join("held.log")).unwrap();
            let child = Command::new("/usr/bin/python3")
                .env_clear()
                .args(["-I", "-u", "-c", HELD_READ])
                .arg(root.path())
                .stdin(Stdio::null())
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap();
            file_ready(&root.path().join("held-ready"), &child.id().to_string()).await;
            Self {
                root,
                writer,
                child: Some(child),
            }
        }
        fn assert_held(&mut self) {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "legacy-read surrogate unexpectedly exited"
            );
            assert!(
                !self.root.path().join("held-released").exists(),
                "held read advanced without release"
            );
        }
        fn release(&mut self) {
            self.writer.write_all(b"x").unwrap();
            if let Some(mut child) = self.child.take() {
                reap(&mut child, "held legacy read");
                super::assert_reaped(child.id());
            }
            assert_eq!(
                fs::read_to_string(self.root.path().join("held-released")).unwrap(),
                "released"
            );
        }
    }
    impl Drop for HeldRead {
        fn drop(&mut self) {
            let _ = self.writer.write_all(b"x");
            if let Some(mut child) = self.child.take() {
                reap(&mut child, "held legacy read");
            }
            if std::thread::panicking() {
                let root = std::mem::replace(&mut self.root, tempfile::tempdir().unwrap());
                eprintln!(
                    "private held-read activity witness retained: {}",
                    root.keep().display()
                );
            }
        }
    }

    /// The activity callback itself performs bounded atomic bookkeeping only.
    /// Slots are preallocated, so timestamps/flags never lock, allocate or do IO.
    struct Samples {
        origin: Instant,
        next: AtomicUsize,
        overflow: AtomicBool,
        values: [AtomicU8; MAX_CALLBACKS],
        at: [AtomicU64; MAX_CALLBACKS],
        revisions: [AtomicU64; MAX_CALLBACKS],
        packed: AtomicU64,
    }
    impl Samples {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                origin: Instant::now(),
                next: AtomicUsize::new(0),
                overflow: AtomicBool::new(false),
                values: std::array::from_fn(|_| AtomicU8::new(3)),
                at: std::array::from_fn(|_| AtomicU64::new(0)),
                revisions: std::array::from_fn(|_| AtomicU64::new(0)),
                packed: AtomicU64::new(1),
            })
        }
        fn now(&self) -> u64 {
            self.origin
                .elapsed()
                .as_nanos()
                .try_into()
                .unwrap_or(u64::MAX)
        }
        fn callback(self: &Arc<Self>) -> Arc<dyn Fn(Option<bool>) + Send + Sync> {
            let samples = self.clone();
            Arc::new(move |active| {
                let code = match active {
                    Some(false) => 0,
                    Some(true) => 1,
                    None => 2,
                };
                let state = u64::from(active.unwrap_or(true));
                let previous = samples
                    .packed
                    .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                        old.checked_add(2).map(|n| (n & !1) | state)
                    })
                    .unwrap();
                let index = samples.next.fetch_add(1, Ordering::AcqRel);
                if index >= MAX_CALLBACKS {
                    samples.overflow.store(true, Ordering::Release);
                    return;
                }
                samples.values[index].store(code, Ordering::Relaxed);
                samples.revisions[index].store((previous >> 1) + 1, Ordering::Relaxed);
                samples.at[index].store(samples.now().max(1), Ordering::Release);
            })
        }
        async fn wait(&self, from: usize, value: u8) -> (usize, u64) {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                assert!(
                    !self.overflow.load(Ordering::Acquire),
                    "bounded callback sample lane overflowed"
                );
                let through = self.next.load(Ordering::Acquire).min(MAX_CALLBACKS);
                for index in from..through {
                    let at = self.at[index].load(Ordering::Acquire);
                    if at != 0 && self.values[index].load(Ordering::Relaxed) == value {
                        return (index, at);
                    }
                }
                assert!(
                    Instant::now() < deadline,
                    "activity callback {value} missing from {from}"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
        fn monotonic_revisions(&self) {
            let mut previous = 0;
            for index in 0..self.next.load(Ordering::Acquire).min(MAX_CALLBACKS) {
                // A later disconnect callback may be publishing concurrently;
                // compare the complete prefix without waiting in that callback.
                if self.at[index].load(Ordering::Acquire) == 0 {
                    break;
                }
                let revision = self.revisions[index].load(Ordering::Relaxed);
                assert!(
                    revision > previous,
                    "activity incarnation/revision moved backwards"
                );
                previous = revision;
            }
        }
        fn no_quiescent_since(&self, from: usize) {
            for index in from..self.next.load(Ordering::Acquire).min(MAX_CALLBACKS) {
                if self.at[index].load(Ordering::Acquire) != 0 {
                    assert_ne!(
                        self.values[index].load(Ordering::Relaxed),
                        0,
                        "stale/untrusted event cleared foreground"
                    );
                }
            }
        }
    }
    fn observe(server: &Server, samples: &Arc<Samples>, pid: u32) -> Connection {
        Connection::start_observing(
            server.path.clone(),
            &tokio::runtime::Handle::current(),
            Arc::new(|| {}),
            Some(pid),
            Some(samples.callback()),
        )
    }
    async fn connected(connection: &Connection, session: Ulid) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if connection
                .client
                .hello()
                .is_some_and(|hello| hello.session == session && hello.epoch == 1)
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "PID-bound Rich Hello was not admitted: {:?}",
                connection.client.state()
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    async fn event(
        server: &mut Server,
        samples: &Samples,
        kind: &str,
        turn: Option<Ulid>,
        expected: u8,
    ) -> u64 {
        let from = samples.next.load(Ordering::Acquire);
        let start = samples.now();
        server.emit(kind, turn);
        let (_, at) = samples.wait(from, expected).await;
        let elapsed = at.saturating_sub(start);
        assert!(
            Duration::from_nanos(elapsed) <= BOUND,
            "{kind} callback exceeded 100 ms: {elapsed} ns"
        );
        elapsed
    }
    fn report(label: &str, values: &mut [u64]) {
        values.sort_unstable();
        let percentile = (values.len() * 99).div_ceil(100) - 1;
        println!(
            "Rich activity {label}: samples={} p99_ns={} max_ns={} limit_ns={}",
            values.len(),
            values[percentile],
            values[values.len() - 1],
            BOUND.as_nanos()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_activity_real_process_user_turn_and_200_durable_turn_samples_bypass_held_legacy_read()
     {
        let mut server = Server::new(0).await;
        let samples = Samples::new();
        let connection = observe(&server, &samples, server.pid());
        connected(&connection, server.session).await;
        event(&mut server, &samples, "session_started", None, 0).await;
        let mut held = HeldRead::new().await;
        held.assert_held();
        let from = samples.next.load(Ordering::Acquire);
        let start = samples.now();
        let client = connection.client.clone();
        let session = server.session;
        let sending = tokio::spawn(async move {
            client.send(Ulid::new(),session,json!({"type":"user_turn","mode":"queue","blocks":[{"type":"text","text":"private activity request"}]})).await
        });
        let (_, at) = samples.wait(from, 1).await;
        assert!(
            Duration::from_nanos(at.saturating_sub(start)) <= BOUND,
            "admitted user_turn callback waited on legacy source IO"
        );
        file_ready(
            &server.root.path().join("request-entered-legacy-read"),
            "held",
        )
        .await;
        assert!(
            !sending.is_finished(),
            "legacy-read surrogate did not hold native Ack"
        );
        assert!(
            !server
                .root
                .path()
                .join("request-released-legacy-read")
                .exists()
        );
        server.gate.write_all(b"x").unwrap();
        let outcome = tokio::time::timeout(Duration::from_secs(2), sending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(outcome.evidence, Evidence::NativeQueued);
        let turn = Ulid::new();
        event(&mut server, &samples, "turn_queued", Some(turn), 1).await;
        event(&mut server, &samples, "turn_started", Some(turn), 1).await;
        event(&mut server, &samples, "turn_ended", Some(turn), 0).await;
        let mut queued = Vec::with_capacity(200);
        let mut started = Vec::with_capacity(200);
        let mut ended = Vec::with_capacity(200);
        for _ in 0..200 {
            let turn = Ulid::new();
            queued.push(event(&mut server, &samples, "turn_queued", Some(turn), 1).await);
            started.push(event(&mut server, &samples, "turn_started", Some(turn), 1).await);
            ended.push(event(&mut server, &samples, "turn_ended", Some(turn), 0).await);
        }
        held.assert_held();
        samples.monotonic_revisions();
        report("turn_queued", &mut queued);
        report("turn_started", &mut started);
        report("turn_ended", &mut ended);
        println!(
            "Rich activity user_turn: callback_ns={} Ack was independently held; no UI/GPU/heap claim",
            at.saturating_sub(start)
        );
        held.release();
        drop(connection);
        server.finish();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_activity_wrong_session_epoch_and_stale_follower_cannot_clear_new_foreground() {
        let mut server = Server::new(0).await;
        let samples = Samples::new();
        let connection = observe(&server, &samples, server.pid());
        connected(&connection, server.session).await;
        event(&mut server, &samples, "session_started", None, 0).await;
        let captured = samples.packed.load(Ordering::Acquire);
        assert_eq!(captured & 1, 0);
        let (release, wait) = std::sync::mpsc::channel();
        let (result, received) = std::sync::mpsc::channel();
        let follower_samples = samples.clone();
        let follower = std::thread::spawn(move || {
            wait.recv().unwrap();
            let cleared = (captured & !1).checked_add(2).unwrap();
            let value = follower_samples.packed.compare_exchange(
                captured,
                cleared,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            result.send(value.is_err()).unwrap();
        });
        let turn = Ulid::new();
        event(&mut server, &samples, "turn_queued", Some(turn), 1).await;
        release.send(()).unwrap();
        assert!(
            received.recv_timeout(BOUND).unwrap(),
            "stale follower CAS cleared a newer actual socket turn"
        );
        follower.join().unwrap();
        assert_eq!(samples.packed.load(Ordering::Acquire) & 1, 1);
        let since = samples.next.load(Ordering::Acquire);
        server.command(json!({"type":"event","kind":"turn_ended","turn":turn,"session":Ulid::new(),"seq":3,"advance":false}));
        event(&mut server, &samples, "turn_started", Some(turn), 1).await;
        server.command(json!({"type":"event","kind":"turn_ended","turn":turn,"epoch":2,"seq":4,"advance":false}));
        let other = Ulid::new();
        event(&mut server, &samples, "turn_queued", Some(other), 1).await;
        event(&mut server, &samples, "turn_started", Some(other), 1).await;
        event(&mut server, &samples, "turn_ended", Some(turn), 1).await;
        samples.no_quiescent_since(since);
        event(&mut server, &samples, "turn_ended", Some(other), 0).await;
        samples.monotonic_revisions();
        drop(connection);
        server.finish();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_activity_kernel_wrong_pid_never_admits_quiescence() {
        let mut server = Server::new(0).await;
        let samples = Samples::new();
        assert_ne!(server.pid(), std::process::id());
        let connection = observe(&server, &samples, std::process::id());
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            assert!(
                connection.client.hello().is_none(),
                "wrong kernel peer PID was admitted"
            );
            samples.no_quiescent_since(0);
            if matches!(connection.client.state(), State::Closed(_)) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "wrong-PID connection did not refuse promptly"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(samples.packed.load(Ordering::Acquire) & 1, 1);
        drop(connection);
        // The child is blocked waiting for a subscriber the refused socket never sent.
        let mut child = server.child.take().unwrap();
        child.kill().unwrap();
        let _ = child.wait().unwrap();
        super::assert_reaped(child.id());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_activity_known_empty_close_preserves_quiescence_but_ordinary_eof_is_unknown() {
        for closed in [false, true] {
            let mut server = Server::new(0).await;
            let samples = Samples::new();
            let connection = observe(&server, &samples, server.pid());
            connected(&connection, server.session).await;
            event(&mut server, &samples, "session_started", None, 0).await;
            let turn = Ulid::new();
            event(&mut server, &samples, "turn_queued", Some(turn), 1).await;
            event(&mut server, &samples, "turn_started", Some(turn), 1).await;
            event(&mut server, &samples, "turn_ended", Some(turn), 0).await;
            if closed {
                event(&mut server, &samples, "session_closed", None, 0).await;
            } else {
                let from = samples.next.load(Ordering::Acquire);
                server.command(json!({"type":"eof"}));
                samples.wait(from, 2).await;
            }
            let deadline = Instant::now() + BOUND;
            loop {
                if matches!(connection.client.state(), State::Closed(_)) {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "Rich EOF did not settle observation"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            assert_eq!(
                samples.packed.load(Ordering::Acquire) & 1,
                u64::from(!closed)
            );
            // This callback is an activity hint only: no source catalogue, close
            // cursor, Dream run or automatic inspection is constructed here.
            drop(connection);
            server.finish();
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rich_activity_hello_head_gaps_and_turn_bound_keep_unknown_foreground() {
        let mut server = Server::new(3).await;
        let samples = Samples::new();
        let connection = observe(&server, &samples, server.pid());
        connected(&connection, server.session).await;
        server.emit("session_started", None);
        let turn = Ulid::new();
        event(&mut server, &samples, "turn_queued", Some(turn), 1).await;
        samples.no_quiescent_since(0); // Hello head=3; prefix seq=1 cannot certify idle.
        event(&mut server, &samples, "turn_started", Some(turn), 1).await;
        event(&mut server, &samples, "turn_ended", Some(turn), 0).await;
        let turn = Ulid::new();
        event(&mut server, &samples, "turn_queued", Some(turn), 1).await;
        let from = samples.next.load(Ordering::Acquire);
        server.command(json!({"type":"event","kind":"turn_ended","turn":turn,"seq":7})); // skips durable seq 6
        let start = samples.now();
        server.emit("session_closed", None);
        let (_, at) = samples.wait(from, 2).await;
        assert!(Duration::from_nanos(at.saturating_sub(start)) <= BOUND);
        samples.no_quiescent_since(from);
        assert_eq!(samples.packed.load(Ordering::Acquire) & 1, 1);
        drop(connection);
        server.finish();

        let mut server = Server::new(0).await;
        let samples = Samples::new();
        let connection = observe(&server, &samples, server.pid());
        connected(&connection, server.session).await;
        event(&mut server, &samples, "session_started", None, 0).await;
        let from = samples.next.load(Ordering::Acquire);
        for index in 0..129 {
            event(
                &mut server,
                &samples,
                "turn_queued",
                Some(Ulid::new()),
                if index < 128 { 1 } else { 2 },
            )
            .await;
        }
        samples.no_quiescent_since(from);
        assert_eq!(samples.packed.load(Ordering::Acquire) & 1, 1);
        event(&mut server, &samples, "session_closed", None, 2).await;
        samples.monotonic_revisions();
        drop(connection);
        server.finish();
    }
}
