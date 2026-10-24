//! L6 retained Box, Text, Stack and ScrollView. Layout is pure; paint is ordered
//! draw-list/material data. The host supplies fonts, a viewport and scroll offsets.
use crate::{Context, Length, Material, ResolvedStyle, ResolvedText, Style};
pub use bbt_ui_model::draw::{LineCap, LineJoin, LinearAlphaMask, PathCommand, PathStroke};
use bbt_ui_model::{
    draw::{Color, DrawCtx, DrawList, Prim, TextStyle, TypeScale},
    layout::{self, Align, Axis, Insets, Rect, Size, TextMeasure, TextStyle as MeasureStyle},
};

/// A retained vector, optionally filled before stroking. Viewport size participates in layout;
/// the stroke may overflow it, like the lab's public UI.Path constructor.
#[derive(Debug, Clone)]
pub struct Path {
    pub size: [f32; 2],
    pub points: std::sync::Arc<[PathCommand]>,
    pub stroke: PathStroke,
    pub paint: crate::Paint,
    pub fill: Option<crate::Paint>,
    pub opacity: f32,
}

/// Filled axis-aligned rects in a caller-sized slot, laid out as one node: a chart's
/// bars, for instance. They paint as the same Rect primitives a filled Box emits, so
/// the renderer snaps their edges exactly as it snaps boxes. Rects are slot-relative.
#[derive(Debug, Clone)]
pub struct Rects {
    pub size: [f32; 2],
    pub rects: std::sync::Arc<[(Rect, crate::Paint)]>,
    /// Repeat the tile along this axis at a logical-pixel period, clipped to the
    /// assigned slot. None paints the supplied rects once.
    pub repeat: Option<(Axis, f32)>,
}

