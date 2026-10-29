// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::{
    Supervisor,
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
};

fn fixture(root: &Path) -> (Supervisor, Logic) {
    let repository = root.join("repository");
    std::fs::create_dir(&repository).unwrap();
    let supervisor = Supervisor::start(crate::Config {
        journal_directory: Some(root.join("journal")),
        ..Default::default()
    })
    .unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[93; 32]),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    (supervisor, logic)
}
async fn reply(read: &mut BufReader<tokio::net::unix::OwnedReadHalf>) -> GuiReply {
    loop {
        let line = read_line(read)
            .await
            .unwrap()
            .expect("observer closed before reply");
        let reply: GuiReply = serde_json::from_slice(&line).unwrap();
        if !matches!(reply, GuiReply::Identity(_)) {
            return reply;
        }
    }
}
fn waiting() -> GuiSeat {
    GuiSeat {
        notification_episode: None,
        attention: None,
        record: SeatRecord::new(Ulid(99), Ulid(100), "current question", "claude"),
        state: bbt_proto::SeatState {
            generation: 1,
            process: bbt_proto::ProcessStatus::Running { pid: 9 },
            activity: bbt_proto::Activity::Waiting(bbt_proto::WaitKind::Question),
            grade: bbt_proto::Grade::Shim,
            title: String::new(),
            detail: Some("Which workspace?".into()),
            turn_open: true,
            observed_ms: 1,
            subagents: Vec::new(),
        },
    }
}
async fn connected_pair() -> (tokio::net::UnixStream, tokio::net::UnixStream) {
    // A real connect/accept records LOCAL_PEERPID on Darwin; socketpair does not.
    let root = tempfile::Builder::new()
        .prefix("bbt-observer-")
        .tempdir_in("/tmp")
        .unwrap();
    let path = root.path().join("observer.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let peer = tokio::net::UnixStream::connect(&path).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();
    (peer, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attention_observer_is_readonly_and_cannot_steal_events_or_capture() {
    tokio::time::timeout(Duration::from_secs(30),async {
        let root=tempfile::tempdir().unwrap();let (supervisor,logic)=fixture(root.path());
        logic.client.set_attention_observer_guard(Arc::new(|pid|pid==std::process::id()));
        let reference=logic.client.gui_device().await.unwrap();
        let lease=logic.client.bus.gui_attach(reference.id,vec!["seat/*/state".into()]).await.unwrap();
        let before=logic.client.bus.gui_cursor(&lease,"account").await.unwrap();
        for door in [crate::mcp::Door::Gui,crate::mcp::Door::Ctl] {
            let (peer,server)=connected_pair().await;let capability=[75;32];
            let task=tokio::spawn(gui_session(server,logic.client.clone(),capability,door));
            let (read,mut write)=peer.into_split();let mut read=BufReader::new(read);
            let hello=Ulid::new();
            send(&mut write,&GuiRequest {version:bbt_proto::gui::VERSION,capability,action:GuiAction::GuiHello {id:hello,claim:false,observer:true}}).await.unwrap();
            assert!(matches!(reply(&mut read).await,GuiReply::GuiHello {op,..} if op==hello));
            let op=Ulid::new();
            send(&mut write,&GuiRequest {version:bbt_proto::gui::VERSION,capability,action:GuiAction::ObserveAttention {id:op}}).await.unwrap();
            assert!(matches!(reply(&mut read).await,GuiReply::CurrentAttention {op:Some(id),update} if id==op&&update.snapshot&&update.seats.is_empty()));
            assert!(!logic.client.captures.attached(),"observer subscribed to window relays");
            for action in [
                GuiAction::AttachEvents {id:Ulid::new()},
                GuiAction::EventAck {stream:"account".into(),seq:1},
                GuiAction::MarkEventsRead {stream:"account".into(),seq:1},
                GuiAction::ListSeats,
                GuiAction::GuiHello {id:Ulid::new(),claim:true,observer:false},
                GuiAction::Input {seat:bbt_proto::SeatId::from_u128(99),generation:1,id:Ulid::new(),bytes:vec![b'x']},
            ] {
                send(&mut write,&GuiRequest {version:bbt_proto::gui::VERSION,capability,action}).await.unwrap();
                assert!(matches!(reply(&mut read).await,GuiReply::Error {reason,..} if reason=="attention observer is read-only"));
                assert!(!logic.client.captures.attached());
            }
            let row=waiting();logic.client.current_attention.update(row.clone(),true).unwrap();
            let GuiReply::CurrentAttention {op:None,update}=reply(&mut read).await else {panic!("missing current change")};
            assert!(!update.snapshot);assert_eq!(update.seats[0].notification_episode,logic.client.current_attention.episode(Ulid(99),1));
            let mut resolved=row;resolved.state.activity=bbt_proto::Activity::Active;
            logic.client.current_attention.update(resolved,false).unwrap();
            assert!(matches!(reply(&mut read).await,GuiReply::CurrentAttention {update,..} if update.removed==vec![Ulid(99)]));
            drop(read);drop(write);task.await.unwrap().unwrap();
        }
        let after=logic.client.bus.gui_cursor(&lease,"account").await.unwrap();
        assert_eq!((before.published,before.read),(after.published,after.read));
        logic.client.bus.gui_detach(&lease).await.unwrap();
        logic.shutdown().unwrap();supervisor.shutdown().unwrap();
    }).await.expect("observer controls or changes stalled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attention_observer_requires_the_current_recorded_peer_guard() {
    tokio::time::timeout(Duration::from_secs(30),async {
        let root=tempfile::tempdir().unwrap();let (supervisor,logic)=fixture(root.path());
        for configured in [false,true] {
            if configured {logic.client.set_attention_observer_guard(Arc::new(|_|false));}
            let (peer,server)=connected_pair().await;let capability=[76;32];
            let task=tokio::spawn(gui_session(server,logic.client.clone(),capability,crate::mcp::Door::Gui));
            let (read,mut write)=peer.into_split();let mut read=BufReader::new(read);
            send(&mut write,&GuiRequest {version:bbt_proto::gui::VERSION,capability,action:GuiAction::GuiHello {id:Ulid::new(),claim:false,observer:true}}).await.unwrap();
            assert!(matches!(reply(&mut read).await,GuiReply::Error {reason,..} if reason.contains("recorded desktop owner")));
            assert!(!logic.client.captures.attached());drop(read);drop(write);assert!(task.await.unwrap().is_err());
        }
        logic.shutdown().unwrap();supervisor.shutdown().unwrap();
    }).await.expect("observer admission stalled");
}
