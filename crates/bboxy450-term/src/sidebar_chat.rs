// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Independent sidebar history and task viewports; bounded transient presence.
use crate::*;
use bbt_ui::{
    components as c,
    modules::stream_dock::{Entry, Message},
};

pub(super) struct NoticeTimer {
    until: Instant,
    id: bbt_mux::Ulid,
    task: tokio::task::AbortHandle,
}
impl Drop for NoticeTimer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) struct TaskPanelTimer {
    epoch: u64,
    task: tokio::task::AbortHandle,
}
impl Drop for TaskPanelTimer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl TaskPanelTimer {
    fn retry(handle: &tokio::runtime::Handle, epoch: u64, delay: Duration, proxy: UiProxy) -> Self {
        let task = handle.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = proxy.send_event(UiEvent::TaskRetry(epoch));
        });
        Self {
            epoch,
            task: task.abort_handle(),
        }
    }
}

impl App {
    pub(super) fn sync_task_retry(&mut self) {
        if self
            .task_retry_timer
            .as_ref()
            .is_some_and(|timer| !self.task_records.retry_pending(timer.epoch))
        {
            self.task_retry_timer = None;
        }
        if let Some((epoch, delay)) = self.task_records.take_retry() {
            self.task_retry_timer = Some(TaskPanelTimer::retry(
                bbt_rt::handle(),
                epoch,
                delay,
                self.proxy.clone(),
            ));
        }
    }
    pub(super) fn task_panel_animating(&self) -> bool {
        fn visible(node: &c::Layout) -> bool {
            if matches!(&node.kind, c::LayoutKind::Named(key) if key == "conversation/tasks-progress")
            {
                return node.rect.w > 0. && node.rect.h > 0.;
            }
            node.children.iter().any(visible)
        }
        matches!(self.task_stream.entries.first(), Some(Entry::TaskState { data }) if data.loading)
            && self
                .conversation_layout
                .as_ref()
                .is_some_and(|layout| visible(&layout.layout))
    }
    pub(super) fn sync_task_panel(&mut self) {
        self.sync_task_retry();
        let Some(data) = self.task_records.panel_state(Instant::now()) else {
            self.task_panel_timer = None;
            return;
        };
        self.task_stream.entries = vec![Entry::TaskState { data }];
        let epoch = self.task_records.retry_epoch();
        if self
            .task_panel_timer
            .as_ref()
            .is_some_and(|timer| timer.epoch == epoch)
        {
            return;
        }
        self.task_panel_timer = None;
        let proxy = self.proxy.clone();
        let task = bbt_rt::handle().spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(300)).await;
                if proxy.send_event(UiEvent::TaskPanelTick(epoch)).is_err() {
                    break;
                }
            }
        });
        self.task_panel_timer = Some(TaskPanelTimer {
            epoch,
            task: task.abort_handle(),
        });
    }
    pub(super) fn task_panel_tick(&mut self, epoch: u64) {
        if epoch != self.task_records.retry_epoch() || self.task_panel_timer.is_none() {
            return;
        }
        let previous = self.task_stream.entries.clone();
        self.sync_task_panel();
        if previous != self.task_stream.entries {
            self.conversation_layout = None;
            self.request_redraw();
        }
    }
}

/// A notice joins the chronological chat rows. Both UI timestamps are epoch milliseconds.
fn insert_notice(entries: &mut Vec<Entry>, notice: &bboxy450_term::close::CloseNotice) {
    let at = entries
        .iter()
        .position(|entry| {
            matches!(entry,
        Entry::Message { data, .. } if data.ts.is_some_and(|ts_ms| ts_ms > notice.ts_ms))
        })
        .unwrap_or_else(|| {
            entries
                .iter()
                .rposition(|entry| matches!(entry, Entry::Message { .. }))
                .map_or_else(
                    || {
                        entries
                            .iter()
                            .take_while(|entry| {
                                matches!(entry, Entry::Tabs { .. } | Entry::Filter { .. })
                            })
                            .count()
                    },
                    |at| at + 1,
                )
        });
    entries.insert(
        at,
        Entry::Message {
            data: Box::new(Message {
                message_ids: vec![format!("close:{}", notice.operation)],
                text: notice.text.clone(),
                time: notice.time.clone(),
                ts: Some(notice.ts_ms),
                presence: true,
                kind: Some("presence".into()),
                ..Default::default()
            }),
            show_channel: None,
        },
    );
}

