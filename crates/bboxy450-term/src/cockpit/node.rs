// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Foreground backend owner, using the same supervisor/logic session as the app.
use super::{Config, Event, Session};
use bbt_mux::{
    GUI_UNAVAILABLE,
    store::{Result, invalid},
};
use bbt_proto::{GuiAction, GuiReply, gui::AppStatus};
use std::{path::PathBuf, sync::Arc};
use tokio::signal::unix::{SignalKind, signal};

pub const USAGE: &str = "usage: bboxy450-term node [--run-dir DIR] [--repository DIR] [--no-ctl]
                          [--unattended --test-account DIR]";

#[allow(
    clippy::disallowed_methods,
    reason = "foreground non-UI process entry point"
)]
pub fn run(args: &[String]) -> Result<()> {
    if args == ["--help"] || args == ["-h"] {
        println!("{USAGE}");
        return Ok(());
    }
    let mut run_dir = None;
    let mut remaining = args.iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--run-dir" => {
                let path = remaining.next().filter(|path| !path.starts_with('-'));
                if run_dir.is_some() || path.is_none_or(String::is_empty) {
                    return Err(invalid("--run-dir needs one directory"));
                }
                run_dir = path.map(PathBuf::from);
            }
            "--repository" | "--test-account" => {
                // Config's shared parsers validate the selector and its value.
                remaining.next();
            }
            "--no-ctl" | "--ctl" | "--unattended" => {}
            arg if arg.starts_with("--repository=") || arg.starts_with("--test-account=") => {}
            _ => return Err(invalid(USAGE)),
        }
    }
    let mut config = Config::discover(run_dir)?;
    config.ctl = !args.iter().any(|arg| arg == "--no-ctl");
    bbt_rt::handle().block_on(serve(config, args.iter().any(|arg| arg == "--unattended")))
}

async fn serve(config: Config, unattended: bool) -> Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut session = Session::start_policy(config, Arc::new(|_| {}), false, None, false).await?;
    let result = loop {
        let event = tokio::select! {
            _ = terminate.recv() => break Ok(()),
            _ = interrupt.recv() => break Ok(()),
            event = session.event() => event,
        };
        let reply = match event {
            Ok(Event::Reply(reply)) => match *reply {
                GuiReply::AppAsk { op, .. } => Some(GuiAction::AppReply {
                    id: op,
                    status: AppStatus {
                        gui: None,
                        running: true,
                        mode: "node".into(),
                        unattended,
                        backend_ready: true,
                        gui_ready: false,
                        events_ready: false,
                        custody_backend: session.endpoint.custody_backend.clone(),
                        generation: session.endpoint.generation,
                        frame_seq: 0,
                        windows: 0,
                        close: serde_json::Value::Null,
                        notifications: serde_json::Value::Null,
                        history: serde_json::Value::Null,
                    },
                }),
                // Logic emits this after writing the authenticated ctl stop receipt.
                GuiReply::AppStopping { .. } => break Ok(()),
                GuiReply::UiAsk { op, .. } => Some(GuiAction::UiReply {
                    id: op,
                    result: Box::new(Err(GUI_UNAVAILABLE.into())),
                }),
                GuiReply::CaptureAsk { op, .. } => Some(GuiAction::Captured {
                    id: op,
                    error: Some(GUI_UNAVAILABLE.into()),
                    frame: Default::default(),
                }),
                _ => None,
            },
            Ok(Event::Disconnected(reason)) if session.recovery_failed => {
                break Err(invalid(reason));
            }
            Ok(_) => None,
            Err(error) => break Err(error),
        };
        if let Some(reply) = reply
            && let Err(error) = session.send(reply).await
        {
            eprintln!("node control reply: {error}");
        }
    };
    result.and(session.shutdown().await)
}
