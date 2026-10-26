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

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    Ok(match command {
        Command::E2e { group } => {
            let mut loader = crate::knowledge::E2eLoader::new(crate::knowledge::promises()?);
            // Validate the group before sending any provider request. The
            // projection still supplies the global summary for filtered rows.
            crate::knowledge::e2e_lines(loader.snapshot(), group.as_deref())?;
            while let Some(call) = loader.next_request() {
                loader.respond(client.knowledge(&call));
            }
            crate::knowledge::e2e_lines(loader.snapshot(), group.as_deref())?
        }
        _ => {
            return Err(no_flat_form());
        }
    })
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &["e2e list"];
