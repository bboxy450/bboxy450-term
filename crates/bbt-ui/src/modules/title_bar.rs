// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Window chrome composed from retained L6 elements. The app owns all actions.
use super::{seat_row::named, strip::plain};
use crate::{self as ui, components as c};
use bbt_ui_model::usage::{UsageMetric, UsageStrip};
use bbt_ui_model::{STYLE, layout::Axis};

#[derive(Debug, Clone)]
pub struct TitleBar {
    pub title: String,
    pub crumb: String,
    pub sidebar_open: bool,
    pub blurred: bool,
}
impl Default for TitleBar {
    fn default() -> Self {
        Self {
            title: "bboxy450-term".into(),
            crumb: String::new(),
            sidebar_open: true,
            blurred: false,
        }
    }
}
fn flexible(element: c::Element) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Content {
            grow: 0.,
            shrink: 1.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn label(key: &str, value: &str, weight: u16, muted: bool, wrap: bool) -> c::Element {
    let mut text = c::Text::new(
        value,
        ui::Text {
            weight,
            paint: ui::Paint::solid(if muted {
                ui::Color::Muted
            } else {
                ui::Color::Foreground
            }),
            // .titlebar.app-bar inherits the lab's 1.4 line height.
            line_height: 1.4,
            ..Default::default()
        },
    );
    text.wrap = wrap;
    text.ellipsis = !wrap;
    named(key, c::Element::Text(text))
}
fn toggle(key: &str, icon: &str, selected: bool) -> c::Element {
    let mut style = plain(
        None,
        [STYLE.spacing.s2 as f32; 4],
        STYLE.radius.badge as f32,
    );
    if selected {
        style.overrides.fill = Some(ui::Paint {
            color: ui::Color::Foreground,
            alpha: ui::Alpha::Selection,
        });
    }
    named(
        key,
        c::Element::Box(c::Box::new(
            style,
            vec![
                named(
                    format!("{key}/hicon"),
                    c::Element::Icon {
                        name: icon.into(),
                        paint: ui::Paint::solid(if selected {
                            ui::Color::Foreground
                        } else {
                            ui::Color::Muted
                        }),
                        // .tbtn --icon in lab.css; fixed independently of density.
                        size: ui::Length::Px(18.),
                    },
                )
                .into(),
            ],
        )),
    )
}
impl TitleBar {
    /// Snapshot-only rendering. The app uses accessible_label for the hover description.
    pub fn usage_element(&self, usage: &UsageStrip) -> c::Element {
        let mut children = Vec::new();
        for (provider, metrics) in [(false, &usage.machine), (true, &usage.providers)] {
            // Edge spaces in a label are trimmed, so spacing comes from the stack gap.
            if !children.is_empty() && !metrics.is_empty() {
                children.push(label("bar/usage/divider", "│", 400, true, false).into());
            }
            for (index, metric) in metrics.iter().enumerate() {
                if index != 0 {
                    children.push(label("bar/usage/dot", "·", 400, true, false).into());
                }
                children.push(usage_metric(metric, provider).into());
            }
        }
        let stack = c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(6.),
            align: c::Alignment::Center,
            children,
        });
        let mut container = c::Box::new(plain(None, [0.; 4], 0.), vec![stack.into()]);
        container.clip = true;
        named("bar/usage", c::Element::Box(container))
    }

    pub fn element(&self, ctx: ui::Context) -> c::Element {
        self.element_with_usage(ctx, &UsageStrip::default())
    }

    pub fn element_with_usage(&self, ctx: ui::Context, usage: &UsageStrip) -> c::Element {
        let dots = (0..3)
            .map(|i| {
                // .win i: 10px circles, with 6px gaps; no OS-native chrome.
                let mut dot = plain(Some(10.), [0.; 4], 5.);
                dot.overrides.width = Some(ui::Length::Px(10.));
                dot.overrides.fill = Some(ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Value(if self.blurred { 0.6 } else { 1. }),
                });
                named(
                    if i == 0 {
                        "bar/win/i".into()
                    } else {
                        format!("bar/win/i{i}")
                    },
                    c::Element::Box(c::Box::new(dot, vec![])),
                )
                .into()
            })
            .collect();
        let mut logo = c::Element::Image {
            asset: "bboxy".into(),
            size: ui::Length::Px(18.),
            radius: ui::Length::Px(9.),
        };
        if self.blurred {
            logo = c::Element::Filter {
                content: Box::new(logo),
                saturation: 0.4,
                opacity: 0.8,
            };
        }
        // Reuse the frosted panel's Style (including its inset highlight),
        // replacing its geometry with the app-bar's tokenised dimensions.
        let mut style = ui::Style::new(ui::Preset::Modal);
        style.overrides.height = Some(ui::Length::Px(STYLE.spacing.title_bar as f32));
        style.overrides.padding = Some([
            ui::Length::Px(0.),
            ui::Length::Spacing(7),
            ui::Length::Px(0.),
            ui::Length::Spacing(7),
        ]);
        style.overrides.radii = Some([ui::Length::Px(0.); 4]);
        style.overrides.shadow = Some(ui::Shadow::None);
        style.overrides.border_sides = Some([false, false, true, false]);
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        let mut bar = c::Box::new(
            style,
            vec![
                named(
                    "bar/win",
                    c::Element::Stack(c::Stack {
                        direction: Axis::Row,
                        gap: ui::Length::Spacing(3),
                        align: c::Alignment::Center,
                        children: dots,
                    }),
                )
                .into(),
                toggle(
                    "bar/tbtn",
                    if self.sidebar_open {
                        "sidebarfoxopen"
                    } else {
                        "sidebarfox"
                    },
                    self.sidebar_open,
                )
                .into(),
                named("bar/logo", logo).into(),
                flexible(label(
                    "bar/ttl",
                    &self.title,
                    STYLE.r#type.weight_title as u16,
                    false,
                    true,
                )),
                flexible(label("bar/crumb", &self.crumb, 400, true, false)),
                c::Item {
                    element: named("bar/spacer", c::Element::Spacer),
                    extent: c::Extent::Fill,
                },
                toggle(
                    "bar/tbtn1",
                    if ctx.theme == ui::Theme::Dark {
                        "sun"
                    } else {
                        "moon"
                    },
                    false,
                )
                .into(),
            ],
        );
        if !usage.machine.is_empty() || !usage.providers.is_empty() {
            bar.children
                .insert(bar.children.len() - 1, flexible(self.usage_element(usage)));
        }
        bar.gap = ui::Length::Spacing(5);
        bar.cross_overflow = true;
        named("bar", c::Element::Box(bar))
    }
}

fn usage_metric(metric: &UsageMetric, provider: bool) -> c::Element {
    let color = match metric.percent {
        Some(left) if provider && left < 10 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.errored).0,
        ),
        Some(left) if provider && left <= 20 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.waiting).0,
        ),
        Some(used) if !provider && used >= 95 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.errored).0,
        ),
        Some(used) if !provider && used >= 80 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.waiting).0,
        ),
        _ => ui::Color::Muted,
    };
    let mut text = c::Text::new(
        metric.text(provider),
        ui::Text {
            paint: ui::Paint::solid(color),
            step: ui::TypeStep::Caption,
            line_height: 1.4,
            ..Default::default()
        },
    );
    text.wrap = false;
    // Description is retained alongside the metric's geometry for hover/accessibility consumers.
    named(
        format!("bar/usage/{}", metric.label),
        named(
            format!("usage-label:{}", metric.description(provider)),
            c::Element::Text(text),
        ),
    )
}
