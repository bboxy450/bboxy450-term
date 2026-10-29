// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private filesystem/account fixtures.
use bbt_mux::{registry::Registry, store::sqlite::SqliteStore};
use ulid::Ulid;
mod common;

#[test]
fn swarm_metadata_persists_canonical_paths_and_noops_keep_the_revision() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(&root.path().join("account")).unwrap());
    let id = Ulid::new();
    assert_eq!(registry.swarm(id).unwrap(), None);
    let reset = registry.update_swarm_workdir(id, None).unwrap();
    assert_eq!(reset.revision, 0);
    assert_eq!(registry.swarm(id).unwrap(), Some(reset));
    let named = registry.rename_swarm(id, "  Build the cockpit  ").unwrap();
    assert_eq!(named.name.as_deref(), Some("Build the cockpit"));
    assert_eq!(named.revision, 1);
    assert_eq!(
        registry.rename_swarm(id, "Build the cockpit").unwrap(),
        named
    );
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let alias = root.path().join("project-alias");
    std::os::unix::fs::symlink(&project, &alias).unwrap();
    let chosen = registry.update_swarm_workdir(id, alias.to_str()).unwrap();
    assert_eq!(
        chosen.workdir.as_deref(),
        project.canonicalize().unwrap().to_str()
    );
    assert_eq!(chosen.revision, 2);
    assert_eq!(
        registry.update_swarm_workdir(id, project.to_str()).unwrap(),
        chosen
    );
    let reopened = Registry::new(SqliteStore::open(&root.path().join("account")).unwrap());
    assert_eq!(reopened.swarm(id).unwrap(), Some(chosen.clone()));
    assert_eq!(reopened.swarms().unwrap(), vec![chosen]);
    let cleared = reopened.update_swarm_workdir(id, None).unwrap();
    assert_eq!(cleared.workdir, None);
    assert_eq!(cleared.revision, 3);
    assert_eq!(reopened.update_swarm_workdir(id, None).unwrap(), cleared);
}

#[test]
fn invalid_swarm_names_and_folders_never_mutate_metadata() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(&root.path().join("account")).unwrap());
    let id = Ulid::new();
    for name in ["".to_owned(), " \t ".into(), "a\nb".into(), "x".repeat(65)] {
        assert!(registry.rename_swarm(id, &name).is_err(), "{name:?}");
    }
    let file = root.path().join("file");
    std::fs::write(&file, "not a folder").unwrap();
    let missing = root.path().join("missing");
    for path in [
        "relative",
        "",
        "/tmp/invalid\npath",
        file.to_str().unwrap(),
        missing.to_str().unwrap(),
    ] {
        assert!(
            registry.update_swarm_workdir(id, Some(path)).is_err(),
            "{path:?}"
        );
    }
    assert!(registry.rename_swarm(Ulid::nil(), "nil").is_err());
    assert!(registry.update_swarm_workdir(Ulid::nil(), None).is_err());
    assert!(registry.swarm(Ulid::nil()).is_err());
    assert_eq!(registry.swarm(id).unwrap(), None);
    assert!(registry.swarms().unwrap().is_empty());
    let unicode = "狐".repeat(64);
    assert_eq!(
        registry.rename_swarm(id, &unicode).unwrap().name,
        Some(unicode)
    );
}

// keeps: native emoji names — Unicode sequences survive authoritative storage verbatim.
#[test]
fn emoji_swarm_names_preserve_variations_joiners_flags_and_modifiers_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("account");
    let id = Ulid::new();
    let name = "🦊 crew 👩🏽‍💻 🇺🇦 👨‍👩‍👧‍👦 ❤️ 1️⃣";
    let registry = Registry::new(SqliteStore::open(&path).unwrap());
    let named = registry.rename_swarm(id, &format!("  {name}  ")).unwrap();
    assert_eq!(named.name.as_deref(), Some(name));
    assert_eq!(registry.rename_swarm(id, name).unwrap(), named);
    drop(registry);
    let reopened = Registry::new(SqliteStore::open(&path).unwrap());
    assert_eq!(reopened.swarm(id).unwrap().unwrap(), named);
}

