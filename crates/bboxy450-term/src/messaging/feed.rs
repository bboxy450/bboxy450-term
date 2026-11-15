// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The Chat section's projection: what needs the operator, pinned, then signal.
//! It only chooses and orders rows the history already built; delivery, read
//! cursors and the full cards ("all") are untouched.
use super::wire::{self, MessageKind};
use bbt_ui::modules::{
    composer::Feed,
    stream_dock::{Entry, Line, Message},
};
use std::collections::{BTreeMap, VecDeque};

/// Pinned rows above the list; the last one counts the rest.
pub const PINNED: usize = 5;
/// A line's object when it is message text.
const OBJECT: usize = 60;

/// Typed facts of one retained message. Nothing is read from its prose except
/// the trailing `?` of a seat's message to the operator inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub kind: MessageKind,
    /// Written by the operator.
    pub operator: bool,
    /// Addressed to the operator inbox.
    pub to_operator: bool,
}
impl Fact {
    pub fn of(row: &wire::MessageRow, context: &str) -> Self {
        Self {
            kind: row.kind,
            operator: matches!(
                row.author.kind,
                wire::AuthorKind::Operator | wire::AuthorKind::OperatorLocal
            ),
            to_operator: context == "operator/inbox",
        }
    }
    fn signal(&self) -> bool {
        self.operator
            || self.to_operator
            || matches!(
                self.kind,
                MessageKind::Decision
                    | MessageKind::Task
                    | MessageKind::GateResult
                    | MessageKind::Landed
            )
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

/// `entries` is the complete history projection. Signal keeps decisions, task
/// messages, anything to or from the operator, gate results and landings; it
/// drops presence and seat-to-seat text. The count is rows shown of messages
/// retained, pinned rows excluded.
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
    let mut shown = 0;
    // A note directly after a message describes it and leaves with it.
    let mut hidden = false;
    for entry in entries {
        match &entry {
            Entry::Message { data, .. } => {
                // The operator's own close notices are not presence traffic.
                let notice = data.message_ids.iter().any(|id| id.starts_with("close:"));
                hidden = !all && !notice && !fact(facts, data).is_some_and(Fact::signal);
                if hidden {
                    continue;
                }
                shown += 1;
            }
            Entry::Note { .. } if hidden => continue,
            _ => hidden = false,
        }
        out.push(entry);
    }
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
    use bbt_mux::Ulid;

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
            .map(|(row, context)| (row.id.to_string(), Fact::of(row, context)))
            .collect();
        (entries, facts)
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
        let (mut entries, facts) = fixture(&rows, u64::MAX);
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
        assert_eq!(
            texts(&signal),
            [
                "decision",
                "task wake",
                "from operator",
                "to operator",
                "gate",
                "landed",
                "closed"
            ]
        );
        assert_eq!((feed.shown, feed.total, feed.pinned), (7, 11, 0));
        assert_eq!(feed.label(), "7 / 11 · signal");
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
        assert_eq!(feed.label(), "11 / 11 · all");
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
        assert_eq!(feed.label(), "0 / 0 · signal");
    }
}
