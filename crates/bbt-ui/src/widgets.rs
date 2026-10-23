//! L6 controls are compositions of retained Box/Text/Stack/ScrollView and icons.
//! State and viewport offsets are caller-owned; no window, clock or input loop.
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color as Rgba, layout::Axis};

fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn border(color: ui::Color, alpha: f32, width: f32, outside: bool) -> ui::Border {
    ui::Border {
        paint: paint(color, alpha),
        width: ui::Length::Px(width),
        outside,
    }
}
fn no_border() -> ui::Border {
    border(ui::Color::Transparent, 0., 0., false)
}
fn text(value: impl Into<String>, style: ui::Text) -> c::Element {
    let mut t = c::Text::new(value, style);
    t.wrap = false;
    c::Element::Text(t)
}
fn normal(step: ui::TypeStep, weight: u16, color: ui::Color) -> ui::Text {
    ui::Text {
        step,
        weight,
        paint: ui::Paint::solid(color),
        // lab.css body is 14px/1.4; controls inherit the unitless line-height.
        line_height: 1.4,
        ..Default::default()
    }
}
fn stack(
    direction: Axis,
    gap: ui::Length,
    align: c::Alignment,
    children: Vec<c::Item>,
) -> c::Element {
    c::Element::Stack(c::Stack {
        direction,
        gap,
        align,
        children,
    })
}
fn icon(name: &str, color: ui::Color) -> c::Element {
    // The web control icons are explicitly 14px (lab.css .ui-li/.ui-input/.ui-btn).
    c::Element::Icon {
        name: name.into(),
        paint: ui::Paint::solid(color),
        size: ui::Length::Px(14.),
    }
}
fn badge(value: &str, foreground: Option<ui::Color>) -> c::Element {
    let mut s = ui::Style::new(ui::Preset::Chip);
    s.overrides.height = Some(ui::Length::Px(16.));
    s.overrides.radii = Some([ui::Length::Radius(ui::Radius::Badge); 4]);
    if let Some(color) = foreground {
        s.overrides.fill = Some(paint(color, 0.18));
    }
    let label = text(
        value,
        ui::Text {
            step: ui::TypeStep::Tiny,
            line_height: 16. / ui::TypeStep::Tiny.pixels(),
            paint: ui::Paint::solid(foreground.unwrap_or(ui::Color::Muted)),
            ..Default::default()
        },
    );
    c::Element::Box(c::Box::new(s, vec![label.into()]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Ghost,
    Danger,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Idle,
    Hover,
    Pressed,
    Focus,
    Disabled,
}
#[derive(Debug, Clone)]
pub struct Button {
    pub label: String,
    pub kind: ButtonKind,
    pub state: ButtonState,
    pub icon: Option<String>,
    pub key: Option<String>,
}
impl Button {
    pub fn element(&self, ctx: ui::Context) -> ui::style::Result<c::Element> {
        let ghost = self.kind == ButtonKind::Ghost;
        let foreground = if ghost {
            ui::Color::Foreground
        } else {
            ui::Color::OnAccent
        };
        let mut label_style = normal(ui::TypeStep::Row, if ghost { 500 } else { 600 }, foreground);
        let mut fill = match self.kind {
            ButtonKind::Primary => ui::Paint::solid(ui::Color::Accent),
            ButtonKind::Ghost => ui::Paint::solid(ui::Color::Transparent),
            ButtonKind::Danger => {
                ui::Paint::solid(ui::Color::Rgba(Rgba::hex(TOKENS.color.state.errored).0))
            }
        };
        if self.state == ButtonState::Hover {
            if ghost {
                fill = paint(
                    ui::Color::Foreground,
                    STYLE.tint.selection_hover_alpha as f32,
                );
            } else {
                let c = fill.resolve(ctx)?;
                fill = ui::Paint::solid(ui::Color::Rgba([
                    c[0] * 0.86 + 0.14,
                    c[1] * 0.86 + 0.14,
                    c[2] * 0.86 + 0.14,
                    c[3],
                ]));
            }
        }
        if self.state == ButtonState::Pressed {
            if ghost {
                fill = paint(ui::Color::Foreground, STYLE.tint.selection_alpha as f32);
            } else {
                let mut f = fill.resolve(ctx)?;
                for c in &mut f[..3] {
                    *c *= 0.88;
                }
                fill = ui::Paint::solid(ui::Color::Rgba(f));
                let mut c = label_style.paint.resolve(ctx)?;
                for c in &mut c[..3] {
                    *c *= 0.88;
                }
                label_style.paint = ui::Paint::solid(ui::Color::Rgba(c));
            }
        }
        let mut s = ui::Style::new(if ghost {
            ui::Preset::ButtonGhost
        } else {
            ui::Preset::ButtonPrimary
        });
        s.overrides.fill = Some(fill);
        s.overrides.border = Some(if ghost {
            border(
                ui::Color::Line,
                STYLE.material.hairline_alpha as f32,
                1.,
                false,
            )
        } else {
            border(ui::Color::Transparent, 0., 1., false)
        });
        let line = label_style.step.pixels() * label_style.line_height;
        // lab.css .ui-btn: min-height max(26px,row_h-2), padding s3 s8.
        s.overrides.height = Some(ui::Length::Px(
            (ctx.density.tokens().row_h as f32 - 2.)
                .max(26.)
                .max(line + 2. * STYLE.spacing.s3 as f32 + 2.),
        ));
        if self.state == ButtonState::Focus {
            s.overrides.rings = Some(vec![
                border(ui::Color::Accent, 1., 4., true),
                border(ui::Color::Background, 1., 2., true),
            ]);
        }
        if self.state == ButtonState::Disabled {
            s.overrides.opacity = Some(0.38);
        }
        let foreground = label_style.paint.color;
        let mut children = vec![];
        if let Some(name) = &self.icon {
            children.push(icon(name, foreground).into());
        }
        children.push(text(&self.label, label_style).into());
        if let Some(key) = &self.key {
            children.push(badge(key, Some(foreground)).into());
        }
        let mut b = c::Box::new(s, children);
        b.gap = ui::Length::Spacing(3);
        if self.state == ButtonState::Pressed {
            b.scale = 0.97;
        }
        Ok(c::Element::Box(b))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub height: f32,
    pub offset: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub start: usize,
    pub end: usize,
    pub before: f32,
    pub after: f32,
    pub offset: f32,
    pub height: f32,
}
pub fn row_window(count: usize, row: f32, gap: f32, viewport: Viewport) -> Window {
    let height = row * count as f32 + gap * count.saturating_sub(1) as f32;
    let offset = viewport
        .offset
        .max(0.)
        .min((height - viewport.height).max(0.));
    let pitch = (row + gap).max(0.001);
    let start = ((offset / pitch).floor() as usize).min(count);
    let end = (((offset + viewport.height) / pitch).ceil() as usize).min(count);
    let before = start as f32 * pitch;
    let end_y = (end as f32 * pitch).min(height);
    Window {
        start,
        end,
        before,
        after: (height - end_y).max(0.),
        offset,
        height,
    }
}
fn fixed_space(height: f32) -> c::Item {
    c::Item {
        element: c::Element::Spacer,
        extent: c::Extent::Fixed(height),
    }
}
fn rows(
    count: usize,
    row: f32,
    gap: f32,
    width: f32,
    viewport: Viewport,
    make: impl Fn(usize) -> c::Element,
) -> c::Element {
    let w = row_window(count, row, gap, viewport);
    let mut children = vec![];
    if w.before > 0. {
        children.push(fixed_space(w.before));
    }
    for i in w.start..w.end {
        children.push(make(i).into());
        if i + 1 < count && gap > 0. {
            children.push(fixed_space(gap));
        }
    }
    if w.after > 0. {
        children.push(fixed_space(w.after));
    }
    let content = stack(
        Axis::Column,
        ui::Length::Px(0.),
        c::Alignment::Stretch,
        children,
    );
    c::Element::ScrollView(c::ScrollView {
        viewport: [width, viewport.height],
        offset: [0., w.offset],
        content_width: None,
        content: std::boxed::Box::new(content),
    })
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub label: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub key: Option<String>,
}
#[derive(Debug, Clone)]
pub struct List {
    pub items: Vec<ListItem>,
    pub selected: Option<usize>,
    pub more: usize,
    pub viewport: Option<Viewport>,
}
impl List {
    fn row(&self, index: usize) -> c::Element {
        let item = &self.items[index];
        let mut children = vec![];
        if let Some(name) = &item.icon {
            children.push(icon(name, ui::Color::Foreground).into());
        }
        children.push(
            text(
                &item.label,
                ui::Text {
                    step: ui::TypeStep::Row,
                    ..Default::default()
                },
            )
            .into(),
        );
        if let Some(sub) = &item.subtitle {
            children.push(
                text(
                    sub,
                    ui::Text {
                        step: ui::TypeStep::Caption,
                        paint: ui::Paint::solid(ui::Color::Muted),
                        ..Default::default()
                    },
                )
                .into(),
            );
        }
        children.push(c::Element::Spacer.into());
        if let Some(key) = &item.key {
            children.push(badge(key, None).into());
        }
        let mut s = ui::Style::new(ui::Preset::Row);
        if self.selected == Some(index) {
            s.overrides.fill = Some(paint(
                ui::Color::Foreground,
                STYLE.tint.selection_alpha as f32,
            ));
        }
        let mut b = c::Box::new(s, children);
        b.gap = ui::Length::Spacing(4);
        c::Element::Box(b)
    }
    pub fn element(&self, ctx: ui::Context, width: f32) -> c::Element {
        if let Some(viewport) = self.viewport {
            return rows(
                self.items.len(),
                ctx.density.tokens().row_h as f32,
                STYLE.spacing.s1 as f32,
                width,
                viewport,
                |i| self.row(i),
            );
        }
        let mut children = (0..self.items.len())
            .map(|i| self.row(i).into())
            .collect::<Vec<_>>();
        if self.more > 0 {
            let mut s = ui::Style::new(ui::Preset::Card);
            s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
            s.overrides.border = Some(no_border());
            s.overrides.padding = Some([
                ui::Length::Spacing(2),
                ui::Length::PadX,
                ui::Length::Spacing(2),
                ui::Length::PadX,
            ]);
            children.push(
                c::Element::Box(c::Box::new(
                    s,
                    vec![
                        text(
                            format!("… {} more, virtualised", self.more),
                            ui::Text {
                                step: ui::TypeStep::Caption,
                                paint: ui::Paint::solid(ui::Color::Muted),
                                ..Default::default()
                            },
                        )
                        .into(),
                    ],
                ))
                .into(),
            );
        }
        stack(
            Axis::Column,
            ui::Length::Spacing(1),
            c::Alignment::Stretch,
            children,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub weight: f32,
}
#[derive(Debug, Clone)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    pub selected: Option<usize>,
    pub zebra: bool,
    pub viewport: Option<Viewport>,
}
impl Table {
    fn line(&self, index: Option<usize>) -> c::Element {
        let cells = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let mut s = ui::Style::new(ui::Preset::Card);
                s.overrides.radii = Some([ui::Length::Px(0.); 4]);
                let label = if let Some(row) = index {
                    s.overrides.height = Some(ui::Length::RowHeight);
                    s.overrides.border = Some(no_border());
                    s.overrides.padding = Some([
                        ui::Length::Px(0.),
                        ui::Length::Spacing(5),
                        ui::Length::Px(0.),
                        ui::Length::Spacing(5),
                    ]);
                    s.overrides.fill = Some(if self.selected == Some(row) {
                        paint(ui::Color::Foreground, STYLE.tint.selection_alpha as f32)
                    } else if self.zebra && row % 2 == 1 {
                        paint(ui::Color::Foreground, 0.04)
                    } else {
                        ui::Paint::solid(ui::Color::Transparent)
                    });
                    let mut t = c::Text::new(
                        &self.rows[row][i],
                        ui::Text {
                            step: ui::TypeStep::Row,
                            ..Default::default()
                        },
                    );
                    t.wrap = false;
                    t.ellipsis = true;
                    c::Element::Text(t)
                } else {
                    // The lab header inherits 14px/1.4; its small inline label keeps 1.35.
                    s.overrides.text = Some(ui::Text {
                        step: ui::TypeStep::Kind,
                        line_height: 1.4,
                        ..Default::default()
                    });
                    s.overrides.fill = Some(paint(ui::Color::Surface, 0.7));
                    s.overrides.padding = Some([
                        ui::Length::Spacing(2),
                        ui::Length::Spacing(5),
                        ui::Length::Spacing(2),
                        ui::Length::Spacing(5),
                    ]);
                    s.overrides.border = Some(border(
                        ui::Color::Line,
                        STYLE.material.hairline_alpha as f32,
                        STYLE.material.hairline_px as f32,
                        false,
                    ));
                    s.overrides.border_sides = Some([false, false, true, false]);
                    let mut label = text(
                        &col.title,
                        ui::Text {
                            step: ui::TypeStep::Small,
                            uppercase: true,
                            tracking_em: 0.22,
                            paint: ui::Paint::solid(ui::Color::Muted),
                            ..Default::default()
                        },
                    );
                    if let c::Element::Text(t) = &mut label {
                        t.inline = true;
                    }
                    label
                };
                let mut b = c::Box::new(s, vec![label.into()]);
                if index.is_none() {
                    b.inline = true;
                    b.align = c::Alignment::Baseline;
                }
                b.clip = true;
                c::Item {
                    element: c::Element::Box(b),
                    extent: c::Extent::Flex(col.weight),
                }
            })
            .collect();
        stack(Axis::Row, ui::Length::Px(0.), c::Alignment::Stretch, cells)
    }
    pub fn element(&self, ctx: ui::Context, width: f32) -> ui::style::Result<c::Element> {
        if self.columns.is_empty()
            || self
                .columns
                .iter()
                .any(|c| !c.weight.is_finite() || c.weight <= 0.)
            || self.rows.iter().any(|r| r.len() != self.columns.len())
        {
            return Err("invalid table columns or row width");
        }
        let mut children = vec![self.line(None).into()];
        if let Some(viewport) = self.viewport {
            children.push(
                rows(
                    self.rows.len(),
                    ctx.density.tokens().row_h as f32,
                    0.,
                    width - 2.,
                    viewport,
                    |i| self.line(Some(i)),
                )
                .into(),
            );
        } else {
            children.extend((0..self.rows.len()).map(|i| self.line(Some(i)).into()));
        }
        let mut s = ui::Style::new(ui::Preset::Card);
        s.overrides.padding = Some([ui::Length::Px(0.); 4]);
        s.overrides.radii = Some([ui::Length::Radius(ui::Radius::Row); 4]);
        s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
        // A table keeps its CSS layout border inside; the shared Card record
        // now uses an outside ring for cards and must not set table geometry.
        s.overrides.border = Some(border(
            ui::Color::Line,
            STYLE.material.hairline_alpha as f32,
            STYLE.material.hairline_px as f32,
            false,
        ));
        let mut b = c::Box::new(s, children);
        b.direction = Axis::Column;
        b.gap = ui::Length::Px(0.);
        b.align = c::Alignment::Stretch;
        b.clip = true;
        Ok(c::Element::Box(b))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputState {
    Empty,
    Value,
    Focus,
    Invalid,
}
#[derive(Debug, Clone)]
pub struct Input {
    pub value: String,
    pub placeholder: String,
    pub state: InputState,
    pub icon: Option<String>,
    pub error: Option<String>,
    pub edit: Option<c::Edit>,
}
impl Input {
    pub fn element(&self, _ctx: ui::Context) -> c::Element {
        let empty = self.edit.is_none() && self.state == InputState::Empty;
        let label_style = normal(
            ui::TypeStep::Body,
            400,
            if empty {
                ui::Color::Muted
            } else {
                ui::Color::Foreground
            },
        );
        let mut control = ui::Style::new(ui::Preset::Card);
        control.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
        control.overrides.border = Some(no_border());
        control.overrides.radii = Some([ui::Length::Px(0.); 4]);
        // Native input's UA content padding in the web fixture (border is reset).
        control.overrides.padding = Some([
            ui::Length::Px(1.),
            ui::Length::Px(2.),
            ui::Length::Px(1.),
            ui::Length::Px(2.),
        ]);
        control.overrides.height = Some(ui::Length::Px(
            (label_style.step.pixels() * label_style.line_height * 64.).floor() / 64. + 2.,
        ));
        let content = if let Some(edit) = &self.edit {
            // Editing and ordinary input share the same explicit content inset.
            c::Element::EditingText(c::EditingText {
                value: self.value.clone(),
                placeholder: self.placeholder.clone(),
                style: label_style,
                placeholder_paint: ui::Paint::solid(ui::Color::Muted),
                edit: edit.clone(),
                name: "input/edit".into(),
            })
        } else {
            text(
                if empty {
                    &self.placeholder
                } else {
                    &self.value
                },
                label_style,
            )
        };
        let item = c::Item {
            element: content,
            extent: if self.edit.is_some() {
                c::Extent::Fill
            } else {
                c::Extent::Auto
            },
        };
        let mut b = c::Box::new(control, vec![item]);
        b.clip = true;
        let mut children = vec![];
        if let Some(name) = &self.icon {
            children.push(icon(name, ui::Color::Muted).into());
        }
        let control = c::Element::Box(b);
        let control = if self.edit.is_some() {
            c::Element::Named {
                name: "input/control".into(),
                content: Box::new(control),
            }
        } else {
            control
        };
        children.push(c::Item {
            element: control,
            extent: c::Extent::Fill,
        });
        let mut s = ui::Style::new(ui::Preset::Input);
        let state = if self.edit.is_some() {
            InputState::Focus
        } else {
            self.state
        };
        let color = match state {
            InputState::Focus => Some((ui::Color::Accent, 0.25)),
            InputState::Invalid => Some((
                ui::Color::Rgba(Rgba::hex(TOKENS.color.state.errored).0),
                0.18,
            )),
            _ => None,
        };
        if let Some((color, alpha)) = color {
            s.overrides.border = Some(border(color, 1., 1., false));
            s.overrides.rings = Some(vec![border(color, alpha, 3., true)]);
        }
        let mut b = c::Box::new(s, children);
        b.gap = ui::Length::Spacing(3);
        let mut children = vec![c::Element::Box(b).into()];
        if state == InputState::Invalid {
            children.push(
                text(
                    self.error.as_deref().unwrap_or("not a valid value"),
                    ui::Text {
                        step: ui::TypeStep::Caption,
                        paint: ui::Paint::solid(ui::Color::Rgba(
                            Rgba::hex(TOKENS.color.state.errored).0,
                        )),
                        ..Default::default()
                    },
                )
                .into(),
            );
        }
        stack(
            Axis::Column,
            ui::Length::Spacing(1),
            c::Alignment::Stretch,
            children,
        )
    }
}
