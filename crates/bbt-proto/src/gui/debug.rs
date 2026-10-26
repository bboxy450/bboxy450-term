//! Bounded read-only diagnostics: `ctl debug …`.
use super::*;

/// Fixed diagnostic sources; a caller cannot use the debug door to read a path or SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugProcess {
    App,
    Logic,
    Supervisor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugRequest {
    /// Unix milliseconds; an unfiltered read acknowledges retained error rows.
    Errors {
        since: Option<u64>,
        seat: Option<Ulid>,
    },
    Logs {
        process: DebugProcess,
        since: u64,
    },
    /// `account/<op>` or `seat/<seat-id>/<op>`; never an SQL statement.
    Journal {
        query: String,
    },
    Metrics,
    /// Test-account-only retained History projection; original ring/live bytes remain intact.
    RingGap {
        seat: Ulid,
        from: u64,
        to: u64,
    },
    /// Destination is part of the journaled arguments; only the CLI writes the file.
    Snapshot {
        destination: String,
    },
    Ps,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! debug_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Bounded diagnostics and test-account fixtures, admitted only through the ctl door.
            Debug {
                id: Ulid,
                request: DebugRequest,
            },
        ] $($head)*);
    };
}
pub(super) use debug_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! debug_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Debug {
                op: Ulid,
                data: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use debug_replies;
