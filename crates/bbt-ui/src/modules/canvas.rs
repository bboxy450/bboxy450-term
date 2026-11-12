// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Static semantic canvas projection. World math and culling precede f32 output.
use crate::{self as ui, components as c};
use bboxy450_canvas::{
    Bounds, Camera, Canvas, Content, Detail, Element, Kind, Shape, Status, TextRole, semantic_zoom,
};
use bbt_ui_model::{STYLE, TOKENS, Theme, draw::Color, layout::Rect};
use c::{LineCap, LineJoin, PathCommand as P, PathStroke};
use ui::TypeStep;

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
        zoom: camera.zoom as f32,
    };
    // Stable id order within three explicit paint passes. Frames under edges
    // under content; record insertion order never becomes an implicit layer.
    for pass in 0..3 {
        for e in canvas.elements() {
            let layer = match e.content {
                Content::Frame { .. } => 0,
                Content::Connector { .. } => 1,
                _ => 2,
            };
            if layer != pass {
                continue;
            }
            if let Content::Connector {
                from,
                to,
                label,
                arrowheads,
                dashed,
            } = &e.content
            {
                let a = canvas
                    .get(from)
                    .and_then(|e| e.content.bounds())
                    .ok_or("missing canvas endpoint")?;
                let b = canvas
                    .get(to)
                    .and_then(|e| e.content.bounds())
                    .ok_or("missing canvas endpoint")?;
                let start = project_point(*camera, viewport, attachment(a, b.center()))?;
                let end = project_point(*camera, viewport, attachment(b, a.center()))?;
                let Some((p, q)) = clip_line(start, end, viewport) else {
                    continue;
                };
                let p = narrow(p)?;
                let q = narrow(q)?;
                if semantic_zoom(f64::from((q[0] - p[0]).hypot(q[1] - p[1]))) == Detail::Culled {
                    continue;
                }
                let color = ink(e, theme);
                stroke(&mut out, vec![P::Move(p), P::Line(q)], color, None, *dashed);
                let d = [end[0] - start[0], end[1] - start[1]];
                let length = d[0].hypot(d[1]);
                if length > 0. {
                    let direction = [(d[0] / length) as f32, (d[1] / length) as f32];
                    // A clipped endpoint is not a real arrow tip.
                    if arrowheads[0] && inside(start, viewport) {
                        arrow(&mut out, p, direction.map(|v| -v), color);
                    }
                    if arrowheads[1] && inside(end, viewport) {
                        arrow(&mut out, q, direction, color);
                    }
                }
                if let Some(label) = label {
                    let mid = [(start[0] + end[0]) / 2., (start[1] + end[1]) / 2.];
                    if inside(mid, viewport) {
                        let p = narrow(mid)?;
                        let Some(px) = out.text_px(TypeStep::Caption) else {
                            continue;
                        };
                        let lh = out.line_height(TypeStep::Caption);
                        let lines: Vec<&str> = label.lines().collect();
                        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
                        let w = (longest as f32 * px * 0.58 + 12.).clamp(24., 320. * out.zoom.max(1.));
                        let h = lh * lines.len().max(1) as f32;
                        let mut rect = Rect::new(p[0] - w / 2., p[1] - h - 2., w, h);
                        // Two labels in the same gap stack instead of overprinting.
                        for _ in 0..4 {
                            if out.labels.iter().any(|placed| overlaps(*placed, rect)) {
                                rect.y += lh;
                            } else {
                                break;
                            }
                        }
                        out.labels.push(rect);
                        label_lines(&mut out, rect, label, TypeStep::Caption, color, false, false);
                    }
                }
                continue;
            }
            let bounds = e.content.bounds().ok_or("missing canvas bounds")?;
            let Some((rect, detail)) = project_bounds(*camera, bounds, viewport)? else {
                continue;
            };
            let color = ink(e, theme);
            let surface = Color::hex(theme.palette().surface);
            match &e.content {
                Content::Frame { name, .. } => {
                    let title_px = out.floor_px(TypeStep::Section);
                    let title_h = title_px * 1.35;
                    if detail == Detail::Full {
                        box_prim(
                            &mut out,
                            rect,
                            STYLE.radius.card as f32,
                            surface.alpha(0.35),
                            color.alpha(0.45),
                        );
                    }
                    text_sized(
                        &mut out,
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
                        TypeStep::Section,
                        color,
                        true,
                        Some(title_px),
                    );
                }
                Content::Shape { shape, label, .. } => {
                    // Text legibility is the only gate: it shrinks with the zoom
                    // and leaves below MIN_TEXT_PX, so nothing pops at a threshold.
                    let labelled = detail == Detail::Full && out.text_px(TypeStep::Body).is_some();
                    let lines = label.lines().count().max(1) as f32;
                    let lh = out.line_height(TypeStep::Body);
                    let (figure, text_rect) = if *shape == Shape::Actor {
                        // The figure and its label share the bounds: label below the figure.
                        let text_h = (lines * lh).min(rect.h * 0.5);
                        (
                            Rect::new(rect.x, rect.y, rect.w, (rect.h - text_h - 4.).max(1.)),
                            Rect::new(rect.x + 4., rect.bottom() - text_h, rect.w - 8., text_h),
                        )
                    } else {
                        (
                            rect,
                            Rect::new(rect.x + 8., rect.y + 4., rect.w - 16., rect.h - 8.),
                        )
                    };
                    shape_prims(&mut out, figure, *shape, color, surface.mix(color, 0.08));
                    if labelled {
                        label_lines(
                            &mut out,
                            text_rect,
                            label,
                            TypeStep::Body,
                            Color::hex(theme.palette().fg),
                            e.emphasis,
                            true,
                        );
                    }
                }
                Content::Text {
                    text: value, role, ..
                } => {
                    let scale = match role {
                        TextRole::Heading => TypeStep::Title,
                        TextRole::Paragraph => TypeStep::Body,
                        TextRole::Note => TypeStep::Caption,
                    };
                    if detail == Detail::Collapsed
                        || semantic_zoom(f64::from(rect.h)) != Detail::Full
                    {
                        box_prim(
                            &mut out,
                            Rect::new(rect.x, rect.y, rect.w, rect.h.min(3.)),
                            1.,
                            color.alpha(0.5),
                            Color::TRANSPARENT,
                        );
                    } else if *role == TextRole::Heading {
                        text(&mut out, rect, value, scale, color, true);
                    } else {
                        text_block(&mut out, rect, value, scale, color, e.emphasis);
                    }
                }
                Content::Connector { .. } => unreachable!(),
            }
        }
    }
    Ok(out.finish(theme))
}
fn ink(e: &Element, theme: Theme) -> Color {
    let p = theme.palette();
    let token = match e.status {
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
    };
    Color::hex(token)
}
fn project_point(
    camera: Camera,
    viewport: Rect,
    point: [f64; 2],
) -> Result<[f64; 2], &'static str> {
    let p = camera.world_to_screen(point)?;
    Ok([
        p[0] + f64::from(viewport.x) + f64::from(viewport.w) / 2.,
        p[1] + f64::from(viewport.y) + f64::from(viewport.h) / 2.,
    ])
}
fn narrow(p: [f64; 2]) -> Result<[f32; 2], &'static str> {
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
fn inside(p: [f64; 2], v: Rect) -> bool {
    p[0] >= f64::from(v.x)
        && p[0] <= f64::from(v.right())
        && p[1] >= f64::from(v.y)
        && p[1] <= f64::from(v.bottom())
}
/// Rectangular attachment keeps connectors following changed endpoint bounds.
fn attachment(b: Bounds, toward: [f64; 2]) -> [f64; 2] {
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
/// Liang–Barsky clipping in f64: long off-screen edges never reach the GPU.
fn clip_line(a: [f64; 2], b: [f64; 2], v: Rect) -> Option<([f64; 2], [f64; 2])> {
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
    /// Connector label boxes already placed; a colliding label moves down.
    labels: Vec<Rect>,
    /// Camera zoom: text scales with the drawing, like every infinite
    /// canvas (tldraw, Excalidraw, Figma), instead of staying a fixed UI size
    /// while the boxes grow and shrink around it.
    zoom: f32,
}
/// Smallest legible glyph size; below it a label is left out rather than
/// rasterized as noise.
const MIN_TEXT_PX: f32 = 6.;
const MAX_TEXT_PX: f32 = 96.;
impl Composition {
    /// The pixel size for a type step at this zoom, snapped to whole pixels so
    /// a zoom gesture passes through a bounded set of glyph rasters (a glyph
    /// atlas with size buckets) instead of a new raster every frame.
    fn text_px(&self, step: TypeStep) -> Option<f32> {
        let px = (step.pixels() * self.zoom).round();
        (px >= MIN_TEXT_PX).then(|| px.min(MAX_TEXT_PX))
    }
    fn line_height(&self, step: TypeStep) -> f32 {
        self.text_px(step).unwrap_or(0.) * 1.35
    }
    /// Frame titles are navigation labels: they grow with the zoom but never
    /// drop below their UI size, so a zoomed-out map still names its regions.
    fn floor_px(&self, step: TypeStep) -> f32 {
        (step.pixels() * self.zoom.max(1.)).round().min(MAX_TEXT_PX)
    }
}
impl Composition {
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
fn box_prim(out: &mut Composition, rect: Rect, radius: f32, fill: Color, border: Color) {
    let element = c::Element::Box(c::Box::new(surface(rect, radius, fill, border), vec![]));
    out.place(element, rect);
}
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}
/// Each `\n` line of `value` at the step's size, one under the other, clipped
/// to `rect`; `center` stacks them around the rect's vertical middle.
fn label_lines(
    out: &mut Composition,
    rect: Rect,
    value: &str,
    step: TypeStep,
    color: Color,
    emphasis: bool,
    center: bool,
) {
    if rect.w <= 0. || rect.h <= 0. || out.text_px(step).is_none() {
        return;
    }
    let lh = out.line_height(step);
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
        text(out, Rect::new(rect.x, y, rect.w, lh), line, step, color, emphasis);
    }
}
/// Wrapped prose inside its bounds: notes and paragraphs.
fn text_block(
    out: &mut Composition,
    rect: Rect,
    value: &str,
    step: TypeStep,
    color: Color,
    emphasis: bool,
) {
    let Some(size) = out.text_px(step) else {
        return;
    };
    if rect.w <= 0. || rect.h <= 0. {
        return;
    }
    let mut label = c::Text::new(
        value,
        ui::Text {
            step,
            size: Some(size),
            weight: if emphasis { 600 } else { 400 },
            paint: paint(color),
            ..Default::default()
        },
    );
    label.wrap = true;
    label.inline = false;
    let mut container = c::Box::new(
        surface(rect, 0., Color::TRANSPARENT, Color::TRANSPARENT),
        vec![c::Element::Text(label).into()],
    );
    container.clip = true;
    container.align = c::Alignment::Start;
    out.place(c::Element::Box(container), rect);
}
fn text(
    out: &mut Composition,
    rect: Rect,
    value: &str,
    step: TypeStep,
    color: Color,
    emphasis: bool,
) {
    text_sized(out, rect, value, step, color, emphasis, None);
}
fn text_sized(
    out: &mut Composition,
    rect: Rect,
    value: &str,
    step: TypeStep,
    color: Color,
    emphasis: bool,
    size: Option<f32>,
) {
    let Some(size) = size.or_else(|| out.text_px(step)) else {
        return;
    };
    if rect.w <= 0. || rect.h <= 0. {
        return;
    }
    let mut label = c::Text::new(
        value,
        ui::Text {
            step,
            size: Some(size),
            weight: if emphasis { 600 } else { 400 },
            paint: paint(color),
            ..Default::default()
        },
    );
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
fn stroke(out: &mut Composition, points: Vec<P>, color: Color, fill: Option<Color>, dashed: bool) {
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
    let pad = 6.;
    let local = Rect::new(lo[0] - pad, lo[1] - pad, hi[0] - lo[0] + pad * 2., hi[1] - lo[1] + pad * 2.);
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
                width: STYLE.material.hairline_px as f32 * 1.2,
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
fn arrow(out: &mut Composition, tip: [f32; 2], direction: [f32; 2], color: Color) {
    stroke(
        out,
        super::arrow_points(tip, direction, 1.2),
        color,
        None,
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
fn shape_prims(out: &mut Composition, r: Rect, shape: Shape, color: Color, fill: Color) {
    let [x, y, w, h] = [r.x, r.y, r.w, r.h];
    let mut p = vec![];
    match shape {
        Shape::Box | Shape::Rounded => {
            box_prim(
                out,
                r,
                if shape == Shape::Box {
                    0.
                } else {
                    STYLE.radius.card as f32
                },
                fill,
                color,
            );
            return;
        }
        Shape::Ellipse => ellipse(&mut p, r),
        Shape::Diamond => p.extend([
            P::Move([x + w / 2., y]),
            P::Line([x + w, y + h / 2.]),
            P::Line([x + w / 2., y + h]),
            P::Line([x, y + h / 2.]),
            P::Line([x + w / 2., y]),
        ]),
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
            stroke(out, p, color, Some(fill), false);
            let mut cap_path = vec![];
            ellipse(&mut cap_path, Rect::new(x, y, w, cap));
            stroke(out, cap_path, color, None, false);
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
            stroke(out, p, color, None, false);
            return;
        }
    }
    stroke(out, p, color, Some(fill), false);
}
