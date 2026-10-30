// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Selection of retained read-only text. Uses the painted runs, shaping widths and clips.
use crate::components::{Layout, LayoutKind};
use bbt_ui_model::{
    draw::{Color, DrawList},
    layout::{Rect, TextMeasure, TextStyle},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    pub rect: Rect,
    clip: Rect,
    boundaries: Vec<usize>,
    style: TextStyle,
    group: usize,
    bounds: Rect,
}
fn intersect(a: Rect, b: Rect) -> Rect {
    Rect::new(
        a.x.max(b.x),
        a.y.max(b.y),
        (a.right().min(b.right()) - a.x.max(b.x)).max(0.),
        (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.),
    )
}
fn contains(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}
pub fn collect(layout: &Layout, clip: Rect, out: &mut Vec<Run>) {
    let clip = match layout.kind {
        LayoutKind::Box { clip: true, .. } | LayoutKind::ScrollView { .. } => {
            intersect(clip, layout.rect)
        }
        _ => clip,
    };
    if clip.w <= 0. || clip.h <= 0. {
        return;
    }
    if let LayoutKind::Text {
        runs,
        selectable: true,
        ..
    } = &layout.kind
    {
        let group = out.len();
        let bounds = intersect(clip, layout.rect);
        for run in runs {
            let visible = intersect(clip, run.rect);
            if visible.w <= 0. || visible.h <= 0. || run.text.is_empty() {
                continue;
            }
            let s = &run.style;
            let style = TextStyle {
                size: s.size,
                weight: s.weight,
                mono: s.mono,
                italic: s.italic,
                tracking_em: s.tracking_em,
            };
            // Keep scalar boundaries only. Shape O(log n) prefixes on a gesture,
            // rather than shaping every prefix of every visible message each frame.
            let boundaries = std::iter::once(0)
                .chain(run.text.char_indices().map(|(i, c)| i + c.len_utf8()))
                .collect();
            out.push(Run {
                text: run.text.clone(),
                rect: run.rect,
                clip: visible,
                boundaries,
                style,
                group,
                bounds,
            });
        }
    }
    for child in &layout.children {
        collect(child, clip, out);
    }
}
#[derive(Default)]
pub struct Selection {
    runs: Vec<Run>,
    range: Option<[(usize, usize); 2]>,
    pub dragging: bool,
}
impl Selection {
    pub fn retain(&mut self, runs: Vec<Run>) {
        if self.runs != runs {
            if let Some([anchor, end]) = self.range {
                let first = anchor.0.min(end.0);
                let last = anchor.0.max(end.0);
                let old = &self.runs[first..=last];
                let at = runs.windows(old.len()).position(|window| window == old);
                if let Some(at) = at {
                    self.range = Some([
                        (at + anchor.0 - first, anchor.1),
                        (at + end.0 - first, end.1),
                    ]);
                } else {
                    self.clear();
                }
            }
            self.runs = runs;
        }
    }

