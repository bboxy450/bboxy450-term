// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Generation-owned projection of harness questions. The harness still decides each answer.
use super::{Command, LogicClient, Worker};
use crate::store::{Result, invalid};
use bboxy450_harness_wake_up::{
    Evidence,
    rich::{self, InboxEvent},
};
use bbt_proto::gui::{
    ApprovalOption, GuiReply, InboxQuestion, QuestionAnswerResult, QuestionAnswerer,
    QuestionClient, QuestionContent, QuestionCursor,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use tokio::sync::oneshot;
use ulid::Ulid;

const PAGE: usize = 64;
pub(super) type Page = (Vec<InboxQuestion>, Option<QuestionCursor>);
type Outcome = Result<QuestionAnswerResult>;
type AnswerWaiters = BTreeMap<Ulid, (Ulid, Vec<oneshot::Sender<Outcome>>)>;

pub(crate) struct AnswerRequest {
    pub seat: Ulid,
    pub generation: u64,
    pub item: Ulid,
    pub op: Ulid,
    pub reply: Value,
    pub by: QuestionClient,
}

pub(super) struct Submission {
    client: rich::Client,
    session: Ulid,
    operation: Value,
    outcome: oneshot::Receiver<Outcome>,
}

pub(super) enum Admission {
    Settled(QuestionAnswerResult),
    Submit(Box<Submission>),
}

#[derive(Debug, Clone)]
pub(crate) enum InboxNotice {
    Invalidated,
    Opened {
        seat: Ulid,
        item: Ulid,
    },
    Closed {
        seat: Ulid,
        item: Ulid,
        by: QuestionAnswerer,
    },
}

impl InboxNotice {
    pub(crate) fn into_reply(self) -> GuiReply {
        match self {
            Self::Invalidated => GuiReply::Gap {
                stream: "inbox".into(),
                first: 0,
                last: 0,
                reason: "harness inbox availability changed; page Questions".into(),
            },
            Self::Opened { seat, item } => GuiReply::QuestionOpened { seat, item },
            Self::Closed { seat, item, by } => GuiReply::QuestionClosed { seat, item, by },
        }
    }
}

struct Open {
    asked_at: u64,
    message: String,
    content: QuestionContent,
    announced: bool,
}

struct Closed {
    op: Option<Ulid>,
    by: QuestionAnswerer,
}

#[derive(Default)]
pub(super) struct Inbox {
    session: Option<Ulid>,
    ready: bool,
    open: BTreeMap<Ulid, Open>,
    closed: BTreeMap<Ulid, Closed>,
    pending: AnswerWaiters,
}

fn answerer(body: &Value) -> QuestionAnswerer {
    ["answered_by", "by", "answerer"]
        .into_iter()
        .find_map(|key| serde_json::from_value(body[key].clone()).ok())
        .unwrap_or_else(|| QuestionAnswerer::Resolver {
            by: "unknown".into(),
            details: BTreeMap::new(),
        })
}

impl Inbox {
    pub(super) fn disconnect(&mut self) -> bool {
        self.pending.clear();
        std::mem::take(&mut self.ready) && !self.open.is_empty()
    }

    fn observe(&mut self, seat: Ulid, event: InboxEvent, mut publish: impl FnMut(InboxNotice)) {
        match event {
            InboxEvent::Connected { session } => {
                if self.session != Some(session) {
                    self.open.clear();
                    self.closed.clear();
                    self.pending.clear();
                    self.session = Some(session);
                }
                self.ready = false;
            }
            InboxEvent::Ready => {
                self.ready = true;
                if self.open.values().any(|entry| entry.announced) {
                    publish(InboxNotice::Invalidated);
                }
                for (item, entry) in &mut self.open {
                    if !entry.announced {
                        entry.announced = true;
                        publish(InboxNotice::Opened { seat, item: *item });
                    }
                }
            }
            InboxEvent::Disconnected => {
                if self.disconnect() {
                    publish(InboxNotice::Invalidated);
                }
            }
            InboxEvent::Opened {
                item,
                asked_at,
                head,
            } => {
                if self.closed.contains_key(&item) {
                    return;
                }
                let Some(message) = head["message"].as_str().map(str::to_owned) else {
                    return;
                };
                let Ok(content) = serde_json::from_value::<QuestionContent>(head) else {
                    return;
                };
                if let std::collections::btree_map::Entry::Vacant(entry) = self.open.entry(item) {
                    entry.insert(Open {
                        asked_at,
                        message,
                        content,
                        announced: self.ready,
                    });
                    if self.ready {
                        publish(InboxNotice::Opened { seat, item });
                    }
                }
            }
            InboxEvent::Closed { item, cause, body } => {
                let std::collections::btree_map::Entry::Vacant(entry) = self.closed.entry(item)
                else {
                    return;
                };
                let by = answerer(&body);
                entry.insert(Closed {
                    op: cause,
                    by: by.clone(),
                });
                if self.open.remove(&item).is_some_and(|open| open.announced) {
                    publish(InboxNotice::Closed { seat, item, by });
                }
                if let Some(op) = cause {
                    self.finish(op, Ok(QuestionAnswerResult::Applied));
                }
            }
            InboxEvent::Rejected { op, reason } => {
                if self.pending.contains_key(&op) {
                    let result = if reason["reason"] == "already_answered" {
                        serde_json::from_value(reason["by"].clone())
                            .map(|by| QuestionAnswerResult::AlreadyAnswered { by })
                            .map_err(|_| invalid("harness omitted the winning answerer"))
                    } else {
                        Err(invalid(format!("harness refused the answer: {reason}")))
                    };
                    self.finish(op, result);
                }
            }
        }
    }

    fn finish(&mut self, op: Ulid, outcome: Outcome) {
        if let Some((_, waiters)) = self.pending.remove(&op) {
            for waiter in waiters {
                let result = outcome
                    .as_ref()
                    .map(Clone::clone)
                    .map_err(|error| invalid(error.to_string()));
                let _ = waiter.send(result);
            }
        }
    }

    fn prepare(&mut self, client: rich::Client, request: AnswerRequest) -> Result<Admission> {
        let op = request.op;
        if request.op == Ulid::nil() {
            return Err(invalid("an answer requires an operation ID"));
        }
        if let Some(Closed { op: winner, by }) = self.closed.get(&request.item) {
            return Ok(Admission::Settled(if Some(op) == *winner {
                QuestionAnswerResult::Applied
            } else {
                QuestionAnswerResult::AlreadyAnswered { by: by.clone() }
            }));
        }
        let Some(hello) = client
            .hello()
            .filter(|hello| self.session == Some(hello.session) && self.ready)
        else {
            return Err(invalid(
                "harness inbox unavailable; retry the same operation ID",
            ));
        };
        let Some(Open { content, .. }) = self.open.get(&request.item) else {
            return Err(invalid("harness inbox item is unknown"));
        };
        let item = request.item;
        let by = request.by;
        let operation = match content {
            QuestionContent::Question { .. } => json!({
                "type":"answer_question", "question":item, "answer":request.reply, "client":by
            }),
            QuestionContent::Approval { options } => {
                let option: ApprovalOption = serde_json::from_value(request.reply)?;
                if !options.contains(&option) {
                    return Err(invalid("approval reply was not offered by the harness"));
                }
                json!({"type":"resolve_approval", "approval":item, "decision":{"decision":option}, "client":by})
            }
        };
        self.pending.retain(|_, (_, waiters)| {
            waiters.retain(|waiter| !waiter.is_closed());
            !waiters.is_empty()
        });
        if self.pending.len() >= 64 {
            return Err(invalid("harness inbox busy; retry the same operation ID"));
        }
        let (tx, outcome) = oneshot::channel();
        let (original_item, waiters) = self.pending.entry(op).or_insert_with(|| (item, Vec::new()));
        if *original_item != item {
            return Err(invalid(
                "operation ID already belongs to another inbox item",
            ));
        }
        if waiters.len() >= 8 {
            return Err(invalid("answer already has eight waiters"));
        }
        waiters.push(tx);
        Ok(Admission::Submit(Box::new(Submission {
            client,
            session: hello.session,
            operation,
            outcome,
        })))
    }
}

impl LogicClient {
    pub(crate) async fn gui_questions(&self, after: Option<QuestionCursor>) -> Result<Page> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiQuestions(after, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }

    pub(crate) async fn gui_answer(&self, request: AnswerRequest) -> Outcome {
        let op = request.op;
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiAnswer(request, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        match rx.await.map_err(|_| self.unavailable("logic stopped"))?? {
            Admission::Settled(result) => Ok(result),
            Admission::Submit(submission) => {
                let Submission {
                    client,
                    session,
                    operation,
                    mut outcome,
                } = *submission;
                let admitted = client.send(op, session, operation).await;
                if admitted.evidence != Evidence::NativeQueued {
                    return outcome
                        .try_recv()
                        .unwrap_or_else(|_| Err(invalid(admitted.note)));
                }
                tokio::time::timeout(Duration::from_secs(5), outcome)
                    .await
                    .ok()
                    .and_then(std::result::Result::ok)
                    .ok_or_else(|| invalid("answer outcome unknown; retry the same operation ID"))?
            }
        }
    }
}

impl Worker {
    pub(super) fn gui_questions(&self, after: Option<QuestionCursor>) -> Result<Page> {
        // Keep at most one page plus its lookahead, irrespective of seat count.
        let mut page = BTreeMap::new();
        for (&seat, view) in &self.views {
            if !view.inbox.ready || view.info.exited.is_some() {
                continue;
            }
            let mut record = None;
            for (
                &item,
                Open {
                    asked_at,
                    message,
                    content,
                    ..
                },
            ) in &view.inbox.open
            {
                let cursor = QuestionCursor {
                    asked_at: *asked_at,
                    seat,
                    generation: view.generation,
                    item,
                };
                if after.is_some_and(|after| cursor <= after)
                    || page.len() > PAGE
                        && page
                            .last_key_value()
                            .is_some_and(|(last, _)| cursor >= *last)
                {
                    continue;
                }
                if record.is_none() {
                    record = self.registry.get(seat).ok();
                }
                let Some(record) = record.as_ref() else { break };
                page.insert(
                    cursor,
                    InboxQuestion {
                        seat,
                        seat_name: record.name.clone(),
                        swarm: record.swarm,
                        role: record.role.name.clone(),
                        generation: view.generation,
                        item,
                        message: message.clone(),
                        asked_at: *asked_at,
                        content: content.clone(),
                    },
                );
                if page.len() > PAGE + 1 {
                    page.pop_last();
                }
            }
        }
        let cursor = if page.len() > PAGE {
            page.pop_last();
            page.last_key_value().map(|(cursor, _)| *cursor)
        } else {
            None
        };
        Ok((page.into_values().collect(), cursor))
    }

    pub(super) fn gui_answer(&mut self, request: AnswerRequest) -> Result<Admission> {
        let Some(view) = self
            .views
            .get_mut(&request.seat)
            .filter(|view| view.generation == request.generation && view.info.exited.is_none())
        else {
            return Ok(Admission::Settled(QuestionAnswerResult::StaleGeneration));
        };
        let client = view
            .rich_connection
            .as_ref()
            .ok_or_else(|| invalid("seat has no harness inbox connection"))?
            .client
            .clone();
        view.inbox.prepare(client, request)
    }

    pub(super) fn inbox_event(&mut self, seat: Ulid, generation: u64, event: InboxEvent) {
        if let Some(view) = self
            .views
            .get_mut(&seat)
            .filter(|view| view.generation == generation)
        {
            view.inbox.observe(seat, event, |notice| {
                let _ = self.client.inbox_notices.send(notice);
            });
        }
    }
}
