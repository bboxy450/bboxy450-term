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

/// A notice joins the chronological chat rows, never the task list or a sticky footer.
fn insert_notice(entries: &mut Vec<Entry>, notice: &bboxy450_term::close::CloseNotice) {
    let at = entries
        .iter()
        .position(|entry| {
            matches!(entry,
        Entry::Message { data, .. } if data.ts.is_some_and(|ts| ts > notice.ts_ms))
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
        if tasks {
            self.task_scroll = (offset + dy).clamp(0., max);
            if dy > 0. {
                self.task_records.more_tasks();
                self.task_reads();
            }
        } else {
            self.chat_scroll.wheel(offset, max, dy);
            if dy < 0. && (offset + dy).max(0.) <= 28. {
                self.message_history.more();
            }
            self.conversation_reads(true);
        }
        self.request_redraw();
        Ok(())
    }

    pub(super) fn sidebar_section_positions(&self) -> Option<bbt_proto::gui::UiSidebarSections> {
        let layout = self.conversation_layout.as_ref()?;
        let (offset, max) = scroll_position(&layout.layout, "conversation/rows")?;
        let chat = bbt_proto::gui::UiSectionScroll {
            offset,
            max,
            following: self.chat_scroll.following(),
        };
        let (offset, max) = scroll_position(&layout.layout, "conversation/tasks")?;
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
    #[test]
    fn close_notice_joins_chat_chronology_between_real_messages() {
        let mut entries = vec![Entry::Filter { name: "all".into() }];
        for ts in [10, 30] {
            entries.push(Entry::Message {
                data: Box::new(Message {
                    ts: Some(ts),
                    text: ts.to_string(),
                    ..Default::default()
                }),
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
            ts_ms: 20,
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
        assert_eq!(times, vec![10, 20, 30]);
        assert!(matches!(&entries[0], Entry::Filter { .. }));
    }
}
