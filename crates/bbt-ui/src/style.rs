use bbt_ui_model::{Density, STYLE as DESIGN, TOKENS, Theme, draw::Color as DrawColor};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, &'static str>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Card,
    Row,
    Band,
    Pill,
    Chip,
    Modal,
    Strip,
    Input,
    ButtonPrimary,
    ButtonGhost,
    PaneGlow,
}
impl Preset {
    // The original ten references plus the explicit Style/Box-owned filled glow case.
    pub const ALL: [Self; 11] = [
        Self::Card,
        Self::Row,
        Self::Band,
        Self::Pill,
        Self::Chip,
        Self::Modal,
        Self::Strip,
        Self::Input,
        Self::ButtonPrimary,
        Self::ButtonGhost,
        Self::PaneGlow,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::Row => "row",
            Self::Band => "band",
            Self::Pill => "pill",
            Self::Chip => "chip",
            Self::Modal => "modal",
            Self::Strip => "strip",
            Self::Input => "input",
            Self::ButtonPrimary => "button-primary",
            Self::ButtonGhost => "button-ghost",
            Self::PaneGlow => "pane-glow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Flat,
    Gloss,
    Frosted,
    Glow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Radius {
    Panel,
    Card,
    Row,
    Tile,
    Badge,
    Pill,
    Tag,
    Mark,
    Chip,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    Px(f32),
    Spacing(u8),
    Radius(Radius),
    RowHeight,
    PadX,
    StripHeight,
    Hairline,
}
impl Length {
    pub fn resolve(self, density: Density) -> Result<f32> {
        let s = DESIGN.spacing;
        let r = DESIGN.radius;
        let v = match self {
            Self::Px(v) => v,
            Self::Spacing(n) => {
                (match n {
                    1 => s.s1,
                    2 => s.s2,
                    3 => s.s3,
                    4 => s.s4,
                    5 => s.s5,
                    6 => s.s6,
                    7 => s.s7,
                    8 => s.s8,
                    9 => s.s9,
                    10 => s.s10,
                    _ => return Err("unknown spacing token"),
                }) as f32
            }
            Self::Radius(n) => {
                (match n {
                    Radius::Panel => r.panel,
                    Radius::Card => r.card,
                    Radius::Row => r.row,
                    Radius::Tile => r.tile,
                    Radius::Badge => r.badge,
                    Radius::Pill => r.pill,
                    Radius::Tag => r.tag,
                    Radius::Mark => r.mark,
                    Radius::Chip => TOKENS.radius.chip,
                }) as f32
            }
            Self::RowHeight => density.tokens().row_h as f32,
            Self::PadX => density.tokens().pad_x as f32,
            Self::StripHeight => TOKENS.strip.height as f32,
            Self::Hairline => DESIGN.material.hairline_px as f32,
        };
        if v.is_finite() && v >= 0. {
            Ok(v)
        } else {
            Err("style length must be finite and nonnegative")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Transparent,
    Background,
    Surface,
    Surface2,
    Line,
    Foreground,
    Muted,
    Accent,
    OnAccent,
    Seat,
    Swarm,
    Rgba([f32; 4]),
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alpha {
    Value(f32),
    Hairline,
    Badge,
    Selection,
    Panel,
}
impl Alpha {
    fn resolve(self) -> Result<f32> {
        unit(match self {
            Self::Value(v) => v,
            Self::Hairline => DESIGN.material.hairline_alpha as f32,
            Self::Badge => DESIGN.tint.badge_alpha as f32,
            Self::Selection => DESIGN.tint.selection_alpha as f32,
            Self::Panel => DESIGN.material.panel_alpha as f32,
        })
    }
}
fn unit(v: f32) -> Result<f32> {
    if v.is_finite() && (0.0..=1.).contains(&v) {
        Ok(v)
    } else {
        Err("style alpha must be between zero and one")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Paint {
    pub color: Color,
    pub alpha: Alpha,
}
impl Paint {
    pub const fn solid(color: Color) -> Self {
        Self {
            color,
            alpha: Alpha::Value(1.),
        }
    }
    pub fn resolve(self, ctx: Context) -> Result<[f32; 4]> {
        let p = ctx.theme.palette();
        let mut c = match self.color {
            Color::Transparent => [0.; 4],
            Color::Background => DrawColor::hex(p.bg).0,
            Color::Surface => DrawColor::hex(p.surface).0,
            Color::Surface2 => DrawColor::hex(p.surface_2).0,
            Color::Line => DrawColor::hex(p.line).0,
            Color::Foreground => DrawColor::hex(p.fg).0,
            Color::Muted => DrawColor::hex(p.fg_dim).0,
            Color::Accent => DrawColor::hex(p.accent).0,
            Color::OnAccent => DrawColor::hex(TOKENS.color.ui.on_accent).0,
            Color::Seat => ctx.seat,
            Color::Swarm => ctx.swarm,
            Color::Rgba(c) => c,
        };
        for v in c {
            unit(v)?;
        }
        c[3] *= self.alpha.resolve()?;
        Ok(c)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Border {
    pub width: Length,
    pub paint: Paint,
    pub outside: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shadow {
    None,
    Floating,
    Pill,
    Custom { blur: f32, color: [f32; 4] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeStep {
    Micro,
    Tiny,
    Small,
    Note,
    Caption,
    Label,
    Section,
    Row,
    Body,
    Kind,
    Subtitle,
    Title,
    Input,
    Badge,
}
impl TypeStep {
    pub fn pixels(self) -> f32 {
        let t = DESIGN.r#type;
        (match self {
            Self::Micro => t.micro,
            Self::Tiny => t.tiny,
            Self::Small => t.small,
            Self::Note => t.note,
            Self::Caption => t.caption,
            Self::Label => t.label,
            Self::Section => t.section,
            Self::Row => t.row,
            Self::Body => t.body,
            Self::Kind => t.kind,
            Self::Subtitle => t.subtitle,
            Self::Title => t.title,
            Self::Input => t.input,
            Self::Badge => t.badge,
        }) as f32
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub step: TypeStep,
    /// Explicit component geometry, e.g. density-sized seat labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    pub weight: u16,
    pub paint: Paint,
    pub tracking_em: f32,
    /// CSS .ui-text line-height, pending a named design token.
    pub line_height: f32,
    pub mono: bool,
    pub uppercase: bool,
}
impl Default for Text {
    fn default() -> Self {
        Self {
            step: TypeStep::Body,
            size: None,
            weight: 400,
            paint: Paint::solid(Color::Foreground),
            tracking_em: 0.,
            line_height: 1.35,
            mono: false,
            uppercase: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dim {
    pub saturate: f32,
    pub opacity: f32,
    pub wash: f32,
}
impl Default for Dim {
    fn default() -> Self {
        Self {
            saturate: TOKENS.dim.saturate as f32,
            opacity: TOKENS.dim.opacity as f32,
            wash: TOKENS.dim.wash as f32,
        }
    }
}

/// None means inherit. Nested records replace their whole value; a zero-width
/// border, Shadow::None or Color::Transparent explicitly clears that feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Overrides {
    pub material: Option<Material>,
    pub blur: Option<Blur>,
    pub fill: Option<Paint>,
    pub radii: Option<[Length; 4]>,
    pub border: Option<Border>,
    /// Top, right, bottom, left. Table headers use a bottom rule only.
    pub border_sides: Option<[bool; 4]>,
    pub dashed: Option<bool>,
    /// Ordered outline rings, independent of the box's layout border.
    pub rings: Option<Vec<Border>>,
    pub shadow: Option<Shadow>,
    pub height: Option<Length>,
    pub width: Option<Length>,
    pub padding: Option<[Length; 4]>,
    pub text: Option<Text>,
    pub opacity: Option<f32>,
    pub dim: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub tokens: Preset,
    #[serde(default)]
    pub overrides: Overrides,
}
impl Style {
    pub fn new(tokens: Preset) -> Self {
        Self {
            tokens,
            overrides: Overrides::default(),
        }
    }
    pub fn resolve(&self, ctx: Context) -> Result<ResolvedStyle> {
        let mut base = defaults(self.tokens);
        let o = &self.overrides;
        if let Some(v) = o.material {
            base.material = v;
        }
        if let Some(v) = o.fill {
            base.fill = v;
        }
        if let Some(v) = o.radii {
            base.radii = v;
        }
        if let Some(v) = o.border {
            base.border = Some(v);
        }
        if let Some(v) = o.shadow {
            base.shadow = v;
        }
        if let Some(v) = o.height {
            base.height = Some(v);
        }
        if let Some(v) = o.padding {
            base.padding = v;
        }
        if let Some(v) = o.text {
            base.text = v;
        }
        if base.text.size.is_some_and(|s| !s.is_finite() || s <= 0.)
            || !base.text.tracking_em.is_finite()
            || !base.text.line_height.is_finite()
            || base.text.line_height <= 0.
            || !(1..=1000).contains(&base.text.weight)
        {
            return Err("invalid style text");
        }
        let opacity = unit(o.opacity.unwrap_or(1.))?;
        let mut radii = [0.; 4];
        let mut padding = [0.; 4];
        for i in 0..4 {
            radii[i] = base.radii[i].resolve(ctx.density)?;
            padding[i] = base.padding[i].resolve(ctx.density)?;
        }
        let border = base
            .border
            .map(|b| {
                Ok(ResolvedBorder {
                    width: b.width.resolve(ctx.density)?,
                    color: b.paint.resolve(ctx)?,
                    outside: b.outside,
                })
            })
            .transpose()?;
        let fill = if self.tokens == Preset::Row && ctx.selected && o.fill.is_none() {
            Paint {
                color: Color::Foreground,
                alpha: Alpha::Selection,
            }
        } else {
            base.fill
        };
        Ok(ResolvedStyle {
            material: base.material,
            blur: o
                .blur
                .map(|b| b.resolve(ctx.density))
                .transpose()?
                .or_else(|| {
                    (base.material == Material::Frosted).then_some(ResolvedBlur {
                        radius: DESIGN.material.panel_blur_px as f32,
                        saturation: DESIGN.material.panel_saturate as f32,
                    })
                }),
            fill: fill.resolve(ctx)?,
            radii,
            border,
            border_sides: o.border_sides.unwrap_or([true; 4]),
            dashed: o.dashed.unwrap_or(false),
            rings: o
                .rings
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|b| {
                    Ok(ResolvedBorder {
                        width: b.width.resolve(ctx.density)?,
                        color: b.paint.resolve(ctx)?,
                        outside: b.outside,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            shadow: resolve_shadow(base.shadow)?,
            height: base.height.map(|n| n.resolve(ctx.density)).transpose()?,
            width: o.width.map(|n| n.resolve(ctx.density)).transpose()?,
            padding,
            text: ResolvedText {
                size: base.text.size.unwrap_or(base.text.step.pixels()),
                weight: base.text.weight,
                color: base.text.paint.resolve(ctx)?,
                tracking_em: base.text.tracking_em,
                line_height: base.text.line_height,
                mono: base.text.mono,
                uppercase: base.text.uppercase,
            },
            opacity,
            dim: o.dim.unwrap_or(false).then(Dim::default),
            inner_highlight: if self.tokens == Preset::Modal {
                Some([1., 1., 1., DESIGN.material.inner_highlight_alpha as f32])
            } else {
                None
            },
            focus_ring: if self.tokens == Preset::Input && ctx.focused {
                Some(ResolvedBorder {
                    width: DESIGN.material.hairline_px as f32,
                    color: Paint::solid(Color::Accent).resolve(ctx)?,
                    outside: true,
                })
            } else {
                None
            },
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub theme: Theme,
    pub density: Density,
    pub seat: [f32; 4],
    pub swarm: [f32; 4],
    pub selected: bool,
    pub focused: bool,
}
impl Context {
    pub fn new(theme: Theme, density: Density) -> Self {
        Self {
            theme,
            density,
            seat: DrawColor::hex(theme.palette().accent).0,
            swarm: DrawColor::hex(TOKENS.color.role.oracle).0,
            selected: false,
            focused: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedBorder {
    pub width: f32,
    pub color: [f32; 4],
    pub outside: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedShadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: [f32; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedText {
    pub size: f32,
    pub weight: u16,
    pub color: [f32; 4],
    pub tracking_em: f32,
    pub line_height: f32,
    pub mono: bool,
    pub uppercase: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blur {
    pub radius: Length,
    pub saturation: f32,
}
impl Blur {
    fn resolve(self, density: Density) -> Result<ResolvedBlur> {
        if !self.saturation.is_finite() || !(0. ..=4.).contains(&self.saturation) {
            return Err("invalid backdrop saturation");
        }
        let radius = self.radius.resolve(density)?;
        if radius > 256. {
            return Err("invalid backdrop blur radius");
        }
        Ok(ResolvedBlur {
            radius,
            saturation: self.saturation,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResolvedBlur {
    pub radius: f32,
    pub saturation: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedStyle {
    pub material: Material,
    pub blur: Option<ResolvedBlur>,
    pub fill: [f32; 4],
    pub radii: [f32; 4],
    pub border: Option<ResolvedBorder>,
    pub border_sides: [bool; 4],
    pub dashed: bool,
    pub rings: Vec<ResolvedBorder>,
    pub shadow: Option<ResolvedShadow>,
    pub height: Option<f32>,
    pub width: Option<f32>,
    pub padding: [f32; 4],
    pub text: ResolvedText,
    pub opacity: f32,
    pub dim: Option<Dim>,
    pub inner_highlight: Option<[f32; 4]>,
    pub focus_ring: Option<ResolvedBorder>,
}

struct Base {
    material: Material,
    fill: Paint,
    radii: [Length; 4],
    border: Option<Border>,
    shadow: Shadow,
    height: Option<Length>,
    padding: [Length; 4],
    text: Text,
}
fn defaults(p: Preset) -> Base {
    use Length::{Px, Spacing};
    let mut b = Base {
        material: Material::Flat,
        fill: Paint::solid(Color::Transparent),
        radii: [Px(0.); 4],
        border: None,
        shadow: Shadow::None,
        height: None,
        padding: [Px(0.); 4],
        text: Text::default(),
    };
    let mut radius = Radius::Row;
    let hair = Border {
        width: Length::Hairline,
        paint: Paint {
            color: Color::Line,
            alpha: Alpha::Hairline,
        },
        outside: false,
    };
    let xy = |y, x| [y, x, y, x];
    match p {
        Preset::Card => {
            radius = Radius::Card;
            b.fill = Paint {
                color: Color::Surface,
                alpha: Alpha::Value(DESIGN.material.card_alpha as f32),
            };
            b.border = Some(Border {
                outside: true,
                ..hair
            });
            b.padding = xy(Spacing(6), Spacing(7));
        }
        Preset::Row => {
            b.height = Some(Length::RowHeight);
            b.padding = xy(Px(0.), Length::PadX);
            b.text.step = TypeStep::Row;
        }
        Preset::Band => {
            radius = Radius::Chip;
            b.material = Material::Gloss;
            b.fill = Paint::solid(Color::Swarm);
            b.height = Some(Length::StripHeight);
            b.padding = xy(Px(0.), Length::PadX);
            b.text = Text {
                step: TypeStep::Small,
                weight: 600,
                paint: Paint::solid(Color::OnAccent),
                tracking_em: 0.22,
                uppercase: true,
                ..Text::default()
            };
        }
        Preset::Pill | Preset::Chip => {
            radius = if p == Preset::Pill {
                Radius::Pill
            } else {
                Radius::Tag
            };
            b.fill = Paint {
                color: Color::Muted,
                alpha: Alpha::Badge,
            };
            b.padding = xy(Px(0.), Spacing(if p == Preset::Pill { 3 } else { 2 }));
            b.text.step = if p == Preset::Pill {
                TypeStep::Tiny
            } else {
                TypeStep::Micro
            };
            b.text.mono = p == Preset::Chip;
        }
        Preset::Modal => {
            radius = Radius::Panel;
            b.material = Material::Frosted;
            b.fill = Paint {
                color: Color::Surface,
                alpha: Alpha::Panel,
            };
            b.border = Some(Border {
                outside: true,
                ..hair
            });
            b.shadow = Shadow::Floating;
            b.padding = xy(Spacing(8), Spacing(9));
        }
        Preset::Strip => {
            b.fill = Paint::solid(Color::Seat);
            b.height = Some(Length::StripHeight);
            b.padding = xy(Px(0.), Spacing(4));
            b.text = Text {
                step: TypeStep::Caption,
                weight: 600,
                paint: Paint::solid(Color::OnAccent),
                ..Text::default()
            };
        }
        Preset::Input => {
            b.fill = Paint::solid(Color::Background);
            b.border = Some(hair);
            b.padding = xy(Spacing(4), Spacing(5));
            b.text.paint = Paint::solid(Color::Muted);
        }
        Preset::ButtonPrimary | Preset::ButtonGhost => {
            radius = Radius::Pill;
            b.padding = xy(Spacing(3), Spacing(8));
            b.text.step = TypeStep::Row;
            if p == Preset::ButtonPrimary {
                b.fill = Paint::solid(Color::Accent);
                b.text.weight = 600;
                b.text.paint = Paint::solid(Color::OnAccent);
            } else {
                b.border = Some(hair);
            }
        }
        Preset::PaneGlow => {
            b.material = Material::Glow;
            b.fill = Paint::solid(Color::Surface);
            b.border = Some(Border {
                width: Px(1.5),
                paint: Paint::solid(Color::Seat),
                outside: false,
            });
            b.padding = xy(Spacing(6), Spacing(7));
        }
    }
    if p != Preset::Strip {
        b.radii = [Length::Radius(radius); 4];
    }
    b
}

pub(crate) fn resolve_shadow(token: Shadow) -> Result<Option<ResolvedShadow>> {
    // These are compile-time design strings; parse their CSS data, never copy
    // their numeric values into a second source of truth.
    let source = match token {
        Shadow::None => return Ok(None),
        Shadow::Custom { blur, color } => {
            if !blur.is_finite() || blur < 0. {
                return Err("invalid custom shadow blur");
            }
            for channel in color {
                unit(channel)?;
            }
            return Ok(Some(ResolvedShadow {
                offset: [0.; 2],
                blur,
                color,
            }));
        }
        Shadow::Floating => DESIGN.shadow.floating,
        Shadow::Pill => DESIGN.shadow.pill,
    };
    let parts: Vec<_> = source.split_whitespace().collect();
    if parts.len() != 4 {
        return Err("unsupported design shadow token");
    }
    let n = |s: &str| {
        s.trim_end_matches("px")
            .parse::<f32>()
            .map_err(|_| "invalid shadow token")
    };
    let rgba = parts[3]
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("invalid shadow colour")?;
    let c = rgba.split(',').map(n).collect::<Result<Vec<_>>>()?;
    if c.len() != 4 {
        return Err("invalid shadow colour");
    }
    Ok(Some(ResolvedShadow {
        offset: [n(parts[0])?, n(parts[1])?],
        blur: n(parts[2])?,
        color: [c[0] / 255., c[1] / 255., c[2] / 255., unit(c[3])?],
    }))
}
