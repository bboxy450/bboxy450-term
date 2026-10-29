// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained composition only. All measurements use the host's real text pipeline.
use super::*;
use bbt_ui::{
    self as ui, components as c,
    modules::document_modal::{Diagram, Document, DocumentModal, Provenance},
};
use bbt_ui_model::{
    STYLE, TOKENS,
    layout::{Axis, Box_, Child, Insets, Rect, Size, TextStyle, solve_box},
};
use unicode_segmentation::UnicodeSegmentation;

fn named(key: impl Into<String>, element: c::Element) -> c::Element {
    c::Element::Named {
        name: key.into(),
        content: Box::new(element),
    }
}
fn style(fill: ui::Color, alpha: f32, radius: f32, pad: [f32; 4]) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.material = Some(ui::Material::Flat);
    s.overrides.fill = Some(ui::Paint {
        color: fill,
        alpha: ui::Alpha::Value(alpha),
    });
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s.overrides.shadow = Some(ui::Shadow::None);
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.padding = Some(pad.map(ui::Length::Px));
    s
}
fn plain(pad: [f32; 4]) -> ui::Style {
    style(ui::Color::Transparent, 0., 0., pad)
}
fn border(mut s: ui::Style, ctx: ui::Context, sides: [bool; 4]) -> ui::Style {
    let mut rgba = bbt_ui_model::draw::Color::hex(ctx.theme.palette().line).0;
    rgba[3] *= STYLE.material.hairline_alpha as f32;
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(STYLE.material.hairline_px as f32),
        paint: ui::Paint::solid(ui::Color::Rgba(rgba)),
        outside: false,
    });
    s.overrides.border_sides = Some(sides);
    s
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
fn row(key: impl Into<String>, style: ui::Style, gap: f32, children: Vec<c::Item>) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.gap = ui::Length::Px(gap);
    named(key, c::Element::Box(b))
}
fn fixed(e: c::Element, n: f32) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::Fixed(n),
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
fn spacer(h: f32) -> c::Item {
    fixed(c::Element::Spacer, h)
}
fn text(key: impl Into<String>, value: impl Into<String>, font: ui::Text) -> c::Element {
    named(key, c::Element::Text(c::Text::new(value, font)))
}
fn nowrap(key: impl Into<String>, value: impl Into<String>, font: ui::Text) -> c::Element {
    let mut t = c::Text::new(value, font);
    t.wrap = false;
    named(key, c::Element::Text(t))
}
fn font(step: ui::TypeStep, weight: u16, mono: bool, paint: ui::Color) -> ui::Text {
    ui::Text {
        step,
        weight,
        mono,
        paint: ui::Paint::solid(paint),
        line_height: 1.6,
        ..Default::default()
    }
}
fn measure_style(f: ui::Text) -> TextStyle {
    TextStyle {
        size: f.size.unwrap_or_else(|| f.step.pixels()),
        weight: f.weight,
        mono: f.mono,
        italic: f.italic,
        tracking_em: f.tracking_em,
    }
}
fn size(e: &c::Element, w: f32, ctx: ui::Context, m: &dyn c::Measure) -> [f32; 2] {
    e.natural(w, ctx, m)
        .expect("valid native document geometry")
        .0
}

