//! Swarm chrome and its accepted tab compositions. Visibility and lifecycle
//! data belong to the caller; this module neither changes them nor runs a clock.
use crate::{
    self as ui, components as c,
    modules::{
        seat_row::{SeatRow, named},
        tab_row::TabRow,
    },
};
use bbt_ui_model::{Activity, STYLE, TOKENS, draw::Color as Rgba, layout::Axis};

pub(crate) fn plain(
    height: Option<f32>,
    padding: [f32; 4],
    fill: ui::Color,
    radius: f32,
) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.height = height.map(ui::Length::Px);
    s.overrides.padding = Some(padding.map(ui::Length::Px));
    s.overrides.material = Some(ui::Material::Flat);
    s.overrides.fill = Some(ui::Paint::solid(fill));
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s
}
pub(crate) fn flow(
    style: ui::Style,
    direction: Axis,
    gap: f32,
    align: c::Alignment,
    children: Vec<c::Item>,
) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = direction;
    b.gap = ui::Length::Px(gap);
    b.align = align;
    c::Element::Box(b)
}
pub(crate) fn text(
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
            line_height: 1.4,
            paint: ui::Paint::solid(color),
            ..Default::default()
        },
    );
    t.wrap = false;
    t.ellipsis = true;
    c::Element::Text(t)
}
pub(crate) fn inset(e: c::Element, x: f32) -> c::Element {
    flow(
        plain(None, [0., x, 0., x], ui::Color::Transparent, 0.),
        Axis::Column,
        0.,
        c::Alignment::Stretch,
        vec![e.into()],
    )
}
pub(crate) fn card_style(fill: ui::Color) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.padding = Some([ui::Length::Px(0.); 4]);
    s.overrides.fill = Some(ui::Paint {
        color: fill,
        alpha: ui::Alpha::Value(STYLE.material.card_alpha as f32),
    });
    s
}
fn dim(element: c::Element, unfocused: bool) -> c::Element {
    if unfocused {
        c::Element::Filter {
            content: Box::new(element),
            saturation: TOKENS.dim.saturate as f32,
            opacity: TOKENS.dim.opacity as f32,
        }
    } else {
        element
    }
}

