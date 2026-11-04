// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Empty task-panel states. The host owns requests and retry deadlines.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskState {
    pub loading: bool,
    pub message: String,
    pub retry_in: Option<u64>,
}
impl TaskState {
    pub fn element(&self, ctx: ui::Context, size: [f32; 2]) -> c::Element {
        let label = |value: &str, step, color, weight| {
            let mut text = c::Text::new(
                value,
                ui::Text {
                    step,
                    weight,
                    paint: ui::Paint::solid(color),
                    ..Default::default()
                },
            );
            text.align = c::Alignment::Center;
            c::Element::Text(text)
        };
        let mut children = vec![label(
            if self.loading {
                "Loading tasks"
            } else {
                "Tasks unavailable"
            },
            ui::TypeStep::Body,
            ui::Color::Foreground,
            600,
        )];
        if !self.message.is_empty() {
            children.push(label(
                &self.message,
                ui::TypeStep::Caption,
                ui::Color::Muted,
                400,
            ));
        }
        if self.loading {
            let mut track_style = plain(None, [0.; 4], 2.);
            track_style.overrides.width = Some(ui::Length::Px(112.));
            track_style.overrides.height = Some(ui::Length::Px(4.));
            track_style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface2));
            // The same active rim material as a working harness pane. Its
            // opposing waves run on the shared effect clock, with no layout motion.
            track_style.overrides.rim = Some(ui::Rim::block(
                ui::GlowActivity::Active,
                false,
                Some(
                    ui::Paint::solid(ui::Color::Accent)
                        .resolve(ctx)
                        .expect("accent token"),
                ),
                ui::rim_phase("conversation/tasks-progress"),
                false,
                2.,
                2.,
            ));
            children.push(named(
                "conversation/tasks-progress",
                c::Element::Box(c::Box::new(track_style, vec![])),
            ));
        } else {
            let refresh = self.retry_in.map_or_else(
                || "Refresh".into(),
                |seconds| format!("Refresh in {seconds}s"),
            );
            if let Ok(button) = (ui::widgets::Button {
                label: refresh,
                kind: ui::widgets::ButtonKind::Ghost,
                state: ui::widgets::ButtonState::Idle,
                icon: None,
                key: None,
            })
            .element(ctx)
            {
                children.push(named("conversation/tasks-refresh", button));
            }
        }
        let content = named(
            "conversation/tasks-state/content",
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(12.),
                align: c::Alignment::Center,
                children: children.into_iter().map(Into::into).collect(),
            }),
        );
        let mut style = plain(None, [16.; 4], 0.);
        style.overrides.width = Some(ui::Length::Px(size[0].max(0.)));
        style.overrides.height = Some(ui::Length::Px(size[1].max(0.)));
        let spacer = || c::Item {
            element: c::Element::Spacer,
            extent: c::Extent::Flex(1.),
        };
        let mut panel = c::Box::new(style, vec![spacer(), content.into(), spacer()]);
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Center;
        named("conversation/tasks-state", c::Element::Box(panel))
    }
}
