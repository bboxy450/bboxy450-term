// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-only fullscreen view of an already prepared vector figure. Zoom/pan
//! never reparse Mermaid, read files or change the underlying document/feed.
use super::*;
use bbt_ui_model::layout::Rect;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Out,
    In,
    Fit,
    Actual,
    Close,
}
impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Self::Out => "diagram-fullscreen/out",
            Self::In => "diagram-fullscreen/in",
            Self::Fit => "diagram-fullscreen/fit",
            Self::Actual => "diagram-fullscreen/actual",
            Self::Close => "diagram-fullscreen/close",
        }
    }
}
const ACTIONS: [(Action, &str); 5] = [
    (Action::Out, "−"),
    (Action::In, "+"),
    (Action::Fit, "Fit"),
    (Action::Actual, "1:1"),
    (Action::Close, "Close · Esc"),
];
#[derive(Default)]
pub struct Regions {
    pub viewport: Rect,
    pub controls: Vec<(Rect, Action)>,
}
impl Regions {
    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        self.controls
            .iter()
            .find(|(r, _)| contains(*r, at))
            .map(|(_, a)| *a)
    }
    pub fn cursor(&self, at: [f32; 2]) -> ui::cursor::Cursor {
        if self.hit(at).is_some() {
            ui::cursor::Cursor::Pointer
        } else {
            ui::cursor::Cursor::Default
        }
    }
}
fn contains(r: Rect, p: [f32; 2]) -> bool {
    p[0] >= r.x && p[0] < r.right() && p[1] >= r.y && p[1] < r.bottom()
}
fn find<'a>(n: &'a c::Layout, id: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(name) if name==id) {
        return Some(n);
    }
    n.children.iter().find_map(|n| find(n, id))
}
fn find_mut<'a>(n: &'a mut c::Layout, id: &str) -> Option<&'a mut c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(name) if name==id) {
        return Some(n);
    }
    n.children.iter_mut().find_map(|n| find_mut(n, id))
}

