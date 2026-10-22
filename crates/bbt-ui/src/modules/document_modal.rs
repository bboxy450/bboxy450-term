//! Static document projection. The host owns parsing/indexing, files, selection,
//! navigation and feed/copy actions. This module only composes retained elements.
use super::{graph_tab::parse_links, seat_row::named, strip::plain};
use crate::{self as ui, components as c};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::Color as Rgba,
    layout::{Axis, TextStyle as MeasureStyle},
};
use serde::{Deserialize, Serialize};
mod diagram;
pub use diagram::{Diagram, FlowEdge, FlowGraph, FlowNode};

impl Diagram {
    /// Compose geometry already supplied by a document producer. This exposes
    /// the existing retained figure without adding a Mermaid parser to the UI.
    pub fn retained_element(
        &self,
        key: &str,
        width: f32,
        ctx: ui::Context,
        measure: &dyn c::Measure,
    ) -> c::Element {
        self.element(key, width, ctx, measure)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Provenance {
    pub created_by: String,
    pub updated_by: Option<String>,
    #[serde(rename = "createdColor")]
    pub created_color: Option<String>,
    #[serde(rename = "updatedColor")]
    pub updated_color: Option<String>,
    pub derived_from: Option<String>,
    pub digest: String,
    pub signed: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Neighbour {
    pub slug: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub rel: String,
    pub w: Option<f32>,
    pub back: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub id: String,
    pub path: String,
    pub title: String,
    pub status: Option<String>,
    pub assignee: Option<String>,
    #[serde(rename = "assigneeColor")]
    pub assignee_color: Option<String>,
    pub role: Option<String>,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub depends_on: Vec<String>,
    pub links: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub verify: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub superseded_by: Option<String>,
    pub broken: Option<String>,
    pub body: String,
    pub prov: Provenance,
    pub near: Vec<Neighbour>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentModal {
    pub document: Document,
    pub modal: bool,
    pub seat_name: Option<String>,
    pub viewport: [f32; 2],
    pub scroll: [f32; 2],
    /// Semantic diagram records supplied by the document producer, not SVGs.
    pub diagrams: Vec<Diagram>,
}
impl Default for DocumentModal {
    fn default() -> Self {
        Self {
            document: Document::default(),
            modal: false,
            seat_name: None,
            viewport: [1280., 960.],
            scroll: [0.; 2],
            diagrams: vec![],
        }
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
fn text(k: impl Into<String>, v: impl Into<String>, f: ui::Text) -> c::Element {
    named(k, c::Element::Text(c::Text::new(v, f)))
}
fn nowrap(k: impl Into<String>, v: impl Into<String>, f: ui::Text) -> c::Element {
    let mut t = c::Text::new(v, f);
    t.wrap = false;
    text_element(k, t)
}
fn text_element(k: impl Into<String>, t: c::Text) -> c::Element {
    named(k, c::Element::Text(t))
}
fn row(k: impl Into<String>, style: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.gap = ui::Length::Px(gap);
    named(k, c::Element::Box(b))
}
fn col(k: impl Into<String>, style: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = Axis::Column;
    b.align = c::Alignment::Stretch;
    b.gap = ui::Length::Px(gap);
    named(k, c::Element::Box(b))
}
fn fixed(element: c::Element, size: f32) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Fixed(size),
    }
}
fn fill(element: c::Element) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Fill,
    }
}
fn natural(element: c::Element) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Content {
            grow: 0.,
            shrink: 0.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn hair(ctx: ui::Context) -> ui::Color {
    let mut v = Rgba::hex(ctx.theme.palette().line).0;
    v[3] *= STYLE.material.hairline_alpha as f32;
    ui::Color::Rgba(v)
}
fn color(s: &str) -> ui::Color {
    ui::Color::Rgba(Rgba::hex(s).0)
}
fn optional_color(s: Option<&str>) -> ui::Color {
    s.map(color).unwrap_or(ui::Color::Foreground)
}
fn surface(fill: ui::Color, alpha: f32, radius: f32, pad: [f32; 4]) -> ui::Style {
    let mut s = plain(None, pad, radius);
    s.overrides.fill = Some(paint(fill, alpha));
    s
}
fn border(
    mut s: ui::Style,
    co: ui::Color,
    width: f32,
    sides: [bool; 4],
    outside: bool,
) -> ui::Style {
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(width),
        paint: ui::Paint::solid(co),
        outside,
    });
    s.overrides.border_sides = Some(sides);
    s
}
fn width(m: &dyn c::Measure, v: &str, f: ui::Text) -> f32 {
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
fn size(e: &c::Element, w: f32, ctx: ui::Context, m: &dyn c::Measure) -> [f32; 2] {
    e.natural(w, ctx, m).expect("document component geometry").0
}
fn space(h: f32) -> c::Item {
    fixed(c::Element::Spacer, h)
}
fn flow(
    k: impl Into<String>,
    children: Vec<c::Item>,
    row_gap: f32,
    gap: f32,
    align: c::Alignment,
) -> c::Element {
    let mut f = c::Flow::new(children);
    f.row_gap = ui::Length::Px(row_gap);
    f.column_gap = ui::Length::Px(gap);
    f.align = align;
    named(k, c::Element::Flow(f))
}
fn hue(kind: &str) -> ui::Color {
    color(match kind {
        "epic" => TOKENS.color.doc.epic,
        "spec" => TOKENS.color.doc.spec,
        "task" => TOKENS.color.doc.task,
        "bug" => TOKENS.color.doc.bug,
        "note" => TOKENS.color.doc.note,
        "invariant" => TOKENS.color.doc.invariant,
        "artifact" => TOKENS.color.doc.artifact,
        _ => TOKENS.color.ui.ok,
    })
}

struct Build<'a> {
    ctx: ui::Context,
    m: &'a dyn c::Measure,
    diagrams: &'a [Diagram],
    body: Option<&'a dyn Fn(f32) -> c::Element>,
}
// A cell's public text, code-font flag and assignee-colour flag.
type Cells = Vec<(String, bool, bool)>;
impl Build<'_> {
    fn link(&self, key: &str, src: &str, parent: ui::Text) -> c::Element {
        let l = parse_links(src).into_iter().next().expect("parsed link");
        let slug = nowrap(
            format!("{key}/slug"),
            l.display,
            ui::Text {
                line_height: 16. / 11.,
                ..font(11., 400, false, color(self.ctx.theme.palette().accent_soft))
            },
        );
        let mut all = vec![
            row(
                format!("{key}/slug-box"),
                surface(ui::Color::Accent, 0.1, 0., [0., 6., 0., 6.]),
                0.,
                vec![slug.into()],
            )
            .into(),
        ];
        if let Some(rel) = l.relation {
            let co = match rel.as_str() {
                "violates" => color(TOKENS.color.state.errored),
                "constrains" => color(TOKENS.color.state.waiting),
                "derives" | "derived_from" => color(self.ctx.theme.palette().accent_soft),
                _ => ui::Color::Muted,
            };
            let value = format!(
                "{rel}{}",
                l.weight.map(|v| format!(" {v}")).unwrap_or_default()
            );
            all.push(
                row(
                    format!("{key}/rel-box"),
                    surface(
                        ui::Color::Muted,
                        STYLE.tint.badge_alpha as f32,
                        0.,
                        [0., 5., 0., 5.],
                    ),
                    0.,
                    vec![
                        nowrap(
                            format!("{key}/rel"),
                            value,
                            ui::Text {
                                line_height: 16. / 9.5,
                                ..font(9.5, 400, true, co)
                            },
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        let mut b = c::Box::new(
            border(
                plain(None, [0.; 4], STYLE.radius.tag as f32),
                hair(self.ctx),
                1.,
                [true; 4],
                true,
            ),
            all,
        );
        b.gap = ui::Length::Px(0.);
        b.clip = true;
        let chip = c::Element::InlineMiddle {
            content: Box::new(named(format!("{key}/chip"), c::Element::Box(b))),
            parent,
        };
        let mut wrap = c::Box::new(plain(None, [0., 2., 0., 2.], 0.), vec![chip.into()]);
        wrap.inline = true;
        wrap.align = c::Alignment::Baseline;
        wrap.gap = ui::Length::Px(0.);
        wrap.style.overrides.text = Some(parent);
        named(key, c::Element::Box(wrap))
    }
    fn rich(&self, key: &str, value: &str, f: ui::Text) -> c::Element {
        if !value.contains("[[") {
            return text(key, value, f);
        }
        let mut all = vec![];
        let mut rest = value;
        let mut i = 0;
        while let Some(at) = rest.find("[[") {
            for word in rest[..at].split_inclusive(' ').filter(|s| !s.is_empty()) {
                all.push(natural(nowrap(format!("{key}/word{i}"), word, f)));
                i += 1;
            }
            let Some(end) = rest[at + 2..].find("]]") else {
                break;
            };
            let mut len = at + end + 4;
            if rest[len..].starts_with('{')
                && let Some(n) = rest[len..].find('}')
            {
                len += n + 1;
            }
            all.push(natural(self.link(
                &format!("{key}/link{i}"),
                &rest[at..len],
                f,
            )));
            i += 1;
            rest = &rest[len..];
        }
        for word in rest.split_inclusive(' ').filter(|s| !s.is_empty()) {
            all.push(natural(nowrap(format!("{key}/word{i}"), word, f)));
            i += 1;
        }
        flow(key, all, 0., 0., c::Alignment::Baseline)
    }
    fn heading(&self, d: &Document, plain_doc: bool) -> c::Element {
        let h = d.kind.as_deref().map(hue).unwrap_or(ui::Color::Accent);
        let mut entries = vec![];
        if !plain_doc {
            let mut tf = font(10., 700, false, h);
            tf.tracking_em = 0.14;
            tf.uppercase = true;
            entries.push(natural(row(
                "type",
                surface(h, 0.16, STYLE.radius.tag as f32, [2., 7., 2., 7.]),
                0.,
                vec![nowrap("type/text", d.kind.as_deref().unwrap_or(""), tf).into()],
            )));
            let st = d.status.as_deref().unwrap_or("current");
            let co = match st {
                "done" | "accepted" | "decided" | "fixed" | "active" => {
                    color(TOKENS.color.state.idle)
                }
                "blocked" | "rejected" => color(TOKENS.color.state.errored),
                "superseded" | "retired" | "wontfix" | "duplicate" => ui::Color::Muted,
                _ => ui::Color::Foreground,
            };
            let bc = if matches!(st, "done" | "accepted" | "decided" | "fixed" | "active") {
                ui::Color::Rgba(paint(co, 0.45).resolve(self.ctx).unwrap())
            } else if matches!(st, "blocked" | "rejected") {
                ui::Color::Rgba(paint(co, 0.5).resolve(self.ctx).unwrap())
            } else {
                ui::Color::Line
            };
            let prefix = if d.kind.as_deref() == Some("task") {
                match st {
                    "running" => "◑ ",
                    "reported" => "◕ ",
                    "accepted" => "● ",
                    "done" => "✓ ",
                    "claimed" => "◔ ",
                    "blocked" => "⊘ ",
                    "rejected" => "✕ ",
                    _ => "○ ",
                }
            } else {
                ""
            };
            let value = format!("{prefix}{st}");
            let status_font = font(11., 400, false, co);
            let mut status = nowrap("status/text", &value, status_font);
            if matches!(st, "superseded" | "retired" | "wontfix" | "duplicate") {
                let [w, h] = size(&status, f32::INFINITY, self.ctx, self.m);
                let baseline = status.natural(w, self.ctx, self.m).unwrap().1;
                let x_height = self.m.x_height(&MeasureStyle {
                    size: 11.,
                    weight: 400,
                    mono: false,
                    italic: false,
                    tracking_em: 0.,
                });
                let y = baseline - x_height * 0.5;
                status = c::Element::Layers {
                    base: Box::new(status),
                    overlays: vec![c::Placed {
                        left: 0.,
                        bottom: 0.,
                        content: path(
                            "status/strike",
                            [w, h],
                            vec![c::PathCommand::Move([0., y]), c::PathCommand::Line([w, y])],
                            1.,
                            ui::Paint::solid(co),
                            vec![],
                        ),
                    }],
                };
            }
            entries.push(natural(row(
                "status",
                border(
                    plain(None, [1., 8., 1., 8.], 999.),
                    bc,
                    1.,
                    [true; 4],
                    false,
                ),
                0.,
                vec![status.into()],
            )));
            entries.push(fill(c::Element::Spacer));
        }
        entries.push(natural(nowrap(
            "path",
            &d.path,
            font(10.5, 400, true, ui::Color::Muted),
        )));
        let top = flow("top", entries, 8., 8., c::Alignment::Center);
        let mut title = font(19., 650, false, ui::Color::Foreground);
        title.tracking_em = -0.01;
        let mut children = vec![
            top.into(),
            space(6.),
            text(
                "title",
                if d.title.is_empty() {
                    &d.path
                } else {
                    &d.title
                },
                title,
            )
            .into(),
        ];
        if let Some(src) = &d.superseded_by {
            children.push(space(8.));
            children.push(
                col(
                    "super",
                    surface(
                        ui::Color::Muted,
                        0.1,
                        STYLE.radius.row as f32,
                        [6., 10., 6., 10.],
                    ),
                    0.,
                    vec![
                        self.rich(
                            "super/text",
                            &format!("superseded by {src} · kept for provenance"),
                            font(12., 400, false, ui::Color::Muted),
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        if let Some(error) = &d.broken {
            children.push(space(8.));
            let mut s = border(
                plain(None, [6., 10., 6., 10.], STYLE.radius.row as f32),
                ui::Color::Rgba(
                    paint(color(TOKENS.color.state.errored), 0.6)
                        .resolve(self.ctx)
                        .unwrap(),
                ),
                1.,
                [true; 4],
                false,
            );
            s.overrides.dashed = Some(true);
            children.push(
                col(
                    "broken",
                    s,
                    0.,
                    vec![
                        text(
                            "broken/text",
                            format!("frontmatter does not parse · {error} · shown, never skipped"),
                            font(12., 400, false, color(TOKENS.color.state.errored)),
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        let s = plain(None, [2., 0., 2., 12.], 0.);
        col(
            "head",
            if plain_doc {
                s
            } else {
                border(s, h, 3., [false, false, false, true], false)
            },
            0.,
            children,
        )
    }
    fn frontmatter(&self, d: &Document, w: f32) -> c::Element {
        let mut fields: Vec<(&str, Cells)> = vec![];
        fields.push(("id", vec![(d.id.clone(), true, false)]));
        if let Some(v) = &d.assignee {
            fields.push(("assignee", vec![(v.clone(), false, true)]))
        }
        for (k, v) in [("role", &d.role), ("parent", &d.parent)] {
            if let Some(v) = v {
                fields.push((k, vec![(v.clone(), false, false)]))
            }
        }
        for (k, v, code) in [
            ("children", &d.children, false),
            ("depends_on", &d.depends_on, false),
            ("links", &d.links, false),
            ("allowed_paths", &d.allowed_paths, true),
        ] {
            if !v.is_empty() {
                fields.push((k, v.iter().map(|s| (s.clone(), code, false)).collect()))
            }
        }
        for (k, v, code) in [
            ("verify", &d.verify, true),
            ("created", &d.created, false),
            ("updated", &d.updated, false),
        ] {
            if let Some(v) = v {
                fields.push((k, vec![(v.clone(), code, false)]))
            }
        }
        let key_font = font(10.5, 400, true, ui::Color::Muted);
        let key_width = fields
            .iter()
            .map(|(k, _)| width(self.m, k, key_font))
            .fold(0., f32::max);
        let value_width = (w - 26. - 14. - key_width).max(0.);
        let mut rows = vec![];
        for (i, (label, values)) in fields.into_iter().enumerate() {
            let mut children = vec![];
            for (j, (v, code, seat)) in values.into_iter().enumerate() {
                let f = font(
                    if code { 10.5 } else { 12. },
                    if seat { 600 } else { 400 },
                    code,
                    if seat {
                        optional_color(d.assignee_color.as_deref())
                    } else {
                        ui::Color::Foreground
                    },
                );
                let k = format!("fm{i}/value{j}");
                let e = if v.contains("[[") {
                    self.rich(&k, &v, f)
                } else {
                    text(&k, v, f)
                };
                children.push(e.into());
            }
            let cells = flow(
                format!("fm{i}/cells"),
                children,
                4.,
                6.,
                c::Alignment::Center,
            );
            // Grid cells stretch to the key's padded height. Centre the flex
            // line within that cell, including shorter code-only values.
            let value = row(
                format!("fm{i}/value"),
                plain(None, [0.; 4], 0.),
                0.,
                vec![fill(cells)],
            );
            let key = col(
                format!("fm{i}/key"),
                plain(None, [2., 0., 0., 0.], 0.),
                0.,
                vec![nowrap(format!("fm{i}/key/text"), label, key_font).into()],
            );
            let mut s = c::Stack {
                direction: Axis::Row,
                gap: ui::Length::Px(14.),
                align: c::Alignment::Stretch,
                children: vec![fixed(key, key_width), fixed(value, value_width)],
            };
            s.align = c::Alignment::Stretch;
            rows.push(c::Element::Stack(s).into());
        }
        col(
            "frontmatter",
            border(
                surface(
                    ui::Color::Surface,
                    0.7,
                    STYLE.radius.row as f32,
                    [10., 12., 10., 12.],
                ),
                hair(self.ctx),
                1.,
                [true; 4],
                false,
            ),
            4.,
            rows,
        )
    }
    fn markdown(&self, source: &str, w: f32) -> c::Element {
        // This deliberately small fixture syntax is not the future bbt-doc parser.
        let body_font = ui::Text {
            line_height: 1.6,
            ..font(13., 400, false, ui::Color::Foreground)
        };
        let limit = (STYLE.spacing.doc_measure as f32).min(w);
        let mut blocks: Vec<(c::Element, f32, f32)> = vec![];
        let mut lines = source.lines().peekable();
        let mut index = 0;
        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
                continue;
            }
            let key = format!("block{index}");
            index += 1;
            if line.starts_with("```") {
                let mermaid = line.contains("mermaid");
                let mut code = vec![];
                for line in lines.by_ref() {
                    if line.starts_with("```") {
                        break;
                    }
                    code.push(line)
                }
                let src = code.join("\n");
                let e = if mermaid {
                    self.diagram(&key, &src, limit)
                } else {
                    let mut f = font(11., 400, true, ui::Color::Foreground);
                    f.line_height = 1.6;
                    col(
                        &key,
                        border(
                            surface(
                                ui::Color::Surface,
                                1.,
                                STYLE.radius.row as f32,
                                [8., 10., 8., 10.],
                            ),
                            hair(self.ctx),
                            1.,
                            [true; 4],
                            false,
                        ),
                        0.,
                        vec![nowrap(format!("{key}/text"), src, f).into()],
                    )
                };
                blocks.push((
                    named(key, e),
                    if mermaid { 0. } else { 11. },
                    if mermaid { 0. } else { 11. },
                ));
                continue;
            }
            let hashes = line.chars().take_while(|c| *c == '#').count();
            if (1..=3).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
                blocks.push((
                    self.rich(
                        &key,
                        &line[hashes + 1..],
                        ui::Text {
                            line_height: 1.6,
                            ..font(13.5, 700, false, ui::Color::Foreground)
                        },
                    ),
                    14.,
                    4.,
                ));
                continue;
            }
            if line.starts_with("- ") || line.starts_with("* ") {
                let mut values = vec![&line[2..]];
                while lines
                    .peek()
                    .is_some_and(|l| l.starts_with("- ") || l.starts_with("* "))
                {
                    values.push(&lines.next().unwrap()[2..])
                }
                let mut items = vec![];
                for (i, v) in values.into_iter().enumerate() {
                    let child = self.rich(&format!("{key}/li{i}"), v, body_font);
                    let bullet = nowrap(format!("{key}/bullet{i}"), "•", body_font);
                    let h = size(&child, (limit - 20.).max(0.), self.ctx, self.m)[1];
                    items.push(
                        c::Element::Layers {
                            base: Box::new(child),
                            overlays: vec![c::Placed {
                                content: bullet,
                                left: -12.,
                                bottom: h - 20.796875,
                            }],
                        }
                        .into(),
                    );
                }
                blocks.push((
                    col(key, plain(None, [0., 0., 0., 20.], 0.), 0., items),
                    4.,
                    10.,
                ));
                continue;
            }
            blocks.push((self.rich(&key, line, body_font), 4., 8.));
        }
        let mut all = vec![];
        let mut bottom = 0.;
        for (i, (e, top, btm)) in blocks.into_iter().enumerate() {
            if i > 0 {
                all.push(space(f32::max(bottom, top)))
            }
            all.push(e.into());
            bottom = btm;
        }
        let mut style = plain(None, [0.; 4], 0.);
        style.overrides.width = Some(ui::Length::Px(limit));
        row(
            "body-slot",
            plain(None, [0.; 4], 0.),
            0.,
            vec![fixed(col("body", style, 0., all), limit)],
        )
    }
    fn diagram(&self, key: &str, source: &str, w: f32) -> c::Element {
        if let Some(diagram) = self.diagrams.iter().find(|d| d.source == source) {
            return diagram.element(key, w, self.ctx, self.m);
        }
        col(
            key,
            plain(None, [0.; 4], 0.),
            0.,
            vec![
                text(
                    format!("{key}/missing"),
                    "diagram record unavailable · shown as source",
                    font(11., 400, false, color(TOKENS.color.state.waiting)),
                )
                .into(),
                text(
                    format!("{key}/source"),
                    source,
                    font(11., 400, true, ui::Color::Foreground),
                )
                .into(),
            ],
        )
    }
    fn provenance(&self, d: &Document, w: f32) -> c::Element {
        let p = &d.prov;
        let base = font(11.5, 400, false, ui::Color::Muted);
        let span = |key: &str, prefix: &str, name: &str, co: Option<&str>| {
            let mut t = c::Text::new(prefix, base);
            t.runs.push(c::TextRun {
                value: name.into(),
                style: font(11.5, 600, false, optional_color(co)),
            });
            text_element(key, t)
        };
        let mut entries = vec![natural(span(
            "provenance/created",
            "created by ",
            if p.created_by.is_empty() {
                "unknown"
            } else {
                &p.created_by
            },
            p.created_color.as_deref(),
        ))];
        if let Some(v) = &p.updated_by {
            entries.push(natural(span(
                "provenance/updated",
                "updated by ",
                v,
                p.updated_color.as_deref(),
            )))
        }
        if let Some(v) = &p.derived_from {
            entries.push(natural(self.rich(
                "provenance/derived",
                &format!("derived from {v}"),
                base,
            )))
        }
        let sig = p
            .signed
            .as_ref()
            .map(|s| format!("✓ signed · {s}"))
            .unwrap_or_else(|| "unsigned · no write is anonymous: updated_by stamped".into());
        let dig = row(
            "provenance/digest",
            plain(None, [0.; 4], 0.),
            8.,
            vec![
                natural(nowrap(
                    "provenance/hash",
                    p.digest.chars().take(12).collect::<String>(),
                    font(10.5, 400, true, ui::Color::Muted),
                )),
                text(
                    "provenance/signed",
                    sig,
                    font(
                        11.,
                        400,
                        false,
                        color(if p.signed.is_some() {
                            TOKENS.color.state.idle
                        } else {
                            TOKENS.color.state.waiting
                        }),
                    ),
                )
                .into(),
            ],
        );
        let used = entries
            .iter()
            .map(|e| size(&e.element, w, self.ctx, self.m)[0])
            .sum::<f32>()
            + 16. * entries.len().saturating_sub(1) as f32;
        let dw = size(&dig, w, self.ctx, self.m)[0];
        let mut children = vec![];
        if used + 16. + dw > w {
            children
                .push(flow("provenance/authors", entries, 6., 16., c::Alignment::Center).into());
            children.push(space(6.));
            children.push(
                row(
                    "provenance/digest-row",
                    plain(None, [0.; 4], 0.),
                    0.,
                    vec![fill(c::Element::Spacer), natural(dig)],
                )
                .into(),
            );
        } else {
            entries.push(fill(c::Element::Spacer));
            entries.push(natural(dig));
            children.push(row("provenance/line", plain(None, [0.; 4], 0.), 16., entries).into());
        }
        col(
            "provenance",
            border(
                plain(None, [10., 0., 0., 0.], 0.),
                hair(self.ctx),
                1.,
                [true, false, false, false],
                false,
            ),
            0.,
            children,
        )
    }
    fn rail(&self, d: &Document) -> c::Element {
        let mut hf = font(10., 400, false, ui::Color::Muted);
        hf.tracking_em = 0.12;
        hf.uppercase = true;
        let mut children = vec![
            space(2.),
            text("rail/h0", "markdown graph · neighbourhood", hf).into(),
            space(6.),
            row(
                "rail/graph-slot",
                plain(None, [0.; 4], 0.),
                0.,
                vec![fixed(self.neighbourhood(d), 260.)],
            )
            .into(),
            space(4.),
        ];
        let mut legend = vec![];
        for kind in [
            "epic",
            "spec",
            "task",
            "bug",
            "note",
            "invariant",
            "artifact",
        ] {
            let mut dot_style = plain(Some(7.), [0.; 4], 3.5);
            dot_style.overrides.width = Some(ui::Length::Px(7.));
            dot_style.overrides.fill = Some(ui::Paint::solid(hue(kind)));
            legend.push(
                row(
                    format!("rail/key/{kind}"),
                    plain(None, [0.; 4], 0.),
                    4.,
                    vec![
                        row(format!("rail/key/{kind}/dot"), dot_style, 0., vec![]).into(),
                        nowrap(
                            format!("rail/key/{kind}/text"),
                            kind,
                            font(10., 400, false, ui::Color::Muted),
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        children.push(flow("rail/legend", legend, 4., 10., c::Alignment::Center).into());
        children.push(space(14.));
        let backs: Vec<_> = d.near.iter().filter(|n| n.back).collect();
        if !backs.is_empty() {
            children.push(space(0.));
            children.push(text("rail/h1", "backlinks", hf).into());
            children.push(space(6.));
        }
        for (i, n) in backs.iter().enumerate() {
            let mut s = surface(hue(&n.kind), 1., 3.5, [0.; 4]);
            s.overrides.height = Some(ui::Length::Px(7.));
            s.overrides.width = Some(ui::Length::Px(7.));
            children.push(
                row(
                    format!("rail/back{i}"),
                    plain(None, [2., 0., 2., 0.], 0.),
                    4.,
                    vec![
                        row(format!("rail/back{i}/dot"), s, 0., vec![]).into(),
                        fill(nowrap(
                            format!("rail/back{i}/slug"),
                            &n.slug,
                            font(10.5, 400, true, ui::Color::Foreground),
                        )),
                        nowrap(
                            format!("rail/back{i}/rel"),
                            if n.rel.is_empty() { "ref" } else { &n.rel },
                            font(10., 400, false, ui::Color::Muted),
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        col(
            "rail",
            border(
                plain(None, [0., 0., 0., 16.], 0.),
                hair(self.ctx),
                1.,
                [false, false, false, true],
                false,
            ),
            0.,
            children,
        )
    }
    fn neighbourhood(&self, d: &Document) -> c::Element {
        let (w, h) = (260., 230.);
        let mut overlays = vec![];
        let mut place = |e: c::Element, x: f32, y: f32| {
            let sz = size(&e, f32::INFINITY, self.ctx, self.m);
            overlays.push(c::Placed {
                content: e,
                left: x,
                bottom: h - y - sz[1],
            })
        };
        for (i, n) in d.near.iter().enumerate() {
            let angle = -std::f32::consts::FRAC_PI_2
                + i as f32 * std::f32::consts::TAU / d.near.len() as f32;
            let r = 78. - n.w.unwrap_or(1.) * 6.;
            let (x, y) = (130. + angle.cos() * r, 109. + angle.sin() * r);
            place(
                path(
                    "rail/link",
                    [w, h],
                    vec![
                        c::PathCommand::Move([130., 109.]),
                        c::PathCommand::Line([x, y]),
                    ],
                    0.8 + n.w.unwrap_or(1.) * 0.7,
                    paint(ui::Color::Muted, 0.45),
                    if n.back { vec![3., 3.] } else { vec![] },
                ),
                0.,
                0.,
            );
            let mut s = surface(hue(&n.kind), 1., 5., [0.; 4]);
            s.overrides.height = Some(ui::Length::Px(10.));
            s.overrides.width = Some(ui::Length::Px(10.));
            place(row("rail/dot", s, 0., vec![]), x - 5., y - 5.);
            let f = font(9.5, 400, true, ui::Color::Foreground);
            let e = nowrap("rail/node-label", &n.slug, f);
            let sz = size(&e, 999., self.ctx, self.m);
            let b = e.natural(999., self.ctx, self.m).unwrap().1;
            place(e, x - sz[0] / 2., y + if y < 109. { -9. } else { 15. } - b);
            if !n.rel.is_empty() {
                let value = format!(
                    "{}{}",
                    n.rel,
                    n.w.filter(|v| *v > 1.)
                        .map(|v| format!(" {v}"))
                        .unwrap_or_default()
                );
                let e = nowrap(
                    "rail/edge-label",
                    &value,
                    font(9., 400, false, ui::Color::Muted),
                );
                let sz = size(&e, 999., self.ctx, self.m);
                let b = e.natural(999., self.ctx, self.m).unwrap().1;
                place(e, (130. + x) / 2. - sz[0] / 2., (109. + y) / 2. - 3. - b);
            }
        }
        let mut s = border(
            surface(
                d.kind.as_deref().map(hue).unwrap_or(ui::Color::Accent),
                1.,
                9.,
                [0.; 4],
            ),
            ui::Color::Background,
            2.,
            [true; 4],
            true,
        );
        s.overrides.height = Some(ui::Length::Px(18.));
        s.overrides.width = Some(ui::Length::Px(18.));
        place(row("rail/center", s, 0., vec![]), 121., 100.);
        let mut base = plain(Some(h), [0.; 4], 0.);
        base.overrides.width = Some(ui::Length::Px(w));
        named(
            "rail/graph",
            c::Element::Layers {
                base: Box::new(row("rail/canvas", base, 0., vec![])),
                overlays,
            },
        )
    }
    fn view(&self, d: &Document, w: f32, modal: bool) -> c::Element {
        let plain_doc = d.kind.is_none();
        let rail = !plain_doc;
        let main_w = (w - 36. - if rail { 298. } else { 0. }).max(0.);
        let mut children = vec![self.heading(d, plain_doc).into(), space(12.)];
        if !plain_doc {
            children.push(self.frontmatter(d, main_w).into());
            children.push(space(14.));
        }
        let md = match self.body {
            Some(build) => build(main_w),
            None => self.markdown(&d.body, main_w),
        };
        children.push(md.into());
        if !plain_doc {
            children.push(space(14.));
            children.push(self.provenance(d, main_w).into());
        } else {
            children.push(space(10.));
        }
        let main = col("main", plain(None, [0.; 4], 0.), 0., children);
        let mut parts = vec![fixed(main, main_w)];
        if rail {
            parts.push(fixed(self.rail(d), 280.));
        }
        let mut root = c::Box::new(
            surface(
                if modal {
                    ui::Color::Transparent
                } else {
                    ui::Color::Background
                },
                1.,
                if modal { 0. } else { STYLE.radius.card as f32 },
                [18.; 4],
            ),
            parts,
        );
        root.gap = ui::Length::Px(18.);
        root.align = c::Alignment::Stretch;
        named("view", c::Element::Box(root))
    }
    fn footer(&self, seat: Option<&str>) -> c::Element {
        let labels = [
            format!("Feed into {}", seat.unwrap_or("the focused seat")),
            "Feed selection".into(),
            "Copy as Markdown".into(),
            "Copy path".into(),
            "Find".into(),
            "Pin as tab".into(),
            "Close".into(),
        ];
        let keys = ["⏎", "⌘⏎", "⌘C", "⌘⇧C", "⌘F", "", "esc"];
        let mut actions = vec![];
        for (i, (value, keys)) in labels.iter().zip(keys).enumerate() {
            if i == 2 || i == 5 {
                let mut s = surface(hair(self.ctx), 1., 0., [0.; 4]);
                s.overrides.height = Some(ui::Length::Px(16.));
                s.overrides.width = Some(ui::Length::Px(1.));
                actions.push(
                    row(
                        format!("sep{i}"),
                        plain(None, [0., 4., 0., 4.], 0.),
                        0.,
                        vec![row("sep/line", s, 0., vec![]).into()],
                    )
                    .into(),
                );
            }
            let mut child = vec![
                nowrap(
                    format!("action{i}/label"),
                    value,
                    font(
                        12.,
                        if i == 0 { 600 } else { 400 },
                        false,
                        if i == 0 {
                            ui::Color::OnAccent
                        } else if i >= 5 {
                            ui::Color::Muted
                        } else {
                            ui::Color::Foreground
                        },
                    ),
                )
                .into(),
            ];
            if !keys.is_empty() {
                let kstyle = if i == 0 {
                    surface(
                        ui::Color::Rgba([1.; 4]),
                        0.22,
                        TOKENS.radius.chip as f32,
                        [0., 5., 0., 5.],
                    )
                } else {
                    surface(
                        ui::Color::Muted,
                        STYLE.tint.badge_alpha as f32,
                        TOKENS.radius.chip as f32,
                        [0., 5., 0., 5.],
                    )
                };
                child.push(
                    row(
                        format!("action{i}/key"),
                        kstyle,
                        0.,
                        vec![
                            nowrap(
                                format!("action{i}/key/text"),
                                keys,
                                font(
                                    10.5,
                                    if i == 0 { 600 } else { 400 },
                                    false,
                                    if i == 0 {
                                        ui::Color::OnAccent
                                    } else {
                                        ui::Color::Muted
                                    },
                                ),
                            )
                            .into(),
                        ],
                    )
                    .into(),
                )
            }
            actions.push(natural(row(
                format!("action{i}"),
                if i == 0 {
                    surface(
                        ui::Color::Accent,
                        1.,
                        STYLE.radius.row as f32,
                        [4., 8., 4., 8.],
                    )
                } else {
                    plain(None, [4., 8., 4., 8.], STYLE.radius.row as f32)
                },
                6.,
                child,
            )));
        }
        col(
            "footer",
            border(
                surface(ui::Color::Surface, 0.6, 0., [8., 12., 8., 12.]),
                hair(self.ctx),
                1.,
                [true, false, false, false],
                false,
            ),
            0.,
            vec![flow("footer/actions", actions, 4., 4., c::Alignment::Center).into()],
        )
    }
}
fn path(
    key: &str,
    sz: [f32; 2],
    points: Vec<c::PathCommand>,
    width: f32,
    paint: ui::Paint,
    dash: Vec<f32>,
) -> c::Element {
    named(
        key,
        c::Element::Path(c::Path {
            size: sz,
            points: points.into(),
            stroke: c::PathStroke {
                width,
                dash: dash.into(),
                cap: c::LineCap::Butt,
                join: c::LineJoin::Miter,
            },
            paint,
            fill: None,
            opacity: 1.,
        }),
    )
}
impl DocumentModal {
    /// Only the enclosing viewport and native font host participate in layout.
    pub fn element(&self, ctx: ui::Context, m: &dyn c::Measure, available: [f32; 2]) -> c::Element {
        self.compose(ctx, m, available, None)
    }
    /// The document producer builds the body at the shell's actual inner width.
    /// Header, metadata, provenance, rail, modal and scrolling retain one owner.
    pub fn element_with_body(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        available: [f32; 2],
        body: &dyn Fn(f32) -> c::Element,
    ) -> c::Element {
        self.compose(ctx, m, available, Some(body))
    }
    fn compose(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        available: [f32; 2],
        body: Option<&dyn Fn(f32) -> c::Element>,
    ) -> c::Element {
        let b = Build {
            ctx,
            m,
            diagrams: &self.diagrams,
            body,
        };
        if !self.modal {
            return named("document", b.view(&self.document, available[0], false));
        }
        let w = 980_f32.min(self.viewport[0] * 0.94).min(available[0]);
        let h = (self.viewport[1] * 0.82).min(available[1]);
        let foot = b.footer(self.seat_name.as_deref());
        let fh = size(&foot, w, ctx, m)[1];
        let view = b.view(&self.document, w, true);
        let vh = size(&view, w, ctx, m)[1];
        let content = col(
            "scroll-content",
            plain(None, [0.; 4], 0.),
            0.,
            vec![fixed(view, vh)],
        );
        let body = named(
            "scroller",
            c::Element::ScrollView(c::ScrollView {
                viewport: [w, (h - fh).max(0.)],
                offset: self.scroll,
                content_width: None,
                content: Box::new(content),
            }),
        );
        let mut s = ui::Style::new(ui::Preset::Modal);
        s.overrides.padding = Some([ui::Length::Px(0.); 4]);
        s.overrides.height = Some(ui::Length::Px(h));
        s.overrides.width = Some(ui::Length::Px(w));
        let mut panel = c::Box::new(s, vec![body.into(), foot.into()]);
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Stretch;
        panel.gap = ui::Length::Px(0.);
        panel.clip = true;
        named("document", c::Element::Box(panel))
    }
}
