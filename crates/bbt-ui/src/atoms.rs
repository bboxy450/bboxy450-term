//! L5: small, stateless painters. The caller supplies bounds, font metrics and phase.
//! Atoms emit the shared draw list; they own no layout tree, clock, input or device.
use bbt_ui_model::{
    Activity, Grade, STYLE, TOKENS, WaitKind,
    draw::{Color, DrawCtx, DrawList, Prim, TextStyle, TypeScale},
    layout::{Rect, TextMeasure, TextStyle as MeasureStyle},
};

pub struct AtomCtx<'a> {
    pub draw: DrawCtx,
    pub measure: &'a dyn TextMeasure,
    /// Physical pixels per logical pixel, for a one-device-pixel hairline.
    pub scale: f32,
}
impl AtomCtx<'_> {
    fn width(&self, text: &str, style: TextStyle) -> f32 {
        self.measure.width(
            text,
            &MeasureStyle {
                size: style.pixels(),
                weight: style.weight,
                mono: style.mono,
                tracking_em: style.tracking_em,
            },
        )
    }
    fn label(&self, out: &mut DrawList, r: Rect, text: &str, style: TextStyle, color: Color) {
        // The chrome text primitive's line box is 1.4 em; centre that line, not its ink.
        let w = self.width(text, style);
        out.push(Prim::Text {
            origin: [
                r.x + (r.w - w) * 0.5,
                r.y + (r.h - style.pixels() * 1.4) * 0.5,
            ],
            text: text.into(),
            style,
            max_width: None,
            color,
        });
    }
}

pub trait Atom {
    fn paint(&self, rect: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList);
}

pub struct Hairline;
impl Atom for Hairline {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let width = 1. / ctx.scale.max(f32::EPSILON);
        // Centre on a physical pixel so a horizontal/vertical rule is crisp.
        let y = ((r.y * ctx.scale).floor() + 0.5) / ctx.scale;
        out.line([r.x, y], [r.right(), y], width, ctx.draw.hairline());
    }
}

pub struct Icon<'a> {
    pub name: &'a str,
    pub color: Color,
}
impl Atom for Icon<'_> {
    fn paint(&self, rect: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        out.push(Prim::Icon {
            rect,
            icon: self.name.into(),
            color: ctx.draw.color(self.color),
        });
    }
}

pub struct Glyph {
    pub activity: Activity,
    pub grade: Grade,
    pub seat_color: Color,
}
impl Atom for Glyph {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let color = ctx
            .draw
            .color(match self.activity {
                Activity::Active => self.seat_color,
                Activity::Idle => Color::hex(TOKENS.color.state.idle),
                Activity::Waiting(_) => Color::hex(TOKENS.color.state.waiting),
                Activity::Errored => Color::hex(TOKENS.color.state.errored),
            })
            .alpha(if self.grade == Grade::Inferred {
                0.72
            } else {
                1.
            });
        let symbol = match self.activity {
            Activity::Active => "●",
            Activity::Idle => "○",
            Activity::Waiting(_) => "◐",
            Activity::Errored => "✕",
        };
        ctx.label(
            out,
            r,
            symbol,
            TextStyle {
                scale: TypeScale::Custom(r.h),
                mono: true,
                ..Default::default()
            },
            color,
        );
        let hair = STYLE.material.hairline_px as f32;
        if self.grade == Grade::Shim {
            out.push(Prim::Rect {
                rect: Rect::new(r.x - hair, r.y - hair, r.w + 2. * hair, r.h + 2. * hair),
                radii: [r.h; 4],
                fill: Color::TRANSPARENT,
                stroke: Some((hair, color.alpha(0.55))),
                inner_highlight: None,
                shadow: None,
            });
        }
        if self.grade == Grade::Inferred {
            let radius = r.h * 0.5 + 1. + hair * 0.5;
            let count = ((std::f32::consts::TAU * radius / 6.).round() as usize).max(4);
            for n in 0..count {
                for part in 0..3 {
                    let a = std::f32::consts::TAU * (n as f32 + part as f32 / 6.) / count as f32;
                    let b = a + std::f32::consts::TAU / (6. * count as f32);
                    out.line(
                        [
                            r.x + r.w * 0.5 + a.cos() * radius,
                            r.y + r.h * 0.5 + a.sin() * radius,
                        ],
                        [
                            r.x + r.w * 0.5 + b.cos() * radius,
                            r.y + r.h * 0.5 + b.sin() * radius,
                        ],
                        hair,
                        color.alpha(0.7),
                    );
                }
            }
        }
        if let Activity::Waiting(kind) = self.activity {
            let text = match kind {
                WaitKind::Permission => "⚿",
                WaitKind::Question => "?",
                WaitKind::Auth => "⚷",
            };
            // Glyph family geometry: lab.css .glyph .kind (7px, right -6, top -5).
            let style = TextStyle {
                scale: TypeScale::Custom(7.),
                weight: 700,
                mono: true,
                ..Default::default()
            };
            let w = ctx.width(text, style) + 2.;
            let mark = Rect::new(r.right() + 6. - w, r.y - 5., w, 7.);
            let mut background = ctx.draw.color(Color::hex(ctx.draw.theme.palette().bg));
            background.0[3] = color.0[3];
            out.rect(mark, 3.5, background);
            ctx.label(out, mark, text, style, color);
        }
    }
}

