use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bbt_mux::protocol::{MAX_INPUT_BYTES, VERSION};
use bbt_mux::transport::{read_frame, write_frame};
use bbt_mux::*;

const DEADLINE: Duration = Duration::from_secs(15);

fn shell() -> LaunchSpec {
    let mut spec = LaunchSpec::new("/bin/zsh", std::env::current_dir().unwrap());
    spec.args = vec!["-f".into()];
    spec.env.insert("PS1".into(), "".into());
    spec.env.insert("RPS1".into(), "".into());
    spec
}

fn output_until(client: &Client, seat: Ulid, needle: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + DEADLINE;
    let mut bytes = Vec::new();
    while Instant::now() < deadline {
        if let Event::Output {
            seat: from,
            bytes: chunk,
            ..
        } = client.recv_timeout(Duration::from_secs(2)).unwrap()
            && from == seat
        {
            bytes.extend(chunk);
            if bytes.windows(needle.len()).any(|window| window == needle) {
                return bytes;
            }
        }
    }
    panic!(
        "expected fixture output missing: {:?}",
        String::from_utf8_lossy(&bytes)
    );
}

fn status(client: &Client, attachment: &Attachment, input_id: Ulid) -> InputReceipt {
    match client
        .request(Action::InputStatus {
            seat: attachment.seat,
            generation: attachment.generation,
            input_id,
        })
        .unwrap()
    {
        Reply::Input { receipt } => receipt,
        reply => panic!("unexpected {reply:?}"),
    }
}

#[test]
fn zsh_echo_cwd_environment_and_resize() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = shell();
    spec.cwd = PathBuf::from("/tmp");
    spec.env = BTreeMap::from([
        ("BBT_FIXTURE".into(), "explicit-value".into()),
        ("BBOXY_SEAT_ID".into(), "cannot-spoof-seat".into()),
    ]);
    let info = client.spawn(spec.clone()).unwrap();
    assert_eq!(
        client.spawn(spec).unwrap().pid,
        info.pid,
        "exact spawn retry must not duplicate a child"
    );
    let attachment = client.attach(info.seat, 1).unwrap();
    let receipt = client
        .input(&attachment, Ulid::new(), b"echo hi\n")
        .unwrap();
    assert_eq!(receipt.state, InputState::PtyWritten);
    output_until(&client, info.seat, b"\r\nhi\r\n");
    client.input(&attachment, Ulid::new(), b"printf '\\nENV:%s:%s\\n' \"$BBOXY_SEAT_ID\" \"$BBT_FIXTURE\"; printf 'CWD:%s\\n' \"$(pwd -P)\"\n").unwrap();
    let expected = format!("ENV:{}:explicit-value\r\n", info.seat);
    let cwd = format!("CWD:{}\r\n", fs::canonicalize("/tmp").unwrap().display());
    let output = output_until(&client, info.seat, cwd.as_bytes());
    assert!(
        output
            .windows(expected.len())
            .any(|window| window == expected.as_bytes())
    );
    assert!(!String::from_utf8_lossy(&output).contains("cannot-spoof-seat"));
    let size = SeatSize {
        rows: 41,
        cols: 113,
        pixel_width: 900,
        pixel_height: 700,
    };
    let reply = client.resize(&attachment, size).unwrap();
    assert!(matches!(reply, Reply::Resized { size: actual, .. } if actual == size));
    client
        .input(&attachment, Ulid::new(), b"stty size\n")
        .unwrap();
    output_until(&client, info.seat, b"\r\n41 113\r\n");
}

