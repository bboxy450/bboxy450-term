// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Own-seat Dream controls use the already admitted operator GUI/ctl door.
macro_rules! dream_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Dream {
                id: Ulid,
                seat: Ulid,
                request: crate::dream::Control,
            },
        ] $($head)*);
    };
}
pub(super) use dream_actions;
macro_rules! dream_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Dream {
                op: Ulid,
                response: crate::dream::ControlReply,
            },
        ] $($head)*);
    };
}
pub(super) use dream_replies;
