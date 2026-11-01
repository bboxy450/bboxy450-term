// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Headless rich toolkit stories. These use the normal retained paint and Metal
//! replay-image path. Independent browser parity requires separate references;
//! native captures alone never claim reference equivalence.
use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_ui::{
    self as ui, components as c,
    modules::{
        approval::{Approval, Key, OptionItem},
        diff::{Diff, DiffView, Folds},
        tool_call::{Status, ToolCall},
        transcript::{Message, ScrollStick, Transcript},
    },
    text_edit::{CompletionPopover, Editor},
    widgets::text_area::TextArea,
};
use bbt_ui_model::{
    draw::{Color, DrawCtx},
    layout::{Axis, Rect},
};
use serde_json::json;

pub const STATES: &[&str] = &[
    "markdown-streaming",
    "syntax-highlighted",
    "transcript",
    "transcript-scrolled",
    "diff",
    "diff-folded",
    "tool-pending",
    "tool-running",
    "tool-succeeded",
    "tool-failed",
    "tool-cancelled",
    "tool-folded",
    "approval",
    "approval-disabled",
    "completion-slash",
    "completion-mention",
];
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "toolkit".into(),
                    state: (*state).into(),
                    slug: (*state).into(),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: String::new(),
                    selector: String::new(),
                    width: 640,
                    height: 480,
                    checker: false,
                    fixture: Some(json!({"layer":"toolkit","reference":"native-story"})),
                });
            }
        }
    }
    out
}
pub fn element(case: &Case) -> Result<c::Element> {
    let ctx = crate::components::context(case);
    let m = crate::components::CachedMetrics::default();
    let width = 608.;
    Ok(match case.state.as_str() {
        "markdown-streaming" => {
            let mut md = bbt_doc::StreamingMarkdown::default();
            for chunk in [
                "## Build notes\n\n",
                "| Check | Result |\n| --- | --- |\n",
                "| Unit | Passed |\n\n",
                "```rust\nfn main() {\n",
            ] {
                md.append(chunk);
            }
            md.document().body_element(ctx, &m, width, &[])
        }
        "syntax-highlighted" => {
            let p=bbt_doc::highlight::Work {generation:1,revision:1,light:case.theme=="light",
                source:"## Prepared code\n\n```rust\npub fn sum(a: u32, b: u32) -> u32 {\n    // Keep the current revision.\n    a + b\n}\n```\n".into()}.prepare();
            p.document
                .highlighted_body_element(&p.highlights, ctx, &m, width)
        }
        "transcript" | "transcript-scrolled" => {
            let messages = (0..6)
                .map(|i| Message {
                    key: format!("message-{i}"),
                    title: if i % 2 == 0 {
                        "You".into()
                    } else {
                        "Assistant".into()
                    },
                    body: bbt_doc::parse(if i % 2 == 0 {
                        "Review **the patch** and its tests."
                    } else {
                        "### Result\n\nThe tests pass.\n\n- Checked parsing\n- Checked rendering"
                    })
                    .body_element(ctx, &m, width, &[]),
                })
                .collect::<Vec<_>>();
            let mut scroll = ScrollStick::default();
            if case.state.ends_with("scrolled") {
                scroll.user_scroll(48., 800.);
            }
            Transcript {
                messages: &messages,
                scroll: &scroll,
            }
            .element([width, 448.])
        }
        "diff" | "diff-folded" => {
            let diff = Diff::parse(
                "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n pub fn ready() -> bool {\n-    false\n+    let current = true;\n+    current\n }\n@@ -8 +9 @@\n-old_name();\n+new_name();\n",
            )?;
            let mut folds = Folds::default();
            if case.state.ends_with("folded") {
                folds.toggle(0);
            }
            DiffView {
                key: "diff",
                diff: &diff,
                folds: &folds,
            }
            .element(ctx, width, &m)
        }
        s if s.starts_with("tool-") => {
            let status = match s {
                "tool-pending" => Status::Pending,
                "tool-running" => Status::Running,
                "tool-failed" => Status::Failed,
                "tool-cancelled" => Status::Cancelled,
                _ => Status::Succeeded,
            };
            ToolCall {
                key: "tool".into(),
                title: "Read configuration".into(),
                status,
                arguments: "{\n  \"path\": \"Cargo.toml\"\n}".into(),
                output: if status == Status::Failed {
                    "Read failed: file does not exist.".into()
                } else {
                    "[package]\nname = \"demo\"".into()
                },
                arguments_open: s != "tool-folded",
                output_open: s != "tool-folded",
            }
            .element(ctx, width, &m)
        }
        "approval" | "approval-disabled" => {
            let mut a = Approval::new(
                "approval",
                "Allow the command to run?",
                vec![
                    OptionItem {
                        key: "once".into(),
                        label: "Allow once".into(),
                        enabled: true,
                    },
                    OptionItem {
                        key: "always".into(),
                        label: "Allow for this session".into(),
                        enabled: false,
                    },
                    OptionItem {
                        key: "deny".into(),
                        label: "Deny".into(),
                        enabled: true,
                    },
                ],
            );
            a.handle(Key::Down);
            if case.state.ends_with("disabled") {
                for o in &mut a.options {
                    o.enabled = false;
                }
            }
            a.element(ctx)
        }
        "completion-slash" | "completion-mention" => {
            let value = if case.state.ends_with("slash") {
                "/re"
            } else {
                "Review @src/"
            }
            .to_string();
            let e = Editor::new(&value);
            let popup = CompletionPopover::new(
                e.completion(&value).unwrap(),
                if case.state.ends_with("slash") {
                    vec!["/review".into(), "/restart".into()]
                } else {
                    vec!["@src/main.rs".into(), "@src/lib.rs".into()]
                },
            );
            let input = TextArea {
                value,
                placeholder: "Write a message".into(),
                style: ui::Text::default(),
                edit: Some(e.edit()),
                min_lines: 2,
                wrap: true,
                name: "editor".into(),
            }
            .element(width, ctx, &m)?;
            c::Element::Stack(c::Stack {
                direction: Axis::Column,
                gap: ui::Length::Px(8.),
                align: c::Alignment::Stretch,
                children: vec![input.into(), popup.element(ctx).into()],
            })
        }
        _ => return Err("unknown toolkit story".into()),
    })
}
pub fn layout_case(case: &Case) -> Result<c::Layout> {
    let ctx = crate::components::context(case);
    Ok(element(case)?.layout(
        Rect::new(16., 16., case.width as f32 - 32., case.height as f32 - 32.),
        ctx,
        &crate::components::CachedMetrics::default(),
    )?)
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let ctx = crate::components::context(case);
    let color = Color::hex(ctx.theme.palette().bg)
        .0
        .map(|c| (c * 255.).round() as u8);
    let pixels = color.repeat((case.width * case.height) as usize);
    crate::components::execute(
        renderer,
        c::paint_with_draw(&layout_case(case)?, DrawCtx::new(ctx.theme, ctx.density)),
        pixels,
        [case.width, case.height],
        ctx,
    )
}
