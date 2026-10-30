// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native and ctl sidebar toggles and resize share this nonblocking input boundary.
use crate::*;

impl App {
    pub(super) fn toggle_sidebar_section(&mut self, key: &str) {
        let Some(current) = self.scene.sidebar_open().get(key).copied() else {
            return;
        };
        let open = !current;
        self.sidebar_accordion_overrides.insert(key.into(), open);
        self.scene.animate_sidebar_open(key, open, self.clock.now());
        if key == "panels/Chat" && !open {
            self.blur_reject();
            self.composer.handle(ComposeCommand::Blur);
            self.composer_dragging = false;
            self.sync_text_ime();
        }
        if let Some(effects) = &mut self.ui_collect {
            effects.push(UiEffect::SidebarSection {
                key: key.into(),
                open,
            });
        }
        self.request_redraw();
    }
    pub(super) fn panel_header(&self, at: [f32; 2]) -> Option<&'static str> {
        fn find<'a>(
            n: &'a bbt_ui::components::Layout,
            key: &str,
        ) -> Option<&'a bbt_ui::components::Layout> {
            if matches!(&n.kind,bbt_ui::components::LayoutKind::Named(name) if name==key) {
                Some(n)
            } else {
                n.children.iter().find_map(|n| find(n, key))
            }
        }
        let layout = &self.conversation_layout.as_ref()?.layout;
        [
            ("panels/Tasks", "conversation/task-section/head"),
            ("panels/Chat", "conversation/chat-section/head"),
        ]
        .into_iter()
        .find(|(_, name)| {
            find(layout, name).is_some_and(|n| {
                at[0] >= n.rect.x
                    && at[0] < n.rect.right()
                    && at[1] >= n.rect.y
                    && at[1] < n.rect.bottom()
            })
        })
        .map(|(key, _)| key)
    }
    pub(super) fn begin_sidebar_resize(&mut self, at: [f32; 2]) -> bool {
        if self.document.open || self.launcher.is_some() {
            return false;
        }
        let Some(handle) = self.sidebar_resize_region else {
            return self.begin_sidebar_vertical(at);
        };
        if !self
            .sidebar_drag
            .begin(at, handle, self.scene.sidebar_width_for(self.solved_size))
        {
            return self.begin_sidebar_vertical(at);
        }
        self.pointer = at;
        self.query_dragging = false;
        self.composer_dragging = false;
        if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
            self.settings_action(settings_controller::Action::EndDrag);
        }
        self.update_cursor();
        true
    }
    pub(super) fn move_sidebar_resize(&mut self, at: [f32; 2]) -> bool {
        if self.sidebar_vertical_drag.is_some() {
            return self.move_sidebar_vertical(at);
        }
        if self.sidebar_drag.active() && (self.document.open || self.launcher.is_some()) {
            self.end_sidebar_resize();
            return false;
        }
        let Some(width) = self.sidebar_drag.update(at[0]) else {
            return false;
        };
        self.pointer = at;
        let previous = self.sidebar_width.unwrap_or(self.scene.sidebar_width);
        self.sidebar_width = Some(width);
        if width != previous {
            self.request_redraw();
        }
        true
    }
    pub(super) fn end_sidebar_resize(&mut self) {
        self.sidebar_drag.finish();
        self.end_sidebar_vertical();
        self.update_cursor();
    }
    pub(super) fn drag_sidebar(&mut self, from: [f32; 2], to: [f32; 2]) -> Result<bool, String> {
        if !self.begin_sidebar_resize(from) {
            return Err("ui_drag_refused: start on a retained sidebar resize handle".into());
        }
        let vertical = self.sidebar_vertical_drag.is_some();
        self.move_sidebar_resize(to);
        self.end_sidebar_resize();
        Ok(vertical)
    }
}
