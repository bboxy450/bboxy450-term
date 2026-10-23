#![allow(
    clippy::disallowed_methods,
    reason = "owned headless GUI/PTY fixture, never app UI IO"
)]

use bboxy450_term::{
    cockpit::UiMsg,
    grids::Grids,
    messaging::{self, Client, Records, Reply, Results},
};
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    identity::{Custody, DeviceKeyStore, PendingReason},
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
};
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, SwarmId,
    messaging::{Body, MessageKind, Request, Response},
};
use bbt_ui::modules::{
    composer::{self, Address, Command, Controller, Outcome},
    stream_dock::Entry,
};
use ed25519_dalek::SigningKey;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
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
struct Fixture {
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    root: tempfile::TempDir,
    own: SeatRecord,
    scope: SwarmId,
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    capability: [u8; 32],
    task: Ulid,
    task_path: String,
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy/tasks")).unwrap();
        let own = SeatRecord::new(Ulid::new(), Ulid::new(), "Actual executor", "zsh");
        let task = Ulid::new();
        let task_path = format!(".bboxy/tasks/{task}.md");
        std::fs::write(repository.join(&task_path), format!("---\nid: {task}\ntype: task\ntitle: Owned UI fixture\nstatus: open\nverify: 'true'\n---\nDo the fixture work.\n")).unwrap();
        let supervisor = Supervisor::start(bbt_mux::Config {
            journal_directory: Some(root.path().join("supervisor-journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start_custodied_with_store(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: root.path().join("run"),
                repository,
                ring: Default::default(),
            },
            Arc::new(Keys::default()),
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let mut launch = LaunchSpec::new("/bin/zsh", root.path());
        launch.seat = own.id;
        launch.args = vec![
            "-f".into(),
            "-c".into(),
            "while IFS= read -r line; do :; done".into(),
        ];
        logic.client.spawn(own.clone(), launch).await.unwrap();
        let endpoint = logic.client.gui_endpoint().await.unwrap();
        let (read, write) = UnixStream::connect(&endpoint.path)
            .await
            .unwrap()
            .into_split();
        Self {
            scope: SwarmId::from_u128(own.swarm.0),
            own,
            logic: Some(logic),
            supervisor: Some(supervisor),
            root,
            reader: BufReader::new(read),
            writer: write,
            capability: endpoint.capability,
            task,
            task_path,
        }
    }
    fn logic(&self) -> &Logic {
        self.logic.as_ref().unwrap()
    }
    async fn action(&mut self, id: Ulid, action: GuiAction) -> GuiReply {
        let mut frame = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .unwrap();
        frame.push(b'\n');
        self.writer.write_all(&frame).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let bytes = bbt_mux::seat_socket::read_line(&mut self.reader).await.unwrap().expect("admitted GUI");
                let reply: GuiReply = serde_json::from_slice(&bytes).unwrap();
                if matches!(&reply, GuiReply::Messaging { op, .. } | GuiReply::Task { op, .. } | GuiReply::Error { op, .. } if *op==id) { return reply; }
            }
        }).await.unwrap()
    }
    async fn command(&mut self, command: UiMsg, results: &mut Results) -> Ulid {
        let (id, action) = match command {
            UiMsg::Messaging {
                id,
                swarm_id,
                request,
            } => (
                id,
                GuiAction::Messaging {
                    id,
                    swarm_id,
                    request,
                },
            ),
            UiMsg::Task { id, request } => (id, GuiAction::Task { id, request }),
            _ => panic!("typed composer request"),
        };
        results.requested(id);
        let reply = match self.action(id, action).await {
            GuiReply::Messaging { response, .. } => Reply::Messaging(response),
            GuiReply::Task { response, .. } => Reply::Task(response),
            GuiReply::Error { reason, .. } => Reply::Refused(reason),
            _ => unreachable!(),
        };
        assert!(results.complete(id, reply));
        id
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(logic) = self.logic.take() {
            logic.shutdown().unwrap();
        }
        if let Some(supervisor) = self.supervisor.take() {
            supervisor.shutdown().unwrap();
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn composed_decision_and_task_delegation_cross_the_real_gui_boundary() {
    let mut f = Fixture::new().await;
    let mut editor = Controller::new(composer::Composer {
        focus: true,
        kind: "decision".into(),
        to: vec![Address {
            kind: "channel".into(),
            label: "Misleading UI title".into(),
            destination: format!("swarms/{}", f.own.swarm),
            ..Default::default()
        }],
        ..Default::default()
    });
    editor.handle(Command::Edit(bbt_ui::text_edit::Command::Insert(
        "Operator decision through the composer".into(),
    )));
    let Outcome::Submit(draft) = editor.handle(Command::Send) else {
        panic!("committed draft");
    };
    let mut client = Client::default();
    let batch = client.prepare(Some(f.scope), draft).unwrap();
    let mut results = Results::default();
    let command = client.admitted(batch).pop().unwrap();
    let message_id = f.command(command, &mut results).await;
    let sent = client
        .replies(&results)
        .0
        .expect("only a correlated durable reply clears");
    editor.stored(&sent);
    assert!(editor.view.text.is_empty());
    let row = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let row = f
                .logic()
                .client
                .gui_message_row(f.own.swarm, message_id)
                .await
                .unwrap();
            if row.bus.is_some() {
                break row;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(row.kind, MessageKind::Decision);
    let mut history = messaging::History::default();
    history.select(f.scope);
    history.row(&row, Some(&format!("swarms/{}", f.own.swarm)), 1);
    assert!(history.entries(&Default::default(),client.groups(),&[]).iter().any(|entry| matches!(entry,Entry::Message {data,..} if data.text=="Operator decision through the composer" && data.kind.as_deref()==Some("decision") && data.issuer_kind.as_deref()==Some("local-dev"))));
    let refused = f
        .logic()
        .client
        .messaging(
            f.own.id,
            f.own.generation,
            Request::Send {
                message: bbt_proto::messaging::SendRequest::text(
                    format!("swarms/{}", f.own.swarm),
                    "ordinary progress",
                ),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("send progress as a DM"), "{refused}");

    let mut records = Records::default();
    records.select(f.scope);
    for command in records.commands() {
        f.command(command, &mut results).await;
    }
    records.replies(&results);
    assert!(
        records.tasks().is_empty(),
        "an unassigned task is not in a swarm inventory"
    );
    let unassigned = records
        .unassigned()
        .iter()
        .find(|card| card.id == f.task)
        .expect("operator read-only inventory");
    assert!(unassigned.assignee.is_none());
    let attachment = messaging::task_attachment(unassigned);
    let refused = f
        .logic()
        .client
        .control_call(
            f.own.id,
            f.own.generation,
            bbt_mux::mcp::Call {
                op_id: Ulid::new(),
                tool: "task_list".into(),
                args: serde_json::json!({"unassigned":true}),
            },
        )
        .await
        .unwrap();
    assert_eq!(refused["isError"], true);
    assert!(refused.to_string().contains("account operator"));
    editor.handle(Command::Delegate(attachment));
    editor.view.to = vec![Address {
        kind: "seat".into(),
        id: f.own.id.to_string(),
        label: "Actual executor".into(),
        destination: format!("seats/{}/inbox", f.own.id),
        ..Default::default()
    }];
    let Outcome::Submit(draft) = editor.handle(Command::Send) else {
        panic!("delegate snapshot");
    };
    let batch = client.prepare(Some(f.scope), draft).unwrap();
    let command = client.admitted(batch).pop().unwrap();
    f.command(command, &mut results).await;
    assert!(client.replies(&results).0.is_some());
    let text =
        std::fs::read_to_string(f.root.path().join("repository").join(&f.task_path)).unwrap();
    let document = bbt_mux::index::Document::parse(&f.task_path, &text);
    assert!(document.error.is_none(), "{document:?}");
    assert_eq!(document.metadata["status"], "claimed");
    assert!(
        document.metadata["delegated_by"]
            .as_str()
            .unwrap()
            .starts_with("operator:")
    );
    let Response::Messages { page } = f
        .logic()
        .client
        .messaging(
            f.own.id,
            f.own.generation,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 128,
            },
        )
        .await
        .unwrap()
    else {
        panic!("native task inbox");
    };
    assert!(page.items.iter().any(|item| matches!(&item.envelope.body,Body::Task {task_id,event,..} if *task_id==f.task && event=="delegate")));
    let task_message = page.items.iter().find(|item| matches!(&item.envelope.body, Body::Task { task_id, event, .. } if *task_id == f.task && event == "delegate")).unwrap().envelope.id;
    let receipt = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let row = f
                .logic()
                .client
                .gui_message_row(f.own.swarm, task_message)
                .await
                .unwrap();
            if row.receipts.delivered > 0 {
                break row;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        messaging::message_view(&receipt, &Default::default())
            .receipts
            .is_some_and(|r| r.delivered > 0)
    );
    records.refresh();
    for command in records.commands() {
        f.command(command, &mut results).await;
    }
    records.replies(&results);
    assert!(
        records.unassigned().is_empty(),
        "delegation removes it from the unscoped inventory"
    );
    let card = records
        .task(f.task)
        .expect("shared TaskCard read after delegation");
    let (task_row, verify) = messaging::task_row(card);
    assert_eq!(task_row.state, "claimed");
    assert_eq!(verify, None);
    assert_eq!(
        card.assignee.as_deref(),
        Some(format!("seat:{}", f.own.id).as_str())
    );
    assert!(records.entries(&bboxy450_term::model::build(std::slice::from_ref(&f.own),&Grids::new(),&Default::default()),None).iter().any(|entry|matches!(entry,Entry::Task {data,..} if data.id==f.task.to_string() && data.assignee=="Actual executor")));
    assert!(matches!(
        records.review(f.task, None),
        Err("task_requires_reported_zero_exit")
    ));
}
