// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended filesystem/socket witnesses, never UI"
)]
use bboxy450_harness_wake_up::{
    Binding, Grade, HarnessRecord, Session, State, StateSource, Ulid,
    harness_log::{self, Log},
};
use serde_json::{Value, json};
use std::io::Write;

fn session() -> Session {
    let mut record = HarnessRecord::builtin("zsh").unwrap();
    record.kind = "bboxy450-harness".into();
    record.state_source = StateSource::Cadence;
    Session::new(
        record,
        Binding {
            seat: Ulid::new(),
            session: format!("process-{}", std::process::id()),
            pid: std::process::id(),
            process_start: "fixture".into(),
            generation: 1,
            attachment_generation: 1,
            runtime: None,
        },
    )
    .unwrap()
}
fn report(s: &mut Session, state: &str, time: u64) {
    s.osc(
        &[b"7501".to_vec(), format!("state={state}").into_bytes()],
        time,
        time,
    );
}

#[test]
fn reported_generation_never_returns_to_cadence_even_after_staleness() {
    let mut s = session();
    s.output(1000);
    s.tick(1, 1000);
    assert_eq!(s.status.source, "cadence");
    report(&mut s, "working", 2000);
    for time in [7000, 22_000, 61_999] {
        s.tick(2, time);
        assert_eq!(
            (s.status.state, s.status.grade, s.status.source.as_str()),
            (State::Active, Grade::Reported, "7501")
        );
    }
    s.tick(2, 62_000);
    assert_eq!(s.status.source, "stale");
    assert_eq!(s.status.detail.as_deref(), Some("· no word for 1m"));
    s.output(62_001);
    s.tick(3, 99_000);
    assert_eq!(s.status.source, "stale");
    let mut lower = s.history().back().unwrap().clone();
    lower.source = "7502".into();
    lower.grade = Grade::Shim;
    lower.state = State::Waiting;
    lower.observed_ms = 99_999;
    assert!(!s.apply(lower));
    assert_eq!(s.status.source, "stale");
    // Parent precedence must not disable independent hook subagent lifecycles.
    let mut child = s.history().back().unwrap().clone();
    child.agent_id = Some("child".into());
    child.grade = Grade::Shim;
    child.state = State::Active;
    assert!(s.apply(child.clone()));
    assert_eq!(s.status.subagents.len(), 1);
    child.state = State::Idle;
    assert!(s.apply(child));
    assert!(s.status.subagents.is_empty());
    report(&mut s, "idle", 100_000);
    assert_eq!(s.status.state, State::Idle);
    assert_eq!(s.status.detail, None);
    s.exit(4, 100_100, 1);
    report(&mut s, "working", 100_200);
    s.tick(5, 200_000);
    assert_eq!(s.status.source, "exit");
    assert_eq!(s.status.state, State::Errored);
}

#[test]
fn heartbeat_extends_freshness_and_only_a_new_generation_reenables_cadence() {
    let mut s = session();
    report(&mut s, "working", 1000);
    s.heartbeat(60_000);
    s.tick(2, 119_999);
    assert_eq!(s.status.source, "7501");
    let mut binding = s.binding.clone();
    binding.attachment_generation += 1;
    s.rebind(binding.clone()).unwrap();
    assert_eq!(s.status.source, "7501");
    s.tick(3, 120_000);
    assert_eq!(s.status.source, "stale");
    binding.generation += 1;
    s.rebind(binding).unwrap();
    s.tick(4, 120_100);
    assert_eq!(s.status.source, "cadence");
    assert_eq!(s.status.grade, Grade::Inferred);
    // An old-generation completion must not latch the new generation.
    let mut old = s.history().back().unwrap().clone();
    old.generation = 1;
    old.grade = Grade::Reported;
    assert!(!s.apply(old));
    s.tick(5, 120_200);
    assert_eq!(s.status.source, "cadence");
}

