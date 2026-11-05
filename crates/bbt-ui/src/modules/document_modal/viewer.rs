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
const TOOLBAR_HEIGHT: f32 = 60.;
const TOUCH_SLOP: f32 = 8.;

const ACTIONS: [(Action, &str); 5] = [
    (Action::Out, "−"),
    (Action::In, "+"),
    (Action::Fit, "Fit"),
    (Action::Actual, "1:1"),
    (Action::Close, "Close"),
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

/// Contact transitions stay independent of the windowing backend. A fixed
/// pair bounds input state even if a device sends extra or unmatched contacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}
#[derive(Clone, Copy)]
struct Contact {
    id: u64,
    at: [f32; 2],
    start: [f32; 2],
    canvas: bool,
    tap: Option<Action>,
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
    contacts: [Option<Contact>; 2],
    controls: Vec<(Rect, Action)>,
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
            viewport: Rect::new(
                0.,
                TOOLBAR_HEIGHT,
                size[0].max(1.),
                (size[1] - TOOLBAR_HEIGHT).max(1.),
            ),
            drag: None,
            selected: 0,
            keyboard: false,
            contacts: [None; 2],
            controls: vec![],
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
                let at = [
                    self.viewport.x + self.viewport.w / 2.,
                    self.viewport.y + self.viewport.h / 2.,
                ];
                self.zoom_about(if action == Action::In { 1.25 } else { 0.8 }, at, at);
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
    fn origin(&self) -> [f32; 2] {
        let scale = self.scale();
        [
            self.viewport.x + (self.viewport.w - self.canvas.rect.w * scale).max(0.) / 2.
                - self.offset[0],
            self.viewport.y + (self.viewport.h - self.canvas.rect.h * scale).max(0.) / 2.
                - self.offset[1],
        ]
    }
    /// Keep the same canvas point under the moving gesture centroid. Centering
    /// margins matter when a fitted figure grows larger than the viewport.
    fn zoom_about(&mut self, factor: f32, from: [f32; 2], to: [f32; 2]) {
        if !factor.is_finite() || factor <= 0. || !from.into_iter().chain(to).all(f32::is_finite) {
            return;
        }
        if factor == 1. && from == to {
            return;
        }
        let old = self.scale();
        let origin = self.origin();
        let next = (old * factor).clamp(0.01, 8.);
        for i in 0..2 {
            let (start, span, content) = if i == 0 {
                (self.viewport.x, self.viewport.w, self.canvas.rect.w)
            } else {
                (self.viewport.y, self.viewport.h, self.canvas.rect.h)
            };
            self.offset[i] = start + (span - content * next).max(0.) / 2. - to[i]
                + (from[i] - origin[i]) / old * next;
        }
        self.zoom = next;
        self.fit = false;
        self.clamp();
    }
    pub fn touching(&self) -> bool {
        self.contacts.iter().any(Option::is_some)
    }
    pub fn zoom_at(&mut self, factor: f32, at: [f32; 2]) {
        if !self.touching() && contains(self.viewport, at) {
            self.zoom_about(factor, at, at);
        }
    }
    pub fn smart_zoom(&mut self, at: [f32; 2]) {
        if self.touching() || !contains(self.viewport, at) {
            return;
        }
        if self.fit {
            self.zoom_about(1. / self.scale(), at, at);
        } else {
            self.act(Action::Fit);
        }
    }
    pub fn scroll(&mut self, at: [f32; 2], delta: [f32; 2], zoom: bool) {
        if self.touching() || !contains(self.viewport, at) {
            return;
        }
        if zoom {
            if delta[1].is_finite() {
                self.zoom_at((-delta[1].clamp(-200., 200.) * 0.005).exp(), at);
            }
        } else {
            self.pan(delta);
        }
    }
    pub fn begin_pan(&mut self, at: [f32; 2]) {
        if !self.touching() && at.into_iter().all(f32::is_finite) && contains(self.viewport, at) {
            self.drag = Some(at);
        }
    }
    pub fn press(&mut self, at: [f32; 2], regions: &Regions) -> bool {
        if self.touching() {
            return false;
        }
        if let Some(action) = regions.hit(at) {
            return self.act(action);
        }
        self.begin_pan(at);
        false
    }
    pub fn drag(&mut self, at: [f32; 2]) -> bool {
        let Some(old) = self.drag else {
            return false;
        };
        if self.touching() || !at.into_iter().all(f32::is_finite) {
            return false;
        }
        self.drag = Some(at);
        self.pan([old[0] - at[0], old[1] - at[1]]);
        true
    }
    pub fn release(&mut self) {
        self.drag = None;
    }
    pub fn cancel_gestures(&mut self) {
        self.release();
        self.contacts = [None; 2];
    }
    /// Buttons activate on a completed tap, never on finger-down. Keep their
    /// last solved hit regions here: several native moves/end events may arrive
    /// before the next redraw, when the host's render cache is invalidated.
    pub fn touch(&mut self, id: u64, phase: TouchPhase, at: [f32; 2]) -> bool {
        let index = self
            .contacts
            .iter()
            .position(|c| c.is_some_and(|c| c.id == id));
        if phase == TouchPhase::Cancelled || !at.into_iter().all(f32::is_finite) {
            if let Some(i) = index {
                self.contacts[i] = None;
            }
            return false;
        }
        match phase {
            TouchPhase::Started => {
                let Some(i) = index.or_else(|| self.contacts.iter().position(Option::is_none))
                else {
                    return false;
                };
                let tap = self
                    .controls
                    .iter()
                    .find(|(r, _)| contains(*r, at))
                    .map(|(_, a)| *a);
                self.contacts[i] = Some(Contact {
                    id,
                    at,
                    start: at,
                    canvas: contains(self.viewport, at),
                    tap,
                });
                self.release();
                if self.contacts.iter().flatten().count() > 1 {
                    for c in self.contacts.iter_mut().flatten() {
                        c.tap = None;
                    }
                }
            }
            TouchPhase::Moved => {
                let Some(i) = index else {
                    return false;
                };
                let old = self.contacts;
                let c = self.contacts[i].as_mut().expect("tracked contact");
                c.at = at;
                if (at[0] - c.start[0]).hypot(at[1] - c.start[1]) > TOUCH_SLOP {
                    c.tap = None;
                }
                if let ([Some(a), Some(b)], [Some(c), Some(d)]) = (old, self.contacts)
                    && a.canvas
                    && b.canvas
                {
                    let from = [(a.at[0] + b.at[0]) / 2., (a.at[1] + b.at[1]) / 2.];
                    let to = [(c.at[0] + d.at[0]) / 2., (c.at[1] + d.at[1]) / 2.];
                    let before = (a.at[0] - b.at[0]).hypot(a.at[1] - b.at[1]);
                    let after = (c.at[0] - d.at[0]).hypot(c.at[1] - d.at[1]);
                    if before > 1. && after > 1. {
                        self.zoom_about(after / before, from, to);
                    }
                } else if let Some(c) = old[i]
                    && c.canvas
                {
                    self.pan([c.at[0] - at[0], c.at[1] - at[1]]);
                }
            }
            TouchPhase::Ended => {
                let Some(c) = index.and_then(|i| self.contacts[i].take()) else {
                    return false;
                };
                if let Some(action) = c.tap
                    && (at[0] - c.start[0]).hypot(at[1] - c.start[1]) <= TOUCH_SLOP
                    && self
                        .controls
                        .iter()
                        .any(|(r, a)| *a == action && contains(*r, at))
                {
                    return self.act(action);
                }
            }
            TouchPhase::Cancelled => {}
        }
        false
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
        self.viewport = Rect::new(
            0.,
            TOOLBAR_HEIGHT,
            size[0].max(1.),
            (size[1] - TOOLBAR_HEIGHT).max(1.),
        );
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
            let mut style = surface(
                if active {
                    ui::Color::Accent
                } else {
                    ui::Color::Muted
                },
                0.12,
                8.,
                [6., 10., 6., 10.],
            );
            style.overrides.height = Some(ui::Length::Px(44.));
            let mut button = natural(row(
                action.id(),
                style,
                0.,
                vec![
                    fill(c::Element::Spacer),
                    natural(c::Element::Text(c::Text::new(
                        *label,
                        font(13., 500, false, ui::Color::Foreground),
                    ))),
                    fill(c::Element::Spacer),
                ],
            ));
            if let c::Extent::Content { min, .. } = &mut button.extent {
                *min = 44.;
            }
            children.push(button);
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
            vec![fixed(header, TOOLBAR_HEIGHT), fill(body)],
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
        self.controls = regions.controls.clone();
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
        assert!(
            regions
                .controls
                .iter()
                .all(|(r, _)| r.w >= 43.99 && r.h >= 43.99),
            "touch controls must be at least 44px"
        );
        assert_eq!(
            regions.viewport,
            Rect::new(0., TOOLBAR_HEIGHT, 1200., 800. - TOOLBAR_HEIGHT)
        );
        let fitted = viewer.scale();
        assert!((fitted - 0.4).abs() < 0.001);
        viewer.act(Action::Actual);
        viewer.pan([10000., 10000.]);
        assert_eq!(viewer.offset, [1800., 1060.]);
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
    fn prepared() -> Viewer {
        let mut viewer = Viewer::new(canvas(), [1200., 800.]).unwrap();
        viewer
            .layout(
                [1200., 800.],
                ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
                &Metrics,
            )
            .unwrap();
        viewer.act(Action::Actual);
        viewer.pan([400., 300.]);
        viewer
    }
    fn canvas_point(viewer: &Viewer, at: [f32; 2]) -> [f32; 2] {
        let origin = viewer.origin();
        [
            (at[0] - origin[0]) / viewer.scale(),
            (at[1] - origin[1]) / viewer.scale(),
        ]
    }
    fn near(a: [f32; 2], b: [f32; 2]) {
        assert!(
            (a[0] - b[0]).abs() < 0.002 && (a[1] - b[1]).abs() < 0.002,
            "{a:?} != {b:?}"
        );
    }
    #[test]
    fn fullscreen_viewer_pointer_zoom_keeps_the_canvas_under_the_pointer() {
        let mut viewer = prepared();
        let at = [500., 320.];
        let point = canvas_point(&viewer, at);
        viewer.zoom_at(1.6, at);
        near(canvas_point(&viewer, at), point);
        viewer.zoom_at(1. / 1.6, at);
        near(canvas_point(&viewer, at), point);
        assert!((viewer.scale() - 1.).abs() < 0.001);
        viewer.act(Action::Fit);
        let at = [600., 430.];
        let point = canvas_point(&viewer, at);
        viewer.zoom_at(2., at);
        near(canvas_point(&viewer, at), point);
        let scale = viewer.scale();
        viewer.scroll(at, [0., 30.], false);
        assert_eq!(
            viewer.scale(),
            scale,
            "trackpad scroll pans without changing zoom"
        );
        let x = viewer.offset[0];
        viewer.scroll(at, [0., 20.], false);
        assert_eq!(viewer.offset[0], x);
        viewer.scroll(at, [0., -1.], true);
        assert!(
            viewer.scale() > scale && viewer.scale() < scale * 1.01,
            "wheel zoom is continuous"
        );
        let before = (viewer.scale(), viewer.offset);
        for factor in [f32::NAN, f32::INFINITY, -1., 0.] {
            viewer.zoom_at(factor, at);
        }
        viewer.zoom_at(2., [f32::NAN, 0.]);
        viewer.scroll([0., 0.], [0., 20.], true);
        assert_eq!((viewer.scale(), viewer.offset), before);
    }
    #[test]
    fn fullscreen_viewer_two_finger_pinch_and_pan_share_one_centroid() {
        use TouchPhase::*;
        let mut viewer = prepared();
        let initial = viewer.offset;
        viewer.touch(1, Started, [300., 300.]);
        viewer.touch(1, Moved, [280., 310.]);
        near(viewer.offset, [initial[0] + 20., initial[1] - 10.]);
        viewer.touch(1, Moved, [300., 300.]);
        viewer.touch(2, Started, [500., 300.]);
        let point = canvas_point(&viewer, [400., 300.]);
        viewer.touch(1, Moved, [250., 300.]);
        viewer.touch(2, Moved, [550., 300.]);
        assert!((viewer.scale() - 1.5).abs() < 0.002);
        near(canvas_point(&viewer, [400., 300.]), point);
        viewer.touch(1, Moved, [290., 320.]);
        viewer.touch(2, Moved, [590., 320.]);
        assert!((viewer.scale() - 1.5).abs() < 0.002);
        near(canvas_point(&viewer, [440., 320.]), point);
        let offset = viewer.offset;
        viewer.touch(2, Ended, [590., 320.]);
        viewer.touch(1, Moved, [280., 320.]);
        near(viewer.offset, [offset[0] + 10., offset[1]]);
        viewer.touch(1, Ended, [280., 320.]);
        assert!(!viewer.touching());
    }
    #[test]
    fn fullscreen_viewer_touch_cancel_ids_and_completed_button_taps() {
        use TouchPhase::*;
        let mut viewer = prepared();
        let close = viewer
            .controls
            .iter()
            .find(|(_, a)| *a == Action::Close)
            .unwrap()
            .0;
        let at = [close.x + close.w / 2., close.y + close.h / 2.];
        assert!(!viewer.touch(1, Started, at));
        assert!(!viewer.touch(1, Cancelled, at));
        assert!(!viewer.touch(1, Ended, at));
        viewer.touch(1, Started, at);
        viewer.touch(1, Moved, [at[0] + 9., at[1]]);
        assert!(
            !viewer.touch(1, Ended, at),
            "a dragged toolbar control is not a tap"
        );
        assert!(!viewer.touch(1, Started, at));
        assert!(viewer.touch(1, Ended, at));
        viewer.touch(1, Started, [300., 300.]);
        viewer.touch(2, Started, [500., 300.]);
        viewer.touch(3, Started, at);
        assert!(!viewer.touch(3, Ended, at));
        assert_eq!(viewer.contacts.iter().flatten().count(), 2);
        let before = (viewer.scale(), viewer.offset);
        viewer.zoom_at(2., [400., 300.]);
        viewer.scroll([400., 300.], [20., 20.], false);
        assert_eq!(
            (viewer.scale(), viewer.offset),
            before,
            "raw touch owns its gesture; synthetic mouse/pinch cannot double-apply it"
        );
        viewer.cancel_gestures();
        viewer.touch(1, Moved, [100., 200.]);
        assert!(!viewer.touch(2, Ended, at));
        assert_eq!((viewer.scale(), viewer.offset), before);
        viewer.touch(1, Started, [300., 300.]);
        viewer.touch(1, Ended, [f32::NAN, 0.]);
        assert!(!viewer.touching());
        // Reused ids establish a new baseline; cancelling/ending an old contact
        // cannot keep a drag alive after focus loss or a DPI/viewport change.
        viewer.touch(1, Started, [300., 300.]);
        viewer.touch(1, Started, [600., 400.]);
        let offset = viewer.offset;
        viewer.touch(1, Moved, [590., 400.]);
        near(viewer.offset, [offset[0] + 10., offset[1]]);
        viewer.cancel_gestures();
        assert!(!viewer.drag([400., 300.]));
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
