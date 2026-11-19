// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Questions and approvals observed on the harness event log.
use super::*;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QuestionCursor {
    pub asked_at: u64,
    pub seat: Ulid,
    pub generation: u64,
    pub item: Ulid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboxQuestion {
    pub seat: Ulid,
    pub seat_name: String,
    pub swarm: Ulid,
    pub role: String,
    pub generation: u64,
    pub item: Ulid,
    pub message: String,
    pub asked_at: u64,
    #[serde(flatten)]
    pub content: QuestionContent,
}

impl InboxQuestion {
    pub fn cursor(&self) -> QuestionCursor {
        QuestionCursor {
            asked_at: self.asked_at,
            seat: self.seat,
            generation: self.generation,
            item: self.item,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuestionContent {
    Question { schema: Value },
    Approval { options: Vec<ApprovalOption> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOption {
    Allow,
    AllowAlways,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionScope {
    View,
    Answer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionClient {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuestionAnswerer {
    Client(QuestionClient),
    Resolver {
        by: String,
        #[serde(flatten)]
        details: std::collections::BTreeMap<String, Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum QuestionAnswerResult {
    Applied,
    AlreadyAnswered { by: QuestionAnswerer },
    StaleGeneration,
    Refused { scope: QuestionScope },
}

macro_rules! question_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Page current items oldest first and subscribe this GUI connection
            /// to question events. An inbox Gap asks the client to page again.
            Questions {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                after: Option<QuestionCursor>,
            },
            /// Reuse the operation ID after an interrupted reply.
            Answer {
                seat: Ulid,
                generation: u64,
                item: Ulid,
                op_id: Ulid,
                reply: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use question_actions;

macro_rules! question_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Questions {
                items: Vec<InboxQuestion>,
                cursor: Option<QuestionCursor>,
            },
            Answer {
                op_id: Ulid,
                result: QuestionAnswerResult,
            },
            QuestionOpened {
                seat: Ulid,
                item: Ulid,
            },
            QuestionClosed {
                seat: Ulid,
                item: Ulid,
                by: QuestionAnswerer,
            },
        ] $($head)*);
    };
}
pub(super) use question_replies;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_and_answer_wire_are_additive() {
        assert!(UiRequest::Key(vec!["cmd-i".into()]).validate().is_ok());
        let questions: GuiAction = serde_json::from_value(serde_json::json!({
            "kind": "Questions", "body": {}
        }))
        .unwrap();
        assert!(matches!(questions, GuiAction::Questions { after: None }));
        let by = QuestionAnswerer::Client(QuestionClient {
            name: "Desktop".into(),
            key: "local".into(),
        });
        let reply = GuiReply::Answer {
            op_id: Ulid::new(),
            result: QuestionAnswerResult::AlreadyAnswered { by },
        };
        let wire = serde_json::to_value(reply).unwrap();
        assert_eq!(
            wire["body"]["result"],
            serde_json::json!({
                "status": "already_answered", "by": {"name": "Desktop", "key": "local"}
            })
        );
        assert!(matches!(
            serde_json::from_value::<GuiReply>(wire).unwrap(),
            GuiReply::Answer { .. }
        ));
    }
}