pub struct Viewer {
    canvas: Arc<c::Layout>,
    zoom: f32,
    fit: bool,
    offset: [f32; 2],
    viewport: Rect,
    drag: Option<[f32; 2]>,
    selected: usize,
    keyboard: bool,
}
impl Viewer {
    pub fn new(canvas: Arc<c::Layout>, size: [f32; 2]) -> Option<Self> {
        if ![canvas.rect.w, canvas.rect.h]
            .into_iter()
            .all(|v| v.is_finite() && v > 0.)
        {
            return None;
        }
        Some(Self {
            canvas,
            zoom: 1.,
            fit: true,
            offset: [0.; 2],
            viewport: Rect::new(0., 52., size[0].max(1.), (size[1] - 52.).max(1.)),
            drag: None,
            selected: 0,
            keyboard: false,
        })
    }
    pub fn scale(&self) -> f32 {
        if self.fit {
            (self.viewport.w / self.canvas.rect.w)
                .min(self.viewport.h / self.canvas.rect.h)
                .min(1.)
                .clamp(0.01, 8.)
        } else {
            self.zoom
        }
    }
    fn extent(&self) -> [f32; 2] {
        let s = self.scale();
        [
            (self.canvas.rect.w * s - self.viewport.w).max(0.),
            (self.canvas.rect.h * s - self.viewport.h).max(0.),
        ]
    }
    fn clamp(&mut self) {
        let extent = self.extent();
        for (p, max) in self.offset.iter_mut().zip(extent) {
            *p = p.clamp(0., max);
        }
    }
    pub fn act(&mut self, action: Action) -> bool {
        match action {
            Action::Close => return true,
            Action::Fit => {
                self.fit = true;
                self.offset = [0.; 2];
            }
            Action::Actual => {
                self.fit = false;
                self.zoom = 1.;
                self.offset = [0.; 2];
            }
            Action::In | Action::Out => {
                let old = self.scale();
                let next = (old * if action == Action::In { 1.25 } else { 0.8 }).clamp(0.01, 8.);
                for (p, span) in self
                    .offset
                    .iter_mut()
                    .zip([self.viewport.w, self.viewport.h])
                {
                    *p = (*p + span / 2.) * (next / old) - span / 2.;
                }
                self.zoom = next;
                self.fit = false;
            }
        }
        self.clamp();
        false
    }
    pub fn pan(&mut self, delta: [f32; 2]) {
        for (p, d) in self.offset.iter_mut().zip(delta) {
            if d.is_finite() {
                *p += d;
            }
        }
        self.clamp();
    }
    pub fn press(&mut self, at: [f32; 2], regions: &Regions) -> bool {
        if let Some(action) = regions.hit(at) {
            return self.act(action);
        }
        if contains(regions.viewport, at) {
            self.drag = Some(at);
        }
        false
    }
    pub fn drag(&mut self, at: [f32; 2]) -> bool {
        let Some(old) = self.drag else {
            return false;
        };
        self.drag = Some(at);
        self.pan([old[0] - at[0], old[1] - at[1]]);
        true
    }
    pub fn release(&mut self) {
        self.drag = None;
    }
    pub fn select(&mut self, backwards: bool) {
        self.keyboard = true;
        self.selected =
            (self.selected + if backwards { ACTIONS.len() - 1 } else { 1 }) % ACTIONS.len();
    }
    pub fn submit(&mut self) -> bool {
        self.act(ACTIONS[self.selected].0)
    }
    pub fn layout(
        &mut self,
        size: [f32; 2],
        ctx: ui::Context,
        measure: &dyn c::Measure,
    ) -> Result<(c::Layout, Regions), String> {
        self.viewport = Rect::new(0., 52., size[0].max(1.), (size[1] - 52.).max(1.));
        let mut children = vec![
            natural(c::Element::Text(c::Text::new(
                "Mermaid",
                font(14., 600, false, ui::Color::Foreground),
            ))),
            fill(c::Element::Spacer),
            natural(c::Element::Text(c::Text::new(
                format!("{:.0}%", self.scale() * 100.),
                font(12., 400, false, ui::Color::Muted),
            ))),
        ];
        for (i, (action, label)) in ACTIONS.iter().enumerate() {
            let active = self.keyboard && self.selected == i;
            children.push(natural(row(
                action.id(),
                surface(
                    if active {
                        ui::Color::Accent
                    } else {
                        ui::Color::Muted
                    },
                    0.12,
                    8.,
                    [5., 10., 5., 10.],
                ),
                0.,
                vec![natural(c::Element::Text(c::Text::new(
                    *label,
                    font(13., 500, false, ui::Color::Foreground),
                )))],
            )));
        }
        let header = row(
            "diagram-fullscreen/toolbar",
            surface(ui::Color::Surface, 1., 0., [8.; 4]),
            6.,
            children,
        );
        let mut style = surface(ui::Color::Background, 1., 0., [0.; 4]);
        style.overrides.width = Some(ui::Length::Px(size[0]));
        style.overrides.height = Some(ui::Length::Px(size[1]));
        let body = named("diagram-fullscreen/viewport", c::Element::Spacer);
        let element = col(
            "diagram-fullscreen",
            style,
            0.,
            vec![fixed(header, 52.), fill(body)],
        );
        let mut solved = element
            .layout(Rect::new(0., 0., size[0], size[1]), ctx, measure)
            .map_err(|e| e.to_string())?;
        let viewport = find(&solved, "diagram-fullscreen/viewport")
            .ok_or("missing diagram viewport")?
            .rect;
        self.viewport = viewport;
        self.clamp();
        let scale = self.scale();
        let scaled = [self.canvas.rect.w * scale, self.canvas.rect.h * scale];
        let at = [
            viewport.x + (viewport.w - scaled[0]).max(0.) / 2. - self.offset[0],
            viewport.y + (viewport.h - scaled[1]).max(0.) / 2. - self.offset[1],
        ];
        let canvas = self
            .canvas
            .scaled_at(at, scale)
            .ok_or("invalid diagram transform")?;
        let regions = Regions {
            viewport,
            controls: ACTIONS
                .iter()
                .filter_map(|(a, _)| find(&solved, a.id()).map(|n| (n.rect, *a)))
                .collect(),
        };
        let slot = find_mut(&mut solved, "diagram-fullscreen/viewport")
            .ok_or("missing diagram viewport")?;
        slot.children = vec![c::Layout {
            rect: viewport,
            baseline: 0.,
            kind: c::LayoutKind::ScrollView {
                content: scaled,
                offset: self.offset,
            },
            children: vec![canvas],
        }];
        Ok((solved, regions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui_model::layout::{TextMeasure, TextStyle};
    struct Metrics;
    impl TextMeasure for Metrics {
        fn width(&self, value: &str, font: &TextStyle) -> f32 {
            value.chars().count() as f32 * font.size * 0.5
        }
    }
    impl c::Measure for Metrics {
        fn font_metrics(&self, font: &TextStyle) -> [f32; 2] {
            [font.size * 0.8, font.size * 0.2]
        }
    }
    fn canvas() -> Arc<c::Layout> {
        Arc::new(c::Layout {
            rect: Rect::new(50., 100., 3000., 1800.),
            baseline: 0.,
            kind: c::LayoutKind::Named("fixture/canvas".into()),
            children: vec![],
        })
    }
    #[test]
    fn fullscreen_viewer_fits_zoom_pans_and_restores_without_mutating_source() {
        let original = canvas();
        let mut viewer = Viewer::new(original.clone(), [1200., 800.]).unwrap();
        let (layout, regions) = viewer
            .layout(
                [1200., 800.],
                ui::Context::new(ui::Theme::Light, ui::Density::Cozy),
                &Metrics,
            )
            .unwrap();
        assert_eq!(layout.rect, Rect::new(0., 0., 1200., 800.));
        assert_eq!(regions.controls.len(), 5);
        assert_eq!(regions.viewport, Rect::new(0., 52., 1200., 748.));
        let fitted = viewer.scale();
        assert!((fitted - 0.4).abs() < 0.001);
        viewer.act(Action::Actual);
        viewer.pan([10000., 10000.]);
        assert_eq!(viewer.offset, [1800., 1052.]);
        viewer.pan([0., -40.]);
        assert_eq!(
            viewer.offset[0], 1800.,
            "vertical wheel never becomes horizontal pan"
        );
        viewer.pan([f32::NAN, f32::INFINITY]);
        assert!(viewer.offset.into_iter().all(f32::is_finite));
        viewer.act(Action::In);
        assert!((viewer.scale() - 1.25).abs() < 0.001);
        viewer.act(Action::Fit);
        assert_eq!(viewer.offset, [0.; 2]);
        assert_eq!(viewer.scale(), fitted);
        assert_eq!(original.rect, Rect::new(50., 100., 3000., 1800.));
        let close = regions
            .controls
            .iter()
            .find(|(_, a)| *a == Action::Close)
            .unwrap()
            .0;
        assert_eq!(
            regions.cursor([close.x + 1., close.y + 1.]),
            ui::cursor::Cursor::Pointer
        );
        assert!(viewer.press([close.x + 1., close.y + 1.], &regions));
        let (dark, _) = viewer
            .layout(
                [450., 700.],
                ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
                &Metrics,
            )
            .unwrap();
        assert_eq!(dark.rect.w, 450.);
        assert!(!c::paint(&dark, ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)).is_empty());
    }
    #[test]
    fn fullscreen_viewer_transform_preserves_path_and_text_coordinates() {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let text = c::Element::Text(c::Text::new(
            "Node",
            font(20., 600, false, ui::Color::Foreground),
        ))
        .layout(Rect::new(50., 100., 300., 40.), ctx, &Metrics)
        .unwrap();
        let scaled = text.scaled_at([10., 20.], 2.).unwrap();
        assert_eq!(scaled.rect.x, 10.);
        assert_eq!(scaled.rect.y, 20.);
        if let (c::LayoutKind::Text { runs: before, .. }, c::LayoutKind::Text { runs: after, .. }) =
            (&text.kind, &scaled.kind)
        {
            assert_eq!(after[0].style.size, before[0].style.size * 2.);
            assert_eq!(after[0].baseline, 20. + (before[0].baseline - 100.) * 2.);
        } else {
            panic!("text layout expected");
        }
        assert!(text.scaled_at([0., 0.], f32::NAN).is_none());
        assert!(text.scaled_at([f32::INFINITY, 0.], 1.).is_none());
    }
}
