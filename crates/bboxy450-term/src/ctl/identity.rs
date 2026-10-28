// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The operator identity hold: `ctl identity …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("identity", "status", "Identity"),
    ("identity", "unlock", "Identity"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! identity_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "identity": [
            Identity(bbt_proto::identity::IdentityRequest),
        ]] $($head)*);
    };
}
pub(super) use identity_commands;

/// `ctl identity …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], _cx: &super::ParseCx) -> Result<Option<Command>, String> {
    Ok(Some(match *words {
        ["identity", "status"] => Command::Identity(bbt_proto::identity::IdentityRequest::Status),
        ["identity", "unlock"] => Command::Identity(bbt_proto::identity::IdentityRequest::Unlock),
        _ => return Ok(None),
    }))
}

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    Ok(match command {
        Command::Identity(request) => {
            let (op, response) = client.identity(request)?;
            vec![json!({"kind":"identity", "op":op, "response":response})]
        }
        _ => {
            return Err(no_flat_form());
        }
    })
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &["identity status", "identity unlock"];