#[test]
fn two_seats_interleave_without_cross_talk() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let a = client.spawn(shell()).unwrap();
    let b = client.spawn(shell()).unwrap();
    let aa = client.attach(a.seat, 1).unwrap();
    let bb = client.attach(b.seat, 1).unwrap();
    client
        .input(&aa, Ulid::new(), b"printf '\\nONLY_A\\n'\n")
        .unwrap();
    client
        .input(&bb, Ulid::new(), b"printf '\\nONLY_B\\n'\n")
        .unwrap();
    let mut outputs = HashMap::<Ulid, Vec<u8>>::new();
    let mut sequence = HashMap::new();
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        if let Event::Output {
            seat,
            generation,
            seq,
            bytes,
        } = client.recv_timeout(Duration::from_secs(2)).unwrap()
        {
            assert_eq!(generation, 1);
            assert!(seq > *sequence.entry(seat).or_insert(0));
            sequence.insert(seat, seq);
            outputs.entry(seat).or_default().extend(bytes);
            let a_done = outputs
                .get(&a.seat)
                .is_some_and(|v| v.windows(10).any(|w| w == b"\r\nONLY_A\r\n"));
            let b_done = outputs
                .get(&b.seat)
                .is_some_and(|v| v.windows(10).any(|w| w == b"\r\nONLY_B\r\n"));
            if a_done && b_done {
                break;
            }
        }
    }
    let a_output = String::from_utf8_lossy(&outputs[&a.seat]);
    let b_output = String::from_utf8_lossy(&outputs[&b.seat]);
    assert!(a_output.contains("\r\nONLY_A\r\n"), "{a_output:?}");
    assert!(b_output.contains("\r\nONLY_B\r\n"), "{b_output:?}");
    assert!(!a_output.contains("ONLY_B"));
    assert!(!b_output.contains("ONLY_A"));
}

#[test]
fn fifty_seats_share_one_supervisor_poller() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut expected = HashMap::new();
    let mut output = HashMap::<Ulid, Vec<u8>>::new();
    for index in 0..50 {
        let info = client.spawn(shell()).unwrap();
        let attachment = client.attach(info.seat, 1).unwrap();
        let marker = format!("BBT_SEAT_{index:02}_OK");
        client
            .input(
                &attachment,
                Ulid::new(),
                format!("echo {marker}\n").as_bytes(),
            )
            .unwrap();
        expected.insert(info.seat, format!("\r\n{marker}\r\n").into_bytes());
    }
    let deadline = Instant::now() + DEADLINE;
    while !expected.is_empty() && Instant::now() < deadline {
        if let Event::Output {
            seat,
            generation,
            bytes,
            ..
        } = client.recv_timeout(Duration::from_secs(2)).unwrap()
        {
            assert_eq!(generation, 1);
            let accumulated = output.entry(seat).or_default();
            accumulated.extend(bytes);
            if let Some(marker) = expected.get(&seat)
                && accumulated
                    .windows(marker.len())
                    .any(|window| window == marker)
            {
                expected.remove(&seat);
            }
        }
    }
    assert!(expected.is_empty(), "{} seats did not echo", expected.len());
    assert_eq!(output.len(), 50);
    eprintln!("50 distinct PTY seats echoed under one Supervisor / one mio::Poll");
}

#[test]
fn killed_child_reports_generation_and_fences_replacement() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = shell();
    let info = client.spawn(spec.clone()).unwrap();
    let attachment = client.attach(info.seat, 1).unwrap();
    client
        .input(&attachment, Ulid::new(), b"echo BEFORE_KILL\n")
        .unwrap();
    output_until(&client, info.seat, b"\r\nBEFORE_KILL\r\n");
    client.kill(&attachment).unwrap();
    let deadline = Instant::now() + DEADLINE;
    let mut exited = false;
    while Instant::now() < deadline {
        if let Event::Exited {
            seat,
            generation,
            seq,
            status,
        } = client.recv_timeout(Duration::from_secs(2)).unwrap()
        {
            assert_eq!(seat, info.seat);
            assert_eq!(generation, 1);
            assert!(seq > 1);
            assert!(status.signal.is_some() || status.code != 0);
            exited = true;
            break;
        }
    }
    assert!(exited);
    spec.generation = 2;
    let replacement = client.spawn(spec).unwrap();
    assert_eq!(replacement.generation, 2);
    assert_eq!(
        client
            .input(&attachment, Ulid::new(), b"echo stale\n")
            .unwrap_err()
            .code,
        FailureCode::StaleGeneration
    );
    let attached = client.attach(info.seat, 2).unwrap();
    client
        .input(&attached, Ulid::new(), b"echo NEW_GENERATION\n")
        .unwrap();
    output_until(&client, info.seat, b"\r\nNEW_GENERATION\r\n");
}

