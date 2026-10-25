//! Messaging requests carried for the operator's seats.

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! message_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Messaging {
                id: Ulid,
                #[serde(with = "text_id")]
                swarm_id: crate::SwarmId,
                request: crate::messaging::Request,
            },
        ] $($head)*);
    };
}
pub(super) use message_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! message_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Messaging {
                op: Ulid,
                response: crate::messaging::Response,
            },
        ] $($head)*);
    };
}
pub(super) use message_replies;
