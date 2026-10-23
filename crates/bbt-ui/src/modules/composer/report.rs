use crate::modules::{
    seat_row::named,
    stream_dock::{badge, column, fill, label, rgba, row, spacer, text_style, tint},
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, layout::Axis};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Verification {
    pub command: String,
    pub exit_code: i32,
    pub evidence: Vec<Criterion>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Criterion {
    pub criterion: String,
    pub reference: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Report {
    pub task: String,
    pub status: String,
    pub summary: String,
    pub commit: Option<String>,
    pub verify_result: Option<Verification>,
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
    pub fn element(&self, key: &str, review: bool) -> c::Element {
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
                    label(
                        format!("{key}/glyph"),
                        super::view::task_glyph(status),
                        text_style(
                            t.caption as f32,
                            400,
                            false,
                            crate::modules::seat_row::role_color("executor"),
                        ),
                        false,
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
        if status == "reported" && review {
            let mut buttons = vec![spacer()];
            for (action, value, col, background) in [
                ("reject", "Re-brief", ui::Color::Foreground, None),
                ("accept", "Accept", ui::Color::Rgba([1.; 4]), Some(good)),
            ] {
                let mut s = plain(
                    None,
                    [2., sp.s5 as f32, 2., sp.s5 as f32],
                    STYLE.radius.pill as f32,
                );
                s.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: if background.is_some() {
                        ui::Paint::solid(ui::Color::Transparent)
                    } else {
                        ui::Paint {
                            color: ui::Color::Line,
                            alpha: ui::Alpha::Hairline,
                        }
                    },
                    outside: false,
                });
                if let Some(background) = background {
                    s.overrides.fill = Some(ui::Paint::solid(background));
                }
                if action == "accept" && !ok {
                    s.overrides.opacity = Some(0.4);
                }
                buttons.push(
                    row(
                        format!("{key}/{action}"),
                        s,
                        0.,
                        c::Alignment::Center,
                        vec![
                            label(
                                format!("{key}/{action}/text"),
                                value,
                                text_style(
                                    t.caption as f32,
                                    if action == "reject" { 500 } else { 600 },
                                    false,
                                    col,
                                ),
                                false,
                            )
                            .into(),
                        ],
                    )
                    .into(),
                );
            }
            children.push(
                row(
                    format!("{key}/actions"),
                    plain(
                        None,
                        [sp.s3 as f32, sp.s4 as f32, sp.s3 as f32, sp.s4 as f32],
                        0.,
                    ),
                    sp.s3 as f32,
                    c::Alignment::Center,
                    buttons,
                )
                .into(),
            );
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
        let mut panel = c::Box::new(s, children);
        panel.direction = Axis::Column;
        panel.gap = ui::Length::Px(0.);
        panel.align = c::Alignment::Stretch;
        panel.clip = true;
        column(
            format!("{key}/margin"),
            plain(None, [sp.s3 as f32, 0., 0., 0.], 0.),
            vec![named(key, c::Element::Box(panel)).into()],
        )
    }
}