impl Frontmatter {
    pub fn string(&self, key: &str) -> Option<String> {
        self.fields.get(key).map(|v| match v {
            serde_yaml::Value::String(s) => s.clone(),
            _ => serde_yaml::to_string(v)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        })
    }
    pub fn strings(&self, key: &str) -> Vec<String> {
        let display = |value: &serde_yaml::Value| {
            if key == "links"
                && let serde_yaml::Value::Mapping(link) = value
                && let Some(target) = link.get("to").and_then(serde_yaml::Value::as_str)
            {
                let Some(relation) = link.get("rel").and_then(serde_yaml::Value::as_str) else {
                    return target.to_owned();
                };
                if target.starts_with("[[") && target.ends_with("]]") {
                    let weight = link
                        .get("weight")
                        .or_else(|| link.get("w"))
                        .and_then(serde_yaml::Value::as_f64)
                        .filter(|w| w.is_finite() && *w >= 0.)
                        .map(|w| format!(":{w}"))
                        .unwrap_or_default();
                    return format!("{target}{{{relation}{weight}}}");
                }
                return format!("{target} · {relation}");
            }
            match value {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::to_string(value)
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
            }
        };
        match self.fields.get(key) {
            Some(serde_yaml::Value::Sequence(values)) => values.iter().map(display).collect(),
            Some(value) => vec![display(value)],
            None => vec![],
        }
    }
}
impl Markdown {
    /// Project known index fields; cryptographic evidence is deliberately not
    /// manufactured from a signature-shaped YAML string.
    pub fn document(&self, path: &str) -> Document {
        let f = &self.frontmatter;
        let title = f
            .string("title")
            .or_else(|| {
                self.blocks.iter().find_map(|b| {
                    if let BlockKind::Heading { content, .. } = &b.kind {
                        Some(Inline::plain_text(content))
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path).into());
        Document {
            kind: f.string("type"),
            number: f.number(),
            id: f.string("id").unwrap_or_default(),
            path: path.into(),
            title,
            status: f.string("status"),
            assignee: f.string("assignee"),
            role: f.string("role"),
            parent: f.string("parent"),
            children: f.strings("children"),
            depends_on: f.strings("depends_on"),
            links: f.strings("links"),
            allowed_paths: f.strings("allowed_paths"),
            verify: f.string("verify"),
            created: f.string("created"),
            updated: f.string("updated"),
            superseded_by: f.string("superseded_by"),
            broken: f.error.clone(),
            body: self.source.clone(),
            prov: Provenance {
                created_by: f.string("created_by").unwrap_or_default(),
                updated_by: f.string("updated_by"),
                derived_from: f.string("derived_from"),
                digest: f.string("digest").unwrap_or_default(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    /// Entire Markdown presentation, including generic YAML fields.
    pub fn element(&self, ctx: ui::Context, m: &dyn c::Measure, width: f32) -> c::Element {
        self.element_with_title(ctx, m, width, None)
    }
    fn element_with_title(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        title: Option<&str>,
    ) -> c::Element {
        let width = finite_width(width).min(STYLE.spacing.doc_measure as f32);
        let r = Render {
            ctx,
            m,
            width,
            diagrams: &[],
            record_links: None,
            scrolls: None,
        };
        let mut children = vec![];
        if !self.frontmatter.fields.is_empty() || self.frontmatter.error.is_some() {
            children.push(r.metadata(&self.frontmatter, &[]).into());
            children.push(spacer(STYLE.spacing.s7 as f32));
        }
        children.push(r.blocks(self.display_blocks(title), "").into());
        column("markdown", plain([0.; 4]), 0., children)
    }
    /// Body seam used by DocumentModal. Diagram records carry Source/scroll
    /// state; their illustrative graph geometry is not consumed.
    pub fn body_element(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        diagrams: &[Diagram],
    ) -> c::Element {
        self.body_element_with_title(None, ctx, m, width, diagrams)
    }
    /// A header is already visible. Suppress only the lab's matching leading
    /// ATX H1; retain the original Markdown bytes and every other heading.
    pub fn body_element_with_title(
        &self,
        title: Option<&str>,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        diagrams: &[Diagram],
    ) -> c::Element {
        let width = finite_width(width).min(STYLE.spacing.doc_measure as f32);
        let r = Render {
            ctx,
            m,
            width,
            diagrams,
            record_links: None,
            scrolls: None,
        };
        r.blocks(self.display_blocks(title), "")
    }
    /// Body with authoritative record resolutions; missing targets stay inert.
    pub fn body_element_with_links(
        &self,
        title: Option<&str>,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        diagrams: &[Diagram],
        links: &ui::modules::document_modal::controller::RecordLinks,
    ) -> c::Element {
        Render {
            ctx,
            m,
            width: finite_width(width).min(STYLE.spacing.doc_measure as f32),
            diagrams,
            record_links: Some(links),
            scrolls: None,
        }
        .blocks(self.display_blocks(title), "")
    }
    /// A live provider retains generic frontmatter alongside the known header
    /// fields. No YAML bytes or verification badge are reconstructed here.
    pub fn hosted_body_element(
        &self,
        title: &str,
        ctx: ui::Context,
        m: &dyn c::Measure,
        width: f32,
        links: &ui::modules::document_modal::controller::RecordLinks,
        scrolls: &std::collections::BTreeMap<String, f32>,
    ) -> c::Element {
        let r = Render {
            ctx,
            m,
            width: finite_width(width).min(STYLE.spacing.doc_measure as f32),
            diagrams: &[],
            record_links: Some(links),
            scrolls: Some(scrolls),
        };
        let typed = self.frontmatter.fields.contains_key("type");
        let extra: Vec<_> = self
            .frontmatter
            .fields
            .keys()
            .filter(|key| {
                !typed || (!KNOWN_FIELDS.contains(&key.as_str()) && key.as_str() != "number")
            })
            .cloned()
            .collect();
        let body = r.blocks(self.display_blocks(Some(title)), "");
        if extra.is_empty() {
            body
        } else {
            column(
                "body-with-metadata",
                plain([0.; 4]),
                STYLE.spacing.s7 as f32,
                vec![r.metadata(&self.frontmatter, &extra).into(), body.into()],
            )
        }
    }
    fn display_blocks(&self, title: Option<&str>) -> &[Block] {
        let Some(title) = title.filter(|title| !title.is_empty()) else {
            return &self.blocks;
        };
        let Some(line) = self
            .source
            .get(self.body_start..)
            .and_then(|body| body.lines().find(|line| !line.trim().is_empty()))
        else {
            return &self.blocks;
        };
        let normalize = |value: &str| {
            value
                .to_lowercase()
                .chars()
                .map(|c| {
                    if matches!(c, '\u{2012}'..='\u{2015}') {
                        '-'
                    } else {
                        c
                    }
                })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        if let Some(h1) = line.strip_prefix("# ")
            && normalize(h1) == normalize(title)
            && matches!(
                self.blocks.first().map(|b| &b.kind),
                Some(BlockKind::Heading { level: 1, .. })
            )
        {
            &self.blocks[1..]
        } else {
            &self.blocks
        }
    }
    pub fn modal_element(
        &self,
        path: &str,
        modal: bool,
        viewport: [f32; 2],
        ctx: ui::Context,
        m: &dyn c::Measure,
    ) -> c::Element {
        let shell = DocumentModal {
            document: self.document(path),
            modal,
            viewport,
            ..Default::default()
        };
        shell.element_with_body(ctx, m, viewport, &|w| {
            let title = if shell.document.title.is_empty() {
                &shell.document.path
            } else {
                &shell.document.title
            };
            if shell.document.kind.is_none() {
                self.element_with_title(ctx, m, w, Some(title))
            } else {
                let width = finite_width(w).min(STYLE.spacing.doc_measure as f32);
                let r = Render {
                    ctx,
                    m,
                    width,
                    diagrams: &[],
                    record_links: None,
                    scrolls: None,
                };
                let extra: Vec<_> = self
                    .frontmatter
                    .fields
                    .keys()
                    .filter(|k| !KNOWN_FIELDS.contains(&k.as_str()))
                    .cloned()
                    .collect();
                if extra.is_empty() {
                    r.blocks(self.display_blocks(Some(title)), "")
                } else {
                    column(
                        "body-with-metadata",
                        plain([0.; 4]),
                        STYLE.spacing.s7 as f32,
                        vec![
                            r.metadata(&self.frontmatter, &extra).into(),
                            r.blocks(self.display_blocks(Some(title)), "").into(),
                        ],
                    )
                }
            }
        })
    }
}
const KNOWN_FIELDS: &[&str] = &[
    "type",
    "id",
    "title",
    "status",
    "assignee",
    "role",
    "parent",
    "children",
    "depends_on",
    "links",
    "allowed_paths",
    "verify",
    "created",
    "updated",
    "superseded_by",
    "created_by",
    "updated_by",
    "derived_from",
    "digest",
];
fn finite_width(width: f32) -> f32 {
    if width.is_finite() { width.max(0.) } else { 0. }
}

struct Render<'a> {
    ctx: ui::Context,
    m: &'a dyn c::Measure,
    width: f32,
    diagrams: &'a [Diagram],
    record_links: Option<&'a ui::modules::document_modal::controller::RecordLinks>,
    scrolls: Option<&'a std::collections::BTreeMap<String, f32>>,
}
#[derive(Clone)]
enum Piece {
    Text {
        value: String,
        font: ui::Text,
        strike: bool,
        underline: bool,
    },
    Wiki {
        target: String,
        display: String,
        relation: Option<String>,
        weight: Option<f32>,
    },
    Break,
}
impl Render<'_> {
    fn blocks(&self, blocks: &[Block], prefix: &str) -> c::Element {
        let mut children = vec![];
        let mut previous_bottom = 0.;
        for (i, b) in blocks.iter().enumerate() {
            let key = if prefix.is_empty() {
                format!("block{i}")
            } else {
                format!("{prefix}/block{i}")
            };
            let (e, top, bottom) = self.block(b, &key);
            if i > 0 {
                children.push(spacer(f32::max(previous_bottom, top)))
            }
            children.push(e.into());
            previous_bottom = bottom;
        }
        let mut s = plain([0.; 4]);
        s.overrides.width = Some(ui::Length::Px(self.width));
        row(
            if prefix.is_empty() {
                "body-slot".into()
            } else {
                format!("{prefix}/slot")
            },
            plain([0.; 4]),
            0.,
            vec![fixed(
                column(
                    if prefix.is_empty() {
                        "body".to_owned()
                    } else {
                        prefix.to_owned()
                    },
                    s,
                    0.,
                    children,
                ),
                self.width,
            )],
        )
    }
    fn block(&self, b: &Block, key: &str) -> (c::Element, f32, f32) {
        let body = font(ui::TypeStep::Body, 400, false, ui::Color::Foreground);
        match &b.kind {
            BlockKind::Paragraph(p) => (
                self.rich(key, p, body),
                STYLE.spacing.s2 as f32,
                STYLE.spacing.s4 as f32,
            ),
            BlockKind::Heading { level, content } => (
                self.rich(
                    key,
                    content,
                    ui::Text {
                        // Heading baseline: reuse the existing named type scale.
                        size: Some(match level {
                            1 => STYLE.r#type.doc_title,
                            2 => STYLE.r#type.heading,
                            3 => STYLE.r#type.subtitle,
                            4 => STYLE.r#type.doc_heading,
                            5 => STYLE.r#type.body,
                            _ => STYLE.r#type.section,
                        } as f32),
                        ..font(ui::TypeStep::Body, 700, false, ui::Color::Foreground)
                    },
                ),
                STYLE.spacing.s7 as f32,
                STYLE.spacing.s2 as f32,
            ),
            BlockKind::Code { info, text } => {
                if info.split_whitespace().next() == Some("mermaid") {
                    let mut options = self
                        .diagrams
                        .iter()
                        .find(|d| d.source.trim_end() == text.trim_end())
                        .map(|d| crate::mermaid::Options {
                            show_source: d.show_source,
                            scroll_x: d.scroll_x,
                        })
                        .unwrap_or_default();
                    if let Some(offset) = self
                        .scrolls
                        .and_then(|scrolls| scrolls.get(&format!("{key}/drawing")))
                    {
                        options.scroll_x = *offset;
                    }
                    (
                        named(
                            key,
                            crate::mermaid::element_with_key(
                                key, text, self.width, self.ctx, self.m, options,
                            ),
                        ),
                        0.,
                        0.,
                    )
                } else {
                    (
                        self.code(key, text, 1.6),
                        ui::TypeStep::Caption.pixels(),
                        ui::TypeStep::Caption.pixels(),
                    )
                }
            }
            BlockKind::List { start, items } => {
                let indent = STYLE.spacing.s9 as f32;
                let inner = Render {
                    width: (self.width - indent).max(0.),
                    ..*self
                };
                let mut rows = vec![];
                for (i, blocks) in items.iter().enumerate() {
                    let item_key = format!("{key}/li{i}");
                    let child = if let [
                        Block {
                            kind: BlockKind::Paragraph(p),
                            ..
                        },
                    ] = blocks.as_slice()
                    {
                        inner.rich(&item_key, p, body)
                    } else {
                        inner.blocks(blocks, &item_key)
                    };
                    let h = size(&child, inner.width, self.ctx, self.m)[1];
                    let label = start
                        .map(|start| format!("{}.", start.saturating_add(i as u64)))
                        .unwrap_or_else(|| "•".into());
                    let marker = nowrap(format!("{key}/bullet{i}"), label, body);
                    let lh = (body.step.pixels() * body.line_height * 64.).floor() / 64.;
                    rows.push(
                        c::Element::Layers {
                            base: Box::new(child),
                            overlays: vec![c::Placed {
                                content: marker,
                                left: -12.,
                                bottom: h - lh,
                            }],
                        }
                        .into(),
                    );
                }
                (
                    column(key, plain([0., 0., 0., indent]), 0., rows),
                    STYLE.spacing.s2 as f32,
                    STYLE.spacing.s5 as f32,
                )
            }
            BlockKind::Quote(blocks) => {
                let pad = STYLE.spacing.s6 as f32;
                let inner = Render {
                    width: (self.width - pad * 2. - 2.).max(0.),
                    ..*self
                };
                let mut s = plain([STYLE.spacing.s2 as f32, pad, STYLE.spacing.s2 as f32, pad]);
                s.overrides.border = Some(ui::Border {
                    width: ui::Length::Px(2.),
                    paint: ui::Paint::solid(ui::Color::Muted),
                    outside: false,
                });
                s.overrides.border_sides = Some([false, false, false, true]);
                (
                    column(
                        key,
                        s,
                        0.,
                        vec![inner.blocks(blocks, &format!("{key}/quote")).into()],
                    ),
                    STYLE.spacing.s2 as f32,
                    STYLE.spacing.s4 as f32,
                )
            }
            BlockKind::Table {
                align,
                header,
                rows,
            } => (
                self.table(key, align, header, rows),
                STYLE.spacing.s4 as f32,
                STYLE.spacing.s4 as f32,
            ),
            BlockKind::Footnote { label, blocks } => {
                let heading = nowrap(
                    format!("{key}/label"),
                    format!("[{label}]"),
                    font(ui::TypeStep::Caption, 600, false, ui::Color::Muted),
                );
                (
                    column(
                        key,
                        plain([0.; 4]),
                        STYLE.spacing.s1 as f32,
                        vec![
                            heading.into(),
                            self.blocks(blocks, &format!("{key}/note")).into(),
                        ],
                    ),
                    STYLE.spacing.s4 as f32,
                    STYLE.spacing.s4 as f32,
                )
            }
            BlockKind::Rule => {
                let mut s = style(
                    ui::Color::Line,
                    STYLE.material.hairline_alpha as f32,
                    0.,
                    [0.; 4],
                );
                s.overrides.height = Some(ui::Length::Px(STYLE.material.hairline_px as f32));
                (
                    row(key, s, 0., vec![]),
                    STYLE.spacing.s7 as f32,
                    STYLE.spacing.s7 as f32,
                )
            }
            BlockKind::Html(value) => (
                self.code(key, value, 1.6),
                STYLE.spacing.s4 as f32,
                STYLE.spacing.s4 as f32,
            ),
        }
    }
    fn pieces(values: &[Inline], f: ui::Text, strike: bool, underline: bool, out: &mut Vec<Piece>) {
        for value in values {
            let text = |value: String, font: ui::Text| Piece::Text {
                value,
                font,
                strike,
                underline,
            };
            match value {
                Inline::Text(s) => out.push(text(s.clone(), f)),
                Inline::Code(s) => out.push(text(s.clone(), ui::Text { mono: true, ..f })),
                Inline::Emphasis(s) => {
                    Self::pieces(s, ui::Text { italic: true, ..f }, strike, underline, out)
                }
                Inline::Strong(s) => {
                    Self::pieces(s, ui::Text { weight: 700, ..f }, strike, underline, out)
                }
                Inline::Strike(s) => Self::pieces(s, f, true, underline, out),
                Inline::Link { content, .. } => Self::pieces(
                    content,
                    ui::Text {
                        paint: ui::Paint::solid(ui::Color::Accent),
                        ..f
                    },
                    strike,
                    true,
                    out,
                ),
                Inline::WikiLink {
                    target,
                    display,
                    relation,
                    weight,
                } => out.push(Piece::Wiki {
                    target: target.clone(),
                    display: display.clone(),
                    relation: relation.clone(),
                    weight: *weight,
                }),
                Inline::Footnote(label) => out.push(text(
                    format!("[{label}]"),
                    ui::Text {
                        step: ui::TypeStep::Micro,
                        size: None,
                        paint: ui::Paint::solid(ui::Color::Accent),
                        ..f
                    },
                )),
                Inline::Task(done) => {
                    out.push(text(if *done { "☑ ".into() } else { "☐ ".into() }, f))
                }
                Inline::SoftBreak => out.push(text(" ".into(), f)),
                Inline::HardBreak => out.push(Piece::Break),
                Inline::Image { target, alt } => out.push(text(
                    format!("[image: {alt}] ({target})"),
                    ui::Text {
                        paint: ui::Paint::solid(ui::Color::Muted),
                        ..f
                    },
                )),
            }
        }
    }
    fn rich(&self, key: &str, values: &[Inline], f: ui::Text) -> c::Element {
        let mut pieces = vec![];
        Self::pieces(values, f, false, false, &mut pieces);
        if pieces.iter().all(|p| {
            matches!(
                p,
                Piece::Text {
                    strike: false,
                    underline: false,
                    ..
                } | Piece::Break
            )
        }) && !pieces.iter().any(|p| match p {
            Piece::Text { value, font, .. } => value
                .split_whitespace()
                .any(|word| self.m.width(word, &measure_style(*font)) > self.width),
            _ => false,
        }) {
            let mut t = c::Text::new("", f);
            t.runs.clear();
            for p in pieces {
                match p {
                    Piece::Text { value, font, .. } => {
                        t.runs.push(c::TextRun { value, style: font })
                    }
                    Piece::Break => t.runs.push(c::TextRun {
                        value: "\n".into(),
                        style: f,
                    }),
                    _ => unreachable!(),
                }
            }
            if t.runs.is_empty() {
                t.runs.push(c::TextRun {
                    value: String::new(),
                    style: f,
                })
            }
            return named(key, c::Element::Text(t));
        }
        let mut lines = vec![];
        let mut children = vec![];
        let mut i = 0;
        for p in pieces {
            match p {
                Piece::Break => {
                    lines.push(Self::flow(key, std::mem::take(&mut children)));
                }
                Piece::Wiki {
                    target,
                    display,
                    relation,
                    weight,
                } => {
                    children.push(natural(self.wiki(
                        &format!("{key}/link{i}"),
                        &target,
                        &display,
                        relation.as_deref(),
                        weight,
                        f,
                    )));
                    i += 1;
                }
                Piece::Text {
                    value,
                    font,
                    strike,
                    underline,
                } => {
                    for word in value
                        .split_inclusive(' ')
                        .filter(|s| !s.is_empty())
                        .flat_map(|word| self.fragments(word, font))
                    {
                        let part = format!("{key}/word{i}");
                        i += 1;
                        let mut t = c::Text::new(&word, font);
                        t.wrap = false;
                        let e = named(&part, c::Element::Text(t));
                        let mut overlays = vec![];
                        for bottom in [
                            strike
                                .then_some(font.size.unwrap_or_else(|| font.step.pixels()) * 0.55),
                            underline.then_some(1.),
                        ]
                        .into_iter()
                        .flatten()
                        {
                            let mut s = style(font.paint.color, 1., 0., [0.; 4]);
                            s.overrides.width = Some(ui::Length::Px(
                                self.m.width(&word, &measure_style(font)).min(self.width),
                            ));
                            s.overrides.height =
                                Some(ui::Length::Px(STYLE.material.hairline_px as f32));
                            overlays.push(c::Placed {
                                content: row("decoration", s, 0., vec![]),
                                left: 0.,
                                bottom,
                            });
                        }
                        children.push(natural(if overlays.is_empty() {
                            e
                        } else {
                            c::Element::Layers {
                                base: Box::new(e),
                                overlays,
                            }
                        }));
                    }
                }
            }
        }
        lines.push(Self::flow(key, children));
        if lines.len() == 1 {
            lines.pop().unwrap()
        } else {
            column(
                key,
                plain([0.; 4]),
                0.,
                lines.into_iter().map(Into::into).collect(),
            )
        }
    }
    fn flow(key: &str, children: Vec<c::Item>) -> c::Element {
        let mut f = c::Flow::new(children);
        f.row_gap = ui::Length::Px(0.);
        f.column_gap = ui::Length::Px(0.);
        f.align = c::Alignment::Baseline;
        named(key, c::Element::Flow(f))
    }
    /// Flow can wrap between retained children. Break only oversized tokens,
    /// at extended grapheme boundaries, using the very same font measurement.
    /// Source bytes and parser spans are never rewritten by presentation.
    fn fragments(&self, value: &str, font: ui::Text) -> Vec<String> {
        let measure = measure_style(font);
        if self.m.width(value, &measure) <= self.width {
            return vec![value.into()];
        }
        let limit = (self.width - self.m.width(" ", &measure)).max(1.);
        let mut out = vec![];
        let mut part = String::new();
        for cluster in value.graphemes(true) {
            let mut next = part.clone();
            next.push_str(cluster);
            if !part.is_empty() && self.m.width(&next, &measure) > limit {
                out.push(std::mem::take(&mut part));
            }
            part.push_str(cluster);
        }
        if !part.is_empty() {
            out.push(part);
        }
        out
    }
    fn wiki(
        &self,
        key: &str,
        target: &str,
        display: &str,
        relation: Option<&str>,
        weight: Option<f32>,
        parent: ui::Text,
    ) -> c::Element {
        let (display, enabled) = ui::modules::document_modal::controller::presentation(
            target,
            display,
            self.record_links,
        );
        let slug = nowrap(
            format!("{key}/slug"),
            display.clone(),
            ui::Text {
                line_height: 16. / 11.,
                ..font(
                    ui::TypeStep::Caption,
                    400,
                    false,
                    if enabled {
                        ui::Color::Rgba(
                            bbt_ui_model::draw::Color::hex(self.ctx.theme.palette().accent_soft).0,
                        )
                    } else {
                        ui::Color::Muted
                    },
                )
            },
        );
        let mut slug_style = style(
            if enabled {
                ui::Color::Accent
            } else {
                ui::Color::Transparent
            },
            if enabled { 0.1 } else { 0. },
            0.,
            [0., 6., 0., 6.],
        );
        if !enabled {
            slug_style.overrides.opacity = Some(0.7);
        }
        let mut cells =
            vec![row(format!("{key}/slug-box"), slug_style, 0., vec![slug.into()]).into()];
        let numbered = self
            .record_links
            .is_some_and(|links| links.contains_key(target))
            && (ui::modules::document_modal::controller::numbered(target).is_some()
                || display.starts_with('#'));
        if let Some(rel) = relation.filter(|_| !numbered) {
            let co = match rel {
                "violates" => {
                    ui::Color::Rgba(bbt_ui_model::draw::Color::hex(TOKENS.color.state.errored).0)
                }
                "constrains" => {
                    ui::Color::Rgba(bbt_ui_model::draw::Color::hex(TOKENS.color.state.waiting).0)
                }
                "derives" | "derived_from" => ui::Color::Rgba(
                    bbt_ui_model::draw::Color::hex(self.ctx.theme.palette().accent_soft).0,
                ),
                _ => ui::Color::Muted,
            };
            let label = format!(
                "{rel}{}",
                weight.map(|w| format!(" {w}")).unwrap_or_default()
            );
            cells.push(
                row(
                    format!("{key}/rel-box"),
                    style(
                        ui::Color::Muted,
                        STYLE.tint.badge_alpha as f32,
                        0.,
                        [0., 5., 0., 5.],
                    ),
                    0.,
                    vec![
                        nowrap(
                            format!("{key}/rel"),
                            label,
                            ui::Text {
                                line_height: 16. / 9.5,
                                ..font(ui::TypeStep::Tiny, 400, true, co)
                            },
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        let mut chip_style = border(plain([0.; 4]), self.ctx, [true; 4]);
        chip_style.overrides.radii = Some([ui::Length::Px(STYLE.radius.tag as f32); 4]);
        chip_style.overrides.border.as_mut().unwrap().outside = enabled;
        let mut b = c::Box::new(chip_style, cells);
        b.gap = ui::Length::Px(0.);
        b.clip = true;
        let chip = c::Element::InlineMiddle {
            content: Box::new(named(format!("{key}/chip"), c::Element::Box(b))),
            parent,
        };
        let mut wrap = c::Box::new(plain([0., 2., 0., 2.]), vec![chip.into()]);
        wrap.inline = true;
        wrap.align = c::Alignment::Baseline;
        wrap.gap = ui::Length::Px(0.);
        wrap.style.overrides.text = Some(parent);
        ui::modules::document_modal::controller::link(
            named(key, c::Element::Box(wrap)),
            target,
            enabled,
        )
    }
    fn code(&self, key: &str, value: &str, line_height: f32) -> c::Element {
        let value = value.strip_suffix('\n').unwrap_or(value);
        let f = ui::Text {
            line_height,
            ..font(ui::TypeStep::Caption, 400, true, ui::Color::Foreground)
        };
        let e = nowrap(format!("{key}/text"), value, f);
        let pad = [
            STYLE.spacing.s4 as f32,
            STYLE.spacing.s5 as f32,
            STYLE.spacing.s4 as f32,
            STYLE.spacing.s5 as f32,
        ];
        let inner = (self.width - pad[1] - pad[3] - 2.).max(0.);
        let [w, h] = size(&e, inner, self.ctx, self.m);
        let child = if w > inner + 0.01 {
            c::Element::ScrollView(c::ScrollView {
                viewport: [inner, h],
                offset: [
                    self.scrolls.and_then(|s| s.get(key)).copied().unwrap_or(0.),
                    0.,
                ],
                content_width: Some(w),
                content: Box::new(e),
            })
        } else {
            e
        };
        column(
            key,
            border(
                style(ui::Color::Surface, 1., STYLE.radius.row as f32, pad),
                self.ctx,
                [true; 4],
            ),
            0.,
            vec![child.into()],
        )
    }
    fn table(
        &self,
        key: &str,
        align: &[TableAlign],
        header: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
    ) -> c::Element {
        let n = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if n == 0 {
            return column(key, plain([0.; 4]), 0., vec![]);
        }
        let pad = STYLE.spacing.s4 as f32;
        let available = (self.width - 2.).max(0.);
        let content_limit = (available - 2. * pad - 2.).max(1.);
        let mut minima = vec![2. * pad + 2.; n];
        let mut preferred = minima.clone();
        for (ri, cells) in std::iter::once(header)
            .chain(rows.iter().map(Vec::as_slice))
            .enumerate()
        {
            let f = font(
                ui::TypeStep::Body,
                if ri == 0 { 600 } else { 400 },
                false,
                ui::Color::Foreground,
            );
            for (i, cell) in cells.iter().enumerate() {
                let [min, max] = self.cell_widths(cell, f);
                // Like CSS auto tables, a column keeps its longest unbreakable
                // word. Only a word wider than the entire table may fragment.
                minima[i] = minima[i].max(min.min(content_limit) + 2. * pad + 2.);
                preferred[i] = preferred[i].max(max + 2. * pad + 2.);
            }
        }
        let minimum: f32 = minima.iter().sum();
        let canvas = available.max(minimum);
        let extra = solve_box(
            &Box_ {
                axis: Axis::Row,
                pad: Insets::default(),
                gap: 0.,
                children: preferred
                    .iter()
                    .zip(&minima)
                    .map(|(want, min)| (Size::Flex((want - min).max(0.001)), Child::default()))
                    .collect(),
                align: bbt_ui_model::layout::Align::Start,
            },
            Rect::new(0., 0., (canvas - minimum).max(0.), 0.),
            self.m,
        );
        let widths: Vec<_> = extra
            .iter()
            .zip(&minima)
            .map(|(r, min)| min + r.w)
            .collect();
        let mut lines = vec![];
        for (ri, cells) in std::iter::once(header)
            .chain(rows.iter().map(Vec::as_slice))
            .enumerate()
        {
            let mut items = vec![];
            for i in 0..n {
                let width = widths[i];
                let render = Render {
                    width: (width - 2. * pad - 2.).max(0.),
                    ..*self
                };
                let f = font(
                    ui::TypeStep::Body,
                    if ri == 0 { 600 } else { 400 },
                    false,
                    ui::Color::Foreground,
                );
                let mut cell = render.rich(
                    &format!("{key}/row{ri}/cell{i}/text"),
                    cells.get(i).map_or(&[], Vec::as_slice),
                    f,
                );
                if !matches!(align.get(i), Some(TableAlign::Start) | None)
                    && let c::Element::Named { content, .. } = &mut cell
                    && let c::Element::Text(t) = &mut **content
                {
                    t.align = if align[i] == TableAlign::Center {
                        c::Alignment::Center
                    } else {
                        c::Alignment::End
                    };
                }
                let s = border(
                    style(
                        if ri == 0 {
                            ui::Color::Surface
                        } else {
                            ui::Color::Transparent
                        },
                        if ri == 0 { 0.65 } else { 0. },
                        0.,
                        [4., pad, 4., pad],
                    ),
                    self.ctx,
                    [false, i + 1 < n, true, false],
                );
                items.push(fixed(
                    column(format!("{key}/row{ri}/cell{i}"), s, 0., vec![cell.into()]),
                    width,
                ));
            }
            let mut r = c::Box::new(plain([0.; 4]), items);
            r.gap = ui::Length::Px(0.);
            r.align = c::Alignment::Stretch;
            lines.push(c::Element::Box(r).into());
        }
        let content = column(format!("{key}/rows"), plain([0.; 4]), 0., lines);
        let h = size(&content, canvas, self.ctx, self.m)[1];
        let mut b = c::Box::new(
            border(
                style(ui::Color::Transparent, 0., STYLE.radius.row as f32, [0.; 4]),
                self.ctx,
                [true; 4],
            ),
            vec![
                c::Element::ScrollView(c::ScrollView {
                    viewport: [(self.width - 2.).max(0.), h],
                    offset: [
                        self.scrolls.and_then(|s| s.get(key)).copied().unwrap_or(0.),
                        0.,
                    ],
                    content_width: Some(canvas),
                    content: Box::new(content),
                })
                .into(),
            ],
        );
        b.clip = true;
        b.gap = ui::Length::Px(0.);
        named(key, c::Element::Box(b))
    }
    fn cell_widths(&self, values: &[Inline], f: ui::Text) -> [f32; 2] {
        let mut pieces = vec![];
        Self::pieces(values, f, false, false, &mut pieces);
        let (mut minimum, mut preferred, mut word, mut line) = (0_f32, 0_f32, 0_f32, 0_f32);
        for piece in pieces {
            match piece {
                Piece::Text { value, font, .. } => {
                    for part in value.split_inclusive(char::is_whitespace) {
                        let content = part.trim_end_matches(char::is_whitespace);
                        word += self.m.width(content, &measure_style(font));
                        line += self.m.width(part, &measure_style(font));
                        if part.chars().last().is_some_and(char::is_whitespace) {
                            minimum = minimum.max(word);
                            word = 0.;
                        }
                    }
                }
                Piece::Wiki {
                    target,
                    display,
                    relation,
                    weight,
                } => {
                    minimum = minimum.max(word);
                    word = 0.;
                    let chip = self.wiki(
                        "table-measure",
                        &target,
                        &display,
                        relation.as_deref(),
                        weight,
                        f,
                    );
                    let width = size(&chip, f32::INFINITY, self.ctx, self.m)[0];
                    minimum = minimum.max(width);
                    line += width;
                }
                Piece::Break => {
                    minimum = minimum.max(word);
                    preferred = preferred.max(line);
                    word = 0.;
                    line = 0.;
                }
            }
        }
        [minimum.max(word), preferred.max(line)]
    }
    fn metadata(&self, front: &Frontmatter, keys: &[String]) -> c::Element {
        let mut rows = vec![];
        for (key, value) in front
            .fields
            .iter()
            .filter(|(k, _)| keys.is_empty() || keys.contains(k))
        {
            let value = match value {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::to_string(value)
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
            };
            rows.push(vec![
                vec![Inline::Code(key.clone())],
                vec![Inline::Text(value)],
            ]);
        }
        let mut children = vec![];
        if let Some(error) = &front.error {
            children.push(
                text(
                    "metadata/error",
                    format!("frontmatter does not parse · {error} · shown as source"),
                    font(ui::TypeStep::Caption, 400, false, ui::Color::Muted),
                )
                .into(),
            );
            children.push(self.code("metadata/source", &front.source, 1.6).into());
        }
        if !rows.is_empty() {
            children.push(
                self.table(
                    "metadata/grid",
                    &[TableAlign::Start; 2],
                    &[
                        vec![Inline::Text("Field".into())],
                        vec![Inline::Text("Value".into())],
                    ],
                    &rows,
                )
                .into(),
            );
        }
        column(
            "metadata",
            plain([0.; 4]),
            STYLE.spacing.s2 as f32,
            children,
        )
    }
}
