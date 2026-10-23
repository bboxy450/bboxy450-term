use super::*;
use crate::modules::{
    seat_row::{named, role_color},
    stream_dock::{column, fill, label, rgba, row, shrink, spacer, text_style, tint},
    strip::plain,
};
use crate::{self as ui, components as c, widgets::text_area::TextArea};
use bbt_ui_model::{
    STYLE, TOKENS,
    layout::{Axis, Rect},
};

fn stack(key: &str, style: ui::Style, axis: Axis, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = axis;
    b.gap = ui::Length::Px(gap);
    b.align = if axis == Axis::Column {
        c::Alignment::Stretch
    } else {
        c::Alignment::Center
    };
    named(key, c::Element::Box(b))
}
fn mark(role: &str) -> &'static str {
    match role {
        "oracle" => "♔",
        "architect" => "♖",
        "executor" => "♟",
        "creativity" => "♝",
        _ => "",
    }
}
fn color(value: Option<&str>) -> ui::Color {
    match value {
        Some(value) if value.starts_with('#') => rgba(value),
        Some("var(--role-executor)") => role_color("executor"),
        Some("var(--role-oracle)") => role_color("oracle"),
        Some("var(--role-creativity)") => role_color("creativity"),
        Some("var(--role-architect)") => role_color("architect"),
        _ => ui::Color::Accent,
    }
}
fn dot(key: &str, size: f32, paint: ui::Color) -> c::Element {
    let mut s = plain(Some(size), [0.; 4], size * 0.5);
    s.overrides.width = Some(ui::Length::Px(size));
    s.overrides.fill = Some(ui::Paint::solid(paint));
    named(key, c::Element::Box(c::Box::new(s, vec![])))
}
pub fn task_glyph(status: &str) -> &'static str {
    match status {
        "claimed" => "◔",
        "running" => "◑",
        "reported" => "◕",
        "accepted" => "●",
        "done" => "✓",
        "rejected" => "✕",
        "blocked" => "⊘",
        _ => "○",
    }
}
fn artifact_glyph(kind: &str) -> &'static str {
    match kind {
        "markdown" => "▤",
        "pr" => "⎇",
        "binary" => "▣",
        "image" => "▧",
        "release" => "◆",
        _ => "▢",
    }
}
fn spinner() -> c::Element {
    let arc = |start: f32, turns: usize, paint| {
        let point = |a: f32| [5. + 4.25 * a.cos(), 5. + 4.25 * a.sin()];
        let mut points = vec![c::PathCommand::Move(point(start))];
        let k = 4. / 3. * (std::f32::consts::FRAC_PI_2 / 4.).tan() * 4.25;
        for i in 0..turns {
            let a = start + i as f32 * std::f32::consts::FRAC_PI_2;
            let b = a + std::f32::consts::FRAC_PI_2;
            let (p, q) = (point(a), point(b));
            points.push(c::PathCommand::Cubic([
                [p[0] - k * a.sin(), p[1] + k * a.cos()],
                [q[0] + k * b.sin(), q[1] - k * b.cos()],
                q,
            ]));
        }
        c::Element::Path(c::Path {
            size: [10., 10.],
            points: points.into(),
            stroke: c::PathStroke {
                width: 1.5,
                ..Default::default()
            },
            paint,
            fill: None,
            opacity: 1.,
        })
    };
    let mut slot = plain(Some(10.), [0.; 4], 0.);
    slot.overrides.width = Some(ui::Length::Px(12.));
    named(
        "composer/notice/mark",
        c::Element::Layers {
            base: Box::new(c::Element::Box(c::Box::new(slot, vec![]))),
            overlays: vec![
                c::Placed {
                    content: arc(
                        -std::f32::consts::PI * 0.75,
                        4,
                        tint(ui::Color::Foreground, 0.25),
                    ),
                    left: 1.,
                    bottom: 0.,
                },
                c::Placed {
                    content: arc(
                        -std::f32::consts::PI * 0.75,
                        1,
                        ui::Paint::solid(ui::Color::Foreground),
                    ),
                    left: 1.,
                    bottom: 0.,
                },
            ],
        },
    )
}
impl Address {
    pub fn element(&self, key: &str) -> c::Element {
        let sp = &STYLE.spacing;
        let caption = STYLE.r#type.caption as f32;
        let mut cells = vec![];
        if self.kind == "seat" {
            cells.push(dot(&format!("{key}/dot"), 7., color(self.color.as_deref())).into());
        }
        if self.kind == "role" {
            cells.push(
                label(
                    format!("{key}/rm"),
                    mark(&self.role),
                    text_style(
                        caption,
                        400,
                        false,
                        self.color
                            .as_deref()
                            .map_or(ui::Color::Muted, |v| color(Some(v))),
                    ),
                    false,
                )
                .into(),
            );
        }
        let prefix = match self.kind.as_str() {
            "channel" => "#",
            "subagent" => "↳ ",
            _ => "",
        };
        let parent = if self.parent.is_empty() {
            String::new()
        } else {
            format!("{} ▸ ", self.parent)
        };
        cells.push(shrink(label(
            format!("{key}/text"),
            format!("{prefix}{parent}{}", self.label),
            text_style(caption, 400, self.kind == "channel", ui::Color::Foreground),
            false,
        )));
        if let Some(n) = self.n {
            cells.push(
                label(
                    format!("{key}/n"),
                    n.to_string(),
                    text_style(STYLE.r#type.micro as f32, 400, false, ui::Color::Muted),
                    false,
                )
                .into(),
            );
        }
        cells.push(
            row(
                format!("{key}/x"),
                plain(None, [0., 2., 0., 2.], 0.),
                0.,
                c::Alignment::Center,
                vec![
                    label(
                        format!("{key}/x/text"),
                        "×",
                        text_style(
                            STYLE.r#type.small as f32,
                            400,
                            self.kind == "channel",
                            ui::Color::Muted,
                        ),
                        false,
                    )
                    .into(),
                ],
            )
            .into(),
        );
        let mut s = plain(
            Some(20.),
            [0., sp.s2 as f32, 0., sp.s3 as f32],
            STYLE.radius.pill as f32,
        );
        s.overrides.fill = Some(tint(ui::Color::Foreground, STYLE.tint.badge_alpha as f32));
        if self.kind == "subagent" {
            s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
            s.overrides.dashed = Some(true);
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: tint(ui::Color::Muted, 0.55),
                outside: false,
            });
        }
        row(key, s, sp.s2 as f32, c::Alignment::Center, cells)
    }
}
impl Attachment {
    pub fn element(&self, key: &str) -> c::Element {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let glyph = if self.kind == "task" {
            task_glyph(&self.status)
        } else {
            artifact_glyph(&self.akind)
        };
        let mut s = plain(
            None,
            [sp.s2 as f32, sp.s3 as f32, sp.s2 as f32, sp.s3 as f32],
            STYLE.radius.row as f32,
        );
        s.overrides.fill = Some(tint(ui::Color::Foreground, 0.05));
        s.overrides.rings = Some(vec![ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        }]);
        row(
            key,
            s,
            sp.s3 as f32,
            c::Alignment::Center,
            vec![
                c::Item {
                    element: label(
                        format!("{key}/ak"),
                        glyph,
                        text_style(t.caption as f32, 400, false, ui::Color::Muted),
                        false,
                    ),
                    extent: c::Extent::Fixed(14.),
                },
                fill(label(
                    format!("{key}/at"),
                    &self.title,
                    text_style(t.caption as f32, 400, false, ui::Color::Foreground),
                    false,
                )),
                label(
                    format!("{key}/meta"),
                    if self.status.is_empty() {
                        &self.meta
                    } else {
                        &self.status
                    },
                    text_style(t.small as f32, 400, false, ui::Color::Muted),
                    false,
                )
                .into(),
            ],
        )
    }
}
impl Composer {
    pub fn element_named(
        &self,
        key: &str,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        fn scope(e: &mut c::Element, key: &str) {
            match e {
                c::Element::Named { name, content } => {
                    if let Some(tail) = name.strip_prefix("composer") {
                        *name = format!("{key}{tail}");
                    }
                    scope(content, key);
                }
                c::Element::Box(b) => {
                    for item in &mut b.children {
                        scope(&mut item.element, key);
                    }
                }
                c::Element::Stack(b) => {
                    for item in &mut b.children {
                        scope(&mut item.element, key);
                    }
                }
                c::Element::Flow(b) => {
                    for item in &mut b.children {
                        scope(&mut item.element, key);
                    }
                }
                c::Element::Layers { base, overlays } => {
                    scope(base, key);
                    for p in overlays {
                        scope(&mut p.content, key);
                    }
                }
                c::Element::ScrollView(s) => scope(&mut s.content, key),
                _ => {}
            }
        }
        let mut e = self.element(ctx, width, m)?;
        scope(&mut e, key);
        Ok(e)
    }
    pub fn element(
        &self,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let inner = (width - 2. * sp.s4 as f32).max(1.);
        let delegate = self.mode == "delegate";
        let accent = if delegate {
            role_color("executor")
        } else {
            ui::Color::Accent
        };
        let mut children = vec![];
        if let Some(n) = &self.notice {
            let col = match n.kind.as_str() {
                "held" => rgba(TOKENS.color.state.waiting),
                "refused" => rgba(TOKENS.color.state.errored),
                _ => ui::Color::Muted,
            };
            let mut s = plain(
                None,
                [sp.s2 as f32, sp.s5 as f32, sp.s2 as f32, sp.s5 as f32],
                0.,
            );
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
            s.overrides.border_sides = Some([false, false, true, false]);
            let mark = match n.kind.as_str() {
                "sent" => "✓",
                "held" => "◷",
                "refused" => "⊘",
                _ => "◌",
            };
            children.push(
                row(
                    "composer/notice",
                    s,
                    sp.s3 as f32,
                    c::Alignment::Center,
                    vec![
                        c::Item {
                            element: if n.kind == "pending" {
                                spinner()
                            } else {
                                label(
                                    "composer/notice/mark",
                                    mark,
                                    text_style(
                                        t.small as f32,
                                        400,
                                        false,
                                        if n.kind == "sent" {
                                            rgba(TOKENS.color.ui.ok)
                                        } else {
                                            col
                                        },
                                    ),
                                    false,
                                )
                            },
                            extent: c::Extent::Fixed(12.),
                        },
                        c::Item {
                            element: label(
                                "composer/notice/text",
                                &n.text,
                                text_style(t.small as f32, 400, false, col),
                                false,
                            ),
                            extent: c::Extent::Content {
                                grow: 0.,
                                shrink: 1.,
                                min: 0.,
                                max: None,
                                max_fraction: None,
                            },
                        },
                    ],
                )
                .into(),
            );
        }
        let mut cells = vec![
            row(
                "composer/address/label",
                plain(None, [0., sp.s1 as f32, 0., 0.], 0.),
                0.,
                c::Alignment::Center,
                vec![
                    label(
                        "composer/address/label/text",
                        if delegate { "Delegate to" } else { "To" },
                        text_style(t.caption as f32, 400, false, ui::Color::Muted),
                        false,
                    )
                    .into(),
                ],
            )
            .into(),
        ];
        cells.extend(
            self.to
                .iter()
                .enumerate()
                .map(|(i, a)| a.element(&format!("composer/address/a{i}")).into()),
        );
        cells.push(
            label(
                "composer/address/typed",
                &self.typed,
                text_style(t.caption as f32, 400, true, ui::Color::Foreground),
                false,
            )
            .into(),
        );
        let flow = c::Element::Flow(c::Flow {
            children: cells,
            wrap: true,
            justify: c::Justify::Start,
            align: c::Alignment::Center,
            row_gap: ui::Length::Spacing(2),
            column_gap: ui::Length::Spacing(2),
        });
        let address_h = flow
            .layout(Rect::new(0., 0., inner, 0.), ctx, m)?
            .rect
            .h
            .max(20.);
        let mut address = c::Box::new(plain(Some(address_h), [0.; 4], 0.), vec![fill(flow)]);
        address.gap = ui::Length::Px(0.);
        children.push(named("composer/address", c::Element::Box(address)).into());
        if !self.attach.is_empty() {
            children.push(
                stack(
                    "composer/attachments",
                    plain(None, [0.; 4], 0.),
                    Axis::Column,
                    sp.s2 as f32,
                    self.attach
                        .iter()
                        .enumerate()
                        .map(|(i, a)| a.element(&format!("composer/attachments/a{i}")).into())
                        .collect(),
                )
                .into(),
            );
        }
        let ph = if delegate {
            "Brief, or leave empty to send the task as written".into()
        } else {
            format!(
                "Message {}",
                self.to.first().map_or_else(
                    || "the swarm".into(),
                    |a| format!(
                        "{}{}",
                        match a.kind.as_str() {
                            "channel" => "#",
                            "subagent" => "↳ ",
                            _ => "",
                        },
                        a.label
                    )
                )
            )
        };
        let mut style = text_style(t.caption as f32, 400, false, ui::Color::Foreground);
        style.line_height = 1.45;
        children.push(
            TextArea {
                value: self.text.clone(),
                placeholder: ph,
                style,
                edit: self.focus.then(|| self.edit.clone().unwrap_or_default()),
                min_lines: 2,
                name: "composer/text".into(),
            }
            .element(inner, ctx, m)?
            .into(),
        );
        let icon = row(
            "composer/attach",
            plain(None, [3.; 4], STYLE.radius.tag as f32),
            0.,
            c::Alignment::Center,
            vec![
                named(
                    "composer/attach/icon",
                    c::Element::Icon {
                        name: "archive".into(),
                        size: ui::Length::Px(13.),
                        paint: ui::Paint::solid(ui::Color::Muted),
                    },
                )
                .into(),
            ],
        );
        let mut foot = vec![
            icon.into(),
            c::Item {
                element: label(
                    "composer/hint",
                    "[[ attach · @ # ↳ address",
                    text_style(t.micro as f32, 400, false, ui::Color::Muted),
                    false,
                ),
                extent: c::Extent::Content {
                    grow: 0.,
                    shrink: 1.,
                    min: 0.,
                    max: None,
                    max_fraction: None,
                },
            },
            spacer(),
        ];
        let kind = if self.kind.is_empty() {
            "text"
        } else {
            &self.kind
        };
        foot.push(named("composer/kpick", kind_chip("composer/kind", kind, true)).into());
        let mut pip = plain(
            None,
            [1., sp.s2 as f32, 1., sp.s2 as f32],
            STYLE.radius.pill as f32,
        );
        pip.overrides.fill = Some(ui::Paint::solid(rgba(TOKENS.color.state.errored)));
        let mut send = plain(None, [3., sp.s4 as f32, 3., 3.], STYLE.radius.pill as f32);
        send.overrides.fill = Some(ui::Paint::solid(accent));
        foot.push(
            row(
                "composer/send",
                send,
                sp.s2 as f32,
                c::Alignment::Center,
                vec![
                    row(
                        "composer/priority",
                        pip,
                        0.,
                        c::Alignment::Center,
                        vec![
                            label(
                                "composer/priority/text",
                                self.priority.to_string(),
                                text_style(t.micro as f32, 500, true, ui::Color::Rgba([1.; 4])),
                                false,
                            )
                            .into(),
                        ],
                    )
                    .into(),
                    label(
                        "composer/send/text",
                        if delegate { "Delegate" } else { "Send" },
                        text_style(t.caption as f32, 600, false, ui::Color::OnAccent),
                        false,
                    )
                    .into(),
                    row(
                        "composer/send/kbd",
                        {
                            let mut s = plain(None, [1., 5., 1., 5.], STYLE.radius.tag as f32);
                            s.overrides.fill = Some(ui::Paint {
                                color: ui::Color::Foreground,
                                alpha: ui::Alpha::Badge,
                            });
                            s.overrides.opacity = Some(0.75);
                            s
                        },
                        0.,
                        c::Alignment::Center,
                        vec![
                            label(
                                "composer/send/kbd/text",
                                "⌘↵",
                                ui::Text {
                                    line_height: 14. / t.micro as f32,
                                    ..text_style(t.micro as f32, 600, true, ui::Color::OnAccent)
                                },
                                false,
                            )
                            .into(),
                        ],
                    )
                    .into(),
                ],
            )
            .into(),
        );
        children.push(
            row(
                "composer/foot",
                plain(None, [0.; 4], 0.),
                sp.s3 as f32,
                c::Alignment::Center,
                foot,
            )
            .into(),
        );
        let mut s = plain(
            None,
            [sp.s3 as f32, sp.s4 as f32, sp.s3 as f32, sp.s4 as f32],
            STYLE.radius.card as f32,
        );
        s.overrides.width = Some(ui::Length::Px(width));
        s.overrides.fill = Some(tint(ui::Color::Surface, STYLE.material.card_alpha as f32));
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: if self.focus {
                ui::Paint::solid(accent)
            } else {
                ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                }
            },
            outside: true,
        });
        if self.focus {
            s.overrides.rings = Some(vec![ui::Border {
                width: ui::Length::Px(3.),
                paint: tint(accent, 0.22),
                outside: true,
            }]);
        }
        let panel = stack("composer", s, Axis::Column, sp.s3 as f32, children);
        let panel = if self.pick.is_empty() && !self.kind_menu {
            panel
        } else {
            let height = panel.layout(Rect::new(0., 0., width, 0.), ctx, m)?.rect.h;
            let overlay = if self.kind_menu {
                let (menu, menu_width) = kind_menu(kind, width - 2. * sp.s4 as f32, m);
                c::Placed {
                    content: menu,
                    left: width - sp.s4 as f32 - menu_width,
                    bottom: height + 6.,
                }
            } else {
                c::Placed {
                    content: self.picker(ctx, width),
                    left: 0.,
                    bottom: height + sp.s3 as f32,
                }
            };
            c::Element::Layers {
                base: Box::new(panel),
                overlays: vec![overlay],
            }
        };
        // Margin belongs to the constructor so both inline hosts agree.
        Ok(column(
            "composer-margin",
            plain(None, [sp.s3 as f32, 0., 0., 0.], 0.),
            vec![panel.into()],
        ))
    }
    fn picker(&self, _ctx: ui::Context, width: f32) -> c::Element {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let mut children = vec![];
        let mut index = 0;
        for (gi, g) in self.pick.iter().enumerate() {
            let key = format!("composer/pick/g{gi}");
            let mut items = vec![
                row(
                    format!("{key}/head"),
                    plain(
                        None,
                        [sp.s2 as f32, sp.s3 as f32, sp.s1 as f32, sp.s3 as f32],
                        0.,
                    ),
                    0.,
                    c::Alignment::Center,
                    vec![
                        label(
                            format!("{key}/head/text"),
                            &g.title,
                            text_style(t.micro as f32, 400, false, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            ];
            for a in &g.items {
                let k = format!("composer/pick/o{index}");
                index += 1;
                let mut s = plain(
                    Some(24.),
                    [0., sp.s3 as f32, 0., sp.s3 as f32],
                    STYLE.radius.row as f32,
                );
                if a.sel {
                    s.overrides.fill = Some(ui::Paint {
                        color: ui::Color::Foreground,
                        alpha: ui::Alpha::Selection,
                    });
                }
                let glyph = if a.kind == "seat" {
                    dot(&format!("{k}/dot"), 8., color(a.color.as_deref()))
                } else {
                    label(
                        format!("{k}/ok"),
                        match a.kind.as_str() {
                            "role" => mark(&a.role),
                            "channel" => "#",
                            _ => "↳",
                        },
                        text_style(t.caption as f32, 400, false, ui::Color::Muted),
                        false,
                    )
                };
                let mut cells = vec![
                    c::Item {
                        element: if a.kind == "seat" {
                            row(
                                "",
                                plain(None, [0.; 4], 0.),
                                0.,
                                c::Alignment::Center,
                                vec![spacer(), glyph.into(), spacer()],
                            )
                        } else {
                            glyph
                        },
                        extent: c::Extent::Fixed(14.),
                    },
                    fill(label(
                        format!("{k}/label"),
                        &a.label,
                        text_style(t.caption as f32, 400, false, ui::Color::Foreground),
                        false,
                    )),
                ];
                if !a.sub.is_empty() {
                    cells.push(
                        label(
                            format!("{k}/sub"),
                            &a.sub,
                            text_style(t.micro as f32, 400, false, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    );
                }
                if let Some(load) = a.load {
                    cells.push(
                        label(
                            format!("{k}/load"),
                            if load == 0 {
                                "free".into()
                            } else {
                                format!("{load} running")
                            },
                            text_style(
                                t.micro as f32,
                                400,
                                false,
                                if load == 0 {
                                    rgba(TOKENS.color.ui.ok)
                                } else {
                                    ui::Color::Muted
                                },
                            ),
                            false,
                        )
                        .into(),
                    );
                }
                while cells.len() < 4 {
                    cells.push(c::Item {
                        element: c::Element::Spacer,
                        extent: c::Extent::Fixed(0.),
                    });
                }
                items.push(row(&k, s, sp.s3 as f32, c::Alignment::Center, cells).into());
            }
            children.push(column(key, plain(None, [0.; 4], 0.), items).into());
        }
        let mut s = ui::Style::new(ui::Preset::Modal);
        s.overrides.width = Some(ui::Length::Px(width));
        s.overrides.padding = Some([ui::Length::Spacing(2); 4]);
        s.overrides.radii = Some([ui::Length::Px(STYLE.radius.card as f32); 4]);
        let panel = stack("composer/pick", s, Axis::Column, 0., children);
        // Positioned relative to the composer by its host after retained layout.
        panel
    }
}

fn kind_chip(key: &str, kind: &str, dropdown: bool) -> c::Element {
    let color = match kind {
        "chunk" => rgba(TOKENS.color.ui.ok),
        "decision" => rgba(TOKENS.color.ui.invariant),
        _ => ui::Color::Muted,
    };
    let h = if dropdown { 16. } else { 14. };
    let mut ts = text_style(STYLE.r#type.micro as f32, 400, false, color);
    ts.uppercase = true;
    ts.tracking_em = 0.08;
    ts.line_height = h / STYLE.r#type.micro as f32;
    let mut cells = vec![label(format!("{key}/text"), kind, ts, false).into()];
    if dropdown {
        cells.push(
            label(
                format!("{key}/chevron"),
                "▾",
                ui::Text {
                    paint: tint(color, 0.7),
                    ..ts
                },
                false,
            )
            .into(),
        );
    }
    let mut s = plain(Some(h), [0., 5., 0., 5.], STYLE.radius.tag as f32);
    s.overrides.fill = Some(tint(color, if kind == "text" { 0.12 } else { 0.14 }));
    row(
        key,
        s,
        if dropdown { 3. } else { 0. },
        c::Alignment::Center,
        cells,
    )
}
fn kind_menu(current: &str, available: f32, m: &dyn c::Measure) -> (c::Element, f32) {
    use bbt_ui_model::layout::TextStyle;
    let sp = &STYLE.spacing;
    let t = &STYLE.r#type;
    let desc = [
        "a message",
        "progress on the stream",
        "a decision on the stream",
    ];
    let width = SEND_KINDS
        .iter()
        .zip(desc)
        .map(|(kind, desc)| {
            m.width(
                &kind.to_uppercase(),
                &TextStyle {
                    size: t.micro as f32,
                    weight: 400,
                    mono: false,
                    italic: false,
                    tracking_em: 0.08,
                },
            ) + 10.
                + m.width(
                    desc,
                    &TextStyle {
                        size: t.small as f32,
                        weight: 400,
                        mono: false,
                        italic: false,
                        tracking_em: 0.,
                    },
                )
                + 12.
                + 4. * sp.s3 as f32
                + 2. * sp.s2 as f32
        })
        .fold(210_f32, f32::max)
        .min(available);
    let mut children = vec![];
    for (i, (kind, desc)) in SEND_KINDS.iter().zip(desc).enumerate() {
        let key = format!("composer/kind-menu/k{i}");
        let mut s = plain(
            None,
            [sp.s2 as f32, sp.s3 as f32, sp.s2 as f32, sp.s3 as f32],
            STYLE.radius.row as f32,
        );
        if *kind == current {
            s.overrides.fill = Some(ui::Paint {
                color: ui::Color::Foreground,
                alpha: ui::Alpha::Selection,
            });
        }
        children.push(
            row(
                &key,
                s,
                sp.s3 as f32,
                c::Alignment::Center,
                vec![
                    c::Item {
                        element: label(
                            format!("{key}/mark"),
                            if *kind == current { "✓" } else { "" },
                            text_style(t.small as f32, 400, false, ui::Color::Accent),
                            false,
                        ),
                        extent: c::Extent::Fixed(12.),
                    },
                    kind_chip(&format!("{key}/chip"), kind, false).into(),
                    fill(label(
                        format!("{key}/description"),
                        desc,
                        text_style(t.small as f32, 400, false, ui::Color::Muted),
                        false,
                    )),
                ],
            )
            .into(),
        );
    }
    let mut style = plain(None, [sp.s2 as f32; 4], STYLE.radius.card as f32);
    style.overrides.width = Some(ui::Length::Px(width));
    style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
    style.overrides.shadow = Some(ui::Shadow::Floating);
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint {
            color: ui::Color::Line,
            alpha: ui::Alpha::Hairline,
        },
        outside: true,
    });
    (
        stack("composer/kind-menu", style, Axis::Column, 0., children),
        width,
    )
}
