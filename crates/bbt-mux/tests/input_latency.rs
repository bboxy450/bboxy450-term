// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private raw PTY/socket measurement fixture"
)]
mod common;

use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
    streams::Replica,
};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatId, Update};
use ed25519_dalek::SigningKey;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Fixture {
    logic: Option<Logic>,
    supervisor: Option<Supervisor>,
    runtime: tokio::runtime::Runtime,
    root: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.logic.take().unwrap().shutdown().unwrap();
        self.supervisor.take().unwrap().shutdown().unwrap();
    }
}
fn update_identity(update: &Update) -> (bbt_proto::View, u64) {
    match update {
        Update::Snapshot { snapshot } => (snapshot.view, snapshot.sequence),
        Update::Delta { delta } => (delta.view, delta.sequence),
    }
}

#[test]
fn isolated_raw_cat_input_ack_to_watched_echo_reports_exact_samples() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("repository")).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(root.path())).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.path().join("logic"),
            repository: root.path().join("repository"),
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[54; 32]),
        runtime.handle(),
    )
    .unwrap();
    logic.client.enable_capture_diagnostics();
    let fixture = Fixture {
        logic: Some(logic),
        supervisor: Some(supervisor),
        runtime,
        root,
    };
    let logic = fixture.logic.as_ref().unwrap();
    let seat = Ulid::new();
    let wire_seat = SeatId::from_u128(seat.0);
    let mut spec = LaunchSpec::new("/bin/sh", fixture.root.path());
    spec.seat = seat;
    spec.args = vec![
        "-c".into(),
        "stty -echo -icanon min 1 time 0; printf 'RAW-CAT-READY\\r\\n'; exec /bin/cat".into(),
    ];
    spec.env = vec![
        (
            "HOME".into(),
            fixture.root.path().to_string_lossy().into_owned(),
        ),
        ("PATH".into(), "/usr/bin:/bin".into()),
    ];
    let info = fixture
        .runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "input latency fixture", "fixture"),
            spec,
        ))
        .unwrap();
    let original_pid = info.pid;
    let report = fixture.runtime.block_on(async {
        let endpoint = logic.client.gui_endpoint().await.unwrap();
        let stream = tokio::net::UnixStream::connect(&endpoint.path).await.unwrap();
        let (read,mut write) = stream.into_split();
        let mut read = BufReader::new(read);
        let mut replica = Replica::default();
        macro_rules! send {
            ($action:expr) => {{
                let mut bytes=serde_json::to_vec(&GuiRequest {version:bbt_proto::gui::VERSION,capability:endpoint.capability,action:$action}).unwrap();
                bytes.push(b'\n');write.write_all(&bytes).await.unwrap();
            }};
        }
        macro_rules! receive {
            () => {{
                let mut line=String::new();
                assert!(tokio::time::timeout(Duration::from_secs(5),read.read_line(&mut line)).await.unwrap().unwrap()>0,"private GUI socket closed");
                let reply:GuiReply=serde_json::from_str(&line).unwrap();
                if let GuiReply::Error {reason,..}=&reply {panic!("private input refused: {reason}");}
                reply
            }};
        }
        let view=logic.client.snapshot(seat,info.generation).await.unwrap().view;
        send!(GuiAction::Watch {seat:wire_seat,view});
        loop {
            if let GuiReply::Update {seat:got,update}=receive!() {
                assert_eq!(got,wire_seat);
                let (view,sequence)=update_identity(&update);
                replica.apply(update).unwrap();
                send!(GuiAction::Applied {seat:wire_seat,view,sequence});
                let text:String=replica.snapshot.as_ref().unwrap().screen.cells.iter().flatten().map(|cell|cell.c).collect();
                if text.contains("RAW-CAT-READY") {break;}
            }
        }
        let mut samples=Vec::new();
        for sample in 0..16 {
            // Explicit idle stimulus spacing, varied against the existing20ms
            // viewport clock. This is not a settling condition for the echo.
            tokio::time::sleep(Duration::from_millis(61+sample%7)).await;
            let id=Ulid::new();
            let marker=format!("P{sample:02}x");
            let before=replica.snapshot.as_ref().unwrap().sequence;
            let started=Instant::now();
            send!(GuiAction::Input {seat:wire_seat,generation:info.generation,id,bytes:marker.as_bytes().to_vec()});
            let mut acknowledged=None;
            let (view,sequence,echo_at)=loop {
                match receive!() {
                    GuiReply::Ok {op} if op==id=>acknowledged=Some(Instant::now()),
                    GuiReply::Update {seat:got,update}=> {
                        assert_eq!(got,wire_seat);
                        let (view,sequence)=update_identity(&update);
                        replica.apply(update).unwrap();
                        let text:String=replica.snapshot.as_ref().unwrap().screen.cells.iter().flatten().map(|cell|cell.c).collect();
                        if text.contains(&marker) {break (view,sequence,Instant::now());}
                        send!(GuiAction::Applied {seat:wire_seat,view,sequence});
                    }
                    _=>{}
                }
            };
            let acknowledged=acknowledged.expect("input acknowledgement precedes its echo update");
            assert_eq!(view.generation,info.generation);
            assert!(sequence>before);
            samples.push(serde_json::json!({"sample":sample,"operation":id,"seat":seat,"generation":view.generation,
                "pid":original_pid,"baseline_sequence":before,"echo_sequence":sequence,
                "input_ack_us":acknowledged.duration_since(started).as_micros(),
                "ack_to_echo_update_us":echo_at.duration_since(acknowledged).as_micros(),
                "input_to_echo_update_us":echo_at.duration_since(started).as_micros()}));
            send!(GuiAction::Applied {seat:wire_seat,view,sequence});
        }
        serde_json::json!({"measurement":"private_gui_input_to_watched_raw_cat_echo","samples":samples,
            "scope":"same admitted persistent GUI socket; no frontend/GPU claim; no ctl process startup or100ms wait polling",
            "capture_diagnostics":logic.client.capture_diagnostics()})
    });
    assert_eq!(
        fixture
            .runtime
            .block_on(logic.client.record(seat))
            .unwrap()
            .generation,
        info.generation
    );
    println!("INPUT_LATENCY {}", serde_json::to_string(&report).unwrap());
}
