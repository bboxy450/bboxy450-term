//! Public messaging presentation. Backend defaults and observations are inputs.
use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Seat {
    pub id: String,
    pub generation: u64,
    pub name: String,
    pub role: String,
}

/// UI destination labels: `direct`, or a displayed channel name. The host owns
/// conversion to the admitted backend's canonical destination paths.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    pub destination: String,
    pub class: String,
    pub mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub conversation: String,
    pub direct_conversation: String,
    pub named_conversation: String,
    pub event: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Messaging {
    pub seat: Seat,
    pub channels: Vec<String>,
    pub rules: Vec<Rule>,
    pub defaults: Defaults,
    pub deferred: Option<crate::modules::seat_row::Deferred>,
    pub muted: bool,
    pub error: Option<String>,
}

impl Messaging {
    pub fn default_mode(&self, destination: &str, class: &str) -> &str {
        if class == "event" {
            &self.defaults.event
        } else if destination == "direct" {
            &self.defaults.direct_conversation
        } else {
            &self.defaults.conversation
        }
    }
    pub fn rule(&self, destination: &str, class: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|rule| rule.destination == destination && rule.class == class)
    }
    pub fn mode(&self, destination: &str, class: &str) -> &str {
        self.rule(destination, class).map_or_else(
            || self.default_mode(destination, class),
            |rule| rule.mode.as_str(),
        )
    }
}

impl Settings {
    pub(super) fn messaging(&self, b: &Build) -> c::Element {
        let v = &self.values.messaging;
        if v.seat.name.is_empty() {
            return b.card(
                "card0",
                "Messaging",
                vec![
                    b.note(
                        "unavailable",
                        v.error
                            .as_deref()
                            .unwrap_or("Focus a seat to read its messaging policy."),
                        false,
                    )
                    .into(),
                ],
                [4., 14., 10., 14.],
            );
        }
        let mut header = vec![
            b.text("messaging/seat/name", &v.seat.name, 13., 600, false, false)
                .into(),
            super::plugins::chip(
                b,
                "messaging/seat/role",
                &format!("{} defaults", v.seat.role),
                None,
            )
            .into(),
        ];
        if let Some(deferred) = &v.deferred {
            header.push(
                b.text(
                    "messaging/seat/deferred",
                    format!("{} deferred · digest at {}", deferred.n, deferred.at),
                    11.,
                    400,
                    true,
                    true,
                )
                .into(),
            );
        }
        let mut rows = vec![
            row(
                "messaging/seat",
                plain(None, [8., 0., 16., 0.], 0.),
                16.,
                header,
            )
            .into(),
        ];
        if v.muted {
            rows.push(col("messaging/muted", plain(None, [0., 0., 8., 0.], 0.), 0., vec![
                col("messaging/muted/alert", surface(color(TOKENS.color.state.waiting),
                    0.14, STYLE.radius.row as f32, [6., 10., 6., 10.], None), 0., vec![
                    wrap("messaging/muted/text", "⚠ muted by the operator: this seat’s traffic is held both ways until unmuted (⌘M); the policy below applies after",
                        font(12.5, 600, false, ui::Color::Foreground)).into(),
                ]).into(),
            ]).into());
        }
        if let Some(error) = &v.error {
            rows.push(b.note("messaging/error", error, true).into());
        }
        for (i, destination) in std::iter::once("direct")
            .chain(v.channels.iter().map(String::as_str))
            .enumerate()
        {
            let key = format!("messaging/group/{i}");
            let mut labels = vec![
                b.text(
                    &format!("{key}/label"),
                    if i == 0 {
                        "Direct and by name"
                    } else {
                        destination
                    },
                    12.,
                    500,
                    false,
                    true,
                )
                .into(),
            ];
            if i == 0 {
                labels.push(
                    b.text(
                        &format!("{key}/hint"),
                        "this seat’s own inbox",
                        11.,
                        400,
                        false,
                        true,
                    )
                    .into(),
                );
            }
            let mut group = c::Box::new(plain(None, [12., 0., 6., 0.], 0.), labels);
            group.gap = ui::Length::Px(10.);
            group.align = c::Alignment::Baseline;
            rows.push(named(key, c::Element::Box(group)).into());
            for (index, (class, label)) in [("conversation", "Conversation"), ("event", "Events")]
                .into_iter()
                .enumerate()
            {
                let key = format!("card0/r{}", i * 2 + index);
                rows.push(
                    b.setting_row(
                        &key,
                        label,
                        None,
                        b.segment("", &["wake", "inbox", "digest"], v.mode(destination, class)),
                        v.rule(destination, class).is_some(),
                        false,
                        false,
                    )
                    .into(),
                );
            }
        }
        rows.push(b.note("messaging/note", "Always wakes, for conversation only: a message from the operator, a task delegated to this seat, a reply in a thread it started; a seat cannot set itself to miss its own work. Events never wake by default, whoever sent them. A digest delivers once, at 10 min or 50 messages, listing every message by id: it delays a wake and never holds a message. A change applies to messages stored after it.", true).into());
        b.card("card0", "Messaging", rows, [4., 14., 10., 14.])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::settings::controller::{Action, Controller};

    // keeps: spec messaging-attention §3 MA-8 — policy edits are intents, including while muted.
    #[test]
    fn muted_policy_cells_emit_a_reset_without_optimistically_changing_the_record() {
        let mut records = Records {
            messaging: Messaging {
                seat: Seat {
                    id: "seat".into(),
                    generation: 3,
                    name: "Receiver".into(),
                    role: "executor".into(),
                },
                channels: vec!["#room".into()],
                muted: true,
                defaults: Defaults {
                    conversation: "digest".into(),
                    direct_conversation: "wake".into(),
                    named_conversation: "wake".into(),
                    event: "inbox".into(),
                },
                rules: vec![Rule {
                    destination: "#room".into(),
                    class: "event".into(),
                    mode: "wake".into(),
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let original = records.messaging.clone();
        let mut controller = Controller::new("Messaging");
        assert!(
            controller
                .handle(
                    Action::Attention(Rule {
                        destination: "#room".into(),
                        class: "event".into(),
                        mode: "inbox".into()
                    }),
                    &mut records,
                    &Records::default()
                )
                .is_none()
        );
        assert_eq!(controller.take_attention().unwrap().mode, "default");
        assert_eq!(records.messaging, original);
        controller.handle(
            Action::Attention(Rule {
                destination: "#other".into(),
                class: "event".into(),
                mode: "wake".into(),
            }),
            &mut records,
            &Records::default(),
        );
        assert!(controller.take_attention().is_none());
        assert!(controller.error.is_some());
    }
}
