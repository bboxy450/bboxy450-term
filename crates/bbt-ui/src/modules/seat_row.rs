//! Seat identity and its subordinate rows. Inputs are view data, never a clock,
//! harness adapter, delivery scheduler or task controller.
use crate::{self as ui, components as c};
use bbt_ui_model::{Activity, Grade, STYLE, SeatColor, TOKENS};

pub fn named(name: impl Into<String>, content: c::Element) -> c::Element {
    c::Element::Named {
        name: name.into(),
        content: Box::new(content),
    }
}
fn rgba(value: &str) -> ui::Color {
    ui::Color::Rgba(bbt_ui_model::draw::Color::hex(value).0)
}
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn label(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    mono: bool,
    color: ui::Color,
) -> c::Element {
    let mut t = c::Text::new(
        value,
        ui::Text {
            size: Some(size),
            weight,
            mono,
            paint: ui::Paint::solid(color),
            line_height: 1.4,
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    c::Element::Text(t)
}
fn plain(height: Option<f32>, pad: [f32; 4], radius: f32) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.height = height.map(ui::Length::Px);
    s.overrides.padding = Some(pad.map(ui::Length::Px));
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s
}
fn row_box(style: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.gap = ui::Length::Px(gap);
    c::Element::Box(b)
}
fn shrink(element: c::Element, grow: f32, min: f32, max: Option<f32>) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Content {
            grow,
            shrink: 1.,
            min,
            max,
            max_fraction: None,
        },
    }
}
fn icon(name: &str, size: f32, color: ui::Color) -> c::Element {
    c::Element::Icon {
        name: name.into(),
        paint: ui::Paint::solid(color),
        size: ui::Length::Px(size),
    }
}
fn chip(value: &str, color: ui::Color, alpha: f32, border: bool, mono: bool) -> c::Element {
    let mut s = plain(Some(14.), [0., 5., 0., 5.], STYLE.radius.tag as f32);
    s.overrides.fill = Some(paint(color, alpha));
    if border {
        s.overrides.dashed = Some(true);
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: paint(ui::Color::Line, 1.),
            outside: false,
        });
    }
    let mut text = label(value, STYLE.r#type.tiny as f32, 400, mono, color);
    if let c::Element::Text(t) = &mut text {
        t.runs[0].style.line_height = if border { 12. } else { 14. } / STYLE.r#type.tiny as f32;
    }
    row_box(s, 0., vec![text.into()])
}
fn noise_chip(value: &str, color: ui::Color, alpha: f32, border: bool, mono: bool) -> c::Element {
    let mut el = chip(value, color, alpha, border, mono);
    if let c::Element::Box(b) = &mut el {
        b.style.overrides.height = Some(ui::Length::Px(16.));
        b.style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: paint(color, 0.6),
            outside: false,
        });
        if let c::Element::Text(t) = &mut b.children[0].element {
            t.runs[0].style.line_height = 14. / STYLE.r#type.tiny as f32;
        }
    }
    el
}
pub fn role_color(role: &str) -> ui::Color {
    rgba(match role {
        "oracle" => TOKENS.color.role.oracle,
        "creativity" => TOKENS.color.role.creativity,
        "architect" => TOKENS.color.role.architect,
        "executor" => TOKENS.color.role.executor,
        _ => TOKENS.color.role.unsigned,
    })
}
fn role_mark(role: &str) -> &'static str {
    match role {
        "oracle" => "♔",
        "creativity" => "♝",
        "architect" => "♖",
        "executor" => "♟",
        _ => "·",
    }
}

