// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The disconnected cockpit owns input above the retained scene.
use crate::*;
use bbt_ui::{self as ui, components as c};

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
        let mut lines = vec![format!(
            "{} s elapsed",
            self.startup.since.elapsed().as_secs()
        )];
        if let Some((attempt, reason)) = &self.startup.reconnect {
            lines.push(format!(
                "Reconnect attempt {attempt} · Last reason: {reason}"
            ));
        }
        if let Some(reason) = &self.startup.refresh_reason {
            lines.push(format!("Refreshing logic: {reason}"));
        }
        if let Some(error) = &self.startup.error {
            lines.push(error.clone());
            if let Some(run) = &self.startup.run_dir {
                lines.push(format!("App log: {}", run.join("app.log").display()));
                lines.push(format!("Logic log: {}", run.join("logic.log").display()));
            }
        }
        let body = c::Element::Stack(c::Stack {
            direction: bbt_ui_model::layout::Axis::Column,
            gap: ui::Length::Spacing(3),
            align: c::Alignment::Stretch,
            children: lines
                .into_iter()
                .map(|line| {
                    c::Element::Text(c::Text::new(
                        line,
                        ui::Text {
                            step: ui::TypeStep::Caption,
                            paint: ui::Paint::solid(ui::Color::Foreground),
                            ..Default::default()
                        },
                    ))
                    .into()
                })
                .collect(),
        });
        let button = |label: &str, kind, key| -> Result<c::Element, String> {
            Ok(ui::modules::seat_row::named(
                key,
                ui::widgets::Button {
                    label: label.into(),
                    kind,
                    state: ui::widgets::ButtonState::Idle,
                    icon: None,
                    key: None,
                }
                .element(ctx)
                .map_err(str::to_owned)?,
            ))
        };
        let mut actions = vec![button(
            "Quit",
            ui::widgets::ButtonKind::Ghost,
            "startup/quit",
        )?];
        if self.startup.error.is_some() {
            actions.push(button(
                "Retry",
                ui::widgets::ButtonKind::Primary,
                "startup/retry",
            )?);
        }
        let modal = ui::overlays::Modal::new(self.startup.label(), body, actions);
        let layout = self.scene.app_dialog_layout(modal, self.solved_size)?;
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
