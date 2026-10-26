//! The perf overlay (Cmd+Opt+F): the runtime meter record in the hud panel, as the lab's
//! components.js perfHud draws it. Phase-stacked frame bars under a dashed budget line,
//! the legend doubling as the table, a foot of CPU/GPU/late. The host maps its meter onto
//! these records; the module never reads a clock.
use super::{seat_row::named, strip::plain};
use crate::{self as ui, components as c};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::Color as Rgba,
    layout::{Axis, Rect},
};
use serde::{Deserialize, Serialize};

/// Spark size in px; the budget sits at two thirds of the height, so a hitch clips at the top.
pub const SPARK_W: f32 = 240.;
pub const SPARK_H: f32 = 44.;
const BAR_W: f32 = 2.;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerfPhases {
    pub drain: f64,
    pub layout: f64,
    pub draw: f64,
    pub encode: f64,
    pub present_wait: f64,
}
impl PerfPhases {
    pub const NAMES: [&'static str; 5] = ["drain", "layout", "draw", "encode", "present_wait"];
    pub fn values(&self) -> [f64; 5] {
        [
            self.drain,
            self.layout,
            self.draw,
            self.encode,
            self.present_wait,
        ]
    }
}
/// One frame in the window: phases in ms, or an idle stub that never counts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfFrame {
    #[serde(flatten)]
    pub phases: PerfPhases,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub idle: bool,
    /// The phase that overran on a late frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hitch: Option<String>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfCpu {
    pub p50: f64,
    pub p99: f64,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfHitch {
    pub phase: String,
    pub ms: f64,
    pub frame: u64,
}
/// The meter record (runtime.md), in the lab's field names.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfHud {
    pub hz: u32,
    pub interval_ms: f64,
    pub budget_ms: f64,
    pub cpu: PerfCpu,
    pub gpu_ms: f64,
    pub phases: PerfPhases,
    pub frames: Vec<PerfFrame>,
    pub hitch: Option<PerfHitch>,
    pub late: u32,
    pub idle_ms: Option<f64>,
}

fn ms(v: f64) -> String {
    format!("{v:.2}")
}
fn label(phase: &str) -> String {
    phase.replacen('_', " ", 1)
}
fn ago(v: f64) -> String {
    if v < 1000. {
        format!("{} ms ago", v.round())
    } else {
        format!("{:.1} s ago", v / 1000.)
    }
}
fn rgba(c: Rgba) -> ui::Paint {
    ui::Paint::solid(ui::Color::Rgba(c.0))
}
fn text(value: impl Into<String>, paint: ui::Paint, line_height: f32) -> c::Element {
    c::Element::Text(c::Text::new(
        value,
        ui::Text {
            step: ui::TypeStep::Micro,
            mono: true,
            paint,
            line_height,
            ..Default::default()
        },
    ))
}
fn fill(height: Option<f32>, width: Option<f32>, radius: f32, paint: ui::Paint) -> ui::Style {
    let mut s = plain(height, [0.; 4], radius);
    s.overrides.width = width.map(ui::Length::Px);
    s.overrides.fill = Some(paint);
    s
}

impl PerfHud {
    /// The card's outer width: the spark is the widest min-content child.
    pub fn width(ctx: ui::Context) -> f32 {
        SPARK_W + 2. * ui::Length::Spacing(6).resolve(ctx.density).unwrap_or(0.)
    }
    /// The panel, its parts named `key/head`, `key/spark`, `key/ph{i}`, `key/foot{i}`.
    pub fn element(&self, ctx: ui::Context, key: &str) -> c::Element {
        let p = ctx.theme.palette();
        let (fg, dim, accent, surface) = (
            Rgba::hex(p.fg),
            Rgba::hex(p.fg_dim),
            Rgba::hex(p.accent),
            Rgba::hex(p.surface),
        );
        let waiting = Rgba::hex(TOKENS.color.state.waiting);
        // .perf-hud .ph-*: color-mix(in srgb, a N%, b) is a.mix(b, 1 - N).
        let colors = [
            accent.mix(fg, 0.55),
            accent.mix(fg, 0.25),
            accent,
            accent.mix(surface, 0.5),
            dim.alpha(0.24),
        ];
        let line = 1.4; // body line-height, inherited by the panel
        let k = SPARK_H as f64 / (self.budget_ms * 1.5);
        let hair = STYLE.material.hairline_px as f32;
        let over = self.hitch.as_ref().map(|h| h.phase.as_str());

        let idle = self.idle_ms.is_some();
        let mut b = c::Text::new(
            if idle {
                "idle".into()
            } else {
                format!("{} Hz", self.hz)
            },
            ui::Text {
                step: ui::TypeStep::Subtitle,
                weight: 650,
                tracking_em: -0.01,
                paint: ui::Paint::solid(ui::Color::Foreground),
                line_height: line,
                ..Default::default()
            },
        );
        b.wrap = false;
        let span = text(
            match self.idle_ms {
                Some(v) => format!("last frame {}", ago(v)),
                None => format!(
                    "{} ms · budget {}",
                    ms(self.interval_ms),
                    ms(self.budget_ms)
                ),
            },
            ui::Paint::solid(ui::Color::Muted),
            line,
        );
        let mut head = c::Box::new(
            plain(None, [0.; 4], 0.),
            vec![
                named(format!("{key}/head/b"), c::Element::Text(b)).into(),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fill,
                },
                named(format!("{key}/head/span"), span).into(),
            ],
        );
        head.align = c::Alignment::Baseline;
        head.gap = ui::Length::Spacing(4);

