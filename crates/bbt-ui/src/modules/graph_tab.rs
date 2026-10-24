//! Read-only projections of the host's work, provenance and subscription records.
//! No index, grant, mutation or verification authority lives in this module.
use super::{
    seat_row::{named, role_color},
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{Activity, Grade, STYLE, TOKENS, draw::Color as Rgba, layout::Axis};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Artifact {
    pub kind: String,
    pub r#ref: String,
    pub responsible: Option<String>,
    #[serde(rename = "responsibleColor")]
    pub responsible_color: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    pub title: String,
    pub rel: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Work {
    pub title: String,
    #[serde(rename = "status", alias = "state")]
    pub state: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub assignee: Option<String>,
    pub waits: Option<String>,
    pub violates: Option<String>,
    pub constrains: Option<String>,
    pub binds: Vec<Binding>,
    pub artifacts: Vec<Artifact>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Epic {
    pub title: String,
    pub items: Vec<Work>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Seat {
    pub key: String,
    pub name: String,
    pub harness: String,
    pub role: Option<String>,
    pub state: String,
    pub color: Option<String>,
    pub grade: String,
    #[serde(rename = "waitKind")]
    pub wait_kind: Option<String>,
    pub tasks: Vec<Work>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pattern {
    pub pattern: String,
    pub auto: bool,
    pub why: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Subscription {
    pub seat: String,
    pub color: Option<String>,
    pub patterns: Vec<Pattern>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Digest {
    pub content: String,
    pub signed: bool,
    pub by: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProvenanceItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub created_by: String,
    pub updated_by: Option<String>,
    #[serde(rename = "createdColor")]
    pub created_color: Option<String>,
    #[serde(rename = "updatedColor")]
    pub updated_color: Option<String>,
    pub derived_from: Option<String>,
    pub digest: Option<Digest>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LogEntry {
    pub hash: String,
    pub time: String,
    pub text: String,
    pub checkpoint: bool,
    pub by: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Provenance {
    pub items: Vec<ProvenanceItem>,
    pub log: Vec<LogEntry>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    pub time: String,
    pub seat: String,
    #[serde(rename = "seatColor")]
    pub seat_color: Option<String>,
    pub event: String,
    pub kind: String,
    pub r#ref: String,
    pub grade: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Harness {
    pub name: String,
    pub tile: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelFolder {
    pub path: String,
    pub label: String,
    pub depth: usize,
    pub kind: String,
    pub filter: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelSink {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelLink {
    pub from: String,
    pub to: String,
    pub w: Option<f32>,
    pub filter: Option<String>,
    pub rate: Option<String>,
    pub paused: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelBridge {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Channels {
    pub root: Option<String>,
    pub folders: Vec<ChannelFolder>,
    pub sinks: Vec<ChannelSink>,
    pub links: Vec<ChannelLink>,
    pub bridges: Vec<ChannelBridge>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphTab {
    pub harnesses: Vec<Harness>,
    pub seats: Vec<Seat>,
    pub epics: Vec<Epic>,
    pub swarm: String,
    pub history: Vec<History>,
    pub provenance: Option<Provenance>,
    pub subscriptions: Option<Vec<Subscription>>,
    pub channels: Option<Channels>,
    pub only: Option<Vec<String>>,
    pub e2e: Option<super::e2e_checklist::Checklist>,
}

fn color(value: Option<&str>, fallback: ui::Color) -> ui::Color {
    value
        .map(|v| ui::Color::Rgba(Rgba::hex(v).0))
        .unwrap_or(fallback)
}
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn font(size: f32, weight: u16, mono: bool, color: ui::Color) -> ui::Text {
    ui::Text {
        size: Some(size),
        weight,
        mono,
        paint: ui::Paint::solid(color),
        line_height: 1.4,
        ..Default::default()
    }
}
fn label(key: impl Into<String>, value: impl Into<String>, style: ui::Text) -> c::Element {
    let mut t = c::Text::new(value, style);
    t.wrap = false;
    t.ellipsis = true;
    named(key, c::Element::Text(t))
}
fn row(key: impl Into<String>, style: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.gap = ui::Length::Px(gap);
    named(key, c::Element::Box(b))
}
fn column(
    key: impl Into<String>,
    style: ui::Style,
    gap: f32,
    children: Vec<c::Item>,
) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = Axis::Column;
    b.align = c::Alignment::Stretch;
    b.gap = ui::Length::Px(gap);
    named(key, c::Element::Box(b))
}
fn fill(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fill,
    }
}
fn fixed(e: c::Element, w: f32) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fixed(w),
    }
}
fn text(
    key: impl Into<String>,
    value: impl Into<String>,
    size: f32,
    weight: u16,
    mono: bool,
    color: ui::Color,
) -> c::Element {
    label(key, value, font(size, weight, mono, color))
}
fn bottom(mut s: ui::Style, last: bool) -> ui::Style {
    if !last {
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Foreground,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        s.overrides.border_sides = Some([false, false, true, false]);
    }
    s
}
fn hair(ctx: ui::Context) -> ui::Color {
    let mut c = Rgba::hex(ctx.theme.palette().line);
    c.0[3] *= STYLE.material.hairline_alpha as f32;
    ui::Color::Rgba(c.0)
}
fn line_style(ctx: ui::Context, pad: [f32; 4], last: bool) -> ui::Style {
    let mut s = bottom(plain(None, pad, 0.), last);
    if let Some(b) = &mut s.overrides.border {
        b.paint = ui::Paint::solid(hair(ctx));
    }
    s
}
fn upper(mut s: ui::Text, tracking: f32) -> ui::Text {
    s.uppercase = true;
    s.tracking_em = tracking;
    s
}
fn task_color(state: &str) -> ui::Color {
    color(
        Some(match state {
            "running" => return ui::Color::Accent,
            "reported" => TOKENS.color.state.waiting,
            "accepted" | "done" => TOKENS.color.ui.ok,
            "rejected" | "blocked" => TOKENS.color.state.errored,
            _ => return ui::Color::Muted,
        }),
        ui::Color::Muted,
    )
}
fn art_mark(kind: &str) -> &'static str {
    match kind {
        "markdown" => "▤",
        "pr" => "⎇",
        "binary" => "▣",
        "image" => "▧",
        "release" => "◆",
        _ => "▢",
    }
}
fn badge(
    key: &str,
    value: &str,
    style: ui::Text,
    pad: [f32; 4],
    radius: f32,
    bg: Option<ui::Paint>,
    border: Option<ui::Paint>,
) -> c::Element {
    let mut s = plain(None, pad, radius);
    if let Some(bg) = bg {
        s.overrides.fill = Some(bg)
    }
    if let Some(paint) = border {
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint,
            outside: true,
        })
    }
    row(
        key,
        s,
        0.,
        vec![label(format!("{key}/text"), value, style).into()],
    )
}
fn kind(key: &str, value: &str) -> c::Element {
    text(key, value, 12., 400, true, ui::Color::Muted)
}
fn card(ctx: ui::Context, key: &str, title: &str, mut children: Vec<c::Item>) -> c::Element {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.padding = Some([4., 14., 10., 14.].map(ui::Length::Px));
    let head = row(
        format!("{key}/head"),
        plain(None, [8., 0., 4., 0.], 0.),
        10.,
        vec![
            text(
                format!("{key}/title"),
                title,
                12.,
                500,
                false,
                ui::Color::Muted,
            )
            .into(),
        ],
    );
    children.insert(0, head.into());
    let _ = ctx;
    column(key, s, 0., children)
}
fn grid_row(
    key: &str,
    style: ui::Style,
    gap: f32,
    cols: Vec<(c::Element, c::Extent)>,
) -> c::Element {
    let mut b = c::Box::new(
        style,
        cols.into_iter()
            .map(|(element, extent)| c::Item { element, extent })
            .collect(),
    );
    b.gap = ui::Length::Px(gap);
    named(key, c::Element::Box(b))
}
fn fraction(w: f32) -> c::Extent {
    c::Extent::Flex(w)
}
fn icon(key: &str, name: &str, size: f32, col: ui::Color) -> c::Element {
    named(
        key,
        c::Element::Icon {
            name: name.into(),
            paint: ui::Paint::solid(col),
            size: ui::Length::Px(size),
        },
    )
}

impl GraphTab {
    fn agents(&self, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
        let key = "agents";
        let mut children = vec![];
        children.push(
            grid_row(
                "agents/header",
                line_style(ctx, [7., 0., 7., 0.], self.seats.is_empty()),
                12.,
                ["seat", "role", "now", "blocked on"]
                    .into_iter()
                    .zip([1.4, 0.8, 1.6, 1.])
                    .enumerate()
                    .map(|(i, (v, w))| {
                        (
                            label(
                                format!("agents/header/c{i}"),
                                v,
                                upper(font(10., 600, false, ui::Color::Muted), 0.08),
                            ),
                            fraction(w),
                        )
                    })
                    .collect(),
            )
            .into(),
        );
        for (i, s) in self.seats.iter().enumerate() {
            let k = format!("agents/r{i}");
            let h = bbt_ui_model::SeatColor::Derived.resolve(&s.key, ctx.theme);
            let col = color(
                s.color.as_deref(),
                ui::Color::Rgba(Rgba::hsl(h.hue, h.saturation / 100., h.lightness / 100.).0),
            );
            let mut tile = plain(Some(16.), [0.; 4], STYLE.radius.tag as f32);
            tile.overrides.fill = Some(ui::Paint::solid(color(
                self.harnesses
                    .iter()
                    .find(|h| h.name == s.harness)
                    .and_then(|h| h.tile.as_deref()),
                ui::Color::Surface2,
            )));
            let tile = row(
                format!("{k}/tile"),
                tile,
                0.,
                vec![c::Item {
                    element: icon(&format!("{k}/hicon"), &s.harness, 10., ui::Color::OnAccent),
                    extent: c::Extent::Fill,
                }],
            );
            let activity = match s.state.as_str() {
                "active" => Activity::Active,
                "waiting" => Activity::Waiting(match s.wait_kind.as_deref() {
                    Some("permission") => bbt_ui_model::WaitKind::Permission,
                    Some("auth") => bbt_ui_model::WaitKind::Auth,
                    _ => bbt_ui_model::WaitKind::Question,
                }),
                "errored" => Activity::Errored,
                _ => Activity::Idle,
            };
            let glyph = named(
                format!("{k}/glyph"),
                c::Element::Glyph {
                    kind_colors: Default::default(),
                    shadow: None,
                    activity,
                    grade: match s.grade.as_str() {
                        "shim" => Grade::Shim,
                        "inferred" => Grade::Inferred,
                        _ => Grade::Reported,
                    },
                    color: ui::Paint::solid(col),
                    size: ctx.density.tokens().glyph as f32,
                    scale: 1.,
                },
            );
            let seat = row(
                format!("{k}/seat"),
                plain(None, [0.; 4], 0.),
                8.,
                vec![
                    fixed(tile, 16.),
                    text(format!("{k}/name"), &s.name, 12.5, 700, false, col).into(),
                    glyph.into(),
                ],
            );
            let role = s.role.as_deref().unwrap_or("shell");
            let mark = match role {
                "oracle" => "♔",
                "executor" => "♟",
                "creativity" => "♝",
                _ => "",
            };
            let mut role_t = c::Text::new("", font(12.5, 400, false, ui::Color::Foreground));
            role_t.runs = vec![
                c::TextRun {
                    value: mark.into(),
                    style: ui::Text {
                        paint: paint(role_color(role), 0.9),
                        ..font(
                            ctx.density.tokens().glyph as f32 - 1.,
                            400,
                            true,
                            role_color(role),
                        )
                    },
                },
                c::TextRun {
                    value: format!(" {role}"),
                    style: font(12.5, 400, false, ui::Color::Foreground),
                },
            ];
            role_t.wrap = false;
            let live = s
                .tasks
                .iter()
                .find(|t| ["claimed", "running", "reported"].contains(&t.state.as_str()));
            let now = if let Some(t) = live {
                let mut inline = c::Flow::new(vec![
                    named(
                        format!("{k}/now/mark"),
                        ui::marks::Mark::task(&t.state)
                            .element(11., ui::Paint::solid(task_color(&t.state))),
                    )
                    .into(),
                    text(
                        format!("{k}/now/title"),
                        &t.title,
                        12.5,
                        400,
                        false,
                        ui::Color::Foreground,
                    )
                    .into(),
                ]);
                inline.wrap = false;
                inline.align = c::Alignment::Baseline;
                inline.column_gap = ui::Length::Px(m.width(
                    " ",
                    &bbt_ui_model::layout::TextStyle {
                        size: 12.5,
                        weight: 400,
                        mono: false,
                        italic: false,
                        tracking_em: 0.,
                    },
                ));
                inline.row_gap = ui::Length::Px(0.);
                named(format!("{k}/now"), c::Element::Flow(inline))
            } else {
                text(
                    format!("{k}/now"),
                    if s.state == "waiting" {
                        format!("waiting · {}", s.wait_kind.as_deref().unwrap_or("answer"))
                    } else {
                        s.state.clone()
                    },
                    12.5,
                    400,
                    false,
                    ui::Color::Foreground,
                )
            };
            let blocked = s
                .tasks
                .iter()
                .find(|t| t.state == "blocked")
                .map(|t| format!("waits on {}", t.waits.as_deref().unwrap_or("")))
                .unwrap_or_else(|| {
                    if s.state == "waiting" {
                        "the operator".into()
                    } else {
                        "—".into()
                    }
                });
            children.push(
                grid_row(
                    &k,
                    line_style(ctx, [7., 0., 7., 0.], i + 1 == self.seats.len()),
                    12.,
                    vec![
                        (seat, fraction(1.4)),
                        (
                            named(format!("{k}/role"), c::Element::Text(role_t)),
                            fraction(0.8),
                        ),
                        (now, fraction(1.6)),
                        (
                            text(
                                format!("{k}/blocked"),
                                blocked,
                                12.5,
                                400,
                                false,
                                ui::Color::Muted,
                            ),
                            fraction(1.),
                        ),
                    ],
                )
                .into(),
            );
        }
        card(ctx, key, &format!("Agents · {}", self.swarm), children)
    }
    fn work(&self, ctx: ui::Context) -> c::Element {
        let mut rows = vec![];
        for (ei, ep) in self.epics.iter().enumerate() {
            let ek = format!("work/e{ei}");
            let done = ep
                .items
                .iter()
                .filter(|i| ["accepted", "done"].contains(&i.state.as_str()))
                .count();
            rows.push(
                row(
                    &ek,
                    plain(None, [6., 0., 2., 0.], 0.),
                    8.,
                    vec![
                        text(
                            format!("{ek}/kind"),
                            "epic",
                            12.,
                            400,
                            false,
                            ui::Color::Foreground,
                        )
                        .into(),
                        text(
                            format!("{ek}/title"),
                            &ep.title,
                            13.,
                            700,
                            false,
                            ui::Color::Foreground,
                        )
                        .into(),
                        text(
                            format!("{ek}/count"),
                            format!("{done}/{}", ep.items.len()),
                            12.,
                            400,
                            false,
                            ui::Color::Foreground,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
            for (wi, it) in ep.items.iter().enumerate() {
                let k = format!("work/e{ei}/i{wi}");
                let last = wi + 1 == ep.items.len();
                let kind = if it.kind.is_empty() { "task" } else { &it.kind };
                let col = if kind == "invariant" {
                    color(Some(TOKENS.color.ui.invariant), ui::Color::Muted)
                } else if kind == "bug" {
                    color(
                        Some(if it.state == "fixed" {
                            TOKENS.color.state.idle
                        } else {
                            TOKENS.color.state.errored
                        }),
                        ui::Color::Muted,
                    )
                } else {
                    task_color(&it.state)
                };
                let mark = if kind == "bug" {
                    icon(&format!("{k}/mark"), "bug", 11., col)
                } else if kind == "invariant" {
                    text(format!("{k}/mark"), "◆", 12., 400, true, col)
                } else {
                    named(
                        format!("{k}/mark"),
                        ui::marks::Mark::task(&it.state).element(11., ui::Paint::solid(col)),
                    )
                };
                let mut cells = vec![
                    text(
                        format!("{k}/branch"),
                        if last { "└─" } else { "├─" },
                        12.,
                        400,
                        true,
                        ui::Color::Line,
                    )
                    .into(),
                    mark.into(),
                    kind_label(
                        &format!("{k}/kind"),
                        if kind == "invariant" { "inv" } else { kind },
                    )
                    .into(),
                    text(
                        format!("{k}/title"),
                        &it.title,
                        12.,
                        if kind == "invariant" { 600 } else { 400 },
                        false,
                        ui::Color::Foreground,
                    )
                    .into(),
                ];
                if let Some(title) = &it.violates {
                    cells.push(invariant(&format!("{k}/invariant"), title, true).into())
                }
                if let Some(title) = &it.constrains {
                    cells.push(invariant(&format!("{k}/invariant"), title, false).into())
                }
                if kind != "invariant" {
                    if let Some(name) = &it.assignee {
                        cells.push(
                            text(
                                format!("{k}/assignee"),
                                format!("· {name}"),
                                12.,
                                400,
                                true,
                                ui::Color::Muted,
                            )
                            .into(),
                        )
                    }
                    if let Some(wait) = &it.waits {
                        cells.push(
                            text(
                                format!("{k}/waits"),
                                format!("◀ waits on {wait}"),
                                11.,
                                400,
                                true,
                                color(Some(TOKENS.color.state.errored), ui::Color::Muted),
                            )
                            .into(),
                        )
                    }
                }
                rows.push(row(&k, plain(None, [0., 0., 0., 8.], 0.), 8., cells).into());
                for (bi, b) in it.binds.iter().enumerate() {
                    let bkey = format!("{k}/b{bi}");
                    rows.push(
                        row(
                            &bkey,
                            plain(None, [0., 0., 0., 8.], 0.),
                            6.,
                            vec![
                                text(
                                    format!("{bkey}/branch"),
                                    format!("{}└─", if last { " " } else { "│ " }),
                                    11.,
                                    400,
                                    true,
                                    ui::Color::Line,
                                )
                                .into(),
                                if b.rel == "violated-by" {
                                    inline_icon(
                                        &format!("{bkey}/mark"),
                                        "bug",
                                        11.,
                                        11.,
                                        color(Some(TOKENS.color.state.errored), ui::Color::Muted),
                                    )
                                } else {
                                    text(
                                        format!("{bkey}/mark"),
                                        "○",
                                        11.,
                                        400,
                                        true,
                                        ui::Color::Muted,
                                    )
                                }
                                .into(),
                                text(
                                    format!("{bkey}/title"),
                                    &b.title,
                                    11.,
                                    400,
                                    true,
                                    ui::Color::Foreground,
                                )
                                .into(),
                                text(
                                    format!("{bkey}/rel"),
                                    &b.rel,
                                    9.5,
                                    400,
                                    true,
                                    if b.rel == "violated-by" {
                                        color(Some(TOKENS.color.state.errored), ui::Color::Muted)
                                    } else {
                                        ui::Color::Muted
                                    },
                                )
                                .into(),
                            ],
                        )
                        .into(),
                    );
                }
                for (ai, a) in it.artifacts.iter().enumerate() {
                    let ak = format!("{k}/a{ai}");
                    let mut fields = vec![
                        text(
                            format!("{ak}/branch"),
                            format!("{}└─", if last { "   " } else { "│  " }),
                            12.,
                            400,
                            true,
                            ui::Color::Line,
                        )
                        .into(),
                        fixed(
                            text(
                                format!("{ak}/mark"),
                                art_mark(&a.kind),
                                12.,
                                400,
                                true,
                                ui::Color::Muted,
                            ),
                            12.,
                        ),
                        kind_label(&format!("{ak}/kind"), &a.kind).into(),
                        text(
                            format!("{ak}/ref"),
                            &a.r#ref,
                            11.,
                            400,
                            true,
                            ui::Color::Foreground,
                        )
                        .into(),
                    ];
                    if let Some(name) = &a.responsible {
                        fields.push(
                            badge(
                                &format!("{ak}/responsible"),
                                name.split(' ').next().unwrap_or(name),
                                font(
                                    9.5,
                                    600,
                                    false,
                                    color(a.responsible_color.as_deref(), ui::Color::Foreground),
                                ),
                                [0., 6., 0., 6.],
                                999.,
                                None,
                                Some(ui::Paint::solid(color(
                                    a.responsible_color.as_deref(),
                                    ui::Color::Line,
                                ))),
                            )
                            .into(),
                        );
                    }
                    rows.push(row(&ak, plain(None, [0., 0., 0., 8.], 0.), 8., fields).into());
                }
            }
        }
        card(
            ctx,
            "work",
            "Graph · epics → work",
            vec![column("work/tree", plain(None, [4., 0., 6., 0.], 0.), 3., rows).into()],
        )
    }
    fn subscriptions(&self, ctx: ui::Context, subs: &[Subscription]) -> c::Element {
        let mut children = vec![];
        for (i, s) in subs.iter().enumerate() {
            let k = format!("subscriptions/r{i}");
            let mut pats = vec![];
            for (pi, p) in s.patterns.iter().enumerate() {
                let pk = format!("{k}/p{pi}");
                let mut st = plain(None, [1., 6., 1., 6.], STYLE.radius.tag as f32);
                st.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: ui::Paint::solid(hair(ctx)),
                    outside: true,
                });
                if p.auto {
                    st.overrides.fill = Some(paint(ui::Color::Accent, 0.08))
                }
                let mut cells = vec![
                    text(
                        format!("{pk}/pattern"),
                        &p.pattern,
                        10.,
                        400,
                        true,
                        ui::Color::Foreground,
                    )
                    .into(),
                ];
                if p.auto {
                    cells.push(
                        label(
                            format!("{pk}/auto"),
                            "auto",
                            upper(
                                font(STYLE.r#type.micro as f32, 400, false, ui::Color::Muted),
                                0.06,
                            ),
                        )
                        .into(),
                    )
                }
                pats.push(row(&pk, st, 4., cells).into());
            }
            let mut flow = c::Flow::new(pats);
            flow.row_gap = ui::Length::Px(4.);
            flow.column_gap = ui::Length::Px(4.);
            flow.align = c::Alignment::Center;
            children.push(
                grid_row(
                    &k,
                    line_style(ctx, [6., 0., 6., 0.], i + 1 == subs.len()),
                    10.,
                    vec![
                        (
                            text(
                                format!("{k}/seat"),
                                &s.seat,
                                11.5,
                                600,
                                false,
                                color(s.color.as_deref(), ui::Color::Foreground),
                            ),
                            c::Extent::Fixed(150.),
                        ),
                        (
                            named(format!("{k}/patterns"), c::Element::Flow(flow)),
                            c::Extent::Fill,
                        ),
                    ],
                )
                .into(),
            );
        }
        card(
            ctx,
            "subscriptions",
            &format!("Subscriptions · {} seats", subs.len()),
            children,
        )
    }
    fn history(&self, ctx: ui::Context) -> c::Element {
        let mut children = vec![];
        for (i, ev) in self.history.iter().rev().take(12).enumerate() {
            let k = format!("history/r{i}");
            let event_color = match ev.event.as_str() {
                "claimed" => color(Some(TOKENS.color.ui.ok), ui::Color::Muted),
                "created" => ui::Color::Accent,
                _ => ui::Color::Muted,
            };
            children.push(
                grid_row(
                    &k,
                    line_style(ctx, [5., 0., 5., 0.], i + 1 == self.history.len().min(12)),
                    8.,
                    vec![
                        (
                            text(
                                format!("{k}/time"),
                                &ev.time,
                                10.,
                                400,
                                true,
                                ui::Color::Muted,
                            ),
                            c::Extent::Fixed(44.),
                        ),
                        (
                            text(
                                format!("{k}/seat"),
                                &ev.seat,
                                11.5,
                                600,
                                false,
                                color(ev.seat_color.as_deref(), ui::Color::Foreground),
                            ),
                            fraction(1.1),
                        ),
                        (
                            text(format!("{k}/event"), &ev.event, 10., 400, true, event_color),
                            c::Extent::Fixed(72.),
                        ),
                        (
                            text(
                                format!("{k}/mark"),
                                art_mark(&ev.kind),
                                11.5,
                                400,
                                false,
                                ui::Color::Foreground,
                            ),
                            c::Extent::Fixed(14.),
                        ),
                        (
                            text(
                                format!("{k}/ref"),
                                &ev.r#ref,
                                10.5,
                                400,
                                true,
                                ui::Color::Foreground,
                            ),
                            fraction(2.2),
                        ),
                        (
                            text(
                                format!("{k}/grade"),
                                if ev.grade.is_empty() {
                                    "inferred"
                                } else {
                                    &ev.grade
                                },
                                9.5,
                                400,
                                true,
                                if ev.grade == "reported" {
                                    ui::Color::Foreground
                                } else {
                                    ui::Color::Muted
                                },
                            ),
                            c::Extent::Fixed(70.),
                        ),
                    ],
                )
                .into(),
            );
        }
        card(
            ctx,
            "history",
            &format!("Artifact history · {}", self.history.len()),
            children,
        )
    }
    fn provenance(&self, ctx: ui::Context, p: &Provenance, m: &dyn c::Measure) -> c::Element {
        let mut children = vec![];
        for (i, it) in p.items.iter().take(6).enumerate() {
            let k = format!("provenance/i{i}");
            let mut by = c::Text::new("", font(11., 400, false, ui::Color::Muted));
            by.wrap = false;
            by.runs = vec![
                c::TextRun {
                    value: "created by ".into(),
                    style: font(11., 400, false, ui::Color::Muted),
                },
                c::TextRun {
                    value: it.created_by.clone(),
                    style: font(
                        11.,
                        400,
                        false,
                        color(it.created_color.as_deref(), ui::Color::Foreground),
                    ),
                },
            ];
            if let Some(updated) = it.updated_by.as_ref().filter(|v| **v != it.created_by) {
                by.runs.extend([
                    c::TextRun {
                        value: " · updated by ".into(),
                        style: font(11., 400, false, ui::Color::Muted),
                    },
                    c::TextRun {
                        value: updated.clone(),
                        style: font(
                            11.,
                            400,
                            false,
                            color(it.updated_color.as_deref(), ui::Color::Foreground),
                        ),
                    },
                ]);
            }
            let mut lines = vec![
                row(
                    format!("{k}/head"),
                    plain(None, [0.; 4], 0.),
                    8.,
                    vec![
                        text(
                            format!("{k}/kind"),
                            if it.kind.is_empty() { "note" } else { &it.kind },
                            11.5,
                            400,
                            false,
                            ui::Color::Foreground,
                        )
                        .into(),
                        text(
                            format!("{k}/title"),
                            &it.title,
                            12.5,
                            700,
                            false,
                            ui::Color::Foreground,
                        )
                        .into(),
                        fill(row(
                            "",
                            plain(None, [0.; 4], 0.),
                            0.,
                            vec![
                                fill(c::Element::Spacer),
                                named(format!("{k}/by"), c::Element::Text(by)).into(),
                            ],
                        )),
                    ],
                )
                .into(),
            ];
            if let Some(derived) = &it.derived_from {
                let links = links(&format!("{k}/links"), derived, ctx, m);
                lines.push(
                    row(
                        format!("{k}/derived"),
                        plain(None, [0.; 4], 0.),
                        8.,
                        vec![
                            fixed(
                                label(
                                    format!("{k}/derived/label"),
                                    "derived from",
                                    upper(font(10., 400, false, ui::Color::Muted), 0.06),
                                ),
                                86.,
                            ),
                            links.into(),
                        ],
                    )
                    .into(),
                );
            }
            if let Some(d) = &it.digest {
                lines.push(
                    row(
                        format!("{k}/digest"),
                        plain(None, [0.; 4], 0.),
                        8.,
                        vec![
                            fixed(
                                label(
                                    format!("{k}/digest/label"),
                                    "digest",
                                    upper(font(10., 400, false, ui::Color::Muted), 0.06),
                                ),
                                86.,
                            ),
                            text(
                                format!("{k}/digest/content"),
                                d.content.chars().take(12).collect::<String>(),
                                10.5,
                                400,
                                true,
                                ui::Color::Foreground,
                            )
                            .into(),
                            text(
                                format!("{k}/digest/signature"),
                                if d.signed {
                                    format!("✓ signed · {}", d.by)
                                } else {
                                    "unsigned".into()
                                },
                                10.,
                                400,
                                true,
                                if d.signed {
                                    color(Some(TOKENS.color.ui.ok), ui::Color::Muted)
                                } else {
                                    ui::Color::Muted
                                },
                            )
                            .into(),
                        ],
                    )
                    .into(),
                );
            }
            children.push(column(&k, line_style(ctx, [7., 0., 7., 0.], false), 4., lines).into());
        }
        if !p.log.is_empty() {
            children.push(
                row(
                    "provenance/loghead",
                    plain(None, [10., 0., 4., 0.], 0.),
                    0.,
                    vec![
                        label(
                            "provenance/logtitle",
                            "activity log · .bboxy/log/ · hash-chained",
                            upper(font(10., 400, false, ui::Color::Muted), 0.06),
                        )
                        .into(),
                    ],
                )
                .into(),
            );
            for (i, ev) in p.log.iter().rev().take(6).enumerate() {
                let k = format!("provenance/log{i}");
                children.push(
                    grid_row(
                        &k,
                        plain(None, [3., 0., 3., 0.], 0.),
                        8.,
                        vec![
                            (
                                text(
                                    format!("{k}/hash"),
                                    ev.hash.chars().take(7).collect::<String>(),
                                    10.,
                                    400,
                                    true,
                                    ui::Color::Foreground,
                                ),
                                c::Extent::Fixed(56.),
                            ),
                            (
                                text(
                                    format!("{k}/time"),
                                    &ev.time,
                                    10.,
                                    400,
                                    true,
                                    ui::Color::Muted,
                                ),
                                c::Extent::Fixed(44.),
                            ),
                            (
                                text(
                                    format!("{k}/text"),
                                    if ev.checkpoint {
                                        format!("✓ daily checkpoint · signed by {}", ev.by)
                                    } else {
                                        ev.text.clone()
                                    },
                                    11.,
                                    400,
                                    false,
                                    if ev.checkpoint {
                                        color(Some(TOKENS.color.ui.ok), ui::Color::Muted)
                                    } else {
                                        ui::Color::Foreground
                                    },
                                ),
                                c::Extent::Fill,
                            ),
                        ],
                    )
                    .into(),
                )
            }
        }
        card(
            ctx,
            "provenance",
            &format!("Provenance · {} items · log {}", p.items.len(), p.log.len()),
            children,
        )
    }
    pub fn card_element(&self, ctx: ui::Context, m: &dyn c::Measure, which: &str) -> c::Element {
        self.card_element_width(ctx, m, which, 960.)
    }
    pub fn card_element_width(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        which: &str,
        width: f32,
    ) -> c::Element {
        match which {
            "e2e" => self
                .e2e
                .as_ref()
                .unwrap_or(&super::e2e_checklist::Checklist::default())
                .element_width(ctx, m, width),
            "provenance" => self.provenance(
                ctx,
                self.provenance.as_ref().unwrap_or(&Provenance::default()),
                m,
            ),
            "subscriptions" => {
                self.subscriptions(ctx, self.subscriptions.as_deref().unwrap_or(&[]))
            }
            _ => self.element(ctx, m),
        }
    }
    pub fn element(&self, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
        self.element_width(ctx, m, 960.)
    }
    /// Responsive modules receive only their enclosing viewport width.
    pub fn element_width(&self, ctx: ui::Context, m: &dyn c::Measure, width: f32) -> c::Element {
        let want = |k: &str| self.only.as_ref().is_none_or(|v| v.iter().any(|s| s == k));
        let mut children = vec![];
        if want("e2e")
            && let Some(e2e) = &self.e2e
        {
            children.push(e2e.element_width(ctx, m, (width - 32.).max(0.)).into());
        }
        if want("agents") {
            children.push(self.agents(ctx, m).into())
        }
        if want("work") {
            children.push(self.work(ctx).into())
        }
        if want("subscriptions")
            && let Some(s) = &self.subscriptions
        {
            children.push(self.subscriptions(ctx, s).into())
        }
        if want("channels")
            && let Some(g) = &self.channels
        {
            children.push(
                self.channels_card(ctx, m, g, (width - 32. - 28.).max(0.))
                    .into(),
            );
        }
        if want("provenance")
            && let Some(p) = &self.provenance
        {
            children.push(self.provenance(ctx, p, m).into())
        }
        if want("history") && !self.history.is_empty() {
            children.push(self.history(ctx).into())
        }
        let mut s = plain(None, [14., 16., 24., 16.], 0.);
        s.overrides.fill = Some(ui::Paint::solid(ui::Color::Background));
        column("graph", s, 14., children)
    }

    fn channels_card(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        g: &Channels,
        width: f32,
    ) -> c::Element {
        // The public 1:1 channel-tree layout. Native font metrics determine
        // its width; the enclosing card scrolls instead of scaling text.
        let sink_gap = 22_f32.max(g.folders.len() as f32 * 22. / g.sinks.len().max(1) as f32);
        let h = (g.folders.len() as f32 * 22.).max(g.sinks.len() as f32 * sink_gap) + 24.;
        let fy = |path: &str| {
            g.folders
                .iter()
                .position(|f| f.path == path)
                .map(|i| 12. + i as f32 * 22. + 11.)
        };
        let sy = |id: &str| {
            g.sinks
                .iter()
                .position(|s| s.id == id)
                .map(|i| 12. + i as f32 * sink_gap + sink_gap * 0.5)
        };
        let fx = |f: &ChannelFolder| 12. + f.depth as f32 * 14.;
        let caption = STYLE.r#type.caption as f32;
        let micro = STYLE.r#type.micro as f32;
        let tiny = STYLE.r#type.tiny as f32;
        let measure = |value: &str, size: f32, weight: u16, mono: bool| {
            m.width(
                value,
                &bbt_ui_model::layout::TextStyle {
                    size,
                    weight,
                    mono,
                    italic: false,
                    tracking_em: 0.,
                },
            )
        };
        let fend = |f: &ChannelFolder| {
            fx(f)
                + measure(&f.label, caption, 400, true)
                + f.filter
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map_or(0., |s| 8. + measure(s, micro, 400, false))
        };
        let bridge_x = |br: &ChannelBridge| match (
            g.folders.iter().find(|f| f.path == br.from),
            g.folders.iter().find(|f| f.path == br.to),
        ) {
            (Some(a), Some(b)) => fend(a).max(fend(b)) + 14.,
            _ => 0.,
        };
        let left = g
            .folders
            .iter()
            .map(fend)
            .chain(
                g.bridges
                    .iter()
                    .map(|br| bridge_x(br) + 28. + measure("bridge", tiny, 400, true)),
            )
            .fold(0., f32::max);
        let sink_x = (left + 220.).round();
        let sink_width = g
            .sinks
            .iter()
            .map(|s| {
                measure(&s.name, caption, 600, false).max(measure(
                    &format!("seats/{}/inbox", s.id),
                    micro,
                    400,
                    true,
                ))
            })
            .fold(0., f32::max);
        let canvas_w = (sink_x + 10. + sink_width + 12.).ceil();
        let h = h.ceil();
        let mut over = vec![];
        let mut add = |element: c::Element, x: f32, top: f32| {
            let (size, _) = element
                .natural(f32::INFINITY, ctx, m)
                .expect("channel element geometry");
            over.push(c::Placed {
                content: element,
                left: x,
                bottom: h - top - size[1],
            });
        };
        for (i, l) in g.links.iter().enumerate() {
            let Some(f) = g.folders.iter().find(|f| f.path == l.from) else {
                continue;
            };
            let (Some(y1), Some(y2)) = (fy(&l.from), sy(&l.to)) else {
                continue;
            };
            let x1 = fend(f) + 6.;
            let x2 = sink_x - 8.;
            let mx = (x1 + x2) * 0.5;
            let points = vec![
                c::PathCommand::Move([x1, y1]),
                c::PathCommand::Cubic([[mx, y1], [mx, y2], [x2, y2]]),
            ];
            add(
                channel_path(
                    format!("channels/link{i}"),
                    [canvas_w, h],
                    points.clone(),
                    1.,
                    ui::Color::Muted,
                    if l.paused { 0.6 } else { 0.45 },
                    if l.paused { vec![1., 4.] } else { vec![3., 3.] },
                    c::LineCap::Butt,
                ),
                0.,
                0.,
            );
            if let Some(w) = l.w.filter(|w| *w != 0.) {
                let col = color(
                    g.sinks
                        .iter()
                        .find(|s| s.id == l.to)
                        .and_then(|s| s.color.as_deref()),
                    ui::Color::Accent,
                );
                add(
                    channel_path(
                        format!("channels/traffic{i}"),
                        [canvas_w, h],
                        points,
                        1. + w * 5.,
                        col,
                        0.45,
                        vec![],
                        c::LineCap::Round,
                    ),
                    0.,
                    0.,
                );
            }
            let caption = [l.filter.as_deref(), l.rate.as_deref()]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            if !caption.is_empty() {
                let w = measure(&caption, tiny, 400, true) + 12.;
                let u = 0.3_f32;
                let v = 1. - u;
                let cx =
                    v * v * v * x1 + 3. * v * v * u * mx + 3. * v * u * u * mx + u * u * u * x2;
                let cy = y1 + (y2 - y1) * (3. * u * u - 2. * u * u * u);
                let mut s = plain(Some(14.), [0.; 4], 7.);
                s.overrides.width = Some(ui::Length::Px(w));
                s.overrides.fill = Some(ui::Paint::solid(ui::Color::Background));
                s.overrides.border = Some(ui::Border {
                    width: ui::Length::Px(1.),
                    paint: ui::Paint::solid(hair(ctx)),
                    outside: false,
                });
                add(
                    named(
                        format!("channels/pill{i}"),
                        c::Element::Box(c::Box::new(s, vec![])),
                    ),
                    cx - w * 0.5,
                    cy - 7.,
                );
                let (element, x, top) = channel_text(
                    format!("channels/pill{i}/text"),
                    &caption,
                    font(tiny, 400, true, ui::Color::Muted),
                    cx,
                    cy,
                    true,
                    ctx,
                    m,
                );
                add(element, x, top);
            }
        }
        for (i, br) in g.bridges.iter().enumerate() {
            let (Some(a), Some(b)) = (
                g.folders.iter().find(|f| f.path == br.from),
                g.folders.iter().find(|f| f.path == br.to),
            ) else {
                continue;
            };
            let (y1, y2) = (fy(&br.from).unwrap(), fy(&br.to).unwrap());
            let x = bridge_x(br);
            add(
                channel_path(
                    format!("channels/bridge{i}"),
                    [canvas_w, h],
                    vec![
                        c::PathCommand::Move([fend(a) + 4., y1]),
                        c::PathCommand::Cubic([[x + 20., y1], [x + 20., y2], [fend(b) + 4., y2]]),
                    ],
                    1.2,
                    ui::Color::Accent,
                    1.,
                    vec![5., 3.],
                    c::LineCap::Butt,
                ),
                0.,
                0.,
            );
            let (el, x, top) = channel_text(
                format!("channels/bridge{i}/text"),
                "bridge",
                font(tiny, 400, true, ui::Color::Muted),
                x + 24.,
                (y1 + y2) * 0.5,
                false,
                ctx,
                m,
            );
            add(el, x, top);
        }
        for (i, f) in g.folders.iter().enumerate() {
            let y = 12. + i as f32 * 22. + 11.;
            let x = fx(f);
            if f.depth > 0 {
                add(
                    channel_path(
                        format!("channels/branch{i}"),
                        [canvas_w, h],
                        vec![
                            c::PathCommand::Move([x - 8.5, y - 22. + 7.]),
                            c::PathCommand::Line([x - 8.5, y + 0.5]),
                            c::PathCommand::Line([x - 3., y + 0.5]),
                        ],
                        1.,
                        ui::Color::Line,
                        1.,
                        vec![],
                        c::LineCap::Butt,
                    ),
                    0.,
                    0.,
                );
            }
            let col = match f.kind.as_str() {
                "group" => ui::Color::Muted,
                "ext" => color(Some("#FF7A45"), ui::Color::Foreground),
                _ => ui::Color::Foreground,
            };
            let (el, x, top) = channel_text(
                format!("channels/folder{i}"),
                &f.label,
                font(caption, 400, true, col),
                x,
                y,
                false,
                ctx,
                m,
            );
            add(el, x, top);
            if let Some(filter) = f.filter.as_deref().filter(|s| !s.is_empty()) {
                let (el, x, top) = channel_text(
                    format!("channels/filter{i}"),
                    filter,
                    font(micro, 400, false, ui::Color::Muted),
                    fx(f) + measure(&f.label, caption, 400, true) + 8.,
                    y,
                    false,
                    ctx,
                    m,
                );
                add(el, x, top);
            }
        }
        for (i, sink) in g.sinks.iter().enumerate() {
            let y = 12. + i as f32 * sink_gap + sink_gap * 0.5;
            let col = color(sink.color.as_deref(), ui::Color::Foreground);
            let mut s = plain(Some(7.), [0.; 4], 3.5);
            s.overrides.width = Some(ui::Length::Px(7.));
            s.overrides.fill = Some(ui::Paint::solid(col));
            add(
                named(
                    format!("channels/sink{i}/dot"),
                    c::Element::Box(c::Box::new(s, vec![])),
                ),
                sink_x - 3.5,
                y - 3.5,
            );
            for (suffix, value, size, weight, mono, col, y) in [
                ("name", sink.name.clone(), caption, 600, false, col, y - 6.),
                (
                    "path",
                    format!("seats/{}/inbox", sink.id),
                    micro,
                    400,
                    true,
                    ui::Color::Muted,
                    y + 6.,
                ),
            ] {
                let (el, x, top) = channel_text(
                    format!("channels/sink{i}/{suffix}"),
                    &value,
                    font(size, weight, mono, col),
                    sink_x + 10.,
                    y,
                    false,
                    ctx,
                    m,
                );
                add(el, x, top);
            }
        }
        let base = c::Element::Box(c::Box::new(plain(Some(h), [0.; 4], 0.), vec![]));
        let tree = named(
            "channels/tree",
            c::Element::Layers {
                base: Box::new(base),
                overlays: over,
            },
        );
        let content = row(
            "",
            plain(Some(h), [0.; 4], 0.),
            0.,
            vec![fixed(tree, canvas_w)],
        );
        let viewport = c::Element::ScrollView(c::ScrollView {
            viewport: [width, h],
            offset: [0.; 2],
            content_width: Some(canvas_w),
            content: Box::new(content),
        });
        let body = column(
            "channels/wrap",
            plain(None, [4., 0., 4., 0.], 0.),
            0.,
            vec![viewport.into()],
        );
        let mut legend = vec![];
        for (name, weight, col, alpha, dash, cap) in [
            (
                "link",
                1.,
                ui::Color::Muted,
                1.,
                vec![3., 3.],
                c::LineCap::Butt,
            ),
            (
                "traffic",
                4.,
                ui::Color::Accent,
                0.6,
                vec![],
                c::LineCap::Round,
            ),
            (
                "paused",
                1.,
                ui::Color::Muted,
                1.,
                vec![0., 2.],
                c::LineCap::Round,
            ),
        ] {
            legend.push(
                channel_path(
                    format!("channels/key/{name}"),
                    [18., weight],
                    vec![
                        c::PathCommand::Move([0., weight * 0.5]),
                        c::PathCommand::Line([18., weight * 0.5]),
                    ],
                    weight,
                    col,
                    alpha,
                    dash,
                    cap,
                )
                .into(),
            );
            legend.push(
                text(
                    format!("channels/key/{name}/text"),
                    name,
                    STYLE.r#type.small as f32,
                    400,
                    false,
                    ui::Color::Muted,
                )
                .into(),
            );
        }
        let legend = row(
            "channels/key",
            plain(None, [0.; 4], 0.),
            STYLE.spacing.s2 as f32,
            legend,
        );
        let head = row(
            "channels/head",
            plain(None, [8., 0., 4., 0.], 0.),
            10.,
            vec![
                text(
                    "channels/title",
                    format!(
                        "Channels · {}",
                        g.root
                            .as_deref()
                            .filter(|s| !s.is_empty())
                            .unwrap_or("bboxy450://")
                    ),
                    12.,
                    500,
                    false,
                    ui::Color::Muted,
                )
                .into(),
                fill(c::Element::Spacer),
                legend.into(),
            ],
        );
        let mut s = ui::Style::new(ui::Preset::Card);
        s.overrides.padding = Some([4., 14., 10., 14.].map(ui::Length::Px));
        column("channels", s, 0., vec![head.into(), body.into()])
    }
}

#[allow(clippy::too_many_arguments)] // One retained stroke record plus its semantic key.
fn channel_path(
    key: impl Into<String>,
    size: [f32; 2],
    points: Vec<c::PathCommand>,
    width: f32,
    color: ui::Color,
    opacity: f32,
    dash: Vec<f32>,
    cap: c::LineCap,
) -> c::Element {
    named(
        key,
        c::Element::Path(c::Path {
            size,
            points: points.into(),
            stroke: c::PathStroke {
                width,
                dash: dash.into(),
                cap,
                join: c::LineJoin::Miter,
            },
            fill: None,
            paint: ui::Paint::solid(color),
            opacity,
        }),
    )
}
#[allow(clippy::too_many_arguments)] // SVG central-baseline/anchor inputs resolved using host font metrics.
fn channel_text(
    key: impl Into<String>,
    value: &str,
    style: ui::Text,
    x: f32,
    y: f32,
    center: bool,
    ctx: ui::Context,
    m: &dyn c::Measure,
) -> (c::Element, f32, f32) {
    let mut t = c::Text::new(value, style);
    t.wrap = false;
    t.ellipsis = false;
    t.inline = true;
    let e = named(key, c::Element::Text(t));
    let (size, baseline) = e
        .natural(f32::INFINITY, ctx, m)
        .expect("channel label geometry");
    let [a, d] = m
        .font_metrics(&bbt_ui_model::layout::TextStyle {
            size: style.size.unwrap(),
            weight: style.weight,
            mono: style.mono,
            italic: false,
            tracking_em: 0.,
        })
        .map(f32::round);
    (
        e,
        x - if center { size[0] * 0.5 } else { 0. },
        y + (a - d) * 0.5 - baseline,
    )
}

fn kind_label(key: &str, value: &str) -> c::Element {
    kind(key, value)
}
fn inline_icon(key: &str, name: &str, size: f32, font_size: f32, col: ui::Color) -> c::Element {
    let mut b = c::Box::new(
        plain(None, [0.; 4], 0.),
        vec![
            c::Element::Baseline {
                content: Box::new(icon(&format!("{key}/icon"), name, size, col)),
                offset: -1.,
            }
            .into(),
        ],
    );
    b.inline = true;
    b.align = c::Alignment::Baseline;
    b.style.overrides.text = Some(font(font_size, 400, true, col));
    b.gap = ui::Length::Px(0.);
    named(key, c::Element::Box(b))
}
fn invariant(key: &str, title: &str, violated: bool) -> c::Element {
    let col = if violated {
        color(Some(TOKENS.color.state.errored), ui::Color::Muted)
    } else {
        color(Some(TOKENS.color.ui.invariant), ui::Color::Muted)
    };
    let mut s = plain(Some(16.), [0., 6., 0., 6.], 999.);
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: paint(col, if violated { 0.6 } else { 0.55 }),
        outside: false,
    });
    if violated {
        s.overrides.fill = Some(paint(col, 0.1))
    }
    row(
        key,
        s,
        2.,
        vec![
            label(
                format!("{key}/prefix"),
                if violated { "breaks" } else { "inv" },
                ui::Text {
                    line_height: 14. / 9.5,
                    ..font(
                        9.5,
                        400,
                        true,
                        if violated { col } else { ui::Color::Muted },
                    )
                },
            )
            .into(),
            label(
                format!("{key}/title"),
                title,
                ui::Text {
                    line_height: 14. / 9.5,
                    ..font(
                        9.5,
                        500,
                        true,
                        if violated { col } else { ui::Color::Foreground },
                    )
                },
            )
            .into(),
        ],
    )
}
/// Parsed wikilinks preserve the slug/display distinction and trailing relation.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub slug: String,
    pub display: String,
    pub relation: Option<String>,
    pub weight: Option<String>,
}
pub fn parse_links(value: &str) -> Vec<Link> {
    let mut out = vec![];
    let mut rest = value;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        let label = &rest[..end];
        let (slug, display) = label
            .split_once('|')
            .map_or((label, label), |(slug, display)| {
                (slug.strip_suffix('\\').unwrap_or(slug), display)
            });
        rest = &rest[end + 2..];
        let mut relation = None;
        let mut weight = None;
        if let Some(tail) = rest.strip_prefix('{')
            && let Some(end) = tail.find('}')
        {
            let (rel, w) = tail[..end]
                .split_once(':')
                .map_or((&tail[..end], None), |(r, w)| (r, Some(w)));
            relation = Some(rel.into());
            weight = w.map(Into::into);
            rest = &tail[end + 1..];
        }
        out.push(Link {
            slug: slug.into(),
            display: display.into(),
            relation,
            weight,
        });
    }
    out
}
fn links(key: &str, value: &str, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
    let mut children = vec![];
    for (i, l) in parse_links(value).iter().enumerate() {
        let k = format!("{key}/l{i}");
        let mut s = plain(None, [0.; 4], STYLE.radius.tag as f32);
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(hair(ctx)),
            outside: true,
        });
        let mut items = vec![
            badge(
                &format!("{k}/slug"),
                &l.display,
                ui::Text {
                    line_height: 16. / 11.,
                    ..font(
                        11.,
                        400,
                        false,
                        color(Some(ctx.theme.palette().accent_soft), ui::Color::Accent),
                    )
                },
                [0., 6., 0., 6.],
                0.,
                Some(paint(ui::Color::Accent, 0.1)),
                None,
            )
            .into(),
        ];
        if let Some(rel) = &l.relation {
            let value = format!(
                "{rel}{}",
                l.weight
                    .as_ref()
                    .map(|w| format!(" {w}"))
                    .unwrap_or_default()
            );
            let col = match rel.as_str() {
                "violates" => color(Some(TOKENS.color.state.errored), ui::Color::Muted),
                "constrains" => color(Some(TOKENS.color.state.waiting), ui::Color::Muted),
                "derives" | "derived_from" => {
                    color(Some(ctx.theme.palette().accent_soft), ui::Color::Accent)
                }
                _ => ui::Color::Muted,
            };
            items.push(
                badge(
                    &format!("{k}/rel"),
                    &value,
                    ui::Text {
                        line_height: 16. / 9.5,
                        ..font(9.5, 400, true, col)
                    },
                    [0., 5., 0., 5.],
                    0.,
                    Some(ui::Paint {
                        color: ui::Color::Muted,
                        alpha: ui::Alpha::Badge,
                    }),
                    None,
                )
                .into(),
            )
        }
        children.push(
            c::Element::InlineMiddle {
                content: Box::new(row(&k, s, 0., items)),
                parent: font(11.5, 400, false, ui::Color::Foreground),
            }
            .into(),
        );
    }
    let mut b = c::Box::new(plain(None, [0., 2., 0., 2.], 0.), children);
    b.gap = ui::Length::Px(
        4. + m.width(
            " ",
            &bbt_ui_model::layout::TextStyle {
                size: 11.5,
                weight: 400,
                mono: false,
                italic: false,
                tracking_em: 0.,
            },
        ),
    );
    b.inline = true;
    b.style.overrides.text = Some(font(11.5, 400, false, ui::Color::Foreground));
    b.align = c::Alignment::Baseline;
    named(key, c::Element::Box(b))
}
