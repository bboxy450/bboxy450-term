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

const EVENT_PAGE: u64 = 128;

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
                limit: EVENT_PAGE as usize,
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
    covered_after: Option<u64>,
    page_floor: Option<u64>,
    older_join: Option<(u64, u64)>, // prior covered lower bound and high
    bootstrap_tail: bool,
    tail_after: Option<u64>,
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
            bootstrap_tail: true,
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
        self.page_wanted = Some(
            self.page_next
                .or(self.covered_high)
                .or_else(|| {
                    self.cursor
                        .map(|(_, high, _)| high.saturating_sub(EVENT_PAGE))
                })
                .unwrap_or(0),
        );
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
        if self.bootstrap_tail {
            self.bootstrap_tail = false;
            let after = high.saturating_sub(EVENT_PAGE);
            if after != 0 {
                // A cursor already present in the lane replaces the initial zero
                // request. Otherwise only one speculative oldest page can precede
                // this bounded correction; never walk the entire event stream.
                self.tail_after = Some(after);
                if self.failed_after.is_none() {
                    self.page_wanted = Some(after);
                }
            }
        }
    }
    pub fn row(&mut self, row: &wire::MessageRow, context: Option<&str>, projected_at: u64) {
        let Some(stream) = self.stream() else {
            return;
        };
        if row.bus.as_ref().is_none_or(|bus| bus.stream != stream) {
            return;
        }
        if self.rows.iter().any(|old| {
            old.row.id == row.id
                && (old.projected_at > projected_at
                        // A display clock cannot revoke a confirmed signature.
                        || (old.row.signature == wire::SignatureState::Signed
                            && row.signature == wire::SignatureState::Pending))
        }) {
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
        if self.tail_after == Some(after) {
            self.tail_after = None;
            self.page_floor = Some(after);
            self.covered_after = Some(after);
            self.older_join = None;
            self.chain_next = next;
            self.covered_high = next.is_none().then_some(high);
            self.page_next = next;
        } else {
            self.page_floor = Some(self.page_floor.map_or(after, |floor| floor.min(after)));
            let join = self.older_join.or_else(|| {
                self.covered_after
                    .zip(self.covered_high)
                    .filter(|(lower, _)| after < *lower)
            });
            if let Some((lower, through)) = join {
                self.covered_after = Some(self.covered_after.map_or(after, |old| old.min(after)));
                if next.is_none_or(|next| next >= lower) {
                    // An older page overlaps the already retained tail. Its `next`
                    // is not another forward request through that same tail.
                    self.covered_high = Some(if next.is_none() {
                        high.max(through)
                    } else {
                        through
                    });
                    self.chain_next = None;
                    self.page_next = None;
                    self.older_join = None;
                } else {
                    // Byte-bounded pages can stop short of the existing interval.
                    // Continue only on an explicit more(), and keep unread unknown.
                    self.older_join = Some((lower, through));
                    self.covered_high = None;
                    self.chain_next = next;
                    self.page_next = next;
                }
            } else {
                if after == 0
                    || (self.chain_next != Some(after) && self.covered_high != Some(after))
                {
                    self.covered_after = Some(after);
                }
                self.chain_next = next;
                self.covered_high = next.is_none().then_some(high);
                self.page_next = next;
            }
        }
        if let Some(after) = self.tail_after {
            self.page_wanted = Some(after);
        }
    }
    pub fn page_failed(&mut self, revision: u64, reason: &str) {
        self.page_revision = self.page_revision.max(revision);
        if let Some((after, previous)) = self.page_pending
            && revision > previous
        {
            self.page_pending = None;
            self.failed_after = Some(after);
            self.page_wanted = None;
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
            request: wire::Request::Recent {
                channel: (self.selected != "all").then(|| self.selected.clone()),
                before: (after > 0).then_some(after),
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
            self.page_wanted = self
                .failed_after
                .take()
                .or(self.tail_after)
                .or(self.page_next)
                .or_else(|| {
                    self.cursor
                        .filter(|(_, high, _)| {
                            self.covered_high.is_some_and(|covered| covered < *high)
                        })
                        .and(self.covered_high)
                })
                .or_else(|| {
                    self.page_floor
                        .filter(|after| *after > 0)
                        .map(|after| after.saturating_sub(EVENT_PAGE))
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
        if let Reply::Messaging(wire::Response::Recent {
            page,
            rows,
            projected_at,
        }) = &reply.reply
        {
            if page.items.len() > 128
                || rows.len() != page.items.len()
                || page
                    .items
                    .windows(2)
                    .any(|items| items[0].sequence >= items[1].sequence)
                || page
                    .items
                    .iter()
                    .any(|item| item.sequence == 0 || (after > 0 && item.sequence >= after))
                || page.next.is_some_and(|next| {
                    next == 0
                        || (after > 0 && next >= after)
                        || page.items.first().is_some_and(|item| next > item.sequence)
                })
                || rows.iter().zip(&page.items).any(|(row, item)| {
                    row.id != item.view().id
                        || row.bus != item.bus
                        || self
                            .scope
                            .is_none_or(|scope| item.view().swarm_id.0 != scope.0.0)
                })
            {
                self.list_failed_after = Some(after);
                self.issue = Some("recent_message_projection_mismatch".into());
                return true;
            }
            // The ledger page chooses messages, so receipt traffic cannot hide
            // the latest conversation. Event coverage/read cursors stay separate.
            for (row, item) in rows.iter().zip(&page.items) {
                self.row(row, Some(item.view().context), *projected_at);
            }
        }
        match &reply.reply {
            Reply::Messaging(
                wire::Response::Messages { page } | wire::Response::Recent { page, .. },
            ) => {
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
        self.covered_after.is_some_and(|after| after <= read)
            && self.covered_high.is_some_and(|covered| covered >= high)
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
    fn recent_message_read_is_scoped_and_does_not_claim_event_coverage() {
        let mut history = History::default();
        let scope = SwarmId::from_u128(90);
        history.select(scope);
        history.channel("channels/review");
        let UiMsg::Messaging {
            id,
            swarm_id,
            request,
        } = history.commands().pop().unwrap()
        else {
            panic!("message read")
        };
        assert_eq!(swarm_id, scope);
        assert!(
            matches!(request, wire::Request::Recent { channel:Some(channel), before:None, limit:128 } if channel == "channels/review")
        );
        let mut results = Results::default();
        results.push(
            id,
            Reply::Messaging(wire::Response::Recent {
                page: wire::Page {
                    items: vec![],
                    next: Some(42),
                },
                rows: vec![],
                projected_at: 100,
            }),
        );
        assert!(history.replies(&results));
        assert!(history.issue.is_none());
        assert!(history.covered_high.is_none());
        assert!(history.cursor.is_none());
        assert!(history.commands().is_empty());
        history.more();
        assert!(matches!(
            history.commands().as_slice(),
            [UiMsg::Messaging {
                request: wire::Request::Recent {
                    before: Some(42),
                    ..
                },
                ..
            }]
        ));
    }

    #[test]
    fn malformed_recent_projection_is_refused_without_inserting_rows_or_retrying() {
        let mut history = History::default();
        let scope = SwarmId::from_u128(89);
        history.select(scope);
        let UiMsg::Messaging { id, .. } = history.commands().pop().unwrap() else {
            panic!("message read")
        };
        let mut results = Results::default();
        results.push(
            id,
            Reply::Messaging(wire::Response::Recent {
                page: wire::Page {
                    items: vec![],
                    next: None,
                },
                rows: vec![latest_row(scope, 1)],
                projected_at: 100,
            }),
        );
        assert!(history.replies(&results));
        assert!(history.rows.is_empty());
        assert_eq!(
            history.issue.as_deref(),
            Some("recent_message_projection_mismatch")
        );
        assert!(history.commands().is_empty());
        history.more();
        assert!(
            !history.commands().is_empty(),
            "only an explicit view retries"
        );
    }

    fn request_page(history: &mut History) -> u64 {
        history
            .actions()
            .into_iter()
            .find_map(|action| match action {
                HistoryAction::Page { after, .. } => Some(after),
                _ => None,
            })
            .expect("one requested retained page")
    }
    fn latest_row(scope: SwarmId, sequence: u64) -> wire::MessageRow {
        wire::MessageRow {
            id: Ulid(300),
            bus: Some(wire::BusPosition {
                stream: format!("swarm-{scope}"),
                seq: sequence,
            }),
            author: wire::AuthorRef {
                kind: wire::AuthorKind::Seat,
                id: "fixture-seat".into(),
                name: "Fixture".into(),
            },
            issuer_kind: Some(wire::IssuerKind::LocalDev),
            signature: wire::SignatureState::Signed,
            signature_reason: None,
            upgrade_of: None,
            kind: wire::MessageKind::Decision,
            priority_class: bbt_proto::registry::Class::Worker,
            priority: 8,
            text: "FOLLOWING-AGAIN".into(),
            ts: 1000,
            receipts: Default::default(),
        }
    }

    #[test]
    fn first_cursor_bootstraps_latest_bounded_page_without_claiming_skipped_unread() {
        let scope = SwarmId::from_u128(91);
        let mut history = History::default();
        history.select(scope);
        history.cursor(0, 300, 1);
        assert_eq!(request_page(&mut history), 172);
        history.row(&latest_row(scope, 299), Some("operator/inbox"), 100);
        history.page(1, 300, None);
        assert_eq!(history.page_floor, Some(172));
        assert_eq!(history.covered_after, Some(172));
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::More);
        assert!(
            history.actions().is_empty(),
            "no automatic walk through older history"
        );
        assert!(history.entries(&Default::default(), &Default::default(), &[])
            .iter().any(|entry| matches!(entry, Entry::Message {data,..} if data.text == "FOLLOWING-AGAIN")));
    }

    #[test]
    fn a_speculative_oldest_page_is_followed_once_by_the_confirmed_tail() {
        let mut history = History::default();
        history.select(SwarmId::from_u128(92));
        assert_eq!(request_page(&mut history), 0);
        history.cursor(0, 300, 1);
        assert!(
            history.actions().is_empty(),
            "the existing read retains its completion fence"
        );
        history.page(1, 300, Some(128));
        assert_eq!(request_page(&mut history), 172);
        history.cursor(0, 320, 1);
        history.page(2, 300, None);
        assert_eq!(
            history.covered_high,
            Some(300),
            "page high is not a newer live cursor"
        );
        assert!(
            history.actions().is_empty(),
            "cursor movement never starts a read loop"
        );
        history.more();
        assert_eq!(request_page(&mut history), 300);
        history.page(3, 320, None);
        assert_eq!(history.covered_after, Some(172));
        assert_eq!(history.covered_high, Some(320));
    }

    #[test]
    fn older_scroll_pages_join_the_tail_only_after_real_contiguous_coverage() {
        let scope = SwarmId::from_u128(93);
        let mut history = History::default();
        history.select(scope);
        history.cursor(0, 300, 1);
        assert_eq!(request_page(&mut history), 172);
        history.row(&latest_row(scope, 299), Some("operator/inbox"), 100);
        history.page(1, 300, None);
        history.more();
        assert_eq!(request_page(&mut history), 44);
        history.page(2, 300, Some(100)); // Byte bound stops before the tail's lower edge.
        assert_eq!(history.covered_high, None);
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::More);
        assert!(history.actions().is_empty());
        history.more();
        assert_eq!(request_page(&mut history), 100);
        history.page(3, 300, Some(228)); // This page overlaps the retained tail.
        assert_eq!(history.covered_high, Some(300));
        assert_eq!(history.covered_after, Some(44));
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::More);
        history.more();
        assert_eq!(request_page(&mut history), 0);
        history.page(4, 300, Some(128));
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::Exact(1));
        history.more();
        assert!(
            history.actions().is_empty(),
            "all older pages are already covered"
        );
        history.gap(5, 6);
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::More);
        history.cursor(6, 300, 1);
        assert_eq!(history.seat_unread("fixture-seat"), UnreadCount::Exact(1));
    }

    #[test]
    fn tail_refusal_requires_explicit_retry_and_a_newer_completion_revision() {
        let mut history = History::default();
        history.select(SwarmId::from_u128(94));
        history.cursor(0, 500, 1);
        assert_eq!(request_page(&mut history), 372);
        history.page_failed(1, "private fixture refused");
        history.cursor(0, 600, 1);
        assert!(history.actions().is_empty());
        history.more();
        assert_eq!(request_page(&mut history), 372);
        history.page(1, 500, None); // Replayed completion cannot satisfy the retry.
        assert!(history.page_pending.is_some());
        history.page(2, 500, None);
        assert!(history.page_pending.is_none());
        assert!(history.page_issue.is_none());
        assert_eq!(history.covered_high, Some(500));
    }

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
