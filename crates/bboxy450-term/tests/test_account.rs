//! Private child processes, generated file identity, no window or native custody.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private-process recovery fixtures"
)]
use std::{path::Path, time::Duration};

use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::{
    Ulid,
    store::{Store, object_dir, sqlite::SqliteStore},
};
use bbt_proto::{GuiAction, GuiReply, GuiSeat, SpawnSpec, SwarmId, Where};

fn authority(run: &Path) -> serde_json::Value {
    let account = SqliteStore::open(&object_dir(run, "account").unwrap()).unwrap();
    let head: serde_json::Value =
        serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
    let context: serde_json::Value =
        serde_json::from_slice(&account.get("identity/test-account").unwrap().unwrap()).unwrap();
    serde_json::json!({"head": head, "context": context})
}

async fn spawn(session: &mut Session, cwd: &Path) -> GuiSeat {
    let operation = Ulid::new();
    session
        .send(GuiAction::Spawn {
            id: operation,
            spec: SpawnSpec {
                swarm: SwarmId::from_u128(57),
                harness_kind: "zsh".into(),
                name: Some("test file identity".into()),
                role: None,
                cwd: cwd.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: None,
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Spawned { op, seat } if op == operation => {
                    let seats = session.list().await.unwrap();
                    return seats
                        .into_iter()
                        .find(|item| bbt_proto::SeatId::from_u128(item.record.id.0) == seat)
                        .unwrap();
                }
                GuiReply::Error { op, reason } if op == operation => {
                    panic!("spawn after file bootstrap: {reason}")
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logic_restart_keeps_the_test_root_device_pty_and_operator_spawn_authority() {
    let root = tempfile::Builder::new()
        .prefix("u0-")
        .tempdir_in("/tmp")
        .unwrap();
    let repository = root.path().join("repository");
    let shell_config = root.path().join("zsh-config");
    tokio::fs::create_dir_all(&repository).await.unwrap();
    tokio::fs::create_dir_all(&shell_config).await.unwrap();
    let run = root.path().join("run");
    let mut session = Session::start(Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository,
        shell_config: Some(shell_config),
        ctl: true,
        test_account: Some(root.path().join("keys")),
    })
    .await
    .unwrap();
    let before = authority(&run);
    assert!(
        session
            .endpoint
            .identity
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("test-account:"))
    );
    let first = spawn(&mut session, root.path()).await;
    let old_logic = session.logic.id();
    let supervisor = session.supervisor.id();
    session.logic.start_kill().unwrap(); // this Session's own recorded child only
    let restored = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Event::Reply(reply) = session.event().await.unwrap()
                && let GuiReply::Seats(seats) = *reply
            {
                break seats;
            }
        }
    })
    .await
    .unwrap();
    let kept = restored
        .iter()
        .find(|seat| seat.record.id == first.record.id)
        .unwrap();
    assert_eq!(kept.record.generation, first.record.generation);
    assert_eq!(kept.state.process, first.state.process);
    assert_eq!(session.supervisor.id(), supervisor);
    assert_ne!(session.logic.id(), old_logic);
    assert_eq!(authority(&run), before);
    let second = spawn(&mut session, root.path()).await;
    assert_ne!(second.record.id, first.record.id);
    assert_eq!(authority(&run), before);
    session.shutdown().await.unwrap();
}

#[test]
fn test_account_selector_is_explicit_and_rejects_bad_flags_before_custody() {
    let parse = |args: &[&str]| {
        Config::test_account_from_args(
            &args
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(parse(&[]).unwrap(), None);
    assert!(parse(&["--test-account"]).is_err());
    assert!(parse(&["--test-account", "relative"]).is_err());
    assert!(parse(&["--test-account", "/a/../b"]).is_err());
    assert!(parse(&["--unattended"]).is_err());
    assert!(parse(&["--test-account-typo", "/a"]).is_err());
    assert!(parse(&["--test-account="]).is_err());
    assert!(parse(&["--unattended=true"]).is_err());
    assert_eq!(parse(&["--test-account=/a"]).unwrap(), Some("/a".into()));
    assert!(parse(&["--test-account", "/a", "--test-account", "/b"]).is_err());
    assert_eq!(
        parse(&["--unattended", "--test-account", "/a"]).unwrap(),
        Some("/a".into())
    );
}