pub(super) fn scroll_position(layout: &c::Layout, key: &str) -> Option<(f32, f32)> {
    fn find<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind, c::LayoutKind::Named(name) if name == key) {
            Some(node)
        } else {
            node.children.iter().find_map(|child| find(child, key))
        }
    }
    fn scroll(node: &c::Layout) -> Option<(f32, f32)> {
        if let c::LayoutKind::ScrollView {
            offset, content, ..
        } = &node.kind
        {
            Some((offset[1], (content[1] - node.rect.h).max(0.)))
        } else {
            node.children.iter().find_map(scroll)
        }
    }
    scroll(find(layout, key)?)
}

fn task_page_needed(dy: f32, offset: f32, max: f32) -> bool {
    dy > 0. && max - offset <= 28.
}

impl App {
    pub(super) fn scroll_sidebar_section(&mut self, tasks: bool, dy: f32) -> Result<(), String> {
        if !dy.is_finite() || dy.abs() > bbt_proto::gui::MAX_SECTION_WHEEL_DY {
            return Err("invalid_ui_section_wheel".into());
        }
        let key = if tasks {
            "conversation/tasks"
        } else {
            "conversation/rows"
        };
        let (offset, max) = self
            .conversation_layout
            .as_ref()
            .and_then(|layout| scroll_position(&layout.layout, key))
            .ok_or("sidebar_section_unavailable")?;
        let changed = if tasks {
            let Some(motion) = self.task_wheel.wheel(offset, max, dy) else {
                return Ok(());
            };
            if motion.changed {
                self.task_scroll = motion.offset;
            }
            if task_page_needed(dy, motion.offset, max) {
                self.task_records.more_tasks();
                self.task_reads();
            }
            motion.changed
        } else {
            if dy == 0. {
                return Ok(());
            }
            let changed = self.chat_scroll.wheel(offset, max, dy);
            if dy < 0. && self.chat_scroll.requested_offset().clamp(0., max) <= 28. {
                self.message_history.more();
            }
            self.conversation_reads(true);
            changed
        };
        if changed {
            self.request_redraw();
        }
        Ok(())
    }

    pub(super) fn sidebar_section_positions(&self) -> Option<bbt_proto::gui::UiSidebarSections> {
        let layout = self.conversation_layout.as_ref()?;
        let (offset, max) =
            scroll_position(&layout.layout, "conversation/rows").unwrap_or((0., 0.));
        let chat = bbt_proto::gui::UiSectionScroll {
            offset,
            max,
            following: self.chat_scroll.following(),
        };
        let (offset, max) =
            scroll_position(&layout.layout, "conversation/tasks").unwrap_or((0., 0.));
        Some(bbt_proto::gui::UiSidebarSections {
            chat,
            tasks: bbt_proto::gui::UiSectionScroll {
                offset,
                max,
                following: false,
            },
        })
    }

    pub(super) fn append_close_notices(&mut self) {
        let Some(model) = &self.live_state else {
            return;
        };
        let Some(swarm) = self.composer_scope else {
            return;
        };
        for notice in model.closes.visible_notices(swarm, Instant::now()) {
            insert_notice(&mut self.conversation.entries, notice);
        }
    }

