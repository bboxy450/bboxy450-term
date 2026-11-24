// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    Diagnostic, FontSet, MAX_DEPTH, MAX_GLYPHS, MAX_PNG_PIXELS, MAX_PNGS, MAX_SCENE_COMMANDS,
    MAX_SVG_BYTES, MAX_XML_NODES, Result, parse_path,
};
use base64::Engine;
use bbt_ui_model::draw::{
    Affine, Color, ColorStop, FillRule, LineCap, LineJoin, Paint, PathCommand, PathStroke,
    TextStyle, TypeScale,
};
use roxmltree::{Document, Node, NodeId};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::Arc,
};

const SVG: &str = "http://www.w3.org/2000/svg";
type Style = BTreeMap<String, String>;

/// Validated source geometry, retained independently of a capture's pixels.
#[derive(Clone)]
pub struct Scene {
    pub(crate) elements: Arc<[Element]>,
    view_box: [f64; 4],
    viewport: [f32; 2],
    decoded_pixels: u64,
}
impl Scene {
    pub fn view_box(&self) -> [f64; 4] {
        self.view_box
    }
    pub fn viewport(&self) -> [f32; 2] {
        self.viewport
    }
    pub fn decoded_pixels(&self) -> u64 {
        self.decoded_pixels
    }
}
#[derive(Clone)]
pub(crate) enum Element {
    Group {
        children: Arc<[Element]>,
        opacity: f32,
        clip: Option<Arc<[Path]>>,
    },
    Path(Path),
    Text(Text),
    Image {
        image: Arc<bbt_render::image::Image>,
        transform: Affine,
    },
}
#[derive(Clone)]
pub(crate) struct Path {
    pub source: String,
    pub paint: bbt_ui_model::draw::VectorPath,
}

#[derive(Clone)]
pub(crate) struct Text {
    pub source: String,
    pub value: Arc<str>,
    pub context: Option<(Arc<str>, [usize; 2])>,
    pub postscript: Arc<str>,
    pub style: TextStyle,
    pub anchor: f32,
    pub advance: f32,
    pub baseline: [f32; 2],
    pub transform: Affine,
    pub color: Color,
}

fn refused(code: &'static str, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::refused(code, detail)
}
fn number(value: &str) -> Result<f32> {
    let n = value
        .trim()
        .parse::<f32>()
        .map_err(|_| refused("number", value))?;
    if !n.is_finite() {
        return Err(refused("nonfinite", value));
    }
    Ok(n)
}
fn length(value: &str) -> Result<f32> {
    let length = svgtypes::Length::from_str(value).map_err(|e| refused("length", e.to_string()))?;
    use svgtypes::LengthUnit as U;
    let scale = match length.unit {
        U::None | U::Px => 1.,
        U::Pt => 96. / 72.,
        U::Pc => 16.,
        U::In => 96.,
        U::Cm => 96. / 2.54,
        U::Mm => 96. / 25.4,
        U::Em => 16.,
        U::Ex => 8.,
        _ => return Err(Diagnostic::unsupported(format!("length unit in {value}"))),
    };
    number(&(length.number * scale).to_string())
}
fn attr_number(n: Node<'_, '_>, name: &str, default: f32) -> Result<f32> {
    n.attribute(name).map(number).unwrap_or(Ok(default))
}
fn unit(value: &str) -> Result<f32> {
    let value = value.trim();
    let n = match value.strip_suffix('%') {
        Some(percent) => number(percent)? / 100.,
        None => number(value)?,
    };
    Ok(n.clamp(0., 1.))
}
fn transform(value: Option<&str>) -> Result<Affine> {
    // Browsers ignore an unparsable transform attribute; so do we.
    let t = svgtypes::Transform::from_str(value.unwrap_or("")).unwrap_or_default();
    let t = Affine([
        t.a as f32, t.b as f32, t.c as f32, t.d as f32, t.e as f32, t.f as f32,
    ]);
    t.validate().map_err(|e| refused("nonfinite", e))?;
    Ok(t)
}
fn color(value: &str) -> Result<Color> {
    let c = svgtypes::Color::from_str(value).map_err(|e| refused("color", e.to_string()))?;
    Ok(Color([
        f32::from(c.red) / 255.,
        f32::from(c.green) / 255.,
        f32::from(c.blue) / 255.,
        f32::from(c.alpha) / 255.,
    ]))
}
fn rule(value: &str) -> Result<FillRule> {
    Ok(match value {
        "evenodd" => FillRule::EvenOdd,
        _ => FillRule::NonZero,
    })
}
fn style_property(name: &str) -> bool {
    matches!(
        name,
        "fill"
            | "fill-rule"
            | "fill-opacity"
            | "stroke"
            | "stroke-width"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-opacity"
            | "opacity"
            | "display"
            | "clip-path"
            | "clip-rule"
            | "font-family"
            | "font-size"
            | "font-weight"
            | "text-anchor"
            | "color"
    )
}
fn declarations(value: &str) -> Result<Style> {
    let mut style = Style::new();
    for declaration in value.split(';').filter(|v| !v.trim().is_empty()) {
        let (name, value) = declaration
            .split_once(':')
            .ok_or_else(|| refused("style", "malformed declaration"))?;
        let value = value.trim();
        let value = value.strip_suffix("!important").unwrap_or(value).trim();
        if value.contains(['{', '}', '@']) {
            return Err(refused("style", "unsupported CSS syntax"));
        }
        // Like a browser, an unknown property is ignored rather than refused.
        if style_property(name.trim()) {
            style.insert(name.trim().into(), value.into());
        }
    }
    Ok(style)
}
fn get<'a>(style: &'a Style, name: &str, default: &'a str) -> &'a str {
    style.get(name).map(String::as_str).unwrap_or(default)
}
fn inert(n: Node<'_, '_>) -> bool {
    n.ancestors().any(|p| {
        p.is_element()
            && p.tag_name().namespace() == Some(SVG)
            && matches!(p.tag_name().name(), "metadata" | "title" | "desc")
    })
}
fn location(n: Node<'_, '_>) -> String {
    let mut parts = Vec::new();
    let mut at = Some(n);
    while let Some(node) = at {
        let tag = node.tag_name().name();
        let index = node
            .prev_siblings()
            .filter(|s| s.is_element() && s.id() != node.id() && s.tag_name().name() == tag)
            .count()
            + 1;
        parts.push(format!("/{tag}[{index}]"));
        at = node.parent_element();
    }
    parts.reverse();
    parts.concat()
}

