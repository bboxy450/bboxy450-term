// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained title-bar hit regions and window gestures, shared by native input and ctl.
use bboxy450_term_desktop::{Action, TitlebarAction};
use bbt_ui::components::{Layout, LayoutKind};
use bbt_ui_model::layout::Rect;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Hit {
    Close,
    Minimize,
    Fullscreen,
    Sidebar,
    Theme,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Intent {
    Window(Action),
    Titlebar(TitlebarAction),
    Sidebar,
    Theme,
    /// The caller may start an OS drag only for a physical mouse-down event.
    Drag,
}

/// Geometry comes from the painted tree; no platform-specific window dimensions leak here.
#[derive(Default)]
pub(super) struct Titlebar {
    bounds: Option<Rect>,
    controls: Vec<(Hit, Rect)>,
    click: Option<(Instant, [f32; 2])>,
}

fn contains(rect: Rect, at: [f32; 2]) -> bool {
    at[0] >= rect.x && at[0] < rect.right() && at[1] >= rect.y && at[1] < rect.bottom()
}

fn named<'a>(layout: &'a Layout, key: &str) -> Option<&'a Layout> {
    if matches!(&layout.kind, LayoutKind::Named(name) if name == key) {
        return Some(layout);
    }
    layout.children.iter().find_map(|child| named(child, key))
}

impl Titlebar {
    pub(super) fn retain(&mut self, layouts: &[Layout]) {
        self.bounds = layouts
            .iter()
            .find_map(|layout| named(layout, "bar"))
            .map(|l| l.rect);
        self.controls = [
            (Hit::Close, "bar/win/i"),
            (Hit::Minimize, "bar/win/i1"),
            (Hit::Fullscreen, "bar/win/i2"),
            (Hit::Sidebar, "bar/tbtn"),
            (Hit::Theme, "bar/tbtn1"),
        ]
        .into_iter()
        .filter_map(|(hit, key)| {
            layouts
                .iter()
                .find_map(|layout| named(layout, key))
                .map(|layout| (hit, layout.rect))
        })
        .collect();
        if self.bounds.is_none() {
            self.click = None;
        }
    }

    pub(super) fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        if !self.bounds.is_some_and(|bounds| contains(bounds, at)) {
            return None;
        }
        Some(
            self.controls
                .iter()
                .find(|(_, rect)| contains(*rect, at))
                .map_or(Hit::Background, |(hit, _)| *hit),
        )
    }

    pub(super) fn actionable(&self, at: [f32; 2]) -> bool {
        matches!(
            self.hit(at),
            Some(Hit::Close | Hit::Minimize | Hit::Fullscreen)
        )
    }

    /// A drag, a click elsewhere, or invalid coordinates cannot complete a double-click.
    pub(super) fn moved(&mut self, at: [f32; 2]) {
        if self.click.is_some_and(|(_, from)| !near(from, at)) {
            self.click = None;
        }
    }

    pub(super) fn press(
        &mut self,
        at: [f32; 2],
        now: Instant,
        interval: Duration,
        double_click: TitlebarAction,
    ) -> Option<Intent> {
        let Some(hit) = self.hit(at) else {
            self.click = None;
            return None;
        };
        if hit != Hit::Background {
            self.click = None;
            return Some(match hit {
                Hit::Close => Intent::Window(Action::CloseWindow),
                Hit::Minimize => Intent::Window(Action::Minimize),
                Hit::Fullscreen => Intent::Window(Action::ToggleFullscreen),
                Hit::Sidebar => Intent::Sidebar,
                Hit::Theme => Intent::Theme,
                Hit::Background => unreachable!(),
            });
        }
        if self.click.take().is_some_and(|(previous, from)| {
            !interval.is_zero()
                && now
                    .checked_duration_since(previous)
                    .is_some_and(|elapsed| elapsed <= interval)
                && near(from, at)
        }) {
            return Some(Intent::Titlebar(double_click));
        }
        self.click = Some((now, at));
        Some(Intent::Drag)
    }
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy <= 16.
}

