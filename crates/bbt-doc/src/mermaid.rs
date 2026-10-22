//! The Mermaid producer owns parsing and layout. This consumer maps its public
//! geometry to retained UI elements, using the same fonts for measure and paint.
use bbt_mermaid as mm;
use bbt_ui::{
    self as ui, components as c,
    modules::document_modal::{Diagram, Figure},
};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::Color as Rgba,
    layout::{Axis, TextStyle},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub show_source: bool,
    pub scroll_x: f32,
}

fn font(role: mm::TextRole) -> ui::Text {
    let (step, weight, mono) = match role {
        mm::TextRole::Node => (ui::TypeStep::Section, 400, false),
        mm::TextRole::Actor => (ui::TypeStep::Section, 400, false),
        mm::TextRole::ClusterTitle => (ui::TypeStep::Caption, 600, false),
        mm::TextRole::FrameLabel => (ui::TypeStep::Caption, 400, true),
        mm::TextRole::EdgeLabel | mm::TextRole::Message | mm::TextRole::Note => {
            (ui::TypeStep::Note, 400, false)
        }
    };
    ui::Text {
        step,
        weight,
        mono,
        line_height: 1.2,
        ..Default::default()
    }
}
fn measure_style(f: ui::Text) -> TextStyle {
    TextStyle {
        size: f.step.pixels(),
        weight: f.weight,
        mono: f.mono,
        italic: f.italic,
        tracking_em: f.tracking_em,
    }
}
struct Fonts<'a> {
    ctx: ui::Context,
    host: &'a dyn c::Measure,
}
impl mm::Measure for Fonts<'_> {
    fn measure(&self, line: &str, role: mm::TextRole) -> mm::Size {
        let f = font(role);
        let mut text = c::Text::new(line, f);
        text.wrap = false;
        let (size, _) = c::Element::Text(text)
            .natural(f32::INFINITY, self.ctx, self.host)
            .expect("valid Mermaid text style");
        mm::Size {
            w: self.host.width(line, &measure_style(f)),
            h: size[1],
        }
    }
}

pub fn geometry(
    source: &str,
    ctx: ui::Context,
    measure: &dyn c::Measure,
) -> Result<mm::Geometry, mm::Error> {
    mm::render(source, &Fonts { ctx, host: measure }, &mm::Style::default())
}