struct Fixture {
    root: tempfile::TempDir,
    id: String,
    file: std::fs::File,
    seq: u64,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("sessions")).unwrap();
        let id = Ulid::new().to_string();
        let file = std::fs::File::create(root.path().join(format!("sessions/{id}.jsonl"))).unwrap();
        Self {
            root,
            id,
            file,
            seq: 0,
        }
    }
    fn event(&mut self, event: Value) -> String {
        self.seq += 1;
        format!(
            "{}\n",
            json!({"session":self.id,"v":{"major":1,"minor":1},"seq":self.seq,"epoch":1,"ts_ms":self.seq * 1000,"event":event})
        )
    }
    fn append(&mut self, event: Value) {
        let bytes = self.event(event);
        self.file.write_all(bytes.as_bytes()).unwrap();
    }
}
#[test]
fn log_reports_open_tool_and_approval_through_quiet_pty_then_clears_detail() {
    let mut f = Fixture::new();
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    let mut s = session();
    f.append(json!({"type":"turn_started","turn":"t"}));
    f.append(json!({"type":"item_started","turn":"t","item":"i","kind":"tool_call","head":{"kind":"tool_call","name":"bash"}}));
    let update = |log: &mut Log, s: &mut Session, now| {
        let mut event = log.poll().unwrap().unwrap();
        event["binding"] = serde_json::to_value(&s.binding).unwrap();
        harness_log::observe(s, &event, now, now).unwrap();
    };
    update(&mut log, &mut s, 1000);
    for time in [1001, 7000, 21_000] {
        assert!(log.poll().unwrap().is_none());
        s.tick(4, time);
        assert_eq!(
            (s.status.state, s.status.grade, s.status.detail.as_deref()),
            (State::Active, Grade::Reported, Some("bash"))
        );
    }
    f.append(json!({"type":"item_started","turn":"t","item":"a","kind":"approval"}));
    update(&mut log, &mut s, 22_000);
    assert_eq!(s.status.state, State::Waiting);
    f.append(json!({"type":"item_completed","item":"a"}));
    update(&mut log, &mut s, 23_000);
    assert_eq!(s.status.state, State::Active);
    assert_eq!(s.status.detail.as_deref(), Some("bash"));
    f.append(json!({"type":"item_completed","item":"i"}));
    f.append(json!({"type":"turn_ended","turn":"t","reason":{"reason":"done"}}));
    update(&mut log, &mut s, 24_000);
    assert_eq!(s.status.state, State::Idle);
    assert_eq!(s.status.detail, None);
    // Socket/log completion A → B → A uses an attachment fence, not just its ID.
    let mut event = json!({"session":f.id,"state":"active","binding":s.binding});
    let old = s.binding.clone();
    for id in [Ulid::new().to_string(), f.id.clone()] {
        let mut b = s.binding.clone();
        b.session = id;
        b.attachment_generation += 1;
        s.rebind(b).unwrap();
    }
    event["binding"] = serde_json::to_value(old).unwrap();
    assert!(harness_log::observe(&mut s, &event, 8, 25_000).is_err());
}

#[test]
fn ui_status_log_uses_operation_kind_without_copying_command_source() {
    for command in [
        "python3 - <<'PY'\nprint('界👩🏽‍💻')\nPY".to_owned(),
        "λ".repeat(4096),
    ] {
        let mut f = Fixture::new();
        let mut log = Log::open(f.root.path(), &f.id).unwrap();
        let mut s = session();
        f.append(json!({"type":"turn_started","turn":"t"}));
        f.append(json!({"type":"item_started","turn":"t","item":"i","kind":"terminal","head":{"command":command}}));
        let mut event = log.poll().unwrap().unwrap();
        assert_eq!(event["detail"], "terminal");
        event["binding"] = json!(s.binding);
        harness_log::observe(&mut s, &event, 1, 2000).unwrap();
        assert_eq!(s.status.detail.as_deref(), Some("terminal"));
        assert_eq!(s.status.state, State::Active);
        let raw = std::fs::read_to_string(f.root.path().join(format!("sessions/{}.jsonl", f.id)))
            .unwrap();
        let item: Value = serde_json::from_str(raw.lines().last().unwrap()).unwrap();
        assert_eq!(
            item["event"]["head"]["command"], command,
            "raw tool details must survive"
        );
    }
}

#[test]
fn log_retains_partial_lines_rejects_gaps_and_never_counts_eof_as_a_heartbeat() {
    let mut f = Fixture::new();
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    let line = f.event(json!({"type":"turn_started","turn":"t"}));
    f.file
        .write_all(&line.as_bytes()[..line.len() - 1])
        .unwrap();
    assert!(log.poll().unwrap().is_none());
    f.file.write_all(b"\n").unwrap();
    assert_eq!(log.poll().unwrap().unwrap()["state"], "active");
    assert!(log.poll().unwrap().is_none());
    f.seq += 1;
    f.append(json!({"type":"turn_ended","turn":"t"}));
    assert!(log.poll().unwrap_err().to_string().contains("gap"));
}

