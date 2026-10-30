// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::Composer;
use crate::{
    self as ui, components as c,
    modules::{seat_row::named, stream_dock::StreamDock, strip::plain},
};
use bbt_ui_model::{
    STYLE,
    layout::{Axis, Rect},
};

/// Requested sidebar splits; fitting a small window never changes these choices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarLayout {
    pub conversation_fraction: f32,
    pub composer_body_height: Option<f32>,
    pub tasks_height: Option<f32>,
}
impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            conversation_fraction: 0.45,
            composer_body_height: None,
            tasks_height: None,
        }
    }
}
impl SidebarLayout {
    pub fn valid(self) -> bool {
        self.conversation_fraction.is_finite()
            && (0.05..=0.95).contains(&self.conversation_fraction)
            && self
                .composer_body_height
                .is_none_or(|v| v.is_finite() && (0.0..=4096.).contains(&v))
            && self
                .tasks_height
                .is_none_or(|v| v.is_finite() && (0.0..=4096.).contains(&v))
    }
}

/// Shared inline stream surface. Only rows scroll; the composer stays at its
/// foot. The caller owns the selected stream, offset, focus and effects.
#[derive(Default)]
pub struct Conversation {
    pub stream: StreamDock,
    pub composer: Composer,
    pub scroll: f32,
    pub tasks: Option<StreamDock>,
    pub task_scroll: f32,
}
impl Conversation {
    pub fn element(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        self.element_inner(ctx, size, m, None, "conversation")
    }
    /// Host-bounded sidebar variant. History and the editable body fit the dock;
    /// an arbitrarily long draft cannot grow the navigation off the window.
    pub fn element_bounded(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
        requested_body: Option<f32>,
    ) -> ui::style::Result<c::Element> {
        self.element_inner(ctx, size, m, Some(requested_body), "conversation")
    }
    /// Independent Chat and Tasks panels. None keeps the legacy stream/gallery tree.
    // bboxy450-term bounded sidebar sections; retain source credit on extraction.
    // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn element_sidebar(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
        requested_body: Option<f32>,
        requested_tasks_height: Option<f32>,
    ) -> ui::style::Result<c::Element> {
        self.element_sidebar_open(
            ctx,
            size,
            m,
            [requested_body, requested_tasks_height],
            [true, true],
        )
    }
    /// Open states are caller-owned, in Tasks/Chat order. Folding never edits the draft.
    pub fn element_sidebar_open(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
        requested: [Option<f32>; 2],
        open: [bool; 2],
    ) -> ui::style::Result<c::Element> {
        use crate::modules::accordion::Accordion;
        let Some(tasks) = &self.tasks else {
            return self.element_bounded(ctx, size, m, requested[0]);
        };
        let sp = &STYLE.spacing;
        let gap = (bbt_ui_model::TOKENS.layout.cohort_gap as f32).min(size[1].max(0.));
        let edge = bbt_ui_model::TOKENS.sidebar.edge as f32;
        let left = edge + sp.s3 as f32;
        let right = sp.s3 as f32 + STYLE.material.hairline_px as f32;
        let width = (size[0] - left - right).max(1.);
        let task_chrome = Accordion::chrome_height(ctx, open[0]);
        let chat_chrome = Accordion::chrome_height(ctx, open[1]);
        let height = size[1].max(0.);
        let chat_min_body = if open[1] {
            self.chat_minimum(ctx, width, m)?
        } else {
            0.
        };
        let max_tasks = (height - chat_chrome - chat_min_body - gap).max(task_chrome);
        let tasks_height = if !open[0] {
            task_chrome.min(height)
        } else if !open[1] {
            max_tasks
        } else {
            requested[1]
                .unwrap_or(3. * task_chrome)
                .clamp(task_chrome.min(max_tasks), max_tasks)
        };
        let chat_height = (height - tasks_height - gap).max(0.);
        let task_size = Accordion::body_size(ctx, [width, tasks_height], open[0]);
        let chat_size = Accordion::body_size(ctx, [width, chat_height], open[1]);
        let tasks_body = open[0].then(|| {
            named(
                "conversation/tasks",
                c::Element::ScrollView(c::ScrollView {
                    viewport: task_size,
                    offset: [0., self.task_scroll],
                    content_width: Some(task_size[0]),
                    content: Box::new(tasks.content_from(ctx, self.stream.entries.len(), m)),
                }),
            )
        });
        let chat_body = if open[1] {
            Some(self.element_inner(
                ctx,
                chat_size,
                m,
                Some(requested[0]),
                "conversation/chat-content",
            )?)
        } else {
            None
        };
        let task_count = tasks
            .entries
            .iter()
            .filter(|entry| matches!(entry, crate::modules::stream_dock::Entry::Task { .. }))
            .count();
        let chat_count = self
            .stream
            .entries
            .iter()
            .filter(|entry| matches!(entry, crate::modules::stream_dock::Entry::Message { .. }))
            .count();
        let tasks = Accordion {
            key: "conversation/task-section",
            title: "Tasks",
            color: bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.section.tasks).0,
            count: task_count,
            open: open[0],
        }
        .element(ctx, [width, tasks_height], tasks_body);
        let chat = Accordion {
            key: "conversation/chat-section",
            title: "Chat",
            color: bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.section.chat).0,
            count: chat_count,
            open: open[1],
        }
        .element(ctx, [width, chat_height], chat_body);
        let mut style = plain(Some(height), [0., right, 0., left], 0.);
        style.overrides.width = Some(ui::Length::Px(size[0]));
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(ctx.theme.palette().sidebar).0,
        )));
        let mut frame = c::Box::new(
            style,
            vec![
                c::Item {
                    element: tasks,
                    extent: c::Extent::Fixed(tasks_height),
                },
                c::Item {
                    element: chat,
                    extent: c::Extent::Fixed(chat_height),
                },
            ],
        );
        frame.direction = Axis::Column;
        frame.gap = ui::Length::Px(gap);
        frame.align = c::Alignment::Stretch;
        frame.clip = true;
        Ok(named("conversation", c::Element::Box(frame)))
    }
    fn chat_minimum(
        &self,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
    ) -> ui::style::Result<f32> {
        Ok((self.minimum_height(ctx, width, m)? - 4. * STYLE.spacing.s8 as f32).max(0.))
    }
    pub fn minimum_sidebar_height(
        &self,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
        open: [bool; 2],
    ) -> ui::style::Result<f32> {
        use crate::modules::accordion::Accordion;
        if self.tasks.is_none() {
            return self.minimum_height(ctx, width, m);
        }
        Ok(Accordion::chrome_height(ctx, open[0])
            + Accordion::chrome_height(ctx, open[1])
            + bbt_ui_model::TOKENS.layout.cohort_gap as f32
            + if open[1] {
                self.chat_minimum(ctx, width, m)?
            } else {
                0.
            })
    }
    pub fn minimum_height(
        &self,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
    ) -> ui::style::Result<f32> {
        let sp = &STYLE.spacing;
        let inner = (width - sp.s2 as f32 - sp.s3 as f32).max(1.);
        let mut composer = self.composer.clone();
        composer.body_height = Some(0.);
        let controls = composer
            .element(ctx, inner, m)?
            .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
            .rect
            .h;
        Ok(controls
            + 2. * sp.s3 as f32
            + 1.
            + 24.
            + STYLE.r#type.caption as f32 * 2.9
            + if self.tasks.is_some() {
                4. * sp.s8 as f32
            } else {
                0.
            })
    }
    fn element_inner(
        &self,
        ctx: ui::Context,
        size: [f32; 2],
        m: &dyn c::Measure,
        bounded: Option<Option<f32>>,
        key: &str,
    ) -> ui::style::Result<c::Element> {
        let sp = &STYLE.spacing;
        let inner = (size[0] - sp.s2 as f32 - sp.s3 as f32).max(1.);
        let mut view = self.composer.clone();
        if let Some(requested) = bounded {
            let natural = view
                .element(ctx, inner, m)?
                .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
                .rect
                .h;
            view.body_height = Some(0.);
            let controls = view
                .element(ctx, inner, m)?
                .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
                .rect
                .h;
            let maximum = (size[1] - 2. * sp.s3 as f32 - 1. - controls - 24.).max(0.);
            view.body_height = Some(
                requested
                    .unwrap_or((natural - controls).max(0.))
                    .clamp(0., maximum),
            );
        }
        let composer = view.element(ctx, inner, m)?;
        let h = composer
            .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
            .rect
            .h;
        let height = if bounded.is_some() {
            size[1].max(0.)
        } else {
            size[1].max(h + 2. * sp.s3 as f32 + 1.)
        };
        let rows_h = (height - 2. * sp.s3 as f32 - 1. - h).max(0.);
        let rows = c::Element::ScrollView(c::ScrollView {
            viewport: [inner, rows_h],
            offset: [0., self.scroll],
            content_width: Some(inner),
            content: Box::new(self.stream.content(ctx, m)),
        });
        let mut style = plain(
            Some(height),
            [sp.s3 as f32, sp.s3 as f32, sp.s3 as f32, sp.s2 as f32],
            0.,
        );
        style.overrides.width = Some(ui::Length::Px(size[0]));
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(ctx.theme.palette().sidebar).0,
        )));
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        style.overrides.border_sides = Some([true, false, false, false]);
        if key == "conversation/chat-content" {
            style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
            style.overrides.border = None;
        }
        let mut b = c::Box::new(
            style,
            vec![named("conversation/rows", rows).into(), composer.into()],
        );
        b.direction = Axis::Column;
        b.gap = ui::Length::Px(0.);
        b.align = c::Alignment::Stretch;
        Ok(named(key, c::Element::Box(b)))
    }
}
