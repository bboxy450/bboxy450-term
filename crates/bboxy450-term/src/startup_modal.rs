// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The disconnected cockpit owns input above the retained scene.
use crate::*;
use bboxy450_term::cockpit;
use bbt_ui::{self as ui, components as c};

fn view(modal: &cockpit::startup::Modal, elapsed_secs: u64) -> ui::modules::startup::Startup {
    use cockpit::startup::Phase;
    use ui::modules::startup::{History, Startup};
    let active_step = match modal.phase {
        Phase::StartingSupervisor
        | Phase::AdoptingSupervisor(_)
        | Phase::Reconnecting { .. }
        | Phase::Failed(_) => 0,
        Phase::StartingLogic | Phase::RefreshingLogic(_) => 1,
        Phase::VerifyingHistory { .. } => 2,
        Phase::AttachingGui | Phase::RestoringState { .. } => 3,
    };
    let mut details = vec![];
    if let Phase::AdoptingSupervisor(pid) = modal.phase {
        details.push(["Backend".into(), format!("PID {pid}")]);
    }
    if let Some((attempt, reason)) = &modal.reconnect {
        details.push([format!("Attempt {attempt}"), reason.clone()]);
    }
    if let Some(reason) = &modal.refresh_reason {
        details.push(["Backend update".into(), reason.clone()]);
    }
    let title = if modal.error.is_some() {
        "Startup failed"
    } else if matches!(modal.phase, Phase::Reconnecting { .. }) {
        "Reconnecting"
    } else if matches!(modal.phase, Phase::RefreshingLogic(_)) {
        "Updating backend"
    } else {
        "Starting bboxy"
    };
    Startup {
        title: title.into(),
        description: String::new(),
        elapsed_secs,
        active_step,
        history: if let Phase::VerifyingHistory {
            stream,
            verified,
            total,
        } = &modal.phase
        {
            Some(History {
                stream: stream.clone(),
                verified: *verified,
                total: *total,
                operation: None,
            })
        } else if let Phase::RestoringState {
            operation,
            completed,
            total,
        } = &modal.phase
        {
            Some(History {
                stream: String::new(),
                verified: *completed,
                total: *total,
                operation: Some(
                    match operation.as_str() {
                        "Loading session registry" => "Load sessions",
                        "Loading workspace records" => "Load workspace",
                        "Restoring messages" => "Restore messages",
                        "Restoring saved sessions" => "Restore session state",
                        "Resuming saved sessions" => "",
                        "Restoring task state" => "Restore tasks",
                        "Restoring pending deliveries" => "Restore deliveries",
                        "Checking identity" => "Check identity",
                        "Opening control endpoint" => "Open controls",
                        _ => operation,
                    }
                    .into(),
                ),
            })
        } else {
            None
        },
        details,
        error: modal.error.clone(),
        logs: modal
            .run_dir
            .as_ref()
            .filter(|_| modal.error.is_some())
            .map(|run| {
                vec![
                    ["App log".into(), run.join("app.log").display().to_string()],
                    [
                        "Backend log".into(),
                        run.join("logic.log").display().to_string(),
                    ],
                ]
            })
            .unwrap_or_default(),
    }
}

fn region(layout: &c::Layout, key: &str) -> Option<Rect> {
    if matches!(&layout.kind, c::LayoutKind::Named(name) if name == key) {
        return Some(layout.rect);
    }
    layout.children.iter().find_map(|child| region(child, key))
}

impl App {
    pub(super) fn paint_startup_modal(&mut self) -> Result<Vec<c::Paint>, String> {
        if !self.startup.visible {
            self.startup_layout = None;
            return Ok(vec![]);
        }
        let ctx = self.scene.ui_context();
        let element = view(&self.startup, self.startup.since.elapsed().as_secs())
            .element(ctx)
            .map_err(str::to_owned)?;
        let layout = self
            .scene
            .app_dialog_element_layout(element, self.solved_size, 560.)?;
        let mut paint = self.scene.app_dialog_backdrop(self.solved_size);
        paint.extend(c::paint(&layout, ctx));
        self.startup_layout = Some(layout);
        Ok(paint)
    }
    pub(super) fn startup_action_at(&self, at: [f32; 2]) -> Option<&'static str> {
        let layout = self.startup_layout.as_ref()?;
        ["startup/quit", "startup/retry"].into_iter().find(|key| {
            region(layout, key).is_some_and(|r| {
                at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
            })
        })
    }
    pub(super) fn retry_startup(&mut self) {
        if self.startup.error.is_none() {
            return;
        }
        self.startup.retry(Instant::now());
        if let Some(commands) = &self.live_commands
            && !commands.is_closed()
        {
            commands.retry_startup();
        } else if let Some(gpu) = &self.gpu {
            let (sender, rendering) = tokio::sync::oneshot::channel();
            let ctx = self.scene.ui_context();
            let _ = sender.send(live::RenderContext {
                context: gpu.shared_context(),
                scale: self
                    .window
                    .as_ref()
                    .map_or(1., |window| window.scale_factor() as f32),
                theme: ctx.theme,
            });
            let proxy = self.proxy.clone();
            self.live_commands = Some(live::start_with_sink(
                self.live_updates.clone(),
                Arc::new(move |event| {
                    let _ = proxy.send_event(event);
                }),
                self.logger.clone(),
                self.startup.run_dir.clone(),
                !std::env::args().any(|arg| arg == "--no-ctl"),
                rendering,
            ));
        }
        self.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cockpit::startup::{Modal, Phase};

    #[test]
    fn startup_modal_projects_observed_steps_and_sticky_failure_without_inventing_progress() {
        let now = Instant::now();
        let mut modal = Modal::new(now, true);
        for (phase, step) in [
            (Phase::StartingSupervisor, 0),
            (Phase::AdoptingSupervisor(42), 0),
            (Phase::StartingLogic, 1),
            (
                Phase::VerifyingHistory {
                    stream: "account".into(),
                    verified: 25,
                    total: 100,
                },
                2,
            ),
            (Phase::AttachingGui, 3),
        ] {
            modal.update(phase, now);
            assert_eq!(view(&modal, 3).active_step, step);
        }
        modal.update(
            Phase::VerifyingHistory {
                stream: "account".into(),
                verified: 25,
                total: 100,
            },
            now,
        );
        modal.fail("history signature failed".into());
        modal.update(Phase::AttachingGui, now);
        let failed = view(&modal, 4);
        assert_eq!(failed.active_step, 2);
        assert_eq!(failed.history.unwrap().verified, 25);
        assert_eq!(failed.error.as_deref(), Some("history signature failed"));
        modal.retry(now);
        let retry = view(&modal, 0);
        assert_eq!(retry.active_step, 0);
        assert!(retry.error.is_none() && retry.history.is_none());
        assert_eq!(retry.details[0], ["Attempt 1", "Retry requested"]);
        modal.update(Phase::AttachingGui, now);
        modal.attached();
        assert!(!modal.visible);
    }
}
