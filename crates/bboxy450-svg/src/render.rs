// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    Diagnostic, Result, Scene,
    prepare::{Element, Text},
};
use bbt_render::chrome::{
    ChromeRenderer, ChromeTarget, MASK_BYTE_BUDGET, PreparedPath, group::Filter,
    prepare_vector_path,
};
use bbt_ui_model::{
    draw::{Affine, Color, DrawList, Paint, path_geometry},
    layout::Rect,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct RenderRequest {
    pub camera: [f64; 4],
    pub scale: f64,
}
impl RenderRequest {
    pub fn dimensions(self) -> Result<[u32; 2]> {
        crate::capture_viewport(self.camera, Some(self.camera), self.scale)
            .map(|m| m.size)
            .map_err(|e| Diagnostic::refused("capture_viewport", e))
    }
}
#[derive(Serialize)]
pub struct Geometry {
    pub source: String,
    pub bounds: [f64; 4],
    pub endpoints: Option<[[f64; 2]; 2]>,
}
pub struct Capture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub mapping: crate::CaptureMapping,
    pub geometry: Vec<Geometry>,
    pub texts: Vec<TextGeometry>,
}

#[derive(Serialize)]
pub struct CharacterGeometry {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub r#box: [f64; 4],
}
#[derive(Serialize)]
pub struct TextGeometry {
    pub source: String,
    pub text: String,
    pub postscript: String,
    pub weight: u16,
    pub font_size: f32,
    pub baseline_anchor: [f64; 2],
    pub anchor_mode: &'static str,
    pub advance: f64,
    pub advance_axis_scale: f64,
    pub bounds: [f64; 4],
    pub chars: Vec<CharacterGeometry>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Render,
    Vectors,
    Text,
    Image,
}
impl Scene {
    /// Paint the requested layer through shared preparation, compositor and readback.
    pub fn render(
        &self,
        renderer: &mut ChromeRenderer,
        request: RenderRequest,
        layer: Layer,
    ) -> Result<Capture> {
        let mapping = crate::capture_viewport(self.view_box(), Some(request.camera), request.scale)
            .map_err(|e| Diagnostic::refused("capture_viewport", e))?;
        let [width, height] = mapping.size;
        let camera = Affine(mapping.transform);
        let mut geometry = Vec::new();
        collect_geometry(&self.elements, request, &mut geometry);
        let mut texts = Vec::new();
        if matches!(layer, Layer::Render | Layer::Text) {
            collect_text(&self.elements, camera, &mut texts)?;
        }
        let mut bytes = 0;
        let mut elements = prepare_elements(
            &self.elements,
            camera,
            [width, height],
            [0., 0., width as f32, height as f32],
            &mut bytes,
            layer,
        )
        .map_err(|e| Diagnostic::refused("coverage", e.to_string()))?;
        let [x, y, w, h] = mapping.viewport;
        if mapping.root_clip {
            // The output surface itself clips cameras inside the root viewport.
            // Outside it, reuse group coverage rather than extending SVG paint.
            let path = bbt_ui_model::draw::VectorPath {
                points: crate::parse_path(&format!("M{x} {y}h{w}v{h}h{}Z", -w))?,
                transform: camera,
                fill: Some(Paint::Solid(Color::WHITE)),
                fill_rule: bbt_ui_model::draw::FillRule::NonZero,
                fill_opacity: 1.,
                stroke: bbt_ui_model::draw::PathStroke::default(),
                stroke_paint: None,
                stroke_opacity: 1.,
                miter_limit: 4.,
                dash_offset: 0.,
            };
            let clip = prepare_vector_path(
                &path,
                [width, height],
                [0., 0., width as f32, height as f32],
            )
            .map_err(|e| Diagnostic::refused("coverage", e.to_string()))?;
            charge(&clip, &mut bytes)
                .map_err(|e| Diagnostic::refused("coverage", e.to_string()))?;
            elements = vec![PreparedElement::Group {
                children: elements,
                opacity: 1.,
                clip: Some(clip),
            }];
        }
        install_images(&elements, renderer)
            .map_err(|e| Diagnostic::refused("image_upload", e.to_string()))?;
        let rgba = renderer
            .render_to_rgba_with(width, height, 1., |r, target, encoder| {
                let mut background = DrawList::default();
                background.rect(
                    Rect::new(0., 0., width as f32, height as f32),
                    0.,
                    Color::WHITE,
                );
                r.draw(&background, target, encoder)?;
                paint_elements(&elements, r, target, encoder)
            })
            .map_err(|e| Diagnostic::refused("render", e.to_string()))?;
        Ok(Capture {
            width,
            height,
            rgba,
            mapping,
            geometry,
            texts,
        })
    }
}
fn collect_text(elements: &[Element], camera: Affine, rows: &mut Vec<TextGeometry>) -> Result<()> {
    let point = |t: Affine, p: [f32; 2]| t.map(p).map(f64::from);
    let rectangle = |t: Affine, b: [f32; 4]| {
        let p = [
            [b[0], b[1]],
            [b[0] + b[2], b[1]],
            [b[0], b[1] + b[3]],
            [b[0] + b[2], b[1] + b[3]],
        ]
        .map(|p| point(t, p));
        let lo = [
            p.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
            p.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
        ];
        let hi = [
            p.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max),
            p.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max),
        ];
        [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]]
    };
    for e in elements {
        match e {
            Element::Group { children, .. } => collect_text(children, camera, rows)?,
            Element::Text(text) => {
                let layout = bbt_render::chrome::text::layout_face(
                    text.context
                        .as_ref()
                        .map_or(text.value.as_ref(), |(v, _)| v.as_ref()),
                    text.style,
                    &text.postscript,
                    text.context.as_ref().map(|(_, range)| *range),
                )
                .map_err(|e| Diagnostic::refused("text_geometry", e.to_string()))?;
                let t = camera.concat(text.transform);
                let start = [
                    text.baseline[0] - text.anchor * text.advance,
                    text.baseline[1],
                ];
                let translate = Affine([1., 0., 0., 1., start[0], start[1]]);
                let paint = t.concat(translate);
                let [a, b, _, _, _, _] = t.0;
                let axis = f64::from(a.hypot(b));
                let chars = layout
                    .characters
                    .iter()
                    .map(|p| CharacterGeometry {
                        start: point(paint, [p[0], 0.]),
                        end: point(paint, [p[1], 0.]),
                        r#box: rectangle(
                            paint,
                            [p[0], layout.bounds[1], p[1] - p[0], layout.bounds[3]],
                        ),
                    })
                    .collect();
                rows.push(TextGeometry {
                    source: text.source.clone(),
                    text: text.value.to_string(),
                    postscript: text.postscript.to_string(),
                    weight: text.style.weight,
                    font_size: text.style.pixels(),
                    baseline_anchor: point(t, text.baseline),
                    anchor_mode: match text.anchor {
                        0. => "start",
                        0.5 => "middle",
                        _ => "end",
                    },
                    advance: f64::from(layout.metrics.advance) * axis,
                    advance_axis_scale: axis,
                    bounds: rectangle(paint, layout.bounds),
                    chars,
                });
            }
            _ => {}
        }
    }
    Ok(())
}
fn collect_geometry(elements: &[Element], request: RenderRequest, rows: &mut Vec<Geometry>) {
    let camera = request.camera.map(|v| v as f32);
    let scale = request.scale as f32;
    for element in elements {
        match element {
            Element::Group { children, .. } => collect_geometry(children, request, rows),
            Element::Path(path) => {
                let transform = path.paint.transform;
                let points: Vec<_> = path
                    .paint
                    .points
                    .iter()
                    .map(|p| transform.command(*p))
                    .collect();
                let (bounds, ends) = path_geometry(&points);
                // Public bounds are world geometry, then a camera crop. Keep
                // that order so a translated deep-zoom view cannot lose an ULP
                // by fusing camera and scene matrices before bounding the path.
                let point = |p: [f32; 2]| {
                    [
                        f64::from((p[0] - camera[0]) * scale),
                        f64::from((p[1] - camera[1]) * scale),
                    ]
                };
                let origin = point([bounds[0], bounds[1]]);
                rows.push(Geometry {
                    source: path.source.clone(),
                    bounds: [
                        origin[0],
                        origin[1],
                        f64::from(bounds[2] * scale),
                        f64::from(bounds[3] * scale),
                    ],
                    endpoints: ends
                        .first()
                        .zip(ends.last())
                        .map(|(a, b)| [point(*a), point(*b)]),
                });
            }
            _ => {}
        }
    }
}
enum PreparedElement {
    Path(PreparedPath),
    Text(Text),
    Image {
        key: String,
        image: std::sync::Arc<bbt_render::image::Image>,
        transform: Affine,
    },
    Group {
        children: Vec<PreparedElement>,
        opacity: f32,
        clip: Option<PreparedPath>,
    },
}
fn charge(path: &PreparedPath, bytes: &mut usize) -> bbt_render::Result<()> {
    *bytes = bytes
        .checked_add(path.bytes())
        .ok_or("coverage byte overflow")?;
    if *bytes > MASK_BYTE_BUDGET {
        return Err("prepared vector coverage exceeds existing mask budget".into());
    }
    Ok(())
}
fn prepare_elements(
    elements: &[Element],
    camera: Affine,
    size: [u32; 2],
    clip: [f32; 4],
    bytes: &mut usize,
    layer: Layer,
) -> bbt_render::Result<Vec<PreparedElement>> {
    use sha2::{Digest, Sha256};
    let mut out = Vec::new();
    for element in elements {
        match element {
            Element::Path(path) if matches!(layer, Layer::Render | Layer::Vectors) => {
                let mut path = path.paint.clone();
                path.transform = camera.concat(path.transform);
                let prepared = prepare_vector_path(&path, size, clip)?;
                charge(&prepared, bytes)?;
                out.push(PreparedElement::Path(prepared));
            }
            Element::Group {
                children,
                opacity,
                clip: paths,
            } => {
                let mask = if let Some(paths) = paths {
                    if paths.len() != 1 {
                        return Err("multiple clip children are outside witnessed grammar".into());
                    }
                    let mut path = paths[0].paint.clone();
                    path.transform = camera.concat(path.transform);
                    path.fill = Some(Paint::Solid(Color::WHITE));
                    path.fill_opacity = 1.;
                    path.stroke_paint = None;
                    let p = prepare_vector_path(&path, size, clip)?;
                    charge(&p, bytes)?;
                    Some(p)
                } else {
                    None
                };
                out.push(PreparedElement::Group {
                    children: prepare_elements(children, camera, size, clip, bytes, layer)?,
                    opacity: *opacity,
                    clip: mask,
                });
            }
            Element::Text(text) if matches!(layer, Layer::Render | Layer::Text) => {
                let mut text = text.clone();
                text.transform = camera.concat(text.transform);
                out.push(PreparedElement::Text(text));
            }
            Element::Image {
                image, transform, ..
            } if matches!(layer, Layer::Render | Layer::Image) => {
                let mut digest = Sha256::new();
                digest.update(image.width.to_be_bytes());
                digest.update(image.height.to_be_bytes());
                digest.update(&image.rgba);
                let key = format!("svg:png:{:x}", digest.finalize());
                out.push(PreparedElement::Image {
                    key,
                    image: image.clone(),
                    transform: camera.concat(*transform),
                });
            }
            _ => {}
        }
    }
    Ok(out)
}
fn install_images(
    elements: &[PreparedElement],
    renderer: &mut ChromeRenderer,
) -> bbt_render::Result<()> {
    for element in elements {
        match element {
            PreparedElement::Group { children, .. } => install_images(children, renderer)?,
            PreparedElement::Image { key, image, .. } => {
                renderer.add_image(&bbt_render::chrome::icons::Image {
                    name: key.clone(),
                    width: image.width,
                    height: image.height,
                    rgba: image.rgba.clone(),
                })?
            }
            _ => {}
        }
    }
    Ok(())
}
fn paint_elements(
    elements: &[PreparedElement],
    renderer: &mut ChromeRenderer,
    target: ChromeTarget<'_>,
    encoder: &mut wgpu::CommandEncoder,
) -> bbt_render::Result<()> {
    for element in elements {
        match element {
            PreparedElement::Path(path) => renderer.draw_prepared_path(path, target, encoder)?,
            PreparedElement::Group {
                children,
                opacity,
                clip,
            } => {
                if *opacity == 1. && clip.is_none() {
                    paint_elements(children, renderer, target, encoder)?;
                } else {
                    renderer.paint_group_coverage(
                        Filter {
                            opacity: *opacity,
                            saturation: 1.,
                        },
                        clip.as_ref(),
                        target,
                        encoder,
                        |r, t, e| paint_elements(children, r, t, e),
                    )?;
                }
            }
            PreparedElement::Text(text) => {
                let origin = Affine([
                    1.,
                    0.,
                    0.,
                    1.,
                    text.baseline[0] - text.anchor * text.advance,
                    text.baseline[1],
                ]);
                renderer.draw_face_text(
                    &bbt_render::chrome::text::FaceText {
                        value: text
                            .context
                            .as_ref()
                            .map_or(text.value.as_ref(), |(v, _)| v.as_ref()),
                        range: text.context.as_ref().map(|(_, range)| *range),
                        postscript: &text.postscript,
                        style: text.style,
                        transform: text.transform.concat(origin),
                        color: text.color,
                    },
                    target,
                    encoder,
                )?;
            }
            PreparedElement::Image {
                key,
                image,
                transform,
            } => renderer.draw_image_affine(
                key,
                [image.width, image.height],
                *transform,
                target,
                encoder,
            )?,
        }
    }
    Ok(())
}