fn href<'a>(n: Node<'a, '_>) -> Result<&'a str> {
    let plain = n.attribute("href");
    let namespaced = n.attribute(("http://www.w3.org/1999/xlink", "href"));
    if plain.is_some() && namespaced.is_some() && plain != namespaced {
        return Err(refused("href", "conflicting hrefs"));
    }
    plain
        .or(namespaced)
        .ok_or_else(|| refused("href", "reference is missing"))
}

/// Safety census before lowering. Like a browser, anything merely unknown
/// (elements, attributes, foreign namespaces) is ignored; only active content
/// and references that could reach outside the document are refused.
fn audit(doc: &Document<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for n in doc.descendants().filter(Node::is_element) {
        if inert(n) {
            continue;
        }
        let tag = n.tag_name().name();
        if matches!(tag, "script" | "foreignObject" | "iframe") {
            diagnostics.push(refused("active_content", tag));
            continue;
        }
        for a in n.attributes() {
            let name = a.name();
            let value = a.value();
            if name.starts_with("on") {
                diagnostics.push(refused("active_content", name));
                continue;
            }
            if name == "href" {
                if value.starts_with('#') && value.len() > 1 {
                    continue;
                }
                if tag == "image" && value.starts_with("data:image/") {
                    continue;
                }
                diagnostics.push(refused("external_reference", value));
                continue;
            }
            if value.contains("url(") && !value.trim().starts_with("url(#") {
                diagnostics.push(refused("external_reference", value));
            }
        }
    }
    diagnostics
}

