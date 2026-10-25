//! The e2e promise records: `ctl e2e …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[("e2e", "list", "Knowledge")];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! e2e_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "e2e": [
            E2e {
                group: Option<String>,
            },
        ]] $($head)*);
    };
}
pub(super) use e2e_commands;

/// `ctl e2e …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    Ok(Some(match *words {
        ["e2e"] => Command::E2e {
            group: flag("group"),
        },
        ["e2e", "list"] => Command::E2e {
            group: flag("group"),
        },
        _ => return Ok(None),
    }))
}
