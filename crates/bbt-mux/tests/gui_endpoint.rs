#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! Invariant #261 (decision #260): no `gui.endpoint` exists unless `ctl` is open, and the
//! app's own seats are refused at the door.

use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    ctl_door::{self, Peers},
    identity::{Custody, DeviceKeyStore, PendingReason},
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
    store::sqlite::SqliteStore,
};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, SwarmId,
    messaging::{Request, Response},
};
use ed25519_dalek::SigningKey;
use std::{
    collections::BTreeMap,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};

#[derive(Default)]
struct Keys(Mutex<BTreeMap<Ulid, SigningKey>>);
impl DeviceKeyStore for Keys {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        Ok(self.0.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        let mut keys = self.0.lock().unwrap();
        if let std::collections::btree_map::Entry::Vacant(entry) = keys.entry(id) {
            entry.insert(key.clone());
            Ok(())
        } else {
            Err(PendingReason::JournalConflict)
        }
    }
}

fn request(capability: [u8; 32], action: GuiAction) -> String {
    serde_json::to_string(&GuiRequest {
        version: bbt_proto::gui::VERSION,
        capability,
        action,
    })
    .unwrap()
}

// Admission can write its refusal and close before this peer writes. The write
// error must not discard that buffered explanation, or hide a missing reply.
const KNOCK_HELPER: &str = r#"import os, socket, sys
def knock(s, line):
    write_error = None
    try:
        s.sendall(line.encode() + b"\n")
    except OSError as error:
        write_error = error
    reply = s.makefile("rb").readline()
    if not reply:
        if write_error is not None:
            raise write_error
        raise EOFError("door closed without a reply")
    return reply, write_error
"#;

#[tokio::test]
async fn a_refusal_queued_before_the_request_survives_a_broken_pipe() {
    for queued in [true, false] {
        tokio::time::timeout(Duration::from_secs(10), async {
            let root = tempfile::Builder::new()
                .prefix("gui-refusal-")
                .tempdir_in("/tmp")
                .unwrap();
            let path = root.path().join("door.sock");
            let listener = tokio::net::UnixListener::bind(&path).unwrap();
            let script = format!(
                r#"{KNOCK_HELPER}
s = socket.socket(socket.AF_UNIX)
s.settimeout(10)
s.connect(os.environ["SOCK"])
sys.stdout.write("CONNECTED\n")
sys.stdout.flush()
sys.stdin.readline()
reply, write_error = knock(s, "request")
assert isinstance(write_error, BrokenPipeError), repr(write_error)
sys.stdout.buffer.write(reply)
"#
            );
            let mut child = tokio::process::Command::new("python3")
                .args(["-I", "-c", &script])
                .env("SOCK", &path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(child.stdout.take().unwrap());
            let ready = bbt_mux::seat_socket::read_line(&mut reader)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ready, b"CONNECTED");
            if queued {
                let mut refusal = serde_json::to_vec(&GuiReply::Error {
                    op: Ulid::nil(),
                    reason: ctl_door::REFUSAL.into(),
                })
                .unwrap();
                refusal.push(b'\n');
                stream.write_all(&refusal).await.unwrap();
            }
            stream.shutdown().await.unwrap();
            drop(stream);
            // The pipe rendezvous forces reply + close before the client's write.
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"go\n")
                .await
                .unwrap();
            let reply = bbt_mux::seat_socket::read_line(&mut reader).await.unwrap();
            let output = child.wait_with_output().await.unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            if queued {
                assert!(output.status.success(), "{}: {stderr}", output.status);
                let reply: GuiReply = serde_json::from_slice(&reply.unwrap()).unwrap();
                assert!(matches!(reply, GuiReply::Error {op, reason}
                    if op.is_nil() && reason == ctl_door::REFUSAL));
            } else {
                assert!(!output.status.success(), "a missing reply must fail");
                assert!(stderr.contains("BrokenPipeError"), "{stderr}");
                assert!(reply.is_none(), "no refusal was queued");
            }
        })
        .await
        .expect("forced-order refusal handshake must complete");
    }
}

