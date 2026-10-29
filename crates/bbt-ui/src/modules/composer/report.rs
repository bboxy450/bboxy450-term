// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::modules::{
    seat_row::named,
    stream_dock::{badge, column, fill, label, rgba, row, spacer, text_style, tint},
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, layout::Axis};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Verification {
    pub command: String,
    pub exit_code: i32,
    pub evidence: Vec<Criterion>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    pub criterion: String,
    pub reference: String,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Report {
    pub task: String,
    pub status: String,
    pub summary: String,
    pub commit: Option<String>,
    pub verify_result: Option<Verification>,
    pub reject: Option<Reject>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reject {
    pub reason: String,
    pub edit: Option<c::Edit>,
    pub notice: Option<super::Notice>,
}
/// Local reason editing; only a confirmed backend reply may change task status.
pub struct RejectController {
    pub view: Reject,
    editor: crate::text_edit::Editor,
}
impl Default for RejectController {
    fn default() -> Self {
        Self::new(String::new())
    }
}
impl RejectController {
    pub fn new(reason: String) -> Self {
        let editor = crate::text_edit::Editor::new(&reason);
        let edit = Some(editor.edit());
        Self {
            view: Reject {
                reason,
                edit,
                notice: None,
            },
            editor,
        }
    }
    pub fn composing(&self) -> bool {
        self.editor.composing()
    }
    pub fn focus(&mut self, focused: bool) {
        if !focused {
            self.editor.handle(
                &mut self.view.reason,
                crate::text_edit::Command::CancelPreedit,
            );
        }
        self.view.edit = focused.then(|| self.editor.edit());
    }
    pub fn handle(&mut self, command: crate::text_edit::Command) -> bool {
        let changed = self.editor.handle(&mut self.view.reason, command);
        self.view.edit = Some(self.editor.edit());
        changed
    }
    pub fn submission(&self) -> Result<String, &'static str> {
        if self.composing() {
            return Err("preedit_pending");
        }
        if self.view.reason.trim().is_empty() {
            return Err("reject_reason_required");
        }
        Ok(self.view.reason.clone())
    }
    pub fn refused(&mut self, reason: impl Into<String>) {
        self.view.notice = Some(super::Notice {
            kind: "refused".into(),
            text: reason.into(),
        });
    }
}
struct Action<'a> {
    id: &'a str,
    label: &'a str,
    fill: Option<ui::Color>,
    enabled: bool,
    weight: u16,
}
fn actions(key: &str, padding: [f32; 4], specs: &[Action<'_>]) -> c::Element {
    let mut buttons = vec![spacer()];
    for action in specs {
        let mut s = plain(
            None,
            [2., STYLE.spacing.s5 as f32, 2., STYLE.spacing.s5 as f32],
            STYLE.radius.pill as f32,
        );
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: if action.fill.is_some() {
                ui::Paint::solid(ui::Color::Transparent)
            } else {
                ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                }
            },
            outside: false,
        });
        if let Some(fill) = action.fill {
            s.overrides.fill = Some(ui::Paint::solid(fill));
        }
        if !action.enabled {
            s.overrides.opacity = Some(0.4);
        }
        buttons.push(
            row(
                format!("{key}/{}", action.id),
                s,
                0.,
                c::Alignment::Center,
                vec![
                    label(
                        format!("{key}/{}/text", action.id),
                        action.label,
                        text_style(
                            STYLE.r#type.caption as f32,
                            action.weight,
                            false,
                            if action.fill.is_some() {
                                ui::Color::Rgba([1.; 4])
                            } else {
                                ui::Color::Foreground
                            },
                        ),
                        false,
                    )
                    .into(),
                ],
            )
            .into(),
        );
    }
    row(
        format!("{key}/actions"),
        plain(None, padding, 0.),
        STYLE.spacing.s3 as f32,
        c::Alignment::Center,
        buttons,
    )
}
fn line(padding: [f32; 4]) -> ui::Style {
    let mut s = plain(None, padding, 0.);
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint {
            color: ui::Color::Line,
            alpha: ui::Alpha::Hairline,
        },
        outside: false,
    });
    s.overrides.border_sides = Some([false, false, true, false]);
    s
}
impl Report {
    pub fn can_accept(&self) -> bool {
        self.status == "reported"
            && self
                .verify_result
                .as_ref()
                .is_some_and(|v| v.exit_code == 0)
    }
    pub fn element(&self, ctx: ui::Context, key: &str, review: bool) -> c::Element {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let status = if self.status.is_empty() {
            "reported"
        } else {
            &self.status
        };
        let ok = self
            .verify_result
            .as_ref()
            .is_some_and(|v| v.exit_code == 0);
        let good = rgba(TOKENS.color.ui.ok);
        let bad = rgba(TOKENS.color.state.errored);
        let waiting = rgba(TOKENS.color.state.waiting);
        let mut children = vec![
            row(
                format!("{key}/head"),
                line([sp.s3 as f32, sp.s4 as f32, sp.s3 as f32, sp.s4 as f32]),
                sp.s3 as f32,
                c::Alignment::Center,
                vec![
                    super::view::task_mark(
                        &format!("{key}/glyph"),
                        status,
                        11.,
                        crate::modules::seat_row::role_color("executor"),
                    )
                    .into(),
                    fill(label(
                        format!("{key}/title"),
                        &self.task,
                        text_style(t.caption as f32, 600, false, ui::Color::Foreground),
                        false,
                    )),
                    badge(
                        &format!("{key}/status"),
                        status,
                        text_style(
                            t.micro as f32,
                            400,
                            false,
                            if status == "accepted" {
                                ui::Color::Rgba([1.; 4])
                            } else {
                                ui::Color::Muted
                            },
                        ),
                        (status == "accepted").then_some(ui::Paint::solid(good)),
                        Some(if status == "accepted" {
                            ui::Paint::solid(ui::Color::Transparent)
                        } else {
                            ui::Paint {
                                color: ui::Color::Line,
                                alpha: ui::Alpha::Hairline,
                            }
                        }),
                        (STYLE.radius.pill as f32, sp.s2 as f32, None, false),
                    )
                    .into(),
                ],
            )
            .into(),
        ];
        let mut summary = text_style(t.small as f32, 400, false, ui::Color::Foreground);
        summary.line_height = 1.45;
        children.push(
            column(
                format!("{key}/summary"),
                line([sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32]),
                vec![label(format!("{key}/summary/text"), &self.summary, summary, true).into()],
            )
            .into(),
        );
        if let Some(v) = &self.verify_result {
            let mut checks = vec![
                row(
                    format!("{key}/verify"),
                    plain(None, [0.; 4], 0.),
                    sp.s3 as f32,
                    c::Alignment::Center,
                    vec![
                        c::Item {
                            element: label(
                                format!("{key}/verify/mark"),
                                if ok { "✓" } else { "✕" },
                                text_style(t.small as f32, 400, false, if ok { good } else { bad }),
                                false,
                            ),
                            extent: c::Extent::Fixed(12.),
                        },
                        fill(label(
                            format!("{key}/verify/command"),
                            &v.command,
                            text_style(t.small as f32, 400, true, ui::Color::Foreground),
                            false,
                        )),
                        label(
                            format!("{key}/verify/exit"),
                            format!("exit {}", v.exit_code),
                            text_style(t.small as f32, 400, true, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            ];
            for (i, e) in v.evidence.iter().enumerate() {
                let k = format!("{key}/evidence{i}");
                let mut b = c::Box::new(
                    plain(None, [0., 0., 0., 20.], 0.),
                    vec![
                        label(
                            format!("{k}/criterion"),
                            &e.criterion,
                            text_style(t.small as f32, 400, false, ui::Color::Muted),
                            true,
                        )
                        .into(),
                        label(
                            format!("{k}/reference"),
                            &e.reference,
                            text_style(t.small as f32, 400, true, ui::Color::Foreground),
                            false,
                        )
                        .into(),
                    ],
                );
                b.direction = Axis::Column;
                b.gap = ui::Length::Px(1.);
                b.align = c::Alignment::Stretch;
                checks.push(named(k, c::Element::Box(b)).into());
            }
            let mut b = c::Box::new(
                line([sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32]),
                checks,
            );
            b.direction = Axis::Column;
            b.gap = ui::Length::Spacing(1);
            b.align = c::Alignment::Stretch;
            children.push(named(format!("{key}/checks"), c::Element::Box(b)).into());
        } else {
            children.push(
                column(
                    format!("{key}/gap"),
                    line([sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32]),
                    vec![
                        label(
                            format!("{key}/gap/text"),
                            "no verify_result · cannot be accepted",
                            text_style(t.small as f32, 400, false, waiting),
                            true,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        if let Some(commit) = &self.commit {
            children.push(
                column(
                    format!("{key}/commit"),
                    plain(
                        None,
                        [sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32],
                        0.,
                    ),
                    vec![
                        label(
                            format!("{key}/commit/text"),
                            commit,
                            ui::Text {
                                line_height: 1.5,
                                ..text_style(t.small as f32, 400, true, ui::Color::Foreground)
                            },
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        if (status == "reported" && review) || self.reject.is_some() {
            if let Some(reject) = &self.reject {
                let k = format!("{key}/reject");
                let mut label_text = c::Text::new(
                    "Reject · reason ",
                    text_style(t.small as f32, 600, false, ui::Color::Foreground),
                );
                label_text.runs.push(c::TextRun {
                    value: "required".into(),
                    style: text_style(t.small as f32, 400, false, waiting),
                });
                let input_key = format!("{k}/input");
                let mut input = crate::widgets::Input {
                    value: reject.reason.clone(),
                    placeholder: "what to change · sent to the seat".into(),
                    state: if reject.reason.is_empty() {
                        crate::widgets::InputState::Empty
                    } else {
                        crate::widgets::InputState::Value
                    },
                    icon: None,
                    error: None,
                    edit: reject.edit.clone(),
                }
                .unresolved();
                super::view::scope_names(&mut input, "input", &input_key);
                let mut fields = vec![
                    named(format!("{k}/label"), c::Element::Text(label_text)).into(),
                    named(&input_key, input).into(),
                ];
                if let Some(n) = &reject.notice {
                    fields.push(super::view::notice_line(&format!("{k}/notice"), n, true).into());
                }
                fields.push(
                    actions(
                        &k,
                        [0., 0., sp.s3 as f32, 0.],
                        &[
                            Action {
                                id: "cancel",
                                label: "Cancel",
                                fill: None,
                                enabled: true,
                                weight: 500,
                            },
                            Action {
                                id: "submit",
                                label: "Reject",
                                fill: Some(bad),
                                enabled: review
                                    && status == "reported"
                                    && !reject.reason.trim().is_empty(),
                                weight: 600,
                            },
                        ],
                    )
                    .into(),
                );
                let mut s = plain(None, [sp.s3 as f32, sp.s4 as f32, 0., sp.s4 as f32], 0.);
                s.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: ui::Paint {
                        color: ui::Color::Line,
                        alpha: ui::Alpha::Hairline,
                    },
                    outside: false,
                });
                s.overrides.border_sides = Some([true, false, false, false]);
                let mut b = c::Box::new(s, fields);
                b.direction = Axis::Column;
                b.gap = ui::Length::Spacing(3);
                b.align = c::Alignment::Stretch;
                children.push(named(k, c::Element::Box(b)).into());
            } else {
                children.push(
                    actions(
                        key,
                        [sp.s3 as f32, sp.s4 as f32, sp.s3 as f32, sp.s4 as f32],
                        &[
                            Action {
                                id: "reject",
                                label: "Reject…",
                                fill: None,
                                enabled: true,
                                weight: 500,
                            },
                            Action {
                                id: "accept",
                                label: "Accept",
                                fill: Some(good),
                                enabled: ok,
                                weight: 600,
                            },
                        ],
                    )
                    .into(),
                );
            }
        }
        let mut s = plain(None, [0.; 4], STYLE.radius.row as f32);
        s.overrides.fill = Some(tint(ui::Color::Foreground, 0.04));
        s.overrides.rings = Some(vec![ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        }]);
        // Glow.blocks gives every report the kit's idle rim, outside its clipped
        // contents. Its own padding is zero, so bloom never reaches the text.
        s.overrides.rim = Some(ui::Rim::block(
            ui::GlowActivity::Idle,
            false,
            Some(
                ui::Paint::solid(ui::Color::Accent)
                    .resolve(ctx)
                    .expect("accent token"),
            ),
            ui::rim_phase(&self.task),
            false,
            TOKENS.glow.rim as f32 * 0.5,
            0.,
        ));
        let mut panel = c::Box::new(s, children);
        panel.direction = Axis::Column;
        panel.gap = ui::Length::Px(0.);
        panel.align = c::Alignment::Stretch;
        panel.clip = true;
        column(
            format!("{key}/margin"),
            plain(None, [sp.s3 as f32, 0., 0., 0.], 0.),
            vec![named(format!("{key}/wrapper"), named(key, c::Element::Box(panel))).into()],
        )
    }
}
