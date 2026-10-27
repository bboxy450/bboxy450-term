//! Real private-process recovery, with isolated custody and fixture harnesses.
#![allow(
    clippy::disallowed_methods,
    reason = "headless integration tests own their Tokio runtimes"
)]

#[path = "support/events.rs"]
mod events;
use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::{
    Ulid,
    store::{Store, sqlite::SqliteStore},
    streams::Replica,
};
use bbt_proto::{
    GuiAction, GuiReply, GuiSeat, ProcessStatus, Screen, SeatId, SpawnSpec, SwarmId, Update, View,
    Where,
};
use events::StateEvents;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

async fn script(path: &Path, text: String) {
    tokio::fs::write(path, text).await.unwrap();
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
}

// keeps: spec c1-live-seats §4
#[test]
#[ignore = "private Codex service stand-in, exercised by the recovery tests"]
fn codex_service_fixture() {
    let socket = PathBuf::from(std::env::var_os("BBT_RECOVERY_CODEX_SOCKET").unwrap());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            use tokio::signal::unix::{SignalKind, signal};
            let mut hangup = signal(SignalKind::hangup()).unwrap();
            let mut terminate = signal(SignalKind::terminate()).unwrap();
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            loop {
                tokio::select! {
                    _ = hangup.recv() => break,
                    _ = terminate.recv() => break,
                    stream = listener.accept() => drop(stream.unwrap()),
                }
            }
            // Like the real Codex server, retire the locator on lifetime HUP.
            drop(listener);
            std::fs::remove_file(socket).unwrap();
        });
}

