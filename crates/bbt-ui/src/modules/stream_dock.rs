// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Chronological stream compositions. Delivery, routing and actions remain host-owned.
use super::{
    seat_row::{named, role_color},
    strip::plain,
};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color as DrawColor, layout::Axis};
use serde::{Deserialize, Serialize};
mod actions;
pub use actions::{Action, Regions};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Message {
    /// Retained message identities for host frame receipts; no visual effect.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub message_ids: Vec<String>,
    pub author: Option<Author>,
    #[serde(rename = "issuer_kind")]
    pub issuer_kind: Option<String>,
    pub signature: Option<String>,
    pub signature_reason: Option<String>,
    #[serde(rename = "priority_class")]
    pub priority_class: Option<String>,
    pub priority: u8,
    pub ts: Option<u64>,
    pub receipts: Option<Receipts>,
    pub from: String,
    pub role: String,
    pub time: String,
    pub text: String,
    pub presence: bool,
    pub rejected: bool,
    pub delegator: String,
    pub audit: Option<Audit>,
    pub foreign: bool,
    pub unread: bool,
    pub reply_to: Option<String>,
    pub evidence: Option<String>,
    pub ch: Option<String>,
    pub kind: Option<String>,
    pub group: Option<Group>,
    pub from_agent: Option<String>,
    pub via: Option<String>,
    pub to: Option<String>,
    pub held: Option<String>,
    pub to_label: Option<String>,
    pub attach: Vec<super::composer::Attachment>,
    pub report: Option<super::composer::Report>,
    pub review: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Author {
    pub kind: String,
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Receipts {
    pub delivered: usize,
    pub acked: usize,
    pub held: usize,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Audit {
    pub actor: String,
    pub verb: String,
    pub target: String,
    pub op: String,
    pub outcome: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    pub name: String,
    pub n: usize,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkItem {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(rename = "status", alias = "state")]
    pub state: String,
    pub assignee: String,
    pub delegated_by: Option<String>,
    pub waits: String,
    #[serde(rename = "type")]
    pub kind: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Epic {
    pub title: String,
    pub open: Option<bool>,
    pub items: Vec<WorkItem>,
    pub delegate: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct External {
    pub source: String,
    pub text: String,
    pub time: String,
}
/// A badge is numeric only when the retained pages prove its count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreadCount {
    Exact(u64),
    More,
}
impl Default for UnreadCount {
    fn default() -> Self {
        Self::Exact(0)
    }
}
impl UnreadCount {
    pub fn any(self) -> bool {
        self != Self::Exact(0)
    }
}
impl std::fmt::Display for UnreadCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact(n) => write!(f, "{n}"),
            Self::More => f.write_str("more"),
        }
    }
}
impl Serialize for UnreadCount {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Exact(n) => s.serialize_u64(*n),
            Self::More => s.serialize_str("more"),
        }
    }
}
impl<'de> Deserialize<'de> for UnreadCount {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Number(u64),
            Marker(String),
        }
        match Value::deserialize(d)? {
            Value::Number(n) => Ok(Self::Exact(n)),
            Value::Marker(s) if s == "more" => Ok(Self::More),
            _ => Err(serde::de::Error::custom("unread must be a count or more")),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Channel {
    pub name: String,
    pub unread: UnreadCount,
    pub muted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Entry {
    Message {
        data: Box<Message>,
        #[serde(default)]
        show_channel: Option<bool>,
    },
    Epic {
        data: Epic,
    },
    Task {
        data: WorkItem,
        delegate: bool,
    },
    Report {
        id: String,
        data: super::composer::Report,
        review: bool,
    },
    External {
        events: Vec<External>,
        open: bool,
    },
    Tabs {
        channels: Vec<Channel>,
        selected: String,
        #[serde(default)]
        total: Option<u64>,
    },
    Filter {
        name: String,
    },
    Note {
        text: String,
        padding: [f32; 4],
    },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamDock {
    pub dock: bool,
    #[serde(default = "framed")]
    pub framed: bool,
    pub entries: Vec<Entry>,
}
fn framed() -> bool {
    true
}

pub(super) fn rgba(hex: &str) -> ui::Color {
    ui::Color::Rgba(DrawColor::hex(hex).0)
}
pub(super) fn tint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
pub(super) fn text_style(size: f32, weight: u16, mono: bool, color: ui::Color) -> ui::Text {
    ui::Text {
        size: Some(size),
        weight,
        mono,
        paint: ui::Paint::solid(color),
        line_height: 1.4,
        ..Default::default()
    }
}
pub(super) fn label(
    key: impl Into<String>,
    value: impl Into<String>,
    style: ui::Text,
    wrap: bool,
) -> c::Element {
    let mut t = c::Text::new(value, style);
    t.wrap = wrap;
    t.ellipsis = !wrap;
    named(key, c::Element::Text(t))
}
pub(super) fn row(
    key: impl Into<String>,
    style: ui::Style,
    gap: f32,
    align: c::Alignment,
    children: Vec<c::Item>,
) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.gap = ui::Length::Px(gap);
    b.align = align;
    named(key, c::Element::Box(b))
}
pub(super) fn column(
    key: impl Into<String>,
    style: ui::Style,
    children: Vec<c::Item>,
) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = Axis::Column;
    b.gap = ui::Length::Px(0.);
    b.align = c::Alignment::Stretch;
    named(key, c::Element::Box(b))
}
pub(super) fn fill(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fill,
    }
}
pub(super) fn shrink(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::MinContent,
    }
}
pub(super) fn spacer() -> c::Item {
    fill(c::Element::Spacer)
}
// An auto start margin consumes spare width without adding another flex gap.
fn auto_left(e: c::Element) -> c::Item {
    c::Item {
        element: row(
            "",
            plain(None, [0.; 4], 0.),
            0.,
            c::Alignment::Center,
            vec![spacer(), e.into()],
        ),
        extent: c::Extent::Content {
            grow: 1.,
            shrink: 0.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn aligned_text(key: &str, value: &str, style: ui::Text, wrap: bool) -> c::Element {
    let mut text = c::Text::new(value, style);
    text.wrap = wrap;
    text.align = c::Alignment::Center;
    named(key, c::Element::Text(text))
}
pub(super) fn badge(
    key: &str,
    value: &str,
    mut style: ui::Text,
    background: Option<ui::Paint>,
    border: Option<ui::Paint>,
    geometry: (f32, f32, Option<f32>, bool),
) -> c::Element {
    let (radius, pad, height, dashed) = geometry;
    if let (Some(height), Some(size)) = (height, style.size) {
        style.line_height = (height - if border.is_some() { 2. } else { 0. }) / size;
    }
    let mut s = plain(height, [0., pad, 0., pad], radius);
    s.overrides.dashed = Some(dashed);
    if let Some(background) = background {
        s.overrides.fill = Some(background);
    }
    if let Some(paint) = border {
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint,
            outside: false,
        });
    }
    let mut b = c::Box::new(
        s,
        vec![fill(label(format!("{key}/text"), value, style, false))],
    );
    b.gap = ui::Length::Px(0.);
    b.clip = true;
    named(key, c::Element::Box(b))
}
fn clipped_text(key: &str, value: &str, style: ui::Text) -> c::Element {
    label(key, value, style, false)
}

pub fn evidence(key: &str, state: &str, m: &dyn c::Measure) -> c::Element {
    let size = STYLE.r#type.row as f32;
    let color = if state == "application-acknowledged" {
        ui::Color::Accent
    } else {
        ui::Color::Muted
    };
    let marks = match state {
        "sender-stored" => "·",
        "recipient-stored" => "✓",
        "native-queued" | "native-written" => "✓✓",
        "application-acknowledged" => "✓✓✓",
        "native-unknown" => "✓◌",
        _ => "",
    };
    let mut base = plain(Some(size), [0.; 4], 0.);
    base.overrides.width = Some(ui::Length::Px(size * 2.4));
    let mut left = 0.;
    let mut overlays = vec![];
    for (i, ch) in marks.chars().enumerate() {
        let k = if ch == '◌' {
            format!("{key}/hollow")
        } else if state == "native-queued" && i == 1 {
            format!("{key}/q")
        } else {
            format!(
                "{key}/i{}",
                if i == 0 { String::new() } else { i.to_string() }
            )
        };
        if ch == '◌' {
            left += 1.;
            let mut ring = plain(Some(8.), [0.; 4], 4.);
            ring.overrides.dashed = Some(true);
            ring.overrides.width = Some(ui::Length::Px(8.));
            ring.overrides.border = Some(ui::Border {
                width: ui::Length::Px(1.3),
                paint: ui::Paint::solid(color),
                outside: false,
            });
            overlays.push(c::Placed {
                content: named(k, c::Element::Box(c::Box::new(ring, vec![]))),
                left,
                bottom: (size - 8.) / 2.,
            });
        } else {
            if i > 0 {
                left -= size * 0.42;
            }
            let mut ts = text_style(size, 600, false, color);
            ts.line_height = 1.;
            if state == "native-queued" && i == 1 {
                ts.paint = tint(color, 0.38);
            }
            overlays.push(c::Placed {
                content: label(k, ch.to_string(), ts, false),
                left,
                bottom: 0.,
            });
            left += m.width(
                &ch.to_string(),
                &bbt_ui_model::layout::TextStyle {
                    size,
                    weight: 600,
                    mono: false,
                    italic: false,
                    tracking_em: 0.,
                },
            );
        }
    }
    named(
        key,
        c::Element::Layers {
            base: Box::new(c::Element::Box(c::Box::new(base, vec![]))),
            overlays,
        },
    )
}
impl Message {
    pub fn element(
        &self,
        ctx: ui::Context,
        key: &str,
        dock: bool,
        show_ch: bool,
        m: &dyn c::Measure,
    ) -> c::Element {
        let mut view = self.clone();
        if let Some(author) = &self.author {
            view.from = author.name.clone();
        }
        if view.time.is_empty()
            && let Some(ts) = self.ts
        {
            view.time = format!("{:02}:{:02}", (ts / 3_600_000) % 24, (ts / 60_000) % 60);
        }
        if self.kind.as_deref() == Some("presence") {
            view.presence = true;
        }
        view.element_view(ctx, key, dock, show_ch, m)
    }
    fn element_view(
        &self,
        ctx: ui::Context,
        key: &str,
        dock: bool,
        show_ch: bool,
        m: &dyn c::Measure,
    ) -> c::Element {
        let t = &STYLE.r#type;
        let sp = &STYLE.spacing;
        let caption = t.caption as f32;
        let small = t.small as f32;
        let notice_size = if dock { caption } else { t.note as f32 };
        let body_size = if dock { caption } else { t.body as f32 };
        let pad = if dock { sp.s2 as f32 } else { sp.s3 as f32 };
        let right = if dock { sp.s7 as f32 } else { 22. };
        let base = plain(None, [pad, right, pad, 0.], STYLE.radius.row as f32);
        let time = || {
            label(
                format!("{key}/time"),
                &self.time,
                text_style(
                    if self.rejected { small } else { notice_size },
                    400,
                    !self.rejected,
                    ui::Color::Muted,
                ),
                false,
            )
        };
        if self.presence && self.audit.is_none() {
            let style = if dock {
                base
            } else {
                plain(
                    None,
                    [3., sp.s4 as f32, 3., sp.s4 as f32],
                    STYLE.radius.row as f32,
                )
            };
            let flow = c::Flow {
                children: vec![
                    shrink(aligned_text(
                        &format!("{key}/ptext"),
                        &self.text,
                        text_style(notice_size, 400, true, ui::Color::Muted),
                        true,
                    )),
                    time().into(),
                ],
                wrap: dock,
                justify: c::Justify::Center,
                align: c::Alignment::Center,
                row_gap: ui::Length::Px(0.),
                column_gap: ui::Length::Px(sp.s4 as f32),
            };
            return column(key, style, vec![c::Element::Flow(flow).into()]);
        }
        if let Some(a) = &self.audit {
            // The routing rejection class also applies to rejected audit notices.
            let notice_size = if a.outcome == "rejected" {
                small
            } else {
                notice_size
            };
            let mut text = c::Text::new("♔ ", text_style(caption, 400, true, role_color("oracle")));
            text.align = c::Alignment::Center;
            text.runs.extend([
                c::TextRun {
                    value: if a.actor.is_empty() {
                        "Oracle".into()
                    } else {
                        a.actor.clone()
                    },
                    style: text_style(notice_size, 500, true, ui::Color::Foreground),
                },
                c::TextRun {
                    value: format!(" {} ", a.verb),
                    style: text_style(notice_size, 400, true, ui::Color::Muted),
                },
                c::TextRun {
                    value: a.target.clone(),
                    style: text_style(notice_size, 500, true, ui::Color::Foreground),
                },
            ]);
            let mut meta = vec![];
            if !a.op.is_empty() {
                meta.push(
                    badge(
                        &format!("{key}/ameta/aop"),
                        &a.op.chars().take(6).collect::<String>(),
                        text_style(t.tiny as f32, 400, true, ui::Color::Muted),
                        Some(ui::Paint {
                            color: ui::Color::Muted,
                            alpha: ui::Alpha::Badge,
                        }),
                        None,
                        (TOKENS.radius.chip as f32, sp.s2 as f32, None, false),
                    )
                    .into(),
                );
            }
            if !a.outcome.is_empty() && a.outcome != "applied" {
                let color = if a.outcome == "rejected" {
                    rgba(TOKENS.color.state.errored)
                } else {
                    rgba(TOKENS.color.state.waiting)
                };
                meta.push(
                    badge(
                        &format!("{key}/ameta/aout"),
                        &a.outcome,
                        text_style(t.tiny as f32, 400, true, color),
                        None,
                        Some(if a.outcome == "rejected" {
                            tint(color, 0.5)
                        } else {
                            ui::Paint::solid(ui::Color::Line)
                        }),
                        (STYLE.radius.pill as f32, 5., None, a.outcome != "rejected"),
                    )
                    .into(),
                );
            }
            meta.push(
                label(
                    format!("{key}/ameta/time"),
                    &self.time,
                    text_style(notice_size, 400, true, ui::Color::Muted),
                    false,
                )
                .into(),
            );
            let mut audit_style = if dock {
                base
            } else {
                plain(
                    None,
                    [3., sp.s4 as f32, 3., sp.s4 as f32],
                    STYLE.radius.row as f32,
                )
            };
            if a.outcome == "rejected" {
                audit_style.overrides.padding = Some([
                    ui::Length::Spacing(1),
                    ui::Length::Spacing(4),
                    ui::Length::Spacing(1),
                    ui::Length::Spacing(4),
                ]);
                audit_style.overrides.dashed = Some(true);
                audit_style.overrides.border = Some(ui::Border {
                    width: ui::Length::Hairline,
                    paint: tint(rgba(TOKENS.color.state.errored), 0.45),
                    outside: false,
                });
            }
            let mut b = c::Box::new(
                audit_style,
                vec![
                    named(format!("{key}/aline"), c::Element::Text(text)).into(),
                    row(
                        format!("{key}/ameta"),
                        plain(None, [0.; 4], 0.),
                        sp.s3 as f32,
                        c::Alignment::Center,
                        meta,
                    )
                    .into(),
                ],
            );
            b.direction = Axis::Column;
            b.gap = ui::Length::Spacing(1);
            b.align = c::Alignment::Center;
            return named(key, c::Element::Box(b));
        }
        if self.rejected {
            let mut s = base;
            s.overrides.dashed = Some(true);
            s.overrides.padding = Some([
                ui::Length::Spacing(1),
                ui::Length::Spacing(4),
                ui::Length::Spacing(1),
                ui::Length::Spacing(4),
            ]);
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: tint(rgba(TOKENS.color.state.errored), 0.45),
                outside: false,
            });
            let mut text = c::Text::new(
                "⊘ ",
                text_style(small, 400, false, rgba(TOKENS.color.state.errored)),
            );
            text.runs.push(c::TextRun {
                value: self.from.clone(),
                style: text_style(small, 500, false, ui::Color::Foreground),
            });
            text.runs.push(c::TextRun {
                value: " tried to post progress to the stream".into(),
                style: text_style(small, 400, false, ui::Color::Muted),
            });
            let flow = c::Flow {
                children: vec![
                    named(format!("{key}/rj-t"), c::Element::Text(text)).into(),
                    label(
                        format!("{key}/rj-hint"),
                        format!(
                            "DM your delegator{}",
                            if self.delegator.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", self.delegator)
                            }
                        ),
                        text_style(small, 500, false, ui::Color::Accent),
                        true,
                    )
                    .into(),
                    auto_left(time()),
                ],
                wrap: true,
                justify: c::Justify::Start,
                align: c::Alignment::Center,
                row_gap: ui::Length::Spacing(3),
                column_gap: ui::Length::Spacing(3),
            };
            return column(key, s, vec![c::Element::Flow(flow).into()]);
        }
        let author_kind = self.author.as_ref().map(|a| a.kind.as_str());
        let role = if author_kind == Some("operator") {
            "oracle"
        } else if self.role.is_empty() {
            if self.author.is_some() {
                ""
            } else {
                "unsigned"
            }
        } else {
            &self.role
        };
        let color = if role.is_empty() {
            ui::Color::Muted
        } else {
            role_color(role)
        };
        let head_key = format!("{key}/body/head");
        let mut head = vec![shrink(label(
            format!("{head_key}/who"),
            &self.from,
            text_style(
                if dock { caption } else { t.section as f32 },
                600,
                false,
                ui::Color::Foreground,
            ),
            true,
        ))];
        // Roles stay visible through the rail. Repeat only exceptional priority,
        // not the default authority class or local issuer on every message.
        if self.priority >= 7 {
            head.push(
                badge(
                    &format!("{head_key}/p1"),
                    &format!("Priority {}", self.priority),
                    text_style(t.micro as f32, 400, true, ui::Color::Accent),
                    Some(tint(ui::Color::Accent, 0.14)),
                    Some(ui::Paint {
                        color: ui::Color::Line,
                        alpha: ui::Alpha::Hairline,
                    }),
                    (STYLE.radius.tag as f32, 5., Some(16.), false),
                )
                .into(),
            );
        }

        if show_ch && let Some(ch) = &self.ch {
            head.push(
                label(
                    format!("{head_key}/chchip"),
                    format!("#{ch}"),
                    text_style(t.micro as f32, 400, true, ui::Color::Muted),
                    false,
                )
                .into(),
            );
        }
        if let Some(kind) = self
            .kind
            .as_deref()
            .filter(|s| *s != "message" && *s != "text" && *s != "chunk")
        {
            let col = match kind {
                "chunk" => rgba(TOKENS.color.ui.ok),
                "decision" => rgba(TOKENS.color.ui.invariant),
                _ => ui::Color::Muted,
            };
            let mut ts = text_style(t.micro as f32, 400, false, col);
            ts.uppercase = true;
            ts.tracking_em = 0.08;
            head.push(
                badge(
                    &format!("{head_key}/kchip"),
                    kind,
                    ts,
                    Some(tint(col, 0.14)),
                    None,
                    (STYLE.radius.tag as f32, 5., Some(14.), false),
                )
                .into(),
            );
        }
        let mut chips = vec![];
        if let Some(g) = &self.group {
            chips.push((
                "grp",
                format!(
                    "⧉ {}{}",
                    g.name,
                    if g.n > 0 {
                        format!(" · {}", g.n)
                    } else {
                        String::new()
                    }
                ),
            ));
        }
        if let Some(a) = &self.from_agent {
            chips.push(("from", format!("↳ {a}")));
        }
        if self.via.as_deref() == Some("main") {
            chips.push(("via", "via main".into()));
        }
        if let Some(to) = &self.to {
            chips.push((
                "to",
                format!(
                    "→ {}",
                    if to == "main" {
                        to.clone()
                    } else {
                        format!("↳ {to}")
                    }
                ),
            ));
        }
        for (i, (kind, value)) in chips.iter().enumerate() {
            let fg = if *kind == "from" {
                color
            } else if *kind == "grp" || (*kind == "to" && self.to.as_deref() != Some("main")) {
                ui::Color::Foreground
            } else {
                ui::Color::Muted
            };
            let mut chip_text = text_style(small, 400, false, fg);
            chip_text.italic = *kind == "via";
            let e = badge(
                &format!(
                    "{head_key}/agchip{}",
                    if i == 0 { String::new() } else { i.to_string() }
                ),
                value,
                chip_text,
                None,
                Some(if *kind == "from" {
                    tint(color, 0.45)
                } else {
                    ui::Paint {
                        color: ui::Color::Line,
                        alpha: ui::Alpha::Hairline,
                    }
                }),
                (STYLE.radius.pill as f32, sp.s3 as f32, Some(16.), false),
            );
            head.push(c::Item {
                element: e,
                extent: c::Extent::Content {
                    grow: 0.,
                    shrink: 1.,
                    min: 0.,
                    max: Some(small * 16.),
                    max_fraction: None,
                },
            });
        }
        head.push(auto_left(label(
            format!("{head_key}/time"),
            &self.time,
            text_style(small, 400, true, ui::Color::Muted),
            false,
        )));
        if self.held.as_deref() == Some("agent_ended") {
            head.push(
                badge(
                    &format!(
                        "{head_key}/agchip{}",
                        if chips.is_empty() {
                            String::new()
                        } else {
                            chips.len().to_string()
                        }
                    ),
                    "held · agent ended",
                    text_style(small, 400, false, rgba(TOKENS.color.state.waiting)),
                    None,
                    Some(tint(rgba(TOKENS.color.state.waiting), 0.6)),
                    (STYLE.radius.pill as f32, sp.s3 as f32, Some(16.), true),
                )
                .into(),
            );
        } else if let Some(receipts) = &self.receipts {
            if receipts.held > 0 {
                head.push(
                    label(
                        format!("{head_key}/receipts"),
                        format!("{} held", receipts.held),
                        text_style(t.tiny as f32, 400, true, rgba(TOKENS.color.state.waiting)),
                        false,
                    )
                    .into(),
                );
            }
        } else if let Some(ev) = &self.evidence {
            head.push(evidence(&format!("{head_key}/ticks"), ev, m).into());
        }
        let mut body = vec![
            row(
                &head_key,
                plain(None, [0.; 4], 0.),
                sp.s3 as f32,
                c::Alignment::Center,
                head,
            )
            .into(),
        ];
        if let Some(signature) = self.signature.as_ref().filter(|grade| grade.as_str() != "signed") {
            let pending = signature == "pending";
            let grade = if pending {
                "pending · inferred"
            } else {
                signature
            };
            let text = self.signature_reason.as_ref().map_or_else(
                || format!("signature: {grade}"),
                |reason| format!("signature: {grade} · {reason}"),
            );
            body.push(
                label(
                    format!("{head_key}/signature"),
                    text,
                    text_style(
                        t.tiny as f32,
                        400,
                        false,
                        if pending {
                            rgba(TOKENS.color.state.waiting)
                        } else {
                            ui::Color::Muted
                        },
                    ),
                    true,
                )
                .into(),
            );
        }
        if let Some(reply) = &self.reply_to {
            let mut s = plain(None, [0., 0., 0., sp.s3 as f32], 0.);
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Px(2.),
                paint: ui::Paint::solid(ui::Color::Line),
                outside: false,
            });
            s.overrides.border_sides = Some([false, false, false, true]);
            body.push(
                column(
                    "",
                    plain(None, [sp.s1 as f32, 0., 0., 0.], 0.),
                    vec![
                        column(
                            format!("{key}/body/reply"),
                            s,
                            vec![
                                label(
                                    format!("{key}/body/reply/text"),
                                    format!("↩ {reply}"),
                                    text_style(caption, 400, false, ui::Color::Muted),
                                    false,
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
        if let Some(to) = &self.to_label {
            body.push(
                column(
                    format!("{key}/body/to"),
                    plain(None, [0.; 4], 0.),
                    vec![
                        label(
                            format!("{key}/body/to/text"),
                            format!("→ {to}"),
                            text_style(small, 400, false, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        let mut ts = text_style(body_size, 400, false, ui::Color::Foreground);
        ts.line_height = 1.5;
        ts.italic = role == "creativity";
        let mut text_box = plain(None, [sp.s2 as f32, 0., 0., 0.], 0.);
        text_box.overrides.text = Some(ts);
        body.push(
            column(
                format!("{key}/body/text-wrap"),
                text_box,
                vec![label(format!("{key}/body/text"), &self.text, ts, true).into()],
            )
            .into(),
        );
        if !self.attach.is_empty() {
            let mut attachments = c::Box::new(
                plain(None, [0.; 4], 0.),
                self.attach
                    .iter()
                    .enumerate()
                    .map(|(i, a)| a.element(&format!("{key}/body/attach/a{i}")).into())
                    .collect(),
            );
            attachments.direction = Axis::Column;
            attachments.gap = ui::Length::Spacing(2);
            attachments.align = c::Alignment::Stretch;
            body.push(
                column(
                    format!("{key}/body/attach-margin"),
                    plain(None, [sp.s2 as f32, 0., 0., 0.], 0.),
                    vec![named(format!("{key}/body/attach"), c::Element::Box(attachments)).into()],
                )
                .into(),
            );
        }
        if let Some(report) = &self.report {
            body.push(
                report
                    .element(ctx, &format!("{key}/body/report"), self.review)
                    .into(),
            );
        }
        let mut rail = plain(None, [0.; 4], STYLE.radius.mark as f32);
        rail.overrides.width = Some(ui::Length::Px(3.));
        rail.overrides.fill = Some(ui::Paint::solid(color));
        let mut rail = c::Box::new(rail, vec![]);
        if role == "creativity" {
            rail.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
            rail.clip = true;
            rail.align = c::Alignment::Stretch;
            rail.gap = ui::Length::Px(0.);
            rail.children.push(
                c::Element::Rects(c::Rects {
                    size: [3., 0.],
                    rects: vec![(
                        bbt_ui_model::layout::Rect::new(0., 0., 3., 4.),
                        ui::Paint::solid(color),
                    )]
                    .into(),
                    repeat: Some((Axis::Column, 7.)),
                })
                .into(),
            );
        }
        let pad = sp.s3 as f32;
        let mut style = base;
        style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
        style.overrides.padding = Some([
            ui::Length::Px(pad),
            ui::Length::Px(0.),
            ui::Length::Px(pad),
            ui::Length::Spacing(3),
        ]);
        let content = row(
            format!("{key}/contents"),
            plain(None, [0.; 4], 0.),
            if dock { sp.s4 as f32 } else { sp.s5 as f32 },
            c::Alignment::Stretch,
            vec![
                named(format!("{key}/rail"), c::Element::Box(rail)).into(),
                fill(column(
                    format!("{key}/body"),
                    plain(None, [0.; 4], 0.),
                    body,
                )),
            ],
        );
        let size = if dock { 5. } else { 7. };
        let mut margin = plain(
            None,
            [
                sp.s2 as f32,
                0.,
                0.,
                right - if dock { 4. } else { 8. } - size,
            ],
            0.,
        );
        margin.overrides.width = Some(ui::Length::Px(right));
        let dots = if self.unread {
            let mut dot = plain(Some(size), [0.; 4], size / 2.);
            dot.overrides.width = Some(ui::Length::Px(size));
            dot.overrides.fill = Some(ui::Paint::solid(color));
            vec![
                named(
                    format!("{key}/unread-dot"),
                    c::Element::Box(c::Box::new(dot, vec![])),
                )
                .into(),
            ]
        } else {
            vec![]
        };
        let mut unread_slot = c::Box::new(margin, dots);
        unread_slot.direction = Axis::Column;
        unread_slot.align = c::Alignment::Start;
        unread_slot.gap = ui::Length::Px(0.);
        let base = row(
            key,
            style,
            0.,
            c::Alignment::Stretch,
            vec![
                fill(content),
                named(format!("{key}/unread-slot"), c::Element::Box(unread_slot)).into(),
            ],
        );
        let card = if self.reply_to.is_some() {
            column(
                format!("{key}/nested"),
                plain(None, [0., 0., 0., 18.], 0.),
                vec![base.into()],
            )
        } else {
            base
        };
        column(
            format!("{key}/space"),
            plain(None, [0., 0., sp.s2 as f32, 0.], 0.),
            vec![card.into()],
        )
    }
}

fn kind_chip(key: &str, value: &str) -> c::Element {
    let mut ts = text_style(STYLE.r#type.micro as f32, 400, true, ui::Color::Muted);
    ts.uppercase = true;
    ts.tracking_em = 0.06;
    let mut s = plain(None, [1., 5., 1., 5.], TOKENS.radius.chip as f32);
    s.overrides.fill = Some(ui::Paint {
        color: ui::Color::Muted,
        alpha: ui::Alpha::Badge,
    });
    column(
        key,
        s,
        vec![label(format!("{key}/text"), value, ts, false).into()],
    )
}
fn task_item(item: &WorkItem, k: &str, delegate: bool) -> c::Element {
    let sp = &STYLE.spacing;
    let t = &STYLE.r#type;
    let col = match item.state.as_str() {
        "running" => ui::Color::Accent,
        "reported" => rgba(TOKENS.color.state.waiting),
        "accepted" | "done" => rgba(TOKENS.color.ui.ok),
        "rejected" | "blocked" => rgba(TOKENS.color.state.errored),
        _ => ui::Color::Muted,
    };
    let mut cells = vec![
        named(
            format!("{k}/tglyph"),
            ui::marks::Mark::task(&item.state).element(11., ui::Paint::solid(col)),
        )
        .into(),
        kind_chip(
            &format!("{k}/ekind"),
            if item.kind.is_empty() {
                "task"
            } else {
                &item.kind
            },
        )
        .into(),
        fill(clipped_text(
            &format!("{k}/etitle"),
            &item.title,
            text_style(t.caption as f32, 400, false, ui::Color::Foreground),
        )),
    ];
    if !item.assignee.is_empty() {
        cells.push(
            label(
                format!("{k}/eass"),
                if item.state == "claimed" {
                    format!("→ {}", item.assignee)
                } else {
                    item.assignee.clone()
                },
                text_style(t.small as f32, 400, false, ui::Color::Muted),
                false,
            )
            .into(),
        );
    }
    if item.state == "claimed" {
        cells.push(
            badge(
                &format!("{k}/echip"),
                "claimed",
                text_style(t.micro as f32, 400, false, role_color("executor")),
                None,
                Some(tint(role_color("executor"), 0.6)),
                (STYLE.radius.pill as f32, sp.s2 as f32, None, true),
            )
            .into(),
        );
    }
    if delegate && item.state == "open" && item.assignee.is_empty() {
        cells.push(
            badge(
                &format!("{k}/edel"),
                "Delegate…",
                text_style(t.small as f32, 400, false, ui::Color::Foreground),
                None,
                Some(ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                }),
                (STYLE.radius.pill as f32, sp.s3 as f32, None, false),
            )
            .into(),
        );
    }
    if item.state == "blocked" && !item.waits.is_empty() {
        cells.push(
            label(
                format!("{k}/twaits"),
                format!("waits on {}", item.waits),
                text_style(t.tiny as f32, 400, false, rgba(TOKENS.color.state.errored)),
                false,
            )
            .into(),
        );
    }
    row(
        k,
        plain(
            None,
            [sp.s1 as f32, sp.s4 as f32, sp.s1 as f32, sp.s10 as f32],
            0.,
        ),
        sp.s3 as f32,
        c::Alignment::Center,
        cells,
    )
}
impl Epic {
    pub fn element(&self, key: &str) -> c::Element {
        let t = &STYLE.r#type;
        let sp = &STYLE.spacing;
        let mut children = vec![
            row(
                format!("{key}/ehead"),
                plain(
                    None,
                    [sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32],
                    STYLE.radius.row as f32,
                ),
                sp.s3 as f32,
                c::Alignment::Center,
                vec![
                    c::Item {
                        element: label(
                            format!("{key}/ehead/ecaret"),
                            if self.open == Some(false) {
                                "▸"
                            } else {
                                "▾"
                            },
                            text_style(t.micro as f32, 400, false, ui::Color::Muted),
                            false,
                        ),
                        extent: c::Extent::Fixed(10.),
                    },
                    kind_chip(&format!("{key}/ehead/ekind"), "epic").into(),
                    fill(clipped_text(
                        &format!("{key}/ehead/etitle"),
                        &self.title,
                        text_style(t.caption as f32, 600, false, ui::Color::Foreground),
                    )),
                    label(
                        format!("{key}/ehead/ecount"),
                        format!(
                            "{}/{}",
                            self.items
                                .iter()
                                .filter(|i| matches!(i.state.as_str(), "accepted" | "done"))
                                .count(),
                            self.items.len()
                        ),
                        text_style(t.small as f32, 400, true, ui::Color::Muted),
                        false,
                    )
                    .into(),
                ],
            )
            .into(),
        ];
        if self.open != Some(false) {
            for (i, item) in self.items.iter().enumerate() {
                let k = format!(
                    "{key}/eitem{}",
                    if i == 0 { String::new() } else { i.to_string() }
                );
                children.push(task_item(item, &k, self.delegate).into());
            }
        }
        column(
            key,
            plain(
                None,
                [sp.s1 as f32, 0., sp.s2 as f32, 0.],
                STYLE.radius.row as f32,
            ),
            children,
        )
    }
}
fn external(key: &str, events: &[External], open: bool) -> c::Element {
    let t = &STYLE.r#type;
    let sp = &STYLE.spacing;
    let mut children = vec![
        row(
            format!("{key}/ext-head"),
            plain(
                None,
                [sp.s2 as f32, sp.s4 as f32, sp.s2 as f32, sp.s4 as f32],
                STYLE.radius.row as f32,
            ),
            sp.s3 as f32,
            c::Alignment::Center,
            vec![
                label(
                    format!("{key}/ext-head/ecaret"),
                    if open { "▾" } else { "▸" },
                    text_style(t.caption as f32, 400, false, ui::Color::Foreground),
                    false,
                )
                .into(),
                label(
                    format!("{key}/ext-head/ekind"),
                    "external",
                    text_style(t.caption as f32, 400, false, ui::Color::Foreground),
                    false,
                )
                .into(),
                auto_left(label(
                    format!("{key}/ext-head/ext-count"),
                    format!("{} events", events.len()),
                    text_style(t.small as f32, 400, true, ui::Color::Muted),
                    false,
                )),
            ],
        )
        .into(),
    ];
    if open {
        for (i, event) in events.iter().rev().take(5).enumerate() {
            let key = format!(
                "{key}/ext-ev{}",
                if i == 0 { String::new() } else { i.to_string() }
            );
            let color = rgba(match event.source.as_str() {
                "github" => "#24292F",
                "ci" => "#2E6BFF",
                "cloud" => "#0EA5E9",
                "cron" => "#7C3AED",
                "webhook" => "#FF7A45",
                _ => "#374151",
            });
            let mut ts = text_style(STYLE.r#type.micro as f32, 400, true, ui::Color::OnAccent);
            ts.uppercase = true;
            ts.tracking_em = 0.06;
            let mut chip = plain(None, [1., 5., 1., 5.], TOKENS.radius.chip as f32);
            chip.overrides.fill = Some(ui::Paint::solid(color));
            children.push(
                row(
                    &key,
                    plain(
                        None,
                        [sp.s1 as f32, sp.s4 as f32, sp.s1 as f32, sp.s10 as f32],
                        0.,
                    ),
                    sp.s3 as f32,
                    c::Alignment::Center,
                    vec![
                        column(
                            format!("{key}/ext-src"),
                            chip,
                            vec![
                                label(format!("{key}/ext-src/text"), &event.source, ts, false)
                                    .into(),
                            ],
                        )
                        .into(),
                        fill(clipped_text(
                            &format!("{key}/ext-text"),
                            &event.text,
                            text_style(t.caption as f32, 400, false, ui::Color::Foreground),
                        )),
                        label(
                            format!("{key}/ext-time"),
                            &event.time,
                            text_style(t.small as f32, 400, true, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
    }
    column(
        key,
        plain(
            None,
            [sp.s1 as f32, 0., sp.s2 as f32, 0.],
            STYLE.radius.row as f32,
        ),
        children,
    )
}
impl StreamDock {
    pub fn items_at(&self, ctx: ui::Context, start: usize, m: &dyn c::Measure) -> Vec<c::Item> {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let mut children = vec![];
        for (i, entry) in self.entries.iter().enumerate() {
            let key = format!("e{}", i + start);
            children.push(
                match entry {
                    Entry::Message { data, show_channel } => {
                        data.element(ctx, &key, self.dock, show_channel.unwrap_or(true), m)
                    }
                    Entry::Epic { data } => data.element(&key),
                    Entry::Task { data, delegate } => task_item(data, &key, *delegate),
                    Entry::Report { data, review, .. } => data.element(ctx, &key, *review),
                    Entry::External { events, open } => external(&key, events, *open),
                    Entry::Filter { name } => row(
                        &key,
                        plain(None, [0., 0., sp.s2 as f32, sp.s3 as f32], 0.),
                        sp.s3 as f32,
                        c::Alignment::Center,
                        vec![
                            label(
                                format!("{key}/text"),
                                "from",
                                text_style(t.small as f32, 400, true, ui::Color::Muted),
                                false,
                            )
                            .into(),
                            label(
                                format!("{key}/b"),
                                name,
                                text_style(t.small as f32, 500, true, ui::Color::Foreground),
                                false,
                            )
                            .into(),
                            badge(
                                &format!("{key}/button"),
                                "× all",
                                text_style(t.small as f32, 400, true, ui::Color::Muted),
                                None,
                                Some(ui::Paint {
                                    color: ui::Color::Line,
                                    alpha: ui::Alpha::Hairline,
                                }),
                                (STYLE.radius.pill as f32, sp.s3 as f32, None, false),
                            )
                            .into(),
                        ],
                    ),
                    Entry::Note { text, padding } => column(
                        &key,
                        plain(None, *padding, 0.),
                        vec![
                            label(
                                format!("{key}/text"),
                                text,
                                text_style(t.row as f32, 400, false, ui::Color::Muted),
                                true,
                            )
                            .into(),
                        ],
                    ),
                    Entry::Tabs {
                        channels,
                        selected,
                        total,
                    } => {
                        let all = Channel {
                            name: "all".into(),
                            unread: total.map(UnreadCount::Exact).unwrap_or_else(|| {
                                channels
                                    .iter()
                                    .filter(|c| !c.muted)
                                    .try_fold(0u64, |sum, c| match c.unread {
                                        UnreadCount::Exact(n) => sum.checked_add(n),
                                        UnreadCount::More => None,
                                    })
                                    .map_or(UnreadCount::More, UnreadCount::Exact)
                            }),
                            muted: false,
                        };
                        let cells = std::iter::once(&all)
                            .chain(channels)
                            .enumerate()
                            .map(|(j, ch)| {
                                let k = format!(
                                    "{key}/ch-tab{}",
                                    if j == 0 { String::new() } else { j.to_string() }
                                );
                                let mut style = plain(
                                    None,
                                    [1., sp.s3 as f32, 1., sp.s3 as f32],
                                    STYLE.radius.pill as f32,
                                );
                                style.overrides.border = Some(ui::Border {
                                    width: ui::Length::Hairline,
                                    paint: if ch.name == *selected {
                                        ui::Paint {
                                            color: ui::Color::Line,
                                            alpha: ui::Alpha::Hairline,
                                        }
                                    } else {
                                        tint(ui::Color::Line, 0.)
                                    },
                                    outside: false,
                                });
                                if ch.muted {
                                    style.overrides.opacity = Some(0.55);
                                }
                                let col = if ch.name == *selected {
                                    ui::Color::Foreground
                                } else {
                                    ui::Color::Muted
                                };
                                if ch.name == *selected {
                                    style.overrides.fill = Some(ui::Paint {
                                        color: ui::Color::Foreground,
                                        alpha: ui::Alpha::Selection,
                                    });
                                }
                                let mut cells = vec![
                                    label(
                                        format!("{k}/text"),
                                        if ch.name == "all" {
                                            ch.name.clone()
                                        } else {
                                            format!("#{}", ch.name)
                                        },
                                        text_style(t.small as f32, 400, false, col),
                                        false,
                                    )
                                    .into(),
                                ];
                                if ch.muted {
                                    cells.push(
                                        named(
                                            format!("{k}/hicon"),
                                            c::Element::Icon {
                                                name: "muted".into(),
                                                paint: ui::Paint::solid(col),
                                                size: ui::Length::Px(10.),
                                            },
                                        )
                                        .into(),
                                    );
                                } else if ch.unread.any() {
                                    cells.push(
                                        badge(
                                            &format!("{k}/b"),
                                            &ch.unread.to_string(),
                                            ui::Text {
                                                italic: ch.unread == UnreadCount::More,
                                                tracking_em: if ch.unread == UnreadCount::More {
                                                    0.02
                                                } else {
                                                    0.
                                                },
                                                ..text_style(
                                                    t.micro as f32,
                                                    600,
                                                    false,
                                                    ui::Color::OnAccent,
                                                )
                                            },
                                            Some(ui::Paint::solid(ui::Color::Accent)),
                                            None,
                                            (STYLE.radius.pill as f32, 5., Some(13.), false),
                                        )
                                        .into(),
                                    );
                                }
                                row(k, style, sp.s1 as f32, c::Alignment::Center, cells).into()
                            })
                            .collect();
                        row(
                            &key,
                            plain(
                                None,
                                [sp.s1 as f32, sp.s1 as f32, sp.s2 as f32, sp.s1 as f32],
                                0.,
                            ),
                            sp.s1 as f32,
                            c::Alignment::Center,
                            cells,
                        )
                    }
                }
                .into(),
            );
        }
        children
    }
    pub fn content(&self, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
        self.content_from(ctx, 0, m)
    }
    /// Compose disjoint retained entry IDs when several streams share one tree.
    pub fn content_from(&self, ctx: ui::Context, start: usize, m: &dyn c::Measure) -> c::Element {
        column(
            "dock/rows",
            plain(None, [0.; 4], 0.),
            self.items_at(ctx, start, m),
        )
    }
    pub fn element(&self, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
        self.frame(ctx, self.items_at(ctx, 0, m))
    }
    pub fn frame(&self, ctx: ui::Context, children: Vec<c::Item>) -> c::Element {
        let sp = &STYLE.spacing;
        let mut frame = plain(
            None,
            if self.dock {
                [0.; 4]
            } else {
                [sp.s3 as f32; 4]
            },
            STYLE.radius.card as f32,
        );
        frame.overrides.fill = Some(ui::Paint::solid(if self.dock {
            ui::Color::Rgba([0., 0., 0., 0.])
        } else {
            ui::Color::Surface
        }));
        frame.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: if self.dock {
                ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                }
            } else {
                ui::Paint::solid(ui::Color::Line)
            },
            outside: false,
        });
        if self.dock {
            let mut dock = plain(
                None,
                [sp.s3 as f32, sp.s3 as f32, sp.s3 as f32, sp.s2 as f32],
                0.,
            );
            dock.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
            dock.overrides.border_sides = Some([true, false, false, false]);
            frame.overrides.fill = Some(ui::Paint::solid(rgba(ctx.theme.palette().sidebar)));
            if self.framed {
                let mut b = c::Box::new(frame, vec![column("dock", dock, children).into()]);
                b.direction = Axis::Column;
                b.gap = ui::Length::Px(0.);
                b.align = c::Alignment::Stretch;
                b.clip = true;
                named("frame", c::Element::Box(b))
            } else {
                named("frame", column("dock", dock, children))
            }
        } else {
            column("frame", frame, children)
        }
    }
}
