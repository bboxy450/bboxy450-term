// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Static semantic canvas projection. World math and culling precede f32 output.
//! Routing, label placement and the look of every element live here, in one
//! file, so a change to how the picture reads is a change to this file alone.
use crate::{self as ui, components as c};
use bboxy450_canvas::{
    Align, Bounds, Camera, Canvas, Content, Detail, Element, Kind, Route, Shape, Side, Status,
    TextRole, semantic_zoom,
};
use bbt_ui_model::{STYLE, TOKENS, Theme, draw::Color, layout::Rect};
use c::{LineCap, LineJoin, PathCommand as P, PathStroke};
use ui::TypeStep;

type Pt = [f64; 2];

/// Viewport is in logical pixels. No font, IO, renderer or application state is
/// needed to compose retained components. The host uses their existing layout/paint pipeline. Finite projection overflow is reported, never clamped.
pub fn render(
    canvas: &Canvas,
    camera: &Camera,
    viewport: Rect,
    theme: Theme,
) -> Result<c::Element, &'static str> {
    camera.validate()?;
    if [viewport.x, viewport.y, viewport.w, viewport.h]
        .iter()
        .any(|v| !v.is_finite())
        || viewport.w <= 0.
        || viewport.h <= 0.
    {
        return Err("invalid canvas viewport");
    }
    let mut out = Composition {
        viewport,
        overlays: vec![],
        labels: vec![],
        obstacles: vec![],
        zoom: camera.zoom as f32,
    };
    // Shapes and text are what connectors route around and what labels avoid.
    let mut world_obstacles: Vec<(&str, Bounds)> = vec![];
    for e in canvas.elements() {
        if matches!(e.content, Content::Shape { .. } | Content::Text { .. })
            && let Some(b) = e.content.bounds()
        {
            world_obstacles.push((&e.id, b));
            if let Some((rect, _)) = project_bounds(*camera, b, viewport)? {
                out.obstacles.push(rect);
            }
        }
    }
    // Stable id order within three explicit paint passes. Frames and closed
    // regions under edges under content; record insertion order never
    // becomes an implicit layer.
    for pass in 0..3 {
        for e in canvas.elements() {
            let layer = match e.content {
                Content::Frame { .. } => 0,
                Content::Path { closed: true, .. } | Content::Connector { .. } => 1,
                _ => 2,
            };
            if layer != pass {
                continue;
            }
            let look = Look::of(e, theme, out.zoom);
            match &e.content {
                Content::Connector {
                    from,
                    to,
                    label,
                    arrowheads,
                    dashed,
                    route,
                    from_side,
                    to_side,
                    waypoints,
                    label_at,
                } => {
                    let a = canvas
                        .get(from)
                        .and_then(|e| e.content.bounds())
                        .ok_or("missing canvas endpoint")?;
                    let b = canvas
                        .get(to)
                        .and_then(|e| e.content.bounds())
                        .ok_or("missing canvas endpoint")?;
                    let obstacles: Vec<Bounds> = world_obstacles
                        .iter()
                        .filter(|(id, _)| *id != from.as_str() && *id != to.as_str())
                        .map(|(_, b)| *b)
                        .collect();
                    let path =
                        route_connector(a, b, *route, *from_side, *to_side, waypoints, &obstacles);
                    if !out.stroke_world(*camera, &path, &look, *dashed, *arrowheads)? {
                        continue;
                    }
                    if let Some(label) = label.as_deref().filter(|l| !l.is_empty()) {
                        let (at, tangent) = path.sample(label_at.unwrap_or(0.5));
                        let at = project_point(*camera, viewport, at)?;
                        if inside(at, viewport) {
                            out.connector_label(at, tangent, label, &look);
                        }
                    }
                }
                Content::Path {
                    points,
                    closed,
                    smooth,
                    arrowheads,
                } => {
                    let path = Polyline {
                        points: points.clone(),
                        smooth: *smooth,
                        closed: *closed,
                        phantom: false,
                    };
                    out.stroke_world(*camera, &path, &look, false, *arrowheads)?;
                }
                _ => {
                    let bounds = e.content.bounds().ok_or("missing canvas bounds")?;
                    let Some((rect, detail)) = project_bounds(*camera, bounds, viewport)? else {
                        continue;
                    };
                    out.element(e, rect, detail, &look, theme);
                }
            }
        }
    }
    Ok(out.finish(theme))
}