pub fn element(
    source: &str,
    width: f32,
    ctx: ui::Context,
    measure: &dyn c::Measure,
    options: Options,
) -> c::Element {
    element_with_key("mermaid", source, width, ctx, measure, options)
}
pub fn element_with_key(
    key: &str,
    source: &str,
    width: f32,
    ctx: ui::Context,
    measure: &dyn c::Measure,
    options: Options,
) -> c::Element {
    let g = match geometry(source, ctx, measure) {
        Ok(g) => g,
        Err(error) => {
            return Diagram {
                source: source.into(),
                error: Some(error.to_string()),
                ..Default::default()
            }
            .retained_element(key, width, ctx, measure);
        }
    };
    let kind = match g.kind {
        mm::Kind::Flowchart => "flowchart",
        mm::Kind::Sequence => "sequence",
        mm::Kind::State => "state",
    };
    let count = if g.kind == mm::Kind::Sequence {
        g.nodes
            .iter()
            .filter(|n| matches!(n.shape, mm::Shape::Actor | mm::Shape::Participant))
            .map(|n| n.id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    } else {
        g.nodes
            .iter()
            .filter(|n| {
                n.label
                    .as_ref()
                    .is_some_and(|l| l.lines.iter().any(|s| !s.is_empty()))
            })
            .count()
    };
    Diagram {
        source: source.into(),
        show_source: options.show_source,
        scroll_x: options.scroll_x,
        ..Default::default()
    }
    .retained_figure(
        key,
        width,
        ctx,
        Figure {
            kind,
            count,
            canvas: &|| {
                let mut canvas = canvas(&g, ctx, measure);
                c::namespace(&mut canvas, &format!("{key}/"));
                (named(format!("{key}/canvas"), canvas), [g.size.w, g.size.h])
            },
        },
    )
}

fn named(key: impl Into<String>, e: c::Element) -> c::Element {
    c::Element::Named {
        name: key.into(),
        content: Box::new(e),
    }
}
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn mix(color: ui::Color, surface: ui::Color, amount: f32, ctx: ui::Context) -> ui::Color {
    ui::Color::Rgba(
        Rgba(ui::Paint::solid(color).resolve(ctx).unwrap())
            .mix(
                Rgba(ui::Paint::solid(surface).resolve(ctx).unwrap()),
                1. - amount,
            )
            .0,
    )
}
fn plain() -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.material = Some(ui::Material::Flat);
    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: true,
    });
    s.overrides.shadow = Some(ui::Shadow::None);
    s.overrides.radii = Some([ui::Length::Px(0.); 4]);
    s.overrides.padding = Some([ui::Length::Px(0.); 4]);
    s
}
fn panel(
    key: &str,
    size: [f32; 2],
    radius: f32,
    fill: ui::Paint,
    stroke: ui::Paint,
    weight: f32,
) -> c::Element {
    let mut s = plain();
    s.overrides.width = Some(ui::Length::Px(size[0]));
    s.overrides.height = Some(ui::Length::Px(size[1]));
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.fill = Some(fill);
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(weight),
        paint: stroke,
        outside: true,
    });
    named(key, c::Element::Box(c::Box::new(s, vec![])))
}
fn p(p: mm::Point) -> [f32; 2] {
    [p.x, p.y]
}
fn path(
    key: impl Into<String>,
    size: mm::Size,
    points: Vec<c::PathCommand>,
    stroke: c::PathStroke,
    paint: ui::Paint,
    fill: Option<ui::Paint>,
) -> c::Element {
    named(
        key,
        c::Element::Path(c::Path {
            size: [size.w, size.h],
            points: points.into(),
            stroke,
            paint,
            fill,
            opacity: 1.,
        }),
    )
}
fn node_path(
    key: &str,
    rect: mm::Rect,
    canvas: mm::Size,
    points: Vec<c::PathCommand>,
    stroke: c::PathStroke,
    paint: ui::Paint,
    fill: Option<ui::Paint>,
) -> c::Element {
    // Keep the semantic node bounds while the vector uses canvas coordinates.
    // Its world origin remains zero, preserving fractional path vertices.
    named(
        key,
        c::Element::Layers {
            base: Box::new(panel(
                &format!("{key}/bounds"),
                [rect.w, rect.h],
                0.,
                ui::Paint::solid(ui::Color::Transparent),
                ui::Paint::solid(ui::Color::Transparent),
                0.,
            )),
            overlays: vec![c::Placed {
                content: path(format!("{key}/vector"), canvas, points, stroke, paint, fill),
                left: -rect.x,
                bottom: rect.h + rect.y - canvas.h,
            }],
        },
    )
}
fn polygon(points: &[[f32; 2]]) -> Vec<c::PathCommand> {
    let mut out = vec![c::PathCommand::Move(points[0])];
    out.extend(points[1..].iter().copied().map(c::PathCommand::Line));
    out.push(c::PathCommand::Line(points[0]));
    out
}
fn ellipse(r: mm::Rect) -> Vec<c::PathCommand> {
    let [cx, cy, rx, ry] = [r.x + r.w / 2., r.y + r.h / 2., r.w / 2., r.h / 2.];
    let k = 0.552_284_8;
    vec![
        c::PathCommand::Move([cx + rx, cy]),
        c::PathCommand::Cubic([
            [cx + rx, cy + k * ry],
            [cx + k * rx, cy + ry],
            [cx, cy + ry],
        ]),
        c::PathCommand::Cubic([
            [cx - k * rx, cy + ry],
            [cx - rx, cy + k * ry],
            [cx - rx, cy],
        ]),
        c::PathCommand::Cubic([
            [cx - rx, cy - k * ry],
            [cx - k * rx, cy - ry],
            [cx, cy - ry],
        ]),
        c::PathCommand::Cubic([
            [cx + k * rx, cy - ry],
            [cx + rx, cy - k * ry],
            [cx + rx, cy],
        ]),
    ]
}
fn rounded_rect(r: mm::Rect, radius: f32) -> Vec<c::PathCommand> {
    let a = radius.min(r.w / 2.).min(r.h / 2.);
    let k = 0.552_284_8 * a;
    vec![
        c::PathCommand::Move([r.x + a, r.y]),
        c::PathCommand::Line([r.right() - a, r.y]),
        c::PathCommand::Cubic([
            [r.right() - a + k, r.y],
            [r.right(), r.y + a - k],
            [r.right(), r.y + a],
        ]),
        c::PathCommand::Line([r.right(), r.bottom() - a]),
        c::PathCommand::Cubic([
            [r.right(), r.bottom() - a + k],
            [r.right() - a + k, r.bottom()],
            [r.right() - a, r.bottom()],
        ]),
        c::PathCommand::Line([r.x + a, r.bottom()]),
        c::PathCommand::Cubic([
            [r.x + a - k, r.bottom()],
            [r.x, r.bottom() - a + k],
            [r.x, r.bottom() - a],
        ]),
        c::PathCommand::Line([r.x, r.y + a]),
        c::PathCommand::Cubic([[r.x, r.y + a - k], [r.x + a - k, r.y], [r.x + a, r.y]]),
    ]
}
fn cylinder(r: mm::Rect) -> (Vec<c::PathCommand>, Vec<c::PathCommand>) {
    let h = 6_f32.min(r.h / 4.);
    let (x, y, right, bottom, mid) = (r.x, r.y + h, r.right(), r.bottom() - h, r.x + r.w / 2.);
    let (kx, ky) = (r.w / 2. * 0.552_284_8, h * 0.552_284_8);
    let body = vec![
        c::PathCommand::Move([x, y]),
        c::PathCommand::Cubic([[x, y - ky], [mid - kx, r.y], [mid, r.y]]),
        c::PathCommand::Cubic([[mid + kx, r.y], [right, y - ky], [right, y]]),
        c::PathCommand::Line([right, bottom]),
        c::PathCommand::Cubic([
            [right, bottom + ky],
            [mid + kx, r.bottom()],
            [mid, r.bottom()],
        ]),
        c::PathCommand::Cubic([[mid - kx, r.bottom()], [x, bottom + ky], [x, bottom]]),
        c::PathCommand::Line([x, y]),
    ];
    let cap = vec![
        c::PathCommand::Move([x, y]),
        c::PathCommand::Cubic([[x, y + ky], [mid - kx, y + h], [mid, y + h]]),
        c::PathCommand::Cubic([[mid + kx, y + h], [right, y + ky], [right, y]]),
    ];
    (body, cap)
}
fn placed(
    e: c::Element,
    x: f32,
    y: f32,
    height: f32,
    ctx: ui::Context,
    m: &dyn c::Measure,
) -> c::Placed {
    let h = e
        .natural(f32::INFINITY, ctx, m)
        .expect("valid retained diagram geometry")
        .0[1];
    c::Placed {
        content: e,
        left: x,
        bottom: height - y - h,
    }
}
fn label(l: &mm::Label, key: &str, color: ui::Color) -> c::Element {
    let mut f = font(l.role);
    f.paint = ui::Paint::solid(color);
    let mut t = c::Text::new(l.lines.join("\n"), f);
    t.wrap = false;
    t.align = c::Alignment::Center;
    let mut s = plain();
    s.overrides.width = Some(ui::Length::Px(l.rect.w));
    s.overrides.height = Some(ui::Length::Px(l.rect.h));
    let mut b = c::Box::new(s, vec![c::Element::Text(t).into()]);
    b.direction = Axis::Column;
    b.align = c::Alignment::Stretch;
    b.gap = ui::Length::Px(0.);
    named(key, c::Element::Box(b))
}
fn direction(a: [f32; 2], b: [f32; 2]) -> ([f32; 2], f32) {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    let length = dx.hypot(dy);
    if length <= 0.000_001 {
        ([0., 0.], 0.)
    } else {
        ([dx / length, dy / length], length)
    }
}
fn quad(out: &mut Vec<c::PathCommand>, from: [f32; 2], control: [f32; 2], to: [f32; 2]) {
    out.push(c::PathCommand::Cubic([
        [
            from[0] + (control[0] - from[0]) * 2. / 3.,
            from[1] + (control[1] - from[1]) * 2. / 3.,
        ],
        [
            to[0] + (control[0] - to[0]) * 2. / 3.,
            to[1] + (control[1] - to[1]) * 2. / 3.,
        ],
        to,
    ]));
}
fn head_length(a: mm::Arrow) -> f32 {
    match a {
        mm::Arrow::Filled => 7.,
        mm::Arrow::Circle => 6.,
        mm::Arrow::Cross => 3.,
        _ => 0.,
    }
}
fn edge_points(e: &mm::Edge, radius: f32) -> Vec<c::PathCommand> {
    let mut points: Vec<_> = e.points.iter().copied().map(p).collect();
    if points.len() < 2 {
        return vec![];
    }
    let end = points.len() - 1;
    let (u, len) = direction(points[0], points[1]);
    let cut = head_length(e.start).min(len * 0.45);
    points[0] = [points[0][0] + u[0] * cut, points[0][1] + u[1] * cut];
    let (u, len) = direction(points[end], points[end - 1]);
    let cut = head_length(e.end).min(len * 0.45);
    points[end] = [points[end][0] + u[0] * cut, points[end][1] + u[1] * cut];
    let mut out = vec![c::PathCommand::Move(points[0])];
    for triple in points.windows(3) {
        let [a, b, c] = [triple[0], triple[1], triple[2]];
        let (u, l1) = direction(a, b);
        let (v, l2) = direction(b, c);
        let r = radius.min(l1 * 0.5).min(l2 * 0.5);
        if r < 0.001 || u[0] * v[0] + u[1] * v[1] > 0.999_99 {
            out.push(c::PathCommand::Line(b));
        } else {
            let from = [b[0] - u[0] * r, b[1] - u[1] * r];
            let to = [b[0] + v[0] * r, b[1] + v[1] * r];
            out.push(c::PathCommand::Line(from));
            quad(&mut out, from, b, to);
        }
    }
    out.push(c::PathCommand::Line(points[end]));
    out
}
fn arrow(
    key: &str,
    size: mm::Size,
    kind: mm::Arrow,
    tip: [f32; 2],
    next: [f32; 2],
    color: ui::Color,
) -> Option<c::Element> {
    let (u, len) = direction(next, tip);
    if kind == mm::Arrow::None || len == 0. {
        return None;
    }
    let base = [tip[0] - u[0] * 7., tip[1] - u[1] * 7.];
    let n = [-u[1] * 3.5, u[0] * 3.5];
    let left = [base[0] + n[0], base[1] + n[1]];
    let right = [base[0] - n[0], base[1] - n[1]];
    let (points, fill, weight) = match kind {
        mm::Arrow::Filled => (
            polygon(&[left, tip, right]),
            Some(ui::Paint::solid(color)),
            0.,
        ),
        mm::Arrow::Open => (
            vec![
                c::PathCommand::Move(left),
                c::PathCommand::Line(tip),
                c::PathCommand::Line(right),
            ],
            None,
            1.2,
        ),
        mm::Arrow::Circle => (
            ellipse(mm::Rect {
                x: tip[0] - u[0] * 3. - 3.,
                y: tip[1] - u[1] * 3. - 3.,
                w: 6.,
                h: 6.,
            }),
            None,
            1.2,
        ),
        mm::Arrow::Cross => {
            let a = [tip[0] - u[0] * 3., tip[1] - u[1] * 3.];
            (
                vec![
                    c::PathCommand::Move([a[0] - 3., a[1] - 3.]),
                    c::PathCommand::Line([a[0] + 3., a[1] + 3.]),
                    c::PathCommand::Move([a[0] + 3., a[1] - 3.]),
                    c::PathCommand::Line([a[0] - 3., a[1] + 3.]),
                ],
                None,
                1.2,
            )
        }
        mm::Arrow::None => unreachable!(),
    };
    Some(path(
        key,
        size,
        points,
        c::PathStroke {
            width: weight,
            cap: c::LineCap::Round,
            join: c::LineJoin::Round,
            ..Default::default()
        },
        ui::Paint::solid(color),
        fill,
    ))
}

