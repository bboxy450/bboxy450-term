// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public registry presentation. The host supplies observations and owns every effect.
use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Contract {
    pub name: String,
    pub version: String,
    pub ok: Option<bool>,
    pub failing: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Health {
    pub state: String,
    pub say: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Span {
    pub name: String,
    pub start: f64,
    pub ms: f64,
    pub depth: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Call {
    /// Milliseconds before the host's snapshot, not a UI clock or a bus cursor.
    pub t: f64,
    pub method: String,
    pub ms: f64,
    pub hook: Option<String>,
    pub spans: Vec<Span>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IntentPending {
    pub verb: String,
}
/// One grant the trusted shape requests (`payload_events:<event>`,
/// `host_api:<method>`, `block:<fail_open|fail_closed>`), and whether the
/// operator granted it. Toggling it re-trusts with the changed grants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GrantRow {
    pub name: String,
    pub granted: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IntentRefused {
    pub verb: String,
    pub reason: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Plugin {
    #[serde(skip)]
    pub candidate: String,
    #[serde(skip)]
    pub actionable: bool,
    /// Live hosts without a timeline must not present an unmeasured zero.
    #[serde(skip)]
    pub measurements_unavailable: bool,
    pub name: String,
    pub bundled: bool,
    pub contract: Contract,
    pub tier: String,
    pub local: bool,
    pub trust: String,
    pub repo_local: bool,
    pub grant: Option<String>,
    /// What the trusted shape requests, each with the operator's current grant.
    #[serde(skip)]
    pub grants: Vec<GrantRow>,
    pub version: Option<String>,
    pub auth: Option<String>,
    pub health: Health,
    pub enabled: bool,
    pub root: Option<String>,
    pub trace: bool,
    pub calls: Vec<Call>,
    pub alerts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<IntentPending>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<IntentRefused>,
}

const WINDOW_MS: f64 = 60_000.;
fn duration(ms: f64) -> String {
    if ms >= 1000. {
        format!("{:.1} s", ms / 1000.)
    } else {
        format!("{ms} ms")
    }
}
impl Plugin {
    pub fn control_key(&self) -> &str {
        if self.candidate.is_empty() {
            &self.name
        } else {
            &self.candidate
        }
    }
    fn recent(&self) -> impl Iterator<Item = &Call> {
        self.calls.iter().filter(|call| {
            call.t.is_finite()
                && (0. ..=WINDOW_MS).contains(&call.t)
                && call.ms.is_finite()
                && call.ms >= 0.
        })
    }
    /// An active root's provider and an untrusted binary cannot be toggled.
    pub fn can_toggle(&self) -> bool {
        self.trust == "trusted" && self.root.is_none()
    }
    fn summary(&self) -> String {
        let mut samples: Vec<_> = self.recent().map(|call| call.ms).collect();
        samples.sort_by(f64::total_cmp);
        if samples.is_empty() {
            "no calls in the last minute".into()
        } else {
            format!(
                "{} in the last minute · p50 {} · max {}",
                samples.len(),
                duration(samples[samples.len() / 2]),
                duration(*samples.last().unwrap())
            )
        }
    }
}

pub(super) fn chip(b: &Build, key: &str, text: &str, foreground: Option<ui::Color>) -> c::Element {
    row(
        key,
        surface(
            badge(b.ctx, STYLE.tint.badge_alpha as f32),
            1.,
            6.,
            [0., 8., 0., 8.],
            Some(20.),
        ),
        0.,
        vec![
            txt(
                &format!("{key}/text"),
                text,
                font(11., 400, false, foreground.unwrap_or(ui::Color::Foreground)),
            )
            .into(),
        ],
    )
}
fn chip_color(b: &Build, co: &str, fraction: f32) -> ui::Color {
    mixed(b.ctx, co, b.ctx.theme.palette().fg, fraction)
}
fn health(key: &str, state: &str) -> c::Element {
    let co = match state {
        "ok" => color(TOKENS.color.ui.ok),
        "degraded" | "setup" => color(TOKENS.color.state.waiting),
        "crashed" => color(TOKENS.color.state.errored),
        _ => ui::Color::Transparent,
    };
    let mut style = surface(co, 1., 4., [0.; 4], Some(8.));
    if state == "never run" || state.is_empty() {
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(1.5),
            paint: ui::Paint::solid(ui::Color::Muted),
            outside: false,
        });
    }
    row(key, style, 0., vec![])
}
pub(super) fn alert(key: &str, text: &str) -> c::Element {
    col(
        "",
        plain(None, [8., 0., 0., 16.], 0.),
        0.,
        vec![
            col(
                key,
                surface(
                    color(TOKENS.color.state.waiting),
                    0.14,
                    STYLE.radius.row as f32,
                    [6., 10., 6., 10.],
                    None,
                ),
                0.,
                vec![
                    wrap(
                        &format!("{key}/text"),
                        format!("⚠ {text}"),
                        font(12.5, 600, false, ui::Color::Foreground),
                    )
                    .into(),
                ],
            )
            .into(),
        ],
    )
}

fn timeline(b: &Build, key: &str, plugin: &Plugin) -> c::Element {
    let width = (b.width - 16. - 44. - 8.).max(1.);
    let lane = |hooks: bool| {
        let name = format!("{key}/{}", if hooks { "hooks" } else { "calls" });
        let bars = plugin
            .recent()
            .enumerate()
            .filter(|(_, call)| call.hook.is_some() == hooks)
            .map(|(i, call)| {
                let left = ((WINDOW_MS - call.t) / WINDOW_MS) as f32 * width;
                let bar_width = (call.ms.min(call.t) / WINDOW_MS) as f32 * width;
                let co = if call.ms >= 5000. {
                    color(TOKENS.color.state.waiting)
                } else if hooks {
                    color(b.ctx.theme.palette().accent_soft)
                } else {
                    ui::Color::Accent
                };
                let mut style = surface(co, 1., 1.5, [0.; 4], Some(8.));
                style.overrides.width = Some(ui::Length::Px(bar_width.max(2.)));
                c::Placed {
                    content: row(&format!("{name}/bar{i}"), style, 0., vec![]),
                    left,
                    bottom: 2.,
                }
            })
            .collect();
        named(
            &name,
            c::Element::Layers {
                base: Box::new(row(
                    "",
                    surface(ui::Color::Muted, 0.1, 2., [0.; 4], Some(12.)),
                    0.,
                    vec![],
                )),
                overlays: bars,
            },
        )
    };
    let label = |name: &str| {
        let mut style = font(9., 400, false, ui::Color::Muted);
        style.tracking_em = 0.05;
        fixed(txt("", name, style), 12.)
    };
    let plot = row(
        &format!("{key}/plot"),
        plain(None, [0.; 4], 0.),
        8.,
        vec![
            fixed(
                col(
                    "",
                    plain(Some(30.), [0.; 4], 0.),
                    3.,
                    vec![label("CALLS"), label("HOOKS")],
                ),
                44.,
            ),
            fill(col(
                &format!("{key}/lanes"),
                plain(Some(30.), [0.; 4], 0.),
                3.,
                vec![fixed(lane(false), 12.), fixed(lane(true), 12.)],
            )),
        ],
    );
    let mut children = vec![
        plot.into(),
        col(
            "",
            plain(None, [0., 0., 0., 52.], 0.),
            0.,
            vec![
                b.text(
                    &format!("{key}/summary"),
                    plugin.summary(),
                    10.,
                    400,
                    false,
                    true,
                )
                .into(),
            ],
        )
        .into(),
    ];
    if plugin.trace
        && let Some(call) = plugin
            .recent()
            .filter(|call| !call.spans.is_empty())
            .min_by(|a, z| a.t.total_cmp(&z.t))
        && let Some(host) = call.spans.first()
        && host.ms.is_finite()
        && host.ms > 0.
    {
        let spans = call
            .spans
            .iter()
            .enumerate()
            .filter(|(_, span)| {
                span.start.is_finite()
                    && span.ms.is_finite()
                    && span.start >= 0.
                    && span.ms >= 0.
                    && span.depth < 3
            })
            .map(|(i, span)| {
                let span_width = (span.ms / host.ms) as f32 * width;
                let co = mixed(
                    b.ctx,
                    b.ctx.theme.palette().accent,
                    b.ctx.theme.palette().surface,
                    match span.depth {
                        0 => 0.34,
                        1 => 0.22,
                        _ => 0.12,
                    },
                );
                let mut style = border(
                    surface(co, 1., 2., [0., 4., 0., 4.], Some(16.)),
                    hair(b.ctx),
                    false,
                );
                style.overrides.width = Some(ui::Length::Px(span_width.max(0.)));
                let mut text = font(9.5, 400, true, ui::Color::Foreground);
                text.line_height = 16. / 9.5;
                let mut node = c::Box::new(
                    style,
                    vec![fill(txt(
                        "",
                        format!("{} {}", span.name, duration(span.ms)),
                        text,
                    ))],
                );
                node.clip = true;
                c::Placed {
                    content: named(format!("{key}/span{i}"), c::Element::Box(node)),
                    left: (span.start / host.ms) as f32 * width,
                    bottom: 54. - span.depth as f32 * 18. - 16.,
                }
            })
            .collect();
        children.push(
            col(
                "",
                plain(None, [6., 0., 0., 52.], 0.),
                0.,
                vec![
                    named(
                        format!("{key}/flame"),
                        c::Element::Layers {
                            base: Box::new(row("", plain(Some(54.), [0.; 4], 0.), 0., vec![])),
                            overlays: spans,
                        },
                    )
                    .into(),
                ],
            )
            .into(),
        );
    }
    col(
        "",
        plain(None, [10., 0., 0., 16.], 0.),
        0.,
        vec![col(key, plain(None, [0.; 4], 0.), 4., children).into()],
    )
}

impl Settings {
    pub(super) fn plugins(&self, b: &Build) -> c::Element {
        let mut children = vec![b.note("plugins/note", "Binaries in plugins/ under the app support directory; repo-local ones in .bboxy/plugins/ are listed and never run until you trust them. Each speaks one contract; the tier is the data that leaves the machine.", true).into()];
        if let Some(notice) = &self.values.plugin_notice {
            children.push(b.note("plugins/source", notice, true).into());
        }
        for (i, plugin) in self.values.plugins.iter().enumerate() {
            let key = format!("card0/plugin{i}");
            let pending = plugin
                .pending
                .as_ref()
                .map(|pending| match pending.verb.as_str() {
                    "trust" => "trusting…",
                    "enable" => "enabling…",
                    "disable" => "disabling…",
                    _ => "pending…",
                });
            let control = if let Some(root) = &plugin.root {
                b.text(
                    &format!("{key}/control"),
                    format!("provider of {root}"),
                    10.,
                    400,
                    false,
                    true,
                )
            } else if plugin.trust != "trusted" {
                if let Some(pending) = pending {
                    b.text(&format!("{key}/control"), pending, 10., 400, false, true)
                } else {
                    b.button_sized(&format!("{key}/control"), "Trust…", None, "primary", 12.5)
                }
            } else if let Some(pending) = pending {
                row(
                    &format!("{key}/control"),
                    plain(None, [0.; 4], 0.),
                    10.,
                    vec![
                        natural(b.text(&format!("{key}/pending"), pending, 10., 400, false, true)),
                        natural(faded(
                            b.toggle(&format!("{key}/toggle"), plugin.enabled),
                            0.4,
                        )),
                    ],
                )
            } else {
                b.toggle(&format!("{key}/control"), plugin.enabled)
            };
            let control = if self.values.plugin_notice.is_some() && !self.values.plugin_controls {
                faded(control, 0.4)
            } else {
                control
            };
            let header = row(
                &format!("{key}/header"),
                plain(None, [0.; 4], 0.),
                8.,
                vec![
                    fixed(health(&format!("{key}/dot"), &plugin.health.state), 8.),
                    natural(b.text(
                        &format!("{key}/name"),
                        format!(
                            "{}{}",
                            plugin.name,
                            if plugin.bundled { " (bundled)" } else { "" }
                        ),
                        10.,
                        600,
                        true,
                        false,
                    )),
                    natural(b.text(
                        &format!("{key}/version"),
                        plugin.version.as_deref().unwrap_or(""),
                        10.,
                        400,
                        true,
                        true,
                    )),
                    fill(c::Element::Spacer),
                    natural(control),
                ],
            );
            let trusted = plugin.trust == "trusted";
            let mut chips = vec![
                chip(
                    b,
                    &format!("{key}/chip0"),
                    if trusted {
                        "trusted"
                    } else if plugin.repo_local {
                        "repo-local · not trusted"
                    } else {
                        "not trusted"
                    },
                    Some(chip_color(
                        b,
                        if trusted {
                            TOKENS.color.ui.ok
                        } else {
                            TOKENS.color.state.waiting
                        },
                        0.7,
                    )),
                )
                .into(),
                chip(
                    b,
                    &format!("{key}/chip1"),
                    &format!(
                        "tier {}{}",
                        plugin.tier,
                        if plugin.local { " (local)" } else { "" }
                    ),
                    None,
                )
                .into(),
                chip(
                    b,
                    &format!("{key}/chip2"),
                    &format!(
                        "{} {}{}",
                        plugin.contract.name,
                        plugin.contract.version,
                        match plugin.contract.ok {
                            Some(true) => " ✓".into(),
                            Some(false) =>
                                format!(" ✕ {}", plugin.contract.failing.as_deref().unwrap_or("")),
                            None => String::new(),
                        }
                    ),
                    (plugin.contract.ok == Some(false))
                        .then(|| chip_color(b, TOKENS.color.state.errored, 0.75)),
                )
                .into(),
            ];
            if let Some(auth) = &plugin.auth {
                chips.push(
                    chip(
                        b,
                        &format!("{key}/auth"),
                        if auth == "ok" { "logged in" } else { auth },
                        (auth != "ok").then(|| chip_color(b, TOKENS.color.state.waiting, 0.7)),
                    )
                    .into(),
                );
            }
            if let Some(grant) = &plugin.grant {
                chips.push(chip(b, &format!("{key}/grant"), grant, None).into());
            }
            // Requested versus granted: each button grants or revokes one.
            for (j, grant) in plugin.grants.iter().enumerate() {
                let label = if grant.granted {
                    format!("granted {} · revoke", grant.name)
                } else {
                    format!("requested {} · grant", grant.name)
                };
                chips.push(
                    b.button(&format!("{key}/grant{j}"), &label, None, "quiet")
                        .into(),
                );
            }
            if plugin.trace {
                chips.push(chip(b, &format!("{key}/trace"), "trace", None).into());
            }
            chips.push(
                row(
                    "",
                    plain(None, [0., 0., 0., 4.], 0.),
                    0.,
                    vec![
                        b.text(
                            &format!("{key}/health"),
                            format!(
                                "{}{}",
                                plugin.health.state,
                                plugin
                                    .health
                                    .say
                                    .as_ref()
                                    .map_or(String::new(), |s| format!(" · {s}"))
                            ),
                            10.,
                            400,
                            false,
                            true,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
            if plugin.auth.as_deref() == Some("needs login") || plugin.health.state == "crashed" {
                chips.push(
                    b.button(&format!("{key}/setup"), "Re-run setup", None, "quiet")
                        .into(),
                );
            }
            let mut parts = vec![
                header.into(),
                col(
                    "",
                    plain(None, [8., 0., 0., 0.], 0.),
                    0.,
                    vec![
                        col(
                            &format!("{key}/chips"),
                            plain(None, [0., 0., 0., 16.], 0.),
                            0.,
                            vec![flow("", chips, 6., 6., c::Alignment::Center).into()],
                        )
                        .into(),
                    ],
                )
                .into(),
            ];
            parts.extend(
                plugin
                    .alerts
                    .iter()
                    .enumerate()
                    .map(|(j, message)| alert(&format!("{key}/alert{j}"), message).into()),
            );
            if let Some(refused) = &plugin.refused {
                parts.push(
                    alert(
                        &format!("{key}/alert{}", plugin.alerts.len()),
                        &format!("{} refused · {}", refused.verb, refused.reason),
                    )
                    .into(),
                );
            }
            if trusted && plugin.measurements_unavailable {
                parts.push(
                    b.note(
                        &format!("{key}/measurements"),
                        "Call and hook measurements unavailable",
                        true,
                    )
                    .into(),
                );
            } else if trusted {
                parts.push(timeline(b, &format!("{key}/timeline"), plugin).into());
            }
            let mut style = border(plain(None, [12., 0., 14., 0.], 0.), hair(b.ctx), false);
            style.overrides.border_sides = Some([true, false, false, false]);
            children.push(col(&key, style, 0., parts).into());
        }
        if self.values.plugins.is_empty() {
            children.push(
                b.note(
                    "plugins/empty",
                    "Plugin registry is unavailable in the current runtime snapshot",
                    true,
                )
                .into(),
            );
        }
        b.card("card0", "Plugins", children, [4., 14., 10., 14.])
    }
}
