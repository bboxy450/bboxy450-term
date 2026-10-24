//! Wrapped editing over the ordinary paragraph solver. Hosts supply the width
//! and editing state; runs supply caret positions, never character estimates.
use crate::{self as ui, components as c};
use bbt_ui_model::layout::{Rect, TextStyle};

pub struct TextArea {
    pub value: String,
    pub placeholder: String,
    pub style: ui::Text,
    pub edit: Option<c::Edit>,
    pub min_lines: usize,
    pub wrap: bool,
    pub name: String,
}
fn named(name: impl Into<String>, content: c::Element) -> c::Element {
    c::Element::Named {
        name: name.into(),
        content: Box::new(content),
    }
}
fn plain(height: f32, radius: f32) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.height = Some(ui::Length::Px(height));
    s.overrides.padding = Some([ui::Length::Px(0.); 4]);
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s
}
fn frame(width: f32, height: f32, content: Vec<c::Item>) -> c::Element {
    let mut s = plain(height, 0.);
    s.overrides.width = Some(ui::Length::Px(width));
    let mut b = c::Box::new(s, content);
    b.gap = ui::Length::Px(0.);
    b.align = c::Alignment::Start;
    c::Element::Box(b)
}
fn mark(key: String, r: Rect, paint: ui::Paint, radius: f32, height: f32) -> c::Placed {
    let mut s = plain(r.h, radius);
    s.overrides.width = Some(ui::Length::Px(r.w));
    s.overrides.fill = Some(paint);
    c::Placed {
        content: named(key, c::Element::Box(c::Box::new(s, vec![]))),
        left: r.x,
        bottom: height - r.bottom(),
    }
}
impl TextArea {
    /// Hit the retained paragraph with its actual shaping measure. The result
    /// is a UTF-16 boundary, including when several words share one line.
    pub fn index_at(value: &str, layout: &c::Layout, at: [f32; 2], m: &dyn c::Measure) -> usize {
        let mut best = (f32::INFINITY, f32::INFINITY, 0);
        for (index, r) in positions(value, layout, m) {
            let dy = if at[1] < r.y {
                r.y - at[1]
            } else {
                (at[1] - r.bottom()).max(0.)
            };
            let dx = (r.x - at[0]).abs();
            if dy < best.0 || dy == best.0 && dx < best.1 {
                best = (dy, dx, index);
            }
        }
        best.2
    }
    pub fn position_at(
        value: &str,
        layout: &c::Layout,
        index: usize,
        m: &dyn c::Measure,
    ) -> [f32; 2] {
        positions(value, layout, m)
            .into_iter()
            .rev()
            .find(|(at, _)| *at == index)
            .map_or([layout.rect.x, layout.rect.y], |(_, r)| {
                [r.x, r.y + r.h * 0.5]
            })
    }
    pub fn element(
        &self,
        width: f32,
        ctx: ui::Context,
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        if !width.is_finite() || width <= 0. {
            return Err("text area width must be positive");
        }
        let mut display = self.value.clone();
        let mut caret = self
            .edit
            .as_ref()
            .and_then(|e| e.caret)
            .unwrap_or(display.encode_utf16().count());
        let (mut selected, mut underline) = (None, None);
        if let Some(edit) = &self.edit {
            let [a, b] = edit.replacement.or(edit.select).unwrap_or([caret, caret]);
            if a > b {
                return Err("field selection is reversed");
            }
            let start = c::utf16_byte_index(&display, a).ok_or("invalid field UTF-16 boundary")?;
            let end = c::utf16_byte_index(&display, b).ok_or("invalid field UTF-16 boundary")?;
            if let Some(p) = &edit.preedit {
                let len = p.text.encode_utf16().count();
                let at = p.caret.unwrap_or(len);
                c::utf16_byte_index(&p.text, at).ok_or("invalid preedit UTF-16 boundary")?;
                display.replace_range(start..end, &p.text);
                underline = Some([a, a + len]);
                caret = a + at;
            } else if a != b {
                selected = Some([a, b]);
            }
        }
        let empty = display.is_empty();
        let shown = if empty { &self.placeholder } else { &display };
        let mut style = self.style;
        if empty {
            style.paint = ui::Paint::solid(ui::Color::Muted);
        }
        let mut paragraph = c::Text::new(shown, style);
        paragraph.wrap = self.wrap;
        let text = c::Element::Text(paragraph);
        let layout = text.layout(Rect::new(0., 0., width, 0.), ctx, m)?;
        let c::LayoutKind::Text { runs, .. } = &layout.kind else {
            unreachable!()
        };
        let size = self.style.size.unwrap_or(self.style.step.pixels());
        let line = (size * self.style.line_height * 64.).floor() / 64.;
        let height = layout
            .rect
            .h
            .max((size * self.style.line_height * self.min_lines as f32 * 64.).floor() / 64.);
        let (mut behind, mut ahead) = (vec![], vec![]);
        let mut byte = 0;
        let mut caret_point = [0., 0.];
        let mut found_caret = false;
        for (i, run) in runs.iter().enumerate() {
            let found = shown[byte..].find(&run.text).map_or(byte, |n| byte + n);
            let first = shown[..found].encode_utf16().count();
            let last = first + run.text.encode_utf16().count();
            let font = TextStyle {
                size: run.style.size,
                weight: run.style.weight,
                mono: run.style.mono,
                italic: run.style.italic,
                tracking_em: run.style.tracking_em,
            };
            let x_at = |at: usize| {
                let byte =
                    c::utf16_byte_index(&run.text, at.saturating_sub(first).min(last - first))
                        .expect("field run boundary");
                run.rect.x + m.width(&run.text[..byte], &font)
            };
            for (range, under) in [(selected, false), (underline, true)] {
                if let Some([a, b]) = range {
                    let (start, end) = (a.max(first), b.min(last));
                    if start < end {
                        let x = x_at(start);
                        let rect = if under {
                            let [a, d] = m.font_metrics(&font).map(f32::round);
                            Rect::new(
                                x,
                                run.rect.y + a + ((line - a - d) * 0.5).floor() + 3.,
                                x_at(end) - x,
                                1.,
                            )
                        } else {
                            let [a, d] = m.font_metrics(&font).map(f32::round);
                            Rect::new(
                                x,
                                run.rect.y + ((line - a - d) * 0.5).floor(),
                                x_at(end) - x,
                                a + d,
                            )
                        };
                        let part = if under { "underline" } else { "selection" };
                        let layer = if under { &mut ahead } else { &mut behind };
                        layer.push(mark(
                            format!("{}/{part}{i}", self.name),
                            rect,
                            if under {
                                self.style.paint
                            } else {
                                ui::Paint {
                                    color: ui::Color::Accent,
                                    alpha: ui::Alpha::Value(0.35),
                                }
                            },
                            if under { 0. } else { 2. },
                            height,
                        ));
                    }
                }
            }
            if !empty
                && !found_caret
                && caret >= first
                && (caret < last || i + 1 == runs.len() && caret == last)
            {
                caret_point = [x_at(caret), run.rect.y];
                found_caret = true;
            }
            byte = found + run.text.len();
            if run.text.is_empty() && shown[byte..].starts_with('\n') {
                byte += 1;
            }
        }
        if let Some(edit) = &self.edit
            && selected.is_none()
            && !edit.hide_caret
        {
            let h = (size * 1.2 * 64.).floor() / 64.;
            ahead.push(mark(
                format!("{}/caret", self.name),
                Rect::new(
                    caret_point[0] - 1.,
                    caret_point[1] + {
                        let font = TextStyle {
                            size,
                            weight: self.style.weight,
                            mono: self.style.mono,
                            italic: self.style.italic,
                            tracking_em: self.style.tracking_em,
                        };
                        let [a, d] = m.font_metrics(&font).map(f32::round);
                        a + ((line - a - d) * 0.5).floor() - h + (size * 0.2 * 64.).floor() / 64.
                    },
                    2.,
                    h,
                ),
                ui::Paint::solid(ui::Color::Accent),
                1.,
                height,
            ));
        }
        let mut overlays = behind;
        overlays.push(c::Placed {
            content: named(
                format!("{}/text", self.name),
                frame(width, layout.rect.h, vec![text.into()]),
            ),
            left: 0.,
            bottom: height - layout.rect.h,
        });
        overlays.extend(ahead);
        Ok(named(
            &self.name,
            c::Element::Layers {
                base: Box::new(frame(width, height, vec![])),
                overlays,
            },
        ))
    }
}

fn positions(value: &str, layout: &c::Layout, m: &dyn c::Measure) -> Vec<(usize, Rect)> {
    fn collect<'a>(layout: &'a c::Layout, out: &mut Vec<&'a c::PositionedRun>) {
        if let c::LayoutKind::Text { runs, .. } = &layout.kind {
            out.extend(runs);
        }
        for child in &layout.children {
            collect(child, out);
        }
    }
    if value.is_empty() {
        return vec![];
    }
    let mut runs = vec![];
    collect(layout, &mut runs);
    let mut byte = 0;
    let mut result = vec![];
    for run in runs {
        let Some(skip) = value[byte..].find(&run.text) else {
            continue;
        };
        let start = byte + skip;
        let font = TextStyle {
            size: run.style.size,
            weight: run.style.weight,
            mono: run.style.mono,
            italic: run.style.italic,
            tracking_em: run.style.tracking_em,
        };
        for at in run
            .text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(run.text.len()))
        {
            let units = value[..start + at].encode_utf16().count();
            let x = run.rect.x + m.width(&run.text[..at], &font);
            result.push((units, Rect::new(x, run.rect.y, 0., run.line_height)));
        }
        byte = start + run.text.len();
    }
    result
}