/// A drawn connector or path in world space: straight segments between the
/// points, or one smooth curve through them. Sampling and arrow directions
/// use the flattened polyline; drawing keeps the cubics.
struct Polyline {
    points: Vec<Pt>,
    smooth: bool,
    closed: bool,
    /// The first and last points only set the end tangents; they are not drawn.
    phantom: bool,
}
impl Polyline {
    fn straight(points: Vec<Pt>) -> Self {
        Self {
            points,
            smooth: false,
            closed: false,
            phantom: false,
        }
    }
    /// The drawing's cubic segments: (start, control, control, end) tuples
    /// from a Catmull-Rom spline when smooth; straight lines otherwise.
    fn segments(&self) -> Vec<[Pt; 4]> {
        let p = &self.points;
        let n = p.len();
        if n < 2 || (self.phantom && n < 4) {
            return vec![];
        }
        let range = if self.closed {
            0..n
        } else if self.phantom {
            1..n - 2
        } else {
            0..n - 1
        };
        let at = |i: isize| -> Pt {
            if self.closed {
                p[i.rem_euclid(n as isize) as usize]
            } else {
                p[i.clamp(0, n as isize - 1) as usize]
            }
        };
        range
            .map(|i| {
                let i = i as isize;
                let [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
                if self.smooth {
                    let c1 = [p1[0] + (p2[0] - p0[0]) / 6., p1[1] + (p2[1] - p0[1]) / 6.];
                    let c2 = [p2[0] - (p3[0] - p1[0]) / 6., p2[1] - (p3[1] - p1[1]) / 6.];
                    [p1, c1, c2, p2]
                } else {
                    [p1, p1, p2, p2]
                }
            })
            .collect()
    }
    /// Flattened points for measurement: eight steps per smooth segment.
    fn flat(&self) -> Vec<Pt> {
        let segments = self.segments();
        let mut out = vec![];
        for (i, [a, c1, c2, b]) in segments.iter().enumerate() {
            if i == 0 {
                out.push(*a);
            }
            if self.smooth {
                for step in 1..=8 {
                    let t = step as f64 / 8.;
                    out.push(bezier(*a, *c1, *c2, *b, t));
                }
            } else {
                out.push(*b);
            }
        }
        out
    }
    /// The point at fraction `t` of the route's length and the unit tangent there.
    fn sample(&self, t: f64) -> (Pt, Pt) {
        let flat = self.flat();
        let lengths: Vec<f64> = flat.windows(2).map(|w| dist(w[0], w[1])).collect();
        let total: f64 = lengths.iter().sum();
        if flat.len() < 2 || total <= 0. {
            let p = flat.first().copied().unwrap_or([0.; 2]);
            return (p, [1., 0.]);
        }
        let mut remaining = total * t.clamp(0., 1.);
        for (i, len) in lengths.iter().enumerate() {
            if remaining <= *len || i == lengths.len() - 1 {
                let f = if *len > 0. {
                    (remaining / len).clamp(0., 1.)
                } else {
                    0.
                };
                let (a, b) = (flat[i], flat[i + 1]);
                let p = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                return (p, unit([b[0] - a[0], b[1] - a[1]]));
            }
            remaining -= len;
        }
        unreachable!()
    }
    /// Unit directions at the two ends, pointing outward from the route.
    fn end_directions(&self) -> Option<(Pt, Pt)> {
        let flat = self.flat();
        if flat.len() < 2 {
            return None;
        }
        let (a, b) = (flat[0], flat[1]);
        let (y, z) = (flat[flat.len() - 2], flat[flat.len() - 1]);
        Some((
            unit([a[0] - b[0], a[1] - b[1]]),
            unit([z[0] - y[0], z[1] - y[1]]),
        ))
    }
}
fn bezier(a: Pt, c1: Pt, c2: Pt, b: Pt, t: f64) -> Pt {
    let u = 1. - t;
    std::array::from_fn(|i| {
        u * u * u * a[i] + 3. * u * u * t * c1[i] + 3. * u * t * t * c2[i] + t * t * t * b[i]
    })
}
fn dist(a: Pt, b: Pt) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}
fn unit(d: Pt) -> Pt {
    let len = d[0].hypot(d[1]);
    if len > 0. {
        [d[0] / len, d[1] / len]
    } else {
        [1., 0.]
    }
}
fn normal(side: Side) -> Pt {
    match side {
        Side::Top => [0., -1.],
        Side::Right => [1., 0.],
        Side::Bottom => [0., 1.],
        Side::Left => [-1., 0.],
    }
}
/// The midpoint of a box's side.
fn port(b: Bounds, side: Side) -> Pt {
    let c = b.center();
    let n = normal(side);
    [c[0] + n[0] * b.size[0] / 2., c[1] + n[1] * b.size[1] / 2.]
}
/// The side of `b` facing `toward`, by the dominant axis of the offset
/// measured in box extents, so a wide box prefers its short sides later.
fn facing(b: Bounds, toward: Pt) -> Side {
    let c = b.center();
    let d = [toward[0] - c[0], toward[1] - c[1]];
    if d[0].abs() / b.size[0].max(1e-9) >= d[1].abs() / b.size[1].max(1e-9) {
        if d[0] >= 0. { Side::Right } else { Side::Left }
    } else if d[1] >= 0. {
        Side::Bottom
    } else {
        Side::Top
    }
}
/// Rectangular attachment keeps connectors following changed endpoint bounds.
fn attachment(b: Bounds, toward: Pt) -> Pt {
    let c = b.center();
    let d = [toward[0] - c[0], toward[1] - c[1]];
    let factor = (if d[0] == 0. {
        f64::INFINITY
    } else {
        b.size[0] / 2. / d[0].abs()
    })
    .min(if d[1] == 0. {
        f64::INFINITY
    } else {
        b.size[1] / 2. / d[1].abs()
    });
    if !factor.is_finite() {
        c
    } else {
        [c[0] + d[0] * factor, c[1] + d[1] * factor]
    }
}
/// Whether the segment crosses the box's interior (touching an edge from
/// outside does not count, so a port on the edge is not a crossing).
fn crosses(a: Pt, b: Pt, r: Bounds) -> bool {
    let (lo, hi) = (r.origin, [r.origin[0] + r.size[0], r.origin[1] + r.size[1]]);
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut t0, mut t1) = (0_f64, 1_f64);
    for i in 0..2 {
        if d[i] == 0. {
            if a[i] <= lo[i] || a[i] >= hi[i] {
                return false;
            }
        } else {
            let (mut ta, mut tb) = ((lo[i] - a[i]) / d[i], (hi[i] - a[i]) / d[i]);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
        }
    }
    t1 - t0 > 1e-9
}
fn crossings(points: &[Pt], obstacles: &[Bounds]) -> usize {
    points
        .windows(2)
        .map(|w| {
            obstacles
                .iter()
                .filter(|o| crosses(w[0], w[1], **o))
                .count()
        })
        .sum()
}
/// Straight: ports toward the next point, through the waypoints. Orthogonal:
/// axis-aligned elbows between side ports, taking the candidate that crosses
/// the fewest other shapes. Curved: one spline leaving and entering along the
/// port normals.
fn route_connector(
    a: Bounds,
    b: Bounds,
    route: Route,
    from_side: Option<Side>,
    to_side: Option<Side>,
    waypoints: &[Pt],
    obstacles: &[Bounds],
) -> Polyline {
    let first_target = waypoints.first().copied().unwrap_or(b.center());
    let last_target = waypoints.last().copied().unwrap_or(a.center());
    match route {
        Route::Straight => {
            let start = from_side.map_or_else(|| attachment(a, first_target), |s| port(a, s));
            let end = to_side.map_or_else(|| attachment(b, last_target), |s| port(b, s));
            let mut points = vec![start];
            points.extend_from_slice(waypoints);
            points.push(end);
            Polyline::straight(points)
        }
        Route::Curved => {
            let sa = from_side.unwrap_or_else(|| facing(a, first_target));
            let sb = to_side.unwrap_or_else(|| facing(b, last_target));
            let (start, end) = (port(a, sa), port(b, sb));
            let reach = (dist(start, end) * 0.4).max(16.);
            let (na, nb) = (normal(sa), normal(sb));
            // Phantom points behind each port make the spline leave and
            // enter along the normals; the knots past the ports hold the S.
            let mut points = vec![[start[0] - na[0] * reach, start[1] - na[1] * reach], start];
            if waypoints.is_empty() {
                points.push([start[0] + na[0] * reach, start[1] + na[1] * reach]);
                points.push([end[0] + nb[0] * reach, end[1] + nb[1] * reach]);
            } else {
                points.extend_from_slice(waypoints);
            }
            points.push(end);
            points.push([end[0] - nb[0] * reach, end[1] - nb[1] * reach]);
            Polyline {
                points,
                smooth: true,
                closed: false,
                phantom: true,
            }
        }
        Route::Orthogonal => {
            let sa = from_side.unwrap_or_else(|| orthogonal_side(a, b, first_target));
            let sb = to_side.unwrap_or_else(|| orthogonal_side(b, a, last_target));
            let (start, end) = (port(a, sa), port(b, sb));
            let stub = 24.;
            let (na, nb) = (normal(sa), normal(sb));
            let s1 = [start[0] + na[0] * stub, start[1] + na[1] * stub];
            let e1 = [end[0] + nb[0] * stub, end[1] + nb[1] * stub];
            let mut anchors = vec![s1];
            anchors.extend_from_slice(waypoints);
            anchors.push(e1);
            // Each gap between anchors has two elbows and, when the ports face
            // each other along one axis, a mid-channel route; the cheapest
            // combination is chosen gap by gap.
            let mut points = vec![start, s1];
            for w in anchors.windows(2) {
                let (p, q) = (w[0], w[1]);
                let horizontal_first = vec![[q[0], p[1]], q];
                let vertical_first = vec![[p[0], q[1]], q];
                let mut candidates = vec![horizontal_first, vertical_first];
                if na[0] != 0. && nb[0] != 0. {
                    let mid = (p[0] + q[0]) / 2.;
                    candidates.push(vec![[mid, p[1]], [mid, q[1]], q]);
                }
                if na[1] != 0. && nb[1] != 0. {
                    let mid = (p[1] + q[1]) / 2.;
                    candidates.push(vec![[p[0], mid], [q[0], mid], q]);
                }
                let best = candidates
                    .into_iter()
                    .min_by_key(|c| {
                        let mut trial = vec![p];
                        trial.extend_from_slice(c);
                        crossings(&trial, obstacles) * 8 + c.len()
                    })
                    .unwrap();
                points.extend(best);
            }
            points.push(end);
            points.dedup_by(|a, b| dist(*a, *b) < 1e-9);
            Polyline::straight(points)
        }
    }
}
/// A side whose outward stub does not head back into the other box.
fn orthogonal_side(from: Bounds, other: Bounds, toward: Pt) -> Side {
    let (lo, hi) = (
        from.origin,
        [from.origin[0] + from.size[0], from.origin[1] + from.size[1]],
    );
    let (olo, ohi) = (
        other.origin,
        [
            other.origin[0] + other.size[0],
            other.origin[1] + other.size[1],
        ],
    );
    if olo[0] >= hi[0] {
        Side::Right
    } else if ohi[0] <= lo[0] {
        Side::Left
    } else if olo[1] >= hi[1] {
        Side::Bottom
    } else if ohi[1] <= lo[1] {
        Side::Top
    } else {
        facing(from, toward)
    }
}

