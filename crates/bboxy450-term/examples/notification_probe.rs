// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Operator-driven native evidence. This executable starts no logic, PTY or
//! identity store; cold activation always recreates the same two fixture tabs.
#![allow(
    clippy::disallowed_methods,
    reason = "debug probe setup and dedicated evidence IO worker, not the product UI thread"
)]
#[cfg(not(debug_assertions))]
compile_error!("notification_probe is debug-only; it must not be included in release builds");

use bboxy450_desktop_notifications::{
    self as notifications, Notification, NotificationId, Notifier, Response,
};
use bbt_ui_model::{
    Model, Seat, SeatId, SplitNode, Swarm, SwarmId, Tab, TabId,
    input::{Focus, FocusOrigin},
    layout::Rect,
};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::mpsc::{self, SyncSender},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

const ID: &str = "00000000000000000000000003";
#[derive(Debug)]
enum Event {
    Response(Response),
    Result(String),
}
enum Command {
    Native(char),
    Log(serde_json::Value),
}
struct Probe {
    commands: SyncSender<Command>,
    model: Model,
    focus: Focus,
    window: Option<Window>,
    pending: Option<NotificationId>,
}
impl Probe {
    fn log(&self, kind: &str, data: serde_json::Value) {
        if self
            .commands
            .try_send(Command::Log(serde_json::json!({"kind":kind,"data":data})))
            .is_err()
        {
            eprintln!("probe log queue full; this run is not evidence");
        }
    }
    fn click(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let Some(id) = self.pending.take() else {
            return;
        };
        if id.0 != ID {
            self.log("refused", serde_json::json!("unknown fixture id"));
            return;
        }
        let seat = SeatId::from_u128(3);
        let previous = self.model.window.stage.clone();
        let events = self
            .focus
            .activate(&mut self.model, seat, FocusOrigin::Row, Instant::now())
            .unwrap();
        let switched = previous.tab != self.model.window.stage.tab;
        if switched {
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, bbt_ui_model::event::Event::Locate { .. })),
                "tab switch must defer the landing"
            );
        }
        let size = window.inner_size();
        let landing = self.focus.layout_ready(
            &self.model,
            &[(
                seat,
                Rect {
                    x: 0.,
                    y: 0.,
                    w: size.width as f32,
                    h: size.height as f32,
                },
            )],
            Instant::now(),
        );
        if switched {
            assert!(
                matches!(landing.as_slice(),[bbt_ui_model::event::Event::Locate {seat:found,..}] if *found==seat)
            );
        }
        window.set_minimized(false);
        window.focus_window();
        window.set_title("bboxy450 notification probe — focused fixture seat 3 / tab 2");
        self.log("focused",serde_json::json!({"seat":ID,"tab":self.model.window.stage.tab.map(|id|id.0.to_string()),"deferred_landing":switched,"raise_requested":true}));
    }
}
impl ApplicationHandler<Event> for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window=Some(event_loop.create_window(Window::default_attributes().with_title("bboxy450 notification probe: A allow · P post · R replace · C count · D remove · Q quit")).unwrap());
            self.log(
                "window_ready",
                serde_json::json!({"pid":std::process::id(),"keys":"A P R C D Q"}),
            );
            self.click();
        }
    }
    fn user_event(&mut self, _: &ActiveEventLoop, event: Event) {
        match event {
            Event::Response(Response::Clicked(id)) => {
                self.log("clicked", serde_json::json!({"id":id.0}));
                self.pending = Some(id);
                self.click();
            }
            Event::Response(Response::Dismissed(id)) => {
                self.log("dismissed", serde_json::json!({"id":id.0}))
            }
            Event::Result(result) => self.log("native_result", serde_json::json!(result)),
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let key = match event {
            WindowEvent::Focused(focused) => {
                self.log("window_focus", serde_json::json!({"focused":focused}));
                None
            }
            WindowEvent::CloseRequested => Some('q'),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.logical_key {
                    Key::Character(text) => text.chars().next().map(|c| c.to_ascii_lowercase()),
                    Key::Named(NamedKey::Escape) => Some('q'),
                    _ => None,
                }
            }
            _ => None,
        };
        match key {
            Some('q') => {
                self.log("exit", serde_json::json!({"pid":std::process::id()}));
                event_loop.exit();
            }
            Some(key @ ('a' | 'p' | 'r' | 'c' | 'd'))
                if self.commands.try_send(Command::Native(key)).is_err() =>
            {
                self.log("busy", serde_json::json!(key));
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--help") {
        println!(
            "debug-only, operator-driven notification proof; A authorize, P post, R replace delivered id, C list actual delivered ids, D remove, Q quit; no logic/PTY/Keychain"
        );
        return Ok(());
    }
    let event_loop = EventLoop::<Event>::with_user_event().build()?;
    let response_proxy = event_loop.create_proxy();
    // Kept before run_app: cold-launch delivery is the empirical acceptance.
    let notifier = match notifications::open(Box::new(move |response| {
        let _ = response_proxy.send_event(Event::Response(response));
    })) {
        Ok(notifier) => notifier,
        Err(error) => {
            eprintln!("probe open: {error:?}");
            return Err(error.into());
        }
    };
    let log_dir = std::env::temp_dir().join("bboxy450-notification-probe");
    std::fs::create_dir_all(&log_dir)?;
    let path = log_dir.join("proof.jsonl");
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err("probe evidence path is not a regular file".into());
    }
    use std::os::unix::fs::OpenOptionsExt;
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)?;
    eprintln!("probe evidence: {}", path.display());
    let (commands, receive) = mpsc::sync_channel(16);
    let results = event_loop.create_proxy();
    let worker = std::thread::spawn(move || {
        while let Ok(command) = receive.recv() {
            let data = match command {
                Command::Log(value) => value,
                Command::Native(key) => {
                    let result = native(notifier.as_ref(), key);
                    let _ = results.send_event(Event::Result(result.clone()));
                    serde_json::json!({"kind":"native","key":key,"result":result})
                }
            };
            let value = serde_json::json!({"pid":std::process::id(),"ts_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string(),"record":data});
            if writeln!(output, "{value}")
                .and_then(|()| output.flush())
                .is_err()
            {
                eprintln!("probe evidence write failed");
            }
        }
    });
    let swarm = SwarmId::from_u128(1);
    let model = Model::new(
        vec![Swarm::new(
            swarm,
            "probe",
            vec![
                Tab::new(
                    TabId::from_u128(1),
                    "initial",
                    SplitNode::Leaf(SeatId::from_u128(1)),
                ),
                Tab::new(
                    TabId::from_u128(2),
                    "notification target",
                    SplitNode::Leaf(SeatId::from_u128(3)),
                ),
            ],
        )],
        [1, 3]
            .into_iter()
            .map(|id| {
                Seat::new(
                    SeatId::from_u128(id),
                    swarm,
                    [0; 32],
                    format!("fixture {id}"),
                    "fixture",
                )
            })
            .collect(),
    )?;
    let mut app = Probe {
        commands,
        model,
        focus: Focus::default(),
        window: None,
        pending: None,
    };
    app.log(
        "launch",
        serde_json::json!({"cold_fixture":ID,"delegate_installed_before_run_app":true,"executable":std::env::current_exe().ok()}),
    );
    let result = event_loop.run_app(&mut app);
    drop(app);
    worker.join().map_err(|_| "probe worker panicked")?;
    result?;
    Ok(())
}
fn native(notifier: &dyn Notifier, key: char) -> String {
    match key {
        'a' => format!("authorize: {:?}", notifier.authorize()),
        'p' | 'r' => format!(
            "post: {:?}",
            notifier.post(&Notification {
                id: NotificationId(ID.into()),
                title: "bboxy450 fixture · needs your answer".into(),
                body: if key == 'p' {
                    "First delivered fixture — click focuses seat 3"
                } else {
                    "Replacement of the already delivered fixture"
                }
                .into()
            })
        ),
        'd' => format!("remove: {:?}", notifier.remove(&NotificationId(ID.into()))),
        'c' => format!("delivered: {:?}", notifications::probe::delivered_ids()),
        _ => "invalid command".into(),
    }
}