        // Bars stack bottom-up (column-reverse) and clip at the spark's content top. All
        // segments are one Rects node: the same Rect primitives the per-bar boxes painted, in
        // the same order (top segment first), at the same exact fractional edges, so the
        // renderer snaps them as it snapped the boxes. A 120-frame window is a few elements.
        let inner = SPARK_H - hair;
        let n = self.frames.len();
        let mut segments: Vec<(Rect, ui::Paint)> = vec![];
        for (i, f) in self.frames.iter().enumerate() {
            let x = i as f32 * BAR_W;
            if f.idle {
                segments.push((Rect::new(x, inner - 2., BAR_W, 2.), rgba(dim.alpha(0.35))));
                continue;
            }
            let mut bottom = 0.;
            let mut bar = vec![];
            for ((name, v), color) in PerfPhases::NAMES.iter().zip(f.phases.values()).zip(colors) {
                let h = ((v * k) as f32).min(inner - bottom).max(0.);
                bottom += h;
                let paint = if f.hitch.as_deref() == Some(*name) {
                    rgba(waiting)
                } else {
                    rgba(color)
                };
                bar.push((Rect::new(x, inner - bottom, BAR_W, h), paint));
            }
            segments.extend(bar.into_iter().rev());
        }
        let slot = |i: usize| {
            let mut bar = c::Box::new(plain(None, [0.; 4], 0.), vec![]);
            bar.style.overrides.width = Some(ui::Length::Px(BAR_W));
            named(format!("{key}/bar{i}"), c::Element::Box(bar)).into()
        };
        let mut bars: Vec<c::Item> = vec![];
        if n > 0 {
            bars.push(slot(0));
        }
        if n > 1 {
            bars.push(c::Item {
                element: c::Element::Spacer,
                extent: c::Extent::Fixed((n - 2) as f32 * BAR_W),
            });
            bars.push(slot(n - 1));
        }
        let mut spark_style = plain(Some(SPARK_H), [0.; 4], 0.);
        spark_style.overrides.width = Some(ui::Length::Px(SPARK_W));
        spark_style.overrides.border_sides = Some([false, false, true, false]);
        spark_style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        let mut spark = c::Box::new(spark_style, bars);
        spark.gap = ui::Length::Px(0.);
        spark.align = c::Alignment::Stretch;
        let budget = c::Element::Path(c::Path {
            size: [SPARK_W, 1.],
            points: vec![
                c::PathCommand::Move([0., 0.5]),
                c::PathCommand::Line([SPARK_W, 0.5]),
            ]
            .into(),
            stroke: c::PathStroke {
                width: 1.,
                dash: vec![3., 2.].into(),
                ..Default::default()
            },
            paint: ui::Paint::solid(ui::Color::Foreground),
            fill: None,
            opacity: 0.55,
        });
        let spark = c::Element::Layers {
            base: Box::new(named(format!("{key}/spark"), c::Element::Box(spark))),
            overlays: [c::Placed {
                content: c::Element::Rects(c::Rects {
                    size: [SPARK_W, inner],
                    rects: segments.into(),
                    repeat: None,
                }),
                left: 0.,
                bottom: hair,
            }]
            .into_iter()
            .chain([c::Placed {
                content: named(format!("{key}/budget"), budget),
                left: 0.,
                bottom: (self.budget_ms * k) as f32 + hair,
            }])
            .collect(),
        };

