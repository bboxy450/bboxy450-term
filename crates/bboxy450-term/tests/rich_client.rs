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
use bboxy450_term::rich_pane::{Input, Pane};
use bbt_ui_model::layout::Rect;
use serde_json::{Value, json};
use std::{
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
                "rich_actor_forwards_composer_stop_approval_and_detaches_without_close",
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
    std::fs::write(&script,r#"import json,socket,sys,os,threading
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
    send(peer,{'v':{'major':1,'minor':1},'meta':meta,'epoch':epoch,'head':head})
    for event in events[:]: deliver(peer,event,connection)
    for line in stream:
        op=json.loads(line); record(op)
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
            capture_frame(&mut view, &mut state, [1120, 820], None, false)
                .unwrap()
                .actions
                .clone()
        })
        .await
    });
    let center = |rect: Rect| [rect.x + rect.w / 2., rect.y + rect.h / 2.];
    let approval = actions
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
    drop(pane);
    let observed = rows(&journal);
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
            .map(|r| r["op_id"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(ops.iter().all(|r| r["op"]["type"] != "close"));
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
                "delayed_cancel_cannot_deny_a_new_approval",
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
