//! Seat and task marks in the lab's centred, sixteen-unit coordinate system.
//! They are geometry, independent of the host font and its baseline metrics.
use crate::{Paint, components as c};
use bbt_ui_model::{
    Activity, Grade, TOKENS, WaitKind,
    draw::{
        Color, Draw, DrawCtx, DrawList, LineCap, LineJoin, PathCommand, PathStroke, Prim,
        TextShadow, TextStyle, TypeScale,
    },
    layout::Rect,
};
use std::sync::Arc;

/// The shared visual vocabulary for seat activity and work-item status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    Open,
    Quarter,
    Half,
    ThreeQuarter,
    Full,
    LeftHalf,
    Check,
    Cross,
    Blocked,
}

pub(crate) struct Geometry {
    pub points: Arc<[PathCommand]>,
    pub stroke: PathStroke,
    pub filled: bool,
}

impl Mark {
    /// The lab's seat activity mapping, before grade and kind annotations.
    pub fn seat(activity: Activity) -> Self {
        match activity {
            Activity::Active => Self::Full,
            Activity::Idle => Self::Open,
            Activity::Waiting(_) => Self::LeftHalf,
            Activity::Errored => Self::Cross,
        }
    }

    /// Unknown frontmatter statuses retain the lab's open-circle fallback.
    pub fn task(status: &str) -> Self {
        match status {
            "claimed" => Self::Quarter,
            "running" => Self::Half,
            "reported" => Self::ThreeQuarter,
            "accepted" => Self::Full,
            "done" => Self::Check,
            "rejected" => Self::Cross,
            "blocked" => Self::Blocked,
            _ => Self::Open,
        }
    }

    /// A square retained element. The caller continues to own its layout slot.
    pub fn element(self, size: f32, paint: Paint) -> c::Element {
        let geometry = self.geometry();
        let factor = size / 16.;
        let point = |p: [f32; 2]| p.map(|v| (v + 8.) * factor);
        let points: Vec<_> = geometry
            .points
            .iter()
            .map(|command| match *command {
                PathCommand::Move(p) => PathCommand::Move(point(p)),
                PathCommand::Line(p) => PathCommand::Line(point(p)),
                PathCommand::Cubic(p) => PathCommand::Cubic(p.map(point)),
            })
            .collect();
        c::Element::Path(c::Path {
            size: [size; 2],
            points: points.into(),
            stroke: PathStroke {
                width: geometry.stroke.width * factor,
                ..geometry.stroke
            },
            paint,
            fill: geometry.filled.then_some(paint),
            opacity: 1.,
        })
    }

    /// Paint at the centre of an existing box without changing that box.
    pub fn paint(self, rect: Rect, color: Color, out: &mut DrawList) {
        self.paint_with_shadow(rect, color, None, out);
    }

    /// Shadow the combined mark coverage; grade and ancestor alpha are supplied
    /// once by the caller, and never affect its layout slot.
    pub fn paint_with_shadow(
        self,
        rect: Rect,
        color: Color,
        shadow: Option<TextShadow>,
        out: &mut DrawList,
    ) {
        let side = rect.w.min(rect.h);
        if side <= 0. {
            return;
        }
        let geometry = self.geometry();
        let origin = [rect.x + rect.w * 0.5, rect.y + rect.h * 0.5];
        if let Some(shadow) = shadow {
            out.push(Prim::PathShadow {
                origin,
                scale: side / 16.,
                points: geometry.points.clone(),
                stroke: geometry.stroke.clone(),
                filled: geometry.filled,
                shadow,
            });
        }
        out.push(Prim::Path {
            origin,
            scale: side / 16.,
            points: geometry.points,
            stroke: geometry.stroke,
            color,
            fill: geometry.filled.then_some(color),
        });
    }

    pub(crate) fn geometry(self) -> Geometry {
        let mut points = Vec::new();
        let mut stroke = PathStroke {
            width: 0.,
            ..Default::default()
        };
        let filled = match self {
            Self::Open | Self::Blocked => {
                circle(&mut points, 0, 0);
                if self == Self::Blocked {
                    // The lab's butt-capped slash, width 1.5, unioned with the
                    // ring so dim/grade alpha is applied to its coverage once.
                    let n = 0.75 * std::f32::consts::FRAC_1_SQRT_2;
                    let a = [-4.24, 4.24];
                    let b = [4.24, -4.24];
                    let corners = [
                        [a[0] - n, a[1] - n],
                        [b[0] - n, b[1] - n],
                        [b[0] + n, b[1] + n],
                        [a[0] + n, a[1] + n],
                    ];
                    points.push(PathCommand::Move(corners[0]));
                    points.extend(corners[1..].iter().copied().map(PathCommand::Line));
                    points.push(PathCommand::Line(corners[0]));
                }
                true
            }
            Self::Quarter | Self::Half | Self::ThreeQuarter | Self::Full | Self::LeftHalf => {
                let (quarters, from) = match self {
                    Self::Quarter => (1, 0),
                    Self::Half => (2, 0),
                    Self::ThreeQuarter => (3, 0),
                    Self::Full => (4, 0),
                    Self::LeftHalf => (2, 2),
                    _ => unreachable!(),
                };
                circle(&mut points, quarters, from);
                true
            }
            Self::Cross => {
                points.extend([
                    PathCommand::Move([-4.2, -4.2]),
                    PathCommand::Line([4.2, 4.2]),
                    PathCommand::Move([4.2, -4.2]),
                    PathCommand::Line([-4.2, 4.2]),
                ]);
                stroke.width = 1.8;
                stroke.cap = LineCap::Round;
                false
            }
            Self::Check => {
                points.extend([
                    PathCommand::Move([-4.8, 0.25]),
                    PathCommand::Line([-1.5, 3.55]),
                    PathCommand::Line([4.8, -3.55]),
                ]);
                stroke.width = 1.8;
                stroke.cap = LineCap::Round;
                stroke.join = LineJoin::Round;
                false
            }
        };
        Geometry {
            points: points.into(),
            stroke,
            filled,
        }
    }
}