pub trait Measure: TextMeasure {
    fn font_metrics(&self, style: &MeasureStyle) -> [f32; 2];
    /// Logical x-height for CSS inline-middle alignment. Hosts with a font
    /// backend override the em-based fallback used by simple test measures.
    fn x_height(&self, style: &MeasureStyle) -> f32 {
        style.size * 0.5
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    Auto,
    /// CSS auto basis/min-content minimum, shrinking without growth.
    MinContent,
    Fixed(f32),
    Fill,
    Flex(f32),
    /// Auto basis with CSS-style shrinking and optional growth/min/max bounds.
    Content {
        grow: f32,
        shrink: f32,
        min: f32,
        max: Option<f32>,
        max_fraction: Option<f32>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    SpaceBetween,
}
#[derive(Debug, Clone)]
pub struct TextRun {
    pub value: String,
    pub style: crate::style::Text,
}
/// An optional glyph-mask shadow; it contributes ink, never layout size.
#[derive(Debug, Clone, Copy)]
pub struct TextShadow {
    pub color: crate::Paint,
    pub blur_radius: f32,
    pub offset: [f32; 2],
}
impl TextShadow {
    fn resolve(self, ctx: Context) -> crate::style::Result<bbt_ui_model::draw::TextShadow> {
        let shadow = bbt_ui_model::draw::TextShadow {
            color: Color(self.color.resolve(ctx)?),
            blur_radius: self.blur_radius,
            offset: self.offset,
        };
        shadow.validate()?;
        Ok(shadow)
    }
}
#[derive(Debug, Clone)]
pub struct Text {
    pub runs: Vec<TextRun>,
    pub wrap: bool,
    /// Keep the inline max-content measure as a flex basis before wrapping.
    pub max_content_basis: bool,
    pub ellipsis: bool,
    /// Clamp wrapped content, ending the last visible line with an ellipsis.
    pub max_lines: Option<usize>,
    pub selectable: bool,
    /// Single inline span: expose its font box, retaining its line box for flow.
    pub inline: bool,
    /// Per-line placement within the text's own assigned box.
    pub align: Alignment,
    pub shadow: Option<TextShadow>,
}
impl Text {
    pub fn new(value: impl Into<String>, style: crate::style::Text) -> Self {
        Self {
            runs: vec![TextRun {
                value: value.into(),
                style,
            }],
            wrap: true,
            max_content_basis: false,
            ellipsis: false,
            max_lines: None,
            selectable: true,
            inline: false,
            align: Alignment::Start,
            shadow: None,
        }
    }
}

/// Public field indices use UTF-16 code units, as in the lab constructor.
/// A boundary inside a surrogate pair is invalid rather than silently shifted.
pub fn utf16_byte_index(value: &str, index: usize) -> Option<usize> {
    let mut units = 0;
    for (byte, ch) in value.char_indices() {
        if units == index {
            return Some(byte);
        }
        units += ch.len_utf16();
    }
    (units == index).then_some(value.len())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Edit {
    pub caret: Option<usize>,
    pub select: Option<[usize; 2]>,
    pub preedit: Option<Preedit>,
    #[serde(default)]
    pub blink: bool,
    /// Host-owned replacement range while the IME has not committed. It is not
    /// a lab input and never modifies the field's committed value.
    #[serde(skip)]
    pub replacement: Option<[usize; 2]>,
    #[serde(skip)]
    pub hide_caret: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Preedit {
    pub text: String,
    pub caret: Option<usize>,
}

/// A single-line field's retained presentation. Committed text determines its
/// intrinsic size; marked IME text never changes the query or its filter.
#[derive(Debug, Clone)]
pub struct EditingText {
    pub value: String,
    pub placeholder: String,
    pub style: crate::style::Text,
    pub placeholder_paint: crate::Paint,
    pub edit: Edit,
    /// Semantic names for hit testing and geometry evidence, not positions.
    pub name: String,
}
impl EditingText {
    fn boundaries(&self) -> crate::style::Result<(usize, usize)> {
        let [a, b] = self
            .edit
            .replacement
            .or(self.edit.select)
            .unwrap_or_else(|| {
                let at = self.edit.caret.unwrap_or(self.value.encode_utf16().count());
                [at, at]
            });
        if a > b {
            return Err("field selection is reversed");
        }
        let a = utf16_byte_index(&self.value, a).ok_or("invalid field UTF-16 boundary")?;
        let b = utf16_byte_index(&self.value, b).ok_or("invalid field UTF-16 boundary")?;
        if let Some(p) = &self.edit.preedit {
            utf16_byte_index(&p.text, p.caret.unwrap_or(p.text.encode_utf16().count()))
                .ok_or("invalid preedit UTF-16 boundary")?;
        }
        Ok((a, b))
    }

    fn natural(&self, ctx: Context, m: &dyn Measure) -> crate::style::Result<Intrinsic> {
        self.boundaries()?;
        let s = resolve_text(self.style, ctx)?;
        let value = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        Ok((
            [m.width(value, &font(&s)), line_height(&s)],
            baseline(&s, m),
        ))
    }

    fn layout(&self, rect: Rect, ctx: Context, m: &dyn Measure) -> crate::style::Result<Layout> {
        let (a, b) = self.boundaries()?;
        let s = resolve_text(self.style, ctx)?;
        let h = line_height(&s);
        let base = baseline(&s, m);
        let top = rect.y + (rect.h - h) * 0.5;
        let mut x = rect.x;
        let mut children = Vec::new();
        let named = |part: &str, child: Layout| Layout {
            rect: child.rect,
            baseline: child.baseline,
            kind: LayoutKind::Named(format!("{}/{part}", self.name)),
            children: vec![child],
        };
        let label = |value: &str, x: f32, style: ResolvedText| {
            let r = Rect::new(x, top, m.width(value, &font(&style)), h);
            Layout {
                rect: r,
                baseline: base,
                kind: LayoutKind::Text {
                    runs: vec![PositionedRun {
                        text: value.into(),
                        style,
                        rect: r,
                        line_height: h,
                        baseline: top + base,
                    }],
                    selectable: false,
                    ellipsis: false,
                    shadow: None,
                },
                children: vec![],
            }
        };
        let mark = |part: &str, r: Rect, paint: crate::Paint, radius: f32| {
            let mut style = Style::new(crate::Preset::Card);
            style.overrides.fill = Some(paint);
            style.overrides.border = Some(crate::Border {
                width: Length::Px(0.),
                paint: crate::Paint::solid(crate::Color::Transparent),
                outside: false,
            });
            style.overrides.radii = Some([Length::Px(radius); 4]);
            Ok::<_, &'static str>(named(
                part,
                Layout {
                    rect: r,
                    baseline: r.h,
                    kind: LayoutKind::Box {
                        style: std::boxed::Box::new(style.resolve(ctx)?),
                        clip: false,
                        group: false,
                        mask: None,
                    },
                    children: vec![],
                },
            ))
        };
        let prefix = label(&self.value[..a], x, s.clone());
        x += prefix.rect.w;
        children.push(named("prefix", prefix));
        if self.edit.select.is_some() && self.edit.preedit.is_none() {
            let selected = label(&self.value[a..b], x, s.clone());
            children.push(mark(
                "selection",
                selected.rect,
                crate::Paint {
                    color: crate::Color::Accent,
                    alpha: crate::Alpha::Value(0.35),
                },
                2.,
            )?);
            x += selected.rect.w;
            children.push(named("selected", selected));
        } else {
            let mut caret_x = x;
            if let Some(p) = &self.edit.preedit {
                let at =
                    utf16_byte_index(&p.text, p.caret.unwrap_or(p.text.encode_utf16().count()))
                        .unwrap();
                let before = label(&p.text[..at], x, s.clone());
                caret_x += before.rect.w;
                children.push(named("preedit-prefix", before));
                let after = label(&p.text[at..], caret_x, s.clone());
                x = after.rect.right();
                children.push(named("preedit-suffix", after));
                let width = x - children[0].rect.right();
                children.push(mark(
                    "underline",
                    Rect::new(x - width, top + base + 3., width, 1.),
                    self.style.paint,
                    0.,
                )?);
            }
            let caret_h = (s.size * 1.2 * 64.).floor() / 64.;
            let caret_y = if self.edit.preedit.is_some() {
                top + base - caret_h + s.size * 0.2
            } else {
                top + (h - caret_h) * 0.5
            };
            if !self.edit.hide_caret {
                children.push(mark(
                    "caret",
                    Rect::new(caret_x - 1., caret_y, 2., caret_h),
                    crate::Paint::solid(crate::Color::Accent),
                    1.,
                )?);
            }
        }
        children.push(named("suffix", label(&self.value[b..], x, s.clone())));
        if self.value.is_empty() && self.edit.preedit.is_none() && self.edit.select.is_none() {
            let mut placeholder = s;
            placeholder.color = self.placeholder_paint.resolve(ctx)?;
            children.push(named(
                "placeholder",
                label(&self.placeholder, x, placeholder),
            ));
        }
        Ok(Layout {
            rect,
            baseline: top - rect.y + base,
            kind: LayoutKind::Stack,
            children,
        })
    }
}
#[derive(Debug, Clone)]
pub struct Box {
    pub style: std::boxed::Box<Style>,
    pub children: Vec<Item>,
    pub direction: Axis,
    pub gap: Length,
    pub align: Alignment,
    pub clip: bool,
    /// Force an isolated layer even at full opacity. Nonidentity opacity/dim
    /// always isolate, as CSS does; the default needs no extra layer.
    pub group: bool,
    /// Group-local alpha ramp, applied to the completed isolated layer.
    pub mask: Option<LinearAlphaMask>,
    /// Visual transform only; the unscaled box still occupies its flow slot.
    pub scale: f32,
    /// Inline formatting includes the Style text's inherited line-height strut.
    pub inline: bool,
    /// Preserve natural cross size in a short fixed row (CSS flex overflow).
    pub cross_overflow: bool,
}
impl Box {
    pub fn new(style: Style, children: Vec<Item>) -> Self {
        Self {
            style: std::boxed::Box::new(style),
            children,
            direction: Axis::Row,
            gap: Length::Spacing(4),
            align: Alignment::Center,
            clip: false,
            group: false,
            mask: None,
            scale: 1.,
            inline: false,
            cross_overflow: false,
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
/// A wrapping row of existing children, with no grid or positioning API.
#[derive(Debug, Clone)]
pub struct Flow {
    pub children: Vec<Item>,
    pub wrap: bool,
    pub justify: Justify,
    pub align: Alignment,
    pub row_gap: Length,
    pub column_gap: Length,
}
impl Flow {
    pub fn new(children: Vec<Item>) -> Self {
        Self {
            children,
            wrap: true,
            justify: Justify::Start,
            align: Alignment::Stretch,
            row_gap: Length::Spacing(4),
            column_gap: Length::Spacing(4),
        }
    }
}
#[derive(Debug, Clone)]
pub struct ScrollView {
    pub viewport: [f32; 2],
    pub offset: [f32; 2],
    pub content_width: Option<f32>,
    pub content: std::boxed::Box<Element>,
}
/// A caller-owned scroll snapshot; it retains identities, never measurements.
#[derive(Debug, Clone)]
pub struct ScrollAnchor {
    key: String,
    within: f32,
    order: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Overlay {
    pub content: std::boxed::Box<Element>,
    pub style: std::boxed::Box<Style>,
    pub min_height: Length,
    pub horizontal: Alignment,
    pub vertical: Alignment,
    pub clip: bool,
}
impl Overlay {
    pub fn new(content: Element) -> Self {
        let mut style = Style::new(crate::Preset::Card);
        style.overrides.material = Some(Material::Frosted);
        style.overrides.blur = Some(crate::Blur {
            radius: Length::Px(bbt_ui_model::STYLE.material.backdrop_blur_px as f32),
            saturation: 1.,
        });
        style.overrides.fill = Some(crate::Paint {
            color: crate::Color::Background,
            alpha: crate::Alpha::Value(bbt_ui_model::STYLE.material.backdrop_dim_alpha as f32),
        });
        style.overrides.border = Some(crate::Border {
            width: Length::Px(0.),
            paint: crate::Paint::solid(crate::Color::Transparent),
            outside: false,
        });
        style.overrides.padding = Some([Length::Px(0.); 4]);
        Self {
            content: std::boxed::Box::new(content),
            style: std::boxed::Box::new(style),
            min_height: Length::Px(220.),
            horizontal: Alignment::Center,
            vertical: Alignment::Center,
            clip: true,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Element {
    /// A producer's fixed-height declaration. Transparent in ordinary layout;
    /// ScrollView can omit offscreen items without measuring their contents.
    FixedHeight {
        height: f32,
        paint_outset: f32,
        key: String,
        content: std::boxed::Box<Element>,
    },
    /// CSS sRGB saturation/opacity over retained colour records.
    Filter {
        content: std::boxed::Box<Element>,
        saturation: f32,
        opacity: f32,
    },
    /// CSS vertical-align: middle relative to the parent font x-height.
    InlineMiddle {
        content: std::boxed::Box<Element>,
        parent: crate::style::Text,
    },
    Baseline {
        content: std::boxed::Box<Element>,
        offset: f32,
    },
    Named {
        name: String,
        content: std::boxed::Box<Element>,
    },
    Layers {
        base: std::boxed::Box<Element>,
        overlays: Vec<Placed>,
    },
    Glyph {
        kind_colors: crate::atoms::KindColors,
        shadow: Option<TextShadow>,
        activity: bbt_ui_model::Activity,
        grade: bbt_ui_model::Grade,
        color: crate::Paint,
        size: f32,
        scale: f32,
    },
    Box(Box),
    Text(Text),
    EditingText(EditingText),
    Stack(Stack),
    Flow(Flow),
    ScrollView(ScrollView),
    Overlay(Overlay),
    Spacer,
    Icon {
        name: String,
        paint: crate::Paint,
        size: Length,
    },
    /// An untinted atlas asset; the host resolves the key, never a file path.
    Image {
        asset: String,
        size: Length,
        radius: Length,
    },
    Path(Path),
    Rects(Rects),
}
#[derive(Debug, Clone)]
pub struct Placed {
    pub content: Element,
    pub left: f32,
    pub bottom: f32,
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
    /// Absolute logical y coordinate, shared by all runs on the same line.
    pub baseline: f32,
}
#[derive(Debug, Clone)]
pub enum LayoutKind {
    Filter {
        saturation: f32,
        opacity: f32,
    },
    Named(String),
    Atom(DrawList),
    Box {
        style: std::boxed::Box<ResolvedStyle>,
        clip: bool,
        group: bool,
        mask: Option<LinearAlphaMask>,
    },
    Text {
        runs: Vec<PositionedRun>,
        selectable: bool,
        ellipsis: bool,
        shadow: Option<bbt_ui_model::draw::TextShadow>,
    },
    Stack,
    ScrollView {
        content: [f32; 2],
        offset: [f32; 2],
    },
    Spacer,
    Icon {
        name: String,
        color: [f32; 4],
    },
    Image {
        asset: String,
        radius: f32,
    },
    Path {
        points: std::sync::Arc<[PathCommand]>,
        stroke: PathStroke,
        color: [f32; 4],
        fill: Option<[f32; 4]>,
        scale: f32,
    },
    /// Slot-relative rects with resolved colours.
    Rects {
        rects: std::sync::Arc<[(Rect, [f32; 4])]>,
        scale: f32,
        repeat: Option<(Axis, f32)>,
    },
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
        italic: s.italic,
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
    if !matches!(
        text.align,
        Alignment::Start | Alignment::Center | Alignment::End
    ) {
        return Err("text alignment must be start, center or end");
    }
    if text.max_lines == Some(0) {
        return Err("text line limit must be positive");
    }
    let mut placed: Vec<PositionedRun> = Vec::new();
    let (mut x, mut y, mut above, mut below, mut widest) = (0_f32, 0_f32, 0_f32, 0_f32, 0_f32);
    let (mut start, mut first) = (0, None);
    let (mut line_number, mut clamped) = (1, false);
    let finish_line = |placed: &mut [PositionedRun], start: usize, y: f32, above: f32| {
        // CSS alignment ignores collapsible spaces at the end of each line.
        let mut right = placed[start..]
            .iter()
            .map(|r| r.rect.right())
            .fold(0., f32::max);
        if text.align != Alignment::Start {
            for run in placed[start..].iter().rev() {
                let trimmed = run.text.trim_end_matches(' ');
                if trimmed.len() != run.text.len() {
                    right -=
                        m.width(&run.text, &font(&run.style)) - m.width(trimmed, &font(&run.style));
                }
                if !trimmed.is_empty() {
                    break;
                }
            }
        }
        let extra = if width.is_finite() {
            (width - right).max(0.)
        } else {
            0.
        };
        let dx = match text.align {
            Alignment::Center => extra * 0.5,
            Alignment::End => extra,
            _ => 0.,
        };
        for run in &mut placed[start..] {
            run.rect.x += dx;
            run.rect.y = y + above - baseline(&run.style, m);
            run.baseline = y + above;
        }
    };
    'runs: for run in &text.runs {
        let s = resolve_text(run.style, ctx)?;
        let line = line_height(&s);
        let value = if s.uppercase {
            run.value.to_uppercase()
        } else {
            run.value.clone()
        };
        let pieces: Vec<&str> = if text.wrap {
            value
                .split_inclusive(|c: char| c.is_whitespace() || c == '-')
                .collect()
        } else {
            value.split_inclusive('\n').collect()
        };
        for piece in pieces {
            let newline = piece.ends_with('\n');
            let value = piece.trim_end_matches(['\n', '\r']);
            let w = m.width(value, &font(&s));
            // Collapsible trailing spaces do not push a fitting word to the
            // next CSS line. Their advance still separates the next word.
            let ink_width = m.width(value.trim_end_matches(' '), &font(&s));
            // Layout subtracts parent insets from measured advances. Ignore
            // float roundoff far below CSS's 1/64px unit, not real shortages.
            if text.wrap && x > 0. && x + ink_width > width + 0.001 {
                if text.max_lines == Some(line_number) {
                    // CSS line-clamp fills the last line with a partial final
                    // word before ellipsizing, rather than dropping that word.
                    let base = baseline(&s, m);
                    above = above.max(base);
                    below = below.max(line - base);
                    placed.push(PositionedRun {
                        text: value.into(),
                        style: s.clone(),
                        rect: Rect::new(x, y, w, line),
                        line_height: line,
                        baseline: y + base,
                    });
                    clamped = true;
                    break 'runs;
                }
                finish_line(&mut placed, start, y, above);
                first.get_or_insert(above);
                widest = widest.max(x);
                y += above + below;
                x = 0.;
                above = 0.;
                below = 0.;
                start = placed.len();
                line_number += 1;
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
                baseline: y + base,
            });
            x += w;
            if newline {
                if text.max_lines == Some(line_number) {
                    clamped = true;
                    break 'runs;
                }
                finish_line(&mut placed, start, y, above);
                first.get_or_insert(above);
                widest = widest.max(x);
                y += above + below;
                x = 0.;
                above = 0.;
                below = 0.;
                start = placed.len();
                line_number += 1;
            }
        }
    }
    if clamped {
        while !placed.is_empty() {
            let count = placed.len();
            let last = placed.last_mut().unwrap();
            last.text = last.text.trim_end().to_owned();
            let value = format!("{}…", last.text);
            let w = m.width(&value, &font(&last.style));
            if last.rect.x + w <= width || (last.rect.x == 0. && last.text.is_empty()) {
                last.text = value;
                last.rect.w = w;
                x = last.rect.right();
                break;
            }
            if last.text.is_empty() && count > start + 1 {
                placed.pop();
            } else {
                last.text.pop();
            }
        }
    }
    finish_line(&mut placed, start, y, above);
    widest = widest.max(x);
    Ok((placed, [widest, y + above + below], first.unwrap_or(above)))
}
type Intrinsic = ([f32; 2], f32);
/// Lives for one immutable-tree layout (or standalone natural-size query).
/// Context and measurer are fixed for that pass; width is the only constraint.
struct LayoutPass<'a> {
    ctx: Context,
    measure: &'a dyn Measure,
    intrinsic: std::collections::HashMap<(*const Element, u32), Intrinsic>,
}
impl<'a> LayoutPass<'a> {
    fn new(ctx: Context, measure: &'a dyn Measure) -> Self {
        Self {
            ctx,
            measure,
            intrinsic: std::collections::HashMap::new(),
        }
    }
}
fn child_measure(
    children: &[Item],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    width: f32,
    pass: &mut LayoutPass<'_>,
    shrink_auto: bool,
) -> crate::style::Result<([f32; 2], Vec<Intrinsic>)> {
    let mut data = children
        .iter()
        .map(|c| c.element.natural_in(width, pass))
        .collect::<crate::style::Result<Vec<_>>>()?;
    if axis == Axis::Row && width.is_finite() {
        let rects = allocate_children(
            children,
            &data,
            axis,
            gap,
            alignment,
            Rect::new(0., 0., width, 0.),
            pass,
            shrink_auto,
        )?;
        // A shrinkable Text may wrap after its main-axis width is assigned.
        // Re-measure cross size at that width before sizing/alignment of the row.
        for ((child, (size, base)), r) in children.iter().zip(&mut data).zip(rects) {
            if r.w < size[0] {
                let (wrapped, baseline) = child.element.natural_in(r.w, pass)?;
                size[1] = wrapped[1];
                *base = baseline;
            }
        }
    }
    let gaps = gap * children.len().saturating_sub(1) as f32;
    let main: f32 = children
        .iter()
        .zip(&data)
        .map(|(c, (size, _))| match c.extent {
            Extent::Fixed(n) => n,
            Extent::Content { min, .. } => size[usize::from(axis == Axis::Column)].max(min),
            _ => size[usize::from(axis == Axis::Column)],
        })
        .sum();
    let size = if axis == Axis::Row {
        let h = if alignment == Alignment::Baseline {
            data.iter().map(|(_, b)| *b).fold(0., f32::max)
                + data.iter().map(|(s, b)| s[1] - b).fold(0., f32::max)
        } else {
            data.iter().map(|(s, _)| s[1]).fold(0., f32::max)
        };
        [main + gaps, h]
    } else {
        [
            data.iter().map(|(s, _)| s[0]).fold(0., f32::max),
            main + gaps,
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
        top: s.padding[0] + if s.border_sides[0] { b } else { 0. },
        right: s.padding[1] + if s.border_sides[1] { b } else { 0. },
        bottom: s.padding[2] + if s.border_sides[2] { b } else { 0. },
        left: s.padding[3] + if s.border_sides[3] { b } else { 0. },
    }
}
impl Element {
    pub fn fixed_height(self, height: f32, key: impl Into<String>, paint_outset: f32) -> Self {
        Self::FixedHeight {
            height,
            paint_outset,
            key: key.into(),
            content: std::boxed::Box::new(self),
        }
    }
    pub fn declared_height(&self) -> Option<f32> {
        match self {
            Self::FixedHeight { height, .. } if height.is_finite() && *height >= 0. => {
                Some(*height)
            }
            _ => None,
        }
    }
    pub fn natural(
        &self,
        width: f32,
        ctx: Context,
        m: &dyn Measure,
    ) -> crate::style::Result<([f32; 2], f32)> {
        self.natural_in(width, &mut LayoutPass::new(ctx, m))
    }
    fn natural_in(&self, width: f32, pass: &mut LayoutPass<'_>) -> crate::style::Result<Intrinsic> {
        // Identity, not text/style equality: distinct nodes can have different
        // baselines or content. Exact constraint bits preserve existing maths.
        let key = (std::ptr::from_ref(self), width.to_bits());
        if let Some(size) = pass.intrinsic.get(&key) {
            return Ok(*size);
        }
        let size = self.measure_in(width, pass)?;
        pass.intrinsic.insert(key, size);
        Ok(size)
    }
    fn measure_in(&self, width: f32, pass: &mut LayoutPass<'_>) -> crate::style::Result<Intrinsic> {
        let (ctx, m) = (pass.ctx, pass.measure);
        match self {
            Self::FixedHeight {
                height,
                paint_outset,
                content,
                ..
            } => {
                if !height.is_finite()
                    || *height < 0.
                    || !paint_outset.is_finite()
                    || *paint_outset < 0.
                {
                    return Err("invalid fixed-height declaration");
                }
                content.natural_in(width, pass)
            }
            Self::Filter {
                content,
                saturation,
                opacity,
            } => {
                if !saturation.is_finite()
                    || !(0. ..=4.).contains(saturation)
                    || !opacity.is_finite()
                    || !(0. ..=1.).contains(opacity)
                {
                    return Err("invalid colour filter");
                }
                content.natural_in(width, pass)
            }
            Self::InlineMiddle { content, parent } => {
                let (size, _) = content.natural_in(width, pass)?;
                let style = resolve_text(*parent, ctx)?;
                let x = m.x_height(&font(&style));
                if !x.is_finite() || x < 0. {
                    return Err("invalid font x-height");
                }
                Ok((size, size[1] * 0.5 + x * 0.5))
            }
            Self::Baseline { content, offset } => {
                let (size, baseline) = content.natural_in(width, pass)?;
                Ok((size, baseline + offset))
            }
            Self::Named { content, .. } => content.natural_in(width, pass),
            Self::Layers { base, .. } => base.natural_in(width, pass),
            Self::Glyph { size, .. } => Ok(([*size; 2], *size)),
            Self::Text(t) => {
                let (_, mut size, base) = text_lines(t, width, ctx, m)?;
                if (t.max_content_basis || t.align != Alignment::Start)
                    && t.wrap
                    && width.is_finite()
                {
                    size[0] = size[0].max(self.natural_in(f32::INFINITY, pass)?.0[0].min(width));
                }
                Ok((size, base))
            }
            Self::EditingText(t) => t.natural(ctx, m),
            Self::Spacer => Ok(([0., 0.], 0.)),
            Self::Path(p) => {
                p.stroke.validate(&p.points)?;
                if p.size.iter().any(|n| !n.is_finite() || *n < 0.)
                    || !p.opacity.is_finite()
                    || !(0. ..=1.).contains(&p.opacity)
                {
                    return Err("invalid path viewport or opacity");
                }
                Ok((p.size, p.size[1]))
            }
            Self::Rects(r) => {
                if r.size.iter().any(|n| !n.is_finite() || *n < 0.)
                    || r.repeat
                        .is_some_and(|(_, period)| !period.is_finite() || period <= 0.)
                    || r.rects.iter().any(|(rect, _)| {
                        [rect.x, rect.y, rect.w, rect.h]
                            .iter()
                            .any(|n| !n.is_finite())
                            || rect.w < 0.
                            || rect.h < 0.
                    })
                {
                    return Err("invalid rects slot or geometry");
                }
                Ok((r.size, r.size[1]))
            }
            Self::Icon { size, .. } => {
                let s = size.resolve(ctx.density)?;
                Ok(([s, s], s))
            }
            Self::Image {
                asset,
                size,
                radius,
            } => {
                if asset.is_empty() {
                    return Err("image asset key is empty");
                }
                let s = size.resolve(ctx.density)?;
                radius.resolve(ctx.density)?;
                Ok(([s, s], s))
            }
            Self::ScrollView(s) => Ok((s.viewport, s.viewport[1])),
            Self::Overlay(o) => {
                let s = o.style.resolve(ctx)?;
                let p = inset(&s);
                let (child, _) = o
                    .content
                    .natural_in((width - p.left - p.right).max(0.), pass)?;
                let h = s.height.unwrap_or(
                    (child[1] + p.top + p.bottom).max(o.min_height.resolve(ctx.density)?),
                );
                Ok(([width, h], h))
            }
            Self::Flow(f) => {
                let (size, base, _) = flow_layout(f, Rect::new(0., 0., width, 0.), pass)?;
                Ok((size, base))
            }
            Self::Stack(s) => {
                let (size, data) = child_measure(
                    &s.children,
                    s.direction,
                    s.gap.resolve(ctx.density)?,
                    s.align,
                    width,
                    pass,
                    false,
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
                let width = s.width.map_or(width, |w| w.min(width));
                let (mut content, data) = child_measure(
                    &b.children,
                    b.direction,
                    b.gap.resolve(ctx.density)?,
                    b.align,
                    (width - p.left - p.right).max(0.),
                    pass,
                    false,
                )?;
                let mut base = data.first().map_or(0., |(_, b)| *b);
                if b.inline {
                    let strut = baseline(&s.text, m);
                    let above = data.iter().map(|(_, b)| *b).fold(strut, f32::max);
                    let below = data
                        .iter()
                        .map(|(s, b)| s[1] - b)
                        .fold(line_height(&s.text) - strut, f32::max);
                    content[1] = above + below;
                    base = above;
                }
                let h = s.height.unwrap_or(content[1] + p.top + p.bottom);
                Ok((
                    [
                        s.width
                            .map_or(content[0] + p.left + p.right, |w| w.min(width)),
                        h,
                    ],
                    p.top + base,
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
        let mut pass = LayoutPass::new(ctx, m);
        let (size, _) = self.natural_in(bounds.w, &mut pass)?;
        self.layout_in(Rect::new(bounds.x, bounds.y, bounds.w, size[1]), &mut pass)
    }
    fn layout_in(&self, rect: Rect, pass: &mut LayoutPass<'_>) -> crate::style::Result<Layout> {
        let (ctx, m) = (pass.ctx, pass.measure);
        let (_, base) = self.natural_in(rect.w, pass)?;
        let (kind, children) = match self {
            Self::FixedHeight { content, .. } => return content.layout_in(rect, pass),
            Self::Filter {
                content,
                saturation,
                opacity,
            } => (
                LayoutKind::Filter {
                    saturation: *saturation,
                    opacity: *opacity,
                },
                vec![content.layout_in(rect, pass)?],
            ),
            Self::InlineMiddle { content, .. } => {
                let mut child = content.layout_in(rect, pass)?;
                child.baseline = base;
                return Ok(child);
            }
            Self::Baseline { content, offset } => {
                let mut child = content.layout_in(rect, pass)?;
                child.baseline += offset;
                return Ok(child);
            }
            Self::Named { name, content } => {
                let child = content.layout_in(rect, pass)?;
                return Ok(Layout {
                    rect: child.rect,
                    baseline: child.baseline,
                    kind: LayoutKind::Named(name.clone()),
                    children: vec![child],
                });
            }
            Self::Layers { base, overlays } => {
                let mut children = vec![base.layout_in(rect, pass)?];
                for p in overlays {
                    let (size, _) = p.content.natural_in((rect.w - p.left).max(0.), pass)?;
                    children.push(p.content.layout_in(
                        Rect::new(
                            rect.x + p.left,
                            rect.bottom() - p.bottom - size[1],
                            size[0],
                            size[1],
                        ),
                        pass,
                    )?);
                }
                (LayoutKind::Stack, children)
            }
            Self::Glyph {
                kind_colors,
                shadow,
                activity,
                grade,
                color,
                size,
                scale,
            } => {
                let draw = DrawCtx::new(ctx.theme, ctx.density);
                let scaled = Rect::new(
                    rect.x + (*size - *size * scale) * 0.5,
                    rect.y + (*size - *size * scale) * 0.5,
                    *size * scale,
                    *size * scale,
                );
                let mut list = DrawList::default();
                crate::atoms::Glyph {
                    activity: *activity,
                    grade: *grade,
                    seat_color: Color(color.resolve(ctx)?),
                }
                .paint_with_kind_colors_and_shadow(
                    scaled,
                    &crate::atoms::AtomCtx {
                        draw,
                        measure: m,
                        scale: 1.,
                    },
                    &mut list,
                    *kind_colors,
                    shadow.map(|s| s.resolve(ctx)).transpose()?.map(|mut s| {
                        s.blur_radius *= scale;
                        s.offset = s.offset.map(|v| v * scale);
                        s
                    }),
                );
                return Ok(Layout {
                    rect: scaled,
                    baseline: scaled.h,
                    kind: LayoutKind::Atom(list),
                    children: vec![],
                });
            }
            Self::Text(t) => {
                let (mut runs, _, _) = text_lines(t, rect.w, ctx, m)?;
                for run in &mut runs {
                    run.rect.x += rect.x;
                    run.rect.y += rect.y;
                    run.baseline += rect.y;
                }
                (
                    LayoutKind::Text {
                        runs,
                        selectable: t.selectable,
                        ellipsis: t.ellipsis,
                        shadow: t.shadow.map(|s| s.resolve(ctx)).transpose()?,
                    },
                    vec![],
                )
            }
            Self::EditingText(t) => return t.layout(rect, ctx, m),
            Self::Spacer => (LayoutKind::Spacer, vec![]),
            Self::Path(p) => (
                LayoutKind::Path {
                    points: p.points.clone(),
                    stroke: p.stroke.clone(),
                    color: Color(p.paint.resolve(ctx)?).alpha(p.opacity).0,
                    fill: p
                        .fill
                        .map(|f| f.resolve(ctx))
                        .transpose()?
                        .map(|f| Color(f).alpha(p.opacity).0),
                    scale: 1.,
                },
                vec![],
            ),
            Self::Rects(r) => (
                LayoutKind::Rects {
                    rects: r
                        .rects
                        .iter()
                        .map(|(rect, paint)| Ok((*rect, paint.resolve(ctx)?)))
                        .collect::<crate::style::Result<_>>()?,
                    scale: 1.,
                    repeat: r.repeat,
                },
                vec![],
            ),
            Self::Icon { name, paint, .. } => (
                LayoutKind::Icon {
                    name: name.clone(),
                    color: paint.resolve(ctx)?,
                },
                vec![],
            ),
            Self::Image { asset, radius, .. } => (
                LayoutKind::Image {
                    asset: asset.clone(),
                    radius: radius.resolve(ctx.density)?,
                },
                vec![],
            ),
            Self::Flow(f) => (LayoutKind::Stack, flow_layout(f, rect, pass)?.2),
            Self::Stack(s) => (
                LayoutKind::Stack,
                layout_children(
                    &s.children,
                    s.direction,
                    s.gap.resolve(ctx.density)?,
                    s.align,
                    rect,
                    pass,
                    false,
                )?,
            ),
            Self::Box(b) => {
                if let Some(mask) = b.mask {
                    mask.validate()?;
                }
                let style = b.style.resolve(ctx)?;
                let mut children = layout_children(
                    &b.children,
                    b.direction,
                    b.gap.resolve(ctx.density)?,
                    b.align,
                    rect.inset(inset(&style)),
                    pass,
                    b.cross_overflow,
                )?;
                if b.inline {
                    let top = rect.inset(inset(&style)).y;
                    let present = children
                        .iter()
                        .map(|c| c.rect.y + c.baseline)
                        .fold(top, f32::max);
                    let delta = (top + baseline(&style.text, m) - present).max(0.);
                    for child in &mut children {
                        translate_layout(child, delta);
                    }
                }
                (
                    LayoutKind::Box {
                        style: std::boxed::Box::new(style),
                        clip: b.clip,
                        group: b.group,
                        mask: b.mask,
                    },
                    children,
                )
            }
            Self::Overlay(o) => {
                let s = o.style.resolve(ctx)?;
                let inner = rect.inset(inset(&s));
                let (natural, _) = o.content.natural_in(inner.w, pass)?;
                let w = if o.horizontal == Alignment::Stretch {
                    inner.w
                } else {
                    natural[0].min(inner.w)
                };
                let h = if o.vertical == Alignment::Stretch {
                    inner.h
                } else {
                    natural[1]
                };
                let offset = |align, free: f32| match align {
                    Alignment::Center => free * 0.5,
                    Alignment::End => free,
                    _ => 0.,
                };
                let child = o.content.layout_in(
                    Rect::new(
                        inner.x + offset(o.horizontal, inner.w - w),
                        inner.y + offset(o.vertical, inner.h - h),
                        w,
                        h,
                    ),
                    pass,
                )?;
                (
                    LayoutKind::Box {
                        style: std::boxed::Box::new(s),
                        clip: o.clip,
                        group: false,
                        mask: None,
                    },
                    vec![child],
                )
            }
            Self::ScrollView(s) => {
                let content_w = s.content_width.unwrap_or(rect.w).max(rect.w);
                let declared = s.content.declared_height();
                let height = match declared {
                    Some(h) => h,
                    None => s.content.natural_in(content_w, pass)?.0[1],
                };
                let content = [content_w, height.max(rect.h)];
                let offset = [
                    s.offset[0].clamp(0., (content[0] - rect.w).max(0.)),
                    s.offset[1].clamp(0., (content[1] - rect.h).max(0.)),
                ];
                let content_rect = Rect::new(
                    rect.x - offset[0],
                    rect.y - offset[1],
                    content[0],
                    content[1],
                );
                let child = if declared.is_some() {
                    let outset = match s.content.as_ref() {
                        Self::FixedHeight { paint_outset, .. } => *paint_outset,
                        _ => 0.,
                    } + pass.ctx.density.tokens().row_h as f32;
                    let visible = Rect::new(rect.x, rect.y - outset, rect.w, rect.h + outset * 2.);
                    layout_fixed(&s.content, content_rect, visible, pass)?
                } else {
                    s.content.layout_in(content_rect, pass)?
                };
                (LayoutKind::ScrollView { content, offset }, vec![child])
            }
        };
        let mut layout = Layout {
            rect,
            baseline: base,
            kind,
            children,
        };
        if let Self::Text(t) = self
            && t.inline
        {
            if t.wrap {
                return Err("inline Text requires a single nonwrapping line");
            }
            if let LayoutKind::Text { runs, .. } = &layout.kind
                && let Some(run) = runs.first()
            {
                let [a, d] = m.font_metrics(&font(&run.style)).map(f32::round);
                let leading = layout.baseline - a;
                layout.rect.y += leading;
                layout.rect.h = a + d;
                layout.baseline = a;
            }
        }
        if let Self::Box(b) = self {
            if !b.scale.is_finite() || b.scale <= 0. {
                return Err("invalid box scale");
            }
            if b.scale != 1. {
                scale_layout(&mut layout, rect, b.scale);
            }
        }
        Ok(layout)
    }
}
fn declared_item_height(item: &Item) -> Option<f32> {
    match item.extent {
        Extent::Auto => item.element.declared_height(),
        Extent::Fixed(h) if h.is_finite() && h >= 0. => Some(h),
        _ => None,
    }
}

/// Keep solve_box's local cursor additions, independently of its natural-height
/// sum. Reassociating these floats changes fractional row positions.
fn fixed_prefix(children: &[Item], gap: f32) -> Option<Vec<(f32, f32)>> {
    let mut cursor = 0.;
    children
        .iter()
        .map(|item| {
            let h = declared_item_height(item)?;
            let start = cursor;
            cursor += h + gap;
            Some((start, h))
        })
        .collect()
}

fn fixed_baseline(e: &Element, width: f32, pass: &mut LayoutPass<'_>) -> crate::style::Result<f32> {
    match e {
        Element::FixedHeight { content, .. }
        | Element::Named { content, .. }
        | Element::Filter { content, .. } => fixed_baseline(content, width, pass),
        Element::Baseline { content, offset } => Ok(fixed_baseline(content, width, pass)? + offset),
        Element::Box(b) if b.direction == Axis::Column && !b.inline => {
            let p = inset(&b.style.resolve(pass.ctx)?);
            Ok(p.top
                + match b.children.first() {
                    Some(c) => {
                        fixed_baseline(&c.element, (width - p.left - p.right).max(0.), pass)?
                    }
                    None => 0.,
                })
        }
        Element::Stack(s) if s.direction == Axis::Column => match s.children.first() {
            Some(c) => fixed_baseline(&c.element, width, pass),
            None => Ok(0.),
        },
        _ => Ok(e.natural_in(width, pass)?.1),
    }
}

fn fixed_children(
    children: &[Item],
    gap: f32,
    rect: Rect,
    visible: Rect,
    pass: &mut LayoutPass<'_>,
) -> crate::style::Result<Option<Vec<Layout>>> {
    let Some(prefix) = fixed_prefix(children, gap) else {
        return Ok(None);
    };
    let start = prefix.partition_point(|(y, h)| rect.y + *y + *h <= visible.y);
    let end = prefix.partition_point(|(y, _)| rect.y + *y < visible.bottom());
    let mut out = Vec::with_capacity(end.saturating_sub(start));
    for i in start..end {
        let (y, h) = prefix[i];
        out.push(layout_fixed(
            &children[i].element,
            Rect::new(rect.x, rect.y + y, rect.w, h),
            visible,
            pass,
        )?);
    }
    Ok(Some(out))
}

fn layout_fixed(
    e: &Element,
    rect: Rect,
    visible: Rect,
    pass: &mut LayoutPass<'_>,
) -> crate::style::Result<Layout> {
    let (kind, children) = match e {
        Element::FixedHeight {
            height,
            paint_outset,
            content,
            ..
        } => {
            if !height.is_finite()
                || *height < 0.
                || !paint_outset.is_finite()
                || *paint_outset < 0.
            {
                return Err("invalid fixed-height declaration");
            }
            return layout_fixed(content, rect, visible, pass);
        }
        Element::Named { name, content } => {
            let child = layout_fixed(content, rect, visible, pass)?;
            return Ok(Layout {
                rect: child.rect,
                baseline: child.baseline,
                kind: LayoutKind::Named(name.clone()),
                children: vec![child],
            });
        }
        Element::Filter {
            content,
            saturation,
            opacity,
        } => {
            if !saturation.is_finite()
                || !(0. ..=4.).contains(saturation)
                || !opacity.is_finite()
                || !(0. ..=1.).contains(opacity)
            {
                return Err("invalid colour filter");
            }
            (
                LayoutKind::Filter {
                    saturation: *saturation,
                    opacity: *opacity,
                },
                vec![layout_fixed(content, rect, visible, pass)?],
            )
        }
        Element::Baseline { content, offset } => {
            let mut child = layout_fixed(content, rect, visible, pass)?;
            child.baseline += offset;
            return Ok(child);
        }
        Element::Box(b)
            if b.direction == Axis::Column
                && b.align == Alignment::Stretch
                && !b.inline
                && b.scale == 1. =>
        {
            let style = b.style.resolve(pass.ctx)?;
            if let Some(mask) = b.mask {
                mask.validate()?;
            }
            let Some(children) = fixed_children(
                &b.children,
                b.gap.resolve(pass.ctx.density)?,
                rect.inset(inset(&style)),
                visible,
                pass,
            )?
            else {
                return e.layout_in(rect, pass);
            };
            (
                LayoutKind::Box {
                    style: std::boxed::Box::new(style),
                    clip: b.clip,
                    group: b.group,
                    mask: b.mask,
                },
                children,
            )
        }
        Element::Stack(s) if s.direction == Axis::Column && s.align == Alignment::Stretch => {
            let Some(children) = fixed_children(
                &s.children,
                s.gap.resolve(pass.ctx.density)?,
                rect,
                visible,
                pass,
            )?
            else {
                return e.layout_in(rect, pass);
            };
            (LayoutKind::Stack, children)
        }
        _ => return e.layout_in(rect, pass),
    };
    Ok(Layout {
        rect,
        baseline: fixed_baseline(e, rect.w, pass)?,
        kind,
        children,
    })
}

fn anchor_rows(
    e: &Element,
    y: f32,
    ctx: Context,
    out: &mut Vec<(String, f32, f32)>,
) -> crate::style::Result<()> {
    match e {
        Element::FixedHeight {
            key,
            height,
            content,
            ..
        } => {
            if !key.is_empty() {
                out.push((key.clone(), y, *height));
            } else {
                anchor_rows(content, y, ctx, out)?;
            }
        }
        Element::Named { content, .. }
        | Element::Filter { content, .. }
        | Element::Baseline { content, .. } => anchor_rows(content, y, ctx, out)?,
        Element::Box(b) if b.direction == Axis::Column => {
            let p = inset(&b.style.resolve(ctx)?);
            if let Some(prefix) = fixed_prefix(&b.children, b.gap.resolve(ctx.density)?) {
                for (item, (dy, _)) in b.children.iter().zip(prefix) {
                    anchor_rows(&item.element, y + p.top + dy, ctx, out)?;
                }
            }
        }
        Element::Stack(s) if s.direction == Axis::Column => {
            if let Some(prefix) = fixed_prefix(&s.children, s.gap.resolve(ctx.density)?) {
                for (item, (dy, _)) in s.children.iter().zip(prefix) {
                    anchor_rows(&item.element, y + dy, ctx, out)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

impl ScrollView {
    /// Snapshot only declared fixed items. Variable-height views retain their
    /// old caller-controlled offset and return no fixed-item anchor.
    pub fn anchor(&self, ctx: Context) -> crate::style::Result<Option<ScrollAnchor>> {
        let Some(height) = self.content.declared_height() else {
            return Ok(None);
        };
        let offset = self.offset[1].clamp(0., (height - self.viewport[1]).max(0.));
        let mut rows = vec![];
        anchor_rows(&self.content, 0., ctx, &mut rows)?;
        let Some((key, y, _)) = rows.iter().find(|(_, y, h)| *y + *h > offset) else {
            return Ok(None);
        };
        Ok(Some(ScrollAnchor {
            key: key.clone(),
            within: offset - *y,
            order: rows.into_iter().map(|r| r.0).collect(),
        }))
    }
    /// Restore the same item and pixel position. If removed, choose the nearest
    /// surviving old neighbour, preferring the following item on a tie.
    pub fn restore_anchor(
        &mut self,
        anchor: &ScrollAnchor,
        ctx: Context,
    ) -> crate::style::Result<bool> {
        let Some(height) = self.content.declared_height() else {
            return Ok(false);
        };
        let mut rows = vec![];
        anchor_rows(&self.content, 0., ctx, &mut rows)?;
        let positions: std::collections::HashMap<&str, f32> =
            rows.iter().map(|(key, y, _)| (key.as_str(), *y)).collect();
        let mut y = positions.get(anchor.key.as_str()).copied();
        if y.is_none()
            && let Some(index) = anchor.order.iter().position(|k| k == &anchor.key)
        {
            for distance in 1..anchor.order.len() {
                for candidate in [
                    index
                        .checked_add(distance)
                        .filter(|i| *i < anchor.order.len()),
                    index.checked_sub(distance),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(found) = positions.get(anchor.order[candidate].as_str()) {
                        y = Some(*found);
                        break;
                    }
                }
                if y.is_some() {
                    break;
                }
            }
        }
        if let Some(y) = y {
            self.offset[1] = (y + anchor.within).clamp(0., (height - self.viewport[1]).max(0.));
            return Ok(true);
        }
        Ok(false)
    }
}

fn translate_layout(node: &mut Layout, dy: f32) {
    node.rect.y += dy;
    if let LayoutKind::Text { runs, .. } = &mut node.kind {
        for run in runs {
            run.rect.y += dy;
            run.baseline += dy;
        }
    }
    for child in &mut node.children {
        translate_layout(child, dy);
    }
}
fn scale_layout(node: &mut Layout, pivot: Rect, scale: f32) {
    let point = |x: f32, y: f32| {
        [
            pivot.x + pivot.w * 0.5 + (x - pivot.x - pivot.w * 0.5) * scale,
            pivot.y + pivot.h * 0.5 + (y - pivot.y - pivot.h * 0.5) * scale,
        ]
    };
    let rect = |r: Rect| {
        let p = point(r.x, r.y);
        Rect::new(p[0], p[1], r.w * scale, r.h * scale)
    };
    node.rect = rect(node.rect);
    node.baseline *= scale;
    match &mut node.kind {
        LayoutKind::Path {
            scale: path_scale, ..
        }
        | LayoutKind::Rects {
            scale: path_scale, ..
        } => *path_scale *= scale,
        LayoutKind::Image { radius, .. } => *radius *= scale,
        LayoutKind::Box { style, mask, .. } => {
            if let Some(mask) = mask {
                mask.start = mask.start.map(|v| v * scale);
                mask.end = mask.end.map(|v| v * scale);
            }
            style.radii = style.radii.map(|r| r * scale);
            for border in style
                .border
                .iter_mut()
                .chain(style.rings.iter_mut())
                .chain(style.focus_ring.iter_mut())
            {
                border.width *= scale;
            }
            if let Some(shadow) = &mut style.shadow {
                shadow.blur *= scale;
                shadow.offset = shadow.offset.map(|v| v * scale);
            }
        }
        LayoutKind::Text { runs, shadow, .. } => {
            for run in runs {
                run.baseline = point(run.rect.x, run.baseline)[1];
                run.rect = rect(run.rect);
                run.style.size *= scale;
                run.line_height *= scale;
            }
            if let Some(shadow) = shadow {
                shadow.blur_radius *= scale;
                shadow.offset = shadow.offset.map(|v| v * scale);
            }
        }
        _ => {}
    }
    for child in &mut node.children {
        scale_layout(child, pivot, scale);
    }
}
// Flow only packs rows. The existing row allocator still owns shrink/grow,
// text reflow and cross alignment inside each row.
fn flow_layout(
    f: &Flow,
    bounds: Rect,
    pass: &mut LayoutPass<'_>,
) -> crate::style::Result<([f32; 2], f32, Vec<Layout>)> {
    let ctx = pass.ctx;
    let column_gap = f.column_gap.resolve(ctx.density)?;
    let row_gap = f.row_gap.resolve(ctx.density)?;
    let mut data = Vec::new();
    for child in &f.children {
        let (size, _) = child.element.natural_in(f32::INFINITY, pass)?;
        let min = child.element.natural_in(0., pass)?.0[0];
        let basis = match child.extent {
            Extent::Fixed(n) => n,
            Extent::Content { min, max, .. } => {
                size[0].clamp(min, max.unwrap_or(f32::MAX).max(min))
            }
            _ => size[0],
        };
        data.push((basis, min));
    }
    let width = if bounds.w.is_finite() {
        bounds.w.max(0.)
    } else {
        data.iter().map(|(w, _)| *w).sum::<f32>() + column_gap * data.len().saturating_sub(1) as f32
    };
    // Keep node identities stable: row-local clones would leave dangling cache
    // keys and could reuse another row's address. Only extents are overridden.
    let mut rows: Vec<std::ops::Range<usize>> = vec![];
    let mut used = 0.;
    for (index, (basis, min)) in data.into_iter().enumerate() {
        let hypothetical = if f.wrap {
            basis.min(width.max(min))
        } else {
            basis
        };
        let next =
            used + if rows.last().is_some_and(|r| !r.is_empty()) {
                column_gap
            } else {
                0.
            } + hypothetical;
        if rows.is_empty()
            || (f.wrap && next > width + 0.001 && rows.last().is_some_and(|r| !r.is_empty()))
        {
            rows.push(index..index);
            used = 0.;
        }
        if !rows.last().unwrap().is_empty() {
            used += column_gap;
        }
        used += hypothetical;
        rows.last_mut().unwrap().end = index + 1;
    }
    let mut output = vec![];
    let mut y = bounds.y;
    let mut widest = 0f32;
    let mut first = None;
    for range in rows {
        let row = &f.children[range];
        let max_content = row
            .iter()
            .map(|c| {
                c.element
                    .natural_in(f32::INFINITY, pass)
                    .map(|(s, _)| match c.extent {
                        Extent::Fixed(n) => n,
                        _ => s[0],
                    })
            })
            .collect::<crate::style::Result<Vec<_>>>()?
            .iter()
            .sum::<f32>()
            + column_gap * row.len().saturating_sub(1) as f32;
        // A fitting row takes the exact existing Stack allocation path. Only
        // actual shortages need CSS min-content shrinking.
        let shrink_auto = max_content > width + 0.001;
        let (_, metrics) = child_measure(
            row,
            Axis::Row,
            column_gap,
            f.align,
            width,
            pass,
            shrink_auto,
        )?;
        let above = metrics.iter().map(|(_, b)| *b).fold(0., f32::max);
        let height = if f.align == Alignment::Baseline {
            above + metrics.iter().map(|(s, b)| s[1] - b).fold(0., f32::max)
        } else {
            metrics.iter().map(|(s, _)| s[1]).fold(0., f32::max)
        };
        let mut rects = allocate_children(
            row,
            &metrics,
            Axis::Row,
            column_gap,
            f.align,
            Rect::new(bounds.x, y, width, height),
            pass,
            shrink_auto,
        )?;
        let used = rects.iter().map(|r| r.w).sum::<f32>()
            + column_gap * rects.len().saturating_sub(1) as f32;
        widest = widest.max(used);
        let free = (width - used).max(0.);
        let (offset, extra_gap) = match f.justify {
            Justify::Start => (0., 0.),
            Justify::Center => (free * 0.5, 0.),
            Justify::SpaceBetween if rects.len() > 1 => (0., free / (rects.len() - 1) as f32),
            Justify::SpaceBetween => (0., 0.),
        };
        for (i, ((child, r), (_, base))) in row.iter().zip(&mut rects).zip(&metrics).enumerate() {
            r.x += offset + extra_gap * i as f32;
            if f.align == Alignment::Baseline {
                r.y += above - base;
            }
            let layout = child.element.layout_in(*r, pass)?;
            first.get_or_insert(layout.rect.y - bounds.y + layout.baseline);
            output.push(layout);
        }
        y += height + row_gap;
    }
    let height = (y - bounds.y - if output.is_empty() { 0. } else { row_gap }).max(0.);
    Ok(([widest, height], first.unwrap_or(0.), output))
}

#[allow(clippy::too_many_arguments)] // Same solver inputs plus explicit CSS cross-overflow policy.
fn layout_children(
    children: &[Item],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    r: Rect,
    pass: &mut LayoutPass<'_>,
    cross_overflow: bool,
) -> crate::style::Result<Vec<Layout>> {
    let (_, data) = child_measure(children, axis, gap, alignment, r.w, pass, false)?;
    let mut rects = allocate_children(children, &data, axis, gap, alignment, r, pass, false)?;
    if cross_overflow && axis == Axis::Row && alignment != Alignment::Stretch {
        for (rect, (size, _)) in rects.iter_mut().zip(&data) {
            rect.h = size[1];
            rect.y = r.y
                + match alignment {
                    Alignment::Center => (r.h - rect.h) * 0.5,
                    Alignment::End => r.h - rect.h,
                    _ => 0.,
                };
        }
    }
    if axis == Axis::Row && alignment == Alignment::Baseline {
        let max = data.iter().map(|(_, b)| *b).fold(0., f32::max);
        for (rect, (_, b)) in rects.iter_mut().zip(&data) {
            rect.y += max - b;
        }
    }
    children
        .iter()
        .zip(rects)
        .map(|(c, r)| c.element.layout_in(r, pass))
        .collect()
}

fn fn_content_inset(element: &Element, ctx: Context) -> crate::style::Result<f32> {
    match element {
        Element::Named { content, .. }
        | Element::FixedHeight { content, .. }
        | Element::Baseline { content, .. }
        | Element::InlineMiddle { content, .. } => fn_content_inset(content, ctx),
        Element::Box(b) => {
            let p = inset(&b.style.resolve(ctx)?);
            Ok(p.left + p.right)
        }
        _ => Ok(0.),
    }
}
#[allow(clippy::too_many_arguments)] // Shared solver adapter; keeps the public tree free of renderer state.
fn allocate_children(
    children: &[Item],
    data: &[([f32; 2], f32)],
    axis: Axis,
    gap: f32,
    alignment: Alignment,
    r: Rect,
    pass: &mut LayoutPass<'_>,
    shrink_auto: bool,
) -> crate::style::Result<Vec<Rect>> {
    let ctx = pass.ctx;
    if axis == Axis::Row
        && children.iter().any(|c| {
            matches!(c.extent, Extent::Content { .. } | Extent::MinContent)
                || (shrink_auto && c.extent == Extent::Auto)
        })
    {
        let available = (r.w - gap * children.len().saturating_sub(1) as f32).max(0.);
        let mut bases = Vec::new();
        let mut growth = Vec::new();
        let mut shrinking = Vec::new();
        let mut mins = Vec::new();
        let mut maxes = Vec::new();
        for (c, (size, _)) in children.iter().zip(data) {
            let extent = if shrink_auto && c.extent == Extent::Auto {
                Extent::MinContent
            } else {
                c.extent
            };
            let (basis, g, s, min, max) = match extent {
                Extent::Fixed(n) => (n, 0., 0., n, n),
                Extent::MinContent => (
                    size[0],
                    0.,
                    1.,
                    c.element.natural_in(0., pass)?.0[0],
                    f32::MAX,
                ),
                Extent::Fill => (0., 1., 1., 0., f32::MAX),
                Extent::Flex(w) => (0., w, 1., 0., f32::MAX),
                Extent::Content {
                    grow,
                    shrink,
                    min,
                    max,
                    max_fraction,
                } => (
                    size[0],
                    grow,
                    shrink,
                    min,
                    max.unwrap_or(f32::MAX)
                        .min(max_fraction.map_or(f32::MAX, |f| r.w * f)),
                ),
                Extent::Auto => (size[0], 0., 0., size[0], size[0]),
            };
            bases.push(basis);
            growth.push(g);
            let content_inset = fn_content_inset(&c.element, ctx)?;
            shrinking.push(s * (basis - content_inset).max(0.));
            mins.push(min);
            maxes.push(max.max(min));
        }
        let mut sizes = bases.clone();
        let positive = available
            >= bases
                .iter()
                .enumerate()
                .map(|(i, b)| b.clamp(mins[i], maxes[i]))
                .sum::<f32>();
        let factors = if positive { &growth } else { &shrinking };
        let mut frozen: Vec<bool> = factors.iter().map(|f| *f == 0.).collect();
        for i in 0..sizes.len() {
            if frozen[i] {
                sizes[i] = bases[i].clamp(mins[i], maxes[i]);
            }
        }
        // CSS flex resolves violations iteratively: a max-clamped invariant chip
        // freezes before distributing the remaining shrink to a title.
        for _ in 0..=children.len() {
            let free = available
                - bases
                    .iter()
                    .enumerate()
                    .map(|(i, b)| if frozen[i] { sizes[i] } else { *b })
                    .sum::<f32>();
            let total = factors
                .iter()
                .enumerate()
                .filter(|(i, _)| !frozen[*i])
                .map(|(_, f)| f)
                .sum::<f32>();
            let mut violations = vec![0.; sizes.len()];
            for i in 0..sizes.len() {
                if frozen[i] {
                    continue;
                }
                let proposed = bases[i]
                    + if total > 0. {
                        free * factors[i] / total
                    } else {
                        0.
                    };
                sizes[i] = proposed.clamp(mins[i], maxes[i]);
                violations[i] = sizes[i] - proposed;
            }
            let adjustment = violations.iter().sum::<f32>();
            if adjustment.abs() < 0.0001 {
                break;
            }
            let mut changed = false;
            for i in 0..sizes.len() {
                if !frozen[i]
                    && ((adjustment > 0. && violations[i] > 0.)
                        || (adjustment < 0. && violations[i] < 0.))
                {
                    frozen[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut x = r.x;
        return Ok(sizes
            .iter()
            .zip(data)
            .map(|(w, (size, _))| {
                let h = if alignment == Alignment::Stretch {
                    r.h
                } else {
                    size[1].min(r.h)
                };
                let y = r.y
                    + match alignment {
                        Alignment::Center => (r.h - h) * 0.5,
                        Alignment::End => r.h - h,
                        _ => 0.,
                    };
                let rect = Rect::new(x, y, *w, h);
                x += *w + gap;
                rect
            })
            .collect());
    }
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
            Extent::Content { .. } | Extent::MinContent => Size::Fixed(size[axis_i]),
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
    Ok(layout::solve_box(&model, r, pass.measure))
}

#[derive(Debug, Clone, Copy)]
pub struct Clip {
    pub rect: Rect,
    pub radii: [f32; 4],
}
#[derive(Debug, Clone)]
pub enum Paint {
    /// Paint children to transparent storage before applying this filter once.
    Group {
        ops: Vec<Paint>,
        opacity: f32,
        saturation: f32,
        mask: Option<LinearAlphaMask>,
    },
    Draw(DrawList),
    Underlay(DrawList),
    Material {
        rect: Rect,
        radii: [f32; 4],
        kind: Material,
        color: Color,
        blur: Option<crate::ResolvedBlur>,
        rim: Option<crate::Rim>,
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
/// Splits a block's top-level alive rims from the rest of its paint (rim-glow §7): a host
/// paints every block first and the rims after, so a swell lies over its neighbours as the
/// lab's rim canvases do; at rest the rims touch no neighbour and nothing changes.
pub fn split_rims(ops: Vec<Paint>) -> (Vec<Paint>, Vec<Paint>) {
    ops.into_iter().partition(|op| {
        !matches!(
            op,
            Paint::Material {
                kind: Material::Rim,
                ..
            }
        )
    })
}
pub fn paint(layout: &Layout, ctx: Context) -> Vec<Paint> {
    paint_with_draw(layout, DrawCtx::new(ctx.theme, ctx.density))
}
/// Hosts may inherit an ancestor's dim context without changing module content.
pub fn paint_with_draw(layout: &Layout, draw: DrawCtx) -> Vec<Paint> {
    paint_with_ancestor_clips(layout, draw, &[])
}
/// A viewport owner may impose clips outside the component's retained tree.
pub fn paint_with_ancestor_clips(layout: &Layout, mut draw: DrawCtx, clips: &[Clip]) -> Vec<Paint> {
    let mut out = Vec::new();
    let dim = draw.dim.take();
    paint_node(layout, draw, 1., clips, &mut out);
    if let Some(dim) = dim {
        vec![Paint::Group {
            ops: out,
            opacity: dim.opacity,
            saturation: dim.saturate,
            mask: None,
        }]
    } else {
        out
    }
}
fn paint_node(node: &Layout, draw: DrawCtx, opacity: f32, clips: &[Clip], out: &mut Vec<Paint>) {
    if let LayoutKind::Filter {
        saturation,
        opacity: filter_alpha,
    } = &node.kind
    {
        let mut ops = Vec::new();
        for child in &node.children {
            paint_node(child, draw, opacity, clips, &mut ops);
        }
        out.push(Paint::Group {
            ops,
            opacity: *filter_alpha,
            saturation: *saturation,
            mask: None,
        });
        return;
    }
    if let LayoutKind::Box {
        style, group, mask, ..
    } = &node.kind
        && (*group || mask.is_some() || style.opacity != 1. || style.dim.is_some())
    {
        let mut ops = Vec::new();
        paint_contents(node, draw, opacity, clips, &mut ops);
        out.push(Paint::Group {
            ops,
            opacity: style.opacity * style.dim.map_or(1., |d| d.opacity),
            saturation: style.dim.map_or(1., |d| d.saturate),
            mask: mask.map(|m| m.translated([node.rect.x, node.rect.y])),
        });
        return;
    }
    paint_contents(node, draw, opacity, clips, out);
}
fn paint_contents(
    node: &Layout,
    draw: DrawCtx,
    opacity: f32,
    clips: &[Clip],
    out: &mut Vec<Paint>,
) {
    let mut children_clips = clips.to_vec();
    let mut after = None;
    match &node.kind {
        LayoutKind::Box { style: s, clip, .. } => {
            let mut list = list_with_clips(clips);
            let underlay_border = s.shadow.is_some() && !s.dashed && s.border_sides == [true; 4];
            // The first CSS box-shadow paints on top: hair goes below the float.
            for ring in s
                .border
                .iter()
                .filter(|b| underlay_border && b.outside && b.color[3] > 0.)
                .chain(s.rings.iter().filter(|b| s.shadow.is_some() && b.outside))
            {
                list.push(Prim::Rect {
                    rect: Rect::new(
                        node.rect.x - ring.width,
                        node.rect.y - ring.width,
                        node.rect.w + 2. * ring.width,
                        node.rect.h + 2. * ring.width,
                    ),
                    radii: s.radii.map(|r| r + ring.width),
                    fill: Color::TRANSPARENT,
                    stroke: Some((ring.width, color(ring.color, draw, opacity))),
                    inner_highlight: None,
                    shadow: None,
                });
            }
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
            // Hair and shadow are underlays; material samples the pre-shadow backdrop.
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
                    blur: s.blur,
                    rim: s.rim,
                    clips: clips.to_vec(),
                });
            }
            let mut body = list_with_clips(clips);
            let fill = if s.material == Material::Gloss {
                Color::TRANSPARENT
            } else {
                color(s.fill, draw, opacity)
            };
            let (rect, radii, stroke) = if let Some(border) = s
                .border
                .as_ref()
                .filter(|b| s.border_sides == [true; 4] && b.color[3] > 0.)
            {
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
            let dash_stroke = if s.dashed { stroke } else { None };
            let stroke = if s.dashed { None } else { stroke };
            // Outside ring and fill have different bounds.
            // A transparent border consumes layout space, while CSS backgrounds
            // still paint through it. It must not cut an empty stroke into fill.
            if s.border
                .as_ref()
                .is_some_and(|b| b.outside && b.color[3] > 0.)
            {
                body.push(Prim::Rect {
                    rect: node.rect,
                    radii: s.radii,
                    fill,
                    stroke: None,
                    inner_highlight: s.inner_highlight.map(|c| color(c, draw, opacity)),
                    shadow: None,
                });
                if !underlay_border {
                    body.push(Prim::Rect {
                        rect,
                        radii,
                        fill: Color::TRANSPARENT,
                        stroke,
                        inner_highlight: None,
                        shadow: None,
                    });
                }
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
            if let Some((width, color)) = dash_stroke {
                dashed_border(&mut body, rect, radii, width, color);
            }
            if let Some(b) = s.border.as_ref().filter(|_| s.border_sides != [true; 4]) {
                let r = node.rect;
                let w = b.width;
                for (enabled, edge) in s.border_sides.into_iter().zip([
                    Rect::new(r.x, r.y, r.w, w),
                    Rect::new(r.right() - w, r.y, w, r.h),
                    Rect::new(r.x, r.bottom() - w, r.w, w),
                    Rect::new(r.x, r.y, w, r.h),
                ]) {
                    if enabled {
                        body.rect(edge, 0., color(b.color, draw, opacity));
                    }
                }
            }
            for ring in &s.rings {
                if s.shadow.is_some() && ring.outside {
                    continue;
                }
                let w = if ring.outside { ring.width } else { 0. };
                body.push(Prim::Rect {
                    rect: Rect::new(
                        node.rect.x - w,
                        node.rect.y - w,
                        node.rect.w + w * 2.,
                        node.rect.h + w * 2.,
                    ),
                    radii: s.radii.map(|v| v + w),
                    fill: Color::TRANSPARENT,
                    stroke: Some((ring.width, color(ring.color, draw, opacity))),
                    inner_highlight: None,
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
            // The alive rim is a property of any block that carries `glow` (rim-glow §8), not
            // only of a Rim material; the legacy particle Glow keeps its own kind.
            if s.material == Material::Glow || s.material == Material::Rim || s.rim.is_some() {
                after = Some(Paint::Material {
                    rect: node.rect,
                    radii: s.radii,
                    kind: if s.material == Material::Glow {
                        Material::Glow
                    } else {
                        Material::Rim
                    },
                    color: color(s.fill, draw, opacity),
                    blur: s.blur,
                    rim: s.rim,
                    clips: clips.to_vec(),
                });
            }
        }
        LayoutKind::Text {
            runs,
            ellipsis,
            shadow,
            ..
        } => {
            let mut list = list_with_clips(clips);
            let inputs = |run: &PositionedRun| {
                (
                    [
                        run.rect.x,
                        run.rect.y + (run.line_height - run.style.size * 1.4) * 0.5,
                    ],
                    TextStyle {
                        scale: TypeScale::Custom(run.style.size),
                        weight: run.style.weight,
                        mono: run.style.mono,
                        italic: run.style.italic,
                        tracking_em: run.style.tracking_em,
                        uppercase: false,
                    },
                    // Avoid truncating a naturally sized run on float subtraction noise.
                    (*ellipsis && run.rect.right() > node.rect.right() + 0.001)
                        .then_some((node.rect.right() - run.rect.x).max(0.)),
                )
            };
            // All run shadows stay beneath all foreground text in this element.
            if let Some(shadow) = shadow {
                let mut shadow = *shadow;
                shadow.color = color(shadow.color.0, draw, opacity);
                for run in runs {
                    let (origin, style, max_width) = inputs(run);
                    list.push(Prim::TextShadow {
                        origin,
                        baseline: Some(run.baseline),
                        text: run.text.clone().into(),
                        style,
                        max_width,
                        shadow,
                    });
                }
            }
            for run in runs {
                let (origin, style, max_width) = inputs(run);
                list.push(Prim::Text {
                    origin,
                    baseline: Some(run.baseline),
                    text: run.text.clone().into(),
                    style,
                    max_width,
                    color: color(run.style.color, draw, opacity),
                });
            }
            finish(list, clips, out);
        }
        LayoutKind::Atom(atom) => {
            let mut list = list_with_clips(clips);
            for primitive in &atom.primitives {
                let mut p = primitive.clone();
                match &mut p {
                    Prim::TextShadow { shadow, .. } | Prim::PathShadow { shadow, .. } => {
                        shadow.color = draw.color(shadow.color).alpha(opacity);
                    }
                    Prim::Path { color, fill, .. } => {
                        *color = draw.color(*color).alpha(opacity);
                        if let Some(fill) = fill {
                            *fill = draw.color(*fill).alpha(opacity);
                        }
                    }
                    Prim::Text { color: c, .. }
                    | Prim::Icon { color: c, .. }
                    | Prim::Hairline { color: c, .. }
                    | Prim::Gloss { color: c, .. } => *c = draw.color(*c).alpha(opacity),
                    Prim::Rect {
                        fill,
                        stroke,
                        inner_highlight,
                        shadow,
                        ..
                    } => {
                        *fill = draw.color(*fill).alpha(opacity);
                        if let Some((_, c)) = stroke {
                            *c = draw.color(*c).alpha(opacity);
                        }
                        if let Some(c) = inner_highlight {
                            *c = draw.color(*c).alpha(opacity);
                        }
                        if let Some((_, _, c)) = shadow {
                            *c = draw.color(*c).alpha(opacity);
                        }
                    }
                    _ => {}
                }
                list.push(p);
            }
            finish(list, clips, out);
        }
        LayoutKind::ScrollView { .. } => children_clips.push(Clip {
            rect: node.rect,
            radii: [0.; 4],
        }),
        LayoutKind::Icon { name, color: rgba } => {
            let mut list = list_with_clips(clips);
            list.push(Prim::Icon {
                rect: node.rect,
                icon: name.as_str().into(),
                color: color(*rgba, draw, opacity),
            });
            finish(list, clips, out);
        }
        LayoutKind::Image { asset, radius } => {
            let mut list = list_with_clips(clips);
            list.push(Prim::Image {
                rect: node.rect,
                image: asset.as_str().into(),
                radius: *radius,
            });
            finish(list, clips, out);
        }
        LayoutKind::Rects {
            rects,
            scale,
            repeat,
        } => {
            let mut list = list_with_clips(clips);
            let (count, step) = repeat.map_or((1, [0.; 2]), |(axis, period)| {
                let step = period * scale;
                let (extent, offset) = match axis {
                    Axis::Row => (node.rect.w, [step, 0.]),
                    Axis::Column => (node.rect.h, [0., step]),
                };
                (((extent / step).ceil().max(0.)) as usize, offset)
            });
            if repeat.is_some() {
                list.clip(node.rect, [0.; 4]);
            }
            for index in 0..count {
                let offset = step.map(|v| v * index as f32);
                for (r, rgba) in rects.iter() {
                    list.push(Prim::Rect {
                        rect: Rect::new(
                            node.rect.x + r.x * scale + offset[0],
                            node.rect.y + r.y * scale + offset[1],
                            r.w * scale,
                            r.h * scale,
                        ),
                        radii: [0.; 4],
                        fill: color(*rgba, draw, opacity),
                        stroke: None,
                        inner_highlight: None,
                        shadow: None,
                    });
                }
            }
            if repeat.is_some() {
                list.unclip();
            }
            finish(list, clips, out);
        }
        LayoutKind::Path {
            points,
            stroke,
            color: rgba,
            fill,
            scale,
        } => {
            let mut list = list_with_clips(clips);
            list.push(Prim::Path {
                origin: [node.rect.x, node.rect.y],
                scale: *scale,
                points: points.clone(),
                stroke: stroke.clone(),
                color: color(*rgba, draw, opacity),
                fill: fill.map(|rgba| color(rgba, draw, opacity)),
            });
            finish(list, clips, out);
        }
        _ => {}
    }
    for child in &node.children {
        paint_node(child, draw, opacity, &children_clips, out);
    }
    if let Some(after) = after {
        out.push(after);
    }
}

fn dashed_border(out: &mut DrawList, rect: Rect, radii: [f32; 4], width: f32, color: Color) {
    if width <= 0. {
        return;
    }
    let r = Rect::new(
        rect.x + width * 0.5,
        rect.y + width * 0.5,
        (rect.w - width).max(0.),
        (rect.h - width).max(0.),
    );
    let radii = radii.map(|v| (v - width * 0.5).max(0.).min(r.w.min(r.h) * 0.5));
    let corners = [
        (
            [r.right() - radii[1], r.y + radii[1]],
            radii[1],
            -std::f32::consts::FRAC_PI_2,
        ),
        ([r.right() - radii[2], r.bottom() - radii[2]], radii[2], 0.),
        (
            [r.x + radii[3], r.bottom() - radii[3]],
            radii[3],
            std::f32::consts::FRAC_PI_2,
        ),
        (
            [r.x + radii[0], r.y + radii[0]],
            radii[0],
            std::f32::consts::PI,
        ),
    ];
    let mut points = vec![[r.x + radii[0], r.y]];
    for (center, radius, start) in corners {
        let count = ((radius * std::f32::consts::FRAC_PI_2 * 2.).ceil() as usize).max(1);
        for i in 0..=count {
            let a = start + std::f32::consts::FRAC_PI_2 * i as f32 / count as f32;
            points.push([center[0] + radius * a.cos(), center[1] + radius * a.sin()]);
        }
    }
    points.push(points[0]);
    let dash = 3. * width;
    let mut distance = 0.;
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        if len < 0.0001 {
            continue;
        }
        let mut position = 0.;
        while position < len - 0.0001 {
            let phase = distance % (2. * dash);
            let on = phase < dash;
            let step = ((if on { dash } else { 2. * dash }) - phase)
                .max(0.0001)
                .min(len - position);
            let point = |t: f32| {
                [
                    a[0] + (b[0] - a[0]) * t / len,
                    a[1] + (b[1] - a[1]) * t / len,
                ]
            };
            if on {
                out.line(point(position), point(position + step), width, color);
            }
            position += step;
            distance += step;
        }
    }
}

/// Qualify semantic names when a module composes repeated component subtrees.
pub fn namespace(element: &mut Element, prefix: &str) {
    match element {
        Element::Named { name, content } => {
            *name = format!("{prefix}{name}");
            namespace(content, prefix)
        }
        Element::Box(b) => {
            for child in &mut b.children {
                namespace(&mut child.element, prefix)
            }
        }
        Element::Stack(s) => {
            for child in &mut s.children {
                namespace(&mut child.element, prefix)
            }
        }
        Element::Filter { content, .. }
        | Element::FixedHeight { content, .. }
        | Element::Baseline { content, .. }
        | Element::InlineMiddle { content, .. }
        | Element::Overlay(Overlay { content, .. })
        | Element::ScrollView(ScrollView { content, .. }) => namespace(content, prefix),
        Element::Layers { base, overlays } => {
            namespace(base, prefix);
            for p in overlays {
                namespace(&mut p.content, prefix);
            }
        }
        _ => {}
    }
}
