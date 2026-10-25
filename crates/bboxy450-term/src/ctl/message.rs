//! Messages between seats: `ctl message …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("message", "send", "Messaging"),
    ("message", "post", "Messaging"),
    ("message", "inbox", "Messaging"),
    ("message", "receipts", "Messaging"),
    ("message", "ack", "Messaging"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! message_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "message": [
            Dm {
                seat: String,
                text: String,
            },
            Post {
                destination: String,
                text: String,
                swarm: Option<Ulid>,
            },
            Inbox {
                since: u64,
            },
            Receipts {
                message: Ulid,
            },
            /// `message send (<destination|seat> | --reply-to <id>) <text>` (#264 routing).
            Send {
                to: Option<String>,
                text: String,
                reply_to: Option<Ulid>,
                swarm: Option<Ulid>,
            },
            /// `message ack <id>`: the operator acknowledges a message addressed to it.
            Ack {
                message: Ulid,
            },
        ]] $($head)*);
    };
}
pub(super) use message_commands;

/// `ctl message …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let ulid = |v: &String| v.parse::<Ulid>().map_err(|_| format!("not an id: {v}"));
    let swarm = cx.swarm;
    Ok(Some(match *words {
        ["dm", seat, text] => Command::Dm {
            seat: seat.into(),
            text: text.into(),
        },
        ["post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["message", "send", to, text] => Command::Send {
            to: Some(to.into()),
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "send", text] if flags.contains_key("reply-to") => Command::Send {
            to: None,
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["message", "inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["message", "receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["message", "ack", message] => Command::Ack {
            message: ulid(&message.to_owned())?,
        },
        _ => return Ok(None),
    }))
}
