// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The Chat section's projection: what needs the operator, pinned, then signal
//! as a ledger of one line per event, task events under their task.
//! It only chooses and orders rows the history already built; delivery, read
//! cursors and the full cards ("all") are untouched.
use super::wire::{self, MessageKind};
use bbt_ui::modules::{
    composer::Feed,
    stream_dock::{Entry, Line, Message},
};
use bbt_mux::Ulid;
use std::collections::{BTreeMap, VecDeque};

/// Pinned rows above the list; the last one counts the rest.
pub const PINNED: usize = 5;
/// A line's object when it is message text.
const OBJECT: usize = 60;

/// What a message is about, from its stored typed body. A row that arrives on
/// the live event lane carries no body, so it stays untyped until a history
/// page covers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    Task { task: Ulid, event: String },
    Gate { red: bool },
}
impl Typed {
    pub fn of(body: &wire::Body) -> Option<Self> {
        match body {
            wire::Body::Task { task_id, event, .. } => Some(Self::Task {
                task: *task_id,
                event: event.clone(),
            }),
            wire::Body::GateResult { verdict, .. } => Some(Self::Gate {
                red: matches!(
                    verdict,
                    wire::GateVerdict::Red | wire::GateVerdict::ManualRevert
                ),
            }),
            _ => None,
        }
    }
}
/// The closed verb set: a task event name, never the message's prose.
fn verb(event: &str) -> &'static str {
    match event {
        "delegate" => "delegated",
        "report" => "reported",
        "accept" => "accepted",
        "reject" => "rejected",
        "amend" => "amended",
        "release" => "released",
        _ => "",
    }
}

/// Typed facts of one retained message. Nothing is read from its prose except
/// the trailing `?` of a seat's message to the operator inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub kind: MessageKind,
    /// Written by the operator.
    pub operator: bool,
    /// Addressed to the operator inbox.
    pub to_operator: bool,
    pub typed: Option<Typed>,
}
impl Fact {
    pub fn of(row: &wire::MessageRow, context: &str, typed: Option<&Typed>) -> Self {
        Self {
            typed: typed.cloned(),
            kind: row.kind,
            operator: matches!(
                row.author.kind,
                wire::AuthorKind::Operator | wire::AuthorKind::OperatorLocal
            ),
            to_operator: context == "operator/inbox",
        }
    }
    /// A gate result is hidden only once its body is known not to be red.
    fn signal(&self) -> bool {
        self.operator
            || self.to_operator
            || matches!(
                self.kind,
                MessageKind::Decision | MessageKind::Task | MessageKind::Landed
            )
            || (self.kind == MessageKind::GateResult
                && self.typed != Some(Typed::Gate { red: false }))
    }
    /// The row as one ledger line; None keeps the card. The operator's own
    /// messages and plain mail to the operator stay cards.
    fn line(&self, message: &Message) -> Option<Line> {
        let verb = match (self.kind, &self.typed) {
            (MessageKind::Task, Some(Typed::Task { event, .. })) => verb(event),
            (MessageKind::Task, _) => "",
            (MessageKind::Decision, _) if !self.operator => "decided",
            _ if asked(self, message) => "asked",
            _ => return None,
        };
        Some(Line {
            time: message.time.clone(),
            who: who(message),
            verb: verb.into(),
            object: object(&message.text),
            needs: verb == "asked" && message.unread,
            ..Default::default()
        })
    }
}

/// A seat waiting on the operator, named by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    pub seat: String,
    pub name: String,
    pub why: &'static str,
}
pub fn waiting(model: &crate::model::Model, scope: bbt_proto::SwarmId) -> Vec<Waiting> {
    use bbt_ui_model::{Activity, WaitKind};
    model
        .ui
        .seats
        .iter()
        .filter(|(_, seat)| seat.swarm == scope)
        .filter_map(|(id, seat)| {
            let Activity::Waiting(kind) = seat.activity else {
                return None;
            };
            Some(Waiting {
                seat: id.to_string(),
                name: seat.name.clone(),
                why: match kind {
                    WaitKind::Permission => "permission",
                    WaitKind::Question => "question",
                    WaitKind::Auth => "auth",
                },
            })
        })
        .collect()
}

