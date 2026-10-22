//! A viewport of supplied terminal spans and history notices. No parser, replay,
//! process state machine or controller lives in this composition.
use super::{
    seat_row::named,
    strip::{Strip, paint, plain, text},
};
use crate::{self as ui, components as c};
use bbt_ui_model::{Activity, Grade, STYLE, TOKENS, layout::Axis};

#[derive(Debug, Clone)]
pub enum Span {
    Text {
        key: String,
        value: String,
        foreground: bool,
        hover: bool,
    },
    Cursor {
        key: String,
    },
    Hint {
        key: String,
        parts: Vec<(String, String, bool)>,
    },
}
#[derive(Debug, Clone)]
pub struct Resumed {
    pub generation: u64,
    pub command: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Pane {
    pub strip: Strip,
    pub lines: Vec<Vec<Span>>,
    /// None: no gap. Some(None): a gap with no retained timestamp supplied.
    pub gap_from: Option<Option<String>>,
    pub resumed: Option<Resumed>,
    pub min_height: f32,
    pub unfocused: bool,
    pub reconnecting: bool,
    pub reconnected: bool,
    /// The host freezes/advances the effect clock. A seed identifies the emitter.
    pub glow_seed: Option<u32>,
    pub opacity: f32,
}
fn column(children: Vec<c::Item>) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: Axis::Column,
        gap: ui::Length::Px(0.),
        align: c::Alignment::Stretch,
        children,
    })
}
fn fixed_box(mut style: ui::Style, width: f32) -> c::Element {
    style.overrides.width = Some(ui::Length::Px(width));
    c::Element::Box(c::Box::new(style, vec![]))
}
fn line(key: &str, caption: String, resumed: bool) -> c::Element {
    let color = if resumed {
        ui::Color::Accent
    } else {
        ui::Color::Muted
    };
    let mut children = vec![];
    for side in ["i", "span", "i1"] {
        if side == "span" {
            let mut label = text(
                &caption,
                STYLE.r#type.small as f32,
                400,
                true,
                color,
                1.,
                1.5,
            );
            if let c::Element::Text(t) = &mut label {
                t.runs[0].style.tracking_em = 0.02;
                t.ellipsis = false;
            }
            children.push(named(format!("{key}/span"), label).into());
        } else {
            let mut s = plain(Some(1.), [0.; 4], 0.);
            s.overrides.fill = Some(if resumed {
                paint(ui::Color::Accent, 0.4)
            } else {
                ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                }
            });
            children.push(c::Item {
                element: named(
                    format!("{key}/{side}"),
                    c::Element::Box(c::Box::new(s, vec![])),
                ),
                extent: c::Extent::Content {
                    grow: 1.,
                    shrink: 1.,
                    min: 12.,
                    max: None,
                    max_fraction: None,
                },
            });
        }
    }
    let row = named(
        key,
        c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Spacing(4),
            align: c::Alignment::Center,
            children,
        }),
    );
    column(vec![
        row.into(),
        c::Item {
            element: c::Element::Spacer,
            extent: c::Extent::Fixed(STYLE.spacing.s3 as f32),
        },
    ])
}
fn span(value: &Span) -> c::Element {
    let size = STYLE.r#type.section as f32;
    match value {
        Span::Text {
            key,
            value,
            foreground,
            hover,
        } => {
            // CSS white-space:pre preserves leading blanks. Retained Text's
            // collapsible-space mode uses NBSP for the identical mono advance.
            let value = value
                .chars()
                .map(|ch| if ch == ' ' { '\u{a0}' } else { ch })
                .collect::<String>();
            let mut label = text(
                value,
                size,
                400,
                true,
                if *foreground {
                    ui::Color::Foreground
                } else {
                    ui::Color::Muted
                },
                1.,
                1.5,
            );
            if let c::Element::Text(t) = &mut label {
                t.inline = true;
                t.ellipsis = false;
            }
            let label = named(key, label);
            if *hover {
                let mut s = plain(None, [0.; 4], STYLE.radius.mark as f32);
                s.overrides.fill = Some(paint(ui::Color::Accent, 0.12));
                let mut b = c::Box::new(s, vec![label.into()]);
                b.gap = ui::Length::Px(0.);
                // The underline is an overlay, so it takes no terminal space.
                let mut underline = plain(Some(1.5), [0.; 4], 0.);
                underline.overrides.fill = Some(ui::Paint::solid(ui::Color::Accent));
                let mut overlay = c::Overlay::new(c::Element::Box(c::Box::new(underline, vec![])));
                overlay.style = Box::new(plain(None, [0.; 4], 0.));
                overlay.min_height = ui::Length::Px(0.);
                overlay.horizontal = c::Alignment::Stretch;
                overlay.vertical = c::Alignment::End;
                c::Element::Layers {
                    base: Box::new(c::Element::Box(b)),
                    overlays: vec![c::Placed {
                        content: c::Element::Overlay(overlay),
                        left: 0.,
                        bottom: 0.,
                    }],
                }
            } else {
                label
            }
        }
        Span::Cursor { key } => {
            let mut s = plain(Some(12.), [0.; 4], 0.);
            s.overrides.fill = Some(ui::Paint::solid(ui::Color::Foreground));
            c::Element::Baseline {
                content: Box::new(named(key, fixed_box(s, 7.))),
                offset: 10.,
            }
        }
        Span::Hint { key, parts } => {
            let mut s = plain(
                None,
                [0., STYLE.spacing.s4 as f32, 0., STYLE.spacing.s4 as f32],
                STYLE.radius.pill as f32,
            );
            s.overrides.fill = Some(paint(ui::Color::Surface, 0.92));
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
            s.overrides.text = Some(ui::Text {
                size: Some(STYLE.r#type.small as f32),
                line_height: 1.5,
                ..Default::default()
            });
            let children = parts
                .iter()
                .map(|(key, value, kbd)| {
                    let mut t = text(
                        value,
                        STYLE.r#type.small as f32,
                        400,
                        false,
                        if *kbd {
                            ui::Color::Muted
                        } else {
                            ui::Color::Foreground
                        },
                        1.,
                        1.5,
                    );
                    if let c::Element::Text(t) = &mut t {
                        t.inline = true;
                        t.ellipsis = false;
                    }
                    named(key, t).into()
                })
                .collect();
            let mut b = c::Box::new(s, children);
            b.inline = true;
            b.gap = ui::Length::Px(0.);
            b.align = c::Alignment::Baseline;
            let label = c::Element::Baseline {
                content: Box::new(named(key, c::Element::Box(b))),
                offset: 1.,
            };
            c::Element::Stack(c::Stack {
                direction: Axis::Row,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Baseline,
                children: vec![
                    c::Item {
                        element: c::Element::Spacer,
                        extent: c::Extent::Fixed(STYLE.spacing.s4 as f32),
                    },
                    label.into(),
                ],
            })
        }
    }
}
impl Pane {
    pub fn height(&self) -> f32 {
        let border = if self.strip.seat.activity == Activity::Active {
            TOKENS.strip.border_active
        } else {
            TOKENS.strip.border_idle
        } as f32;
        let notices = usize::from(self.gap_from.is_some()) + usize::from(self.resumed.is_some());
        let content = TOKENS.strip.height as f32
            + 2. * STYLE.spacing.s4 as f32
            + self.lines.len() as f32 * STYLE.r#type.section as f32 * 1.5
            + notices as f32 * (STYLE.r#type.small as f32 * 1.5 + STYLE.spacing.s3 as f32)
            + 2. * border;
        self.min_height.max(content)
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let seat = &self.strip.seat;
        let mut blocks = vec![];
        if let Some(from) = &self.gap_from {
            let caption = from
                .as_ref()
                .map(|from| format!("earlier output evicted · retained from {from}"))
                .unwrap_or_else(|| "earlier output unavailable".into());
            blocks.push(line("pane/term/gap-mark", caption, false).into());
        }
        if let Some(resumed) = &self.resumed {
            blocks.push(
                line(
                    if self.gap_from.is_some() {
                        "pane/term/gap-mark1"
                    } else {
                        "pane/term/gap-mark"
                    },
                    format!(
                        "resumed · process generation {} · {}",
                        resumed.generation,
                        resumed
                            .command
                            .as_deref()
                            .unwrap_or("new shell, old output above")
                    ),
                    true,
                )
                .into(),
            );
        }
        for row in &self.lines {
            // The inherited mono strut keeps empty/cursor-only lines 18px high.
            let mut style = plain(Some(STYLE.r#type.section as f32 * 1.5), [0.; 4], 0.);
            style.overrides.text = Some(ui::Text {
                size: Some(STYLE.r#type.section as f32),
                mono: true,
                line_height: 1.5,
                ..Default::default()
            });
            let mut b = c::Box::new(style, row.iter().map(|s| span(s).into()).collect());
            b.inline = true;
            b.gap = ui::Length::Px(0.);
            b.align = c::Alignment::Baseline;
            b.cross_overflow = true;
            blocks.push(c::Element::Box(b).into());
        }
        let mut term = c::Box::new(
            plain(None, [STYLE.spacing.s4 as f32; 4], 0.),
            vec![column(blocks).into()],
        );
        term.direction = Axis::Column;
        term.align = c::Alignment::Stretch;
        term.gap = ui::Length::Px(0.);
        term.clip = true;
        let mut term = named("pane/term", c::Element::Box(term));
        let mut strip = self.strip.element(ctx);
        if self.unfocused || self.reconnecting {
            let filter = |content| c::Element::Filter {
                content: Box::new(content),
                saturation: TOKENS.dim.saturate as f32,
                opacity: TOKENS.dim.opacity as f32,
            };
            strip = filter(strip);
            term = filter(term);
        }
        let border = if seat.activity == Activity::Active {
            TOKENS.strip.border_active
        } else {
            TOKENS.strip.border_idle
        } as f32;
        let mut style = ui::Style::new(ui::Preset::PaneGlow);
        style.overrides.height = Some(ui::Length::Px(self.height()));
        style.overrides.padding = Some([ui::Length::Px(0.); 4]);
        let mut bg = bbt_ui_model::draw::Color::hex(ctx.theme.palette().bg);
        if self.unfocused {
            bg = bg.mix(
                bbt_ui_model::draw::Color::hex(ctx.theme.palette().fg_dim),
                TOKENS.dim.wash as f32,
            );
        }
        bg.0[3] = self.opacity;
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(bg.0)));
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(border),
            paint: ui::Paint::solid(if seat.activity == Activity::Errored {
                ui::Color::Rgba(bbt_ui_model::draw::Color::hex(TOKENS.color.state.errored).0)
            } else {
                seat.color(ctx)
            }),
            outside: false,
        });
        let alpha = match seat.activity {
            Activity::Idle => TOKENS.glow.idle_alpha,
            Activity::Active => TOKENS.glow.active_alpha_min,
            Activity::Waiting(_) => TOKENS.glow.waiting_alpha_min,
            Activity::Errored => TOKENS.glow.errored_alpha,
        } as f32;
        let boost = if seat.focused {
            TOKENS.glow.focused_boost as f32
        } else {
            1.
        };
        let particles = if self.glow_seed.is_none() || self.reconnecting {
            0
        } else {
            match seat.activity {
                Activity::Active => TOKENS.glow.particles_active as u32,
                Activity::Waiting(_) => TOKENS.glow.particles_idle as u32 * 2,
                _ => TOKENS.glow.particles_idle as u32,
            }
        };
        style.overrides.glow = Some(ui::Glow {
            activity: seat.activity.into(),
            inferred_multiplier: if seat.grade == Grade::Inferred {
                0.5
            } else {
                1.
            },
            seed: self.glow_seed.unwrap_or(7),
            halo_alpha: Some((alpha * boost).min(1.)),
            particles: Some(particles),
        });
        // The lab emitter sits behind the pane. Compose the effect before the
        // opaque frame so jittered particles cannot paint over terminal text.
        let mut fx = style.clone();
        fx.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
        fx.overrides.border = Some(ui::Border {
            width: ui::Length::Px(0.),
            paint: ui::Paint::solid(ui::Color::Transparent),
            outside: false,
        });
        style.overrides.material = Some(ui::Material::Flat);
        style.overrides.glow = None;
        let effect = c::Element::Box(c::Box::new(fx, vec![]));
        let mut b = c::Box::new(
            style,
            vec![
                c::Item {
                    element: strip,
                    extent: c::Extent::Fixed(TOKENS.strip.height as f32),
                },
                c::Item {
                    element: term,
                    extent: c::Extent::Fill,
                },
            ],
        );
        b.direction = Axis::Column;
        b.gap = ui::Length::Px(0.);
        b.align = c::Alignment::Stretch;
        b.clip = true;
        let mut cover = c::Overlay::new(c::Element::Box(b));
        cover.style = Box::new(plain(None, [0.; 4], 0.));
        cover.min_height = ui::Length::Px(self.height());
        cover.horizontal = c::Alignment::Stretch;
        cover.vertical = c::Alignment::Stretch;
        cover.clip = false;
        let base = c::Element::Layers {
            base: Box::new(effect),
            overlays: vec![c::Placed {
                content: c::Element::Overlay(cover),
                left: 0.,
                bottom: 0.,
            }],
        };
        let overlays = if self.reconnecting {
            let mut ring = plain(Some(10.), [0.; 4], 5.);
            ring.overrides.width = Some(ui::Length::Px(10.));
            ring.overrides.border = Some(ui::Border {
                width: ui::Length::Px(1.5),
                paint: paint(ui::Color::Foreground, 0.25),
                outside: false,
            });
            let mut top = plain(Some(1.5), [0.; 4], 0.);
            top.overrides.width = Some(ui::Length::Px(10.));
            top.overrides.fill = Some(ui::Paint::solid(ui::Color::Foreground));
            let spinner = c::Element::Layers {
                base: Box::new(c::Element::Box(c::Box::new(ring, vec![]))),
                overlays: vec![c::Placed {
                    content: c::Element::Box(c::Box::new(top, vec![])),
                    left: 0.,
                    bottom: 8.5,
                }],
            };
            let mut mask = plain(Some(10.), [0.; 4], 5.);
            mask.overrides.width = Some(ui::Length::Px(10.));
            let mut mask = c::Box::new(mask, vec![spinner.into()]);
            mask.clip = true;
            mask.gap = ui::Length::Px(0.);
            let spinner = c::Element::Box(mask);
            let mut pill = ui::Style::new(ui::Preset::Pill);
            pill.overrides.height = Some(ui::Length::Px(STYLE.r#type.caption as f32 * 1.4 + 12.));
            pill.overrides.padding = Some([
                ui::Length::Px(5.),
                ui::Length::Spacing(6),
                ui::Length::Px(5.),
                ui::Length::Spacing(6),
            ]);
            pill.overrides.radii = Some([ui::Length::Radius(ui::Radius::Pill); 4]);
            pill.overrides.material = Some(ui::Material::Frosted);
            pill.overrides.fill = Some(ui::Paint {
                color: ui::Color::Surface,
                alpha: ui::Alpha::Panel,
            });
            pill.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
            pill.overrides.shadow = Some(ui::Shadow::Pill);
            let mut body = c::Box::new(
                pill,
                vec![
                    named("pane/reconnect/spin", spinner).into(),
                    named(
                        "pane/reconnect/text",
                        text(
                            "reconnecting…",
                            STYLE.r#type.caption as f32,
                            400,
                            false,
                            ui::Color::Foreground,
                            1.,
                            1.4,
                        ),
                    )
                    .into(),
                ],
            );
            body.gap = ui::Length::Px(7.);
            let mut overlay = c::Overlay::new(named("pane/reconnect", c::Element::Box(body)));
            overlay.style = Box::new(plain(None, [0.; 4], 0.));
            overlay.min_height = ui::Length::Px(self.height());
            overlay.clip = false;
            vec![c::Placed {
                content: c::Element::Overlay(overlay),
                left: 0.,
                bottom: 0.,
            }]
        } else {
            vec![]
        };
        named(
            "pane",
            c::Element::Layers {
                base: Box::new(base),
                overlays,
            },
        )
    }
}