struct Context<'a, 'input> {
    doc: &'a Document<'input>,
    fonts: Option<&'a FontSet>,
    ids: HashMap<String, NodeId>,
    rules: Vec<(Vec<Selector>, Style)>,
    gradients: HashMap<String, Paint>,
    images: HashMap<NodeId, Arc<bbt_render::image::Image>>,
    stack: Vec<NodeId>,
    commands: usize,
    glyphs: usize,
    expanded: usize,
}
impl<'a, 'input> Context<'a, 'input> {
    fn style(&self, n: Node<'_, '_>, parent: &Style) -> Result<Style> {
        let mut style = parent.clone();
        for name in ["opacity", "clip-path", "display"] {
            style.remove(name);
        }
        for a in n
            .attributes()
            .filter(|a| a.namespace().is_none() && style_property(a.name()))
        {
            style.insert(a.name().into(), a.value().into());
        }
        for (selectors, declared) in &self.rules {
            if selectors.iter().any(|s| s.matches(n)) {
                style.extend(declared.clone());
            }
        }
        style.extend(declarations(n.attribute("style").unwrap_or(""))?);
        Ok(style)
    }
    fn target(&self, reference: &str) -> Result<Node<'a, 'input>> {
        let id = reference
            .strip_prefix('#')
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refused("external_reference", reference))?;
        let id = self
            .ids
            .get(id)
            .ok_or_else(|| refused("missing_reference", reference))?;
        self.doc
            .get_node(*id)
            .ok_or_else(|| refused("missing_reference", reference))
    }
    /// Browser-style paint: `url(#id) fallback`, `currentColor`, and an
    /// unparsable value falls back to the property's initial paint.
    fn paint(&self, value: &str, style: &Style, initial: Option<Paint>) -> Result<Option<Paint>> {
        let value = value.trim();
        if value == "none" {
            return Ok(None);
        }
        if let Some(rest) = value.strip_prefix("url(#") {
            let (id, fallback) = rest.split_once(')').unwrap_or((rest, ""));
            if let Some(paint) = self.gradients.get(id.trim()) {
                return Ok(Some(paint.clone()));
            }
            let fallback = fallback.trim();
            return if fallback.is_empty() {
                Ok(None)
            } else {
                self.paint(fallback, style, initial)
            };
        }
        if value.eq_ignore_ascii_case("currentcolor") {
            let current = get(style, "color", "black");
            if current.eq_ignore_ascii_case("currentcolor") {
                return Ok(Some(Paint::Solid(Color([0., 0., 0., 1.]))));
            }
            return self.paint(current, style, initial);
        }
        Ok(color(value).map(Paint::Solid).ok().or(initial))
    }
    fn path(
        &mut self,
        n: Node<'_, '_>,
        points: Arc<[PathCommand]>,
        style: &Style,
        transform: Affine,
    ) -> Result<Path> {
        self.commands = self
            .commands
            .checked_add(points.len())
            .ok_or_else(|| refused("scene_commands", "overflow"))?;
        if self.commands > MAX_SCENE_COMMANDS {
            return Err(refused(
                "scene_commands",
                "scene exceeds lowered command bound",
            ));
        }
        let width = length(get(style, "stroke-width", "1"))
            .unwrap_or(1.)
            .max(0.);
        let cap = match get(style, "stroke-linecap", "butt") {
            "round" => LineCap::Round,
            "square" => LineCap::Square,
            _ => LineCap::Butt,
        };
        let join = match get(style, "stroke-linejoin", "miter") {
            "round" => LineJoin::Round,
            "bevel" => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
        let dash = get(style, "stroke-dasharray", "none");
        let dash: Vec<f32> = if dash == "none" {
            vec![]
        } else {
            svgtypes::NumberListParser::from(dash)
                .take(257)
                .map(|n| {
                    n.map_err(|e| refused("stroke", e.to_string()))
                        .and_then(|n| number(&n.to_string()))
                })
                .collect::<Result<_>>()?
        };
        let stroke = PathStroke {
            width,
            cap,
            join,
            dash: dash.into(),
        };
        stroke.validate(&points).map_err(|e| refused("path", e))?;
        let miter_limit = number(get(style, "stroke-miterlimit", "4"))
            .unwrap_or(4.)
            .max(1.);
        let black = Some(Paint::Solid(Color([0., 0., 0., 1.])));
        let fill = self.paint(get(style, "fill", "black"), style, black)?;
        let stroke_paint = self.paint(get(style, "stroke", "none"), style, None)?;
        Ok(Path {
            source: location(n),
            paint: bbt_ui_model::draw::VectorPath {
                points,
                transform,
                fill,
                fill_rule: rule(get(style, "fill-rule", "nonzero"))?,
                fill_opacity: unit(get(style, "fill-opacity", "1"))?,
                stroke,
                stroke_paint,
                stroke_opacity: unit(get(style, "stroke-opacity", "1"))?,
                miter_limit,
                dash_offset: number(get(style, "stroke-dashoffset", "0"))?,
            },
        })
    }
    fn lower(
        &mut self,
        n: Node<'a, 'input>,
        parent: &Style,
        inherited: Affine,
        depth: usize,
    ) -> Result<Vec<Element>> {
        self.expanded += 1;
        if depth > MAX_DEPTH || self.expanded > MAX_XML_NODES {
            return Err(refused("reference_expansion", "expanded node/depth bound"));
        }
        if !n.is_element() || inert(n) {
            return Ok(vec![]);
        }
        let tag = n.tag_name().name();
        if n.tag_name().namespace() != Some(SVG)
            || !matches!(
                tag,
                "svg"
                    | "g"
                    | "a"
                    | "switch"
                    | "use"
                    | "path"
                    | "rect"
                    | "line"
                    | "circle"
                    | "ellipse"
                    | "polygon"
                    | "polyline"
                    | "text"
                    | "image"
            )
        {
            // defs, gradients, clip paths, symbols, filters, masks, animation and
            // anything unknown paint nothing directly, as in a browser.
            return Ok(vec![]);
        }
        let style = self.style(n, parent)?;
        let transform = inherited.concat(transform(n.attribute("transform"))?);
        transform.validate().map_err(|e| refused("nonfinite", e))?;
        let mut elements = match tag {
            "svg" | "g" | "a" | "switch" => {
                let mut t = transform;
                let mut clip = None;
                if tag == "svg" && n.parent_element().is_some() {
                    let box_ = view_box(n)?.map(|v| v as f32);
                    let w = n
                        .attribute("width")
                        .and_then(|v| length(v).ok())
                        .unwrap_or(box_[2]);
                    let h = n
                        .attribute("height")
                        .and_then(|v| length(v).ok())
                        .unwrap_or(box_[3]);
                    let x = attr_number(n, "x", 0.)?;
                    let y = attr_number(n, "y", 0.)?;
                    let rectangle = parse_path(&format!("M{x} {y}h{w}v{h}h{}Z", -w))?;
                    clip = Some(Arc::from([self.path(
                        n,
                        rectangle,
                        &Style::new(),
                        transform,
                    )?]));
                    t = transform.concat(viewport_transform(
                        box_,
                        [x, y, w, h],
                        n.attribute("preserveAspectRatio"),
                    )?);
                }
                let mut children = Vec::new();
                for child in n.children().filter(Node::is_element) {
                    children.extend(self.lower(child, &style, t, depth + 1)?);
                }
                vec![Element::Group {
                    children: children.into(),
                    opacity: 1.,
                    clip,
                }]
            }
            "use" => {
                let target = self.target(href(n)?)?;
                if self.stack.contains(&target.id()) {
                    return Err(refused("reference_cycle", href(n)?));
                }
                self.stack.push(target.id());
                let translate = Affine([
                    1.,
                    0.,
                    0.,
                    1.,
                    attr_number(n, "x", 0.)?,
                    attr_number(n, "y", 0.)?,
                ]);
                let result = if target.tag_name().name() == "symbol" {
                    let mut children = Vec::new();
                    let mut failed = None;
                    for child in target.children().filter(Node::is_element) {
                        match self.lower(child, &style, transform.concat(translate), depth + 1) {
                            Ok(c) => children.extend(c),
                            Err(e) => {
                                failed = Some(e);
                                break;
                            }
                        }
                    }
                    failed.map_or(Ok(children), Err)
                } else {
                    self.lower(target, &style, transform.concat(translate), depth + 1)
                };
                self.stack.pop();
                let mut elements = result?;
                fn identify(elements: &mut [Element], source: &str) {
                    for element in elements {
                        match element {
                            Element::Path(path) => path.source = source.to_owned(),
                            Element::Group { children, .. } => {
                                identify(Arc::make_mut(children), source)
                            }
                            _ => {}
                        }
                    }
                }
                identify(&mut elements, &location(n));
                elements
            }
            "path" | "rect" | "line" | "circle" | "ellipse" | "polygon" | "polyline" => {
                let points = shape(n)?;
                if points.is_empty() {
                    return Ok(vec![]);
                }
                vec![Element::Path(self.path(n, points, &style, transform)?)]
            }
            "text" => self.text(n, &style, transform, depth)?,
            "image" => {
                let x = attr_number(n, "x", 0.)?;
                let y = attr_number(n, "y", 0.)?;
                let w = attr_number(n, "width", 0.)?;
                let h = attr_number(n, "height", 0.)?;
                let Some(image) = self.images.get(&n.id()).cloned() else {
                    // Non-PNG data images are skipped rather than refused.
                    return Ok(vec![]);
                };
                let (w, h) = if w > 0. && h > 0. {
                    (w, h)
                } else {
                    (image.width as f32, image.height as f32)
                };
                let placement = viewport_transform(
                    [0., 0., image.width as f32, image.height as f32],
                    [x, y, w, h],
                    n.attribute("preserveAspectRatio"),
                )?;
                vec![Element::Image {
                    image,
                    transform: transform.concat(placement),
                }]
            }
            _ => return Err(Diagnostic::unsupported(format!("paint element {tag}"))),
        };
        let opacity = unit(get(&style, "opacity", "1"))?;
        let clip = match style
            .get("clip-path")
            .and_then(|v| v.trim().strip_prefix("url(#"))
            .and_then(|v| v.split_once(')'))
            .map(|(id, _)| id.trim().to_owned())
        {
            None => None,
            Some(id) => self.clip(&format!("#{id}"), transform)?,
        };
        // Validate hidden geometry and references too, then omit its paint.
        if get(&style, "display", "inline") == "none" {
            return Ok(vec![]);
        }
        if opacity != 1. || clip.is_some() {
            elements = vec![Element::Group {
                children: elements.into(),
                opacity,
                clip,
            }];
        }
        Ok(elements)
    }
    /// A clip whose target is missing, cyclic or not a user-space clipPath is
    /// ignored, as a browser does with an unresolvable clip-path.
    fn clip(&mut self, reference: &str, transform: Affine) -> Result<Option<Arc<[Path]>>> {
        let Ok(target) = self.target(reference) else {
            return Ok(None);
        };
        if target.tag_name().name() != "clipPath"
            || self.stack.contains(&target.id())
            || target
                .attribute("clipPathUnits")
                .is_some_and(|v| v != "userSpaceOnUse")
        {
            return Ok(None);
        }
        self.stack.push(target.id());
        let t = transform.concat(transform_fn(target.attribute("transform"))?);
        let mut paths = Vec::new();
        for child in target.children().filter(Node::is_element) {
            let points = shape(child)?;
            if points.is_empty() {
                continue;
            }
            let s = self.style(child, &Style::new())?;
            let mut p = self.path(
                child,
                points,
                &s,
                t.concat(transform_fn(child.attribute("transform"))?),
            )?;
            p.paint.fill_rule = rule(get(&s, "clip-rule", "nonzero"))?;
            paths.push(p);
        }
        self.stack.pop();
        Ok(Some(paths.into()))
    }
    fn text(
        &mut self,
        n: Node<'a, 'input>,
        style: &Style,
        transform: Affine,
        depth: usize,
    ) -> Result<Vec<Element>> {
        let fonts = self
            .fonts
            .ok_or_else(|| refused("font_unavailable", "no active FontSet"))?;
        let mut cursor = [
            attr_number(n, "x", 0.)? + attr_number(n, "dx", 0.)?,
            attr_number(n, "y", 0.)? + attr_number(n, "dy", 0.)?,
        ];
        let mut out = Vec::new();
        for child in n.children() {
            if child.is_text() {
                let value = child.text().unwrap_or("").replace(['\n', '\r', '\t'], " ");
                if value.is_empty() {
                    continue;
                }
                self.glyphs = self
                    .glyphs
                    .checked_add(value.chars().count())
                    .ok_or_else(|| refused("glyphs", "overflow"))?;
                if self.glyphs > MAX_GLYPHS {
                    return Err(refused("glyphs", "scene exceeds glyph bound"));
                }
                // Any family/weight renders with the nearest bundled face.
                let weight = match get(style, "font-weight", "400") {
                    "bold" | "bolder" => 700,
                    "normal" | "lighter" => 400,
                    w => {
                        if number(w).unwrap_or(400.) >= 600. {
                            700
                        } else {
                            400
                        }
                    }
                };
                let face = fonts.face(weight)?;
                let size = length(get(style, "font-size", "16")).unwrap_or(16.);
                if size <= 0. {
                    continue;
                }
                let text_style = TextStyle {
                    scale: TypeScale::Custom(size),
                    weight: weight as u16,
                    ..Default::default()
                };
                let metrics = bbt_render::chrome::text::measure_face(
                    &value,
                    text_style,
                    &face.postscript_name,
                )
                .map_err(|e| refused("font_unavailable", e.to_string()))?;
                let anchor = match get(style, "text-anchor", "start") {
                    "middle" => 0.5,
                    "end" => 1.,
                    _ => 0.,
                };
                let black = Color([0., 0., 0., 1.]);
                let fill = self.paint(
                    get(style, "fill", "black"),
                    style,
                    Some(Paint::Solid(black)),
                )?;
                let color = match fill {
                    Some(Paint::Solid(c)) => c.alpha(unit(get(style, "fill-opacity", "1"))?),
                    None => Color::TRANSPARENT,
                    Some(Paint::LinearGradient { stops, .. }) => stops
                        .first()
                        .map_or(black, |s| s.color)
                        .alpha(unit(get(style, "fill-opacity", "1"))?),
                };
                out.push(Element::Text(Text {
                    source: location(n),
                    value: value.into(),
                    context: None,
                    postscript: face.postscript_name.clone().into(),
                    style: text_style,
                    anchor,
                    advance: metrics.advance,
                    baseline: cursor,
                    transform,
                    color,
                }));
                cursor[0] += metrics.advance;
            } else if child.is_element() {
                if !matches!(child.tag_name().name(), "tspan" | "a") {
                    continue;
                }
                if depth + 1 > MAX_DEPTH {
                    return Err(refused("depth", "text nesting bound"));
                }
                let s = self.style(child, style)?;
                let x = child.attribute("x").map(number).unwrap_or(Ok(cursor[0]))?
                    + attr_number(child, "dx", 0.)?;
                let y = child.attribute("y").map(number).unwrap_or(Ok(cursor[1]))?
                    + attr_number(child, "dy", 0.)?;
                // Position inheritance is passed explicitly, never rewritten into source XML.
                let children = self.text_at(
                    child,
                    &s,
                    transform.concat(transform_fn(child.attribute("transform"))?),
                    depth + 1,
                    [x, y],
                )?;
                if let Some(Element::Text(t)) = children.last() {
                    let m =
                        bbt_render::chrome::text::measure_face(&t.value, t.style, &t.postscript)
                            .map_err(|e| refused("font_unavailable", e.to_string()))?;
                    cursor = [t.baseline[0] + m.advance, t.baseline[1]];
                }
                out.extend(children);
            }
        }
        if n.has_tag_name((SVG, "text"))
            && out.len() > 1
            && n.children()
                .filter(Node::is_element)
                .all(|c| c.attribute("x").is_none() && c.attribute("y").is_none())
        {
            let texts: Vec<_> = out
                .iter()
                .filter_map(|e| {
                    if let Element::Text(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .collect();
            if texts.len() == out.len()
                && texts.iter().all(|t| {
                    t.postscript == texts[0].postscript
                        && t.style == texts[0].style
                        && t.transform == texts[0].transform
                })
            {
                let context: Arc<str> = texts
                    .iter()
                    .map(|t| t.value.as_ref())
                    .collect::<String>()
                    .into();
                let mut start = 0;
                let mut correction = 0.;
                for element in &mut out {
                    let Element::Text(t) = element else {
                        unreachable!()
                    };
                    let end = start + t.value.encode_utf16().count();
                    let layout = bbt_render::chrome::text::layout_face(
                        &context,
                        t.style,
                        &t.postscript,
                        Some([start, end]),
                    )
                    .map_err(|e| refused("text_geometry", e.to_string()))?;
                    t.baseline[0] += correction;
                    correction += layout.metrics.advance - t.advance;
                    t.advance = layout.metrics.advance;
                    t.context = Some((context.clone(), [start, end]));
                    start = end;
                }
            }
        }
        Ok(out)
    }
    fn text_at(
        &mut self,
        n: Node<'a, 'input>,
        style: &Style,
        transform: Affine,
        depth: usize,
        position: [f32; 2],
    ) -> Result<Vec<Element>> {
        let mut out = self.text(n, style, transform, depth)?;
        let original = [
            attr_number(n, "x", 0.)? + attr_number(n, "dx", 0.)?,
            attr_number(n, "y", 0.)? + attr_number(n, "dy", 0.)?,
        ];
        for item in &mut out {
            if let Element::Text(t) = item {
                for i in 0..2 {
                    t.baseline[i] += position[i] - original[i];
                }
            }
        }
        Ok(out)
    }
}

// Keep the helper callable where a local `transform` record is in scope.
fn transform_fn(value: Option<&str>) -> Result<Affine> {
    transform(value)
}

/// Basic shapes lower to path data the way SVG 2 defines them; an empty
/// result means the shape is not rendered (zero size or non-shape element).
fn shape(n: Node<'_, '_>) -> Result<Arc<[PathCommand]>> {
    let tag = n.tag_name().name();
    let a = |name: &str, default: f32| -> f32 {
        n.attribute(name)
            .and_then(|v| length(v).ok())
            .unwrap_or(default)
    };
    // Four cubic quarter-arcs per ellipse (kappa approximation).
    let ellipse = |cx: f32, cy: f32, rx: f32, ry: f32| -> String {
        let (kx, ky) = (rx * 0.552_284_8, ry * 0.552_284_8);
        format!(
            "M{} {cy}C{} {} {} {} {cx} {}C{} {} {} {} {} {cy}C{} {} {} {} {cx} {}C{} {} {} {} {} {cy}Z",
            cx + rx,
            cx + rx,
            cy + ky,
            cx + kx,
            cy + ry,
            cy + ry,
            cx - kx,
            cy + ry,
            cx - rx,
            cy + ky,
            cx - rx,
            cx - rx,
            cy - ky,
            cx - kx,
            cy - ry,
            cy - ry,
            cx + kx,
            cy - ry,
            cx + rx,
            cy - ky,
            cx + rx,
        )
    };
    let data = match tag {
        "path" => return parse_path(n.attribute("d").unwrap_or("")),
        "rect" => {
            let (x, y, w, h) = (a("x", 0.), a("y", 0.), a("width", 0.), a("height", 0.));
            if w <= 0. || h <= 0. {
                return Ok(Arc::from([]));
            }
            let (rx, ry) = match (n.attribute("rx"), n.attribute("ry")) {
                (None, None) => (0., 0.),
                (Some(_), None) => (a("rx", 0.), a("rx", 0.)),
                (None, Some(_)) => (a("ry", 0.), a("ry", 0.)),
                _ => (a("rx", 0.), a("ry", 0.)),
            };
            let (rx, ry) = (rx.clamp(0., w / 2.), ry.clamp(0., h / 2.));
            if rx == 0. || ry == 0. {
                format!("M{x} {y}h{w}v{h}h{}Z", -w)
            } else {
                format!(
                    "M{} {y}H{}A{rx} {ry} 0 0 1 {} {}V{}A{rx} {ry} 0 0 1 {} {}H{}A{rx} {ry} 0 0 1 {x} {}V{}A{rx} {ry} 0 0 1 {} {y}Z",
                    x + rx,
                    x + w - rx,
                    x + w,
                    y + ry,
                    y + h - ry,
                    x + w - rx,
                    y + h,
                    x + rx,
                    y + h - ry,
                    y + ry,
                    x + rx,
                )
            }
        }
        "circle" => {
            let r = a("r", 0.);
            if r <= 0. {
                return Ok(Arc::from([]));
            }
            ellipse(a("cx", 0.), a("cy", 0.), r, r)
        }
        "ellipse" => {
            let (rx, ry) = (a("rx", 0.), a("ry", 0.));
            if rx <= 0. || ry <= 0. {
                return Ok(Arc::from([]));
            }
            ellipse(a("cx", 0.), a("cy", 0.), rx, ry)
        }
        "line" => format!(
            "M{} {}L{} {}",
            a("x1", 0.),
            a("y1", 0.),
            a("x2", 0.),
            a("y2", 0.)
        ),
        "polygon" | "polyline" => {
            let points: Vec<f64> =
                svgtypes::NumberListParser::from(n.attribute("points").unwrap_or(""))
                    .map_while(std::result::Result::ok)
                    .collect();
            if points.len() < 4 {
                return Ok(Arc::from([]));
            }
            let mut d = String::new();
            for (i, p) in points.as_chunks::<2>().0.iter().enumerate() {
                d.push_str(&format!(
                    "{}{} {}",
                    if i == 0 { "M" } else { "L" },
                    p[0],
                    p[1]
                ));
            }
            if tag == "polygon" {
                d.push('Z');
            }
            d
        }
        _ => return Ok(Arc::from([])),
    };
    parse_path(&data)
}
fn view_box(n: Node<'_, '_>) -> Result<[f64; 4]> {
    let b = if let Some(value) = n.attribute("viewBox") {
        let v: Vec<_> = svgtypes::NumberListParser::from(value)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| refused("view_box", e.to_string()))?;
        if v.len() != 4 {
            return Err(refused("view_box", "requires four values"));
        }
        [v[0], v[1], v[2], v[3]]
    } else {
        [
            0.,
            0.,
            f64::from(
                n.attribute("width")
                    .and_then(|v| length(v).ok())
                    .unwrap_or(300.),
            ),
            f64::from(
                n.attribute("height")
                    .and_then(|v| length(v).ok())
                    .unwrap_or(150.),
            ),
        ]
    };
    if b.iter().any(|v| !v.is_finite()) || b[2] <= 0. || b[3] <= 0. {
        return Err(refused("viewport", "finite positive viewport required"));
    }
    Ok(b)
}
fn viewport_transform(b: [f32; 4], r: [f32; 4], aspect: Option<&str>) -> Result<Affine> {
    if r.iter().any(|v| !v.is_finite()) || r[2] <= 0. || r[3] <= 0. {
        return Err(refused("viewport", "nonpositive viewport"));
    }
    let aspect = aspect.unwrap_or("xMidYMid meet");
    let parts: Vec<_> = aspect.split_whitespace().collect();
    let (sx, sy) = (r[2] / b[2], r[3] / b[3]);
    if parts == ["none"] {
        return Ok(Affine([sx, 0., 0., sy, r[0] - b[0] * sx, r[1] - b[1] * sy]));
    }
    if parts.is_empty()
        || parts.len() > 2
        || !matches!(parts.get(1), None | Some(&"meet") | Some(&"slice"))
    {
        return Err(refused("aspect_ratio", aspect));
    }
    let s = if parts.get(1) == Some(&"slice") {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    let align = parts[0];
    let pair = match align {
        "xMinYMin" => (0., 0.),
        "xMidYMin" => (0.5, 0.),
        "xMaxYMin" => (1., 0.),
        "xMinYMid" => (0., 0.5),
        "xMidYMid" => (0.5, 0.5),
        "xMaxYMid" => (1., 0.5),
        "xMinYMax" => (0., 1.),
        "xMidYMax" => (0.5, 1.),
        "xMaxYMax" => (1., 1.),
        _ => return Err(refused("aspect_ratio", aspect)),
    };
    Ok(Affine([
        s,
        0.,
        0.,
        s,
        r[0] + (r[2] - b[2] * s) * pair.0 - b[0] * s,
        r[1] + (r[3] - b[3] * s) * pair.1 - b[1] * s,
    ]))
}
fn gradient(n: Node<'_, '_>) -> Result<Paint> {
    for (name, expected) in [
        ("gradientUnits", "objectBoundingBox"),
        ("spreadMethod", "pad"),
        ("color-interpolation", "sRGB"),
    ] {
        if n.attribute(name).is_some_and(|v| v != expected) {
            return Err(refused("gradient_option", name));
        }
    }
    let mut stops = Vec::new();
    for child in n.children().filter(Node::is_element) {
        if child.tag_name().name() != "stop" {
            return Err(refused("gradient_stops", "non-stop child"));
        }
        if stops.len() == 256 {
            return Err(refused("gradient_stops", "more than 256 stops"));
        }
        let offset = unit(child.attribute("offset").unwrap_or("0"))?;
        let color = color(child.attribute("stop-color").unwrap_or("black"))?;
        if unit(child.attribute("stop-opacity").unwrap_or("1"))? != 1. || color.0[3] != 1. {
            return Err(refused("gradient_stops", "nonopaque stop"));
        }
        stops.push(ColorStop { offset, color });
    }
    let paint = Paint::LinearGradient {
        start: [attr_number(n, "x1", 0.)?, attr_number(n, "y1", 0.)?],
        end: [attr_number(n, "x2", 1.)?, attr_number(n, "y2", 0.)?],
        stops: stops.into(),
    };
    paint.validate().map_err(|e| refused("gradient_stops", e))?;
    Ok(paint)
}

/// A gradient the retained painter cannot draw exactly (radial, user-space,
/// transformed, translucent stops) still paints: its first stop as a solid.
fn gradient_or_solid(n: Node<'_, '_>) -> Result<Paint> {
    if n.children()
        .filter(|c| c.has_tag_name((SVG, "stop")))
        .count()
        > 256
    {
        return Err(refused("gradient_stops", "more than 256 stops"));
    }
    if n.has_tag_name((SVG, "linearGradient"))
        && let Ok(paint) = gradient(n)
    {
        return Ok(paint);
    }
    let first = n
        .children()
        .find(|c| c.has_tag_name((SVG, "stop")))
        .map(|stop| {
            let style = declarations(stop.attribute("style").unwrap_or("")).unwrap_or_default();
            let value = style
                .get("stop-color")
                .map(String::as_str)
                .or(stop.attribute("stop-color"))
                .unwrap_or("black");
            let opacity = style
                .get("stop-opacity")
                .map(String::as_str)
                .or(stop.attribute("stop-opacity"))
                .and_then(|v| unit(v).ok())
                .unwrap_or(1.);
            color(value)
                .unwrap_or(Color([0., 0., 0., 1.]))
                .alpha(opacity)
        })
        .unwrap_or(Color([0., 0., 0., 1.]));
    Ok(Paint::Solid(first))
}

/// A simple selector: `*`, `tag`, `.class`, `#id` or `tag.class`.
pub(crate) struct Selector {
    tag: Option<String>,
    class: Option<String>,
    id: Option<String>,
}
impl Selector {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() || text.contains([' ', '>', '+', '~', '[', ':']) {
            return None;
        }
        if text == "*" {
            return Some(Self {
                tag: None,
                class: None,
                id: None,
            });
        }
        if let Some(id) = text.strip_prefix('#') {
            return Some(Self {
                tag: None,
                class: None,
                id: Some(id.into()),
            });
        }
        let (tag, class) = match text.split_once('.') {
            Some((tag, class)) => (tag, Some(class.to_owned())),
            None => (text, None),
        };
        Some(Self {
            tag: (!tag.is_empty()).then(|| tag.to_owned()),
            class,
            id: None,
        })
    }
    fn matches(&self, n: Node<'_, '_>) -> bool {
        self.tag.as_deref().is_none_or(|t| n.tag_name().name() == t)
            && self
                .id
                .as_deref()
                .is_none_or(|id| n.attribute("id") == Some(id))
            && self.class.as_deref().is_none_or(|c| {
                n.attribute("class")
                    .is_some_and(|v| v.split_whitespace().any(|k| k == c))
            })
    }
}

/// Rules with simple selectors apply; rules we cannot match are ignored.
fn stylesheet(text: &str) -> Result<Vec<(Vec<Selector>, Style)>> {
    let mut rules = Vec::new();
    let text = strip_comments(text);
    let mut rest = text.as_str();
    while let Some((selectors, after)) = rest.split_once('{') {
        let (body, after) = after
            .split_once('}')
            .ok_or_else(|| refused("style", "malformed rule"))?;
        let selectors: Vec<_> = selectors.split(',').filter_map(Selector::parse).collect();
        if !selectors.is_empty() {
            rules.push((selectors, declarations(body)?));
        }
        rest = after;
    }
    Ok(rules)
}
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once("/*") {
        out.push_str(before);
        rest = after.split_once("*/").map_or("", |(_, a)| a);
    }
    out.push_str(rest);
    out
}

pub fn prepare(svg: &[u8], fonts: Option<&FontSet>) -> std::result::Result<Scene, Vec<Diagnostic>> {
    prepare_bounded(svg, fonts, MAX_PNG_PIXELS, false)
}

/// Stored images use the same grammar, a positive intrinsic viewport under
/// 2048 logical pixels per side and the remainder of the host PNG budget.
pub fn prepare_image(svg: &[u8], remaining_png_pixels: u64) -> Result<Scene> {
    prepare_bounded(
        svg,
        Some(FontSet::bundled()?),
        remaining_png_pixels.min(MAX_PNG_PIXELS),
        true,
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .next()
            .expect("refusal has a reason")
    })
}

