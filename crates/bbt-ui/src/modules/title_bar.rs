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
use bbt_ui_model::{
    STYLE,
    draw::{LineCap, PathCommand, PathStroke},
    layout::Axis,
};

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

/// Tachyon's bands, half-open: [0,50) green · [50,70) yellow · [70,90) orange ·
/// [90,100] red, judged on `band_percent` (lifted one band when on pace to run out).
pub fn band_color(metric: &UsageMetric) -> Option<ui::Color> {
    let hex = match metric.band_percent()? {
        p if p < 50 => "#30D158",
        p if p < 70 => "#FFD60A",
        p if p < 90 => "#FF9F0A",
        _ => "#FF453A",
    };
    Some(ui::Color::Rgba(bbt_ui_model::draw::Color::hex(hex).0))
}

fn usage_metric(metric: &UsageMetric, provider: bool) -> c::Element {
    let color = match metric.percent {
        _ if provider => ui::Color::Foreground,
        Some(used) if used >= 95 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.errored).0,
        ),
        Some(used) if used >= 80 => ui::Color::Rgba(
            bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.waiting).0,
        ),
        _ => ui::Color::Muted,
    };
    // Providers show a Tachyon ring with the harness icon; the number sits beside it.
    let value = if provider {
        metric.percent.map_or("—".into(), |p| format!("{p}%"))
    } else {
        metric.text(false)
    };
    let mut text = c::Text::new(
        value,
        ui::Text {
            paint: ui::Paint::solid(color),
            step: ui::TypeStep::Caption,
            weight: if provider { 600 } else { 400 },
            line_height: 1.4,
            ..Default::default()
        },
    );
    text.wrap = false;
    let text = c::Element::Text(text);
    let content = if provider {
        c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(4.),
            align: c::Alignment::Center,
            children: vec![
                named(format!("bar/usage/{}/ring", metric.label), ring(metric, 16.)).into(),
                text.into(),
            ],
        })
    } else {
        text
    };
    // Description is retained alongside the metric's geometry for hover/accessibility consumers.
    named(
        format!("bar/usage/{}", metric.label),
        named(
            format!("usage-label:{}", metric.description(provider)),
            content,
        ),
    )
}

