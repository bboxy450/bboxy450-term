// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private native clipboard values through the real worker, Unix Bridge and actor.
#![allow(
    clippy::disallowed_methods,
    reason = "Private unattended fixture owns its runtime and files; no UI thread or general clipboard"
)]
use bboxy450_harness::{
    engine::{Config, SessionActor},
    gateway::{Gateway, unix::UnixServer},
    policy::Policy,
    protocol::*,
    provider::{ScriptedProvider, profile::Unattended},
    redact::Set,
    store::Blobs,
};
use bboxy450_term::rich_pane::{Input, Pane};
use bboxy450_term_desktop::{ClipboardContent, ClipboardImageFormat};
use bbt_ui_model::layout::Rect;
use std::{sync::Arc, time::Duration};
struct Wake;
impl bbt_rt::Waker for Wake {
    fn wake(&self) {}
}
fn png() -> Vec<u8> {
    let mut out = vec![];
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 80, 255, 20, 255, 0, 255])
            .unwrap();
    }
    out
}
async fn wait(pane: &mut Pane, predicate: impl Fn(&Pane) -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            pane.drain();
            if predicate(pane) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("native pane timeout: ready={} error={:?} notice={:?} composer={} attachments={} pending={} items={} inputs={}",pane.snapshot.ready,pane.snapshot.error,pane.snapshot.notice,pane.snapshot.composer_len,pane.snapshot.attachment_count,pane.snapshot.pending_attachment_count,pane.snapshot.retained_items,pane.snapshot.completed_inputs));
}
#[tokio::test]
async fn private_native_clipboard_and_drop_reach_shared_worker_and_durable_image() {
    let temp = tempfile::tempdir().unwrap();
    let meta = SessionMeta {
        source: None,
        session: SessionId::new(),
        cwd: temp.path().into(),
        profile: None,
        fork: None,
        dream: None,
        vcs: None,
        protocol: PROTOCOL,
        harness: "fixture".into(),
        created_ms: 0,
    };
    let session = meta.session;
    let config = Config {
        meta,
        profiles: temp.path().join("profiles"),
        sessions: temp.path().join("sessions"),
        provider: Arc::new(ScriptedProvider::new(vec![])),
        model: "scripted".into(),
        policy: Policy::new(Unattended::Deny),
        redactor: Set::default(),
        max_rounds: 1,
        profile_error: None,
    };
    let (gateway, task) = SessionActor::open(config, OpId::new()).await.unwrap();
    let socket = temp.path().join("private.sock");
    let server = UnixServer::bind(socket.clone(), gateway.clone())
        .await
        .unwrap();
    let mut pane = Pane::connect(
        socket,
        17,
        Rect::new(0., 0., 900., 800.),
        None,
        Arc::new(Wake),
    );
    wait(&mut pane, |p| p.snapshot.ready && p.snapshot.image_intake).await;
    let bytes = png();
    assert!(pane.send(Input::clipboard(ClipboardContent::Image {
        bytes: bytes.clone(),
        format: ClipboardImageFormat::Png
    })));
    wait(&mut pane, |p| p.snapshot.attachment_count == 1).await;
    assert_eq!(pane.snapshot.images.len(), 1);
    // Drop-style file intake uses the identical worker-owned shared composer.
    let file = temp.path().join("界 note.txt");
    std::fs::write(&file, "NATIVE FILE CONTENT").unwrap();
    assert!(pane.send(Input::Attach(
        bboxy450_harness::frontends::gui::attachments::Intake::Files(vec![file])
    )));
    wait(&mut pane, |p| p.snapshot.attachment_count == 2).await;
    assert!(pane.send(Input::Enter));
    wait(&mut pane, |p| {
        p.snapshot.attachment_count == 0
            && p.snapshot.pending_attachment_count == 0
            && p.snapshot.retained_items >= 1
    })
    .await;
    let log = std::fs::read_to_string(
        temp.path()
            .join("sessions")
            .join(format!("{session}.jsonl")),
    )
    .unwrap();
    let events: Vec<Envelope> = log
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let users: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.event {
            Event::ItemCompleted {
                body: Item::UserMessage { blocks },
                ..
            } => Some(blocks),
            _ => None,
        })
        .collect();
    assert_eq!(
        users.len(),
        1,
        "one Enter has exactly one durable user message"
    );
    assert!(
        users[0]
            .iter()
            .any(|b| matches!(b,InputBlock::Text {text} if text.contains("NATIVE FILE CONTENT")))
    );
    let blob = users[0]
        .iter()
        .find_map(|b| match b {
            InputBlock::Image { blob } => Some(blob),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        Blobs {
            dir: temp
                .path()
                .join("sessions")
                .join(session.to_string())
                .join("blobs"),
            redactor: Set::default()
        }
        .get(blob)
        .await
        .unwrap(),
        bytes
    );
    drop(pane);
    gateway
        .send(OpEnvelope {
            v: PROTOCOL,
            op_id: OpId::new(),
            session,
            op: Op::Close,
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.close().await.unwrap();
}
