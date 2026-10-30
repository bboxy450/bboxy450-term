// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One exact retained input snapshot; unrelated terminal frames reuse the solved tree.
use crate::*;
use bbt_ui::modules::{composer::Conversation, stream_dock::StreamDock};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Geometry {
    pub size: [f32; 2],
    pub width: f32,
    pub footer: bool,
    pub sidebar: composer::SidebarLayout,
    pub context: bbt_ui::Context,
    pub open: [bool; 2],
}
struct Inputs<'a> {
    geometry: Geometry,
    scope: Option<bbt_ui_model::SwarmId>,
    channel: &'a str,
    stream: &'a StreamDock,
    tasks: &'a StreamDock,
    composer: &'a composer::Composer,
    scroll: f32,
    task_scroll: f32,
}
struct Snapshot {
    geometry: Geometry,
    scope: Option<bbt_ui_model::SwarmId>,
    channel: String,
    view: Conversation,
}
impl Snapshot {
    fn new(input: Inputs<'_>) -> Self {
        Self {
            geometry: input.geometry,
            scope: input.scope,
            channel: input.channel.into(),
            view: Conversation {
                stream: input.stream.clone(),
                tasks: Some(input.tasks.clone()),
                composer: input.composer.clone(),
                scroll: input.scroll,
                task_scroll: input.task_scroll,
            },
        }
    }
    fn matches(&self, input: Inputs<'_>) -> bool {
        self.geometry == input.geometry
            && self.scope == input.scope
            && self.channel == input.channel
            && self.view.scroll == input.scroll
            && self.view.task_scroll == input.task_scroll
            && self.view.stream == *input.stream
            && self.view.tasks.as_ref() == Some(input.tasks)
            && self.view.composer == *input.composer
    }
}
#[derive(Default)]
pub(super) struct Cache {
    snapshot: Option<Snapshot>,
    #[cfg(test)]
    builds: usize,
}
impl App {
    fn conversation_inputs(&self, logical: [f32; 2]) -> Inputs<'_> {
        let sending = self.ui_wait.as_ref().is_some_and(|wait| {
            wait.effects
                .iter()
                .any(|effect| matches!(effect, UiEffect::Compose(_)))
        });
        Inputs {
            geometry: self.scene.conversation_geometry(
                logical,
                self.sidebar_width,
                self.effective_sidebar_layout(),
            ),
            scope: self.composer_scope,
            channel: self.message_history.selected(),
            stream: &self.conversation,
            tasks: &self.task_stream,
            composer: &self.composer.view,
            scroll: if sending {
                f32::MAX
            } else {
                self.chat_scroll.layout_offset()
            },
            task_scroll: self.task_scroll,
        }
    }
    pub(super) fn conversation_cache_matches(&self, logical: [f32; 2]) -> bool {
        self.conversation_cache
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.matches(self.conversation_inputs(logical)))
    }
    pub(super) fn solve_conversation(&mut self, logical: [f32; 2]) -> Result<(), String> {
        if self.live_state.is_none() && !self.compose_startup {
            self.conversation_layout = None;
            self.conversation_cache.snapshot = None;
            return Ok(());
        }
        // Compare borrowed DTOs before cloning. The snapshot includes hidden
        // action identities, attachment paths, caret and serde-skipped IME state.
        // Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        if self.conversation_layout.is_some() && self.conversation_cache_matches(logical) {
            return Ok(());
        }
        let snapshot = Snapshot::new(self.conversation_inputs(logical));
        let layout = self.scene.conversation_layout(logical, &snapshot.view)?;
        self.conversation_cache.snapshot = Some(snapshot);
        self.conversation_layout = Some(layout);
        #[cfg(test)]
        {
            self.conversation_cache.builds += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui::{
        components as c,
        modules::stream_dock::{Entry, Message, WorkItem},
    };

    fn fresh_matches(app: &App, size: [f32; 2]) {
        let snapshot = Snapshot::new(app.conversation_inputs(size));
        let fresh = app.scene.conversation_layout(size, &snapshot.view).unwrap();
        let cached = app.conversation_layout.as_ref().unwrap();
        let tree = |layout: &c::Layout| {
            serde_json::to_value(
                crate::ui_layout_node(layout, &mut 4096, &mut (16 * 1024)).unwrap(),
            )
            .unwrap()
        };
        assert_eq!(tree(&cached.layout), tree(&fresh.layout));
        assert_eq!(cached.splitters, fresh.splitters);
        assert_eq!(cached.regions.body, fresh.regions.body);
        let r = cached.layout.rect;
        for y in (0..r.h as usize).step_by(13) {
            for x in (0..r.w as usize).step_by(11) {
                let at = [r.x + x as f32, r.y + y as f32];
                assert_eq!(cached.actions.hit(at), fresh.actions.hit(at));
            }
        }
    }
    fn rebuild(app: &mut App, size: [f32; 2]) {
        let before = app.conversation_cache.builds;
        app.solve_ui(size).unwrap();
        assert_eq!(app.conversation_cache.builds, before + 1);
        fresh_matches(app, size);
    }
    fn row(scope: bbt_proto::SwarmId) -> bbt_proto::messaging::MessageRow {
        use bbt_proto::messaging::*;
        MessageRow {
            id: bbt_mux::Ulid(1),
            bus: Some(BusPosition {
                stream: format!("swarm-{scope}"),
                seq: 1,
            }),
            author: AuthorRef {
                kind: AuthorKind::Operator,
                id: "fixture".into(),
                name: "Fixture".into(),
            },
            issuer_kind: None,
            signature: SignatureState::Pending,
            signature_reason: Some("fixture".into()),
            upgrade_of: None,
            kind: MessageKind::Text,
            priority_class: bbt_proto::registry::Class::Oracle,
            priority: 0,
            text: "Identical chat survives unrelated terminal publications.".into(),
            ts: 1000,
            receipts: Default::default(),
        }
    }
    #[test]
    fn unrelated_terminal_publications_and_refresh_reuse_the_actual_conversation_tree() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "conversation_cache::tests::unrelated_terminal_publications_and_refresh_reuse_the_actual_conversation_tree",
        ) {
            return;
        }
        // Resolve optional timezone metadata before registering the fixture UI.
        // A later "—" -> local clock transition is a real input change, not a cache hit.
        bboxy450_term::local_time::initialize();
        let (mut app, latest) = crate::lazy_projection_tests::fixture();
        let model = crate::lazy_projection_tests::published_model(0);
        latest.publish(model.clone());
        app.drain(); // Settle catalog/workspace state using a real retained swarm and seats.
        let scope = bbt_proto::SwarmId::from_u128(2);
        assert_eq!(app.composer_scope, Some(scope));
        assert_eq!(app.message_history.selected(), "all");
        app.message_history.cursor(0, 1, 0);
        app.message_history
            .row(&row(scope), Some("operator/inbox"), 1000);
        app.refresh_conversation();
        app.project_scene(true);
        app.solve_ui([1200., 900.]).unwrap();
        let builds = app.conversation_cache.builds;
        let children = app
            .conversation_layout
            .as_ref()
            .unwrap()
            .layout
            .children
            .as_ptr();
        for cell_width in [7, 8, 9] {
            // This is a real Latest publication consumed by App::drain, whose
            // normal path calls refresh_conversation before the following solve.
            let mut publication = model.as_ref().clone();
            publication.terminal_cell_pixels = Some([cell_width, 16]);
            latest.publish(Arc::new(publication));
            app.drain();
            assert_eq!(app.composer_scope, Some(scope));
            let input = app.conversation_inputs(app.solved_size);
            let previous = app.conversation_cache.snapshot.as_ref().unwrap();
            assert!(
                app.conversation_layout.is_some(),
                "identical refresh must retain hit geometry: scope={:?}/{:?}, channel={}/{}, geometry={:?}/{:?}, stream_equal={}, tasks_equal={}, composer_equal={}",
                previous.scope,
                input.scope,
                previous.channel,
                input.channel,
                previous.geometry,
                input.geometry,
                previous.view.stream == *input.stream,
                previous.view.tasks.as_ref() == Some(input.tasks),
                previous.view.composer == *input.composer,
            );
            app.project_scene(true);
            app.solve_ui([1200., 900.]).unwrap();
            assert_eq!(app.conversation_cache.builds, builds);
            assert_eq!(
                app.conversation_layout
                    .as_ref()
                    .unwrap()
                    .layout
                    .children
                    .as_ptr(),
                children
            );
            fresh_matches(&app, [1200., 900.]);
        }
        let mut changed = row(scope);
        changed.text = "A real changed message invalidates hit geometry before solving.".into();
        app.message_history
            .row(&changed, Some("operator/inbox"), 2000);
        app.refresh_conversation();
        assert!(app.conversation_layout.is_none());
        rebuild(&mut app, [1200., 900.]);
    }
    #[test]
    fn exact_dto_geometry_scroll_and_scope_changes_match_a_fresh_solver() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "conversation_cache::tests::exact_dto_geometry_scroll_and_scope_changes_match_a_fresh_solver",
        ) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.compose_startup = true;
        app.backend_lost = true;
        app.composer_scope = Some(bbt_ui_model::SwarmId::from_u128(1));
        app.message_history.select(bbt_proto::SwarmId::from_u128(1));
        app.chat_scroll
            .bind(bbt_ui_model::SwarmId::from_u128(1), "all")
            .unwrap();
        app.conversation.entries = vec![Entry::Message {
            data: Box::new(Message {
                message_ids: vec!["message-a".into()],
                text: "A long retained message body.\n".repeat(60),
                attach: vec![composer::Attachment {
                    id: "attachment-a".into(),
                    path: ".bboxy/tasks/a.md".into(),
                    title: "Attachment".into(),
                    kind: "task".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            show_channel: None,
        }];
        app.task_stream.entries = vec![Entry::Task {
            data: WorkItem {
                id: "task-a".into(),
                title: "Task".into(),
                state: "open".into(),
                ..Default::default()
            },
            delegate: true,
        }];
        app.composer.view.text = "draft".into();
        app.composer.view.focus = true;
        app.composer.view.edit = Some(c::Edit {
            caret: Some(5),
            ..Default::default()
        });
        let mut size = [1200., 900.];
        rebuild(&mut app, size);
        if let Entry::Message { data, .. } = &mut app.conversation.entries[0] {
            data.text.push_str("changed");
        }
        rebuild(&mut app, size);
        if let Entry::Message { data, .. } = &mut app.conversation.entries[0] {
            data.message_ids.push("new-hidden-message-id".into());
        }
        rebuild(&mut app, size);
        if let Entry::Task { data, .. } = &mut app.task_stream.entries[0] {
            data.id = "task-b".into();
        }
        rebuild(&mut app, size); // Hidden action identity changes with identical label/geometry.
        if let Entry::Message { data, .. } = &mut app.conversation.entries[0] {
            data.attach[0].path = ".bboxy/tasks/b.md".into();
        }
        rebuild(&mut app, size);
        app.composer.view.text.push('!');
        rebuild(&mut app, size);
        app.composer.view.edit.as_mut().unwrap().caret = Some(2);
        rebuild(&mut app, size);
        app.composer.view.edit.as_mut().unwrap().preedit = Some(c::Preedit {
            text: "marked".into(),
            caret: Some(2),
        });
        rebuild(&mut app, size);
        app.composer.view.edit.as_mut().unwrap().replacement = Some([0, 2]);
        rebuild(&mut app, size); // serde-skipped IME state is part of exact equality.
        app.composer.view.edit.as_mut().unwrap().hide_caret = true;
        rebuild(&mut app, size);
        app.scroll_sidebar_section(false, -12.).unwrap();
        rebuild(&mut app, size);
        app.task_scroll = 1.;
        rebuild(&mut app, size);
        size = [1250., 950.];
        rebuild(&mut app, size);
        app.sidebar_width = Some(320.);
        rebuild(&mut app, size);
        app.sidebar_layout_preview = Some(composer::SidebarLayout {
            tasks_height: Some(180.),
            composer_body_height: Some(80.),
            ..Default::default()
        });
        rebuild(&mut app, size);
        let mut context = app.scene.ui_context();
        context.theme = bbt_ui::Theme::Light;
        app.scene.set_context(context);
        rebuild(&mut app, size);
        context.density = bbt_ui::Density::Compact;
        app.scene.set_context(context);
        rebuild(&mut app, size);
        app.composer_scope = Some(bbt_ui_model::SwarmId::from_u128(2));
        rebuild(&mut app, size);
        app.message_history.channel("channels/other");
        rebuild(&mut app, size);
        let builds = app.conversation_cache.builds;
        app.solve_ui(size).unwrap();
        assert_eq!(app.conversation_cache.builds, builds);
        app.conversation_layout = None; // Explicit invalidations still force a solve.
        rebuild(&mut app, size);
        let mut input = app.conversation_inputs(size);
        input.geometry.footer = !input.geometry.footer;
        assert!(
            !app.conversation_cache
                .snapshot
                .as_ref()
                .unwrap()
                .matches(input)
        );
        let mut input = app.conversation_inputs(size);
        input.geometry.context.focused = !input.geometry.context.focused;
        assert!(
            !app.conversation_cache
                .snapshot
                .as_ref()
                .unwrap()
                .matches(input)
        );
    }
}
