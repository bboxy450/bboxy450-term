// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two retained horizontal handles share native and unattended ctl pointer input.
use crate::*;
use bbt_ui::modules::composer::SidebarLayout;

#[derive(Debug, Clone, Copy)]
pub(super) struct VerticalDrag {
    index: usize,
    from_y: f32,
    value: f32,
    available: f32,
}
fn contains(r: Rect, p: [f32; 2]) -> bool {
    p[0] >= r.x && p[0] < r.right() && p[1] >= r.y && p[1] < r.bottom()
}
impl App {
    pub(super) fn sidebar_vertical_handle(&self, point: [f32; 2]) -> Option<usize> {
        if self.document.open || self.launcher.is_some() {
            return None;
        }
        self.conversation_layout
            .as_ref()?
            .splitters
            .iter()
            .position(|rect| contains(*rect, point))
    }
    pub(super) fn begin_sidebar_vertical(&mut self, point: [f32; 2]) -> bool {
        let Some(index) = self.sidebar_vertical_handle(point) else {
            return false;
        };
        if !self.preferences.loaded {
            return false;
        }
        let layout = self.conversation_layout.as_ref().unwrap();
        self.sidebar_vertical_drag = Some(VerticalDrag {
            index,
            from_y: point[1],
            available: layout.available_height,
            value: if index == 0 {
                layout.height
            } else {
                layout.regions.body.h
            },
        });
        self.sidebar_layout_preview = Some(self.preferences.preferences.sidebar);
        self.pointer = point;
        self.query_dragging = false;
        self.composer_dragging = false;
        if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
            self.settings_action(settings_controller::Action::EndDrag);
        }
        self.update_cursor();
        true
    }
    pub(super) fn move_sidebar_vertical(&mut self, point: [f32; 2]) -> bool {
        let Some(drag) = self.sidebar_vertical_drag else {
            return false;
        };
        if self.document.open || self.launcher.is_some() {
            self.end_sidebar_vertical();
            return false;
        }
        if !point.into_iter().all(f32::is_finite) {
            return true;
        }
        let layout = self
            .sidebar_layout_preview
            .get_or_insert(self.preferences.preferences.sidebar);
        let value = drag.value - (point[1] - drag.from_y);
        if drag.index == 0 {
            layout.conversation_fraction = (value / drag.available.max(1.)).clamp(0.05, 0.95);
        } else {
            layout.composer_body_height = Some(value.clamp(0., 4096.));
        }
        self.pointer = point;
        self.request_redraw();
        true
    }
    pub(super) fn end_sidebar_vertical(&mut self) {
        if self.sidebar_vertical_drag.take().is_none() {
            return;
        }
        let Some(layout) = self.sidebar_layout_preview.take() else {
            return;
        };
        let patch = settings_controller::Patch::SidebarLayout {
            conversation_fraction: layout.conversation_fraction,
            composer_body_height: layout.composer_body_height,
        };
        match self.preferences.edit(patch) {
            Ok(request) => match &self.preferences_io {
                Some(io) => {
                    if let Err(error) = io.submit(request) {
                        self.preferences.refused(error);
                    }
                }
                None => self
                    .preferences
                    .refused("Settings persistence is unavailable"),
            },
            Err(error) => self.preferences.refused(error),
        }
        self.request_redraw();
    }
    pub(super) fn scroll_composer_body(&mut self, dy: f32) {
        if !dy.is_finite() {
            return;
        }
        if let Some(layout) = &self.conversation_layout {
            let regions = &layout.regions;
            self.composer.view.body_scroll =
                Some((regions.body_offset + dy).clamp(0., regions.body_max_scroll));
            self.request_redraw();
        }
    }
    pub(super) fn effective_sidebar_layout(&self) -> SidebarLayout {
        self.sidebar_layout_preview
            .unwrap_or(self.preferences.preferences.sidebar)
    }
}