fn prepare_bounded(
    svg: &[u8],
    fonts: Option<&FontSet>,
    png_pixels: u64,
    image: bool,
) -> std::result::Result<Scene, Vec<Diagnostic>> {
    let doc = read_svg(svg).map_err(|e| vec![e])?;
    let diagnostics = audit(&doc);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    prepare_inner(&doc, fonts, png_pixels, image).map_err(|e| vec![e])
}
fn read_svg(svg: &[u8]) -> Result<Document<'_>> {
    if svg.len() > MAX_SVG_BYTES {
        return Err(refused("svg_bytes", "SVG exceeds byte ceiling"));
    }
    let text = std::str::from_utf8(svg).map_err(|e| refused("utf8", e.to_string()))?;
    if text.contains("<!ENTITY") {
        return Err(refused("entity", "entity declarations are refused"));
    }
    let doc = Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: (MAX_XML_NODES + 1) as u32,
            ..Default::default()
        },
    )
    .map_err(|e| refused("xml", e.to_string()))?;
    if doc.descendants().skip(1).count() > MAX_XML_NODES {
        return Err(refused("xml_nodes", "XML node ceiling"));
    }
    if doc
        .descendants()
        .filter(Node::is_element)
        .any(|n| n.ancestors().filter(Node::is_element).count() > MAX_DEPTH)
    {
        return Err(refused("depth", "XML nesting ceiling"));
    }
    let root = doc.root_element();
    if root.tag_name().namespace() != Some(SVG) || root.tag_name().name() != "svg" {
        return Err(refused("root", "expected SVG root"));
    }
    Ok(doc)
}
fn prepare_inner(
    doc: &Document<'_>,
    fonts: Option<&FontSet>,
    png_pixels: u64,
    image: bool,
) -> Result<Scene> {
    let root = doc.root_element();
    let has_text = doc.descendants().any(|n| n.has_tag_name((SVG, "text")));
    if let Some(fonts) = fonts {
        // Hand-written SVG carries no identity and renders with the active set;
        // a declared identity that names a different set is still refused.
        if let Some(identity) = root.attribute("data-bboxy-font-set")
            && identity != fonts.identity()
        {
            return Err(refused(
                "font_identity_mismatch",
                "SVG and active FontSet differ",
            ));
        }
    } else if has_text || root.attribute("data-bboxy-font-set").is_some() {
        return Err(refused("font_unavailable", "no active FontSet"));
    }
    let box_ = view_box(root)?;
    // A percentage or unknown unit falls back to the viewBox size.
    let viewport = [
        root.attribute("width")
            .and_then(|v| length(v).ok())
            .unwrap_or(box_[2] as f32),
        root.attribute("height")
            .and_then(|v| length(v).ok())
            .unwrap_or(box_[3] as f32),
    ];
    if viewport.iter().any(|v| *v <= 0.) {
        return Err(refused("viewport", "nonpositive root viewport"));
    }
    if image && viewport.iter().any(|side| *side >= 2048.) {
        return Err(refused(
            "viewport",
            "stored image sides must be under 2048 logical pixels",
        ));
    }
    let mut ids = HashMap::new();
    let mut rules = Vec::new();
    let mut gradients = HashMap::new();
    let mut images = HashMap::new();
    let mut decoded = HashMap::<String, Arc<bbt_render::image::Image>>::new();
    let mut decoded_pixels = 0;
    for n in doc
        .descendants()
        .filter(Node::is_element)
        .filter(|n| !inert(*n))
    {
        if let Some(id) = n.attribute("id").filter(|id| !id.is_empty()) {
            ids.entry(id.into()).or_insert(n.id());
        }
        if n.has_tag_name((SVG, "style")) {
            let value = n.text().unwrap_or("");
            if value.contains('@') || value.contains("url(") {
                return Err(refused("css_import", "font/import/resource CSS is refused"));
            }
            rules.extend(stylesheet(value)?);
        }
        if (n.has_tag_name((SVG, "linearGradient")) || n.has_tag_name((SVG, "radialGradient")))
            && let Some(id) = n.attribute("id")
        {
            gradients.insert(id.into(), gradient_or_solid(n)?);
        }
        if n.has_tag_name((SVG, "image")) {
            let Some(encoded) = href(n)?.strip_prefix("data:image/png;base64,") else {
                continue;
            };
            let encoded: Vec<_> = encoded
                .bytes()
                .filter(|b| !matches!(*b, b' ' | b'\t' | b'\r' | b'\n'))
                .collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| refused("image_base64", e.to_string()))?;
            let digest = format!("{:x}", Sha256::digest(&bytes));
            let image = if let Some(image) = decoded.get(&digest) {
                image.clone()
            } else {
                if decoded.len() == MAX_PNGS {
                    return Err(refused("png_distinct", "too many distinct PNGs"));
                }
                let image = bbt_render::image::decode_png(&bytes, png_pixels - decoded_pixels)
                    .map_err(|e| refused("png", e.to_string()))?;
                decoded_pixels += u64::from(image.width) * u64::from(image.height);
                let image = Arc::new(image);
                decoded.insert(digest, image.clone());
                image
            };
            images.insert(n.id(), image);
        }
    }
    let mut context = Context {
        doc,
        fonts,
        ids,
        rules,
        gradients,
        images,
        stack: vec![],
        commands: 0,
        glyphs: 0,
        expanded: 0,
    };
    let elements = context.lower(root, &Style::new(), Affine::default(), 1)?;
    Ok(Scene {
        elements: elements.into(),
        view_box: box_,
        viewport,
        decoded_pixels,
    })
}