pub struct Badge<'a> {
    pub label: &'a str,
    pub selected: bool,
}
impl Atom for Badge<'_> {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let p = ctx.draw.theme.palette();
        let alpha = if self.selected {
            STYLE.tint.badge_selected_alpha
        } else {
            STYLE.tint.badge_alpha
        } as f32;
        out.rect(
            r,
            STYLE.radius.badge as f32,
            ctx.draw.color(Color::hex(p.fg_dim).alpha(alpha)),
        );
        ctx.label(
            out,
            r,
            self.label,
            TextStyle {
                scale: TypeScale::Badge,
                mono: true,
                ..Default::default()
            },
            if self.selected {
                ctx.draw.fg()
            } else {
                ctx.draw.muted()
            },
        );
    }
}

pub struct Chip<'a> {
    pub label: &'a str,
    pub color: Color,
    pub outlined: bool,
}
impl Atom for Chip<'_> {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let color = ctx.draw.color(self.color);
        out.push(Prim::Rect {
            rect: r,
            radii: [STYLE.radius.tag as f32; 4],
            fill: color.alpha(STYLE.tint.badge_alpha as f32),
            stroke: self
                .outlined
                .then_some((STYLE.material.hairline_px as f32, color.alpha(0.45))),
            inner_highlight: None,
            shadow: None,
        });
        ctx.label(
            out,
            r,
            self.label,
            TextStyle {
                scale: TypeScale::Custom(STYLE.r#type.micro as f32),
                mono: true,
                ..Default::default()
            },
            color,
        );
    }
}

pub struct Pill<'a> {
    pub items: &'a [(&'a str, &'a str)],
}
impl Atom for Pill<'_> {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        out.push(Prim::Rect {
            rect: r,
            radii: [STYLE.radius.pill as f32; 4],
            fill: ctx.draw.color(
                Color::hex(ctx.draw.theme.palette().surface)
                    .alpha(STYLE.material.panel_alpha as f32),
            ),
            stroke: Some((STYLE.material.hairline_px as f32, ctx.draw.hairline())),
            inner_highlight: Some(Color::WHITE.alpha(STYLE.material.inner_highlight_alpha as f32)),
            shadow: crate::style::resolve_shadow(crate::Shadow::Pill)
                .expect("compiled Style pill shadow")
                .map(|s| (s.offset[1], s.blur, ctx.draw.color(Color(s.color)))),
        });
        let w = r.w / self.items.len().max(1) as f32;
        for (i, (label, key)) in self.items.iter().enumerate() {
            let x = r.x + i as f32 * w;
            if i > 0 {
                out.line(
                    [x, r.y + 6.],
                    [x, r.bottom() - 6.],
                    STYLE.material.hairline_px as f32,
                    ctx.draw.hairline(),
                );
            }
            ctx.label(
                out,
                Rect::new(x, r.y, w, r.h),
                &format!("{label}  {key}"),
                TextStyle {
                    scale: TypeScale::Caption,
                    ..Default::default()
                },
                ctx.draw.fg(),
            );
        }
    }
}

pub struct Swatch {
    pub color: Color,
    pub hovered: bool,
}
impl Atom for Swatch {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let scale = if self.hovered { 1.25 } else { 1. };
        let r = Rect::new(
            r.x - r.w * (scale - 1.) * 0.5,
            r.y - r.h * (scale - 1.) * 0.5,
            r.w * scale,
            r.h * scale,
        );
        out.push(Prim::Rect {
            rect: r,
            radii: [r.h; 4],
            fill: ctx.draw.color(self.color),
            stroke: Some((1.5, ctx.draw.color(Color::WHITE.alpha(0.85)))),
            inner_highlight: None,
            shadow: None,
        });
    }
}

/// Explicit phase in turns; no timer or event state is owned by the atom.
pub struct Spinner {
    pub phase: f32,
}
impl Atom for Spinner {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let radius = r.w.min(r.h) * 0.36;
        for n in 0..12 {
            let a = (self.phase + n as f32 / 12.) * std::f32::consts::TAU;
            out.line(
                [
                    r.x + r.w * 0.5 + a.cos() * radius * 0.55,
                    r.y + r.h * 0.5 + a.sin() * radius * 0.55,
                ],
                [
                    r.x + r.w * 0.5 + a.cos() * radius,
                    r.y + r.h * 0.5 + a.sin() * radius,
                ],
                STYLE.material.hairline_px as f32,
                ctx.draw.muted().alpha((n + 1) as f32 / 12.),
            );
        }
    }
}

pub struct Checkerboard;
impl Atom for Checkerboard {
    fn paint(&self, r: Rect, ctx: &AtomCtx<'_>, out: &mut DrawList) {
        let size = STYLE.spacing.grid as f32;
        let a = ctx.draw.color(Color::hex(ctx.draw.theme.palette().bg));
        let b = ctx
            .draw
            .color(Color::hex(ctx.draw.theme.palette().surface_2));
        for y in 0..(r.h / size).ceil().max(0.) as u32 {
            for x in 0..(r.w / size).ceil().max(0.) as u32 {
                out.rect(
                    Rect::new(
                        r.x + x as f32 * size,
                        r.y + y as f32 * size,
                        size.min(r.w - x as f32 * size),
                        size.min(r.h - y as f32 * size),
                    ),
                    0.,
                    if (x + y) % 2 == 0 { a } else { b },
                );
            }
        }
    }
}