#[test]
fn log_rejects_wrong_sessions_replacement_and_symlinks() {
    let mut f = Fixture::new();
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    f.append(json!({"type":"turn_started","turn":"t"}));
    let path = f.root.path().join(format!("sessions/{}.jsonl", f.id));
    let replacement = f.root.path().join("replacement");
    std::fs::write(&replacement, "").unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    assert!(log.poll().is_err());
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("../replacement", &path).unwrap();
    assert!(Log::open(f.root.path(), &f.id).is_err());
}

#[test]
fn historical_prefix_is_not_a_fresh_state_and_wrong_session_is_refused() {
    let mut f = Fixture::new();
    f.append(json!({"type":"session_started"}));
    f.append(json!({"type":"item_completed","item":"old","body":{"markdown":"x".repeat(70_000)}}));
    f.append(json!({"type":"turn_started","turn":"t"}));
    f.append(json!({"type":"item_started","item":"i","kind":"tool_call","head":{"name":"bash"}}));
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    assert!(log.poll().unwrap().is_none());
    assert!(!log.caught_up());
    let state = log.poll().unwrap().unwrap();
    assert!(log.caught_up());
    assert_eq!(state["state"], "active");
    assert_eq!(state["detail"], "bash");
    f.id = Ulid::new().to_string();
    f.append(json!({"type":"turn_ended","turn":"t"}));
    assert!(
        log.poll()
            .unwrap_err()
            .to_string()
            .contains("session/version")
    );
}

#[test]
fn socket_discovery_requires_the_seat_peer_and_a_matching_native_hello() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("run")).unwrap();
    let id = Ulid::new().to_string();
    let path = root.path().join("run").join(format!("{id}.sock"));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let listener = tokio::net::UnixListener::bind(&path).unwrap();
            let server_id = id.clone();
            let server = tokio::spawn(async move {
                // Wrong PID is rejected before term sends even a Subscribe.
                let (mut peer, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                tokio::io::AsyncReadExt::read_to_end(&mut peer, &mut bytes)
                    .await
                    .unwrap();
                assert!(bytes.is_empty());
                let (peer, _) = listener.accept().await.unwrap();
                let mut peer = tokio::io::BufReader::new(peer);
                let mut request = String::new();
                peer.read_line(&mut request).await.unwrap();
                let request: Value = serde_json::from_str(&request).unwrap();
                assert!(request["subscribe"].get("session").is_none());
                assert!(request.get("op").is_none());
                peer.get_mut()
                    .write_all(
                        format!(
                            "{}\n",
                            json!({"v":{"major":1,"minor":1},"meta":{"session":server_id}})
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            });
            let s = session();
            let mut foreign = s.binding.clone();
            foreign.pid += 1;
            assert!(harness_log::hello(&path, &id, &foreign).await.is_err());
            assert_eq!(
                harness_log::discover(root.path(), &s.binding)
                    .await
                    .unwrap(),
                id
            );
            server.await.unwrap();
        });
}

#[test]
fn old_log_time_never_freshens_a_report_or_resurrects_stale_state() {
    let mut f = Fixture::new();
    f.append(json!({"type":"turn_started","turn":"t"}));
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    let mut event = log.poll().unwrap().unwrap();
    let mut s = session();
    report(&mut s, "working", 5000);
    event["binding"] = serde_json::to_value(&s.binding).unwrap();
    harness_log::observe(&mut s, &event, 2, 60_000).unwrap();
    assert_eq!(
        s.status.observed_ms, 1000,
        "native ts_ms is the observed time"
    );
    s.tick(3, 64_999);
    assert_ne!(s.status.source, "stale");
    s.tick(3, 65_000);
    assert_eq!(s.status.source, "stale");
    event["observed_ms"] = json!(5000);
    assert!(
        harness_log::observe(&mut s, &event, 4, 65_001)
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.status.source, "stale");
}

#[test]
fn completed_turns_do_not_expire_and_fresh_reports_replace_the_stale_label() {
    let mut s = session();
    for state in ["idle", "error"] {
        report(&mut s, state, 1000);
        s.tick(1, 100_000);
        assert_eq!(s.status.source, "7501", "closed turn {state}");
        assert!(!s.status.turn_open);
    }
    s.osc(
        &[b"7501".to_vec(), b"state=working:msg=YmFzaA==".to_vec()],
        2,
        101_000,
    );
    s.tick(3, 161_000);
    assert_eq!(s.status.detail.as_deref(), Some("bash · no word for 1m"));
    assert!(s.status.turn_open);
    s.heartbeat(162_000);
    s.tick(4, 162_001);
    assert_eq!(
        s.status.source, "stale",
        "heartbeat cannot invent fresh state"
    );
    report(&mut s, "working", 162_002);
    assert_eq!(s.status.source, "7501");
    assert_eq!(s.status.detail, None);
}