/// Sends one request as an outside peer (this test process) and returns the reply to `op`.
async fn ask(path: &std::path::Path, line: &str, op: Ulid) -> GuiReply {
    let (read, mut write) = UnixStream::connect(path).await.unwrap().into_split();
    write
        .write_all(format!("{line}\n").as_bytes())
        .await
        .unwrap();
    let mut reader = BufReader::new(read);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let bytes = bbt_mux::seat_socket::read_line(&mut reader)
                .await
                .unwrap()
                .expect("an outside peer is admitted");
            let reply: GuiReply = serde_json::from_slice(&bytes).unwrap();
            if matches!(&reply, GuiReply::Messaging { op: o, .. } | GuiReply::Error { op: o, .. } if *o == op)
            {
                return reply;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctl_endpoint_opt_in_and_ancestry_refusal() {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("run");
    let repository = root.path().join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
    let supervisor = Supervisor::start(bbt_mux::Config {
        journal_directory: Some(root.path().join("supervisor-journal")),
        ..Default::default()
    })
    .unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository,
            ring: Default::default(),
        },
        Arc::new(Keys::default()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // A seat, so its swarm exists for the operator's messaging read.
    let own = SeatRecord::new(Ulid::new(), Ulid::new(), "Door fixture", "zsh");
    let mut launch = LaunchSpec::new("/bin/zsh", root.path());
    launch.seat = own.id;
    launch.args = vec![
        "-f".into(),
        "-c".into(),
        "while IFS= read -r line; do :; done".into(),
    ];
    logic.client.spawn(own.clone(), launch).await.unwrap();
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let file = run.join(ctl_door::ENDPOINT);

    // 1. Off by default: nothing on disk.
    assert!(!file.exists(), "no gui.endpoint without ctl");
    // A stale file a killed run left behind is replaced when the door opens (below).
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(&file, r#"{"path":"/stale","capability":"00"}"#).unwrap();

    // A stand-in supervisor: a shell that waits, then starts a child that knocks on the door.
    let script = format!(
        r#"{KNOCK_HELPER}
s = socket.socket(socket.AF_UNIX)
s.settimeout(10)
s.connect(os.environ["SOCK"])
reply, _ = knock(s, os.environ["LINE"])
sys.stdout.buffer.write(reply)"#
    );
    let op = Ulid::new();
    let list = GuiAction::Messaging {
        id: op,
        swarm_id: SwarmId::from_u128(own.swarm.0),
        request: Request::List {
            thread: None,
            channel: None,
            after: 0,
            limit: 8,
        },
    };
    let line = request(endpoint.capability, list);
    let mut stand_in = Command::new("/bin/sh")
        .args(["-c", "read go; python3 -I -c \"$SCRIPT\""])
        .env("SCRIPT", script)
        .env("SOCK", &endpoint.path)
        .env("LINE", &line)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // 2. Opening the door writes the file, 0600, naming the live endpoint.
    let door = ctl_door::open(
        &logic.client,
        &run,
        Peers {
            supervisor: stand_in.id(),
            gui: u32::MAX,
        },
    )
    .await
    .unwrap();
    assert_eq!(door.file(), file);
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "gui.endpoint is the operator's alone");
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(written["path"], serde_json::json!(endpoint.path));
    let hex: String = endpoint
        .capability
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(written["capability"], hex);
    assert_eq!(written["generation"], endpoint.generation);

    // 3. A supervisor-descended child is refused, by name, before any action runs.
    stand_in.stdin.take().unwrap().write_all(b"go\n").unwrap();
    let out = stand_in.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "stand-in knock exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let reply: GuiReply = serde_json::from_slice(&out.stdout).expect("the door explains itself");
    assert!(
        matches!(&reply, GuiReply::Error { op, reason } if op.is_nil() && reason == ctl_door::REFUSAL),
        "{reply:?}"
    );

    // 4. An outside peer (this process) is admitted through the ctl door and journaled as such.
    let reply = ask(&endpoint.path, &line, op).await;
    assert!(
        matches!(
            reply,
            GuiReply::Messaging {
                response: Response::Messages { .. },
                ..
            }
        ),
        "{reply:?}"
    );
    let account = SqliteStore::open(&bbt_mux::store::object_dir(&run, "account").unwrap()).unwrap();
    let entry: Vec<u8> = account
        .connection()
        .unwrap()
        .query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("control/op/{op}")],
            |row| row.get(0),
        )
        .expect("the ctl op is journaled under control/op");
    let entry: serde_json::Value = serde_json::from_slice(&entry).unwrap();
    assert_eq!(entry["actor"]["door"], "ctl", "{entry}");

    // The window's own pid keeps the window's door: no door field is written for it.
    drop(door);
    let door = ctl_door::open(
        &logic.client,
        &run,
        Peers {
            supervisor: u32::MAX,
            gui: std::process::id(),
        },
    )
    .await
    .unwrap();
    let window = Ulid::new();
    let line = request(
        endpoint.capability,
        GuiAction::Messaging {
            id: window,
            swarm_id: SwarmId::from_u128(own.swarm.0),
            request: Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 8,
            },
        },
    );
    assert!(matches!(
        ask(&endpoint.path, &line, window).await,
        GuiReply::Messaging { .. }
    ));
    let entry: Vec<u8> = account
        .connection()
        .unwrap()
        .query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("control/op/{window}")],
            |row| row.get(0),
        )
        .unwrap();
    let entry: serde_json::Value = serde_json::from_slice(&entry).unwrap();
    assert!(entry["actor"].get("door").is_none(), "{entry}");

    // Closing the door removes the file.
    drop(door);
    assert!(
        !file.exists(),
        "gui.endpoint is removed when the door closes"
    );
    drop(logic);
    drop(supervisor);
}
