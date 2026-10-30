// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native pointer input and launcher navigation share one dispatch boundary.
use crate::*;
impl App {
    pub(super) fn solve_launcher(&mut self, size: [f32; 2]) -> Result<(), String> {
        self.launcher_layout = if let Some(controller) = &self.launcher {
            let launcher = &controller.launcher;
            let build = |offset| {
                self.scene.launcher_layout_with_preview(
                    size,
                    launcher,
                    Some(self.scene.launcher_preview(launcher, self.launcher_preset)),
                    offset,
                )
            };
            let mut layout = build(self.launcher_scroll)?;
            if self.launcher_reveal
                && let Some(row) = layout.rows.get(launcher.view().selected)
            {
                let delta = if row.y < layout.viewport.y {
                    row.y - layout.viewport.y
                } else {
                    (row.bottom() - layout.viewport.bottom()).max(0.)
                };
                let next = (self.launcher_scroll + delta).clamp(0., layout.max_scroll);
                if next != self.launcher_scroll {
                    self.launcher_scroll = next;
                    layout = build(next)?;
                }
            }
            self.launcher_reveal = false;
            Some(layout)
        } else {
            None
        };
        Ok(())
    }
    pub(super) fn scroll_launcher(&mut self, delta: f32) {
        if delta.is_finite()
            && let Some(layout) = &self.launcher_layout
        {
            self.launcher_scroll = (self.launcher_scroll + delta).clamp(0., layout.max_scroll);
            self.launcher_reveal = false;
            self.request_redraw();
        }
    }