        let rows = PerfPhases::NAMES
            .iter()
            .zip(self.phases.values())
            .zip(colors)
            .enumerate()
            .map(|(i, ((name, v), color))| {
                let hit = over == Some(*name);
                let value = match (&self.hitch, hit) {
                    (Some(h), true) => h.ms,
                    _ => v,
                };
                let row_line = 16. / ui::TypeStep::Micro.pixels();
                let mut row = c::Box::new(
                    plain(None, [0.; 4], STYLE.radius.tag as f32),
                    vec![
                        named(
                            format!("{key}/ph{i}/icon"),
                            c::Element::Box(c::Box::new(
                                fill(
                                    Some(8.),
                                    Some(8.),
                                    2.,
                                    rgba(if hit { waiting } else { color }),
                                ),
                                vec![],
                            )),
                        )
                        .into(),
                        c::Item {
                            element: named(
                                format!("{key}/ph{i}/label"),
                                text(
                                    label(name),
                                    ui::Paint::solid(if hit {
                                        ui::Color::Foreground
                                    } else {
                                        ui::Color::Muted
                                    }),
                                    row_line,
                                ),
                            ),
                            extent: c::Extent::Fill,
                        },
                        named(format!("{key}/ph{i}/value"), {
                            let mut t =
                                text(ms(value), ui::Paint::solid(ui::Color::Foreground), row_line);
                            if let c::Element::Text(t) = &mut t {
                                t.runs[0].style.weight = 500;
                            }
                            t
                        })
                        .into(),
                    ],
                );
                row.style.overrides.padding = Some([
                    ui::Length::Px(0.),
                    ui::Length::Spacing(2),
                    ui::Length::Px(0.),
                    ui::Length::Spacing(2),
                ]);
                if hit {
                    row.style.overrides.fill = Some(rgba(waiting.alpha(0.22)));
                }
                row.align = c::Alignment::Center;
                row.gap = ui::Length::Spacing(3);
                named(format!("{key}/ph{i}"), c::Element::Box(row)).into()
            })
            .collect();
        let legend = c::Element::Stack(c::Stack {
            direction: Axis::Column,
            gap: ui::Length::Spacing(1),
            align: c::Alignment::Stretch,
            children: rows,
        });

        let late = match &self.hitch {
            Some(h) => format!(
                "late {} · {} {} ms · #{}",
                self.late,
                label(&h.phase),
                ms(h.ms),
                h.frame
            ),
            None => format!("late {} / {}", self.late, self.frames.len()),
        };
        let spans = [
            (
                format!("CPU {} · p99 {} ms", ms(self.cpu.p50), ms(self.cpu.p99)),
                ui::Paint::solid(ui::Color::Muted),
            ),
            (
                format!("GPU {} ms", ms(self.gpu_ms)),
                ui::Paint::solid(ui::Color::Muted),
            ),
            (
                late,
                if self.hitch.is_some() {
                    rgba(waiting)
                } else {
                    ui::Paint::solid(ui::Color::Muted)
                },
            ),
        ];
        let mut foot = c::Flow::new(
            spans
                .into_iter()
                .enumerate()
                .map(|(i, (s, paint))| named(format!("{key}/foot{i}"), text(s, paint, line)).into())
                .collect(),
        );
        foot.column_gap = ui::Length::Spacing(4);
        foot.row_gap = ui::Length::Spacing(1);

        let mut card = c::Box::new(
            ui::Style::new(ui::Preset::Hud),
            vec![
                named(format!("{key}/head"), c::Element::Box(head)).into(),
                spark.into(),
                legend.into(),
                named(format!("{key}/foot"), c::Element::Flow(foot)).into(),
            ],
        );
        card.direction = Axis::Column;
        card.align = c::Alignment::Stretch;
        card.gap = ui::Length::Spacing(4);
        named(key, c::Element::Box(card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec runtime §§4–6
    #[test]
    fn lab_records_round_trip_and_idle_frames_stay_stubs() {
        let json = r#"{"hz":120,"interval_ms":8.33,"budget_ms":8.33,"cpu":{"p50":2.4,"p99":3.1},"gpu_ms":2.9,
            "phases":{"drain":0.17,"layout":0.42,"draw":1.4,"encode":0.6,"present_wait":5.7},
            "frames":[{"drain":0.1,"layout":0.3,"draw":1.2,"encode":0.5,"present_wait":6.0},{"idle":true},
            {"drain":0.1,"layout":9.7,"draw":1.2,"encode":0.5,"present_wait":0.0,"hitch":"layout"}],
            "hitch":{"phase":"layout","ms":9.7,"frame":4812},"late":1,"idle_ms":null}"#;
        let hud: PerfHud = serde_json::from_str(json).unwrap();
        assert!(hud.frames[1].idle);
        assert_eq!(hud.frames[1].phases, PerfPhases::default());
        assert_eq!(hud.frames[2].hitch.as_deref(), Some("layout"));
        let back: PerfHud = serde_json::from_value(serde_json::to_value(&hud).unwrap()).unwrap();
        assert_eq!(back, hud);
        assert_eq!(ago(2400.), "2.4 s ago");
        assert_eq!(ago(240.4), "240 ms ago");
        assert_eq!(label("present_wait"), "present wait");
    }
}
