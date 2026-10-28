// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded view state. Shared server projections, ledger cursors and bus
//! positions remain separate; this cache never acknowledges or proves read.
use super::{
    MessageAttachments, Reply, Results, SendGroup, UiMsg, message_entries_with_attachments, wire,
};
use bbt_mux::Ulid;
use bbt_proto::SwarmId;
use bbt_ui::modules::stream_dock::{Channel, Entry, UnreadCount};
use std::collections::{BTreeMap, VecDeque};

pub enum HistoryAction {
    Cursor { stream: String },
    Page { stream: String, after: u64 },
    Read { stream: String, sequence: u64 },
}

impl From<HistoryAction> for UiMsg {
    fn from(action: HistoryAction) -> Self {
        match action {
            HistoryAction::Cursor { stream } => Self::EventCursor { stream },
            HistoryAction::Page { stream, after } => Self::EventPage {
                stream,
                after,
                limit: 128,
            },
            HistoryAction::Read { stream, sequence } => Self::MarkEventsRead { stream, sequence },
        }
    }
}
struct Row {
    row: wire::MessageRow,
    context: String,
    size: usize,
    projected_at: u64,
}
struct List {
    id: Ulid,
    after: u64,
}
#[derive(Default)]
pub struct History {
    scope: Option<SwarmId>,
    rows: VecDeque<Row>,
    bytes: usize,
    listed: Vec<(Ulid, Option<wire::BusPosition>, String, MessageAttachments)>,
    list: Option<List>,
    list_wanted: Option<u64>,
    list_next: Option<u64>,
    list_failed_after: Option<u64>,
    cursor: Option<(u64, u64, u64)>, // confirmed read, high, unread
    cursor_wanted: bool,
    page_wanted: Option<u64>,
    page_pending: Option<(u64, u64)>, // requested after, previous completion revision
    page_revision: u64,
    page_next: Option<u64>,
    failed_after: Option<u64>,
    page_issue: Option<String>,
    covered_high: Option<u64>,
    chain_next: Option<u64>,
    gap_after: Option<u64>,
    seen_activity: BTreeMap<String, u64>,
    artifacts: VecDeque<bbt_ui::modules::composer::Attachment>,
    lane_issue: Option<String>,
    read_sent: u64,
    selected: String,
    pub issue: Option<String>,
    pub omitted: u64,
}
impl History {
    /// Consume the IO owner's one published lane. Neither transport ACKs nor
    /// a model publication imply that the operator has read a message.
    pub fn observe(&mut self, lane: &crate::model::EventLane) -> Vec<Option<bbt_mux::Ulid>> {
        use bbt_proto::events::{GuiEventPayload, GuiEventRecord};
        let Some(stream) = self.stream() else {
            return vec![];
        };
        self.lane_issue = match lane.state() {
            crate::model::EventLaneState::Held(reason) => Some(format!("events held · {reason}")),
            crate::model::EventLaneState::Detached | crate::model::EventLaneState::Attaching => {
                Some("events attaching".into())
            }
            crate::model::EventLaneState::Active => None,
        };
        if let Some(cursor) = lane.cursors().get(&stream) {
            self.cursor(cursor.read, cursor.high, cursor.unread);
        }
        let mut tasks = vec![];
        for record in lane.records().iter().chain(lane.read_page()) {
            match record.as_ref() {
                GuiEventRecord::Event(event) => match &event.payload {
                    GuiEventPayload::Message { row } => {
                        // A receipt's retained position is not the message's
                        // publication position and cannot supply its context.
                        let position = &event.metadata.reference.position;
                        let context = row
                            .bus
                            .as_ref()
                            .filter(|bus| {
                                bus.stream == position.stream && bus.seq == position.sequence
                            })
                            .and_then(|_| event.metadata.subject.strip_suffix("/message"));
                        self.row(row, context, event.projected_at);
                    }
                    GuiEventPayload::Activity { summary } => {
                        let position = &event.metadata.reference.position;
                        let seen = self
                            .seen_activity
                            .entry(position.stream.clone())
                            .or_default();
                        if position.sequence <= *seen {
                            continue;
                        }
                        *seen = position.sequence;
                        if event.metadata.subject.starts_with("task/")
                            && event.metadata.subject.ends_with("/status")
                        {
                            tasks.push(summary.id);
                        }
                        if event.metadata.subject.starts_with("artifact/")
                            && !summary.path_redacted
                            && let Some(path) = &summary.path
                        {
                            self.artifacts.retain(|attachment| attachment.path != *path);
                            if summary.action != "deleted" {
                                self.artifacts
                                    .push_back(bbt_ui::modules::composer::Attachment {
                                        id: summary
                                            .id
                                            .map_or_else(|| path.clone(), |id| id.to_string()),
                                        path: path.clone(),
                                        title: path.clone(),
                                        kind: "artifact".into(),
                                        ..Default::default()
                                    });
                            }
                            while self.artifacts.len() > 128
                                || serde_json::to_vec(&self.artifacts)
                                    .map_or(true, |bytes| bytes.len() > 128 * 1024)
                            {
                                if self.artifacts.pop_front().is_none() {
                                    break;
                                }
                            }
                        }
                    }
                    _ => (),
                },
                GuiEventRecord::Gap(gap) if gap.stream == stream => self.gap(gap.first, gap.last),
                _ => (),
            }
        }
        if let Some(gap) = lane.omitted().get(&stream) {
            self.gap(gap.first, gap.last);
        }
        if let Some(gap) = lane.read_gap().filter(|gap| gap.stream == stream) {
            self.gap(gap.first, gap.last);
        }
        let valid = lane.page_snapshots().get(&stream);
        let error = lane.page_errors().get(&stream);
        if let Some(error) =
            error.filter(|error| valid.is_none_or(|page| error.revision > page.revision))
        {
            self.page_failed(error.revision, &error.reason);
        } else if let Some(page) = valid {
            self.page(page.revision, page.high, page.next);
        }
        tasks
    }
    pub fn artifacts(&self) -> impl Iterator<Item = &bbt_ui::modules::composer::Attachment> {
        self.artifacts.iter()
    }
    pub fn select(&mut self, scope: SwarmId) {
        if self.scope == Some(scope) {
            return;
        }
        *self = Self {
            scope: Some(scope),
            selected: "all".into(),
            list_wanted: Some(0),
            cursor_wanted: true,
            page_wanted: Some(0),
            ..Default::default()
        };
    }
    pub fn stream(&self) -> Option<String> {
        self.scope.map(|id| format!("swarm-{id}"))
    }
    pub fn selected(&self) -> &str {
        &self.selected
    }
    pub fn channel(&mut self, path: &str) {
        if path == "all" {
            self.read_sent = 0;
        }
        if self.page_pending.is_none() && self.failed_after.is_some() {
            self.page_wanted = self.failed_after.take();
        }
        if self.list.is_none() && self.list_failed_after.is_some() {
            self.list_wanted = self.list_failed_after.take();
        }
        if self.selected != path {
            self.selected = path.into();
            self.list = None;
            self.listed.clear();
            self.list_next = None;
            self.list_failed_after = None;
            self.list_wanted = Some(0);
        }
    }
    pub fn reconnect(&mut self) {
        self.page_pending = None;
        self.page_wanted = Some(self.page_next.unwrap_or(0));
        self.list = None;
        self.list_wanted = Some(0);
        self.cursor_wanted = true;
        self.read_sent = 0;
    }
    pub fn cursor(&mut self, read: u64, high: u64, unread: u64) {
        if self
            .cursor
            .is_some_and(|(old_read, old_high, _)| read < old_read || high < old_high)
        {
            return;
        }
        self.cursor = Some((read, high, unread));
        self.cursor_wanted = false;
    }
    pub fn row(&mut self, row: &wire::MessageRow, context: Option<&str>, projected_at: u64) {
        let Some(stream) = self.stream() else {
            return;
        };
        if row.bus.as_ref().is_none_or(|bus| bus.stream != stream) {
            return;
        }
        if self
            .rows
            .iter()
            .any(|old| {
                old.row.id == row.id
                    && (old.projected_at > projected_at
                        // A display clock cannot revoke a confirmed signature.
                        || (old.row.signature == wire::SignatureState::Signed
                            && row.signature == wire::SignatureState::Pending))
            })
        {
            return;
        }
        if let Some(old) = self
            .rows
            .iter_mut()
            .find(|old| old.row == *row && context.is_none_or(|context| context == old.context))
        {
            old.projected_at = old.projected_at.max(projected_at);
            return;
        }
        let previous = self
            .rows
            .iter()
            .position(|old| old.row.id == row.id)
            .and_then(|i| self.rows.remove(i));
        if let Some(previous) = &previous {
            self.bytes -= previous.size;
        }
        let context = context
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| previous.map(|p| p.context))
            .unwrap_or_default();
        let size = serde_json::to_vec(&(row, &context)).map_or(1024 * 1024 + 1, |b| b.len() + 33);
        self.bytes += size;
        self.rows.push_back(Row {
            row: row.clone(),
            context,
            size,
            projected_at,
        });
        self.trim();
    }
    fn trim(&mut self) {
        while self.rows.len() > 128 || self.bytes + 2 > 1024 * 1024 {
            let Some(old) = self.rows.pop_front() else {
                break;
            };
            self.bytes -= old.size;
            self.omitted += 1;
        }
    }
    pub fn gap(&mut self, first: u64, last: u64) {
        if first <= last {
            self.gap_after = Some(self.gap_after.map_or(last, |old| old.max(last)));
        }
    }
    /// Page-specific high is captured before a newer live cursor can replace it.
    pub fn page(&mut self, revision: u64, high: u64, next: Option<u64>) {
        self.page_revision = self.page_revision.max(revision);
        let Some((after, old)) = self.page_pending else {
            return;
        };
        if revision <= old {
            return;
        }
        self.page_pending = None;
        self.page_issue = None;
        self.failed_after = None;
        if after == 0 || self.chain_next == Some(after) || self.covered_high == Some(after) {
            self.chain_next = next;
            self.covered_high = if next.is_none() { Some(high) } else { None };
        } else {
            self.covered_high = None;
            self.chain_next = next;
        }
        self.page_next = next;
    }
    pub fn page_failed(&mut self, revision: u64, reason: &str) {
        self.page_revision = self.page_revision.max(revision);
        if let Some((after, previous)) = self.page_pending
            && revision > previous
        {
            self.page_pending = None;
            self.failed_after = Some(after);
            self.page_issue = Some(reason.chars().take(256).collect());
        }
    }
    pub fn commands(&mut self) -> Vec<UiMsg> {
        let Some(scope) = self.scope else {
            return vec![];
        };
        if self.list.is_some() {
            return vec![];
        }
        let Some(after) = self.list_wanted.take() else {
            return vec![];
        };
        let id = Ulid::new();
        self.list = Some(List { id, after });
        vec![UiMsg::Messaging {
            id,
            swarm_id: scope,
            request: wire::Request::List {
                thread: None,
                channel: (self.selected != "all").then(|| self.selected.clone()),
                after,
                limit: 128,
            },
        }]
    }
    pub fn actions(&mut self) -> Vec<HistoryAction> {
        let Some(stream) = self.stream() else {
            return vec![];
        };
        let mut out = vec![];
        if self.cursor_wanted {
            self.cursor_wanted = false;
            out.push(HistoryAction::Cursor {
                stream: stream.clone(),
            });
        }
        if self.page_pending.is_none()
            && let Some(after) = self.page_wanted.take()
        {
            self.page_pending = Some((after, self.page_revision));
            out.push(HistoryAction::Page { stream, after });
        }
        out
    }
    pub fn more(&mut self) {
        if self.list.is_none() {
            self.list_wanted = self.list_failed_after.take().or(self.list_next);
        }
        if self.page_pending.is_none() {
            self.page_wanted = self.failed_after.take().or(self.page_next).or_else(|| {
                self.cursor
                    .filter(|(_, high, _)| self.covered_high.is_some_and(|covered| covered < *high))
                    .and(self.covered_high)
            });
        }
    }
    /// Explicit operator view only. Receive/publish/ACK never call this.
    pub fn viewed_all(&mut self) -> Option<HistoryAction> {
        let (read, high, _) = self.cursor?;
        if self.selected != "all" || high <= read.max(self.read_sent) {
            return None;
        }
        self.read_sent = high;
        Some(HistoryAction::Read {
            stream: self.stream()?,
            sequence: high,
        })
    }
    pub fn replies(&mut self, results: &Results) -> bool {
        let Some(request) = &self.list else {
            return false;
        };
        let Some(reply) = results.rows().find(|r| r.op == request.id) else {
            return false;
        };
        let after = request.after;
        self.list = None;
        match &reply.reply {
            Reply::Messaging(wire::Response::Messages { page }) => {
                self.issue = None;
                self.list_failed_after = None;
                self.list_next = page.next;
                self.listed = page
                    .items
                    .iter()
                    .take(128)
                    .map(|item| {
                        (
                            item.view().id,
                            item.bus.clone(),
                            item.view().context.to_owned(),
                            MessageAttachments::bounded(item.view().attachments),
                        )
                    })
                    .collect();
                while serde_json::to_vec(&self.listed)
                    .map_or(true, |bytes| bytes.len() > 256 * 1024)
                {
                    if self.listed.is_empty() {
                        break;
                    }
                    self.listed.remove(0);
                    self.omitted += 1;
                }
                // Join by the actual publication position, never ledger sequence.
                for (id, bus, context, _) in &self.listed {
                    if let Some(old) = self
                        .rows
                        .iter_mut()
                        .find(|r| r.row.id == *id && r.row.bus.as_ref() == bus.as_ref())
                    {
                        self.bytes -= old.size;
                        old.context = context.clone();
                        old.size = serde_json::to_vec(&(&old.row, &old.context))
                            .map_or(1024 * 1024 + 1, |bytes| bytes.len() + 33);
                        self.bytes += old.size;
                    }
                }
                self.trim();
                if page.items.len() > 128 {
                    self.issue = Some("inbox page exceeds view window · more retained".into());
                }
            }
            Reply::Refused(reason) | Reply::Unknown(reason) => {
                self.list_failed_after = Some(after);
                self.issue = Some(format!("inbox after {after} · {reason}"));
            }
            _ => self.issue = Some("unexpected_inbox_reply".into()),
        }
        true
    }
    fn complete(&self) -> bool {
        let Some((read, high, unread)) = self.cursor else {
            return false;
        };
        if unread == 0 {
            return true;
        }
        self.covered_high.is_some_and(|covered| covered >= high)
            && self.gap_after.is_none_or(|last| last <= read)
            && self
                .rows
                .iter()
                .filter(|r| r.row.bus.as_ref().is_some_and(|bus| bus.seq > read))
                .count() as u64
                == unread
    }
    pub fn seat_unread(&self, id: &str) -> UnreadCount {
        if !self.complete() {
            return UnreadCount::More;
        }
        let read = self.cursor.map_or(0, |c| c.0);
        UnreadCount::Exact(
            self.rows
                .iter()
                .filter(|row| {
                    row.row.author.kind == wire::AuthorKind::Seat
                        && row.row.author.id == id
                        && row.row.bus.as_ref().is_some_and(|b| b.seq > read)
                })
                .count() as u64,
        )
    }
    pub fn entries(
        &self,
        model: &crate::model::Model,
        groups: &VecDeque<SendGroup>,
        channels: &[wire::Node],
    ) -> Vec<Entry> {
        let Some(scope) = self.scope else {
            return vec![];
        };
        let mut out = vec![];
        if let Some((read, _, unread)) = self.cursor {
            let complete = self.complete();
            out.push(Entry::Tabs {
                channels: channels
                    .iter()
                    .filter(|node| node.path.starts_with("channels/"))
                    .map(|node| Channel {
                        name: node.path.trim_start_matches("channels/").into(),
                        muted: node.paused,
                        unread: if complete {
                            UnreadCount::Exact(
                                self.rows
                                    .iter()
                                    .filter(|r| {
                                        r.context == node.path
                                            && r.row.bus.as_ref().is_some_and(|bus| bus.seq > read)
                                    })
                                    .count() as u64,
                            )
                        } else {
                            UnreadCount::More
                        },
                    })
                    .collect(),
                selected: self.selected.trim_start_matches("channels/").into(),
                total: Some(unread),
            });
        }
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .filter(|row| self.selected == "all" || row.context == self.selected)
            .map(|r| r.row.clone())
            .collect();
        rows.sort_by_key(|row| row.bus.as_ref().map(|bus| bus.seq));
        let stream = self.stream().unwrap();
        let contexts = self
            .rows
            .iter()
            .map(|row| (row.row.id, row.context.clone()))
            .collect::<BTreeMap<_, _>>();
        let attachments = self
            .listed
            .iter()
            .map(|(id, _, _, attachments)| (*id, attachments.clone()))
            .collect();
        let messages = message_entries_with_attachments(
            &rows,
            &contexts,
            groups,
            scope,
            model,
            self.cursor.map(|c| (stream.as_str(), c.0)),
            &attachments,
        );
        out.extend(messages);
        // Bounded cache/pagination bookkeeping belongs to the host, not the
        // conversation. Read coverage still uses complete(); failures stay visible.
        let note = if let Some(issue) = self
            .lane_issue
            .as_ref()
            .or(self.page_issue.as_ref())
            .or(self.issue.as_ref())
        {
            Some(issue.clone())
        } else if self.cursor.is_none() && self.rows.is_empty() {
            Some("Loading messages…".into())
        } else {
            None
        };
        if let Some(text) = note {
            out.push(Entry::Note {
                text,
                padding: [4.; 4],
            });
        }
        out
    }
}