/// The resolved look of one element: the house style from kind, emphasis and
/// status, with the element's explicit style laid over it.
struct Look {
    ink: Color,
    fill: Color,
    text: Color,
    width: f32,
    glow: Option<Color>,
    shadow: bool,
    radius: Option<f32>,
    font_px: Option<f32>,
    weight: Option<u16>,
    italic: bool,
    mono: bool,
    align: c::Alignment,
}
impl Look {
    /// The kit text style for one run: the element's weight, slant and face over
    /// the step's size at this zoom.
    fn font(&self, step: TypeStep, size: f32, color: Color, emphasis: bool) -> ui::Text {
        ui::Text {
            step,
            size: Some(size),
            weight: self.weight.unwrap_or(if emphasis { 600 } else { 400 }),
            paint: paint(color),
            mono: self.mono,
            italic: self.italic,
            ..Default::default()
        }
    }
    fn of(e: &Element, theme: Theme, zoom: f32) -> Self {
        let p = theme.palette();
        let ink = Color::hex(match e.status {
            Status::Waiting => TOKENS.color.state.waiting,
            Status::Error => TOKENS.color.state.errored,
            Status::Done => p.accent,
            Status::Normal => {
                if e.emphasis || matches!(e.kind, Some(Kind::Service | Kind::Store)) {
                    p.accent
                } else {
                    p.fg_dim
                }
            }
        });
        let style = e.style.clone().unwrap_or_default();
        let named = |value: &Option<String>| value.as_deref().and_then(|v| color(v, theme));
        let ink = named(&style.stroke).unwrap_or(ink);
        let fill = named(&style.fill).unwrap_or_else(|| Color::hex(p.surface).mix(ink, 0.08));
        let text = named(&style.text).unwrap_or(Color::hex(p.fg));
        let opacity = style.opacity.unwrap_or(1.) as f32;
        let fade = |c: Color| c.alpha(c.0[3] * opacity);
        Self {
            ink: fade(ink),
            fill: fade(fill),
            text: fade(text),
            // Like the text, the ink scales with the drawing, within reason:
            // a zoomed-in box keeps a visible edge, a zoomed-out map stays crisp.
            width: style.stroke_width.map_or(
                STYLE.material.hairline_px as f32 * 1.2 * zoom.clamp(0.5, 3.),
                |w| (w as f32 * zoom).max(0.5),
            ),
            glow: named(&style.glow).map(fade),
            shadow: style.shadow,
            radius: style.radius.map(|r| r as f32 * zoom),
            font_px: style
                .font_size
                .map(|s| (s as f32 * zoom).round().clamp(MIN_TEXT_PX, MAX_TEXT_PX)),
            weight: style.weight,
            italic: style.italic,
            mono: style.mono,
            align: match style.align.unwrap_or_default() {
                Align::Left => c::Alignment::Start,
                Align::Center => c::Alignment::Center,
                Align::Right => c::Alignment::End,
            },
        }
    }
}
/// A style colour: a theme token by name, a fixed named hue, or a hex literal.
fn color(value: &str, theme: Theme) -> Option<Color> {
    let p = theme.palette();
    let hex = match value {
        "accent" => p.accent,
        "fg" => p.fg,
        "dim" => p.fg_dim,
        "surface" => p.surface,
        "bg" => p.bg,
        "waiting" => TOKENS.color.state.waiting,
        "error" => TOKENS.color.state.errored,
        "done" => p.accent,
        "red" => "#e5484d",
        "orange" => "#f76b15",
        "yellow" => "#f5d90a",
        "green" => "#30a46c",
        "teal" => "#12a594",
        "blue" => "#3e63dd",
        "indigo" => "#5b5bd6",
        "violet" => "#8e4ec6",
        "pink" => "#d6409f",
        "white" => "#ffffff",
        "black" => "#000000",
        hex if bboxy450_canvas::valid_color(hex) => hex,
        _ => return None,
    };
    Some(Color::hex(hex))
}
fn project_point(camera: Camera, viewport: Rect, point: Pt) -> Result<Pt, &'static str> {
    let p = camera.world_to_screen(point)?;
    Ok([
        p[0] + f64::from(viewport.x) + f64::from(viewport.w) / 2.,
        p[1] + f64::from(viewport.y) + f64::from(viewport.h) / 2.,
    ])
}
fn narrow(p: Pt) -> Result<[f32; 2], &'static str> {
    let p = p.map(|n| n as f32);
    if p.iter().all(|n| n.is_finite()) {
        Ok(p)
    } else {
        Err("canvas projection exceeds f32 range")
    }
}
fn project_bounds(
    camera: Camera,
    b: Bounds,
    v: Rect,
) -> Result<Option<(Rect, Detail)>, &'static str> {
    let p = project_point(camera, v, b.origin)?;
    let size = b.size.map(|n| n * camera.zoom);
    if size.iter().any(|n| !n.is_finite()) {
        return Err("non-finite canvas extent");
    }
    let detail = semantic_zoom(size[0].max(size[1]));
    if detail == Detail::Culled
        || p[0] > f64::from(v.right()) + 16.
        || p[1] > f64::from(v.bottom()) + 16.
        || p[0] + size[0] < f64::from(v.x) - 16.
        || p[1] + size[1] < f64::from(v.y) - 16.
    {
        return Ok(None);
    }
    let p = narrow(p)?;
    let size = narrow(size)?;
    Ok(Some((Rect::new(p[0], p[1], size[0], size[1]), detail)))
}
fn inside(p: Pt, v: Rect) -> bool {
    p[0] >= f64::from(v.x)
        && p[0] <= f64::from(v.right())
        && p[1] >= f64::from(v.y)
        && p[1] <= f64::from(v.bottom())
}
/// Liang–Barsky clipping in f64: long off-screen edges never reach the GPU.
fn clip_line(a: Pt, b: Pt, v: Rect) -> Option<(Pt, Pt)> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut lo, mut hi) = (0_f64, 1_f64);
    for (p, q) in [
        (-d[0], a[0] - f64::from(v.x) + 12.),
        (d[0], f64::from(v.right()) - a[0] + 12.),
        (-d[1], a[1] - f64::from(v.y) + 12.),
        (d[1], f64::from(v.bottom()) - a[1] + 12.),
    ] {
        if p == 0. {
            if q < 0. {
                return None;
            }
        } else if p < 0. {
            lo = lo.max(q / p);
        } else {
            hi = hi.min(q / p);
        }
    }
    (lo <= hi).then(|| {
        (
            [a[0] + d[0] * lo, a[1] + d[1] * lo],
            [a[0] + d[0] * hi, a[1] + d[1] * hi],
        )
    })
}
// Reuse the kit's positioned Layers, clipped Box, Text and Path components.
// No second painter or font measurer lives in this module.
struct Composition {
    viewport: Rect,
    overlays: Vec<c::Placed>,
    /// Connector label boxes already placed; a colliding label moves.
    labels: Vec<Rect>,
    /// Projected shape and text boxes; labels keep off them too.
    obstacles: Vec<Rect>,
    /// Camera zoom: text scales with the drawing, like every infinite
    /// canvas (tldraw, Excalidraw, Figma), instead of staying a fixed UI size
    /// while the boxes grow and shrink around it.
    zoom: f32,
}
/// Smallest legible glyph size; below it a label is left out rather than
/// rasterized as noise.
const MIN_TEXT_PX: f32 = 6.;
const MAX_TEXT_PX: f32 = 96.;
/// Estimated advance per character as a fraction of the glyph size, for
/// label boxes and wrapping; the real measurer lays the text out.
const CHAR_EM: f32 = 0.56;
impl Composition {
    /// The pixel size for a type step at this zoom, snapped to whole pixels so
    /// a zoom gesture passes through a bounded set of glyph rasters (a glyph
    /// atlas with size buckets) instead of a new raster every frame.
    fn text_px(&self, step: TypeStep) -> Option<f32> {
        let px = (step.pixels() * self.zoom).round();
        (px >= MIN_TEXT_PX).then(|| px.min(MAX_TEXT_PX))
    }
    fn size_px(&self, step: TypeStep, look: &Look) -> Option<f32> {
        look.font_px.or_else(|| self.text_px(step))
    }
    /// Frame titles are navigation labels: they grow with the zoom but never
    /// drop below their UI size, so a zoomed-out map still names its regions.
    fn floor_px(&self, step: TypeStep) -> f32 {
        (step.pixels() * self.zoom.max(1.)).round().min(MAX_TEXT_PX)
    }
    fn place(&mut self, content: c::Element, rect: Rect) {
        self.overlays.push(c::Placed {
            content,
            left: rect.x - self.viewport.x,
            bottom: self.viewport.bottom() - rect.bottom(),
        });
    }
    fn finish(self, theme: Theme) -> c::Element {
        let v = self.viewport;
        let mut base = surface(v, 0., Color::hex(theme.palette().bg), Color::TRANSPARENT);
        base.overrides.border = None;
        let layers = c::Element::Layers {
            base: Box::new(c::Element::Box(c::Box::new(base, vec![]))),
            overlays: self.overlays,
        };
        let mut root = c::Box::new(
            surface(v, 0., Color::TRANSPARENT, Color::TRANSPARENT),
            vec![layers.into()],
        );
        root.clip = true;
        c::Element::Box(root)
    }
    /// Projects, clips and strokes a world polyline with its arrowheads; a
    /// closed path is filled. Returns whether any of it reached the screen.
    fn stroke_world(
        &mut self,
        camera: Camera,
        path: &Polyline,
        look: &Look,
        dashed: bool,
        arrowheads: [bool; 2],
    ) -> Result<bool, &'static str> {
        let v = self.viewport;
        let fill = path.closed;
        let segments = path.segments();
        if segments.is_empty() {
            return Ok(false);
        }
        // Cull by the flattened extent; a path whose box is sub-pixel has no ink.
        let flat = path.flat();
        let extent = Bounds::union(flat.iter().map(|p| Bounds {
            origin: *p,
            size: [0.; 2],
        }))
        .ok_or("empty canvas path")?;
        let size = extent.size.map(|n| n * camera.zoom);
        if size.iter().any(|n| !n.is_finite()) {
            return Err("non-finite canvas extent");
        }
        if semantic_zoom(size[0].max(size[1])) == Detail::Culled {
            return Ok(false);
        }
        let mut commands = vec![];
        let mut any = false;
        if path.smooth || path.closed {
            // A curve or a closed figure is one path: the GPU clips it.
            let mut screen = Vec::with_capacity(segments.len());
            for seg in &segments {
                let mut s = [[0_f32; 2]; 4];
                for (i, p) in seg.iter().enumerate() {
                    s[i] = narrow(project_point(camera, v, *p)?)?;
                }
                screen.push(s);
            }
            let on_screen = screen.iter().any(|s| {
                let lo = [
                    s.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
                    s.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min),
                ];
                let hi = [
                    s.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max),
                    s.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max),
                ];
                lo[0] <= v.right() + 12.
                    && hi[0] >= v.x - 12.
                    && lo[1] <= v.bottom() + 12.
                    && hi[1] >= v.y - 12.
            });
            if !on_screen {
                return Ok(false);
            }
            commands.push(P::Move(screen[0][0]));
            for s in &screen {
                if path.smooth {
                    commands.push(P::Cubic([s[1], s[2], s[3]]));
                } else {
                    commands.push(P::Line(s[3]));
                }
            }
            if path.closed {
                commands.push(P::Line(screen[0][0]));
            }
            any = true;
        } else {
            // Straight runs are clipped segment by segment in f64.
            let mut pen: Option<[f32; 2]> = None;
            for seg in &segments {
                let (a, b) = (
                    project_point(camera, v, seg[0])?,
                    project_point(camera, v, seg[3])?,
                );
                let Some((p, q)) = clip_line(a, b, v) else {
                    pen = None;
                    continue;
                };
                let (p, q) = (narrow(p)?, narrow(q)?);
                if pen != Some(p) {
                    commands.push(P::Move(p));
                }
                commands.push(P::Line(q));
                pen = Some(q);
                any = true;
            }
        }
        if !any {
            return Ok(false);
        }
        if let Some(glow) = look.glow {
            stroke(
                self,
                commands.clone(),
                glow.alpha(glow.0[3] * 0.35),
                fill.then_some(glow.alpha(glow.0[3] * 0.12)),
                look.width + 6. * self.zoom.clamp(0.5, 2.),
                false,
            );
        }
        stroke(
            self,
            commands,
            look.ink,
            fill.then_some(look.fill),
            look.width,
            dashed,
        );
        if let Some((start_dir, end_dir)) = path.end_directions() {
            let ends = [
                (arrowheads[0], flat[0], start_dir),
                (arrowheads[1], flat[flat.len() - 1], end_dir),
            ];
            for (wanted, at, dir) in ends {
                if !wanted {
                    continue;
                }
                let tip = project_point(camera, v, at)?;
                // A clipped endpoint is not a real arrow tip.
                if inside(tip, v) {
                    arrow(self, narrow(tip)?, [dir[0] as f32, dir[1] as f32], look);
                }
            }
        }
        Ok(true)
    }
    /// A connector label sits beside its route, not on it: offset along the
    /// route's normal and nudged until it overlaps no earlier label or shape.
    fn connector_label(&mut self, at: Pt, tangent: Pt, label: &str, look: &Look) {
        let Some(px) = self.size_px(TypeStep::Caption, look) else {
            return;
        };
        let lh = px * 1.35;
        let lines: Vec<&str> = label.lines().collect();
        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let w = (longest as f32 * px * CHAR_EM + 10.).clamp(16., 320. * self.zoom.max(1.));
        let h = lh * lines.len().max(1) as f32;
        // The normal that points up (or right for a vertical route), so
        // labels read above horizontal lines and beside vertical ones.
        let mut n = [-tangent[1] as f32, tangent[0] as f32];
        if n[1] > 0. || (n[1] == 0. && n[0] < 0.) {
            n = [-n[0], -n[1]];
        }
        let gap = 4. + self.zoom.clamp(0.5, 2.) * 2.;
        let centre = |k: f32| {
            let d = (h / 2. + gap) * k;
            [at[0] as f32 + n[0] * d, at[1] as f32 + n[1] * d]
        };
        let rect_at = |c: [f32; 2]| Rect::new(c[0] - w / 2., c[1] - h / 2., w, h);
        let taken = |rect: Rect, labels: &[Rect], obstacles: &[Rect]| {
            labels.iter().chain(obstacles).any(|r| overlaps(*r, rect))
        };
        // First beside the line, then the far side, then further out.
        let mut rect = rect_at(centre(1.));
        for k in [1., -1., 2.2, -2.2, 3.4, -3.4] {
            rect = rect_at(centre(k));
            if !taken(rect, &self.labels, &self.obstacles) {
                break;
            }
        }
        self.labels.push(rect);
        label_lines(
            self,
            rect,
            label,
            look.font(TypeStep::Caption, px, look.ink, false),
            false,
        );
    }
    fn element(&mut self, e: &Element, rect: Rect, detail: Detail, look: &Look, theme: Theme) {
        let surface_color = Color::hex(theme.palette().surface);
        match &e.content {
            Content::Frame { name, .. } => {
                let title_px = look
                    .font_px
                    .unwrap_or_else(|| self.floor_px(TypeStep::Section));
                let title_h = title_px * 1.35;
                if detail == Detail::Full {
                    box_prim(
                        self,
                        rect,
                        look.radius.unwrap_or(STYLE.radius.card as f32),
                        if e.style.as_ref().is_some_and(|s| s.fill.is_some()) {
                            look.fill
                        } else {
                            surface_color.alpha(0.35)
                        },
                        look.ink.alpha(look.ink.0[3] * 0.45),
                        look,
                    );
                }
                text_sized(
                    self,
                    Rect::new(
                        rect.x + 8.,
                        rect.y + 6.,
                        if detail == Detail::Collapsed {
                            rect.w.max(120.)
                        } else {
                            rect.w - 16.
                        },
                        title_h,
                    ),
                    name,
                    look.font(TypeStep::Section, title_px, look.ink, true),
                );
            }
            Content::Shape { shape, label, .. } => {
                // Text legibility is the only gate: it shrinks with the zoom
                // and leaves below MIN_TEXT_PX, so nothing pops at a threshold.
                let labelled =
                    detail == Detail::Full && self.size_px(TypeStep::Body, look).is_some();
                let px = self.size_px(TypeStep::Body, look).unwrap_or(0.);
                let lh = px * 1.35;
                let (figure, text_rect) = if *shape == Shape::Actor {
                    // The figure and its label share the bounds: label below the figure.
                    let lines = label.lines().count().max(1) as f32;
                    let text_h = (lines * lh).min(rect.h * 0.5);
                    (
                        Rect::new(rect.x, rect.y, rect.w, (rect.h - text_h - 4.).max(1.)),
                        Rect::new(rect.x + 4., rect.bottom() - text_h, rect.w - 8., text_h),
                    )
                } else {
                    let inset = match shape {
                        Shape::Diamond | Shape::Star => rect.w * 0.22,
                        Shape::Triangle | Shape::Hexagon | Shape::Parallelogram => rect.w * 0.16,
                        Shape::Cloud => rect.w * 0.12,
                        _ => 8.,
                    };
                    (
                        rect,
                        Rect::new(
                            rect.x + inset,
                            rect.y + 4.,
                            (rect.w - inset * 2.).max(1.),
                            rect.h - 8.,
                        ),
                    )
                };
                shape_prims(self, figure, *shape, look);
                if labelled {
                    // Lines wrap to the box; an overlong label clips rather than overprints.
                    let wrapped = wrap(label, text_rect.w, px);
                    label_lines(
                        self,
                        text_rect,
                        &wrapped,
                        look.font(TypeStep::Body, px, look.text, e.emphasis),
                        true,
                    );
                }
            }
            Content::Text {
                text: value, role, ..
            } => {
                let step = match role {
                    TextRole::Heading => TypeStep::Title,
                    TextRole::Paragraph => TypeStep::Body,
                    TextRole::Note => TypeStep::Caption,
                };
                let ink = if e.style.as_ref().is_some_and(|s| s.text.is_some()) {
                    look.text
                } else {
                    look.ink
                };
                if detail == Detail::Collapsed || semantic_zoom(f64::from(rect.h)) != Detail::Full {
                    box_prim(
                        self,
                        Rect::new(rect.x, rect.y, rect.w, rect.h.min(3.)),
                        1.,
                        ink.alpha(0.5),
                        Color::TRANSPARENT,
                        look,
                    );
                } else {
                    let card = e.style.as_ref().is_some_and(|s| {
                        s.fill.is_some() || s.stroke.is_some() || s.glow.is_some()
                    });
                    if card {
                        // A styled text is a card: its fill, stroke and glow draw behind the words.
                        let stroked = e.style.as_ref().is_some_and(|s| s.stroke.is_some());
                        box_prim(
                            self,
                            rect,
                            look.radius.unwrap_or(STYLE.radius.card as f32),
                            look.fill,
                            if stroked {
                                look.ink
                            } else {
                                Color::TRANSPARENT
                            },
                            look,
                        );
                    }
                    let Some(px) = self.size_px(step, look) else {
                        return;
                    };
                    let font = look.font(step, px, ink, e.emphasis);
                    if *role == TextRole::Heading && !value.contains('\n') {
                        text_sized(self, rect, value, font);
                    } else {
                        text_block(
                            self,
                            Rect::new(
                                rect.x + 6.,
                                rect.y + 4.,
                                (rect.w - 12.).max(1.),
                                (rect.h - 8.).max(1.),
                            ),
                            value,
                            font,
                            look.align,
                        );
                    }
                }
            }
            Content::Connector { .. } | Content::Path { .. } => unreachable!(),
        }
    }
}
fn paint(color: Color) -> ui::Paint {
    ui::Paint::solid(ui::Color::Rgba(color.0))
}
fn surface(rect: Rect, radius: f32, fill: Color, border: Color) -> ui::Style {
    let mut style = super::strip::plain(
        Some(rect.h),
        [0.; 4],
        radius.min(rect.h / 2.).min(rect.w / 2.),
    );
    style.tokens = ui::Preset::Row;
    style.overrides.material = Some(ui::Material::Flat);
    style.overrides.width = Some(ui::Length::Px(rect.w));
    style.overrides.fill = Some(paint(fill));
    style.overrides.shadow = Some(ui::Shadow::None);
    style.overrides.border = Some(ui::Border {
        width: ui::Length::Px(if border.0[3] == 0. {
            0.
        } else {
            STYLE.material.hairline_px as f32
        }),
        paint: paint(border),
        outside: false,
    });
    style
}
fn box_prim(
    out: &mut Composition,
    rect: Rect,
    radius: f32,
    fill: Color,
    border: Color,
    look: &Look,
) {
    let mut style = surface(rect, radius, fill, border);
    if border.0[3] != 0. {
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(look.width),
            paint: paint(border),
            outside: false,
        });
    }
    if let Some(glow) = look.glow {
        style.overrides.shadow = Some(ui::Shadow::Custom {
            blur: 14. * out.zoom.clamp(0.5, 2.),
            color: glow.alpha(glow.0[3] * 0.8).0,
        });
    } else if look.shadow {
        style.overrides.shadow = Some(ui::Shadow::Floating);
    }
    let element = c::Element::Box(c::Box::new(style, vec![]));
    out.place(element, rect);
}
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}
/// Greedy word wrap by the character estimate; explicit newlines are kept.
fn wrap(value: &str, width: f32, px: f32) -> String {
    let per_line = ((width / (px * CHAR_EM)).floor() as usize).max(1);
    let mut out = String::new();
    for (i, line) in value.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut used = 0;
        for word in line.split(' ') {
            let len = word.chars().count();
            if used > 0 && used + 1 + len > per_line {
                out.push('\n');
                used = 0;
            } else if used > 0 {
                out.push(' ');
                used += 1;
            }
            out.push_str(word);
            used += len;
        }
    }
    out
}
/// Each `\n` line of `value` in `font`, one under the other, clipped to
/// `rect`; `center` stacks them around the rect's vertical middle.
fn label_lines(out: &mut Composition, rect: Rect, value: &str, font: ui::Text, center: bool) {
    let px = font.size.unwrap_or(0.);
    if rect.w <= 0. || rect.h <= 0. || px < MIN_TEXT_PX {
        return;
    }
    let lh = px * 1.35;
    let lines: Vec<&str> = value.lines().collect();
    let total = lh * lines.len() as f32;
    let top = if center {
        rect.y + ((rect.h - total) / 2.).max(0.)
    } else {
        rect.y
    };
    for (i, line) in lines.into_iter().enumerate() {
        let y = top + i as f32 * lh;
        if y + lh > rect.bottom() + 0.5 {
            break; // A label taller than its box clips; it never overprints.
        }
        text_sized(out, Rect::new(rect.x, y, rect.w, lh), line, font);
    }
}
/// Wrapped prose inside its bounds: notes and paragraphs.
fn text_block(out: &mut Composition, rect: Rect, value: &str, font: ui::Text, align: c::Alignment) {
    if rect.w <= 0. || rect.h <= 0. {
        return;
    }
    let mut label = c::Text::new(value, font);
    label.wrap = true;
    label.inline = false;
    label.align = align;
    let mut container = c::Box::new(
        surface(rect, 0., Color::TRANSPARENT, Color::TRANSPARENT),
        vec![c::Element::Text(label).into()],
    );
    container.clip = true;
    container.align = c::Alignment::Start;
    out.place(c::Element::Box(container), rect);
}
fn text_sized(out: &mut Composition, rect: Rect, value: &str, font: ui::Text) {
    if rect.w <= 0. || rect.h <= 0. {
        return;
    }
    let mut label = c::Text::new(value, font);
    label.wrap = false;
    label.inline = true;
    let mut container = c::Box::new(
        surface(rect, 0., Color::TRANSPARENT, Color::TRANSPARENT),
        vec![c::Element::Text(label).into()],
    );
    container.clip = true;
    container.align = c::Alignment::Start;
    out.place(c::Element::Box(container), rect);
}
fn stroke(
    out: &mut Composition,
    points: Vec<P>,
    color: Color,
    fill: Option<Color>,
    width: f32,
    dashed: bool,
) {
    // The path lives in its own box: its mask is keyed by box-relative points,
    // so a pan or zoom that keeps the geometry reuses the rasterized mask
    // instead of redrawing every stroke against viewport coordinates.
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    let mut seen = |p: [f32; 2]| {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    };
    for p in &points {
        match p {
            P::Move(p) | P::Line(p) => seen(*p),
            P::Cubic(c) => c.iter().for_each(|p| seen(*p)),
        }
    }
    if lo.iter().chain(hi.iter()).any(|v| !v.is_finite()) {
        return;
    }
    let pad = 6. + width;
    let local = Rect::new(
        lo[0] - pad,
        lo[1] - pad,
        hi[0] - lo[0] + pad * 2.,
        hi[1] - lo[1] + pad * 2.,
    );
    let relative = |p: [f32; 2]| [p[0] - local.x, p[1] - local.y];
    let points = points
        .into_iter()
        .map(|p| match p {
            P::Move(p) => P::Move(relative(p)),
            P::Line(p) => P::Line(relative(p)),
            P::Cubic(p) => P::Cubic(p.map(relative)),
        })
        .collect::<Vec<_>>();
    out.place(
        c::Element::Path(c::Path {
            size: [local.w, local.h],
            points: points.into(),
            stroke: PathStroke {
                width,
                dash: if dashed {
                    vec![4., 4.].into()
                } else {
                    vec![].into()
                },
                cap: LineCap::Round,
                join: LineJoin::Round,
            },
            paint: paint(color),
            fill: fill.map(paint),
            opacity: 1.,
        }),
        local,
    );
}
fn arrow(out: &mut Composition, tip: [f32; 2], direction: [f32; 2], look: &Look) {
    let weight = (look.width / (STYLE.material.hairline_px as f32 * 1.2)).clamp(1., 4.);
    stroke(
        out,
        super::arrow_points(tip, direction, 1.2 * weight),
        look.ink,
        None,
        look.width,
        false,
    );
}
fn ellipse(out: &mut Vec<P>, r: Rect) {
    let [x, y, w, h] = [r.x, r.y, r.w / 2., r.h / 2.];
    let [cx, cy] = [x + w, y + h];
    let k = 0.552_284_8;
    out.extend([
        P::Move([cx + w, cy]),
        P::Cubic([[cx + w, cy + h * k], [cx + w * k, cy + h], [cx, cy + h]]),
        P::Cubic([[cx - w * k, cy + h], [cx - w, cy + h * k], [cx - w, cy]]),
        P::Cubic([[cx - w, cy - h * k], [cx - w * k, cy - h], [cx, cy - h]]),
        P::Cubic([[cx + w * k, cy - h], [cx + w, cy - h * k], [cx + w, cy]]),
    ]);
}
fn polygon(out: &mut Vec<P>, points: &[[f32; 2]]) {
    out.push(P::Move(points[0]));
    out.extend(points[1..].iter().map(|p| P::Line(*p)));
    out.push(P::Line(points[0]));
}
fn shape_prims(out: &mut Composition, r: Rect, shape: Shape, look: &Look) {
    let [x, y, w, h] = [r.x, r.y, r.w, r.h];
    let mut p = vec![];
    match shape {
        Shape::Box | Shape::Rounded => {
            box_prim(
                out,
                r,
                look.radius.unwrap_or(if shape == Shape::Box {
                    0.
                } else {
                    STYLE.radius.card as f32
                }),
                look.fill,
                look.ink,
                look,
            );
            return;
        }
        Shape::Ellipse => ellipse(&mut p, r),
        Shape::Diamond => polygon(
            &mut p,
            &[
                [x + w / 2., y],
                [x + w, y + h / 2.],
                [x + w / 2., y + h],
                [x, y + h / 2.],
            ],
        ),
        Shape::Hexagon => {
            let k = (w * 0.18).min(h / 2.);
            polygon(
                &mut p,
                &[
                    [x + k, y],
                    [x + w - k, y],
                    [x + w, y + h / 2.],
                    [x + w - k, y + h],
                    [x + k, y + h],
                    [x, y + h / 2.],
                ],
            );
        }
        Shape::Triangle => polygon(&mut p, &[[x + w / 2., y], [x + w, y + h], [x, y + h]]),
        Shape::Parallelogram => {
            let k = (w * 0.2).min(h);
            polygon(
                &mut p,
                &[[x + k, y], [x + w, y], [x + w - k, y + h], [x, y + h]],
            );
        }
        Shape::Star => {
            let (cx, cy) = (x + w / 2., y + h / 2.);
            let points: Vec<[f32; 2]> = (0..10)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.;
                    let k = if i % 2 == 0 { 1. } else { 0.45 };
                    [cx + angle.cos() * w / 2. * k, cy + angle.sin() * h / 2. * k]
                })
                .collect();
            polygon(&mut p, &points);
        }
        Shape::Cloud => {
            // A scalloped ellipse: each arc bulges outward between its ends.
            let (cx, cy, rx, ry) = (x + w / 2., y + h / 2., w / 2. * 0.86, h / 2. * 0.8);
            let bumps = 7;
            let at = |i: usize| {
                let a = i as f32 / bumps as f32 * std::f32::consts::TAU;
                [cx + a.cos() * rx, cy + a.sin() * ry]
            };
            p.push(P::Move(at(0)));
            for i in 0..bumps {
                let (a, b) = (at(i), at(i + 1));
                let mid = (i as f32 + 0.5) / bumps as f32 * std::f32::consts::TAU;
                let bulge = [cx + mid.cos() * rx * 1.42, cy + mid.sin() * ry * 1.5];
                let c1 = [a[0] * 0.3 + bulge[0] * 0.7, a[1] * 0.3 + bulge[1] * 0.7];
                let c2 = [b[0] * 0.3 + bulge[0] * 0.7, b[1] * 0.3 + bulge[1] * 0.7];
                p.push(P::Cubic([c1, c2, b]));
            }
        }
        Shape::Cylinder => {
            let cap = h * 0.22;
            p.extend([
                P::Move([x, y + cap / 2.]),
                P::Line([x, y + h - cap / 2.]),
                P::Cubic([
                    [x, y + h + cap / 6.],
                    [x + w, y + h + cap / 6.],
                    [x + w, y + h - cap / 2.],
                ]),
                P::Line([x + w, y + cap / 2.]),
                P::Cubic([[x + w, y - cap / 6.], [x, y - cap / 6.], [x, y + cap / 2.]]),
            ]);
            figure(out, p, look, true);
            let mut cap_path = vec![];
            ellipse(&mut cap_path, Rect::new(x, y, w, cap));
            stroke(out, cap_path, look.ink, None, look.width, false);
            return;
        }
        Shape::Actor => {
            // A stick figure keeps its proportions in a square inside the bounds.
            let side = w.min(h).max(1.);
            let [x, y, w, h] = [x + (w - side) / 2., y + (h - side) / 2., side, side];
            ellipse(&mut p, Rect::new(x + w * 0.4, y, w * 0.2, h * 0.25));
            p.extend([
                P::Move([x + w / 2., y + h * 0.25]),
                P::Line([x + w / 2., y + h * 0.65]),
                P::Move([x + w * 0.2, y + h * 0.4]),
                P::Line([x + w * 0.8, y + h * 0.4]),
                P::Move([x + w * 0.25, y + h]),
                P::Line([x + w / 2., y + h * 0.65]),
                P::Line([x + w * 0.75, y + h]),
            ]);
            figure(out, p, look, false);
            return;
        }
    }
    figure(out, p, look, true);
}
/// A vector figure with its optional glow halo under the main stroke.
fn figure(out: &mut Composition, p: Vec<P>, look: &Look, filled: bool) {
    if let Some(glow) = look.glow {
        stroke(
            out,
            p.clone(),
            glow.alpha(glow.0[3] * 0.35),
            filled.then_some(glow.alpha(glow.0[3] * 0.12)),
            look.width + 6. * out.zoom.clamp(0.5, 2.),
            false,
        );
    }
    stroke(
        out,
        p,
        look.ink,
        filled.then_some(look.fill),
        look.width,
        false,
    );
}