#[derive(Debug, Clone)]
pub struct Held {
    pub id: String,
    pub blocked: Option<String>,
}
#[derive(Debug, Clone)]
pub struct SeatRow {
    pub key: String,
    pub name: String,
    pub harness: String,
    pub role: Option<String>,
    pub title: String,
    pub activity: Activity,
    pub grade: Grade,
    pub color: Option<[f32; 4]>,
    pub where_: Option<String>,
    pub focused: bool,
    pub muted: bool,
    pub authority: Option<String>,
    pub held: Vec<Held>,
    pub listeners: u32,
    pub touched: u32,
    pub unread: Option<(String, String)>,
    pub shortcut: Option<String>,
    pub subtitle: Option<String>,
    pub restarted: bool,
    pub interrupt_sent: bool,
    pub noise: Option<f32>,
    /// Optional per-container density override, as in the subtitle fixture.
    pub row_height: Option<f32>,
}
impl SeatRow {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        harness: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            harness: harness.into(),
            role: None,
            title: String::new(),
            activity: Activity::Idle,
            grade: Grade::Reported,
            color: None,
            where_: None,
            focused: false,
            muted: false,
            authority: None,
            held: vec![],
            listeners: 0,
            touched: 0,
            unread: None,
            shortcut: None,
            subtitle: None,
            restarted: false,
            interrupt_sent: false,
            noise: None,
            row_height: None,
        }
    }
    pub fn color(&self, ctx: ui::Context) -> ui::Color {
        ui::Color::Rgba(self.color.unwrap_or_else(|| {
            let h = SeatColor::Derived.resolve(&self.key, ctx.theme);
            bbt_ui_model::draw::Color::hsl(h.hue, h.saturation / 100., h.lightness / 100.).0
        }))
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let d = ctx.density.tokens();
        let color = self.color(ctx);
        let mut style = plain(
            Some(self.row_height.unwrap_or(d.row_h as f32)),
            [0., d.pad_x as f32, 0., d.pad_x as f32],
            TOKENS.radius.chip as f32,
        );
        let state_color = match self.activity {
            Activity::Waiting(_) => rgba(TOKENS.color.state.waiting),
            Activity::Errored => rgba(TOKENS.color.state.errored),
            _ => color,
        };
        let fill = if self.focused {
            paint(ui::Color::Accent, 0.16)
        } else {
            match self.activity {
                Activity::Waiting(_) => paint(state_color, 0.10),
                Activity::Errored => paint(state_color, 0.08),
                _ => ui::Paint::solid(ui::Color::Transparent),
            }
        };
        style.overrides.fill = Some(fill);
        let a = match self.activity {
            Activity::Active => TOKENS.glow.active_alpha_min,
            Activity::Waiting(_) => TOKENS.glow.waiting_alpha_min,
            Activity::Errored => TOKENS.glow.errored_alpha,
            _ => TOKENS.glow.idle_alpha,
        } as f32;
        if !self.focused || matches!(self.activity, Activity::Active | Activity::Waiting(_)) {
            let mut shadow = ui::Paint {
                color: state_color,
                alpha: ui::Alpha::Value(
                    (a * if self.focused {
                        TOKENS.glow.focused_boost as f32
                    } else {
                        1.
                    })
                    .min(1.),
                ),
            }
            .resolve(ctx)
            .expect("token colour");
            if self.grade == Grade::Inferred {
                shadow[3] *= 0.5;
            }
            style.overrides.shadow = Some(ui::Shadow::Custom {
                blur: TOKENS.glow.row_radius as f32,
                color: shadow,
            });
        }
        let mut children = vec![];
        if let Some(where_) = &self.where_ {
            children.push(
                named(
                    "where",
                    named(
                        "where/hicon",
                        icon(
                            where_,
                            12.,
                            if where_ == "cloud" {
                                rgba(ctx.theme.palette().accent_soft)
                            } else {
                                ui::Color::Muted
                            },
                        ),
                    ),
                )
                .into(),
            );
        }
        let tile_color = match self.harness.as_str() {
            "claude" => rgba("#D97757"),
            "codex" => rgba("#1F2937"),
            "pi" => rgba("#7C3AED"),
            "omp" => rgba("#0EA5E9"),
            "zsh" => rgba("#111827"),
            _ => ui::Color::Surface2,
        };
        let mut tile = plain(Some(d.icon as f32), [0.; 4], d.icon as f32 * 0.22);
        tile.overrides.fill = Some(ui::Paint::solid(tile_color));
        children.push(
            named(
                "tile",
                row_box(
                    tile,
                    0.,
                    vec![
                        named(
                            "tile/hicon",
                            icon(&self.harness, d.icon as f32, ui::Color::Foreground),
                        )
                        .into(),
                    ],
                ),
            )
            .into(),
        );
        let name_color = if self.muted {
            ui::Color::Rgba({
                let mut c = ui::Paint::solid(color).resolve(ctx).unwrap();
                c[3] *= 0.7;
                c
            })
        } else {
            color
        };
        children.push(shrink(
            named(
                "name",
                label(&self.name, d.font as f32, 600, false, name_color),
            ),
            0.,
            0.,
            None,
        ));
        if let Some(role) = &self.role {
            let mut mark = label(
                role_mark(role),
                d.glyph as f32 - 1.,
                400,
                true,
                role_color(role),
            );
            if let c::Element::Text(t) = &mut mark {
                t.runs[0].style.paint.alpha = ui::Alpha::Value(0.9);
            }
            children.push(named("rmark", mark).into());
        }
        if let Some(cause) = &self.authority {
            let c = rgba(if cause == "expired" {
                TOKENS.color.state.waiting
            } else {
                TOKENS.color.state.errored
            });
            children.push(named("authmark", named("authmark/hicon", icon("keyx", 11., c))).into());
        }
        if self.muted {
            children.push(
                named(
                    "mutemark",
                    named(
                        "mutemark/hicon",
                        icon("muted", 11., rgba(TOKENS.color.state.waiting)),
                    ),
                )
                .into(),
            );
        }
        if ctx.density != ui::Density::Compact {
            children.push(c::Item {
                element: named(
                    "title",
                    label(
                        &self.title,
                        d.font_small as f32,
                        400,
                        false,
                        ui::Color::Muted,
                    ),
                ),
                extent: c::Extent::Fill,
            });
        }
        let mut right: Vec<c::Item> = vec![];
        if self.interrupt_sent {
            right.push(
                named(
                    "isent-chip",
                    chip(
                        "interrupt sent",
                        rgba(TOKENS.color.state.errored),
                        0.14,
                        false,
                        false,
                    ),
                )
                .into(),
            );
        }
        if self.restarted {
            right.push(
                named(
                    "restarted",
                    chip("restarted", ui::Color::Accent, 0.16, false, false),
                )
                .into(),
            );
        }
        if !self.held.is_empty() {
            right.push(
                named(
                    "held",
                    chip(
                        &format!("held {}", self.held.len()),
                        if self.muted {
                            rgba(TOKENS.color.state.waiting)
                        } else {
                            ui::Color::Muted
                        },
                        if self.muted { 0.14 } else { 0. },
                        !self.muted,
                        true,
                    ),
                )
                .into(),
            );
        }
        if let Some(noise) = self.noise {
            right.push(
                named(
                    "noise",
                    noise_chip(
                        &format!("noisy {:.0}%", noise * 100.),
                        rgba(TOKENS.color.state.waiting),
                        0.,
                        true,
                        false,
                    ),
                )
                .into(),
            );
        }
        for (count, name, ico, gap, padded) in [
            (self.listeners, "listeners", "plug", 1., false),
            (
                self.touched,
                "touched",
                "archive",
                STYLE.spacing.s1 as f32,
                true,
            ),
        ] {
            if count == 0 {
                continue;
            }
            let mut s = plain(
                Some(14.),
                if padded { [0., 5., 0., 5.] } else { [0.; 4] },
                STYLE.radius.tag as f32,
            );
            if padded {
                s.overrides.fill = Some(paint(ui::Color::Muted, STYLE.tint.badge_alpha as f32));
            }
            let mut n = label(
                count.to_string(),
                STYLE.r#type.tiny as f32,
                400,
                true,
                ui::Color::Muted,
            );
            if let c::Element::Text(t) = &mut n {
                t.runs[0].style.line_height = 14. / STYLE.r#type.tiny as f32;
            }
            right.push(
                named(
                    name,
                    row_box(
                        s,
                        gap,
                        vec![
                            named(format!("{name}/hicon"), icon(ico, 10., ui::Color::Muted)).into(),
                            named(format!("{name}/text"), n).into(),
                        ],
                    ),
                )
                .into(),
            );
        }
        if let Some((unread, role)) = &self.unread {
            let mut s = plain(Some(16.), [0., 5., 0., 5.], STYLE.radius.pill as f32);
            s.overrides.fill = Some(ui::Paint::solid(role_color(role)));
            let mut text = label(
                unread,
                STYLE.r#type.small as f32,
                600,
                true,
                ui::Color::OnAccent,
            );
            if let c::Element::Text(t) = &mut text {
                t.runs[0].style.line_height = 16. / STYLE.r#type.small as f32;
            }
            right.push(named("badge", row_box(s, 0., vec![text.into()])).into());
        }
        if let Some(shortcut) = &self.shortcut {
            right.push(
                named(
                    "kbd",
                    label(shortcut, d.font_small as f32, 400, true, ui::Color::Muted),
                )
                .into(),
            );
        }
        right.push(
            named(
                "glyph",
                c::Element::Glyph {
                    activity: self.activity,
                    grade: self.grade,
                    color: ui::Paint::solid(color),
                    size: d.glyph as f32,
                    scale: 1.,
                },
            )
            .into(),
        );
        // Prefix nested semantic labels without changing the composition's geometry.
        for item in &mut right {
            prefix(&mut item.element, "right/");
        }
        let right = named(
            "right",
            row_box(plain(None, [0.; 4], 0.), d.gap as f32, right),
        );
        if ctx.density == ui::Density::Compact {
            // margin-left:auto pushes the natural-sized group without adding
            // another flex gap or changing the group's diagnostic rectangle.
            let pushed = c::Element::Stack(c::Stack {
                direction: bbt_ui_model::layout::Axis::Row,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Center,
                children: vec![c::Element::Spacer.into(), right.into()],
            });
            children.push(c::Item {
                element: pushed,
                extent: c::Extent::Content {
                    grow: 1.,
                    shrink: 0.,
                    min: 0.,
                    max: None,
                    max_fraction: None,
                },
            });
        } else {
            children.push(right.into());
        }
        let base = row_box(style, d.gap as f32, children);
        let base = if let Some(subtitle) = &self.subtitle {
            c::Element::Layers {
                base: Box::new(base),
                overlays: vec![c::Placed {
                    content: named(
                        "sub",
                        label(
                            subtitle,
                            STYLE.r#type.small as f32,
                            400,
                            false,
                            ui::Color::Muted,
                        ),
                    ),
                    left: (d.pad_x + d.icon + d.gap) as f32,
                    bottom: 2.,
                }],
            }
        } else {
            base
        };
        named("row", base)
    }
}
fn prefix(element: &mut c::Element, prefix: &str) {
    match element {
        c::Element::Named { name, content } => {
            *name = format!("{prefix}{name}");
            prefix_names(content, prefix);
        }
        _ => prefix_names(element, prefix),
    }
}
fn prefix_names(element: &mut c::Element, value: &str) {
    match element {
        c::Element::Box(b) => {
            for child in &mut b.children {
                prefix(&mut child.element, value)
            }
        }
        c::Element::Stack(s) => {
            for child in &mut s.children {
                prefix(&mut child.element, value)
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
pub struct Subagent {
    pub id: String,
    pub description: String,
    pub detail: Option<String>,
    pub activity: Activity,
}
impl SeatRow {
    pub fn subagent_rows(&self, agents: &[Subagent], ctx: ui::Context) -> Vec<c::Element> {
        let running: Vec<_> = agents
            .iter()
            .filter(|a| matches!(a.activity, Activity::Active | Activity::Waiting(_)))
            .collect();
        let count = if running.len() > 8 { 7 } else { running.len() };
        let mut rows = running
            .iter()
            .take(count)
            .map(|a| self.subagent(Some(a), None, ctx))
            .collect::<Vec<_>>();
        if running.len() > 8 {
            rows.push(self.subagent(None, Some(running.len() - 7), ctx));
        }
        rows
    }
    fn subagent(
        &self,
        agent: Option<&&Subagent>,
        more: Option<usize>,
        ctx: ui::Context,
    ) -> c::Element {
        let d = ctx.density.tokens();
        let h = d.row_h as f32 - 6.;
        let s = plain(
            Some(h),
            [
                0.,
                d.pad_x as f32,
                0.,
                d.pad_x as f32 + STYLE.spacing.s4 as f32,
            ],
            STYLE.radius.row as f32,
        );
        let mut branch = plain(Some(h * 0.5), [0.; 4], 0.);
        branch.overrides.width = Some(ui::Length::Px(10.));
        branch.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: paint(ui::Color::Line, STYLE.material.hairline_alpha as f32),
            outside: false,
        });
        branch.overrides.border_sides = Some([false, false, true, true]);
        branch.overrides.radii = Some([
            ui::Length::Px(0.),
            ui::Length::Px(0.),
            ui::Length::Px(0.),
            ui::Length::Px(TOKENS.radius.chip as f32),
        ]);
        // Branch owns a 4px leading margin and aligns to the top, independently of the centred text.
        let mut branch_wrap = c::Box::new(
            plain(Some(h), [0., 0., h * 0.5, STYLE.spacing.s2 as f32], 0.),
            vec![named("branch", row_box(branch, 0., vec![])).into()],
        );
        branch_wrap.align = c::Alignment::Start;
        let mut children = vec![c::Element::Box(branch_wrap).into()];
        if let Some(agent) = agent {
            children.push(
                named(
                    "glyph",
                    c::Element::Glyph {
                        activity: agent.activity,
                        grade: if self.harness == "codex" {
                            Grade::Shim
                        } else {
                            Grade::Reported
                        },
                        color: ui::Paint::solid(self.color(ctx)),
                        size: d.glyph as f32,
                        scale: 0.8,
                    },
                )
                .into(),
            );
            children.push(shrink(
                named(
                    "sdesc",
                    label(
                        &agent.description,
                        d.font_small as f32,
                        400,
                        false,
                        ui::Color::Foreground,
                    ),
                ),
                0.,
                0.,
                None,
            ));
            if let Some(detail) = &agent.detail {
                children.push(c::Item {
                    element: named(
                        "sdetail",
                        label(
                            detail,
                            STYLE.r#type.small as f32,
                            400,
                            true,
                            ui::Color::Muted,
                        ),
                    ),
                    extent: c::Extent::Fill,
                });
            }
        } else {
            children.push(shrink(
                named(
                    "sdesc",
                    label(
                        format!("+{} running", more.unwrap()),
                        STYLE.r#type.small as f32,
                        400,
                        true,
                        ui::Color::Muted,
                    ),
                ),
                0.,
                0.,
                None,
            ));
        }
        named("row", row_box(s, STYLE.spacing.s3 as f32, children))
    }
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub title: String,
    pub state: String,
    pub bug: bool,
    pub violates: Option<String>,
    pub constrains: Option<String>,
    pub waits: Option<String>,
    pub may_accept: bool,
}
impl TaskRow {
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let d = ctx.density.tokens();
        let h = d.row_h as f32 - 4.;
        let pad = d.pad_x as f32;
        let mut s = plain(Some(h), [0., pad, 0., pad + 22.], STYLE.radius.row as f32);
        if matches!(self.state.as_str(), "duplicate" | "wontfix") {
            s.overrides.opacity = Some(0.6);
        }
        let status = match self.state.as_str() {
            "running" => ui::Color::Accent,
            "reported" => rgba(TOKENS.color.state.waiting),
            "accepted" | "done" => rgba(TOKENS.color.ui.ok),
            "rejected" | "blocked" => rgba(TOKENS.color.state.errored),
            _ => ui::Color::Muted,
        };
        let glyph_color = if self.bug {
            rgba(if self.state == "fixed" {
                TOKENS.color.state.idle
            } else {
                TOKENS.color.state.errored
            })
        } else {
            status
        };
        let glyph = if self.bug {
            named("tglyph/hicon", icon("bug", 11., glyph_color))
        } else {
            label(
                match self.state.as_str() {
                    "claimed" => "◔",
                    "running" => "◑",
                    "reported" => "◕",
                    "accepted" => "●",
                    "done" => "✓",
                    "rejected" => "✕",
                    "blocked" => "⊘",
                    _ => "○",
                },
                d.font_small as f32,
                400,
                true,
                glyph_color,
            )
        };
        let glyph = if self.bug {
            c::Element::Baseline {
                content: Box::new(glyph),
                offset: -1.,
            }
        } else {
            glyph
        };
        let mut gs = plain(
            Some((d.font_small as f32 * 1.4 * 64.).floor() / 64.),
            [0.; 4],
            0.,
        );
        gs.overrides.width = Some(ui::Length::Px(12.));
        gs.overrides.text = Some(ui::Text {
            size: Some(d.font_small as f32),
            mono: true,
            line_height: 1.4,
            ..Default::default()
        });
        let mut glyph_box = c::Box::new(
            gs,
            vec![
                c::Element::Spacer.into(),
                glyph.into(),
                c::Element::Spacer.into(),
            ],
        );
        glyph_box.gap = ui::Length::Px(0.);
        glyph_box.align = c::Alignment::Start;
        glyph_box.inline = true;
        let mut children = vec![
            named("tglyph", c::Element::Box(glyph_box)).into(),
            shrink(
                named(
                    "ttitle",
                    label(
                        &self.title,
                        d.font_small as f32,
                        400,
                        false,
                        ui::Color::Foreground,
                    ),
                ),
                1.,
                d.font_small as f32 * 5.,
                None,
            ),
        ];
        for (title, violates) in [(&self.violates, true), (&self.constrains, false)] {
            if let Some(title) = title {
                let color = if violates {
                    rgba(TOKENS.color.state.errored)
                } else {
                    rgba(TOKENS.color.ui.invariant)
                };
                let mut st = plain(
                    Some(16.),
                    [0., STYLE.spacing.s3 as f32, 0., STYLE.spacing.s3 as f32],
                    STYLE.radius.pill as f32,
                );
                st.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: paint(color, if violates { 0.6 } else { 0.55 }),
                    outside: false,
                });
                if violates {
                    st.overrides.fill = Some(paint(color, 0.1));
                }
                let mut lead = label(
                    if violates { "breaks" } else { "inv" },
                    STYLE.r#type.tiny as f32,
                    400,
                    false,
                    if violates { color } else { ui::Color::Muted },
                );
                let mut title_text = label(
                    title,
                    STYLE.r#type.tiny as f32,
                    500,
                    false,
                    if violates {
                        color
                    } else {
                        ui::Color::Foreground
                    },
                );
                for el in [&mut lead, &mut title_text] {
                    if let c::Element::Text(t) = el {
                        t.runs[0].style.line_height = 14. / STYLE.r#type.tiny as f32;
                    }
                }
                let item = row_box(
                    st,
                    STYLE.spacing.s1 as f32,
                    vec![
                        named("invchip/text", lead).into(),
                        shrink(named("invchip/b", title_text), 0., 0., None),
                    ],
                );
                // Percentage bound is resolved from the module's input width by element_at_width.
                let mut child = shrink(named("invchip", item), 0., 0., None);
                if let c::Extent::Content { max_fraction, .. } = &mut child.extent {
                    *max_fraction = Some(0.42);
                }
                children.push(child);
            }
        }
        if self.state == "blocked"
            && let Some(waits) = &self.waits
        {
            children.push(
                named(
                    "twaits",
                    label(
                        format!("waits on {waits}"),
                        STYLE.r#type.tiny as f32,
                        400,
                        false,
                        rgba(TOKENS.color.state.errored),
                    ),
                )
                .into(),
            );
        }
        let mut right: Vec<c::Item> = vec![];
        if self.state == "reported" && self.may_accept {
            let mut st = plain(None, [1., 7., 1., 7.], STYLE.radius.pill as f32);
            st.overrides.fill = Some(ui::Paint::solid(rgba(TOKENS.color.state.waiting)));
            right.push(
                named(
                    "accept",
                    row_box(
                        st,
                        0.,
                        vec![
                            label(
                                "accept",
                                STYLE.r#type.tiny as f32,
                                600,
                                false,
                                rgba("#000000"),
                            )
                            .into(),
                        ],
                    ),
                )
                .into(),
            );
        }
        let mut text = label(
            &self.state,
            STYLE.r#type.tiny as f32,
            400,
            true,
            if self.state == "running" {
                ui::Color::Muted
            } else {
                status
            },
        );
        if let c::Element::Text(t) = &mut text {
            t.runs[0].style.tracking_em = 0.04;
        }
        right.push(named("tstate", text).into());
        for item in &mut right {
            prefix(&mut item.element, "right/");
        }
        children.push(
            named(
                "right",
                row_box(plain(None, [0.; 4], 0.), STYLE.spacing.s3 as f32, right),
            )
            .into(),
        );
        named("row", row_box(s, STYLE.spacing.s3 as f32, children))
    }
}

