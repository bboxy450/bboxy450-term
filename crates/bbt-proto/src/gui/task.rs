//! Task requests carried for the operator's seats.

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! task_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Task {
                id: Ulid,
                request: crate::messaging::TaskRequest,
            },
        ] $($head)*);
    };
}
pub(super) use task_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! task_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Task {
                op: Ulid,
                response: crate::messaging::TaskResponse,
            },
        ] $($head)*);
    };
}
pub(super) use task_replies;
