//! L6 retained Box, Text, Stack and ScrollView. Layout is pure; paint is ordered
//! draw-list/material data. The host supplies fonts, a viewport and scroll offsets.
use crate::{Context, Length, Material, ResolvedStyle, ResolvedText, Style};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList, Prim, TextStyle, TypeScale},
    layout::{self, Align, Axis, Insets, Rect, Size, TextMeasure, TextStyle as MeasureStyle},
};

pub trait Measure: TextMeasure {
    fn font_metrics(&self, style: &MeasureStyle) -> [f32; 2];
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    Auto,
    Fixed(f32),
    Fill,
    Flex(f32),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}
#[derive(Debug, Clone)]
pub struct TextRun {
    pub value: String,
    pub style: crate::style::Text,
}
#[derive(Debug, Clone)]
pub struct Text {
    pub runs: Vec<TextRun>,
    pub wrap: bool,
    pub ellipsis: bool,
    pub selectable: bool,
}
impl Text {
    pub fn new(value: impl Into<String>, style: crate::style::Text) -> Self {
        Self {
            runs: vec![TextRun {
                value: value.into(),
                style,
            }],
            wrap: true,
            ellipsis: false,
            selectable: true,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Box {
    pub style: Style,
    pub children: Vec<Item>,
    pub direction: Axis,
    pub gap: Length,
    pub align: Alignment,
    pub clip: bool,
}
impl Box {
    pub fn new(style: Style, children: Vec<Item>) -> Self {
        Self {
            style,
            children,
            direction: Axis::Row,
            gap: Length::Spacing(4),
            align: Alignment::Center,
            clip: false,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Stack {
    pub direction: Axis,
    pub gap: Length,
    pub align: Alignment,
    pub children: Vec<Item>,
}
#[derive(Debug, Clone)]
pub struct ScrollView {
    pub viewport: [f32; 2],
    pub offset: [f32; 2],
    pub content_width: Option<f32>,
    pub content: std::boxed::Box<Element>,
}
#[derive(Debug, Clone)]
pub enum Element {
    Box(Box),
    Text(Text),
    Stack(Stack),
    ScrollView(ScrollView),
    Spacer,
}
#[derive(Debug, Clone)]
pub struct Item {
    pub element: Element,
    pub extent: Extent,
}
impl From<Element> for Item {
    fn from(element: Element) -> Self {
        Self {
            element,
            extent: Extent::Auto,
        }
    }
}
#[derive(Debug, Clone)]
pub struct PositionedRun {
    pub text: String,
    pub style: ResolvedText,
    pub rect: Rect,
    pub line_height: f32,
}
#[derive(Debug, Clone)]
pub enum LayoutKind {
    Box {
        style: ResolvedStyle,
        clip: bool,
    },
    Text {
        runs: Vec<PositionedRun>,
        selectable: bool,
        ellipsis: bool,
    },
    Stack,
    ScrollView {
        content: [f32; 2],
        offset: [f32; 2],
    },
    Spacer,
}
#[derive(Debug, Clone)]
pub struct Layout {
    pub rect: Rect,
    pub baseline: f32,
    pub kind: LayoutKind,
    pub children: Vec<Layout>,
}

fn font(s: &ResolvedText) -> MeasureStyle {
    MeasureStyle {
        size: s.size,
        weight: s.weight,
        mono: s.mono,
        tracking_em: s.tracking_em,
    }
}
fn resolve_text(s: crate::style::Text, ctx: Context) -> crate::style::Result<ResolvedText> {
    let mut st = Style::new(crate::Preset::Card);
    st.overrides.text = Some(s);
    Ok(st.resolve(ctx)?.text)
}
fn line_height(s: &ResolvedText) -> f32 {
    // CSS line boxes use 1/64 px layout units, truncating computed line-height.
    (s.size * s.line_height * 64.).floor() / 64.
}
fn baseline(s: &ResolvedText, m: &dyn Measure) -> f32 {
    let [a, d] = m.font_metrics(&font(s)).map(f32::round);
    // Integer font ascent/descent and floor-distributed half-leading match the
    // web inline-baseline model; font shaping widths remain renderer-owned.
    a + ((line_height(s) - a - d) * 0.5).floor()
}
fn text_lines(
    text: &Text,
    width: f32,
    ctx: Context,
    m: &dyn Measure,
) -> crate::style::Result<(Vec<PositionedRun>, [f32; 2], f32)> {
    let mut placed: Vec<PositionedRun> = Vec::new();
    let (mut x, mut y, mut above, mut below, mut widest) = (0_f32, 0_f32, 0_f32, 0_f32, 0_f32);
    let (mut start, mut first) = (0, None);
    let finish_line = |placed: &mut [PositionedRun], start: usize, y: f32, above: f32| {
        for run in &mut placed[start..] {
            run.rect.y = y + above - baseline(&run.style, m);
        }
    };
    for run in &text.runs {
        let s = resolve_text(run.style, ctx)?;
        let line = line_height(&s);
        let value = if s.uppercase {
            run.value.to_uppercase()
        } else {
            run.value.clone()
        };
        let pieces: Vec<&str> = if text.wrap {
            value.split_inclusive(char::is_whitespace).collect()
        } else {
            value.split_inclusive('\n').collect()
        };
        for piece in pieces {
            let newline = piece.ends_with('\n');
            let value = piece.trim_end_matches(['\n', '\r']);
            let w = m.width(value, &font(&s));
            if text.wrap && x > 0. && x + w > width {
                finish_line(&mut placed, start, y, above);
                first.get_or_insert(above);
                widest = widest.max(x);
                y += above + below;
                x = 0.;
                above = 0.;
                below = 0.;
                start = placed.len();
            }
            let value = if x == 0. {
                value.trim_start_matches(' ')
            } else {
                value
            };
            let w = m.width(value, &font(&s));
            let base = baseline(&s, m);
            above = above.max(base);
            below = below.max(line - base);
            placed.push(PositionedRun {
                text: value.into(),
                style: s.clone(),
                rect: Rect::new(x, y, w, line),
                line_height: line,
            });
            x += w;
            if newline {
                finish_line(&mut placed, start, y, above);
                first.get_or_insert(above);
                widest = widest.max(x);
                y += above + below;
                x = 0.;
                above = 0.;
                below = 0.;
                start = placed.len();
            }
        }
    }
    finish_line(&mut placed, start, y, above);
    widest = widest.max(x);
    Ok((placed, [widest, y + above + below], first.unwrap_or(above)))
}
type Intrinsic = ([f32; 2], f32);
fn child_measure(
    children: &[Item],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    width: f32,
    ctx: Context,
    m: &dyn Measure,
) -> crate::style::Result<([f32; 2], Vec<Intrinsic>)> {
    let mut data = children
        .iter()
        .map(|c| c.element.natural(width, ctx, m))
        .collect::<crate::style::Result<Vec<_>>>()?;
    if axis == Axis::Row && width.is_finite() {
        let rects = allocate_children(
            children,
            &data,
            axis,
            gap,
            alignment,
            Rect::new(0., 0., width, 0.),
            ctx,
            m,
        )?;
        // A shrinkable Text may wrap after its main-axis width is assigned.
        // Re-measure cross size at that width before sizing/alignment of the row.
        for ((child, (size, base)), r) in children.iter().zip(&mut data).zip(rects) {
            if r.w < size[0] {
                let (wrapped, baseline) = child.element.natural(r.w, ctx, m)?;
                size[1] = wrapped[1];
                *base = baseline;
            }
        }
    }
    let gaps = gap * children.len().saturating_sub(1) as f32;
    let size = if axis == Axis::Row {
        let h = if alignment == Alignment::Baseline {
            data.iter().map(|(_, b)| *b).fold(0., f32::max)
                + data.iter().map(|(s, b)| s[1] - b).fold(0., f32::max)
        } else {
            data.iter().map(|(s, _)| s[1]).fold(0., f32::max)
        };
        [data.iter().map(|(s, _)| s[0]).sum::<f32>() + gaps, h]
    } else {
        [
            data.iter().map(|(s, _)| s[0]).fold(0., f32::max),
            data.iter().map(|(s, _)| s[1]).sum::<f32>() + gaps,
        ]
    };
    Ok((size, data))
}
fn inset(s: &ResolvedStyle) -> Insets {
    let b = s
        .border
        .as_ref()
        .filter(|b| !b.outside)
        .map_or(0., |b| b.width);
    Insets {
        top: s.padding[0] + b,
        right: s.padding[1] + b,
        bottom: s.padding[2] + b,
        left: s.padding[3] + b,
    }
}
impl Element {
    pub fn natural(
        &self,
        width: f32,
        ctx: Context,
        m: &dyn Measure,
    ) -> crate::style::Result<([f32; 2], f32)> {
        match self {
            Self::Text(t) => {
                let (_, size, base) = text_lines(t, width, ctx, m)?;
                Ok((size, base))
            }
            Self::Spacer => Ok(([0., 0.], 0.)),
            Self::ScrollView(s) => Ok((s.viewport, s.viewport[1])),
            Self::Stack(s) => {
                let (size, data) = child_measure(
                    &s.children,
                    s.direction,
                    s.gap.resolve(ctx.density)?,
                    s.align,
                    width,
                    ctx,
                    m,
                )?;
                let b = if s.direction == Axis::Row && s.align == Alignment::Baseline {
                    data.iter().map(|(_, b)| *b).fold(0., f32::max)
                } else {
                    data.first().map_or(size[1], |(_, b)| *b)
                };
                Ok((size, b))
            }
            Self::Box(b) => {
                let s = b.style.resolve(ctx)?;
                let p = inset(&s);
                let (content, data) = child_measure(
                    &b.children,
                    b.direction,
                    b.gap.resolve(ctx.density)?,
                    b.align,
                    (width - p.left - p.right).max(0.),
                    ctx,
                    m,
                )?;
                let h = s.height.unwrap_or(content[1] + p.top + p.bottom);
                Ok((
                    [content[0] + p.left + p.right, h],
                    p.top + data.first().map_or(0., |(_, b)| *b),
                ))
            }
        }
    }
    /// Root width is supplied by the host; its height comes from native content.
    pub fn layout(
        &self,
        bounds: Rect,
        ctx: Context,
        m: &dyn Measure,
    ) -> crate::style::Result<Layout> {
        let (size, _) = self.natural(bounds.w, ctx, m)?;
        self.layout_in(Rect::new(bounds.x, bounds.y, bounds.w, size[1]), ctx, m)
    }
    fn layout_in(&self, rect: Rect, ctx: Context, m: &dyn Measure) -> crate::style::Result<Layout> {
        let (_, base) = self.natural(rect.w, ctx, m)?;
        let (kind, children) = match self {
            Self::Text(t) => {
                let (mut runs, _, _) = text_lines(t, rect.w, ctx, m)?;
                for run in &mut runs {
                    run.rect.x += rect.x;
                    run.rect.y += rect.y;
                }
                (
                    LayoutKind::Text {
                        runs,
                        selectable: t.selectable,
                        ellipsis: t.ellipsis,
                    },
                    vec![],
                )
            }
            Self::Spacer => (LayoutKind::Spacer, vec![]),
            Self::Stack(s) => (
                LayoutKind::Stack,
                layout_children(
                    &s.children,
                    s.direction,
                    s.gap.resolve(ctx.density)?,
                    s.align,
                    rect,
                    ctx,
                    m,
                )?,
            ),
            Self::Box(b) => {
                let style = b.style.resolve(ctx)?;
                let children = layout_children(
                    &b.children,
                    b.direction,
                    b.gap.resolve(ctx.density)?,
                    b.align,
                    rect.inset(inset(&style)),
                    ctx,
                    m,
                )?;
                (
                    LayoutKind::Box {
                        style,
                        clip: b.clip,
                    },
                    children,
                )
            }
            Self::ScrollView(s) => {
                let content_w = s.content_width.unwrap_or(rect.w).max(rect.w);
                let (natural, _) = s.content.natural(content_w, ctx, m)?;
                let content = [content_w, natural[1].max(rect.h)];
                let offset = [
                    s.offset[0].clamp(0., (content[0] - rect.w).max(0.)),
                    s.offset[1].clamp(0., (content[1] - rect.h).max(0.)),
                ];
                let child = s.content.layout_in(
                    Rect::new(
                        rect.x - offset[0],
                        rect.y - offset[1],
                        content[0],
                        content[1],
                    ),
                    ctx,
                    m,
                )?;
                (LayoutKind::ScrollView { content, offset }, vec![child])
            }
        };
        Ok(Layout {
            rect,
            baseline: base,
            kind,
            children,
        })
    }
}
fn layout_children(
    children: &[Item],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    r: Rect,
    ctx: Context,
    m: &dyn Measure,
) -> crate::style::Result<Vec<Layout>> {
    let (_, data) = child_measure(children, axis, gap, alignment, r.w, ctx, m)?;
    let mut rects = allocate_children(children, &data, axis, gap, alignment, r, ctx, m)?;
    if axis == Axis::Row && alignment == Alignment::Baseline {
        let max = data.iter().map(|(_, b)| *b).fold(0., f32::max);
        for (rect, (_, b)) in rects.iter_mut().zip(&data) {
            rect.y += max - b;
        }
    }
    children
        .iter()
        .zip(rects)
        .map(|(c, r)| c.element.layout_in(r, ctx, m))
        .collect()
}

#[allow(clippy::too_many_arguments)] // Shared solver adapter; keeps the public tree free of renderer state.
fn allocate_children(
    children: &[Item],
    data: &[([f32; 2], f32)],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    r: Rect,
    ctx: Context,
    m: &dyn Measure,
) -> crate::style::Result<Vec<Rect>> {
    let axis_i = if axis == Axis::Row { 0 } else { 1 };
    let align = match alignment {
        Alignment::Center => Align::Center,
        Alignment::End => Align::End,
        Alignment::Stretch => Align::Stretch,
        _ => Align::Start,
    };
    let mut model = layout::Box_::new(axis, ctx.density, vec![]);
    model.gap = gap;
    model.align = align;
    for (c, (size, _)) in children.iter().zip(data) {
        let text = if let Element::Text(t) = &c.element
            && t.runs.len() == 1
        {
            if let Some(run) = t.runs.first() {
                let style = resolve_text(run.style, ctx)?;
                let value = t.runs.iter().map(|r| r.value.as_str()).collect::<String>();
                Some((
                    if style.uppercase {
                        value.to_uppercase()
                    } else {
                        value
                    },
                    font(&style),
                ))
            } else {
                None
            }
        } else {
            None
        };
        let extent = match c.extent {
            Extent::Fixed(v) => Size::Fixed(v),
            Extent::Fill => Size::Fill,
            Extent::Flex(v) => Size::Flex(v),
            Extent::Auto => {
                if matches!(c.element, Element::Spacer) {
                    Size::Fill
                } else if axis == Axis::Row && text.is_some() {
                    Size::Text
                } else {
                    Size::Fixed(size[axis_i])
                }
            }
        };
        model.children.push((
            extent,
            layout::Child {
                text,
                cross: Some(size[1 - axis_i]),
            },
        ));
    }
    Ok(layout::solve_box(&model, r, m))
}

#[derive(Debug, Clone, Copy)]
pub struct Clip {
    pub rect: Rect,
    pub radii: [f32; 4],
}
#[derive(Debug, Clone)]
pub enum Paint {
    Draw(DrawList),
    Underlay(DrawList),
    Material {
        rect: Rect,
        radii: [f32; 4],
        kind: Material,
        color: Color,
        clips: Vec<Clip>,
    },
}
fn list_with_clips(clips: &[Clip]) -> DrawList {
    let mut list = DrawList::default();
    for c in clips {
        list.clip(c.rect, c.radii);
    }
    list
}
fn finish(mut list: DrawList, clips: &[Clip], out: &mut Vec<Paint>) {
    for _ in clips {
        list.unclip();
    }
    out.push(Paint::Draw(list));
}
fn color(c: [f32; 4], draw: DrawCtx, opacity: f32) -> Color {
    draw.color(Color(c)).alpha(opacity)
}
pub fn paint(layout: &Layout, ctx: Context) -> Vec<Paint> {
    let mut out = Vec::new();
    paint_node(
        layout,
        DrawCtx::new(ctx.theme, ctx.density),
        1.,
        &[],
        &mut out,
    );
    out
}
fn paint_node(
    node: &Layout,
    mut draw: DrawCtx,
    mut opacity: f32,
    clips: &[Clip],
    out: &mut Vec<Paint>,
) {
    let mut children_clips = clips.to_vec();
    let mut after = None;
    match &node.kind {
        LayoutKind::Box { style: s, clip } => {
            if let Some(dim) = s.dim {
                draw.dim = Some(bbt_ui_model::draw::Dim {
                    saturate: dim.saturate,
                    opacity: dim.opacity,
                    wash: dim.wash,
                });
            }
            opacity *= s.opacity;
            let mut list = list_with_clips(clips);
            if let Some(shadow) = &s.shadow {
                list.push(Prim::Rect {
                    rect: node.rect,
                    radii: s.radii,
                    fill: Color::TRANSPARENT,
                    stroke: None,
                    inner_highlight: None,
                    shadow: Some((
                        shadow.offset[1],
                        shadow.blur,
                        color(shadow.color, draw, opacity),
                    )),
                });
            }
            // Shadow is an underlay; the material receives the pre-shadow backdrop.
            // Its draw record stays separate from the sampled material record.
            if !list.primitives.is_empty() {
                let mut record = Vec::new();
                finish(list, clips, &mut record);
                if let Some(Paint::Draw(list)) = record.pop() {
                    out.push(Paint::Underlay(list));
                }
            }
            if matches!(s.material, Material::Gloss | Material::Frosted) {
                out.push(Paint::Material {
                    rect: node.rect,
                    radii: s.radii,
                    kind: s.material,
                    color: color(s.fill, draw, opacity),
                    clips: clips.to_vec(),
                });
            }
            let mut body = list_with_clips(clips);
            let fill = if s.material == Material::Gloss {
                Color::TRANSPARENT
            } else {
                color(s.fill, draw, opacity)
            };
            let (rect, radii, stroke) = if let Some(border) = &s.border {
                if border.outside {
                    (
                        Rect::new(
                            node.rect.x - border.width,
                            node.rect.y - border.width,
                            node.rect.w + 2. * border.width,
                            node.rect.h + 2. * border.width,
                        ),
                        s.radii.map(|r| r + border.width),
                        Some((border.width, color(border.color, draw, opacity))),
                    )
                } else {
                    (
                        node.rect,
                        s.radii,
                        Some((border.width, color(border.color, draw, opacity))),
                    )
                }
            } else {
                (node.rect, s.radii, None)
            };
            // Outside ring and fill have different bounds.
            if s.border.as_ref().is_some_and(|b| b.outside) {
                body.push(Prim::Rect {
                    rect: node.rect,
                    radii: s.radii,
                    fill,
                    stroke: None,
                    inner_highlight: None,
                    shadow: None,
                });
                body.push(Prim::Rect {
                    rect,
                    radii,
                    fill: Color::TRANSPARENT,
                    stroke,
                    inner_highlight: None,
                    shadow: None,
                });
            } else {
                body.push(Prim::Rect {
                    rect,
                    radii,
                    fill,
                    stroke,
                    inner_highlight: s.inner_highlight.map(|c| color(c, draw, opacity)),
                    shadow: None,
                });
            }
            if let Some(ring) = &s.focus_ring {
                body.push(Prim::Rect {
                    rect: Rect::new(
                        node.rect.x - ring.width,
                        node.rect.y - ring.width,
                        node.rect.w + 2. * ring.width,
                        node.rect.h + 2. * ring.width,
                    ),
                    radii: s.radii.map(|v| v + ring.width),
                    fill: Color::TRANSPARENT,
                    stroke: Some((ring.width, color(ring.color, draw, opacity))),
                    inner_highlight: None,
                    shadow: None,
                });
            }
            finish(body, clips, out);
            if *clip {
                children_clips.push(Clip {
                    rect: node.rect,
                    radii: s.radii,
                });
            }
            if s.material == Material::Glow {
                after = Some(Paint::Material {
                    rect: node.rect,
                    radii: s.radii,
                    kind: s.material,
                    color: color(s.fill, draw, opacity),
                    clips: clips.to_vec(),
                });
            }
        }
        LayoutKind::Text { runs, ellipsis, .. } => {
            let mut list = list_with_clips(clips);
            for run in runs {
                list.push(Prim::Text {
                    origin: [
                        run.rect.x,
                        run.rect.y + (run.line_height - run.style.size * 1.4) * 0.5,
                    ],
                    text: run.text.clone().into(),
                    style: TextStyle {
                        scale: TypeScale::Custom(run.style.size),
                        weight: run.style.weight,
                        mono: run.style.mono,
                        tracking_em: run.style.tracking_em,
                        uppercase: false,
                    },
                    max_width: ellipsis.then_some(node.rect.right() - run.rect.x),
                    color: color(run.style.color, draw, opacity),
                });
            }
            finish(list, clips, out);
        }
        LayoutKind::ScrollView { .. } => children_clips.push(Clip {
            rect: node.rect,
            radii: [0.; 4],
        }),
        _ => {}
    }
    for child in &node.children {
        paint_node(child, draw, opacity, &children_clips, out);
    }
    if let Some(after) = after {
        out.push(after);
    }
}
