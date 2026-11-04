// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The native owner receives usage events; private/headless fixture owners never read accounts.
impl crate::App {
    #[allow(clippy::disallowed_methods)] // The snapshot lock is taken in the poller's callback, off the UI thread.
    pub(super) fn start_usage(&mut self) {
        let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
            return;
        };
        let proxy = self.proxy.clone();
        // The same readings, for MCP `usage_read` (logic reads run/usage.json).
        let snapshot = run_dir().map(|dir| {
            std::sync::Arc::new(std::sync::Mutex::new(Snapshot {
                path: dir.join(USAGE_SNAPSHOT),
                value: serde_json::json!({"machine": null, "providers": {}}),
            }))
        });
        self.usage_worker = Some(bboxy450_usage::poller::Worker::start(
            bbt_rt::handle(),
            home.clone(),
            bboxy450_usage::providers(),
            std::sync::Arc::new(bboxy450_usage::io::LiveReader { home }),
            std::sync::Arc::new(bboxy450_usage::io::LiveHttp::default()),
            std::sync::Arc::new(move |update| {
                if let Some(snapshot) = &snapshot {
                    snapshot
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record(&update);
                }
                let _ = proxy.send_event(crate::UiEvent::Usage(update));
            }),
            Default::default(),
        ));
    }
}

/// File name of the usage snapshot in the run directory.
pub const USAGE_SNAPSHOT: &str = "usage.json";

/// The run directory logic also uses: --run-dir, else the supervisor socket's.
fn run_dir() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pair) = args.windows(2).find(|pair| pair[0] == "--run-dir") {
        return Some(pair[1].clone().into());
    }
    bbt_mux::transport::default_socket_path()
        .ok()?
        .parent()
        .map(std::path::Path::to_path_buf)
}

/// Last machine sample and provider readings, rewritten whole on each update.
struct Snapshot {
    path: std::path::PathBuf,
    value: serde_json::Value,
}
impl Snapshot {
    /// Runs on the poller's runtime thread, never the UI thread.
    fn record(&mut self, update: &bboxy450_usage::poller::Update) {
        use bboxy450_usage::poller::Update;
        use serde_json::json;
        match update {
            Update::Machine(m) => {
                self.value["machine"] = json!({"cpu": m.cpu, "memory": m.memory, "disk": m.disk, "reason": m.reason});
            }
            Update::Provider { id, reading } => {
                let previous = &self.value["providers"][*id];
                let reading = match reading {
                    None => json!(null),
                    // A refresh or error keeps the last known numbers, marked stale.
                    Some(r) if r.used.is_none() && previous["used"].is_number() => {
                        let mut kept = previous.clone();
                        kept["stale"] = json!(true);
                        kept["reason"] = json!(r.reason);
                        kept
                    }
                    Some(r) => json!({
                        "used": r.used,
                        "projected": r.projected,
                        "stale": r.stale,
                        "reason": r.reason,
                        "windows": r.windows.iter().map(|w| json!({
                            "label": w.label,
                            "used": w.used,
                            "projected": w.projected,
                            "resets_at": w.resets_at,
                        })).collect::<Vec<_>>(),
                    }),
                };
                self.value["providers"][*id] = reading;
            }
        }
        self.value["sampled_at"] = json!(bboxy450_usage::now() as i64);
        if let Err(error) = self.write() {
            eprintln!("usage snapshot: {error}");
        }
    }
    #[allow(clippy::disallowed_methods)] // A small atomic file write on the poller's runtime thread.
    fn write(&self) -> std::io::Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let partial = self.path.with_extension("json.partial");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        file.write_all(&serde_json::to_vec(&self.value)?)?;
        std::fs::rename(&partial, &self.path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(clippy::disallowed_methods)] // Test reads of its own temporary snapshot.
    fn snapshot_file_holds_the_last_readings_and_keeps_numbers_through_a_refresh() {
        use bboxy450_usage::{Reading, machine::Machine, poller::Update};
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = super::Snapshot {
            path: dir.path().join(super::USAGE_SNAPSHOT),
            value: serde_json::json!({"machine": null, "providers": {}}),
        };
        snapshot.record(&Update::Machine(Machine {
            cpu: Some(48),
            memory: Some(64),
            disk: Some(95),
            reason: None,
        }));
        snapshot.record(&Update::Provider {
            id: "claude",
            reading: Some(Reading::current(96)),
        });
        snapshot.record(&Update::Provider {
            id: "claude",
            reading: Some(Reading::error("Refreshing usage")),
        });
        let read: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&snapshot.path).unwrap()).unwrap();
        assert_eq!(read["machine"]["disk"], 95);
        assert_eq!(read["providers"]["claude"]["used"], 96);
        assert_eq!(read["providers"]["claude"]["stale"], true);
        assert!(read["sampled_at"].as_i64().unwrap() > 0);
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&snapshot.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    use bboxy450_usage::{Reading, machine::Machine, poller::Update};
    #[test]
    fn machine_and_provider_events_update_only_the_last_view() {
        let mut scene = crate::scene::Scene::live();
        scene.apply_usage(Update::Machine(Machine {
            cpu: Some(42),
            memory: Some(71),
            disk: Some(63),
            reason: None,
        }));
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: Some(Reading::current(58)),
        });
        scene.apply_usage(Update::Provider {
            id: "grok",
            reading: Some(Reading::error("Run `grok` to sign in")),
        });
        scene.apply_usage(Update::Provider {
            id: "codex",
            reading: None,
        });
        // A signed-out account is hidden, not shown as "grok —".
        assert_eq!(scene.usage.providers.len(), 1);
        assert!(scene.usage.accessible_label().contains("CPU 42%"));
        assert!(scene.usage.accessible_label().contains("Claude 58% used"));
        // A refresh in flight keeps the last value instead of blinking out.
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: Some(Reading::error("Refreshing usage")),
        });
        assert!(scene.usage.accessible_label().contains("Claude 58% used"));
        assert!(!scene.usage.accessible_label().contains("rok"));
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: None,
        });
        assert_eq!(scene.usage.providers.len(), 0);
        let mut prepared = crate::scene::Scene::live();
        let _ = prepared.layout([1600., 900.], false).unwrap();
        prepared.inherit_usage(&mut scene);
        assert!(scene.usage.machine.is_empty());
        assert!(prepared.usage.accessible_label().contains("CPU 42%"));
        let installed = prepared.layout([1600., 900.], false).unwrap();
        assert!(format!("{installed:?}").contains("CPU 42%"));
    }

    #[test]
    fn theme_button_label_names_the_saved_choice_and_the_next_one() {
        let mut scene = crate::scene::Scene::live();
        let mut preferences = bboxy450_term::settings::Preferences::default();
        for (saved, want) in [
            ("auto", "Theme: Auto (system is light)\nClick for Light"),
            ("light", "Theme: Light\nClick for Dark"),
            ("dark", "Theme: Dark\nClick for Auto"),
        ] {
            preferences.appearance.theme = saved.into();
            scene.set_appearance(&preferences, bbt_ui::Theme::Light);
            // Text paints one run per word; join them back into the visible label.
            let paints = format!("{:?}", scene.theme_tooltip([1600., 900.]));
            let shown: String = paints
                .split("text: \"")
                .skip(1)
                .filter_map(|run| run.split('"').next())
                .collect();
            assert_eq!(shown, want.replace('\n', ""), "{saved}");
        }
    }
}