impl Scene {
    /// A live placement shares admitted geometry with the capture path.
    /// Coverage is disposable and keyed by exact device transform and clip.
    pub fn retained_image(&self) -> bbt_ui_model::draw::RetainedImage {
        let scene = self.clone();
        let last = std::sync::Mutex::new(None::<([u32; 6], [u32; 2], Arc<[PreparedElement]>)>);
        bbt_render::chrome::retained_picture(
            self.viewport(),
            move |renderer, target, encoder, place| {
                let [x, y, width, height] = place.region.pixels;
                let bounds = place.bounds;
                let [left, top, w, h] = scene.view_box().map(|n| n as f32);
                let scale = target.scale * (bounds.w / w).min(bounds.h / h);
                let tx = (bounds.x + (bounds.w - w * scale / target.scale) * 0.5) * target.scale
                    - x as f32
                    - left * scale;
                let ty = (bounds.y + (bounds.h - h * scale / target.scale) * 0.5) * target.scale
                    - y as f32
                    - top * scale;
                let camera = Affine([scale, 0., 0., scale, tx, ty]);
                let size = [width, height];
                let key = camera.0.map(f32::to_bits);
                let cached = last.try_lock().ok().and_then(|guard| {
                    guard
                        .as_ref()
                        .filter(|(old, extent, _)| *old == key && *extent == size)
                        .map(|(_, _, old)| old.clone())
                });
                let pixels = u64::from(width) * u64::from(height);
                if pixels > 4_000_000 || width > 4096 || height > 4096 {
                    return Err("visible SVG clip exceeds its native render budget".into());
                }
                let elements = match cached {
                    Some(scene) => scene,
                    None => {
                        let mut bytes = 0;
                        let prepared: Arc<[PreparedElement]> = prepare_elements(
                            &scene.elements,
                            camera,
                            size,
                            [0., 0., width as f32, height as f32],
                            &mut bytes,
                            Layer::Render,
                        )?
                        .into();
                        if let Ok(mut cache) = last.try_lock() {
                            *cache = Some((key, size, prepared.clone()));
                        }
                        prepared
                    }
                };
                if pixels * (1 + isolated_depth(&elements)) > 4_000_000 {
                    return Err(
                        "visible SVG isolation layers exceed its native render budget".into(),
                    );
                }
                install_images(&elements, renderer)?;
                renderer.paint_image_region(
                    place.region,
                    place.opacity,
                    target,
                    encoder,
                    |renderer, target, encoder| {
                        paint_elements(&elements, renderer, target, encoder)
                    },
                )
            },
        )
    }
}

fn isolated_depth(elements: &[PreparedElement]) -> u64 {
    elements
        .iter()
        .map(|element| match element {
            PreparedElement::Group {
                children,
                opacity,
                clip,
            } => u64::from(*opacity != 1. || clip.is_some()) + isolated_depth(children),
            _ => 0,
        })
        .max()
        .unwrap_or_default()
}