struct AliasGuard {
    path: PathBuf,
    target: PathBuf,
}
impl Drop for AliasGuard {
    fn drop(&mut self) {
        if std::fs::read_link(&self.path).ok().as_ref() == Some(&self.target) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

async fn start(root: &Path) -> Session {
    tokio::fs::create_dir_all(root.join("repository"))
        .await
        .unwrap();
    tokio::fs::create_dir_all(root.join("zsh-config"))
        .await
        .unwrap();
    // The private service binds its readiness socket; only the TUI records argv
    // and reads the PTY. No native session, account or approval store is touched.
    for harness in ["claude", "codex"] {
        let server = if harness == "codex" {
            format!(
                "if [ \"$1\" = app-server ]; then\n  BBT_RECOVERY_CODEX_SOCKET=\"${{3#unix://}}\" exec '{}' --ignored --exact codex_service_fixture --nocapture\nfi\n",
                std::env::current_exe().unwrap().display()
            )
        } else {
            String::new()
        };
        script(&root.join(harness), format!("#!/bin/sh\n{server}while [ -e '{}' ]; do sleep 0.01; done\nprintf '%s\\n' \"$@\" >> '{}'\nprintf 'fixture-ready\\n'\nwhile IFS= read -r line; do printf 'fixture: %s\\n' \"$line\"; done\n", root.join(format!("{harness}-hold-argv")).display(), root.join(format!("{harness}-argv")).display())).await;
    }
    let executable = root.join("test-app");
    script(
        &executable,
        format!(
            "#!/bin/sh\nPATH='{}':\"$PATH\" BBOXY_TEST_CUSTODY='fixture:{}' exec '{}' \"$@\"\n",
            root.display(),
            root.join("fixture-custody").display(),
            env!("CARGO_BIN_EXE_bboxy450-term")
        ),
    )
    .await;
    let mut session = Session::start(Config {
        executable,
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository: root.join("repository"),
        shell_config: Some(root.join("zsh-config")),
        ctl: false,
        test_account: None,
    })
    .await
    .unwrap();
    events::attach(&mut session).await.unwrap();
    session
}
fn pid(seat: &GuiSeat) -> u32 {
    match seat.state.process {
        ProcessStatus::Running { pid } => pid,
        _ => panic!("not running: {seat:?}"),
    }
}
fn id(seat: &GuiSeat) -> SeatId {
    SeatId::from_u128(seat.record.id.0)
}
async fn spawn(session: &mut Session, root: &Path, harness: &str, resume: Option<&str>) -> GuiSeat {
    let known: Vec<_> = session
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.record.id)
        .collect();
    session
        .send(GuiAction::Spawn {
            id: bbt_mux::Ulid::new(),
            spec: SpawnSpec {
                args: Vec::new(),
                env: Vec::new(),
                command: None,
                swarm: SwarmId::from_u128(57),
                harness_kind: harness.into(),
                name: Some(harness.into()),
                role: None,
                cwd: root.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: resume.map(str::to_owned),
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.receive_state().await.unwrap() {
                GuiReply::Seat(seat) if !known.contains(&seat.record.id) => return seat,
                GuiReply::Error { reason, .. } => panic!("spawn: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}
async fn watch(session: &mut Session, seat: &GuiSeat) {
    session
        .send(GuiAction::Watch {
            seat: id(seat),
            view: View {
                seat: seat.record.id,
                generation: seat.record.generation,
                view_generation: Ulid::nil(),
            },
        })
        .await
        .unwrap();
}
async fn update(session: &mut Session, seat: SeatId) -> Update {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.receive_state().await.unwrap() {
                GuiReply::Update { seat: got, update } if got == seat => return update,
                GuiReply::Error { reason, .. } => panic!("GUI: {reason}"),
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}
fn cursor(update: &Update) -> (View, u64) {
    match update {
        Update::Snapshot { snapshot } => (snapshot.view, snapshot.sequence),
        Update::Delta { delta } => (delta.view, delta.sequence),
    }
}
async fn apply(session: &mut Session, seat: SeatId, replica: &mut Replica, update: Update) {
    let (view, sequence) = cursor(&update);
    replica.apply(update).unwrap();
    // The Replica's new state is available before acknowledging, as in IO.
    session
        .send(GuiAction::Applied {
            seat,
            view,
            sequence,
        })
        .await
        .unwrap();
}
fn text(screen: &Screen) -> String {
    screen.cells.iter().flatten().map(|c| c.c).collect()
}
async fn input(session: &mut Session, seat: SeatId, generation: u64, bytes: &[u8]) {
    session
        .send(GuiAction::Input {
            seat,
            generation,
            id: Ulid::new(),
            bytes: bytes.into(),
        })
        .await
        .unwrap();
}
async fn until_text(session: &mut Session, seat: SeatId, replica: &mut Replica, wanted: &str) {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let next = update(session, seat).await;
            apply(session, seat, replica, next).await;
            if replica
                .snapshot
                .as_ref()
                .unwrap()
                .screen
                .cells
                .iter()
                .any(|row| {
                    let line: String = row.iter().map(|cell| cell.c).collect();
                    line.trim() == wanted
                })
            {
                break;
            }
        }
    })
    .await
    .unwrap();
}
async fn restarted(session: &mut Session) -> Vec<GuiSeat> {
    restart_catalogue(async || session.event().await)
        .await
        .unwrap()
}
async fn restart_catalogue(
    mut event: impl AsyncFnMut() -> bbt_mux::store::Result<Event>,
) -> bbt_mux::store::Result<Vec<GuiSeat>> {
    let started = tokio::time::Instant::now();
    let mut last = String::from("waiting for process exit/recovery");
    // Session gives startup 30 s after observing/reaping the old processes.
    // This is a hang watchdog, not a stricter recovery performance budget.
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            match event().await? {
                Event::Reply(reply) => {
                    if let GuiReply::Seats(seats) = *reply {
                        println!(
                            "restart catalogue after {:?}; last={last}",
                            started.elapsed()
                        );
                        return Ok(seats);
                    }
                    last = "non-catalogue reply".into();
                }
                Event::Exited { process, status } => last = format!("{process} exited: {status}"),
                Event::Disconnected(reason) => {
                    if reason.starts_with("recovery failed:") {
                        return Err(bbt_mux::store::invalid(reason));
                    }
                    last = format!("disconnected: {reason}");
                }
            }
        }
    })
    .await
    .map_err(|_| {
        bbt_mux::store::invalid(format!(
            "restart catalogue watchdog elapsed after {:?}; last={last}",
            started.elapsed()
        ))
    })?
}

// keeps: spec c1-live-seats §4
#[tokio::test(start_paused = true)]
async fn restart_catalogue_accepts_completion_within_the_sessions_startup_budget() {
    // Session allows 30 s for recovery startup. A scheduling delay within that
    // budget must not turn a functional catalogue assertion into a speed test.
    let seats = restart_catalogue(async || {
        tokio::time::sleep(Duration::from_secs(16)).await;
        Ok(Event::Reply(Box::new(GuiReply::Seats(Vec::new()))))
    })
    .await
    .unwrap();
    assert!(seats.is_empty());
}

// keeps: spec c1-live-seats §4
#[tokio::test(start_paused = true)]
async fn restart_catalogue_preserves_terminal_recovery_failures() {
    let error = restart_catalogue(async || {
        Ok(Event::Disconnected(
            "recovery failed: startup refused".into(),
        ))
    })
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "recovery failed: startup refused");
}

// keeps: spec c1-live-seats §4
#[tokio::test(start_paused = true)]
async fn restart_catalogue_still_rejects_a_hang() {
    let started = tokio::time::Instant::now();
    let error = restart_catalogue(async || std::future::pending().await)
        .await
        .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(60));
    assert!(error.to_string().contains("watchdog elapsed"));
    assert!(
        error
            .to_string()
            .contains("waiting for process exit/recovery")
    );
}
async fn durable(root: &Path, seat: &GuiSeat, sequence: u64) {
    let path = root.join(format!(
        "run/seat-{}/ring-{}/committed",
        seat.record.id, seat.record.generation
    ));
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Ok(bytes) = tokio::fs::read(&path).await
                && bytes.len() > 4
                && let Ok(boundary) =
                    serde_json::from_slice::<serde_json::Value>(&bytes[..bytes.len() - 4])
                && boundary["sequence"].as_u64().is_some_and(|n| n >= sequence)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logic_death_keeps_pty_generation_rewatches_and_preserves_ack_backpressure() {
    let root = tempfile::Builder::new()
        .prefix("c1c-logic-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let seat = spawn(&mut session, root.path(), "zsh", None).await;
    let unwatched = spawn(&mut session, root.path(), "zsh", None).await;
    let supervisor_pid = session.supervisor.id();
    let logic_pid = session.logic.id();
    watch(&mut session, &seat).await;
    let mut replica = Replica::default();
    input(
        &mut session,
        id(&seat),
        seat.record.generation,
        b"printf 'before-restart\\n'\n",
    )
    .await;
    until_text(&mut session, id(&seat), &mut replica, "before-restart").await;
    let previous_view = replica.snapshot.as_ref().unwrap().view;
    session.logic.start_kill().unwrap();
    loop {
        if matches!(
            session.event().await.unwrap(),
            Event::Exited {
                process: "logic",
                ..
            }
        ) {
            break;
        }
    }
    // Cancellation is exercised directly by the pending-startup regression.
    let seats = restarted(&mut session).await;
    let restored = seats
        .iter()
        .find(|s| s.record.id == seat.record.id)
        .unwrap();
    assert_eq!(seats.len(), 2);
    let restored_unwatched = seats
        .iter()
        .find(|item| item.record.id == unwatched.record.id)
        .unwrap();
    assert_eq!(pid(restored_unwatched), pid(&unwatched));
    assert_eq!(
        restored_unwatched.record.generation,
        unwatched.record.generation
    );
    assert_eq!(pid(restored), pid(&seat));
    assert_eq!(restored.record.generation, seat.record.generation);
    assert_eq!(session.supervisor.id(), supervisor_pid);
    assert_ne!(session.logic.id(), logic_pid);
    let first = update(&mut session, id(&seat)).await;
    let (view, _) = cursor(&first);
    assert_eq!(view.generation, previous_view.generation);
    assert_ne!(view.view_generation, previous_view.view_generation);
    assert!(matches!(first, Update::Snapshot { .. }));
    input(
        &mut session,
        id(&seat),
        seat.record.generation,
        b"printf 'after-restart\\n'\n",
    )
    .await;
    // A control reply is a protocol barrier after the new input. It must not
    // release the already outstanding frame; only Applied below can do that.
    session.send(GuiAction::ListSeats).await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.receive_state().await.unwrap() {
                GuiReply::Seats(_) => break,
                GuiReply::Update { .. } => panic!("second update before Applied"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    apply(&mut session, id(&seat), &mut replica, first).await;
    until_text(&mut session, id(&seat), &mut replica, "after-restart").await;
    println!(
        "logic recovery: seat={} pid={} generation={} supervisor={:?} logic {:?}->{:?}; fresh view; control-barrier ack hold",
        seat.record.id,
        pid(&seat),
        seat.record.generation,
        supervisor_pid,
        logic_pid,
        session.logic.id()
    );
    session.shutdown().await.unwrap();
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_death_replays_ring_with_gap_same_ids_new_generation_and_native_resume_argv() {
    let root = tempfile::Builder::new()
        .prefix("c1c-super-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let zsh = spawn(&mut session, root.path(), "zsh", None).await;
    let claude = spawn(
        &mut session,
        root.path(),
        "claude",
        Some("fixture-claude-session"),
    )
    .await;
    let codex = spawn(
        &mut session,
        root.path(),
        "codex",
        Some("fixture-codex-thread"),
    )
    .await;
    let store =
        SqliteStore::open(&root.path().join(format!("run/seat-{}", codex.record.id))).unwrap();
    let old_codex_endpoint: Option<PathBuf> =
        serde_json::from_slice(&store.get("adapter_runtime_endpoint").unwrap().unwrap()).unwrap();
    let old_codex_endpoint = old_codex_endpoint.unwrap();
    let old_alias = AliasGuard {
        path: old_codex_endpoint.parent().unwrap().into(),
        target: std::fs::read_link(old_codex_endpoint.parent().unwrap()).unwrap(),
    };
    assert_eq!(old_alias.path.parent(), Some(Path::new("/tmp")));
    assert!(
        old_alias
            .path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("bboxy-cx-")
    );
    assert!(
        old_alias.target.starts_with(
            root.path()
                .join(format!("run/seat-{}/overlays", codex.record.id))
        )
    );
    watch(&mut session, &zsh).await;
    let mut replica = Replica::default();
    input(
        &mut session,
        id(&zsh),
        zsh.record.generation,
        b"printf 'durable-before-supervisor\\n'\n",
    )
    .await;
    until_text(
        &mut session,
        id(&zsh),
        &mut replica,
        "durable-before-supervisor",
    )
    .await;
    session
        .send(GuiAction::Resize {
            seat: id(&zsh),
            generation: zsh.record.generation,
            cols: 100,
            rows: 30,
        })
        .await
        .unwrap();
    loop {
        let next = update(&mut session, id(&zsh)).await;
        apply(&mut session, id(&zsh), &mut replica, next).await;
        if replica.snapshot.as_ref().unwrap().screen.cols == 100 {
            break;
        }
    }
    durable(
        root.path(),
        &zsh,
        replica.snapshot.as_ref().unwrap().sequence,
    )
    .await;
    let old_supervisor = session.supervisor.id();
    let old_logic = session.logic.id();
    // These actions were formed against the displayed old process and held
    // across the outage. Neither may reach the replacement process.
    let stale_id = Ulid::new();
    let stale_file = root.path().join("stale-input-ran");
    let stale_input = GuiAction::Input {
        seat: id(&zsh),
        generation: zsh.record.generation,
        id: stale_id,
        bytes: format!(": > '{}'\n", stale_file.display()).into_bytes(),
    };
    let stale_resize = GuiAction::Resize {
        seat: id(&zsh),
        generation: zsh.record.generation,
        cols: 77,
        rows: 7,
    };
    // Confirm the first fixture has executed before holding only its replacement.
    let mut initial_argv = String::new();
    let initial_published = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            initial_argv = tokio::fs::read_to_string(root.path().join("codex-argv"))
                .await
                .unwrap_or_default();
            let args: Vec<_> = initial_argv.lines().collect();
            if args
                .windows(2)
                .any(|pair| pair == ["resume", "fixture-codex-thread"])
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(
        initial_published.is_ok(),
        "initial Codex argv publication timed out: {initial_argv}"
    );
    // A spawned process can be reported before its fixture has published argv.
    // Hold that child-side publication to exercise the observation race exactly.
    let argv_hold = root.path().join("codex-hold-argv");
    tokio::fs::write(&argv_hold, b"").await.unwrap();
    session.supervisor.start_kill().unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if !old_codex_endpoint.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("old Codex fixture did not retire its socket on supervisor death");
    let seats = restarted(&mut session).await;
    let old_alias_retired = matches!(
        old_alias.path.symlink_metadata(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    );
    assert_eq!(seats.len(), 3);
    for old in [&zsh, &claude, &codex] {
        let new = seats.iter().find(|s| s.record.id == old.record.id).unwrap();
        assert_eq!(new.record.generation, old.record.generation + 1);
        assert_ne!(pid(new), pid(old));
        assert_eq!(new.record.harness_session_id, old.record.harness_session_id);
    }
    assert_ne!(session.supervisor.id(), old_supervisor);
    assert_ne!(session.logic.id(), old_logic);
    let next = update(&mut session, id(&zsh)).await;
    assert!(matches!(next, Update::Snapshot { .. }));
    apply(&mut session, id(&zsh), &mut replica, next).await;
    let screen = &replica.snapshot.as_ref().unwrap().screen;
    assert_eq!((screen.cols, screen.rows), (100, 30));
    assert!(text(screen).contains("durable-before-supervisor"));
    let gap = screen.gap.as_ref().unwrap();
    assert!(
        gap.contains("generation 1") && gap.contains("uncommitted output unavailable"),
        "{gap}"
    );
    for (harness, flag, session_id) in [
        ("claude", "--resume", "fixture-claude-session"),
        ("codex", "resume", "fixture-codex-thread"),
    ] {
        let path = root.path().join(format!("{harness}-argv"));
        let mut argv = String::new();
        let mut first_count = None;
        let mut observed_count = 0;
        let published = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                argv = tokio::fs::read_to_string(&path).await.unwrap();
                let args: Vec<_> = argv.lines().collect();
                observed_count = args
                    .windows(2)
                    .filter(|pair| pair == &[flag, session_id])
                    .count();
                if first_count.is_none() {
                    first_count = Some(observed_count);
                    println!("{harness} resume argv first read: {observed_count}, expected 2");
                    if harness == "codex" {
                        assert_eq!(observed_count, 1, "fixture argv publication gate");
                        tokio::fs::remove_file(&argv_hold).await.unwrap();
                    }
                }
                if observed_count >= 2 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            published.is_ok(),
            "{harness} resume argv publication timed out: first={first_count:?}, observed={observed_count}, expected=2\n{argv}"
        );
        if first_count != Some(observed_count) {
            println!(
                "{harness} resume argv publication wait was needed: first={first_count:?}, observed={observed_count}"
            );
        }
        let args: Vec<_> = argv.lines().collect();
        assert_eq!(
            args.windows(2)
                .filter(|pair| pair == &[flag, session_id])
                .count(),
            2,
            "{argv}"
        );
    }
    session.send(stale_input).await.unwrap();
    session.send(stale_resize).await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        for dropped in 1..=2 {
            loop {
                match session.receive_state().await.unwrap() {
                    GuiReply::Error { op, reason } => {
                        if dropped == 1 {
                            assert_eq!(op, stale_id, "Input rejection lost its operation ID");
                        }
                        assert!(reason.contains("StaleGeneration"), "{reason}");
                        assert!(
                            reason.contains("displayed generation 1, current 2"),
                            "{reason}"
                        );
                        assert!(reason.contains(&format!("dropped={dropped}")), "{reason}");
                        break;
                    }
                    GuiReply::Update { seat, update } => {
                        apply(&mut session, seat, &mut replica, update).await;
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .unwrap();
    input(
        &mut session,
        id(&zsh),
        replica.snapshot.as_ref().unwrap().view.generation,
        b"printf 'echo-after-supervisor\\n'; stty size\n",
    )
    .await;
    until_text(&mut session, id(&zsh), &mut replica, "30 100").await;
    assert!(!tokio::fs::try_exists(stale_file).await.unwrap());
    let screen = &replica.snapshot.as_ref().unwrap().screen;
    assert_eq!((screen.cols, screen.rows), (100, 30));
    assert!(text(screen).contains("echo-after-supervisor"));
    println!(
        "supervisor recovery: 3 stable seat IDs, gen1->2, zsh/Claude/Codex new PIDs; 100x30, retained screen, explicit gap, actual resume argv, post-restart echo; stale Input ID preserved, drops1/2 counted, stale input file absent, stale resize rejected at PTY30x100"
    );
    session.shutdown().await.unwrap();
    assert!(
        old_alias_retired,
        "old generation runtime alias survived generation 2 start: {}",
        old_alias.path.display()
    );
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_during_pending_recovery_reaps_children_without_restarting() {
    let root = tempfile::Builder::new()
        .prefix("c1c-stop-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    let seat = spawn(&mut session, root.path(), "zsh", None).await;
    session.logic.start_kill().unwrap();
    loop {
        if matches!(
            session.event().await.unwrap(),
            Event::Exited {
                process: "logic",
                ..
            }
        ) {
            break;
        }
    }
    session.shutdown().await.unwrap();
    assert!(
        !tokio::fs::try_exists(root.path().join("run/mux.sock"))
            .await
            .unwrap()
    );
    let status = tokio::process::Command::new("/bin/ps")
        .args(["-p", &pid(&seat).to_string(), "-o", "comm="])
        .output()
        .await
        .unwrap();
    assert!(!status.status.success(), "owned seat survived shutdown");
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partial_startup_survives_cancelled_poll_and_shutdown_reaps_its_child() {
    let root = tempfile::Builder::new()
        .prefix("c1c-handshake-")
        .tempdir()
        .unwrap();
    let mut session = start(root.path()).await;
    // Replace only this fixture's executable. Its next logic child records its
    // own PID, consumes bootstrap, then holds readiness on the inherited pipe.
    script(&root.path().join("test-app"), format!(
        "#!/bin/sh\nif [ \"$1\" = logic ]; then\n  echo $$ > '{}'\n  read bootstrap\n  read stop\n  exit 0\nfi\nBBOXY_TEST_CUSTODY=memory exec '{}' \"$@\"\n",
        root.path().join("pending-pid").display(), env!("CARGO_BIN_EXE_bboxy450-term")
    )).await;
    session.logic.start_kill().unwrap();
    loop {
        if matches!(
            session.event().await.unwrap(),
            Event::Exited {
                process: "logic",
                ..
            }
        ) {
            break;
        }
    }
    let pending_pid = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Ok(pid) = tokio::fs::read_to_string(root.path().join("pending-pid")).await {
                break pid.trim().to_owned();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    use std::{
        future::Future,
        task::{Context, Waker},
    };
    assert!(
        std::pin::pin!(session.event())
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Known loss returns promptly; the owner can retain this stable-ID command
    // and its independent close channel remains selectable during startup.
    assert!(
        tokio::time::timeout(Duration::from_secs(30), session.send(GuiAction::ListSeats))
            .await
            .unwrap()
            .is_err()
    );
    session.shutdown().await.unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let status = tokio::process::Command::new("/bin/ps")
                .args(["-p", &pending_pid, "-o", "comm="])
                .output()
                .await
                .unwrap();
            if !status.status.success() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    println!(
        "cancelled readiness future retained startup; shutdown reaped recorded replacement PID {pending_pid}"
    );
}

async fn raw_supervisor(root: &Path) -> (tokio::process::Child, bbt_mux::logic::SocketSupervisor) {
    use tokio::io::{AsyncWriteExt, BufReader};
    #[derive(serde::Deserialize)]
    struct Ready {
        socket: std::path::PathBuf,
        capability: [u8; 32],
    }
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .arg("supervisor")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let config = Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: root.join("run/mux.sock"),
        run_dir: root.join("run"),
        repository: root.into(),
        shell_config: None,
        ctl: false,
        test_account: None,
    };
    let mut bytes = serde_json::to_vec(&config).unwrap();
    bytes.push(b'\n');
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&bytes)
        .await
        .unwrap();
    let line = tokio::time::timeout(
        Duration::from_secs(30),
        bbt_mux::seat_socket::read_line(&mut BufReader::new(child.stdout.take().unwrap())),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let ready: Ready = serde_json::from_slice(&line).unwrap();
    let client =
        bbt_mux::logic::SocketSupervisor::connect(&ready.socket, ready.capability).unwrap();
    (child, client)
}

// keeps: spec c1-live-seats §4
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovered_generation_admission_rejects_unknown_and_conflicting_launches_but_exact_retry_is_idempotent()
 {
    use bbt_mux::{Action, Reply, logic::SupervisorConnection};
    use tokio::io::AsyncWriteExt;
    let root = tempfile::Builder::new()
        .prefix("c1c-admit-")
        .tempdir()
        .unwrap();
    let (mut first, mut client) = raw_supervisor(root.path()).await;
    let mut spec = bbt_mux::LaunchSpec::new("/bin/cat", root.path());
    let info = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("spawn mismatch"),
    };
    let attachment = match client
        .request(Action::AttachRing {
            seat: info.seat,
            generation: 1,
            directory: root.path().join(format!("run/seat-{}/ring-1", info.seat)),
            settings: Default::default(),
        })
        .unwrap()
    {
        Reply::Attached { attachment, .. } => attachment,
        _ => panic!("attach mismatch"),
    };
    client
        .request(Action::Input {
            attachment: attachment.clone(),
            input_id: Ulid::new(),
            bytes: b"durable-generation\n".to_vec(),
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Reply::RingRecovery { status, .. } = client
                .request(Action::RingRecovery {
                    attachment: attachment.clone(),
                })
                .unwrap()
                && status.committed_sequence > 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    first.start_kill().unwrap();
    first.wait().await.unwrap();
    drop(client);
    let predecessor = root.path().join(format!("run/seat-{}/ring-1", spec.seat));
    // Recovery must also work with a valid previous boundary and a torn current
    // marker, using Ring's existing validation/repair rather than a new format.
    tokio::fs::copy(predecessor.join("committed"), predecessor.join("previous"))
        .await
        .unwrap();
    tokio::fs::write(predecessor.join("committed"), b"torn marker")
        .await
        .unwrap();
    let (mut second, mut client) = raw_supervisor(root.path()).await;
    assert!(
        client
            .request(Action::Spawn { spec: spec.clone() })
            .is_err(),
        "recovered old generation was reused"
    );
    let mut gap = spec.clone();
    gap.generation = 3;
    assert!(
        client.request(Action::Spawn { spec: gap }).is_err(),
        "generation gap admitted"
    );
    spec.generation = 2;
    let occupied = root.path().join(format!("run/seat-{}/ring-2", spec.seat));
    tokio::fs::create_dir(&occupied).await.unwrap();
    assert!(
        client
            .request(Action::Spawn { spec: spec.clone() })
            .is_err(),
        "occupied destination generation was reused"
    );
    tokio::fs::remove_dir(occupied).await.unwrap();
    let info = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("resume mismatch"),
    };
    let retry = match client
        .request(Action::Spawn { spec: spec.clone() })
        .unwrap()
    {
        Reply::Spawned { info } => info,
        _ => panic!("retry mismatch"),
    };
    assert_eq!(retry.pid, info.pid);
    let mut conflict = spec.clone();
    conflict.args.push("different".into());
    assert!(client.request(Action::Spawn { spec: conflict }).is_err());
    let mut unknown = spec.clone();
    unknown.seat = Ulid::new();
    let unknown_root = root.path().join(format!("run/seat-{}", unknown.seat));
    assert!(
        client.request(Action::Spawn { spec: unknown }).is_err(),
        "unknown seat started at generation2"
    );
    assert!(
        !tokio::fs::try_exists(unknown_root).await.unwrap(),
        "admission created a ring for an unknown seat"
    );
    let mut corrupt = spec.clone();
    corrupt.seat = Ulid::new();
    let corrupt_ring = root
        .path()
        .join(format!("run/seat-{}/ring-1", corrupt.seat));
    tokio::fs::create_dir_all(&corrupt_ring).await.unwrap();
    tokio::fs::write(corrupt_ring.join("committed"), b"invalid boundary")
        .await
        .unwrap();
    assert!(client.request(Action::Spawn { spec: corrupt }).is_err());
    assert_eq!(
        tokio::fs::read_to_string(corrupt_ring.join("committed"))
            .await
            .unwrap(),
        "invalid boundary",
        "invalid predecessor was changed during admission"
    );
    assert_eq!(std::fs::read_dir(&corrupt_ring).unwrap().count(), 1);
    let mut empty = spec;
    empty.seat = Ulid::new();
    let empty_ring = root.path().join(format!("run/seat-{}/ring-1", empty.seat));
    tokio::fs::create_dir_all(&empty_ring).await.unwrap();
    assert!(
        client.request(Action::Spawn { spec: empty }).is_err(),
        "empty ring directory proved a generation"
    );
    assert!(
        tokio::fs::read_dir(&empty_ring)
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .is_none(),
        "admission initialized an empty predecessor"
    );
    second
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"stop\n")
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(30), second.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    println!(
        "admission: valid previous boundary authorized gen1 -> gen2; old/gap/occupied/unknown/empty/corrupt/conflicting generations rejected; identical gen2 retry returned PID {}",
        info.pid
    );
}
