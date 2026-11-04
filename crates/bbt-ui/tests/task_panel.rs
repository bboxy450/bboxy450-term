// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::stream_dock::{Action, Entry, Regions, StreamDock, TaskState},
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind, c::LayoutKind::Named(name) if name == key) {
        Some(n)
    } else {
        n.children.iter().find_map(|child| find(child, key))
    }
}
#[test]
fn task_loading_and_retry_are_centered_and_refresh_is_clickable() {
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        for size in [[160., 220.], [240., 260.], [420., 700.], [840., 480.]] {
            for loading in [false, true] {
                let state = TaskState {
                    loading,
                    message: if loading { "" } else { "Workspace is opening." }.into(),
                    retry_in: Some(5),
                    phase: 1,
                };
                let layout = state
                    .element(ctx, size)
                    .layout(Rect::new(0., 0., size[0], size[1]), ctx, &Metrics)
                    .unwrap();
                let content = find(&layout, "conversation/tasks-state/content")
                    .unwrap()
                    .rect;
                assert!(
                    content.x >= 0. && content.right() <= size[0] + 1.,
                    "content stays inside {size:?}: {content:?}"
                );
                assert!(
                    (content.x + content.w / 2. - size[0] / 2.).abs() < 1.,
                    "horizontal {size:?}: {content:?}"
                );
                assert!(
                    (content.y + content.h / 2. - size[1] / 2.).abs() < 1.,
                    "vertical {size:?}: {content:?}"
                );
                if !loading {
                    let button = find(&layout, "conversation/tasks-refresh").unwrap().rect;
                    let view = StreamDock {
                        entries: vec![Entry::TaskState { data: state }],
                        ..Default::default()
                    };
                    let regions = Regions::dock(&layout, &view);
                    assert_eq!(
                        regions.hit([button.x + button.w / 2., button.y + button.h / 2.]),
                        Some(Action::RefreshTasks)
                    );
                }
            }
        }
    }
}
