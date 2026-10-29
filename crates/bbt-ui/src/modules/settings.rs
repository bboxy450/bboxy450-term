// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-mostly Settings composition. Host-owned records and section selection;
//! this view never reads config files, unlocks keys, or issues grants.
pub mod controller;
mod hosted;
pub mod knowledge;
pub mod messaging;
pub mod plugins;
use super::link::{Cloud, MachineState};
use super::{
    seat_row::{named, role_color},
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::Color as Rgba,
    layout::{Axis, TextStyle as MeasureStyle},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Account {
    pub name: String,
    pub handle: String,
    pub email: String,
    pub fingerprint: String,
    pub algo: String,
    #[serde(rename = "keyState")]
    pub key_state: String,
    pub trust: Vec<String>,
    pub devices: Vec<(String, String, String)>,
    pub sign: Signatures,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
}
/// Host-supplied public custody status; activating the button emits an intent only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Custody {
    pub tier: String,
    pub acl: String,
    pub hold: Option<CustodyHold>,
    pub attempt: Option<String>,
    pub op: Option<String>,
    #[serde(skip)]
    pub enabled: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustodyHold {
    pub reason: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Signatures {
    pub commits: bool,
    pub messages: bool,
    #[serde(rename = "seatCommits")]
    pub seat_commits: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Role {
    pub prompt: String,
    pub send: String,
    pub color: String,
    pub mark: String,
    pub cls: String,
    pub custom: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Harness {
    pub role: String,
    pub available_roles: Vec<String>,
    pub eligible_roles: Vec<String>,
    pub wake: String,
    pub hooks: bool,
    pub command: String,
    pub tile: String,
    pub icon: String,
    #[serde(rename = "stateFrom")]
    pub state_from: String,
    pub custom: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: String,
    pub density: String,
    /// Logical terminal points only; chrome typography and window scale stay independent.
    pub terminal_font_size: f32,
    /// Rim size in px (glow.rim_min..=glow.rim_max): the alive rim centred on each pane's
    /// border, half outside and half inside; the gutter follows it, so it relayouts (rim-glow §3).
    pub rim: f32,
    pub glow: bool,
    pub subtitle: bool,
    pub opacity: f32,
    pub blur: bool,
    #[serde(rename = "blurRadius")]
    pub blur_radius: f32,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: String::new(),
            density: String::new(),
            terminal_font_size: 13.,
            rim: 0.,
            glow: false,
            subtitle: false,
            opacity: 0.,
            blur: false,
            blur_radius: 0.,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub enabled: bool,
    pub only_when_not_frontmost: bool,
    pub disabled_harnesses: std::collections::BTreeSet<String>,
}
impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            only_when_not_frontmost: false,
            disabled_harnesses: Default::default(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Records {
    pub account: Account,
    pub cloud: Vec<Cloud>,
    pub machines: Vec<(String, String, String, String, MachineState)>,
    /// Host-supplied stable IDs for observed machine/{index} and cloud/{index} rows.
    pub destination_ids: std::collections::BTreeMap<String, String>,
    pub disabled_destinations: std::collections::BTreeSet<String>,
    pub roles: Vec<(String, Role)>,
    pub harness: Vec<(String, Harness)>,
    pub plugins: Vec<plugins::Plugin>,
    pub plugin_notice: Option<String>,
    /// The host has a live registry; gallery/reference records never authorize effects.
    #[serde(skip)]
    pub plugin_controls: bool,
    #[serde(skip)]
    pub knowledge_controls: bool,
    pub knowledge: knowledge::Knowledge,
    pub messaging: messaging::Messaging,
    pub appearance: Appearance,
    pub notifications: Notifications,
    pub shortcuts: Vec<(String, String)>,
}
impl Records {
    pub(super) fn destination_id(&self, key: &str) -> Option<&str> {
        let (kind, index) = key.split_once('/')?;
        let index = index.parse::<usize>().ok()?;
        if key != format!("{kind}/{index}") {
            return None;
        }
        let id = self.destination_ids.get(key)?;
        let observed = match kind {
            "machine" => index < self.machines.len() && id.starts_with("ssh:"),
            "cloud" => index < self.cloud.len() && id.starts_with("cloud:"),
            _ => false,
        };
        (observed && valid_destination_id(id)).then_some(id.as_str())
    }
}
fn valid_destination_id(id: &str) -> bool {
    id.strip_prefix("ssh:")
        .or_else(|| id.strip_prefix("cloud:"))
        .is_some_and(|name| !name.is_empty() && name.trim() == name)
        && id.len() <= 512
        && !id.chars().any(char::is_control)
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DraftHarness {
    pub name: String,
    pub command: String,
    pub ci: i32,
    pub icon: String,
    pub role: String,
    pub wake: String,
    #[serde(rename = "stateFrom")]
    pub state_from: String,
    pub hooks: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Launch {
    pub harness: String,
    pub label: String,
    pub tile: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub section: String,
    pub values: Records,
    pub defaults: Records,
    #[serde(rename = "draftHarness")]
    pub draft_harness: Option<DraftHarness>,
    pub launch: Vec<Launch>,
}
pub const SECTIONS: [(&str, &str); 11] = [
    ("Account", "key"),
    ("Cloud", "cloud"),
    ("Machines", "ssh"),
    ("Roles", "roles"),
    ("Harnesses", "gear"),
    ("Plugins", "plug"),
    ("Knowledge", "book"),
    ("Appearance", "sun"),
    ("Notifications", "bell"),
    ("Messaging", "tray"),
    ("Shortcuts", "keys"),
];
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn color(v: &str) -> ui::Color {
    if let Some(r) = v
        .strip_prefix("var(--role-")
        .and_then(|s| s.strip_suffix(')'))
    {
        role_color(r)
    } else {
        ui::Color::Rgba(Rgba::hex(v).0)
    }
}
fn font(size: f32, weight: u16, mono: bool, co: ui::Color) -> ui::Text {
    ui::Text {
        size: Some(size),
        weight,
        mono,
        paint: ui::Paint::solid(co),
        line_height: 1.35,
        ..Default::default()
    }
}
fn txt(k: &str, v: impl Into<String>, f: ui::Text) -> c::Element {
    let mut t = c::Text::new(v, f);
    t.wrap = false;
    t.ellipsis = true;
    named(k, c::Element::Text(t))
}
fn wrap(k: &str, v: impl Into<String>, f: ui::Text) -> c::Element {
    named(k, c::Element::Text(c::Text::new(v, f)))
}
fn row(k: &str, s: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(s, children);
    b.gap = ui::Length::Px(gap);
    named(k, c::Element::Box(b))
}
fn col(k: &str, s: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(s, children);
    b.direction = Axis::Column;
    b.align = c::Alignment::Stretch;
    b.gap = ui::Length::Px(gap);
    named(k, c::Element::Box(b))
}
fn fixed(e: c::Element, n: f32) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fixed(n),
    }
}
fn fill(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fill,
    }
}
fn natural(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Content {
            grow: 0.,
            shrink: 0.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn icon(k: &str, name: &str, size: f32, co: ui::Color) -> c::Element {
    named(
        k,
        c::Element::Icon {
            name: name.into(),
            paint: ui::Paint::solid(co),
            size: ui::Length::Px(size),
        },
    )
}
fn surface(
    fill: ui::Color,
    alpha: f32,
    radius: f32,
    pad: [f32; 4],
    height: Option<f32>,
) -> ui::Style {
    let mut s = plain(height, pad, radius);
    s.overrides.fill = Some(paint(fill, alpha));
    s
}
fn hair(ctx: ui::Context) -> ui::Color {
    let mut c = Rgba::hex(ctx.theme.palette().line);
    c.0[3] *= STYLE.material.hairline_alpha as f32;
    ui::Color::Rgba(c.0)
}
fn border(mut s: ui::Style, co: ui::Color, outside: bool) -> ui::Style {
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint::solid(co),
        outside,
    });
    s
}
fn bottom(s: ui::Style, ctx: ui::Context, last: bool) -> ui::Style {
    if last {
        s
    } else {
        let mut s = border(s, hair(ctx), false);
        s.overrides.border_sides = Some([false, false, true, false]);
        s
    }
}
fn tint(ctx: ui::Context, alpha: f32) -> ui::Color {
    let mut v = Rgba::hex(ctx.theme.palette().fg);
    v.0[3] *= alpha;
    ui::Color::Rgba(v.0)
}
fn badge(ctx: ui::Context, alpha: f32) -> ui::Color {
    let mut v = Rgba::hex(ctx.theme.palette().fg_dim);
    v.0[3] *= alpha;
    ui::Color::Rgba(v.0)
}
fn mixed(ctx: ui::Context, a: &str, b: &str, f: f32) -> ui::Color {
    let _ = ctx;
    ui::Color::Rgba(Rgba::hex(a).mix(Rgba::hex(b), 1. - f).0)
}
fn measured(m: &dyn c::Measure, v: &str, f: ui::Text) -> f32 {
    m.width(
        v,
        &MeasureStyle {
            size: f.size.unwrap_or(13.),
            weight: f.weight,
            mono: f.mono,
            italic: f.italic,
            tracking_em: f.tracking_em,
        },
    )
}
fn flow(
    k: &str,
    items: Vec<c::Item>,
    row_gap: f32,
    col_gap: f32,
    align: c::Alignment,
) -> c::Element {
    let mut f = c::Flow::new(items);
    f.row_gap = ui::Length::Px(row_gap);
    f.column_gap = ui::Length::Px(col_gap);
    f.align = align;
    named(k, c::Element::Flow(f))
}
struct Build<'a> {
    ctx: ui::Context,
    m: &'a dyn c::Measure,
    width: f32,
}
impl Build<'_> {
    fn text(
        &self,
        k: &str,
        v: impl Into<String>,
        size: f32,
        weight: u16,
        mono: bool,
        dim: bool,
    ) -> c::Element {
        txt(
            k,
            v,
            font(
                size,
                weight,
                mono,
                if dim {
                    ui::Color::Muted
                } else {
                    ui::Color::Foreground
                },
            ),
        )
    }
    fn note(&self, k: &str, v: &str, inside: bool) -> c::Element {
        col(
            k,
            plain(
                None,
                if inside {
                    [8., 0., 0., 0.]
                } else {
                    [0., 2., 0., 2.]
                },
                0.,
            ),
            0.,
            vec![
                wrap(
                    &format!("{k}/text"),
                    v,
                    ui::Text {
                        line_height: 1.5,
                        ..font(12., 400, false, ui::Color::Muted)
                    },
                )
                .into(),
            ],
        )
    }
    fn card(&self, k: &str, title: &str, children: Vec<c::Item>, pad: [f32; 4]) -> c::Element {
        let mut all = vec![];
        if !title.is_empty() {
            all.push(
                row(
                    &format!("{k}/head"),
                    plain(None, [8., 0., 4., 0.], 0.),
                    10.,
                    vec![
                        self.text(&format!("{k}/title"), title, 13., 600, false, false)
                            .into(),
                    ],
                )
                .into(),
            );
        }
        all.extend(children);
        col(
            k,
            border(
                surface(
                    ui::Color::Surface,
                    STYLE.material.card_alpha as f32,
                    STYLE.radius.card as f32,
                    pad,
                    None,
                ),
                hair(self.ctx),
                true,
            ),
            0.,
            all,
        )
    }
    fn dot(&self, k: &str, on: bool, size: f32) -> c::Element {
        let co = if on {
            color(TOKENS.color.ui.ok)
        } else {
            color(TOKENS.color.state.idle)
        };
        let mut s = surface(co, 1., size / 2., [0.; 4], Some(size));
        s.overrides.width = Some(ui::Length::Px(size));
        if on {
            s.overrides.shadow = Some(ui::Shadow::Positioned {
                offset: [0., 0.],
                blur: 6.,
                color: ui::Paint {
                    color: co,
                    alpha: ui::Alpha::Value(0.6),
                }
                .resolve(self.ctx)
                .unwrap(),
            });
        }
        row(k, s, 0., vec![])
    }
    fn toggle(&self, k: &str, on: bool) -> c::Element {
        let mut knob = surface(
            if on {
                ui::Color::Rgba([1.; 4])
            } else {
                ui::Color::Surface
            },
            1.,
            7.,
            [0.; 4],
            Some(14.),
        );
        knob.overrides.width = Some(ui::Length::Px(14.));
        knob.overrides.shadow = Some(ui::Shadow::Positioned {
            offset: [0., 1.],
            blur: 2.,
            color: [0., 0., 0., 0.3],
        });
        row(
            k,
            surface(
                if on {
                    ui::Color::Accent
                } else {
                    badge(self.ctx, STYLE.tint.badge_selected_alpha as f32)
                },
                1.,
                9.,
                [2., if on { 2. } else { 14. }, 2., if on { 14. } else { 2. }],
                Some(18.),
            ),
            0.,
            vec![fixed(row(&format!("{k}/knob"), knob, 0., vec![]), 14.)],
        )
    }
    fn chip(
        &self,
        k: &str,
        v: &str,
        builtin: bool,
        co: Option<ui::Color>,
        ico: Option<&str>,
    ) -> c::Element {
        let fg = co.unwrap_or(if builtin {
            ui::Color::Muted
        } else {
            ui::Color::Foreground
        });
        let mut items = vec![];
        if let Some(i) = ico {
            items.push(icon(&format!("{k}/icon"), i, 12., ui::Color::Muted).into());
        }
        let mut f = font(
            if builtin { 10. } else { 11. },
            if co.is_some() { 600 } else { 400 },
            false,
            fg,
        );
        if builtin {
            f.tracking_em = 0.04;
        }
        items.push(txt(&format!("{k}/text"), v, f).into());
        row(
            k,
            surface(
                co.unwrap_or(badge(self.ctx, STYLE.tint.badge_alpha as f32)),
                if co.is_some() { 0.14 } else { 1. },
                6.,
                [
                    0.,
                    if builtin { 6. } else { 8. },
                    0.,
                    if builtin { 6. } else { 8. },
                ],
                Some(if builtin { 18. } else { 20. }),
            ),
            6.,
            items,
        )
    }
    fn diff(&self, k: &str) -> c::Element {
        let s = border(plain(None, [1., 8., 1., 8.], 999.), hair(self.ctx), false);
        let mut t = font(11., 400, false, ui::Color::Muted);
        t.line_height = 16. / 11.;
        row(
            k,
            plain(None, [0.; 4], 0.),
            6.,
            vec![
                fixed(
                    row(
                        "",
                        surface(ui::Color::Accent, 1., 3.5, [0.; 4], Some(7.)),
                        0.,
                        vec![],
                    ),
                    7.,
                ),
                row("", s, 0., vec![txt("", "reset", t).into()]).into(),
            ],
        )
    }
    fn select(&self, k: &str, opts: &[String], v: &str, small: bool, full: bool) -> c::Element {
        let size = if small { 11. } else { 12.5 };
        let f = font(size, 400, false, ui::Color::Foreground);
        let pad_left = if small { 8. } else { 9. };
        let pad_right = if small { 22. } else { STYLE.spacing.s10 as f32 };
        let w = opts
            .iter()
            .map(|s| measured(self.m, s, f))
            .fold(0., f32::max)
            .ceil()
            + pad_left
            + pad_right;
        let mut style = border(
            surface(
                mixed(
                    self.ctx,
                    self.ctx.theme.palette().bg,
                    self.ctx.theme.palette().surface,
                    0.6,
                ),
                1.,
                6.,
                [
                    if small { 3. } else { 5. },
                    pad_right,
                    if small { 3. } else { 5. },
                    if small { 8. } else { 9. },
                ],
                None,
            ),
            hair(self.ctx),
            true,
        );
        if !full {
            style.overrides.width = Some(ui::Length::Px(if small { w.min(128.) } else { w }));
        }
        // The lab's explicit 12px chevdown tile; no browser-native indicator.
        let control_width = if full {
            self.width
        } else {
            w.min(if small { 128. } else { w })
        };
        let line_height = (size * 1.35 * 64.).floor() / 64.;
        let control_height = line_height + if small { 6. } else { 10. };
        let arrow = c::Placed {
            content: icon(&format!("{k}/caret"), "chevdown", 12., ui::Color::Muted),
            left: control_width - STYLE.spacing.s4 as f32 - 12.,
            bottom: (control_height - 12.) * 0.5,
        };
        let mut b = c::Box::new(style, vec![fill(txt(&format!("{k}/text"), v, f))]);
        b.gap = ui::Length::Px(0.);
        named(
            k,
            c::Element::Layers {
                base: Box::new(c::Element::Box(b)),
                overlays: vec![arrow],
            },
        )
    }
    #[allow(clippy::too_many_arguments)] // A labelled control row and its supplied display flags.
    fn setting_row(
        &self,
        k: &str,
        label: &str,
        hint: Option<&str>,
        control: c::Element,
        changed: bool,
        last: bool,
        small: bool,
    ) -> c::Element {
        let mut labels = vec![
            wrap(
                &format!("{k}/label/text"),
                label,
                font(
                    if small { 12.5 } else { 13. },
                    500,
                    false,
                    ui::Color::Foreground,
                ),
            )
            .into(),
        ];
        if let Some(h) = hint {
            labels.push(
                wrap(
                    &format!("{k}/hint"),
                    h,
                    font(11., 400, false, ui::Color::Muted),
                )
                .into(),
            );
        }
        let lbl = col(&format!("{k}/label"), plain(None, [0.; 4], 0.), 1., labels);
        let mut s = bottom(
            plain(
                None,
                [
                    if small { 6. } else { 8. },
                    0.,
                    if small { 6. } else { 8. },
                    0.,
                ],
                0.,
            ),
            self.ctx,
            last,
        );
        s.overrides.height = None;
        let mut children = vec![fill(lbl), natural(named(format!("{k}/ctl"), control))];
        // Grid's empty third auto column still contributes its second gap.
        children.push(if changed {
            natural(self.diff(&format!("{k}/diff")))
        } else {
            fixed(c::Element::Spacer, 0.)
        });
        let mut e = row(k, s, if small { 10. } else { 14. }, children);
        let h = e
            .layout(
                bbt_ui_model::layout::Rect::new(
                    0.,
                    0.,
                    self.width + if changed && small { 16. } else { 0. },
                    0.,
                ),
                self.ctx,
                self.m,
            )
            .unwrap()
            .rect
            .h
            .max(if small { 34. } else { 40. });
        set_height(&mut e, h);
        if changed {
            expand_row(&mut e, k, 8., self.width, self.ctx, self.m, small);
        }
        e
    }
    fn field(
        &self,
        k: &str,
        label: &str,
        hint: Option<&str>,
        control: c::Element,
        changed: bool,
        small: bool,
    ) -> c::Element {
        let mut labels = vec![self.text("", label, 12.5, 500, false, false).into()];
        if let Some(h) = hint {
            labels.push(wrap("", h, font(11., 400, false, ui::Color::Muted)).into());
        }
        let mut line = vec![fill(col("", plain(None, [0.; 4], 0.), 0., labels))];
        if changed {
            line.push(self.diff("").into());
        }
        col(
            k,
            plain(
                None,
                [
                    if small { 6. } else { 7. },
                    0.,
                    if small { 6. } else { 7. },
                    0.,
                ],
                0.,
            ),
            5.,
            vec![
                row(&format!("{k}/label"), plain(None, [0.; 4], 0.), 8., line).into(),
                named(format!("{k}/control"), control).into(),
            ],
        )
    }
    fn segment(&self, k: &str, opts: &[&str], v: &str) -> c::Element {
        row(
            k,
            surface(
                badge(self.ctx, STYLE.tint.badge_alpha as f32),
                1.,
                8.,
                [2.; 4],
                None,
            ),
            2.,
            opts.iter()
                .enumerate()
                .map(|(i, o)| {
                    row(
                        &format!("{k}/{i}"),
                        if *o == v {
                            border(
                                surface(ui::Color::Surface, 1., 6., [2., 10., 2., 10.], None),
                                hair(self.ctx),
                                true,
                            )
                        } else {
                            plain(None, [2., 10., 2., 10.], 6.)
                        },
                        0.,
                        vec![self.text("", *o, 12.5, 400, false, *o != v).into()],
                    )
                    .into()
                })
                .collect(),
        )
    }
    fn button(&self, k: &str, v: &str, ico: Option<&str>, kind: &str) -> c::Element {
        self.button_sized(k, v, ico, kind, 13.)
    }
    fn button_sized(
        &self,
        k: &str,
        v: &str,
        ico: Option<&str>,
        kind: &str,
        size: f32,
    ) -> c::Element {
        let co = if kind == "primary" {
            ui::Color::OnAccent
        } else if kind == "link" {
            color(self.ctx.theme.palette().accent_soft)
        } else if kind == "ghost" {
            ui::Color::Muted
        } else {
            ui::Color::Foreground
        };
        let s = surface(
            if kind == "primary" {
                ui::Color::Accent
            } else if kind == "link" || kind == "ghost" {
                ui::Color::Transparent
            } else {
                tint(self.ctx, STYLE.tint.selection_alpha as f32)
            },
            1.,
            999.,
            [
                0.,
                if kind == "link" { 4. } else { 13. },
                0.,
                if kind == "link" { 4. } else { 13. },
            ],
            Some(28.),
        );
        let mut kids = vec![];
        if let Some(i) = ico {
            kids.push(icon("", i, 14., co).into());
        }
        kids.push(txt("", v, font(size, 400, false, co)).into());
        row(
            k,
            if kind == "ghost" {
                border(s, hair(self.ctx), true)
            } else {
                s
            },
            7.,
            kids,
        )
    }
    fn grid(&self, k: &str, width: f32, min: f32, gap: f32, items: Vec<c::Element>) -> c::Element {
        let cols = (((width + gap) / (min + gap)).floor() as usize).max(1);
        let cw = (width - gap * (cols - 1) as f32) / cols as f32;
        let rows = items
            .chunks(cols)
            .map(|chunk| {
                let height = chunk
                    .iter()
                    .map(|e| {
                        e.layout(
                            bbt_ui_model::layout::Rect::new(0., 0., cw, 0.),
                            self.ctx,
                            self.m,
                        )
                        .expect("settings grid child")
                        .rect
                        .h
                    })
                    .fold(0., f32::max);
                let children = chunk
                    .iter()
                    .map(|e| {
                        let mut child = e.clone();
                        set_height(&mut child, height);
                        fixed(child, cw)
                    })
                    .collect();
                row("", plain(None, [0.; 4], 0.), gap, children).into()
            })
            .collect();
        col(k, plain(None, [0.; 4], 0.), gap, rows)
    }
    fn role_card(&self, k: &str, name: &str, v: &Role, d: Option<&Role>) -> c::Element {
        let co = color(&v.color);
        let mut f = font(10.5, 600, false, co);
        f.tracking_em = 0.16;
        f.uppercase = true;
        let mut band = vec![
            txt("", &v.mark, font(14., 600, false, co)).into(),
            fill(txt(&format!("{k}/name"), name, f)),
            self.chip("", "built-in", true, Some(co), None).into(),
        ];
        if v.custom {
            band.pop();
            band.push(
                self.text(&format!("{k}/delete"), "delete", 10., 500, false, false)
                    .into(),
            );
        }
        let mut s = surface(co, 0.18, 0., [0., 12., 0., 12.], Some(30.));
        s = border(s, co, false);
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Px(2.),
            paint: ui::Paint::solid(co),
            outside: false,
        });
        s.overrides.border_sides = Some([false, false, true, false]);
        let prompt_text = wrap(
            "",
            &v.prompt,
            ui::Text {
                line_height: 1.45,
                ..font(11., 400, true, ui::Color::Foreground)
            },
        );
        let text_height = prompt_text
            .natural((self.width - 18.).max(0.), self.ctx, self.m)
            .expect("prompt geometry")
            .0[1];
        let prompt_height = (text_height + 10.).clamp(88., 240.);
        let prompt = col(
            "",
            border(
                surface(
                    mixed(
                        self.ctx,
                        self.ctx.theme.palette().bg,
                        self.ctx.theme.palette().surface,
                        0.6,
                    ),
                    1.,
                    6.,
                    [5., 9., 5., 9.],
                    Some(prompt_height),
                ),
                hair(self.ctx),
                true,
            ),
            0.,
            vec![
                c::Element::ScrollView(c::ScrollView {
                    viewport: [(self.width - 18.).max(0.), prompt_height - 10.],
                    offset: [0.; 2],
                    content_width: None,
                    content: Box::new(prompt_text),
                })
                .into(),
            ],
        );
        let fld0 = self.field(
            &format!("{k}/f0"),
            "Default system prompt",
            None,
            prompt,
            d.is_some_and(|d| d.prompt != v.prompt),
            false,
        );
        let fld1 = self.field(
            &format!("{k}/f1"),
            "May send to",
            Some("terminal-enforced"),
            self.select(
                "",
                &[
                    "all channels · all DMs".into(),
                    "own swarm · operator DM".into(),
                    "operator DM only".into(),
                ],
                &v.send,
                false,
                true,
            ),
            d.is_some_and(|d| d.send != v.send),
            false,
        );
        let mut fld2 = self.field(
            &format!("{k}/f2"),
            "Priority class",
            Some("delivery order and badging follow the class"),
            self.segment(
                "",
                &["oracle class", "worker class"],
                &format!("{} class", v.cls),
            ),
            d.is_some_and(|d| d.cls != v.cls),
            false,
        );
        if let c::Element::Named { content, .. } = &mut fld2
            && let c::Element::Box(b) = content.as_mut()
        {
            b.children.push(
                wrap(
                    "",
                    "changing class re-issues this role's seat grants",
                    font(11., 400, false, ui::Color::Muted),
                )
                .into(),
            );
        }
        let mut fld0 = fld0;
        if d.is_some_and(|d| d.prompt != v.prompt) {
            expand_row(
                &mut fld0,
                &format!("{k}/f0"),
                8.,
                self.width,
                self.ctx,
                self.m,
                false,
            );
        }

        let mut b = c::Box::new(
            surface(
                ui::Color::Surface,
                STYLE.material.card_alpha as f32,
                12.,
                [0.; 4],
                None,
            ),
            vec![
                row(&format!("{k}/band"), s, 8., band).into(),
                col(
                    "",
                    plain(None, [4., 12., 10., 12.], 0.),
                    0.,
                    vec![fld0.into(), fld1.into(), fld2.into()],
                )
                .into(),
            ],
        );
        b.direction = Axis::Column;
        b.align = c::Alignment::Stretch;
        b.gap = ui::Length::Px(0.);
        b.clip = true;
        let mut s = b.style.as_ref().clone();
        s = border(s, hair(self.ctx), true);
        b.style = Box::new(s);
        named(k, c::Element::Box(b))
    }
    fn add_card(&self, k: &str, title: &str, note: &str, height: f32) -> c::Element {
        let mut style = border(plain(Some(height), [0.; 4], 12.), ui::Color::Line, false);
        style.overrides.dashed = Some(true);
        let mut children = vec![
            fill(c::Element::Spacer),
            self.text(
                "",
                title,
                if note.is_empty() { 12.5 } else { 13. },
                500,
                false,
                false,
            )
            .into(),
        ];
        if !note.is_empty() {
            children.push(self.text("", note, 11., 400, false, true).into());
        }
        children.push(fill(c::Element::Spacer));
        let mut b = c::Box::new(style, children);
        b.direction = Axis::Column;
        b.align = c::Alignment::Center;
        b.gap = ui::Length::Px(4.);
        named(k, c::Element::Box(b))
    }
    fn harness_card(
        &self,
        k: &str,
        name: &str,
        v: &Harness,
        d: Option<&Harness>,
        launch: &[Launch],
        roles: &[String],
    ) -> c::Element {
        let label = launch
            .iter()
            .find(|h| h.harness == name)
            .map(|h| h.label.as_str())
            .unwrap_or(name);
        let title = col(
            "",
            plain(None, [0.; 4], 0.),
            1.,
            vec![
                named(
                    format!("{k}/hname"),
                    txt(
                        "",
                        label,
                        ui::Text {
                            line_height: 1.25,
                            ..font(13., 600, false, ui::Color::Foreground)
                        },
                    ),
                )
                .into(),
                txt(
                    &format!("{k}/command"),
                    &v.command,
                    ui::Text {
                        line_height: 1.25,
                        ..font(11., 400, true, ui::Color::Muted)
                    },
                )
                .into(),
            ],
        );
        let tile = row(
            &format!("{k}/tile"),
            surface(color(&v.tile), 1., 8., [0.; 4], Some(30.)),
            0.,
            vec![
                icon(
                    "",
                    if v.icon.is_empty() { name } else { &v.icon },
                    17.,
                    ui::Color::OnAccent,
                )
                .into(),
            ],
        );
        let head = row(
            &format!("{k}/head"),
            bottom(plain(None, [0., 0., 6., 0.], 0.), self.ctx, false),
            10.,
            vec![
                fixed(tile, 30.),
                fill(title),
                if v.custom {
                    self.text("", "delete", 10., 500, false, true)
                } else {
                    self.chip("", "built-in", true, None, None)
                }
                .into(),
            ],
        );
        let row0 = self.setting_row(
            &format!("{k}/r0"),
            "Default role",
            None,
            self.select("", roles, &v.role, true, false),
            d.is_some_and(|d| d.role != v.role),
            false,
            true,
        );
        let row1 = self.setting_row(
            &format!("{k}/r1"),
            "Wake",
            if v.custom && v.wake == "pty" {
                Some(if v.hooks {
                    "types after an idle hook"
                } else {
                    "never submits on its own"
                })
            } else {
                None
            },
            self.select(
                "",
                &["socket", "hook", "mcp", "pty"].map(str::to_string),
                &v.wake,
                true,
                false,
            ),
            d.is_some_and(|d| d.wake != v.wake),
            false,
            true,
        );
        let row2 = self.setting_row(
            &format!("{k}/r2"),
            "State from",
            None,
            self.select(
                "",
                &["osc7501", "hooks", "output cadence"].map(str::to_string),
                &v.state_from,
                true,
                false,
            ),
            d.is_some_and(|d| d.state_from != v.state_from),
            false,
            true,
        );
        let row3 = self.setting_row(
            &format!("{k}/r3"),
            "Hooks",
            Some("state from harness hooks"),
            self.toggle("", v.hooks),
            d.is_some_and(|d| d.hooks != v.hooks),
            true,
            true,
        );
        self.card(
            k,
            "",
            vec![
                head.into(),
                col(
                    "",
                    plain(None, [2., 0., 0., 0.], 0.),
                    0.,
                    vec![row0.into(), row1.into(), row2.into(), row3.into()],
                )
                .into(),
            ],
            [10., 12., 8., 12.],
        )
    }
}
impl Settings {
    pub fn element(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        height: f32,
    ) -> c::Element {
        self.element_with(ctx, m, width, height, &controller::Presentation::default())
    }
    pub fn element_with(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        height: f32,
        state: &controller::Presentation,
    ) -> c::Element {
        let b = Build {
            ctx,
            m,
            width: (width - 200. - 36. - 28.).max(1.),
        };
        let side_fill = mixed(
            ctx,
            ctx.theme.palette().sidebar,
            ctx.theme.palette().bg,
            STYLE.material.panel_alpha as f32,
        );
        let mut q = row(
            "search",
            border(
                surface(
                    ui::Color::Surface,
                    STYLE.material.card_alpha as f32,
                    10.,
                    [0., 10., 0., 9.],
                    Some(28.),
                ),
                hair(ctx),
                true,
            ),
            6.,
            vec![
                icon("", "search", 13., ui::Color::Muted).into(),
                if state.query.is_empty() && state.edit.is_none() {
                    b.text("", "Search settings", 12.5, 400, false, true)
                } else {
                    c::Element::EditingText(c::EditingText {
                        value: state.query.clone(),
                        placeholder: "Search settings".into(),
                        style: font(12.5, 400, false, ui::Color::Foreground),
                        placeholder_paint: ui::Paint::solid(ui::Color::Muted),
                        edit: state.edit.clone().unwrap_or_default(),
                        name: "settings/query".into(),
                    })
                }
                .into(),
            ],
        );
        if state.read_only
            && let c::Element::Named { content, .. } = &mut q
            && let c::Element::Box(box_) = content.as_mut()
        {
            box_.clip = true;
        }
        let nav = SECTIONS
            .iter()
            .enumerate()
            .map(|(i, (name, ico))| {
                row(
                    &format!("nav{i}"),
                    surface(
                        if *name == self.section {
                            tint(ctx, STYLE.tint.selection_alpha as f32)
                        } else {
                            ui::Color::Transparent
                        },
                        1.,
                        10.,
                        [0., 9., 0., 9.],
                        Some(28.),
                    ),
                    9.,
                    vec![
                        icon(
                            &format!("nav{i}/icon"),
                            ico,
                            14.,
                            if *name == self.section {
                                ui::Color::Accent
                            } else {
                                ui::Color::Muted
                            },
                        )
                        .into(),
                        b.text(
                            &format!("nav{i}/text"),
                            *name,
                            12.5,
                            500,
                            false,
                            *name != self.section,
                        )
                        .into(),
                    ],
                )
                .into()
            })
            .collect();
        let mut side = border(
            surface(side_fill, 1., 0., [10., 8., 10., 8.], Some(height)),
            hair(ctx),
            false,
        );
        side.overrides.border_sides = Some([false, true, false, false]);
        let side = col(
            "side",
            side,
            2.,
            vec![
                col("", plain(None, [0., 0., 8., 0.], 0.), 0., vec![q.into()]).into(),
                col("", plain(None, [0.; 4], 0.), 1., nav).into(),
                fill(c::Element::Spacer),
                col(
                    "source",
                    plain(None, [8., 9., 0., 9.], 0.),
                    0.,
                    vec![
                        row(
                            "",
                            plain(None, [0.; 4], 0.),
                            0.,
                            vec![
                                b.text(
                                    "source/text",
                                    "~/.config/bboxy/config.toml",
                                    10.,
                                    400,
                                    true,
                                    true,
                                )
                                .into(),
                            ],
                        )
                        .into(),
                    ],
                )
                .into(),
            ],
        );
        let content_width = (width - 200. - 36.).max(1.);
        let query = state.query.trim().to_lowercase();
        let kids = if query.is_empty() {
            self.presented_section(&b, content_width, state)
        } else {
            let mut matches = vec![];
            for (name, _) in SECTIONS {
                let mut view = self.clone();
                view.section = name.into();
                let cards = view.presented_section(&b, content_width, state);
                let cards: Vec<_> = cards
                    .into_iter()
                    .filter(|item| {
                        name.to_lowercase().contains(&query) || contains_text(&item.element, &query)
                    })
                    .collect();
                if !cards.is_empty() {
                    matches.push(
                        col(
                            &format!("results/{name}"),
                            plain(None, [0.; 4], 0.),
                            12.,
                            cards,
                        )
                        .into(),
                    );
                }
            }
            if matches.is_empty() {
                matches.push(b.note("empty", "nothing matches the search", false).into());
            }
            matches
        };
        let main = named(
            "main",
            c::Element::ScrollView(c::ScrollView {
                viewport: [(width - 200.).max(1.), height],
                offset: [0., state.scroll.max(0.)],
                content_width: None,
                content: Box::new(col("", plain(None, [14., 18., 32., 18.], 0.), 12., kids)),
            }),
        );
        let root = row(
            "settings",
            surface(ui::Color::Background, 1., 0., [0.; 4], Some(height)),
            0.,
            vec![fixed(side, 200.), fill(main)],
        );
        hosted::picker(root, state, ctx, m, width, height)
    }
    fn presented_section(
        &self,
        b: &Build,
        width: f32,
        state: &controller::Presentation,
    ) -> Vec<c::Item> {
        if state.read_only {
            match self.section.as_str() {
                "Roles" => return hosted::roles(self, b, width, state),
                "Harnesses" => return hosted::harnesses(self, b, width),
                "Cloud" => return vec![self.cloud(b, width, false).into()],
                "Machines" => return vec![self.machines(b, false).into()],
                _ => {}
            }
        }
        if !state.read_only
            || matches!(
                self.section.as_str(),
                "Appearance" | "Notifications" | "Plugins" | "Knowledge" | "Messaging"
            )
        {
            return self.section_elements_state(b, width, state.window_controls_available);
        }
        let unavailable = "Not exposed by the runtime";
        let pairs: Vec<(String, String)> = match self.section.as_str() {
            "Account" => vec![
                ("Identity".into(), self.values.account.key_state.clone()),
                ("Reason".into(), self.values.account.name.clone()),
                (
                    "Fingerprint, devices and signing policy".into(),
                    unavailable.into(),
                ),
            ],
            "Cloud" => self
                .values
                .cloud
                .iter()
                .map(|Cloud(name, region, on, _, link)| {
                    (
                        name.clone(),
                        format!(
                            "{} · {}",
                            region,
                            link.as_ref().map_or(
                                if *on {
                                    "connected".into()
                                } else {
                                    "not connected".into()
                                },
                                |v| v.label()
                            )
                        ),
                    )
                })
                .collect(),
            "Machines" => self
                .values
                .machines
                .iter()
                .map(|(name, address, _, _, state)| {
                    (
                        name.clone(),
                        format!(
                            "{address} · {}",
                            match state {
                                MachineState::Link(v) => v.label(),
                                MachineState::Legacy(on) =>
                                    if *on { "connected" } else { "not connected" }.into(),
                            }
                        ),
                    )
                })
                .collect(),
            "Roles" => self
                .values
                .roles
                .iter()
                .map(|(name, role)| {
                    (
                        name.clone(),
                        format!("{} · prompt and grants {unavailable}", role.cls),
                    )
                })
                .collect(),
            "Harnesses" => self
                .values
                .harness
                .iter()
                .map(|(name, h)| {
                    (
                        name.clone(),
                        format!(
                            "{} · role {} · wake {} · hooks {} · {}",
                            h.command, h.role, h.wake, h.hooks, h.state_from
                        ),
                    )
                })
                .collect(),
            "Shortcuts" => self.values.shortcuts.clone(),
            _ => vec![],
        };
        let mut rows: Vec<c::Item> = pairs
            .iter()
            .enumerate()
            .map(|(i, (label, value))| {
                b.setting_row(
                    &format!("readonly/r{i}"),
                    label,
                    None,
                    b.text("", value, 12., 400, false, true),
                    false,
                    false,
                    i + 1 == pairs.len(),
                )
                .into()
            })
            .collect();
        if rows.is_empty() {
            rows.push(
                b.note("empty", "No records in the current provider snapshot", true)
                    .into(),
            );
        }
        rows.push(b.note("readonly/reason","Read-only runtime projection. Editing account, providers, roles and harness authority is unavailable in this checkpoint.",true).into());
        vec![
            b.card("readonly", &self.section, rows, [4., 14., 10., 14.])
                .into(),
        ]
    }
    fn section_elements_state(
        &self,
        b: &Build,
        width: f32,
        window_controls_available: bool,
    ) -> Vec<c::Item> {
        match self.section.as_str() {
            "Roles" => {
                let mut items: Vec<_> = self
                    .values
                    .roles
                    .iter()
                    .enumerate()
                    .map(|(i, (n, v))| {
                        Build {
                            width: (width
                                - 10.
                                    * ((((width + 10.) / 240.).floor() as usize).max(1) - 1)
                                        as f32)
                                / ((((width + 10.) / 240.).floor() as usize).max(1)) as f32
                                - 24.,
                            ..*b
                        }
                        .role_card(
                            &format!("card{i}"),
                            n,
                            v,
                            self.defaults
                                .roles
                                .iter()
                                .find(|(k, _)| k == n)
                                .map(|(_, v)| v),
                        )
                    })
                    .collect();
                items.push(b.add_card(
                    &format!("card{}", items.len()),
                    "+ new role",
                    "your own mark, colour and prompt",
                    160.,
                ));
                vec![b.grid("roles", width, 230., 10., items).into()]
            }
            "Harnesses" => {
                let mut items: Vec<_> = self
                    .values
                    .harness
                    .iter()
                    .enumerate()
                    .map(|(i, (n, v))| {
                        Build {
                            width: (width
                                - 10.
                                    * ((((width + 10.) / 240.).floor() as usize).max(1) - 1)
                                        as f32)
                                / ((((width + 10.) / 240.).floor() as usize).max(1)) as f32
                                - 24.,
                            ..*b
                        }
                        .harness_card(
                            &format!("card{i}"),
                            n,
                            v,
                            self.defaults
                                .harness
                                .iter()
                                .find(|(k, _)| k == n)
                                .map(|(_, v)| v),
                            &self.launch,
                            &self
                                .values
                                .roles
                                .iter()
                                .map(|(n, _)| n.clone())
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect();
                if let Some(d) = &self.draft_harness {
                    items.push(self.draft_card(
                        &Build {
                            width: (width
                                - 10.
                                    * ((((width + 10.) / 240.).floor() as usize).max(1) - 1)
                                        as f32)
                                / ((((width + 10.) / 240.).floor() as usize).max(1)) as f32
                                - 24.,
                            ..*b
                        },
                        &format!("card{}", items.len()),
                        d,
                    ));
                } else {
                    items.push(b.add_card(
                        &format!("card{}", items.len()),
                        "+ new harness",
                        "any command that talks in a terminal",
                        150.,
                    ));
                }
                vec![b.grid("harnesses", width, 230., 10., items).into()]
            }
            "Cloud" => vec![self.cloud(b, width, true).into()],
            "Machines" => vec![self.machines(b, true).into()],
            "Shortcuts" => vec![self.shortcuts(b, width).into()],
            "Appearance" => vec![self.appearance(b, window_controls_available).into()],
            "Notifications" => vec![self.notifications(b).into()],
            "Messaging" => vec![self.messaging(b).into()],
            "Plugins" => vec![self.plugins(b).into()],
            "Knowledge" => vec![self.knowledge(b).into()],
            _ => self.account(b, width),
        }
    }
}
impl Settings {
    fn cloud(&self, b: &Build, width: f32, allow_create: bool) -> c::Element {
        let tiles = self
            .values
            .cloud
            .iter()
            .enumerate()
            .map(|(i, Cloud(name, region, on, co, link))| {
                let title = col(
                    "",
                    plain(None, [0.; 4], 0.),
                    0.,
                    vec![
                        txt(
                            "",
                            name,
                            ui::Text {
                                line_height: 1.25,
                                ..font(13., 600, false, ui::Color::Foreground)
                            },
                        )
                        .into(),
                        txt(
                            "",
                            region,
                            ui::Text {
                                line_height: 1.25,
                                ..font(11., 400, false, ui::Color::Muted)
                            },
                        )
                        .into(),
                    ],
                );
                let tile = row(
                    "",
                    surface(color(co), 1., 8., [0.; 4], Some(30.)),
                    0.,
                    vec![
                        b.text(
                            "",
                            name.chars().next().unwrap_or('?').to_string(),
                            14.,
                            700,
                            false,
                            false,
                        )
                        .into(),
                    ],
                );
                let head = row(
                    "",
                    plain(None, [0.; 4], 0.),
                    10.,
                    vec![
                        fixed(tile, 30.),
                        fill(title),
                        if *on {
                            link.as_ref()
                                .map_or_else(|| b.dot("", *on, 8.), |link| link.dot())
                        } else {
                            b.dot("", false, 8.)
                        }
                        .into(),
                    ],
                );
                let foot = row(
                    "",
                    plain(None, [0.; 4], 0.),
                    8.,
                    vec![
                        fill(b.text(
                            "",
                            if *on {
                                link.as_ref()
                                    .map_or_else(|| "connected".into(), |link| link.label())
                            } else {
                                "not connected".into()
                            },
                            11.,
                            400,
                            false,
                            true,
                        )),
                        if allow_create {
                            b.toggle("", *on)
                        } else {
                            c::Element::Spacer
                        }
                        .into(),
                    ],
                );
                let mut contents = vec![head.into(), foot.into()];
                if !allow_create
                    && let Some(control) = hosted::availability(self, b, &format!("cloud/{i}"))
                {
                    contents.push(control.into());
                }
                col(
                    &format!("cloud/{i}"),
                    border(
                        surface(ui::Color::Background, 0.45, 10., [10., 11., 10., 11.], None),
                        hair(b.ctx),
                        true,
                    ),
                    10.,
                    contents,
                )
            })
            .chain(allow_create.then(|| b.add_card("cloud/add", "+ connect a provider", "", 72.)))
            .collect();
        b.card("card0","Cloud providers",vec![col("",plain(None,[4.,0.,0.,0.],0.),0.,vec![b.grid("cloud",width-28.,190.,8.,tiles).into()]).into(),b.note("",if allow_create { "Regions appear under Cloud in the launcher. Credentials live in the keychain, never in config.toml." } else { "Existing destinations from the runtime provider snapshot. Connection configuration is read-only here." },true).into()],[4.,14.,10.,14.])
    }
    fn machines(&self, b: &Build, allow_create: bool) -> c::Element {
        let mut rows: Vec<c::Item> = self
            .values
            .machines
            .iter()
            .enumerate()
            .map(|(i, (host, user, hw, key, up))| {
                let link = match up {
                    MachineState::Link(link) => Some(link),
                    MachineState::Legacy(_) => None,
                };
                let address =
                    link.map_or_else(|| user.clone(), |link| format!("{user} · {}", link.label()));
                let mt = col(
                    "",
                    plain(None, [0.; 4], 0.),
                    0.,
                    vec![
                        txt(
                            "",
                            host,
                            ui::Text {
                                line_height: 1.25,
                                ..font(13., 600, false, ui::Color::Foreground)
                            },
                        )
                        .into(),
                        txt(
                            "",
                            address,
                            ui::Text {
                                line_height: 1.25,
                                ..font(11., 400, true, ui::Color::Muted)
                            },
                        )
                        .into(),
                    ],
                );
                let mut contents = vec![
                    fixed(
                        link.map_or_else(
                            || b.dot("", matches!(up, MachineState::Legacy(true)), 8.),
                            |link| link.dot(),
                        ),
                        8.,
                    ),
                    fill(mt),
                    b.text("", hw, 11., 400, false, true).into(),
                    row(
                        "",
                        surface(
                            badge(b.ctx, STYLE.tint.badge_alpha as f32),
                            1.,
                            5.,
                            [1., 6., 1., 6.],
                            None,
                        ),
                        0.,
                        vec![
                            txt(
                                "",
                                key,
                                ui::Text {
                                    line_height: 16. / 11.,
                                    ..font(11., 400, true, ui::Color::Foreground)
                                },
                            )
                            .into(),
                        ],
                    )
                    .into(),
                ];
                if !allow_create
                    && let Some(control) = hosted::availability(self, b, &format!("machine/{i}"))
                {
                    contents.push(control.into());
                }
                row(
                    &format!("machine{i}"),
                    bottom(plain(None, [8., 0., 8., 0.], 0.), b.ctx, false),
                    12.,
                    contents,
                )
                .into()
            })
            .collect();
        if allow_create {
            rows.push(
                row(
                    "",
                    bottom(plain(Some(44.), [10., 0., 10., 0.], 0.), b.ctx, false),
                    0.,
                    vec![b.text("", "+ add a machine", 12.5, 400, false, true).into()],
                )
                .into(),
            );
        }
        rows.push(
            b.note(
                "",
                if allow_create { "Read from ~/.ssh/config. Machines appear under SSH in the launcher." } else { "Existing destinations from the runtime provider snapshot. Connection configuration is read-only here." },
                true,
            )
            .into(),
        );
        b.card("card0", "SSH machines", rows, [4., 14., 10., 14.])
    }
    fn shortcuts(&self, b: &Build, width: f32) -> c::Element {
        let w = (width - 28. - 28.) / 2.;
        let rows = self
            .values
            .shortcuts
            .chunks(2)
            .enumerate()
            .map(|(ri, rs)| {
                row(
                    "",
                    plain(None, [0.; 4], 0.),
                    28.,
                    rs.iter()
                        .map(|(label, key)| {
                            fixed(
                                row(
                                    "",
                                    bottom(
                                        plain(Some(34.), [0.; 4], 0.),
                                        b.ctx,
                                        ri + 1 == self.values.shortcuts.len().div_ceil(2),
                                    ),
                                    12.,
                                    vec![
                                        fill(b.text("", label, 12.5, 400, false, false)),
                                        row(
                                            "",
                                            surface(
                                                badge(b.ctx, STYLE.tint.badge_alpha as f32),
                                                1.,
                                                5.,
                                                [1., 6., 1., 6.],
                                                None,
                                            ),
                                            0.,
                                            vec![
                                                txt(
                                                    "",
                                                    key,
                                                    ui::Text {
                                                        line_height: 16. / 11.,
                                                        ..font(
                                                            11.,
                                                            400,
                                                            true,
                                                            ui::Color::Foreground,
                                                        )
                                                    },
                                                )
                                                .into(),
                                            ],
                                        )
                                        .into(),
                                    ],
                                ),
                                w,
                            )
                        })
                        .collect(),
                )
                .into()
            })
            .collect();
        b.card(
            "card0",
            "Shortcuts",
            vec![col("", plain(None, [2., 0., 4., 0.], 0.), 0., rows).into()],
            [4., 14., 10., 14.],
        )
    }
    fn account(&self, b: &Build, width: f32) -> Vec<c::Item> {
        let a = &self.values.account;
        let d = &self.defaults.account;
        let ok = color(TOKENS.color.ui.ok);
        let status = row(
            "",
            surface(ok, 0.14, 999., [1., 8., 1., 6.], None),
            5.,
            vec![
                fixed(
                    row("", surface(ok, 1., 3., [0.; 4], Some(6.)), 0., vec![]),
                    6.,
                ),
                txt("", "signed in", font(11., 500, false, ok)).into(),
            ],
        );
        let name = row(
            "account/name",
            plain(None, [0.; 4], 0.),
            10.,
            vec![
                txt(
                    "",
                    &a.name,
                    ui::Text {
                        line_height: 1.2,
                        ..font(18., 600, false, ui::Color::Foreground)
                    },
                )
                .into(),
                status.into(),
            ],
        );
        let fp = flow(
            "account/fp",
            a.fingerprint
                .as_bytes()
                .chunks(4)
                .enumerate()
                .map(|(i, s)| {
                    let mut f = font(
                        15.,
                        500,
                        true,
                        if i % 2 == 0 {
                            ui::Color::Foreground
                        } else {
                            ui::Color::Muted
                        },
                    );
                    f.line_height = 1.3;
                    f.tracking_em = 0.02;
                    txt("", String::from_utf8_lossy(s).into_owned(), f).into()
                })
                .collect(),
            3.,
            9.,
            c::Alignment::Start,
        );
        let chips = flow(
            "",
            vec![
                mono(b.chip(
                    "account/algo",
                    &format!("{} · signing", a.algo),
                    false,
                    None,
                    Some("key"),
                ))
                .into(),
                row(
                    "",
                    surface(
                        badge(b.ctx, STYLE.tint.badge_alpha as f32),
                        1.,
                        6.,
                        [0., 8., 0., 8.],
                        Some(20.),
                    ),
                    6.,
                    vec![
                        b.dot("", true, 8.).into(),
                        b.text("", &a.key_state, 11., 400, false, false).into(),
                    ],
                )
                .into(),
                b.chip("", "copy fingerprint", false, None, Some("copy"))
                    .into(),
            ],
            6.,
            6.,
            c::Alignment::Center,
        );
        let mut trust = vec![
            row(
                "",
                plain(None, [0., 2., 0., 0.], 0.),
                0.,
                vec![b.text("", "trust path", 11., 400, false, true).into()],
            )
            .into(),
        ];
        for (i, t) in a.trust.iter().enumerate() {
            if i > 0 {
                trust.push(b.text("", "→", 11., 400, false, true).into());
            }
            trust.push(b.chip("", t, false, None, None).into());
        }
        let trust = flow("", trust, 6., 6., c::Alignment::Center);
        let held = row(
            "",
            plain(None, [0.; 4], 0.),
            8.,
            vec![
                natural(b.text(
                    "",
                    "primary held by gpg-agent · pinned from your keyring · never exported",
                    11.,
                    400,
                    false,
                    true,
                )),
                b.chip("", "open in gpg", false, None, Some("open")).into(),
            ],
        );
        let action = |title: &str, ico: &str, kind: &str, note: &str| {
            let mut f = font(10.5, 400, false, ui::Color::Muted);
            f.line_height = 1.35;
            col(
                "",
                plain(None, [0.; 4], 0.),
                4.,
                vec![
                    row(
                        "",
                        plain(None, [0.; 4], 0.),
                        0.,
                        vec![b.button("", title, Some(ico), kind).into()],
                    )
                    .into(),
                    wrap("", note, f).into(),
                ],
            )
        };
        let acts = align_box(
            row(
                "",
                plain(None, [4., 0., 0., 0.], 0.),
                14.,
                vec![
                    fixed(
                        action(
                            "Show paper recovery",
                            "key",
                            "primary",
                            "restores identity only · needs your GPG backup or paperkey plus the public certificate · seats, history and grants are not recovered",
                        ),
                        220.,
                    ),
                    fixed(
                        action(
                            "Link Google for cloud",
                            "link",
                            "link",
                            "binds a provider for cloud seats · the key stays the identity",
                        ),
                        220.,
                    ),
                ],
            ),
            c::Alignment::Stretch,
        );
        let hb = col(
            "",
            plain(None, [0.; 4], 0.),
            7.,
            vec![
                name.into(),
                fp.into(),
                chips.into(),
                b.text(
                    "",
                    format!("{} ({}) <{}>", a.name, a.handle, a.email),
                    11.,
                    400,
                    true,
                    true,
                )
                .into(),
                trust.into(),
                held.into(),
                acts.into(),
            ],
        );
        let avatar = named(
            "account/avatar",
            c::Element::Image {
                asset: "bboxy".into(),
                size: ui::Length::Px(72.),
                radius: ui::Length::Px(16.),
            },
        );
        let mut hero = c::Box::new(
            bottom(plain(None, [14., 0., 12., 0.], 0.), b.ctx, false),
            vec![fixed(avatar, 72.), fill(hb)],
        );
        hero.gap = ui::Length::Px(16.);
        hero.align = c::Alignment::Start;
        let group = align_box(
            row(
                "",
                plain(None, [12., 0., 6., 0.], 0.),
                10.,
                vec![
                    b.text("", "Devices · certified leaf keys", 12., 500, false, true)
                        .into(),
                    b.text(
                        "",
                        "revoke is local · peers learn on next contact",
                        11.,
                        400,
                        false,
                        true,
                    )
                    .into(),
                ],
            ),
            c::Alignment::Baseline,
        );
        let mut keys: Vec<_> = a
            .devices
            .iter()
            .map(|(n, s, when)| {
                col(
                    "",
                    border(
                        surface(ui::Color::Background, 0.45, 10., [9., 11., 9., 11.], None),
                        hair(b.ctx),
                        true,
                    ),
                    3.,
                    vec![
                        row(
                            "",
                            plain(None, [0.; 4], 0.),
                            8.,
                            vec![
                                fill(b.text("", n, 12.5, 600, false, false)),
                                b.dot("", true, 8.).into(),
                            ],
                        )
                        .into(),
                        b.text("", "device cert · ed25519", 10.5, 400, true, true)
                            .into(),
                        b.text("", format!("{s} · {when}"), 11., 400, false, true)
                            .into(),
                        col(
                            "",
                            plain(None, [4., 0., 0., 0.], 0.),
                            0.,
                            vec![
                                row(
                                    "",
                                    plain(None, [0.; 4], 0.),
                                    0.,
                                    vec![
                                        row(
                                            "",
                                            border(
                                                plain(None, [0., 8., 0., 8.], 999.),
                                                hair(b.ctx),
                                                false,
                                            ),
                                            0.,
                                            vec![
                                                txt(
                                                    "",
                                                    "revoke",
                                                    ui::Text {
                                                        line_height: 17. / 10.5,
                                                        ..font(10.5, 400, false, ui::Color::Muted)
                                                    },
                                                )
                                                .into(),
                                            ],
                                        )
                                        .into(),
                                    ],
                                )
                                .into(),
                            ],
                        )
                        .into(),
                    ],
                )
            })
            .collect();
        let mut enroll_style = border(plain(None, [9., 11., 9., 11.], 10.), ui::Color::Line, false);
        enroll_style.overrides.dashed = Some(true);
        let enroll_body = col(
            "",
            plain(None, [0.; 4], 0.),
            2.,
            vec![
                b.text(
                    "account/enroll/title",
                    "+ enroll a device",
                    12.5,
                    500,
                    false,
                    false,
                )
                .into(),
                b.text(
                    "account/enroll/hint",
                    "request if primary is elsewhere",
                    11.,
                    400,
                    false,
                    true,
                )
                .into(),
                col(
                    "",
                    plain(None, [2., 0., 0., 0.], 0.),
                    0.,
                    vec![
                        wrap(
                            "account/enroll/caption",
                            "primary not available here → produces a signed request",
                            ui::Text {
                                line_height: 1.3,
                                ..font(10., 400, false, ui::Color::Muted)
                            },
                        )
                        .into(),
                    ],
                )
                .into(),
            ],
        );
        keys.push(col(
            "account/enroll",
            enroll_style,
            0.,
            vec![
                fill(c::Element::Spacer),
                enroll_body.into(),
                fill(c::Element::Spacer),
            ],
        ));
        let identity = vec![
            named("account/hero", c::Element::Box(hero)).into(),
            group.into(),
            b.grid("account/devices", width - 28., 200., 8., keys)
                .into(),
        ];
        let first = b.card(
            if a.custody.is_some() {
                "card1"
            } else {
                "card0"
            },
            "",
            identity,
            [4., 14., 14., 14.],
        );
        let note=b.note("account/note","Devices and seats are leaf keys certified by your GPG primary; enrollment is a one-use ticket signed where the primary lives. Expired, revoked or forked authority blocks controls on its own, apart from seat state. Google is a later link for cloud; the primary stays the identity.",false);
        let sign_key = if a.custody.is_some() {
            "card2"
        } else {
            "card1"
        };
        let sign = b.card(
            sign_key,
            "Signatures",
            vec![
                b.setting_row(
                    &format!("{sign_key}/r0"),
                    "Sign commits as this key",
                    Some(&format!("git author and committer · Boxy <{}>", a.email)),
                    b.toggle("", a.sign.commits),
                    a.sign.commits != d.sign.commits,
                    false,
                    false,
                )
                .into(),
                b.setting_row(
                    &format!("{sign_key}/r1"),
                    "Sign messages and presence with the seat's leaf key",
                    Some("chain verifies to this key"),
                    b.toggle("", a.sign.messages),
                    a.sign.messages != d.sign.messages,
                    false,
                    false,
                )
                .into(),
                b.setting_row(
                    &format!("{sign_key}/r2"),
                    "Seats sign commits with their own keys",
                    Some("assumed, not decided"),
                    b.toggle("", a.sign.seat_commits),
                    a.sign.seat_commits != d.sign.seat_commits,
                    true,
                    false,
                )
                .into(),
            ],
            [4., 14., 10., 14.],
        );
        let mut cards = vec![];
        if let Some(custody) = &a.custody {
            cards.push(self.custody(b, custody).into());
        }
        cards.extend([first.into(), note.into(), sign.into()]);
        cards
    }
    fn custody(&self, b: &Build, value: &Custody) -> c::Element {
        let held = value.hold.is_some();
        let pending = value.attempt.as_deref() == Some("pending");
        let waiting = color(TOKENS.color.state.waiting);
        let chip = |value: &str, co| {
            row(
                "",
                surface(
                    badge(b.ctx, STYLE.tint.badge_alpha as f32),
                    1.,
                    6.,
                    [0., 8., 0., 8.],
                    Some(20.),
                ),
                6.,
                vec![txt("", value, font(11., 400, false, co)).into()],
            )
        };
        let status_color = |co| mixed(b.ctx, co, b.ctx.theme.palette().fg, 0.7);
        let tier = match value.tier.as_str() {
            "keychain" => "login Keychain · native",
            "file" => "file · test account",
            "local-dev" => "local-dev issuer",
            other => other,
        };
        let acl = match value.acl.as_str() {
            "designated" => {
                "keyed to the team-signed requirement (UQB3368A84), verified · every build keeps access"
            }
            "legacy" => "keyed to one build’s signature · migrates during the one unlock",
            "unknown" => "unknown until verified",
            other => other,
        };
        let mut children = vec![
            b.setting_row(
                "card0/r0",
                "Device key",
                Some("where the device leaf key lives"),
                chip(tier, ui::Color::Foreground),
                false,
                false,
                false,
            )
            .into(),
            b.setting_row(
                "card0/r1",
                "Access rule",
                Some("the Keychain item’s ACL"),
                chip(
                    acl,
                    status_color(if value.acl == "designated" {
                        TOKENS.color.ui.ok
                    } else {
                        TOKENS.color.state.waiting
                    }),
                ),
                false,
                false,
                false,
            )
            .into(),
            b.setting_row(
                "card0/r2",
                "Signing",
                Some(if held {
                    "held, fail closed; the terminal keeps running"
                } else {
                    "available"
                }),
                chip(
                    if held {
                        if value.attempt.as_deref() == Some("granted") {
                            "signature pending"
                        } else {
                            "held"
                        }
                    } else {
                        "ready"
                    },
                    status_color(if held {
                        TOKENS.color.state.waiting
                    } else {
                        TOKENS.color.ui.ok
                    }),
                ),
                false,
                !held && value.attempt.is_none(),
                false,
            )
            .into(),
        ];
        if let Some(hold) = &value.hold {
            let mut detail = c::Text::new(
                "Clear it with Unlock here or ",
                font(12.5, 400, false, ui::Color::Foreground),
            );
            detail.runs.push(c::TextRun {
                value: "bboxy450-term ctl identity unlock".into(),
                style: font(10., 400, true, ui::Color::Foreground),
            });
            detail.runs.push(c::TextRun{value:": one interactive Keychain access, macOS shows its own dialog, then the no-dialog default returns.".into(),style:font(12.5,400,false,ui::Color::Foreground)});
            let button = if pending {
                "Waiting for macOS…"
            } else if value.attempt.as_deref() == Some("unknown") {
                "Check result"
            } else {
                "Unlock"
            };
            children.push(
                col(
                    "custody/hold-margin",
                    plain(
                        None,
                        [10., 0., if value.attempt.is_some() { 0. } else { 6. }, 0.],
                        0.,
                    ),
                    0.,
                    vec![
                        col(
                            "custody/hold",
                            surface(waiting, 0.12, 6., [10., 12., 10., 12.], None),
                            6.,
                            vec![
                                wrap(
                                    "custody/reason",
                                    format!("Pending: {}; signing held", hold.reason),
                                    font(12.5, 600, false, ui::Color::Foreground),
                                )
                                .into(),
                                named("custody/detail", c::Element::Text(detail)).into(),
                                row(
                                    "custody/act",
                                    plain(None, [0.; 4], 0.),
                                    0.,
                                    vec![natural(b.button_sized(
                                        "custody/unlock",
                                        button,
                                        Some(if value.attempt.as_deref() == Some("unknown") {
                                            "open"
                                        } else {
                                            "key"
                                        }),
                                        if pending { "quiet" } else { "primary" },
                                        12.5,
                                    ))],
                                )
                                .into(),
                            ],
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        let unknown = format!(
            "signing still held · the result is kept under op {} · Check result reads it, never reopens the dialog",
            value.op.as_deref().unwrap_or("—")
        );
        let returned = format!(
            "signing ready · a device-signed completion links to the pending entry {}",
            value.op.as_deref().unwrap_or("")
        );
        let outcome = match value.attempt.as_deref() {
            Some("pending") => Some((
                "waiting for the macOS Keychain dialog",
                "act journaled on account/control · signature pending",
                ui::Color::Foreground,
            )),
            Some("cancelled") => Some((
                "you cancelled the macOS dialog",
                "signing still held · the no-dialog default is back · Unlock again to retry",
                mixed(
                    b.ctx,
                    TOKENS.color.state.waiting,
                    b.ctx.theme.palette().fg,
                    0.7,
                ),
            )),
            Some("unknown") => Some((
                "no answer from the Keychain",
                unknown.as_str(),
                mixed(
                    b.ctx,
                    TOKENS.color.state.waiting,
                    b.ctx.theme.palette().fg,
                    0.7,
                ),
            )),
            Some("granted") => Some((
                "macOS granted access",
                "signature pending until the device signer returns · the pending entry stays as written",
                mixed(b.ctx, TOKENS.color.ui.ok, b.ctx.theme.palette().fg, 0.7),
            )),
            Some("returned") => Some((
                "the device signer returned",
                returned.as_str(),
                ui::Color::Foreground,
            )),
            _ => None,
        };
        if let Some((title, detail, co)) = outcome {
            let mut text = c::Text::new(title, font(10., 600, false, co));
            text.runs.push(c::TextRun {
                value: format!(" · {detail}"),
                style: font(10., 400, false, ui::Color::Muted),
            });
            children.push(
                col(
                    "custody/out-margin",
                    plain(None, [6., 0., 6., 0.], 0.),
                    0.,
                    vec![
                        col(
                            "custody/out",
                            surface(
                                badge(b.ctx, STYLE.tint.badge_alpha as f32),
                                1.,
                                6.,
                                [6., 12., 6., 12.],
                                None,
                            ),
                            0.,
                            vec![named("custody/out/text", c::Element::Text(text)).into()],
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        b.card("card0", "Custody", children, [4., 14., 10., 14.])
    }
    fn draft_card(&self, b: &Build, k: &str, d: &DraftHarness) -> c::Element {
        let hue =
            [0., 30., 55., 95., 160., 190., 215., 250., 285., 325.][d.ci.rem_euclid(10) as usize];
        let co = ui::Color::Rgba(hsl(hue, 0.85, 0.64));
        let input = |v: &str, mono: bool| {
            row(
                "",
                border(
                    surface(
                        mixed(
                            b.ctx,
                            b.ctx.theme.palette().bg,
                            b.ctx.theme.palette().surface,
                            0.6,
                        ),
                        1.,
                        6.,
                        [3., 7., 3., 7.],
                        None,
                    ),
                    hair(b.ctx),
                    true,
                ),
                0.,
                vec![
                    txt(
                        "",
                        v,
                        ui::Text {
                            line_height: 1.25,
                            ..font(
                                if mono { 10.5 } else { 12.5 },
                                400,
                                mono,
                                ui::Color::Foreground,
                            )
                        },
                    )
                    .into(),
                ],
            )
        };
        let head = row(
            &format!("{k}/head"),
            bottom(plain(None, [0., 0., 6., 0.], 0.), b.ctx, false),
            10.,
            vec![
                fixed(
                    row("", surface(co, 1., 7., [0.; 4], Some(14.)), 0., vec![]),
                    14.,
                ),
                fill(col(
                    "",
                    plain(None, [0.; 4], 0.),
                    1.,
                    vec![
                        named(format!("{k}/hname"), input(&d.name, false)).into(),
                        input(&d.command, true).into(),
                    ],
                )),
                b.chip("", "new", true, None, None).into(),
            ],
        );
        let marks = row(
            "",
            plain(None, [0.; 4], 0.),
            4.,
            ["claude", "codex", "pi", "omp", "zsh"]
                .iter()
                .map(|i| {
                    fixed(
                        row(
                            "",
                            surface(
                                if *i == d.icon {
                                    co
                                } else {
                                    badge(b.ctx, STYLE.tint.badge_alpha as f32)
                                },
                                1.,
                                6.,
                                [0.; 4],
                                Some(26.),
                            ),
                            0.,
                            vec![
                                icon(
                                    "",
                                    i,
                                    14.,
                                    if *i == d.icon {
                                        ui::Color::OnAccent
                                    } else {
                                        ui::Color::Muted
                                    },
                                )
                                .into(),
                            ],
                        ),
                        28.,
                    )
                })
                .collect(),
        );
        let roles = self
            .values
            .roles
            .iter()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>();
        let rows = vec![
            b.setting_row(
                &format!("{k}/r0"),
                "Default role",
                None,
                b.select("", &roles, &d.role, true, false),
                false,
                false,
                true,
            )
            .into(),
            b.setting_row(
                &format!("{k}/r1"),
                "Wake",
                if d.wake == "pty" {
                    Some(if d.hooks {
                        "types after an idle hook"
                    } else {
                        "never submits on its own"
                    })
                } else {
                    None
                },
                b.select(
                    "",
                    &["socket", "hook", "mcp", "pty"].map(str::to_string),
                    &d.wake,
                    true,
                    false,
                ),
                false,
                false,
                true,
            )
            .into(),
            b.setting_row(
                &format!("{k}/r2"),
                "State from",
                None,
                b.select(
                    "",
                    &["osc7501", "hooks", "output cadence"].map(str::to_string),
                    &d.state_from,
                    true,
                    false,
                ),
                false,
                false,
                true,
            )
            .into(),
            b.setting_row(
                &format!("{k}/r3"),
                "Hooks",
                None,
                b.toggle("", d.hooks),
                false,
                false,
                true,
            )
            .into(),
        ];
        let mut children = vec![
            col("", plain(None, [0., 0., 2., 0.], 0.), 0., vec![head.into()]).into(),
            with_bottom(
                b.field(&format!("{k}/f0"), "Icon", None, marks, false, true),
                b.ctx,
            )
            .into(),
        ];
        children.extend(rows);
        children.push(col("",plain(None,[6.,0.,0.,0.],0.),0.,vec![wrap("","unknown harnesses run through the PTY adapter: state is inferred from output until hooks or OSC 7501 upgrade it",ui::Text{line_height:1.5,..font(11.,400,false,ui::Color::Muted)}).into()]).into());
        children.push(
            row(
                "",
                plain(None, [8., 0., 0., 0.], 0.),
                8.,
                vec![
                    b.button("", "Add harness", None, "primary").into(),
                    b.button("", "Cancel", None, "ghost").into(),
                ],
            )
            .into(),
        );
        b.card(k, "", children, [10., 12., 8., 12.])
    }

    fn appearance(&self, b: &Build, window_controls_available: bool) -> c::Element {
        let v = &self.values.appearance;
        let d = &self.defaults.appearance;
        let mut rows = vec![];
        let themes = row(
            "",
            plain(None, [0.; 4], 0.),
            8.,
            ["auto", "dark", "light"]
                .iter()
                .map(|t| self.theme_thumb(b, t, *t == v.theme).into())
                .collect(),
        );
        rows.push(
            b.setting_row(
                "card0/r0",
                "Theme",
                Some("starts from the OS"),
                themes,
                v.theme != d.theme,
                false,
                false,
            )
            .into(),
        );
        let dens = row(
            "",
            plain(None, [0.; 4], 0.),
            8.,
            [
                bbt_ui_model::Density::Touch,
                bbt_ui_model::Density::Comfortable,
                bbt_ui_model::Density::Cozy,
                bbt_ui_model::Density::Compact,
            ]
            .iter()
            .map(|d| {
                let name = match d {
                    bbt_ui_model::Density::Touch => "touch",
                    bbt_ui_model::Density::Comfortable => "comfortable",
                    bbt_ui_model::Density::Cozy => "cozy",
                    bbt_ui_model::Density::Compact => "compact",
                };
                self.density_thumb(b, *d, name, name == v.density).into()
            })
            .collect(),
        );
        rows.push(
            b.setting_row(
                "card0/r1",
                "Density",
                Some("row height on the map"),
                dens,
                v.density != d.density,
                false,
                false,
            )
            .into(),
        );
        let (lo, hi) = (TOKENS.glow.rim_min as f32, TOKENS.glow.rim_max as f32);
        let rim = row(
            "",
            plain(None, [0.; 4], 0.),
            10.,
            vec![
                fixed(slider(((v.rim - lo) / (hi - lo)).clamp(0., 1.)), 120.),
                fixed(
                    b.text("", format!("{} px", v.rim), 11., 500, true, false),
                    35.2,
                ),
            ],
        );
        rows.push(
            b.setting_row(
                "card0/r2",
                "Rim size",
                Some(&format!(
                    "the glowing border, half outside and half inside · panes space to fit · {lo} to {hi} px"
                )),
                rim,
                v.rim != self.defaults.appearance.rim,
                false,
                false,
            )
            .into(),
        );
        let opacity = row(
            "",
            plain(None, [0.; 4], 0.),
            10.,
            vec![
                fixed(self.opacity_preview(b), 64.),
                fixed(self.opacity_slider(b), 120.),
                fixed(
                    b.text(
                        "",
                        format!("{}%", (v.opacity * 100.).round()),
                        11.,
                        500,
                        true,
                        false,
                    ),
                    35.2,
                ),
            ],
        );
        rows.push(
            b.setting_row(
                "card0/r3",
                "Window opacity",
                Some(if window_controls_available {
                    "panes only; chrome and text stay opaque"
                } else {
                    "not yet applied to the window"
                }),
                if window_controls_available {
                    opacity
                } else {
                    faded(opacity, 0.4)
                },
                v.opacity != d.opacity,
                false,
                false,
            )
            .into(),
        );
        rows.push(
            b.setting_row(
                "card0/r4",
                "Blur behind",
                Some(if window_controls_available {
                    "what is behind the window, like iTerm2"
                } else {
                    "not yet applied to the window"
                }),
                if window_controls_available {
                    b.toggle("", v.blur)
                } else {
                    faded(b.toggle("", v.blur), 0.4)
                },
                v.blur != d.blur,
                false,
                false,
            )
            .into(),
        );
        if !window_controls_available && !v.blur {
            rows.push(
                b.setting_row(
                    "unavailable/blur-radius",
                    "Blur radius",
                    Some("not yet applied to the window"),
                    faded(
                        b.segment(
                            "",
                            &["10 px", "20 px", "40 px"],
                            &format!("{} px", v.blur_radius),
                        ),
                        0.4,
                    ),
                    false,
                    false,
                    false,
                )
                .into(),
            );
        }
        let mut i = 5;
        if v.blur {
            rows.push(
                b.setting_row(
                    "card0/r5",
                    "Blur radius",
                    (!window_controls_available).then_some("not yet applied to the window"),
                    faded(
                        b.segment(
                            "",
                            &["10 px", "20 px", "40 px"],
                            &format!("{} px", v.blur_radius),
                        ),
                        if window_controls_available { 1. } else { 0.4 },
                    ),
                    false,
                    false,
                    false,
                )
                .into(),
            );
            i += 1;
        }
        rows.push(
            b.setting_row(
                &format!("card0/r{i}"),
                "Border glow",
                Some("particles in the seat colour"),
                b.toggle("", v.glow),
                v.glow != d.glow,
                false,
                false,
            )
            .into(),
        );
        i += 1;
        rows.push(
            b.setting_row(
                &format!("card0/r{i}"),
                "Last-output subtitle",
                Some("second line on seat rows"),
                b.toggle("", v.subtitle),
                v.subtitle != d.subtitle,
                true,
                false,
            )
            .into(),
        );
        b.card("card0", "Appearance", rows, [4., 14., 10., 14.])
    }
    fn notifications(&self, b: &Build) -> c::Element {
        let v = &self.values.notifications;
        let d = &self.defaults.notifications;
        let mut rows = vec![
            b.setting_row(
                "card0/r0",
                "Desktop notifications",
                Some("a seat that needs your answer posts one; clicking it focuses the seat"),
                b.toggle("", v.enabled),
                v.enabled != d.enabled,
                false,
                false,
            )
            .into(),
            b.setting_row(
                "card0/r1",
                "Only in the background",
                Some("none while this window is frontmost"),
                faded(
                    b.toggle("", v.only_when_not_frontmost),
                    if v.enabled { 1. } else { 0.4 },
                ),
                v.only_when_not_frontmost != d.only_when_not_frontmost,
                false,
                false,
            )
            .into(),
        ];
        let mut group = c::Box::new(
            plain(None, [12., 0., 6., 0.], 0.),
            vec![
                b.text("", "Per harness", 12., 500, false, true).into(),
                b.text(
                    "",
                    "off silences every seat of that harness",
                    11.,
                    400,
                    false,
                    true,
                )
                .into(),
            ],
        );
        group.gap = ui::Length::Px(10.);
        group.align = c::Alignment::Baseline;
        rows.push(named("group", c::Element::Box(group)).into());
        for (i, (name, harness)) in self.values.harness.iter().enumerate() {
            let key = format!("card0/r{}", i + 2);
            let on = !v.disabled_harnesses.contains(name);
            let mut item = b.setting_row(
                &key,
                name,
                None,
                faded(b.toggle("", on), if v.enabled { 1. } else { 0.4 }),
                on != !d.disabled_harnesses.contains(name),
                false,
                false,
            );
            let label = row(
                "",
                plain(None, [0.; 4], 0.),
                6.,
                vec![
                    icon(
                        "",
                        if harness.icon.is_empty() {
                            "zsh"
                        } else {
                            &harness.icon
                        },
                        16.,
                        ui::Color::Foreground,
                    )
                    .into(),
                    txt(
                        &format!("{key}/label/text"),
                        name,
                        font(13., 500, false, ui::Color::Foreground),
                    )
                    .into(),
                ],
            );
            // .lbl is an inline formatting context around the .nh inline-flex.
            // Include its inherited font strut, just as for the launcher kind label.
            let mut label_style = plain(None, [0.; 4], 0.);
            label_style.overrides.text = Some(font(13., 500, false, ui::Color::Foreground));
            // CSS exposes the first flex child's baseline after align-items:center.
            // The icon's baseline is its bottom edge, shifted by the cross-axis centring.
            let height = label
                .natural(b.width, b.ctx, b.m)
                .expect("Notifications label Style")
                .0[1];
            let label = c::Element::Baseline {
                content: Box::new(label),
                offset: (height - 16.) * 0.5,
            };
            let mut inline = c::Box::new(label_style, vec![label.into()]);
            inline.inline = true;
            inline.align = c::Alignment::Baseline;
            inline.gap = ui::Length::Px(0.);
            replace_named(&mut item, &format!("{key}/label"), &c::Element::Box(inline));
            rows.push(item.into());
        }
        rows.push(
            b.note(
                "note",
                "A muted seat (⌘M) never notifies and keeps the ordinary waiting rim.",
                true,
            )
            .into(),
        );
        b.card("card0", "Notifications", rows, [4., 14., 10., 14.])
    }
    fn theme_thumb(&self, b: &Build, t: &str, selected: bool) -> c::Element {
        let p = if t == "light" {
            &TOKENS.color.light
        } else {
            &TOKENS.color.dark
        };
        let base = col(
            "",
            surface(color(p.bg), 1., 6., [0.; 4], Some(48.)),
            0.,
            vec![],
        );
        let mut overlays = vec![
            c::Placed {
                content: row(
                    "",
                    surface(color(p.surface), 1., 0., [0.; 4], Some(7.)),
                    0.,
                    vec![],
                ),
                left: 0.,
                bottom: 41.,
            },
            c::Placed {
                content: row(
                    "",
                    surface(color(p.sidebar), 1., 0., [0.; 4], Some(41.)),
                    0.,
                    vec![],
                ),
                left: 0.,
                bottom: 0.,
            },
            c::Placed {
                content: row(
                    "",
                    surface(color(p.surface_2), 1., 2., [0.; 4], Some(33.)),
                    0.,
                    vec![],
                ),
                left: 26.,
                bottom: 4.,
            },
        ];
        // Layers' fixed child widths are source geometry, not browser positions.
        for (i, p) in overlays.iter_mut().enumerate() {
            if let c::Element::Named { content, .. } = &mut p.content
                && let c::Element::Box(b) = content.as_mut()
            {
                b.style.overrides.width = Some(ui::Length::Px([72., 22., 42.][i]));
            }
        }
        if t == "auto" {
            for (left, bottom, w, h, co, radius) in [
                (36., 0., 36., 48., TOKENS.color.light.bg, 0.),
                (36., 41., 36., 7., TOKENS.color.light.surface, 0.),
                (40., 4., 28., 33., TOKENS.color.light.surface_2, 2.),
            ] {
                overlays.push(c::Placed {
                    content: row(
                        "",
                        surface(color(co), 1., radius, [0.; 4], Some(h)),
                        0.,
                        vec![fixed(c::Element::Spacer, w)],
                    ),
                    left,
                    bottom,
                });
            }
        }
        let mut s = border(
            plain(None, [0.; 4], 6.),
            if selected {
                ui::Color::Accent
            } else {
                hair(b.ctx)
            },
            true,
        );
        if selected {
            s.overrides.border.as_mut().unwrap().width = ui::Length::Px(2.);
        }
        let mut bx = c::Box::new(
            s,
            vec![fixed(
                c::Element::Layers {
                    base: Box::new(base),
                    overlays,
                },
                72.,
            )],
        );
        bx.clip = true;
        bx.gap = ui::Length::Px(0.);
        align_box(
            col(
                "",
                plain(None, [4.; 4], 10.),
                5.,
                vec![
                    fixed(c::Element::Box(bx), 48.),
                    txt(
                        "",
                        t,
                        ui::Text {
                            line_height: 1.2,
                            ..font(13., 400, false, ui::Color::Foreground)
                        },
                    )
                    .into(),
                ],
            ),
            c::Alignment::Center,
        )
    }
    fn density_thumb(
        &self,
        b: &Build,
        d: bbt_ui_model::Density,
        name: &str,
        selected: bool,
    ) -> c::Element {
        let t = d.tokens();
        let rows = (100. / t.row_h as f32).floor() as usize;
        let mut all = vec![];
        for i in 0..rows {
            let tile = row(
                "",
                surface(
                    badge(b.ctx, STYLE.tint.badge_selected_alpha as f32),
                    1.,
                    TOKENS.radius.grid as f32,
                    [0.; 4],
                    Some(t.icon as f32),
                ),
                0.,
                vec![],
            );
            let bar = row(
                "",
                surface(ui::Color::Muted, 0.55, 2., [0.; 4], Some(3.)),
                0.,
                vec![],
            );
            let glyph = row(
                "",
                surface(
                    color(TOKENS.color.state.idle),
                    1.,
                    t.glyph as f32 / 4.,
                    [0.; 4],
                    Some(t.glyph as f32 / 2.),
                ),
                0.,
                vec![],
            );
            all.push(
                row(
                    "",
                    bottom(
                        surface(
                            if i == 1 {
                                tint(b.ctx, STYLE.tint.selection_alpha as f32)
                            } else {
                                ui::Color::Transparent
                            },
                            1.,
                            0.,
                            [
                                0.,
                                (t.pad_x as f32 / 2.).round(),
                                0.,
                                (t.pad_x as f32 / 2.).round(),
                            ],
                            Some(t.row_h as f32),
                        ),
                        b.ctx,
                        false,
                    ),
                    t.gap as f32,
                    vec![
                        fixed(tile, t.icon as f32),
                        fill(bar),
                        fixed(glyph, t.glyph as f32 / 2.),
                    ],
                )
                .into(),
            );
        }
        let mut s = border(
            surface(
                mixed(
                    b.ctx,
                    b.ctx.theme.palette().sidebar,
                    b.ctx.theme.palette().bg,
                    STYLE.material.panel_alpha as f32,
                ),
                1.,
                6.,
                [0.; 4],
                Some(100.),
            ),
            if selected {
                ui::Color::Accent
            } else {
                hair(b.ctx)
            },
            true,
        );
        if selected {
            s.overrides.border.as_mut().unwrap().width = ui::Length::Px(2.);
        }
        s.overrides.width = Some(ui::Length::Px(76.));
        let mut bx = c::Box::new(s, all);
        bx.direction = Axis::Column;
        bx.align = c::Alignment::Stretch;
        bx.gap = ui::Length::Px(0.);
        bx.clip = true;
        align_box(
            col(
                "",
                plain(None, [4.; 4], 10.),
                5.,
                vec![
                    c::Element::Box(bx).into(),
                    align_box(
                        col(
                            "",
                            plain(None, [0.; 4], 0.),
                            0.,
                            vec![
                                txt(
                                    "",
                                    name,
                                    ui::Text {
                                        line_height: 1.2,
                                        ..font(13., 400, false, ui::Color::Foreground)
                                    },
                                )
                                .into(),
                                txt(
                                    "",
                                    format!("{} px", t.row_h),
                                    ui::Text {
                                        line_height: 1.2,
                                        ..font(9.5, 400, true, ui::Color::Muted)
                                    },
                                )
                                .into(),
                            ],
                        ),
                        c::Alignment::Center,
                    )
                    .into(),
                ],
            ),
            c::Alignment::Center,
        )
    }
    fn opacity_slider(&self, _b: &Build) -> c::Element {
        slider(
            ((self.values.appearance.opacity - TOKENS.window.opacity_min as f32)
                / (1. - TOKENS.window.opacity_min as f32))
                .clamp(0., 1.),
        )
    }
    fn opacity_preview(&self, b: &Build) -> c::Element {
        let mut base = c::Box::new(
            border(
                surface(ui::Color::Background, 1., 6., [0.; 4], Some(40.)),
                hair(b.ctx),
                true,
            ),
            vec![fixed(c::Element::Spacer, 64.)],
        );
        base.gap = ui::Length::Px(0.);
        base.clip = true;
        let side = row(
            "",
            surface(
                color(b.ctx.theme.palette().sidebar),
                1.,
                0.,
                [0.; 4],
                Some(40.),
            ),
            0.,
            vec![fixed(c::Element::Spacer, 16.)],
        );
        let pane = col(
            "",
            surface(
                ui::Color::Background,
                self.values.appearance.opacity,
                4.,
                [5.; 4],
                Some(36.),
            ),
            3.,
            [0.7, 0.45, 0.58]
                .iter()
                .map(|fraction| {
                    row(
                        "",
                        plain(None, [0.; 4], 0.),
                        0.,
                        vec![fixed(
                            row(
                                "",
                                surface(ui::Color::Muted, 1., 1., [0.; 4], Some(2.)),
                                0.,
                                vec![],
                            ),
                            34. * fraction,
                        )],
                    )
                    .into()
                })
                .collect(),
        );
        c::Element::Layers {
            base: Box::new(c::Element::Box(base)),
            overlays: vec![
                c::Placed {
                    content: side,
                    left: 0.,
                    bottom: 0.,
                },
                c::Placed {
                    content: row("", plain(None, [0.; 4], 0.), 0., vec![fixed(pane, 44.)]),
                    left: 18.,
                    bottom: 2.,
                },
            ],
        }
    }
}
fn hsl(h: f32, s: f32, l: f32) -> [f32; 4] {
    let a = s * l.min(1. - l);
    let f = |n: f32| {
        let k = (n + h / 30.) % 12.;
        l - a * (k - 3.).min(9. - k).clamp(-1., 1.)
    };
    [f(0.), f(8.), f(4.), 1.]
}
fn faded(element: c::Element, alpha: f32) -> c::Element {
    if alpha == 1. {
        return element;
    }
    let mut style = plain(None, [0.; 4], 0.);
    style.overrides.opacity = Some(alpha);
    let mut b = c::Box::new(style, vec![element.into()]);
    b.gap = ui::Length::Px(0.);
    b.group = true;
    c::Element::Box(b)
}
fn replace_named(element: &mut c::Element, key: &str, replacement: &c::Element) -> bool {
    match element {
        c::Element::Named { name, content } if name == key => {
            **content = replacement.clone();
            true
        }
        c::Element::Named { content, .. } => replace_named(content, key, replacement),
        c::Element::Box(b) => b
            .children
            .iter_mut()
            .any(|item| replace_named(&mut item.element, key, replacement)),
        c::Element::Layers { base, overlays } => {
            replace_named(base, key, replacement)
                || overlays
                    .iter_mut()
                    .any(|placed| replace_named(&mut placed.content, key, replacement))
        }
        _ => false,
    }
}
fn contains_text(element: &c::Element, query: &str) -> bool {
    match element {
        c::Element::Text(t) => t
            .runs
            .iter()
            .any(|r| r.value.to_lowercase().contains(query)),
        c::Element::EditingText(t) => t.value.to_lowercase().contains(query),
        c::Element::Named { content, .. } | c::Element::Baseline { content, .. } => {
            contains_text(content, query)
        }
        c::Element::Box(b) => b
            .children
            .iter()
            .any(|item| contains_text(&item.element, query)),
        c::Element::ScrollView(s) => contains_text(&s.content, query),
        c::Element::Layers { base, overlays } => {
            contains_text(base, query) || overlays.iter().any(|p| contains_text(&p.content, query))
        }
        _ => false,
    }
}

fn set_height(e: &mut c::Element, h: f32) {
    match e {
        c::Element::Named { content, .. } => set_height(content, h),
        c::Element::Box(b) => b.style.overrides.height = Some(ui::Length::Px(h)),
        _ => {}
    }
}
fn expand_row(
    e: &mut c::Element,
    k: &str,
    pad: f32,
    width: f32,
    ctx: ui::Context,
    m: &dyn c::Measure,
    move_content: bool,
) {
    let content_width = width + if move_content { pad * 2. } else { 0. };
    let height = e
        .layout(
            bbt_ui_model::layout::Rect::new(0., 0., content_width, 0.),
            ctx,
            m,
        )
        .expect("changed Settings row")
        .rect
        .h;
    let base = row("", plain(Some(height), [0.; 4], 0.), 0., vec![]);
    let mut background = surface(ui::Color::Accent, 0.06, 10., [0.; 4], Some(height));
    background.overrides.width = None;
    let old = std::mem::replace(e, c::Element::Spacer);
    let mut content = match old {
        c::Element::Named { content, .. } => *content,
        other => other,
    };
    if let c::Element::Box(b) = &mut content {
        b.style.overrides.width = Some(ui::Length::Px(content_width));
    }
    *e = c::Element::Layers {
        base: Box::new(base),
        overlays: vec![
            c::Placed {
                content: row(
                    k,
                    background,
                    0.,
                    vec![fixed(c::Element::Spacer, width + pad * 2.)],
                ),
                left: -pad,
                bottom: 0.,
            },
            c::Placed {
                content: if move_content {
                    row(
                        "",
                        plain(None, [0.; 4], 0.),
                        0.,
                        vec![fixed(content, content_width)],
                    )
                } else {
                    content
                },
                left: if move_content { -pad } else { 0. },
                bottom: 0.,
            },
        ],
    };
}

fn with_bottom(mut e: c::Element, ctx: ui::Context) -> c::Element {
    if let c::Element::Named { content, .. } = &mut e
        && let c::Element::Box(b) = content.as_mut()
    {
        *b.style = bottom(*b.style.clone(), ctx, false);
    }
    e
}

fn mono(mut e: c::Element) -> c::Element {
    fn set(e: &mut c::Element) {
        match e {
            c::Element::Named { content, .. } => set(content),
            c::Element::Text(t) => {
                for r in &mut t.runs {
                    r.style.mono = true
                }
            }
            c::Element::Box(b) => {
                for i in &mut b.children {
                    set(&mut i.element)
                }
            }
            _ => {}
        }
    }
    set(&mut e);
    e
}
fn align_box(mut e: c::Element, align: c::Alignment) -> c::Element {
    match &mut e {
        c::Element::Named { content, .. } => return named("", align_box(*content.clone(), align)),
        c::Element::Box(b) => b.align = align,
        _ => {}
    }
    e
}

/// A 120x16 px, margin-free range control matching the lab's input type=range.
fn slider(fraction: f32) -> c::Element {
    let base = row(
        "",
        plain(Some(16.), [0.; 4], 0.),
        0.,
        vec![fixed(c::Element::Spacer, 120.)],
    );
    let track = row(
        "",
        surface(ui::Color::Accent, 1., 2., [0.; 4], Some(4.)),
        0.,
        vec![fixed(c::Element::Spacer, 120.)],
    );
    let knob = row(
        "",
        surface(ui::Color::Accent, 1., 8., [0.; 4], Some(16.)),
        0.,
        vec![fixed(c::Element::Spacer, 16.)],
    );
    c::Element::Layers {
        base: Box::new(base),
        overlays: vec![
            c::Placed {
                content: track,
                left: 0.,
                bottom: 6.,
            },
            c::Placed {
                content: knob,
                left: 104. * fraction,
                bottom: 0.,
            },
        ],
    }
}
