// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The production rich actor against a separately scripted Unix process.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended process and replay fixture"
)]
use bboxy450_harness::frontends::gui::{
    bridge::FrameState,
    smoke::capture_frame,
    view::{Action, HarnessView},
};
use bboxy450_harness::protocol::{Epoch, Op, OpEnvelope, OpId, PROTOCOL, Seq, SessionId};
use bboxy450_term::rich_pane::{Input, Pane};
use bbt_ui_model::layout::Rect;
use serde_json::{Value, json};
use std::{
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
struct Wake;
impl bbt_rt::Waker for Wake {
    fn wake(&self) {}
}
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct PrivateRoot(Option<tempfile::TempDir>);
impl std::ops::Deref for PrivateRoot {
    type Target = tempfile::TempDir;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}
impl Drop for PrivateRoot {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "private rich fixture retained: {}",
                self.0.take().unwrap().keep().display()
            );
        }
    }
}
fn until(pane: &mut Pane, mut predicate: impl FnMut(&Pane) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        pane.drain();
        if predicate(pane) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "rich fixture timed out: error={:?} ready={} cursor={:?} generation={} completed={} notice={:?}",
            pane.snapshot.error,
            pane.snapshot.ready,
            pane.snapshot.cursor,
            pane.snapshot.generation,
            pane.snapshot.completed_inputs,
            pane.snapshot.notice
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn rows(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

// The scripted actor has two visible bindings, one abandoned fresh-cursor
// prefix probe, and one retained pending-turn binding after detach. These are
// fixture lifecycle bounds, not a protocol allowance for arbitrary query noise.
const ACTOR_BINDINGS: usize = 4;
const ACTOR_MUTATIONS: usize = 4;
const ACTOR_QUERY_BINDINGS: usize = 3;

fn bounded_actor_journal(path: &Path) -> String {
    let mut raw = String::new();
    std::fs::File::open(path)
        .unwrap()
        .take(64 * 1024 + 1)
        .read_to_string(&mut raw)
        .unwrap();
    assert!(raw.len() <= 64 * 1024, "bounded private actor journal");
    raw
}

fn actor_rows(path: &Path) -> Vec<Value> {
    bounded_actor_journal(path)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn validate_actor_traffic(
    observed: &[Value],
    cursor: (SessionId, Epoch, Seq),
    initial_head: u64,
) -> Result<(), String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut subscriptions = BTreeMap::new();
    let mut headers = BTreeMap::new();
    let mut closed = BTreeMap::new();
    for (index, row) in observed.iter().enumerate() {
        if row.get("subscribe").is_some() || row.get("hello").is_some() {
            let connection = row["connection"].as_u64().ok_or("missing connection")?;
            let target = if row.get("subscribe").is_some() {
                &mut subscriptions
            } else {
                &mut headers
            };
            if target.insert(connection, (index, row)).is_some() {
                return Err("duplicate connection receipt".into());
            }
        }
        if let Some(connection) = row.get("disconnected") {
            let connection = connection.as_u64().ok_or("invalid disconnection")?;
            if closed.insert(connection, index).is_some() {
                return Err("duplicate disconnection receipt".into());
            }
        }
    }
    if subscriptions.len() != ACTOR_BINDINGS || headers.len() != ACTOR_BINDINGS {
        return Err("expected four actual subscription/Hello bindings".into());
    }
    let after = serde_json::to_value((cursor.1, cursor.2)).unwrap();
    let mut roles = BTreeMap::new();
    let mut role_counts = BTreeMap::new();
    let mut initial = None;
    let mut probe = None;
    let mut rebuilt = None;
    for (&connection, &(index, row)) in &subscriptions {
        let sub = &row["subscribe"];
        if connection == 0
            || sub["v"] != serde_json::to_value(PROTOCOL).unwrap()
            || sub["subscribe"]["ephemeral"] != true
        {
            return Err("invalid subscription binding".into());
        }
        let role = match (
            sub["subscribe"]["after"].is_null(),
            sub["approver"].as_bool(),
        ) {
            (true, Some(true)) if connection == 1 => {
                initial = Some(connection);
                "initial"
            }
            (true, Some(true)) => {
                rebuilt = Some(connection);
                "rebuilt"
            }
            (false, Some(true)) if sub["subscribe"]["after"] == after => {
                probe = Some(connection);
                "probe"
            }
            (false, Some(false)) if sub["subscribe"]["after"] == after => "detached",
            _ => return Err("unexpected subscription/readiness choreography".into()),
        };
        *role_counts.entry(role).or_insert(0usize) += 1;
        roles.insert(connection, role);
        let &(hello_index, hello) = headers.get(&connection).ok_or("missing Hello receipt")?;
        if hello_index <= index
            || hello["hello"]["session"] != serde_json::to_value(cursor.0).unwrap()
            || hello["hello"]["epoch"] != serde_json::to_value(cursor.1).unwrap()
            || hello["hello"]["head"].as_u64()
                != Some(if role == "initial" {
                    initial_head
                } else {
                    cursor.2.0
                })
        {
            return Err("wrong live Hello binding".into());
        }
        match closed.get(&connection) {
            Some(closed_index) if *closed_index > hello_index && role != "detached" => {}
            None if role == "detached" => {}
            _ => return Err("unexpected connection retirement".into()),
        }
    }
    if role_counts
        != BTreeMap::from([
            ("initial", 1),
            ("probe", 1),
            ("rebuilt", 1),
            ("detached", 1),
        ])
        || probe.is_none_or(|probe| rebuilt.is_none_or(|rebuilt| probe >= rebuilt))
        || closed.len() != ACTOR_BINDINGS - 1
    {
        return Err("expected initial/probe/rebuilt/detached bindings".into());
    }
    let mut ids = BTreeSet::new();
    let mut mutation_ids = BTreeSet::new();
    let mut mutations = BTreeMap::new();
    let mut query_bindings = BTreeSet::new();
    for (index, row) in observed
        .iter()
        .enumerate()
        .filter(|(_, r)| r.get("op").is_some())
    {
        let mut wire = row.as_object().ok_or("operation is not an object")?.clone();
        let connection = wire
            .remove("connection")
            .and_then(|value| value.as_u64())
            .ok_or("operation lacks connection")?;
        if connection == 0
            || !headers.contains_key(&connection)
            || wire.len() != 4
            || !wire
                .keys()
                .all(|key| matches!(key.as_str(), "v" | "op_id" | "session" | "op"))
        {
            return Err("operation has unverified fixture metadata".into());
        }
        // Connection is our verified process receipt, not a public DTO field.
        // Deserialize only the original four-field wire envelope.
        let envelope: OpEnvelope = serde_json::from_value(Value::Object(wire))
            .map_err(|_| "unknown operation or malformed typed envelope")?;
        if envelope.op_id.0.to_bytes() == [0; 16]
            || row["op_id"].as_str() != Some(envelope.op_id.to_string().as_str())
            || !ids.insert(envelope.op_id)
        {
            return Err("invalid or duplicate operation ID".into());
        }
        if envelope.session != cursor.0 || envelope.v != PROTOCOL {
            return Err("wrong session or operation protocol".into());
        }
        let &(hello_index, _) = headers
            .get(&connection)
            .ok_or("operation lacks live Hello")?;
        if index <= hello_index
            || closed
                .get(&connection)
                .is_some_and(|closed| index >= *closed)
        {
            return Err("operation outside live binding".into());
        }
        let kind = match envelope.op {
            Op::QueryInputCapabilities { request } => {
                if request != envelope.op_id
                    || request.0.to_bytes() == [0; 16]
                    || row["op"]["request"].as_str() != Some(request.to_string().as_str())
                    || row["op"].as_object().is_none_or(|op| op.len() != 2)
                {
                    return Err("query request does not match operation ID".into());
                }
                if roles.get(&connection) == Some(&"probe") {
                    return Err("query belongs to abandoned probe binding".into());
                }
                if !query_bindings.insert(connection) {
                    return Err("duplicate query on live binding".into());
                }
                continue;
            }
            Op::UserTurn { .. } => "user_turn",
            Op::Interrupt { .. } => "interrupt",
            Op::ResolveApproval { .. } => "resolve_approval",
            Op::SetProfile { .. } => "set_profile",
            _ => return Err("unknown operation for this fixture, including Close".into()),
        };
        if Some(connection) != initial {
            return Err("mutation escaped original visible binding".into());
        }
        mutation_ids.insert(envelope.op_id);
        *mutations.entry(kind).or_insert(0usize) += 1;
    }
    let expected_queries = roles
        .iter()
        .filter_map(|(&connection, &role)| (role != "probe").then_some(connection))
        .collect::<BTreeSet<_>>();
    if mutations
        != BTreeMap::from([
            ("user_turn", 1),
            ("interrupt", 1),
            ("resolve_approval", 1),
            ("set_profile", 1),
        ])
        || mutation_ids.len() != ACTOR_MUTATIONS
        || query_bindings != expected_queries
        || query_bindings.len() != ACTOR_QUERY_BINDINGS
        || ids.len() != mutation_ids.len() + query_bindings.len()
    {
        return Err("mutation/query count or binding mismatch".into());
    }
    Ok(())
}

#[test]
fn rich_actor_forwards_composer_stop_approval_and_detaches_without_close() {
    if std::env::var_os("BBOXY_RICH_ACTOR_PRIVATE").is_none() {
        let home = tempfile::Builder::new()
            .prefix("ractor-home-")
            .tempdir_in("/tmp")
            .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "rich_client::rich_actor_forwards_composer_stop_approval_and_detaches_without_close",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .env("BBOXY_RICH_ACTOR_PRIVATE", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "private actor child {}:\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        return;
    }
    let root = PrivateRoot(Some(
        tempfile::Builder::new()
            .prefix("rface-")
            .tempdir_in("/tmp")
            .unwrap(),
    ));
    let socket = root.path().join("s.sock");
    let journal = root.path().join("ops.jsonl");
    let replay = root.path().join("prefix.jsonl");
    let prefix = approval_prefix();
    std::fs::write(
        &replay,
        prefix.iter().map(|e| format!("{e}\n")).collect::<String>(),
    )
    .unwrap();
    let script = root.path().join("peer.py");
    std::fs::write(&script,r#"# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; keep the credit attached.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Private protocol peer; the pinned harness protocol remains the DTO authority.
import json,socket,sys,os,threading
from pathlib import Path
address,replay,journal=map(Path,sys.argv[1:])
events=[json.loads(x) for x in replay.read_text().splitlines()]
meta=events[0]['event']['meta']; epoch=events[0]['epoch']; head=max(e['seq'] for e in events if e.get('seq') is not None)
approval_row=next(e['event'] for e in events if e['event']['type']=='item_snapshot' and e['event']['partial']['kind']=='approval')
approval_body=dict(approval_row['partial'])
listener=socket.socket(socket.AF_UNIX); listener.bind(str(address)); os.chmod(address,0o600); listener.listen()
def record(value):
    with journal.open('a') as f: f.write(json.dumps(value)+'\n'); f.flush()
def send(peer,value): peer.sendall((json.dumps(value)+'\n').encode())
def deliver(peer,event,connection):
    send(peer,event)
    if event.get('seq') is not None: record({'connection':connection,'seq':event['seq']})
def serve(peer,connection):
    global head
    stream=peer.makefile('rb'); sub=json.loads(stream.readline()); record({'subscribe':sub,'connection':connection})
    hello={'v':{'major':1,'minor':1},'meta':meta,'epoch':epoch,'head':head}
    send(peer,hello)
    record({'connection':connection,'hello':{'session':hello['meta']['session'],'epoch':hello['epoch'],'head':hello['head']}})
    for event in events[:]: deliver(peer,event,connection)
    for line in stream:
        op=json.loads(line); record(dict(op,connection=connection))
        send(peer,{'op_id':op['op_id'],'ack':{'op_id':op['op_id'],'duplicate':False}})
        if op['op']['type']=='resolve_approval':
            body=dict(approval_body);body['decision']=op['op']['decision'];body['by']={'by':'face','name':'fixture'}
            head+=1
            event={'v':{'major':1,'minor':1},'session':meta['session'],'epoch':epoch,'seq':head,'at':head,'ts_ms':head,'cause':op['op_id'],'event':{'type':'item_completed','item':approval_row['item'],'body':body,'status':{'status':'completed'}}}
            events.append(event);deliver(peer,event,connection)
connection=0
def connected(peer,connection):
    try:
        with peer: serve(peer,connection)
    except (BrokenPipeError,ConnectionResetError): pass
    finally: record({'disconnected':connection})
while True:
    peer,_=listener.accept(); connection+=1
    threading.Thread(target=connected,args=(peer,connection),daemon=True).start()
"#).unwrap();
    let child = std::process::Command::new("/usr/bin/python3")
        .arg("-u")
        .arg(&script)
        .args([socket.as_os_str(), replay.as_os_str(), journal.as_os_str()])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap();
    let child = Child(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let actions = bbt_rt::handle().block_on(async {
        // A live prefix intentionally has an unfinished turn; replay-file audit
        // requires completed turns. Feed the same Envelope reducer as the socket.
        let mut transcript = bboxy450_harness::frontends::gui::model::Transcript::default();
        for event in &prefix {
            let envelope =
                bboxy450_harness::protocol::Envelope::from_line(&format!("{event}\n")).unwrap();
            transcript.apply(&envelope);
        }
        bboxy450_harness::runtime::blocking(move || {
            let mut state = FrameState {
                transcript: Arc::new(transcript),
                ready: true,
                ..Default::default()
            };
            let mut view = HarnessView::default();
            capture_frame(&mut view, &mut state, &{
                let mut options =
                    bboxy450_harness::frontends::gui::options::Options::parse(Vec::<String>::new())
                        .unwrap();
                options.capture = Some([1120, 820]);
                options.no_chrome = true;
                options
            })
            .unwrap()
            .actions
            .clone()
        })
        .await
    });
    let center = |rect: Rect| [rect.x + rect.w / 2., rect.y + rect.h / 2.];
    let reference_approval = actions
        .iter()
        .find(|(_, a)| matches!(a, Action::Approval(..)))
        .map(|(r, _)| center(*r))
        .expect("actionable shared approval");
    let mut pane = Pane::connect(
        socket.clone(),
        1,
        Rect::new(0., 0., 1120., 820.),
        None,
        Arc::new(Wake),
    );
    until(&mut pane, |p| {
        p.snapshot.ready && p.snapshot.cursor.is_some_and(|(_, _, seq)| seq.0 == 18)
    });
    until(&mut pane, |_| {
        actor_rows(&journal)
            .iter()
            .any(|r| r["connection"] == 1 && r["op"]["type"] == "query_input_capabilities")
    });
    until(&mut pane, |p| !p.snapshot.approval_choices.is_empty());
    assert!(pane.snapshot.approval_choices.len() <= 16);
    let live_approval = pane.snapshot.approval_choices[0];
    assert!(reference_approval.iter().all(|value| value.is_finite()));
    assert!(live_approval.iter().all(|value| value.is_finite()));
    let [x, y, w, h] = live_approval;
    assert!(
        w > 0. && h > 0. && x >= 0. && y >= 0. && x + w <= 1120. && y + h <= 820.,
        "live approval target must be a positive bounded viewport rectangle"
    );
    let approval = [x + w / 2., y + h / 2.];
    eprintln!(
        "rich-client approval geometry reference_center={reference_approval:?} live_rect={live_approval:?} live_center={approval:?} changed={}",
        reference_approval != approval
    );
    assert!(pane.send(Input::Click(approval)));
    until(&mut pane, |p| {
        p.snapshot.cursor.is_some_and(|(_, _, seq)| seq.0 > 18)
    });
    assert!(pane.send(Input::Edit(bbt_ui::text_edit::Command::Insert(
        "private rich composer".into()
    ))));
    assert!(pane.send(Input::Enter));
    until(&mut pane, |_| {
        rows(&journal)
            .iter()
            .any(|r| r["op"]["type"] == "user_turn")
    });
    let stop = center(
        pane.snapshot
            .send_rect
            .expect("shared composer send/Stop cell"),
    );
    assert!(pane.send(Input::Click(stop)));
    until(&mut pane, |_| {
        rows(&journal)
            .iter()
            .any(|r| r["op"]["type"] == "interrupt")
    });
    assert!(pane.send(Input::Edit(bbt_ui::text_edit::Command::Insert(
        "/profile other".into()
    ))));
    assert!(pane.send(Input::Enter));
    until(&mut pane, |_| {
        rows(&journal)
            .iter()
            .any(|r| r["op"]["type"] == "set_profile")
    });
    let cursor = pane.snapshot.cursor;
    let item_count = pane.snapshot.retained_items;
    assert_eq!(pane.snapshot.order_len, item_count);
    assert_eq!(pane.snapshot.duplicate_durable, 0);
    drop(pane);
    let mut pane = Pane::connect(
        socket,
        1,
        Rect::new(0., 0., 1120., 820.),
        cursor.map(|(_, e, s)| (e, s)),
        Arc::new(Wake),
    );
    until(&mut pane, |p| {
        p.snapshot.ready && p.snapshot.cursor == cursor
    });
    assert_eq!(pane.snapshot.retained_items, item_count);
    assert_eq!(pane.snapshot.order_len, item_count);
    assert_eq!(pane.snapshot.duplicate_durable, 0);
    until(&mut pane, |_| {
        let observed = actor_rows(&journal);
        observed.iter().any(|binding| {
            binding["connection"] != 1
                && binding["subscribe"]["approver"] == true
                && binding["subscribe"]["subscribe"]["after"].is_null()
                && observed.iter().any(|r| {
                    r["connection"] == binding["connection"]
                        && r["op"]["type"] == "query_input_capabilities"
                })
        })
    });
    drop(pane);
    // Await actual connection/query/retirement receipts, not a sleep or a
    // hard-coded total-ID relaxation. The retained old turn owns the sole live
    // detached binding; the initial, cursor probe and rebuilt view have closed.
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let observed = actor_rows(&journal);
        let bindings = observed
            .iter()
            .filter(|r| r.get("subscribe").is_some())
            .count();
        let headers = observed.iter().filter(|r| r.get("hello").is_some()).count();
        let closed = observed
            .iter()
            .filter(|r| r.get("disconnected").is_some())
            .count();
        let operations = observed.iter().filter(|r| r.get("op").is_some()).count();
        assert!(
            bindings <= ACTOR_BINDINGS
                && headers <= ACTOR_BINDINGS
                && closed < ACTOR_BINDINGS
                && operations <= ACTOR_MUTATIONS + ACTOR_QUERY_BINDINGS,
            "unexpected actor traffic counts: bindings={bindings} headers={headers} closed={closed} ops={operations}"
        );
        if bindings == ACTOR_BINDINGS
            && headers == ACTOR_BINDINGS
            && closed == ACTOR_BINDINGS - 1
            && operations == ACTOR_MUTATIONS + ACTOR_QUERY_BINDINGS
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "actor binding receipts timed out: bindings={bindings} headers={headers} closed={closed} ops={operations}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let raw = bounded_actor_journal(&journal);
    assert!(
        raw.len() <= 64 * 1024 && raw.ends_with('\n'),
        "bounded complete private journal"
    );
    let observed = raw
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line)
                .expect("every private actor journal row must decode")
        })
        .collect::<Vec<_>>();
    let full_cursor = cursor.expect("ready actor has full session/epoch/sequence cursor");
    let initial_head = prefix
        .iter()
        .filter_map(|e| e["seq"].as_u64())
        .max()
        .unwrap();
    validate_actor_traffic(&observed, full_cursor, initial_head).unwrap();
    let mut seqs = std::collections::BTreeMap::<u64, Vec<u64>>::new();
    for row in &observed {
        if let (Some(connection), Some(seq)) = (row["connection"].as_u64(), row["seq"].as_u64()) {
            seqs.entry(connection).or_default().push(seq);
        }
    }
    let expected: Vec<_> = (1..=cursor.unwrap().2.0).collect();
    assert!(
        seqs.values().filter(|seq| **seq == expected).count() >= 2,
        "detach/reattach full durable sequence: {seqs:?}"
    );
    let ops: Vec<_> = observed.iter().filter(|r| r.get("op").is_some()).collect();
    for kind in ["user_turn", "interrupt", "resolve_approval", "set_profile"] {
        assert_eq!(
            ops.iter().filter(|r| r["op"]["type"] == kind).count(),
            1,
            "{kind}: {ops:?}"
        );
    }
    let turn = ops.iter().find(|r| r["op"]["type"] == "user_turn").unwrap();
    assert_eq!(turn["op"]["blocks"][0]["text"], "private rich composer");
    assert_eq!(turn["op"]["mode"], "queue");
    assert_eq!(
        ops.iter()
            .filter(|r| r["op"]["type"] != "query_input_capabilities")
            .map(|r| r["op_id"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        ACTOR_MUTATIONS
    );
    assert_eq!(
        ops.iter()
            .map(|r| r["op_id"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        ops.len()
    );
    assert!(ops.iter().all(|r| r["op"]["type"] != "close"));
    // Mutate the actual observed records only: these are validator controls,
    // never another canonical protocol fixture or an extra product operation.
    let query_index = observed
        .iter()
        .position(|r| r["op"]["type"] == "query_input_capabilities")
        .unwrap();
    let mutation_id = turn["op_id"].clone();
    let mut unknown = observed.clone();
    unknown[query_index]["op"]["type"] = json!("unexpected_fixture_mutation");
    assert!(
        validate_actor_traffic(&unknown, full_cursor, initial_head)
            .unwrap_err()
            .contains("unknown operation")
    );
    let mut duplicate = observed.clone();
    duplicate[query_index]["op_id"] = mutation_id.clone();
    duplicate[query_index]["op"]["request"] = mutation_id;
    assert!(
        validate_actor_traffic(&duplicate, full_cursor, initial_head)
            .unwrap_err()
            .contains("duplicate operation ID")
    );
    let mut misbound = observed.clone();
    misbound[query_index]["op"]["request"] = json!(OpId::new());
    assert!(
        validate_actor_traffic(&misbound, full_cursor, initial_head)
            .unwrap_err()
            .contains("query request does not match")
    );
    let mut foreign = observed.clone();
    foreign[query_index]["session"] = json!(SessionId::new());
    assert!(
        validate_actor_traffic(&foreign, full_cursor, initial_head)
            .unwrap_err()
            .contains("wrong session")
    );
    let mut zero = observed.clone();
    zero[query_index]["op_id"] = json!("00000000000000000000000000");
    zero[query_index]["op"]["request"] = zero[query_index]["op_id"].clone();
    assert!(
        validate_actor_traffic(&zero, full_cursor, initial_head)
            .unwrap_err()
            .contains("invalid or duplicate operation ID")
    );
    let mut repeat = observed.clone();
    let mut repeated_query = repeat[query_index].clone();
    let fresh = json!(OpId::new());
    repeated_query["op_id"] = fresh.clone();
    repeated_query["op"]["request"] = fresh;
    repeat.insert(query_index + 1, repeated_query);
    assert!(
        validate_actor_traffic(&repeat, full_cursor, initial_head)
            .unwrap_err()
            .contains("duplicate query on live binding")
    );
    let probe_connection = observed
        .iter()
        .find(|r| {
            r["subscribe"]["approver"] == true && r["subscribe"]["subscribe"]["after"].is_array()
        })
        .unwrap()["connection"]
        .clone();
    let probe_hello = observed
        .iter()
        .position(|r| r.get("hello").is_some() && r["connection"] == probe_connection)
        .unwrap();
    let mut probe_query = observed[query_index].clone();
    let fresh = json!(OpId::new());
    probe_query["op_id"] = fresh.clone();
    probe_query["op"]["request"] = fresh;
    probe_query["connection"] = probe_connection;
    let mut abandoned = observed.clone();
    abandoned.insert(probe_hello + 1, probe_query);
    assert!(
        validate_actor_traffic(&abandoned, full_cursor, initial_head)
            .unwrap_err()
            .contains("query belongs to abandoned probe binding")
    );
    assert!(
        observed
            .iter()
            .any(|r| !r["subscribe"]["subscribe"]["after"].is_null())
    );
    // SAFETY: this is exactly the fixture child PID returned by spawn above.
    assert!(
        unsafe { libc::kill(child.0.id() as i32, 0) } == 0,
        "dropping views must leave their session process alive"
    );
    eprintln!(
        "rich-client: composer=UserTurn, Stop=Interrupt, approval=ResolveApproval, profile=SetProfile; reattach cursor retained; no Close; session PID {} survives detach",
        child.0.id()
    );
}

fn approval_prefix() -> Vec<Value> {
    // Adapted fixture prefix from bboxy450-harness tests/fixtures/ir-v1/v0.4-all-items.jsonl:
    // turn stays open and its approval snapshot stays actionable.
    // https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bboxy450-harness/tests/fixtures/ir-v1/v0.4-all-items.jsonl");
    let mut prefix = Vec::new();
    for line in std::fs::read_to_string(corpus).unwrap().lines() {
        let mut event: Value = serde_json::from_str(line).unwrap();
        if event["event"]["type"] == "item_completed"
            && event["event"]["body"]["kind"] == "approval"
        {
            let mut body = event["event"]["body"].clone();
            body["decision"] = Value::Null;
            body["by"] = Value::Null;
            event["event"] =
                json!({"type":"item_snapshot","item":event["event"]["item"],"partial":body});
            prefix.push(event);
            break;
        }
        prefix.push(event);
    }
    assert_eq!(prefix.last().unwrap()["event"]["type"], "item_snapshot");
    let last = prefix.last().unwrap();
    prefix.push(json!({"v":{"major":1,"minor":1},"session":last["session"],"epoch":last["epoch"],"seq":null,"at":last["seq"],"ts_ms":22,"cause":null,"event":{"type":"status","phase":{"phase":"thinking"}}}));
    prefix
}

#[test]
fn delayed_cancel_cannot_deny_a_new_approval() {
    if std::env::var_os("BBOXY_RICH_ACTOR_PRIVATE").is_none() {
        let home = tempfile::Builder::new()
            .prefix("rcancel-home-")
            .tempdir_in("/tmp")
            .unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "rich_client::delayed_cancel_cannot_deny_a_new_approval",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .env("BBOXY_RICH_ACTOR_PRIVATE", "1");
        if let Some(case) = std::env::var_os("BBOXY_RICH_CANCEL_CASE") {
            child.env("BBOXY_RICH_CANCEL_CASE", case);
        }
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "private cancel child {}:\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        return;
    }
    for case in ["session", "epoch", "generation"] {
        if std::env::var("BBOXY_RICH_CANCEL_CASE").is_ok_and(|only| only != case) {
            continue;
        }
        delayed_cancel(case);
    }
}
fn delayed_cancel(case: &str) {
    let root = PrivateRoot(Some(
        tempfile::Builder::new()
            .prefix("rcancel-")
            .tempdir_in("/tmp")
            .unwrap(),
    ));
    let socket = root.path().join("s.sock");
    let journal = root.path().join("ops.jsonl");
    let replay = root.path().join("prefix.jsonl");
    let variant = root.path().join("variant.json");
    let prefix = approval_prefix();
    let session = prefix[0]["session"].clone();
    let epoch = prefix[0]["epoch"].clone();
    let approval = prefix
        .iter()
        .find(|e| {
            e["event"]["type"] == "item_snapshot" && e["event"]["partial"]["kind"] == "approval"
        })
        .unwrap()["event"]["item"]
        .clone();
    std::fs::write(
        &replay,
        prefix.iter().map(|e| format!("{e}\n")).collect::<String>(),
    )
    .unwrap();
    std::fs::write(
        &variant,
        json!({"session":session,"epoch":epoch,"approval":approval}).to_string(),
    )
    .unwrap();
    let script = root.path().join("peer.py");
    std::fs::write(&script, r#"import json,socket,sys,os,select,threading
from pathlib import Path
address,replay,journal,variant=map(Path,sys.argv[1:])
base=[json.loads(x) for x in replay.read_text().splitlines()]
old_approval=next(e['event']['item'] for e in base if e['event']['type']=='item_snapshot' and e['event']['partial']['kind']=='approval')
listener=socket.socket(socket.AF_UNIX);listener.bind(str(address));os.chmod(address,0o600);listener.listen()
def record(value):
    with journal.open('a') as f:f.write(json.dumps(value)+'\n');f.flush()
def send(peer,value):peer.sendall((json.dumps(value)+'\n').encode())
def replace(value,before,after):
    if isinstance(value,dict):return {k:replace(v,before,after) for k,v in value.items()}
    if isinstance(value,list):return [replace(v,before,after) for v in value]
    return after if value==before else value
def serve(peer):
    current=variant.read_text();v=json.loads(current)
    events=replace(base,old_approval,v['approval'])
    for e in events:
        e['session']=v['session'];e['epoch']=v['epoch']
        if e['event']['type'] in ('session_started','session_created'):e['event']['meta']['session']=v['session']
    head=max(e['seq'] for e in events if e.get('seq') is not None)
    try:
        with peer,peer.makefile('rb') as stream:
            sub=json.loads(stream.readline());record({'subscribe':sub,'variant':v})
            send(peer,{'v':{'major':1,'minor':1},'meta':events[0]['event']['meta'],'epoch':v['epoch'],'head':head})
            after=sub['subscribe'].get('after')
            if after is not None and after[0]!=v['epoch']:
                send(peer,{'v':{'major':1,'minor':1},'session':v['session'],'epoch':v['epoch'],'seq':None,'at':head,'ts_ms':head,'cause':None,'event':{'type':'reset','epoch':v['epoch'],'head':head}})
                return
            for event in events:send(peer,event)
            while variant.read_text()==current:
                if not select.select([peer],[],[],0.01)[0]:continue
                line=stream.readline()
                if not line:break
                op=json.loads(line);record(op)
                send(peer,{'op_id':op['op_id'],'ack':{'op_id':op['op_id'],'duplicate':False}})
    except (BrokenPipeError,ConnectionResetError):pass
while True:
    peer,_=listener.accept()
    threading.Thread(target=serve,args=(peer,),daemon=True).start()
"#).unwrap();
    let child = std::process::Command::new("/usr/bin/python3")
        .arg("-u")
        .arg(&script)
        .args([
            socket.as_os_str(),
            replay.as_os_str(),
            journal.as_os_str(),
            variant.as_os_str(),
        ])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap();
    let _child = Child(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut pane = Pane::connect(
        socket.clone(),
        1,
        Rect::new(0., 0., 1120., 820.),
        None,
        Arc::new(Wake),
    );
    until(&mut pane, |p| {
        p.snapshot.ready && p.snapshot.cursor.is_some_and(|(_, _, seq)| seq.0 == 18)
    });
    let captured = pane.snapshot.clone();
    let (new_session, new_epoch, new_approval) = match case {
        "session" => (
            json!("0000000000000000000000000B"),
            epoch.clone(),
            json!("0000000000000000000000000Y"),
        ),
        "epoch" => (
            session.clone(),
            json!(epoch.as_u64().unwrap() + 1),
            json!("0000000000000000000000000Y"),
        ),
        _ => (session.clone(), epoch.clone(), approval.clone()),
    };
    if case == "generation" {
        drop(pane);
        pane = Pane::connect(
            socket,
            2,
            Rect::new(0., 0., 1120., 820.),
            None,
            Arc::new(Wake),
        );
    } else {
        let next = root.path().join("variant.next");
        std::fs::write(
            &next,
            json!({"session":new_session,"epoch":new_epoch,"approval":new_approval}).to_string(),
        )
        .unwrap();
        std::fs::rename(next, &variant).unwrap();
    }
    until(&mut pane, |p| {
        p.snapshot.ready
            && p.snapshot.cursor.is_some_and(|(s, e, seq)| {
                json!(s) == new_session && json!(e) == new_epoch && seq.0 == 18
            })
            && p.snapshot.generation == if case == "generation" { 2 } else { 1 }
    });
    let completed = pane.snapshot.completed_inputs;
    // Reproduce an event captured from displayed A, delivered after the worker
    // has projected B. The stamp comes from A's actual production snapshot.
    pane.snapshot = captured;
    assert!(pane.send(Input::Cancel));
    until(&mut pane, |p| {
        p.snapshot.completed_inputs > completed
            && (p.snapshot.notice.as_deref()
                == Some("Session changed or unavailable; wait for its frame before submitting")
                || rows(&journal)
                    .iter()
                    .any(|r| r["op"]["type"] == "resolve_approval"))
    });
    assert_eq!(
        rows(&journal)
            .iter()
            .filter(|r| r["op"]["type"] == "resolve_approval")
            .count(),
        0,
        "stale cancel crossed identity fence: {case}"
    );
    // Positive calibration: the current B snapshot still cancels its approval.
    assert!(pane.send(Input::Cancel));
    until(&mut pane, |_| {
        rows(&journal)
            .iter()
            .any(|r| r["op"]["type"] == "resolve_approval")
    });
    let observed = rows(&journal);
    let denials: Vec<_> = observed
        .iter()
        .filter(|r| r["op"]["type"] == "resolve_approval")
        .collect();
    assert_eq!(
        denials.len(),
        1,
        "current approval must be denied exactly once"
    );
    assert_eq!(denials[0]["session"], new_session);
    assert_eq!(denials[0]["op"]["approval"], new_approval);
    assert_eq!(denials[0]["op"]["decision"]["decision"], "deny");
    eprintln!(
        "rich-cancel: {case}: old snapshot emitted zero ResolveApproval; current snapshot denied its pending approval once"
    );
}