/// Clockwise circular arc from 12 o'clock as cubic segments (at most 90° each).
fn arc(center: f32, radius: f32, fraction: f32) -> Vec<PathCommand> {
    let total = fraction.clamp(0., 1.) * std::f32::consts::TAU;
    let segments = (total / std::f32::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let step = total / segments as f32;
    let k = 4. / 3. * (step / 4.).tan() * radius;
    let at = |a: f32| [center + radius * a.sin(), center - radius * a.cos()];
    let mut out = vec![PathCommand::Move(at(0.))];
    for i in 0..segments {
        let (a0, a1) = (i as f32 * step, (i + 1) as f32 * step);
        let (p0, p3) = (at(a0), at(a1));
        out.push(PathCommand::Cubic([
            [p0[0] + k * a0.cos(), p0[1] + k * a0.sin()],
            [p3[0] - k * a1.cos(), p3[1] - k * a1.sin()],
            p3,
        ]));
    }
    out
}

fn stroked(size: [f32; 2], points: Vec<PathCommand>, width: f32, paint: ui::Paint, opacity: f32) -> c::Element {
    c::Element::Path(c::Path {
        size,
        points: points.into(),
        stroke: PathStroke {
            width,
            cap: LineCap::Round,
            ..Default::default()
        },
        paint,
        fill: None,
        opacity,
    })
}

/// Tachyon ring: a faint track, a band-colored arc for percent used, the harness
/// icon in the middle (a letter when the harness has no baked icon).
pub fn ring(metric: &UsageMetric, size: f32) -> c::Element {
    let width = (size * 0.14).max(1.5);
    let (center, radius) = (size / 2., (size - width) / 2.);
    let track = stroked([size; 2], arc(center, radius, 1.), width, ui::Paint::solid(ui::Color::Muted), 0.3);
    let mut overlays = Vec::new();
    if let (Some(p), Some(color)) = (metric.percent.filter(|p| *p > 0), band_color(metric)) {
        overlays.push(c::Placed {
            content: stroked([size; 2], arc(center, radius, p as f32 / 100.), width, ui::Paint::solid(color), 1.),
            left: 0.,
            bottom: 0.,
        });
    }
    let glyph = size * 0.5;
    let icon = match metric.label.as_str() {
        "claude" | "codex" => c::Element::Icon {
            name: metric.label.clone(),
            paint: ui::Paint::solid(ui::Color::Foreground),
            size: ui::Length::Px(glyph),
        },
        other => {
            let mut letter = c::Text::new(
                other.get(..1).unwrap_or("?").to_ascii_uppercase(),
                ui::Text {
                    weight: 700,
                    step: ui::TypeStep::Caption,
                    paint: ui::Paint::solid(ui::Color::Foreground),
                    ..Default::default()
                },
            );
            letter.wrap = false;
            c::Element::Text(letter)
        }
    };
    overlays.push(c::Placed {
        content: icon,
        left: (size - glyph) / 2.,
        bottom: (size - glyph) / 2.,
    });
    c::Element::Layers {
        base: std::boxed::Box::new(track),
        overlays,
    }
}

/// Tachyon progress bar: rounded track with a band-colored fill.
pub fn progress_bar(metric: &UsageMetric, width: f32) -> c::Element {
    let height = 6.;
    let line = |to: f32| {
        vec![
            PathCommand::Move([height / 2., height / 2.]),
            PathCommand::Line([to.max(height / 2. + 0.01), height / 2.]),
        ]
    };
    let size = [width, height];
    let track = stroked(size, line(width - height / 2.), height, ui::Paint::solid(ui::Color::Muted), 0.25);
    let mut overlays = Vec::new();
    if let (Some(p), Some(color)) = (metric.percent.filter(|p| *p > 0), band_color(metric)) {
        let to = height / 2. + (width - height) * p.min(100) as f32 / 100.;
        overlays.push(c::Placed {
            content: stroked(size, line(to), height, ui::Paint::solid(color), 1.),
            left: 0.,
            bottom: 0.,
        });
    }
    c::Element::Layers {
        base: std::boxed::Box::new(track),
        overlays,
    }
}

fn caption(value: impl Into<String>, weight: u16, color: ui::Color, step: ui::TypeStep) -> c::Element {
    let mut text = c::Text::new(
        value.into(),
        ui::Text {
            weight,
            step,
            paint: ui::Paint::solid(color),
            line_height: 1.35,
            ..Default::default()
        },
    );
    text.wrap = false;
    c::Element::Text(text)
}

fn spread(left: c::Element, right: Option<c::Element>) -> c::Element {
    let mut children = vec![left.into()];
    children.push(c::Item {
        element: c::Element::Spacer,
        extent: c::Extent::Fill,
    });
    children.extend(right.map(Into::into));
    c::Element::Stack(c::Stack {
        direction: Axis::Row,
        gap: ui::Length::Px(8.),
        align: c::Alignment::Center,
        children,
    })
}

fn column(gap: f32, children: Vec<c::Item>) -> c::Element {
    c::Element::Stack(c::Stack {
        direction: Axis::Column,
        gap: ui::Length::Px(gap),
        align: c::Alignment::Start,
        children,
    })
}

/// The hover breakdown, laid out like Tachyon's popover: per account a header,
/// then each window with its reset, a progress bar, percent used and pace.
pub fn usage_breakdown(usage: &UsageStrip, width: f32) -> c::Element {
    let muted = ui::Color::Muted;
    let mut sections: Vec<c::Item> = Vec::new();
    if !usage.machine.is_empty() {
        let line = usage.machine.iter().map(|m| m.description(false)).collect::<Vec<_>>().join("  ·  ");
        sections.push(caption(line, 500, muted, ui::TypeStep::Caption).into());
    }
    for provider in &usage.providers {
        let mut name = provider.label.clone();
        if let Some(first) = name.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        let header = c::Element::Stack(c::Stack {
            direction: Axis::Row,
            gap: ui::Length::Px(8.),
            align: c::Alignment::Center,
            children: vec![
                ring(provider, 20.).into(),
                caption(name, 700, ui::Color::Foreground, ui::TypeStep::Body).into(),
            ],
        });
        let mut rows: Vec<c::Item> = vec![header.into()];
        let windows: Vec<(String, Option<String>, UsageMetric)> = if provider.windows.is_empty() {
            vec![("Usage".into(), None, provider.clone())]
        } else {
            provider
                .windows
                .iter()
                .map(|w| (w.label.clone(), w.resets.clone(), w.metric()))
                .collect()
        };
        for (title, resets, window) in windows {
            let resets = resets.map(|r| caption(r, 400, muted, ui::TypeStep::Caption));
            let used = window
                .percent
                .map_or("—".to_string(), |p| format!("{p}% Used"));
            let pace = window.pace_caption().map(|mut c| {
                if let Some(first) = c.get_mut(..1) {
                    first.make_ascii_uppercase();
                }
                caption(c, 400, muted, ui::TypeStep::Caption)
            });
            rows.push(
                column(
                    4.,
                    vec![
                        spread(caption(title, 500, ui::Color::Foreground, ui::TypeStep::Caption), resets).into(),
                        progress_bar(&window, width).into(),
                        spread(caption(used, 400, muted, ui::TypeStep::Caption), pace).into(),
                    ],
                )
                .into(),
            );
        }
        // Routine refresh states are not news; a historical or failed reading is.
        if let Some(reason) = provider.reason.as_ref().filter(|r| !r.starts_with("Refresh")) {
            rows.push(caption(reason.clone(), 400, muted, ui::TypeStep::Caption).into());
        }
        sections.push(column(8., rows).into());
    }
    sections.push(caption("Click to refresh accounts", 400, muted, ui::TypeStep::Caption).into());
    named("usage/breakdown", column(14., sections))
}