fn object(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(OBJECT)
        .collect()
}
fn fact<'a>(facts: &'a BTreeMap<String, Fact>, message: &Message) -> Option<&'a Fact> {
    message.message_ids.iter().find_map(|id| facts.get(id))
}
/// A seat's message to the operator inbox that ends with a question mark.
fn asked(fact: &Fact, message: &Message) -> bool {
    fact.to_operator && !fact.operator && message.text.trim_end().ends_with('?')
}
fn who(message: &Message) -> String {
    message
        .author
        .as_ref()
        .map_or_else(|| message.from.clone(), |author| author.name.clone())
}

/// What needs the operator, in the order it blocks work: seats waiting now,
/// reports that verified and await acceptance, unread messages to the operator.
fn needs(
    entries: &[Entry],
    facts: &BTreeMap<String, Fact>,
    tasks: &VecDeque<wire::TaskCard>,
    waiting: &[Waiting],
    model: &crate::model::Model,
) -> Vec<Line> {
    let mut out: Vec<Line> = waiting
        .iter()
        .map(|seat| Line {
            who: seat.name.clone(),
            verb: "waiting".into(),
            object: seat.why.into(),
            seat: seat.seat.clone(),
            needs: true,
            ..Default::default()
        })
        .collect();
    out.extend(
        tasks
            .iter()
            .filter(|card| super::task_row(card).0.may_accept)
            .map(|card| Line {
                who: super::task_item(card, model).assignee,
                verb: "reported".into(),
                object: card.title.clone(),
                task: card.id.to_string(),
                needs: true,
                ..Default::default()
            }),
    );
    out.extend(entries.iter().rev().filter_map(|entry| {
        let Entry::Message { data, .. } = entry else {
            return None;
        };
        let fact = fact(facts, data)?;
        (data.unread && fact.to_operator && !fact.operator).then(|| Line {
            time: data.time.clone(),
            who: who(data),
            verb: if asked(fact, data) { "asked" } else { "" }.into(),
            object: object(&data.text),
            needs: true,
            ..Default::default()
        })
    }));
    if out.len() > PINNED {
        let more = out.len() - (PINNED - 1);
        out.truncate(PINNED - 1);
        out.push(Line {
            object: format!("+{more} more"),
            needs: true,
            ..Default::default()
        });
    }
    out
}

/// One task's events in arrival order; `latest` orders tasks newest first.
#[derive(Default)]
struct Thread {
    latest: usize,
    events: Vec<(String, Line)>,
}
/// Tasks that need the operator first, then newest first. A done task whose
/// last event is its acceptance folds to `title · done`.
fn ledger(threads: BTreeMap<Ulid, Thread>, tasks: &VecDeque<wire::TaskCard>) -> Vec<Entry> {
    let mut threads: Vec<_> = threads
        .into_iter()
        .map(|(task, thread)| {
            let card = tasks.iter().find(|card| card.id == task);
            let needs = card.is_some_and(|card| super::task_row(card).0.may_accept);
            (!needs, std::cmp::Reverse(thread.latest), task, card, thread)
        })
        .collect();
    threads.sort_by_key(|(waits, latest, ..)| (*waits, *latest));
    let mut out = vec![];
    for (waits, _, task, card, thread) in threads {
        let task = task.to_string();
        if let Some(card) = card
            && card.status == "done"
            && thread.events.last().is_some_and(|(event, _)| event == "accept")
        {
            out.push(Entry::Line {
                data: Line {
                    object: format!("{} · done", card.title),
                    task,
                    first: true,
                    ..Default::default()
                },
            });
            continue;
        }
        let last = thread.events.len() - 1;
        for (i, (_, mut data)) in thread.events.into_iter().enumerate() {
            if let Some(card) = card {
                data.object = card.title.clone();
            }
            data.task = task.clone();
            data.first = i == 0;
            data.needs = !waits && i == last;
            out.push(Entry::Line { data });
        }
    }
    out
}