#[test]
fn logic_and_writer_replacement_preserve_child_and_deduplicate_input() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let first = supervisor.connect().unwrap();
    let info = first.spawn(shell()).unwrap();
    let old = first.attach(info.seat, 1).unwrap();
    let input_id = Ulid::new();
    let bytes =
        b"BBT_COUNT=${BBT_COUNT:-0}; (( BBT_COUNT++ )); printf '\\nCOUNT:%s\\n' \"$BBT_COUNT\"\n";
    first.input(&old, input_id, bytes).unwrap();
    output_until(&first, info.seat, b"\r\nCOUNT:1\r\n");
    let second = supervisor.connect().unwrap();
    assert_eq!(
        first.input(&old, Ulid::new(), b"wrong\n").unwrap_err().code,
        FailureCode::StaleLease
    );
    assert_eq!(
        second
            .input(&old, Ulid::new(), b"wrong\n")
            .unwrap_err()
            .code,
        FailureCode::StaleWriter
    );
    let new = second.attach(info.seat, 1).unwrap();
    assert_ne!(old.writer_lease, new.writer_lease);
    let retry = second.input(&new, input_id, bytes).unwrap();
    assert_eq!(retry.state, InputState::PtyWritten);
    assert_eq!(
        second
            .input(&new, input_id, b"different\n")
            .unwrap_err()
            .code,
        FailureCode::InputConflict
    );
    second
        .input(
            &new,
            Ulid::new(),
            b"printf '\\nFINAL:%s\\n' \"$BBT_COUNT\"\n",
        )
        .unwrap();
    output_until(&second, info.seat, b"\r\nFINAL:1\r\n");
    let Reply::Seats { seats } = second.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert_eq!(seats[0].pid, info.pid);
    drop(second);
    let third = supervisor.connect().unwrap();
    let attached = third.attach(info.seat, 1).unwrap();
    third
        .input(&attached, Ulid::new(), b"echo SURVIVED_DISCONNECT\n")
        .unwrap();
    output_until(&third, info.seat, b"\r\nSURVIVED_DISCONNECT\r\n");
}

#[test]
fn validation_rejects_stale_and_oversized_input() {
    let supervisor = Supervisor::start(Config {
        max_seats: 1,
        ..Config::default()
    })
    .unwrap();
    let client = supervisor.connect().unwrap();
    let info = client.spawn(shell()).unwrap();
    assert_eq!(client.spawn(shell()).unwrap_err().code, FailureCode::Limit);
    let old = client.attach(info.seat, 1).unwrap();
    let new = client.attach(info.seat, 1).unwrap();
    assert_eq!(
        client
            .input(&old, Ulid::new(), b"wrong\n")
            .unwrap_err()
            .code,
        FailureCode::StaleWriter
    );
    assert_eq!(
        client
            .input(&new, Ulid::new(), &vec![0; MAX_INPUT_BYTES + 1])
            .unwrap_err()
            .code,
        FailureCode::Invalid
    );
    let request = Request::new(Some(client.logic_lease()), Action::ListOwnedSeats);
    assert_eq!(
        client
            .request_frame(Frame {
                version: VERSION + 1,
                body: request
            })
            .unwrap_err()
            .code,
        FailureCode::Version
    );
    assert_eq!(
        client
            .resize(
                &new,
                SeatSize {
                    rows: 0,
                    ..SeatSize::default()
                }
            )
            .unwrap_err()
            .code,
        FailureCode::Invalid
    );
}

