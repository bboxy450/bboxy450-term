//! Knowledge reads and operator-signed record writes.

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! record_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// An operator-signed knowledge write of an `e2e` record (`ctl record write`, e2e
            /// acceptance §2): `args` is the knowledge_write DTO (`path` or `ref`, `frontmatter`,
            /// `body`, `etag`). Logic signs it with the operator device under its issuer and journals
            /// it; the reply is `Knowledge` with the serialized data_source result.
            RecordWrite {
                id: Ulid,
                args: serde_json::Value,
            },
            /// Read-only portable data_source call. `args` is the existing method DTO;
            /// admission is the same operator GUI admission as every other view read.
            Knowledge {
                id: Ulid,
                method: String,
                args: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use record_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! record_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Serialized data_source Result<Value, CommandError>, bounded by the
            /// provider page limit. It carries no new document or error DTO.
            Knowledge {
                op: Ulid,
                response: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use record_replies;
