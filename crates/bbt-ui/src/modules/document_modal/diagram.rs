//! Static flow-figure composition. Input is semantic nodes/edges; layout and
//! paths are native. Full Markdown/Mermaid parsing belongs to bbt-doc later.
use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub shape: String,
    pub cls: Option<String>,
    pub link: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub dashed: bool,
    pub thick: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FlowGraph {
    #[serde(rename = "type")]
    pub kind: String,
    pub dir: String,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Diagram {
    pub source: String,
    pub graph: Option<FlowGraph>,
    pub error: Option<String>,
    pub show_source: bool,
    pub scroll_x: f32,
}
#[derive(Clone)]
struct Vertex {
    id: String,
    node: Option<usize>,
    rank: usize,
    w: f32,
    h: f32,
    x: f32,
    y: f32,
    main: f32,
    cross: f32,
    total: f32,
}
struct Edge {
    source: usize,
    target: usize,
    back: bool,
    chain: Vec<usize>,
}
struct Geometry {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    size: [f32; 2],
    cross_max: f32,
}
fn tw(v: &str) -> f32 {
    (v.chars().count() as f32 * 6.6).max(24.)
}
fn node_size(n: &FlowNode) -> [f32; 2] {
    match n.shape.as_str() {
        "start" | "end" => [18., 18.],
        "circle" => {
            let n = (tw(&n.label) + 24.).max(40.);
            [n, n]
        }
        "diamond" => [tw(&n.label) + 38., 48.],
        _ => [tw(&n.label) + 22., 32.],
    }
}
impl FlowGraph {
    fn geometry(&self) -> Geometry {
        let mut vertices: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let [w, h] = node_size(n);
                Vertex {
                    id: n.id.clone(),
                    node: Some(i),
                    rank: 0,
                    w,
                    h,
                    x: 0.,
                    y: 0.,
                    main: 0.,
                    cross: 0.,
                    total: 0.,
                }
            })
            .collect();
        let mut edges: Vec<_> = self
            .edges
            .iter()
            .filter_map(|e| {
                Some(Edge {
                    source: vertices.iter().position(|v| v.id == e.from)?,
                    target: vertices.iter().position(|v| v.id == e.to)?,
                    back: false,
                    chain: vec![],
                })
            })
            .collect();
        fn visit(i: usize, seen: &mut [u8], edges: &mut [Edge]) {
            seen[i] = 1;
            for j in 0..edges.len() {
                if edges[j].source != i {
                    continue;
                }
                let to = edges[j].target;
                if seen[to] == 1 {
                    edges[j].back = true
                } else if seen[to] == 0 {
                    visit(to, seen, edges)
                }
            }
            seen[i] = 2;
        }
        let mut seen = vec![0; vertices.len()];
        let mut order: Vec<_> = (0..vertices.len())
            .filter(|i| !edges.iter().any(|e| e.target == *i))
            .collect();
        order.extend(0..vertices.len());
        for i in order {
            if seen[i] == 0 {
                visit(i, &mut seen, &mut edges)
            }
        }
        for _ in 0..vertices.len() {
            for e in &edges {
                if !e.back {
                    vertices[e.target].rank =
                        vertices[e.target].rank.max(vertices[e.source].rank + 1);
                }
            }
        }
        let mut layers: Vec<Vec<usize>> =
            vec![vec![]; vertices.iter().map(|v| v.rank).max().unwrap_or(0) + 1];
        for (i, v) in vertices.iter().enumerate() {
            layers[v.rank].push(i)
        }
        let lr = self.dir == "LR";
        for (i, e) in edges.iter_mut().enumerate() {
            if e.back {
                continue;
            }
            e.chain.push(e.source);
            let from_rank = vertices[e.source].rank + 1;
            let to_rank = vertices[e.target].rank;
            for (rank, layer) in layers.iter_mut().enumerate().take(to_rank).skip(from_rank) {
                let at = vertices.len();
                vertices.push(Vertex {
                    id: format!("~{i}_{rank}"),
                    node: None,
                    rank,
                    w: if lr { 2. } else { 10. },
                    h: if lr { 10. } else { 2. },
                    x: 0.,
                    y: 0.,
                    main: 0.,
                    cross: 0.,
                    total: 0.,
                });
                layer.push(at);
                e.chain.push(at)
            }
            e.chain.push(e.target);
        }
        let links: Vec<_> = edges
            .iter()
            .flat_map(|e| e.chain.windows(2).map(|p| (p[0], p[1])))
            .collect();
        fn bary(layer: &mut [usize], reference: &[usize], links: &[(usize, usize)], forward: bool) {
            let scores: std::collections::BTreeMap<_, _> = layer
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let adj: Vec<_> = links
                        .iter()
                        .filter_map(|(a, b)| {
                            if if forward { b == v } else { a == v } {
                                reference
                                    .iter()
                                    .position(|r| *r == if forward { *a } else { *b })
                            } else {
                                None
                            }
                        })
                        .collect();
                    (
                        *v,
                        if adj.is_empty() {
                            i as f32
                        } else {
                            adj.iter().sum::<usize>() as f32 / adj.len() as f32
                        },
                    )
                })
                .collect();
            layer.sort_by(|a, b| scores[a].total_cmp(&scores[b]));
        }
        for _ in 0..3 {
            for r in 1..layers.len() {
                let (lo, hi) = layers.split_at_mut(r);
                bary(&mut hi[0], &lo[r - 1], &links, true)
            }
            for r in (0..layers.len().saturating_sub(1)).rev() {
                let (lo, hi) = layers.split_at_mut(r + 1);
                bary(&mut lo[r], &hi[0], &links, false)
            }
        }
        let (main_gap, cross_gap) = if lr { (74., 22.) } else { (58., 28.) };
        let (mut main, mut cross_max) = (14., 0_f32);
        for layer in &layers {
            let thick = layer
                .iter()
                .map(|i| if lr { vertices[*i].w } else { vertices[*i].h })
                .fold(0., f32::max);
            let total = layer
                .iter()
                .map(|i| if lr { vertices[*i].h } else { vertices[*i].w })
                .sum::<f32>()
                + cross_gap * layer.len().saturating_sub(1) as f32;
            cross_max = cross_max.max(total);
            let mut cross = 0.;
            for i in layer {
                let v = &mut vertices[*i];
                let extent = if lr { v.h } else { v.w };
                v.main = main + thick / 2.;
                v.cross = cross + extent / 2.;
                v.total = total;
                cross += extent + cross_gap;
            }
            main += thick + main_gap;
        }
        for v in &mut vertices {
            v.cross += (cross_max - v.total) / 2. + 14.;
            v.x = if lr { v.main } else { v.cross };
            v.y = if lr { v.cross } else { v.main };
        }
        let backs = edges.iter().filter(|e| e.back).count();
        Geometry {
            vertices,
            edges,
            size: [
                if lr {
                    main - main_gap + 28.
                } else {
                    cross_max + 28.
                },
                if lr {
                    cross_max
                        + 28.
                        + if backs > 0 {
                            20. + backs as f32 * 14.
                        } else {
                            0.
                        }
                } else {
                    main - main_gap + 28.
                },
            ],
            cross_max,
        }
    }
}
fn add(
    over: &mut Vec<c::Placed>,
    e: c::Element,
    p: [f32; 2],
    height: f32,
    ctx: ui::Context,
    m: &dyn c::Measure,
) {
    let h = size(&e, f32::INFINITY, ctx, m)[1];
    over.push(c::Placed {
        content: e,
        left: p[0],
        bottom: height - p[1] - h,
    })
}
fn svg_text(
    key: &str,
    v: &str,
    xy: [f32; 2],
    f: ui::Text,
    ctx: ui::Context,
    m: &dyn c::Measure,
) -> (c::Element, [f32; 2]) {
    let mut t = c::Text::new(v, f);
    t.wrap = false;
    t.inline = true;
    let e = named(key, c::Element::Text(t));
    let (sz, base) = e.natural(f32::INFINITY, ctx, m).unwrap();
    let [a, d] = m
        .font_metrics(&MeasureStyle {
            size: f.size.unwrap(),
            weight: f.weight,
            mono: f.mono,
            italic: f.italic,
            tracking_em: f.tracking_em,
        })
        .map(f32::round);
    (e, [xy[0] - sz[0] / 2., xy[1] + (a - d) / 2. - base])
}
fn quad(out: &mut Vec<c::PathCommand>, from: [f32; 2], control: [f32; 2], to: [f32; 2]) {
    out.push(c::PathCommand::Cubic([
        [
            from[0] + 2. / 3. * (control[0] - from[0]),
            from[1] + 2. / 3. * (control[1] - from[1]),
        ],
        [
            to[0] + 2. / 3. * (control[0] - to[0]),
            to[1] + 2. / 3. * (control[1] - to[1]),
        ],
        to,
    ]));
}
fn step(out: &mut Vec<c::PathCommand>, a: [f32; 2], b: [f32; 2], lr: bool) {
    if lr {
        if (a[1] - b[1]).abs() < 1. {
            out.push(c::PathCommand::Line(b));
            return;
        }
        let mid = (a[0] + b[0]) / 2.;
        let r = 8_f32.min((b[1] - a[1]).abs() / 2.).min((mid - a[0]).abs());
        let sign = (b[1] - a[1]).signum();
        let p = [mid - r, a[1]];
        out.push(c::PathCommand::Line(p));
        quad(out, p, [mid, a[1]], [mid, a[1] + sign * r]);
        let p = [mid, b[1] - sign * r];
        out.push(c::PathCommand::Line(p));
        quad(out, p, [mid, b[1]], [mid + r, b[1]]);
        out.push(c::PathCommand::Line(b));
    } else {
        let swap = |p: [f32; 2]| [p[1], p[0]];
        let mut trans = vec![];
        step(&mut trans, swap(a), swap(b), true);
        out.extend(trans.into_iter().map(|p| match p {
            c::PathCommand::Move(p) => c::PathCommand::Move(swap(p)),
            c::PathCommand::Line(p) => c::PathCommand::Line(swap(p)),
            c::PathCommand::Cubic(p) => c::PathCommand::Cubic(p.map(swap)),
        }));
    }
}
fn edge_path(
    key: &str,
    sz: [f32; 2],
    points: Vec<c::PathCommand>,
    stroke: f32,
    col: ui::Color,
    alpha: f32,
    dashed: bool,
) -> c::Element {
    let mut e = path(
        key,
        sz,
        points,
        stroke,
        paint(col, alpha),
        if dashed { vec![4., 4.] } else { vec![] },
    );
    if let c::Element::Named { content, .. } = &mut e
        && let c::Element::Path(p) = content.as_mut()
    {
        p.stroke.cap = c::LineCap::Round;
        p.stroke.join = c::LineJoin::Round;
    }
    e
}
fn arrow(key: &str, sz: [f32; 2], tip: [f32; 2], direction: [f32; 2], weight: f32) -> c::Element {
    // The static figure uses an L6 stroked marker; paths remain stroke-only.
    let u = weight * 0.7;
    let base = [
        tip[0] - direction[0] * 8. * u,
        tip[1] - direction[1] * 8. * u,
    ];
    let normal = [-direction[1] * 3.5 * u, direction[0] * 3.5 * u];
    let a = [base[0] + normal[0], base[1] + normal[1]];
    let b = [base[0] - normal[0], base[1] - normal[1]];
    path(
        key,
        sz,
        vec![
            c::PathCommand::Move(a),
            c::PathCommand::Line(tip),
            c::PathCommand::Line(b),
            c::PathCommand::Line(a),
            c::PathCommand::Move(base),
            c::PathCommand::Line(tip),
        ],
        weight * 0.75,
        paint(ui::Color::Muted, 0.95),
        vec![],
    )
}
impl Diagram {
    fn canvas(
        &self,
        g: &FlowGraph,
        key: &str,
        ctx: ui::Context,
        m: &dyn c::Measure,
    ) -> (c::Element, [f32; 2]) {
        let geo = g.geometry();
        let [w, h] = geo.size;
        let lr = g.dir == "LR";
        let mut over = vec![];
        let mut pills: Vec<(String, [f32; 2])> = vec![];
        let mut back = 0;
        for (i, (e, record)) in geo.edges.iter().zip(&g.edges).enumerate() {
            let a = &geo.vertices[e.source];
            let b = &geo.vertices[e.target];
            let (mut points, tip, dir);
            if e.back {
                // The document fixture uses an LR back lane. TD is retained for
                // future producers but uses the same external-lane construction.
                let lane = 14. + geo.cross_max + 14. + back as f32 * 14.;
                back += 1;
                let start = [a.x, a.y + a.h / 2.];
                let end = [b.x, b.y + b.h / 2. + 2.];
                points = vec![
                    c::PathCommand::Move(start),
                    c::PathCommand::Line([a.x, lane - 8.]),
                ];
                quad(&mut points, [a.x, lane - 8.], [a.x, lane], [a.x - 8., lane]);
                points.push(c::PathCommand::Line([b.x + 8., lane]));
                quad(&mut points, [b.x + 8., lane], [b.x, lane], [b.x, lane - 8.]);
                points.push(c::PathCommand::Line(end));
                tip = end;
                dir = [0., -1.];
                if !record.label.is_empty() {
                    pills.push((record.label.clone(), [(a.x + b.x) / 2., lane]));
                }
            } else {
                let start = if lr {
                    [a.x + a.w / 2., a.y]
                } else {
                    [a.x, a.y + a.h / 2.]
                };
                points = vec![c::PathCommand::Move(start)];
                let mut from = start;
                for (j, id) in e.chain.iter().enumerate().skip(1) {
                    let q = &geo.vertices[*id];
                    let last = j == e.chain.len() - 1;
                    let to = if lr {
                        [if last { q.x - q.w / 2. - 2. } else { q.x }, q.y]
                    } else {
                        [q.x, if last { q.y - q.h / 2. - 2. } else { q.y }]
                    };
                    step(&mut points, from, to, lr);
                    from = to;
                }
                tip = from;
                dir = if lr { [1., 0.] } else { [0., 1.] };
                if !record.label.is_empty() {
                    let q = &geo.vertices[e.chain[1]];
                    let x = if lr {
                        (a.x + a.w / 2. + q.x - if q.node.is_none() { 0. } else { q.w / 2. }) / 2.
                    } else {
                        q.x
                    };
                    let y = if lr {
                        q.y
                    } else {
                        (a.y + a.h / 2. + q.y - if q.node.is_none() { 0. } else { q.h / 2. }) / 2.
                            + if q.node.is_none() { 0. } else { 6. }
                    };
                    pills.push((record.label.clone(), [x, y]));
                }
            }
            let weight = if !e.back && record.thick { 2.4 } else { 1.2 };
            let co = if !e.back && record.thick {
                ui::Color::Foreground
            } else {
                ui::Color::Muted
            };
            let alpha = if e.back {
                0.65
            } else if record.thick {
                1.
            } else {
                0.85
            };
            add(
                &mut over,
                edge_path(
                    &format!("{key}/edge{i}"),
                    geo.size,
                    points,
                    weight,
                    co,
                    alpha,
                    record.dashed,
                ),
                [0.; 2],
                h,
                ctx,
                m,
            );
            add(
                &mut over,
                arrow(&format!("{key}/arrow{i}"), geo.size, tip, dir, weight),
                [0.; 2],
                h,
                ctx,
                m,
            );
        }
        for (i, n) in g.nodes.iter().enumerate() {
            let p = &geo.vertices[i];
            let nc = n
                .cls
                .as_deref()
                .map(|c| match c {
                    "oracle" => color(TOKENS.color.role.oracle),
                    "executor" => color(TOKENS.color.role.executor),
                    "creativity" => color(TOKENS.color.role.creativity),
                    "waiting" => color(TOKENS.color.state.waiting),
                    "errored" => color(TOKENS.color.state.errored),
                    _ => ui::Color::Accent,
                })
                .unwrap_or(ui::Color::Muted);
            let tinted = n.cls.is_some() && n.shape != "circle";
            let fill = Rgba(paint(nc, 1.).resolve(ctx).unwrap()).mix(
                Rgba(paint(ui::Color::Surface, 1.).resolve(ctx).unwrap()),
                if tinted { 0.8 } else { 0.9 },
            );
            let bc = ui::Color::Rgba(
                paint(nc, if tinted { 1. } else { 0.7 })
                    .resolve(ctx)
                    .unwrap(),
            );
            let mut style = surface(
                ui::Color::Rgba(fill.0),
                1.,
                match n.shape.as_str() {
                    "circle" | "stadium" => p.h / 2.,
                    "store" => 12.,
                    _ => 8.,
                },
                [0.; 4],
            );
            style.overrides.width = Some(ui::Length::Px(p.w));
            style.overrides.height = Some(ui::Length::Px(p.h));
            style = border(style, bc, 1.2, [true; 4], true);
            add(
                &mut over,
                row(format!("{key}/node{i}"), style, 0., vec![]),
                [p.x - p.w / 2., p.y - p.h / 2.],
                h,
                ctx,
                m,
            );
            if n.shape == "store" {
                let start = [p.x - p.w / 2. + 4., p.y - p.h / 2. + 7.];
                let end = [p.x + p.w / 2. - 4., start[1]];
                let mut pts = vec![c::PathCommand::Move(start)];
                quad(&mut pts, start, [p.x, p.y - p.h / 2. + 13.], end);
                add(
                    &mut over,
                    path(
                        &format!("{key}/store{i}"),
                        geo.size,
                        pts,
                        1.2,
                        ui::Paint::solid(bc),
                        vec![],
                    ),
                    [0.; 2],
                    h,
                    ctx,
                    m,
                );
            }
            if !n.label.is_empty() {
                let (e, at) = svg_text(
                    &format!("{key}/label{i}"),
                    &n.label,
                    [p.x, p.y],
                    font(
                        12.,
                        400,
                        false,
                        if n.link.is_some() {
                            ui::Color::Accent
                        } else {
                            ui::Color::Foreground
                        },
                    ),
                    ctx,
                    m,
                );
                add(&mut over, e, at, h, ctx, m)
            }
        }
        for (i, (value, xy)) in pills.iter().enumerate() {
            let pw = value.chars().count() as f32 * 5.8 + 10.;
            let mut style = border(
                surface(ui::Color::Background, 1., 8., [0.; 4]),
                hair(ctx),
                1.,
                [true; 4],
                true,
            );
            style.overrides.width = Some(ui::Length::Px(pw));
            style.overrides.height = Some(ui::Length::Px(16.));
            add(
                &mut over,
                row(format!("{key}/pill{i}"), style, 0., vec![]),
                [xy[0] - pw / 2., xy[1] - 8.],
                h,
                ctx,
                m,
            );
            let (e, at) = svg_text(
                &format!("{key}/pill{i}/text"),
                value,
                *xy,
                font(10.5, 400, false, ui::Color::Muted),
                ctx,
                m,
            );
            add(&mut over, e, at, h, ctx, m);
        }
        let mut base = plain(Some(h), [0.; 4], 0.);
        base.overrides.width = Some(ui::Length::Px(w));
        (
            named(
                format!("{key}/canvas"),
                c::Element::Layers {
                    base: Box::new(row("canvas", base, 0., vec![])),
                    overlays: over,
                },
            ),
            geo.size,
        )
    }
    pub(super) fn element(
        &self,
        key: &str,
        width: f32,
        ctx: ui::Context,
        m: &dyn c::Measure,
    ) -> c::Element {
        let Some(g) = self
            .graph
            .as_ref()
            .filter(|g| g.kind == "flow" && !g.nodes.is_empty())
        else {
            return col(
                format!("{key}/raw"),
                plain(None, [0.; 4], 0.),
                6.,
                vec![
                    text(
                        format!("{key}/note"),
                        format!(
                            "not drawn · {} · shown as source",
                            self.error
                                .as_deref()
                                .unwrap_or("unsupported static diagram")
                        ),
                        font(11., 400, false, color(TOKENS.color.state.waiting)),
                    )
                    .into(),
                    text(
                        format!("{key}/source"),
                        &self.source,
                        font(11., 400, true, ui::Color::Foreground),
                    )
                    .into(),
                ],
            );
        };
        let kind = if self.source.trim_start().starts_with("state") {
            "state"
        } else {
            "flowchart"
        };
        let mut header = vec![
            nowrap(
                format!("{key}/kind"),
                kind,
                ui::Text {
                    line_height: 1.6,
                    ..font(11., 400, true, ui::Color::Foreground)
                },
            )
            .into(),
            nowrap(
                format!("{key}/count"),
                format!(
                    "{} nodes",
                    g.nodes.iter().filter(|n| !n.label.is_empty()).count()
                ),
                ui::Text {
                    line_height: 1.6,
                    ..font(11., 400, false, ui::Color::Muted)
                },
            )
            .into(),
            fill(c::Element::Spacer),
        ];
        let mut seg = vec![];
        for title in ["Drawing", "Source"] {
            let on = (title == "Source") == self.show_source;
            let s = if on {
                border(
                    surface(ui::Color::Surface, 1., 999., [1., 8., 1., 8.]),
                    hair(ctx),
                    1.,
                    [true; 4],
                    true,
                )
            } else {
                plain(None, [1., 8., 1., 8.], 999.)
            };
            seg.push(
                row(
                    format!("{key}/button-{}", title.to_lowercase()),
                    s,
                    0.,
                    vec![
                        nowrap(
                            format!("{key}/{}/text", title.to_lowercase()),
                            title,
                            ui::Text {
                                line_height: 1.6,
                                ..font(
                                    11.,
                                    400,
                                    false,
                                    if on {
                                        ui::Color::Foreground
                                    } else {
                                        ui::Color::Muted
                                    },
                                )
                            },
                        )
                        .into(),
                    ],
                )
                .into(),
            );
        }
        header.push(
            row(
                format!("{key}/segment"),
                border(
                    surface(
                        ui::Color::Muted,
                        STYLE.tint.badge_alpha as f32,
                        999.,
                        [2.; 4],
                    ),
                    ui::Color::Line,
                    1.,
                    [true; 4],
                    false,
                ),
                0.,
                seg,
            )
            .into(),
        );
        header.push(
            row(
                format!("{key}/expand"),
                plain(None, [1., 8., 1., 8.], 999.),
                0.,
                vec![
                    nowrap(
                        format!("{key}/expand/text"),
                        "⤢",
                        ui::Text {
                            line_height: 1.6,
                            ..font(11., 400, false, ui::Color::Muted)
                        },
                    )
                    .into(),
                ],
            )
            .into(),
        );
        let head = row(
            format!("{key}/head"),
            border(
                plain(None, [6., 8., 6., 8.], 0.),
                hair(ctx),
                1.,
                [false, false, true, false],
                false,
            ),
            8.,
            header,
        );
        let body = if self.show_source {
            col(
                format!("{key}/drawing"),
                plain(None, [4., 8., 8., 8.], 0.),
                0.,
                vec![
                    col(
                        format!("{key}/source"),
                        border(
                            surface(
                                ui::Color::Surface,
                                1.,
                                STYLE.radius.row as f32,
                                [8., 10., 8., 10.],
                            ),
                            hair(ctx),
                            1.,
                            [true; 4],
                            false,
                        ),
                        0.,
                        vec![
                            nowrap(
                                format!("{key}/source/text"),
                                self.source.trim(),
                                ui::Text {
                                    line_height: 1.5,
                                    ..font(11., 400, true, ui::Color::Foreground)
                                },
                            )
                            .into(),
                        ],
                    )
                    .into(),
                ],
            )
        } else {
            let (canvas, sz) = self.canvas(g, key, ctx, m);
            let vw = (width - 16.).max(0.);
            let content = col(
                format!("{key}/canvas-pad"),
                plain(None, [6., 0., 6., 0.], 0.),
                0.,
                vec![fixed(
                    row(
                        format!("{key}/canvas-slot"),
                        plain(None, [0.; 4], 0.),
                        0.,
                        vec![fixed(canvas, sz[0])],
                    ),
                    sz[1],
                )],
            );
            let view = c::Element::ScrollView(c::ScrollView {
                viewport: [vw, sz[1] + 12.],
                offset: [self.scroll_x, 0.],
                content_width: Some(sz[0]),
                content: Box::new(content),
            });
            let view = if sz[0] > vw + 0.5 {
                let mut group = c::Box::new(plain(None, [0.; 4], 0.), vec![view.into()]);
                group.direction = Axis::Column;
                group.align = c::Alignment::Stretch;
                group.gap = ui::Length::Px(0.);
                group.mask = Some(c::LinearAlphaMask {
                    start: [vw - 28., 0.],
                    end: [vw, 0.],
                    alpha0: 1.,
                    alpha1: 0.,
                });
                c::Element::Box(group)
            } else {
                view
            };
            col(
                format!("{key}/drawing"),
                plain(None, [4., 8., 8., 8.], 0.),
                0.,
                vec![view.into()],
            )
        };
        let mut box_ = c::Box::new(
            border(
                surface(
                    ui::Color::Surface,
                    STYLE.material.card_alpha as f32,
                    STYLE.radius.card as f32,
                    [0.; 4],
                ),
                hair(ctx),
                1.,
                [true; 4],
                true,
            ),
            vec![head.into(), body.into()],
        );
        box_.direction = Axis::Column;
        box_.align = c::Alignment::Stretch;
        box_.gap = ui::Length::Px(0.);
        box_.clip = true;
        named(format!("{key}/figure"), c::Element::Box(box_))
    }
}