struct Runtime(PathBuf);
impl Runtime {
    fn new() -> Self {
        Self(PathBuf::from("/tmp").join(format!("bbt-w2-{}", Ulid::new())))
    }
    fn socket(&self) -> PathBuf {
        self.0.join("mux.sock")
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn socket_call(
    stream: &mut UnixStream,
    lease: Option<Ulid>,
    action: Action,
) -> Result<Reply, Failure> {
    let request = Request::new(lease, action);
    let id = request.request_id;
    write_frame(stream, &Frame::new(request)).unwrap();
    loop {
        let frame: Frame<Message> = read_frame(stream).unwrap();
        if let Message::Reply { request_id, result } = frame.body {
            assert_eq!(request_id, id);
            return result;
        }
    }
}

#[test]
fn unix_socket_auth_framing_and_live_pty() {
    let runtime = Runtime::new();
    let supervisor = Supervisor::start(Config {
        socket_path: Some(runtime.socket()),
        ..Config::default()
    })
    .unwrap();
    assert_eq!(
        fs::metadata(&runtime.0).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(runtime.socket()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        Supervisor::start(Config {
            socket_path: Some(runtime.socket()),
            ..Config::default()
        })
        .is_err()
    );
    let mut stream = UnixStream::connect(runtime.socket()).unwrap();
    stream.set_read_timeout(Some(DEADLINE)).unwrap();
    stream.set_write_timeout(Some(DEADLINE)).unwrap();
    assert_eq!(
        socket_call(&mut stream, None, Action::ListOwnedSeats)
            .unwrap_err()
            .code,
        FailureCode::StaleLease
    );
    assert_eq!(
        socket_call(
            &mut stream,
            None,
            Action::AcquireLogic {
                capability: [0; 32]
            }
        )
        .unwrap_err()
        .code,
        FailureCode::Unauthorized
    );
    let Reply::Acquired { logic_lease, .. } = socket_call(
        &mut stream,
        None,
        Action::AcquireLogic {
            capability: supervisor.socket_capability(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let Reply::Spawned { info } = socket_call(
        &mut stream,
        Some(logic_lease),
        Action::Spawn { spec: shell() },
    )
    .unwrap() else {
        panic!()
    };
    let Reply::Attached { attachment, .. } = socket_call(
        &mut stream,
        Some(logic_lease),
        Action::Attach {
            seat: info.seat,
            generation: 1,
        },
    )
    .unwrap() else {
        panic!()
    };
    socket_call(
        &mut stream,
        Some(logic_lease),
        Action::Input {
            attachment,
            input_id: Ulid::new(),
            bytes: b"echo SOCKET_HI\n".to_vec(),
        },
    )
    .unwrap();
    let mut output = Vec::new();
    loop {
        let frame: Frame<Message> = read_frame(&mut stream).unwrap();
        if let Message::Event {
            event:
                Event::Output {
                    seat,
                    generation,
                    bytes,
                    ..
                },
        } = frame.body
        {
            assert_eq!(seat, info.seat);
            assert_eq!(generation, 1);
            output.extend(bytes);
            if output.windows(13).any(|w| w == b"\r\nSOCKET_HI\r\n") {
                break;
            }
        }
    }
    supervisor.shutdown().unwrap();
    assert!(!runtime.socket().exists());
}

#[test]
fn blocked_write_survives_fencing_and_becomes_unknown_without_replay() {
    let supervisor = Supervisor::start(Config {
        write_timeout: Duration::from_secs(1),
        ..Config::default()
    })
    .unwrap();
    let first = supervisor.connect().unwrap();
    let mut spec = shell();
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        "stty raw -echo; printf '\\nBLOCK_READY\\n'; sleep 10".into(),
    ];
    let info = first.spawn(spec).unwrap();
    let old = first.attach(info.seat, 1).unwrap();
    output_until(&first, info.seat, b"BLOCK_READY\n");
    let bytes = vec![b'x'; MAX_INPUT_BYTES];
    let mut blocked = None;
    for _ in 0..32 {
        let id = Ulid::new();
        let receipt = first.input(&old, id, &bytes).unwrap();
        if receipt.state == InputState::Pending {
            blocked = Some(id);
            break;
        }
    }
    let id = blocked.expect("PTY never applied backpressure to 2 MiB without a reader");
    let second = supervisor.connect().unwrap();
    let new = second.attach(info.seat, 1).unwrap();
    assert_eq!(
        second
            .input(&new, Ulid::new(), b"must not interleave")
            .unwrap_err()
            .code,
        FailureCode::Busy
    );
    let deadline = Instant::now() + Duration::from_secs(4);
    let final_receipt = loop {
        let receipt = status(&second, &new, id);
        if receipt.state != InputState::Pending {
            break receipt;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(final_receipt.state, InputState::PartialUnknown);
    assert!(final_receipt.bytes_written < final_receipt.bytes_total);
    assert_eq!(second.input(&new, id, &bytes).unwrap(), final_receipt);
    eprintln!(
        "blocked write: {} of {} bytes, PartialUnknown retained across logic replacement",
        final_receipt.bytes_written, final_receipt.bytes_total
    );
}

#[test]
fn natural_exit_follows_all_final_output() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = shell();
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        "printf FINAL_BYTES; exit 7".into(),
    ];
    let info = client.spawn(spec).unwrap();
    client.attach(info.seat, 1).unwrap();
    let mut output = Vec::new();
    let mut next_sequence = 1;
    let deadline = Instant::now() + DEADLINE;
    loop {
        assert!(Instant::now() < deadline);
        match client.recv_timeout(Duration::from_secs(2)).unwrap() {
            Event::Output {
                seat,
                generation,
                seq,
                bytes,
            } => {
                assert_eq!((seat, generation, seq), (info.seat, 1, next_sequence));
                next_sequence += 1;
                output.extend(bytes);
            }
            Event::Exited {
                seat,
                generation,
                seq,
                status,
            } => {
                assert_eq!((seat, generation, seq), (info.seat, 1, next_sequence));
                assert_eq!(status.code, 7);
                assert_eq!(status.signal, None);
                break;
            }
            event => panic!("unexpected {event:?}"),
        }
    }
    assert_eq!(output, b"FINAL_BYTES");
}

#[test]
fn slow_reader_backpressures_and_recovers_the_complete_output() {
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = shell();
    spec.args = vec![
        "-f".into(),
        "-c".into(),
        "dd if=/dev/zero bs=65536 count=64 2>/dev/null; printf TAIL".into(),
    ];
    let info = client.spawn(spec).unwrap();
    client.attach(info.seat, 1).unwrap();
    // More than the 128 x 16 KiB channel plus one pending chunk: the child
    // cannot finish until this test starts consuming, rather than losing bytes.
    std::thread::sleep(Duration::from_millis(100));
    let Reply::Seats { seats } = client.request(Action::ListOwnedSeats).unwrap() else {
        panic!()
    };
    assert!(seats[0].exited.is_none());
    let mut count = 0;
    let mut next_sequence = 1;
    let mut tail = Vec::new();
    let deadline = Instant::now() + DEADLINE;
    loop {
        assert!(Instant::now() < deadline);
        match client.recv_timeout(Duration::from_secs(2)).unwrap() {
            Event::Output {
                seat,
                generation,
                seq,
                bytes,
            } => {
                assert_eq!((seat, generation, seq), (info.seat, 1, next_sequence));
                next_sequence += 1;
                for byte in bytes {
                    if count < 4 * 1024 * 1024 {
                        assert_eq!(byte, 0);
                    } else {
                        tail.push(byte);
                    }
                    count += 1;
                }
            }
            Event::Exited { seq, status, .. } => {
                assert_eq!(seq, next_sequence);
                assert_eq!(status.code, 0);
                break;
            }
            event => panic!("unexpected {event:?}"),
        }
    }
    assert_eq!(count, 4 * 1024 * 1024 + 4);
    assert_eq!(tail, b"TAIL");
    eprintln!("slow subscriber: complete 4 MiB + TAIL recovered with contiguous record sequences");
}