#[derive(Debug, Clone)]
pub struct SwarmCard {
    pub key: String,
    pub name: String,
    pub color: [f32; 4],
    pub seats: Vec<SeatRow>,
    pub expanded: bool,
    /// The sidebar adds its closed bumper reading; the standalone header page
    /// deliberately keeps the normal bumper when it hides its body.
    pub closed: bool,
    pub unfocused: bool,
    pub tabs: Vec<TabRow>,
    pub body: bool,
    pub new_seat: bool,
    pub extra: Vec<c::Element>,
    pub shadows: Vec<SeatRow>,
}
impl SwarmCard {
    pub fn header_element(&self, ctx: ui::Context) -> c::Element {
        let white = |a: f32| ui::Color::Rgba([1., 1., 1., a]);
        let mut title = match text(
            &self.name,
            STYLE.r#type.note as f32,
            600,
            false,
            white(0.92),
        ) {
            c::Element::Text(t) => t,
            _ => unreachable!(),
        };
        title.wrap = true;
        title.ellipsis = false;
        title.runs[0].style.uppercase = true;
        title.runs[0].style.tracking_em = 0.16;
        let mut children = vec![
            c::Item {
                element: named("head/sname", c::Element::Text(title)),
                extent: c::Extent::MinContent,
            },
            named(
                "head/count",
                text(
                    self.seats.len().to_string(),
                    STYLE.r#type.note as f32,
                    400,
                    true,
                    white(0.7),
                ),
            )
            .into(),
        ];
        if !self.expanded {
            let mut seats = self.seats.iter().collect::<Vec<_>>();
            seats.sort_by_key(|s| match s.activity {
                Activity::Errored => 0,
                Activity::Waiting(_) => 1,
                Activity::Active => 2,
                Activity::Idle => 3,
            });
            let mut roll = vec![];
            for (i, s) in seats
                .into_iter()
                .take(TOKENS.sidebar.collapse_glyphs as usize)
                .enumerate()
            {
                roll.push(
                    named(
                        format!(
                            "head/roll/glyph{}",
                            if i == 0 { String::new() } else { i.to_string() }
                        ),
                        c::Element::Glyph {
                            activity: s.activity,
                            grade: s.grade,
                            color: ui::Paint::solid(s.color(ctx)),
                            size: 9.,
                            scale: 1.,
                        },
                    )
                    .into(),
                );
            }
            if self.seats.len() > TOKENS.sidebar.collapse_glyphs as usize {
                let mut label = match text(
                    "…",
                    ctx.density.tokens().glyph as f32,
                    600,
                    false,
                    ui::Color::Muted,
                ) {
                    c::Element::Text(t) => t,
                    _ => unreachable!(),
                };
                label.runs[0].style.line_height = 1.;
                label.runs[0].style.tracking_em =
                    0.16 * STYLE.r#type.note as f32 / ctx.density.tokens().glyph as f32;
                roll.push(named("head/roll/ellip", c::Element::Text(label)).into());
            }
            children.push(
                named(
                    "head/roll",
                    flow(
                        plain(None, [0.; 4], ui::Color::Transparent, 0.),
                        Axis::Row,
                        STYLE.spacing.s1 as f32,
                        c::Alignment::Start,
                        roll,
                    ),
                )
                .into(),
            );
        }
        let mut right = vec![];
        let waiting = self
            .seats
            .iter()
            .filter(|s| matches!(s.activity, Activity::Waiting(_) | Activity::Errored))
            .count();
        if waiting > 0 && !self.expanded {
            let mut badge = plain(
                Some(14.),
                [0., 5., 0., 5.],
                ui::Color::Rgba(Rgba::hex(TOKENS.color.state.waiting).0),
                999.,
            );
            let min_width = 14.;
            let mut label = match text(
                waiting.to_string(),
                STYLE.r#type.micro as f32,
                600,
                true,
                ui::Color::Rgba([0., 0., 0., 1.]),
            ) {
                c::Element::Text(t) => t,
                _ => unreachable!(),
            };
            label.runs[0].style.line_height = 14. / STYLE.r#type.micro as f32;
            label.runs[0].style.tracking_em =
                0.16 * STYLE.r#type.note as f32 / STYLE.r#type.micro as f32;
            badge.overrides.radii = Some([ui::Length::Px(999.); 4]);
            right.push(c::Item {
                element: named(
                    "head/right/badge",
                    flow(
                        badge,
                        Axis::Row,
                        0.,
                        c::Alignment::Center,
                        vec![c::Element::Text(label).into()],
                    ),
                ),
                extent: c::Extent::Content {
                    grow: 0.,
                    shrink: 0.,
                    min: min_width,
                    max: None,
                    max_fraction: None,
                },
            });
        }
        let mut swatch = plain(Some(10.), [0.; 4], ui::Color::Rgba(self.color), 999.);
        swatch.overrides.width = Some(ui::Length::Px(10.));
        swatch.overrides.border = Some(ui::Border {
            width: ui::Length::Px(1.5),
            paint: ui::Paint::solid(white(0.85)),
            outside: false,
        });
        swatch.overrides.rings = Some(vec![ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(ui::Color::Rgba([0., 0., 0., 0.2])),
            outside: true,
        }]);
        right.push(
            named(
                "head/right/swatch",
                c::Element::Box(c::Box::new(swatch, vec![])),
            )
            .into(),
        );
        let right = named(
            "head/right",
            flow(
                plain(None, [0.; 4], ui::Color::Transparent, 0.),
                Axis::Row,
                ctx.density.tokens().gap as f32,
                c::Alignment::Center,
                right,
            ),
        );
        children.push(c::Item {
            element: c::Element::Stack(c::Stack {
                direction: Axis::Row,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Center,
                children: vec![c::Element::Spacer.into(), right.into()],
            }),
            extent: c::Extent::Content {
                grow: 1.,
                shrink: 0.,
                min: 0.,
                max: None,
                max_fraction: None,
            },
        });
        let mut style = plain(
            Some(TOKENS.strip.height as f32 + 2.),
            [0., STYLE.spacing.s6 as f32, 0., STYLE.spacing.s6 as f32],
            ui::Color::Rgba(self.color),
            TOKENS.radius.chip as f32,
        );
        style.overrides.material = Some(ui::Material::Gloss);
        let mut header = flow(
            style,
            Axis::Row,
            STYLE.spacing.s5 as f32,
            c::Alignment::Center,
            children,
        );
        if let c::Element::Box(b) = &mut header {
            b.cross_overflow = true;
        }
        named("head", dim(header, self.unfocused))
    }

    fn section(
        &self,
        label: &str,
        key: &str,
        ctx: ui::Context,
        count: Option<usize>,
    ) -> c::Element {
        let mut row = vec![];
        if key == "newseat" {
            let mut s = plain(None, [0.; 4], ui::Color::Transparent, 0.);
            s.overrides.width = Some(ui::Length::Px(12.));
            row.push(
                named(
                    format!("body/{key}/plus"),
                    flow(
                        s,
                        Axis::Row,
                        0.,
                        c::Alignment::Center,
                        vec![
                            c::Element::Spacer.into(),
                            text("+", STYLE.r#type.kind as f32, 500, false, ui::Color::Muted)
                                .into(),
                            c::Element::Spacer.into(),
                        ],
                    ),
                )
                .into(),
            );
        }
        row.push(c::Item {
            element: named(
                format!("body/{key}/tname"),
                text(
                    label,
                    ctx.density.tokens().font_small as f32,
                    if key == "newseat" { 500 } else { 600 },
                    false,
                    ui::Color::Muted,
                ),
            ),
            extent: c::Extent::Auto,
        });
        row.push(c::Element::Spacer.into());
        let right = named(
            format!("body/{key}/right"),
            flow(
                plain(None, [0.; 4], ui::Color::Transparent, 0.),
                Axis::Row,
                0.,
                c::Alignment::Center,
                vec![
                    named(
                        format!("body/{key}/right/kbd"),
                        text(
                            count.map_or("⌘T".into(), |n| n.to_string()),
                            ctx.density.tokens().font_small as f32,
                            if key == "newseat" { 500 } else { 600 },
                            true,
                            ui::Color::Muted,
                        ),
                    )
                    .into(),
                ],
            ),
        );
        row.push(right.into());
        let mut style = plain(
            Some(ctx.density.tokens().tab_h as f32),
            [
                0.,
                ctx.density.tokens().pad_x as f32,
                0.,
                ctx.density.tokens().pad_x as f32,
            ],
            ui::Color::Transparent,
            STYLE.radius.badge as f32,
        );
        style.overrides.padding = Some(
            [
                0.,
                ctx.density.tokens().pad_x as f32,
                0.,
                ctx.density.tokens().pad_x as f32,
            ]
            .map(ui::Length::Px),
        );
        let mut row = flow(
            style,
            Axis::Row,
            ctx.density.tokens().gap as f32,
            c::Alignment::Center,
            row,
        );
        if let c::Element::Box(b) = &mut row {
            b.cross_overflow = true;
        }
        let actual = inset(named(format!("body/{key}"), row), STYLE.spacing.s1 as f32);
        flow(
            plain(
                None,
                [STYLE.spacing.s3 as f32, 0., 0., 0.],
                ui::Color::Transparent,
                0.,
            ),
            Axis::Column,
            0.,
            c::Alignment::Stretch,
            vec![actual.into()],
        )
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let mut children = vec![self.header_element(ctx).into()];
        if !self.closed {
            children.push(
                c::Element::Box(c::Box::new(
                    plain(Some(3.), [0.; 4], ui::Color::Transparent, 0.),
                    vec![],
                ))
                .into(),
            );
        }
        if self.body {
            let mut body = vec![];
            for (i, tab) in self.tabs.iter().enumerate() {
                let mut e = tab.element_in_swarm(ctx);
                c::namespace(&mut e, &format!("body/tab{i}/"));
                body.push(e.into());
            }
            for (i, e) in self.extra.iter().enumerate() {
                let mut e = e.clone();
                c::namespace(&mut e, &format!("body/extra{i}/"));
                body.push(inset(e, STYLE.spacing.s1 as f32).into());
            }
            if self.new_seat {
                body.push(self.section("new seat", "newseat", ctx, None).into());
            }
            if !self.shadows.is_empty() {
                body.push(
                    self.section("Shadows", "shadows", ctx, Some(self.shadows.len()))
                        .into(),
                );
                for (i, row) in self.shadows.iter().enumerate() {
                    let mut e = row.element_with_radius(ctx, STYLE.radius.badge as f32);
                    c::namespace(&mut e, &format!("body/shadow{i}/"));
                    body.push(inset(e, STYLE.spacing.s1 as f32).into());
                }
            }
            let s = plain(
                None,
                [0., 0., STYLE.spacing.s2 as f32, 0.],
                ui::Color::Transparent,
                0.,
            );
            children.push(
                named(
                    "body",
                    dim(
                        flow(s, Axis::Column, 0., c::Alignment::Stretch, body),
                        self.unfocused,
                    ),
                )
                .into(),
            );
        }
        let surface = if self.unfocused {
            ui::Color::Rgba(
                Rgba::hex(ctx.theme.palette().surface)
                    .mix(
                        Rgba::hex(ctx.theme.palette().fg_dim),
                        TOKENS.dim.wash as f32,
                    )
                    .0,
            )
        } else {
            ui::Color::Surface
        };
        let inner = flow(
            plain(None, [4., 4., 0., 4.], ui::Color::Transparent, 0.),
            Axis::Column,
            0.,
            c::Alignment::Stretch,
            children,
        );
        let bumper = flow(
            plain(
                Some(if self.closed { 3. } else { 4. }),
                [0.; 4],
                ui::Color::Rgba(self.color),
                0.,
            ),
            Axis::Row,
            0.,
            c::Alignment::Stretch,
            vec![],
        );
        let style = card_style(surface);
        let mut frame = c::Box::new(style, vec![inner.into(), bumper.into()]);
        frame.direction = Axis::Column;
        frame.gap = ui::Length::Px(0.);
        frame.align = c::Alignment::Stretch;
        frame.clip = true;
        named("card", c::Element::Box(frame))
    }
}
#[derive(Debug, Clone)]
pub struct PendingSpawn {
    pub name: String,
    pub harness: String,
    pub status: String,
    pub caption: String,
    pub dismiss: bool,
}
pub fn pending_row(p: &PendingSpawn, ctx: ui::Context) -> c::Element {
    let d = ctx.density.tokens();
    let rejected = p.status == "rejected";
    let accepted = p.status == "accepted";
    let mark = if rejected {
        named("x", {
            let mut s = plain(None, [0.; 4], ui::Color::Transparent, 0.);
            s.overrides.width = Some(ui::Length::Px(12.));
            flow(
                s,
                Axis::Row,
                0.,
                c::Alignment::Center,
                vec![
                    text(
                        "×",
                        d.font_small as f32,
                        700,
                        false,
                        ui::Color::Rgba(Rgba::hex(TOKENS.color.state.errored).0),
                    )
                    .into(),
                ],
            )
        })
    } else {
        let mut ring = plain(Some(9.), [0.; 4], ui::Color::Transparent, 999.);
        ring.overrides.width = Some(ui::Length::Px(9.));
        ring.overrides.border = Some(ui::Border {
            width: ui::Length::Px(if accepted { 1.5 } else { 1.3 }),
            paint: ui::Paint {
                color: ui::Color::Muted,
                alpha: ui::Alpha::Value(if accepted { 0.3 } else { 1. }),
            },
            outside: false,
        });
        ring.overrides.dashed = Some(!accepted);
        let base = c::Element::Box(c::Box::new(ring.clone(), vec![]));
        let e = if accepted {
            ring.overrides.border.as_mut().unwrap().paint = ui::Paint::solid(ui::Color::Foreground);
            ring.overrides.border_sides = Some([true, false, false, false]);
            c::Element::Layers {
                base: Box::new(base),
                overlays: vec![c::Placed {
                    content: c::Element::Box(c::Box::new(ring, vec![])),
                    left: 0.,
                    bottom: 0.,
                }],
            }
        } else {
            base
        };
        named(if accepted { "spin" } else { "ring" }, e)
    };
    let top_margin = |e: c::Element| {
        flow(
            plain(None, [3., 0., 0., 0.], ui::Color::Transparent, 0.),
            Axis::Column,
            0.,
            c::Alignment::Start,
            vec![e.into()],
        )
    };
    let fg = Rgba::hex(ctx.theme.palette().fg_dim).0;
    let lum = fg[0] * 0.2126 + fg[1] * 0.7152 + fg[2] * 0.0722;
    let icon = named(
        "hicon",
        c::Element::Icon {
            name: p.harness.clone(),
            size: ui::Length::Px(d.icon as f32),
            paint: ui::Paint::solid(ui::Color::Rgba([lum, lum, lum, 0.55])),
        },
    );
    let mut title = match text(&p.name, d.font_small as f32, 400, false, ui::Color::Muted) {
        c::Element::Text(t) => t,
        _ => unreachable!(),
    };
    title.runs[0].style.line_height = 1.25;
    let mut top = vec![c::Item {
        element: named("pcol/ptop/tname", c::Element::Text(title)),
        extent: c::Extent::Content {
            grow: 1.,
            shrink: 1.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }];
    if p.dismiss && !accepted {
        let mut s = plain(None, [0., 7., 0., 7.], ui::Color::Transparent, 999.);
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(ui::Color::Line),
            outside: false,
        });
        let mut t = match text(
            "dismiss",
            STYLE.r#type.micro as f32,
            400,
            false,
            ui::Color::Foreground,
        ) {
            c::Element::Text(t) => t,
            _ => unreachable!(),
        };
        t.runs[0].style.line_height = 1.25;
        top.push(
            named(
                "pcol/ptop/undo",
                flow(
                    s,
                    Axis::Row,
                    0.,
                    c::Alignment::Center,
                    vec![c::Element::Text(t).into()],
                ),
            )
            .into(),
        );
    }
    let top = named(
        "pcol/ptop",
        flow(
            plain(None, [0.; 4], ui::Color::Transparent, 0.),
            Axis::Row,
            STYLE.spacing.s3 as f32,
            c::Alignment::Center,
            top,
        ),
    );
    let mut caption = c::Text::new(
        &p.caption,
        ui::Text {
            step: ui::TypeStep::Tiny,
            mono: true,
            line_height: 1.25,
            paint: ui::Paint::solid(ui::Color::Muted),
            ..Default::default()
        },
    );
    caption.wrap = true;
    caption.max_lines = Some(2);
    let column = named(
        "pcol",
        flow(
            plain(None, [0.; 4], ui::Color::Transparent, 0.),
            Axis::Column,
            0.,
            c::Alignment::Stretch,
            vec![
                top.into(),
                named("pcol/pcap", c::Element::Text(caption)).into(),
            ],
        ),
    );
    let row = flow(
        plain(None, [0.; 4], ui::Color::Transparent, 0.),
        Axis::Row,
        STYLE.spacing.s3 as f32,
        c::Alignment::Start,
        vec![
            top_margin(mark).into(),
            top_margin(icon).into(),
            c::Item {
                element: column,
                extent: c::Extent::Content {
                    grow: 1.,
                    shrink: 1.,
                    min: 0.,
                    max: None,
                    max_fraction: None,
                },
            },
        ],
    );
    let mut s = plain(
        None,
        [3., d.pad_x as f32, 3., d.pad_x as f32],
        if rejected {
            ui::Color::Rgba([fg[0], fg[1], fg[2], 0.1])
        } else {
            ui::Color::Transparent
        },
        STYLE.radius.badge as f32,
    );
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint::solid(if rejected {
            ui::Color::Transparent
        } else {
            ui::Color::Line
        }),
        outside: false,
    });
    s.overrides.dashed = Some(true);
    let mut outer = c::Overlay::new(row);
    outer.style = Box::new(s);
    outer.min_height = ui::Length::Px(d.tab_h as f32);
    outer.horizontal = c::Alignment::Stretch;
    outer.vertical = c::Alignment::Center;
    outer.clip = false;
    named("row", c::Element::Overlay(outer))
}