fn node_color(n: &mm::Node) -> ui::Color {
    let color = n.classes.iter().find_map(|c| match c.as_str() {
        "oracle" => Some(TOKENS.color.role.oracle),
        "executor" => Some(TOKENS.color.role.executor),
        "creativity" => Some(TOKENS.color.role.creativity),
        "waiting" => Some(TOKENS.color.state.waiting),
        "errored" => Some(TOKENS.color.state.errored),
        _ => None,
    });
    color
        .map(|s| ui::Color::Rgba(Rgba::hex(s).0))
        .unwrap_or(if n.classes.is_empty() {
            ui::Color::Muted
        } else {
            ui::Color::Accent
        })
}
fn dash(g: &mm::Geometry, n: &mm::Node) -> Vec<f32> {
    let styles = n.style.iter().map(String::as_str).chain(
        g.class_defs
            .iter()
            .filter(|c| n.classes.contains(&c.name))
            .map(|c| c.style.as_str()),
    );
    for style in styles {
        for declaration in style.split(';') {
            if let Some((name, value)) = declaration.split_once(':')
                && name.trim() == "stroke-dasharray"
            {
                let values = value
                    .split([',', ' '])
                    .filter(|s| !s.is_empty())
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>();
                if let Ok(values) = values
                    && values.len() <= 256
                    && values.iter().all(|v| v.is_finite() && *v >= 0.)
                {
                    return values;
                }
            }
        }
    }
    vec![]
}
fn node_paths(shape: mm::Shape, r: mm::Rect) -> Vec<c::PathCommand> {
    let (x, y, w, h) = (r.x, r.y, r.w, r.h);
    let d = (h * 0.3).min(w * 0.25);
    match shape {
        mm::Shape::Diamond | mm::Shape::Choice => polygon(&[
            [x + w / 2., y],
            [x + w, y + h / 2.],
            [x + w / 2., y + h],
            [x, y + h / 2.],
        ]),
        mm::Shape::Hexagon => polygon(&[
            [x + d, y],
            [x + w - d, y],
            [x + w, y + h / 2.],
            [x + w - d, y + h],
            [x + d, y + h],
            [x, y + h / 2.],
        ]),
        mm::Shape::Asymmetric => polygon(&[
            [x, y],
            [x + w, y],
            [x + w, y + h],
            [x, y + h],
            [x + d, y + h / 2.],
        ]),
        mm::Shape::Parallelogram => {
            polygon(&[[x + d, y], [x + w, y], [x + w - d, y + h], [x, y + h]])
        }
        mm::Shape::ParallelogramAlt => {
            polygon(&[[x, y], [x + w - d, y], [x + w, y + h], [x + d, y + h]])
        }
        mm::Shape::Trapezoid => polygon(&[[x + d, y], [x + w - d, y], [x + w, y + h], [x, y + h]]),
        mm::Shape::TrapezoidAlt => {
            polygon(&[[x, y], [x + w, y], [x + w - d, y + h], [x + d, y + h]])
        }
        _ => polygon(&[[x, y], [x + w, y], [x + w, y + h], [x, y + h]]),
    }
}

