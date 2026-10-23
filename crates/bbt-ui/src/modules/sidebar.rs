//! A bounded sidebar viewport over swarm cards. Scroll offset and all actions
//! are caller data; no autonomous focus, navigation or lifecycle handling.
use crate::{
    self as ui, components as c,
    modules::{
        seat_row::named,
        swarm_card::{SwarmCard, declare_fixed, flow, plain, text},
    },
};
use bbt_ui_model::{STYLE, layout::Axis};
#[derive(Debug, Clone)]
pub struct Sidebar {
    pub cards: Vec<SwarmCard>,
    pub size: [f32; 2],
    pub scroll: f32,
    pub phone: bool,
    pub new_swarm: bool,
}
impl Sidebar {
    /// Capture before replacing the caller's model or density, then pass the
    /// snapshot to element_anchored. An undeclared subtree returns None.
    pub fn anchor(&self, ctx: ui::Context) -> crate::style::Result<Option<c::ScrollAnchor>> {
        fn find(e: &c::Element, ctx: ui::Context) -> crate::style::Result<Option<c::ScrollAnchor>> {
            match e {
                c::Element::ScrollView(s) => s.anchor(ctx),
                c::Element::Named { content, .. } => find(content, ctx),
                c::Element::Box(b) => {
                    for item in &b.children {
                        if let Some(anchor) = find(&item.element, ctx)? {
                            return Ok(Some(anchor));
                        }
                    }
                    Ok(None)
                }
                _ => Ok(None),
            }
        }
        find(&self.element(ctx), ctx)
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        self.element_anchored(ctx, None)
    }
    /// Rebuild after a content, width or density change while preserving the
    /// caller's first visible stable item. No retained height estimates.
    pub fn element_anchored(
        &self,
        ctx: ui::Context,
        anchor: Option<&c::ScrollAnchor>,
    ) -> c::Element {
        let border = STYLE.material.hairline_px as f32;
        // The sidebar's own left inset (the pane gutter now follows the glow lane).
        let edge = bbt_ui_model::TOKENS.sidebar.edge as f32;
        let width = self.size[0] - if self.phone { 0. } else { border } - edge;
        let top_height = bbt_ui_model::TOKENS.strip.height as f32;
        let handle_height = if self.phone { 44. } else { 0. };
        let scroll_height =
            (self.size[1] - top_height - handle_height - if self.phone { border } else { 0. })
                .max(0.);
        let icon = |name: &str| c::Element::Icon {
            name: name.into(),
            size: ui::Length::Px(ctx.density.tokens().icon as f32),
            paint: ui::Paint::solid(ui::Color::Muted),
        };
        let mut label = match text(
            "bboxy450-term",
            STYLE.r#type.section as f32,
            400,
            false,
            ui::Color::Muted,
        ) {
            c::Element::Text(t) => t,
            _ => unreachable!(),
        };
        label.runs[0].style.tracking_em = 0.08;
        let right = named(
            "top/span1",
            flow(
                plain(
                    Some(
                        ((STYLE.r#type.section as f32 * 1.4 * 64.).floor() / 64.)
                            .max(ctx.density.tokens().icon as f32),
                    ),
                    [0.; 4],
                    ui::Color::Transparent,
                    0.,
                ),
                Axis::Row,
                0.,
                c::Alignment::Center,
                vec![named("top/span1/hicon", icon("gear")).into()],
            ),
        );
        let top = named(
            "top",
            flow(
                plain(
                    Some(top_height),
                    [0., STYLE.spacing.s5 as f32, 0., STYLE.spacing.s5 as f32],
                    ui::Color::Transparent,
                    0.,
                ),
                Axis::Row,
                STYLE.spacing.s4 as f32,
                c::Alignment::Center,
                vec![
                    named("top/hicon", icon("sidebarfox")).into(),
                    named("top/span", c::Element::Text(label)).into(),
                    c::Element::Spacer.into(),
                    right.into(),
                ],
            ),
        );
        let mut content = vec![];
        for (i, card) in self.cards.iter().enumerate() {
            let mut e = card.element(ctx);
            c::namespace(&mut e, &format!("swarm{i}/"));
            content.push(e.into());
            content.push(
                c::Element::Box(c::Box::new(
                    plain(
                        Some(bbt_ui_model::TOKENS.layout.cohort_gap as f32),
                        [0.; 4],
                        ui::Color::Transparent,
                        0.,
                    ),
                    vec![],
                ))
                .into(),
            );
        }
        if self.new_swarm {
            let plus = named(
                "new-swarm/plus",
                text("+", 18., 500, false, ui::Color::Muted),
            );
            let style = ui::Style::new(ui::Preset::NewSwarm);
            let body = flow(
                style,
                Axis::Row,
                STYLE.spacing.s4 as f32,
                c::Alignment::Center,
                vec![
                    c::Element::Spacer.into(),
                    plus.into(),
                    named(
                        "new-swarm/text",
                        text(
                            "new swarm",
                            STYLE.r#type.row as f32,
                            500,
                            false,
                            ui::Color::Muted,
                        ),
                    )
                    .into(),
                    c::Element::Spacer.into(),
                ],
            );
            content.push(named("new-swarm", body).into());
        }
        let content = declare_fixed(
            flow(
                plain(None, [0.; 4], ui::Color::Transparent, 0.),
                Axis::Column,
                0.,
                c::Alignment::Stretch,
                content,
            ),
            ctx,
            &Default::default(),
        );
        let mut scroll = c::ScrollView {
            viewport: [
                (width - 2. * STYLE.spacing.s3 as f32).max(0.),
                (scroll_height - STYLE.spacing.s4 as f32).max(0.),
            ],
            offset: [0., self.scroll],
            content_width: None,
            content: Box::new(content),
        };
        if let Some(anchor) = anchor {
            scroll
                .restore_anchor(anchor, ctx)
                .expect("fixed sidebar anchor");
        }
        let scroll = c::Element::ScrollView(scroll);
        let scroll = named(
            "scroll",
            flow(
                plain(
                    Some(scroll_height),
                    [
                        STYLE.spacing.s4 as f32,
                        STYLE.spacing.s3 as f32,
                        0.,
                        STYLE.spacing.s3 as f32,
                    ],
                    ui::Color::Transparent,
                    0.,
                ),
                Axis::Column,
                0.,
                c::Alignment::Stretch,
                vec![scroll.into()],
            ),
        );
        let mut children = vec![];
        if self.phone {
            let mut grab = plain(Some(4.), [0.; 4], ui::Color::Line, STYLE.radius.mark as f32);
            grab.overrides.width = Some(ui::Length::Px(40.));
            let handle = flow(
                plain(Some(44.), [0.; 4], ui::Color::Transparent, 0.),
                Axis::Row,
                STYLE.spacing.s5 as f32,
                c::Alignment::Center,
                vec![
                    c::Element::Spacer.into(),
                    named("handle/i", c::Element::Box(c::Box::new(grab, vec![]))).into(),
                    named(
                        "handle/span",
                        text(
                            "drag or tap",
                            STYLE.r#type.caption as f32,
                            400,
                            false,
                            ui::Color::Muted,
                        ),
                    )
                    .into(),
                    c::Element::Spacer.into(),
                ],
            );
            children.push(named("handle", handle).into());
        }
        children.extend([top.into(), scroll.into()]);
        let mut style = plain(
            Some(self.size[1]),
            [0., 0., 0., edge],
            ui::Color::Rgba(
                bbt_ui_model::draw::Color::hex(ctx.theme.palette().bg)
                    .mix(
                        bbt_ui_model::draw::Color::hex(ctx.theme.palette().sidebar),
                        STYLE.material.panel_alpha as f32,
                    )
                    .0,
            ),
            0.,
        );
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(border),
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: if self.phone {
                    ui::Alpha::Value(1.)
                } else {
                    ui::Alpha::Hairline
                },
            },
            outside: false,
        });
        style.overrides.border_sides = Some(if self.phone {
            [true, false, false, false]
        } else {
            [false, true, false, false]
        });
        if self.phone {
            style.overrides.radii = Some([20., 20., 0., 0.].map(ui::Length::Px));
        }
        named(
            "sidebar",
            flow(style, Axis::Column, 0., c::Alignment::Stretch, children),
        )
    }
}