    pub(super) fn launch_directory_checked(
        &mut self,
        token: bbt_mux::Ulid,
        value: String,
        valid: bool,
    ) {
        if self.launch_directory_pending == Some(token) {
            self.launch_directory_pending = None;
            if let Some(controller) = &mut self.launcher {
                if valid {
                    controller.accept_directory(value);
                }
                self.launch_directory_error =
                    (!valid).then(|| "Directory does not exist · choose an existing folder".into());
                self.update_launcher_runs();
                self.launcher_layout = None;
                self.request_redraw();
            }
        }
    }
    pub(super) fn scroll_sidebar(&mut self, delta: f32) {
        if let Some((_, max)) = self.sidebar_viewport
            && let Some(motion) = self
                .sidebar_wheel
                .wheel(self.scene.sidebar_scroll, max, delta)
            && motion.changed
        {
            self.sidebar_scroll = Some(motion.offset);
            self.request_redraw();
        }
    }
    /// Complete the same pointer gesture for native input and private ctl clicks.
    pub(super) fn pointer_released(&mut self) {
        self.terminal_selection.release();
        self.ui_selection.release();
        if self.app_overlays.open() {
            return;
        }
        self.end_sidebar_resize();
        self.query_dragging = false;
        self.composer_dragging = false;
        if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
            self.settings_action(settings_controller::Action::EndDrag);
        }
        self.update_cursor();
    }
    pub(super) fn pointer_pressed(&mut self, at: [f32; 2]) {
        self.pointer = at;
        if self.ui_selection.text().is_some() {
            self.request_redraw();
        }
        self.ui_selection.clear();
        if self.overlay_pointer(at) {
            return;
        }
        if self.document.open {
            use bbt_ui::modules::document_modal::controller::Action;
            let action = self
                .document_layout
                .as_ref()
                .and_then(|l| l.regions.hit(self.pointer));
            match action {
                Some(Action::Open(target)) => self.follow_document_link(target),
                Some(Action::Close) => self.close_document(),
                Some(Action::ToggleProvenance) => {
                    self.document.scroll.provenance_expanded =
                        !self.document.scroll.provenance_expanded;
                    self.document_layout = None;
                    self.request_redraw();
                }
                Some(Action::Unavailable(name)) => {
                    self.document.notice = Some(format!("{name} is not available yet"));
                    self.document_layout = None;
                    self.request_redraw();
                }
                None => {
                    if self.ui_selection.begin(at, self.scene.text_metrics()) {
                        self.request_redraw();
                    }
                }
            }
            return;
        }
        if self.launcher.is_some() {
            if let Some(layout) = &self.launcher_layout {
                if layout.query_contains(self.pointer) {
                    let at = self.scene.query_index(
                        layout,
                        &self.launcher.as_ref().unwrap().launcher.query,
                        self.pointer[0],
                    );
                    self.query_dragging = true;
                    self.launcher_command(LauncherCommand::SetCaret {
                        at,
                        extend: self.modifiers.shift_key(),
                    });
                } else if let Some(index) = layout.row_at(self.pointer) {
                    self.launcher_command(LauncherCommand::ActivateAt {
                        index,
                        modifiers: input_modifiers(self.modifiers),
                    });
                } else if !layout.contains(self.pointer) {
                    self.close_launcher();
                }
            }
            return;
        }
        if self.titlebar_pressed(at, false) {
            return;
        }
        if self.begin_sidebar_resize(at) {
            return;
        }
        let hit =
            |r: &Rect| at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom();
        let child_control = self.close_regions.iter().any(|(_, r, _)| hit(r))
            || self.swarm_folder_regions.iter().any(|(_, r, _)| hit(r));
        if !child_control
            && let Some((id, _)) = self.navigation_regions.iter().find(|(_, r)| {
                self.pointer[0] >= r.x
                    && self.pointer[0] < r.right()
                    && self.pointer[1] >= r.y
                    && self.pointer[1] < r.bottom()
            })
        {
            let id = id.clone();
            self.logger.write(format!("ui navigation: {id}"));
            self.navigate_to(&id);
            return;
        }
        if let Some(layout) = &self.conversation_layout
            && layout.regions.contains(self.pointer)
        {
            use composer::Hit;
            let hit = layout.regions.hit(self.pointer);
            if hit.is_none()
                && let Some(bbt_ui::modules::stream_dock::Action::OpenRecord(target)) =
                    layout.actions.hit(self.pointer)
            {
                self.open_document(target);
                return;
            }
            let at = if hit == Some(Hit::AttachQuery) {
                self.composer.view.attach_pick.as_ref().map(|p| {
                    self.scene
                        .conversation_attachment_index(layout, &p.query, self.pointer)
                })
            } else if hit == Some(Hit::Body) {
                Some(
                    self.scene
                        .conversation_index(layout, &self.composer.view.text, self.pointer),
                )
            } else {
                None
            };
            if hit.is_none()
                && self
                    .ui_selection
                    .begin(self.pointer, self.scene.text_metrics())
            {
                self.terminal_selection.clear();
                self.request_redraw();
                return;
            }
            self.blur_reject();
            match hit {
                Some(Hit::Body) => {
                    self.composer_command(ComposeCommand::Focus(ComposeField::Body));
                    self.composer_dragging = true;
                    self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                        at: at.unwrap(),
                        extend: self.modifiers.shift_key(),
                    }));
                }
                Some(Hit::Address) => {
                    self.composer_command(ComposeCommand::Focus(ComposeField::Address))
                }
                Some(Hit::Choose(index)) => self.composer_command(ComposeCommand::Choose(index)),
                Some(Hit::Remove(index)) => self.composer_command(ComposeCommand::Remove(index)),
                Some(Hit::Priority) => self.composer_command(ComposeCommand::Priority(
                    (self.composer.view.priority + 1) % 10,
                )),
                Some(Hit::Kind) => self.composer_command(ComposeCommand::ToggleKindMenu),
                Some(Hit::KindChoice(index)) => {
                    self.composer_command(ComposeCommand::Kind(composer::SEND_KINDS[index].into()))
                }
                Some(Hit::Send) => self.composer_command(ComposeCommand::Send),
                Some(Hit::Attach) => self.composer_command(ComposeCommand::PickAttachments),
                Some(Hit::AttachChoice(index)) => {
                    self.composer_command(ComposeCommand::ChooseAttachment(index))
                }
                Some(Hit::AttachQuery) => {
                    self.composer_dragging = true;
                    self.composer_command(ComposeCommand::Edit(EditCommand::SetCaret {
                        at: at.unwrap(),
                        extend: self.modifiers.shift_key(),
                    }));
                }
                _ => {}
            }
            return;
        }
        if self
            .local_page_layout
            .as_ref()
            .is_some_and(|p| p.closes(self.pointer))
        {
            self.close_local_tab();
            return;
        }
        if let Some(regions) = self
            .local_page_layout
            .as_ref()
            .and_then(|p| p.settings.as_ref())
            && regions.hit(self.pointer).is_some()
        {
            use settings_controller::{Action, Focus, Hit};
            let action = match regions.hit(self.pointer).unwrap() {
                Hit::Search => Action::Focus(Focus::Search),
                Hit::Section(name) => Action::Section(name),
                Hit::Set(patch) => Action::Set(patch),
                Hit::Attention(rule) => Action::Attention(rule),
                Hit::Toggle(field) => Action::Toggle(field),
                Hit::Reset(field) => Action::Reset(field),
                Hit::Rim => Action::Drag(regions.rim_fraction(self.pointer[0]).unwrap_or(0.)),
            };
            self.blur_reject();
            self.composer.handle(ComposeCommand::Blur);
            self.settings_action(action);
            return;
        }
        if self
            .local_page_layout
            .as_ref()
            .is_some_and(|p| p.focuses(self.pointer))
        {
            if self.ui_selection.begin(at, self.scene.text_metrics()) {
                self.terminal_selection.clear();
                self.request_redraw();
                return;
            }
            self.blur_reject();
            self.composer.handle(ComposeCommand::Blur);
            self.composer_dragging = false;
            if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
                self.settings_action(settings_controller::Action::Blur);
            }
            if let Some(window) = &self.window {
                window.set_ime_allowed(false);
            }
            self.request_redraw();
            return;
        }
        if let Some(action) = self
            .conversation_layout
            .as_ref()
            .and_then(|layout| layout.actions.hit(self.pointer))
            .or_else(|| self.task_regions.hit(self.pointer))
        {
            self.task_action(action);
            return;
        }
        if self
            .conversation_layout
            .as_ref()
            .is_some_and(|layout| layout.actions.contains("conversation/rows", self.pointer))
        {
            self.conversation_reads(true);
            if self
                .ui_selection
                .begin(self.pointer, self.scene.text_metrics())
            {
                self.terminal_selection.clear();
                self.composer.handle(ComposeCommand::Blur);
                self.request_redraw();
            }
            return;
        }
        if let Some((op, _)) = self
            .starting_close_regions
            .iter()
            .copied()
            .find(|(_, rect)| {
                self.pointer[0] >= rect.x
                    && self.pointer[0] < rect.right()
                    && self.pointer[1] >= rect.y
                    && self.pointer[1] < rect.bottom()
            })
        {
            self.dismiss_starting(op);
            return;
        }
        if let Some((seat, _, undo)) = self.close_regions.iter().copied().find(|(_, rect, _)| {
            self.pointer[0] >= rect.x
                && self.pointer[0] < rect.right()
                && self.pointer[1] >= rect.y
                && self.pointer[1] < rect.bottom()
        }) {
            self.close_seat(seat, bbt_mux::Ulid::new(), undo);
            return;
        }
        if let Some((swarm, _, reset)) =
            self.swarm_folder_regions
                .iter()
                .copied()
                .find(|(_, rect, _)| {
                    self.pointer[0] >= rect.x
                        && self.pointer[0] < rect.right()
                        && self.pointer[1] >= rect.y
                        && self.pointer[1] < rect.bottom()
                })
        {
            if reset {
                let _ = self.set_swarm_workdir(swarm, None);
            } else if !self.swarm_folder_pending {
                self.swarm_folder_pending = true;
                let directory = self
                    .live_state
                    .as_ref()
                    .and_then(|model| {
                        model
                            .ui
                            .window
                            .sidebar
                            .swarms
                            .iter()
                            .find(|row| row.id == swarm)
                    })
                    .and_then(|row| row.workdir.clone());
                if let Err(reason) =
                    swarm_folder::choose(swarm, self.proxy.clone(), directory.as_deref())
                {
                    self.swarm_folder_pending = false;
                    self.logger.write(reason);
                }
            }
            return;
        }
        if let Some((id, _)) = self.focus_bounds.iter().find(|(_, rect)| {
            self.pointer[0] >= rect.x
                && self.pointer[0] < rect.right()
                && self.pointer[1] >= rect.y
                && self.pointer[1] < rect.bottom()
        }) {
            let id = *id;
            self.focus_action(FocusAction::Activate(id, FocusOrigin::Pane));
            if self.modifiers.super_key() && self.open_terminal_link(id, at) {
                return;
            }
            self.begin_terminal_selection(id, at);
        }
    }
    pub(super) fn select_empty_swarm(&mut self, swarm: bbt_proto::SwarmId) {
        self.sidebar_reveal = Some(swarm);
        self.starting.release_focus();
        if self.document.open {
            self.close_document();
        }
        if self.local_tab.is_some() {
            self.close_local_tab();
        }
        self.blur_reject();
        self.composer.handle(ComposeCommand::Blur);
        self.focus.pending_swarm = self
            .live_state
            .as_ref()
            .is_none_or(|m| !m.ui.window.sidebar.swarms.iter().any(|s| s.id == swarm))
            .then_some(swarm);
        self.focus.stage = Some(bbt_ui_model::Stage {
            swarm: Some(swarm),
            tab: None,
            focused: None,
        });
        self.scene_dirty = true;
        self.request_redraw();
    }
    pub(super) fn navigate_to(&mut self, id: &str) {
        if let Some(seat) = id
            .strip_prefix("seat-mute:")
            .or_else(|| id.strip_prefix("pane-mute:"))
            .and_then(|id| id.parse::<bbt_ui_model::SeatId>().ok())
        {
            self.mute_seat(seat);
            return;
        }
        if let Some(swarm) = id.strip_prefix("toggle-swarm:") {
            self.toggle_sidebar_section(&format!("swarm:{swarm}"));
            return;
        }
        if let Some(section) = id.strip_prefix("toggle-section:") {
            self.toggle_sidebar_section(section);
            return;
        }
        if id == "new-swarm" {
            let id = bbt_proto::SwarmId::from_u128(bbt_mux::Ulid::new().0);
            self.queue(live::UiMsg::NewSwarm(id));
            self.select_empty_swarm(id);
            return;
        }
        if id == "settings" {
            self.open_settings("Account");
            return;
        }
        if let Some(swarm) = id.strip_prefix("new-seat:") {
            self.navigate_to(&format!("swarm:{swarm}"));
            self.open_launcher();
            return;
        }
        if let Some(seat) = id
            .strip_prefix("seat:")
            .or_else(|| id.strip_prefix("mini:"))
            .and_then(|id| id.parse::<bbt_ui_model::SeatId>().ok())
        {
            self.focus_action(FocusAction::Activate(seat, FocusOrigin::Row));
            return;
        }
        let target = self.live_state.as_ref().and_then(|model| {
            model.ui.window.sidebar.swarms.iter().find_map(|swarm| {
                if id == format!("swarm:{}", swarm.id) {
                    let current = self
                        .focus
                        .stage
                        .as_ref()
                        .filter(|s| s.swarm == Some(swarm.id))
                        .and_then(|s| s.focused);
                    Some((
                        swarm.id,
                        current
                            .or_else(|| swarm.tabs.first().and_then(|t| t.seats.first().copied())),
                    ))
                } else {
                    swarm
                        .tabs
                        .iter()
                        .find(|tab| id == format!("tab:{}", tab.id))
                        .map(|tab| {
                            (
                                swarm.id,
                                self.focus
                                    .stage
                                    .as_ref()
                                    .filter(|s| s.tab == Some(tab.id))
                                    .and_then(|s| s.focused)
                                    .or_else(|| tab.seats.first().copied()),
                            )
                        })
                }
            })
        });
        if let Some((swarm, seat)) = target {
            if let Some(seat) = seat {
                self.focus_action(FocusAction::Activate(seat, FocusOrigin::Row));
            } else {
                self.select_empty_swarm(swarm);
            }
        }
    }
    pub(super) fn interaction_regions(&self) -> std::collections::BTreeMap<String, [f32; 4]> {
        let mut regions: std::collections::BTreeMap<_, _> = self
            .navigation_regions
            .iter()
            .map(|(id, r)| (id.clone(), [r.x, r.y, r.w, r.h]))
            .collect();
        if let Some(r) = self.sidebar_resize_region {
            regions.insert("sidebar-resize".into(), [r.x, r.y, r.w, r.h]);
        }
        if let Some(layout) = &self.conversation_layout {
            for (name, r) in [
                "sidebar-conversation-resize",
                "sidebar-composer-resize",
                "sidebar-tasks-resize",
            ]
            .into_iter()
            .zip(layout.splitters)
            {
                if r.w > 0. && r.h > 0. {
                    regions.insert(name.into(), [r.x, r.y, r.w, r.h]);
                }
            }
        }
        if let Some(settings) = self
            .local_page_layout
            .as_ref()
            .and_then(|page| page.settings.as_ref())
        {
            for (field, r) in settings.control_regions() {
                if let Some(key) = crate::settings_roles::control_key(&field) {
                    regions.insert(key, [r.x, r.y, r.w, r.h]);
                }
            }
        }
        for (seat, r) in &self.focus_bounds {
            regions.insert(format!("pane:{seat}"), [r.x, r.y, r.w, r.h]);
        }
        for (swarm, r, reset) in &self.swarm_folder_regions {
            let action = if *reset { "default" } else { "choose" };
            regions.insert(
                format!("swarm-folder/{swarm}/{action}"),
                [r.x, r.y, r.w, r.h],
            );
        }
        if let Some(layout) = &self.launcher_layout {
            let clip = |r: Rect| {
                let v = layout.viewport;
                let x = r.x.max(v.x);
                let y = r.y.max(v.y);
                let w = r.right().min(v.right()) - x;
                let h = r.bottom().min(v.bottom()) - y;
                (w > 0. && h > 0.).then_some([x, y, w, h])
            };
            let view = self.launcher.as_ref().unwrap().launcher.view();
            for (i, r) in layout.rows.iter().enumerate() {
                if let Some(rect) = clip(*r) {
                    regions.insert(format!("launcher:row:{i}"), rect);
                    regions.insert(format!("launcher:item:{}", view.rows[i].id), rect);
                }
            }
            if let Some(r) = clip(layout.query) {
                regions.insert("launcher:query".into(), r);
            }
            if let Some(r) = layout.rect("banner").and_then(clip) {
                regions.insert("launcher:preview".into(), r);
            }
            let r = layout.viewport;
            regions.insert("launcher:viewport".into(), [r.x, r.y, r.w, r.h]);
        }
        for (key, r) in self.app_overlays.regions() {
            regions.insert(key, [r.x, r.y, r.w, r.h]);
        }
        regions
    }
}
