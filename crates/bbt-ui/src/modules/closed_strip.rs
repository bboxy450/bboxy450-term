//! The T7 grace strip. Its owner supplies remaining time and handles undo;
//! this retained component never runs a timer or changes seat lifecycle.
use super::{
    seat_row::{keyboard_badge, named},
    swarm_card::{flow, plain, text},
};
use crate::{self as ui, components as c};
use bbt_ui_model::{
    STYLE, TOKENS,
    layout::{Axis, TextStyle as MeasureStyle},
};

#[derive(Debug, Clone)]
pub struct ClosedStrip {
    pub key: String,
    pub name: String,
    pub harness: String,
    pub remaining_ms: u64,
    pub grace_ms: u64,
}
impl ClosedStrip {
    pub fn element(&self, ctx: ui::Context, measure: &dyn c::Measure, width: f32) -> c::Element {
        if self.remaining_ms == 0 || self.grace_ms == 0 {
            return c::Element::Spacer;
        }
        let d = ctx.density.tokens();
        let h = d.tab_h as f32;
        let gap = d.gap as f32;
        let pad = STYLE.spacing.s4 as f32;
        let rgba = |s: &str| ui::Color::Rgba(bbt_ui_model::draw::Color::hex(s).0);
        let paint = |color, alpha| ui::Paint {
            color,
            alpha: ui::Alpha::Value(alpha),
        };
        let error = rgba(TOKENS.color.state.errored);
        let mut root = plain(
            Some(h),
            [0., pad, 0., pad],
            ui::Color::Transparent,
            STYLE.radius.row as f32,
        );
        root.overrides.fill = Some(paint(
            ui::Color::Foreground,
            STYLE.tint.selection_hover_alpha as f32,
        ));
        let mut button_style = plain(
            None,
            [0., 7., 0., 7.],
            ui::Color::Transparent,
            STYLE.radius.pill as f32,
        );
        button_style.overrides.fill =
            Some(paint(ui::Color::Surface, STYLE.material.panel_alpha as f32));
        button_style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        let mut undo = text(
            "undo ⌘Z",
            STYLE.r#type.small as f32,
            400,
            false,
            ui::Color::Foreground,
        );
        if let c::Element::Text(t) = &mut undo {
            t.runs[0].style.tracking_em = 0.08;
            t.wrap = true;
            t.ellipsis = false;
            t.align = c::Alignment::Center;
        }
        let right = flow(
            plain(None, [0.; 4], ui::Color::Transparent, 0.),
            Axis::Row,
            gap,
            c::Alignment::Center,
            vec![
                named(
                    "left",
                    keyboard_badge(
                        format!("{} s", self.remaining_ms.div_ceil(1000)),
                        false,
                        false,
                        400,
                    ),
                )
                .into(),
                c::Item {
                    element: named(
                        "undo",
                        flow(
                            button_style,
                            Axis::Row,
                            0.,
                            c::Alignment::Center,
                            vec![undo.into()],
                        ),
                    ),
                    extent: c::Extent::MinContent,
                },
            ],
        );
        let rw = right
            .natural(f32::INFINITY, ctx, measure)
            .expect("grace right")
            .0[0];
        let mut name = text(
            &self.name,
            d.font_small as f32,
            400,
            false,
            ui::Color::Foreground,
        );
        let nw = name
            .natural(f32::INFINITY, ctx, measure)
            .expect("grace name")
            .0[0]
            .min((width - 2. * pad - 12. - d.icon as f32 - rw - 3. * gap).max(0.));
        if let c::Element::Text(t) = &mut name {
            t.ellipsis = true;
            t.wrap = false;
        }
        let intrinsic = name.natural(nw, ctx, measure).expect("grace name");
        let font = MeasureStyle {
            size: d.font_small as f32,
            weight: 400,
            mono: false,
            italic: false,
            tracking_em: 0.,
        };
        let y = intrinsic.1 - measure.x_height(&font) * 0.5;
        let name = c::Element::Layers {
            base: Box::new(flow(
                plain(None, [0.; 4], ui::Color::Transparent, 0.),
                Axis::Row,
                0.,
                c::Alignment::Center,
                vec![c::Item {
                    element: name,
                    extent: c::Extent::Fill,
                }],
            )),
            overlays: vec![c::Placed {
                left: 0.,
                bottom: 0.,
                content: c::Element::Path(c::Path {
                    size: [nw, intrinsic.0[1]],
                    points: vec![c::PathCommand::Move([0., y]), c::PathCommand::Line([nw, y])]
                        .into(),
                    stroke: c::PathStroke::default(),
                    paint: paint(ui::Color::Muted, 0.6),
                    fill: None,
                    opacity: 1.,
                }),
            }],
        };
        let children = vec![
            c::Item {
                element: named(
                    "x",
                    flow(
                        plain(None, [0.; 4], ui::Color::Transparent, 0.),
                        Axis::Row,
                        0.,
                        c::Alignment::Center,
                        vec![
                            c::Item {
                                element: c::Element::Spacer,
                                extent: c::Extent::Fill,
                            },
                            text("×", d.font_small as f32, 700, false, error).into(),
                            c::Item {
                                element: c::Element::Spacer,
                                extent: c::Extent::Fill,
                            },
                        ],
                    ),
                ),
                extent: c::Extent::Fixed(12.),
            },
            named(
                "hicon",
                c::Element::Filter {
                    content: Box::new(c::Element::Icon {
                        name: self.harness.clone(),
                        paint: ui::Paint::solid(ui::Color::Muted),
                        size: ui::Length::Px(d.icon as f32),
                    }),
                    saturation: 0.,
                    opacity: 0.5,
                },
            )
            .into(),
            c::Item {
                element: named("tname", name),
                extent: c::Extent::Content {
                    grow: 0.,
                    shrink: 1.,
                    min: 0.,
                    max: None,
                    max_fraction: None,
                },
            },
            c::Item {
                element: c::Element::Stack(c::Stack {
                    direction: Axis::Row,
                    gap: ui::Length::Px(0.),
                    align: c::Alignment::Center,
                    children: vec![
                        c::Item {
                            element: c::Element::Spacer,
                            extent: c::Extent::Fill,
                        },
                        c::Item {
                            element: named("right", right),
                            extent: c::Extent::MinContent,
                        },
                    ],
                }),
                extent: c::Extent::Content {
                    grow: 1.,
                    shrink: 1.,
                    min: 0.,
                    max: None,
                    max_fraction: None,
                },
            },
        ];
        let row = flow(root, Axis::Row, gap, c::Alignment::Center, children);
        let fraction = (self.remaining_ms as f64 / self.grace_ms as f64).clamp(0., 1.) as f32;
        let mut element = c::Element::Layers {
            base: Box::new(row),
            overlays: vec![c::Placed {
                left: 0.,
                bottom: 0.,
                content: named(
                    "grace",
                    c::Element::Rects(c::Rects {
                        repeat: None,
                        size: [width * fraction, 2.],
                        rects: vec![(
                            bbt_ui_model::layout::Rect::new(0., 0., width * fraction, 2.),
                            paint(error, 0.7),
                        )]
                        .into(),
                    }),
                ),
            }],
        };
        let mut clip = c::Box::new(
            plain(
                Some(h),
                [0.; 4],
                ui::Color::Transparent,
                STYLE.radius.row as f32,
            ),
            vec![c::Item {
                element,
                extent: c::Extent::Fill,
            }],
        );
        clip.clip = true;
        element = c::Element::Box(clip);
        c::namespace(&mut element, &format!("closed/{}/", self.key));
        named(format!("closed/{}", self.key), element).fixed_height(
            h,
            format!("closed/{}", self.key),
            0.,
        )
    }
}
