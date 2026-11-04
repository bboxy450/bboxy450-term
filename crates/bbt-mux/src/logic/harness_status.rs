// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One independently scheduled log reader per harness seat; no PTY output needed.
use super::*;
use bboxy450_harness_wake_up::harness_log::{self, Log};

pub(super) async fn run(
    client: LogicClient,
    binding: Binding,
    root: PathBuf,
    endpoint: Option<PathBuf>,
) {
    let mut previous = binding.session.clone();
    loop {
        let session = match match &endpoint {
            Some(path) => harness_log::discover_socket(path, None, &binding).await,
            None => harness_log::discover(&root, &binding).await,
        } {
            Ok(session) => session,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let log_root = root.clone();
        let log_session = session.clone();
        let Ok(Ok(mut log)) =
            tokio::task::spawn_blocking(move || Log::open(&log_root, &log_session)).await
        else {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };
        let socket = endpoint
            .clone()
            .unwrap_or_else(|| root.join("run").join(format!("{session}.sock")));
        let mut discovery_at = Instant::now() + Duration::from_secs(15);
        loop {
            // Slow file IO belongs to this seat's blocking task, never the logic
            // owner or a shared reader. EOF is not a new observation/heartbeat.
            let Ok((returned, update)) = tokio::task::spawn_blocking(move || {
                let update = log.poll();
                (log, update)
            })
            .await
            else {
                return;
            };
            log = returned;
            match update {
                Ok(Some(mut event)) => {
                    event["binding"] = serde_json::to_value(&binding).unwrap();
                    event["previous_session"] = serde_json::json!(previous);
                    if client
                        .native_event(binding.seat, binding.generation, event)
                        .await
                        .is_err()
                    {
                        return;
                    }
                    previous = session.clone();
                }
                Ok(None) => {}
                Err(_) => break,
            }
            if Instant::now() >= discovery_at {
                if harness_log::hello(&socket, &session, &binding)
                    .await
                    .is_err()
                {
                    break;
                }
                discovery_at = Instant::now() + Duration::from_secs(15);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