#[derive(Debug, Clone)]
pub struct ArtifactRow {
    pub kind: String,
    pub reference: String,
    pub digest: Option<String>,
    pub responsible: Option<String>,
    pub responsible_color: Option<[f32; 4]>,
}
impl ArtifactRow {
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let d = ctx.density.tokens();
        let s = plain(
            Some(d.row_h as f32 - 6.),
            [0., d.pad_x as f32, 0., d.pad_x as f32 + 36.],
            STYLE.radius.row as f32,
        );
        let glyph = match self.kind.as_str() {
            "markdown" => "▤",
            "pr" => "⎇",
            "binary" => "▣",
            "image" => "▧",
            "release" => "◆",
            _ => "▢",
        };
        let mut gs = plain(None, [0.; 4], 0.);
        gs.overrides.width = Some(ui::Length::Px(12.));
        let mut kind = label(
            &self.kind,
            STYLE.r#type.micro as f32,
            400,
            true,
            ui::Color::Muted,
        );
        if let c::Element::Text(t) = &mut kind {
            t.runs[0].style.uppercase = true;
            t.runs[0].style.tracking_em = 0.06;
        }
        let mut ks = plain(
            None,
            [0., STYLE.spacing.s2 as f32, 0., STYLE.spacing.s2 as f32],
            TOKENS.radius.chip as f32,
        );
        ks.overrides.fill = Some(paint(ui::Color::Muted, STYLE.tint.badge_alpha as f32));
        let mut right: Vec<c::Item> = vec![];
        if let Some(digest) = &self.digest {
            right.push(
                named(
                    "adigest",
                    label(
                        digest.chars().take(7).collect::<String>(),
                        STYLE.r#type.micro as f32,
                        400,
                        true,
                        ui::Color::Muted,
                    ),
                )
                .into(),
            );
        }
        if let Some(responsible) = &self.responsible {
            let color = self
                .responsible_color
                .map(ui::Color::Rgba)
                .unwrap_or(ui::Color::Foreground);
            let mut cs = plain(
                Some(16.),
                [0., STYLE.spacing.s3 as f32, 0., STYLE.spacing.s3 as f32],
                STYLE.radius.pill as f32,
            );
            cs.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint::solid(if self.responsible_color.is_some() {
                    color
                } else {
                    ui::Color::Line
                }),
                outside: false,
            });
            let mut title = label(
                responsible.split_whitespace().next().unwrap_or(""),
                STYLE.r#type.tiny as f32,
                600,
                false,
                color,
            );
            if let c::Element::Text(t) = &mut title {
                t.runs[0].style.line_height = 14. / STYLE.r#type.tiny as f32;
            }
            right.push(named("achip", row_box(cs, 0., vec![title.into()])).into());
        }
        for item in &mut right {
            prefix(&mut item.element, "right/");
        }
        let mut result = named(
            "row",
            row_box(
                s,
                STYLE.spacing.s3 as f32,
                vec![
                    named(
                        "aglyph",
                        row_box(
                            gs,
                            0.,
                            vec![
                                c::Element::Spacer.into(),
                                label(glyph, d.font_small as f32, 400, true, ui::Color::Muted)
                                    .into(),
                                c::Element::Spacer.into(),
                            ],
                        ),
                    )
                    .into(),
                    named("akind", row_box(ks, 0., vec![kind.into()])).into(),
                    c::Item {
                        element: named(
                            "aref",
                            label(
                                &self.reference,
                                STYLE.r#type.note as f32,
                                400,
                                true,
                                ui::Color::Foreground,
                            ),
                        ),
                        extent: c::Extent::Fill,
                    },
                    named(
                        "right",
                        row_box(plain(None, [0.; 4], 0.), STYLE.spacing.s3 as f32, right),
                    )
                    .into(),
                ],
            ),
        );
        if let c::Element::Named { content, .. } = &mut result
            && let c::Element::Box(b) = content.as_mut()
        {
            b.cross_overflow = true;
        }
        result
    }
}
