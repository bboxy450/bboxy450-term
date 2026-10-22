//! Pane identity strip. The host supplies capabilities and one-shot notice state.
use super::seat_row::{SeatRow, named, role_color};
use crate::{self as ui, components as c};
use bbt_ui_model::{Activity, STYLE, TOKENS};

pub(crate) fn plain(height: Option<f32>, pad: [f32; 4], radius: f32) -> ui::Style {
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
pub(crate) fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
pub(crate) fn text(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    mono: bool,
    color: ui::Color,
    alpha: f32,
    line_height: f32,
) -> c::Element {
    let mut t = c::Text::new(
        value,
        ui::Text {
            size: Some(size),
            weight,
            mono,
            paint: paint(color, alpha),
            line_height,
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    c::Element::Text(t)
}
fn flexible(element: c::Element, grow: f32) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Content {
            grow,
            shrink: 1.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn icon(key: &str, name: &str, size: f32, color: ui::Color, alpha: f32) -> c::Element {
    named(
        key,
        c::Element::Icon {
            name: name.into(),
            paint: paint(color, alpha),
            size: ui::Length::Px(size),
        },
    )
}
fn pill(
    key: &str,
    value: &str,
    size: f32,
    alpha: f32,
    pad: f32,
    height: Option<f32>,
    mono: bool,
) -> c::Element {
    let mut s = plain(
        height,
        [
            if mono { 1. } else { 0. },
            pad,
            if mono { 1. } else { 0. },
            pad,
        ],
        if mono {
            STYLE.radius.tag as f32
        } else {
            STYLE.radius.pill as f32
        },
    );
    s.overrides.fill = Some(paint(
        ui::Color::Rgba([0., 0., 0., 1.]),
        if key == "isent-chip" { 0.28 } else { 0.22 },
    ));
    let mut b = c::Box::new(
        s,
        vec![
            text(
                value,
                size,
                400,
                mono,
                ui::Color::OnAccent,
                alpha,
                if height.is_some() { 14. / size } else { 1.4 },
            )
            .into(),
        ],
    );
    b.gap = ui::Length::Px(0.);
    named(format!("pane/strip/{key}"), c::Element::Box(b))
}

#[derive(Debug, Clone)]
pub struct Strip {
    pub seat: SeatRow,
    /// Public waiting caption; absent means the lab's "an answer" default.
    pub waiting_description: Option<String>,
    pub may_interrupt: bool,
    pub may_mute: bool,
}
impl Strip {
    pub fn new(seat: SeatRow) -> Self {
        Self {
            seat,
            waiting_description: None,
            may_interrupt: false,
            may_mute: false,
        }
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let seat = &self.seat;
        let on = ui::Color::OnAccent;
        let mut style = ui::Style::new(ui::Preset::Strip);
        style.overrides.fill = Some(ui::Paint::solid(seat.color(ctx)));
        let mut close = plain(Some(12.), [0.; 4], 6.);
        close.overrides.width = Some(ui::Length::Px(12.));
        close.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba([
            1.,
            95. / 255.,
            87. / 255.,
            1.,
        ])));
        close.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: paint(ui::Color::Rgba([0., 0., 0., 1.]), 0.25),
            outside: false,
        });
        let mut children = vec![
            named(
                "pane/strip/close",
                c::Element::Box(c::Box::new(close, vec![])),
            )
            .into(),
        ];
        if let Some(where_) = &seat.where_ {
            children.push(icon("pane/strip/where", where_, 13., on, 0.85).into());
        }
        children.push(icon("pane/strip/hicon", &seat.harness, 13., on, 1.).into());
        children.push(flexible(
            named(
                "pane/strip/name",
                text(
                    &seat.name,
                    STYLE.r#type.caption as f32,
                    600,
                    false,
                    on,
                    1.,
                    1.4,
                ),
            ),
            0.,
        ));
        if let Some(role) = &seat.role {
            let mark = match role.as_str() {
                "oracle" => "♔",
                "creativity" => "♝",
                "architect" => "♖",
                "executor" => "♟",
                _ => "·",
            };
            children.push(
                named(
                    "pane/strip/rmark",
                    text(
                        mark,
                        ctx.density.tokens().glyph as f32 - 1.,
                        400,
                        true,
                        role_color(role),
                        0.9,
                        1.4,
                    ),
                )
                .into(),
            );
        }
        if let Some(authority) = &seat.authority {
            children.push(
                named(
                    "pane/strip/authmark",
                    icon("pane/strip/authmark/hicon", "keyx", 13., on, 1.),
                )
                .into(),
            );
            let caption = match authority.as_str() {
                "expired" => "authority expired · renew the seat grant",
                "revoked" => "revoked here · peers learn on next contact",
                _ => "forked · two heads at one epoch · the operator resolves",
            };
            children.push(flexible(
                pill(
                    "authsay",
                    caption,
                    STYLE.r#type.small as f32,
                    0.85,
                    STYLE.spacing.s3 as f32,
                    None,
                    false,
                ),
                0.,
            ));
        }
        let title = if matches!(seat.activity, Activity::Waiting(_)) {
            format!(
                "{} · waiting for {}",
                seat.title,
                self.waiting_description.as_deref().unwrap_or("an answer")
            )
        } else {
            seat.title.clone()
        };
        children.push(flexible(
            named(
                "pane/strip/title",
                text(title, STYLE.r#type.caption as f32, 400, false, on, 0.8, 1.4),
            ),
            1.,
        ));
        if let Some(key) = &seat.shortcut {
            children.push(
                pill(
                    "kbd",
                    key,
                    STYLE.r#type.small as f32,
                    0.85,
                    5.,
                    Some(16.),
                    true,
                )
                .into(),
            );
        }
        if seat.interrupt_sent {
            children.push(
                pill(
                    "isent-chip",
                    "interrupt sent",
                    STYLE.r#type.tiny as f32,
                    1.,
                    5.,
                    Some(14.),
                    false,
                )
                .into(),
            );
        }
        if self.may_interrupt {
            let mut square = plain(Some(8.), [0.; 4], 1.5);
            square.overrides.width = Some(ui::Length::Px(8.));
            square.overrides.fill = Some(paint(on, 0.55));
            let mut holder = plain(Some(14.), [0.; 4], 0.);
            holder.overrides.width = Some(ui::Length::Px(14.));
            let mut outer = c::Box::new(
                holder,
                vec![
                    named(
                        "pane/strip/stop/i",
                        c::Element::Box(c::Box::new(square, vec![])),
                    )
                    .into(),
                ],
            );
            outer.gap = ui::Length::Px(0.);
            // Centre the 8px square inside the 14px hit target.
            outer.style.overrides.padding = Some([ui::Length::Px(3.); 4]);
            children.push(named("pane/strip/stop", c::Element::Box(outer)).into());
        }
        if self.may_mute {
            let alpha = if seat.authority.is_some() {
                0.3
            } else if seat.muted {
                1.
            } else {
                0.55
            };
            let color = if seat.muted {
                ui::Color::Rgba([1., 224. / 255., 138. / 255., 1.])
            } else {
                on
            };
            children.push(
                named(
                    "pane/strip/mute",
                    icon(
                        "pane/strip/mute/hicon",
                        if seat.muted { "muted" } else { "unmuted" },
                        13.,
                        color,
                        alpha,
                    ),
                )
                .into(),
            );
        }
        children.push(
            named(
                "pane/strip/glyph",
                c::Element::Glyph {
                    activity: seat.activity,
                    grade: seat.grade,
                    color: ui::Paint::solid(seat.color(ctx)),
                    size: 11.,
                    scale: 1.,
                    kind_colors: ui::atoms::KindColors {
                        foreground: Some(bbt_ui_model::draw::Color::hex(TOKENS.color.ui.on_accent)),
                        background: Some(bbt_ui_model::draw::Color::TRANSPARENT),
                    },
                },
            )
            .into(),
        );
        let mut b = c::Box::new(style, children);
        b.clip = true;
        b.cross_overflow = true;
        named("pane/strip", c::Element::Box(b))
    }
}
