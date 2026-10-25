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