impl crate::App {
    /// Native mouse-down and ctl click use identical hit/gesture dispatch. Only the
    /// native caller may consume an NSEvent to begin a window drag.
    pub(super) fn titlebar_pressed(&mut self, at: [f32; 2], native_drag: bool) -> bool {
        if matches!(
            self.titlebar.hit(at),
            None | Some(Hit::Sidebar | Hit::Theme)
        ) {
            // Sidebar/theme are retained controls, so they can never be a
            // draggable background or the second half of a title-bar gesture.
            self.titlebar.click = None;
            return false;
        }
        let intent = self.titlebar.press(
            at,
            self.clock.now(),
            bboxy450_term_desktop::double_click_interval(),
            bboxy450_term_desktop::titlebar_action(),
        );
        match intent {
            Some(Intent::Window(action)) => {
                let _ = self.proxy.send_event(crate::UiEvent::DesktopAction(action));
            }
            Some(Intent::Titlebar(_)) => {
                let _ = self.proxy.send_event(crate::UiEvent::TitlebarDoubleClick);
            }
            Some(Intent::Drag) if native_drag => {
                if let Some(window) = &self.window
                    && let Err(error) = window.drag_window()
                {
                    self.logger.write(format!("desktop window drag: {error}"));
                }
            }
            _ => {}
        }
        intent.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titlebar() -> Titlebar {
        Titlebar {
            bounds: Some(Rect::new(0., 0., 400., 38.)),
            controls: vec![
                (Hit::Close, Rect::new(14., 14., 10., 10.)),
                (Hit::Minimize, Rect::new(30., 14., 10., 10.)),
                (Hit::Fullscreen, Rect::new(46., 14., 10., 10.)),
                (Hit::Sidebar, Rect::new(60., 8., 26., 26.)),
                (Hit::Theme, Rect::new(370., 8., 26., 26.)),
            ],
            click: None,
        }
    }

    // keeps: operator standard title-bar behavior — controls never begin window gestures.
    #[test]
    fn controls_follow_painted_regions_and_exclude_double_clicks_and_drag() {
        let mut bar = titlebar();
        let now = Instant::now();
        for (point, intent) in [
            ([19., 19.], Intent::Window(Action::CloseWindow)),
            ([35., 19.], Intent::Window(Action::Minimize)),
            ([51., 19.], Intent::Window(Action::ToggleFullscreen)),
            ([70., 19.], Intent::Sidebar),
            ([380., 19.], Intent::Theme),
        ] {
            for _ in 0..2 {
                assert_eq!(
                    bar.press(point, now, Duration::from_millis(500), TitlebarAction::Zoom),
                    Some(intent)
                );
                assert_eq!(
                    bar.actionable(point),
                    !matches!(intent, Intent::Sidebar | Intent::Theme)
                );
            }
        }
        assert_eq!(bar.hit([400., 10.]), None);
        assert_eq!(bar.hit([100., 38.]), None);
        assert_eq!(bar.hit([100., f32::NAN]), None);
        assert!(!bar.actionable([200., 20.]));
    }

    // keeps: operator standard title-bar behavior — honor platform policy and timing.
    #[test]
    fn double_clicks_honor_interval_distance_policy_and_do_not_overlap() {
        let interval = Duration::from_millis(500);
        let now = Instant::now();
        for policy in [
            TitlebarAction::Fill,
            TitlebarAction::Zoom,
            TitlebarAction::Minimize,
            TitlebarAction::None,
        ] {
            let mut bar = titlebar();
            assert_eq!(
                bar.press([200., 20.], now, interval, policy),
                Some(Intent::Drag)
            );
            assert_eq!(
                bar.press([202., 21.], now + interval, interval, policy),
                Some(Intent::Titlebar(policy))
            );
            assert_eq!(
                bar.press([200., 20.], now + interval, interval, policy),
                Some(Intent::Drag)
            );
        }
        for (elapsed, at) in [(501, [200., 20.]), (100, [205., 20.])] {
            let mut bar = titlebar();
            bar.press([200., 20.], now, interval, TitlebarAction::Zoom);
            assert_eq!(
                bar.press(
                    at,
                    now + Duration::from_millis(elapsed),
                    interval,
                    TitlebarAction::Zoom
                ),
                Some(Intent::Drag)
            );
        }
    }

    #[test]
    fn drag_movement_and_non_titlebar_clicks_cancel_the_first_click() {
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        let mut bar = titlebar();
        bar.press([200., 20.], now, interval, TitlebarAction::Zoom);
        bar.moved([220., 20.]);
        bar.moved([200., 20.]);
        assert_eq!(
            bar.press([200., 20.], now, interval, TitlebarAction::Zoom),
            Some(Intent::Drag)
        );
        assert_eq!(
            bar.press([200., 200.], now, interval, TitlebarAction::Zoom),
            None
        );
        assert_eq!(
            bar.press([200., 20.], now, interval, TitlebarAction::Zoom),
            Some(Intent::Drag)
        );
    }
}
