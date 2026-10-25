//! Messages between seats: `ctl message …`.

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