#[test]
fn restoring_a_stale_report_does_not_restart_its_freshness_hold() {
    let mut s = session();
    report(&mut s, "working", 1000);
    s.tick(1, 61_000);
    let mut retained = s.history().back().unwrap().clone();
    retained.source = s.status.source.clone();
    retained.detail = s.status.detail.clone();
    let mut restored = session();
    assert!(restored.apply(retained));
    restored.heartbeat(62_000);
    restored.tick(2, 62_001);
    assert_eq!(
        restored.status, s.status,
        "restore must not append a second stale marker"
    );
}

#[test]
fn harness_does_not_use_the_five_second_lower_grade_window() {
    let mut s = session();
    s.tick(1, 1000);
    let mut obs = s.history().back().unwrap().clone();
    obs.grade = Grade::Shim;
    obs.source = "fixture".into();
    obs.state = State::Active;
    assert!(s.apply(obs.clone()));
    obs.grade = Grade::Inferred;
    obs.state = State::Idle;
    obs.observed_ms = 1001;
    assert!(s.apply(obs));
    assert_eq!(s.status.state, State::Idle);
    report(&mut s, "working", 2000);
    s.tick(2, 7001);
    assert_eq!(
        s.status.state,
        State::Active,
        "generation latch still applies"
    );
}

#[test]
fn log_tails_consumed_bytes_readonly_and_session_closed_clears_an_error() {
    use std::os::unix::fs::FileExt;
    let mut f = Fixture::new();
    f.append(json!({"type":"turn_started","turn":"t"}));
    let mut log = Log::open(f.root.path(), &f.id).unwrap();
    assert_eq!(log.poll().unwrap().unwrap()["state"], "active");
    // Corrupt only this fixture's already-consumed prefix. A full reread would
    // fail JSON decoding; the retained cursor must read only appended bytes.
    f.file.write_at(b"!", 0).unwrap();
    f.append(
        json!({"type":"turn_ended","turn":"t","reason":{"reason":"error","message":"tool failed"}}),
    );
    let error = log.poll().unwrap().unwrap();
    assert_eq!(error["state"], "errored");
    assert_eq!(error["detail"], "tool failed");
    f.append(json!({"type":"session_closed"}));
    let before = std::fs::read(f.root.path().join(format!("sessions/{}.jsonl", f.id))).unwrap();
    let closed = log.poll().unwrap().unwrap();
    assert_eq!(closed["state"], "idle");
    assert_eq!(closed["detail"], Value::Null);
    assert_eq!(closed["observed_ms"], 3000);
    let mut s = session();
    let mut event = closed;
    event["observed_ms"] = json!(7000);
    event["binding"] = serde_json::to_value(&s.binding).unwrap();
    harness_log::observe(&mut s, &event, 1, 6000).unwrap();
    assert_eq!(
        s.status.observed_ms, 7000,
        "ts_ms is not clamped to poll time"
    );
    assert_eq!(
        std::fs::read(f.root.path().join(format!("sessions/{}.jsonl", f.id))).unwrap(),
        before
    );
}

#[test]
fn phase_heartbeat_retains_native_item_detail_and_event_time() {
    let mut s = session();
    let event = json!({"binding":s.binding,"session":Ulid::new(),"state":"active","detail":"bash","observed_ms":1000});
    harness_log::observe(&mut s, &event, 1, 9000).unwrap();
    s.osc(
        &[
            b"7501".to_vec(),
            b"state=working:msg=UnVuIGNvbW1hbmQ=".to_vec(),
        ],
        2,
        2000,
    );
    assert_eq!(s.status.source, "sessions_file");
    assert_eq!(s.status.detail.as_deref(), Some("bash"));
    assert_eq!(s.status.observed_ms, 1000);
    s.tick(3, 61_000);
    assert_eq!(
        s.status.source, "sessions_file",
        "phase heartbeat extended the separate freshness clock"
    );
    s.tick(4, 62_000);
    assert_eq!(s.status.source, "stale");
}