    pub(super) fn schedule_chat_notice_expiry(&mut self) {
        let until = self
            .live_state
            .as_ref()
            .and_then(|model| model.closes.next_notice_expiry(Instant::now()));
        if self.chat_notice_timer.as_ref().map(|timer| timer.until) == until {
            return;
        }
        self.chat_notice_timer = None;
        let Some(until) = until else { return };
        let proxy = self.proxy.clone();
        let id = bbt_mux::Ulid::new();
        let task = bbt_rt::handle().spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(until)).await;
            let _ = proxy.send_event(UiEvent::ChatNoticeExpiry(id));
        });
        self.chat_notice_timer = Some(NoticeTimer {
            until,
            id,
            task: task.abort_handle(),
        });
    }

    pub(super) fn chat_notice_expired(&mut self, id: bbt_mux::Ulid) {
        if self
            .chat_notice_timer
            .as_ref()
            .is_none_or(|timer| timer.id != id)
        {
            return;
        }
        self.chat_notice_timer = None;
        self.refresh_conversation();
        self.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(clippy::disallowed_methods, reason = "private paused runtime witness, never a UI turn")]
    #[tokio::test(start_paused = true)]
    async fn retired_task_retry_timer_cannot_wake_the_ui() {
        let (send, recv) = std::sync::mpsc::channel();
        let proxy = UiProxy::Headless(Arc::new(move |event| {
            if let UiEvent::TaskRetry(epoch) = event {
                send.send(epoch).unwrap();
            }
        }));
        let handle = tokio::runtime::Handle::current();
        let old = TaskPanelTimer::retry(&handle, 1, Duration::from_secs(5), proxy.clone());
        tokio::task::yield_now().await;
        drop(old);
        tokio::time::advance(Duration::from_secs(6)).await;
        tokio::task::yield_now().await;
        assert!(
            recv.try_recv().is_err(),
            "retired timer must not send a callback"
        );
        let live = TaskPanelTimer::retry(&handle, 2, Duration::from_secs(5), proxy);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(6)).await;
        tokio::task::yield_now().await;
        assert_eq!(recv.try_recv().unwrap(), 2);
        assert!(recv.try_recv().is_err());
        drop(live);
    }
    #[test]
    fn shared_wheel_handlers_keep_bursts_noops_reflow_and_task_page_boundaries() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "sidebar_chat::tests::shared_wheel_handlers_keep_bursts_noops_reflow_and_task_page_boundaries",
        ) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.compose_startup = true;
        // This in-memory owner has no backend; retain generated read intents for assertions.
        app.backend_lost = true;
        app.sidebar_layout_preview = Some(composer::SidebarLayout {
            conversation_fraction: 0.7,
            tasks_height: Some(150.),
            ..Default::default()
        });
        app.chat_scroll
            .bind(bbt_ui_model::SwarmId::from_u128(1), "all")
            .unwrap();
        app.conversation.entries = (0..64)
            .map(|number| Entry::Note {
                text: format!("chat {number}"),
                padding: [0.; 4],
            })
            .collect();
        app.task_stream.entries = (0..64)
            .map(|number| Entry::Note {
                text: format!("task {number}"),
                padding: [0.; 4],
            })
            .collect();
        app.task_records.select(bbt_proto::SwarmId::from_u128(1));
        let _ = app.task_records.commands(); // Admit initial reads before measuring pagination.
        let next = format!("99:{}", bbt_mux::Ulid(99));
        app.task_records.task_next = Some(next.clone());
        app.solve_ui([1200., 1000.]).unwrap();
        let (_, chat_max) = scroll_position(
            &app.conversation_layout.as_ref().unwrap().layout,
            "conversation/rows",
        )
        .unwrap();
        let (_, tasks_max) = scroll_position(
            &app.conversation_layout.as_ref().unwrap().layout,
            "conversation/tasks",
        )
        .unwrap();
        assert!(chat_max > 100. && tasks_max > 100.);
        app.redraw_pending.set(false);
        app.scroll_sidebar_section(false, 10.).unwrap();
        assert!(
            !app.redraw_pending.get(),
            "already following tail: no redundant redraw"
        );
        app.scroll_sidebar_section(false, -10.).unwrap();
        app.scroll_sidebar_section(false, -10.).unwrap();
        assert_eq!(app.chat_scroll.requested_offset(), chat_max - 20.);
        app.scroll_sidebar_section(true, 1.25).unwrap();
        app.scroll_sidebar_section(true, 1.25).unwrap();
        assert_eq!(
            app.task_scroll, 2.5,
            "both native/ctl-shared events precede solve"
        );
        assert!(
            app.task_records.commands().is_empty(),
            "scrolling near top must not request next task page"
        );
        app.scene.sidebar_scroll = 100.;
        app.sidebar_viewport = Some((Rect::new(0., 0., 100., 100.), 500.));
        app.scroll_sidebar(1.25);
        app.scroll_sidebar(1.25);
        assert_eq!(app.sidebar_scroll, Some(102.5));
        app.scroll_sidebar(-32768.);
        app.redraw_pending.set(false);
        app.scroll_sidebar(-1.);
        assert!(
            !app.redraw_pending.get(),
            "clamped navigation wheel needs no redraw"
        );
        app.scroll_sidebar(102.5);
        app.solve_ui([1200., 1800.]).unwrap(); // Explicitly retires all three pending bursts.
        app.scene.sidebar_scroll = 20.;
        app.sidebar_viewport = Some((Rect::new(0., 0., 100., 100.), 50.));
        app.scroll_sidebar(1.);
        assert_eq!(
            app.sidebar_scroll,
            Some(21.),
            "new solved geometry retires navigation's stale target"
        );
        let (actual, _) = scroll_position(
            &app.conversation_layout.as_ref().unwrap().layout,
            "conversation/rows",
        )
        .unwrap();
        app.scroll_sidebar_section(false, -5.).unwrap();
        assert_eq!(app.chat_scroll.requested_offset(), (actual - 5.).max(0.));
        let (actual, _) = scroll_position(
            &app.conversation_layout.as_ref().unwrap().layout,
            "conversation/tasks",
        )
        .unwrap();
        app.scroll_sidebar_section(true, -1.).unwrap();
        assert_eq!(app.task_scroll, (actual - 1.).max(0.));
        app.scroll_sidebar_section(true, -32768.).unwrap();
        app.redraw_pending.set(false);
        app.scroll_sidebar_section(true, -1.).unwrap();
        assert!(
            !app.redraw_pending.get(),
            "clamped top wheel needs no redraw"
        );
        app.scroll_sidebar_section(true, 32768.).unwrap();
        assert!(app.task_records.commands().iter().any(|command| matches!(command,
            live::UiMsg::Task {request:bbt_proto::messaging::TaskRequest::List {after:Some(cursor),..},..} if *cursor == next
        )), "near-bottom motion must request the retained next page");
        app.redraw_pending.set(false);
        app.scroll_sidebar_section(true, 1.).unwrap();
        assert!(
            !app.redraw_pending.get(),
            "clamped bottom wheel still pages without redundant redraw"
        );
    }
    #[test]
    fn tasks_pagination_threshold_counts_only_downward_boundary_gestures() {
        let mut intent = bboxy450_term::chat_scroll::WheelIntent::default();
        let mut reads = 0;
        for dy in [1., 1., 1., 1.] {
            let motion = intent.wheel(0., 100., dy).unwrap();
            reads += usize::from(task_page_needed(dy, motion.offset, 100.));
        }
        assert_eq!(reads, 0);
        let motion = intent.wheel(0., 100., 68.).unwrap();
        assert!(task_page_needed(68., motion.offset, 100.));
        assert!(!task_page_needed(-1., 72., 100.));
        assert!(!task_page_needed(0., 100., 100.));
    }
    #[test]
    fn close_notice_joins_chat_chronology_between_real_messages() {
        let mut entries = vec![Entry::Filter { name: "all".into() }];
        let first_seconds = 1_793_211_000_u64;
        for seconds in [first_seconds, first_seconds + 3600] {
            // Use the real row adapter: fabricated millisecond UI rows hid this bug.
            let row = serde_json::from_value(serde_json::json!({
                "id": bbt_mux::Ulid::new(), "bus": null,
                "author": {"kind": "observer", "id": "fixture", "name": "Fixture"},
                "kind": "text", "priority_class": "worker", "priority": 0,
                "text": seconds.to_string(), "ts": seconds,
                "receipts": {"delivered": 0, "acked": 0, "held": 0}
            }))
            .unwrap();
            entries.push(Entry::Message {
                data: Box::new(bboxy450_term::messaging::message_view(
                    &row,
                    &Default::default(),
                )),
                show_channel: None,
            });
        }
        let notice = bboxy450_term::close::CloseNotice {
            operation: bbt_mux::Ulid(1),
            seat: SeatId::from_u128(2),
            generation: 1,
            swarm: bbt_ui_model::SwarmId::from_u128(3),
            text: "fixture closed".into(),
            time: "00:00".into(),
            ts_ms: (first_seconds + 1800) * 1000,
            until: Instant::now(),
        };
        insert_notice(&mut entries, &notice);
        let times: Vec<_> = entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Message { data, .. } => data.ts,
                _ => None,
            })
            .collect();
        assert_eq!(
            times,
            [first_seconds, first_seconds + 1800, first_seconds + 3600]
                .map(|seconds| seconds * 1000)
        );
        assert!(matches!(&entries[0], Entry::Filter { .. }));
    }
}
