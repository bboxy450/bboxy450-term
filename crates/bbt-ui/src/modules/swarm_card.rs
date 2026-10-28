// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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

/// Declare only heights already fixed by the composition. An unknown-height
/// child leaves its ancestors undeclared, selecting ScrollView's old path.
pub(crate) fn declare_fixed(
    mut e: c::Element,
    ctx: ui::Context,
    keys: &std::collections::HashMap<String, String>,
) -> c::Element {
    let mut key = String::new();
    let declared = match &mut e {
        c::Element::FixedHeight { .. } => return e,
        c::Element::Named { name, content } => {
            key = keys.get(name).cloned().unwrap_or_default();
            **content = declare_fixed(
                std::mem::replace(content.as_mut(), c::Element::Spacer),
                ctx,
                keys,
            );
            fixed_dimensions(content)
        }
        c::Element::Filter { content, .. } => {
            **content = declare_fixed(
                std::mem::replace(content.as_mut(), c::Element::Spacer),
                ctx,
                keys,
            );
            fixed_dimensions(content)
        }
        c::Element::Layers { base, .. } => {
            **base = declare_fixed(
                std::mem::replace(base.as_mut(), c::Element::Spacer),
                ctx,
                keys,
            );
            fixed_dimensions(base)
        }
        c::Element::Box(b) => {
            // Invalid input keeps the normal layout validation/error path.
            let Ok(s) = b.style.resolve(ctx) else {
                return e;
            };
            let outset = s
                .shadow
                .as_ref()
                .map_or(0., |s| {
                    s.blur * 1.5 + s.offset[0].abs().max(s.offset[1].abs())
                })
                .max(s.rings.iter().map(|r| r.width).fold(0., f32::max))
                .max(s.border.as_ref().map_or(0., |b| b.width));
            if let Some(height) = s.height {
                Some((height, outset))
            } else if b.direction == Axis::Column
                && b.align == c::Alignment::Stretch
                && !b.inline
                && b.scale == 1.
            {
                for item in &mut b.children {
                    item.element = declare_fixed(
                        std::mem::replace(&mut item.element, c::Element::Spacer),
                        ctx,
                        keys,
                    );
                }
                let Ok(gap) = b.gap.resolve(ctx.density) else {
                    return e;
                };
                fixed_column(&b.children, gap).map(|(h, child_outset)| {
                    let border = s
                        .border
                        .as_ref()
                        .filter(|b| !b.outside)
                        .map_or(0., |b| b.width);
                    let top = s.padding[0] + if s.border_sides[0] { border } else { 0. };
                    let bottom = s.padding[2] + if s.border_sides[2] { border } else { 0. };
                    (h + top + bottom, outset.max(child_outset))
                })
            } else {
                None
            }
        }
        c::Element::Stack(s) if s.direction == Axis::Column && s.align == c::Alignment::Stretch => {
            for item in &mut s.children {
                item.element = declare_fixed(
                    std::mem::replace(&mut item.element, c::Element::Spacer),
                    ctx,
                    keys,
                );
            }
            let Ok(gap) = s.gap.resolve(ctx.density) else {
                return e;
            };
            fixed_column(&s.children, gap)
        }
        _ => None,
    };
    if let Some((height, outset)) = declared {
        e.fixed_height(height, key, outset)
    } else {
        e
    }
}
fn fixed_dimensions(e: &c::Element) -> Option<(f32, f32)> {
    match e {
        c::Element::FixedHeight {
            height,
            paint_outset,
            ..
        } => Some((*height, *paint_outset)),
        _ => None,
    }
}
fn fixed_column(items: &[c::Item], gap: f32) -> Option<(f32, f32)> {
    let mut height = 0.;
    let mut outset: f32 = 0.;
    for item in items {
        let (h, o) = fixed_dimensions(&item.element)?;
        height += match item.extent {
            c::Extent::Auto => h,
            c::Extent::Fixed(h) => h,
            _ => return None,
        };
        outset = outset.max(o);
    }
    Some((height + gap * items.len().saturating_sub(1) as f32, outset))
}

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
    let mut s = ui::Style::new(ui::Preset::SwarmCard);
    s.overrides.padding = Some([ui::Length::Px(0.); 4]);
    s.overrides.fill = Some(ui::Paint {
        color: fill,
        alpha: ui::Alpha::Value(STYLE.material.card_alpha as f32),
    });
    s
}
fn dim(mut element: c::Element, unfocused: bool) -> c::Element {
    if unfocused && let c::Element::Box(node) = &mut element {
        node.style.overrides.dim = Some(true);
    }
    element
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
        let band = ui::Style::new(ui::Preset::Band)
            .resolve(ctx)
            .expect("band Style");
        let white = |a: f32| ui::Color::Rgba([1., 1., 1., a]);
        let mut title = match text(
            &self.name,
            band.text.size,
            band.text.weight,
            false,
            white(0.92),
        ) {
            c::Element::Text(t) => t,
            _ => unreachable!(),
        };
        title.wrap = true;
        title.ellipsis = false;
        title.runs[0].style.uppercase = band.text.uppercase;
        title.runs[0].style.tracking_em = band.text.tracking_em;
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
                            kind_colors: Default::default(),
                            shadow: None,
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
        let mut style = ui::Style::new(ui::Preset::Band);
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(self.color)));
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
                    500,
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
                        super::seat_row::keyboard_badge(
                            count.map_or("⌘T".into(), |n| n.to_string()),
                            false,
                            key == "shadows",
                            500,
                        ),
                    )
                    .into(),
                ],
            ),
        );
        row.push(right.into());
        let mut style = ui::Style::new(ui::Preset::CardRow);
        style.overrides.height = Some(ui::Length::Px(ctx.density.tokens().tab_h as f32));
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
        let chrome = ui::Style::new(ui::Preset::SwarmCard)
            .resolve(ctx)
            .expect("swarm Style");
        let row_style = ui::Style::new(ui::Preset::CardRow)
            .resolve(ctx)
            .expect("row Style");
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
                body.push(inset(e, row_style.margin[3]).into());
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
                    let mut e = row.element_in_swarm(ctx);
                    c::namespace(&mut e, &format!("body/shadow{i}/"));
                    body.push(inset(e, row_style.margin[3]).into());
                }
            }
            let s = plain(None, chrome.body_padding, ui::Color::Transparent, 0.);
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
            plain(None, chrome.padding, ui::Color::Transparent, 0.),
            Axis::Column,
            0.,
            c::Alignment::Stretch,
            children,
        );
        let bumper = flow(
            plain(
                Some(chrome.bumper[usize::from(self.closed)]),
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
        let mut keys = std::collections::HashMap::from([
            ("head".into(), format!("swarm/{}/head", self.key)),
            ("body/newseat".into(), format!("swarm/{}/newseat", self.key)),
            ("body/shadows".into(), format!("swarm/{}/shadows", self.key)),
        ]);
        for (i, tab) in self.tabs.iter().enumerate() {
            // A tab's header follows its seat identities, not its array index.
            let identity = tab
                .seats
                .iter()
                .map(|s| s.row.key.as_str())
                .collect::<Vec<_>>()
                .join("/");
            keys.insert(
                format!("body/tab{i}/header"),
                format!("swarm/{}/tab/{identity}", self.key),
            );
            for (j, seat) in tab.seats.iter().enumerate() {
                keys.insert(
                    format!("body/tab{i}/seat{j}/row"),
                    format!("seat/{}", seat.row.key),
                );
            }
        }
        for (i, row) in self.shadows.iter().enumerate() {
            keys.insert(format!("body/shadow{i}/row"), format!("shadow/{}", row.key));
        }
        declare_fixed(named("card", c::Element::Box(frame)), ctx, &keys)
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

/// Host-owned launch default, using the card's existing row tokens.
pub fn workdir_row(key: &str, path: Option<&str>, ctx: ui::Context) -> c::Element {
    let label=path.map_or_else(||"Choose cwd…".into(), |path| {
        let name=std::path::Path::new(path).file_name().and_then(|name|name.to_str()).unwrap_or(path);
        format!("cwd: {name}")
    });
    let button=|action:&str,label:String| named(format!("swarm-folder/{key}/{action}"),
        flow(plain(Some(ctx.density.tokens().tab_h as f32),[0.,STYLE.spacing.s4 as f32,0.,STYLE.spacing.s4 as f32],ui::Color::Transparent,STYLE.radius.row as f32),
            Axis::Row,0.,c::Alignment::Center,vec![text(label,STYLE.r#type.small as f32,400,false,ui::Color::Foreground).into()]));
    let mut children=vec![c::Item {element:button("choose",label),extent:c::Extent::Flex(1.)}];
    if path.is_some() {children.push(button("default","Default".into()).into());}
    flow(plain(None,[0.;4],ui::Color::Transparent,0.),Axis::Row,0.,c::Alignment::Center,children)
}
pub fn pending_row(p: &PendingSpawn, ctx: ui::Context) -> c::Element {
    let d = ctx.density.tokens();
    let card_row = ui::Style::new(ui::Preset::CardRow)
        .resolve(ctx)
        .expect("card row Style");
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
    // CSS centers the text column but aligns the status mark and icon to
    // the row's top edge, even when min-height adds spare vertical space.
    let mut column_center = c::Overlay::new(column);
    column_center.style = Box::new(plain(None, [0.; 4], ui::Color::Transparent, 0.));
    column_center.min_height = ui::Length::Px(0.);
    column_center.horizontal = c::Alignment::Stretch;
    column_center.vertical = c::Alignment::Center;
    column_center.clip = false;
    let row = flow(
        plain(None, [0.; 4], ui::Color::Transparent, 0.),
        Axis::Row,
        STYLE.spacing.s3 as f32,
        c::Alignment::Stretch,
        vec![
            top_margin(mark).into(),
            top_margin(icon).into(),
            c::Item {
                element: c::Element::Overlay(column_center),
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
        [3., card_row.padding[1], 3., card_row.padding[3]],
        if rejected {
            ui::Color::Rgba([fg[0], fg[1], fg[2], 0.1])
        } else {
            ui::Color::Transparent
        },
        card_row.radii[0],
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
    outer.vertical = c::Alignment::Stretch;
    outer.clip = false;
    named("row", c::Element::Overlay(outer))
}