/// `entries` is the complete history projection. Signal keeps decisions, task
/// messages, anything to or from the operator, gate results and landings; it
/// drops presence and seat-to-seat text. Task events, decisions and questions
/// become ledger lines, task threads first. The count is rows shown of
/// messages retained, pinned rows excluded.
pub fn project(
    entries: Vec<Entry>,
    facts: &BTreeMap<String, Fact>,
    tasks: &VecDeque<wire::TaskCard>,
    waiting: &[Waiting],
    model: &crate::model::Model,
    all: bool,
) -> (Vec<Entry>, Feed) {
    let total = entries
        .iter()
        .filter(|entry| matches!(entry, Entry::Message { .. }))
        .count();
    let pinned = needs(&entries, facts, tasks, waiting, model);
    let mut out: Vec<Entry> = pinned
        .iter()
        .cloned()
        .map(|data| Entry::Line { data })
        .collect();
    let mut threads: BTreeMap<Ulid, Thread> = BTreeMap::new();
    // Task threads go where the first row would: under the tabs, above the rest.
    let mut ledger_at = None;
    // A note directly after a message describes its card and leaves with it.
    let mut hidden = false;
    for (index, entry) in entries.into_iter().enumerate() {
        if !matches!(entry, Entry::Tabs { .. } | Entry::Filter { .. }) {
            ledger_at.get_or_insert(out.len());
        }
        match &entry {
            Entry::Message { data, .. } => {
                // The operator's own close notices are not presence traffic.
                let notice = data.message_ids.iter().any(|id| id.starts_with("close:"));
                let fact = fact(facts, data);
                hidden = !all && !notice && !fact.is_some_and(Fact::signal);
                if hidden {
                    continue;
                }
                if !all
                    && let Some(fact) = fact
                    && let Some(line) = fact.line(data)
                {
                    hidden = true;
                    if let Some(Typed::Task { task, event }) = &fact.typed {
                        let thread = threads.entry(*task).or_default();
                        thread.latest = index;
                        thread.events.push((event.clone(), line));
                    } else {
                        out.push(Entry::Line { data: line });
                    }
                    continue;
                }
            }
            Entry::Note { .. } if hidden => continue,
            _ => hidden = false,
        }
        out.push(entry);
    }
    let at = ledger_at.unwrap_or(out.len());
    out.splice(at..at, ledger(threads, tasks));
    let shown = out[pinned.len()..]
        .iter()
        .filter(|entry| matches!(entry, Entry::Message { .. } | Entry::Line { .. }))
        .count();
    (
        out,
        Feed {
            all,
            shown,
            total,
            pinned: pinned.len(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u128, kind: MessageKind, author: wire::AuthorKind, text: &str) -> wire::MessageRow {
        wire::MessageRow {
            id: Ulid(id),
            bus: Some(wire::BusPosition {
                stream: "swarm-test".into(),
                seq: id as u64,
            }),
            author: wire::AuthorRef {
                kind: author,
                id: "fixture".into(),
                name: if author == wire::AuthorKind::Seat {
                    "Vex"
                } else {
                    "operator"
                }
                .into(),
            },
            issuer_kind: None,
            signature: wire::SignatureState::Signed,
            signature_reason: None,
            upgrade_of: None,
            kind,
            priority_class: bbt_proto::registry::Class::Worker,
            priority: 0,
            text: text.into(),
            ts_ms: id as u64 * 60_000,
            receipts: Default::default(),
        }
    }
    fn card(id: u128, title: &str, status: &str, exit: Option<i32>) -> wire::TaskCard {
        wire::TaskCard {
            id: Ulid(id),
            path: format!("tasks/{id}.md"),
            title: title.into(),
            status: status.into(),
            assignee: Some("seat:vex".into()),
            delegated_by: None,
            updated: None,
            verify: exit.map(|exit_code| wire::TaskVerifySummary { exit_code }),
            touches: vec![],
        }
    }
    /// The history's own card projection over `(row, context)`, read up to `read`.
    fn fixture(
        rows: &[(wire::MessageRow, &str)],
        read: u64,
    ) -> (Vec<Entry>, BTreeMap<String, Fact>) {
        let model = crate::model::Model::default();
        let wire: Vec<_> = rows.iter().map(|(row, _)| row.clone()).collect();
        let mut entries = vec![Entry::Tabs {
            channels: vec![],
            selected: "all".into(),
            total: Some(0),
        }];
        entries.extend(super::super::message_entries(
            &wire,
            &BTreeMap::new(),
            &VecDeque::new(),
            bbt_proto::SwarmId::from_u128(1),
            &model,
            Some(("swarm-test", read)),
        ));
        let facts = rows
            .iter()
            .map(|(row, context)| (row.id.to_string(), Fact::of(row, context, None)))
            .collect();
        (entries, facts)
    }
    fn typed(facts: &mut BTreeMap<String, Fact>, message: u128, task: u128, event: &str) {
        facts.get_mut(&Ulid(message).to_string()).unwrap().typed = Some(Typed::Task {
            task: Ulid(task),
            event: event.into(),
        });
    }
    fn texts(entries: &[Entry]) -> Vec<&str> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Message { data, .. } => Some(data.text.as_str()),
                _ => None,
            })
            .collect()
    }
    fn lines(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Line { data } => Some(
                    [&data.time, &data.who, &data.verb, &data.object]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" · "),
                ),
                _ => None,
            })
            .collect()
    }
    use wire::AuthorKind::{Operator, Seat};

    #[test]
    fn signal_keeps_decisions_tasks_operator_traffic_and_gates_and_counts_shown_of_total() {
        let rows = [
            (row(1, MessageKind::Text, Seat, "seat to seat"), "seats/a/inbox"),
            (row(2, MessageKind::Decision, Seat, "decision"), "channels/review"),
            (row(3, MessageKind::Task, Seat, "task wake"), "seats/a/inbox"),
            (row(4, MessageKind::Text, Operator, "from operator"), "seats/a/inbox"),
            (row(5, MessageKind::Text, Seat, "to operator"), "operator/inbox"),
            (row(6, MessageKind::Presence, Seat, "Vex muted"), "channels/review"),
            (row(7, MessageKind::GateResult, Seat, "gate"), "channels/review"),
            (row(8, MessageKind::Landed, Seat, "landed"), "channels/review"),
            (row(9, MessageKind::Chunk, Seat, "channel chatter"), "channels/review"),
        ];
        let (mut entries, mut facts) = fixture(&rows, u64::MAX);
        facts.get_mut(&Ulid(7).to_string()).unwrap().typed = Some(Typed::Gate { red: true });
        // Not history rows: a presence line, the operator's close notice, and
        // a note describing the hidden message before it.
        entries.insert(
            2,
            Entry::Note {
                text: "attachment_view_limit".into(),
                padding: [4.; 4],
            },
        );
        for (ids, text) in [(vec![], "seat restarted"), (vec!["close:1".into()], "closed")] {
            entries.push(Entry::Message {
                data: Box::new(Message {
                    message_ids: ids,
                    text: text.into(),
                    presence: true,
                    ..Default::default()
                }),
                show_channel: None,
            });
        }
        entries.push(Entry::Note {
            text: "read failed".into(),
            padding: [4.; 4],
        });
        let model = crate::model::Model::default();
        let (signal, feed) =
            project(entries.clone(), &facts, &VecDeque::new(), &[], &model, false);
        // The decision and the untyped task wake are ledger lines, in place.
        let time = |minute: u64| crate::local_time::hm(minute * 60_000);
        assert_eq!(
            lines(&signal),
            [
                format!("{} · Vex · decided · decision", time(2)),
                format!("{} · Vex · task wake", time(3)),
            ]
        );
        assert_eq!(
            texts(&signal),
            ["from operator", "to operator", "gate", "landed", "closed"]
        );
        assert_eq!((feed.shown, feed.total, feed.pinned), (7, 11, 0));
        // A gate result known to be green is not signal.
        let mut green = facts.clone();
        green.get_mut(&Ulid(7).to_string()).unwrap().typed = Some(Typed::Gate { red: false });
        let (quiet, feed) =
            project(entries.clone(), &green, &VecDeque::new(), &[], &model, false);
        assert!(!texts(&quiet).contains(&"gate"));
        assert_eq!((feed.label().as_str(), feed.mode()), ("6 / 11", "signal"));
        assert!(matches!(signal[0], Entry::Tabs { .. }));
        let notes: Vec<_> = signal
            .iter()
            .filter_map(|entry| match entry {
                Entry::Note { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(notes, ["read failed"], "a hidden message takes its note");

        let (everything, feed) = project(entries.clone(), &facts, &VecDeque::new(), &[], &model, true);
        assert_eq!(everything, entries);
        assert_eq!((feed.label().as_str(), feed.mode()), ("11 / 11", "all"));
    }

    #[test]
    fn needs_you_pins_waiting_seats_then_verified_reports_then_unread_operator_mail() {
        let rows = [
            (row(1, MessageKind::Text, Seat, "old and read"), "operator/inbox"),
            (
                row(2, MessageKind::Text, Seat, "ready\nfor   review"),
                "operator/inbox",
            ),
            (row(3, MessageKind::Text, Seat, "unread peer"), "seats/a/inbox"),
            (row(4, MessageKind::Text, Operator, "mine"), "operator/inbox"),
            (
                row(5, MessageKind::Text, Seat, &format!("{}?", "why ".repeat(30))),
                "operator/inbox",
            ),
        ];
        let (entries, facts) = fixture(&rows, 1);
        let tasks: VecDeque<_> = [
            card(10, "Verified report", "reported", Some(0)),
            card(11, "Failed verify", "reported", Some(1)),
            card(12, "Unverified", "reported", None),
            card(13, "Accepted", "accepted", Some(0)),
        ]
        .into();
        let waiting = [Waiting {
            seat: "seat-1".into(),
            name: "Lumen".into(),
            why: "permission",
        }];
        let model = crate::model::Model::default();
        for all in [false, true] {
            let (out, feed) = project(entries.clone(), &facts, &tasks, &waiting, &model, all);
            assert_eq!(feed.pinned, 4);
            let pinned = &out[..feed.pinned];
            let asked = "why ".repeat(15);
            assert_eq!(
                lines(pinned),
                [
                    "Lumen · waiting · permission".to_owned(),
                    "seat:vex · reported · Verified report".to_owned(),
                    format!("{} · Vex · asked · {asked}", crate::local_time::hm(300_000)),
                    format!("{} · Vex · ready for review", crate::local_time::hm(120_000)),
                ]
            );
            let Entry::Line { data: seat } = &pinned[0] else {
                unreachable!()
            };
            assert_eq!((seat.seat.as_str(), seat.task.as_str()), ("seat-1", ""));
            let Entry::Line { data: task } = &pinned[1] else {
                unreachable!()
            };
            assert_eq!((task.seat.as_str(), &task.task), ("", &Ulid(10).to_string()));
            assert!(pinned.iter().all(
                |entry| matches!(entry, Entry::Line { data } if data.needs)
            ));
            // The pinned rows are extra: the list and its count are unchanged.
            assert_eq!(feed.total, 5);
            assert_eq!(feed.shown, if all { 5 } else { 4 });
        }
    }

    #[test]
    fn needs_you_is_bounded_and_counts_what_it_leaves_out() {
        let waiting: Vec<_> = (0..8)
            .map(|i| Waiting {
                seat: format!("seat-{i}"),
                name: format!("Seat{i}"),
                why: "question",
            })
            .collect();
        let model = crate::model::Model::default();
        let (out, feed) = project(vec![], &BTreeMap::new(), &VecDeque::new(), &waiting, &model, false);
        assert_eq!(feed.pinned, PINNED);
        assert_eq!(lines(&out).last().unwrap(), "+4 more");
        assert_eq!((feed.label().as_str(), feed.mode()), ("0 / 0", "signal"));
    }

    #[test]
    fn ledger_threads_task_events_needs_you_first_then_newest_and_folds_done() {
        let task = |id, text: &str| (row(id, MessageKind::Task, Seat, text), "seats/a/inbox");
        let rows = [
            task(1, "Task A delegated"),
            task(2, "Task B delegated"),
            task(3, "Task C delegated"),
            task(4, "Task A reported"),
            task(5, "Task C reported"),
            task(6, "Task C accepted"),
            task(7, "Task D delegated: no card is loaded for this one"),
            task(8, "Task B rejected: tests are red"),
            (row(9, MessageKind::Text, Seat, "ship it?"), "operator/inbox"),
            task(10, "Task E accepted"),
            task(11, "Task B amended"),
            task(12, "Task B released"),
            (row(13, MessageKind::Text, Operator, "ok"), "seats/a/inbox"),
            (row(14, MessageKind::Task, Operator, "arrived live"), "seats/a/inbox"),
        ];
        let (entries, mut facts) = fixture(&rows, 8);
        for (message, task, event) in [
            (1, 100, "delegate"),
            (2, 101, "delegate"),
            (3, 102, "delegate"),
            (4, 100, "report"),
            (5, 102, "report"),
            (6, 102, "accept"),
            (7, 103, "delegate"),
            (8, 101, "reject"),
            (10, 104, "accept"),
            (11, 101, "amend"),
            (12, 101, "release"),
        ] {
            typed(&mut facts, message, task, event);
        }
        let tasks: VecDeque<_> = [
            card(100, "Alpha", "reported", Some(0)),
            card(101, "Beta", "claimed", None),
            card(102, "Gamma", "done", Some(0)),
            card(104, "Epsilon", "accepted", Some(0)),
        ]
        .into();
        let model = crate::model::Model::default();
        let (out, feed) = project(entries.clone(), &facts, &tasks, &[], &model, false);
        let time = |minute: u64| crate::local_time::hm(minute * 60_000);
        assert_eq!(
            lines(&out[..feed.pinned]),
            [
                "seat:vex · reported · Alpha".to_owned(),
                format!("{} · Vex · asked · ship it?", time(9)),
            ]
        );
        assert!(matches!(out[feed.pinned], Entry::Tabs { .. }));
        assert_eq!(
            lines(&out[feed.pinned..]),
            [
                // Alpha awaits acceptance: first, though Beta is newer.
                format!("{} · Vex · delegated · Alpha", time(1)),
                format!("{} · Vex · reported · Alpha", time(4)),
                // Then newest task first.
                format!("{} · Vex · delegated · Beta", time(2)),
                format!("{} · Vex · rejected · Beta", time(8)),
                format!("{} · Vex · amended · Beta", time(11)),
                format!("{} · Vex · released · Beta", time(12)),
                // Accepted but not closed: not folded.
                format!("{} · Vex · accepted · Epsilon", time(10)),
                // No card: the object is the message's first 60 characters.
                format!(
                    "{} · Vex · delegated · Task D delegated: no card is loaded for this one",
                    time(7)
                ),
                // Done, and its last event is the acceptance: one line.
                "Gamma · done".to_owned(),
                // Not task threads: in arrival order after them.
                format!("{} · Vex · asked · ship it?", time(9)),
                format!("{} · operator · arrived live", time(14)),
            ]
        );
        assert_eq!(texts(&out), ["ok"], "the operator's own message stays a card");
        let threads: Vec<_> = out[feed.pinned..]
            .iter()
            .filter_map(|entry| match entry {
                Entry::Line { data } => Some((data.task.clone(), data.first, data.needs)),
                _ => None,
            })
            .collect();
        let id = |task: u128| Ulid(task).to_string();
        assert_eq!(
            threads,
            [
                (id(100), true, false),
                (id(100), false, true),
                (id(101), true, false),
                (id(101), false, false),
                (id(101), false, false),
                (id(101), false, false),
                (id(104), true, false),
                (id(103), true, false),
                (id(102), true, false),
                (String::new(), false, true),
                (String::new(), false, false),
            ]
        );
        // Three Gamma events fold into one row: 14 messages, 12 rows.
        assert_eq!((feed.label().as_str(), feed.mode()), ("12 / 14", "signal"));
        // "all" keeps every full card and no ledger.
        let (all, feed) = project(entries.clone(), &facts, &tasks, &[], &model, true);
        assert_eq!(all[feed.pinned..], entries[..]);
        assert_eq!((feed.label().as_str(), feed.mode()), ("14 / 14", "all"));
    }

    #[test]
    fn a_done_task_does_not_fold_while_a_later_event_follows_its_acceptance() {
        let rows = [
            (row(1, MessageKind::Task, Seat, "accepted"), "seats/a/inbox"),
            (row(2, MessageKind::Task, Seat, "amended"), "seats/a/inbox"),
        ];
        let (entries, mut facts) = fixture(&rows, u64::MAX);
        typed(&mut facts, 1, 100, "accept");
        typed(&mut facts, 2, 100, "amend");
        let tasks: VecDeque<_> = [card(100, "Alpha", "done", Some(0))].into();
        let model = crate::model::Model::default();
        let (out, _) = project(entries, &facts, &tasks, &[], &model, false);
        let time = |minute: u64| crate::local_time::hm(minute * 60_000);
        assert_eq!(
            lines(&out),
            [
                format!("{} · Vex · accepted · Alpha", time(1)),
                format!("{} · Vex · amended · Alpha", time(2)),
            ]
        );
    }

    #[test]
    fn typed_facts_come_only_from_the_stored_body() {
        let task = wire::Body::Task {
            task_id: Ulid(7),
            event: "report".into(),
            path: ".bboxy/tasks/x.md".into(),
            summary: "Task accepted, says the prose".into(),
        };
        assert_eq!(
            Typed::of(&task),
            Some(Typed::Task {
                task: Ulid(7),
                event: "report".into()
            })
        );
        assert_eq!(Typed::of(&wire::Body::Text { text: "x".into() }), None);
        for (event, expected) in [
            ("delegate", "delegated"),
            ("report", "reported"),
            ("accept", "accepted"),
            ("reject", "rejected"),
            ("amend", "amended"),
            ("release", "released"),
            ("anything else", ""),
        ] {
            assert_eq!(verb(event), expected);
        }
    }
}