pub fn canvas(g: &mm::Geometry, ctx: ui::Context, m: &dyn c::Measure) -> c::Element {
    let size = g.size;
    let mut overlays = vec![];
    let mut add = |e, x, y| overlays.push(placed(e, x, y, size.h, ctx, m));
    for (i, cluster) in g.clusters.iter().enumerate() {
        let r = cluster.rect;
        add(
            panel(
                &format!("cluster/{i}"),
                [r.w, r.h],
                STYLE.radius.tag as f32,
                paint(ui::Color::Surface, STYLE.material.card_alpha as f32),
                paint(ui::Color::Line, 0.7),
                1.,
            ),
            r.x,
            r.y,
        );
        if cluster.kind != mm::ClusterKind::Subgraph {
            let role = mm::TextRole::FrameLabel;
            let word = mm::Label {
                lines: vec![cluster.id.clone()],
                role,
                rect: mm::Rect {
                    x: r.x + 4.,
                    y: r.y + 2.,
                    w: m.width(&cluster.id, &measure_style(font(role))) + 4.,
                    h: font(role).step.pixels() * 1.2,
                },
            };
            add(
                label(&word, &format!("cluster/{i}/kind"), ui::Color::Foreground),
                word.rect.x,
                word.rect.y,
            );
        }
        if let Some(l) = &cluster.title {
            add(
                label(l, &format!("cluster/{i}/title"), ui::Color::Muted),
                l.rect.x,
                l.rect.y,
            );
        }
        for (j, d) in cluster.dividers.iter().enumerate() {
            add(
                path(
                    format!("cluster/{i}/divider{j}"),
                    size,
                    vec![
                        c::PathCommand::Move([r.x, d.y]),
                        c::PathCommand::Line([r.right(), d.y]),
                    ],
                    c::PathStroke {
                        width: 1.,
                        dash: vec![4., 4.].into(),
                        ..Default::default()
                    },
                    ui::Paint::solid(ui::Color::Line),
                    None,
                ),
                0.,
                0.,
            );
            if let Some(l) = &d.label {
                add(
                    label(
                        l,
                        &format!("cluster/{i}/divider{j}/label"),
                        ui::Color::Muted,
                    ),
                    l.rect.x,
                    l.rect.y,
                );
            }
        }
    }
    for (i, line) in g.lines.iter().enumerate() {
        if line.stroke == mm::Stroke::Invisible {
            continue;
        }
        add(
            path(
                format!("lifeline/{i}"),
                size,
                vec![
                    c::PathCommand::Move(p(line.a)),
                    c::PathCommand::Line(p(line.b)),
                ],
                c::PathStroke {
                    width: 1.2,
                    dash: if line.stroke == mm::Stroke::Dotted {
                        vec![3., 4.]
                    } else {
                        vec![]
                    }
                    .into(),
                    ..Default::default()
                },
                paint(ui::Color::Line, 0.7),
                None,
            ),
            0.,
            0.,
        );
    }
    for (i, node) in g.nodes.iter().enumerate() {
        let r = node.rect;
        let co = node_color(node);
        let fill = ui::Paint::solid(mix(
            co,
            ui::Color::Surface,
            if node.classes.is_empty() { 0.1 } else { 0.2 },
            ctx,
        ));
        let stroke = paint(co, if node.classes.is_empty() { 0.7 } else { 1. });
        let key = format!("node{i}");
        match node.shape {
            mm::Shape::Start | mm::Shape::End | mm::Shape::Bar => {
                let radius = if node.shape == mm::Shape::Bar {
                    0.
                } else {
                    r.w / 2.
                };
                add(
                    panel(
                        &key,
                        [r.w, r.h],
                        radius,
                        if node.shape == mm::Shape::End {
                            ui::Paint::solid(ui::Color::Transparent)
                        } else {
                            ui::Paint::solid(ui::Color::Foreground)
                        },
                        ui::Paint::solid(ui::Color::Foreground),
                        if node.shape == mm::Shape::End {
                            1.4
                        } else {
                            0.
                        },
                    ),
                    r.x,
                    r.y,
                );
                if node.shape == mm::Shape::End {
                    add(
                        panel(
                            &format!("{key}/dot"),
                            [9., 9.],
                            4.5,
                            ui::Paint::solid(ui::Color::Foreground),
                            stroke,
                            0.,
                        ),
                        r.x + (r.w - 9.) / 2.,
                        r.y + (r.h - 9.) / 2.,
                    );
                }
            }
            mm::Shape::Actor => {
                add(
                    panel(
                        &format!("{key}/head"),
                        [10., 10.],
                        5.,
                        ui::Paint::solid(ui::Color::Transparent),
                        stroke,
                        1.2,
                    ),
                    r.x + r.w / 2. - 5.,
                    r.y + 2.,
                );
                let x = r.x + r.w / 2.;
                add(
                    node_path(
                        &key,
                        r,
                        size,
                        vec![
                            c::PathCommand::Move([x, r.y + 12.]),
                            c::PathCommand::Line([x, r.y + 28.]),
                            c::PathCommand::Move([x - 9., r.y + 18.]),
                            c::PathCommand::Line([x + 9., r.y + 18.]),
                            c::PathCommand::Move([x - 8., r.y + 38.]),
                            c::PathCommand::Line([x, r.y + 28.]),
                            c::PathCommand::Line([x + 8., r.y + 38.]),
                        ],
                        c::PathStroke::default(),
                        stroke,
                        None,
                    ),
                    r.x,
                    r.y,
                );
            }
            mm::Shape::Cylinder => {
                let (body, cap) = cylinder(r);
                add(
                    node_path(
                        &key,
                        r,
                        size,
                        body,
                        c::PathStroke {
                            width: 1.2,
                            dash: dash(g, node).into(),
                            ..Default::default()
                        },
                        stroke,
                        Some(fill),
                    ),
                    r.x,
                    r.y,
                );
                add(
                    node_path(
                        &format!("{key}/cap"),
                        r,
                        size,
                        cap,
                        c::PathStroke {
                            width: 1.2,
                            ..Default::default()
                        },
                        stroke,
                        None,
                    ),
                    r.x,
                    r.y,
                );
            }
            mm::Shape::Diamond
            | mm::Shape::Choice
            | mm::Shape::Hexagon
            | mm::Shape::Asymmetric
            | mm::Shape::Parallelogram
            | mm::Shape::ParallelogramAlt
            | mm::Shape::Trapezoid
            | mm::Shape::TrapezoidAlt => {
                add(
                    node_path(
                        &key,
                        r,
                        size,
                        node_paths(node.shape, r),
                        c::PathStroke {
                            width: 1.2,
                            dash: dash(g, node).into(),
                            join: c::LineJoin::Round,
                            ..Default::default()
                        },
                        stroke,
                        Some(fill),
                    ),
                    r.x,
                    r.y,
                );
            }
            _ => {
                let radius = match node.shape {
                    mm::Shape::Circle | mm::Shape::DoubleCircle | mm::Shape::Stadium => r.h / 2.,
                    mm::Shape::Rect
                    | mm::Shape::Rounded
                    | mm::Shape::Participant
                    | mm::Shape::Subroutine => 8.,
                    mm::Shape::Note => 6.,
                    _ => 0.,
                };
                let (fill, stroke) = if node.shape == mm::Shape::Note {
                    (
                        ui::Paint::solid(mix(
                            ui::Color::Rgba(Rgba::hex(TOKENS.color.state.waiting).0),
                            ui::Color::Surface,
                            0.12,
                            ctx,
                        )),
                        paint(
                            ui::Color::Rgba(Rgba::hex(TOKENS.color.state.waiting).0),
                            0.45,
                        ),
                    )
                } else {
                    (fill, stroke)
                };
                let dashes = dash(g, node);
                add(
                    panel(
                        &key,
                        [r.w, r.h],
                        radius,
                        fill,
                        stroke,
                        if dashes.is_empty() { 1.2 } else { 0. },
                    ),
                    r.x,
                    r.y,
                );
                if !dashes.is_empty() {
                    add(
                        path(
                            format!("{key}/outline"),
                            size,
                            if matches!(node.shape, mm::Shape::Circle | mm::Shape::DoubleCircle) {
                                ellipse(r)
                            } else {
                                rounded_rect(r, radius)
                            },
                            c::PathStroke {
                                width: 1.2,
                                dash: dashes.into(),
                                ..Default::default()
                            },
                            stroke,
                            None,
                        ),
                        0.,
                        0.,
                    );
                }
                if node.shape == mm::Shape::DoubleCircle {
                    add(
                        panel(
                            &format!("{key}/inner"),
                            [r.w - 8., r.h - 8.],
                            (r.h - 8.) / 2.,
                            ui::Paint::solid(ui::Color::Transparent),
                            stroke,
                            1.2,
                        ),
                        r.x + 4.,
                        r.y + 4.,
                    );
                }
                if node.shape == mm::Shape::Subroutine {
                    add(
                        path(
                            format!("{key}/inner"),
                            size,
                            vec![
                                c::PathCommand::Move([r.x + 8., r.y]),
                                c::PathCommand::Line([r.x + 8., r.bottom()]),
                                c::PathCommand::Move([r.right() - 8., r.y]),
                                c::PathCommand::Line([r.right() - 8., r.bottom()]),
                            ],
                            c::PathStroke {
                                width: 1.2,
                                ..Default::default()
                            },
                            stroke,
                            None,
                        ),
                        0.,
                        0.,
                    );
                }
            }
        }
        if let Some(l) = &node.label {
            add(
                label(
                    l,
                    &format!("label{i}"),
                    if node.link.is_some() {
                        ui::Color::Accent
                    } else {
                        ui::Color::Foreground
                    },
                ),
                l.rect.x,
                l.rect.y,
            );
        }
    }
    for (i, edge) in g.edges.iter().enumerate() {
        if edge.stroke == mm::Stroke::Invisible {
            continue;
        }
        let co = if edge.stroke == mm::Stroke::Thick {
            ui::Color::Foreground
        } else {
            ui::Color::Muted
        };
        let stroke = c::PathStroke {
            width: if edge.stroke == mm::Stroke::Thick {
                2.4
            } else {
                1.2
            },
            dash: if edge.stroke == mm::Stroke::Dotted {
                vec![4., 4.]
            } else {
                vec![]
            }
            .into(),
            cap: c::LineCap::Round,
            join: c::LineJoin::Round,
        };
        add(
            path(
                format!("edge/{i}"),
                size,
                edge_points(edge, g.corner_radius),
                stroke,
                paint(co, 0.85),
                None,
            ),
            0.,
            0.,
        );
        if edge.points.len() >= 2 {
            let last = edge.points.len() - 1;
            for (name, kind, tip, next) in [
                ("start", edge.start, p(edge.points[0]), p(edge.points[1])),
                (
                    "end",
                    edge.end,
                    p(edge.points[last]),
                    p(edge.points[last - 1]),
                ),
            ] {
                if let Some(head) = arrow(&format!("edge/{i}/{name}"), size, kind, tip, next, co) {
                    add(head, 0., 0.);
                }
            }
        }
        if let Some(l) = &edge.label {
            if g.kind != mm::Kind::Sequence {
                add(
                    panel(
                        &format!("edge/{i}/pill"),
                        [l.rect.w + 6., l.rect.h + 4.],
                        STYLE.radius.tag as f32,
                        ui::Paint::solid(ui::Color::Background),
                        paint(ui::Color::Line, 0.7),
                        1.,
                    ),
                    l.rect.x - 3.,
                    l.rect.y - 2.,
                );
            }
            add(
                label(l, &format!("edge/{i}/label"), ui::Color::Muted),
                l.rect.x,
                l.rect.y,
            );
        }
    }
    named(
        "diagram-canvas",
        c::Element::Layers {
            base: Box::new(panel(
                "diagram-size",
                [size.w, size.h],
                0.,
                ui::Paint::solid(ui::Color::Transparent),
                ui::Paint::solid(ui::Color::Transparent),
                0.,
            )),
            overlays,
        },
    )
}