    pub fn clear(&mut self) {
        self.range = None;
        self.dragging = false;
    }
    pub fn release(&mut self) {
        self.dragging = false;
    }
    pub fn contains(&self, at: [f32; 2]) -> bool {
        self.runs.iter().any(|r| contains(r.clip, at))
    }
    fn point(
        &self,
        at: [f32; 2],
        nearest: bool,
        measure: &dyn TextMeasure,
    ) -> Option<(usize, usize)> {
        // Within the anchor's text block, horizontal overshoot clamps to that
        // block. It must not jump through another card or a lower paragraph.
        let scope = self
            .range
            .and_then(|r| self.runs.get(r[0].0))
            .filter(|r| nearest && at[1] >= r.bounds.y && at[1] < r.bounds.bottom())
            .map(|r| r.group);
        let (i, r) = self
            .runs
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                (nearest || contains(r.clip, at)) && scope.is_none_or(|g| r.group == g)
            })
            .min_by(|(_, a), (_, b)| {
                let distance = |r: &Run| {
                    let dy = (r.clip.y - at[1]).max(0.).max(at[1] - r.clip.bottom());
                    let dx = (r.clip.x - at[0]).max(0.).max(at[0] - r.clip.right());
                    (dy, dx)
                };
                let a = distance(a);
                let b = distance(b);
                a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
            })?;
        let x = at[0] - r.rect.x;
        let width = |n: usize| measure.width(&r.text[..r.boundaries[n]], &r.style);
        let mut lo = 0;
        let mut hi = r.boundaries.len() - 1;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if width(mid) < x {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let n = if lo > 0 && (width(lo - 1) - x).abs() < (width(lo) - x).abs() {
            lo - 1
        } else {
            lo
        };
        let byte = r.boundaries[n];
        Some((i, byte))
    }
    pub fn begin(&mut self, at: [f32; 2], measure: &dyn TextMeasure) -> bool {
        self.clear();
        let Some(p) = self.point(at, false, measure) else {
            return false;
        };
        self.range = Some([p, p]);
        self.dragging = true;
        true
    }
    pub fn extend(&mut self, at: [f32; 2], measure: &dyn TextMeasure) -> bool {
        if !self.dragging {
            return false;
        }
        let p = self.point(at, true, measure).unwrap();
        let range = self.range.as_mut().unwrap();
        let changed = range[1] != p;
        range[1] = p;
        changed
    }
    fn ordered(&self) -> Option<[(usize, usize); 2]> {
        let [a, b] = self.range?;
        (a != b).then_some(if a < b { [a, b] } else { [b, a] })
    }
    pub fn text(&self) -> Option<String> {
        let [(a, lo), (b, hi)] = self.ordered()?;
        let mut text = String::new();
        for i in a..=b {
            let r = &self.runs[i];
            if i > a && (r.rect.y - self.runs[i - 1].rect.y).abs() > 1. {
                text.push('\n');
            }
            text.push_str(
                &r.text[if i == a { lo } else { 0 }..if i == b { hi } else { r.text.len() }],
            );
        }
        Some(text)
    }
    pub fn paint(&self, measure: &dyn TextMeasure) -> DrawList {
        let mut draw = DrawList::default();
        if let Some([(a, lo), (b, hi)]) = self.ordered() {
            for i in a..=b {
                let r = &self.runs[i];
                let x = |byte| r.rect.x + measure.width(&r.text[..byte], &r.style);
                let start = x(if i == a { lo } else { 0 });
                let end = x(if i == b { hi } else { r.text.len() });
                let rect = intersect(Rect::new(start, r.rect.y, end - start, r.rect.h), r.clip);
                if rect.w > 0. && rect.h > 0. {
                    draw.rect(rect, 0., Color([0.25, 0.50, 0.95, 0.35]));
                }
            }
        }
        draw
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Measure;
    impl TextMeasure for Measure {
        fn width(&self, s: &str, _: &TextStyle) -> f32 {
            s.chars().count() as f32 * 10.
        }
    }
    fn run(text: &str, y: f32) -> Run {
        Run {
            text: text.into(),
            rect: Rect::new(10., y, 100., 20.),
            clip: Rect::new(10., y, 100., 20.),
            boundaries: std::iter::once(0)
                .chain(text.char_indices().map(|(i, c)| i + c.len_utf8()))
                .collect(),
            group: 0,
            bounds: Rect::new(10., y, 100., 20.),
            style: TextStyle {
                size: 10.,
                weight: 400,
                mono: false,
                italic: false,
                tracking_em: 0.,
            },
        }
    }
    #[test]
    fn ui_selection_unicode_reverse_clip_release_and_source_change() {
        let mut s = Selection::default();
        s.retain(vec![run("a界é", 10.), run("next", 30.)]);
        assert!(!s.begin([0., 15.], &Measure));
        assert!(s.begin([40., 15.], &Measure));
        s.extend([10., 15.], &Measure);
        assert_eq!(s.text().as_deref(), Some("a界é"));
        assert!(!s.paint(&Measure).primitives.is_empty());
        s.begin([10., 15.], &Measure);
        s.extend([1000., 15.], &Measure);
        assert_eq!(s.text().as_deref(), Some("a界é"));
        s.release();
        assert!(!s.dragging);
        assert!(!s.extend([50., 35.], &Measure));
        s.begin([10., 15.], &Measure);
        s.extend([50., 35.], &Measure);
        assert_eq!(s.text().as_deref(), Some("a界é\nnext"));
        s.retain(vec![run("a界é", 10.), run("next", 30.)]);
        assert!(s.text().is_some());
        s.retain(vec![run("changed", 10.)]);
        assert!(s.text().is_none());
        assert!(!s.dragging);
        s.retain(vec![run("a界é", 10.), run("next", 30.)]);
        assert!(
            s.text().is_none(),
            "A to B to A cannot resurrect a captured range"
        );
    }
}