fn cardinal(quarter: i32, radius: f32) -> [f32; 2] {
    match quarter.rem_euclid(4) {
        0 => [0., -radius],
        1 => [radius, 0.],
        2 => [0., radius],
        _ => [-radius, 0.],
    }
}

fn arc(points: &mut Vec<PathCommand>, radius: f32, from: i32, quarters: i32) {
    const KAPPA: f32 = 0.552_284_8;
    let direction = quarters.signum();
    for step in 0..quarters.abs() {
        let a = from + step * direction;
        let b = a + direction;
        let p = cardinal(a, radius);
        let q = cardinal(b, radius);
        let t0 = cardinal(a + 1, radius);
        let t1 = cardinal(b + 1, radius);
        let k = KAPPA * direction as f32;
        points.push(PathCommand::Cubic([
            [p[0] + t0[0] * k, p[1] + t0[1] * k],
            [q[0] - t1[0] * k, q[1] - t1[1] * k],
            q,
        ]));
    }
}

fn circle(points: &mut Vec<PathCommand>, quarters: i32, from: i32) {
    // A radius-6 ring with stroke 1.5 is the annulus [5.25, 6.75].
    // Subtract only the unfilled inner sector. This is one nonzero-winding
    // coverage union, avoiding darker overlaps when the mark is translucent.
    points.push(PathCommand::Move(cardinal(0, 6.75)));
    arc(points, 6.75, 0, 4);
    if quarters == 0 {
        points.push(PathCommand::Move(cardinal(0, 5.25)));
        arc(points, 5.25, 0, -4);
    } else if quarters < 4 {
        points.push(PathCommand::Move([0., 0.]));
        points.push(PathCommand::Line(cardinal(from, 5.25)));
        arc(points, 5.25, from, quarters - 4);
        points.push(PathCommand::Line([0., 0.]));
    }
}

/// Legacy storybook painter, sharing the same mark geometry as retained UI.
pub struct StateGlyph {
    pub activity: Activity,
    pub grade: Grade,
    pub color: Color,
}
impl Draw for StateGlyph {
    fn draw(&self, r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
        let color = match self.activity {
            Activity::Idle => Color::hex(TOKENS.color.state.idle),
            Activity::Waiting(_) => Color::hex(TOKENS.color.state.waiting),
            Activity::Errored => Color::hex(TOKENS.color.state.errored),
            Activity::Active => self.color,
        };
        let color = ctx.color(color).alpha(if self.grade == Grade::Inferred {
            0.72
        } else {
            1.0
        });
        Mark::seat(self.activity).paint(r, color, out);
        if self.grade == Grade::Shim {
            out.push(Prim::Rect {
                rect: r,
                radii: [r.h; 4],
                fill: Color::TRANSPARENT,
                stroke: Some((1.0, color.alpha(0.55))),
                inner_highlight: None,
                shadow: None,
            });
        }
        if self.grade == Grade::Inferred {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let b = a + 0.42;
                let rad = r.h * 0.55;
                out.line(
                    [
                        r.x + r.w * 0.5 + a.cos() * rad,
                        r.y + r.h * 0.5 + a.sin() * rad,
                    ],
                    [
                        r.x + r.w * 0.5 + b.cos() * rad,
                        r.y + r.h * 0.5 + b.sin() * rad,
                    ],
                    1.0,
                    color,
                );
            }
        }
        if let Activity::Waiting(kind) = self.activity {
            let label = match kind {
                WaitKind::Permission => "⚿",
                WaitKind::Question => "?",
                WaitKind::Auth => "⚷",
            };
            out.text(
                Rect::new(r.right() - 2.0, r.y - 5.0, 10.0, 9.0),
                label,
                TextStyle {
                    scale: TypeScale::Custom(7.0),
                    weight: 700,
                    ..Default::default()
                },
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec gui-components §3.2
    #[test]
    fn glyphs_encode_grade_and_wait_kind_and_clips_balance() {
        let ctx = DrawCtx::new(bbt_ui_model::Theme::Dark, bbt_ui_model::Density::Cozy);
        let mut list = DrawList::default();
        StateGlyph {
            activity: Activity::Waiting(WaitKind::Auth),
            grade: Grade::Inferred,
            color: Color::WHITE,
        }
        .draw(Rect::new(0.0, 0.0, 12.0, 12.0), &ctx, &mut list);
        assert!(
            list.primitives
                .iter()
                .filter(|p| matches!(p, Prim::Hairline { .. }))
                .count()
                >= 8
        );
        assert!(
            list.primitives
                .iter()
                .any(|p| matches!(p,Prim::Text{text,..} if text.as_ref()=="⚷"))
        );
        list.clip(Rect::new(0.0, 0.0, 5.0, 5.0), [0.0; 4]);
        assert!(list.validate().is_err());
        list.unclip();
        assert!(list.validate().is_ok());
        list.unclip();
        assert!(list.validate().is_err());
    }
}
