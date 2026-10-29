// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::Composer;
use crate::{
    self as ui, components as c,
    modules::{
        seat_row::named,
        stream_dock::StreamDock,
        strip::{plain, text},
    },
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
        let Some(tasks) = &self.tasks else {
            return self.element_bounded(ctx, size, m, requested_body);
        };
        let sp = &STYLE.spacing;
        let height = size[1].max(0.);
        let header = 2. * sp.s8 as f32;
        let chat_minimum = self.minimum_height(ctx, size[0], m)? - header;
        let tasks_maximum = (height - chat_minimum).max(0.);
        let tasks_height = requested_tasks_height
            .filter(|v| v.is_finite())
            .unwrap_or(3. * header)
            .clamp(header.min(tasks_maximum), tasks_maximum);
        let chat_height = (height - tasks_height).max(0.);
        let chat_header = header.min(chat_height);
        let chat_body = (chat_height - chat_header).max(0.);
        let heading = |key: &str, title: &str, h: f32, divider: bool| {
            let mut style = plain(Some(h), [0., sp.s3 as f32, 0., sp.s2 as f32], 0.);
            if divider {
                style.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: ui::Paint {
                        color: ui::Color::Line,
                        alpha: ui::Alpha::Hairline,
                    },
                    outside: false,
                });
                style.overrides.border_sides = Some([true, false, false, false]);
            }
            let mut block = c::Box::new(
                style,
                vec![
                    text(
                        title,
                        STYLE.r#type.body as f32,
                        STYLE.r#type.weight_name as u16,
                        false,
                        ui::Color::Foreground,
                        1.,
                        1.3,
                    )
                    .into(),
                ],
            );
            block.direction = Axis::Row;
            block.align = c::Alignment::Center;
            block.clip = true;
            named(key, c::Element::Box(block))
        };
        let panel = |key: &str, h: f32, children: Vec<c::Item>| {
            let mut style = plain(Some(h), [0.; 4], 0.);
            style.overrides.width = Some(ui::Length::Px(size[0]));
            let mut block = c::Box::new(style, children);
            block.direction = Axis::Column;
            block.gap = ui::Length::Px(0.);
            block.align = c::Alignment::Stretch;
            block.clip = true;
            named(key, c::Element::Box(block))
        };
        let fixed = |element, h| c::Item {
            element,
            extent: c::Extent::Fixed(h),
        };
        let chat = panel(
            "conversation/chat-section",
            chat_height,
            vec![
                fixed(
                    heading("conversation/chat-header", "Chat", chat_header, false),
                    chat_header,
                ),
                fixed(
                    self.element_inner(
                        ctx,
                        [size[0], chat_body],
                        m,
                        Some(requested_body),
                        "conversation/chat-content",
                    )?,
                    chat_body,
                ),
            ],
        );
        let tasks_header = header.min(tasks_height);
        let tasks_rows = (tasks_height - tasks_header).max(0.);
        let inner = (size[0] - sp.s2 as f32 - sp.s3 as f32).max(1.);
        let rows = named(
            "conversation/tasks",
            c::Element::ScrollView(c::ScrollView {
                viewport: [inner, tasks_rows],
                offset: [0., self.task_scroll],
                content_width: Some(inner),
                content: Box::new(tasks.content_from(ctx, self.stream.entries.len(), m)),
            }),
        );
        let mut inset = c::Box::new(
            plain(Some(tasks_rows), [0., sp.s3 as f32, 0., sp.s2 as f32], 0.),
            vec![fixed(rows, tasks_rows)],
        );
        inset.direction = Axis::Column;
        inset.align = c::Alignment::Stretch;
        let tasks = panel(
            "conversation/task-section",
            tasks_height,
            vec![
                fixed(
                    heading("conversation/tasks-header", "Tasks", tasks_header, true),
                    tasks_header,
                ),
                fixed(c::Element::Box(inset), tasks_rows),
            ],
        );
        let mut outer = panel(
            "conversation",
            height,
            vec![fixed(chat, chat_height), fixed(tasks, tasks_height)],
        );
        if let c::Element::Named { content, .. } = &mut outer
            && let c::Element::Box(block) = content.as_mut()
        {
            block.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(
                bbt_ui_model::draw::Color::hex(ctx.theme.palette().sidebar).0,
            )));
        }
        Ok(outer)
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
