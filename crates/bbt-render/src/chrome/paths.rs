//! Cached CPU stroke tessellation and analytic GPU edge coverage.
//! Paint is one source-over operation per path, including translucent joins.
use super::ChromeTarget;
use crate::Result;
use bbt_ui_model::draw::{LineCap, LineJoin, PathCommand, PathStroke, Prim};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
    path::{Path, geom::CubicBezierSegment, math::point},
};
use std::collections::{BTreeMap, HashMap};
use wgpu::util::DeviceExt;

const TOLERANCE: f32 = 0.002;
#[derive(Default)]
pub(super) struct Renderer {
    geometry: HashMap<Vec<u32>, Geometry>,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}
struct Geometry {
    edges: wgpu::Buffer,
    count: u32,
    bounds: [f32; 4],
}
fn key(points: &[PathCommand], s: &PathStroke) -> Vec<u32> {
    let mut out = vec![
        s.width.to_bits(),
        s.cap as u32,
        s.join as u32,
        s.dash.len() as u32,
    ];
    out.extend(s.dash.iter().map(|n| n.to_bits()));
    for p in points {
        match p {
            PathCommand::Move(p) => {
                out.push(0);
                out.extend(p.map(f32::to_bits));
            }
            PathCommand::Line(p) => {
                out.push(1);
                out.extend(p.map(f32::to_bits));
            }
            PathCommand::Cubic(p) => {
                out.push(2);
                out.extend(p.iter().flatten().map(|n| n.to_bits()));
            }
        }
    }
    out
}
fn cap(c: LineCap) -> lyon_tessellation::LineCap {
    match c {
        LineCap::Butt => lyon_tessellation::LineCap::Butt,
        LineCap::Round => lyon_tessellation::LineCap::Round,
        LineCap::Square => lyon_tessellation::LineCap::Square,
    }
}
fn join(j: LineJoin) -> lyon_tessellation::LineJoin {
    match j {
        LineJoin::Miter => lyon_tessellation::LineJoin::Miter,
        LineJoin::Round => lyon_tessellation::LineJoin::Round,
        LineJoin::Bevel => lyon_tessellation::LineJoin::Bevel,
    }
}
fn source(points: &[PathCommand]) -> Path {
    let mut b = Path::builder();
    let mut open = false;
    for p in points {
        match *p {
            PathCommand::Move(p) => {
                if open {
                    b.end(false);
                }
                b.begin(point(p[0], p[1]));
                open = true;
            }
            PathCommand::Line(p) => {
                b.line_to(point(p[0], p[1]));
            }
            PathCommand::Cubic(p) => {
                b.cubic_bezier_to(
                    point(p[0][0], p[0][1]),
                    point(p[1][0], p[1][1]),
                    point(p[2][0], p[2][1]),
                );
            }
        }
    }
    if open {
        b.end(false);
    }
    b.build()
}
/// SVG dash lengths run continuously across flattened cubic segments and reset
/// at Move. A zero on-length with round caps is a dot, not an infinite loop.
fn dashed(points: &[PathCommand], dash: &[f32]) -> Result<(Path, Vec<[f32; 2]>)> {
    let mut contours: Vec<Vec<[f32; 2]>> = vec![];
    let mut at = [0.; 2];
    for cmd in points {
        match *cmd {
            PathCommand::Move(p) => {
                contours.push(vec![p]);
                at = p;
            }
            PathCommand::Line(p) => {
                contours.last_mut().unwrap().push(p);
                at = p;
            }
            PathCommand::Cubic(p) => {
                let curve = CubicBezierSegment {
                    from: point(at[0], at[1]),
                    ctrl1: point(p[0][0], p[0][1]),
                    ctrl2: point(p[1][0], p[1][1]),
                    to: point(p[2][0], p[2][1]),
                };
                contours
                    .last_mut()
                    .unwrap()
                    .extend(curve.flattened(TOLERANCE).map(|p| [p.x, p.y]));
                at = p[2];
            }
        }
    }
    let mut pattern = dash.to_vec();
    if pattern.len() % 2 == 1 {
        pattern.extend_from_slice(dash);
    }
    if pattern.iter().all(|part| *part <= 0.000001) {
        return Err("dash pattern is below path precision".into());
    }
    let mut b = Path::builder();
    let mut dots = vec![];
    for contour in contours {
        let mut i = 0;
        let mut remain = f64::from(pattern[0]);
        let mut open = false;
        // Preserve the established f32 vertices for already-renderable paths.
        // A sub-ULP remainder switches this contour to an exact f64 cursor;
        // it must not append the same vertex forever.
        let mut precise = false;
        for pair in contour.windows(2) {
            let a = pair[0];
            let z = pair[1];
            let len = (z[0] - a[0]).hypot(z[1] - a[1]);
            if len <= f32::EPSILON {
                continue;
            }
            let pos = |t: f64| {
                let t = t as f32;
                [
                    a[0] + (z[0] - a[0]) * t / len,
                    a[1] + (z[1] - a[1]) * t / len,
                ]
            };
            let mut t = 0_f64;
            while t < f64::from(len - 0.000001) {
                let mut skipped = 0;
                while remain <= 0.000001 {
                    if i % 2 == 0 && pattern[i] == 0. {
                        dots.push(pos(t));
                    }
                    if open {
                        b.end(false);
                        open = false;
                    }
                    i = (i + 1) % pattern.len();
                    remain = f64::from(pattern[i]);
                    skipped += 1;
                    if skipped > pattern.len() {
                        return Err("dash pattern does not advance".into());
                    }
                }
                let rounded = ((t as f32 + remain as f32).min(len)) as f64;
                precise |= rounded <= t;
                let end = if precise {
                    (t + remain).min(f64::from(len))
                } else {
                    rounded
                };
                if end <= t {
                    return Err("dash cursor does not advance".into());
                }
                let p = pos(t);
                let q = pos(end);
                if i % 2 == 0 {
                    if !open {
                        b.begin(point(p[0], p[1]));
                        open = true;
                    }
                    if !precise || p != q {
                        b.line_to(point(q[0], q[1]));
                    }
                }
                remain = if precise {
                    remain - (end - t)
                } else {
                    f64::from(remain as f32 - (end as f32 - t as f32).max(0.))
                };
                t = end;
            }
        }
        if open {
            b.end(false);
        }
    }
    Ok((b.build(), dots))
}
fn tessellate(points: &[PathCommand], stroke: &PathStroke) -> Result<Vec<[f32; 4]>> {
    stroke.validate(points)?;
    if stroke.width == 0. || points.is_empty() {
        return Ok(vec![]);
    }
    let (path, dots) = if stroke.dash.iter().sum::<f32>() > 0. {
        dashed(points, &stroke.dash)?
    } else {
        (source(points), vec![])
    };
    let mut mesh: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let options = StrokeOptions::default()
        .with_line_width(stroke.width)
        .with_tolerance(TOLERANCE)
        .with_line_cap(cap(stroke.cap))
        .with_line_join(join(stroke.join))
        .with_miter_limit(4.);
    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| {
                let p = v.position();
                [p.x, p.y]
            }),
        )
        .map_err(|e| format!("path stroke tessellation: {e:?}"))?;
    // Nonzero union removes internal tessellation/overlap edges. Otherwise a
    // translucent self-crossing would blend twice or gain an AA seam.
    let mut union = Path::builder();
    for tri in mesh.indices.chunks_exact(3) {
        let [a, mut b, mut c] = [
            mesh.vertices[tri[0] as usize],
            mesh.vertices[tri[1] as usize],
            mesh.vertices[tri[2] as usize],
        ];
        let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if cross.abs() < 1e-10 {
            continue;
        }
        if cross < 0. {
            std::mem::swap(&mut b, &mut c);
        }
        union.begin(point(a[0], a[1]));
        union.line_to(point(b[0], b[1]));
        union.line_to(point(c[0], c[1]));
        union.end(true);
    }
    if stroke.cap == LineCap::Round {
        for p in dots {
            let radius = stroke.width * 0.5;
            let n =
                ((std::f32::consts::PI / (1. - TOLERANCE / radius).clamp(-1., 1.).acos()).ceil()
                    as usize)
                    .clamp(12, 1024);
            for j in 0..n {
                let a = std::f32::consts::TAU * j as f32 / n as f32;
                let q = point(p[0] + radius * a.cos(), p[1] + radius * a.sin());
                if j == 0 {
                    union.begin(q);
                } else {
                    union.line_to(q);
                }
            }
            union.end(true);
        }
    }
    let mut result: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            &union.build(),
            &FillOptions::non_zero().with_tolerance(TOLERANCE),
            &mut BuffersBuilder::new(&mut result, |v: FillVertex| {
                let p = v.position();
                [p.x, p.y]
            }),
        )
        .map_err(|e| format!("path stroke union: {e:?}"))?;
    let mut edges = BTreeMap::new();
    for tri in result.indices.chunks_exact(3) {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let k = if a < b { (a, b) } else { (b, a) };
            edges
                .entry(k)
                .and_modify(|v: &mut (u32, u32, u32)| v.2 += 1)
                .or_insert((a, b, 1));
        }
    }
    Ok(edges
        .into_values()
        .filter(|v| v.2 == 1)
        .map(|(a, b, _)| {
            let a = result.vertices[a as usize];
            let b = result.vertices[b as usize];
            [a[0], a[1], b[0], b[1]]
        })
        .collect())
}
impl Renderer {
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        target: ChromeTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        primitive: &Prim,
        clips: &[Prim],
    ) -> Result<()> {
        let Prim::Path {
            points,
            stroke,
            color,
            origin,
            scale,
        } = primitive
        else {
            return Err("expected path primitive".into());
        };
        let key = key(points, stroke);
        if !self.geometry.contains_key(&key) {
            let edges = tessellate(points, stroke)?;
            let count = edges.len() as u32;
            if count == 0 {
                return Ok(());
            }
            if count > 65536 {
                return Err("path edge budget exceeded".into());
            }
            let mut lo = [f32::INFINITY; 2];
            let mut hi = [f32::NEG_INFINITY; 2];
            for e in &edges {
                for p in [[e[0], e[1]], [e[2], e[3]]] {
                    for j in 0..2 {
                        lo[j] = lo[j].min(p[j]);
                        hi[j] = hi[j].max(p[j]);
                    }
                }
            }
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("retained path edges"),
                contents: bytemuck::cast_slice(&edges),
                usage: wgpu::BufferUsages::STORAGE,
            });
            self.geometry.insert(
                key.clone(),
                Geometry {
                    edges: buffer,
                    count,
                    bounds: [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]],
                },
            );
        }
        let geometry = &self.geometry[&key];
        let format = target.format.remove_srgb_suffix();
        self.pipelines
            .entry(format)
            .or_insert_with(|| pipeline(device, format));
        let mut clip_data = vec![];
        for clip in clips {
            if let Prim::Clip { rect, radii } = clip {
                clip_data.push([rect.x, rect.y, rect.w, rect.h]);
                clip_data.push(*radii);
            }
        }
        let clip_count = clip_data.len() / 2;
        if clip_data.is_empty() {
            clip_data.push([0.; 4]);
        }
        let s = target.scale * *scale;
        let data = [
            [
                target.width as f32,
                target.height as f32,
                target.scale,
                clip_count as f32,
            ],
            [
                (origin[0] * target.scale).round(),
                (origin[1] * target.scale).round(),
                s,
                geometry.count as f32,
            ],
            geometry.bounds,
            color.0,
        ];
        let u = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("path transform"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let clips = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("path clips"),
            contents: bytemuck::cast_slice(&clip_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let pipeline = &self.pipelines[&format];
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("path bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: u.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: geometry.edges.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: clips.as_entire_binding(),
                },
            ],
        });
        let view = target
            .view
            .texture()
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(format),
                ..Default::default()
            });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("analytic path coverage"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..6, 0..1);
        Ok(())
    }
}
fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("path analytic edges"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                include_str!("shapes.wgsl"),
                include_str!("paths.wgsl")
            )
            .into(),
        ),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("path stroke"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::ChromeRenderer;
    use super::*;
    use bbt_ui_model::{
        draw::{Color, DrawList},
        layout::Rect,
    };
    #[test]
    fn sub_ulp_dash_remainder_advances_instead_of_repeating_a_vertex() {
        let points = [
            PathCommand::Move([0., 0.]),
            // Exactly 3.999998092651367: leaves a 2^-19 dash remainder.
            PathCommand::Line([f32::from_bits(0x407f_fff8), 0.]),
            PathCommand::Line([204., 0.]),
        ];
        let (path, dots) = dashed(&points, &[4., 4.]).unwrap();
        let events: Vec<_> = path.iter().collect();
        assert!(
            events.len() < 200,
            "bounded dash expansion: {}",
            events.len()
        );
        assert!(dots.is_empty());
        assert!(events.iter().any(
            |e| matches!(e, lyon_tessellation::path::PathEvent::Line { to, .. } if to.x > 195.)
        ));
        assert!(events.iter().all(|e| match e {
            lyon_tessellation::path::PathEvent::Begin { at } =>
                at.x.is_finite() && at.y.is_finite(),
            lyon_tessellation::path::PathEvent::Line { from, to } =>
                from.x.is_finite() && to.x.is_finite(),
            _ => true,
        }));
    }
    #[test]
    fn unrepresentable_pattern_cannot_cycle_through_zero_length_phases() {
        let points = [PathCommand::Move([0., 0.]), PathCommand::Line([1., 0.])];
        assert!(dashed(&points, &[0., 0.000_000_1]).is_err());
    }
    fn path(points: Vec<PathCommand>, stroke: PathStroke) -> Prim {
        Prim::Path {
            origin: [0.; 2],
            scale: 1.,
            points: points.into(),
            stroke,
            color: Color([1., 0., 0., 0.5]),
        }
    }
    #[test]
    fn gpu_path_preserves_order_clip_alpha_and_retained_geometry() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut list = DrawList::default();
        list.rect(Rect::new(0., 0., 64., 32.), 0., Color::WHITE);
        list.clip(Rect::new(16., 0., 48., 32.), [0.; 4]);
        list.push(path(
            vec![PathCommand::Move([8., 16.]), PathCommand::Line([56., 16.])],
            PathStroke {
                width: 4.,
                ..Default::default()
            },
        ));
        list.unclip();
        list.rect(Rect::new(40., 0., 24., 32.), 0., Color([0., 0., 1., 1.]));
        let a = renderer.render_to_rgba(&list, 64, 32, 1.).unwrap();
        let pixel = |x: usize, y: usize| &a[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
        assert_eq!(pixel(10, 16), [255, 255, 255, 255]);
        assert_eq!(pixel(20, 16), [255, 128, 128, 255]);
        assert_eq!(pixel(50, 16), [0, 0, 255, 255]);
        assert_eq!(a, renderer.render_to_rgba(&list, 64, 32, 1.).unwrap());
        assert_eq!(renderer.paths.geometry.len(), 1);
        let Prim::Path { origin, color, .. } = &mut list.primitives[2] else {
            unreachable!()
        };
        *origin = [0., 1.];
        *color = Color::BLACK;
        renderer.render_to_rgba(&list, 64, 32, 1.).unwrap();
        assert_eq!(
            renderer.paths.geometry.len(),
            1,
            "paint and translation do not retessellate"
        );
    }
    #[test]
    fn zero_length_round_dashes_render_dots_and_crossings_composite_once() {
        let mut renderer = ChromeRenderer::new(vec![]).unwrap();
        let mut list = DrawList::default();
        list.push(path(
            vec![PathCommand::Move([4., 4.]), PathCommand::Line([28., 4.])],
            PathStroke {
                width: 2.,
                dash: vec![0., 6.].into(),
                cap: LineCap::Round,
                ..Default::default()
            },
        ));
        list.push(path(
            vec![
                PathCommand::Move([4., 14.]),
                PathCommand::Line([28., 28.]),
                PathCommand::Line([28., 14.]),
                PathCommand::Line([4., 28.]),
            ],
            PathStroke {
                width: 4.,
                ..Default::default()
            },
        ));
        let pixels = renderer.render_to_rgba(&list, 32, 32, 1.).unwrap();
        let alpha = |x: usize, y: usize| pixels[(y * 32 + x) * 4 + 3];
        assert!(alpha(4, 4) > 60);
        assert_eq!(alpha(7, 4), 0);
        assert!(alpha(10, 4) > 60);
        assert!(
            alpha(16, 21) >= 126 && alpha(16, 21) <= 128,
            "crossing is one source-over, not two"
        );
    }
    #[test]
    fn cubic_dash_geometry_is_finite_and_invalid_input_is_rejected() {
        let commands = [
            PathCommand::Move([0., 0.]),
            PathCommand::Cubic([[4., 0.], [8., 12.], [12., 12.]]),
        ];
        let s = PathStroke {
            dash: vec![3., 2., 1.].into(),
            cap: LineCap::Square,
            join: LineJoin::Bevel,
            ..Default::default()
        };
        let edges = tessellate(&commands, &s).unwrap();
        assert!(!edges.is_empty());
        assert!(edges.iter().flatten().all(|n| n.is_finite()));
        assert!(tessellate(&[PathCommand::Line([0., 0.])], &s).is_err());
        assert!(
            tessellate(
                &commands,
                &PathStroke {
                    width: f32::NAN,
                    ..s
                }
            )
            .is_err()
        );
    }
}
