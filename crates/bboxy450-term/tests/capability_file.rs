// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private process witnesses for seat-capability-delivery §§2–4. No model service.
#![allow(
    clippy::disallowed_methods,
    reason = "owned unattended process and file fixtures"
)]
use bbt_mux::{
    Supervisor, Ulid,
    adapters::{HarnessRecord, capability},
    logic::{Logic, LogicConfig},
    registry::{Position, SeatRecord},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
const APP: &str = env!("CARGO_BIN_EXE_bboxy450-term");
#[track_caller]
fn wait<T>(mut read: impl FnMut() -> Option<T>) -> T {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(value) = read() {
            return value;
        }
        assert!(
            Instant::now() < until,
            "private capability fixture watchdog"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn child(root: &Path) -> Command {
    let mut c = Command::new(APP);
    c.env_clear()
        .env("HOME", root)
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("BBOXY_RUN_DIR", root)
        .env("BBOXY_SEAT_ID", Ulid::new().to_string())
        .env("BBOXY_GENERATION", "1")
        .env("BBOXY_SOCK", root.join("missing.sock"));
    c
}
#[test]
fn capability_file_shape_exclusive_create_and_repeated_reads() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let file = capability::create(root.path(), &[0xab; 32]).unwrap();
    let meta = fs::symlink_metadata(&file).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.mode() & 0o7777, 0o600);
    assert_eq!(meta.uid(), fs::metadata(root.path()).unwrap().uid());
    assert_eq!(fs::metadata(root.path()).unwrap().mode() & 0o7777, 0o700);
    assert_eq!(fs::read(&file).unwrap().len(), 65);
    for _ in 0..3 {
        assert_eq!(capability::read(&file).unwrap(), [0xab; 32]);
        assert!(file.exists());
    }
    assert_eq!(
        capability::create(root.path(), &[0; 32])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    fs::remove_file(&file).unwrap();
    let target = root.path().join("target");
    fs::write(&target, b"keep").unwrap();
    std::os::unix::fs::symlink(&target, &file).unwrap();
    assert!(capability::create(root.path(), &[0; 32]).is_err());
    assert_eq!(fs::read(target).unwrap(), b"keep");
}
#[test]
fn capability_file_refuses_uppercase_hex() {
    let root = tempfile::tempdir().unwrap();
    let file = capability::create(root.path(), &[0xab; 32]).unwrap();
    fs::write(&file, fs::read(&file).unwrap().to_ascii_uppercase()).unwrap();
    assert_eq!(
        capability::read(&file).unwrap_err().to_string(),
        "seat_admission_file_malformed"
    );
}
#[test]
fn both_consumers_refuse_each_unsafe_file_with_a_typed_reason() {
    for reason in [
        "symlink",
        "not_regular",
        "wrong_owner",
        "wrong_mode",
        "malformed",
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let good = capability::create(root.path(), &[0xab; 32]).unwrap();
        let path = match reason {
            "symlink" => {
                let path = root.path().join("link");
                std::os::unix::fs::symlink(&good, &path).unwrap();
                path
            }
            "not_regular" => root.path().to_path_buf(),
            "wrong_owner" => {
                let host = Path::new("/etc/hosts");
                if fs::metadata(host).unwrap().uid() != fs::metadata(root.path()).unwrap().uid() {
                    host.to_path_buf()
                } else {
                    std::os::unix::fs::chown(&good, Some(1), None).unwrap();
                    good.clone()
                }
            }
            "wrong_mode" => {
                fs::set_permissions(&good, fs::Permissions::from_mode(0o644)).unwrap();
                good.clone()
            }
            _ => {
                fs::write(&good, b"malformed\n").unwrap();
                good.clone()
            }
        };
        let expected = format!("seat_admission_file_{reason}");
        assert_eq!(capability::read(&path).unwrap_err().to_string(), expected);
        let bridge = child(root.path())
            .arg("mcp")
            .env("BBOXY_CAPABILITY_FILE", &path)
            .output()
            .unwrap();
        assert!(!bridge.status.success(), "bridge admitted {reason}");
        assert!(
            String::from_utf8_lossy(&bridge.stderr).contains(&expected),
            "bridge lacks typed {reason} refusal"
        );
        let server =
            bbt_rt::error_journal::Server::start(root.path().to_path_buf(), |_| {}).unwrap();
        server
            .ready()
            .recv_timeout(Duration::from_secs(60))
            .unwrap()
            .unwrap();
        let mut hook = child(root.path())
            .args(["hook", "--agent", "claude"])
            .env("BBOXY_CAPABILITY_FILE", &path)
            .env("BBOXY_HOOK_SOCKET_ONLY", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        // Startup must validate the file before parsing input or taking OSC.
        let _ = writeln!(hook.stdin.take().unwrap(), "not-json");
        hook.wait().unwrap(); // Hooks fail open; the typed failure, not exit 0, is the witness.
        wait(|| {
            let rows = bbt_rt::error_journal::read(root.path(), None, None, false).unwrap();
            rows["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r.to_string().contains(&expected))
                .then_some(())
        });
    }
}

/// The helper is the owned supervisor process; its exec environment is clean.
#[test]
fn every_seat_kind_keeps_capability_out_of_seat_and_ancestor_environments() {
    let root = tempfile::Builder::new()
        .prefix("bbt-capability-file-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let python = Command::new("python3")
        .args(["-c", "import sys;print(sys.executable)"])
        .output()
        .unwrap();
    assert!(python.status.success());
    let mut helper = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("HOME", root.path())
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("CAPABILITY_FIXTURE_ROOT", root.path())
        .env("CAPABILITY_APP", APP)
        .env(
            "CAPABILITY_PYTHON",
            String::from_utf8(python.stdout).unwrap().trim(),
        )
        .args([
            "--ignored",
            "--exact",
            "capability_file::capability_process_fixture_child",
            "--nocapture",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(180);
    let status = loop {
        if let Some(s) = helper.try_wait().unwrap() {
            break s;
        }
        if Instant::now() >= until {
            helper.kill().unwrap();
            helper.wait().unwrap();
            panic!("owned capability process watchdog");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let output = helper.wait_with_output().unwrap();
    assert!(
        status.success(),
        "capability process failed: {}; progress={}",
        String::from_utf8_lossy(&output.stderr),
        fs::read_to_string(root.path().join("progress.json")).unwrap_or_default()
    );
    let proof: Value =
        serde_json::from_slice(&fs::read(root.path().join("proof.json")).unwrap()).unwrap();
    assert_eq!(proof.as_array().unwrap().len(), 12);
    println!("seat capability environment witness: {}", proof);
}
struct Relay {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    accepted: Arc<Mutex<Vec<bool>>>,
}
impl Relay {
    fn start(path: &Path, target: Arc<Mutex<PathBuf>>) -> Self {
        let listener = UnixListener::bind(path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(Mutex::new(vec![]));
        let (done, results) = (stop.clone(), accepted.clone());
        let worker = std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let success = (|| -> std::io::Result<bool> {
                            fn line(stream: &mut UnixStream) -> std::io::Result<Vec<u8>> {
                                stream.set_nonblocking(true)?;
                                let until = Instant::now() + Duration::from_secs(2);
                                let mut out = Vec::new();
                                loop {
                                    let mut chunk = [0; 1024];
                                    match stream.read(&mut chunk) {
                                        Ok(0) => {
                                            return Err(std::io::Error::from(
                                                std::io::ErrorKind::UnexpectedEof,
                                            ));
                                        }
                                        Ok(n) => {
                                            out.extend_from_slice(&chunk[..n]);
                                            if out.contains(&b'\n') {
                                                return Ok(out);
                                            }
                                            if out.len() > 65536 {
                                                return Err(std::io::Error::from(
                                                    std::io::ErrorKind::InvalidData,
                                                ));
                                            }
                                        }
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            if Instant::now() >= until {
                                                return Err(std::io::Error::from(
                                                    std::io::ErrorKind::TimedOut,
                                                ));
                                            }
                                            std::thread::sleep(Duration::from_millis(1));
                                        }
                                        Err(e) => return Err(e),
                                    }
                                }
                            }
                            let request = line(&mut stream)?;
                            let mut endpoint = UnixStream::connect(target.lock().unwrap().clone())?;
                            endpoint.write_all(&request)?;
                            let response = line(&mut endpoint)?;
                            let reply: Value = serde_json::from_slice(&response)?;
                            let _ = stream.write_all(&response); // fire-and-forget hooks close after writing
                            Ok(reply["kind"] == "HookObserved")
                        })();
                        results.lock().unwrap().push(success.unwrap_or(false));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("private hook relay: {e}"),
                }
            }
        });
        Self {
            stop,
            worker: Some(worker),
            accepted,
        }
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.worker.take().unwrap().join();
    }
}
#[test]
#[ignore = "private process child; parent starts it with a clean exec environment"]
fn capability_process_fixture_child() {
    let root = PathBuf::from(std::env::var_os("CAPABILITY_FIXTURE_ROOT").unwrap());
    let repository = root.join("repository/.bboxy");
    fs::create_dir_all(&repository).unwrap();
    let home = root.join("home");
    fs::create_dir(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join("supervisor.pid"), std::process::id().to_string()).unwrap();
    // Generic/non-native restart deliberately resumes zsh. Its private startup
    // file runs the same fixture command so every replacement has a child scan.
    fs::write(home.join(".zshrc"),r#"if [[ ${BBOXY_GENERATION:-1} -gt 1 ]]; then
  exec "$CAPABILITY_PYTHON" "$CAPABILITY_FIXTURE_ROOT/capability-probe.py" "$(< "$CAPABILITY_FIXTURE_ROOT/current-kind")"
fi
"#).unwrap();
    let script = root.join("capability-probe.py");
    fs::write(
        &script,
        include_bytes!("../../../scripts/e2e/capability_probe.py"),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(bbt_mux::Config {
        journal_directory: Some(root.join("journal")),
        ..Default::default()
    })
    .unwrap();
    let custody =
        bbt_mux::identity::TestAccount::open(&root.join("custody"), &root.join("run")).unwrap();
    let logic = Logic::start_custodied_with_fixture_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.join("run"),
            repository,
            ring: Default::default(),
        },
        Some(custody.key_store()),
        runtime.handle(),
    )
    .unwrap();
    let mut proofs = vec![];
    for kind in ["claude", "codex", "zsh", "bboxy450-harness"] {
        fs::write(root.join("current-kind"), kind).unwrap();
        let mut harness = HarnessRecord::builtin(if kind == "bboxy450-harness" {
            "zsh"
        } else {
            kind
        })
        .unwrap();
        harness.kind = kind.into();
        harness.command = vec![
            std::env::var("CAPABILITY_PYTHON").unwrap(),
            script.to_string_lossy().into_owned(),
            kind.into(),
        ];
        if kind == "codex" {
            harness.command.push("--no-daemon".into());
        }
        let record = SeatRecord::new(Ulid::new(), Ulid::new(), "capability fixture", kind);
        let id = record.id;
        let swarm = record.swarm;
        let target = Arc::new(Mutex::new(PathBuf::new()));
        let relay = Relay::start(&root.join(format!("{kind}-hook.sock")), target.clone());
        runtime
            .block_on(logic.client.spawn_harness(
                record,
                harness,
                bbt_mux::adapters::launch::LaunchOptions {
                    transport_argv: None,
                    via: None,
                    cwd: root.clone(),
                    role_prompt: String::new(),
                    executable: APP.into(),
                    user_home: home.clone(),
                    user_zdotdir: Some(home.clone()),
                    resume: false,
                    env: vec![
                        (
                            "CAPABILITY_FIXTURE_ROOT".into(),
                            root.to_string_lossy().into_owned(),
                        ),
                        ("CAPABILITY_APP".into(), APP.into()),
                        (
                            "CAPABILITY_SUPERVISOR_PID".into(),
                            std::process::id().to_string(),
                        ),
                    ],
                },
            ))
            .unwrap();
        let mut previous: Option<(PathBuf, [u8; 32])> = None;
        for generation in 1..=3 {
            fs::write(
                root.join("progress.json"),
                json!({"kind":kind,"generation":generation,"stage":"endpoint"}).to_string(),
            )
            .unwrap();
            let endpoint = wait(|| {
                runtime
                    .block_on(logic.client.endpoint(id))
                    .ok()
                    .filter(|e| e.generation == generation)
            });
            *target.lock().unwrap() = endpoint.path.clone();
            fs::write(root.join(format!("{kind}-{generation}.go")), b"ready").unwrap();
            fs::write(
                root.join("progress.json"),
                json!({"kind":kind,"generation":generation,"stage":"driver"}).to_string(),
            )
            .unwrap();
            let proof: Value = wait(|| {
                fs::read(root.join(format!("{kind}-{generation}.json")))
                    .ok()
                    .and_then(|b| serde_json::from_slice(&b).ok())
            });
            assert!(
                proof["orient"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v == true),
                "both fresh MCP starts must authenticate: {kind}; {}",
                proof["probe_status"]
            );
            for scan in proof["scans"].as_array().unwrap() {
                assert_eq!(
                    scan["capability_matches"], 0,
                    "SeatAndAncestorEnvironmentIsCapabilityFree: {kind} token count"
                );
                assert_eq!(
                    scan["legacy_keys"], 0,
                    "SeatAndAncestorEnvironmentIsCapabilityFree: {kind} legacy key"
                );
            }
            wait(|| (relay.accepted.lock().unwrap().len() >= generation as usize).then_some(()));
            assert!(
                relay
                    .accepted
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|accepted| *accepted),
                "hook was not authenticated"
            );
            let file = PathBuf::from(proof["capability_file"].as_str().unwrap());
            let current = capability::read(&file).unwrap();
            assert_eq!(fs::metadata(&file).unwrap().mode() & 0o7777, 0o600);
            assert_eq!(
                fs::metadata(file.parent().unwrap()).unwrap().mode() & 0o7777,
                0o700
            );
            if let Some((old_path, old_value)) = previous {
                assert_ne!(old_path, file);
                assert!(old_value != current, "generation reused the capability");
                assert!(!old_path.exists(), "old generation overlay survived");
                let stale = tempfile::tempdir_in(&root).unwrap();
                let stale_file = capability::create(stale.path(), &old_value).unwrap();
                let mut command = child(&home);
                command
                    .arg("mcp")
                    .env("BBOXY_CAPABILITY_FILE", stale_file)
                    .env("BBOXY_SOCK", &endpoint.path)
                    .env("BBOXY_SEAT_ID", id.to_string())
                    .env("BBOXY_GENERATION", generation.to_string())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());
                let mut mcp = command.spawn().unwrap();
                let mut input = mcp.stdin.take().unwrap();
                writeln!(input,"{}",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"old-file-fixture","version":"1"},"capabilities":{}}})).unwrap();
                writeln!(input,"{}",json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"orient","arguments":{}}})).unwrap();
                drop(input);
                let output = mcp.wait_with_output().unwrap();
                let reply: Value = String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .find(|reply| reply["id"] == 2)
                    .unwrap();
                assert!(
                    reply.get("error").is_some() || reply["result"]["isError"] == true,
                    "old file content authenticated a new generation"
                );
            }
            previous = Some((file, current));
            proofs.push(proof);
            if generation < 3 {
                runtime
                    .block_on(logic.client.ctl_restart(
                        id,
                        generation,
                        Ulid::new(),
                        bbt_mux::mcp::Origin::default(),
                    ))
                    .unwrap();
            }
        }
        fs::write(
            root.join("progress.json"),
            json!({"kind":kind,"stage":"close"}).to_string(),
        )
        .unwrap();
        let last = previous.unwrap().0;
        runtime
            .block_on(logic.client.close(
                id,
                3,
                Ulid::new(),
                Position {
                    swarm,
                    tab: Ulid::new(),
                    split_path: vec![0],
                    ratios: vec![],
                    layout_revision: 1,
                },
                0,
            ))
            .unwrap();
        wait(|| (!last.exists()).then_some(()));
        drop(relay);
    }
    logic.shutdown().unwrap();
    drop(supervisor);
    fs::write(
        root.join("proof.json"),
        serde_json::to_vec(&proofs).unwrap(),
    )
    .unwrap();
}