#[cfg(test)]
mod quiet_history_tests {
    use super::*;

    #[test]
    fn normal_window_bookkeeping_is_quiet_while_real_failures_and_read_gaps_remain() {
        let mut history = History {
            scope: Some(SwarmId::from_u128(17)),
            selected: "all".into(),
            cursor: Some((0, 4, 4)),
            listed: (1..=4)
                .map(|id| {
                    (
                        Ulid(id),
                        None,
                        "operator/inbox".into(),
                        MessageAttachments::default(),
                    )
                })
                .collect(),
            list_next: Some(128),
            page_next: Some(128),
            omitted: 10,
            ..Default::default()
        };
        let entries = history.entries(&Default::default(), &Default::default(), &[]);
        assert!(
            !entries
                .iter()
                .any(|entry| matches!(entry, Entry::Note { .. }))
        );
        assert!(matches!(history.seat_unread("a seat"), UnreadCount::More));
        assert_eq!(
            history.listed.len(),
            4,
            "presentation never removes originals or coverage evidence"
        );
        history.issue = Some("inbox read failed · retry available".into());
        assert!(
            history
                .entries(&Default::default(), &Default::default(), &[])
                .iter()
                .any(
                    |entry| matches!(entry, Entry::Note {text,..} if text.contains("read failed"))
                )
        );
    }
}