#[test]
fn independent_name_and_folder_writers_preserve_both_atomic_changes() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(&root.path().join("account")).unwrap());
    let id = Ulid::new();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writer = registry.clone();
    let first = barrier.clone();
    let rename = std::thread::spawn(move || {
        first.wait();
        writer.rename_swarm(id, "Ship the terminal").unwrap();
    });
    barrier.wait();
    registry
        .update_swarm_workdir(id, root.path().to_str())
        .unwrap();
    rename.join().unwrap();
    let record = registry.swarm(id).unwrap().unwrap();
    assert_eq!(record.name.as_deref(), Some("Ship the terminal"));
    assert_eq!(
        record.workdir.as_deref(),
        root.path().canonicalize().unwrap().to_str()
    );
    assert_eq!(record.revision, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_operator_swarm_api_replays_and_seat_tools_cannot_edit_metadata() {
    use bbt_mux::{
        LaunchSpec, Supervisor,
        identity::TestAccount,
        logic::{Logic, LogicConfig},
        mcp::Call,
    };
    use bbt_proto::{GuiAction, GuiReply, GuiRequest};
    use tokio::{
        io::{AsyncWriteExt, BufReader},
        net::{
            UnixStream,
            unix::{OwnedReadHalf, OwnedWriteHalf},
        },
    };
    async fn ask(
        read: &mut BufReader<OwnedReadHalf>,
        write: &mut OwnedWriteHalf,
        capability: [u8; 32],
        action: GuiAction,
    ) -> GuiReply {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability,
            action,
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                let line = bbt_mux::seat_socket::read_line(read)
                    .await
                    .unwrap()
                    .expect("metadata reply connection closed");
                let reply = serde_json::from_slice::<GuiReply>(&line).unwrap();
                if !matches!(reply, GuiReply::Identity(_)) {
                    return reply;
                }
            }
        })
        .await
        .expect("metadata reply deadline")
    }
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("run");
    let repository = root.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let account = TestAccount::open(&root.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(root.path())).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: repository.clone(),
            ring: Default::default(),
        },
        account.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let swarm = Ulid::new();
    let (read, mut write) = UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    let mut wrong = endpoint.capability;
    wrong[0] ^= 1;
    assert!(
        matches!(ask(&mut read, &mut write, wrong, GuiAction::SwarmRename {id:Ulid::new(),swarm,name:"not admitted".into()}).await, GuiReply::Error {reason,..} if reason.contains("admission rejected"))
    );
    drop((read, write));
    let (read, mut write) = UnixStream::connect(&endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    assert!(matches!(
        ask(
            &mut read,
            &mut write,
            endpoint.capability,
            GuiAction::SwarmGet {
                id: Ulid::new(),
                swarm
            }
        )
        .await,
        GuiReply::Swarm { record: None, .. }
    ));
    let op = Ulid::new();
    let action = GuiAction::SwarmRename {
        id: op,
        swarm,
        name: "  Ship the terminal  ".into(),
    };
    let first = ask(&mut read, &mut write, endpoint.capability, action.clone()).await;
    assert!(
        matches!(&first, GuiReply::Swarm {op:id,record:Some(record)} if *id==op && record.name.as_deref()==Some("Ship the terminal") && record.revision==1)
    );
    let changed = ask(
        &mut read,
        &mut write,
        endpoint.capability,
        GuiAction::SwarmWorkdir {
            id: Ulid::new(),
            swarm,
            path: repository.to_str().map(String::from),
        },
    )
    .await;
    assert!(
        matches!(changed, GuiReply::Swarm {record:Some(record),..} if record.revision==2 && record.workdir.as_deref()==repository.canonicalize().unwrap().to_str())
    );
    let replay = ask(&mut read, &mut write, endpoint.capability, action).await;
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(first).unwrap()
    );
    let list = ask(
        &mut read,
        &mut write,
        endpoint.capability,
        GuiAction::SwarmList { id: Ulid::new() },
    )
    .await;
    assert!(
        matches!(list, GuiReply::Swarms {records,..} if records.len()==1 && records[0].revision==2)
    );
    let invalid = ask(
        &mut read,
        &mut write,
        endpoint.capability,
        GuiAction::SwarmRename {
            id: Ulid::new(),
            swarm,
            name: "  ".into(),
        },
    )
    .await;
    assert!(matches!(invalid, GuiReply::Error {reason,..} if reason.contains("printable")));
    let spec = LaunchSpec::new("/bin/cat", root.path());
    let seat = spec.seat;
    logic
        .client
        .spawn(
            bbt_mux::registry::SeatRecord::new(seat, swarm, "private worker", "zsh"),
            spec,
        )
        .await
        .unwrap();
    let denied = logic
        .client
        .control_call(
            seat,
            1,
            Call {
                op_id: Ulid::new(),
                tool: "swarm_rename".into(),
                args: serde_json::json!({"swarm":swarm,"name":"seat injection"}),
            },
        )
        .await
        .unwrap();
    assert_eq!(denied["isError"], true);
    let retained = Registry::new(SqliteStore::open(&run.join("account")).unwrap())
        .swarm(swarm)
        .unwrap()
        .unwrap();
    assert_eq!(retained.name.as_deref(), Some("Ship the terminal"));
    assert_eq!(retained.revision, 2);
    drop((read, write));
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
