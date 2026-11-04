// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Durable original-session admission. A reconnect never re-aims a retained message.
use crate::{
    adapters::{Evidence, WakeCause, WakeOutcome},
    store::{Store, sqlite::SqliteStore},
};
use bboxy450_harness_wake_up::rich::Client;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use ulid::Ulid;
#[derive(Clone)]
pub(crate) struct Delivery {
    pub client: Client,
    directory: PathBuf,
    admission: Arc<tokio::sync::Mutex<()>>,
}
impl Delivery {
    pub fn new(client: Client, directory: PathBuf) -> Self {
        Self {
            client,
            directory,
            admission: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
    pub async fn text(&self, id: Ulid, text: &str, submit: bool) -> WakeOutcome {
        if !submit {
            return refuse(
                WakeCause::NativeRejected,
                "harness socket accepts complete turns, submit must be true",
            );
        }
        self.send(
            id,
            json!({"type":"user_turn","blocks":[{"type":"text","text":text}],"mode":"queue"}),
        )
        .await
    }
    pub async fn send(&self, id: Ulid, op: Value) -> WakeOutcome {
        let Some(hello) = self.client.hello() else {
            return unavailable();
        };
        // Per-seat, not a global gate; disk work never occupies the logic/UI owner.
        let guard = self.admission.lock().await;
        let directory = self.directory.clone();
        let saved_op = op.clone();
        let binding = tokio::task::spawn_blocking(move || -> Result<Ulid, String> {
            let store = SqliteStore::open(&directory).map_err(|e| e.to_string())?;
            let key = format!("harness-rich-op/{id}");
            if let Some(bytes) = store.get(&key).map_err(|e| e.to_string())? {
                let prior: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if prior["op"] != saved_op {
                    return Err("op_id reused with different content".into());
                }
                return serde_json::from_value(prior["session"].clone()).map_err(|e| e.to_string());
            }
            store
                .put(
                    &key,
                    &serde_json::to_vec(&json!({"session":hello.session,"op":saved_op}))
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            Ok(hello.session)
        })
        .await;
        drop(guard);
        match binding {
            Ok(Ok(session)) => {
                crate::adapters::mapping::outcome(self.client.send(id, session, op).await)
            }
            _ => refuse(
                WakeCause::InputBindingChanged,
                "harness admission could not be retained or op_id content changed; no write",
            ),
        }
    }
}
fn refuse(cause: WakeCause, note: &str) -> WakeOutcome {
    WakeOutcome {
        evidence: Evidence::RecipientStored,
        native_id: None,
        note: note.into(),
        cause: Some(cause),
    }
}

pub(crate) fn unavailable() -> WakeOutcome {
    crate::adapters::mapping::outcome(bboxy450_harness_wake_up::rich::unavailable())
}
