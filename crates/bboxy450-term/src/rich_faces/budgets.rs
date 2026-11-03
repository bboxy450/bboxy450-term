// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Actual four-actor App drain/composition; never a whole-frame budget verdict.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended benchmark setup/observation outside measured callbacks"
)]
use super::*;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
};
const NAME: &str = "rich_faces::budget_verification::rich_four_view_aggregate_budget";
struct Peer(Child);
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait(mut ready: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(30);
    while !ready() {
        assert!(
            Instant::now() < until,
            "four-view fixture did not make progress"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn p99(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[(samples.len() * 99).div_ceil(100) - 1]
}
fn pending(app: &App, targets: &[(SeatId, u64)]) -> bool {
    targets
        .iter()
        .any(|(id, n)| app.rich_panes[id].snapshot.completed_inputs < *n)
}
fn publish_four(app: &App, focused: bool) -> Vec<(SeatId, u64)> {
    let targets: Vec<_> = app
        .rich_panes
        .iter()
        .map(|(id, pane)| {
            assert!(pane.send(Input::Focus(focused)));
            (*id, pane.snapshot.completed_inputs + 1)
        })
        .collect();
    // This observes Release after Latest.publish, without taking its contents.
    wait(|| {
        targets
            .iter()
            .all(|(id, n)| app.rich_panes[id].published_inputs() >= *n)
    });
    targets
}
fn clock_frame(app: &mut App) {
    let mut micros = 0;
    app.rich_drain_with_clock(|| {
        let at = Duration::from_micros(micros);
        micros += 100;
        at
    });
}
#[test]
#[ignore = "private four-view benchmark; make bench-budgets"]
fn rich_four_view_aggregate_budget() {
    const CHILD: &str = "BBOXY_RICH_BUDGET_PRIVATE";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::Builder::new()
            .prefix("rich-budget-home-")
            .tempdir_in("/tmp")
            .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                NAME,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .env(CHILD, "1");
        for name in ["BBOXY_HARNESS_CANDIDATE", "BBOXY_RICH_BUDGET_OUT"] {
            if let Some(value) = std::env::var_os(name) {
                child.env(name, value);
            }
        }
        let result = child.output().unwrap();
        assert!(
            result.status.success(),
            "private budget child {}:\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        print!("{}", String::from_utf8_lossy(&result.stdout));
        eprint!("{}", String::from_utf8_lossy(&result.stderr));
        return;
    }
    let root = tempfile::Builder::new()
        .prefix("rich-four-")
        .tempdir_in("/tmp")
        .unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bboxy450-harness/tests/fixtures/ir-v1/v1.1-all-items.jsonl");
    let rows: Vec<Value> = std::fs::read_to_string(&fixture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|row: &Value| row["event"]["type"] != "session_closed")
        .collect();
    let head = rows.iter().filter_map(|r| r["seq"].as_u64()).max().unwrap();
    let expected_items = rows
        .iter()
        .filter(|r| r["event"]["type"] == "item_started")
        .count();
    let replay = root.path().join("live.jsonl");
    std::fs::write(
        &replay,
        rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
    )
    .unwrap();
    let script = root.path().join("peers.py");
    // Adapted from this project's tests/rich_client.rs private Unix fixture.
    // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    std::fs::write(&script,r#"import json,socket,threading,sys,os
from pathlib import Path
root,replay=map(Path,sys.argv[1:])
base=[json.loads(x) for x in replay.read_text().splitlines()]
def serve(peer,session):
    events=json.loads(json.dumps(base))
    for e in events:
        e['session']=session
        if e['event']['type'] in ('session_started','session_created'):e['event']['meta']['session']=session
    head=max(e['seq'] for e in events if e.get('seq') is not None)
    try:
        with peer,peer.makefile('rb') as stream:
            sub=json.loads(stream.readline())
            peer.sendall((json.dumps({'v':{'major':1,'minor':1},'meta':events[0]['event']['meta'],'epoch':1,'head':head})+'\n').encode())
            for e in events:peer.sendall((json.dumps(e)+'\n').encode())
            for line in stream:
                op=json.loads(line)
                peer.sendall((json.dumps({'op_id':op['op_id'],'ack':{'op_id':op['op_id'],'duplicate':False}})+'\n').encode())
    except (BrokenPipeError,ConnectionResetError):pass
def listen(i):
    listener=socket.socket(socket.AF_UNIX);address=root/(str(i)+'.sock');listener.bind(str(address));os.chmod(address,0o600);listener.listen()
    while True:
        peer,_=listener.accept()
        threading.Thread(target=serve,args=(peer,'0'*25+str(i+1)),daemon=True).start()
for i in range(4):threading.Thread(target=listen,args=(i,),daemon=True).start()
threading.Event().wait()
"#).unwrap();
    let peer = Peer(
        Command::new("/usr/bin/python3")
            .arg("-u")
            .arg(&script)
            .arg(root.path())
            .arg(&replay)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait(|| (0..4).all(|i| root.path().join(format!("{i}.sock")).exists()));
    let (mut app, _) = crate::lazy_projection_tests::fixture();
    app.document.open = false;
    app.local_tab = None;
    let mut model = bboxy450_term::model::Model::default();
    let mut seats = Vec::new();
    for i in 0..4 {
        let id = SeatId::from_u128(i + 1);
        let rect = Rect::new(
            (i % 2) as f32 * 560.,
            (i / 2) as f32 * 410. + 24.,
            560.,
            386.,
        );
        app.pane_bounds.push((id, rect));
        app.pane_faces
            .insert(bbt_mux::Ulid(id.0.0), PaneFace::HarnessRich);
        let mut record = bbt_proto::registry::SeatRecord::new(
            bbt_mux::Ulid(id.0.0),
            bbt_mux::Ulid(10),
            "budget fixture",
            "bboxy450-harness",
        );
        record.generation = 1;
        record.harness_session_id = Some(bbt_mux::Ulid(id.0.0).to_string());
        seats.push(bbt_proto::GuiSeat {
            rich_socket: Some(root.path().join(format!("{i}.sock"))),
            record,
            attention: None,
            notification_episode: None,
            state: bbt_proto::SeatState {
                generation: 1,
                process: bbt_proto::ProcessStatus::Running { pid: peer.0.id() },
                activity: bbt_proto::Activity::Idle,
                grade: bbt_proto::Grade::Shim,
                title: String::new(),
                detail: None,
                turn_open: false,
                observed_ms: 0,
                subagents: vec![],
            },
        });
    }
    model.gui_seats = seats.into();
    app.live_state = Some(Arc::new(model));
    app.rich_sync();
    assert_eq!(app.rich_panes.len(), 4);
    wait(|| {
        app.rich_drain();
        app.rich_panes.values().all(|p| {
            p.snapshot.ready
                && p.snapshot.completed_inputs >= 1
                && p.snapshot.cursor.is_some_and(|(_, _, s)| s.0 == head)
        })
    });
    let cold_projection_sum: f64 = app
        .rich_panes
        .values()
        .map(|p| p.snapshot.projection.as_secs_f64() * 1000.)
        .sum();
    let targets = publish_four(&app, true);
    clock_frame(&mut app);
    let first = targets
        .iter()
        .filter(|(id, n)| app.rich_panes[id].snapshot.completed_inputs >= *n)
        .count();
    assert_eq!(
        first, 3,
        "shared 300us deadline must defer the fourth 100us drain"
    );
    assert!(pending(&app, &targets));
    let mut frames = 1;
    while pending(&app, &targets) && frames < 4 {
        clock_frame(&mut app);
        frames += 1;
    }
    assert!(
        !pending(&app, &targets),
        "round-robin deferral must retain every latest publication"
    );
    assert_eq!(frames, 2);
    let mut drain = Vec::new();
    let mut compose = Vec::new();
    let mut slice = Vec::new();
    let mut worker = Vec::new();
    for round in 0..128 {
        let targets = publish_four(&app, round % 2 == 0);
        let until = Instant::now() + Duration::from_secs(30);
        while pending(&app, &targets) {
            assert!(
                Instant::now() < until,
                "real clock starved a visible latest snapshot"
            );
            let before: Vec<_> = app
                .rich_panes
                .iter()
                .map(|(id, p)| (*id, Arc::as_ptr(&p.snapshot.paints)))
                .collect();
            let at = Instant::now();
            app.rich_drain();
            let d = at.elapsed().as_secs_f64() * 1000.;
            let at = Instant::now();
            let paints = app.rich_paints();
            let c = at.elapsed().as_secs_f64() * 1000.;
            assert!(!paints.is_empty());
            let projected: f64 = before
                .iter()
                .filter(|(id, previous)| {
                    !std::ptr::eq(*previous, Arc::as_ptr(&app.rich_panes[id].snapshot.paints))
                })
                .map(|(id, _)| app.rich_panes[id].snapshot.projection.as_secs_f64() * 1000.)
                .sum();
            drain.push(d);
            compose.push(c);
            slice.push(d + c);
            worker.push(projected);
        }
        for pane in app.rich_panes.values() {
            assert_eq!(pane.snapshot.retained_items, expected_items);
            assert_eq!(pane.snapshot.order_len, expected_items);
            assert_eq!(pane.snapshot.duplicate_durable, 0);
            assert_eq!(pane.snapshot.cursor.unwrap().2.0, head);
        }
    }
    let d = p99(&mut drain);
    let c = p99(&mut compose);
    let total = p99(&mut slice);
    let w = p99(&mut worker);
    app.occluded = true;
    app.rich_sync();
    assert!(app.rich_panes.is_empty());
    app.occluded = false;
    app.rich_sync();
    assert_eq!(app.rich_panes.len(), 4);
    app.pane_bounds.clear();
    app.rich_sync();
    assert!(app.rich_panes.is_empty());
    // SAFETY: observe only the exact private Python session fixture PID above.
    assert_eq!(unsafe { libc::kill(peer.0.id() as i32, 0) }, 0);
    let report = json!({"measurement":"actual App::rich_drain and App::rich_paints, four production Unix actors",
        "harness":std::env::var("BBOXY_HARNESS_CANDIDATE").ok(),"visible_views":4,"rounds":128,"ui_frames":drain.len(),
        "ui_drain_p99_ms":d,"ui_composition_p99_ms":c,"ui_rich_slice_p99_ms":total,
        "worker_projection_elapsed_sum_for_drained_snapshots_p99_ms":w,"cold_worker_projection_elapsed_sum_ms":cold_projection_sum,
        "worker_scope":"HarnessView::render plus Frame-to-Paint; excludes Bridge background preparation",
        "worker_sum_is_concurrent_elapsed_not_cpu_or_ui_time":true,"drain_allocation_ms":0.3,"projection_allocation_ms":0.6,
        "ui_drain_p99_within_allocation":d<=0.3,"ui_composition_p99_within_allocation":c<=0.6,
        "worker_projection_sum_p99_over_allocation":w>0.6,"injected_clock":{"budget_us":300,"per_drain_us":100,
            "first_frame_drained":first,"deferred":4-first,"frames_to_catch_up":frames,"latest_retained":true},
        "durable_head_per_view":head,"retained_items_per_view":expected_items,"duplicate_durable_per_view":0,
        "hidden_native_views":0,"occluded_native_views":0,"under_load":"UNMEASURED","whole_frame_budget_verdict":"UNPROVEN",
        "heap_usage":"UNMEASURED","memory_allocation_mib_per_view":32});
    let output = std::env::var_os("BBOXY_RICH_BUDGET_OUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/rich-budgets"));
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "| term row | visible | drain p99 ms | composition p99 ms | worker projection elapsed sum p99 ms | load | whole frame | heap |"
    );
    println!(
        "| harness-rich | 4 | {d:.6} | {c:.6} | {w:.6} | UNMEASURED | UNPROVEN | UNMEASURED (32 MiB allocation/view) |"
    );
    println!("rich-budget: {report}");
}
