// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended Unix actor fixture"
)]
use bboxy450_term::rich_pane::{Input, Pane};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{Color, Prim},
    layout::Rect,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
struct Wake;
impl bbt_rt::Waker for Wake {
    fn wake(&self) {}
}
struct Server {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}
fn until(pane: &mut Pane, predicate: impl Fn(&Pane) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        pane.drain();
        if predicate(pane) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "worker timeout: ready={} theme={:?} error={:?}",
            pane.snapshot.ready,
            pane.snapshot.theme,
            pane.snapshot.error
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn current_content(pane: &Pane) -> bool {
    pane.painted().iter().any(|paint|matches!(paint,c::Paint::Draw(list) if list.primitives.iter().any(|prim|matches!(prim,Prim::Text{text,..} if text.contains("actor:")))))
}
#[test]
fn harness_theme_worker_preserves_identity_draft_and_latest_aba() {
    if std::env::var_os("BBOXY_PARENT_THEME_PRIVATE").is_none() {
        let home = tempfile::Builder::new()
            .prefix("parent-theme-home-")
            .tempdir_in("/tmp")
            .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "harness_theme::harness_theme_worker_preserves_identity_draft_and_latest_aba",
                "--nocapture",
            ])
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .env("BBOXY_PARENT_THEME_PRIVATE", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let root = tempfile::Builder::new()
        .prefix("parent-theme-socket-")
        .tempdir_in("/tmp")
        .unwrap();
    let socket = root.path().join("private.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let operations = Arc::new(AtomicUsize::new(0));
    let server_stop = stop.clone();
    let observed = operations.clone();
    let server = Server {
        stop,
        thread: Some(std::thread::spawn(move || {
            while !server_stop.load(Ordering::Acquire) {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_millis(50)))
                    .unwrap();
                let mut reader = BufReader::new(stream);
                let mut subscription = String::new();
                reader.read_line(&mut subscription).unwrap();
                let meta = json!({"created_ms":0,"cwd":"/private/theme-fixture","fork":null,"harness":"fixture","profile":"private","protocol":{"major":1,"minor":1},"session":"00000000000000000000000001"});
                let hello = json!({"v":{"major":1,"minor":1},"meta":meta,"epoch":7,"head":4});
                writeln!(reader.get_mut(), "{hello}").unwrap();
                let events = [
                    json!({"type":"session_started","meta":meta}),
                    json!({"type":"turn_started","turn":"00000000000000000000000002"}),
                    json!({"type":"item_started","item":"00000000000000000000000003","kind":"agent_message","head":null,"parent":null,"turn":"00000000000000000000000002"}),
                    json!({"type":"item_completed","item":"00000000000000000000000003","body":{"kind":"agent_message","markdown":"Theme actor: `retained input`.\n\n```python\nprint(42)\n```"},"status":{"status":"completed"}}),
                ];
                for (i, event) in events.into_iter().enumerate() {
                    let envelope = json!({"v":{"major":1,"minor":1},"session":"00000000000000000000000001","epoch":7,"seq":i+1,"at":i+1,"ts_ms":i+1,"cause":null,"event":event});
                    writeln!(reader.get_mut(), "{envelope}").unwrap();
                }
                while !server_stop.load(Ordering::Acquire) {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let value: Value = serde_json::from_str(&line).unwrap();
                            if value.get("op").is_some() {
                                observed.fetch_add(1, Ordering::AcqRel);
                            }
                        }
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(e) => panic!("private peer: {e}"),
                    }
                }
            }
        })),
    };
    let mut pane = Pane::connect(
        socket,
        23,
        Rect::new(0., 0., 640., 420.),
        None,
        Arc::new(Wake),
    );
    until(&mut pane, |p| {
        p.snapshot.ready
            && p.snapshot.cursor.is_some_and(|(_, _, s)| s.0 == 4)
            && current_content(p)
    });
    let cursor = pane.snapshot.cursor;
    let retained = pane.snapshot.retained_items;
    assert!(pane.send(Input::Edit(ui::text_edit::Command::Insert(
        "draft survives".into()
    ))));
    until(&mut pane, |p| p.snapshot.composer_len == 14);
    for theme in [ui::Theme::Light, ui::Theme::Dark, ui::Theme::Light] {
        assert!(pane.set_theme(theme));
        assert!(!pane.set_theme(theme));
        assert!(
            pane.painted().is_empty(),
            "old paint must be fenced immediately"
        );
        until(&mut pane, |p| {
            p.snapshot.theme == theme && current_content(p)
        });
        assert_eq!(pane.snapshot.cursor, cursor);
        assert_eq!(pane.generation, 23);
        assert_eq!(pane.snapshot.retained_items, retained);
        assert_eq!(pane.snapshot.composer_len, 14);
        let bg = Color::hex(theme.palette().bg);
        assert!(pane.painted().iter().any(|paint|matches!(paint,c::Paint::Draw(list) if list.primitives.iter().any(|prim|matches!(prim,Prim::Rect{fill,..} if *fill==bg)))));
    }
    let revision = pane.snapshot.appearance_revision;
    pane.set_theme(ui::Theme::Dark);
    pane.set_theme(ui::Theme::Light);
    assert!(pane.painted().is_empty());
    until(&mut pane, |p| {
        p.snapshot.appearance_revision > revision
            && p.snapshot.theme == ui::Theme::Light
            && current_content(p)
    });
    assert_eq!(pane.snapshot.cursor, cursor);
    assert_eq!(pane.snapshot.composer_len, 14);
    assert_eq!(
        operations.load(Ordering::Acquire),
        0,
        "appearance sends no agent/model operations"
    );
    drop(pane);
    drop(server);
}
